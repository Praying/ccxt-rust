//! Binance exchange implementation.
//!
//! Supports spot trading, futures trading, and options trading with complete REST API and WebSocket support.

use ccxt_core::{BaseExchange, ExchangeConfig, Result};
use std::collections::HashMap;

pub mod auth;
pub mod builder;
pub mod error;
mod exchange_impl;
pub mod parser;
pub mod rest;
pub mod symbol;
pub mod ws;
mod ws_exchange_impl;

pub use builder::BinanceBuilder;

/// Binance exchange structure.
#[derive(Debug)]
pub struct Binance {
    /// Base exchange instance.
    base: BaseExchange,
    /// Binance-specific options.
    options: BinanceOptions,
}

/// Binance-specific options.
#[derive(Debug, Clone)]
pub struct BinanceOptions {
    /// Enables time synchronization.
    pub adjust_for_time_difference: bool,
    /// Receive window in milliseconds.
    pub recv_window: u64,
    /// Default trading type (spot/margin/future/delivery/option).
    pub default_type: String,
    /// Enables testnet mode.
    pub test: bool,
    /// Enables demo environment.
    pub demo: bool,
}

impl Default for BinanceOptions {
    fn default() -> Self {
        Self {
            adjust_for_time_difference: false,
            recv_window: 5000,
            default_type: "spot".to_string(),
            test: false,
            demo: false,
        }
    }
}

impl Binance {
    /// Creates a new Binance instance using the builder pattern.
    ///
    /// This is the recommended way to create a Binance instance.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ccxt_exchanges::binance::Binance;
    ///
    /// let binance = Binance::builder()
    ///     .api_key("your-api-key")
    ///     .secret("your-secret")
    ///     .sandbox(true)
    ///     .build()
    ///     .unwrap();
    /// ```
    pub fn builder() -> BinanceBuilder {
        BinanceBuilder::new()
    }

    /// Creates a new Binance instance.
    ///
    /// # Arguments
    ///
    /// * `config` - Exchange configuration.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ccxt_exchanges::binance::Binance;
    /// use ccxt_core::ExchangeConfig;
    ///
    /// let config = ExchangeConfig {
    ///     id: "binance".to_string(),
    ///     name: "Binance".to_string(),
    ///     api_key: Some("your-api-key".to_string()),
    ///     secret: Some("your-secret".to_string()),
    ///     ..Default::default()
    /// };
    ///
    /// let binance = Binance::new(config).unwrap();
    /// ```
    pub fn new(config: ExchangeConfig) -> Result<Self> {
        let base = BaseExchange::new(config)?;
        let options = BinanceOptions::default();

        Ok(Self { base, options })
    }

    /// Creates a new Binance instance with custom options.
    ///
    /// This is used internally by the builder pattern.
    ///
    /// # Arguments
    ///
    /// * `config` - Exchange configuration.
    /// * `options` - Binance-specific options.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ccxt_exchanges::binance::{Binance, BinanceOptions};
    /// use ccxt_core::ExchangeConfig;
    ///
    /// let config = ExchangeConfig::default();
    /// let options = BinanceOptions {
    ///     default_type: "future".to_string(),
    ///     ..Default::default()
    /// };
    ///
    /// let binance = Binance::new_with_options(config, options).unwrap();
    /// ```
    pub fn new_with_options(config: ExchangeConfig, options: BinanceOptions) -> Result<Self> {
        let base = BaseExchange::new(config)?;
        Ok(Self { base, options })
    }

