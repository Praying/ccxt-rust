//! Bybit REST API implementation.
//!
//! Implements all REST API endpoint operations for the Bybit exchange.

use super::{Bybit, BybitAuth, error, parser};
use ccxt_core::{
    Error, ParseError, Result,
    types::{
        Amount, Balance, Market, OHLCV, Order, OrderBook, OrderSide, OrderType, Price, Ticker,
        Trade,
    },
};
use reqwest::header::{HeaderMap, HeaderValue};
use serde_json::Value;
use std::{collections::HashMap, sync::Arc};
use tracing::{debug, info, warn};

impl Bybit {
    // ============================================================================
    // Helper Methods
    // ============================================================================

    /// Get the current timestamp in milliseconds.
    ///
    /// # Deprecated
    ///
    /// This method is deprecated. Use [`signed_request()`](Self::signed_request) instead.
    /// The `signed_request()` builder handles timestamp generation internally.
    #[deprecated(
        since = "0.1.0",
        note = "Use `signed_request()` builder instead which handles timestamps internally"
    )]
    #[allow(dead_code)]
    fn get_timestamp(&self) -> String {
        chrono::Utc::now().timestamp_millis().to_string()
    }

    /// Get the authentication instance if credentials are configured.
    pub fn get_auth(&self) -> Result<BybitAuth> {
        let config = &self.base().config;

        let api_key = config
            .api_key
            .as_ref()
            .ok_or_else(|| Error::authentication("API key is required"))?;
        let secret = config
            .secret
            .as_ref()
            .ok_or_else(|| Error::authentication("API secret is required"))?;

        Ok(BybitAuth::new(api_key.clone(), secret.clone()))
    }

    /// Check that required credentials are configured.
    pub fn check_required_credentials(&self) -> Result<()> {
        self.base().check_required_credentials()
    }

    /// Build the API path for Bybit V5 API.
    fn build_api_path(&self, endpoint: &str) -> String {
        format!("/v5{}", endpoint)
    }

    /// Get the category for API requests based on account type.
    fn get_category(&self) -> &str {
        match self.options().account_type.as_str() {
            "SPOT" => "spot",
            "CONTRACT" | "LINEAR" => "linear",
            "INVERSE" => "inverse",
            "OPTION" => "option",
            _ => "spot",
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

        debug!("Bybit public request: {} {}", method, url);

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

        // Check for Bybit error response
        if error::is_error_response(&response) {
            return Err(error::parse_error(&response));
        }

        Ok(response)
    }

    /// Make a private API request (authentication required).
    ///
    /// # Deprecated
    ///
    /// This method is deprecated. Use [`signed_request()`](Self::signed_request) instead.
    /// The `signed_request()` builder provides a cleaner, more maintainable API for
    /// constructing authenticated requests.
    #[deprecated(
        since = "0.1.0",
        note = "Use `signed_request()` builder instead for cleaner, more maintainable code"
    )]
    #[allow(dead_code)]
    #[allow(deprecated)]
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
        let recv_window = self.options().recv_window;

        // Build query string for GET requests
        let query_string = if let Some(p) = params {
            if !p.is_empty() {
                let query: Vec<String> = p
                    .iter()
                    .map(|(k, v)| format!("{}={}", k, urlencoding::encode(v)))
                    .collect();
                query.join("&")
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

        // Sign the request - Bybit uses query string for GET, body for POST
        let sign_params = if method.to_uppercase() == "GET" {
            &query_string
        } else {
            &body_string
        };
        let signature = auth.sign(&timestamp, recv_window, sign_params);

        // Build headers
        let mut headers = HeaderMap::new();
        auth.add_auth_headers(&mut headers, &timestamp, &signature, recv_window);
        headers.insert("Content-Type", HeaderValue::from_static("application/json"));

        let url = if query_string.is_empty() {
            format!("{}{}", urls.rest, path)
        } else {
            format!("{}{}?{}", urls.rest, path, query_string)
        };
        debug!("Bybit private request: {} {}", method, url);

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

        // Check for Bybit error response
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
    /// # use ccxt_exchanges::bybit::Bybit;
    /// # async fn example() -> ccxt_core::Result<()> {
    /// let bybit = Bybit::builder().build()?;
    /// let markets = bybit.fetch_markets().await?;
    /// println!("Found {} markets", markets.len());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn fetch_markets(&self) -> Result<Arc<HashMap<String, Arc<Market>>>> {
        let path = self.build_api_path("/market/instruments-info");
        let mut params = HashMap::new();
        params.insert("category".to_string(), self.get_category().to_string());

        let response = self.public_request("GET", &path, Some(&params)).await?;

        let result = response
            .get("result")
            .ok_or_else(|| Error::from(ParseError::missing_field("result")))?;

        let list = result
            .get("list")
            .ok_or_else(|| Error::from(ParseError::missing_field("list")))?;

        let instruments = list.as_array().ok_or_else(|| {
            Error::from(ParseError::invalid_format(
                "list",
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

        info!("Loaded {} markets for Bybit", markets.len());
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
    pub async fn load_markets(&self, reload: bool) -> Result<Arc<HashMap<String, Arc<Market>>>> {
        // Acquire the loading lock to serialize concurrent load_markets calls
        // This prevents multiple tasks from making duplicate API calls
        let _loading_guard = self.base().market_loading_lock.lock().await;

        // Check cache status while holding the lock
        {
            let cache = self.base().market_cache.read().await;
            if cache.loaded && !reload {
                debug!(
                    "Returning cached markets for Bybit ({} markets)",
                    cache.markets.len()
                );
                return Ok(cache.markets.clone());
            }
        }

        info!("Loading markets for Bybit (reload: {})", reload);
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

        let path = self.build_api_path("/market/tickers");
        let mut params = HashMap::new();
        params.insert("category".to_string(), self.get_category().to_string());
        params.insert("symbol".to_string(), market.id.clone());

        let response = self.public_request("GET", &path, Some(&params)).await?;

        let result = response
            .get("result")
            .ok_or_else(|| Error::from(ParseError::missing_field("result")))?;

        let list = result
            .get("list")
            .ok_or_else(|| Error::from(ParseError::missing_field("list")))?;

        let tickers = list.as_array().ok_or_else(|| {
            Error::from(ParseError::invalid_format(
                "list",
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
        params.insert("category".to_string(), self.get_category().to_string());

        let response = self.public_request("GET", &path, Some(&params)).await?;

        let result = response
            .get("result")
            .ok_or_else(|| Error::from(ParseError::missing_field("result")))?;

        let list = result
            .get("list")
            .ok_or_else(|| Error::from(ParseError::missing_field("list")))?;

        let tickers_array = list.as_array().ok_or_else(|| {
            Error::from(ParseError::invalid_format(
                "list",
                "Expected array of tickers",
            ))
        })?;

        let mut tickers = Vec::new();
        for ticker_data in tickers_array {
            if let Some(symbol_id) = ticker_data["symbol"].as_str() {
                let cache = self.base().market_cache.read().await;
                if let Some(market) = cache.markets_by_id.get(symbol_id) {
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
                                symbol = %symbol_id,
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
    /// * `limit` - Optional depth limit (valid values: 1-500; default: 25).
    ///
    /// # Returns
    ///
    /// Returns [`OrderBook`] data containing bids and asks.
    pub async fn fetch_order_book(&self, symbol: &str, limit: Option<u32>) -> Result<OrderBook> {
        let market = self.base().market(symbol).await?;

        let path = self.build_api_path("/market/orderbook");
        let mut params = HashMap::new();
        params.insert("category".to_string(), self.get_category().to_string());
        params.insert("symbol".to_string(), market.id.clone());

        // Bybit valid limits: 1-500, default 25
        // Cap to maximum allowed value
        let actual_limit = limit.map(|l| l.min(500)).unwrap_or(25);
        params.insert("limit".to_string(), actual_limit.to_string());

        let response = self.public_request("GET", &path, Some(&params)).await?;

        let result = response
            .get("result")
            .ok_or_else(|| Error::from(ParseError::missing_field("result")))?;

        parser::parse_orderbook(result, market.symbol.clone())
    }

    /// Fetch recent public trades.
    ///
    /// # Arguments
    ///
    /// * `symbol` - Trading pair symbol.
    /// * `limit` - Optional limit on number of trades (maximum: 1000).
    ///
    /// # Returns
    ///
    /// Returns a vector of [`Trade`] structures, sorted by timestamp in descending order.
    pub async fn fetch_trades(&self, symbol: &str, limit: Option<u32>) -> Result<Vec<Trade>> {
        let market = self.base().market(symbol).await?;

        let path = self.build_api_path("/market/recent-trade");
        let mut params = HashMap::new();
        params.insert("category".to_string(), self.get_category().to_string());
        params.insert("symbol".to_string(), market.id.clone());

        // Bybit maximum limit is 1000
        let actual_limit = limit.map(|l| l.min(1000)).unwrap_or(60);
        params.insert("limit".to_string(), actual_limit.to_string());

        let response = self.public_request("GET", &path, Some(&params)).await?;

        let result = response
            .get("result")
            .ok_or_else(|| Error::from(ParseError::missing_field("result")))?;

        let list = result
            .get("list")
            .ok_or_else(|| Error::from(ParseError::missing_field("list")))?;

        let trades_array = list.as_array().ok_or_else(|| {
            Error::from(ParseError::invalid_format(
                "list",
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
    pub async fn fetch_ohlcv(
        &self,
        symbol: &str,
        timeframe: &str,
        since: Option<i64>,
        limit: Option<u32>,
    ) -> Result<Vec<OHLCV>> {
        let market = self.base().market(symbol).await?;

        // Convert timeframe to Bybit format
        let timeframes = self.timeframes();
        let bybit_timeframe = timeframes.get(timeframe).ok_or_else(|| {
            Error::invalid_request(format!("Unsupported timeframe: {}", timeframe))
        })?;

        let path = self.build_api_path("/market/kline");
        let mut params = HashMap::new();
        params.insert("category".to_string(), self.get_category().to_string());
        params.insert("symbol".to_string(), market.id.clone());
        params.insert("interval".to_string(), bybit_timeframe.clone());

        // Bybit maximum limit is 1000
        let actual_limit = limit.map(|l| l.min(1000)).unwrap_or(200);
        params.insert("limit".to_string(), actual_limit.to_string());

        if let Some(start_time) = since {
            params.insert("start".to_string(), start_time.to_string());
        }

        let response = self.public_request("GET", &path, Some(&params)).await?;

        let result = response
            .get("result")
            .ok_or_else(|| Error::from(ParseError::missing_field("result")))?;

        let list = result
            .get("list")
            .ok_or_else(|| Error::from(ParseError::missing_field("list")))?;

        let candles_array = list.as_array().ok_or_else(|| {
            Error::from(ParseError::invalid_format(
                "list",
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
        let path = self.build_api_path("/account/wallet-balance");

        let response = self
            .signed_request(&path)
            .param("accountType", &self.options().account_type)
            .execute()
            .await?;

        let result = response
            .get("result")
            .ok_or_else(|| Error::from(ParseError::missing_field("result")))?;

        parser::parse_balance(result)
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

        let path = self.build_api_path("/execution/list");

        // Bybit maximum limit is 100
        let actual_limit = limit.map(|l| l.min(100)).unwrap_or(50);

        let response = self
            .signed_request(&path)
            .param("category", self.get_category())
            .param("symbol", &market.id)
            .param("limit", actual_limit)
            .optional_param("startTime", since)
            .execute()
            .await?;

        let result = response
            .get("result")
            .ok_or_else(|| Error::from(ParseError::missing_field("result")))?;

        let list = result
            .get("list")
            .ok_or_else(|| Error::from(ParseError::missing_field("list")))?;

        let trades_array = list.as_array().ok_or_else(|| {
            Error::from(ParseError::invalid_format(
                "list",
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
    /// * `amount` - Order quantity as [`Amount`] type.
    /// * `price` - Optional price as [`Price`] type (required for limit orders).
    ///
    /// # Returns
    ///
    /// Returns the created [`Order`] structure with order details.
    pub async fn create_order(
        &self,
        symbol: &str,
        order_type: OrderType,
        side: OrderSide,
        amount: Amount,
        price: Option<Price>,
    ) -> Result<Order> {
        use crate::bybit::signed_request::HttpMethod;

        let market = self.base().market(symbol).await?;

        let path = self.build_api_path("/order/create");

        // Build order body
        let mut map = serde_json::Map::new();
        map.insert(
            "category".to_string(),
            serde_json::Value::String(self.get_category().to_string()),
        );
        map.insert(
            "symbol".to_string(),
            serde_json::Value::String(market.id.clone()),
        );
        map.insert(
            "side".to_string(),
            serde_json::Value::String(match side {
                OrderSide::Buy => "Buy".to_string(),
                OrderSide::Sell => "Sell".to_string(),
            }),
        );
        map.insert(
            "orderType".to_string(),
            serde_json::Value::String(match order_type {
                OrderType::Market => "Market".to_string(),
                OrderType::Limit => "Limit".to_string(),
                OrderType::LimitMaker => "Limit".to_string(),
                _ => "Limit".to_string(),
            }),
        );
        map.insert(
            "qty".to_string(),
            serde_json::Value::String(amount.to_string()),
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

        // Add time in force for limit maker orders
        if order_type == OrderType::LimitMaker {
            map.insert(
                "timeInForce".to_string(),
                serde_json::Value::String("PostOnly".to_string()),
            );
        }

        let body = serde_json::Value::Object(map);

        let response = self
            .signed_request(&path)
            .method(HttpMethod::Post)
            .body(body)
            .execute()
            .await?;

        let result = response
            .get("result")
            .ok_or_else(|| Error::from(ParseError::missing_field("result")))?;

        parser::parse_order(result, Some(&market))
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
        use crate::bybit::signed_request::HttpMethod;

        let market = self.base().market(symbol).await?;

        let path = self.build_api_path("/order/cancel");

        let mut map = serde_json::Map::new();
        map.insert(
            "category".to_string(),
            serde_json::Value::String(self.get_category().to_string()),
        );
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
            .signed_request(&path)
            .method(HttpMethod::Post)
            .body(body)
            .execute()
            .await?;

        let result = response
            .get("result")
            .ok_or_else(|| Error::from(ParseError::missing_field("result")))?;

        parser::parse_order(result, Some(&market))
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

        let path = self.build_api_path("/order/realtime");

        let response = self
            .signed_request(&path)
            .param("category", self.get_category())
            .param("symbol", &market.id)
            .param("orderId", id)
            .execute()
            .await?;

        let result = response
            .get("result")
            .ok_or_else(|| Error::from(ParseError::missing_field("result")))?;

        let list = result
            .get("list")
            .ok_or_else(|| Error::from(ParseError::missing_field("list")))?;

        let orders = list.as_array().ok_or_else(|| {
            Error::from(ParseError::invalid_format(
                "list",
                "Expected array of orders",
            ))
        })?;

        if orders.is_empty() {
            return Err(Error::exchange("110008", "Order not found"));
        }

        parser::parse_order(&orders[0], Some(&market))
    }

    /// Fetch open orders.
    ///
    /// # Arguments
    ///
    /// * `symbol` - Optional trading pair symbol. If None, fetches all open orders.
    /// * `since` - Optional start timestamp in milliseconds.
    /// * `limit` - Optional limit on number of orders (maximum: 50).
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
        let path = self.build_api_path("/order/realtime");

        // Bybit maximum limit is 50
        let actual_limit = limit.map(|l| l.min(50)).unwrap_or(50);

        let market = if let Some(sym) = symbol {
            Some(self.base().market(sym).await?)
        } else {
            None
        };

        let mut builder = self
            .signed_request(&path)
            .param("category", self.get_category())
            .param("limit", actual_limit)
            .optional_param("startTime", since);

        if let Some(ref m) = market {
            builder = builder.param("symbol", &m.id);
        }

        let response = builder.execute().await?;

        let result = response
            .get("result")
            .ok_or_else(|| Error::from(ParseError::missing_field("result")))?;

        let list = result
            .get("list")
            .ok_or_else(|| Error::from(ParseError::missing_field("list")))?;

        let orders_array = list.as_array().ok_or_else(|| {
            Error::from(ParseError::invalid_format(
                "list",
                "Expected array of orders",
            ))
        })?;

        let mut orders = Vec::new();
        for order_data in orders_array {
            match parser::parse_order(order_data, market.as_ref().map(|v| &**v)) {
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
    /// * `limit` - Optional limit on number of orders (maximum: 50).
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
        let path = self.build_api_path("/order/history");

        // Bybit maximum limit is 50
        let actual_limit = limit.map(|l| l.min(50)).unwrap_or(50);

        let market = if let Some(sym) = symbol {
            Some(self.base().market(sym).await?)
        } else {
            None
        };

        let mut builder = self
            .signed_request(&path)
            .param("category", self.get_category())
            .param("limit", actual_limit)
            .optional_param("startTime", since);

        if let Some(ref m) = market {
            builder = builder.param("symbol", &m.id);
        }

        let response = builder.execute().await?;

        let result = response
            .get("result")
            .ok_or_else(|| Error::from(ParseError::missing_field("result")))?;

        let list = result
            .get("list")
            .ok_or_else(|| Error::from(ParseError::missing_field("list")))?;

        let orders_array = list.as_array().ok_or_else(|| {
            Error::from(ParseError::invalid_format(
                "list",
                "Expected array of orders",
            ))
        })?;

        let mut orders = Vec::new();
        for order_data in orders_array {
            match parser::parse_order(order_data, market.as_ref().map(|v| &**v)) {
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
        let bybit = Bybit::builder().build().unwrap();
        let path = bybit.build_api_path("/market/instruments-info");
        assert_eq!(path, "/v5/market/instruments-info");
    }

    #[test]
    fn test_get_category_spot() {
        let bybit = Bybit::builder().build().unwrap();
        let category = bybit.get_category();
        assert_eq!(category, "spot");
    }

    #[test]
    fn test_get_category_linear() {
        let bybit = Bybit::builder().account_type("LINEAR").build().unwrap();
        let category = bybit.get_category();
        assert_eq!(category, "linear");
    }

    #[test]
    fn test_get_timestamp() {
        let bybit = Bybit::builder().build().unwrap();
        let ts = bybit.get_timestamp();

        // Should be a valid timestamp string
        assert!(!ts.is_empty());
        let parsed: i64 = ts.parse().unwrap();
        assert!(parsed > 0);
    }
}
