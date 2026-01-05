//! WsExchange trait implementation for Binance
//!
//! This module implements the unified `WsExchange` trait from `ccxt-core` for Binance,
//! providing real-time WebSocket data streaming capabilities.

use async_trait::async_trait;
use ccxt_core::{
    error::{Error, Result},
    types::{
        Balance, Ohlcv, Order, OrderBook, Ticker, Timeframe, Trade, financial::Amount,
        financial::Price,
    },
    ws_client::WsConnectionState,
    ws_exchange::{MessageStream, WsExchange},
};

use rust_decimal::Decimal;
use std::collections::HashMap;
use tokio::sync::mpsc;

use super::Binance;

/// A simple stream wrapper that converts an mpsc receiver into a Stream
struct ReceiverStream<T> {
    receiver: mpsc::UnboundedReceiver<T>,
}

impl<T> ReceiverStream<T> {
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
impl WsExchange for Binance {
    // ==================== Connection Management ====================

    async fn ws_connect(&self) -> Result<()> {
        // Create a public WebSocket connection
        let ws = self.create_ws();
        ws.connect().await?;

        // Store the connected WebSocket instance for state tracking
        let mut conn = self.ws_connection.write().await;
        *conn = Some(ws);

        Ok(())
    }

    async fn ws_disconnect(&self) -> Result<()> {
        // Get and clear the stored WebSocket connection
        let mut conn = self.ws_connection.write().await;
        if let Some(ws) = conn.take() {
            ws.disconnect().await?;
        }
        Ok(())
    }

    fn ws_is_connected(&self) -> bool {
        // Use try_read to avoid blocking
        if let Ok(conn) = self.ws_connection.try_read() {
            if let Some(ref ws) = *conn {
                return ws.is_connected();
            }
        }
        false
    }

    fn ws_state(&self) -> WsConnectionState {
        // Use try_read to avoid blocking
        if let Ok(conn) = self.ws_connection.try_read() {
            if let Some(ref ws) = *conn {
                return ws.state();
            }
        }
        WsConnectionState::Disconnected
    }

    // ==================== Public Data Streams ====================

    async fn watch_ticker(&self, symbol: &str) -> Result<MessageStream<Ticker>> {
        // Load markets to validate symbol
        self.load_markets(false).await?;

        // Get market info
        let market = self.base.market(symbol).await?;
        let binance_symbol = market.id.to_lowercase();

        // Create WebSocket and connect
        let ws = self.create_ws();
        ws.connect().await?;

        // Subscribe to ticker stream
        ws.subscribe_ticker(&binance_symbol).await?;

        // Create a channel for streaming data
        let (tx, rx) = mpsc::unbounded_channel::<Result<Ticker>>();

        // Spawn a task to receive messages and forward them
        let market_clone = market.clone();
        tokio::spawn(async move {
            loop {
                if let Some(msg) = ws.receive().await {
                    // Skip subscription confirmations
                    if msg.get("result").is_some() || msg.get("id").is_some() {
                        continue;
                    }

                    // Parse ticker
                    match super::parser::parse_ws_ticker(&msg, Some(&market_clone)) {
                        Ok(ticker) => {
                            if tx.send(Ok(ticker)).is_err() {
                                break; // Receiver dropped
                            }
                        }
                        Err(e) => {
                            let _ = tx.send(Err(e));
                        }
                    }
                }
            }
        });

        // Convert receiver to stream
        let stream = ReceiverStream::new(rx);
        Ok(Box::pin(stream))
    }

    async fn watch_tickers(&self, symbols: &[String]) -> Result<MessageStream<Vec<Ticker>>> {
        // Load markets
        self.load_markets(false).await?;

        // Create WebSocket and connect
        let ws = self.create_ws();
        ws.connect().await?;

        // Subscribe to all requested symbols
        let mut markets = HashMap::new();
        for symbol in symbols {
            let market = self.base.market(symbol).await?;
            let binance_symbol = market.id.to_lowercase();
            ws.subscribe_ticker(&binance_symbol).await?;
            markets.insert(binance_symbol, market);
        }

        // Create channel for streaming
        let (tx, rx) = mpsc::unbounded_channel::<Result<Vec<Ticker>>>();

        // Spawn receiver task
        let markets_clone = markets;
        tokio::spawn(async move {
            let mut tickers: HashMap<String, Ticker> = HashMap::new();

            loop {
                if let Some(msg) = ws.receive().await {
                    // Skip subscription confirmations
                    if msg.get("result").is_some() || msg.get("id").is_some() {
                        continue;
                    }

                    // Get symbol from message
                    if let Some(symbol_str) = msg.get("s").and_then(|s| s.as_str()) {
                        let binance_symbol = symbol_str.to_lowercase();
                        if let Some(market) = markets_clone.get(&binance_symbol) {
                            if let Ok(ticker) = super::parser::parse_ws_ticker(&msg, Some(market)) {
                                tickers.insert(ticker.symbol.clone(), ticker);

                                // Send current state of all tickers
                                let ticker_vec: Vec<Ticker> = tickers.values().cloned().collect();
                                if tx.send(Ok(ticker_vec)).is_err() {
                                    break;
                                }
                            }
                        }
                    }
                }
            }
        });

        let stream = ReceiverStream::new(rx);
        Ok(Box::pin(stream))
    }

