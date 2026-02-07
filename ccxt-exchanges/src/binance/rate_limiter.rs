//! Binance rate limiter implementation.
//!
//! This module provides weight-based rate limiting for Binance API requests.
//! Binance uses a weight system where different endpoints consume different
//! amounts of weight, and there are separate limits for different categories.
//!
//! # Rate Limit Categories
//!
//! - **Request Weight**: General API request limit (default: 1200/minute for spot)
//! - **Order Count**: Order placement limit (default: 10 orders/second, 100000/day)
//! - **Raw Requests**: Raw request count limit
//!
//! # Response Headers
//!
//! Binance returns rate limit information in response headers:
//! - `X-MBX-USED-WEIGHT-*`: Current used weight
//! - `X-MBX-ORDER-COUNT-*`: Current order count
//! - `Retry-After`: Seconds to wait when rate limited
//!
//! # Example
//!
//! ```rust
//! use ccxt_exchanges::binance::rate_limiter::{WeightRateLimiter, RateLimitInfo};
//!
//! let limiter = WeightRateLimiter::new();
//!
//! // Update from response headers
//! let info = RateLimitInfo {
//!     used_weight_1m: Some(500),
//!     order_count_10s: Some(5),
//!     order_count_1d: Some(1000),
//!     retry_after: None,
//! };
//! limiter.update(info);
//!
//! // Check if we should throttle
//! if limiter.should_throttle() {
//!     // Wait before making more requests
//! }
//! ```

use std::sync::RwLock;
use std::sync::atomic::{AtomicI64, AtomicU64, Ordering};
use std::time::{Duration, Instant};

/// Default weight limit per minute for spot API.
pub const DEFAULT_WEIGHT_LIMIT_1M: u64 = 1200;

/// Default order count limit per 10 seconds.
pub const DEFAULT_ORDER_LIMIT_10S: u64 = 100;

/// Default order count limit per day.
pub const DEFAULT_ORDER_LIMIT_1D: u64 = 200000;

/// Threshold percentage at which to start throttling (80%).
pub const THROTTLE_THRESHOLD: f64 = 0.80;

/// Rate limit information extracted from response headers.
#[derive(Debug, Clone, Default)]
pub struct RateLimitInfo {
    /// Used weight in the current 1-minute window.
    pub used_weight_1m: Option<u64>,
    /// Order count in the current 10-second window.
    pub order_count_10s: Option<u64>,
    /// Order count in the current day.
    pub order_count_1d: Option<u64>,
    /// Retry-After header value in seconds (when rate limited).
    pub retry_after: Option<u64>,
}

impl RateLimitInfo {
    /// Creates a new `RateLimitInfo` from response headers.
    ///
    /// # Arguments
    ///
    /// * `headers` - JSON object containing response headers
    ///
    /// # Returns
    ///
    /// Returns a `RateLimitInfo` with values extracted from headers.
    pub fn from_headers(headers: &serde_json::Value) -> Self {
        let mut info = Self::default();

        if let Some(obj) = headers.as_object() {
            // Extract X-MBX-USED-WEIGHT-1M (also fallback to x-mbx-used-weight without -1m suffix,
            // and x-sapi-used-uid-weight-1m for SAPI endpoints)
            if let Some(weight) = obj
                .get("x-mbx-used-weight-1m")
                .or_else(|| obj.get("X-MBX-USED-WEIGHT-1M"))
                .or_else(|| obj.get("x-mbx-used-weight"))
                .or_else(|| obj.get("X-MBX-USED-WEIGHT"))
                .or_else(|| obj.get("x-sapi-used-uid-weight-1m"))
                .or_else(|| obj.get("X-SAPI-USED-UID-WEIGHT-1M"))
                .and_then(|v| v.as_str())
                .and_then(|s| s.parse::<u64>().ok())
            {
                info.used_weight_1m = Some(weight);
            }

            // Extract X-MBX-ORDER-COUNT-10S
            if let Some(count) = obj
                .get("x-mbx-order-count-10s")
                .or_else(|| obj.get("X-MBX-ORDER-COUNT-10S"))
                .and_then(|v| v.as_str())
                .and_then(|s| s.parse::<u64>().ok())
            {
                info.order_count_10s = Some(count);
            }

            // Extract X-MBX-ORDER-COUNT-1D
            if let Some(count) = obj
                .get("x-mbx-order-count-1d")
                .or_else(|| obj.get("X-MBX-ORDER-COUNT-1D"))
                .and_then(|v| v.as_str())
                .and_then(|s| s.parse::<u64>().ok())
            {
                info.order_count_1d = Some(count);
            }

            // Extract Retry-After
            if let Some(retry) = obj
                .get("retry-after")
                .or_else(|| obj.get("Retry-After"))
                .and_then(|v| v.as_str())
                .and_then(|s| s.parse::<u64>().ok())
            {
                info.retry_after = Some(retry);
            }
        }

        info
    }

