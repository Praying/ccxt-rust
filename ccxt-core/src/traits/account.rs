//! Account trait definition.
//!
//! The `Account` trait provides methods for account-related operations
//! including fetching balances and trade history. These operations
//! require authentication.
//!
//! # Timestamp Format
//!
//! All timestamp parameters and return values in this trait use the standardized format:
//! - **Type**: `i64`
//! - **Unit**: Milliseconds since Unix epoch (January 1, 1970, 00:00:00 UTC)
//! - **Range**: Supports dates from 1970 to approximately year 294,276
//!
//! # Object Safety
//!
//! This trait is designed to be object-safe, allowing for dynamic dispatch via
//! trait objects (`dyn Account`).
//!
//! # Example
//!
//! ```rust,ignore
//! use ccxt_core::traits::Account;
//! use ccxt_core::types::params::BalanceParams;
//!
//! async fn check_balance(exchange: &dyn Account) -> Result<(), ccxt_core::Error> {
//!     // Fetch spot balance
//!     let balance = exchange.fetch_balance().await?;
//!     
//!     // Fetch futures balance
//!     let balance = exchange.fetch_balance_with_params(BalanceParams::futures()).await?;
//!     
//!     // Get specific currency balance
//!     let btc = exchange.get_balance("BTC").await?;
//!     
//!     // Fetch trade history with i64 timestamp
//!     let since: i64 = chrono::Utc::now().timestamp_millis() - 86400000; // 24 hours ago
//!     let trades = exchange.fetch_my_trades_since("BTC/USDT", Some(since), Some(100)).await?;
//!     
//!     Ok(())
//! }
//! ```

use async_trait::async_trait;

use crate::error::{Error, Result};
use crate::traits::PublicExchange;
use crate::types::{Balance, BalanceEntry, Trade, params::BalanceParams};

/// Trait for account-related operations.
///
/// This trait provides methods for fetching account balances and trade history.
/// All methods require authentication and are async.
///
/// # Timestamp Format
///
/// All timestamp parameters and fields in returned data structures use:
/// - **Type**: `i64`
/// - **Unit**: Milliseconds since Unix epoch (January 1, 1970, 00:00:00 UTC)
/// - **Example**: `1609459200000` represents January 1, 2021, 00:00:00 UTC
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
pub trait Account: PublicExchange {
    // ========================================================================
    // Balance
    // ========================================================================

    /// Fetch account balance (default: spot account).
    ///
    /// Returns the current balance for all currencies in the account.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let balance = exchange.fetch_balance().await?;
    /// for (currency, entry) in &balance.currencies {
    ///     println!("{}: free={}, used={}", currency, entry.free, entry.used);
    /// }
    /// ```
    async fn fetch_balance(&self) -> Result<Balance> {
        self.fetch_balance_with_params(BalanceParams::default())
            .await
    }

    /// Fetch balance with parameters.
    ///
    /// Allows specifying account type and currency filters.
    ///
    /// # Arguments
    ///
    /// * `params` - Balance parameters including account type and currency filters
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use ccxt_core::types::params::BalanceParams;
    ///
    /// // Futures balance
    /// let balance = exchange.fetch_balance_with_params(BalanceParams::futures()).await?;
    ///
    /// // Specific currencies only
    /// let balance = exchange.fetch_balance_with_params(
    ///     BalanceParams::spot().currencies(&["BTC", "USDT"])
    /// ).await?;
    /// ```
    async fn fetch_balance_with_params(&self, params: BalanceParams) -> Result<Balance>;

    /// Get balance for a specific currency.
    ///
    /// Convenience method that fetches the full balance and extracts
    /// the entry for the specified currency.
    ///
    /// # Arguments
    ///
    /// * `currency` - Currency code (e.g., "BTC", "USDT")
    ///
    /// # Returns
    ///
    /// Returns the balance entry for the currency, or an error if not found.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let btc = exchange.get_balance("BTC").await?;
    /// println!("BTC: free={}, used={}, total={}", btc.free, btc.used, btc.total);
    /// ```
    async fn get_balance(&self, currency: &str) -> Result<BalanceEntry> {
        let balance = self.fetch_balance().await?;
        balance.get(currency).cloned().ok_or_else(|| {
            Error::invalid_request(format!("Currency {currency} not found in balance"))
        })
    }

    // ========================================================================
    // Trade History
    // ========================================================================

    /// Fetch user's trade history for a symbol.
    ///
    /// Returns the user's executed trades for the specified symbol.
    ///
    /// # Arguments
    ///
    /// * `symbol` - Trading pair symbol (e.g., "BTC/USDT")
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let trades = exchange.fetch_my_trades("BTC/USDT").await?;
    /// for trade in trades {
    ///     println!("{}: {} {} @ {}", trade.id, trade.side, trade.amount, trade.price);
    /// }
    /// ```
    async fn fetch_my_trades(&self, symbol: &str) -> Result<Vec<Trade>> {
        self.fetch_my_trades_since(symbol, None, None).await
    }

