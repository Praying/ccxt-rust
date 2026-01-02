//! OKX WebSocket implementation.
//!
//! Provides real-time data streaming via WebSocket for OKX exchange.
//! Supports public streams (ticker, orderbook, trades) and private streams
//! (balance, orders) with automatic reconnection.

use crate::okx::parser::{parse_orderbook, parse_ticker, parse_trade};
use ccxt_core::error::{Error, Result};
use ccxt_core::types::{Market, OrderBook, Ticker, Trade};
use ccxt_core::ws_client::{WsClient, WsConfig, WsConnectionState};
use ccxt_core::ws_exchange::MessageStream;
use futures::Stream;
use serde_json::Value;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use tokio::sync::{RwLock, mpsc};

/// Default ping interval for OKX WebSocket (25 seconds).
/// OKX requires ping every 30 seconds, we use 25 for safety margin.
const DEFAULT_PING_INTERVAL_MS: u64 = 25000;

/// Default reconnect delay (5 seconds).
const DEFAULT_RECONNECT_INTERVAL_MS: u64 = 5000;

/// Maximum reconnect attempts.
const MAX_RECONNECT_ATTEMPTS: u32 = 10;

/// OKX WebSocket client.
///
/// Provides real-time data streaming for OKX exchange.
pub struct OkxWs {
    /// WebSocket client instance.
    client: Arc<WsClient>,
    /// Active subscriptions.
    subscriptions: Arc<RwLock<Vec<String>>>,
}