    /// Returns true if any rate limit information is present.
    pub fn has_data(&self) -> bool {
        self.used_weight_1m.is_some()
            || self.order_count_10s.is_some()
            || self.order_count_1d.is_some()
            || self.retry_after.is_some()
    }
}

/// Weight-based rate limiter for Binance API.
///
/// Tracks API usage based on response headers and provides throttling
/// recommendations to avoid hitting rate limits.
#[derive(Debug)]
pub struct WeightRateLimiter {
    /// Current used weight (1-minute window).
    used_weight_1m: AtomicU64,
    /// Current order count (10-second window).
    order_count_10s: AtomicU64,
    /// Current order count (daily).
    order_count_1d: AtomicU64,
    /// Weight limit per minute.
    weight_limit_1m: AtomicU64,
    /// Order limit per 10 seconds.
    order_limit_10s: AtomicU64,
    /// Order limit per day.
    order_limit_1d: AtomicU64,
    /// Retry-After timestamp (when we can resume requests).
    retry_after_until: RwLock<Option<Instant>>,
    /// Last update timestamp.
    last_update: RwLock<Option<Instant>>,
    /// IP ban detection timestamp.
    ip_banned_until: AtomicI64,
}

impl Default for WeightRateLimiter {
    fn default() -> Self {
        Self::new()
    }
}

impl WeightRateLimiter {
    /// Creates a new rate limiter with default limits.
    pub fn new() -> Self {
        Self {
            used_weight_1m: AtomicU64::new(0),
            order_count_10s: AtomicU64::new(0),
            order_count_1d: AtomicU64::new(0),
            weight_limit_1m: AtomicU64::new(DEFAULT_WEIGHT_LIMIT_1M),
            order_limit_10s: AtomicU64::new(DEFAULT_ORDER_LIMIT_10S),
            order_limit_1d: AtomicU64::new(DEFAULT_ORDER_LIMIT_1D),
            retry_after_until: RwLock::new(None),
            last_update: RwLock::new(None),
            ip_banned_until: AtomicI64::new(0),
        }
    }

    /// Creates a new rate limiter with custom limits.
    ///
    /// # Arguments
    ///
    /// * `weight_limit_1m` - Weight limit per minute
    /// * `order_limit_10s` - Order limit per 10 seconds
    /// * `order_limit_1d` - Order limit per day
    pub fn with_limits(weight_limit_1m: u64, order_limit_10s: u64, order_limit_1d: u64) -> Self {
        Self {
            used_weight_1m: AtomicU64::new(0),
            order_count_10s: AtomicU64::new(0),
            order_count_1d: AtomicU64::new(0),
            weight_limit_1m: AtomicU64::new(weight_limit_1m),
            order_limit_10s: AtomicU64::new(order_limit_10s),
            order_limit_1d: AtomicU64::new(order_limit_1d),
            retry_after_until: RwLock::new(None),
            last_update: RwLock::new(None),
            ip_banned_until: AtomicI64::new(0),
        }
    }

    /// Updates the rate limiter with information from response headers.
    ///
    /// # Arguments
    ///
    /// * `info` - Rate limit information extracted from headers
    pub fn update(&self, info: RateLimitInfo) {
        if let Some(weight) = info.used_weight_1m {
            self.used_weight_1m.store(weight, Ordering::SeqCst);
        }

        if let Some(count) = info.order_count_10s {
            self.order_count_10s.store(count, Ordering::SeqCst);
        }

        if let Some(count) = info.order_count_1d {
            self.order_count_1d.store(count, Ordering::SeqCst);
        }

        if let Some(retry_secs) = info.retry_after {
            let until = Instant::now() + Duration::from_secs(retry_secs);
            if let Ok(mut guard) = self.retry_after_until.write() {
                *guard = Some(until);
            }
        }

        if let Ok(mut guard) = self.last_update.write() {
            *guard = Some(Instant::now());
        }
    }

    /// Sets the IP ban duration.
    ///
    /// # Arguments
    ///
    /// * `duration` - Duration of the IP ban
    pub fn set_ip_banned(&self, duration: Duration) {
        let until = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64
            + duration.as_secs() as i64;
        self.ip_banned_until.store(until, Ordering::SeqCst);
    }

