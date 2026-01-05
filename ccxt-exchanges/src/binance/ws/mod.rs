//! Binance WebSocket implementation
//!
//! Provides WebSocket real-time data stream subscriptions for the Binance exchange

mod handlers;
mod listen_key;
mod streams;
mod subscriptions;
mod user_data;

// Re-export public types for backward compatibility
pub use handlers::MessageRouter;
pub use listen_key::ListenKeyManager;
pub use streams::*;
pub use subscriptions::{ReconnectConfig, Subscription, SubscriptionManager, SubscriptionType};

use crate::binance::{Binance, parser};
use ccxt_core::error::{Error, Result};
use ccxt_core::types::{
    Balance, BidAsk, MarkPrice, MarketType, OHLCV, Order, OrderBook, Position, Ticker, Trade,
};
use ccxt_core::ws_client::{WsClient, WsConfig};
use serde_json::Value;
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{Mutex, RwLock};

/// Binance WebSocket client wrapper
pub struct BinanceWs {
    pub(crate) client: Arc<WsClient>,
    listen_key: Arc<RwLock<Option<String>>>,
    listen_key_manager: Option<Arc<ListenKeyManager>>,
    auto_reconnect_coordinator: Arc<Mutex<Option<ccxt_core::ws_client::AutoReconnectCoordinator>>>,
    pub(crate) tickers: Arc<Mutex<HashMap<String, Ticker>>>,
    pub(crate) bids_asks: Arc<Mutex<HashMap<String, BidAsk>>>,
    #[allow(dead_code)]
    mark_prices: Arc<Mutex<HashMap<String, MarkPrice>>>,
    pub(crate) orderbooks: Arc<Mutex<HashMap<String, OrderBook>>>,
    pub(crate) trades: Arc<Mutex<HashMap<String, VecDeque<Trade>>>>,
    pub(crate) ohlcvs: Arc<Mutex<HashMap<String, VecDeque<OHLCV>>>>,
    pub(crate) balances: Arc<RwLock<HashMap<String, Balance>>>,
    pub(crate) orders: Arc<RwLock<HashMap<String, HashMap<String, Order>>>>,
    pub(crate) my_trades: Arc<RwLock<HashMap<String, VecDeque<Trade>>>>,
    pub(crate) positions: Arc<RwLock<HashMap<String, HashMap<String, Position>>>>,
}

impl std::fmt::Debug for BinanceWs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BinanceWs")
            .field("is_connected", &self.client.is_connected())
            .field("state", &self.client.state())
            .finish_non_exhaustive()
    }
}

