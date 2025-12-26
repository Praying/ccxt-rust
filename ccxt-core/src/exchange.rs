//! # Unified Exchange Trait
//!
//! This module defines the core [`Exchange`] trait that all exchange implementations must implement.
//! It provides a unified, polymorphic interface for interacting with cryptocurrency exchanges.
//!
//! ## Overview
//!
//! The `Exchange` trait is the central abstraction in CCXT-Rust. It enables:
//!
//! - **Polymorphic Exchange Usage**: Write exchange-agnostic trading code using `dyn Exchange`
//! - **Capability Discovery**: Query exchange features at runtime via [`ExchangeCapabilities`]
//! - **Type Safety**: Leverage Rust's type system for compile-time guarantees
//! - **Thread Safety**: All implementations are `Send + Sync` for async runtime compatibility
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                      Exchange Trait                         │
//! ├─────────────────────────────────────────────────────────────┤
//! │  Metadata Methods                                           │
//! │  ├── id(), name(), version(), certified()                   │
//! │  ├── capabilities(), timeframes(), rate_limit()             │
//! │  └── has_websocket()                                        │
//! ├─────────────────────────────────────────────────────────────┤
//! │  Market Data Methods (Public API)                           │
//! │  ├── fetch_markets(), load_markets()                        │
//! │  ├── fetch_ticker(), fetch_tickers()                        │
//! │  ├── fetch_order_book(), fetch_trades()                     │
//! │  └── fetch_ohlcv()                                          │
//! ├─────────────────────────────────────────────────────────────┤
//! │  Trading Methods (Private API)                              │
//! │  ├── create_order(), cancel_order(), cancel_all_orders()    │
//! │  └── fetch_order(), fetch_open_orders(), fetch_closed_orders()│
//! ├─────────────────────────────────────────────────────────────┤
//! │  Account Methods (Private API)                              │
//! │  └── fetch_balance(), fetch_my_trades()                     │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Key Types
//!
//! - [`Exchange`]: The core trait defining the unified exchange interface
//! - [`ExchangeCapabilities`]: Describes which features an exchange supports
//! - [`BoxedExchange`]: Type alias for `Box<dyn Exchange>` (owned trait object)
//! - [`ArcExchange`]: Type alias for `Arc<dyn Exchange>` (shared trait object)
//!
//! ## Usage Examples
//!
//! ### Basic Exchange Usage
//!
//! ```rust,no_run
//! use ccxt_core::exchange::{Exchange, ExchangeCapabilities};
//!
//! async fn print_exchange_info(exchange: &dyn Exchange) {
//!     println!("Exchange: {} ({})", exchange.name(), exchange.id());
//!     println!("Version: {}", exchange.version());
//!     println!("Certified: {}", exchange.certified());
//!     println!("Rate Limit: {} req/s", exchange.rate_limit());
//! }
//! ```
//!
//! ### Checking Capabilities Before Calling Methods
//!
//! ```rust,no_run
//! use ccxt_core::exchange::{Exchange, ExchangeCapabilities};
//!
//! async fn safe_fetch_ticker(
//!     exchange: &dyn Exchange,
//!     symbol: &str,
//! ) -> ccxt_core::Result<ccxt_core::Ticker> {
//!     // Always check capability before calling
//!     if !exchange.capabilities().fetch_ticker() {
//!         return Err(ccxt_core::Error::not_implemented("fetch_ticker"));
//!     }
//!     exchange.fetch_ticker(symbol).await
//! }
//! ```
//!
//! ### Using Multiple Exchanges Polymorphically
//!
//! ```rust,no_run
//! use ccxt_core::exchange::{Exchange, BoxedExchange};
//! use ccxt_core::types::Price;
//!
//! async fn fetch_best_price(
//!     exchanges: &[BoxedExchange],
//!     symbol: &str,
//! ) -> ccxt_core::Result<Price> {
//!     let mut best_price: Option<Price> = None;
//!
//!     for exchange in exchanges {
//!         if exchange.capabilities().fetch_ticker() {
//!             if let Ok(ticker) = exchange.fetch_ticker(symbol).await {
//!                 if let Some(last) = ticker.last {
//!                     best_price = Some(match best_price {
//!                         None => last,
//!                         Some(current) => if current < last { current } else { last },
//!                     });
//!                 }
//!             }
//!         }
//!     }
//!     
//!     best_price.ok_or_else(|| ccxt_core::Error::market_not_found("symbol"))
//! }
//! ```
//!
//! ### Thread-Safe Shared Exchange
//!
//! ```rust,no_run
//! use ccxt_core::exchange::{Exchange, ArcExchange};
//! use std::sync::Arc;
//!
//! async fn spawn_ticker_tasks(
//!     exchange: ArcExchange,
//!     symbols: Vec<String>,
//! ) {
//!     let handles: Vec<_> = symbols
//!         .into_iter()
//!         .map(|symbol| {
//!             let ex = Arc::clone(&exchange);
//!             tokio::spawn(async move {
//!                 ex.fetch_ticker(&symbol).await
//!             })
//!         })
//!         .collect();
//!     
//!     for handle in handles {
//!         let _ = handle.await;
//!     }
//! }
//! ```
//!
//! ## ExchangeCapabilities
//!
//! The [`ExchangeCapabilities`] struct provides runtime feature discovery using
//! efficient bitflags storage (8 bytes instead of 46+ bytes for individual booleans):
//!
//! ```rust
//! use ccxt_core::exchange::ExchangeCapabilities;
//!
//! // Create capabilities for public-only access
//! let public_caps = ExchangeCapabilities::public_only();
//! assert!(public_caps.fetch_ticker());
//! assert!(!public_caps.create_order());
//!
//! // Create capabilities with all features
//! let all_caps = ExchangeCapabilities::all();
//! assert!(all_caps.create_order());
//! assert!(all_caps.websocket());
//!
//! // Check capability by name (CCXT-style camelCase)
//! assert!(all_caps.has("fetchTicker"));
//! assert!(all_caps.has("createOrder"));
//!
//! // Use builder pattern for custom configurations
//! use ccxt_core::ExchangeCapabilitiesBuilder;
//! let custom = ExchangeCapabilitiesBuilder::new()
//!     .market_data()     // Add all market data capabilities
//!     .trading()         // Add all trading capabilities
//!     .build();
//! ```
//!
//! ## Error Handling
//!
//! All exchange methods return `Result<T>` with comprehensive error types:
//!
//! - `NotImplemented`: Method not supported by this exchange
//! - `Authentication`: API credentials missing or invalid
//! - `RateLimit`: Too many requests
//! - `Network`: Connection or timeout errors
//! - `Exchange`: Exchange-specific errors
//!
//! ## Thread Safety
//!
//! The `Exchange` trait requires `Send + Sync` bounds, ensuring:
//!
//! - Exchanges can be sent across thread boundaries (`Send`)
//! - Exchanges can be shared across threads via `Arc` (`Sync`)
//! - Compatible with Tokio and other async runtimes
//!
//! ## See Also
//!
//! - [`crate::ws_exchange::WsExchange`]: WebSocket streaming trait
//! - [`crate::ws_exchange::FullExchange`]: Combined REST + WebSocket trait
//! - [`crate::base_exchange::BaseExchange`]: Base implementation utilities

