//! Binance margin trading operations.
//!
//! This module contains all margin trading methods including borrowing, repaying,
//! and margin-specific account operations.

use super::super::{Binance, parser};
use ccxt_core::{
    Error, ParseError, Result,
    types::{MarginAdjustment, MarginLoan, MarginRepay},
};
use reqwest::header::HeaderMap;
use rust_decimal::Decimal;
use std::collections::HashMap;

impl Binance {
    // ==================== Borrow Methods ====================

    /// Borrow funds in cross margin mode.
    ///
    /// # Arguments
    ///
    /// * `currency` - Currency code (e.g., "USDT", "BTC").
    /// * `amount` - Borrow amount.
    ///
    /// # Returns
    ///
    /// Returns a [`MarginLoan`] record with transaction details.
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
    /// let loan = binance.borrow_cross_margin("USDT", 100.0).await?;
    /// println!("Loan ID: {}", loan.id);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn borrow_cross_margin(&self, currency: &str, amount: f64) -> Result<MarginLoan> {
        self.check_required_credentials()?;

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
    /// * `symbol` - Trading pair symbol (e.g., "BTC/USDT").
    /// * `currency` - Currency code to borrow.
    /// * `amount` - Borrow amount.
    ///
    /// # Returns
    ///
    /// Returns a [`MarginLoan`] record with transaction details.
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
    /// let loan = binance.borrow_isolated_margin("BTC/USDT", "USDT", 100.0).await?;
    /// println!("Loan ID: {}", loan.id);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn borrow_isolated_margin(
        &self,
        symbol: &str,
        currency: &str,
        amount: f64,
    ) -> Result<MarginLoan> {
        self.check_required_credentials()?;
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

    // ==================== Repay Methods ====================

    /// Repay borrowed funds in cross margin mode.
    ///
    /// # Arguments
    ///
    /// * `currency` - Currency code (e.g., "USDT", "BTC").
    /// * `amount` - Repayment amount.
    ///
    /// # Returns
    ///
    /// Returns a [`MarginRepay`] record with transaction details.
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
    /// let repay = binance.repay_cross_margin("USDT", 100.0).await?;
    /// println!("Repay ID: {}", repay.id);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn repay_cross_margin(&self, currency: &str, amount: f64) -> Result<MarginRepay> {
        self.check_required_credentials()?;

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
    /// * `symbol` - Trading pair symbol (e.g., "BTC/USDT").
    /// * `currency` - Currency code to repay.
    /// * `amount` - Repayment amount.
    ///
    /// # Returns
    ///
    /// Returns a [`MarginRepay`] record with transaction details.
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
    /// # use rust_decimal_macros::dec;
    /// # async fn example() -> ccxt_core::Result<()> {
    /// let mut config = ExchangeConfig::default();
    /// config.api_key = Some("your_api_key".to_string());
    /// config.secret = Some("your_secret".to_string());
    /// let binance = Binance::new(config)?;
    /// let repay = binance.repay_isolated_margin("BTC/USDT", "USDT", dec!(100)).await?;
    /// println!("Repay ID: {}", repay.id);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn repay_isolated_margin(
        &self,
        symbol: &str,
        currency: &str,
        amount: Decimal,
    ) -> Result<MarginRepay> {
        self.check_required_credentials()?;
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

    // ==================== Margin Info Methods ====================

    /// Fetch margin adjustment history.
    ///
    /// Retrieves liquidation records and margin adjustment history.
    ///
    /// # Arguments
    ///
    /// * `symbol` - Optional trading pair symbol (required for isolated margin).
    /// * `since` - Optional start timestamp in milliseconds.
    /// * `limit` - Optional maximum number of records to return.
    ///
    /// # Returns
    ///
    /// Returns a vector of [`MarginAdjustment`] records.
    ///
    /// # Errors
    ///
    /// Returns an error if authentication fails or the API request fails.
    pub async fn fetch_margin_adjustment_history(
        &self,
        symbol: Option<&str>,
        since: Option<i64>,
        limit: Option<i64>,
    ) -> Result<Vec<MarginAdjustment>> {
        self.check_required_credentials()?;

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
        let signed_params =
            auth.sign_with_timestamp(&params, timestamp, Some(self.options().recv_window))?;

        let query_string: String = signed_params
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<_>>()
            .join("&");

        let url = format!(
            "{}/sapi/v1/margin/forceLiquidationRec?{}",
            self.urls().sapi,
            query_string
        );

        let mut headers = HeaderMap::new();
        auth.add_auth_headers_reqwest(&mut headers);

        let data = self.base().http_client.get(&url, Some(headers)).await?;

        let rows = data["rows"].as_array().ok_or_else(|| {
            Error::from(ParseError::invalid_format(
                "data",
                "Expected rows array in response",
            ))
        })?;

        let mut adjustments = Vec::new();
        for row in rows {
            if let Ok(adjustment) = parser::parse_margin_adjustment(row) {
                adjustments.push(adjustment);
            }
        }

        Ok(adjustments)
    }

    /// Fetch maximum borrowable amount for cross margin.
    ///
    /// # Arguments
    ///
    /// * `currency` - Currency code (e.g., "USDT", "BTC").
    ///
    /// # Returns
    ///
    /// Returns the maximum borrowable amount as a `Decimal`.
    ///
    /// # Errors
    ///
    /// Returns an error if authentication fails or the API request fails.
    pub async fn fetch_cross_margin_max_borrowable(&self, currency: &str) -> Result<Decimal> {
        self.check_required_credentials()?;

        let mut params = HashMap::new();
        params.insert("asset".to_string(), currency.to_string());

        let timestamp = self.fetch_time_raw().await?;
        let auth = self.get_auth()?;
        let signed_params =
            auth.sign_with_timestamp(&params, timestamp, Some(self.options().recv_window))?;

        let query_string: String = signed_params
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<_>>()
            .join("&");

        let url = format!(
            "{}/sapi/v1/margin/maxBorrowable?{}",
            self.urls().sapi,
            query_string
        );

        let mut headers = HeaderMap::new();
        auth.add_auth_headers_reqwest(&mut headers);

        let data = self.base().http_client.get(&url, Some(headers)).await?;

        let amount_str = data["amount"]
            .as_str()
            .ok_or_else(|| Error::from(ParseError::missing_field("amount")))?;

        amount_str.parse::<Decimal>().map_err(|e| {
            Error::from(ParseError::invalid_format(
                "amount",
                format!("Failed to parse amount: {}", e),
            ))
        })
    }

    /// Fetch maximum borrowable amount for isolated margin.
    ///
    /// # Arguments
    ///
    /// * `symbol` - Trading pair symbol (e.g., "BTC/USDT").
    /// * `currency` - Currency code to check.
    ///
    /// # Returns
    ///
    /// Returns the maximum borrowable amount as a `Decimal`.
    ///
    /// # Errors
    ///
    /// Returns an error if authentication fails or the API request fails.
    pub async fn fetch_isolated_margin_max_borrowable(
        &self,
        symbol: &str,
        currency: &str,
    ) -> Result<Decimal> {
        self.check_required_credentials()?;
        self.load_markets(false).await?;
        let market = self.base().market(symbol).await?;

        let mut params = HashMap::new();
        params.insert("asset".to_string(), currency.to_string());
        params.insert("isolatedSymbol".to_string(), market.id.clone());

        let timestamp = self.fetch_time_raw().await?;
        let auth = self.get_auth()?;
        let signed_params =
            auth.sign_with_timestamp(&params, timestamp, Some(self.options().recv_window))?;

        let query_string: String = signed_params
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<_>>()
            .join("&");

        let url = format!(
            "{}/sapi/v1/margin/maxBorrowable?{}",
            self.urls().sapi,
            query_string
        );

        let mut headers = HeaderMap::new();
        auth.add_auth_headers_reqwest(&mut headers);

        let data = self.base().http_client.get(&url, Some(headers)).await?;

        let amount_str = data["amount"]
            .as_str()
            .ok_or_else(|| Error::from(ParseError::missing_field("amount")))?;

        amount_str.parse::<Decimal>().map_err(|e| {
            Error::from(ParseError::invalid_format(
                "amount",
                format!("Failed to parse amount: {}", e),
            ))
        })
    }

    /// Fetch maximum transferable amount.
    ///
    /// # Arguments
    ///
    /// * `currency` - Currency code (e.g., "USDT", "BTC").
    ///
    /// # Returns
    ///
    /// Returns the maximum transferable amount as a `Decimal`.
    ///
    /// # Errors
    ///
    /// Returns an error if authentication fails or the API request fails.
    pub async fn fetch_max_transferable(&self, currency: &str) -> Result<Decimal> {
        self.check_required_credentials()?;

        let mut params = HashMap::new();
        params.insert("asset".to_string(), currency.to_string());

        let timestamp = self.fetch_time_raw().await?;
        let auth = self.get_auth()?;
        let signed_params =
            auth.sign_with_timestamp(&params, timestamp, Some(self.options().recv_window))?;

        let query_string: String = signed_params
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

        let amount_str = data["amount"]
            .as_str()
            .ok_or_else(|| Error::from(ParseError::missing_field("amount")))?;

        amount_str.parse::<Decimal>().map_err(|e| {
            Error::from(ParseError::invalid_format(
                "amount",
                format!("Failed to parse amount: {}", e),
            ))
        })
    }
}
