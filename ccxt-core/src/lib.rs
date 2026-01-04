//! CCXT Core Library
//!
//! This is the core library for CCXT Rust implementation, providing fundamental
//! data structures, error types, and traits for cryptocurrency exchange integration.
//!
//! # Features
//!
//! - **Type Safety**: Leverages Rust's type system for compile-time guarantees
//! - **Precision**: Uses `rust_decimal::Decimal` for accurate financial calculations
//! - **Async/Await**: Built on tokio for high-performance async operations
//! - **Error Handling**: Comprehensive error types with `thiserror`
//!
//! # Example
//!
//! ```rust,no_run
//! use ccxt_core::prelude::*;
//!
//! # async fn example() -> Result<()> {
//! // Create a market
//! let market = Market::new_spot(
//!     "btc/usdt".to_string(),
//!     "BTC/USDT".to_string(),
//!     "BTC".to_string(),
//!     "USDT".to_string(),
//! );
//!
//! // Create an order
//! let order = Order::new(
//!     "12345".to_string(),
//!     "BTC/USDT".to_string(),
//!     OrderType::Limit,
//!     OrderSide::Buy,
//!     rust_decimal_macros::dec!(0.1),
//!     Some(rust_decimal_macros::dec!(50000.0)),
//!     OrderStatus::Open,
//! );
//! # Ok(())
//! # }
//! ```

#![warn(missing_docs)]
#![warn(clippy::all)]
#![warn(clippy::pedantic)]
// =============================================================================
// Global Clippy Lint Suppressions
// =============================================================================
// These lints are suppressed globally because they apply broadly across the
// codebase and would require excessive local annotations.
//
// Justified Global Suppressions (per Requirement 3.4):
// - module_name_repetitions: Common pattern in Rust libraries (e.g., OrderType in order module)
// - missing_errors_doc: Too verbose to document every Result-returning function
// - missing_panics_doc: Too verbose to document every potential panic
// - must_use_candidate: Not all return values need #[must_use]
//
// Practical Global Suppressions:
// - doc_markdown: Technical terms in docs don't need backticks (e.g., OHLCV, HMAC)
// - similar_names: Trading terminology requires similar names (bid/ask, buy/sell)
// - cast_sign_loss: Common in timestamp operations (i64 <-> u64)
// - cast_possible_wrap: Common in timestamp/numeric operations
// - struct_excessive_bools: Config structs legitimately have many boolean flags
// - too_many_lines: Some complex parsing/validation functions are unavoidable
// - return_self_not_must_use: Builder pattern methods return Self without must_use
// - unreadable_literal: Timestamps are more readable without separators (1704110400000)
// =============================================================================
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::doc_markdown)]
#![allow(clippy::similar_names)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_possible_wrap)]
#![allow(clippy::struct_excessive_bools)]
#![allow(clippy::too_many_lines)]
#![allow(clippy::return_self_not_must_use)]
#![allow(clippy::unreadable_literal)]

// Re-exports of external dependencies
pub use rust_decimal;
pub use serde;
pub use serde_json;

// Core modules
pub mod auth;
pub mod base_exchange;
pub mod capability;
pub mod config;
pub mod credentials;
pub mod error;
pub mod exchange;
pub mod http_client;
pub mod logging;
pub mod parser_utils;
pub mod precision;
pub mod rate_limiter;
pub mod retry_strategy;
pub mod symbol;
pub mod time;
/// Exchange trait hierarchy for modular capability composition
pub mod traits;
pub mod types;
pub mod ws_client;
pub mod ws_exchange;

// Test utilities (available in dev-dependencies context)
#[cfg(any(test, feature = "test-utils", debug_assertions))]
pub mod test_config;

// Note: Macros from test_config are automatically exported to crate root via #[macro_export]
// No explicit pub use required here

