//! Binance REST API implementation.
//!
//! Implements all REST API endpoint operations for the Binance exchange.

use super::{Binance, auth::BinanceAuth, parser};
use ccxt_core::{
    Error, ParseError, Result,
    types::{
        AccountConfig, AggTrade, Balance, BatchCancelResult, BatchOrderRequest, BatchOrderResult,
        BatchOrderUpdate, BidAsk, BorrowInterest, BorrowRateHistory, CancelAllOrdersResult,
        CommissionRate, Currency, DepositAddress, FeeFundingRate, FeeFundingRateHistory,
        FeeTradingFee, FundingFee, IntoTickerParams, LastPrice, LeverageTier, MarginAdjustment,
        MarginLoan, MarginRepay, MarkPrice, Market, MarketType, MaxBorrowable, MaxLeverage,
        MaxTransferable, NextFundingRate, OcoOrder, OpenInterest, OpenInterestHistory, Order,
        OrderBook, OrderSide, OrderStatus, OrderType, Stats24hr, Ticker, Trade, TradingLimits,
        Transaction, TransactionType, Transfer,
    },
};
use reqwest::header::HeaderMap;
use rust_decimal::Decimal;
use serde_json::Value;
use std::{collections::HashMap, sync::Arc};
use tracing::{debug, info, warn};

impl Binance {
    /// Fetch server timestamp for internal use.
    ///
    /// # Returns
    ///
    /// Returns the server timestamp in milliseconds.
    ///
    /// # Errors
    ///
    /// Returns an error if the request fails or the response is malformed.
    async fn fetch_time_raw(&self) -> Result<u64> {
        let url = format!("{}/time", self.urls().public);
        let response = self.base().http_client.get(&url, None).await?;

        response["serverTime"]
            .as_u64()
            .ok_or_else(|| ParseError::missing_field("serverTime").into())
    }

    /// Fetch exchange system status.
    ///
    /// # Returns
    ///
    /// Returns formatted exchange status information with the following structure:
    /// ```json
    /// {
    ///     "status": "ok" | "maintenance",
    ///     "updated": null,
    ///     "eta": null,
    ///     "url": null,
    ///     "info": { ... }
    /// }
    /// ```
    pub async fn fetch_status(&self) -> Result<Value> {
        let url = format!("{}/system/status", self.urls().sapi);
        let response = self.base().http_client.get(&url, None).await?;

        // Response format: { "status": 0, "msg": "normal" }
        // Status codes: 0 = normal, 1 = system maintenance
        let status_raw = response
            .get("status")
            .and_then(|v| v.as_i64())
            .ok_or_else(|| {
                Error::from(ParseError::invalid_format(
                    "status",
                    "status field missing or not an integer",
                ))
            })?;

        let status = match status_raw {
            0 => "ok",
            1 => "maintenance",
            _ => "unknown",
        };

        // json! macro with literal values is infallible
        #[allow(clippy::disallowed_methods)]
        Ok(serde_json::json!({
            "status": status,
            "updated": null,
            "eta": null,
            "url": null,
            "info": response
        }))
    }

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
    /// # use ccxt_exchanges::binance::Binance;
    /// # use ccxt_core::ExchangeConfig;
    /// # async fn example() -> ccxt_core::Result<()> {
    /// let binance = Binance::new(ExchangeConfig::default())?;
    /// let markets = binance.fetch_markets().await?;
    /// println!("Found {} markets", markets.len());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn fetch_markets(&self) -> Result<HashMap<String, Arc<Market>>> {
        let url = format!("{}/exchangeInfo", self.urls().public);
        let data = self.base().http_client.get(&url, None).await?;

        let symbols = data["symbols"]
            .as_array()
            .ok_or_else(|| Error::from(ParseError::missing_field("symbols")))?;

        let mut markets = Vec::new();
        for symbol in symbols {
            match parser::parse_market(symbol) {
                Ok(market) => markets.push(market),
                Err(e) => {
                    warn!(error = %e, "Failed to parse market");
                }
            }
        }

