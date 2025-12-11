//! OKX exchange implementation.
//!
//! Supports spot trading and futures trading (USDT-M and Coin-M) with REST API and WebSocket support.
//! OKX uses V5 unified API with HMAC-SHA256 + Base64 authentication.

use ccxt_core::{BaseExchange, ExchangeConfig, Result};
use std::collections::HashMap;

pub mod auth;
pub mod builder;
pub mod error;
pub mod exchange_impl;
pub mod parser;
pub mod rest;
pub mod symbol;
pub mod ws;
pub mod ws_exchange_impl;

pub use auth::OkxAuth;
pub use builder::OkxBuilder;
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
#[derive(Debug, Clone)]
pub struct OkxOptions {
    /// Account mode: cash (spot), cross (cross margin), isolated (isolated margin).
    pub account_mode: String,
    /// Enables demo trading environment.
    pub demo: bool,
}

impl Default for OkxOptions {
    fn default() -> Self {
        Self {
            account_mode: "cash".to_string(),
            demo: false,
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
    pub fn rate_limit(&self) -> f64 {
        20.0
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
        if self.base().config.sandbox || self.options.demo {
            OkxUrls::demo()
        } else {
            OkxUrls::production()
        }
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
    fn test_default_options() {
        let options = OkxOptions::default();
        assert_eq!(options.account_mode, "cash");
        assert!(!options.demo);
    }
}
