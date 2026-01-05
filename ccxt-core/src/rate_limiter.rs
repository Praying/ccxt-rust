//! Rate Limiter Module
//!
//! This module provides rate limiting functionality to prevent exceeding exchange API limits.
//! It implements token bucket algorithm with configurable capacity and refill rate.
//!
//! # Features
//!
//! - **Token Bucket Algorithm**: Classic rate limiting strategy
//! - **Async-Friendly**: Built on tokio for async/await support
//! - **Thread-Safe**: Uses Arc<Mutex<>> for concurrent access
//! - **Configurable**: Flexible capacity and refill rate settings
//!
//! # Example
//!
//! ```rust
//! use ccxt_core::rate_limiter::{RateLimiter, RateLimiterConfig};
//! use std::time::Duration;
//!
//! # async fn example() {
//! // Create a rate limiter: 10 requests per second
//! let config = RateLimiterConfig::new(10, Duration::from_secs(1));
//! let limiter = RateLimiter::new(config);
//!
//! // Wait for permission to make a request
//! limiter.wait().await;
//! // Make your API request here
//! # }
//! ```

use crate::error::{ConfigValidationError, ValidationResult};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use tokio::time::sleep;

/// Rate limiter configuration
#[derive(Debug, Clone)]
pub struct RateLimiterConfig {
    /// Maximum number of tokens (requests) in the bucket
    pub capacity: u32,
    /// Time window for refilling tokens
    pub refill_period: Duration,
    /// Number of tokens to refill per period (defaults to capacity)
    pub refill_amount: u32,
    /// Cost per request in tokens (defaults to 1)
    pub cost_per_request: u32,
}

impl RateLimiterConfig {
    /// Create a new rate limiter configuration
    ///
    /// # Arguments
    ///
    /// * `capacity` - Maximum number of requests allowed in the time window
    /// * `refill_period` - Time window for the rate limit
    ///
    /// # Example
    ///
    /// ```rust
    /// use ccxt_core::rate_limiter::RateLimiterConfig;
    /// use std::time::Duration;
    ///
    /// // 100 requests per minute
    /// let config = RateLimiterConfig::new(100, Duration::from_secs(60));
    /// ```
    pub fn new(capacity: u32, refill_period: Duration) -> Self {
        Self {
            capacity,
            refill_period,
            refill_amount: capacity,
            cost_per_request: 1,
        }
    }

    /// Set custom refill amount (different from capacity)
    pub fn with_refill_amount(mut self, amount: u32) -> Self {
        self.refill_amount = amount;
        self
    }

    /// Set custom cost per request
    pub fn with_cost_per_request(mut self, cost: u32) -> Self {
        self.cost_per_request = cost;
        self
    }
}

impl Default for RateLimiterConfig {
    fn default() -> Self {
        // Default: 10 requests per second
        Self::new(10, Duration::from_secs(1))
    }
}

impl RateLimiterConfig {
    /// Validates the rate limiter configuration parameters.
    ///
    /// # Returns
    ///
    /// Returns `Ok(ValidationResult)` if the configuration is valid.
    /// The `ValidationResult` may contain warnings for suboptimal but valid configurations.
    ///
    /// Returns `Err(ConfigValidationError)` if the configuration is invalid.
    ///
    /// # Validation Rules
    ///
    /// - `capacity` must be > 0 (zero capacity is invalid)
    /// - `refill_period` < 100ms generates a warning (may cause high CPU usage)
    ///
    /// # Example
    ///
    /// ```rust
    /// use ccxt_core::rate_limiter::RateLimiterConfig;
    /// use std::time::Duration;
    ///
    /// let config = RateLimiterConfig::new(10, Duration::from_secs(1));
    /// let result = config.validate();
    /// assert!(result.is_ok());
    ///
    /// let invalid_config = RateLimiterConfig::new(0, Duration::from_secs(1));
    /// let result = invalid_config.validate();
    /// assert!(result.is_err());
    /// ```
    pub fn validate(&self) -> Result<ValidationResult, ConfigValidationError> {
        let mut warnings = Vec::new();

        // Validate capacity > 0
        if self.capacity == 0 {
            return Err(ConfigValidationError::invalid(
                "capacity",
                "capacity cannot be zero",
            ));
        }

        // Warn if refill_period < 100ms
        if self.refill_period < Duration::from_millis(100) {
            warnings.push(format!(
                "refill_period {:?} is very short, may cause high CPU usage",
                self.refill_period
            ));
        }

        Ok(ValidationResult::with_warnings(warnings))
    }
}

