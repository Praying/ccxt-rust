//! HyperLiquid WebSocket implementation.
//!
//! Provides real-time data streaming via WebSocket for HyperLiquid exchange.
//! Supports public streams (ticker, orderbook, trades) and private streams
//! (user events, fills, order updates) with automatic reconnection.

use ccxt_core::{
    error::Result,
    ws_client::{WsClient, WsConfig, WsConnectionState},
};
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Default ping interval for HyperLiquid WebSocket (30 seconds).
const DEFAULT_PING_INTERVAL_MS: u64 = 30000;

/// Default reconnect delay (5 seconds).
const DEFAULT_RECONNECT_INTERVAL_MS: u64 = 5000;

/// Maximum reconnect attempts.
const MAX_RECONNECT_ATTEMPTS: u32 = 10;

/// HyperLiquid WebSocket client.
///
/// Provides real-time data streaming for HyperLiquid exchange.
pub struct HyperLiquidWs {
    /// WebSocket client instance.
    client: Arc<WsClient>,
    /// Active subscriptions.
    subscriptions: Arc<RwLock<Vec<Subscription>>>,
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
    pub async fn state(&self) -> WsConnectionState {
        self.client.state().await
    }

    /// Checks if the WebSocket is connected.
    pub async fn is_connected(&self) -> bool {
        self.client.is_connected().await
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
        let mut subscription_map = serde_json::Map::new();
        subscription_map.insert(
            "type".to_string(),
            serde_json::Value::String("allMids".to_string()),
        );

        let mut msg_map = serde_json::Map::new();
        msg_map.insert(
            "method".to_string(),
            serde_json::Value::String("subscribe".to_string()),
        );
        msg_map.insert(
            "subscription".to_string(),
            serde_json::Value::Object(subscription_map),
        );

        let msg = serde_json::Value::Object(msg_map);

        self.client.send_json(&msg).await?;

        let mut subs = self.subscriptions.write().await;
        subs.push(Subscription {
            sub_type: SubscriptionType::AllMids,
            symbol: None,
        });

        Ok(())
    }

    /// Subscribes to L2 order book updates.
    ///
    /// # Arguments
    ///
    /// * `symbol` - Trading pair symbol (e.g., "BTC")
    pub async fn subscribe_l2_book(&self, symbol: &str) -> Result<()> {
        let mut subscription_map = serde_json::Map::new();
        subscription_map.insert(
            "type".to_string(),
            serde_json::Value::String("l2Book".to_string()),
        );
        subscription_map.insert(
            "coin".to_string(),
            serde_json::Value::String(symbol.to_string()),
        );

        let mut msg_map = serde_json::Map::new();
        msg_map.insert(
            "method".to_string(),
            serde_json::Value::String("subscribe".to_string()),
        );
        msg_map.insert(
            "subscription".to_string(),
            serde_json::Value::Object(subscription_map),
        );

        let msg = serde_json::Value::Object(msg_map);

        self.client.send_json(&msg).await?;

        let mut subs = self.subscriptions.write().await;
        subs.push(Subscription {
            sub_type: SubscriptionType::L2Book,
            symbol: Some(symbol.to_string()),
        });

        Ok(())
    }

    /// Subscribes to trade updates.
    ///
    /// # Arguments
    ///
    /// * `symbol` - Trading pair symbol (e.g., "BTC")
    pub async fn subscribe_trades(&self, symbol: &str) -> Result<()> {
        let mut subscription_map = serde_json::Map::new();
        subscription_map.insert(
            "type".to_string(),
            serde_json::Value::String("trades".to_string()),
        );
        subscription_map.insert(
            "coin".to_string(),
            serde_json::Value::String(symbol.to_string()),
        );

        let mut msg_map = serde_json::Map::new();
        msg_map.insert(
            "method".to_string(),
            serde_json::Value::String("subscribe".to_string()),
        );
        msg_map.insert(
            "subscription".to_string(),
            serde_json::Value::Object(subscription_map),
        );

        let msg = serde_json::Value::Object(msg_map);

        self.client.send_json(&msg).await?;

        let mut subs = self.subscriptions.write().await;
        subs.push(Subscription {
            sub_type: SubscriptionType::Trades,
            symbol: Some(symbol.to_string()),
        });

        Ok(())
    }