    /// Returns true if the IP is currently banned.
    pub fn is_ip_banned(&self) -> bool {
        let banned_until = self.ip_banned_until.load(Ordering::SeqCst);
        if banned_until == 0 {
            return false;
        }

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        now < banned_until
    }

    /// Returns true if we should throttle requests.
    ///
    /// Throttling is recommended when:
    /// - We're in a Retry-After period
    /// - Weight usage exceeds 80% of the limit
    /// - Order count exceeds 80% of the limit
    /// - IP is banned
    pub fn should_throttle(&self) -> bool {
        // Check IP ban
        if self.is_ip_banned() {
            return true;
        }

        // Check Retry-After
        if let Ok(guard) = self.retry_after_until.read() {
            if let Some(until) = *guard {
                if Instant::now() < until {
                    return true;
                }
            }
        }

        // Check weight threshold (80%)
        let weight = self.used_weight_1m.load(Ordering::SeqCst);
        let weight_limit = self.weight_limit_1m.load(Ordering::SeqCst);
        #[allow(clippy::cast_precision_loss)]
        if (weight as f64) >= (weight_limit as f64) * THROTTLE_THRESHOLD {
            return true;
        }

        // Check order count threshold (80%)
        let order_count = self.order_count_10s.load(Ordering::SeqCst);
        let order_limit = self.order_limit_10s.load(Ordering::SeqCst);
        if order_count as f64 >= order_limit as f64 * THROTTLE_THRESHOLD {
            return true;
        }

        false
    }

    /// Returns the recommended wait duration before making the next request.
    ///
    /// Returns `None` if no waiting is needed.
    pub fn wait_duration(&self) -> Option<Duration> {
        // Check IP ban first
        if self.is_ip_banned() {
            let banned_until = self.ip_banned_until.load(Ordering::SeqCst);
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as i64;
            if banned_until > now {
                return Some(Duration::from_secs((banned_until - now) as u64));
            }
        }

        // Check Retry-After
        if let Ok(guard) = self.retry_after_until.read() {
            if let Some(until) = *guard {
                let now = Instant::now();
                if until > now {
                    return Some(until - now);
                }
            }
        }

        // If we're at the threshold, suggest a small wait
        if self.should_throttle() {
            // Wait until the next minute window resets (conservative estimate)
            return Some(Duration::from_secs(1));
        }

        None
    }

    /// Returns the current used weight.
    pub fn used_weight(&self) -> u64 {
        self.used_weight_1m.load(Ordering::SeqCst)
    }

    /// Returns the weight limit.
    pub fn weight_limit(&self) -> u64 {
        self.weight_limit_1m.load(Ordering::SeqCst)
    }

    /// Returns the current order count (10s window).
    pub fn order_count_10s(&self) -> u64 {
        self.order_count_10s.load(Ordering::SeqCst)
    }

    /// Returns the current order count (daily).
    pub fn order_count_1d(&self) -> u64 {
        self.order_count_1d.load(Ordering::SeqCst)
    }

    /// Returns the order limit per day.
    pub fn order_limit_1d(&self) -> u64 {
        self.order_limit_1d.load(Ordering::SeqCst)
    }

    /// Returns the weight usage percentage (0.0 to 1.0).
    pub fn weight_usage_ratio(&self) -> f64 {
        let weight = self.used_weight_1m.load(Ordering::SeqCst) as f64;
        let limit = self.weight_limit_1m.load(Ordering::SeqCst) as f64;
        if limit > 0.0 { weight / limit } else { 0.0 }
    }

