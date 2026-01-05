//! Signed request builder for Bybit API.
//!
//! This module provides a builder pattern for creating authenticated Bybit API requests,
//! encapsulating the common signing workflow used across all authenticated endpoints.
//!
//! # Overview
//!
//! The `BybitSignedRequestBuilder` eliminates code duplication by centralizing:
//! - Credential validation
//! - Millisecond timestamp generation
//! - Parameter signing with HMAC-SHA256 (hex encoded)
//! - Authentication header injection
//! - HTTP request execution
//!
//! # Example
//!
//! ```no_run
//! # use ccxt_exchanges::bybit::Bybit;
//! # use ccxt_exchanges::bybit::signed_request::HttpMethod;
//! # use ccxt_core::ExchangeConfig;
//! # async fn example() -> ccxt_core::Result<()> {
//! let bybit = Bybit::new(ExchangeConfig::default())?;
//!
//! // Simple GET request
//! let data = bybit.signed_request("/v5/account/wallet-balance")
//!     .param("accountType", "UNIFIED")
//!     .execute()
//!     .await?;
//!
//! // POST request with body
//! let data = bybit.signed_request("/v5/order/create")
//!     .method(HttpMethod::Post)
//!     .param("category", "spot")
//!     .param("symbol", "BTCUSDT")
//!     .param("side", "Buy")
//!     .execute()
//!     .await?;
//! # Ok(())
//! # }
//! ```

use super::Bybit;
use ccxt_core::{Error, ParseError, Result};
use reqwest::header::{HeaderMap, HeaderValue};
use serde_json::Value;
use std::collections::BTreeMap;

/// HTTP request methods supported by the signed request builder.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HttpMethod {
    /// GET request - parameters in query string
    #[default]
    Get,
    /// POST request - parameters in JSON body
    Post,
    /// DELETE request - parameters in JSON body
    Delete,
}

/// Builder for creating authenticated Bybit API requests.
///
/// This builder encapsulates the common signing workflow:
/// 1. Credential validation
/// 2. Millisecond timestamp generation
/// 3. Parameter signing with HMAC-SHA256 (hex encoded)
/// 4. Authentication header injection (X-BAPI-* headers)
/// 5. HTTP request execution
///
/// # Bybit Signature Format
///
/// Bybit uses a unique signature format:
/// - Sign string: `timestamp + api_key + recv_window + params`
/// - For GET: params is the query string
/// - For POST: params is the JSON body string
/// - Signature is hex-encoded HMAC-SHA256
///
/// # Example
///
/// ```no_run
/// # use ccxt_exchanges::bybit::Bybit;
/// # use ccxt_exchanges::bybit::signed_request::HttpMethod;
/// # use ccxt_core::ExchangeConfig;
/// # async fn example() -> ccxt_core::Result<()> {
/// let bybit = Bybit::new(ExchangeConfig::default())?;
///
/// let data = bybit.signed_request("/v5/account/wallet-balance")
///     .param("accountType", "UNIFIED")
///     .optional_param("coin", Some("BTC"))
///     .execute()
///     .await?;
/// # Ok(())
/// # }
/// ```
pub struct BybitSignedRequestBuilder<'a> {
    /// Reference to the Bybit exchange instance
    bybit: &'a Bybit,
    /// Request parameters to be signed
    params: BTreeMap<String, String>,
    /// Request body for POST/DELETE requests
    body: Option<Value>,
    /// API endpoint path
    endpoint: String,
    /// HTTP method for the request
    method: HttpMethod,
}

impl<'a> BybitSignedRequestBuilder<'a> {
    /// Creates a new signed request builder.
    ///
    /// # Arguments
    ///
    /// * `bybit` - Reference to the Bybit exchange instance
    /// * `endpoint` - API endpoint path (e.g., "/v5/account/wallet-balance")
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use ccxt_exchanges::bybit::Bybit;
    /// # use ccxt_exchanges::bybit::signed_request::BybitSignedRequestBuilder;
    /// # use ccxt_core::ExchangeConfig;
    /// let bybit = Bybit::new(ExchangeConfig::default()).unwrap();
    /// let builder = BybitSignedRequestBuilder::new(&bybit, "/v5/account/wallet-balance");
    /// ```
    pub fn new(bybit: &'a Bybit, endpoint: impl Into<String>) -> Self {
        Self {
            bybit,
            params: BTreeMap::new(),
            body: None,
            endpoint: endpoint.into(),
            method: HttpMethod::default(),
        }
    }

