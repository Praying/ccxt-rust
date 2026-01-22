//! WsExchange trait implementation for HyperLiquid
//!
//! This module implements the unified `WsExchange` trait from `ccxt-core` for HyperLiquid.

use async_trait::async_trait;
use ccxt_core::{
    Error, Result,
    types::financial::{Amount, Price},
    types::{Balance, Market, Ohlcv, Order, OrderBook, Ticker, Timeframe, Trade},
    ws_client::WsConnectionState,
    ws_exchange::{MessageStream, WsExchange},
};
use rust_decimal::Decimal;
use rust_decimal::prelude::FromStr;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::time::{Duration, timeout};
use tokio_stream::wrappers::UnboundedReceiverStream;

use super::{HyperLiquid, parser};

async fn get_ws(exchange: &HyperLiquid) -> Result<super::ws::HyperLiquidWs> {
    let mut guard = exchange.ws_connection.write().await;
    if let Some(ws) = guard.as_ref() {
        return Ok(ws.clone());
    }
    let ws = exchange.create_ws();
    ws.connect().await?;
    *guard = Some(ws);
    Ok(guard.as_ref().unwrap().clone())
}

async fn resolve_market(
    exchange: &HyperLiquid,
    symbol: &str,
) -> Result<(String, String, Option<Arc<Market>>)> {
    if !symbol.contains('/') && !symbol.contains(':') {
        let normalized = format!("{}/USDC:USDC", symbol);
        return Ok((symbol.to_string(), normalized, None));
    }
    exchange.load_markets(false).await?;
    if let Ok(market) = exchange.base().market(symbol).await {
        return Ok((market.base.clone(), market.symbol.clone(), Some(market)));
    }
    let normalized = format!("{}/USDC:USDC", symbol);
    if let Ok(market) = exchange.base().market(&normalized).await {
        return Ok((market.base.clone(), market.symbol.clone(), Some(market)));
    }
    Ok((symbol.to_string(), normalized, None))
}

#[async_trait]
impl WsExchange for HyperLiquid {
    // ==================== Connection Management ====================

    async fn ws_connect(&self) -> Result<()> {
        let _ = get_ws(self).await?;
        Ok(())
    }

    async fn ws_disconnect(&self) -> Result<()> {
        let mut guard = self.ws_connection.write().await;
        if let Some(ws) = guard.take() {
            ws.disconnect().await?;
        }
        Ok(())
    }

    fn ws_is_connected(&self) -> bool {
        if let Ok(guard) = self.ws_connection.try_read() {
            if let Some(ws) = guard.as_ref() {
                return ws.is_connected();
            }
        }
        false
    }

    fn ws_state(&self) -> WsConnectionState {
        if let Ok(guard) = self.ws_connection.try_read() {
            if let Some(ws) = guard.as_ref() {
                return ws.state();
            }
        }
        WsConnectionState::Disconnected
    }

    fn subscriptions(&self) -> Vec<String> {
        if let Ok(guard) = self.ws_connection.try_read() {
            if let Some(ws) = guard.as_ref() {
                if let Ok(subs) = ws.subscriptions().try_read() {
                    return subs.iter().map(|s| format!("{:?}", s.sub_type)).collect();
                }
            }
        }
        Vec::new()
    }

