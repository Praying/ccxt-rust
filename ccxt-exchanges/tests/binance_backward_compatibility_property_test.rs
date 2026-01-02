//! Property-based tests for backward compatibility.
//!
//! This test module verifies that calling methods through the legacy `Exchange` trait
//! produces the same results as calling methods through the new decomposed trait structure.
//!
//! **Feature: binance-rest-api-modularization, Property 1: Backward Compatibility - Method Behavior Equivalence**
//! **Validates: Requirements 3.1, 3.2**

use ccxt_core::{
    ExchangeConfig,
    exchange::{Exchange, ExchangeExt},
    traits::PublicExchange,
};
use ccxt_exchanges::binance::Binance;
use proptest::prelude::*;

// ============================================================================
// Strategies
// ============================================================================

/// Strategy to generate various ExchangeConfig configurations
fn arb_exchange_config() -> impl Strategy<Value = ExchangeConfig> {
    (
        prop::bool::ANY,                                                      // sandbox
        prop::option::of(any::<u64>().prop_map(|n| format!("key_{}", n))),    // api_key
        prop::option::of(any::<u64>().prop_map(|n| format!("secret_{}", n))), // secret
    )
        .prop_map(|(sandbox, api_key, secret)| ExchangeConfig {
            sandbox,
            api_key: api_key.map(ccxt_core::SecretString::new),
            secret: secret.map(ccxt_core::SecretString::new),
            ..Default::default()
        })
}

