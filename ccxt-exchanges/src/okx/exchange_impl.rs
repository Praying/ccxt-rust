//! Exchange trait implementation for OKX
//!
//! This module implements the unified `Exchange` trait from `ccxt-core` for OKX.

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
use rust_decimal::prelude::ToPrimitive;
use std::collections::HashMap;

use super::Okx;

#[async_trait]
impl Exchange for Okx {
    // ==================== Metadata ====================

    fn id(&self) -> &str {
        "okx"
    }

    fn name(&self) -> &str {
        "OKX"
    }

    fn version(&self) -> &'static str {
        "v5"
    }

    fn certified(&self) -> bool {
        false
    }

    fn has_websocket(&self) -> bool {
        true
    }

    fn capabilities(&self) -> ExchangeCapabilities {
        // OKX capabilities: market data + trading (with some exclusions) + limited account + limited websocket
        ExchangeCapabilities::builder()
            // Market Data: all except fetch_currencies, fetch_status, fetch_time
            .market_data()
            .without_capability(Capability::FetchCurrencies)
            .without_capability(Capability::FetchStatus)
            .without_capability(Capability::FetchTime)
            // Trading: all except cancel_all_orders, edit_order, fetch_orders, fetch_canceled_orders
            .trading()
            .without_capability(Capability::CancelAllOrders)
            .without_capability(Capability::EditOrder)
            .without_capability(Capability::FetchOrders)
            .without_capability(Capability::FetchCanceledOrders)
            // Account: only fetch_balance and fetch_my_trades
            .capability(Capability::FetchBalance)
            .capability(Capability::FetchMyTrades)
            // WebSocket: websocket, watch_ticker, watch_order_book, watch_trades
            .capability(Capability::Websocket)
            .capability(Capability::WatchTicker)
            .capability(Capability::WatchOrderBook)
            .capability(Capability::WatchTrades)
            .build()
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
            Timeframe::H12,
            Timeframe::D1,
            Timeframe::W1,
            Timeframe::Mon1,
        ]
    }

    fn rate_limit(&self) -> u32 {
        20
    }

    // ==================== Market Data (Public API) ====================

    async fn fetch_markets(&self) -> Result<Vec<Market>> {
        Okx::fetch_markets(self).await
    }

    async fn load_markets(&self, reload: bool) -> Result<HashMap<String, Market>> {
        Okx::load_markets(self, reload).await
    }

    async fn fetch_ticker(&self, symbol: &str) -> Result<Ticker> {
        Okx::fetch_ticker(self, symbol).await
    }

    async fn fetch_tickers(&self, symbols: Option<&[String]>) -> Result<Vec<Ticker>> {
        let symbols_vec = symbols.map(|s| s.to_vec());
        Okx::fetch_tickers(self, symbols_vec).await
    }

    async fn fetch_order_book(&self, symbol: &str, limit: Option<u32>) -> Result<OrderBook> {
        Okx::fetch_order_book(self, symbol, limit).await
    }

    async fn fetch_trades(&self, symbol: &str, limit: Option<u32>) -> Result<Vec<Trade>> {
        Okx::fetch_trades(self, symbol, limit).await
    }

    async fn fetch_ohlcv(
        &self,
        symbol: &str,
        timeframe: Timeframe,
        since: Option<i64>,
        limit: Option<u32>,
    ) -> Result<Vec<Ohlcv>> {
        let timeframe_str = timeframe.to_string();
        let ohlcv_data = Okx::fetch_ohlcv(self, symbol, &timeframe_str, since, limit).await?;

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
        let amount_f64 = amount
            .to_f64()
            .ok_or_else(|| ccxt_core::Error::invalid_request("Failed to convert amount to f64"))?;
        let price_f64 = match price {
            Some(p) => Some(p.to_f64().ok_or_else(|| {
                ccxt_core::Error::invalid_request("Failed to convert price to f64")
            })?),
            None => None,
        };

        Okx::create_order(self, symbol, order_type, side, amount_f64, price_f64).await
    }

    async fn cancel_order(&self, id: &str, symbol: Option<&str>) -> Result<Order> {
        let symbol_str = symbol.ok_or_else(|| {
            ccxt_core::Error::invalid_request("Symbol is required for cancel_order on OKX")
        })?;
        Okx::cancel_order(self, id, symbol_str).await
    }

    async fn cancel_all_orders(&self, _symbol: Option<&str>) -> Result<Vec<Order>> {
        Err(ccxt_core::Error::not_implemented("cancel_all_orders"))
    }

    async fn fetch_order(&self, id: &str, symbol: Option<&str>) -> Result<Order> {
        let symbol_str = symbol.ok_or_else(|| {
            ccxt_core::Error::invalid_request("Symbol is required for fetch_order on OKX")
        })?;
        Okx::fetch_order(self, id, symbol_str).await
    }

    async fn fetch_open_orders(
        &self,
        symbol: Option<&str>,
        since: Option<i64>,
        limit: Option<u32>,
    ) -> Result<Vec<Order>> {
        Okx::fetch_open_orders(self, symbol, since, limit).await
    }

    async fn fetch_closed_orders(
        &self,
        symbol: Option<&str>,
        since: Option<i64>,
        limit: Option<u32>,
    ) -> Result<Vec<Order>> {
        Okx::fetch_closed_orders(self, symbol, since, limit).await
    }

    // ==================== Account (Private API) ====================

    async fn fetch_balance(&self) -> Result<Balance> {
        Okx::fetch_balance(self).await
    }

    async fn fetch_my_trades(
        &self,
        symbol: Option<&str>,
        since: Option<i64>,
        limit: Option<u32>,
    ) -> Result<Vec<Trade>> {
        let symbol_str = symbol.ok_or_else(|| {
            ccxt_core::Error::invalid_request("Symbol is required for fetch_my_trades on OKX")
        })?;
        Okx::fetch_my_trades(self, symbol_str, since, limit).await
    }

    // ==================== Helper Methods ====================

    async fn market(&self, symbol: &str) -> Result<Market> {
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
        let cache = self.base().market_cache.read().await;
        cache.markets.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ccxt_core::ExchangeConfig;

    #[test]
    fn test_okx_exchange_trait_metadata() {
        let config = ExchangeConfig::default();
        let okx = Okx::new(config).unwrap();

        // Test metadata methods via Exchange trait
        let exchange: &dyn Exchange = &okx;

        assert_eq!(exchange.id(), "okx");
        assert_eq!(exchange.name(), "OKX");
        assert_eq!(exchange.version(), "v5");
        assert!(!exchange.certified());
        assert!(exchange.has_websocket());
    }

    #[test]
    fn test_okx_exchange_trait_capabilities() {
        let config = ExchangeConfig::default();
        let okx = Okx::new(config).unwrap();

        let exchange: &dyn Exchange = &okx;
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
    fn test_okx_exchange_trait_timeframes() {
        let config = ExchangeConfig::default();
        let okx = Okx::new(config).unwrap();

        let exchange: &dyn Exchange = &okx;
        let timeframes = exchange.timeframes();

        assert!(!timeframes.is_empty());
        assert!(timeframes.contains(&Timeframe::M1));
        assert!(timeframes.contains(&Timeframe::M3));
        assert!(timeframes.contains(&Timeframe::M5));
        assert!(timeframes.contains(&Timeframe::M15));
        assert!(timeframes.contains(&Timeframe::H1));
        assert!(timeframes.contains(&Timeframe::H4));
        assert!(timeframes.contains(&Timeframe::D1));
        assert!(timeframes.contains(&Timeframe::W1));
        assert!(timeframes.contains(&Timeframe::Mon1));
    }

    #[test]
    fn test_okx_exchange_trait_rate_limit() {
        let config = ExchangeConfig::default();
        let okx = Okx::new(config).unwrap();

        let exchange: &dyn Exchange = &okx;
        assert_eq!(exchange.rate_limit(), 20);
    }

    #[test]
    fn test_okx_exchange_trait_object_safety() {
        let config = ExchangeConfig::default();
        let okx = Okx::new(config).unwrap();

        // Test that we can create a trait object (Box<dyn Exchange>)
        let exchange: Box<dyn Exchange> = Box::new(okx);

        assert_eq!(exchange.id(), "okx");
        assert_eq!(exchange.name(), "OKX");
        assert_eq!(exchange.rate_limit(), 20);
    }

    #[test]
    fn test_okx_exchange_trait_polymorphic_usage() {
        let config = ExchangeConfig::default();
        let okx = Okx::new(config).unwrap();

        // Test polymorphic usage with &dyn Exchange
        fn check_exchange_metadata(exchange: &dyn Exchange) -> (&str, &str, bool) {
            (exchange.id(), exchange.name(), exchange.has_websocket())
        }

        let (id, name, has_ws) = check_exchange_metadata(&okx);
        assert_eq!(id, "okx");
        assert_eq!(name, "OKX");
        assert!(has_ws);
    }

    #[test]
    fn test_okx_capabilities_has_method() {
        let config = ExchangeConfig::default();
        let okx = Okx::new(config).unwrap();

        let exchange: &dyn Exchange = &okx;
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