    /// Creates a new Binance futures instance for perpetual contracts.
    ///
    /// # Arguments
    ///
    /// * `config` - Exchange configuration.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ccxt_exchanges::binance::Binance;
    /// use ccxt_core::ExchangeConfig;
    ///
    /// let config = ExchangeConfig::default();
    /// let futures = Binance::new_futures(config).unwrap();
    /// ```
    pub fn new_futures(config: ExchangeConfig) -> Result<Self> {
        let base = BaseExchange::new(config)?;
        let mut options = BinanceOptions::default();
        options.default_type = "future".to_string();

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

    /// Returns the Binance options.
    pub fn options(&self) -> &BinanceOptions {
        &self.options
    }

    /// Sets the Binance options.
    pub fn set_options(&mut self, options: BinanceOptions) {
        self.options = options;
    }

    /// Returns the exchange ID.
    pub fn id(&self) -> &str {
        "binance"
    }

    /// Returns the exchange name.
    pub fn name(&self) -> &str {
        "Binance"
    }

    /// Returns the API version.
    pub fn version(&self) -> &str {
        "v3"
    }

    /// Returns `true` if the exchange is CCXT-certified.
    pub fn certified(&self) -> bool {
        true
    }

    /// Returns `true` if Pro version (WebSocket) is supported.
    pub fn pro(&self) -> bool {
        true
    }

    /// Returns the rate limit in requests per second.
    pub fn rate_limit(&self) -> f64 {
        50.0
    }

    /// Returns the supported timeframes.
    pub fn timeframes(&self) -> HashMap<String, String> {
        let mut timeframes = HashMap::new();
        timeframes.insert("1s".to_string(), "1s".to_string());
        timeframes.insert("1m".to_string(), "1m".to_string());
        timeframes.insert("3m".to_string(), "3m".to_string());
        timeframes.insert("5m".to_string(), "5m".to_string());
        timeframes.insert("15m".to_string(), "15m".to_string());
        timeframes.insert("30m".to_string(), "30m".to_string());
        timeframes.insert("1h".to_string(), "1h".to_string());
        timeframes.insert("2h".to_string(), "2h".to_string());
        timeframes.insert("4h".to_string(), "4h".to_string());
        timeframes.insert("6h".to_string(), "6h".to_string());
        timeframes.insert("8h".to_string(), "8h".to_string());
        timeframes.insert("12h".to_string(), "12h".to_string());
        timeframes.insert("1d".to_string(), "1d".to_string());
        timeframes.insert("3d".to_string(), "3d".to_string());
        timeframes.insert("1w".to_string(), "1w".to_string());
        timeframes.insert("1M".to_string(), "1M".to_string());
        timeframes
    }

    /// Returns the API URLs.
    pub fn urls(&self) -> BinanceUrls {
        let mut urls = if self.base().config.sandbox {
            BinanceUrls::testnet()
        } else if self.options.demo {
            BinanceUrls::demo()
        } else {
            BinanceUrls::production()
        };

        // Apply URL overrides if present
        if let Some(public_url) = self.base().config.url_overrides.get("public") {
            urls.public = public_url.clone();
        }
        if let Some(private_url) = self.base().config.url_overrides.get("private") {
            urls.private = private_url.clone();
        }
        if let Some(fapi_public_url) = self.base().config.url_overrides.get("fapiPublic") {
            urls.fapi_public = fapi_public_url.clone();
        }
        if let Some(fapi_private_url) = self.base().config.url_overrides.get("fapiPrivate") {
            urls.fapi_private = fapi_private_url.clone();
        }
        if let Some(dapi_public_url) = self.base().config.url_overrides.get("dapiPublic") {
            urls.dapi_public = dapi_public_url.clone();
        }
        if let Some(dapi_private_url) = self.base().config.url_overrides.get("dapiPrivate") {
            urls.dapi_private = dapi_private_url.clone();
        }

        urls
    }

    /// Creates a WebSocket client for public data streams.
    ///
    /// Used for subscribing to public data streams (ticker, orderbook, trades, etc.).
    ///
    /// # Returns
    ///
    /// Returns a `BinanceWs` instance.
    ///
    /// # Example
    /// ```no_run
    /// use ccxt_exchanges::binance::Binance;
    /// use ccxt_core::ExchangeConfig;
    ///
    /// # async fn example() -> ccxt_core::error::Result<()> {
    /// let binance = Binance::new(ExchangeConfig::default())?;
    /// let ws = binance.create_ws();
    /// ws.connect().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn create_ws(&self) -> ws::BinanceWs {
        let urls = self.urls();
        let ws_url = if self.options.default_type == "future" {
            urls.ws_fapi
        } else {
            urls.ws
        };
        ws::BinanceWs::new(ws_url)
    }

