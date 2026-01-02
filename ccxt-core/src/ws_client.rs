//! WebSocket client module.
//!
//! Provides asynchronous WebSocket connection management, subscription handling,
//! and heartbeat maintenance for cryptocurrency exchange streaming APIs.
//!
//! # Observability
//!
//! This module uses the `tracing` crate for structured logging. Key events:
//! - Connection establishment and disconnection
//! - Subscription and unsubscription events with stream names
//! - Message parsing failures with raw message preview (truncated)
//! - Reconnection attempts and outcomes
//! - Ping/pong heartbeat events

use crate::error::{Error, Result};
use dashmap::DashMap;
use futures_util::{SinkExt, StreamExt, stream::SplitSink};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU8, AtomicU32, Ordering};
use tokio::net::TcpStream;
use tokio::sync::{Mutex, RwLock, mpsc};
use tokio::task::JoinHandle;
use tokio::time::{Duration, interval};
use tokio_tungstenite::{
    MaybeTlsStream, WebSocketStream, connect_async, tungstenite::protocol::Message,
};
use tracing::{debug, error, info, instrument, warn};

/// WebSocket connection state.
///
/// Uses `#[repr(u8)]` to enable atomic storage via `AtomicU8`.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WsConnectionState {
    /// Not connected
    Disconnected = 0,
    /// Establishing connection
    Connecting = 1,
    /// Successfully connected
    Connected = 2,
    /// Attempting to reconnect
    Reconnecting = 3,
    /// Error state
    Error = 4,
}

impl WsConnectionState {
    /// Converts a `u8` value to `WsConnectionState`.
    ///
    /// # Arguments
    ///
    /// * `value` - The u8 value to convert
    ///
    /// # Returns
    ///
    /// The corresponding `WsConnectionState`, defaulting to `Error` for unknown values.
    #[inline]
    pub fn from_u8(value: u8) -> Self {
        match value {
            0 => Self::Disconnected,
            1 => Self::Connecting,
            2 => Self::Connected,
            3 => Self::Reconnecting,
            _ => Self::Error,
        }
    }

    /// Converts the `WsConnectionState` to its `u8` representation.
    #[inline]
    pub fn as_u8(self) -> u8 {
        self as u8
    }
}

/// WebSocket message types for exchange communication.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum WsMessage {
    /// Subscribe to a channel
    Subscribe {
        /// Channel name
        channel: String,
        /// Optional trading pair symbol
        symbol: Option<String>,
        /// Additional parameters
        params: Option<HashMap<String, Value>>,
    },
    /// Unsubscribe from a channel
    Unsubscribe {
        /// Channel name
        channel: String,
        /// Optional trading pair symbol
        symbol: Option<String>,
    },
    /// Ping message for keepalive
    Ping {
        /// Timestamp in milliseconds
        timestamp: i64,
    },
    /// Pong response to ping
    Pong {
        /// Timestamp in milliseconds
        timestamp: i64,
    },
    /// Authentication message
    Auth {
        /// API key
        api_key: String,
        /// HMAC signature
        signature: String,
        /// Timestamp in milliseconds
        timestamp: i64,
    },
    /// Custom message payload
    Custom(Value),
}

/// WebSocket connection configuration.
#[derive(Debug, Clone)]
pub struct WsConfig {
    /// WebSocket server URL
    pub url: String,
    /// Connection timeout in milliseconds
    pub connect_timeout: u64,
    /// Ping interval in milliseconds
    pub ping_interval: u64,
    /// Reconnection delay in milliseconds
    pub reconnect_interval: u64,
    /// Maximum reconnection attempts before giving up
    pub max_reconnect_attempts: u32,
    /// Enable automatic reconnection on disconnect
    pub auto_reconnect: bool,
    /// Enable message compression
    pub enable_compression: bool,
    /// Pong timeout in milliseconds
    ///
    /// Connection is considered dead if no pong received within this duration.
    pub pong_timeout: u64,
}

impl Default for WsConfig {
    fn default() -> Self {
        Self {
            url: String::new(),
            connect_timeout: 10000,
            ping_interval: 30000,
            reconnect_interval: 5000,
            max_reconnect_attempts: 5,
            auto_reconnect: true,
            enable_compression: false,
            pong_timeout: 90000,
        }
    }
}

/// WebSocket subscription metadata.
#[derive(Debug, Clone)]
pub struct Subscription {
    channel: String,
    symbol: Option<String>,
    params: Option<HashMap<String, Value>>,
}

/// Type alias for WebSocket write half.
#[allow(dead_code)]
type WsWriter = SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>;

/// Async WebSocket client for exchange streaming APIs.
pub struct WsClient {
    config: WsConfig,
    /// Connection state (atomic for lock-free reads)
    state: Arc<AtomicU8>,
    /// Subscription tracking using DashMap for lock-free concurrent access
    subscriptions: DashMap<String, Subscription>,

