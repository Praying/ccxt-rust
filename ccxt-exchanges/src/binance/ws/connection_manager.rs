use crate::binance::Binance;
use crate::binance::BinanceUrls;
use crate::binance::ws::BinanceWs;
use ccxt_core::error::Result;
use ccxt_core::types::MarketType;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

const MAX_STREAMS_PER_CONNECTION: usize = 200;

/// Manages multiple WebSocket connections for Binance to handle sharding and isolation.
///
/// Connections are organized by market type (Spot, Swap, Futures, Option) to ensure
/// that each market type connects to the correct WebSocket endpoint.
#[derive(Debug)]
pub struct BinanceConnectionManager {
    /// Pool of public data connections organized by market type
    public_shards: RwLock<HashMap<MarketType, Vec<Arc<BinanceWs>>>>,
    /// Dedicated connections for private user data organized by market type
    private_clients: RwLock<HashMap<MarketType, Arc<BinanceWs>>>,
    /// WebSocket URLs for different market types
    urls: BinanceUrls,
    /// Whether to use sandbox/testnet URLs
    is_sandbox: bool,
}

impl BinanceConnectionManager {
    /// Creates a new connection manager with the given URLs.
    pub fn new(urls: BinanceUrls, is_sandbox: bool) -> Self {
        Self {
            public_shards: RwLock::new(HashMap::new()),
            private_clients: RwLock::new(HashMap::new()),
            urls,
            is_sandbox,
        }
    }

    /// Creates a new connection manager for production environment.
    pub fn production() -> Self {
        Self::new(BinanceUrls::production(), false)
    }

    /// Creates a new connection manager for testnet environment.
    pub fn testnet() -> Self {
        Self::new(BinanceUrls::testnet(), true)
    }

    /// Returns the WebSocket URL for the given market type.
    ///
    /// # Arguments
    ///
    /// * `market_type` - The market type to get the URL for
    ///
    /// # Returns
    ///
    /// The WebSocket URL for the specified market type.
    pub fn get_ws_url(&self, market_type: MarketType) -> &str {
        match market_type {
            MarketType::Spot => &self.urls.ws,
            MarketType::Futures => &self.urls.ws_fapi, // Linear/USDT-margined futures
            MarketType::Swap => &self.urls.ws_dapi,    // Inverse/Coin-margined perpetuals
            MarketType::Option => &self.urls.ws_eapi,
        }
    }

    /// Gets or creates a public connection suitable for a new subscription.
    ///
    /// Connections are lazily initialized on first subscription for each market type.
    ///
    /// # Arguments
    ///
    /// * `market_type` - The market type for the connection
    ///
    /// # Returns
    ///
    /// An Arc to the WebSocket client for the specified market type.
    pub async fn get_public_connection(&self, market_type: MarketType) -> Result<Arc<BinanceWs>> {
        let mut shards = self.public_shards.write().await;

        // Get or create the shard list for this market type
        let market_shards = shards.entry(market_type).or_insert_with(Vec::new);

        // Try to find an existing shard with capacity
        for shard in market_shards.iter() {
            if shard.subscription_count() < MAX_STREAMS_PER_CONNECTION {
                return Ok(shard.clone());
            }
        }

        // If no shard available or all full, create a new one
        let ws_url = self.get_ws_url(market_type).to_string();
        let new_shard = Arc::new(BinanceWs::new(ws_url));

        // Connect the new shard immediately
        new_shard.connect().await?;

        market_shards.push(new_shard.clone());
        Ok(new_shard)
    }

    /// Gets or creates the dedicated private connection for the given market type.
    ///
    /// # Arguments
    ///
    /// * `market_type` - The market type for the connection
    /// * `binance` - Reference to the Binance instance for authentication
    ///
    /// # Returns
    ///
    /// An Arc to the authenticated WebSocket client.
    pub async fn get_private_connection(
        &self,
        market_type: MarketType,
        binance: &Arc<Binance>,
    ) -> Result<Arc<BinanceWs>> {
        let mut clients = self.private_clients.write().await;

        // Check if we have an existing connected client for this market type
        if let Some(client) = clients.get(&market_type) {
            if client.is_connected() {
                return Ok(client.clone());
            }
        }

        // Create new authenticated client with the correct URL for this market type
        let ws_url = self.get_ws_url(market_type).to_string();
        let new_client = Arc::new(BinanceWs::new_with_auth(
            ws_url,
            binance.clone(),
            market_type,
        ));

        // Connect user stream (this handles listenKey generation and management)
        new_client.connect_user_stream().await?;

        clients.insert(market_type, new_client.clone());
        Ok(new_client)
    }