// ============================================================================
// Property Tests
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// **Feature: binance-rest-api-modularization, Property 1: Backward Compatibility - Method Behavior Equivalence**
    ///
    /// *For any* valid exchange configuration, calling metadata methods through the
    /// legacy `Exchange` trait SHALL produce identical results to calling them directly
    /// on the Binance struct.
    ///
    /// This property ensures that existing code using `dyn Exchange` continues to work
    /// without modification after the refactoring.
    ///
    /// **Validates: Requirements 3.1, 3.2**
    #[test]
    fn prop_backward_compatibility_id(config in arb_exchange_config()) {
        let binance = Binance::new(config).expect("Should create Binance instance");

        // Get trait object reference
        let exchange: &dyn Exchange = &binance;

        // Property: id() through trait object should match direct call
        prop_assert_eq!(
            exchange.id(),
            Binance::id(&binance),
            "id() should be consistent between trait object and direct call"
        );

        // Property: id() should always return "binance"
        prop_assert_eq!(
            exchange.id(),
            "binance",
            "Binance id() should always return 'binance'"
        );
    }

    /// **Feature: binance-rest-api-modularization, Property 1: Backward Compatibility - Method Behavior Equivalence**
    ///
    /// *For any* valid exchange configuration, calling name() through the legacy
    /// `Exchange` trait SHALL produce identical results to calling it directly.
    ///
    /// **Validates: Requirements 3.1, 3.2**
    #[test]
    fn prop_backward_compatibility_name(config in arb_exchange_config()) {
        let binance = Binance::new(config).expect("Should create Binance instance");

        let exchange: &dyn Exchange = &binance;

        // Property: name() through trait object should match direct call
        prop_assert_eq!(
            exchange.name(),
            Binance::name(&binance),
            "name() should be consistent between trait object and direct call"
        );

        // Property: name() should always return "Binance"
        prop_assert_eq!(
            exchange.name(),
            "Binance",
            "Binance name() should always return 'Binance'"
        );
    }

    /// **Feature: binance-rest-api-modularization, Property 1: Backward Compatibility - Method Behavior Equivalence**
    ///
    /// *For any* valid exchange configuration, calling version() through the legacy
    /// `Exchange` trait SHALL produce identical results to calling it directly.
    ///
    /// **Validates: Requirements 3.1, 3.2**
    #[test]
    fn prop_backward_compatibility_version(config in arb_exchange_config()) {
        let binance = Binance::new(config).expect("Should create Binance instance");

        let exchange: &dyn Exchange = &binance;

        // Property: version() through trait object should match direct call
        prop_assert_eq!(
            exchange.version(),
            Binance::version(&binance),
            "version() should be consistent between trait object and direct call"
        );

        // Property: version() should always return "v3"
        prop_assert_eq!(
            exchange.version(),
            "v3",
            "Binance version() should always return 'v3'"
        );
    }

    /// **Feature: binance-rest-api-modularization, Property 1: Backward Compatibility - Method Behavior Equivalence**
    ///
    /// *For any* valid exchange configuration, calling certified() through the legacy
    /// `Exchange` trait SHALL produce identical results to calling it directly.
    ///
    /// **Validates: Requirements 3.1, 3.2**
    #[test]
    fn prop_backward_compatibility_certified(config in arb_exchange_config()) {
        let binance = Binance::new(config).expect("Should create Binance instance");

        let exchange: &dyn Exchange = &binance;

        // Property: certified() through trait object should match direct call
        prop_assert_eq!(
            exchange.certified(),
            Binance::certified(&binance),
            "certified() should be consistent between trait object and direct call"
        );

        // Property: Binance should always be certified
        prop_assert!(
            exchange.certified(),
            "Binance should always be certified"
        );
    }

    /// **Feature: binance-rest-api-modularization, Property 1: Backward Compatibility - Method Behavior Equivalence**
    ///
    /// *For any* valid exchange configuration, calling rate_limit() through the legacy
    /// `Exchange` trait SHALL produce identical results to calling it directly.
    ///
    /// **Validates: Requirements 3.1, 3.2**
    #[test]
    fn prop_backward_compatibility_rate_limit(config in arb_exchange_config()) {
        let binance = Binance::new(config).expect("Should create Binance instance");

        let exchange: &dyn Exchange = &binance;

        // Property: rate_limit() through trait object should match direct call
        prop_assert_eq!(
            exchange.rate_limit(),
            Binance::rate_limit(&binance),
            "rate_limit() should be consistent between trait object and direct call"
        );

        // Property: rate_limit() should return a positive value
        prop_assert!(
            exchange.rate_limit() > 0,
            "rate_limit() should return a positive value"
        );

        // Property: Binance rate_limit should be 50
        prop_assert_eq!(
            exchange.rate_limit(),
            50,
            "Binance rate_limit() should be 50"
        );
    }

    /// **Feature: binance-rest-api-modularization, Property 1: Backward Compatibility - Method Behavior Equivalence**
    ///
    /// *For any* valid exchange configuration, calling has_websocket() through the legacy
    /// `Exchange` trait SHALL produce identical results to calling it directly.
    ///
    /// **Validates: Requirements 3.1, 3.2**
    #[test]
    fn prop_backward_compatibility_has_websocket(config in arb_exchange_config()) {
        let binance = Binance::new(config).expect("Should create Binance instance");

        let exchange: &dyn Exchange = &binance;

        // Property: has_websocket() through trait object should match direct call
        let trait_has_ws = exchange.has_websocket();
        let direct_has_ws = <Binance as Exchange>::has_websocket(&binance);

        prop_assert_eq!(
            trait_has_ws,
            direct_has_ws,
            "has_websocket() should be consistent between trait object and direct call"
        );

        // Property: Binance should support websocket
        prop_assert!(
            exchange.has_websocket(),
            "Binance should support websocket"
        );
    }

    /// **Feature: binance-rest-api-modularization, Property 1: Backward Compatibility - Method Behavior Equivalence**
    ///
    /// *For any* valid exchange configuration, calling timeframes() through the legacy
    /// `Exchange` trait SHALL produce identical results to calling it directly.
    ///
    /// **Validates: Requirements 3.1, 3.2**
    #[test]
    fn prop_backward_compatibility_timeframes(config in arb_exchange_config()) {
        let binance = Binance::new(config).expect("Should create Binance instance");

        let exchange: &dyn Exchange = &binance;

        // Property: timeframes() through trait object should match direct call
        let trait_timeframes = exchange.timeframes();
        let direct_timeframes = <Binance as Exchange>::timeframes(&binance);

        prop_assert_eq!(
            trait_timeframes.len(),
            direct_timeframes.len(),
            "timeframes() should return same number of timeframes"
        );

        // Property: timeframes should not be empty
        prop_assert!(
            !trait_timeframes.is_empty(),
            "timeframes() should not be empty"
        );

        // Property: all timeframes from trait object should be in direct call
        for tf in &trait_timeframes {
            prop_assert!(
                direct_timeframes.iter().any(|dtf| dtf == tf),
                "All timeframes from trait object should be in direct call"
            );
        }
    }

    /// **Feature: binance-rest-api-modularization, Property 1: Backward Compatibility - Method Behavior Equivalence**
    ///
    /// *For any* valid exchange configuration, calling capabilities() through the legacy
    /// `Exchange` trait SHALL produce identical results to calling it directly.
    ///
    /// **Validates: Requirements 3.1, 3.2**
    #[test]
    fn prop_backward_compatibility_capabilities(config in arb_exchange_config()) {
        let binance = Binance::new(config).expect("Should create Binance instance");

        let exchange: &dyn Exchange = &binance;

        // Property: capabilities() through trait object should match direct call
        let trait_caps = exchange.capabilities();
        let direct_caps = <Binance as Exchange>::capabilities(&binance);

        prop_assert_eq!(
            trait_caps, direct_caps,
            "capabilities() should be consistent between trait object and direct call"
        );

        // Property: Binance should support market data capabilities
        prop_assert!(
            trait_caps.fetch_markets(),
            "Binance should support fetch_markets"
        );
        prop_assert!(
            trait_caps.fetch_ticker(),
            "Binance should support fetch_ticker"
        );
        prop_assert!(
            trait_caps.fetch_order_book(),
            "Binance should support fetch_order_book"
        );
        prop_assert!(
            trait_caps.fetch_trades(),
            "Binance should support fetch_trades"
        );
        prop_assert!(
            trait_caps.fetch_ohlcv(),
            "Binance should support fetch_ohlcv"
        );

        // Property: Binance should support trading capabilities
        prop_assert!(
            trait_caps.create_order(),
            "Binance should support create_order"
        );
        prop_assert!(
            trait_caps.cancel_order(),
            "Binance should support cancel_order"
        );
        prop_assert!(
            trait_caps.fetch_order(),
            "Binance should support fetch_order"
        );

        // Property: Binance should support account capabilities
        prop_assert!(
            trait_caps.fetch_balance(),
            "Binance should support fetch_balance"
        );
        prop_assert!(
            trait_caps.fetch_my_trades(),
            "Binance should support fetch_my_trades"
        );

        // Property: Binance should NOT support edit_order
        prop_assert!(
            !trait_caps.edit_order(),
            "Binance should NOT support edit_order"
        );
    }

    /// **Feature: binance-rest-api-modularization, Property 1: Backward Compatibility - Method Behavior Equivalence**
    ///
    /// *For any* valid exchange configuration, the Exchange trait object should be
    /// usable in the same way as the direct Binance struct for all metadata operations.
    ///
    /// **Validates: Requirements 3.1, 3.2**
    #[test]
    fn prop_backward_compatibility_all_metadata(config in arb_exchange_config()) {
        let binance = Binance::new(config).expect("Should create Binance instance");

        let exchange: &dyn Exchange = &binance;

        // Property: All metadata methods should be callable and consistent
        let id = exchange.id();
        let name = exchange.name();
        let version = exchange.version();
        let certified = exchange.certified();
        let rate_limit = exchange.rate_limit();
        let has_websocket = exchange.has_websocket();
        let timeframes = exchange.timeframes();
        let capabilities = exchange.capabilities();

        // Verify all values are valid
        prop_assert!(!id.is_empty(), "id should not be empty");
        prop_assert!(!name.is_empty(), "name should not be empty");
        prop_assert!(!version.is_empty(), "version should not be empty");
        prop_assert!(rate_limit > 0, "rate_limit should be positive");
        prop_assert!(!timeframes.is_empty(), "timeframes should not be empty");

        // Verify consistency with direct calls
        prop_assert_eq!(id, <Binance as Exchange>::id(&binance));
        prop_assert_eq!(name, <Binance as Exchange>::name(&binance));
        prop_assert_eq!(version, <Binance as Exchange>::version(&binance));
        prop_assert_eq!(certified, <Binance as Exchange>::certified(&binance));
        prop_assert_eq!(rate_limit, <Binance as Exchange>::rate_limit(&binance));
        prop_assert_eq!(has_websocket, <Binance as Exchange>::has_websocket(&binance));
        prop_assert_eq!(timeframes.len(), <Binance as Exchange>::timeframes(&binance).len());
        prop_assert_eq!(capabilities, <Binance as Exchange>::capabilities(&binance));
    }

    /// **Feature: binance-rest-api-modularization, Property 1: Backward Compatibility - Method Behavior Equivalence**
    ///
    /// *For any* valid exchange configuration, the Exchange trait should support
    /// the ExchangeExt extension trait methods for capability checking.
    ///
    /// **Validates: Requirements 3.1, 3.2**
    #[test]
    fn prop_backward_compatibility_exchange_ext(config in arb_exchange_config()) {
        let binance = Binance::new(config).expect("Should create Binance instance");

        let exchange: &dyn Exchange = &binance;

        // Property: ExchangeExt methods should work through trait object
        prop_assert!(
            exchange.supports_market_data(),
            "Binance should support market data"
        );
        prop_assert!(
            exchange.supports_trading(),
            "Binance should support trading"
        );
        prop_assert!(
            exchange.supports_account(),
            "Binance should support account"
        );
        prop_assert!(
            exchange.supports_margin(),
            "Binance should support margin"
        );
        prop_assert!(
            exchange.supports_funding(),
            "Binance should support funding"
        );
    }

    /// **Feature: binance-rest-api-modularization, Property 1: Backward Compatibility - Method Behavior Equivalence**
    ///
    /// *For any* valid exchange configuration, both Exchange and PublicExchange traits
    /// should return identical metadata values.
    ///
    /// **Validates: Requirements 3.1, 3.2**
    #[test]
    fn prop_backward_compatibility_exchange_vs_public_exchange(config in arb_exchange_config()) {
        let binance = Binance::new(config).expect("Should create Binance instance");

        let exchange: &dyn Exchange = &binance;
        let public_exchange: &dyn PublicExchange = &binance;

        // Property: Both traits should return identical values
        prop_assert_eq!(
            exchange.id(),
            public_exchange.id(),
            "id() should be identical between Exchange and PublicExchange"
        );

        prop_assert_eq!(
            exchange.name(),
            public_exchange.name(),
            "name() should be identical between Exchange and PublicExchange"
        );

        prop_assert_eq!(
            exchange.version(),
            public_exchange.version(),
            "version() should be identical between Exchange and PublicExchange"
        );

        prop_assert_eq!(
            exchange.certified(),
            public_exchange.certified(),
            "certified() should be identical between Exchange and PublicExchange"
        );

        prop_assert_eq!(
            exchange.rate_limit(),
            public_exchange.rate_limit(),
            "rate_limit() should be identical between Exchange and PublicExchange"
        );

        prop_assert_eq!(
            exchange.has_websocket(),
            public_exchange.has_websocket(),
            "has_websocket() should be identical between Exchange and PublicExchange"
        );

        prop_assert_eq!(
            exchange.timeframes(),
            public_exchange.timeframes(),
            "timeframes() should be identical between Exchange and PublicExchange"
        );

        prop_assert_eq!(
            exchange.capabilities(),
            public_exchange.capabilities(),
            "capabilities() should be identical between Exchange and PublicExchange"
        );
    }
}

