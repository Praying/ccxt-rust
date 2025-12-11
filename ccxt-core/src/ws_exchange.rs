//! # WebSocket Exchange Trait
//!
//! This module defines the [`WsExchange`] trait for real-time data streaming
//! via WebSocket connections, enabling live market data and account updates.
//!
//! ## Overview
//!
//! The `WsExchange` trait complements the REST-based [`Exchange`](crate::exchange::Exchange)
//! trait by providing real-time streaming capabilities:
//!
//! - **Connection Management**: Connect, disconnect, and monitor WebSocket state
//! - **Public Data Streams**: Real-time ticker, order book, trades, and OHLCV updates
//! - **Private Data Streams**: Live balance, order, and trade updates (requires authentication)
//! - **Subscription Management**: Subscribe/unsubscribe to specific channels
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                     WsExchange Trait                        │
//! ├─────────────────────────────────────────────────────────────┤
//! │  Connection Management                                      │
//! │  ├── ws_connect(), ws_disconnect()                          │
//! │  └── ws_is_connected(), ws_state()                          │
//! ├─────────────────────────────────────────────────────────────┤
//! │  Public Data Streams                                        │
//! │  ├── watch_ticker(), watch_tickers()                        │
//! │  ├── watch_order_book(), watch_trades()                     │
//! │  └── watch_ohlcv()                                          │
//! ├─────────────────────────────────────────────────────────────┤
//! │  Private Data Streams (Authenticated)                       │
//! │  ├── watch_balance()                                        │
//! │  ├── watch_orders()                                         │
//! │  └── watch_my_trades()                                      │
//! ├─────────────────────────────────────────────────────────────┤
//! │  Subscription Management                                    │
//! │  └── subscribe(), unsubscribe(), subscriptions()            │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Key Types
//!
//! - [`WsExchange`]: The WebSocket streaming trait
//! - [`MessageStream<T>`]: A pinned, boxed async stream yielding `Result<T>` items
//! - [`FullExchange`]: Combined trait for exchanges supporting both REST and WebSocket
//!
//! ## Usage Examples
//!
//! ### Watching Real-Time Ticker Updates
//!
//! ```rust,no_run
//! use ccxt_core::ws_exchange::WsExchange;
//! use futures::StreamExt;
//!
//! async fn watch_ticker(exchange: &dyn WsExchange, symbol: &str) {
//!     // Connect to WebSocket
//!     exchange.ws_connect().await.unwrap();
//!     
//!     // Watch ticker updates
//!     let mut stream = exchange.watch_ticker(symbol).await.unwrap();
//!     
//!     while let Some(ticker) = stream.next().await {
//!         match ticker {
//!             Ok(t) => println!("Price: {:?}", t.last),
//!             Err(e) => eprintln!("Error: {}", e),
//!         }
//!     }
//! }
//! ```
//!
//! ### Watching Order Book Depth
//!
//! ```rust,no_run
//! use ccxt_core::ws_exchange::WsExchange;
//! use futures::StreamExt;
//!
//! async fn watch_orderbook(exchange: &dyn WsExchange, symbol: &str) {
//!     exchange.ws_connect().await.unwrap();
//!     
//!     let mut stream = exchange.watch_order_book(symbol, Some(10)).await.unwrap();
//!     
//!     while let Some(result) = stream.next().await {
//!         if let Ok(orderbook) = result {
//!             if let (Some(best_bid), Some(best_ask)) = (
//!                 orderbook.bids.first(),
//!                 orderbook.asks.first()
//!             ) {
//!                 println!("Spread: {} - {}", best_bid.price, best_ask.price);
//!             }
//!         }
//!     }
//! }
//! ```
//!
//! ### Monitoring Connection State
//!
//! ```rust,no_run
//! use ccxt_core::ws_exchange::WsExchange;
//! use ccxt_core::ws_client::WsConnectionState;
//!
//! async fn ensure_connected(exchange: &dyn WsExchange) -> ccxt_core::Result<()> {
//!     if !exchange.ws_is_connected() {
//!         println!("Connecting to WebSocket...");
//!         exchange.ws_connect().await?;
//!     }
//!     
//!     match exchange.ws_state() {
//!         WsConnectionState::Connected => println!("Connected!"),
//!         WsConnectionState::Connecting => println!("Still connecting..."),
//!         WsConnectionState::Disconnected => println!("Disconnected"),
//!         WsConnectionState::Reconnecting => println!("Reconnecting..."),
//!         WsConnectionState::Error => println!("Error state"),
//!     }
//!     
//!     Ok(())
//! }
//! ```
//!
//! ### Using FullExchange for REST + WebSocket
//!
//! ```rust,no_run
//! use ccxt_core::ws_exchange::FullExchange;
//! use futures::StreamExt;
//!
//! async fn hybrid_trading(exchange: &dyn FullExchange, symbol: &str) {
//!     // Use REST API to get initial state
//!     let ticker = exchange.fetch_ticker(symbol).await.unwrap();
//!     println!("Initial price: {:?}", ticker.last);
//!     
//!     // Switch to WebSocket for real-time updates
//!     exchange.ws_connect().await.unwrap();
//!     let mut stream = exchange.watch_ticker(symbol).await.unwrap();
//!     
//!     while let Some(Ok(update)) = stream.next().await {
//!         println!("Live price: {:?}", update.last);
//!     }
//! }
//! ```
//!
//! ### Watching Private Account Updates
//!
//! ```rust,no_run
//! use ccxt_core::ws_exchange::WsExchange;
//! use futures::StreamExt;
//!
//! async fn watch_account(exchange: &dyn WsExchange) {
//!     exchange.ws_connect().await.unwrap();
//!     
//!     // Watch balance updates (requires authentication)
//!     let mut balance_stream = exchange.watch_balance().await.unwrap();
//!     
//!     // Watch order updates
//!     let mut order_stream = exchange.watch_orders(None).await.unwrap();
//!     
//!     tokio::select! {
//!         Some(Ok(balance)) = balance_stream.next() => {
//!             println!("Balance update: {:?}", balance);
//!         }
//!         Some(Ok(order)) = order_stream.next() => {
//!             println!("Order update: {} - {:?}", order.id, order.status);
//!         }
//!     }
//! }
//! ```
//!
//! ## MessageStream Type
//!
//! The [`MessageStream<T>`] type alias represents an async stream of results:
//!
//! ```rust,ignore
//! pub type MessageStream<T> = Pin<Box<dyn Stream<Item = Result<T>> + Send>>;
//! ```
//!
//! This type is:
//! - **Pinned**: Required for async iteration
//! - **Boxed**: Allows for dynamic dispatch and type erasure
//! - **Send**: Can be sent across thread boundaries
//! - **Yields Results**: Each item is `Result<T>` to handle stream errors
//!
//! ## Connection States
//!
//! WebSocket connections can be in one of several states:
//!
//! - `Disconnected`: Not connected to the server
//! - `Connecting`: Connection in progress
//! - `Connected`: Active connection ready for use
//! - `Reconnecting`: Automatic reconnection in progress
//!
//! ## Error Handling
//!
//! WebSocket operations can fail with:
//!
//! - `WebSocket`: Connection or protocol errors
//! - `Authentication`: Invalid credentials for private streams
//! - `Network`: Network connectivity issues
//! - `Timeout`: Connection or subscription timeout
//!
//! ## Thread Safety
//!
//! Like the `Exchange` trait, `WsExchange` requires `Send + Sync` bounds,
//! ensuring compatibility with async runtimes and multi-threaded applications.
//!
//! ## See Also
//!
//! - [`crate::exchange::Exchange`]: REST API trait
//! - [`crate::ws_client::WsClient`]: Low-level WebSocket client
//! - [`crate::ws_client::WsConnectionState`]: Connection state enum

