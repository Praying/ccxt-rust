//! Exchange trait hierarchy for modular capability composition.
//!
//! This module provides a decomposed trait hierarchy that allows exchanges to
//! implement only the capabilities they support, without requiring stub
//! implementations for unsupported features.
//!
//! # Trait Hierarchy
//!
//! ```text
//! PublicExchange (base trait - metadata, capabilities)
//!     │
//!     ├── MarketData (public market data)
//!     ├── Trading (order management)
//!     ├── Account (balance, trade history)
//!     ├── Margin (positions, leverage, funding)
//!     └── Funding (deposits, withdrawals, transfers)
//!
//! FullExchange = PublicExchange + MarketData + Trading + Account + Margin + Funding
//! ```
//!
//! # Object Safety
//!
//! All traits are designed to be object-safe, allowing for dynamic dispatch:
//!
//! ```rust,ignore
//! use ccxt_core::traits::{BoxedMarketData, BoxedTrading, BoxedFullExchange};
//!
//! let market_data: BoxedMarketData = Box::new(my_exchange);
//! let trading: BoxedTrading = Box::new(my_exchange);
//! let full: BoxedFullExchange = Box::new(my_exchange);
//! ```
//!
//! # Thread Safety
//!
//! All traits require `Send + Sync` bounds for async runtime compatibility.
//!
//! # Example
//!
//! ```rust,ignore
//! use ccxt_core::traits::{PublicExchange, MarketData, Trading};
//!
//! // Exchange that only supports market data (no trading)
//! struct ReadOnlyExchange;
//!
//! impl PublicExchange for ReadOnlyExchange {
//!     fn id(&self) -> &str { "readonly" }
//!     fn name(&self) -> &str { "Read Only Exchange" }
//!     // ...
//! }
//!
//! impl MarketData for ReadOnlyExchange {
//!     // Only implement market data methods
//! }
//!
//! // Full-featured exchange
//! struct FullFeaturedExchange;
//!
//! impl PublicExchange for FullFeaturedExchange { /* ... */ }
//! impl MarketData for FullFeaturedExchange { /* ... */ }
//! impl Trading for FullFeaturedExchange { /* ... */ }
//! impl Account for FullFeaturedExchange { /* ... */ }
//! impl Margin for FullFeaturedExchange { /* ... */ }
//! impl Funding for FullFeaturedExchange { /* ... */ }
//! // Automatically implements FullExchange via blanket impl
//! ```

use std::sync::Arc;

// Trait modules
mod account;
mod funding;
mod margin;
mod market_data;
mod public_exchange;
mod trading;

// Re-export traits
pub use account::{Account, BoxedAccount};
pub use funding::{BoxedFunding, Funding};
pub use margin::{BoxedMargin, Margin};
pub use market_data::{BoxedMarketData, MarketData};
pub use public_exchange::PublicExchange;
pub use trading::{BoxedTrading, Trading};

// ============================================================================
// FullExchange Trait
// ============================================================================

/// Combined trait for exchanges supporting all capabilities.
///
/// This trait is automatically implemented for any type that implements
/// all of the component traits: `PublicExchange`, `MarketData`, `Trading`,
/// `Account`, `Margin`, and `Funding`.
///
/// # Example
///
/// ```rust,ignore
/// use ccxt_core::traits::FullExchange;
///
/// async fn use_full_exchange<E: FullExchange>(exchange: &E) {
///     // Can use all exchange capabilities
///     let markets = exchange.fetch_markets().await?;
///     let balance = exchange.fetch_balance().await?;
///     let positions = exchange.fetch_positions().await?;
/// }
/// ```
pub trait FullExchange: PublicExchange + MarketData + Trading + Account + Margin + Funding {}

/// Blanket implementation of FullExchange for any type implementing all component traits.
impl<T> FullExchange for T where
    T: PublicExchange + MarketData + Trading + Account + Margin + Funding
{
}

// ============================================================================
// Type Aliases for Trait Objects
// ============================================================================

/// Type alias for boxed FullExchange trait object.
///
/// Use this when you need a heap-allocated exchange with all capabilities
/// and single ownership.
///
/// # Example
///
/// ```rust,ignore
/// let exchange: BoxedFullExchange = Box::new(my_exchange);
/// ```
pub type BoxedFullExchange = Box<dyn FullExchange>;

/// Type alias for Arc-wrapped FullExchange trait object.
///
/// Use this when you need shared ownership of an exchange across multiple
/// tasks or threads.
///
/// # Example
///
/// ```rust,ignore
/// let exchange: ArcFullExchange = Arc::new(my_exchange);
/// let exchange_clone = exchange.clone();
/// tokio::spawn(async move {
///     exchange_clone.fetch_ticker("BTC/USDT").await
/// });
/// ```
pub type ArcFullExchange = Arc<dyn FullExchange>;

