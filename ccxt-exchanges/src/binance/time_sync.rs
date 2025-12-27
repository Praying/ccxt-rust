//! Time synchronization manager for Binance API.
//!
//! This module provides a thread-safe time synchronization mechanism that caches
//! the time offset between local system time and Binance server time. This optimization
//! reduces the number of network round-trips for signed API requests from 2 to 1.
//!
//! # Overview
//!
//! When making signed requests to Binance, a timestamp is required. Previously,
//! each signed request required fetching the server time first. With `TimeSyncManager`,
//! the time offset is cached and used to calculate server timestamps locally.
//!
//! # Example
//!
//! ```rust
//! use ccxt_exchanges::binance::time_sync::{TimeSyncConfig, TimeSyncManager};
//! use std::time::Duration;
//!
//! // Create with default configuration
//! let manager = TimeSyncManager::new();
//!
//! // Or with custom configuration
//! let config = TimeSyncConfig {
//!     sync_interval: Duration::from_secs(60),
//!     auto_sync: true,
//!     max_offset_drift: 3000,
//! };
//! let manager = TimeSyncManager::with_config(config);
//!
//! // Simulate receiving server time and updating offset
//! let server_time = 1704110400000i64; // Example server timestamp
//! manager.update_offset(server_time);
//!
//! // Get estimated server timestamp using cached offset
//! let estimated_server_time = manager.get_server_timestamp();
//! ```

use ccxt_core::time::TimestampUtils;
use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};
use std::time::Duration;

/// Time synchronization configuration.
///
/// Controls the behavior of the `TimeSyncManager` including sync intervals,
/// automatic sync, and drift tolerance.
///
/// # Example
///
/// ```rust
/// use ccxt_exchanges::binance::time_sync::TimeSyncConfig;
/// use std::time::Duration;
///
/// let config = TimeSyncConfig {
///     sync_interval: Duration::from_secs(30),
///     auto_sync: true,
///     max_offset_drift: 5000,
/// };
/// ```
#[derive(Debug, Clone)]
pub struct TimeSyncConfig {
    /// Sync interval duration.
    ///
    /// The time between automatic resyncs when `auto_sync` is enabled.
    /// Default: 30 seconds.
    pub sync_interval: Duration,

    /// Enable automatic periodic sync.
    ///
    /// When enabled, `needs_resync()` will return `true` after the sync interval
    /// has elapsed since the last sync.
    /// Default: true.
    pub auto_sync: bool,

    /// Maximum allowed time offset drift in milliseconds.
    ///
    /// This value represents the maximum acceptable drift before forcing a resync.
    /// Should be less than Binance's `recvWindow` (default 5000ms) to ensure
    /// signed requests are accepted.
    /// Default: 5000ms.
    pub max_offset_drift: i64,
}

impl Default for TimeSyncConfig {
    fn default() -> Self {
        Self {
            sync_interval: Duration::from_secs(30),
            auto_sync: true,
            max_offset_drift: 5000,
        }
    }
}

impl TimeSyncConfig {
    /// Creates a new configuration with the specified sync interval.
    ///
    /// # Arguments
    ///
    /// * `sync_interval` - Duration between automatic resyncs
    ///
    /// # Example
    ///
    /// ```rust
    /// use ccxt_exchanges::binance::time_sync::TimeSyncConfig;
    /// use std::time::Duration;
    ///
    /// let config = TimeSyncConfig::with_interval(Duration::from_secs(60));
    /// assert_eq!(config.sync_interval, Duration::from_secs(60));
    /// ```
    pub fn with_interval(sync_interval: Duration) -> Self {
        Self {
            sync_interval,
            ..Default::default()
        }
    }

    /// Creates a configuration with automatic sync disabled.
    ///
    /// Useful when you want to control sync timing manually.
    ///
    /// # Example
    ///
    /// ```rust
    /// use ccxt_exchanges::binance::time_sync::TimeSyncConfig;
    ///
    /// let config = TimeSyncConfig::manual_sync_only();
    /// assert!(!config.auto_sync);
    /// ```
    pub fn manual_sync_only() -> Self {
        Self {
            auto_sync: false,
            ..Default::default()
        }
    }
}

