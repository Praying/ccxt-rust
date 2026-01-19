//! Bybit WebSocket implementation.
//!
//! Provides real-time data streaming via WebSocket for Bybit exchange.
//! Supports public streams (ticker, orderbook, trades) and private streams
//! (balance, orders) with automatic reconnection.

use crate::bybit::parser::{parse_orderbook, parse_ticker, parse_trade};
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

/// Default ping interval for Bybit WebSocket (20 seconds).
/// Bybit requires ping every 20 seconds to keep connection alive.
const DEFAULT_PING_INTERVAL_MS: u64 = 20000;

/// Default reconnect delay (5 seconds).
const DEFAULT_RECONNECT_INTERVAL_MS: u64 = 5000;

/// Maximum reconnect attempts.
const MAX_RECONNECT_ATTEMPTS: u32 = 10;

/// Bybit WebSocket client.
///
/// Provides real-time data streaming for Bybit exchange.
pub struct BybitWs {
    /// WebSocket client instance.
    client: Arc<WsClient>,
    /// Active subscriptions.
    subscriptions: Arc<RwLock<Vec<String>>>,
}

impl BybitWs {
    /// Creates a new Bybit WebSocket client.
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
            ..Default::default()
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
    /// * `symbol` - Trading pair symbol (e.g., "BTCUSDT")
    pub async fn subscribe_ticker(&self, symbol: &str) -> Result<()> {
        // Bybit V5 WebSocket subscription format
        let topic = format!("tickers.{}", symbol);

        // json! macro with literal values is infallible
        #[allow(clippy::disallowed_methods)]
        let msg = serde_json::json!({
            "op": "subscribe",
            "args": [topic]
        });

        self.client.send_json(&msg).await?;

        let sub_key = format!("ticker:{}", symbol);
        self.subscriptions.write().await.push(sub_key);

        Ok(())
    }

    /// Subscribes to multiple ticker streams.
    ///
    /// # Arguments
    ///
    /// * `symbols` - List of trading pair symbols (e.g., ["BTCUSDT", "ETHUSDT"])
    pub async fn subscribe_tickers(&self, symbols: &[String]) -> Result<()> {
        let topics: Vec<String> = symbols.iter().map(|s| format!("tickers.{}", s)).collect();

        // json! macro with literal values is infallible
        #[allow(clippy::disallowed_methods)]
        let msg = serde_json::json!({
            "op": "subscribe",
            "args": topics
        });

        self.client.send_json(&msg).await?;

        let mut subs = self.subscriptions.write().await;
        for symbol in symbols {
            subs.push(format!("ticker:{}", symbol));
        }

        Ok(())
    }