/// Type alias for boxed PublicExchange trait object.
pub type BoxedPublicExchange = Box<dyn PublicExchange>;

/// Type alias for Arc-wrapped PublicExchange trait object.
pub type ArcPublicExchange = Arc<dyn PublicExchange>;

/// Type alias for Arc-wrapped MarketData trait object.
pub type ArcMarketData = Arc<dyn MarketData>;

/// Type alias for Arc-wrapped Trading trait object.
pub type ArcTrading = Arc<dyn Trading>;

/// Type alias for Arc-wrapped Account trait object.
pub type ArcAccount = Arc<dyn Account>;

/// Type alias for Arc-wrapped Margin trait object.
pub type ArcMargin = Arc<dyn Margin>;

/// Type alias for Arc-wrapped Funding trait object.
pub type ArcFunding = Arc<dyn Funding>;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capability::ExchangeCapabilities;
    use crate::error::Result;
    use crate::types::{
        Balance, BalanceEntry, DepositAddress, FundingRate, FundingRateHistory, Market, Ohlcv,
        Order, OrderBook, Position, Ticker, Timeframe, Trade, Transaction, Transfer,
        params::{
            BalanceParams, LeverageParams, MarginMode, OhlcvParams, OrderBookParams, OrderParams,
            TransferParams, WithdrawParams,
        },
        transaction::{TransactionStatus, TransactionType},
    };
    use async_trait::async_trait;
    use rust_decimal_macros::dec;
    use std::collections::HashMap;

    // Full mock implementation for testing FullExchange
    struct MockFullExchange;

    impl PublicExchange for MockFullExchange {
        fn id(&self) -> &str {
            "mock_full"
        }
        fn name(&self) -> &str {
            "Mock Full Exchange"
        }
        fn capabilities(&self) -> ExchangeCapabilities {
            ExchangeCapabilities::all()
        }
        fn timeframes(&self) -> Vec<Timeframe> {
            vec![Timeframe::H1, Timeframe::D1]
        }
    }

    #[async_trait]
    impl MarketData for MockFullExchange {
        async fn fetch_markets(&self) -> Result<Vec<Market>> {
            Ok(vec![])
        }

        async fn load_markets_with_reload(&self, _reload: bool) -> Result<HashMap<String, Market>> {
            Ok(HashMap::new())
        }

        async fn fetch_ticker(&self, symbol: &str) -> Result<Ticker> {
            Ok(Ticker {
                symbol: symbol.to_string(),
                ..Default::default()
            })
        }

        async fn fetch_tickers(&self, _symbols: &[&str]) -> Result<Vec<Ticker>> {
            Ok(vec![])
        }

        async fn fetch_order_book_with_params(
            &self,
            symbol: &str,
            _params: OrderBookParams,
        ) -> Result<OrderBook> {
            Ok(OrderBook::new(symbol.to_string(), 0))
        }

        async fn fetch_trades_with_limit(
            &self,
            _symbol: &str,
            _since: Option<i64>,
            _limit: Option<u32>,
        ) -> Result<Vec<Trade>> {
            Ok(vec![])
        }

        async fn fetch_ohlcv_with_params(
            &self,
            _symbol: &str,
            _params: OhlcvParams,
        ) -> Result<Vec<Ohlcv>> {
            Ok(vec![])
        }

        async fn market(&self, symbol: &str) -> Result<Market> {
            Ok(Market {
                symbol: symbol.to_string(),
                ..Default::default()
            })
        }

        async fn markets(&self) -> HashMap<String, Market> {
            HashMap::new()
        }
    }

    #[async_trait]
    impl Trading for MockFullExchange {
        async fn create_order(&self, params: OrderParams) -> Result<Order> {
            Ok(Order::new(
                "order_123".to_string(),
                params.symbol,
                params.order_type,
                params.side,
                params.amount,
                params.price,
                crate::types::OrderStatus::Open,
            ))
        }

        async fn cancel_order(&self, id: &str, symbol: &str) -> Result<Order> {
            Ok(Order::new(
                id.to_string(),
                symbol.to_string(),
                crate::types::OrderType::Limit,
                crate::types::OrderSide::Buy,
                dec!(0),
                None,
                crate::types::OrderStatus::Cancelled,
            ))
        }

        async fn cancel_all_orders(&self, _symbol: &str) -> Result<Vec<Order>> {
            Ok(vec![])
        }

        async fn fetch_order(&self, id: &str, symbol: &str) -> Result<Order> {
            Ok(Order::new(
                id.to_string(),
                symbol.to_string(),
                crate::types::OrderType::Limit,
                crate::types::OrderSide::Buy,
                dec!(0),
                None,
                crate::types::OrderStatus::Open,
            ))
        }

        async fn fetch_open_orders(&self, _symbol: Option<&str>) -> Result<Vec<Order>> {
            Ok(vec![])
        }

        async fn fetch_closed_orders(
            &self,
            _symbol: Option<&str>,
            _since: Option<i64>,
            _limit: Option<u32>,
        ) -> Result<Vec<Order>> {
            Ok(vec![])
        }
    }

    #[async_trait]
    impl Account for MockFullExchange {
        async fn fetch_balance_with_params(&self, _params: BalanceParams) -> Result<Balance> {
            let mut balance = Balance::new();
            balance.set("USDT".to_string(), BalanceEntry::new(dec!(10000), dec!(0)));
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

    #[async_trait]
    impl Margin for MockFullExchange {
        async fn fetch_positions_for(&self, _symbols: &[&str]) -> Result<Vec<Position>> {
            Ok(vec![])
        }

        async fn fetch_position(&self, symbol: &str) -> Result<Position> {
            Ok(Position {
                symbol: symbol.to_string(),
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
                ..Default::default()
            })
        }

        async fn fetch_funding_rates(&self, _symbols: &[&str]) -> Result<Vec<FundingRate>> {
            Ok(vec![])
        }

        async fn fetch_funding_rate_history(
            &self,
            _symbol: &str,
            _since: Option<i64>,
            _limit: Option<u32>,
        ) -> Result<Vec<FundingRateHistory>> {
            Ok(vec![])
        }
    }

    #[async_trait]
    impl Funding for MockFullExchange {
        async fn fetch_deposit_address(&self, code: &str) -> Result<DepositAddress> {
            Ok(DepositAddress::new(code.to_string(), "0x123".to_string()))
        }

        async fn fetch_deposit_address_on_network(
            &self,
            code: &str,
            network: &str,
        ) -> Result<DepositAddress> {
            let mut addr = DepositAddress::new(code.to_string(), "0x123".to_string());
            addr.network = Some(network.to_string());
            Ok(addr)
        }

        async fn withdraw(&self, params: WithdrawParams) -> Result<Transaction> {
            Ok(Transaction::new(
                "tx_123".to_string(),
                TransactionType::Withdrawal,
                params.amount,
                params.currency,
                TransactionStatus::Pending,
            ))
        }

        async fn transfer(&self, params: TransferParams) -> Result<Transfer> {
            Ok(Transfer {
                id: Some("transfer_123".to_string()),
                timestamp: 0,
                datetime: "".to_string(),
                currency: params.currency,
                amount: 0.0,
                from_account: None,
                to_account: None,
                status: "success".to_string(),
                info: None,
            })
        }

        async fn fetch_deposits(
            &self,
            _code: Option<&str>,
            _since: Option<i64>,
            _limit: Option<u32>,
        ) -> Result<Vec<Transaction>> {
            Ok(vec![])
        }

        async fn fetch_withdrawals(
            &self,
            _code: Option<&str>,
            _since: Option<i64>,
            _limit: Option<u32>,
        ) -> Result<Vec<Transaction>> {
            Ok(vec![])
        }
    }

    #[test]
    fn test_full_exchange_blanket_impl() {
        // MockFullExchange should automatically implement FullExchange
        fn assert_full_exchange<T: FullExchange>(_: &T) {}
        let exchange = MockFullExchange;
        assert_full_exchange(&exchange);
    }

    #[test]
    fn test_boxed_full_exchange() {
        let _exchange: BoxedFullExchange = Box::new(MockFullExchange);
    }

    #[test]
    fn test_arc_full_exchange() {
        let _exchange: ArcFullExchange = Arc::new(MockFullExchange);
    }

    #[tokio::test]
    async fn test_full_exchange_methods() {
        let exchange = MockFullExchange;

        // Test methods from different traits
        assert_eq!(exchange.id(), "mock_full");

        let ticker = exchange.fetch_ticker("BTC/USDT").await.unwrap();
        assert_eq!(ticker.symbol, "BTC/USDT");

        let balance = exchange.fetch_balance().await.unwrap();
        assert!(balance.get("USDT").is_some());

        let positions = exchange.fetch_positions().await.unwrap();
        assert!(positions.is_empty());

        let address = exchange.fetch_deposit_address("USDT").await.unwrap();
        assert_eq!(address.currency, "USDT");
    }

    #[test]
    fn test_arc_type_aliases() {
        let exchange = Arc::new(MockFullExchange);

        // All Arc type aliases should work
        let _: ArcFullExchange = exchange.clone();
        let _: ArcMarketData = exchange.clone();
        let _: ArcTrading = exchange.clone();
        let _: ArcAccount = exchange.clone();
        let _: ArcMargin = exchange.clone();
        let _: ArcFunding = exchange.clone();
    }
}
