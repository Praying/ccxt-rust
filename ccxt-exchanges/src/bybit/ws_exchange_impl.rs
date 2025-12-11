//! WsExchange trait implementation for Bybit
//!
//! This module implements the unified `WsExchange` trait from `ccxt-core` for Bybit,
//! providing real-time WebSocket data streaming capabilities.

use async_trait::async_trait;
use ccxt_core::{
    error::{Error, Result},
    types::{Balance, Ohlcv, Order, OrderBook, Ticker, Timeframe, Trade},
    ws_client::WsConnectionState,
    ws_exchange::{MessageStream, WsExchange},
};

use tokio::sync::mpsc;

use super::Bybit;

/// A simple stream wrapper that converts an mpsc receiver into a Stream
struct ReceiverStream<T> {
    receiver: mpsc::UnboundedReceiver<T>,
}

impl<T> ReceiverStream<T> {
    #[allow(dead_code)]
    fn new(receiver: mpsc::UnboundedReceiver<T>) -> Self {
        Self { receiver }
    }
}

impl<T> futures::Stream for ReceiverStream<T> {
    type Item = T;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        self.receiver.poll_recv(cx)
    }
}

#[async_trait]
impl WsExchange for Bybit {
    // ==================== Connection Management ====================

    async fn ws_connect(&self) -> Result<()> {
        let ws = self.create_ws();
        ws.connect().await?;
        Ok(())
    }

    async fn ws_disconnect(&self) -> Result<()> {
        let ws = self.create_ws();
        ws.disconnect().await
    }

    fn ws_is_connected(&self) -> bool {
        // Without persistent state tracking, return false
        // In a full implementation, we'd track the active connection
        false
    }

    fn ws_state(&self) -> WsConnectionState {
        // Return disconnected since we don't have persistent state
        WsConnectionState::Disconnected
    }

    // ==================== Public Data Streams ====================

    async fn watch_ticker(&self, symbol: &str) -> Result<MessageStream<Ticker>> {
        let ws = self.create_ws();

        // Convert unified symbol (BTC/USDT) to Bybit format (BTCUSDT)
        let bybit_symbol = symbol.replace('/', "");

        // Try to get market info if available (optional)
        let market = self.base().market(symbol).await.ok();

        ws.watch_ticker(&bybit_symbol, market).await
    }

    async fn watch_tickers(&self, _symbols: &[String]) -> Result<MessageStream<Vec<Ticker>>> {
        // TODO: Implement multi-symbol ticker watching
        Err(Error::not_implemented(
            "watch_tickers not yet implemented for Bybit",
        ))
    }

    async fn watch_order_book(
        &self,
        symbol: &str,
        limit: Option<u32>,
    ) -> Result<MessageStream<OrderBook>> {
        let ws = self.create_ws();

        // Convert unified symbol (BTC/USDT) to Bybit format (BTCUSDT)
        let bybit_symbol = symbol.replace('/', "");

        ws.watch_order_book(&bybit_symbol, limit).await
    }

    async fn watch_trades(&self, symbol: &str) -> Result<MessageStream<Vec<Trade>>> {
        let ws = self.create_ws();

        // Convert unified symbol (BTC/USDT) to Bybit format (BTCUSDT)
        let bybit_symbol = symbol.replace('/', "");

        // Try to get market info if available (optional)
        let market = self.base().market(symbol).await.ok();

        ws.watch_trades(&bybit_symbol, market).await
    }

    async fn watch_ohlcv(
        &self,
        _symbol: &str,
        _timeframe: Timeframe,
    ) -> Result<MessageStream<Ohlcv>> {
        Err(Error::not_implemented(
            "watch_ohlcv not yet implemented for Bybit",
        ))
    }

    // ==================== Private Data Streams ====================

    async fn watch_balance(&self) -> Result<MessageStream<Balance>> {
        Err(Error::not_implemented(
            "watch_balance not yet implemented for Bybit",
        ))
    }

    async fn watch_orders(&self, _symbol: Option<&str>) -> Result<MessageStream<Order>> {
        Err(Error::not_implemented(
            "watch_orders not yet implemented for Bybit",
        ))
    }

    async fn watch_my_trades(&self, _symbol: Option<&str>) -> Result<MessageStream<Trade>> {
        Err(Error::not_implemented(
            "watch_my_trades not yet implemented for Bybit",
        ))
    }

    // ==================== Subscription Management ====================

    async fn subscribe(&self, _channel: &str, _symbol: Option<&str>) -> Result<()> {
        Err(Error::not_implemented(
            "subscribe not yet implemented for Bybit",
        ))
    }

    async fn unsubscribe(&self, _channel: &str, _symbol: Option<&str>) -> Result<()> {
        Err(Error::not_implemented(
            "unsubscribe not yet implemented for Bybit",
        ))
    }

    fn subscriptions(&self) -> Vec<String> {
        Vec::new()
    }
}
