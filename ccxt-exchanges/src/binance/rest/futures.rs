//! Binance perpetual futures (FAPI) operations.
//!
//! This module contains all FAPI (USDT-margined perpetual futures) specific methods
//! including position management, leverage, funding rates, and position mode.

use super::super::{Binance, parser};
use ccxt_core::{
    Error, ParseError, Result,
    types::{FeeFundingRate, FeeFundingRateHistory, LeverageTier, MarketType, Position},
};
use reqwest::header::HeaderMap;
use rust_decimal::Decimal;
use serde_json::Value;
use std::collections::HashMap;
use tracing::warn;

impl Binance {
    // ==================== Position Methods ====================

    /// Fetch a single position for a trading pair.
    ///
    /// # Arguments
    ///
    /// * `symbol` - Trading pair symbol (e.g., "BTC/USDT:USDT").
    /// * `params` - Optional additional parameters.
    ///
    /// # Returns
    ///
    /// Returns a [`Position`] structure for the specified symbol.
    ///
    /// # Errors
    ///
    /// Returns an error if authentication fails, the market is not found,
    /// or the API request fails.
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
    /// let position = binance.fetch_position("BTC/USDT:USDT", None).await?;
    /// println!("Position: {:?}", position);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn fetch_position(&self, symbol: &str, params: Option<Value>) -> Result<Position> {
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
    ) -> Result<Vec<Position>> {
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
    ) -> Result<Vec<Position>> {
        self.fetch_positions(symbols, params).await
    }

    /// Fetch position risk information (raw JSON).
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
    /// Returns position risk information as raw JSON.
    ///
    /// # Errors
    ///
    /// Returns an error if authentication fails or the API request fails.
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

    // ==================== Leverage Methods ====================

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

        let mut tiers_map: HashMap<String, Vec<LeverageTier>> = HashMap::new();

        // Response can be array of symbols with brackets
        if let Some(symbols_array) = response.as_array() {
            for symbol_data in symbols_array {
                if let (Some(symbol_id), Some(brackets)) = (
                    symbol_data["symbol"].as_str(),
                    symbol_data["brackets"].as_array(),
                ) {
                    // Try to get the market from cache
                    if let Ok(market) = self.base().market_by_id(symbol_id).await {
                        let mut tier_list = Vec::new();
                        for bracket in brackets {
                            if let Ok(tier) = parser::parse_leverage_tier(bracket, &market) {
                                tier_list.push(tier);
                            }
                        }

                        if !tier_list.is_empty() {
                            // Filter by requested symbols if provided
                            if let Some(ref filter_symbols) = symbols {
                                if filter_symbols.contains(&market.symbol) {
                                    tiers_map.insert(market.symbol.clone(), tier_list);
                                }
                            } else {
                                tiers_map.insert(market.symbol.clone(), tier_list);
                            }
                        }
                    }
                }
            }
        }

        Ok(tiers_map)
    }

    // ==================== Margin Mode Methods ====================

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

    // ==================== Position Mode Methods ====================

    /// Set position mode (hedge mode or one-way mode).
    ///
    /// # Arguments
    ///
    /// * `dual_side` - `true` for hedge mode (dual-side position), `false` for one-way mode.
    /// * `params` - Optional parameters.
    ///
    /// # Returns
    ///
    /// Returns the API response.
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

    // ==================== Funding Rate Methods ====================

    /// Fetch current funding rate for a trading pair.
    ///
    /// # Arguments
    ///
    /// * `symbol` - Trading pair symbol (e.g., "BTC/USDT:USDT").
    /// * `params` - Optional parameters.
    ///
    /// # Returns
    ///
    /// Returns the current funding rate information.
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

    /// Fetch funding payment history for a trading pair.
    ///
    /// # Arguments
    ///
    /// * `symbol` - Optional trading pair symbol. `None` returns all.
    /// * `since` - Optional start timestamp in milliseconds.
    /// * `limit` - Optional record limit.
    /// * `params` - Optional parameters.
    ///
    /// # Returns
    ///
    /// Returns funding payment history as raw JSON.
    ///
    /// # Errors
    ///
    /// Returns an error if authentication fails or the API request fails.
    pub async fn fetch_funding_history(
        &self,
        symbol: Option<&str>,
        since: Option<u64>,
        limit: Option<u32>,
        params: Option<HashMap<String, String>>,
    ) -> Result<Value> {
        self.check_required_credentials()?;
        self.load_markets(false).await?;

        let mut request_params = HashMap::new();

        if let Some(sym) = symbol {
            let market = self.base().market(sym).await?;
            request_params.insert("symbol".to_string(), market.id.clone());
        }

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

        let timestamp = self.fetch_time_raw().await?;
        let auth = self.get_auth()?;
        let signed_params =
            auth.sign_with_timestamp(&request_params, timestamp, Some(self.options().recv_window))?;

        let query_string: String = signed_params
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<_>>()
            .join("&");

        let url = format!("{}/income?{}", self.urls().fapi_private, query_string);

        let mut headers = HeaderMap::new();
        auth.add_auth_headers_reqwest(&mut headers);

        let data = self.base().http_client.get(&url, Some(headers)).await?;

        Ok(data)
    }
}