    /// Returns the number of active public shards across all market types.
    pub async fn active_shards_count(&self) -> usize {
        self.public_shards.read().await.values().map(Vec::len).sum()
    }

    /// Returns the number of active public shards for a specific market type.
    pub async fn active_shards_count_for(&self, market_type: MarketType) -> usize {
        self.public_shards
            .read()
            .await
            .get(&market_type)
            .map_or(0, Vec::len)
    }

    /// Disconnects all active connections across all market types.
    pub async fn disconnect_all(&self) -> Result<()> {
        // Disconnect all public shards
        let mut shards = self.public_shards.write().await;
        for market_shards in shards.values() {
            for shard in market_shards {
                let _ = shard.disconnect().await;
            }
        }
        shards.clear();

        // Disconnect all private clients
        let mut private = self.private_clients.write().await;
        for (_, client) in private.drain() {
            let _ = client.disconnect().await;
        }

        Ok(())
    }

    /// Disconnects all connections for a specific market type.
    pub async fn disconnect_market(&self, market_type: MarketType) -> Result<()> {
        // Disconnect public shards for this market type
        let mut shards = self.public_shards.write().await;
        if let Some(market_shards) = shards.remove(&market_type) {
            for shard in &market_shards {
                let _ = shard.disconnect().await;
            }
        }

        // Disconnect private client for this market type
        let mut private = self.private_clients.write().await;
        if let Some(client) = private.remove(&market_type) {
            let _ = client.disconnect().await;
        }

        Ok(())
    }

    /// Checks if any connection is active (non-blocking).
    pub fn is_connected(&self) -> bool {
        if let Ok(shards) = self.public_shards.try_read() {
            for market_shards in shards.values() {
                for shard in market_shards {
                    if shard.is_connected() {
                        return true;
                    }
                }
            }
        }

        if let Ok(private) = self.private_clients.try_read() {
            for (_, client) in private.iter() {
                if client.is_connected() {
                    return true;
                }
            }
        }

        false
    }

    /// Checks if a connection is active for a specific market type (non-blocking).
    pub fn is_connected_for(&self, market_type: MarketType) -> bool {
        if let Ok(shards) = self.public_shards.try_read() {
            if let Some(market_shards) = shards.get(&market_type) {
                for shard in market_shards {
                    if shard.is_connected() {
                        return true;
                    }
                }
            }
        }

        if let Ok(private) = self.private_clients.try_read() {
            if let Some(client) = private.get(&market_type) {
                if client.is_connected() {
                    return true;
                }
            }
        }

        false
    }

    /// Returns a list of all active subscriptions across all connections.
    pub fn get_all_subscriptions(&self) -> Vec<String> {
        let mut subs = Vec::new();
        if let Ok(shards) = self.public_shards.try_read() {
            for market_shards in shards.values() {
                for shard in market_shards {
                    subs.extend(shard.subscriptions());
                }
            }
        }
        if let Ok(private) = self.private_clients.try_read() {
            for (_, client) in private.iter() {
                subs.extend(client.subscriptions());
            }
        }
        subs
    }

    /// Returns a list of active subscriptions for a specific market type.
    pub fn get_subscriptions_for(&self, market_type: MarketType) -> Vec<String> {
        let mut subs = Vec::new();
        if let Ok(shards) = self.public_shards.try_read() {
            if let Some(market_shards) = shards.get(&market_type) {
                for shard in market_shards {
                    subs.extend(shard.subscriptions());
                }
            }
        }
        if let Ok(private) = self.private_clients.try_read() {
            if let Some(client) = private.get(&market_type) {
                subs.extend(client.subscriptions());
            }
        }
        subs
    }

    /// Returns a reference to the URLs configuration.
    pub fn urls(&self) -> &BinanceUrls {
        &self.urls
    }