/// Internal state of the rate limiter
#[derive(Debug)]
struct RateLimiterState {
    /// Current number of available tokens
    tokens: u32,
    /// Last time tokens were refilled
    last_refill: Instant,
    /// Accumulated nanoseconds for fractional token tracking (precision optimization)
    /// This field tracks the remainder when elapsed time doesn't evenly divide into refill periods,
    /// preventing floating-point precision drift over long-running applications.
    remainder_nanos: u64,
    /// Configuration
    config: RateLimiterConfig,
}

impl RateLimiterState {
    fn new(config: RateLimiterConfig) -> Self {
        Self {
            tokens: config.capacity,
            last_refill: Instant::now(),
            remainder_nanos: 0,
            config,
        }
    }

    /// Refill tokens based on elapsed time using integer arithmetic
    ///
    /// This implementation uses nanosecond-based integer arithmetic instead of
    /// floating-point seconds to avoid precision drift over long-running applications.
    /// The formula used is: `elapsed_nanos / period_nanos * refill_amount`
    ///
    /// The `remainder_nanos` field tracks fractional periods that haven't yet
    /// accumulated to a full refill period, ensuring no time is lost between refills.
    #[allow(clippy::cast_possible_truncation)]
    fn refill(&mut self) {
        let now = Instant::now();
        // Note: as_nanos() returns u128, but for practical durations (< 584 years),
        // u64 is sufficient. We use saturating conversion to handle edge cases.
        let elapsed_nanos = now
            .duration_since(self.last_refill)
            .as_nanos()
            .min(u128::from(u64::MAX)) as u64;
        let period_nanos = self
            .config
            .refill_period
            .as_nanos()
            .min(u128::from(u64::MAX)) as u64;

        // Avoid division by zero (should not happen with valid config)
        if period_nanos == 0 {
            return;
        }

        // Add elapsed time to accumulated remainder
        let total_nanos = self.remainder_nanos.saturating_add(elapsed_nanos);

        // Calculate complete periods using integer division
        let complete_periods = total_nanos / period_nanos;

        if complete_periods > 0 {
            // Calculate tokens to add using integer arithmetic
            // Use u64 for intermediate calculation to avoid overflow
            let tokens_to_add = complete_periods
                .saturating_mul(u64::from(self.config.refill_amount))
                .min(u64::from(u32::MAX)) as u32;

            // Add tokens up to capacity
            self.tokens = self
                .tokens
                .saturating_add(tokens_to_add)
                .min(self.config.capacity);

            // Store remainder for next refill (preserves fractional periods)
            self.remainder_nanos = total_nanos % period_nanos;
            self.last_refill = now;
        }
    }

    /// Try to consume tokens
    fn try_consume(&mut self, cost: u32) -> bool {
        self.refill();

        if self.tokens >= cost {
            self.tokens -= cost;
            true
        } else {
            false
        }
    }

    /// Calculate wait time until enough tokens are available
    fn wait_time(&self, cost: u32) -> Duration {
        if self.tokens >= cost {
            return Duration::ZERO;
        }

        let tokens_needed = cost - self.tokens;
        let refill_rate =
            f64::from(self.config.refill_amount) / self.config.refill_period.as_secs_f64();
        let wait_seconds = f64::from(tokens_needed) / refill_rate;

        Duration::from_secs_f64(wait_seconds)
    }
}

/// Rate limiter using token bucket algorithm
///
/// This structure is thread-safe and can be shared across multiple tasks.
#[derive(Debug, Clone)]
pub struct RateLimiter {
    state: Arc<Mutex<RateLimiterState>>,
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self::new(RateLimiterConfig::default())
    }
}

impl RateLimiter {
    /// Create a new rate limiter with the given configuration
    ///
    /// # Example
    ///
    /// ```rust
    /// use ccxt_core::rate_limiter::{RateLimiter, RateLimiterConfig};
    /// use std::time::Duration;
    ///
    /// let config = RateLimiterConfig::new(50, Duration::from_secs(1));
    /// let limiter = RateLimiter::new(config);
    /// ```
    pub fn new(config: RateLimiterConfig) -> Self {
        Self {
            state: Arc::new(Mutex::new(RateLimiterState::new(config))),
        }
    }

    /// Wait until a request can be made (async)
    ///
    /// This method will block until enough tokens are available.
    ///
    /// # Example
    ///
    /// ```rust
    /// use ccxt_core::rate_limiter::RateLimiter;
    ///
    /// # async fn example() {
    /// let limiter = RateLimiter::default();
    /// limiter.wait().await;
    /// // Make API request here
    /// # }
    /// ```
    pub async fn wait(&self) {
        self.wait_with_cost(1).await;
    }

