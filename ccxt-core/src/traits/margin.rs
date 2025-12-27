//! Margin trait definition.
//!
//! The `Margin` trait provides methods for margin and futures trading operations
//! including position management, leverage configuration, and funding rate queries.
//! These operations require authentication.
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
//! trait objects (`dyn Margin`).
//!
//! # Example
//!
//! ```rust,ignore
//! use ccxt_core::traits::Margin;
//! use ccxt_core::types::params::LeverageParams;
//!
//! async fn manage_positions(exchange: &dyn Margin) -> Result<(), ccxt_core::Error> {
//!     // Fetch all positions
//!     let positions = exchange.fetch_positions().await?;
//!     
//!     // Set leverage
//!     exchange.set_leverage("BTC/USDT:USDT", 10).await?;
//!     
//!     // Fetch funding rate history with i64 timestamp
//!     let since: i64 = chrono::Utc::now().timestamp_millis() - 604800000; // 7 days ago
//!     let history = exchange.fetch_funding_rate_history("BTC/USDT:USDT", Some(since), Some(100)).await?;
//!     
//!     Ok(())
//! }
//! ```

use async_trait::async_trait;

use crate::error::Result;
use crate::traits::PublicExchange;
use crate::types::{
    FundingRate, FundingRateHistory, Position,
    params::{LeverageParams, MarginMode},
};

/// Trait for margin and futures trading operations.
///
/// This trait provides methods for managing positions, leverage, margin mode,
/// and funding rates. All methods require authentication and are async.
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
pub trait Margin: PublicExchange {
    // ========================================================================
    // Position Management
    // ========================================================================

    /// Fetch all positions.
    ///
    /// Returns all open positions across all symbols.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let positions = exchange.fetch_positions().await?;
    /// for pos in positions {
    ///     println!("{}: {} contracts @ {}", pos.symbol, pos.contracts.unwrap_or(0.0), pos.entry_price.unwrap_or(0.0));
    /// }
    /// ```
    async fn fetch_positions(&self) -> Result<Vec<Position>> {
        self.fetch_positions_for(&[]).await
    }

    /// Fetch positions for specific symbols.
    ///
    /// # Arguments
    ///
    /// * `symbols` - Array of trading pair symbols (e.g., ["BTC/USDT:USDT", "ETH/USDT:USDT"])
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let positions = exchange.fetch_positions_for(&["BTC/USDT:USDT", "ETH/USDT:USDT"]).await?;
    /// ```
    async fn fetch_positions_for(&self, symbols: &[&str]) -> Result<Vec<Position>>;

    /// Fetch a single position for a symbol.
    ///
    /// # Arguments
    ///
    /// * `symbol` - Trading pair symbol (e.g., "BTC/USDT:USDT")
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let position = exchange.fetch_position("BTC/USDT:USDT").await?;
    /// println!("PnL: {}", position.unrealized_pnl.unwrap_or(0.0));
    /// ```
    async fn fetch_position(&self, symbol: &str) -> Result<Position>;

    // ========================================================================
    // Leverage Management
    // ========================================================================

    /// Set leverage for a symbol.
    ///
    /// # Arguments
    ///
    /// * `symbol` - Trading pair symbol
    /// * `leverage` - Leverage multiplier (e.g., 10 for 10x)
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// exchange.set_leverage("BTC/USDT:USDT", 10).await?;
    /// ```
    async fn set_leverage(&self, symbol: &str, leverage: u32) -> Result<()> {
        self.set_leverage_with_params(LeverageParams::new(symbol, leverage))
            .await
    }

    /// Set leverage with full parameters.
    ///
    /// Allows specifying margin mode along with leverage.
    ///
    /// # Arguments
    ///
    /// * `params` - Leverage parameters including symbol, leverage, and optional margin mode
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use ccxt_core::types::params::LeverageParams;
    ///
    /// exchange.set_leverage_with_params(
    ///     LeverageParams::new("BTC/USDT:USDT", 10).isolated()
    /// ).await?;
    /// ```
    async fn set_leverage_with_params(&self, params: LeverageParams) -> Result<()>;

    /// Get current leverage for a symbol.
    ///
    /// # Arguments
    ///
    /// * `symbol` - Trading pair symbol
    ///
    /// # Returns
    ///
    /// Returns the current leverage multiplier.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let leverage = exchange.get_leverage("BTC/USDT:USDT").await?;
    /// println!("Current leverage: {}x", leverage);
    /// ```
    async fn get_leverage(&self, symbol: &str) -> Result<u32>;

