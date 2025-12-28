//! OKX exchange implementation.
//!
//! Supports spot trading and futures trading (USDT-M and Coin-M) with REST API and WebSocket support.
//! OKX uses V5 unified API with HMAC-SHA256 + Base64 authentication.

use ccxt_core::types::default_type::{DefaultSubType, DefaultType};
use ccxt_core::{BaseExchange, ExchangeConfig, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub mod auth;
pub mod builder;
pub mod endpoint_router;
pub mod error;
pub mod exchange_impl;
pub mod parser;
pub mod rest;
pub mod symbol;
pub mod ws;
pub mod ws_exchange_impl;

pub use auth::OkxAuth;
pub use builder::OkxBuilder;
pub use endpoint_router::{OkxChannelType, OkxEndpointRouter};
pub use error::{OkxErrorCode, is_error_response, parse_error};

/// OKX exchange structure.
#[derive(Debug)]
pub struct Okx {
    /// Base exchange instance.
    base: BaseExchange,
    /// OKX-specific options.
    options: OkxOptions,
}

/// OKX-specific options.
///
/// # Example
///
/// ```rust
/// use ccxt_exchanges::okx::OkxOptions;
/// use ccxt_core::types::default_type::{DefaultType, DefaultSubType};
///
/// let options = OkxOptions {
///     default_type: DefaultType::Swap,
///     default_sub_type: Some(DefaultSubType::Linear),
///     ..Default::default()
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OkxOptions {
    /// Account mode: cash (spot), cross (cross margin), isolated (isolated margin).
    ///
    /// This is kept for backward compatibility with existing configurations.
    pub account_mode: String,
    /// Default trading type (spot/margin/swap/futures/option).
    ///
    /// This determines which instrument type (instType) to use for API calls.
    /// OKX uses a unified V5 API, so this primarily affects market filtering
    /// rather than endpoint selection.
    #[serde(default)]
    pub default_type: DefaultType,
    /// Default sub-type for contract settlement (linear/inverse).
    ///
    /// - `Linear`: USDT-margined contracts
    /// - `Inverse`: Coin-margined contracts
    ///
    /// Only applicable when `default_type` is `Swap`, `Futures`, or `Option`.
    /// Ignored for `Spot` and `Margin` types.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_sub_type: Option<DefaultSubType>,
    /// Enables demo trading environment.
    pub testnet: bool,
}

impl Default for OkxOptions {
    fn default() -> Self {
        Self {
            account_mode: "cash".to_string(),
            default_type: DefaultType::default(), // Defaults to Spot
            default_sub_type: None,
            testnet: false,
        }
    }
}

impl Okx {
    /// Creates a new OKX instance using the builder pattern.
    ///
    /// This is the recommended way to create an OKX instance.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ccxt_exchanges::okx::Okx;
    ///
    /// let okx = Okx::builder()
    ///     .api_key("your-api-key")
    ///     .secret("your-secret")
    ///     .passphrase("your-passphrase")
    ///     .sandbox(true)
    ///     .build()
    ///     .unwrap();
    /// ```
    pub fn builder() -> OkxBuilder {
        OkxBuilder::new()
    }

    /// Creates a new OKX instance.
    ///
    /// # Arguments
    ///
    /// * `config` - Exchange configuration.
    pub fn new(config: ExchangeConfig) -> Result<Self> {
        let base = BaseExchange::new(config)?;
        let options = OkxOptions::default();

        Ok(Self { base, options })
    }

    /// Creates a new OKX instance with custom options.
    ///
    /// This is used internally by the builder pattern.
    ///
    /// # Arguments
    ///
    /// * `config` - Exchange configuration.
    /// * `options` - OKX-specific options.
    pub fn new_with_options(config: ExchangeConfig, options: OkxOptions) -> Result<Self> {
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

    /// Returns the OKX options.
    pub fn options(&self) -> &OkxOptions {
        &self.options
    }

    /// Sets the OKX options.
    pub fn set_options(&mut self, options: OkxOptions) {
        self.options = options;
    }

    /// Returns the exchange ID.
    pub fn id(&self) -> &str {
        "okx"
    }

    /// Returns the exchange name.
    pub fn name(&self) -> &str {
        "OKX"
    }

    /// Returns the API version.
    pub fn version(&self) -> &str {
        "v5"
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

    /// Returns `true` if sandbox/demo mode is enabled.
    ///
    /// Sandbox mode is enabled when either:
    /// - `config.sandbox` is set to `true`
    /// - `options.demo` is set to `true`
    ///
    /// # Returns
    ///
    /// `true` if sandbox mode is enabled, `false` otherwise.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ccxt_exchanges::okx::Okx;
    /// use ccxt_core::ExchangeConfig;
    ///
    /// let config = ExchangeConfig {
    ///     sandbox: true,
    ///     ..Default::default()
    /// };
    /// let okx = Okx::new(config).unwrap();
    /// assert!(okx.is_sandbox());
    /// ```
    pub fn is_sandbox(&self) -> bool {
        self.base().config.sandbox || self.options.testnet
    }

    /// Returns `true` if demo trading mode is enabled.
    ///
    /// This is an OKX-specific alias for `is_sandbox()`. Demo trading mode
    /// is enabled when either:
    /// - `config.sandbox` is set to `true`
    /// - `options.demo` is set to `true`
    ///
    /// When demo trading is enabled, the client will:
    /// - Add the `x-simulated-trading: 1` header to all REST API requests
    /// - Use demo WebSocket URLs (`wss://wspap.okx.com:8443/ws/v5/*?brokerId=9999`)
    /// - Continue using the production REST domain (`https://www.okx.com`)
    ///
    /// # Returns
    ///
    /// `true` if demo trading mode is enabled, `false` otherwise.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ccxt_exchanges::okx::Okx;
    /// use ccxt_core::ExchangeConfig;
    ///
    /// let config = ExchangeConfig {
    ///     sandbox: true,
    ///     ..Default::default()
    /// };
    /// let okx = Okx::new(config).unwrap();
    /// assert!(okx.is_testnet_trading());
    /// ```
    pub fn is_testnet_trading(&self) -> bool {
        self.base().config.sandbox || self.options.testnet
    }

    /// Returns the supported timeframes.
    pub fn timeframes(&self) -> HashMap<String, String> {
        let mut timeframes = HashMap::new();
        timeframes.insert("1m".to_string(), "1m".to_string());
        timeframes.insert("3m".to_string(), "3m".to_string());
        timeframes.insert("5m".to_string(), "5m".to_string());
        timeframes.insert("15m".to_string(), "15m".to_string());
        timeframes.insert("30m".to_string(), "30m".to_string());
        timeframes.insert("1h".to_string(), "1H".to_string());
        timeframes.insert("2h".to_string(), "2H".to_string());
        timeframes.insert("4h".to_string(), "4H".to_string());
        timeframes.insert("6h".to_string(), "6Hutc".to_string());
        timeframes.insert("12h".to_string(), "12Hutc".to_string());
        timeframes.insert("1d".to_string(), "1Dutc".to_string());
        timeframes.insert("1w".to_string(), "1Wutc".to_string());
        timeframes.insert("1M".to_string(), "1Mutc".to_string());
        timeframes
    }

    /// Returns the API URLs.
    pub fn urls(&self) -> OkxUrls {
        if self.base().config.sandbox || self.options.testnet {
            OkxUrls::demo()
        } else {
            OkxUrls::production()
        }
    }

    /// Returns the default type configuration.
    pub fn default_type(&self) -> DefaultType {
        self.options.default_type
    }

    /// Returns the default sub-type configuration.
    pub fn default_sub_type(&self) -> Option<DefaultSubType> {
        self.options.default_sub_type
    }

    /// Checks if the current default_type is a contract type (Swap, Futures, or Option).
    ///
    /// This is useful for determining whether contract-specific API parameters should be used.
    ///
    /// # Returns
    ///
    /// `true` if the default_type is Swap, Futures, or Option; `false` otherwise.
    pub fn is_contract_type(&self) -> bool {
        self.options.default_type.is_contract()
    }

    /// Checks if the current configuration uses inverse (coin-margined) contracts.
    ///
    /// # Returns
    ///
    /// `true` if default_sub_type is Inverse; `false` otherwise.
    pub fn is_inverse(&self) -> bool {
        matches!(self.options.default_sub_type, Some(DefaultSubType::Inverse))
    }

    /// Checks if the current configuration uses linear (USDT-margined) contracts.
    ///
    /// # Returns
    ///
    /// `true` if default_sub_type is Linear or not specified (defaults to Linear); `false` otherwise.
    pub fn is_linear(&self) -> bool {
        !self.is_inverse()
    }

    /// Creates a new WebSocket client for OKX.
    ///
    /// Returns an `OkxWs` instance configured with the appropriate WebSocket URL
    /// based on the exchange configuration (production or demo).
    ///
    /// # Example
    /// ```no_run
    /// use ccxt_exchanges::okx::Okx;
    /// use ccxt_core::ExchangeConfig;
    ///
    /// # async fn example() -> ccxt_core::error::Result<()> {
    /// let okx = Okx::new(ExchangeConfig::default())?;
    /// let ws = okx.create_ws();
    /// ws.connect().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn create_ws(&self) -> ws::OkxWs {
        let urls = self.urls();
        ws::OkxWs::new(urls.ws_public)
    }
}

/// OKX API URLs.
#[derive(Debug, Clone)]
pub struct OkxUrls {
    /// REST API base URL.
    pub rest: String,
    /// Public WebSocket URL.
    pub ws_public: String,
    /// Private WebSocket URL.
    pub ws_private: String,
    /// Business WebSocket URL.
    pub ws_business: String,
}

impl OkxUrls {
    /// Returns production environment URLs.
    pub fn production() -> Self {
        Self {
            rest: "https://www.okx.com".to_string(),
            ws_public: "wss://ws.okx.com:8443/ws/v5/public".to_string(),
            ws_private: "wss://ws.okx.com:8443/ws/v5/private".to_string(),
            ws_business: "wss://ws.okx.com:8443/ws/v5/business".to_string(),
        }
    }

    /// Returns demo environment URLs.
    pub fn demo() -> Self {
        Self {
            rest: "https://www.okx.com".to_string(),
            ws_public: "wss://wspap.okx.com:8443/ws/v5/public?brokerId=9999".to_string(),
            ws_private: "wss://wspap.okx.com:8443/ws/v5/private?brokerId=9999".to_string(),
            ws_business: "wss://wspap.okx.com:8443/ws/v5/business?brokerId=9999".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_okx_creation() {
        let config = ExchangeConfig {
            id: "okx".to_string(),
            name: "OKX".to_string(),
            ..Default::default()
        };

        let okx = Okx::new(config);
        assert!(okx.is_ok());

        let okx = okx.unwrap();
        assert_eq!(okx.id(), "okx");
        assert_eq!(okx.name(), "OKX");
        assert_eq!(okx.version(), "v5");
        assert!(!okx.certified());
        assert!(okx.pro());
    }

    #[test]
    fn test_timeframes() {
        let config = ExchangeConfig::default();
        let okx = Okx::new(config).unwrap();
        let timeframes = okx.timeframes();

        assert!(timeframes.contains_key("1m"));
        assert!(timeframes.contains_key("1h"));
        assert!(timeframes.contains_key("1d"));
        assert_eq!(timeframes.len(), 13);
    }

    #[test]
    fn test_urls() {
        let config = ExchangeConfig::default();
        let okx = Okx::new(config).unwrap();
        let urls = okx.urls();

        assert!(urls.rest.contains("okx.com"));
        assert!(urls.ws_public.contains("ws.okx.com"));
    }

    #[test]
    fn test_sandbox_urls() {
        let config = ExchangeConfig {
            sandbox: true,
            ..Default::default()
        };
        let okx = Okx::new(config).unwrap();
        let urls = okx.urls();

        assert!(urls.ws_public.contains("wspap.okx.com"));
        assert!(urls.ws_public.contains("brokerId=9999"));
    }

    #[test]
    fn test_demo_rest_url_uses_production_domain() {
        // Verify that OKX demo mode uses the production REST domain
        // OKX uses the same REST domain for demo trading, but adds a special header
        let demo_urls = OkxUrls::demo();
        let production_urls = OkxUrls::production();

        // REST URL should be the same (production domain)
        assert_eq!(demo_urls.rest, production_urls.rest);
        assert_eq!(demo_urls.rest, "https://www.okx.com");

        // WebSocket URLs should be different (demo uses wspap.okx.com)
        assert_ne!(demo_urls.ws_public, production_urls.ws_public);
        assert!(demo_urls.ws_public.contains("wspap.okx.com"));
        assert!(demo_urls.ws_public.contains("brokerId=9999"));
    }

    #[test]
    fn test_sandbox_mode_rest_url_is_production() {
        // When sandbox mode is enabled, REST URL should still be production domain
        let config = ExchangeConfig {
            sandbox: true,
            ..Default::default()
        };
        let okx = Okx::new(config).unwrap();
        let urls = okx.urls();

        // REST URL should be production domain
        assert_eq!(urls.rest, "https://www.okx.com");
    }

    #[test]
    fn test_is_sandbox_with_config_sandbox() {
        let config = ExchangeConfig {
            sandbox: true,
            ..Default::default()
        };
        let okx = Okx::new(config).unwrap();
        assert!(okx.is_sandbox());
    }

    #[test]
    fn test_is_sandbox_with_options_demo() {
        let config = ExchangeConfig::default();
        let options = OkxOptions {
            testnet: true,
            ..Default::default()
        };
        let okx = Okx::new_with_options(config, options).unwrap();
        assert!(okx.is_sandbox());
    }

    #[test]
    fn test_is_sandbox_false_by_default() {
        let config = ExchangeConfig::default();
        let okx = Okx::new(config).unwrap();
        assert!(!okx.is_sandbox());
    }

    #[test]
    fn test_is_demo_trading_with_config_sandbox() {
        let config = ExchangeConfig {
            sandbox: true,
            ..Default::default()
        };
        let okx = Okx::new(config).unwrap();
        assert!(okx.is_testnet_trading());
    }

    #[test]
    fn test_is_demo_trading_with_options_demo() {
        let config = ExchangeConfig::default();
        let options = OkxOptions {
            testnet: true,
            ..Default::default()
        };
        let okx = Okx::new_with_options(config, options).unwrap();
        assert!(okx.is_testnet_trading());
    }

    #[test]
    fn test_is_demo_trading_false_by_default() {
        let config = ExchangeConfig::default();
        let okx = Okx::new(config).unwrap();
        assert!(!okx.is_testnet_trading());
    }

    #[test]
    fn test_is_demo_trading_equals_is_sandbox() {
        // Test that is_demo_trading() and is_sandbox() return the same value
        let config = ExchangeConfig::default();
        let okx = Okx::new(config).unwrap();
        assert_eq!(okx.is_testnet_trading(), okx.is_sandbox());

        let config_sandbox = ExchangeConfig {
            sandbox: true,
            ..Default::default()
        };
        let okx_sandbox = Okx::new(config_sandbox).unwrap();
        assert_eq!(okx_sandbox.is_testnet_trading(), okx_sandbox.is_sandbox());
    }

    #[test]
    fn test_default_options() {
        let options = OkxOptions::default();
        assert_eq!(options.account_mode, "cash");
        assert_eq!(options.default_type, DefaultType::Spot);
        assert_eq!(options.default_sub_type, None);
        assert!(!options.testnet);
    }

    #[test]
    fn test_okx_options_with_default_type() {
        let options = OkxOptions {
            default_type: DefaultType::Swap,
            default_sub_type: Some(DefaultSubType::Linear),
            ..Default::default()
        };
        assert_eq!(options.default_type, DefaultType::Swap);
        assert_eq!(options.default_sub_type, Some(DefaultSubType::Linear));
    }

    #[test]
    fn test_okx_options_serialization() {
        let options = OkxOptions {
            default_type: DefaultType::Swap,
            default_sub_type: Some(DefaultSubType::Linear),
            ..Default::default()
        };
        let json = serde_json::to_string(&options).unwrap();
        assert!(json.contains("\"default_type\":\"swap\""));
        assert!(json.contains("\"default_sub_type\":\"linear\""));
    }

    #[test]
    fn test_okx_options_deserialization() {
        let json = r#"{
            "account_mode": "cross",
            "default_type": "swap",
            "default_sub_type": "inverse",
            "testnet": true
        }"#;
        let options: OkxOptions = serde_json::from_str(json).unwrap();
        assert_eq!(options.account_mode, "cross");
        assert_eq!(options.default_type, DefaultType::Swap);
        assert_eq!(options.default_sub_type, Some(DefaultSubType::Inverse));
        assert!(options.testnet);
    }

    #[test]
    fn test_okx_options_deserialization_without_default_type() {
        // Test backward compatibility - default_type should default to Spot
        let json = r#"{
            "account_mode": "cash",
            "testnet": false
        }"#;
        let options: OkxOptions = serde_json::from_str(json).unwrap();
        assert_eq!(options.default_type, DefaultType::Spot);
        assert_eq!(options.default_sub_type, None);
    }
}
