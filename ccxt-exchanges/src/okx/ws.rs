//! OKX WebSocket implementation.
//!
//! Provides real-time data streaming via WebSocket for OKX exchange.
//! Supports public streams (ticker, orderbook, trades) and private streams
//! (balance, orders) with automatic reconnection.

use crate::okx::auth::OkxAuth;
use crate::okx::parser::{parse_orderbook, parse_ticker, parse_trade};
use ccxt_core::error::{Error, Result};
use ccxt_core::types::{
    Balance, BalanceEntry, Fee, Market, Order, OrderBook, OrderSide, OrderStatus, OrderType,
    Ticker, Trade,
    financial::{Amount, Cost, Price},
};
use ccxt_core::ws_client::{WsClient, WsConfig, WsConnectionState};
use ccxt_core::ws_exchange::MessageStream;
use futures::Stream;
use rust_decimal::Decimal;
use rust_decimal::prelude::FromStr;
use serde_json::Value;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use tokio::sync::{RwLock, mpsc};
use tracing::{debug, warn};

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

    /// Subscribes to multiple ticker streams.
    ///
    /// # Arguments
    ///
    /// * `symbols` - List of trading pair symbols (e.g., `["BTC-USDT", "ETH-USDT"]`)
    pub async fn subscribe_tickers(&self, symbols: &[String]) -> Result<()> {
        let mut args = Vec::new();
        for symbol in symbols {
            let mut arg_map = serde_json::Map::new();
            arg_map.insert(
                "channel".to_string(),
                serde_json::Value::String("tickers".to_string()),
            );
            arg_map.insert(
                "instId".to_string(),
                serde_json::Value::String(symbol.clone()),
            );
            args.push(serde_json::Value::Object(arg_map));
        }

        // json! macro with literal values is infallible
        #[allow(clippy::disallowed_methods)]
        let msg = serde_json::json!({
            "op": "subscribe",
            "args": args
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
    /// * `symbols` - List of trading pair symbols (e.g., `["BTC-USDT", "ETH-USDT"]`)
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
                if let Some(arg) = msg.get("arg") {
                    let channel = arg.get("channel").and_then(|c| c.as_str());
                    let inst_id = arg.get("instId").and_then(|i| i.as_str());

                    if channel == Some("tickers") {
                        if let Some(id) = inst_id {
                            let has_data = msg
                                .get("data")
                                .and_then(|d| d.as_array())
                                .and_then(|arr| arr.first())
                                .is_some();
                            if has_data && symbols_owned.iter().any(|s| s == id) {
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
            }
        });

        Ok(Box::pin(ReceiverStream::new(rx)))
    }
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

    // ========================================================================
    // Private WebSocket Methods
    // ========================================================================

    /// Sends a login message to authenticate the private WebSocket connection.
    ///
    /// OKX WebSocket login format:
    /// ```json
    /// {
    ///   "op": "login",
    ///   "args": [{
    ///     "apiKey": "...",
    ///     "passphrase": "...",
    ///     "timestamp": "...",
    ///     "sign": "..."
    ///   }]
    /// }
    /// ```
    ///
    /// The signature is: HMAC-SHA256(timestamp + "GET" + "/users/self/verify", secret)
    pub async fn login(&self, auth: &OkxAuth) -> Result<()> {
        let timestamp = (chrono::Utc::now().timestamp_millis() / 1000).to_string();
        let sign = auth.sign(&timestamp, "GET", "/users/self/verify", "");

        #[allow(clippy::disallowed_methods)]
        let msg = serde_json::json!({
            "op": "login",
            "args": [{
                "apiKey": auth.api_key(),
                "passphrase": auth.passphrase(),
                "timestamp": timestamp,
                "sign": sign
            }]
        });

        self.client.send_json(&msg).await?;

        // Wait for login response
        let timeout = tokio::time::Duration::from_secs(10);
        let start = tokio::time::Instant::now();

        while start.elapsed() < timeout {
            if let Some(resp) = self.client.receive().await {
                if let Some(event) = resp.get("event").and_then(|e| e.as_str()) {
                    if event == "login" {
                        let code = resp.get("code").and_then(|c| c.as_str()).unwrap_or("1");
                        if code == "0" {
                            debug!("OKX WebSocket login successful");
                            return Ok(());
                        }
                        let msg_text = resp
                            .get("msg")
                            .and_then(|m| m.as_str())
                            .unwrap_or("Unknown error");
                        return Err(Error::authentication(format!(
                            "OKX WebSocket login failed: {} (code: {})",
                            msg_text, code
                        )));
                    }
                }
            }
        }

        Err(Error::authentication(
            "OKX WebSocket login timed out waiting for response",
        ))
    }

    /// Subscribes to the orders channel (private).
    ///
    /// Receives real-time order updates for all instrument types.
    pub async fn subscribe_orders(&self, inst_type: Option<&str>) -> Result<()> {
        let mut arg = serde_json::Map::new();
        arg.insert("channel".to_string(), Value::String("orders".to_string()));
        arg.insert(
            "instType".to_string(),
            Value::String(inst_type.unwrap_or("ANY").to_string()),
        );

        #[allow(clippy::disallowed_methods)]
        let msg = serde_json::json!({
            "op": "subscribe",
            "args": [Value::Object(arg)]
        });

        self.client.send_json(&msg).await?;

        self.subscriptions.write().await.push("orders".to_string());

        Ok(())
    }

    /// Subscribes to the account channel (private).
    ///
    /// Receives real-time balance/account updates.
    pub async fn subscribe_account(&self) -> Result<()> {
        #[allow(clippy::disallowed_methods)]
        let msg = serde_json::json!({
            "op": "subscribe",
            "args": [{
                "channel": "account"
            }]
        });

        self.client.send_json(&msg).await?;

        self.subscriptions.write().await.push("account".to_string());

        Ok(())
    }

    /// Watches balance updates via the private WebSocket.
    ///
    /// Requires prior authentication via `login()`.
    pub async fn watch_balance(&self) -> Result<MessageStream<Balance>> {
        if !self.is_connected() {
            self.connect().await?;
        }

        self.subscribe_account().await?;

        let (tx, rx) = mpsc::unbounded_channel::<Result<Balance>>();
        let client = Arc::clone(&self.client);

        tokio::spawn(async move {
            while let Some(msg) = client.receive().await {
                if is_account_message(&msg) {
                    match parse_ws_balance(&msg) {
                        Ok(balance) => {
                            if tx.send(Ok(balance)).is_err() {
                                break;
                            }
                        }
                        Err(e) => {
                            warn!(error = %e, "Failed to parse OKX balance update");
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

    /// Watches order updates via the private WebSocket.
    ///
    /// Requires prior authentication via `login()`.
    pub async fn watch_orders(
        &self,
        symbol_filter: Option<String>,
    ) -> Result<MessageStream<Order>> {
        if !self.is_connected() {
            self.connect().await?;
        }

        self.subscribe_orders(None).await?;

        let (tx, rx) = mpsc::unbounded_channel::<Result<Order>>();
        let client = Arc::clone(&self.client);

        tokio::spawn(async move {
            while let Some(msg) = client.receive().await {
                if is_orders_message(&msg) {
                    if let Some(data_array) = msg.get("data").and_then(|d| d.as_array()) {
                        for data in data_array {
                            // Apply symbol filter if specified
                            if let Some(ref filter) = symbol_filter {
                                let inst_id =
                                    data.get("instId").and_then(|i| i.as_str()).unwrap_or("");
                                let unified = inst_id.replace('-', "/");
                                if unified != *filter && inst_id != filter.as_str() {
                                    continue;
                                }
                            }

                            match parse_ws_order(data) {
                                Ok(order) => {
                                    if tx.send(Ok(order)).is_err() {
                                        return;
                                    }
                                }
                                Err(e) => {
                                    warn!(error = %e, "Failed to parse OKX order update");
                                    if tx.send(Err(e)).is_err() {
                                        return;
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

    /// Watches trade fills via the private WebSocket (orders channel).
    ///
    /// Extracts trade information from order execution reports.
    /// Requires prior authentication via `login()`.
    pub async fn watch_my_trades(
        &self,
        symbol_filter: Option<String>,
    ) -> Result<MessageStream<Trade>> {
        if !self.is_connected() {
            self.connect().await?;
        }

        self.subscribe_orders(None).await?;

        let (tx, rx) = mpsc::unbounded_channel::<Result<Trade>>();
        let client = Arc::clone(&self.client);

        tokio::spawn(async move {
            while let Some(msg) = client.receive().await {
                if is_orders_message(&msg) {
                    if let Some(data_array) = msg.get("data").and_then(|d| d.as_array()) {
                        for data in data_array {
                            // Only emit trades for filled/partially filled orders
                            let fill_sz = data
                                .get("fillSz")
                                .and_then(|v| v.as_str())
                                .and_then(|s| s.parse::<f64>().ok())
                                .unwrap_or(0.0);

                            if fill_sz <= 0.0 {
                                continue;
                            }

                            // Apply symbol filter
                            if let Some(ref filter) = symbol_filter {
                                let inst_id =
                                    data.get("instId").and_then(|i| i.as_str()).unwrap_or("");
                                let unified = inst_id.replace('-', "/");
                                if unified != *filter && inst_id != filter.as_str() {
                                    continue;
                                }
                            }

                            match parse_ws_my_trade(data) {
                                Ok(trade) => {
                                    if tx.send(Ok(trade)).is_err() {
                                        return;
                                    }
                                }
                                Err(e) => {
                                    warn!(error = %e, "Failed to parse OKX trade update");
                                    if tx.send(Err(e)).is_err() {
                                        return;
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
        let has_data = msg
            .get("data")
            .and_then(|d| d.as_array())
            .and_then(|arr| arr.first())
            .is_some();
        channel == Some("tickers") && inst_id == Some(symbol) && has_data
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
        let has_data = msg
            .get("data")
            .and_then(|d| d.as_array())
            .and_then(|arr| arr.first())
            .is_some();
        if let (Some(ch), Some(id)) = (channel, inst_id) {
            ch.starts_with("books") && id == symbol && has_data
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
        let has_data = msg
            .get("data")
            .and_then(|d| d.as_array())
            .and_then(|arr| arr.first())
            .is_some();
        channel == Some("trades") && inst_id == Some(symbol) && has_data
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

// ============================================================================
// Private Message Type Detection Helpers
// ============================================================================

/// Check if a WebSocket message is an account/balance update.
fn is_account_message(msg: &Value) -> bool {
    if let Some(arg) = msg.get("arg") {
        arg.get("channel").and_then(|c| c.as_str()) == Some("account")
    } else {
        false
    }
}

/// Check if a WebSocket message is an orders update.
fn is_orders_message(msg: &Value) -> bool {
    if let Some(arg) = msg.get("arg") {
        arg.get("channel").and_then(|c| c.as_str()) == Some("orders")
    } else {
        false
    }
}

// ============================================================================
// Private WebSocket Message Parsers
// ============================================================================

/// Parse a WebSocket account/balance message into a Balance.
///
/// OKX account update format:
/// ```json
/// {
///   "arg": {"channel": "account"},
///   "data": [{
///     "uTime": "1700000000000",
///     "totalEq": "100000.00",
///     "details": [
///       {"ccy": "USDT", "availBal": "50000", "frozenBal": "10000", "eq": "60000"},
///       {"ccy": "BTC", "availBal": "1.5", "frozenBal": "0.5", "eq": "2.0"}
///     ]
///   }]
/// }
/// ```
pub fn parse_ws_balance(msg: &Value) -> Result<Balance> {
    let data = msg
        .get("data")
        .and_then(|d| d.as_array())
        .and_then(|arr| arr.first())
        .ok_or_else(|| Error::invalid_request("Missing data in account message"))?;

    // uTime is available but Balance struct doesn't have a timestamp field
    let mut info = HashMap::new();

    if let Some(details) = data.get("details").and_then(|d| d.as_array()) {
        for detail in details {
            let currency = detail
                .get("ccy")
                .and_then(|c| c.as_str())
                .unwrap_or_default()
                .to_string();

            if currency.is_empty() {
                continue;
            }

            let free = detail
                .get("availBal")
                .and_then(|v| v.as_str())
                .and_then(|s| Decimal::from_str(s).ok())
                .unwrap_or_default();

            let used = detail
                .get("frozenBal")
                .and_then(|v| v.as_str())
                .and_then(|s| Decimal::from_str(s).ok())
                .unwrap_or_default();

            let total = free + used;

            info.insert(currency, BalanceEntry { free, used, total });
        }
    }

    Ok(Balance {
        balances: info,
        info: HashMap::new(),
    })
}

/// Parse a WebSocket order update message into an Order.
///
/// OKX order update format:
/// ```json
/// {
///   "instId": "BTC-USDT",
///   "ordId": "123456",
///   "clOrdId": "client123",
///   "side": "buy",
///   "ordType": "limit",
///   "px": "50000",
///   "sz": "0.1",
///   "fillPx": "49999",
///   "fillSz": "0.05",
///   "avgPx": "49999",
///   "state": "partially_filled",
///   "fee": "-0.5",
///   "feeCcy": "USDT",
///   "uTime": "1700000000000",
///   "cTime": "1700000000000"
/// }
/// ```
pub fn parse_ws_order(data: &Value) -> Result<Order> {
    let inst_id = data
        .get("instId")
        .and_then(|v| v.as_str())
        .unwrap_or_default();
    let symbol = inst_id.replace('-', "/");

    let id = data
        .get("ordId")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();

    let client_order_id = data
        .get("clOrdId")
        .and_then(|v| v.as_str())
        .map(ToString::to_string);

    let side = match data.get("side").and_then(|v| v.as_str()) {
        Some("sell") => OrderSide::Sell,
        _ => OrderSide::Buy,
    };

    let order_type = match data.get("ordType").and_then(|v| v.as_str()) {
        Some("market") => OrderType::Market,
        _ => OrderType::Limit,
    };

    let price = data
        .get("px")
        .and_then(|v| v.as_str())
        .and_then(|s| Decimal::from_str(s).ok());

    let amount = data
        .get("sz")
        .and_then(|v| v.as_str())
        .and_then(|s| Decimal::from_str(s).ok())
        .unwrap_or_default();

    let filled = data
        .get("accFillSz")
        .and_then(|v| v.as_str())
        .and_then(|s| Decimal::from_str(s).ok());

    let average = data
        .get("avgPx")
        .and_then(|v| v.as_str())
        .and_then(|s| Decimal::from_str(s).ok())
        .filter(|d| !d.is_zero());

    let cost = match (filled, average) {
        (Some(f), Some(a)) => Some(f * a),
        _ => None,
    };

    let status = match data.get("state").and_then(|v| v.as_str()) {
        Some("filled") => OrderStatus::Closed,
        Some("canceled" | "cancelled") => OrderStatus::Cancelled,
        _ => OrderStatus::Open,
    };

    let fee = {
        let fee_amount = data
            .get("fee")
            .and_then(|v| v.as_str())
            .and_then(|s| Decimal::from_str(s).ok());
        let fee_currency = data
            .get("feeCcy")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();
        fee_amount.map(|f| Fee::new(fee_currency, f.abs()))
    };

    let timestamp = data
        .get("cTime")
        .and_then(|v| v.as_str())
        .and_then(|s| s.parse::<i64>().ok());

    Ok(Order {
        id,
        client_order_id,
        symbol,
        order_type,
        side,
        price,
        amount,
        filled,
        remaining: None,
        cost,
        average,
        status,
        fee,
        fees: None,
        timestamp,
        datetime: None,
        last_trade_timestamp: None,
        time_in_force: None,
        post_only: None,
        reduce_only: None,
        stop_price: None,
        trigger_price: None,
        take_profit_price: None,
        stop_loss_price: None,
        trailing_delta: None,
        trailing_percent: None,
        activation_price: None,
        callback_rate: None,
        working_type: None,
        trades: None,
        info: HashMap::new(),
    })
}

/// Parse a WebSocket order fill into a Trade (my trade).
///
/// Extracts the latest fill information from an order update.
pub fn parse_ws_my_trade(data: &Value) -> Result<Trade> {
    let inst_id = data
        .get("instId")
        .and_then(|v| v.as_str())
        .unwrap_or_default();
    let symbol = inst_id.replace('-', "/");

    let trade_id = data
        .get("tradeId")
        .and_then(|v| v.as_str())
        .map(ToString::to_string);

    let order_id = data
        .get("ordId")
        .and_then(|v| v.as_str())
        .map(ToString::to_string);

    let side = match data.get("side").and_then(|v| v.as_str()) {
        Some("buy") => OrderSide::Buy,
        _ => OrderSide::Sell,
    };

    let fill_px = data
        .get("fillPx")
        .and_then(|v| v.as_str())
        .and_then(|s| Decimal::from_str(s).ok())
        .unwrap_or_default();

    let fill_sz = data
        .get("fillSz")
        .and_then(|v| v.as_str())
        .and_then(|s| Decimal::from_str(s).ok())
        .unwrap_or_default();

    let timestamp = data
        .get("fillTime")
        .or_else(|| data.get("uTime"))
        .and_then(|v| v.as_str())
        .and_then(|s| s.parse::<i64>().ok())
        .unwrap_or(0);

    let fee = {
        let fee_amount = data
            .get("fillFee")
            .or_else(|| data.get("fee"))
            .and_then(|v| v.as_str())
            .and_then(|s| Decimal::from_str(s).ok());
        let fee_currency = data
            .get("fillFeeCcy")
            .or_else(|| data.get("feeCcy"))
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();
        fee_amount.map(|f| Fee::new(fee_currency, f.abs()))
    };

    Ok(Trade {
        id: trade_id,
        order: order_id,
        symbol,
        trade_type: None,
        side,
        taker_or_maker: None,
        price: Price(fill_px),
        amount: Amount(fill_sz),
        cost: Some(Cost(fill_px * fill_sz)),
        fee,
        timestamp,
        datetime: None,
        info: HashMap::new(),
    })
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
