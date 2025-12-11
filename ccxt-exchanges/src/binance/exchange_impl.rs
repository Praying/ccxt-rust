//! Exchange trait implementation for Binance
//!
//! This module implements the unified `Exchange` trait from `ccxt-core` for Binance.

use async_trait::async_trait;
use ccxt_core::{
    Result,
    exchange::{Exchange, ExchangeCapabilities},
    types::{
        Balance, Market, Ohlcv, Order, OrderBook, OrderSide, OrderType, Ticker, Timeframe, Trade,
    },
};
use rust_decimal::Decimal;
use std::collections::HashMap;

use super::Binance;

#[async_trait]
impl Exchange for Binance {
    // ==================== Metadata ====================

    fn id(&self) -> &str {
        "binance"
    }

    fn name(&self) -> &str {
        "Binance"
    }

    fn version(&self) -> &'static str {
        "v3"
    }

    fn certified(&self) -> bool {
        true
    }

    fn has_websocket(&self) -> bool {
        true
    }

    fn capabilities(&self) -> ExchangeCapabilities {
        ExchangeCapabilities {
            // Market Data (Public API)
            fetch_markets: true,
            fetch_currencies: true,
            fetch_ticker: true,
            fetch_tickers: true,
            fetch_order_book: true,
            fetch_trades: true,
            fetch_ohlcv: true,
            fetch_status: true,
            fetch_time: true,

            // Trading (Private API)
            create_order: true,
            create_market_order: true,
            create_limit_order: true,
            cancel_order: true,
            cancel_all_orders: true,
            edit_order: false, // Binance doesn't support order editing
            fetch_order: true,
            fetch_orders: true,
            fetch_open_orders: true,
            fetch_closed_orders: true,
            fetch_canceled_orders: false,

            // Account (Private API)
            fetch_balance: true,
            fetch_my_trades: true,
            fetch_deposits: true,
            fetch_withdrawals: true,
            fetch_transactions: true,
            fetch_ledger: true,

            // Funding
            fetch_deposit_address: true,
            create_deposit_address: true,
            withdraw: true,
            transfer: true,

            // Margin Trading
            fetch_borrow_rate: true,
            fetch_borrow_rates: true,
            fetch_funding_rate: true,
            fetch_funding_rates: true,
            fetch_positions: true,
            set_leverage: true,
            set_margin_mode: true,

            // WebSocket
            websocket: true,
            watch_ticker: true,
            watch_tickers: true,
            watch_order_book: true,
            watch_trades: true,
            watch_ohlcv: true,
            watch_balance: true,
            watch_orders: true,
            watch_my_trades: true,
        }
    }

    fn timeframes(&self) -> Vec<Timeframe> {
        vec![
            Timeframe::M1,
            Timeframe::M3,
            Timeframe::M5,
            Timeframe::M15,
            Timeframe::M30,
            Timeframe::H1,
            Timeframe::H2,
            Timeframe::H4,
            Timeframe::H6,
            Timeframe::H8,
            Timeframe::H12,
            Timeframe::D1,
            Timeframe::D3,
            Timeframe::W1,
            Timeframe::Mon1,
        ]
    }

    fn rate_limit(&self) -> f64 {
        50.0
    }

    // ==================== Market Data (Public API) ====================

    async fn fetch_markets(&self) -> Result<Vec<Market>> {
        // Delegate to existing implementation
        Binance::fetch_markets(self).await
    }

    async fn load_markets(&self, reload: bool) -> Result<HashMap<String, Market>> {
        // Delegate to existing implementation
        Binance::load_markets(self, reload).await
    }

    async fn fetch_ticker(&self, symbol: &str) -> Result<Ticker> {
        // Delegate to existing implementation using default parameters
        Binance::fetch_ticker(self, symbol, ()).await
    }

    async fn fetch_tickers(&self, symbols: Option<&[String]>) -> Result<Vec<Ticker>> {
        // Convert slice to Vec for existing implementation
        let symbols_vec = symbols.map(|s| s.to_vec());
        Binance::fetch_tickers(self, symbols_vec).await
    }

    async fn fetch_order_book(&self, symbol: &str, limit: Option<u32>) -> Result<OrderBook> {
        // Delegate to existing implementation
        Binance::fetch_order_book(self, symbol, limit).await
    }

    async fn fetch_trades(&self, symbol: &str, limit: Option<u32>) -> Result<Vec<Trade>> {
        // Delegate to existing implementation
        Binance::fetch_trades(self, symbol, limit).await
    }

    async fn fetch_ohlcv(
        &self,
        symbol: &str,
        timeframe: Timeframe,
        since: Option<i64>,
        limit: Option<u32>,
    ) -> Result<Vec<Ohlcv>> {
        use ccxt_core::types::{Amount, Price};

        // Convert Timeframe enum to string for existing implementation
        let timeframe_str = timeframe.to_string();
        let ohlcv_data =
            Binance::fetch_ohlcv(self, symbol, &timeframe_str, since, limit, None).await?;

        // Convert OHLCV to Ohlcv with proper type conversions
        Ok(ohlcv_data
            .into_iter()
            .map(|o| Ohlcv {
                timestamp: o.timestamp,
                open: Price::from(Decimal::try_from(o.open).unwrap_or_default()),
                high: Price::from(Decimal::try_from(o.high).unwrap_or_default()),
                low: Price::from(Decimal::try_from(o.low).unwrap_or_default()),
                close: Price::from(Decimal::try_from(o.close).unwrap_or_default()),
                volume: Amount::from(Decimal::try_from(o.volume).unwrap_or_default()),
            })
            .collect())
    }

    // ==================== Trading (Private API) ====================

    async fn create_order(
        &self,
        symbol: &str,
        order_type: OrderType,
        side: OrderSide,
        amount: Decimal,
        price: Option<Decimal>,
    ) -> Result<Order> {
        // Convert Decimal to f64 for existing implementation
        let amount_f64 = amount.to_string().parse::<f64>().unwrap_or(0.0);
        let price_f64 = price.map(|p| p.to_string().parse::<f64>().unwrap_or(0.0));

        Binance::create_order(self, symbol, order_type, side, amount_f64, price_f64, None).await
    }

    async fn cancel_order(&self, id: &str, symbol: Option<&str>) -> Result<Order> {
        // Delegate to existing implementation
        // Note: Binance requires symbol for cancel_order
        let symbol_str = symbol.ok_or_else(|| {
            ccxt_core::Error::invalid_request("Symbol is required for cancel_order on Binance")
        })?;
        Binance::cancel_order(self, id, symbol_str).await
    }

    async fn cancel_all_orders(&self, symbol: Option<&str>) -> Result<Vec<Order>> {
        // Delegate to existing implementation
        // Note: Binance requires symbol for cancel_all_orders
        let symbol_str = symbol.ok_or_else(|| {
            ccxt_core::Error::invalid_request("Symbol is required for cancel_all_orders on Binance")
        })?;
        Binance::cancel_all_orders(self, symbol_str).await
    }

    async fn fetch_order(&self, id: &str, symbol: Option<&str>) -> Result<Order> {
        // Delegate to existing implementation
        // Note: Binance requires symbol for fetch_order
        let symbol_str = symbol.ok_or_else(|| {
            ccxt_core::Error::invalid_request("Symbol is required for fetch_order on Binance")
        })?;
        Binance::fetch_order(self, id, symbol_str).await
    }

    async fn fetch_open_orders(
        &self,
        symbol: Option<&str>,
        _since: Option<i64>,
        _limit: Option<u32>,
    ) -> Result<Vec<Order>> {
        // Delegate to existing implementation
        // Note: Binance's fetch_open_orders doesn't support since/limit parameters
        Binance::fetch_open_orders(self, symbol).await
    }

    async fn fetch_closed_orders(
        &self,
        symbol: Option<&str>,
        since: Option<i64>,
        limit: Option<u32>,
    ) -> Result<Vec<Order>> {
        // Delegate to existing implementation
        // Convert i64 to u64 for since parameter
        let since_u64 = since.map(|s| s as u64);
        Binance::fetch_closed_orders(self, symbol, since_u64, limit).await
    }

    // ==================== Account (Private API) ====================

    async fn fetch_balance(&self) -> Result<Balance> {
        // Delegate to existing implementation
        Binance::fetch_balance(self, None).await
    }

    async fn fetch_my_trades(
        &self,
        symbol: Option<&str>,
        since: Option<i64>,
        limit: Option<u32>,
    ) -> Result<Vec<Trade>> {
        // Delegate to existing implementation
        // Note: Binance's fetch_my_trades requires a symbol
        let symbol_str = symbol.ok_or_else(|| {
            ccxt_core::Error::invalid_request("Symbol is required for fetch_my_trades on Binance")
        })?;
        let since_u64 = since.map(|s| s as u64);
        Binance::fetch_my_trades(self, symbol_str, since_u64, limit).await
    }

    // ==================== Helper Methods ====================

    async fn market(&self, symbol: &str) -> Result<Market> {
        // Use async read for async method
        let cache = self.base().market_cache.read().await;

        if !cache.loaded {
            return Err(ccxt_core::Error::exchange(
                "-1",
                "Markets not loaded. Call load_markets() first.",
            ));
        }

        cache
            .markets
            .get(symbol)
            .cloned()
            .ok_or_else(|| ccxt_core::Error::bad_symbol(format!("Market {} not found", symbol)))
    }

    async fn markets(&self) -> HashMap<String, Market> {
        // Use async read for async method
        let cache = self.base().market_cache.read().await;
        cache.markets.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ccxt_core::ExchangeConfig;

    #[test]
    fn test_binance_exchange_trait_metadata() {
        let config = ExchangeConfig::default();
        let binance = Binance::new(config).unwrap();

        // Test metadata methods via Exchange trait
        let exchange: &dyn Exchange = &binance;

        assert_eq!(exchange.id(), "binance");
        assert_eq!(exchange.name(), "Binance");
        assert_eq!(exchange.version(), "v3");
        assert!(exchange.certified());
        assert!(exchange.has_websocket());
    }

    #[test]
    fn test_binance_exchange_trait_capabilities() {
        let config = ExchangeConfig::default();
        let binance = Binance::new(config).unwrap();

        let exchange: &dyn Exchange = &binance;
        let caps = exchange.capabilities();

        assert!(caps.fetch_markets);
        assert!(caps.fetch_ticker);
        assert!(caps.create_order);
        assert!(caps.websocket);
        assert!(!caps.edit_order); // Binance doesn't support order editing
    }

    #[test]
    fn test_binance_exchange_trait_timeframes() {
        let config = ExchangeConfig::default();
        let binance = Binance::new(config).unwrap();

        let exchange: &dyn Exchange = &binance;
        let timeframes = exchange.timeframes();

        assert!(!timeframes.is_empty());
        assert!(timeframes.contains(&Timeframe::M1));
        assert!(timeframes.contains(&Timeframe::H1));
        assert!(timeframes.contains(&Timeframe::D1));
    }

    #[test]
    fn test_binance_exchange_trait_object_safety() {
        let config = ExchangeConfig::default();
        let binance = Binance::new(config).unwrap();

        // Test that we can create a trait object
        let exchange: Box<dyn Exchange> = Box::new(binance);

        assert_eq!(exchange.id(), "binance");
        assert_eq!(exchange.rate_limit(), 50.0);
    }

    // ==================== Property-Based Tests ====================

    mod property_tests {
        use super::*;
        use proptest::prelude::*;

        // Strategy to generate various ExchangeConfig configurations
        fn arb_exchange_config() -> impl Strategy<Value = ExchangeConfig> {
            (
                prop::bool::ANY,                                                      // sandbox
                prop::option::of(any::<u64>().prop_map(|n| format!("key_{}", n))),    // api_key
                prop::option::of(any::<u64>().prop_map(|n| format!("secret_{}", n))), // secret
            )
                .prop_map(|(sandbox, api_key, secret)| ExchangeConfig {
                    sandbox,
                    api_key,
                    secret,
                    ..Default::default()
                })
        }

        proptest! {
            #![proptest_config(ProptestConfig::with_cases(100))]

            /// **Feature: unified-exchange-trait, Property 8: Timeframes Non-Empty**
            ///
            /// *For any* exchange configuration, calling `timeframes()` should return
            /// a non-empty vector of valid `Timeframe` values.
            ///
            /// **Validates: Requirements 8.4**
            #[test]
            fn prop_timeframes_non_empty(config in arb_exchange_config()) {
                let binance = Binance::new(config).expect("Should create Binance instance");
                let exchange: &dyn Exchange = &binance;

                let timeframes = exchange.timeframes();

                // Property: timeframes should never be empty
                prop_assert!(!timeframes.is_empty(), "Timeframes should not be empty");

                // Property: all timeframes should be valid (no duplicates)
                let mut seen = std::collections::HashSet::new();
                for tf in &timeframes {
                    prop_assert!(
                        seen.insert(tf.clone()),
                        "Timeframes should not contain duplicates: {:?}",
                        tf
                    );
                }

                // Property: should contain common timeframes
                prop_assert!(
                    timeframes.contains(&Timeframe::M1),
                    "Should contain 1-minute timeframe"
                );
                prop_assert!(
                    timeframes.contains(&Timeframe::H1),
                    "Should contain 1-hour timeframe"
                );
                prop_assert!(
                    timeframes.contains(&Timeframe::D1),
                    "Should contain 1-day timeframe"
                );
            }

            /// **Feature: unified-exchange-trait, Property 7: Backward Compatibility**
            ///
            /// *For any* exchange configuration, metadata methods called through the Exchange trait
            /// should return the same values as calling them directly on Binance.
            ///
            /// **Validates: Requirements 3.2, 3.4**
            #[test]
            fn prop_backward_compatibility_metadata(config in arb_exchange_config()) {
                let binance = Binance::new(config).expect("Should create Binance instance");

                // Get trait object reference
                let exchange: &dyn Exchange = &binance;

                // Property: id() should be consistent
                prop_assert_eq!(
                    exchange.id(),
                    Binance::id(&binance),
                    "id() should be consistent between trait and direct call"
                );

                // Property: name() should be consistent
                prop_assert_eq!(
                    exchange.name(),
                    Binance::name(&binance),
                    "name() should be consistent between trait and direct call"
                );

                // Property: version() should be consistent
                prop_assert_eq!(
                    exchange.version(),
                    Binance::version(&binance),
                    "version() should be consistent between trait and direct call"
                );

                // Property: certified() should be consistent
                prop_assert_eq!(
                    exchange.certified(),
                    Binance::certified(&binance),
                    "certified() should be consistent between trait and direct call"
                );

                // Property: rate_limit() should be consistent
                prop_assert!(
                    (exchange.rate_limit() - Binance::rate_limit(&binance)).abs() < f64::EPSILON,
                    "rate_limit() should be consistent between trait and direct call"
                );

                // Property: capabilities should be consistent
                let trait_caps = exchange.capabilities();
                prop_assert!(trait_caps.fetch_markets, "Should support fetch_markets");
                prop_assert!(trait_caps.fetch_ticker, "Should support fetch_ticker");
                prop_assert!(trait_caps.websocket, "Should support websocket");
            }
        }
    }
}