    /// Sets the HTTP method for the request.
    ///
    /// Default is GET.
    ///
    /// # Arguments
    ///
    /// * `method` - The HTTP method to use
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use ccxt_exchanges::bybit::Bybit;
    /// # use ccxt_exchanges::bybit::signed_request::HttpMethod;
    /// # use ccxt_core::ExchangeConfig;
    /// # async fn example() -> ccxt_core::Result<()> {
    /// let bybit = Bybit::new(ExchangeConfig::default())?;
    /// let data = bybit.signed_request("/v5/order/create")
    ///     .method(HttpMethod::Post)
    ///     .param("category", "spot")
    ///     .execute()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn method(mut self, method: HttpMethod) -> Self {
        self.method = method;
        self
    }

    /// Adds a required parameter.
    ///
    /// # Arguments
    ///
    /// * `key` - Parameter name
    /// * `value` - Parameter value (will be converted to string)
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use ccxt_exchanges::bybit::Bybit;
    /// # use ccxt_core::ExchangeConfig;
    /// # async fn example() -> ccxt_core::Result<()> {
    /// let bybit = Bybit::new(ExchangeConfig::default())?;
    /// let data = bybit.signed_request("/v5/order/realtime")
    ///     .param("category", "spot")
    ///     .param("symbol", "BTCUSDT")
    ///     .execute()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn param(mut self, key: impl Into<String>, value: impl ToString) -> Self {
        self.params.insert(key.into(), value.to_string());
        self
    }

    /// Adds an optional parameter (only if value is Some).
    ///
    /// # Arguments
    ///
    /// * `key` - Parameter name
    /// * `value` - Optional parameter value
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use ccxt_exchanges::bybit::Bybit;
    /// # use ccxt_core::ExchangeConfig;
    /// # async fn example() -> ccxt_core::Result<()> {
    /// let bybit = Bybit::new(ExchangeConfig::default())?;
    /// let since: Option<i64> = Some(1234567890000);
    /// let limit: Option<u32> = None;
    ///
    /// let data = bybit.signed_request("/v5/order/history")
    ///     .param("category", "spot")
    ///     .optional_param("startTime", since)
    ///     .optional_param("limit", limit)  // Won't be added since it's None
    ///     .execute()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn optional_param<T: ToString>(mut self, key: impl Into<String>, value: Option<T>) -> Self {
        if let Some(v) = value {
            self.params.insert(key.into(), v.to_string());
        }
        self
    }

    /// Adds multiple parameters from a BTreeMap.
    ///
    /// # Arguments
    ///
    /// * `params` - Map of parameter key-value pairs
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use ccxt_exchanges::bybit::Bybit;
    /// # use ccxt_core::ExchangeConfig;
    /// # use std::collections::BTreeMap;
    /// # async fn example() -> ccxt_core::Result<()> {
    /// let bybit = Bybit::new(ExchangeConfig::default())?;
    /// let mut params = BTreeMap::new();
    /// params.insert("category".to_string(), "spot".to_string());
    /// params.insert("symbol".to_string(), "BTCUSDT".to_string());
    ///
    /// let data = bybit.signed_request("/v5/order/create")
    ///     .params(params)
    ///     .execute()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn params(mut self, params: BTreeMap<String, String>) -> Self {
        self.params.extend(params);
        self
    }

    /// Sets the request body for POST/DELETE requests.
    ///
    /// # Arguments
    ///
    /// * `body` - JSON value to use as request body
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use ccxt_exchanges::bybit::Bybit;
    /// # use ccxt_core::ExchangeConfig;
    /// # use serde_json::json;
    /// # async fn example() -> ccxt_core::Result<()> {
    /// let bybit = Bybit::new(ExchangeConfig::default())?;
    /// let body = json!({
    ///     "category": "spot",
    ///     "symbol": "BTCUSDT",
    ///     "side": "Buy",
    ///     "orderType": "Limit"
    /// });
    ///
    /// let data = bybit.signed_request("/v5/order/create")
    ///     .method(ccxt_exchanges::bybit::signed_request::HttpMethod::Post)
    ///     .body(body)
    ///     .execute()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn body(mut self, body: Value) -> Self {
        self.body = Some(body);
        self
    }

    /// Executes the signed request and returns the response.
    ///
    /// This method:
    /// 1. Validates that credentials are configured
    /// 2. Gets the current millisecond timestamp
    /// 3. Signs the request with HMAC-SHA256 (hex encoded)
    /// 4. Adds authentication headers (X-BAPI-*)
    /// 5. Executes the HTTP request
    ///
    /// # Bybit Signature Details
    ///
    /// - Sign string format: `timestamp + api_key + recv_window + params`
    /// - For GET requests: params is the query string
    /// - For POST requests: params is the JSON body string
    /// - Signature is hex-encoded (not Base64 like OKX/Bitget)
    ///
    /// # Returns
    ///
    /// Returns the raw `serde_json::Value` response for further parsing.
    ///
    /// # Errors
    ///
    /// - Returns authentication error if credentials are missing
    /// - Returns network error if the request fails
    /// - Returns parse error if response parsing fails
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use ccxt_exchanges::bybit::Bybit;
    /// # use ccxt_core::ExchangeConfig;
    /// # async fn example() -> ccxt_core::Result<()> {
    /// let bybit = Bybit::new(ExchangeConfig::default())?;
    /// let data = bybit.signed_request("/v5/account/wallet-balance")
    ///     .param("accountType", "UNIFIED")
    ///     .execute()
    ///     .await?;
    /// println!("Response: {:?}", data);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn execute(self) -> Result<Value> {
        // Step 1: Validate credentials
        self.bybit.check_required_credentials()?;

        // Step 2: Get current millisecond timestamp
        let timestamp = chrono::Utc::now().timestamp_millis().to_string();

        // Step 3: Get auth and recv_window
        let auth = self.bybit.get_auth()?;
        let recv_window = self.bybit.options().recv_window;

        // Build the sign params based on HTTP method
        // For GET: sign the query string
        // For POST/DELETE: sign the JSON body
        let sign_params = if self.method == HttpMethod::Get {
            build_query_string(&self.params)
        } else if let Some(ref body) = self.body {
            body.to_string()
        } else if !self.params.is_empty() {
            // Convert params to JSON for POST/DELETE
            serde_json::to_string(&self.params).map_err(|e| {
                ccxt_core::Error::from(ccxt_core::ParseError::invalid_format(
                    "request params",
                    format!("JSON serialization failed: {}", e),
                ))
            })?
        } else {
            String::new()
        };

        // Sign the request (Bybit uses hex encoding)
        let signature = auth.sign(&timestamp, recv_window, &sign_params);

        // Step 4: Add authentication headers
        let mut headers = HeaderMap::new();
        auth.add_auth_headers(&mut headers, &timestamp, &signature, recv_window);
        headers.insert("Content-Type", HeaderValue::from_static("application/json"));

        // Step 5: Execute HTTP request based on method
        let urls = self.bybit.urls();
        let full_url = format!("{}{}", urls.rest, self.endpoint);

        match self.method {
            HttpMethod::Get => {
                // Build query string for GET requests
                let query_string = build_query_string(&self.params);
                let url = if query_string.is_empty() {
                    full_url
                } else {
                    format!("{}?{}", full_url, query_string)
                };
                self.bybit.base().http_client.get(&url, Some(headers)).await
            }
            HttpMethod::Post => {
                // Use explicit body if provided, otherwise use params as JSON
                let body = if let Some(b) = self.body {
                    b
                } else {
                    serde_json::to_value(&self.params).map_err(|e| {
                        Error::from(ParseError::invalid_format(
                            "data",
                            format!("Failed to serialize request body: {}", e),
                        ))
                    })?
                };
                self.bybit
                    .base()
                    .http_client
                    .post(&full_url, Some(headers), Some(body))
                    .await
            }
            HttpMethod::Delete => {
                // Use explicit body if provided, otherwise use params as JSON
                let body = if let Some(b) = self.body {
                    b
                } else {
                    serde_json::to_value(&self.params).map_err(|e| {
                        Error::from(ParseError::invalid_format(
                            "data",
                            format!("Failed to serialize request body: {}", e),
                        ))
                    })?
                };
                self.bybit
                    .base()
                    .http_client
                    .delete(&full_url, Some(headers), Some(body))
                    .await
            }
        }
    }
}