    // ========================================================================
    // Margin Mode
    // ========================================================================

    /// Set margin mode for a symbol.
    ///
    /// # Arguments
    ///
    /// * `symbol` - Trading pair symbol
    /// * `mode` - Margin mode (Cross or Isolated)
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use ccxt_core::types::params::MarginMode;
    ///
    /// exchange.set_margin_mode("BTC/USDT:USDT", MarginMode::Isolated).await?;
    /// ```
    async fn set_margin_mode(&self, symbol: &str, mode: MarginMode) -> Result<()>;

    // ========================================================================
    // Funding Rates
    // ========================================================================

    /// Fetch current funding rate for a symbol.
    ///
    /// # Arguments
    ///
    /// * `symbol` - Trading pair symbol
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let rate = exchange.fetch_funding_rate("BTC/USDT:USDT").await?;
    /// println!("Funding rate: {}", rate.funding_rate.unwrap_or(0.0));
    /// ```
    async fn fetch_funding_rate(&self, symbol: &str) -> Result<FundingRate>;

    /// Fetch funding rates for multiple symbols.
    ///
    /// # Arguments
    ///
    /// * `symbols` - Array of trading pair symbols
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let rates = exchange.fetch_funding_rates(&["BTC/USDT:USDT", "ETH/USDT:USDT"]).await?;
    /// ```
    async fn fetch_funding_rates(&self, symbols: &[&str]) -> Result<Vec<FundingRate>>;

    /// Fetch funding rate history for a symbol.
    ///
    /// # Arguments
    ///
    /// * `symbol` - Trading pair symbol (e.g., "BTC/USDT:USDT" for futures)
    /// * `since` - Optional start timestamp in milliseconds (i64) since Unix epoch
    /// * `limit` - Optional maximum number of records to return
    ///
    /// # Timestamp Format
    ///
    /// The `since` parameter uses `i64` milliseconds since Unix epoch:
    /// - `1609459200000` = January 1, 2021, 00:00:00 UTC
    /// - `chrono::Utc::now().timestamp_millis()` = Current time
    /// - `chrono::Utc::now().timestamp_millis() - 604800000` = 7 days ago
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// // Fetch recent funding rate history
    /// let history = exchange.fetch_funding_rate_history(
    ///     "BTC/USDT:USDT",
    ///     None,
    ///     Some(100)
    /// ).await?;
    ///
    /// // Fetch funding rate history from the last 7 days
    /// let since = chrono::Utc::now().timestamp_millis() - 604800000;
    /// let history = exchange.fetch_funding_rate_history(
    ///     "BTC/USDT:USDT",
    ///     Some(since),
    ///     Some(50)
    /// ).await?;
    /// ```
    async fn fetch_funding_rate_history(
        &self,
        symbol: &str,
        since: Option<i64>,
        limit: Option<u32>,
    ) -> Result<Vec<FundingRateHistory>>;

    // ========================================================================
    // Deprecated u64 Wrapper Methods (Backward Compatibility)
    // ========================================================================

    /// Fetch funding rate history with u64 timestamp filtering (deprecated).
    ///
    /// **DEPRECATED**: Use `fetch_funding_rate_history` with i64 timestamps instead.
    /// This method is provided for backward compatibility during migration.
    ///
    /// # Migration
    ///
    /// ```rust,ignore
    /// // Old code (deprecated)
    /// let history = exchange.fetch_funding_rate_history_u64("BTC/USDT:USDT", Some(1609459200000u64), Some(100)).await?;
    ///
    /// // New code (recommended)
    /// let history = exchange.fetch_funding_rate_history("BTC/USDT:USDT", Some(1609459200000i64), Some(100)).await?;
    /// ```
    #[deprecated(
        since = "0.x.0",
        note = "Use fetch_funding_rate_history with i64 timestamps. Convert using TimestampUtils::u64_to_i64()"
    )]
    async fn fetch_funding_rate_history_u64(
        &self,
        symbol: &str,
        since: Option<u64>,
        limit: Option<u32>,
    ) -> Result<Vec<FundingRateHistory>> {
        use crate::time::TimestampConversion;

        let since_i64 = since.to_i64()?;
        self.fetch_funding_rate_history(symbol, since_i64, limit)
            .await
    }
}

