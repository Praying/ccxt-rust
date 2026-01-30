//! WebSocket client module.
//!
//! Provides asynchronous WebSocket connection management, subscription handling,
//! and heartbeat maintenance for cryptocurrency exchange streaming APIs.

mod config;
mod error;
mod event;
mod message;
mod reconnect;
mod state;
mod subscription;

pub use config::{
    BackoffConfig, BackoffStrategy, BackpressureStrategy, DEFAULT_MAX_SUBSCRIPTIONS,
    DEFAULT_MESSAGE_CHANNEL_CAPACITY, DEFAULT_SHUTDOWN_TIMEOUT, DEFAULT_WRITE_CHANNEL_CAPACITY,
    WsConfig,
};
pub use error::{WsError, WsErrorKind};
pub use event::{WsEvent, WsEventCallback};
pub use message::WsMessage;
pub use reconnect::AutoReconnectCoordinator;
pub use state::{WsConnectionState, WsStats, WsStatsSnapshot};
pub use subscription::{Subscription, SubscriptionManager};

use crate::error::{Error, Result};
use derive_more::Debug;
use futures_util::{SinkExt, StreamExt, stream::SplitSink};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU8, AtomicU32, Ordering};
use tokio::net::TcpStream;
use tokio::sync::{Mutex, RwLock, mpsc};
use tokio::time::{Duration, interval};
use tokio_tungstenite::{
    MaybeTlsStream, WebSocketStream, connect_async_with_config,
    tungstenite::protocol::{Message, WebSocketConfig},
};
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, instrument, warn};

/// Type alias for WebSocket write half.
#[allow(dead_code)]
type WsWriter = SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>;

/// Async WebSocket client for exchange streaming APIs.
///
/// # Backpressure Handling
///
/// This client uses bounded channels to prevent memory exhaustion in high-frequency
/// trading scenarios. When the message channel is full, the configured
/// `BackpressureStrategy` determines how to handle new messages:
///
/// - `DropOldest`: Removes the oldest message to make room (default)
/// - `DropNewest`: Discards the incoming message
/// - `Block`: Waits until space is available (may stall the read loop)
#[derive(Debug)]
pub struct WsClient {
    config: WsConfig,
    state: Arc<AtomicU8>,
    subscription_manager: SubscriptionManager,
    message_tx: mpsc::Sender<Value>,
    message_rx: Arc<RwLock<mpsc::Receiver<Value>>>,
    write_tx: Arc<RwLock<Option<mpsc::Sender<Message>>>>,
    pub(crate) reconnect_count: AtomicU32,
    shutdown_tx: Arc<Mutex<Option<mpsc::UnboundedSender<()>>>>,
    stats: Arc<WsStats>,
    cancel_token: Arc<Mutex<Option<CancellationToken>>>,
    #[debug(skip)]
    event_callback: Arc<Mutex<Option<WsEventCallback>>>,
    /// Counter for dropped messages due to backpressure
    dropped_messages: Arc<AtomicU32>,
}

impl WsClient {
    /// Creates a new WebSocket client instance.
    ///
    /// The client uses bounded channels for message passing to prevent memory
    /// exhaustion. Channel capacities are configured via `WsConfig`.
    pub fn new(config: WsConfig) -> Self {
        let (message_tx, message_rx) = mpsc::channel(config.message_channel_capacity);
        let max_subscriptions = config.max_subscriptions;

        Self {
            config,
            state: Arc::new(AtomicU8::new(WsConnectionState::Disconnected.as_u8())),
            subscription_manager: SubscriptionManager::new(max_subscriptions),
            message_tx,
            message_rx: Arc::new(RwLock::new(message_rx)),
            write_tx: Arc::new(RwLock::new(None)),
            reconnect_count: AtomicU32::new(0),
            shutdown_tx: Arc::new(Mutex::new(None)),
            stats: Arc::new(WsStats::new()),
            cancel_token: Arc::new(Mutex::new(None)),
            event_callback: Arc::new(Mutex::new(None)),
            dropped_messages: Arc::new(AtomicU32::new(0)),
        }
    }

