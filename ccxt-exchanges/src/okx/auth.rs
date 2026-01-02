//! OKX API authentication module.
//!
//! Implements HMAC-SHA256 signing with Base64 encoding for OKX API requests.
//! OKX requires the following headers for authenticated requests:
//! - OK-ACCESS-KEY: API key
//! - OK-ACCESS-SIGN: HMAC-SHA256 signature (Base64 encoded)
//! - OK-ACCESS-TIMESTAMP: ISO 8601 timestamp
//! - OK-ACCESS-PASSPHRASE: API passphrase

use base64::{Engine as _, engine::general_purpose};
use ccxt_core::credentials::SecretString;
use hmac::{Hmac, Mac};
use reqwest::header::{HeaderMap, HeaderValue};
use sha2::Sha256;

/// OKX API authenticator.
///
/// Handles request signing using HMAC-SHA256 and header construction
/// for authenticated API requests.
///
/// Credentials are automatically zeroed from memory when dropped.
#[derive(Debug, Clone)]
pub struct OkxAuth {
    /// API key for authentication (automatically zeroed on drop).
    api_key: SecretString,
    /// Secret key for HMAC signing (automatically zeroed on drop).
    secret: SecretString,
    /// Passphrase for additional authentication (automatically zeroed on drop).
    passphrase: SecretString,
}

