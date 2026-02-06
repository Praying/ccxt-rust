//! Binance account operations.
//!
//! This module contains all account-related methods including balance,
//! trade history, account configuration, and user data stream management.

use super::super::{Binance, BinanceEndpointRouter, parser};
use ccxt_core::types::AccountType;
use ccxt_core::{
    Error, ParseError, Result,
    types::{Balance, Currency, EndpointType, FeeTradingFee, MarketType, Trade},
};
use reqwest::header::HeaderMap;
use std::collections::HashMap;
use tracing::warn;

/// Balance fetch parameters for Binance.
#[derive(Debug, Clone, Default)]
pub struct BalanceFetchParams {
    /// Account type to query (spot, margin, futures, etc.).
    pub account_type: Option<AccountType>,
    /// Margin mode: "cross" or "isolated".
    pub margin_mode: Option<String>,
    /// Symbols for isolated margin (e.g., `["BTC/USDT", "ETH/USDT"]`).
    pub symbols: Option<Vec<String>>,
    /// Whether to use Portfolio Margin API.
    pub portfolio_margin: bool,
    /// Sub-type for futures: "linear" or "inverse".
    pub sub_type: Option<String>,
}

impl BalanceFetchParams {
    /// Create params for spot balance.
    pub fn spot() -> Self {
        Self {
            account_type: Some(AccountType::Spot),
            ..Default::default()
        }
    }

    /// Create params for cross margin balance.
    pub fn cross_margin() -> Self {
        Self {
            account_type: Some(AccountType::Margin),
            margin_mode: Some("cross".to_string()),
            ..Default::default()
        }
    }

    /// Create params for isolated margin balance.
    pub fn isolated_margin(symbols: Option<Vec<String>>) -> Self {
        Self {
            account_type: Some(AccountType::IsolatedMargin),
            margin_mode: Some("isolated".to_string()),
            symbols,
            ..Default::default()
        }
    }

    /// Create params for USDT-margined futures (linear).
    pub fn linear_futures() -> Self {
        Self {
            account_type: Some(AccountType::Futures),
            sub_type: Some("linear".to_string()),
            ..Default::default()
        }
    }

    /// Create params for coin-margined futures (inverse/delivery).
    pub fn inverse_futures() -> Self {
        Self {
            account_type: Some(AccountType::Delivery),
            sub_type: Some("inverse".to_string()),
            ..Default::default()
        }
    }

    /// Create params for funding wallet.
    pub fn funding() -> Self {
        Self {
            account_type: Some(AccountType::Funding),
            ..Default::default()
        }
    }

    /// Create params for options account.
    pub fn option() -> Self {
        Self {
            account_type: Some(AccountType::Option),
            ..Default::default()
        }
    }