    /// Subscribes to candle updates.
    ///
    /// # Arguments
    ///
    /// * `symbol` - Trading pair symbol (e.g., "BTC")
    /// * `interval` - Candle interval (e.g., "1m", "1h")
    pub async fn subscribe_candle(&self, symbol: &str, interval: &str) -> Result<()> {
        let mut subscription_map = serde_json::Map::new();
        subscription_map.insert(
            "type".to_string(),
            serde_json::Value::String("candle".to_string()),
        );
        subscription_map.insert(
            "coin".to_string(),
            serde_json::Value::String(symbol.to_string()),
        );
        subscription_map.insert(
            "interval".to_string(),
            serde_json::Value::String(interval.to_string()),
        );

        let mut msg_map = serde_json::Map::new();
        msg_map.insert(
            "method".to_string(),
            serde_json::Value::String("subscribe".to_string()),
        );
        msg_map.insert(
            "subscription".to_string(),
            serde_json::Value::Object(subscription_map),
        );

        let msg = serde_json::Value::Object(msg_map);

        self.client.send_json(&msg).await?;

        let mut subs = self.subscriptions.write().await;
        subs.push(Subscription {
            sub_type: SubscriptionType::Candle,
            symbol: Some(symbol.to_string()),
        });

        Ok(())
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
        let mut subscription_map = serde_json::Map::new();
        subscription_map.insert(
            "type".to_string(),
            serde_json::Value::String("userEvents".to_string()),
        );
        subscription_map.insert(
            "user".to_string(),
            serde_json::Value::String(address.to_string()),
        );

        let mut msg_map = serde_json::Map::new();
        msg_map.insert(
            "method".to_string(),
            serde_json::Value::String("subscribe".to_string()),
        );
        msg_map.insert(
            "subscription".to_string(),
            serde_json::Value::Object(subscription_map),
        );

        let msg = serde_json::Value::Object(msg_map);

        self.client.send_json(&msg).await?;

        let mut subs = self.subscriptions.write().await;
        subs.push(Subscription {
            sub_type: SubscriptionType::UserEvents,
            symbol: None,
        });

        Ok(())
    }

    /// Subscribes to user fills.
    ///
    /// # Arguments
    ///
    /// * `address` - User's Ethereum address
    pub async fn subscribe_user_fills(&self, address: &str) -> Result<()> {
        let mut subscription_map = serde_json::Map::new();
        subscription_map.insert(
            "type".to_string(),
            serde_json::Value::String("userFills".to_string()),
        );
        subscription_map.insert(
            "user".to_string(),
            serde_json::Value::String(address.to_string()),
        );

        let mut msg_map = serde_json::Map::new();
        msg_map.insert(
            "method".to_string(),
            serde_json::Value::String("subscribe".to_string()),
        );
        msg_map.insert(
            "subscription".to_string(),
            serde_json::Value::Object(subscription_map),
        );

        let msg = serde_json::Value::Object(msg_map);

        self.client.send_json(&msg).await?;

        let mut subs = self.subscriptions.write().await;
        subs.push(Subscription {
            sub_type: SubscriptionType::UserFills,
            symbol: None,
        });

        Ok(())
    }

    /// Subscribes to order updates.
    ///
    /// # Arguments
    ///
    /// * `address` - User's Ethereum address
    pub async fn subscribe_order_updates(&self, address: &str) -> Result<()> {
        let mut subscription_map = serde_json::Map::new();
        subscription_map.insert(
            "type".to_string(),
            serde_json::Value::String("orderUpdates".to_string()),
        );
        subscription_map.insert(
            "user".to_string(),
            serde_json::Value::String(address.to_string()),
        );

        let mut msg_map = serde_json::Map::new();
        msg_map.insert(
            "method".to_string(),
            serde_json::Value::String("subscribe".to_string()),
        );
        msg_map.insert(
            "subscription".to_string(),
            serde_json::Value::Object(subscription_map),
        );

        let msg = serde_json::Value::Object(msg_map);

        self.client.send_json(&msg).await?;

        let mut subs = self.subscriptions.write().await;
        subs.push(Subscription {
            sub_type: SubscriptionType::OrderUpdates,
            symbol: None,
        });

        Ok(())
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