use async_trait::async_trait;
use rust_decimal::Decimal;
use std::collections::HashMap;
use std::sync::Arc;

use crate::error::Result;
use crate::types::*;

// Re-export ExchangeCapabilities and related types from the capability module
// The new implementation uses bitflags for efficient storage (8 bytes instead of 46+ bytes)
pub use crate::capability::{
    Capabilities, Capability, ExchangeCapabilities, ExchangeCapabilitiesBuilder,
};

// Re-export sub-traits for convenience
// These modular traits allow exchanges to implement only the capabilities they support
pub use crate::traits::{
    Account, ArcAccount, ArcFullExchange, ArcFunding, ArcMargin, ArcMarketData, ArcTrading,
    BoxedAccount, BoxedFullExchange, BoxedFunding, BoxedMargin, BoxedMarketData, BoxedTrading,
    FullExchange as ModularFullExchange, Funding, Margin, MarketData, PublicExchange, Trading,
};

// ============================================================================
// Exchange Trait
// ============================================================================

/// Core Exchange trait - the unified interface for all exchanges
///
/// This trait defines the standard API that all exchange implementations
/// must provide. It is designed to be object-safe for dynamic dispatch,
/// allowing exchanges to be used polymorphically via `dyn Exchange`.
///
/// # Relationship to Modular Traits
///
/// The `Exchange` trait provides a unified interface that combines functionality
/// from the modular trait hierarchy:
///
/// - [`PublicExchange`]: Metadata and capabilities (id, name, capabilities, etc.)
/// - [`MarketData`]: Public market data (fetch_markets, fetch_ticker, etc.)
/// - [`Trading`]: Order management (create_order, cancel_order, etc.)
/// - [`Account`]: Account operations (fetch_balance, fetch_my_trades)
/// - [`Margin`]: Margin/futures operations (available via separate trait)
/// - [`Funding`]: Deposit/withdrawal operations (available via separate trait)
///
/// For new implementations, consider implementing the modular traits instead,
/// which allows for more granular capability composition. Types implementing
/// all modular traits automatically satisfy the requirements for `Exchange`.
///
/// # Thread Safety
///
/// All implementations must be `Send + Sync` to allow safe usage across
/// thread boundaries.
///
/// # Backward Compatibility
///
/// This trait maintains full backward compatibility with existing code.
/// All methods from the original monolithic trait are still available.
/// Existing implementations continue to work without modification.
///
/// # Example
///
/// ```rust,no_run
/// use ccxt_core::exchange::Exchange;
///
/// async fn print_exchange_info(exchange: &dyn Exchange) {
///     println!("Exchange: {} ({})", exchange.name(), exchange.id());
///     println!("Version: {}", exchange.version());
///     println!("Certified: {}", exchange.certified());
/// }
/// ```
#[async_trait]
pub trait Exchange: Send + Sync {
    // ==================== Metadata ====================

