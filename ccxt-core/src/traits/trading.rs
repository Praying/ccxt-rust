//! Trading trait definition.
//!
//! The `Trading` trait provides methods for order management operations
//! including creating, canceling, and fetching orders. These operations
//! require authentication.
//!
//! # Object Safety
//!
//! This trait is designed to be object-safe, allowing for dynamic dispatch via
//! trait objects (`dyn Trading`).
//!
//! # Example
//!
//! ```rust,ignore
//! use ccxt_core::traits::Trading;
//! use ccxt_core::types::params::OrderParams;
//! use rust_decimal_macros::dec;
//!
//! async fn place_order(exchange: &dyn Trading) -> Result<(), ccxt_core::Error> {
//!     // Market buy
//!     let order = exchange.market_buy("BTC/USDT", dec!(0.01)).await?;
//!     
//!     // Limit sell with options
//!     let order = exchange.create_order(
//!         OrderParams::limit_sell("BTC/USDT", dec!(0.01), dec!(50000))
//!             .time_in_force(ccxt_core::types::TimeInForce::GTC)
//!     ).await?;
//!     
//!     Ok(())
//! }
//! ```

use async_trait::async_trait;
use rust_decimal::Decimal;

use crate::error::Result;
use crate::traits::PublicExchange;
use crate::types::{Order, params::OrderParams};

/// Trait for order management operations.
///
/// This trait provides methods for creating, canceling, and fetching orders.
/// All methods require authentication and are async.
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
pub trait Trading: PublicExchange {
    // ========================================================================
    // Order Creation
    // ========================================================================

    /// Create a new order using OrderParams builder.
    ///
    /// This is the primary method for creating orders. Use the `OrderParams`
    /// builder for ergonomic order construction.
    ///
    /// # Arguments
    ///
    /// * `params` - Order parameters including symbol, side, type, amount, price
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use ccxt_core::types::params::OrderParams;
    /// use rust_decimal_macros::dec;
    ///
    /// // Market buy
    /// let order = exchange.create_order(
    ///     OrderParams::market_buy("BTC/USDT", dec!(0.01))
    /// ).await?;
    ///
    /// // Limit sell with custom options
    /// let order = exchange.create_order(
    ///     OrderParams::limit_sell("BTC/USDT", dec!(0.01), dec!(50000))
    ///         .time_in_force(TimeInForce::IOC)
    ///         .client_id("my-order-123")
    /// ).await?;
    /// ```
    async fn create_order(&self, params: OrderParams) -> Result<Order>;

    /// Convenience method for market buy order.
    ///
    /// # Arguments
    ///
    /// * `symbol` - Trading pair symbol (e.g., "BTC/USDT")
    /// * `amount` - Order amount
    async fn market_buy(&self, symbol: &str, amount: Decimal) -> Result<Order> {
        self.create_order(OrderParams::market_buy(symbol, amount))
            .await
    }

    /// Convenience method for market sell order.
    ///
    /// # Arguments
    ///
    /// * `symbol` - Trading pair symbol
    /// * `amount` - Order amount
    async fn market_sell(&self, symbol: &str, amount: Decimal) -> Result<Order> {
        self.create_order(OrderParams::market_sell(symbol, amount))
            .await
    }

    /// Convenience method for limit buy order.
    ///
    /// # Arguments
    ///
    /// * `symbol` - Trading pair symbol
    /// * `amount` - Order amount
    /// * `price` - Limit price
    async fn limit_buy(&self, symbol: &str, amount: Decimal, price: Decimal) -> Result<Order> {
        self.create_order(OrderParams::limit_buy(symbol, amount, price))
            .await
    }

    /// Convenience method for limit sell order.
    ///
    /// # Arguments
    ///
    /// * `symbol` - Trading pair symbol
    /// * `amount` - Order amount
    /// * `price` - Limit price
    async fn limit_sell(&self, symbol: &str, amount: Decimal, price: Decimal) -> Result<Order> {
        self.create_order(OrderParams::limit_sell(symbol, amount, price))
            .await
    }

    // ========================================================================
    // Order Cancellation
    // ========================================================================

    /// Cancel an existing order.
    ///
    /// # Arguments
    ///
    /// * `id` - Order ID to cancel
    /// * `symbol` - Trading pair symbol (required for most exchanges)
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let cancelled = exchange.cancel_order("12345", "BTC/USDT").await?;
    /// println!("Cancelled order: {}", cancelled.id);
    /// ```
    async fn cancel_order(&self, id: &str, symbol: &str) -> Result<Order>;

    /// Cancel all orders for a symbol.
    ///
    /// # Arguments
    ///
    /// * `symbol` - Trading pair symbol
    ///
    /// # Returns
    ///
    /// Returns a vector of cancelled orders.
    async fn cancel_all_orders(&self, symbol: &str) -> Result<Vec<Order>>;

    // ========================================================================
    // Order Queries
    // ========================================================================

    /// Fetch a specific order by ID.
    ///
    /// # Arguments
    ///
    /// * `id` - Order ID
    /// * `symbol` - Trading pair symbol (required for most exchanges)
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let order = exchange.fetch_order("12345", "BTC/USDT").await?;
    /// println!("Order status: {:?}", order.status);
    /// ```
    async fn fetch_order(&self, id: &str, symbol: &str) -> Result<Order>;