/// Thread-safe time synchronization manager.
///
/// Maintains a cached time offset between local system time and Binance server time.
/// Uses atomic operations for thread-safe access without locks.
///
/// # Thread Safety
///
/// All operations use atomic memory ordering:
/// - `Ordering::Acquire` for reads to ensure visibility of prior writes
/// - `Ordering::Release` for writes to ensure visibility to subsequent reads
///
/// # Example
///
/// ```rust
/// use ccxt_exchanges::binance::time_sync::TimeSyncManager;
///
/// let manager = TimeSyncManager::new();
///
/// // Check if sync is needed (always true initially)
/// assert!(manager.needs_resync());
///
/// // Simulate server time update
/// let server_time = 1704110400000i64;
/// manager.update_offset(server_time);
///
/// // Now initialized
/// assert!(manager.is_initialized());
///
/// // Get estimated server timestamp
/// let timestamp = manager.get_server_timestamp();
/// assert!(timestamp > 0);
/// ```
#[derive(Debug)]
pub struct TimeSyncManager {
    /// Cached time offset: server_time - local_time (in milliseconds).
    ///
    /// Positive value means server is ahead of local time.
    /// Negative value means server is behind local time.
    time_offset: AtomicI64,

    /// Timestamp of last successful sync (local time in milliseconds).
    last_sync_time: AtomicI64,

    /// Whether initial sync has been performed.
    initialized: AtomicBool,

    /// Sync configuration.
    config: TimeSyncConfig,
}

impl TimeSyncManager {
    /// Creates a new `TimeSyncManager` with default configuration.
    ///
    /// # Example
    ///
    /// ```rust
    /// use ccxt_exchanges::binance::time_sync::TimeSyncManager;
    ///
    /// let manager = TimeSyncManager::new();
    /// assert!(!manager.is_initialized());
    /// ```
    pub fn new() -> Self {
        Self::with_config(TimeSyncConfig::default())
    }

    /// Creates a new `TimeSyncManager` with custom configuration.
    ///
    /// # Arguments
    ///
    /// * `config` - Time sync configuration
    ///
    /// # Example
    ///
    /// ```rust
    /// use ccxt_exchanges::binance::time_sync::{TimeSyncConfig, TimeSyncManager};
    /// use std::time::Duration;
    ///
    /// let config = TimeSyncConfig {
    ///     sync_interval: Duration::from_secs(60),
    ///     auto_sync: true,
    ///     max_offset_drift: 3000,
    /// };
    /// let manager = TimeSyncManager::with_config(config);
    /// ```
    pub fn with_config(config: TimeSyncConfig) -> Self {
        Self {
            time_offset: AtomicI64::new(0),
            last_sync_time: AtomicI64::new(0),
            initialized: AtomicBool::new(false),
            config,
        }
    }

    /// Returns whether initial sync has been performed.
    ///
    /// # Returns
    ///
    /// `true` if `update_offset()` has been called at least once successfully.
    ///
    /// # Example
    ///
    /// ```rust
    /// use ccxt_exchanges::binance::time_sync::TimeSyncManager;
    ///
    /// let manager = TimeSyncManager::new();
    /// assert!(!manager.is_initialized());
    ///
    /// manager.update_offset(1704110400000);
    /// assert!(manager.is_initialized());
    /// ```
    #[inline]
    pub fn is_initialized(&self) -> bool {
        self.initialized.load(Ordering::Acquire)
    }

