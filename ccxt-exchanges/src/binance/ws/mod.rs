//! Binance WebSocket implementation
//!
//! Provides WebSocket real-time data stream subscriptions for the Binance exchange

/// Connection manager module
pub mod connection_manager;
mod handlers;
mod listen_key;
mod streams;
mod subscriptions;
pub(crate) mod user_data;

// Re-export public types for backward compatibility
pub use connection_manager::BinanceConnectionManager;
pub use handlers::MessageRouter;
pub use listen_key::ListenKeyManager;
pub use streams::*;
pub use subscriptions::{ReconnectConfig, Subscription, SubscriptionManager, SubscriptionType};

use crate::binance::{Binance, parser};
use ccxt_core::error::{Error, Result};
use ccxt_core::types::{
    Balance, BidAsk, MarkPrice, MarketType, OHLCV, Order, OrderBook, Position, Ticker, Trade,
};
use serde_json::Value;
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{Mutex, RwLock};

const MAX_TRADES: usize = 1000;
const MAX_OHLCVS: usize = 1000;

/// Binance WebSocket client wrapper
pub struct BinanceWs {
    pub(crate) message_router: Arc<MessageRouter>,
    pub(crate) subscription_manager: Arc<SubscriptionManager>,
    listen_key: Arc<RwLock<Option<String>>>,
    listen_key_manager: Option<Arc<ListenKeyManager>>,
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
            .field("is_connected", &self.message_router.is_connected())
            .finish_non_exhaustive()
    }
}

