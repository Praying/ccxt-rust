//! Binance margin trading operations.
//!
//! This module contains all margin trading methods including borrowing, repaying,
//! and margin-specific account operations.

use super::super::signed_request::HttpMethod;
use super::super::{Binance, parser};
use ccxt_core::{
    Error, ParseError, Result,
    types::{MarginAdjustment, MarginLoan, MarginRepay},
};
use rust_decimal::Decimal;

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
        let url = format!("{}/sapi/v1/margin/loan", self.urls().sapi);

        let data = self
            .signed_request(url)
            .method(HttpMethod::Post)
            .param("asset", currency)
            .param("amount", amount)
            .execute()
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
        self.load_markets(false).await?;
        let market = self.base().market(symbol).await?;

        let url = format!("{}/sapi/v1/margin/loan", self.urls().sapi);

        let data = self
            .signed_request(url)
            .method(HttpMethod::Post)
            .param("asset", currency)
            .param("amount", amount)
            .param("symbol", &market.id)
            .param("isIsolated", "TRUE")
            .execute()
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
        let url = format!("{}/sapi/v1/margin/repay", self.urls().sapi);

        let data = self
            .signed_request(url)
            .method(HttpMethod::Post)
            .param("asset", currency)
            .param("amount", amount)
            .execute()
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
        self.load_markets(false).await?;
        let market = self.base().market(symbol).await?;

        let url = format!("{}/sapi/v1/margin/repay", self.urls().sapi);

        let data = self
            .signed_request(url)
            .method(HttpMethod::Post)
            .param("asset", currency)
            .param("amount", amount)
            .param("symbol", &market.id)
            .param("isIsolated", "TRUE")
            .execute()
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
        let market_id = if let Some(sym) = symbol {
            self.load_markets(false).await?;
            let market = self.base().market(sym).await?;
            Some(market.id.clone())
        } else {
            None
        };

        let url = format!("{}/sapi/v1/margin/forceLiquidationRec", self.urls().sapi);

        let data = self
            .signed_request(url)
            .optional_param("symbol", market_id.as_ref())
            .optional_param("isolatedSymbol", market_id.as_ref())
            .optional_param("startTime", since)
            .optional_param("size", limit)
            .execute()
            .await?;

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
        let url = format!("{}/sapi/v1/margin/maxBorrowable", self.urls().sapi);

        let data = self
            .signed_request(url)
            .param("asset", currency)
            .execute()
            .await?;

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
        self.load_markets(false).await?;
        let market = self.base().market(symbol).await?;

        let url = format!("{}/sapi/v1/margin/maxBorrowable", self.urls().sapi);

        let data = self
            .signed_request(url)
            .param("asset", currency)
            .param("isolatedSymbol", &market.id)
            .execute()
            .await?;

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
        let url = format!("{}/sapi/v1/margin/maxTransferable", self.urls().sapi);

        let data = self
            .signed_request(url)
            .param("asset", currency)
            .execute()
            .await?;

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