    /// Returns whether a resync is needed based on sync interval.
    ///
    /// Returns `true` if:
    /// - The manager is not initialized, OR
    /// - Auto sync is enabled AND the time since last sync exceeds the sync interval
    ///
    /// # Returns
    ///
    /// `true` if resync is needed, `false` otherwise.
    ///
    /// # Example
    ///
    /// ```rust
    /// use ccxt_exchanges::binance::time_sync::TimeSyncManager;
    ///
    /// let manager = TimeSyncManager::new();
    ///
    /// // Always needs resync when not initialized
    /// assert!(manager.needs_resync());
    ///
    /// // After initialization, depends on sync interval
    /// manager.update_offset(1704110400000);
    /// assert!(!manager.needs_resync()); // Just synced
    /// ```
    pub fn needs_resync(&self) -> bool {
        // Always need sync if not initialized
        if !self.is_initialized() {
            return true;
        }

        // If auto sync is disabled, never need automatic resync
        if !self.config.auto_sync {
            return false;
        }

        // Check if sync interval has elapsed
        let last_sync = self.last_sync_time.load(Ordering::Acquire);
        let now = TimestampUtils::now_ms();
        let elapsed = now.saturating_sub(last_sync);

        elapsed >= self.config.sync_interval.as_millis() as i64
    }

    /// Gets the current cached time offset.
    ///
    /// The offset represents: `server_time - local_time` in milliseconds.
    ///
    /// # Returns
    ///
    /// The cached time offset in milliseconds.
    ///
    /// # Example
    ///
    /// ```rust
    /// use ccxt_exchanges::binance::time_sync::TimeSyncManager;
    ///
    /// let manager = TimeSyncManager::new();
    /// assert_eq!(manager.get_offset(), 0); // Default offset
    ///
    /// // After sync, offset reflects the difference
    /// // (actual value depends on local vs server time)
    /// ```
    #[inline]
    pub fn get_offset(&self) -> i64 {
        self.time_offset.load(Ordering::Acquire)
    }

    /// Calculates the estimated server timestamp using cached offset.
    ///
    /// Formula: `server_timestamp = local_time + offset`
    ///
    /// Uses saturating arithmetic to prevent overflow.
    ///
    /// # Returns
    ///
    /// Estimated server timestamp in milliseconds.
    ///
    /// # Example
    ///
    /// ```rust
    /// use ccxt_exchanges::binance::time_sync::TimeSyncManager;
    ///
    /// let manager = TimeSyncManager::new();
    /// let timestamp = manager.get_server_timestamp();
    /// assert!(timestamp > 0);
    /// ```
    #[inline]
    pub fn get_server_timestamp(&self) -> i64 {
        let local_time = TimestampUtils::now_ms();
        let offset = self.get_offset();
        local_time.saturating_add(offset)
    }

    /// Updates the time offset based on server time.
    ///
    /// This method should be called after fetching server time from the API.
    /// It calculates the offset as: `offset = server_time - local_time`
    ///
    /// # Arguments
    ///
    /// * `server_time` - The server timestamp in milliseconds
    ///
    /// # Example
    ///
    /// ```rust
    /// use ccxt_exchanges::binance::time_sync::TimeSyncManager;
    ///
    /// let manager = TimeSyncManager::new();
    ///
    /// // Simulate receiving server time
    /// let server_time = 1704110400000i64;
    /// manager.update_offset(server_time);
    ///
    /// assert!(manager.is_initialized());
    /// ```
    pub fn update_offset(&self, server_time: i64) {
        let local_time = TimestampUtils::now_ms();
        let offset = server_time.saturating_sub(local_time);

        // Update all fields atomically (in terms of visibility)
        // Using Release ordering ensures these writes are visible to
        // subsequent Acquire reads
        self.time_offset.store(offset, Ordering::Release);
        self.last_sync_time.store(local_time, Ordering::Release);
        self.initialized.store(true, Ordering::Release);
    }

    /// Returns the last sync timestamp (local time).
    ///
    /// # Returns
    ///
    /// The local timestamp when the last sync occurred, in milliseconds.
    /// Returns 0 if never synced.
    ///
    /// # Example
    ///
    /// ```rust
    /// use ccxt_exchanges::binance::time_sync::TimeSyncManager;
    ///
    /// let manager = TimeSyncManager::new();
    /// assert_eq!(manager.last_sync_time(), 0);
    ///
    /// manager.update_offset(1704110400000);
    /// assert!(manager.last_sync_time() > 0);
    /// ```
    #[inline]
    pub fn last_sync_time(&self) -> i64 {
        self.last_sync_time.load(Ordering::Acquire)
    }