    /// Watches ticker updates for multiple symbols.
    ///
    /// Returns a stream of `Vec<Ticker>` updates for the specified symbols.
    ///
    /// # Arguments
    ///
    /// * `symbols` - List of trading pair symbols (e.g., ["BTCUSDT", "ETHUSDT"])
    ///
    /// # Returns
    ///
    /// A `MessageStream<Vec<Ticker>>` that yields ticker updates.
    pub async fn watch_tickers(&self, symbols: &[String]) -> Result<MessageStream<Vec<Ticker>>> {
        // Ensure connected
        if !self.is_connected() {
            self.connect().await?;
        }

        // Subscribe to ticker channels
        self.subscribe_tickers(symbols).await?;

        // Create channel for ticker updates
        let (tx, rx) = mpsc::unbounded_channel::<Result<Vec<Ticker>>>();
        let symbols_owned: Vec<String> = symbols.to_vec();
        let client = Arc::clone(&self.client);

        // Spawn task to process messages and filter ticker updates
        tokio::spawn(async move {
            while let Some(msg) = client.receive().await {
                // Check if this is a ticker message for ANY of our symbols
                if let Some(topic) = msg.get("topic").and_then(|t| t.as_str()) {
                    if let Some(symbol_part) = topic.strip_prefix("tickers.") {
                        if symbols_owned.iter().any(|s| s == symbol_part) {
                            match parse_ws_ticker(&msg, None) {
                                Ok(ticker) => {
                                    if tx.send(Ok(vec![ticker])).is_err() {
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
                }
            }
        });

        Ok(Box::pin(ReceiverStream::new(rx)))
    }
    /// * `depth` - Orderbook depth (1, 50, 200, or 500)
    pub async fn subscribe_orderbook(&self, symbol: &str, depth: u32) -> Result<()> {
        // Bybit supports orderbook depths: 1, 50, 200, 500
        let actual_depth = match depth {
            1 => 1,
            d if d <= 50 => 50,
            d if d <= 200 => 200,
            _ => 500,
        };

        let topic = format!("orderbook.{}.{}", actual_depth, symbol);

        // json! macro with literal values is infallible
        #[allow(clippy::disallowed_methods)]
        let msg = serde_json::json!({
            "op": "subscribe",
            "args": [topic]
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
    /// * `symbol` - Trading pair symbol (e.g., "BTCUSDT")
    pub async fn subscribe_trades(&self, symbol: &str) -> Result<()> {
        let topic = format!("publicTrade.{}", symbol);

        // json! macro with literal values is infallible
        #[allow(clippy::disallowed_methods)]
        let msg = serde_json::json!({
            "op": "subscribe",
            "args": [topic]
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
    /// * `symbol` - Trading pair symbol (e.g., "BTCUSDT")
    /// * `interval` - Kline interval (e.g., "1", "5", "60", "D")
    pub async fn subscribe_kline(&self, symbol: &str, interval: &str) -> Result<()> {
        let topic = format!("kline.{}.{}", interval, symbol);

        // json! macro with literal values is infallible
        #[allow(clippy::disallowed_methods)]
        let msg = serde_json::json!({
            "op": "subscribe",
            "args": [topic]
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
        // Parse stream name to determine topic
        let parts: Vec<&str> = stream_name.split(':').collect();
        if parts.len() < 2 {
            return Err(Error::invalid_request(format!(
                "Invalid stream name: {}",
                stream_name
            )));
        }

        let channel = parts[0];
        let symbol = parts[1];

        let topic = match channel {
            "ticker" => format!("tickers.{}", symbol),
            "orderbook" => format!("orderbook.50.{}", symbol),
            "trades" => format!("publicTrade.{}", symbol),
            "kline" => {
                if parts.len() >= 3 {
                    format!("kline.{}.{}", parts[2], symbol)
                } else {
                    return Err(Error::invalid_request(
                        "Kline unsubscribe requires interval",
                    ));
                }
            }
            _ => {
                return Err(Error::invalid_request(format!(
                    "Unknown channel: {}",
                    channel
                )));
            }
        };

        // json! macro with literal values is infallible
        #[allow(clippy::disallowed_methods)]
        let msg = serde_json::json!({
            "op": "unsubscribe",
            "args": [topic]
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
    /// * `symbol` - Trading pair symbol (e.g., "BTCUSDT")
    /// * `market` - Optional market information for symbol resolution
    ///
    /// # Returns
    ///
    /// A `MessageStream<Ticker>` that yields ticker updates.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ccxt_exchanges::bybit::ws::BybitWs;
    /// use futures::StreamExt;
    ///
    /// # async fn example() -> ccxt_core::error::Result<()> {
    /// let ws = BybitWs::new("wss://stream.bybit.com/v5/public/spot".to_string());
    /// ws.connect().await?;
    /// let mut stream = ws.watch_ticker("BTCUSDT", None).await?;
    /// while let Some(result) = stream.next().await {
    ///     match result {
    ///         Ok(ticker) => println!("Ticker: {:?}", ticker.last),
    ///         Err(e) => eprintln!("Error: {}", e),
    ///     }
    /// }
    /// # Ok(())
    /// # }
    /// ```
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
    /// * `symbol` - Trading pair symbol (e.g., "BTCUSDT")
    /// * `limit` - Optional depth limit (1, 50, 200, or 500)
    ///
    /// # Returns
    ///
    /// A `MessageStream<OrderBook>` that yields order book updates.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ccxt_exchanges::bybit::ws::BybitWs;
    /// use futures::StreamExt;
    ///
    /// # async fn example() -> ccxt_core::error::Result<()> {
    /// let ws = BybitWs::new("wss://stream.bybit.com/v5/public/spot".to_string());
    /// ws.connect().await?;
    /// let mut stream = ws.watch_order_book("BTCUSDT", Some(50)).await?;
    /// while let Some(result) = stream.next().await {
    ///     match result {
    ///         Ok(orderbook) => println!("Best bid: {:?}", orderbook.bids.first()),
    ///         Err(e) => eprintln!("Error: {}", e),
    ///     }
    /// }
    /// # Ok(())
    /// # }
    /// ```
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
        let depth = limit.unwrap_or(50);
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
    /// * `symbol` - Trading pair symbol (e.g., "BTCUSDT")
    /// * `market` - Optional market information for symbol resolution
    ///
    /// # Returns
    ///
    /// A `MessageStream<Vec<Trade>>` that yields trade updates.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ccxt_exchanges::bybit::ws::BybitWs;
    /// use futures::StreamExt;
    ///
    /// # async fn example() -> ccxt_core::error::Result<()> {
    /// let ws = BybitWs::new("wss://stream.bybit.com/v5/public/spot".to_string());
    /// ws.connect().await?;
    /// let mut stream = ws.watch_trades("BTCUSDT", None).await?;
    /// while let Some(result) = stream.next().await {
    ///     match result {
    ///         Ok(trades) => {
    ///             for trade in trades {
    ///                 println!("Trade: {:?} @ {:?}", trade.amount, trade.price);
    ///             }
    ///         }
    ///         Err(e) => eprintln!("Error: {}", e),
    ///     }
    /// }
    /// # Ok(())
    /// # }
    /// ```
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
    // Bybit V5 WebSocket ticker format:
    // {"topic":"tickers.BTCUSDT","type":"snapshot","data":{...},"ts":...}
    if let Some(topic) = msg.get("topic").and_then(|t| t.as_str()) {
        let expected_topic = format!("tickers.{}", symbol);
        topic == expected_topic
    } else {
        false
    }
}

/// Check if a WebSocket message is an orderbook message for the given symbol.
fn is_orderbook_message(msg: &Value, symbol: &str) -> bool {
    // Bybit V5 WebSocket orderbook format:
    // {"topic":"orderbook.50.BTCUSDT","type":"snapshot","data":{...},"ts":...}
    if let Some(topic) = msg.get("topic").and_then(|t| t.as_str()) {
        // Check if topic starts with "orderbook." and ends with the symbol
        topic.starts_with("orderbook.") && topic.ends_with(symbol)
    } else {
        false
    }
}

/// Check if a WebSocket message is a trade message for the given symbol.
fn is_trade_message(msg: &Value, symbol: &str) -> bool {
    // Bybit V5 WebSocket trade format:
    // {"topic":"publicTrade.BTCUSDT","type":"snapshot","data":[{...}],"ts":...}
    if let Some(topic) = msg.get("topic").and_then(|t| t.as_str()) {
        let expected_topic = format!("publicTrade.{}", symbol);
        topic == expected_topic
    } else {
        false
    }
}

/// Format a Bybit symbol (e.g., "BTCUSDT") to unified format (e.g., "BTC/USDT").
fn format_unified_symbol(symbol: &str) -> String {
    // Common quote currencies to detect
    let quote_currencies = ["USDT", "USDC", "BTC", "ETH", "EUR", "USD"];

    for quote in &quote_currencies {
        if let Some(base) = symbol.strip_suffix(quote) {
            if !base.is_empty() {
                return format!("{}/{}", base, quote);
            }
        }
    }

    // If no known quote currency found, return as-is
    symbol.to_string()
}

// ============================================================================
// WebSocket Message Parsers
// ============================================================================

/// Parse a WebSocket ticker message.
pub fn parse_ws_ticker(msg: &Value, market: Option<&Market>) -> Result<Ticker> {
    // Bybit V5 WebSocket ticker format:
    // {"topic":"tickers.BTCUSDT","type":"snapshot","data":{...},"ts":...}
    let data = msg
        .get("data")
        .ok_or_else(|| Error::invalid_request("Missing data in ticker message"))?;

    parse_ticker(data, market)
}

/// Parse a WebSocket orderbook message.
pub fn parse_ws_orderbook(msg: &Value, symbol: String) -> Result<OrderBook> {
    // Bybit V5 WebSocket orderbook format:
    // {"topic":"orderbook.50.BTCUSDT","type":"snapshot","data":{"s":"BTCUSDT","b":[...],"a":[...],"u":...,"seq":...},"ts":...}
    let data = msg
        .get("data")
        .ok_or_else(|| Error::invalid_request("Missing data in orderbook message"))?;

    parse_orderbook(data, symbol)
}

/// Parse a WebSocket trade message (single trade).
pub fn parse_ws_trade(msg: &Value, market: Option<&Market>) -> Result<Trade> {
    // Bybit V5 WebSocket trade format:
    // {"topic":"publicTrade.BTCUSDT","type":"snapshot","data":[{...}],"ts":...}
    let data = msg
        .get("data")
        .and_then(|d| d.as_array())
        .and_then(|arr| arr.first())
        .ok_or_else(|| Error::invalid_request("Missing data in trade message"))?;

    parse_trade(data, market)
}

/// Parse a WebSocket trade message (multiple trades).
pub fn parse_ws_trades(msg: &Value, market: Option<&Market>) -> Result<Vec<Trade>> {
    // Bybit V5 WebSocket trade format:
    // {"topic":"publicTrade.BTCUSDT","type":"snapshot","data":[{...}, {...}],"ts":...}
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
    fn test_bybit_ws_creation() {
        let ws = BybitWs::new("wss://stream.bybit.com/v5/public/spot".to_string());
        // WsClient is created successfully - config is private so we just verify creation works
        assert!(ws.subscriptions.try_read().is_ok());
    }

    #[tokio::test]
    async fn test_subscriptions_empty_by_default() {
        let ws = BybitWs::new("wss://stream.bybit.com/v5/public/spot".to_string());
        let subs = ws.subscriptions().await;
        assert!(subs.is_empty());
    }

    // ==================== Ticker Message Parsing Tests ====================

    #[test]
    fn test_parse_ws_ticker_snapshot() {
        let msg = serde_json::from_str(
            r#"{
                "topic": "tickers.BTCUSDT",
                "type": "snapshot",
                "data": {
                    "symbol": "BTCUSDT",
                    "lastPrice": "50000.00",
                    "highPrice24h": "51000.00",
                    "lowPrice24h": "49000.00",
                    "bid1Price": "49999.00",
                    "ask1Price": "50001.00",
                    "volume24h": "1000.5",
                    "time": "1700000000000"
                },
                "ts": 1700000000000
            }"#,
        )
        .unwrap();

        let ticker = parse_ws_ticker(&msg, None).unwrap();
        assert_eq!(ticker.symbol, "BTCUSDT");
        assert_eq!(ticker.last, Some(Price::new(dec!(50000.00))));
        assert_eq!(ticker.high, Some(Price::new(dec!(51000.00))));
        assert_eq!(ticker.low, Some(Price::new(dec!(49000.00))));
        assert_eq!(ticker.bid, Some(Price::new(dec!(49999.00))));
        assert_eq!(ticker.ask, Some(Price::new(dec!(50001.00))));
        assert_eq!(ticker.timestamp, 1700000000000);
    }

    #[test]
    fn test_parse_ws_ticker_with_market() {
        let msg = serde_json::from_str(
            r#"{
                "topic": "tickers.BTCUSDT",
                "type": "snapshot",
                "data": {
                    "symbol": "BTCUSDT",
                    "lastPrice": "50000.00",
                    "time": "1700000000000"
                },
                "ts": 1700000000000
            }"#,
        )
        .unwrap();

        let market = Market {
            id: "BTCUSDT".to_string(),
            symbol: "BTC/USDT".to_string(),
            base: "BTC".to_string(),
            quote: "USDT".to_string(),
            ..Default::default()
        };

        let ticker = parse_ws_ticker(&msg, Some(&market)).unwrap();
        assert_eq!(ticker.symbol, "BTC/USDT");
        assert_eq!(ticker.last, Some(Price::new(dec!(50000.00))));
    }

    #[test]
    fn test_parse_ws_ticker_missing_data() {
        let msg = serde_json::from_str(
            r#"{
                "topic": "tickers.BTCUSDT",
                "type": "snapshot",
                "ts": 1700000000000
            }"#,
        )
        .unwrap();

        let result = parse_ws_ticker(&msg, None);
        assert!(result.is_err());
    }

    // ==================== OrderBook Message Parsing Tests ====================

    #[test]
    fn test_parse_ws_orderbook_snapshot() {
        let msg = serde_json::from_str(
            r#"{
                "topic": "orderbook.50.BTCUSDT",
                "type": "snapshot",
                "data": {
                    "s": "BTCUSDT",
                    "b": [
                        ["50000.00", "1.5"],
                        ["49999.00", "2.0"],
                        ["49998.00", "0.5"]
                    ],
                    "a": [
                        ["50001.00", "1.0"],
                        ["50002.00", "3.0"],
                        ["50003.00", "2.5"]
                    ],
                    "u": 12345,
                    "seq": 67890,
                    "ts": "1700000000000"
                },
                "ts": 1700000000000
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
        assert_eq!(orderbook.bids[2].price, Price::new(dec!(49998.00)));

        // Verify asks are sorted in ascending order
        assert_eq!(orderbook.asks[0].price, Price::new(dec!(50001.00)));
        assert_eq!(orderbook.asks[1].price, Price::new(dec!(50002.00)));
        assert_eq!(orderbook.asks[2].price, Price::new(dec!(50003.00)));
    }

    #[test]
    fn test_parse_ws_orderbook_missing_data() {
        let msg = serde_json::from_str(
            r#"{
                "topic": "orderbook.50.BTCUSDT",
                "type": "snapshot",
                "ts": 1700000000000
            }"#,
        )
        .unwrap();

        let result = parse_ws_orderbook(&msg, "BTC/USDT".to_string());
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_ws_orderbook_empty_sides() {
        let msg = serde_json::from_str(
            r#"{
                "topic": "orderbook.50.BTCUSDT",
                "type": "snapshot",
                "data": {
                    "s": "BTCUSDT",
                    "b": [],
                    "a": [],
                    "u": 12345,
                    "ts": "1700000000000"
                },
                "ts": 1700000000000
            }"#,
        )
        .unwrap();

        let orderbook = parse_ws_orderbook(&msg, "BTC/USDT".to_string()).unwrap();
        assert!(orderbook.bids.is_empty());
        assert!(orderbook.asks.is_empty());
    }

    // ==================== Trade Message Parsing Tests ====================

    #[test]
    fn test_parse_ws_trade_single() {
        let msg = serde_json::from_str(
            r#"{
                "topic": "publicTrade.BTCUSDT",
                "type": "snapshot",
                "data": [{
                    "i": "123456789",
                    "T": 1700000000000,
                    "p": "50000.00",
                    "v": "0.5",
                    "S": "Buy",
                    "s": "BTCUSDT"
                }],
                "ts": 1700000000000
            }"#,
        )
        .unwrap();

        let trade = parse_ws_trade(&msg, None).unwrap();
        // Note: The trade parser uses different field names
        assert_eq!(trade.timestamp, 1700000000000);
    }

    #[test]
    fn test_parse_ws_trades_multiple() {
        let msg = serde_json::from_str(
            r#"{
                "topic": "publicTrade.BTCUSDT",
                "type": "snapshot",
                "data": [
                    {
                        "i": "123456789",
                        "T": 1700000000000,
                        "p": "50000.00",
                        "v": "0.5",
                        "S": "Buy",
                        "s": "BTCUSDT",
                        "side": "Buy",
                        "price": "50000.00",
                        "size": "0.5",
                        "time": "1700000000000"
                    },
                    {
                        "i": "123456790",
                        "T": 1700000000001,
                        "p": "50001.00",
                        "v": "1.0",
                        "S": "Sell",
                        "s": "BTCUSDT",
                        "side": "Sell",
                        "price": "50001.00",
                        "size": "1.0",
                        "time": "1700000000001"
                    }
                ],
                "ts": 1700000000001
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
                "topic": "publicTrade.BTCUSDT",
                "type": "snapshot",
                "ts": 1700000000000
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
                "topic": "publicTrade.BTCUSDT",
                "type": "snapshot",
                "data": [],
                "ts": 1700000000000
            }"#,
        )
        .unwrap();

        let trades = parse_ws_trades(&msg, None).unwrap();
        assert!(trades.is_empty());
    }

    // ==================== Message Type Detection Tests ====================

    #[test]
    fn test_is_ticker_message_true() {
        let msg = serde_json::from_str(
            r#"{
                "topic": "tickers.BTCUSDT",
                "type": "snapshot",
                "data": {},
                "ts": 1700000000000
            }"#,
        )
        .unwrap();

        assert!(is_ticker_message(&msg, "BTCUSDT"));
    }

    #[test]
    fn test_is_ticker_message_wrong_symbol() {
        let msg = serde_json::from_str(
            r#"{
                "topic": "tickers.ETHUSDT",
                "type": "snapshot",
                "data": {},
                "ts": 1700000000000
            }"#,
        )
        .unwrap();

        assert!(!is_ticker_message(&msg, "BTCUSDT"));
    }

    #[test]
    fn test_is_ticker_message_wrong_topic() {
        let msg = serde_json::from_str(
            r#"{
                "topic": "publicTrade.BTCUSDT",
                "type": "snapshot",
                "data": [],
                "ts": 1700000000000
            }"#,
        )
        .unwrap();

        assert!(!is_ticker_message(&msg, "BTCUSDT"));
    }

    #[test]
    fn test_is_orderbook_message_depth_50() {
        let msg = serde_json::from_str(
            r#"{
                "topic": "orderbook.50.BTCUSDT",
                "type": "snapshot",
                "data": {},
                "ts": 1700000000000
            }"#,
        )
        .unwrap();

        assert!(is_orderbook_message(&msg, "BTCUSDT"));
    }

    #[test]
    fn test_is_orderbook_message_depth_200() {
        let msg = serde_json::from_str(
            r#"{
                "topic": "orderbook.200.BTCUSDT",
                "type": "snapshot",
                "data": {},
                "ts": 1700000000000
            }"#,
        )
        .unwrap();

        assert!(is_orderbook_message(&msg, "BTCUSDT"));
    }

    #[test]
    fn test_is_trade_message_true() {
        let msg = serde_json::from_str(
            r#"{
                "topic": "publicTrade.BTCUSDT",
                "type": "snapshot",
                "data": [],
                "ts": 1700000000000
            }"#,
        )
        .unwrap();

        assert!(is_trade_message(&msg, "BTCUSDT"));
    }

    #[test]
    fn test_is_trade_message_wrong_topic() {
        let msg = serde_json::from_str(
            r#"{
                "topic": "tickers.BTCUSDT",
                "type": "snapshot",
                "data": {},
                "ts": 1700000000000
            }"#,
        )
        .unwrap();

        assert!(!is_trade_message(&msg, "BTCUSDT"));
    }

    // ==================== Symbol Formatting Tests ====================

    #[test]
    fn test_format_unified_symbol_usdt() {
        assert_eq!(format_unified_symbol("BTCUSDT"), "BTC/USDT");
        assert_eq!(format_unified_symbol("ETHUSDT"), "ETH/USDT");
    }

    #[test]
    fn test_format_unified_symbol_usdc() {
        assert_eq!(format_unified_symbol("BTCUSDC"), "BTC/USDC");
    }

    #[test]
    fn test_format_unified_symbol_btc() {
        assert_eq!(format_unified_symbol("ETHBTC"), "ETH/BTC");
    }

    #[test]
    fn test_format_unified_symbol_unknown() {
        // Unknown quote currency returns as-is
        assert_eq!(format_unified_symbol("BTCXYZ"), "BTCXYZ");
    }
}
