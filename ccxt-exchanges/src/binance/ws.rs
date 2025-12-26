//! Binance WebSocket implementation
//!
//! Provides WebSocket real-time data stream subscriptions for the Binance exchange

use crate::binance::Binance;
use crate::binance::parser;
use ccxt_core::error::{Error, Result};
use ccxt_core::types::financial::{Amount, Cost, Price};
use ccxt_core::types::{
    Balance, BidAsk, MarkPrice, MarketType, OHLCV, Order, OrderBook, Position, Ticker, Trade,
};
use ccxt_core::ws_client::{WsClient, WsConfig};
use serde_json::Value;
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, RwLock};
use tokio::task::JoinHandle;
use tokio_tungstenite::tungstenite::protocol::Message;

/// Binance WebSocket endpoints
#[allow(dead_code)]
const WS_BASE_URL: &str = "wss://stream.binance.com:9443/ws";
#[allow(dead_code)]
const WS_TESTNET_URL: &str = "wss://testnet.binance.vision/ws";

/// Listen key refresh interval (30 minutes)
const LISTEN_KEY_REFRESH_INTERVAL: Duration = Duration::from_secs(30 * 60);

/// Listen key manager
///
/// Automatically manages Binance user data stream listen keys by:
/// - Creating and caching listen keys
/// - Refreshing them every 30 minutes
/// - Detecting expiration and rebuilding
/// - Tracking connection state
pub struct ListenKeyManager {
    /// Reference to the Binance instance
    binance: Arc<Binance>,
    /// Currently active listen key
    listen_key: Arc<RwLock<Option<String>>>,
    /// Listen key creation timestamp
    created_at: Arc<RwLock<Option<Instant>>>,
    /// Configured refresh interval
    refresh_interval: Duration,
    /// Handle to the auto-refresh task
    refresh_task: Arc<Mutex<Option<JoinHandle<()>>>>,
}

impl ListenKeyManager {
    /// Creates a new `ListenKeyManager`
    ///
    /// # Arguments
    /// * `binance` - Reference-counted Binance instance
    ///
    /// # Returns
    /// Configured `ListenKeyManager`
    pub fn new(binance: Arc<Binance>) -> Self {
        Self {
            binance,
            listen_key: Arc::new(RwLock::new(None)),
            created_at: Arc::new(RwLock::new(None)),
            refresh_interval: LISTEN_KEY_REFRESH_INTERVAL,
            refresh_task: Arc::new(Mutex::new(None)),
        }
    }

    /// Retrieves or creates a listen key
    ///
    /// Returns an existing valid listen key when present; otherwise creates a new one.
    ///
    /// # Returns
    /// Listen key string
    ///
    /// # Errors
    /// - Failed to create the listen key
    /// - Missing API credentials
    pub async fn get_or_create(&self) -> Result<String> {
        // Check if a listen key already exists
        let key_opt = self.listen_key.read().await.clone();

        if let Some(key) = key_opt {
            // Check whether the key needs to be refreshed
            let created = self.created_at.read().await;
            if let Some(created_time) = *created {
                let elapsed = created_time.elapsed();
                // If more than 50 minutes have elapsed, create a new key
                if elapsed > Duration::from_secs(50 * 60) {
                    drop(created);
                    return self.create_new().await;
                }
            }
            return Ok(key);
        }

        // Create a new listen key
        self.create_new().await
    }

    /// Creates a new listen key
    ///
    /// # Returns
    /// Newly created listen key
    async fn create_new(&self) -> Result<String> {
        let key = self.binance.create_listen_key().await?;

        // Update cache
        *self.listen_key.write().await = Some(key.clone());
        *self.created_at.write().await = Some(Instant::now());

        Ok(key)
    }

    /// Refreshes the current listen key
    ///
    /// Extends the listen key lifetime by 60 minutes
    ///
    /// # Returns
    /// Refresh result
    pub async fn refresh(&self) -> Result<()> {
        let key_opt = self.listen_key.read().await.clone();

        if let Some(key) = key_opt {
            self.binance.refresh_listen_key(&key).await?;
            // Update the creation timestamp
            *self.created_at.write().await = Some(Instant::now());
            Ok(())
        } else {
            Err(Error::invalid_request("No listen key to refresh"))
        }
    }

    /// Starts the auto-refresh task
    ///
    /// Refreshes the listen key every 30 minutes
    pub async fn start_auto_refresh(&self) {
        // Stop any existing task
        self.stop_auto_refresh().await;

        let listen_key = self.listen_key.clone();
        let created_at = self.created_at.clone();
        let binance = self.binance.clone();
        let interval = self.refresh_interval;

        let handle = tokio::spawn(async move {
            loop {
                tokio::time::sleep(interval).await;

                // Check whether a listen key exists
                let key_opt = listen_key.read().await.clone();
                if let Some(key) = key_opt {
                    // Attempt to refresh the key
                    match binance.refresh_listen_key(&key).await {
                        Ok(_) => {
                            *created_at.write().await = Some(Instant::now());
                            // Listen key refreshed successfully
                        }
                        Err(_e) => {
                            // Failed to refresh; clear cache so a new key will be created next time
                            *listen_key.write().await = None;
                            *created_at.write().await = None;
                            break;
                        }
                    }
                } else {
                    // No listen key available; stop the task
                    break;
                }
            }
        });

        *self.refresh_task.lock().await = Some(handle);
    }

    /// Stops the auto-refresh task
    pub async fn stop_auto_refresh(&self) {
        let mut task_opt = self.refresh_task.lock().await;
        if let Some(handle) = task_opt.take() {
            handle.abort();
        }
    }

    /// Deletes the listen key
    ///
    /// Closes the user data stream and invalidates the key
    ///
    /// # Returns
    /// Result of the deletion
    pub async fn delete(&self) -> Result<()> {
        // Stop the auto-refresh first
        self.stop_auto_refresh().await;

        let key_opt = self.listen_key.read().await.clone();

        if let Some(key) = key_opt {
            self.binance.delete_listen_key(&key).await?;

            // Clear cached state
            *self.listen_key.write().await = None;
            *self.created_at.write().await = None;

            Ok(())
        } else {
            Ok(()) // No listen key; treat as success
        }
    }

    /// Returns the current listen key when available
    pub async fn get_current(&self) -> Option<String> {
        self.listen_key.read().await.clone()
    }

    /// Checks whether the listen key is still valid
    pub async fn is_valid(&self) -> bool {
        let key_opt = self.listen_key.read().await;
        if key_opt.is_none() {
            return false;
        }

        let created = self.created_at.read().await;
        if let Some(created_time) = *created {
            // Key is considered valid if less than 55 minutes old
            created_time.elapsed() < Duration::from_secs(55 * 60)
        } else {
            false
        }
    }
}

impl Drop for ListenKeyManager {
    fn drop(&mut self) {
        // Note: Drop is synchronous, so we cannot await asynchronous operations here.
        // Callers should explicitly invoke `delete()` to release resources.
    }
}
/// Subscription types supported by the Binance WebSocket API.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SubscriptionType {
    /// 24-hour ticker stream
    Ticker,
    /// Order book depth stream
    OrderBook,
    /// Real-time trade stream
    Trades,
    /// Kline (candlestick) stream with interval (e.g. "1m", "5m", "1h")
    Kline(String),
    /// Account balance stream
    Balance,
    /// Order update stream
    Orders,
    /// Position update stream
    Positions,
    /// Personal trade execution stream
    MyTrades,
    /// Mark price stream
    MarkPrice,
    /// Book ticker (best bid/ask) stream
    BookTicker,
}

impl SubscriptionType {
    /// Infers a subscription type from a stream name
    ///
    /// # Arguments
    /// * `stream` - Stream identifier (e.g. "btcusdt@ticker")
    ///
    /// # Returns
    /// Subscription type, when the stream can be identified
    pub fn from_stream(stream: &str) -> Option<Self> {
        if stream.contains("@ticker") {
            Some(Self::Ticker)
        } else if stream.contains("@depth") {
            Some(Self::OrderBook)
        } else if stream.contains("@trade") || stream.contains("@aggTrade") {
            Some(Self::Trades)
        } else if stream.contains("@kline_") {
            // Extract timeframe suffix
            let parts: Vec<&str> = stream.split("@kline_").collect();
            if parts.len() == 2 {
                Some(Self::Kline(parts[1].to_string()))
            } else {
                None
            }
        } else if stream.contains("@markPrice") {
            Some(Self::MarkPrice)
        } else if stream.contains("@bookTicker") {
            Some(Self::BookTicker)
        } else {
            None
        }
    }
}

/// Subscription metadata
#[derive(Clone)]
pub struct Subscription {
    /// Stream name (e.g. "btcusdt@ticker")
    pub stream: String,
    /// Normalized trading symbol (e.g. "BTCUSDT")
    pub symbol: String,
    /// Subscription type descriptor
    pub sub_type: SubscriptionType,
    /// Timestamp when the subscription was created
    pub subscribed_at: Instant,
    /// Sender for forwarding WebSocket messages to consumers
    pub sender: tokio::sync::mpsc::UnboundedSender<Value>,
}

impl Subscription {
    /// Creates a new subscription with the provided parameters
    pub fn new(
        stream: String,
        symbol: String,
        sub_type: SubscriptionType,
        sender: tokio::sync::mpsc::UnboundedSender<Value>,
    ) -> Self {
        Self {
            stream,
            symbol,
            sub_type,
            subscribed_at: Instant::now(),
            sender,
        }
    }

    /// Sends a message to the subscriber
    ///
    /// # Arguments
    /// * `message` - WebSocket payload to forward
    ///
    /// # Returns
    /// Whether the message was delivered successfully
    pub fn send(&self, message: Value) -> bool {
        self.sender.send(message).is_ok()
    }
}

/// Subscription manager
///
/// Manages the lifecycle of all WebSocket subscriptions, including:
/// - Adding and removing subscriptions
/// - Querying and validating subscriptions
/// - Tracking the number of active subscriptions
pub struct SubscriptionManager {
    /// Mapping of `stream_name -> Subscription`
    subscriptions: Arc<RwLock<HashMap<String, Subscription>>>,
    /// Index by symbol: `symbol -> Vec<stream_name>`
    symbol_index: Arc<RwLock<HashMap<String, Vec<String>>>>,
    /// Counter of active subscriptions
    active_count: Arc<std::sync::atomic::AtomicUsize>,
}

impl SubscriptionManager {
    /// Creates a new `SubscriptionManager`
    pub fn new() -> Self {
        Self {
            subscriptions: Arc::new(RwLock::new(HashMap::new())),
            symbol_index: Arc::new(RwLock::new(HashMap::new())),
            active_count: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
        }
    }

    /// Adds a subscription to the manager
    ///
    /// # Arguments
    /// * `stream` - Stream identifier (e.g. "btcusdt@ticker")
    /// * `symbol` - Normalized trading symbol (e.g. "BTCUSDT")
    /// * `sub_type` - Subscription classification
    /// * `sender` - Channel for dispatching messages
    ///
    /// # Returns
    /// Result of the insertion
    pub async fn add_subscription(
        &self,
        stream: String,
        symbol: String,
        sub_type: SubscriptionType,
        sender: tokio::sync::mpsc::UnboundedSender<Value>,
    ) -> Result<()> {
        let subscription = Subscription::new(stream.clone(), symbol.clone(), sub_type, sender);

        // Insert into the subscription map
        let mut subs = self.subscriptions.write().await;
        subs.insert(stream.clone(), subscription);

        // Update the per-symbol index
        let mut index = self.symbol_index.write().await;
        index.entry(symbol).or_insert_with(Vec::new).push(stream);

        // Increment the active subscription count
        self.active_count
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);

        Ok(())
    }

    /// Removes a subscription by stream name
    ///
    /// # Arguments
    /// * `stream` - Stream identifier to remove
    ///
    /// # Returns
    /// Result of the removal
    pub async fn remove_subscription(&self, stream: &str) -> Result<()> {
        let mut subs = self.subscriptions.write().await;

        if let Some(subscription) = subs.remove(stream) {
            // Remove from the symbol index as well
            let mut index = self.symbol_index.write().await;
            if let Some(streams) = index.get_mut(&subscription.symbol) {
                streams.retain(|s| s != stream);
                // Drop the symbol entry when no subscriptions remain
                if streams.is_empty() {
                    index.remove(&subscription.symbol);
                }
            }

            // Decrement the active subscription counter
            self.active_count
                .fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
        }

        Ok(())
    }

    /// Retrieves a subscription by stream name
    ///
    /// # Arguments
    /// * `stream` - Stream identifier
    ///
    /// # Returns
    /// Optional subscription record
    pub async fn get_subscription(&self, stream: &str) -> Option<Subscription> {
        let subs = self.subscriptions.read().await;
        subs.get(stream).cloned()
    }

    /// Checks whether a subscription exists for the given stream
    ///
    /// # Arguments
    /// * `stream` - Stream identifier
    ///
    /// # Returns
    /// `true` if the subscription exists, otherwise `false`
    pub async fn has_subscription(&self, stream: &str) -> bool {
        let subs = self.subscriptions.read().await;
        subs.contains_key(stream)
    }

    /// Returns all registered subscriptions
    pub async fn get_all_subscriptions(&self) -> Vec<Subscription> {
        let subs = self.subscriptions.read().await;
        subs.values().cloned().collect()
    }

    /// Returns all subscriptions associated with a symbol
    ///
    /// # Arguments
    /// * `symbol` - Trading symbol
    pub async fn get_subscriptions_by_symbol(&self, symbol: &str) -> Vec<Subscription> {
        let index = self.symbol_index.read().await;
        let subs = self.subscriptions.read().await;

        if let Some(streams) = index.get(symbol) {
            streams
                .iter()
                .filter_map(|stream| subs.get(stream).cloned())
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Returns the number of active subscriptions
    pub fn active_count(&self) -> usize {
        self.active_count.load(std::sync::atomic::Ordering::SeqCst)
    }

    /// Removes all subscriptions and clears indexes
    pub async fn clear(&self) {
        let mut subs = self.subscriptions.write().await;
        let mut index = self.symbol_index.write().await;

        subs.clear();
        index.clear();
        self.active_count
            .store(0, std::sync::atomic::Ordering::SeqCst);
    }

    /// Sends a message to subscribers of a specific stream
    ///
    /// # Arguments
    /// * `stream` - Stream identifier
    /// * `message` - WebSocket payload to broadcast
    ///
    /// # Returns
    /// `true` when the message was delivered, otherwise `false`
    pub async fn send_to_stream(&self, stream: &str, message: Value) -> bool {
        let subs = self.subscriptions.read().await;
        if let Some(subscription) = subs.get(stream) {
            subscription.send(message)
        } else {
            false
        }
    }

    /// Sends a message to all subscribers of a symbol
    ///
    /// # Arguments
    /// * `symbol` - Trading symbol
    /// * `message` - Shared WebSocket payload
    ///
    /// # Returns
    /// Number of subscribers that received the message
    pub async fn send_to_symbol(&self, symbol: &str, message: &Value) -> usize {
        let index = self.symbol_index.read().await;
        let subs = self.subscriptions.read().await;

        let mut sent_count = 0;

        if let Some(streams) = index.get(symbol) {
            for stream in streams {
                if let Some(subscription) = subs.get(stream) {
                    if subscription.send(message.clone()) {
                        sent_count += 1;
                    }
                }
            }
        }

        sent_count
    }
}
/// Reconnect configuration
///
/// Defines the automatic reconnection strategy after a WebSocket disconnect
#[derive(Debug, Clone)]
pub struct ReconnectConfig {
    /// Enables or disables automatic reconnection
    pub enabled: bool,

    /// Initial reconnection delay in milliseconds
    pub initial_delay_ms: u64,

    /// Maximum reconnection delay in milliseconds
    pub max_delay_ms: u64,

    /// Exponential backoff multiplier
    pub backoff_multiplier: f64,

    /// Maximum number of reconnection attempts (0 means unlimited)
    pub max_attempts: usize,
}

impl Default for ReconnectConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            initial_delay_ms: 1000,  // 1 second
            max_delay_ms: 30000,     // 30 seconds
            backoff_multiplier: 2.0, // Exponential backoff
            max_attempts: 0,         // Unlimited retries
        }
    }
}

