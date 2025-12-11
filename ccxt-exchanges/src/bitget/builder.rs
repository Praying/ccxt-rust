//! Bitget exchange builder pattern implementation.
//!
//! Provides a fluent API for constructing Bitget exchange instances with
//! type-safe configuration options.

use super::{Bitget, BitgetOptions};
use ccxt_core::{ExchangeConfig, Result};
use serde_json::Value;
use std::collections::HashMap;

/// Builder for creating Bitget exchange instances.
///
/// Provides a fluent API for configuring all aspects of the Bitget exchange,
/// including authentication, connection settings, and Bitget-specific options.
///
/// # Example
///
/// ```no_run
/// use ccxt_exchanges::bitget::BitgetBuilder;
///
/// let bitget = BitgetBuilder::new()
///     .api_key("your-api-key")
///     .secret("your-secret")
///     .passphrase("your-passphrase")
///     .sandbox(true)
///     .timeout(30)
///     .build()
///     .unwrap();
/// ```
#[derive(Debug, Clone)]
pub struct BitgetBuilder {
    /// Exchange configuration
    config: ExchangeConfig,
    /// Bitget-specific options
    options: BitgetOptions,
}

impl Default for BitgetBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl BitgetBuilder {
    /// Creates a new builder with default configuration.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ccxt_exchanges::bitget::BitgetBuilder;
    ///
    /// let builder = BitgetBuilder::new();
    /// ```
    pub fn new() -> Self {
        Self {
            config: ExchangeConfig {
                id: "bitget".to_string(),
                name: "Bitget".to_string(),
                ..Default::default()
            },
            options: BitgetOptions::default(),
        }
    }

    /// Sets the API key for authentication.
    ///
    /// # Arguments
    ///
    /// * `key` - The API key string.
    pub fn api_key(mut self, key: impl Into<String>) -> Self {
        self.config.api_key = Some(key.into());
        self
    }

    /// Sets the API secret for authentication.
    ///
    /// # Arguments
    ///
    /// * `secret` - The API secret string.
    pub fn secret(mut self, secret: impl Into<String>) -> Self {
        self.config.secret = Some(secret.into());
        self
    }

    /// Sets the passphrase for authentication.
    ///
    /// Bitget requires a passphrase in addition to API key and secret.
    ///
    /// # Arguments
    ///
    /// * `passphrase` - The passphrase string.
    pub fn passphrase(mut self, passphrase: impl Into<String>) -> Self {
        self.config.password = Some(passphrase.into());
        self
    }

    /// Enables or disables sandbox/demo mode.
    ///
    /// When enabled, the exchange will connect to Bitget's demo
    /// environment instead of the production environment.
    ///
    /// # Arguments
    ///
    /// * `enabled` - Whether to enable sandbox mode.
    pub fn sandbox(mut self, enabled: bool) -> Self {
        self.config.sandbox = enabled;
        self.options.demo = enabled;
        self
    }

    /// Sets the product type for trading.
    ///
    /// Valid values: "spot", "umcbl" (USDT-M futures), "dmcbl" (Coin-M futures).
    ///
    /// # Arguments
    ///
    /// * `product_type` - The product type string.
    pub fn product_type(mut self, product_type: impl Into<String>) -> Self {
        self.options.product_type = product_type.into();
        self
    }

    /// Sets the request timeout in seconds.
    ///
    /// # Arguments
    ///
    /// * `seconds` - Timeout duration in seconds.
    pub fn timeout(mut self, seconds: u64) -> Self {
        self.config.timeout = seconds;
        self
    }

    /// Sets the receive window for signed requests.
    ///
    /// The receive window specifies how long a request is valid after
    /// the timestamp. Default is 5000 milliseconds.
    ///
    /// # Arguments
    ///
    /// * `millis` - Receive window in milliseconds.
    pub fn recv_window(mut self, millis: u64) -> Self {
        self.options.recv_window = millis;
        self
    }

    /// Enables or disables rate limiting.
    ///
    /// # Arguments
    ///
    /// * `enabled` - Whether to enable rate limiting.
    pub fn enable_rate_limit(mut self, enabled: bool) -> Self {
        self.config.enable_rate_limit = enabled;
        self
    }

    /// Sets the HTTP proxy server URL.
    ///
    /// # Arguments
    ///
    /// * `proxy` - The proxy server URL.
    pub fn proxy(mut self, proxy: impl Into<String>) -> Self {
        self.config.proxy = Some(proxy.into());
        self
    }