    /// Returns the exchange identifier (e.g., "binance", "coinbase")
    ///
    /// This is a lowercase, URL-safe identifier used internally.
    fn id(&self) -> &str;

    /// Returns the human-readable exchange name (e.g., "Binance", "Coinbase")
    fn name(&self) -> &str;

    /// Returns the API version string
    fn version(&self) -> &'static str {
        "1.0.0"
    }

    /// Returns whether this exchange is CCXT certified
    ///
    /// Certified exchanges have been thoroughly tested and verified.
    fn certified(&self) -> bool {
        false
    }

    /// Returns whether this exchange supports WebSocket (pro features)
    fn has_websocket(&self) -> bool {
        self.capabilities().websocket()
    }

    /// Returns the exchange capabilities
    ///
    /// Use this to check which features are supported before calling methods.
    fn capabilities(&self) -> ExchangeCapabilities;

    /// Returns supported timeframes for OHLCV data
    fn timeframes(&self) -> Vec<Timeframe> {
        vec![
            Timeframe::M1,
            Timeframe::M5,
            Timeframe::M15,
            Timeframe::H1,
            Timeframe::H4,
            Timeframe::D1,
        ]
    }

    /// Returns the rate limit (requests per second)
    fn rate_limit(&self) -> u32 {
        10
    }

    // ==================== Market Data (Public API) ====================

    /// Fetch all available markets
    ///
    /// # Returns
    ///
    /// A vector of `Market` structs containing market definitions.
    ///
    /// # Errors
    ///
    /// Returns an error if the request fails or the exchange is unavailable.
    async fn fetch_markets(&self) -> Result<Vec<Market>>;

    /// Load markets and cache them
    ///
    /// # Arguments
    ///
    /// * `reload` - If true, force reload even if markets are cached
    ///
    /// # Returns
    ///
    /// A HashMap of markets indexed by symbol.
    async fn load_markets(&self, reload: bool) -> Result<HashMap<String, Market>>;

    /// Fetch ticker for a single symbol
    ///
    /// # Arguments
    ///
    /// * `symbol` - Trading pair symbol (e.g., "BTC/USDT")
    ///
    /// # Returns
    ///
    /// The ticker data for the specified symbol.
    async fn fetch_ticker(&self, symbol: &str) -> Result<Ticker>;

    /// Fetch tickers for multiple symbols (or all if None)
    ///
    /// # Arguments
    ///
    /// * `symbols` - Optional list of symbols to fetch. If None, fetches all.
    ///
    /// # Returns
    ///
    /// A vector of tickers.
    async fn fetch_tickers(&self, symbols: Option<&[String]>) -> Result<Vec<Ticker>>;

    /// Fetch order book for a symbol
    ///
    /// # Arguments
    ///
    /// * `symbol` - Trading pair symbol
    /// * `limit` - Optional limit on the number of orders per side
    ///
    /// # Returns
    ///
    /// The order book containing bids and asks.
    async fn fetch_order_book(&self, symbol: &str, limit: Option<u32>) -> Result<OrderBook>;

    /// Fetch recent public trades
    ///
    /// # Arguments
    ///
    /// * `symbol` - Trading pair symbol
    /// * `limit` - Optional limit on the number of trades
    ///
    /// # Returns
    ///
    /// A vector of recent trades.
    async fn fetch_trades(&self, symbol: &str, limit: Option<u32>) -> Result<Vec<Trade>>;

    /// Fetch OHLCV candlestick data
    ///
    /// # Arguments
    ///
    /// * `symbol` - Trading pair symbol
    /// * `timeframe` - Candlestick timeframe
    /// * `since` - Optional start timestamp in milliseconds
    /// * `limit` - Optional limit on the number of candles
    ///
    /// # Returns
    ///
    /// A vector of OHLCV candles.
    async fn fetch_ohlcv(
        &self,
        symbol: &str,
        timeframe: Timeframe,
        since: Option<i64>,
        limit: Option<u32>,
    ) -> Result<Vec<Ohlcv>>;

    // ==================== Trading (Private API) ====================

    /// Create a new order
    ///
    /// # Arguments
    ///
    /// * `symbol` - Trading pair symbol
    /// * `order_type` - Order type (limit, market, etc.)
    /// * `side` - Order side (buy or sell)
    /// * `amount` - Order amount
    /// * `price` - Optional price (required for limit orders)
    ///
    /// # Returns
    ///
    /// The created order.
    ///
    /// # Errors
    ///
    /// Returns an error if authentication fails or the order is invalid.
    async fn create_order(
        &self,
        symbol: &str,
        order_type: OrderType,
        side: OrderSide,
        amount: Decimal,
        price: Option<Decimal>,
    ) -> Result<Order>;

    /// Cancel an existing order
    ///
    /// # Arguments
    ///
    /// * `id` - Order ID to cancel
    /// * `symbol` - Optional symbol (required by some exchanges)
    ///
    /// # Returns
    ///
    /// The canceled order.
    async fn cancel_order(&self, id: &str, symbol: Option<&str>) -> Result<Order>;

    /// Cancel all orders (optionally for a specific symbol)
    ///
    /// # Arguments
    ///
    /// * `symbol` - Optional symbol to cancel orders for
    ///
    /// # Returns
    ///
    /// A vector of canceled orders.
    async fn cancel_all_orders(&self, symbol: Option<&str>) -> Result<Vec<Order>>;

    /// Fetch a specific order by ID
    ///
    /// # Arguments
    ///
    /// * `id` - Order ID
    /// * `symbol` - Optional symbol (required by some exchanges)
    ///
    /// # Returns
    ///
    /// The order details.
    async fn fetch_order(&self, id: &str, symbol: Option<&str>) -> Result<Order>;

    /// Fetch all open orders
    ///
    /// # Arguments
    ///
    /// * `symbol` - Optional symbol to filter by
    /// * `since` - Optional start timestamp
    /// * `limit` - Optional limit on results
    ///
    /// # Returns
    ///
    /// A vector of open orders.
    async fn fetch_open_orders(
        &self,
        symbol: Option<&str>,
        since: Option<i64>,
        limit: Option<u32>,
    ) -> Result<Vec<Order>>;

    /// Fetch closed orders
    ///
    /// # Arguments
    ///
    /// * `symbol` - Optional symbol to filter by
    /// * `since` - Optional start timestamp
    /// * `limit` - Optional limit on results
    ///
    /// # Returns
    ///
    /// A vector of closed orders.
    async fn fetch_closed_orders(
        &self,
        symbol: Option<&str>,
        since: Option<i64>,
        limit: Option<u32>,
    ) -> Result<Vec<Order>>;

    // ==================== Account (Private API) ====================

    /// Fetch account balance
    ///
    /// # Returns
    ///
    /// The account balance containing all currencies.
    async fn fetch_balance(&self) -> Result<Balance>;

    /// Fetch user's trade history
    ///
    /// # Arguments
    ///
    /// * `symbol` - Optional symbol to filter by
    /// * `since` - Optional start timestamp
    /// * `limit` - Optional limit on results
    ///
    /// # Returns
    ///
    /// A vector of user's trades.
    async fn fetch_my_trades(
        &self,
        symbol: Option<&str>,
        since: Option<i64>,
        limit: Option<u32>,
    ) -> Result<Vec<Trade>>;

    // ==================== Helper Methods ====================

    /// Get a specific market by symbol
    ///
    /// # Arguments
    ///
    /// * `symbol` - Trading pair symbol
    ///
    /// # Returns
    ///
    /// The market definition.
    ///
    /// # Errors
    ///
    /// Returns an error if the market is not found or markets are not loaded.
    async fn market(&self, symbol: &str) -> Result<Market>;

    /// Get all cached markets
    ///
    /// # Returns
    ///
    /// A HashMap of all markets indexed by symbol.
    async fn markets(&self) -> HashMap<String, Market>;

    /// Check if a symbol is valid and active
    ///
    /// # Arguments
    ///
    /// * `symbol` - Trading pair symbol
    ///
    /// # Returns
    ///
    /// True if the symbol exists and is active.
    async fn is_symbol_active(&self, symbol: &str) -> bool {
        self.market(symbol).await.map(|m| m.active).unwrap_or(false)
    }
}

