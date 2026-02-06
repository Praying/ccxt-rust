//! Binance exchange implementation.
//!
//! Supports spot trading, futures trading, and options trading with complete REST API and WebSocket support.

use ccxt_core::types::EndpointType;
use ccxt_core::types::MarketType;
use ccxt_core::types::default_type::{DefaultSubType, DefaultType};
use ccxt_core::{BaseExchange, ExchangeConfig, Result};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

pub mod auth;
pub mod builder;
pub mod constants;
pub mod endpoint_router;
pub mod error;
mod exchange_impl;
pub mod options;
pub mod parser;
pub mod rate_limiter;
pub mod rest;
pub mod signed_request;
pub mod signing_strategy;
pub mod symbol;
pub mod time_sync;
pub mod urls;
pub mod ws;
mod ws_exchange_impl;

pub use builder::BinanceBuilder;
pub use endpoint_router::BinanceEndpointRouter;
pub use error::BinanceApiError;
pub use options::BinanceOptions;
pub use signed_request::{HttpMethod, SignedRequestBuilder};
pub use time_sync::{TimeSyncConfig, TimeSyncManager};
pub use urls::BinanceUrls;

use rate_limiter::WeightRateLimiter;

/// Binance exchange structure.
#[derive(Debug, Clone)]
pub struct Binance {
    /// Base exchange instance.
    base: BaseExchange,
    /// Binance-specific options.
    options: BinanceOptions,
    /// Time synchronization manager for caching server time offset.
    time_sync: Arc<TimeSyncManager>,
    /// Persistent WebSocket connection reference for state tracking.
    #[deprecated(note = "Use connection_manager instead")]
    ws_connection: Arc<RwLock<Option<ws::BinanceWs>>>,
    /// Centralized WebSocket connection manager
    pub connection_manager: Arc<ws::BinanceConnectionManager>,
    /// Rate limiter for API requests.
    rate_limiter: Arc<WeightRateLimiter>,
}

