#![allow(clippy::disallowed_methods)]
//! Property-based tests for capability-trait consistency.
//!
//! This test module verifies that exchange capability flags match the implemented traits.
//! For any exchange that implements a specific trait (e.g., MarketData), all corresponding
//! capability flags in ExchangeCapabilities SHALL return true.
//!
//! **Feature: binance-rest-api-modularization, Property 2: Capability-Trait Consistency**
//! **Validates: Requirements 5.2**

use ccxt_core::{ExchangeConfig, capability::Capability, exchange::Exchange};
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

    /// **Feature: binance-rest-api-modularization, Property 2: Capability-Trait Consistency**
    ///
    /// *For any* exchange that declares market data capabilities, all market data
    /// capability flags in `ExchangeCapabilities` SHALL return `true`.
    ///
    /// This property ensures that capability flags accurately reflect supported features.
    ///
    /// **Validates: Requirements 5.2**
    #[test]
    fn prop_market_data_capabilities_consistency(config in arb_exchange_config()) {
        let binance = Binance::new(config).expect("Should create Binance instance");

        // Get capabilities
        let caps = Exchange::capabilities(&binance);

        // Property: All market data capabilities should be enabled for Binance
        prop_assert!(
            caps.fetch_markets(),
            "Binance should support fetch_markets capability"
        );
        prop_assert!(
            caps.fetch_ticker(),
            "Binance should support fetch_ticker capability"
        );
        prop_assert!(
            caps.fetch_tickers(),
            "Binance should support fetch_tickers capability"
        );
        prop_assert!(
            caps.fetch_order_book(),
            "Binance should support fetch_order_book capability"
        );
        prop_assert!(
            caps.fetch_trades(),
            "Binance should support fetch_trades capability"
        );
        prop_assert!(
            caps.fetch_ohlcv(),
            "Binance should support fetch_ohlcv capability"
        );
    }

    /// **Feature: binance-rest-api-modularization, Property 2: Capability-Trait Consistency**
    ///
    /// *For any* exchange that declares trading capabilities, all trading
    /// capability flags in `ExchangeCapabilities` SHALL return `true`.
    ///
    /// **Validates: Requirements 5.2**
    #[test]
    fn prop_trading_capabilities_consistency(config in arb_exchange_config()) {
        let binance = Binance::new(config).expect("Should create Binance instance");

        // Get capabilities
        let caps = Exchange::capabilities(&binance);

        // Property: All trading capabilities should be enabled for Binance
        prop_assert!(
            caps.create_order(),
            "Binance should support create_order capability"
        );
        prop_assert!(
            caps.cancel_order(),
            "Binance should support cancel_order capability"
        );
        prop_assert!(
            caps.fetch_order(),
            "Binance should support fetch_order capability"
        );
        prop_assert!(
            caps.fetch_open_orders(),
            "Binance should support fetch_open_orders capability"
        );
        prop_assert!(
            caps.fetch_closed_orders(),
            "Binance should support fetch_closed_orders capability"
        );
    }

    /// **Feature: binance-rest-api-modularization, Property 2: Capability-Trait Consistency**
    ///
    /// *For any* exchange that declares account capabilities, all account
    /// capability flags in `ExchangeCapabilities` SHALL return `true`.
    ///
    /// **Validates: Requirements 5.2**
    #[test]
    fn prop_account_capabilities_consistency(config in arb_exchange_config()) {
        let binance = Binance::new(config).expect("Should create Binance instance");

        // Get capabilities
        let caps = Exchange::capabilities(&binance);

        // Property: All account capabilities should be enabled for Binance
        prop_assert!(
            caps.fetch_balance(),
            "Binance should support fetch_balance capability"
        );
        prop_assert!(
            caps.fetch_my_trades(),
            "Binance should support fetch_my_trades capability"
        );
    }

    /// **Feature: binance-rest-api-modularization, Property 2: Capability-Trait Consistency**
    ///
    /// *For any* exchange that declares margin capabilities, all margin
    /// capability flags in `ExchangeCapabilities` SHALL return `true`.
    ///
    /// **Validates: Requirements 5.2**
    #[test]
    fn prop_margin_capabilities_consistency(config in arb_exchange_config()) {
        let binance = Binance::new(config).expect("Should create Binance instance");

        // Get capabilities
        let caps = Exchange::capabilities(&binance);

        // Property: All margin capabilities should be enabled for Binance
        prop_assert!(
            caps.fetch_positions(),
            "Binance should support fetch_positions capability"
        );
        prop_assert!(
            caps.set_leverage(),
            "Binance should support set_leverage capability"
        );
        prop_assert!(
            caps.set_margin_mode(),
            "Binance should support set_margin_mode capability"
        );
        prop_assert!(
            caps.fetch_funding_rate(),
            "Binance should support fetch_funding_rate capability"
        );
        prop_assert!(
            caps.fetch_funding_rates(),
            "Binance should support fetch_funding_rates capability"
        );
    }

    /// **Feature: binance-rest-api-modularization, Property 2: Capability-Trait Consistency**
    ///
    /// *For any* exchange that declares funding capabilities, all funding
    /// capability flags in `ExchangeCapabilities` SHALL return `true`.
    ///
    /// **Validates: Requirements 5.2**
    #[test]
    fn prop_funding_capabilities_consistency(config in arb_exchange_config()) {
        let binance = Binance::new(config).expect("Should create Binance instance");

        // Get capabilities
        let caps = Exchange::capabilities(&binance);

        // Property: All funding capabilities should be enabled for Binance
        prop_assert!(
            caps.fetch_deposit_address(),
            "Binance should support fetch_deposit_address capability"
        );
        prop_assert!(
            caps.withdraw(),
            "Binance should support withdraw capability"
        );
        prop_assert!(
            caps.fetch_deposits(),
            "Binance should support fetch_deposits capability"
        );
        prop_assert!(
            caps.fetch_withdrawals(),
            "Binance should support fetch_withdrawals capability"
        );
    }

    /// **Feature: binance-rest-api-modularization, Property 2: Capability-Trait Consistency**
    ///
    /// *For any* exchange configuration, the capabilities returned by the exchange
    /// SHALL be consistent across multiple calls.
    ///
    /// **Validates: Requirements 5.2**
    #[test]
    fn prop_capabilities_consistency(config in arb_exchange_config()) {
        let binance = Binance::new(config).expect("Should create Binance instance");

        // Property: Multiple calls to capabilities() should return identical results
        let caps1 = Exchange::capabilities(&binance);
        let caps2 = Exchange::capabilities(&binance);
        let caps3 = Exchange::capabilities(&binance);

        prop_assert_eq!(
            caps1, caps2,
            "capabilities() should return consistent results on multiple calls"
        );
        prop_assert_eq!(
            caps2, caps3,
            "capabilities() should return consistent results on multiple calls"
        );
    }

    /// **Feature: binance-rest-api-modularization, Property 2: Capability-Trait Consistency**
    ///
    /// *For any* exchange that declares all capabilities, all corresponding
    /// capability flags SHALL be enabled.
    ///
    /// **Validates: Requirements 5.2**
    #[test]
    fn prop_all_capabilities_consistency(config in arb_exchange_config()) {
        let binance = Binance::new(config).expect("Should create Binance instance");

        // Get capabilities
        let caps = Exchange::capabilities(&binance);

        // Property: All capabilities should be enabled (except those explicitly disabled)
        // Market Data
        prop_assert!(caps.fetch_markets());
        prop_assert!(caps.fetch_ticker());
        prop_assert!(caps.fetch_tickers());
        prop_assert!(caps.fetch_order_book());
        prop_assert!(caps.fetch_trades());
        prop_assert!(caps.fetch_ohlcv());

        // Trading
        prop_assert!(caps.create_order());
        prop_assert!(caps.cancel_order());
        prop_assert!(caps.fetch_order());
        prop_assert!(caps.fetch_open_orders());
        prop_assert!(caps.fetch_closed_orders());

        // Account
        prop_assert!(caps.fetch_balance());
        prop_assert!(caps.fetch_my_trades());

        // Margin
        prop_assert!(caps.fetch_positions());
        prop_assert!(caps.set_leverage());
        prop_assert!(caps.set_margin_mode());
        prop_assert!(caps.fetch_funding_rate());
        prop_assert!(caps.fetch_funding_rates());

        // Funding
        prop_assert!(caps.fetch_deposit_address());
        prop_assert!(caps.withdraw());
        prop_assert!(caps.fetch_deposits());
        prop_assert!(caps.fetch_withdrawals());

        // Property: Binance should NOT support edit_order (explicitly disabled)
        prop_assert!(
            !caps.edit_order(),
            "Binance should NOT support edit_order capability"
        );
    }

    /// **Feature: binance-rest-api-modularization, Property 2: Capability-Trait Consistency**
    ///
    /// *For any* capability that is enabled, the corresponding capability name
    /// should be in the list of supported capabilities.
    ///
    /// **Validates: Requirements 5.2**
    #[test]
    fn prop_enabled_capabilities_in_supported_list(config in arb_exchange_config()) {
        let binance = Binance::new(config).expect("Should create Binance instance");

        let caps = Exchange::capabilities(&binance);
        let supported = caps.supported_capabilities();

        // Property: All enabled capabilities should be in the supported list
        for cap in Capability::all().iter() {
            let cap_name = cap.as_ccxt_name();
            let is_enabled = caps.has(cap_name);
            let is_in_list = supported.contains(&cap_name);

            prop_assert_eq!(
                is_enabled, is_in_list,
                "Capability {} should be in supported list if enabled",
                cap_name
            );
        }
    }

    /// **Feature: binance-rest-api-modularization, Property 2: Capability-Trait Consistency**
    ///
    /// *For any* exchange, the capability count should match the number of
    /// capabilities in the supported list.
    ///
    /// **Validates: Requirements 5.2**
    #[test]
    fn prop_capability_count_matches_supported_list(config in arb_exchange_config()) {
        let binance = Binance::new(config).expect("Should create Binance instance");

        let caps = Exchange::capabilities(&binance);
        let supported = caps.supported_capabilities();
        let count = caps.count();

        prop_assert_eq!(
            count as usize, supported.len(),
            "Capability count should match supported list length"
        );
    }

    /// **Feature: binance-rest-api-modularization, Property 2: Capability-Trait Consistency**
    ///
    /// *For any* exchange, querying capabilities by name should be consistent
    /// with the capability flags.
    ///
    /// **Validates: Requirements 5.2**
    #[test]
    fn prop_capability_name_query_consistency(config in arb_exchange_config()) {
        let binance = Binance::new(config).expect("Should create Binance instance");

        let caps = Exchange::capabilities(&binance);

        // Property: has() method should be consistent with individual capability methods
        prop_assert_eq!(
            caps.has("fetchMarkets"),
            caps.fetch_markets(),
            "has('fetchMarkets') should match fetch_markets()"
        );
        prop_assert_eq!(
            caps.has("fetchTicker"),
            caps.fetch_ticker(),
            "has('fetchTicker') should match fetch_ticker()"
        );
        prop_assert_eq!(
            caps.has("createOrder"),
            caps.create_order(),
            "has('createOrder') should match create_order()"
        );
        prop_assert_eq!(
            caps.has("fetchBalance"),
            caps.fetch_balance(),
            "has('fetchBalance') should match fetch_balance()"
        );
        prop_assert_eq!(
            caps.has("fetchPositions"),
            caps.fetch_positions(),
            "has('fetchPositions') should match fetch_positions()"
        );
        prop_assert_eq!(
            caps.has("fetchDepositAddress"),
            caps.fetch_deposit_address(),
            "has('fetchDepositAddress') should match fetch_deposit_address()"
        );
    }
}

