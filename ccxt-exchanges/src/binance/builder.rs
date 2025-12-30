//! Binance exchange builder pattern implementation.
//!
//! Provides a fluent API for constructing Binance exchange instances with
//! type-safe configuration options.

use super::{Binance, BinanceOptions};
use ccxt_core::config::{ProxyConfig, RetryPolicy};
use ccxt_core::types::default_type::{DefaultSubType, DefaultType};
use ccxt_core::{ExchangeConfig, Result};
use serde_json::Value;
use std::collections::HashMap;
use std::time::Duration;

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

    /// Sets the request timeout.
    ///
    /// # Arguments
    ///
    /// * `timeout` - Timeout duration.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ccxt_exchanges::binance::BinanceBuilder;
    /// use std::time::Duration;
    ///
    /// let builder = BinanceBuilder::new()
    ///     .timeout(Duration::from_secs(60));
    /// ```
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.config.timeout = timeout;
        self
    }

    /// Sets the request timeout in seconds (convenience method).
    ///
    /// # Arguments
    ///
    /// * `seconds` - Timeout duration in seconds.
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
    /// Accepts both `DefaultType` enum values and string values for backward compatibility.
    /// Valid string values: "spot", "margin", "swap", "futures", "option".
    /// Legacy values "future" and "delivery" are also supported.
    ///
    /// # Arguments
    ///
    /// * `trading_type` - The default trading type (DefaultType or string).
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ccxt_exchanges::binance::BinanceBuilder;
    /// use ccxt_core::types::default_type::DefaultType;
    ///
    /// // Using DefaultType enum (recommended)
    /// let builder = BinanceBuilder::new()
    ///     .default_type(DefaultType::Swap);
    ///
    /// // Using string (backward compatible)
    /// let builder = BinanceBuilder::new()
    ///     .default_type("swap");
    /// ```
    pub fn default_type(mut self, trading_type: impl Into<DefaultType>) -> Self {
        self.options.default_type = trading_type.into();
        self
    }

    /// Sets the default sub-type for contract settlement.
    ///
    /// - `Linear`: USDT-margined contracts (FAPI)
    /// - `Inverse`: Coin-margined contracts (DAPI)
    ///
    /// Only applicable when `default_type` is `Swap`, `Futures`, or `Option`.
    ///
    /// # Arguments
    ///
    /// * `sub_type` - The default sub-type for contract settlement.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ccxt_exchanges::binance::BinanceBuilder;
    /// use ccxt_core::types::default_type::{DefaultType, DefaultSubType};
    ///
    /// let builder = BinanceBuilder::new()
    ///     .default_type(DefaultType::Swap)
    ///     .default_sub_type(DefaultSubType::Linear);
    /// ```
    pub fn default_sub_type(mut self, sub_type: DefaultSubType) -> Self {
        self.options.default_sub_type = Some(sub_type);
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

    /// Sets the time sync interval in seconds.
    ///
    /// Controls how often the time offset is refreshed when auto sync is enabled.
    /// Default is 30 seconds.
    ///
    /// # Arguments
    ///
    /// * `seconds` - Sync interval in seconds.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ccxt_exchanges::binance::BinanceBuilder;
    ///
    /// let builder = BinanceBuilder::new()
    ///     .time_sync_interval(60); // Sync every 60 seconds
    /// ```
    pub fn time_sync_interval(mut self, seconds: u64) -> Self {
        self.options.time_sync_interval_secs = seconds;
        self
    }

    /// Enables or disables automatic periodic time sync.
    ///
    /// When enabled, the time offset will be automatically refreshed
    /// based on the configured sync interval.
    /// Default is true.
    ///
    /// # Arguments
    ///
    /// * `enabled` - Whether to enable automatic time sync.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ccxt_exchanges::binance::BinanceBuilder;
    ///
    /// // Disable automatic sync for manual control
    /// let builder = BinanceBuilder::new()
    ///     .auto_time_sync(false);
    /// ```
    pub fn auto_time_sync(mut self, enabled: bool) -> Self {
        self.options.auto_time_sync = enabled;
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

    /// Sets the HTTP proxy configuration.
    ///
    /// # Arguments
    ///
    /// * `proxy` - The proxy configuration.
    pub fn proxy(mut self, proxy: ProxyConfig) -> Self {
        self.config.proxy = Some(proxy);
        self
    }

    /// Sets the HTTP proxy URL (convenience method).
    ///
    /// # Arguments
    ///
    /// * `url` - The proxy server URL.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ccxt_exchanges::binance::BinanceBuilder;
    ///
    /// let builder = BinanceBuilder::new()
    ///     .proxy_url("http://proxy.example.com:8080");
    /// ```
    pub fn proxy_url(mut self, url: impl Into<String>) -> Self {
        self.config.proxy = Some(ProxyConfig::new(url));
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
        assert_eq!(builder.options.default_type, DefaultType::Spot);
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
        let builder = BinanceBuilder::new().timeout(Duration::from_secs(60));
        assert_eq!(builder.config.timeout, Duration::from_secs(60));
    }

    #[test]
    fn test_builder_recv_window() {
        let builder = BinanceBuilder::new().recv_window(10000);
        assert_eq!(builder.options.recv_window, 10000);
    }

    #[test]
    fn test_builder_default_type_with_enum() {
        let builder = BinanceBuilder::new().default_type(DefaultType::Swap);
        assert_eq!(builder.options.default_type, DefaultType::Swap);
    }

    #[test]
    fn test_builder_default_type_with_string() {
        // Test backward compatibility with string values
        let builder = BinanceBuilder::new().default_type("swap");
        assert_eq!(builder.options.default_type, DefaultType::Swap);
    }

    #[test]
    fn test_builder_default_type_legacy_future() {
        // Test backward compatibility with legacy "future" value
        let builder = BinanceBuilder::new().default_type("future");
        assert_eq!(builder.options.default_type, DefaultType::Swap);
    }

    #[test]
    fn test_builder_default_sub_type() {
        let builder = BinanceBuilder::new()
            .default_type(DefaultType::Swap)
            .default_sub_type(DefaultSubType::Linear);
        assert_eq!(builder.options.default_type, DefaultType::Swap);
        assert_eq!(
            builder.options.default_sub_type,
            Some(DefaultSubType::Linear)
        );
    }

    #[test]
    fn test_builder_chaining() {
        let builder = BinanceBuilder::new()
            .api_key("key")
            .secret("secret")
            .sandbox(true)
            .timeout(Duration::from_secs(30))
            .recv_window(5000)
            .default_type(DefaultType::Spot);

        assert_eq!(builder.config.api_key, Some("key".to_string()));
        assert_eq!(builder.config.secret, Some("secret".to_string()));
        assert!(builder.config.sandbox);
        assert_eq!(builder.config.timeout, Duration::from_secs(30));
        assert_eq!(builder.options.recv_window, 5000);
        assert_eq!(builder.options.default_type, DefaultType::Spot);
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

    #[test]
    fn test_builder_time_sync_interval() {
        let builder = BinanceBuilder::new().time_sync_interval(60);
        assert_eq!(builder.options.time_sync_interval_secs, 60);
    }

    #[test]
    fn test_builder_auto_time_sync() {
        let builder = BinanceBuilder::new().auto_time_sync(false);
        assert!(!builder.options.auto_time_sync);
    }

    #[test]
    fn test_builder_time_sync_chaining() {
        let builder = BinanceBuilder::new()
            .time_sync_interval(120)
            .auto_time_sync(false);

        assert_eq!(builder.options.time_sync_interval_secs, 120);
        assert!(!builder.options.auto_time_sync);
    }

    #[test]
    fn test_builder_build_with_time_sync_config() {
        let result = BinanceBuilder::new()
            .time_sync_interval(60)
            .auto_time_sync(true)
            .build();

        assert!(result.is_ok());
        let binance = result.unwrap();

        // Verify the TimeSyncManager was created with correct config
        let time_sync = binance.time_sync();
        assert_eq!(
            time_sync.config().sync_interval,
            std::time::Duration::from_secs(60)
        );
        assert!(time_sync.config().auto_sync);
    }

    #[test]
    fn test_builder_build_time_sync_disabled() {
        let result = BinanceBuilder::new().auto_time_sync(false).build();

        assert!(result.is_ok());
        let binance = result.unwrap();

        // Verify auto_sync is disabled
        let time_sync = binance.time_sync();
        assert!(!time_sync.config().auto_sync);
    }
}
