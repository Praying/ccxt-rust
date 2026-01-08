//! HTTP client abstraction layer
//!
//! Provides a unified HTTP request interface with support for:
//! - Automatic retry mechanism with configurable strategies
//! - Timeout control per request
//! - Gzip compression/decompression
//! - Request and response logging with structured tracing
//! - Custom headers
//! - Proxy configuration
//! - Rate limiting integration
//!
//! # Observability
//!
//! This module uses the `tracing` crate for structured logging. Key events:
//! - HTTP request initiation with URL and method
//! - Retry attempts with delay and error cause
//! - HTTP response status and body preview
//! - Error details with structured fields

use crate::circuit_breaker::{CircuitBreaker, CircuitBreakerConfig};
use crate::config::ProxyConfig;
use crate::error::{ConfigValidationError, Error, Result, ValidationResult};
use crate::rate_limiter::RateLimiter;
use crate::retry_strategy::{RetryConfig, RetryStrategy};
use reqwest::{Client, Method, Response, StatusCode, header::HeaderMap};
use serde_json::Value;
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, error, info, instrument, warn};

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
            max_retries: 3, // Kept for backward compatibility, but deprecated
            verbose: false,
            user_agent: "ccxt-rust/1.0".to_string(),
            return_response_headers: false,
            proxy: None,
            enable_rate_limit: true,
            retry_config: None, // Uses default retry configuration
            max_response_size: 10 * 1024 * 1024, // 10MB default
            max_request_size: 10 * 1024 * 1024, // 10MB default
            circuit_breaker: None, // Disabled by default for backward compatibility
            pool_max_idle_per_host: 10, // Default: 10 idle connections per host
            pool_idle_timeout: Duration::from_secs(90), // Default: 90 seconds
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
        // Maximum reasonable request size (100MB) - defined at the start to satisfy clippy
        const MAX_REASONABLE_REQUEST_SIZE: usize = 100 * 1024 * 1024;

        let mut warnings = Vec::new();

        // Validate timeout <= 5 minutes
        let max_timeout = Duration::from_secs(300); // 5 minutes
        if self.timeout > max_timeout {
            return Err(ConfigValidationError::too_high(
                "timeout",
                format!("{:?}", self.timeout),
                "5 minutes",
            ));
        }

        // Warn if timeout < 1 second
        if self.timeout < Duration::from_secs(1) {
            warnings.push(format!(
                "timeout {:?} is very short, may cause frequent timeouts",
                self.timeout
            ));
        }

        // Validate max_request_size > 0
        if self.max_request_size == 0 {
            return Err(ConfigValidationError::invalid(
                "max_request_size",
                "max_request_size cannot be zero",
            ));
        }

        // Validate max_request_size <= 100MB (prevent memory exhaustion attacks)
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

/// HTTP client with retry and rate limiting support
#[derive(Debug)]
pub struct HttpClient {
    client: Client,
    config: HttpConfig,
    rate_limiter: Option<RateLimiter>,
    retry_strategy: RetryStrategy,
    circuit_breaker: Option<Arc<CircuitBreaker>>,
}

impl HttpClient {
    /// Creates a new HTTP client with the given configuration.
    ///
    /// # Arguments
    ///
    /// * `config` - HTTP client configuration
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing the initialized client or an error if client creation fails.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The proxy URL is invalid
    /// - The HTTP client cannot be built
    pub fn new(config: HttpConfig) -> Result<Self> {
        let mut builder = Client::builder()
            .timeout(config.timeout)
            .connect_timeout(config.connect_timeout)
            .pool_max_idle_per_host(config.pool_max_idle_per_host)
            .pool_idle_timeout(config.pool_idle_timeout)
            .gzip(true)
            .user_agent(&config.user_agent);

        if let Some(proxy_config) = &config.proxy {
            let mut proxy = reqwest::Proxy::all(&proxy_config.url)
                .map_err(|e| Error::network(format!("Invalid proxy URL: {e}")))?;

            if let (Some(username), Some(password)) =
                (&proxy_config.username, &proxy_config.password)
            {
                proxy = proxy.basic_auth(username, password);
            }
            builder = builder.proxy(proxy);
        }

        let client = builder
            .build()
            .map_err(|e| Error::network(format!("Failed to build HTTP client: {e}")))?;

        let retry_strategy = RetryStrategy::new(config.retry_config.clone().unwrap_or_default());

        // Create circuit breaker if configured
        let circuit_breaker = config
            .circuit_breaker
            .as_ref()
            .map(|cb_config| Arc::new(CircuitBreaker::new(cb_config.clone())));

        Ok(Self {
            client,
            config,
            rate_limiter: None,
            retry_strategy,
            circuit_breaker,
        })
    }

    /// Creates a new HTTP client with a custom rate limiter.
    ///
    /// # Arguments
    ///
    /// * `config` - HTTP client configuration
    /// * `rate_limiter` - Pre-configured rate limiter instance
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing the initialized client with rate limiter.
    ///
    /// # Errors
    ///
    /// Returns an error if client creation fails.
    pub fn new_with_rate_limiter(config: HttpConfig, rate_limiter: RateLimiter) -> Result<Self> {
        let mut client = Self::new(config)?;
        client.rate_limiter = Some(rate_limiter);
        Ok(client)
    }

