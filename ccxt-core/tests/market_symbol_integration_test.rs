//! Integration tests for Market symbol handling
//!
//! Tests Market creation with unified symbols and Market lookup by unified symbol.
//! **Validates: Requirements 8.1, 8.4**

use ccxt_core::types::Market;
use ccxt_core::types::symbol::{ContractType, ParsedSymbol, SymbolMarketType};
use rust_decimal_macros::dec;

// ============================================================================
// Market Creation with Unified Symbols Tests
// ============================================================================

/// Test that Market::new_spot creates a market with correct parsed_symbol
#[test]
fn test_market_new_spot_with_parsed_symbol() {
    let market = Market::new_spot(
        "BTCUSDT".to_string(),
        "BTC/USDT".to_string(),
        "BTC".to_string(),
        "USDT".to_string(),
    );

    // Verify basic fields
    assert_eq!(market.id, "BTCUSDT");
    assert_eq!(market.symbol, "BTC/USDT");
    assert_eq!(market.base, "BTC");
    assert_eq!(market.quote, "USDT");
    assert!(market.is_spot());

    // Verify parsed_symbol is populated
    assert!(market.parsed_symbol.is_some());
    let parsed = market
        .parsed_symbol
        .as_ref()
        .expect("parsed_symbol should be Some");
    assert_eq!(parsed.base, "BTC");
    assert_eq!(parsed.quote, "USDT");
    assert!(parsed.settle.is_none());
    assert!(parsed.expiry.is_none());
    assert_eq!(parsed.market_type(), SymbolMarketType::Spot);
}

/// Test that Market::new_futures creates a market with correct parsed_symbol
#[test]
fn test_market_new_futures_with_parsed_symbol() {
    let market = Market::new_futures(
        "BTCUSDT_PERP".to_string(),
        "BTC/USDT:USDT".to_string(),
        "BTC".to_string(),
        "USDT".to_string(),
        "USDT".to_string(),
        dec!(1.0),
    );

    // Verify basic fields
    assert_eq!(market.id, "BTCUSDT_PERP");
    assert_eq!(market.symbol, "BTC/USDT:USDT");
    assert_eq!(market.base, "BTC");
    assert_eq!(market.quote, "USDT");
    assert_eq!(market.settle, Some("USDT".to_string()));
    assert!(market.is_futures());

    // Verify parsed_symbol is populated
    assert!(market.parsed_symbol.is_some());
    let parsed = market
        .parsed_symbol
        .as_ref()
        .expect("parsed_symbol should be Some");
    assert_eq!(parsed.base, "BTC");
    assert_eq!(parsed.quote, "USDT");
    assert_eq!(parsed.settle, Some("USDT".to_string()));
    assert!(parsed.expiry.is_none());
    assert_eq!(parsed.market_type(), SymbolMarketType::Swap);
    assert_eq!(parsed.contract_type(), Some(ContractType::Linear));
}

/// Test that Market::new_swap creates a market with correct parsed_symbol
#[test]
fn test_market_new_swap_with_parsed_symbol() {
    let market = Market::new_swap(
        "BTCUSDT".to_string(),
        "BTC/USDT:USDT".to_string(),
        "BTC".to_string(),
        "USDT".to_string(),
        "USDT".to_string(),
        dec!(0.001),
    );

    // Verify basic fields
    assert_eq!(market.id, "BTCUSDT");
    assert_eq!(market.symbol, "BTC/USDT:USDT");
    assert_eq!(market.base, "BTC");
    assert_eq!(market.quote, "USDT");
    assert_eq!(market.settle, Some("USDT".to_string()));
    assert!(market.is_swap());

    // Verify parsed_symbol is populated
    assert!(market.parsed_symbol.is_some());
    let parsed = market
        .parsed_symbol
        .as_ref()
        .expect("parsed_symbol should be Some");
    assert_eq!(parsed.base, "BTC");
    assert_eq!(parsed.quote, "USDT");
    assert_eq!(parsed.settle, Some("USDT".to_string()));
    assert!(parsed.is_swap());
    assert!(parsed.is_linear());
}

/// Test inverse swap market creation
#[test]
fn test_market_new_swap_inverse_with_parsed_symbol() {
    let market = Market::new_swap(
        "BTCUSD".to_string(),
        "BTC/USD:BTC".to_string(),
        "BTC".to_string(),
        "USD".to_string(),
        "BTC".to_string(),
        dec!(100.0),
    );

    // Verify basic fields
    assert_eq!(market.symbol, "BTC/USD:BTC");
    assert_eq!(market.settle, Some("BTC".to_string()));
    assert!(market.is_swap());

    // Verify parsed_symbol detects inverse contract
    assert!(market.parsed_symbol.is_some());
    let parsed = market
        .parsed_symbol
        .as_ref()
        .expect("parsed_symbol should be Some");
    assert_eq!(parsed.contract_type(), Some(ContractType::Inverse));
    assert!(parsed.is_inverse());
}

// ============================================================================
// Market parse_symbol Method Tests
// ============================================================================

/// Test that parse_symbol method works correctly
#[test]
fn test_market_parse_symbol_method() {
    let market = ccxt_core::Market {
        symbol: "ETH/USDT".to_string(),
        base: "ETH".to_string(),
        quote: "USDT".to_string(),
        ..Default::default()
    };

    // Initially parsed_symbol should be None
    assert!(market.parsed_symbol.is_none());

    // Call parse_symbol
    let mut market = market;
    let result = market.parse_symbol();
    assert!(result);

    // Now parsed_symbol should be populated
    assert!(market.parsed_symbol.is_some());
    let parsed = market
        .parsed_symbol
        .as_ref()
        .expect("parsed_symbol should be Some");
    assert_eq!(parsed.base, "ETH");
    assert_eq!(parsed.quote, "USDT");
}