/// Type alias for boxed Margin trait object.
pub type BoxedMargin = Box<dyn Margin>;

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
    impl Margin for MockExchange {
        async fn fetch_positions_for(&self, _symbols: &[&str]) -> Result<Vec<Position>> {
            Ok(vec![Position {
                symbol: "BTC/USDT:USDT".to_string(),
                contracts: Some(1.0),
                entry_price: Some(50000.0),
                leverage: Some(10.0),
                unrealized_pnl: Some(100.0),
                ..Default::default()
            }])
        }

        async fn fetch_position(&self, symbol: &str) -> Result<Position> {
            Ok(Position {
                symbol: symbol.to_string(),
                contracts: Some(1.0),
                entry_price: Some(50000.0),
                leverage: Some(10.0),
                ..Default::default()
            })
        }

        async fn set_leverage_with_params(&self, _params: LeverageParams) -> Result<()> {
            Ok(())
        }

        async fn get_leverage(&self, _symbol: &str) -> Result<u32> {
            Ok(10)
        }

        async fn set_margin_mode(&self, _symbol: &str, _mode: MarginMode) -> Result<()> {
            Ok(())
        }

        async fn fetch_funding_rate(&self, symbol: &str) -> Result<FundingRate> {
            Ok(FundingRate {
                symbol: symbol.to_string(),
                funding_rate: Some(0.0001),
                ..Default::default()
            })
        }

        async fn fetch_funding_rates(&self, symbols: &[&str]) -> Result<Vec<FundingRate>> {
            Ok(symbols
                .iter()
                .map(|s| FundingRate {
                    symbol: s.to_string(),
                    funding_rate: Some(0.0001),
                    ..Default::default()
                })
                .collect())
        }

        async fn fetch_funding_rate_history(
            &self,
            symbol: &str,
            _since: Option<i64>,
            _limit: Option<u32>,
        ) -> Result<Vec<FundingRateHistory>> {
            Ok(vec![FundingRateHistory {
                symbol: symbol.to_string(),
                funding_rate: Some(0.0001),
                timestamp: Some(1609459200000),
                ..Default::default()
            }])
        }
    }

    #[test]
    fn test_trait_object_safety() {
        // Verify trait is object-safe by creating a trait object
        let _exchange: BoxedMargin = Box::new(MockExchange);
    }

    #[tokio::test]
    async fn test_fetch_positions() {
        let exchange = MockExchange;

        let positions = exchange.fetch_positions().await.unwrap();
        assert_eq!(positions.len(), 1);
        assert_eq!(positions[0].symbol, "BTC/USDT:USDT");
    }

    #[tokio::test]
    async fn test_fetch_position() {
        let exchange = MockExchange;

        let position = exchange.fetch_position("BTC/USDT:USDT").await.unwrap();
        assert_eq!(position.symbol, "BTC/USDT:USDT");
        assert_eq!(position.contracts, Some(1.0));
    }

    #[tokio::test]
    async fn test_set_leverage() {
        let exchange = MockExchange;

        let result = exchange.set_leverage("BTC/USDT:USDT", 10).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_get_leverage() {
        let exchange = MockExchange;

        let leverage = exchange.get_leverage("BTC/USDT:USDT").await.unwrap();
        assert_eq!(leverage, 10);
    }

    #[tokio::test]
    async fn test_set_margin_mode() {
        let exchange = MockExchange;

        let result = exchange
            .set_margin_mode("BTC/USDT:USDT", MarginMode::Isolated)
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_fetch_funding_rate() {
        let exchange = MockExchange;

        let rate = exchange.fetch_funding_rate("BTC/USDT:USDT").await.unwrap();
        assert_eq!(rate.symbol, "BTC/USDT:USDT");
        assert_eq!(rate.funding_rate, Some(0.0001));
    }

    #[tokio::test]
    async fn test_fetch_funding_rates() {
        let exchange = MockExchange;

        let rates = exchange
            .fetch_funding_rates(&["BTC/USDT:USDT", "ETH/USDT:USDT"])
            .await
            .unwrap();
        assert_eq!(rates.len(), 2);
    }

    #[tokio::test]
    async fn test_fetch_funding_rate_history() {
        let exchange = MockExchange;

        let history = exchange
            .fetch_funding_rate_history("BTC/USDT:USDT", None, Some(100))
            .await
            .unwrap();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].funding_rate, Some(0.0001));
    }
}