    /// Create params for portfolio margin.
    pub fn portfolio_margin() -> Self {
        Self {
            portfolio_margin: true,
            ..Default::default()
        }
    }
}

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
        let url = format!("{}/account", self.urls().private);
        let data = self.signed_request(url).execute().await?;
        parser::parse_balance(&data)
    }

    /// Fetch account balance with optional account type parameter.
    ///
    /// Query for balance and get the amount of funds available for trading or funds locked in orders.
    ///
    /// # Arguments
    ///
    /// * `account_type` - Optional account type. Supported values:
    ///   - `Spot` - Spot trading account (default)
    ///   - `Margin` - Cross margin account
    ///   - `IsolatedMargin` - Isolated margin account
    ///   - `Futures` - USDT-margined futures (linear)
    ///   - `Delivery` - Coin-margined futures (inverse)
    ///   - `Funding` - Funding wallet
    ///   - `Option` - Options account
    ///
    /// # Returns
    ///
    /// Returns the account [`Balance`] information.
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
    /// # use ccxt_core::types::AccountType;
    /// # async fn example() -> ccxt_core::Result<()> {
    /// let mut config = ExchangeConfig::default();
    /// config.api_key = Some("your_api_key".to_string());
    /// config.secret = Some("your_secret".to_string());
    /// let binance = Binance::new(config)?;
    ///
    /// // Fetch spot balance (default)
    /// let balance = binance.fetch_balance(None).await?;
    ///
    /// // Fetch futures balance
    /// let futures_balance = binance.fetch_balance(Some(AccountType::Futures)).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn fetch_balance(&self, account_type: Option<AccountType>) -> Result<Balance> {
        let params = BalanceFetchParams {
            account_type,
            ..Default::default()
        };
        self.fetch_balance_with_params(params).await
    }

    /// Fetch account balance with detailed parameters.
    ///
    /// This method provides full control over balance fetching, supporting all Binance account types
    /// and margin modes.
    ///
    /// # Arguments
    ///
    /// * `params` - Balance fetch parameters including:
    ///   - `account_type` - Account type (spot, margin, futures, etc.)
    ///   - `margin_mode` - "cross" or "isolated" for margin trading
    ///   - `symbols` - Symbols for isolated margin queries
    ///   - `portfolio_margin` - Use Portfolio Margin API
    ///   - `sub_type` - "linear" or "inverse" for futures
    ///
    /// # Returns
    ///
    /// Returns the account [`Balance`] information.
    ///
    /// # Errors
    ///
    /// Returns an error if authentication fails or the API request fails.
    ///
    /// # API Endpoints
    ///
    /// - Spot: `GET /api/v3/account`
    /// - Cross Margin: `GET /sapi/v1/margin/account`
    /// - Isolated Margin: `GET /sapi/v1/margin/isolated/account`
    /// - Funding Wallet: `POST /sapi/v1/asset/get-funding-asset`
    /// - USDT-M Futures: `GET /fapi/v2/balance`
    /// - COIN-M Futures: `GET /dapi/v1/balance`
    /// - Options: `GET /eapi/v1/account`
    /// - Portfolio Margin: `GET /papi/v1/balance`
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use ccxt_exchanges::binance::Binance;
    /// # use ccxt_exchanges::binance::rest::account::BalanceFetchParams;
    /// # use ccxt_core::ExchangeConfig;
    /// # async fn example() -> ccxt_core::Result<()> {
    /// let mut config = ExchangeConfig::default();
    /// config.api_key = Some("your_api_key".to_string());
    /// config.secret = Some("your_secret".to_string());
    /// let binance = Binance::new(config)?;
    ///
    /// // Fetch cross margin balance
    /// let margin_balance = binance.fetch_balance_with_params(
    ///     BalanceFetchParams::cross_margin()
    /// ).await?;
    ///
    /// // Fetch isolated margin balance for specific symbols
    /// let isolated_balance = binance.fetch_balance_with_params(
    ///     BalanceFetchParams::isolated_margin(Some(vec!["BTC/USDT".to_string()]))
    /// ).await?;
    ///
    /// // Fetch USDT-margined futures balance
    /// let futures_balance = binance.fetch_balance_with_params(
    ///     BalanceFetchParams::linear_futures()
    /// ).await?;
    ///
    /// // Fetch portfolio margin balance
    /// let pm_balance = binance.fetch_balance_with_params(
    ///     BalanceFetchParams::portfolio_margin()
    /// ).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn fetch_balance_with_params(&self, params: BalanceFetchParams) -> Result<Balance> {
        // Handle portfolio margin first
        if params.portfolio_margin {
            return self.fetch_portfolio_margin_balance().await;
        }

        let account_type = params.account_type.unwrap_or(AccountType::Spot);

        match account_type {
            AccountType::Spot => self.fetch_spot_balance().await,
            AccountType::Margin => {
                let margin_mode = params.margin_mode.as_deref().unwrap_or("cross");
                if margin_mode == "isolated" {
                    self.fetch_isolated_margin_balance(params.symbols).await
                } else {
                    self.fetch_cross_margin_balance().await
                }
            }
            AccountType::IsolatedMargin => self.fetch_isolated_margin_balance(params.symbols).await,
            AccountType::Futures => {
                let sub_type = params.sub_type.as_deref().unwrap_or("linear");
                if sub_type == "inverse" {
                    self.fetch_delivery_balance().await
                } else {
                    self.fetch_futures_balance().await
                }
            }
            AccountType::Delivery => self.fetch_delivery_balance().await,
            AccountType::Funding => self.fetch_funding_balance().await,
            AccountType::Option => self.fetch_option_balance().await,
        }
    }

    /// Fetch spot account balance.
    ///
    /// Uses the `/api/v3/account` endpoint.
    async fn fetch_spot_balance(&self) -> Result<Balance> {
        let url = format!("{}/account", self.urls().private);
        let data = self.signed_request(url).execute().await?;
        parser::parse_balance_with_type(&data, "spot")
    }

    /// Fetch cross margin account balance.
    ///
    /// Uses the `/sapi/v1/margin/account` endpoint.
    async fn fetch_cross_margin_balance(&self) -> Result<Balance> {
        let url = format!("{}/margin/account", self.urls().sapi);
        let data = self.signed_request(url).execute().await?;
        parser::parse_balance_with_type(&data, "margin")
    }

    /// Fetch isolated margin account balance.
    ///
    /// Uses the `/sapi/v1/margin/isolated/account` endpoint.
    ///
    /// # Arguments
    ///
    /// * `symbols` - Optional list of symbols to query. If None, fetches all isolated margin pairs.
    async fn fetch_isolated_margin_balance(&self, symbols: Option<Vec<String>>) -> Result<Balance> {
        let url = format!("{}/margin/isolated/account", self.urls().sapi);

        let mut request = self.signed_request(url);

        // Add symbols parameter if provided
        if let Some(syms) = symbols {
            // Convert unified symbols to market IDs
            let mut market_ids = Vec::new();
            for sym in &syms {
                if let Ok(market) = self.base().market(sym).await {
                    market_ids.push(market.id.clone());
                } else {
                    // If market not found, use the symbol as-is (might be already in exchange format)
                    market_ids.push(sym.replace('/', ""));
                }
            }
            if !market_ids.is_empty() {
                request = request.param("symbols", market_ids.join(","));
            }
        }

        let data = request.execute().await?;
        parser::parse_balance_with_type(&data, "isolated")
    }

    /// Fetch funding wallet balance.
    ///
    /// Uses the `/sapi/v1/asset/get-funding-asset` endpoint.
    async fn fetch_funding_balance(&self) -> Result<Balance> {
        use super::super::signed_request::HttpMethod;

        let url = format!("{}/asset/get-funding-asset", self.urls().sapi);
        let data = self
            .signed_request(url)
            .method(HttpMethod::Post)
            .execute()
            .await?;
        parser::parse_balance_with_type(&data, "funding")
    }

    /// Fetch USDT-margined futures balance.
    ///
    /// Uses the `/fapi/v2/balance` endpoint.
    async fn fetch_futures_balance(&self) -> Result<Balance> {
        // Use v2 endpoint for better data
        let url = format!("{}/balance", self.urls().fapi_private.replace("/v1", "/v2"));
        let data = self.signed_request(url).execute().await?;
        parser::parse_balance_with_type(&data, "linear")
    }

    /// Fetch coin-margined futures (delivery) balance.
    ///
    /// Uses the `/dapi/v1/balance` endpoint.
    async fn fetch_delivery_balance(&self) -> Result<Balance> {
        let url = format!("{}/balance", self.urls().dapi_private);
        let data = self.signed_request(url).execute().await?;
        parser::parse_balance_with_type(&data, "inverse")
    }

    /// Fetch options account balance.
    ///
    /// Uses the `/eapi/v1/account` endpoint.
    async fn fetch_option_balance(&self) -> Result<Balance> {
        let url = format!("{}/account", self.urls().eapi_private);
        let data = self.signed_request(url).execute().await?;
        parser::parse_balance_with_type(&data, "option")
    }

    /// Fetch portfolio margin account balance.
    ///
    /// Uses the `/papi/v1/balance` endpoint.
    async fn fetch_portfolio_margin_balance(&self) -> Result<Balance> {
        let url = format!("{}/balance", self.urls().papi);
        let data = self.signed_request(url).execute().await?;
        parser::parse_balance_with_type(&data, "portfolio")
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
        let market = self.base().market(symbol).await?;
        // Use market-type-aware endpoint routing
        let base_url = self.rest_endpoint(&market, EndpointType::Private);
        let url = format!("{}/myTrades", base_url);

        let data = self
            .signed_request(url)
            .param("symbol", &market.id)
            .optional_param("startTime", since)
            .optional_param("limit", limit)
            .execute()
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
        // Use market-type-aware endpoint routing
        let base_url = self.rest_endpoint(&market, EndpointType::Private);
        let url = format!("{}/myTrades", base_url);

        // Convert HashMap to serde_json::Value for merge_json_params
        let json_params = params.map(|p| {
            serde_json::Value::Object(
                p.into_iter()
                    .map(|(k, v)| (k, serde_json::Value::String(v)))
                    .collect(),
            )
        });

        let data = self
            .signed_request(url)
            .param("symbol", &market.id)
            .optional_param("startTime", since)
            .optional_param("limit", limit)
            .merge_json_params(json_params)
            .execute()
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
        let data = self.signed_request(url).execute().await?;
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
        self.load_markets(false).await?;
        let market = self.base().market(symbol).await?;

        let is_portfolio_margin = params
            .as_ref()
            .and_then(|p| p.get("portfolioMargin"))
            .is_some_and(|v| v == "true");

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
            MarketType::Option => {
                return Err(Error::invalid_request(format!(
                    "fetch_trading_fee not supported for market type: {:?}",
                    market.market_type
                )));
            }
        };

        // Convert HashMap to serde_json::Value, filtering out portfolioMargin
        let json_params = params.map(|p| {
            serde_json::Value::Object(
                p.into_iter()
                    .filter(|(k, _)| k != "portfolioMargin")
                    .map(|(k, v)| (k, serde_json::Value::String(v)))
                    .collect(),
            )
        });

        let response = self
            .signed_request(url)
            .param("symbol", &market.id)
            .merge_json_params(json_params)
            .execute()
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
        self.load_markets(false).await?;

        let url = format!("{}/asset/tradeFee", self.urls().sapi);

        // Build symbols parameter if provided
        let symbols_param = if let Some(syms) = &symbols {
            let mut market_ids: Vec<String> = Vec::new();
            for s in syms {
                if let Ok(market) = self.base().market(s).await {
                    market_ids.push(market.id.clone());
                }
            }
            if market_ids.is_empty() {
                None
            } else {
                Some(market_ids.join(","))
            }
        } else {
            None
        };

        // Convert HashMap to serde_json::Value for merge_json_params
        let json_params = params.map(|p| {
            serde_json::Value::Object(
                p.into_iter()
                    .map(|(k, v)| (k, serde_json::Value::String(v)))
                    .collect(),
            )
        });

        let response = self
            .signed_request(url)
            .optional_param("symbols", symbols_param)
            .merge_json_params(json_params)
            .execute()
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

    /// Create a listen key for user data stream (spot market).
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
        self.create_listen_key_for_market(None).await
    }

    /// Create a listen key for user data stream with market type routing.
    ///
    /// Creates a new listen key for the specified market type. Different market types
    /// use different API endpoints:
    /// - Spot: `/api/v3/userDataStream`
    /// - USDT-M Futures: `/fapi/v1/listenKey`
    /// - COIN-M Futures: `/dapi/v1/listenKey`
    /// - Options: `/eapi/v1/listenKey`
    ///
    /// # Arguments
    ///
    /// * `market_type` - Optional market type. If None, defaults to Spot.
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
    /// # use ccxt_core::types::MarketType;
    /// # async fn example() -> ccxt_core::Result<()> {
    /// let mut config = ExchangeConfig::default();
    /// config.api_key = Some("your_api_key".to_string());
    /// config.secret = Some("your_secret".to_string());
    /// let binance = Binance::new(config)?;
    ///
    /// // Create listen key for futures
    /// let listen_key = binance.create_listen_key_for_market(Some(MarketType::Swap)).await?;
    /// println!("Futures Listen Key: {}", listen_key);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn create_listen_key_for_market(
        &self,
        market_type: Option<MarketType>,
    ) -> Result<String> {
        self.check_required_credentials()?;

        let url = self.get_listen_key_url(market_type, None);
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
            .map(ToString::to_string)
            .ok_or_else(|| Error::from(ParseError::missing_field("listenKey")))
    }

    /// Refresh listen key to extend validity (spot market).
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
        self.refresh_listen_key_for_market(listen_key, None).await
    }

    /// Refresh listen key with market type routing.
    ///
    /// Extends the listen key validity by 60 minutes for the specified market type.
    ///
    /// # Arguments
    ///
    /// * `listen_key` - The listen key to refresh.
    /// * `market_type` - Optional market type. If None, defaults to Spot.
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
    pub async fn refresh_listen_key_for_market(
        &self,
        listen_key: &str,
        market_type: Option<MarketType>,
    ) -> Result<()> {
        self.check_required_credentials()?;

        let url = self.get_listen_key_url(market_type, Some(listen_key));
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

    /// Delete listen key to close user data stream (spot market).
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
        self.delete_listen_key_for_market(listen_key, None).await
    }

    /// Delete listen key with market type routing.
    ///
    /// Closes the user data stream connection for the specified market type.
    ///
    /// # Arguments
    ///
    /// * `listen_key` - The listen key to delete.
    /// * `market_type` - Optional market type. If None, defaults to Spot.
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
    pub async fn delete_listen_key_for_market(
        &self,
        listen_key: &str,
        market_type: Option<MarketType>,
    ) -> Result<()> {
        self.check_required_credentials()?;

        let url = self.get_listen_key_url(market_type, Some(listen_key));
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

    /// Get the listen key URL for the specified market type.
    ///
    /// # Arguments
    ///
    /// * `market_type` - Optional market type. If None, defaults to Spot.
    /// * `listen_key` - Optional listen key to include in the URL (for refresh/delete).
    ///
    /// # Returns
    ///
    /// Returns the appropriate URL for the listen key operation.
    fn get_listen_key_url(
        &self,
        market_type: Option<MarketType>,
        listen_key: Option<&str>,
    ) -> String {
        let urls = self.urls();
        let (base_url, endpoint) = match market_type {
            Some(MarketType::Swap | MarketType::Futures) => {
                // Check if it's linear (USDT-M) or inverse (COIN-M) based on default_type
                // For now, default to linear (fapi) for Swap/Futures
                (urls.fapi_public.clone(), "listenKey")
            }
            Some(MarketType::Option) => (urls.eapi_public.clone(), "listenKey"),
            // Spot is the default
            None | Some(MarketType::Spot) => (urls.public.clone(), "userDataStream"),
        };

        match listen_key {
            Some(key) => format!("{}/{}?listenKey={}", base_url, endpoint, key),
            None => format!("{}/{}", base_url, endpoint),
        }
    }

    /// Get the listen key URL for linear (USDT-M) futures.
    ///
    /// # Arguments
    ///
    /// * `listen_key` - Optional listen key to include in the URL.
    ///
    /// # Returns
    ///
    /// Returns the URL for linear futures listen key operations.
    pub fn get_linear_listen_key_url(&self, listen_key: Option<&str>) -> String {
        let base_url = &self.urls().fapi_public;
        match listen_key {
            Some(key) => format!("{}/listenKey?listenKey={}", base_url, key),
            None => format!("{}/listenKey", base_url),
        }
    }

    /// Get the listen key URL for inverse (COIN-M) futures.
    ///
    /// # Arguments
    ///
    /// * `listen_key` - Optional listen key to include in the URL.
    ///
    /// # Returns
    ///
    /// Returns the URL for inverse futures listen key operations.
    pub fn get_inverse_listen_key_url(&self, listen_key: Option<&str>) -> String {
        let base_url = &self.urls().dapi_public;
        match listen_key {
            Some(key) => format!("{}/listenKey?listenKey={}", base_url, key),
            None => format!("{}/listenKey", base_url),
        }
    }

    /// Create a listen key for linear (USDT-M) futures.
    ///
    /// # Returns
    ///
    /// Returns the listen key string.
    ///
    /// # Errors
    ///
    /// Returns an error if API credentials are not configured or the request fails.
    pub async fn create_linear_listen_key(&self) -> Result<String> {
        self.check_required_credentials()?;

        let url = self.get_linear_listen_key_url(None);
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
            .map(ToString::to_string)
            .ok_or_else(|| Error::from(ParseError::missing_field("listenKey")))
    }

    /// Create a listen key for inverse (COIN-M) futures.
    ///
    /// # Returns
    ///
    /// Returns the listen key string.
    ///
    /// # Errors
    ///
    /// Returns an error if API credentials are not configured or the request fails.
    pub async fn create_inverse_listen_key(&self) -> Result<String> {
        self.check_required_credentials()?;

        let url = self.get_inverse_listen_key_url(None);
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
            .map(ToString::to_string)
            .ok_or_else(|| Error::from(ParseError::missing_field("listenKey")))
    }

    /// Refresh a linear (USDT-M) futures listen key.
    ///
    /// # Arguments
    ///
    /// * `listen_key` - The listen key to refresh.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on success.
    pub async fn refresh_linear_listen_key(&self, listen_key: &str) -> Result<()> {
        self.check_required_credentials()?;

        let url = self.get_linear_listen_key_url(Some(listen_key));
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

    /// Refresh an inverse (COIN-M) futures listen key.
    ///
    /// # Arguments
    ///
    /// * `listen_key` - The listen key to refresh.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on success.
    pub async fn refresh_inverse_listen_key(&self, listen_key: &str) -> Result<()> {
        self.check_required_credentials()?;

        let url = self.get_inverse_listen_key_url(Some(listen_key));
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

    /// Delete a linear (USDT-M) futures listen key.
    ///
    /// # Arguments
    ///
    /// * `listen_key` - The listen key to delete.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on success.
    pub async fn delete_linear_listen_key(&self, listen_key: &str) -> Result<()> {
        self.check_required_credentials()?;

        let url = self.get_linear_listen_key_url(Some(listen_key));
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

    /// Delete an inverse (COIN-M) futures listen key.
    ///
    /// # Arguments
    ///
    /// * `listen_key` - The listen key to delete.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on success.
    pub async fn delete_inverse_listen_key(&self, listen_key: &str) -> Result<()> {
        self.check_required_credentials()?;

        let url = self.get_inverse_listen_key_url(Some(listen_key));
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