// ============================================================================
// Unit Tests
// ============================================================================

#[test]
fn test_capability_trait_consistency_market_data() {
    let config = ExchangeConfig::default();
    let binance = Binance::new(config).expect("Should create Binance instance");

    // Verify all market data capabilities are enabled
    let caps = Exchange::capabilities(&binance);
    assert!(caps.fetch_markets());
    assert!(caps.fetch_ticker());
    assert!(caps.fetch_tickers());
    assert!(caps.fetch_order_book());
    assert!(caps.fetch_trades());
    assert!(caps.fetch_ohlcv());
}

#[test]
fn test_capability_trait_consistency_trading() {
    let config = ExchangeConfig::default();
    let binance = Binance::new(config).expect("Should create Binance instance");

    // Verify all trading capabilities are enabled
    let caps = Exchange::capabilities(&binance);
    assert!(caps.create_order());
    assert!(caps.cancel_order());
    assert!(caps.fetch_order());
    assert!(caps.fetch_open_orders());
    assert!(caps.fetch_closed_orders());
}

#[test]
fn test_capability_trait_consistency_account() {
    let config = ExchangeConfig::default();
    let binance = Binance::new(config).expect("Should create Binance instance");

    // Verify all account capabilities are enabled
    let caps = Exchange::capabilities(&binance);
    assert!(caps.fetch_balance());
    assert!(caps.fetch_my_trades());
}

