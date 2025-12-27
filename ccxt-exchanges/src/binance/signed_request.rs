//! Signed request builder for Binance API.
//!
//! This module provides a builder pattern for creating authenticated Binance API requests,
//! encapsulating the common signing workflow used across all authenticated endpoints.
//!
//! # Overview
//!
//! The `SignedRequestBuilder` eliminates code duplication by centralizing:
//! - Credential validation
//! - Timestamp generation
//! - Parameter signing with HMAC-SHA256
//! - Authentication header injection
//! - HTTP request execution
//!
//! # Example
//!
//! ```no_run
//! # use ccxt_exchanges::binance::Binance;
//! # use ccxt_exchanges::binance::signed_request::HttpMethod;
//! # use ccxt_core::ExchangeConfig;
//! # async fn example() -> ccxt_core::Result<()> {
//! let binance = Binance::new(ExchangeConfig::default())?;
//!
//! // Simple GET request
//! let data = binance.signed_request("/api/v3/account")
//!     .execute()
//!     .await?;
//!
//! // POST request with parameters
//! let data = binance.signed_request("/api/v3/order")
//!     .method(HttpMethod::Post)
//!     .param("symbol", "BTCUSDT")
//!     .param("side", "BUY")
//!     .param("type", "MARKET")
//!     .param("quantity", "0.001")
//!     .execute()
//!     .await?;
//! # Ok(())
//! # }
//! ```

use super::Binance;
use ccxt_core::{Error, ParseError, Result};
use reqwest::header::HeaderMap;
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
    /// PUT request - parameters in JSON body
    Put,
    /// DELETE request - parameters in JSON body
    Delete,
}

/// Builder for creating authenticated Binance API requests.
///
/// This builder encapsulates the common signing workflow:
/// 1. Credential validation
/// 2. Timestamp generation
/// 3. Parameter signing with HMAC-SHA256
/// 4. Authentication header injection
/// 5. HTTP request execution
///
/// # Example
///
/// ```no_run
/// # use ccxt_exchanges::binance::Binance;
/// # use ccxt_exchanges::binance::signed_request::HttpMethod;
/// # use ccxt_core::ExchangeConfig;
/// # async fn example() -> ccxt_core::Result<()> {
/// let binance = Binance::new(ExchangeConfig::default())?;
///
/// let data = binance.signed_request("/api/v3/account")
///     .param("symbol", "BTCUSDT")
///     .optional_param("limit", Some(100))
///     .execute()
///     .await?;
/// # Ok(())
/// # }
/// ```
pub struct SignedRequestBuilder<'a> {
    /// Reference to the Binance exchange instance
    binance: &'a Binance,
    /// Request parameters to be signed
    params: BTreeMap<String, String>,
    /// API endpoint URL
    endpoint: String,
    /// HTTP method for the request
    method: HttpMethod,
}

