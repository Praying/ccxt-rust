//! Property-based tests for thread safety of exchange traits.
//!
//! This test module verifies that all traits in the exchange trait hierarchy
//! satisfy Send + Sync bounds, allowing safe usage across thread boundaries
//! in async contexts.
//!
//! **Feature: binance-rest-api-modularization, Property 4: Thread Safety**
//! **Validates: Requirements 4.3**

use async_trait::async_trait;
use ccxt_core::capability::ExchangeCapabilities;
use ccxt_core::error::Result;
use ccxt_core::traits::{Account, FullExchange, Funding, MarketData, PublicExchange, Trading};
use ccxt_core::types::{
    Balance, BalanceEntry, DepositAddress, FundingRate, FundingRateHistory, Market, Ohlcv, Order,
    OrderBook, Position, Ticker, Timeframe, Trade, Transaction, Transfer,
    params::{
        BalanceParams, LeverageParams, MarginMode, OhlcvParams, OrderBookParams, OrderParams,
        TransferParams, WithdrawParams,
    },
    transaction::{TransactionStatus, TransactionType},
};
use proptest::prelude::*;
use rust_decimal_macros::dec;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::task;

// ============================================================================
// Mock Exchange Implementation for Testing
// ============================================================================

/// A complete mock exchange implementation that supports all traits.
/// Used to verify thread safety across async boundaries.
#[derive(Debug, Clone)]
struct ThreadSafeExchange {
    id: String,
    name: String,
}

impl ThreadSafeExchange {
    fn new(id: &str, name: &str) -> Self {
        Self {
            id: id.to_string(),
            name: name.to_string(),
        }
    }
}

impl PublicExchange for ThreadSafeExchange {
    fn id(&self) -> &str {
        &self.id
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn capabilities(&self) -> ExchangeCapabilities {
        ExchangeCapabilities::all()
    }

    fn timeframes(&self) -> Vec<Timeframe> {
        vec![Timeframe::M1, Timeframe::H1, Timeframe::D1]
    }
}

#[async_trait]
impl MarketData for ThreadSafeExchange {
    async fn fetch_markets(&self) -> Result<Vec<Market>> {
        Ok(vec![])
    }

    async fn load_markets_with_reload(
        &self,
        _reload: bool,
    ) -> Result<Arc<HashMap<String, Arc<Market>>>> {
        Ok(Arc::new(HashMap::new()))
    }

    async fn fetch_ticker(&self, symbol: &str) -> Result<Ticker> {
        Ok(Ticker::new(symbol.to_string(), 0))
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

    async fn market(&self, symbol: &str) -> Result<Arc<Market>> {
        Ok(Arc::new(Market {
            symbol: symbol.to_string(),
            ..Default::default()
        }))
    }

    async fn markets(&self) -> Arc<HashMap<String, Arc<Market>>> {
        Arc::new(HashMap::new())
    }
}

#[async_trait]
impl Trading for ThreadSafeExchange {
    async fn create_order(&self, params: OrderParams) -> Result<Order> {
        Ok(Order::new(
            "order_123".to_string(),
            params.symbol,
            params.order_type,
            params.side,
            params.amount,
            params.price,
            ccxt_core::types::OrderStatus::Open,
        ))
    }

    async fn cancel_order(&self, id: &str, symbol: &str) -> Result<Order> {
        Ok(Order::new(
            id.to_string(),
            symbol.to_string(),
            ccxt_core::types::OrderType::Limit,
            ccxt_core::types::OrderSide::Buy,
            dec!(0),
            None,
            ccxt_core::types::OrderStatus::Cancelled,
        ))
    }

    async fn cancel_all_orders(&self, _symbol: &str) -> Result<Vec<Order>> {
        Ok(vec![])
    }

