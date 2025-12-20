//! Bitget exchange implementation.
//!
//! Supports spot trading and futures trading (USDT-M and Coin-M) with REST API and WebSocket support.

use ccxt_core::types::default_type::{DefaultSubType, DefaultType};
use ccxt_core::{BaseExchange, ExchangeConfig, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub mod auth;
pub mod builder;
pub mod error;
mod exchange_impl;
pub mod parser;
pub mod rest;
pub mod ws;
mod ws_exchange_impl;

pub use auth::BitgetAuth;
pub use builder::BitgetBuilder;
pub use error::{BitgetErrorCode, is_error_response, parse_error};
pub use parser::{
    datetime_to_timestamp, parse_balance, parse_market, parse_ohlcv, parse_order,
    parse_order_status, parse_orderbook, parse_ticker, parse_trade, timestamp_to_datetime,
};

/// Bitget exchange structure.
#[derive(Debug)]
pub struct Bitget {
    /// Base exchange instance.
    base: BaseExchange,
    /// Bitget-specific options.
    options: BitgetOptions,
}

/// Bitget-specific options.
///
/// # Example
///
/// ```rust
/// use ccxt_exchanges::bitget::BitgetOptions;
/// use ccxt_core::types::default_type::{DefaultType, DefaultSubType};
///
/// let options = BitgetOptions {
///     default_type: DefaultType::Swap,
///     default_sub_type: Some(DefaultSubType::Linear),
///     ..Default::default()
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BitgetOptions {
    /// Product type: spot, umcbl (USDT-M), dmcbl (Coin-M).
    ///
    /// This is kept for backward compatibility with existing configurations.
    /// When using `default_type` and `default_sub_type`, this field is automatically
    /// derived from those values.
    pub product_type: String,
    /// Default trading type (spot/swap/futures/option).
    ///
    /// This determines which product type to use for API calls.
    /// Bitget uses product_type-based filtering:
    /// - `Spot` -> product_type=spot
    /// - `Swap` + Linear -> product_type=umcbl (USDT-M)
    /// - `Swap` + Inverse -> product_type=dmcbl (Coin-M)
    /// - `Futures` + Linear -> product_type=umcbl (USDT-M futures)
    /// - `Futures` + Inverse -> product_type=dmcbl (Coin-M futures)
    #[serde(default)]
    pub default_type: DefaultType,
    /// Default sub-type for contract settlement (linear/inverse).
    ///
    /// - `Linear`: USDT-margined contracts (product_type=umcbl)
    /// - `Inverse`: Coin-margined contracts (product_type=dmcbl)
    ///
    /// Only applicable when `default_type` is `Swap` or `Futures`.
    /// Ignored for `Spot` type.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_sub_type: Option<DefaultSubType>,
    /// Receive window in milliseconds.
    pub recv_window: u64,
    /// Enables testnet environment.
    pub testnet: bool,
}

impl Default for BitgetOptions {
    fn default() -> Self {
        Self {
            product_type: "spot".to_string(),
            default_type: DefaultType::default(), // Defaults to Spot
            default_sub_type: None,
            recv_window: 5000,
            testnet: false,
        }
    }
}

impl BitgetOptions {
    /// Returns the effective product_type based on default_type and default_sub_type.
    ///
    /// This method maps the DefaultType and DefaultSubType to Bitget's product_type:
    /// - `Spot` -> "spot"
    /// - `Swap` + Linear (or None) -> "umcbl" (USDT-M perpetuals)
    /// - `Swap` + Inverse -> "dmcbl" (Coin-M perpetuals)
    /// - `Futures` + Linear (or None) -> "umcbl" (USDT-M futures)
    /// - `Futures` + Inverse -> "dmcbl" (Coin-M futures)
    /// - `Margin` -> "spot" (margin uses spot markets)
    /// - `Option` -> "spot" (options not fully supported, fallback to spot)
    ///
    /// # Example
    ///
    /// ```rust
    /// use ccxt_exchanges::bitget::BitgetOptions;
    /// use ccxt_core::types::default_type::{DefaultType, DefaultSubType};
    ///
    /// let mut options = BitgetOptions::default();
    /// options.default_type = DefaultType::Swap;
    /// options.default_sub_type = Some(DefaultSubType::Linear);
    /// assert_eq!(options.effective_product_type(), "umcbl");
    ///
    /// options.default_sub_type = Some(DefaultSubType::Inverse);
    /// assert_eq!(options.effective_product_type(), "dmcbl");
    /// ```
    pub fn effective_product_type(&self) -> &str {
        match self.default_type {
            DefaultType::Spot => "spot",
            DefaultType::Margin => "spot", // Margin uses spot markets
            DefaultType::Swap | DefaultType::Futures => {
                match self.default_sub_type.unwrap_or(DefaultSubType::Linear) {
                    DefaultSubType::Linear => "umcbl",  // USDT-M
                    DefaultSubType::Inverse => "dmcbl", // Coin-M
                }
            }
            DefaultType::Option => "spot", // Options not fully supported, fallback to spot
        }
    }
}

impl Bitget {
    /// Creates a new Bitget instance using the builder pattern.
    ///
    /// This is the recommended way to create a Bitget instance.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ccxt_exchanges::bitget::Bitget;
    ///
    /// let bitget = Bitget::builder()
    ///     .api_key("your-api-key")
    ///     .secret("your-secret")
    ///     .passphrase("your-passphrase")
    ///     .sandbox(true)
    ///     .build()
    ///     .unwrap();
    /// ```
    pub fn builder() -> BitgetBuilder {
        BitgetBuilder::new()
    }

    /// Creates a new Bitget instance.
    ///
    /// # Arguments
    ///
    /// * `config` - Exchange configuration.
    pub fn new(config: ExchangeConfig) -> Result<Self> {
        let base = BaseExchange::new(config)?;
        let options = BitgetOptions::default();

        Ok(Self { base, options })
    }

    /// Creates a new Bitget instance with custom options.
    ///
    /// This is used internally by the builder pattern.
    ///
    /// # Arguments
    ///
    /// * `config` - Exchange configuration.
    /// * `options` - Bitget-specific options.
    pub fn new_with_options(config: ExchangeConfig, options: BitgetOptions) -> Result<Self> {
        let base = BaseExchange::new(config)?;
        Ok(Self { base, options })
    }

    /// Returns a reference to the base exchange.
    pub fn base(&self) -> &BaseExchange {
        &self.base
    }

    /// Returns a mutable reference to the base exchange.
    pub fn base_mut(&mut self) -> &mut BaseExchange {
        &mut self.base
    }

    /// Returns the Bitget options.
    pub fn options(&self) -> &BitgetOptions {
        &self.options
    }

    /// Sets the Bitget options.
    pub fn set_options(&mut self, options: BitgetOptions) {
        self.options = options;
    }

    /// Returns the exchange ID.
    pub fn id(&self) -> &str {
        "bitget"
    }

    /// Returns the exchange name.
    pub fn name(&self) -> &str {
        "Bitget"
    }

    /// Returns the API version.
    pub fn version(&self) -> &str {
        "v2"
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
        20
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
    /// use ccxt_exchanges::bitget::Bitget;
    /// use ccxt_core::ExchangeConfig;
    ///
    /// let config = ExchangeConfig {
    ///     sandbox: true,
    ///     ..Default::default()
    /// };
    /// let bitget = Bitget::new(config).unwrap();
    /// assert!(bitget.is_sandbox());
    /// ```
    pub fn is_sandbox(&self) -> bool {
        self.base().config.sandbox || self.options.testnet
    }

    /// Returns the supported timeframes.
    pub fn timeframes(&self) -> HashMap<String, String> {
        let mut timeframes = HashMap::new();
        timeframes.insert("1m".to_string(), "1m".to_string());
        timeframes.insert("5m".to_string(), "5m".to_string());
        timeframes.insert("15m".to_string(), "15m".to_string());
        timeframes.insert("30m".to_string(), "30m".to_string());
        timeframes.insert("1h".to_string(), "1H".to_string());
        timeframes.insert("4h".to_string(), "4H".to_string());
        timeframes.insert("6h".to_string(), "6H".to_string());
        timeframes.insert("12h".to_string(), "12H".to_string());
        timeframes.insert("1d".to_string(), "1D".to_string());
        timeframes.insert("3d".to_string(), "3D".to_string());
        timeframes.insert("1w".to_string(), "1W".to_string());
        timeframes.insert("1M".to_string(), "1M".to_string());
        timeframes
    }

    /// Returns the API URLs.
    ///
    /// Returns testnet URLs when sandbox mode is enabled (via `config.sandbox` or `options.testnet`),
    /// otherwise returns production URLs.
    ///
    /// # Returns
    ///
    /// - `BitgetUrls::testnet()` when sandbox mode is enabled
    /// - `BitgetUrls::production()` when sandbox mode is disabled
    pub fn urls(&self) -> BitgetUrls {
        if self.base().config.sandbox || self.options.testnet {
            BitgetUrls::testnet()
        } else {
            BitgetUrls::production()
        }
    }

    /// Creates a public WebSocket client.
    ///
    /// # Returns
    ///
    /// Returns a `BitgetWs` instance for public data streams.
    ///
    /// # Example
    /// ```no_run
    /// use ccxt_exchanges::bitget::Bitget;
    /// use ccxt_core::ExchangeConfig;
    ///
    /// # async fn example() -> ccxt_core::error::Result<()> {
    /// let bitget = Bitget::new(ExchangeConfig::default())?;
    /// let ws = bitget.create_ws();
    /// ws.connect().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn create_ws(&self) -> ws::BitgetWs {
        let urls = self.urls();
        ws::BitgetWs::new(urls.ws_public)
    }
}

/// Bitget API URLs.
#[derive(Debug, Clone)]
pub struct BitgetUrls {
    /// REST API base URL.
    pub rest: String,
    /// Public WebSocket URL.
    pub ws_public: String,
    /// Private WebSocket URL.
    pub ws_private: String,
}

impl BitgetUrls {
    /// Returns production environment URLs.
    pub fn production() -> Self {
        Self {
            rest: "https://api.bitget.com".to_string(),
            ws_public: "wss://ws.bitget.com/v2/ws/public".to_string(),
            ws_private: "wss://ws.bitget.com/v2/ws/private".to_string(),
        }
    }

    /// Returns testnet environment URLs.
    ///
    /// Testnet uses completely isolated domains for both REST and WebSocket APIs.
    /// This is the recommended environment for testing without risking real funds.
    ///
    /// # URLs
    ///
    /// - REST: `https://api-testnet.bitget.com`
    /// - WS Public: `wss://ws-testnet.bitget.com/v2/ws/public`
    /// - WS Private: `wss://ws-testnet.bitget.com/v2/ws/private`
    pub fn testnet() -> Self {
        Self {
            rest: "https://api-testnet.bitget.com".to_string(),
            ws_public: "wss://ws-testnet.bitget.com/v2/ws/public".to_string(),
            ws_private: "wss://ws-testnet.bitget.com/v2/ws/private".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bitget_creation() {
        let config = ExchangeConfig {
            id: "bitget".to_string(),
            name: "Bitget".to_string(),
            ..Default::default()
        };

        let bitget = Bitget::new(config);
        assert!(bitget.is_ok());

        let bitget = bitget.unwrap();
        assert_eq!(bitget.id(), "bitget");
        assert_eq!(bitget.name(), "Bitget");
        assert_eq!(bitget.version(), "v2");
        assert!(!bitget.certified());
        assert!(bitget.pro());
    }

    #[test]
    fn test_timeframes() {
        let config = ExchangeConfig::default();
        let bitget = Bitget::new(config).unwrap();
        let timeframes = bitget.timeframes();

        assert!(timeframes.contains_key("1m"));
        assert!(timeframes.contains_key("1h"));
        assert!(timeframes.contains_key("1d"));
        assert_eq!(timeframes.len(), 12);
    }

    #[test]
    fn test_urls() {
        let config = ExchangeConfig::default();
        let bitget = Bitget::new(config).unwrap();
        let urls = bitget.urls();

        assert!(urls.rest.contains("api.bitget.com"));
        assert!(urls.ws_public.contains("ws.bitget.com"));
    }

    #[test]
    fn test_sandbox_urls() {
        let config = ExchangeConfig {
            sandbox: true,
            ..Default::default()
        };
        let bitget = Bitget::new(config).unwrap();
        let urls = bitget.urls();

        // Sandbox mode should use testnet URLs
        assert_eq!(urls.rest, "https://api-testnet.bitget.com");
        assert_eq!(urls.ws_public, "wss://ws-testnet.bitget.com/v2/ws/public");
        assert_eq!(urls.ws_private, "wss://ws-testnet.bitget.com/v2/ws/private");
    }

    #[test]
    fn test_bitget_urls_testnet() {
        let urls = BitgetUrls::testnet();
        assert_eq!(urls.rest, "https://api-testnet.bitget.com");
        assert_eq!(urls.ws_public, "wss://ws-testnet.bitget.com/v2/ws/public");
        assert_eq!(urls.ws_private, "wss://ws-testnet.bitget.com/v2/ws/private");
    }

    #[test]
    fn test_sandbox_urls_with_testnet_option() {
        let config = ExchangeConfig::default();
        let options = BitgetOptions {
            testnet: true,
            ..Default::default()
        };
        let bitget = Bitget::new_with_options(config, options).unwrap();
        let urls = bitget.urls();

        // Testnet option should also use testnet URLs
        assert_eq!(urls.rest, "https://api-testnet.bitget.com");
        assert_eq!(urls.ws_public, "wss://ws-testnet.bitget.com/v2/ws/public");
        assert_eq!(urls.ws_private, "wss://ws-testnet.bitget.com/v2/ws/private");
    }

    #[test]
    fn test_is_sandbox_with_config_sandbox() {
        let config = ExchangeConfig {
            sandbox: true,
            ..Default::default()
        };
        let bitget = Bitget::new(config).unwrap();
        assert!(bitget.is_sandbox());
    }

    #[test]
    fn test_is_sandbox_with_options_testnet() {
        let config = ExchangeConfig::default();
        let options = BitgetOptions {
            testnet: true,
            ..Default::default()
        };
        let bitget = Bitget::new_with_options(config, options).unwrap();
        assert!(bitget.is_sandbox());
    }

    #[test]
    fn test_is_sandbox_false_by_default() {
        let config = ExchangeConfig::default();
        let bitget = Bitget::new(config).unwrap();
        assert!(!bitget.is_sandbox());
    }

    #[test]
    fn test_default_options() {
        let options = BitgetOptions::default();
        assert_eq!(options.product_type, "spot");
        assert_eq!(options.default_type, DefaultType::Spot);
        assert_eq!(options.default_sub_type, None);
        assert_eq!(options.recv_window, 5000);
        assert!(!options.testnet);
    }

    #[test]
    fn test_effective_product_type_spot() {
        let mut options = BitgetOptions::default();
        options.default_type = DefaultType::Spot;
        assert_eq!(options.effective_product_type(), "spot");
    }

    #[test]
    fn test_effective_product_type_swap_linear() {
        let mut options = BitgetOptions::default();
        options.default_type = DefaultType::Swap;
        options.default_sub_type = Some(DefaultSubType::Linear);
        assert_eq!(options.effective_product_type(), "umcbl");
    }

    #[test]
    fn test_effective_product_type_swap_inverse() {
        let mut options = BitgetOptions::default();
        options.default_type = DefaultType::Swap;
        options.default_sub_type = Some(DefaultSubType::Inverse);
        assert_eq!(options.effective_product_type(), "dmcbl");
    }

    #[test]
    fn test_effective_product_type_swap_default_sub_type() {
        let mut options = BitgetOptions::default();
        options.default_type = DefaultType::Swap;
        // No sub_type specified, should default to Linear (umcbl)
        assert_eq!(options.effective_product_type(), "umcbl");
    }

    #[test]
    fn test_effective_product_type_futures_linear() {
        let mut options = BitgetOptions::default();
        options.default_type = DefaultType::Futures;
        options.default_sub_type = Some(DefaultSubType::Linear);
        assert_eq!(options.effective_product_type(), "umcbl");
    }

    #[test]
    fn test_effective_product_type_futures_inverse() {
        let mut options = BitgetOptions::default();
        options.default_type = DefaultType::Futures;
        options.default_sub_type = Some(DefaultSubType::Inverse);
        assert_eq!(options.effective_product_type(), "dmcbl");
    }

    #[test]
    fn test_effective_product_type_margin() {
        let mut options = BitgetOptions::default();
        options.default_type = DefaultType::Margin;
        assert_eq!(options.effective_product_type(), "spot");
    }

    #[test]
    fn test_effective_product_type_option() {
        let mut options = BitgetOptions::default();
        options.default_type = DefaultType::Option;
        assert_eq!(options.effective_product_type(), "spot");
    }
}