impl<'a> SignedRequestBuilder<'a> {
    /// Creates a new signed request builder.
    ///
    /// # Arguments
    ///
    /// * `binance` - Reference to the Binance exchange instance
    /// * `endpoint` - Full API endpoint URL
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use ccxt_exchanges::binance::Binance;
    /// # use ccxt_exchanges::binance::signed_request::SignedRequestBuilder;
    /// # use ccxt_core::ExchangeConfig;
    /// let binance = Binance::new(ExchangeConfig::default()).unwrap();
    /// let builder = SignedRequestBuilder::new(&binance, "https://api.binance.com/api/v3/account");
    /// ```
    pub fn new(binance: &'a Binance, endpoint: impl Into<String>) -> Self {
        Self {
            binance,
            params: BTreeMap::new(),
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
    /// # use ccxt_exchanges::binance::Binance;
    /// # use ccxt_exchanges::binance::signed_request::HttpMethod;
    /// # use ccxt_core::ExchangeConfig;
    /// # async fn example() -> ccxt_core::Result<()> {
    /// let binance = Binance::new(ExchangeConfig::default())?;
    /// let data = binance.signed_request("/api/v3/order")
    ///     .method(HttpMethod::Post)
    ///     .param("symbol", "BTCUSDT")
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
    /// # use ccxt_exchanges::binance::Binance;
    /// # use ccxt_core::ExchangeConfig;
    /// # async fn example() -> ccxt_core::Result<()> {
    /// let binance = Binance::new(ExchangeConfig::default())?;
    /// let data = binance.signed_request("/api/v3/myTrades")
    ///     .param("symbol", "BTCUSDT")
    ///     .param("limit", 100)
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
    /// # use ccxt_exchanges::binance::Binance;
    /// # use ccxt_core::ExchangeConfig;
    /// # async fn example() -> ccxt_core::Result<()> {
    /// let binance = Binance::new(ExchangeConfig::default())?;
    /// let since: Option<i64> = Some(1234567890);
    /// let limit: Option<u32> = None;
    ///
    /// let data = binance.signed_request("/api/v3/myTrades")
    ///     .param("symbol", "BTCUSDT")
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
    /// # use ccxt_exchanges::binance::Binance;
    /// # use ccxt_core::ExchangeConfig;
    /// # use std::collections::BTreeMap;
    /// # async fn example() -> ccxt_core::Result<()> {
    /// let binance = Binance::new(ExchangeConfig::default())?;
    /// let mut params = BTreeMap::new();
    /// params.insert("symbol".to_string(), "BTCUSDT".to_string());
    /// params.insert("side".to_string(), "BUY".to_string());
    ///
    /// let data = binance.signed_request("/api/v3/order")
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

    /// Merges parameters from a JSON Value object.
    ///
    /// Only string, number, and boolean values are supported.
    /// Nested objects and arrays are ignored.
    ///
    /// # Arguments
    ///
    /// * `params` - Optional JSON Value containing parameters
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use ccxt_exchanges::binance::Binance;
    /// # use ccxt_core::ExchangeConfig;
    /// # use serde_json::json;
    /// # async fn example() -> ccxt_core::Result<()> {
    /// let binance = Binance::new(ExchangeConfig::default())?;
    /// let extra_params = Some(json!({
    ///     "orderId": "12345",
    ///     "fromId": 67890
    /// }));
    ///
    /// let data = binance.signed_request("/api/v3/myTrades")
    ///     .param("symbol", "BTCUSDT")
    ///     .merge_json_params(extra_params)
    ///     .execute()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn merge_json_params(mut self, params: Option<Value>) -> Self {
        if let Some(Value::Object(map)) = params {
            for (key, value) in map {
                let string_value = match value {
                    Value::String(s) => s,
                    Value::Number(n) => n.to_string(),
                    Value::Bool(b) => b.to_string(),
                    _ => continue, // Skip arrays, objects, and null
                };
                self.params.insert(key, string_value);
            }
        }
        self
    }

    /// Executes the signed request and returns the response.
    ///
    /// This method:
    /// 1. Validates that credentials are configured
    /// 2. Gets the signing timestamp (using cached offset if available)
    /// 3. Signs the parameters with HMAC-SHA256
    /// 4. Adds authentication headers
    /// 5. Executes the HTTP request
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
    /// # use ccxt_exchanges::binance::Binance;
    /// # use ccxt_core::ExchangeConfig;
    /// # async fn example() -> ccxt_core::Result<()> {
    /// let binance = Binance::new(ExchangeConfig::default())?;
    /// let data = binance.signed_request("/api/v3/account")
    ///     .execute()
    ///     .await?;
    /// println!("Response: {:?}", data);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn execute(self) -> Result<Value> {
        // Step 1: Validate credentials
        self.binance.check_required_credentials()?;

        // Step 2: Get signing timestamp
        let timestamp = self.binance.get_signing_timestamp().await?;

        // Step 3: Get auth and sign parameters
        let auth = self.binance.get_auth()?;
        let signed_params = auth.sign_with_timestamp(
            &self.params,
            timestamp,
            Some(self.binance.options().recv_window),
        )?;

        // Step 4: Add authentication headers
        let mut headers = HeaderMap::new();
        auth.add_auth_headers_reqwest(&mut headers);

        // Step 5: Execute HTTP request based on method
        match self.method {
            HttpMethod::Get => {
                // Build query string for GET requests
                let query_string = build_query_string(&signed_params);
                let url = if query_string.is_empty() {
                    self.endpoint
                } else {
                    format!("{}?{}", self.endpoint, query_string)
                };
                self.binance
                    .base()
                    .http_client
                    .get(&url, Some(headers))
                    .await
            }
            HttpMethod::Post => {
                // Serialize parameters as JSON body for POST requests
                let body = serde_json::to_value(&signed_params).map_err(|e| {
                    Error::from(ParseError::invalid_format(
                        "data",
                        format!("Failed to serialize request body: {}", e),
                    ))
                })?;
                self.binance
                    .base()
                    .http_client
                    .post(&self.endpoint, Some(headers), Some(body))
                    .await
            }
            HttpMethod::Put => {
                // Serialize parameters as JSON body for PUT requests
                let body = serde_json::to_value(&signed_params).map_err(|e| {
                    Error::from(ParseError::invalid_format(
                        "data",
                        format!("Failed to serialize request body: {}", e),
                    ))
                })?;
                self.binance
                    .base()
                    .http_client
                    .put(&self.endpoint, Some(headers), Some(body))
                    .await
            }
            HttpMethod::Delete => {
                // Serialize parameters as JSON body for DELETE requests
                let body = serde_json::to_value(&signed_params).map_err(|e| {
                    Error::from(ParseError::invalid_format(
                        "data",
                        format!("Failed to serialize request body: {}", e),
                    ))
                })?;
                self.binance
                    .base()
                    .http_client
                    .delete(&self.endpoint, Some(headers), Some(body))
                    .await
            }
        }
    }
}

