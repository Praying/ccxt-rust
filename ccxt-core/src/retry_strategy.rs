//! Retry strategy module.
//!
//! Provides flexible retry strategy configuration and implementation:
//! - Fixed delay
//! - Exponential backoff
//! - Linear backoff
//! - Configurable retry conditions
//! - Retry budget mechanism

use crate::error::Error;
use std::time::Duration;

/// Retry strategy type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetryStrategyType {
    /// Fixed delay: wait a constant duration between retries.
    Fixed,
    /// Exponential backoff: delay grows exponentially (base_delay * 2^attempt).
    Exponential,
    /// Linear backoff: delay grows linearly (base_delay * attempt).
    Linear,
}

/// Retry configuration.
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// Maximum number of retry attempts.
    pub max_retries: u32,
    /// Type of retry strategy to use.
    pub strategy_type: RetryStrategyType,
    /// Base delay in milliseconds.
    pub base_delay_ms: u64,
    /// Maximum delay in milliseconds to prevent excessive backoff.
    pub max_delay_ms: u64,
    /// Whether to retry on network errors.
    pub retry_on_network_error: bool,
    /// Whether to retry on rate limit errors.
    pub retry_on_rate_limit: bool,
    /// Whether to retry on server errors (5xx).
    pub retry_on_server_error: bool,
    /// Whether to retry on timeout errors.
    pub retry_on_timeout: bool,
    /// Jitter factor (0.0-1.0) to add randomness and prevent thundering herd.
    pub jitter_factor: f64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            strategy_type: RetryStrategyType::Exponential,
            base_delay_ms: 100,
            max_delay_ms: 30000,
            retry_on_network_error: true,
            retry_on_rate_limit: true,
            retry_on_server_error: true,
            retry_on_timeout: true,
            jitter_factor: 0.1,
        }
    }
}

impl RetryConfig {
    /// Creates a conservative retry configuration with fewer retries and shorter delays.
    pub fn conservative() -> Self {
        Self {
            max_retries: 2,
            strategy_type: RetryStrategyType::Fixed,
            base_delay_ms: 500,
            max_delay_ms: 5000,
            retry_on_network_error: true,
            retry_on_rate_limit: true,
            retry_on_server_error: false,
            retry_on_timeout: false,
            jitter_factor: 0.0,
        }
    }

    /// Creates an aggressive retry configuration with more retries and longer delays.
    pub fn aggressive() -> Self {
        Self {
            max_retries: 5,
            strategy_type: RetryStrategyType::Exponential,
            base_delay_ms: 200,
            max_delay_ms: 60000,
            retry_on_network_error: true,
            retry_on_rate_limit: true,
            retry_on_server_error: true,
            retry_on_timeout: true,
            jitter_factor: 0.2,
        }
    }

    /// Creates a retry configuration for rate limit errors only.
    pub fn rate_limit_only() -> Self {
        Self {
            max_retries: 3,
            strategy_type: RetryStrategyType::Linear,
            base_delay_ms: 2000,
            max_delay_ms: 10000,
            retry_on_network_error: false,
            retry_on_rate_limit: true,
            retry_on_server_error: false,
            retry_on_timeout: false,
            jitter_factor: 0.0,
        }
    }
}

/// Retry strategy.
#[derive(Debug)]
pub struct RetryStrategy {
    config: RetryConfig,
}

impl RetryStrategy {
    /// Creates a new retry strategy with the given configuration.
    pub fn new(config: RetryConfig) -> Self {
        Self { config }
    }

    /// Creates a retry strategy with default configuration.
    pub fn default_strategy() -> Self {
        Self::new(RetryConfig::default())
    }

    /// Determines whether an error should be retried.
    ///
    /// # Arguments
    ///
    /// * `error` - The error to evaluate.
    /// * `attempt` - The current retry attempt number (1-based).
    ///
    /// # Returns
    ///
    /// `true` if the error should be retried, `false` otherwise.
    pub fn should_retry(&self, error: &Error, attempt: u32) -> bool {
        if attempt > self.config.max_retries {
            return false;
        }
        match error {
            Error::Network(_) => self.config.retry_on_network_error,
            Error::RateLimit { .. } => self.config.retry_on_rate_limit,
            Error::Exchange(details) => {
                if self.config.retry_on_server_error && Self::is_server_error(&details.message) {
                    return true;
                }
                if self.config.retry_on_timeout && Self::is_timeout_error(&details.message) {
                    return true;
                }
                false
            }
            _ => false,
        }
    }

