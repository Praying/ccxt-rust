//! # CCXT Rust
//!
//! A cryptocurrency exchange trading library in Rust, providing unified API
//! access to 100+ cryptocurrency exchanges.
//!
//! ## Features
//!
//! - **Async/Await**: Built on tokio for efficient async operations
//! - **Type Safety**: Leverages Rust's type system for compile-time guarantees
//! - **Performance**: Zero-cost abstractions and optimized for speed
//! - **WebSocket Support**: Real-time market data streaming
//! - **Comprehensive**: Support for REST APIs and WebSocket connections
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! // use ccxt_rust::prelude::*;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Example usage will be available once exchanges are implemented
//!     Ok(())
//! }
//! ```

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
pub use ccxt_exchanges::{exchange::Exchange, prelude::*};

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