impl OkxWs {
    /// Creates a new OKX WebSocket client.
    ///
    /// # Arguments
    ///
    /// * `url` - WebSocket server URL
    pub fn new(url: String) -> Self {
        let config = WsConfig {
            url: url.clone(),
            connect_timeout: 10000,
            ping_interval: DEFAULT_PING_INTERVAL_MS,
            reconnect_interval: DEFAULT_RECONNECT_INTERVAL_MS,
            max_reconnect_attempts: MAX_RECONNECT_ATTEMPTS,
            auto_reconnect: true,
            enable_compression: false,
            pong_timeout: 90000,
        };

        Self {
            client: Arc::new(WsClient::new(config)),
            subscriptions: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Connects to the WebSocket server.
    pub async fn connect(&self) -> Result<()> {
        self.client.connect().await
    }

    /// Disconnects from the WebSocket server.
    pub async fn disconnect(&self) -> Result<()> {
        self.client.disconnect().await
    }

    /// Returns the current connection state.
    pub fn state(&self) -> WsConnectionState {
        self.client.state()
    }

    /// Checks if the WebSocket is connected.
    pub fn is_connected(&self) -> bool {
        self.client.is_connected()
    }

    /// Receives the next message from the WebSocket.
    pub async fn receive(&self) -> Option<Value> {
        self.client.receive().await
    }

    /// Subscribes to a ticker stream.
    ///
    /// # Arguments
    ///
    /// * `symbol` - Trading pair symbol in OKX format (e.g., "BTC-USDT")
    pub async fn subscribe_ticker(&self, symbol: &str) -> Result<()> {
        // OKX V5 WebSocket subscription format
        // json! macro with literal values is infallible
        #[allow(clippy::disallowed_methods)]
        let msg = serde_json::json!({
            "op": "subscribe",
            "args": [{
                "channel": "tickers",
                "instId": symbol
            }]
        });

        self.client.send_json(&msg).await?;

        let sub_key = format!("ticker:{}", symbol);
        self.subscriptions.write().await.push(sub_key);

        Ok(())
    }

    /// Subscribes to an orderbook stream.
    ///
    /// # Arguments
    ///
    /// * `symbol` - Trading pair symbol in OKX format (e.g., "BTC-USDT")
    /// * `depth` - Orderbook depth (5, 50, or 400)
    pub async fn subscribe_orderbook(&self, symbol: &str, depth: u32) -> Result<()> {
        // OKX supports orderbook depths: books5, books, books50-l2
        let channel = match depth {
            d if d <= 5 => "books5",
            d if d <= 50 => "books50-l2",
            _ => "books",
        };

        // json! macro with literal values is infallible
        #[allow(clippy::disallowed_methods)]
        let msg = serde_json::json!({
            "op": "subscribe",
            "args": [{
                "channel": channel,
                "instId": symbol
            }]
        });

        self.client.send_json(&msg).await?;

        let sub_key = format!("orderbook:{}", symbol);
        self.subscriptions.write().await.push(sub_key);

        Ok(())
    }

    /// Subscribes to a trades stream.
    ///
    /// # Arguments
    ///
    /// * `symbol` - Trading pair symbol in OKX format (e.g., "BTC-USDT")
    pub async fn subscribe_trades(&self, symbol: &str) -> Result<()> {
        // json! macro with literal values is infallible
        #[allow(clippy::disallowed_methods)]
        let msg = serde_json::json!({
            "op": "subscribe",
            "args": [{
                "channel": "trades",
                "instId": symbol
            }]
        });

        self.client.send_json(&msg).await?;

        let sub_key = format!("trades:{}", symbol);
        self.subscriptions.write().await.push(sub_key);

        Ok(())
    }

    /// Subscribes to a kline/candlestick stream.
    ///
    /// # Arguments
    ///
    /// * `symbol` - Trading pair symbol in OKX format (e.g., "BTC-USDT")
    /// * `interval` - Kline interval (e.g., "1m", "5m", "1H", "1D")
    pub async fn subscribe_kline(&self, symbol: &str, interval: &str) -> Result<()> {
        let channel = format!("candle{}", interval);

        // json! macro with literal values is infallible
        #[allow(clippy::disallowed_methods)]
        let msg = serde_json::json!({
            "op": "subscribe",
            "args": [{
                "channel": channel,
                "instId": symbol
            }]
        });

        self.client.send_json(&msg).await?;

        let sub_key = format!("kline:{}:{}", symbol, interval);
        self.subscriptions.write().await.push(sub_key);

        Ok(())
    }

    /// Unsubscribes from a stream.
    ///
    /// # Arguments
    ///
    /// * `stream_name` - Stream identifier to unsubscribe from
    pub async fn unsubscribe(&self, stream_name: String) -> Result<()> {
        // Parse stream name to determine channel and symbol
        let parts: Vec<&str> = stream_name.split(':').collect();
        if parts.len() < 2 {
            return Err(Error::invalid_request(format!(
                "Invalid stream name: {}",
                stream_name
            )));
        }

        let channel_type = parts[0];
        let symbol = parts[1];

        let channel = match channel_type {
            "ticker" => "tickers".to_string(),
            "orderbook" => "books5".to_string(),
            "trades" => "trades".to_string(),
            "kline" => {
                if parts.len() >= 3 {
                    format!("candle{}", parts[2])
                } else {
                    return Err(Error::invalid_request(
                        "Kline unsubscribe requires interval",
                    ));
                }
            }
            _ => {
                return Err(Error::invalid_request(format!(
                    "Unknown channel: {}",
                    channel_type
                )));
            }
        };

        // json! macro with literal values is infallible
        #[allow(clippy::disallowed_methods)]
        let msg = serde_json::json!({
            "op": "unsubscribe",
            "args": [{
                "channel": channel,
                "instId": symbol
            }]
        });

        self.client.send_json(&msg).await?;

        // Remove from subscriptions
        let mut subs = self.subscriptions.write().await;
        subs.retain(|s| s != &stream_name);

        Ok(())
    }

    /// Returns the list of active subscriptions.
    pub async fn subscriptions(&self) -> Vec<String> {
        self.subscriptions.read().await.clone()
    }

    /// Watches ticker updates for a symbol.
    ///
    /// Returns a stream of `Ticker` updates for the specified symbol.
    ///
    /// # Arguments
    ///
    /// * `symbol` - Trading pair symbol in OKX format (e.g., "BTC-USDT")
    /// * `market` - Optional market information for symbol resolution
    ///
    /// # Returns
    ///
    /// A `MessageStream<Ticker>` that yields ticker updates.
    pub async fn watch_ticker(
        &self,
        symbol: &str,
        market: Option<Market>,
    ) -> Result<MessageStream<Ticker>> {
        // Ensure connected
        if !self.is_connected() {
            self.connect().await?;
        }

        // Subscribe to ticker channel
        self.subscribe_ticker(symbol).await?;

        // Create channel for ticker updates
        let (tx, rx) = mpsc::unbounded_channel::<Result<Ticker>>();
        let symbol_owned = symbol.to_string();
        let client = Arc::clone(&self.client);

        // Spawn task to process messages and filter ticker updates
        tokio::spawn(async move {
            while let Some(msg) = client.receive().await {
                // Check if this is a ticker message for our symbol
                if is_ticker_message(&msg, &symbol_owned) {
                    match parse_ws_ticker(&msg, market.as_ref()) {
                        Ok(ticker) => {
                            if tx.send(Ok(ticker)).is_err() {
                                break; // Receiver dropped
                            }
                        }
                        Err(e) => {
                            if tx.send(Err(e)).is_err() {
                                break;
                            }
                        }
                    }
                }
            }
        });

        Ok(Box::pin(ReceiverStream::new(rx)))
    }

    /// Watches order book updates for a symbol.
    ///
    /// Returns a stream of `OrderBook` updates for the specified symbol.
    ///
    /// # Arguments
    ///
    /// * `symbol` - Trading pair symbol in OKX format (e.g., "BTC-USDT")
    /// * `limit` - Optional depth limit (5, 50, or 400)
    ///
    /// # Returns
    ///
    /// A `MessageStream<OrderBook>` that yields order book updates.
    pub async fn watch_order_book(
        &self,
        symbol: &str,
        limit: Option<u32>,
    ) -> Result<MessageStream<OrderBook>> {
        // Ensure connected
        if !self.is_connected() {
            self.connect().await?;
        }

        // Subscribe to orderbook channel
        let depth = limit.unwrap_or(5);
        self.subscribe_orderbook(symbol, depth).await?;

        // Create channel for orderbook updates
        let (tx, rx) = mpsc::unbounded_channel::<Result<OrderBook>>();
        let symbol_owned = symbol.to_string();
        let unified_symbol = format_unified_symbol(&symbol_owned);
        let client = Arc::clone(&self.client);

        // Spawn task to process messages and filter orderbook updates
        tokio::spawn(async move {
            while let Some(msg) = client.receive().await {
                // Check if this is an orderbook message for our symbol
                if is_orderbook_message(&msg, &symbol_owned) {
                    match parse_ws_orderbook(&msg, unified_symbol.clone()) {
                        Ok(orderbook) => {
                            if tx.send(Ok(orderbook)).is_err() {
                                break; // Receiver dropped
                            }
                        }
                        Err(e) => {
                            if tx.send(Err(e)).is_err() {
                                break;
                            }
                        }
                    }
                }
            }
        });

        Ok(Box::pin(ReceiverStream::new(rx)))
    }

    /// Watches trade updates for a symbol.
    ///
    /// Returns a stream of `Trade` updates for the specified symbol.
    ///
    /// # Arguments
    ///
    /// * `symbol` - Trading pair symbol in OKX format (e.g., "BTC-USDT")
    /// * `market` - Optional market information for symbol resolution
    ///
    /// # Returns
    ///
    /// A `MessageStream<Vec<Trade>>` that yields trade updates.
    pub async fn watch_trades(
        &self,
        symbol: &str,
        market: Option<Market>,
    ) -> Result<MessageStream<Vec<Trade>>> {
        // Ensure connected
        if !self.is_connected() {
            self.connect().await?;
        }

        // Subscribe to trades channel
        self.subscribe_trades(symbol).await?;

        // Create channel for trade updates
        let (tx, rx) = mpsc::unbounded_channel::<Result<Vec<Trade>>>();
        let symbol_owned = symbol.to_string();
        let client = Arc::clone(&self.client);

        // Spawn task to process messages and filter trade updates
        tokio::spawn(async move {
            while let Some(msg) = client.receive().await {
                // Check if this is a trade message for our symbol
                if is_trade_message(&msg, &symbol_owned) {
                    match parse_ws_trades(&msg, market.as_ref()) {
                        Ok(trades) => {
                            if tx.send(Ok(trades)).is_err() {
                                break; // Receiver dropped
                            }
                        }
                        Err(e) => {
                            if tx.send(Err(e)).is_err() {
                                break;
                            }
                        }
                    }
                }
            }
        });

        Ok(Box::pin(ReceiverStream::new(rx)))
    }
}

// ============================================================================
// Stream Wrapper
// ============================================================================

/// A stream wrapper that converts an mpsc receiver into a futures Stream.
struct ReceiverStream<T> {
    receiver: mpsc::UnboundedReceiver<T>,
}

impl<T> ReceiverStream<T> {
    fn new(receiver: mpsc::UnboundedReceiver<T>) -> Self {
        Self { receiver }
    }
}

impl<T> Stream for ReceiverStream<T> {
    type Item = T;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.receiver.poll_recv(cx)
    }
}

// ============================================================================
// Message Type Detection Helpers
// ============================================================================

/// Check if a WebSocket message is a ticker message for the given symbol.
fn is_ticker_message(msg: &Value, symbol: &str) -> bool {
    // OKX V5 WebSocket ticker format:
    // {"arg":{"channel":"tickers","instId":"BTC-USDT"},"data":[{...}]}
    if let Some(arg) = msg.get("arg") {
        let channel = arg.get("channel").and_then(|c| c.as_str());
        let inst_id = arg.get("instId").and_then(|i| i.as_str());
        channel == Some("tickers") && inst_id == Some(symbol)
    } else {
        false
    }
}

/// Check if a WebSocket message is an orderbook message for the given symbol.
fn is_orderbook_message(msg: &Value, symbol: &str) -> bool {
    // OKX V5 WebSocket orderbook format:
    // {"arg":{"channel":"books5","instId":"BTC-USDT"},"data":[{...}]}
    if let Some(arg) = msg.get("arg") {
        let channel = arg.get("channel").and_then(|c| c.as_str());
        let inst_id = arg.get("instId").and_then(|i| i.as_str());
        // Check if channel starts with "books" and instId matches
        if let (Some(ch), Some(id)) = (channel, inst_id) {
            ch.starts_with("books") && id == symbol
        } else {
            false
        }
    } else {
        false
    }
}

/// Check if a WebSocket message is a trade message for the given symbol.
fn is_trade_message(msg: &Value, symbol: &str) -> bool {
    // OKX V5 WebSocket trade format:
    // {"arg":{"channel":"trades","instId":"BTC-USDT"},"data":[{...}]}
    if let Some(arg) = msg.get("arg") {
        let channel = arg.get("channel").and_then(|c| c.as_str());
        let inst_id = arg.get("instId").and_then(|i| i.as_str());
        channel == Some("trades") && inst_id == Some(symbol)
    } else {
        false
    }
}

/// Format an OKX symbol (e.g., "BTC-USDT") to unified format (e.g., "BTC/USDT").
fn format_unified_symbol(symbol: &str) -> String {
    symbol.replace('-', "/")
}

// ============================================================================
// WebSocket Message Parsers
// ============================================================================

/// Parse a WebSocket ticker message.
pub fn parse_ws_ticker(msg: &Value, market: Option<&Market>) -> Result<Ticker> {
    // OKX V5 WebSocket ticker format:
    // {"arg":{"channel":"tickers","instId":"BTC-USDT"},"data":[{...}]}
    let data = msg
        .get("data")
        .and_then(|d| d.as_array())
        .and_then(|arr| arr.first())
        .ok_or_else(|| Error::invalid_request("Missing data in ticker message"))?;

    parse_ticker(data, market)
}

/// Parse a WebSocket orderbook message.
pub fn parse_ws_orderbook(msg: &Value, symbol: String) -> Result<OrderBook> {
    // OKX V5 WebSocket orderbook format:
    // {"arg":{"channel":"books5","instId":"BTC-USDT"},"data":[{"asks":[...],"bids":[...],"ts":"..."}]}
    let data = msg
        .get("data")
        .and_then(|d| d.as_array())
        .and_then(|arr| arr.first())
        .ok_or_else(|| Error::invalid_request("Missing data in orderbook message"))?;

    parse_orderbook(data, symbol)
}

/// Parse a WebSocket trade message (single trade).
pub fn parse_ws_trade(msg: &Value, market: Option<&Market>) -> Result<Trade> {
    // OKX V5 WebSocket trade format:
    // {"arg":{"channel":"trades","instId":"BTC-USDT"},"data":[{...}]}
    let data = msg
        .get("data")
        .and_then(|d| d.as_array())
        .and_then(|arr| arr.first())
        .ok_or_else(|| Error::invalid_request("Missing data in trade message"))?;

    parse_trade(data, market)
}

/// Parse a WebSocket trade message (multiple trades).
pub fn parse_ws_trades(msg: &Value, market: Option<&Market>) -> Result<Vec<Trade>> {
    // OKX V5 WebSocket trade format:
    // {"arg":{"channel":"trades","instId":"BTC-USDT"},"data":[{...}, {...}]}
    let data_array = msg
        .get("data")
        .and_then(|d| d.as_array())
        .ok_or_else(|| Error::invalid_request("Missing data in trade message"))?;

    let mut trades = Vec::with_capacity(data_array.len());
    for data in data_array {
        trades.push(parse_trade(data, market)?);
    }

    Ok(trades)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ccxt_core::types::financial::Price;
    use rust_decimal_macros::dec;

    #[test]
    fn test_okx_ws_creation() {
        let ws = OkxWs::new("wss://ws.okx.com:8443/ws/v5/public".to_string());
        assert!(ws.subscriptions.try_read().is_ok());
    }

    #[tokio::test]
    async fn test_subscriptions_empty_by_default() {
        let ws = OkxWs::new("wss://ws.okx.com:8443/ws/v5/public".to_string());
        let subs = ws.subscriptions().await;
        assert!(subs.is_empty());
    }

    // ==================== Message Type Detection Tests ====================

    #[test]
    fn test_is_ticker_message_true() {
        let msg = serde_json::from_str(
            r#"{
                "arg": {"channel": "tickers", "instId": "BTC-USDT"},
                "data": [{}]
            }"#,
        )
        .unwrap();

        assert!(is_ticker_message(&msg, "BTC-USDT"));
    }

