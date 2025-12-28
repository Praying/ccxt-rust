//! Binance exchange implementation.
//!
//! Supports spot trading, futures trading, and options trading with complete REST API and WebSocket support.

use ccxt_core::types::EndpointType;
use ccxt_core::types::default_type::{DefaultSubType, DefaultType};
use ccxt_core::{BaseExchange, ExchangeConfig, Result};
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

pub mod auth;
pub mod builder;
pub mod endpoint_router;
pub mod error;
mod exchange_impl;
pub mod parser;
pub mod rest;
pub mod signed_request;
pub mod symbol;
pub mod time_sync;
pub mod ws;
mod ws_exchange_impl;

pub use builder::BinanceBuilder;
pub use endpoint_router::BinanceEndpointRouter;
pub use signed_request::{HttpMethod, SignedRequestBuilder};
pub use time_sync::{TimeSyncConfig, TimeSyncManager};
/// Binance exchange structure.
#[derive(Debug)]
pub struct Binance {
    /// Base exchange instance.
    base: BaseExchange,
    /// Binance-specific options.
    options: BinanceOptions,
    /// Time synchronization manager for caching server time offset.
    time_sync: Arc<TimeSyncManager>,
}

/// Binance-specific options.
///
/// # Example
///
/// ```rust
/// use ccxt_exchanges::binance::BinanceOptions;
/// use ccxt_core::types::default_type::{DefaultType, DefaultSubType};
///
/// let options = BinanceOptions {
///     default_type: DefaultType::Swap,
///     default_sub_type: Some(DefaultSubType::Linear),
///     ..Default::default()
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BinanceOptions {
    /// Enables time synchronization.
    pub adjust_for_time_difference: bool,
    /// Receive window in milliseconds.
    pub recv_window: u64,
    /// Default trading type (spot/margin/swap/futures/option).
    ///
    /// This determines which API endpoints to use for operations.
    /// Supports both `DefaultType` enum and string values for backward compatibility.
    #[serde(deserialize_with = "deserialize_default_type")]
    pub default_type: DefaultType,
    /// Default sub-type for contract settlement (linear/inverse).
    ///
    /// - `Linear`: USDT-margined contracts (FAPI)
    /// - `Inverse`: Coin-margined contracts (DAPI)
    ///
    /// Only applicable when `default_type` is `Swap`, `Futures`, or `Option`.
    /// Ignored for `Spot` and `Margin` types.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_sub_type: Option<DefaultSubType>,
    /// Enables testnet mode.
    pub test: bool,
    /// Time sync interval in seconds.
    ///
    /// Controls how often the time offset is refreshed when auto sync is enabled.
    /// Default: 30 seconds.
    #[serde(default = "default_sync_interval")]
    pub time_sync_interval_secs: u64,
    /// Enable automatic periodic time sync.
    ///
    /// When enabled, the time offset will be automatically refreshed
    /// based on `time_sync_interval_secs`.
    /// Default: true.
    #[serde(default = "default_auto_sync")]
    pub auto_time_sync: bool,
}

fn default_sync_interval() -> u64 {
    30
}

fn default_auto_sync() -> bool {
    true
}

/// Custom deserializer for DefaultType that accepts both enum values and strings.
///
/// This provides backward compatibility with configurations that use string values
/// like "spot", "future", "swap", etc.
fn deserialize_default_type<'de, D>(deserializer: D) -> std::result::Result<DefaultType, D::Error>
where
    D: Deserializer<'de>,
{
    use serde::de::Error;

    // First try to deserialize as a string (for backward compatibility)
    // Then try as the enum directly
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum StringOrDefaultType {
        String(String),
        DefaultType(DefaultType),
    }

    match StringOrDefaultType::deserialize(deserializer)? {
        StringOrDefaultType::String(s) => {
            // Handle legacy "future" value (map to Swap for perpetuals)
            let lowercase = s.to_lowercase();
            let normalized = match lowercase.as_str() {
                "future" => "swap",      // Legacy: "future" typically meant perpetual futures
                "delivery" => "futures", // Legacy: "delivery" meant dated futures
                _ => lowercase.as_str(),
            };
            DefaultType::from_str(normalized).map_err(|e| D::Error::custom(e.to_string()))
        }
        StringOrDefaultType::DefaultType(dt) => Ok(dt),
    }
}