/// Converts HttpMethod to string representation.
#[allow(dead_code)]
fn method_to_string(method: HttpMethod) -> String {
    match method {
        HttpMethod::Get => "GET".to_string(),
        HttpMethod::Post => "POST".to_string(),
        HttpMethod::Delete => "DELETE".to_string(),
    }
}

/// Builds a URL-encoded query string from parameters.
///
/// Parameters are sorted alphabetically by key (due to BTreeMap ordering).
fn build_query_string(params: &BTreeMap<String, String>) -> String {
    params
        .iter()
        .map(|(k, v)| format!("{}={}", k, urlencoding::encode(v)))
        .collect::<Vec<_>>()
        .join("&")
}

#[cfg(test)]
mod tests {
    use super::*;
    use ccxt_core::ExchangeConfig;

    #[test]
    fn test_http_method_default() {
        assert_eq!(HttpMethod::default(), HttpMethod::Get);
    }

    #[test]
    fn test_builder_construction() {
        let config = ExchangeConfig::default();
        let bybit = Bybit::new(config).unwrap();

        let builder = BybitSignedRequestBuilder::new(&bybit, "/v5/account/wallet-balance");

        assert_eq!(builder.endpoint, "/v5/account/wallet-balance");
        assert_eq!(builder.method, HttpMethod::Get);
        assert!(builder.params.is_empty());
        assert!(builder.body.is_none());
    }