    /// Returns whether this manager is configured for sandbox/testnet.
    pub fn is_sandbox(&self) -> bool {
        self.is_sandbox
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_production_urls() {
        let manager = BinanceConnectionManager::production();
        assert!(!manager.is_sandbox());

        assert_eq!(
            manager.get_ws_url(MarketType::Spot),
            "wss://stream.binance.com:9443/ws"
        );
        assert_eq!(
            manager.get_ws_url(MarketType::Futures),
            "wss://fstream.binance.com/ws"
        );
        assert_eq!(
            manager.get_ws_url(MarketType::Swap),
            "wss://dstream.binance.com/ws"
        );
        assert_eq!(
            manager.get_ws_url(MarketType::Option),
            "wss://nbstream.binance.com/eoptions/ws"
        );
    }

    #[test]
    fn test_testnet_urls() {
        let manager = BinanceConnectionManager::testnet();
        assert!(manager.is_sandbox());

        assert_eq!(
            manager.get_ws_url(MarketType::Spot),
            "wss://testnet.binance.vision/ws"
        );
        assert_eq!(
            manager.get_ws_url(MarketType::Futures),
            "wss://stream.binancefuture.com/ws"
        );
        assert_eq!(
            manager.get_ws_url(MarketType::Swap),
            "wss://dstream.binancefuture.com/ws"
        );
    }

    #[test]
    fn test_custom_urls() {
        let mut urls = BinanceUrls::production();
        urls.ws = "wss://custom.example.com/ws".to_string();
        urls.ws_fapi = "wss://custom-fapi.example.com/ws".to_string();

        let manager = BinanceConnectionManager::new(urls, false);

        assert_eq!(
            manager.get_ws_url(MarketType::Spot),
            "wss://custom.example.com/ws"
        );
        assert_eq!(
            manager.get_ws_url(MarketType::Futures),
            "wss://custom-fapi.example.com/ws"
        );
        // Unchanged URLs should still be production defaults
        assert_eq!(
            manager.get_ws_url(MarketType::Swap),
            "wss://dstream.binance.com/ws"
        );
    }

    #[test]
    fn test_initial_state() {
        let manager = BinanceConnectionManager::production();

        // No connections initially
        assert!(!manager.is_connected());
        assert!(!manager.is_connected_for(MarketType::Spot));
        assert!(!manager.is_connected_for(MarketType::Futures));
        assert!(!manager.is_connected_for(MarketType::Swap));
        assert!(!manager.is_connected_for(MarketType::Option));

        // No subscriptions initially
        assert!(manager.get_all_subscriptions().is_empty());
        assert!(manager.get_subscriptions_for(MarketType::Spot).is_empty());
    }

    #[tokio::test]
    async fn test_active_shards_count_initial() {
        let manager = BinanceConnectionManager::production();

        assert_eq!(manager.active_shards_count().await, 0);
        assert_eq!(manager.active_shards_count_for(MarketType::Spot).await, 0);
        assert_eq!(
            manager.active_shards_count_for(MarketType::Futures).await,
            0
        );
    }

    #[tokio::test]
    async fn test_disconnect_all_empty() {
        let manager = BinanceConnectionManager::production();

        // Should not error when disconnecting with no connections
        let result = manager.disconnect_all().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_disconnect_market_empty() {
        let manager = BinanceConnectionManager::production();

        // Should not error when disconnecting a market type with no connections
        let result = manager.disconnect_market(MarketType::Spot).await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_urls_accessor() {
        let manager = BinanceConnectionManager::production();
        let urls = manager.urls();

        assert!(urls.ws.contains("stream.binance.com"));
        assert!(urls.ws_fapi.contains("fstream.binance.com"));
        assert!(urls.ws_dapi.contains("dstream.binance.com"));
        assert!(urls.ws_eapi.contains("nbstream.binance.com"));
    }

    #[test]
    fn test_market_type_url_routing_completeness() {
        let manager = BinanceConnectionManager::production();

        // Ensure all market types return non-empty URLs
        for market_type in &[
            MarketType::Spot,
            MarketType::Futures,
            MarketType::Swap,
            MarketType::Option,
        ] {
            let url = manager.get_ws_url(*market_type);
            assert!(
                !url.is_empty(),
                "URL for {:?} should not be empty",
                market_type
            );
            assert!(
                url.starts_with("wss://"),
                "URL for {:?} should start with wss://, got: {}",
                market_type,
                url
            );
        }
    }

    #[test]
    fn test_different_market_types_get_different_urls() {
        let manager = BinanceConnectionManager::production();

        let spot_url = manager.get_ws_url(MarketType::Spot);
        let futures_url = manager.get_ws_url(MarketType::Futures);
        let swap_url = manager.get_ws_url(MarketType::Swap);
        let option_url = manager.get_ws_url(MarketType::Option);

        // All URLs should be different
        assert_ne!(spot_url, futures_url);
        assert_ne!(spot_url, swap_url);
        assert_ne!(spot_url, option_url);
        assert_ne!(futures_url, swap_url);
        assert_ne!(futures_url, option_url);
        assert_ne!(swap_url, option_url);
    }
}
