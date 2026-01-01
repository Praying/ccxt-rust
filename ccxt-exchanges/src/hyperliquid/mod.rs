//! HyperLiquid exchange implementation.
//!
//! HyperLiquid is a decentralized perpetual futures exchange built on its own L1 blockchain.
//! Unlike centralized exchanges (CEX) like Binance or Bitget, HyperLiquid uses:
//! - Ethereum wallet private keys for authentication (EIP-712 typed data signatures)
//! - Wallet addresses as account identifiers (no registration required)
//! - USDC as the sole settlement currency
//!
//! # Features
//!
//! - Perpetual futures trading with up to 50x leverage
//! - Cross-margin and isolated margin modes
//! - Real-time WebSocket data streaming
//! - EIP-712 compliant transaction signing
//!
//! # Note on Market Types
//!
//! HyperLiquid only supports perpetual futures (Swap). Attempting to configure
//! other market types (Spot, Futures, Margin, Option) will result in an error.
//!
//! # Example
//!
//! ```no_run
//! use ccxt_exchanges::hyperliquid::HyperLiquid;
//!
//! # async fn example() -> ccxt_core::Result<()> {
//! // Create a public-only instance (no authentication)
//! let exchange = HyperLiquid::builder()
//!     .testnet(true)
//!     .build()?;
//!
//! // Fetch markets
//! let markets = exchange.fetch_markets().await?;
//! println!("Found {} markets", markets.len());
//!
//! // Create an authenticated instance
//! let exchange = HyperLiquid::builder()
//!     .private_key("0x...")
//!     .testnet(true)
//!     .build()?;
//!
//! // Fetch balance
//! let balance = exchange.fetch_balance().await?;
//! # Ok(())
//! # }
//! ```

use ccxt_core::types::default_type::DefaultType;
use ccxt_core::{BaseExchange, ExchangeConfig, Result};

pub mod auth;
pub mod builder;
pub mod endpoint_router;
pub mod error;
mod exchange_impl;
pub mod parser;
pub mod rest;
pub mod signed_request;
pub mod ws;
mod ws_exchange_impl;

pub use auth::HyperLiquidAuth;
pub use builder::{HyperLiquidBuilder, validate_default_type};
pub use endpoint_router::HyperLiquidEndpointRouter;
pub use error::{HyperLiquidErrorCode, is_error_response, parse_error};

/// HyperLiquid exchange structure.
#[derive(Debug)]
pub struct HyperLiquid {
    /// Base exchange instance.
    base: BaseExchange,
    /// HyperLiquid-specific options.
    options: HyperLiquidOptions,
    /// Authentication instance (optional, for private API).
    auth: Option<HyperLiquidAuth>,
}

/// HyperLiquid-specific options.
///
/// Note: HyperLiquid only supports perpetual futures (Swap). The `default_type`
/// field defaults to `Swap` and attempting to set it to any other value will
/// result in a validation error.
#[derive(Debug, Clone)]
pub struct HyperLiquidOptions {
    /// Whether to use testnet.
    pub testnet: bool,
    /// Vault address for vault trading (optional).
    pub vault_address: Option<String>,
    /// Default leverage multiplier.
    pub default_leverage: u32,
    /// Default market type for trading.
    ///
    /// HyperLiquid only supports perpetual futures, so this must be `Swap`.
    /// Attempting to set any other value will result in a validation error.
    pub default_type: DefaultType,
}

impl Default for HyperLiquidOptions {
    fn default() -> Self {
        Self {
            testnet: false,
            vault_address: None,
            default_leverage: 1,
            // HyperLiquid only supports perpetual futures (Swap)
            default_type: DefaultType::Swap,
        }
    }
}

impl HyperLiquid {
    /// Creates a new HyperLiquid instance using the builder pattern.
    ///
    /// This is the recommended way to create a HyperLiquid instance.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ccxt_exchanges::hyperliquid::HyperLiquid;
    ///
    /// let exchange = HyperLiquid::builder()
    ///     .private_key("0x...")
    ///     .testnet(true)
    ///     .build()
    ///     .unwrap();
    /// ```
    pub fn builder() -> HyperLiquidBuilder {
        HyperLiquidBuilder::new()
    }

