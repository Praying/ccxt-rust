//! Subscription management for Binance WebSocket streams

use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;

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
    pub fn from_stream(stream: &str) -> Option<Self> {
        if stream.contains("@ticker") {
            Some(Self::Ticker)
        } else if stream.contains("@depth") {
            Some(Self::OrderBook)
        } else if stream.contains("@trade") || stream.contains("@aggTrade") {
            Some(Self::Trades)
        } else if stream.contains("@kline_") {
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
    pub fn send(&self, message: Value) -> bool {
        self.sender.send(message).is_ok()
    }
}

/// Subscription manager
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
    pub async fn add_subscription(
        &self,
        stream: String,
        symbol: String,
        sub_type: SubscriptionType,
        sender: tokio::sync::mpsc::UnboundedSender<Value>,
    ) -> ccxt_core::error::Result<()> {
        let subscription = Subscription::new(stream.clone(), symbol.clone(), sub_type, sender);

        let mut subs = self.subscriptions.write().await;
        subs.insert(stream.clone(), subscription);

        let mut index = self.symbol_index.write().await;
        index.entry(symbol).or_insert_with(Vec::new).push(stream);

        self.active_count
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);

        Ok(())
    }

    /// Removes a subscription by stream name
    pub async fn remove_subscription(&self, stream: &str) -> ccxt_core::error::Result<()> {
        let mut subs = self.subscriptions.write().await;

        if let Some(subscription) = subs.remove(stream) {
            let mut index = self.symbol_index.write().await;
            if let Some(streams) = index.get_mut(&subscription.symbol) {
                streams.retain(|s| s != stream);
                if streams.is_empty() {
                    index.remove(&subscription.symbol);
                }
            }

            self.active_count
                .fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
        }

        Ok(())
    }

    /// Retrieves a subscription by stream name
    pub async fn get_subscription(&self, stream: &str) -> Option<Subscription> {
        let subs = self.subscriptions.read().await;
        subs.get(stream).cloned()
    }

    /// Checks whether a subscription exists for the given stream
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
    pub async fn send_to_stream(&self, stream: &str, message: Value) -> bool {
        let subs = self.subscriptions.read().await;
        if let Some(subscription) = subs.get(stream) {
            subscription.send(message)
        } else {
            false
        }
    }

    /// Sends a message to all subscribers of a symbol
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

impl Default for SubscriptionManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Reconnect configuration
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
            initial_delay_ms: 1000,
            max_delay_ms: 30000,
            backoff_multiplier: 2.0,
            max_attempts: 0,
        }
    }
}

impl ReconnectConfig {
    /// Calculates the reconnection delay
    pub fn calculate_delay(&self, attempt: usize) -> u64 {
        let delay = (self.initial_delay_ms as f64) * self.backoff_multiplier.powi(attempt as i32);
        delay.min(self.max_delay_ms as f64) as u64
    }

    /// Determines whether another reconnection attempt should be made
    pub fn should_retry(&self, attempt: usize) -> bool {
        self.enabled && (self.max_attempts == 0 || attempt < self.max_attempts)
    }
}