    #[test]
    fn test_builder_method_chaining() {
        let config = ExchangeConfig::default();
        let bybit = Bybit::new(config).unwrap();

        let builder = BybitSignedRequestBuilder::new(&bybit, "/v5/order/create")
            .method(HttpMethod::Post)
            .param("category", "spot")
            .param("symbol", "BTCUSDT");

        assert_eq!(builder.method, HttpMethod::Post);
        assert_eq!(builder.params.get("category"), Some(&"spot".to_string()));
        assert_eq!(builder.params.get("symbol"), Some(&"BTCUSDT".to_string()));
    }

    #[test]
    fn test_builder_param() {
        let config = ExchangeConfig::default();
        let bybit = Bybit::new(config).unwrap();

        let builder = BybitSignedRequestBuilder::new(&bybit, "/test")
            .param("string_param", "value")
            .param("int_param", 123)
            .param("float_param", 45.67);

        assert_eq!(
            builder.params.get("string_param"),
            Some(&"value".to_string())
        );
        assert_eq!(builder.params.get("int_param"), Some(&"123".to_string()));
        assert_eq!(
            builder.params.get("float_param"),
            Some(&"45.67".to_string())
        );
    }

    #[test]
    fn test_builder_optional_param_some() {
        let config = ExchangeConfig::default();
        let bybit = Bybit::new(config).unwrap();

        let builder = BybitSignedRequestBuilder::new(&bybit, "/test")
            .optional_param("limit", Some(100u32))
            .optional_param("startTime", Some(1234567890i64));

        assert_eq!(builder.params.get("limit"), Some(&"100".to_string()));
        assert_eq!(
            builder.params.get("startTime"),
            Some(&"1234567890".to_string())
        );
    }

    #[test]
    fn test_builder_optional_param_none() {
        let config = ExchangeConfig::default();
        let bybit = Bybit::new(config).unwrap();

        let none_value: Option<u32> = None;
        let builder =
            BybitSignedRequestBuilder::new(&bybit, "/test").optional_param("limit", none_value);

        assert!(builder.params.get("limit").is_none());
    }