impl BinanceWs {
    /// Creates a new Binance WebSocket client
    pub fn new(url: String) -> Self {
        let subscription_manager = Arc::new(SubscriptionManager::new());
        let message_router = Arc::new(MessageRouter::new(url, subscription_manager.clone(), None));

        // Start the router immediately
        let router_clone = message_router.clone();
        tokio::spawn(async move {
            if let Err(e) = router_clone.start(None).await {
                tracing::error!("Failed to start MessageRouter: {}", e);
            }
        });

        Self {
            message_router,
            subscription_manager,
            listen_key: Arc::new(RwLock::new(None)),
            listen_key_manager: None,
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
        let subscription_manager = Arc::new(SubscriptionManager::new());
        let listen_key_manager = Arc::new(ListenKeyManager::new(binance));
        let message_router = Arc::new(MessageRouter::new(
            url,
            subscription_manager.clone(),
            Some(listen_key_manager.clone()),
        ));

        // Start the router immediately
        let router_clone = message_router.clone();
        tokio::spawn(async move {
            if let Err(e) = router_clone.start(None).await {
                tracing::error!("Failed to start MessageRouter: {}", e);
            }
        });

        Self {
            message_router,
            subscription_manager,
            listen_key: Arc::new(RwLock::new(None)),
            listen_key_manager: Some(listen_key_manager),
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
        if self.is_connected() {
            return Ok(());
        }

        self.message_router.start(None).await?;

        // No auto-reconnect coordinator needed as MessageRouter handles it internally.
        // The router's loop manages connection state and retries.

        Ok(())
    }

    /// Disconnects from the WebSocket server
    pub async fn disconnect(&self) -> Result<()> {
        self.message_router.stop().await?;

        if let Some(manager) = &self.listen_key_manager {
            manager.stop_auto_refresh().await;
        }

        Ok(())
    }

    /// Connects to the user data stream
    pub async fn connect_user_stream(&self) -> Result<()> {
        let manager = self.listen_key_manager.as_ref()
            .ok_or_else(|| Error::invalid_request(
                "Listen key manager not available. Use new_with_auth() to create authenticated WebSocket"
            ))?;

        let listen_key = manager.get_or_create().await?;

        let base_url = self.message_router.get_url();
        let base_url = if let Some(stripped) = base_url.strip_suffix('/') {
            stripped
        } else {
            &base_url
        };

        let url = format!("{}/{}", base_url, listen_key);

        self.message_router.start(Some(url)).await?;
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
        let (tx, _rx) = tokio::sync::mpsc::channel(1024);

        self.subscription_manager
            .add_subscription(
                stream.clone(),
                symbol.to_string(),
                SubscriptionType::Ticker,
                tx,
            )
            .await?;

        self.message_router.subscribe(vec![stream]).await
    }

    /// Subscribes to the 24-hour ticker stream for all symbols
    pub async fn subscribe_all_tickers(&self) -> Result<()> {
        let stream = "!ticker@arr".to_string();
        let (tx, _rx) = tokio::sync::mpsc::channel(1024);

        self.subscription_manager
            .add_subscription(
                stream.clone(),
                "all".to_string(),
                SubscriptionType::Ticker,
                tx,
            )
            .await?;

        self.message_router.subscribe(vec![stream]).await
    }

    /// Subscribes to real-time trade executions for a symbol
    pub async fn subscribe_trades(&self, symbol: &str) -> Result<()> {
        let stream = format!("{}@trade", symbol.to_lowercase());
        let (tx, _rx) = tokio::sync::mpsc::channel(1024);

        self.subscription_manager
            .add_subscription(
                stream.clone(),
                symbol.to_string(),
                SubscriptionType::Trades,
                tx,
            )
            .await?;

        self.message_router.subscribe(vec![stream]).await
    }

    /// Subscribes to the aggregated trade stream for a symbol
    pub async fn subscribe_agg_trades(&self, symbol: &str) -> Result<()> {
        let stream = format!("{}@aggTrade", symbol.to_lowercase());
        let (tx, _rx) = tokio::sync::mpsc::channel(1024);

        self.subscription_manager
            .add_subscription(
                stream.clone(),
                symbol.to_string(),
                SubscriptionType::Trades,
                tx,
            )
            .await?;

        self.message_router.subscribe(vec![stream]).await
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
        let (tx, _rx) = tokio::sync::mpsc::channel(1024);

        self.subscription_manager
            .add_subscription(
                stream.clone(),
                symbol.to_string(),
                SubscriptionType::OrderBook,
                tx,
            )
            .await?;

        self.message_router.subscribe(vec![stream]).await
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
        let (tx, _rx) = tokio::sync::mpsc::channel(1024);

        self.subscription_manager
            .add_subscription(
                stream.clone(),
                symbol.to_string(),
                SubscriptionType::OrderBook,
                tx,
            )
            .await?;

        self.message_router.subscribe(vec![stream]).await
    }

    /// Subscribes to Kline (candlestick) data for a symbol
    pub async fn subscribe_kline(&self, symbol: &str, interval: &str) -> Result<()> {
        let stream = format!("{}@kline_{}", symbol.to_lowercase(), interval);
        let (tx, _rx) = tokio::sync::mpsc::channel(1024);

        self.subscription_manager
            .add_subscription(
                stream.clone(),
                symbol.to_string(),
                SubscriptionType::Kline(interval.to_string()),
                tx,
            )
            .await?;

        self.message_router.subscribe(vec![stream]).await
    }

    /// Subscribes to the mini ticker stream for a symbol
    pub async fn subscribe_mini_ticker(&self, symbol: &str) -> Result<()> {
        let stream = format!("{}@miniTicker", symbol.to_lowercase());
        let (tx, _rx) = tokio::sync::mpsc::channel(1024);

        self.subscription_manager
            .add_subscription(
                stream.clone(),
                symbol.to_string(),
                SubscriptionType::Ticker,
                tx,
            )
            .await?;

        self.message_router.subscribe(vec![stream]).await
    }

    /// Subscribes to the mini ticker stream for all symbols
    pub async fn subscribe_all_mini_tickers(&self) -> Result<()> {
        let stream = "!miniTicker@arr".to_string();
        let (tx, _rx) = tokio::sync::mpsc::channel(1024);

        self.subscription_manager
            .add_subscription(
                stream.clone(),
                "all".to_string(),
                SubscriptionType::Ticker,
                tx,
            )
            .await?;

        self.message_router.subscribe(vec![stream]).await
    }

    /// Cancels an existing subscription
    pub async fn unsubscribe(&self, stream: String) -> Result<()> {
        self.subscription_manager
            .remove_subscription(&stream)
            .await?;
        self.message_router.unsubscribe(vec![stream]).await
    }

    /// Receives the next available message
    pub fn receive(&self) -> Option<Value> {
        None
    }

    /// Indicates whether the WebSocket connection is active
    pub fn is_connected(&self) -> bool {
        self.message_router.is_connected()
    }

    /// Returns the current connection state.
    pub fn state(&self) -> ccxt_core::ws_client::WsConnectionState {
        if self.message_router.is_connected() {
            ccxt_core::ws_client::WsConnectionState::Connected
        } else {
            ccxt_core::ws_client::WsConnectionState::Disconnected
        }
    }

    /// Returns the list of active subscriptions.
    ///
    /// Returns a vector of subscription channel names that are currently active.
    /// This method retrieves the actual subscriptions from the underlying WsClient's
    /// subscription manager, providing accurate state tracking.
    pub fn subscriptions(&self) -> Vec<String> {
        let subs = self.subscription_manager.get_all_subscriptions_sync();
        subs.into_iter().map(|s| s.stream).collect()
    }

    /// Returns the number of active subscriptions.
    pub fn subscription_count(&self) -> usize {
        self.subscription_manager.active_count()
    }

    /// Watches a single mark price stream (internal helper)
    async fn watch_mark_price_internal(
        &self,
        symbol: &str,
        channel_name: &str,
    ) -> Result<MarkPrice> {
        let stream = format!("{}@{}", symbol.to_lowercase(), channel_name);
        tracing::debug!(
            "watch_mark_price_internal: stream={}, symbol={}",
            stream,
            symbol
        );

        let (tx, mut rx) = tokio::sync::mpsc::channel(1024);
        self.subscription_manager
            .add_subscription(
                stream.clone(),
                symbol.to_string(),
                SubscriptionType::MarkPrice,
                tx,
            )
            .await?;

        self.message_router.subscribe(vec![stream.clone()]).await?;

        loop {
            if let Some(message) = rx.recv().await {
                if message.get("result").is_some() {
                    tracing::debug!("Received subscription result for {}", stream);
                    continue;
                }

                tracing::debug!("Received mark price message for {}", stream);

                match parser::parse_ws_mark_price(&message) {
                    Ok(mark_price) => {
                        let mut mark_prices = self.mark_prices.lock().await;
                        mark_prices.insert(mark_price.symbol.clone(), mark_price.clone());
                        return Ok(mark_price);
                    }
                    Err(e) => {
                        tracing::warn!(
                            "Failed to parse mark price message for stream {}: {:?}. Payload: {:?}",
                            stream,
                            e,
                            message
                        );
                    }
                }
            } else {
                tracing::warn!("Subscription channel closed for {}", stream);
                return Err(Error::network("Subscription channel closed"));
            }
        }
    }

    /// Watches multiple mark price streams (internal helper)
    async fn watch_mark_prices_internal(
        &self,
        symbols: Option<Vec<String>>,
        channel_name: &str,
    ) -> Result<HashMap<String, MarkPrice>> {
        let (tx, mut rx) = tokio::sync::mpsc::channel(1024);

        let streams: Vec<String> = if let Some(syms) = symbols.as_ref() {
            let mut streams = Vec::with_capacity(syms.len());
            for sym in syms {
                let symbol = sym.to_lowercase();
                let stream = format!("{}@{}", symbol, channel_name);
                self.subscription_manager
                    .add_subscription(
                        stream.clone(),
                        symbol,
                        SubscriptionType::MarkPrice,
                        tx.clone(),
                    )
                    .await?;
                streams.push(stream);
            }
            streams
        } else {
            let stream = format!("!{}@arr", channel_name);
            self.subscription_manager
                .add_subscription(
                    stream.clone(),
                    "all".to_string(),
                    SubscriptionType::MarkPrice,
                    tx.clone(),
                )
                .await?;
            vec![stream]
        };

        self.message_router.subscribe(streams.clone()).await?;

        let mut result = HashMap::new();

        loop {
            if let Some(message) = rx.recv().await {
                if message.get("result").is_some() {
                    continue;
                }

                if let Some(arr) = message.as_array() {
                    for item in arr {
                        if let Ok(mark_price) = parser::parse_ws_mark_price(item) {
                            let symbol = mark_price.symbol.clone();

                            if let Some(syms) = &symbols {
                                if syms.contains(&symbol.to_lowercase()) {
                                    result.insert(symbol.clone(), mark_price.clone());
                                }
                            } else {
                                result.insert(symbol.clone(), mark_price.clone());
                            }

                            let mut mark_prices = self.mark_prices.lock().await;
                            mark_prices.insert(symbol, mark_price);
                        } else {
                            tracing::warn!("Failed to parse item in mark price array: {:?}", item);
                        }
                    }

                    if let Some(syms) = &symbols {
                        if result.len() >= syms.len() {
                            return Ok(result);
                        }
                    } else {
                        // For array updates without specific symbols filter, return what we got
                        return Ok(result);
                    }
                } else {
                    match parser::parse_ws_mark_price(&message) {
                        Ok(mark_price) => {
                            let symbol = mark_price.symbol.clone();
                            result.insert(symbol.clone(), mark_price.clone());

                            let mut mark_prices = self.mark_prices.lock().await;
                            mark_prices.insert(symbol, mark_price);

                            if let Some(syms) = &symbols {
                                if result.len() >= syms.len() {
                                    return Ok(result);
                                }
                            }
                        }
                        Err(e) => {
                            tracing::warn!(
                                "Failed to parse mark price message: {:?}. Payload: {:?}",
                                e,
                                message
                            );
                        }
                    }
                }
            } else {
                return Err(Error::network("Subscription channel closed"));
            }
        }
    }

    /// Watches a single ticker stream (internal helper)
    async fn watch_ticker_internal(&self, symbol: &str, channel_name: &str) -> Result<Ticker> {
        let stream = format!("{}@{}", symbol.to_lowercase(), channel_name);

        let (tx, mut rx) = tokio::sync::mpsc::channel(1024);
        self.subscription_manager
            .add_subscription(
                stream.clone(),
                symbol.to_string(),
                SubscriptionType::Ticker,
                tx,
            )
            .await?;

        self.message_router.subscribe(vec![stream.clone()]).await?;

        loop {
            if let Some(message) = rx.recv().await {
                if message.get("result").is_some() {
                    continue;
                }

                match parser::parse_ws_ticker(&message, None) {
                    Ok(ticker) => {
                        let mut tickers = self.tickers.lock().await;
                        tickers.insert(ticker.symbol.clone(), ticker.clone());
                        return Ok(ticker);
                    }
                    Err(e) => {
                        tracing::warn!(
                            "Failed to parse ticker message for stream {}: {:?}. Payload: {:?}",
                            stream,
                            e,
                            message
                        );
                        // Continue waiting for the next message instead of hanging silently
                    }
                }
            } else {
                return Err(Error::network("Subscription channel closed"));
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

        let (tx, mut rx) = tokio::sync::mpsc::channel(1024);

        for stream in &streams {
            self.subscription_manager
                .add_subscription(
                    stream.clone(),
                    "all".to_string(),
                    SubscriptionType::Ticker,
                    tx.clone(),
                )
                .await?;
        }

        self.message_router.subscribe(streams.clone()).await?;

        let mut result = HashMap::new();

        loop {
            if let Some(message) = rx.recv().await {
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
                        } else {
                            tracing::warn!("Failed to parse item in ticker array: {:?}", item);
                        }
                    }

                    if let Some(syms) = &symbols {
                        if result.len() >= syms.len() {
                            return Ok(result);
                        }
                    } else {
                        // If we received an array but we are not waiting for specific symbols,
                        // we can assume we got the update for "all" markets.
                        // However, !ticker@arr returns ALL tickers in one message usually.
                        return Ok(result);
                    }
                } else {
                    match parser::parse_ws_ticker(&message, None) {
                        Ok(ticker) => {
                            let symbol = ticker.symbol.clone();
                            result.insert(symbol.clone(), ticker.clone());

                            let mut tickers = self.tickers.lock().await;
                            tickers.insert(symbol, ticker);

                            if let Some(syms) = &symbols {
                                if result.len() >= syms.len() {
                                    return Ok(result);
                                }
                            }
                        }
                        Err(e) => {
                            tracing::warn!(
                                "Failed to parse ticker message: {:?}. Payload: {:?}",
                                e,
                                message
                            );
                        }
                    }
                }
            } else {
                return Err(Error::network("Subscription channel closed"));
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

        let (tx, mut rx) = tokio::sync::mpsc::channel(1024);
        self.subscription_manager
            .add_subscription(
                stream.clone(),
                symbol.to_string(),
                SubscriptionType::OrderBook,
                tx,
            )
            .await?;

        self.message_router.subscribe(vec![stream.clone()]).await?;

        tokio::time::sleep(Duration::from_millis(500)).await;

        let _snapshot = self
            .fetch_orderbook_snapshot(exchange, symbol, limit, is_futures)
            .await?;

        loop {
            if let Some(message) = rx.recv().await {
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
            } else {
                return Err(Error::network("Subscription channel closed"));
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

        let (tx, mut rx) = tokio::sync::mpsc::channel(1024);
        let mut streams = Vec::new();

        for symbol in &symbols {
            let stream = if update_speed == 100 {
                format!("{}@depth@100ms", symbol.to_lowercase())
            } else {
                format!("{}@depth", symbol.to_lowercase())
            };

            streams.push(stream.clone());

            self.subscription_manager
                .add_subscription(
                    stream,
                    symbol.clone(),
                    SubscriptionType::OrderBook,
                    tx.clone(),
                )
                .await?;
        }

        self.message_router.subscribe(streams).await?;

        tokio::time::sleep(Duration::from_millis(500)).await;

        for symbol in &symbols {
            let _ = self
                .fetch_orderbook_snapshot(exchange, symbol, limit, is_futures)
                .await;
        }

        let mut result = HashMap::new();
        let mut synced_symbols = std::collections::HashSet::new();

        while synced_symbols.len() < symbols.len() {
            if let Some(message) = rx.recv().await {
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

                            let orderbooks = self.orderbooks.lock().await;
                            if let Some(ob) = orderbooks.get(msg_symbol) {
                                if ob.is_synced {
                                    synced_symbols.insert(msg_symbol.to_string());
                                }
                            }
                        }
                    }
                }
            } else {
                return Err(Error::network("Subscription channel closed"));
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

    /// Watches the best bid/ask data for a unified symbol (internal helper)
    async fn watch_bids_asks_internal(&self, symbol: &str, market_id: &str) -> Result<BidAsk> {
        let stream = format!("{}@bookTicker", market_id.to_lowercase());
        let (tx, mut rx) = tokio::sync::mpsc::channel(1024);

        self.subscription_manager
            .add_subscription(
                stream.clone(),
                symbol.to_string(),
                SubscriptionType::BookTicker,
                tx,
            )
            .await?;

        self.message_router.subscribe(vec![stream.clone()]).await?;

        loop {
            if let Some(message) = rx.recv().await {
                if message.get("result").is_some() {
                    continue;
                }

                if let Ok(bid_ask) = parser::parse_ws_bid_ask(&message) {
                    let mut bids_asks_map = self.bids_asks.lock().await;
                    bids_asks_map.insert(symbol.to_string(), bid_ask.clone());
                    return Ok(bid_ask);
                }
            } else {
                return Err(Error::network("Subscription channel closed"));
            }
        }
    }

    /// Streams trade data for a unified symbol (internal helper)
    async fn watch_trades_internal(
        &self,
        symbol: &str,
        market_id: &str,
        since: Option<i64>,
        limit: Option<usize>,
        market: Option<&ccxt_core::types::Market>,
    ) -> Result<Vec<Trade>> {
        let stream = format!("{}@trade", market_id.to_lowercase());
        let (tx, mut rx) = tokio::sync::mpsc::channel(1024);

        self.subscription_manager
            .add_subscription(
                stream.clone(),
                symbol.to_string(),
                SubscriptionType::Trades,
                tx,
            )
            .await?;

        self.message_router.subscribe(vec![stream.clone()]).await?;

        // Wait for at least one trade or use a loop with timeout if we want to mimic "polling" until data arrives?
        // Usually watch_trades returns the latest trades.

        loop {
            if let Some(message) = rx.recv().await {
                if message.get("result").is_some() {
                    continue;
                }

                if let Ok(trade) = parser::parse_ws_trade(&message, market) {
                    let mut trades_map = self.trades.lock().await;
                    let trades = trades_map
                        .entry(symbol.to_string())
                        .or_insert_with(VecDeque::new);

                    if trades.len() >= MAX_TRADES {
                        trades.pop_front();
                    }
                    trades.push_back(trade);

                    let mut result: Vec<Trade> = trades.iter().cloned().collect();

                    if let Some(since_ts) = since {
                        result.retain(|t| t.timestamp >= since_ts);
                    }

                    if let Some(limit_size) = limit {
                        if result.len() > limit_size {
                            result = result.split_off(result.len() - limit_size);
                        }
                    }

                    return Ok(result);
                }
            } else {
                return Err(Error::network("Subscription channel closed"));
            }
        }
    }

    /// Streams OHLCV data for a unified symbol (internal helper)
    async fn watch_ohlcv_internal(
        &self,
        symbol: &str,
        market_id: &str,
        timeframe: &str,
        since: Option<i64>,
        limit: Option<usize>,
    ) -> Result<Vec<OHLCV>> {
        let stream = format!("{}@kline_{}", market_id.to_lowercase(), timeframe);
        let (tx, mut rx) = tokio::sync::mpsc::channel(1024);

        self.subscription_manager
            .add_subscription(
                stream.clone(),
                symbol.to_string(),
                SubscriptionType::Kline(timeframe.to_string()),
                tx,
            )
            .await?;

        self.message_router.subscribe(vec![stream.clone()]).await?;

        loop {
            if let Some(message) = rx.recv().await {
                if message.get("result").is_some() {
                    continue;
                }

                if let Ok(ohlcv) = parser::parse_ws_ohlcv(&message) {
                    let cache_key = format!("{}:{}", symbol, timeframe);
                    let mut ohlcvs_map = self.ohlcvs.lock().await;
                    let ohlcvs = ohlcvs_map.entry(cache_key).or_insert_with(VecDeque::new);

                    if ohlcvs.len() >= MAX_OHLCVS {
                        ohlcvs.pop_front();
                    }
                    ohlcvs.push_back(ohlcv);

                    let mut result: Vec<OHLCV> = ohlcvs.iter().cloned().collect();

                    if let Some(since_ts) = since {
                        result.retain(|o| o.timestamp >= since_ts);
                    }

                    if let Some(limit_size) = limit {
                        if result.len() > limit_size {
                            result = result.split_off(result.len() - limit_size);
                        }
                    }

                    return Ok(result);
                }
            } else {
                return Err(Error::network("Subscription channel closed"));
            }
        }
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

    /// Watches for balance updates (internal helper)
    async fn watch_balance_internal(&self, account_type: &str) -> Result<Balance> {
        self.connect_user_stream().await?;

        let (tx, mut rx) = tokio::sync::mpsc::channel(1024);

        self.subscription_manager
            .add_subscription(
                "!userData".to_string(),
                "user".to_string(),
                SubscriptionType::Balance,
                tx,
            )
            .await?;

        loop {
            if let Some(message) = rx.recv().await {
                if let Some(event_type) = message.get("e").and_then(|e| e.as_str()) {
                    if matches!(
                        event_type,
                        "balanceUpdate" | "outboundAccountPosition" | "ACCOUNT_UPDATE"
                    ) {
                        if let Ok(()) = self.handle_balance_message(&message, account_type).await {
                            let balances = self.balances.read().await;
                            if let Some(balance) = balances.get(account_type) {
                                return Ok(balance.clone());
                            }
                        }
                    }
                }
            } else {
                return Err(Error::network("Subscription channel closed"));
            }
        }
    }

    /// Watches for order updates (internal helper)
    async fn watch_orders_internal(
        &self,
        symbol: Option<&str>,
        since: Option<i64>,
        limit: Option<usize>,
    ) -> Result<Vec<Order>> {
        self.connect_user_stream().await?;

        let (tx, mut rx) = tokio::sync::mpsc::channel(1024);

        self.subscription_manager
            .add_subscription(
                "!userData".to_string(),
                "user".to_string(),
                SubscriptionType::Orders,
                tx,
            )
            .await?;

        loop {
            if let Some(message) = rx.recv().await {
                if let Value::Object(data) = message {
                    if let Some(event_type) = data.get("e").and_then(serde_json::Value::as_str) {
                        if event_type == "executionReport" {
                            let order = user_data::parse_ws_order(&data);

                            let mut orders = self.orders.write().await;
                            let symbol_orders = orders
                                .entry(order.symbol.clone())
                                .or_insert_with(HashMap::new);
                            symbol_orders.insert(order.id.clone(), order.clone());
                            drop(orders);

                            if let Some(exec_type) =
                                data.get("x").and_then(serde_json::Value::as_str)
                            {
                                if exec_type == "TRADE" {
                                    if let Ok(trade) =
                                        BinanceWs::parse_ws_trade(&Value::Object(data.clone()))
                                    {
                                        let mut trades = self.my_trades.write().await;
                                        let symbol_trades = trades
                                            .entry(trade.symbol.clone())
                                            .or_insert_with(VecDeque::new);

                                        symbol_trades.push_front(trade);
                                        if symbol_trades.len() > 1000 {
                                            symbol_trades.pop_back();
                                        }
                                    }
                                }
                            }

                            return self.filter_orders(symbol, since, limit).await;
                        }
                    }
                }
            } else {
                return Err(Error::network("Subscription channel closed"));
            }
        }
    }

    /// Watches for my trades (internal helper)
    async fn watch_my_trades_internal(
        &self,
        symbol: Option<&str>,
        since: Option<i64>,
        limit: Option<usize>,
    ) -> Result<Vec<Trade>> {
        self.connect_user_stream().await?;

        let (tx, mut rx) = tokio::sync::mpsc::channel(1024);

        self.subscription_manager
            .add_subscription(
                "!userData".to_string(),
                "user".to_string(),
                SubscriptionType::MyTrades,
                tx,
            )
            .await?;

        loop {
            if let Some(msg) = rx.recv().await {
                if let Some(event_type) = msg.get("e").and_then(|e| e.as_str()) {
                    if event_type == "executionReport" {
                        if let Ok(trade) = BinanceWs::parse_ws_trade(&msg) {
                            let symbol_key = trade.symbol.clone();

                            let mut trades_map = self.my_trades.write().await;
                            let symbol_trades =
                                trades_map.entry(symbol_key).or_insert_with(VecDeque::new);

                            symbol_trades.push_front(trade);
                            if symbol_trades.len() > 1000 {
                                symbol_trades.pop_back();
                            }

                            drop(trades_map);
                            return self.filter_my_trades(symbol, since, limit).await;
                        }
                    }
                }
            } else {
                return Err(Error::network("Subscription channel closed"));
            }
        }
    }

    /// Watches for positions (internal helper)
    async fn watch_positions_internal(
        &self,
        symbols: Option<Vec<String>>,
        since: Option<i64>,
        limit: Option<usize>,
    ) -> Result<Vec<Position>> {
        self.connect_user_stream().await?;

        let (tx, mut rx) = tokio::sync::mpsc::channel(1024);

        self.subscription_manager
            .add_subscription(
                "!userData".to_string(),
                "user".to_string(),
                SubscriptionType::Positions,
                tx,
            )
            .await?;

        loop {
            if let Some(msg) = rx.recv().await {
                if let Some(event_type) = msg.get("e").and_then(|e| e.as_str()) {
                    if event_type == "ACCOUNT_UPDATE" {
                        if let Some(account_data) = msg.get("a") {
                            if let Some(positions_array) =
                                account_data.get("P").and_then(|p| p.as_array())
                            {
                                for position_data in positions_array {
                                    if let Ok(position) =
                                        BinanceWs::parse_ws_position(position_data)
                                    {
                                        let symbol_key = position.symbol.clone();
                                        let side_key = position
                                            .side
                                            .clone()
                                            .unwrap_or_else(|| "both".to_string());

                                        let mut positions_map = self.positions.write().await;
                                        let symbol_positions = positions_map
                                            .entry(symbol_key)
                                            .or_insert_with(HashMap::new);

                                        if position.contracts.unwrap_or(0.0).abs() < 0.000001 {
                                            symbol_positions.remove(&side_key);
                                            if symbol_positions.is_empty() {
                                                positions_map.remove(&position.symbol);
                                            }
                                        } else {
                                            symbol_positions.insert(side_key, position);
                                        }
                                    }
                                }

                                let symbols_ref = symbols.as_deref();
                                return self.filter_positions(symbols_ref, since, limit).await;
                            }
                        }
                    }
                }
            } else {
                return Err(Error::network("Subscription channel closed"));
            }
        }
    }

    /// Filters cached orders by symbol, time range, and limit
    async fn filter_orders(
        &self,
        symbol: Option<&str>,
        since: Option<i64>,
        limit: Option<usize>,
    ) -> Result<Vec<Order>> {
        let orders_map = self.orders.read().await;

        let mut orders: Vec<Order> = if let Some(sym) = symbol {
            orders_map
                .get(sym)
                .map(|symbol_orders| symbol_orders.values().cloned().collect())
                .unwrap_or_default()
        } else {
            orders_map
                .values()
                .flat_map(|symbol_orders| symbol_orders.values().cloned())
                .collect()
        };

        if let Some(since_ts) = since {
            orders.retain(|order| order.timestamp.is_some_and(|ts| ts >= since_ts));
        }

        orders.sort_by(|a, b| {
            let ts_a = a.timestamp.unwrap_or(0);
            let ts_b = b.timestamp.unwrap_or(0);
            ts_b.cmp(&ts_a)
        });

        if let Some(lim) = limit {
            orders.truncate(lim);
        }

        Ok(orders)
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
#[allow(clippy::disallowed_methods)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_binance_ws_creation() {
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
        let (tx, _rx) = tokio::sync::mpsc::channel(1024);

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