    /// Fetch trades with pagination.
    ///
    /// # Arguments
    ///
    /// * `symbol` - Trading pair symbol (e.g., "BTC/USDT")
    /// * `since` - Optional start timestamp in milliseconds (i64) since Unix epoch
    /// * `limit` - Optional maximum number of trades to return
    ///
    /// # Timestamp Format
    ///
    /// The `since` parameter uses `i64` milliseconds since Unix epoch:
    /// - `1609459200000` = January 1, 2021, 00:00:00 UTC
    /// - `chrono::Utc::now().timestamp_millis()` = Current time
    /// - `chrono::Utc::now().timestamp_millis() - 86400000` = 24 hours ago
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// // Recent trades (no timestamp filter)
    /// let trades = exchange.fetch_my_trades_since("BTC/USDT", None, Some(100)).await?;
    ///
    /// // Trades from the last 24 hours
    /// let since = chrono::Utc::now().timestamp_millis() - 86400000;
    /// let trades = exchange.fetch_my_trades_since(
    ///     "BTC/USDT",
    ///     Some(since),
    ///     Some(50)
    /// ).await?;
    /// ```
    async fn fetch_my_trades_since(
        &self,
        symbol: &str,
        since: Option<i64>,
        limit: Option<u32>,
    ) -> Result<Vec<Trade>>;

    // ========================================================================
    // Deprecated u64 Wrapper Methods (Backward Compatibility)
    // ========================================================================

    /// Fetch user's trade history with u64 timestamp filtering (deprecated).
    ///
    /// **DEPRECATED**: Use `fetch_my_trades_since` with i64 timestamps instead.
    /// This method is provided for backward compatibility during migration.
    ///
    /// # Migration
    ///
    /// ```rust,ignore
    /// // Old code (deprecated)
    /// let trades = exchange.fetch_my_trades_since_u64("BTC/USDT", Some(1609459200000u64), Some(100)).await?;
    ///
    /// // New code (recommended)
    /// let trades = exchange.fetch_my_trades_since("BTC/USDT", Some(1609459200000i64), Some(100)).await?;
    /// ```
    #[deprecated(
        since = "0.1.0",
        note = "Use fetch_my_trades_since with i64 timestamps. Convert using TimestampUtils::u64_to_i64()"
    )]
    async fn fetch_my_trades_since_u64(
        &self,
        symbol: &str,
        since: Option<u64>,
        limit: Option<u32>,
    ) -> Result<Vec<Trade>> {
        use crate::time::TimestampConversion;

        let since_i64 = since.to_i64()?;
        self.fetch_my_trades_since(symbol, since_i64, limit).await
    }
}

/// Type alias for boxed Account trait object.
pub type BoxedAccount = Box<dyn Account>;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capability::ExchangeCapabilities;
    use crate::types::Timeframe;

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
    impl Account for MockExchange {
        async fn fetch_balance_with_params(&self, _params: BalanceParams) -> Result<Balance> {
            let mut balance = Balance::new();
            balance.set(
                "BTC".to_string(),
                BalanceEntry {
                    free: rust_decimal_macros::dec!(1.5),
                    used: rust_decimal_macros::dec!(0.5),
                    total: rust_decimal_macros::dec!(2.0),
                },
            );
            balance.set(
                "USDT".to_string(),
                BalanceEntry {
                    free: rust_decimal_macros::dec!(10000),
                    used: rust_decimal_macros::dec!(5000),
                    total: rust_decimal_macros::dec!(15000),
                },
            );

            Ok(balance)
        }

        async fn fetch_my_trades_since(
            &self,
            _symbol: &str,
            _since: Option<i64>,
            _limit: Option<u32>,
        ) -> Result<Vec<Trade>> {
            Ok(vec![])
        }
    }

    #[test]
    fn test_trait_object_safety() {
        // Verify trait is object-safe by creating a trait object
        let _exchange: BoxedAccount = Box::new(MockExchange);
    }

    #[tokio::test]
    async fn test_fetch_balance() {
        let exchange = MockExchange;

        let balance = exchange.fetch_balance().await.unwrap();
        assert!(balance.get("BTC").is_some());
        assert!(balance.get("USDT").is_some());
    }

    #[tokio::test]
    async fn test_fetch_balance_with_params() {
        let exchange = MockExchange;

        let balance = exchange
            .fetch_balance_with_params(BalanceParams::futures())
            .await
            .unwrap();
        assert!(balance.get("BTC").is_some());
    }

    #[tokio::test]
    async fn test_get_balance() {
        let exchange = MockExchange;

        let btc = exchange.get_balance("BTC").await.unwrap();
        assert_eq!(btc.free, rust_decimal_macros::dec!(1.5));
        assert_eq!(btc.total, rust_decimal_macros::dec!(2.0));

        // Test non-existent currency
        let result = exchange.get_balance("XYZ").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_fetch_my_trades() {
        let exchange = MockExchange;

        let trades = exchange.fetch_my_trades("BTC/USDT").await.unwrap();
        assert!(trades.is_empty());
    }

    #[tokio::test]
    async fn test_fetch_my_trades_since() {
        let exchange = MockExchange;

        let trades = exchange
            .fetch_my_trades_since("BTC/USDT", Some(1609459200000), Some(100))
            .await
            .unwrap();
        assert!(trades.is_empty());
    }
}
