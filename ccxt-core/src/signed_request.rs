//! Generic signed request builder for authenticated API requests.
//!
//! This module provides a reusable infrastructure for building authenticated
//! requests across different cryptocurrency exchanges. Each exchange implements
//! the [`SigningStrategy`] trait to define its specific signing behavior.
//!
//! # Architecture
//!
//! - [`SignedRequestBuilder`]: Generic builder handling parameter management and request execution
//! - [`SigningStrategy`]: Trait for exchange-specific signing (timestamp, signature, headers)
//! - [`HasHttpClient`]: Trait for exchanges that can execute HTTP requests
//! - [`SigningContext`]: Data structure containing all signing-related information
//!
//! # Example
//!
//! ```rust,ignore
//! // Exchange implements HasHttpClient and provides a SigningStrategy
//! let response = exchange
//!     .signed_request("/api/v3/account")
//!     .param("symbol", "BTCUSDT")
//!     .optional_param("limit", Some(100))
//!     .execute()
//!     .await?;
//! ```

use crate::error::Result;
use crate::http_client::HttpClient;
use async_trait::async_trait;
use reqwest::header::HeaderMap;
use serde_json::Value;
use std::collections::BTreeMap;

/// HTTP request methods supported by signed requests.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HttpMethod {
    /// GET request - parameters in query string
    #[default]
    Get,
    /// POST request - parameters in JSON body
    Post,
    /// PUT request - parameters in JSON body
    Put,
    /// DELETE request - parameters in JSON body
    Delete,
}

impl HttpMethod {
    /// Convert to uppercase string representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Get => "GET",
            Self::Post => "POST",
            Self::Put => "PUT",
            Self::Delete => "DELETE",
        }
    }
}

/// Context containing all data needed for request signing.
#[derive(Debug, Clone)]
pub struct SigningContext {
    /// HTTP method for the request.
    pub method: HttpMethod,
    /// API endpoint path (e.g., "/api/v3/account").
    pub endpoint: String,
    /// Request parameters (BTreeMap for deterministic ordering).
    pub params: BTreeMap<String, String>,
    /// Optional JSON body for POST/PUT/DELETE.
    pub body: Option<Value>,
    /// Timestamp string (format varies by exchange).
    pub timestamp: String,
    /// Computed signature (set by SigningStrategy).
    pub signature: Option<String>,
}

impl SigningContext {
    /// Create a new signing context.
    pub fn new(method: HttpMethod, endpoint: String) -> Self {
        Self {
            method,
            endpoint,
            params: BTreeMap::new(),
            body: None,
            timestamp: String::new(),
            signature: None,
        }
    }
}

/// Trait for exchange-specific request signing strategies.
///
/// Implementors define how to:
/// - Generate timestamps in exchange-specific format
/// - Compute request signatures
/// - Add authentication headers
#[async_trait]
pub trait SigningStrategy: Send + Sync {
    /// Prepare the request for signing.
    ///
    /// This method should:
    /// 1. Generate timestamp in exchange-specific format
    /// 2. Compute signature based on method, path, params, body
    /// 3. Update ctx.timestamp and ctx.signature
    /// 4. Optionally modify ctx.params (e.g., add timestamp param)
    async fn prepare_request(&self, ctx: &mut SigningContext) -> Result<()>;

    /// Add authentication headers to the request.
    ///
    /// Called after prepare_request(). Should add all required
    /// authentication headers (API key, signature, timestamp, etc.)
    fn add_auth_headers(&self, headers: &mut HeaderMap, ctx: &SigningContext);
}

/// Trait for exchanges that can execute HTTP requests.
pub trait HasHttpClient {
    /// Get reference to the HTTP client.
    fn http_client(&self) -> &HttpClient;

    /// Get the base URL for API requests.
    fn base_url(&self) -> &str;
}

/// Generic builder for creating authenticated API requests.
///
/// This builder encapsulates the common workflow:
/// 1. Parameter building with fluent API
/// 2. Request signing via [`SigningStrategy`]
/// 3. HTTP execution via [`HasHttpClient`]
pub struct SignedRequestBuilder<'a, E, S>
where
    E: HasHttpClient,
    S: SigningStrategy,
{
    exchange: &'a E,
    strategy: S,
    params: BTreeMap<String, String>,
    body: Option<Value>,
    endpoint: String,
    method: HttpMethod,
}

