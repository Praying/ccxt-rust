//! Listen key management for Binance user data streams

use crate::binance::Binance;
use ccxt_core::error::{Error, Result};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, RwLock};
use tokio::task::JoinHandle;

/// Listen key refresh interval (25 minutes)
///
/// Binance listen keys expire after 60 minutes. We refresh at 25 minutes
/// to provide a safety margin and avoid expiration during network delays.
const LISTEN_KEY_REFRESH_INTERVAL: Duration = Duration::from_secs(25 * 60);

/// Maximum age before forcing a new listen key (20 minutes)
///
/// If a listen key is older than this, we create a new one instead of
/// returning the cached key. This is more conservative than the refresh
/// interval to handle cases where auto-refresh might have failed.
const LISTEN_KEY_MAX_AGE: Duration = Duration::from_secs(20 * 60);

/// Maximum number of retry attempts for refresh failures
const MAX_REFRESH_RETRIES: u32 = 3;

/// Delay between refresh retry attempts
const REFRESH_RETRY_DELAY: Duration = Duration::from_secs(5);

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
    pub async fn get_or_create(&self) -> Result<String> {
        let key_opt = self.listen_key.read().await.clone();

        if let Some(key) = key_opt {
            let created = self.created_at.read().await;
            if let Some(created_time) = *created {
                let elapsed = created_time.elapsed();
                // Use the more conservative max age threshold
                if elapsed > LISTEN_KEY_MAX_AGE {
                    drop(created);
                    tracing::info!(
                        "Listen key is {} minutes old, creating new one",
                        elapsed.as_secs() / 60
                    );
                    return self.create_new().await;
                }
            }
            return Ok(key);
        }

        self.create_new().await
    }

    /// Creates a new listen key
    async fn create_new(&self) -> Result<String> {
        let key = self.binance.create_listen_key().await?;

        *self.listen_key.write().await = Some(key.clone());
        *self.created_at.write().await = Some(Instant::now());

        Ok(key)
    }

    /// Refreshes the current listen key
    pub async fn refresh(&self) -> Result<()> {
        let key_opt = self.listen_key.read().await.clone();

        if let Some(key) = key_opt {
            self.binance.refresh_listen_key(&key).await?;
            *self.created_at.write().await = Some(Instant::now());
            Ok(())
        } else {
            Err(Error::invalid_request("No listen key to refresh"))
        }
    }

    /// Regenerates the listen key (delete old, create new)
    pub async fn regenerate(&self) -> Result<String> {
        let _ = self.delete().await;
        self.create_new().await
    }

    /// Starts the auto-refresh task
    pub async fn start_auto_refresh(&self) {
        self.stop_auto_refresh().await;

        let listen_key = self.listen_key.clone();
        let created_at = self.created_at.clone();
        let binance = self.binance.clone();
        let interval = self.refresh_interval;

        let handle = tokio::spawn(async move {
            loop {
                tokio::time::sleep(interval).await;

                let key_opt = listen_key.read().await.clone();
                if let Some(key) = key_opt {
                    // Try to refresh with retries
                    let mut refresh_succeeded = false;
                    for attempt in 1..=MAX_REFRESH_RETRIES {
                        match binance.refresh_listen_key(&key).await {
                            Ok(()) => {
                                *created_at.write().await = Some(Instant::now());
                                tracing::debug!("Listen key refreshed successfully");
                                refresh_succeeded = true;
                                break;
                            }
                            Err(e) => {
                                tracing::warn!(
                                    "Failed to refresh listen key (attempt {}/{}): {}",
                                    attempt,
                                    MAX_REFRESH_RETRIES,
                                    e
                                );
                                if attempt < MAX_REFRESH_RETRIES {
                                    tokio::time::sleep(REFRESH_RETRY_DELAY).await;
                                }
                            }
                        }
                    }

                    // If all refresh attempts failed, try to regenerate
                    if !refresh_succeeded {
                        tracing::warn!(
                            "All refresh attempts failed, attempting to regenerate listen key"
                        );
                        match binance.create_listen_key().await {
                            Ok(new_key) => {
                                tracing::info!("Regenerated listen key successfully");
                                *listen_key.write().await = Some(new_key);
                                *created_at.write().await = Some(Instant::now());
                            }
                            Err(create_err) => {
                                tracing::error!("Failed to regenerate listen key: {}", create_err);
                                *listen_key.write().await = None;
                                *created_at.write().await = None;
                                break;
                            }
                        }
                    }
                } else {
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
    pub async fn delete(&self) -> Result<()> {
        self.stop_auto_refresh().await;

        let key_opt = self.listen_key.read().await.clone();

        if let Some(key) = key_opt {
            self.binance.delete_listen_key(&key).await?;

            *self.listen_key.write().await = None;
            *self.created_at.write().await = None;

            Ok(())
        } else {
            Ok(())
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
