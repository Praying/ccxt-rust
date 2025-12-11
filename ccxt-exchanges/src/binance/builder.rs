//! Binance exchange builder pattern implementation.
//!
//! Provides a fluent API for constructing Binance exchange instances with
//! type-safe configuration options.

use super::{Binance, BinanceOptions};
use ccxt_core::{ExchangeConfig, Result};
use serde_json::Value;
use std::collections::HashMap;

/// Builder for creating Binance exchange instances.
///
/// Provides a fluent API for configuring all aspects of the Binance exchange,
/// including authentication, connection settings, and Binance-specific options.
///
/// # Example
///
/// ```no_run
/// use ccxt_exchanges::binance::BinanceBuilder;
///
/// let binance = BinanceBuilder::new()
///     .api_key("your-api-key")
///     .secret("your-secret")
///     .sandbox(true)
///     .timeout(30)
///     .build()
///     .unwrap();
/// ```
#[derive(Debug, Clone)]
pub struct BinanceBuilder {
    /// Exchange configuration
    config: ExchangeConfig,
    /// Binance-specific options
    options: BinanceOptions,
}

impl Default for BinanceBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl BinanceBuilder {
    /// Creates a new builder with default configuration.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ccxt_exchanges::binance::BinanceBuilder;
    ///
    /// let builder = BinanceBuilder::new();
    /// ```
    pub fn new() -> Self {
        Self {
            config: ExchangeConfig {
                id: "binance".to_string(),
                name: "Binance".to_string(),
                ..Default::default()
            },
            options: BinanceOptions::default(),
        }
    }

    /// Sets the API key for authentication.
    ///
    /// # Arguments
    ///
    /// * `key` - The API key string.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ccxt_exchanges::binance::BinanceBuilder;
    ///
    /// let builder = BinanceBuilder::new()
    ///     .api_key("your-api-key");
    /// ```
    pub fn api_key(mut self, key: impl Into<String>) -> Self {
        self.config.api_key = Some(key.into());
        self
    }

    /// Sets the API secret for authentication.
    ///
    /// # Arguments
    ///
    /// * `secret` - The API secret string.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ccxt_exchanges::binance::BinanceBuilder;
    ///
    /// let builder = BinanceBuilder::new()
    ///     .secret("your-secret");
    /// ```
    pub fn secret(mut self, secret: impl Into<String>) -> Self {
        self.config.secret = Some(secret.into());
        self
    }

    /// Enables or disables sandbox/testnet mode.
    ///
    /// When enabled, the exchange will connect to Binance's testnet
    /// instead of the production environment.
    ///
    /// # Arguments
    ///
    /// * `enabled` - Whether to enable sandbox mode.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ccxt_exchanges::binance::BinanceBuilder;
    ///
    /// let builder = BinanceBuilder::new()
    ///     .sandbox(true);
    /// ```
    pub fn sandbox(mut self, enabled: bool) -> Self {
        self.config.sandbox = enabled;
        self.options.test = enabled;
        self
    }

    /// Sets the request timeout in seconds.
    ///
    /// # Arguments
    ///
    /// * `seconds` - Timeout duration in seconds.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ccxt_exchanges::binance::BinanceBuilder;
    ///
    /// let builder = BinanceBuilder::new()
    ///     .timeout(60);
    /// ```
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
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ccxt_exchanges::binance::BinanceBuilder;
    ///
    /// let builder = BinanceBuilder::new()
    ///     .recv_window(10000);
    /// ```
    pub fn recv_window(mut self, millis: u64) -> Self {
        self.options.recv_window = millis;
        self
    }

    /// Sets the default trading type.
    ///
    /// Valid values: "spot", "margin", "future", "delivery", "option".
    ///
    /// # Arguments
    ///
    /// * `trading_type` - The default trading type.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ccxt_exchanges::binance::BinanceBuilder;
    ///
    /// let builder = BinanceBuilder::new()
    ///     .default_type("future");
    /// ```
    pub fn default_type(mut self, trading_type: impl Into<String>) -> Self {
        self.options.default_type = trading_type.into();
        self
    }

    /// Enables or disables time synchronization adjustment.
    ///
    /// When enabled, the exchange will adjust for time differences
    /// between the local system and Binance servers.
    ///
    /// # Arguments
    ///
    /// * `enabled` - Whether to enable time adjustment.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ccxt_exchanges::binance::BinanceBuilder;
    ///
    /// let builder = BinanceBuilder::new()
    ///     .adjust_for_time_difference(true);
    /// ```
    pub fn adjust_for_time_difference(mut self, enabled: bool) -> Self {
        self.options.adjust_for_time_difference = enabled;
        self
    }

    /// Enables or disables demo environment.
    ///
    /// # Arguments
    ///
    /// * `enabled` - Whether to enable demo mode.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ccxt_exchanges::binance::BinanceBuilder;
    ///
    /// let builder = BinanceBuilder::new()
    ///     .demo(true);
    /// ```
    pub fn demo(mut self, enabled: bool) -> Self {
        self.options.demo = enabled;
        self
    }