    // ==================== Data Streams ====================
    async fn watch_ticker(&self, symbol: &str) -> Result<MessageStream<Ticker>> {
        let (base, symbol_name, market) = resolve_market(self, symbol).await?;

        let ws = get_ws(self).await?;
        ws.subscribe_all_mids().await?;
        ws.subscribe_l2_book(&base).await?;

        let (tx, rx) = mpsc::unbounded_channel::<Result<Ticker>>();
        let ws = ws.clone();
        tokio::spawn(async move {
            loop {
                if !ws.is_connected() {
                    let _ = ws.connect().await;
                }

                let msg = match timeout(Duration::from_secs(2), ws.receive()).await {
                    Ok(msg) => msg,
                    Err(_) => None,
                };

                if let Some(msg) = msg {
                    let channel = msg.get("channel").and_then(|v| v.as_str());
                    if channel == Some("allMids") {
                        let data = msg.get("data").unwrap_or(&msg);
                        if let Some(obj) = data.as_object() {
                            if let Some(mid) = obj.get(&base).and_then(|v| v.as_str()) {
                                if let Ok(price) = Decimal::from_str(mid) {
                                    match parser::parse_ticker(
                                        &symbol_name,
                                        price,
                                        market.as_deref(),
                                    ) {
                                        Ok(ticker) => {
                                            if tx.send(Ok(ticker)).is_err() {
                                                break;
                                            }
                                        }
                                        Err(e) => {
                                            let _ = tx.send(Err(e));
                                        }
                                    }
                                }
                            }
                        }
                        continue;
                    }

                    if channel == Some("l2Book") {
                        let data = msg.get("data").unwrap_or(&msg);
                        if let Some(coin) = data.get("coin").and_then(|v| v.as_str()) {
                            if coin != base {
                                continue;
                            }
                        }
                        if let Ok(book) = parser::parse_orderbook(data, symbol_name.clone()) {
                            if let (Some(best_bid), Some(best_ask)) =
                                (book.bids.first(), book.asks.first())
                            {
                                let mid = (best_bid.price.as_decimal()
                                    + best_ask.price.as_decimal())
                                    / Decimal::from(2);
                                let timestamp = chrono::Utc::now().timestamp_millis();
                                let ticker = Ticker {
                                    symbol: symbol_name.clone(),
                                    timestamp,
                                    datetime: parser::timestamp_to_datetime(timestamp),
                                    high: None,
                                    low: None,
                                    bid: Some(Price::new(best_bid.price.as_decimal())),
                                    bid_volume: Some(Amount::new(best_bid.amount.as_decimal())),
                                    ask: Some(Price::new(best_ask.price.as_decimal())),
                                    ask_volume: Some(Amount::new(best_ask.amount.as_decimal())),
                                    vwap: None,
                                    open: None,
                                    close: Some(Price::new(mid)),
                                    last: Some(Price::new(mid)),
                                    previous_close: None,
                                    change: None,
                                    percentage: None,
                                    average: None,
                                    base_volume: None,
                                    quote_volume: None,
                                    funding_rate: None,
                                    open_interest: None,
                                    index_price: None,
                                    mark_price: None,
                                    info: std::collections::HashMap::new(),
                                };
                                if tx.send(Ok(ticker)).is_err() {
                                    break;
                                }
                            }
                        }
                    }
                }
            }
        });

        let stream = UnboundedReceiverStream::new(rx);
        Ok(Box::pin(stream))
    }

    async fn watch_tickers(&self, _symbols: &[String]) -> Result<MessageStream<Vec<Ticker>>> {
        Err(Error::not_implemented("watch_tickers"))
    }

