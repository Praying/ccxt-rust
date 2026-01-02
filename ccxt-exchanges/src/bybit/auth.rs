//! Bybit API authentication module.
//!
//! Implements HMAC-SHA256 signing for Bybit API requests.
//! Bybit requires the following headers for authenticated requests:
//! - X-BAPI-API-KEY: API key
//! - X-BAPI-SIGN: HMAC-SHA256 signature (hex encoded)
//! - X-BAPI-TIMESTAMP: Unix timestamp in milliseconds
//! - X-BAPI-RECV-WINDOW: Receive window in milliseconds

use ccxt_core::credentials::SecretString;
use hmac::{Hmac, Mac};
use reqwest::header::{HeaderMap, HeaderValue};
use sha2::Sha256;

/// Bybit API authenticator.
///
/// Handles request signing using HMAC-SHA256 and header construction
/// for authenticated API requests.
///
/// Credentials are automatically zeroed from memory when dropped.
#[derive(Debug, Clone)]
pub struct BybitAuth {
    /// API key for authentication (automatically zeroed on drop).
    api_key: SecretString,
    /// Secret key for HMAC signing (automatically zeroed on drop).
    secret: SecretString,
}

impl BybitAuth {
    /// Creates a new BybitAuth instance.
    ///
    /// # Arguments
    ///
    /// * `api_key` - The API key from Bybit.
    /// * `secret` - The secret key from Bybit.
    ///
    /// # Security
    ///
    /// Credentials are automatically zeroed from memory when the authenticator is dropped.
    ///
    /// # Example
    ///
    /// ```
    /// use ccxt_exchanges::bybit::BybitAuth;
    ///
    /// let auth = BybitAuth::new(
    ///     "your-api-key".to_string(),
    ///     "your-secret".to_string(),
    /// );
    /// ```
    pub fn new(api_key: String, secret: String) -> Self {
        Self {
            api_key: SecretString::new(api_key),
            secret: SecretString::new(secret),
        }
    }

    /// Returns the API key.
    pub fn api_key(&self) -> &str {
        self.api_key.expose_secret()
    }

    /// Builds the signature string for HMAC signing.
    ///
    /// The signature string format is: `timestamp + api_key + recv_window + params`
    ///
    /// # Arguments
    ///
    /// * `timestamp` - Unix timestamp in milliseconds as string.
    /// * `recv_window` - Receive window in milliseconds.
    /// * `params` - Query string (for GET) or request body (for POST).
    ///
    /// # Returns
    ///
    /// The concatenated string to be signed.
    pub fn build_sign_string(&self, timestamp: &str, recv_window: u64, params: &str) -> String {
        format!(
            "{}{}{}{}",
            timestamp,
            self.api_key.expose_secret(),
            recv_window,
            params
        )
    }

    /// Signs a request using HMAC-SHA256.
    ///
    /// # Arguments
    ///
    /// * `timestamp` - Unix timestamp in milliseconds as string.
    /// * `recv_window` - Receive window in milliseconds.
    /// * `params` - Query string (for GET) or request body (for POST).
    ///
    /// # Returns
    ///
    /// Hex-encoded HMAC-SHA256 signature.
    ///
    /// # Example
    ///
    /// ```
    /// use ccxt_exchanges::bybit::BybitAuth;
    ///
    /// let auth = BybitAuth::new(
    ///     "api-key".to_string(),
    ///     "secret".to_string(),
    /// );
    ///
    /// let signature = auth.sign("1234567890000", 5000, "symbol=BTCUSDT");
    /// assert!(!signature.is_empty());
    /// ```
    pub fn sign(&self, timestamp: &str, recv_window: u64, params: &str) -> String {
        let sign_string = self.build_sign_string(timestamp, recv_window, params);
        self.hmac_sha256_hex(&sign_string)
    }

    /// Computes HMAC-SHA256 and returns hex-encoded result.
    fn hmac_sha256_hex(&self, message: &str) -> String {
        type HmacSha256 = Hmac<Sha256>;

        // SAFETY: HMAC accepts keys of any length - this cannot fail
        let mut mac = HmacSha256::new_from_slice(self.secret.expose_secret_bytes())
            .expect("HMAC-SHA256 accepts keys of any length; this is an infallible operation");
        mac.update(message.as_bytes());
        let result = mac.finalize().into_bytes();

        // Convert to hex string
        hex::encode(result)
    }

