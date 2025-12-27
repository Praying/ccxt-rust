//! Binance delivery futures (DAPI) operations.
//!
//! This module provides DAPI (coin-margined delivery futures) specific methods
//! that either don't have direct equivalents in the unified `futures` module
//! or require DAPI-specific implementations.
//!
//! # Overview
//!
//! The DAPI (Delivery API) is Binance's API for coin-margined futures contracts,
//! where contracts are settled in the base currency (e.g., BTC/USD:BTC is settled in BTC).
//! This differs from FAPI (Futures API) which handles USDT-margined contracts.
//!
//! # DAPI vs FAPI Differences
//!
//! | Feature | FAPI (Linear) | DAPI (Inverse) |
//! |---------|---------------|----------------|
//! | Settlement | USDT | Base currency (BTC, ETH, etc.) |
//! | Symbol format | BTC/USDT:USDT | BTC/USD:BTC |
//! | Base URL | /fapi/v1/* | /dapi/v1/* |
//! | Margin type | Quote currency | Base currency |
//! | PnL calculation | In USDT | In base currency |
//!
//! # Automatic Routing
//!
//! Most DAPI operations are handled automatically by the methods in the `futures` module.
//! When you call methods like `fetch_position`, `set_leverage`, or `fetch_funding_rate`
//! with a coin-margined symbol (e.g., "BTC/USD:BTC"), the implementation automatically
//! routes to the appropriate DAPI endpoint based on the market's `inverse` flag.
//!
//! # Available Methods
//!
//! This module provides the following DAPI-specific methods:
//!
//! | Method | Description | Endpoint |
//! |--------|-------------|----------|
//! | [`set_position_mode_dapi`](Binance::set_position_mode_dapi) | Set hedge/one-way mode | `/dapi/v1/positionSide/dual` |
//! | [`fetch_position_mode_dapi`](Binance::fetch_position_mode_dapi) | Get current position mode | `/dapi/v1/positionSide/dual` |
//! | [`fetch_dapi_account`](Binance::fetch_dapi_account) | Get account information | `/dapi/v1/account` |
//! | [`fetch_dapi_income`](Binance::fetch_dapi_income) | Get income history | `/dapi/v1/income` |
//! | [`fetch_dapi_commission_rate`](Binance::fetch_dapi_commission_rate) | Get commission rates | `/dapi/v1/commissionRate` |
//! | [`fetch_dapi_adl_quantile`](Binance::fetch_dapi_adl_quantile) | Get ADL quantile | `/dapi/v1/adlQuantile` |
//! | [`fetch_dapi_force_orders`](Binance::fetch_dapi_force_orders) | Get force liquidation orders | `/dapi/v1/forceOrders` |
//!
//! # DAPI-Specific vs Unified Methods
//!
//! - **DAPI-specific**: Methods in this module (prefixed with `dapi_` or suffixed with `_dapi`)
//!   are exclusively for coin-margined futures and use DAPI endpoints directly.
//! - **Unified**: Methods in the `futures` module automatically route to FAPI or DAPI
//!   based on the symbol's market type (linear vs inverse).
//!
//! # Authentication
//!
//! All methods in this module require authentication. You must provide valid API credentials
//! when creating the Binance instance.
//!
//! # Example
//!
//! ```no_run
//! # use ccxt_exchanges::binance::Binance;
//! # use ccxt_core::ExchangeConfig;
//! # async fn example() -> ccxt_core::Result<()> {
//! let mut config = ExchangeConfig::default();
//! config.api_key = Some("your_api_key".to_string());
//! config.secret = Some("your_secret".to_string());
//! let binance = Binance::new_swap(config)?;
//!
//! // Fetch DAPI position mode
//! let is_hedge_mode = binance.fetch_position_mode_dapi(None).await?;
//! println!("DAPI Hedge mode: {}", is_hedge_mode);
//!
//! // Set DAPI position mode to hedge mode
//! let result = binance.set_position_mode_dapi(true, None).await?;
//! # Ok(())
//! # }
//! ```
//!
//! # Error Handling
//!
//! All methods return `Result<T>` and can fail with the following error types:
//!
//! - **AuthenticationError**: Missing or invalid API credentials
//! - **ExchangeError**: API returned an error (e.g., invalid symbol, insufficient balance)
//! - **NetworkError**: Connection or timeout issues
//! - **ParseError**: Failed to parse API response
//!
//! ```no_run
//! # use ccxt_exchanges::binance::Binance;
//! # use ccxt_core::{ExchangeConfig, Error};
//! # async fn example() -> ccxt_core::Result<()> {
//! let config = ExchangeConfig::default(); // No credentials
//! let binance = Binance::new_swap(config)?;
//!
//! // This will fail with AuthenticationError
//! match binance.fetch_dapi_account(None).await {
//!     Ok(account) => println!("Account: {:?}", account),
//!     Err(e) => {
//!         eprintln!("Error fetching account: {}", e);
//!         // Handle specific error types
//!     }
//! }
//! # Ok(())
//! # }
//! ```

