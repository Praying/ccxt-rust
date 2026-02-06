//! Binance WebSocket implementation
//!
//! Provides WebSocket real-time data stream subscriptions for the Binance exchange.
//!
//! # Architecture
//!
//! The WebSocket implementation is organized into several modules:
//!
//! - [`types`] - Configuration structures and enums (`BinanceWsConfig`, `DepthLevel`, etc.)
//! - [`parsers`] - Stream message parsers implementing [`StreamParser`] trait
//! - [`connection_manager`] - Connection sharding and management
//! - `handlers` - Message routing and dispatch
//! - `subscriptions` - Subscription tracking and management
//!
//! # Usage
//!
//! ```rust,ignore
//! use ccxt_exchanges::binance::ws::{BinanceWs, BinanceWsConfig, DepthLevel, UpdateSpeed};
//!
//! // Create a WebSocket client
//! let ws = BinanceWs::new("wss://stream.binance.com:9443/ws".to_string());
//!
//! // Subscribe to ticker updates
//! let ticker = ws.watch_ticker("BTC/USDT", MarketType::Spot).await?;
//!
//! // Check connection health
//! let health = ws.health();
//! println!("Latency: {:?}ms", health.latency_ms);
//! ```
//!
//! # Health Monitoring
//!
//! Use the [`BinanceWs::health`] method to get connection metrics:
//! - Latency (from ping/pong)
//! - Message counts (received/dropped)
//! - Connection uptime
//! - Reconnection count
//!
//! # Error Handling
//!
//! Errors are classified into recovery strategies via [`WsErrorRecovery`]:
//! - `Retry` - Transient errors, retry with backoff
//! - `Resync` - State out of sync, need to refresh (e.g., orderbook)
//! - `Reconnect` - Connection lost, need to reconnect
//! - `Fatal` - Unrecoverable, requires user intervention

/// Data cache for WebSocket streams
pub mod cache;
/// Connection manager module
pub mod connection_manager;
mod handlers;
/// Health tracking for WebSocket connections
pub mod health;
mod listen_key;
/// Stream parsers for WebSocket messages
pub mod parsers;
/// Shutdown state management
pub mod shutdown;
mod streams;
mod subscriptions;
/// Types and configuration for WebSocket
pub mod types;
pub(crate) mod user_data;

// Re-export public types for backward compatibility
pub use cache::WsDataCache;
pub use connection_manager::BinanceConnectionManager;
pub use handlers::MessageRouter;
pub use health::WsHealthTracker;
pub use listen_key::ListenKeyManager;
pub use parsers::{
    BidAskParser, MarkPriceParser, OhlcvParser, StreamParser, TickerParser, TradeParser,
};
pub use shutdown::ShutdownState;
pub use streams::normalize_symbol;
pub use subscriptions::{ReconnectConfig, Subscription, SubscriptionManager, SubscriptionType};
pub use types::{
    BinanceWsConfig, DepthLevel, UpdateSpeed, WsChannelConfig, WsErrorRecovery, WsHealthSnapshot,
};

use crate::binance::{Binance, parser};
use ccxt_core::error::{Error, Result};
use ccxt_core::types::{
    Balance, BidAsk, MarkPrice, MarketType, OHLCV, Order, OrderBook, Position, Ticker, Trade,
};
use serde_json::Value;
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::Duration;
use tokio::sync::RwLock;

const MAX_TRADES: usize = 1000;
const MAX_OHLCVS: usize = 1000;
/// Maximum consecutive non-fatal errors before returning fatal in orderbook delta processing
const MAX_CONSECUTIVE_DELTA_ERRORS: u32 = 5;
/// Default shutdown timeout in milliseconds
const DEFAULT_SHUTDOWN_TIMEOUT_MS: u64 = 5000;

/// Binance WebSocket client wrapper
pub struct BinanceWs {
    pub(crate) message_router: Arc<MessageRouter>,
    pub(crate) subscription_manager: Arc<SubscriptionManager>,
    listen_key: Arc<RwLock<Option<String>>>,
    listen_key_manager: Option<Arc<ListenKeyManager>>,
    /// Consolidated data cache
    pub(crate) cache: Arc<WsDataCache>,
    /// Health monitoring metrics
    pub(crate) health: Arc<WsHealthTracker>,
    /// Shutdown lifecycle state
    pub(crate) shutdown: Arc<ShutdownState>,
    /// Channel capacity configuration
    channel_config: WsChannelConfig,
    /// Timeout for watch_* methods
    watch_timeout: Option<Duration>,
}

impl std::fmt::Debug for BinanceWs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BinanceWs")
            .field("is_connected", &self.message_router.is_connected())
            .finish_non_exhaustive()
    }
}

impl Drop for BinanceWs {
    fn drop(&mut self) {
        if self.shutdown.needs_shutdown_warning() {
            tracing::warn!(
                "BinanceWs dropped without calling shutdown(). \
                 This may leave resources uncleaned. \
                 Consider calling shutdown() before dropping."
            );
        }
    }
}