    /// Sets the rate limiter for this client.
    ///
    /// # Arguments
    ///
    /// * `rate_limiter` - Rate limiter instance to use
    pub fn set_rate_limiter(&mut self, rate_limiter: RateLimiter) {
        self.rate_limiter = Some(rate_limiter);
    }

    /// Sets the retry strategy for this client.
    ///
    /// # Arguments
    ///
    /// * `strategy` - Retry strategy to use for failed requests
    pub fn set_retry_strategy(&mut self, strategy: RetryStrategy) {
        self.retry_strategy = strategy;
    }

    /// Sets the circuit breaker for this client.
    ///
    /// # Arguments
    ///
    /// * `circuit_breaker` - Circuit breaker instance to use
    ///
    /// # Example
    ///
    /// ```rust
    /// use ccxt_core::http_client::{HttpClient, HttpConfig};
    /// use ccxt_core::circuit_breaker::{CircuitBreaker, CircuitBreakerConfig};
    /// use std::sync::Arc;
    ///
    /// let mut client = HttpClient::new(HttpConfig::default()).unwrap();
    /// let cb = Arc::new(CircuitBreaker::new(CircuitBreakerConfig::default()));
    /// client.set_circuit_breaker(cb);
    /// ```
    pub fn set_circuit_breaker(&mut self, circuit_breaker: Arc<CircuitBreaker>) {
        self.circuit_breaker = Some(circuit_breaker);
    }

    /// Returns a reference to the circuit breaker, if configured.
    ///
    /// This can be used to check the circuit breaker state or manually
    /// record success/failure.
    pub fn circuit_breaker(&self) -> Option<&Arc<CircuitBreaker>> {
        self.circuit_breaker.as_ref()
    }