    /// Returns a reference to the sync configuration.
    ///
    /// # Returns
    ///
    /// Reference to the `TimeSyncConfig`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use ccxt_exchanges::binance::time_sync::TimeSyncManager;
    /// use std::time::Duration;
    ///
    /// let manager = TimeSyncManager::new();
    /// assert_eq!(manager.config().sync_interval, Duration::from_secs(30));
    /// ```
    #[inline]
    pub fn config(&self) -> &TimeSyncConfig {
        &self.config
    }

    /// Resets the manager to uninitialized state.
    ///
    /// This clears the cached offset and marks the manager as needing resync.
    /// Useful for testing or when a forced resync is needed.
    ///
    /// # Example
    ///
    /// ```rust
    /// use ccxt_exchanges::binance::time_sync::TimeSyncManager;
    ///
    /// let manager = TimeSyncManager::new();
    /// manager.update_offset(1704110400000);
    /// assert!(manager.is_initialized());
    ///
    /// manager.reset();
    /// assert!(!manager.is_initialized());
    /// ```
    pub fn reset(&self) {
        self.time_offset.store(0, Ordering::Release);
        self.last_sync_time.store(0, Ordering::Release);
        self.initialized.store(false, Ordering::Release);
    }
}

impl Default for TimeSyncManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_time_sync_config_default() {
        let config = TimeSyncConfig::default();
        assert_eq!(config.sync_interval, Duration::from_secs(30));
        assert!(config.auto_sync);
        assert_eq!(config.max_offset_drift, 5000);
    }

    #[test]
    fn test_time_sync_config_with_interval() {
        let config = TimeSyncConfig::with_interval(Duration::from_secs(60));
        assert_eq!(config.sync_interval, Duration::from_secs(60));
        assert!(config.auto_sync);
    }

    #[test]
    fn test_time_sync_config_manual_sync_only() {
        let config = TimeSyncConfig::manual_sync_only();
        assert!(!config.auto_sync);
    }

    #[test]
    fn test_time_sync_manager_new() {
        let manager = TimeSyncManager::new();
        assert!(!manager.is_initialized());
        assert_eq!(manager.get_offset(), 0);
        assert_eq!(manager.last_sync_time(), 0);
    }

    #[test]
    fn test_time_sync_manager_with_config() {
        let config = TimeSyncConfig {
            sync_interval: Duration::from_secs(60),
            auto_sync: false,
            max_offset_drift: 3000,
        };
        let manager = TimeSyncManager::with_config(config);
        assert_eq!(manager.config().sync_interval, Duration::from_secs(60));
        assert!(!manager.config().auto_sync);
        assert_eq!(manager.config().max_offset_drift, 3000);
    }

    #[test]
    fn test_needs_resync_when_not_initialized() {
        let manager = TimeSyncManager::new();
        assert!(manager.needs_resync());
    }

    #[test]
    fn test_needs_resync_after_initialization() {
        let manager = TimeSyncManager::new();
        let server_time = TimestampUtils::now_ms();
        manager.update_offset(server_time);

        // Should not need resync immediately after sync
        assert!(!manager.needs_resync());
    }

    #[test]
    fn test_needs_resync_with_auto_sync_disabled() {
        let config = TimeSyncConfig::manual_sync_only();
        let manager = TimeSyncManager::with_config(config);

        let server_time = TimestampUtils::now_ms();
        manager.update_offset(server_time);

        // Should never need automatic resync when auto_sync is disabled
        assert!(!manager.needs_resync());
    }

    #[test]
    fn test_update_offset() {
        let manager = TimeSyncManager::new();
        let local_time = TimestampUtils::now_ms();

        // Simulate server being 100ms ahead
        let server_time = local_time + 100;
        manager.update_offset(server_time);

        assert!(manager.is_initialized());
        // Offset should be approximately 100 (may vary slightly due to timing)
        let offset = manager.get_offset();
        assert!(offset >= 90 && offset <= 110, "Offset was: {}", offset);
    }

    #[test]
    fn test_get_server_timestamp() {
        let manager = TimeSyncManager::new();
        let local_time = TimestampUtils::now_ms();

        // Simulate server being 1000ms ahead
        let server_time = local_time + 1000;
        manager.update_offset(server_time);

        let estimated = manager.get_server_timestamp();
        // Estimated server time should be close to actual server time
        let diff = (estimated - server_time).abs();
        assert!(diff < 100, "Difference was: {}", diff);
    }

    #[test]
    fn test_reset() {
        let manager = TimeSyncManager::new();
        manager.update_offset(TimestampUtils::now_ms());
        assert!(manager.is_initialized());

        manager.reset();
        assert!(!manager.is_initialized());
        assert_eq!(manager.get_offset(), 0);
        assert_eq!(manager.last_sync_time(), 0);
    }

    #[test]
    fn test_thread_safety_concurrent_reads() {
        use std::sync::Arc;

        let manager = Arc::new(TimeSyncManager::new());
        manager.update_offset(TimestampUtils::now_ms() + 500);

        let mut handles = vec![];

        // Spawn multiple reader threads
        for _ in 0..10 {
            let manager_clone = Arc::clone(&manager);
            let handle = thread::spawn(move || {
                for _ in 0..100 {
                    let _ = manager_clone.get_server_timestamp();
                    let _ = manager_clone.get_offset();
                    let _ = manager_clone.is_initialized();
                }
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.join().unwrap();
        }
    }

    #[test]
    fn test_thread_safety_concurrent_writes() {
        use std::sync::Arc;

        let manager = Arc::new(TimeSyncManager::new());

        let mut handles = vec![];

        // Spawn multiple writer threads
        for i in 0..5 {
            let manager_clone = Arc::clone(&manager);
            let handle = thread::spawn(move || {
                for j in 0..20 {
                    let server_time = TimestampUtils::now_ms() + (i * 100 + j) as i64;
                    manager_clone.update_offset(server_time);
                }
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.join().unwrap();
        }

        // Manager should be initialized after all writes
        assert!(manager.is_initialized());
    }

    #[test]
    fn test_thread_safety_concurrent_read_write() {
        use std::sync::Arc;

        let manager = Arc::new(TimeSyncManager::new());
        manager.update_offset(TimestampUtils::now_ms());

        let mut handles = vec![];

        // Spawn reader threads
        for _ in 0..5 {
            let manager_clone = Arc::clone(&manager);
            let handle = thread::spawn(move || {
                for _ in 0..100 {
                    let _ = manager_clone.get_server_timestamp();
                    let _ = manager_clone.needs_resync();
                }
            });
            handles.push(handle);
        }

        // Spawn writer threads
        for i in 0..3 {
            let manager_clone = Arc::clone(&manager);
            let handle = thread::spawn(move || {
                for j in 0..50 {
                    let server_time = TimestampUtils::now_ms() + (i * 10 + j) as i64;
                    manager_clone.update_offset(server_time);
                }
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.join().unwrap();
        }

        // Verify manager is in a consistent state
        assert!(manager.is_initialized());
        assert!(manager.last_sync_time() > 0);
    }

    #[test]
    fn test_offset_calculation_with_negative_offset() {
        let manager = TimeSyncManager::new();
        let local_time = TimestampUtils::now_ms();

        // Simulate server being 500ms behind
        let server_time = local_time - 500;
        manager.update_offset(server_time);

        let offset = manager.get_offset();
        // Offset should be approximately -500
        assert!(offset >= -600 && offset <= -400, "Offset was: {}", offset);
    }

    #[test]
    fn test_saturating_arithmetic() {
        let manager = TimeSyncManager::new();

        // Test with extreme offset values
        manager.time_offset.store(i64::MAX, Ordering::Release);
        manager.initialized.store(true, Ordering::Release);

        // Should not panic due to overflow
        let timestamp = manager.get_server_timestamp();
        assert!(timestamp > 0);
    }
}
