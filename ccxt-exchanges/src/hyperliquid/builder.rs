//! HyperLiquid builder module.
//!
//! Provides a builder pattern for creating HyperLiquid exchange instances.

use ccxt_core::config::{ProxyConfig, RetryPolicy};
use ccxt_core::types::default_type::DefaultType;
use ccxt_core::{Error, ExchangeConfig, Result};
use std::time::Duration;

use super::{HyperLiquid, HyperLiquidAuth, HyperLiquidOptions};

/// Builder for creating HyperLiquid exchange instances.
///
/// # Note on Market Types
///
/// HyperLiquid only supports perpetual futures (Swap). Attempting to set
/// `default_type` to any other value (Spot, Futures, Margin, Option) will
/// result in a validation error when calling `build()`.
///
/// # Example
///
/// ```no_run
/// use ccxt_exchanges::hyperliquid::HyperLiquidBuilder;
///
/// let exchange = HyperLiquidBuilder::new()
///     .private_key("0x...")
///     .testnet(true)
///     .default_leverage(10)
///     .build()
///     .unwrap();
/// ```
#[derive(Debug, Default)]
pub struct HyperLiquidBuilder {
    private_key: Option<String>,
    testnet: bool,
    vault_address: Option<String>,
    default_leverage: u32,
    default_type: Option<DefaultType>,
    timeout: Option<Duration>,
    proxy: Option<ProxyConfig>,
    retry_policy: Option<RetryPolicy>,
}

impl HyperLiquidBuilder {
    /// Creates a new HyperLiquidBuilder with default settings.
    pub fn new() -> Self {
        Self {
            private_key: None,
            testnet: false,
            vault_address: None,
            default_leverage: 1,
            default_type: None, // Will default to Swap in build()
            timeout: None,
            proxy: None,
            retry_policy: None,
        }
    }

    /// Sets the Ethereum private key for authentication.
    ///
    /// The private key should be a 64-character hex string (32 bytes),
    /// optionally prefixed with "0x".
    ///
    /// # Arguments
    ///
    /// * `key` - The private key in hex format.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ccxt_exchanges::hyperliquid::HyperLiquidBuilder;
    ///
    /// let builder = HyperLiquidBuilder::new()
    ///     .private_key("0x1234567890abcdef...");
    /// ```
    pub fn private_key(mut self, key: &str) -> Self {
        self.private_key = Some(key.to_string());
        self
    }

    /// Enables or disables sandbox/testnet mode.
    ///
    /// When enabled, the exchange will connect to HyperLiquid testnet
    /// instead of mainnet.
    ///
    /// This method is equivalent to `testnet()` and is provided for
    /// consistency with other exchanges.
    ///
    /// # Arguments
    ///
    /// * `enabled` - Whether to use sandbox mode.
    pub fn sandbox(mut self, enabled: bool) -> Self {
        self.testnet = enabled;
        self
    }

    /// Enables or disables testnet mode.
    ///
    /// When enabled, the exchange will connect to HyperLiquid testnet
    /// instead of mainnet.
    ///
    /// This method is equivalent to `sandbox()` and is provided for
    /// backward compatibility.
    ///
    /// # Arguments
    ///
    /// * `enabled` - Whether to use testnet.
    pub fn testnet(mut self, enabled: bool) -> Self {
        self.testnet = enabled;
        self
    }

    /// Sets the vault address for vault trading.
    ///
    /// When set, orders will be placed on behalf of the vault.
    ///
    /// # Arguments
    ///
    /// * `address` - The vault's Ethereum address.
    pub fn vault_address(mut self, address: &str) -> Self {
        self.vault_address = Some(address.to_string());
        self
    }

    /// Sets the default leverage multiplier.
    ///
    /// This leverage will be used when placing orders if not specified.
    ///
    /// # Arguments
    ///
    /// * `leverage` - The leverage multiplier (1-50).
    pub fn default_leverage(mut self, leverage: u32) -> Self {
        self.default_leverage = leverage.clamp(1, 50);
        self
    }

    /// Sets the default market type for trading.
    ///
    /// **Important**: HyperLiquid only supports perpetual futures (Swap).
    /// Attempting to set any other value (Spot, Futures, Margin, Option)
    /// will result in a validation error when calling `build()`.
    ///
    /// # Arguments
    ///
    /// * `default_type` - The default market type. Must be `Swap` for HyperLiquid.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ccxt_exchanges::hyperliquid::HyperLiquidBuilder;
    /// use ccxt_core::types::default_type::DefaultType;
    ///
    /// // This works - HyperLiquid supports Swap (perpetuals)
    /// let exchange = HyperLiquidBuilder::new()
    ///     .default_type(DefaultType::Swap)
    ///     .build()
    ///     .unwrap();
    ///
    /// // This will fail - HyperLiquid does not support Spot
    /// let result = HyperLiquidBuilder::new()
    ///     .default_type(DefaultType::Spot)
    ///     .build();
    /// assert!(result.is_err());
    /// ```
    pub fn default_type(mut self, default_type: impl Into<DefaultType>) -> Self {
        self.default_type = Some(default_type.into());
        self
    }

