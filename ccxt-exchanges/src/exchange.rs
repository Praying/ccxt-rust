//! Exchange trait definition (DEPRECATED)
//!
//! This module is deprecated. Please use `ccxt_core::Exchange` and
//! `ccxt_core::ExchangeCapabilities` instead.
//!
//! # Migration Guide
//!
//! Replace:
//! ```rust,ignore
//! use ccxt_exchanges::exchange::{Exchange, ExchangeCapabilities};
//! ```
//!
//! With:
//! ```rust,ignore
//! use ccxt_core::{Exchange, ExchangeCapabilities};
//! ```
//!
//! The new unified `Exchange` trait in `ccxt_core` provides:
//! - A single, canonical trait definition
//! - Object-safe design for `dyn Exchange` usage
//! - Comprehensive `ExchangeCapabilities` with all features
//! - Better documentation and examples

#![allow(deprecated)]

// Re-export the unified Exchange trait and ExchangeCapabilities from ccxt-core
// for backward compatibility
#[deprecated(
    since = "0.2.0",
    note = "Use `ccxt_core::Exchange` instead. This module will be removed in a future version."
)]
pub use ccxt_core::Exchange;

#[deprecated(
    since = "0.2.0",
    note = "Use `ccxt_core::ExchangeCapabilities` instead. This module will be removed in a future version."
)]
pub use ccxt_core::ExchangeCapabilities;

// Re-export type aliases for convenience
#[deprecated(
    since = "0.2.0",
    note = "Use `ccxt_core::BoxedExchange` instead. This module will be removed in a future version."
)]
pub use ccxt_core::BoxedExchange;

#[deprecated(
    since = "0.2.0",
    note = "Use `ccxt_core::ArcExchange` instead. This module will be removed in a future version."
)]
pub use ccxt_core::ArcExchange;

#[cfg(test)]
mod tests {
    #[allow(deprecated)]
    use super::*;

    #[test]
    fn test_deprecated_capabilities_reexport() {
        // Test that the re-exported ExchangeCapabilities works
        // Note: New API uses method calls instead of field access (bitflags implementation)
        let caps = ExchangeCapabilities::all();
        assert!(caps.fetch_ticker());
        assert!(caps.create_order());
        assert!(caps.websocket());

        let public_caps = ExchangeCapabilities::public_only();
        assert!(public_caps.fetch_tickers());
        assert!(!public_caps.create_order());
        assert!(!public_caps.websocket());
    }

    #[test]
    fn test_deprecated_capabilities_has_method() {
        let caps = ExchangeCapabilities::all();
        assert!(caps.has("fetchTicker"));
        assert!(caps.has("createOrder"));
        assert!(!caps.has("unknownCapability"));
    }
}
