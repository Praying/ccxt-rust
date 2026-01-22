//! HyperLiquid WebSocket implementation.
//!
//! Provides real-time data streaming via WebSocket for HyperLiquid exchange.
//! Supports public streams (ticker, orderbook, trades) and private streams
//! (user events, fills, order updates) with automatic reconnection.

use ccxt_core::{
    error::Result,
    ws_client::{WsClient, WsConfig, WsConnectionState},
};
use serde_json::{Value, json};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::{Mutex, RwLock};
use tokio::task::JoinHandle;
use tokio::time::{Duration, interval};

/// Default ping interval for HyperLiquid WebSocket (30 seconds).
const DEFAULT_PING_INTERVAL_MS: u64 = 50000;

/// Default reconnect delay (5 seconds).
const DEFAULT_RECONNECT_INTERVAL_MS: u64 = 5000;

/// Maximum reconnect attempts.
const MAX_RECONNECT_ATTEMPTS: u32 = 10;

/// HyperLiquid WebSocket client.
///
/// Provides real-time data streaming for HyperLiquid exchange.
#[derive(Debug, Clone)]
pub struct HyperLiquidWs {
    /// WebSocket client instance.
    client: Arc<WsClient>,
    /// Active subscriptions.
    subscriptions: Arc<RwLock<Vec<Subscription>>>,
    ping_active: Arc<AtomicBool>,
    ping_task: Arc<Mutex<Option<JoinHandle<()>>>>,
}

/// Subscription information.
#[derive(Debug, Clone)]
pub struct Subscription {
    /// Subscription type.
    pub sub_type: SubscriptionType,
    /// Symbol (if applicable).
    pub symbol: Option<String>,
}

/// Subscription types.
#[derive(Debug, Clone, PartialEq)]
pub enum SubscriptionType {
    /// All mid prices.
    AllMids,
    /// L2 order book.
    L2Book,
    /// Trades.
    Trades,
    /// Candles.
    Candle,
    /// User events.
    UserEvents,
    /// User fills.
    UserFills,
    /// Order updates.
    OrderUpdates,
}

impl HyperLiquidWs {
    /// Creates a new HyperLiquid WebSocket client.
    ///
    /// # Arguments
    ///
    /// * `url` - WebSocket server URL
    pub fn new(url: String) -> Self {
        let config = WsConfig {
            url,
            connect_timeout: 10000,
            ping_interval: DEFAULT_PING_INTERVAL_MS,
            reconnect_interval: DEFAULT_RECONNECT_INTERVAL_MS,
            max_reconnect_attempts: MAX_RECONNECT_ATTEMPTS,
            auto_reconnect: true,
            enable_compression: false,
            pong_timeout: 90000,
            ..Default::default()
        };

        // Initialize ping handler to maintain connection
        let mut config = config;
        config.ping_interval = DEFAULT_PING_INTERVAL_MS;

        // TODO: HyperLiquid uses {"method": "ping"} format
        // This requires custom ping handler in WsClient or message hook
        // For now relying on standard ping/pong frames or server keepalive

        Self {
            client: Arc::new(WsClient::new(config)),
            subscriptions: Arc::new(RwLock::new(Vec::new())),
            ping_active: Arc::new(AtomicBool::new(false)),
            ping_task: Arc::new(Mutex::new(None)),
        }
    }

    /// Connects to the WebSocket server.
    pub async fn connect(&self) -> Result<()> {
        self.client.connect().await?;
        self.start_ping_loop().await;
        self.resubscribe_all().await?;
        Ok(())
    }

