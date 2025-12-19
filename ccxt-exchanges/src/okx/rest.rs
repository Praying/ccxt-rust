//! OKX REST API implementation.
//!
//! Implements all REST API endpoint operations for the OKX exchange.

use super::{Okx, OkxAuth, error, parser};
use ccxt_core::{
    Error, ParseError, Result,
    types::{Balance, Market, OHLCV, Order, OrderBook, OrderSide, OrderType, Ticker, Trade},
};
use reqwest::header::{HeaderMap, HeaderValue};
use serde_json::Value;
use std::collections::HashMap;
use tracing::{debug, info, warn};

impl Okx {
    // ============================================================================
    // Helper Methods
    // ============================================================================

    /// Get the current timestamp in ISO 8601 format for OKX API.
    fn get_timestamp(&self) -> String {
        chrono::Utc::now()
            .format("%Y-%m-%dT%H:%M:%S%.3fZ")
            .to_string()
    }

    /// Get the authentication instance if credentials are configured.
    fn get_auth(&self) -> Result<OkxAuth> {
        let config = &self.base().config;

        let api_key = config
            .api_key
            .as_ref()
            .ok_or_else(|| Error::authentication("API key is required"))?;
        let secret = config
            .secret
            .as_ref()
            .ok_or_else(|| Error::authentication("API secret is required"))?;
        let passphrase = config
            .password
            .as_ref()
            .ok_or_else(|| Error::authentication("Passphrase is required"))?;

        Ok(OkxAuth::new(
            api_key.clone(),
            secret.clone(),
            passphrase.clone(),
        ))
    }

    /// Check that required credentials are configured.
    pub fn check_required_credentials(&self) -> Result<()> {
        self.base().check_required_credentials()?;
        if self.base().config.password.is_none() {
            return Err(Error::authentication("Passphrase is required for OKX"));
        }
        Ok(())
    }

    /// Build the API path for OKX V5 API.
    fn build_api_path(&self, endpoint: &str) -> String {
        format!("/api/v5{}", endpoint)
    }

    /// Get the instrument type for API requests.
    ///
    /// Maps the configured `default_type` to OKX's instrument type (instType) parameter.
    /// OKX uses a unified V5 API, so this primarily affects market filtering.
    ///
    /// # Returns
    ///
    /// Returns the OKX instrument type string:
    /// - "SPOT" for spot trading
    /// - "MARGIN" for margin trading
    /// - "SWAP" for perpetual contracts
    /// - "FUTURES" for delivery contracts
    /// - "OPTION" for options trading
    pub fn get_inst_type(&self) -> &str {
        use ccxt_core::types::default_type::DefaultType;

        match self.options().default_type {
            DefaultType::Spot => "SPOT",
            DefaultType::Margin => "MARGIN",
            DefaultType::Swap => "SWAP",
            DefaultType::Futures => "FUTURES",
            DefaultType::Option => "OPTION",
        }
    }

    /// Make a public API request (no authentication required).
    async fn public_request(
        &self,
        method: &str,
        path: &str,
        params: Option<&HashMap<String, String>>,
    ) -> Result<Value> {
        let urls = self.urls();
        let mut url = format!("{}{}", urls.rest, path);

        if let Some(p) = params {
            if !p.is_empty() {
                let query: Vec<String> = p
                    .iter()
                    .map(|(k, v)| format!("{}={}", k, urlencoding::encode(v)))
                    .collect();
                url = format!("{}?{}", url, query.join("&"));
            }
        }

        debug!("OKX public request: {} {}", method, url);

        // Add demo trading header if in sandbox mode
        let mut headers = HeaderMap::new();
        if self.is_testnet_trading() {
            headers.insert("x-simulated-trading", HeaderValue::from_static("1"));
        }

        let response = match method.to_uppercase().as_str() {
            "GET" => {
                if headers.is_empty() {
                    self.base().http_client.get(&url, None).await?
                } else {
                    self.base().http_client.get(&url, Some(headers)).await?
                }
            }
            "POST" => {
                if headers.is_empty() {
                    self.base().http_client.post(&url, None, None).await?
                } else {
                    self.base()
                        .http_client
                        .post(&url, Some(headers), None)
                        .await?
                }
            }
            _ => {
                return Err(Error::invalid_request(format!(
                    "Unsupported HTTP method: {}",
                    method
                )));
            }
        };

        // Check for OKX error response
        if error::is_error_response(&response) {
            return Err(error::parse_error(&response));
        }

        Ok(response)
    }

