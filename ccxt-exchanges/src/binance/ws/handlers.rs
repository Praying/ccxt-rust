//! WebSocket message handlers

#![allow(dead_code)]

use crate::binance::Binance;
use ccxt_core::error::{Error, Result};
use ccxt_core::types::OrderBook;
use ccxt_core::types::financial::{Amount, Price};
use ccxt_core::types::orderbook::{OrderBookDelta, OrderBookEntry};
use rust_decimal::Decimal;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

struct MessageLoopContext {
    ws_client: Arc<tokio::sync::RwLock<Option<ccxt_core::ws_client::WsClient>>>,
    subscription_manager: Arc<super::subscriptions::SubscriptionManager>,
    is_connected: Arc<std::sync::atomic::AtomicBool>,
    reconnect_config: Arc<tokio::sync::RwLock<super::subscriptions::ReconnectConfig>>,
    request_id: Arc<std::sync::atomic::AtomicU64>,
    listen_key_manager: Option<Arc<super::listen_key::ListenKeyManager>>,
    base_url: String,
    current_url: String,
    ready_tx: Option<tokio::sync::oneshot::Sender<()>>,
}

/// Message router for handling WebSocket connections and message routing
pub struct MessageRouter {
    /// WebSocket client instance
    ws_client: Arc<tokio::sync::RwLock<Option<ccxt_core::ws_client::WsClient>>>,

    /// Subscription manager registry
    subscription_manager: Arc<super::subscriptions::SubscriptionManager>,

    /// Handle to the background routing task
    router_task: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,

    /// Connection state flag
    is_connected: Arc<std::sync::atomic::AtomicBool>,

    /// Connection lock to ensure idempotent start
    connection_lock: Arc<Mutex<()>>,

    /// Configuration for reconnection behavior
    reconnect_config: Arc<tokio::sync::RwLock<super::subscriptions::ReconnectConfig>>,

    /// Listen key manager for user data streams
    listen_key_manager: Option<Arc<super::listen_key::ListenKeyManager>>,

    /// WebSocket endpoint URL
    ws_url: String,

    /// Request ID counter (used for subscribe/unsubscribe)
    request_id: Arc<std::sync::atomic::AtomicU64>,
}

impl MessageRouter {
    /// Creates a new message router
    pub fn new(
        ws_url: String,
        subscription_manager: Arc<super::subscriptions::SubscriptionManager>,
        listen_key_manager: Option<Arc<super::listen_key::ListenKeyManager>>,
    ) -> Self {
        Self {
            ws_client: Arc::new(tokio::sync::RwLock::new(None)),
            subscription_manager,
            router_task: Arc::new(Mutex::new(None)),
            is_connected: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            connection_lock: Arc::new(Mutex::new(())),
            reconnect_config: Arc::new(tokio::sync::RwLock::new(
                super::subscriptions::ReconnectConfig::default(),
            )),
            listen_key_manager,
            ws_url,
            request_id: Arc::new(std::sync::atomic::AtomicU64::new(1)),
        }
    }

    /// Starts the message router
    pub async fn start(&self, url_override: Option<String>) -> Result<()> {
        let _lock = self.connection_lock.lock().await;

        if self.is_connected() {
            if url_override.is_some() {
                self.stop().await?;
            } else {
                return Ok(());
            }
        }

        let url = url_override.unwrap_or_else(|| self.ws_url.clone());
        let config = ccxt_core::ws_client::WsConfig {
            url: url.clone(),
            ..Default::default()
        };
        let client = ccxt_core::ws_client::WsClient::new(config);
        client.connect().await?;

        *self.ws_client.write().await = Some(client);

        let (ready_tx, ready_rx) = tokio::sync::oneshot::channel();

        let ws_client = self.ws_client.clone();
        let subscription_manager = self.subscription_manager.clone();
        let is_connected = self.is_connected.clone();
        let reconnect_config = self.reconnect_config.clone();
        let request_id = self.request_id.clone();
        let listen_key_manager = self.listen_key_manager.clone();

        let ws_url = self.ws_url.clone();
        let current_url = url;

        let ctx = MessageLoopContext {
            ws_client,
            subscription_manager,
            is_connected,
            reconnect_config,
            request_id,
            listen_key_manager,
            base_url: ws_url,
            current_url,
            ready_tx: Some(ready_tx),
        };

        let handle = tokio::spawn(async move {
            Self::message_loop(ctx).await;
        });

        *self.router_task.lock().await = Some(handle);

        // Wait for the message loop to signal readiness (with timeout)
        match tokio::time::timeout(Duration::from_secs(5), ready_rx).await {
            Ok(Ok(())) => {
                self.is_connected
                    .store(true, std::sync::atomic::Ordering::SeqCst);
                Ok(())
            }
            Ok(Err(_)) => {
                // oneshot sender was dropped — message loop task died
                self.is_connected
                    .store(false, std::sync::atomic::Ordering::SeqCst);
                Err(Error::network("Message loop failed to start"))
            }
            Err(_) => {
                // Timeout waiting for readiness — assume loop is running
                // (it may just not have received any messages yet)
                self.is_connected
                    .store(true, std::sync::atomic::Ordering::SeqCst);
                Ok(())
            }
        }
    }