#[test]
fn test_capability_trait_consistency_margin() {
    let config = ExchangeConfig::default();
    let binance = Binance::new(config).expect("Should create Binance instance");

    // Verify all margin capabilities are enabled
    let caps = Exchange::capabilities(&binance);
    assert!(caps.fetch_positions());
    assert!(caps.set_leverage());
    assert!(caps.set_margin_mode());
    assert!(caps.fetch_funding_rate());
    assert!(caps.fetch_funding_rates());
}

#[test]
fn test_capability_trait_consistency_funding() {
    let config = ExchangeConfig::default();
    let binance = Binance::new(config).expect("Should create Binance instance");

    // Verify all funding capabilities are enabled
    let caps = Exchange::capabilities(&binance);
    assert!(caps.fetch_deposit_address());
    assert!(caps.withdraw());
    assert!(caps.fetch_deposits());
    assert!(caps.fetch_withdrawals());
}

#[test]
fn test_capability_trait_consistency_all_capabilities() {
    let config = ExchangeConfig::default();
    let binance = Binance::new(config).expect("Should create Binance instance");

    // Verify all capabilities are enabled (except explicitly disabled ones)
    let caps = Exchange::capabilities(&binance);

    // Market Data
    assert!(caps.fetch_markets());
    assert!(caps.fetch_ticker());
    assert!(caps.fetch_tickers());
    assert!(caps.fetch_order_book());
    assert!(caps.fetch_trades());
    assert!(caps.fetch_ohlcv());

    // Trading
    assert!(caps.create_order());
    assert!(caps.cancel_order());
    assert!(caps.fetch_order());
    assert!(caps.fetch_open_orders());
    assert!(caps.fetch_closed_orders());

    // Account
    assert!(caps.fetch_balance());
    assert!(caps.fetch_my_trades());

    // Margin
    assert!(caps.fetch_positions());
    assert!(caps.set_leverage());
    assert!(caps.set_margin_mode());
    assert!(caps.fetch_funding_rate());
    assert!(caps.fetch_funding_rates());

    // Funding
    assert!(caps.fetch_deposit_address());
    assert!(caps.withdraw());
    assert!(caps.fetch_deposits());
    assert!(caps.fetch_withdrawals());

    // Explicitly disabled
    assert!(!caps.edit_order());
}