// ============================================================================
// Type Aliases
// ============================================================================

/// Type alias for a boxed Exchange trait object
///
/// Use this when you need owned, heap-allocated exchange instances.
pub type BoxedExchange = Box<dyn Exchange>;

/// Type alias for an Arc-wrapped Exchange trait object
///
/// Use this when you need shared ownership across threads.
pub type ArcExchange = Arc<dyn Exchange>;

// ============================================================================
// Exchange Extension Trait
// ============================================================================

/// Extension trait providing access to modular sub-traits from Exchange.
///
/// This trait provides helper methods to access the modular trait interfaces
/// from an Exchange implementation. It enables gradual migration from the
/// monolithic Exchange trait to the modular trait hierarchy.
///
/// # Example
///
/// ```rust,ignore
/// use ccxt_core::exchange::{Exchange, ExchangeExt};
///
/// async fn use_modular_traits(exchange: &dyn Exchange) {
///     // Access market data functionality
///     if let Some(market_data) = exchange.as_market_data() {
///         let ticker = market_data.fetch_ticker("BTC/USDT").await;
///     }
/// }
/// ```
pub trait ExchangeExt: Exchange {
    /// Check if this exchange implements the MarketData trait.
    ///
    /// Returns true if the exchange supports market data operations.
    fn supports_market_data(&self) -> bool {
        self.capabilities().fetch_markets() || self.capabilities().fetch_ticker()
    }

