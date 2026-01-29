use crate::binance::Binance;
use crate::binance::ws::BinanceWs;
use ccxt_core::error::Result;
use std::sync::Arc;
use tokio::sync::RwLock;

const MAX_STREAMS_PER_CONNECTION: usize = 200;

/// Manages multiple WebSocket connections for Binance to handle sharding and isolation
#[derive(Debug)]
pub struct BinanceConnectionManager {
    /// Pool of public data connections (shards)
    public_shards: RwLock<Vec<Arc<BinanceWs>>>,
    /// Dedicated connection for private user data
    private_client: RwLock<Option<Arc<BinanceWs>>>,
    /// Base WebSocket URL
    base_ws_url: String,
}

impl BinanceConnectionManager {
    /// Creates a new connection manager
    pub fn new(base_ws_url: String) -> Self {
        Self {
            public_shards: RwLock::new(Vec::new()),
            private_client: RwLock::new(None),
            base_ws_url,
        }
    }

    /// Gets or creates a public connection suitable for a new subscription
    pub async fn get_public_connection(&self) -> Result<Arc<BinanceWs>> {
        let mut shards = self.public_shards.write().await;

        // Try to find an existing shard with capacity
        for shard in shards.iter() {
            if shard.subscription_count() < MAX_STREAMS_PER_CONNECTION {
                return Ok(shard.clone());
            }
        }

        // If no shard available or all full, create a new one
        let new_shard = Arc::new(BinanceWs::new(self.base_ws_url.clone()));

        // Connect the new shard immediately
        new_shard.connect().await?;

        shards.push(new_shard.clone());
        Ok(new_shard)
    }

    /// Gets or creates the dedicated private connection
    pub async fn get_private_connection(&self, binance: &Arc<Binance>) -> Result<Arc<BinanceWs>> {
        let mut client_lock = self.private_client.write().await;

        if let Some(client) = client_lock.as_ref() {
            if client.is_connected() {
                return Ok(client.clone());
            }
        }

        // Create new authenticated client
        let new_client = Arc::new(BinanceWs::new_with_auth(
            self.base_ws_url.clone(),
            binance.clone(),
        ));

        // Connect user stream (this handles listenKey generation and management)
        new_client.connect_user_stream().await?;

        *client_lock = Some(new_client.clone());
        Ok(new_client)
    }

    /// Returns the number of active public shards
    pub async fn active_shards_count(&self) -> usize {
        self.public_shards.read().await.len()
    }

    /// Disconnects all active connections
    pub async fn disconnect_all(&self) -> Result<()> {
        let mut shards = self.public_shards.write().await;
        for shard in shards.iter() {
            let _ = shard.disconnect().await;
        }
        shards.clear();

        let mut private = self.private_client.write().await;
        if let Some(client) = private.take() {
            let _ = client.disconnect().await;
        }

        Ok(())
    }

    /// Checks if any connection is active (non-blocking)
    pub fn is_connected(&self) -> bool {
        if let Ok(shards) = self.public_shards.try_read() {
            if !shards.is_empty() {
                // Check if at least one shard is connected?
                for shard in shards.iter() {
                    if shard.is_connected() {
                        return true;
                    }
                }
            }
        }

        if let Ok(private) = self.private_client.try_read() {
            if let Some(client) = private.as_ref() {
                if client.is_connected() {
                    return true;
                }
            }
        }

        false
    }

    /// Returns a list of all active subscriptions across all connections
    pub fn get_all_subscriptions(&self) -> Vec<String> {
        let mut subs = Vec::new();
        if let Ok(shards) = self.public_shards.try_read() {
            for shard in shards.iter() {
                subs.extend(shard.subscriptions());
            }
        }
        if let Ok(private) = self.private_client.try_read() {
            if let Some(client) = private.as_ref() {
                subs.extend(client.subscriptions());
            }
        }
        subs
    }
}