use super::super::Binance;
use ccxt_core::{Error, ParseError, Result};
use reqwest::header::HeaderMap;
use serde_json::Value;
use std::collections::BTreeMap;

impl Binance {
    // ==================== DAPI Position Mode Methods ====================

    /// Set position mode for coin-margined futures (DAPI).
    ///
    /// This method sets the position mode (hedge mode or one-way mode) specifically
    /// for coin-margined (inverse) futures contracts using the DAPI endpoint.
    ///
    /// # Arguments
    ///
    /// * `dual_side` - `true` for hedge mode (dual-side position), `false` for one-way mode.
    /// * `params` - Optional additional parameters to include in the request.
    ///
    /// # Returns
    ///
    /// Returns the API response as a JSON `Value`.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Authentication credentials are missing
    /// - The API request fails
    /// - The response cannot be parsed
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
    ///
    /// // Enable hedge mode for DAPI
    /// let result = binance.set_position_mode_dapi(true, None).await?;
    /// println!("Result: {:?}", result);
    ///
    /// // Switch back to one-way mode for DAPI
    /// let result = binance.set_position_mode_dapi(false, None).await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Error Handling Example
    ///
    /// ```no_run
    /// # use ccxt_exchanges::binance::Binance;
    /// # use ccxt_core::ExchangeConfig;
    /// # async fn example() -> ccxt_core::Result<()> {
    /// let mut config = ExchangeConfig::default();
    /// config.api_key = Some("your_api_key".to_string());
    /// config.secret = Some("your_secret".to_string());
    /// let binance = Binance::new_swap(config)?;
    ///
    /// match binance.set_position_mode_dapi(true, None).await {
    ///     Ok(result) => println!("Position mode set successfully: {:?}", result),
    ///     Err(e) => {
    ///         eprintln!("Failed to set position mode: {}", e);
    ///         // Handle error - could be auth error, network error, or exchange error
    ///     }
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn set_position_mode_dapi(
        &self,
        dual_side: bool,
        params: Option<Value>,
    ) -> Result<Value> {
        self.check_required_credentials()?;

        let mut request_params = BTreeMap::new();
        request_params.insert("dualSidePosition".to_string(), dual_side.to_string());

        // Merge additional parameters if provided
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

        let timestamp = self.get_signing_timestamp().await?;
        let auth = self.get_auth()?;
        let signed_params =
            auth.sign_with_timestamp(&request_params, timestamp, Some(self.options().recv_window))?;

        // Use DAPI endpoint for coin-margined futures
        let url = format!("{}/positionSide/dual", self.urls().dapi_private);

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

    /// Fetch current position mode for coin-margined futures (DAPI).
    ///
    /// This method retrieves the current position mode (hedge mode or one-way mode)
    /// specifically for coin-margined (inverse) futures contracts using the DAPI endpoint.
    ///
    /// # Arguments
    ///
    /// * `params` - Optional additional parameters to include in the request.
    ///
    /// # Returns
    ///
    /// Returns the current position mode:
    /// - `true`: Hedge mode (dual-side position) is enabled.
    /// - `false`: One-way mode is enabled.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Authentication credentials are missing
    /// - The API request fails
    /// - The response cannot be parsed
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
    ///
    /// let is_hedge_mode = binance.fetch_position_mode_dapi(None).await?;
    /// if is_hedge_mode {
    ///     println!("DAPI is in hedge mode (dual-side positions)");
    /// } else {
    ///     println!("DAPI is in one-way mode");
    /// }
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Error Handling Example
    ///
    /// ```no_run
    /// # use ccxt_exchanges::binance::Binance;
    /// # use ccxt_core::ExchangeConfig;
    /// # async fn example() -> ccxt_core::Result<()> {
    /// let mut config = ExchangeConfig::default();
    /// config.api_key = Some("your_api_key".to_string());
    /// config.secret = Some("your_secret".to_string());
    /// let binance = Binance::new_swap(config)?;
    ///
    /// match binance.fetch_position_mode_dapi(None).await {
    ///     Ok(is_hedge) => {
    ///         let mode = if is_hedge { "hedge" } else { "one-way" };
    ///         println!("Current DAPI position mode: {}", mode);
    ///     }
    ///     Err(e) => eprintln!("Failed to fetch position mode: {}", e),
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn fetch_position_mode_dapi(&self, params: Option<Value>) -> Result<bool> {
        self.check_required_credentials()?;

        let mut request_params = BTreeMap::new();

        // Merge additional parameters if provided
        if let Some(params) = params {
            if let Some(obj) = params.as_object() {
                for (key, value) in obj {
                    if let Some(v) = value.as_str() {
                        request_params.insert(key.clone(), v.to_string());
                    }
                }
            }
        }

        let timestamp = self.get_signing_timestamp().await?;
        let auth = self.get_auth()?;
        let signed_params =
            auth.sign_with_timestamp(&request_params, timestamp, Some(self.options().recv_window))?;

        let query_string = signed_params
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<_>>()
            .join("&");

        // Use DAPI endpoint for coin-margined futures
        let url = format!(
            "{}/positionSide/dual?{}",
            self.urls().dapi_private,
            query_string
        );

        let mut headers = HeaderMap::new();
        auth.add_auth_headers_reqwest(&mut headers);

        let data = self.base().http_client.get(&url, Some(headers)).await?;

        // Parse the response to extract the dualSidePosition value
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
            "Failed to parse DAPI position mode response: missing or invalid dualSidePosition field",
        )))
    }

    // ==================== DAPI Account Methods ====================

    /// Fetch coin-margined futures account information.
    ///
    /// This method retrieves account information specifically for coin-margined (inverse)
    /// futures contracts using the DAPI endpoint. The response includes wallet balance,
    /// unrealized profit, margin balance, available balance, and position information.
    ///
    /// # Arguments
    ///
    /// * `params` - Optional additional parameters to include in the request.
    ///
    /// # Returns
    ///
    /// Returns the account information as a raw JSON `Value`. The response includes:
    /// - `totalWalletBalance`: Total wallet balance
    /// - `totalUnrealizedProfit`: Total unrealized profit
    /// - `totalMarginBalance`: Total margin balance
    /// - `availableBalance`: Available balance for new positions
    /// - `maxWithdrawAmount`: Maximum withdraw amount
    /// - `assets`: List of asset balances
    /// - `positions`: List of positions
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Authentication credentials are missing
    /// - The API request fails
    /// - The response cannot be parsed
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
    ///
    /// // Fetch DAPI account information
    /// let account = binance.fetch_dapi_account(None).await?;
    /// println!("Account: {:?}", account);
    ///
    /// // Access specific fields
    /// if let Some(balance) = account.get("totalWalletBalance") {
    ///     println!("Total wallet balance: {}", balance);
    /// }
    /// if let Some(positions) = account.get("positions") {
    ///     println!("Positions: {:?}", positions);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Error Handling Example
    ///
    /// ```no_run
    /// # use ccxt_exchanges::binance::Binance;
    /// # use ccxt_core::ExchangeConfig;
    /// # async fn example() -> ccxt_core::Result<()> {
    /// let mut config = ExchangeConfig::default();
    /// config.api_key = Some("your_api_key".to_string());
    /// config.secret = Some("your_secret".to_string());
    /// let binance = Binance::new_swap(config)?;
    ///
    /// match binance.fetch_dapi_account(None).await {
    ///     Ok(account) => {
    ///         // Process account data
    ///         if let Some(balance) = account.get("totalWalletBalance") {
    ///             println!("Wallet balance: {}", balance);
    ///         }
    ///     }
    ///     Err(e) => {
    ///         eprintln!("Failed to fetch DAPI account: {}", e);
    ///         // Could be authentication error, network error, or API error
    ///     }
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn fetch_dapi_account(&self, params: Option<Value>) -> Result<Value> {
        self.check_required_credentials()?;

        let mut request_params = BTreeMap::new();

        // Merge additional parameters if provided
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

        let timestamp = self.get_signing_timestamp().await?;
        let auth = self.get_auth()?;
        let signed_params =
            auth.sign_with_timestamp(&request_params, timestamp, Some(self.options().recv_window))?;

        let query_string = signed_params
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<_>>()
            .join("&");

        // Use DAPI endpoint for coin-margined futures account
        let url = format!("{}/account?{}", self.urls().dapi_private, query_string);

        let mut headers = HeaderMap::new();
        auth.add_auth_headers_reqwest(&mut headers);

        let data = self.base().http_client.get(&url, Some(headers)).await?;

        // Check for API errors in response
        if let Some(code) = data.get("code") {
            if let Some(msg) = data.get("msg") {
                return Err(Error::exchange(
                    code.to_string(),
                    msg.as_str().unwrap_or("Unknown error").to_string(),
                ));
            }
        }

        Ok(data)
    }

    // ==================== DAPI Income History Methods ====================

    /// Fetch income history for coin-margined futures.
    ///
    /// This method retrieves income history specifically for coin-margined (inverse)
    /// futures contracts using the DAPI endpoint. Income types include realized PnL,
    /// funding fees, commissions, and other transaction types.
    ///
    /// # Arguments
    ///
    /// * `symbol` - Optional symbol to filter by (e.g., "BTC/USD:BTC").
    /// * `income_type` - Optional income type to filter by. Valid values:
    ///   - `"TRANSFER"`: Transfer in/out
    ///   - `"WELCOME_BONUS"`: Welcome bonus
    ///   - `"REALIZED_PNL"`: Realized profit and loss
    ///   - `"FUNDING_FEE"`: Funding fee
    ///   - `"COMMISSION"`: Trading commission
    ///   - `"INSURANCE_CLEAR"`: Insurance fund clear
    ///   - `"REFERRAL_KICKBACK"`: Referral kickback
    ///   - `"COMMISSION_REBATE"`: Commission rebate
    ///   - `"DELIVERED_SETTELMENT"`: Delivered settlement
    /// * `since` - Optional start timestamp in milliseconds.
    /// * `limit` - Optional record limit (default 100, max 1000).
    /// * `params` - Optional additional parameters. Supports:
    ///   - `endTime`: End timestamp in milliseconds.
    ///
    /// # Returns
    ///
    /// Returns income history records as a raw JSON `Value` array. Each record includes:
    /// - `symbol`: Trading pair symbol
    /// - `incomeType`: Type of income
    /// - `income`: Income amount
    /// - `asset`: Asset currency
    /// - `info`: Additional information
    /// - `time`: Timestamp
    /// - `tranId`: Transaction ID
    /// - `tradeId`: Trade ID (if applicable)
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Authentication credentials are missing
    /// - The API request fails
    /// - The response cannot be parsed
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use ccxt_exchanges::binance::Binance;
    /// # use ccxt_core::ExchangeConfig;
    /// # use serde_json::json;
    /// # async fn example() -> ccxt_core::Result<()> {
    /// let mut config = ExchangeConfig::default();
    /// config.api_key = Some("your_api_key".to_string());
    /// config.secret = Some("your_secret".to_string());
    /// let binance = Binance::new_swap(config)?;
    ///
    /// // Fetch all income history
    /// let income = binance.fetch_dapi_income(None, None, None, None, None).await?;
    /// println!("Income history: {:?}", income);
    ///
    /// // Fetch funding fees for a specific symbol
    /// let funding_fees = binance.fetch_dapi_income(
    ///     Some("BTC/USD:BTC"),
    ///     Some("FUNDING_FEE"),
    ///     None,
    ///     Some(100),
    ///     None,
    /// ).await?;
    ///
    /// // Fetch income with time range
    /// let params = json!({
    ///     "endTime": 1704067200000_i64
    /// });
    /// let income = binance.fetch_dapi_income(
    ///     None,
    ///     None,
    ///     Some(1703980800000),  // since
    ///     Some(50),             // limit
    ///     Some(params),
    /// ).await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Error Handling Example
    ///
    /// ```no_run
    /// # use ccxt_exchanges::binance::Binance;
    /// # use ccxt_core::ExchangeConfig;
    /// # async fn example() -> ccxt_core::Result<()> {
    /// let mut config = ExchangeConfig::default();
    /// config.api_key = Some("your_api_key".to_string());
    /// config.secret = Some("your_secret".to_string());
    /// let binance = Binance::new_swap(config)?;
    ///
    /// match binance.fetch_dapi_income(None, Some("FUNDING_FEE"), None, Some(100), None).await {
    ///     Ok(income) => {
    ///         if let Some(records) = income.as_array() {
    ///             println!("Found {} income records", records.len());
    ///             for record in records {
    ///                 println!("Income: {:?}", record.get("income"));
    ///             }
    ///         }
    ///     }
    ///     Err(e) => eprintln!("Failed to fetch income history: {}", e),
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn fetch_dapi_income(
        &self,
        symbol: Option<&str>,
        income_type: Option<&str>,
        since: Option<i64>,
        limit: Option<u32>,
        params: Option<Value>,
    ) -> Result<Value> {
        self.check_required_credentials()?;

        let mut request_params = BTreeMap::new();

        // Add symbol parameter if provided
        if let Some(sym) = symbol {
            // Load markets to convert unified symbol to exchange symbol
            self.load_markets(false).await?;
            let market = self.base().market(sym).await?;
            request_params.insert("symbol".to_string(), market.id.clone());
        }

        // Add income type filter if provided
        if let Some(income_t) = income_type {
            request_params.insert("incomeType".to_string(), income_t.to_string());
        }

        // Add start time (since) parameter if provided
        if let Some(s) = since {
            request_params.insert("startTime".to_string(), s.to_string());
        }

        // Add limit parameter if provided
        if let Some(l) = limit {
            request_params.insert("limit".to_string(), l.to_string());
        }

        // Merge additional parameters if provided (including endTime)
        if let Some(params) = params {
            if let Some(obj) = params.as_object() {
                for (key, value) in obj {
                    if let Some(v) = value.as_str() {
                        request_params.insert(key.clone(), v.to_string());
                    } else if let Some(v) = value.as_bool() {
                        request_params.insert(key.clone(), v.to_string());
                    } else if let Some(v) = value.as_i64() {
                        request_params.insert(key.clone(), v.to_string());
                    } else if let Some(v) = value.as_u64() {
                        request_params.insert(key.clone(), v.to_string());
                    }
                }
            }
        }

        let timestamp = self.get_signing_timestamp().await?;
        let auth = self.get_auth()?;
        let signed_params =
            auth.sign_with_timestamp(&request_params, timestamp, Some(self.options().recv_window))?;

        let query_string = signed_params
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<_>>()
            .join("&");

        // Use DAPI endpoint for coin-margined futures income
        let url = format!("{}/income?{}", self.urls().dapi_private, query_string);

        let mut headers = HeaderMap::new();
        auth.add_auth_headers_reqwest(&mut headers);

        let data = self.base().http_client.get(&url, Some(headers)).await?;

        // Check for API errors in response
        if let Some(code) = data.get("code") {
            if let Some(msg) = data.get("msg") {
                return Err(Error::exchange(
                    code.to_string(),
                    msg.as_str().unwrap_or("Unknown error").to_string(),
                ));
            }
        }

        Ok(data)
    }

    // ==================== DAPI Commission Rate Methods ====================

    /// Fetch commission rate for a coin-margined futures symbol.
    ///
    /// This method retrieves the maker and taker commission rates for a specific
    /// coin-margined (inverse) futures symbol using the DAPI endpoint.
    ///
    /// # Arguments
    ///
    /// * `symbol` - Trading pair symbol (e.g., "BTC/USD:BTC"). This parameter is required.
    /// * `params` - Optional additional parameters to include in the request.
    ///
    /// # Returns
    ///
    /// Returns the commission rate information as a raw JSON `Value`. The response includes:
    /// - `symbol`: Trading pair symbol
    /// - `makerCommissionRate`: Maker commission rate (e.g., "0.0002")
    /// - `takerCommissionRate`: Taker commission rate (e.g., "0.0004")
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Authentication credentials are missing
    /// - The symbol parameter is invalid or not found
    /// - The API request fails
    /// - The response cannot be parsed
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
    ///
    /// // Fetch commission rate for BTC/USD:BTC
    /// let commission = binance.fetch_dapi_commission_rate("BTC/USD:BTC", None).await?;
    /// println!("Commission rate: {:?}", commission);
    ///
    /// // Access specific fields
    /// if let Some(maker_rate) = commission.get("makerCommissionRate") {
    ///     println!("Maker rate: {}", maker_rate);
    /// }
    /// if let Some(taker_rate) = commission.get("takerCommissionRate") {
    ///     println!("Taker rate: {}", taker_rate);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Error Handling Example
    ///
    /// ```no_run
    /// # use ccxt_exchanges::binance::Binance;
    /// # use ccxt_core::ExchangeConfig;
    /// # async fn example() -> ccxt_core::Result<()> {
    /// let mut config = ExchangeConfig::default();
    /// config.api_key = Some("your_api_key".to_string());
    /// config.secret = Some("your_secret".to_string());
    /// let binance = Binance::new_swap(config)?;
    ///
    /// match binance.fetch_dapi_commission_rate("BTC/USD:BTC", None).await {
    ///     Ok(commission) => {
    ///         let maker = commission.get("makerCommissionRate")
    ///             .and_then(|v| v.as_str())
    ///             .unwrap_or("N/A");
    ///         let taker = commission.get("takerCommissionRate")
    ///             .and_then(|v| v.as_str())
    ///             .unwrap_or("N/A");
    ///         println!("Maker: {}, Taker: {}", maker, taker);
    ///     }
    ///     Err(e) => eprintln!("Failed to fetch commission rate: {}", e),
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn fetch_dapi_commission_rate(
        &self,
        symbol: &str,
        params: Option<Value>,
    ) -> Result<Value> {
        self.check_required_credentials()?;

        // Load markets to convert unified symbol to exchange symbol
        self.load_markets(false).await?;
        let market = self.base().market(symbol).await?;

        let mut request_params = BTreeMap::new();
        request_params.insert("symbol".to_string(), market.id.clone());

        // Merge additional parameters if provided
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

        let timestamp = self.get_signing_timestamp().await?;
        let auth = self.get_auth()?;
        let signed_params =
            auth.sign_with_timestamp(&request_params, timestamp, Some(self.options().recv_window))?;

        let query_string = signed_params
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<_>>()
            .join("&");

        // Use DAPI endpoint for coin-margined futures commission rate
        let url = format!(
            "{}/commissionRate?{}",
            self.urls().dapi_private,
            query_string
        );

        let mut headers = HeaderMap::new();
        auth.add_auth_headers_reqwest(&mut headers);

        let data = self.base().http_client.get(&url, Some(headers)).await?;

        // Check for API errors in response
        if let Some(code) = data.get("code") {
            if let Some(msg) = data.get("msg") {
                return Err(Error::exchange(
                    code.to_string(),
                    msg.as_str().unwrap_or("Unknown error").to_string(),
                ));
            }
        }

        Ok(data)
    }

    // ==================== DAPI ADL Quantile Methods ====================

    /// Fetch ADL (Auto-Deleveraging) quantile for coin-margined futures.
    ///
    /// This method retrieves the ADL quantile information for coin-margined (inverse)
    /// futures positions using the DAPI endpoint. The ADL quantile indicates the
    /// priority of a position for auto-deleveraging during extreme market conditions.
    ///
    /// A higher quantile means a higher priority for auto-deleveraging. Traders with
    /// profitable positions and high leverage are more likely to be auto-deleveraged
    /// when the insurance fund is insufficient to cover liquidation losses.
    ///
    /// # Arguments
    ///
    /// * `symbol` - Optional symbol to filter by (e.g., "BTC/USD:BTC"). If not provided,
    ///   returns ADL quantile for all positions.
    /// * `params` - Optional additional parameters to include in the request.
    ///
    /// # Returns
    ///
    /// Returns the ADL quantile information as a raw JSON `Value`. The response includes:
    /// - `symbol`: Trading pair symbol
    /// - `adlQuantile`: ADL quantile object containing:
    ///   - `LONG`: Quantile for long positions (1-5, where 5 is highest priority)
    ///   - `SHORT`: Quantile for short positions (1-5, where 5 is highest priority)
    ///   - `HEDGE`: Quantile for hedge mode positions (if applicable)
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Authentication credentials are missing
    /// - The symbol parameter is invalid or not found (if provided)
    /// - The API request fails
    /// - The response cannot be parsed
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
    ///
    /// // Fetch ADL quantile for all positions
    /// let adl = binance.fetch_dapi_adl_quantile(None, None).await?;
    /// println!("ADL quantile: {:?}", adl);
    ///
    /// // Fetch ADL quantile for a specific symbol
    /// let adl = binance.fetch_dapi_adl_quantile(Some("BTC/USD:BTC"), None).await?;
    /// println!("BTC ADL quantile: {:?}", adl);
    ///
    /// // Access specific fields from the response
    /// if let Some(arr) = adl.as_array() {
    ///     for item in arr {
    ///         if let Some(symbol) = item.get("symbol") {
    ///             println!("Symbol: {}", symbol);
    ///         }
    ///         if let Some(quantile) = item.get("adlQuantile") {
    ///             if let Some(long_q) = quantile.get("LONG") {
    ///                 println!("Long ADL quantile: {}", long_q);
    ///             }
    ///             if let Some(short_q) = quantile.get("SHORT") {
    ///                 println!("Short ADL quantile: {}", short_q);
    ///             }
    ///         }
    ///     }
    /// }
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Error Handling Example
    ///
    /// ```no_run
    /// # use ccxt_exchanges::binance::Binance;
    /// # use ccxt_core::ExchangeConfig;
    /// # async fn example() -> ccxt_core::Result<()> {
    /// let mut config = ExchangeConfig::default();
    /// config.api_key = Some("your_api_key".to_string());
    /// config.secret = Some("your_secret".to_string());
    /// let binance = Binance::new_swap(config)?;
    ///
    /// match binance.fetch_dapi_adl_quantile(Some("BTC/USD:BTC"), None).await {
    ///     Ok(adl) => {
    ///         // Check ADL risk level
    ///         if let Some(arr) = adl.as_array() {
    ///             for item in arr {
    ///                 if let Some(quantile) = item.get("adlQuantile") {
    ///                     if let Some(long_q) = quantile.get("LONG").and_then(|v| v.as_i64()) {
    ///                         if long_q >= 4 {
    ///                             println!("Warning: High ADL risk for long position!");
    ///                         }
    ///                     }
    ///                 }
    ///             }
    ///         }
    ///     }
    ///     Err(e) => eprintln!("Failed to fetch ADL quantile: {}", e),
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn fetch_dapi_adl_quantile(
        &self,
        symbol: Option<&str>,
        params: Option<Value>,
    ) -> Result<Value> {
        self.check_required_credentials()?;

        let mut request_params = BTreeMap::new();

        // Add symbol parameter if provided
        if let Some(sym) = symbol {
            // Load markets to convert unified symbol to exchange symbol
            self.load_markets(false).await?;
            let market = self.base().market(sym).await?;
            request_params.insert("symbol".to_string(), market.id.clone());
        }

        // Merge additional parameters if provided
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

        let timestamp = self.get_signing_timestamp().await?;
        let auth = self.get_auth()?;
        let signed_params =
            auth.sign_with_timestamp(&request_params, timestamp, Some(self.options().recv_window))?;

        let query_string = signed_params
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<_>>()
            .join("&");

        // Use DAPI endpoint for coin-margined futures ADL quantile
        let url = format!("{}/adlQuantile?{}", self.urls().dapi_private, query_string);

        let mut headers = HeaderMap::new();
        auth.add_auth_headers_reqwest(&mut headers);

        let data = self.base().http_client.get(&url, Some(headers)).await?;

        // Check for API errors in response
        if let Some(code) = data.get("code") {
            if let Some(msg) = data.get("msg") {
                return Err(Error::exchange(
                    code.to_string(),
                    msg.as_str().unwrap_or("Unknown error").to_string(),
                ));
            }
        }

        Ok(data)
    }

    // ==================== DAPI Force Orders Methods ====================

    /// Fetch force liquidation orders for coin-margined futures.
    ///
    /// This method retrieves force liquidation orders (liquidations and auto-deleveraging)
    /// for coin-margined (inverse) futures contracts using the DAPI endpoint.
    ///
    /// Force orders occur when:
    /// - **Liquidation**: A position is forcibly closed due to insufficient margin
    /// - **ADL (Auto-Deleveraging)**: A profitable position is reduced to cover losses
    ///   from liquidated positions when the insurance fund is insufficient
    ///
    /// # Arguments
    ///
    /// * `symbol` - Optional symbol to filter by (e.g., "BTC/USD:BTC").
    /// * `auto_close_type` - Optional auto-close type to filter by. Valid values:
    ///   - `"LIQUIDATION"`: Liquidation orders
    ///   - `"ADL"`: Auto-deleveraging orders
    /// * `since` - Optional start timestamp in milliseconds.
    /// * `limit` - Optional record limit (default 50, max 100).
    /// * `params` - Optional additional parameters. Supports:
    ///   - `endTime`: End timestamp in milliseconds.
    ///
    /// # Returns
    ///
    /// Returns force liquidation order records as a raw JSON `Value` array. Each record includes:
    /// - `orderId`: Order ID
    /// - `symbol`: Trading pair symbol
    /// - `status`: Order status
    /// - `clientOrderId`: Client order ID
    /// - `price`: Order price
    /// - `avgPrice`: Average fill price
    /// - `origQty`: Original quantity
    /// - `executedQty`: Executed quantity
    /// - `cumBase`: Cumulative base asset
    /// - `timeInForce`: Time in force
    /// - `type`: Order type
    /// - `reduceOnly`: Whether reduce-only
    /// - `side`: Order side (BUY/SELL)
    /// - `positionSide`: Position side (LONG/SHORT/BOTH)
    /// - `origType`: Original order type
    /// - `time`: Order time
    /// - `updateTime`: Last update time
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Authentication credentials are missing
    /// - The symbol parameter is invalid or not found (if provided)
    /// - The API request fails
    /// - The response cannot be parsed
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use ccxt_exchanges::binance::Binance;
    /// # use ccxt_core::ExchangeConfig;
    /// # use serde_json::json;
    /// # async fn example() -> ccxt_core::Result<()> {
    /// let mut config = ExchangeConfig::default();
    /// config.api_key = Some("your_api_key".to_string());
    /// config.secret = Some("your_secret".to_string());
    /// let binance = Binance::new_swap(config)?;
    ///
    /// // Fetch all force orders
    /// let force_orders = binance.fetch_dapi_force_orders(None, None, None, None, None).await?;
    /// println!("Force orders: {:?}", force_orders);
    ///
    /// // Fetch liquidation orders for a specific symbol
    /// let liquidations = binance.fetch_dapi_force_orders(
    ///     Some("BTC/USD:BTC"),
    ///     Some("LIQUIDATION"),
    ///     None,
    ///     Some(50),
    ///     None,
    /// ).await?;
    ///
    /// // Fetch ADL orders with time range
    /// let params = json!({
    ///     "endTime": 1704067200000_i64
    /// });
    /// let adl_orders = binance.fetch_dapi_force_orders(
    ///     None,
    ///     Some("ADL"),
    ///     Some(1703980800000),  // since
    ///     Some(100),            // limit
    ///     Some(params),
    /// ).await?;
    ///
    /// // Process the results
    /// if let Some(orders) = force_orders.as_array() {
    ///     for order in orders {
    ///         if let Some(order_id) = order.get("orderId") {
    ///             println!("Order ID: {}", order_id);
    ///         }
    ///         if let Some(symbol) = order.get("symbol") {
    ///             println!("Symbol: {}", symbol);
    ///         }
    ///         if let Some(side) = order.get("side") {
    ///             println!("Side: {}", side);
    ///         }
    ///     }
    /// }
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Error Handling Example
    ///
    /// ```no_run
    /// # use ccxt_exchanges::binance::Binance;
    /// # use ccxt_core::ExchangeConfig;
    /// # async fn example() -> ccxt_core::Result<()> {
    /// let mut config = ExchangeConfig::default();
    /// config.api_key = Some("your_api_key".to_string());
    /// config.secret = Some("your_secret".to_string());
    /// let binance = Binance::new_swap(config)?;
    ///
    /// match binance.fetch_dapi_force_orders(None, Some("LIQUIDATION"), None, Some(50), None).await {
    ///     Ok(orders) => {
    ///         if let Some(arr) = orders.as_array() {
    ///             if arr.is_empty() {
    ///                 println!("No liquidation orders found");
    ///             } else {
    ///                 println!("Found {} liquidation orders", arr.len());
    ///                 for order in arr {
    ///                     println!("Order: {:?}", order.get("orderId"));
    ///                 }
    ///             }
    ///         }
    ///     }
    ///     Err(e) => eprintln!("Failed to fetch force orders: {}", e),
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn fetch_dapi_force_orders(
        &self,
        symbol: Option<&str>,
        auto_close_type: Option<&str>,
        since: Option<i64>,
        limit: Option<u32>,
        params: Option<Value>,
    ) -> Result<Value> {
        self.check_required_credentials()?;

        let mut request_params = BTreeMap::new();

        // Add symbol parameter if provided
        if let Some(sym) = symbol {
            // Load markets to convert unified symbol to exchange symbol
            self.load_markets(false).await?;
            let market = self.base().market(sym).await?;
            request_params.insert("symbol".to_string(), market.id.clone());
        }

        // Add auto close type filter if provided (LIQUIDATION or ADL)
        if let Some(close_type) = auto_close_type {
            request_params.insert("autoCloseType".to_string(), close_type.to_string());
        }

        // Add start time (since) parameter if provided
        if let Some(s) = since {
            request_params.insert("startTime".to_string(), s.to_string());
        }

        // Add limit parameter if provided (default 50, max 100)
        if let Some(l) = limit {
            request_params.insert("limit".to_string(), l.to_string());
        }

        // Merge additional parameters if provided (including endTime)
        if let Some(params) = params {
            if let Some(obj) = params.as_object() {
                for (key, value) in obj {
                    if let Some(v) = value.as_str() {
                        request_params.insert(key.clone(), v.to_string());
                    } else if let Some(v) = value.as_bool() {
                        request_params.insert(key.clone(), v.to_string());
                    } else if let Some(v) = value.as_i64() {
                        request_params.insert(key.clone(), v.to_string());
                    } else if let Some(v) = value.as_u64() {
                        request_params.insert(key.clone(), v.to_string());
                    }
                }
            }
        }

        let timestamp = self.get_signing_timestamp().await?;
        let auth = self.get_auth()?;
        let signed_params =
            auth.sign_with_timestamp(&request_params, timestamp, Some(self.options().recv_window))?;

        let query_string = signed_params
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<_>>()
            .join("&");

        // Use DAPI endpoint for coin-margined futures force orders
        let url = format!("{}/forceOrders?{}", self.urls().dapi_private, query_string);

        let mut headers = HeaderMap::new();
        auth.add_auth_headers_reqwest(&mut headers);

        let data = self.base().http_client.get(&url, Some(headers)).await?;

        // Check for API errors in response
        if let Some(code) = data.get("code") {
            if let Some(msg) = data.get("msg") {
                return Err(Error::exchange(
                    code.to_string(),
                    msg.as_str().unwrap_or("Unknown error").to_string(),
                ));
            }
        }

        Ok(data)
    }
}
