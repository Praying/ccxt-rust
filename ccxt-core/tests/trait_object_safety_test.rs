//! Property-based tests for trait object safety.
//!
//! This test module verifies that all traits in the exchange trait hierarchy
//! are object-safe and can be used with dynamic dispatch via trait objects.
//!
//! **Feature: binance-rest-api-modularization, Property 3: Trait Object Safety**
//! **Validates: Requirements 4.1**

use async_trait::async_trait;
use ccxt_core::capability::ExchangeCapabilities;
use ccxt_core::error::Result;
use ccxt_core::traits::{
    Account, BoxedAccount, BoxedFullExchange, BoxedFunding, BoxedMarketData, BoxedTrading,
    FullExchange, Funding, MarketData, PublicExchange, Trading,
};
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

// ============================================================================
// Mock Exchange Implementation for Testing
// ============================================================================

/// A complete mock exchange implementation that supports all traits.
/// Used to verify that trait objects can be created and used.
#[derive(Debug, Clone)]
struct MockExchange {
    id: String,
    name: String,
}

impl MockExchange {
    fn new(id: &str, name: &str) -> Self {
        Self {
            id: id.to_string(),
            name: name.to_string(),
        }
    }
}

impl PublicExchange for MockExchange {
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
impl MarketData for MockExchange {
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
impl Trading for MockExchange {
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
impl Account for MockExchange {
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
impl ccxt_core::traits::Margin for MockExchange {
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
impl Funding for MockExchange {
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
fn arb_mock_exchange() -> impl Strategy<Value = MockExchange> {
    ("[a-z]{3,10}", "[A-Z][a-z]{3,15}( [A-Z][a-z]{3,10})?")
        .prop_map(|(id, name)| MockExchange::new(&id, &name))
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1000))]

    /// **Feature: binance-rest-api-modularization, Property 3: Trait Object Safety**
    ///
    /// *For any* exchange that implements all traits, it SHALL be possible to create
    /// trait objects for each individual trait (PublicExchange, MarketData, Trading,
    /// Account, Margin, Funding) without compilation errors.
    ///
    /// **Validates: Requirements 4.1**
    #[test]
    fn prop_public_exchange_trait_object_creation(exchange in arb_mock_exchange()) {
        // Create trait object for PublicExchange
        let exchange_obj: Box<dyn PublicExchange> = Box::new(exchange);

        // Verify the trait object can be used
        prop_assert!(!exchange_obj.id().is_empty());
    }

    /// **Feature: binance-rest-api-modularization, Property 3: Trait Object Safety**
    ///
    /// *For any* exchange that implements MarketData, it SHALL be possible to create
    /// a BoxedMarketData trait object.
    ///
    /// **Validates: Requirements 4.1**
    #[test]
    fn prop_market_data_trait_object_creation(exchange in arb_mock_exchange()) {
        // Create trait object for MarketData
        let exchange_obj: BoxedMarketData = Box::new(exchange);

        // Verify the trait object can be used
        prop_assert!(!exchange_obj.id().is_empty());
    }

    /// **Feature: binance-rest-api-modularization, Property 3: Trait Object Safety**
    ///
    /// *For any* exchange that implements Trading, it SHALL be possible to create
    /// a BoxedTrading trait object.
    ///
    /// **Validates: Requirements 4.1**
    #[test]
    fn prop_trading_trait_object_creation(exchange in arb_mock_exchange()) {
        // Create trait object for Trading
        let exchange_obj: BoxedTrading = Box::new(exchange);

        // Verify the trait object can be used
        prop_assert!(!exchange_obj.id().is_empty());
    }

    /// **Feature: binance-rest-api-modularization, Property 3: Trait Object Safety**
    ///
    /// *For any* exchange that implements Account, it SHALL be possible to create
    /// a BoxedAccount trait object.
    ///
    /// **Validates: Requirements 4.1**
    #[test]
    fn prop_account_trait_object_creation(exchange in arb_mock_exchange()) {
        // Create trait object for Account
        let exchange_obj: BoxedAccount = Box::new(exchange);

        // Verify the trait object can be used
        prop_assert!(!exchange_obj.id().is_empty());
    }

    /// **Feature: binance-rest-api-modularization, Property 3: Trait Object Safety**
    ///
    /// *For any* exchange that implements Margin, it SHALL be possible to create
    /// a BoxedMargin trait object.
    ///
    /// **Validates: Requirements 4.1**
    #[test]
    fn prop_margin_trait_object_creation(exchange in arb_mock_exchange()) {
        // Create trait object for Margin
        let exchange_obj: ccxt_core::traits::BoxedMargin = Box::new(exchange);

        // Verify the trait object can be used
        prop_assert!(!exchange_obj.id().is_empty());
    }

    /// **Feature: binance-rest-api-modularization, Property 3: Trait Object Safety**
    ///
    /// *For any* exchange that implements Funding, it SHALL be possible to create
    /// a BoxedFunding trait object.
    ///
    /// **Validates: Requirements 4.1**
    #[test]
    fn prop_funding_trait_object_creation(exchange in arb_mock_exchange()) {
        // Create trait object for Funding
        let exchange_obj: BoxedFunding = Box::new(exchange);

        // Verify the trait object can be used
        prop_assert!(!exchange_obj.id().is_empty());
    }

    /// **Feature: binance-rest-api-modularization, Property 3: Trait Object Safety**
    ///
    /// *For any* exchange that implements all traits, it SHALL be possible to create
    /// a BoxedFullExchange trait object combining all capabilities.
    ///
    /// **Validates: Requirements 4.1**
    #[test]
    fn prop_full_exchange_trait_object_creation(exchange in arb_mock_exchange()) {
        // Create trait object for FullExchange
        let exchange_obj: BoxedFullExchange = Box::new(exchange);

        // Verify the trait object can be used
        prop_assert!(exchange_obj.id().len() > 0);
    }