    /// Creates an authenticated WebSocket client for user data streams.
    ///
    /// Used for subscribing to private data streams (account balance, order updates, trade history, etc.).
    /// Requires API key configuration.
    ///
    /// # Returns
    ///
    /// Returns a `BinanceWs` instance with listen key manager.
    ///
    /// # Example
    /// ```no_run
    /// use ccxt_exchanges::binance::Binance;
    /// use ccxt_core::ExchangeConfig;
    /// use std::sync::Arc;
    ///
    /// # async fn example() -> ccxt_core::error::Result<()> {
    /// let config = ExchangeConfig {
    ///     api_key: Some("your-api-key".to_string()),
    ///     secret: Some("your-secret".to_string()),
    ///     ..Default::default()
    /// };
    /// let binance = Arc::new(Binance::new(config)?);
    /// let ws = binance.create_authenticated_ws();
    /// ws.connect_user_stream().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn create_authenticated_ws(self: &std::sync::Arc<Self>) -> ws::BinanceWs {
        let urls = self.urls();
        let ws_url = if self.options.default_type == "future" {
            urls.ws_fapi
        } else {
            urls.ws
        };
        ws::BinanceWs::new_with_auth(ws_url, self.clone())
    }
}

/// Binance API URLs.
#[derive(Debug, Clone)]
pub struct BinanceUrls {
    /// Public API URL.
    pub public: String,
    /// Private API URL.
    pub private: String,
    /// SAPI URL (Spot API).
    pub sapi: String,
    /// SAPI V2 URL.
    pub sapi_v2: String,
    /// FAPI URL (Futures API - short form).
    pub fapi: String,
    /// FAPI URL (Futures API).
    pub fapi_public: String,
    /// FAPI Private URL.
    pub fapi_private: String,
    /// DAPI URL (Delivery API - short form).
    pub dapi: String,
    /// DAPI URL (Delivery API).
    pub dapi_public: String,
    /// DAPI Private URL.
    pub dapi_private: String,
    /// EAPI URL (Options API - short form).
    pub eapi: String,
    /// EAPI URL (Options API).
    pub eapi_public: String,
    /// EAPI Private URL.
    pub eapi_private: String,
    /// PAPI URL (Portfolio Margin API).
    pub papi: String,
    /// WebSocket URL.
    pub ws: String,
    /// WebSocket Futures URL.
    pub ws_fapi: String,
}

impl BinanceUrls {
    /// Returns production environment URLs.
    pub fn production() -> Self {
        Self {
            public: "https://api.binance.com/api/v3".to_string(),
            private: "https://api.binance.com/api/v3".to_string(),
            sapi: "https://api.binance.com/sapi/v1".to_string(),
            sapi_v2: "https://api.binance.com/sapi/v2".to_string(),
            fapi: "https://fapi.binance.com/fapi/v1".to_string(),
            fapi_public: "https://fapi.binance.com/fapi/v1".to_string(),
            fapi_private: "https://fapi.binance.com/fapi/v1".to_string(),
            dapi: "https://dapi.binance.com/dapi/v1".to_string(),
            dapi_public: "https://dapi.binance.com/dapi/v1".to_string(),
            dapi_private: "https://dapi.binance.com/dapi/v1".to_string(),
            eapi: "https://eapi.binance.com/eapi/v1".to_string(),
            eapi_public: "https://eapi.binance.com/eapi/v1".to_string(),
            eapi_private: "https://eapi.binance.com/eapi/v1".to_string(),
            papi: "https://papi.binance.com/papi/v1".to_string(),
            ws: "wss://stream.binance.com:9443/ws".to_string(),
            ws_fapi: "wss://fstream.binance.com/ws".to_string(),
        }
    }