use async_trait::async_trait;
use futures::Stream;
use std::pin::Pin;

use crate::error::Result;
use crate::exchange::Exchange;
use crate::types::*;
use crate::ws_client::WsConnectionState;

// ============================================================================
// Type Aliases
// ============================================================================

/// WebSocket message stream type
///
/// A pinned, boxed stream that yields `Result<T>` items.
/// This type is used for all WebSocket data streams.
pub type MessageStream<T> = Pin<Box<dyn Stream<Item = Result<T>> + Send>>;

// ============================================================================
// WsExchange Trait
// ============================================================================

/// WebSocket exchange trait for real-time data streaming
///
/// This trait defines the WebSocket API that exchanges can implement
/// for real-time market data and account updates.
///
/// # Thread Safety
///
/// All implementations must be `Send + Sync` to allow safe usage across
/// thread boundaries.
///
/// # Example
///
/// ```rust,no_run
/// use ccxt_core::ws_exchange::WsExchange;
///
/// async fn check_connection(exchange: &dyn WsExchange) {
///     if !exchange.ws_is_connected() {
///         exchange.ws_connect().await.unwrap();
///     }
///     println!("Connection state: {:?}", exchange.ws_state());
/// }
/// ```
#[async_trait]
pub trait WsExchange: Send + Sync {
    // ==================== Connection Management ====================

    /// Connect to the WebSocket server
    ///
    /// Establishes a WebSocket connection to the exchange.
    /// If already connected, this may be a no-op or reconnect.
    ///
    /// # Errors
    ///
    /// Returns an error if the connection fails.
    async fn ws_connect(&self) -> Result<()>;

    /// Disconnect from the WebSocket server
    ///
    /// Closes the WebSocket connection gracefully.
    ///
    /// # Errors
    ///
    /// Returns an error if disconnection fails.
    async fn ws_disconnect(&self) -> Result<()>;

    /// Check if WebSocket is connected
    ///
    /// # Returns
    ///
    /// True if the WebSocket connection is active.
    fn ws_is_connected(&self) -> bool;

    /// Get WebSocket connection state
    ///
    /// # Returns
    ///
    /// The current connection state.
    fn ws_state(&self) -> WsConnectionState;