    /// Calculates the retry delay based on strategy type and attempt number.
    ///
    /// # Arguments
    ///
    /// * `attempt` - The current retry attempt number (1-based).
    /// * `error` - The error that triggered the retry.
    ///
    /// # Returns
    ///
    /// The calculated delay duration before the next retry.
    pub fn calculate_delay(&self, attempt: u32, error: &Error) -> Duration {
        let base_delay = match self.config.strategy_type {
            RetryStrategyType::Fixed => self.config.base_delay_ms,
            RetryStrategyType::Exponential => {
                self.config.base_delay_ms * 2_u64.pow(attempt.saturating_sub(1))
            }
            RetryStrategyType::Linear => self.config.base_delay_ms * u64::from(attempt),
        };

        let mut delay = base_delay.min(self.config.max_delay_ms);

        if matches!(error, Error::RateLimit { .. }) {
            delay = delay.max(2000);
        }
        if self.config.jitter_factor > 0.0 {
            delay = self.apply_jitter(delay);
        }

        Duration::from_millis(delay)
    }

    /// Applies jitter to the delay to add randomness and prevent thundering herd.
    fn apply_jitter(&self, delay_ms: u64) -> u64 {
        use rand::Rng;
        let mut rng = rand::rngs::ThreadRng::default();
        #[allow(clippy::cast_precision_loss)]
        #[allow(clippy::cast_possible_truncation)]
        let jitter_range = (delay_ms as f64 * self.config.jitter_factor) as u64;
        let jitter = rng.random_range(0..=jitter_range);
        delay_ms + jitter
    }

    /// Checks if the message indicates a server error (5xx).
    fn is_server_error(msg: &str) -> bool {
        let msg_lower = msg.to_lowercase();
        msg_lower.contains("500")
            || msg_lower.contains("502")
            || msg_lower.contains("503")
            || msg_lower.contains("504")
            || msg_lower.contains("internal server error")
            || msg_lower.contains("bad gateway")
            || msg_lower.contains("service unavailable")
            || msg_lower.contains("gateway timeout")
    }

    /// Checks if the message indicates a timeout error.
    fn is_timeout_error(msg: &str) -> bool {
        let msg_lower = msg.to_lowercase();
        msg_lower.contains("timeout")
            || msg_lower.contains("timed out")
            || msg_lower.contains("408")
    }

    /// Returns a reference to the retry configuration.
    pub fn config(&self) -> &RetryConfig {
        &self.config
    }