// ============================================================================
// Unit Tests
// ============================================================================

#[test]
fn test_backward_compatibility_metadata_consistency() {
    let config = ExchangeConfig::default();
    let binance = Binance::new(config).expect("Should create Binance instance");

    let exchange: &dyn Exchange = &binance;

    // Verify all metadata methods work through trait object
    assert_eq!(exchange.id(), "binance");
    assert_eq!(exchange.name(), "Binance");
    assert_eq!(exchange.version(), "v3");
    assert!(exchange.certified());
    assert_eq!(exchange.rate_limit(), 50);
    assert!(exchange.has_websocket());
    assert!(!exchange.timeframes().is_empty());
}

#[test]
fn test_backward_compatibility_capabilities_consistency() {
    let config = ExchangeConfig::default();
    let binance = Binance::new(config).expect("Should create Binance instance");

    let exchange: &dyn Exchange = &binance;
    let caps = exchange.capabilities();

    // Verify capabilities are consistent
    assert!(caps.fetch_markets());
    assert!(caps.fetch_ticker());
    assert!(caps.create_order());
    assert!(caps.fetch_balance());
    assert!(!caps.edit_order());
}

#[test]
fn test_backward_compatibility_trait_object_creation() {
    let config = ExchangeConfig::default();
    let binance = Binance::new(config).expect("Should create Binance instance");

    // Verify we can create a trait object
    let _exchange: Box<dyn Exchange> = Box::new(binance);
}