impl Default for BinanceOptions {
    fn default() -> Self {
        Self {
            adjust_for_time_difference: false,
            recv_window: 5000,
            default_type: DefaultType::default(), // Defaults to Spot
            default_sub_type: None,
            test: false,
            time_sync_interval_secs: default_sync_interval(),
            auto_time_sync: default_auto_sync(),
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
        let time_sync = Arc::new(TimeSyncManager::new());

        Ok(Self {
            base,
            options,
            time_sync,
        })
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
    /// use ccxt_core::types::default_type::DefaultType;
    ///
    /// let config = ExchangeConfig::default();
    /// let options = BinanceOptions {
    ///     default_type: DefaultType::Swap,
    ///     ..Default::default()
    /// };
    ///
    /// let binance = Binance::new_with_options(config, options).unwrap();
    /// ```
    pub fn new_with_options(config: ExchangeConfig, options: BinanceOptions) -> Result<Self> {
        let base = BaseExchange::new(config)?;

        // Create TimeSyncManager with configuration from options
        let time_sync_config = TimeSyncConfig {
            sync_interval: Duration::from_secs(options.time_sync_interval_secs),
            auto_sync: options.auto_time_sync,
            max_offset_drift: options.recv_window as i64,
        };
        let time_sync = Arc::new(TimeSyncManager::with_config(time_sync_config));

        Ok(Self {
            base,
            options,
            time_sync,
        })
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
    /// let futures = Binance::new_swap(config).unwrap();
    /// ```
    pub fn new_swap(config: ExchangeConfig) -> Result<Self> {
        let base = BaseExchange::new(config)?;
        let mut options = BinanceOptions::default();
        options.default_type = DefaultType::Swap; // Perpetual futures

        // Create TimeSyncManager with configuration from options
        let time_sync_config = TimeSyncConfig {
            sync_interval: Duration::from_secs(options.time_sync_interval_secs),
            auto_sync: options.auto_time_sync,
            max_offset_drift: options.recv_window as i64,
        };
        let time_sync = Arc::new(TimeSyncManager::with_config(time_sync_config));

        Ok(Self {
            base,
            options,
            time_sync,
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

    /// Returns the Binance options.
    pub fn options(&self) -> &BinanceOptions {
        &self.options
    }

    /// Sets the Binance options.
    pub fn set_options(&mut self, options: BinanceOptions) {
        self.options = options;
    }

    /// Returns a reference to the time synchronization manager.
    ///
    /// The `TimeSyncManager` caches the time offset between local system time
    /// and Binance server time, reducing network round-trips for signed requests.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ccxt_exchanges::binance::Binance;
    /// use ccxt_core::ExchangeConfig;
    ///
    /// let binance = Binance::new(ExchangeConfig::default()).unwrap();
    /// let time_sync = binance.time_sync();
    /// println!("Time sync initialized: {}", time_sync.is_initialized());
    /// ```
    pub fn time_sync(&self) -> &Arc<TimeSyncManager> {
        &self.time_sync
    }

    /// Creates a new signed request builder for the given endpoint.
    ///
    /// This is the recommended way to make authenticated API requests.
    /// The builder handles credential validation, timestamp generation,
    /// parameter signing, and request execution.
    ///
    /// # Arguments
    ///
    /// * `endpoint` - Full API endpoint URL
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ccxt_exchanges::binance::{Binance, HttpMethod};
    /// use ccxt_core::ExchangeConfig;
    ///
    /// # async fn example() -> ccxt_core::Result<()> {
    /// let binance = Binance::new(ExchangeConfig::default())?;
    ///
    /// // Simple GET request
    /// let response = binance.signed_request("https://api.binance.com/api/v3/account")
    ///     .execute()
    ///     .await?;
    ///
    /// // POST request with parameters
    /// let response = binance.signed_request("https://api.binance.com/api/v3/order")
    ///     .method(HttpMethod::Post)
    ///     .param("symbol", "BTCUSDT")
    ///     .param("side", "BUY")
    ///     .execute()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn signed_request(&self, endpoint: impl Into<String>) -> SignedRequestBuilder<'_> {
        SignedRequestBuilder::new(self, endpoint)
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
    pub fn rate_limit(&self) -> u32 {
        50
    }

    /// Returns `true` if sandbox/testnet mode is enabled.
    ///
    /// Sandbox mode is enabled when either:
    /// - `config.sandbox` is set to `true`
    /// - `options.test` is set to `true`
    ///
    /// # Returns
    ///
    /// `true` if sandbox mode is enabled, `false` otherwise.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ccxt_exchanges::binance::Binance;
    /// use ccxt_core::ExchangeConfig;
    ///
    /// let config = ExchangeConfig {
    ///     sandbox: true,
    ///     ..Default::default()
    /// };
    /// let binance = Binance::new(config).unwrap();
    /// assert!(binance.is_sandbox());
    /// ```
    pub fn is_sandbox(&self) -> bool {
        self.base().config.sandbox || self.options.test
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
        // WebSocket URL overrides
        if let Some(ws_url) = self.base().config.url_overrides.get("ws") {
            urls.ws = ws_url.clone();
        }
        if let Some(ws_fapi_url) = self.base().config.url_overrides.get("wsFapi") {
            urls.ws_fapi = ws_fapi_url.clone();
        }
        if let Some(ws_dapi_url) = self.base().config.url_overrides.get("wsDapi") {
            urls.ws_dapi = ws_dapi_url.clone();
        }
        if let Some(ws_eapi_url) = self.base().config.url_overrides.get("wsEapi") {
            urls.ws_eapi = ws_eapi_url.clone();
        }

        urls
    }

    /// Determines the WebSocket URL based on default_type and default_sub_type.
    ///
    /// This method implements the endpoint routing logic according to:
    /// - Spot/Margin: Uses the standard WebSocket endpoint
    /// - Swap/Futures with Linear sub-type: Uses FAPI WebSocket endpoint
    /// - Swap/Futures with Inverse sub-type: Uses DAPI WebSocket endpoint
    /// - Option: Uses EAPI WebSocket endpoint
    ///
    /// # Returns
    ///
    /// The appropriate WebSocket URL string.
    ///
    /// # Note
    ///
    /// This method delegates to `BinanceEndpointRouter::default_ws_endpoint()`.
    /// The routing logic is centralized in the trait implementation.
    pub fn get_ws_url(&self) -> String {
        BinanceEndpointRouter::default_ws_endpoint(self)
    }

    /// Returns the public REST API base URL based on default_type and default_sub_type.
    ///
    /// This method implements the endpoint routing logic for public REST API calls:
    /// - Spot: Uses the public API endpoint (api.binance.com)
    /// - Margin: Uses the SAPI endpoint (api.binance.com/sapi)
    /// - Swap/Futures with Linear sub-type: Uses FAPI endpoint (fapi.binance.com)
    /// - Swap/Futures with Inverse sub-type: Uses DAPI endpoint (dapi.binance.com)
    /// - Option: Uses EAPI endpoint (eapi.binance.com)
    ///
    /// # Returns
    ///
    /// The appropriate REST API base URL string.
    ///
    /// # Note
    ///
    /// This method delegates to `BinanceEndpointRouter::default_rest_endpoint(EndpointType::Public)`.
    /// The routing logic is centralized in the trait implementation.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ccxt_exchanges::binance::{Binance, BinanceOptions};
    /// use ccxt_core::ExchangeConfig;
    /// use ccxt_core::types::default_type::{DefaultType, DefaultSubType};
    ///
    /// let options = BinanceOptions {
    ///     default_type: DefaultType::Swap,
    ///     default_sub_type: Some(DefaultSubType::Linear),
    ///     ..Default::default()
    /// };
    /// let binance = Binance::new_with_options(ExchangeConfig::default(), options).unwrap();
    /// let url = binance.get_rest_url_public();
    /// assert!(url.contains("fapi.binance.com"));
    /// ```
    pub fn get_rest_url_public(&self) -> String {
        BinanceEndpointRouter::default_rest_endpoint(self, EndpointType::Public)
    }

    /// Returns the private REST API base URL based on default_type and default_sub_type.
    ///
    /// This method implements the endpoint routing logic for private REST API calls:
    /// - Spot: Uses the private API endpoint (api.binance.com)
    /// - Margin: Uses the SAPI endpoint (api.binance.com/sapi)
    /// - Swap/Futures with Linear sub-type: Uses FAPI endpoint (fapi.binance.com)
    /// - Swap/Futures with Inverse sub-type: Uses DAPI endpoint (dapi.binance.com)
    /// - Option: Uses EAPI endpoint (eapi.binance.com)
    ///
    /// # Returns
    ///
    /// The appropriate REST API base URL string.
    ///
    /// # Note
    ///
    /// This method delegates to `BinanceEndpointRouter::default_rest_endpoint(EndpointType::Private)`.
    /// The routing logic is centralized in the trait implementation.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ccxt_exchanges::binance::{Binance, BinanceOptions};
    /// use ccxt_core::ExchangeConfig;
    /// use ccxt_core::types::default_type::{DefaultType, DefaultSubType};
    ///
    /// let options = BinanceOptions {
    ///     default_type: DefaultType::Swap,
    ///     default_sub_type: Some(DefaultSubType::Inverse),
    ///     ..Default::default()
    /// };
    /// let binance = Binance::new_with_options(ExchangeConfig::default(), options).unwrap();
    /// let url = binance.get_rest_url_private();
    /// assert!(url.contains("dapi.binance.com"));
    /// ```
    pub fn get_rest_url_private(&self) -> String {
        BinanceEndpointRouter::default_rest_endpoint(self, EndpointType::Private)
    }

    /// Checks if the current default_type is a contract type (Swap, Futures, or Option).
    ///
    /// This is useful for determining whether contract-specific API endpoints should be used.
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

    /// Creates a WebSocket client for public data streams.
    ///
    /// Used for subscribing to public data streams (ticker, orderbook, trades, etc.).
    /// The WebSocket endpoint is automatically selected based on `default_type` and `default_sub_type`:
    /// - Spot/Margin: `wss://stream.binance.com:9443/ws`
    /// - Swap/Futures (Linear): `wss://fstream.binance.com/ws`
    /// - Swap/Futures (Inverse): `wss://dstream.binance.com/ws`
    /// - Option: `wss://nbstream.binance.com/eoptions/ws`
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
        let ws_url = self.get_ws_url();
        ws::BinanceWs::new(ws_url)
    }

    /// Creates an authenticated WebSocket client for user data streams.
    ///
    /// Used for subscribing to private data streams (account balance, order updates, trade history, etc.).
    /// Requires API key configuration.
    /// The WebSocket endpoint is automatically selected based on `default_type` and `default_sub_type`.
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
        let ws_url = self.get_ws_url();
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
    /// WebSocket URL (Spot).
    pub ws: String,
    /// WebSocket Futures URL (USDT-margined perpetuals/futures).
    pub ws_fapi: String,
    /// WebSocket Delivery URL (Coin-margined perpetuals/futures).
    pub ws_dapi: String,
    /// WebSocket Options URL.
    pub ws_eapi: String,
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
            ws_dapi: "wss://dstream.binance.com/ws".to_string(),
            ws_eapi: "wss://nbstream.binance.com/eoptions/ws".to_string(),
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
            ws_dapi: "wss://dstream.binancefuture.com/ws".to_string(),
            ws_eapi: "wss://testnet.binanceops.com/ws-api/v3".to_string(),
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
            ws_dapi: "wss://demo-dstream.binance.com/ws".to_string(),
            ws_eapi: "wss://demo-nbstream.binance.com/eoptions/ws".to_string(),
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

    #[test]
    fn test_is_sandbox_with_config_sandbox() {
        let config = ExchangeConfig {
            sandbox: true,
            ..Default::default()
        };
        let binance = Binance::new(config).unwrap();
        assert!(binance.is_sandbox());
    }

    #[test]
    fn test_is_sandbox_with_options_test() {
        let config = ExchangeConfig::default();
        let options = BinanceOptions {
            test: true,
            ..Default::default()
        };
        let binance = Binance::new_with_options(config, options).unwrap();
        assert!(binance.is_sandbox());
    }

    #[test]
    fn test_is_sandbox_false_by_default() {
        let config = ExchangeConfig::default();
        let binance = Binance::new(config).unwrap();
        assert!(!binance.is_sandbox());
    }

    #[test]
    fn test_binance_options_default() {
        let options = BinanceOptions::default();
        assert_eq!(options.default_type, DefaultType::Spot);
        assert_eq!(options.default_sub_type, None);
        assert!(!options.adjust_for_time_difference);
        assert_eq!(options.recv_window, 5000);
        assert!(!options.test);
        assert_eq!(options.time_sync_interval_secs, 30);
        assert!(options.auto_time_sync);
    }

    #[test]
    fn test_binance_options_with_default_type() {
        let options = BinanceOptions {
            default_type: DefaultType::Swap,
            default_sub_type: Some(DefaultSubType::Linear),
            ..Default::default()
        };
        assert_eq!(options.default_type, DefaultType::Swap);
        assert_eq!(options.default_sub_type, Some(DefaultSubType::Linear));
    }

    #[test]
    fn test_binance_options_serialization() {
        let options = BinanceOptions {
            default_type: DefaultType::Swap,
            default_sub_type: Some(DefaultSubType::Linear),
            ..Default::default()
        };
        let json = serde_json::to_string(&options).unwrap();
        assert!(json.contains("\"default_type\":\"swap\""));
        assert!(json.contains("\"default_sub_type\":\"linear\""));
        assert!(json.contains("\"time_sync_interval_secs\":30"));
        assert!(json.contains("\"auto_time_sync\":true"));
    }

    #[test]
    fn test_binance_options_deserialization_with_enum() {
        let json = r#"{
            "adjust_for_time_difference": false,
            "recv_window": 5000,
            "default_type": "swap",
            "default_sub_type": "linear",
            "test": false,
            "time_sync_interval_secs": 60,
            "auto_time_sync": false
        }"#;
        let options: BinanceOptions = serde_json::from_str(json).unwrap();
        assert_eq!(options.default_type, DefaultType::Swap);
        assert_eq!(options.default_sub_type, Some(DefaultSubType::Linear));
        assert_eq!(options.time_sync_interval_secs, 60);
        assert!(!options.auto_time_sync);
    }

    #[test]
    fn test_binance_options_deserialization_legacy_future() {
        // Test backward compatibility with legacy "future" value
        let json = r#"{
            "adjust_for_time_difference": false,
            "recv_window": 5000,
            "default_type": "future",
            "test": false
        }"#;
        let options: BinanceOptions = serde_json::from_str(json).unwrap();
        assert_eq!(options.default_type, DefaultType::Swap);
        // Verify defaults are applied for missing fields
        assert_eq!(options.time_sync_interval_secs, 30);
        assert!(options.auto_time_sync);
    }

    #[test]
    fn test_binance_options_deserialization_legacy_delivery() {
        // Test backward compatibility with legacy "delivery" value
        let json = r#"{
            "adjust_for_time_difference": false,
            "recv_window": 5000,
            "default_type": "delivery",
            "test": false
        }"#;
        let options: BinanceOptions = serde_json::from_str(json).unwrap();
        assert_eq!(options.default_type, DefaultType::Futures);
    }

    #[test]
    fn test_binance_options_deserialization_without_sub_type() {
        let json = r#"{
            "adjust_for_time_difference": false,
            "recv_window": 5000,
            "default_type": "spot",
            "test": false
        }"#;
        let options: BinanceOptions = serde_json::from_str(json).unwrap();
        assert_eq!(options.default_type, DefaultType::Spot);
        assert_eq!(options.default_sub_type, None);
    }

    #[test]
    fn test_binance_options_deserialization_case_insensitive() {
        // Test case-insensitive deserialization
        let json = r#"{
            "adjust_for_time_difference": false,
            "recv_window": 5000,
            "default_type": "SWAP",
            "test": false
        }"#;
        let options: BinanceOptions = serde_json::from_str(json).unwrap();
        assert_eq!(options.default_type, DefaultType::Swap);

        // Test mixed case
        let json = r#"{
            "adjust_for_time_difference": false,
            "recv_window": 5000,
            "default_type": "FuTuReS",
            "test": false
        }"#;
        let options: BinanceOptions = serde_json::from_str(json).unwrap();
        assert_eq!(options.default_type, DefaultType::Futures);
    }

    #[test]
    fn test_new_futures_uses_swap_type() {
        let config = ExchangeConfig::default();
        let binance = Binance::new_swap(config).unwrap();
        assert_eq!(binance.options().default_type, DefaultType::Swap);
    }

    #[test]
    fn test_get_ws_url_spot() {
        let config = ExchangeConfig::default();
        let options = BinanceOptions {
            default_type: DefaultType::Spot,
            ..Default::default()
        };
        let binance = Binance::new_with_options(config, options).unwrap();
        let ws_url = binance.get_ws_url();
        assert!(ws_url.contains("stream.binance.com"));
    }

    #[test]
    fn test_get_ws_url_margin() {
        let config = ExchangeConfig::default();
        let options = BinanceOptions {
            default_type: DefaultType::Margin,
            ..Default::default()
        };
        let binance = Binance::new_with_options(config, options).unwrap();
        let ws_url = binance.get_ws_url();
        // Margin uses the same WebSocket as Spot
        assert!(ws_url.contains("stream.binance.com"));
    }

    #[test]
    fn test_get_ws_url_swap_linear() {
        let config = ExchangeConfig::default();
        let options = BinanceOptions {
            default_type: DefaultType::Swap,
            default_sub_type: Some(DefaultSubType::Linear),
            ..Default::default()
        };
        let binance = Binance::new_with_options(config, options).unwrap();
        let ws_url = binance.get_ws_url();
        assert!(ws_url.contains("fstream.binance.com"));
    }

    #[test]
    fn test_get_ws_url_swap_inverse() {
        let config = ExchangeConfig::default();
        let options = BinanceOptions {
            default_type: DefaultType::Swap,
            default_sub_type: Some(DefaultSubType::Inverse),
            ..Default::default()
        };
        let binance = Binance::new_with_options(config, options).unwrap();
        let ws_url = binance.get_ws_url();
        assert!(ws_url.contains("dstream.binance.com"));
    }

    #[test]
    fn test_get_ws_url_swap_default_sub_type() {
        // When sub_type is not specified, should default to Linear (FAPI)
        let config = ExchangeConfig::default();
        let options = BinanceOptions {
            default_type: DefaultType::Swap,
            default_sub_type: None,
            ..Default::default()
        };
        let binance = Binance::new_with_options(config, options).unwrap();
        let ws_url = binance.get_ws_url();
        assert!(ws_url.contains("fstream.binance.com"));
    }

    #[test]
    fn test_get_ws_url_futures_linear() {
        let config = ExchangeConfig::default();
        let options = BinanceOptions {
            default_type: DefaultType::Futures,
            default_sub_type: Some(DefaultSubType::Linear),
            ..Default::default()
        };
        let binance = Binance::new_with_options(config, options).unwrap();
        let ws_url = binance.get_ws_url();
        assert!(ws_url.contains("fstream.binance.com"));
    }

    #[test]
    fn test_get_ws_url_futures_inverse() {
        let config = ExchangeConfig::default();
        let options = BinanceOptions {
            default_type: DefaultType::Futures,
            default_sub_type: Some(DefaultSubType::Inverse),
            ..Default::default()
        };
        let binance = Binance::new_with_options(config, options).unwrap();
        let ws_url = binance.get_ws_url();
        assert!(ws_url.contains("dstream.binance.com"));
    }

    #[test]
    fn test_get_ws_url_option() {
        let config = ExchangeConfig::default();
        let options = BinanceOptions {
            default_type: DefaultType::Option,
            ..Default::default()
        };
        let binance = Binance::new_with_options(config, options).unwrap();
        let ws_url = binance.get_ws_url();
        assert!(ws_url.contains("nbstream.binance.com") || ws_url.contains("eoptions"));
    }

    #[test]
    fn test_binance_urls_has_all_ws_endpoints() {
        let urls = BinanceUrls::production();
        assert!(!urls.ws.is_empty());
        assert!(!urls.ws_fapi.is_empty());
        assert!(!urls.ws_dapi.is_empty());
        assert!(!urls.ws_eapi.is_empty());
    }

    #[test]
    fn test_binance_urls_testnet_has_all_ws_endpoints() {
        let urls = BinanceUrls::testnet();
        assert!(!urls.ws.is_empty());
        assert!(!urls.ws_fapi.is_empty());
        assert!(!urls.ws_dapi.is_empty());
        assert!(!urls.ws_eapi.is_empty());
    }

    #[test]
    fn test_binance_urls_demo_has_all_ws_endpoints() {
        let urls = BinanceUrls::demo();
        assert!(!urls.ws.is_empty());
        assert!(!urls.ws_fapi.is_empty());
        assert!(!urls.ws_dapi.is_empty());
        assert!(!urls.ws_eapi.is_empty());
    }

    #[test]
    fn test_get_rest_url_public_spot() {
        let config = ExchangeConfig::default();
        let options = BinanceOptions {
            default_type: DefaultType::Spot,
            ..Default::default()
        };
        let binance = Binance::new_with_options(config, options).unwrap();
        let url = binance.get_rest_url_public();
        assert!(url.contains("api.binance.com"));
        assert!(url.contains("/api/v3"));
    }

    #[test]
    fn test_get_rest_url_public_margin() {
        let config = ExchangeConfig::default();
        let options = BinanceOptions {
            default_type: DefaultType::Margin,
            ..Default::default()
        };
        let binance = Binance::new_with_options(config, options).unwrap();
        let url = binance.get_rest_url_public();
        assert!(url.contains("api.binance.com"));
        assert!(url.contains("/sapi/"));
    }

    #[test]
    fn test_get_rest_url_public_swap_linear() {
        let config = ExchangeConfig::default();
        let options = BinanceOptions {
            default_type: DefaultType::Swap,
            default_sub_type: Some(DefaultSubType::Linear),
            ..Default::default()
        };
        let binance = Binance::new_with_options(config, options).unwrap();
        let url = binance.get_rest_url_public();
        assert!(url.contains("fapi.binance.com"));
    }

    #[test]
    fn test_get_rest_url_public_swap_inverse() {
        let config = ExchangeConfig::default();
        let options = BinanceOptions {
            default_type: DefaultType::Swap,
            default_sub_type: Some(DefaultSubType::Inverse),
            ..Default::default()
        };
        let binance = Binance::new_with_options(config, options).unwrap();
        let url = binance.get_rest_url_public();
        assert!(url.contains("dapi.binance.com"));
    }

    #[test]
    fn test_get_rest_url_public_futures_linear() {
        let config = ExchangeConfig::default();
        let options = BinanceOptions {
            default_type: DefaultType::Futures,
            default_sub_type: Some(DefaultSubType::Linear),
            ..Default::default()
        };
        let binance = Binance::new_with_options(config, options).unwrap();
        let url = binance.get_rest_url_public();
        assert!(url.contains("fapi.binance.com"));
    }

    #[test]
    fn test_get_rest_url_public_futures_inverse() {
        let config = ExchangeConfig::default();
        let options = BinanceOptions {
            default_type: DefaultType::Futures,
            default_sub_type: Some(DefaultSubType::Inverse),
            ..Default::default()
        };
        let binance = Binance::new_with_options(config, options).unwrap();
        let url = binance.get_rest_url_public();
        assert!(url.contains("dapi.binance.com"));
    }

    #[test]
    fn test_get_rest_url_public_option() {
        let config = ExchangeConfig::default();
        let options = BinanceOptions {
            default_type: DefaultType::Option,
            ..Default::default()
        };
        let binance = Binance::new_with_options(config, options).unwrap();
        let url = binance.get_rest_url_public();
        assert!(url.contains("eapi.binance.com"));
    }

    #[test]
    fn test_get_rest_url_private_swap_linear() {
        let config = ExchangeConfig::default();
        let options = BinanceOptions {
            default_type: DefaultType::Swap,
            default_sub_type: Some(DefaultSubType::Linear),
            ..Default::default()
        };
        let binance = Binance::new_with_options(config, options).unwrap();
        let url = binance.get_rest_url_private();
        assert!(url.contains("fapi.binance.com"));
    }

    #[test]
    fn test_get_rest_url_private_swap_inverse() {
        let config = ExchangeConfig::default();
        let options = BinanceOptions {
            default_type: DefaultType::Swap,
            default_sub_type: Some(DefaultSubType::Inverse),
            ..Default::default()
        };
        let binance = Binance::new_with_options(config, options).unwrap();
        let url = binance.get_rest_url_private();
        assert!(url.contains("dapi.binance.com"));
    }

    #[test]
    fn test_is_contract_type() {
        let config = ExchangeConfig::default();

        // Spot is not a contract type
        let options = BinanceOptions {
            default_type: DefaultType::Spot,
            ..Default::default()
        };
        let binance = Binance::new_with_options(config.clone(), options).unwrap();
        assert!(!binance.is_contract_type());

        // Margin is not a contract type
        let options = BinanceOptions {
            default_type: DefaultType::Margin,
            ..Default::default()
        };
        let binance = Binance::new_with_options(config.clone(), options).unwrap();
        assert!(!binance.is_contract_type());

        // Swap is a contract type
        let options = BinanceOptions {
            default_type: DefaultType::Swap,
            ..Default::default()
        };
        let binance = Binance::new_with_options(config.clone(), options).unwrap();
        assert!(binance.is_contract_type());

        // Futures is a contract type
        let options = BinanceOptions {
            default_type: DefaultType::Futures,
            ..Default::default()
        };
        let binance = Binance::new_with_options(config.clone(), options).unwrap();
        assert!(binance.is_contract_type());

        // Option is a contract type
        let options = BinanceOptions {
            default_type: DefaultType::Option,
            ..Default::default()
        };
        let binance = Binance::new_with_options(config, options).unwrap();
        assert!(binance.is_contract_type());
    }

    #[test]
    fn test_is_linear_and_is_inverse() {
        let config = ExchangeConfig::default();

        // No sub-type specified defaults to linear
        let options = BinanceOptions {
            default_type: DefaultType::Swap,
            default_sub_type: None,
            ..Default::default()
        };
        let binance = Binance::new_with_options(config.clone(), options).unwrap();
        assert!(binance.is_linear());
        assert!(!binance.is_inverse());

        // Explicit linear
        let options = BinanceOptions {
            default_type: DefaultType::Swap,
            default_sub_type: Some(DefaultSubType::Linear),
            ..Default::default()
        };
        let binance = Binance::new_with_options(config.clone(), options).unwrap();
        assert!(binance.is_linear());
        assert!(!binance.is_inverse());

        // Explicit inverse
        let options = BinanceOptions {
            default_type: DefaultType::Swap,
            default_sub_type: Some(DefaultSubType::Inverse),
            ..Default::default()
        };
        let binance = Binance::new_with_options(config, options).unwrap();
        assert!(!binance.is_linear());
        assert!(binance.is_inverse());
    }

    // ============================================================
    // Sandbox Mode Market Type URL Selection Tests
    // ============================================================

    #[test]
    fn test_sandbox_market_type_spot() {
        let config = ExchangeConfig {
            sandbox: true,
            ..Default::default()
        };
        let options = BinanceOptions {
            default_type: DefaultType::Spot,
            ..Default::default()
        };
        let binance = Binance::new_with_options(config, options).unwrap();

        assert!(binance.is_sandbox());
        let url = binance.get_rest_url_public();
        assert!(
            url.contains("testnet.binance.vision"),
            "Spot sandbox URL should contain testnet.binance.vision, got: {}",
            url
        );
        assert!(
            url.contains("/api/v3"),
            "Spot sandbox URL should contain /api/v3, got: {}",
            url
        );
    }

    #[test]
    fn test_sandbox_market_type_swap_linear() {
        let config = ExchangeConfig {
            sandbox: true,
            ..Default::default()
        };
        let options = BinanceOptions {
            default_type: DefaultType::Swap,
            default_sub_type: Some(DefaultSubType::Linear),
            ..Default::default()
        };
        let binance = Binance::new_with_options(config, options).unwrap();

        assert!(binance.is_sandbox());
        let url = binance.get_rest_url_public();
        assert!(
            url.contains("testnet.binancefuture.com"),
            "Linear sandbox URL should contain testnet.binancefuture.com, got: {}",
            url
        );
        assert!(
            url.contains("/fapi/"),
            "Linear sandbox URL should contain /fapi/, got: {}",
            url
        );
    }

    #[test]
    fn test_sandbox_market_type_swap_inverse() {
        let config = ExchangeConfig {
            sandbox: true,
            ..Default::default()
        };
        let options = BinanceOptions {
            default_type: DefaultType::Swap,
            default_sub_type: Some(DefaultSubType::Inverse),
            ..Default::default()
        };
        let binance = Binance::new_with_options(config, options).unwrap();

        assert!(binance.is_sandbox());
        let url = binance.get_rest_url_public();
        assert!(
            url.contains("testnet.binancefuture.com"),
            "Inverse sandbox URL should contain testnet.binancefuture.com, got: {}",
            url
        );
        assert!(
            url.contains("/dapi/"),
            "Inverse sandbox URL should contain /dapi/, got: {}",
            url
        );
    }

    #[test]
    fn test_sandbox_market_type_option() {
        let config = ExchangeConfig {
            sandbox: true,
            ..Default::default()
        };
        let options = BinanceOptions {
            default_type: DefaultType::Option,
            ..Default::default()
        };
        let binance = Binance::new_with_options(config, options).unwrap();

        assert!(binance.is_sandbox());
        let url = binance.get_rest_url_public();
        assert!(
            url.contains("testnet.binanceops.com"),
            "Option sandbox URL should contain testnet.binanceops.com, got: {}",
            url
        );
        assert!(
            url.contains("/eapi/"),
            "Option sandbox URL should contain /eapi/, got: {}",
            url
        );
    }

    #[test]
    fn test_sandbox_market_type_futures_linear() {
        let config = ExchangeConfig {
            sandbox: true,
            ..Default::default()
        };
        let options = BinanceOptions {
            default_type: DefaultType::Futures,
            default_sub_type: Some(DefaultSubType::Linear),
            ..Default::default()
        };
        let binance = Binance::new_with_options(config, options).unwrap();

        assert!(binance.is_sandbox());
        let url = binance.get_rest_url_public();
        assert!(
            url.contains("testnet.binancefuture.com"),
            "Futures Linear sandbox URL should contain testnet.binancefuture.com, got: {}",
            url
        );
        assert!(
            url.contains("/fapi/"),
            "Futures Linear sandbox URL should contain /fapi/, got: {}",
            url
        );
    }

    #[test]
    fn test_sandbox_market_type_futures_inverse() {
        let config = ExchangeConfig {
            sandbox: true,
            ..Default::default()
        };
        let options = BinanceOptions {
            default_type: DefaultType::Futures,
            default_sub_type: Some(DefaultSubType::Inverse),
            ..Default::default()
        };
        let binance = Binance::new_with_options(config, options).unwrap();

        assert!(binance.is_sandbox());
        let url = binance.get_rest_url_public();
        assert!(
            url.contains("testnet.binancefuture.com"),
            "Futures Inverse sandbox URL should contain testnet.binancefuture.com, got: {}",
            url
        );
        assert!(
            url.contains("/dapi/"),
            "Futures Inverse sandbox URL should contain /dapi/, got: {}",
            url
        );
    }

    #[test]
    fn test_sandbox_websocket_url_spot() {
        // Verify WebSocket URL selection in sandbox mode for Spot
        let config = ExchangeConfig {
            sandbox: true,
            ..Default::default()
        };
        let options = BinanceOptions {
            default_type: DefaultType::Spot,
            ..Default::default()
        };
        let binance = Binance::new_with_options(config, options).unwrap();

        let urls = binance.urls();
        assert!(
            urls.ws.contains("testnet.binance.vision"),
            "Spot WS sandbox URL should contain testnet.binance.vision, got: {}",
            urls.ws
        );
    }

    #[test]
    fn test_sandbox_websocket_url_fapi() {
        // Verify WebSocket URL selection in sandbox mode for FAPI (Linear)
        let config = ExchangeConfig {
            sandbox: true,
            ..Default::default()
        };
        let options = BinanceOptions {
            default_type: DefaultType::Swap,
            default_sub_type: Some(DefaultSubType::Linear),
            ..Default::default()
        };
        let binance = Binance::new_with_options(config, options).unwrap();

        let urls = binance.urls();
        assert!(
            urls.ws_fapi.contains("binancefuture.com"),
            "FAPI WS sandbox URL should contain binancefuture.com, got: {}",
            urls.ws_fapi
        );
    }

    #[test]
    fn test_sandbox_websocket_url_dapi() {
        // Verify WebSocket URL selection in sandbox mode for DAPI (Inverse)
        let config = ExchangeConfig {
            sandbox: true,
            ..Default::default()
        };
        let options = BinanceOptions {
            default_type: DefaultType::Swap,
            default_sub_type: Some(DefaultSubType::Inverse),
            ..Default::default()
        };
        let binance = Binance::new_with_options(config, options).unwrap();

        let urls = binance.urls();
        assert!(
            urls.ws_dapi.contains("dstream.binancefuture.com"),
            "DAPI WS sandbox URL should contain dstream.binancefuture.com, got: {}",
            urls.ws_dapi
        );
    }

    #[test]
    fn test_sandbox_websocket_url_eapi() {
        // Verify WebSocket URL selection in sandbox mode for EAPI (Options)
        let config = ExchangeConfig {
            sandbox: true,
            ..Default::default()
        };
        let options = BinanceOptions {
            default_type: DefaultType::Option,
            ..Default::default()
        };
        let binance = Binance::new_with_options(config, options).unwrap();

        let urls = binance.urls();
        assert!(
            urls.ws_eapi.contains("testnet.binanceops.com"),
            "EAPI WS sandbox URL should contain testnet.binanceops.com, got: {}",
            urls.ws_eapi
        );
    }
}