    /// Returns the maximum number of retries.
    pub fn max_retries(&self) -> u32 {
        self.config.max_retries
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_retry_config_default() {
        let config = RetryConfig::default();
        assert_eq!(config.max_retries, 3);
        assert_eq!(config.strategy_type, RetryStrategyType::Exponential);
        assert_eq!(config.base_delay_ms, 100);
        assert!(config.retry_on_network_error);
        assert!(config.retry_on_rate_limit);
    }

    #[test]
    fn test_retry_config_conservative() {
        let config = RetryConfig::conservative();
        assert_eq!(config.max_retries, 2);
        assert_eq!(config.strategy_type, RetryStrategyType::Fixed);
        assert!(!config.retry_on_server_error);
    }

    #[test]
    fn test_retry_config_aggressive() {
        let config = RetryConfig::aggressive();
        assert_eq!(config.max_retries, 5);
        assert!(config.retry_on_server_error);
        assert!(config.retry_on_timeout);
    }

    #[test]
    fn test_should_retry_network_error() {
        let strategy = RetryStrategy::default_strategy();
        let error = Error::network("Connection failed");

        assert!(strategy.should_retry(&error, 1));
        assert!(strategy.should_retry(&error, 2));
        assert!(strategy.should_retry(&error, 3));
        assert!(!strategy.should_retry(&error, 4));
    }

    #[test]
    fn test_should_retry_rate_limit() {
        let strategy = RetryStrategy::default_strategy();
        let error = Error::rate_limit("Rate limit exceeded", None);

        assert!(strategy.should_retry(&error, 1));
        assert!(strategy.should_retry(&error, 3));
    }

    #[test]
    fn test_should_not_retry_invalid_request() {
        let strategy = RetryStrategy::default_strategy();
        let error = Error::invalid_request("Bad request");

        assert!(!strategy.should_retry(&error, 1));
    }

    #[test]
    fn test_calculate_delay_fixed() {
        let config = RetryConfig {
            strategy_type: RetryStrategyType::Fixed,
            base_delay_ms: 1000,
            jitter_factor: 0.0,
            ..Default::default()
        };
        let strategy = RetryStrategy::new(config);
        let error = Error::network("test");

        assert_eq!(strategy.calculate_delay(1, &error).as_millis(), 1000);
        assert_eq!(strategy.calculate_delay(2, &error).as_millis(), 1000);
        assert_eq!(strategy.calculate_delay(3, &error).as_millis(), 1000);
    }

    #[test]
    fn test_calculate_delay_exponential() {
        let config = RetryConfig {
            strategy_type: RetryStrategyType::Exponential,
            base_delay_ms: 100,
            max_delay_ms: 10000,
            jitter_factor: 0.0,
            ..Default::default()
        };
        let strategy = RetryStrategy::new(config);
        let error = Error::network("test");

        assert_eq!(strategy.calculate_delay(1, &error).as_millis(), 100);
        assert_eq!(strategy.calculate_delay(2, &error).as_millis(), 200);
        assert_eq!(strategy.calculate_delay(3, &error).as_millis(), 400);
        assert_eq!(strategy.calculate_delay(4, &error).as_millis(), 800);
    }

    #[test]
    fn test_calculate_delay_linear() {
        let config = RetryConfig {
            strategy_type: RetryStrategyType::Linear,
            base_delay_ms: 500,
            max_delay_ms: 10000,
            jitter_factor: 0.0,
            ..Default::default()
        };
        let strategy = RetryStrategy::new(config);
        let error = Error::network("test");

        assert_eq!(strategy.calculate_delay(1, &error).as_millis(), 500);
        assert_eq!(strategy.calculate_delay(2, &error).as_millis(), 1000);
        assert_eq!(strategy.calculate_delay(3, &error).as_millis(), 1500);
    }

    #[test]
    fn test_calculate_delay_with_max_limit() {
        let config = RetryConfig {
            strategy_type: RetryStrategyType::Exponential,
            base_delay_ms: 1000,
            max_delay_ms: 5000,
            jitter_factor: 0.0,
            ..Default::default()
        };
        let strategy = RetryStrategy::new(config);
        let error = Error::network("test");

        assert_eq!(strategy.calculate_delay(1, &error).as_millis(), 1000);
        assert_eq!(strategy.calculate_delay(2, &error).as_millis(), 2000);
        assert_eq!(strategy.calculate_delay(3, &error).as_millis(), 4000);
        assert_eq!(strategy.calculate_delay(4, &error).as_millis(), 5000);
        assert_eq!(strategy.calculate_delay(5, &error).as_millis(), 5000);
    }

    #[test]
    fn test_is_server_error() {
        assert!(RetryStrategy::is_server_error("500 Internal Server Error"));
        assert!(RetryStrategy::is_server_error("502 Bad Gateway"));
        assert!(RetryStrategy::is_server_error("503 Service Unavailable"));
        assert!(RetryStrategy::is_server_error("504 Gateway Timeout"));
        assert!(!RetryStrategy::is_server_error("400 Bad Request"));
        assert!(!RetryStrategy::is_server_error("404 Not Found"));
    }

    #[test]
    fn test_is_timeout_error() {
        assert!(RetryStrategy::is_timeout_error("Request timeout"));
        assert!(RetryStrategy::is_timeout_error("Connection timed out"));
        assert!(RetryStrategy::is_timeout_error("408 Request Timeout"));
        assert!(!RetryStrategy::is_timeout_error("Connection refused"));
    }

    #[test]
    fn test_rate_limit_error_minimum_delay() {
        let config = RetryConfig {
            strategy_type: RetryStrategyType::Fixed,
            base_delay_ms: 100, // 很短的基础延迟
            jitter_factor: 0.0,
            ..Default::default()
        };
        let strategy = RetryStrategy::new(config);
        let error = Error::rate_limit("Rate limit exceeded", None);

        assert!(strategy.calculate_delay(1, &error).as_millis() >= 2000);
    }
}
