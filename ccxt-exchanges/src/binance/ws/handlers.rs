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

    /// Configuration for reconnection behavior
    reconnect_config: Arc<tokio::sync::RwLock<super::subscriptions::ReconnectConfig>>,

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
    ) -> Self {
        Self {
            ws_client: Arc::new(tokio::sync::RwLock::new(None)),
            subscription_manager,
            router_task: Arc::new(Mutex::new(None)),
            is_connected: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            reconnect_config: Arc::new(tokio::sync::RwLock::new(
                super::subscriptions::ReconnectConfig::default(),
            )),
            ws_url,
            request_id: Arc::new(std::sync::atomic::AtomicU64::new(1)),
        }
    }

    /// Starts the message router
    pub async fn start(&self, url_override: Option<String>) -> Result<()> {
        if self.is_connected() {
            self.stop().await?;
        }

        let url = url_override.unwrap_or_else(|| self.ws_url.clone());
        let config = ccxt_core::ws_client::WsConfig {
            url: url.clone(),
            ..Default::default()
        };
        let client = ccxt_core::ws_client::WsClient::new(config);
        client.connect().await?;

        *self.ws_client.write().await = Some(client);

        self.is_connected
            .store(true, std::sync::atomic::Ordering::SeqCst);

        let ws_client = self.ws_client.clone();
        let subscription_manager = self.subscription_manager.clone();
        let is_connected = self.is_connected.clone();
        let reconnect_config = self.reconnect_config.clone();

        let ws_url = url;

        let handle = tokio::spawn(async move {
            Self::message_loop(
                ws_client,
                subscription_manager,
                is_connected,
                reconnect_config,
                ws_url,
            )
            .await;
        });

        *self.router_task.lock().await = Some(handle);

        Ok(())
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
    async fn message_loop(
        ws_client: Arc<tokio::sync::RwLock<Option<ccxt_core::ws_client::WsClient>>>,
        subscription_manager: Arc<super::subscriptions::SubscriptionManager>,
        is_connected: Arc<std::sync::atomic::AtomicBool>,
        reconnect_config: Arc<tokio::sync::RwLock<super::subscriptions::ReconnectConfig>>,
        ws_url: String,
    ) {
        let mut reconnect_attempt = 0;

        loop {
            if !is_connected.load(std::sync::atomic::Ordering::SeqCst) {
                break;
            }

            let has_client = ws_client.read().await.is_some();

            if !has_client {
                let config = reconnect_config.read().await;
                if config.should_retry(reconnect_attempt) {
                    let delay = config.calculate_delay(reconnect_attempt);
                    drop(config);

                    tokio::time::sleep(Duration::from_millis(delay)).await;

                    if let Ok(()) = Self::reconnect(&ws_url, ws_client.clone()).await {
                        reconnect_attempt = 0;
                        continue;
                    }
                    reconnect_attempt += 1;
                    continue;
                }
                is_connected.store(false, std::sync::atomic::Ordering::SeqCst);
                break;
            }

            let message_opt = {
                let guard = ws_client.read().await;
                if let Some(client) = guard.as_ref() {
                    client.receive().await
                } else {
                    None
                }
            };

            if let Some(value) = message_opt {
                if let Err(_e) = Self::handle_message(value, subscription_manager.clone()).await {
                    continue;
                }

                reconnect_attempt = 0;
            } else {
                let config = reconnect_config.read().await;
                if config.should_retry(reconnect_attempt) {
                    let delay = config.calculate_delay(reconnect_attempt);
                    drop(config);

                    tokio::time::sleep(Duration::from_millis(delay)).await;

                    if let Ok(()) = Self::reconnect(&ws_url, ws_client.clone()).await {
                        reconnect_attempt = 0;
                        continue;
                    }
                    reconnect_attempt += 1;
                    continue;
                }
                is_connected.store(false, std::sync::atomic::Ordering::SeqCst);
                break;
            }
        }
    }

    /// Processes a WebSocket message
    async fn handle_message(
        message: Value,
        subscription_manager: Arc<super::subscriptions::SubscriptionManager>,
    ) -> Result<()> {
        let stream_name = Self::extract_stream_name(&message)?;

        // Check for "data" field (Combined Stream format) and unwrap if present
        let payload = if message.get("stream").is_some() && message.get("data").is_some() {
            message.get("data").cloned().unwrap_or(message)
        } else {
            message
        };

        let sent = subscription_manager
            .send_to_stream(&stream_name, payload)
            .await;

        if sent {
            Ok(())
        } else {
            Err(Error::generic("No subscribers for stream"))
        }
    }

    /// Extracts the stream name from an incoming message
    pub fn extract_stream_name(message: &Value) -> Result<String> {
        if let Some(stream) = message.get("stream").and_then(|s| s.as_str()) {
            return Ok(stream.to_string());
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
        ws_url: &str,
        ws_client: Arc<tokio::sync::RwLock<Option<ccxt_core::ws_client::WsClient>>>,
    ) -> Result<()> {
        {
            let mut client_opt = ws_client.write().await;
            if let Some(client) = client_opt.take() {
                let _ = client.disconnect().await;
            }
        }

        let config = ccxt_core::ws_client::WsConfig {
            url: ws_url.to_string(),
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
    let mut params = HashMap::new();
    if let Some(l) = limit {
        #[allow(clippy::disallowed_methods)]
        let limit_value = serde_json::json!(l);
        params.insert("limit".to_string(), limit_value);
    }

    let mut snapshot = exchange.fetch_order_book(symbol, None).await?;

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
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();

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

        MessageRouter::handle_message(message, manager)
            .await
            .unwrap();

        let received = rx.recv().await.unwrap();
        assert!(received.get("stream").is_none());
        assert_eq!(received["e"], "24hrTicker");
        assert_eq!(received["c"], "50000.00");
    }
}
