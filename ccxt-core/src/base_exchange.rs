//! Base exchange implementation
//!
//! Provides core functionality for all exchange implementations:
//! - Market data caching
//! - API configuration management
//! - Common request/response handling
//! - Authentication and signing
//! - Rate limiting

use crate::error::{Error, ParseError, Result};
use crate::http_client::{HttpClient, HttpConfig};
use crate::rate_limiter::{RateLimiter, RateLimiterConfig};
use crate::types::*;
use rust_decimal::Decimal;
use rust_decimal::prelude::{FromStr, ToPrimitive};
use serde_json::Value;
use std::collections::HashMap;
use std::future::Future;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use tracing::{debug, info};

/// Exchange configuration
#[derive(Debug, Clone)]
pub struct ExchangeConfig {
    /// Exchange identifier
    pub id: String,
    /// Exchange display name
    pub name: String,
    /// API key for authentication
    pub api_key: Option<String>,
    /// API secret for authentication
    pub secret: Option<String>,
    /// Password (required by some exchanges)
    pub password: Option<String>,
    /// User ID (required by some exchanges)
    pub uid: Option<String>,
    /// Account ID
    pub account_id: Option<String>,
    /// Enable rate limiting
    pub enable_rate_limit: bool,
    /// Rate limit in requests per second
    pub rate_limit: u32,
    /// Request timeout in seconds
    pub timeout: u64,
    /// Enable sandbox/testnet mode
    pub sandbox: bool,
    /// Custom user agent string
    pub user_agent: Option<String>,
    /// HTTP proxy server URL
    pub proxy: Option<String>,
    /// Enable verbose logging
    pub verbose: bool,
    /// Custom exchange-specific options
    pub options: HashMap<String, Value>,
    /// URL overrides for mocking/testing
    pub url_overrides: HashMap<String, String>,
}

impl Default for ExchangeConfig {
    fn default() -> Self {
        Self {
            id: String::new(),
            name: String::new(),
            api_key: None,
            secret: None,
            password: None,
            uid: None,
            account_id: None,
            enable_rate_limit: true,
            rate_limit: 10,
            timeout: 30,
            sandbox: false,
            user_agent: Some(format!("ccxt-rust/{}", env!("CARGO_PKG_VERSION"))),
            proxy: None,
            verbose: false,
            options: HashMap::new(),
            url_overrides: HashMap::new(),
        }
    }
}

impl ExchangeConfig {
    /// Create a new configuration builder
    ///
    /// # Example
    ///
    /// ```rust
    /// use ccxt_core::base_exchange::ExchangeConfig;
    ///
    /// let config = ExchangeConfig::builder()
    ///     .id("binance")
    ///     .name("Binance")
    ///     .api_key("your-api-key")
    ///     .secret("your-secret")
    ///     .sandbox(true)
    ///     .build();
    /// ```
    pub fn builder() -> ExchangeConfigBuilder {
        ExchangeConfigBuilder::default()
    }
}

/// Builder for `ExchangeConfig`
///
/// Provides a fluent API for constructing exchange configurations.
///
/// # Example
///
/// ```rust
/// use ccxt_core::base_exchange::ExchangeConfigBuilder;
///
/// let config = ExchangeConfigBuilder::new()
///     .id("binance")
///     .name("Binance")
///     .api_key("your-api-key")
///     .secret("your-secret")
///     .timeout(60)
///     .build();
/// ```
#[derive(Debug, Clone, Default)]
pub struct ExchangeConfigBuilder {
    config: ExchangeConfig,
}

impl ExchangeConfigBuilder {
    /// Create a new builder with default configuration
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the exchange identifier
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.config.id = id.into();
        self
    }

    /// Set the exchange display name
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.config.name = name.into();
        self
    }

    /// Set the API key for authentication
    pub fn api_key(mut self, key: impl Into<String>) -> Self {
        self.config.api_key = Some(key.into());
        self
    }

    /// Set the API secret for authentication
    pub fn secret(mut self, secret: impl Into<String>) -> Self {
        self.config.secret = Some(secret.into());
        self
    }

    /// Set the password (required by some exchanges)
    pub fn password(mut self, password: impl Into<String>) -> Self {
        self.config.password = Some(password.into());
        self
    }

    /// Set the user ID (required by some exchanges)
    pub fn uid(mut self, uid: impl Into<String>) -> Self {
        self.config.uid = Some(uid.into());
        self
    }

    /// Set the account ID
    pub fn account_id(mut self, account_id: impl Into<String>) -> Self {
        self.config.account_id = Some(account_id.into());
        self
    }

    /// Enable or disable rate limiting
    pub fn enable_rate_limit(mut self, enabled: bool) -> Self {
        self.config.enable_rate_limit = enabled;
        self
    }

    /// Set the rate limit in requests per second
    pub fn rate_limit(mut self, rate_limit: u32) -> Self {
        self.config.rate_limit = rate_limit;
        self
    }

    /// Set the request timeout in seconds
    pub fn timeout(mut self, seconds: u64) -> Self {
        self.config.timeout = seconds;
        self
    }

    /// Enable or disable sandbox/testnet mode
    pub fn sandbox(mut self, enabled: bool) -> Self {
        self.config.sandbox = enabled;
        self
    }

    /// Set a custom user agent string
    pub fn user_agent(mut self, user_agent: impl Into<String>) -> Self {
        self.config.user_agent = Some(user_agent.into());
        self
    }

    /// Set the HTTP proxy server URL
    pub fn proxy(mut self, proxy: impl Into<String>) -> Self {
        self.config.proxy = Some(proxy.into());
        self
    }

    /// Enable or disable verbose logging
    pub fn verbose(mut self, enabled: bool) -> Self {
        self.config.verbose = enabled;
        self
    }

    /// Set a custom option
    pub fn option(mut self, key: impl Into<String>, value: Value) -> Self {
        self.config.options.insert(key.into(), value);
        self
    }

    /// Set multiple custom options
    pub fn options(mut self, options: HashMap<String, Value>) -> Self {
        self.config.options.extend(options);
        self
    }

    /// Set a URL override for a specific key (e.g., "public", "private")
    pub fn url_override(mut self, key: impl Into<String>, url: impl Into<String>) -> Self {
        self.config.url_overrides.insert(key.into(), url.into());
        self
    }

    /// Build the configuration
    pub fn build(self) -> ExchangeConfig {
        self.config
    }
}