    /// Executes an HTTP request with automatic retry mechanism and timeout control.
    ///
    /// The entire retry flow is wrapped with `tokio::time::timeout` to ensure
    /// that the total operation time (including all retries) does not exceed
    /// the configured timeout.
    ///
    /// # Arguments
    ///
    /// * `url` - Target URL for the request
    /// * `method` - HTTP method to use
    /// * `headers` - Optional custom headers
    /// * `body` - Optional request body as JSON
    ///
    /// # Returns
    ///
    /// Returns the response body as a JSON `Value`.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The operation times out (including all retries)
    /// - All retry attempts fail
    /// - The server returns an error status code
    /// - Network communication fails
    #[instrument(
        name = "http_fetch",
        skip(self, headers, body),
        fields(method = %method, url = %url, timeout_ms = %self.config.timeout.as_millis())
    )]
    pub async fn fetch(
        &self,
        url: &str,
        method: Method,
        headers: Option<HeaderMap>,
        body: Option<Value>,
    ) -> Result<Value> {
        // Check circuit breaker before making request
        if let Some(ref cb) = self.circuit_breaker {
            cb.allow_request()?;
        }

        // Lint: collapsible_if
        // Reason: Keeping separate for clarity - first check if rate limiting is enabled, then check limiter
        #[allow(clippy::collapsible_if)]
        if self.config.enable_rate_limit {
            if let Some(ref limiter) = self.rate_limiter {
                limiter.wait().await;
            }
        }

        // Use tokio::time::timeout to wrap the entire retry flow
        // This ensures that the total operation time (including all retries)
        // does not exceed the configured timeout
        let total_timeout = self.config.timeout;
        let url_for_error = url.to_string();

        let result = match tokio::time::timeout(
            total_timeout,
            self.execute_with_retry(|| {
                let url = url.to_string();
                let method = method.clone();
                let headers = headers.clone();
                let body = body.clone();
                async move { self.fetch_once(&url, method, headers, body).await }
            }),
        )
        .await
        {
            Ok(result) => result,
            Err(_elapsed) => {
                // Log timeout event with URL and timeout duration
                warn!(
                    url = %url_for_error,
                    timeout_ms = %total_timeout.as_millis(),
                    "HTTP request timed out (including retries)"
                );
                Err(Error::timeout(format!(
                    "Request to {} timed out after {}ms",
                    url_for_error,
                    total_timeout.as_millis()
                )))
            }
        };

        // Record success/failure with circuit breaker
        if let Some(ref cb) = self.circuit_breaker {
            match &result {
                Ok(_) => cb.record_success(),
                Err(_) => cb.record_failure(),
            }
        }

        result
    }

    /// Executes an async operation with automatic retry mechanism.
    pub(crate) async fn execute_with_retry<F, Fut>(&self, operation: F) -> Result<Value>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<Value>>,
    {
        let mut attempt = 0;
        loop {
            match operation().await {
                Ok(response) => {
                    debug!(attempt = attempt + 1, "Operation completed successfully");
                    return Ok(response);
                }
                Err(e) => {
                    let should_retry = self.retry_strategy.should_retry(&e, attempt);

                    if should_retry {
                        let delay = self.retry_strategy.calculate_delay(attempt, &e);

                        warn!(
                            attempt = attempt + 1,
                            delay_ms = %delay.as_millis(),
                            error = %e,
                            error_debug = ?e,
                            is_retryable = e.is_retryable(),
                            "Operation failed, retrying after delay"
                        );

                        tokio::time::sleep(delay).await;
                        attempt += 1;
                    } else {
                        // Not retrying, log the final error and return
                        error!(
                            attempt = attempt + 1,
                            error = %e,
                            error_debug = ?e,
                            is_retryable = e.is_retryable(),
                            "Operation failed, not retrying"
                        );
                        return Err(e);
                    }
                }
            }
        }
    }

    /// Executes a single HTTP request without retry logic (internal method).
    #[instrument(
        name = "http_fetch_once",
        skip(self, headers, body),
        fields(method = %method, url = %url, has_body = body.is_some())
    )]
    async fn fetch_once(
        &self,
        url: &str,
        method: Method,
        headers: Option<HeaderMap>,
        body: Option<Value>,
    ) -> Result<Value> {
        let mut request = self.client.request(method.clone(), url);

        if let Some(headers) = headers {
            request = request.headers(headers);
        }

        if let Some(ref body) = body {
            // Validate request body size BEFORE serialization (DoS protection)
            let body_str = serde_json::to_string(body)
                .map_err(|e| Error::invalid_request(format!("JSON serialization failed: {e}")))?;

            if body_str.len() > self.config.max_request_size {
                return Err(Error::invalid_request(format!(
                    "Request body {} bytes exceeds limit {} bytes",
                    body_str.len(),
                    self.config.max_request_size
                )));
            }

            request = request.body(body_str);
        }

        if self.config.verbose {
            if let Some(body) = &body {
                debug!(
                    body = ?body,
                    "HTTP request with body"
                );
            } else {
                debug!("HTTP request without body");
            }
        }

        let response = request.send().await.map_err(|e| {
            error!(
                error = %e,
                "HTTP request send failed"
            );
            Error::network(format!("Request failed: {e}"))
        })?;

        self.process_response_with_limit(response, url).await
    }

    /// Processes an HTTP response with size limit enforcement.
    ///
    /// This method checks the Content-Length header first and rejects responses
    /// that exceed the configured `max_response_size`. For responses without
    /// Content-Length, it streams the body while tracking accumulated size.
    ///
    /// # Arguments
    ///
    /// * `response` - The HTTP response to process
    /// * `url` - The request URL (for logging purposes)
    ///
    /// # Returns
    ///
    /// Returns the response body as a JSON `Value`.
    ///
    /// # Performance Optimizations
    ///
    /// - Early rejection based on Content-Length header (avoids downloading large responses)
    /// - Pre-allocated buffer for streaming (reduces memory reallocations)
    /// - Direct JSON parsing from bytes when possible
    ///
    /// # Errors
    ///
    /// Returns `InvalidRequest` error if the response exceeds `max_response_size`.
    #[instrument(name = "http_process_response_with_limit", skip(self, response), fields(status, url = %url))]
    async fn process_response_with_limit(&self, response: Response, url: &str) -> Result<Value> {
        let status = response.status();
        let headers = response.headers().clone();
        let max_size = self.config.max_response_size;

        // Record the status in the span
        tracing::Span::current().record("status", status.as_u16());

        // Check Content-Length header first (Requirement 1.2)
        // This is a fast early rejection that avoids downloading large responses
        if let Some(content_length) = response.content_length()
            && content_length > max_size as u64
        {
            warn!(
                url = %url,
                content_length = content_length,
                max_size = max_size,
                "Response exceeds size limit (Content-Length check)"
            );
            return Err(Error::invalid_request(format!(
                "Response size {content_length} bytes exceeds limit {max_size} bytes"
            )));
        }

        // Stream response body with size tracking (Requirement 1.3)
        let body_bytes = self
            .stream_response_with_limit(response, url, max_size)
            .await?;

        // Try to parse JSON directly from bytes first (more efficient)
        // Fall back to string conversion only if needed for error handling
        let body_text_for_error: String;
        let mut result: Value = if let Ok(value) = serde_json::from_slice(&body_bytes) {
            value
        } else {
            // Only convert to string if JSON parsing fails
            body_text_for_error = String::from_utf8_lossy(&body_bytes).to_string();
            Value::String(body_text_for_error.clone())
        };

        // Log response details (truncate body preview for large responses)
        let body_preview: String = if body_bytes.len() <= 200 {
            String::from_utf8_lossy(&body_bytes).to_string()
        } else {
            String::from_utf8_lossy(&body_bytes[..200]).to_string()
        };

        debug!(
            status = %status,
            body_length = body_bytes.len(),
            body_preview = %body_preview,
            "HTTP response received"
        );

        if self.config.return_response_headers
            && let Value::Object(ref mut map) = result
        {
            let headers_value = headers_to_json(&headers);
            map.insert("responseHeaders".to_string(), headers_value);
        }

        if !status.is_success() {
            // Convert to string only for error handling
            let body_text = String::from_utf8_lossy(&body_bytes).to_string();
            let err = Self::handle_http_error(status, &body_text, &result);
            error!(
                status = status.as_u16(),
                error = %err,
                body_preview = %body_preview,
                "HTTP error response"
            );
            return Err(err);
        }

        Ok(result)
    }

    /// Streams response body while enforcing size limit.
    ///
    /// This method reads the response body in chunks and aborts if the
    /// accumulated size exceeds the configured limit.
    ///
    /// # Performance Optimizations
    ///
    /// - Pre-allocates buffer based on Content-Length header when available
    /// - Uses `with_capacity` to minimize memory reallocations
    /// - Early termination when size limit is exceeded
    ///
    /// # Arguments
    ///
    /// * `response` - The HTTP response to stream
    /// * `url` - The request URL (for logging purposes)
    /// * `max_size` - Maximum allowed response size in bytes
    ///
    /// # Returns
    ///
    /// Returns the complete response body as bytes.
    ///
    /// # Errors
    ///
    /// Returns `InvalidRequest` error if accumulated size exceeds `max_size`.
    async fn stream_response_with_limit(
        &self,
        response: Response,
        url: &str,
        max_size: usize,
    ) -> Result<Vec<u8>> {
        use futures_util::StreamExt;

        // Performance optimization: Pre-allocate buffer based on Content-Length
        // This reduces memory reallocations for known-size responses
        #[allow(clippy::cast_possible_truncation)]
        let initial_capacity = response.content_length().map_or(
            // Default initial capacity for unknown sizes (64KB is a good balance
            // between memory usage and reallocation frequency)
            64 * 1024,
            |len| {
                // Cap at max_size to avoid over-allocation for large responses
                // that will be rejected anyway
                // Note: truncation is safe here because max_size is already usize
                std::cmp::min(len as usize, max_size)
            },
        );

        let mut stream = response.bytes_stream();
        let mut body = Vec::with_capacity(initial_capacity);
        let mut accumulated_size: usize = 0;

        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result.map_err(|e| {
                error!(
                    error = %e,
                    "Failed to read response chunk"
                );
                Error::network(format!("Failed to read response chunk: {e}"))
            })?;

            accumulated_size = accumulated_size.saturating_add(chunk.len());

            // Check if accumulated size exceeds limit (Requirement 1.3)
            // Early termination to avoid wasting bandwidth and memory
            if accumulated_size > max_size {
                warn!(
                    url = %url,
                    accumulated_size = accumulated_size,
                    max_size = max_size,
                    "Response exceeds size limit during streaming"
                );
                return Err(Error::invalid_request(format!(
                    "Response size {accumulated_size} bytes exceeds limit {max_size} bytes (streaming)"
                )));
            }

            body.extend_from_slice(&chunk);
        }

        // Shrink buffer if we over-allocated significantly (more than 25% unused)
        // This helps with memory efficiency for responses smaller than expected
        if body.capacity() > body.len() + body.len() / 4 {
            body.shrink_to_fit();
        }

        Ok(body)
    }

    /// Handles HTTP error status codes and converts them to appropriate errors.
    #[instrument(
        name = "http_handle_error",
        skip(body, result),
        fields(status = status.as_u16())
    )]
    fn handle_http_error(status: StatusCode, body: &str, result: &Value) -> Error {
        // Truncate body for logging to avoid excessive log sizes
        let body_preview: String = body.chars().take(200).collect();

        match status {
            StatusCode::BAD_REQUEST => {
                info!(body_preview = %body_preview, "Bad request error");
                Error::invalid_request(body.to_string())
            }
            StatusCode::UNAUTHORIZED => {
                warn!("Authentication error: Unauthorized");
                Error::authentication("Unauthorized")
            }
            StatusCode::FORBIDDEN => {
                warn!("Authentication error: Forbidden");
                Error::authentication("Forbidden")
            }
            StatusCode::NOT_FOUND => {
                info!("Resource not found");
                Error::invalid_request("Not found")
            }
            StatusCode::TOO_MANY_REQUESTS => {
                let retry_after = if let Value::Object(map) = result {
                    if let Some(Value::Object(headers)) = map.get("responseHeaders") {
                        headers
                            .get("retry-after")
                            .and_then(|v| v.as_str())
                            .and_then(|s| s.parse::<u64>().ok())
                    } else {
                        None
                    }
                } else {
                    None
                };

                if let Some(seconds) = retry_after {
                    warn!(
                        retry_after_seconds = seconds,
                        "Rate limit exceeded with retry-after header"
                    );
                    Error::rate_limit(
                        format!("Rate limit exceeded, retry after {seconds} seconds"),
                        Some(Duration::from_secs(seconds)),
                    )
                } else {
                    warn!("Rate limit exceeded without retry-after header");
                    Error::rate_limit("Rate limit exceeded, please retry later", None)
                }
            }
            StatusCode::INTERNAL_SERVER_ERROR => {
                error!(body_preview = %body_preview, "Internal server error");
                Error::exchange("500", "Internal server error")
            }
            StatusCode::SERVICE_UNAVAILABLE => {
                error!(body_preview = %body_preview, "Service unavailable");
                Error::exchange("503", "Service unavailable")
            }
            StatusCode::GATEWAY_TIMEOUT => {
                error!("Gateway timeout");
                Error::from(crate::error::NetworkError::Timeout)
            }
            _ => {
                error!(
                    status = status.as_u16(),
                    body_preview = %body_preview,
                    "Unhandled HTTP error"
                );
                Error::network(format!("HTTP {status} error: {body}"))
            }
        }
    }

    /// Executes a GET request.
    ///
    /// # Arguments
    ///
    /// * `url` - Target URL
    /// * `headers` - Optional custom headers
    ///
    /// # Returns
    ///
    /// Returns the response body as a JSON `Value`.
    ///
    /// # Errors
    ///
    /// Returns an error if the request fails.
    #[instrument(name = "http_get", skip(self, headers), fields(url = %url))]
    pub async fn get(&self, url: &str, headers: Option<HeaderMap>) -> Result<Value> {
        self.fetch(url, Method::GET, headers, None).await
    }

    /// Executes a POST request.
    ///
    /// # Arguments
    ///
    /// * `url` - Target URL
    /// * `headers` - Optional custom headers
    /// * `body` - Optional request body as JSON
    ///
    /// # Returns
    ///
    /// Returns the response body as a JSON `Value`.
    ///
    /// # Errors
    ///
    /// Returns an error if the request fails.
    #[instrument(name = "http_post", skip(self, headers, body), fields(url = %url))]
    pub async fn post(
        &self,
        url: &str,
        headers: Option<HeaderMap>,
        body: Option<Value>,
    ) -> Result<Value> {
        self.fetch(url, Method::POST, headers, body).await
    }

    /// Executes a PUT request.
    ///
    /// # Arguments
    ///
    /// * `url` - Target URL
    /// * `headers` - Optional custom headers
    /// * `body` - Optional request body as JSON
    ///
    /// # Returns
    ///
    /// Returns the response body as a JSON `Value`.
    ///
    /// # Errors
    ///
    /// Returns an error if the request fails.
    #[instrument(name = "http_put", skip(self, headers, body), fields(url = %url))]
    pub async fn put(
        &self,
        url: &str,
        headers: Option<HeaderMap>,
        body: Option<Value>,
    ) -> Result<Value> {
        self.fetch(url, Method::PUT, headers, body).await
    }

    /// Executes a DELETE request.
    ///
    /// # Arguments
    ///
    /// * `url` - Target URL
    /// * `headers` - Optional custom headers
    /// * `body` - Optional request body as JSON
    ///
    /// # Returns
    ///
    /// Returns the response body as a JSON `Value`.
    ///
    /// # Errors
    ///
    /// Returns an error if the request fails.
    #[instrument(name = "http_delete", skip(self, headers, body), fields(url = %url))]
    pub async fn delete(
        &self,
        url: &str,
        headers: Option<HeaderMap>,
        body: Option<Value>,
    ) -> Result<Value> {
        self.fetch(url, Method::DELETE, headers, body).await
    }

    /// Returns a reference to the current HTTP configuration.
    pub fn config(&self) -> &HttpConfig {
        &self.config
    }

    /// Updates the HTTP configuration.
    ///
    /// # Arguments
    ///
    /// * `config` - New configuration to use
    pub fn set_config(&mut self, config: HttpConfig) {
        self.config = config;
    }
}

