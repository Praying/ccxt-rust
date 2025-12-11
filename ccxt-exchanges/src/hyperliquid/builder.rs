//! HyperLiquid builder module.
//!
//! Provides a builder pattern for creating HyperLiquid exchange instances.

use ccxt_core::{ExchangeConfig, Result};

use super::{HyperLiquid, HyperLiquidAuth, HyperLiquidOptions};

/// Builder for creating HyperLiquid exchange instances.
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
}

impl HyperLiquidBuilder {
    /// Creates a new HyperLiquidBuilder with default settings.
    pub fn new() -> Self {
        Self {
            private_key: None,
            testnet: false,
            vault_address: None,
            default_leverage: 1,
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

    /// Enables or disables testnet mode.
    ///
    /// When enabled, the exchange will connect to HyperLiquid testnet
    /// instead of mainnet.
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
    pub fn build(self) -> Result<HyperLiquid> {
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
        };

        // Create exchange config
        let config = ExchangeConfig {
            id: "hyperliquid".to_string(),
            name: "HyperLiquid".to_string(),
            sandbox: self.testnet,
            ..Default::default()
        };

        HyperLiquid::new_with_options(config, options, auth)
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
    }

    #[test]
    fn test_builder_testnet() {
        let builder = HyperLiquidBuilder::new().testnet(true);
        assert!(builder.testnet);
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
    fn test_build_without_auth() {
        let exchange = HyperLiquidBuilder::new().testnet(true).build();

        assert!(exchange.is_ok());
        let exchange = exchange.unwrap();
        assert_eq!(exchange.id(), "hyperliquid");
        assert!(exchange.options().testnet);
        assert!(exchange.auth().is_none());
    }
}