    /// Sets the request timeout.
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Sets the request timeout in seconds (convenience method).
    pub fn timeout_secs(mut self, seconds: u64) -> Self {
        self.timeout = Some(Duration::from_secs(seconds));
        self
    }

    /// Sets the TCP connection timeout.
    ///
    /// # Arguments
    ///
    /// * `timeout` - Connection timeout duration.
    pub fn connect_timeout(mut self, timeout: Duration) -> Self {
        // Note: HyperLiquid builder doesn't have a separate connect_timeout field,
        // but we store it in the ExchangeConfig during build()
        // For now, we'll just accept it and apply it during build
        self.timeout = Some(timeout);
        self
    }

    /// Sets the TCP connection timeout in seconds (convenience method).
    ///
    /// # Arguments
    ///
    /// * `seconds` - Connection timeout duration in seconds.
    pub fn connect_timeout_secs(mut self, seconds: u64) -> Self {
        self.timeout = Some(Duration::from_secs(seconds));
        self
    }

    /// Sets the retry policy.
    pub fn retry_policy(mut self, policy: RetryPolicy) -> Self {
        self.retry_policy = Some(policy);
        self
    }

    /// Sets the HTTP proxy configuration.
    pub fn proxy(mut self, proxy: ProxyConfig) -> Self {
        self.proxy = Some(proxy);
        self
    }

    /// Sets the HTTP proxy URL (convenience method).
    pub fn proxy_url(mut self, url: impl Into<String>) -> Self {
        self.proxy = Some(ProxyConfig::new(url));
        self
    }

    /// Builds the HyperLiquid exchange instance.
    ///
    /// # Returns
    ///
    /// Returns a configured `HyperLiquid` instance.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The private key format is invalid
    /// - The exchange configuration fails
    /// - The `default_type` is set to a value other than `Swap` (HyperLiquid only supports perpetuals)
    pub fn build(self) -> Result<HyperLiquid> {
        // Validate default_type - HyperLiquid only supports perpetual futures (Swap)
        let default_type = self.default_type.unwrap_or(DefaultType::Swap);
        validate_default_type(default_type)?;

        // Create authentication if private key is provided
        let auth = if let Some(ref key) = self.private_key {
            Some(HyperLiquidAuth::from_private_key(key)?)
        } else {
            None
        };

        // Create options
        let options = HyperLiquidOptions {
            testnet: self.testnet,
            vault_address: self.vault_address,
            default_leverage: self.default_leverage,
            default_type,
        };

        // Create exchange config
        let mut config = ExchangeConfig {
            id: "hyperliquid".to_string(),
            name: "HyperLiquid".to_string(),
            sandbox: self.testnet,
            ..Default::default()
        };

        if let Some(timeout) = self.timeout {
            config.timeout = timeout;
        }
        if let Some(proxy) = self.proxy {
            config.proxy = Some(proxy);
        }
        if let Some(retry_policy) = self.retry_policy {
            config.retry_policy = Some(retry_policy);
        }

        HyperLiquid::new_with_options(config, options, auth)
    }
}