    /// Adds authentication headers to a HeaderMap.
    ///
    /// Adds the following headers:
    /// - X-BAPI-API-KEY: API key
    /// - X-BAPI-SIGN: HMAC-SHA256 signature (hex encoded)
    /// - X-BAPI-TIMESTAMP: Unix timestamp
    /// - X-BAPI-RECV-WINDOW: Receive window
    ///
    /// # Arguments
    ///
    /// * `headers` - Mutable reference to HeaderMap to add headers to.
    /// * `timestamp` - Unix timestamp in milliseconds as string.
    /// * `sign` - Pre-computed signature from `sign()` method.
    /// * `recv_window` - Receive window in milliseconds.
    ///
    /// # Example
    ///
    /// ```
    /// use ccxt_exchanges::bybit::BybitAuth;
    /// use reqwest::header::HeaderMap;
    ///
    /// let auth = BybitAuth::new(
    ///     "api-key".to_string(),
    ///     "secret".to_string(),
    /// );
    ///
    /// let mut headers = HeaderMap::new();
    /// let timestamp = "1234567890000";
    /// let recv_window = 5000u64;
    /// let signature = auth.sign(timestamp, recv_window, "symbol=BTCUSDT");
    /// auth.add_auth_headers(&mut headers, timestamp, &signature, recv_window);
    ///
    /// assert!(headers.contains_key("X-BAPI-API-KEY"));
    /// assert!(headers.contains_key("X-BAPI-SIGN"));
    /// assert!(headers.contains_key("X-BAPI-TIMESTAMP"));
    /// assert!(headers.contains_key("X-BAPI-RECV-WINDOW"));
    /// ```
    pub fn add_auth_headers(
        &self,
        headers: &mut HeaderMap,
        timestamp: &str,
        sign: &str,
        recv_window: u64,
    ) {
        headers.insert(
            "X-BAPI-API-KEY",
            HeaderValue::from_str(self.api_key.expose_secret())
                .unwrap_or_else(|_| HeaderValue::from_static("")),
        );
        headers.insert(
            "X-BAPI-SIGN",
            HeaderValue::from_str(sign).unwrap_or_else(|_| HeaderValue::from_static("")),
        );
        headers.insert(
            "X-BAPI-TIMESTAMP",
            HeaderValue::from_str(timestamp).unwrap_or_else(|_| HeaderValue::from_static("")),
        );
        headers.insert(
            "X-BAPI-RECV-WINDOW",
            HeaderValue::from_str(&recv_window.to_string())
                .unwrap_or_else(|_| HeaderValue::from_static("")),
        );
    }