    /// Make a private API request (authentication required).
    async fn private_request(
        &self,
        method: &str,
        path: &str,
        params: Option<&HashMap<String, String>>,
        body: Option<&Value>,
    ) -> Result<Value> {
        self.check_required_credentials()?;

        let auth = self.get_auth()?;
        let urls = self.urls();
        let timestamp = self.get_timestamp();

        // Build query string for GET requests
        let query_string = if let Some(p) = params {
            if !p.is_empty() {
                let query: Vec<String> = p
                    .iter()
                    .map(|(k, v)| format!("{}={}", k, urlencoding::encode(v)))
                    .collect();
                format!("?{}", query.join("&"))
            } else {
                String::new()
            }
        } else {
            String::new()
        };

        // Build body string for POST requests
        let body_string = body
            .map(|b| serde_json::to_string(b).unwrap_or_default())
            .unwrap_or_default();

        // Sign the request - OKX uses path with query string
        let sign_path = format!("{}{}", path, query_string);
        let signature = auth.sign(&timestamp, method, &sign_path, &body_string);

        // Build headers
        let mut headers = HeaderMap::new();
        auth.add_auth_headers(&mut headers, &timestamp, &signature);
        headers.insert("Content-Type", HeaderValue::from_static("application/json"));

        // Add demo trading header if in sandbox mode
        if self.is_testnet_trading() {
            headers.insert("x-simulated-trading", HeaderValue::from_static("1"));
        }

        let url = format!("{}{}{}", urls.rest, path, query_string);
        debug!("OKX private request: {} {}", method, url);

        let response = match method.to_uppercase().as_str() {
            "GET" => self.base().http_client.get(&url, Some(headers)).await?,
            "POST" => {
                let body_value = body.cloned();
                self.base()
                    .http_client
                    .post(&url, Some(headers), body_value)
                    .await?
            }
            "DELETE" => {
                self.base()
                    .http_client
                    .delete(&url, Some(headers), None)
                    .await?
            }
            _ => {
                return Err(Error::invalid_request(format!(
                    "Unsupported HTTP method: {}",
                    method
                )));
            }
        };

        // Check for OKX error response
        if error::is_error_response(&response) {
            return Err(error::parse_error(&response));
        }

        Ok(response)
    }

    // ============================================================================
    // Public API Methods - Market Data
    // ============================================================================

    /// Fetch all trading markets.
    ///
    /// # Returns
    ///
    /// Returns a vector of [`Market`] structures containing market information.
    ///
    /// # Errors
    ///
    /// Returns an error if the API request fails or response parsing fails.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use ccxt_exchanges::okx::Okx;
    /// # async fn example() -> ccxt_core::Result<()> {
    /// let okx = Okx::builder().build()?;
    /// let markets = okx.fetch_markets().await?;
    /// println!("Found {} markets", markets.len());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn fetch_markets(&self) -> Result<Vec<Market>> {
        let path = self.build_api_path("/public/instruments");
        let mut params = HashMap::new();
        params.insert("instType".to_string(), self.get_inst_type().to_string());

        let response = self.public_request("GET", &path, Some(&params)).await?;

        let data = response
            .get("data")
            .ok_or_else(|| Error::from(ParseError::missing_field("data")))?;

        let instruments = data.as_array().ok_or_else(|| {
            Error::from(ParseError::invalid_format(
                "data",
                "Expected array of instruments",
            ))
        })?;

