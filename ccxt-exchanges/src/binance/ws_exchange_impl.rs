//! WsExchange trait implementation for Binance
//!
//! This module implements the unified `WsExchange` trait from `ccxt-core` for Binance,
//! providing real-time WebSocket data streaming capabilities.

use async_trait::async_trait;
use ccxt_core::{
    error::{Error, Result},
    types::{
        Balance, MarketType, Ohlcv, Order, OrderBook, Ticker, Timeframe, Trade, financial::Amount,
        financial::Price,
    },
    ws_client::WsConnectionState,
    ws_exchange::{MessageStream, WsExchange},
};

use rust_decimal::Decimal;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;

use super::Binance;

/// A simple stream wrapper that converts an mpsc receiver into a Stream
struct ReceiverStream<T> {
    receiver: mpsc::Receiver<T>,
}

impl<T> ReceiverStream<T> {
    fn new(receiver: mpsc::Receiver<T>) -> Self {
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
        // Ensure at least one public connection is active for the default market type
        let default_market_type = MarketType::from(self.options.default_type);
        let _ = self
            .connection_manager
            .get_public_connection(default_market_type)
            .await?;
        Ok(())
    }

    async fn ws_disconnect(&self) -> Result<()> {
        self.connection_manager.disconnect_all().await
    }

    fn ws_is_connected(&self) -> bool {
        self.connection_manager.is_connected()
    }

    fn ws_state(&self) -> WsConnectionState {
        if self.connection_manager.is_connected() {
            WsConnectionState::Connected
        } else {
            WsConnectionState::Disconnected
        }
    }

    // ==================== Public Data Streams ====================