    /// Stops the message router
    pub async fn stop(&self) -> Result<()> {
        self.is_connected
            .store(false, std::sync::atomic::Ordering::SeqCst);

        let mut task_opt = self.router_task.lock().await;
        if let Some(handle) = task_opt.take() {
            handle.abort();
        }

        let mut client_opt = self.ws_client.write().await;
        if let Some(client) = client_opt.take() {
            let _ = client.disconnect().await;
        }

        Ok(())
    }

    /// Restarts the message router
    pub async fn restart(&self) -> Result<()> {
        self.stop().await?;
        tokio::time::sleep(Duration::from_millis(100)).await;
        self.start(None).await
    }

    /// Returns the configured WebSocket URL
    pub fn get_url(&self) -> String {
        self.ws_url.clone()
    }

    /// Returns the current connection state
    pub fn is_connected(&self) -> bool {
        self.is_connected.load(std::sync::atomic::Ordering::SeqCst)
    }

    /// Returns the current connection latency in milliseconds.
    ///
    /// This is calculated from the ping/pong round-trip time.
    /// Returns None if no latency data is available.
    pub fn latency(&self) -> Option<i64> {
        // Try to get latency from the underlying WsClient
        // This requires a blocking read, so we use try_read
        if let Ok(guard) = self.ws_client.try_read() {
            if let Some(ref client) = *guard {
                return client.latency();
            }
        }
        None
    }

    /// Returns the number of reconnection attempts.
    pub fn reconnect_count(&self) -> u32 {
        if let Ok(guard) = self.ws_client.try_read() {
            if let Some(ref client) = *guard {
                return client.reconnect_count();
            }
        }
        0
    }

    /// Applies a new reconnection configuration
    pub async fn set_reconnect_config(&self, config: super::subscriptions::ReconnectConfig) {
        *self.reconnect_config.write().await = config;
    }

    /// Retrieves the current reconnection configuration
    pub async fn get_reconnect_config(&self) -> super::subscriptions::ReconnectConfig {
        self.reconnect_config.read().await.clone()
    }

    /// Subscribes to the provided streams
    pub async fn subscribe(&self, streams: Vec<String>) -> Result<()> {
        if streams.is_empty() {
            return Ok(());
        }

        let client_opt = self.ws_client.read().await;
        let client = client_opt
            .as_ref()
            .ok_or_else(|| Error::network("WebSocket not connected"))?;

        let id = self
            .request_id
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);

        #[allow(clippy::disallowed_methods)]
        let request = serde_json::json!({
            "method": "SUBSCRIBE",
            "params": streams,
            "id": id
        });

        client
            .send(tokio_tungstenite::tungstenite::protocol::Message::Text(
                request.to_string().into(),
            ))
            .await?;

