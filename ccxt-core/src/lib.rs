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
// Allow common patterns that are acceptable in this codebase
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::doc_markdown)]
#![allow(clippy::similar_names)]
#![allow(clippy::uninlined_format_args)]
#![allow(clippy::should_implement_trait)]
#![allow(clippy::match_same_arms)]
#![allow(clippy::redundant_closure_for_method_calls)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_possible_wrap)]
#![allow(clippy::cast_lossless)]
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::struct_excessive_bools)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::too_many_lines)]
#![allow(clippy::wildcard_imports)]
#![allow(clippy::needless_pass_by_value)]
#![allow(clippy::unnecessary_wraps)]
#![allow(clippy::items_after_statements)]
#![allow(clippy::collapsible_if)]
#![allow(clippy::if_not_else)]
#![allow(clippy::if_same_then_else)]
#![allow(clippy::derivable_impls)]
#![allow(clippy::from_over_into)]
#![allow(clippy::map_unwrap_or)]
#![allow(clippy::unnecessary_map_or)]
#![allow(clippy::clone_on_copy)]
#![allow(clippy::explicit_iter_loop)]
#![allow(clippy::ref_option)]
#![allow(clippy::ignored_unit_patterns)]
#![allow(clippy::manual_midpoint)]
#![allow(clippy::manual_pattern_char_comparison)]
#![allow(clippy::return_self_not_must_use)]
#![allow(clippy::format_push_string)]
#![allow(clippy::redundant_closure)]
#![allow(clippy::unused_self)]
#![allow(clippy::match_wildcard_for_single_variants)]

// Re-exports of external dependencies
pub use rust_decimal;
pub use serde;
pub use serde_json;

// Core modules
pub mod auth;
pub mod base_exchange;
pub mod capability;
pub mod error;
pub mod exchange;
pub mod http_client;
pub mod logging;
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
pub use ws_client::{Subscription, WsClient, WsConfig, WsConnectionState, WsMessage};

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
    pub use crate::ws_client::{Subscription, WsClient, WsConfig, WsConnectionState, WsMessage};
    pub use rust_decimal::Decimal;
    pub use serde::{Deserialize, Serialize};
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