/// Test that parse_symbol returns false for invalid symbols
#[test]
fn test_market_parse_symbol_invalid() {
    let mut market = ccxt_core::Market {
        symbol: "INVALID".to_string(), // No slash separator
        ..Default::default()
    };

    let result = market.parse_symbol();
    assert!(!result);
    assert!(market.parsed_symbol.is_none());
}

// ============================================================================
// Market set_parsed_symbol Method Tests
// ============================================================================

/// Test that set_parsed_symbol works correctly
#[test]
fn test_market_set_parsed_symbol() {
    let mut market = Market::default();

    let parsed = ParsedSymbol::linear_swap("SOL", "USDT");
    market.set_parsed_symbol(parsed);

    assert!(market.parsed_symbol.is_some());
    let stored = market
        .get_parsed_symbol()
        .expect("parsed_symbol should be Some");
    assert_eq!(stored.base, "SOL");
    assert_eq!(stored.quote, "USDT");
    assert!(stored.is_swap());
}

// ============================================================================
// Market Lookup by Unified Symbol Tests
// ============================================================================

/// Test that markets can be looked up by unified symbol
#[test]
fn test_market_lookup_by_unified_symbol() {
    use std::collections::HashMap;

    // Create a collection of markets
    #[allow(clippy::useless_vec)]
    let markets = vec![
        Market::new_spot(
            "BTCUSDT".to_string(),
            "BTC/USDT".to_string(),
            "BTC".to_string(),
            "USDT".to_string(),
        ),
        Market::new_spot(
            "ETHUSDT".to_string(),
            "ETH/USDT".to_string(),
            "ETH".to_string(),
            "USDT".to_string(),
        ),
        Market::new_swap(
            "BTCUSDT_PERP".to_string(),
            "BTC/USDT:USDT".to_string(),
            "BTC".to_string(),
            "USDT".to_string(),
            "USDT".to_string(),
            dec!(0.001),
        ),
    ];

    // Build a lookup map by unified symbol
    let market_map: HashMap<String, &Market> =
        markets.iter().map(|m| (m.symbol.clone(), m)).collect();

    // Test lookups
    assert!(market_map.contains_key("BTC/USDT"));
    assert!(market_map.contains_key("ETH/USDT"));
    assert!(market_map.contains_key("BTC/USDT:USDT"));

    // Verify the looked up market has correct parsed_symbol
    let btc_spot = market_map.get("BTC/USDT").expect("BTC/USDT should exist");
    assert!(btc_spot.parsed_symbol.is_some());
    assert!(
        btc_spot
            .parsed_symbol
            .as_ref()
            .expect("parsed_symbol should be Some")
            .is_spot()
    );

    let btc_swap = market_map
        .get("BTC/USDT:USDT")
        .expect("BTC/USDT:USDT should exist");
    assert!(btc_swap.parsed_symbol.is_some());
    assert!(
        btc_swap
            .parsed_symbol
            .as_ref()
            .expect("parsed_symbol should be Some")
            .is_swap()
    );
}

/// Test that markets with different types but same base/quote are distinguishable
#[test]
fn test_market_type_distinction_by_symbol() {
    let spot = Market::new_spot(
        "BTCUSDT".to_string(),
        "BTC/USDT".to_string(),
        "BTC".to_string(),
        "USDT".to_string(),
    );

    let swap = Market::new_swap(
        "BTCUSDT_PERP".to_string(),
        "BTC/USDT:USDT".to_string(),
        "BTC".to_string(),
        "USDT".to_string(),
        "USDT".to_string(),
        dec!(0.001),
    );

    // Symbols should be different
    assert_ne!(spot.symbol, swap.symbol);

    // Both should have parsed_symbol
    assert!(spot.parsed_symbol.is_some());
    assert!(swap.parsed_symbol.is_some());

    // Market types should be correctly identified
    let spot_parsed = spot
        .parsed_symbol
        .as_ref()
        .expect("parsed_symbol should be Some");
    let swap_parsed = swap
        .parsed_symbol
        .as_ref()
        .expect("parsed_symbol should be Some");

    assert_eq!(spot_parsed.market_type(), SymbolMarketType::Spot);
    assert_eq!(swap_parsed.market_type(), SymbolMarketType::Swap);
}

// ============================================================================
// Symbol Consistency Tests
// ============================================================================

/// Test that parsed_symbol is consistent with Market fields
#[test]
fn test_parsed_symbol_consistency_with_market() {
    let market = Market::new_swap(
        "ETHUSDT".to_string(),
        "ETH/USDT:USDT".to_string(),
        "ETH".to_string(),
        "USDT".to_string(),
        "USDT".to_string(),
        dec!(0.01),
    );

    let parsed = market
        .parsed_symbol
        .as_ref()
        .expect("parsed_symbol should be Some");

    // Verify consistency between Market fields and ParsedSymbol
    assert_eq!(market.base, parsed.base);
    assert_eq!(market.quote, parsed.quote);
    assert_eq!(market.settle, parsed.settle);

    // Verify linear/inverse flags are consistent
    assert_eq!(market.linear, Some(parsed.is_linear()));
    assert_eq!(market.inverse, Some(parsed.is_inverse()));
}

/// Test that symbol string matches parsed_symbol.to_string()
#[test]
fn test_symbol_string_matches_parsed_symbol_display() {
    let market = Market::new_spot(
        "BTCUSDT".to_string(),
        "BTC/USDT".to_string(),
        "BTC".to_string(),
        "USDT".to_string(),
    );

    let parsed = market
        .parsed_symbol
        .as_ref()
        .expect("parsed_symbol should be Some");
    assert_eq!(market.symbol, parsed.to_string());
}