        Ok(())
    }

    /// Unsubscribes from the provided streams
    pub async fn unsubscribe(&self, streams: Vec<String>) -> Result<()> {
        if streams.is_empty() {
            return Ok(());
        }

        let client_opt = self.ws_client.read().await;
        let client = client_opt
            .as_ref()
            .ok_or_else(|| Error::network("WebSocket not connected"))?;

        let id = self
            .request_id
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);

        #[allow(clippy::disallowed_methods)]
        let request = serde_json::json!({
            "method": "UNSUBSCRIBE",
            "params": streams,
            "id": id
        });

        client
            .send(tokio_tungstenite::tungstenite::protocol::Message::Text(
                request.to_string().into(),
            ))
            .await?;

        Ok(())
    }

    /// Message reception loop
    async fn message_loop(mut ctx: MessageLoopContext) {
        let mut reconnect_attempt = 0;

        Self::resubscribe_all(&ctx.ws_client, &ctx.subscription_manager, &ctx.request_id).await;

        // Signal readiness to the caller
        if let Some(ready_tx) = ctx.ready_tx.take() {
            let _ = ready_tx.send(());
        }

        loop {
            if !ctx.is_connected.load(std::sync::atomic::Ordering::SeqCst) {
                break;
            }

            let has_client = ctx.ws_client.read().await.is_some();

            if !has_client {
                let config = ctx.reconnect_config.read().await;
                if config.should_retry(reconnect_attempt) {
                    let delay = config.calculate_delay(reconnect_attempt);
                    drop(config);

                    tokio::time::sleep(Duration::from_millis(delay)).await;

                    if let Ok(()) = Self::reconnect(
                        &ctx.base_url,
                        &ctx.current_url,
                        ctx.ws_client.clone(),
                        ctx.listen_key_manager.clone(),
                    )
                    .await
                    {
                        Self::resubscribe_all(
                            &ctx.ws_client,
                            &ctx.subscription_manager,
                            &ctx.request_id,
                        )
                        .await;
                        reconnect_attempt = 0;
                        continue;
                    }
                    reconnect_attempt += 1;
                    continue;
                }
                ctx.is_connected
                    .store(false, std::sync::atomic::Ordering::SeqCst);
                break;
            }

            let message_opt = {
                let guard = ctx.ws_client.read().await;
                if let Some(client) = guard.as_ref() {
                    client.receive().await
                } else {
                    None
                }
            };

            if let Some(value) = message_opt {
                if let Err(e) = Self::handle_message(
                    value,
                    ctx.subscription_manager.clone(),
                    ctx.listen_key_manager.clone(),
                )
                .await
                {
                    tracing::warn!(
                        error = %e,
                        "Message handling failed, continuing"
                    );
                    continue;
                }

                reconnect_attempt = 0;
            } else {
                let config = ctx.reconnect_config.read().await;
                if config.should_retry(reconnect_attempt) {
                    let delay = config.calculate_delay(reconnect_attempt);
                    drop(config);

                    tokio::time::sleep(Duration::from_millis(delay)).await;

                    if let Ok(()) = Self::reconnect(
                        &ctx.base_url,
                        &ctx.current_url,
                        ctx.ws_client.clone(),
                        ctx.listen_key_manager.clone(),
                    )
                    .await
                    {
                        Self::resubscribe_all(
                            &ctx.ws_client,
                            &ctx.subscription_manager,
                            &ctx.request_id,
                        )
                        .await;
                        reconnect_attempt = 0;
                        continue;
                    }
                    reconnect_attempt += 1;
                    continue;
                }
                ctx.is_connected
                    .store(false, std::sync::atomic::Ordering::SeqCst);
                break;
            }
        }
    }

    /// Resubscribes to all active streams
    async fn resubscribe_all(
        ws_client: &Arc<tokio::sync::RwLock<Option<ccxt_core::ws_client::WsClient>>>,
        subscription_manager: &Arc<super::subscriptions::SubscriptionManager>,
        request_id: &Arc<std::sync::atomic::AtomicU64>,
    ) {
        let streams = subscription_manager.get_active_streams().await;
        if streams.is_empty() {
            return;
        }

        let client_opt = ws_client.read().await;
        if let Some(client) = client_opt.as_ref() {
            // Split into chunks of 10 to avoid hitting limits
            for chunk in streams.chunks(10) {
                let id = request_id.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                #[allow(clippy::disallowed_methods)]
                let request = serde_json::json!({
                    "method": "SUBSCRIBE",
                    "params": chunk,
                    "id": id
                });

                if let Err(e) = client
                    .send(tokio_tungstenite::tungstenite::protocol::Message::Text(
                        request.to_string().into(),
                    ))
                    .await
                {
                    tracing::error!("Failed to resubscribe: {}", e);
                }
            }
        }
    }

    /// Processes a WebSocket message
    async fn handle_message(
        message: Value,
        subscription_manager: Arc<super::subscriptions::SubscriptionManager>,
        listen_key_manager: Option<Arc<super::listen_key::ListenKeyManager>>,
    ) -> Result<()> {
        let stream_name = Self::extract_stream_name(&message)?;

        // Check for "data" field (Combined Stream format) and unwrap if present
        let payload = if message.get("stream").is_some() && message.get("data").is_some() {
            message.get("data").cloned().unwrap_or(message.clone())
        } else {
            message.clone()
        };

        // Handle listenKeyExpired event
        if stream_name == "!userData" {
            if let Some(event) = payload.get("e").and_then(|e| e.as_str()) {
                if event == "listenKeyExpired" {
                    if let Some(manager) = listen_key_manager {
                        tracing::info!("Received listenKeyExpired event, regenerating key...");
                        let _ = manager.regenerate().await;
                    }
                }
            }
        }

        let sent = subscription_manager
            .send_to_stream(&stream_name, payload.clone())
            .await;

        if sent {
            return Ok(());
        }

        // Handle optional stream suffixes (e.g. @1s, @100ms)
        // If exact match failed, try to find matches by symbol that start with the extracted stream name
        let symbol_opt = payload.get("s").and_then(|s| s.as_str());

        if let Some(symbol) = symbol_opt {
            // Normalize symbol to lowercase since subscriptions are stored normalized
            let normalized_symbol = symbol.to_lowercase();
            let active_streams = subscription_manager
                .get_subscriptions_by_symbol(&normalized_symbol)
                .await;

            tracing::debug!(
                "Routing message for symbol {} (normalized: {}): stream_name={}, active_subscriptions={}",
                symbol,
                normalized_symbol,
                stream_name,
                active_streams.len()
            );

            let mut fallback_sent = false;

            for sub in active_streams {
                // For mark price updates, the stream name is usually like "btcusdt@markPrice" or "btcusdt@markprice"
                // But the user might have subscribed to "btcusdt@markprice@1s"
                // The starts_with check handles the suffix @1s
                tracing::debug!(
                    "Checking subscription: stream={}, expected_starts_with={}",
                    sub.stream,
                    stream_name
                );

                if sub.stream.starts_with(&stream_name) {
                    if subscription_manager
                        .send_to_stream(&sub.stream, payload.clone())
                        .await
                    {
                        fallback_sent = true;
                        tracing::debug!("Successfully routed to fallback stream: {}", sub.stream);
                    }
                }
            }

            if fallback_sent {
                return Ok(());
            }
        }

        Err(Error::generic("No subscribers for stream"))
    }

    /// Extracts the stream name from an incoming message
    pub fn extract_stream_name(message: &Value) -> Result<String> {
        if let Some(stream) = message.get("stream").and_then(|s| s.as_str()) {
            return Ok(stream.to_string());
        }

        // Handle array messages (e.g. !ticker@arr, !miniTicker@arr)
        if let Some(arr) = message.as_array() {
            if let Some(first) = arr.first() {
                if let Some(event_type) = first.get("e").and_then(|e| e.as_str()) {
                    match event_type {
                        "24hrTicker" => return Ok("!ticker@arr".to_string()),
                        "24hrMiniTicker" => return Ok("!miniTicker@arr".to_string()),
                        _ => {}
                    }
                }
            }
        }

        if let Some(event_type) = message.get("e").and_then(|e| e.as_str()) {
            match event_type {
                "outboundAccountPosition"
                | "balanceUpdate"
                | "executionReport"
                | "listStatus"
                | "ACCOUNT_UPDATE"
                | "ORDER_TRADE_UPDATE"
                | "listenKeyExpired" => {
                    return Ok("!userData".to_string());
                }
                _ => {}
            }

            if let Some(symbol) = message.get("s").and_then(|s| s.as_str()) {
                let stream = match event_type {
                    "24hrTicker" => format!("{}@ticker", symbol.to_lowercase()),
                    "24hrMiniTicker" => format!("{}@miniTicker", symbol.to_lowercase()),
                    "depthUpdate" => format!("{}@depth", symbol.to_lowercase()),
                    "aggTrade" => format!("{}@aggTrade", symbol.to_lowercase()),
                    "trade" => format!("{}@trade", symbol.to_lowercase()),
                    "kline" => {
                        if let Some(kline) = message.get("k") {
                            if let Some(interval) = kline.get("i").and_then(|i| i.as_str()) {
                                format!("{}@kline_{}", symbol.to_lowercase(), interval)
                            } else {
                                return Err(Error::generic("Missing kline interval"));
                            }
                        } else {
                            return Err(Error::generic("Missing kline data"));
                        }
                    }
                    "markPriceUpdate" => format!("{}@markPrice", symbol.to_lowercase()),
                    "bookTicker" => format!("{}@bookTicker", symbol.to_lowercase()),
                    _ => {
                        return Err(Error::generic(format!(
                            "Unknown event type: {}",
                            event_type
                        )));
                    }
                };
                return Ok(stream);
            }
        }

        if message.get("result").is_some() || message.get("error").is_some() {
            return Err(Error::generic("Subscription response, skip routing"));
        }

        Err(Error::generic("Cannot extract stream name from message"))
    }

    /// Reconnects the WebSocket client
    async fn reconnect(
        base_url: &str,
        current_url: &str,
        ws_client: Arc<tokio::sync::RwLock<Option<ccxt_core::ws_client::WsClient>>>,
        listen_key_manager: Option<Arc<super::listen_key::ListenKeyManager>>,
    ) -> Result<()> {
        {
            let mut client_opt = ws_client.write().await;
            if let Some(client) = client_opt.take() {
                let _ = client.disconnect().await;
            }
        }

        // Determine if we should refresh URL from listen key manager
        let mut final_url = current_url.to_string();

        if let Some(manager) = listen_key_manager {
            // If current URL is different from base, we assume it's a private stream
            // that might need a fresh listen key
            if current_url != base_url {
                if let Ok(key) = manager.get_or_create().await {
                    let base = if let Some(stripped) = base_url.strip_suffix('/') {
                        stripped
                    } else {
                        base_url
                    };
                    final_url = format!("{}/{}", base, key);
                }
            }
        }

        let config = ccxt_core::ws_client::WsConfig {
            url: final_url,
            ..Default::default()
        };
        let new_client = ccxt_core::ws_client::WsClient::new(config);

        new_client.connect().await?;

        *ws_client.write().await = Some(new_client);

        Ok(())
    }
}