impl BinanceWs {
    /// Shared initialization logic for all constructors.
    fn new_inner(
        message_router: Arc<MessageRouter>,
        subscription_manager: Arc<SubscriptionManager>,
        listen_key_manager: Option<Arc<ListenKeyManager>>,
        channel_config: WsChannelConfig,
        watch_timeout: Option<Duration>,
    ) -> Self {
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
            listen_key_manager,
            cache: Arc::new(WsDataCache::new()),
            health: Arc::new(WsHealthTracker::new()),
            shutdown: Arc::new(ShutdownState::new()),
            channel_config,
            watch_timeout,
        }
    }

    /// Creates a new Binance WebSocket client
    pub fn new(url: String) -> Self {
        let subscription_manager = Arc::new(SubscriptionManager::new());
        let message_router = Arc::new(MessageRouter::new(url, subscription_manager.clone(), None));

        Self::new_inner(
            message_router,
            subscription_manager,
            None,
            WsChannelConfig::default(),
            None,
        )
    }

    /// Creates a new Binance WebSocket client with custom configuration.
    ///
    /// # Arguments
    /// * `config` - Configuration options including URL, channel capacities, and backpressure strategy
    ///
    /// # Example
    /// ```rust,ignore
    /// use ccxt_exchanges::binance::ws::{BinanceWs, BinanceWsConfig, WsChannelConfig};
    /// use ccxt_core::ws_client::BackpressureStrategy;
    ///
    /// let config = BinanceWsConfig::new("wss://stream.binance.com:9443/ws".to_string())
    ///     .with_backpressure(BackpressureStrategy::DropOldest);
    /// let ws = BinanceWs::new_with_config(config);
    /// ```
    pub fn new_with_config(config: BinanceWsConfig) -> Self {
        let subscription_manager = Arc::new(SubscriptionManager::new());
        let message_router = Arc::new(MessageRouter::new(
            config.url,
            subscription_manager.clone(),
            None,
        ));

        Self::new_inner(
            message_router,
            subscription_manager,
            None,
            config.channel_config,
            config.watch_timeout,
        )
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

        Self::new_inner(
            message_router,
            subscription_manager,
            Some(listen_key_manager),
            WsChannelConfig::default(),
            None,
        )
    }

    /// Returns the channel capacity for a given subscription type
    fn channel_capacity_for(&self, sub_type: &SubscriptionType) -> usize {
        match sub_type {
            SubscriptionType::Ticker | SubscriptionType::BookTicker => {
                self.channel_config.ticker_capacity
            }
            SubscriptionType::OrderBook => self.channel_config.orderbook_capacity,
            SubscriptionType::Trades | SubscriptionType::Kline(_) | SubscriptionType::MarkPrice => {
                self.channel_config.trades_capacity
            }
            SubscriptionType::Balance
            | SubscriptionType::Orders
            | SubscriptionType::MyTrades
            | SubscriptionType::Positions => self.channel_config.user_data_capacity,
        }
    }

    /// Returns the configured watch timeout or the default (30 seconds)
    fn watch_timeout(&self) -> Duration {
        self.watch_timeout
            .unwrap_or(Duration::from_secs(types::DEFAULT_WATCH_TIMEOUT_SECS))
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

    /// Gracefully shuts down the WebSocket client.
    ///
    /// This method performs a complete shutdown sequence:
    /// 1. Sets the shutting down flag to reject new subscriptions
    /// 2. Stops the message router
    /// 3. Stops listen key auto-refresh
    /// 4. Clears all subscriptions
    /// 5. Clears cached data
    ///
    /// After calling this method, the client cannot be reused.
    /// This method is idempotent - calling it multiple times is safe.
    pub async fn shutdown(&self) -> Result<()> {
        // Check if already shutting down or complete
        if self.shutdown.shutdown_complete.load(Ordering::Acquire) {
            return Ok(());
        }

        // Set shutting down flag to reject new subscriptions
        if self.shutdown.is_shutting_down.swap(true, Ordering::AcqRel) {
            // Another shutdown is in progress, wait for it
            while !self.shutdown.shutdown_complete.load(Ordering::Acquire) {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
            return Ok(());
        }

        tracing::info!("Initiating graceful shutdown of BinanceWs");

        // Stop the message router (sends close frame)
        let shutdown_result = tokio::time::timeout(
            Duration::from_millis(DEFAULT_SHUTDOWN_TIMEOUT_MS),
            self.message_router.stop(),
        )
        .await;

        if shutdown_result.is_err() {
            tracing::warn!("Shutdown timeout exceeded, forcing close");
        }

        // Stop listen key auto-refresh
        if let Some(manager) = &self.listen_key_manager {
            manager.stop_auto_refresh().await;
        }

        // Clear all subscriptions
        self.subscription_manager.clear().await;

        // Clear cached data
        self.cache.tickers.lock().await.clear();
        self.cache.bids_asks.lock().await.clear();
        self.cache.mark_prices.lock().await.clear();
        self.cache.orderbooks.lock().await.clear();
        self.cache.trades.lock().await.clear();
        self.cache.ohlcvs.lock().await.clear();
        self.cache.balances.write().await.clear();
        self.cache.orders.write().await.clear();
        self.cache.my_trades.write().await.clear();
        self.cache.positions.write().await.clear();

        // Mark shutdown as complete
        self.shutdown
            .shutdown_complete
            .store(true, Ordering::Release);

        tracing::info!("BinanceWs shutdown complete");
        Ok(())
    }

    /// Returns true if the client is shutting down or has shut down.
    #[inline]
    pub fn is_shutting_down(&self) -> bool {
        self.shutdown.is_shutting_down.load(Ordering::Acquire)
    }

    /// Checks if shutdown is allowed and returns an error if shutting down.
    #[inline]
    #[allow(dead_code)]
    fn check_not_shutting_down(&self) -> Result<()> {
        if self.shutdown.is_shutting_down.load(Ordering::Acquire) {
            return Err(Error::invalid_request("WebSocket client is shutting down"));
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

    /// Subscribes to the ticker stream for a symbol.
    ///
    /// Returns a receiver that will receive ticker updates as JSON values.
    /// The caller is responsible for consuming messages from the receiver.
    pub async fn subscribe_ticker(
        &self,
        symbol: &str,
    ) -> Result<tokio::sync::mpsc::Receiver<Value>> {
        let normalized = normalize_symbol(symbol);
        let stream = format!("{}@ticker", normalized);
        let capacity = self.channel_capacity_for(&SubscriptionType::Ticker);
        let (tx, rx) = tokio::sync::mpsc::channel(capacity);

        self.subscription_manager
            .add_subscription(
                stream.clone(),
                symbol.to_string(),
                SubscriptionType::Ticker,
                tx,
            )
            .await?;

        self.message_router.subscribe(vec![stream]).await?;
        Ok(rx)
    }

    /// Subscribes to the 24-hour ticker stream for all symbols.
    ///
    /// Returns a receiver that will receive ticker updates as JSON values.
    pub async fn subscribe_all_tickers(&self) -> Result<tokio::sync::mpsc::Receiver<Value>> {
        let stream = "!ticker@arr".to_string();
        let capacity = self.channel_capacity_for(&SubscriptionType::Ticker);
        let (tx, rx) = tokio::sync::mpsc::channel(capacity);

        self.subscription_manager
            .add_subscription(
                stream.clone(),
                "all".to_string(),
                SubscriptionType::Ticker,
                tx,
            )
            .await?;

        self.message_router.subscribe(vec![stream]).await?;
        Ok(rx)
    }

    /// Subscribes to real-time trade executions for a symbol.
    ///
    /// Returns a receiver that will receive trade updates as JSON values.
    pub async fn subscribe_trades(
        &self,
        symbol: &str,
    ) -> Result<tokio::sync::mpsc::Receiver<Value>> {
        let normalized = normalize_symbol(symbol);
        let stream = format!("{}@trade", normalized);
        let capacity = self.channel_capacity_for(&SubscriptionType::Trades);
        let (tx, rx) = tokio::sync::mpsc::channel(capacity);

        self.subscription_manager
            .add_subscription(
                stream.clone(),
                symbol.to_string(),
                SubscriptionType::Trades,
                tx,
            )
            .await?;

        self.message_router.subscribe(vec![stream]).await?;
        Ok(rx)
    }

    /// Subscribes to the aggregated trade stream for a symbol.
    ///
    /// Returns a receiver that will receive aggregated trade updates as JSON values.
    pub async fn subscribe_agg_trades(
        &self,
        symbol: &str,
    ) -> Result<tokio::sync::mpsc::Receiver<Value>> {
        let normalized = normalize_symbol(symbol);
        let stream = format!("{}@aggTrade", normalized);
        let capacity = self.channel_capacity_for(&SubscriptionType::Trades);
        let (tx, rx) = tokio::sync::mpsc::channel(capacity);

        self.subscription_manager
            .add_subscription(
                stream.clone(),
                symbol.to_string(),
                SubscriptionType::Trades,
                tx,
            )
            .await?;

        self.message_router.subscribe(vec![stream]).await?;
        Ok(rx)
    }

    /// Subscribes to the order book depth stream.
    ///
    /// Returns a receiver that will receive order book updates as JSON values.
    ///
    /// # Arguments
    /// * `symbol` - Trading pair symbol (e.g., "BTCUSDT" or "BTC/USDT")
    /// * `levels` - Depth level (L5, L10, or L20)
    /// * `update_speed` - Update frequency (Ms100 or Ms1000)
    ///
    /// # Example
    /// ```rust,ignore
    /// use ccxt_exchanges::binance::ws::{DepthLevel, UpdateSpeed};
    /// let mut rx = ws.subscribe_orderbook("BTCUSDT", DepthLevel::L20, UpdateSpeed::Ms100).await?;
    /// while let Some(msg) = rx.recv().await {
    ///     println!("Order book update: {:?}", msg);
    /// }
    /// ```
    pub async fn subscribe_orderbook(
        &self,
        symbol: &str,
        levels: DepthLevel,
        update_speed: UpdateSpeed,
    ) -> Result<tokio::sync::mpsc::Receiver<Value>> {
        let normalized = normalize_symbol(symbol);
        let stream = match update_speed {
            UpdateSpeed::Ms100 => format!("{}@depth{}@100ms", normalized, levels.as_u32()),
            UpdateSpeed::Ms1000 => format!("{}@depth{}", normalized, levels.as_u32()),
        };
        let capacity = self.channel_capacity_for(&SubscriptionType::OrderBook);
        let (tx, rx) = tokio::sync::mpsc::channel(capacity);

        self.subscription_manager
            .add_subscription(
                stream.clone(),
                symbol.to_string(),
                SubscriptionType::OrderBook,
                tx,
            )
            .await?;

        self.message_router.subscribe(vec![stream]).await?;
        Ok(rx)
    }

    /// Subscribes to the diff order book stream.
    ///
    /// Returns a receiver that will receive order book diff updates as JSON values.
    ///
    /// # Arguments
    /// * `symbol` - Trading pair symbol
    /// * `update_speed` - Optional update frequency
    pub async fn subscribe_orderbook_diff(
        &self,
        symbol: &str,
        update_speed: Option<UpdateSpeed>,
    ) -> Result<tokio::sync::mpsc::Receiver<Value>> {
        let normalized = normalize_symbol(symbol);
        let stream = match update_speed {
            Some(UpdateSpeed::Ms100) => format!("{}@depth@100ms", normalized),
            _ => format!("{}@depth", normalized),
        };
        let capacity = self.channel_capacity_for(&SubscriptionType::OrderBook);
        let (tx, rx) = tokio::sync::mpsc::channel(capacity);

        self.subscription_manager
            .add_subscription(
                stream.clone(),
                symbol.to_string(),
                SubscriptionType::OrderBook,
                tx,
            )
            .await?;

        self.message_router.subscribe(vec![stream]).await?;
        Ok(rx)
    }

    /// Subscribes to Kline (candlestick) data for a symbol.
    ///
    /// Returns a receiver that will receive kline updates as JSON values.
    pub async fn subscribe_kline(
        &self,
        symbol: &str,
        interval: &str,
    ) -> Result<tokio::sync::mpsc::Receiver<Value>> {
        let normalized = normalize_symbol(symbol);
        let stream = format!("{}@kline_{}", normalized, interval);
        let sub_type = SubscriptionType::Kline(interval.to_string());
        let capacity = self.channel_capacity_for(&sub_type);
        let (tx, rx) = tokio::sync::mpsc::channel(capacity);

        self.subscription_manager
            .add_subscription(stream.clone(), symbol.to_string(), sub_type, tx)
            .await?;

        self.message_router.subscribe(vec![stream]).await?;
        Ok(rx)
    }

    /// Subscribes to the mini ticker stream for a symbol.
    ///
    /// Returns a receiver that will receive mini ticker updates as JSON values.
    pub async fn subscribe_mini_ticker(
        &self,
        symbol: &str,
    ) -> Result<tokio::sync::mpsc::Receiver<Value>> {
        let normalized = normalize_symbol(symbol);
        let stream = format!("{}@miniTicker", normalized);
        let capacity = self.channel_capacity_for(&SubscriptionType::Ticker);
        let (tx, rx) = tokio::sync::mpsc::channel(capacity);

        self.subscription_manager
            .add_subscription(
                stream.clone(),
                symbol.to_string(),
                SubscriptionType::Ticker,
                tx,
            )
            .await?;

        self.message_router.subscribe(vec![stream]).await?;
        Ok(rx)
    }

    /// Subscribes to the mini ticker stream for all symbols.
    ///
    /// Returns a receiver that will receive mini ticker updates as JSON values.
    pub async fn subscribe_all_mini_tickers(&self) -> Result<tokio::sync::mpsc::Receiver<Value>> {
        let stream = "!miniTicker@arr".to_string();
        let capacity = self.channel_capacity_for(&SubscriptionType::Ticker);
        let (tx, rx) = tokio::sync::mpsc::channel(capacity);

        self.subscription_manager
            .add_subscription(
                stream.clone(),
                "all".to_string(),
                SubscriptionType::Ticker,
                tx,
            )
            .await?;

        self.message_router.subscribe(vec![stream]).await?;
        Ok(rx)
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

    /// Returns a health snapshot for monitoring the WebSocket connection.
    ///
    /// This provides metrics useful for monitoring connection health:
    /// - Latency from ping/pong
    /// - Message counts (received and dropped)
    /// - Connection uptime
    /// - Reconnection count
    pub fn health(&self) -> WsHealthSnapshot {
        let now = chrono::Utc::now().timestamp_millis() as u64;
        let start_time = self.health.connection_start_time.load(Ordering::Relaxed);
        let last_msg = self.health.last_message_time.load(Ordering::Relaxed);

        WsHealthSnapshot {
            latency_ms: self.message_router.latency(),
            messages_received: self.health.messages_received.load(Ordering::Relaxed),
            messages_dropped: self.health.messages_dropped.load(Ordering::Relaxed),
            last_message_time: if last_msg > 0 {
                Some(last_msg as i64)
            } else {
                None
            },
            connection_uptime_ms: if start_time > 0 {
                now.saturating_sub(start_time)
            } else {
                0
            },
            reconnect_count: self.message_router.reconnect_count(),
        }
    }

    /// Records that a message was received (for health tracking).
    #[inline]
    #[allow(dead_code)]
    pub(crate) fn record_message_received(&self) {
        self.health
            .messages_received
            .fetch_add(1, Ordering::Relaxed);
        self.health.last_message_time.store(
            chrono::Utc::now().timestamp_millis() as u64,
            Ordering::Relaxed,
        );
    }

    /// Records that a message was dropped (for health tracking).
    #[inline]
    #[allow(dead_code)]
    pub(crate) fn record_message_dropped(&self) {
        let count = self.health.messages_dropped.fetch_add(1, Ordering::Relaxed) + 1;
        // Log every 100th drop to avoid log spam
        if count % 100 == 1 {
            tracing::warn!(dropped_count = count, "Message dropped due to backpressure");
        }
    }

    /// Generic method for watching a WebSocket stream.
    ///
    /// This method abstracts the common pattern used by all watch_* methods:
    /// 1. Create a channel for receiving messages
    /// 2. Add subscription to the manager
    /// 3. Subscribe via the message router
    /// 4. Wait for and parse messages
    ///
    /// # Type Parameters
    /// * `T` - The output type to parse messages into
    /// * `P` - The parser type implementing `StreamParser<Output = T>`
    ///
    /// # Arguments
    /// * `stream` - The stream name (e.g., "btcusdt@ticker")
    /// * `symbol` - The symbol for subscription tracking
    /// * `sub_type` - The subscription type
    /// * `market` - Optional market info for parsing
    async fn watch_stream<T, P>(
        &self,
        stream: String,
        symbol: String,
        sub_type: SubscriptionType,
        market: Option<&ccxt_core::types::Market>,
    ) -> Result<T>
    where
        P: parsers::StreamParser<Output = T>,
    {
        let capacity = self.channel_capacity_for(&sub_type);
        let (tx, mut rx) = tokio::sync::mpsc::channel(capacity);
        self.subscription_manager
            .add_subscription(stream.clone(), symbol, sub_type, tx)
            .await?;

        self.message_router.subscribe(vec![stream.clone()]).await?;

        let timeout_duration = self.watch_timeout();

        loop {
            match tokio::time::timeout(timeout_duration, rx.recv()).await {
                Ok(Some(message)) => {
                    // Skip subscription confirmation messages
                    if message.get("result").is_some() {
                        continue;
                    }

                    match P::parse(&message, market) {
                        Ok(data) => return Ok(data),
                        Err(e) => {
                            tracing::warn!(
                                "Failed to parse message for stream {}: {:?}. Payload: {:?}",
                                stream,
                                e,
                                message
                            );
                            // Continue waiting for the next message
                        }
                    }
                }
                Ok(None) => {
                    return Err(Error::network("Subscription channel closed"));
                }
                Err(_) => {
                    return Err(Error::network(format!(
                        "No valid message received within {:?} for stream {}",
                        timeout_duration, stream
                    )));
                }
            }
        }
    }

    /// Watches a single mark price stream (internal helper)
    async fn watch_mark_price_internal(
        &self,
        symbol: &str,
        channel_name: &str,
    ) -> Result<MarkPrice> {
        let normalized = normalize_symbol(symbol);
        let stream = format!("{}@{}", normalized, channel_name);
        tracing::debug!(
            "watch_mark_price_internal: stream={}, symbol={}",
            stream,
            symbol
        );

        let mark_price = self
            .watch_stream::<MarkPrice, parsers::MarkPriceParser>(
                stream,
                symbol.to_string(),
                SubscriptionType::MarkPrice,
                None,
            )
            .await?;

        // Update cache
        let mut mark_prices = self.cache.mark_prices.lock().await;
        mark_prices.insert(mark_price.symbol.clone(), mark_price.clone());

        Ok(mark_price)
    }

    /// Watches multiple mark price streams (internal helper)
    async fn watch_mark_prices_internal(
        &self,
        symbols: Option<Vec<String>>,
        channel_name: &str,
    ) -> Result<HashMap<String, MarkPrice>> {
        let capacity = self.channel_capacity_for(&SubscriptionType::MarkPrice);
        let (tx, mut rx) = tokio::sync::mpsc::channel(capacity);

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

        let timeout_duration = self.watch_timeout();

        loop {
            match tokio::time::timeout(timeout_duration, rx.recv()).await {
                Ok(Some(message)) => {
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

                                let mut mark_prices = self.cache.mark_prices.lock().await;
                                mark_prices.insert(symbol, mark_price);
                            } else {
                                tracing::warn!(
                                    "Failed to parse item in mark price array: {:?}",
                                    item
                                );
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

                                let mut mark_prices = self.cache.mark_prices.lock().await;
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
                }
                Ok(None) => {
                    return Err(Error::network("Subscription channel closed"));
                }
                Err(_) => {
                    return Err(Error::network(format!(
                        "No valid message received within {:?} for mark prices",
                        timeout_duration
                    )));
                }
            }
        }
    }

    /// Watches a single ticker stream (internal helper)
    async fn watch_ticker_internal(&self, symbol: &str, channel_name: &str) -> Result<Ticker> {
        let normalized = normalize_symbol(symbol);
        let stream = format!("{}@{}", normalized, channel_name);

        let ticker = self
            .watch_stream::<Ticker, parsers::TickerParser>(
                stream,
                symbol.to_string(),
                SubscriptionType::Ticker,
                None,
            )
            .await?;

        // Update cache
        let mut tickers = self.cache.tickers.lock().await;
        tickers.insert(ticker.symbol.clone(), ticker.clone());

        Ok(ticker)
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

        let capacity = self.channel_capacity_for(&SubscriptionType::Ticker);
        let (tx, mut rx) = tokio::sync::mpsc::channel(capacity);

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

        let timeout_duration = self.watch_timeout();

        loop {
            match tokio::time::timeout(timeout_duration, rx.recv()).await {
                Ok(Some(message)) => {
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

                                let mut tickers = self.cache.tickers.lock().await;
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

                                let mut tickers = self.cache.tickers.lock().await;
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
                }
                Ok(None) => {
                    return Err(Error::network("Subscription channel closed"));
                }
                Err(_) => {
                    return Err(Error::network(format!(
                        "No valid message received within {:?} for tickers",
                        timeout_duration
                    )));
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
        handlers::handle_orderbook_delta(symbol, delta_message, is_futures, &self.cache.orderbooks)
            .await
    }

    /// Retrieves an order book snapshot and initializes cached state (internal helper)
    async fn fetch_orderbook_snapshot(
        &self,
        exchange: &Binance,
        symbol: &str,
        limit: Option<i64>,
        is_futures: bool,
    ) -> Result<OrderBook> {
        handlers::fetch_orderbook_snapshot(
            exchange,
            symbol,
            limit,
            is_futures,
            &self.cache.orderbooks,
        )
        .await
    }

    /// Watches a single order book stream (internal helper)
    async fn watch_orderbook_internal(
        &self,
        exchange: &Binance,
        symbol: &str,
        limit: Option<i64>,
        update_speed: UpdateSpeed,
        is_futures: bool,
    ) -> Result<OrderBook> {
        let stream = match update_speed {
            UpdateSpeed::Ms100 => format!("{}@depth@100ms", symbol.to_lowercase()),
            UpdateSpeed::Ms1000 => format!("{}@depth", symbol.to_lowercase()),
        };

        let capacity = self.channel_capacity_for(&SubscriptionType::OrderBook);
        let (tx, mut rx) = tokio::sync::mpsc::channel(capacity);
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

        let timeout_duration = self.watch_timeout();
        let mut consecutive_errors: u32 = 0;

        loop {
            match tokio::time::timeout(timeout_duration, rx.recv()).await {
                Ok(Some(message)) => {
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
                                    consecutive_errors = 0;
                                    let orderbooks = self.cache.orderbooks.lock().await;
                                    if let Some(ob) = orderbooks.get(symbol) {
                                        if ob.is_synced {
                                            return Ok(ob.clone());
                                        }
                                    }
                                }
                                Err(e) => {
                                    let err_msg = e.to_string();
                                    let recovery = WsErrorRecovery::from_error_message(&err_msg);

                                    match recovery {
                                        WsErrorRecovery::Resync => {
                                            tracing::warn!(
                                                "Resync needed for {}: {}",
                                                symbol,
                                                err_msg
                                            );
                                            match self
                                                .resync_orderbook(
                                                    exchange, symbol, limit, is_futures,
                                                )
                                                .await
                                            {
                                                Ok(true) => {
                                                    tracing::info!(
                                                        "Resync completed successfully for {}",
                                                        symbol
                                                    );
                                                }
                                                Ok(false) => {
                                                    tracing::debug!(
                                                        "Resync rate limited for {}, skipping",
                                                        symbol
                                                    );
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
                                        WsErrorRecovery::Fatal => {
                                            tracing::error!(
                                                "Fatal error handling orderbook delta: {}",
                                                err_msg
                                            );
                                            return Err(e);
                                        }
                                        _ => {
                                            consecutive_errors += 1;
                                            tracing::error!(
                                                consecutive = consecutive_errors,
                                                max = MAX_CONSECUTIVE_DELTA_ERRORS,
                                                "Failed to handle orderbook delta: {}",
                                                err_msg
                                            );
                                            if consecutive_errors >= MAX_CONSECUTIVE_DELTA_ERRORS {
                                                return Err(Error::exchange(
                                                    "Too many consecutive orderbook delta errors",
                                                    err_msg,
                                                ));
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                Ok(None) => {
                    return Err(Error::network("Subscription channel closed"));
                }
                Err(_) => {
                    return Err(Error::network(format!(
                        "No valid message received within {:?} for orderbook {}",
                        timeout_duration, symbol
                    )));
                }
            }
        }
    }

    /// Resyncs the orderbook for a symbol by fetching a fresh snapshot.
    ///
    /// Returns `Ok(true)` if resync was performed, `Ok(false)` if rate limited.
    async fn resync_orderbook(
        &self,
        exchange: &Binance,
        symbol: &str,
        limit: Option<i64>,
        is_futures: bool,
    ) -> Result<bool> {
        let current_time = chrono::Utc::now().timestamp_millis();

        // Check rate limit
        let should_resync = {
            let orderbooks = self.cache.orderbooks.lock().await;
            if let Some(ob) = orderbooks.get(symbol) {
                ob.should_resync(current_time)
            } else {
                true
            }
        };

        if !should_resync {
            return Ok(false);
        }

        // Reset orderbook state
        {
            let mut orderbooks = self.cache.orderbooks.lock().await;
            if let Some(ob) = orderbooks.get_mut(symbol) {
                ob.reset_for_resync();
                ob.mark_resync_initiated(current_time);
            }
        }

        // Wait before fetching snapshot
        tokio::time::sleep(Duration::from_millis(500)).await;

        // Fetch fresh snapshot
        self.fetch_orderbook_snapshot(exchange, symbol, limit, is_futures)
            .await?;

        Ok(true)
    }

    /// Watches multiple order book streams (internal helper)
    async fn watch_orderbooks_internal(
        &self,
        exchange: &Binance,
        symbols: Vec<String>,
        limit: Option<i64>,
        update_speed: UpdateSpeed,
        is_futures: bool,
    ) -> Result<HashMap<String, OrderBook>> {
        if symbols.len() > 200 {
            return Err(Error::invalid_request(
                "Binance supports max 200 symbols per connection",
            ));
        }

        let capacity = self.channel_capacity_for(&SubscriptionType::OrderBook);
        let (tx, mut rx) = tokio::sync::mpsc::channel(capacity);
        let mut streams = Vec::new();

        for symbol in &symbols {
            let stream = match update_speed {
                UpdateSpeed::Ms100 => format!("{}@depth@100ms", symbol.to_lowercase()),
                UpdateSpeed::Ms1000 => format!("{}@depth", symbol.to_lowercase()),
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

        let timeout_duration = self.watch_timeout();

        while synced_symbols.len() < symbols.len() {
            match tokio::time::timeout(timeout_duration, rx.recv()).await {
                Ok(Some(message)) => {
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

                                let orderbooks = self.cache.orderbooks.lock().await;
                                if let Some(ob) = orderbooks.get(msg_symbol) {
                                    if ob.is_synced {
                                        synced_symbols.insert(msg_symbol.to_string());
                                    }
                                }
                            }
                        }
                    }
                }
                Ok(None) => {
                    return Err(Error::network("Subscription channel closed"));
                }
                Err(_) => {
                    return Err(Error::network(format!(
                        "No valid message received within {:?} for orderbooks",
                        timeout_duration
                    )));
                }
            }
        }

        let orderbooks = self.cache.orderbooks.lock().await;
        for symbol in &symbols {
            if let Some(ob) = orderbooks.get(symbol) {
                result.insert(symbol.clone(), ob.clone());
            }
        }

        Ok(result)
    }

    /// Watches the best bid/ask data for a unified symbol (internal helper)
    async fn watch_bids_asks_internal(&self, symbol: &str, market_id: &str) -> Result<BidAsk> {
        let normalized = normalize_symbol(market_id);
        let stream = format!("{}@bookTicker", normalized);

        let bid_ask = self
            .watch_stream::<BidAsk, parsers::BidAskParser>(
                stream,
                symbol.to_string(),
                SubscriptionType::BookTicker,
                None,
            )
            .await?;

        // Update cache
        let mut bids_asks_map = self.cache.bids_asks.lock().await;
        bids_asks_map.insert(symbol.to_string(), bid_ask.clone());

        Ok(bid_ask)
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
        let capacity = self.channel_capacity_for(&SubscriptionType::Trades);
        let (tx, mut rx) = tokio::sync::mpsc::channel(capacity);

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
                    let mut trades_map = self.cache.trades.lock().await;
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
        let sub_type = SubscriptionType::Kline(timeframe.to_string());
        let capacity = self.channel_capacity_for(&sub_type);
        let (tx, mut rx) = tokio::sync::mpsc::channel(capacity);

        self.subscription_manager
            .add_subscription(stream.clone(), symbol.to_string(), sub_type, tx)
            .await?;

        self.message_router.subscribe(vec![stream.clone()]).await?;

        loop {
            if let Some(message) = rx.recv().await {
                if message.get("result").is_some() {
                    continue;
                }

                if let Ok(ohlcv) = parser::parse_ws_ohlcv(&message) {
                    let cache_key = format!("{}:{}", symbol, timeframe);
                    let mut ohlcvs_map = self.cache.ohlcvs.lock().await;
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
        let tickers = self.cache.tickers.lock().await;
        tickers.get(symbol).cloned()
    }

    /// Returns all cached ticker snapshots
    pub async fn get_all_cached_tickers(&self) -> HashMap<String, Ticker> {
        let tickers = self.cache.tickers.lock().await;
        tickers.clone()
    }

    /// Handles balance update messages (internal helper)
    async fn handle_balance_message(&self, message: &Value, account_type: &str) -> Result<()> {
        user_data::handle_balance_message(message, account_type, &self.cache.balances).await
    }

    /// Watches for balance updates (internal helper)
    async fn watch_balance_internal(&self, account_type: &str) -> Result<Balance> {
        self.connect_user_stream().await?;

        let capacity = self.channel_capacity_for(&SubscriptionType::Balance);
        let (tx, mut rx) = tokio::sync::mpsc::channel(capacity);

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
                            let balances = self.cache.balances.read().await;
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

        let capacity = self.channel_capacity_for(&SubscriptionType::Orders);
        let (tx, mut rx) = tokio::sync::mpsc::channel(capacity);

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

                            let mut orders = self.cache.orders.write().await;
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
                                        let mut trades = self.cache.my_trades.write().await;
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

        let capacity = self.channel_capacity_for(&SubscriptionType::MyTrades);
        let (tx, mut rx) = tokio::sync::mpsc::channel(capacity);

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

                            let mut trades_map = self.cache.my_trades.write().await;
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

        let capacity = self.channel_capacity_for(&SubscriptionType::Positions);
        let (tx, mut rx) = tokio::sync::mpsc::channel(capacity);

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

                                        let mut positions_map = self.cache.positions.write().await;
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
        let orders_map = self.cache.orders.read().await;

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
        let trades_map = self.cache.my_trades.read().await;

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
        let positions_map = self.cache.positions.read().await;

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
    use streams::WS_BASE_URL;
    use types::{
        DEFAULT_ORDERBOOK_CAPACITY, DEFAULT_TICKER_CAPACITY, DEFAULT_TRADES_CAPACITY,
        DEFAULT_USER_DATA_CAPACITY,
    };

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

    #[test]
    fn test_ws_health_snapshot_default() {
        let health = WsHealthSnapshot::default();
        assert_eq!(health.messages_received, 0);
        assert_eq!(health.messages_dropped, 0);
        assert!(health.latency_ms.is_none());
        assert!(health.last_message_time.is_none());
        assert_eq!(health.connection_uptime_ms, 0);
        assert_eq!(health.reconnect_count, 0);
    }

    #[test]
    fn test_ws_health_snapshot_is_healthy() {
        let mut health = WsHealthSnapshot::default();

        // Empty snapshot is healthy (no data yet)
        assert!(health.is_healthy());

        // High drop rate is unhealthy
        health.messages_received = 100;
        health.messages_dropped = 20; // 20% drop rate
        assert!(!health.is_healthy());

        // Low drop rate is healthy
        health.messages_dropped = 5; // 5% drop rate
        assert!(health.is_healthy());
    }

    #[tokio::test]
    async fn test_shutdown_sets_flags() {
        let ws = BinanceWs::new("wss://stream.binance.com:9443/ws".to_string());

        // Initially not shutting down
        assert!(!ws.is_shutting_down());

        // After shutdown, flag should be set
        let _ = ws.shutdown().await;
        assert!(ws.is_shutting_down());
    }

    #[tokio::test]
    async fn test_shutdown_is_idempotent() {
        let ws = BinanceWs::new("wss://stream.binance.com:9443/ws".to_string());

        // Multiple shutdowns should not panic
        let result1 = ws.shutdown().await;
        let result2 = ws.shutdown().await;

        assert!(result1.is_ok());
        assert!(result2.is_ok());
    }

    #[tokio::test]
    async fn test_channel_capacity_configuration() {
        // Test default configuration
        let ws = BinanceWs::new("wss://stream.binance.com:9443/ws".to_string());

        assert_eq!(
            ws.channel_capacity_for(&SubscriptionType::Ticker),
            DEFAULT_TICKER_CAPACITY
        );
        assert_eq!(
            ws.channel_capacity_for(&SubscriptionType::OrderBook),
            DEFAULT_ORDERBOOK_CAPACITY
        );
        assert_eq!(
            ws.channel_capacity_for(&SubscriptionType::Trades),
            DEFAULT_TRADES_CAPACITY
        );
        assert_eq!(
            ws.channel_capacity_for(&SubscriptionType::Balance),
            DEFAULT_USER_DATA_CAPACITY
        );
    }

    #[tokio::test]
    async fn test_custom_channel_capacity_configuration() {
        use ccxt_core::ws_client::BackpressureStrategy;

        let custom_config = WsChannelConfig {
            ticker_capacity: 128,
            orderbook_capacity: 256,
            trades_capacity: 512,
            user_data_capacity: 64,
        };

        let config = BinanceWsConfig::new("wss://stream.binance.com:9443/ws".to_string())
            .with_channel_config(custom_config)
            .with_backpressure(BackpressureStrategy::DropOldest);

        let ws = BinanceWs::new_with_config(config);

        assert_eq!(ws.channel_capacity_for(&SubscriptionType::Ticker), 128);
        assert_eq!(ws.channel_capacity_for(&SubscriptionType::OrderBook), 256);
        assert_eq!(ws.channel_capacity_for(&SubscriptionType::Trades), 512);
        assert_eq!(ws.channel_capacity_for(&SubscriptionType::Balance), 64);
        assert_eq!(ws.channel_capacity_for(&SubscriptionType::Orders), 64);
        assert_eq!(ws.channel_capacity_for(&SubscriptionType::MyTrades), 64);
        assert_eq!(ws.channel_capacity_for(&SubscriptionType::Positions), 64);
    }
}
