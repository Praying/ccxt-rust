//! Binance account operations.
//!
//! This module contains all account-related methods including balance,
//! trade history, account configuration, and user data stream management.

use super::super::{Binance, parser};
use ccxt_core::{
    Error, ParseError, Result,
    types::{Balance, Currency, FeeTradingFee, MarketType, Trade},
};
use reqwest::header::HeaderMap;
use std::collections::HashMap;
use tracing::warn;

impl Binance {
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

    /// Fetch account balance with optional account type parameter.
    ///
    /// # Arguments
    ///
    /// * `account_type` - Optional account type (e.g., "spot", "margin", "futures").
    ///
    /// # Returns
    ///
    /// Returns the account [`Balance`] information.
    ///
    /// # Errors
    ///
    /// Returns an error if authentication fails or the API request fails.
    pub async fn fetch_balance(&self, _account_type: Option<&str>) -> Result<Balance> {
        // For now, delegate to the simple implementation
        // TODO: Add support for different account types (margin, futures)
        self.fetch_balance_simple().await
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
        since: Option<i64>,
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

    /// Fetch user's recent trade history with additional parameters.
    ///
    /// This method is similar to `fetch_my_trades` but accepts additional parameters
    /// for more flexible querying.
    ///
    /// # Arguments
    ///
    /// * `symbol` - Trading pair symbol.
    /// * `since` - Optional start timestamp in milliseconds.
    /// * `limit` - Optional limit on number of trades (default: 500, max: 1000).
    /// * `params` - Optional additional parameters that may include:
    ///   - `orderId`: Filter by order ID.
    ///   - `fromId`: Start from specific trade ID.
    ///   - `endTime`: End timestamp in milliseconds.
    ///
    /// # Returns
    ///
    /// Returns a vector of [`Trade`] structures for the user's trades.
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
    /// let my_trades = binance.fetch_my_recent_trades("BTC/USDT", None, Some(50), None).await?;
    /// for trade in &my_trades {
    ///     println!("Trade: {} {} @ {}", trade.side, trade.amount, trade.price);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn fetch_my_recent_trades(
        &self,
        symbol: &str,
        since: Option<i64>,
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

    /// Fetch all currency information.
    ///
    /// # Returns
    ///
    /// Returns a vector of [`Currency`] structures.
    ///
    /// # Errors
    ///
    /// Returns an error if authentication fails or the API request fails.
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

    /// Fetch trading fees for a symbol.
    ///
    /// # Arguments
    ///
    /// * `symbol` - Trading pair symbol.
    /// * `params` - Optional parameters. Supports `portfolioMargin` key for Portfolio Margin mode.
    ///
    /// # Returns
    ///
    /// Returns trading fee information for the symbol as a [`FeeTradingFee`] structure.
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

    /// Fetch trading fees for multiple symbols.
    ///
    /// # Arguments
    ///
    /// * `symbols` - Optional list of trading pair symbols. `None` fetches all pairs.
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
    /// for (symbol, fee) in &fees {
    ///     println!("{}: maker={}, taker={}", symbol, fee.maker, fee.taker);
    /// }
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

    /// Create a listen key for user data stream.
    ///
    /// Creates a new listen key that can be used to subscribe to user data streams
    /// via WebSocket. The listen key is valid for 60 minutes.
    ///
    /// # Returns
    ///
    /// Returns the listen key string.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - API credentials are not configured
    /// - API request fails
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

    /// Refresh listen key to extend validity.
    ///
    /// Extends the listen key validity by 60 minutes. Recommended to call
    /// every 30 minutes to maintain the connection.
    ///
    /// # Arguments
    ///
    /// * `listen_key` - The listen key to refresh.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on success.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - API credentials are not configured
    /// - Listen key is invalid or expired
    /// - API request fails
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

    /// Delete listen key to close user data stream.
    ///
    /// Closes the user data stream connection and invalidates the listen key.
    ///
    /// # Arguments
    ///
    /// * `listen_key` - The listen key to delete.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on success.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - API credentials are not configured
    /// - API request fails
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
}