    /// Check if this exchange implements the Trading trait.
    ///
    /// Returns true if the exchange supports trading operations.
    fn supports_trading(&self) -> bool {
        self.capabilities().create_order()
    }

    /// Check if this exchange implements the Account trait.
    ///
    /// Returns true if the exchange supports account operations.
    fn supports_account(&self) -> bool {
        self.capabilities().fetch_balance()
    }

    /// Check if this exchange implements the Margin trait.
    ///
    /// Returns true if the exchange supports margin/futures operations.
    fn supports_margin(&self) -> bool {
        self.capabilities().fetch_positions()
    }

    /// Check if this exchange implements the Funding trait.
    ///
    /// Returns true if the exchange supports funding operations.
    fn supports_funding(&self) -> bool {
        self.capabilities().withdraw()
    }
}

/// Blanket implementation of ExchangeExt for all Exchange implementations.
impl<T: Exchange + ?Sized> ExchangeExt for T {}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_capabilities_default() {
        let caps = ExchangeCapabilities::default();
        assert!(!caps.fetch_ticker());
        assert!(!caps.create_order());
        assert!(!caps.websocket());
    }

    #[test]
    fn test_capabilities_all() {
        let caps = ExchangeCapabilities::all();
        assert!(caps.fetch_ticker());
        assert!(caps.create_order());
        assert!(caps.websocket());
        assert!(caps.fetch_ohlcv());
        assert!(caps.fetch_balance());
    }

    #[test]
    fn test_capabilities_public_only() {
        let caps = ExchangeCapabilities::public_only();
        assert!(caps.fetch_ticker());
        assert!(caps.fetch_order_book());
        assert!(caps.fetch_trades());
        assert!(!caps.create_order());
        assert!(!caps.fetch_balance());
        assert!(!caps.websocket());
    }

    #[test]
    fn test_capabilities_has() {
        let caps = ExchangeCapabilities::all();
        assert!(caps.has("fetchTicker"));
        assert!(caps.has("createOrder"));
        assert!(caps.has("websocket"));
        assert!(!caps.has("unknownCapability"));
    }

    #[test]
    fn test_capabilities_supported_list() {
        let caps = ExchangeCapabilities::public_only();
        let supported = caps.supported_capabilities();
        assert!(supported.contains(&"fetchTicker"));
        assert!(supported.contains(&"fetchOrderBook"));
        assert!(!supported.contains(&"createOrder"));
    }

    #[test]
    fn test_capabilities_equality() {
        let caps1 = ExchangeCapabilities::all();
        let caps2 = ExchangeCapabilities::all();
        assert_eq!(caps1, caps2);

        let caps3 = ExchangeCapabilities::public_only();
        assert_ne!(caps1, caps3);
    }
}

