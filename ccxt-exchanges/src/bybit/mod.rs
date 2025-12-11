//! Bybit exchange implementation.
//!
//! Supports spot trading and futures trading (USDT-M and Coin-M) with REST API and WebSocket support.
//! Bybit uses V5 unified account API with HMAC-SHA256 authentication.

use ccxt_core::{BaseExchange, ExchangeConfig, Result};
use std::collections::HashMap;

pub mod auth;
pub mod builder;
pub mod error;
pub mod parser;
pub mod rest;
pub mod symbol;
pub mod ws;
mod ws_exchange_impl;

pub use auth::BybitAuth;
pub use builder::BybitBuilder;
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
#[derive(Debug, Clone)]
pub struct BybitOptions {
    /// Account type: UNIFIED, CONTRACT, SPOT.
    pub account_type: String,
    /// Enables testnet environment.
    pub testnet: bool,
    /// Receive window in milliseconds.
    pub recv_window: u64,
}

impl Default for BybitOptions {
    fn default() -> Self {
        Self {
            account_type: "UNIFIED".to_string(),
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
    pub fn rate_limit(&self) -> f64 {
        20.0
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

    /// Creates a public WebSocket client.
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
        ws::BybitWs::new(urls.ws_public)
    }
}

/// Bybit API URLs.
#[derive(Debug, Clone)]
pub struct BybitUrls {
    /// REST API base URL.
    pub rest: String,
    /// Public WebSocket URL.
    pub ws_public: String,
    /// Private WebSocket URL.
    pub ws_private: String,
}

impl BybitUrls {
    /// Returns production environment URLs.
    pub fn production() -> Self {
        Self {
            rest: "https://api.bybit.com".to_string(),
            ws_public: "wss://stream.bybit.com/v5/public/spot".to_string(),
            ws_private: "wss://stream.bybit.com/v5/private".to_string(),
        }
    }

    /// Returns testnet environment URLs.
    pub fn testnet() -> Self {
        Self {
            rest: "https://api-testnet.bybit.com".to_string(),
            ws_public: "wss://stream-testnet.bybit.com/v5/public/spot".to_string(),
            ws_private: "wss://stream-testnet.bybit.com/v5/private".to_string(),
        }
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
    }

    #[test]
    fn test_default_options() {
        let options = BybitOptions::default();
        assert_eq!(options.account_type, "UNIFIED");
        assert!(!options.testnet);
        assert_eq!(options.recv_window, 5000);
    }
}