    // ==================== Public Data Streams ====================

    /// Watch ticker updates for a symbol
    ///
    /// # Arguments
    ///
    /// * `symbol` - Trading pair symbol (e.g., "BTC/USDT")
    ///
    /// # Returns
    ///
    /// A stream of ticker updates.
    ///
    /// # Errors
    ///
    /// Returns an error if subscription fails.
    async fn watch_ticker(&self, symbol: &str) -> Result<MessageStream<Ticker>>;

    /// Watch tickers for multiple symbols
    ///
    /// # Arguments
    ///
    /// * `symbols` - List of trading pair symbols
    ///
    /// # Returns
    ///
    /// A stream of ticker vectors.
    async fn watch_tickers(&self, symbols: &[String]) -> Result<MessageStream<Vec<Ticker>>>;

    /// Watch order book updates
    ///
    /// # Arguments
    ///
    /// * `symbol` - Trading pair symbol
    /// * `limit` - Optional depth limit
    ///
    /// # Returns
    ///
    /// A stream of order book updates.
    async fn watch_order_book(
        &self,
        symbol: &str,
        limit: Option<u32>,
    ) -> Result<MessageStream<OrderBook>>;

    /// Watch public trades
    ///
    /// # Arguments
    ///
    /// * `symbol` - Trading pair symbol
    ///
    /// # Returns
    ///
    /// A stream of trade vectors.
    async fn watch_trades(&self, symbol: &str) -> Result<MessageStream<Vec<Trade>>>;

    /// Watch OHLCV candlestick updates
    ///
    /// # Arguments
    ///
    /// * `symbol` - Trading pair symbol
    /// * `timeframe` - Candlestick timeframe
    ///
    /// # Returns
    ///
    /// A stream of OHLCV updates.
    async fn watch_ohlcv(&self, symbol: &str, timeframe: Timeframe)
    -> Result<MessageStream<Ohlcv>>;

    // ==================== Private Data Streams ====================

    /// Watch account balance updates
    ///
    /// Requires authentication.
    ///
    /// # Returns
    ///
    /// A stream of balance updates.
    ///
    /// # Errors
    ///
    /// Returns an error if not authenticated or subscription fails.
    async fn watch_balance(&self) -> Result<MessageStream<Balance>>;

    /// Watch order updates
    ///
    /// Requires authentication.
    ///
    /// # Arguments
    ///
    /// * `symbol` - Optional symbol to filter orders
    ///
    /// # Returns
    ///
    /// A stream of order updates.
    async fn watch_orders(&self, symbol: Option<&str>) -> Result<MessageStream<Order>>;

    /// Watch user trade updates
    ///
    /// Requires authentication.
    ///
    /// # Arguments
    ///
    /// * `symbol` - Optional symbol to filter trades
    ///
    /// # Returns
    ///
    /// A stream of trade updates.
    async fn watch_my_trades(&self, symbol: Option<&str>) -> Result<MessageStream<Trade>>;

    // ==================== Subscription Management ====================

    /// Subscribe to a channel
    ///
    /// # Arguments
    ///
    /// * `channel` - Channel name to subscribe to
    /// * `symbol` - Optional symbol for the subscription
    ///
    /// # Errors
    ///
    /// Returns an error if subscription fails.
    async fn subscribe(&self, channel: &str, symbol: Option<&str>) -> Result<()>;

    /// Unsubscribe from a channel
    ///
    /// # Arguments
    ///
    /// * `channel` - Channel name to unsubscribe from
    /// * `symbol` - Optional symbol for the subscription
    ///
    /// # Errors
    ///
    /// Returns an error if unsubscription fails.
    async fn unsubscribe(&self, channel: &str, symbol: Option<&str>) -> Result<()>;

    /// Get list of active subscriptions
    ///
    /// # Returns
    ///
    /// A vector of subscription identifiers.
    fn subscriptions(&self) -> Vec<String>;
}

// ============================================================================
// FullExchange Trait
// ============================================================================

/// Combined trait for exchanges that support both REST and WebSocket
///
/// This trait is automatically implemented for any type that implements
/// both `Exchange` and `WsExchange`.
///
/// # Example
///
/// ```rust,no_run
/// use ccxt_core::ws_exchange::FullExchange;
///
/// async fn use_full_exchange(exchange: &dyn FullExchange) {
///     // Use REST API
///     let ticker = exchange.fetch_ticker("BTC/USDT").await.unwrap();
///     
///     // Use WebSocket API
///     exchange.ws_connect().await.unwrap();
///     let stream = exchange.watch_ticker("BTC/USDT").await.unwrap();
/// }
/// ```
pub trait FullExchange: Exchange + WsExchange {}

// Blanket implementation for any type that implements both traits
impl<T: Exchange + WsExchange> FullExchange for T {}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ws_connection_state_debug() {
        let state = WsConnectionState::Disconnected;
        assert_eq!(format!("{:?}", state), "Disconnected");
    }
}
