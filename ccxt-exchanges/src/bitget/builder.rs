//! Bitget exchange builder pattern implementation.
//!
//! Provides a fluent API for constructing Bitget exchange instances with
//! type-safe configuration options.

use super::{Bitget, BitgetOptions};
use ccxt_core::config::{ProxyConfig, RetryPolicy};
use ccxt_core::types::default_type::{DefaultSubType, DefaultType};
use ccxt_core::{ExchangeConfig, Result};
use serde_json::Value;
use std::collections::HashMap;
use std::time::Duration;

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
        self.config.api_key = Some(ccxt_core::SecretString::new(key));
        self
    }

    /// Sets the API secret for authentication.
    ///
    /// # Arguments
    ///
    /// * `secret` - The API secret string.
    pub fn secret(mut self, secret: impl Into<String>) -> Self {
        self.config.secret = Some(ccxt_core::SecretString::new(secret));
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
        self.config.password = Some(ccxt_core::SecretString::new(passphrase));
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
        self.options.testnet = enabled;
        self
    }

    /// Sets the product type for trading.
    ///
    /// Valid values: "spot", "umcbl" (USDT-M futures), "dmcbl" (Coin-M futures).
    ///
    /// Note: Consider using `default_type()` and `default_sub_type()` instead
    /// for a more type-safe configuration.
    ///
    /// # Arguments
    ///
    /// * `product_type` - The product type string.
    pub fn product_type(mut self, product_type: impl Into<String>) -> Self {
        self.options.product_type = product_type.into();
        self
    }

    /// Sets the default market type for trading.
    ///
    /// This determines which product type to use for API calls.
    /// Bitget uses product_type-based filtering:
    /// - `Spot` -> product_type=spot
    /// - `Swap` + Linear -> product_type=umcbl (USDT-M)
    /// - `Swap` + Inverse -> product_type=dmcbl (Coin-M)
    /// - `Futures` + Linear -> product_type=umcbl (USDT-M futures)
    /// - `Futures` + Inverse -> product_type=dmcbl (Coin-M futures)
    ///
    /// # Arguments
    ///
    /// * `default_type` - The default market type (spot, swap, futures, margin, option).
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ccxt_exchanges::bitget::BitgetBuilder;
    /// use ccxt_core::types::default_type::DefaultType;
    ///
    /// let bitget = BitgetBuilder::new()
    ///     .default_type(DefaultType::Swap)
    ///     .build()
    ///     .unwrap();
    /// ```
    pub fn default_type(mut self, default_type: impl Into<DefaultType>) -> Self {
        self.options.default_type = default_type.into();
        self
    }

    /// Sets the default sub-type for contract settlement.
    ///
    /// - `Linear`: USDT-margined contracts (product_type=umcbl)
    /// - `Inverse`: Coin-margined contracts (product_type=dmcbl)
    ///
    /// Only applicable when `default_type` is `Swap` or `Futures`.
    /// Ignored for `Spot` and `Option` types.
    ///
    /// # Arguments
    ///
    /// * `sub_type` - The contract settlement type (linear or inverse).
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ccxt_exchanges::bitget::BitgetBuilder;
    /// use ccxt_core::types::default_type::{DefaultType, DefaultSubType};
    ///
    /// let bitget = BitgetBuilder::new()
    ///     .default_type(DefaultType::Swap)
    ///     .default_sub_type(DefaultSubType::Linear)
    ///     .build()
    ///     .unwrap();
    /// ```
    pub fn default_sub_type(mut self, sub_type: DefaultSubType) -> Self {
        self.options.default_sub_type = Some(sub_type);
        self
    }

    /// Sets the request timeout.
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.config.timeout = timeout;
        self
    }

    /// Sets the request timeout in seconds (convenience method).
    pub fn timeout_secs(mut self, seconds: u64) -> Self {
        self.config.timeout = Duration::from_secs(seconds);
        self
    }

    /// Sets the retry policy.
    pub fn retry_policy(mut self, policy: RetryPolicy) -> Self {
        self.config.retry_policy = Some(policy);
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

    /// Sets the HTTP proxy configuration.
    pub fn proxy(mut self, proxy: ProxyConfig) -> Self {
        self.config.proxy = Some(proxy);
        self
    }

    /// Sets the HTTP proxy URL (convenience method).
    pub fn proxy_url(mut self, url: impl Into<String>) -> Self {
        self.config.proxy = Some(ProxyConfig::new(url));
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
        assert_eq!(
            builder.config.api_key.as_ref().map(|s| s.expose_secret()),
            Some("test-key")
        );
    }

    #[test]
    fn test_builder_secret() {
        let builder = BitgetBuilder::new().secret("test-secret");
        assert_eq!(
            builder.config.secret.as_ref().map(|s| s.expose_secret()),
            Some("test-secret")
        );
    }

    #[test]
    fn test_builder_passphrase() {
        let builder = BitgetBuilder::new().passphrase("test-passphrase");
        assert_eq!(
            builder.config.password.as_ref().map(|s| s.expose_secret()),
            Some("test-passphrase")
        );
    }

    #[test]
    fn test_builder_sandbox() {
        let builder = BitgetBuilder::new().sandbox(true);
        assert!(builder.config.sandbox);
        assert!(builder.options.testnet);
    }

    #[test]
    fn test_builder_product_type() {
        let builder = BitgetBuilder::new().product_type("umcbl");
        assert_eq!(builder.options.product_type, "umcbl");
    }

    #[test]
    fn test_builder_default_type() {
        let builder = BitgetBuilder::new().default_type(DefaultType::Swap);
        assert_eq!(builder.options.default_type, DefaultType::Swap);
    }

    #[test]
    fn test_builder_default_type_from_string() {
        let builder = BitgetBuilder::new().default_type("futures");
        assert_eq!(builder.options.default_type, DefaultType::Futures);
    }

    #[test]
    fn test_builder_default_sub_type() {
        let builder = BitgetBuilder::new().default_sub_type(DefaultSubType::Inverse);
        assert_eq!(
            builder.options.default_sub_type,
            Some(DefaultSubType::Inverse)
        );
    }

    #[test]
    fn test_builder_default_type_and_sub_type() {
        let builder = BitgetBuilder::new()
            .default_type(DefaultType::Swap)
            .default_sub_type(DefaultSubType::Linear);
        assert_eq!(builder.options.default_type, DefaultType::Swap);
        assert_eq!(
            builder.options.default_sub_type,
            Some(DefaultSubType::Linear)
        );
    }

    #[test]
    fn test_builder_timeout() {
        let builder = BitgetBuilder::new().timeout(Duration::from_secs(60));
        assert_eq!(builder.config.timeout, Duration::from_secs(60));
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
            .timeout(Duration::from_secs(30))
            .recv_window(5000)
            .product_type("spot")
            .default_type(DefaultType::Swap)
            .default_sub_type(DefaultSubType::Linear);

        assert_eq!(
            builder.config.api_key.as_ref().map(|s| s.expose_secret()),
            Some("key")
        );
        assert_eq!(
            builder.config.secret.as_ref().map(|s| s.expose_secret()),
            Some("secret")
        );
        assert_eq!(
            builder.config.password.as_ref().map(|s| s.expose_secret()),
            Some("pass")
        );
        assert!(builder.config.sandbox);
        assert_eq!(builder.config.timeout, Duration::from_secs(30));
        assert_eq!(builder.options.recv_window, 5000);
        assert_eq!(builder.options.product_type, "spot");
        assert_eq!(builder.options.default_type, DefaultType::Swap);
        assert_eq!(
            builder.options.default_sub_type,
            Some(DefaultSubType::Linear)
        );
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
