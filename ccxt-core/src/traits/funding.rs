//! Funding trait definition.
//!
//! The `Funding` trait provides methods for deposit and withdrawal operations
//! including fetching deposit addresses, withdrawing funds, and transferring
//! between accounts. These operations require authentication.
//!
//! # Object Safety
//!
//! This trait is designed to be object-safe, allowing for dynamic dispatch via
//! trait objects (`dyn Funding`).
//!
//! # Example
//!
//! ```rust,ignore
//! use ccxt_core::traits::Funding;
//! use ccxt_core::types::params::{WithdrawParams, TransferParams};
//!
//! async fn manage_funds(exchange: &dyn Funding) -> Result<(), ccxt_core::Error> {
//!     // Get deposit address
//!     let address = exchange.fetch_deposit_address("USDT").await?;
//!     
//!     // Transfer between accounts
//!     let transfer = exchange.transfer(
//!         TransferParams::spot_to_futures("USDT", rust_decimal_macros::dec!(1000))
//!     ).await?;
//!     
//!     Ok(())
//! }
//! ```

use async_trait::async_trait;

use crate::error::Result;
use crate::traits::PublicExchange;
use crate::types::{
    DepositAddress, Transaction, Transfer,
    params::{TransferParams, WithdrawParams},
};

/// Trait for deposit and withdrawal operations.
///
/// This trait provides methods for managing deposits, withdrawals, and
/// inter-account transfers. All methods require authentication and are async.
///
/// # Supertrait
///
/// Requires `PublicExchange` as a supertrait to access exchange metadata
/// and capabilities.
///
/// # Thread Safety
///
/// This trait requires `Send + Sync` bounds (inherited from `PublicExchange`)
/// to ensure safe usage across thread boundaries in async contexts.
#[async_trait]
pub trait Funding: PublicExchange {
    // ========================================================================
    // Deposit Address
    // ========================================================================

    /// Fetch deposit address for a currency.
    ///
    /// Returns the deposit address for the specified currency on the default network.
    ///
    /// # Arguments
    ///
    /// * `code` - Currency code (e.g., "BTC", "USDT")
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let address = exchange.fetch_deposit_address("USDT").await?;
    /// println!("Deposit to: {}", address.address);
    /// ```
    async fn fetch_deposit_address(&self, code: &str) -> Result<DepositAddress>;

    /// Fetch deposit address for a specific network.
    ///
    /// # Arguments
    ///
    /// * `code` - Currency code (e.g., "USDT")
    /// * `network` - Network name (e.g., "TRC20", "ERC20", "BEP20")
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let address = exchange.fetch_deposit_address_on_network("USDT", "TRC20").await?;
    /// println!("TRC20 address: {}", address.address);
    /// ```
    async fn fetch_deposit_address_on_network(
        &self,
        code: &str,
        network: &str,
    ) -> Result<DepositAddress>;

    // ========================================================================
    // Withdrawal
    // ========================================================================

    /// Withdraw funds to an external address.
    ///
    /// # Arguments
    ///
    /// * `params` - Withdrawal parameters including currency, amount, address, and optional network
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use ccxt_core::types::params::WithdrawParams;
    /// use rust_decimal_macros::dec;
    ///
    /// let tx = exchange.withdraw(
    ///     WithdrawParams::new("USDT", dec!(100), "TAddress...")
    ///         .network("TRC20")
    /// ).await?;
    /// println!("Withdrawal ID: {}", tx.id);
    /// ```
    async fn withdraw(&self, params: WithdrawParams) -> Result<Transaction>;

    // ========================================================================
    // Transfer
    // ========================================================================

    /// Transfer funds between accounts.
    ///
    /// # Arguments
    ///
    /// * `params` - Transfer parameters including currency, amount, source and destination accounts
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use ccxt_core::types::params::TransferParams;
    /// use rust_decimal_macros::dec;
    ///
    /// // Transfer USDT from spot to futures
    /// let transfer = exchange.transfer(
    ///     TransferParams::spot_to_futures("USDT", dec!(1000))
    /// ).await?;
    /// ```
    async fn transfer(&self, params: TransferParams) -> Result<Transfer>;

    // ========================================================================
    // Transaction History
    // ========================================================================

    /// Fetch deposit history.
    ///
    /// # Arguments
    ///
    /// * `code` - Optional currency code to filter by
    /// * `since` - Optional start timestamp in milliseconds
    /// * `limit` - Optional maximum number of records to return
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// // All deposits
    /// let deposits = exchange.fetch_deposits(None, None, None).await?;
    ///
    /// // USDT deposits only
    /// let deposits = exchange.fetch_deposits(Some("USDT"), None, Some(100)).await?;
    /// ```
    async fn fetch_deposits(
        &self,
        code: Option<&str>,
        since: Option<i64>,
        limit: Option<u32>,
    ) -> Result<Vec<Transaction>>;

    /// Fetch withdrawal history.
    ///
    /// # Arguments
    ///
    /// * `code` - Optional currency code to filter by
    /// * `since` - Optional start timestamp in milliseconds
    /// * `limit` - Optional maximum number of records to return
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// // All withdrawals
    /// let withdrawals = exchange.fetch_withdrawals(None, None, None).await?;
    ///
    /// // Recent BTC withdrawals
    /// let withdrawals = exchange.fetch_withdrawals(Some("BTC"), None, Some(50)).await?;
    /// ```
    async fn fetch_withdrawals(
        &self,
        code: Option<&str>,
        since: Option<i64>,
        limit: Option<u32>,
    ) -> Result<Vec<Transaction>>;
}