    async fn watch_order_book(
        &self,
        symbol: &str,
        _limit: Option<u32>,
    ) -> Result<MessageStream<OrderBook>> {
        let (base, symbol_name, _market) = resolve_market(self, symbol).await?;

        let ws = get_ws(self).await?;
        ws.subscribe_l2_book(&base).await?;

        let (tx, rx) = mpsc::unbounded_channel::<Result<OrderBook>>();
        let ws = ws.clone();
        tokio::spawn(async move {
            loop {
                if !ws.is_connected() {
                    let _ = ws.connect().await;
                }

                let msg = match timeout(Duration::from_secs(2), ws.receive()).await {
                    Ok(msg) => msg,
                    Err(_) => None,
                };

                if let Some(msg) = msg {
                    let channel = msg.get("channel").and_then(|v| v.as_str());
                    if channel != Some("l2Book") {
                        continue;
                    }
                    let data = msg.get("data").unwrap_or(&msg);
                    if let Some(coin) = data.get("coin").and_then(|v| v.as_str()) {
                        if coin != base {
                            continue;
                        }
                    }
                    match parser::parse_orderbook(data, symbol_name.clone()) {
                        Ok(book) => {
                            if tx.send(Ok(book)).is_err() {
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

        let stream = UnboundedReceiverStream::new(rx);
        Ok(Box::pin(stream))
    }

    async fn watch_trades(&self, symbol: &str) -> Result<MessageStream<Vec<Trade>>> {
        let (base, _symbol_name, market) = resolve_market(self, symbol).await?;

        let ws = get_ws(self).await?;
        ws.subscribe_trades(&base).await?;

        let (tx, rx) = mpsc::unbounded_channel::<Result<Vec<Trade>>>();
        let ws = ws.clone();
        tokio::spawn(async move {
            loop {
                if !ws.is_connected() {
                    let _ = ws.connect().await;
                }

                let msg = match timeout(Duration::from_secs(2), ws.receive()).await {
                    Ok(msg) => msg,
                    Err(_) => None,
                };

                if let Some(msg) = msg {
                    let channel = msg.get("channel").and_then(|v| v.as_str());
                    if channel != Some("trades") {
                        continue;
                    }
                    let data = msg.get("data").unwrap_or(&msg);
                    let mut trades = Vec::new();
                    if let Some(items) = data.as_array() {
                        for item in items {
                            if let Ok(trade) = parser::parse_trade(item, market.as_deref()) {
                                trades.push(trade);
                            }
                        }
                    } else if let Ok(trade) = parser::parse_trade(data, market.as_deref()) {
                        trades.push(trade);
                    }
                    if !trades.is_empty() {
                        if tx.send(Ok(trades)).is_err() {
                            break;
                        }
                    }
                }
            }
        });

        let stream = UnboundedReceiverStream::new(rx);
        Ok(Box::pin(stream))
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
        let address = self.wallet_address().ok_or_else(|| {
            Error::authentication("watch_orders requires authentication (wallet address)")
        })?;

        self.load_markets(false).await?;
        let ws = get_ws(self).await?;

        ws.subscribe_order_updates(address).await?;

        let (tx, rx) = mpsc::unbounded_channel::<Result<Order>>();
        let ws = ws.clone();
        let symbol_filter = _symbol.map(|s| s.to_string());
        let market_cache = self.base().market_cache.clone();
        tokio::spawn(async move {
            loop {
                if !ws.is_connected() {
                    let _ = ws.connect().await;
                }

                let msg = match timeout(Duration::from_secs(2), ws.receive()).await {
                    Ok(msg) => msg,
                    Err(_) => None,
                };

                if let Some(msg) = msg {
                    let channel = msg.get("channel").and_then(|v| v.as_str());
                    if channel != Some("orderUpdates") {
                        continue;
                    }
                    let data = msg.get("data").unwrap_or(&msg);
                    let mut items = Vec::new();
                    if let Some(array) = data.as_array() {
                        items.extend(array.iter().cloned());
                    } else if let Some(inner) = data.get("data") {
                        if let Some(array) = inner.as_array() {
                            items.extend(array.iter().cloned());
                        } else {
                            items.push(inner.clone());
                        }
                    } else {
                        items.push(data.clone());
                    }

                    for item in items {
                        let market = if let Some(coin) = item.get("coin").and_then(|v| v.as_str()) {
                            let symbol = format!("{}/USDC:USDC", coin);
                            market_cache.read().await.get_market(&symbol)
                        } else {
                            None
                        };

                        if let Ok(order) = parser::parse_order(&item, market.as_deref()) {
                            if let Some(ref filter) = symbol_filter {
                                if order.symbol != *filter {
                                    continue;
                                }
                            }
                            if tx.send(Ok(order)).is_err() {
                                return;
                            }
                        }
                    }
                }
            }
        });

        let stream = UnboundedReceiverStream::new(rx);
        Ok(Box::pin(stream))
    }

    async fn watch_my_trades(&self, _symbol: Option<&str>) -> Result<MessageStream<Trade>> {
        let address = self.wallet_address().ok_or_else(|| {
            Error::authentication("watch_my_trades requires authentication (wallet address)")
        })?;

        self.load_markets(false).await?;
        let ws = get_ws(self).await?;

        ws.subscribe_user_fills(address).await?;

        let (tx, rx) = mpsc::unbounded_channel::<Result<Trade>>();
        let ws = ws.clone();
        let symbol_filter = _symbol.map(|s| s.to_string());
        let market_cache = self.base().market_cache.clone();
        tokio::spawn(async move {
            loop {
                if !ws.is_connected() {
                    let _ = ws.connect().await;
                }

                let msg = match timeout(Duration::from_secs(2), ws.receive()).await {
                    Ok(msg) => msg,
                    Err(_) => None,
                };

                if let Some(msg) = msg {
                    let channel = msg.get("channel").and_then(|v| v.as_str());
                    if channel != Some("userFills") {
                        continue;
                    }
                    let data = msg.get("data").unwrap_or(&msg);
                    let mut items = Vec::new();
                    if let Some(array) = data.as_array() {
                        items.extend(array.iter().cloned());
                    } else if let Some(inner) = data.get("data") {
                        if let Some(array) = inner.as_array() {
                            items.extend(array.iter().cloned());
                        } else {
                            items.push(inner.clone());
                        }
                    } else {
                        items.push(data.clone());
                    }

                    for item in items {
                        let market = if let Some(coin) = item.get("coin").and_then(|v| v.as_str()) {
                            let symbol = format!("{}/USDC:USDC", coin);
                            market_cache.read().await.get_market(&symbol)
                        } else {
                            None
                        };

                        if let Ok(trade) = parser::parse_trade(&item, market.as_deref()) {
                            if let Some(ref filter) = symbol_filter {
                                if trade.symbol != *filter {
                                    continue;
                                }
                            }
                            if tx.send(Ok(trade)).is_err() {
                                return;
                            }
                        }
                    }
                }
            }
        });

        let stream = UnboundedReceiverStream::new(rx);
        Ok(Box::pin(stream))
    }

    async fn subscribe(&self, _channel: &str, _symbol: Option<&str>) -> Result<()> {
        Err(Error::not_implemented("subscribe"))
    }

    async fn unsubscribe(&self, _channel: &str, _symbol: Option<&str>) -> Result<()> {
        Err(Error::not_implemented("unsubscribe"))
    }
}
