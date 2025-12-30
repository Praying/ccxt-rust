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

/// Internal state of the rate limiter
#[derive(Debug)]
struct RateLimiterState {
    /// Current number of available tokens
    tokens: u32,
    /// Last time tokens were refilled
    last_refill: Instant,
    /// Configuration
    config: RateLimiterConfig,
}

impl RateLimiterState {
    fn new(config: RateLimiterConfig) -> Self {
        Self {
            tokens: config.capacity,
            last_refill: Instant::now(),
            config,
        }
    }

    /// Refill tokens based on elapsed time
    fn refill(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill);

        if elapsed >= self.config.refill_period {
            // Calculate how many refill periods have passed
            let periods = elapsed.as_secs_f64() / self.config.refill_period.as_secs_f64();
            #[allow(clippy::cast_possible_truncation)]
            let tokens_to_add = (periods * self.config.refill_amount as f64) as u32;

            // Add tokens up to capacity
            self.tokens = (self.tokens + tokens_to_add).min(self.config.capacity);
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
            self.config.refill_amount as f64 / self.config.refill_period.as_secs_f64();
        let wait_seconds = tokens_needed as f64 / refill_rate;

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

    /// Create a rate limiter with default configuration (10 req/sec)
    pub fn default() -> Self {
        Self::new(RateLimiterConfig::default())
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
}
