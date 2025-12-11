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
#![allow(clippy::unnested_or_patterns)]
#![allow(clippy::redundant_field_names)]
#![allow(clippy::unreadable_literal)]
#![allow(clippy::empty_line_after_doc_comments)]
#![allow(clippy::needless_continue)]
#![allow(clippy::redundant_else)]
#![allow(clippy::unnecessary_literal_bound)]
#![allow(clippy::let_and_return)]
#![allow(clippy::implicit_hasher)]
#![allow(clippy::get_first)]
#![allow(clippy::unnecessary_literal_unwrap)]
#![allow(clippy::map_flatten)]
#![allow(clippy::manual_map)]
#![allow(clippy::unnecessary_cast)]
#![allow(clippy::manual_strip)]
#![allow(clippy::unnecessary_lazy_evaluations)]
#![allow(clippy::manual_range_contains)]
#![allow(clippy::collapsible_else_if)]
#![allow(clippy::implicit_clone)]
#![allow(clippy::semicolon_if_nothing_returned)]
#![allow(clippy::single_match_else)]
#![allow(clippy::assigning_clones)]
#![allow(clippy::collapsible_match)]
#![allow(clippy::option_as_ref_deref)]
#![allow(clippy::field_reassign_with_default)]
#![allow(clippy::unused_async)]

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