#[cfg(test)]
mod property_tests {
    use super::*;
    use crate::error::Error;
    use proptest::prelude::*;
    use std::thread;

    // ==================== Strategies ====================

    /// Strategy to generate arbitrary ExchangeCapabilities using builder API
    fn arb_capabilities() -> impl Strategy<Value = ExchangeCapabilities> {
        prop_oneof![
            Just(ExchangeCapabilities::default()),
            Just(ExchangeCapabilities::all()),
            Just(ExchangeCapabilities::public_only()),
            // Random capabilities using builder
            (
                prop::bool::ANY,
                prop::bool::ANY,
                prop::bool::ANY,
                prop::bool::ANY,
                prop::bool::ANY,
                prop::bool::ANY,
            )
                .prop_map(
                    |(
                        fetch_ticker,
                        fetch_order_book,
                        create_order,
                        websocket,
                        fetch_balance,
                        fetch_ohlcv,
                    )| {
                        let mut builder = ExchangeCapabilities::builder();
                        if fetch_ticker {
                            builder = builder.capability(Capability::FetchTicker);
                        }
                        if fetch_order_book {
                            builder = builder.capability(Capability::FetchOrderBook);
                        }
                        if create_order {
                            builder = builder.capability(Capability::CreateOrder);
                        }
                        if websocket {
                            builder = builder.capability(Capability::Websocket);
                        }
                        if fetch_balance {
                            builder = builder.capability(Capability::FetchBalance);
                        }
                        if fetch_ohlcv {
                            builder = builder.capability(Capability::FetchOhlcv);
                        }
                        builder.build()
                    }
                ),
        ]
    }

    /// Strategy to generate arbitrary error messages
    fn arb_error_message() -> impl Strategy<Value = String> {
        prop_oneof![
            Just("".to_string()),
            "[a-zA-Z0-9 .,!?-]{1,100}",
            // Unicode messages
            "\\PC{1,50}",
        ]
    }

    /// Strategy to generate arbitrary Error variants for testing error propagation
    fn arb_error() -> impl Strategy<Value = Error> {
        prop_oneof![
            // Authentication errors
            arb_error_message().prop_map(|msg| Error::authentication(msg)),
            // Invalid request errors
            arb_error_message().prop_map(|msg| Error::invalid_request(msg)),
            // Market not found errors
            arb_error_message().prop_map(|msg| Error::market_not_found(msg)),
            // Timeout errors
            arb_error_message().prop_map(|msg| Error::timeout(msg)),
            // Not implemented errors
            arb_error_message().prop_map(|msg| Error::not_implemented(msg)),
            // Network errors
            arb_error_message().prop_map(|msg| Error::network(msg)),
            // WebSocket errors
            arb_error_message().prop_map(|msg| Error::websocket(msg)),
        ]
    }

