//! WebSocket types and configuration for Binance
//!
//! This module contains configuration structures, enums, and types used
//! throughout the Binance WebSocket implementation.

use ccxt_core::ws_client::BackpressureStrategy;
use std::sync::atomic::{AtomicBool, AtomicU64};
use std::time::Duration;

/// Default channel capacity for ticker streams
pub const DEFAULT_TICKER_CAPACITY: usize = 256;
/// Default channel capacity for orderbook streams
pub const DEFAULT_ORDERBOOK_CAPACITY: usize = 512;
/// Default channel capacity for trade streams
pub const DEFAULT_TRADES_CAPACITY: usize = 1024;
/// Default channel capacity for user data streams
pub const DEFAULT_USER_DATA_CAPACITY: usize = 256;

/// Configuration for WebSocket channel capacities.
///
/// Different data streams have different message frequencies:
/// - Trades are highest frequency, need larger buffers
/// - Tickers and user data are lower frequency
#[derive(Debug, Clone)]
pub struct WsChannelConfig {
    /// Channel capacity for ticker streams (default: 256)
    pub ticker_capacity: usize,
    /// Channel capacity for orderbook streams (default: 512)
    pub orderbook_capacity: usize,
    /// Channel capacity for trade streams (default: 1024)
    pub trades_capacity: usize,
    /// Channel capacity for user data streams (default: 256)
    pub user_data_capacity: usize,
}

impl Default for WsChannelConfig {
    fn default() -> Self {
        Self {
            ticker_capacity: DEFAULT_TICKER_CAPACITY,
            orderbook_capacity: DEFAULT_ORDERBOOK_CAPACITY,
            trades_capacity: DEFAULT_TRADES_CAPACITY,
            user_data_capacity: DEFAULT_USER_DATA_CAPACITY,
        }
    }
}

/// Order book depth levels supported by Binance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum DepthLevel {
    /// 5 levels of depth
    L5 = 5,
    /// 10 levels of depth
    L10 = 10,
    /// 20 levels of depth
    #[default]
    L20 = 20,
}

impl DepthLevel {
    /// Returns the numeric value of the depth level
    pub fn as_u32(self) -> u32 {
        self as u32
    }
}

/// Update speed for order book streams.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum UpdateSpeed {
    /// 100 millisecond updates (faster, more bandwidth)
    #[default]
    Ms100,
    /// 1000 millisecond updates (slower, less bandwidth)
    Ms1000,
}

impl UpdateSpeed {
    /// Returns the string representation for Binance API
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Ms100 => "100ms",
            Self::Ms1000 => "1000ms",
        }
    }

    /// Returns the millisecond value
    pub fn as_millis(&self) -> u64 {
        match self {
            Self::Ms100 => 100,
            Self::Ms1000 => 1000,
        }
    }
}

/// Error recovery strategy for WebSocket errors.
///
/// This enum helps callers understand how to handle different error types.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WsErrorRecovery {
    /// Retry the operation with exponential backoff
    Retry {
        /// Maximum number of retry attempts
        max_attempts: u32,
        /// Base backoff duration
        backoff: Duration,
    },
    /// Resync state (e.g., fetch fresh orderbook snapshot)
    Resync,
    /// Reconnect the WebSocket connection
    Reconnect,
    /// Fatal error - requires user intervention
    Fatal,
}

impl WsErrorRecovery {
    /// Creates a retry recovery with default settings
    pub fn retry_default() -> Self {
        Self::Retry {
            max_attempts: 3,
            backoff: Duration::from_secs(1),
        }
    }

    /// Returns true if this is a recoverable error
    pub fn is_recoverable(&self) -> bool {
        !matches!(self, Self::Fatal)
    }

    /// Determines the recovery strategy for a given error message.
    ///
    /// This classifies errors into transient (retryable), resync-needed,
    /// reconnect-needed, or fatal categories.
    pub fn from_error_message(error_msg: &str) -> Self {
        let lower = error_msg.to_lowercase();

        // Fatal errors - require user intervention
        if is_fatal_error(&lower) {
            return Self::Fatal;
        }

        // Resync errors - need to refresh state
        if is_resync_error(&lower) {
            return Self::Resync;
        }

        // Reconnect errors - need new connection
        if is_reconnect_error(&lower) {
            return Self::Reconnect;
        }

        // Default to retry for transient errors
        Self::retry_default()
    }
}