    /// Returns testnet URLs.
    pub fn testnet() -> Self {
        Self {
            public: "https://testnet.binance.vision/api/v3".to_string(),
            private: "https://testnet.binance.vision/api/v3".to_string(),
            sapi: "https://testnet.binance.vision/sapi/v1".to_string(),
            sapi_v2: "https://testnet.binance.vision/sapi/v2".to_string(),
            fapi: "https://testnet.binancefuture.com/fapi/v1".to_string(),
            fapi_public: "https://testnet.binancefuture.com/fapi/v1".to_string(),
            fapi_private: "https://testnet.binancefuture.com/fapi/v1".to_string(),
            dapi: "https://testnet.binancefuture.com/dapi/v1".to_string(),
            dapi_public: "https://testnet.binancefuture.com/dapi/v1".to_string(),
            dapi_private: "https://testnet.binancefuture.com/dapi/v1".to_string(),
            eapi: "https://testnet.binanceops.com/eapi/v1".to_string(),
            eapi_public: "https://testnet.binanceops.com/eapi/v1".to_string(),
            eapi_private: "https://testnet.binanceops.com/eapi/v1".to_string(),
            papi: "https://testnet.binance.vision/papi/v1".to_string(),
            ws: "wss://testnet.binance.vision/ws".to_string(),
            ws_fapi: "wss://stream.binancefuture.com/ws".to_string(),
        }
    }

    /// Returns demo environment URLs.
    pub fn demo() -> Self {
        Self {
            public: "https://demo-api.binance.com/api/v3".to_string(),
            private: "https://demo-api.binance.com/api/v3".to_string(),
            sapi: "https://demo-api.binance.com/sapi/v1".to_string(),
            sapi_v2: "https://demo-api.binance.com/sapi/v2".to_string(),
            fapi: "https://demo-fapi.binance.com/fapi/v1".to_string(),
            fapi_public: "https://demo-fapi.binance.com/fapi/v1".to_string(),
            fapi_private: "https://demo-fapi.binance.com/fapi/v1".to_string(),
            dapi: "https://demo-dapi.binance.com/dapi/v1".to_string(),
            dapi_public: "https://demo-dapi.binance.com/dapi/v1".to_string(),
            dapi_private: "https://demo-dapi.binance.com/dapi/v1".to_string(),
            eapi: "https://demo-eapi.binance.com/eapi/v1".to_string(),
            eapi_public: "https://demo-eapi.binance.com/eapi/v1".to_string(),
            eapi_private: "https://demo-eapi.binance.com/eapi/v1".to_string(),
            papi: "https://demo-papi.binance.com/papi/v1".to_string(),
            ws: "wss://demo-stream.binance.com/ws".to_string(),
            ws_fapi: "wss://demo-fstream.binance.com/ws".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_binance_creation() {
        let config = ExchangeConfig {
            id: "binance".to_string(),
            name: "Binance".to_string(),
            ..Default::default()
        };

        let binance = Binance::new(config);
        assert!(binance.is_ok());

        let binance = binance.unwrap();
        assert_eq!(binance.id(), "binance");
        assert_eq!(binance.name(), "Binance");
        assert_eq!(binance.version(), "v3");
        assert!(binance.certified());
        assert!(binance.pro());
    }

    #[test]
    fn test_timeframes() {
        let config = ExchangeConfig::default();
        let binance = Binance::new(config).unwrap();
        let timeframes = binance.timeframes();

        assert!(timeframes.contains_key("1m"));
        assert!(timeframes.contains_key("1h"));
        assert!(timeframes.contains_key("1d"));
        assert_eq!(timeframes.len(), 16);
    }

    #[test]
    fn test_urls() {
        let config = ExchangeConfig::default();
        let binance = Binance::new(config).unwrap();
        let urls = binance.urls();

        assert!(urls.public.contains("api.binance.com"));
        assert!(urls.ws.contains("stream.binance.com"));
    }

    #[test]
    fn test_sandbox_urls() {
        let config = ExchangeConfig {
            sandbox: true,
            ..Default::default()
        };
        let binance = Binance::new(config).unwrap();
        let urls = binance.urls();

        assert!(urls.public.contains("testnet"));
    }
}