    // ==================== Property 3: Thread Safety ====================

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// **Feature: unified-exchange-trait, Property 3: Thread Safety**
        ///
        /// *For any* exchange trait object, it should be possible to send it across
        /// thread boundaries (`Send`) and share references across threads (`Sync`).
        #[test]
        fn prop_exchange_capabilities_send_sync(caps in arb_capabilities()) {
            // Compile-time assertion: ExchangeCapabilities must be Send + Sync
            fn assert_send_sync<T: Send + Sync>(_: &T) {}
            assert_send_sync(&caps);

            // Runtime verification: ExchangeCapabilities can be sent across threads
            let caps_clone = caps.clone();
            let handle = thread::spawn(move || {
                // Capabilities were successfully moved to another thread (Send)
                caps_clone.fetch_ticker()
            });
            let result = handle.join().expect("Thread should not panic");
            prop_assert_eq!(result, caps.fetch_ticker());
        }

        #[test]
        fn prop_exchange_capabilities_arc_sharing(caps in arb_capabilities()) {
            use std::sync::Arc;

            let shared_caps = Arc::new(caps.clone());

            // Spawn multiple threads that read from the shared capabilities
            let handles: Vec<_> = (0..4)
                .map(|_| {
                    let caps_ref = Arc::clone(&shared_caps);
                    thread::spawn(move || {
                        // Read various capabilities from different threads
                        (
                            caps_ref.fetch_ticker(),
                            caps_ref.create_order(),
                            caps_ref.websocket(),
                        )
                    })
                })
                .collect();

            // All threads should complete successfully with consistent values
            for handle in handles {
                let (fetch_ticker, create_order, websocket) =
                    handle.join().expect("Thread should not panic");
                prop_assert_eq!(fetch_ticker, caps.fetch_ticker());
                prop_assert_eq!(create_order, caps.create_order());
                prop_assert_eq!(websocket, caps.websocket());
            }
        }

        /// **Feature: unified-exchange-trait, Property 3: Thread Safety (BoxedExchange type alias)**
        ///
        /// Verifies that the BoxedExchange type alias (Box<dyn Exchange>) satisfies
        /// Send + Sync bounds required for async runtime usage.
        #[test]
        fn prop_boxed_exchange_type_is_send_sync(_dummy in Just(())) {
            // Compile-time assertion: BoxedExchange must be Send
            fn assert_send<T: Send>() {}
            assert_send::<BoxedExchange>();

            // Note: Box<dyn Exchange> is Send because Exchange: Send + Sync
            // This is a compile-time check that validates the trait bounds
            prop_assert!(true, "BoxedExchange type satisfies Send bound");
        }