#[test]
fn test_capability_consistency_across_calls() {
    let config = ExchangeConfig::default();
    let binance = Binance::new(config).expect("Should create Binance instance");

    // Verify capabilities are consistent across multiple calls
    let caps1 = Exchange::capabilities(&binance);
    let caps2 = Exchange::capabilities(&binance);
    let caps3 = Exchange::capabilities(&binance);

    assert_eq!(caps1, caps2);
    assert_eq!(caps2, caps3);
}

#[test]
fn test_capability_supported_list_consistency() {
    let config = ExchangeConfig::default();
    let binance = Binance::new(config).expect("Should create Binance instance");

    let caps = Exchange::capabilities(&binance);
    let supported = caps.supported_capabilities();

    // Verify all supported capabilities are actually enabled
    for cap_name in supported {
        assert!(
            caps.has(cap_name),
            "Capability {} should be enabled if in supported list",
            cap_name
        );
    }
}

#[test]
fn test_capability_count_matches_supported_list() {
    let config = ExchangeConfig::default();
    let binance = Binance::new(config).expect("Should create Binance instance");

    let caps = Exchange::capabilities(&binance);
    let supported = caps.supported_capabilities();
    let count = caps.count();

    assert_eq!(
        count as usize,
        supported.len(),
        "Capability count should match supported list length"
    );
}

#[test]
fn test_capability_name_query_consistency() {
    let config = ExchangeConfig::default();
    let binance = Binance::new(config).expect("Should create Binance instance");

    let caps = Exchange::capabilities(&binance);

    // Verify has() method is consistent with individual capability methods
    assert_eq!(caps.has("fetchMarkets"), caps.fetch_markets());
    assert_eq!(caps.has("fetchTicker"), caps.fetch_ticker());
    assert_eq!(caps.has("createOrder"), caps.create_order());
    assert_eq!(caps.has("fetchBalance"), caps.fetch_balance());
    assert_eq!(caps.has("fetchPositions"), caps.fetch_positions());
    assert_eq!(
        caps.has("fetchDepositAddress"),
        caps.fetch_deposit_address()
    );
}
