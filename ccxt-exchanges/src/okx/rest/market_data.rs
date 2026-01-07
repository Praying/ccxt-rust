//! Market data endpoints for OKX REST API.

use super::super::{Okx, parser};
use ccxt_core::{
    Error, ParseError, Result,
    types::{Market, OHLCV, OhlcvRequest, OrderBook, Ticker, Trade},
};
use std::{collections::HashMap, sync::Arc};
use tracing::{info, warn};

impl Okx {
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
    pub async fn fetch_markets(&self) -> Result<Arc<HashMap<String, Arc<Market>>>> {
        let path = Self::build_api_path("/public/instruments");
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

        let result = self.base().set_markets(markets, None).await?;

        info!("Loaded {} markets for OKX", result.len());
        Ok(result)
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
        let _loading_guard = self.base().market_loading_lock.lock().await;

        {
            let cache = self.base().market_cache.read().await;
            if cache.is_loaded() && !reload {
                return Ok(cache.markets());
            }
        }

        info!("Loading markets for OKX (reload: {})", reload);
        let _markets = self.fetch_markets().await?;

        let cache = self.base().market_cache.read().await;
        Ok(cache.markets())
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

        let path = Self::build_api_path("/market/ticker");
        let mut params = HashMap::new();
        params.insert("instId".to_string(), market.id.clone());

        let response = self.public_request("GET", &path, Some(&params)).await?;

        let data = response
            .get("data")
            .ok_or_else(|| Error::from(ParseError::missing_field("data")))?;

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
        if !cache.is_loaded() {
            drop(cache);
            return Err(Error::exchange(
                "-1",
                "Markets not loaded. Call load_markets() first.",
            ));
        }
        // Build a snapshot of markets by ID for efficient lookup
        let markets_snapshot: std::collections::HashMap<String, Arc<Market>> = cache
            .iter_markets()
            .map(|(_, m)| (m.id.clone(), m))
            .collect();
        drop(cache);

        let path = Self::build_api_path("/market/tickers");
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
                if let Some(market) = markets_snapshot.get(inst_id) {
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
                                symbol = %inst_id,
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
    /// * `limit` - Optional depth limit (valid values: 1-400; default: 100).
    ///
    /// # Returns
    ///
    /// Returns [`OrderBook`] data containing bids and asks.
    pub async fn fetch_order_book(&self, symbol: &str, limit: Option<u32>) -> Result<OrderBook> {
        let market = self.base().market(symbol).await?;

        let path = Self::build_api_path("/market/books");
        let mut params = HashMap::new();
        params.insert("instId".to_string(), market.id.clone());

        let actual_limit = limit.map_or(100, |l| l.min(400));
        params.insert("sz".to_string(), actual_limit.to_string());

        let response = self.public_request("GET", &path, Some(&params)).await?;

        let data = response
            .get("data")
            .ok_or_else(|| Error::from(ParseError::missing_field("data")))?;

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

        let path = Self::build_api_path("/market/trades");
        let mut params = HashMap::new();
        params.insert("instId".to_string(), market.id.clone());

        let actual_limit = limit.map_or(100, |l| l.min(500));
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

        trades.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

        Ok(trades)
    }

    /// Fetch OHLCV (candlestick) data using the builder pattern.
    ///
    /// This is the preferred method for fetching OHLCV data. It accepts an [`OhlcvRequest`]
    /// built using the builder pattern, which provides validation and a more ergonomic API.
    ///
    /// # Arguments
    ///
    /// * `request` - OHLCV request built via [`OhlcvRequest::builder()`]
    ///
    /// # Returns
    ///
    /// Returns a vector of [`OHLCV`] structures.
    ///
    /// # Errors
    ///
    /// Returns an error if the market is not found or the API request fails.
    ///
    /// _Requirements: 2.3, 2.6_
    pub async fn fetch_ohlcv_v2(&self, request: OhlcvRequest) -> Result<Vec<OHLCV>> {
        let market = self.base().market(&request.symbol).await?;

        let timeframes = self.timeframes();
        let okx_timeframe = timeframes.get(&request.timeframe).ok_or_else(|| {
            Error::invalid_request(format!("Unsupported timeframe: {}", request.timeframe))
        })?;

        let path = Self::build_api_path("/market/candles");
        let mut params = HashMap::new();
        params.insert("instId".to_string(), market.id.clone());
        params.insert("bar".to_string(), okx_timeframe.clone());

        let actual_limit = request.limit.map_or(100, |l| l.min(300));
        params.insert("limit".to_string(), actual_limit.to_string());

        if let Some(start_time) = request.since {
            params.insert("after".to_string(), start_time.to_string());
        }

        if let Some(end_time) = request.until {
            params.insert("before".to_string(), end_time.to_string());
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

        ohlcv.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));

        Ok(ohlcv)
    }

    /// Fetch OHLCV (candlestick) data (deprecated).
    ///
    /// # Deprecated
    ///
    /// This method is deprecated. Use [`fetch_ohlcv_v2`](Self::fetch_ohlcv_v2) with
    /// [`OhlcvRequest::builder()`] instead for a more ergonomic API.
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
    #[deprecated(
        since = "0.2.0",
        note = "Use fetch_ohlcv_v2 with OhlcvRequest::builder() instead"
    )]
    pub async fn fetch_ohlcv(
        &self,
        symbol: &str,
        timeframe: &str,
        since: Option<i64>,
        limit: Option<u32>,
    ) -> Result<Vec<OHLCV>> {
        let market = self.base().market(symbol).await?;

        let timeframes = self.timeframes();
        let okx_timeframe = timeframes.get(timeframe).ok_or_else(|| {
            Error::invalid_request(format!("Unsupported timeframe: {}", timeframe))
        })?;

        let path = Self::build_api_path("/market/candles");
        let mut params = HashMap::new();
        params.insert("instId".to_string(), market.id.clone());
        params.insert("bar".to_string(), okx_timeframe.clone());

        let actual_limit = limit.map_or(100, |l| l.min(300));
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

        ohlcv.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));

        Ok(ohlcv)
    }
}