    /// Resets all counters.
    ///
    /// This should be called when the rate limit window resets.
    pub fn reset(&self) {
        self.used_weight_1m.store(0, Ordering::SeqCst);
        self.order_count_10s.store(0, Ordering::SeqCst);
        self.order_count_1d.store(0, Ordering::SeqCst);
        if let Ok(mut guard) = self.retry_after_until.write() {
            *guard = None;
        }
        self.ip_banned_until.store(0, Ordering::SeqCst);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_limiter_new() {
        let limiter = WeightRateLimiter::new();
        assert_eq!(limiter.used_weight(), 0);
        assert_eq!(limiter.weight_limit(), DEFAULT_WEIGHT_LIMIT_1M);
        assert!(!limiter.should_throttle());
    }

    #[test]
    fn test_rate_limiter_with_limits() {
        let limiter = WeightRateLimiter::with_limits(2400, 200, 400000);
        assert_eq!(limiter.weight_limit(), 2400);
    }

    #[test]
    fn test_rate_limiter_update() {
        let limiter = WeightRateLimiter::new();

        let info = RateLimitInfo {
            used_weight_1m: Some(500),
            order_count_10s: Some(5),
            order_count_1d: Some(1000),
            retry_after: None,
        };

        limiter.update(info);

        assert_eq!(limiter.used_weight(), 500);
        assert_eq!(limiter.order_count_10s(), 5);
        assert_eq!(limiter.order_count_1d(), 1000);
    }

    #[test]
    fn test_rate_limiter_throttle_at_threshold() {
        let limiter = WeightRateLimiter::new();

        // Set weight to 80% of limit
        let threshold_weight = (DEFAULT_WEIGHT_LIMIT_1M as f64 * THROTTLE_THRESHOLD) as u64;
        let info = RateLimitInfo {
            used_weight_1m: Some(threshold_weight),
            ..Default::default()
        };

        limiter.update(info);
        assert!(limiter.should_throttle());
    }

    #[test]
    fn test_rate_limiter_no_throttle_below_threshold() {
        let limiter = WeightRateLimiter::new();

        // Set weight to 50% of limit
        let info = RateLimitInfo {
            used_weight_1m: Some(DEFAULT_WEIGHT_LIMIT_1M / 2),
            ..Default::default()
        };

        limiter.update(info);
        assert!(!limiter.should_throttle());
    }

    #[test]
    fn test_rate_limiter_retry_after() {
        let limiter = WeightRateLimiter::new();

        let info = RateLimitInfo {
            retry_after: Some(5),
            ..Default::default()
        };

        limiter.update(info);
        assert!(limiter.should_throttle());
        assert!(limiter.wait_duration().is_some());
    }

    #[test]
    fn test_rate_limiter_ip_banned() {
        let limiter = WeightRateLimiter::new();

        limiter.set_ip_banned(Duration::from_secs(60));
        assert!(limiter.is_ip_banned());
        assert!(limiter.should_throttle());
    }

    #[test]
    fn test_rate_limiter_reset() {
        let limiter = WeightRateLimiter::new();

        let info = RateLimitInfo {
            used_weight_1m: Some(1000),
            order_count_10s: Some(50),
            order_count_1d: Some(5000),
            retry_after: Some(10),
        };

        limiter.update(info);
        limiter.set_ip_banned(Duration::from_secs(60));

        limiter.reset();

        assert_eq!(limiter.used_weight(), 0);
        assert_eq!(limiter.order_count_10s(), 0);
        assert_eq!(limiter.order_count_1d(), 0);
        assert!(!limiter.is_ip_banned());
        assert!(!limiter.should_throttle());
    }

    #[test]
    fn test_rate_limit_info_from_headers() {
        let headers = serde_json::json!({
            "x-mbx-used-weight-1m": "500",
            "x-mbx-order-count-10s": "5",
            "x-mbx-order-count-1d": "1000",
            "retry-after": "30"
        });

        let info = RateLimitInfo::from_headers(&headers);

        assert_eq!(info.used_weight_1m, Some(500));
        assert_eq!(info.order_count_10s, Some(5));
        assert_eq!(info.order_count_1d, Some(1000));
        assert_eq!(info.retry_after, Some(30));
    }

    #[test]
    fn test_rate_limit_info_from_headers_uppercase() {
        let headers = serde_json::json!({
            "X-MBX-USED-WEIGHT-1M": "600",
            "X-MBX-ORDER-COUNT-10S": "10",
            "Retry-After": "60"
        });

        let info = RateLimitInfo::from_headers(&headers);

        assert_eq!(info.used_weight_1m, Some(600));
        assert_eq!(info.order_count_10s, Some(10));
        assert_eq!(info.retry_after, Some(60));
    }

    #[test]
    fn test_rate_limit_info_has_data() {
        let empty = RateLimitInfo::default();
        assert!(!empty.has_data());

        let with_weight = RateLimitInfo {
            used_weight_1m: Some(100),
            ..Default::default()
        };
        assert!(with_weight.has_data());
    }

    #[test]
    fn test_weight_usage_ratio() {
        let limiter = WeightRateLimiter::new();

        // 50% usage
        let info = RateLimitInfo {
            used_weight_1m: Some(DEFAULT_WEIGHT_LIMIT_1M / 2),
            ..Default::default()
        };
        limiter.update(info);

        let ratio = limiter.weight_usage_ratio();
        assert!((ratio - 0.5).abs() < 0.01);
    }
}