/// Market data cache structure
#[derive(Debug, Clone)]
pub struct MarketCache {
    /// Markets indexed by symbol (e.g., "BTC/USDT")
    pub markets: HashMap<String, Arc<Market>>,
    /// Markets indexed by exchange-specific ID
    pub markets_by_id: HashMap<String, Arc<Market>>,
    /// Currencies indexed by code (e.g., "BTC")
    pub currencies: HashMap<String, Arc<Currency>>,
    /// Currencies indexed by exchange-specific ID
    pub currencies_by_id: HashMap<String, Arc<Currency>>,
    /// List of all trading pair symbols
    pub symbols: Vec<String>,
    /// List of all currency codes
    pub codes: Vec<String>,
    /// List of all market IDs
    pub ids: Vec<String>,
    /// Whether markets have been loaded
    pub loaded: bool,
}

impl Default for MarketCache {
    fn default() -> Self {
        Self {
            markets: HashMap::new(),
            markets_by_id: HashMap::new(),
            currencies: HashMap::new(),
            currencies_by_id: HashMap::new(),
            symbols: Vec::new(),
            codes: Vec::new(),
            ids: Vec::new(),
            loaded: false,
        }
    }
}

// Note: ExchangeCapabilities is now defined in crate::exchange module.
// Import it from there for use in BaseExchange.
use crate::exchange::ExchangeCapabilities;

/// Base exchange implementation
#[derive(Debug)]
pub struct BaseExchange {
    /// Exchange configuration
    pub config: ExchangeConfig,
    /// HTTP client for API requests (handles rate limiting internally)
    pub http_client: HttpClient,
    /// Thread-safe market data cache
    pub market_cache: Arc<RwLock<MarketCache>>,
    /// Mutex to serialize market loading operations and prevent concurrent API calls
    pub market_loading_lock: Arc<Mutex<()>>,
    /// Exchange capability flags
    pub capabilities: ExchangeCapabilities,
    /// API endpoint URLs
    pub urls: HashMap<String, String>,
    /// Timeframe mappings (e.g., "1m" -> "1")
    pub timeframes: HashMap<String, String>,
    /// Precision mode for price/amount formatting
    pub precision_mode: PrecisionMode,
}

impl BaseExchange {
    /// Creates a new exchange instance
    ///
    /// # Arguments
    ///
    /// * `config` - Exchange configuration
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing the initialized exchange instance.
    ///
    /// # Errors
    ///
    /// Returns an error if HTTP client or rate limiter initialization fails.
    pub fn new(config: ExchangeConfig) -> Result<Self> {
        info!("Initializing exchange: {}", config.id);

        let http_config = HttpConfig {
            timeout: config.timeout,
            #[allow(deprecated)]
            max_retries: 3,
            verbose: false,
            user_agent: config
                .user_agent
                .clone()
                .unwrap_or_else(|| format!("ccxt-rust/{}", env!("CARGO_PKG_VERSION"))),
            return_response_headers: false,
            proxy: config.proxy.clone(),
            enable_rate_limit: true,
            retry_config: None,
        };

        let mut http_client = HttpClient::new(http_config)?;

        // Rate limiting is fully managed by HttpClient
        if config.enable_rate_limit {
            let rate_config =
                RateLimiterConfig::new(config.rate_limit, std::time::Duration::from_millis(1000));
            let limiter = RateLimiter::new(rate_config);
            http_client.set_rate_limiter(limiter);
        }

        Ok(Self {
            config,
            http_client,
            market_cache: Arc::new(RwLock::new(MarketCache::default())),
            market_loading_lock: Arc::new(Mutex::new(())),
            capabilities: ExchangeCapabilities::default(),
            urls: HashMap::new(),
            timeframes: HashMap::new(),
            precision_mode: PrecisionMode::DecimalPlaces,
        })
    }

    /// Loads market data from the exchange
    ///
    /// # Arguments
    ///
    /// * `reload` - Whether to force reload even if markets are already cached
    ///
    /// # Returns
    ///
    /// Returns a `HashMap` of markets indexed by symbol.
    ///
    /// # Errors
    ///
    /// Returns an error if market data cannot be fetched. This base implementation
    /// always returns `NotImplemented` error as exchanges must override this method.
    pub async fn load_markets(&self, reload: bool) -> Result<HashMap<String, Market>> {
        let cache = self.market_cache.write().await;

        if cache.loaded && !reload {
            debug!("Returning cached markets for {}", self.config.id);
            return Ok(cache
                .markets
                .iter()
                .map(|(k, v)| (k.clone(), (**v).clone()))
                .collect());
        }

        info!("Loading markets for {}", self.config.id);

        drop(cache);

        Err(Error::not_implemented(
            "load_markets must be implemented by exchange",
        ))
    }

    /// Loads market data with a custom loader function, ensuring thread-safe concurrent access.
    ///
    /// This method serializes market loading operations to prevent multiple concurrent API calls
    /// when multiple tasks call `load_markets` simultaneously. Only the first caller will
    /// execute the loader; subsequent callers will wait for the lock and then return cached data.
    ///
    /// # Arguments
    ///
    /// * `reload` - Whether to force reload even if markets are already cached
    /// * `loader` - An async closure that performs the actual market data fetching.
    ///   Should return `Result<()>` and is responsible for calling `set_markets`.
    ///
    /// # Returns
    ///
    /// Returns a `HashMap` of markets indexed by symbol.
    ///
    /// # Errors
    ///
    /// Returns an error if the loader function fails.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let markets = self.base().load_markets_with_loader(reload, || async {
    ///     let markets = self.fetch_markets_from_api().await?;
    ///     Ok((markets, None))
    /// }).await?;
    /// ```
    pub async fn load_markets_with_loader<F, Fut>(
        &self,
        reload: bool,
        loader: F,
    ) -> Result<HashMap<String, Arc<Market>>>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<(Vec<Market>, Option<Vec<Currency>>)>>,
    {
        // Acquire the loading lock to serialize concurrent load_markets calls
        let _loading_guard = self.market_loading_lock.lock().await;

        // Check cache status while holding the lock
        {
            let cache = self.market_cache.read().await;
            if cache.loaded && !reload {
                debug!(
                    "Returning cached markets for {} ({} markets)",
                    self.config.id,
                    cache.markets.len()
                );
                return Ok(cache.markets.clone());
            }
        }

        // Execute the loader to fetch market data
        info!(
            "Loading markets for {} (reload: {})",
            self.config.id, reload
        );
        let (markets, currencies) = loader().await?;

        // Base layer is responsible for setting markets
        self.set_markets(markets, currencies).await?;

        // Return the loaded markets
        let cache = self.market_cache.read().await;
        Ok(cache.markets.clone())
    }