    /// Fetch open orders.
    ///
    /// # Arguments
    ///
    /// * `symbol` - Optional trading pair symbol. If `None`, returns all open orders.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// // All open orders
    /// let orders = exchange.fetch_open_orders(None).await?;
    ///
    /// // Open orders for specific symbol
    /// let orders = exchange.fetch_open_orders(Some("BTC/USDT")).await?;
    /// ```
    async fn fetch_open_orders(&self, symbol: Option<&str>) -> Result<Vec<Order>>;

    /// Fetch closed orders with pagination.
    ///
    /// # Arguments
    ///
    /// * `symbol` - Optional trading pair symbol
    /// * `since` - Optional start timestamp in milliseconds
    /// * `limit` - Optional maximum number of orders to return
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// // Recent closed orders
    /// let orders = exchange.fetch_closed_orders(Some("BTC/USDT"), None, Some(100)).await?;
    ///
    /// // Closed orders since timestamp
    /// let orders = exchange.fetch_closed_orders(
    ///     Some("BTC/USDT"),
    ///     Some(1609459200000),
    ///     Some(50)
    /// ).await?;
    /// ```
    async fn fetch_closed_orders(
        &self,
        symbol: Option<&str>,
        since: Option<i64>,
        limit: Option<u32>,
    ) -> Result<Vec<Order>>;
}

/// Type alias for boxed Trading trait object.
pub type BoxedTrading = Box<dyn Trading>;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capability::ExchangeCapabilities;
    use crate::types::{OrderSide, OrderStatus, OrderType, Timeframe};

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
    impl Trading for MockExchange {
        async fn create_order(&self, params: OrderParams) -> Result<Order> {
            Ok(Order::new(
                "test-order-123".to_string(),
                params.symbol,
                params.order_type,
                params.side,
                params.amount,
                params.price,
                OrderStatus::Open,
            ))
        }

        async fn cancel_order(&self, id: &str, symbol: &str) -> Result<Order> {
            Ok(Order::new(
                id.to_string(),
                symbol.to_string(),
                OrderType::Limit,
                OrderSide::Buy,
                Decimal::ZERO,
                None,
                OrderStatus::Cancelled,
            ))
        }

        async fn cancel_all_orders(&self, _symbol: &str) -> Result<Vec<Order>> {
            Ok(vec![])
        }

        async fn fetch_order(&self, id: &str, symbol: &str) -> Result<Order> {
            Ok(Order::new(
                id.to_string(),
                symbol.to_string(),
                OrderType::Limit,
                OrderSide::Buy,
                Decimal::ZERO,
                None,
                OrderStatus::Open,
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

    #[test]
    fn test_trait_object_safety() {
        // Verify trait is object-safe by creating a trait object
        let _exchange: BoxedTrading = Box::new(MockExchange);
    }

    #[tokio::test]
    async fn test_create_order() {
        use rust_decimal_macros::dec;

        let exchange = MockExchange;

        let order = exchange
            .create_order(OrderParams::market_buy("BTC/USDT", dec!(0.01)))
            .await
            .unwrap();

        assert_eq!(order.symbol, "BTC/USDT");
        assert_eq!(order.side, OrderSide::Buy);
        assert_eq!(order.order_type, OrderType::Market);
    }

    #[tokio::test]
    async fn test_convenience_methods() {
        use rust_decimal_macros::dec;

        let exchange = MockExchange;

        // Test market_buy
        let order = exchange.market_buy("BTC/USDT", dec!(0.01)).await.unwrap();
        assert_eq!(order.side, OrderSide::Buy);
        assert_eq!(order.order_type, OrderType::Market);

        // Test market_sell
        let order = exchange.market_sell("BTC/USDT", dec!(0.01)).await.unwrap();
        assert_eq!(order.side, OrderSide::Sell);
        assert_eq!(order.order_type, OrderType::Market);

        // Test limit_buy
        let order = exchange
            .limit_buy("BTC/USDT", dec!(0.01), dec!(50000))
            .await
            .unwrap();
        assert_eq!(order.side, OrderSide::Buy);
        assert_eq!(order.order_type, OrderType::Limit);

        // Test limit_sell
        let order = exchange
            .limit_sell("BTC/USDT", dec!(0.01), dec!(50000))
            .await
            .unwrap();
        assert_eq!(order.side, OrderSide::Sell);
        assert_eq!(order.order_type, OrderType::Limit);
    }

    #[tokio::test]
    async fn test_cancel_order() {
        let exchange = MockExchange;

        let order = exchange.cancel_order("12345", "BTC/USDT").await.unwrap();
        assert_eq!(order.id, "12345");
        assert_eq!(order.status, OrderStatus::Cancelled);
    }

    #[tokio::test]
    async fn test_fetch_orders() {
        let exchange = MockExchange;

        // Test fetch_order
        let order = exchange.fetch_order("12345", "BTC/USDT").await.unwrap();
        assert_eq!(order.id, "12345");

        // Test fetch_open_orders
        let orders = exchange.fetch_open_orders(Some("BTC/USDT")).await.unwrap();
        assert!(orders.is_empty());

        // Test fetch_closed_orders
        let orders = exchange
            .fetch_closed_orders(Some("BTC/USDT"), None, Some(100))
            .await
            .unwrap();
        assert!(orders.is_empty());
    }
}