#[test]
fn test_backward_compatibility_exchange_ext_methods() {
    let config = ExchangeConfig::default();
    let binance = Binance::new(config).expect("Should create Binance instance");

    let exchange: &dyn Exchange = &binance;

    // Verify ExchangeExt methods work
    assert!(exchange.supports_market_data());
    assert!(exchange.supports_trading());
    assert!(exchange.supports_account());
    assert!(exchange.supports_margin());
    assert!(exchange.supports_funding());
}

#[test]
fn test_backward_compatibility_exchange_vs_public_exchange() {
    let config = ExchangeConfig::default();
    let binance = Binance::new(config).expect("Should create Binance instance");

    let exchange: &dyn Exchange = &binance;
    let public_exchange: &dyn PublicExchange = &binance;

    // Verify both traits return identical values
    assert_eq!(exchange.id(), public_exchange.id());
    assert_eq!(exchange.name(), public_exchange.name());
    assert_eq!(exchange.version(), public_exchange.version());
    assert_eq!(exchange.certified(), public_exchange.certified());
    assert_eq!(exchange.rate_limit(), public_exchange.rate_limit());
    assert_eq!(exchange.has_websocket(), public_exchange.has_websocket());
    assert_eq!(exchange.timeframes(), public_exchange.timeframes());
    assert_eq!(exchange.capabilities(), public_exchange.capabilities());
}