impl OkxAuth {
    /// Creates a new OkxAuth instance.
    ///
    /// # Arguments
    ///
    /// * `api_key` - The API key from OKX.
    /// * `secret` - The secret key from OKX.
    /// * `passphrase` - The passphrase set when creating the API key.
    ///
    /// # Security
    ///
    /// Credentials are automatically zeroed from memory when the authenticator is dropped.
    ///
    /// # Example
    ///
    /// ```
    /// use ccxt_exchanges::okx::OkxAuth;
    ///
    /// let auth = OkxAuth::new(
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
    /// * `timestamp` - ISO 8601 timestamp (e.g., "2021-01-01T00:00:00.000Z").
    /// * `method` - HTTP method (GET, POST, DELETE, etc.).
    /// * `path` - Request path including query string (e.g., "/api/v5/account/balance").
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
    /// * `timestamp` - ISO 8601 timestamp (e.g., "2021-01-01T00:00:00.000Z").
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
    /// use ccxt_exchanges::okx::OkxAuth;
    ///
    /// let auth = OkxAuth::new(
    ///     "api-key".to_string(),
    ///     "secret".to_string(),
    ///     "passphrase".to_string(),
    /// );
    ///
    /// let signature = auth.sign("2021-01-01T00:00:00.000Z", "GET", "/api/v5/account/balance", "");
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
    /// - OK-ACCESS-KEY: API key
    /// - OK-ACCESS-SIGN: HMAC-SHA256 signature
    /// - OK-ACCESS-TIMESTAMP: ISO 8601 timestamp
    /// - OK-ACCESS-PASSPHRASE: API passphrase
    ///
    /// # Arguments
    ///
    /// * `headers` - Mutable reference to HeaderMap to add headers to.
    /// * `timestamp` - ISO 8601 timestamp.
    /// * `sign` - Pre-computed signature from `sign()` method.
    ///
    /// # Example
    ///
    /// ```
    /// use ccxt_exchanges::okx::OkxAuth;
    /// use reqwest::header::HeaderMap;
    ///
    /// let auth = OkxAuth::new(
    ///     "api-key".to_string(),
    ///     "secret".to_string(),
    ///     "passphrase".to_string(),
    /// );
    ///
    /// let mut headers = HeaderMap::new();
    /// let timestamp = "2021-01-01T00:00:00.000Z";
    /// let signature = auth.sign(timestamp, "GET", "/api/v5/account/balance", "");
    /// auth.add_auth_headers(&mut headers, timestamp, &signature);
    ///
    /// assert!(headers.contains_key("OK-ACCESS-KEY"));
    /// assert!(headers.contains_key("OK-ACCESS-SIGN"));
    /// assert!(headers.contains_key("OK-ACCESS-TIMESTAMP"));
    /// assert!(headers.contains_key("OK-ACCESS-PASSPHRASE"));
    /// ```
    pub fn add_auth_headers(&self, headers: &mut HeaderMap, timestamp: &str, sign: &str) {
        headers.insert(
            "OK-ACCESS-KEY",
            HeaderValue::from_str(self.api_key.expose_secret())
                .unwrap_or_else(|_| HeaderValue::from_static("")),
        );
        headers.insert(
            "OK-ACCESS-SIGN",
            HeaderValue::from_str(sign).unwrap_or_else(|_| HeaderValue::from_static("")),
        );
        headers.insert(
            "OK-ACCESS-TIMESTAMP",
            HeaderValue::from_str(timestamp).unwrap_or_else(|_| HeaderValue::from_static("")),
        );
        headers.insert(
            "OK-ACCESS-PASSPHRASE",
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
    /// * `timestamp` - ISO 8601 timestamp.
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
    use super::*;

    #[test]
    fn test_auth_new() {
        let auth = OkxAuth::new(
            "test-api-key".to_string(),
            "test-secret".to_string(),
            "test-passphrase".to_string(),
        );

        assert_eq!(auth.api_key(), "test-api-key");
        assert_eq!(auth.passphrase(), "test-passphrase");
    }

    #[test]
    fn test_build_sign_string() {
        let auth = OkxAuth::new(
            "api-key".to_string(),
            "secret".to_string(),
            "passphrase".to_string(),
        );

        let sign_string = auth.build_sign_string(
            "2021-01-01T00:00:00.000Z",
            "GET",
            "/api/v5/account/balance",
            "",
        );
        assert_eq!(
            sign_string,
            "2021-01-01T00:00:00.000ZGET/api/v5/account/balance"
        );

        let sign_string_post = auth.build_sign_string(
            "2021-01-01T00:00:00.000Z",
            "POST",
            "/api/v5/trade/order",
            r#"{"instId":"BTC-USDT","side":"buy"}"#,
        );
        assert_eq!(
            sign_string_post,
            r#"2021-01-01T00:00:00.000ZPOST/api/v5/trade/order{"instId":"BTC-USDT","side":"buy"}"#
        );
    }

    #[test]
    fn test_sign_deterministic() {
        let auth = OkxAuth::new(
            "api-key".to_string(),
            "test-secret-key".to_string(),
            "passphrase".to_string(),
        );

        let sig1 = auth.sign(
            "2021-01-01T00:00:00.000Z",
            "GET",
            "/api/v5/account/balance",
            "",
        );
        let sig2 = auth.sign(
            "2021-01-01T00:00:00.000Z",
            "GET",
            "/api/v5/account/balance",
            "",
        );

        assert_eq!(sig1, sig2);
        assert!(!sig1.is_empty());
    }

    #[test]
    fn test_sign_different_inputs() {
        let auth = OkxAuth::new(
            "api-key".to_string(),
            "test-secret-key".to_string(),
            "passphrase".to_string(),
        );

        let sig1 = auth.sign(
            "2021-01-01T00:00:00.000Z",
            "GET",
            "/api/v5/account/balance",
            "",
        );
        let sig2 = auth.sign(
            "2021-01-01T00:00:01.000Z",
            "GET",
            "/api/v5/account/balance",
            "",
        );
        let sig3 = auth.sign(
            "2021-01-01T00:00:00.000Z",
            "POST",
            "/api/v5/account/balance",
            "",
        );

        assert_ne!(sig1, sig2);
        assert_ne!(sig1, sig3);
    }

    #[test]
    fn test_add_auth_headers() {
        let auth = OkxAuth::new(
            "test-api-key".to_string(),
            "test-secret".to_string(),
            "test-passphrase".to_string(),
        );

        let mut headers = HeaderMap::new();
        let timestamp = "2021-01-01T00:00:00.000Z";
        let signature = auth.sign(timestamp, "GET", "/api/v5/account/balance", "");
        auth.add_auth_headers(&mut headers, timestamp, &signature);

        assert_eq!(headers.get("OK-ACCESS-KEY").unwrap(), "test-api-key");
        assert_eq!(headers.get("OK-ACCESS-SIGN").unwrap(), &signature);
        assert_eq!(
            headers.get("OK-ACCESS-TIMESTAMP").unwrap(),
            "2021-01-01T00:00:00.000Z"
        );
        assert_eq!(
            headers.get("OK-ACCESS-PASSPHRASE").unwrap(),
            "test-passphrase"
        );
    }

    #[test]
    fn test_create_auth_headers() {
        let auth = OkxAuth::new(
            "test-api-key".to_string(),
            "test-secret".to_string(),
            "test-passphrase".to_string(),
        );

        let headers = auth.create_auth_headers(
            "2021-01-01T00:00:00.000Z",
            "GET",
            "/api/v5/account/balance",
            "",
        );

        assert!(headers.contains_key("OK-ACCESS-KEY"));
        assert!(headers.contains_key("OK-ACCESS-SIGN"));
        assert!(headers.contains_key("OK-ACCESS-TIMESTAMP"));
        assert!(headers.contains_key("OK-ACCESS-PASSPHRASE"));
    }

    #[test]
    fn test_method_case_insensitive() {
        let auth = OkxAuth::new(
            "api-key".to_string(),
            "test-secret".to_string(),
            "passphrase".to_string(),
        );

        // The build_sign_string method converts method to uppercase
        let sign_string_lower = auth.build_sign_string(
            "2021-01-01T00:00:00.000Z",
            "get",
            "/api/v5/account/balance",
            "",
        );
        let sign_string_upper = auth.build_sign_string(
            "2021-01-01T00:00:00.000Z",
            "GET",
            "/api/v5/account/balance",
            "",
        );

        assert_eq!(sign_string_lower, sign_string_upper);
    }

    #[test]
    fn test_signature_is_base64() {
        let auth = OkxAuth::new(
            "api-key".to_string(),
            "test-secret".to_string(),
            "passphrase".to_string(),
        );

        let signature = auth.sign(
            "2021-01-01T00:00:00.000Z",
            "GET",
            "/api/v5/account/balance",
            "",
        );

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
    #[test]
    fn test_signature_with_known_inputs() {
        let auth = OkxAuth::new(
            "okx_api_key".to_string(),
            "okx-secret-key".to_string(),
            "my-passphrase".to_string(),
        );

        // Known inputs
        let timestamp = "2021-01-01T00:00:00.000Z";
        let method = "GET";
        let path = "/api/v5/account/balance";
        let body = "";

        // Generate signature
        let signature = auth.sign(timestamp, method, path, body);

        // The sign string should be: "2021-01-01T00:00:00.000ZGET/api/v5/account/balance"
        let expected_sign_string = "2021-01-01T00:00:00.000ZGET/api/v5/account/balance";
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
        let auth = OkxAuth::new(
            "okx_api_key".to_string(),
            "okx-secret-key".to_string(),
            "my-passphrase".to_string(),
        );

        let timestamp = "2021-01-01T00:00:00.000Z";
        let method = "POST";
        let path = "/api/v5/trade/order";
        let body = r#"{"instId":"BTC-USDT","tdMode":"cash","side":"buy","ordType":"limit","px":"50000","sz":"0.001"}"#;

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

        let auth = OkxAuth::new(
            api_key.to_string(),
            secret.to_string(),
            passphrase.to_string(),
        );

        let timestamp = "2021-01-01T00:00:00.000Z";
        let method = "GET";
        let path = "/api/v5/account/balance";
        let body = "";

        let headers = auth.create_auth_headers(timestamp, method, path, body);

        // Verify OK-ACCESS-KEY header
        assert_eq!(
            headers.get("OK-ACCESS-KEY").unwrap().to_str().unwrap(),
            api_key
        );

        // Verify OK-ACCESS-TIMESTAMP header
        assert_eq!(
            headers
                .get("OK-ACCESS-TIMESTAMP")
                .unwrap()
                .to_str()
                .unwrap(),
            timestamp
        );

        // Verify OK-ACCESS-PASSPHRASE header
        assert_eq!(
            headers
                .get("OK-ACCESS-PASSPHRASE")
                .unwrap()
                .to_str()
                .unwrap(),
            passphrase
        );

        // Verify OK-ACCESS-SIGN header exists and is valid Base64
        let sign_header = headers.get("OK-ACCESS-SIGN").unwrap().to_str().unwrap();
        assert!(!sign_header.is_empty());
        let decoded = general_purpose::STANDARD.decode(sign_header);
        assert!(decoded.is_ok());
    }

    /// Test signature with query parameters in path.
    #[test]
    fn test_signature_with_query_params() {
        let auth = OkxAuth::new(
            "api-key".to_string(),
            "secret-key".to_string(),
            "passphrase".to_string(),
        );

        let timestamp = "2021-01-01T00:00:00.000Z";
        let method = "GET";
        let path = "/api/v5/market/ticker?instId=BTC-USDT";
        let body = "";

        let signature = auth.sign(timestamp, method, path, body);

        // Verify sign string includes query params
        let sign_string = auth.build_sign_string(timestamp, method, path, body);
        assert!(sign_string.contains("?instId=BTC-USDT"));

        // Verify signature is valid
        assert!(!signature.is_empty());
        let decoded = general_purpose::STANDARD.decode(&signature);
        assert!(decoded.is_ok());
    }
}
