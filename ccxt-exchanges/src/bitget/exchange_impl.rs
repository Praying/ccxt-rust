//! Exchange trait implementation for Bitget
//!
//! This module implements the unified `Exchange` trait from `ccxt-core` for Bitget.

use async_trait::async_trait;
use ccxt_core::{
    Result,
    exchange::{Capability, Exchange, ExchangeCapabilities},
    types::{
        Amount, Balance, Market, Ohlcv, Order, OrderBook, OrderSide, OrderType, Price, Ticker,
        Timeframe, Trade,
    },
};
use rust_decimal::Decimal;
use std::collections::HashMap;
use std::sync::Arc;

use super::Bitget;

#[async_trait]
impl Exchange for Bitget {
    // ==================== Metadata ====================

    fn id(&self) -> &'static str {
        "bitget"
    }

    fn name(&self) -> &'static str {
        "Bitget"
    }

    fn version(&self) -> &'static str {
        "v2"
    }

    fn certified(&self) -> bool {
        false
    }

    fn has_websocket(&self) -> bool {
        true
    }

    fn capabilities(&self) -> ExchangeCapabilities {
        // Bitget supports:
        // - Market Data: markets, ticker, tickers, order_book, trades, ohlcv
        // - Trading: create_order, cancel_order, fetch_order, open_orders, closed_orders
        // - Account: balance, my_trades
        // - WebSocket: ticker, order_book, trades
        ExchangeCapabilities::builder()
            .market_data()
            .trading()
            .account()
            // Remove unsupported market data capabilities
            .without_capability(Capability::FetchCurrencies)
            .without_capability(Capability::FetchStatus)
            .without_capability(Capability::FetchTime)
            // Remove unsupported trading capabilities
            .without_capability(Capability::CancelAllOrders)
            .without_capability(Capability::EditOrder)
            .without_capability(Capability::FetchOrders)
            .without_capability(Capability::FetchCanceledOrders)
            // Remove unsupported account capabilities
            .without_capability(Capability::FetchDeposits)
            .without_capability(Capability::FetchWithdrawals)
            .without_capability(Capability::FetchTransactions)
            .without_capability(Capability::FetchLedger)
            // Add WebSocket capabilities
            .capability(Capability::Websocket)
            .capability(Capability::WatchTicker)
            .capability(Capability::WatchOrderBook)
            .capability(Capability::WatchTrades)
            .build()
    }

    fn timeframes(&self) -> Vec<Timeframe> {
        vec![
            Timeframe::M1,
            Timeframe::M5,
            Timeframe::M15,
            Timeframe::M30,
            Timeframe::H1,
            Timeframe::H4,
            Timeframe::H6,
            Timeframe::H12,
            Timeframe::D1,
            Timeframe::D3,
            Timeframe::W1,
            Timeframe::Mon1,
        ]
    }

    fn rate_limit(&self) -> u32 {
        20
    }

    // ==================== Market Data (Public API) ====================

    async fn fetch_markets(&self) -> Result<Vec<Market>> {
        let arc_markets = Bitget::fetch_markets(self).await?;
        Ok(arc_markets.values().map(|v| (**v).clone()).collect())
    }

    async fn load_markets(&self, reload: bool) -> Result<Arc<HashMap<String, Arc<Market>>>> {
        Bitget::load_markets(self, reload).await
    }

    async fn fetch_ticker(&self, symbol: &str) -> Result<Ticker> {
        Bitget::fetch_ticker(self, symbol).await
    }

    async fn fetch_tickers(&self, symbols: Option<&[String]>) -> Result<Vec<Ticker>> {
        let symbols_vec = symbols.map(<[String]>::to_vec);
        Bitget::fetch_tickers(self, symbols_vec).await
    }

    async fn fetch_order_book(&self, symbol: &str, limit: Option<u32>) -> Result<OrderBook> {
        Bitget::fetch_order_book(self, symbol, limit).await
    }

    async fn fetch_trades(&self, symbol: &str, limit: Option<u32>) -> Result<Vec<Trade>> {
        Bitget::fetch_trades(self, symbol, limit).await
    }

    async fn fetch_ohlcv(
        &self,
        symbol: &str,
        timeframe: Timeframe,
        since: Option<i64>,
        limit: Option<u32>,
    ) -> Result<Vec<Ohlcv>> {
        let timeframe_str = timeframe.to_string();
        #[allow(deprecated)]
        let ohlcv_data = Bitget::fetch_ohlcv(self, symbol, &timeframe_str, since, limit).await?;

        // Convert OHLCV to Ohlcv with proper type conversions
        ohlcv_data
            .into_iter()
            .map(|o| -> ccxt_core::Result<Ohlcv> {
                Ok(Ohlcv {
                    timestamp: o.timestamp,
                    open: Price(Decimal::try_from(o.open).map_err(|e| {
                        ccxt_core::Error::from(ccxt_core::ParseError::invalid_value(
                            "OHLCV open",
                            format!("{e}"),
                        ))
                    })?),
                    high: Price(Decimal::try_from(o.high).map_err(|e| {
                        ccxt_core::Error::from(ccxt_core::ParseError::invalid_value(
                            "OHLCV high",
                            format!("{e}"),
                        ))
                    })?),
                    low: Price(Decimal::try_from(o.low).map_err(|e| {
                        ccxt_core::Error::from(ccxt_core::ParseError::invalid_value(
                            "OHLCV low",
                            format!("{e}"),
                        ))
                    })?),
                    close: Price(Decimal::try_from(o.close).map_err(|e| {
                        ccxt_core::Error::from(ccxt_core::ParseError::invalid_value(
                            "OHLCV close",
                            format!("{e}"),
                        ))
                    })?),
                    volume: Amount(Decimal::try_from(o.volume).map_err(|e| {
                        ccxt_core::Error::from(ccxt_core::ParseError::invalid_value(
                            "OHLCV volume",
                            format!("{e}"),
                        ))
                    })?),
                })
            })
            .collect::<ccxt_core::Result<Vec<Ohlcv>>>()
            .map_err(|e| e.context("Failed to convert Bitget OHLCV data"))
    }

    // ==================== Trading (Private API) ====================

    async fn create_order(
        &self,
        symbol: &str,
        order_type: OrderType,
        side: OrderSide,
        amount: Amount,
        price: Option<Price>,
    ) -> Result<Order> {
        // Direct delegation - no type conversion needed
        #[allow(deprecated)]
        Bitget::create_order(self, symbol, order_type, side, amount, price).await
    }

    async fn cancel_order(&self, id: &str, symbol: Option<&str>) -> Result<Order> {
        let symbol_str = symbol.ok_or_else(|| {
            ccxt_core::Error::invalid_request("Symbol is required for cancel_order on Bitget")
        })?;
        Bitget::cancel_order(self, id, symbol_str).await
    }

    async fn cancel_all_orders(&self, _symbol: Option<&str>) -> Result<Vec<Order>> {
        Err(ccxt_core::Error::not_implemented("cancel_all_orders"))
    }

    async fn fetch_order(&self, id: &str, symbol: Option<&str>) -> Result<Order> {
        let symbol_str = symbol.ok_or_else(|| {
            ccxt_core::Error::invalid_request("Symbol is required for fetch_order on Bitget")
        })?;
        Bitget::fetch_order(self, id, symbol_str).await
    }

    async fn fetch_open_orders(
        &self,
        symbol: Option<&str>,
        since: Option<i64>,
        limit: Option<u32>,
    ) -> Result<Vec<Order>> {
        Bitget::fetch_open_orders(self, symbol, since, limit).await
    }

    async fn fetch_closed_orders(
        &self,
        symbol: Option<&str>,
        since: Option<i64>,
        limit: Option<u32>,
    ) -> Result<Vec<Order>> {
        Bitget::fetch_closed_orders(self, symbol, since, limit).await
    }

    // ==================== Account (Private API) ====================

    async fn fetch_balance(&self) -> Result<Balance> {
        Bitget::fetch_balance(self).await
    }

    async fn fetch_my_trades(
        &self,
        symbol: Option<&str>,
        since: Option<i64>,
        limit: Option<u32>,
    ) -> Result<Vec<Trade>> {
        let symbol_str = symbol.ok_or_else(|| {
            ccxt_core::Error::invalid_request("Symbol is required for fetch_my_trades on Bitget")
        })?;
        Bitget::fetch_my_trades(self, symbol_str, since, limit).await
    }

    // ==================== Helper Methods ====================

    async fn market(&self, symbol: &str) -> Result<Arc<Market>> {
        let cache = self.base().market_cache.read().await;

        if !cache.is_loaded() {
            return Err(ccxt_core::Error::exchange(
                "-1",
                "Markets not loaded. Call load_markets() first.",
            ));
        }

        cache
            .get_market(symbol)
            .ok_or_else(|| ccxt_core::Error::bad_symbol(format!("Market {} not found", symbol)))
    }

    async fn markets(&self) -> Arc<HashMap<String, Arc<Market>>> {
        let cache = self.base().market_cache.read().await;
        cache.markets()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ccxt_core::ExchangeConfig;

    #[test]
    fn test_bitget_exchange_trait_metadata() {
        let config = ExchangeConfig::default();
        let bitget = Bitget::new(config).unwrap();

        // Test metadata methods via Exchange trait
        let exchange: &dyn Exchange = &bitget;

        assert_eq!(exchange.id(), "bitget");
        assert_eq!(exchange.name(), "Bitget");
        assert_eq!(exchange.version(), "v2");
        assert!(!exchange.certified());
        assert!(exchange.has_websocket());
    }

    #[test]
    fn test_bitget_exchange_trait_capabilities() {
        let config = ExchangeConfig::default();
        let bitget = Bitget::new(config).unwrap();

        let exchange: &dyn Exchange = &bitget;
        let caps = exchange.capabilities();

        // Public API capabilities
        assert!(caps.fetch_markets());
        assert!(caps.fetch_ticker());
        assert!(caps.fetch_tickers());
        assert!(caps.fetch_order_book());
        assert!(caps.fetch_trades());
        assert!(caps.fetch_ohlcv());

        // Private API capabilities
        assert!(caps.create_order());
        assert!(caps.cancel_order());
        assert!(caps.fetch_order());
        assert!(caps.fetch_open_orders());
        assert!(caps.fetch_closed_orders());
        assert!(caps.fetch_balance());
        assert!(caps.fetch_my_trades());

        // WebSocket capabilities
        assert!(caps.websocket());
        assert!(caps.watch_ticker());
        assert!(caps.watch_order_book());
        assert!(caps.watch_trades());

        // Not implemented capabilities
        assert!(!caps.edit_order());
        assert!(!caps.cancel_all_orders());
        assert!(!caps.fetch_currencies());
    }

    #[test]
    fn test_bitget_exchange_trait_timeframes() {
        let config = ExchangeConfig::default();
        let bitget = Bitget::new(config).unwrap();

        let exchange: &dyn Exchange = &bitget;
        let timeframes = exchange.timeframes();

        assert!(!timeframes.is_empty());
        assert!(timeframes.contains(&Timeframe::M1));
        assert!(timeframes.contains(&Timeframe::M5));
        assert!(timeframes.contains(&Timeframe::M15));
        assert!(timeframes.contains(&Timeframe::H1));
        assert!(timeframes.contains(&Timeframe::H4));
        assert!(timeframes.contains(&Timeframe::D1));
        assert!(timeframes.contains(&Timeframe::W1));
        assert!(timeframes.contains(&Timeframe::Mon1));
    }

    #[test]
    fn test_bitget_exchange_trait_rate_limit() {
        let config = ExchangeConfig::default();
        let bitget = Bitget::new(config).unwrap();

        let exchange: &dyn Exchange = &bitget;
        assert_eq!(exchange.rate_limit(), 20);
    }

    #[test]
    fn test_bitget_exchange_trait_object_safety() {
        let config = ExchangeConfig::default();
        let bitget = Bitget::new(config).unwrap();

        // Test that we can create a trait object (Box<dyn Exchange>)
        let exchange: Box<dyn Exchange> = Box::new(bitget);

        assert_eq!(exchange.id(), "bitget");
        assert_eq!(exchange.name(), "Bitget");
        assert_eq!(exchange.rate_limit(), 20);
    }

    #[test]
    fn test_bitget_exchange_trait_polymorphic_usage() {
        let config = ExchangeConfig::default();
        let bitget = Bitget::new(config).unwrap();

        // Test polymorphic usage with &dyn Exchange
        fn check_exchange_metadata(exchange: &dyn Exchange) -> (&str, &str, bool) {
            (exchange.id(), exchange.name(), exchange.has_websocket())
        }

        let (id, name, has_ws) = check_exchange_metadata(&bitget);
        assert_eq!(id, "bitget");
        assert_eq!(name, "Bitget");
        assert!(has_ws);
    }

    #[test]
    fn test_bitget_capabilities_has_method() {
        let config = ExchangeConfig::default();
        let bitget = Bitget::new(config).unwrap();

        let exchange: &dyn Exchange = &bitget;
        let caps = exchange.capabilities();

        // Test the has() method with CCXT-style camelCase names
        assert!(caps.has("fetchMarkets"));
        assert!(caps.has("fetchTicker"));
        assert!(caps.has("fetchTickers"));
        assert!(caps.has("fetchOrderBook"));
        assert!(caps.has("fetchTrades"));
        assert!(caps.has("fetchOHLCV"));
        assert!(caps.has("createOrder"));
        assert!(caps.has("cancelOrder"));
        assert!(caps.has("fetchOrder"));
        assert!(caps.has("fetchOpenOrders"));
        assert!(caps.has("fetchClosedOrders"));
        assert!(caps.has("fetchBalance"));
        assert!(caps.has("fetchMyTrades"));
        assert!(caps.has("websocket"));

        // Not implemented
        assert!(!caps.has("editOrder"));
        assert!(!caps.has("cancelAllOrders"));
        assert!(!caps.has("fetchCurrencies"));
        assert!(!caps.has("unknownCapability"));
    }
}