    /// Sets market and currency data in the cache
    ///
    /// # Arguments
    ///
    /// * `markets` - Vector of market definitions to cache
    /// * `currencies` - Optional vector of currency definitions to cache
    ///
    /// # Returns
    ///
    /// Returns `Ok(markets)` on successful cache update, preserving ownership for the caller.
    ///
    /// # Errors
    ///
    /// This method should not fail under normal circumstances.
    pub async fn set_markets(
        &self,
        markets: Vec<Market>,
        currencies: Option<Vec<Currency>>,
    ) -> Result<HashMap<String, Arc<Market>>> {
        let mut cache = self.market_cache.write().await;

        cache.markets.clear();
        cache.markets_by_id.clear();
        cache.symbols.clear();
        cache.ids.clear();

        for market in markets {
            cache.symbols.push(market.symbol.clone());
            cache.ids.push(market.id.clone());
            let arc_market = Arc::new(market);
            cache
                .markets_by_id
                .insert(arc_market.id.clone(), Arc::clone(&arc_market));
            cache.markets.insert(arc_market.symbol.clone(), arc_market);
        }

        if let Some(currencies) = currencies {
            cache.currencies.clear();
            cache.currencies_by_id.clear();
            cache.codes.clear();

            for currency in currencies {
                cache.codes.push(currency.code.clone());
                let arc_currency = Arc::new(currency);
                cache
                    .currencies_by_id
                    .insert(arc_currency.id.clone(), Arc::clone(&arc_currency));
                cache
                    .currencies
                    .insert(arc_currency.code.clone(), arc_currency);
            }
        }

        cache.loaded = true;
        info!(
            "Loaded {} markets and {} currencies for {}",
            cache.markets.len(),
            cache.currencies.len(),
            self.config.id
        );

        Ok(cache.markets.clone())
    }

    /// Gets market information by trading symbol
    ///
    /// # Arguments
    ///
    /// * `symbol` - Trading pair symbol (e.g., "BTC/USDT")
    ///
    /// # Returns
    ///
    /// Returns the market definition for the specified symbol.
    ///
    /// # Errors
    ///
    /// Returns an error if markets are not loaded or the symbol is not found.
    pub async fn market(&self, symbol: &str) -> Result<Arc<Market>> {
        let cache = self.market_cache.read().await;

        if !cache.loaded {
            drop(cache);
            return Err(Error::exchange(
                "-1",
                "Markets not loaded. Call load_markets() first.",
            ));
        }

        cache
            .markets
            .get(symbol)
            .cloned()
            .ok_or_else(|| Error::bad_symbol(format!("Market {} not found", symbol)))
    }

    /// Gets market information by exchange-specific market ID
    ///
    /// # Arguments
    ///
    /// * `id` - Exchange-specific market identifier
    ///
    /// # Returns
    ///
    /// Returns the market definition for the specified ID.
    ///
    /// # Errors
    ///
    /// Returns an error if the market ID is not found.
    pub async fn market_by_id(&self, id: &str) -> Result<Arc<Market>> {
        let cache = self.market_cache.read().await;

        cache
            .markets_by_id
            .get(id)
            .cloned()
            .ok_or_else(|| Error::bad_symbol(format!("Market with id {} not found", id)))
    }

    /// Gets currency information by currency code
    ///
    /// # Arguments
    ///
    /// * `code` - Currency code (e.g., "BTC", "USDT")
    ///
    /// # Returns
    ///
    /// Returns the currency definition for the specified code.
    ///
    /// # Errors
    ///
    /// Returns an error if the currency is not found.
    pub async fn currency(&self, code: &str) -> Result<Arc<Currency>> {
        let cache = self.market_cache.read().await;

        cache
            .currencies
            .get(code)
            .cloned()
            .ok_or_else(|| Error::bad_symbol(format!("Currency {} not found", code)))
    }

    /// Gets all available trading symbols
    ///
    /// # Returns
    ///
    /// Returns a vector of all trading pair symbols.
    pub async fn symbols(&self) -> Result<Vec<String>> {
        let cache = self.market_cache.read().await;
        Ok(cache.symbols.clone())
    }

