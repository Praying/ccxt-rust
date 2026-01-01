//! Bybit exchange implementation.
//!
//! Supports spot trading and futures trading (USDT-M and Coin-M) with REST API and WebSocket support.
//! Bybit uses V5 unified account API with HMAC-SHA256 authentication.

use ccxt_core::types::default_type::{DefaultSubType, DefaultType};
use ccxt_core::{BaseExchange, ExchangeConfig, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub mod auth;
pub mod builder;
pub mod endpoint_router;
pub mod error;
pub mod parser;
pub mod rest;
pub mod signed_request;
pub mod symbol;
pub mod ws;
mod ws_exchange_impl;

pub use auth::BybitAuth;
pub use builder::BybitBuilder;
pub use endpoint_router::BybitEndpointRouter;
pub use error::{BybitErrorCode, is_error_response, parse_error};

/// Bybit exchange structure.
#[derive(Debug)]
pub struct Bybit {
    /// Base exchange instance.
    base: BaseExchange,
    /// Bybit-specific options.
    options: BybitOptions,
}

/// Bybit-specific options.
///
/// # Example
///
/// ```rust
/// use ccxt_exchanges::bybit::BybitOptions;
/// use ccxt_core::types::default_type::{DefaultType, DefaultSubType};
///
/// let options = BybitOptions {
///     default_type: DefaultType::Swap,
///     default_sub_type: Some(DefaultSubType::Linear),
///     ..Default::default()
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BybitOptions {
    /// Account type: UNIFIED, CONTRACT, SPOT.
    ///
    /// This is kept for backward compatibility with existing configurations.
    pub account_type: String,
    /// Default trading type (spot/swap/futures/option).
    ///
    /// This determines which category to use for API calls.
    /// Bybit uses a unified V5 API with category-based filtering:
    /// - `Spot` -> category=spot
    /// - `Swap` + Linear -> category=linear
    /// - `Swap` + Inverse -> category=inverse
    /// - `Option` -> category=option
    #[serde(default)]
    pub default_type: DefaultType,
    /// Default sub-type for contract settlement (linear/inverse).
    ///
    /// - `Linear`: USDT-margined contracts (category=linear)
    /// - `Inverse`: Coin-margined contracts (category=inverse)
    ///
    /// Only applicable when `default_type` is `Swap` or `Futures`.
    /// Ignored for `Spot` and `Option` types.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_sub_type: Option<DefaultSubType>,
    /// Enables testnet environment.
    pub testnet: bool,
    /// Receive window in milliseconds.
    pub recv_window: u64,
}

impl Default for BybitOptions {
    fn default() -> Self {
        Self {
            account_type: "UNIFIED".to_string(),
            default_type: DefaultType::default(), // Defaults to Spot
            default_sub_type: None,
            testnet: false,
            recv_window: 5000,
        }
    }
}

impl Bybit {
    /// Creates a new Bybit instance using the builder pattern.
    ///
    /// This is the recommended way to create a Bybit instance.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ccxt_exchanges::bybit::Bybit;
    ///
    /// let bybit = Bybit::builder()
    ///     .api_key("your-api-key")
    ///     .secret("your-secret")
    ///     .testnet(true)
    ///     .build()
    ///     .unwrap();
    /// ```
    pub fn builder() -> BybitBuilder {
        BybitBuilder::new()
    }

    /// Creates a new Bybit instance.
    ///
    /// # Arguments
    ///
    /// * `config` - Exchange configuration.
    pub fn new(config: ExchangeConfig) -> Result<Self> {
        let base = BaseExchange::new(config)?;
        let options = BybitOptions::default();

        Ok(Self { base, options })
    }

    /// Creates a new Bybit instance with custom options.
    ///
    /// This is used internally by the builder pattern.
    ///
    /// # Arguments
    ///
    /// * `config` - Exchange configuration.
    /// * `options` - Bybit-specific options.
    pub fn new_with_options(config: ExchangeConfig, options: BybitOptions) -> Result<Self> {
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

    /// Returns the Bybit options.
    pub fn options(&self) -> &BybitOptions {
        &self.options
    }

    /// Sets the Bybit options.
    pub fn set_options(&mut self, options: BybitOptions) {
        self.options = options;
    }

    /// Returns the exchange ID.
    pub fn id(&self) -> &str {
        "bybit"
    }

    /// Returns the exchange name.
    pub fn name(&self) -> &str {
        "Bybit"
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
    /// use ccxt_exchanges::bybit::Bybit;
    /// use ccxt_core::ExchangeConfig;
    ///
    /// let config = ExchangeConfig {
    ///     sandbox: true,
    ///     ..Default::default()
    /// };
    /// let bybit = Bybit::new(config).unwrap();
    /// assert!(bybit.is_sandbox());
    /// ```
    pub fn is_sandbox(&self) -> bool {
        self.base().config.sandbox || self.options.testnet
    }

    /// Returns the supported timeframes.
    pub fn timeframes(&self) -> HashMap<String, String> {
        let mut timeframes = HashMap::new();
        timeframes.insert("1m".to_string(), "1".to_string());
        timeframes.insert("3m".to_string(), "3".to_string());
        timeframes.insert("5m".to_string(), "5".to_string());
        timeframes.insert("15m".to_string(), "15".to_string());
        timeframes.insert("30m".to_string(), "30".to_string());
        timeframes.insert("1h".to_string(), "60".to_string());
        timeframes.insert("2h".to_string(), "120".to_string());
        timeframes.insert("4h".to_string(), "240".to_string());
        timeframes.insert("6h".to_string(), "360".to_string());
        timeframes.insert("12h".to_string(), "720".to_string());
        timeframes.insert("1d".to_string(), "D".to_string());
        timeframes.insert("1w".to_string(), "W".to_string());
        timeframes.insert("1M".to_string(), "M".to_string());
        timeframes
    }

    /// Returns the API URLs.
    pub fn urls(&self) -> BybitUrls {
        if self.base().config.sandbox || self.options.testnet {
            BybitUrls::testnet()
        } else {
            BybitUrls::production()
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

    /// Returns the Bybit category string based on the current default_type and default_sub_type.
    ///
    /// Bybit V5 API uses category parameter for filtering:
    /// - `Spot` -> "spot"
    /// - `Swap` + Linear -> "linear"
    /// - `Swap` + Inverse -> "inverse"
    /// - `Futures` + Linear -> "linear"
    /// - `Futures` + Inverse -> "inverse"
    /// - `Option` -> "option"
    /// - `Margin` -> "spot" (margin trading uses spot category)
    ///
    /// # Returns
    ///
    /// The category string to use for Bybit API calls.
    pub fn category(&self) -> &'static str {
        match self.options.default_type {
            DefaultType::Spot | DefaultType::Margin => "spot",
            DefaultType::Swap | DefaultType::Futures => {
                if self.is_inverse() {
                    "inverse"
                } else {
                    "linear"
                }
            }
            DefaultType::Option => "option",
        }
    }

    /// Creates a public WebSocket client.
    ///
    /// The WebSocket URL is selected based on the configured `default_type`:
    /// - `Spot` -> spot public stream
    /// - `Swap`/`Futures` + Linear -> linear public stream
    /// - `Swap`/`Futures` + Inverse -> inverse public stream
    /// - `Option` -> option public stream
    ///
    /// # Returns
    ///
    /// Returns a `BybitWs` instance for public data streams.
    ///
    /// # Example
    /// ```no_run
    /// use ccxt_exchanges::bybit::Bybit;
    /// use ccxt_core::ExchangeConfig;
    ///
    /// # async fn example() -> ccxt_core::error::Result<()> {
    /// let bybit = Bybit::new(ExchangeConfig::default())?;
    /// let ws = bybit.create_ws();
    /// ws.connect().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn create_ws(&self) -> ws::BybitWs {
        let urls = self.urls();
        let category = self.category();
        let ws_url = urls.ws_public_for_category(category);
        ws::BybitWs::new(ws_url)
    }

    /// Creates a signed request builder for authenticated API calls.
    ///
    /// This method provides a fluent API for constructing authenticated requests
    /// to Bybit's private endpoints. The builder handles:
    /// - Credential validation
    /// - Millisecond timestamp generation
    /// - HMAC-SHA256 signature generation (hex encoded)
    /// - Authentication header injection (X-BAPI-* headers)
    ///
    /// # Arguments
    ///
    /// * `endpoint` - API endpoint path (e.g., "/v5/account/wallet-balance")
    ///
    /// # Returns
    ///
    /// Returns a `BybitSignedRequestBuilder` for method chaining.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ccxt_exchanges::bybit::Bybit;
    /// use ccxt_exchanges::bybit::signed_request::HttpMethod;
    /// use ccxt_core::ExchangeConfig;
    ///
    /// # async fn example() -> ccxt_core::Result<()> {
    /// let bybit = Bybit::builder()
    ///     .api_key("your-api-key")
    ///     .secret("your-secret")
    ///     .build()?;
    ///
    /// // GET request
    /// let balance = bybit.signed_request("/v5/account/wallet-balance")
    ///     .param("accountType", "UNIFIED")
    ///     .execute()
    ///     .await?;
    ///
    /// // POST request
    /// let order = bybit.signed_request("/v5/order/create")
    ///     .method(HttpMethod::Post)
    ///     .param("category", "spot")
    ///     .param("symbol", "BTCUSDT")
    ///     .param("side", "Buy")
    ///     .param("orderType", "Limit")
    ///     .param("qty", "0.001")
    ///     .param("price", "50000")
    ///     .execute()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn signed_request(
        &self,
        endpoint: impl Into<String>,
    ) -> signed_request::BybitSignedRequestBuilder<'_> {
        signed_request::BybitSignedRequestBuilder::new(self, endpoint)
    }
}

/// Bybit API URLs.
#[derive(Debug, Clone)]
pub struct BybitUrls {
    /// REST API base URL.
    pub rest: String,
    /// Public WebSocket base URL (without category suffix).
    pub ws_public_base: String,
    /// Public WebSocket URL (default: spot).
    pub ws_public: String,
    /// Private WebSocket URL.
    pub ws_private: String,
}

impl BybitUrls {
    /// Returns production environment URLs.
    pub fn production() -> Self {
        Self {
            rest: "https://api.bybit.com".to_string(),
            ws_public_base: "wss://stream.bybit.com/v5/public".to_string(),
            ws_public: "wss://stream.bybit.com/v5/public/spot".to_string(),
            ws_private: "wss://stream.bybit.com/v5/private".to_string(),
        }
    }

    /// Returns testnet environment URLs.
    pub fn testnet() -> Self {
        Self {
            rest: "https://api-testnet.bybit.com".to_string(),
            ws_public_base: "wss://stream-testnet.bybit.com/v5/public".to_string(),
            ws_public: "wss://stream-testnet.bybit.com/v5/public/spot".to_string(),
            ws_private: "wss://stream-testnet.bybit.com/v5/private".to_string(),
        }
    }

    /// Returns the public WebSocket URL for a specific category.
    ///
    /// Bybit V5 API uses different WebSocket endpoints for different categories:
    /// - spot: /v5/public/spot
    /// - linear: /v5/public/linear
    /// - inverse: /v5/public/inverse
    /// - option: /v5/public/option
    ///
    /// # Arguments
    ///
    /// * `category` - The category string (spot, linear, inverse, option)
    ///
    /// # Returns
    ///
    /// The full WebSocket URL for the specified category.
    pub fn ws_public_for_category(&self, category: &str) -> String {
        format!("{}/{}", self.ws_public_base, category)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bybit_creation() {
        let config = ExchangeConfig {
            id: "bybit".to_string(),
            name: "Bybit".to_string(),
            ..Default::default()
        };

        let bybit = Bybit::new(config);
        assert!(bybit.is_ok());

        let bybit = bybit.unwrap();
        assert_eq!(bybit.id(), "bybit");
        assert_eq!(bybit.name(), "Bybit");
        assert_eq!(bybit.version(), "v5");
        assert!(!bybit.certified());
        assert!(bybit.pro());
    }

    #[test]
    fn test_timeframes() {
        let config = ExchangeConfig::default();
        let bybit = Bybit::new(config).unwrap();
        let timeframes = bybit.timeframes();

        assert!(timeframes.contains_key("1m"));
        assert!(timeframes.contains_key("1h"));
        assert!(timeframes.contains_key("1d"));
        assert_eq!(timeframes.len(), 13);
    }

    #[test]
    fn test_urls() {
        let config = ExchangeConfig::default();
        let bybit = Bybit::new(config).unwrap();
        let urls = bybit.urls();

        assert!(urls.rest.contains("api.bybit.com"));
        assert!(urls.ws_public.contains("stream.bybit.com"));
        assert!(urls.ws_public_base.contains("stream.bybit.com"));
    }

    #[test]
    fn test_testnet_urls() {
        let config = ExchangeConfig {
            sandbox: true,
            ..Default::default()
        };
        let bybit = Bybit::new(config).unwrap();
        let urls = bybit.urls();

        assert!(urls.rest.contains("api-testnet.bybit.com"));
        assert!(urls.ws_public.contains("stream-testnet.bybit.com"));
        assert!(urls.ws_public_base.contains("stream-testnet.bybit.com"));
    }

    #[test]
    fn test_is_sandbox_with_config_sandbox() {
        let config = ExchangeConfig {
            sandbox: true,
            ..Default::default()
        };
        let bybit = Bybit::new(config).unwrap();
        assert!(bybit.is_sandbox());
    }

    #[test]
    fn test_is_sandbox_with_options_testnet() {
        let config = ExchangeConfig::default();
        let options = BybitOptions {
            testnet: true,
            ..Default::default()
        };
        let bybit = Bybit::new_with_options(config, options).unwrap();
        assert!(bybit.is_sandbox());
    }

    #[test]
    fn test_is_sandbox_false_by_default() {
        let config = ExchangeConfig::default();
        let bybit = Bybit::new(config).unwrap();
        assert!(!bybit.is_sandbox());
    }

    #[test]
    fn test_ws_public_for_category() {
        let urls = BybitUrls::production();

        assert_eq!(
            urls.ws_public_for_category("spot"),
            "wss://stream.bybit.com/v5/public/spot"
        );
        assert_eq!(
            urls.ws_public_for_category("linear"),
            "wss://stream.bybit.com/v5/public/linear"
        );
        assert_eq!(
            urls.ws_public_for_category("inverse"),
            "wss://stream.bybit.com/v5/public/inverse"
        );
        assert_eq!(
            urls.ws_public_for_category("option"),
            "wss://stream.bybit.com/v5/public/option"
        );
    }

    #[test]
    fn test_ws_public_for_category_testnet() {
        let urls = BybitUrls::testnet();

        assert_eq!(
            urls.ws_public_for_category("spot"),
            "wss://stream-testnet.bybit.com/v5/public/spot"
        );
        assert_eq!(
            urls.ws_public_for_category("linear"),
            "wss://stream-testnet.bybit.com/v5/public/linear"
        );
    }

    #[test]
    fn test_default_options() {
        let options = BybitOptions::default();
        assert_eq!(options.account_type, "UNIFIED");
        assert_eq!(options.default_type, DefaultType::Spot);
        assert_eq!(options.default_sub_type, None);
        assert!(!options.testnet);
        assert_eq!(options.recv_window, 5000);
    }

    #[test]
    fn test_bybit_options_with_default_type() {
        let options = BybitOptions {
            default_type: DefaultType::Swap,
            default_sub_type: Some(DefaultSubType::Linear),
            ..Default::default()
        };
        assert_eq!(options.default_type, DefaultType::Swap);
        assert_eq!(options.default_sub_type, Some(DefaultSubType::Linear));
    }

    #[test]
    fn test_bybit_options_serialization() {
        let options = BybitOptions {
            default_type: DefaultType::Swap,
            default_sub_type: Some(DefaultSubType::Linear),
            ..Default::default()
        };
        let json = serde_json::to_string(&options).unwrap();
        assert!(json.contains("\"default_type\":\"swap\""));
        assert!(json.contains("\"default_sub_type\":\"linear\""));
    }

    #[test]
    fn test_bybit_options_deserialization() {
        let json = r#"{
            "account_type": "CONTRACT",
            "default_type": "swap",
            "default_sub_type": "inverse",
            "testnet": true,
            "recv_window": 10000
        }"#;
        let options: BybitOptions = serde_json::from_str(json).unwrap();
        assert_eq!(options.account_type, "CONTRACT");
        assert_eq!(options.default_type, DefaultType::Swap);
        assert_eq!(options.default_sub_type, Some(DefaultSubType::Inverse));
        assert!(options.testnet);
        assert_eq!(options.recv_window, 10000);
    }

    #[test]
    fn test_bybit_options_deserialization_without_default_type() {
        // Test backward compatibility - default_type should default to Spot
        let json = r#"{
            "account_type": "UNIFIED",
            "testnet": false,
            "recv_window": 5000
        }"#;
        let options: BybitOptions = serde_json::from_str(json).unwrap();
        assert_eq!(options.default_type, DefaultType::Spot);
        assert_eq!(options.default_sub_type, None);
    }

    #[test]
    fn test_bybit_category_spot() {
        let config = ExchangeConfig::default();
        let options = BybitOptions {
            default_type: DefaultType::Spot,
            ..Default::default()
        };
        let bybit = Bybit::new_with_options(config, options).unwrap();
        assert_eq!(bybit.category(), "spot");
    }

    #[test]
    fn test_bybit_category_linear() {
        let config = ExchangeConfig::default();
        let options = BybitOptions {
            default_type: DefaultType::Swap,
            default_sub_type: Some(DefaultSubType::Linear),
            ..Default::default()
        };
        let bybit = Bybit::new_with_options(config, options).unwrap();
        assert_eq!(bybit.category(), "linear");
    }

    #[test]
    fn test_bybit_category_inverse() {
        let config = ExchangeConfig::default();
        let options = BybitOptions {
            default_type: DefaultType::Swap,
            default_sub_type: Some(DefaultSubType::Inverse),
            ..Default::default()
        };
        let bybit = Bybit::new_with_options(config, options).unwrap();
        assert_eq!(bybit.category(), "inverse");
    }

    #[test]
    fn test_bybit_category_option() {
        let config = ExchangeConfig::default();
        let options = BybitOptions {
            default_type: DefaultType::Option,
            ..Default::default()
        };
        let bybit = Bybit::new_with_options(config, options).unwrap();
        assert_eq!(bybit.category(), "option");
    }

    #[test]
    fn test_bybit_is_contract_type() {
        let config = ExchangeConfig::default();

        // Spot is not a contract type
        let options = BybitOptions {
            default_type: DefaultType::Spot,
            ..Default::default()
        };
        let bybit = Bybit::new_with_options(config.clone(), options).unwrap();
        assert!(!bybit.is_contract_type());

        // Swap is a contract type
        let options = BybitOptions {
            default_type: DefaultType::Swap,
            ..Default::default()
        };
        let bybit = Bybit::new_with_options(config.clone(), options).unwrap();
        assert!(bybit.is_contract_type());

        // Futures is a contract type
        let options = BybitOptions {
            default_type: DefaultType::Futures,
            ..Default::default()
        };
        let bybit = Bybit::new_with_options(config.clone(), options).unwrap();
        assert!(bybit.is_contract_type());

        // Option is a contract type
        let options = BybitOptions {
            default_type: DefaultType::Option,
            ..Default::default()
        };
        let bybit = Bybit::new_with_options(config, options).unwrap();
        assert!(bybit.is_contract_type());
    }

    // ============================================================
    // Sandbox Mode Market Type URL Selection Tests
    // ============================================================

    #[test]
    fn test_sandbox_market_type_spot() {
        // Sandbox mode with Spot type should use spot testnet endpoints
        let config = ExchangeConfig {
            sandbox: true,
            ..Default::default()
        };
        let options = BybitOptions {
            default_type: DefaultType::Spot,
            ..Default::default()
        };
        let bybit = Bybit::new_with_options(config, options).unwrap();

        assert!(bybit.is_sandbox());
        assert_eq!(bybit.category(), "spot");

        let urls = bybit.urls();
        assert!(
            urls.rest.contains("api-testnet.bybit.com"),
            "Spot sandbox REST URL should contain api-testnet.bybit.com, got: {}",
            urls.rest
        );

        let ws_url = urls.ws_public_for_category("spot");
        assert!(
            ws_url.contains("stream-testnet.bybit.com"),
            "Spot sandbox WS URL should contain stream-testnet.bybit.com, got: {}",
            ws_url
        );
        assert!(
            ws_url.contains("/v5/public/spot"),
            "Spot sandbox WS URL should contain /v5/public/spot, got: {}",
            ws_url
        );
    }

    #[test]
    fn test_sandbox_market_type_linear() {
        // Sandbox mode with Swap/Linear should use linear testnet endpoints
        let config = ExchangeConfig {
            sandbox: true,
            ..Default::default()
        };
        let options = BybitOptions {
            default_type: DefaultType::Swap,
            default_sub_type: Some(DefaultSubType::Linear),
            ..Default::default()
        };
        let bybit = Bybit::new_with_options(config, options).unwrap();

        assert!(bybit.is_sandbox());
        assert_eq!(bybit.category(), "linear");

        let urls = bybit.urls();
        assert!(
            urls.rest.contains("api-testnet.bybit.com"),
            "Linear sandbox REST URL should contain api-testnet.bybit.com, got: {}",
            urls.rest
        );

        let ws_url = urls.ws_public_for_category("linear");
        assert!(
            ws_url.contains("stream-testnet.bybit.com"),
            "Linear sandbox WS URL should contain stream-testnet.bybit.com, got: {}",
            ws_url
        );
        assert!(
            ws_url.contains("/v5/public/linear"),
            "Linear sandbox WS URL should contain /v5/public/linear, got: {}",
            ws_url
        );
    }

    #[test]
    fn test_sandbox_market_type_inverse() {
        // Sandbox mode with Swap/Inverse should use inverse testnet endpoints
        let config = ExchangeConfig {
            sandbox: true,
            ..Default::default()
        };
        let options = BybitOptions {
            default_type: DefaultType::Swap,
            default_sub_type: Some(DefaultSubType::Inverse),
            ..Default::default()
        };
        let bybit = Bybit::new_with_options(config, options).unwrap();

        assert!(bybit.is_sandbox());
        assert_eq!(bybit.category(), "inverse");

        let urls = bybit.urls();
        assert!(
            urls.rest.contains("api-testnet.bybit.com"),
            "Inverse sandbox REST URL should contain api-testnet.bybit.com, got: {}",
            urls.rest
        );

        let ws_url = urls.ws_public_for_category("inverse");
        assert!(
            ws_url.contains("stream-testnet.bybit.com"),
            "Inverse sandbox WS URL should contain stream-testnet.bybit.com, got: {}",
            ws_url
        );
        assert!(
            ws_url.contains("/v5/public/inverse"),
            "Inverse sandbox WS URL should contain /v5/public/inverse, got: {}",
            ws_url
        );
    }

    #[test]
    fn test_sandbox_market_type_option() {
        // Sandbox mode with Option type should use option testnet endpoints
        let config = ExchangeConfig {
            sandbox: true,
            ..Default::default()
        };
        let options = BybitOptions {
            default_type: DefaultType::Option,
            ..Default::default()
        };
        let bybit = Bybit::new_with_options(config, options).unwrap();

        assert!(bybit.is_sandbox());
        assert_eq!(bybit.category(), "option");

        let urls = bybit.urls();
        assert!(
            urls.rest.contains("api-testnet.bybit.com"),
            "Option sandbox REST URL should contain api-testnet.bybit.com, got: {}",
            urls.rest
        );

        let ws_url = urls.ws_public_for_category("option");
        assert!(
            ws_url.contains("stream-testnet.bybit.com"),
            "Option sandbox WS URL should contain stream-testnet.bybit.com, got: {}",
            ws_url
        );
        assert!(
            ws_url.contains("/v5/public/option"),
            "Option sandbox WS URL should contain /v5/public/option, got: {}",
            ws_url
        );
    }

    #[test]
    fn test_sandbox_private_websocket_url() {
        // Verify private WebSocket URL in sandbox mode
        let config = ExchangeConfig {
            sandbox: true,
            ..Default::default()
        };
        let bybit = Bybit::new(config).unwrap();

        assert!(bybit.is_sandbox());
        let urls = bybit.urls();
        assert!(
            urls.ws_private.contains("stream-testnet.bybit.com"),
            "Private WS sandbox URL should contain stream-testnet.bybit.com, got: {}",
            urls.ws_private
        );
        assert!(
            urls.ws_private.contains("/v5/private"),
            "Private WS sandbox URL should contain /v5/private, got: {}",
            urls.ws_private
        );
    }

    #[test]
    fn test_sandbox_futures_linear() {
        // Sandbox mode with Futures/Linear should use linear testnet endpoints
        let config = ExchangeConfig {
            sandbox: true,
            ..Default::default()
        };
        let options = BybitOptions {
            default_type: DefaultType::Futures,
            default_sub_type: Some(DefaultSubType::Linear),
            ..Default::default()
        };
        let bybit = Bybit::new_with_options(config, options).unwrap();

        assert!(bybit.is_sandbox());
        assert_eq!(bybit.category(), "linear");

        let urls = bybit.urls();
        assert!(
            urls.rest.contains("api-testnet.bybit.com"),
            "Futures Linear sandbox REST URL should contain api-testnet.bybit.com, got: {}",
            urls.rest
        );
    }

    #[test]
    fn test_sandbox_futures_inverse() {
        let config = ExchangeConfig {
            sandbox: true,
            ..Default::default()
        };
        let options = BybitOptions {
            default_type: DefaultType::Futures,
            default_sub_type: Some(DefaultSubType::Inverse),
            ..Default::default()
        };
        let bybit = Bybit::new_with_options(config, options).unwrap();

        assert!(bybit.is_sandbox());
        assert_eq!(bybit.category(), "inverse");

        let urls = bybit.urls();
        assert!(
            urls.rest.contains("api-testnet.bybit.com"),
            "Futures Inverse sandbox REST URL should contain api-testnet.bybit.com, got: {}",
            urls.rest
        );
    }
}