/// Checks if an error message indicates a fatal (non-recoverable) error.
///
/// Fatal errors include:
/// - Authentication failures
/// - Invalid API keys
/// - Permission denied
/// - Account issues
fn is_fatal_error(error_msg: &str) -> bool {
    const FATAL_PATTERNS: &[&str] = &[
        "invalid api",
        "api key",
        "authentication",
        "unauthorized",
        "permission denied",
        "forbidden",
        "account",
        "banned",
        "ip banned",
        "invalid signature",
        "signature",
    ];

    FATAL_PATTERNS.iter().any(|p| error_msg.contains(p))
}

/// Checks if an error message indicates a resync is needed.
///
/// Resync errors include:
/// - Sequence gaps in orderbook
/// - Stale data
/// - Out of sync
fn is_resync_error(error_msg: &str) -> bool {
    const RESYNC_PATTERNS: &[&str] = &[
        "resync",
        "sequence",
        "out of sync",
        "stale",
        "gap",
        "missing update",
    ];

    RESYNC_PATTERNS.iter().any(|p| error_msg.contains(p))
}

/// Checks if an error message indicates a reconnection is needed.
///
/// Reconnect errors include:
/// - Connection closed
/// - Connection reset
/// - Server disconnect
fn is_reconnect_error(error_msg: &str) -> bool {
    const RECONNECT_PATTERNS: &[&str] = &[
        "connection closed",
        "connection reset",
        "disconnected",
        "eof",
        "broken pipe",
        "connection refused",
        "server closed",
    ];

    RECONNECT_PATTERNS.iter().any(|p| error_msg.contains(p))
}

/// Health snapshot for WebSocket connection monitoring.
///
/// Provides metrics for monitoring connection health and performance.
#[derive(Debug, Clone, Default)]
pub struct WsHealthSnapshot {
    /// Round-trip latency in milliseconds (from ping/pong)
    pub latency_ms: Option<i64>,
    /// Total number of messages received
    pub messages_received: u64,
    /// Number of messages dropped due to backpressure
    pub messages_dropped: u64,
    /// Timestamp of the last received message (milliseconds since epoch)
    pub last_message_time: Option<i64>,
    /// Connection uptime in milliseconds
    pub connection_uptime_ms: u64,
    /// Number of reconnection attempts
    pub reconnect_count: u32,
}

impl WsHealthSnapshot {
    /// Returns true if the connection appears healthy
    pub fn is_healthy(&self) -> bool {
        // Consider unhealthy if:
        // - No messages received in last 60 seconds
        // - Drop rate > 10%
        if let Some(last_time) = self.last_message_time {
            let now = chrono::Utc::now().timestamp_millis();
            if now - last_time > 60_000 {
                return false;
            }
        }

        if self.messages_received > 0 {
            let drop_rate = self.messages_dropped as f64 / self.messages_received as f64;
            if drop_rate > 0.1 {
                return false;
            }
        }

        true
    }
}

/// Internal statistics tracking for WebSocket connections.
#[derive(Debug, Default)]
pub struct WsStats {
    /// Total messages received
    pub messages_received: AtomicU64,
    /// Total messages dropped
    pub messages_dropped: AtomicU64,
    /// Last message timestamp
    pub last_message_time: AtomicU64,
    /// Connection start time
    pub connection_start_time: AtomicU64,
    /// Reconnect count
    pub reconnect_count: std::sync::atomic::AtomicU32,
}

/// Configuration for Binance WebSocket client.
///
/// Combines all configuration options for creating a BinanceWs instance.
#[derive(Debug, Clone)]
pub struct BinanceWsConfig {
    /// WebSocket endpoint URL
    pub url: String,
    /// Channel capacity configuration
    pub channel_config: WsChannelConfig,
    /// Backpressure strategy for handling channel overflow
    pub backpressure_strategy: BackpressureStrategy,
    /// Shutdown timeout in milliseconds
    pub shutdown_timeout_ms: u64,
}

impl BinanceWsConfig {
    /// Creates a new configuration with the given URL
    pub fn new(url: String) -> Self {
        Self {
            url,
            channel_config: WsChannelConfig::default(),
            backpressure_strategy: BackpressureStrategy::DropOldest,
            shutdown_timeout_ms: 5000,
        }
    }