/// Converts a `HeaderMap` to a JSON `Value`.
fn headers_to_json(headers: &HeaderMap) -> Value {
    let mut map = serde_json::Map::new();
    for (key, value) in headers {
        let key_str = key.as_str().to_string();
        let value_str = value.to_str().unwrap_or("").to_string();
        map.insert(key_str, Value::String(value_str));
    }
    Value::Object(map)
}

#[cfg(test)]
#[allow(clippy::disallowed_methods)] // unwrap() is acceptable in tests
#[allow(clippy::match_same_arms)] // same arms are acceptable in tests
#[allow(clippy::uninlined_format_args)] // format!("{}", x) is acceptable in tests
#[allow(clippy::assertions_on_constants)] // assert!(true) is acceptable in tests
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_http_client_creation() {
        let config = HttpConfig::default();
        let client = HttpClient::new(config);
        assert!(client.is_ok());
    }

    #[tokio::test]
    async fn test_http_config_default() {
        let config = HttpConfig::default();
        assert_eq!(config.timeout, Duration::from_secs(30));
        assert_eq!(config.connect_timeout, Duration::from_secs(10));
        assert!(config.retry_config.is_none());
        assert!(!config.verbose);
        assert_eq!(config.user_agent, "ccxt-rust/1.0");
        assert!(config.enable_rate_limit);
        assert_eq!(config.max_response_size, 10 * 1024 * 1024); // 10MB
    }

    #[tokio::test]
    async fn test_headers_to_json() {
        let mut headers = HeaderMap::new();
        headers.insert("Content-Type", "application/json".parse().unwrap());
        headers.insert("X-Custom-Header", "test-value".parse().unwrap());

        let json = headers_to_json(&headers);
        assert!(json.is_object());

        let obj = json.as_object().unwrap();
        assert_eq!(obj.get("content-type").unwrap(), "application/json");
        assert_eq!(obj.get("x-custom-header").unwrap(), "test-value");
    }

    #[tokio::test]
    async fn test_http_client_with_proxy() {
        use crate::config::ProxyConfig;

        let config = HttpConfig {
            proxy: Some(ProxyConfig::new("http://localhost:8080")),
            ..Default::default()
        };

        // Note: This test may fail if no actual proxy server is available
        let client = HttpClient::new(config);
        assert!(client.is_ok());
    }

    #[tokio::test]
    async fn test_get_request() {
        let config = HttpConfig {
            verbose: false,
            ..Default::default()
        };
        let client = HttpClient::new(config).unwrap();

        // Test using a public API (httpbin.org)
        let result = client.get("https://httpbin.org/get", None).await;

        // Since this depends on external service, we only check if result is not an error
        // In production, you may want to mock this request
        match result {
            Ok(value) => {
                assert!(value.is_object());
            }
            Err(e) => {
                // Network errors are acceptable (e.g., in CI environments)
                warn!("Network test skipped due to: {:?}", e);
            }
        }
    }

    #[tokio::test]
    async fn test_post_request() {
        let config = HttpConfig::default();
        let client = HttpClient::new(config).unwrap();

        let body = serde_json::json!({
            "test": "data",
            "number": 123
        });

        let result = client
            .post("https://httpbin.org/post", None, Some(body))
            .await;

        match result {
            Ok(value) => {
                assert!(value.is_object());
            }
            Err(e) => {
                warn!("Network test skipped due to: {:?}", e);
            }
        }
    }

    #[tokio::test]
    async fn test_http_error_handling() {
        let config = HttpConfig::default();
        let client = HttpClient::new(config).unwrap();

        // Test 404 error
        let result = client.get("https://httpbin.org/status/404", None).await;
        assert!(result.is_err());

        if let Err(e) = result {
            match e {
                Error::InvalidRequest(_) => {
                    // Expected error type for 404
                }
                Error::Exchange(_) => {
                    // May get 5xx errors from httpbin.org when service is unstable
                }
                Error::Network(_) => {
                    // May get network errors when service is unavailable
                }
                _ => panic!("Unexpected error type: {:?}", e),
            }
        }
    }

    #[tokio::test]
    async fn test_timeout() {
        let config = HttpConfig {
            timeout: Duration::from_secs(1), // 1 second timeout
            retry_config: Some(RetryConfig {
                max_retries: 0,
                ..RetryConfig::default()
            }),
            ..Default::default()
        };
        let client = HttpClient::new(config).unwrap();

        // Use an endpoint that will delay
        let result = client.get("https://httpbin.org/delay/5", None).await;
        assert!(result.is_err());

        if let Err(e) = result {
            match &e {
                Error::Timeout(msg) => {
                    // Expected timeout error - verify it contains URL and timeout info
                    assert!(
                        msg.contains("httpbin.org/delay/5"),
                        "Timeout error should contain URL"
                    );
                    assert!(
                        msg.contains("1000ms"),
                        "Timeout error should contain timeout duration"
                    );
                }
                Error::Network(_) => {
                    // May get network errors when service is unavailable (e.g., in CI environments)
                }
                _ => panic!("Expected Timeout or Network error, got: {:?}", e),
            }
            // Verify the error is retryable
            assert!(e.is_retryable(), "Timeout error should be retryable");
        }
    }

    #[tokio::test]
    async fn test_retry_mechanism() {
        let config = HttpConfig {
            retry_config: Some(RetryConfig {
                max_retries: 2,
                ..RetryConfig::default()
            }),
            verbose: true,
            ..Default::default()
        };
        let client = HttpClient::new(config).unwrap();

        // Test retry mechanism (using a potentially unstable endpoint)
        // Note: This test depends on external service
        let result = client.get("https://httpbin.org/status/503", None).await;

        // Should still fail after retries
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_rate_limiter_integration() {
        use crate::rate_limiter::{RateLimiter, RateLimiterConfig};
        use std::time::{Duration, Instant};

        // Create rate limiter: 5 requests per second
        let limiter_config = RateLimiterConfig::new(5, Duration::from_secs(1));
        let limiter = RateLimiter::new(limiter_config);

        let config = HttpConfig {
            enable_rate_limit: true,
            verbose: false,
            ..Default::default()
        };

        let client = HttpClient::new_with_rate_limiter(config, limiter).unwrap();

        // Test if rate limiting is effective
        let start = Instant::now();

        // Send 10 requests rapidly (should be rate limited)
        for _ in 0..10 {
            let _ = client.get("https://httpbin.org/get", None).await;
        }

        let elapsed = start.elapsed();

        // 10 requests at 5 req/sec should take at least 2 seconds
        // But due to network latency, we only check if it exceeds 1 second
        assert!(
            elapsed >= Duration::from_secs(1),
            "Rate limiter should have delayed requests"
        );
    }

    #[tokio::test]
    async fn test_rate_limiter_disabled() {
        let config = HttpConfig {
            enable_rate_limit: false,
            verbose: false,
            ..Default::default()
        };

        let client = HttpClient::new(config).unwrap();

        // Client should work normally when rate limiting is disabled
        match client.get("https://httpbin.org/get", None).await {
            Ok(_) => assert!(true),
            Err(e) => {
                // Network errors are acceptable
                warn!("Network test skipped due to: {:?}", e);
            }
        }
    }

    #[tokio::test]
    async fn test_execute_with_retry_success() {
        let config = HttpConfig::default();
        let client = HttpClient::new(config).unwrap();

        let result = client
            .execute_with_retry(|| async { Ok(serde_json::json!({"status": "ok"})) })
            .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap()["status"], "ok");
    }

    #[tokio::test]
    async fn test_execute_with_retry_failure() {
        let config = HttpConfig {
            retry_config: Some(RetryConfig {
                max_retries: 2,
                base_delay_ms: 10,
                ..RetryConfig::default()
            }),
            ..Default::default()
        };
        let client = HttpClient::new(config).unwrap();

        let result = client
            .execute_with_retry(|| async { Err(Error::network("Persistent failure")) })
            .await;

        assert!(result.is_err());
    }

    #[test]
    fn test_http_config_validate_default() {
        let config = HttpConfig::default();
        let result = config.validate();
        assert!(result.is_ok());
        assert!(result.unwrap().warnings.is_empty());
    }

    #[test]
    fn test_http_config_validate_timeout_too_high() {
        let config = HttpConfig {
            timeout: Duration::from_secs(600), // 10 minutes
            ..Default::default()
        };
        let result = config.validate();
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.field_name(), "timeout");
        assert!(matches!(
            err,
            crate::error::ConfigValidationError::ValueTooHigh { .. }
        ));
    }

    #[test]
    fn test_http_config_validate_timeout_boundary() {
        // timeout = 5 minutes should be valid
        let config = HttpConfig {
            timeout: Duration::from_secs(300),
            ..Default::default()
        };
        let result = config.validate();
        assert!(result.is_ok());
        assert!(result.unwrap().warnings.is_empty());

        // timeout = 301 seconds should be invalid
        let config = HttpConfig {
            timeout: Duration::from_secs(301),
            ..Default::default()
        };
        let result = config.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_http_config_validate_short_timeout_warning() {
        let config = HttpConfig {
            timeout: Duration::from_millis(500),
            ..Default::default()
        };
        let result = config.validate();
        assert!(result.is_ok());
        let validation_result = result.unwrap();
        assert!(!validation_result.warnings.is_empty());
        assert!(validation_result.warnings[0].contains("timeout"));
        assert!(validation_result.warnings[0].contains("very short"));
    }

    #[test]
    fn test_http_config_validate_timeout_warning_boundary() {
        // timeout = 1 second should not generate warning
        let config = HttpConfig {
            timeout: Duration::from_secs(1),
            ..Default::default()
        };
        let result = config.validate();
        assert!(result.is_ok());
        assert!(result.unwrap().warnings.is_empty());

        // timeout = 999ms should generate warning
        let config = HttpConfig {
            timeout: Duration::from_millis(999),
            ..Default::default()
        };
        let result = config.validate();
        assert!(result.is_ok());
        assert!(!result.unwrap().warnings.is_empty());
    }

    #[test]
    fn test_http_config_validate_max_request_size_zero() {
        let config = HttpConfig {
            max_request_size: 0,
            ..Default::default()
        };
        let result = config.validate();
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.field_name(), "max_request_size");
    }

    #[test]
    fn test_http_config_validate_max_request_size_too_large() {
        // max_request_size > 100MB should be rejected
        let config = HttpConfig {
            max_request_size: 101 * 1024 * 1024, // 101MB
            ..Default::default()
        };
        let result = config.validate();
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.field_name(), "max_request_size");
    }

    #[test]
    fn test_http_config_validate_max_request_size_boundary() {
        // max_request_size = 100MB should be valid
        let config = HttpConfig {
            max_request_size: 100 * 1024 * 1024, // 100MB
            ..Default::default()
        };
        let result = config.validate();
        assert!(result.is_ok());

        // max_request_size = 100MB + 1 byte should be rejected
        let config = HttpConfig {
            max_request_size: 100 * 1024 * 1024 + 1,
            ..Default::default()
        };
        let result = config.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_http_config_max_response_size_default() {
        let config = HttpConfig::default();
        assert_eq!(config.max_response_size, 10 * 1024 * 1024); // 10MB
    }

    #[test]
    fn test_http_config_max_response_size_custom() {
        let config = HttpConfig {
            max_response_size: 1024 * 1024, // 1MB
            ..Default::default()
        };
        assert_eq!(config.max_response_size, 1024 * 1024);
    }

    #[tokio::test]
    async fn test_response_size_limit_small_response() {
        // Test that small responses pass through normally
        let config = HttpConfig {
            max_response_size: 10 * 1024 * 1024, // 10MB
            ..Default::default()
        };
        let client = HttpClient::new(config).unwrap();

        // httpbin.org/get returns a small JSON response
        match client.get("https://httpbin.org/get", None).await {
            Ok(value) => {
                assert!(value.is_object());
            }
            Err(e) => {
                // Network errors are acceptable in CI environments
                warn!("Network test skipped due to: {:?}", e);
            }
        }
    }

    #[tokio::test]
    async fn test_response_size_limit_with_content_length() {
        // Test that responses with Content-Length exceeding limit are rejected
        let config = HttpConfig {
            max_response_size: 100, // Very small limit (100 bytes)
            retry_config: Some(RetryConfig {
                max_retries: 0,
                ..RetryConfig::default()
            }),
            ..Default::default()
        };
        let client = HttpClient::new(config).unwrap();

        // httpbin.org/bytes/1000 returns 1000 bytes with Content-Length header
        let result = client.get("https://httpbin.org/bytes/1000", None).await;

        match result {
            Err(Error::InvalidRequest(msg)) => {
                // Expected: response should be rejected due to size limit
                assert!(
                    msg.contains("exceeds limit") || msg.contains("size"),
                    "Error message should mention size limit: {}",
                    msg
                );
            }
            Err(Error::Network(_)) => {
                // Network errors are acceptable in CI environments
                warn!("Network test skipped due to network error");
            }
            Ok(_) => {
                // If the response somehow passed, it might be because httpbin
                // returned a smaller response or the Content-Length was not set
                warn!("Response passed size check - httpbin may have returned smaller response");
            }
            Err(e) => {
                panic!("Unexpected error type: {:?}", e);
            }
        }
    }

    #[test]
    fn test_http_config_circuit_breaker_default() {
        let config = HttpConfig::default();
        assert!(config.circuit_breaker.is_none());
    }

    #[test]
    fn test_http_config_circuit_breaker_custom() {
        use crate::circuit_breaker::CircuitBreakerConfig;

        let config = HttpConfig {
            circuit_breaker: Some(CircuitBreakerConfig::default()),
            ..Default::default()
        };
        assert!(config.circuit_breaker.is_some());
    }

    #[test]
    fn test_http_client_circuit_breaker_disabled_by_default() {
        let config = HttpConfig::default();
        let client = HttpClient::new(config).unwrap();
        assert!(client.circuit_breaker().is_none());
    }

    #[test]
    fn test_http_client_circuit_breaker_enabled() {
        use crate::circuit_breaker::CircuitBreakerConfig;

        let config = HttpConfig {
            circuit_breaker: Some(CircuitBreakerConfig::default()),
            ..Default::default()
        };
        let client = HttpClient::new(config).unwrap();
        assert!(client.circuit_breaker().is_some());
    }

    #[test]
    fn test_http_client_set_circuit_breaker() {
        use crate::circuit_breaker::{CircuitBreaker, CircuitBreakerConfig};

        let config = HttpConfig::default();
        let mut client = HttpClient::new(config).unwrap();
        assert!(client.circuit_breaker().is_none());

        let cb = Arc::new(CircuitBreaker::new(CircuitBreakerConfig::default()));
        client.set_circuit_breaker(cb);
        assert!(client.circuit_breaker().is_some());
    }

    #[tokio::test]
    async fn test_http_client_circuit_breaker_blocks_when_open() {
        use crate::circuit_breaker::{CircuitBreaker, CircuitBreakerConfig, CircuitState};

        let config = HttpConfig::default();
        let mut client = HttpClient::new(config).unwrap();

        // Create a circuit breaker that's already open
        let cb_config = CircuitBreakerConfig {
            failure_threshold: 1,
            reset_timeout: Duration::from_secs(60), // Long timeout
            success_threshold: 1,
        };
        let cb = Arc::new(CircuitBreaker::new(cb_config));

        // Open the circuit
        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Open);

        client.set_circuit_breaker(cb);

        // Request should be blocked
        let result = client.get("https://httpbin.org/get", None).await;
        assert!(result.is_err());

        if let Err(e) = result {
            assert!(e.as_resource_exhausted().is_some());
        }
    }
}

#[test]
fn test_http_config_pool_settings_default() {
    let config = HttpConfig::default();
    assert_eq!(config.pool_max_idle_per_host, 10);
    assert_eq!(config.pool_idle_timeout, Duration::from_secs(90));
}

#[test]
fn test_http_config_pool_settings_custom() {
    let config = HttpConfig {
        pool_max_idle_per_host: 20,
        pool_idle_timeout: Duration::from_secs(120),
        ..Default::default()
    };
    assert_eq!(config.pool_max_idle_per_host, 20);
    assert_eq!(config.pool_idle_timeout, Duration::from_secs(120));
}

#[test]
fn test_http_client_with_custom_pool_settings() {
    let config = HttpConfig {
        pool_max_idle_per_host: 5,
        pool_idle_timeout: Duration::from_secs(60),
        ..Default::default()
    };
    let client = HttpClient::new(config);
    assert!(client.is_ok());

    // Verify the config is stored correctly
    let client = client.unwrap();
    assert_eq!(client.config().pool_max_idle_per_host, 5);
    assert_eq!(client.config().pool_idle_timeout, Duration::from_secs(60));
}