    /// Creates a new HyperLiquid instance with custom options.
    ///
    /// This is used internally by the builder pattern.
    ///
    /// # Arguments
    ///
    /// * `config` - Exchange configuration.
    /// * `options` - HyperLiquid-specific options.
    /// * `auth` - Optional authentication instance.
    pub fn new_with_options(
        config: ExchangeConfig,
        options: HyperLiquidOptions,
        auth: Option<HyperLiquidAuth>,
    ) -> Result<Self> {
        let base = BaseExchange::new(config)?;
        Ok(Self {
            base,
            options,
            auth,
        })
    }

    /// Returns a reference to the base exchange.
    pub fn base(&self) -> &BaseExchange {
        &self.base
    }

    /// Returns a mutable reference to the base exchange.
    pub fn base_mut(&mut self) -> &mut BaseExchange {
        &mut self.base
    }

    /// Returns the HyperLiquid options.
    pub fn options(&self) -> &HyperLiquidOptions {
        &self.options
    }

    /// Returns a reference to the authentication instance.
    pub fn auth(&self) -> Option<&HyperLiquidAuth> {
        self.auth.as_ref()
    }

    /// Creates a signed action builder for authenticated exchange requests.
    ///
    /// This method provides a fluent API for constructing and executing
    /// authenticated Hyperliquid exchange actions using EIP-712 signing.
    ///
    /// # Arguments
    ///
    /// * `action` - The action JSON to be signed and executed
    ///
    /// # Returns
    ///
    /// A `HyperliquidSignedRequestBuilder` that can be configured and executed.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ccxt_exchanges::hyperliquid::HyperLiquid;
    /// use serde_json::json;
    ///
    /// # async fn example() -> ccxt_core::Result<()> {
    /// let exchange = HyperLiquid::builder()
    ///     .private_key("0x...")
    ///     .testnet(true)
    ///     .build()?;
    ///
    /// // Create an order
    /// let action = json!({
    ///     "type": "order",
    ///     "orders": [{"a": 0, "b": true, "p": "50000", "s": "0.001", "r": false, "t": {"limit": {"tif": "Gtc"}}}],
    ///     "grouping": "na"
    /// });
    ///
    /// let response = exchange.signed_action(action)
    ///     .execute()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn signed_action(
        &self,
        action: serde_json::Value,
    ) -> signed_request::HyperliquidSignedRequestBuilder<'_> {
        signed_request::HyperliquidSignedRequestBuilder::new(self, action)
    }

    /// Returns the exchange ID.
    pub fn id(&self) -> &str {
        "hyperliquid"
    }

    /// Returns the exchange name.
    pub fn name(&self) -> &str {
        "HyperLiquid"
    }

    /// Returns the API version.
    pub fn version(&self) -> &str {
        "1"
    }

    /// Returns `true` if the exchange is CCXT-certified.
    pub fn certified(&self) -> bool {
        false
    }

    /// Returns `true` if Pro version (WebSocket) is supported.
    pub fn pro(&self) -> bool {
        true
    }

    /// Returns the rate limit in requests per second.
    pub fn rate_limit(&self) -> u32 {
        // HyperLiquid has generous rate limits
        100
    }

    /// Returns `true` if sandbox/testnet mode is enabled.
    ///
    /// Sandbox mode is enabled when either:
    /// - `config.sandbox` is set to `true`
    /// - `options.testnet` is set to `true`
    ///
    /// # Returns
    ///
    /// `true` if sandbox mode is enabled, `false` otherwise.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ccxt_exchanges::hyperliquid::HyperLiquid;
    ///
    /// let exchange = HyperLiquid::builder()
    ///     .testnet(true)
    ///     .build()
    ///     .unwrap();
    /// assert!(exchange.is_sandbox());
    /// ```
    pub fn is_sandbox(&self) -> bool {
        self.base().config.sandbox || self.options.testnet
    }

    /// Returns the API URLs.
    ///
    /// Returns testnet URLs when sandbox mode is enabled (either via
    /// `config.sandbox` or `options.testnet`), otherwise returns mainnet URLs.
    pub fn urls(&self) -> HyperLiquidUrls {
        if self.is_sandbox() {
            HyperLiquidUrls::testnet()
        } else {
            HyperLiquidUrls::mainnet()
        }
    }

    /// Returns the wallet address if authenticated.
    pub fn wallet_address(&self) -> Option<&str> {
        self.auth.as_ref().map(|a| a.wallet_address())
    }

    // TODO: Implement in task 11 (WebSocket Implementation)
    // /// Creates a public WebSocket client.
    // pub fn create_ws(&self) -> ws::HyperLiquidWs {
    //     let urls = self.urls();
    //     ws::HyperLiquidWs::new(urls.ws)
    // }
}

