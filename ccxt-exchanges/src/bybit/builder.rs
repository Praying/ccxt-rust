//! Bybit exchange builder pattern implementation.
//!
//! Provides a fluent API for constructing Bybit exchange instances with
//! type-safe configuration options.

use super::{Bybit, BybitOptions};
use ccxt_core::config::{ProxyConfig, RetryPolicy};
use ccxt_core::types::default_type::{DefaultSubType, DefaultType};
use ccxt_core::{ExchangeConfig, Result};
use serde_json::Value;
use std::collections::HashMap;
use std::time::Duration;

/// Builder for creating Bybit exchange instances.
///
/// Provides a fluent API for configuring all aspects of the Bybit exchange,
/// including authentication, connection settings, and Bybit-specific options.
///
/// # Example
///
/// ```no_run
/// use ccxt_exchanges::bybit::BybitBuilder;
///
/// let bybit = BybitBuilder::new()
///     .api_key("your-api-key")
///     .secret("your-secret")
///     .testnet(true)
///     .timeout(30)
///     .build()
///     .unwrap();
/// ```
#[derive(Debug, Clone)]
pub struct BybitBuilder {
    /// Exchange configuration
    config: ExchangeConfig,
    /// Bybit-specific options
    options: BybitOptions,
}

impl Default for BybitBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl BybitBuilder {
    /// Creates a new builder with default configuration.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ccxt_exchanges::bybit::BybitBuilder;
    ///
    /// let builder = BybitBuilder::new();
    /// ```
    pub fn new() -> Self {
        Self {
            config: ExchangeConfig {
                id: "bybit".to_string(),
                name: "Bybit".to_string(),
                ..Default::default()
            },
            options: BybitOptions::default(),
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

    /// Enables or disables sandbox/testnet mode.
    ///
    /// When enabled, the exchange will connect to Bybit's testnet
    /// environment instead of the production environment.
    ///
    /// This method is equivalent to `testnet()` and is provided for
    /// consistency with other exchanges.
    ///
    /// # Arguments
    ///
    /// * `enabled` - Whether to enable sandbox mode.
    pub fn sandbox(mut self, enabled: bool) -> Self {
        self.config.sandbox = enabled;
        self.options.testnet = enabled;
        self
    }

    /// Enables or disables testnet mode.
    ///
    /// When enabled, the exchange will connect to Bybit's testnet
    /// environment instead of the production environment.
    ///
    /// This method is equivalent to `sandbox()` and is provided for
    /// backward compatibility.
    ///
    /// # Arguments
    ///
    /// * `enabled` - Whether to enable testnet mode.
    pub fn testnet(mut self, enabled: bool) -> Self {
        self.config.sandbox = enabled;
        self.options.testnet = enabled;
        self
    }

    /// Sets the account type for trading.
    ///
    /// Valid values: "UNIFIED", "CONTRACT", "SPOT".
    ///
    /// # Arguments
    ///
    /// * `account_type` - The account type string.
    pub fn account_type(mut self, account_type: impl Into<String>) -> Self {
        self.options.account_type = account_type.into();
        self
    }

    /// Sets the default market type for trading.
    ///
    /// This determines which category to use for API calls.
    /// Bybit uses a unified V5 API with category-based filtering:
    /// - `Spot` -> category=spot
    /// - `Swap` + Linear -> category=linear
    /// - `Swap` + Inverse -> category=inverse
    /// - `Option` -> category=option
    ///
    /// # Arguments
    ///
    /// * `default_type` - The default market type (spot, swap, futures, option).
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ccxt_exchanges::bybit::BybitBuilder;
    /// use ccxt_core::types::default_type::DefaultType;
    ///
    /// let bybit = BybitBuilder::new()
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
    /// - `Linear`: USDT-margined contracts (category=linear)
    /// - `Inverse`: Coin-margined contracts (category=inverse)
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
    /// use ccxt_exchanges::bybit::BybitBuilder;
    /// use ccxt_core::types::default_type::{DefaultType, DefaultSubType};
    ///
    /// let bybit = BybitBuilder::new()
    ///     .default_type(DefaultType::Swap)
    ///     .default_sub_type(DefaultSubType::Linear)
    ///     .build()
    ///     .unwrap();
    /// ```
    pub fn default_sub_type(mut self, sub_type: DefaultSubType) -> Self {
        self.options.default_sub_type = Some(sub_type);
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
    pub fn get_options(&self) -> &BybitOptions {
        &self.options
    }

    /// Builds the Bybit exchange instance.
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing the configured `Bybit` instance.
    ///
    /// # Errors
    ///
    /// Returns an error if the exchange cannot be initialized.
    pub fn build(self) -> Result<Bybit> {
        Bybit::new_with_options(self.config, self.options)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_default() {
        let builder = BybitBuilder::new();
        assert_eq!(builder.config.id, "bybit");
        assert_eq!(builder.config.name, "Bybit");
        assert!(!builder.config.sandbox);
        assert_eq!(builder.options.account_type, "UNIFIED");
        assert_eq!(builder.options.recv_window, 5000);
    }

    #[test]
    fn test_builder_api_key() {
        let builder = BybitBuilder::new().api_key("test-key");
        assert_eq!(
            builder.config.api_key.as_ref().map(|s| s.expose_secret()),
            Some("test-key")
        );
    }

    #[test]
    fn test_builder_secret() {
        let builder = BybitBuilder::new().secret("test-secret");
        assert_eq!(
            builder.config.secret.as_ref().map(|s| s.expose_secret()),
            Some("test-secret")
        );
    }

    #[test]
    fn test_builder_sandbox() {
        let builder = BybitBuilder::new().sandbox(true);
        assert!(builder.config.sandbox);
        assert!(builder.options.testnet);
    }

    #[test]
    fn test_builder_testnet() {
        let builder = BybitBuilder::new().testnet(true);
        assert!(builder.config.sandbox);
        assert!(builder.options.testnet);
    }

    #[test]
    fn test_builder_sandbox_testnet_equivalence() {
        // Verify that sandbox() and testnet() produce equivalent results
        let sandbox_builder = BybitBuilder::new().sandbox(true);
        let testnet_builder = BybitBuilder::new().testnet(true);

        assert_eq!(
            sandbox_builder.config.sandbox,
            testnet_builder.config.sandbox
        );
        assert_eq!(
            sandbox_builder.options.testnet,
            testnet_builder.options.testnet
        );
    }

    #[test]
    fn test_builder_account_type() {
        let builder = BybitBuilder::new().account_type("CONTRACT");
        assert_eq!(builder.options.account_type, "CONTRACT");
    }

    #[test]
    fn test_builder_default_type() {
        let builder = BybitBuilder::new().default_type(DefaultType::Swap);
        assert_eq!(builder.options.default_type, DefaultType::Swap);
    }

    #[test]
    fn test_builder_default_type_from_string() {
        let builder = BybitBuilder::new().default_type("futures");
        assert_eq!(builder.options.default_type, DefaultType::Futures);
    }

    #[test]
    fn test_builder_default_sub_type() {
        let builder = BybitBuilder::new().default_sub_type(DefaultSubType::Inverse);
        assert_eq!(
            builder.options.default_sub_type,
            Some(DefaultSubType::Inverse)
        );
    }

    #[test]
    fn test_builder_default_type_and_sub_type() {
        let builder = BybitBuilder::new()
            .default_type(DefaultType::Swap)
            .default_sub_type(DefaultSubType::Linear);
        assert_eq!(builder.options.default_type, DefaultType::Swap);
        assert_eq!(
            builder.options.default_sub_type,
            Some(DefaultSubType::Linear)
        );
    }

    #[test]
    fn test_builder_recv_window() {
        let builder = BybitBuilder::new().recv_window(10000);
        assert_eq!(builder.options.recv_window, 10000);
    }

    #[test]
    fn test_builder_timeout() {
        let builder = BybitBuilder::new().timeout(Duration::from_secs(60));
        assert_eq!(builder.config.timeout, Duration::from_secs(60));
    }

    #[test]
    fn test_builder_chaining() {
        let builder = BybitBuilder::new()
            .api_key("key")
            .secret("secret")
            .testnet(true)
            .timeout(Duration::from_secs(30))
            .recv_window(10000)
            .account_type("SPOT")
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
        assert!(builder.config.sandbox);
        assert_eq!(builder.config.timeout, Duration::from_secs(30));
        assert_eq!(builder.options.recv_window, 10000);
        assert_eq!(builder.options.account_type, "SPOT");
        assert_eq!(builder.options.default_type, DefaultType::Swap);
        assert_eq!(
            builder.options.default_sub_type,
            Some(DefaultSubType::Linear)
        );
    }

    #[test]
    fn test_builder_build() {
        let result = BybitBuilder::new().build();
        assert!(result.is_ok());

        let bybit = result.unwrap();
        assert_eq!(bybit.id(), "bybit");
        assert_eq!(bybit.name(), "Bybit");
    }

    #[test]
    fn test_builder_build_with_credentials() {
        let result = BybitBuilder::new()
            .api_key("test-key")
            .secret("test-secret")
            .build();

        assert!(result.is_ok());
    }

    #[test]
    fn test_builder_enable_rate_limit() {
        let builder = BybitBuilder::new().enable_rate_limit(false);
        assert!(!builder.config.enable_rate_limit);
    }

    #[test]
    fn test_builder_proxy() {
        let builder = BybitBuilder::new().proxy(ProxyConfig::new("http://proxy.example.com:8080"));
        assert_eq!(
            builder.config.proxy,
            Some(ProxyConfig::new("http://proxy.example.com:8080"))
        );
    }

    #[test]
    fn test_builder_verbose() {
        let builder = BybitBuilder::new().verbose(true);
        assert!(builder.config.verbose);
    }
}