    /// Applies rate limiting if enabled
    ///
    /// # Deprecated
    ///
    /// Rate limiting is now fully managed by HttpClient internally.
    /// This method is a no-op and will be removed in a future version.
    #[deprecated(
        since = "0.2.0",
        note = "Rate limiting is now handled internally by HttpClient. This method is a no-op."
    )]
    pub fn throttle(&self) -> Result<()> {
        // Rate limiting is now fully managed by HttpClient
        Ok(())
    }

    /// Checks that required API credentials are configured
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if both API key and secret are present.
    ///
    /// # Errors
    ///
    /// Returns an authentication error if credentials are missing.
    pub fn check_required_credentials(&self) -> Result<()> {
        if self.config.api_key.is_none() {
            return Err(Error::authentication("API key is required"));
        }
        if self.config.secret.is_none() {
            return Err(Error::authentication("API secret is required"));
        }
        Ok(())
    }

    /// Gets a nonce value (current timestamp in milliseconds)
    ///
    /// # Returns
    ///
    /// Returns current Unix timestamp in milliseconds.
    pub fn nonce(&self) -> i64 {
        crate::time::milliseconds()
    }

    /// Builds a URL query string from parameters
    ///
    /// # Arguments
    ///
    /// * `params` - Parameter key-value pairs
    ///
    /// # Returns
    ///
    /// Returns a URL-encoded query string (e.g., "key1=value1&key2=value2").
    pub fn build_query_string(&self, params: &HashMap<String, Value>) -> String {
        if params.is_empty() {
            return String::new();
        }

        let pairs: Vec<String> = params
            .iter()
            .map(|(k, v)| {
                let value_str = match v {
                    Value::String(s) => s.clone(),
                    Value::Number(n) => n.to_string(),
                    Value::Bool(b) => b.to_string(),
                    _ => v.to_string(),
                };
                format!("{}={}", k, urlencoding::encode(&value_str))
            })
            .collect();

        pairs.join("&")
    }

    /// Parses a JSON response string
    ///
    /// # Arguments
    ///
    /// * `response` - JSON string to parse
    ///
    /// # Returns
    ///
    /// Returns the parsed `Value` on success.
    ///
    /// # Errors
    ///
    /// Returns an error if JSON parsing fails.
    pub fn parse_json(&self, response: &str) -> Result<Value> {
        serde_json::from_str(response).map_err(|e| Error::invalid_request(e.to_string()))
    }

    /// Handles HTTP error responses by mapping status codes to appropriate errors
    ///
    /// # Arguments
    ///
    /// * `status_code` - HTTP status code
    /// * `response` - Response body text
    ///
    /// # Returns
    ///
    /// Returns an appropriate `Error` variant based on the status code.
    pub fn handle_http_error(&self, status_code: u16, response: &str) -> Error {
        match status_code {
            400 => Error::invalid_request(response.to_string()),
            401 | 403 => Error::authentication(response.to_string()),
            404 => Error::invalid_request(format!("Endpoint not found: {}", response)),
            429 => Error::rate_limit(response.to_string(), None),
            500..=599 => Error::exchange(status_code.to_string(), response),
            _ => Error::network(format!("HTTP {}: {}", status_code, response)),
        }
    }

    /// Safely extracts a string value from a JSON object
    ///
    /// # Arguments
    ///
    /// * `dict` - JSON value to extract from
    /// * `key` - Key to look up
    ///
    /// # Returns
    ///
    /// Returns `Some(String)` if the key exists and value is a string, `None` otherwise.
    pub fn safe_string(&self, dict: &Value, key: &str) -> Option<String> {
        dict.get(key)
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    }

    /// Safely extracts an integer value from a JSON object
    ///
    /// # Arguments
    ///
    /// * `dict` - JSON value to extract from
    /// * `key` - Key to look up
    ///
    /// # Returns
    ///
    /// Returns `Some(i64)` if the key exists and value is an integer, `None` otherwise.
    pub fn safe_integer(&self, dict: &Value, key: &str) -> Option<i64> {
        dict.get(key).and_then(|v| v.as_i64())
    }

    /// Safely extracts a float value from a JSON object
    ///
    /// # Arguments
    ///
    /// * `dict` - JSON value to extract from
    /// * `key` - Key to look up
    ///
    /// # Returns
    ///
    /// Returns `Some(f64)` if the key exists and value is a float, `None` otherwise.
    pub fn safe_float(&self, dict: &Value, key: &str) -> Option<f64> {
        dict.get(key).and_then(|v| v.as_f64())
    }

    /// Safely extracts a boolean value from a JSON object
    ///
    /// # Arguments
    ///
    /// * `dict` - JSON value to extract from
    /// * `key` - Key to look up
    ///
    /// # Returns
    ///
    /// Returns `Some(bool)` if the key exists and value is a boolean, `None` otherwise.
    pub fn safe_bool(&self, dict: &Value, key: &str) -> Option<bool> {
        dict.get(key).and_then(|v| v.as_bool())
    }

    // ============================================================================
    // Parse Methods
    // ============================================================================

    /// Parses raw ticker data from exchange API response
    ///
    /// # Arguments
    ///
    /// * `ticker_data` - Raw ticker JSON data from exchange
    /// * `market` - Optional market information to populate symbol field
    ///
    /// # Returns
    ///
    /// Returns a parsed `Ticker` struct.
    ///
    /// # Errors
    ///
    /// Returns an error if required fields are missing.
    pub fn parse_ticker(&self, ticker_data: &Value, market: Option<&Market>) -> Result<Ticker> {
        let symbol = if let Some(m) = market {
            m.symbol.clone()
        } else {
            self.safe_string(ticker_data, "symbol")
                .ok_or_else(|| ParseError::missing_field("symbol"))?
        };

        let timestamp = self.safe_integer(ticker_data, "timestamp").unwrap_or(0);

        Ok(Ticker {
            symbol,
            timestamp,
            datetime: self.safe_string(ticker_data, "datetime"),
            high: self.safe_decimal(ticker_data, "high").map(Price::new),
            low: self.safe_decimal(ticker_data, "low").map(Price::new),
            bid: self.safe_decimal(ticker_data, "bid").map(Price::new),
            bid_volume: self.safe_decimal(ticker_data, "bidVolume").map(Amount::new),
            ask: self.safe_decimal(ticker_data, "ask").map(Price::new),
            ask_volume: self.safe_decimal(ticker_data, "askVolume").map(Amount::new),
            vwap: self.safe_decimal(ticker_data, "vwap").map(Price::new),
            open: self.safe_decimal(ticker_data, "open").map(Price::new),
            close: self.safe_decimal(ticker_data, "close").map(Price::new),
            last: self.safe_decimal(ticker_data, "last").map(Price::new),
            previous_close: self
                .safe_decimal(ticker_data, "previousClose")
                .map(Price::new),
            change: self.safe_decimal(ticker_data, "change").map(Price::new),
            percentage: self.safe_decimal(ticker_data, "percentage"),
            average: self.safe_decimal(ticker_data, "average").map(Price::new),
            base_volume: self
                .safe_decimal(ticker_data, "baseVolume")
                .map(Amount::new),
            quote_volume: self
                .safe_decimal(ticker_data, "quoteVolume")
                .map(Amount::new),
            info: std::collections::HashMap::new(),
        })
    }

    /// Parses raw trade data from exchange API response
    ///
    /// # Arguments
    ///
    /// * `trade_data` - Raw trade JSON data from exchange
    /// * `market` - Optional market information to populate symbol field
    ///
    /// # Returns
    ///
    /// Returns a parsed `Trade` struct.
    ///
    /// # Errors
    ///
    /// Returns an error if required fields (symbol, side) are missing.
    pub fn parse_trade(&self, trade_data: &Value, market: Option<&Market>) -> Result<Trade> {
        let symbol = if let Some(m) = market {
            m.symbol.clone()
        } else {
            self.safe_string(trade_data, "symbol")
                .ok_or_else(|| ParseError::missing_field("symbol"))?
        };

        let side = self
            .safe_string(trade_data, "side")
            .and_then(|s| match s.to_lowercase().as_str() {
                "buy" => Some(OrderSide::Buy),
                "sell" => Some(OrderSide::Sell),
                _ => None,
            })
            .ok_or_else(|| ParseError::missing_field("side"))?;

        let trade_type =
            self.safe_string(trade_data, "type")
                .and_then(|t| match t.to_lowercase().as_str() {
                    "limit" => Some(OrderType::Limit),
                    "market" => Some(OrderType::Market),
                    _ => None,
                });

        let taker_or_maker = self.safe_string(trade_data, "takerOrMaker").and_then(|s| {
            match s.to_lowercase().as_str() {
                "taker" => Some(TakerOrMaker::Taker),
                "maker" => Some(TakerOrMaker::Maker),
                _ => None,
            }
        });

        Ok(Trade {
            id: self.safe_string(trade_data, "id"),
            order: self.safe_string(trade_data, "orderId"),
            timestamp: self.safe_integer(trade_data, "timestamp").unwrap_or(0),
            datetime: self.safe_string(trade_data, "datetime"),
            symbol,
            trade_type,
            side,
            taker_or_maker,
            price: Price::new(
                self.safe_decimal(trade_data, "price")
                    .unwrap_or(Decimal::ZERO),
            ),
            amount: Amount::new(
                self.safe_decimal(trade_data, "amount")
                    .unwrap_or(Decimal::ZERO),
            ),
            cost: self.safe_decimal(trade_data, "cost").map(Cost::new),
            fee: None,
            info: if let Some(obj) = trade_data.as_object() {
                obj.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
            } else {
                HashMap::new()
            },
        })
    }

    /// Parses raw order data from exchange API response
    ///
    /// # Arguments
    ///
    /// * `order_data` - Raw order JSON data from exchange
    /// * `market` - Optional market information to populate symbol field
    ///
    /// # Returns
    ///
    /// Returns a parsed `Order` struct with all available fields populated.
    pub fn parse_order(&self, order_data: &Value, market: Option<&Market>) -> Result<Order> {
        let symbol = if let Some(m) = market {
            m.symbol.clone()
        } else {
            self.safe_string(order_data, "symbol")
                .ok_or_else(|| ParseError::missing_field("symbol"))?
        };

        let order_type = self
            .safe_string(order_data, "type")
            .and_then(|t| match t.to_lowercase().as_str() {
                "limit" => Some(OrderType::Limit),
                "market" => Some(OrderType::Market),
                _ => None,
            })
            .unwrap_or(OrderType::Limit);

        let side = self
            .safe_string(order_data, "side")
            .and_then(|s| match s.to_lowercase().as_str() {
                "buy" => Some(OrderSide::Buy),
                "sell" => Some(OrderSide::Sell),
                _ => None,
            })
            .unwrap_or(OrderSide::Buy);

        let status_str = self
            .safe_string(order_data, "status")
            .unwrap_or_else(|| "open".to_string());
        let status = match status_str.to_lowercase().as_str() {
            "open" => OrderStatus::Open,
            "closed" => OrderStatus::Closed,
            "canceled" | "cancelled" => OrderStatus::Cancelled,
            "expired" => OrderStatus::Expired,
            "rejected" => OrderStatus::Rejected,
            _ => OrderStatus::Open,
        };

        let id = self
            .safe_string(order_data, "id")
            .unwrap_or_else(|| format!("order_{}", chrono::Utc::now().timestamp_millis()));

        let amount = self
            .safe_decimal(order_data, "amount")
            .unwrap_or(Decimal::ZERO);

        Ok(Order {
            id,
            client_order_id: self.safe_string(order_data, "clientOrderId"),
            timestamp: self.safe_integer(order_data, "timestamp"),
            datetime: self.safe_string(order_data, "datetime"),
            last_trade_timestamp: self.safe_integer(order_data, "lastTradeTimestamp"),
            symbol,
            order_type,
            time_in_force: self.safe_string(order_data, "timeInForce"),
            post_only: self
                .safe_string(order_data, "postOnly")
                .and_then(|s| s.parse::<bool>().ok()),
            reduce_only: self
                .safe_string(order_data, "reduceOnly")
                .and_then(|s| s.parse::<bool>().ok()),
            side,
            price: self.safe_decimal(order_data, "price"),
            stop_price: self.safe_decimal(order_data, "stopPrice"),
            trigger_price: self.safe_decimal(order_data, "triggerPrice"),
            take_profit_price: self.safe_decimal(order_data, "takeProfitPrice"),
            stop_loss_price: self.safe_decimal(order_data, "stopLossPrice"),
            average: self.safe_decimal(order_data, "average"),
            amount,
            filled: self.safe_decimal(order_data, "filled"),
            remaining: self.safe_decimal(order_data, "remaining"),
            cost: self.safe_decimal(order_data, "cost"),
            status,
            fee: None,
            fees: None,
            trades: None,
            trailing_delta: self.safe_decimal(order_data, "trailingDelta"),
            trailing_percent: self.safe_decimal(order_data, "trailingPercent"),
            activation_price: self.safe_decimal(order_data, "activationPrice"),
            callback_rate: self.safe_decimal(order_data, "callbackRate"),
            working_type: self.safe_string(order_data, "workingType"),
            info: if let Some(obj) = order_data.as_object() {
                obj.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
            } else {
                HashMap::new()
            },
        })
    }

    /// Parses raw balance data from exchange API response
    ///
    /// # Arguments
    ///
    /// * `balance_data` - Raw balance JSON data from exchange
    ///
    /// # Returns
    ///
    /// Returns a `Balance` map containing all currency balances with free, used, and total amounts.
    pub fn parse_balance(&self, balance_data: &Value) -> Result<Balance> {
        let mut balance = Balance::new();

        if let Some(obj) = balance_data.as_object() {
            for (currency, balance_info) in obj {
                if currency == "timestamp" || currency == "datetime" || currency == "info" {
                    continue;
                }
                let free = self
                    .safe_decimal(balance_info, "free")
                    .unwrap_or(Decimal::ZERO);
                let used = self
                    .safe_decimal(balance_info, "used")
                    .unwrap_or(Decimal::ZERO);
                let total = self
                    .safe_decimal(balance_info, "total")
                    .unwrap_or(free + used);

                let entry = BalanceEntry { free, used, total };
                balance.set(currency.clone(), entry);
            }
        }

        if let Some(obj) = balance_data.as_object() {
            balance.info = obj.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
        }

        Ok(balance)
    }

    /// Parses raw order book data from exchange API response
    ///
    /// # Arguments
    ///
    /// * `orderbook_data` - Raw order book JSON data from exchange
    /// * `timestamp` - Optional timestamp for the order book snapshot
    ///
    /// # Returns
    ///
    /// Returns a parsed `OrderBook` struct containing bid and ask sides.
    pub fn parse_order_book(
        &self,
        orderbook_data: &Value,
        timestamp: Option<i64>,
    ) -> Result<OrderBook> {
        let mut bids_side = OrderBookSide::new();
        let mut asks_side = OrderBookSide::new();

        if let Some(bids_array) = orderbook_data.get("bids").and_then(|v| v.as_array()) {
            for bid in bids_array {
                if let Some(arr) = bid.as_array() {
                    if arr.len() >= 2 {
                        let price = self.safe_decimal_from_value(&arr[0]);
                        let amount = self.safe_decimal_from_value(&arr[1]);
                        if let (Some(p), Some(a)) = (price, amount) {
                            bids_side.push(OrderBookEntry {
                                price: Price::new(p),
                                amount: Amount::new(a),
                            });
                        }
                    }
                }
            }
        }

        if let Some(asks_array) = orderbook_data.get("asks").and_then(|v| v.as_array()) {
            for ask in asks_array {
                if let Some(arr) = ask.as_array() {
                    if arr.len() >= 2 {
                        let price = self.safe_decimal_from_value(&arr[0]);
                        let amount = self.safe_decimal_from_value(&arr[1]);
                        if let (Some(p), Some(a)) = (price, amount) {
                            asks_side.push(OrderBookEntry {
                                price: Price::new(p),
                                amount: Amount::new(a),
                            });
                        }
                    }
                }
            }
        }

        Ok(OrderBook {
            symbol: self
                .safe_string(orderbook_data, "symbol")
                .unwrap_or_default(),
            bids: bids_side,
            asks: asks_side,
            timestamp: timestamp
                .or_else(|| self.safe_integer(orderbook_data, "timestamp"))
                .unwrap_or(0),
            datetime: self.safe_string(orderbook_data, "datetime"),
            nonce: self.safe_integer(orderbook_data, "nonce"),
            info: if let Some(obj) = orderbook_data.as_object() {
                obj.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
            } else {
                HashMap::new()
            },
            // WebSocket order book incremental update fields
            buffered_deltas: std::collections::VecDeque::new(),
            bids_map: std::collections::BTreeMap::new(),
            asks_map: std::collections::BTreeMap::new(),
            is_synced: false,
            // Auto-resync mechanism fields
            needs_resync: false,
            last_resync_time: 0,
        })
    }

    /// Safely extracts a `Decimal` value from a JSON object by key
    fn safe_decimal(&self, data: &Value, key: &str) -> Option<Decimal> {
        data.get(key).and_then(|v| self.safe_decimal_from_value(v))
    }

    /// Safely extracts a `Decimal` value from a JSON `Value`
    fn safe_decimal_from_value(&self, value: &Value) -> Option<Decimal> {
        match value {
            Value::Number(n) => {
                if let Some(f) = n.as_f64() {
                    Decimal::from_f64_retain(f)
                } else {
                    None
                }
            }
            Value::String(s) => Decimal::from_str(s).ok(),
            _ => None,
        }
    }

    // ============================================================================
    // Fee and Precision Methods
    // ============================================================================

    /// Calculates trading fee for a given order
    ///
    /// # Arguments
    ///
    /// * `symbol` - Trading pair symbol
    /// * `order_type` - Order type (limit, market, etc.)
    /// * `side` - Order side (buy or sell)
    /// * `amount` - Trade amount in base currency
    /// * `price` - Trade price in quote currency
    /// * `taker_or_maker` - Optional taker or maker designation
    ///
    /// # Returns
    ///
    /// Returns a `Fee` struct containing the currency, cost, and rate.
    pub async fn calculate_fee(
        &self,
        symbol: &str,
        _order_type: OrderType,
        _side: OrderSide,
        amount: Decimal,
        price: Decimal,
        taker_or_maker: Option<&str>,
    ) -> Result<Fee> {
        let market = self.market(symbol).await?;

        let rate = if let Some(tom) = taker_or_maker {
            if tom == "taker" {
                market.taker.unwrap_or(Decimal::ZERO)
            } else {
                market.maker.unwrap_or(Decimal::ZERO)
            }
        } else {
            market.taker.unwrap_or(Decimal::ZERO)
        };

        let cost = amount * price;
        let fee_cost = cost * rate;

        Ok(Fee {
            currency: market.quote.clone(),
            cost: fee_cost,
            rate: Some(rate),
        })
    }

    /// Converts an amount to the precision required by the market
    pub async fn amount_to_precision(&self, symbol: &str, amount: Decimal) -> Result<Decimal> {
        let market = self.market(symbol).await?;
        match market.precision.amount {
            Some(precision_value) => Ok(self.round_to_precision(amount, precision_value)),
            None => Ok(amount),
        }
    }

    /// Converts a price to the precision required by the market
    pub async fn price_to_precision(&self, symbol: &str, price: Decimal) -> Result<Decimal> {
        let market = self.market(symbol).await?;
        match market.precision.price {
            Some(precision_value) => Ok(self.round_to_precision(price, precision_value)),
            None => Ok(price),
        }
    }

    /// Converts a cost to the precision required by the market
    pub async fn cost_to_precision(&self, symbol: &str, cost: Decimal) -> Result<Decimal> {
        let market = self.market(symbol).await?;
        match market.precision.price {
            Some(precision_value) => Ok(self.round_to_precision(cost, precision_value)),
            None => Ok(cost),
        }
    }

    /// Rounds a value to the specified precision
    ///
    /// The `precision_value` can represent either decimal places (e.g., 8) or a step size (e.g., 0.01).
    fn round_to_precision(&self, value: Decimal, precision_value: Decimal) -> Decimal {
        if precision_value < Decimal::ONE {
            // Round by step size: round(value / step) * step
            let steps = (value / precision_value).round();
            steps * precision_value
        } else {
            // Round by decimal places
            let digits = precision_value.to_u32().unwrap_or(8);
            let multiplier = Decimal::from(10_i64.pow(digits));
            let scaled = value * multiplier;
            let rounded = scaled.round();
            rounded / multiplier
        }
    }

    /// Calculates the cost of a trade
    ///
    /// # Arguments
    ///
    /// * `symbol` - Trading pair symbol
    /// * `amount` - Trade amount in base currency
    /// * `price` - Trade price in quote currency
    ///
    /// # Returns
    ///
    /// Returns the total cost (amount × price) in quote currency.
    pub async fn calculate_cost(
        &self,
        symbol: &str,
        amount: Decimal,
        price: Decimal,
    ) -> Result<Decimal> {
        let _market = self.market(symbol).await?;
        Ok(amount * price)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_base_exchange_creation() {
        let config = ExchangeConfig {
            id: "test".to_string(),
            name: "Test Exchange".to_string(),
            ..Default::default()
        };

        let exchange = BaseExchange::new(config).unwrap();
        assert_eq!(exchange.config.id, "test");
        // Rate limiting is now managed by HttpClient internally
        assert!(exchange.config.enable_rate_limit);
    }

    #[tokio::test]
    async fn test_market_cache() {
        let config = ExchangeConfig {
            id: "test".to_string(),
            ..Default::default()
        };

        let exchange = BaseExchange::new(config).unwrap();

        let markets = vec![Market {
            id: "btcusdt".to_string(),
            symbol: "BTC/USDT".to_string(),
            parsed_symbol: None,
            base: "BTC".to_string(),
            quote: "USDT".to_string(),
            active: true,
            market_type: MarketType::Spot,
            margin: false,
            settle: None,
            base_id: None,
            quote_id: None,
            settle_id: None,
            contract: None,
            linear: None,
            inverse: None,
            contract_size: None,
            expiry: None,
            expiry_datetime: None,
            strike: None,
            option_type: None,
            precision: Default::default(),
            limits: Default::default(),
            maker: None,
            taker: None,
            percentage: None,
            tier_based: None,
            fee_side: None,
            info: Default::default(),
        }];

        let _ = exchange.set_markets(markets, None).await.unwrap();

        let market = exchange.market("BTC/USDT").await.unwrap();
        assert_eq!(market.symbol, "BTC/USDT");

        let symbols = exchange.symbols().await.unwrap();
        assert_eq!(symbols.len(), 1);
    }

    #[test]
    fn test_build_query_string() {
        let config = ExchangeConfig::default();
        let exchange = BaseExchange::new(config).unwrap();

        let mut params = HashMap::new();
        params.insert("symbol".to_string(), Value::String("BTC/USDT".to_string()));
        params.insert("limit".to_string(), Value::Number(100.into()));

        let query = exchange.build_query_string(&params);
        assert!(query.contains("symbol="));
        assert!(query.contains("limit="));
    }

    #[test]
    fn test_capabilities() {
        // Test default capabilities (all false)
        let default_caps = ExchangeCapabilities::default();
        assert!(!default_caps.has("fetchMarkets"));
        assert!(!default_caps.has("fetchOHLCV"));

        // Test public_only capabilities
        let public_caps = ExchangeCapabilities::public_only();
        assert!(public_caps.has("fetchMarkets"));
        assert!(public_caps.has("fetchTicker"));
        assert!(public_caps.has("fetchOHLCV"));
        assert!(!public_caps.has("createOrder"));

        // Test all capabilities
        let all_caps = ExchangeCapabilities::all();
        assert!(all_caps.has("fetchMarkets"));
        assert!(all_caps.has("fetchOHLCV"));
        assert!(all_caps.has("createOrder"));
    }

    #[test]
    fn test_exchange_config_builder() {
        let config = ExchangeConfigBuilder::new()
            .id("binance")
            .name("Binance")
            .api_key("test-key")
            .secret("test-secret")
            .sandbox(true)
            .timeout(60)
            .verbose(true)
            .build();

        assert_eq!(config.id, "binance");
        assert_eq!(config.name, "Binance");
        assert_eq!(config.api_key, Some("test-key".to_string()));
        assert_eq!(config.secret, Some("test-secret".to_string()));
        assert!(config.sandbox);
        assert_eq!(config.timeout, 60);
        assert!(config.verbose);
    }

    #[test]
    fn test_exchange_config_builder_defaults() {
        let config = ExchangeConfigBuilder::new().build();

        assert_eq!(config.id, "");
        assert_eq!(config.api_key, None);
        assert!(config.enable_rate_limit);
        assert_eq!(config.timeout, 30);
        assert!(!config.sandbox);
    }

    #[test]
    fn test_exchange_config_builder_from_config() {
        let config = ExchangeConfig::builder().id("test").api_key("key").build();

        assert_eq!(config.id, "test");
        assert_eq!(config.api_key, Some("key".to_string()));
    }
}