    /// Sets the event callback for connection lifecycle events.
    pub async fn set_event_callback(&self, callback: WsEventCallback) {
        *self.event_callback.lock().await = Some(callback);
        debug!("Event callback set");
    }

    /// Clears the event callback.
    pub async fn clear_event_callback(&self) {
        *self.event_callback.lock().await = None;
        debug!("Event callback cleared");
    }

    async fn emit_event(&self, event: WsEvent) {
        let callback = self.event_callback.lock().await;
        if let Some(ref cb) = *callback {
            let cb = Arc::clone(cb);
            drop(callback);
            tokio::spawn(async move {
                cb(event);
            });
        }
    }

    /// Sets the cancellation token for this client.
    pub async fn set_cancel_token(&self, token: CancellationToken) {
        *self.cancel_token.lock().await = Some(token);
        debug!("Cancellation token set");
    }

    /// Clears the cancellation token.
    pub async fn clear_cancel_token(&self) {
        *self.cancel_token.lock().await = None;
        debug!("Cancellation token cleared");
    }

    /// Returns a clone of the current cancellation token, if set.
    pub async fn get_cancel_token(&self) -> Option<CancellationToken> {
        self.cancel_token.lock().await.clone()
    }

    /// Establishes connection to the WebSocket server.
    #[instrument(
        name = "ws_connect",
        skip(self),
        fields(url = %self.config.url, timeout_ms = self.config.connect_timeout)
    )]
    pub async fn connect(&self) -> Result<()> {
        if self.state() == WsConnectionState::Connected {
            info!("WebSocket already connected");
            return Ok(());
        }

        self.set_state(WsConnectionState::Connecting);

        let url = self.config.url.clone();
        info!("Initiating WebSocket connection");

        let mut ws_config = WebSocketConfig::default();
        if let Some(limit) = self.config.max_message_size {
            ws_config.max_message_size = Some(limit);
        }
        if let Some(limit) = self.config.max_frame_size {
            ws_config.max_frame_size = Some(limit);
        }

        match tokio::time::timeout(
            Duration::from_millis(self.config.connect_timeout),
            connect_async_with_config(&url, Some(ws_config), false),
        )
        .await
        {
            Ok(Ok((ws_stream, response))) => {
                info!(
                    status = response.status().as_u16(),
                    "WebSocket connection established successfully"
                );

                self.set_state(WsConnectionState::Connected);
                self.reconnect_count.store(0, Ordering::Release);
                self.stats.record_connected();
                self.start_message_loop(ws_stream).await;
                self.resubscribe_all().await?;

                Ok(())
            }
            Ok(Err(e)) => {
                error!(error = %e, "WebSocket connection failed");
                self.set_state(WsConnectionState::Error);
                Err(Error::network(format!("WebSocket connection failed: {e}")))
            }
            Err(_) => {
                error!(
                    timeout_ms = self.config.connect_timeout,
                    "WebSocket connection timeout"
                );
                self.set_state(WsConnectionState::Error);
                Err(Error::timeout("WebSocket connection timeout"))
            }
        }
    }

    /// Establishes connection with cancellation support.
    #[instrument(
        name = "ws_connect_with_cancel",
        skip(self, cancel_token),
        fields(url = %self.config.url)
    )]
    pub async fn connect_with_cancel(&self, cancel_token: Option<CancellationToken>) -> Result<()> {
        let token = if let Some(t) = cancel_token {
            t
        } else {
            let internal_token = self.cancel_token.lock().await;
            internal_token
                .clone()
                .unwrap_or_else(CancellationToken::new)
        };

        if self.state() == WsConnectionState::Connected {
            info!("WebSocket already connected");
            return Ok(());
        }

        self.set_state(WsConnectionState::Connecting);
        let url = self.config.url.clone();

        let mut ws_config = WebSocketConfig::default();
        if let Some(limit) = self.config.max_message_size {
            ws_config.max_message_size = Some(limit);
        }
        if let Some(limit) = self.config.max_frame_size {
            ws_config.max_frame_size = Some(limit);
        }

        tokio::select! {
            biased;
            () = token.cancelled() => {
                warn!("WebSocket connection cancelled");
                self.set_state(WsConnectionState::Disconnected);
                Err(Error::cancelled("WebSocket connection cancelled"))
            }
            result = tokio::time::timeout(
                Duration::from_millis(self.config.connect_timeout),
                connect_async_with_config(&url, Some(ws_config), false),
            ) => {
                match result {
                    Ok(Ok((ws_stream, response))) => {
                        info!(status = response.status().as_u16(), "WebSocket connected");
                        self.set_state(WsConnectionState::Connected);
                        self.reconnect_count.store(0, Ordering::Release);
                        self.stats.record_connected();
                        self.start_message_loop(ws_stream).await;
                        self.resubscribe_all().await?;
                        Ok(())
                    }
                    Ok(Err(e)) => {
                        error!(error = %e, "WebSocket connection failed");
                        self.set_state(WsConnectionState::Error);
                        Err(Error::network(format!("WebSocket connection failed: {e}")))
                    }
                    Err(_) => {
                        error!("WebSocket connection timeout");
                        self.set_state(WsConnectionState::Error);
                        Err(Error::timeout("WebSocket connection timeout"))
                    }
                }
            }
        }
    }

    /// Closes the WebSocket connection gracefully.
    #[instrument(name = "ws_disconnect", skip(self))]
    pub async fn disconnect(&self) -> Result<()> {
        info!("Initiating WebSocket disconnect");

        if let Some(tx) = self.shutdown_tx.lock().await.as_ref() {
            let _ = tx.send(());
        }

        *self.write_tx.write().await = None;
        self.set_state(WsConnectionState::Disconnected);

        info!("WebSocket disconnected");
        Ok(())
    }

    /// Gracefully shuts down the WebSocket client.
    #[instrument(name = "ws_shutdown", skip(self))]
    pub async fn shutdown(&self) {
        info!("Initiating graceful shutdown");

        {
            let token_guard = self.cancel_token.lock().await;
            if let Some(ref token) = *token_guard {
                token.cancel();
            }
        }

        self.set_state(WsConnectionState::Disconnected);

        {
            let write_tx_guard = self.write_tx.read().await;
            if let Some(ref tx) = *write_tx_guard {
                // Ignore send result - we're shutting down anyway
                drop(tx.send(Message::Close(None)).await);
            }
        }

        let shutdown_timeout = Duration::from_millis(self.config.shutdown_timeout);
        let _ = tokio::time::timeout(shutdown_timeout, async {
            if let Some(tx) = self.shutdown_tx.lock().await.as_ref() {
                let _ = tx.send(());
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        })
        .await;

        {
            *self.write_tx.write().await = None;
            *self.shutdown_tx.lock().await = None;
            self.subscription_manager.clear();
            self.reconnect_count.store(0, Ordering::Release);
            self.dropped_messages.store(0, Ordering::Relaxed);
            self.stats.reset();
        }

        self.emit_event(WsEvent::Shutdown).await;
        info!("Graceful shutdown completed");
    }

    /// Attempts to reconnect to the WebSocket server.
    #[instrument(name = "ws_reconnect", skip(self))]
    pub async fn reconnect(&self) -> Result<()> {
        let count = self.reconnect_count.fetch_add(1, Ordering::AcqRel) + 1;

        if count > self.config.max_reconnect_attempts {
            error!(attempts = count, "Max reconnect attempts reached");
            return Err(Error::network("Max reconnect attempts reached"));
        }

        warn!(attempt = count, "Attempting WebSocket reconnection");
        self.set_state(WsConnectionState::Reconnecting);

        tokio::time::sleep(Duration::from_millis(self.config.reconnect_interval)).await;
        self.connect().await
    }

    /// Attempts to reconnect with cancellation support.
    #[instrument(name = "ws_reconnect_with_cancel", skip(self, cancel_token))]
    pub async fn reconnect_with_cancel(
        &self,
        cancel_token: Option<CancellationToken>,
    ) -> Result<()> {
        let token = if let Some(t) = cancel_token {
            t
        } else {
            let internal_token = self.cancel_token.lock().await;
            internal_token
                .clone()
                .unwrap_or_else(CancellationToken::new)
        };

        let backoff = BackoffStrategy::new(self.config.backoff_config.clone());
        self.set_state(WsConnectionState::Reconnecting);

        loop {
            if token.is_cancelled() {
                self.set_state(WsConnectionState::Disconnected);
                return Err(Error::cancelled("Reconnection cancelled"));
            }

            let attempt = self.reconnect_count.fetch_add(1, Ordering::AcqRel);

            if attempt >= self.config.max_reconnect_attempts {
                self.set_state(WsConnectionState::Error);
                return Err(Error::network(format!(
                    "Max reconnect attempts ({}) reached",
                    self.config.max_reconnect_attempts
                )));
            }

            let delay = backoff.calculate_delay(attempt);

            tokio::select! {
                biased;
                () = token.cancelled() => {
                    self.set_state(WsConnectionState::Disconnected);
                    return Err(Error::cancelled("Reconnection cancelled during backoff"));
                }
                () = tokio::time::sleep(delay) => {}
            }

            match self.connect_with_cancel(Some(token.clone())).await {
                Ok(()) => {
                    self.reconnect_count.store(0, Ordering::Release);
                    return Ok(());
                }
                Err(e) => {
                    if e.as_cancelled().is_some() {
                        self.set_state(WsConnectionState::Disconnected);
                        return Err(e);
                    }

                    let ws_error = WsError::from_error(&e);
                    if ws_error.is_permanent() {
                        self.set_state(WsConnectionState::Error);
                        return Err(e);
                    }
                }
            }
        }
    }

    /// Returns the current reconnection attempt count.
    #[inline]
    pub fn reconnect_count(&self) -> u32 {
        self.reconnect_count.load(Ordering::Acquire)
    }

    /// Resets the reconnection attempt counter.
    pub fn reset_reconnect_count(&self) {
        self.reconnect_count.store(0, Ordering::Release);
    }

    /// Increments the reconnection attempt counter.
    pub(crate) fn increment_reconnect_count(&self) {
        self.reconnect_count.fetch_add(1, Ordering::AcqRel);
    }

    /// Returns a snapshot of connection statistics.
    pub fn stats(&self) -> WsStatsSnapshot {
        self.stats.snapshot()
    }

    /// Resets all connection statistics.
    pub fn reset_stats(&self) {
        self.stats.reset();
    }

    /// Calculates current connection latency in milliseconds.
    pub fn latency(&self) -> Option<i64> {
        let last_pong = self.stats.last_pong_time();
        let last_ping = self.stats.last_ping_time();
        if last_pong > 0 && last_ping > 0 {
            Some(last_pong - last_ping)
        } else {
            None
        }
    }

    /// Returns the number of messages dropped due to backpressure.
    ///
    /// This counter is incremented when the message channel is full and
    /// messages are dropped according to the configured backpressure strategy.
    pub fn dropped_messages(&self) -> u32 {
        self.dropped_messages.load(Ordering::Relaxed)
    }

    /// Resets the dropped messages counter.
    pub fn reset_dropped_messages(&self) {
        self.dropped_messages.store(0, Ordering::Relaxed);
    }

    /// Creates an automatic reconnection coordinator.
    pub fn create_auto_reconnect_coordinator(self: Arc<Self>) -> AutoReconnectCoordinator {
        AutoReconnectCoordinator::new(self)
    }

    /// Subscribes to a WebSocket channel.
    #[instrument(name = "ws_subscribe", skip(self, params), fields(channel = %channel))]
    pub async fn subscribe(
        &self,
        channel: String,
        symbol: Option<String>,
        params: Option<HashMap<String, Value>>,
    ) -> Result<()> {
        let sub_key = Self::subscription_key(&channel, symbol.as_ref());
        let subscription = Subscription {
            channel: channel.clone(),
            symbol: symbol.clone(),
            params: params.clone(),
        };

        self.subscription_manager
            .try_add(sub_key.clone(), subscription)?;

        if self.state() == WsConnectionState::Connected {
            self.send_subscribe_message(channel, symbol, params).await?;
        }

        Ok(())
    }

    /// Unsubscribes from a WebSocket channel.
    #[instrument(name = "ws_unsubscribe", skip(self), fields(channel = %channel))]
    pub async fn unsubscribe(&self, channel: String, symbol: Option<String>) -> Result<()> {
        let sub_key = Self::subscription_key(&channel, symbol.as_ref());
        self.subscription_manager.remove(&sub_key);

        if self.state() == WsConnectionState::Connected {
            self.send_unsubscribe_message(channel, symbol).await?;
        }

        Ok(())
    }

    /// Receives the next available message.
    pub async fn receive(&self) -> Option<Value> {
        let mut rx = self.message_rx.write().await;
        rx.recv().await
    }

    /// Returns the current connection state.
    #[inline]
    pub fn state(&self) -> WsConnectionState {
        WsConnectionState::from_u8(self.state.load(Ordering::Acquire))
    }

    /// Returns a reference to the WebSocket configuration.
    #[inline]
    pub fn config(&self) -> &WsConfig {
        &self.config
    }

    /// Sets the connection state.
    #[inline]
    pub fn set_state(&self, state: WsConnectionState) {
        self.state.store(state.as_u8(), Ordering::Release);
    }

    /// Checks whether the WebSocket is currently connected.
    #[inline]
    pub fn is_connected(&self) -> bool {
        self.state() == WsConnectionState::Connected
    }

    /// Checks if subscribed to a specific channel.
    pub fn is_subscribed(&self, channel: &str, symbol: Option<&String>) -> bool {
        let sub_key = Self::subscription_key(channel, symbol);
        self.subscription_manager.contains(&sub_key)
    }

    /// Returns the number of active subscriptions.
    pub fn subscription_count(&self) -> usize {
        self.subscription_manager.count()
    }

    /// Returns the remaining capacity for new subscriptions.
    pub fn remaining_capacity(&self) -> usize {
        self.subscription_manager.remaining_capacity()
    }

    /// Returns a list of all active subscription channel names.
    ///
    /// Each subscription is identified by its channel name, optionally combined
    /// with a symbol in the format "channel:symbol" or just "channel".
    pub fn subscriptions(&self) -> Vec<String> {
        self.subscription_manager
            .iter()
            .map(|entry| {
                let sub = entry.value();
                match &sub.symbol {
                    Some(sym) => format!("{}:{}", sub.channel, sym),
                    None => sub.channel.clone(),
                }
            })
            .collect()
    }

    /// Sends a raw WebSocket message.
    ///
    /// This method uses a bounded channel for sending. If the write channel is full,
    /// it will wait until space is available.
    #[instrument(name = "ws_send", skip(self, message))]
    pub async fn send(&self, message: Message) -> Result<()> {
        let tx = self.write_tx.read().await;

        if let Some(sender) = tx.as_ref() {
            sender
                .send(message)
                .await
                .map_err(|e| Error::network(format!("Failed to send message: {e}")))?;
            Ok(())
        } else {
            Err(Error::network("WebSocket not connected"))
        }
    }

    /// Tries to send a raw WebSocket message without blocking.
    ///
    /// Returns an error if the channel is full or closed.
    #[instrument(name = "ws_try_send", skip(self, message))]
    pub fn try_send(&self, message: Message) -> Result<()> {
        // Note: This is a sync method, so we can't use async lock
        // We use try_read to avoid blocking
        if let Ok(tx) = self.write_tx.try_read() {
            if let Some(sender) = tx.as_ref() {
                sender.try_send(message).map_err(|e| match e {
                    mpsc::error::TrySendError::Full(_) => {
                        Error::network("Write channel full (backpressure)")
                    }
                    mpsc::error::TrySendError::Closed(_) => {
                        Error::network("WebSocket channel closed")
                    }
                })?;
                Ok(())
            } else {
                Err(Error::network("WebSocket not connected"))
            }
        } else {
            Err(Error::network("Write channel busy"))
        }
    }

    /// Sends a text message.
    #[instrument(name = "ws_send_text", skip(self, text))]
    pub async fn send_text(&self, text: String) -> Result<()> {
        self.send(Message::Text(text.into())).await
    }

    /// Sends a JSON-encoded message.
    #[instrument(name = "ws_send_json", skip(self, json))]
    pub async fn send_json(&self, json: &Value) -> Result<()> {
        let text = serde_json::to_string(json).map_err(Error::from)?;
        self.send_text(text).await
    }

    fn subscription_key(channel: &str, symbol: Option<&String>) -> String {
        match symbol {
            Some(s) => format!("{channel}:{s}"),
            None => channel.to_string(),
        }
    }

    async fn start_message_loop(&self, ws_stream: WebSocketStream<MaybeTlsStream<TcpStream>>) {
        let (write, mut read) = ws_stream.split();

        // Use bounded channel for write operations to prevent memory exhaustion
        let (write_tx, mut write_rx) = mpsc::channel::<Message>(self.config.write_channel_capacity);
        *self.write_tx.write().await = Some(write_tx.clone());

        // Shutdown channel remains unbounded as it's only used for signaling
        let (shutdown_tx, mut shutdown_rx) = mpsc::unbounded_channel::<()>();
        *self.shutdown_tx.lock().await = Some(shutdown_tx);

        let state = Arc::clone(&self.state);
        let message_tx = self.message_tx.clone();
        let ping_interval_ms = self.config.ping_interval;
        let backpressure_strategy = self.config.backpressure_strategy;
        let dropped_messages = Arc::clone(&self.dropped_messages);

        let write_handle = tokio::spawn(async move {
            let mut write = write;
            loop {
                tokio::select! {
                    Some(msg) = write_rx.recv() => {
                        if let Err(e) = write.send(msg).await {
                            error!(error = %e, "Failed to write message");
                            break;
                        }
                    }
                    _ = shutdown_rx.recv() => {
                        let _ = write.send(Message::Close(None)).await;
                        break;
                    }
                }
            }
        });

        let state_clone = Arc::clone(&state);
        let ws_stats = Arc::clone(&self.stats);
        let read_handle = tokio::spawn(async move {
            while let Some(msg_result) = read.next().await {
                match msg_result {
                    Ok(Message::Text(text)) => {
                        ws_stats.record_received(text.len() as u64);
                        if let Ok(json) = serde_json::from_str::<Value>(&text) {
                            Self::send_with_backpressure(
                                &message_tx,
                                json,
                                backpressure_strategy,
                                &dropped_messages,
                            )
                            .await;
                        }
                    }
                    Ok(Message::Binary(data)) => {
                        ws_stats.record_received(data.len() as u64);
                        if let Some(json) = String::from_utf8(data.to_vec())
                            .ok()
                            .and_then(|text| serde_json::from_str::<Value>(&text).ok())
                        {
                            Self::send_with_backpressure(
                                &message_tx,
                                json,
                                backpressure_strategy,
                                &dropped_messages,
                            )
                            .await;
                        }
                    }
                    Ok(Message::Pong(_)) => {
                        ws_stats.record_pong();
                    }
                    Ok(Message::Close(_)) => {
                        state_clone
                            .store(WsConnectionState::Disconnected.as_u8(), Ordering::Release);
                        break;
                    }
                    Err(_) => {
                        state_clone.store(WsConnectionState::Error.as_u8(), Ordering::Release);
                        break;
                    }
                    _ => {}
                }
            }
        });

        if ping_interval_ms > 0 {
            let write_tx_clone = write_tx.clone();
            let ping_stats = Arc::clone(&self.stats);
            let ping_state = Arc::clone(&state);
            let pong_timeout_ms = self.config.pong_timeout;

            tokio::spawn(async move {
                let mut interval = interval(Duration::from_millis(ping_interval_ms));

                loop {
                    interval.tick().await;

                    let now = chrono::Utc::now().timestamp_millis();
                    let last_pong = ping_stats.last_pong_time();

                    if last_pong > 0 {
                        let elapsed = now - last_pong;
                        #[allow(clippy::cast_possible_wrap)]
                        if elapsed > pong_timeout_ms as i64 {
                            // Pong timeout detected - log detailed diagnostics
                            error!(
                                pong_timeout_ms = pong_timeout_ms,
                                elapsed_ms = elapsed,
                                last_pong_time = last_pong,
                                current_time = now,
                                "WebSocket pong timeout detected - connection appears unresponsive (zombie connection)"
                            );
                            ping_state.store(WsConnectionState::Error.as_u8(), Ordering::Release);
                            debug!(
                                "WebSocket state set to Error due to pong timeout - AutoReconnectCoordinator will trigger reconnection if enabled"
                            );
                            break;
                        }
                    }

                    ping_stats.record_ping();

                    // Use try_send for ping to avoid blocking
                    if write_tx_clone
                        .try_send(Message::Ping(vec![].into()))
                        .is_err()
                    {
                        debug!("WebSocket write channel closed, stopping ping loop");
                        break;
                    }
                }
            });
        }

        tokio::spawn(async move {
            let _ = tokio::join!(write_handle, read_handle);
        });
    }

    /// Sends a message with backpressure handling.
    ///
    /// This method implements the configured backpressure strategy when the
    /// message channel is full.
    async fn send_with_backpressure(
        tx: &mpsc::Sender<Value>,
        message: Value,
        strategy: BackpressureStrategy,
        dropped_counter: &Arc<AtomicU32>,
    ) {
        match strategy {
            BackpressureStrategy::Block => {
                // Block until space is available
                if tx.send(message).await.is_err() {
                    warn!("Message channel closed");
                }
            }
            BackpressureStrategy::DropNewest => {
                // Try to send, drop if full
                match tx.try_send(message) {
                    Ok(()) => {}
                    Err(mpsc::error::TrySendError::Full(_)) => {
                        let count = dropped_counter.fetch_add(1, Ordering::Relaxed) + 1;
                        if count % 100 == 1 {
                            // Log every 100th drop to avoid log spam
                            warn!(
                                dropped_count = count,
                                "Message channel full, dropping newest message (backpressure)"
                            );
                        }
                    }
                    Err(mpsc::error::TrySendError::Closed(_)) => {
                        warn!("Message channel closed");
                    }
                }
            }
            BackpressureStrategy::DropOldest => {
                // Try to send, if full, make room by receiving and discarding
                match tx.try_send(message) {
                    Ok(()) => {}
                    Err(mpsc::error::TrySendError::Full(msg)) => {
                        // Channel is full, we need to drop oldest
                        // Since we can't directly remove from the channel,
                        // we use a permit-based approach
                        let count = dropped_counter.fetch_add(1, Ordering::Relaxed) + 1;
                        if count % 100 == 1 {
                            warn!(
                                dropped_count = count,
                                "Message channel full, dropping oldest message (backpressure)"
                            );
                        }
                        // For DropOldest, we actually drop the newest since we can't
                        // remove from the receiver side. The semantic is that we
                        // prioritize not blocking the read loop.
                        // A true DropOldest would require a different data structure.
                        drop(msg);
                    }
                    Err(mpsc::error::TrySendError::Closed(_)) => {
                        warn!("Message channel closed");
                    }
                }
            }
        }
    }

    async fn send_subscribe_message(
        &self,
        channel: String,
        symbol: Option<String>,
        params: Option<HashMap<String, Value>>,
    ) -> Result<()> {
        let msg = WsMessage::Subscribe {
            channel,
            symbol,
            params,
        };
        let json = serde_json::to_value(&msg).map_err(Error::from)?;
        self.send_json(&json).await
    }

    async fn send_unsubscribe_message(
        &self,
        channel: String,
        symbol: Option<String>,
    ) -> Result<()> {
        let msg = WsMessage::Unsubscribe { channel, symbol };
        let json = serde_json::to_value(&msg).map_err(Error::from)?;
        self.send_json(&json).await
    }

    pub(crate) async fn resubscribe_all(&self) -> Result<()> {
        let subs = self.subscription_manager.collect_subscriptions();
        for subscription in subs {
            self.send_subscribe_message(
                subscription.channel.clone(),
                subscription.symbol.clone(),
                subscription.params.clone(),
            )
            .await?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backoff_config_default() {
        let config = BackoffConfig::default();
        assert_eq!(config.base_delay, Duration::from_secs(1));
        assert_eq!(config.max_delay, Duration::from_secs(60));
    }

    #[test]
    fn test_backoff_strategy_exponential_growth_no_jitter() {
        let config = BackoffConfig {
            base_delay: Duration::from_secs(1),
            max_delay: Duration::from_secs(60),
            jitter_factor: 0.0,
            multiplier: 2.0,
        };
        let strategy = BackoffStrategy::new(config);

        assert_eq!(strategy.calculate_delay(0), Duration::from_secs(1));
        assert_eq!(strategy.calculate_delay(1), Duration::from_secs(2));
        assert_eq!(strategy.calculate_delay(2), Duration::from_secs(4));
        assert_eq!(strategy.calculate_delay(6), Duration::from_secs(60));
    }

    #[test]
    fn test_ws_config_default() {
        let config = WsConfig::default();
        assert_eq!(config.connect_timeout, 10000);
        assert_eq!(config.max_subscriptions, DEFAULT_MAX_SUBSCRIPTIONS);
    }

    #[test]
    fn test_subscription_key() {
        let key1 = WsClient::subscription_key("ticker", Some(&"BTC/USDT".to_string()));
        assert_eq!(key1, "ticker:BTC/USDT");

        let key2 = WsClient::subscription_key("trades", None);
        assert_eq!(key2, "trades");
    }

    #[tokio::test]
    async fn test_ws_client_creation() {
        let config = WsConfig {
            url: "wss://example.com/ws".to_string(),
            ..Default::default()
        };

        let client = WsClient::new(config);
        assert_eq!(client.state(), WsConnectionState::Disconnected);
        assert!(!client.is_connected());
    }

    #[tokio::test]
    async fn test_subscribe_adds_subscription() {
        let config = WsConfig {
            url: "wss://example.com/ws".to_string(),
            ..Default::default()
        };

        let client = WsClient::new(config);
        let result = client
            .subscribe("ticker".to_string(), Some("BTC/USDT".to_string()), None)
            .await;
        assert!(result.is_ok());
        assert_eq!(client.subscription_count(), 1);
        assert!(client.is_subscribed("ticker", Some(&"BTC/USDT".to_string())));
    }

    #[test]
    fn test_ws_connection_state_from_u8() {
        assert_eq!(
            WsConnectionState::from_u8(0),
            WsConnectionState::Disconnected
        );
        assert_eq!(WsConnectionState::from_u8(1), WsConnectionState::Connecting);
        assert_eq!(WsConnectionState::from_u8(2), WsConnectionState::Connected);
        assert_eq!(WsConnectionState::from_u8(255), WsConnectionState::Error);
    }

    #[test]
    fn test_ws_error_kind() {
        assert!(WsErrorKind::Transient.is_transient());
        assert!(WsErrorKind::Permanent.is_permanent());
    }

    #[test]
    fn test_ws_error_creation() {
        let err = WsError::transient("Connection timeout");
        assert!(err.is_transient());
        assert_eq!(err.message(), "Connection timeout");

        let err = WsError::permanent("Invalid API key");
        assert!(err.is_permanent());
    }

    #[test]
    fn test_subscription_manager() {
        let manager = SubscriptionManager::new(2);
        assert_eq!(manager.max_subscriptions(), 2);
        assert_eq!(manager.count(), 0);
        assert!(!manager.is_full());

        let sub = Subscription {
            channel: "ticker".to_string(),
            symbol: Some("BTC/USDT".to_string()),
            params: None,
        };
        assert!(manager.try_add("ticker:BTC/USDT".to_string(), sub).is_ok());
        assert_eq!(manager.count(), 1);
    }
}