    /// Sets the channel configuration
    pub fn with_channel_config(mut self, config: WsChannelConfig) -> Self {
        self.channel_config = config;
        self
    }

    /// Sets the backpressure strategy
    pub fn with_backpressure(mut self, strategy: BackpressureStrategy) -> Self {
        self.backpressure_strategy = strategy;
        self
    }

    /// Sets the shutdown timeout
    pub fn with_shutdown_timeout(mut self, timeout_ms: u64) -> Self {
        self.shutdown_timeout_ms = timeout_ms;
        self
    }
}

/// Shutdown state tracking
#[derive(Debug, Default)]
pub struct ShutdownState {
    /// Whether shutdown has been initiated
    pub is_shutting_down: AtomicBool,
    /// Whether shutdown has completed
    pub shutdown_complete: AtomicBool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_depth_level_values() {
        assert_eq!(DepthLevel::L5.as_u32(), 5);
        assert_eq!(DepthLevel::L10.as_u32(), 10);
        assert_eq!(DepthLevel::L20.as_u32(), 20);
    }

    #[test]
    fn test_update_speed_str() {
        assert_eq!(UpdateSpeed::Ms100.as_str(), "100ms");
        assert_eq!(UpdateSpeed::Ms1000.as_str(), "1000ms");
    }

    #[test]
    fn test_update_speed_millis() {
        assert_eq!(UpdateSpeed::Ms100.as_millis(), 100);
        assert_eq!(UpdateSpeed::Ms1000.as_millis(), 1000);
    }

    #[test]
    fn test_ws_error_recovery_is_recoverable() {
        assert!(WsErrorRecovery::retry_default().is_recoverable());
        assert!(WsErrorRecovery::Resync.is_recoverable());
        assert!(WsErrorRecovery::Reconnect.is_recoverable());
        assert!(!WsErrorRecovery::Fatal.is_recoverable());
    }

    #[test]
    fn test_ws_channel_config_default() {
        let config = WsChannelConfig::default();
        assert_eq!(config.ticker_capacity, DEFAULT_TICKER_CAPACITY);
        assert_eq!(config.orderbook_capacity, DEFAULT_ORDERBOOK_CAPACITY);
        assert_eq!(config.trades_capacity, DEFAULT_TRADES_CAPACITY);
        assert_eq!(config.user_data_capacity, DEFAULT_USER_DATA_CAPACITY);
    }

    #[test]
    fn test_binance_ws_config_builder() {
        let config = BinanceWsConfig::new("wss://test.com".to_string())
            .with_backpressure(BackpressureStrategy::DropNewest)
            .with_shutdown_timeout(10000);

        assert_eq!(config.url, "wss://test.com");
        assert_eq!(
            config.backpressure_strategy,
            BackpressureStrategy::DropNewest
        );
        assert_eq!(config.shutdown_timeout_ms, 10000);
    }

    #[test]
    fn test_ws_error_recovery_fatal_errors() {
        assert_eq!(
            WsErrorRecovery::from_error_message("Invalid API key"),
            WsErrorRecovery::Fatal
        );
        assert_eq!(
            WsErrorRecovery::from_error_message("Authentication failed"),
            WsErrorRecovery::Fatal
        );
        assert_eq!(
            WsErrorRecovery::from_error_message("Permission denied"),
            WsErrorRecovery::Fatal
        );
    }

    #[test]
    fn test_ws_error_recovery_resync_errors() {
        assert_eq!(
            WsErrorRecovery::from_error_message("RESYNC_NEEDED: sequence gap"),
            WsErrorRecovery::Resync
        );
        assert_eq!(
            WsErrorRecovery::from_error_message("Out of sync with server"),
            WsErrorRecovery::Resync
        );
    }

    #[test]
    fn test_ws_error_recovery_reconnect_errors() {
        assert_eq!(
            WsErrorRecovery::from_error_message("Connection closed by server"),
            WsErrorRecovery::Reconnect
        );
        assert_eq!(
            WsErrorRecovery::from_error_message("Connection reset"),
            WsErrorRecovery::Reconnect
        );
    }

    #[test]
    fn test_ws_error_recovery_transient_errors() {
        // Unknown errors default to retry
        let recovery = WsErrorRecovery::from_error_message("Network timeout");
        assert!(matches!(recovery, WsErrorRecovery::Retry { .. }));
    }
}