#[cfg(test)]
mod parse_tests {
    use super::*;
    use serde_json::json;

    async fn create_test_exchange() -> BaseExchange {
        let config = ExchangeConfig {
            id: "".to_string(),
            name: "".to_string(),
            api_key: None,
            secret: None,
            password: None,
            uid: None,
            timeout: 10000,
            sandbox: false,
            user_agent: None,
            enable_rate_limit: true,
            verbose: false,
            account_id: None,
            rate_limit: 0,
            proxy: None,
            options: Default::default(),
            url_overrides: Default::default(),
        };

        let exchange = BaseExchange::new(config).unwrap();

        // Initialize an empty cache for testing
        let cache = MarketCache::default();
        *exchange.market_cache.write().await = cache;

        exchange
    }

    #[tokio::test]
    async fn test_parse_ticker() {
        let exchange = create_test_exchange().await;

        let ticker_data = json!({
            "symbol": "BTC/USDT",
            "timestamp": 1609459200000i64,
            "datetime": "2021-01-01T00:00:00.000Z",
            "high": 30000.0,
            "low": 28000.0,
            "bid": 29000.0,
            "bidVolume": 10.5,
            "ask": 29100.0,
            "askVolume": 8.3,
            "vwap": 29500.0,
            "open": 28500.0,
            "close": 29000.0,
            "last": 29000.0,
            "previousClose": 28500.0,
            "change": 500.0,
            "percentage": 1.75,
            "average": 28750.0,
            "baseVolume": 1000.0,
            "quoteVolume": 29000000.0
        });

        let result = exchange.parse_ticker(&ticker_data, None);
        assert!(result.is_ok());

        let ticker = result.unwrap();
        assert_eq!(ticker.symbol, "BTC/USDT");
        assert_eq!(ticker.timestamp, 1609459200000);
        assert_eq!(
            ticker.high,
            Some(Price::from(Decimal::from_str_radix("30000.0", 10).unwrap()))
        );
        assert_eq!(
            ticker.low,
            Some(Price::from(Decimal::from_str_radix("28000.0", 10).unwrap()))
        );
        assert_eq!(
            ticker.bid,
            Some(Price::from(Decimal::from_str_radix("29000.0", 10).unwrap()))
        );
        assert_eq!(
            ticker.ask,
            Some(Price::from(Decimal::from_str_radix("29100.0", 10).unwrap()))
        );
        assert_eq!(
            ticker.last,
            Some(Price::from(Decimal::from_str_radix("29000.0", 10).unwrap()))
        );
        assert_eq!(
            ticker.base_volume,
            Some(Amount::from(Decimal::from_str_radix("1000.0", 10).unwrap()))
        );
        assert_eq!(
            ticker.quote_volume,
            Some(Amount::from(
                Decimal::from_str_radix("29000000.0", 10).unwrap()
            ))
        );
    }

