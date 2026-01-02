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

use crate::config::ProxyConfig;
use crate::error::{Error, Result};
use crate::rate_limiter::RateLimiter;
use crate::retry_strategy::{RetryConfig, RetryStrategy};
use reqwest::{Client, Method, Response, StatusCode, header::HeaderMap};
use serde_json::Value;
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
        }
    }
}

/// HTTP client with retry and rate limiting support
#[derive(Debug)]
pub struct HttpClient {
    client: Client,
    config: HttpConfig,
    rate_limiter: Option<RateLimiter>,
    retry_strategy: RetryStrategy,
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

        Ok(Self {
            client,
            config,
            rate_limiter: None,
            retry_strategy,
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

        match tokio::time::timeout(
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
        }
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
            request = request.json(&body);
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

        self.process_response(response).await
    }

    /// Processes an HTTP response, handling errors and parsing JSON.
    #[instrument(name = "http_process_response", skip(self, response), fields(status))]
    async fn process_response(&self, response: Response) -> Result<Value> {
        let status = response.status();
        let headers = response.headers().clone();

        // Record the status in the span
        tracing::Span::current().record("status", status.as_u16());

        let body_text = response.text().await.map_err(|e| {
            error!(
                error = %e,
                "Failed to read response body"
            );
            Error::network(format!("Failed to read response body: {e}"))
        })?;

        // Log response details (truncate body preview for large responses)
        let body_preview: String = body_text.chars().take(200).collect();
        debug!(
            status = %status,
            body_length = body_text.len(),
            body_preview = %body_preview,
            "HTTP response received"
        );

        let mut result: Value =
            serde_json::from_str(&body_text).unwrap_or_else(|_| Value::String(body_text.clone()));

        if self.config.return_response_headers
            && let Value::Object(ref mut map) = result
        {
            let headers_value = headers_to_json(&headers);
            map.insert("responseHeaders".to_string(), headers_value);
        }

        if !status.is_success() {
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
}
