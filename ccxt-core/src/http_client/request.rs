use crate::error::{Error, Result};
use reqwest::{Method, header::HeaderMap};
use serde_json::Value;
use tracing::{debug, error, instrument, warn};

use super::builder::HttpClient;

impl HttpClient {
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
        fields(method = %method, url = %url, timeout_ms = %self.config().timeout.as_millis())
    )]
    pub async fn fetch(
        &self,
        url: &str,
        method: Method,
        headers: Option<HeaderMap>,
        body: Option<Value>,
    ) -> Result<Value> {
        if let Some(cb) = self.circuit_breaker() {
            cb.allow_request()?;
        }

        #[allow(clippy::collapsible_if)]
        if self.config().enable_rate_limit {
            if let Some(limiter) = self.rate_limiter() {
                limiter.wait().await;
            }
        }

        let total_timeout = self.config().timeout;
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

        if let Some(cb) = self.circuit_breaker() {
            match &result {
                Ok(_) => cb.record_success(),
                Err(_) => cb.record_failure(),
            }
        }

        result
    }

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
        let mut request = self.client().request(method.clone(), url);

        if let Some(headers) = headers {
            request = request.headers(headers);
        }

        if let Some(ref body) = body {
            let body_str = serde_json::to_string(body)
                .map_err(|e| Error::invalid_request(format!("JSON serialization failed: {e}")))?;

            if body_str.len() > self.config().max_request_size {
                return Err(Error::invalid_request(format!(
                    "Request body {} bytes exceeds limit {} bytes",
                    body_str.len(),
                    self.config().max_request_size
                )));
            }

            request = request.body(body_str);
        }

        if self.config().verbose {
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
}
