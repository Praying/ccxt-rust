//! WsExchange trait implementation for HyperLiquid
//!
//! This module implements the unified `WsExchange` trait from `ccxt-core` for HyperLiquid.
//!
//! Note: Full WsExchange implementation requires async_stream and additional setup.
//! This is a placeholder that will be completed when WebSocket streaming is needed.

use async_trait::async_trait;
use ccxt_core::{
    Error, Result,
    types::{Balance, Ohlcv, Order, OrderBook, Ticker, Timeframe, Trade},
    ws_client::WsConnectionState,
    ws_exchange::{MessageStream, WsExchange},
};

use super::HyperLiquid;

#[async_trait]
impl WsExchange for HyperLiquid {
    // ==================== Connection Management ====================

    async fn ws_connect(&self) -> Result<()> {
        Err(Error::not_implemented(
            "ws_connect - WebSocket not yet initialized",
        ))
    }

    async fn ws_disconnect(&self) -> Result<()> {
        Err(Error::not_implemented(
            "ws_disconnect - WebSocket not yet initialized",
        ))
    }

    fn ws_is_connected(&self) -> bool {
        false
    }

    fn ws_state(&self) -> WsConnectionState {
        WsConnectionState::Disconnected
    }

    fn subscriptions(&self) -> Vec<String> {
        Vec::new()
    }

    // ==================== Data Streams ====================
    async fn watch_ticker(&self, _symbol: &str) -> Result<MessageStream<Ticker>> {
        Err(Error::not_implemented(
            "watch_ticker - WebSocket streaming requires additional setup",
        ))
    }

    async fn watch_tickers(&self, _symbols: &[String]) -> Result<MessageStream<Vec<Ticker>>> {
        Err(Error::not_implemented("watch_tickers"))
    }

    async fn watch_order_book(
        &self,
        _symbol: &str,
        _limit: Option<u32>,
    ) -> Result<MessageStream<OrderBook>> {
        Err(Error::not_implemented(
            "watch_order_book - WebSocket streaming requires additional setup",
        ))
    }

    async fn watch_trades(&self, _symbol: &str) -> Result<MessageStream<Vec<Trade>>> {
        Err(Error::not_implemented(
            "watch_trades - WebSocket streaming requires additional setup",
        ))
    }

    async fn watch_ohlcv(
        &self,
        _symbol: &str,
        _timeframe: Timeframe,
    ) -> Result<MessageStream<Ohlcv>> {
        Err(Error::not_implemented("watch_ohlcv"))
    }

    async fn watch_balance(&self) -> Result<MessageStream<Balance>> {
        Err(Error::not_implemented("watch_balance"))
    }

    async fn watch_orders(&self, _symbol: Option<&str>) -> Result<MessageStream<Order>> {
        Err(Error::not_implemented("watch_orders"))
    }

    async fn watch_my_trades(&self, _symbol: Option<&str>) -> Result<MessageStream<Trade>> {
        Err(Error::not_implemented("watch_my_trades"))
    }

    async fn subscribe(&self, _channel: &str, _symbol: Option<&str>) -> Result<()> {
        Err(Error::not_implemented("subscribe"))
    }

    async fn unsubscribe(&self, _channel: &str, _symbol: Option<&str>) -> Result<()> {
        Err(Error::not_implemented("unsubscribe"))
    }
}