impl ReconnectConfig {
    /// Calculates the reconnection delay
    ///
    /// Uses exponential backoff to determine the delay duration
    ///
    /// # Arguments
    /// * `attempt` - Current reconnection attempt (zero-based)
    ///
    /// # Returns
    /// Delay duration in milliseconds
    pub fn calculate_delay(&self, attempt: usize) -> u64 {
        let delay = (self.initial_delay_ms as f64) * self.backoff_multiplier.powi(attempt as i32);
        delay.min(self.max_delay_ms as f64) as u64
    }

    /// Determines whether another reconnection attempt should be made
    ///
    /// # Arguments
    /// * `attempt` - Current reconnection attempt
    ///
    /// # Returns
    /// `true` if another retry should be attempted
    pub fn should_retry(&self, attempt: usize) -> bool {
        self.enabled && (self.max_attempts == 0 || attempt < self.max_attempts)
    }
}

/// Message router
///
/// Handles reception, parsing, and dispatching of WebSocket messages. Core responsibilities:
/// - Managing WebSocket connections
/// - Receiving and routing messages
/// - Coordinating automatic reconnection
/// - Managing subscriptions
pub struct MessageRouter {
    /// WebSocket client instance
    ws_client: Arc<RwLock<Option<WsClient>>>,

    /// Subscription manager registry
    subscription_manager: Arc<SubscriptionManager>,

    /// Handle to the background routing task
    router_task: Arc<Mutex<Option<JoinHandle<()>>>>,

    /// Connection state flag
    is_connected: Arc<std::sync::atomic::AtomicBool>,

    /// Configuration for reconnection behavior
    reconnect_config: Arc<RwLock<ReconnectConfig>>,

    /// WebSocket endpoint URL
    ws_url: String,

    /// Request ID counter (used for subscribe/unsubscribe)
    request_id: Arc<std::sync::atomic::AtomicU64>,
}

impl MessageRouter {
    /// Creates a new message router
    ///
    /// # Arguments
    /// * `ws_url` - WebSocket connection URL
    /// * `subscription_manager` - Subscription manager handle
    ///
    /// # Returns
    /// Configured router instance
    pub fn new(ws_url: String, subscription_manager: Arc<SubscriptionManager>) -> Self {
        Self {
            ws_client: Arc::new(RwLock::new(None)),
            subscription_manager,
            router_task: Arc::new(Mutex::new(None)),
            is_connected: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            reconnect_config: Arc::new(RwLock::new(ReconnectConfig::default())),
            ws_url,
            request_id: Arc::new(std::sync::atomic::AtomicU64::new(1)),
        }
    }

    /// Starts the message router
    ///
    /// Establishes the WebSocket connection and launches the message loop
    ///
    /// # Returns
    /// Result of the startup sequence
    pub async fn start(&self) -> Result<()> {
        // Stop any running instance before starting again
        if self.is_connected() {
            self.stop().await?;
        }

        // Establish a new WebSocket connection
        let config = WsConfig {
            url: self.ws_url.clone(),
            ..Default::default()
        };
        let client = WsClient::new(config);
        client.connect().await?;

        // Store the client handle
        *self.ws_client.write().await = Some(client);

        // Update connection state
        self.is_connected
            .store(true, std::sync::atomic::Ordering::SeqCst);

        // Spawn the message processing loop
        let ws_client = self.ws_client.clone();
        let subscription_manager = self.subscription_manager.clone();
        let is_connected = self.is_connected.clone();
        let reconnect_config = self.reconnect_config.clone();
        let ws_url = self.ws_url.clone();

        let handle = tokio::spawn(async move {
            Self::message_loop(
                ws_client,
                subscription_manager,
                is_connected,
                reconnect_config,
                ws_url,
            )
            .await
        });

        *self.router_task.lock().await = Some(handle);

        Ok(())
    }

    /// Stops the message router
    ///
    /// Terminates the message loop and disconnects the WebSocket client
    ///
    /// # Returns
    /// Result of the shutdown procedure
    pub async fn stop(&self) -> Result<()> {
        // Mark the connection as inactive
        self.is_connected
            .store(false, std::sync::atomic::Ordering::SeqCst);

        // Cancel the background routing task
        let mut task_opt = self.router_task.lock().await;
        if let Some(handle) = task_opt.take() {
            handle.abort();
        }

        // Disconnect the WebSocket client if present
        let mut client_opt = self.ws_client.write().await;
        if let Some(client) = client_opt.take() {
            let _ = client.disconnect().await;
        }

        Ok(())
    }

    /// Restarts the message router
    ///
    /// Stops the current connection and restarts it, typically used during reconnect scenarios
    ///
    /// # Returns
    /// Result of the restart sequence
    pub async fn restart(&self) -> Result<()> {
        self.stop().await?;
        tokio::time::sleep(Duration::from_millis(100)).await;
        self.start().await
    }

    /// Returns the current connection state
    pub fn is_connected(&self) -> bool {
        self.is_connected.load(std::sync::atomic::Ordering::SeqCst)
    }

    /// Applies a new reconnection configuration
    ///
    /// # Arguments
    /// * `config` - Updated reconnection configuration
    pub async fn set_reconnect_config(&self, config: ReconnectConfig) {
        *self.reconnect_config.write().await = config;
    }

    /// Retrieves the current reconnection configuration
    pub async fn get_reconnect_config(&self) -> ReconnectConfig {
        self.reconnect_config.read().await.clone()
    }

    /// Subscribes to the provided streams
    ///
    /// Sends a subscription request to Binance
    ///
    /// # Arguments
    /// * `streams` - Collection of stream identifiers
    pub async fn subscribe(&self, streams: Vec<String>) -> Result<()> {
        if streams.is_empty() {
            return Ok(());
        }

        let client_opt = self.ws_client.read().await;
        let client = client_opt
            .as_ref()
            .ok_or_else(|| Error::network("WebSocket not connected"))?;

        // Generate a request identifier
        let id = self
            .request_id
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);

        // Build the subscribe payload
        #[allow(clippy::disallowed_methods)]
        let request = serde_json::json!({
            "method": "SUBSCRIBE",
            "params": streams,
            "id": id
        });

        // Send the request to Binance
        client
            .send(Message::Text(request.to_string().into()))
            .await?;

