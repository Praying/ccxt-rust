//! Market data endpoints for Bitget REST API.

use super::super::{Bitget, parser};
use ccxt_core::{
    Error, ParseError, Result,
    types::{Market, OHLCV, OhlcvRequest, OrderBook, Ticker, Trade},
};
use std::{collections::HashMap, sync::Arc};
use tracing::{info, warn};

impl Bitget {
    /// Fetch all trading markets.
    pub async fn fetch_markets(&self) -> Result<Arc<HashMap<String, Arc<Market>>>> {
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

        let result = self.base().set_markets(markets, None).await?;
        info!("Loaded {} markets for Bitget", result.len());
        Ok(result)
    }

    /// Load and cache market data.
    pub async fn load_markets(&self, reload: bool) -> Result<Arc<HashMap<String, Arc<Market>>>> {
        let _loading_guard = self.base().market_loading_lock.lock().await;

        {
            let cache = self.base().market_cache.read().await;
            if cache.loaded && !reload {
                return Ok(cache.markets.clone());
            }
        }

        info!("Loading markets for Bitget (reload: {})", reload);
        let _markets = self.fetch_markets().await?;

        let cache = self.base().market_cache.read().await;
        Ok(cache.markets.clone())
    }

    /// Fetch ticker for a single trading pair.
    pub async fn fetch_ticker(&self, symbol: &str) -> Result<Ticker> {
        let market = self.base().market(symbol).await?;

        let path = self.build_api_path("/market/tickers");
        let mut params = HashMap::new();
        params.insert("symbol".to_string(), market.id.clone());

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

    /// Fetch order book for a trading pair.
    pub async fn fetch_order_book(&self, symbol: &str, limit: Option<u32>) -> Result<OrderBook> {
        let market = self.base().market(symbol).await?;

        let path = self.build_api_path("/market/orderbook");
        let mut params = HashMap::new();
        params.insert("symbol".to_string(), market.id.clone());

        let actual_limit = limit.map_or(100, |l| l.min(100));
        params.insert("limit".to_string(), actual_limit.to_string());

        let response = self.public_request("GET", &path, Some(&params)).await?;

        let data = response
            .get("data")
            .ok_or_else(|| Error::from(ParseError::missing_field("data")))?;

        parser::parse_orderbook(data, market.symbol.clone())
    }

    /// Fetch recent public trades.
    pub async fn fetch_trades(&self, symbol: &str, limit: Option<u32>) -> Result<Vec<Trade>> {
        let market = self.base().market(symbol).await?;

        let path = self.build_api_path("/market/fills");
        let mut params = HashMap::new();
        params.insert("symbol".to_string(), market.id.clone());

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

    /// Fetch OHLCV (candlestick) data.
    pub async fn fetch_ohlcv_v2(&self, request: OhlcvRequest) -> Result<Vec<OHLCV>> {
        let market = self.base().market(&request.symbol).await?;

        let timeframes = self.timeframes();
        let bitget_timeframe = timeframes.get(&request.timeframe).ok_or_else(|| {
            Error::invalid_request(format!("Unsupported timeframe: {}", request.timeframe))
        })?;

        let path = self.build_api_path("/market/candles");
        let mut params = HashMap::new();
        params.insert("symbol".to_string(), market.id.clone());
        params.insert("granularity".to_string(), bitget_timeframe.clone());

        let actual_limit = request.limit.map_or(100, |l| l.min(1000));
        params.insert("limit".to_string(), actual_limit.to_string());

        if let Some(start_time) = request.since {
            params.insert("startTime".to_string(), start_time.to_string());
        }

        if let Some(end_time) = request.until {
            params.insert("endTime".to_string(), end_time.to_string());
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
        let bitget_timeframe = timeframes.get(timeframe).ok_or_else(|| {
            Error::invalid_request(format!("Unsupported timeframe: {}", timeframe))
        })?;

        let path = self.build_api_path("/market/candles");
        let mut params = HashMap::new();
        params.insert("symbol".to_string(), market.id.clone());
        params.insert("granularity".to_string(), bitget_timeframe.clone());

        let actual_limit = limit.map_or(100, |l| l.min(1000));
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

        ohlcv.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));
        Ok(ohlcv)
    }
}
