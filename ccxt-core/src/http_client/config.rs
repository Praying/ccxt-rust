use crate::circuit_breaker::CircuitBreakerConfig;
use crate::config::ProxyConfig;
use crate::error::{ConfigValidationError, ValidationResult};
use crate::retry_strategy::RetryConfig;
use std::time::Duration;

/// HTTP request configuration
#[derive(Debug, Clone)]
pub struct HttpConfig {
    /// Request timeout
    pub timeout: Duration,
    /// TCP connection timeout (default: 10 seconds)
    pub connect_timeout: Duration,
    /// Maximum retry attempts (deprecated, use `retry_config` instead)
    #[deprecated(note = "Use retry_config instead")]
    pub max_retries: u32,
    /// Whether to enable verbose logging
    pub verbose: bool,
    /// Default User-Agent header value
    pub user_agent: String,
    /// Whether to include response headers in the result
    pub return_response_headers: bool,
    /// Optional proxy configuration
    pub proxy: Option<ProxyConfig>,
    /// Whether to enable rate limiting
    pub enable_rate_limit: bool,
    /// Optional retry configuration (uses default if `None`)
    pub retry_config: Option<RetryConfig>,
    /// Maximum response body size in bytes (default: 10MB)
    ///
    /// Responses exceeding this limit will be rejected with an `InvalidRequest` error.
    /// This protects against malicious or abnormal responses that could exhaust memory.
    pub max_response_size: usize,

    /// Maximum request body size in bytes (default: 10MB)
    ///
    /// Request bodies exceeding this limit will be rejected BEFORE serialization.
    /// This protects against DoS attacks via oversized request payloads.
    pub max_request_size: usize,

    /// Optional circuit breaker configuration.
    ///
    /// When enabled, the circuit breaker will track request failures and
    /// automatically block requests to failing endpoints, allowing the
    /// system to recover.
    ///
    /// Default: `None` (disabled for backward compatibility)
    pub circuit_breaker: Option<CircuitBreakerConfig>,

    /// Maximum number of idle connections per host in the connection pool.
    ///
    /// This controls how many keep-alive connections are maintained for each host.
    /// Higher values improve performance for repeated requests to the same host
    /// but consume more resources.
    ///
    /// Default: 10
    pub pool_max_idle_per_host: usize,

    /// Timeout for idle connections in the pool.
    ///
    /// Connections that have been idle longer than this duration will be closed.
    /// This helps free up resources and avoid stale connections.
    ///
    /// Default: 90 seconds
    pub pool_idle_timeout: Duration,
}

impl Default for HttpConfig {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(30),
            connect_timeout: Duration::from_secs(10),
            #[allow(deprecated)]
            max_retries: 3,
            verbose: false,
            user_agent: "ccxt-rust/1.0".to_string(),
            return_response_headers: false,
            proxy: None,
            enable_rate_limit: true,
            retry_config: None,
            max_response_size: 10 * 1024 * 1024,
            max_request_size: 10 * 1024 * 1024,
            circuit_breaker: None,
            pool_max_idle_per_host: 10,
            pool_idle_timeout: Duration::from_secs(90),
        }
    }
}

impl HttpConfig {
    /// Validates the HTTP configuration parameters.
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
    /// - `timeout` > 5 minutes returns an error (excessive timeout)
    /// - `timeout` < 1 second generates a warning (may cause frequent timeouts)
    ///
    /// # Example
    ///
    /// ```rust
    /// use ccxt_core::http_client::HttpConfig;
    /// use std::time::Duration;
    ///
    /// let config = HttpConfig::default();
    /// let result = config.validate();
    /// assert!(result.is_ok());
    ///
    /// let invalid_config = HttpConfig {
    ///     timeout: Duration::from_secs(600), // 10 minutes - too long
    ///     ..Default::default()
    /// };
    /// let result = invalid_config.validate();
    /// assert!(result.is_err());
    /// ```
    pub fn validate(&self) -> std::result::Result<ValidationResult, ConfigValidationError> {
        const MAX_REASONABLE_REQUEST_SIZE: usize = 100 * 1024 * 1024;

        let mut warnings = Vec::new();
        let max_timeout = Duration::from_secs(300);
        if self.timeout > max_timeout {
            return Err(ConfigValidationError::too_high(
                "timeout",
                format!("{:?}", self.timeout),
                "5 minutes",
            ));
        }

        if self.timeout < Duration::from_secs(1) {
            warnings.push(format!(
                "timeout {:?} is very short, may cause frequent timeouts",
                self.timeout
            ));
        }

        if self.max_request_size == 0 {
            return Err(ConfigValidationError::invalid(
                "max_request_size",
                "max_request_size cannot be zero",
            ));
        }

        if self.max_request_size > MAX_REASONABLE_REQUEST_SIZE {
            return Err(ConfigValidationError::too_high(
                "max_request_size",
                format!("{}", self.max_request_size),
                "100MB (104857600 bytes)",
            ));
        }

        Ok(ValidationResult::with_warnings(warnings))
    }
}