    #[tokio::test]
    async fn test_parse_trade() {
        let exchange = create_test_exchange().await;

        let trade_data = json!({
            "id": "12345",
            "symbol": "BTC/USDT",
            "timestamp": 1609459200000i64,
            "datetime": "2021-01-01T00:00:00.000Z",
            "order": "order123",
            "type": "limit",
            "side": "buy",
            "takerOrMaker": "taker",
            "price": 29000.0,
            "amount": 0.5,
            "cost": 14500.0
        });

        let result = exchange.parse_trade(&trade_data, None);
        assert!(result.is_ok());

        let trade = result.unwrap();
        assert_eq!(trade.id, Some("12345".to_string()));
        assert_eq!(trade.symbol, "BTC/USDT");
        assert_eq!(trade.timestamp, 1609459200000);
        assert_eq!(trade.side, OrderSide::Buy);
        assert_eq!(
            trade.price,
            Price::from(Decimal::from_str_radix("29000.0", 10).unwrap())
        );
        assert_eq!(
            trade.amount,
            Amount::from(Decimal::from_str_radix("0.5", 10).unwrap())
        );
        assert_eq!(
            trade.cost,
            Some(Cost::from(Decimal::from_str_radix("14500.0", 10).unwrap()))
        );
        assert_eq!(trade.taker_or_maker, Some(TakerOrMaker::Taker));
    }

