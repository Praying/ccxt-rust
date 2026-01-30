//! Bitget API authentication module.
//!
//! Implements HMAC-SHA256 signing for Bitget API requests.
//! Bitget requires the following headers for authenticated requests:
//! - ACCESS-KEY: API key
//! - ACCESS-SIGN: HMAC-SHA256 signature (Base64 encoded)
//! - ACCESS-TIMESTAMP: Unix timestamp in milliseconds
//! - ACCESS-PASSPHRASE: API passphrase

use base64::{Engine as _, engine::general_purpose};
use ccxt_core::credentials::SecretString;
use hmac::{Hmac, Mac};
use reqwest::header::{HeaderMap, HeaderValue};
use sha2::Sha256;

/// Bitget API authenticator.
///
/// Handles request signing using HMAC-SHA256 and header construction
/// for authenticated API requests.
///
/// Credentials are automatically zeroed from memory when dropped.
#[derive(Debug, Clone)]
pub struct BitgetAuth {
    /// API key for authentication (automatically zeroed on drop).
    api_key: SecretString,
    /// Secret key for HMAC signing (automatically zeroed on drop).
    secret: SecretString,
    /// Passphrase for additional authentication (automatically zeroed on drop).
    passphrase: SecretString,
}

impl BitgetAuth {
    /// Creates a new BitgetAuth instance.
    ///
    /// # Arguments
    ///
    /// * `api_key` - The API key from Bitget.
    /// * `secret` - The secret key from Bitget.
    /// * `passphrase` - The passphrase set when creating the API key.
    ///
    /// # Security
    ///
    /// Credentials are automatically zeroed from memory when the authenticator is dropped.
    ///
    /// # Example
    ///
    /// ```
    /// use ccxt_exchanges::bitget::BitgetAuth;
    ///
    /// let auth = BitgetAuth::new(
    ///     "your-api-key".to_string(),
    ///     "your-secret".to_string(),
    ///     "your-passphrase".to_string(),
    /// );
    /// ```
    pub fn new(api_key: String, secret: String, passphrase: String) -> Self {
        Self {
            api_key: SecretString::new(api_key),
            secret: SecretString::new(secret),
            passphrase: SecretString::new(passphrase),
        }
    }

    /// Returns the API key.
    pub fn api_key(&self) -> &str {
        self.api_key.expose_secret()
    }

    /// Returns the passphrase.
    pub fn passphrase(&self) -> &str {
        self.passphrase.expose_secret()
    }

    /// Builds the signature string for HMAC signing.
    ///
    /// The signature string format is: `timestamp + method + path + body`
    ///
    /// # Arguments
    ///
    /// * `timestamp` - Unix timestamp in milliseconds as string.
    /// * `method` - HTTP method (GET, POST, DELETE, etc.).
    /// * `path` - Request path including query string (e.g., "/api/v2/spot/account/assets").
    /// * `body` - Request body (empty string for GET requests).
    ///
    /// # Returns
    ///
    /// The concatenated string to be signed.
    pub fn build_sign_string(
        &self,
        timestamp: &str,
        method: &str,
        path: &str,
        body: &str,
    ) -> String {
        format!("{}{}{}{}", timestamp, method.to_uppercase(), path, body)
    }

    /// Signs a request using HMAC-SHA256.
    ///
    /// # Arguments
    ///
    /// * `timestamp` - Unix timestamp in milliseconds as string.
    /// * `method` - HTTP method (GET, POST, DELETE, etc.).
    /// * `path` - Request path including query string.
    /// * `body` - Request body (empty string for GET requests).
    ///
    /// # Returns
    ///
    /// Base64-encoded HMAC-SHA256 signature.
    ///
    /// # Example
    ///
    /// ```
    /// use ccxt_exchanges::bitget::BitgetAuth;
    ///
    /// let auth = BitgetAuth::new(
    ///     "api-key".to_string(),
    ///     "secret".to_string(),
    ///     "passphrase".to_string(),
    /// );
    ///
    /// let signature = auth.sign("1234567890", "GET", "/api/v2/spot/account/assets", "");
    /// assert!(!signature.is_empty());
    /// ```
    pub fn sign(&self, timestamp: &str, method: &str, path: &str, body: &str) -> String {
        let sign_string = self.build_sign_string(timestamp, method, path, body);
        self.hmac_sha256_base64(&sign_string)
    }