impl<'a, E, S> SignedRequestBuilder<'a, E, S>
where
    E: HasHttpClient,
    S: SigningStrategy,
{
    /// Create a new signed request builder.
    pub fn new(exchange: &'a E, strategy: S, endpoint: impl Into<String>) -> Self {
        Self {
            exchange,
            strategy,
            params: BTreeMap::new(),
            body: None,
            endpoint: endpoint.into(),
            method: HttpMethod::default(),
        }
    }

    /// Set the HTTP method (default: GET).
    pub fn method(mut self, method: HttpMethod) -> Self {
        self.method = method;
        self
    }

    /// Add a required parameter.
    pub fn param(mut self, key: impl Into<String>, value: impl ToString) -> Self {
        self.params.insert(key.into(), value.to_string());
        self
    }

    /// Add an optional parameter (only if Some).
    pub fn optional_param<T: ToString>(mut self, key: impl Into<String>, value: Option<T>) -> Self {
        if let Some(v) = value {
            self.params.insert(key.into(), v.to_string());
        }
        self
    }

    /// Add multiple parameters from a BTreeMap.
    pub fn params(mut self, params: BTreeMap<String, String>) -> Self {
        self.params.extend(params);
        self
    }

    /// Set the request body (for POST/PUT/DELETE).
    pub fn body(mut self, body: Value) -> Self {
        self.body = Some(body);
        self
    }

    /// Merge parameters from a JSON Value object.
    ///
    /// Only string, number, and boolean values are supported.
    /// Nested objects and arrays are ignored.
    pub fn merge_json_params(mut self, params: Option<Value>) -> Self {
        if let Some(Value::Object(map)) = params {
            for (key, value) in map {
                let string_value = match value {
                    Value::String(s) => s,
                    Value::Number(n) => n.to_string(),
                    Value::Bool(b) => b.to_string(),
                    _ => continue,
                };
                self.params.insert(key, string_value);
            }
        }
        self
    }

    /// Execute the signed request.
    ///
    /// # Steps
    /// 1. Create SigningContext from builder state
    /// 2. Call strategy.prepare_request() to sign
    /// 3. Build headers via strategy.add_auth_headers()
    /// 4. Execute HTTP request via exchange.http_client()
    pub async fn execute(self) -> Result<Value> {
        // Create signing context
        let mut ctx = SigningContext {
            method: self.method,
            endpoint: self.endpoint.clone(),
            params: self.params,
            body: self.body,
            timestamp: String::new(),
            signature: None,
        };

        // Prepare request (generate timestamp, compute signature)
        self.strategy.prepare_request(&mut ctx).await?;

        // Build headers
        let mut headers = HeaderMap::new();
        self.strategy.add_auth_headers(&mut headers, &ctx);

        // Build URL
        let base_url = self.exchange.base_url();
        let full_url = format!("{}{}", base_url, self.endpoint);

        // Execute request based on method
        let client = self.exchange.http_client();

        match self.method {
            HttpMethod::Get => {
                let query_string = build_query_string(&ctx.params);
                let url = if query_string.is_empty() {
                    full_url
                } else {
                    format!("{}?{}", full_url, query_string)
                };
                client.get(&url, Some(headers)).await
            }
            HttpMethod::Post => {
                let body = ctx.body.unwrap_or_else(|| {
                    serde_json::to_value(&ctx.params).unwrap_or(Value::Object(Default::default()))
                });
                client.post(&full_url, Some(headers), Some(body)).await
            }
            HttpMethod::Put => {
                let body = ctx.body.unwrap_or_else(|| {
                    serde_json::to_value(&ctx.params).unwrap_or(Value::Object(Default::default()))
                });
                client.put(&full_url, Some(headers), Some(body)).await
            }
            HttpMethod::Delete => {
                let body = ctx.body.unwrap_or_else(|| {
                    serde_json::to_value(&ctx.params).unwrap_or(Value::Object(Default::default()))
                });
                client.delete(&full_url, Some(headers), Some(body)).await
            }
        }
    }
}

/// Build URL-encoded query string from parameters.
///
/// Parameters are sorted alphabetically (BTreeMap ordering).
pub fn build_query_string(params: &BTreeMap<String, String>) -> String {
    params
        .iter()
        .map(|(k, v)| format!("{}={}", k, urlencoding::encode(v)))
        .collect::<Vec<_>>()
        .join("&")
}

/// Build query string without URL encoding (for signing).
pub fn build_query_string_raw(params: &BTreeMap<String, String>) -> String {
    params
        .iter()
        .map(|(k, v)| format!("{}={}", k, v))
        .collect::<Vec<_>>()
        .join("&")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_http_method_default() {
        assert_eq!(HttpMethod::default(), HttpMethod::Get);
    }

    #[test]
    fn test_http_method_as_str() {
        assert_eq!(HttpMethod::Get.as_str(), "GET");
        assert_eq!(HttpMethod::Post.as_str(), "POST");
        assert_eq!(HttpMethod::Put.as_str(), "PUT");
        assert_eq!(HttpMethod::Delete.as_str(), "DELETE");
    }

    #[test]
    fn test_signing_context_new() {
        let ctx = SigningContext::new(HttpMethod::Post, "/api/test".to_string());
        assert_eq!(ctx.method, HttpMethod::Post);
        assert_eq!(ctx.endpoint, "/api/test");
        assert!(ctx.params.is_empty());
        assert!(ctx.body.is_none());
        assert!(ctx.timestamp.is_empty());
        assert!(ctx.signature.is_none());
    }

    #[test]
    fn test_build_query_string() {
        let mut params = BTreeMap::new();
        params.insert("symbol".to_string(), "BTCUSDT".to_string());
        params.insert("side".to_string(), "BUY".to_string());
        params.insert("amount".to_string(), "1.5".to_string());

        let query = build_query_string(&params);
        // BTreeMap maintains alphabetical order
        assert_eq!(query, "amount=1.5&side=BUY&symbol=BTCUSDT");
    }

    #[test]
    fn test_build_query_string_empty() {
        let params = BTreeMap::new();
        let query = build_query_string(&params);
        assert!(query.is_empty());
    }

    #[test]
    fn test_build_query_string_with_special_chars() {
        let mut params = BTreeMap::new();
        params.insert("symbol".to_string(), "BTC/USDT".to_string());

        let query = build_query_string(&params);
        assert_eq!(query, "symbol=BTC%2FUSDT");
    }

    #[test]
    fn test_build_query_string_raw() {
        let mut params = BTreeMap::new();
        params.insert("symbol".to_string(), "BTC/USDT".to_string());

        let query = build_query_string_raw(&params);
        assert_eq!(query, "symbol=BTC/USDT");
    }
}
