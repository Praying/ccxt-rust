//! Binance authentication and signature module.
//!
//! Implements HMAC-SHA256 signature algorithm for API request authentication.

use ccxt_core::credentials::SecretString;
use ccxt_core::{Error, Result};
use hmac::{Hmac, Mac};
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use sha2::Sha256;
use std::collections::{BTreeMap, HashMap};

type HmacSha256 = Hmac<Sha256>;

/// Binance authenticator.
///
/// Credentials are automatically zeroed from memory when dropped.
#[derive(Debug, Clone)]
pub struct BinanceAuth {
    /// API key (automatically zeroed on drop).
    api_key: SecretString,
    /// Secret key (automatically zeroed on drop).
    secret: SecretString,
}

impl BinanceAuth {
    /// Creates a new authenticator.
    ///
    /// # Arguments
    ///
    /// * `api_key` - API key
    /// * `secret` - Secret key
    ///
    /// # Security
    ///
    /// Credentials are automatically zeroed from memory when the authenticator is dropped.
    pub fn new(api_key: impl Into<String>, secret: impl Into<String>) -> Self {
        Self {
            api_key: SecretString::new(api_key),
            secret: SecretString::new(secret),
        }
    }

    /// Returns the API key.
    pub fn api_key(&self) -> &str {
        self.api_key.expose_secret()
    }

    /// Returns the secret key.
    pub fn secret(&self) -> &str {
        self.secret.expose_secret()
    }

    /// Signs a query string using HMAC-SHA256.
    ///
    /// # Arguments
    ///
    /// * `query_string` - Query string to sign
    ///
    /// # Returns
    ///
    /// Returns the HMAC-SHA256 signature as a hex string.
    ///
    /// # Errors
    ///
    /// Returns an error if the secret key is invalid.
    pub fn sign(&self, query_string: &str) -> Result<String> {
        let mut mac = HmacSha256::new_from_slice(self.secret.expose_secret_bytes())
            .map_err(|e| Error::authentication(format!("Invalid secret key: {}", e)))?;

        mac.update(query_string.as_bytes());
        let result = mac.finalize();
        let signature = hex::encode(result.into_bytes());

        Ok(signature)
    }

    /// Signs a parameter map.
    ///
    /// # Arguments
    ///
    /// * `params` - Parameter map
    ///
    /// # Returns
    ///
    /// Returns a new parameter map containing the signature.
    ///
    /// # Errors
    ///
    /// Returns an error if signature generation fails.
    pub fn sign_params(
        &self,
        params: &BTreeMap<String, String>,
    ) -> Result<BTreeMap<String, String>> {
        let query_string = self.build_query_string(params);
        let signature = self.sign(&query_string)?;

        let mut signed_params = params.clone();
        signed_params.insert("signature".to_string(), signature);

        Ok(signed_params)
    }

    /// Signs parameters with timestamp and optional receive window.
    ///
    /// # Arguments
    ///
    /// * `params` - Parameter map
    /// * `timestamp` - Timestamp in milliseconds
    /// * `recv_window` - Optional receive window in milliseconds
    ///
    /// # Returns
    ///
    /// Returns a new parameter map containing timestamp and signature.
    ///
    /// # Errors
    ///
    /// Returns an error if signature generation fails.
    pub fn sign_with_timestamp(
        &self,
        params: &BTreeMap<String, String>,
        timestamp: i64,
        recv_window: Option<u64>,
    ) -> Result<BTreeMap<String, String>> {
        let mut params_with_time = params.clone();
        params_with_time.insert("timestamp".to_string(), timestamp.to_string());

        if let Some(window) = recv_window {
            params_with_time.insert("recvWindow".to_string(), window.to_string());
        }

        self.sign_params(&params_with_time)
    }

    /// Builds a query string from parameters.
    ///
    /// # Arguments
    ///
    /// * `params` - Parameter map
    ///
    /// # Returns
    ///
    /// Returns a URL-encoded query string with parameters sorted by key.
    /// Both keys and values are URL-encoded to handle special characters.
    #[allow(clippy::unused_self)]
    pub(crate) fn build_query_string(&self, params: &BTreeMap<String, String>) -> String {
        let mut pairs: Vec<_> = params.iter().collect();
        pairs.sort_by_key(|(k, _)| *k);

        pairs
            .iter()
            .map(|(k, v)| format!("{}={}", urlencoding::encode(k), urlencoding::encode(v)))
            .collect::<Vec<_>>()
            .join("&")
    }

