//! Subscription management for Binance WebSocket streams

use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
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
    pub sender: tokio::sync::mpsc::Sender<Value>,
    /// Reference count for this subscription (how many handles are active)
    ref_count: Arc<AtomicUsize>,
}

impl Subscription {
    /// Creates a new subscription with the provided parameters
    pub fn new(
        stream: String,
        symbol: String,
        sub_type: SubscriptionType,
        sender: tokio::sync::mpsc::Sender<Value>,
    ) -> Self {
        Self {
            stream,
            symbol,
            sub_type,
            subscribed_at: Instant::now(),
            sender,
            ref_count: Arc::new(AtomicUsize::new(1)),
        }
    }

    /// Sends a message to the subscriber
    pub fn send(&self, message: Value) -> bool {
        // Use try_send to avoid blocking if channel is full (drop strategy)
        self.sender.try_send(message).is_ok()
    }

    /// Increments the reference count and returns the new value
    pub fn add_ref(&self) -> usize {
        self.ref_count.fetch_add(1, Ordering::SeqCst) + 1
    }

    /// Decrements the reference count and returns the new value
    pub fn remove_ref(&self) -> usize {
        let prev = self.ref_count.fetch_sub(1, Ordering::SeqCst);
        prev.saturating_sub(1)
    }

    /// Returns the current reference count
    pub fn ref_count(&self) -> usize {
        self.ref_count.load(Ordering::SeqCst)
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

    /// Adds a subscription to the manager.
    ///
    /// If a subscription for the same stream already exists, increments the reference count
    /// instead of creating a duplicate subscription.
    pub async fn add_subscription(
        &self,
        stream: String,
        symbol: String,
        sub_type: SubscriptionType,
        sender: tokio::sync::mpsc::Sender<Value>,
    ) -> ccxt_core::error::Result<()> {
        let mut subs = self.subscriptions.write().await;

        // Check if subscription already exists - increment ref count
        if let Some(existing) = subs.get(&stream) {
            existing.add_ref();
            return Ok(());
        }

        let subscription = Subscription::new(stream.clone(), symbol.clone(), sub_type, sender);

        subs.insert(stream.clone(), subscription);

        let mut index = self.symbol_index.write().await;
        index.entry(symbol).or_insert_with(Vec::new).push(stream);

        self.active_count
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);

        Ok(())
    }

    /// Removes a subscription by stream name.
    ///
    /// Only actually removes the subscription when the reference count reaches zero.
    /// Returns `true` if the subscription was fully removed (ref count hit zero).
    pub async fn remove_subscription(&self, stream: &str) -> ccxt_core::error::Result<bool> {
        let mut subs = self.subscriptions.write().await;

        if let Some(subscription) = subs.get(stream) {
            let remaining = subscription.remove_ref();
            if remaining > 0 {
                // Still has active references, don't remove
                return Ok(false);
            }

            // Ref count is zero, remove the subscription
            let Some(subscription) = subs.remove(stream) else {
                return Ok(false);
            };
            let mut index = self.symbol_index.write().await;
            if let Some(streams) = index.get_mut(&subscription.symbol) {
                streams.retain(|s| s != stream);
                if streams.is_empty() {
                    index.remove(&subscription.symbol);
                }
            }

            self.active_count
                .fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
            Ok(true)
        } else {
            Ok(false)
        }
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

    /// Returns all registered subscriptions synchronously (non-blocking)
    pub fn get_all_subscriptions_sync(&self) -> Vec<Subscription> {
        if let Ok(subs) = self.subscriptions.try_read() {
            subs.values().cloned().collect()
        } else {
            Vec::new()
        }
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
            if subscription.send(message) {
                return true;
            }
        } else {
            return false;
        }
        drop(subs);

        let _ = self.remove_subscription(stream).await;
        false
    }

    /// Sends a message to all subscribers of a symbol
    pub async fn send_to_symbol(&self, symbol: &str, message: &Value) -> usize {
        let index = self.symbol_index.read().await;
        let subs = self.subscriptions.read().await;

        let mut sent_count = 0;
        let mut streams_to_remove = Vec::new();

        if let Some(streams) = index.get(symbol) {
            for stream in streams {
                if let Some(subscription) = subs.get(stream) {
                    if subscription.send(message.clone()) {
                        sent_count += 1;
                    } else {
                        streams_to_remove.push(stream.clone());
                    }
                }
            }
        }
        drop(subs);
        drop(index);

        for stream in streams_to_remove {
            let _ = self.remove_subscription(&stream).await;
        }

        sent_count
    }

    /// Returns a list of all active stream names for resubscription
    pub async fn get_active_streams(&self) -> Vec<String> {
        let subs = self.subscriptions.read().await;
        subs.keys().cloned().collect()
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

/// A handle to an active subscription that automatically unsubscribes when dropped.
///
/// `SubscriptionHandle` implements a RAII pattern for WebSocket subscriptions.
/// When the handle is dropped, it decrements the reference count for the stream
/// and triggers an UNSUBSCRIBE command if no more handles reference the stream.
///
/// # Example
///
/// ```rust,ignore
/// let handle = subscription_manager.subscribe("btcusdt@ticker", ...).await?;
/// // ... use the subscription ...
/// drop(handle); // Automatically unsubscribes when dropped
/// ```
pub struct SubscriptionHandle {
    /// The stream name this handle is associated with
    stream: String,
    /// Reference to the subscription manager for cleanup
    subscription_manager: Arc<SubscriptionManager>,
    /// Reference to the message router for sending UNSUBSCRIBE
    message_router: Option<Arc<crate::binance::ws::handlers::MessageRouter>>,
    /// Whether this handle has already been released
    released: bool,
}

impl SubscriptionHandle {
    /// Creates a new subscription handle.
    pub fn new(
        stream: String,
        subscription_manager: Arc<SubscriptionManager>,
        message_router: Option<Arc<crate::binance::ws::handlers::MessageRouter>>,
    ) -> Self {
        Self {
            stream,
            subscription_manager,
            message_router,
            released: false,
        }
    }

    /// Returns the stream name associated with this handle.
    pub fn stream(&self) -> &str {
        &self.stream
    }

    /// Manually releases the subscription handle.
    ///
    /// This is equivalent to dropping the handle, but allows for async cleanup.
    /// After calling this method, the Drop implementation will be a no-op.
    pub async fn release(mut self) -> ccxt_core::error::Result<()> {
        self.released = true;
        self.do_release().await
    }

    /// Internal release logic
    async fn do_release(&self) -> ccxt_core::error::Result<()> {
        let fully_removed = self
            .subscription_manager
            .remove_subscription(&self.stream)
            .await?;

        if fully_removed {
            if let Some(router) = &self.message_router {
                router.unsubscribe(vec![self.stream.clone()]).await?;
            }
        }

        Ok(())
    }
}

impl Drop for SubscriptionHandle {
    fn drop(&mut self) {
        if self.released {
            return;
        }

        // We can't do async work in Drop, so we spawn a task to handle cleanup
        let stream = self.stream.clone();
        let subscription_manager = self.subscription_manager.clone();
        let message_router = self.message_router.clone();

        tokio::spawn(async move {
            let fully_removed = subscription_manager
                .remove_subscription(&stream)
                .await
                .unwrap_or(false);

            if fully_removed {
                if let Some(router) = &message_router {
                    let _ = router.unsubscribe(vec![stream]).await;
                }
            }
        });
    }
}
