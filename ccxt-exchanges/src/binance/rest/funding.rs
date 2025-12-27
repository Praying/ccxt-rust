//! Binance funding operations.
//!
//! This module contains all deposit and withdrawal methods including
//! deposit addresses, withdrawals, transfers, and transaction history.

use super::super::{Binance, parser};
use ccxt_core::{
    Error, Result,
    types::{DepositAddress, DepositWithdrawFee, Transaction, TransactionType},
};
use std::collections::BTreeMap;

impl Binance {
    // ==================== Deposit Address Methods ====================

    /// Fetch deposit address for a currency.
    ///
    /// # Arguments
    ///
    /// * `code` - Currency code (e.g., "BTC", "USDT").
    /// * `params` - Optional parameters:
    ///   - `network`: Network type (e.g., "ETH", "BSC", "TRX").
    ///
    /// # Returns
    ///
    /// Returns a [`DepositAddress`] with the deposit address and optional tag.
    ///
    /// # Errors
    ///
    /// Returns an error if authentication fails or the API request fails.
    ///
    /// # Notes
    ///
    /// - Binance automatically generates a new address if none exists.
    /// - Some currencies require the network parameter (e.g., USDT can be on ETH/BSC/TRX).
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use ccxt_exchanges::binance::Binance;
    /// # use ccxt_core::ExchangeConfig;
    /// # use std::collections::BTreeMap;
    /// # async fn example() -> ccxt_core::Result<()> {
    /// let binance = Binance::new(ExchangeConfig::default())?;
    ///
    /// // Fetch BTC deposit address
    /// let addr = binance.fetch_deposit_address("BTC", None).await?;
    /// println!("BTC address: {}", addr.address);
    ///
    /// // Fetch USDT deposit address on TRX network
    /// let mut params = BTreeMap::new();
    /// params.insert("network".to_string(), "TRX".to_string());
    /// let addr = binance.fetch_deposit_address("USDT", Some(params)).await?;
    /// println!("USDT-TRX address: {}", addr.address);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn fetch_deposit_address(
        &self,
        code: &str,
        params: Option<BTreeMap<String, String>>,
    ) -> Result<DepositAddress> {
        let url = format!("{}/capital/deposit/address", self.urls().sapi);

        let data = self
            .signed_request(url)
            .param("coin", code.to_uppercase())
            .params(params.unwrap_or_default())
            .execute()
            .await?;

        parser::parse_deposit_address(&data)
    }

    // ==================== Withdrawal Methods ====================

    /// Withdraw funds to an external address.
    ///
    /// # Arguments
    ///
    /// * `code` - Currency code (e.g., "BTC", "USDT").
    /// * `amount` - Withdrawal amount.
    /// * `address` - Withdrawal address.
    /// * `params` - Optional parameters:
    ///   - `tag`: Address tag (e.g., XRP tag).
    ///   - `network`: Network type (e.g., "ETH", "BSC", "TRX").
    ///   - `addressTag`: Address tag (alias for `tag`).
    ///   - `name`: Address memo name.
    ///   - `walletType`: Wallet type (0=spot, 1=funding).
    ///
    /// # Returns
    ///
    /// Returns a [`Transaction`] with withdrawal details.
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
    /// # use std::collections::BTreeMap;
    /// # async fn example() -> ccxt_core::Result<()> {
    /// let binance = Binance::new(ExchangeConfig::default())?;
    ///
    /// // Basic withdrawal
    /// let tx = binance.withdraw("USDT", "100.0", "TXxxx...", None).await?;
    /// println!("Withdrawal ID: {}", tx.id);
    ///
    /// // Withdrawal with specific network
    /// let mut params = BTreeMap::new();
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
        params: Option<BTreeMap<String, String>>,
    ) -> Result<Transaction> {
        let url = format!("{}/capital/withdraw/apply", self.urls().sapi);

        let mut request_params = params.unwrap_or_default();

        // Handle optional tag parameter (supports both 'tag' and 'addressTag' names)
        if let Some(tag) = request_params.get("tag").cloned() {
            request_params.insert("addressTag".to_string(), tag);
        }

        let data = self
            .signed_request(url)
            .method(super::super::signed_request::HttpMethod::Post)
            .param("coin", code.to_uppercase())
            .param("address", address)
            .param("amount", amount)
            .params(request_params)
            .execute()
            .await?;

        parser::parse_transaction(&data, TransactionType::Withdrawal)
    }

    // ==================== Transaction History Methods ====================

    /// Fetch deposit history.
    ///
    /// # Arguments
    ///
    /// * `code` - Optional currency code (e.g., "BTC", "USDT").
    /// * `since` - Optional start timestamp in milliseconds.
    /// * `limit` - Optional quantity limit.
    /// * `params` - Optional parameters:
    ///   - `coin`: Currency (overrides code parameter).
    ///   - `status`: Status filter (0=pending, 6=credited, 1=success).
    ///   - `startTime`: Start timestamp in milliseconds.
    ///   - `endTime`: End timestamp in milliseconds.
    ///   - `offset`: Offset for pagination.
    ///   - `txId`: Transaction ID filter.
    ///
    /// # Returns
    ///
    /// Returns a vector of [`Transaction`] records.
    ///
    /// # Notes
    ///
    /// - Maximum query range: 90 days.
    /// - Default returns last 90 days if no time range provided.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use ccxt_exchanges::binance::Binance;
    /// # use ccxt_core::ExchangeConfig;
    /// # async fn example() -> ccxt_core::Result<()> {
    /// let binance = Binance::new(ExchangeConfig::default())?;
    ///
    /// // Query all deposits
    /// let deposits = binance.fetch_deposits(None, None, Some(100), None).await?;
    /// println!("Total deposits: {}", deposits.len());
    ///
    /// // Query BTC deposits
    /// let btc_deposits = binance.fetch_deposits(Some("BTC"), None, None, None).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn fetch_deposits(
        &self,
        code: Option<&str>,
        since: Option<i64>,
        limit: Option<i64>,
        params: Option<BTreeMap<String, String>>,
    ) -> Result<Vec<Transaction>> {
        let url = format!("{}/capital/deposit/hisrec", self.urls().sapi);

        let data = self
            .signed_request(url)
            .optional_param("coin", code.map(|c| c.to_uppercase()))
            .optional_param("startTime", since)
            .optional_param("limit", limit)
            .params(params.unwrap_or_default())
            .execute()
            .await?;

        if let Some(arr) = data.as_array() {
            arr.iter()
                .map(|item| parser::parse_transaction(item, TransactionType::Deposit))
                .collect()
        } else {
            Err(Error::invalid_request("Expected array response"))
        }
    }

    /// Fetch withdrawal history.
    ///
    /// # Arguments
    ///
    /// * `code` - Optional currency code (e.g., "BTC", "USDT").
    /// * `since` - Optional start timestamp in milliseconds.
    /// * `limit` - Optional quantity limit.
    /// * `params` - Optional parameters:
    ///   - `coin`: Currency (overrides code parameter).
    ///   - `withdrawOrderId`: Withdrawal order ID.
    ///   - `status`: Status filter (0=email sent, 1=cancelled, 2=awaiting approval,
    ///     3=rejected, 4=processing, 5=failure, 6=completed).
    ///   - `startTime`: Start timestamp in milliseconds.
    ///   - `endTime`: End timestamp in milliseconds.
    ///   - `offset`: Offset for pagination.
    ///
    /// # Returns
    ///
    /// Returns a vector of [`Transaction`] records.
    ///
    /// # Notes
    ///
    /// - Maximum query range: 90 days.
    /// - Default returns last 90 days if no time range provided.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use ccxt_exchanges::binance::Binance;
    /// # use ccxt_core::ExchangeConfig;
    /// # async fn example() -> ccxt_core::Result<()> {
    /// let binance = Binance::new(ExchangeConfig::default())?;
    ///
    /// // Query all withdrawals
    /// let withdrawals = binance.fetch_withdrawals(None, None, Some(100), None).await?;
    /// println!("Total withdrawals: {}", withdrawals.len());
    ///
    /// // Query USDT withdrawals
    /// let usdt_withdrawals = binance.fetch_withdrawals(Some("USDT"), None, None, None).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn fetch_withdrawals(
        &self,
        code: Option<&str>,
        since: Option<i64>,
        limit: Option<i64>,
        params: Option<BTreeMap<String, String>>,
    ) -> Result<Vec<Transaction>> {
        let url = format!("{}/capital/withdraw/history", self.urls().sapi);

        let data = self
            .signed_request(url)
            .optional_param("coin", code.map(|c| c.to_uppercase()))
            .optional_param("startTime", since)
            .optional_param("limit", limit)
            .params(params.unwrap_or_default())
            .execute()
            .await?;

        if let Some(arr) = data.as_array() {
            arr.iter()
                .map(|item| parser::parse_transaction(item, TransactionType::Withdrawal))
                .collect()
        } else {
            Err(Error::invalid_request("Expected array response"))
        }
    }

    // ==================== Fee Methods ====================

    /// Fetch deposit and withdrawal fees for currencies.
    ///
    /// # Arguments
    ///
    /// * `currency` - Optional currency code to filter results.
    /// * `params` - Optional additional parameters.
    ///
    /// # Returns
    ///
    /// Returns a vector of [`DepositWithdrawFee`] structures.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use ccxt_exchanges::binance::Binance;
    /// # use ccxt_core::ExchangeConfig;
    /// # async fn example() -> ccxt_core::Result<()> {
    /// let binance = Binance::new(ExchangeConfig::default())?;
    ///
    /// // Fetch all fees
    /// let fees = binance.fetch_deposit_withdraw_fees(None, None).await?;
    ///
    /// // Fetch fees for specific currency
    /// let btc_fees = binance.fetch_deposit_withdraw_fees(Some("BTC"), None).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn fetch_deposit_withdraw_fees(
        &self,
        currency: Option<&str>,
        params: Option<BTreeMap<String, String>>,
    ) -> Result<Vec<DepositWithdrawFee>> {
        // Note: Binance /sapi/v1/capital/config/getall endpoint returns all currencies
        // If currency is specified, client-side filtering is required

        let url = format!("{}/capital/config/getall", self.urls().sapi);

        let data = self
            .signed_request(url)
            .params(params.unwrap_or_default())
            .execute()
            .await?;

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
}
