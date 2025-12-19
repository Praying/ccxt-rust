//! Bitget REST API implementation.
//!
//! Implements all REST API endpoint operations for the Bitget exchange.

use super::{Bitget, BitgetAuth, error, parser};
use ccxt_core::{
    Error, ParseError, Result,
    types::{Balance, Market, OHLCV, Order, OrderBook, OrderSide, OrderType, Ticker, Trade},
};
use reqwest::header::{HeaderMap, HeaderValue};
use serde_json::Value;
use std::collections::HashMap;
use tracing::{debug, info, warn};

impl Bitget {
    // ============================================================================
    // Helper Methods
    // ============================================================================

    /// Get the current timestamp in milliseconds.
    fn get_timestamp(&self) -> String {
        chrono::Utc::now().timestamp_millis().to_string()
    }

    /// Get the authentication instance if credentials are configured.
    fn get_auth(&self) -> Result<BitgetAuth> {
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

        Ok(BitgetAuth::new(
            api_key.clone(),
            secret.clone(),
            passphrase.clone(),
        ))
    }

    /// Check that required credentials are configured.
    pub fn check_required_credentials(&self) -> Result<()> {
        self.base().check_required_credentials()?;
        if self.base().config.password.is_none() {
            return Err(Error::authentication("Passphrase is required for Bitget"));
        }
        Ok(())
    }