    #[test]
    fn test_builder_params_bulk() {
        let config = ExchangeConfig::default();
        let bybit = Bybit::new(config).unwrap();

        let mut params = BTreeMap::new();
        params.insert("category".to_string(), "spot".to_string());
        params.insert("symbol".to_string(), "BTCUSDT".to_string());

        let builder = BybitSignedRequestBuilder::new(&bybit, "/test").params(params);

        assert_eq!(builder.params.get("category"), Some(&"spot".to_string()));
        assert_eq!(builder.params.get("symbol"), Some(&"BTCUSDT".to_string()));
    }

    #[test]
    fn test_builder_body() {
        let config = ExchangeConfig::default();
        let bybit = Bybit::new(config).unwrap();

        let body = serde_json::json!({
            "category": "spot",
            "symbol": "BTCUSDT"
        });

        let builder = BybitSignedRequestBuilder::new(&bybit, "/test").body(body.clone());

        assert_eq!(builder.body, Some(body));
    }

    #[test]
    fn test_builder_all_http_methods() {
        let config = ExchangeConfig::default();
        let bybit = Bybit::new(config).unwrap();

        // Test all HTTP methods can be set
        let get_builder = BybitSignedRequestBuilder::new(&bybit, "/test").method(HttpMethod::Get);
        assert_eq!(get_builder.method, HttpMethod::Get);

        let post_builder = BybitSignedRequestBuilder::new(&bybit, "/test").method(HttpMethod::Post);
        assert_eq!(post_builder.method, HttpMethod::Post);

        let delete_builder =
            BybitSignedRequestBuilder::new(&bybit, "/test").method(HttpMethod::Delete);
        assert_eq!(delete_builder.method, HttpMethod::Delete);
    }

    #[test]
    fn test_builder_parameter_ordering() {
        let config = ExchangeConfig::default();
        let bybit = Bybit::new(config).unwrap();

        // Add parameters in non-alphabetical order
        let builder = BybitSignedRequestBuilder::new(&bybit, "/test")
            .param("zebra", "z")
            .param("apple", "a")
            .param("mango", "m");

        // BTreeMap should maintain alphabetical order
        let keys: Vec<_> = builder.params.keys().collect();
        assert_eq!(keys, vec!["apple", "mango", "zebra"]);
    }

    #[test]
    fn test_method_to_string() {
        assert_eq!(method_to_string(HttpMethod::Get), "GET");
        assert_eq!(method_to_string(HttpMethod::Post), "POST");
        assert_eq!(method_to_string(HttpMethod::Delete), "DELETE");
    }

    #[test]
    fn test_build_query_string() {
        let mut params = BTreeMap::new();
        params.insert("category".to_string(), "spot".to_string());
        params.insert("symbol".to_string(), "BTCUSDT".to_string());
        params.insert("limit".to_string(), "50".to_string());

        let query = build_query_string(&params);

        // BTreeMap maintains alphabetical order
        assert_eq!(query, "category=spot&limit=50&symbol=BTCUSDT");
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

        // Should URL-encode the slash
        assert_eq!(query, "symbol=BTC%2FUSDT");
    }
}

#[cfg(test)]
mod property_tests {
    use super::*;
    use ccxt_core::ExchangeConfig;
    use proptest::prelude::*;

    // Strategy to generate valid parameter keys (alphanumeric, non-empty)
    fn param_key_strategy() -> impl Strategy<Value = String> {
        "[a-zA-Z][a-zA-Z0-9]{0,19}".prop_map(|s| s)
    }

    // Strategy to generate valid parameter values
    fn param_value_strategy() -> impl Strategy<Value = String> {
        "[a-zA-Z0-9._-]{1,50}".prop_map(|s| s)
    }