    message_tx: mpsc::UnboundedSender<Value>,
    message_rx: Arc<RwLock<mpsc::UnboundedReceiver<Value>>>,

    write_tx: Arc<Mutex<Option<mpsc::UnboundedSender<Message>>>>,

    /// Reconnection counter (atomic for lock-free access)
    reconnect_count: AtomicU32,

    shutdown_tx: Arc<Mutex<Option<mpsc::UnboundedSender<()>>>>,

    /// Stats still use RwLock (requires consistent multi-field updates)
    stats: Arc<RwLock<WsStats>>,
}

/// WebSocket connection statistics.
#[derive(Debug, Clone, Default)]
pub struct WsStats {
    /// Total messages received
    pub messages_received: u64,
    /// Total messages sent
    pub messages_sent: u64,
    /// Total bytes received
    pub bytes_received: u64,
    /// Total bytes sent
    pub bytes_sent: u64,
    /// Last message timestamp in milliseconds
    pub last_message_time: i64,
    /// Last ping timestamp in milliseconds
    pub last_ping_time: i64,
    /// Last pong timestamp in milliseconds
    pub last_pong_time: i64,
    /// Connection established timestamp in milliseconds
    pub connected_at: i64,
    /// Number of reconnection attempts
    pub reconnect_attempts: u32,
}

impl WsClient {
    /// Creates a new WebSocket client instance.
    ///
    /// # Arguments
    ///
    /// * `config` - WebSocket connection configuration
    ///
    /// # Returns
    ///
    /// A new `WsClient` instance ready to connect
    pub fn new(config: WsConfig) -> Self {
        let (message_tx, message_rx) = mpsc::unbounded_channel();

        Self {
            config,
            state: Arc::new(AtomicU8::new(WsConnectionState::Disconnected.as_u8())),
            subscriptions: DashMap::new(),
            message_tx,
            message_rx: Arc::new(RwLock::new(message_rx)),
            write_tx: Arc::new(Mutex::new(None)),
            reconnect_count: AtomicU32::new(0),
            shutdown_tx: Arc::new(Mutex::new(None)),
            stats: Arc::new(RwLock::new(WsStats::default())),
        }
    }