impl BinanceWs {
    /// Creates a new Binance WebSocket client
    pub fn new(url: String) -> Self {
        let config = WsConfig {
            url,
            connect_timeout: 10000,
            ping_interval: 180000,
            reconnect_interval: 5000,
            max_reconnect_attempts: 5,
            auto_reconnect: true,
            enable_compression: false,
            pong_timeout: 90000,
            ..Default::default()
        };

        Self {
            client: Arc::new(WsClient::new(config)),
            listen_key: Arc::new(RwLock::new(None)),
            listen_key_manager: None,
            auto_reconnect_coordinator: Arc::new(Mutex::new(None)),
            tickers: Arc::new(Mutex::new(HashMap::new())),
            bids_asks: Arc::new(Mutex::new(HashMap::new())),
            mark_prices: Arc::new(Mutex::new(HashMap::new())),
            orderbooks: Arc::new(Mutex::new(HashMap::new())),
            trades: Arc::new(Mutex::new(HashMap::new())),
            ohlcvs: Arc::new(Mutex::new(HashMap::new())),
            balances: Arc::new(RwLock::new(HashMap::new())),
            orders: Arc::new(RwLock::new(HashMap::new())),
            my_trades: Arc::new(RwLock::new(HashMap::new())),
            positions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Creates a WebSocket client with a listen key manager
    pub fn new_with_auth(url: String, binance: Arc<Binance>) -> Self {
        let config = WsConfig {
            url,
            connect_timeout: 10000,
            ping_interval: 180000,
            reconnect_interval: 5000,
            max_reconnect_attempts: 5,
            auto_reconnect: true,
            enable_compression: false,
            pong_timeout: 90000,
            ..Default::default()
        };

        Self {
            client: Arc::new(WsClient::new(config)),
            listen_key: Arc::new(RwLock::new(None)),
            listen_key_manager: Some(Arc::new(ListenKeyManager::new(binance))),
            auto_reconnect_coordinator: Arc::new(Mutex::new(None)),
            tickers: Arc::new(Mutex::new(HashMap::new())),
            bids_asks: Arc::new(Mutex::new(HashMap::new())),
            mark_prices: Arc::new(Mutex::new(HashMap::new())),
            orderbooks: Arc::new(Mutex::new(HashMap::new())),
            trades: Arc::new(Mutex::new(HashMap::new())),
            ohlcvs: Arc::new(Mutex::new(HashMap::new())),
            balances: Arc::new(RwLock::new(HashMap::new())),
            orders: Arc::new(RwLock::new(HashMap::new())),
            my_trades: Arc::new(RwLock::new(HashMap::new())),
            positions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Connects to the WebSocket server
    pub async fn connect(&self) -> Result<()> {
        self.client.connect().await?;

        let mut coordinator_guard = self.auto_reconnect_coordinator.lock().await;
        if coordinator_guard.is_none() {
            let coordinator = self.client.clone().create_auto_reconnect_coordinator();
            coordinator.start().await;
            *coordinator_guard = Some(coordinator);
            tracing::info!("Auto-reconnect coordinator started");
        }

        Ok(())
    }

    /// Disconnects from the WebSocket server
    pub async fn disconnect(&self) -> Result<()> {
        let mut coordinator_guard = self.auto_reconnect_coordinator.lock().await;
        if let Some(coordinator) = coordinator_guard.take() {
            coordinator.stop().await;
            tracing::info!("Auto-reconnect coordinator stopped");
        }

        if let Some(manager) = &self.listen_key_manager {
            manager.stop_auto_refresh().await;
        }

        self.client.disconnect().await
    }

    /// Connects to the user data stream
    pub async fn connect_user_stream(&self) -> Result<()> {
        let manager = self.listen_key_manager.as_ref()
            .ok_or_else(|| Error::invalid_request(
                "Listen key manager not available. Use new_with_auth() to create authenticated WebSocket"
            ))?;

        let listen_key = manager.get_or_create().await?;
        let _user_stream_url = format!("wss://stream.binance.com:9443/ws/{}", listen_key);

        self.client.connect().await?;
        manager.start_auto_refresh().await;
        *self.listen_key.write().await = Some(listen_key);

        Ok(())
    }

    /// Closes the user data stream
    pub async fn close_user_stream(&self) -> Result<()> {
        if let Some(manager) = &self.listen_key_manager {
            manager.delete().await?;
        }
        *self.listen_key.write().await = None;
        Ok(())
    }

    /// Returns the active listen key, when available
    pub async fn get_listen_key(&self) -> Option<String> {
        if let Some(manager) = &self.listen_key_manager {
            manager.get_current().await
        } else {
            self.listen_key.read().await.clone()
        }
    }

    /// Subscribes to the ticker stream for a symbol
    pub async fn subscribe_ticker(&self, symbol: &str) -> Result<()> {
        let stream = format!("{}@ticker", symbol.to_lowercase());
        self.client
            .subscribe(stream, Some(symbol.to_string()), None)
            .await
    }

    /// Subscribes to the 24-hour ticker stream for all symbols
    pub async fn subscribe_all_tickers(&self) -> Result<()> {
        self.client
            .subscribe("!ticker@arr".to_string(), None, None)
            .await
    }

    /// Subscribes to real-time trade executions for a symbol
    pub async fn subscribe_trades(&self, symbol: &str) -> Result<()> {
        let stream = format!("{}@trade", symbol.to_lowercase());
        self.client
            .subscribe(stream, Some(symbol.to_string()), None)
            .await
    }

    /// Subscribes to the aggregated trade stream for a symbol
    pub async fn subscribe_agg_trades(&self, symbol: &str) -> Result<()> {
        let stream = format!("{}@aggTrade", symbol.to_lowercase());
        self.client
            .subscribe(stream, Some(symbol.to_string()), None)
            .await
    }

    /// Subscribes to the order book depth stream
    pub async fn subscribe_orderbook(
        &self,
        symbol: &str,
        levels: u32,
        update_speed: &str,
    ) -> Result<()> {
        let stream = if update_speed == "100ms" {
            format!("{}@depth{}@100ms", symbol.to_lowercase(), levels)
        } else {
            format!("{}@depth{}", symbol.to_lowercase(), levels)
        };

        self.client
            .subscribe(stream, Some(symbol.to_string()), None)
            .await
    }

    /// Subscribes to the diff order book stream
    pub async fn subscribe_orderbook_diff(
        &self,
        symbol: &str,
        update_speed: Option<&str>,
    ) -> Result<()> {
        let stream = if let Some(speed) = update_speed {
            if speed == "100ms" {
                format!("{}@depth@100ms", symbol.to_lowercase())
            } else {
                format!("{}@depth", symbol.to_lowercase())
            }
        } else {
            format!("{}@depth", symbol.to_lowercase())
        };

        self.client
            .subscribe(stream, Some(symbol.to_string()), None)
            .await
    }

    /// Subscribes to Kline (candlestick) data for a symbol
    pub async fn subscribe_kline(&self, symbol: &str, interval: &str) -> Result<()> {
        let stream = format!("{}@kline_{}", symbol.to_lowercase(), interval);
        self.client
            .subscribe(stream, Some(symbol.to_string()), None)
            .await
    }

    /// Subscribes to the mini ticker stream for a symbol
    pub async fn subscribe_mini_ticker(&self, symbol: &str) -> Result<()> {
        let stream = format!("{}@miniTicker", symbol.to_lowercase());
        self.client
            .subscribe(stream, Some(symbol.to_string()), None)
            .await
    }

    /// Subscribes to the mini ticker stream for all symbols
    pub async fn subscribe_all_mini_tickers(&self) -> Result<()> {
        self.client
            .subscribe("!miniTicker@arr".to_string(), None, None)
            .await
    }

    /// Cancels an existing subscription
    pub async fn unsubscribe(&self, stream: String) -> Result<()> {
        self.client.unsubscribe(stream, None).await
    }

    /// Receives the next available message
    pub async fn receive(&self) -> Option<Value> {
        self.client.receive().await
    }

    /// Indicates whether the WebSocket connection is active
    pub fn is_connected(&self) -> bool {
        self.client.is_connected()
    }

    /// Returns the current connection state.
    pub fn state(&self) -> ccxt_core::ws_client::WsConnectionState {
        self.client.state()
    }

    /// Returns the list of active subscriptions.
    ///
    /// Returns a vector of subscription channel names that are currently active.
    /// This method retrieves the actual subscriptions from the underlying WsClient's
    /// subscription manager, providing accurate state tracking.
    pub fn subscriptions(&self) -> Vec<String> {
        self.client.subscriptions()
    }

    /// Returns the number of active subscriptions.
    pub fn subscription_count(&self) -> usize {
        self.client.subscription_count()
    }

    /// Watches a single ticker stream (internal helper)
    async fn watch_ticker_internal(&self, symbol: &str, channel_name: &str) -> Result<Ticker> {
        let stream = format!("{}@{}", symbol.to_lowercase(), channel_name);

        self.client
            .subscribe(stream.clone(), Some(symbol.to_string()), None)
            .await?;

        loop {
            if let Some(message) = self.client.receive().await {
                if message.get("result").is_some() {
                    continue;
                }

                if let Ok(ticker) = parser::parse_ws_ticker(&message, None) {
                    let mut tickers = self.tickers.lock().await;
                    tickers.insert(ticker.symbol.clone(), ticker.clone());
                    return Ok(ticker);
                }
            }
        }
    }

    /// Watches multiple ticker streams (internal helper)
    async fn watch_tickers_internal(
        &self,
        symbols: Option<Vec<String>>,
        channel_name: &str,
    ) -> Result<HashMap<String, Ticker>> {
        let streams: Vec<String> = if let Some(syms) = symbols.as_ref() {
            syms.iter()
                .map(|s| format!("{}@{}", s.to_lowercase(), channel_name))
                .collect()
        } else {
            vec![format!("!{}@arr", channel_name)]
        };

        for stream in &streams {
            self.client.subscribe(stream.clone(), None, None).await?;
        }

        let mut result = HashMap::new();

        loop {
            if let Some(message) = self.client.receive().await {
                if message.get("result").is_some() {
                    continue;
                }

                if let Some(arr) = message.as_array() {
                    for item in arr {
                        if let Ok(ticker) = parser::parse_ws_ticker(item, None) {
                            let symbol = ticker.symbol.clone();

                            if let Some(syms) = &symbols {
                                if syms.contains(&symbol.to_lowercase()) {
                                    result.insert(symbol.clone(), ticker.clone());
                                }
                            } else {
                                result.insert(symbol.clone(), ticker.clone());
                            }

                            let mut tickers = self.tickers.lock().await;
                            tickers.insert(symbol, ticker);
                        }
                    }

                    if let Some(syms) = &symbols {
                        if result.len() == syms.len() {
                            return Ok(result);
                        }
                    } else {
                        return Ok(result);
                    }
                } else if let Ok(ticker) = parser::parse_ws_ticker(&message, None) {
                    let symbol = ticker.symbol.clone();
                    result.insert(symbol.clone(), ticker.clone());

                    let mut tickers = self.tickers.lock().await;
                    tickers.insert(symbol, ticker);

                    if let Some(syms) = &symbols {
                        if result.len() == syms.len() {
                            return Ok(result);
                        }
                    }
                }
            }
        }
    }

    /// Processes an order book delta update (internal helper)
    async fn handle_orderbook_delta(
        &self,
        symbol: &str,
        delta_message: &Value,
        is_futures: bool,
    ) -> Result<()> {
        handlers::handle_orderbook_delta(symbol, delta_message, is_futures, &self.orderbooks).await
    }

    /// Retrieves an order book snapshot and initializes cached state (internal helper)
    async fn fetch_orderbook_snapshot(
        &self,
        exchange: &Binance,
        symbol: &str,
        limit: Option<i64>,
        is_futures: bool,
    ) -> Result<OrderBook> {
        handlers::fetch_orderbook_snapshot(exchange, symbol, limit, is_futures, &self.orderbooks)
            .await
    }

    /// Watches a single order book stream (internal helper)
    async fn watch_orderbook_internal(
        &self,
        exchange: &Binance,
        symbol: &str,
        limit: Option<i64>,
        update_speed: i32,
        is_futures: bool,
    ) -> Result<OrderBook> {
        let stream = if update_speed == 100 {
            format!("{}@depth@100ms", symbol.to_lowercase())
        } else {
            format!("{}@depth", symbol.to_lowercase())
        };

        self.client
            .subscribe(stream.clone(), Some(symbol.to_string()), None)
            .await?;

        tokio::time::sleep(Duration::from_millis(500)).await;

        let _snapshot = self
            .fetch_orderbook_snapshot(exchange, symbol, limit, is_futures)
            .await?;

        loop {
            if let Some(message) = self.client.receive().await {
                if message.get("result").is_some() {
                    continue;
                }

                if let Some(event_type) = message.get("e").and_then(serde_json::Value::as_str) {
                    if event_type == "depthUpdate" {
                        match self
                            .handle_orderbook_delta(symbol, &message, is_futures)
                            .await
                        {
                            Ok(()) => {
                                let orderbooks = self.orderbooks.lock().await;
                                if let Some(ob) = orderbooks.get(symbol) {
                                    if ob.is_synced {
                                        return Ok(ob.clone());
                                    }
                                }
                            }
                            Err(e) => {
                                let err_msg = e.to_string();

                                if err_msg.contains("RESYNC_NEEDED") {
                                    tracing::warn!("Resync needed for {}: {}", symbol, err_msg);

                                    let current_time = chrono::Utc::now().timestamp_millis();
                                    let should_resync = {
                                        let orderbooks = self.orderbooks.lock().await;
                                        if let Some(ob) = orderbooks.get(symbol) {
                                            ob.should_resync(current_time)
                                        } else {
                                            true
                                        }
                                    };

                                    if should_resync {
                                        tracing::info!("Initiating resync for {}", symbol);

                                        {
                                            let mut orderbooks = self.orderbooks.lock().await;
                                            if let Some(ob) = orderbooks.get_mut(symbol) {
                                                ob.reset_for_resync();
                                                ob.mark_resync_initiated(current_time);
                                            }
                                        }

                                        tokio::time::sleep(Duration::from_millis(500)).await;

                                        match self
                                            .fetch_orderbook_snapshot(
                                                exchange, symbol, limit, is_futures,
                                            )
                                            .await
                                        {
                                            Ok(_) => {
                                                tracing::info!(
                                                    "Resync completed successfully for {}",
                                                    symbol
                                                );
                                                continue;
                                            }
                                            Err(resync_err) => {
                                                tracing::error!(
                                                    "Resync failed for {}: {}",
                                                    symbol,
                                                    resync_err
                                                );
                                                return Err(resync_err);
                                            }
                                        }
                                    }
                                    tracing::debug!("Resync rate limited for {}, skipping", symbol);
                                }
                                tracing::error!("Failed to handle orderbook delta: {}", err_msg);
                            }
                        }
                    }
                }
            }
        }
    }

    /// Watches multiple order book streams (internal helper)
    async fn watch_orderbooks_internal(
        &self,
        exchange: &Binance,
        symbols: Vec<String>,
        limit: Option<i64>,
        update_speed: i32,
        is_futures: bool,
    ) -> Result<HashMap<String, OrderBook>> {
        if symbols.len() > 200 {
            return Err(Error::invalid_request(
                "Binance supports max 200 symbols per connection",
            ));
        }

        for symbol in &symbols {
            let stream = if update_speed == 100 {
                format!("{}@depth@100ms", symbol.to_lowercase())
            } else {
                format!("{}@depth", symbol.to_lowercase())
            };

            self.client
                .subscribe(stream, Some(symbol.clone()), None)
                .await?;
        }

        tokio::time::sleep(Duration::from_millis(500)).await;

        for symbol in &symbols {
            let _ = self
                .fetch_orderbook_snapshot(exchange, symbol, limit, is_futures)
                .await;
        }

        let mut result = HashMap::new();
        let mut update_count = 0;

        while update_count < symbols.len() {
            if let Some(message) = self.client.receive().await {
                if message.get("result").is_some() {
                    continue;
                }

                if let Some(event_type) = message.get("e").and_then(serde_json::Value::as_str) {
                    if event_type == "depthUpdate" {
                        if let Some(msg_symbol) =
                            message.get("s").and_then(serde_json::Value::as_str)
                        {
                            if let Err(e) = self
                                .handle_orderbook_delta(msg_symbol, &message, is_futures)
                                .await
                            {
                                tracing::error!("Failed to handle orderbook delta: {}", e);
                                continue;
                            }

                            update_count += 1;
                        }
                    }
                }
            }
        }

        let orderbooks = self.orderbooks.lock().await;
        for symbol in &symbols {
            if let Some(ob) = orderbooks.get(symbol) {
                result.insert(symbol.clone(), ob.clone());
            }
        }

        Ok(result)
    }

    /// Returns cached ticker snapshot
    pub async fn get_cached_ticker(&self, symbol: &str) -> Option<Ticker> {
        let tickers = self.tickers.lock().await;
        tickers.get(symbol).cloned()
    }

    /// Returns all cached ticker snapshots
    pub async fn get_all_cached_tickers(&self) -> HashMap<String, Ticker> {
        let tickers = self.tickers.lock().await;
        tickers.clone()
    }

    /// Handles balance update messages (internal helper)
    async fn handle_balance_message(&self, message: &Value, account_type: &str) -> Result<()> {
        user_data::handle_balance_message(message, account_type, &self.balances).await
    }

    /// Parses a WebSocket trade message
    fn parse_ws_trade(data: &Value) -> Result<Trade> {
        user_data::parse_ws_trade(data)
    }

    /// Filters cached personal trades by symbol, time range, and limit
    async fn filter_my_trades(
        &self,
        symbol: Option<&str>,
        since: Option<i64>,
        limit: Option<usize>,
    ) -> Result<Vec<Trade>> {
        let trades_map = self.my_trades.read().await;

        let mut trades: Vec<Trade> = if let Some(sym) = symbol {
            trades_map
                .get(sym)
                .map(|symbol_trades| symbol_trades.iter().cloned().collect())
                .unwrap_or_default()
        } else {
            trades_map
                .values()
                .flat_map(|symbol_trades| symbol_trades.iter().cloned())
                .collect()
        };

        if let Some(since_ts) = since {
            trades.retain(|trade| trade.timestamp >= since_ts);
        }

        trades.sort_by(|a, b| {
            let ts_a = a.timestamp;
            let ts_b = b.timestamp;
            ts_b.cmp(&ts_a)
        });

        if let Some(lim) = limit {
            trades.truncate(lim);
        }

        Ok(trades)
    }

    /// Parses a WebSocket position payload
    fn parse_ws_position(data: &Value) -> Result<Position> {
        user_data::parse_ws_position(data)
    }

    /// Filters cached positions by symbol, time range, and limit
    async fn filter_positions(
        &self,
        symbols: Option<&[String]>,
        since: Option<i64>,
        limit: Option<usize>,
    ) -> Result<Vec<Position>> {
        let positions_map = self.positions.read().await;

        let mut positions: Vec<Position> = if let Some(syms) = symbols {
            syms.iter()
                .filter_map(|sym| positions_map.get(sym))
                .flat_map(|side_map| side_map.values().cloned())
                .collect()
        } else {
            positions_map
                .values()
                .flat_map(|side_map| side_map.values().cloned())
                .collect()
        };

        if let Some(since_ts) = since {
            positions.retain(|pos| pos.timestamp.is_some_and(|ts| ts >= since_ts));
        }

        positions.sort_by(|a, b| {
            let ts_a = a.timestamp.unwrap_or(0);
            let ts_b = b.timestamp.unwrap_or(0);
            ts_b.cmp(&ts_a)
        });

        if let Some(lim) = limit {
            positions.truncate(lim);
        }

        Ok(positions)
    }
}

// Include Binance impl methods in a separate file to keep mod.rs manageable
include!("binance_impl.rs");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_binance_ws_creation() {
        let ws = BinanceWs::new(WS_BASE_URL.to_string());
        assert!(ws.listen_key.try_read().is_ok());
    }

    #[test]
    fn test_stream_format() {
        let symbol = "btcusdt";

        let ticker_stream = format!("{}@ticker", symbol);
        assert_eq!(ticker_stream, "btcusdt@ticker");

        let trade_stream = format!("{}@trade", symbol);
        assert_eq!(trade_stream, "btcusdt@trade");

        let depth_stream = format!("{}@depth20", symbol);
        assert_eq!(depth_stream, "btcusdt@depth20");

        let kline_stream = format!("{}@kline_1m", symbol);
        assert_eq!(kline_stream, "btcusdt@kline_1m");
    }

    #[tokio::test]
    async fn test_subscription_manager_basic() {
        let manager = SubscriptionManager::new();
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();

        assert_eq!(manager.active_count(), 0);
        assert!(!manager.has_subscription("btcusdt@ticker").await);

        manager
            .add_subscription(
                "btcusdt@ticker".to_string(),
                "BTCUSDT".to_string(),
                SubscriptionType::Ticker,
                tx.clone(),
            )
            .await
            .unwrap();

        assert_eq!(manager.active_count(), 1);
        assert!(manager.has_subscription("btcusdt@ticker").await);

        let sub = manager.get_subscription("btcusdt@ticker").await;
        assert!(sub.is_some());
        let sub = sub.unwrap();
        assert_eq!(sub.stream, "btcusdt@ticker");
        assert_eq!(sub.symbol, "BTCUSDT");
        assert_eq!(sub.sub_type, SubscriptionType::Ticker);

        manager.remove_subscription("btcusdt@ticker").await.unwrap();
        assert_eq!(manager.active_count(), 0);
        assert!(!manager.has_subscription("btcusdt@ticker").await);
    }

    #[test]
    fn test_symbol_conversion() {
        let symbol = "BTC/USDT";
        let binance_symbol = symbol.replace('/', "").to_lowercase();
        assert_eq!(binance_symbol, "btcusdt");
    }
}