    // Strategy to generate a BTreeMap of parameters
    fn params_strategy() -> impl Strategy<Value = BTreeMap<String, String>> {
        proptest::collection::btree_map(param_key_strategy(), param_value_strategy(), 0..10)
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// **Feature: exchange-signed-request-refactor, Property 1: Fluent API Method Chaining**
        ///
        /// *For any* sequence of builder method calls (param, optional_param, params, method),
        /// each method SHALL return Self allowing further method chaining, and the final execute()
        /// call SHALL consume the builder.
        ///
        /// **Validates: Requirements 3.1**
        #[test]
        fn prop_fluent_api_method_chaining(
            params in params_strategy(),
            other_key in param_key_strategy(),
            other_value in param_value_strategy()
        ) {
            let config = ExchangeConfig::default();
            let bybit = Bybit::new(config).unwrap();

            // Build with method chaining
            let builder = BybitSignedRequestBuilder::new(&bybit, "/test")
                .method(HttpMethod::Post)
                .params(params.clone())
                .param(&other_key, &other_value)
                .optional_param("optional", Some("value"));

            // Verify all parameters are present
            for (key, value) in &params {
                prop_assert_eq!(builder.params.get(key), Some(value));
            }
            prop_assert_eq!(builder.params.get(&other_key), Some(&other_value));
            prop_assert_eq!(builder.params.get("optional"), Some(&"value".to_string()));
            prop_assert_eq!(builder.method, HttpMethod::Post);
        }

        /// **Feature: exchange-signed-request-refactor, Property 5: Parameter Methods Correctly Populate Params**
        ///
        /// *For any* combination of param(), optional_param(), and params() calls:
        /// - param(key, value) SHALL always add the key-value pair
        /// - optional_param(key, Some(value)) SHALL add the key-value pair
        /// - optional_param(key, None) SHALL NOT add any key-value pair
        /// - params(map) SHALL add all key-value pairs from the map
        ///
        /// **Validates: Requirements 3.1, 3.2, 3.3, 3.4, 3.5, 3.6, 3.7, 3.8**
        #[test]
        fn prop_optional_param_conditional_addition(
            key in param_key_strategy(),
            value in proptest::option::of(param_value_strategy()),
            other_key in param_key_strategy(),
            other_value in param_value_strategy()
        ) {
            let config = ExchangeConfig::default();
            let bybit = Bybit::new(config).unwrap();

            // Build with optional parameter
            let builder = BybitSignedRequestBuilder::new(&bybit, "/test")
                .param(&other_key, &other_value)
                .optional_param(&key, value.clone());

            // Property: If value is Some, parameter should be present with correct value
            // If value is None, parameter should NOT be present
            match value {
                Some(v) => {
                    prop_assert!(
                        builder.params.contains_key(&key),
                        "Parameter {} should be present when value is Some",
                        key
                    );
                    prop_assert_eq!(
                        builder.params.get(&key).unwrap(),
                        &v,
                        "Parameter {} should have correct value",
                        key
                    );
                }
                None => {
                    // Only check if key is different from other_key
                    if key != other_key {
                        prop_assert!(
                            !builder.params.contains_key(&key),
                            "Parameter {} should NOT be present when value is None",
                            key
                        );
                    }
                }
            }

            // Property: Other parameters should always be present
            prop_assert!(
                builder.params.contains_key(&other_key),
                "Other parameter {} should always be present",
                other_key
            );
            prop_assert_eq!(
                builder.params.get(&other_key).unwrap(),
                &other_value,
                "Other parameter {} should have correct value",
                other_key
            );
        }

        /// **Feature: exchange-signed-request-refactor, Property 6: HTTP Method Determines Parameter Location**
        ///
        /// *For any* signed request:
        /// - GET requests SHALL have parameters in the URL query string
        /// - POST requests SHALL have parameters in the JSON request body
        /// - The signing input SHALL use the appropriate format based on HTTP method
        ///
        /// **Validates: Requirements 3.8**
        #[test]
        fn prop_http_method_determines_param_location(
            params in params_strategy(),
            method in prop_oneof![
                Just(HttpMethod::Get),
                Just(HttpMethod::Post),
                Just(HttpMethod::Delete)
            ]
        ) {
            let config = ExchangeConfig::default();
            let bybit = Bybit::new(config).unwrap();

            let builder = BybitSignedRequestBuilder::new(&bybit, "/test")
                .method(method)
                .params(params.clone());

            // Verify method is set correctly
            prop_assert_eq!(builder.method, method);

            // Verify all params are stored
            for (key, value) in &params {
                prop_assert_eq!(builder.params.get(key), Some(value));
            }
        }
    }
}