    /// Establishes connection to the WebSocket server.
    ///
    /// Returns immediately if already connected. Automatically starts message
    /// processing loop and resubscribes to previous channels on success.
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Connection timeout exceeded
    /// - Network error occurs
    /// - Server rejects connection
    #[instrument(
        name = "ws_connect",
        skip(self),
        fields(url = %self.config.url, timeout_ms = self.config.connect_timeout)
    )]
    pub async fn connect(&self) -> Result<()> {
        // Lock-free state check
        if self.state() == WsConnectionState::Connected {
            info!("WebSocket already connected");
            return Ok(());
        }

        // Lock-free state update
        self.set_state(WsConnectionState::Connecting);

        let url = self.config.url.clone();
        info!("Initiating WebSocket connection");

        match tokio::time::timeout(
            Duration::from_millis(self.config.connect_timeout),
            connect_async(&url),
        )
        .await
        {
            Ok(Ok((ws_stream, response))) => {
                info!(
                    status = response.status().as_u16(),
                    "WebSocket connection established successfully"
                );

                self.set_state(WsConnectionState::Connected);
                // Lock-free reconnect count reset
                self.reconnect_count.store(0, Ordering::Release);

                {
                    let mut stats = self.stats.write().await;
                    stats.connected_at = chrono::Utc::now().timestamp_millis();
                }

                self.start_message_loop(ws_stream).await;

                self.resubscribe_all().await?;

                Ok(())
            }
            Ok(Err(e)) => {
                error!(
                    error = %e,
                    error_debug = ?e,
                    "WebSocket connection failed"
                );
                self.set_state(WsConnectionState::Error);
                Err(Error::network(format!("WebSocket connection failed: {e}")))
            }
            Err(_) => {
                error!(
                    timeout_ms = self.config.connect_timeout,
                    "WebSocket connection timeout exceeded"
                );
                self.set_state(WsConnectionState::Error);
                Err(Error::timeout("WebSocket connection timeout"))
            }
        }
    }

    /// Closes the WebSocket connection gracefully.
    ///
    /// Sends shutdown signal to background tasks and clears internal state.
    #[instrument(name = "ws_disconnect", skip(self))]
    pub async fn disconnect(&self) -> Result<()> {
        info!("Initiating WebSocket disconnect");

        if let Some(tx) = self.shutdown_tx.lock().await.as_ref() {
            let _ = tx.send(());
            debug!("Shutdown signal sent to background tasks");
        }

        *self.write_tx.lock().await = None;

        // Lock-free state update
        self.set_state(WsConnectionState::Disconnected);

        info!("WebSocket disconnected successfully");
        Ok(())
    }

    /// Attempts to reconnect to the WebSocket server.
    ///
    /// Respects `max_reconnect_attempts` configuration and waits for
    /// `reconnect_interval` before attempting connection.
    ///
    /// # Errors
    ///
    /// Returns error if maximum reconnection attempts exceeded or connection fails.
    #[instrument(
        name = "ws_reconnect",
        skip(self),
        fields(
            max_attempts = self.config.max_reconnect_attempts,
            reconnect_interval_ms = self.config.reconnect_interval
        )
    )]
    pub async fn reconnect(&self) -> Result<()> {
        // Lock-free atomic increment and check
        let count = self.reconnect_count.fetch_add(1, Ordering::AcqRel) + 1;

        if count > self.config.max_reconnect_attempts {
            error!(
                attempts = count,
                max = self.config.max_reconnect_attempts,
                "Max reconnect attempts reached, giving up"
            );
            return Err(Error::network("Max reconnect attempts reached"));
        }

        warn!(
            attempt = count,
            max = self.config.max_reconnect_attempts,
            delay_ms = self.config.reconnect_interval,
            "Attempting WebSocket reconnection"
        );

        // Lock-free state update
        self.set_state(WsConnectionState::Reconnecting);

        tokio::time::sleep(Duration::from_millis(self.config.reconnect_interval)).await;

        self.connect().await
    }

    /// Returns the current reconnection attempt count (lock-free).
    #[inline]
    pub fn reconnect_count(&self) -> u32 {
        self.reconnect_count.load(Ordering::Acquire)
    }

    /// Resets the reconnection attempt counter to zero (lock-free).
    pub fn reset_reconnect_count(&self) {
        self.reconnect_count.store(0, Ordering::Release);
        debug!("Reconnect count reset");
    }

    /// Returns a snapshot of connection statistics.
    pub async fn stats(&self) -> WsStats {
        self.stats.read().await.clone()
    }

    /// Resets all connection statistics to default values.
    pub async fn reset_stats(&self) {
        *self.stats.write().await = WsStats::default();
        debug!("Stats reset");
    }

    /// Calculates current connection latency in milliseconds.
    ///
    /// # Returns
    ///
    /// Time difference between last pong and ping, or `None` if no data available.
    pub async fn latency(&self) -> Option<i64> {
        let stats = self.stats.read().await;
        if stats.last_pong_time > 0 && stats.last_ping_time > 0 {
            Some(stats.last_pong_time - stats.last_ping_time)
        } else {
            None
        }
    }

    /// Creates an automatic reconnection coordinator.
    ///
    /// # Returns
    ///
    /// A new [`AutoReconnectCoordinator`] instance for managing reconnection logic.
    pub fn create_auto_reconnect_coordinator(self: Arc<Self>) -> AutoReconnectCoordinator {
        AutoReconnectCoordinator::new(self)
    }

    /// Subscribes to a WebSocket channel.
    ///
    /// Subscription is persisted and automatically reestablished on reconnection.
    ///
    /// # Arguments
    ///
    /// * `channel` - Channel name to subscribe to
    /// * `symbol` - Optional trading pair symbol
    /// * `params` - Optional additional subscription parameters
    ///
    /// # Errors
    ///
    /// Returns error if subscription message fails to send.
    #[instrument(
        name = "ws_subscribe",
        skip(self, params),
        fields(channel = %channel, symbol = ?symbol)
    )]
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

        // DashMap insert is lock-free for reads
        self.subscriptions.insert(sub_key.clone(), subscription);

        info!(subscription_key = %sub_key, "Subscription registered");

        // Lock-free state check
        let state = self.state();
        if state == WsConnectionState::Connected {
            self.send_subscribe_message(channel, symbol, params).await?;
            info!(subscription_key = %sub_key, "Subscription message sent");
        } else {
            debug!(
                subscription_key = %sub_key,
                state = ?state,
                "Subscription queued (not connected)"
            );
        }

        Ok(())
    }

    /// Unsubscribes from a WebSocket channel.
    ///
    /// Removes subscription from internal state and sends unsubscribe message if connected.
    ///
    /// # Arguments
    ///
    /// * `channel` - Channel name to unsubscribe from
    /// * `symbol` - Optional trading pair symbol
    ///
    /// # Errors
    ///
    /// Returns error if unsubscribe message fails to send.
    #[instrument(
        name = "ws_unsubscribe",
        skip(self),
        fields(channel = %channel, symbol = ?symbol)
    )]
    pub async fn unsubscribe(&self, channel: String, symbol: Option<String>) -> Result<()> {
        let sub_key = Self::subscription_key(&channel, symbol.as_ref());

        // DashMap remove is lock-free for reads
        self.subscriptions.remove(&sub_key);

        info!(subscription_key = %sub_key, "Subscription removed");

        // Lock-free state check
        let state = self.state();
        if state == WsConnectionState::Connected {
            self.send_unsubscribe_message(channel, symbol).await?;
            info!(subscription_key = %sub_key, "Unsubscribe message sent");
        }

        Ok(())
    }

    /// Receives the next available message from the WebSocket stream.
    ///
    /// # Returns
    ///
    /// The received JSON message, or `None` if the channel is closed.
    pub async fn receive(&self) -> Option<Value> {
        let mut rx = self.message_rx.write().await;
        rx.recv().await
    }

    /// Returns the current connection state (lock-free).
    #[inline]
    pub fn state(&self) -> WsConnectionState {
        WsConnectionState::from_u8(self.state.load(Ordering::Acquire))
    }

    /// Sets the connection state (lock-free).
    #[inline]
    fn set_state(&self, state: WsConnectionState) {
        self.state.store(state.as_u8(), Ordering::Release);
    }

    /// Checks whether the WebSocket is currently connected (lock-free).
    #[inline]
    pub fn is_connected(&self) -> bool {
        self.state() == WsConnectionState::Connected
    }

    /// Checks if subscribed to a specific channel (lock-free).
    ///
    /// # Arguments
    ///
    /// * `channel` - Channel name to check
    /// * `symbol` - Optional trading pair symbol
    ///
    /// # Returns
    ///
    /// `true` if subscribed to the channel, `false` otherwise.
    pub fn is_subscribed(&self, channel: &str, symbol: Option<&String>) -> bool {
        let sub_key = Self::subscription_key(channel, symbol);
        self.subscriptions.contains_key(&sub_key)
    }

    /// Returns the number of active subscriptions (lock-free).
    pub fn subscription_count(&self) -> usize {
        self.subscriptions.len()
    }

    /// Sends a raw WebSocket message.
    ///
    /// # Arguments
    ///
    /// * `message` - WebSocket message to send
    ///
    /// # Errors
    ///
    /// Returns error if not connected or message transmission fails.
    #[instrument(name = "ws_send", skip(self, message))]
    pub async fn send(&self, message: Message) -> Result<()> {
        let tx = self.write_tx.lock().await;

        if let Some(sender) = tx.as_ref() {
            sender.send(message).map_err(|e| {
                error!(
                    error = %e,
                    "Failed to send WebSocket message"
                );
                Error::network(format!("Failed to send message: {e}"))
            })?;
            debug!("WebSocket message sent successfully");
            Ok(())
        } else {
            warn!("WebSocket not connected, cannot send message");
            Err(Error::network("WebSocket not connected"))
        }
    }

    /// Sends a text message over the WebSocket connection.
    ///
    /// # Arguments
    ///
    /// * `text` - Text content to send
    ///
    /// # Errors
    ///
    /// Returns error if not connected or transmission fails.
    #[instrument(name = "ws_send_text", skip(self, text), fields(text_len = text.len()))]
    pub async fn send_text(&self, text: String) -> Result<()> {
        self.send(Message::Text(text.into())).await
    }

    /// Sends a JSON-encoded message over the WebSocket connection.
    ///
    /// # Arguments
    ///
    /// * `json` - JSON value to serialize and send
    ///
    /// # Errors
    ///
    /// Returns error if serialization fails, not connected, or transmission fails.
    #[instrument(name = "ws_send_json", skip(self, json))]
    pub async fn send_json(&self, json: &Value) -> Result<()> {
        let text = serde_json::to_string(json).map_err(|e| {
            error!(error = %e, "Failed to serialize JSON for WebSocket");
            Error::from(e)
        })?;
        self.send_text(text).await
    }

    /// Generates a unique subscription key from channel and symbol.
    fn subscription_key(channel: &str, symbol: Option<&String>) -> String {
        match symbol {
            Some(s) => format!("{channel}:{s}"),
            None => channel.to_string(),
        }
    }

    /// Starts the WebSocket message processing loop.
    ///
    /// Spawns separate tasks for reading and writing messages, handling shutdown signals.
    async fn start_message_loop(&self, ws_stream: WebSocketStream<MaybeTlsStream<TcpStream>>) {
        let (write, mut read) = ws_stream.split();

        let (write_tx, mut write_rx) = mpsc::unbounded_channel::<Message>();
        *self.write_tx.lock().await = Some(write_tx.clone());

        let (shutdown_tx, mut shutdown_rx) = mpsc::unbounded_channel::<()>();
        *self.shutdown_tx.lock().await = Some(shutdown_tx);

        let state = Arc::clone(&self.state);
        let message_tx = self.message_tx.clone();
        let ping_interval_ms = self.config.ping_interval;

        info!("Starting WebSocket message loop");

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
                        debug!("Write task received shutdown signal");
                        let _ = write.send(Message::Close(None)).await;
                        break;
                    }
                }
            }
            debug!("Write task terminated");
        });

        let state_clone = Arc::clone(&state);
        let ws_stats = Arc::clone(&self.stats);
        let read_handle = tokio::spawn(async move {
            debug!("Starting WebSocket read task");
            while let Some(msg_result) = read.next().await {
                match msg_result {
                    Ok(Message::Text(text)) => {
                        debug!(len = text.len(), "Received text message");

                        {
                            let mut stats_guard = ws_stats.write().await;
                            stats_guard.messages_received += 1;
                            stats_guard.bytes_received += text.len() as u64;
                            stats_guard.last_message_time = chrono::Utc::now().timestamp_millis();
                        }

                        match serde_json::from_str::<Value>(&text) {
                            Ok(json) => {
                                let _ = message_tx.send(json);
                            }
                            Err(e) => {
                                // Log parse failure with truncated raw message preview
                                let raw_preview: String = text.chars().take(200).collect();
                                warn!(
                                    error = %e,
                                    raw_message_preview = %raw_preview,
                                    raw_message_len = text.len(),
                                    "Failed to parse WebSocket text message as JSON"
                                );
                            }
                        }
                    }
                    Ok(Message::Binary(data)) => {
                        debug!(len = data.len(), "Received binary message");

                        {
                            let mut stats_guard = ws_stats.write().await;
                            stats_guard.messages_received += 1;
                            stats_guard.bytes_received += data.len() as u64;
                            stats_guard.last_message_time = chrono::Utc::now().timestamp_millis();
                        }

                        match String::from_utf8(data.to_vec()) {
                            Ok(text) => {
                                match serde_json::from_str::<Value>(&text) {
                                    Ok(json) => {
                                        let _ = message_tx.send(json);
                                    }
                                    Err(e) => {
                                        // Log parse failure with truncated raw message preview
                                        let raw_preview: String = text.chars().take(200).collect();
                                        warn!(
                                            error = %e,
                                            raw_message_preview = %raw_preview,
                                            raw_message_len = text.len(),
                                            "Failed to parse WebSocket binary message as JSON"
                                        );
                                    }
                                }
                            }
                            Err(e) => {
                                // Log UTF-8 decode failure with hex preview
                                let hex_preview: String = data
                                    .iter()
                                    .take(50)
                                    .map(|b| format!("{b:02x}"))
                                    .collect::<Vec<_>>()
                                    .join(" ");
                                warn!(
                                    error = %e,
                                    hex_preview = %hex_preview,
                                    data_len = data.len(),
                                    "Failed to decode WebSocket binary message as UTF-8"
                                );
                            }
                        }
                    }
                    Ok(Message::Ping(_)) => {
                        debug!("Received ping, auto-responding with pong");
                    }
                    Ok(Message::Pong(_)) => {
                        debug!("Received pong");

                        {
                            let mut stats_guard = ws_stats.write().await;
                            stats_guard.last_pong_time = chrono::Utc::now().timestamp_millis();
                        }
                    }
                    Ok(Message::Close(frame)) => {
                        info!(
                            close_frame = ?frame,
                            "Received WebSocket close frame"
                        );
                        // Lock-free state update
                        state_clone
                            .store(WsConnectionState::Disconnected.as_u8(), Ordering::Release);
                        break;
                    }
                    Err(e) => {
                        error!(
                            error = %e,
                            error_debug = ?e,
                            "WebSocket read error"
                        );
                        // Lock-free state update
                        state_clone.store(WsConnectionState::Error.as_u8(), Ordering::Release);
                        break;
                    }
                    _ => {
                        debug!("Received other WebSocket message type");
                    }
                }
            }
            debug!("WebSocket read task terminated");
        });

        if ping_interval_ms > 0 {
            let write_tx_clone = write_tx.clone();
            let ping_stats = Arc::clone(&self.stats);
            let ping_state = Arc::clone(&state);
            let pong_timeout_ms = self.config.pong_timeout;

            tokio::spawn(async move {
                let mut interval = interval(Duration::from_millis(ping_interval_ms));
                debug!(
                    interval_ms = ping_interval_ms,
                    timeout_ms = pong_timeout_ms,
                    "Starting ping task with timeout detection"
                );

                loop {
                    interval.tick().await;

                    let now = chrono::Utc::now().timestamp_millis();
                    let last_pong = {
                        let stats_guard = ping_stats.read().await;
                        stats_guard.last_pong_time
                    };

                    if last_pong > 0 {
                        let elapsed = now - last_pong;
                        #[allow(clippy::cast_possible_wrap)]
                        if elapsed > pong_timeout_ms as i64 {
                            warn!(
                                elapsed_ms = elapsed,
                                timeout_ms = pong_timeout_ms,
                                "Pong timeout detected, marking connection as error"
                            );
                            // Lock-free state update
                            ping_state.store(WsConnectionState::Error.as_u8(), Ordering::Release);
                            break;
                        }
                    }

                    {
                        let mut stats_guard = ping_stats.write().await;
                        stats_guard.last_ping_time = now;
                    }

                    if write_tx_clone.send(Message::Ping(vec![].into())).is_err() {
                        debug!("Ping task: write channel closed");
                        break;
                    }
                    debug!("Sent ping");
                }
                debug!("Ping task terminated");
            });
        }

        tokio::spawn(async move {
            let _ = tokio::join!(write_handle, read_handle);
            info!("All WebSocket tasks completed");
        });
    }

    /// Sends a subscription message to the WebSocket server.
    #[instrument(
        name = "ws_send_subscribe",
        skip(self, params),
        fields(channel = %channel, symbol = ?symbol)
    )]
    async fn send_subscribe_message(
        &self,
        channel: String,
        symbol: Option<String>,
        params: Option<HashMap<String, Value>>,
    ) -> Result<()> {
        let msg = WsMessage::Subscribe {
            channel: channel.clone(),
            symbol: symbol.clone(),
            params,
        };

        let json = serde_json::to_value(&msg).map_err(|e| {
            error!(error = %e, "Failed to serialize subscribe message");
            Error::from(e)
        })?;

        debug!("Sending subscribe message to server");

        self.send_json(&json).await?;
        info!("Subscribe message sent successfully");
        Ok(())
    }

    /// Sends an unsubscribe message to the WebSocket server.
    #[instrument(
        name = "ws_send_unsubscribe",
        skip(self),
        fields(channel = %channel, symbol = ?symbol)
    )]
    async fn send_unsubscribe_message(
        &self,
        channel: String,
        symbol: Option<String>,
    ) -> Result<()> {
        let msg = WsMessage::Unsubscribe {
            channel: channel.clone(),
            symbol: symbol.clone(),
        };

        let json = serde_json::to_value(&msg).map_err(|e| {
            error!(error = %e, "Failed to serialize unsubscribe message");
            Error::from(e)
        })?;

        debug!("Sending unsubscribe message to server");

        self.send_json(&json).await?;
        info!("Unsubscribe message sent successfully");
        Ok(())
    }

    /// Resubscribes to all previously subscribed channels.
    async fn resubscribe_all(&self) -> Result<()> {
        // Collect subscriptions to avoid holding DashMap reference during async calls
        let subs: Vec<Subscription> = self
            .subscriptions
            .iter()
            .map(|entry| entry.value().clone())
            .collect();

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
/// WebSocket connection event types.
#[derive(Debug, Clone)]
pub enum WsEvent {
    /// Connection established successfully
    Connected,
    /// Connection closed
    Disconnected,
    /// Reconnection in progress
    Reconnecting {
        /// Current reconnection attempt number
        attempt: u32,
    },
    /// Reconnection succeeded
    ReconnectSuccess,
    /// Reconnection failed
    ReconnectFailed {
        /// Error message
        error: String,
    },
    /// Subscriptions restored after reconnection
    SubscriptionRestored,
}

/// Event callback function type.
pub type WsEventCallback = Arc<dyn Fn(WsEvent) + Send + Sync>;

/// Automatic reconnection coordinator for WebSocket connections.
///
/// Monitors connection state and triggers reconnection attempts when disconnected.
pub struct AutoReconnectCoordinator {
    client: Arc<WsClient>,
    enabled: Arc<AtomicBool>,
    reconnect_task: Arc<Mutex<Option<JoinHandle<()>>>>,
    event_callback: Option<WsEventCallback>,
}

impl AutoReconnectCoordinator {
    /// Creates a new automatic reconnection coordinator.
    ///
    /// # Arguments
    ///
    /// * `client` - Arc reference to the WebSocket client
    pub fn new(client: Arc<WsClient>) -> Self {
        Self {
            client,
            enabled: Arc::new(AtomicBool::new(false)),
            reconnect_task: Arc::new(Mutex::new(None)),
            event_callback: None,
        }
    }

    /// 设置事件回调
    ///
    /// # Arguments
    /// * `callback` - 事件回调函数
    ///
    /// # Returns
    /// Self，用于链式调用
    pub fn with_callback(mut self, callback: WsEventCallback) -> Self {
        self.event_callback = Some(callback);
        self
    }

    /// Starts the automatic reconnection coordinator.
    ///
    /// Begins monitoring connection state and automatically reconnects on disconnect.
    pub async fn start(&self) {
        if self.enabled.swap(true, Ordering::SeqCst) {
            info!("Auto-reconnect already started");
            return;
        }

        info!("Starting auto-reconnect coordinator");

        let client = Arc::clone(&self.client);
        let enabled = Arc::clone(&self.enabled);
        let callback = self.event_callback.clone();

        let handle = tokio::spawn(async move {
            Self::reconnect_loop(client, enabled, callback).await;
        });

        *self.reconnect_task.lock().await = Some(handle);
    }

    /// Stops the automatic reconnection coordinator.
    ///
    /// Halts monitoring and reconnection tasks.
    pub async fn stop(&self) {
        if !self.enabled.swap(false, Ordering::SeqCst) {
            info!("Auto-reconnect already stopped");
            return;
        }

        info!("Stopping auto-reconnect coordinator");

        let mut task = self.reconnect_task.lock().await;
        if let Some(handle) = task.take() {
            handle.abort();
        }
    }

    /// Internal reconnection loop.
    ///
    /// Continuously monitors connection state and triggers reconnection
    /// when `Error` or `Disconnected` state is detected.
    async fn reconnect_loop(
        client: Arc<WsClient>,
        enabled: Arc<AtomicBool>,
        callback: Option<WsEventCallback>,
    ) {
        let mut check_interval = interval(Duration::from_secs(1));

        loop {
            check_interval.tick().await;

            if !enabled.load(Ordering::SeqCst) {
                debug!("Auto-reconnect disabled, exiting loop");
                break;
            }

            // Lock-free state check
            let state = client.state();

            if matches!(
                state,
                WsConnectionState::Disconnected | WsConnectionState::Error
            ) {
                // Lock-free reconnect count access
                let attempt = client.reconnect_count();

                info!(
                    attempt = attempt + 1,
                    state = ?state,
                    "Connection lost, attempting reconnect"
                );

                if let Some(ref cb) = callback {
                    cb(WsEvent::Reconnecting {
                        attempt: attempt + 1,
                    });
                }

                match client.reconnect().await {
                    Ok(()) => {
                        info!("Reconnection successful");

                        if let Some(ref cb) = callback {
                            cb(WsEvent::ReconnectSuccess);
                        }

                        match client.resubscribe_all().await {
                            Ok(()) => {
                                info!("Subscriptions restored");
                                if let Some(ref cb) = callback {
                                    cb(WsEvent::SubscriptionRestored);
                                }
                            }
                            Err(e) => {
                                error!(error = %e, "Failed to restore subscriptions");
                            }
                        }
                    }
                    Err(e) => {
                        error!(error = %e, "Reconnection failed");

                        if let Some(ref cb) = callback {
                            cb(WsEvent::ReconnectFailed {
                                error: e.to_string(),
                            });
                        }

                        tokio::time::sleep(Duration::from_secs(5)).await;
                    }
                }
            }
        }

        info!("Auto-reconnect loop terminated");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ws_config_default() {
        let config = WsConfig::default();
        assert_eq!(config.connect_timeout, 10000);
        assert_eq!(config.ping_interval, 30000);
        assert_eq!(config.reconnect_interval, 5000);
        assert_eq!(config.max_reconnect_attempts, 5);
        assert!(config.auto_reconnect);
        assert!(!config.enable_compression);
        assert_eq!(config.pong_timeout, 90000);
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
        // state() is now lock-free (no await needed)
        assert_eq!(client.state(), WsConnectionState::Disconnected);
        // is_connected() is now lock-free (no await needed)
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

        // Use DashMap API (lock-free)
        assert_eq!(client.subscription_count(), 1);
        assert!(client.is_subscribed("ticker", Some(&"BTC/USDT".to_string())));
    }

    #[tokio::test]
    async fn test_unsubscribe_removes_subscription() {
        let config = WsConfig {
            url: "wss://example.com/ws".to_string(),
            ..Default::default()
        };

        let client = WsClient::new(config);

        client
            .subscribe("ticker".to_string(), Some("BTC/USDT".to_string()), None)
            .await
            .unwrap();

        let result = client
            .unsubscribe("ticker".to_string(), Some("BTC/USDT".to_string()))
            .await;
        assert!(result.is_ok());

        // Use DashMap API (lock-free)
        assert_eq!(client.subscription_count(), 0);
        assert!(!client.is_subscribed("ticker", Some(&"BTC/USDT".to_string())));
    }

    #[test]
    fn test_ws_message_serialization() {
        let msg = WsMessage::Subscribe {
            channel: "ticker".to_string(),
            symbol: Some("BTC/USDT".to_string()),
            params: None,
        };

        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"type\":\"subscribe\""));
        assert!(json.contains("\"channel\":\"ticker\""));
    }

    #[test]
    fn test_ws_connection_state_from_u8() {
        assert_eq!(
            WsConnectionState::from_u8(0),
            WsConnectionState::Disconnected
        );
        assert_eq!(WsConnectionState::from_u8(1), WsConnectionState::Connecting);
        assert_eq!(WsConnectionState::from_u8(2), WsConnectionState::Connected);
        assert_eq!(
            WsConnectionState::from_u8(3),
            WsConnectionState::Reconnecting
        );
        assert_eq!(WsConnectionState::from_u8(4), WsConnectionState::Error);
        // Unknown values default to Error
        assert_eq!(WsConnectionState::from_u8(5), WsConnectionState::Error);
        assert_eq!(WsConnectionState::from_u8(255), WsConnectionState::Error);
    }

    #[test]
    fn test_ws_connection_state_as_u8() {
        assert_eq!(WsConnectionState::Disconnected.as_u8(), 0);
        assert_eq!(WsConnectionState::Connecting.as_u8(), 1);
        assert_eq!(WsConnectionState::Connected.as_u8(), 2);
        assert_eq!(WsConnectionState::Reconnecting.as_u8(), 3);
        assert_eq!(WsConnectionState::Error.as_u8(), 4);
    }

    #[test]
    fn test_reconnect_count_lock_free() {
        let config = WsConfig {
            url: "wss://example.com/ws".to_string(),
            ..Default::default()
        };

        let client = WsClient::new(config);

        // Initial count should be 0
        assert_eq!(client.reconnect_count(), 0);

        // Reset should work
        client.reset_reconnect_count();
        assert_eq!(client.reconnect_count(), 0);
    }

    #[tokio::test]
    async fn test_concurrent_subscription_operations() {
        use std::sync::Arc;

        let config = WsConfig {
            url: "wss://example.com/ws".to_string(),
            ..Default::default()
        };

        let client = Arc::new(WsClient::new(config));

        // Spawn multiple tasks that add subscriptions concurrently
        let mut handles = vec![];

        for i in 0..10 {
            let client_clone = Arc::clone(&client);
            let handle = tokio::spawn(async move {
                let channel = format!("channel{}", i);
                let symbol = Some(format!("SYMBOL{}/USDT", i));
                client_clone.subscribe(channel, symbol, None).await.unwrap();
            });
            handles.push(handle);
        }

        // Wait for all tasks to complete
        for handle in handles {
            handle.await.unwrap();
        }

        // All subscriptions should be present
        assert_eq!(client.subscription_count(), 10);

        // Verify each subscription exists
        for i in 0..10 {
            assert!(
                client.is_subscribed(&format!("channel{}", i), Some(&format!("SYMBOL{}/USDT", i)))
            );
        }
    }

    #[tokio::test]
    async fn test_concurrent_state_access() {
        use std::sync::Arc;

        let config = WsConfig {
            url: "wss://example.com/ws".to_string(),
            ..Default::default()
        };

        let client = Arc::new(WsClient::new(config));

        // Spawn multiple tasks that read state concurrently
        let mut handles = vec![];

        for _ in 0..100 {
            let client_clone = Arc::clone(&client);
            let handle = tokio::spawn(async move {
                // Lock-free state access should not panic or deadlock
                let _ = client_clone.state();
                let _ = client_clone.is_connected();
            });
            handles.push(handle);
        }

        // Wait for all tasks to complete
        for handle in handles {
            handle.await.unwrap();
        }

        // State should still be Disconnected
        assert_eq!(client.state(), WsConnectionState::Disconnected);
    }

    #[tokio::test]
    async fn test_concurrent_subscription_add_remove() {
        use std::sync::Arc;

        let config = WsConfig {
            url: "wss://example.com/ws".to_string(),
            ..Default::default()
        };

        let client = Arc::new(WsClient::new(config));

        // Add some initial subscriptions
        for i in 0..5 {
            client
                .subscribe(
                    format!("channel{}", i),
                    Some(format!("SYM{}/USDT", i)),
                    None,
                )
                .await
                .unwrap();
        }

        // Spawn tasks that add and remove subscriptions concurrently
        let mut handles = vec![];

        // Add new subscriptions
        for i in 5..10 {
            let client_clone = Arc::clone(&client);
            let handle = tokio::spawn(async move {
                client_clone
                    .subscribe(
                        format!("channel{}", i),
                        Some(format!("SYM{}/USDT", i)),
                        None,
                    )
                    .await
                    .unwrap();
            });
            handles.push(handle);
        }

        // Remove some existing subscriptions
        for i in 0..3 {
            let client_clone = Arc::clone(&client);
            let handle = tokio::spawn(async move {
                client_clone
                    .unsubscribe(format!("channel{}", i), Some(format!("SYM{}/USDT", i)))
                    .await
                    .unwrap();
            });
            handles.push(handle);
        }

        // Wait for all tasks to complete
        for handle in handles {
            handle.await.unwrap();
        }

        // Should have 7 subscriptions (5 initial + 5 added - 3 removed)
        assert_eq!(client.subscription_count(), 7);
    }
}