        /// **Feature: unified-exchange-trait, Property 3: Thread Safety (ArcExchange type alias)**
        ///
        /// Verifies that the ArcExchange type alias (Arc<dyn Exchange>) satisfies
        /// Send + Sync bounds required for shared ownership across threads.
        #[test]
        fn prop_arc_exchange_type_is_send_sync(_dummy in Just(())) {
            // Compile-time assertion: ArcExchange must be Send + Sync
            fn assert_send_sync<T: Send + Sync>() {}
            assert_send_sync::<ArcExchange>();

            // This is a compile-time check that validates the trait bounds
            prop_assert!(true, "ArcExchange type satisfies Send + Sync bounds");
        }
    }

    // ==================== Property 4: Error Propagation ====================

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// **Feature: unified-exchange-trait, Property 4: Error Propagation**
        ///
        /// *For any* async method call that fails, the error should be properly
        /// propagated through the `Result` type without panicking.
        #[test]
        fn prop_error_propagation_through_result(error in arb_error()) {
            // Store the error string before moving
            let error_string = error.to_string();

            // Create a Result with the error
            let result: Result<()> = Err(error);

            // Verify error can be extracted without panicking
            prop_assert!(result.is_err());

            let extracted_error = result.unwrap_err();

            // Error should preserve its display message
            prop_assert_eq!(
                extracted_error.to_string(),
                error_string,
                "Error display should be preserved"
            );
        }

        /// **Feature: unified-exchange-trait, Property 4: Error Propagation (with context)**
        ///
        /// *For any* error with context attached, the context should be preserved
        /// and the error chain should be traversable.
        #[test]
        fn prop_error_propagation_with_context(
            base_error in arb_error(),
            context in "[a-zA-Z0-9 ]{1,50}"
        ) {
            // Add context to the error
            let error_with_context = base_error.context(context.clone());

            // The error display should contain the context
            let display = error_with_context.to_string();
            prop_assert!(
                display.contains(&context),
                "Error display '{}' should contain context '{}'",
                display,
                context
            );

            // Error should still be usable in Result
            let result: Result<()> = Err(error_with_context);
            prop_assert!(result.is_err());
        }

        /// **Feature: unified-exchange-trait, Property 4: Error Propagation (Send + Sync)**
        ///
        /// *For any* error, it should be possible to send it across thread boundaries,
        /// which is essential for async error propagation.
        #[test]
        fn prop_error_send_across_threads(error in arb_error()) {
            // Compile-time assertion: Error must be Send + Sync
            fn assert_send_sync<T: Send + Sync + 'static>(_: &T) {}
            assert_send_sync(&error);

            // Runtime verification: Error can be sent across threads
            let error_string = error.to_string();
            let handle = thread::spawn(move || {
                // Error was successfully moved to another thread (Send)
                error.to_string()
            });
            let result = handle.join().expect("Thread should not panic");
            prop_assert_eq!(result, error_string);
        }

        /// **Feature: unified-exchange-trait, Property 4: Error Propagation (Result chain)**
        ///
        /// *For any* sequence of operations that may fail, errors should propagate
        /// correctly through the ? operator pattern.
        #[test]
        fn prop_error_propagation_chain(
            error_msg in arb_error_message(),
            should_fail_first in prop::bool::ANY,
            should_fail_second in prop::bool::ANY
        ) {
            fn operation_one(fail: bool, msg: &str) -> Result<i32> {
                if fail {
                    Err(Error::invalid_request(msg.to_string()))
                } else {
                    Ok(42)
                }
            }

            fn operation_two(fail: bool, msg: &str, input: i32) -> Result<i32> {
                if fail {
                    Err(Error::invalid_request(msg.to_string()))
                } else {
                    Ok(input * 2)
                }
            }

            fn chained_operations(
                fail_first: bool,
                fail_second: bool,
                msg: &str,
            ) -> Result<i32> {
                let result = operation_one(fail_first, msg)?;
                operation_two(fail_second, msg, result)
            }

            let result = chained_operations(should_fail_first, should_fail_second, &error_msg);

            // Verify the result matches expected behavior
            if should_fail_first {
                prop_assert!(result.is_err(), "Should fail on first operation");
            } else if should_fail_second {
                prop_assert!(result.is_err(), "Should fail on second operation");
            } else {
                prop_assert!(result.is_ok(), "Should succeed when no failures");
                prop_assert_eq!(result.unwrap(), 84, "Result should be 42 * 2 = 84");
            }
        }

        /// **Feature: unified-exchange-trait, Property 4: Error Propagation (async compatibility)**
        ///
        /// *For any* error, it should be compatible with async/await patterns,
        /// meaning it can be returned from async functions.
        #[test]
        fn prop_error_async_compatible(error in arb_error()) {
            // Verify error implements required traits for async usage
            fn assert_async_compatible<T: Send + Sync + 'static + std::error::Error>(_: &T) {}
            assert_async_compatible(&error);

            // Verify error can be boxed as dyn Error (required for anyhow compatibility)
            let boxed: Box<dyn std::error::Error + Send + Sync + 'static> = Box::new(error);

            // Verify the boxed error can be sent across threads (simulating async task spawn)
            let handle = thread::spawn(move || {
                boxed.to_string()
            });
            let _ = handle.join().expect("Thread should not panic");
        }
    }
}