#[allow(deprecated)]
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

        // Determine URLs for ConnectionManager
        let mut urls = if base.config.sandbox {
            BinanceUrls::testnet()
        } else {
            BinanceUrls::production()
        };
        // Apply URL overrides
        if let Some(ws_url) = base.config.url_overrides.get("ws") {
            urls.ws.clone_from(ws_url);
        }
        if let Some(ws_fapi_url) = base.config.url_overrides.get("wsFapi") {
            urls.ws_fapi.clone_from(ws_fapi_url);
        }
        if let Some(ws_dapi_url) = base.config.url_overrides.get("wsDapi") {
            urls.ws_dapi.clone_from(ws_dapi_url);
        }
        if let Some(ws_eapi_url) = base.config.url_overrides.get("wsEapi") {
            urls.ws_eapi.clone_from(ws_eapi_url);
        }

        let connection_manager =
            Arc::new(ws::BinanceConnectionManager::new(urls, base.config.sandbox));
        let rate_limiter = Arc::new(WeightRateLimiter::new());

        Ok(Self {
            base,
            options,
            time_sync,
            ws_connection: Arc::new(RwLock::new(None)),
            connection_manager,
            rate_limiter,
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

        // Determine WS URLs
        let mut urls = if base.config.sandbox {
            BinanceUrls::testnet()
        } else {
            BinanceUrls::production()
        };

        // Apply overrides
        if let Some(ws_url) = base.config.url_overrides.get("ws") {
            urls.ws.clone_from(ws_url);
        }
        if let Some(ws_fapi_url) = base.config.url_overrides.get("wsFapi") {
            urls.ws_fapi.clone_from(ws_fapi_url);
        }
        if let Some(ws_dapi_url) = base.config.url_overrides.get("wsDapi") {
            urls.ws_dapi.clone_from(ws_dapi_url);
        }
        if let Some(ws_eapi_url) = base.config.url_overrides.get("wsEapi") {
            urls.ws_eapi.clone_from(ws_eapi_url);
        }

        let connection_manager =
            Arc::new(ws::BinanceConnectionManager::new(urls, base.config.sandbox));
        let rate_limiter = Arc::new(WeightRateLimiter::new());

        Ok(Self {
            base,
            options,
            time_sync,
            ws_connection: Arc::new(RwLock::new(None)),
            connection_manager,
            rate_limiter,
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
        let options = BinanceOptions {
            default_type: DefaultType::Swap, // Perpetual futures
            ..Default::default()
        };

        // Create TimeSyncManager with configuration from options
        let time_sync_config = TimeSyncConfig {
            sync_interval: Duration::from_secs(options.time_sync_interval_secs),
            auto_sync: options.auto_time_sync,
            max_offset_drift: options.recv_window as i64,
        };
        let time_sync = Arc::new(TimeSyncManager::with_config(time_sync_config));

        // Determine WS URLs (Futures/Swap)
        let mut urls = if base.config.sandbox {
            BinanceUrls::testnet()
        } else {
            BinanceUrls::production()
        };

        if let Some(ws_fapi_url) = base.config.url_overrides.get("wsFapi") {
            urls.ws_fapi.clone_from(ws_fapi_url);
        }

        let connection_manager =
            Arc::new(ws::BinanceConnectionManager::new(urls, base.config.sandbox));
        let rate_limiter = Arc::new(WeightRateLimiter::new());

        Ok(Self {
            base,
            options,
            time_sync,
            ws_connection: Arc::new(RwLock::new(None)),
            connection_manager,
            rate_limiter,
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

    /// Returns a reference to the persistent WebSocket connection.
    ///
    /// This allows access to the stored WebSocket instance for state tracking.
    /// The connection is stored when `ws_connect()` is called and cleared
    /// when `ws_disconnect()` is called.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ccxt_exchanges::binance::Binance;
    /// use ccxt_core::ExchangeConfig;
    ///
    /// # async fn example() -> ccxt_core::Result<()> {
    /// let binance = Binance::new(ExchangeConfig::default())?;
    /// let ws_conn = binance.ws_connection();
    /// let guard = ws_conn.read().await;
    /// if guard.is_some() {
    ///     println!("WebSocket is connected");
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn ws_connection(&self) -> &Arc<RwLock<Option<ws::BinanceWs>>> {
        &self.ws_connection
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

    /// Returns a reference to the rate limiter.
    ///
    /// The `WeightRateLimiter` tracks API usage based on response headers
    /// and provides throttling recommendations to avoid hitting rate limits.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ccxt_exchanges::binance::Binance;
    /// use ccxt_core::ExchangeConfig;
    ///
    /// let binance = Binance::new(ExchangeConfig::default()).unwrap();
    /// let rate_limiter = binance.rate_limiter();
    /// println!("Current weight usage: {}%", rate_limiter.weight_usage_ratio() * 100.0);
    /// ```
    pub fn rate_limiter(&self) -> &Arc<WeightRateLimiter> {
        &self.rate_limiter
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
    pub fn id(&self) -> &'static str {
        "binance"
    }

    /// Returns the exchange name.
    pub fn name(&self) -> &'static str {
        "Binance"
    }

    /// Returns the API version.
    pub fn version(&self) -> &'static str {
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
    pub fn timeframes(&self) -> std::collections::HashMap<String, String> {
        constants::timeframes()
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
            urls.public.clone_from(public_url);
        }
        if let Some(private_url) = self.base().config.url_overrides.get("private") {
            urls.private.clone_from(private_url);
        }
        if let Some(fapi_public_url) = self.base().config.url_overrides.get("fapiPublic") {
            urls.fapi_public.clone_from(fapi_public_url);
        }
        if let Some(fapi_private_url) = self.base().config.url_overrides.get("fapiPrivate") {
            urls.fapi_private.clone_from(fapi_private_url);
        }
        if let Some(dapi_public_url) = self.base().config.url_overrides.get("dapiPublic") {
            urls.dapi_public.clone_from(dapi_public_url);
        }
        if let Some(dapi_private_url) = self.base().config.url_overrides.get("dapiPrivate") {
            urls.dapi_private.clone_from(dapi_private_url);
        }
        // WebSocket URL overrides
        if let Some(ws_url) = self.base().config.url_overrides.get("ws") {
            urls.ws.clone_from(ws_url);
        }
        if let Some(ws_fapi_url) = self.base().config.url_overrides.get("wsFapi") {
            urls.ws_fapi.clone_from(ws_fapi_url);
        }
        if let Some(ws_dapi_url) = self.base().config.url_overrides.get("wsDapi") {
            urls.ws_dapi.clone_from(ws_dapi_url);
        }
        if let Some(ws_eapi_url) = self.base().config.url_overrides.get("wsEapi") {
            urls.ws_eapi.clone_from(ws_eapi_url);
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
        let market_type = MarketType::from(self.options.default_type);
        ws::BinanceWs::new_with_auth(ws_url, self.clone(), market_type)
    }

    /// Checks a JSON response for Binance API errors and converts them to `Result`.
    ///
    /// Binance API may return errors in the response body even with HTTP 200 status.
    /// The error format is: `{"code": -1121, "msg": "Invalid symbol."}`
    ///
    /// This method should be called after receiving a response from public API endpoints
    /// to ensure proper error handling.
    ///
    /// # Arguments
    ///
    /// * `response` - The JSON response from Binance API
    ///
    /// # Returns
    ///
    /// Returns `Ok(response)` if no error is found, or `Err(CoreError)` if the response
    /// contains a Binance API error.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ccxt_exchanges::binance::Binance;
    /// use ccxt_core::ExchangeConfig;
    ///
    /// # async fn example() -> ccxt_core::Result<()> {
    /// let binance = Binance::new(ExchangeConfig::default())?;
    /// let response = binance.base().http_client.get("https://api.binance.com/api/v3/ticker/price", None).await?;
    /// let validated = binance.check_response(response)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn check_response(&self, response: serde_json::Value) -> Result<serde_json::Value> {
        if let Some(api_error) = BinanceApiError::from_json(&response) {
            return Err(api_error.into());
        }
        Ok(response)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::disallowed_methods)]
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

    // ============================================================
    // check_response Tests
    // ============================================================

    #[test]
    fn test_check_response_success() {
        let config = ExchangeConfig::default();
        let binance = Binance::new(config).unwrap();

        // Valid response without error
        let response = serde_json::json!({
            "symbol": "BTCUSDT",
            "price": "50000.00"
        });

        let result = binance.check_response(response.clone());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), response);
    }

    #[test]
    fn test_check_response_with_binance_error() {
        let config = ExchangeConfig::default();
        let binance = Binance::new(config).unwrap();

        // Response with Binance API error
        let response = serde_json::json!({
            "code": -1121,
            "msg": "Invalid symbol."
        });

        let result = binance.check_response(response);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Invalid symbol"));
    }

    #[test]
    fn test_check_response_with_rate_limit_error() {
        let config = ExchangeConfig::default();
        let binance = Binance::new(config).unwrap();

        // Response with rate limit error
        let response = serde_json::json!({
            "code": -1003,
            "msg": "Too many requests; please use the websocket for live updates."
        });

        let result = binance.check_response(response);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, ccxt_core::error::Error::RateLimit { .. }));
    }

    #[test]
    fn test_check_response_with_auth_error() {
        let config = ExchangeConfig::default();
        let binance = Binance::new(config).unwrap();

        // Response with authentication error
        let response = serde_json::json!({
            "code": -2015,
            "msg": "Invalid API-key, IP, or permissions for action."
        });

        let result = binance.check_response(response);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, ccxt_core::error::Error::Authentication(_)));
    }

    #[test]
    fn test_check_response_array_response() {
        let config = ExchangeConfig::default();
        let binance = Binance::new(config).unwrap();

        // Array response (common for list endpoints)
        let response = serde_json::json!([
            {"symbol": "BTCUSDT", "price": "50000.00"},
            {"symbol": "ETHUSDT", "price": "3000.00"}
        ]);

        let result = binance.check_response(response.clone());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), response);
    }
}