    /// **Feature: binance-rest-api-modularization, Property 3: Trait Object Safety**
    ///
    /// *For any* trait object, it SHALL be possible to call methods through the
    /// trait object interface without errors.
    ///
    /// **Validates: Requirements 4.1**
    #[test]
    fn prop_trait_object_method_calls(exchange in arb_mock_exchange()) {
        let exchange_obj: Box<dyn PublicExchange> = Box::new(exchange);

        // Verify methods can be called through trait object
        prop_assert!(!exchange_obj.id().is_empty());
        prop_assert!(!exchange_obj.name().is_empty());
        prop_assert!(!exchange_obj.timeframes().is_empty());
        prop_assert!(exchange_obj.rate_limit() > 0);
    }

    /// **Feature: binance-rest-api-modularization, Property 3: Trait Object Safety**
    ///
    /// *For any* trait object, it SHALL be possible to store it in a collection
    /// and retrieve it later.
    ///
    /// **Validates: Requirements 4.1**
    #[test]
    fn prop_trait_object_in_collection(exchange in arb_mock_exchange()) {
        let exchange_obj: Box<dyn PublicExchange> = Box::new(exchange);
        let mut exchanges: Vec<Box<dyn PublicExchange>> = vec![];

        exchanges.push(exchange_obj);

        prop_assert_eq!(exchanges.len(), 1);
        prop_assert!(!exchanges[0].id().is_empty());
    }

    /// **Feature: binance-rest-api-modularization, Property 3: Trait Object Safety**
    ///
    /// *For any* trait object wrapped in Arc, it SHALL be possible to clone and
    /// share across async tasks.
    ///
    /// **Validates: Requirements 4.1**
    #[test]
    fn prop_arc_trait_object_cloning(exchange in arb_mock_exchange()) {
        let exchange_obj: Arc<dyn PublicExchange> = Arc::new(exchange);
        let cloned = exchange_obj.clone();

        prop_assert_eq!(exchange_obj.id(), cloned.id());
        prop_assert_eq!(exchange_obj.name(), cloned.name());
    }

    /// **Feature: binance-rest-api-modularization, Property 3: Trait Object Safety**
    ///
    /// *For any* FullExchange trait object, it SHALL be possible to downcast to
    /// individual trait objects.
    ///
    /// **Validates: Requirements 4.1**
    #[test]
    fn prop_full_exchange_trait_composition(exchange in arb_mock_exchange()) {
        let full_exchange: Box<dyn FullExchange> = Box::new(exchange);

        // Verify that FullExchange includes all component traits
        // by calling methods from each trait
        prop_assert!(!full_exchange.id().is_empty());
        prop_assert!(!full_exchange.timeframes().is_empty());
    }
}

// ============================================================================
// Unit Tests for Trait Object Safety
// ============================================================================

#[test]
fn test_public_exchange_trait_object() {
    let exchange = MockExchange::new("test", "Test Exchange");
    let _: Box<dyn PublicExchange> = Box::new(exchange);
}

#[test]
fn test_market_data_trait_object() {
    let exchange = MockExchange::new("test", "Test Exchange");
    let _: BoxedMarketData = Box::new(exchange);
}

#[test]
fn test_trading_trait_object() {
    let exchange = MockExchange::new("test", "Test Exchange");
    let _: BoxedTrading = Box::new(exchange);
}

#[test]
fn test_account_trait_object() {
    let exchange = MockExchange::new("test", "Test Exchange");
    let _: BoxedAccount = Box::new(exchange);
}

#[test]
fn test_margin_trait_object() {
    let exchange = MockExchange::new("test", "Test Exchange");
    let _: ccxt_core::traits::BoxedMargin = Box::new(exchange);
}

#[test]
fn test_funding_trait_object() {
    let exchange = MockExchange::new("test", "Test Exchange");
    let _: BoxedFunding = Box::new(exchange);
}

#[test]
fn test_full_exchange_trait_object() {
    let exchange = MockExchange::new("test", "Test Exchange");
    let _: BoxedFullExchange = Box::new(exchange);
}

#[test]
fn test_trait_object_method_access() {
    let exchange = MockExchange::new("binance", "Binance");
    let obj: Box<dyn PublicExchange> = Box::new(exchange);

    assert_eq!(obj.id(), "binance");
    assert_eq!(obj.name(), "Binance");
    assert!(!obj.timeframes().is_empty());
}

#[test]
fn test_arc_trait_object() {
    let exchange = MockExchange::new("test", "Test Exchange");
    let arc_obj: Arc<dyn PublicExchange> = Arc::new(exchange);
    let cloned = arc_obj.clone();

    assert_eq!(arc_obj.id(), cloned.id());
}

#[test]
fn test_trait_object_in_vec() {
    let exchange1 = MockExchange::new("test1", "Test 1");
    let exchange2 = MockExchange::new("test2", "Test 2");

    let mut exchanges: Vec<Box<dyn PublicExchange>> = vec![];
    exchanges.push(Box::new(exchange1));
    exchanges.push(Box::new(exchange2));

    assert_eq!(exchanges.len(), 2);
    assert_eq!(exchanges[0].id(), "test1");
    assert_eq!(exchanges[1].id(), "test2");
}

#[tokio::test]
async fn test_full_exchange_async_methods() {
    let exchange = MockExchange::new("test", "Test Exchange");
    let full: Box<dyn FullExchange> = Box::new(exchange);

    // Test that async methods can be called through trait object
    let balance = full.fetch_balance().await;
    assert!(balance.is_ok());

    let ticker = full.fetch_ticker("BTC/USDT").await;
    assert!(ticker.is_ok());
}