        self.base().set_markets(markets, None).await
    }
    /// Load and cache market data.
    ///
    /// Standard CCXT method for loading all market data from the exchange.
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
    /// # use ccxt_exchanges::binance::Binance;
    /// # use ccxt_core::ExchangeConfig;
    /// # async fn example() -> ccxt_core::error::Result<()> {
    /// let binance = Binance::new(ExchangeConfig::default())?;
    ///
    /// // Load markets for the first time
    /// let markets = binance.load_markets(false).await?;
    /// println!("Loaded {} markets", markets.len());
    ///
    /// // Subsequent calls use cache (no API request)
    /// let markets = binance.load_markets(false).await?;
    ///
    /// // Force reload
    /// let markets = binance.load_markets(true).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn load_markets(&self, reload: bool) -> Result<HashMap<String, Arc<Market>>> {
        // Acquire the loading lock to serialize concurrent load_markets calls
        // This prevents multiple tasks from making duplicate API calls
        let _loading_guard = self.base().market_loading_lock.lock().await;

        // Check cache status while holding the lock
        {
            let cache = self.base().market_cache.read().await;
            if cache.loaded && !reload {
                debug!(
                    "Returning cached markets for Binance ({} markets)",
                    cache.markets.len()
                );
                return Ok(cache.markets.clone());
            }
        }

        info!("Loading markets for Binance (reload: {})", reload);
        let _markets = self.fetch_markets().await?;

        let cache = self.base().market_cache.read().await;
        Ok(cache.markets.clone())
    }

    /// Fetch ticker for a single trading pair.
    ///
    /// # Arguments
    ///
    /// * `symbol` - Trading pair symbol (e.g., "BTC/USDT").
    /// * `params` - Optional parameters to configure the ticker request.
    ///
    /// # Returns
    ///
    /// Returns [`Ticker`] data for the specified symbol.
    ///
    /// # Errors
    ///
    /// Returns an error if the market is not found or the API request fails.
    pub async fn fetch_ticker(
        &self,
        symbol: &str,
        params: impl IntoTickerParams,
    ) -> Result<Ticker> {
        let market = self.base().market(symbol).await?;

        let params = params.into_ticker_params();
        let rolling = params.rolling.unwrap_or(false);

        let endpoint = if rolling { "ticker" } else { "ticker/24hr" };

        let mut url = format!("{}/{}?symbol={}", self.urls().public, endpoint, market.id);

        if let Some(window) = params.window_size {
            url.push_str(&format!("&windowSize={}", window));
        }

        for (key, value) in &params.extras {
            if key != "rolling" && key != "windowSize" {
                url.push_str(&format!("&{}={}", key, value));
            }
        }

        let data = self.base().http_client.get(&url, None).await?;

        parser::parse_ticker(&data, Some(&market))
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
    pub async fn fetch_tickers(&self, symbols: Option<Vec<String>>) -> Result<Vec<Ticker>> {
        // Acquire read lock once and clone the necessary data to avoid lock contention in the loop
        let markets_by_id = {
            let cache = self.base().market_cache.read().await;
            if !cache.loaded {
                return Err(Error::exchange(
                    "-1",
                    "Markets not loaded. Call load_markets() first.",
                ));
            }
            cache.markets_by_id.clone()
        };

        let url = format!("{}/ticker/24hr", self.urls().public);
        let data = self.base().http_client.get(&url, None).await?;

        let tickers_array = data.as_array().ok_or_else(|| {
            Error::from(ParseError::invalid_format(
                "response",
                "Expected array of tickers",
            ))
        })?;

        let mut tickers = Vec::new();
        for ticker_data in tickers_array {
            if let Some(binance_symbol) = ticker_data["symbol"].as_str() {
                // Use the pre-cloned map instead of acquiring a lock on each iteration
                if let Some(market) = markets_by_id.get(binance_symbol) {
                    match parser::parse_ticker(ticker_data, Some(market)) {
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
                                symbol = %binance_symbol,
                                "Failed to parse ticker"
                            );
                        }
                    }
                }
            }
        }

        Ok(tickers)
    }

    /// Fetch order book for a trading pair.
    ///
    /// # Arguments
    ///
    /// * `symbol` - Trading pair symbol.
    /// * `limit` - Optional depth limit (valid values: 5, 10, 20, 50, 100, 500, 1000, 5000).
    ///
    /// # Returns
    ///
    /// Returns [`OrderBook`] data containing bids and asks.
    ///
    /// # Errors
    ///
    /// Returns an error if the market is not found or the API request fails.
    pub async fn fetch_order_book(&self, symbol: &str, limit: Option<u32>) -> Result<OrderBook> {
        let market = self.base().market(symbol).await?;

        let url = if let Some(l) = limit {
            format!(
                "{}/depth?symbol={}&limit={}",
                self.urls().public,
                market.id,
                l
            )
        } else {
            format!("{}/depth?symbol={}", self.urls().public, market.id)
        };

        let data = self.base().http_client.get(&url, None).await?;

        parser::parse_orderbook(&data, market.symbol.clone())
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
    ///
    /// # Errors
    ///
    /// Returns an error if the market is not found or the API request fails.
    pub async fn fetch_trades(&self, symbol: &str, limit: Option<u32>) -> Result<Vec<Trade>> {
        let market = self.base().market(symbol).await?;

        let url = if let Some(l) = limit {
            format!(
                "{}/trades?symbol={}&limit={}",
                self.urls().public,
                market.id,
                l
            )
        } else {
            format!("{}/trades?symbol={}", self.urls().public, market.id)
        };

        let data = self.base().http_client.get(&url, None).await?;

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

        // CCXT convention: trades should be sorted by timestamp descending (newest first)
        trades.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

        Ok(trades)
    }
    /// Fetch recent public trades (alias for `fetch_trades`).
    ///
    /// # Arguments
    ///
    /// * `symbol` - Trading pair symbol.
    /// * `limit` - Optional limit on number of trades (default: 500, maximum: 1000).
    ///
    /// # Returns
    ///
    /// Returns a vector of [`Trade`] structures for recent public trades.
    ///
    /// # Errors
    ///
    /// Returns an error if the market is not found or the API request fails.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use ccxt_exchanges::binance::Binance;
    /// # use ccxt_core::ExchangeConfig;
    /// # async fn example() -> ccxt_core::Result<()> {
    /// # let binance = Binance::new(ExchangeConfig::default())?;
    /// let recent_trades = binance.fetch_recent_trades("BTC/USDT", Some(100)).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn fetch_recent_trades(
        &self,
        symbol: &str,
        limit: Option<u32>,
    ) -> Result<Vec<Trade>> {
        self.fetch_trades(symbol, limit).await
    }

    /// Fetch authenticated user's recent trades (private API).
    ///
    /// # Arguments
    ///
    /// * `symbol` - Trading pair symbol.
    /// * `since` - Optional start timestamp in milliseconds.
    /// * `limit` - Optional limit on number of trades (default: 500, maximum: 1000).
    /// * `params` - Additional parameters.
    ///
    /// # Returns
    ///
    /// Returns a vector of [`Trade`] structures for the user's trades.
    ///
    /// # Errors
    ///
    /// Returns an error if authentication fails or the API request fails.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use ccxt_exchanges::binance::Binance;
    /// # use ccxt_core::ExchangeConfig;
    /// # async fn example() -> ccxt_core::Result<()> {
    /// # let binance = Binance::new(ExchangeConfig::default())?;
    /// let my_trades = binance.fetch_my_recent_trades("BTC/USDT", None, Some(50), None).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn fetch_my_recent_trades(
        &self,
        symbol: &str,
        since: Option<u64>,
        limit: Option<u32>,
        params: Option<HashMap<String, String>>,
    ) -> Result<Vec<Trade>> {
        let market = self.base().market(symbol).await?;

        self.check_required_credentials()?;

        let mut request_params = HashMap::new();
        request_params.insert("symbol".to_string(), market.id.clone());

        if let Some(s) = since {
            request_params.insert("startTime".to_string(), s.to_string());
        }

        if let Some(l) = limit {
            request_params.insert("limit".to_string(), l.to_string());
        }

        if let Some(p) = params {
            for (k, v) in p {
                request_params.insert(k, v);
            }
        }

        let url = format!("{}/myTrades", self.urls().private);
        let timestamp = self.fetch_time_raw().await?;
        let auth = self.get_auth()?;
        let signed_params =
            auth.sign_with_timestamp(&request_params, timestamp, Some(self.options().recv_window))?;

        let mut request_url = format!("{}?", url);
        for (key, value) in &signed_params {
            request_url.push_str(&format!("{}={}&", key, value));
        }

        let mut headers = HeaderMap::new();
        auth.add_auth_headers_reqwest(&mut headers);

        let data = self
            .base()
            .http_client
            .get(&request_url, Some(headers))
            .await?;

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

    /// Fetch aggregated trade data.
    ///
    /// # Arguments
    ///
    /// * `symbol` - Trading pair symbol.
    /// * `since` - Optional start timestamp in milliseconds.
    /// * `limit` - Optional limit on number of records (default: 500, maximum: 1000).
    /// * `params` - Additional parameters that may include:
    ///   - `fromId`: Start from specific aggTradeId.
    ///   - `endTime`: End timestamp in milliseconds.
    ///
    /// # Returns
    ///
    /// Returns a vector of aggregated trade records.
    ///
    /// # Errors
    ///
    /// Returns an error if the market is not found or the API request fails.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use ccxt_exchanges::binance::Binance;
    /// # use ccxt_core::ExchangeConfig;
    /// # async fn example() -> ccxt_core::Result<()> {
    /// # let binance = Binance::new(ExchangeConfig::default())?;
    /// let agg_trades = binance.fetch_agg_trades("BTC/USDT", None, Some(100), None).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn fetch_agg_trades(
        &self,
        symbol: &str,
        since: Option<u64>,
        limit: Option<u32>,
        params: Option<HashMap<String, String>>,
    ) -> Result<Vec<AggTrade>> {
        let market = self.base().market(symbol).await?;

        let mut url = format!("{}/aggTrades?symbol={}", self.urls().public, market.id);

        if let Some(s) = since {
            url.push_str(&format!("&startTime={}", s));
        }

        if let Some(l) = limit {
            url.push_str(&format!("&limit={}", l));
        }

        if let Some(p) = params {
            if let Some(from_id) = p.get("fromId") {
                url.push_str(&format!("&fromId={}", from_id));
            }
            if let Some(end_time) = p.get("endTime") {
                url.push_str(&format!("&endTime={}", end_time));
            }
        }

        let data = self.base().http_client.get(&url, None).await?;

        let agg_trades_array = data.as_array().ok_or_else(|| {
            Error::from(ParseError::invalid_format(
                "data",
                "Expected array of agg trades",
            ))
        })?;

        let mut agg_trades = Vec::new();
        for agg_trade_data in agg_trades_array {
            match parser::parse_agg_trade(agg_trade_data, Some(market.symbol.clone())) {
                Ok(agg_trade) => agg_trades.push(agg_trade),
                Err(e) => {
                    warn!(error = %e, "Failed to parse agg trade");
                }
            }
        }

        Ok(agg_trades)
    }

    /// Fetch historical trade data (requires API key but not signature).
    ///
    /// Note: Binance API uses `fromId` parameter instead of timestamp.
    ///
    /// # Arguments
    ///
    /// * `symbol` - Trading pair symbol.
    /// * `_since` - Optional start timestamp (unused, Binance uses `fromId` instead).
    /// * `limit` - Optional limit on number of records (default: 500, maximum: 1000).
    /// * `params` - Additional parameters that may include:
    ///   - `fromId`: Start from specific tradeId.
    ///
    /// # Returns
    ///
    /// Returns a vector of historical [`Trade`] records.
    ///
    /// # Errors
    ///
    /// Returns an error if authentication fails or the API request fails.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use ccxt_exchanges::binance::Binance;
    /// # use ccxt_core::ExchangeConfig;
    /// # async fn example() -> ccxt_core::Result<()> {
    /// # let binance = Binance::new(ExchangeConfig::default())?;
    /// let historical_trades = binance.fetch_historical_trades("BTC/USDT", None, Some(100), None).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn fetch_historical_trades(
        &self,
        symbol: &str,
        _since: Option<u64>,
        limit: Option<u32>,
        params: Option<HashMap<String, String>>,
    ) -> Result<Vec<Trade>> {
        let market = self.base().market(symbol).await?;

        self.check_required_credentials()?;

        let mut url = format!(
            "{}/historicalTrades?symbol={}",
            self.urls().public,
            market.id
        );

        // Binance historicalTrades endpoint uses fromId instead of timestamp
        if let Some(p) = &params {
            if let Some(from_id) = p.get("fromId") {
                url.push_str(&format!("&fromId={}", from_id));
            }
        }

        if let Some(l) = limit {
            url.push_str(&format!("&limit={}", l));
        }

        let mut headers = HeaderMap::new();
        let auth = self.get_auth()?;
        auth.add_auth_headers_reqwest(&mut headers);

        let data = self.base().http_client.get(&url, Some(headers)).await?;

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
                    warn!(error = %e, "Failed to parse historical trade");
                }
            }
        }

        Ok(trades)
    }

    /// Fetch 24-hour trading statistics.
    ///
    /// # Arguments
    ///
    /// * `symbol` - Optional trading pair symbol. If `None`, returns statistics for all pairs.
    ///
    /// # Returns
    ///
    /// Returns a vector of [`Stats24hr`] structures. Single symbol returns one item, all symbols return multiple items.
    ///
    /// # Errors
    ///
    /// Returns an error if the market is not found or the API request fails.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use ccxt_exchanges::binance::Binance;
    /// # use ccxt_core::ExchangeConfig;
    /// # async fn example() -> ccxt_core::Result<()> {
    /// # let binance = Binance::new(ExchangeConfig::default())?;
    /// // Fetch statistics for a single pair
    /// let stats = binance.fetch_24hr_stats(Some("BTC/USDT")).await?;
    ///
    /// // Fetch statistics for all pairs
    /// let all_stats = binance.fetch_24hr_stats(None).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn fetch_24hr_stats(&self, symbol: Option<&str>) -> Result<Vec<Stats24hr>> {
        let url = if let Some(sym) = symbol {
            let market = self.base().market(sym).await?;
            format!("{}/ticker/24hr?symbol={}", self.urls().public, market.id)
        } else {
            format!("{}/ticker/24hr", self.urls().public)
        };

        let data = self.base().http_client.get(&url, None).await?;

        // Single symbol returns object, all symbols return array
        let stats_vec = if data.is_array() {
            let stats_array = data.as_array().ok_or_else(|| {
                Error::from(ParseError::invalid_format(
                    "data",
                    "Expected array of 24hr stats",
                ))
            })?;

            let mut stats = Vec::new();
            for stats_data in stats_array {
                match parser::parse_stats_24hr(stats_data) {
                    Ok(stat) => stats.push(stat),
                    Err(e) => {
                        warn!(error = %e, "Failed to parse 24hr stats");
                    }
                }
            }
            stats
        } else {
            vec![parser::parse_stats_24hr(&data)?]
        };

        Ok(stats_vec)
    }

    /// Fetch trading limits information for a symbol.
    ///
    /// # Arguments
    ///
    /// * `symbol` - Trading pair symbol.
    ///
    /// # Returns
    ///
    /// Returns [`TradingLimits`] containing minimum/maximum order constraints.
    ///
    /// # Errors
    ///
    /// Returns an error if the market is not found or the API request fails.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use ccxt_exchanges::binance::Binance;
    /// # use ccxt_core::ExchangeConfig;
    /// # async fn example() -> ccxt_core::Result<()> {
    /// # let binance = Binance::new(ExchangeConfig::default())?;
    /// let limits = binance.fetch_trading_limits("BTC/USDT").await?;
    /// if let Some(ref amount) = limits.amount {
    ///     println!("Min amount: {:?}", amount.min);
    ///     println!("Max amount: {:?}", amount.max);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn fetch_trading_limits(&self, symbol: &str) -> Result<TradingLimits> {
        let market = self.base().market(symbol).await?;

        let url = format!("{}/exchangeInfo?symbol={}", self.urls().public, market.id);
        let data = self.base().http_client.get(&url, None).await?;

        let symbols_array = data["symbols"].as_array().ok_or_else(|| {
            Error::from(ParseError::invalid_format("data", "Expected symbols array"))
        })?;

        if symbols_array.is_empty() {
            return Err(Error::from(ParseError::invalid_format(
                "data",
                format!("No symbol info found for {}", symbol),
            )));
        }

        let symbol_data = &symbols_array[0];

        parser::parse_trading_limits(symbol_data, market.symbol.clone())
    }

    /// Create a new order.
    ///
    /// # Arguments
    ///
    /// * `symbol` - Trading pair symbol.
    /// * `order_type` - Order type (Market, Limit, StopLoss, etc.).
    /// * `side` - Order side (Buy or Sell).
    /// * `amount` - Order quantity.
    /// * `price` - Optional price (required for limit orders).
    /// * `params` - Additional parameters.
    ///
    /// # Returns
    ///
    /// Returns the created [`Order`] structure with order details.
    ///
    /// # Errors
    ///
    /// Returns an error if authentication fails, market is not found, or the API request fails.
    pub async fn create_order(
        &self,
        symbol: &str,
        order_type: OrderType,
        side: OrderSide,
        amount: f64,
        price: Option<f64>,
        params: Option<HashMap<String, String>>,
    ) -> Result<Order> {
        self.check_required_credentials()?;

        let market = self.base().market(symbol).await?;
        let mut request_params = HashMap::new();

        request_params.insert("symbol".to_string(), market.id.clone());
        request_params.insert(
            "side".to_string(),
            match side {
                OrderSide::Buy => "BUY".to_string(),
                OrderSide::Sell => "SELL".to_string(),
            },
        );
        request_params.insert(
            "type".to_string(),
            match order_type {
                OrderType::Market => "MARKET".to_string(),
                OrderType::Limit => "LIMIT".to_string(),
                OrderType::StopLoss => "STOP_LOSS".to_string(),
                OrderType::StopLossLimit => "STOP_LOSS_LIMIT".to_string(),
                OrderType::TakeProfit => "TAKE_PROFIT".to_string(),
                OrderType::TakeProfitLimit => "TAKE_PROFIT_LIMIT".to_string(),
                OrderType::LimitMaker => "LIMIT_MAKER".to_string(),
                OrderType::StopMarket => "STOP_MARKET".to_string(),
                OrderType::StopLimit => "STOP_LIMIT".to_string(),
                OrderType::TrailingStop => "TRAILING_STOP_MARKET".to_string(),
            },
        );
        request_params.insert("quantity".to_string(), amount.to_string());

        if let Some(p) = price {
            request_params.insert("price".to_string(), p.to_string());
        }

        // Limit orders require timeInForce parameter
        if order_type == OrderType::Limit
            || order_type == OrderType::StopLossLimit
            || order_type == OrderType::TakeProfitLimit
        {
            if !request_params.contains_key("timeInForce") {
                request_params.insert("timeInForce".to_string(), "GTC".to_string());
            }
        }

        if let Some(extra) = params {
            for (k, v) in extra {
                request_params.insert(k, v);
            }
        }

        // Handle cost parameter for market buy orders (quoteOrderQty)
        // Convert cost parameter to Binance API's quoteOrderQty
        if order_type == OrderType::Market && side == OrderSide::Buy {
            if let Some(cost_str) = request_params.get("cost") {
                request_params.insert("quoteOrderQty".to_string(), cost_str.clone());
                // Remove quantity parameter (not needed with quoteOrderQty)
                request_params.remove("quantity");
                // Remove cost parameter (Binance API doesn't recognize it)
                request_params.remove("cost");
            }
        }

        // Handle conditional order parameters
        // stopPrice: trigger price for stop-loss/take-profit orders
        if matches!(
            order_type,
            OrderType::StopLoss
                | OrderType::StopLossLimit
                | OrderType::TakeProfit
                | OrderType::TakeProfitLimit
                | OrderType::StopMarket
        ) {
            // Use stopLossPrice or takeProfitPrice if stopPrice not provided
            if !request_params.contains_key("stopPrice") {
                if let Some(stop_loss) = request_params.get("stopLossPrice") {
                    request_params.insert("stopPrice".to_string(), stop_loss.clone());
                } else if let Some(take_profit) = request_params.get("takeProfitPrice") {
                    request_params.insert("stopPrice".to_string(), take_profit.clone());
                }
            }
        }

        // trailingDelta: price offset for trailing stop (basis points)
        // Spot market: requires trailingDelta parameter
        // Futures market: uses callbackRate parameter
        if order_type == OrderType::TrailingStop {
            if market.is_spot() {
                // Spot trailing stop: use trailingDelta
                if !request_params.contains_key("trailingDelta") {
                    // Convert trailingPercent to trailingDelta (basis points) if provided
                    if let Some(percent_str) = request_params.get("trailingPercent") {
                        if let Ok(percent) = percent_str.parse::<f64>() {
                            let delta = (percent * 100.0) as i64;
                            request_params.insert("trailingDelta".to_string(), delta.to_string());
                            request_params.remove("trailingPercent");
                        }
                    }
                }
            } else if market.is_swap() || market.is_futures() {
                // Futures trailing stop: use callbackRate
                if !request_params.contains_key("callbackRate") {
                    if let Some(percent_str) = request_params.get("trailingPercent") {
                        request_params.insert("callbackRate".to_string(), percent_str.clone());
                        request_params.remove("trailingPercent");
                    }
                }
            }
        }

        // Futures order advanced parameters (Stage 24.3)
        if market.is_swap() || market.is_futures() {
            // reduceOnly: reduce-only flag (only reduces existing position, won't reverse)
            if let Some(reduce_only) = request_params.get("reduceOnly") {
                // Keep original value, Binance API accepts "true" or "false" strings
                request_params.insert("reduceOnly".to_string(), reduce_only.clone());
            }

            // postOnly: maker-only flag (ensures order won't execute immediately)
            if let Some(post_only) = request_params.get("postOnly") {
                request_params.insert("postOnly".to_string(), post_only.clone());
            }

            // positionSide: position direction (LONG/SHORT/BOTH)
            // Required in hedge mode, defaults to BOTH in one-way mode
            if let Some(position_side) = request_params.get("positionSide") {
                request_params.insert("positionSide".to_string(), position_side.clone());
            }

            // closePosition: close all positions flag (market orders only)
            // When true, closes all positions; quantity can be omitted
            if let Some(close_position) = request_params.get("closePosition") {
                if order_type == OrderType::Market {
                    request_params.insert("closePosition".to_string(), close_position.clone());
                }
            }
        }

        let timestamp = self.fetch_time_raw().await?;
        let auth = self.get_auth()?;
        let signed_params =
            auth.sign_with_timestamp(&request_params, timestamp, Some(self.options().recv_window))?;

        let url = format!("{}/order", self.urls().private);
        let mut headers = HeaderMap::new();
        auth.add_auth_headers_reqwest(&mut headers);

        let body = serde_json::to_value(&signed_params).map_err(|e| {
            Error::from(ParseError::invalid_format(
                "data",
                format!("Failed to serialize params: {}", e),
            ))
        })?;

        let data = self
            .base()
            .http_client
            .post(&url, Some(headers), Some(body))
            .await?;

        parser::parse_order(&data, Some(&market))
    }

    /// Cancel an order.
    ///
    /// # Arguments
    ///
    /// * `id` - Order ID.
    /// * `symbol` - Trading pair symbol.
    ///
    /// # Returns
    ///
    /// Returns the cancelled [`Order`] information.
    ///
    /// # Errors
    ///
    /// Returns an error if authentication fails, market is not found, or the API request fails.
    pub async fn cancel_order(&self, id: &str, symbol: &str) -> Result<Order> {
        self.check_required_credentials()?;

        let market = self.base().market(symbol).await?;
        let mut params = HashMap::new();
        params.insert("symbol".to_string(), market.id.clone());
        params.insert("orderId".to_string(), id.to_string());

        let timestamp = self.fetch_time_raw().await?;
        let auth = self.get_auth()?;
        let signed_params =
            auth.sign_with_timestamp(&params, timestamp, Some(self.options().recv_window))?;

        let url = format!("{}/order", self.urls().private);
        let mut headers = HeaderMap::new();
        auth.add_auth_headers_reqwest(&mut headers);

        let body = serde_json::to_value(&signed_params).map_err(|e| {
            Error::from(ParseError::invalid_format(
                "data",
                format!("Failed to serialize params: {}", e),
            ))
        })?;

        let data = self
            .base()
            .http_client
            .delete(&url, Some(headers), Some(body))
            .await?;

        parser::parse_order(&data, Some(&market))
    }

    /// Fetch order details.
    ///
    /// # Arguments
    ///
    /// * `id` - Order ID.
    /// * `symbol` - Trading pair symbol.
    ///
    /// # Returns
    ///
    /// Returns the [`Order`] information.
    ///
    /// # Errors
    ///
    /// Returns an error if authentication fails, market is not found, or the API request fails.
    pub async fn fetch_order(&self, id: &str, symbol: &str) -> Result<Order> {
        self.check_required_credentials()?;

        let market = self.base().market(symbol).await?;
        let mut params = HashMap::new();
        params.insert("symbol".to_string(), market.id.clone());
        params.insert("orderId".to_string(), id.to_string());

        let timestamp = self.fetch_time_raw().await?;
        let auth = self.get_auth()?;
        let signed_params =
            auth.sign_with_timestamp(&params, timestamp, Some(self.options().recv_window))?;

        let mut url = format!("{}/order?", self.urls().private);
        for (key, value) in &signed_params {
            url.push_str(&format!("{}={}&", key, value));
        }

        let mut headers = HeaderMap::new();
        auth.add_auth_headers_reqwest(&mut headers);

        let data = self.base().http_client.get(&url, Some(headers)).await?;

        parser::parse_order(&data, Some(&market))
    }

    /// Fetch open (unfilled) orders.
    ///
    /// # Arguments
    ///
    /// * `symbol` - Optional trading pair symbol. If `None`, fetches all open orders.
    ///
    /// # Returns
    ///
    /// Returns a vector of open [`Order`] structures.
    ///
    /// # Errors
    ///
    /// Returns an error if authentication fails or the API request fails.
    pub async fn fetch_open_orders(&self, symbol: Option<&str>) -> Result<Vec<Order>> {
        self.check_required_credentials()?;

        let mut params = HashMap::new();
        let market = if let Some(sym) = symbol {
            let m = self.base().market(sym).await?;
            params.insert("symbol".to_string(), m.id.clone());
            Some(m)
        } else {
            None
        };

        let timestamp = self.fetch_time_raw().await?;
        let auth = self.get_auth()?;
        let signed_params =
            auth.sign_with_timestamp(&params, timestamp, Some(self.options().recv_window))?;

        let mut url = format!("{}/openOrders?", self.urls().private);
        for (key, value) in &signed_params {
            url.push_str(&format!("{}={}&", key, value));
        }

        let mut headers = HeaderMap::new();
        auth.add_auth_headers_reqwest(&mut headers);

        let data = self.base().http_client.get(&url, Some(headers)).await?;

        let orders_array = data.as_array().ok_or_else(|| {
            Error::from(ParseError::invalid_format(
                "data",
                "Expected array of orders",
            ))
        })?;

        let mut orders = Vec::new();
        for order_data in orders_array {
            match parser::parse_order(order_data, market.as_ref().map(|v| &**v)) {
                Ok(order) => orders.push(order),
                Err(e) => {
                    warn!(error = %e, "Failed to parse order");
                }
            }
        }

        Ok(orders)
    }
    /// Create a stop-loss order.
    ///
    /// # Arguments
    ///
    /// * `symbol` - Trading pair symbol.
    /// * `side` - Order side (Buy/Sell).
    /// * `amount` - Order quantity.
    /// * `stop_price` - Stop-loss trigger price.
    /// * `price` - Optional limit price (if `None`, creates market stop-loss order).
    /// * `params` - Optional additional parameters.
    ///
    /// # Returns
    ///
    /// Returns the created stop-loss [`Order`].
    ///
    /// # Errors
    ///
    /// Returns an error if authentication fails or the API request fails.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use ccxt_exchanges::binance::Binance;
    /// # use ccxt_core::ExchangeConfig;
    /// # use ccxt_core::types::{OrderSide, OrderType};
    /// # async fn example() -> ccxt_core::Result<()> {
    /// # let exchange = Binance::new(ExchangeConfig::default())?;
    /// // Create market stop-loss order
    /// let order = exchange.create_stop_loss_order(
    ///     "BTC/USDT",
    ///     OrderSide::Sell,
    ///     0.1,
    ///     45000.0,
    ///     None,
    ///     None
    /// ).await?;
    ///
    /// // Create limit stop-loss order
    /// let order = exchange.create_stop_loss_order(
    ///     "BTC/USDT",
    ///     OrderSide::Sell,
    ///     0.1,
    ///     45000.0,
    ///     Some(44900.0),
    ///     None
    /// ).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn create_stop_loss_order(
        &self,
        symbol: &str,
        side: OrderSide,
        amount: f64,
        stop_price: f64,
        price: Option<f64>,
        params: Option<HashMap<String, String>>,
    ) -> Result<Order> {
        let mut request_params = params.unwrap_or_default();

        request_params.insert("stopPrice".to_string(), stop_price.to_string());

        let order_type = if price.is_some() {
            OrderType::StopLossLimit
        } else {
            OrderType::StopLoss
        };

        self.create_order(
            symbol,
            order_type,
            side,
            amount,
            price,
            Some(request_params),
        )
        .await
    }

    /// Create a take-profit order.
    ///
    /// # Arguments
    ///
    /// * `symbol` - Trading pair symbol.
    /// * `side` - Order side (Buy/Sell).
    /// * `amount` - Order quantity.
    /// * `take_profit_price` - Take-profit trigger price.
    /// * `price` - Optional limit price (if `None`, creates market take-profit order).
    /// * `params` - Optional additional parameters.
    ///
    /// # Returns
    ///
    /// Returns the created take-profit [`Order`].
    ///
    /// # Errors
    ///
    /// Returns an error if authentication fails or the API request fails.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use ccxt_exchanges::binance::Binance;
    /// # use ccxt_core::ExchangeConfig;
    /// # use ccxt_core::types::{OrderSide, OrderType};
    /// # async fn example() -> ccxt_core::Result<()> {
    /// # let exchange = Binance::new(ExchangeConfig::default())?;
    /// // Create market take-profit order
    /// let order = exchange.create_take_profit_order(
    ///     "BTC/USDT",
    ///     OrderSide::Sell,
    ///     0.1,
    ///     55000.0,
    ///     None,
    ///     None
    /// ).await?;
    ///
    /// // Create limit take-profit order
    /// let order = exchange.create_take_profit_order(
    ///     "BTC/USDT",
    ///     OrderSide::Sell,
    ///     0.1,
    ///     55000.0,
    ///     Some(55100.0),
    ///     None
    /// ).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn create_take_profit_order(
        &self,
        symbol: &str,
        side: OrderSide,
        amount: f64,
        take_profit_price: f64,
        price: Option<f64>,
        params: Option<HashMap<String, String>>,
    ) -> Result<Order> {
        let mut request_params = params.unwrap_or_default();

        request_params.insert("stopPrice".to_string(), take_profit_price.to_string());

        let order_type = if price.is_some() {
            OrderType::TakeProfitLimit
        } else {
            OrderType::TakeProfit
        };

        self.create_order(
            symbol,
            order_type,
            side,
            amount,
            price,
            Some(request_params),
        )
        .await
    }

    /// Create a trailing stop order.
    ///
    /// # Arguments
    ///
    /// * `symbol` - Trading pair symbol.
    /// * `side` - Order side (Buy/Sell).
    /// * `amount` - Order quantity.
    /// * `trailing_percent` - Trailing percentage (e.g., 2.0 for 2%).
    /// * `activation_price` - Optional activation price (not supported for spot markets).
    /// * `params` - Optional additional parameters.
    ///
    /// # Returns
    ///
    /// Returns the created trailing stop [`Order`].
    ///
    /// # Errors
    ///
    /// Returns an error if authentication fails or the API request fails.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use ccxt_exchanges::binance::Binance;
    /// # use ccxt_core::ExchangeConfig;
    /// # use ccxt_core::types::{OrderSide, OrderType};
    /// # async fn example() -> ccxt_core::Result<()> {
    /// # let exchange = Binance::new(ExchangeConfig::default())?;
    /// // Spot market: create trailing stop order with 2% trail
    /// let order = exchange.create_trailing_stop_order(
    ///     "BTC/USDT",
    ///     OrderSide::Sell,
    ///     0.1,
    ///     2.0,
    ///     None,
    ///     None
    /// ).await?;
    ///
    /// // Futures market: create trailing stop order with 1.5% trail, activation price 50000
    /// let order = exchange.create_trailing_stop_order(
    ///     "BTC/USDT:USDT",
    ///     OrderSide::Sell,
    ///     0.1,
    ///     1.5,
    ///     Some(50000.0),
    ///     None
    /// ).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn create_trailing_stop_order(
        &self,
        symbol: &str,
        side: OrderSide,
        amount: f64,
        trailing_percent: f64,
        activation_price: Option<f64>,
        params: Option<HashMap<String, String>>,
    ) -> Result<Order> {
        let mut request_params = params.unwrap_or_default();

        request_params.insert("trailingPercent".to_string(), trailing_percent.to_string());

        if let Some(activation) = activation_price {
            request_params.insert("activationPrice".to_string(), activation.to_string());
        }

        self.create_order(
            symbol,
            OrderType::TrailingStop,
            side,
            amount,
            None,
            Some(request_params),
        )
        .await
    }

    /// Fetch account balance.
    ///
    /// # Returns
    ///
    /// Returns the account [`Balance`] information.
    ///
    /// # Errors
    ///
    /// Returns an error if authentication fails or the API request fails.
    pub async fn fetch_balance_simple(&self) -> Result<Balance> {
        self.check_required_credentials()?;

        let params = HashMap::new();
        let timestamp = self.fetch_time_raw().await?;
        let auth = self.get_auth()?;
        let signed_params =
            auth.sign_with_timestamp(&params, timestamp, Some(self.options().recv_window))?;

        let mut url = format!("{}/account?", self.urls().private);
        for (key, value) in &signed_params {
            url.push_str(&format!("{}={}&", key, value));
        }

        let mut headers = HeaderMap::new();
        auth.add_auth_headers_reqwest(&mut headers);

        let data = self.base().http_client.get(&url, Some(headers)).await?;

        parser::parse_balance(&data)
    }

    /// Fetch user's trade history.
    ///
    /// # Arguments
    ///
    /// * `symbol` - Trading pair symbol.
    /// * `since` - Optional start timestamp.
    /// * `limit` - Optional limit on number of trades.
    ///
    /// # Returns
    ///
    /// Returns a vector of [`Trade`] structures for the user's trades.
    ///
    /// # Errors
    ///
    /// Returns an error if authentication fails, market is not found, or the API request fails.
    pub async fn fetch_my_trades(
        &self,
        symbol: &str,
        since: Option<u64>,
        limit: Option<u32>,
    ) -> Result<Vec<Trade>> {
        self.check_required_credentials()?;

        let market = self.base().market(symbol).await?;
        let mut params = HashMap::new();
        params.insert("symbol".to_string(), market.id.clone());

        if let Some(s) = since {
            params.insert("startTime".to_string(), s.to_string());
        }

        if let Some(l) = limit {
            params.insert("limit".to_string(), l.to_string());
        }

        let timestamp = self.fetch_time_raw().await?;
        let auth = self.get_auth()?;
        let signed_params =
            auth.sign_with_timestamp(&params, timestamp, Some(self.options().recv_window))?;

        let mut url = format!("{}/myTrades?", self.urls().private);
        for (key, value) in &signed_params {
            url.push_str(&format!("{}={}&", key, value));
        }

        let mut headers = HeaderMap::new();
        auth.add_auth_headers_reqwest(&mut headers);

        let data = self.base().http_client.get(&url, Some(headers)).await?;

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

        Ok(trades)
    }

    /// Fetch all currency information.
    ///
    /// # Returns
    ///
    /// Returns a vector of [`Currency`] structures.
    ///
    /// # Errors
    ///
    /// Returns an error if authentication fails or the API request fails.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use ccxt_exchanges::binance::Binance;
    /// # use ccxt_core::ExchangeConfig;
    /// # async fn example() -> ccxt_core::Result<()> {
    /// # let binance = Binance::new(ExchangeConfig::default())?;
    /// let currencies = binance.fetch_currencies().await?;
    /// for currency in &currencies {
    ///     println!("{}: {:?} - {}", currency.code, currency.name, currency.active);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn fetch_currencies(&self) -> Result<Vec<Currency>> {
        let url = format!("{}/capital/config/getall", self.urls().sapi);

        // Private API requires signature
        self.check_required_credentials()?;

        let params = HashMap::new();
        let timestamp = self.fetch_time_raw().await?;
        let auth = self.get_auth()?;
        let signed_params =
            auth.sign_with_timestamp(&params, timestamp, Some(self.options().recv_window))?;

        let mut request_url = format!("{}?", url);
        for (key, value) in &signed_params {
            request_url.push_str(&format!("{}={}&", key, value));
        }

        let mut headers = HeaderMap::new();
        auth.add_auth_headers_reqwest(&mut headers);

        let data = self
            .base()
            .http_client
            .get(&request_url, Some(headers))
            .await?;

        parser::parse_currencies(&data)
    }

    /// Fetch closed (completed) orders.
    ///
    /// # Arguments
    ///
    /// * `symbol` - Optional trading pair symbol.
    /// * `since` - Optional start timestamp (milliseconds).
    /// * `limit` - Optional limit on number of orders (default 500, max 1000).
    ///
    /// # Returns
    ///
    /// Returns a vector of closed [`Order`] structures.
    ///
    /// # Errors
    ///
    /// Returns an error if authentication fails or the API request fails.
    ///
    /// # Note
    ///
    /// This method calls `fetch_orders` to get all orders, then filters for "closed" status.
    /// This matches the Go version's implementation logic.
    pub async fn fetch_closed_orders(
        &self,
        symbol: Option<&str>,
        since: Option<u64>,
        limit: Option<u32>,
    ) -> Result<Vec<Order>> {
        let all_orders = self.fetch_orders(symbol, since, None).await?;

        let mut closed_orders: Vec<Order> = all_orders
            .into_iter()
            .filter(|order| order.status == OrderStatus::Closed)
            .collect();

        if let Some(l) = limit {
            closed_orders.truncate(l as usize);
        }

        Ok(closed_orders)
    }

    /// Cancel all open orders.
    ///
    /// # Arguments
    ///
    /// * `symbol` - Trading pair symbol.
    ///
    /// # Returns
    ///
    /// Returns a vector of cancelled [`Order`] structures.
    ///
    /// # Errors
    ///
    /// Returns an error if authentication fails, market is not found, or the API request fails.
    pub async fn cancel_all_orders(&self, symbol: &str) -> Result<Vec<Order>> {
        self.check_required_credentials()?;

        let market = self.base().market(symbol).await?;
        let mut params = HashMap::new();
        params.insert("symbol".to_string(), market.id.clone());

        let timestamp = self.fetch_time_raw().await?;
        let auth = self.get_auth()?;
        let signed_params =
            auth.sign_with_timestamp(&params, timestamp, Some(self.options().recv_window))?;

        let url = format!("{}/openOrders", self.urls().private);
        let mut headers = HeaderMap::new();
        auth.add_auth_headers_reqwest(&mut headers);

        let body = serde_json::to_value(&signed_params).map_err(|e| {
            Error::from(ParseError::invalid_format(
                "data",
                format!("Failed to serialize params: {}", e),
            ))
        })?;

        let data = self
            .base()
            .http_client
            .delete(&url, Some(headers), Some(body))
            .await?;

        let orders_array = data.as_array().ok_or_else(|| {
            Error::from(ParseError::invalid_format(
                "data",
                "Expected array of orders",
            ))
        })?;

        let mut orders = Vec::new();
        for order_data in orders_array {
            match parser::parse_order(order_data, Some(&market)) {
                Ok(order) => orders.push(order),
                Err(e) => {
                    warn!(error = %e, "Failed to parse order");
                }
            }
        }

        Ok(orders)
    }

    /// Cancel multiple orders.
    ///
    /// # Arguments
    ///
    /// * `ids` - Vector of order IDs to cancel.
    /// * `symbol` - Trading pair symbol.
    ///
    /// # Returns
    ///
    /// Returns a vector of cancelled [`Order`] structures.
    ///
    /// # Errors
    ///
    /// Returns an error if authentication fails, market is not found, or the API request fails.
    pub async fn cancel_orders(&self, ids: Vec<String>, symbol: &str) -> Result<Vec<Order>> {
        self.check_required_credentials()?;

        let market = self.base().market(symbol).await?;

        // Binance supports batch cancellation using orderIdList parameter
        let mut params = HashMap::new();
        params.insert("symbol".to_string(), market.id.clone());

        let order_ids_json = serde_json::to_string(&ids).map_err(|e| {
            Error::from(ParseError::invalid_format(
                "data",
                format!("Failed to serialize order IDs: {}", e),
            ))
        })?;
        params.insert("orderIdList".to_string(), order_ids_json);

        let timestamp = self.fetch_time_raw().await?;
        let auth = self.get_auth()?;
        let signed_params =
            auth.sign_with_timestamp(&params, timestamp, Some(self.options().recv_window))?;

        let url = format!("{}/openOrders", self.urls().private);
        let mut headers = HeaderMap::new();
        auth.add_auth_headers_reqwest(&mut headers);

        let body = serde_json::to_value(&signed_params).map_err(|e| {
            Error::from(ParseError::invalid_format(
                "data",
                format!("Failed to serialize params: {}", e),
            ))
        })?;

        let data = self
            .base()
            .http_client
            .delete(&url, Some(headers), Some(body))
            .await?;

        let orders_array = data.as_array().ok_or_else(|| {
            Error::from(ParseError::invalid_format(
                "data",
                "Expected array of orders",
            ))
        })?;

        let mut orders = Vec::new();
        for order_data in orders_array {
            match parser::parse_order(order_data, Some(&market)) {
                Ok(order) => orders.push(order),
                Err(e) => {
                    warn!(error = %e, "Failed to parse order");
                }
            }
        }

        Ok(orders)
    }

    /// Fetch all orders (historical and current).
    ///
    /// # Arguments
    ///
    /// * `symbol` - Optional trading pair symbol.
    /// * `since` - Optional start timestamp (milliseconds).
    /// * `limit` - Optional limit on number of orders (default 500, max 1000).
    ///
    /// # Returns
    ///
    /// Returns a vector of all [`Order`] structures.
    ///
    /// # Errors
    ///
    /// Returns an error if authentication fails or the API request fails.
    pub async fn fetch_orders(
        &self,
        symbol: Option<&str>,
        since: Option<u64>,
        limit: Option<u32>,
    ) -> Result<Vec<Order>> {
        self.check_required_credentials()?;

        let mut params = HashMap::new();
        let market = if let Some(sym) = symbol {
            let m = self.base().market(sym).await?;
            params.insert("symbol".to_string(), m.id.clone());
            Some(m)
        } else {
            None
        };

        if let Some(s) = since {
            params.insert("startTime".to_string(), s.to_string());
        }

        if let Some(l) = limit {
            params.insert("limit".to_string(), l.to_string());
        }

        let timestamp = self.fetch_time_raw().await?;
        let auth = self.get_auth()?;
        let signed_params =
            auth.sign_with_timestamp(&params, timestamp, Some(self.options().recv_window))?;

        let mut url = format!("{}/allOrders?", self.urls().private);
        for (key, value) in &signed_params {
            url.push_str(&format!("{}={}&", key, value));
        }

        let mut headers = HeaderMap::new();
        auth.add_auth_headers_reqwest(&mut headers);

        let data = self.base().http_client.get(&url, Some(headers)).await?;

        let orders_array = data.as_array().ok_or_else(|| {
            Error::from(ParseError::invalid_format(
                "data",
                "Expected array of orders",
            ))
        })?;

        let mut orders = Vec::new();
        for order_data in orders_array {
            match parser::parse_order(order_data, market.as_ref().map(|v| &**v)) {
                Ok(order) => orders.push(order),
                Err(e) => {
                    warn!(error = %e, "Failed to parse order");
                }
            }
        }

        Ok(orders)
    }

    /// Check required authentication credentials.
    fn check_required_credentials(&self) -> Result<()> {
        if self.base().config.api_key.is_none() || self.base().config.secret.is_none() {
            return Err(Error::authentication("API key and secret are required"));
        }
        Ok(())
    }

    /// Get authenticator instance.
    fn get_auth(&self) -> Result<BinanceAuth> {
        let api_key = self
            .base()
            .config
            .api_key
            .as_ref()
            .ok_or_else(|| Error::authentication("Missing API key"))?;
        let secret = self
            .base()
            .config
            .secret
            .as_ref()
            .ok_or_else(|| Error::authentication("Missing secret"))?;

        Ok(BinanceAuth::new(api_key, secret))
    }
    /// Fetch funding rate for a single trading pair.
    ///
    /// # Arguments
    ///
    /// * `symbol` - Trading pair symbol (e.g., "BTC/USDT:USDT").
    /// * `params` - Optional parameters.
    ///
    /// # Returns
    ///
    /// Returns the [`FundingRate`] information.
    ///
    /// # Errors
    ///
    /// Returns an error if the API request fails.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use ccxt_exchanges::binance::Binance;
    /// # use ccxt_core::ExchangeConfig;
    /// # async fn example() -> ccxt_core::Result<()> {
    /// let binance = Binance::new_swap(ExchangeConfig::default())?;
    /// let funding_rate = binance.fetch_funding_rate("BTC/USDT:USDT", None).await?;
    /// # Ok(())
    /// # }
    /// ```
    /// Fetch funding rates for multiple trading pairs.
    ///
    /// # Arguments
    ///
    /// * `symbols` - Optional vector of trading pair symbols, fetches all if `None`.
    /// * `params` - Optional parameters.
    ///
    /// # Returns
    ///
    /// Returns a vector of [`FundingRate`] structures.
    ///
    /// # Errors
    ///
    /// Returns an error if the API request fails.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use ccxt_exchanges::binance::Binance;
    /// # use ccxt_core::ExchangeConfig;
    /// # async fn example() -> ccxt_core::Result<()> {
    /// let binance = Binance::new_swap(ExchangeConfig::default())?;
    /// let funding_rates = binance.fetch_funding_rates(None, None).await?;
    /// # Ok(())
    /// # }
    /// ```
    /// Fetch funding rate history.
    ///
    /// # Arguments
    ///
    /// * `symbol` - Optional trading pair symbol.
    /// * `since` - Optional start timestamp (milliseconds).
    /// * `limit` - Optional limit on number of records (default 100, max 1000).
    /// * `params` - Optional parameters.
    ///
    /// # Returns
    ///
    /// Returns a vector of historical [`FundingRate`] structures.
    ///
    /// # Errors
    ///
    /// Returns an error if the API request fails.
    ///
    /// Fetch position for a single trading pair.
    ///
    /// # Arguments
    ///
    /// * `symbol` - Trading pair symbol (e.g., "BTC/USDT:USDT").
    /// * `params` - Optional parameters.
    ///
    /// # Returns
    ///
    /// Returns the [`Position`] information.
    ///
    /// # Errors
    ///
    /// Returns an error if authentication fails, market is not found, or the API request fails.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use ccxt_exchanges::binance::Binance;
    /// # use ccxt_core::ExchangeConfig;
    /// # async fn example() -> ccxt_core::Result<()> {
    /// let mut config = ExchangeConfig::default();
    /// config.api_key = Some("your_api_key".to_string());
    /// config.secret = Some("your_secret".to_string());
    /// let binance = Binance::new(config)?;
    /// let position = binance.fetch_position("BTC/USDT:USDT", None).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn fetch_position(
        &self,
        symbol: &str,
        params: Option<Value>,
    ) -> Result<ccxt_core::types::Position> {
        self.check_required_credentials()?;

        let market = self.base().market(symbol).await?;
        let mut request_params = HashMap::new();
        request_params.insert("symbol".to_string(), market.id.clone());

        if let Some(params) = params {
            if let Some(obj) = params.as_object() {
                for (key, value) in obj {
                    if let Some(v) = value.as_str() {
                        request_params.insert(key.clone(), v.to_string());
                    }
                }
            }
        }

        let timestamp = self.fetch_time_raw().await?;
        let auth = self.get_auth()?;
        let signed_params =
            auth.sign_with_timestamp(&request_params, timestamp, Some(self.options().recv_window))?;

        // Determine API endpoint based on market type
        let url = if market.linear.unwrap_or(true) {
            format!("{}/positionRisk", self.urls().fapi_private)
        } else {
            format!("{}/positionRisk", self.urls().dapi_private)
        };

        let mut request_url = format!("{}?", url);
        for (key, value) in &signed_params {
            request_url.push_str(&format!("{}={}&", key, value));
        }

        let mut headers = HeaderMap::new();
        auth.add_auth_headers_reqwest(&mut headers);

        let data = self
            .base()
            .http_client
            .get(&request_url, Some(headers))
            .await?;

        // API returns array, find matching symbol
        let positions_array = data.as_array().ok_or_else(|| {
            Error::from(ParseError::invalid_format(
                "data",
                "Expected array of positions",
            ))
        })?;

        for position_data in positions_array {
            if let Some(pos_symbol) = position_data["symbol"].as_str() {
                if pos_symbol == market.id {
                    return parser::parse_position(position_data, Some(&market));
                }
            }
        }

        Err(Error::from(ParseError::missing_field_owned(format!(
            "Position not found for symbol: {}",
            symbol
        ))))
    }

    /// Fetch all positions.
    ///
    /// # Arguments
    ///
    /// * `symbols` - Optional vector of trading pair symbols.
    /// * `params` - Optional parameters.
    ///
    /// # Returns
    ///
    /// Returns a vector of [`Position`] structures.
    ///
    /// # Errors
    ///
    /// Returns an error if authentication fails or the API request fails.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use ccxt_exchanges::binance::Binance;
    /// # use ccxt_core::ExchangeConfig;
    /// # async fn example() -> ccxt_core::Result<()> {
    /// let mut config = ExchangeConfig::default();
    /// config.api_key = Some("your_api_key".to_string());
    /// config.secret = Some("your_secret".to_string());
    /// let binance = Binance::new_swap(config)?;
    /// let positions = binance.fetch_positions(None, None).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn fetch_positions(
        &self,
        symbols: Option<Vec<String>>,
        params: Option<Value>,
    ) -> Result<Vec<ccxt_core::types::Position>> {
        self.check_required_credentials()?;

        let mut request_params = HashMap::new();

        if let Some(ref params) = params {
            if let Some(obj) = params.as_object() {
                for (key, value) in obj {
                    if let Some(v) = value.as_str() {
                        request_params.insert(key.clone(), v.to_string());
                    }
                }
            }
        }

        let timestamp = self.fetch_time_raw().await?;
        let auth = self.get_auth()?;
        let signed_params =
            auth.sign_with_timestamp(&request_params, timestamp, Some(self.options().recv_window))?;
        let query_string = auth.build_query_string(&signed_params);
        // Default to USDT-M futures endpoint
        let use_coin_m = params
            .as_ref()
            .and_then(|p| p.get("type"))
            .and_then(|v| v.as_str())
            .map(|t| t == "delivery" || t == "coin_m")
            .unwrap_or(false);

        let url = if use_coin_m {
            format!("{}/positionRisk", self.urls().dapi_private)
        } else {
            format!("{}/positionRisk", self.urls().fapi_private)
        };

        let request_url = format!("{}?{}", url, query_string);

        let mut headers = HeaderMap::new();
        auth.add_auth_headers_reqwest(&mut headers);

        let data = self
            .base()
            .http_client
            .get(&request_url, Some(headers))
            .await?;

        let positions_array = data.as_array().ok_or_else(|| {
            Error::from(ParseError::invalid_format(
                "data",
                "Expected array of positions",
            ))
        })?;

        // Clone markets_by_id map once before the loop to avoid lock contention
        let markets_by_id = {
            let cache = self.base().market_cache.read().await;
            cache.markets_by_id.clone()
        };

        let mut positions = Vec::new();
        for position_data in positions_array {
            if let Some(binance_symbol) = position_data["symbol"].as_str() {
                if let Some(market) = markets_by_id.get(binance_symbol) {
                    match parser::parse_position(position_data, Some(market)) {
                        Ok(position) => {
                            // Only return positions with contracts > 0
                            if position.contracts.unwrap_or(0.0) > 0.0 {
                                // If symbols specified, only return matching ones
                                if let Some(ref syms) = symbols {
                                    if syms.contains(&position.symbol) {
                                        positions.push(position);
                                    }
                                } else {
                                    positions.push(position);
                                }
                            }
                        }
                        Err(e) => {
                            warn!(
                                error = %e,
                                symbol = %binance_symbol,
                                "Failed to parse position"
                            );
                        }
                    }
                }
            }
        }

        Ok(positions)
    }

    /// Fetch position risk information.
    ///
    /// This is an alias for [`fetch_positions`](Self::fetch_positions) provided for CCXT naming consistency.
    ///
    /// # Arguments
    ///
    /// * `symbols` - Optional list of trading pair symbols.
    /// * `params` - Optional additional parameters.
    ///
    /// # Returns
    ///
    /// Returns a vector of position risk information as [`Position`] structures.
    ///
    /// # Errors
    ///
    /// Returns an error if authentication fails or the API request fails.
    pub async fn fetch_positions_risk(
        &self,
        symbols: Option<Vec<String>>,
        params: Option<Value>,
    ) -> Result<Vec<ccxt_core::types::Position>> {
        self.fetch_positions(symbols, params).await
    }

    /// Fetch leverage settings for multiple trading pairs.
    ///
    /// # Arguments
    ///
    /// * `symbols` - Optional list of trading pairs. `None` queries all pairs.
    /// * `params` - Optional parameters:
    ///   - `portfolioMargin`: Whether to use portfolio margin account.
    ///
    /// # Returns
    ///
    /// Returns a `HashMap` of leverage information keyed by trading pair symbol.
    ///
    /// # Errors
    ///
    /// Returns an error if authentication fails or the API request fails.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use ccxt_exchanges::binance::Binance;
    /// # use ccxt_core::ExchangeConfig;
    /// # async fn example() -> ccxt_core::Result<()> {
    /// # let exchange = Binance::new(ExchangeConfig::default())?;
    /// // Query leverage settings for all trading pairs
    /// let leverages = exchange.fetch_leverages(None, None).await?;
    ///
    /// // Query leverage settings for specific pairs
    /// let symbols = vec!["BTC/USDT:USDT".to_string(), "ETH/USDT:USDT".to_string()];
    /// let leverages = exchange.fetch_leverages(Some(symbols), None).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn fetch_leverages(
        &self,
        symbols: Option<Vec<String>>,
        params: Option<Value>,
    ) -> Result<HashMap<String, ccxt_core::types::Leverage>> {
        self.load_markets(false).await?;

        let mut params_map = if let Some(p) = params {
            serde_json::from_value::<HashMap<String, String>>(p).unwrap_or_default()
        } else {
            HashMap::new()
        };

        let market_type = params_map
            .remove("type")
            .or_else(|| params_map.remove("marketType"))
            .unwrap_or_else(|| "future".to_string());

        let sub_type = params_map
            .remove("subType")
            .unwrap_or_else(|| "linear".to_string());

        let is_portfolio_margin = params_map
            .remove("portfolioMargin")
            .and_then(|v| v.parse::<bool>().ok())
            .unwrap_or(false);

        let timestamp = self.fetch_time_raw().await?;
        let auth = self.get_auth()?;
        let signed_params =
            auth.sign_with_timestamp(&params_map, timestamp, Some(self.options().recv_window))?;

        let url = if market_type == "future" && sub_type == "linear" {
            // USDT-M futures
            if is_portfolio_margin {
                format!(
                    "{}/account",
                    self.urls().fapi_private.replace("/fapi/v1", "/papi/v1/um")
                )
            } else {
                format!("{}/symbolConfig", self.urls().fapi_private)
            }
        } else if market_type == "future" && sub_type == "inverse" {
            // Coin-M futures
            if is_portfolio_margin {
                format!(
                    "{}/account",
                    self.urls().dapi_private.replace("/dapi/v1", "/papi/v1/cm")
                )
            } else {
                format!("{}/account", self.urls().dapi_private)
            }
        } else {
            return Err(Error::invalid_request(
                "fetchLeverages() supports linear and inverse contracts only",
            ));
        };

        let mut request_url = format!("{}?", url);
        for (key, value) in &signed_params {
            request_url.push_str(&format!("{}={}&", key, value));
        }

        let mut headers = HeaderMap::new();
        auth.add_auth_headers_reqwest(&mut headers);

        let response = self
            .base()
            .http_client
            .get(&request_url, Some(headers))
            .await?;

        let leverages_data = if let Some(positions) = response.get("positions") {
            positions.as_array().cloned().unwrap_or_default()
        } else if response.is_array() {
            response.as_array().cloned().unwrap_or_default()
        } else {
            vec![]
        };

        let mut leverages = HashMap::new();

        for item in leverages_data {
            if let Ok(leverage) = parser::parse_leverage(&item, None) {
                // If symbols specified, only keep matching ones
                if let Some(ref filter_symbols) = symbols {
                    if filter_symbols.contains(&leverage.symbol) {
                        leverages.insert(leverage.symbol.clone(), leverage);
                    }
                } else {
                    leverages.insert(leverage.symbol.clone(), leverage);
                }
            }
        }

        Ok(leverages)
    }

    /// Fetch leverage settings for a single trading pair.
    ///
    /// # Arguments
    ///
    /// * `symbol` - Trading pair symbol.
    /// * `params` - Optional additional parameters.
    ///
    /// # Returns
    ///
    /// Returns leverage information for the specified trading pair.
    ///
    /// # Errors
    ///
    /// Returns an error if the symbol is not found or the API request fails.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use ccxt_exchanges::binance::Binance;
    /// # use ccxt_core::ExchangeConfig;
    /// # async fn example() -> ccxt_core::Result<()> {
    /// # let exchange = Binance::new(ExchangeConfig::default())?;
    /// // Query leverage settings for BTC/USDT futures
    /// let leverage = exchange.fetch_leverage("BTC/USDT:USDT", None).await?;
    /// println!("Long leverage: {:?}", leverage.long_leverage);
    /// println!("Short leverage: {:?}", leverage.short_leverage);
    /// println!("Margin mode: {:?}", leverage.margin_mode);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn fetch_leverage(
        &self,
        symbol: &str,
        params: Option<Value>,
    ) -> Result<ccxt_core::types::Leverage> {
        let symbols = Some(vec![symbol.to_string()]);
        let leverages = self.fetch_leverages(symbols, params).await?;

        leverages.get(symbol).cloned().ok_or_else(|| {
            Error::exchange("404", format!("Leverage not found for symbol: {}", symbol))
        })
    }
    /// Set leverage multiplier for a trading pair.
    ///
    /// # Arguments
    ///
    /// * `symbol` - Trading pair symbol.
    /// * `leverage` - Leverage multiplier (1-125).
    /// * `params` - Optional additional parameters.
    ///
    /// # Returns
    ///
    /// Returns the operation result as a `HashMap`.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Authentication credentials are missing
    /// - Leverage is outside valid range (1-125)
    /// - The market is not a futures/swap market
    /// - The API request fails
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use ccxt_exchanges::binance::Binance;
    /// # use ccxt_core::ExchangeConfig;
    /// # async fn example() -> ccxt_core::Result<()> {
    /// let mut config = ExchangeConfig::default();
    /// config.api_key = Some("your_api_key".to_string());
    /// config.secret = Some("your_secret".to_string());
    /// let binance = Binance::new_swap(config)?;
    /// let result = binance.set_leverage("BTC/USDT:USDT", 10, None).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn set_leverage(
        &self,
        symbol: &str,
        leverage: i64,
        params: Option<HashMap<String, String>>,
    ) -> Result<HashMap<String, Value>> {
        self.check_required_credentials()?;

        if leverage < 1 || leverage > 125 {
            return Err(Error::invalid_request(
                "Leverage must be between 1 and 125".to_string(),
            ));
        }

        self.load_markets(false).await?;
        let market = self.base().market(symbol).await?;

        if market.market_type != MarketType::Futures && market.market_type != MarketType::Swap {
            return Err(Error::invalid_request(
                "set_leverage() supports futures and swap markets only".to_string(),
            ));
        }

        let mut request_params = HashMap::new();
        request_params.insert("symbol".to_string(), market.id.clone());
        request_params.insert("leverage".to_string(), leverage.to_string());

        if let Some(p) = params {
            for (key, value) in p {
                request_params.insert(key, value);
            }
        }

        // Select API endpoint based on market type
        let url = if market.linear.unwrap_or(true) {
            format!("{}/leverage", self.urls().fapi_private)
        } else if market.inverse.unwrap_or(false) {
            format!("{}/leverage", self.urls().dapi_private)
        } else {
            return Err(Error::invalid_request(
                "Unknown futures market type".to_string(),
            ));
        };

        let timestamp = self.fetch_time_raw().await?;
        let auth = self.get_auth()?;

        let signed_params =
            auth.sign_with_timestamp(&request_params, timestamp, Some(self.options().recv_window))?;

        let mut headers = HeaderMap::new();
        auth.add_auth_headers_reqwest(&mut headers);

        let body = serde_json::to_value(&signed_params).map_err(|e| {
            Error::from(ParseError::invalid_format(
                "data",
                format!("Failed to serialize params: {}", e),
            ))
        })?;

        let response = self
            .base()
            .http_client
            .post(&url, Some(headers), Some(body))
            .await?;

        let result: HashMap<String, Value> = serde_json::from_value(response).map_err(|e| {
            Error::from(ParseError::invalid_format(
                "data",
                format!("Failed to parse response: {}", e),
            ))
        })?;

        Ok(result)
    }

    /// Set margin mode for a trading pair.
    ///
    /// # Arguments
    ///
    /// * `symbol` - Trading pair symbol.
    /// * `margin_mode` - Margin mode (`isolated` or `cross`).
    /// * `params` - Optional additional parameters.
    ///
    /// # Returns
    ///
    /// Returns the operation result as a `HashMap`.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Authentication credentials are missing
    /// - Margin mode is invalid (must be `isolated` or `cross`)
    /// - The market is not a futures/swap market
    /// - The API request fails
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use ccxt_exchanges::binance::Binance;
    /// # use ccxt_core::ExchangeConfig;
    /// # async fn example() -> ccxt_core::Result<()> {
    /// let mut config = ExchangeConfig::default();
    /// config.api_key = Some("your_api_key".to_string());
    /// config.secret = Some("your_secret".to_string());
    /// let binance = Binance::new_swap(config)?;
    /// let result = binance.set_margin_mode("BTC/USDT:USDT", "isolated", None).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn set_margin_mode(
        &self,
        symbol: &str,
        margin_mode: &str,
        params: Option<HashMap<String, String>>,
    ) -> Result<HashMap<String, Value>> {
        self.check_required_credentials()?;

        let margin_type = match margin_mode.to_uppercase().as_str() {
            "ISOLATED" | "ISOLATED_MARGIN" => "ISOLATED",
            "CROSS" | "CROSSED" | "CROSS_MARGIN" => "CROSSED",
            _ => {
                return Err(Error::invalid_request(format!(
                    "Invalid margin mode: {}. Must be 'isolated' or 'cross'",
                    margin_mode
                )));
            }
        };

        self.load_markets(false).await?;
        let market = self.base().market(symbol).await?;

        if market.market_type != MarketType::Futures && market.market_type != MarketType::Swap {
            return Err(Error::invalid_request(
                "set_margin_mode() supports futures and swap markets only".to_string(),
            ));
        }

        let mut request_params = HashMap::new();
        request_params.insert("symbol".to_string(), market.id.clone());
        request_params.insert("marginType".to_string(), margin_type.to_string());

        if let Some(p) = params {
            for (key, value) in p {
                request_params.insert(key, value);
            }
        }

        // Select API endpoint based on market type
        let url = if market.linear.unwrap_or(true) {
            format!("{}/marginType", self.urls().fapi_private)
        } else if market.inverse.unwrap_or(false) {
            format!("{}/marginType", self.urls().dapi_private)
        } else {
            return Err(Error::invalid_request(
                "Unknown futures market type".to_string(),
            ));
        };

        let timestamp = self.fetch_time_raw().await?;
        let auth = self.get_auth()?;
        let signed_params =
            auth.sign_with_timestamp(&request_params, timestamp, Some(self.options().recv_window))?;

        let mut headers = HeaderMap::new();
        auth.add_auth_headers_reqwest(&mut headers);

        let body = serde_json::to_value(&signed_params).map_err(|e| {
            Error::from(ParseError::invalid_format(
                "data",
                format!("Failed to serialize params: {}", e),
            ))
        })?;

        let response = self
            .base()
            .http_client
            .post(&url, Some(headers), Some(body))
            .await?;

        let result: HashMap<String, Value> = serde_json::from_value(response).map_err(|e| {
            Error::from(ParseError::invalid_format(
                "data",
                format!("Failed to parse response: {}", e),
            ))
        })?;

        Ok(result)
    }

    // ==================== P2.2: Fee Management ====================

    /// Fetch trading fee for a single trading pair.
    ///
    /// # Arguments
    ///
    /// * `symbol` - Trading pair symbol (e.g., "BTC/USDT").
    /// * `params` - Optional parameters.
    ///
    /// # Returns
    ///
    /// Returns trading fee information.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Authentication credentials are missing
    /// - The market type is not supported
    /// - The API request fails
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use ccxt_exchanges::binance::Binance;
    /// # use ccxt_core::ExchangeConfig;
    /// # async fn example() -> ccxt_core::Result<()> {
    /// let mut config = ExchangeConfig::default();
    /// config.api_key = Some("your_api_key".to_string());
    /// config.secret = Some("your_secret".to_string());
    /// let binance = Binance::new(config)?;
    /// let fee = binance.fetch_trading_fee("BTC/USDT", None).await?;
    /// println!("Maker: {}, Taker: {}", fee.maker, fee.taker);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn fetch_trading_fee(
        &self,
        symbol: &str,
        params: Option<HashMap<String, String>>,
    ) -> Result<FeeTradingFee> {
        self.check_required_credentials()?;

        self.load_markets(false).await?;
        let market = self.base().market(symbol).await?;

        let mut request_params = HashMap::new();
        request_params.insert("symbol".to_string(), market.id.clone());

        let is_portfolio_margin = params
            .as_ref()
            .and_then(|p| p.get("portfolioMargin"))
            .map(|v| v == "true")
            .unwrap_or(false);

        if let Some(p) = params {
            for (key, value) in p {
                // Do not pass portfolioMargin parameter to API
                if key != "portfolioMargin" {
                    request_params.insert(key, value);
                }
            }
        }

        // Select API endpoint based on market type and Portfolio Margin mode
        let url = match market.market_type {
            MarketType::Spot => format!("{}/asset/tradeFee", self.urls().sapi),
            MarketType::Futures | MarketType::Swap => {
                if is_portfolio_margin {
                    // Portfolio Margin mode uses papi endpoints
                    if market.is_linear() {
                        format!("{}/um/commissionRate", self.urls().papi)
                    } else {
                        format!("{}/cm/commissionRate", self.urls().papi)
                    }
                } else {
                    // Standard mode
                    if market.is_linear() {
                        format!("{}/commissionRate", self.urls().fapi_private)
                    } else {
                        format!("{}/commissionRate", self.urls().dapi_private)
                    }
                }
            }
            _ => {
                return Err(Error::invalid_request(format!(
                    "fetch_trading_fee not supported for market type: {:?}",
                    market.market_type
                )));
            }
        };

        let timestamp = self.fetch_time_raw().await?;
        let auth = self.get_auth()?;
        let signed_params =
            auth.sign_with_timestamp(&request_params, timestamp, Some(self.options().recv_window))?;

        let mut request_url = format!("{}?", url);
        for (key, value) in &signed_params {
            request_url.push_str(&format!("{}={}&", key, value));
        }

        let mut headers = HeaderMap::new();
        auth.add_auth_headers_reqwest(&mut headers);

        let response = self
            .base()
            .http_client
            .get(&request_url, Some(headers))
            .await?;

        parser::parse_trading_fee(&response)
    }

    /// Fetch trading fees for multiple trading pairs.
    ///
    /// # Arguments
    ///
    /// * `symbols` - Optional list of trading pairs. `None` fetches all pairs.
    /// * `params` - Optional parameters.
    ///
    /// # Returns
    ///
    /// Returns a `HashMap` of trading fees keyed by symbol.
    ///
    /// # Errors
    ///
    /// Returns an error if authentication fails or the API request fails.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use ccxt_exchanges::binance::Binance;
    /// # use ccxt_core::ExchangeConfig;
    /// # async fn example() -> ccxt_core::Result<()> {
    /// let mut config = ExchangeConfig::default();
    /// config.api_key = Some("your_api_key".to_string());
    /// config.secret = Some("your_secret".to_string());
    /// let binance = Binance::new(config)?;
    /// let fees = binance.fetch_trading_fees(None, None).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn fetch_trading_fees(
        &self,
        symbols: Option<Vec<String>>,
        params: Option<HashMap<String, String>>,
    ) -> Result<HashMap<String, FeeTradingFee>> {
        self.check_required_credentials()?;

        self.load_markets(false).await?;

        let mut request_params = HashMap::new();

        if let Some(syms) = &symbols {
            let mut market_ids: Vec<String> = Vec::new();
            for s in syms {
                if let Ok(market) = self.base().market(s).await {
                    market_ids.push(market.id.clone());
                }
            }
            if !market_ids.is_empty() {
                request_params.insert("symbols".to_string(), market_ids.join(","));
            }
        }

        if let Some(p) = params {
            for (key, value) in p {
                request_params.insert(key, value);
            }
        }

        let url = format!("{}/asset/tradeFee", self.urls().sapi);

        let timestamp = self.fetch_time_raw().await?;
        let auth = self.get_auth()?;
        let signed_params =
            auth.sign_with_timestamp(&request_params, timestamp, Some(self.options().recv_window))?;

        let mut request_url = format!("{}?", url);
        for (key, value) in &signed_params {
            request_url.push_str(&format!("{}={}&", key, value));
        }

        let mut headers = HeaderMap::new();
        auth.add_auth_headers_reqwest(&mut headers);

        let response = self
            .base()
            .http_client
            .get(&request_url, Some(headers))
            .await?;

        let fees_array = response.as_array().ok_or_else(|| {
            Error::from(ParseError::invalid_format(
                "data",
                "Expected array of trading fees",
            ))
        })?;

        let mut fees = HashMap::new();
        for fee_data in fees_array {
            if let Ok(symbol_id) = fee_data["symbol"]
                .as_str()
                .ok_or_else(|| Error::from(ParseError::missing_field("symbol")))
            {
                if let Ok(market) = self.base().market_by_id(symbol_id).await {
                    if let Ok(fee) = parser::parse_trading_fee(fee_data) {
                        fees.insert(market.symbol.clone(), fee);
                    }
                }
            }
        }

        Ok(fees)
    }

    /// Fetch current funding rate for a single trading pair.
    ///
    /// # Arguments
    ///
    /// * `symbol` - Trading pair symbol (e.g., "BTC/USDT:USDT").
    /// * `params` - Optional parameters.
    ///
    /// # Returns
    ///
    /// Returns current funding rate information.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The market is not a futures or swap market
    /// - The API request fails
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use ccxt_exchanges::binance::Binance;
    /// # use ccxt_core::ExchangeConfig;
    /// # async fn example() -> ccxt_core::Result<()> {
    /// let binance = Binance::new_swap(ExchangeConfig::default())?;
    /// let rate = binance.fetch_funding_rate("BTC/USDT:USDT", None).await?;
    /// println!("Funding rate: {:?}", rate.funding_rate);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn fetch_funding_rate(
        &self,
        symbol: &str,
        params: Option<HashMap<String, String>>,
    ) -> Result<FeeFundingRate> {
        self.load_markets(false).await?;
        let market = self.base().market(symbol).await?;

        if market.market_type != MarketType::Futures && market.market_type != MarketType::Swap {
            return Err(Error::invalid_request(
                "fetch_funding_rate() supports futures and swap markets only".to_string(),
            ));
        }

        let mut request_params = HashMap::new();
        request_params.insert("symbol".to_string(), market.id.clone());

        if let Some(p) = params {
            for (key, value) in p {
                request_params.insert(key, value);
            }
        }

        let url = if market.linear.unwrap_or(true) {
            format!("{}/premiumIndex", self.urls().fapi_public)
        } else {
            format!("{}/premiumIndex", self.urls().dapi_public)
        };

        let mut request_url = format!("{}?", url);
        for (key, value) in &request_params {
            request_url.push_str(&format!("{}={}&", key, value));
        }

        let response = self.base().http_client.get(&request_url, None).await?;

        // COIN-M futures API returns array format - extract first element
        let data = if !market.linear.unwrap_or(true) {
            response
                .as_array()
                .and_then(|arr| arr.first())
                .ok_or_else(|| {
                    Error::from(ParseError::invalid_format(
                        "data",
                        "COIN-M funding rate response should be an array with at least one element",
                    ))
                })?
        } else {
            &response
        };

        parser::parse_funding_rate(data, Some(&market))
    }

    /// Fetch funding rate history for a trading pair.
    ///
    /// # Arguments
    ///
    /// * `symbol` - Trading pair symbol (e.g., "BTC/USDT:USDT").
    /// * `since` - Optional start timestamp in milliseconds.
    /// * `limit` - Optional record limit (default 100, max 1000).
    /// * `params` - Optional parameters.
    ///
    /// # Returns
    ///
    /// Returns a vector of historical funding rate records.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The market is not a futures or swap market
    /// - The API request fails
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use ccxt_exchanges::binance::Binance;
    /// # use ccxt_core::ExchangeConfig;
    /// # async fn example() -> ccxt_core::Result<()> {
    /// let binance = Binance::new_swap(ExchangeConfig::default())?;
    /// let history = binance.fetch_funding_rate_history("BTC/USDT:USDT", None, Some(10), None).await?;
    /// println!("Found {} records", history.len());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn fetch_funding_rate_history(
        &self,
        symbol: &str,
        since: Option<u64>,
        limit: Option<u32>,
        params: Option<HashMap<String, String>>,
    ) -> Result<Vec<FeeFundingRateHistory>> {
        self.load_markets(false).await?;
        let market = self.base().market(symbol).await?;

        if market.market_type != MarketType::Futures && market.market_type != MarketType::Swap {
            return Err(Error::invalid_request(
                "fetch_funding_rate_history() supports futures and swap markets only".to_string(),
            ));
        }

        let mut request_params = HashMap::new();
        request_params.insert("symbol".to_string(), market.id.clone());

        if let Some(s) = since {
            request_params.insert("startTime".to_string(), s.to_string());
        }

        if let Some(l) = limit {
            request_params.insert("limit".to_string(), l.to_string());
        }

        if let Some(p) = params {
            for (key, value) in p {
                request_params.insert(key, value);
            }
        }

        let url = if market.linear.unwrap_or(true) {
            format!("{}/fundingRate", self.urls().fapi_public)
        } else {
            format!("{}/fundingRate", self.urls().dapi_public)
        };

        let mut request_url = format!("{}?", url);
        for (key, value) in &request_params {
            request_url.push_str(&format!("{}={}&", key, value));
        }

        let response = self.base().http_client.get(&request_url, None).await?;

        let history_array = response.as_array().ok_or_else(|| {
            Error::from(ParseError::invalid_format(
                "data",
                "Expected array of funding rate history",
            ))
        })?;

        let mut history = Vec::new();
        for history_data in history_array {
            match parser::parse_funding_rate_history(history_data, Some(&market)) {
                Ok(record) => history.push(record),
                Err(e) => {
                    warn!(error = %e, "Failed to parse funding rate history");
                }
            }
        }

        Ok(history)
    }

    /// Fetch current funding rates for multiple trading pairs.
    ///
    /// # Arguments
    ///
    /// * `symbols` - Optional list of trading pairs. `None` fetches all pairs.
    /// * `params` - Optional parameters.
    ///
    /// # Returns
    ///
    /// Returns a `HashMap` of funding rates keyed by symbol.
    ///
    /// # Errors
    ///
    /// Returns an error if the API request fails.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use ccxt_exchanges::binance::Binance;
    /// # use ccxt_core::ExchangeConfig;
    /// # async fn example() -> ccxt_core::Result<()> {
    /// let binance = Binance::new_swap(ExchangeConfig::default())?;
    /// let rates = binance.fetch_funding_rates(None, None).await?;
    /// println!("Found {} funding rates", rates.len());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn fetch_funding_rates(
        &self,
        symbols: Option<Vec<String>>,
        params: Option<HashMap<String, String>>,
    ) -> Result<HashMap<String, FeeFundingRate>> {
        self.load_markets(false).await?;

        let mut request_params = HashMap::new();

        if let Some(p) = params {
            for (key, value) in p {
                request_params.insert(key, value);
            }
        }

        let url = format!("{}/premiumIndex", self.urls().fapi_public);

        let mut request_url = url.clone();
        if !request_params.is_empty() {
            request_url.push('?');
            for (key, value) in &request_params {
                request_url.push_str(&format!("{}={}&", key, value));
            }
        }

        let response = self.base().http_client.get(&request_url, None).await?;

        let mut rates = HashMap::new();

        if let Some(rates_array) = response.as_array() {
            for rate_data in rates_array {
                if let Ok(symbol_id) = rate_data["symbol"]
                    .as_str()
                    .ok_or_else(|| Error::from(ParseError::missing_field("symbol")))
                {
                    if let Ok(market) = self.base().market_by_id(symbol_id).await {
                        if let Some(ref syms) = symbols {
                            if !syms.contains(&market.symbol) {
                                continue;
                            }
                        }

                        if let Ok(rate) = parser::parse_funding_rate(rate_data, Some(&market)) {
                            rates.insert(market.symbol.clone(), rate);
                        }
                    }
                }
            }
        } else {
            if let Ok(symbol_id) = response["symbol"]
                .as_str()
                .ok_or_else(|| Error::from(ParseError::missing_field("symbol")))
            {
                if let Ok(market) = self.base().market_by_id(symbol_id).await {
                    if let Ok(rate) = parser::parse_funding_rate(&response, Some(&market)) {
                        rates.insert(market.symbol.clone(), rate);
                    }
                }
            }
        }

        Ok(rates)
    }

    /// Fetch leverage tier information for trading pairs.
    ///
    /// # Arguments
    ///
    /// * `symbols` - Optional list of trading pairs. `None` fetches all pairs.
    /// * `params` - Optional parameters.
    ///
    /// # Returns
    ///
    /// Returns a `HashMap` of leverage tiers keyed by symbol.
    ///
    /// # Errors
    ///
    /// Returns an error if authentication fails or the API request fails.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use ccxt_exchanges::binance::Binance;
    /// # use ccxt_core::ExchangeConfig;
    /// # async fn example() -> ccxt_core::Result<()> {
    /// let mut config = ExchangeConfig::default();
    /// config.api_key = Some("your_api_key".to_string());
    /// config.secret = Some("your_secret".to_string());
    /// let binance = Binance::new_swap(config)?;
    /// let tiers = binance.fetch_leverage_tiers(None, None).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn fetch_leverage_tiers(
        &self,
        symbols: Option<Vec<String>>,
        params: Option<HashMap<String, String>>,
    ) -> Result<HashMap<String, Vec<LeverageTier>>> {
        self.check_required_credentials()?;

        self.load_markets(false).await?;

        let mut request_params = HashMap::new();

        let is_portfolio_margin = params
            .as_ref()
            .and_then(|p| p.get("portfolioMargin"))
            .map(|v| v == "true")
            .unwrap_or(false);

        let market = if let Some(syms) = &symbols {
            if let Some(first_symbol) = syms.first() {
                let m = self.base().market(first_symbol).await?;
                request_params.insert("symbol".to_string(), m.id.clone());
                Some(m)
            } else {
                None
            }
        } else {
            None
        };

        if let Some(p) = params {
            for (key, value) in p {
                // Do not pass portfolioMargin parameter to API
                if key != "portfolioMargin" {
                    request_params.insert(key, value);
                }
            }
        }

        // Select API endpoint based on market type and Portfolio Margin mode
        let url = if let Some(ref m) = market {
            if is_portfolio_margin {
                // Portfolio Margin mode uses papi endpoints
                if m.is_linear() {
                    format!("{}/um/leverageBracket", self.urls().papi)
                } else {
                    format!("{}/cm/leverageBracket", self.urls().papi)
                }
            } else {
                // Standard mode
                if m.is_linear() {
                    format!("{}/leverageBracket", self.urls().fapi_private)
                } else {
                    format!("{}/v2/leverageBracket", self.urls().dapi_private)
                }
            }
        } else {
            format!("{}/leverageBracket", self.urls().fapi_private)
        };

        let timestamp = self.fetch_time_raw().await?;
        let auth = self.get_auth()?;
        let signed_params =
            auth.sign_with_timestamp(&request_params, timestamp, Some(self.options().recv_window))?;

        let mut request_url = format!("{}?", url);
        for (key, value) in &signed_params {
            request_url.push_str(&format!("{}={}&", key, value));
        }

        let mut headers = HeaderMap::new();
        auth.add_auth_headers_reqwest(&mut headers);

        let response = self
            .base()
            .http_client
            .get(&request_url, Some(headers))
            .await?;

        let mut tiers = HashMap::new();

        if let Some(tiers_array) = response.as_array() {
            for tier_data in tiers_array {
                if let Ok(symbol_id) = tier_data["symbol"]
                    .as_str()
                    .ok_or_else(|| Error::from(ParseError::missing_field("symbol")))
                {
                    if let Ok(market) = self.base().market_by_id(symbol_id).await {
                        if let Some(ref syms) = symbols {
                            if !syms.contains(&market.symbol) {
                                continue;
                            }
                        }

                        if let Some(brackets) = tier_data["brackets"].as_array() {
                            let mut tier_list = Vec::new();
                            for bracket in brackets {
                                match parser::parse_leverage_tier(bracket, &market) {
                                    Ok(tier) => tier_list.push(tier),
                                    Err(e) => {
                                        warn!(error = %e, "Failed to parse leverage tier");
                                    }
                                }
                            }
                            if !tier_list.is_empty() {
                                tiers.insert(market.symbol.clone(), tier_list);
                            }
                        }
                    }
                }
            }
        }

        Ok(tiers)
    }

    /// Fetch funding fee history.
    ///
    /// # Arguments
    ///
    /// * `symbol` - Optional trading pair symbol.
    /// * `since` - Optional start timestamp in milliseconds.
    /// * `limit` - Optional record limit (default 100, max 1000).
    /// * `params` - Optional parameters.
    ///
    /// # Returns
    ///
    /// Returns a vector of funding fee history records.
    ///
    /// # Errors
    ///
    /// Returns an error if authentication fails or the API request fails.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use ccxt_exchanges::binance::Binance;
    /// # use ccxt_core::ExchangeConfig;
    /// # async fn example() -> ccxt_core::Result<()> {
    /// let mut config = ExchangeConfig::default();
    /// config.api_key = Some("your_api_key".to_string());
    /// config.secret = Some("your_secret".to_string());
    /// let binance = Binance::new_swap(config)?;
    /// let history = binance.fetch_funding_history(
    ///     Some("BTC/USDT:USDT"),
    ///     None,
    ///     Some(100),
    ///     None
    /// ).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn fetch_funding_history(
        &self,
        symbol: Option<&str>,
        since: Option<u64>,
        limit: Option<u32>,
        params: Option<Value>,
    ) -> Result<Vec<ccxt_core::types::FundingHistory>> {
        self.check_required_credentials()?;

        let mut request_params = HashMap::new();

        request_params.insert("incomeType".to_string(), "FUNDING_FEE".to_string());

        let market = if let Some(sym) = symbol {
            let m = self.base().market(sym).await?;
            request_params.insert("symbol".to_string(), m.id.clone());
            Some(m)
        } else {
            None
        };

        if let Some(s) = since {
            request_params.insert("startTime".to_string(), s.to_string());
        }

        if let Some(l) = limit {
            request_params.insert("limit".to_string(), l.to_string());
        }

        if let Some(params) = params {
            if let Some(obj) = params.as_object() {
                for (key, value) in obj {
                    if let Some(v) = value.as_str() {
                        request_params.insert(key.clone(), v.to_string());
                    } else if let Some(v) = value.as_i64() {
                        request_params.insert(key.clone(), v.to_string());
                    }
                }
            }
        }

        let timestamp = self.fetch_time_raw().await?;
        let auth = self.get_auth()?;
        let signed_params =
            auth.sign_with_timestamp(&request_params, timestamp, Some(self.options().recv_window))?;
        let query_string = auth.build_query_string(&signed_params);
        let use_coin_m = market
            .as_ref()
            .map(|m| !m.linear.unwrap_or(false))
            .unwrap_or(false);

        let url = if use_coin_m {
            format!("{}/income", self.urls().dapi_private)
        } else {
            format!("{}/income", self.urls().fapi_private)
        };

        let request_url = format!("{}?{}", url, query_string);

        let mut headers = HeaderMap::new();
        auth.add_auth_headers_reqwest(&mut headers);

        let data = self
            .base()
            .http_client
            .get(&request_url, Some(headers))
            .await?;

        let history_array = data.as_array().ok_or_else(|| {
            Error::from(ParseError::invalid_format(
                "data",
                "Expected array of funding history",
            ))
        })?;

        let mut history = Vec::new();
        for history_data in history_array {
            match parser::parse_funding_history(history_data, market.as_ref().map(|v| &**v)) {
                Ok(funding) => history.push(funding),
                Err(e) => {
                    warn!(error = %e, "Failed to parse funding history");
                }
            }
        }

        Ok(history)
    }
    /// Fetch my funding fee history
    ///
    /// # Arguments
    ///
    /// * `symbol` - Optional trading pair symbol.
    /// * `start_time` - Optional start timestamp.
    /// * `end_time` - Optional end timestamp.
    /// * `limit` - Optional record limit (default 100, max 1000).
    ///
    /// # Returns
    ///
    /// Returns a vector of funding fee history records.
    ///
    /// # Errors
    ///
    /// Returns an error if authentication fails or the API request fails.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use ccxt_exchanges::binance::Binance;
    /// # use ccxt_core::ExchangeConfig;
    /// # async fn example() -> ccxt_core::Result<()> {
    /// let mut config = ExchangeConfig::default();
    /// config.api_key = Some("your_api_key".to_string());
    /// config.secret = Some("your_secret".to_string());
    /// let binance = Binance::new_swap(config)?;
    /// let fees = binance.fetch_my_funding_history(Some("BTC/USDT:USDT"), None, None, Some(50)).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn fetch_my_funding_history(
        &self,
        symbol: Option<&str>,
        start_time: Option<u64>,
        end_time: Option<u64>,
        limit: Option<u32>,
    ) -> Result<Vec<FundingFee>> {
        self.check_required_credentials()?;

        let mut request_params = HashMap::new();

        request_params.insert("incomeType".to_string(), "FUNDING_FEE".to_string());

        let market = if let Some(sym) = symbol {
            let m = self.base().market(sym).await?;
            request_params.insert("symbol".to_string(), m.id.clone());
            Some(m)
        } else {
            None
        };

        if let Some(s) = start_time {
            request_params.insert("startTime".to_string(), s.to_string());
        }

        if let Some(e) = end_time {
            request_params.insert("endTime".to_string(), e.to_string());
        }

        if let Some(l) = limit {
            request_params.insert("limit".to_string(), l.to_string());
        }

        let timestamp = self.fetch_time_raw().await?;
        let auth = self.get_auth()?;
        let signed_params =
            auth.sign_with_timestamp(&request_params, timestamp, Some(self.options().recv_window))?;

        let use_coin_m = market
            .as_ref()
            .map(|m| !m.linear.unwrap_or(false))
            .unwrap_or(false);

        let url = if use_coin_m {
            format!("{}/income", self.urls().dapi_private)
        } else {
            format!("{}/income", self.urls().fapi_private)
        };
        let query_string = auth.build_query_string(&signed_params);
        let request_url = format!("{}?{}", url, query_string);

        let mut headers = HeaderMap::new();
        auth.add_auth_headers_reqwest(&mut headers);

        let data = self
            .base()
            .http_client
            .get(&request_url, Some(headers))
            .await?;

        let fees_array = data.as_array().ok_or_else(|| {
            Error::from(ParseError::invalid_format(
                "data",
                "Expected array of funding fees",
            ))
        })?;

        let mut fees = Vec::new();
        for fee_data in fees_array {
            match parser::parse_funding_fee(fee_data, market.as_ref().map(|v| &**v)) {
                Ok(fee) => fees.push(fee),
                Err(e) => {
                    warn!(error = %e, "Failed to parse funding fee");
                }
            }
        }

        Ok(fees)
    }

    /// Fetch next funding rate (prediction) for a trading pair.
    ///
    /// # Arguments
    ///
    /// * `symbol` - Trading pair symbol.
    ///
    /// # Returns
    ///
    /// Returns next funding rate information.
    ///
    /// # Errors
    ///
    /// Returns an error if the API request fails.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use ccxt_exchanges::binance::Binance;
    /// # use ccxt_core::ExchangeConfig;
    /// # async fn example() -> ccxt_core::Result<()> {
    /// let binance = Binance::new_swap(ExchangeConfig::default())?;
    /// let next_rate = binance.fetch_next_funding_rate("BTC/USDT:USDT").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn fetch_next_funding_rate(&self, symbol: &str) -> Result<NextFundingRate> {
        let market = self.base().market(symbol).await?;

        let use_coin_m = !market.linear.unwrap_or(false);

        let url = if use_coin_m {
            format!("{}/premiumIndex", self.urls().dapi_public)
        } else {
            format!("{}/premiumIndex", self.urls().fapi_public)
        };

        let mut request_params = HashMap::new();
        request_params.insert("symbol".to_string(), market.id.clone());

        let mut request_url = format!("{}?", url);
        for (key, value) in &request_params {
            request_url.push_str(&format!("{}={}&", key, value));
        }

        let data = self.base().http_client.get(&request_url, None).await?;

        parser::parse_next_funding_rate(&data, &market)
    }

    /// Set position mode (one-way or hedge mode).
    ///
    /// # Arguments
    ///
    /// * `dual_side` - Enable hedge mode (dual-side position).
    ///   - `true`: Hedge mode (can hold both long and short positions simultaneously).
    ///   - `false`: One-way mode (can only hold one direction).
    /// * `params` - Optional parameters.
    ///
    /// # Returns
    ///
    /// Returns the operation result.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Authentication fails
    /// - Positions are currently open (cannot switch modes with open positions)
    /// - The API request fails
    ///
    /// # Notes
    ///
    /// - This setting applies globally to the account, not per trading pair.
    /// - In hedge mode, orders must specify `positionSide` (LONG/SHORT).
    /// - In one-way mode, `positionSide` is fixed to BOTH.
    /// - Cannot switch modes while holding positions.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use ccxt_exchanges::binance::Binance;
    /// # use ccxt_core::ExchangeConfig;
    /// # async fn example() -> ccxt_core::Result<()> {
    /// let binance = Binance::new_swap(ExchangeConfig::default())?;
    ///
    /// // Enable hedge mode
    /// let result = binance.set_position_mode(true, None).await?;
    ///
    /// // Switch back to one-way mode
    /// let result = binance.set_position_mode(false, None).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn set_position_mode(&self, dual_side: bool, params: Option<Value>) -> Result<Value> {
        self.check_required_credentials()?;

        let mut request_params = HashMap::new();
        request_params.insert("dualSidePosition".to_string(), dual_side.to_string());

        if let Some(params) = params {
            if let Some(obj) = params.as_object() {
                for (key, value) in obj {
                    if let Some(v) = value.as_str() {
                        request_params.insert(key.clone(), v.to_string());
                    } else if let Some(v) = value.as_bool() {
                        request_params.insert(key.clone(), v.to_string());
                    } else if let Some(v) = value.as_i64() {
                        request_params.insert(key.clone(), v.to_string());
                    }
                }
            }
        }

        let timestamp = self.fetch_time_raw().await?;
        let auth = self.get_auth()?;
        let signed_params =
            auth.sign_with_timestamp(&request_params, timestamp, Some(self.options().recv_window))?;

        let url = format!("{}/positionSide/dual", self.urls().fapi_private);

        let mut headers = HeaderMap::new();
        auth.add_auth_headers_reqwest(&mut headers);

        let body = serde_json::to_value(&signed_params).map_err(|e| {
            Error::from(ParseError::invalid_format(
                "data",
                format!("Failed to serialize params: {}", e),
            ))
        })?;

        let data = self
            .base()
            .http_client
            .post(&url, Some(headers), Some(body))
            .await?;

        Ok(data)
    }

    /// Fetch current position mode.
    ///
    /// # Arguments
    ///
    /// * `params` - Optional parameters.
    ///
    /// # Returns
    ///
    /// Returns the current position mode:
    /// - `true`: Hedge mode (dual-side position).
    /// - `false`: One-way mode.
    ///
    /// # Errors
    ///
    /// Returns an error if authentication fails or the API request fails.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use ccxt_exchanges::binance::Binance;
    /// # use ccxt_core::ExchangeConfig;
    /// # async fn example() -> ccxt_core::Result<()> {
    /// let binance = Binance::new_swap(ExchangeConfig::default())?;
    ///
    /// let dual_side = binance.fetch_position_mode(None).await?;
    /// println!("Hedge mode enabled: {}", dual_side);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn fetch_position_mode(&self, params: Option<Value>) -> Result<bool> {
        self.check_required_credentials()?;

        let mut request_params = HashMap::new();

        if let Some(params) = params {
            if let Some(obj) = params.as_object() {
                for (key, value) in obj {
                    if let Some(v) = value.as_str() {
                        request_params.insert(key.clone(), v.to_string());
                    }
                }
            }
        }

        let timestamp = self.fetch_time_raw().await?;
        let auth = self.get_auth()?;
        let signed_params =
            auth.sign_with_timestamp(&request_params, timestamp, Some(self.options().recv_window))?;

        let query_string = signed_params
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<_>>()
            .join("&");

        let url = format!(
            "{}/positionSide/dual?{}",
            self.urls().fapi_private,
            query_string
        );

        let mut headers = HeaderMap::new();
        auth.add_auth_headers_reqwest(&mut headers);

        let data = self.base().http_client.get(&url, Some(headers)).await?;

        if let Some(dual_side) = data.get("dualSidePosition") {
            if let Some(value) = dual_side.as_bool() {
                return Ok(value);
            }
            if let Some(value_str) = dual_side.as_str() {
                return Ok(value_str.to_lowercase() == "true");
            }
        }

        Err(Error::from(ParseError::invalid_format(
            "data",
            "Failed to parse position mode response",
        )))
    }

    // ==================== Futures Margin Management ====================

    /// Modify isolated position margin.
    ///
    /// # Arguments
    ///
    /// * `symbol` - Trading pair symbol (e.g., "BTC/USDT").
    /// * `amount` - Adjustment amount (positive to add, negative to reduce).
    /// * `params` - Optional parameters:
    ///   - `type`: Operation type (1=add, 2=reduce). If provided, `amount` sign is ignored.
    ///   - `positionSide`: Position side "LONG" | "SHORT" (required in hedge mode).
    ///
    /// # Returns
    ///
    /// Returns the adjustment result including the new margin amount.
    ///
    /// # Errors
    ///
    /// Returns an error if authentication fails or the API request fails.
    /// ```rust,no_run
    /// # use ccxt_exchanges::binance::Binance;
    /// # use ccxt_core::ExchangeConfig;
    /// # use rust_decimal_macros::dec;
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let binance = Binance::new_swap(ExchangeConfig::default())?;
    ///
    /// // Add 100 USDT to isolated margin
    /// binance.modify_isolated_position_margin("BTC/USDT", dec!(100.0), None).await?;
    ///
    /// // Reduce 50 USDT from isolated margin
    /// binance.modify_isolated_position_margin("BTC/USDT", dec!(-50.0), None).await?;
    ///
    /// // Adjust long position margin in hedge mode
    /// let params = serde_json::json!({"positionSide": "LONG"});
    /// binance.modify_isolated_position_margin("BTC/USDT", dec!(100.0), Some(params)).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn modify_isolated_position_margin(
        &self,
        symbol: &str,
        amount: Decimal,
        params: Option<Value>,
    ) -> Result<Value> {
        self.check_required_credentials()?;

        let market = self.base().market(symbol).await?;
        let mut request_params = HashMap::new();
        request_params.insert("symbol".to_string(), market.id.clone());
        request_params.insert("amount".to_string(), amount.abs().to_string());
        request_params.insert(
            "type".to_string(),
            if amount > Decimal::ZERO {
                "1".to_string()
            } else {
                "2".to_string()
            },
        );

        if let Some(params) = params {
            if let Some(obj) = params.as_object() {
                for (key, value) in obj {
                    if let Some(v) = value.as_str() {
                        request_params.insert(key.clone(), v.to_string());
                    }
                }
            }
        }

        let timestamp = self.fetch_time_raw().await?;
        let auth = self.get_auth()?;
        let signed_params =
            auth.sign_with_timestamp(&request_params, timestamp, Some(self.options().recv_window))?;

        let url = if market.linear.unwrap_or(true) {
            format!("{}/positionMargin", self.urls().fapi_private)
        } else {
            format!("{}/positionMargin", self.urls().dapi_private)
        };

        let mut headers = HeaderMap::new();
        auth.add_auth_headers_reqwest(&mut headers);

        let body = serde_json::to_value(&signed_params).map_err(|e| {
            Error::from(ParseError::invalid_format(
                "data",
                format!("Failed to serialize params: {}", e),
            ))
        })?;

        let data = self
            .base()
            .http_client
            .post(&url, Some(headers), Some(body))
            .await?;

        Ok(data)
    }

    /// Fetch position risk information.
    ///
    /// Retrieves risk information for all futures positions, including unrealized PnL,
    /// liquidation price, leverage, etc.
    ///
    /// # Arguments
    ///
    /// * `symbol` - Optional trading pair symbol. `None` returns all positions.
    /// * `params` - Optional parameters.
    ///
    /// # Returns
    ///
    /// Returns position risk information array.
    ///
    /// # Errors
    ///
    /// Returns an error if authentication fails or the API request fails.
    ///
    /// # Examples
    /// ```rust,no_run
    /// # use ccxt_exchanges::binance::Binance;
    /// # use ccxt_core::ExchangeConfig;
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let binance = Binance::new_swap(ExchangeConfig::default())?;
    ///
    /// // Fetch all position risks
    /// let risks = binance.fetch_position_risk(None, None).await?;
    ///
    /// // Fetch position risk for specific symbol
    /// let risk = binance.fetch_position_risk(Some("BTC/USDT"), None).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn fetch_position_risk(
        &self,
        symbol: Option<&str>,
        params: Option<Value>,
    ) -> Result<Value> {
        self.check_required_credentials()?;

        let mut request_params = HashMap::new();

        if let Some(sym) = symbol {
            let market = self.base().market(sym).await?;
            request_params.insert("symbol".to_string(), market.id.clone());
        }

        if let Some(params) = params {
            if let Some(obj) = params.as_object() {
                for (key, value) in obj {
                    if let Some(v) = value.as_str() {
                        request_params.insert(key.clone(), v.to_string());
                    }
                }
            }
        }

        let timestamp = self.fetch_time_raw().await?;
        let auth = self.get_auth()?;
        let signed_params =
            auth.sign_with_timestamp(&request_params, timestamp, Some(self.options().recv_window))?;

        let query_string: String = signed_params
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<_>>()
            .join("&");

        let url = format!("{}/positionRisk?{}", self.urls().fapi_private, query_string);

        let mut headers = HeaderMap::new();
        auth.add_auth_headers_reqwest(&mut headers);

        let data = self.base().http_client.get(&url, Some(headers)).await?;

        Ok(data)
    }

    /// Fetch leverage bracket information.
    ///
    /// Retrieves leverage bracket information for specified or all trading pairs,
    /// showing maximum leverage for different notional value tiers.
    ///
    /// # Arguments
    ///
    /// * `symbol` - Optional trading pair symbol. `None` returns all pairs.
    /// * `params` - Optional parameters.
    ///
    /// # Returns
    ///
    /// Returns leverage bracket information including maximum notional value and
    /// corresponding maximum leverage for each tier.
    ///
    /// # Errors
    ///
    /// Returns an error if authentication fails or the API request fails.
    ///
    /// # Examples
    /// ```rust,no_run
    /// # use ccxt_exchanges::binance::Binance;
    /// # use ccxt_core::ExchangeConfig;
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let binance = Binance::new_swap(ExchangeConfig::default())?;
    ///
    /// // Fetch all leverage brackets
    /// let brackets = binance.fetch_leverage_bracket(None, None).await?;
    ///
    /// // Fetch bracket for specific symbol
    /// let bracket = binance.fetch_leverage_bracket(Some("BTC/USDT"), None).await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Response Example
    /// ```json
    /// [
    ///   {
    ///     "symbol": "BTCUSDT",
    ///     "brackets": [
    ///       {
    ///         "bracket": 1,
    ///         "initialLeverage": 125,
    ///         "notionalCap": 50000,
    ///         "notionalFloor": 0,
    ///         "maintMarginRatio": 0.004,
    ///         "cum": 0
    ///       },
    ///       {
    ///         "bracket": 2,
    ///         "initialLeverage": 100,
    ///         "notionalCap": 250000,
    ///         "notionalFloor": 50000,
    ///         "maintMarginRatio": 0.005,
    ///         "cum": 50
    ///       }
    ///     ]
    ///   }
    /// ]
    /// ```
    pub async fn fetch_leverage_bracket(
        &self,
        symbol: Option<&str>,
        params: Option<Value>,
    ) -> Result<Value> {
        self.check_required_credentials()?;

        let mut request_params = HashMap::new();

        if let Some(sym) = symbol {
            let market = self.base().market(sym).await?;
            request_params.insert("symbol".to_string(), market.id.clone());
        }

        if let Some(params) = params {
            if let Some(obj) = params.as_object() {
                for (key, value) in obj {
                    if let Some(v) = value.as_str() {
                        request_params.insert(key.clone(), v.to_string());
                    }
                }
            }
        }

        let timestamp = self.fetch_time_raw().await?;
        let auth = self.get_auth()?;
        let signed_params =
            auth.sign_with_timestamp(&request_params, timestamp, Some(self.options().recv_window))?;

        let query_string: String = signed_params
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<_>>()
            .join("&");

        let url = format!(
            "{}/leverageBracket?{}",
            self.urls().fapi_private,
            query_string
        );

        let mut headers = HeaderMap::new();
        auth.add_auth_headers_reqwest(&mut headers);

        let data = self.base().http_client.get(&url, Some(headers)).await?;

        Ok(data)
    }

    // ==================== Margin Trading ====================

    /// Borrow funds in cross margin mode.
    ///
    /// # Arguments
    ///
    /// * `currency` - Currency code.
    /// * `amount` - Borrow amount.
    ///
    /// # Returns
    ///
    /// Returns a margin loan record.
    ///
    /// # Errors
    ///
    /// Returns an error if authentication fails or the API request fails.
    pub async fn borrow_cross_margin(&self, currency: &str, amount: f64) -> Result<MarginLoan> {
        let mut params = HashMap::new();
        params.insert("asset".to_string(), currency.to_string());
        params.insert("amount".to_string(), amount.to_string());

        let timestamp = self.fetch_time_raw().await?;
        let auth = self.get_auth()?;
        let signed_params =
            auth.sign_with_timestamp(&params, timestamp, Some(self.options().recv_window))?;

        let url = format!("{}/sapi/v1/margin/loan", self.urls().sapi);

        let mut headers = HeaderMap::new();
        auth.add_auth_headers_reqwest(&mut headers);

        let body = serde_json::to_value(&signed_params).map_err(|e| {
            Error::from(ParseError::invalid_format(
                "data",
                format!("Failed to serialize params: {}", e),
            ))
        })?;

        let data = self
            .base()
            .http_client
            .post(&url, Some(headers), Some(body))
            .await?;

        parser::parse_margin_loan(&data)
    }

    /// Borrow funds in isolated margin mode.
    ///
    /// # Arguments
    ///
    /// * `symbol` - Trading pair symbol.
    /// * `currency` - Currency code.
    /// * `amount` - Borrow amount.
    ///
    /// # Returns
    ///
    /// Returns a margin loan record.
    ///
    /// # Errors
    ///
    /// Returns an error if authentication fails or the API request fails.
    pub async fn borrow_isolated_margin(
        &self,
        symbol: &str,
        currency: &str,
        amount: f64,
    ) -> Result<MarginLoan> {
        self.load_markets(false).await?;
        let market = self.base().market(symbol).await?;

        let mut params = HashMap::new();
        params.insert("asset".to_string(), currency.to_string());
        params.insert("amount".to_string(), amount.to_string());
        params.insert("symbol".to_string(), market.id.clone());
        params.insert("isIsolated".to_string(), "TRUE".to_string());

        let timestamp = self.fetch_time_raw().await?;
        let auth = self.get_auth()?;
        let signed_params =
            auth.sign_with_timestamp(&params, timestamp, Some(self.options().recv_window))?;

        let url = format!("{}/sapi/v1/margin/loan", self.urls().sapi);

        let mut headers = HeaderMap::new();
        auth.add_auth_headers_reqwest(&mut headers);

        let body = serde_json::to_value(&signed_params).map_err(|e| {
            Error::from(ParseError::invalid_format(
                "data",
                format!("Failed to serialize params: {}", e),
            ))
        })?;

        let data = self
            .base()
            .http_client
            .post(&url, Some(headers), Some(body))
            .await?;

        parser::parse_margin_loan(&data)
    }

    /// Repay borrowed funds in cross margin mode.
    ///
    /// # Arguments
    ///
    /// * `currency` - Currency code.
    /// * `amount` - Repayment amount.
    ///
    /// # Returns
    ///
    /// Returns a margin repayment record.
    ///
    /// # Errors
    ///
    /// Returns an error if authentication fails or the API request fails.
    pub async fn repay_cross_margin(&self, currency: &str, amount: f64) -> Result<MarginRepay> {
        let mut params = HashMap::new();
        params.insert("asset".to_string(), currency.to_string());
        params.insert("amount".to_string(), amount.to_string());

        let timestamp = self.fetch_time_raw().await?;
        let auth = self.get_auth()?;
        let signed_params =
            auth.sign_with_timestamp(&params, timestamp, Some(self.options().recv_window))?;

        let url = format!("{}/sapi/v1/margin/repay", self.urls().sapi);

        let mut headers = HeaderMap::new();
        auth.add_auth_headers_reqwest(&mut headers);

        let body = serde_json::to_value(&signed_params).map_err(|e| {
            Error::from(ParseError::invalid_format(
                "data",
                format!("Failed to serialize params: {}", e),
            ))
        })?;

        let data = self
            .base()
            .http_client
            .post(&url, Some(headers), Some(body))
            .await?;

        let loan = parser::parse_margin_loan(&data)?;

        Ok(MarginRepay {
            id: loan.id,
            currency: loan.currency,
            amount: loan.amount,
            symbol: loan.symbol,
            timestamp: loan.timestamp,
            datetime: loan.datetime,
            status: loan.status,
            is_isolated: loan.is_isolated,
            info: loan.info,
        })
    }

    /// Repay borrowed funds in isolated margin mode.
    ///
    /// # Arguments
    ///
    /// * `symbol` - Trading pair symbol.
    /// * `currency` - Currency code.
    /// * `amount` - Repayment amount.
    ///
    /// # Returns
    ///
    /// Returns a margin repayment record.
    ///
    /// # Errors
    ///
    /// Returns an error if authentication fails or the API request fails.
    pub async fn repay_isolated_margin(
        &self,
        symbol: &str,
        currency: &str,
        amount: rust_decimal::Decimal,
    ) -> Result<MarginRepay> {
        self.load_markets(false).await?;
        let market = self.base().market(symbol).await?;

        let mut params = HashMap::new();
        params.insert("asset".to_string(), currency.to_string());
        params.insert("amount".to_string(), amount.to_string());
        params.insert("symbol".to_string(), market.id.clone());
        params.insert("isIsolated".to_string(), "TRUE".to_string());

        let timestamp = self.fetch_time_raw().await?;
        let auth = self.get_auth()?;
        let signed_params =
            auth.sign_with_timestamp(&params, timestamp, Some(self.options().recv_window))?;

        let url = format!("{}/sapi/v1/margin/repay", self.urls().sapi);

        let mut headers = HeaderMap::new();
        auth.add_auth_headers_reqwest(&mut headers);

        let body = serde_json::to_value(&signed_params).map_err(|e| {
            Error::from(ParseError::invalid_format(
                "data",
                format!("Failed to serialize params: {}", e),
            ))
        })?;

        let data = self
            .base()
            .http_client
            .post(&url, Some(headers), Some(body))
            .await?;

        let loan = parser::parse_margin_loan(&data)?;

        Ok(MarginRepay {
            id: loan.id,
            currency: loan.currency,
            amount: loan.amount,
            symbol: loan.symbol,
            timestamp: loan.timestamp,
            datetime: loan.datetime,
            status: loan.status,
            is_isolated: loan.is_isolated,
            info: loan.info,
        })
    }

    /// Fetch margin adjustment history.
    ///
    /// Retrieves liquidation records and margin adjustment history.
    ///
    /// # Arguments
    /// * `symbol` - Optional trading pair symbol (required for isolated margin)
    /// * `since` - Start timestamp in milliseconds
    /// * `limit` - Maximum number of records to return
    ///
    /// # Returns
    /// Returns a list of margin adjustments.
    ///
    /// # Errors
    /// Returns an error if authentication fails or the API request fails.
    pub async fn fetch_margin_adjustment_history(
        &self,
        symbol: Option<&str>,
        since: Option<i64>,
        limit: Option<i64>,
    ) -> Result<Vec<MarginAdjustment>> {
        let mut params = HashMap::new();

        if let Some(sym) = symbol {
            self.load_markets(false).await?;
            let market = self.base().market(sym).await?;
            params.insert("symbol".to_string(), market.id.clone());
            params.insert("isolatedSymbol".to_string(), market.id.clone());
        }

        if let Some(start_time) = since {
            params.insert("startTime".to_string(), start_time.to_string());
        }

        if let Some(size) = limit {
            params.insert("size".to_string(), size.to_string());
        }

        let timestamp = self.fetch_time_raw().await?;
        let auth = self.get_auth()?;
        let _signed_params =
            auth.sign_with_timestamp(&params, timestamp, Some(self.options().recv_window))?;

        let url = format!("{}/sapi/v1/margin/forceLiquidationRec", self.urls().sapi);

        let mut headers = HeaderMap::new();
        auth.add_auth_headers_reqwest(&mut headers);

        let data = self.base().http_client.get(&url, Some(headers)).await?;

        let rows = data["rows"].as_array().ok_or_else(|| {
            Error::from(ParseError::invalid_format(
                "data",
                "Expected rows array in response",
            ))
        })?;

        let mut results = Vec::new();
        for item in rows {
            if let Ok(adjustment) = parser::parse_margin_adjustment(item) {
                results.push(adjustment);
            }
        }

        Ok(results)
    }
    /// Fetch account balance.
    ///
    /// Retrieves balance information for various account types including spot,
    /// margin, futures, savings, and funding accounts.
    ///
    /// # Arguments
    /// * `params` - Optional parameters
    ///   - `type`: Account type ("spot", "margin", "isolated", "linear", "inverse", "savings", "funding", "papi")
    ///   - `marginMode`: Margin mode ("cross", "isolated")
    ///   - `symbols`: List of isolated margin trading pairs (used with "isolated" type)
    ///
    /// # Returns
    /// Returns account balance information.
    ///
    /// # Errors
    /// Returns an error if authentication fails or the API request fails.
    pub async fn fetch_balance(&self, params: Option<HashMap<String, Value>>) -> Result<Balance> {
        let params = params.unwrap_or_default();

        let account_type = params
            .get("type")
            .and_then(|v| v.as_str())
            .unwrap_or("spot");
        let _margin_mode = params.get("marginMode").and_then(|v| v.as_str());

        let (url, method) = match account_type {
            "spot" => (format!("{}/account", self.urls().public), "GET"),
            "margin" | "cross" => (
                format!("{}/sapi/v1/margin/account", self.urls().sapi),
                "GET",
            ),
            "isolated" => (
                format!("{}/sapi/v1/margin/isolated/account", self.urls().sapi),
                "GET",
            ),
            "linear" | "future" => (
                format!("{}/balance", self.urls().fapi_public.replace("/v1", "/v2")),
                "GET",
            ),
            "inverse" | "delivery" => (format!("{}/account", self.urls().dapi_public), "GET"),
            "savings" => (
                format!("{}/sapi/v1/lending/union/account", self.urls().sapi),
                "GET",
            ),
            "funding" => (
                format!("{}/asset/get-funding-asset", self.urls().sapi),
                "POST",
            ),
            "papi" => (format!("{}/papi/v1/balance", self.urls().sapi), "GET"),
            _ => (format!("{}/account", self.urls().public), "GET"),
        };

        let mut request_params = HashMap::new();

        if account_type == "isolated" {
            if let Some(symbols) = params.get("symbols").and_then(|v| v.as_array()) {
                let symbols_str: Vec<String> = symbols
                    .iter()
                    .filter_map(|v| v.as_str())
                    .map(|s| s.to_string())
                    .collect();
                if !symbols_str.is_empty() {
                    request_params.insert("symbols".to_string(), symbols_str.join(","));
                }
            }
        }
        let timestamp = self.fetch_time_raw().await?;
        let auth = self.get_auth()?;
        let signed_params =
            auth.sign_with_timestamp(&request_params, timestamp, Some(self.options().recv_window))?;

        let query_string = {
            let mut pairs: Vec<_> = signed_params.iter().collect();
            pairs.sort_by_key(|(k, _)| *k);
            pairs
                .iter()
                .map(|(k, v)| format!("{}={}", k, v))
                .collect::<Vec<_>>()
                .join("&")
        };
        let full_url = format!("{}?{}", url, query_string);

        let mut headers = HeaderMap::new();
        auth.add_auth_headers_reqwest(&mut headers);

        let data = if method == "POST" {
            self.base()
                .http_client
                .post(&full_url, Some(headers), None)
                .await?
        } else {
            self.base()
                .http_client
                .get(&full_url, Some(headers))
                .await?
        };

        parser::parse_balance_with_type(&data, account_type)
    }

    /// Fetch maximum borrowable amount for cross margin.
    ///
    /// Queries the maximum amount that can be borrowed for a specific currency
    /// in cross margin mode.
    ///
    /// # Arguments
    /// * `code` - Currency code (e.g., "USDT")
    /// * `params` - Optional parameters (reserved for future use)
    ///
    /// # Returns
    /// Returns maximum borrowable amount information.
    ///
    /// # Errors
    /// Returns an error if authentication fails or the API request fails.
    pub async fn fetch_cross_margin_max_borrowable(
        &self,
        code: &str,
        params: Option<HashMap<String, Value>>,
    ) -> Result<MaxBorrowable> {
        let _params = params.unwrap_or_default();

        let mut request_params = HashMap::new();
        request_params.insert("asset".to_string(), code.to_string());

        let timestamp = self.fetch_time_raw().await?;
        let auth = self.get_auth()?;
        let signed_params =
            auth.sign_with_timestamp(&request_params, timestamp, Some(self.options().recv_window))?;

        let query_string = {
            let mut pairs: Vec<_> = signed_params.iter().collect();
            pairs.sort_by_key(|(k, _)| *k);
            pairs
                .iter()
                .map(|(k, v)| format!("{}={}", k, v))
                .collect::<Vec<_>>()
                .join("&")
        };
        let url = format!(
            "{}/sapi/v1/margin/maxBorrowable?{}",
            self.urls().sapi,
            query_string
        );

        let mut headers = HeaderMap::new();
        auth.add_auth_headers_reqwest(&mut headers);

        let data = self.base().http_client.get(&url, Some(headers)).await?;

        parser::parse_max_borrowable(&data, code, None)
    }

    /// Fetch maximum borrowable amount for isolated margin.
    ///
    /// Queries the maximum amount that can be borrowed for a specific currency
    /// in a specific isolated margin trading pair.
    ///
    /// # Arguments
    /// * `code` - Currency code (e.g., "USDT")
    /// * `symbol` - Trading pair symbol (e.g., "BTC/USDT")
    /// * `params` - Optional parameters (reserved for future use)
    ///
    /// # Returns
    /// Returns maximum borrowable amount information.
    ///
    /// # Errors
    /// Returns an error if authentication fails or the API request fails.
    pub async fn fetch_isolated_margin_max_borrowable(
        &self,
        code: &str,
        symbol: &str,
        params: Option<HashMap<String, Value>>,
    ) -> Result<MaxBorrowable> {
        let _params = params.unwrap_or_default();

        self.load_markets(false).await?;
        let market = self.base().market(symbol).await?;

        let mut request_params = HashMap::new();
        request_params.insert("asset".to_string(), code.to_string());
        request_params.insert("isolatedSymbol".to_string(), market.id.clone());

        let timestamp = self.fetch_time_raw().await?;
        let auth = self.get_auth()?;
        let signed_params =
            auth.sign_with_timestamp(&request_params, timestamp, Some(self.options().recv_window))?;

        let query_string = signed_params
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<_>>()
            .join("&");
        let url = format!(
            "{}/sapi/v1/margin/isolated/maxBorrowable?{}",
            self.urls().sapi,
            query_string
        );

        let mut headers = HeaderMap::new();
        auth.add_auth_headers_reqwest(&mut headers);

        let data = self.base().http_client.get(&url, Some(headers)).await?;

        parser::parse_max_borrowable(&data, code, Some(symbol.to_string()))
    }

    /// Fetch maximum transferable amount.
    ///
    /// Queries the maximum amount that can be transferred out of margin account.
    ///
    /// # Arguments
    /// * `code` - Currency code (e.g., "USDT")
    /// * `params` - Optional parameters
    ///   - `symbol`: Isolated margin trading pair symbol (e.g., "BTC/USDT")
    ///
    /// # Returns
    /// Returns maximum transferable amount information.
    ///
    /// # Errors
    /// Returns an error if authentication fails or the API request fails.
    pub async fn fetch_max_transferable(
        &self,
        code: &str,
        params: Option<HashMap<String, Value>>,
    ) -> Result<MaxTransferable> {
        let params = params.unwrap_or_default();

        let mut request_params = HashMap::new();
        request_params.insert("asset".to_string(), code.to_string());

        let symbol_opt: Option<String> = if let Some(symbol_value) = params.get("symbol") {
            if let Some(symbol_str) = symbol_value.as_str() {
                self.load_markets(false).await?;
                let market = self.base().market(symbol_str).await?;
                request_params.insert("isolatedSymbol".to_string(), market.id.clone());
                Some(symbol_str.to_string())
            } else {
                None
            }
        } else {
            None
        };

        let timestamp = self.fetch_time_raw().await?;
        let auth = self.get_auth()?;
        let signed_params =
            auth.sign_with_timestamp(&request_params, timestamp, Some(self.options().recv_window))?;

        let query_string = signed_params
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<_>>()
            .join("&");
        let url = format!(
            "{}/sapi/v1/margin/maxTransferable?{}",
            self.urls().sapi,
            query_string
        );

        let mut headers = HeaderMap::new();
        auth.add_auth_headers_reqwest(&mut headers);

        let data = self.base().http_client.get(&url, Some(headers)).await?;

        parser::parse_max_transferable(&data, code, symbol_opt)
    }

    // ============================================================================
    // Enhanced Market Data API
    // ============================================================================

    /// Fetch bid/ask prices (Book Ticker)
    ///
    /// # Arguments
    ///
    /// * `symbol` - Optional trading pair symbol; if omitted, returns all symbols
    ///
    /// # Returns
    ///
    /// Returns bid/ask price information
    ///
    /// # API Endpoint
    ///
    /// * GET `/api/v3/ticker/bookTicker`
    /// * Weight: 1 for single symbol, 2 for all symbols
    /// * Requires signature: No
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use ccxt_exchanges::binance::Binance;
    /// # use ccxt_core::ExchangeConfig;
    /// # async fn example() -> ccxt_core::Result<()> {
    /// let binance = Binance::new(ExchangeConfig::default())?;
    ///
    /// // Fetch bid/ask for single symbol
    /// let bid_ask = binance.fetch_bids_asks(Some("BTC/USDT")).await?;
    /// println!("BTC/USDT bid: {}, ask: {}", bid_ask[0].bid_price, bid_ask[0].ask_price);
    ///
    /// // Fetch bid/ask for all symbols
    /// let all_bid_asks = binance.fetch_bids_asks(None).await?;
    /// println!("Total symbols: {}", all_bid_asks.len());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn fetch_bids_asks(&self, symbol: Option<&str>) -> Result<Vec<BidAsk>> {
        self.load_markets(false).await?;

        let url = if let Some(sym) = symbol {
            let market = self.base().market(sym).await?;
            format!(
                "{}/ticker/bookTicker?symbol={}",
                self.urls().public,
                market.id
            )
        } else {
            format!("{}/ticker/bookTicker", self.urls().public)
        };

        let data = self.base().http_client.get(&url, None).await?;

        parser::parse_bids_asks(&data)
    }

    /// Fetch latest prices.
    ///
    /// Retrieves the most recent price for one or all trading pairs.
    ///
    /// # Arguments
    ///
    /// * `symbol` - Optional trading pair symbol; if omitted, returns all symbols
    ///
    /// # Returns
    ///
    /// Returns latest price information.
    ///
    /// # Errors
    ///
    /// Returns an error if the API request fails.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use ccxt_exchanges::binance::Binance;
    /// # use ccxt_core::ExchangeConfig;
    /// # async fn example() -> ccxt_core::Result<()> {
    /// let binance = Binance::new(ExchangeConfig::default())?;
    ///
    /// // Fetch latest price for single symbol
    /// let price = binance.fetch_last_prices(Some("BTC/USDT")).await?;
    /// println!("BTC/USDT last price: {}", price[0].price);
    ///
    /// // Fetch latest prices for all symbols
    /// let all_prices = binance.fetch_last_prices(None).await?;
    /// println!("Total symbols: {}", all_prices.len());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn fetch_last_prices(&self, symbol: Option<&str>) -> Result<Vec<LastPrice>> {
        self.load_markets(false).await?;

        let url = if let Some(sym) = symbol {
            let market = self.base().market(sym).await?;
            format!("{}/ticker/price?symbol={}", self.urls().public, market.id)
        } else {
            format!("{}/ticker/price", self.urls().public)
        };

        let data = self.base().http_client.get(&url, None).await?;

        parser::parse_last_prices(&data)
    }

    /// Fetch futures mark prices.
    ///
    /// Retrieves mark prices for futures contracts, used for calculating unrealized PnL.
    /// Includes funding rates and next funding time.
    ///
    /// # Arguments
    ///
    /// * `symbol` - Optional trading pair symbol; if omitted, returns all futures pairs
    ///
    /// # Returns
    ///
    /// Returns mark price information including funding rates.
    ///
    /// # Errors
    ///
    /// Returns an error if the API request fails.
    ///
    /// # Note
    ///
    /// This API only applies to futures markets.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use ccxt_exchanges::binance::Binance;
    /// # use ccxt_core::ExchangeConfig;
    /// # async fn example() -> ccxt_core::Result<()> {
    /// let binance = Binance::new(ExchangeConfig::default())?;
    ///
    /// // Fetch mark price for single futures symbol
    /// let mark_price = binance.fetch_mark_price(Some("BTC/USDT")).await?;
    /// println!("BTC/USDT mark price: {}", mark_price[0].mark_price);
    /// println!("Funding rate: {:?}", mark_price[0].last_funding_rate);
    ///
    /// // Fetch mark prices for all futures symbols
    /// let all_mark_prices = binance.fetch_mark_price(None).await?;
    /// println!("Total futures symbols: {}", all_mark_prices.len());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn fetch_mark_price(&self, symbol: Option<&str>) -> Result<Vec<MarkPrice>> {
        self.load_markets(false).await?;

        let url = if let Some(sym) = symbol {
            let market = self.base().market(sym).await?;
            format!(
                "{}/premiumIndex?symbol={}",
                self.urls().fapi_public,
                market.id
            )
        } else {
            format!("{}/premiumIndex", self.urls().fapi_public)
        };

        let data = self.base().http_client.get(&url, None).await?;

        parser::parse_mark_prices(&data)
    }
    /// Parse timeframe string into seconds.
    ///
    /// Converts a timeframe string like "1m", "5m", "1h", "1d" into the equivalent number of seconds.
    ///
    /// # Arguments
    ///
    /// * `timeframe` - Timeframe string such as "1m", "5m", "1h", "1d"
    ///
    /// # Returns
    ///
    /// Returns the time interval in seconds.
    ///
    /// # Errors
    ///
    /// Returns an error if the timeframe is empty or has an invalid format.
    fn parse_timeframe(&self, timeframe: &str) -> Result<i64> {
        let unit_map = [
            ("s", 1),
            ("m", 60),
            ("h", 3600),
            ("d", 86400),
            ("w", 604800),
            ("M", 2592000),
            ("y", 31536000),
        ];

        if timeframe.is_empty() {
            return Err(Error::invalid_request("timeframe cannot be empty"));
        }

        let mut num_str = String::new();
        let mut unit_str = String::new();

        for ch in timeframe.chars() {
            if ch.is_ascii_digit() {
                num_str.push(ch);
            } else {
                unit_str.push(ch);
            }
        }

        let amount: i64 = if num_str.is_empty() {
            1
        } else {
            num_str.parse().map_err(|_| {
                Error::invalid_request(format!("Invalid timeframe format: {}", timeframe))
            })?
        };

        let unit_seconds = unit_map
            .iter()
            .find(|(unit, _)| unit == &unit_str.as_str())
            .map(|(_, seconds)| *seconds)
            .ok_or_else(|| {
                Error::invalid_request(format!("Unsupported timeframe unit: {}", unit_str))
            })?;

        Ok(amount * unit_seconds)
    }

    /// Get OHLCV API endpoint based on market type and price type
    ///
    /// # Arguments
    /// * `market` - Market information
    /// * `price` - Price type: None (default) | "mark" | "index" | "premiumIndex"
    ///
    /// # Returns
    /// Returns tuple (base_url, endpoint, use_pair)
    /// * `base_url` - API base URL
    /// * `endpoint` - Specific endpoint path
    /// * `use_pair` - Whether to use pair instead of symbol (required for index price)
    ///
    /// # API Endpoint Mapping
    /// | Market Type | Price Type | API Endpoint |
    /// |-------------|------------|--------------|
    /// | spot | default | `/api/v3/klines` |
    /// | linear | default | `/fapi/v1/klines` |
    /// | linear | mark | `/fapi/v1/markPriceKlines` |
    /// | linear | index | `/fapi/v1/indexPriceKlines` |
    /// | linear | premiumIndex | `/fapi/v1/premiumIndexKlines` |
    /// | inverse | default | `/dapi/v1/klines` |
    /// | inverse | mark | `/dapi/v1/markPriceKlines` |
    /// | inverse | index | `/dapi/v1/indexPriceKlines` |
    /// | inverse | premiumIndex | `/dapi/v1/premiumIndexKlines` |
    /// | option | default | `/eapi/v1/klines` |
    fn get_ohlcv_endpoint(
        &self,
        market: &Market,
        price: Option<&str>,
    ) -> Result<(String, String, bool)> {
        use ccxt_core::types::MarketType;

        if let Some(p) = price {
            if !["mark", "index", "premiumIndex"].contains(&p) {
                return Err(Error::invalid_request(format!(
                    "Unsupported price type: {}. Supported types: mark, index, premiumIndex",
                    p
                )));
            }
        }

        match market.market_type {
            MarketType::Spot => {
                if let Some(p) = price {
                    return Err(Error::invalid_request(format!(
                        "Spot market does not support '{}' price type",
                        p
                    )));
                }
                Ok((self.urls().public.clone(), "/klines".to_string(), false))
            }

            MarketType::Swap | MarketType::Futures => {
                let is_linear = market.linear.unwrap_or(false);
                let is_inverse = market.inverse.unwrap_or(false);

                if is_linear {
                    let (endpoint, use_pair) = match price {
                        None => ("/klines".to_string(), false),
                        Some("mark") => ("/markPriceKlines".to_string(), false),
                        Some("index") => ("/indexPriceKlines".to_string(), true),
                        Some("premiumIndex") => ("/premiumIndexKlines".to_string(), false),
                        _ => unreachable!(),
                    };
                    Ok((self.urls().fapi_public.clone(), endpoint, use_pair))
                } else if is_inverse {
                    let (endpoint, use_pair) = match price {
                        None => ("/klines".to_string(), false),
                        Some("mark") => ("/markPriceKlines".to_string(), false),
                        Some("index") => ("/indexPriceKlines".to_string(), true),
                        Some("premiumIndex") => ("/premiumIndexKlines".to_string(), false),
                        _ => unreachable!(),
                    };
                    Ok((self.urls().dapi_public.clone(), endpoint, use_pair))
                } else {
                    Err(Error::invalid_request(
                        "Cannot determine futures contract type (linear or inverse)",
                    ))
                }
            }

            MarketType::Option => {
                if let Some(p) = price {
                    return Err(Error::invalid_request(format!(
                        "Option market does not support '{}' price type",
                        p
                    )));
                }
                Ok((
                    self.urls().eapi_public.clone(),
                    "/klines".to_string(),
                    false,
                ))
            }
        }
    }

    /// Fetch OHLCV (candlestick) data
    ///
    /// # Arguments
    ///
    /// * `symbol` - Trading pair symbol, e.g., "BTC/USDT"
    /// * `timeframe` - Time period, e.g., "1m", "5m", "1h", "1d"
    /// * `since` - Start timestamp in milliseconds
    /// * `limit` - Maximum number of candlesticks to return
    /// * `params` - Optional parameters
    ///   * `price` - Price type: "mark" | "index" | "premiumIndex" (futures only)
    ///   * `until` - End timestamp in milliseconds
    ///   * `paginate` - Whether to automatically paginate (default: false)
    ///
    /// # Returns
    ///
    /// Returns OHLCV data array: [timestamp, open, high, low, close, volume]
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use ccxt_exchanges::binance::Binance;
    /// # use std::collections::HashMap;
    /// # use serde_json::json;
    /// # async fn example() -> ccxt_core::Result<()> {
    /// let binance = Binance::new(Default::default())?;
    ///
    /// // Basic usage
    /// let ohlcv = binance.fetch_ohlcv("BTC/USDT", "1h", None, Some(100), None).await?;
    ///
    /// // Using time range
    /// let since = 1609459200000i64; // 2021-01-01
    /// let until = 1612137600000i64; // 2021-02-01
    /// let mut params = HashMap::new();
    /// params.insert("until".to_string(), json!(until));
    /// let ohlcv = binance.fetch_ohlcv("BTC/USDT", "1h", Some(since), None, Some(params)).await?;
    ///
    /// // Futures mark price candlesticks
    /// let mut params = HashMap::new();
    /// params.insert("price".to_string(), json!("mark"));
    /// let mark_ohlcv = binance.fetch_ohlcv("BTC/USDT:USDT", "1h", None, Some(100), Some(params)).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn fetch_ohlcv(
        &self,
        symbol: &str,
        timeframe: &str,
        since: Option<i64>,
        limit: Option<u32>,
        params: Option<HashMap<String, Value>>,
    ) -> Result<Vec<ccxt_core::types::OHLCV>> {
        self.load_markets(false).await?;

        let price = params
            .as_ref()
            .and_then(|p| p.get("price"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let until = params
            .as_ref()
            .and_then(|p| p.get("until"))
            .and_then(|v| v.as_i64());

        let paginate = params
            .as_ref()
            .and_then(|p| p.get("paginate"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        // TODO: Phase 4 - Implement pagination
        if paginate {
            return Err(Error::not_implemented(
                "Pagination feature not yet implemented, will be added in Phase 4",
            ));
        }

        let market = self.base().market(symbol).await?;

        let default_limit = 500u32;
        let max_limit = 1500u32;

        let adjusted_limit = if since.is_some() && until.is_some() && limit.is_none() {
            max_limit
        } else if let Some(lim) = limit {
            lim.min(max_limit)
        } else {
            default_limit
        };

        let (base_url, endpoint, use_pair) = self.get_ohlcv_endpoint(&market, price.as_deref())?;

        let symbol_param = if use_pair {
            market.symbol.replace('/', "")
        } else {
            market.id.clone()
        };

        let mut url = format!(
            "{}{}?symbol={}&interval={}&limit={}",
            base_url, endpoint, symbol_param, timeframe, adjusted_limit
        );

        if let Some(start_time) = since {
            url.push_str(&format!("&startTime={}", start_time));

            // Calculate endTime for inverse markets (ref: Go implementation)
            if market.inverse.unwrap_or(false) && start_time > 0 && until.is_none() {
                let duration = self.parse_timeframe(timeframe)?;
                let calculated_end_time =
                    start_time + (adjusted_limit as i64 * duration * 1000) - 1;
                // SAFETY: SystemTime::now() is always after UNIX_EPOCH on any modern system.
                // This can only fail if the system clock is set before 1970, which is not
                // a supported configuration for this library.
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .expect("System clock is set before UNIX_EPOCH (1970); this is not supported")
                    .as_millis() as i64;
                let end_time = calculated_end_time.min(now);
                url.push_str(&format!("&endTime={}", end_time));
            }
        }

        if let Some(end_time) = until {
            url.push_str(&format!("&endTime={}", end_time));
        }

        let data = self.base().http_client.get(&url, None).await?;

        parser::parse_ohlcvs(&data)
    }

    /// Fetch trading fee for a single trading pair
    ///
    /// # Arguments
    /// * `symbol` - Trading pair symbol (e.g., BTC/USDT)
    ///
    /// # Binance API
    /// - Endpoint: GET /sapi/v1/asset/tradeFee
    /// - Weight: 1
    /// - Requires signature: Yes
    ///
    /// # Examples
    /// ```rust,no_run
    /// # use ccxt_exchanges::binance::Binance;
    /// # use ccxt_core::ExchangeConfig;
    /// # use rust_decimal::prelude::*;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let config = ExchangeConfig {
    ///     api_key: Some("your-api-key".to_string()),
    ///     secret: Some("your-secret".to_string()),
    ///     ..Default::default()
    /// };
    /// let binance = Binance::new(config)?;
    ///
    /// // Fetch trading fee for BTC/USDT
    /// let fee = binance.fetch_trading_fee("BTC/USDT", None).await?;
    /// let multiplier = Decimal::from_f64(100.0).unwrap();
    /// println!("Maker fee: {}%, Taker fee: {}%",
    ///     fee.maker * multiplier, fee.taker * multiplier);
    /// # Ok(())
    /// # }
    /// ```
    // pub async fn fetch_trading_fee(&self, symbol: &str) -> Result<ccxt_core::types::TradingFee> {
    //     self.base().load_markets(false).await?;
    //
    //     let market = self.base().market(symbol).await?;
    //     let mut params = std::collections::HashMap::new();
    //     params.insert("symbol".to_string(), market.id.clone());
    //
    //     // Sign parameters
    //     let auth = crate::binance::auth::BinanceAuth::new(
    //         self.base().config.api_key.as_ref().ok_or_else(|| {
    //             ccxt_core::error::Error::authentication("API key required")
    //         })?,
    //         self.base().config.secret.as_ref().ok_or_else(|| {
    //             ccxt_core::error::Error::authentication("API secret required")
    //         })?,
    //     );
    //
    //     let signed_params = auth.sign_params(&params)?;
    //
    //     // Build URL
    //     let query_string: Vec<String> = signed_params
    //         .iter()
    //         .map(|(k, v)| format!("{}={}", k, v))
    //         .collect();
    //     let url = format!("{}/asset/tradeFee?{}", self.urls().sapi, query_string.join("&"));
    //
    //     // Add API key to headers
    //     let mut headers = reqwest::header::HeaderMap::new();
    //     auth.add_auth_headers_reqwest(&mut headers);
    //
    //     let data = self.base().http_client.get(&url, Some(headers)).await?;
    //
    //     // Parse response array and take first element
    //     let fees = parser::parse_trading_fees(&data)?;
    //     fees.into_iter().next().ok_or_else(|| {
    //         ccxt_core::error::Error::invalid_request("No trading fee data returned")
    //     })
    // }

    /// Fetch trading fees for multiple trading pairs
    ///
    /// # Arguments
    /// * `symbols` - Trading pair symbols (e.g., vec!["BTC/USDT", "ETH/USDT"]), or None for all pairs
    ///
    /// # Binance API
    /// - Endpoint: GET /sapi/v1/asset/tradeFee
    /// - Weight: 1
    /// - Signature required: Yes
    ///
    /// # Examples
    /// ```rust,no_run
    /// # use ccxt_exchanges::binance::Binance;
    /// # use ccxt_core::ExchangeConfig;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let config = ExchangeConfig {
    ///     api_key: Some("your-api-key".to_string()),
    ///     secret: Some("your-secret".to_string()),
    ///     ..Default::default()
    /// };
    /// let binance = Binance::new(config)?;
    ///
    /// // Fetch fees for all trading pairs
    /// let all_fees = binance.fetch_trading_fees(None, None).await?;
    /// println!("Total symbols with fees: {}", all_fees.len());
    ///
    /// // Fetch fees for specific trading pairs
    /// let fees = binance.fetch_trading_fees(Some(vec!["BTC/USDT".to_string(), "ETH/USDT".to_string()]), None).await?;
    /// # use rust_decimal::Decimal;
    /// for (symbol, fee) in &fees {
    ///     println!("{}: Maker {}%, Taker {}%",
    ///         symbol, fee.maker * Decimal::from(100), fee.taker * Decimal::from(100));
    /// }
    /// # Ok(())
    /// # }
    /// ```
    /// Fetch server time
    ///
    /// # Binance API
    /// - Endpoint: GET /api/v3/time
    /// - Weight: 1
    /// - Data source: Memory
    ///
    /// # Examples
    /// ```rust,no_run
    /// # use ccxt_exchanges::binance::Binance;
    /// # use ccxt_core::ExchangeConfig;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let config = ExchangeConfig::default();
    /// let binance = Binance::new(config)?;
    ///
    /// // Fetch Binance server time
    /// let server_time = binance.fetch_time().await?;
    /// println!("Server time: {} ({})", server_time.server_time, server_time.datetime);
    ///
    /// // Check local time offset
    /// let local_time = chrono::Utc::now().timestamp_millis();
    /// let offset = server_time.server_time - local_time;
    /// println!("Time offset: {} ms", offset);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn fetch_time(&self) -> Result<ccxt_core::types::ServerTime> {
        let url = format!("{}/time", self.urls().public);

        let data = self.base().http_client.get(&url, None).await?;

        parser::parse_server_time(&data)
    }

    /// Fetch canceled orders
    ///
    /// # Arguments
    ///
    /// * `symbol` - Trading pair symbol (e.g., "BTC/USDT")
    /// * `since` - Optional start timestamp in milliseconds
    /// * `limit` - Optional record limit (default 500, max 1000)
    ///
    /// # Returns
    ///
    /// Returns vector of canceled orders
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use ccxt_exchanges::binance::Binance;
    /// # use ccxt_core::ExchangeConfig;
    /// # async fn example() -> ccxt_core::Result<()> {
    /// let binance = Binance::new(ExchangeConfig::default())?;
    /// let orders = binance.fetch_canceled_orders("BTC/USDT", None, Some(10)).await?;
    /// println!("Found {} canceled orders", orders.len());
    /// # Ok(())
    /// # }
    /// ```
    /// Fetch canceled and closed orders
    ///
    /// # Arguments
    ///
    /// * `symbol` - Trading pair symbol (e.g., "BTC/USDT")
    /// * `since` - Optional start timestamp in milliseconds
    /// * `limit` - Optional record limit (default 500, max 1000)
    ///
    /// # Returns
    ///
    /// Returns vector of canceled and closed orders
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use ccxt_exchanges::binance::Binance;
    /// # use ccxt_core::ExchangeConfig;
    /// # async fn example() -> ccxt_core::Result<()> {
    /// let binance = Binance::new(ExchangeConfig::default())?;
    /// let orders = binance.fetch_canceled_and_closed_orders("BTC/USDT", None, Some(20)).await?;
    /// println!("Found {} closed/canceled orders", orders.len());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn fetch_canceled_and_closed_orders(
        &self,
        symbol: &str,
        since: Option<u64>,
        limit: Option<u64>,
    ) -> Result<Vec<Order>> {
        self.check_required_credentials()?;

        let market = self.base().market(symbol).await?;
        let market_id = &market.id;

        let mut params = HashMap::new();
        params.insert("symbol".to_string(), market_id.to_string());

        if let Some(ts) = since {
            params.insert("startTime".to_string(), ts.to_string());
        }

        if let Some(l) = limit {
            params.insert("limit".to_string(), l.to_string());
        }

        let timestamp = self.fetch_time_raw().await?;
        let auth = self.get_auth()?;
        let signed_params =
            auth.sign_with_timestamp(&params, timestamp, Some(self.options().recv_window))?;

        let mut url = format!("{}/api/v3/allOrders?", self.urls().private);
        for (key, value) in &signed_params {
            url.push_str(&format!("{}={}&", key, value));
        }

        let mut headers = HeaderMap::new();
        auth.add_auth_headers_reqwest(&mut headers);

        let data = self.base().http_client.get(&url, Some(headers)).await?;

        let all_orders = if let Some(array) = data.as_array() {
            array
                .iter()
                .filter_map(|item| parser::parse_order(item, None).ok())
                .collect::<Vec<_>>()
        } else {
            vec![]
        };

        Ok(all_orders
            .into_iter()
            .filter(|order| {
                order.status == OrderStatus::Cancelled || order.status == OrderStatus::Closed
            })
            .collect())
    }

    /// Fetch a single open order
    ///
    /// # Arguments
    ///
    /// * `id` - Order ID
    /// * `symbol` - Trading pair symbol (e.g., "BTC/USDT")
    ///
    /// # Returns
    ///
    /// Returns order information
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use ccxt_exchanges::binance::Binance;
    /// # use ccxt_core::ExchangeConfig;
    /// # async fn example() -> ccxt_core::Result<()> {
    /// let binance = Binance::new(ExchangeConfig::default())?;
    /// let order = binance.fetch_open_order("12345", "BTC/USDT").await?;
    /// println!("Order status: {:?}", order.status);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn fetch_open_order(&self, id: &str, symbol: &str) -> Result<Order> {
        self.check_required_credentials()?;

        let market = self.base().market(symbol).await?;
        let market_id = &market.id;

        let mut params = HashMap::new();
        params.insert("symbol".to_string(), market_id.to_string());
        params.insert("orderId".to_string(), id.to_string());

        let timestamp = self.fetch_time_raw().await?;
        let auth = self.get_auth()?;
        let signed_params =
            auth.sign_with_timestamp(&params, timestamp, Some(self.options().recv_window))?;

        let mut url = format!("{}/api/v3/order?", self.urls().private);
        for (key, value) in &signed_params {
            url.push_str(&format!("{}={}&", key, value));
        }

        let mut headers = HeaderMap::new();
        auth.add_auth_headers_reqwest(&mut headers);

        let data = self.base().http_client.get(&url, Some(headers)).await?;

        parser::parse_order(&data, None)
    }

    /// Fetch trades for a specific order
    ///
    /// # Arguments
    ///
    /// * `id` - Order ID
    /// * `symbol` - Trading pair symbol (e.g., "BTC/USDT")
    /// * `since` - Optional start timestamp in milliseconds
    /// * `limit` - Optional record limit (default 500, max 1000)
    ///
    /// # Returns
    ///
    /// Returns vector of trade records
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use ccxt_exchanges::binance::Binance;
    /// # use ccxt_core::ExchangeConfig;
    /// # async fn example() -> ccxt_core::Result<()> {
    /// let binance = Binance::new(ExchangeConfig::default())?;
    /// let trades = binance.fetch_order_trades("12345", "BTC/USDT", None, Some(50)).await?;
    /// println!("Order has {} trades", trades.len());
    /// # Ok(())
    /// # }
    /// ```
    /// Edit order (cancel and replace)
    ///
    /// # Arguments
    ///
    /// * `id` - Original order ID
    /// * `symbol` - Trading pair symbol (e.g., "BTC/USDT")
    /// * `order_type` - Order type
    /// * `side` - Order side (Buy/Sell)
    /// * `amount` - Order quantity
    /// * `price` - Optional order price (required for limit orders)
    /// * `params` - Optional additional parameters
    ///
    /// # Returns
    ///
    /// Returns new order information
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use ccxt_exchanges::binance::Binance;
    /// # use ccxt_core::{ExchangeConfig, types::{OrderType, OrderSide}};
    /// # use rust_decimal_macros::dec;
    /// # async fn example() -> ccxt_core::Result<()> {
    /// let binance = Binance::new(ExchangeConfig::default())?;
    /// let order = binance.edit_order(
    ///     "12345",
    ///     "BTC/USDT",
    ///     OrderType::Limit,
    ///     OrderSide::Buy,
    ///     dec!(0.01),
    ///     Some(dec!(50000.0)),
    ///     None
    /// ).await?;
    /// println!("New order ID: {}", order.id);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn edit_order(
        &self,
        id: &str,
        symbol: &str,
        order_type: OrderType,
        side: OrderSide,
        amount: rust_decimal::Decimal,
        price: Option<rust_decimal::Decimal>,
        params: Option<HashMap<String, String>>,
    ) -> Result<Order> {
        self.check_required_credentials()?;

        let market = self.base().market(symbol).await?;
        let market_id = &market.id;

        let mut request_params = params.unwrap_or_default();

        request_params.insert("symbol".to_string(), market_id.to_string());
        request_params.insert("cancelOrderId".to_string(), id.to_string());
        request_params.insert("side".to_string(), side.to_string().to_uppercase());
        request_params.insert("type".to_string(), order_type.to_string().to_uppercase());
        request_params.insert("quantity".to_string(), amount.to_string());

        if let Some(p) = price {
            request_params.insert("price".to_string(), p.to_string());
        }

        if !request_params.contains_key("cancelReplaceMode") {
            request_params.insert(
                "cancelReplaceMode".to_string(),
                "STOP_ON_FAILURE".to_string(),
            );
        }

        let timestamp = self.fetch_time_raw().await?;
        let auth = self.get_auth()?;
        let signed_params =
            auth.sign_with_timestamp(&request_params, timestamp, Some(self.options().recv_window))?;

        let mut url = format!("{}/api/v3/order/cancelReplace?", self.urls().private);
        for (key, value) in &signed_params {
            url.push_str(&format!("{}={}&", key, value));
        }

        let mut headers = HeaderMap::new();
        auth.add_auth_headers_reqwest(&mut headers);

        let data = self
            .base()
            .http_client
            .post(&url, Some(headers), None)
            .await?;

        parser::parse_edit_order_result(&data, None)
    }

    /// Edit multiple orders in batch
    ///
    /// # Arguments
    ///
    /// * `orders` - Vector of order edit parameters, each element contains:
    ///   - id: Original order ID
    ///   - symbol: Trading pair symbol
    ///   - order_type: Order type
    ///   - side: Order side
    ///   - amount: Order quantity
    ///   - price: Optional order price
    ///
    /// # Returns
    ///
    /// Returns vector of new order information
    ///
    /// # Note
    ///
    /// Binance does not support native batch order editing API. This method implements batch editing by concurrent calls to single order edits.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use ccxt_exchanges::binance::Binance;
    /// # use ccxt_core::{ExchangeConfig, types::{OrderType, OrderSide}};
    /// # use rust_decimal_macros::dec;
    /// # async fn example() -> ccxt_core::Result<()> {
    /// let binance = Binance::new(ExchangeConfig::default())?;
    /// let order_params = vec![
    ///     ("12345", "BTC/USDT", OrderType::Limit, OrderSide::Buy, dec!(0.01), Some(dec!(50000.0))),
    ///     ("12346", "ETH/USDT", OrderType::Limit, OrderSide::Sell, dec!(0.1), Some(dec!(3000.0))),
    /// ];
    /// let orders = binance.edit_orders(order_params).await?;
    /// println!("Edited {} orders", orders.len());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn edit_orders(
        &self,
        orders: Vec<(
            &str,                          // id
            &str,                          // symbol
            OrderType,                     // order_type
            OrderSide,                     // side
            rust_decimal::Decimal,         // amount
            Option<rust_decimal::Decimal>, // price
        )>,
    ) -> Result<Vec<Order>> {
        let futures: Vec<_> = orders
            .into_iter()
            .map(|(id, symbol, order_type, side, amount, price)| {
                self.edit_order(id, symbol, order_type, side, amount, price, None)
            })
            .collect();

        let results = futures::future::join_all(futures).await;

        results.into_iter().collect()
    }

    // ==================== WebSocket User Data Stream (Listen Key Management) ====================

    /// Create listen key for user data stream
    ///
    /// Creates a listen key with 60-minute validity for subscribing to WebSocket user data stream.
    /// The listen key must be refreshed periodically to keep the connection active.
    ///
    /// # Returns
    ///
    /// Returns the listen key string
    ///
    /// # Errors
    ///
    /// - If API credentials are not configured
    /// - If API request fails
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use ccxt_exchanges::binance::Binance;
    /// # use ccxt_core::ExchangeConfig;
    /// # async fn example() -> ccxt_core::Result<()> {
    /// let mut config = ExchangeConfig::default();
    /// config.api_key = Some("your_api_key".to_string());
    /// config.secret = Some("your_secret".to_string());
    /// let binance = Binance::new(config)?;
    ///
    /// let listen_key = binance.create_listen_key().await?;
    /// println!("Listen Key: {}", listen_key);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn create_listen_key(&self) -> Result<String> {
        self.check_required_credentials()?;

        let url = format!("{}/userDataStream", self.urls().public);
        let mut headers = HeaderMap::new();

        let auth = self.get_auth()?;
        auth.add_auth_headers_reqwest(&mut headers);

        let response = self
            .base()
            .http_client
            .post(&url, Some(headers), None)
            .await?;

        response["listenKey"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| Error::from(ParseError::missing_field("listenKey")))
    }

    /// Refresh listen key to extend validity
    ///
    /// Extends the listen key validity by 60 minutes. Recommended to call every 30 minutes to maintain the connection.
    ///
    /// # Arguments
    ///
    /// * `listen_key` - The listen key to refresh
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on success
    ///
    /// # Errors
    ///
    /// - If API credentials are not configured
    /// - If listen key is invalid or expired
    /// - If API request fails
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use ccxt_exchanges::binance::Binance;
    /// # use ccxt_core::ExchangeConfig;
    /// # async fn example() -> ccxt_core::Result<()> {
    /// # let mut config = ExchangeConfig::default();
    /// # config.api_key = Some("your_api_key".to_string());
    /// # config.secret = Some("your_secret".to_string());
    /// # let binance = Binance::new(config)?;
    /// let listen_key = binance.create_listen_key().await?;
    ///
    /// // Refresh after 30 minutes
    /// binance.refresh_listen_key(&listen_key).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn refresh_listen_key(&self, listen_key: &str) -> Result<()> {
        self.check_required_credentials()?;

        let url = format!(
            "{}/userDataStream?listenKey={}",
            self.urls().public,
            listen_key
        );
        let mut headers = HeaderMap::new();

        let auth = self.get_auth()?;
        auth.add_auth_headers_reqwest(&mut headers);

        let _response = self
            .base()
            .http_client
            .put(&url, Some(headers), None)
            .await?;

        Ok(())
    }

    /// Delete listen key to close user data stream
    ///
    /// Closes the user data stream connection and invalidates the listen key.
    ///
    /// # Arguments
    ///
    /// * `listen_key` - The listen key to delete
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on success
    ///
    /// # Errors
    ///
    /// - If API credentials are not configured
    /// - If API request fails
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use ccxt_exchanges::binance::Binance;
    /// # use ccxt_core::ExchangeConfig;
    /// # async fn example() -> ccxt_core::Result<()> {
    /// # let mut config = ExchangeConfig::default();
    /// # config.api_key = Some("your_api_key".to_string());
    /// # config.secret = Some("your_secret".to_string());
    /// # let binance = Binance::new(config)?;
    /// let listen_key = binance.create_listen_key().await?;
    ///
    /// // Delete after use
    /// binance.delete_listen_key(&listen_key).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn delete_listen_key(&self, listen_key: &str) -> Result<()> {
        self.check_required_credentials()?;

        let url = format!(
            "{}/userDataStream?listenKey={}",
            self.urls().public,
            listen_key
        );
        let mut headers = HeaderMap::new();

        let auth = self.get_auth()?;
        auth.add_auth_headers_reqwest(&mut headers);

        let _response = self
            .base()
            .http_client
            .delete(&url, Some(headers), None)
            .await?;

        Ok(())
    }
    /// Create orders in batch (up to 5 orders)
    ///
    /// Creates multiple orders in a single API request to improve order placement efficiency.
    ///
    /// # Arguments
    /// - `orders`: List of order requests (maximum 5)
    /// - `params`: Optional parameters
    ///
    /// # Returns
    /// Returns list of batch order results
    ///
    /// # Examples
    /// ```rust,no_run
    /// # use ccxt_exchanges::binance::Binance;
    /// # use ccxt_core::ExchangeConfig;
    /// # use ccxt_core::types::order::BatchOrderRequest;
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let binance = Binance::new_swap(ExchangeConfig::default())?;
    ///
    /// // Batch create orders
    /// let orders = vec![
    ///     BatchOrderRequest {
    ///         symbol: "BTCUSDT".to_string(),
    ///         side: "BUY".to_string(),
    ///         order_type: "LIMIT".to_string(),
    ///         quantity: "0.001".to_string(),
    ///         price: Some("40000".to_string()),
    ///         time_in_force: Some("GTC".to_string()),
    ///         reduce_only: None,
    ///         position_side: Some("LONG".to_string()),
    ///         new_client_order_id: None,
    ///     },
    ///     BatchOrderRequest {
    ///         symbol: "ETHUSDT".to_string(),
    ///         side: "SELL".to_string(),
    ///         order_type: "LIMIT".to_string(),
    ///         quantity: "0.01".to_string(),
    ///         price: Some("3000".to_string()),
    ///         time_in_force: Some("GTC".to_string()),
    ///         reduce_only: None,
    ///         position_side: Some("SHORT".to_string()),
    ///         new_client_order_id: None,
    ///     },
    /// ];
    /// let results = binance.create_orders(orders, None).await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # API Endpoint
    /// - Endpoint: POST /fapi/v1/batchOrders
    /// - Weight: 5
    /// - Requires signature: Yes
    pub async fn create_orders(
        &self,
        orders: Vec<BatchOrderRequest>,
        params: Option<Value>,
    ) -> Result<Vec<BatchOrderResult>> {
        self.check_required_credentials()?;

        if orders.is_empty() {
            return Err(Error::invalid_request("Orders list cannot be empty"));
        }

        if orders.len() > 5 {
            return Err(Error::invalid_request(
                "Cannot create more than 5 orders at once",
            ));
        }

        let batch_orders_json = serde_json::to_string(&orders).map_err(|e| {
            Error::from(ParseError::invalid_format(
                "data",
                format!("Failed to serialize orders: {}", e),
            ))
        })?;

        let mut request_params = HashMap::new();
        request_params.insert("batchOrders".to_string(), batch_orders_json);

        if let Some(params) = params {
            if let Some(obj) = params.as_object() {
                for (key, value) in obj {
                    if let Some(v) = value.as_str() {
                        request_params.insert(key.clone(), v.to_string());
                    }
                }
            }
        }

        let timestamp = self.fetch_time_raw().await?;
        let auth = self.get_auth()?;
        let signed_params =
            auth.sign_with_timestamp(&request_params, timestamp, Some(self.options().recv_window))?;

        let url = format!("{}/batchOrders", self.urls().fapi_private);

        let mut headers = HeaderMap::new();
        auth.add_auth_headers_reqwest(&mut headers);

        let body = serde_json::to_value(&signed_params).map_err(|e| {
            Error::from(ParseError::invalid_format(
                "data",
                format!("Failed to serialize params: {}", e),
            ))
        })?;

        let data = self
            .base()
            .http_client
            .post(&url, Some(headers), Some(body))
            .await?;

        let results: Vec<BatchOrderResult> = serde_json::from_value(data).map_err(|e| {
            Error::from(ParseError::invalid_format(
                "data",
                format!("Failed to parse batch order results: {}", e),
            ))
        })?;

        Ok(results)
    }

    /// Modify multiple orders in batch
    ///
    /// Modifies the price and quantity of multiple orders in a single API request.
    ///
    /// # Arguments
    /// - `updates`: List of order update requests (maximum 5)
    /// - `params`: Optional parameters
    ///
    /// # Returns
    /// Returns list of batch order results
    ///
    /// # Examples
    /// ```rust,no_run
    /// # use ccxt_exchanges::binance::Binance;
    /// # use ccxt_core::ExchangeConfig;
    /// # use ccxt_core::types::order::BatchOrderUpdate;
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let binance = Binance::new_swap(ExchangeConfig::default())?;
    ///
    /// // Batch modify orders
    /// let updates = vec![
    ///     BatchOrderUpdate {
    ///         order_id: Some(12345),
    ///         orig_client_order_id: None,
    ///         symbol: "BTCUSDT".to_string(),
    ///         side: "BUY".to_string(),
    ///         quantity: "0.002".to_string(),
    ///         price: "41000".to_string(),
    ///     },
    /// ];
    /// let results = binance.batch_edit_orders(updates, None).await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # API Endpoint
    /// - Endpoint: PUT /fapi/v1/batchOrders
    /// - Weight: 5
    /// - Requires signature: Yes
    pub async fn batch_edit_orders(
        &self,
        updates: Vec<BatchOrderUpdate>,
        params: Option<Value>,
    ) -> Result<Vec<BatchOrderResult>> {
        self.check_required_credentials()?;

        if updates.is_empty() {
            return Err(Error::invalid_request("Updates list cannot be empty"));
        }

        if updates.len() > 5 {
            return Err(Error::invalid_request(
                "Cannot update more than 5 orders at once",
            ));
        }

        let batch_orders_json = serde_json::to_string(&updates).map_err(|e| {
            Error::from(ParseError::invalid_format(
                "data",
                format!("Failed to serialize updates: {}", e),
            ))
        })?;

        let mut request_params = HashMap::new();
        request_params.insert("batchOrders".to_string(), batch_orders_json);

        if let Some(params) = params {
            if let Some(obj) = params.as_object() {
                for (key, value) in obj {
                    if let Some(v) = value.as_str() {
                        request_params.insert(key.clone(), v.to_string());
                    }
                }
            }
        }

        let timestamp = self.fetch_time_raw().await?;
        let auth = self.get_auth()?;
        let signed_params =
            auth.sign_with_timestamp(&request_params, timestamp, Some(self.options().recv_window))?;

        let url = format!("{}/batchOrders", self.urls().fapi_private);

        let mut headers = HeaderMap::new();
        auth.add_auth_headers_reqwest(&mut headers);

        let body = serde_json::to_value(&signed_params).map_err(|e| {
            Error::from(ParseError::invalid_format(
                "data",
                format!("Failed to serialize params: {}", e),
            ))
        })?;

        let data = self
            .base()
            .http_client
            .put(&url, Some(headers), Some(body))
            .await?;

        let results: Vec<BatchOrderResult> = serde_json::from_value(data).map_err(|e| {
            Error::from(ParseError::invalid_format(
                "data",
                format!("Failed to parse batch order results: {}", e),
            ))
        })?;

        Ok(results)
    }

    /// Cancel orders in batch by order IDs
    ///
    /// Cancels multiple orders in a single API request.
    ///
    /// # Arguments
    /// - `symbol`: Trading pair symbol
    /// - `order_ids`: List of order IDs (maximum 10)
    /// - `params`: Optional parameters
    ///   - `origClientOrderIdList`: List of original client order IDs (alternative to order_ids)
    ///
    /// # Returns
    /// Returns list of batch cancel results
    ///
    /// # Examples
    /// ```rust,no_run
    /// # use ccxt_exchanges::binance::Binance;
    /// # use ccxt_core::ExchangeConfig;
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let binance = Binance::new_swap(ExchangeConfig::default())?;
    ///
    /// // Batch cancel orders
    /// let order_ids = vec![12345, 12346, 12347];
    /// let results = binance.batch_cancel_orders("BTC/USDT", order_ids, None).await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # API Endpoint
    /// - Endpoint: DELETE /fapi/v1/batchOrders
    /// - Weight: 1
    /// - Requires signature: Yes
    pub async fn batch_cancel_orders(
        &self,
        symbol: &str,
        order_ids: Vec<i64>,
        params: Option<Value>,
    ) -> Result<Vec<BatchCancelResult>> {
        self.check_required_credentials()?;

        if order_ids.is_empty() {
            return Err(Error::invalid_request("Order IDs list cannot be empty"));
        }

        if order_ids.len() > 10 {
            return Err(Error::invalid_request(
                "Cannot cancel more than 10 orders at once",
            ));
        }

        let market = self.base().market(symbol).await?;

        let mut request_params = HashMap::new();
        request_params.insert("symbol".to_string(), market.id.clone());

        let order_id_list_json = serde_json::to_string(&order_ids).map_err(|e| {
            Error::from(ParseError::invalid_format(
                "data",
                format!("Failed to serialize order IDs: {}", e),
            ))
        })?;
        request_params.insert("orderIdList".to_string(), order_id_list_json);

        if let Some(params) = params {
            if let Some(obj) = params.as_object() {
                for (key, value) in obj {
                    if key == "origClientOrderIdList" {
                        if let Some(v) = value.as_str() {
                            request_params.insert(key.clone(), v.to_string());
                        }
                    }
                }
            }
        }

        let timestamp = self.fetch_time_raw().await?;
        let auth = self.get_auth()?;
        let signed_params =
            auth.sign_with_timestamp(&request_params, timestamp, Some(self.options().recv_window))?;

        let url = if market.linear.unwrap_or(true) {
            format!("{}/batchOrders", self.urls().fapi_private)
        } else {
            format!("{}/batchOrders", self.urls().dapi_private)
        };

        let mut headers = HeaderMap::new();
        auth.add_auth_headers_reqwest(&mut headers);

        let body = serde_json::to_value(&signed_params).map_err(|e| {
            Error::from(ParseError::invalid_format(
                "data",
                format!("Failed to serialize params: {}", e),
            ))
        })?;

        let data = self
            .base()
            .http_client
            .delete(&url, Some(headers), Some(body))
            .await?;

        let results: Vec<BatchCancelResult> = serde_json::from_value(data).map_err(|e| {
            Error::from(ParseError::invalid_format(
                "data",
                format!("Failed to parse batch cancel results: {}", e),
            ))
        })?;

        Ok(results)
    }

    /// Cancel all open orders for a trading pair
    ///
    /// Cancels all open orders for the specified trading pair.
    ///
    /// # Arguments
    /// - `symbol`: Trading pair symbol
    /// - `params`: Optional parameters
    ///
    /// # Returns
    /// Returns result of canceling all orders
    ///
    /// # Examples
    /// ```rust,no_run
    /// # use ccxt_exchanges::binance::Binance;
    /// # use ccxt_core::ExchangeConfig;
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let binance = Binance::new(ExchangeConfig::default())?;
    ///
    /// // Cancel all open orders for BTC/USDT
    /// let result = binance.cancel_all_orders("BTC/USDT").await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # API Endpoint
    /// - Endpoint: DELETE /fapi/v1/allOpenOrders
    /// - Weight: 1
    /// - Requires signature: Yes
    pub async fn cancel_all_orders_futures(
        &self,
        symbol: &str,
        params: Option<Value>,
    ) -> Result<CancelAllOrdersResult> {
        self.check_required_credentials()?;

        let market = self.base().market(symbol).await?;

        let mut request_params = HashMap::new();
        request_params.insert("symbol".to_string(), market.id.clone());

        if let Some(params) = params {
            if let Some(obj) = params.as_object() {
                for (key, value) in obj {
                    if let Some(v) = value.as_str() {
                        request_params.insert(key.clone(), v.to_string());
                    }
                }
            }
        }

        let timestamp = self.fetch_time_raw().await?;
        let auth = self.get_auth()?;
        let signed_params =
            auth.sign_with_timestamp(&request_params, timestamp, Some(self.options().recv_window))?;

        let url = if market.linear.unwrap_or(true) {
            format!("{}/allOpenOrders", self.urls().fapi_private)
        } else {
            format!("{}/allOpenOrders", self.urls().dapi_private)
        };

        let mut headers = HeaderMap::new();
        auth.add_auth_headers_reqwest(&mut headers);

        let body = serde_json::to_value(&signed_params).map_err(|e| {
            Error::from(ParseError::invalid_format(
                "data",
                format!("Failed to serialize params: {}", e),
            ))
        })?;

        let data = self
            .base()
            .http_client
            .delete(&url, Some(headers), Some(body))
            .await?;

        let result: CancelAllOrdersResult = serde_json::from_value(data).map_err(|e| {
            Error::from(ParseError::invalid_format(
                "data",
                format!("Failed to parse cancel all orders result: {}", e),
            ))
        })?;

        Ok(result)
    }
    // ============================================================================
    // Account Configuration Management
    // ============================================================================

    /// Fetch account configuration information
    ///
    /// Retrieves futures account configuration parameters including multi-asset mode status and fee tier.
    ///
    /// # Returns
    ///
    /// Returns account configuration information
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use ccxt_exchanges::binance::Binance;
    /// # use ccxt_core::ExchangeConfig;
    /// # async fn example() -> ccxt_core::Result<()> {
    /// let mut config = ExchangeConfig::default();
    /// config.api_key = Some("your_api_key".to_string());
    /// config.secret = Some("your_secret".to_string());
    /// let binance = Binance::new_swap(config)?;
    /// let account_config = binance.fetch_account_configuration().await?;
    /// println!("Multi-asset mode: {}", account_config.multi_assets_margin);
    /// println!("Fee tier: {}", account_config.fee_tier);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn fetch_account_configuration(&self) -> Result<AccountConfig> {
        self.check_required_credentials()?;

        let request_params = HashMap::new();

        let timestamp = self.fetch_time_raw().await?;
        let auth = self.get_auth()?;
        let signed_params =
            auth.sign_with_timestamp(&request_params, timestamp, Some(self.options().recv_window))?;

        let mut request_url = format!("{}/account?", self.urls().fapi_private);
        for (key, value) in &signed_params {
            request_url.push_str(&format!("{}={}&", key, value));
        }

        let mut headers = HeaderMap::new();
        auth.add_auth_headers_reqwest(&mut headers);

        let data = self
            .base()
            .http_client
            .get(&request_url, Some(headers))
            .await?;

        parser::parse_account_config(&data)
    }

    /// Set multi-asset mode
    ///
    /// Enable or disable multi-asset margin mode.
    ///
    /// # Arguments
    ///
    /// * `multi_assets` - `true` to enable multi-asset mode, `false` for single-asset mode
    ///
    /// # Returns
    ///
    /// Returns operation result
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use ccxt_exchanges::binance::Binance;
    /// # use ccxt_core::ExchangeConfig;
    /// # async fn example() -> ccxt_core::Result<()> {
    /// let mut config = ExchangeConfig::default();
    /// config.api_key = Some("your_api_key".to_string());
    /// config.secret = Some("your_secret".to_string());
    /// let binance = Binance::new_swap(config)?;
    /// binance.set_multi_assets_mode(true).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn set_multi_assets_mode(&self, multi_assets: bool) -> Result<()> {
        self.check_required_credentials()?;

        let mut request_params = HashMap::new();
        request_params.insert("multiAssetsMargin".to_string(), multi_assets.to_string());

        let timestamp = self.fetch_time_raw().await?;
        let auth = self.get_auth()?;
        let signed_params =
            auth.sign_with_timestamp(&request_params, timestamp, Some(self.options().recv_window))?;

        let url = format!("{}/multiAssetsMargin", self.urls().fapi_private);

        let mut headers = HeaderMap::new();
        auth.add_auth_headers_reqwest(&mut headers);

        let body = serde_json::to_value(&signed_params).map_err(|e| {
            Error::from(ParseError::invalid_format(
                "data",
                format!("Failed to serialize params: {}", e),
            ))
        })?;

        let _data = self
            .base()
            .http_client
            .post(&url, Some(headers), Some(body))
            .await?;

        Ok(())
    }

    /// Fetch futures commission rate
    ///
    /// Retrieves maker and taker commission rates for the specified trading pair.
    ///
    /// # Arguments
    ///
    /// * `symbol` - Trading pair symbol
    ///
    /// # Returns
    ///
    /// Returns commission rate information
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use ccxt_exchanges::binance::Binance;
    /// # use ccxt_core::ExchangeConfig;
    /// # async fn example() -> ccxt_core::Result<()> {
    /// let mut config = ExchangeConfig::default();
    /// config.api_key = Some("your_api_key".to_string());
    /// config.secret = Some("your_secret".to_string());
    /// let binance = Binance::new_swap(config)?;
    /// let rate = binance.fetch_commission_rate("BTC/USDT:USDT").await?;
    /// println!("Maker rate: {}", rate.maker_commission_rate);
    /// println!("Taker rate: {}", rate.taker_commission_rate);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn fetch_commission_rate(&self, symbol: &str) -> Result<CommissionRate> {
        self.check_required_credentials()?;

        let market = self.base().market(symbol).await?;
        let mut request_params = HashMap::new();
        request_params.insert("symbol".to_string(), market.id.clone());

        let timestamp = self.fetch_time_raw().await?;
        let auth = self.get_auth()?;
        let signed_params =
            auth.sign_with_timestamp(&request_params, timestamp, Some(self.options().recv_window))?;

        let mut request_url = format!("{}/commissionRate?", self.urls().fapi_private);
        for (key, value) in &signed_params {
            request_url.push_str(&format!("{}={}&", key, value));
        }

        let mut headers = HeaderMap::new();
        auth.add_auth_headers_reqwest(&mut headers);

        let data = self
            .base()
            .http_client
            .get(&request_url, Some(headers))
            .await?;

        parser::parse_commission_rate(&data, &market)
    }
    // ==================== Stage 24.6: Risk and Limit Queries ====================

    /// Fetch futures open interest
    ///
    /// Retrieves the total amount of outstanding (open) contracts.
    ///
    /// # Arguments
    ///
    /// * `symbol` - Trading pair symbol, e.g. "BTC/USDT:USDT"
    ///
    /// # Returns
    ///
    /// Returns open interest information
    ///
    /// # API Endpoint
    ///
    /// GET /fapi/v1/openInterest (MARKET_DATA)
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use ccxt_exchanges::binance::Binance;
    /// # use ccxt_core::ExchangeConfig;
    /// # async fn example() -> ccxt_core::Result<()> {
    /// let binance = Binance::new_swap(ExchangeConfig::default())?;
    /// let open_interest = binance.fetch_open_interest("BTC/USDT:USDT").await?;
    /// println!("Open interest: {}", open_interest.open_interest);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn fetch_open_interest(&self, symbol: &str) -> Result<OpenInterest> {
        let market = self.base().market(symbol).await?;
        let mut request_params = HashMap::new();
        request_params.insert("symbol".to_string(), market.id.clone());

        let mut request_url = format!("{}/openInterest?", self.urls().fapi_public);
        for (key, value) in &request_params {
            request_url.push_str(&format!("{}={}&", key, value));
        }

        let data = self.base().http_client.get(&request_url, None).await?;

        parser::parse_open_interest(&data, &market)
    }

    /// Fetch open interest history
    ///
    /// # Arguments
    ///
    /// * `symbol` - Trading pair symbol, e.g. "BTC/USDT:USDT"
    /// * `period` - Time period: "5m", "15m", "30m", "1h", "2h", "4h", "6h", "12h", "1d"
    /// * `limit` - Number of records to return (default 30, maximum 500)
    /// * `start_time` - Start timestamp in milliseconds
    /// * `end_time` - End timestamp in milliseconds
    ///
    /// # Returns
    ///
    /// Returns list of open interest history records
    ///
    /// # API Endpoint
    ///
    /// GET /futures/data/openInterestHist (MARKET_DATA)
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use ccxt_exchanges::binance::Binance;
    /// # use ccxt_core::ExchangeConfig;
    /// # async fn example() -> ccxt_core::Result<()> {
    /// let binance = Binance::new_swap(ExchangeConfig::default())?;
    /// let history = binance.fetch_open_interest_history(
    ///     "BTC/USDT:USDT",
    ///     "1h",
    ///     Some(100),
    ///     None,
    ///     None
    /// ).await?;
    /// println!("History records: {}", history.len());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn fetch_open_interest_history(
        &self,
        symbol: &str,
        period: &str,
        limit: Option<u32>,
        start_time: Option<u64>,
        end_time: Option<u64>,
    ) -> Result<Vec<OpenInterestHistory>> {
        let market = self.base().market(symbol).await?;
        let mut request_params = HashMap::new();
        request_params.insert("symbol".to_string(), market.id.clone());
        request_params.insert("period".to_string(), period.to_string());

        if let Some(limit) = limit {
            request_params.insert("limit".to_string(), limit.to_string());
        }
        if let Some(start_time) = start_time {
            request_params.insert("startTime".to_string(), start_time.to_string());
        }
        if let Some(end_time) = end_time {
            request_params.insert("endTime".to_string(), end_time.to_string());
        }

        let mut request_url = format!("{}/data/openInterestHist?", self.urls().fapi_public);
        for (key, value) in &request_params {
            request_url.push_str(&format!("{}={}&", key, value));
        }

        let data = self.base().http_client.get(&request_url, None).await?;

        parser::parse_open_interest_history(&data, &market)
    }

    /// Fetch maximum available leverage for trading pair
    ///
    /// # Arguments
    ///
    /// * `symbol` - Trading pair symbol, e.g. "BTC/USDT:USDT"
    ///
    /// # Returns
    ///
    /// Returns maximum leverage information
    ///
    /// # Notes
    ///
    /// This method retrieves maximum leverage by querying leverage bracket data.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use ccxt_exchanges::binance::Binance;
    /// # use ccxt_core::ExchangeConfig;
    /// # async fn example() -> ccxt_core::Result<()> {
    /// let mut config = ExchangeConfig::default();
    /// config.api_key = Some("your-api-key".to_string());
    /// config.secret = Some("your-secret".to_string());
    /// let binance = Binance::new_swap(config)?;
    /// let max_leverage = binance.fetch_max_leverage("BTC/USDT:USDT").await?;
    /// println!("Maximum leverage: {}x", max_leverage.max_leverage);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn fetch_max_leverage(&self, symbol: &str) -> Result<MaxLeverage> {
        self.check_required_credentials()?;

        let market = self.base().market(symbol).await?;
        let mut request_params = HashMap::new();

        // Optional: if symbol not provided, returns leverage brackets for all pairs
        request_params.insert("symbol".to_string(), market.id.clone());

        let timestamp = self.fetch_time_raw().await?;
        let auth = self.get_auth()?;
        let signed_params =
            auth.sign_with_timestamp(&request_params, timestamp, Some(self.options().recv_window))?;

        let mut request_url = format!("{}/leverageBracket?", self.urls().fapi_private);
        for (key, value) in &signed_params {
            request_url.push_str(&format!("{}={}&", key, value));
        }

        let mut headers = HeaderMap::new();
        auth.add_auth_headers_reqwest(&mut headers);

        let data = self
            .base()
            .http_client
            .get(&request_url, Some(headers))
            .await?;

        parser::parse_max_leverage(&data, &market)
    }
    // ========================================================================
    // Futures Market Data Queries
    // ========================================================================

    /// Fetch index price
    ///
    /// # Arguments
    ///
    /// * `symbol` - Trading pair symbol, e.g. "BTC/USDT"
    ///
    /// # Returns
    ///
    /// Returns index price information
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ccxt_exchanges::binance::Binance;
    /// use ccxt_core::ExchangeConfig;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let config = ExchangeConfig::default();
    ///     let binance = Binance::new(config).unwrap();
    ///     
    ///     let index_price = binance.fetch_index_price("BTC/USDT").await.unwrap();
    ///     println!("Index Price: {}", index_price.index_price);
    /// }
    /// ```
    pub async fn fetch_index_price(&self, symbol: &str) -> Result<ccxt_core::types::IndexPrice> {
        let market = self.base().market(symbol).await?;

        let url = format!(
            "{}/premiumIndex?symbol={}",
            self.urls().fapi_public,
            market.id
        );

        let data = self.base().http_client.get(&url, None).await?;

        parser::parse_index_price(&data, &market)
    }

    /// Fetch premium index including mark price and funding rate
    ///
    /// # Arguments
    ///
    /// * `symbol` - Trading pair symbol, e.g. "BTC/USDT"; if `None`, returns all trading pairs
    ///
    /// # Returns
    ///
    /// Returns premium index information (single or multiple)
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ccxt_exchanges::binance::Binance;
    /// use ccxt_core::ExchangeConfig;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let config = ExchangeConfig::default();
    ///     let binance = Binance::new(config).unwrap();
    ///     
    ///     // Query single trading pair
    ///     let premium = binance.fetch_premium_index(Some("BTC/USDT")).await.unwrap();
    ///     println!("Mark Price: {}", premium[0].mark_price);
    ///
    ///     // Query all trading pairs
    ///     let all_premiums = binance.fetch_premium_index(None).await.unwrap();
    ///     println!("Total pairs: {}", all_premiums.len());
    /// }
    /// ```
    pub async fn fetch_premium_index(
        &self,
        symbol: Option<&str>,
    ) -> Result<Vec<ccxt_core::types::PremiumIndex>> {
        let url = if let Some(sym) = symbol {
            let market = self.base().market(sym).await?;
            format!(
                "{}/premiumIndex?symbol={}",
                self.urls().fapi_public,
                market.id
            )
        } else {
            format!("{}/premiumIndex", self.urls().fapi_public)
        };

        let data = self.base().http_client.get(&url, None).await?;

        if let Some(array) = data.as_array() {
            let mut results = Vec::new();

            let cache = self.base().market_cache.read().await;

            for item in array {
                if let Some(symbol_str) = item["symbol"].as_str() {
                    if let Some(market) = cache.markets_by_id.get(symbol_str) {
                        if let Ok(premium) = parser::parse_premium_index(item, market) {
                            results.push(premium);
                        }
                    }
                }
            }

            Ok(results)
        } else {
            // When symbol is provided, the API returns a single object (not an array).
            // In this case, symbol must be Some because we only reach this branch
            // when a specific symbol was requested.
            let sym = symbol.ok_or_else(|| {
                Error::from(ParseError::missing_field(
                    "symbol required when API returns single object",
                ))
            })?;
            let market = self.base().market(sym).await?;
            let premium = parser::parse_premium_index(&data, &market)?;
            Ok(vec![premium])
        }
    }

    /// Fetch liquidation orders
    ///
    /// # Arguments
    ///
    /// * `symbol` - Trading pair symbol, e.g. "BTC/USDT"; if `None`, returns all trading pairs
    /// * `start_time` - Start time (millisecond timestamp)
    /// * `end_time` - End time (millisecond timestamp)
    /// * `limit` - Quantity limit (default 100, maximum 1000)
    ///
    /// # Returns
    ///
    /// Returns list of liquidation orders
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ccxt_exchanges::binance::Binance;
    /// use ccxt_core::ExchangeConfig;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let config = ExchangeConfig::default();
    ///     let binance = Binance::new(config).unwrap();
    ///     
    ///     // Query recent liquidation orders
    ///     let liquidations = binance.fetch_liquidations(
    ///         Some("BTC/USDT"),
    ///         None,
    ///         None,
    ///         Some(50)
    ///     ).await.unwrap();
    ///     
    ///     for liq in liquidations {
    ///         println!("Liquidation: {} {} @ {}", liq.side, liq.quantity, liq.price);
    ///     }
    /// }
    /// ```
    pub async fn fetch_liquidations(
        &self,
        symbol: Option<&str>,
        start_time: Option<u64>,
        end_time: Option<u64>,
        limit: Option<u32>,
    ) -> Result<Vec<ccxt_core::types::Liquidation>> {
        let mut request_params = HashMap::new();

        if let Some(sym) = symbol {
            let market = self.base().market(sym).await?;
            request_params.insert("symbol".to_string(), market.id.clone());
        }

        if let Some(start) = start_time {
            request_params.insert("startTime".to_string(), start.to_string());
        }

        if let Some(end) = end_time {
            request_params.insert("endTime".to_string(), end.to_string());
        }

        if let Some(lim) = limit {
            request_params.insert("limit".to_string(), lim.to_string());
        }

        let mut url = format!("{}/allForceOrders?", self.urls().fapi_public);
        for (key, value) in &request_params {
            url.push_str(&format!("{}={}&", key, value));
        }

        let data = self.base().http_client.get(&url, None).await?;

        let array = data.as_array().ok_or_else(|| {
            Error::from(ParseError::invalid_value("data", "Expected array response"))
        })?;

        let mut results = Vec::new();

        let cache = self.base().market_cache.read().await;

        for item in array {
            // Get symbol and find corresponding market
            if let Some(symbol_str) = item["symbol"].as_str() {
                if let Some(market) = cache.markets_by_id.get(symbol_str) {
                    if let Ok(liquidation) = parser::parse_liquidation(item, market) {
                        results.push(liquidation);
                    }
                }
            }
        }

        Ok(results)
    }
    // ============================================================================
    // Internal Transfer System (P0.1)
    // ============================================================================

    /// Execute internal transfer
    ///
    /// Transfer assets between different account types (spot, margin, futures, funding, etc.)
    ///
    /// # Arguments
    ///
    /// * `currency` - Asset code (e.g. "USDT", "BTC")
    /// * `amount` - Transfer amount
    /// * `from_account` - Source account type (spot/margin/future/funding, etc.)
    /// * `to_account` - Target account type
    /// * `params` - Optional additional parameters
    ///
    /// # Binance API
    /// - Endpoint: POST /sapi/v1/asset/transfer
    /// - Weight: 900 (UID)
    /// - Required permission: TRADE
    /// - Signature required: Yes
    ///
    /// # Supported Account Types
    /// - MAIN: Spot account
    /// - UMFUTURE: USDT-margined futures account
    /// - CMFUTURE: Coin-margined futures account
    /// - MARGIN: Cross margin account
    /// - ISOLATED_MARGIN: Isolated margin account
    /// - FUNDING: Funding account
    /// - OPTION: Options account
    ///
    /// # Examples
    /// ```rust,no_run
    /// # use ccxt_exchanges::binance::Binance;
    /// # use ccxt_core::ExchangeConfig;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let config = ExchangeConfig {
    ///     api_key: Some("your_api_key".to_string()),
    ///     secret: Some("your_secret".to_string()),
    ///     ..Default::default()
    /// };
    /// let binance = Binance::new(config)?;
    ///
    /// // Transfer 100 USDT from spot to futures account
    /// let transfer = binance.transfer(
    ///     "USDT",
    ///     100.0,
    ///     "spot",      // From spot
    ///     "future",    // To futures
    ///     None
    /// ).await?;
    ///
    /// println!("Transfer ID: {:?}", transfer.id);
    /// println!("Status: {}", transfer.status);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn transfer(
        &self,
        currency: &str,
        amount: f64,
        from_account: &str,
        to_account: &str,
        params: Option<HashMap<String, String>>,
    ) -> Result<Transfer> {
        println!(
            "🚨 [TRANSFER ENTRY] Method called with currency={}, amount={}, from={}, to={}",
            currency, amount, from_account, to_account
        );

        println!("🚨 [TRANSFER STEP 1] Checking credentials...");
        self.check_required_credentials()?;
        println!("🚨 [TRANSFER STEP 1] ✅ Credentials check passed");

        let from_upper = from_account.to_uppercase();
        let to_upper = to_account.to_uppercase();

        // Normalize account names to Binance API format
        let from_normalized = match from_upper.as_str() {
            "SPOT" => "MAIN",
            "FUTURE" | "FUTURES" => "UMFUTURE",
            "MARGIN" => "MARGIN",
            "FUNDING" => "FUNDING",
            "OPTION" => "OPTION",
            other => other,
        };

        let to_normalized = match to_upper.as_str() {
            "SPOT" => "MAIN",
            "FUTURE" | "FUTURES" => "UMFUTURE",
            "MARGIN" => "MARGIN",
            "FUNDING" => "FUNDING",
            "OPTION" => "OPTION",
            other => other,
        };

        let transfer_type = format!("{}_{}", from_normalized, to_normalized);

        let mut request_params = params.unwrap_or_default();
        request_params.insert("type".to_string(), transfer_type);
        request_params.insert("asset".to_string(), currency.to_uppercase());
        request_params.insert("amount".to_string(), amount.to_string());

        println!(
            "🚨 [TRANSFER STEP 2] Request params prepared: {:?}",
            request_params
        );

        // Sign request with timestamp
        println!("🚨 [TRANSFER STEP 3] Fetching server timestamp...");
        let timestamp = self.fetch_time_raw().await?;
        println!("🚨 [TRANSFER STEP 3] ✅ Got timestamp: {}", timestamp);
        println!("🔍 [TRANSFER DEBUG] Timestamp: {}", timestamp);
        println!(
            "🔍 [TRANSFER DEBUG] Request params before signing: {:?}",
            request_params
        );

        println!("🚨 [TRANSFER STEP 4] Getting auth object...");
        let auth = self.get_auth()?;
        println!("🚨 [TRANSFER STEP 4] ✅ Got auth, now signing params...");

        // Use new tuple return value: (HashMap, sorted query string)
        let signed_params =
            auth.sign_with_timestamp(&request_params, timestamp, Some(self.options().recv_window))?;
        println!("🚨 [TRANSFER STEP 4] ✅ Params signed successfully");
        let query_string = auth.build_query_string(&signed_params);
        println!(
            "🔍 [TRANSFER DEBUG] Query string (signed and sorted): {}",
            query_string
        );

        // Use sorted query string from Auth module to ensure consistent parameter order
        let url = format!("{}/asset/transfer?{}", self.urls().sapi, &query_string);

        println!("🔍 [TRANSFER DEBUG] Final URL: {}", url);

        let mut headers = reqwest::header::HeaderMap::new();
        auth.add_auth_headers_reqwest(&mut headers);

        println!("🚨 [TRANSFER STEP 5] Sending POST request to URL: {}", url);
        let data = self
            .base()
            .http_client
            .post(&url, Some(headers), None)
            .await?;
        println!("🚨 [TRANSFER STEP 5] ✅ Got response, parsing...");

        println!("🚨 [TRANSFER STEP 6] Parsing transfer response...");
        let result = parser::parse_transfer(&data);
        match &result {
            Ok(_) => tracing::error!("🚨 [TRANSFER STEP 6] ✅ Parse successful"),
            Err(e) => tracing::error!("🚨 [TRANSFER STEP 6] ❌ Parse failed: {:?}", e),
        }
        result
    }

    /// Fetch internal transfer history
    ///
    /// Retrieve historical records of transfers between accounts
    ///
    /// # Arguments
    ///
    /// * `currency` - Optional asset code filter
    /// * `since` - Optional start timestamp (milliseconds)
    /// * `limit` - Optional quantity limit (default 100, maximum 100)
    /// * `params` - Optional additional parameters (may include fromSymbol/toSymbol to specify transfer direction)
    ///
    /// # API Endpoint
    /// - Endpoint: GET /sapi/v1/asset/transfer
    /// - Weight: 1
    /// - Required permission: USER_DATA
    /// - Signature required: Yes
    ///
    /// # Examples
    /// ```rust,no_run
    /// # use ccxt_exchanges::binance::Binance;
    /// # use ccxt_core::ExchangeConfig;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let config = ExchangeConfig {
    ///     api_key: Some("your_api_key".to_string()),
    ///     secret: Some("your_secret".to_string()),
    ///     ..Default::default()
    /// };
    /// let binance = Binance::new(config)?;
    ///
    ///     // Query recent USDT transfer records
    ///     let transfers = binance.fetch_transfers(
    ///     Some("USDT"),
    ///     None,
    ///     Some(50),
    ///     None
    /// ).await?;
    ///
    /// for transfer in transfers {
    ///     println!("{} {} from {:?} to {:?}",
    ///         transfer.amount,
    ///         transfer.currency,
    ///         transfer.from_account,
    ///         transfer.to_account
    ///     );
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn fetch_transfers(
        &self,
        currency: Option<&str>,
        since: Option<u64>,
        limit: Option<u64>,
        params: Option<HashMap<String, String>>,
    ) -> Result<Vec<Transfer>> {
        self.check_required_credentials()?;

        let mut request_params = params.unwrap_or_default();

        // The 'type' parameter is required; return error if not provided in params
        let transfer_type = request_params
            .get("type")
            .ok_or_else(|| {
                Error::invalid_request(
                    "type parameter is required for fetch_transfers. Examples: MAIN_UMFUTURE, MAIN_CMFUTURE, MAIN_MARGIN, etc."
                )
            })?
            .clone();

        // Validate fromSymbol and toSymbol are provided when required
        // fromSymbol is required when type is ISOLATEDMARGIN_MARGIN or ISOLATEDMARGIN_ISOLATEDMARGIN
        if transfer_type == "ISOLATEDMARGIN_MARGIN"
            || transfer_type == "ISOLATEDMARGIN_ISOLATEDMARGIN"
        {
            if !request_params.contains_key("fromSymbol") {
                return Err(Error::invalid_request(format!(
                    "fromSymbol is required when type is {}",
                    transfer_type
                )));
            }
        }

        // toSymbol is required when type is MARGIN_ISOLATEDMARGIN or ISOLATEDMARGIN_ISOLATEDMARGIN
        if transfer_type == "MARGIN_ISOLATEDMARGIN"
            || transfer_type == "ISOLATEDMARGIN_ISOLATEDMARGIN"
        {
            if !request_params.contains_key("toSymbol") {
                return Err(Error::invalid_request(format!(
                    "toSymbol is required when type is {}",
                    transfer_type
                )));
            }
        }

        if let Some(code) = currency {
            request_params.insert("asset".to_string(), code.to_uppercase());
        }

        if let Some(ts) = since {
            request_params.insert("startTime".to_string(), ts.to_string());
        }

        let size = limit.unwrap_or(10).min(100);
        request_params.insert("size".to_string(), size.to_string());

        if !request_params.contains_key("current") {
            request_params.insert("current".to_string(), "1".to_string());
        }

        let timestamp = self.fetch_time_raw().await?;
        let auth = self.get_auth()?;
        let signed_params =
            auth.sign_with_timestamp(&request_params, timestamp, Some(self.options().recv_window))?;

        let query_string = auth.build_query_string(&signed_params);
        let url = format!(
            "{}/sapi/v1/asset/transfer?{}",
            self.urls().sapi,
            query_string
        );

        let mut headers = reqwest::header::HeaderMap::new();
        auth.add_auth_headers_reqwest(&mut headers);

        let data = self.base().http_client.get(&url, Some(headers)).await?;

        // Binance response format: { "total": 2, "rows": [...] }
        let rows = data["rows"]
            .as_array()
            .ok_or_else(|| Error::from(ParseError::missing_field("rows")))?;

        let mut transfers = Vec::new();
        for row in rows {
            match parser::parse_transfer(row) {
                Ok(transfer) => transfers.push(transfer),
                Err(e) => {
                    warn!(error = %e, "Failed to parse transfer");
                }
            }
        }

        Ok(transfers)
    }
    // ============================================================================
    // Futures Transfer API
    // ============================================================================

    /// Execute futures account transfer
    ///
    /// Transfer assets between spot account and futures accounts
    ///
    /// # Arguments
    ///
    /// * `currency` - Asset code (e.g. "USDT", "BTC")
    /// * `amount` - Transfer quantity
    /// * `transfer_type` - Transfer type:
    ///   - 1: Spot account → USDT-M futures account
    ///   - 2: USDT-M futures account → Spot account
    ///   - 3: Spot account → COIN-M futures account
    ///   - 4: COIN-M futures account → Spot account
    /// * `params` - Optional additional parameters
    ///
    /// # API Endpoint
    /// - Endpoint: POST /sapi/v1/futures/transfer
    /// - Weight: 1
    /// - Required permission: TRADE
    /// - Signature required: Yes
    ///
    /// # Differences from transfer()
    /// - `futures_transfer()`: Futures-specific API with concise parameters, only requires type (1-4)
    /// - `transfer()`: Generic API supporting 8 account types, requires fromSymbol/toSymbol parameters
    ///
    /// # Examples
    /// ```rust,no_run
    /// # use ccxt_exchanges::binance::Binance;
    /// # use ccxt_core::ExchangeConfig;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let config = ExchangeConfig {
    ///     api_key: Some("your_api_key".to_string()),
    ///     secret: Some("your_secret".to_string()),
    ///     ..Default::default()
    /// };
    /// let binance = Binance::new(config)?;
    ///
    ///     // Transfer 100 USDT from spot to USDT-M futures account
    ///     let transfer = binance.futures_transfer(
    ///     "USDT",
    ///     100.0,
    ///     1,  // type 1 = Spot → USDT-M futures
    ///     None
    /// ).await?;
    ///
    /// println!("Transfer ID: {:?}", transfer.id);
    /// println!("Status: {}", transfer.status);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn futures_transfer(
        &self,
        currency: &str,
        amount: f64,
        transfer_type: i32,
        params: Option<HashMap<String, String>>,
    ) -> Result<Transfer> {
        self.check_required_credentials()?;

        // Validate transfer_type parameter range
        if !(1..=4).contains(&transfer_type) {
            return Err(Error::invalid_request(format!(
                "Invalid futures transfer type: {}. Must be between 1 and 4",
                transfer_type
            )));
        }

        // Use parser helper function to get account names (for response parsing)
        let (from_account, to_account) = parser::parse_futures_transfer_type(transfer_type)?;

        let mut request_params = params.unwrap_or_default();
        request_params.insert("asset".to_string(), currency.to_uppercase());
        request_params.insert("amount".to_string(), amount.to_string());
        request_params.insert("type".to_string(), transfer_type.to_string());

        let auth = self.get_auth()?;
        let signed_params = auth.sign_params(&request_params)?;

        let query_string: Vec<String> = signed_params
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect();
        let url = format!(
            "{}/futures/transfer?{}",
            self.urls().sapi,
            query_string.join("&")
        );

        let mut headers = reqwest::header::HeaderMap::new();
        auth.add_auth_headers_reqwest(&mut headers);

        let data = self
            .base()
            .http_client
            .post(&url, Some(headers), None)
            .await?;

        // Binance response format: { "tranId": 100000001 }
        // Manually construct Transfer object
        let tran_id = data["tranId"]
            .as_i64()
            .ok_or_else(|| Error::from(ParseError::missing_field("tranId")))?;

        let timestamp = chrono::Utc::now().timestamp_millis();
        let datetime = chrono::DateTime::from_timestamp_millis(timestamp)
            .map(|dt| dt.to_rfc3339())
            .unwrap_or_default();

        Ok(Transfer {
            id: Some(tran_id.to_string()),
            timestamp: timestamp as u64,
            datetime,
            currency: currency.to_uppercase(),
            amount,
            from_account: Some(from_account.to_string()),
            to_account: Some(to_account.to_string()),
            status: "pending".to_string(), // Transfer request submitted, but status unknown
            info: Some(data),
        })
    }

    /// Fetch futures transfer history
    ///
    /// Retrieve historical records of transfers between spot and futures accounts
    ///
    /// # Arguments
    ///
    /// * `currency` - Asset code (e.g. "USDT", "BTC")
    /// * `since` - Optional start timestamp (milliseconds)
    /// * `limit` - Optional quantity limit (default 10, maximum 100)
    /// * `params` - Optional additional parameters
    ///   - `endTime`: End timestamp (milliseconds)
    ///   - `current`: Current page number (default 1)
    ///
    /// # API Endpoint
    /// - Endpoint: GET /sapi/v1/futures/transfer
    /// - Weight: 10
    /// - Required permission: USER_DATA
    /// - Signature required: Yes
    ///
    /// # Return Fields
    /// - tranId: Transfer ID
    /// - asset: Asset code
    /// - amount: Transfer quantity
    /// - type: Transfer type (1-4)
    /// - timestamp: Transfer time
    /// - status: Transfer status (CONFIRMED/FAILED/PENDING)
    ///
    /// # Examples
    /// ```rust,no_run
    /// # use ccxt_exchanges::binance::Binance;
    /// # use ccxt_core::ExchangeConfig;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let config = ExchangeConfig {
    ///     api_key: Some("your_api_key".to_string()),
    ///     secret: Some("your_secret".to_string()),
    ///     ..Default::default()
    /// };
    /// let binance = Binance::new(config)?;
    ///
    ///     // Query USDT transfer records from the last 30 days
    ///     let since = chrono::Utc::now().timestamp_millis() as u64 - 30 * 24 * 60 * 60 * 1000;
    /// let transfers = binance.fetch_futures_transfers(
    ///     "USDT",
    ///     Some(since),
    ///     Some(50),
    ///     None
    /// ).await?;
    ///
    /// for transfer in transfers {
    ///     println!("{} {} from {:?} to {:?} - Status: {}",
    ///         transfer.amount,
    ///         transfer.currency,
    ///         transfer.from_account,
    ///         transfer.to_account,
    ///         transfer.status
    ///     );
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn fetch_futures_transfers(
        &self,
        currency: &str,
        since: Option<u64>,
        limit: Option<u64>,
        params: Option<HashMap<String, String>>,
    ) -> Result<Vec<Transfer>> {
        self.check_required_credentials()?;

        let mut request_params = params.unwrap_or_default();

        request_params.insert("asset".to_string(), currency.to_uppercase());

        if let Some(ts) = since {
            request_params.insert("startTime".to_string(), ts.to_string());
        }

        // Binance limit: size maximum 100
        let size = limit.unwrap_or(10).min(100);
        request_params.insert("size".to_string(), size.to_string());

        if !request_params.contains_key("current") {
            request_params.insert("current".to_string(), "1".to_string());
        }

        let auth = self.get_auth()?;
        let signed_params = auth.sign_params(&request_params)?;

        let query_string: Vec<String> = signed_params
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect();
        let url = format!(
            "{}/futures/transfer?{}",
            self.urls().sapi,
            query_string.join("&")
        );

        let mut headers = reqwest::header::HeaderMap::new();
        auth.add_auth_headers_reqwest(&mut headers);

        let data = self.base().http_client.get(&url, Some(headers)).await?;

        // Binance response format: { "total": 2, "rows": [...] }
        let rows = data["rows"]
            .as_array()
            .ok_or_else(|| Error::from(ParseError::missing_field("rows")))?;

        let mut transfers = Vec::new();
        for row in rows {
            // Parse type field and convert to account names
            if let Some(transfer_type) = row["type"].as_i64() {
                if let Ok((from_account, to_account)) =
                    parser::parse_futures_transfer_type(transfer_type as i32)
                {
                    let tran_id = row["tranId"].as_i64().map(|id| id.to_string());

                    let timestamp = row["timestamp"]
                        .as_i64()
                        .unwrap_or_else(|| chrono::Utc::now().timestamp_millis());

                    let datetime = chrono::DateTime::from_timestamp_millis(timestamp)
                        .map(|dt| dt.to_rfc3339())
                        .unwrap_or_default();

                    let asset = row["asset"].as_str().unwrap_or(currency).to_string();

                    let amount = if let Some(amount_str) = row["amount"].as_str() {
                        amount_str.parse::<f64>().unwrap_or(0.0)
                    } else {
                        row["amount"].as_f64().unwrap_or(0.0)
                    };

                    let status = row["status"].as_str().unwrap_or("SUCCESS").to_lowercase();

                    transfers.push(Transfer {
                        id: tran_id,
                        timestamp: timestamp as u64,
                        datetime,
                        currency: asset,
                        amount,
                        from_account: Some(from_account.to_string()),
                        to_account: Some(to_account.to_string()),
                        status,
                        info: Some(row.clone()),
                    });
                } else {
                    warn!(
                        transfer_type = transfer_type,
                        "Invalid futures transfer type"
                    );
                }
            }
        }

        Ok(transfers)
    }

    /// Fetch deposit and withdrawal fee information
    ///
    /// Query deposit and withdrawal fee configuration for all assets
    ///
    /// # Arguments
    ///
    /// * `currency` - Optional asset code filter (returns all assets if not specified)
    /// * `params` - Optional additional parameters
    ///
    /// # API Endpoint
    /// - Endpoint: GET /sapi/v1/capital/config/getall
    /// - Weight: 10
    /// - Required permission: USER_DATA
    /// - Signature required: Yes
    ///
    /// # Return Information
    /// - Withdrawal fee
    /// - Minimum/maximum withdrawal amount
    /// - Deposit/withdrawal support status
    /// - Network configurations (BTC, ETH, BSC, etc.)
    /// - Confirmation requirements
    ///
    /// # Examples
    /// ```rust,no_run
    /// # use ccxt_exchanges::binance::Binance;
    /// # use ccxt_core::ExchangeConfig;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let config = ExchangeConfig {
    ///     api_key: Some("your_api_key".to_string()),
    ///     secret: Some("your_secret".to_string()),
    ///     ..Default::default()
    /// };
    /// let binance = Binance::new(config)?;
    ///
    ///     // Fetch fee information for all assets
    ///     let fees = binance.fetch_deposit_withdraw_fees(None, None).await?;
    ///
    ///     // Fetch fee information for specific asset (e.g. USDT)
    ///     let usdt_fees = binance.fetch_deposit_withdraw_fees(Some("USDT"), None).await?;
    ///
    /// for fee in usdt_fees {
    ///     println!("{}: withdraw fee = {}, min = {}, max = {}",
    ///         fee.currency,
    ///         fee.withdraw_fee,
    ///         fee.withdraw_min,
    ///         fee.withdraw_max
    ///     );
    ///     
    ///     for network in &fee.networks {
    ///         println!("  Network {}: fee = {}", network.network, network.withdraw_fee);
    ///     }
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn fetch_deposit_withdraw_fees(
        &self,
        currency: Option<&str>,
        params: Option<HashMap<String, String>>,
    ) -> Result<Vec<ccxt_core::types::DepositWithdrawFee>> {
        self.check_required_credentials()?;

        let request_params = params.unwrap_or_default();

        // Note: Binance /sapi/v1/capital/config/getall endpoint returns all currencies
        // If currency is specified, client-side filtering is required

        let auth = self.get_auth()?;
        let signed_params = auth.sign_params(&request_params)?;

        let query_string: Vec<String> = signed_params
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect();
        let url = format!(
            "{}/capital/config/getall?{}",
            self.urls().sapi,
            query_string.join("&")
        );

        let mut headers = reqwest::header::HeaderMap::new();
        auth.add_auth_headers_reqwest(&mut headers);

        let data = self.base().http_client.get(&url, Some(headers)).await?;

        let all_fees = parser::parse_deposit_withdraw_fees(&data)?;

        // Filter results if currency is specified
        if let Some(code) = currency {
            let code_upper = code.to_uppercase();
            Ok(all_fees
                .into_iter()
                .filter(|fee| fee.currency == code_upper)
                .collect())
        } else {
            Ok(all_fees)
        }
    }

    // ============================================================================
    // Deposit/Withdrawal API - P1.1
    // ============================================================================

    /// Withdraw to external address
    ///
    /// # Arguments
    /// * `code` - Currency code (e.g. "BTC", "USDT")
    /// * `amount` - Withdrawal amount
    /// * `address` - Withdrawal address
    /// * `params` - Optional parameters
    ///   - `tag`: Address tag (e.g. XRP tag)
    ///   - `network`: Network type (e.g. "ETH", "BSC", "TRX")
    ///   - `addressTag`: Address tag (alias)
    ///   - `name`: Address memo name
    ///   - `walletType`: Wallet type (0=spot, 1=funding)
    ///
    /// # API Endpoint
    /// - Endpoint: POST /sapi/v1/capital/withdraw/apply
    /// - Required permission: API key with withdrawal enabled
    ///
    /// # Examples
    /// ```no_run
    /// # use ccxt_exchanges::binance::Binance;
    /// # use ccxt_core::ExchangeConfig;
    /// # use std::collections::HashMap;
    /// # async fn example() -> ccxt_core::Result<()> {
    /// let binance = Binance::new(ExchangeConfig::default())?;
    ///
    ///     // Basic withdrawal
    ///     let tx = binance.withdraw("USDT", "100.0", "TXxxx...", None).await?;
    ///     println!("Withdrawal ID: {}", tx.id);
    ///
    ///     // Withdrawal with specific network
    ///     let mut params = HashMap::new();
    /// params.insert("network".to_string(), "TRX".to_string());
    /// let tx = binance.withdraw("USDT", "100.0", "TXxxx...", Some(params)).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn withdraw(
        &self,
        code: &str,
        amount: &str,
        address: &str,
        params: Option<HashMap<String, String>>,
    ) -> Result<Transaction> {
        self.check_required_credentials()?;

        let mut request_params = params.unwrap_or_default();

        request_params.insert("coin".to_string(), code.to_uppercase());
        request_params.insert("address".to_string(), address.to_string());
        request_params.insert("amount".to_string(), amount.to_string());

        // Handle optional tag parameter (supports both 'tag' and 'addressTag' names)
        if let Some(tag) = request_params.get("tag").cloned() {
            request_params.insert("addressTag".to_string(), tag.clone());
        }

        let auth = self.get_auth()?;
        let signed_params = auth.sign_params(&request_params)?;

        let query_string: Vec<String> = signed_params
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect();
        let url = format!(
            "{}/capital/withdraw/apply?{}",
            self.urls().sapi,
            query_string.join("&")
        );

        let mut headers = reqwest::header::HeaderMap::new();
        auth.add_auth_headers_reqwest(&mut headers);

        let data = self
            .base()
            .http_client
            .post(&url, Some(headers), None)
            .await?;

        parser::parse_transaction(&data, TransactionType::Withdrawal)
    }

    /// Fetch deposit history
    ///
    /// # Arguments
    /// * `code` - Optional currency code (e.g. "BTC", "USDT")
    /// * `since` - Optional start timestamp (milliseconds)
    /// * `limit` - Optional quantity limit
    /// * `params` - Optional parameters
    ///   - `coin`: Currency (overrides code parameter)
    ///   - `status`: Status filter (0=pending, 6=credited, 1=success)
    ///   - `startTime`: Start timestamp (milliseconds)
    ///   - `endTime`: End timestamp (milliseconds)
    ///   - `offset`: Offset (for pagination)
    ///   - `txId`: Transaction ID filter
    ///
    /// # API Endpoint
    /// - Endpoint: GET /sapi/v1/capital/deposit/hisrec
    /// - Required permission: API key
    ///
    /// # Time Range Limit
    /// - Maximum query range: 90 days
    /// - Default returns last 90 days if no time range provided
    ///
    /// # Examples
    /// ```no_run
    /// # use ccxt_exchanges::binance::Binance;
    /// # use ccxt_core::ExchangeConfig;
    /// # async fn example() -> ccxt_core::Result<()> {
    /// let binance = Binance::new(ExchangeConfig::default())?;
    ///
    ///     // Query all deposits
    /// let deposits = binance.fetch_deposits(None, None, Some(100), None).await?;
    /// println!("Total deposits: {}", deposits.len());
    ///
    ///     // Query BTC deposits
    ///     let btc_deposits = binance.fetch_deposits(Some("BTC"), None, None, None).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn fetch_deposits(
        &self,
        code: Option<&str>,
        since: Option<i64>,
        limit: Option<i64>,
        params: Option<HashMap<String, String>>,
    ) -> Result<Vec<Transaction>> {
        self.check_required_credentials()?;

        let mut request_params = params.unwrap_or_default();

        if let Some(coin) = code {
            request_params
                .entry("coin".to_string())
                .or_insert_with(|| coin.to_uppercase());
        }

        if let Some(start_time) = since {
            request_params
                .entry("startTime".to_string())
                .or_insert_with(|| start_time.to_string());
        }

        if let Some(lim) = limit {
            request_params.insert("limit".to_string(), lim.to_string());
        }

        let auth = self.get_auth()?;
        let signed_params = auth.sign_params(&request_params)?;

        let query_string: Vec<String> = signed_params
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect();
        let url = format!(
            "{}/capital/deposit/hisrec?{}",
            self.urls().sapi,
            query_string.join("&")
        );

        let mut headers = reqwest::header::HeaderMap::new();
        auth.add_auth_headers_reqwest(&mut headers);

        let data = self.base().http_client.get(&url, Some(headers)).await?;

        if let Some(arr) = data.as_array() {
            arr.iter()
                .map(|item| parser::parse_transaction(item, TransactionType::Deposit))
                .collect()
        } else {
            Err(Error::invalid_request("Expected array response"))
        }
    }

    /// Fetch withdrawal history
    ///
    /// # Arguments
    /// * `code` - Optional currency code (e.g. "BTC", "USDT")
    /// * `since` - Optional start timestamp (milliseconds)
    /// * `limit` - Optional quantity limit
    /// * `params` - Optional parameters
    ///   - `coin`: Currency (overrides code parameter)
    ///   - `withdrawOrderId`: Withdrawal order ID
    ///   - `status`: Status filter (0=email sent, 1=cancelled, 2=awaiting approval, 3=rejected, 4=processing, 5=failure, 6=completed)
    ///   - `startTime`: Start timestamp (milliseconds)
    ///   - `endTime`: End timestamp (milliseconds)
    ///   - `offset`: Offset (for pagination)
    ///
    /// # API Endpoint
    /// - Endpoint: GET /sapi/v1/capital/withdraw/history
    /// - Required permission: API key
    ///
    /// # Time Range Limit
    /// - Maximum query range: 90 days
    /// - Default returns last 90 days if no time range provided
    ///
    /// # Examples
    /// ```no_run
    /// # use ccxt_exchanges::binance::Binance;
    /// # use ccxt_core::ExchangeConfig;
    /// # async fn example() -> ccxt_core::Result<()> {
    /// let binance = Binance::new(ExchangeConfig::default())?;
    ///
    ///     // Query all withdrawals
    ///     let withdrawals = binance.fetch_withdrawals(None, None, Some(100), None).await?;
    ///     println!("Total withdrawals: {}", withdrawals.len());
    ///
    ///     // Query USDT withdrawals
    ///     let usdt_withdrawals = binance.fetch_withdrawals(Some("USDT"), None, None, None).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn fetch_withdrawals(
        &self,
        code: Option<&str>,
        since: Option<i64>,
        limit: Option<i64>,
        params: Option<HashMap<String, String>>,
    ) -> Result<Vec<Transaction>> {
        self.check_required_credentials()?;

        let mut request_params = params.unwrap_or_default();

        if let Some(coin) = code {
            request_params
                .entry("coin".to_string())
                .or_insert_with(|| coin.to_uppercase());
        }

        if let Some(start_time) = since {
            request_params
                .entry("startTime".to_string())
                .or_insert_with(|| start_time.to_string());
        }

        if let Some(lim) = limit {
            request_params.insert("limit".to_string(), lim.to_string());
        }

        let auth = self.get_auth()?;
        let signed_params = auth.sign_params(&request_params)?;

        let query_string: Vec<String> = signed_params
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect();
        let url = format!(
            "{}/capital/withdraw/history?{}",
            self.urls().sapi,
            query_string.join("&")
        );

        let mut headers = reqwest::header::HeaderMap::new();
        auth.add_auth_headers_reqwest(&mut headers);

        let data = self.base().http_client.get(&url, Some(headers)).await?;

        if let Some(arr) = data.as_array() {
            arr.iter()
                .map(|item| parser::parse_transaction(item, TransactionType::Withdrawal))
                .collect()
        } else {
            Err(Error::invalid_request("Expected array response"))
        }
    }

    /// Fetch deposit address
    ///
    /// # Arguments
    /// * `code` - Currency code (e.g. "BTC", "USDT")
    /// * `params` - Optional parameters
    ///   - `network`: Network type (e.g. "ETH", "BSC", "TRX")
    ///   - `coin`: Currency (overrides code parameter)
    ///
    /// # API Endpoint
    /// - Endpoint: GET /sapi/v1/capital/deposit/address
    /// - Required permission: API key
    ///
    /// # Notes
    /// - Binance automatically generates new address if none exists
    /// - Some currencies require network parameter (e.g. USDT can be ETH/BSC/TRX network)
    ///
    /// # Examples
    /// ```no_run
    /// # use ccxt_exchanges::binance::Binance;
    /// # use ccxt_core::ExchangeConfig;
    /// # use std::collections::HashMap;
    /// # async fn example() -> ccxt_core::Result<()> {
    /// let binance = Binance::new(ExchangeConfig::default())?;
    ///
    ///     // Fetch BTC deposit address
    ///     let addr = binance.fetch_deposit_address("BTC", None).await?;
    ///     println!("BTC address: {}", addr.address);
    ///
    ///     // Fetch USDT deposit address on TRX network
    ///     let mut params = HashMap::new();
    /// params.insert("network".to_string(), "TRX".to_string());
    /// let addr = binance.fetch_deposit_address("USDT", Some(params)).await?;
    /// println!("USDT-TRX address: {}", addr.address);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn fetch_deposit_address(
        &self,
        code: &str,
        params: Option<HashMap<String, String>>,
    ) -> Result<DepositAddress> {
        self.check_required_credentials()?;

        let mut request_params = params.unwrap_or_default();

        request_params
            .entry("coin".to_string())
            .or_insert_with(|| code.to_uppercase());

        let auth = self.get_auth()?;
        let signed_params = auth.sign_params(&request_params)?;

        let query_string: Vec<String> = signed_params
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect();
        let url = format!(
            "{}/capital/deposit/address?{}",
            self.urls().sapi,
            query_string.join("&")
        );

        let mut headers = reqwest::header::HeaderMap::new();
        auth.add_auth_headers_reqwest(&mut headers);

        let data = self.base().http_client.get(&url, Some(headers)).await?;

        parser::parse_deposit_address(&data)
    }

    // ============================================================================
    // OCO Order Management Methods (P1.3)
    // ============================================================================

    /// Create OCO order (One-Cancels-the-Other)
    ///
    /// OCO order contains two orders: take-profit order (limit) + stop-loss order (stop-limit).
    /// When one order is filled, the other is automatically cancelled.
    ///
    /// # Arguments
    ///
    /// * `symbol` - Trading pair symbol, e.g. "BTC/USDT"
    /// * `side` - Order side (Buy or Sell)
    /// * `amount` - Order quantity
    /// * `price` - Take-profit price (limit order price)
    /// * `stop_price` - Stop-loss trigger price
    /// * `stop_limit_price` - Optional stop-limit order price, defaults to stop_price
    /// * `params` - Optional additional parameters
    ///
    /// # Returns
    ///
    /// Returns created OCO order information
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use ccxt_exchanges::binance::Binance;
    /// # use ccxt_core::{ExchangeConfig, types::OrderSide};
    /// # async fn example() -> ccxt_core::Result<()> {
    /// let binance = Binance::new(ExchangeConfig::default())?;
    /// // Long BTC position: take profit at 50000, stop loss at 45000
    /// let oco = binance.create_oco_order(
    ///     "BTC/USDT",
    ///     OrderSide::Sell,
    ///     0.1,
    ///     50000.0,
    ///     45000.0,
    ///     Some(44500.0),
    ///     None
    /// ).await?;
    /// println!("OCO order ID: {}", oco.order_list_id);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn create_oco_order(
        &self,
        symbol: &str,
        side: OrderSide,
        amount: f64,
        price: f64,
        stop_price: f64,
        stop_limit_price: Option<f64>,
        params: Option<HashMap<String, String>>,
    ) -> Result<OcoOrder> {
        self.check_required_credentials()?;

        let market = self.base().market(symbol).await?;
        let mut request_params = HashMap::new();

        request_params.insert("symbol".to_string(), market.id.clone());
        request_params.insert(
            "side".to_string(),
            match side {
                OrderSide::Buy => "BUY".to_string(),
                OrderSide::Sell => "SELL".to_string(),
            },
        );
        request_params.insert("quantity".to_string(), amount.to_string());
        request_params.insert("price".to_string(), price.to_string());
        request_params.insert("stopPrice".to_string(), stop_price.to_string());

        // Default stop-limit price to stop_price if not specified
        let stop_limit = stop_limit_price.unwrap_or(stop_price);
        request_params.insert("stopLimitPrice".to_string(), stop_limit.to_string());

        request_params.insert("stopLimitTimeInForce".to_string(), "GTC".to_string());

        if let Some(extra) = params {
            for (k, v) in extra {
                request_params.insert(k, v);
            }
        }

        let timestamp = self.fetch_time_raw().await?;
        let auth = self.get_auth()?;
        let signed_params =
            auth.sign_with_timestamp(&request_params, timestamp, Some(self.options().recv_window))?;

        let url = format!("{}/order/oco", self.urls().private);
        let mut headers = HeaderMap::new();
        auth.add_auth_headers_reqwest(&mut headers);

        let body = serde_json::to_value(&signed_params).map_err(|e| {
            Error::from(ParseError::invalid_format(
                "data",
                format!("Failed to serialize params: {}", e),
            ))
        })?;

        let data = self
            .base()
            .http_client
            .post(&url, Some(headers), Some(body))
            .await?;

        parser::parse_oco_order(&data)
    }

    /// Fetch single OCO order
    ///
    /// # Arguments
    ///
    /// * `order_list_id` - OCO order list ID
    /// * `symbol` - Trading pair symbol
    /// * `params` - Optional additional parameters
    ///
    /// # Returns
    ///
    /// Returns OCO order information
    pub async fn fetch_oco_order(
        &self,
        order_list_id: i64,
        symbol: &str,
        params: Option<HashMap<String, String>>,
    ) -> Result<OcoOrder> {
        self.check_required_credentials()?;

        let market = self.base().market(symbol).await?;
        let mut request_params = HashMap::new();

        request_params.insert("orderListId".to_string(), order_list_id.to_string());
        request_params.insert("symbol".to_string(), market.id.clone());

        if let Some(extra) = params {
            for (k, v) in extra {
                request_params.insert(k, v);
            }
        }

        let timestamp = self.fetch_time_raw().await?;
        let auth = self.get_auth()?;
        let signed_params =
            auth.sign_with_timestamp(&request_params, timestamp, Some(self.options().recv_window))?;

        let mut url = format!("{}/orderList?", self.urls().private);
        for (key, value) in &signed_params {
            url.push_str(&format!("{}={}&", key, value));
        }

        let mut headers = HeaderMap::new();
        auth.add_auth_headers_reqwest(&mut headers);

        let data = self.base().http_client.get(&url, Some(headers)).await?;

        parser::parse_oco_order(&data)
    }

    /// Fetch all OCO orders
    ///
    /// # Arguments
    ///
    /// * `symbol` - Trading pair symbol
    /// * `since` - Optional start timestamp (milliseconds)
    /// * `limit` - Optional record quantity limit (default 500, maximum 1000)
    /// * `params` - Optional additional parameters
    ///
    /// # Returns
    ///
    /// Returns list of OCO orders
    pub async fn fetch_oco_orders(
        &self,
        symbol: &str,
        since: Option<u64>,
        limit: Option<u32>,
        params: Option<HashMap<String, String>>,
    ) -> Result<Vec<OcoOrder>> {
        self.check_required_credentials()?;

        let market = self.base().market(symbol).await?;
        let mut request_params = HashMap::new();

        request_params.insert("symbol".to_string(), market.id.clone());

        if let Some(s) = since {
            request_params.insert("startTime".to_string(), s.to_string());
        }

        if let Some(l) = limit {
            request_params.insert("limit".to_string(), l.to_string());
        }

        if let Some(extra) = params {
            for (k, v) in extra {
                request_params.insert(k, v);
            }
        }

        let timestamp = self.fetch_time_raw().await?;
        let auth = self.get_auth()?;
        let signed_params =
            auth.sign_with_timestamp(&request_params, timestamp, Some(self.options().recv_window))?;

        let mut url = format!("{}/allOrderList?", self.urls().private);
        for (key, value) in &signed_params {
            url.push_str(&format!("{}={}&", key, value));
        }

        let mut headers = HeaderMap::new();
        auth.add_auth_headers_reqwest(&mut headers);

        let data = self.base().http_client.get(&url, Some(headers)).await?;

        let oco_array = data.as_array().ok_or_else(|| {
            Error::from(ParseError::invalid_format(
                "data",
                "Expected array of OCO orders",
            ))
        })?;

        let mut oco_orders = Vec::new();
        for oco_data in oco_array {
            match parser::parse_oco_order(oco_data) {
                Ok(oco) => oco_orders.push(oco),
                Err(e) => {
                    warn!(error = %e, "Failed to parse OCO order");
                }
            }
        }

        Ok(oco_orders)
    }

    /// Cancel OCO order
    ///
    /// # Arguments
    ///
    /// * `order_list_id` - OCO order list ID
    /// * `symbol` - Trading pair symbol
    /// * `params` - Optional additional parameters
    ///
    /// # Returns
    ///
    /// Returns cancelled OCO order information
    pub async fn cancel_oco_order(
        &self,
        order_list_id: i64,
        symbol: &str,
        params: Option<HashMap<String, String>>,
    ) -> Result<OcoOrder> {
        self.check_required_credentials()?;

        let market = self.base().market(symbol).await?;
        let mut request_params = HashMap::new();

        request_params.insert("symbol".to_string(), market.id.clone());
        request_params.insert("orderListId".to_string(), order_list_id.to_string());

        if let Some(extra) = params {
            for (k, v) in extra {
                request_params.insert(k, v);
            }
        }

        let timestamp = self.fetch_time_raw().await?;
        let auth = self.get_auth()?;
        let signed_params =
            auth.sign_with_timestamp(&request_params, timestamp, Some(self.options().recv_window))?;

        let url = format!("{}/orderList", self.urls().private);
        let mut headers = HeaderMap::new();
        auth.add_auth_headers_reqwest(&mut headers);

        let body = serde_json::to_value(&signed_params).map_err(|e| {
            Error::from(ParseError::invalid_format(
                "data",
                format!("Failed to serialize params: {}", e),
            ))
        })?;

        let data = self
            .base()
            .http_client
            .delete(&url, Some(headers), Some(body))
            .await?;

        parser::parse_oco_order(&data)
    }

    /// Create test order (does not place actual order)
    ///
    /// Used to test if order parameters are correct without submitting to exchange.
    ///
    /// # Arguments
    ///
    /// * `symbol` - Trading pair symbol
    /// * `order_type` - Order type
    /// * `side` - Order side (Buy/Sell)
    /// * `amount` - Order quantity
    /// * `price` - Optional price
    /// * `params` - Optional additional parameters
    ///
    /// # Returns
    ///
    /// Returns empty order information (validation result only)
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use ccxt_exchanges::binance::Binance;
    /// # use ccxt_core::{ExchangeConfig, types::{OrderType, OrderSide}};
    /// # async fn example() -> ccxt_core::Result<()> {
    /// let binance = Binance::new(ExchangeConfig::default())?;
    /// // Test if order parameters are correct
    /// let test = binance.create_test_order(
    ///     "BTC/USDT",
    ///     OrderType::Limit,
    ///     OrderSide::Buy,
    ///     0.001,
    ///     Some(40000.0),
    ///     None
    /// ).await?;
    /// println!("Order parameters validated successfully");
    /// # Ok(())
    /// # }
    /// ```
    pub async fn create_test_order(
        &self,
        symbol: &str,
        order_type: OrderType,
        side: OrderSide,
        amount: f64,
        price: Option<f64>,
        params: Option<HashMap<String, String>>,
    ) -> Result<Order> {
        self.check_required_credentials()?;

        let market = self.base().market(symbol).await?;
        let mut request_params = HashMap::new();

        request_params.insert("symbol".to_string(), market.id.clone());
        request_params.insert(
            "side".to_string(),
            match side {
                OrderSide::Buy => "BUY".to_string(),
                OrderSide::Sell => "SELL".to_string(),
            },
        );
        request_params.insert(
            "type".to_string(),
            match order_type {
                OrderType::Market => "MARKET".to_string(),
                OrderType::Limit => "LIMIT".to_string(),
                OrderType::StopLoss => "STOP_LOSS".to_string(),
                OrderType::StopLossLimit => "STOP_LOSS_LIMIT".to_string(),
                OrderType::TakeProfit => "TAKE_PROFIT".to_string(),
                OrderType::TakeProfitLimit => "TAKE_PROFIT_LIMIT".to_string(),
                OrderType::LimitMaker => "LIMIT_MAKER".to_string(),
                OrderType::StopMarket => "STOP_MARKET".to_string(),
                OrderType::StopLimit => "STOP_LIMIT".to_string(),
                OrderType::TrailingStop => "TRAILING_STOP_MARKET".to_string(),
            },
        );
        request_params.insert("quantity".to_string(), amount.to_string());

        if let Some(p) = price {
            request_params.insert("price".to_string(), p.to_string());
        }

        // Limit orders require timeInForce
        if order_type == OrderType::Limit
            || order_type == OrderType::StopLossLimit
            || order_type == OrderType::TakeProfitLimit
        {
            if !request_params.contains_key("timeInForce") {
                request_params.insert("timeInForce".to_string(), "GTC".to_string());
            }
        }

        if let Some(extra) = params {
            for (k, v) in extra {
                request_params.insert(k, v);
            }
        }

        let timestamp = self.fetch_time_raw().await?;
        let auth = self.get_auth()?;
        let signed_params =
            auth.sign_with_timestamp(&request_params, timestamp, Some(self.options().recv_window))?;

        // Use test endpoint
        let url = format!("{}/order/test", self.urls().private);
        let mut headers = HeaderMap::new();
        auth.add_auth_headers_reqwest(&mut headers);

        let body = serde_json::to_value(&signed_params).map_err(|e| {
            Error::from(ParseError::invalid_format(
                "data",
                format!("Failed to serialize params: {}", e),
            ))
        })?;

        let data = self
            .base()
            .http_client
            .post(&url, Some(headers), Some(body))
            .await?;

        // Test order returns empty object {}, return a dummy order
        Ok(Order {
            id: "test".to_string(),
            client_order_id: None,
            timestamp: Some(timestamp as i64),
            datetime: Some(
                chrono::DateTime::from_timestamp(timestamp as i64 / 1000, 0)
                    .map(|dt| dt.to_rfc3339())
                    .unwrap_or_default(),
            ),
            last_trade_timestamp: None,
            status: OrderStatus::Open,
            symbol: symbol.to_string(),
            order_type,
            time_in_force: Some("GTC".to_string()),
            side,
            price: price.map(Decimal::from_f64_retain).flatten(),
            average: None,
            amount: Decimal::from_f64_retain(amount)
                .ok_or_else(|| Error::from(ParseError::missing_field("amount")))?,
            filled: Some(Decimal::ZERO),
            remaining: Decimal::from_f64_retain(amount).map(Some).unwrap_or(None),
            cost: None,
            trades: None,
            fee: None,
            post_only: None,
            reduce_only: None,
            trigger_price: None,
            stop_price: None,
            take_profit_price: None,
            stop_loss_price: None,
            trailing_delta: None,
            trailing_percent: None,
            activation_price: None,
            callback_rate: None,
            working_type: None,
            fees: Some(Vec::new()),
            info: data
                .as_object()
                .map(|obj| obj.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
                .unwrap_or_default(),
        })
    }

    /// Fetch all isolated margin borrow rates
    ///
    /// # Arguments
    /// * `params` - Optional parameters
    ///
    /// # Returns
    /// HashMap<Symbol, IsolatedBorrowRate>
    ///
    /// # API Endpoint
    /// GET /sapi/v1/margin/isolatedMarginData
    ///
    /// # Examples
    /// ```no_run
    /// # use ccxt_exchanges::binance::Binance;
    /// # use ccxt_core::ExchangeConfig;
    /// # async fn example() -> ccxt_core::Result<()> {
    /// let binance = Binance::new(ExchangeConfig::default())?;
    /// let rates = binance.fetch_isolated_borrow_rates(None).await?;
    /// println!("Got {} symbols", rates.len());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn fetch_isolated_borrow_rates(
        &self,
        params: Option<HashMap<String, String>>,
    ) -> Result<HashMap<String, ccxt_core::types::IsolatedBorrowRate>> {
        self.check_required_credentials()?;

        let request_params = params.unwrap_or_default();

        let auth = self.get_auth()?;
        let signed_params = auth.sign_params(&request_params)?;

        let query_string: Vec<String> = signed_params
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect();
        let url = format!(
            "{}/margin/isolatedMarginData?{}",
            self.urls().sapi,
            query_string.join("&")
        );

        let mut headers = reqwest::header::HeaderMap::new();
        auth.add_auth_headers_reqwest(&mut headers);

        let response = self.base().http_client.get(&url, Some(headers)).await?;

        parser::parse_isolated_borrow_rates(&response)
    }

    /// Fetch borrow interest history
    ///
    /// # Arguments
    /// * `code` - Asset code (optional, e.g., "BTC")
    /// * `symbol` - Trading pair symbol (optional, for isolated margin only, e.g., "BTC/USDT")
    /// * `since` - Start timestamp (milliseconds)
    /// * `limit` - Number of records to return (default 10, max 100)
    /// * `params` - Optional parameters
    ///   - `endTime`: End timestamp
    ///   - `isolatedSymbol`: Isolated margin trading pair (overrides symbol parameter)
    ///   - `archived`: Whether to query archived data (default false)
    ///
    /// # API Endpoint
    /// - Spot Margin: GET /sapi/v1/margin/interestHistory
    /// - Portfolio Margin: GET /papi/v1/margin/marginInterestHistory
    ///
    /// # Examples
    /// ```no_run
    /// # use ccxt_exchanges::binance::Binance;
    /// # use ccxt_core::ExchangeConfig;
    /// # async fn example() -> ccxt_core::Result<()> {
    /// let binance = Binance::new(ExchangeConfig::default())?;
    ///
    /// // Fetch cross-margin BTC borrow interest
    /// let interests = binance.fetch_borrow_interest(
    ///     Some("BTC"),
    ///     None,
    ///     None,
    ///     Some(10),
    ///     None
    /// ).await?;
    ///
    /// // Fetch isolated margin BTC/USDT borrow interest
    /// let interests = binance.fetch_borrow_interest(
    ///     None,
    ///     Some("BTC/USDT"),
    ///     None,
    ///     Some(10),
    ///     None
    /// ).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn fetch_borrow_interest(
        &self,
        code: Option<&str>,
        symbol: Option<&str>,
        since: Option<u64>,
        limit: Option<u64>,
        params: Option<HashMap<String, String>>,
    ) -> Result<Vec<BorrowInterest>> {
        self.check_required_credentials()?;

        let mut request_params = params.unwrap_or_default();

        if let Some(c) = code {
            request_params.insert("asset".to_string(), c.to_string());
        }

        if let Some(s) = symbol {
            self.load_markets(false).await?;
            let market = self.base().market(s).await?;
            request_params.insert("isolatedSymbol".to_string(), market.id.clone());
        }

        if let Some(st) = since {
            request_params.insert("startTime".to_string(), st.to_string());
        }

        if let Some(l) = limit {
            request_params.insert("size".to_string(), l.to_string());
        }

        let is_portfolio = request_params
            .get("type")
            .map(|t| t == "PORTFOLIO")
            .unwrap_or(false);

        let auth = self.get_auth()?;
        let signed_params = auth.sign_params(&request_params)?;

        let query_string: Vec<String> = signed_params
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect();

        let url = if is_portfolio {
            format!(
                "{}/margin/marginInterestHistory?{}",
                self.urls().papi,
                query_string.join("&")
            )
        } else {
            format!(
                "{}/margin/interestHistory?{}",
                self.urls().sapi,
                query_string.join("&")
            )
        };

        let mut headers = reqwest::header::HeaderMap::new();
        auth.add_auth_headers_reqwest(&mut headers);

        let response = self.base().http_client.get(&url, Some(headers)).await?;

        parser::parse_borrow_interests(&response)
    }

    /// Fetch margin borrow rate history
    ///
    /// # Arguments
    /// * `code` - Asset code (e.g., "USDT")
    /// * `since` - Start timestamp in milliseconds
    /// * `limit` - Number of records to return (default 10, max 100, but actual limit is 92)
    /// * `params` - Optional parameters
    ///   - `endTime`: End timestamp
    ///   - `vipLevel`: VIP level
    ///
    /// # Notes
    /// - Binance API has special limit restriction, cannot exceed 92
    /// - For more data, use multiple requests with pagination
    ///
    /// # API Endpoint
    /// GET /sapi/v1/margin/interestRateHistory
    ///
    /// # Examples
    /// ```no_run
    /// # use ccxt_exchanges::binance::Binance;
    /// # use ccxt_core::ExchangeConfig;
    /// # async fn example() -> ccxt_core::Result<()> {
    /// let binance = Binance::new(ExchangeConfig::default())?;
    /// let history = binance.fetch_borrow_rate_history(
    ///     "USDT",
    ///     None,
    ///     Some(50),
    ///     None
    /// ).await?;
    /// println!("Got {} records", history.len());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn fetch_borrow_rate_history(
        &self,
        code: &str,
        since: Option<u64>,
        limit: Option<u64>,
        params: Option<HashMap<String, String>>,
    ) -> Result<Vec<BorrowRateHistory>> {
        self.check_required_credentials()?;

        let mut request_params = params.unwrap_or_default();
        request_params.insert("asset".to_string(), code.to_string());

        // Binance API limit: cannot exceed 92
        let actual_limit = limit.unwrap_or(10).min(92);

        if let Some(st) = since {
            request_params.insert("startTime".to_string(), st.to_string());

            // Binance requires limited time range per request
            if !request_params.contains_key("endTime") {
                // Each record represents 1 day, so calculate endTime = startTime + (limit * 1 day)
                let end_time = st + (actual_limit * 86400000); // 86400000 ms = 1 day
                request_params.insert("endTime".to_string(), end_time.to_string());
            }
        }

        let auth = self.get_auth()?;
        let signed_params = auth.sign_params(&request_params)?;

        let query_string: Vec<String> = signed_params
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect();
        let url = format!(
            "{}/margin/interestRateHistory?{}",
            self.urls().sapi,
            query_string.join("&")
        );

        let mut headers = reqwest::header::HeaderMap::new();
        auth.add_auth_headers_reqwest(&mut headers);

        let response = self.base().http_client.get(&url, Some(headers)).await?;

        let records = response
            .as_array()
            .ok_or_else(|| Error::from(ParseError::invalid_format("data", "Expected array")))?;

        let mut history = Vec::new();
        for record in records {
            let rate = parser::parse_borrow_rate_history(record, code)?;
            history.push(rate);
        }

        Ok(history)
    }

    // ========================================================================
    // Ledger Methods
    // ========================================================================

    /// Fetch ledger history
    ///
    /// **Important Notes**:
    /// - Only supports futures wallets (option, USDT-M, COIN-M)
    /// - Spot wallet is not supported, returns NotSupported error
    /// - For portfolioMargin accounts, uses unified account API
    ///
    /// # API Endpoint Mapping
    ///
    /// | Market Type | Endpoint | Field Format |
    /// |-------------|----------|--------------|
    /// | Option | GET /eapi/v1/bill | id, asset, amount, type |
    /// | USDT-M (linear) | GET /fapi/v1/income | tranId, asset, income, incomeType |
    /// | USDT-M+Portfolio | GET /papi/v1/um/income | tranId, asset, income, incomeType |
    /// | COIN-M (inverse) | GET /dapi/v1/income | tranId, asset, income, incomeType |
    /// | COIN-M+Portfolio | GET /papi/v1/cm/income | tranId, asset, income, incomeType |
    ///
    /// # Arguments
    ///
    /// * `code` - Currency code (required for option market)
    /// * `since` - Start timestamp in milliseconds
    /// * `limit` - Maximum number of records to return
    /// * `params` - Additional parameters:
    ///   - `until`: End timestamp in milliseconds
    ///   - `portfolioMargin`: Whether to use portfolio margin account (unified account)
    ///   - `type`: Market type ("spot", "margin", "future", "swap", "option")
    ///   - `subType`: Sub-type ("linear", "inverse")
    ///
    /// # Returns
    ///
    /// Returns list of LedgerEntry, sorted by time in descending order
    ///
    /// # Errors
    ///
    /// - Returns NotSupported error if market type is spot
    /// - Returns ArgumentsRequired error if code parameter is not provided for option market
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use ccxt_exchanges::binance::Binance;
    /// use ccxt_core::ExchangeConfig;
    /// use std::collections::HashMap;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let mut config = ExchangeConfig::default();
    ///     config.api_key = Some("your-api-key".to_string());
    ///     config.secret = Some("your-api-secret".to_string());
    ///     
    ///     let binance = Binance::new(config).unwrap();
    ///     
    ///     // USDT-M futures ledger
    ///     let mut params = HashMap::new();
    ///     params.insert("type".to_string(), "future".to_string());
    ///     params.insert("subType".to_string(), "linear".to_string());
    ///     let ledger = binance.fetch_ledger(
    ///         Some("USDT".to_string()),
    ///         None,
    ///         Some(100),
    ///         Some(params)
    ///     ).await.unwrap();
    ///
    ///     // Option ledger (code is required)
    ///     let mut params = HashMap::new();
    ///     params.insert("type".to_string(), "option".to_string());
    ///     let ledger = binance.fetch_ledger(
    ///         Some("USDT".to_string()),
    ///         None,
    ///         Some(100),
    ///         Some(params)
    ///     ).await.unwrap();
    /// }
    /// ```
    pub async fn fetch_ledger(
        &self,
        code: Option<String>,
        since: Option<i64>,
        limit: Option<i64>,
        params: Option<HashMap<String, String>>,
    ) -> Result<Vec<ccxt_core::types::LedgerEntry>> {
        self.check_required_credentials()?;

        let params = params.unwrap_or_default();

        let market_type = params.get("type").map(|s| s.as_str()).unwrap_or("future");
        let sub_type = params.get("subType").map(|s| s.as_str());
        let portfolio_margin = params
            .get("portfolioMargin")
            .and_then(|s| s.parse::<bool>().ok())
            .unwrap_or(false);

        if market_type == "spot" {
            return Err(Error::not_implemented(
                "fetch_ledger() is not supported for spot market, only for futures/swap/option",
            ));
        }

        let (endpoint_base, path) = if market_type == "option" {
            if code.is_none() {
                return Err(Error::invalid_request(
                    "fetch_ledger() requires code for option market",
                ));
            }
            (self.urls().eapi.clone(), "/eapi/v1/bill")
        } else if portfolio_margin {
            if sub_type == Some("inverse") {
                (self.urls().papi.clone(), "/papi/v1/cm/income")
            } else {
                (self.urls().papi.clone(), "/papi/v1/um/income")
            }
        } else if sub_type == Some("inverse") {
            (self.urls().dapi.clone(), "/dapi/v1/income")
        } else {
            (self.urls().fapi.clone(), "/fapi/v1/income")
        };

        let mut request_params = HashMap::new();

        if let Some(currency) = &code {
            request_params.insert("asset".to_string(), currency.clone());
        }

        if let Some(start_time) = since {
            request_params.insert("startTime".to_string(), start_time.to_string());
        }
        if let Some(end_time_str) = params.get("until") {
            request_params.insert("endTime".to_string(), end_time_str.clone());
        }

        if let Some(limit_val) = limit {
            request_params.insert("limit".to_string(), limit_val.to_string());
        }

        let auth = self.get_auth()?;
        let signed_params = auth.sign_params(&request_params)?;

        let query_string: Vec<String> = signed_params
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect();
        let url = format!("{}{}?{}", endpoint_base, path, query_string.join("&"));

        let mut headers = reqwest::header::HeaderMap::new();
        auth.add_auth_headers_reqwest(&mut headers);

        let response = self.base().http_client.get(&url, Some(headers)).await?;

        let records = response
            .as_array()
            .ok_or_else(|| Error::from(ParseError::invalid_format("data", "Expected array")))?;

        let mut ledger_entries = Vec::new();
        for record in records {
            let entry = parser::parse_ledger_entry(record)?;
            ledger_entries.push(entry);
        }

        Ok(ledger_entries)
    }
    /// Fetch trades for a specific order
    ///
    /// # Arguments
    ///
    /// * `id` - Order ID
    /// * `symbol` - Trading pair symbol (required)
    /// * `since` - Optional start timestamp in milliseconds
    /// * `limit` - Optional maximum number of records
    ///
    /// # Returns
    ///
    /// Returns all trades for the specified order
    ///
    /// # Notes
    ///
    /// - `symbol` parameter is required
    /// - Only supports spot markets
    /// - Internally calls fetch_my_trades method with orderId filter
    /// - API endpoint: GET /api/v3/myTrades
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use ccxt_exchanges::binance::Binance;
    /// # use ccxt_core::ExchangeConfig;
    /// # async fn example() -> ccxt_core::Result<()> {
    /// let mut config = ExchangeConfig::default();
    /// config.api_key = Some("your-api-key".to_string());
    /// config.secret = Some("your-secret".to_string());
    /// let binance = Binance::new(config)?;
    ///
    /// let trades = binance.fetch_order_trades("123456789", "BTC/USDT", None, None).await?;
    /// println!("Order has {} trades", trades.len());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn fetch_order_trades(
        &self,
        id: &str,
        symbol: &str,
        since: Option<u64>,
        limit: Option<u32>,
    ) -> Result<Vec<Trade>> {
        self.check_required_credentials()?;

        let market = self.base().market(symbol).await?;
        if !market.is_spot() {
            return Err(Error::not_implemented(
                "fetch_order_trades() supports spot markets only",
            ));
        }

        let mut params = HashMap::new();
        params.insert("symbol".to_string(), market.id.clone());
        params.insert("orderId".to_string(), id.to_string());

        if let Some(s) = since {
            params.insert("startTime".to_string(), s.to_string());
        }

        if let Some(l) = limit {
            params.insert("limit".to_string(), l.to_string());
        }

        let timestamp = self.fetch_time_raw().await?;
        let auth = self.get_auth()?;
        let signed_params =
            auth.sign_with_timestamp(&params, timestamp, Some(self.options().recv_window))?;

        let mut url = format!("{}/myTrades?", self.urls().private);
        for (key, value) in &signed_params {
            url.push_str(&format!("{}={}&", key, value));
        }

        let mut headers = HeaderMap::new();
        auth.add_auth_headers_reqwest(&mut headers);

        let data = self.base().http_client.get(&url, Some(headers)).await?;

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

        Ok(trades)
    }

    /// Fetch canceled orders
    ///
    /// # Arguments
    ///
    /// * `symbol` - Trading pair symbol (required)
    /// * `since` - Optional start timestamp in milliseconds
    /// * `limit` - Optional maximum number of records
    ///
    /// # Returns
    ///
    /// Returns list of canceled orders
    ///
    /// # Notes
    ///
    /// - `symbol` parameter is required
    /// - Calls fetch_orders to get all orders, then filters for canceled status
    /// - `limit` parameter is applied client-side (fetch all orders first, then take first N)
    /// - Implementation logic matches Go version
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use ccxt_exchanges::binance::Binance;
    /// # use ccxt_core::ExchangeConfig;
    /// # async fn example() -> ccxt_core::Result<()> {
    /// let mut config = ExchangeConfig::default();
    /// config.api_key = Some("your-api-key".to_string());
    /// config.secret = Some("your-secret".to_string());
    /// let binance = Binance::new(config)?;
    ///
    /// let canceled_orders = binance.fetch_canceled_orders("BTC/USDT", None, Some(10)).await?;
    /// println!("Found {} canceled orders", canceled_orders.len());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn fetch_canceled_orders(
        &self,
        symbol: &str,
        since: Option<u64>,
        limit: Option<u32>,
    ) -> Result<Vec<Order>> {
        let all_orders = self.fetch_orders(Some(symbol), since, None).await?;

        let mut canceled_orders: Vec<Order> = all_orders
            .into_iter()
            .filter(|order| order.status == OrderStatus::Cancelled)
            .collect();

        if let Some(since_time) = since {
            canceled_orders.retain(|order| {
                order
                    .timestamp
                    .map(|ts| ts >= since_time as i64)
                    .unwrap_or(true)
            });
        }

        if let Some(limit_val) = limit {
            canceled_orders.truncate(limit_val as usize);
        }

        Ok(canceled_orders)
    }

    /// Create market buy order by cost amount
    ///
    /// # Arguments
    ///
    /// * `symbol` - Trading pair symbol
    /// * `cost` - Amount in quote currency to spend (e.g., spend 100 USDT to buy BTC)
    /// * `params` - Optional additional parameters
    ///
    /// # Returns
    ///
    /// Returns created order information
    ///
    /// # Notes
    ///
    /// - Only supports spot markets
    /// - `cost` parameter is converted to Binance API's `quoteOrderQty` parameter
    /// - This is a convenience method to simplify market buy order creation
    /// - Internally calls create_order method
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use ccxt_exchanges::binance::Binance;
    /// # use ccxt_core::ExchangeConfig;
    /// # async fn example() -> ccxt_core::Result<()> {
    /// let mut config = ExchangeConfig::default();
    /// config.api_key = Some("your-api-key".to_string());
    /// config.secret = Some("your-secret".to_string());
    /// let binance = Binance::new(config)?;
    ///
    /// // Spend 100 USDT to buy BTC (market order)
    /// let order = binance.create_market_buy_order_with_cost("BTC/USDT", 100.0, None).await?;
    /// println!("Order created: {:?}", order);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn create_market_buy_order_with_cost(
        &self,
        symbol: &str,
        cost: f64,
        params: Option<HashMap<String, String>>,
    ) -> Result<Order> {
        self.check_required_credentials()?;

        let market = self.base().market(symbol).await?;
        if !market.is_spot() {
            return Err(Error::not_implemented(
                "create_market_buy_order_with_cost() supports spot orders only",
            ));
        }

        let mut order_params = params.unwrap_or_default();
        order_params.insert("cost".to_string(), cost.to_string());

        // The create_order method detects the cost parameter and converts it to quoteOrderQty
        self.create_order(
            symbol,
            OrderType::Market,
            OrderSide::Buy,
            cost,
            None,
            Some(order_params),
        )
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ccxt_core::ExchangeConfig;

    #[tokio::test]
    #[ignore] // Requires actual API connection
    async fn test_fetch_markets() {
        let config = ExchangeConfig::default();
        let binance = Binance::new(config).unwrap();

        let markets = binance.fetch_markets().await;
        assert!(markets.is_ok());

        let markets = markets.unwrap();
        assert!(!markets.is_empty());
    }

    #[tokio::test]
    #[ignore] // Requires actual API connection
    async fn test_fetch_ticker() {
        let config = ExchangeConfig::default();
        let binance = Binance::new(config).unwrap();

        let _ = binance.fetch_markets().await;

        let ticker = binance
            .fetch_ticker("BTC/USDT", ccxt_core::types::TickerParams::default())
            .await;
        assert!(ticker.is_ok());

        let ticker = ticker.unwrap();
        assert_eq!(ticker.symbol, "BTC/USDT");
        assert!(ticker.last.is_some());
    }

    #[tokio::test]
    #[ignore] // Requires actual API connection
    async fn test_fetch_order_book() {
        let config = ExchangeConfig::default();
        let binance = Binance::new(config).unwrap();

        let _ = binance.fetch_markets().await;

        let orderbook = binance.fetch_order_book("BTC/USDT", Some(10)).await;
        assert!(orderbook.is_ok());

        let orderbook = orderbook.unwrap();
        assert_eq!(orderbook.symbol, "BTC/USDT");
        assert!(!orderbook.bids.is_empty());
        assert!(!orderbook.asks.is_empty());
    }

    #[tokio::test]
    #[ignore] // Requires actual API connection
    async fn test_fetch_trades() {
        let config = ExchangeConfig::default();
        let binance = Binance::new(config).unwrap();

        let _ = binance.fetch_markets().await;

        let trades = binance.fetch_trades("BTC/USDT", Some(10)).await;
        assert!(trades.is_ok());

        let trades = trades.unwrap();
        assert!(!trades.is_empty());
        assert!(trades.len() <= 10);
    }
}