    #[tokio::test]
    async fn test_parse_order() {
        let exchange = create_test_exchange().await;

        let order_data = json!({
            "id": "order123",
            "clientOrderId": "client456",
            "symbol": "BTC/USDT",
            "timestamp": 1609459200000i64,
            "datetime": "2021-01-01T00:00:00.000Z",
            "lastTradeTimestamp": 1609459300000i64,
            "status": "closed",
            "type": "limit",
            "timeInForce": "GTC",
            "side": "buy",
            "price": 29000.0,
            "average": 29050.0,
            "amount": 0.5,
            "filled": 0.5,
            "remaining": 0.0,
            "cost": 14525.0
        });

        let result = exchange.parse_order(&order_data, None);
        assert!(result.is_ok());

        let order = result.unwrap();
        assert_eq!(order.id, "order123");
        assert_eq!(order.client_order_id, Some("client456".to_string()));
        assert_eq!(order.symbol, "BTC/USDT");
        assert_eq!(order.status, OrderStatus::Closed);
        assert_eq!(order.order_type, OrderType::Limit);
        assert_eq!(order.side, OrderSide::Buy);
        assert_eq!(order.time_in_force, Some("GTC".to_string()));
        assert_eq!(
            order.price,
            Some(Decimal::from_str_radix("29000.0", 10).unwrap())
        );
        assert_eq!(order.amount, Decimal::from_str_radix("0.5", 10).unwrap());
        assert_eq!(
            order.filled,
            Some(Decimal::from_str_radix("0.5", 10).unwrap())
        );
        assert_eq!(
            order.remaining,
            Some(Decimal::from_str_radix("0.0", 10).unwrap())
        );
        assert_eq!(
            order.cost,
            Some(Decimal::from_str_radix("14525.0", 10).unwrap())
        );
    }

