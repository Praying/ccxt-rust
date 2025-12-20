//! Exchange trait implementation for HyperLiquid
//!
//! This module implements the unified `Exchange` trait from `ccxt-core` for HyperLiquid.

use async_trait::async_trait;
use ccxt_core::{
    Result,
    exchange::{Capability, Exchange, ExchangeCapabilities},
    types::{
        Balance, Market, Ohlcv, Order, OrderBook, OrderSide, OrderType, Ticker, Timeframe, Trade,
    },
};
use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;
use std::collections::HashMap;

use super::HyperLiquid;

#[async_trait]
impl Exchange for HyperLiquid {
    // ==================== Metadata ====================

    fn id(&self) -> &str {
        "hyperliquid"
    }

    fn name(&self) -> &str {
        "HyperLiquid"
    }

    fn version(&self) -> &'static str {
        "1"
    }

    fn certified(&self) -> bool {
        false
    }

    fn has_websocket(&self) -> bool {
        true
    }

    fn capabilities(&self) -> ExchangeCapabilities {
        // HyperLiquid supports:
        // - Market Data: markets, ticker, tickers, order_book, trades, ohlcv
        // - Trading: create_order, cancel_order, cancel_all_orders, open_orders
        // - Account: balance
        // - Margin: funding_rate, positions, set_leverage
        // - WebSocket: ticker, order_book, trades, orders
        ExchangeCapabilities::builder()
            .market_data()
            .trading()
            // Remove unsupported market data capabilities
            .without_capability(Capability::FetchCurrencies)
            .without_capability(Capability::FetchStatus)
            .without_capability(Capability::FetchTime)
            // Remove unsupported trading capabilities
            .without_capability(Capability::EditOrder)
            .without_capability(Capability::FetchOrder)
            .without_capability(Capability::FetchOrders)
            .without_capability(Capability::FetchClosedOrders)
            .without_capability(Capability::FetchCanceledOrders)
            // Add account capabilities
            .capability(Capability::FetchBalance)
            // Add margin capabilities
            .capability(Capability::FetchFundingRate)
            .capability(Capability::FetchPositions)
            .capability(Capability::SetLeverage)
            // Add WebSocket capabilities
            .capability(Capability::Websocket)
            .capability(Capability::WatchTicker)
            .capability(Capability::WatchOrderBook)
            .capability(Capability::WatchTrades)
            .capability(Capability::WatchOrders)
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
            Timeframe::D1,
            Timeframe::W1,
        ]
    }

    fn rate_limit(&self) -> f64 {
        100.0
    }

    // ==================== Market Data (Public API) ====================

    async fn fetch_markets(&self) -> Result<Vec<Market>> {
        HyperLiquid::fetch_markets(self).await
    }

    async fn load_markets(&self, reload: bool) -> Result<HashMap<String, Market>> {
        HyperLiquid::load_markets(self, reload).await
    }

    async fn fetch_ticker(&self, symbol: &str) -> Result<Ticker> {
        HyperLiquid::fetch_ticker(self, symbol).await
    }

    async fn fetch_tickers(&self, symbols: Option<&[String]>) -> Result<Vec<Ticker>> {
        let symbols_vec = symbols.map(|s| s.to_vec());
        HyperLiquid::fetch_tickers(self, symbols_vec).await
    }

    async fn fetch_order_book(&self, symbol: &str, limit: Option<u32>) -> Result<OrderBook> {
        HyperLiquid::fetch_order_book(self, symbol, limit).await
    }

    async fn fetch_trades(&self, symbol: &str, limit: Option<u32>) -> Result<Vec<Trade>> {
        HyperLiquid::fetch_trades(self, symbol, limit).await
    }

    async fn fetch_ohlcv(
        &self,
        symbol: &str,
        timeframe: Timeframe,
        since: Option<i64>,
        limit: Option<u32>,
    ) -> Result<Vec<Ohlcv>> {
        let timeframe_str = timeframe.to_string();
        HyperLiquid::fetch_ohlcv(self, symbol, &timeframe_str, since, limit).await
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

        HyperLiquid::create_order(self, symbol, order_type, side, amount_f64, price_f64).await
    }

    async fn cancel_order(&self, id: &str, symbol: Option<&str>) -> Result<Order> {
        let symbol_str = symbol.ok_or_else(|| {
            ccxt_core::Error::invalid_request("Symbol is required for cancel_order on HyperLiquid")
        })?;
        HyperLiquid::cancel_order(self, id, symbol_str).await
    }

    async fn cancel_all_orders(&self, symbol: Option<&str>) -> Result<Vec<Order>> {
        HyperLiquid::cancel_all_orders(self, symbol).await
    }

    async fn fetch_order(&self, _id: &str, _symbol: Option<&str>) -> Result<Order> {
        Err(ccxt_core::Error::not_implemented("fetch_order"))
    }

    async fn fetch_open_orders(
        &self,
        symbol: Option<&str>,
        since: Option<i64>,
        limit: Option<u32>,
    ) -> Result<Vec<Order>> {
        HyperLiquid::fetch_open_orders(self, symbol, since, limit).await
    }

    async fn fetch_closed_orders(
        &self,
        _symbol: Option<&str>,
        _since: Option<i64>,
        _limit: Option<u32>,
    ) -> Result<Vec<Order>> {
        Err(ccxt_core::Error::not_implemented("fetch_closed_orders"))
    }

    // ==================== Account (Private API) ====================

    async fn fetch_balance(&self) -> Result<Balance> {
        HyperLiquid::fetch_balance(self).await
    }

    async fn fetch_my_trades(
        &self,
        _symbol: Option<&str>,
        _since: Option<i64>,
        _limit: Option<u32>,
    ) -> Result<Vec<Trade>> {
        Err(ccxt_core::Error::not_implemented("fetch_my_trades"))
    }

    // ==================== Helper Methods ====================

    async fn market(&self, symbol: &str) -> Result<Market> {
        self.base().market(symbol).await
    }

    async fn markets(&self) -> HashMap<String, Market> {
        let cache = self.base().market_cache.read().await;
        cache.markets.clone()
    }
}