        Ok(())
    }

    /// Unsubscribes from the provided streams
    ///
    /// Sends an unsubscribe request to Binance
    ///
    /// # Arguments
    /// * `streams` - Collection of stream identifiers
    pub async fn unsubscribe(&self, streams: Vec<String>) -> Result<()> {
        if streams.is_empty() {
            return Ok(());
        }

        let client_opt = self.ws_client.read().await;
        let client = client_opt
            .as_ref()
            .ok_or_else(|| Error::network("WebSocket not connected"))?;

        // Generate a request identifier
        let id = self
            .request_id
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);

        // Build the unsubscribe payload
        #[allow(clippy::disallowed_methods)]
        let request = serde_json::json!({
            "method": "UNSUBSCRIBE",
            "params": streams,
            "id": id
        });

        // Send the request to Binance
        client
            .send(Message::Text(request.to_string().into()))
            .await?;

        Ok(())
    }

    /// Message reception loop
    ///
    /// Continuously receives WebSocket messages and routes them to subscribers
    async fn message_loop(
        ws_client: Arc<RwLock<Option<WsClient>>>,
        subscription_manager: Arc<SubscriptionManager>,
        is_connected: Arc<std::sync::atomic::AtomicBool>,
        reconnect_config: Arc<RwLock<ReconnectConfig>>,
        ws_url: String,
    ) {
        let mut reconnect_attempt = 0;

        loop {
            // Exit if instructed to stop
            if !is_connected.load(std::sync::atomic::Ordering::SeqCst) {
                break;
            }

            // Confirm that a client instance exists
            let has_client = ws_client.read().await.is_some();

            if !has_client {
                // Attempt to reconnect when no client is present
                let config = reconnect_config.read().await;
                if config.should_retry(reconnect_attempt) {
                    let delay = config.calculate_delay(reconnect_attempt);
                    drop(config);

                    tokio::time::sleep(Duration::from_millis(delay)).await;

                    match Self::reconnect(&ws_url, ws_client.clone()).await {
                        Ok(_) => {
                            reconnect_attempt = 0; // Reset counter on success
                            continue;
                        }
                        Err(_) => {
                            reconnect_attempt += 1;
                            continue;
                        }
                    }
                } else {
                    // Stop attempting to reconnect
                    is_connected.store(false, std::sync::atomic::Ordering::SeqCst);
                    break;
                }
            }

            // Receive the next message (requires re-acquiring the lock)
            let message_opt = {
                let guard = ws_client.read().await;
                if let Some(client) = guard.as_ref() {
                    client.receive().await
                } else {
                    None
                }
            };

            match message_opt {
                Some(value) => {
                    // Handle the message; ignore failures but continue the loop
                    if let Err(_e) = Self::handle_message(value, subscription_manager.clone()).await
                    {
                        continue;
                    }

                    // Reset the reconnection counter because the connection is healthy
                    reconnect_attempt = 0;
                }
                None => {
                    // Connection failure; attempt to reconnect
                    let config = reconnect_config.read().await;
                    if config.should_retry(reconnect_attempt) {
                        let delay = config.calculate_delay(reconnect_attempt);
                        drop(config);

                        tokio::time::sleep(Duration::from_millis(delay)).await;

                        match Self::reconnect(&ws_url, ws_client.clone()).await {
                            Ok(_) => {
                                reconnect_attempt = 0;
                                continue;
                            }
                            Err(_) => {
                                reconnect_attempt += 1;
                                continue;
                            }
                        }
                    } else {
                        // Reconnection not permitted anymore; stop the loop
                        is_connected.store(false, std::sync::atomic::Ordering::SeqCst);
                        break;
                    }
                }
            }
        }
    }

    /// Processes a WebSocket message
    ///
    /// Parses the message and routes it to relevant subscribers
    async fn handle_message(
        message: Value,
        subscription_manager: Arc<SubscriptionManager>,
    ) -> Result<()> {
        // Determine the stream name for routing
        let stream_name = Self::extract_stream_name(&message)?;

        // Forward the message to the corresponding subscribers
        let sent = subscription_manager
            .send_to_stream(&stream_name, message)
            .await;

        if sent {
            Ok(())
        } else {
            Err(Error::generic("No subscribers for stream"))
        }
    }

    /// Extracts the stream name from an incoming message
    ///
    /// Supports two formats:
    /// 1. Combined stream: `{"stream":"btcusdt@ticker","data":{...}}`
    /// 2. Single stream: `{"e":"24hrTicker","s":"BTCUSDT",...}`
    ///
    /// # Arguments
    /// * `message` - Raw WebSocket payload
    ///
    /// # Returns
    /// Stream identifier for routing
    fn extract_stream_name(message: &Value) -> Result<String> {
        // Attempt to read the combined-stream format
        if let Some(stream) = message.get("stream").and_then(|s| s.as_str()) {
            return Ok(stream.to_string());
        }

        // Fall back to single-stream format: event@symbol (lowercase)
        if let Some(event_type) = message.get("e").and_then(|e| e.as_str()) {
            if let Some(symbol) = message.get("s").and_then(|s| s.as_str()) {
                // Build the stream identifier
                let stream = match event_type {
                    "24hrTicker" => format!("{}@ticker", symbol.to_lowercase()),
                    "depthUpdate" => format!("{}@depth", symbol.to_lowercase()),
                    "aggTrade" => format!("{}@aggTrade", symbol.to_lowercase()),
                    "trade" => format!("{}@trade", symbol.to_lowercase()),
                    "kline" => {
                        // Kline messages carry the interval within the "k" object
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

        // Ignore subscription acknowledgements and error responses
        if message.get("result").is_some() || message.get("error").is_some() {
            return Err(Error::generic("Subscription response, skip routing"));
        }

        Err(Error::generic("Cannot extract stream name from message"))
    }

    /// Reconnects the WebSocket client
    ///
    /// Closes the existing connection and establishes a new one
    async fn reconnect(ws_url: &str, ws_client: Arc<RwLock<Option<WsClient>>>) -> Result<()> {
        // Close the previous connection
        {
            let mut client_opt = ws_client.write().await;
            if let Some(client) = client_opt.take() {
                let _ = client.disconnect().await;
            }
        }

        // Establish a fresh connection
        let config = WsConfig {
            url: ws_url.to_string(),
            ..Default::default()
        };
        let new_client = WsClient::new(config);

        // Connect to the WebSocket endpoint
        new_client.connect().await?;

        // Store the new client handle
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

impl Default for SubscriptionManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Binance WebSocket client wrapper
pub struct BinanceWs {
    client: Arc<WsClient>,
    listen_key: Arc<RwLock<Option<String>>>,
    /// Listen key manager
    listen_key_manager: Option<Arc<ListenKeyManager>>,
    /// Auto-reconnect coordinator
    auto_reconnect_coordinator: Arc<Mutex<Option<ccxt_core::ws_client::AutoReconnectCoordinator>>>,
    /// Cached ticker data
    tickers: Arc<Mutex<HashMap<String, Ticker>>>,
    /// Cached bid/ask snapshots
    bids_asks: Arc<Mutex<HashMap<String, BidAsk>>>,
    /// Cached mark price entries
    #[allow(dead_code)]
    mark_prices: Arc<Mutex<HashMap<String, MarkPrice>>>,
    /// Cached order book snapshots
    orderbooks: Arc<Mutex<HashMap<String, OrderBook>>>,
    /// Cached trade history (retains the latest 1000 entries)
    trades: Arc<Mutex<HashMap<String, VecDeque<Trade>>>>,
    /// Cached OHLCV candles
    ohlcvs: Arc<Mutex<HashMap<String, VecDeque<OHLCV>>>>,
    /// Cached balance data grouped by account type
    balances: Arc<RwLock<HashMap<String, Balance>>>,
    /// Cached orders grouped by symbol then order ID
    orders: Arc<RwLock<HashMap<String, HashMap<String, Order>>>>,
    /// Cached personal trades grouped by symbol (retains the latest 1000 entries)
    my_trades: Arc<RwLock<HashMap<String, VecDeque<Trade>>>>,
    /// Cached positions grouped by symbol and side
    positions: Arc<RwLock<HashMap<String, HashMap<String, Position>>>>,
}

impl BinanceWs {
    /// Creates a new Binance WebSocket client
    ///
    /// # Arguments
    /// * `url` - WebSocket server URL
    ///
    /// # Returns
    /// Initialized Binance WebSocket client
    pub fn new(url: String) -> Self {
        let config = WsConfig {
            url,
            connect_timeout: 10000,
            ping_interval: 180000, // Binance recommends 3 minutes
            reconnect_interval: 5000,
            max_reconnect_attempts: 5,
            auto_reconnect: true,
            enable_compression: false,
            pong_timeout: 90000, // Set pong timeout to 90 seconds
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
    ///
    /// # Arguments
    /// * `url` - WebSocket server URL
    /// * `binance` - Shared Binance instance
    ///
    /// # Returns
    /// Binance WebSocket client configured with a listen key manager
    pub fn new_with_auth(url: String, binance: Arc<Binance>) -> Self {
        let config = WsConfig {
            url,
            connect_timeout: 10000,
            ping_interval: 180000, // Binance recommends 3 minutes
            reconnect_interval: 5000,
            max_reconnect_attempts: 5,
            auto_reconnect: true,
            enable_compression: false,
            pong_timeout: 90000, // Set pong timeout to 90 seconds
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
        // Establish the WebSocket connection
        self.client.connect().await?;

        // Start the auto-reconnect coordinator when not already running
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
        // Stop the auto-reconnect coordinator first
        let mut coordinator_guard = self.auto_reconnect_coordinator.lock().await;
        if let Some(coordinator) = coordinator_guard.take() {
            coordinator.stop().await;
            tracing::info!("Auto-reconnect coordinator stopped");
        }

        // Stop automatic listen key refresh if available
        if let Some(manager) = &self.listen_key_manager {
            manager.stop_auto_refresh().await;
        }

        self.client.disconnect().await
    }

    /// Connects to the user data stream
    ///
    /// Creates or retrieves a listen key, connects to the user data WebSocket, and starts auto-refresh
    ///
    /// # Returns
    /// Result of the connection attempt
    ///
    /// # Errors
    /// - Listen key manager is missing (requires `new_with_auth` constructor)
    /// - Listen key creation failed
    /// - WebSocket connection failed
    ///
    /// # Example
    /// ```rust,no_run
    /// # use ccxt_exchanges::binance::Binance;
    /// # use ccxt_core::ExchangeConfig;
    /// # use std::sync::Arc;
    /// # async fn example() -> ccxt_core::error::Result<()> {
    /// let binance = Arc::new(Binance::new(ExchangeConfig::default())?);
    /// let ws = binance.create_authenticated_ws();
    /// ws.connect_user_stream().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn connect_user_stream(&self) -> Result<()> {
        let manager = self.listen_key_manager.as_ref()
            .ok_or_else(|| Error::invalid_request(
                "Listen key manager not available. Use new_with_auth() to create authenticated WebSocket"
            ))?;

        // Obtain an existing listen key or create a new one
        let listen_key = manager.get_or_create().await?;

        // Update the WebSocket URL using the listen key
        let user_stream_url = format!("wss://stream.binance.com:9443/ws/{}", listen_key);

        // Recreate the WebSocket client configuration
        let config = WsConfig {
            url: user_stream_url,
            connect_timeout: 10000,
            ping_interval: 180000,
            reconnect_interval: 5000,
            max_reconnect_attempts: 5,
            auto_reconnect: true,
            enable_compression: false,
            pong_timeout: 90000, // Default 90-second timeout
        };

        // Initialize a new client instance (retained for future replacement logic)
        let _new_client = Arc::new(WsClient::new(config));
        // An `Arc` cannot be modified in place; concrete handling is deferred for future work

        // Connect to the WebSocket endpoint
        self.client.connect().await?;

        // Start automatic listen key refresh
        manager.start_auto_refresh().await;

        // Cache the current listen key locally
        *self.listen_key.write().await = Some(listen_key);

        Ok(())
    }

    /// Closes the user data stream
    ///
    /// Stops auto-refresh and deletes the listen key
    ///
    /// # Returns
    /// Result of the shutdown
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
    ///
    /// # Arguments
    /// * `symbol` - Trading pair (e.g. "btcusdt")
    ///
    /// # Returns
    /// Result of the subscription request
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
    ///
    /// # Arguments
    /// * `symbol` - Trading pair (e.g. "btcusdt")
    ///
    /// # Returns
    /// Result of the subscription request
    pub async fn subscribe_trades(&self, symbol: &str) -> Result<()> {
        let stream = format!("{}@trade", symbol.to_lowercase());
        self.client
            .subscribe(stream, Some(symbol.to_string()), None)
            .await
    }

    /// Subscribes to the aggregated trade stream for a symbol
    ///
    /// # Arguments
    /// * `symbol` - Trading pair (e.g. "btcusdt")
    ///
    /// # Returns
    /// Result of the subscription request
    pub async fn subscribe_agg_trades(&self, symbol: &str) -> Result<()> {
        let stream = format!("{}@aggTrade", symbol.to_lowercase());
        self.client
            .subscribe(stream, Some(symbol.to_string()), None)
            .await
    }

    /// Subscribes to the order book depth stream
    ///
    /// # Arguments
    /// * `symbol` - Trading pair (e.g. "btcusdt")
    /// * `levels` - Depth levels (5, 10, 20)
    /// * `update_speed` - Update frequency ("100ms" or "1000ms")
    ///
    /// # Returns
    /// Result of the subscription request
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
    ///
    /// # Arguments
    /// * `symbol` - Trading pair (e.g. "btcusdt")
    /// * `update_speed` - Update frequency ("100ms" or "1000ms")
    ///
    /// # Returns
    /// Result of the subscription request
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
    ///
    /// # Arguments
    /// * `symbol` - Trading pair (e.g. "btcusdt")
    /// * `interval` - Kline interval (1m, 3m, 5m, 15m, 30m, 1h, 2h, 4h, 6h, 8h, 12h, 1d, 3d, 1w, 1M)
    ///
    /// # Returns
    /// Result of the subscription request
    pub async fn subscribe_kline(&self, symbol: &str, interval: &str) -> Result<()> {
        let stream = format!("{}@kline_{}", symbol.to_lowercase(), interval);
        self.client
            .subscribe(stream, Some(symbol.to_string()), None)
            .await
    }

    /// Subscribes to the mini ticker stream for a symbol
    ///
    /// # Arguments
    /// * `symbol` - Trading pair (e.g. "btcusdt")
    ///
    /// # Returns
    /// Result of the subscription request
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
    ///
    /// # Arguments
    /// * `stream` - Stream identifier to unsubscribe from
    ///
    /// # Returns
    /// Result of the unsubscribe request
    pub async fn unsubscribe(&self, stream: String) -> Result<()> {
        self.client.unsubscribe(stream, None).await
    }

    /// Receives the next available message
    ///
    /// # Returns
    /// Optional message payload when available
    pub async fn receive(&self) -> Option<Value> {
        self.client.receive().await
    }

    /// Indicates whether the WebSocket connection is active
    pub async fn is_connected(&self) -> bool {
        self.client.is_connected().await
    }

    /// Watches a single ticker stream (internal helper)
    ///
    /// # Arguments
    /// * `symbol` - Lowercase trading pair (e.g. "btcusdt")
    /// * `channel_name` - Channel identifier (ticker/miniTicker/markPrice/bookTicker)
    ///
    /// # Returns
    /// Parsed ticker data
    async fn watch_ticker_internal(&self, symbol: &str, channel_name: &str) -> Result<Ticker> {
        let stream = format!("{}@{}", symbol.to_lowercase(), channel_name);

        // Subscribe to the requested stream
        self.client
            .subscribe(stream.clone(), Some(symbol.to_string()), None)
            .await?;

        // Wait for and parse incoming messages
        loop {
            if let Some(message) = self.client.receive().await {
                // Ignore subscription acknowledgements
                if message.get("result").is_some() {
                    continue;
                }

                // Parse ticker payload
                if let Ok(ticker) = parser::parse_ws_ticker(&message, None) {
                    // Cache the ticker for later retrieval
                    let mut tickers = self.tickers.lock().await;
                    tickers.insert(ticker.symbol.clone(), ticker.clone());

                    return Ok(ticker);
                }
            }
        }
    }

    /// Watches multiple ticker streams (internal helper)
    ///
    /// # Arguments
    /// * `symbols` - Optional list of lowercase trading pairs
    /// * `channel_name` - Target channel name
    ///
    /// # Returns
    /// Mapping of symbol to ticker payloads
    async fn watch_tickers_internal(
        &self,
        symbols: Option<Vec<String>>,
        channel_name: &str,
    ) -> Result<HashMap<String, Ticker>> {
        let streams: Vec<String> = if let Some(syms) = symbols.as_ref() {
            // Subscribe to specific trading pairs
            syms.iter()
                .map(|s| format!("{}@{}", s.to_lowercase(), channel_name))
                .collect()
        } else {
            // Subscribe to the aggregated ticker stream
            vec![format!("!{}@arr", channel_name)]
        };

        // Issue subscription requests
        for stream in &streams {
            self.client.subscribe(stream.clone(), None, None).await?;
        }

        // Collect and parse messages
        let mut result = HashMap::new();

        loop {
            if let Some(message) = self.client.receive().await {
                // Skip subscription acknowledgements
                if message.get("result").is_some() {
                    continue;
                }

                // Handle array payloads (all tickers)
                if let Some(arr) = message.as_array() {
                    for item in arr {
                        if let Ok(ticker) = parser::parse_ws_ticker(item, None) {
                            let symbol = ticker.symbol.clone();

                            // Return only requested symbols when provided
                            if let Some(syms) = &symbols {
                                if syms.contains(&symbol.to_lowercase()) {
                                    result.insert(symbol.clone(), ticker.clone());
                                }
                            } else {
                                result.insert(symbol.clone(), ticker.clone());
                            }

                            // Cache the ticker payload
                            let mut tickers = self.tickers.lock().await;
                            tickers.insert(symbol, ticker);
                        }
                    }

                    // Exit once all requested tickers have been observed
                    if let Some(syms) = &symbols {
                        if result.len() == syms.len() {
                            return Ok(result);
                        }
                    } else {
                        return Ok(result);
                    }
                } else if let Ok(ticker) = parser::parse_ws_ticker(&message, None) {
                    // Handle single-ticker payloads
                    let symbol = ticker.symbol.clone();
                    result.insert(symbol.clone(), ticker.clone());

                    // Cache the ticker payload
                    let mut tickers = self.tickers.lock().await;
                    tickers.insert(symbol, ticker);

                    // Exit once all requested tickers have been observed
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
    ///
    /// # Arguments
    /// * `symbol` - Trading pair
    /// * `delta_message` - Raw WebSocket delta payload
    /// * `is_futures` - Whether the feed originates from the futures market
    ///
    /// # Returns
    /// Result of the update; returns a special error when resynchronization is required
    async fn handle_orderbook_delta(
        &self,
        symbol: &str,
        delta_message: &Value,
        is_futures: bool,
    ) -> Result<()> {
        use ccxt_core::types::orderbook::{OrderBookDelta, OrderBookEntry};
        use rust_decimal::Decimal;

        // Parse the delta message
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

        // Parse bid updates
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

        // Parse ask updates
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

        // Build the delta structure
        let delta = OrderBookDelta {
            symbol: symbol.to_string(),
            first_update_id,
            final_update_id,
            prev_final_update_id,
            timestamp,
            bids,
            asks,
        };

        // Retrieve or create the cached order book
        let mut orderbooks = self.orderbooks.lock().await;
        let orderbook = orderbooks
            .entry(symbol.to_string())
            .or_insert_with(|| OrderBook::new(symbol.to_string(), timestamp));

        // If the order book is not synchronized yet, buffer the delta
        if !orderbook.is_synced {
            orderbook.buffer_delta(delta);
            return Ok(());
        }

        // Apply the delta to the order book
        if let Err(e) = orderbook.apply_delta(&delta, is_futures) {
            // Check whether a resynchronization cycle is needed
            if orderbook.needs_resync {
                tracing::warn!("Orderbook {} needs resync due to: {}", symbol, e);
                // Buffer the delta so it can be reused after resync
                orderbook.buffer_delta(delta);
                // Signal that resynchronization is required
                return Err(Error::invalid_request(format!("RESYNC_NEEDED: {}", e)));
            }
            return Err(Error::invalid_request(e));
        }

        Ok(())
    }

    /// Retrieves an order book snapshot and initializes cached state (internal helper)
    ///
    /// # Arguments
    /// * `exchange` - Exchange reference used for REST API calls
    /// * `symbol` - Trading pair identifier
    /// * `limit` - Depth limit to request
    /// * `is_futures` - Whether the symbol is a futures market
    ///
    /// # Returns
    /// Initialized order book structure
    async fn fetch_orderbook_snapshot(
        &self,
        exchange: &Binance,
        symbol: &str,
        limit: Option<i64>,
        is_futures: bool,
    ) -> Result<OrderBook> {
        // Fetch snapshot via REST API
        let mut params = HashMap::new();
        if let Some(l) = limit {
            // json! macro with simple values is infallible
            #[allow(clippy::disallowed_methods)]
            let limit_value = serde_json::json!(l);
            params.insert("limit".to_string(), limit_value);
        }

        let mut snapshot = exchange.fetch_order_book(symbol, None).await?;

        // Mark the snapshot as synchronized
        snapshot.is_synced = true;

        // Apply buffered deltas captured before the snapshot
        let mut orderbooks = self.orderbooks.lock().await;
        if let Some(cached_ob) = orderbooks.get_mut(symbol) {
            // Transfer buffered deltas to the snapshot instance
            snapshot.buffered_deltas = cached_ob.buffered_deltas.clone();

            // Apply buffered deltas to catch up
            if let Ok(processed) = snapshot.process_buffered_deltas(is_futures) {
                tracing::debug!("Processed {} buffered deltas for {}", processed, symbol);
            }
        }

        // Update cache with the synchronized snapshot
        orderbooks.insert(symbol.to_string(), snapshot.clone());

        Ok(snapshot)
    }

    /// Watches a single order book stream (internal helper)
    ///
    /// # Arguments
    /// * `exchange` - Exchange reference
    /// * `symbol` - Lowercase trading pair
    /// * `limit` - Depth limit
    /// * `update_speed` - Update frequency (100 or 1000 ms)
    /// * `is_futures` - Whether the symbol is a futures market
    ///
    /// # Returns
    /// Order book snapshot enriched with streamed updates
    async fn watch_orderbook_internal(
        &self,
        exchange: &Binance,
        symbol: &str,
        limit: Option<i64>,
        update_speed: i32,
        is_futures: bool,
    ) -> Result<OrderBook> {
        // Construct the stream name
        let stream = if update_speed == 100 {
            format!("{}@depth@100ms", symbol.to_lowercase())
        } else {
            format!("{}@depth", symbol.to_lowercase())
        };

        // Subscribe to depth updates
        self.client
            .subscribe(stream.clone(), Some(symbol.to_string()), None)
            .await?;

        // Start buffering deltas before the snapshot is ready
        let snapshot_fetched = Arc::new(Mutex::new(false));
        let _snapshot_fetched_clone = snapshot_fetched.clone();

        // Spawn a placeholder processing loop (actual handling occurs elsewhere)
        let _orderbooks_clone = self.orderbooks.clone();
        let _symbol_clone = symbol.to_string();

        tokio::spawn(async move {
            // Placeholder: actual message handling occurs in the main loop
        });

        // Give the stream time to accumulate initial deltas before fetching a snapshot
        tokio::time::sleep(Duration::from_millis(500)).await;

        // Fetch the initial snapshot
        let _snapshot = self
            .fetch_orderbook_snapshot(exchange, symbol, limit, is_futures)
            .await?;

        *snapshot_fetched.lock().await = true;

        // Main processing loop
        loop {
            if let Some(message) = self.client.receive().await {
                // Skip subscription acknowledgements
                if message.get("result").is_some() {
                    continue;
                }

                // Process depth updates only
                if let Some(event_type) = message.get("e").and_then(|v| v.as_str()) {
                    if event_type == "depthUpdate" {
                        match self
                            .handle_orderbook_delta(symbol, &message, is_futures)
                            .await
                        {
                            Ok(_) => {
                                // Return the updated order book once synchronized
                                let orderbooks = self.orderbooks.lock().await;
                                if let Some(ob) = orderbooks.get(symbol) {
                                    if ob.is_synced {
                                        return Ok(ob.clone());
                                    }
                                }
                            }
                            Err(e) => {
                                let err_msg = e.to_string();

                                // Initiate resynchronization when instructed
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

                                        // Reset local state in preparation for resync
                                        {
                                            let mut orderbooks = self.orderbooks.lock().await;
                                            if let Some(ob) = orderbooks.get_mut(symbol) {
                                                ob.reset_for_resync();
                                                ob.mark_resync_initiated(current_time);
                                            }
                                        }

                                        // Allow some deltas to buffer before fetching a new snapshot
                                        tokio::time::sleep(Duration::from_millis(500)).await;

                                        // Fetch a fresh snapshot and continue processing
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
                                    } else {
                                        tracing::debug!(
                                            "Resync rate limited for {}, skipping",
                                            symbol
                                        );
                                        continue;
                                    }
                                } else {
                                    tracing::error!(
                                        "Failed to handle orderbook delta: {}",
                                        err_msg
                                    );
                                    continue;
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    /// Watches multiple order book streams (internal helper)
    ///
    /// # Arguments
    /// * `exchange` - Exchange reference
    /// * `symbols` - List of trading pairs
    /// * `limit` - Requested depth level
    /// * `update_speed` - Update frequency
    /// * `is_futures` - Whether the symbols are futures markets
    ///
    /// # Returns
    /// Mapping of symbol to order book data
    async fn watch_orderbooks_internal(
        &self,
        exchange: &Binance,
        symbols: Vec<String>,
        limit: Option<i64>,
        update_speed: i32,
        is_futures: bool,
    ) -> Result<HashMap<String, OrderBook>> {
        // Binance enforces a 200-symbol limit per connection
        if symbols.len() > 200 {
            return Err(Error::invalid_request(
                "Binance supports max 200 symbols per connection",
            ));
        }

        // Subscribe to each symbol
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

        // Allow messages to accumulate before snapshot retrieval
        tokio::time::sleep(Duration::from_millis(500)).await;

        // Fetch snapshots for all symbols
        for symbol in &symbols {
            let _ = self
                .fetch_orderbook_snapshot(exchange, symbol, limit, is_futures)
                .await;
        }

        // Process incremental updates
        let mut result = HashMap::new();
        let mut update_count = 0;

        while update_count < symbols.len() {
            if let Some(message) = self.client.receive().await {
                // Skip subscription acknowledgements
                if message.get("result").is_some() {
                    continue;
                }

                // Handle depth updates
                if let Some(event_type) = message.get("e").and_then(|v| v.as_str()) {
                    if event_type == "depthUpdate" {
                        if let Some(msg_symbol) = message.get("s").and_then(|v| v.as_str()) {
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

        // Collect the resulting order books
        let orderbooks = self.orderbooks.lock().await;
        for symbol in &symbols {
            if let Some(ob) = orderbooks.get(symbol) {
                result.insert(symbol.clone(), ob.clone());
            }
        }

        Ok(result)
    }

    ///
    /// # Arguments
    /// * `symbol` - Trading pair identifier
    ///
    /// # Returns
    /// Ticker snapshot when available
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
    ///
    /// # Arguments
    /// * `message` - WebSocket payload
    /// * `account_type` - Account category (spot/future/delivery, etc.)
    ///
    /// # Returns
    /// Result of the balance update processing
    async fn handle_balance_message(&self, message: &Value, account_type: &str) -> Result<()> {
        use rust_decimal::Decimal;
        use std::str::FromStr;

        // Identify the event type
        let event_type = message
            .get("e")
            .and_then(|e| e.as_str())
            .ok_or_else(|| Error::invalid_request("Missing event type in balance message"))?;

        // Retrieve or create cached balances for the account
        let mut balances = self.balances.write().await;
        let balance = balances
            .entry(account_type.to_string())
            .or_insert_with(Balance::new);

        match event_type {
            // Spot account incremental balance update
            "balanceUpdate" => {
                let asset = message
                    .get("a")
                    .and_then(|a| a.as_str())
                    .ok_or_else(|| Error::invalid_request("Missing asset in balanceUpdate"))?;

                let delta_str = message
                    .get("d")
                    .and_then(|d| d.as_str())
                    .ok_or_else(|| Error::invalid_request("Missing delta in balanceUpdate"))?;

                let delta = Decimal::from_str(delta_str)
                    .map_err(|e| Error::invalid_request(format!("Invalid delta value: {}", e)))?;

                // Apply delta to the cached balance
                balance.apply_delta(asset.to_string(), delta);
            }

            // Spot account full balance update
            "outboundAccountPosition" => {
                if let Some(balances_array) = message.get("B").and_then(|b| b.as_array()) {
                    for balance_item in balances_array {
                        let asset =
                            balance_item
                                .get("a")
                                .and_then(|a| a.as_str())
                                .ok_or_else(|| {
                                    Error::invalid_request("Missing asset in balance item")
                                })?;

                        let free_str = balance_item
                            .get("f")
                            .and_then(|f| f.as_str())
                            .ok_or_else(|| Error::invalid_request("Missing free balance"))?;

                        let locked_str = balance_item
                            .get("l")
                            .and_then(|l| l.as_str())
                            .ok_or_else(|| Error::invalid_request("Missing locked balance"))?;

                        let free = Decimal::from_str(free_str).map_err(|e| {
                            Error::invalid_request(format!("Invalid free value: {}", e))
                        })?;

                        let locked = Decimal::from_str(locked_str).map_err(|e| {
                            Error::invalid_request(format!("Invalid locked value: {}", e))
                        })?;

                        // Update the cached balance snapshot
                        balance.update_balance(asset.to_string(), free, locked);
                    }
                }
            }

            // Futures/delivery account updates
            "ACCOUNT_UPDATE" => {
                if let Some(account_data) = message.get("a") {
                    // Parse balance array
                    if let Some(balances_array) = account_data.get("B").and_then(|b| b.as_array()) {
                        for balance_item in balances_array {
                            let asset = balance_item.get("a").and_then(|a| a.as_str()).ok_or_else(
                                || Error::invalid_request("Missing asset in balance item"),
                            )?;

                            let wallet_balance_str = balance_item
                                .get("wb")
                                .and_then(|wb| wb.as_str())
                                .ok_or_else(|| Error::invalid_request("Missing wallet balance"))?;

                            let wallet_balance =
                                Decimal::from_str(wallet_balance_str).map_err(|e| {
                                    Error::invalid_request(format!("Invalid wallet balance: {}", e))
                                })?;

                            // Optional cross wallet balance
                            let cross_wallet = balance_item
                                .get("cw")
                                .and_then(|cw| cw.as_str())
                                .and_then(|s| Decimal::from_str(s).ok());

                            // Update wallet balance snapshot
                            balance.update_wallet(asset.to_string(), wallet_balance, cross_wallet);
                        }
                    }

                    // Positions are contained in account_data["P"]; handling occurs in watch_positions
                }
            }

            _ => {
                return Err(Error::invalid_request(format!(
                    "Unknown balance event type: {}",
                    event_type
                )));
            }
        }

        Ok(())
    }

    /// Parses a WebSocket trade message
    ///
    /// Extracts trade information from an `executionReport` event
    ///
    /// # Arguments
    /// * `data` - Raw WebSocket JSON payload
    ///
    /// # Returns
    /// Parsed `Trade` structure
    fn parse_ws_trade(&self, data: &Value) -> Result<Trade> {
        use ccxt_core::types::{Fee, OrderSide, OrderType, TakerOrMaker};
        use rust_decimal::Decimal;
        use std::str::FromStr;

        // Extract symbol field
        let symbol = data
            .get("s")
            .and_then(|v| v.as_str())
            .ok_or_else(|| Error::invalid_request("Missing symbol field".to_string()))?
            .to_string();

        // Trade ID (field `t`)
        let id = data
            .get("t")
            .and_then(|v| v.as_i64())
            .map(|v| v.to_string());

        // Trade timestamp (field `T`)
        let timestamp = data.get("T").and_then(|v| v.as_i64()).unwrap_or(0);

        // Executed price (field `L` - last executed price)
        let price = data
            .get("L")
            .and_then(|v| v.as_str())
            .and_then(|s| Decimal::from_str(s).ok())
            .unwrap_or(Decimal::ZERO);

        // Executed amount (field `l` - last executed quantity)
        let amount = data
            .get("l")
            .and_then(|v| v.as_str())
            .and_then(|s| Decimal::from_str(s).ok())
            .unwrap_or(Decimal::ZERO);

        // Quote asset amount (field `Y` - last quote asset transacted quantity)
        let cost = data
            .get("Y")
            .and_then(|v| v.as_str())
            .and_then(|s| Decimal::from_str(s).ok())
            .or_else(|| {
                // Fallback: compute from price * amount when `Y` is unavailable
                if price > Decimal::ZERO && amount > Decimal::ZERO {
                    Some(price * amount)
                } else {
                    None
                }
            });

        // Trade side (field `S`)
        let side = data
            .get("S")
            .and_then(|v| v.as_str())
            .and_then(|s| match s.to_uppercase().as_str() {
                "BUY" => Some(OrderSide::Buy),
                "SELL" => Some(OrderSide::Sell),
                _ => None,
            })
            .unwrap_or(OrderSide::Buy);

        // Order type (field `o`)
        let trade_type =
            data.get("o")
                .and_then(|v| v.as_str())
                .and_then(|s| match s.to_uppercase().as_str() {
                    "LIMIT" => Some(OrderType::Limit),
                    "MARKET" => Some(OrderType::Market),
                    _ => None,
                });

        // Associated order ID (field `i`)
        let order_id = data
            .get("i")
            .and_then(|v| v.as_i64())
            .map(|v| v.to_string());

        // Maker/taker flag (field `m` - true when buyer is the maker)
        let taker_or_maker = data.get("m").and_then(|v| v.as_bool()).map(|is_maker| {
            if is_maker {
                TakerOrMaker::Maker
            } else {
                TakerOrMaker::Taker
            }
        });

        // Fee information (fields `n` = fee amount, `N` = fee currency)
        let fee = if let Some(fee_cost_str) = data.get("n").and_then(|v| v.as_str()) {
            if let Ok(fee_cost) = Decimal::from_str(fee_cost_str) {
                let currency = data
                    .get("N")
                    .and_then(|v| v.as_str())
                    .unwrap_or("UNKNOWN")
                    .to_string();
                Some(Fee {
                    currency,
                    cost: fee_cost,
                    rate: None,
                })
            } else {
                None
            }
        } else {
            None
        };

        // Derive ISO8601 timestamp string when possible
        let datetime = chrono::DateTime::from_timestamp_millis(timestamp)
            .map(|dt| dt.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string());

        // Preserve the raw payload in the `info` map
        let mut info = HashMap::new();
        if let Value::Object(map) = data {
            for (k, v) in map.iter() {
                info.insert(k.clone(), v.clone());
            }
        }

        Ok(Trade {
            id,
            order: order_id,
            symbol,
            trade_type,
            side,
            taker_or_maker,
            price: Price::from(price),
            amount: Amount::from(amount),
            cost: cost.map(Cost::from),
            fee,
            timestamp,
            datetime,
            info,
        })
    }

    /// Filters cached personal trades by symbol, time range, and limit
    ///
    /// # Arguments
    /// * `symbol` - Optional symbol filter
    /// * `since` - Optional starting timestamp (inclusive)
    /// * `limit` - Optional maximum number of trades to return
    async fn filter_my_trades(
        &self,
        symbol: Option<&str>,
        since: Option<i64>,
        limit: Option<usize>,
    ) -> Result<Vec<Trade>> {
        let trades_map = self.my_trades.read().await;

        // Filter by symbol when provided
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

        // Apply `since` filter when provided
        if let Some(since_ts) = since {
            trades.retain(|trade| trade.timestamp >= since_ts);
        }

        // Sort by timestamp descending (latest first)
        trades.sort_by(|a, b| {
            let ts_a = a.timestamp;
            let ts_b = b.timestamp;
            ts_b.cmp(&ts_a)
        });

        // Apply optional limit
        if let Some(lim) = limit {
            trades.truncate(lim);
        }

        Ok(trades)
    }

    /// Parses a WebSocket position payload
    ///
    /// # Arguments
    /// * `data` - Position data from the ACCOUNT_UPDATE event (`P` array element)
    ///
    /// # Returns
    /// Parsed `Position` instance
    ///
    /// # Binance WebSocket Position Payload Example
    /// ```json
    /// {
    ///   "s": "BTCUSDT",           // Trading pair
    ///   "pa": "-0.089",           // Position amount (negative indicates short)
    ///   "ep": "19700.03933",      // Entry price
    ///   "cr": "-1260.24809979",   // Accumulated realized PnL
    ///   "up": "1.53058860",       // Unrealized PnL
    ///   "mt": "isolated",         // Margin mode: isolated/cross
    ///   "iw": "87.13658940",      // Isolated wallet balance
    ///   "ps": "BOTH",             // Position side: BOTH/LONG/SHORT
    ///   "ma": "USDT"              // Margin asset
    /// }
    /// ```
    async fn parse_ws_position(&self, data: &Value) -> Result<Position> {
        // Extract required fields
        let symbol = data["s"]
            .as_str()
            .ok_or_else(|| Error::invalid_request("Missing symbol field"))?
            .to_string();

        let position_amount_str = data["pa"]
            .as_str()
            .ok_or_else(|| Error::invalid_request("Missing position amount"))?;

        let position_amount = position_amount_str
            .parse::<f64>()
            .map_err(|e| Error::invalid_request(format!("Invalid position amount: {}", e)))?;

        // Extract position side
        let position_side = data["ps"]
            .as_str()
            .ok_or_else(|| Error::invalid_request("Missing position side"))?
            .to_uppercase();

        // Determine hedged mode and actual side
        // - If ps = BOTH, hedged = false and use sign of `pa` for actual side
        // - If ps = LONG/SHORT, hedged = true and side equals ps
        let (side, hedged) = if position_side == "BOTH" {
            let actual_side = if position_amount < 0.0 {
                "short"
            } else {
                "long"
            };
            (actual_side.to_string(), false)
        } else {
            (position_side.to_lowercase(), true)
        };

        // Extract additional fields
        let entry_price = data["ep"].as_str().and_then(|s| s.parse::<f64>().ok());
        let unrealized_pnl = data["up"].as_str().and_then(|s| s.parse::<f64>().ok());
        let realized_pnl = data["cr"].as_str().and_then(|s| s.parse::<f64>().ok());
        let margin_mode = data["mt"].as_str().map(|s| s.to_string());
        let initial_margin = data["iw"].as_str().and_then(|s| s.parse::<f64>().ok());
        let _margin_asset = data["ma"].as_str().map(|s| s.to_string());

        // Construct the `Position` object
        Ok(Position {
            info: data.clone(),
            id: None,
            symbol,
            side: Some(side),
            contracts: Some(position_amount.abs()), // Absolute contract amount
            contract_size: None,
            entry_price,
            mark_price: None,
            notional: None,
            leverage: None,
            collateral: initial_margin, // Use isolated wallet balance as collateral
            initial_margin,
            initial_margin_percentage: None,
            maintenance_margin: None,
            maintenance_margin_percentage: None,
            unrealized_pnl,
            realized_pnl,
            liquidation_price: None,
            margin_ratio: None,
            margin_mode,
            hedged: Some(hedged),
            percentage: None,
            position_side: None,
            dual_side_position: None,
            timestamp: Some(chrono::Utc::now().timestamp_millis() as u64),
            datetime: Some(chrono::Utc::now().to_rfc3339()),
        })
    }

    /// Filters cached positions by symbol, time range, and limit
    ///
    /// # Arguments
    /// * `symbols` - Optional list of symbols to include
    /// * `since` - Optional starting timestamp (inclusive)
    /// * `limit` - Optional maximum number of positions to return
    ///
    /// # Returns
    /// Filtered list of positions
    async fn filter_positions(
        &self,
        symbols: Option<&[String]>,
        since: Option<i64>,
        limit: Option<usize>,
    ) -> Result<Vec<Position>> {
        let positions_map = self.positions.read().await;

        // Filter by symbol list when provided
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

        // Apply `since` filter when provided
        if let Some(since_ts) = since {
            positions.retain(|pos| {
                pos.timestamp
                    .map(|ts| ts as i64 >= since_ts)
                    .unwrap_or(false)
            });
        }

        // Sort by timestamp descending (latest first)
        positions.sort_by(|a, b| {
            let ts_a = a.timestamp.unwrap_or(0);
            let ts_b = b.timestamp.unwrap_or(0);
            ts_b.cmp(&ts_a)
        });

        // Apply optional limit
        if let Some(lim) = limit {
            positions.truncate(lim);
        }

        Ok(positions)
    }
}

impl Binance {
    /// Subscribes to the ticker stream for a unified symbol
    ///
    /// # Arguments
    /// * `symbol` - Unified trading pair identifier
    ///
    /// # Returns
    /// Result of the subscription call
    pub async fn subscribe_ticker(&self, symbol: &str) -> Result<()> {
        let ws = self.create_ws();
        ws.connect().await?;

        // Convert symbol format BTC/USDT -> btcusdt
        let binance_symbol = symbol.replace('/', "").to_lowercase();
        ws.subscribe_ticker(&binance_symbol).await
    }

    /// Subscribes to the trade stream for a unified symbol
    ///
    /// # Arguments
    /// * `symbol` - Unified trading pair identifier
    ///
    /// # Returns
    /// Result of the subscription call
    pub async fn subscribe_trades(&self, symbol: &str) -> Result<()> {
        let ws = self.create_ws();
        ws.connect().await?;

        let binance_symbol = symbol.replace('/', "").to_lowercase();
        ws.subscribe_trades(&binance_symbol).await
    }

    /// Subscribes to the order book stream for a unified symbol
    ///
    /// # Arguments
    /// * `symbol` - Unified trading pair identifier
    /// * `levels` - Optional depth limit (default 20)
    ///
    /// # Returns
    /// Result of the subscription call
    pub async fn subscribe_orderbook(&self, symbol: &str, levels: Option<u32>) -> Result<()> {
        let ws = self.create_ws();
        ws.connect().await?;

        let binance_symbol = symbol.replace('/', "").to_lowercase();
        let depth_levels = levels.unwrap_or(20);
        ws.subscribe_orderbook(&binance_symbol, depth_levels, "1000ms")
            .await
    }

    /// Subscribes to the candlestick stream for a unified symbol
    ///
    /// # Arguments
    /// * `symbol` - Unified trading pair identifier
    /// * `interval` - Candlestick interval identifier
    ///
    /// # Returns
    /// Result of the subscription call
    pub async fn subscribe_kline(&self, symbol: &str, interval: &str) -> Result<()> {
        let ws = self.create_ws();
        ws.connect().await?;

        let binance_symbol = symbol.replace('/', "").to_lowercase();
        ws.subscribe_kline(&binance_symbol, interval).await
    }

    /// Watches a ticker stream for a single unified symbol
    ///
    /// # Arguments
    /// * `symbol` - Unified trading pair identifier (e.g. "BTC/USDT")
    /// * `params` - Optional parameters
    ///   - `name`: Channel name (ticker/miniTicker, defaults to ticker)
    ///
    /// # Returns
    /// Parsed ticker structure
    ///
    /// # Example
    /// ```rust,no_run
    /// # use ccxt_exchanges::binance::Binance;
    /// # use ccxt_core::ExchangeConfig;
    /// # use ccxt_core::types::Price;
    /// # async fn example() -> ccxt_core::error::Result<()> {
    /// let exchange = Binance::new(ExchangeConfig::default())?;
    /// let ticker = exchange.watch_ticker("BTC/USDT", None).await?;
    /// println!("Price: {}", ticker.last.unwrap_or(Price::ZERO));
    /// # Ok(())
    /// # }
    /// ```
    pub async fn watch_ticker(
        &self,
        symbol: &str,
        params: Option<HashMap<String, Value>>,
    ) -> Result<Ticker> {
        // Load market metadata
        self.load_markets(false).await?;

        // Convert unified symbol to exchange format
        let market = self.base.market(symbol).await?;
        let binance_symbol = market.id.to_lowercase();

        // Select channel name
        let channel_name = if let Some(p) = &params {
            p.get("name").and_then(|v| v.as_str()).unwrap_or("ticker")
        } else {
            "ticker"
        };

        // Establish WebSocket connection
        let ws = self.create_ws();
        ws.connect().await?;

        // Watch ticker
        ws.watch_ticker_internal(&binance_symbol, channel_name)
            .await
    }

    /// Watches ticker streams for multiple unified symbols
    ///
    /// # Arguments
    /// * `symbols` - Optional list of unified trading pairs (None subscribes to all)
    /// * `params` - Optional parameters
    ///   - `name`: Channel name (ticker/miniTicker, defaults to ticker)
    ///
    /// # Returns
    /// Mapping of symbol to ticker data
    ///
    /// # Example
    /// ```rust,no_run
    /// # use ccxt_exchanges::binance::Binance;
    /// # use ccxt_core::ExchangeConfig;
    /// # async fn example() -> ccxt_core::error::Result<()> {
    /// let exchange = Binance::new(ExchangeConfig::default())?;
    ///
    /// // Watch a subset of symbols
    /// let tickers = exchange.watch_tickers(
    ///     Some(vec!["BTC/USDT".to_string(), "ETH/USDT".to_string()]),
    ///     None
    /// ).await?;
    ///
    /// // Watch all symbols
    /// let all_tickers = exchange.watch_tickers(None, None).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn watch_tickers(
        &self,
        symbols: Option<Vec<String>>,
        params: Option<HashMap<String, Value>>,
    ) -> Result<HashMap<String, Ticker>> {
        // Load market metadata
        self.load_markets(false).await?;

        // Determine channel name
        let channel_name = if let Some(p) = &params {
            p.get("name").and_then(|v| v.as_str()).unwrap_or("ticker")
        } else {
            "ticker"
        };

        // Validate channel selection
        if channel_name == "bookTicker" {
            return Err(Error::invalid_request(
                "To subscribe for bids-asks, use watch_bids_asks() method instead",
            ));
        }

        // Convert unified symbols to exchange format
        let binance_symbols = if let Some(syms) = symbols {
            let mut result = Vec::new();
            for symbol in syms {
                let market = self.base.market(&symbol).await?;
                result.push(market.id.to_lowercase());
            }
            Some(result)
        } else {
            None
        };

        // Establish WebSocket connection
        let ws = self.create_ws();
        ws.connect().await?;

        // Watch tickers
        ws.watch_tickers_internal(binance_symbols, channel_name)
            .await
    }

    /// Watches the mark price stream for a futures market
    ///
    /// # Arguments
    /// * `symbol` - Unified trading pair identifier (e.g. "BTC/USDT:USDT")
    /// * `params` - Optional parameters
    ///   - `use1sFreq`: Whether to use 1-second updates (defaults to true)
    ///
    /// # Returns
    /// Ticker structure representing the mark price
    ///
    /// # Example
    /// ```rust,no_run
    /// # use ccxt_exchanges::binance::Binance;
    /// # use ccxt_core::ExchangeConfig;
    /// # use std::collections::HashMap;
    /// # use serde_json::json;
    /// # async fn example() -> ccxt_core::error::Result<()> {
    /// let exchange = Binance::new(ExchangeConfig::default())?;
    ///
    /// // Use 1-second updates
    /// let ticker = exchange.watch_mark_price("BTC/USDT:USDT", None).await?;
    ///
    /// // Use 3-second updates
    /// let mut params = HashMap::new();
    /// params.insert("use1sFreq".to_string(), json!(false));
    /// let ticker = exchange.watch_mark_price("BTC/USDT:USDT", Some(params)).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn watch_mark_price(
        &self,
        symbol: &str,
        params: Option<HashMap<String, Value>>,
    ) -> Result<Ticker> {
        // Load market metadata
        self.load_markets(false).await?;

        // Ensure the symbol belongs to a futures market
        let market = self.base.market(symbol).await?;
        if market.market_type != MarketType::Swap && market.market_type != MarketType::Futures {
            return Err(Error::invalid_request(format!(
                "watch_mark_price() does not support {} markets",
                market.market_type
            )));
        }

        let binance_symbol = market.id.to_lowercase();

        // Determine update frequency
        let use_1s_freq = if let Some(p) = &params {
            p.get("use1sFreq").and_then(|v| v.as_bool()).unwrap_or(true)
        } else {
            true
        };

        // Construct channel name
        let channel_name = if use_1s_freq {
            "markPrice@1s"
        } else {
            "markPrice"
        };

        // Establish WebSocket connection
        let ws = self.create_ws();
        ws.connect().await?;

        // Watch mark price
        ws.watch_ticker_internal(&binance_symbol, channel_name)
            .await
    }

    /// Watches an order book stream for a unified symbol
    ///
    /// # Arguments
    /// * `symbol` - Unified trading pair identifier (e.g. "BTC/USDT")
    /// * `limit` - Optional depth limit (defaults to unlimited)
    /// * `params` - Optional parameters
    ///   - `speed`: Update frequency (100 or 1000 ms, defaults to 100)
    ///
    /// # Returns
    /// Order book snapshot populated with streaming updates
    ///
    /// # Example
    /// ```rust,no_run
    /// # use ccxt_exchanges::binance::Binance;
    /// # use ccxt_core::ExchangeConfig;
    /// # use std::collections::HashMap;
    /// # use serde_json::json;
    /// # async fn example() -> ccxt_core::error::Result<()> {
    /// let exchange = Binance::new(ExchangeConfig::default())?;
    ///
    /// // Watch order book with 100 ms updates
    /// let orderbook = exchange.watch_order_book("BTC/USDT", None, None).await?;
    /// println!("Best bid: {:?}", orderbook.best_bid());
    /// println!("Best ask: {:?}", orderbook.best_ask());
    ///
    /// // Watch order book limited to 100 levels with 1000 ms updates
    /// let mut params = HashMap::new();
    /// params.insert("speed".to_string(), json!(1000));
    /// let orderbook = exchange.watch_order_book(
    ///     "BTC/USDT",
    ///     Some(100),
    ///     Some(params)
    /// ).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn watch_order_book(
        &self,
        symbol: &str,
        limit: Option<i64>,
        params: Option<HashMap<String, Value>>,
    ) -> Result<OrderBook> {
        // Load market metadata
        self.load_markets(false).await?;

        // Resolve market details
        let market = self.base.market(symbol).await?;
        let binance_symbol = market.id.to_lowercase();

        // Determine whether this is a futures market
        let is_futures =
            market.market_type == MarketType::Swap || market.market_type == MarketType::Futures;

        // Determine update speed
        let update_speed = if let Some(p) = &params {
            p.get("speed").and_then(|v| v.as_i64()).unwrap_or(100) as i32
        } else {
            100
        };

        // Validate update speed
        if update_speed != 100 && update_speed != 1000 {
            return Err(Error::invalid_request(
                "Update speed must be 100 or 1000 milliseconds",
            ));
        }

        // Establish WebSocket connection
        let ws = self.create_ws();
        ws.connect().await?;

        // Watch the order book
        ws.watch_orderbook_internal(self, &binance_symbol, limit, update_speed, is_futures)
            .await
    }

    /// Watches order books for multiple symbols
    ///
    /// # Arguments
    /// * `symbols` - List of trading pairs (maximum 200)
    /// * `limit` - Optional depth limit
    /// * `params` - Optional parameters
    ///   - `speed`: Update frequency (100 or 1000 ms)
    ///
    /// # Returns
    /// Mapping of symbol to corresponding order book
    ///
    /// # Example
    /// ```rust,no_run
    /// # use ccxt_exchanges::binance::Binance;
    /// # use ccxt_core::ExchangeConfig;
    /// # async fn example() -> ccxt_core::error::Result<()> {
    /// let exchange = Binance::new(ExchangeConfig::default())?;
    ///
    /// let symbols = vec![
    ///     "BTC/USDT".to_string(),
    ///     "ETH/USDT".to_string(),
    /// ];
    ///
    /// let orderbooks = exchange.watch_order_books(symbols, None, None).await?;
    /// for (symbol, ob) in orderbooks {
    ///     println!("{}: spread = {:?}", symbol, ob.spread());
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn watch_order_books(
        &self,
        symbols: Vec<String>,
        limit: Option<i64>,
        params: Option<HashMap<String, Value>>,
    ) -> Result<HashMap<String, OrderBook>> {
        // Enforce symbol count constraints
        if symbols.is_empty() {
            return Err(Error::invalid_request("Symbols list cannot be empty"));
        }

        if symbols.len() > 200 {
            return Err(Error::invalid_request(
                "Binance supports max 200 symbols per connection",
            ));
        }

        // Load market metadata
        self.load_markets(false).await?;

        // Convert symbols to exchange format and ensure consistent market type
        let mut binance_symbols = Vec::new();
        let mut is_futures = false;

        for symbol in &symbols {
            let market = self.base.market(symbol).await?;
            binance_symbols.push(market.id.to_lowercase());

            let current_is_futures =
                market.market_type == MarketType::Swap || market.market_type == MarketType::Futures;
            if !binance_symbols.is_empty() && current_is_futures != is_futures {
                return Err(Error::invalid_request(
                    "Cannot mix spot and futures markets in watch_order_books",
                ));
            }
            is_futures = current_is_futures;
        }

        // Determine update speed
        let update_speed = if let Some(p) = &params {
            p.get("speed").and_then(|v| v.as_i64()).unwrap_or(100) as i32
        } else {
            100
        };

        // Establish WebSocket connection
        let ws = self.create_ws();
        ws.connect().await?;

        // Watch order books
        ws.watch_orderbooks_internal(self, binance_symbols, limit, update_speed, is_futures)
            .await
    }

    /// Watches mark prices for multiple futures symbols
    ///
    /// # Arguments
    /// * `symbols` - Optional list of symbols (None subscribes to all)
    /// * `params` - Optional parameters
    ///   - `use1sFreq`: Whether to use 1-second updates (defaults to true)
    ///
    /// # Returns
    /// Mapping of symbol to ticker data
    pub async fn watch_mark_prices(
        &self,
        symbols: Option<Vec<String>>,
        params: Option<HashMap<String, Value>>,
    ) -> Result<HashMap<String, Ticker>> {
        // Load market metadata
        self.load_markets(false).await?;

        // Determine update frequency
        let use_1s_freq = if let Some(p) = &params {
            p.get("use1sFreq").and_then(|v| v.as_bool()).unwrap_or(true)
        } else {
            true
        };

        // Construct channel name
        let channel_name = if use_1s_freq {
            "markPrice@1s"
        } else {
            "markPrice"
        };

        // Convert symbols and validate market type
        let binance_symbols = if let Some(syms) = symbols {
            let mut result = Vec::new();
            for symbol in syms {
                let market = self.base.market(&symbol).await?;
                if market.market_type != MarketType::Swap
                    && market.market_type != MarketType::Futures
                {
                    return Err(Error::invalid_request(format!(
                        "watch_mark_prices() does not support {} markets",
                        market.market_type
                    )));
                }
                result.push(market.id.to_lowercase());
            }
            Some(result)
        } else {
            None
        };

        // Establish WebSocket connection
        let ws = self.create_ws();
        ws.connect().await?;

        // Watch mark prices
        ws.watch_tickers_internal(binance_symbols, channel_name)
            .await
    }
    /// Streams trade data for a unified symbol
    ///
    /// # Arguments
    /// * `symbol` - Unified trading pair identifier (e.g. "BTC/USDT")
    /// * `since` - Optional starting timestamp in milliseconds
    /// * `limit` - Optional maximum number of trades to return
    ///
    /// # Returns
    /// Vector of parsed trade data
    pub async fn watch_trades(
        &self,
        symbol: &str,
        since: Option<i64>,
        limit: Option<usize>,
    ) -> Result<Vec<Trade>> {
        // Ensure market metadata is loaded
        self.base.load_markets(false).await?;

        // Resolve market information
        let market = self.base.market(symbol).await?;
        let binance_symbol = market.id.to_lowercase();

        // Establish WebSocket connection
        let ws = self.create_ws();
        ws.connect().await?;

        // Subscribe to the trade stream
        ws.subscribe_trades(&binance_symbol).await?;

        // Process incoming messages
        let mut retries = 0;
        const MAX_RETRIES: u32 = 50;

        while retries < MAX_RETRIES {
            if let Some(msg) = ws.client.receive().await {
                // Skip subscription acknowledgement messages
                if msg.get("result").is_some() || msg.get("id").is_some() {
                    continue;
                }

                // Parse trade payload
                if let Ok(trade) = parser::parse_ws_trade(&msg, Some(&market)) {
                    // Cache trade payload
                    let mut trades_map = ws.trades.lock().await;
                    let trades = trades_map
                        .entry(symbol.to_string())
                        .or_insert_with(VecDeque::new);

                    // Enforce cache size limit
                    const MAX_TRADES: usize = 1000;
                    if trades.len() >= MAX_TRADES {
                        trades.pop_front();
                    }
                    trades.push_back(trade);

                    // Gather trades from cache
                    let mut result: Vec<Trade> = trades.iter().cloned().collect();

                    // Apply optional `since` filter
                    if let Some(since_ts) = since {
                        result.retain(|t| t.timestamp >= since_ts);
                    }

                    // Apply optional limit
                    if let Some(limit_size) = limit {
                        if result.len() > limit_size {
                            result = result.split_off(result.len() - limit_size);
                        }
                    }

                    return Ok(result);
                }
            }

            retries += 1;
            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        Err(Error::network("Timeout waiting for trade data"))
    }

    /// Streams OHLCV data for a unified symbol
    ///
    /// # Arguments
    /// * `symbol` - Unified trading pair identifier (e.g. "BTC/USDT")
    /// * `timeframe` - Candlestick interval (e.g. "1m", "5m", "1h", "1d")
    /// * `since` - Optional starting timestamp in milliseconds
    /// * `limit` - Optional maximum number of entries to return
    ///
    /// # Returns
    /// Vector of OHLCV entries
    pub async fn watch_ohlcv(
        &self,
        symbol: &str,
        timeframe: &str,
        since: Option<i64>,
        limit: Option<usize>,
    ) -> Result<Vec<OHLCV>> {
        // Ensure market metadata is loaded
        self.base.load_markets(false).await?;

        // Resolve market information
        let market = self.base.market(symbol).await?;
        let binance_symbol = market.id.to_lowercase();

        // Establish WebSocket connection
        let ws = self.create_ws();
        ws.connect().await?;

        // Subscribe to the Kline stream
        ws.subscribe_kline(&binance_symbol, timeframe).await?;

        // Process incoming messages
        let mut retries = 0;
        const MAX_RETRIES: u32 = 50;

        while retries < MAX_RETRIES {
            if let Some(msg) = ws.client.receive().await {
                // Skip subscription acknowledgement messages
                if msg.get("result").is_some() || msg.get("id").is_some() {
                    continue;
                }

                // Parse OHLCV payload
                if let Ok(ohlcv) = parser::parse_ws_ohlcv(&msg) {
                    // Cache OHLCV entries
                    let cache_key = format!("{}:{}", symbol, timeframe);
                    let mut ohlcvs_map = ws.ohlcvs.lock().await;
                    let ohlcvs = ohlcvs_map.entry(cache_key).or_insert_with(VecDeque::new);

                    // Enforce cache size limit
                    const MAX_OHLCVS: usize = 1000;
                    if ohlcvs.len() >= MAX_OHLCVS {
                        ohlcvs.pop_front();
                    }
                    ohlcvs.push_back(ohlcv);

                    // Collect results from cache
                    let mut result: Vec<OHLCV> = ohlcvs.iter().cloned().collect();

                    // Apply optional `since` filter
                    if let Some(since_ts) = since {
                        result.retain(|o| o.timestamp >= since_ts);
                    }

                    // Apply optional limit
                    if let Some(limit_size) = limit {
                        if result.len() > limit_size {
                            result = result.split_off(result.len() - limit_size);
                        }
                    }

                    return Ok(result);
                }
            }

            retries += 1;
            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        Err(Error::network("Timeout waiting for OHLCV data"))
    }

    /// Streams the best bid/ask data for a unified symbol
    ///
    /// # Arguments
    /// * `symbol` - Unified trading pair identifier (e.g. "BTC/USDT")
    ///
    /// # Returns
    /// Latest bid/ask snapshot
    pub async fn watch_bids_asks(&self, symbol: &str) -> Result<BidAsk> {
        // Ensure market metadata is loaded
        self.base.load_markets(false).await?;

        // Resolve market details
        let market = self.base.market(symbol).await?;
        let binance_symbol = market.id.to_lowercase();

        // Establish WebSocket connection
        let ws = self.create_ws();
        ws.connect().await?;

        // Subscribe to the bookTicker stream
        let stream_name = format!("{}@bookTicker", binance_symbol);
        ws.client
            .subscribe(stream_name, Some(symbol.to_string()), None)
            .await?;

        // Process incoming messages
        let mut retries = 0;
        const MAX_RETRIES: u32 = 50;

        while retries < MAX_RETRIES {
            if let Some(msg) = ws.client.receive().await {
                // Skip subscription acknowledgement messages
                if msg.get("result").is_some() || msg.get("id").is_some() {
                    continue;
                }

                // Parse bid/ask payload
                if let Ok(bid_ask) = parser::parse_ws_bid_ask(&msg) {
                    // Cache the snapshot
                    let mut bids_asks_map = ws.bids_asks.lock().await;
                    bids_asks_map.insert(symbol.to_string(), bid_ask.clone());

                    return Ok(bid_ask);
                }
            }

            retries += 1;
            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        Err(Error::network("Timeout waiting for BidAsk data"))
    }

    /// Streams account balance changes (private user data stream)
    ///
    /// # Arguments
    /// * `params` - Optional parameters
    ///   - `type`: Account type (spot/future/delivery/margin, etc.)
    ///   - `fetchBalanceSnapshot`: Whether to fetch an initial snapshot (default false)
    ///   - `awaitBalanceSnapshot`: Whether to wait for snapshot completion (default true)
    ///
    /// # Returns
    /// Updated account balances
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use ccxt_exchanges::binance::Binance;
    /// # use ccxt_core::ExchangeConfig;
    /// # use std::collections::HashMap;
    /// # use std::sync::Arc;
    /// # use serde_json::json;
    /// # async fn example() -> ccxt_core::error::Result<()> {
    /// let mut config = ExchangeConfig::default();
    /// config.api_key = Some("your-api-key".to_string());
    /// config.secret = Some("your-secret".to_string());
    /// let exchange = Arc::new(Binance::new(config)?);
    ///
    /// // Watch spot account balance
    /// let balance = exchange.clone().watch_balance(None).await?;
    ///
    /// // Watch futures account balance
    /// let mut params = HashMap::new();
    /// params.insert("type".to_string(), json!("future"));
    /// let futures_balance = exchange.clone().watch_balance(Some(params)).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn watch_balance(
        self: Arc<Self>,
        params: Option<HashMap<String, Value>>,
    ) -> Result<Balance> {
        // Ensure market metadata is loaded
        self.base.load_markets(false).await?;

        // Resolve account type
        let account_type = if let Some(p) = &params {
            p.get("type")
                .and_then(|v| v.as_str())
                .unwrap_or_else(|| self.options.default_type.as_str())
        } else {
            self.options.default_type.as_str()
        };

        // Determine configuration flags
        let fetch_snapshot = if let Some(p) = &params {
            p.get("fetchBalanceSnapshot")
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
        } else {
            false
        };

        let await_snapshot = if let Some(p) = &params {
            p.get("awaitBalanceSnapshot")
                .and_then(|v| v.as_bool())
                .unwrap_or(true)
        } else {
            true
        };

        // Establish authenticated WebSocket connection
        let ws = self.create_authenticated_ws();
        ws.connect().await?;

        // Optionally fetch the initial snapshot
        if fetch_snapshot {
            let snapshot = self.fetch_balance(Some(account_type)).await?;

            // Update cache with the snapshot
            let mut balances = ws.balances.write().await;
            balances.insert(account_type.to_string(), snapshot.clone());

            if !await_snapshot {
                return Ok(snapshot);
            }
        }

        // Process balance update messages
        let mut retries = 0;
        const MAX_RETRIES: u32 = 100;

        while retries < MAX_RETRIES {
            if let Some(msg) = ws.client.receive().await {
                // Skip subscription acknowledgement messages
                if msg.get("result").is_some() || msg.get("id").is_some() {
                    continue;
                }

                // Determine message event type
                if let Some(event_type) = msg.get("e").and_then(|e| e.as_str()) {
                    // Handle supported balance message types
                    match event_type {
                        "balanceUpdate" | "outboundAccountPosition" | "ACCOUNT_UPDATE" => {
                            // Parse and update local balance cache
                            if let Ok(()) = ws.handle_balance_message(&msg, account_type).await {
                                // Retrieve the updated balance snapshot
                                let balances = ws.balances.read().await;
                                if let Some(balance) = balances.get(account_type) {
                                    return Ok(balance.clone());
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }

            retries += 1;
            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        Err(Error::network("Timeout waiting for balance data"))
    }

    /// Watches authenticated order updates via the user data stream
    ///
    /// Streams real-time order status changes delivered by Binance user data WebSocket messages
    ///
    /// # Arguments
    /// * `symbol` - Optional trading pair filter (e.g. "BTC/USDT")
    /// * `since` - Optional starting timestamp in milliseconds
    /// * `limit` - Optional maximum number of orders to return
    /// * `params` - Optional additional parameters
    ///
    /// # Returns
    /// Orders returned in descending chronological order
    ///
    /// # Examples
    /// ```no_run
    /// use std::sync::Arc;
    /// use ccxt_exchanges::binance::Binance;
    ///
    /// async fn example() -> Result<(), Box<dyn std::error::Error>> {
    ///     let exchange = Arc::new(Binance::new(Default::default())?);
    ///
    ///     // Watch all order updates
    ///     let orders = exchange.clone().watch_orders(None, None, None, None).await?;
    ///     println!("Received {} order updates", orders.len());
    ///
    ///     // Watch updates for a specific trading pair
    ///     let btc_orders = exchange.clone().watch_orders(Some("BTC/USDT"), None, None, None).await?;
    ///     println!("BTC/USDT orders: {:?}", btc_orders);
    ///
    ///     Ok(())
    /// }
    /// ```
    pub async fn watch_orders(
        self: Arc<Self>,
        symbol: Option<&str>,
        since: Option<i64>,
        limit: Option<usize>,
        _params: Option<HashMap<String, Value>>,
    ) -> Result<Vec<Order>> {
        self.base.load_markets(false).await?;

        let ws = self.create_authenticated_ws();
        ws.connect().await?;

        // Receive messages in a loop
        loop {
            if let Some(msg) = ws.client.receive().await {
                if let Value::Object(data) = msg {
                    if let Some(event_type) = data.get("e").and_then(|v| v.as_str()) {
                        match event_type {
                            "executionReport" => {
                                // Parse order payload
                                let order = self.parse_ws_order(&data)?;

                                // Update order cache
                                let mut orders = ws.orders.write().await;
                                let symbol_orders = orders
                                    .entry(order.symbol.clone())
                                    .or_insert_with(HashMap::new);
                                symbol_orders.insert(order.id.clone(), order.clone());
                                drop(orders);

                                // Check for trade execution events
                                if let Some(exec_type) = data.get("x").and_then(|v| v.as_str()) {
                                    if exec_type == "TRADE" {
                                        // Parse execution trade payload
                                        if let Ok(trade) =
                                            ws.parse_ws_trade(&Value::Object(data.clone()))
                                        {
                                            // Update my trades cache
                                            let mut trades = ws.my_trades.write().await;
                                            let symbol_trades = trades
                                                .entry(trade.symbol.clone())
                                                .or_insert_with(VecDeque::new);

                                            // Prepend newest trade and enforce max length of 1000
                                            symbol_trades.push_front(trade);
                                            if symbol_trades.len() > 1000 {
                                                symbol_trades.pop_back();
                                            }
                                        }
                                    }
                                }

                                // Return filtered orders
                                return self.filter_orders(&ws, symbol, since, limit).await;
                            }
                            _ => continue,
                        }
                    }
                }
            } else {
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        }
    }

    /// Parses a WebSocket order message
    fn parse_ws_order(&self, data: &serde_json::Map<String, Value>) -> Result<Order> {
        use ccxt_core::types::{OrderSide, OrderStatus, OrderType};
        use rust_decimal::Decimal;
        use std::str::FromStr;

        // Extract core fields
        let symbol = data.get("s").and_then(|v| v.as_str()).unwrap_or("");
        let order_id = data
            .get("i")
            .and_then(|v| v.as_i64())
            .map(|id| id.to_string())
            .unwrap_or_default();
        let client_order_id = data.get("c").and_then(|v| v.as_str()).map(String::from);

        // Map order status
        let status_str = data.get("X").and_then(|v| v.as_str()).unwrap_or("NEW");
        let status = match status_str {
            "NEW" => OrderStatus::Open,
            "PARTIALLY_FILLED" => OrderStatus::Open,
            "FILLED" => OrderStatus::Closed,
            "CANCELED" => OrderStatus::Cancelled,
            "REJECTED" => OrderStatus::Rejected,
            "EXPIRED" => OrderStatus::Expired,
            _ => OrderStatus::Open,
        };

        // Map order side
        let side_str = data.get("S").and_then(|v| v.as_str()).unwrap_or("BUY");
        let side = match side_str {
            "BUY" => OrderSide::Buy,
            "SELL" => OrderSide::Sell,
            _ => OrderSide::Buy,
        };

        // Map order type
        let type_str = data.get("o").and_then(|v| v.as_str()).unwrap_or("LIMIT");
        let order_type = match type_str {
            "LIMIT" => OrderType::Limit,
            "MARKET" => OrderType::Market,
            "STOP_LOSS" => OrderType::StopLoss,
            "STOP_LOSS_LIMIT" => OrderType::StopLossLimit,
            "TAKE_PROFIT" => OrderType::TakeProfit,
            "TAKE_PROFIT_LIMIT" => OrderType::TakeProfitLimit,
            "LIMIT_MAKER" => OrderType::LimitMaker,
            _ => OrderType::Limit,
        };

        // Parse amount and price fields
        let amount = data
            .get("q")
            .and_then(|v| v.as_str())
            .and_then(|s| Decimal::from_str(s).ok())
            .unwrap_or(Decimal::ZERO);

        let price = data
            .get("p")
            .and_then(|v| v.as_str())
            .and_then(|s| Decimal::from_str(s).ok());

        let filled = data
            .get("z")
            .and_then(|v| v.as_str())
            .and_then(|s| Decimal::from_str(s).ok());

        let cost = data
            .get("Z")
            .and_then(|v| v.as_str())
            .and_then(|s| Decimal::from_str(s).ok());

        // Derive remaining quantity
        let remaining = match filled {
            Some(fill) => Some(amount - fill),
            None => None,
        };

        // Compute average price
        let average = match (filled, cost) {
            (Some(fill), Some(c)) if fill > Decimal::ZERO && c > Decimal::ZERO => Some(c / fill),
            _ => None,
        };

        // Parse timestamps
        let timestamp = data.get("T").and_then(|v| v.as_i64());
        let last_trade_timestamp = data.get("T").and_then(|v| v.as_i64());

        Ok(Order {
            id: order_id,
            client_order_id,
            timestamp,
            datetime: timestamp.map(|ts| {
                chrono::DateTime::from_timestamp_millis(ts)
                    .map(|dt| dt.to_rfc3339())
                    .unwrap_or_default()
            }),
            last_trade_timestamp,
            symbol: symbol.to_string(),
            order_type,
            side,
            price,
            average,
            amount,
            cost,
            filled,
            remaining,
            status,
            fee: None,
            fees: None,
            trades: None,
            time_in_force: data.get("f").and_then(|v| v.as_str()).map(String::from),
            post_only: None,
            reduce_only: None,
            stop_price: data
                .get("P")
                .and_then(|v| v.as_str())
                .and_then(|s| Decimal::from_str(s).ok()),
            trigger_price: None,
            take_profit_price: None,
            stop_loss_price: None,
            trailing_delta: None,
            trailing_percent: None,
            activation_price: None,
            callback_rate: None,
            working_type: data.get("wt").and_then(|v| v.as_str()).map(String::from),
            info: data.iter().map(|(k, v)| (k.clone(), v.clone())).collect(),
        })
    }

    /// Filters cached orders by symbol, time range, and limit
    async fn filter_orders(
        &self,
        ws: &BinanceWs,
        symbol: Option<&str>,
        since: Option<i64>,
        limit: Option<usize>,
    ) -> Result<Vec<Order>> {
        let orders_map = ws.orders.read().await;

        // Filter by symbol when provided
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

        // Apply optional `since` filter
        if let Some(since_ts) = since {
            orders.retain(|order| order.timestamp.map_or(false, |ts| ts >= since_ts));
        }

        // Sort by timestamp descending
        orders.sort_by(|a, b| {
            let ts_a = a.timestamp.unwrap_or(0);
            let ts_b = b.timestamp.unwrap_or(0);
            ts_b.cmp(&ts_a)
        });

        // Apply optional limit
        if let Some(lim) = limit {
            orders.truncate(lim);
        }

        Ok(orders)
    }

    /// Watches authenticated user trade updates
    ///
    /// # Arguments
    /// * `symbol` - Optional trading pair to filter (None subscribes to all)
    /// * `since` - Starting timestamp in milliseconds
    /// * `limit` - Maximum number of trades to return
    /// * `params` - Additional parameters
    ///
    /// # Returns
    /// List of trade records
    ///
    /// # Example
    /// ```rust,no_run
    /// use std::sync::Arc;
    /// use ccxt_exchanges::binance::Binance;
    /// use ccxt_core::ExchangeConfig;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let mut config = ExchangeConfig::default();
    ///     config.api_key = Some("your-api-key".to_string());
    ///     config.secret = Some("your-secret".to_string());
    ///     let exchange = Arc::new(Binance::new(config)?);
    ///
    ///     // Subscribe to BTC/USDT trade updates
    ///     let trades = exchange.clone().watch_my_trades(Some("BTC/USDT"), None, None, None).await?;
    ///     println!("My trades: {:?}", trades);
    ///     Ok(())
    /// }
    /// ```
    pub async fn watch_my_trades(
        self: Arc<Self>,
        symbol: Option<&str>,
        since: Option<i64>,
        limit: Option<usize>,
        _params: Option<HashMap<String, Value>>,
    ) -> Result<Vec<Trade>> {
        // Establish authenticated WebSocket connection
        let ws = self.create_authenticated_ws();
        ws.connect().await?;

        // Process trade update messages
        let mut retries = 0;
        const MAX_RETRIES: u32 = 100;

        while retries < MAX_RETRIES {
            if let Some(msg) = ws.client.receive().await {
                // Skip subscription acknowledgements
                if msg.get("result").is_some() || msg.get("id").is_some() {
                    continue;
                }

                // Identify event type
                if let Some(event_type) = msg.get("e").and_then(|e| e.as_str()) {
                    // Handle executionReport events containing trade updates
                    if event_type == "executionReport" {
                        if let Ok(trade) = ws.parse_ws_trade(&msg) {
                            let symbol_key = trade.symbol.clone();

                            // Update cached trades
                            let mut trades_map = ws.my_trades.write().await;
                            let symbol_trades =
                                trades_map.entry(symbol_key).or_insert_with(VecDeque::new);

                            // Prepend latest trade; bound cache to 1000 entries
                            symbol_trades.push_front(trade);
                            if symbol_trades.len() > 1000 {
                                symbol_trades.pop_back();
                            }
                        }
                    }
                }
            } else {
                tokio::time::sleep(Duration::from_millis(100)).await;
            }

            retries += 1;
        }

        // Filter and return personal trades
        ws.filter_my_trades(symbol, since, limit).await
    }

    /// Watches authenticated futures position updates
    ///
    /// Receives ACCOUNT_UPDATE events via the user data stream to track changes to futures
    /// Supports both USD-margined (USD-M) and coin-margined (COIN-M) contracts.
    ///
    /// # Arguments
    /// * `symbols` - Optional list of symbols (None subscribes to all positions)
    /// * `since` - Optional starting timestamp
    /// * `limit` - Optional maximum number of positions to return
    /// * `params` - Optional parameters
    ///   - `type`: Market type (`future`/`delivery`, default `future`)
    ///   - `subType`: Subtype (`linear`/`inverse`)
    ///
    /// # Returns
    /// Collection of positions
    ///
    /// # Implementation Details
    /// 1. Subscribe to ACCOUNT_UPDATE events through the user data stream.
    /// 2. Parse the position data contained in the `P` array.
    /// 3. Update the internal position cache.
    /// 4. Filter results according to the provided arguments.
    ///
    /// # WebSocket Message Format
    /// ```json
    /// {
    ///   "e": "ACCOUNT_UPDATE",
    ///   "T": 1667881353112,
    ///   "E": 1667881353115,
    ///   "a": {
    ///     "P": [
    ///       {
    ///         "s": "BTCUSDT",
    ///         "pa": "-0.089",
    ///         "ep": "19700.03933",
    ///         "up": "1.53058860",
    ///         "mt": "isolated",
    ///         "ps": "BOTH"
    ///       }
    ///     ]
    ///   }
    /// }
    /// ```
    ///
    /// # Example
    /// ```rust,no_run
    /// # use ccxt_exchanges::binance::Binance;
    /// # use ccxt_core::ExchangeConfig;
    /// # use std::sync::Arc;
    /// # async fn example() -> ccxt_core::error::Result<()> {
    /// let exchange = Arc::new(Binance::new(ExchangeConfig::default())?);
    ///
    /// // Watch all positions
    /// let positions = exchange.clone().watch_positions(None, None, None, None).await?;
    /// for pos in positions {
    ///     println!("Symbol: {}, Side: {:?}, Contracts: {:?}",
    ///              pos.symbol, pos.side, pos.contracts);
    /// }
    ///
    /// // Watch a subset of symbols
    /// let symbols = vec!["BTC/USDT".to_string(), "ETH/USDT".to_string()];
    /// let positions = exchange.clone().watch_positions(Some(symbols), None, Some(10), None).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn watch_positions(
        self: Arc<Self>,
        symbols: Option<Vec<String>>,
        since: Option<i64>,
        limit: Option<usize>,
        _params: Option<HashMap<String, Value>>,
    ) -> Result<Vec<Position>> {
        // Establish authenticated WebSocket connection
        let ws = self.create_authenticated_ws();
        ws.connect().await?;

        // Process position update messages
        let mut retries = 0;
        const MAX_RETRIES: u32 = 100;

        while retries < MAX_RETRIES {
            if let Some(msg) = ws.client.receive().await {
                // Skip subscription acknowledgement messages
                if msg.get("result").is_some() || msg.get("id").is_some() {
                    continue;
                }

                // Handle ACCOUNT_UPDATE events only
                if let Some(event_type) = msg.get("e").and_then(|e| e.as_str()) {
                    if event_type == "ACCOUNT_UPDATE" {
                        if let Some(account_data) = msg.get("a") {
                            if let Some(positions_array) =
                                account_data.get("P").and_then(|p| p.as_array())
                            {
                                for position_data in positions_array {
                                    if let Ok(position) = ws.parse_ws_position(position_data).await
                                    {
                                        let symbol_key = position.symbol.clone();
                                        let side_key = position
                                            .side
                                            .clone()
                                            .unwrap_or_else(|| "both".to_string());

                                        // Update cached positions
                                        let mut positions_map = ws.positions.write().await;
                                        let symbol_positions = positions_map
                                            .entry(symbol_key)
                                            .or_insert_with(HashMap::new);

                                        // Remove positions with effectively zero contracts
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
                            }
                        }
                    }
                }
            } else {
                tokio::time::sleep(Duration::from_millis(100)).await;
            }

            retries += 1;
        }

        // Filter and return positions
        let symbols_ref = symbols.as_ref().map(|v| v.as_slice());
        ws.filter_positions(symbols_ref, since, limit).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_binance_ws_creation() {
        let ws = BinanceWs::new(WS_BASE_URL.to_string());
        // Basic creation test: ensure the listen key lock is accessible
        assert!(ws.listen_key.try_read().is_ok());
    }

    #[test]
    fn test_stream_format() {
        let symbol = "btcusdt";

        // Ticker stream format
        let ticker_stream = format!("{}@ticker", symbol);
        assert_eq!(ticker_stream, "btcusdt@ticker");

        // Trade stream format
        let trade_stream = format!("{}@trade", symbol);
        assert_eq!(trade_stream, "btcusdt@trade");

        // Depth stream format
        let depth_stream = format!("{}@depth20", symbol);
        assert_eq!(depth_stream, "btcusdt@depth20");

        // Kline stream format
        let kline_stream = format!("{}@kline_1m", symbol);
        assert_eq!(kline_stream, "btcusdt@kline_1m");
    }

    #[tokio::test]
    async fn test_subscription_manager_basic() {
        let manager = SubscriptionManager::new();
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();

        // Validate initial state
        assert_eq!(manager.active_count(), 0);
        assert!(!manager.has_subscription("btcusdt@ticker").await);

        // Add a subscription
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

        // Retrieve subscription
        let sub = manager.get_subscription("btcusdt@ticker").await;
        assert!(sub.is_some());
        let sub = sub.unwrap();
        assert_eq!(sub.stream, "btcusdt@ticker");
        assert_eq!(sub.symbol, "BTCUSDT");
        assert_eq!(sub.sub_type, SubscriptionType::Ticker);

        // Remove subscription
        manager.remove_subscription("btcusdt@ticker").await.unwrap();
        assert_eq!(manager.active_count(), 0);
        assert!(!manager.has_subscription("btcusdt@ticker").await);
    }

    #[tokio::test]
    async fn test_subscription_manager_multiple() {
        let manager = SubscriptionManager::new();
        let (tx1, _rx1) = tokio::sync::mpsc::unbounded_channel();
        let (tx2, _rx2) = tokio::sync::mpsc::unbounded_channel();
        let (tx3, _rx3) = tokio::sync::mpsc::unbounded_channel();

        // Add multiple subscriptions
        manager
            .add_subscription(
                "btcusdt@ticker".to_string(),
                "BTCUSDT".to_string(),
                SubscriptionType::Ticker,
                tx1,
            )
            .await
            .unwrap();

        manager
            .add_subscription(
                "btcusdt@depth".to_string(),
                "BTCUSDT".to_string(),
                SubscriptionType::OrderBook,
                tx2,
            )
            .await
            .unwrap();

        manager
            .add_subscription(
                "ethusdt@ticker".to_string(),
                "ETHUSDT".to_string(),
                SubscriptionType::Ticker,
                tx3,
            )
            .await
            .unwrap();

        assert_eq!(manager.active_count(), 3);

        // Query by symbol
        let btc_subs = manager.get_subscriptions_by_symbol("BTCUSDT").await;
        assert_eq!(btc_subs.len(), 2);

        let eth_subs = manager.get_subscriptions_by_symbol("ETHUSDT").await;
        assert_eq!(eth_subs.len(), 1);

        // Retrieve all subscriptions
        let all_subs = manager.get_all_subscriptions().await;
        assert_eq!(all_subs.len(), 3);

        // Clear all subscriptions
        manager.clear().await;
        assert_eq!(manager.active_count(), 0);
    }

    #[tokio::test]
    async fn test_subscription_type_from_stream() {
        // Ticker stream
        let sub_type = SubscriptionType::from_stream("btcusdt@ticker");
        assert_eq!(sub_type, Some(SubscriptionType::Ticker));

        // Order book streams
        let sub_type = SubscriptionType::from_stream("btcusdt@depth");
        assert_eq!(sub_type, Some(SubscriptionType::OrderBook));

        let sub_type = SubscriptionType::from_stream("btcusdt@depth@100ms");
        assert_eq!(sub_type, Some(SubscriptionType::OrderBook));

        // Trade streams
        let sub_type = SubscriptionType::from_stream("btcusdt@trade");
        assert_eq!(sub_type, Some(SubscriptionType::Trades));

        let sub_type = SubscriptionType::from_stream("btcusdt@aggTrade");
        assert_eq!(sub_type, Some(SubscriptionType::Trades));

        // Kline streams
        let sub_type = SubscriptionType::from_stream("btcusdt@kline_1m");
        assert_eq!(sub_type, Some(SubscriptionType::Kline("1m".to_string())));

        let sub_type = SubscriptionType::from_stream("btcusdt@kline_1h");
        assert_eq!(sub_type, Some(SubscriptionType::Kline("1h".to_string())));

        // Mark price stream
        let sub_type = SubscriptionType::from_stream("btcusdt@markPrice");
        assert_eq!(sub_type, Some(SubscriptionType::MarkPrice));

        // Book ticker stream
        let sub_type = SubscriptionType::from_stream("btcusdt@bookTicker");
        assert_eq!(sub_type, Some(SubscriptionType::BookTicker));

        // Unknown stream
        let sub_type = SubscriptionType::from_stream("btcusdt@unknown");
        assert_eq!(sub_type, None);
    }

    #[tokio::test]
    async fn test_subscription_send_message() {
        let manager = SubscriptionManager::new();
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();

        // Add subscription
        manager
            .add_subscription(
                "btcusdt@ticker".to_string(),
                "BTCUSDT".to_string(),
                SubscriptionType::Ticker,
                tx,
            )
            .await
            .unwrap();

        // Send message to stream
        let test_msg = serde_json::json!({
            "e": "24hrTicker",
            "s": "BTCUSDT",
            "c": "50000"
        });

        let sent = manager
            .send_to_stream("btcusdt@ticker", test_msg.clone())
            .await;
        assert!(sent);

        // Receive message
        let received = rx.recv().await;
        assert!(received.is_some());
        assert_eq!(received.unwrap(), test_msg);
    }

    #[tokio::test]
    async fn test_subscription_send_to_symbol() {
        let manager = SubscriptionManager::new();
        let (tx1, mut rx1) = tokio::sync::mpsc::unbounded_channel();
        let (tx2, mut rx2) = tokio::sync::mpsc::unbounded_channel();

        // Add two subscriptions for the same symbol
        manager
            .add_subscription(
                "btcusdt@ticker".to_string(),
                "BTCUSDT".to_string(),
                SubscriptionType::Ticker,
                tx1,
            )
            .await
            .unwrap();

        manager
            .add_subscription(
                "btcusdt@depth".to_string(),
                "BTCUSDT".to_string(),
                SubscriptionType::OrderBook,
                tx2,
            )
            .await
            .unwrap();

        // Send a message to all subscriptions for the symbol
        let test_msg = serde_json::json!({
            "s": "BTCUSDT",
            "data": "test"
        });

        let sent_count = manager.send_to_symbol("BTCUSDT", &test_msg).await;
        assert_eq!(sent_count, 2);

        // Receive messages
        let received1 = rx1.recv().await;
        assert!(received1.is_some());
        assert_eq!(received1.unwrap(), test_msg);

        let received2 = rx2.recv().await;
        assert!(received2.is_some());
        assert_eq!(received2.unwrap(), test_msg);
    }

    #[test]
    fn test_symbol_conversion() {
        let symbol = "BTC/USDT";
        let binance_symbol = symbol.replace('/', "").to_lowercase();
        assert_eq!(binance_symbol, "btcusdt");
    }

    // ==================== MessageRouter tests ====================

    #[test]
    fn test_reconnect_config_default() {
        let config = ReconnectConfig::default();

        assert!(config.enabled);
        assert_eq!(config.initial_delay_ms, 1000);
        assert_eq!(config.max_delay_ms, 30000);
        assert_eq!(config.backoff_multiplier, 2.0);
        assert_eq!(config.max_attempts, 0); // Unlimited retries
    }

    #[test]
    fn test_reconnect_config_calculate_delay() {
        let config = ReconnectConfig::default();

        // Exponential backoff tests
        assert_eq!(config.calculate_delay(0), 1000); // 1s
        assert_eq!(config.calculate_delay(1), 2000); // 2s
        assert_eq!(config.calculate_delay(2), 4000); // 4s
        assert_eq!(config.calculate_delay(3), 8000); // 8s
        assert_eq!(config.calculate_delay(4), 16000); // 16s
        assert_eq!(config.calculate_delay(5), 30000); // 30s (capped at max)
        assert_eq!(config.calculate_delay(6), 30000); // 30s (remains at max)
    }

    #[test]
    fn test_reconnect_config_should_retry() {
        let mut config = ReconnectConfig::default();

        // Unlimited retries
        assert!(config.should_retry(0));
        assert!(config.should_retry(10));
        assert!(config.should_retry(100));

        // Finite retries
        config.max_attempts = 3;
        assert!(config.should_retry(0));
        assert!(config.should_retry(1));
        assert!(config.should_retry(2));
        assert!(!config.should_retry(3));
        assert!(!config.should_retry(4));

        // Disabled retries
        config.enabled = false;
        assert!(!config.should_retry(0));
        assert!(!config.should_retry(1));
    }

    #[test]
    fn test_message_router_extract_stream_name_combined() {
        // Combined stream format
        let message = serde_json::json!({
            "stream": "btcusdt@ticker",
            "data": {
                "e": "24hrTicker",
                "s": "BTCUSDT"
            }
        });

        let stream_name = MessageRouter::extract_stream_name(&message).unwrap();
        assert_eq!(stream_name, "btcusdt@ticker");
    }

    #[test]
    fn test_message_router_extract_stream_name_ticker() {
        // Ticker single stream format
        let message = serde_json::json!({
            "e": "24hrTicker",
            "s": "BTCUSDT",
            "E": 1672531200000_u64,
            "c": "16950.00",
            "h": "17100.00"
        });

        let stream_name = MessageRouter::extract_stream_name(&message).unwrap();
        assert_eq!(stream_name, "btcusdt@ticker");
    }

    #[test]
    fn test_message_router_extract_stream_name_depth() {
        // Depth single stream format
        let message = serde_json::json!({
            "e": "depthUpdate",
            "s": "ETHUSDT",
            "E": 1672531200000_u64,
            "U": 157,
            "u": 160
        });

        let stream_name = MessageRouter::extract_stream_name(&message).unwrap();
        assert_eq!(stream_name, "ethusdt@depth");
    }

    #[test]
    fn test_message_router_extract_stream_name_trade() {
        // Test trade single-stream format
        let message = serde_json::json!({
            "e": "trade",
            "s": "BNBUSDT",
            "E": 1672531200000_u64,
            "t": 12345
        });

        let stream_name = MessageRouter::extract_stream_name(&message).unwrap();
        assert_eq!(stream_name, "bnbusdt@trade");
    }

    #[test]
    fn test_message_router_extract_stream_name_kline() {
        // Kline single stream format
        let message = serde_json::json!({
            "e": "kline",
            "s": "BTCUSDT",
            "E": 1672531200000_u64,
            "k": {
                "i": "1m",
                "t": 1672531200000_u64,
                "o": "16950.00"
            }
        });

        let stream_name = MessageRouter::extract_stream_name(&message).unwrap();
        assert_eq!(stream_name, "btcusdt@kline_1m");
    }

    #[test]
    fn test_message_router_extract_stream_name_mark_price() {
        // Mark price single stream format
        let message = serde_json::json!({
            "e": "markPriceUpdate",
            "s": "BTCUSDT",
            "E": 1672531200000_u64,
            "p": "16950.00"
        });

        let stream_name = MessageRouter::extract_stream_name(&message).unwrap();
        assert_eq!(stream_name, "btcusdt@markPrice");
    }

    #[test]
    fn test_message_router_extract_stream_name_book_ticker() {
        // Book ticker single stream format
        let message = serde_json::json!({
            "e": "bookTicker",
            "s": "ETHUSDT",
            "E": 1672531200000_u64,
            "b": "1200.00",
            "a": "1200.50"
        });

        let stream_name = MessageRouter::extract_stream_name(&message).unwrap();
        assert_eq!(stream_name, "ethusdt@bookTicker");
    }

    #[test]
    fn test_message_router_extract_stream_name_subscription_response() {
        // Subscription response should yield an error
        let message = serde_json::json!({
            "result": null,
            "id": 1
        });

        let result = MessageRouter::extract_stream_name(&message);
        assert!(result.is_err());
    }

    #[test]
    fn test_message_router_extract_stream_name_error_response() {
        // Error responses should yield an error
        let message = serde_json::json!({
            "error": {
                "code": -1,
                "msg": "Invalid request"
            },
            "id": 1
        });

        let result = MessageRouter::extract_stream_name(&message);
        assert!(result.is_err());
    }

    #[test]
    fn test_message_router_extract_stream_name_invalid() {
        // Invalid message format
        let message = serde_json::json!({
            "unknown": "data"
        });

        let result = MessageRouter::extract_stream_name(&message);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_message_router_creation() {
        let ws_url = "wss://stream.binance.com:9443/ws".to_string();
        let subscription_manager = Arc::new(SubscriptionManager::new());

        let router = MessageRouter::new(ws_url.clone(), subscription_manager);

        // Validate initial state
        assert!(!router.is_connected());
        assert_eq!(router.ws_url, ws_url);
    }

    #[tokio::test]
    async fn test_message_router_reconnect_config() {
        let ws_url = "wss://stream.binance.com:9443/ws".to_string();
        let subscription_manager = Arc::new(SubscriptionManager::new());

        let router = MessageRouter::new(ws_url, subscription_manager);

        // Default configuration
        let config = router.get_reconnect_config().await;
        assert!(config.enabled);
        assert_eq!(config.initial_delay_ms, 1000);

        // Setting new configuration
        let new_config = ReconnectConfig {
            enabled: false,
            initial_delay_ms: 2000,
            max_delay_ms: 60000,
            backoff_multiplier: 1.5,
            max_attempts: 5,
        };

        router.set_reconnect_config(new_config.clone()).await;

        let updated_config = router.get_reconnect_config().await;
        assert!(!updated_config.enabled);
        assert_eq!(updated_config.initial_delay_ms, 2000);
        assert_eq!(updated_config.max_delay_ms, 60000);
        assert_eq!(updated_config.backoff_multiplier, 1.5);
        assert_eq!(updated_config.max_attempts, 5);
    }
}