/// Type alias for boxed Funding trait object.
pub type BoxedFunding = Box<dyn Funding>;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capability::ExchangeCapabilities;
    use crate::types::Timeframe;
    use crate::types::transaction::{TransactionStatus, TransactionType};
    use rust_decimal_macros::dec;

    // Mock implementation for testing trait object safety
    struct MockExchange;

    impl PublicExchange for MockExchange {
        fn id(&self) -> &str {
            "mock"
        }
        fn name(&self) -> &str {
            "Mock Exchange"
        }
        fn capabilities(&self) -> ExchangeCapabilities {
            ExchangeCapabilities::all()
        }
        fn timeframes(&self) -> Vec<Timeframe> {
            vec![Timeframe::H1]
        }
    }

    #[async_trait]
    impl Funding for MockExchange {
        async fn fetch_deposit_address(&self, code: &str) -> Result<DepositAddress> {
            Ok(DepositAddress::new(
                code.to_string(),
                "0x1234567890abcdef".to_string(),
            ))
        }

        async fn fetch_deposit_address_on_network(
            &self,
            code: &str,
            network: &str,
        ) -> Result<DepositAddress> {
            let mut address =
                DepositAddress::new(code.to_string(), "0x1234567890abcdef".to_string());
            address.network = Some(network.to_string());
            Ok(address)
        }

        async fn withdraw(&self, params: WithdrawParams) -> Result<Transaction> {
            Ok(Transaction::new(
                "withdraw_123".to_string(),
                TransactionType::Withdrawal,
                params.amount,
                params.currency,
                TransactionStatus::Pending,
            ))
        }

        async fn transfer(&self, params: TransferParams) -> Result<Transfer> {
            Ok(Transfer {
                id: Some("transfer_123".to_string()),
                timestamp: 1609459200000,
                datetime: "2021-01-01T00:00:00Z".to_string(),
                currency: params.currency,
                amount: params.amount.to_string().parse().unwrap_or(0.0),
                from_account: Some(format!("{:?}", params.from_account)),
                to_account: Some(format!("{:?}", params.to_account)),
                status: "success".to_string(),
                info: None,
            })
        }

        async fn fetch_deposits(
            &self,
            code: Option<&str>,
            _since: Option<i64>,
            _limit: Option<u32>,
        ) -> Result<Vec<Transaction>> {
            let currency = code.unwrap_or("USDT").to_string();
            Ok(vec![Transaction::new(
                "deposit_123".to_string(),
                TransactionType::Deposit,
                dec!(100),
                currency,
                TransactionStatus::Ok,
            )])
        }

        async fn fetch_withdrawals(
            &self,
            code: Option<&str>,
            _since: Option<i64>,
            _limit: Option<u32>,
        ) -> Result<Vec<Transaction>> {
            let currency = code.unwrap_or("USDT").to_string();
            Ok(vec![Transaction::new(
                "withdraw_456".to_string(),
                TransactionType::Withdrawal,
                dec!(50),
                currency,
                TransactionStatus::Ok,
            )])
        }
    }

    #[test]
    fn test_trait_object_safety() {
        // Verify trait is object-safe by creating a trait object
        let _exchange: BoxedFunding = Box::new(MockExchange);
    }

    #[tokio::test]
    async fn test_fetch_deposit_address() {
        let exchange = MockExchange;

        let address = exchange.fetch_deposit_address("USDT").await.unwrap();
        assert_eq!(address.currency, "USDT");
        assert!(!address.address.is_empty());
    }

    #[tokio::test]
    async fn test_fetch_deposit_address_on_network() {
        let exchange = MockExchange;

        let address = exchange
            .fetch_deposit_address_on_network("USDT", "TRC20")
            .await
            .unwrap();
        assert_eq!(address.currency, "USDT");
        assert_eq!(address.network, Some("TRC20".to_string()));
    }

    #[tokio::test]
    async fn test_withdraw() {
        let exchange = MockExchange;

        let tx = exchange
            .withdraw(WithdrawParams::new("USDT", dec!(100), "TAddress123"))
            .await
            .unwrap();

        assert_eq!(tx.currency, "USDT");
        assert_eq!(tx.amount, dec!(100));
        assert!(tx.is_withdrawal());
        assert!(tx.is_pending());
    }

    #[tokio::test]
    async fn test_transfer() {
        let exchange = MockExchange;

        let transfer = exchange
            .transfer(TransferParams::spot_to_futures("USDT", dec!(1000)))
            .await
            .unwrap();

        assert_eq!(transfer.currency, "USDT");
        assert_eq!(transfer.status, "success");
    }

    #[tokio::test]
    async fn test_fetch_deposits() {
        let exchange = MockExchange;

        let deposits = exchange
            .fetch_deposits(Some("USDT"), None, None)
            .await
            .unwrap();
        assert_eq!(deposits.len(), 1);
        assert!(deposits[0].is_deposit());
        assert!(deposits[0].is_completed());
    }

    #[tokio::test]
    async fn test_fetch_withdrawals() {
        let exchange = MockExchange;

        let withdrawals = exchange
            .fetch_withdrawals(None, None, Some(50))
            .await
            .unwrap();
        assert_eq!(withdrawals.len(), 1);
        assert!(withdrawals[0].is_withdrawal());
    }
}