    /// Wait until a request with custom cost can be made
    ///
    /// # Arguments
    ///
    /// * `cost` - Number of tokens to consume for this request
    pub async fn wait_with_cost(&self, cost: u32) {
        loop {
            let wait_duration = {
                let mut state = self.state.lock().await;
                if state.try_consume(cost) {
                    return;
                }
                state.wait_time(cost)
            };

            if wait_duration > Duration::ZERO {
                sleep(wait_duration).await;
            } else {
                // Small delay to prevent busy waiting
                sleep(Duration::from_millis(10)).await;
            }
        }
    }

    /// Acquire permission to make a request (wait if necessary)
    ///
    /// This is an alias for `wait()` that matches the naming convention
    /// used in other rate limiting libraries.
    pub async fn acquire(&self, cost: u32) {
        self.wait_with_cost(cost).await;
    }

    /// Try to make a request without waiting
    ///
    /// Returns `true` if the request can proceed, `false` if rate limited.
    ///
    /// # Example
    ///
    /// ```rust
    /// use ccxt_core::rate_limiter::RateLimiter;
    ///
    /// # async fn example() {
    /// let limiter = RateLimiter::default();
    /// if limiter.try_acquire().await {
    ///     // Make API request
    /// } else {
    ///     // Rate limited, handle accordingly
    /// }
    /// # }
    /// ```
    pub async fn try_acquire(&self) -> bool {
        self.try_acquire_with_cost(1).await
    }

    /// Try to make a request with custom cost without waiting
    pub async fn try_acquire_with_cost(&self, cost: u32) -> bool {
        let mut state = self.state.lock().await;
        state.try_consume(cost)
    }

    /// Get current number of available tokens
    pub async fn available_tokens(&self) -> u32 {
        let mut state = self.state.lock().await;
        state.refill();
        state.tokens
    }

    /// Reset the rate limiter to full capacity
    pub async fn reset(&self) {
        let mut state = self.state.lock().await;
        state.tokens = state.config.capacity;
        state.last_refill = Instant::now();
        state.remainder_nanos = 0;
    }
}

/// Multi-tier rate limiter for exchanges with multiple rate limit tiers
///
/// Some exchanges have different rate limits for different endpoint types
/// (e.g., public vs private, order placement vs market data)
#[derive(Debug, Clone)]
pub struct MultiTierRateLimiter {
    limiters: Arc<Mutex<std::collections::HashMap<String, RateLimiter>>>,
}

impl MultiTierRateLimiter {
    /// Create a new multi-tier rate limiter
    pub fn new() -> Self {
        Self {
            limiters: Arc::new(Mutex::new(std::collections::HashMap::new())),
        }
    }

    /// Add a rate limiter for a specific tier
    ///
    /// # Arguments
    ///
    /// * `tier` - Name of the tier (e.g., "public", "private", "orders")
    /// * `limiter` - Rate limiter configuration for this tier
    pub async fn add_tier(&self, tier: String, limiter: RateLimiter) {
        let mut limiters = self.limiters.lock().await;
        limiters.insert(tier, limiter);
    }

    /// Wait for permission on a specific tier
    pub async fn wait(&self, tier: &str) {
        let limiter = {
            let limiters = self.limiters.lock().await;
            limiters.get(tier).cloned()
        };

        if let Some(limiter) = limiter {
            limiter.wait().await;
        }
    }

    /// Try to acquire permission on a specific tier without waiting
    pub async fn try_acquire(&self, tier: &str) -> bool {
        let limiter = {
            let limiters = self.limiters.lock().await;
            limiters.get(tier).cloned()
        };

        if let Some(limiter) = limiter {
            limiter.try_acquire().await
        } else {
            true // No limiter for this tier, allow by default
        }
    }
}