        let mut markets = Vec::new();
        for instrument in instruments {
            match parser::parse_market(instrument) {
                Ok(market) => markets.push(market),
                Err(e) => {
                    warn!(error = %e, "Failed to parse market");
                }
            }
        }

        // Cache the markets and preserve ownership for the caller
        let markets = self.base().set_markets(markets, None).await?;

        info!("Loaded {} markets for OKX", markets.len());
        Ok(markets)
    }

    /// Load and cache market data.
    ///
    /// If markets are already loaded and `reload` is false, returns cached data.
    ///
    /// # Arguments
    ///
    /// * `reload` - Whether to force reload market data from the API.
    ///
    /// # Returns
    ///
    /// Returns a `HashMap` containing all market data, keyed by symbol (e.g., "BTC/USDT").
    pub async fn load_markets(&self, reload: bool) -> Result<HashMap<String, Market>> {
        // Acquire the loading lock to serialize concurrent load_markets calls
        // This prevents multiple tasks from making duplicate API calls
        let _loading_guard = self.base().market_loading_lock.lock().await;

        // Check cache status while holding the lock
        {
            let cache = self.base().market_cache.read().await;
            if cache.loaded && !reload {
                debug!(
                    "Returning cached markets for OKX ({} markets)",
                    cache.markets.len()
                );
                return Ok(cache.markets.clone());
            }
        }

        info!("Loading markets for OKX (reload: {})", reload);
        let _markets = self.fetch_markets().await?;

        let cache = self.base().market_cache.read().await;
        Ok(cache.markets.clone())
    }

    /// Fetch ticker for a single trading pair.
    ///
    /// # Arguments
    ///
    /// * `symbol` - Trading pair symbol (e.g., "BTC/USDT").
    ///
    /// # Returns
    ///
    /// Returns [`Ticker`] data for the specified symbol.
    pub async fn fetch_ticker(&self, symbol: &str) -> Result<Ticker> {
        let market = self.base().market(symbol).await?;

        let path = self.build_api_path("/market/ticker");
        let mut params = HashMap::new();
        params.insert("instId".to_string(), market.id.clone());

        let response = self.public_request("GET", &path, Some(&params)).await?;

        let data = response
            .get("data")
            .ok_or_else(|| Error::from(ParseError::missing_field("data")))?;

        // OKX returns an array even for single ticker
        let tickers = data.as_array().ok_or_else(|| {
            Error::from(ParseError::invalid_format(
                "data",
                "Expected array of tickers",
            ))
        })?;

        if tickers.is_empty() {
            return Err(Error::bad_symbol(format!("No ticker data for {}", symbol)));
        }

        parser::parse_ticker(&tickers[0], Some(&market))
    }

    /// Fetch tickers for multiple trading pairs.
    ///
    /// # Arguments
    ///
    /// * `symbols` - Optional list of trading pair symbols; fetches all if `None`.
    ///
    /// # Returns
    ///
    /// Returns a vector of [`Ticker`] structures.
    pub async fn fetch_tickers(&self, symbols: Option<Vec<String>>) -> Result<Vec<Ticker>> {
        let cache = self.base().market_cache.read().await;
        if !cache.loaded {
            drop(cache);
            return Err(Error::exchange(
                "-1",
                "Markets not loaded. Call load_markets() first.",
            ));
        }
        drop(cache);

        let path = self.build_api_path("/market/tickers");
        let mut params = HashMap::new();
        params.insert("instType".to_string(), self.get_inst_type().to_string());

        let response = self.public_request("GET", &path, Some(&params)).await?;

        let data = response
            .get("data")
            .ok_or_else(|| Error::from(ParseError::missing_field("data")))?;

        let tickers_array = data.as_array().ok_or_else(|| {
            Error::from(ParseError::invalid_format(
                "data",
                "Expected array of tickers",
            ))
        })?;

        let mut tickers = Vec::new();
        for ticker_data in tickers_array {
            if let Some(inst_id) = ticker_data["instId"].as_str() {
                let cache = self.base().market_cache.read().await;
                if let Some(market) = cache.markets_by_id.get(inst_id) {
                    let market_clone = market.clone();
                    drop(cache);

                    match parser::parse_ticker(ticker_data, Some(&market_clone)) {
                        Ok(ticker) => {
                            if let Some(ref syms) = symbols {
                                if syms.contains(&ticker.symbol) {
                                    tickers.push(ticker);
                                }
                            } else {
                                tickers.push(ticker);
                            }
                        }
                        Err(e) => {
                            warn!(
                                error = %e,
                                symbol = %inst_id,
                                "Failed to parse ticker"
                            );
                        }
                    }
                } else {
                    drop(cache);
                }
            }
        }

        Ok(tickers)
    }

    // ============================================================================
    // Public API Methods - Order Book and Trades
    // ============================================================================

    /// Fetch order book for a trading pair.
    ///
    /// # Arguments
    ///
    /// * `symbol` - Trading pair symbol.
    /// * `limit` - Optional depth limit (valid values: 1-400; default: 100).
    ///
    /// # Returns
    ///
    /// Returns [`OrderBook`] data containing bids and asks.
    pub async fn fetch_order_book(&self, symbol: &str, limit: Option<u32>) -> Result<OrderBook> {
        let market = self.base().market(symbol).await?;

        let path = self.build_api_path("/market/books");
        let mut params = HashMap::new();
        params.insert("instId".to_string(), market.id.clone());

        // OKX valid limits: 1-400, default 100
        // Cap to maximum allowed value
        let actual_limit = limit.map(|l| l.min(400)).unwrap_or(100);
        params.insert("sz".to_string(), actual_limit.to_string());

        let response = self.public_request("GET", &path, Some(&params)).await?;

        let data = response
            .get("data")
            .ok_or_else(|| Error::from(ParseError::missing_field("data")))?;

        // OKX returns array with single orderbook
        let books = data.as_array().ok_or_else(|| {
            Error::from(ParseError::invalid_format(
                "data",
                "Expected array of orderbooks",
            ))
        })?;

        if books.is_empty() {
            return Err(Error::bad_symbol(format!(
                "No orderbook data for {}",
                symbol
            )));
        }

        parser::parse_orderbook(&books[0], market.symbol.clone())
    }

    /// Fetch recent public trades.
    ///
    /// # Arguments
    ///
    /// * `symbol` - Trading pair symbol.
    /// * `limit` - Optional limit on number of trades (maximum: 500).
    ///
    /// # Returns
    ///
    /// Returns a vector of [`Trade`] structures, sorted by timestamp in descending order.
    pub async fn fetch_trades(&self, symbol: &str, limit: Option<u32>) -> Result<Vec<Trade>> {
        let market = self.base().market(symbol).await?;

        let path = self.build_api_path("/market/trades");
        let mut params = HashMap::new();
        params.insert("instId".to_string(), market.id.clone());

        // OKX maximum limit is 500
        let actual_limit = limit.map(|l| l.min(500)).unwrap_or(100);
        params.insert("limit".to_string(), actual_limit.to_string());

        let response = self.public_request("GET", &path, Some(&params)).await?;

        let data = response
            .get("data")
            .ok_or_else(|| Error::from(ParseError::missing_field("data")))?;

        let trades_array = data.as_array().ok_or_else(|| {
            Error::from(ParseError::invalid_format(
                "data",
                "Expected array of trades",
            ))
        })?;

        let mut trades = Vec::new();
        for trade_data in trades_array {
            match parser::parse_trade(trade_data, Some(&market)) {
                Ok(trade) => trades.push(trade),
                Err(e) => {
                    warn!(error = %e, "Failed to parse trade");
                }
            }
        }

        // Sort by timestamp descending (newest first)
        trades.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

        Ok(trades)
    }

    /// Fetch OHLCV (candlestick) data.
    ///
    /// # Arguments
    ///
    /// * `symbol` - Trading pair symbol.
    /// * `timeframe` - Candlestick timeframe (e.g., "1m", "5m", "1h", "1d").
    /// * `since` - Optional start timestamp in milliseconds.
    /// * `limit` - Optional limit on number of candles (maximum: 300).
    ///
    /// # Returns
    ///
    /// Returns a vector of [`OHLCV`] structures.
    pub async fn fetch_ohlcv(
        &self,
        symbol: &str,
        timeframe: &str,
        since: Option<i64>,
        limit: Option<u32>,
    ) -> Result<Vec<OHLCV>> {
        let market = self.base().market(symbol).await?;

        // Convert timeframe to OKX format
        let timeframes = self.timeframes();
        let okx_timeframe = timeframes.get(timeframe).ok_or_else(|| {
            Error::invalid_request(format!("Unsupported timeframe: {}", timeframe))
        })?;

        let path = self.build_api_path("/market/candles");
        let mut params = HashMap::new();
        params.insert("instId".to_string(), market.id.clone());
        params.insert("bar".to_string(), okx_timeframe.clone());

        // OKX maximum limit is 300
        let actual_limit = limit.map(|l| l.min(300)).unwrap_or(100);
        params.insert("limit".to_string(), actual_limit.to_string());

        if let Some(start_time) = since {
            params.insert("after".to_string(), start_time.to_string());
        }

        let response = self.public_request("GET", &path, Some(&params)).await?;

        let data = response
            .get("data")
            .ok_or_else(|| Error::from(ParseError::missing_field("data")))?;

        let candles_array = data.as_array().ok_or_else(|| {
            Error::from(ParseError::invalid_format(
                "data",
                "Expected array of candles",
            ))
        })?;

        let mut ohlcv = Vec::new();
        for candle_data in candles_array {
            match parser::parse_ohlcv(candle_data) {
                Ok(candle) => ohlcv.push(candle),
                Err(e) => {
                    warn!(error = %e, "Failed to parse OHLCV");
                }
            }
        }

        // Sort by timestamp ascending (oldest first)
        ohlcv.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));

        Ok(ohlcv)
    }

    // ============================================================================
    // Private API Methods - Account
    // ============================================================================

    /// Fetch account balances.
    ///
    /// # Returns
    ///
    /// Returns a [`Balance`] structure with all currency balances.
    ///
    /// # Errors
    ///
    /// Returns an error if authentication fails or the API request fails.
    pub async fn fetch_balance(&self) -> Result<Balance> {
        let path = self.build_api_path("/account/balance");
        let response = self.private_request("GET", &path, None, None).await?;

        let data = response
            .get("data")
            .ok_or_else(|| Error::from(ParseError::missing_field("data")))?;

        // OKX returns array with single balance object
        let balances = data.as_array().ok_or_else(|| {
            Error::from(ParseError::invalid_format(
                "data",
                "Expected array of balances",
            ))
        })?;

        if balances.is_empty() {
            return Ok(Balance {
                balances: HashMap::new(),
                info: HashMap::new(),
            });
        }

        parser::parse_balance(&balances[0])
    }

    /// Fetch user's trade history.
    ///
    /// # Arguments
    ///
    /// * `symbol` - Trading pair symbol.
    /// * `since` - Optional start timestamp in milliseconds.
    /// * `limit` - Optional limit on number of trades (maximum: 100).
    ///
    /// # Returns
    ///
    /// Returns a vector of [`Trade`] structures representing user's trade history.
    pub async fn fetch_my_trades(
        &self,
        symbol: &str,
        since: Option<i64>,
        limit: Option<u32>,
    ) -> Result<Vec<Trade>> {
        let market = self.base().market(symbol).await?;

        let path = self.build_api_path("/trade/fills");
        let mut params = HashMap::new();
        params.insert("instId".to_string(), market.id.clone());
        params.insert("instType".to_string(), self.get_inst_type().to_string());

        // OKX maximum limit is 100
        let actual_limit = limit.map(|l| l.min(100)).unwrap_or(100);
        params.insert("limit".to_string(), actual_limit.to_string());

        if let Some(start_time) = since {
            params.insert("begin".to_string(), start_time.to_string());
        }

        let response = self
            .private_request("GET", &path, Some(&params), None)
            .await?;

        let data = response
            .get("data")
            .ok_or_else(|| Error::from(ParseError::missing_field("data")))?;

        let trades_array = data.as_array().ok_or_else(|| {
            Error::from(ParseError::invalid_format(
                "data",
                "Expected array of trades",
            ))
        })?;

        let mut trades = Vec::new();
        for trade_data in trades_array {
            match parser::parse_trade(trade_data, Some(&market)) {
                Ok(trade) => trades.push(trade),
                Err(e) => {
                    warn!(error = %e, "Failed to parse my trade");
                }
            }
        }

        Ok(trades)
    }

    // ============================================================================
    // Private API Methods - Order Management
    // ============================================================================

    /// Create a new order.
    ///
    /// # Arguments
    ///
    /// * `symbol` - Trading pair symbol.
    /// * `order_type` - Order type (Market, Limit).
    /// * `side` - Order side (Buy or Sell).
    /// * `amount` - Order quantity.
    /// * `price` - Optional price (required for limit orders).
    ///
    /// # Returns
    ///
    /// Returns the created [`Order`] structure with order details.
    pub async fn create_order(
        &self,
        symbol: &str,
        order_type: OrderType,
        side: OrderSide,
        amount: f64,
        price: Option<f64>,
    ) -> Result<Order> {
        let market = self.base().market(symbol).await?;

        let path = self.build_api_path("/trade/order");

        // Build order body
        let mut map = serde_json::Map::new();
        map.insert(
            "instId".to_string(),
            serde_json::Value::String(market.id.clone()),
        );
        map.insert(
            "tdMode".to_string(),
            serde_json::Value::String(self.options().account_mode.clone()),
        );
        map.insert(
            "side".to_string(),
            serde_json::Value::String(match side {
                OrderSide::Buy => "buy".to_string(),
                OrderSide::Sell => "sell".to_string(),
            }),
        );
        map.insert(
            "ordType".to_string(),
            serde_json::Value::String(match order_type {
                OrderType::Market => "market".to_string(),
                OrderType::Limit => "limit".to_string(),
                OrderType::LimitMaker => "post_only".to_string(),
                _ => "limit".to_string(),
            }),
        );
        map.insert(
            "sz".to_string(),
            serde_json::Value::String(amount.to_string()),
        );

        // Add price for limit orders
        if let Some(p) = price {
            if order_type == OrderType::Limit || order_type == OrderType::LimitMaker {
                map.insert("px".to_string(), serde_json::Value::String(p.to_string()));
            }
        }
        let body = serde_json::Value::Object(map);

        let response = self
            .private_request("POST", &path, None, Some(&body))
            .await?;

        let data = response
            .get("data")
            .ok_or_else(|| Error::from(ParseError::missing_field("data")))?;

        // OKX returns array with single order
        let orders = data.as_array().ok_or_else(|| {
            Error::from(ParseError::invalid_format(
                "data",
                "Expected array of orders",
            ))
        })?;

        if orders.is_empty() {
            return Err(Error::exchange("-1", "No order data returned"));
        }

        parser::parse_order(&orders[0], Some(&market))
    }

    /// Cancel an existing order.
    ///
    /// # Arguments
    ///
    /// * `id` - Order ID to cancel.
    /// * `symbol` - Trading pair symbol.
    ///
    /// # Returns
    ///
    /// Returns the canceled [`Order`] structure.
    pub async fn cancel_order(&self, id: &str, symbol: &str) -> Result<Order> {
        let market = self.base().market(symbol).await?;

        let path = self.build_api_path("/trade/cancel-order");

        let mut map = serde_json::Map::new();
        map.insert(
            "instId".to_string(),
            serde_json::Value::String(market.id.clone()),
        );
        map.insert(
            "ordId".to_string(),
            serde_json::Value::String(id.to_string()),
        );
        let body = serde_json::Value::Object(map);

        let response = self
            .private_request("POST", &path, None, Some(&body))
            .await?;

        let data = response
            .get("data")
            .ok_or_else(|| Error::from(ParseError::missing_field("data")))?;

        // OKX returns array with single order
        let orders = data.as_array().ok_or_else(|| {
            Error::from(ParseError::invalid_format(
                "data",
                "Expected array of orders",
            ))
        })?;

        if orders.is_empty() {
            return Err(Error::exchange("-1", "No order data returned"));
        }

        parser::parse_order(&orders[0], Some(&market))
    }

    /// Fetch a single order by ID.
    ///
    /// # Arguments
    ///
    /// * `id` - Order ID to fetch.
    /// * `symbol` - Trading pair symbol.
    ///
    /// # Returns
    ///
    /// Returns the [`Order`] structure with current status.
    pub async fn fetch_order(&self, id: &str, symbol: &str) -> Result<Order> {
        let market = self.base().market(symbol).await?;

        let path = self.build_api_path("/trade/order");
        let mut params = HashMap::new();
        params.insert("instId".to_string(), market.id.clone());
        params.insert("ordId".to_string(), id.to_string());

        let response = self
            .private_request("GET", &path, Some(&params), None)
            .await?;

        let data = response
            .get("data")
            .ok_or_else(|| Error::from(ParseError::missing_field("data")))?;

        // OKX returns array with single order
        let orders = data.as_array().ok_or_else(|| {
            Error::from(ParseError::invalid_format(
                "data",
                "Expected array of orders",
            ))
        })?;

        if orders.is_empty() {
            return Err(Error::exchange("51400", "Order not found"));
        }

        parser::parse_order(&orders[0], Some(&market))
    }

    /// Fetch open orders.
    ///
    /// # Arguments
    ///
    /// * `symbol` - Optional trading pair symbol. If None, fetches all open orders.
    /// * `since` - Optional start timestamp in milliseconds.
    /// * `limit` - Optional limit on number of orders (maximum: 100).
    ///
    /// # Returns
    ///
    /// Returns a vector of open [`Order`] structures.
    pub async fn fetch_open_orders(
        &self,
        symbol: Option<&str>,
        since: Option<i64>,
        limit: Option<u32>,
    ) -> Result<Vec<Order>> {
        let path = self.build_api_path("/trade/orders-pending");
        let mut params = HashMap::new();
        params.insert("instType".to_string(), self.get_inst_type().to_string());

        let market = if let Some(sym) = symbol {
            let m = self.base().market(sym).await?;
            params.insert("instId".to_string(), m.id.clone());
            Some(m)
        } else {
            None
        };

        // OKX maximum limit is 100
        let actual_limit = limit.map(|l| l.min(100)).unwrap_or(100);
        params.insert("limit".to_string(), actual_limit.to_string());

        if let Some(start_time) = since {
            params.insert("begin".to_string(), start_time.to_string());
        }

        let response = self
            .private_request("GET", &path, Some(&params), None)
            .await?;

        let data = response
            .get("data")
            .ok_or_else(|| Error::from(ParseError::missing_field("data")))?;

        let orders_array = data.as_array().ok_or_else(|| {
            Error::from(ParseError::invalid_format(
                "data",
                "Expected array of orders",
            ))
        })?;

        let mut orders = Vec::new();
        for order_data in orders_array {
            match parser::parse_order(order_data, market.as_ref()) {
                Ok(order) => orders.push(order),
                Err(e) => {
                    warn!(error = %e, "Failed to parse open order");
                }
            }
        }

        Ok(orders)
    }

    /// Fetch closed orders.
    ///
    /// # Arguments
    ///
    /// * `symbol` - Optional trading pair symbol. If None, fetches all closed orders.
    /// * `since` - Optional start timestamp in milliseconds.
    /// * `limit` - Optional limit on number of orders (maximum: 100).
    ///
    /// # Returns
    ///
    /// Returns a vector of closed [`Order`] structures.
    pub async fn fetch_closed_orders(
        &self,
        symbol: Option<&str>,
        since: Option<i64>,
        limit: Option<u32>,
    ) -> Result<Vec<Order>> {
        let path = self.build_api_path("/trade/orders-history");
        let mut params = HashMap::new();
        params.insert("instType".to_string(), self.get_inst_type().to_string());

        let market = if let Some(sym) = symbol {
            let m = self.base().market(sym).await?;
            params.insert("instId".to_string(), m.id.clone());
            Some(m)
        } else {
            None
        };

        // OKX maximum limit is 100
        let actual_limit = limit.map(|l| l.min(100)).unwrap_or(100);
        params.insert("limit".to_string(), actual_limit.to_string());

        if let Some(start_time) = since {
            params.insert("begin".to_string(), start_time.to_string());
        }

        let response = self
            .private_request("GET", &path, Some(&params), None)
            .await?;

        let data = response
            .get("data")
            .ok_or_else(|| Error::from(ParseError::missing_field("data")))?;

        let orders_array = data.as_array().ok_or_else(|| {
            Error::from(ParseError::invalid_format(
                "data",
                "Expected array of orders",
            ))
        })?;

        let mut orders = Vec::new();
        for order_data in orders_array {
            match parser::parse_order(order_data, market.as_ref()) {
                Ok(order) => orders.push(order),
                Err(e) => {
                    warn!(error = %e, "Failed to parse closed order");
                }
            }
        }

        Ok(orders)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_api_path() {
        let okx = Okx::builder().build().unwrap();
        let path = okx.build_api_path("/public/instruments");
        assert_eq!(path, "/api/v5/public/instruments");
    }

    #[test]
    fn test_get_inst_type_spot() {
        let okx = Okx::builder().build().unwrap();
        let inst_type = okx.get_inst_type();
        assert_eq!(inst_type, "SPOT");
    }

    #[test]
    fn test_get_inst_type_margin() {
        use ccxt_core::types::default_type::DefaultType;
        let okx = Okx::builder()
            .default_type(DefaultType::Margin)
            .build()
            .unwrap();
        let inst_type = okx.get_inst_type();
        assert_eq!(inst_type, "MARGIN");
    }

    #[test]
    fn test_get_inst_type_swap() {
        use ccxt_core::types::default_type::DefaultType;
        let okx = Okx::builder()
            .default_type(DefaultType::Swap)
            .build()
            .unwrap();
        let inst_type = okx.get_inst_type();
        assert_eq!(inst_type, "SWAP");
    }

    #[test]
    fn test_get_inst_type_futures() {
        use ccxt_core::types::default_type::DefaultType;
        let okx = Okx::builder()
            .default_type(DefaultType::Futures)
            .build()
            .unwrap();
        let inst_type = okx.get_inst_type();
        assert_eq!(inst_type, "FUTURES");
    }

    #[test]
    fn test_get_inst_type_option() {
        use ccxt_core::types::default_type::DefaultType;
        let okx = Okx::builder()
            .default_type(DefaultType::Option)
            .build()
            .unwrap();
        let inst_type = okx.get_inst_type();
        assert_eq!(inst_type, "OPTION");
    }

    #[test]
    fn test_get_timestamp() {
        let okx = Okx::builder().build().unwrap();
        let ts = okx.get_timestamp();

        // Should be in ISO 8601 format
        assert!(ts.contains("T"));
        assert!(ts.contains("Z"));
        assert!(ts.len() > 20);
    }
}