    /// Enables or disables rate limiting.
    ///
    /// # Arguments
    ///
    /// * `enabled` - Whether to enable rate limiting.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ccxt_exchanges::binance::BinanceBuilder;
    ///
    /// let builder = BinanceBuilder::new()
    ///     .enable_rate_limit(true);
    /// ```
    pub fn enable_rate_limit(mut self, enabled: bool) -> Self {
        self.config.enable_rate_limit = enabled;
        self
    }

    /// Sets the HTTP proxy server URL.
    ///
    /// # Arguments
    ///
    /// * `proxy` - The proxy server URL.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ccxt_exchanges::binance::BinanceBuilder;
    ///
    /// let builder = BinanceBuilder::new()
    ///     .proxy("http://proxy.example.com:8080");
    /// ```
    pub fn proxy(mut self, proxy: impl Into<String>) -> Self {
        self.config.proxy = Some(proxy.into());
        self
    }

    /// Enables or disables verbose logging.
    ///
    /// # Arguments
    ///
    /// * `enabled` - Whether to enable verbose logging.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ccxt_exchanges::binance::BinanceBuilder;
    ///
    /// let builder = BinanceBuilder::new()
    ///     .verbose(true);
    /// ```
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
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ccxt_exchanges::binance::BinanceBuilder;
    /// use serde_json::json;
    ///
    /// let builder = BinanceBuilder::new()
    ///     .option("customOption", json!("value"));
    /// ```
    pub fn option(mut self, key: impl Into<String>, value: Value) -> Self {
        self.config.options.insert(key.into(), value);
        self
    }

    /// Sets multiple custom options.
    ///
    /// # Arguments
    ///
    /// * `options` - HashMap of option key-value pairs.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ccxt_exchanges::binance::BinanceBuilder;
    /// use serde_json::json;
    /// use std::collections::HashMap;
    ///
    /// let mut options = HashMap::new();
    /// options.insert("option1".to_string(), json!("value1"));
    /// options.insert("option2".to_string(), json!(42));
    ///
    /// let builder = BinanceBuilder::new()
    ///     .options(options);
    /// ```
    pub fn options(mut self, options: HashMap<String, Value>) -> Self {
        self.config.options.extend(options);
        self
    }

    /// Builds the Binance exchange instance.
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing the configured `Binance` instance.
    ///
    /// # Errors
    ///
    /// Returns an error if the exchange cannot be initialized.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ccxt_exchanges::binance::BinanceBuilder;
    ///
    /// let binance = BinanceBuilder::new()
    ///     .api_key("your-api-key")
    ///     .secret("your-secret")
    ///     .build()
    ///     .unwrap();
    /// ```
    pub fn build(self) -> Result<Binance> {
        Binance::new_with_options(self.config, self.options)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_default() {
        let builder = BinanceBuilder::new();
        assert_eq!(builder.config.id, "binance");
        assert_eq!(builder.config.name, "Binance");
        assert!(!builder.config.sandbox);
        assert_eq!(builder.options.default_type, "spot");
    }

    #[test]
    fn test_builder_api_key() {
        let builder = BinanceBuilder::new().api_key("test-key");
        assert_eq!(builder.config.api_key, Some("test-key".to_string()));
    }

    #[test]
    fn test_builder_secret() {
        let builder = BinanceBuilder::new().secret("test-secret");
        assert_eq!(builder.config.secret, Some("test-secret".to_string()));
    }

    #[test]
    fn test_builder_sandbox() {
        let builder = BinanceBuilder::new().sandbox(true);
        assert!(builder.config.sandbox);
        assert!(builder.options.test);
    }

    #[test]
    fn test_builder_timeout() {
        let builder = BinanceBuilder::new().timeout(60);
        assert_eq!(builder.config.timeout, 60);
    }

    #[test]
    fn test_builder_recv_window() {
        let builder = BinanceBuilder::new().recv_window(10000);
        assert_eq!(builder.options.recv_window, 10000);
    }

    #[test]
    fn test_builder_default_type() {
        let builder = BinanceBuilder::new().default_type("future");
        assert_eq!(builder.options.default_type, "future");
    }

    #[test]
    fn test_builder_chaining() {
        let builder = BinanceBuilder::new()
            .api_key("key")
            .secret("secret")
            .sandbox(true)
            .timeout(30)
            .recv_window(5000)
            .default_type("spot");

        assert_eq!(builder.config.api_key, Some("key".to_string()));
        assert_eq!(builder.config.secret, Some("secret".to_string()));
        assert!(builder.config.sandbox);
        assert_eq!(builder.config.timeout, 30);
        assert_eq!(builder.options.recv_window, 5000);
        assert_eq!(builder.options.default_type, "spot");
    }

    #[test]
    fn test_builder_build() {
        let result = BinanceBuilder::new().build();
        assert!(result.is_ok());

        let binance = result.unwrap();
        assert_eq!(binance.id(), "binance");
        assert_eq!(binance.name(), "Binance");
    }

    #[test]
    fn test_builder_build_with_credentials() {
        let result = BinanceBuilder::new()
            .api_key("test-key")
            .secret("test-secret")
            .build();

        assert!(result.is_ok());
    }
}