    /// Enables or disables verbose logging.
    ///
    /// # Arguments
    ///
    /// * `enabled` - Whether to enable verbose logging.
    pub fn verbose(mut self, enabled: bool) -> Self {
        self.config.verbose = enabled;
        self
    }

    /// Sets a custom option.
    ///
    /// # Arguments
    ///
    /// * `key` - Option key.
    /// * `value` - Option value as JSON.
    pub fn option(mut self, key: impl Into<String>, value: Value) -> Self {
        self.config.options.insert(key.into(), value);
        self
    }

    /// Sets multiple custom options.
    ///
    /// # Arguments
    ///
    /// * `options` - HashMap of option key-value pairs.
    pub fn options(mut self, options: HashMap<String, Value>) -> Self {
        self.config.options.extend(options);
        self
    }

    /// Returns the current configuration (for testing purposes).
    #[cfg(test)]
    pub fn get_config(&self) -> &ExchangeConfig {
        &self.config
    }

    /// Returns the current options (for testing purposes).
    #[cfg(test)]
    pub fn get_options(&self) -> &BitgetOptions {
        &self.options
    }

    /// Builds the Bitget exchange instance.
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing the configured `Bitget` instance.
    ///
    /// # Errors
    ///
    /// Returns an error if the exchange cannot be initialized.
    pub fn build(self) -> Result<Bitget> {
        Bitget::new_with_options(self.config, self.options)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_default() {
        let builder = BitgetBuilder::new();
        assert_eq!(builder.config.id, "bitget");
        assert_eq!(builder.config.name, "Bitget");
        assert!(!builder.config.sandbox);
        assert_eq!(builder.options.product_type, "spot");
    }

    #[test]
    fn test_builder_api_key() {
        let builder = BitgetBuilder::new().api_key("test-key");
        assert_eq!(builder.config.api_key, Some("test-key".to_string()));
    }

    #[test]
    fn test_builder_secret() {
        let builder = BitgetBuilder::new().secret("test-secret");
        assert_eq!(builder.config.secret, Some("test-secret".to_string()));
    }

    #[test]
    fn test_builder_passphrase() {
        let builder = BitgetBuilder::new().passphrase("test-passphrase");
        assert_eq!(builder.config.password, Some("test-passphrase".to_string()));
    }

    #[test]
    fn test_builder_sandbox() {
        let builder = BitgetBuilder::new().sandbox(true);
        assert!(builder.config.sandbox);
        assert!(builder.options.demo);
    }

    #[test]
    fn test_builder_product_type() {
        let builder = BitgetBuilder::new().product_type("umcbl");
        assert_eq!(builder.options.product_type, "umcbl");
    }

    #[test]
    fn test_builder_timeout() {
        let builder = BitgetBuilder::new().timeout(60);
        assert_eq!(builder.config.timeout, 60);
    }

    #[test]
    fn test_builder_recv_window() {
        let builder = BitgetBuilder::new().recv_window(10000);
        assert_eq!(builder.options.recv_window, 10000);
    }

    #[test]
    fn test_builder_chaining() {
        let builder = BitgetBuilder::new()
            .api_key("key")
            .secret("secret")
            .passphrase("pass")
            .sandbox(true)
            .timeout(30)
            .recv_window(5000)
            .product_type("spot");

        assert_eq!(builder.config.api_key, Some("key".to_string()));
        assert_eq!(builder.config.secret, Some("secret".to_string()));
        assert_eq!(builder.config.password, Some("pass".to_string()));
        assert!(builder.config.sandbox);
        assert_eq!(builder.config.timeout, 30);
        assert_eq!(builder.options.recv_window, 5000);
        assert_eq!(builder.options.product_type, "spot");
    }

    #[test]
    fn test_builder_build() {
        let result = BitgetBuilder::new().build();
        assert!(result.is_ok());

        let bitget = result.unwrap();
        assert_eq!(bitget.id(), "bitget");
        assert_eq!(bitget.name(), "Bitget");
    }

    #[test]
    fn test_builder_build_with_credentials() {
        let result = BitgetBuilder::new()
            .api_key("test-key")
            .secret("test-secret")
            .passphrase("test-passphrase")
            .build();

        assert!(result.is_ok());
    }
}