    /// Disconnects from the WebSocket server.
    pub async fn disconnect(&self) -> Result<()> {
        self.stop_ping_loop().await;
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

    // ========================================================================
    // Public Subscriptions
    // ========================================================================

    /// Subscribes to all mid prices.
    pub async fn subscribe_all_mids(&self) -> Result<()> {
        self.send_subscription(SubscriptionType::AllMids, None)
            .await
    }

    /// Subscribes to L2 order book updates.
    ///
    /// # Arguments
    ///
    /// * `symbol` - Trading pair symbol (e.g., "BTC")
    pub async fn subscribe_l2_book(&self, symbol: &str) -> Result<()> {
        self.send_subscription(SubscriptionType::L2Book, Some(symbol.to_string()))
            .await
    }

    /// Subscribes to trade updates.
    ///
    /// # Arguments
    ///
    /// * `symbol` - Trading pair symbol (e.g., "BTC")
    pub async fn subscribe_trades(&self, symbol: &str) -> Result<()> {
        self.send_subscription(SubscriptionType::Trades, Some(symbol.to_string()))
            .await
    }

    /// Subscribes to candle updates.
    ///
    /// # Arguments
    ///
    /// * `symbol` - Trading pair symbol (e.g., "BTC")
    /// * `interval` - Candle interval (e.g., "1m", "1h")
    pub async fn subscribe_candle(&self, symbol: &str, interval: &str) -> Result<()> {
        self.send_candle_subscription(symbol, interval).await
    }

    // ========================================================================
    // Private Subscriptions
    // ========================================================================

    /// Subscribes to user events.
    ///
    /// # Arguments
    ///
    /// * `address` - User's Ethereum address
    pub async fn subscribe_user_events(&self, address: &str) -> Result<()> {
        self.send_subscription(SubscriptionType::UserEvents, Some(address.to_string()))
            .await
    }

    /// Subscribes to user fills.
    ///
    /// # Arguments
    ///
    /// * `address` - User's Ethereum address
    pub async fn subscribe_user_fills(&self, address: &str) -> Result<()> {
        self.send_subscription(SubscriptionType::UserFills, Some(address.to_string()))
            .await
    }

    /// Subscribes to order updates.
    ///
    /// # Arguments
    ///
    /// * `address` - User's Ethereum address
    pub async fn subscribe_order_updates(&self, address: &str) -> Result<()> {
        self.send_subscription(SubscriptionType::OrderUpdates, Some(address.to_string()))
            .await
    }
}

impl HyperLiquidWs {
    /// Returns a reference to the WebSocket client.
    pub fn client(&self) -> &Arc<WsClient> {
        &self.client
    }

    /// Returns a reference to the subscriptions.
    pub fn subscriptions(&self) -> &Arc<RwLock<Vec<Subscription>>> {
        &self.subscriptions
    }
}

impl HyperLiquidWs {
    async fn send_subscription(
        &self,
        sub_type: SubscriptionType,
        symbol: Option<String>,
    ) -> Result<()> {
        let mut subs = self.subscriptions.write().await;
        if subs
            .iter()
            .any(|sub| sub.sub_type == sub_type && sub.symbol == symbol)
        {
            return Ok(());
        }
        let msg = self.build_subscription_message(&sub_type, symbol.as_deref())?;
        self.client.send_json(&msg).await?;
        subs.push(Subscription { sub_type, symbol });
        Ok(())
    }

    async fn send_candle_subscription(&self, symbol: &str, interval: &str) -> Result<()> {
        let mut subs = self.subscriptions.write().await;
        let key = format!("{}:{}", symbol, interval);
        if subs.iter().any(|sub| {
            sub.sub_type == SubscriptionType::Candle && sub.symbol.as_deref() == Some(&key)
        }) {
            return Ok(());
        }
        let mut subscription_map = serde_json::Map::new();
        subscription_map.insert("type".to_string(), Value::String("candle".to_string()));
        subscription_map.insert("coin".to_string(), Value::String(symbol.to_string()));
        subscription_map.insert("interval".to_string(), Value::String(interval.to_string()));
        let msg = json!({"method": "subscribe", "subscription": subscription_map});
        self.client.send_json(&msg).await?;
        subs.push(Subscription {
            sub_type: SubscriptionType::Candle,
            symbol: Some(key),
        });
        Ok(())
    }

