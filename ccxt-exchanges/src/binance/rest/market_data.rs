//! Binance public market data operations.
//!
//! This module contains all public market data methods that don't require authentication.
//! These include ticker data, order books, trades, OHLCV data, and market statistics.

use super::super::{Binance, parser};
use ccxt_core::{
    Error, ParseError, Result,
    types::{
        AggTrade, BidAsk, IntoTickerParams, LastPrice, MarkPrice, ServerTime, Stats24hr, Ticker,
        Trade, TradingLimits,
    },
};
use reqwest::header::HeaderMap;
use std::sync::Arc;
use tracing::warn;

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
    pub(crate) async fn fetch_time_raw(&self) -> Result<u64> {
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
    pub async fn fetch_status(&self) -> Result<serde_json::Value> {
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
    /// Returns a HashMap of [`Market`] structures containing market information.
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
    pub async fn fetch_markets(
        &self,
    ) -> Result<std::collections::HashMap<String, Arc<ccxt_core::types::Market>>> {
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
    pub async fn load_markets(
        &self,
        reload: bool,
    ) -> Result<std::collections::HashMap<String, Arc<ccxt_core::types::Market>>> {
        // Acquire the loading lock to serialize concurrent load_markets calls
        // This prevents multiple tasks from making duplicate API calls
        let _loading_guard = self.base().market_loading_lock.lock().await;

        // Check cache status while holding the lock
        {
            let cache = self.base().market_cache.read().await;
            if cache.loaded && !reload {
                tracing::debug!(
                    "Returning cached markets for Binance ({} markets)",
                    cache.markets.len()
                );
                return Ok(cache.markets.clone());
            }
        }

        tracing::info!("Loading markets for Binance (reload: {})", reload);
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
    pub async fn fetch_order_book(
        &self,
        symbol: &str,
        limit: Option<u32>,
    ) -> Result<ccxt_core::types::OrderBook> {
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
    pub async fn fetch_recent_trades(
        &self,
        symbol: &str,
        limit: Option<u32>,
    ) -> Result<Vec<Trade>> {
        self.fetch_trades(symbol, limit).await
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
    pub async fn fetch_agg_trades(
        &self,
        symbol: &str,
        since: Option<u64>,
        limit: Option<u32>,
        params: Option<std::collections::HashMap<String, String>>,
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
    pub async fn fetch_historical_trades(
        &self,
        symbol: &str,
        _since: Option<u64>,
        limit: Option<u32>,
        params: Option<std::collections::HashMap<String, String>>,
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

    /// Get OHLCV API endpoint based on market type and price type.
    ///
    /// # Arguments
    /// * `market` - Market information
    /// * `price` - Price type: None (default) | "mark" | "index" | "premiumIndex"
    ///
    /// # Returns
    /// Returns tuple (base_url, endpoint, use_pair)
    fn get_ohlcv_endpoint(
        &self,
        market: &std::sync::Arc<ccxt_core::types::Market>,
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

    /// Fetch OHLCV (candlestick) data.
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
    ///
    /// # Returns
    ///
    /// Returns OHLCV data array: [timestamp, open, high, low, close, volume]
    pub async fn fetch_ohlcv(
        &self,
        symbol: &str,
        timeframe: &str,
        since: Option<i64>,
        limit: Option<u32>,
        params: Option<std::collections::HashMap<String, serde_json::Value>>,
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

            // Calculate endTime for inverse markets
            if market.inverse.unwrap_or(false) && start_time > 0 && until.is_none() {
                let duration = self.parse_timeframe(timeframe)?;
                let calculated_end_time =
                    start_time + (adjusted_limit as i64 * duration * 1000) - 1;
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

    /// Fetch server time.
    ///
    /// Retrieves the current server timestamp from the exchange.
    ///
    /// # Returns
    ///
    /// Returns [`ServerTime`] containing the server timestamp and formatted datetime.
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
    /// let binance = Binance::new(ExchangeConfig::default())?;
    /// let server_time = binance.fetch_time().await?;
    /// println!("Server time: {} ({})", server_time.server_time, server_time.datetime);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn fetch_time(&self) -> Result<ServerTime> {
        let timestamp = self.fetch_time_raw().await?;
        Ok(ServerTime::new(timestamp as i64))
    }

    /// Fetch best bid/ask prices.
    ///
    /// Retrieves the best bid and ask prices for one or all trading pairs.
    ///
    /// # Arguments
    ///
    /// * `symbol` - Optional trading pair symbol; if omitted, returns all symbols
    ///
    /// # Returns
    ///
    /// Returns a vector of [`BidAsk`] structures containing bid/ask prices.
    ///
    /// # API Endpoint
    ///
    /// * GET `/api/v3/ticker/bookTicker`
    /// * Weight: 1 for single symbol, 2 for all symbols
    /// * Requires signature: No
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
    /// Returns a vector of [`LastPrice`] structures containing the latest prices.
    ///
    /// # API Endpoint
    ///
    /// * GET `/api/v3/ticker/price`
    /// * Weight: 1 for single symbol, 2 for all symbols
    /// * Requires signature: No
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
    /// Returns a vector of [`MarkPrice`] structures containing mark prices and funding rates.
    ///
    /// # API Endpoint
    ///
    /// * GET `/fapi/v1/premiumIndex`
    /// * Weight: 1 for single symbol, 10 for all symbols
    /// * Requires signature: No
    ///
    /// # Note
    ///
    /// This API only applies to futures markets (USDT-margined perpetual contracts).
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
    /// let binance = Binance::new(ExchangeConfig::default())?;
    ///
    /// // Fetch mark price for single futures symbol
    /// let mark_price = binance.fetch_mark_price(Some("BTC/USDT:USDT")).await?;
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
}