    /// Adds authentication headers to the request.
    ///
    /// # Arguments
    ///
    /// * `headers` - Existing header map to modify
    pub fn add_auth_headers(&self, headers: &mut HashMap<String, String>) {
        headers.insert(
            "X-MBX-APIKEY".to_string(),
            self.api_key.expose_secret().to_string(),
        );
    }

    /// Adds authentication headers to a `reqwest` request.
    ///
    /// # Arguments
    ///
    /// * `headers` - Reqwest `HeaderMap` to modify
    pub fn add_auth_headers_reqwest(&self, headers: &mut HeaderMap) {
        if let Ok(header_name) = HeaderName::from_bytes(b"X-MBX-APIKEY") {
            if let Ok(header_value) = HeaderValue::from_str(self.api_key.expose_secret()) {
                headers.insert(header_name, header_value);
            }
        }
    }
}

/// Builds a signed URL with query parameters.
///
/// # Arguments
///
/// * `base_url` - Base URL
/// * `endpoint` - API endpoint path
/// * `params` - Parameter map (must include signature)
///
/// # Returns
///
/// Returns the complete URL with URL-encoded query parameters.
pub fn build_signed_url<S: std::hash::BuildHasher>(
    base_url: &str,
    endpoint: &str,
    params: &HashMap<String, String, S>,
) -> String {
    let query_string = params
        .iter()
        .map(|(k, v)| format!("{}={}", urlencoding::encode(k), urlencoding::encode(v)))
        .collect::<Vec<_>>()
        .join("&");

    if query_string.is_empty() {
        format!("{base_url}{endpoint}")
    } else {
        format!("{base_url}{endpoint}?{query_string}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sign() {
        let auth = BinanceAuth::new("test_key", "test_secret");
        let query = "symbol=BTCUSDT&side=BUY&type=LIMIT&quantity=1&timestamp=1234567890";

        let signature = auth.sign(query);
        assert!(signature.is_ok());

        let sig = signature.expect("Signature failed");
        assert!(!sig.is_empty());
        assert_eq!(sig.len(), 64); // HMAC-SHA256 produces 64 hex characters
    }

    #[test]
    fn test_sign_params() {
        let auth = BinanceAuth::new("test_key", "test_secret");
        let mut params = BTreeMap::new();
        params.insert("symbol".to_string(), "BTCUSDT".to_string());
        params.insert("side".to_string(), "BUY".to_string());

        let signed = auth.sign_params(&params);
        assert!(signed.is_ok());

        let signed_params = signed.expect("Sign params failed");
        assert!(signed_params.contains_key("signature"));
        assert_eq!(
            signed_params.get("symbol").expect("Missing symbol"),
            "BTCUSDT"
        );
    }

    #[test]
    fn test_sign_with_timestamp() {
        let auth = BinanceAuth::new("test_key", "test_secret");
        let params = BTreeMap::new();
        let timestamp = 1234567890i64;

        let signed = auth.sign_with_timestamp(&params, timestamp, Some(5000));
        assert!(signed.is_ok());

        let signed_params = signed.expect("Sign timestamp failed");
        assert!(signed_params.contains_key("timestamp"));
        assert!(signed_params.contains_key("recvWindow"));
        assert!(signed_params.contains_key("signature"));
        assert_eq!(
            signed_params.get("timestamp").expect("Missing timestamp"),
            "1234567890"
        );
        assert_eq!(
            signed_params.get("recvWindow").expect("Missing recvWindow"),
            "5000"
        );
    }

    #[test]
    fn test_build_query_string() {
        let auth = BinanceAuth::new("test_key", "test_secret");
        let mut params = BTreeMap::new();
        params.insert("symbol".to_string(), "BTCUSDT".to_string());
        params.insert("side".to_string(), "BUY".to_string());

        let query = auth.build_query_string(&params);
        assert!(query == "side=BUY&symbol=BTCUSDT" || query == "symbol=BTCUSDT&side=BUY");
    }

    #[test]
    fn test_add_auth_headers() {
        let auth = BinanceAuth::new("my_api_key", "test_secret");
        let mut headers = HashMap::new();

        auth.add_auth_headers(&mut headers);

        assert!(headers.contains_key("X-MBX-APIKEY"));
        assert_eq!(
            headers.get("X-MBX-APIKEY").expect("Missing header"),
            "my_api_key"
        );
    }

    #[test]
    fn test_build_signed_url() {
        let mut params = HashMap::new();
        params.insert("symbol".to_string(), "BTCUSDT".to_string());
        params.insert("signature".to_string(), "abc123".to_string());

        let url = build_signed_url("https://api.binance.com", "/api/v3/order", &params);

        assert!(url.contains("https://api.binance.com/api/v3/order?"));
        assert!(url.contains("symbol=BTCUSDT"));
        assert!(url.contains("signature=abc123"));
    }

    #[test]
    fn test_build_signed_url_empty_params() {
        let params = HashMap::new();
        let url = build_signed_url("https://api.binance.com", "/api/v3/time", &params);

        assert_eq!(url, "https://api.binance.com/api/v3/time");
    }

    #[test]
    fn test_build_query_string_url_encoding() {
        let auth = BinanceAuth::new("test_key", "test_secret");
        let mut params = BTreeMap::new();
        // Test special characters that need URL encoding
        params.insert("clientOrderId".to_string(), "order+123&test".to_string());
        params.insert("symbol".to_string(), "BTC/USDT".to_string());

        let query = auth.build_query_string(&params);

        // Verify special characters are URL encoded
        assert!(query.contains("clientOrderId=order%2B123%26test"));
        assert!(query.contains("symbol=BTC%2FUSDT"));
        // Verify & is used as separator (not encoded)
        assert!(query.contains("&"));
    }

    #[test]
    fn test_build_query_string_space_encoding() {
        let auth = BinanceAuth::new("test_key", "test_secret");
        let mut params = BTreeMap::new();
        params.insert("note".to_string(), "hello world".to_string());

        let query = auth.build_query_string(&params);

        // Space should be encoded as %20
        assert!(query.contains("note=hello%20world"));
    }

    #[test]
    fn test_build_query_string_equals_encoding() {
        let auth = BinanceAuth::new("test_key", "test_secret");
        let mut params = BTreeMap::new();
        params.insert("filter".to_string(), "price=100".to_string());

        let query = auth.build_query_string(&params);

        // = in value should be encoded as %3D
        assert!(query.contains("filter=price%3D100"));
    }

    #[test]
    fn test_signature_consistency_with_encoding() {
        let auth = BinanceAuth::new("test_key", "test_secret");
        let mut params = BTreeMap::new();
        params.insert("symbol".to_string(), "BTC/USDT".to_string());
        params.insert("side".to_string(), "BUY".to_string());

        // Sign the same params twice
        let signed1 = auth.sign_params(&params).expect("Sign failed");
        let signed2 = auth.sign_params(&params).expect("Sign failed");

        // Signatures should be identical for same input
        assert_eq!(
            signed1.get("signature").expect("Missing sig1"),
            signed2.get("signature").expect("Missing sig2")
        );
    }

    #[test]
    fn test_signature_changes_with_different_params() {
        let auth = BinanceAuth::new("test_key", "test_secret");

        let mut params1 = BTreeMap::new();
        params1.insert("symbol".to_string(), "BTCUSDT".to_string());

        let mut params2 = BTreeMap::new();
        params2.insert("symbol".to_string(), "ETHUSDT".to_string());

        let signed1 = auth.sign_params(&params1).expect("Sign failed");
        let signed2 = auth.sign_params(&params2).expect("Sign failed");

        // Signatures should be different for different input
        assert_ne!(
            signed1.get("signature").expect("Missing sig1"),
            signed2.get("signature").expect("Missing sig2")
        );
    }

    #[test]
    fn test_build_signed_url_with_special_chars() {
        let mut params = HashMap::new();
        params.insert("clientOrderId".to_string(), "test+order".to_string());
        params.insert("signature".to_string(), "abc123".to_string());

        let url = build_signed_url("https://api.binance.com", "/api/v3/order", &params);

        // Special chars in values should be URL encoded
        assert!(url.contains("clientOrderId=test%2Border"));
    }

    #[test]
    fn test_alphabetical_sorting() {
        let auth = BinanceAuth::new("test_key", "test_secret");
        let mut params = BTreeMap::new();
        params.insert("zebra".to_string(), "z".to_string());
        params.insert("apple".to_string(), "a".to_string());
        params.insert("mango".to_string(), "m".to_string());

        let query = auth.build_query_string(&params);

        // BTreeMap ensures alphabetical order
        assert_eq!(query, "apple=a&mango=m&zebra=z");
    }
}