    fn build_subscription_message(
        &self,
        sub_type: &SubscriptionType,
        symbol: Option<&str>,
    ) -> Result<Value> {
        let mut subscription_map = serde_json::Map::new();
        match sub_type {
            SubscriptionType::AllMids => {
                subscription_map.insert("type".to_string(), Value::String("allMids".to_string()));
            }
            SubscriptionType::L2Book => {
                subscription_map.insert("type".to_string(), Value::String("l2Book".to_string()));
                if let Some(symbol) = symbol {
                    subscription_map.insert("coin".to_string(), Value::String(symbol.to_string()));
                }
            }
            SubscriptionType::Trades => {
                subscription_map.insert("type".to_string(), Value::String("trades".to_string()));
                if let Some(symbol) = symbol {
                    subscription_map.insert("coin".to_string(), Value::String(symbol.to_string()));
                }
            }
            SubscriptionType::UserEvents => {
                subscription_map
                    .insert("type".to_string(), Value::String("userEvents".to_string()));
                if let Some(address) = symbol {
                    subscription_map.insert("user".to_string(), Value::String(address.to_string()));
                }
            }
            SubscriptionType::UserFills => {
                subscription_map.insert("type".to_string(), Value::String("userFills".to_string()));
                if let Some(address) = symbol {
                    subscription_map.insert("user".to_string(), Value::String(address.to_string()));
                }
            }
            SubscriptionType::OrderUpdates => {
                subscription_map.insert(
                    "type".to_string(),
                    Value::String("orderUpdates".to_string()),
                );
                if let Some(address) = symbol {
                    subscription_map.insert("user".to_string(), Value::String(address.to_string()));
                }
            }
            SubscriptionType::Candle => {}
        }
        Ok(json!({"method": "subscribe", "subscription": subscription_map}))
    }

    async fn resubscribe_all(&self) -> Result<()> {
        let subs = self.subscriptions.read().await.clone();
        for sub in subs {
            if sub.sub_type == SubscriptionType::Candle {
                if let Some(symbol) = sub.symbol.as_deref() {
                    if let Some((coin, interval)) = symbol.split_once(':') {
                        let mut subscription_map = serde_json::Map::new();
                        subscription_map
                            .insert("type".to_string(), Value::String("candle".to_string()));
                        subscription_map
                            .insert("coin".to_string(), Value::String(coin.to_string()));
                        subscription_map
                            .insert("interval".to_string(), Value::String(interval.to_string()));
                        let msg = json!({"method": "subscribe", "subscription": subscription_map});
                        self.client.send_json(&msg).await?;
                        continue;
                    }
                }
            }
            let msg = self.build_subscription_message(&sub.sub_type, sub.symbol.as_deref())?;
            self.client.send_json(&msg).await?;
        }
        Ok(())
    }

    async fn start_ping_loop(&self) {
        if self.ping_active.swap(true, Ordering::SeqCst) {
            return;
        }
        let client = Arc::clone(&self.client);
        let active = Arc::clone(&self.ping_active);
        let mut guard = self.ping_task.lock().await;
        *guard = Some(tokio::spawn(async move {
            let mut interval = interval(Duration::from_millis(DEFAULT_PING_INTERVAL_MS));
            loop {
                interval.tick().await;
                if !active.load(Ordering::SeqCst) {
                    break;
                }
                let _ = client.send_json(&json!({"method": "ping"})).await;
            }
        }));
    }

    async fn stop_ping_loop(&self) {
        self.ping_active.store(false, Ordering::SeqCst);
        if let Some(handle) = self.ping_task.lock().await.take() {
            handle.abort();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_subscription_type() {
        let sub = Subscription {
            sub_type: SubscriptionType::AllMids,
            symbol: None,
        };
        assert_eq!(sub.sub_type, SubscriptionType::AllMids);
        assert!(sub.symbol.is_none());
    }

    #[test]
    fn test_subscription_with_symbol() {
        let sub = Subscription {
            sub_type: SubscriptionType::L2Book,
            symbol: Some("BTC".to_string()),
        };
        assert_eq!(sub.sub_type, SubscriptionType::L2Book);
        assert_eq!(sub.symbol, Some("BTC".to_string()));
    }
}