    #[test]
    fn test_is_ticker_message_wrong_symbol() {
        let msg = serde_json::from_str(
            r#"{
                "arg": {"channel": "tickers", "instId": "ETH-USDT"},
                "data": [{}]
            }"#,
        )
        .unwrap();

        assert!(!is_ticker_message(&msg, "BTC-USDT"));
    }

    #[test]
    fn test_is_ticker_message_wrong_channel() {
        let msg = serde_json::from_str(
            r#"{
                "arg": {"channel": "trades", "instId": "BTC-USDT"},
                "data": [{}]
            }"#,
        )
        .unwrap();

        assert!(!is_ticker_message(&msg, "BTC-USDT"));
    }

    #[test]
    fn test_is_orderbook_message_books5() {
        let msg = serde_json::from_str(
            r#"{
                "arg": {"channel": "books5", "instId": "BTC-USDT"},
                "data": [{}]
            }"#,
        )
        .unwrap();

        assert!(is_orderbook_message(&msg, "BTC-USDT"));
    }

    #[test]
    fn test_is_orderbook_message_books50() {
        let msg = serde_json::from_str(
            r#"{
                "arg": {"channel": "books50-l2", "instId": "BTC-USDT"},
                "data": [{}]
            }"#,
        )
        .unwrap();

        assert!(is_orderbook_message(&msg, "BTC-USDT"));
    }

    #[test]
    fn test_is_trade_message_true() {
        let msg = serde_json::from_str(
            r#"{
                "arg": {"channel": "trades", "instId": "BTC-USDT"},
                "data": [{}]
            }"#,
        )
        .unwrap();

        assert!(is_trade_message(&msg, "BTC-USDT"));
    }

    #[test]
    fn test_format_unified_symbol() {
        assert_eq!(format_unified_symbol("BTC-USDT"), "BTC/USDT");
        assert_eq!(format_unified_symbol("ETH-BTC"), "ETH/BTC");
    }

    // ==================== Ticker Message Parsing Tests ====================

    #[test]
    fn test_parse_ws_ticker() {
        let msg = serde_json::from_str(
            r#"{
                "arg": {"channel": "tickers", "instId": "BTC-USDT"},
                "data": [{
                    "instId": "BTC-USDT",
                    "last": "50000.00",
                    "high24h": "51000.00",
                    "low24h": "49000.00",
                    "bidPx": "49999.00",
                    "askPx": "50001.00",
                    "vol24h": "1000.5",
                    "ts": "1700000000000"
                }]
            }"#,
        )
        .unwrap();

        let ticker = parse_ws_ticker(&msg, None).unwrap();
        // Parser converts BTC-USDT to BTC/USDT (unified format) when no market is provided
        assert_eq!(ticker.symbol, "BTC/USDT");
        assert_eq!(ticker.last, Some(Price::new(dec!(50000.00))));
        assert_eq!(ticker.high, Some(Price::new(dec!(51000.00))));
        assert_eq!(ticker.low, Some(Price::new(dec!(49000.00))));
    }

    #[test]
    fn test_parse_ws_ticker_with_market() {
        let msg = serde_json::from_str(
            r#"{
                "arg": {"channel": "tickers", "instId": "BTC-USDT"},
                "data": [{
                    "instId": "BTC-USDT",
                    "last": "50000.00",
                    "ts": "1700000000000"
                }]
            }"#,
        )
        .unwrap();

        let market = Market {
            id: "BTC-USDT".to_string(),
            symbol: "BTC/USDT".to_string(),
            base: "BTC".to_string(),
            quote: "USDT".to_string(),
            ..Default::default()
        };

        let ticker = parse_ws_ticker(&msg, Some(&market)).unwrap();
        assert_eq!(ticker.symbol, "BTC/USDT");
    }

    #[test]
    fn test_parse_ws_ticker_missing_data() {
        let msg = serde_json::from_str(
            r#"{
                "arg": {"channel": "tickers", "instId": "BTC-USDT"}
            }"#,
        )
        .unwrap();

        let result = parse_ws_ticker(&msg, None);
        assert!(result.is_err());
    }

    // ==================== OrderBook Message Parsing Tests ====================

    #[test]
    fn test_parse_ws_orderbook() {
        let msg = serde_json::from_str(
            r#"{
                "arg": {"channel": "books5", "instId": "BTC-USDT"},
                "data": [{
                    "asks": [
                        ["50001.00", "1.0", "0", "1"],
                        ["50002.00", "3.0", "0", "2"],
                        ["50003.00", "2.5", "0", "1"]
                    ],
                    "bids": [
                        ["50000.00", "1.5", "0", "2"],
                        ["49999.00", "2.0", "0", "1"],
                        ["49998.00", "0.5", "0", "1"]
                    ],
                    "ts": "1700000000000"
                }]
            }"#,
        )
        .unwrap();

        let orderbook = parse_ws_orderbook(&msg, "BTC/USDT".to_string()).unwrap();
        assert_eq!(orderbook.symbol, "BTC/USDT");
        assert_eq!(orderbook.bids.len(), 3);
        assert_eq!(orderbook.asks.len(), 3);

        // Verify bids are sorted in descending order
        assert_eq!(orderbook.bids[0].price, Price::new(dec!(50000.00)));
        assert_eq!(orderbook.bids[1].price, Price::new(dec!(49999.00)));

        // Verify asks are sorted in ascending order
        assert_eq!(orderbook.asks[0].price, Price::new(dec!(50001.00)));
        assert_eq!(orderbook.asks[1].price, Price::new(dec!(50002.00)));
    }

    #[test]
    fn test_parse_ws_orderbook_missing_data() {
        let msg = serde_json::from_str(
            r#"{
                "arg": {"channel": "books5", "instId": "BTC-USDT"}
            }"#,
        )
        .unwrap();

        let result = parse_ws_orderbook(&msg, "BTC/USDT".to_string());
        assert!(result.is_err());
    }

    // ==================== Trade Message Parsing Tests ====================

    #[test]
    fn test_parse_ws_trade() {
        let msg = serde_json::from_str(
            r#"{
                "arg": {"channel": "trades", "instId": "BTC-USDT"},
                "data": [{
                    "instId": "BTC-USDT",
                    "tradeId": "123456789",
                    "px": "50000.00",
                    "sz": "0.5",
                    "side": "buy",
                    "ts": "1700000000000"
                }]
            }"#,
        )
        .unwrap();

        let trade = parse_ws_trade(&msg, None).unwrap();
        assert_eq!(trade.timestamp, 1700000000000);
    }

    #[test]
    fn test_parse_ws_trades_multiple() {
        let msg = serde_json::from_str(
            r#"{
                "arg": {"channel": "trades", "instId": "BTC-USDT"},
                "data": [
                    {
                        "instId": "BTC-USDT",
                        "tradeId": "123456789",
                        "px": "50000.00",
                        "sz": "0.5",
                        "side": "buy",
                        "ts": "1700000000000"
                    },
                    {
                        "instId": "BTC-USDT",
                        "tradeId": "123456790",
                        "px": "50001.00",
                        "sz": "1.0",
                        "side": "sell",
                        "ts": "1700000000001"
                    }
                ]
            }"#,
        )
        .unwrap();

        let trades = parse_ws_trades(&msg, None).unwrap();
        assert_eq!(trades.len(), 2);
    }

    #[test]
    fn test_parse_ws_trade_missing_data() {
        let msg = serde_json::from_str(
            r#"{
                "arg": {"channel": "trades", "instId": "BTC-USDT"}
            }"#,
        )
        .unwrap();

        let result = parse_ws_trade(&msg, None);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_ws_trades_empty_array() {
        let msg = serde_json::from_str(
            r#"{
                "arg": {"channel": "trades", "instId": "BTC-USDT"},
                "data": []
            }"#,
        )
        .unwrap();

        let trades = parse_ws_trades(&msg, None).unwrap();
        assert!(trades.is_empty());
    }
}