    async fn fetch_order(&self, id: &str, symbol: &str) -> Result<Order> {
        Ok(Order::new(
            id.to_string(),
            symbol.to_string(),
            ccxt_core::types::OrderType::Limit,
            ccxt_core::types::OrderSide::Buy,
            dec!(0),
            None,
            ccxt_core::types::OrderStatus::Open,
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
impl Account for ThreadSafeExchange {
    async fn fetch_balance_with_params(&self, _params: BalanceParams) -> Result<Balance> {
        let mut balance = Balance::new();
        balance.set(
            "USDT".to_string(),
            BalanceEntry {
                free: dec!(10000),
                used: dec!(0),
                total: dec!(10000),
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

#[async_trait]
impl ccxt_core::traits::Margin for ThreadSafeExchange {
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
impl Funding for ThreadSafeExchange {
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

// ============================================================================
// Property Tests
// ============================================================================

/// Strategy to generate mock exchange instances
fn arb_thread_safe_exchange() -> impl Strategy<Value = ThreadSafeExchange> {
    ("[a-z]{3,10}", "[A-Z][a-z]{3,15}( [A-Z][a-z]{3,10})?")
        .prop_map(|(id, name)| ThreadSafeExchange::new(&id, &name))
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(50))]

    /// **Feature: binance-rest-api-modularization, Property 4: Thread Safety**
    ///
    /// *For any* exchange that implements PublicExchange, it SHALL satisfy
    /// Send + Sync bounds, allowing safe usage across thread boundaries.
    ///
    /// **Validates: Requirements 4.3**
    #[test]
    fn prop_public_exchange_is_send_sync(exchange in arb_thread_safe_exchange()) {
        // This test verifies that PublicExchange satisfies Send + Sync
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<ThreadSafeExchange>();

        // Verify the exchange can be wrapped in Arc
        let _arc: Arc<dyn PublicExchange> = Arc::new(exchange);
    }

    /// **Feature: binance-rest-api-modularization, Property 4: Thread Safety**
    ///
    /// *For any* exchange that implements MarketData, it SHALL satisfy
    /// Send + Sync bounds.
    ///
    /// **Validates: Requirements 4.3**
    #[test]
    fn prop_market_data_is_send_sync(exchange in arb_thread_safe_exchange()) {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<ThreadSafeExchange>();

        let _arc: Arc<dyn MarketData> = Arc::new(exchange);
    }

    /// **Feature: binance-rest-api-modularization, Property 4: Thread Safety**
    ///
    /// *For any* exchange that implements Trading, it SHALL satisfy
    /// Send + Sync bounds.
    ///
    /// **Validates: Requirements 4.3**
    #[test]
    fn prop_trading_is_send_sync(exchange in arb_thread_safe_exchange()) {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<ThreadSafeExchange>();

        let _arc: Arc<dyn Trading> = Arc::new(exchange);
    }

    /// **Feature: binance-rest-api-modularization, Property 4: Thread Safety**
    ///
    /// *For any* exchange that implements Account, it SHALL satisfy
    /// Send + Sync bounds.
    ///
    /// **Validates: Requirements 4.3**
    #[test]
    fn prop_account_is_send_sync(exchange in arb_thread_safe_exchange()) {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<ThreadSafeExchange>();

        let _arc: Arc<dyn Account> = Arc::new(exchange);
    }

    /// **Feature: binance-rest-api-modularization, Property 4: Thread Safety**
    ///
    /// *For any* exchange that implements Margin, it SHALL satisfy
    /// Send + Sync bounds.
    ///
    /// **Validates: Requirements 4.3**
    #[test]
    fn prop_margin_is_send_sync(exchange in arb_thread_safe_exchange()) {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<ThreadSafeExchange>();

        let _arc: Arc<dyn ccxt_core::traits::Margin> = Arc::new(exchange);
    }

    /// **Feature: binance-rest-api-modularization, Property 4: Thread Safety**
    ///
    /// *For any* exchange that implements Funding, it SHALL satisfy
    /// Send + Sync bounds.
    ///
    /// **Validates: Requirements 4.3**
    #[test]
    fn prop_funding_is_send_sync(exchange in arb_thread_safe_exchange()) {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<ThreadSafeExchange>();

        let _arc: Arc<dyn Funding> = Arc::new(exchange);
    }

    /// **Feature: binance-rest-api-modularization, Property 4: Thread Safety**
    ///
    /// *For any* exchange that implements FullExchange, it SHALL satisfy
    /// Send + Sync bounds.
    ///
    /// **Validates: Requirements 4.3**
    #[test]
    fn prop_full_exchange_is_send_sync(exchange in arb_thread_safe_exchange()) {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<ThreadSafeExchange>();

        let _arc: Arc<dyn FullExchange> = Arc::new(exchange);
    }
}

// ============================================================================
// Unit Tests for Thread Safety
// ============================================================================

#[test]
fn test_public_exchange_send_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<ThreadSafeExchange>();
}

#[test]
fn test_market_data_send_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<ThreadSafeExchange>();
}

#[test]
fn test_trading_send_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<ThreadSafeExchange>();
}

#[test]
fn test_account_send_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<ThreadSafeExchange>();
}

#[test]
fn test_margin_send_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<ThreadSafeExchange>();
}

#[test]
fn test_funding_send_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<ThreadSafeExchange>();
}

#[test]
fn test_full_exchange_send_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<ThreadSafeExchange>();
}

#[test]
fn test_arc_public_exchange() {
    let exchange = ThreadSafeExchange::new("test", "Test Exchange");
    let _arc: Arc<dyn PublicExchange> = Arc::new(exchange);
}

#[test]
fn test_arc_market_data() {
    let exchange = ThreadSafeExchange::new("test", "Test Exchange");
    let _arc: Arc<dyn MarketData> = Arc::new(exchange);
}

#[test]
fn test_arc_trading() {
    let exchange = ThreadSafeExchange::new("test", "Test Exchange");
    let _arc: Arc<dyn Trading> = Arc::new(exchange);
}

#[test]
fn test_arc_account() {
    let exchange = ThreadSafeExchange::new("test", "Test Exchange");
    let _arc: Arc<dyn Account> = Arc::new(exchange);
}

#[test]
fn test_arc_margin() {
    let exchange = ThreadSafeExchange::new("test", "Test Exchange");
    let _arc: Arc<dyn ccxt_core::traits::Margin> = Arc::new(exchange);
}

#[test]
fn test_arc_funding() {
    let exchange = ThreadSafeExchange::new("test", "Test Exchange");
    let _arc: Arc<dyn Funding> = Arc::new(exchange);
}

#[test]
fn test_arc_full_exchange() {
    let exchange = ThreadSafeExchange::new("test", "Test Exchange");
    let _arc: Arc<dyn FullExchange> = Arc::new(exchange);
}

#[test]
fn test_arc_clone() {
    let exchange = ThreadSafeExchange::new("test", "Test Exchange");
    let arc1: Arc<dyn PublicExchange> = Arc::new(exchange);
    let arc2 = arc1.clone();

    assert_eq!(arc1.id(), arc2.id());
    assert_eq!(arc1.name(), arc2.name());
}

#[tokio::test]
async fn test_arc_across_task_boundary() {
    let exchange = ThreadSafeExchange::new("test", "Test Exchange");
    let arc_exchange: Arc<dyn PublicExchange> = Arc::new(exchange);
    let cloned = arc_exchange.clone();

    let handle = task::spawn(async move { cloned.id().to_string() });

    let result = handle.await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "test");
}

#[tokio::test]
async fn test_concurrent_arc_access() {
    let exchange = ThreadSafeExchange::new("test", "Test Exchange");
    let arc_exchange: Arc<dyn FullExchange> = Arc::new(exchange);

    let mut handles = vec![];
    for i in 0..5 {
        let exchange_clone = arc_exchange.clone();
        let handle = task::spawn(async move {
            let id = exchange_clone.id();
            (i, id.to_string())
        });
        handles.push(handle);
    }

    for handle in handles {
        let result = handle.await;
        assert!(result.is_ok());
        let (_, id) = result.unwrap();
        assert_eq!(id, "test");
    }
}

#[tokio::test]
async fn test_concurrent_async_method_calls() {
    let exchange = ThreadSafeExchange::new("test", "Test Exchange");
    let arc_exchange: Arc<dyn FullExchange> = Arc::new(exchange);

    let mut handles = vec![];
    for _ in 0..3 {
        let exchange_clone = arc_exchange.clone();
        let handle = task::spawn(async move {
            let ticker = exchange_clone.fetch_ticker("BTC/USDT").await;
            ticker.is_ok()
        });
        handles.push(handle);
    }

    for handle in handles {
        let result = handle.await;
        assert!(result.is_ok());
        assert!(result.unwrap());
    }
}
