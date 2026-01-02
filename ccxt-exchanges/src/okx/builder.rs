//! OKX exchange builder pattern implementation.
//!
//! Provides a fluent API for constructing OKX exchange instances with
//! type-safe configuration options.

use super::{Okx, OkxOptions};
use ccxt_core::config::{ProxyConfig, RetryPolicy};
use ccxt_core::types::default_type::{DefaultSubType, DefaultType};
use ccxt_core::{ExchangeConfig, Result};
use serde_json::Value;
use std::collections::HashMap;
use std::time::Duration;

/// Builder for creating OKX exchange instances.
///
/// Provides a fluent API for configuring all aspects of the OKX exchange,
/// including authentication, connection settings, and OKX-specific options.
///
/// # Example
///
/// ```no_run
/// use ccxt_exchanges::okx::OkxBuilder;
///
/// let okx = OkxBuilder::new()
///     .api_key("your-api-key")
///     .secret("your-secret")
///     .passphrase("your-passphrase")
///     .sandbox(true)
///     .timeout(30)
///     .build()
///     .unwrap();
/// ```
#[derive(Debug, Clone)]
pub struct OkxBuilder {
    /// Exchange configuration
    config: ExchangeConfig,
    /// OKX-specific options
    options: OkxOptions,
}

impl Default for OkxBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl OkxBuilder {
    /// Creates a new builder with default configuration.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ccxt_exchanges::okx::OkxBuilder;
    ///
    /// let builder = OkxBuilder::new();
    /// ```
    pub fn new() -> Self {
        Self {
            config: ExchangeConfig {
                id: "okx".to_string(),
                name: "OKX".to_string(),
                ..Default::default()
            },
            options: OkxOptions::default(),
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
    /// OKX requires a passphrase in addition to API key and secret.
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
    /// When enabled, the exchange will connect to OKX's demo
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

    /// Sets the account mode for trading.
    ///
    /// Valid values: "cash" (spot), "cross" (cross margin), "isolated" (isolated margin).
    ///
    /// # Arguments
    ///
    /// * `mode` - The account mode string.
    pub fn account_mode(mut self, mode: impl Into<String>) -> Self {
        self.options.account_mode = mode.into();
        self
    }

    /// Sets the default market type for trading.
    ///
    /// This determines which instrument type (instType) to use for API calls.
    /// OKX uses a unified V5 API, so this primarily affects market filtering.
    ///
    /// # Arguments
    ///
    /// * `default_type` - The default market type (spot, swap, futures, margin, option).
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ccxt_exchanges::okx::OkxBuilder;
    /// use ccxt_core::types::default_type::DefaultType;
    ///
    /// let okx = OkxBuilder::new()
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
    /// - `Linear`: USDT-margined contracts
    /// - `Inverse`: Coin-margined contracts
    ///
    /// Only applicable when `default_type` is `Swap`, `Futures`, or `Option`.
    /// Ignored for `Spot` and `Margin` types.
    ///
    /// # Arguments
    ///
    /// * `sub_type` - The contract settlement type (linear or inverse).
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ccxt_exchanges::okx::OkxBuilder;
    /// use ccxt_core::types::default_type::{DefaultType, DefaultSubType};
    ///
    /// let okx = OkxBuilder::new()
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

    /// Sets the TCP connection timeout.
    ///
    /// # Arguments
    ///
    /// * `timeout` - Connection timeout duration.
    pub fn connect_timeout(mut self, timeout: Duration) -> Self {
        self.config.connect_timeout = timeout;
        self
    }

    /// Sets the TCP connection timeout in seconds (convenience method).
    ///
    /// # Arguments
    ///
    /// * `seconds` - Connection timeout duration in seconds.
    pub fn connect_timeout_secs(mut self, seconds: u64) -> Self {
        self.config.connect_timeout = Duration::from_secs(seconds);
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
    pub fn get_options(&self) -> &OkxOptions {
        &self.options
    }

    /// Builds the OKX exchange instance.
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing the configured `Okx` instance.
    ///
    /// # Errors
    ///
    /// Returns an error if the exchange cannot be initialized.
    pub fn build(self) -> Result<Okx> {
        Okx::new_with_options(self.config, self.options)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_default() {
        let builder = OkxBuilder::new();
        assert_eq!(builder.config.id, "okx");
        assert_eq!(builder.config.name, "OKX");
        assert!(!builder.config.sandbox);
        assert_eq!(builder.options.account_mode, "cash");
    }

    #[test]
    fn test_builder_api_key() {
        let builder = OkxBuilder::new().api_key("test-key");
        assert_eq!(
            builder.config.api_key.as_ref().map(|s| s.expose_secret()),
            Some("test-key")
        );
    }

    #[test]
    fn test_builder_secret() {
        let builder = OkxBuilder::new().secret("test-secret");
        assert_eq!(
            builder.config.secret.as_ref().map(|s| s.expose_secret()),
            Some("test-secret")
        );
    }

    #[test]
    fn test_builder_passphrase() {
        let builder = OkxBuilder::new().passphrase("test-passphrase");
        assert_eq!(
            builder.config.password.as_ref().map(|s| s.expose_secret()),
            Some("test-passphrase")
        );
    }

    #[test]
    fn test_builder_sandbox() {
        let builder = OkxBuilder::new().sandbox(true);
        assert!(builder.config.sandbox);
        assert!(builder.options.testnet);
    }

    #[test]
    fn test_builder_account_mode() {
        let builder = OkxBuilder::new().account_mode("cross");
        assert_eq!(builder.options.account_mode, "cross");
    }

    #[test]
    fn test_builder_default_type() {
        let builder = OkxBuilder::new().default_type(DefaultType::Swap);
        assert_eq!(builder.options.default_type, DefaultType::Swap);
    }

    #[test]
    fn test_builder_default_type_from_string() {
        let builder = OkxBuilder::new().default_type("futures");
        assert_eq!(builder.options.default_type, DefaultType::Futures);
    }

    #[test]
    fn test_builder_default_sub_type() {
        let builder = OkxBuilder::new().default_sub_type(DefaultSubType::Inverse);
        assert_eq!(
            builder.options.default_sub_type,
            Some(DefaultSubType::Inverse)
        );
    }

    #[test]
    fn test_builder_default_type_and_sub_type() {
        let builder = OkxBuilder::new()
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
        let builder = OkxBuilder::new().timeout(Duration::from_secs(60));
        assert_eq!(builder.config.timeout, Duration::from_secs(60));
    }

    #[test]
    fn test_builder_connect_timeout() {
        let builder = OkxBuilder::new().connect_timeout(Duration::from_secs(15));
        assert_eq!(builder.config.connect_timeout, Duration::from_secs(15));
    }

    #[test]
    fn test_builder_connect_timeout_secs() {
        let builder = OkxBuilder::new().connect_timeout_secs(20);
        assert_eq!(builder.config.connect_timeout, Duration::from_secs(20));
    }

    #[test]
    fn test_builder_chaining() {
        let builder = OkxBuilder::new()
            .api_key("key")
            .secret("secret")
            .passphrase("pass")
            .sandbox(true)
            .timeout(Duration::from_secs(30))
            .account_mode("isolated");

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
        assert_eq!(builder.options.account_mode, "isolated");
    }

    #[test]
    fn test_builder_build() {
        let result = OkxBuilder::new().build();
        assert!(result.is_ok());

        let okx = result.unwrap();
        assert_eq!(okx.id(), "okx");
        assert_eq!(okx.name(), "OKX");
    }

    #[test]
    fn test_builder_build_with_credentials() {
        let result = OkxBuilder::new()
            .api_key("test-key")
            .secret("test-secret")
            .passphrase("test-passphrase")
            .build();

        assert!(result.is_ok());
    }

    #[test]
    fn test_builder_enable_rate_limit() {
        let builder = OkxBuilder::new().enable_rate_limit(false);
        assert!(!builder.config.enable_rate_limit);
    }

    #[test]
    fn test_builder_proxy() {
        let builder = OkxBuilder::new().proxy(ProxyConfig::new("http://proxy.example.com:8080"));
        assert_eq!(
            builder.config.proxy,
            Some(ProxyConfig::new("http://proxy.example.com:8080"))
        );
    }

    #[test]
    fn test_builder_verbose() {
        let builder = OkxBuilder::new().verbose(true);
        assert!(builder.config.verbose);
    }
}