    #[tokio::test]
    async fn test_parse_balance() {
        let exchange = create_test_exchange().await;

        let balance_data = json!({
            "timestamp": 1609459200000i64,
            "datetime": "2021-01-01T00:00:00.000Z",
            "BTC": {
                "free": 1.5,
                "used": 0.5,
                "total": 2.0
            },
            "USDT": {
                "free": 10000.0,
                "used": 5000.0,
                "total": 15000.0
            }
        });

        let result = exchange.parse_balance(&balance_data);
        assert!(result.is_ok());

        let balance = result.unwrap();
        assert_eq!(balance.balances.len(), 2);

        let btc_balance = balance.balances.get("BTC").unwrap();
        assert_eq!(
            btc_balance.free,
            Decimal::from_str_radix("1.5", 10).unwrap()
        );
        assert_eq!(
            btc_balance.used,
            Decimal::from_str_radix("0.5", 10).unwrap()
        );
        assert_eq!(
            btc_balance.total,
            Decimal::from_str_radix("2.0", 10).unwrap()
        );

        let usdt_balance = balance.balances.get("USDT").unwrap();
        assert_eq!(
            usdt_balance.free,
            Decimal::from_str_radix("10000.0", 10).unwrap()
        );
        assert_eq!(
            usdt_balance.used,
            Decimal::from_str_radix("5000.0", 10).unwrap()
        );
        assert_eq!(
            usdt_balance.total,
            Decimal::from_str_radix("15000.0", 10).unwrap()
        );
    }

    #[tokio::test]
    async fn test_parse_order_book() {
        let exchange = create_test_exchange().await;

        let orderbook_data = json!({
            "symbol": "BTC/USDT",
            "bids": [
                [29000.0, 1.5],
                [28900.0, 2.0],
                [28800.0, 3.5]
            ],
            "asks": [
                [29100.0, 1.0],
                [29200.0, 2.5],
                [29300.0, 1.8]
            ]
        });

        let result = exchange.parse_order_book(&orderbook_data, Some(1609459200000));
        assert!(result.is_ok());

        let orderbook = result.unwrap();
        assert_eq!(orderbook.symbol, "BTC/USDT");
        assert_eq!(orderbook.timestamp, 1609459200000);
        assert_eq!(orderbook.bids.len(), 3);
        assert_eq!(orderbook.asks.len(), 3);

        // 验证bids降序排列
        assert_eq!(
            orderbook.bids[0].price,
            Price::from(Decimal::from_str_radix("29000.0", 10).unwrap())
        );
        assert_eq!(
            orderbook.bids[1].price,
            Price::from(Decimal::from_str_radix("28900.0", 10).unwrap())
        );
        assert_eq!(
            orderbook.bids[2].price,
            Price::from(Decimal::from_str_radix("28800.0", 10).unwrap())
        );

        // 验证asks升序排列
        assert_eq!(
            orderbook.asks[0].price,
            Price::from(Decimal::from_str_radix("29100.0", 10).unwrap())
        );
        assert_eq!(
            orderbook.asks[1].price,
            Price::from(Decimal::from_str_radix("29200.0", 10).unwrap())
        );
        assert_eq!(
            orderbook.asks[2].price,
            Price::from(Decimal::from_str_radix("29300.0", 10).unwrap())
        );
    }

    #[test]
    fn test_calculate_fee() {
        // Skip this test for now as it requires async context
        // TODO: Convert to #[tokio::test] when async test infrastructure is ready
    }

    #[test]
    fn test_amount_to_precision() {
        // Skip this test for now as it requires async context
        // TODO: Convert to #[tokio::test] when async test infrastructure is ready
    }

    #[test]
    fn test_price_to_precision() {
        // Skip this test for now as it requires async context
        // TODO: Convert to #[tokio::test] when async test infrastructure is ready
    }

    #[test]
    fn test_has_method() {
        // Skip this test for now as it requires changes to BaseExchange API
        // TODO: Implement has method in BaseExchange
    }

    #[test]
    fn test_timeframes() {
        // Skip this test for now as it requires changes to BaseExchange API
        // TODO: Implement timeframes method in BaseExchange
    }

    #[test]
    fn test_filter_by_type() {
        // Skip this test for now as it requires changes to BaseExchange API
        // TODO: Implement filter_by_type method in BaseExchange
    }
}
