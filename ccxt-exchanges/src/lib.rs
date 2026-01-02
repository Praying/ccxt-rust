//! CCXT Exchange Implementations
//!
//! This library contains concrete implementations of cryptocurrency exchanges
//! built on top of ccxt-core.
//!
//! # Supported Exchanges
//!
//! - Binance ✅
//! - Coinbase (planned)
//! - Kraken (planned)
//! - ... and more
//!
//! # Example
//!
//! ```rust,no_run
//! // use ccxt_exchanges::binance::Binance;
//!
//! # async fn example() -> Result<(), ccxt_core::Error> {
//! // let exchange = Binance::new(
//! //     Some("your_api_key".to_string()),
//! //     Some("your_secret".to_string())
//! // );
//! //
//! // let markets = exchange.fetch_markets().await?;
//! // println!("Found {} markets", markets.len());
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
// - module_name_repetitions: Common pattern in Rust libraries (e.g., BinanceError in binance module)
// - missing_errors_doc: Too verbose to document every Result-returning function
// - missing_panics_doc: Too verbose to document every potential panic
// - must_use_candidate: Not all return values need #[must_use]
//
// Practical Global Suppressions:
// - doc_markdown: Technical terms in docs don't need backticks (e.g., OHLCV, HMAC, WebSocket)
// - similar_names: Trading terminology requires similar names (bid/ask, buy/sell, base/quote)
// - uninlined_format_args: Style preference, many existing format! calls use explicit args
// - cast_*: Common in timestamp and numeric operations throughout exchange implementations
// - struct_excessive_bools: Config structs legitimately have many boolean flags
// - too_many_lines: Some complex parsing/validation functions are unavoidable
// - return_self_not_must_use: Builder pattern methods return Self without must_use
// - unreadable_literal: Timestamps are more readable without separators (1704110400000)
// - needless_pass_by_value: API design choice for consistency across exchange implementations
// - redundant_closure: Closures used for consistency in map chains and API handlers
// - collapsible_if: Separate if statements improve readability in parsing/API logic
// =============================================================================
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::doc_markdown)]
#![allow(clippy::similar_names)]
#![allow(clippy::uninlined_format_args)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_possible_wrap)]
#![allow(clippy::cast_lossless)]
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::struct_excessive_bools)]
#![allow(clippy::too_many_lines)]
#![allow(clippy::return_self_not_must_use)]
#![allow(clippy::unreadable_literal)]
#![allow(clippy::needless_pass_by_value)]
#![allow(clippy::redundant_closure)]
#![allow(clippy::collapsible_if)]

// Re-export ccxt-core
pub use ccxt_core;

/// Exchange trait module (DEPRECATED)
///
/// This module is deprecated. Use `ccxt_core::Exchange` and
/// `ccxt_core::ExchangeCapabilities` directly instead.
#[deprecated(
    since = "0.2.0",
    note = "Use `ccxt_core::Exchange` and `ccxt_core::ExchangeCapabilities` directly instead."
)]
pub mod exchange;

/// Binance exchange implementation
pub mod binance;

/// Bitget exchange implementation
pub mod bitget;

/// Bybit exchange implementation
pub mod bybit;

/// HyperLiquid exchange implementation
pub mod hyperliquid;

/// OKX exchange implementation
pub mod okx;

/// Prelude module for convenient imports
pub mod prelude {
    pub use crate::binance::{Binance, BinanceBuilder};
    pub use crate::bitget::{Bitget, BitgetBuilder};
    pub use crate::bybit::{Bybit, BybitBuilder};
    pub use crate::hyperliquid::{HyperLiquid, HyperLiquidBuilder};
    pub use crate::okx::{Okx, OkxBuilder};
    // Re-export unified Exchange trait from ccxt-core (not the deprecated local module)
    pub use ccxt_core::{ArcExchange, BoxedExchange, Exchange, ExchangeCapabilities};
    // Re-export WsExchange, FullExchange, and MessageStream from ccxt-core
    pub use ccxt_core::prelude::*;
    pub use ccxt_core::{FullExchange, MessageStream, WsExchange};
}

/// Library version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