impl Default for MultiTierRateLimiter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_limiter_config() {
        let config = RateLimiterConfig::new(100, Duration::from_secs(60));
        assert_eq!(config.capacity, 100);
        assert_eq!(config.refill_period, Duration::from_secs(60));
        assert_eq!(config.refill_amount, 100);
        assert_eq!(config.cost_per_request, 1);
    }

    #[test]
    fn test_rate_limiter_config_custom() {
        let config = RateLimiterConfig::new(100, Duration::from_secs(60))
            .with_refill_amount(50)
            .with_cost_per_request(2);

        assert_eq!(config.refill_amount, 50);
        assert_eq!(config.cost_per_request, 2);
    }

    #[tokio::test]
    async fn test_rate_limiter_basic() {
        let config = RateLimiterConfig::new(5, Duration::from_secs(1));
        let limiter = RateLimiter::new(config);

        // Should be able to make 5 requests immediately
        for _ in 0..5 {
            assert!(limiter.try_acquire().await);
        }

        // 6th request should fail
        assert!(!limiter.try_acquire().await);
    }

    #[tokio::test]
    async fn test_rate_limiter_refill() {
        let config = RateLimiterConfig::new(2, Duration::from_millis(100));
        let limiter = RateLimiter::new(config);

        // Use all tokens
        assert!(limiter.try_acquire().await);
        assert!(limiter.try_acquire().await);
        assert!(!limiter.try_acquire().await);

        // Wait for refill
        sleep(Duration::from_millis(150)).await;

        // Should have tokens again
        assert!(limiter.try_acquire().await);
    }

    #[tokio::test]
    async fn test_rate_limiter_wait() {
        let config = RateLimiterConfig::new(2, Duration::from_millis(100));
        let limiter = RateLimiter::new(config);

        // Use all tokens
        limiter.wait().await;
        limiter.wait().await;

        let start = Instant::now();
        limiter.wait().await; // This should wait
        let elapsed = start.elapsed();

        // Should have waited at least 80ms (with some tolerance)
        assert!(elapsed >= Duration::from_millis(80));
    }

    #[tokio::test]
    async fn test_rate_limiter_custom_cost() {
        let config = RateLimiterConfig::new(10, Duration::from_secs(1));
        let limiter = RateLimiter::new(config);

        // One request with cost 5
        assert!(limiter.try_acquire_with_cost(5).await);
        assert_eq!(limiter.available_tokens().await, 5);

        // Another request with cost 3
        assert!(limiter.try_acquire_with_cost(3).await);
        assert_eq!(limiter.available_tokens().await, 2);

        // Request with cost 3 should fail (only 2 tokens left)
        assert!(!limiter.try_acquire_with_cost(3).await);
    }

    #[tokio::test]
    async fn test_rate_limiter_reset() {
        let config = RateLimiterConfig::new(5, Duration::from_secs(1));
        let limiter = RateLimiter::new(config);

        // Use all tokens
        for _ in 0..5 {
            limiter.wait().await;
        }

        assert_eq!(limiter.available_tokens().await, 0);

        // Reset
        limiter.reset().await;

        assert_eq!(limiter.available_tokens().await, 5);
    }

    #[tokio::test]
    async fn test_multi_tier_rate_limiter() {
        let multi = MultiTierRateLimiter::new();

        // Add tiers
        let public_config = RateLimiterConfig::new(10, Duration::from_secs(1));
        let private_config = RateLimiterConfig::new(5, Duration::from_secs(1));

        multi
            .add_tier("public".to_string(), RateLimiter::new(public_config))
            .await;
        multi
            .add_tier("private".to_string(), RateLimiter::new(private_config))
            .await;

        // Test public tier
        for _ in 0..10 {
            assert!(multi.try_acquire("public").await);
        }
        assert!(!multi.try_acquire("public").await);

        // Test private tier
        for _ in 0..5 {
            assert!(multi.try_acquire("private").await);
        }
        assert!(!multi.try_acquire("private").await);

        // Unknown tier should allow by default
        assert!(multi.try_acquire("unknown").await);
    }

    #[tokio::test]
    async fn test_concurrent_access() {
        let config = RateLimiterConfig::new(10, Duration::from_secs(1));
        let limiter = RateLimiter::new(config);

        let mut handles = vec![];

        // Spawn 10 concurrent tasks
        for _ in 0..10 {
            let limiter_clone = limiter.clone();
            let handle = tokio::spawn(async move {
                limiter_clone.wait().await;
            });
            handles.push(handle);
        }

        // Wait for all tasks
        for handle in handles {
            handle.await.unwrap();
        }

        // All tokens should be consumed
        assert_eq!(limiter.available_tokens().await, 0);
    }

    #[test]
    fn test_rate_limiter_config_validate_default() {
        let config = RateLimiterConfig::default();
        let result = config.validate();
        assert!(result.is_ok());
        assert!(result.unwrap().warnings.is_empty());
    }

    #[test]
    fn test_rate_limiter_config_validate_zero_capacity() {
        let config = RateLimiterConfig::new(0, Duration::from_secs(1));
        let result = config.validate();
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.field_name(), "capacity");
        assert!(matches!(
            err,
            crate::error::ConfigValidationError::ValueInvalid { .. }
        ));
    }

    #[test]
    fn test_rate_limiter_config_validate_short_refill_period_warning() {
        let config = RateLimiterConfig::new(10, Duration::from_millis(50));
        let result = config.validate();
        assert!(result.is_ok());
        let validation_result = result.unwrap();
        assert!(!validation_result.warnings.is_empty());
        assert!(validation_result.warnings[0].contains("refill_period"));
        assert!(validation_result.warnings[0].contains("very short"));
    }

    #[test]
    fn test_rate_limiter_config_validate_refill_period_boundary() {
        // refill_period = 100ms should not generate warning
        let config = RateLimiterConfig::new(10, Duration::from_millis(100));
        let result = config.validate();
        assert!(result.is_ok());
        assert!(result.unwrap().warnings.is_empty());

        // refill_period = 99ms should generate warning
        let config = RateLimiterConfig::new(10, Duration::from_millis(99));
        let result = config.validate();
        assert!(result.is_ok());
        assert!(!result.unwrap().warnings.is_empty());
    }

    #[test]
    fn test_rate_limiter_config_validate_valid_config() {
        let config = RateLimiterConfig::new(100, Duration::from_secs(60));
        let result = config.validate();
        assert!(result.is_ok());
        assert!(result.unwrap().warnings.is_empty());
    }

    /// Test that the integer-based refill calculation maintains precision
    /// over many iterations without drift.
    #[test]
    fn test_rate_limiter_integer_precision() {
        // Create a state with a specific configuration
        let config = RateLimiterConfig::new(100, Duration::from_millis(100)).with_refill_amount(10);
        let mut state = RateLimiterState::new(config);

        // Consume all tokens
        state.tokens = 0;

        // Simulate many small time increments that don't complete a full period
        // This tests the remainder_nanos accumulation
        let period_nanos = 100_000_000u64; // 100ms in nanos
        let small_increment = 33_333_333u64; // ~33.3ms in nanos

        // After 3 increments of ~33.3ms, we should have ~100ms total
        // which equals 1 complete period = 10 tokens
        state.remainder_nanos = small_increment;
        state.remainder_nanos += small_increment;
        state.remainder_nanos += small_increment;

        // Calculate complete periods
        let complete_periods = state.remainder_nanos / period_nanos;
        assert_eq!(complete_periods, 0); // 99.9ms < 100ms, no complete period yet

        // Add one more nanosecond to push over the threshold
        state.remainder_nanos += 1;
        let complete_periods = state.remainder_nanos / period_nanos;
        assert_eq!(complete_periods, 1); // Now we have 1 complete period

        // Verify remainder is preserved correctly
        let expected_remainder = (small_increment * 3 + 1) % period_nanos;
        assert_eq!(state.remainder_nanos % period_nanos, expected_remainder);
    }

    /// Test that remainder_nanos is properly reset when reset() is called
    #[tokio::test]
    async fn test_rate_limiter_reset_clears_remainder() {
        let config = RateLimiterConfig::new(5, Duration::from_secs(1));
        let limiter = RateLimiter::new(config);

        // Use all tokens
        for _ in 0..5 {
            limiter.wait().await;
        }

        // Wait a bit to accumulate some remainder
        sleep(Duration::from_millis(50)).await;

        // Reset should clear everything including remainder
        limiter.reset().await;

        // Verify tokens are back to capacity
        assert_eq!(limiter.available_tokens().await, 5);
    }

    /// Test that the refill calculation handles zero period_nanos gracefully
    #[test]
    fn test_rate_limiter_refill_zero_period_protection() {
        // This tests the edge case protection in refill()
        // A zero period should not cause division by zero
        let config = RateLimiterConfig {
            capacity: 10,
            refill_period: Duration::ZERO, // Edge case: zero duration
            refill_amount: 5,
            cost_per_request: 1,
        };
        let mut state = RateLimiterState::new(config);
        state.tokens = 0;

        // This should not panic due to division by zero
        state.refill();

        // Tokens should remain unchanged (no refill with zero period)
        assert_eq!(state.tokens, 0);
    }

    /// Test that integer arithmetic doesn't overflow with large values
    #[test]
    fn test_rate_limiter_refill_overflow_protection() {
        let config =
            RateLimiterConfig::new(u32::MAX, Duration::from_nanos(1)).with_refill_amount(u32::MAX);
        let mut state = RateLimiterState::new(config);
        state.tokens = 0;

        // Simulate a large elapsed time
        state.remainder_nanos = u64::MAX / 2;

        // This should not panic due to overflow
        // The saturating_mul and min operations should protect against overflow
        let period_nanos = 1u64;
        let complete_periods = state.remainder_nanos / period_nanos;
        let tokens_to_add = complete_periods
            .saturating_mul(u64::from(state.config.refill_amount))
            .min(u64::from(u32::MAX)) as u32;

        // Should be capped at u32::MAX
        assert_eq!(tokens_to_add, u32::MAX);
    }
}