    async fn watch_order_book(
        &self,
        symbol: &str,
        limit: Option<u32>,
    ) -> Result<MessageStream<OrderBook>> {
        // Load markets
        self.load_markets(false).await?;

        // Get market info
        let market = self.base.market(symbol).await?;
        let binance_symbol = market.id.to_lowercase();

        // Create WebSocket and connect
        let ws = self.create_ws();
        ws.connect().await?;

        // Subscribe to orderbook stream
        let levels = limit.unwrap_or(20);
        ws.subscribe_orderbook(&binance_symbol, levels, "100ms")
            .await?;

        // Create channel
        let (tx, rx) = mpsc::unbounded_channel::<Result<OrderBook>>();

        // Spawn receiver task
        let symbol_clone = symbol.to_string();
        tokio::spawn(async move {
            loop {
                if let Some(msg) = ws.receive().await {
                    // Skip subscription confirmations
                    if msg.get("result").is_some() || msg.get("id").is_some() {
                        continue;
                    }

                    // Parse orderbook - pass String as required by the parser
                    match super::parser::parse_ws_orderbook(&msg, symbol_clone.clone()) {
                        Ok(orderbook) => {
                            if tx.send(Ok(orderbook)).is_err() {
                                break;
                            }
                        }
                        Err(e) => {
                            let _ = tx.send(Err(e));
                        }
                    }
                }
            }
        });

        let stream = ReceiverStream::new(rx);
        Ok(Box::pin(stream))
    }

    async fn watch_trades(&self, symbol: &str) -> Result<MessageStream<Vec<Trade>>> {
        // Load markets
        self.load_markets(false).await?;

        // Get market info
        let market = self.base.market(symbol).await?;
        let binance_symbol = market.id.to_lowercase();

        // Create WebSocket and connect
        let ws = self.create_ws();
        ws.connect().await?;

        // Subscribe to trades stream
        ws.subscribe_trades(&binance_symbol).await?;

        // Create channel
        let (tx, rx) = mpsc::unbounded_channel::<Result<Vec<Trade>>>();

        // Spawn receiver task
        let market_clone = market.clone();
        tokio::spawn(async move {
            loop {
                if let Some(msg) = ws.receive().await {
                    // Skip subscription confirmations
                    if msg.get("result").is_some() || msg.get("id").is_some() {
                        continue;
                    }

                    // Parse trade
                    match super::parser::parse_ws_trade(&msg, Some(&market_clone)) {
                        Ok(trade) => {
                            if tx.send(Ok(vec![trade])).is_err() {
                                break;
                            }
                        }
                        Err(e) => {
                            let _ = tx.send(Err(e));
                        }
                    }
                }
            }
        });

        let stream = ReceiverStream::new(rx);
        Ok(Box::pin(stream))
    }

    async fn watch_ohlcv(
        &self,
        symbol: &str,
        timeframe: Timeframe,
    ) -> Result<MessageStream<Ohlcv>> {
        // Load markets
        self.load_markets(false).await?;

        // Get market info
        let market = self.base.market(symbol).await?;
        let binance_symbol = market.id.to_lowercase();

        // Convert timeframe to Binance format
        let interval = timeframe.to_string();

        // Create WebSocket and connect
        let ws = self.create_ws();
        ws.connect().await?;

        // Subscribe to kline stream
        ws.subscribe_kline(&binance_symbol, &interval).await?;

        // Create channel
        let (tx, rx) = mpsc::unbounded_channel::<Result<Ohlcv>>();

        // Spawn receiver task
        tokio::spawn(async move {
            loop {
                if let Some(msg) = ws.receive().await {
                    // Skip subscription confirmations
                    if msg.get("result").is_some() || msg.get("id").is_some() {
                        continue;
                    }

                    // Parse OHLCV from kline message
                    match super::parser::parse_ws_ohlcv(&msg) {
                        Ok(ohlcv_f64) => {
                            // Convert OHLCV (f64) to Ohlcv (Decimal)
                            let ohlcv = Ohlcv {
                                timestamp: ohlcv_f64.timestamp,
                                open: Price::from(
                                    Decimal::try_from(ohlcv_f64.open).unwrap_or_default(),
                                ),
                                high: Price::from(
                                    Decimal::try_from(ohlcv_f64.high).unwrap_or_default(),
                                ),
                                low: Price::from(
                                    Decimal::try_from(ohlcv_f64.low).unwrap_or_default(),
                                ),
                                close: Price::from(
                                    Decimal::try_from(ohlcv_f64.close).unwrap_or_default(),
                                ),
                                volume: Amount::from(
                                    Decimal::try_from(ohlcv_f64.volume).unwrap_or_default(),
                                ),
                            };
                            if tx.send(Ok(ohlcv)).is_err() {
                                break;
                            }
                        }
                        Err(e) => {
                            let _ = tx.send(Err(e));
                        }
                    }
                }
            }
        });

        let stream = ReceiverStream::new(rx);
        Ok(Box::pin(stream))
    }

