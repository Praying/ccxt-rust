//! Bitget exchange implementation.
//!
//! Supports spot trading and futures trading (USDT-M and Coin-M) with REST API and WebSocket support.

use ccxt_core::{BaseExchange, ExchangeConfig, Result};
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
#[derive(Debug, Clone)]
pub struct BitgetOptions {
    /// Product type: spot, umcbl (USDT-M), dmcbl (Coin-M).
    pub product_type: String,
    /// Receive window in milliseconds.
    pub recv_window: u64,
    /// Enables demo environment.
    pub demo: bool,
}

impl Default for BitgetOptions {
    fn default() -> Self {
        Self {
            product_type: "spot".to_string(),
            recv_window: 5000,
            demo: false,
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
    pub fn rate_limit(&self) -> f64 {
        20.0
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
    pub fn urls(&self) -> BitgetUrls {
        if self.base().config.sandbox || self.options.demo {
            BitgetUrls::demo()
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

    /// Returns demo environment URLs.
    pub fn demo() -> Self {
        Self {
            rest: "https://api.bitget.com".to_string(),
            ws_public: "wss://ws.bitget.com/v2/ws/public/demo".to_string(),
            ws_private: "wss://ws.bitget.com/v2/ws/private/demo".to_string(),
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

        assert!(urls.ws_public.contains("demo"));
    }

    #[test]
    fn test_default_options() {
        let options = BitgetOptions::default();
        assert_eq!(options.product_type, "spot");
        assert_eq!(options.recv_window, 5000);
        assert!(!options.demo);
    }
}
