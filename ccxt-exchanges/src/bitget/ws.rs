//! Bitget WebSocket implementation.
//!
//! Provides real-time data streaming via WebSocket for Bitget exchange.
//! Supports public streams (ticker, orderbook, trades) and private streams
//! (balance, orders) with automatic reconnection.

use crate::bitget::auth::BitgetAuth;
use crate::bitget::parser::{parse_orderbook, parse_ticker, parse_trade};
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

/// Default ping interval for Bitget WebSocket (30 seconds).
const DEFAULT_PING_INTERVAL_MS: u64 = 30000;

/// Default reconnect delay (5 seconds).
const DEFAULT_RECONNECT_INTERVAL_MS: u64 = 5000;

/// Maximum reconnect attempts.
const MAX_RECONNECT_ATTEMPTS: u32 = 10;

/// Bitget WebSocket client.
///
/// Provides real-time data streaming for Bitget exchange.
pub struct BitgetWs {
    /// WebSocket client instance.
    client: Arc<WsClient>,
    /// Active subscriptions.
    subscriptions: Arc<RwLock<Vec<String>>>,
}

impl BitgetWs {
    /// Creates a new Bitget WebSocket client.
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
        let mut arg_map = serde_json::Map::new();
        arg_map.insert(
            "instType".to_string(),
            serde_json::Value::String("SPOT".to_string()),
        );
        arg_map.insert(
            "channel".to_string(),
            serde_json::Value::String("ticker".to_string()),
        );
        arg_map.insert(
            "instId".to_string(),
            serde_json::Value::String(symbol.to_string()),
        );
        let args = serde_json::Value::Array(vec![serde_json::Value::Object(arg_map)]);

        let mut msg_map = serde_json::Map::new();
        msg_map.insert(
            "op".to_string(),
            serde_json::Value::String("subscribe".to_string()),
        );
        msg_map.insert("args".to_string(), args);
        let msg = serde_json::Value::Object(msg_map);

        self.client.send_json(&msg).await?;

        let sub_key = format!("ticker:{}", symbol);
        self.subscriptions.write().await.push(sub_key);