    /// Build the API path with product type prefix.
    ///
    /// Uses the effective product type derived from `default_type` and `default_sub_type`
    /// to determine the correct API endpoint:
    /// - "spot" -> /api/v2/spot
    /// - "umcbl" (USDT-M) -> /api/v2/mix
    /// - "dmcbl" (Coin-M) -> /api/v2/mix
    fn build_api_path(&self, endpoint: &str) -> String {
        let product_type = self.options().effective_product_type();
        match product_type {
            "spot" => format!("/api/v2/spot{}", endpoint),
            "umcbl" | "usdt-futures" => format!("/api/v2/mix{}", endpoint),
            "dmcbl" | "coin-futures" => format!("/api/v2/mix{}", endpoint),
            _ => format!("/api/v2/spot{}", endpoint),
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

        debug!("Bitget public request: {} {}", method, url);

        let response = match method.to_uppercase().as_str() {
            "GET" => self.base().http_client.get(&url, None).await?,
            "POST" => self.base().http_client.post(&url, None, None).await?,
            _ => {
                return Err(Error::invalid_request(format!(
                    "Unsupported HTTP method: {}",
                    method
                )));
            }
        };

        // Check for Bitget error response
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

        // Sign the request
        let sign_path = format!("{}{}", path, query_string);
        let signature = auth.sign(&timestamp, method, &sign_path, &body_string);

        // Build headers
        let mut headers = HeaderMap::new();
        auth.add_auth_headers(&mut headers, &timestamp, &signature);
        headers.insert("Content-Type", HeaderValue::from_static("application/json"));

        let url = format!("{}{}{}", urls.rest, path, query_string);
        debug!("Bitget private request: {} {}", method, url);

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

        // Check for Bitget error response
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
    /// # use ccxt_exchanges::bitget::Bitget;
    /// # async fn example() -> ccxt_core::Result<()> {
    /// let bitget = Bitget::builder().build()?;
    /// let markets = bitget.fetch_markets().await?;
    /// println!("Found {} markets", markets.len());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn fetch_markets(&self) -> Result<Vec<Market>> {
        let path = self.build_api_path("/public/symbols");
        let response = self.public_request("GET", &path, None).await?;

        let data = response
            .get("data")
            .ok_or_else(|| Error::from(ParseError::missing_field("data")))?;

        let symbols = data.as_array().ok_or_else(|| {
            Error::from(ParseError::invalid_format(
                "data",
                "Expected array of symbols",
            ))
        })?;

        let mut markets = Vec::new();
        for symbol in symbols {
            match parser::parse_market(symbol) {
                Ok(market) => markets.push(market),
                Err(e) => {
                    warn!(error = %e, "Failed to parse market");
                }
            }
        }

        // Cache the markets and preserve ownership for the caller
        let markets = self.base().set_markets(markets, None).await?;

        info!("Loaded {} markets for Bitget", markets.len());
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
    ///
    /// # Errors
    ///
    /// Returns an error if the API request fails or response parsing fails.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use ccxt_exchanges::bitget::Bitget;
    /// # async fn example() -> ccxt_core::Result<()> {
    /// let bitget = Bitget::builder().build()?;
    ///
    /// // Load markets for the first time
    /// let markets = bitget.load_markets(false).await?;
    /// println!("Loaded {} markets", markets.len());
    ///
    /// // Subsequent calls use cache (no API request)
    /// let markets = bitget.load_markets(false).await?;
    ///
    /// // Force reload
    /// let markets = bitget.load_markets(true).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn load_markets(&self, reload: bool) -> Result<HashMap<String, Market>> {
        // Acquire the loading lock to serialize concurrent load_markets calls
        // This prevents multiple tasks from making duplicate API calls
        let _loading_guard = self.base().market_loading_lock.lock().await;

        // Check cache status while holding the lock
        {
            let cache = self.base().market_cache.read().await;
            if cache.loaded && !reload {
                debug!(
                    "Returning cached markets for Bitget ({} markets)",
                    cache.markets.len()
                );
                return Ok(cache.markets.clone());
            }
        }

        info!("Loading markets for Bitget (reload: {})", reload);
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
    ///
    /// # Errors
    ///
    /// Returns an error if the market is not found or the API request fails.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use ccxt_exchanges::bitget::Bitget;
    /// # async fn example() -> ccxt_core::Result<()> {
    /// let bitget = Bitget::builder().build()?;
    /// bitget.load_markets(false).await?;
    /// let ticker = bitget.fetch_ticker("BTC/USDT").await?;
    /// println!("BTC/USDT last price: {:?}", ticker.last);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn fetch_ticker(&self, symbol: &str) -> Result<Ticker> {
        let market = self.base().market(symbol).await?;

        let path = self.build_api_path("/market/tickers");
        let mut params = HashMap::new();
        params.insert("symbol".to_string(), market.id.clone());

        let response = self.public_request("GET", &path, Some(&params)).await?;

        let data = response
            .get("data")
            .ok_or_else(|| Error::from(ParseError::missing_field("data")))?;

        // Bitget returns an array even for single ticker
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
    ///
    /// # Errors
    ///
    /// Returns an error if markets are not loaded or the API request fails.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use ccxt_exchanges::bitget::Bitget;
    /// # async fn example() -> ccxt_core::Result<()> {
    /// let bitget = Bitget::builder().build()?;
    /// bitget.load_markets(false).await?;
    ///
    /// // Fetch all tickers
    /// let all_tickers = bitget.fetch_tickers(None).await?;
    ///
    /// // Fetch specific tickers
    /// let tickers = bitget.fetch_tickers(Some(vec!["BTC/USDT".to_string(), "ETH/USDT".to_string()])).await?;
    /// # Ok(())
    /// # }
    /// ```
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
        let response = self.public_request("GET", &path, None).await?;

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
            if let Some(bitget_symbol) = ticker_data["symbol"].as_str() {
                let cache = self.base().market_cache.read().await;
                if let Some(market) = cache.markets_by_id.get(bitget_symbol) {
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
                                symbol = %bitget_symbol,
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
    /// * `limit` - Optional depth limit (valid values: 1, 5, 15, 50, 100; default: 100).
    ///
    /// # Returns
    ///
    /// Returns [`OrderBook`] data containing bids and asks.
    ///
    /// # Errors
    ///
    /// Returns an error if the market is not found or the API request fails.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use ccxt_exchanges::bitget::Bitget;
    /// # async fn example() -> ccxt_core::Result<()> {
    /// let bitget = Bitget::builder().build()?;
    /// bitget.load_markets(false).await?;
    /// let orderbook = bitget.fetch_order_book("BTC/USDT", Some(50)).await?;
    /// println!("Best bid: {:?}", orderbook.bids.first());
    /// println!("Best ask: {:?}", orderbook.asks.first());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn fetch_order_book(&self, symbol: &str, limit: Option<u32>) -> Result<OrderBook> {
        let market = self.base().market(symbol).await?;

        let path = self.build_api_path("/market/orderbook");
        let mut params = HashMap::new();
        params.insert("symbol".to_string(), market.id.clone());

        // Bitget valid limits: 1, 5, 15, 50, 100
        // Cap to maximum allowed value
        let actual_limit = limit.map(|l| l.min(100)).unwrap_or(100);
        params.insert("limit".to_string(), actual_limit.to_string());

        let response = self.public_request("GET", &path, Some(&params)).await?;

        let data = response
            .get("data")
            .ok_or_else(|| Error::from(ParseError::missing_field("data")))?;

        parser::parse_orderbook(data, market.symbol.clone())
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
    ///
    /// # Errors
    ///
    /// Returns an error if the market is not found or the API request fails.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use ccxt_exchanges::bitget::Bitget;
    /// # async fn example() -> ccxt_core::Result<()> {
    /// let bitget = Bitget::builder().build()?;
    /// bitget.load_markets(false).await?;
    /// let trades = bitget.fetch_trades("BTC/USDT", Some(100)).await?;
    /// for trade in trades.iter().take(5) {
    ///     println!("Trade: {:?} @ {:?}", trade.amount, trade.price);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn fetch_trades(&self, symbol: &str, limit: Option<u32>) -> Result<Vec<Trade>> {
        let market = self.base().market(symbol).await?;

        let path = self.build_api_path("/market/fills");
        let mut params = HashMap::new();
        params.insert("symbol".to_string(), market.id.clone());

        // Bitget maximum limit is 500
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
    /// * `limit` - Optional limit on number of candles (maximum: 1000).
    ///
    /// # Returns
    ///
    /// Returns a vector of [`OHLCV`] structures.
    ///
    /// # Errors
    ///
    /// Returns an error if the market is not found or the API request fails.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use ccxt_exchanges::bitget::Bitget;
    /// # async fn example() -> ccxt_core::Result<()> {
    /// let bitget = Bitget::builder().build()?;
    /// bitget.load_markets(false).await?;
    /// let ohlcv = bitget.fetch_ohlcv("BTC/USDT", "1h", None, Some(100)).await?;
    /// for candle in ohlcv.iter().take(5) {
    ///     println!("Open: {}, Close: {}", candle.open, candle.close);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn fetch_ohlcv(
        &self,
        symbol: &str,
        timeframe: &str,
        since: Option<i64>,
        limit: Option<u32>,
    ) -> Result<Vec<OHLCV>> {
        let market = self.base().market(symbol).await?;

        // Convert timeframe to Bitget format
        let timeframes = self.timeframes();
        let bitget_timeframe = timeframes.get(timeframe).ok_or_else(|| {
            Error::invalid_request(format!("Unsupported timeframe: {}", timeframe))
        })?;

        let path = self.build_api_path("/market/candles");
        let mut params = HashMap::new();
        params.insert("symbol".to_string(), market.id.clone());
        params.insert("granularity".to_string(), bitget_timeframe.clone());

        // Bitget maximum limit is 1000
        let actual_limit = limit.map(|l| l.min(1000)).unwrap_or(100);
        params.insert("limit".to_string(), actual_limit.to_string());

        if let Some(start_time) = since {
            params.insert("startTime".to_string(), start_time.to_string());
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
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use ccxt_exchanges::bitget::Bitget;
    /// # async fn example() -> ccxt_core::Result<()> {
    /// let bitget = Bitget::builder()
    ///     .api_key("your-api-key")
    ///     .secret("your-secret")
    ///     .passphrase("your-passphrase")
    ///     .build()?;
    /// let balance = bitget.fetch_balance().await?;
    /// if let Some(btc) = balance.get("BTC") {
    ///     println!("BTC balance: free={}, used={}, total={}", btc.free, btc.used, btc.total);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn fetch_balance(&self) -> Result<Balance> {
        let path = self.build_api_path("/account/assets");
        let response = self.private_request("GET", &path, None, None).await?;

        let data = response
            .get("data")
            .ok_or_else(|| Error::from(ParseError::missing_field("data")))?;

        parser::parse_balance(data)
    }

    /// Fetch user's trade history.
    ///
    /// # Arguments
    ///
    /// * `symbol` - Trading pair symbol.
    /// * `since` - Optional start timestamp in milliseconds.
    /// * `limit` - Optional limit on number of trades (maximum: 500).
    ///
    /// # Returns
    ///
    /// Returns a vector of [`Trade`] structures representing user's trade history.
    ///
    /// # Errors
    ///
    /// Returns an error if authentication fails or the API request fails.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use ccxt_exchanges::bitget::Bitget;
    /// # async fn example() -> ccxt_core::Result<()> {
    /// let bitget = Bitget::builder()
    ///     .api_key("your-api-key")
    ///     .secret("your-secret")
    ///     .passphrase("your-passphrase")
    ///     .build()?;
    /// bitget.load_markets(false).await?;
    /// let my_trades = bitget.fetch_my_trades("BTC/USDT", None, Some(50)).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn fetch_my_trades(
        &self,
        symbol: &str,
        since: Option<i64>,
        limit: Option<u32>,
    ) -> Result<Vec<Trade>> {
        let market = self.base().market(symbol).await?;

        let path = self.build_api_path("/trade/fills");
        let mut params = HashMap::new();
        params.insert("symbol".to_string(), market.id.clone());

        // Bitget maximum limit is 500
        let actual_limit = limit.map(|l| l.min(500)).unwrap_or(100);
        params.insert("limit".to_string(), actual_limit.to_string());

        if let Some(start_time) = since {
            params.insert("startTime".to_string(), start_time.to_string());
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
    ///
    /// # Errors
    ///
    /// Returns an error if authentication fails, market is not found, or the API request fails.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use ccxt_exchanges::bitget::Bitget;
    /// # use ccxt_core::types::{OrderType, OrderSide};
    /// # async fn example() -> ccxt_core::Result<()> {
    /// let bitget = Bitget::builder()
    ///     .api_key("your-api-key")
    ///     .secret("your-secret")
    ///     .passphrase("your-passphrase")
    ///     .build()?;
    /// bitget.load_markets(false).await?;
    ///
    /// // Create a limit buy order
    /// let order = bitget.create_order(
    ///     "BTC/USDT",
    ///     OrderType::Limit,
    ///     OrderSide::Buy,
    ///     0.001,
    ///     Some(50000.0),
    /// ).await?;
    /// println!("Order created: {}", order.id);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn create_order(
        &self,
        symbol: &str,
        order_type: OrderType,
        side: OrderSide,
        amount: f64,
        price: Option<f64>,
    ) -> Result<Order> {
        let market = self.base().market(symbol).await?;

        let path = self.build_api_path("/trade/place-order");

        // Build order body
        let mut map = serde_json::Map::new();
        map.insert(
            "symbol".to_string(),
            serde_json::Value::String(market.id.clone()),
        );
        map.insert(
            "side".to_string(),
            serde_json::Value::String(match side {
                OrderSide::Buy => "buy".to_string(),
                OrderSide::Sell => "sell".to_string(),
            }),
        );
        map.insert(
            "orderType".to_string(),
            serde_json::Value::String(match order_type {
                OrderType::Market => "market".to_string(),
                OrderType::Limit => "limit".to_string(),
                OrderType::LimitMaker => "limit_maker".to_string(),
                _ => "limit".to_string(),
            }),
        );
        map.insert(
            "size".to_string(),
            serde_json::Value::String(amount.to_string()),
        );
        map.insert(
            "force".to_string(),
            serde_json::Value::String("gtc".to_string()),
        );

        // Add price for limit orders
        if let Some(p) = price {
            if order_type == OrderType::Limit || order_type == OrderType::LimitMaker {
                map.insert(
                    "price".to_string(),
                    serde_json::Value::String(p.to_string()),
                );
            }
        }
        let body = serde_json::Value::Object(map);

        let response = self
            .private_request("POST", &path, None, Some(&body))
            .await?;

        let data = response
            .get("data")
            .ok_or_else(|| Error::from(ParseError::missing_field("data")))?;

        parser::parse_order(data, Some(&market))
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
    ///
    /// # Errors
    ///
    /// Returns an error if authentication fails or the API request fails.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use ccxt_exchanges::bitget::Bitget;
    /// # async fn example() -> ccxt_core::Result<()> {
    /// let bitget = Bitget::builder()
    ///     .api_key("your-api-key")
    ///     .secret("your-secret")
    ///     .passphrase("your-passphrase")
    ///     .build()?;
    /// bitget.load_markets(false).await?;
    /// let order = bitget.cancel_order("123456789", "BTC/USDT").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn cancel_order(&self, id: &str, symbol: &str) -> Result<Order> {
        let market = self.base().market(symbol).await?;

        let path = self.build_api_path("/trade/cancel-order");

        let mut map = serde_json::Map::new();
        map.insert(
            "symbol".to_string(),
            serde_json::Value::String(market.id.clone()),
        );
        map.insert(
            "orderId".to_string(),
            serde_json::Value::String(id.to_string()),
        );
        let body = serde_json::Value::Object(map);

        let response = self
            .private_request("POST", &path, None, Some(&body))
            .await?;

        let data = response
            .get("data")
            .ok_or_else(|| Error::from(ParseError::missing_field("data")))?;

        parser::parse_order(data, Some(&market))
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
    ///
    /// # Errors
    ///
    /// Returns an error if authentication fails or the API request fails.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use ccxt_exchanges::bitget::Bitget;
    /// # async fn example() -> ccxt_core::Result<()> {
    /// let bitget = Bitget::builder()
    ///     .api_key("your-api-key")
    ///     .secret("your-secret")
    ///     .passphrase("your-passphrase")
    ///     .build()?;
    /// bitget.load_markets(false).await?;
    /// let order = bitget.fetch_order("123456789", "BTC/USDT").await?;
    /// println!("Order status: {:?}", order.status);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn fetch_order(&self, id: &str, symbol: &str) -> Result<Order> {
        let market = self.base().market(symbol).await?;

        let path = self.build_api_path("/trade/orderInfo");
        let mut params = HashMap::new();
        params.insert("symbol".to_string(), market.id.clone());
        params.insert("orderId".to_string(), id.to_string());

        let response = self
            .private_request("GET", &path, Some(&params), None)
            .await?;

        let data = response
            .get("data")
            .ok_or_else(|| Error::from(ParseError::missing_field("data")))?;

        // Bitget may return an array with single order
        let order_data = if data.is_array() {
            data.as_array()
                .and_then(|arr| arr.first())
                .ok_or_else(|| Error::exchange("40007", "Order not found"))?
        } else {
            data
        };

        parser::parse_order(order_data, Some(&market))
    }

    /// Fetch open orders.
    ///
    /// # Arguments
    ///
    /// * `symbol` - Optional trading pair symbol. If None, fetches all open orders.
    /// * `since` - Optional start timestamp in milliseconds.
    /// * `limit` - Optional limit on number of orders (maximum: 500).
    ///
    /// # Returns
    ///
    /// Returns a vector of open [`Order`] structures.
    ///
    /// # Errors
    ///
    /// Returns an error if authentication fails or the API request fails.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use ccxt_exchanges::bitget::Bitget;
    /// # async fn example() -> ccxt_core::Result<()> {
    /// let bitget = Bitget::builder()
    ///     .api_key("your-api-key")
    ///     .secret("your-secret")
    ///     .passphrase("your-passphrase")
    ///     .build()?;
    /// bitget.load_markets(false).await?;
    ///
    /// // Fetch all open orders
    /// let all_open = bitget.fetch_open_orders(None, None, None).await?;
    ///
    /// // Fetch open orders for specific symbol
    /// let btc_open = bitget.fetch_open_orders(Some("BTC/USDT"), None, Some(50)).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn fetch_open_orders(
        &self,
        symbol: Option<&str>,
        since: Option<i64>,
        limit: Option<u32>,
    ) -> Result<Vec<Order>> {
        let path = self.build_api_path("/trade/unfilled-orders");
        let mut params = HashMap::new();

        let market = if let Some(sym) = symbol {
            let m = self.base().market(sym).await?;
            params.insert("symbol".to_string(), m.id.clone());
            Some(m)
        } else {
            None
        };

        // Bitget maximum limit is 500
        let actual_limit = limit.map(|l| l.min(500)).unwrap_or(100);
        params.insert("limit".to_string(), actual_limit.to_string());

        if let Some(start_time) = since {
            params.insert("startTime".to_string(), start_time.to_string());
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
    /// * `limit` - Optional limit on number of orders (maximum: 500).
    ///
    /// # Returns
    ///
    /// Returns a vector of closed [`Order`] structures.
    ///
    /// # Errors
    ///
    /// Returns an error if authentication fails or the API request fails.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use ccxt_exchanges::bitget::Bitget;
    /// # async fn example() -> ccxt_core::Result<()> {
    /// let bitget = Bitget::builder()
    ///     .api_key("your-api-key")
    ///     .secret("your-secret")
    ///     .passphrase("your-passphrase")
    ///     .build()?;
    /// bitget.load_markets(false).await?;
    ///
    /// // Fetch closed orders for specific symbol
    /// let closed = bitget.fetch_closed_orders(Some("BTC/USDT"), None, Some(50)).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn fetch_closed_orders(
        &self,
        symbol: Option<&str>,
        since: Option<i64>,
        limit: Option<u32>,
    ) -> Result<Vec<Order>> {
        let path = self.build_api_path("/trade/history-orders");
        let mut params = HashMap::new();

        let market = if let Some(sym) = symbol {
            let m = self.base().market(sym).await?;
            params.insert("symbol".to_string(), m.id.clone());
            Some(m)
        } else {
            None
        };

        // Bitget maximum limit is 500
        let actual_limit = limit.map(|l| l.min(500)).unwrap_or(100);
        params.insert("limit".to_string(), actual_limit.to_string());

        if let Some(start_time) = since {
            params.insert("startTime".to_string(), start_time.to_string());
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
    use ccxt_core::types::default_type::{DefaultSubType, DefaultType};

    #[test]
    fn test_build_api_path_spot() {
        let bitget = Bitget::builder().build().unwrap();
        let path = bitget.build_api_path("/public/symbols");
        assert_eq!(path, "/api/v2/spot/public/symbols");
    }

    #[test]
    fn test_build_api_path_futures_legacy() {
        // Legacy test using product_type directly
        // Note: product_type is kept for backward compatibility but
        // effective_product_type() now derives from default_type/default_sub_type
        // This test verifies that using default_type achieves the same result
        let bitget = Bitget::builder()
            .default_type(DefaultType::Swap)
            .default_sub_type(DefaultSubType::Linear)
            .build()
            .unwrap();
        let path = bitget.build_api_path("/public/symbols");
        assert_eq!(path, "/api/v2/mix/public/symbols");
    }

    #[test]
    fn test_build_api_path_with_default_type_spot() {
        let bitget = Bitget::builder()
            .default_type(DefaultType::Spot)
            .build()
            .unwrap();
        let path = bitget.build_api_path("/public/symbols");
        assert_eq!(path, "/api/v2/spot/public/symbols");
    }

    #[test]
    fn test_build_api_path_with_default_type_swap_linear() {
        let bitget = Bitget::builder()
            .default_type(DefaultType::Swap)
            .default_sub_type(DefaultSubType::Linear)
            .build()
            .unwrap();
        let path = bitget.build_api_path("/public/symbols");
        assert_eq!(path, "/api/v2/mix/public/symbols");
    }

    #[test]
    fn test_build_api_path_with_default_type_swap_inverse() {
        let bitget = Bitget::builder()
            .default_type(DefaultType::Swap)
            .default_sub_type(DefaultSubType::Inverse)
            .build()
            .unwrap();
        let path = bitget.build_api_path("/public/symbols");
        assert_eq!(path, "/api/v2/mix/public/symbols");
    }

    #[test]
    fn test_build_api_path_with_default_type_futures() {
        let bitget = Bitget::builder()
            .default_type(DefaultType::Futures)
            .build()
            .unwrap();
        let path = bitget.build_api_path("/public/symbols");
        // Futures defaults to Linear (umcbl) which uses mix API
        assert_eq!(path, "/api/v2/mix/public/symbols");
    }

    #[test]
    fn test_build_api_path_with_default_type_margin() {
        let bitget = Bitget::builder()
            .default_type(DefaultType::Margin)
            .build()
            .unwrap();
        let path = bitget.build_api_path("/public/symbols");
        // Margin uses spot API
        assert_eq!(path, "/api/v2/spot/public/symbols");
    }

    #[test]
    fn test_get_timestamp() {
        let bitget = Bitget::builder().build().unwrap();
        let ts = bitget.get_timestamp();

        // Should be a valid timestamp string
        let parsed: i64 = ts.parse().unwrap();
        assert!(parsed > 0);

        // Should be close to current time (within 1 second)
        let now = chrono::Utc::now().timestamp_millis();
        assert!((now - parsed).abs() < 1000);
    }
}