/// HyperLiquid API URLs.
#[derive(Debug, Clone)]
pub struct HyperLiquidUrls {
    /// REST API base URL.
    pub rest: String,
    /// WebSocket URL.
    pub ws: String,
}

impl HyperLiquidUrls {
    /// Returns mainnet environment URLs.
    pub fn mainnet() -> Self {
        Self {
            rest: "https://api.hyperliquid.xyz".to_string(),
            ws: "wss://api.hyperliquid.xyz/ws".to_string(),
        }
    }

    /// Returns testnet environment URLs.
    pub fn testnet() -> Self {
        Self {
            rest: "https://api.hyperliquid-testnet.xyz".to_string(),
            ws: "wss://api.hyperliquid-testnet.xyz/ws".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_options() {
        let options = HyperLiquidOptions::default();
        assert!(!options.testnet);
        assert!(options.vault_address.is_none());
        assert_eq!(options.default_leverage, 1);
        // HyperLiquid only supports perpetual futures, so default_type must be Swap
        assert_eq!(options.default_type, DefaultType::Swap);
    }

    #[test]
    fn test_mainnet_urls() {
        let urls = HyperLiquidUrls::mainnet();
        assert_eq!(urls.rest, "https://api.hyperliquid.xyz");
        assert_eq!(urls.ws, "wss://api.hyperliquid.xyz/ws");
    }

    #[test]
    fn test_testnet_urls() {
        let urls = HyperLiquidUrls::testnet();
        assert_eq!(urls.rest, "https://api.hyperliquid-testnet.xyz");
        assert_eq!(urls.ws, "wss://api.hyperliquid-testnet.xyz/ws");
    }

    #[test]
    fn test_is_sandbox_with_options_testnet() {
        let config = ExchangeConfig::default();
        let options = HyperLiquidOptions {
            testnet: true,
            ..Default::default()
        };
        let exchange = HyperLiquid::new_with_options(config, options, None).unwrap();
        assert!(exchange.is_sandbox());
    }

    #[test]
    fn test_is_sandbox_with_config_sandbox() {
        let config = ExchangeConfig {
            sandbox: true,
            ..Default::default()
        };
        let options = HyperLiquidOptions::default();
        let exchange = HyperLiquid::new_with_options(config, options, None).unwrap();
        assert!(exchange.is_sandbox());
    }

    #[test]
    fn test_is_sandbox_false_by_default() {
        let config = ExchangeConfig::default();
        let options = HyperLiquidOptions::default();
        let exchange = HyperLiquid::new_with_options(config, options, None).unwrap();
        assert!(!exchange.is_sandbox());
    }

    #[test]
    fn test_urls_returns_mainnet_by_default() {
        let config = ExchangeConfig::default();
        let options = HyperLiquidOptions::default();
        let exchange = HyperLiquid::new_with_options(config, options, None).unwrap();
        let urls = exchange.urls();
        assert_eq!(urls.rest, "https://api.hyperliquid.xyz");
        assert_eq!(urls.ws, "wss://api.hyperliquid.xyz/ws");
    }

    #[test]
    fn test_urls_returns_testnet_with_options_testnet() {
        let config = ExchangeConfig::default();
        let options = HyperLiquidOptions {
            testnet: true,
            ..Default::default()
        };
        let exchange = HyperLiquid::new_with_options(config, options, None).unwrap();
        let urls = exchange.urls();
        assert_eq!(urls.rest, "https://api.hyperliquid-testnet.xyz");
        assert_eq!(urls.ws, "wss://api.hyperliquid-testnet.xyz/ws");
    }

    #[test]
    fn test_urls_returns_testnet_with_config_sandbox() {
        let config = ExchangeConfig {
            sandbox: true,
            ..Default::default()
        };
        let options = HyperLiquidOptions::default();
        let exchange = HyperLiquid::new_with_options(config, options, None).unwrap();
        let urls = exchange.urls();
        assert_eq!(urls.rest, "https://api.hyperliquid-testnet.xyz");
        assert_eq!(urls.ws, "wss://api.hyperliquid-testnet.xyz/ws");
    }
}