        Ok(())
    }

    /// Subscribes to multiple ticker streams.
    ///
    /// # Arguments
    ///
    /// * `symbols` - List of trading pair symbols (e.g., `["BTCUSDT", "ETHUSDT"]`)
    pub async fn subscribe_tickers(&self, symbols: &[String]) -> Result<()> {
        let mut args = Vec::new();
        for symbol in symbols {
            let mut arg_map = serde_json::Map::new();
            arg_map.insert(
                "instType".to_string(),
                serde_json::Value::String("SPOT".to_string()),
            );
            arg_map.insert(
                "channel".to_string(),
                serde_json::Value::String("ticker".to_string()),
            );
            arg_map.insert(
                "instId".to_string(),
                serde_json::Value::String(symbol.clone()),
            );
            args.push(serde_json::Value::Object(arg_map));
        }

        let mut msg_map = serde_json::Map::new();
        msg_map.insert(
            "op".to_string(),
            serde_json::Value::String("subscribe".to_string()),
        );
        msg_map.insert("args".to_string(), serde_json::Value::Array(args));
        let msg = serde_json::Value::Object(msg_map);

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
    /// * `symbols` - List of trading pair symbols (e.g., `["BTCUSDT", "ETHUSDT"]`)
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

                    if channel == Some("ticker") {
                        if let Some(id) = inst_id {
                            if symbols_owned.iter().any(|s| s == id) {
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
    /// * `depth` - Orderbook depth (5, 15, or default)
    pub async fn subscribe_orderbook(&self, symbol: &str, depth: u32) -> Result<()> {
        let channel = match depth {
            5 => "books5",
            15 => "books15",
            _ => "books",
        };

        let mut arg_map = serde_json::Map::new();
        arg_map.insert(
            "instType".to_string(),
            serde_json::Value::String("SPOT".to_string()),
        );
        arg_map.insert(
            "channel".to_string(),
            serde_json::Value::String(channel.to_string()),
        );
        arg_map.insert(
            "instId".to_string(),
            serde_json::Value::String(symbol.to_string()),
        );
        let args = serde_json::Value::Array(vec![serde_json::Value::Object(arg_map)]);

        let mut msg_map = serde_json::Map::new();
        msg_map.insert(
            "op".to_string(),
            serde_json::Value::String("subscribe".to_string()),
        );
        msg_map.insert("args".to_string(), args);
        let msg = serde_json::Value::Object(msg_map);

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
        let mut arg_map = serde_json::Map::new();
        arg_map.insert(
            "instType".to_string(),
            serde_json::Value::String("SPOT".to_string()),
        );
        arg_map.insert(
            "channel".to_string(),
            serde_json::Value::String("trade".to_string()),
        );
        arg_map.insert(
            "instId".to_string(),
            serde_json::Value::String(symbol.to_string()),
        );
        let args = serde_json::Value::Array(vec![serde_json::Value::Object(arg_map)]);

        let mut msg_map = serde_json::Map::new();
        msg_map.insert(
            "op".to_string(),
            serde_json::Value::String("subscribe".to_string()),
        );
        msg_map.insert("args".to_string(), args);
        let msg = serde_json::Value::Object(msg_map);

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
    /// * `interval` - Kline interval (e.g., "1m", "5m", "1H")
    pub async fn subscribe_kline(&self, symbol: &str, interval: &str) -> Result<()> {
        let channel = format!("candle{}", interval);

        let mut arg_map = serde_json::Map::new();
        arg_map.insert(
            "instType".to_string(),
            serde_json::Value::String("SPOT".to_string()),
        );
        arg_map.insert(
            "channel".to_string(),
            serde_json::Value::String(channel.clone()),
        );
        arg_map.insert(
            "instId".to_string(),
            serde_json::Value::String(symbol.to_string()),
        );
        let args = serde_json::Value::Array(vec![serde_json::Value::Object(arg_map)]);

        let mut msg_map = serde_json::Map::new();
        msg_map.insert(
            "op".to_string(),
            serde_json::Value::String("subscribe".to_string()),
        );
        msg_map.insert("args".to_string(), args);
        let msg = serde_json::Value::Object(msg_map);

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

        let channel = parts[0];
        let symbol = parts[1];

        let bitget_channel = match channel {
            "ticker" => "ticker",
            "orderbook" => "books",
            "trades" => "trade",
            "kline" => {
                if parts.len() >= 3 {
                    // For kline, we need the interval
                    return self.unsubscribe_kline(symbol, parts[2]).await;
                }
                return Err(Error::invalid_request(
                    "Kline unsubscribe requires interval",
                ));
            }
            _ => channel,
        };

        let mut arg_map = serde_json::Map::new();
        arg_map.insert(
            "instType".to_string(),
            serde_json::Value::String("SPOT".to_string()),
        );
        arg_map.insert(
            "channel".to_string(),
            serde_json::Value::String(bitget_channel.to_string()),
        );
        arg_map.insert(
            "instId".to_string(),
            serde_json::Value::String(symbol.to_string()),
        );
        let args = serde_json::Value::Array(vec![serde_json::Value::Object(arg_map)]);

        let mut msg_map = serde_json::Map::new();
        msg_map.insert(
            "op".to_string(),
            serde_json::Value::String("unsubscribe".to_string()),
        );
        msg_map.insert("args".to_string(), args);
        let msg = serde_json::Value::Object(msg_map);

        self.client.send_json(&msg).await?;

        // Remove from subscriptions
        let mut subs = self.subscriptions.write().await;
        subs.retain(|s| s != &stream_name);

        Ok(())
    }

    /// Unsubscribes from a kline stream.
    async fn unsubscribe_kline(&self, symbol: &str, interval: &str) -> Result<()> {
        let channel = format!("candle{}", interval);

        let mut arg_map = serde_json::Map::new();
        arg_map.insert(
            "instType".to_string(),
            serde_json::Value::String("SPOT".to_string()),
        );
        arg_map.insert(
            "channel".to_string(),
            serde_json::Value::String(channel.clone()),
        );
        arg_map.insert(
            "instId".to_string(),
            serde_json::Value::String(symbol.to_string()),
        );
        let args = serde_json::Value::Array(vec![serde_json::Value::Object(arg_map)]);

        let mut msg_map = serde_json::Map::new();
        msg_map.insert(
            "op".to_string(),
            serde_json::Value::String("unsubscribe".to_string()),
        );
        msg_map.insert("args".to_string(), args);
        let msg = serde_json::Value::Object(msg_map);

        self.client.send_json(&msg).await?;

        let sub_key = format!("kline:{}:{}", symbol, interval);
        let mut subs = self.subscriptions.write().await;
        subs.retain(|s| s != &sub_key);

        Ok(())
    }

    /// Returns the list of active subscriptions.
    pub async fn subscriptions(&self) -> Vec<String> {
        self.subscriptions.read().await.clone()
    }

    // ==================== Private WebSocket Methods ====================

    /// Authenticates with the Bitget private WebSocket.
    ///
    /// Bitget WS login uses the same HMAC-SHA256 signing as REST API:
    /// sign(timestamp + "GET" + "/user/verify", secret)
    pub async fn login(&self, auth: &BitgetAuth) -> Result<()> {
        let timestamp = (chrono::Utc::now().timestamp_millis() / 1000).to_string();
        let sign = auth.sign(&timestamp, "GET", "/user/verify", "");

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
                            debug!("Bitget WebSocket login successful");
                            return Ok(());
                        }
                        let msg_text = resp
                            .get("msg")
                            .and_then(|m| m.as_str())
                            .unwrap_or("Unknown error");
                        return Err(Error::authentication(format!(
                            "Bitget WebSocket login failed: {} (code: {})",
                            msg_text, code
                        )));
                    }
                }
            }
        }

        Err(Error::authentication(
            "Bitget WebSocket login timed out waiting for response",
        ))
    }

    /// Subscribes to the orders channel (private).
    ///
    /// Receives real-time order updates for the specified instrument type.
    pub async fn subscribe_orders(&self, inst_type: Option<&str>) -> Result<()> {
        let inst_type = inst_type.unwrap_or("SPOT");

        #[allow(clippy::disallowed_methods)]
        let msg = serde_json::json!({
            "op": "subscribe",
            "args": [{
                "instType": inst_type,
                "channel": "orders",
                "instId": "default"
            }]
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
                "instType": "SPOT",
                "channel": "account",
                "coin": "default"
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
                            warn!(error = %e, "Failed to parse Bitget balance update");
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
                                let unified = format_unified_symbol(inst_id);
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
                                    warn!(error = %e, "Failed to parse Bitget order update");
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
                                .or_else(|| data.get("baseVolume"))
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
                                let unified = format_unified_symbol(inst_id);
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
                                    warn!(error = %e, "Failed to parse Bitget trade update");
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
    /// use ccxt_exchanges::bitget::ws::BitgetWs;
    /// use futures::StreamExt;
    ///
    /// # async fn example() -> ccxt_core::error::Result<()> {
    /// let ws = BitgetWs::new("wss://ws.bitget.com/v2/ws/public".to_string());
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
    /// * `limit` - Optional depth limit (5, 15, or full depth)
    ///
    /// # Returns
    ///
    /// A `MessageStream<OrderBook>` that yields order book updates.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ccxt_exchanges::bitget::ws::BitgetWs;
    /// use futures::StreamExt;
    ///
    /// # async fn example() -> ccxt_core::error::Result<()> {
    /// let ws = BitgetWs::new("wss://ws.bitget.com/v2/ws/public".to_string());
    /// ws.connect().await?;
    /// let mut stream = ws.watch_order_book("BTCUSDT", Some(5)).await?;
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
        let depth = limit.unwrap_or(15);
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
    /// use ccxt_exchanges::bitget::ws::BitgetWs;
    /// use futures::StreamExt;
    ///
    /// # async fn example() -> ccxt_core::error::Result<()> {
    /// let ws = BitgetWs::new("wss://ws.bitget.com/v2/ws/public".to_string());
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
    if let Some(arg) = msg.get("arg") {
        let channel = arg.get("channel").and_then(|c| c.as_str());
        let inst_id = arg.get("instId").and_then(|i| i.as_str());

        channel == Some("ticker") && inst_id == Some(symbol)
    } else {
        false
    }
}

/// Check if a WebSocket message is an orderbook message for the given symbol.
fn is_orderbook_message(msg: &Value, symbol: &str) -> bool {
    if let Some(arg) = msg.get("arg") {
        let channel = arg.get("channel").and_then(|c| c.as_str());
        let inst_id = arg.get("instId").and_then(|i| i.as_str());

        // Bitget uses books, books5, books15 for orderbook channels
        let is_orderbook_channel = channel.is_some_and(|c| c.starts_with("books"));
        is_orderbook_channel && inst_id == Some(symbol)
    } else {
        false
    }
}

/// Check if a WebSocket message is a trade message for the given symbol.
fn is_trade_message(msg: &Value, symbol: &str) -> bool {
    if let Some(arg) = msg.get("arg") {
        let channel = arg.get("channel").and_then(|c| c.as_str());
        let inst_id = arg.get("instId").and_then(|i| i.as_str());

        channel == Some("trade") && inst_id == Some(symbol)
    } else {
        false
    }
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

/// Format a Bitget symbol (e.g., "BTCUSDT") to unified format (e.g., "BTC/USDT").
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
    // Bitget WebSocket ticker format:
    // {"action":"snapshot","arg":{"instType":"SPOT","channel":"ticker","instId":"BTCUSDT"},"data":[{...}]}
    let data = msg
        .get("data")
        .and_then(|d| d.as_array())
        .and_then(|arr| arr.first())
        .ok_or_else(|| Error::invalid_request("Missing data in ticker message"))?;

    parse_ticker(data, market)
}

/// Parse a WebSocket orderbook message.
pub fn parse_ws_orderbook(msg: &Value, symbol: String) -> Result<OrderBook> {
    // Bitget WebSocket orderbook format:
    // {"action":"snapshot","arg":{"instType":"SPOT","channel":"books5","instId":"BTCUSDT"},"data":[{...}]}
    let data = msg
        .get("data")
        .and_then(|d| d.as_array())
        .and_then(|arr| arr.first())
        .ok_or_else(|| Error::invalid_request("Missing data in orderbook message"))?;

    parse_orderbook(data, symbol)
}

/// Parse a WebSocket trade message (single trade).
pub fn parse_ws_trade(msg: &Value, market: Option<&Market>) -> Result<Trade> {
    // Bitget WebSocket trade format:
    // {"action":"snapshot","arg":{"instType":"SPOT","channel":"trade","instId":"BTCUSDT"},"data":[{...}]}
    let data = msg
        .get("data")
        .and_then(|d| d.as_array())
        .and_then(|arr| arr.first())
        .ok_or_else(|| Error::invalid_request("Missing data in trade message"))?;

    parse_trade(data, market)
}

/// Parse a WebSocket trade message (multiple trades).
pub fn parse_ws_trades(msg: &Value, market: Option<&Market>) -> Result<Vec<Trade>> {
    // Bitget WebSocket trade format:
    // {"action":"snapshot","arg":{"instType":"SPOT","channel":"trade","instId":"BTCUSDT"},"data":[{...}, {...}]}
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
// Private WebSocket Message Parsers
// ============================================================================

/// Parse a WebSocket account/balance message into a Balance.
///
/// Bitget account update format:
/// ```json
/// {
///   "arg": {"instType": "SPOT", "channel": "account", "coin": "default"},
///   "data": [{
///     "coin": "USDT",
///     "available": "50000",
///     "frozen": "10000",
///     "uTime": "1700000000000"
///   }]
/// }
/// ```
pub fn parse_ws_balance(msg: &Value) -> Result<Balance> {
    let data_array = msg
        .get("data")
        .and_then(|d| d.as_array())
        .ok_or_else(|| Error::invalid_request("Missing data in account message"))?;

    let mut balances = HashMap::new();

    for data in data_array {
        // Bitget V2 sends individual coin updates
        let currency = data
            .get("coin")
            .and_then(|c| c.as_str())
            .unwrap_or_default()
            .to_string();

        if currency.is_empty() {
            continue;
        }

        let free = data
            .get("available")
            .and_then(|v| v.as_str())
            .and_then(|s| Decimal::from_str(s).ok())
            .unwrap_or_default();

        let used = data
            .get("frozen")
            .and_then(|v| v.as_str())
            .and_then(|s| Decimal::from_str(s).ok())
            .unwrap_or_default();

        let total = free + used;

        balances.insert(currency, BalanceEntry { free, used, total });
    }

    Ok(Balance {
        balances,
        info: HashMap::new(),
    })
}

/// Parse a WebSocket order update message into an Order.
///
/// Bitget order update format:
/// ```json
/// {
///   "instId": "BTCUSDT",
///   "ordId": "123456",
///   "clOrdId": "client123",
///   "side": "buy",
///   "ordType": "limit",
///   "px": "50000",
///   "sz": "0.1",
///   "fillPx": "49999",
///   "fillSz": "0.05",
///   "avgPx": "49999",
///   "status": "partially_filled",
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
    let symbol = format_unified_symbol(inst_id);

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
        .or_else(|| data.get("price"))
        .and_then(|v| v.as_str())
        .and_then(|s| Decimal::from_str(s).ok());

    let amount = data
        .get("sz")
        .or_else(|| data.get("size"))
        .and_then(|v| v.as_str())
        .and_then(|s| Decimal::from_str(s).ok())
        .unwrap_or_default();

    let filled = data
        .get("accFillSz")
        .or_else(|| data.get("baseVolume"))
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

    let status = match data.get("status").and_then(|v| v.as_str()) {
        Some("filled" | "full-fill") => OrderStatus::Closed,
        Some("cancelled" | "canceled") => OrderStatus::Cancelled,
        _ => OrderStatus::Open,
    };

    let fee = {
        let fee_amount = data
            .get("fee")
            .or_else(|| data.get("feeDetail"))
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
    let symbol = format_unified_symbol(inst_id);

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
        .or_else(|| data.get("price"))
        .and_then(|v| v.as_str())
        .and_then(|s| Decimal::from_str(s).ok())
        .unwrap_or_default();

    let fill_sz = data
        .get("fillSz")
        .or_else(|| data.get("baseVolume"))
        .and_then(|v| v.as_str())
        .and_then(|s| Decimal::from_str(s).ok())
        .unwrap_or_default();

    let timestamp = data
        .get("fillTime")
        .or_else(|| data.get("uTime"))
        .or_else(|| data.get("cTime"))
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
    fn test_bitget_ws_creation() {
        let ws = BitgetWs::new("wss://ws.bitget.com/v2/ws/public".to_string());
        // WsClient is created successfully - config is private so we just verify creation works
        assert!(ws.subscriptions.try_read().is_ok());
    }

    #[tokio::test]
    async fn test_subscriptions_empty_by_default() {
        let ws = BitgetWs::new("wss://ws.bitget.com/v2/ws/public".to_string());
        let subs = ws.subscriptions().await;
        assert!(subs.is_empty());
    }

    // ==================== Ticker Message Parsing Tests ====================

    #[test]
    fn test_parse_ws_ticker_snapshot() {
        let msg = serde_json::from_str(
            r#"{
                "action": "snapshot",
                "arg": {
                    "instType": "SPOT",
                    "channel": "ticker",
                    "instId": "BTCUSDT"
                },
                "data": [{
                    "instId": "BTCUSDT",
                    "lastPr": "50000.00",
                    "high24h": "51000.00",
                    "low24h": "49000.00",
                    "bidPr": "49999.00",
                    "askPr": "50001.00",
                    "baseVolume": "1000.5",
                    "ts": "1700000000000"
                }]
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
                "action": "snapshot",
                "arg": {
                    "instType": "SPOT",
                    "channel": "ticker",
                    "instId": "BTCUSDT"
                },
                "data": [{
                    "instId": "BTCUSDT",
                    "lastPr": "50000.00",
                    "ts": "1700000000000"
                }]
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
                "action": "snapshot",
                "arg": {
                    "instType": "SPOT",
                    "channel": "ticker",
                    "instId": "BTCUSDT"
                }
            }"#,
        )
        .unwrap();

        let result = parse_ws_ticker(&msg, None);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_ws_ticker_empty_data_array() {
        let msg = serde_json::from_str(
            r#"{
                "action": "snapshot",
                "arg": {
                    "instType": "SPOT",
                    "channel": "ticker",
                    "instId": "BTCUSDT"
                },
                "data": []
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
                "action": "snapshot",
                "arg": {
                    "instType": "SPOT",
                    "channel": "books5",
                    "instId": "BTCUSDT"
                },
                "data": [{
                    "bids": [
                        ["50000.00", "1.5"],
                        ["49999.00", "2.0"],
                        ["49998.00", "0.5"]
                    ],
                    "asks": [
                        ["50001.00", "1.0"],
                        ["50002.00", "3.0"],
                        ["50003.00", "2.5"]
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
        assert_eq!(orderbook.bids[2].price, Price::new(dec!(49998.00)));

        // Verify asks are sorted in ascending order
        assert_eq!(orderbook.asks[0].price, Price::new(dec!(50001.00)));
        assert_eq!(orderbook.asks[1].price, Price::new(dec!(50002.00)));
        assert_eq!(orderbook.asks[2].price, Price::new(dec!(50003.00)));
    }

    #[test]
    fn test_parse_ws_orderbook_update() {
        let msg = serde_json::from_str(
            r#"{
                "action": "update",
                "arg": {
                    "instType": "SPOT",
                    "channel": "books",
                    "instId": "ETHUSDT"
                },
                "data": [{
                    "bids": [
                        ["2000.00", "10.0"]
                    ],
                    "asks": [
                        ["2001.00", "5.0"]
                    ],
                    "ts": "1700000000001"
                }]
            }"#,
        )
        .unwrap();

        let orderbook = parse_ws_orderbook(&msg, "ETH/USDT".to_string()).unwrap();
        assert_eq!(orderbook.symbol, "ETH/USDT");
        assert_eq!(orderbook.bids.len(), 1);
        assert_eq!(orderbook.asks.len(), 1);
        assert_eq!(orderbook.timestamp, 1700000000001);
    }

    #[test]
    fn test_parse_ws_orderbook_missing_data() {
        let msg = serde_json::from_str(
            r#"{
                "action": "snapshot",
                "arg": {
                    "instType": "SPOT",
                    "channel": "books5",
                    "instId": "BTCUSDT"
                }
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
                "action": "snapshot",
                "arg": {
                    "instType": "SPOT",
                    "channel": "books5",
                    "instId": "BTCUSDT"
                },
                "data": [{
                    "bids": [],
                    "asks": [],
                    "ts": "1700000000000"
                }]
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
                "action": "snapshot",
                "arg": {
                    "instType": "SPOT",
                    "channel": "trade",
                    "instId": "BTCUSDT"
                },
                "data": [{
                    "tradeId": "123456789",
                    "symbol": "BTCUSDT",
                    "side": "buy",
                    "price": "50000.00",
                    "size": "0.5",
                    "ts": "1700000000000"
                }]
            }"#,
        )
        .unwrap();

        let trade = parse_ws_trade(&msg, None).unwrap();
        assert_eq!(trade.id, Some("123456789".to_string()));
        assert_eq!(trade.side, ccxt_core::types::OrderSide::Buy);
        assert_eq!(trade.price, Price::new(dec!(50000.00)));
        assert_eq!(
            trade.amount,
            ccxt_core::types::financial::Amount::new(dec!(0.5))
        );
        assert_eq!(trade.timestamp, 1700000000000);
    }

    #[test]
    fn test_parse_ws_trades_multiple() {
        let msg = serde_json::from_str(
            r#"{
                "action": "snapshot",
                "arg": {
                    "instType": "SPOT",
                    "channel": "trade",
                    "instId": "BTCUSDT"
                },
                "data": [
                    {
                        "tradeId": "123456789",
                        "symbol": "BTCUSDT",
                        "side": "buy",
                        "price": "50000.00",
                        "size": "0.5",
                        "ts": "1700000000000"
                    },
                    {
                        "tradeId": "123456790",
                        "symbol": "BTCUSDT",
                        "side": "sell",
                        "price": "50001.00",
                        "size": "1.0",
                        "ts": "1700000000001"
                    }
                ]
            }"#,
        )
        .unwrap();

        let trades = parse_ws_trades(&msg, None).unwrap();
        assert_eq!(trades.len(), 2);

        assert_eq!(trades[0].id, Some("123456789".to_string()));
        assert_eq!(trades[0].side, ccxt_core::types::OrderSide::Buy);

        assert_eq!(trades[1].id, Some("123456790".to_string()));
        assert_eq!(trades[1].side, ccxt_core::types::OrderSide::Sell);
    }

    #[test]
    fn test_parse_ws_trade_sell_side() {
        let msg = serde_json::from_str(
            r#"{
                "action": "snapshot",
                "arg": {
                    "instType": "SPOT",
                    "channel": "trade",
                    "instId": "BTCUSDT"
                },
                "data": [{
                    "tradeId": "123456789",
                    "symbol": "BTCUSDT",
                    "side": "sell",
                    "price": "50000.00",
                    "size": "0.5",
                    "ts": "1700000000000"
                }]
            }"#,
        )
        .unwrap();

        let trade = parse_ws_trade(&msg, None).unwrap();
        assert_eq!(trade.side, ccxt_core::types::OrderSide::Sell);
    }

    #[test]
    fn test_parse_ws_trade_missing_data() {
        let msg = serde_json::from_str(
            r#"{
                "action": "snapshot",
                "arg": {
                    "instType": "SPOT",
                    "channel": "trade",
                    "instId": "BTCUSDT"
                }
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
                "action": "snapshot",
                "arg": {
                    "instType": "SPOT",
                    "channel": "trade",
                    "instId": "BTCUSDT"
                },
                "data": []
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
                "action": "snapshot",
                "arg": {
                    "instType": "SPOT",
                    "channel": "ticker",
                    "instId": "BTCUSDT"
                },
                "data": [{}]
            }"#,
        )
        .unwrap();

        assert!(is_ticker_message(&msg, "BTCUSDT"));
    }

    #[test]
    fn test_is_ticker_message_wrong_symbol() {
        let msg = serde_json::from_str(
            r#"{
                "action": "snapshot",
                "arg": {
                    "instType": "SPOT",
                    "channel": "ticker",
                    "instId": "ETHUSDT"
                },
                "data": [{}]
            }"#,
        )
        .unwrap();

        assert!(!is_ticker_message(&msg, "BTCUSDT"));
    }

    #[test]
    fn test_is_ticker_message_wrong_channel() {
        let msg = serde_json::from_str(
            r#"{
                "action": "snapshot",
                "arg": {
                    "instType": "SPOT",
                    "channel": "trade",
                    "instId": "BTCUSDT"
                },
                "data": [{}]
            }"#,
        )
        .unwrap();

        assert!(!is_ticker_message(&msg, "BTCUSDT"));
    }

    #[test]
    fn test_is_orderbook_message_books5() {
        let msg = serde_json::from_str(
            r#"{
                "arg": {
                    "instType": "SPOT",
                    "channel": "books5",
                    "instId": "BTCUSDT"
                }
            }"#,
        )
        .unwrap();

        assert!(is_orderbook_message(&msg, "BTCUSDT"));
    }

    #[test]
    fn test_is_orderbook_message_books15() {
        let msg = serde_json::from_str(
            r#"{
                "arg": {
                    "instType": "SPOT",
                    "channel": "books15",
                    "instId": "BTCUSDT"
                }
            }"#,
        )
        .unwrap();

        assert!(is_orderbook_message(&msg, "BTCUSDT"));
    }

    #[test]
    fn test_is_orderbook_message_books() {
        let msg = serde_json::from_str(
            r#"{
                "arg": {
                    "instType": "SPOT",
                    "channel": "books",
                    "instId": "BTCUSDT"
                }
            }"#,
        )
        .unwrap();

        assert!(is_orderbook_message(&msg, "BTCUSDT"));
    }

    #[test]
    fn test_is_trade_message_true() {
        let msg = serde_json::from_str(
            r#"{
                "arg": {
                    "instType": "SPOT",
                    "channel": "trade",
                    "instId": "BTCUSDT"
                }
            }"#,
        )
        .unwrap();

        assert!(is_trade_message(&msg, "BTCUSDT"));
    }

    #[test]
    fn test_is_trade_message_wrong_channel() {
        let msg = serde_json::from_str(
            r#"{
                "arg": {
                    "instType": "SPOT",
                    "channel": "ticker",
                    "instId": "BTCUSDT"
                }
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

    // ==================== Private Message Type Detection Tests ====================

    #[test]
    fn test_is_account_message_true() {
        let msg: Value = serde_json::from_str(
            r#"{
                "arg": {"instType": "SPOT", "channel": "account", "coin": "default"},
                "data": [{}]
            }"#,
        )
        .unwrap();
        assert!(is_account_message(&msg));
    }

    #[test]
    fn test_is_account_message_wrong_channel() {
        let msg: Value = serde_json::from_str(
            r#"{
                "arg": {"instType": "SPOT", "channel": "orders"},
                "data": [{}]
            }"#,
        )
        .unwrap();
        assert!(!is_account_message(&msg));
    }

    #[test]
    fn test_is_orders_message_true() {
        let msg: Value = serde_json::from_str(
            r#"{
                "arg": {"instType": "SPOT", "channel": "orders", "instId": "default"},
                "data": [{}]
            }"#,
        )
        .unwrap();
        assert!(is_orders_message(&msg));
    }

    #[test]
    fn test_is_orders_message_wrong_channel() {
        let msg: Value = serde_json::from_str(
            r#"{
                "arg": {"instType": "SPOT", "channel": "account"},
                "data": [{}]
            }"#,
        )
        .unwrap();
        assert!(!is_orders_message(&msg));
    }

    // ==================== Private Balance Parsing Tests ====================

    #[test]
    fn test_parse_ws_balance_single_coin() {
        let msg: Value = serde_json::from_str(
            r#"{
                "arg": {"instType": "SPOT", "channel": "account", "coin": "default"},
                "data": [{
                    "coin": "USDT",
                    "available": "50000.50",
                    "frozen": "10000.25",
                    "uTime": "1700000000000"
                }]
            }"#,
        )
        .unwrap();

        let balance = parse_ws_balance(&msg).unwrap();
        assert_eq!(balance.balances.len(), 1);

        let usdt = balance.balances.get("USDT").unwrap();
        assert_eq!(usdt.free, dec!(50000.50));
        assert_eq!(usdt.used, dec!(10000.25));
        assert_eq!(usdt.total, dec!(60000.75));
    }

    #[test]
    fn test_parse_ws_balance_multiple_coins() {
        let msg: Value = serde_json::from_str(
            r#"{
                "arg": {"instType": "SPOT", "channel": "account"},
                "data": [
                    {"coin": "USDT", "available": "50000", "frozen": "10000"},
                    {"coin": "BTC", "available": "1.5", "frozen": "0.5"}
                ]
            }"#,
        )
        .unwrap();

        let balance = parse_ws_balance(&msg).unwrap();
        assert_eq!(balance.balances.len(), 2);

        let usdt = balance.balances.get("USDT").unwrap();
        assert_eq!(usdt.free, dec!(50000));
        assert_eq!(usdt.total, dec!(60000));

        let btc = balance.balances.get("BTC").unwrap();
        assert_eq!(btc.free, dec!(1.5));
        assert_eq!(btc.total, dec!(2.0));
    }

    #[test]
    fn test_parse_ws_balance_missing_data() {
        let msg: Value = serde_json::from_str(
            r#"{
                "arg": {"instType": "SPOT", "channel": "account"}
            }"#,
        )
        .unwrap();

        let result = parse_ws_balance(&msg);
        assert!(result.is_err());
    }

    // ==================== Private Order Parsing Tests ====================

    #[test]
    fn test_parse_ws_order_limit_buy() {
        let data: Value = serde_json::from_str(
            r#"{
                "instId": "BTCUSDT",
                "ordId": "123456789",
                "clOrdId": "client123",
                "side": "buy",
                "ordType": "limit",
                "px": "50000",
                "sz": "0.1",
                "accFillSz": "0.05",
                "avgPx": "49999",
                "status": "partially_filled",
                "fee": "-0.5",
                "feeCcy": "USDT",
                "cTime": "1700000000000"
            }"#,
        )
        .unwrap();

        let order = parse_ws_order(&data).unwrap();
        assert_eq!(order.symbol, "BTC/USDT");
        assert_eq!(order.id, "123456789");
        assert_eq!(order.client_order_id, Some("client123".to_string()));
        assert_eq!(order.side, OrderSide::Buy);
        assert_eq!(order.order_type, OrderType::Limit);
        assert_eq!(order.price, Some(dec!(50000)));
        assert_eq!(order.amount, dec!(0.1));
        assert_eq!(order.filled, Some(dec!(0.05)));
        assert_eq!(order.average, Some(dec!(49999)));
        assert_eq!(order.status, OrderStatus::Open); // partially_filled is still open
        assert_eq!(order.timestamp, Some(1700000000000));
        assert!(order.fee.is_some());
        let fee = order.fee.unwrap();
        assert_eq!(fee.cost, dec!(0.5)); // abs value
    }

    #[test]
    fn test_parse_ws_order_market_sell() {
        let data: Value = serde_json::from_str(
            r#"{
                "instId": "ETHUSDT",
                "ordId": "987654321",
                "side": "sell",
                "ordType": "market",
                "sz": "1.0",
                "accFillSz": "1.0",
                "avgPx": "2000",
                "status": "filled",
                "fee": "-2.0",
                "feeCcy": "USDT",
                "cTime": "1700000000000"
            }"#,
        )
        .unwrap();

        let order = parse_ws_order(&data).unwrap();
        assert_eq!(order.symbol, "ETH/USDT");
        assert_eq!(order.side, OrderSide::Sell);
        assert_eq!(order.order_type, OrderType::Market);
        assert_eq!(order.status, OrderStatus::Closed);
        assert_eq!(order.cost, Some(dec!(2000))); // 1.0 * 2000
    }

    #[test]
    fn test_parse_ws_order_cancelled() {
        let data: Value = serde_json::from_str(
            r#"{
                "instId": "BTCUSDT",
                "ordId": "111222333",
                "side": "buy",
                "ordType": "limit",
                "px": "45000",
                "sz": "0.5",
                "status": "cancelled",
                "cTime": "1700000000000"
            }"#,
        )
        .unwrap();

        let order = parse_ws_order(&data).unwrap();
        assert_eq!(order.status, OrderStatus::Cancelled);
    }

    // ==================== Private Trade (My Trade) Parsing Tests ====================

    #[test]
    fn test_parse_ws_my_trade_basic() {
        let data: Value = serde_json::from_str(
            r#"{
                "instId": "BTCUSDT",
                "ordId": "123456789",
                "tradeId": "trade001",
                "side": "buy",
                "fillPx": "50000",
                "fillSz": "0.1",
                "fillTime": "1700000000000",
                "fillFee": "-5.0",
                "fillFeeCcy": "USDT"
            }"#,
        )
        .unwrap();

        let trade = parse_ws_my_trade(&data).unwrap();
        assert_eq!(trade.symbol, "BTC/USDT");
        assert_eq!(trade.id, Some("trade001".to_string()));
        assert_eq!(trade.order, Some("123456789".to_string()));
        assert_eq!(trade.side, OrderSide::Buy);
        assert_eq!(trade.price, Price::new(dec!(50000)));
        assert_eq!(trade.amount, Amount::new(dec!(0.1)));
        assert_eq!(trade.cost, Some(Cost::new(dec!(5000)))); // 50000 * 0.1
        assert_eq!(trade.timestamp, 1700000000000);
        assert!(trade.fee.is_some());
        let fee = trade.fee.unwrap();
        assert_eq!(fee.cost, dec!(5.0)); // abs value
        assert_eq!(fee.currency, "USDT");
    }

    #[test]
    fn test_parse_ws_my_trade_sell() {
        let data: Value = serde_json::from_str(
            r#"{
                "instId": "ETHUSDT",
                "ordId": "987654321",
                "tradeId": "trade002",
                "side": "sell",
                "fillPx": "2000",
                "fillSz": "0.5",
                "fillTime": "1700000000001"
            }"#,
        )
        .unwrap();

        let trade = parse_ws_my_trade(&data).unwrap();
        assert_eq!(trade.symbol, "ETH/USDT");
        assert_eq!(trade.side, OrderSide::Sell);
        assert_eq!(trade.price, Price::new(dec!(2000)));
        assert_eq!(trade.amount, Amount::new(dec!(0.5)));
        assert_eq!(trade.cost, Some(Cost::new(dec!(1000)))); // 2000 * 0.5
        assert!(trade.fee.is_none()); // No fee fields
    }

    #[test]
    fn test_parse_ws_my_trade_fallback_fields() {
        // Test fallback field names (price instead of fillPx, etc.)
        let data: Value = serde_json::from_str(
            r#"{
                "instId": "BTCUSDT",
                "ordId": "111222333",
                "side": "buy",
                "price": "48000",
                "baseVolume": "0.2",
                "uTime": "1700000000002",
                "fee": "-1.5",
                "feeCcy": "USDT"
            }"#,
        )
        .unwrap();

        let trade = parse_ws_my_trade(&data).unwrap();
        assert_eq!(trade.price, Price::new(dec!(48000)));
        assert_eq!(trade.amount, Amount::new(dec!(0.2)));
        assert_eq!(trade.timestamp, 1700000000002);
        assert!(trade.fee.is_some());
    }
}