/// Builds a URL-encoded query string from parameters.
///
/// Parameters are sorted alphabetically by key (due to BTreeMap ordering).
fn build_query_string(params: &BTreeMap<String, String>) -> String {
    params
        .iter()
        .map(|(k, v)| format!("{}={}", k, v))
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
        let binance = Binance::new(config).unwrap();

        let builder = SignedRequestBuilder::new(&binance, "https://api.binance.com/api/v3/account");

        assert_eq!(builder.endpoint, "https://api.binance.com/api/v3/account");
        assert_eq!(builder.method, HttpMethod::Get);
        assert!(builder.params.is_empty());
    }

    #[test]
    fn test_builder_method_chaining() {
        let config = ExchangeConfig::default();
        let binance = Binance::new(config).unwrap();

        let builder = SignedRequestBuilder::new(&binance, "/api/v3/order")
            .method(HttpMethod::Post)
            .param("symbol", "BTCUSDT")
            .param("side", "BUY");

        assert_eq!(builder.method, HttpMethod::Post);
        assert_eq!(builder.params.get("symbol"), Some(&"BTCUSDT".to_string()));
        assert_eq!(builder.params.get("side"), Some(&"BUY".to_string()));
    }

    #[test]
    fn test_builder_param() {
        let config = ExchangeConfig::default();
        let binance = Binance::new(config).unwrap();

        let builder = SignedRequestBuilder::new(&binance, "/test")
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
        let binance = Binance::new(config).unwrap();

        let builder = SignedRequestBuilder::new(&binance, "/test")
            .optional_param("limit", Some(100u32))
            .optional_param("since", Some(1234567890i64));

        assert_eq!(builder.params.get("limit"), Some(&"100".to_string()));
        assert_eq!(builder.params.get("since"), Some(&"1234567890".to_string()));
    }

    #[test]
    fn test_builder_optional_param_none() {
        let config = ExchangeConfig::default();
        let binance = Binance::new(config).unwrap();

        let none_value: Option<u32> = None;
        let builder =
            SignedRequestBuilder::new(&binance, "/test").optional_param("limit", none_value);

        assert!(builder.params.get("limit").is_none());
    }

    #[test]
    fn test_builder_params_bulk() {
        let config = ExchangeConfig::default();
        let binance = Binance::new(config).unwrap();

        let mut params = BTreeMap::new();
        params.insert("symbol".to_string(), "BTCUSDT".to_string());
        params.insert("side".to_string(), "BUY".to_string());

        let builder = SignedRequestBuilder::new(&binance, "/test").params(params);

        assert_eq!(builder.params.get("symbol"), Some(&"BTCUSDT".to_string()));
        assert_eq!(builder.params.get("side"), Some(&"BUY".to_string()));
    }

    #[test]
    fn test_builder_merge_json_params() {
        let config = ExchangeConfig::default();
        let binance = Binance::new(config).unwrap();

        let json_params = Some(serde_json::json!({
            "orderId": "12345",
            "fromId": 67890,
            "active": true,
            "nested": {"ignored": "value"},
            "array": [1, 2, 3]
        }));

        let builder = SignedRequestBuilder::new(&binance, "/test").merge_json_params(json_params);

        assert_eq!(builder.params.get("orderId"), Some(&"12345".to_string()));
        assert_eq!(builder.params.get("fromId"), Some(&"67890".to_string()));
        assert_eq!(builder.params.get("active"), Some(&"true".to_string()));
        // Nested objects and arrays should be ignored
        assert!(builder.params.get("nested").is_none());
        assert!(builder.params.get("array").is_none());
    }

    #[test]
    fn test_builder_merge_json_params_none() {
        let config = ExchangeConfig::default();
        let binance = Binance::new(config).unwrap();

        let builder = SignedRequestBuilder::new(&binance, "/test")
            .param("existing", "value")
            .merge_json_params(None);

        assert_eq!(builder.params.get("existing"), Some(&"value".to_string()));
        assert_eq!(builder.params.len(), 1);
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
    fn test_builder_all_http_methods() {
        let config = ExchangeConfig::default();
        let binance = Binance::new(config).unwrap();

        // Test all HTTP methods can be set
        let get_builder = SignedRequestBuilder::new(&binance, "/test").method(HttpMethod::Get);
        assert_eq!(get_builder.method, HttpMethod::Get);

        let post_builder = SignedRequestBuilder::new(&binance, "/test").method(HttpMethod::Post);
        assert_eq!(post_builder.method, HttpMethod::Post);

        let put_builder = SignedRequestBuilder::new(&binance, "/test").method(HttpMethod::Put);
        assert_eq!(put_builder.method, HttpMethod::Put);

        let delete_builder =
            SignedRequestBuilder::new(&binance, "/test").method(HttpMethod::Delete);
        assert_eq!(delete_builder.method, HttpMethod::Delete);
    }

    #[test]
    fn test_builder_parameter_ordering() {
        let config = ExchangeConfig::default();
        let binance = Binance::new(config).unwrap();

        // Add parameters in non-alphabetical order
        let builder = SignedRequestBuilder::new(&binance, "/test")
            .param("zebra", "z")
            .param("apple", "a")
            .param("mango", "m");

        // BTreeMap should maintain alphabetical order
        let keys: Vec<_> = builder.params.keys().collect();
        assert_eq!(keys, vec!["apple", "mango", "zebra"]);
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

        /// **Feature: binance-signed-request-refactor, Property 2: Signed Request Contains Required Fields**
        ///
        /// *For any* executed signed request with valid credentials, the signed parameters SHALL contain:
        /// - A `timestamp` field with a valid millisecond timestamp
        /// - A `signature` field with a 64-character hex string (HMAC-SHA256)
        /// - A `recvWindow` field matching the configured value
        ///
        /// **Validates: Requirements 1.3, 1.4, 1.5, 1.6**
        #[test]
        fn prop_signed_request_contains_required_fields(
            params in params_strategy(),
            recv_window in 1000u64..60000u64
        ) {
            // Create a Binance instance with credentials
            let mut config = ExchangeConfig::default();
            config.api_key = Some("test_api_key".to_string());
            config.secret = Some("test_secret_key".to_string());

            let mut options = super::super::BinanceOptions::default();
            options.recv_window = recv_window;

            let binance = super::super::Binance::new_with_options(config, options).unwrap();

            // Get auth and sign parameters manually to verify the signing logic
            let auth = binance.get_auth().unwrap();
            let timestamp = 1234567890000i64; // Fixed timestamp for testing

            let signed_params = auth.sign_with_timestamp(&params, timestamp, Some(recv_window)).unwrap();

            // Property 1: timestamp field exists and matches
            prop_assert!(signed_params.contains_key("timestamp"));
            prop_assert_eq!(signed_params.get("timestamp").unwrap(), &timestamp.to_string());

            // Property 2: signature field exists and is 64 hex characters
            prop_assert!(signed_params.contains_key("signature"));
            let signature = signed_params.get("signature").unwrap();
            prop_assert_eq!(signature.len(), 64, "Signature should be 64 hex characters");
            prop_assert!(signature.chars().all(|c| c.is_ascii_hexdigit()), "Signature should be hex");

            // Property 3: recvWindow field exists and matches configured value
            prop_assert!(signed_params.contains_key("recvWindow"));
            prop_assert_eq!(signed_params.get("recvWindow").unwrap(), &recv_window.to_string());

            // Property 4: All original parameters are preserved
            for (key, value) in &params {
                prop_assert!(signed_params.contains_key(key), "Original param {} should be preserved", key);
                prop_assert_eq!(signed_params.get(key).unwrap(), value, "Original param {} value should match", key);
            }
        }

        /// **Feature: binance-signed-request-refactor, Property 5: Optional Parameters Conditional Addition**
        ///
        /// *For any* call to `optional_param(key, value)`:
        /// - If `value` is `Some(v)`, the parameter SHALL be added with key and v.to_string()
        /// - If `value` is `None`, the parameter SHALL NOT be added
        ///
        /// **Validates: Requirements 2.2**
        #[test]
        fn prop_optional_param_conditional_addition(
            key in param_key_strategy(),
            value in proptest::option::of(param_value_strategy()),
            other_key in param_key_strategy(),
            other_value in param_value_strategy()
        ) {
            let config = ExchangeConfig::default();
            let binance = super::super::Binance::new(config).unwrap();

            // Build with optional parameter
            let builder = SignedRequestBuilder::new(&binance, "/test")
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
    }
}