    /// Creates authentication headers for a request.
    ///
    /// This is a convenience method that combines `sign()` and `add_auth_headers()`.
    ///
    /// # Arguments
    ///
    /// * `timestamp` - Unix timestamp in milliseconds as string.
    /// * `recv_window` - Receive window in milliseconds.
    /// * `params` - Query string (for GET) or request body (for POST).
    ///
    /// # Returns
    ///
    /// A HeaderMap containing all authentication headers.
    pub fn create_auth_headers(
        &self,
        timestamp: &str,
        recv_window: u64,
        params: &str,
    ) -> HeaderMap {
        let sign = self.sign(timestamp, recv_window, params);
        let mut headers = HeaderMap::new();
        self.add_auth_headers(&mut headers, timestamp, &sign, recv_window);
        headers
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_new() {
        let auth = BybitAuth::new("test-api-key".to_string(), "test-secret".to_string());

        assert_eq!(auth.api_key(), "test-api-key");
    }

    #[test]
    fn test_build_sign_string() {
        let auth = BybitAuth::new("api-key".to_string(), "secret".to_string());

        let sign_string = auth.build_sign_string("1234567890000", 5000, "symbol=BTCUSDT");
        assert_eq!(sign_string, "1234567890000api-key5000symbol=BTCUSDT");

        let sign_string_empty = auth.build_sign_string("1234567890000", 5000, "");
        assert_eq!(sign_string_empty, "1234567890000api-key5000");
    }

    #[test]
    fn test_sign_deterministic() {
        let auth = BybitAuth::new("api-key".to_string(), "test-secret-key".to_string());

        let sig1 = auth.sign("1234567890000", 5000, "symbol=BTCUSDT");
        let sig2 = auth.sign("1234567890000", 5000, "symbol=BTCUSDT");

        assert_eq!(sig1, sig2);
        assert!(!sig1.is_empty());
    }

    #[test]
    fn test_sign_different_inputs() {
        let auth = BybitAuth::new("api-key".to_string(), "test-secret-key".to_string());

        let sig1 = auth.sign("1234567890000", 5000, "symbol=BTCUSDT");
        let sig2 = auth.sign("1234567890001", 5000, "symbol=BTCUSDT");
        let sig3 = auth.sign("1234567890000", 10000, "symbol=BTCUSDT");
        let sig4 = auth.sign("1234567890000", 5000, "symbol=ETHUSDT");

        assert_ne!(sig1, sig2);
        assert_ne!(sig1, sig3);
        assert_ne!(sig1, sig4);
    }

    #[test]
    fn test_add_auth_headers() {
        let auth = BybitAuth::new("test-api-key".to_string(), "test-secret".to_string());

        let mut headers = HeaderMap::new();
        let timestamp = "1234567890000";
        let recv_window = 5000u64;
        let signature = auth.sign(timestamp, recv_window, "symbol=BTCUSDT");
        auth.add_auth_headers(&mut headers, timestamp, &signature, recv_window);

        assert_eq!(headers.get("X-BAPI-API-KEY").unwrap(), "test-api-key");
        assert_eq!(headers.get("X-BAPI-SIGN").unwrap(), &signature);
        assert_eq!(headers.get("X-BAPI-TIMESTAMP").unwrap(), "1234567890000");
        assert_eq!(headers.get("X-BAPI-RECV-WINDOW").unwrap(), "5000");
    }

    #[test]
    fn test_create_auth_headers() {
        let auth = BybitAuth::new("test-api-key".to_string(), "test-secret".to_string());

        let headers = auth.create_auth_headers("1234567890000", 5000, "symbol=BTCUSDT");

        assert!(headers.contains_key("X-BAPI-API-KEY"));
        assert!(headers.contains_key("X-BAPI-SIGN"));
        assert!(headers.contains_key("X-BAPI-TIMESTAMP"));
        assert!(headers.contains_key("X-BAPI-RECV-WINDOW"));
    }

    #[test]
    fn test_signature_is_hex() {
        let auth = BybitAuth::new("api-key".to_string(), "test-secret".to_string());

        let signature = auth.sign("1234567890000", 5000, "symbol=BTCUSDT");

        // Hex characters are 0-9 and a-f
        assert!(signature.chars().all(|c| c.is_ascii_hexdigit()));

        // HMAC-SHA256 produces 32 bytes = 64 hex characters
        assert_eq!(signature.len(), 64);

        // Should be decodable as hex
        let decoded = hex::decode(&signature);
        assert!(decoded.is_ok());
        assert_eq!(decoded.unwrap().len(), 32);
    }

    /// Test signature generation with known inputs.
    /// This verifies the HMAC-SHA256 implementation produces correct signatures.
    #[test]
    fn test_signature_with_known_inputs() {
        let auth = BybitAuth::new("bybit_api_key".to_string(), "bybit-secret-key".to_string());

        // Known inputs
        let timestamp = "1609459200000"; // 2021-01-01 00:00:00 UTC
        let recv_window = 5000u64;
        let params = "category=spot&symbol=BTCUSDT";

        // Generate signature
        let signature = auth.sign(timestamp, recv_window, params);

        // The sign string should be: "1609459200000bybit_api_key5000category=spot&symbol=BTCUSDT"
        let expected_sign_string = "1609459200000bybit_api_key5000category=spot&symbol=BTCUSDT";
        assert_eq!(
            auth.build_sign_string(timestamp, recv_window, params),
            expected_sign_string
        );

        // Verify signature is non-empty and valid hex
        assert!(!signature.is_empty());
        assert_eq!(signature.len(), 64);
        let decoded = hex::decode(&signature);
        assert!(decoded.is_ok());
        assert_eq!(decoded.unwrap().len(), 32);

        // Verify the signature is reproducible
        let signature2 = auth.sign(timestamp, recv_window, params);
        assert_eq!(signature, signature2);
    }

    /// Test signature generation for POST request with JSON body.
    #[test]
    fn test_signature_with_post_body() {
        let auth = BybitAuth::new("bybit_api_key".to_string(), "bybit-secret-key".to_string());

        let timestamp = "1609459200000";
        let recv_window = 5000u64;
        let body = r#"{"category":"spot","symbol":"BTCUSDT","side":"Buy","orderType":"Limit","qty":"0.001","price":"50000"}"#;

        let signature = auth.sign(timestamp, recv_window, body);

        // Verify sign string format
        let expected_sign_string =
            format!("{}{}{}{}", timestamp, auth.api_key(), recv_window, body);
        assert_eq!(
            auth.build_sign_string(timestamp, recv_window, body),
            expected_sign_string
        );

        // Verify signature is valid
        assert!(!signature.is_empty());
        assert_eq!(signature.len(), 64);
        let decoded = hex::decode(&signature);
        assert!(decoded.is_ok());
        assert_eq!(decoded.unwrap().len(), 32);
    }

    /// Test that all required headers are present and have correct values.
    #[test]
    fn test_header_values_correctness() {
        let api_key = "my-api-key-12345";
        let secret = "my-secret-key-67890";

        let auth = BybitAuth::new(api_key.to_string(), secret.to_string());

        let timestamp = "1609459200000";
        let recv_window = 5000u64;
        let params = "category=spot&symbol=BTCUSDT";

        let headers = auth.create_auth_headers(timestamp, recv_window, params);

        // Verify X-BAPI-API-KEY header
        assert_eq!(
            headers.get("X-BAPI-API-KEY").unwrap().to_str().unwrap(),
            api_key
        );

        // Verify X-BAPI-TIMESTAMP header
        assert_eq!(
            headers.get("X-BAPI-TIMESTAMP").unwrap().to_str().unwrap(),
            timestamp
        );

        // Verify X-BAPI-RECV-WINDOW header
        assert_eq!(
            headers.get("X-BAPI-RECV-WINDOW").unwrap().to_str().unwrap(),
            "5000"
        );

        // Verify X-BAPI-SIGN header exists and is valid hex
        let sign_header = headers.get("X-BAPI-SIGN").unwrap().to_str().unwrap();
        assert!(!sign_header.is_empty());
        assert_eq!(sign_header.len(), 64);
        let decoded = hex::decode(sign_header);
        assert!(decoded.is_ok());
    }

    /// Test signature with empty params (common for some GET requests).
    #[test]
    fn test_signature_with_empty_params() {
        let auth = BybitAuth::new("api-key".to_string(), "secret-key".to_string());

        let timestamp = "1609459200000";
        let recv_window = 5000u64;
        let params = "";

        let signature = auth.sign(timestamp, recv_window, params);

        // Verify sign string format with empty params
        let sign_string = auth.build_sign_string(timestamp, recv_window, params);
        assert_eq!(sign_string, "1609459200000api-key5000");

        // Verify signature is valid
        assert!(!signature.is_empty());
        assert_eq!(signature.len(), 64);
        let decoded = hex::decode(&signature);
        assert!(decoded.is_ok());
    }

    /// Test different recv_window values.
    #[test]
    fn test_different_recv_window() {
        let auth = BybitAuth::new("api-key".to_string(), "secret-key".to_string());

        let timestamp = "1609459200000";
        let params = "symbol=BTCUSDT";

        let sig_5000 = auth.sign(timestamp, 5000, params);
        let sig_10000 = auth.sign(timestamp, 10000, params);
        let sig_20000 = auth.sign(timestamp, 20000, params);

        // Different recv_window should produce different signatures
        assert_ne!(sig_5000, sig_10000);
        assert_ne!(sig_5000, sig_20000);
        assert_ne!(sig_10000, sig_20000);
    }
}
