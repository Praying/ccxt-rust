//! # CCXT Rust
//!
//! CCXT Rust is a professional-grade trading toolkit that brings the CCXT
//! unified exchange interface to native Rust projects. It wraps REST and
//! WebSocket APIs for major cryptocurrency exchanges under a single, strongly
//! typed abstraction.
//!
//! ## Features
//!
//! - **Async first**: Powered by `tokio` with ergonomic async/await APIs for every call
//! - **Unified types**: Shared `Exchange` trait and strongly typed market/order models
//! - **Performance oriented**: Zero-cost abstractions with `rust_decimal` for precise math
//! - **Live data**: WebSocket clients with automatic reconnection and streamed order books
//! - **Extensible**: Builder patterns and configuration hooks for custom environments
//!
//! ## Installation
//!
//! ```toml
//! [dependencies]
//! ccxt-rust = { version = "0.1", features = ["full"] }
//! ```
//!
//! Alternatively depend on the workspace members directly when developing inside the repo.
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use ccxt_exchanges::binance::Binance;
//! use ccxt_rust::prelude::*;
//!
//! #[tokio::main]
//! async fn main() -> Result<()> {
//!     // Prefer testnet when experimenting to avoid hitting live balances.
//!     let exchange = Binance::builder()
//!         .sandbox(true)
//!         .build()?;
//!
//!     if exchange.capabilities().fetch_ticker() {
//!         let ticker = exchange.fetch_ticker("BTC/USDT", ()).await?;
//!         println!("Last price: {:?}", ticker.last);
//!     }
//!
//!     Ok(())
//! }
//! ```
//!
//! ## Further Reading
//!
//! See the repository README and the `examples/` folder for advanced scenarios
//! covering authenticated calls, streaming data, and multi-exchange orchestration.

#![warn(missing_docs)]
#![warn(clippy::all)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::missing_errors_doc)]

// Re-export core types and traits
pub use ccxt_core::{
    error::{Error, Result},
    types::*,
};

// Re-export exchange implementations
pub use ccxt_core::Exchange;
pub use ccxt_exchanges::prelude::*;

// Test configuration module (only available in test builds)
#[cfg(test)]
pub mod test_config;

/// Library version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Prelude module for convenient imports
pub mod prelude {
    pub use ccxt_core::{
        error::{Error, Result},
        types::*,
    };
    pub use ccxt_exchanges::{exchange::Exchange, prelude::*};
}
