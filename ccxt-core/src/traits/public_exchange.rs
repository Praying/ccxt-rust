//! PublicExchange trait definition.
//!
//! The `PublicExchange` trait is the base trait for all exchange implementations,
//! providing metadata and capability information. All other exchange traits
//! (MarketData, Trading, Account, Margin, Funding) require this as a supertrait.
//!
//! # Object Safety
//!
//! This trait is designed to be object-safe, allowing for dynamic dispatch via
//! trait objects (`dyn PublicExchange`). All methods return concrete types or
//! references, and the trait requires `Send + Sync` bounds for async runtime
//! compatibility.
//!
//! # Example
//!
//! ```rust,ignore
//! use ccxt_core::traits::PublicExchange;
//! use ccxt_core::capability::ExchangeCapabilities;
//!
//! struct MyExchange;
//!
//! impl PublicExchange for MyExchange {
//!     fn id(&self) -> &str { "myexchange" }
//!     fn name(&self) -> &str { "My Exchange" }
//!     fn capabilities(&self) -> ExchangeCapabilities {
//!         ExchangeCapabilities::public_only()
//!     }
//!     fn timeframes(&self) -> Vec<crate::types::Timeframe> {
//!         vec![crate::types::Timeframe::H1, crate::types::Timeframe::D1]
//!     }
//! }
//! ```

use crate::capability::ExchangeCapabilities;
use crate::types::Timeframe;

/// Base trait for all exchange implementations.
///
/// Provides exchange metadata and capability information. This trait is the
/// foundation of the exchange trait hierarchy and is required by all other
/// exchange traits.
///
/// # Thread Safety
///
/// This trait requires `Send + Sync` bounds to ensure safe usage across
/// thread boundaries in async contexts.
///
/// # Object Safety
///
/// This trait is object-safe and can be used with trait objects:
///
/// ```rust,ignore
/// let exchange: Box<dyn PublicExchange> = Box::new(my_exchange);
/// println!("Exchange: {}", exchange.name());
/// ```
pub trait PublicExchange: Send + Sync {
    /// Returns the exchange identifier (e.g., "binance", "okx").
    ///
    /// This is a lowercase, unique identifier used internally.
    fn id(&self) -> &str;

    /// Returns the human-readable exchange name (e.g., "Binance", "OKX").
    fn name(&self) -> &str;

    /// Returns the API version string.
    ///
    /// Default implementation returns "1.0.0".
    fn version(&self) -> &'static str {
        "1.0.0"
    }

    /// Returns whether this exchange implementation is CCXT certified.
    ///
    /// Certified exchanges have been thoroughly tested and verified.
    /// Default implementation returns `false`.
    fn certified(&self) -> bool {
        false
    }

    /// Returns the exchange capabilities.
    ///
    /// Capabilities indicate which API methods are supported by this exchange.
    fn capabilities(&self) -> ExchangeCapabilities;

    /// Returns supported timeframes for OHLCV data.
    ///
    /// Returns a vector of timeframes that can be used with `fetch_ohlcv`.
    fn timeframes(&self) -> Vec<Timeframe>;

    /// Returns the rate limit (requests per second).
    ///
    /// Default implementation returns 10 requests per second.
    fn rate_limit(&self) -> u32 {
        10
    }

    /// Returns whether WebSocket is supported.
    ///
    /// Default implementation returns `false`.
    fn has_websocket(&self) -> bool {
        false
    }

    /// Returns whether the exchange is in sandbox/testnet mode.
    ///
    /// Default implementation returns `false`.
    fn is_sandbox(&self) -> bool {
        false
    }

    /// Returns the exchange's countries of operation.
    ///
    /// Default implementation returns an empty vector.
    fn countries(&self) -> Vec<&'static str> {
        vec![]
    }

    /// Returns the exchange's website URLs.
    ///
    /// Default implementation returns an empty vector.
    fn urls(&self) -> Vec<&'static str> {
        vec![]
    }

    /// Check if a specific capability is supported.
    ///
    /// Convenience method that delegates to `capabilities().has(name)`.
    ///
    /// # Arguments
    ///
    /// * `name` - The capability name in camelCase format (e.g., "fetchTicker")
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// if exchange.has_capability("fetchOHLCV") {
    ///     // Fetch OHLCV data
    /// }
    /// ```
    fn has_capability(&self, name: &str) -> bool {
        self.capabilities().has(name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestExchange;

    impl PublicExchange for TestExchange {
        fn id(&self) -> &str {
            "test"
        }

        fn name(&self) -> &str {
            "Test Exchange"
        }

        fn capabilities(&self) -> ExchangeCapabilities {
            ExchangeCapabilities::public_only()
        }

        fn timeframes(&self) -> Vec<Timeframe> {
            vec![Timeframe::H1, Timeframe::D1]
        }
    }

    #[test]
    fn test_public_exchange_defaults() {
        let exchange = TestExchange;

        assert_eq!(exchange.id(), "test");
        assert_eq!(exchange.name(), "Test Exchange");
        assert_eq!(exchange.version(), "1.0.0");
        assert!(!exchange.certified());
        assert_eq!(exchange.rate_limit(), 10);
        assert!(!exchange.has_websocket());
        assert!(!exchange.is_sandbox());
        assert!(exchange.countries().is_empty());
        assert!(exchange.urls().is_empty());
    }

    #[test]
    fn test_has_capability() {
        let exchange = TestExchange;

        assert!(exchange.has_capability("fetchTicker"));
        assert!(exchange.has_capability("fetchMarkets"));
        assert!(!exchange.has_capability("createOrder"));
    }

    #[test]
    fn test_timeframes() {
        let exchange = TestExchange;
        let timeframes = exchange.timeframes();

        assert_eq!(timeframes.len(), 2);
        assert!(timeframes.contains(&Timeframe::H1));
        assert!(timeframes.contains(&Timeframe::D1));
    }

    #[test]
    fn test_trait_object_safety() {
        // Verify trait is object-safe by creating a trait object
        let exchange: Box<dyn PublicExchange> = Box::new(TestExchange);
        assert_eq!(exchange.id(), "test");
        assert_eq!(exchange.name(), "Test Exchange");
    }

    #[test]
    fn test_send_sync_bounds() {
        // Verify Send + Sync bounds are satisfied
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<TestExchange>();
    }
}