// Re-exports of core types for convenience
pub use base_exchange::{BaseExchange, ExchangeConfig, ExchangeConfigBuilder, MarketCache};
pub use credentials::{SecretBytes, SecretString};
// Re-export unified Exchange trait from exchange module
pub use exchange::{ArcExchange, BoxedExchange, Exchange, ExchangeExt};
// Re-export capabilities from capability module (new bitflags-based implementation)
pub use capability::{
    Capabilities, Capability, ExchangeCapabilities, ExchangeCapabilitiesBuilder, TraitCategory,
};
// Re-export WebSocket exchange trait
pub use error::{
    ContextExt, Error, ExchangeErrorDetails, NetworkError, OrderError, ParseError, Result,
};
pub use ws_exchange::{FullExchange, MessageStream, WsExchange};
// Re-export deprecated ErrorContext for backwards compatibility
#[allow(deprecated)]
pub use error::ErrorContext;
pub use types::{
    Amount, Balance, BalanceEntry, Cost, Currency, CurrencyNetwork, DefaultSubType, DefaultType,
    DefaultTypeError, EndpointType, Fee, Market, MarketLimits, MarketPrecision, MarketType, MinMax,
    Ohlcv, Order, OrderBook, OrderBookEntry, OrderBookSide, OrderSide, OrderStatus, OrderType,
    PrecisionMode, Price, TakerOrMaker, Ticker, TickerParams, TickerParamsBuilder, Timeframe,
    Trade, TradingLimits, resolve_market_type,
};
// Re-export symbol types for unified symbol format
pub use symbol::{SymbolError, SymbolFormatter, SymbolParser};
pub use types::symbol::{ContractType, ExpiryDate, ParsedSymbol, SymbolMarketType};
pub use ws_client::{
    BackoffConfig, BackoffStrategy, DEFAULT_MAX_SUBSCRIPTIONS, DEFAULT_SHUTDOWN_TIMEOUT,
    Subscription, SubscriptionManager, WsClient, WsConfig, WsConnectionState, WsError, WsErrorKind,
    WsEvent, WsMessage, WsStats, WsStatsSnapshot,
};
// Re-export CancellationToken for convenient access
pub use tokio_util::sync::CancellationToken;

/// Prelude module for convenient imports
///
/// Import everything you need with:
/// ```rust
/// use ccxt_core::prelude::*;
/// ```
pub mod prelude {
    pub use crate::auth::{
        DigestFormat, HashAlgorithm, base64_to_base64url, base64url_decode, eddsa_sign, hash,
        hmac_sign, jwt_sign,
    };
    pub use crate::base_exchange::{
        BaseExchange, ExchangeConfig, ExchangeConfigBuilder, MarketCache,
    };
    pub use crate::error::{ContextExt, Error, Result};
    // Re-export unified Exchange trait and capabilities
    pub use crate::exchange::{
        ArcExchange, BoxedExchange, Exchange, ExchangeCapabilities, ExchangeExt,
    };
    // Re-export WebSocket exchange trait
    pub use crate::ws_exchange::{FullExchange, MessageStream, WsExchange};
    // Re-export deprecated ErrorContext for backwards compatibility
    #[allow(deprecated)]
    pub use crate::error::ErrorContext;
    pub use crate::http_client::{HttpClient, HttpConfig};
    pub use crate::logging::{LogConfig, LogFormat, LogLevel, init_logging, try_init_logging};
    pub use crate::precision::{
        CountingMode, PaddingMode, RoundingMode, decimal_to_precision, number_to_string,
        precision_from_string,
    };
    pub use crate::rate_limiter::{MultiTierRateLimiter, RateLimiter, RateLimiterConfig};
    pub use crate::retry_strategy::{RetryConfig, RetryStrategy, RetryStrategyType};
    pub use crate::time::{
        iso8601, microseconds, milliseconds, parse_date, parse_iso8601, seconds, ymd, ymdhms,
        yymmdd, yyyymmdd,
    };
    pub use crate::types::{
        Amount, Balance, BalanceEntry, Currency, DefaultSubType, DefaultType, DefaultTypeError,
        EndpointType, Fee, Market, MarketLimits, MarketPrecision, MarketType, Ohlcv, Order,
        OrderBook, OrderBookEntry, OrderBookSide, OrderSide, OrderStatus, OrderType, PrecisionMode,
        Price, Symbol, TakerOrMaker, Ticker, TickerParams, TickerParamsBuilder, Timeframe,
        Timestamp, Trade, TradingLimits, resolve_market_type,
    };
    // Symbol types for unified symbol format
    pub use crate::symbol::{SymbolError, SymbolFormatter, SymbolParser};
    pub use crate::types::symbol::{ContractType, ExpiryDate, ParsedSymbol, SymbolMarketType};
    pub use crate::ws_client::{
        BackoffConfig, BackoffStrategy, DEFAULT_MAX_SUBSCRIPTIONS, DEFAULT_SHUTDOWN_TIMEOUT,
        Subscription, SubscriptionManager, WsClient, WsConfig, WsConnectionState, WsError,
        WsErrorKind, WsEvent, WsMessage, WsStats, WsStatsSnapshot,
    };
    // Re-export CancellationToken for convenient access
    pub use rust_decimal::Decimal;
    pub use serde::{Deserialize, Serialize};
    pub use tokio_util::sync::CancellationToken;
}

/// Library version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Library name
pub const NAME: &str = env!("CARGO_PKG_NAME");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        assert!(!VERSION.is_empty());
        assert_eq!(NAME, "ccxt-core");
    }
}