    // ==================== Private Data Streams ====================

    async fn watch_balance(&self) -> Result<MessageStream<Balance>> {
        // Check credentials
        self.base
            .check_required_credentials()
            .map_err(|_| Error::authentication("API credentials required for watch_balance"))?;

        // For user data streams, we need an Arc<Binance> to create authenticated WS
        // This is a limitation of the current design
        Err(Error::not_implemented(
            "watch_balance requires Arc<Binance> for authenticated WebSocket. \
             Use create_authenticated_ws() directly for now.",
        ))
    }

    async fn watch_orders(&self, _symbol: Option<&str>) -> Result<MessageStream<Order>> {
        // Check credentials
        self.base
            .check_required_credentials()
            .map_err(|_| Error::authentication("API credentials required for watch_orders"))?;

        Err(Error::not_implemented(
            "watch_orders requires Arc<Binance> for authenticated WebSocket. \
             Use create_authenticated_ws() directly for now.",
        ))
    }

    async fn watch_my_trades(&self, _symbol: Option<&str>) -> Result<MessageStream<Trade>> {
        // Check credentials
        self.base
            .check_required_credentials()
            .map_err(|_| Error::authentication("API credentials required for watch_my_trades"))?;

        Err(Error::not_implemented(
            "watch_my_trades requires Arc<Binance> for authenticated WebSocket. \
             Use create_authenticated_ws() directly for now.",
        ))
    }

    // ==================== Subscription Management ====================

    async fn subscribe(&self, channel: &str, symbol: Option<&str>) -> Result<()> {
        // Create WebSocket and connect
        let ws = self.create_ws();
        ws.connect().await?;

        // Use the appropriate subscription method based on channel
        match channel {
            "ticker" => {
                if let Some(sym) = symbol {
                    let market = self.base.market(sym).await?;
                    ws.subscribe_ticker(&market.id.to_lowercase()).await
                } else {
                    ws.subscribe_all_tickers().await
                }
            }
            "trade" | "trades" => {
                if let Some(sym) = symbol {
                    let market = self.base.market(sym).await?;
                    ws.subscribe_trades(&market.id.to_lowercase()).await
                } else {
                    Err(Error::invalid_request(
                        "Symbol required for trades subscription",
                    ))
                }
            }
            _ => {
                // For other channels, try generic subscription
                Err(Error::invalid_request(format!(
                    "Unknown channel: {}. Use specific watch_* methods instead.",
                    channel
                )))
            }
        }
    }

    async fn unsubscribe(&self, channel: &str, symbol: Option<&str>) -> Result<()> {
        // Build stream name
        let stream_name = if let Some(sym) = symbol {
            // Load markets to get proper symbol format
            self.load_markets(false).await?;
            let market = self.base.market(sym).await?;
            let binance_symbol = market.id.to_lowercase();
            format!("{}@{}", binance_symbol, channel)
        } else {
            channel.to_string()
        };

        // Create WS to unsubscribe
        let ws = self.create_ws();
        ws.unsubscribe(stream_name).await
    }

    fn subscriptions(&self) -> Vec<String> {
        // Use try_read to avoid blocking
        if let Ok(conn) = self.ws_connection.try_read() {
            if let Some(ref ws) = *conn {
                return ws.subscriptions();
            }
        }
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ccxt_core::ExchangeConfig;

    #[test]
    fn test_ws_exchange_trait_object_safety() {
        let config = ExchangeConfig::default();
        let binance = Binance::new(config).unwrap();

        // Test that we can create a WsExchange trait object
        let _ws_exchange: &dyn WsExchange = &binance;

        // Test connection state methods
        assert!(!binance.ws_is_connected());
        assert_eq!(binance.ws_state(), WsConnectionState::Disconnected);
    }

    #[test]
    fn test_subscriptions_empty_by_default() {
        let config = ExchangeConfig::default();
        let binance = Binance::new(config).unwrap();

        let subs = binance.subscriptions();
        assert!(subs.is_empty());
    }
}