impl Drop for MessageRouter {
    fn drop(&mut self) {
        // Note: Drop is synchronous, so we cannot await asynchronous operations here.
        // Callers should explicitly invoke `stop()` to release resources.
    }
}

/// Processes an order book delta update
pub async fn handle_orderbook_delta(
    symbol: &str,
    delta_message: &Value,
    is_futures: bool,
    orderbooks: &Mutex<HashMap<String, OrderBook>>,
) -> Result<()> {
    let first_update_id = delta_message["U"]
        .as_i64()
        .ok_or_else(|| Error::invalid_request("Missing first update ID in delta message"))?;

    let final_update_id = delta_message["u"]
        .as_i64()
        .ok_or_else(|| Error::invalid_request("Missing final update ID in delta message"))?;

    let prev_final_update_id = if is_futures {
        delta_message["pu"].as_i64()
    } else {
        None
    };

    let timestamp = delta_message["E"]
        .as_i64()
        .unwrap_or_else(|| chrono::Utc::now().timestamp_millis());

    let mut bids = Vec::new();
    if let Some(bids_arr) = delta_message["b"].as_array() {
        for bid in bids_arr {
            if let (Some(price_str), Some(amount_str)) = (bid[0].as_str(), bid[1].as_str()) {
                if let (Ok(price), Ok(amount)) =
                    (price_str.parse::<Decimal>(), amount_str.parse::<Decimal>())
                {
                    bids.push(OrderBookEntry::new(Price::new(price), Amount::new(amount)));
                }
            }
        }
    }

    let mut asks = Vec::new();
    if let Some(asks_arr) = delta_message["a"].as_array() {
        for ask in asks_arr {
            if let (Some(price_str), Some(amount_str)) = (ask[0].as_str(), ask[1].as_str()) {
                if let (Ok(price), Ok(amount)) =
                    (price_str.parse::<Decimal>(), amount_str.parse::<Decimal>())
                {
                    asks.push(OrderBookEntry::new(Price::new(price), Amount::new(amount)));
                }
            }
        }
    }

    let delta = OrderBookDelta {
        symbol: symbol.to_string(),
        first_update_id,
        final_update_id,
        prev_final_update_id,
        timestamp,
        bids,
        asks,
    };

    let mut orderbooks_map = orderbooks.lock().await;
    let orderbook = orderbooks_map
        .entry(symbol.to_string())
        .or_insert_with(|| OrderBook::new(symbol.to_string(), timestamp));

    if !orderbook.is_synced {
        orderbook.buffer_delta(delta);
        return Ok(());
    }

    if let Err(e) = orderbook.apply_delta(&delta, is_futures) {
        if orderbook.needs_resync {
            tracing::warn!("Orderbook {} needs resync due to: {}", symbol, e);
            orderbook.buffer_delta(delta);
            return Err(Error::invalid_request(format!("RESYNC_NEEDED: {}", e)));
        }
        return Err(Error::invalid_request(e));
    }

    Ok(())
}

