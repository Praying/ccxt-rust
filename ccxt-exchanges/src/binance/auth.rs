//! Binance authentication and signature module.
//!
//! Implements HMAC-SHA256 signature algorithm for API request authentication.

use ccxt_core::{Error, Result};
use hmac::{Hmac, Mac};
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use sha2::Sha256;
use std::collections::HashMap;

type HmacSha256 = Hmac<Sha256>;

/// Binance authenticator.
#[derive(Debug, Clone)]
pub struct BinanceAuth {
    /// API key.
    api_key: String,
    /// Secret key.
    secret: String,
}

impl BinanceAuth {
    /// Creates a new authenticator.
    ///
    /// # Arguments
    ///
    /// * `api_key` - API key
    /// * `secret` - Secret key
    pub fn new(api_key: impl Into<String>, secret: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            secret: secret.into(),
        }
    }

    /// Returns the API key.
    pub fn api_key(&self) -> &str {
        &self.api_key
    }

    /// Returns the secret key.
    pub fn secret(&self) -> &str {
        &self.secret
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
        let mut mac = HmacSha256::new_from_slice(self.secret.as_bytes())
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
    pub fn sign_params(&self, params: &HashMap<String, String>) -> Result<HashMap<String, String>> {
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
        params: &HashMap<String, String>,
        timestamp: u64,
        recv_window: Option<u64>,
    ) -> Result<HashMap<String, String>> {
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
    pub(crate) fn build_query_string(&self, params: &HashMap<String, String>) -> String {
        let mut pairs: Vec<_> = params.iter().collect();
        pairs.sort_by_key(|(k, _)| *k);

        let query_string = pairs
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<_>>()
            .join("&");
        query_string
    }

    /// Adds authentication headers to the request.
    ///
    /// # Arguments
    ///
    /// * `headers` - Existing header map to modify
    pub fn add_auth_headers(&self, headers: &mut HashMap<String, String>) {
        headers.insert("X-MBX-APIKEY".to_string(), self.api_key.clone());
    }

    /// Adds authentication headers to a `reqwest` request.
    ///
    /// # Arguments
    ///
    /// * `headers` - Reqwest `HeaderMap` to modify
    pub fn add_auth_headers_reqwest(&self, headers: &mut HeaderMap) {
        if let Ok(header_name) = HeaderName::from_bytes(b"X-MBX-APIKEY") {
            if let Ok(header_value) = HeaderValue::from_str(&self.api_key) {
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
/// Returns the complete URL with query parameters.
pub fn build_signed_url(
    base_url: &str,
    endpoint: &str,
    params: &HashMap<String, String>,
) -> String {
    let query_string = params
        .iter()
        .map(|(k, v)| format!("{}={}", k, v))
        .collect::<Vec<_>>()
        .join("&");

    if query_string.is_empty() {
        format!("{}{}", base_url, endpoint)
    } else {
        format!("{}{}?{}", base_url, endpoint, query_string)
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

        let sig = signature.unwrap();
        assert!(!sig.is_empty());
        assert_eq!(sig.len(), 64); // HMAC-SHA256 produces 64 hex characters
    }

    #[test]
    fn test_sign_params() {
        let auth = BinanceAuth::new("test_key", "test_secret");
        let mut params = HashMap::new();
        params.insert("symbol".to_string(), "BTCUSDT".to_string());
        params.insert("side".to_string(), "BUY".to_string());

        let signed = auth.sign_params(&params);
        assert!(signed.is_ok());

        let signed_params = signed.unwrap();
        assert!(signed_params.contains_key("signature"));
        assert_eq!(signed_params.get("symbol").unwrap(), "BTCUSDT");
    }

    #[test]
    fn test_sign_with_timestamp() {
        let auth = BinanceAuth::new("test_key", "test_secret");
        let params = HashMap::new();
        let timestamp = 1234567890u64;

        let signed = auth.sign_with_timestamp(&params, timestamp, Some(5000));
        assert!(signed.is_ok());

        let signed_params = signed.unwrap();
        assert!(signed_params.contains_key("timestamp"));
        assert!(signed_params.contains_key("recvWindow"));
        assert!(signed_params.contains_key("signature"));
        assert_eq!(signed_params.get("timestamp").unwrap(), "1234567890");
        assert_eq!(signed_params.get("recvWindow").unwrap(), "5000");
    }

    #[test]
    fn test_build_query_string() {
        let auth = BinanceAuth::new("test_key", "test_secret");
        let mut params = HashMap::new();
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
        assert_eq!(headers.get("X-MBX-APIKEY").unwrap(), "my_api_key");
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
}
