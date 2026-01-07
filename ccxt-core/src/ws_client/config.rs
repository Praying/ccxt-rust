//! WebSocket configuration types.

use rand::Rng;
use std::time::Duration;

/// Exponential backoff configuration for reconnection.
#[derive(Debug, Clone)]
pub struct BackoffConfig {
    /// Base delay for first retry (default: 1 second)
    pub base_delay: Duration,
    /// Maximum delay cap (default: 60 seconds)
    pub max_delay: Duration,
    /// Jitter factor (0.0 - 1.0, default: 0.25 for 25%)
    pub jitter_factor: f64,
    /// Multiplier for exponential growth (default: 2.0)
    pub multiplier: f64,
}

impl Default for BackoffConfig {
    fn default() -> Self {
        Self {
            base_delay: Duration::from_secs(1),
            max_delay: Duration::from_secs(60),
            jitter_factor: 0.25,
            multiplier: 2.0,
        }
    }
}

/// Calculates retry delay with exponential backoff and jitter.
#[derive(Debug, Clone)]
pub struct BackoffStrategy {
    config: BackoffConfig,
}

impl BackoffStrategy {
    /// Creates a new backoff strategy with the given configuration.
    pub fn new(config: BackoffConfig) -> Self {
        Self { config }
    }

    /// Creates a new backoff strategy with default configuration.
    pub fn with_defaults() -> Self {
        Self::new(BackoffConfig::default())
    }

    /// Returns a reference to the underlying configuration.
    pub fn config(&self) -> &BackoffConfig {
        &self.config
    }

    /// Calculates delay for the given attempt number.
    #[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
    pub fn calculate_delay(&self, attempt: u32) -> Duration {
        let base_ms = self.config.base_delay.as_millis() as f64;
        let multiplier = self.config.multiplier;
        let max_ms = self.config.max_delay.as_millis() as f64;

        let exponential_delay_ms = base_ms * multiplier.powi(attempt as i32);
        let capped_delay_ms = exponential_delay_ms.min(max_ms);

        let jitter_ms = if self.config.jitter_factor > 0.0 {
            let jitter_range = capped_delay_ms * self.config.jitter_factor;
            rand::rng().random::<f64>() * jitter_range
        } else {
            0.0
        };

        Duration::from_millis((capped_delay_ms + jitter_ms) as u64)
    }

    /// Calculates the base delay (without jitter) for the given attempt number.
    #[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
    pub fn calculate_delay_without_jitter(&self, attempt: u32) -> Duration {
        let base_ms = self.config.base_delay.as_millis() as f64;
        let multiplier = self.config.multiplier;
        let max_ms = self.config.max_delay.as_millis() as f64;

        let exponential_delay_ms = base_ms * multiplier.powi(attempt as i32);
        let capped_delay_ms = exponential_delay_ms.min(max_ms);

        Duration::from_millis(capped_delay_ms as u64)
    }
}

/// Default maximum number of subscriptions.
pub const DEFAULT_MAX_SUBSCRIPTIONS: usize = 100;

/// Default shutdown timeout in milliseconds.
pub const DEFAULT_SHUTDOWN_TIMEOUT: u64 = 5000;

/// Default message channel capacity.
///
/// This is the maximum number of messages that can be buffered before
/// backpressure is applied. A value of 1000 provides a good balance
/// between memory usage and handling burst traffic.
pub const DEFAULT_MESSAGE_CHANNEL_CAPACITY: usize = 1000;

/// Default write channel capacity.
///
/// This is the maximum number of outgoing messages that can be buffered.
/// A smaller value (100) is used since writes are typically less frequent
/// than incoming messages.
pub const DEFAULT_WRITE_CHANNEL_CAPACITY: usize = 100;

/// Backpressure strategy when message channel is full.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BackpressureStrategy {
    /// Drop the oldest message in the queue (default).
    /// This ensures the most recent data is always available.
    #[default]
    DropOldest,
    /// Drop the newest message (the one being sent).
    /// This preserves message ordering but may lose recent updates.
    DropNewest,
    /// Block until space is available.
    /// Warning: This can cause the WebSocket read loop to stall.
    Block,
}

/// WebSocket connection configuration.
#[derive(Debug, Clone)]
pub struct WsConfig {
    /// WebSocket server URL
    pub url: String,
    /// Connection timeout in milliseconds
    pub connect_timeout: u64,
    /// Ping interval in milliseconds
    pub ping_interval: u64,
    /// Reconnection delay in milliseconds (legacy, use `backoff_config` for exponential backoff)
    pub reconnect_interval: u64,
    /// Maximum reconnection attempts before giving up
    pub max_reconnect_attempts: u32,
    /// Enable automatic reconnection on disconnect
    pub auto_reconnect: bool,
    /// Enable message compression
    pub enable_compression: bool,
    /// Pong timeout in milliseconds
    pub pong_timeout: u64,
    /// Exponential backoff configuration for reconnection.
    pub backoff_config: BackoffConfig,
    /// Maximum number of subscriptions allowed.
    pub max_subscriptions: usize,
    /// Shutdown timeout in milliseconds.
    pub shutdown_timeout: u64,
    /// Message channel capacity (incoming messages buffer size).
    ///
    /// When this limit is reached, backpressure is applied according to
    /// `backpressure_strategy`. Default: 1000 messages.
    pub message_channel_capacity: usize,
    /// Write channel capacity (outgoing messages buffer size).
    ///
    /// Default: 100 messages.
    pub write_channel_capacity: usize,
    /// Strategy to use when message channel is full.
    ///
    /// Default: `DropOldest` to ensure most recent data is available.
    pub backpressure_strategy: BackpressureStrategy,
}

impl Default for WsConfig {
    fn default() -> Self {
        Self {
            url: String::new(),
            connect_timeout: 10000,
            ping_interval: 30000,
            reconnect_interval: 5000,
            max_reconnect_attempts: 5,
            auto_reconnect: true,
            enable_compression: false,
            pong_timeout: 90000,
            backoff_config: BackoffConfig::default(),
            max_subscriptions: DEFAULT_MAX_SUBSCRIPTIONS,
            shutdown_timeout: DEFAULT_SHUTDOWN_TIMEOUT,
            message_channel_capacity: DEFAULT_MESSAGE_CHANNEL_CAPACITY,
            write_channel_capacity: DEFAULT_WRITE_CHANNEL_CAPACITY,
            backpressure_strategy: BackpressureStrategy::default(),
        }
    }
}