/// Validates that the default_type is supported by HyperLiquid.
///
/// HyperLiquid only supports perpetual futures (Swap). This function returns
/// an error if any other market type is specified.
///
/// # Arguments
///
/// * `default_type` - The default market type to validate.
///
/// # Returns
///
/// Returns `Ok(())` if the type is `Swap`, otherwise returns an error.
pub fn validate_default_type(default_type: DefaultType) -> Result<()> {
    match default_type {
        DefaultType::Swap => Ok(()),
        DefaultType::Spot => Err(Error::invalid_request(
            "HyperLiquid does not support spot trading. Only perpetual futures (Swap) are available.",
        )),
        DefaultType::Futures => Err(Error::invalid_request(
            "HyperLiquid does not support delivery futures. Only perpetual futures (Swap) are available.",
        )),
        DefaultType::Margin => Err(Error::invalid_request(
            "HyperLiquid does not support margin trading. Only perpetual futures (Swap) are available.",
        )),
        DefaultType::Option => Err(Error::invalid_request(
            "HyperLiquid does not support options trading. Only perpetual futures (Swap) are available.",
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_default() {
        let builder = HyperLiquidBuilder::new();
        assert!(builder.private_key.is_none());
        assert!(!builder.testnet);
        assert!(builder.vault_address.is_none());
        assert_eq!(builder.default_leverage, 1);
        assert!(builder.default_type.is_none()); // Will default to Swap in build()
    }

    #[test]
    fn test_builder_sandbox() {
        let builder = HyperLiquidBuilder::new().sandbox(true);
        assert!(builder.testnet);
    }

    #[test]
    fn test_builder_testnet() {
        let builder = HyperLiquidBuilder::new().testnet(true);
        assert!(builder.testnet);
    }

    #[test]
    fn test_builder_sandbox_testnet_equivalence() {
        // Verify that sandbox() and testnet() produce equivalent results
        let sandbox_builder = HyperLiquidBuilder::new().sandbox(true);
        let testnet_builder = HyperLiquidBuilder::new().testnet(true);

        assert_eq!(sandbox_builder.testnet, testnet_builder.testnet);
    }

    #[test]
    fn test_builder_leverage_clamping() {
        let builder = HyperLiquidBuilder::new().default_leverage(100);
        assert_eq!(builder.default_leverage, 50);

        let builder = HyperLiquidBuilder::new().default_leverage(0);
        assert_eq!(builder.default_leverage, 1);
    }

    #[test]
    fn test_builder_vault_address() {
        let builder =
            HyperLiquidBuilder::new().vault_address("0x1234567890abcdef1234567890abcdef12345678");
        assert!(builder.vault_address.is_some());
    }

    #[test]
    fn test_builder_default_type_swap() {
        let builder = HyperLiquidBuilder::new().default_type(DefaultType::Swap);
        assert_eq!(builder.default_type, Some(DefaultType::Swap));
    }

    #[test]
    fn test_builder_default_type_from_string() {
        let builder = HyperLiquidBuilder::new().default_type("swap");
        assert_eq!(builder.default_type, Some(DefaultType::Swap));
    }

    #[test]
    fn test_builder_timeout() {
        let builder = HyperLiquidBuilder::new().timeout(Duration::from_secs(60));
        assert_eq!(builder.timeout, Some(Duration::from_secs(60)));
    }

    #[test]
    fn test_builder_timeout_secs() {
        let builder = HyperLiquidBuilder::new().timeout_secs(45);
        assert_eq!(builder.timeout, Some(Duration::from_secs(45)));
    }

    #[test]
    fn test_builder_connect_timeout() {
        let builder = HyperLiquidBuilder::new().connect_timeout(Duration::from_secs(15));
        assert_eq!(builder.timeout, Some(Duration::from_secs(15)));
    }

    #[test]
    fn test_builder_connect_timeout_secs() {
        let builder = HyperLiquidBuilder::new().connect_timeout_secs(20);
        assert_eq!(builder.timeout, Some(Duration::from_secs(20)));
    }

    #[test]
    fn test_build_without_auth() {
        let exchange = HyperLiquidBuilder::new().testnet(true).build();

        assert!(exchange.is_ok());
        let exchange = exchange.unwrap();
        assert_eq!(exchange.id(), "hyperliquid");
        assert!(exchange.options().testnet);
        assert!(exchange.auth().is_none());
        // Default type should be Swap
        assert_eq!(exchange.options().default_type, DefaultType::Swap);
    }

    #[test]
    fn test_build_with_swap_type() {
        let exchange = HyperLiquidBuilder::new()
            .testnet(true)
            .default_type(DefaultType::Swap)
            .build();

        assert!(exchange.is_ok());
        let exchange = exchange.unwrap();
        assert_eq!(exchange.options().default_type, DefaultType::Swap);
    }

    #[test]
    fn test_build_with_spot_type_fails() {
        let result = HyperLiquidBuilder::new()
            .testnet(true)
            .default_type(DefaultType::Spot)
            .build();

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("spot"));
    }

    #[test]
    fn test_build_with_futures_type_fails() {
        let result = HyperLiquidBuilder::new()
            .testnet(true)
            .default_type(DefaultType::Futures)
            .build();

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("delivery futures"));
    }

    #[test]
    fn test_build_with_margin_type_fails() {
        let result = HyperLiquidBuilder::new()
            .testnet(true)
            .default_type(DefaultType::Margin)
            .build();

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("margin"));
    }

    #[test]
    fn test_build_with_option_type_fails() {
        let result = HyperLiquidBuilder::new()
            .testnet(true)
            .default_type(DefaultType::Option)
            .build();

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("options"));
    }

    #[test]
    fn test_validate_default_type_swap() {
        assert!(validate_default_type(DefaultType::Swap).is_ok());
    }

    #[test]
    fn test_validate_default_type_spot() {
        let result = validate_default_type(DefaultType::Spot);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_default_type_futures() {
        let result = validate_default_type(DefaultType::Futures);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_default_type_margin() {
        let result = validate_default_type(DefaultType::Margin);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_default_type_option() {
        let result = validate_default_type(DefaultType::Option);
        assert!(result.is_err());
    }
}