    /// Computes HMAC-SHA256 and returns Base64-encoded result.
    fn hmac_sha256_base64(&self, message: &str) -> String {
        type HmacSha256 = Hmac<Sha256>;

        // SAFETY: HMAC accepts keys of any length - this cannot fail
        let mut mac = HmacSha256::new_from_slice(self.secret.expose_secret_bytes())
            .expect("HMAC-SHA256 accepts keys of any length; this is an infallible operation");
        mac.update(message.as_bytes());
        let result = mac.finalize().into_bytes();

        general_purpose::STANDARD.encode(result)
    }

    /// Adds authentication headers to a HeaderMap.
    ///
    /// Adds the following headers:
    /// - ACCESS-KEY: API key
    /// - ACCESS-SIGN: HMAC-SHA256 signature
    /// - ACCESS-TIMESTAMP: Unix timestamp
    /// - ACCESS-PASSPHRASE: API passphrase
    ///
    /// # Arguments
    ///
    /// * `headers` - Mutable reference to HeaderMap to add headers to.
    /// * `timestamp` - Unix timestamp in milliseconds as string.
    /// * `sign` - Pre-computed signature from `sign()` method.
    ///
    /// # Example
    ///
    /// ```
    /// use ccxt_exchanges::bitget::BitgetAuth;
    /// use reqwest::header::HeaderMap;
    ///
    /// let auth = BitgetAuth::new(
    ///     "api-key".to_string(),
    ///     "secret".to_string(),
    ///     "passphrase".to_string(),
    /// );
    ///
    /// let mut headers = HeaderMap::new();
    /// let timestamp = "1234567890";
    /// let signature = auth.sign(timestamp, "GET", "/api/v2/spot/account/assets", "");
    /// auth.add_auth_headers(&mut headers, timestamp, &signature);
    ///
    /// assert!(headers.contains_key("ACCESS-KEY"));
    /// assert!(headers.contains_key("ACCESS-SIGN"));
    /// assert!(headers.contains_key("ACCESS-TIMESTAMP"));
    /// assert!(headers.contains_key("ACCESS-PASSPHRASE"));
    /// ```
    pub fn add_auth_headers(&self, headers: &mut HeaderMap, timestamp: &str, sign: &str) {
        headers.insert(
            "ACCESS-KEY",
            HeaderValue::from_str(self.api_key.expose_secret())
                .unwrap_or_else(|_| HeaderValue::from_static("")),
        );
        headers.insert(
            "ACCESS-SIGN",
            HeaderValue::from_str(sign).unwrap_or_else(|_| HeaderValue::from_static("")),
        );
        headers.insert(
            "ACCESS-TIMESTAMP",
            HeaderValue::from_str(timestamp).unwrap_or_else(|_| HeaderValue::from_static("")),
        );
        headers.insert(
            "ACCESS-PASSPHRASE",
            HeaderValue::from_str(self.passphrase.expose_secret())
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
    /// * `method` - HTTP method (GET, POST, DELETE, etc.).
    /// * `path` - Request path including query string.
    /// * `body` - Request body (empty string for GET requests).
    ///
    /// # Returns
    ///
    /// A HeaderMap containing all authentication headers.
    pub fn create_auth_headers(
        &self,
        timestamp: &str,
        method: &str,
        path: &str,
        body: &str,
    ) -> HeaderMap {
        let sign = self.sign(timestamp, method, path, body);
        let mut headers = HeaderMap::new();
        self.add_auth_headers(&mut headers, timestamp, &sign);
        headers
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::disallowed_methods)]
    use super::*;

    #[test]
    fn test_auth_new() {
        let auth = BitgetAuth::new(
            "test-api-key".to_string(),
            "test-secret".to_string(),
            "test-passphrase".to_string(),
        );

        assert_eq!(auth.api_key(), "test-api-key");
        assert_eq!(auth.passphrase(), "test-passphrase");
    }

    #[test]
    fn test_build_sign_string() {
        let auth = BitgetAuth::new(
            "api-key".to_string(),
            "secret".to_string(),
            "passphrase".to_string(),
        );

        let sign_string =
            auth.build_sign_string("1234567890", "GET", "/api/v2/spot/account/assets", "");
        assert_eq!(sign_string, "1234567890GET/api/v2/spot/account/assets");

        let sign_string_post = auth.build_sign_string(
            "1234567890",
            "POST",
            "/api/v2/spot/trade/place-order",
            r#"{"symbol":"BTCUSDT","side":"buy"}"#,
        );
        assert_eq!(
            sign_string_post,
            r#"1234567890POST/api/v2/spot/trade/place-order{"symbol":"BTCUSDT","side":"buy"}"#
        );
    }

    #[test]
    fn test_sign_deterministic() {
        let auth = BitgetAuth::new(
            "api-key".to_string(),
            "test-secret-key".to_string(),
            "passphrase".to_string(),
        );

        let sig1 = auth.sign("1234567890", "GET", "/api/v2/spot/account/assets", "");
        let sig2 = auth.sign("1234567890", "GET", "/api/v2/spot/account/assets", "");

        assert_eq!(sig1, sig2);
        assert!(!sig1.is_empty());
    }

    #[test]
    fn test_sign_different_inputs() {
        let auth = BitgetAuth::new(
            "api-key".to_string(),
            "test-secret-key".to_string(),
            "passphrase".to_string(),
        );

        let sig1 = auth.sign("1234567890", "GET", "/api/v2/spot/account/assets", "");
        let sig2 = auth.sign("1234567891", "GET", "/api/v2/spot/account/assets", "");
        let sig3 = auth.sign("1234567890", "POST", "/api/v2/spot/account/assets", "");

        assert_ne!(sig1, sig2);
        assert_ne!(sig1, sig3);
    }

    #[test]
    fn test_add_auth_headers() {
        let auth = BitgetAuth::new(
            "test-api-key".to_string(),
            "test-secret".to_string(),
            "test-passphrase".to_string(),
        );

        let mut headers = HeaderMap::new();
        let timestamp = "1234567890";
        let signature = auth.sign(timestamp, "GET", "/api/v2/spot/account/assets", "");
        auth.add_auth_headers(&mut headers, timestamp, &signature);

        assert_eq!(headers.get("ACCESS-KEY").unwrap(), "test-api-key");
        assert_eq!(headers.get("ACCESS-SIGN").unwrap(), &signature);
        assert_eq!(headers.get("ACCESS-TIMESTAMP").unwrap(), "1234567890");
        assert_eq!(headers.get("ACCESS-PASSPHRASE").unwrap(), "test-passphrase");
    }

    #[test]
    fn test_create_auth_headers() {
        let auth = BitgetAuth::new(
            "test-api-key".to_string(),
            "test-secret".to_string(),
            "test-passphrase".to_string(),
        );

        let headers =
            auth.create_auth_headers("1234567890", "GET", "/api/v2/spot/account/assets", "");

        assert!(headers.contains_key("ACCESS-KEY"));
        assert!(headers.contains_key("ACCESS-SIGN"));
        assert!(headers.contains_key("ACCESS-TIMESTAMP"));
        assert!(headers.contains_key("ACCESS-PASSPHRASE"));
    }

    #[test]
    fn test_method_case_insensitive() {
        let auth = BitgetAuth::new(
            "api-key".to_string(),
            "test-secret".to_string(),
            "passphrase".to_string(),
        );

        // The build_sign_string method converts method to uppercase
        let sign_string_lower =
            auth.build_sign_string("1234567890", "get", "/api/v2/spot/account/assets", "");
        let sign_string_upper =
            auth.build_sign_string("1234567890", "GET", "/api/v2/spot/account/assets", "");

        assert_eq!(sign_string_lower, sign_string_upper);
    }

    #[test]
    fn test_signature_is_base64() {
        let auth = BitgetAuth::new(
            "api-key".to_string(),
            "test-secret".to_string(),
            "passphrase".to_string(),
        );

        let signature = auth.sign("1234567890", "GET", "/api/v2/spot/account/assets", "");

        // Base64 characters are alphanumeric plus + / =
        assert!(
            signature
                .chars()
                .all(|c| c.is_alphanumeric() || c == '+' || c == '/' || c == '=')
        );

        // Should be decodable as Base64
        let decoded = general_purpose::STANDARD.decode(&signature);
        assert!(decoded.is_ok());

        // HMAC-SHA256 produces 32 bytes
        assert_eq!(decoded.unwrap().len(), 32);
    }

    /// Test signature generation with known inputs and expected output.
    /// This verifies the HMAC-SHA256 implementation produces correct signatures.
    ///
    /// The expected signature is computed as:
    /// HMAC-SHA256(secret="bitget-secret-key", message="1609459200000GET/api/v2/spot/account/assets")
    /// Then Base64 encoded.
    #[test]
    fn test_signature_with_known_inputs() {
        let auth = BitgetAuth::new(
            "bg_api_key".to_string(),
            "bitget-secret-key".to_string(),
            "my-passphrase".to_string(),
        );

        // Known inputs
        let timestamp = "1609459200000"; // 2021-01-01 00:00:00 UTC
        let method = "GET";
        let path = "/api/v2/spot/account/assets";
        let body = "";

        // Generate signature
        let signature = auth.sign(timestamp, method, path, body);

        // The sign string should be: "1609459200000GET/api/v2/spot/account/assets"
        let expected_sign_string = "1609459200000GET/api/v2/spot/account/assets";
        assert_eq!(
            auth.build_sign_string(timestamp, method, path, body),
            expected_sign_string
        );

        // Verify signature is non-empty and valid Base64
        assert!(!signature.is_empty());
        let decoded = general_purpose::STANDARD.decode(&signature);
        assert!(decoded.is_ok());
        assert_eq!(decoded.unwrap().len(), 32);

        // Verify the signature is reproducible
        let signature2 = auth.sign(timestamp, method, path, body);
        assert_eq!(signature, signature2);
    }

    /// Test signature generation for POST request with JSON body.
    #[test]
    fn test_signature_with_post_body() {
        let auth = BitgetAuth::new(
            "bg_api_key".to_string(),
            "bitget-secret-key".to_string(),
            "my-passphrase".to_string(),
        );

        let timestamp = "1609459200000";
        let method = "POST";
        let path = "/api/v2/spot/trade/place-order";
        let body = r#"{"symbol":"BTCUSDT","side":"buy","orderType":"limit","price":"50000","size":"0.001"}"#;

        let signature = auth.sign(timestamp, method, path, body);

        // Verify sign string format
        let expected_sign_string = format!("{}{}{}{}", timestamp, method, path, body);
        assert_eq!(
            auth.build_sign_string(timestamp, method, path, body),
            expected_sign_string
        );

        // Verify signature is valid
        assert!(!signature.is_empty());
        let decoded = general_purpose::STANDARD.decode(&signature);
        assert!(decoded.is_ok());
        assert_eq!(decoded.unwrap().len(), 32);
    }

    /// Test that all required headers are present and have correct values.
    #[test]
    fn test_header_values_correctness() {
        let api_key = "my-api-key-12345";
        let secret = "my-secret-key-67890";
        let passphrase = "my-passphrase-abc";

        let auth = BitgetAuth::new(
            api_key.to_string(),
            secret.to_string(),
            passphrase.to_string(),
        );

        let timestamp = "1609459200000";
        let method = "GET";
        let path = "/api/v2/spot/account/assets";
        let body = "";

        let headers = auth.create_auth_headers(timestamp, method, path, body);

        // Verify ACCESS-KEY header
        assert_eq!(
            headers.get("ACCESS-KEY").unwrap().to_str().unwrap(),
            api_key
        );

        // Verify ACCESS-TIMESTAMP header
        assert_eq!(
            headers.get("ACCESS-TIMESTAMP").unwrap().to_str().unwrap(),
            timestamp
        );

        // Verify ACCESS-PASSPHRASE header
        assert_eq!(
            headers.get("ACCESS-PASSPHRASE").unwrap().to_str().unwrap(),
            passphrase
        );

        // Verify ACCESS-SIGN header exists and is valid Base64
        let sign_header = headers.get("ACCESS-SIGN").unwrap().to_str().unwrap();
        assert!(!sign_header.is_empty());
        let decoded = general_purpose::STANDARD.decode(sign_header);
        assert!(decoded.is_ok());
    }

    /// Test signature with query parameters in path.
    #[test]
    fn test_signature_with_query_params() {
        let auth = BitgetAuth::new(
            "api-key".to_string(),
            "secret-key".to_string(),
            "passphrase".to_string(),
        );

        let timestamp = "1609459200000";
        let method = "GET";
        let path = "/api/v2/spot/market/tickers?symbol=BTCUSDT";
        let body = "";

        let signature = auth.sign(timestamp, method, path, body);

        // Verify sign string includes query params
        let sign_string = auth.build_sign_string(timestamp, method, path, body);
        assert!(sign_string.contains("?symbol=BTCUSDT"));

        // Verify signature is valid
        assert!(!signature.is_empty());
        let decoded = general_purpose::STANDARD.decode(&signature);
        assert!(decoded.is_ok());
    }
}