/// Retrieves an order book snapshot and initializes cached state
pub async fn fetch_orderbook_snapshot(
    exchange: &Binance,
    symbol: &str,
    limit: Option<i64>,
    is_futures: bool,
    orderbooks: &Mutex<HashMap<String, OrderBook>>,
) -> Result<OrderBook> {
    let mut snapshot = exchange
        .fetch_order_book(symbol, limit.map(|l| l as u32))
        .await?;

    snapshot.is_synced = true;

    let mut orderbooks_map = orderbooks.lock().await;
    if let Some(cached_ob) = orderbooks_map.get_mut(symbol) {
        snapshot
            .buffered_deltas
            .clone_from(&cached_ob.buffered_deltas);

        if let Ok(processed) = snapshot.process_buffered_deltas(is_futures) {
            tracing::debug!("Processed {} buffered deltas for {}", processed, symbol);
        }
    }

    orderbooks_map.insert(symbol.to_string(), snapshot.clone());

    Ok(snapshot)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::disallowed_methods)]
    use super::*;
    use serde_json::json;
    use std::sync::Arc;

    #[test]
    fn test_extract_stream_name_combined() {
        let message = json!({
            "stream": "btcusdt@ticker",
            "data": {
                "e": "24hrTicker",
                "s": "BTCUSDT"
            }
        });

        let stream = MessageRouter::extract_stream_name(&message).unwrap();
        assert_eq!(stream, "btcusdt@ticker");
    }

    #[test]
    fn test_extract_stream_name_raw() {
        let message = json!({
            "e": "24hrTicker",
            "s": "BTCUSDT"
        });

        let stream = MessageRouter::extract_stream_name(&message).unwrap();
        assert_eq!(stream, "btcusdt@ticker");
    }

    #[tokio::test]
    async fn test_handle_message_unwrapping() {
        let manager = Arc::new(crate::binance::ws::subscriptions::SubscriptionManager::new());
        let (tx, mut rx) = tokio::sync::mpsc::channel(100);

        manager
            .add_subscription(
                "btcusdt@ticker".to_string(),
                "BTCUSDT".to_string(),
                crate::binance::ws::subscriptions::SubscriptionType::Ticker,
                tx,
            )
            .await
            .unwrap();

        let message = json!({
            "stream": "btcusdt@ticker",
            "data": {
                "e": "24hrTicker",
                "s": "BTCUSDT",
                "c": "50000.00"
            }
        });

        MessageRouter::handle_message(message, manager, None)
            .await
            .unwrap();

        let received = rx.recv().await.unwrap();
        assert!(received.get("stream").is_none());
        assert_eq!(received["e"], "24hrTicker");
        assert_eq!(received["c"], "50000.00");
    }

    #[tokio::test]
    async fn test_handle_message_mark_price_fallback() {
        let manager = Arc::new(crate::binance::ws::subscriptions::SubscriptionManager::new());
        let (tx, mut rx) = tokio::sync::mpsc::channel(100);

        // Subscription uses lowercase symbol "btcusdt" and specific stream "btcusdt@markPrice@1s"
        manager
            .add_subscription(
                "btcusdt@markPrice@1s".to_string(),
                "btcusdt".to_string(),
                crate::binance::ws::subscriptions::SubscriptionType::MarkPrice,
                tx,
            )
            .await
            .unwrap();

        // Incoming message has uppercase symbol "BTCUSDT" and event "markPriceUpdate"
        // extract_stream_name will derive "btcusdt@markPrice"
        let message = json!({
            "e": "markPriceUpdate",
            "s": "BTCUSDT",
            "p": "50000.00",
            "E": 123456789
        });

        // This should succeed thanks to the fallback logic
        MessageRouter::handle_message(message, manager, None)
            .await
            .unwrap();

        let received = rx.recv().await.unwrap();
        assert_eq!(received["e"], "markPriceUpdate");
        assert_eq!(received["p"], "50000.00");
    }
}