    async fn watch_ticker(&self, symbol: &str) -> Result<MessageStream<Ticker>> {
        // Load markets to validate symbol
        self.load_markets(false).await?;

        // Get market info
        let market = self.base.market(symbol).await?;
        let binance_symbol = market.id.to_lowercase();
        let stream = format!("{}@ticker", binance_symbol);

        // Get shared public connection for this market type
        let ws = self
            .connection_manager
            .get_public_connection(market.market_type)
            .await?;

        // Create subscription channel
        let (tx, mut rx) = mpsc::channel(1024);

        // Register with manager
        let is_new = ws
            .subscription_manager
            .add_subscription(
                stream.clone(),
                symbol.to_string(),
                super::ws::SubscriptionType::Ticker,
                tx,
            )
            .await?;

        // Only send subscribe command if this is a new subscription
        if is_new {
            ws.message_router.subscribe(vec![stream]).await?;
        }

        // Create user channel
        let (user_tx, user_rx) = mpsc::channel::<Result<Ticker>>(1024);

        // Spawn parser task
        let market_clone = market.clone();
        tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                match super::parser::parse_ws_ticker(&msg, Some(&market_clone)) {
                    Ok(ticker) => {
                        if user_tx.send(Ok(ticker)).await.is_err() {
                            break;
                        }
                    }
                    Err(e) => {
                        let _ = user_tx.send(Err(e)).await;
                    }
                }
            }
        });

        // Convert receiver to stream
        let stream = ReceiverStream::new(user_rx);
        Ok(Box::pin(stream))
    }

    async fn watch_tickers(&self, symbols: &[String]) -> Result<MessageStream<Vec<Ticker>>> {
        // Load markets
        self.load_markets(false).await?;

        // Create aggregator channel
        let (agg_tx, mut agg_rx) = mpsc::channel::<Ticker>(1024);

        // Subscribe to all requested symbols (potentially across shards)
        let mut markets = HashMap::new();
        for symbol in symbols {
            let market = self.base.market(symbol).await?;
            let binance_symbol = market.id.to_lowercase();
            let stream = format!("{}@ticker", binance_symbol);

            markets.insert(binance_symbol.clone(), market);

            // Get connection (might be different for each symbol if sharding active)
            let Some(market_ref) = markets.get(&binance_symbol) else {
                continue;
            };
            let ws = self
                .connection_manager
                .get_public_connection(market_ref.market_type)
                .await?;
            let (tx, mut rx) = mpsc::channel(1024);

            let is_new = ws
                .subscription_manager
                .add_subscription(
                    stream.clone(),
                    symbol.clone(),
                    super::ws::SubscriptionType::Ticker,
                    tx,
                )
                .await?;

            if is_new {
                ws.message_router.subscribe(vec![stream]).await?;
            }

            // Spawn parser task for this symbol
            let agg_tx_clone = agg_tx.clone();
            let market_clone = self.base.market(symbol).await?;

            tokio::spawn(async move {
                while let Some(msg) = rx.recv().await {
                    if let Ok(ticker) = super::parser::parse_ws_ticker(&msg, Some(&market_clone)) {
                        let _ = agg_tx_clone.send(ticker).await;
                    }
                }
            });
        }

        drop(agg_tx);

        // Create user channel
        let (user_tx, user_rx) = mpsc::channel::<Result<Vec<Ticker>>>(1024);

        // Spawn aggregator task
        tokio::spawn(async move {
            let mut tickers: HashMap<String, Ticker> = HashMap::new();

            while let Some(ticker) = agg_rx.recv().await {
                tickers.insert(ticker.symbol.clone(), ticker);

                let ticker_vec: Vec<Ticker> = tickers.values().cloned().collect();
                if user_tx.send(Ok(ticker_vec)).await.is_err() {
                    break;
                }
            }
        });

        let stream = ReceiverStream::new(user_rx);
        Ok(Box::pin(stream))
    }

    async fn watch_order_book(
        &self,
        symbol: &str,
        limit: Option<u32>,
    ) -> Result<MessageStream<OrderBook>> {
        // Validate depth limit if provided
        const VALID_WS_DEPTH_LIMITS: &[u32] = &[5, 10, 20];
        if let Some(l) = limit {
            if !VALID_WS_DEPTH_LIMITS.contains(&l) {
                return Err(Error::invalid_request(format!(
                    "Invalid WebSocket depth limit: {}. Valid values: {:?}",
                    l, VALID_WS_DEPTH_LIMITS
                )));
            }
        }

        // Load markets
        self.load_markets(false).await?;

        // Get market info
        let market = self.base.market(symbol).await?;
        let binance_symbol = market.id.to_lowercase();

        // Choose stream based on limit:
        // - With limit (5/10/20): use @depth{N}@100ms for partial book snapshots
        // - Without limit: use @depth@100ms for incremental diff updates
        // When no limit is specified, default to @depth20@100ms (partial book snapshots)
        // instead of @depth@100ms (diff stream) which requires snapshot+delta management.
        // This is consistent with CCXT Python behavior.
        let stream = if let Some(l) = limit {
            format!("{}@depth{}@100ms", binance_symbol, l)
        } else {
            format!("{}@depth20@100ms", binance_symbol)
        };

        // Get shared public connection for this market type
        let ws = self
            .connection_manager
            .get_public_connection(market.market_type)
            .await?;

        // Create subscription channel
        let (tx, mut rx) = mpsc::channel(1024);

        // Register with manager
        let is_new = ws
            .subscription_manager
            .add_subscription(
                stream.clone(),
                symbol.to_string(),
                super::ws::SubscriptionType::OrderBook,
                tx,
            )
            .await?;

        // Only send subscribe command if this is a new subscription
        if is_new {
            ws.message_router.subscribe(vec![stream]).await?;
        }

        // Create user channel
        let (user_tx, user_rx) = mpsc::channel::<Result<OrderBook>>(1024);

        // Spawn parser task
        let symbol_clone = symbol.to_string();
        tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                match super::parser::parse_ws_orderbook(&msg, symbol_clone.clone()) {
                    Ok(orderbook) => {
                        if user_tx.send(Ok(orderbook)).await.is_err() {
                            break;
                        }
                    }
                    Err(e) => {
                        let _ = user_tx.send(Err(e)).await;
                    }
                }
            }
        });

        let stream = ReceiverStream::new(user_rx);
        Ok(Box::pin(stream))
    }

    async fn watch_trades(&self, symbol: &str) -> Result<MessageStream<Vec<Trade>>> {
        // Load markets
        self.load_markets(false).await?;

        // Get market info
        let market = self.base.market(symbol).await?;
        let binance_symbol = market.id.to_lowercase();
        let stream = format!("{}@trade", binance_symbol);

        // Get shared public connection for this market type
        let ws = self
            .connection_manager
            .get_public_connection(market.market_type)
            .await?;

        // Create subscription channel
        let (tx, mut rx) = mpsc::channel(1024);

        // Register with manager
        let is_new = ws
            .subscription_manager
            .add_subscription(
                stream.clone(),
                symbol.to_string(),
                super::ws::SubscriptionType::Trades,
                tx,
            )
            .await?;

        // Only send subscribe command if this is a new subscription
        if is_new {
            ws.message_router.subscribe(vec![stream]).await?;
        }

        // Create user channel
        let (user_tx, user_rx) = mpsc::channel::<Result<Vec<Trade>>>(1024);

        // Spawn parser task
        let market_clone = market.clone();
        tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                match super::parser::parse_ws_trade(&msg, Some(&market_clone)) {
                    Ok(trade) => {
                        if user_tx.send(Ok(vec![trade])).await.is_err() {
                            break;
                        }
                    }
                    Err(e) => {
                        let _ = user_tx.send(Err(e)).await;
                    }
                }
            }
        });

        // Convert receiver to stream
        let stream = ReceiverStream::new(user_rx);
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
        let stream = format!("{}@kline_{}", binance_symbol, interval);

        // Get shared public connection for this market type
        let ws = self
            .connection_manager
            .get_public_connection(market.market_type)
            .await?;

        // Create subscription channel
        let (tx, mut rx) = mpsc::channel(1024);

        // Register with manager
        let is_new = ws
            .subscription_manager
            .add_subscription(
                stream.clone(),
                symbol.to_string(),
                super::ws::SubscriptionType::Kline(interval),
                tx,
            )
            .await?;

        // Only send subscribe command if this is a new subscription
        if is_new {
            ws.message_router.subscribe(vec![stream]).await?;
        }

        // Create user channel
        let (user_tx, user_rx) = mpsc::channel::<Result<Ohlcv>>(1024);

        // Spawn parser task
        tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
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
                            low: Price::from(Decimal::try_from(ohlcv_f64.low).unwrap_or_default()),
                            close: Price::from(
                                Decimal::try_from(ohlcv_f64.close).unwrap_or_default(),
                            ),
                            volume: Amount::from(
                                Decimal::try_from(ohlcv_f64.volume).unwrap_or_default(),
                            ),
                        };
                        if user_tx.send(Ok(ohlcv)).await.is_err() {
                            break;
                        }
                    }
                    Err(e) => {
                        let _ = user_tx.send(Err(e)).await;
                    }
                }
            }
        });

        let stream = ReceiverStream::new(user_rx);
        Ok(Box::pin(stream))
    }

    // ==================== Private Data Streams ====================

    async fn watch_balance(&self) -> Result<MessageStream<Balance>> {
        self.base
            .check_required_credentials()
            .map_err(|_| Error::authentication("API credentials required for watch_balance"))?;

        let binance_arc = Arc::new(self.clone());
        let default_market_type = MarketType::from(self.options.default_type);
        let ws = self
            .connection_manager
            .get_private_connection(default_market_type, &binance_arc)
            .await?;

        let (tx, mut rx) = mpsc::channel(1024);

        ws.subscription_manager
            .add_subscription(
                "!userData".to_string(),
                "user".to_string(),
                super::ws::SubscriptionType::Balance,
                tx,
            )
            .await?;

        let (user_tx, user_rx) = mpsc::channel::<Result<Balance>>(1024);
        let account_type = self.options.default_type.to_string();
        let balances_cache = ws.balances.clone();

        tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                if let Some(event_type) = msg.get("e").and_then(|e| e.as_str()) {
                    if matches!(
                        event_type,
                        "balanceUpdate" | "outboundAccountPosition" | "ACCOUNT_UPDATE"
                    ) {
                        if let Ok(()) = super::ws::user_data::handle_balance_message(
                            &msg,
                            &account_type,
                            &balances_cache,
                        )
                        .await
                        {
                            let balances = balances_cache.read().await;
                            if let Some(balance) = balances.get(&account_type) {
                                if user_tx.send(Ok(balance.clone())).await.is_err() {
                                    break;
                                }
                            }
                        }
                    }
                }
            }
        });

        let stream = ReceiverStream::new(user_rx);
        Ok(Box::pin(stream))
    }

    async fn watch_orders(&self, symbol: Option<&str>) -> Result<MessageStream<Order>> {
        self.base
            .check_required_credentials()
            .map_err(|_| Error::authentication("API credentials required for watch_orders"))?;

        let binance_arc = Arc::new(self.clone());
        let default_market_type = MarketType::from(self.options.default_type);
        let ws = self
            .connection_manager
            .get_private_connection(default_market_type, &binance_arc)
            .await?;

        let (tx, mut rx) = mpsc::channel(1024);

        ws.subscription_manager
            .add_subscription(
                "!userData".to_string(),
                "user".to_string(),
                super::ws::SubscriptionType::Orders,
                tx,
            )
            .await?;

        let (user_tx, user_rx) = mpsc::channel::<Result<Order>>(1024);
        let symbol_filter = symbol.map(ToString::to_string);
        let orders_cache = ws.orders.clone();

        tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                if let Some(data) = msg.as_object() {
                    if let Some(event_type) = data.get("e").and_then(|e| e.as_str()) {
                        let order = match event_type {
                            // Spot market order updates
                            "executionReport" => Some(super::ws::user_data::parse_ws_order(data)),
                            // Futures market order updates
                            "ORDER_TRADE_UPDATE" => {
                                super::ws::user_data::parse_order_trade_update_to_order(&msg).ok()
                            }
                            _ => None,
                        };

                        if let Some(order) = order {
                            {
                                let mut orders = orders_cache.write().await;
                                let symbol_orders = orders
                                    .entry(order.symbol.clone())
                                    .or_insert_with(HashMap::new);
                                symbol_orders.insert(order.id.clone(), order.clone());
                            }

                            // Filter and send
                            if let Some(s) = &symbol_filter {
                                if &order.symbol != s {
                                    continue;
                                }
                            }

                            if user_tx.send(Ok(order)).await.is_err() {
                                break;
                            }
                        }
                    }
                }
            }
        });

        let stream = ReceiverStream::new(user_rx);
        Ok(Box::pin(stream))
    }

    async fn watch_my_trades(&self, symbol: Option<&str>) -> Result<MessageStream<Trade>> {
        self.base
            .check_required_credentials()
            .map_err(|_| Error::authentication("API credentials required for watch_my_trades"))?;

        let binance_arc = Arc::new(self.clone());
        let default_market_type = MarketType::from(self.options.default_type);
        let ws = self
            .connection_manager
            .get_private_connection(default_market_type, &binance_arc)
            .await?;

        let (tx, mut rx) = mpsc::channel(1024);

        ws.subscription_manager
            .add_subscription(
                "!userData".to_string(),
                "user".to_string(),
                super::ws::SubscriptionType::MyTrades,
                tx,
            )
            .await?;

        let (user_tx, user_rx) = mpsc::channel::<Result<Trade>>(1024);
        let symbol_filter = symbol.map(ToString::to_string);
        let trades_cache = ws.my_trades.clone();

        tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                if let Some(event_type) = msg.get("e").and_then(|e| e.as_str()) {
                    let trade_result = match event_type {
                        // Spot market trade executions
                        "executionReport" => super::ws::user_data::parse_ws_trade(&msg).ok(),
                        // Futures market trade executions
                        "ORDER_TRADE_UPDATE" => {
                            super::ws::user_data::parse_order_trade_update_to_trade(&msg).ok()
                        }
                        _ => None,
                    };

                    if let Some(trade) = trade_result {
                        {
                            let mut trades = trades_cache.write().await;

                            let symbol_trades = trades
                                .entry(trade.symbol.clone())
                                .or_insert_with(std::collections::VecDeque::new);
                            symbol_trades.push_front(trade.clone());
                            if symbol_trades.len() > 1000 {
                                symbol_trades.pop_back();
                            }
                        }

                        // Filter and send
                        if let Some(s) = &symbol_filter {
                            if &trade.symbol != s {
                                continue;
                            }
                        }

                        if user_tx.send(Ok(trade)).await.is_err() {
                            break;
                        }
                    }
                }
            }
        });

        let stream = ReceiverStream::new(user_rx);
        Ok(Box::pin(stream))
    }

    // ==================== Subscription Management ====================

    async fn subscribe(&self, channel: &str, symbol: Option<&str>) -> Result<()> {
        // Determine market type from symbol or use default
        let market_type = if let Some(sym) = symbol {
            let market = self.base.market(sym).await?;
            market.market_type
        } else {
            MarketType::from(self.options.default_type)
        };

        // Create WebSocket and connect
        let ws = self
            .connection_manager
            .get_public_connection(market_type)
            .await?;

        // Use the appropriate subscription method based on channel
        // Note: The receiver is intentionally dropped here as this is a fire-and-forget subscription.
        // For proper message handling, use the specific watch_* methods instead.
        match channel {
            "ticker" => {
                if let Some(sym) = symbol {
                    let market = self.base.market(sym).await?;
                    ws.subscribe_ticker(&market.id.to_lowercase())
                        .await
                        .map(|_| ())
                } else {
                    ws.subscribe_all_tickers().await.map(|_| ())
                }
            }
            "trade" | "trades" => {
                if let Some(sym) = symbol {
                    let market = self.base.market(sym).await?;
                    ws.subscribe_trades(&market.id.to_lowercase())
                        .await
                        .map(|_| ())
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

        // Determine market type from symbol or use default
        let market_type = if let Some(sym) = symbol {
            let market = self.base.market(sym).await?;
            market.market_type
        } else {
            MarketType::from(self.options.default_type)
        };

        // Create WS to unsubscribe
        let ws = self
            .connection_manager
            .get_public_connection(market_type)
            .await?;
        ws.unsubscribe(stream_name).await
    }

    fn subscriptions(&self) -> Vec<String> {
        self.connection_manager.get_all_subscriptions()
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::disallowed_methods)]
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
