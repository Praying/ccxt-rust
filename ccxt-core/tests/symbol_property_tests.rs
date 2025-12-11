//! Property-based tests for the unified symbol format.
//!
//! These tests verify the correctness properties defined in the design document
//! using the proptest framework.

use ccxt_core::symbol::SymbolParser;
use ccxt_core::types::symbol::{ContractType, ExpiryDate, ParsedSymbol, SymbolMarketType};
use proptest::prelude::*;

// ============================================================================
// Test Generators
// ============================================================================

/// Generator for valid currency codes (2-10 uppercase alphanumeric characters)
fn arb_currency() -> impl Strategy<Value = String> {
    "[A-Z0-9]{2,10}".prop_map(|s| s.to_uppercase())
}

/// Generator for valid expiry dates
/// Uses conservative day range (1-28) to avoid month-specific day validation issues
fn arb_expiry_date() -> impl Strategy<Value = ExpiryDate> {
    (20u8..=99, 1u8..=12, 1u8..=28)
        .prop_map(|(y, m, d)| ExpiryDate::new(y, m, d).expect("Generated date should be valid"))
}

/// Generator for valid spot ParsedSymbol
fn arb_spot_symbol() -> impl Strategy<Value = ParsedSymbol> {
    (arb_currency(), arb_currency()).prop_map(|(base, quote)| ParsedSymbol::spot(base, quote))
}

/// Generator for valid linear swap ParsedSymbol
fn arb_linear_swap_symbol() -> impl Strategy<Value = ParsedSymbol> {
    (arb_currency(), arb_currency())
        .prop_map(|(base, quote)| ParsedSymbol::linear_swap(base, quote))
}

/// Generator for valid inverse swap ParsedSymbol
fn arb_inverse_swap_symbol() -> impl Strategy<Value = ParsedSymbol> {
    (arb_currency(), arb_currency())
        .prop_map(|(base, quote)| ParsedSymbol::inverse_swap(base, quote))
}

/// Generator for valid futures ParsedSymbol
fn arb_futures_symbol() -> impl Strategy<Value = ParsedSymbol> {
    (arb_currency(), arb_currency(), arb_expiry_date())
        .prop_map(|(base, quote, expiry)| ParsedSymbol::futures(base, quote.clone(), quote, expiry))
}

/// Generator for any valid ParsedSymbol
fn arb_parsed_symbol() -> impl Strategy<Value = ParsedSymbol> {
    prop_oneof![
        arb_spot_symbol(),
        arb_linear_swap_symbol(),
        arb_inverse_swap_symbol(),
        arb_futures_symbol(),
    ]
}

// ============================================================================
// Property 1: Spot Symbol Format Validation
// **Feature: unified-symbol-format, Property 1: Spot Symbol Format Validation**
// **Validates: Requirements 1.1, 1.3**
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// **Feature: unified-symbol-format, Property 1: Spot Symbol Format Validation**
    /// **Validates: Requirements 1.1, 1.3**
    ///
    /// *For any* valid base and quote currency pair, creating a spot symbol
    /// SHALL produce a ParsedSymbol with:
    /// - Correct base and quote fields (normalized to uppercase)
    /// - No settle field
    /// - No expiry field
    /// - Market type is Spot
    #[test]
    fn prop_spot_symbol_format_validation(
        base in arb_currency(),
        quote in arb_currency()
    ) {
        let symbol = ParsedSymbol::spot(base.clone(), quote.clone());

        // Verify base and quote are correctly set and normalized to uppercase
        prop_assert_eq!(&symbol.base, &base.to_uppercase());
        prop_assert_eq!(&symbol.quote, &quote.to_uppercase());

        // Verify no settle field (spot symbols don't have settlement currency)
        prop_assert!(symbol.settle.is_none(), "Spot symbol should not have settle field");

        // Verify no expiry field (spot symbols don't have expiry)
        prop_assert!(symbol.expiry.is_none(), "Spot symbol should not have expiry field");

        // Verify market type is Spot
        prop_assert_eq!(symbol.market_type(), SymbolMarketType::Spot);

        // Verify helper methods
        prop_assert!(symbol.is_spot());
        prop_assert!(!symbol.is_swap());
        prop_assert!(!symbol.is_futures());
        prop_assert!(!symbol.is_derivative());

        // Verify formatted string matches expected format BASE/QUOTE
        let formatted = symbol.to_string();
        prop_assert!(
            !formatted.contains(':'),
            "Spot symbol should not contain colon: {}",
            formatted
        );
        prop_assert!(
            !formatted.contains('-'),
            "Spot symbol should not contain date suffix: {}",
            formatted
        );
        prop_assert_eq!(formatted, format!("{}/{}", base.to_uppercase(), quote.to_uppercase()));
    }

    /// **Feature: unified-symbol-format, Property 1: Spot Symbol Parsing**
    /// **Validates: Requirements 1.1, 1.3**
    ///
    /// *For any* valid base and quote currency pair:
    /// - Parsing a spot symbol string `{BASE}/{QUOTE}` SHALL produce a ParsedSymbol
    ///   with correct base and quote fields, no settle field, and no expiry field
    /// - The parser SHALL normalize currency codes to uppercase
    /// - The parser SHALL reject symbols containing colons or date suffixes
    #[test]
    fn prop_spot_symbol_parsing(
        base in arb_currency(),
        quote in arb_currency()
    ) {
        // Create spot symbol string
        let symbol_str = format!("{}/{}", base, quote);

        // Parse the symbol
        let parsed = SymbolParser::parse(&symbol_str)
            .expect("Valid spot symbol should parse successfully");

        // Verify base and quote are correctly extracted and normalized
        prop_assert_eq!(&parsed.base, &base.to_uppercase());
        prop_assert_eq!(&parsed.quote, &quote.to_uppercase());

        // Verify no settle field (spot symbols don't have settlement currency)
        prop_assert!(parsed.settle.is_none(), "Parsed spot symbol should not have settle field");

        // Verify no expiry field (spot symbols don't have expiry)
        prop_assert!(parsed.expiry.is_none(), "Parsed spot symbol should not have expiry field");

        // Verify market type is Spot
        prop_assert_eq!(parsed.market_type(), SymbolMarketType::Spot);
        prop_assert!(parsed.is_spot());
        prop_assert!(!parsed.is_derivative());
    }

    /// **Feature: unified-symbol-format, Property 1: Spot Symbol Parsing with Whitespace**
    /// **Validates: Requirements 1.1, 4.4**
    ///
    /// *For any* valid spot symbol with leading/trailing whitespace:
    /// - The parser SHALL trim whitespace and parse correctly
    #[test]
    fn prop_spot_symbol_parsing_with_whitespace(
        base in arb_currency(),
        quote in arb_currency(),
        leading_spaces in 0usize..5,
        trailing_spaces in 0usize..5
    ) {
        // Create spot symbol string with whitespace
        let leading = " ".repeat(leading_spaces);
        let trailing = " ".repeat(trailing_spaces);
        let symbol_str = format!("{}{}/{}{}", leading, base, quote, trailing);

        // Parse the symbol
        let parsed = SymbolParser::parse(&symbol_str)
            .expect("Spot symbol with whitespace should parse successfully");

        // Verify base and quote are correctly extracted
        prop_assert_eq!(&parsed.base, &base.to_uppercase());
        prop_assert_eq!(&parsed.quote, &quote.to_uppercase());
        prop_assert!(parsed.is_spot());
    }

    /// **Feature: unified-symbol-format, Property 1: Spot Symbol Parsing with Lowercase**
    /// **Validates: Requirements 1.1, 4.3**
    ///
    /// *For any* valid spot symbol in lowercase:
    /// - The parser SHALL normalize to uppercase
    #[test]
    fn prop_spot_symbol_parsing_lowercase_normalization(
        base in arb_currency(),
        quote in arb_currency()
    ) {
        // Create lowercase spot symbol string
        let symbol_str = format!("{}/{}", base.to_lowercase(), quote.to_lowercase());

        // Parse the symbol
        let parsed = SymbolParser::parse(&symbol_str)
            .expect("Lowercase spot symbol should parse successfully");

        // Verify normalization to uppercase
        prop_assert_eq!(&parsed.base, &base.to_uppercase());
        prop_assert_eq!(&parsed.quote, &quote.to_uppercase());
    }
}

#[cfg(test)]
mod spot_symbol_tests {
    use super::*;

    #[test]
    fn test_spot_symbol_basic() {
        let symbol = ParsedSymbol::spot("BTC".to_string(), "USDT".to_string());
        assert_eq!(symbol.base, "BTC");
        assert_eq!(symbol.quote, "USDT");
        assert!(symbol.settle.is_none());
        assert!(symbol.expiry.is_none());
        assert_eq!(symbol.market_type(), SymbolMarketType::Spot);
        assert_eq!(symbol.to_string(), "BTC/USDT");
    }

    #[test]
    fn test_spot_symbol_lowercase_normalization() {
        let symbol = ParsedSymbol::spot("btc".to_string(), "usdt".to_string());
        assert_eq!(symbol.base, "BTC");
        assert_eq!(symbol.quote, "USDT");
    }
}

// ============================================================================
// Property 2: Swap Symbol Format Validation
// **Feature: unified-symbol-format, Property 2: Swap Symbol Format Validation**
// **Validates: Requirements 2.1, 2.2, 2.3, 2.4**
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// **Feature: unified-symbol-format, Property 2: Swap Symbol Format Validation**
    /// **Validates: Requirements 2.1, 2.2, 2.3, 2.4**
    ///
    /// *For any* valid base, quote, and settle currency combination:
    /// - Parsing a swap symbol `{BASE}/{QUOTE}:{SETTLE}` SHALL produce a ParsedSymbol
    ///   with correct base, quote, and settle fields, and no expiry field
    /// - For linear contracts, settle equals quote
    /// - For inverse contracts, settle equals base
    #[test]
    fn prop_swap_symbol_parsing_linear(
        base in arb_currency(),
        quote in arb_currency()
    ) {
        // Create linear swap symbol string (settle == quote)
        let symbol_str = format!("{}/{}:{}", base, quote, quote);

        // Parse the symbol
        let parsed = SymbolParser::parse(&symbol_str)
            .expect("Valid linear swap symbol should parse successfully");

        // Verify base, quote, and settle are correctly extracted
        prop_assert_eq!(&parsed.base, &base.to_uppercase());
        prop_assert_eq!(&parsed.quote, &quote.to_uppercase());
        prop_assert_eq!(parsed.settle.as_ref(), Some(&quote.to_uppercase()));

        // Verify no expiry field (swap symbols don't have expiry)
        prop_assert!(parsed.expiry.is_none(), "Swap symbol should not have expiry field");

        // Verify market type is Swap
        prop_assert_eq!(parsed.market_type(), SymbolMarketType::Swap);
        prop_assert!(parsed.is_swap());
        prop_assert!(parsed.is_derivative());
        prop_assert!(!parsed.is_spot());
        prop_assert!(!parsed.is_futures());

        // Verify contract type is Linear (settle == quote)
        prop_assert!(parsed.is_linear(), "Linear swap should have linear contract type");
    }

    /// **Feature: unified-symbol-format, Property 2: Inverse Swap Symbol Parsing**
    /// **Validates: Requirements 2.2**
    ///
    /// *For any* valid base and quote currency:
    /// - Parsing an inverse swap symbol `{BASE}/{QUOTE}:{BASE}` SHALL produce
    ///   a ParsedSymbol with settle equal to base (inverse contract)
    #[test]
    fn prop_swap_symbol_parsing_inverse(
        base in arb_currency(),
        quote in arb_currency()
    ) {
        // Create inverse swap symbol string (settle == base)
        let symbol_str = format!("{}/{}:{}", base, quote, base);

        // Parse the symbol
        let parsed = SymbolParser::parse(&symbol_str)
            .expect("Valid inverse swap symbol should parse successfully");

        // Verify base, quote, and settle are correctly extracted
        prop_assert_eq!(&parsed.base, &base.to_uppercase());
        prop_assert_eq!(&parsed.quote, &quote.to_uppercase());
        prop_assert_eq!(parsed.settle.as_ref(), Some(&base.to_uppercase()));

        // Verify market type is Swap
        prop_assert_eq!(parsed.market_type(), SymbolMarketType::Swap);
        prop_assert!(parsed.is_swap());

        // Verify contract type is Inverse (settle == base)
        prop_assert!(parsed.is_inverse(), "Inverse swap should have inverse contract type");
    }

    /// **Feature: unified-symbol-format, Property 2: Swap Symbol Rejects Date Suffix**
    /// **Validates: Requirements 2.4**
    ///
    /// *For any* swap symbol with a date suffix:
    /// - The parser SHALL parse it as a futures symbol, not a swap
    #[test]
    fn prop_swap_symbol_with_date_becomes_futures(
        base in arb_currency(),
        quote in arb_currency(),
        year in 20u8..=99,
        month in 1u8..=12,
        day in 1u8..=28
    ) {
        // Create symbol string with date suffix (this is futures, not swap)
        let date_str = format!("{:02}{:02}{:02}", year, month, day);
        let symbol_str = format!("{}/{}:{}-{}", base, quote, quote, date_str);

        // Parse the symbol - should succeed as futures
        let parsed = SymbolParser::parse(&symbol_str)
            .expect("Symbol with date suffix should parse as futures");

        // Verify it's parsed as futures, not swap
        prop_assert_eq!(parsed.market_type(), SymbolMarketType::Futures);
        prop_assert!(parsed.is_futures());
        prop_assert!(!parsed.is_swap());
        prop_assert!(parsed.expiry.is_some());
    }

    /// **Feature: unified-symbol-format, Property 2: Swap Symbol Rejects Multiple Colons**
    /// **Validates: Requirements 2.3**
    ///
    /// *For any* symbol with multiple colons:
    /// - The parser SHALL return an error
    #[test]
    fn prop_swap_symbol_rejects_multiple_colons(
        base in arb_currency(),
        quote in arb_currency(),
        settle in arb_currency(),
        extra in arb_currency()
    ) {
        // Create symbol string with multiple colons
        let symbol_str = format!("{}/{}:{}:{}", base, quote, settle, extra);

        // Parse should fail
        let result = SymbolParser::parse(&symbol_str);
        prop_assert!(result.is_err(), "Symbol with multiple colons should fail to parse");
    }
}

#[cfg(test)]
mod swap_symbol_tests {
    use super::*;

    #[test]
    fn test_swap_symbol_linear_basic() {
        let parsed = SymbolParser::parse("BTC/USDT:USDT").unwrap();
        assert_eq!(parsed.base, "BTC");
        assert_eq!(parsed.quote, "USDT");
        assert_eq!(parsed.settle, Some("USDT".to_string()));
        assert!(parsed.expiry.is_none());
        assert_eq!(parsed.market_type(), SymbolMarketType::Swap);
        assert!(parsed.is_linear());
    }

    #[test]
    fn test_swap_symbol_inverse_basic() {
        let parsed = SymbolParser::parse("BTC/USD:BTC").unwrap();
        assert_eq!(parsed.base, "BTC");
        assert_eq!(parsed.quote, "USD");
        assert_eq!(parsed.settle, Some("BTC".to_string()));
        assert!(parsed.is_inverse());
    }

    #[test]
    fn test_swap_symbol_lowercase_normalization() {
        let parsed = SymbolParser::parse("eth/usdt:usdt").unwrap();
        assert_eq!(parsed.base, "ETH");
        assert_eq!(parsed.quote, "USDT");
        assert_eq!(parsed.settle, Some("USDT".to_string()));
    }
}

// ============================================================================
// Property 3: Futures Symbol Format Validation (parsing)
// **Feature: unified-symbol-format, Property 3: Futures Symbol Format Validation**
// **Validates: Requirements 3.1, 3.3, 3.4**
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// **Feature: unified-symbol-format, Property 3: Futures Symbol Parsing**
    /// **Validates: Requirements 3.1, 3.3, 3.4**
    ///
    /// *For any* valid base, quote, settle currency and valid expiry date:
    /// - Parsing a futures symbol `{BASE}/{QUOTE}:{SETTLE}-{YYMMDD}` SHALL produce
    ///   a ParsedSymbol with all fields correctly populated
    /// - The expiry date SHALL be correctly extracted in YYMMDD format
    #[test]
    fn prop_futures_symbol_parsing(
        base in arb_currency(),
        quote in arb_currency(),
        year in 20u8..=99,
        month in 1u8..=12,
        day in 1u8..=28
    ) {
        // Create futures symbol string
        let date_str = format!("{:02}{:02}{:02}", year, month, day);
        let symbol_str = format!("{}/{}:{}-{}", base, quote, quote, date_str);

        // Parse the symbol
        let parsed = SymbolParser::parse(&symbol_str)
            .expect("Valid futures symbol should parse successfully");

        // Verify base, quote, and settle are correctly extracted
        prop_assert_eq!(&parsed.base, &base.to_uppercase());
        prop_assert_eq!(&parsed.quote, &quote.to_uppercase());
        prop_assert_eq!(parsed.settle.as_ref(), Some(&quote.to_uppercase()));

        // Verify expiry is correctly extracted
        prop_assert!(parsed.expiry.is_some(), "Futures symbol should have expiry");
        let expiry = parsed.expiry.unwrap();
        prop_assert_eq!(expiry.year, year);
        prop_assert_eq!(expiry.month, month);
        prop_assert_eq!(expiry.day, day);

        // Verify market type is Futures
        prop_assert_eq!(parsed.market_type(), SymbolMarketType::Futures);
        prop_assert!(parsed.is_futures());
        prop_assert!(parsed.is_derivative());
        prop_assert!(!parsed.is_spot());
        prop_assert!(!parsed.is_swap());
    }

    /// **Feature: unified-symbol-format, Property 3: Futures Symbol Rejects Invalid Date**
    /// **Validates: Requirements 3.4**
    ///
    /// *For any* futures symbol with invalid date (month > 12 or day > 31):
    /// - The parser SHALL return an error
    #[test]
    fn prop_futures_symbol_rejects_invalid_month(
        base in arb_currency(),
        quote in arb_currency(),
        year in 20u8..=99,
        invalid_month in 13u8..=99,
        day in 1u8..=28
    ) {
        // Create futures symbol string with invalid month
        let date_str = format!("{:02}{:02}{:02}", year, invalid_month, day);
        let symbol_str = format!("{}/{}:{}-{}", base, quote, quote, date_str);

        // Parse should fail
        let result = SymbolParser::parse(&symbol_str);
        prop_assert!(result.is_err(), "Futures symbol with invalid month should fail to parse");
    }

    #[test]
    fn prop_futures_symbol_rejects_invalid_day(
        base in arb_currency(),
        quote in arb_currency(),
        year in 20u8..=99,
        month in 1u8..=12,
        invalid_day in 32u8..=99
    ) {
        // Create futures symbol string with invalid day
        let date_str = format!("{:02}{:02}{:02}", year, month, invalid_day);
        let symbol_str = format!("{}/{}:{}-{}", base, quote, quote, date_str);

        // Parse should fail
        let result = SymbolParser::parse(&symbol_str);
        prop_assert!(result.is_err(), "Futures symbol with invalid day should fail to parse");
    }

    /// **Feature: unified-symbol-format, Property 3: Futures Symbol Inverse Contract**
    /// **Validates: Requirements 3.1**
    ///
    /// *For any* valid inverse futures symbol (settle == base):
    /// - The parser SHALL correctly identify it as an inverse contract
    #[test]
    fn prop_futures_symbol_parsing_inverse(
        base in arb_currency(),
        quote in arb_currency(),
        year in 20u8..=99,
        month in 1u8..=12,
        day in 1u8..=28
    ) {
        // Create inverse futures symbol string (settle == base)
        let date_str = format!("{:02}{:02}{:02}", year, month, day);
        let symbol_str = format!("{}/{}:{}-{}", base, quote, base, date_str);

        // Parse the symbol
        let parsed = SymbolParser::parse(&symbol_str)
            .expect("Valid inverse futures symbol should parse successfully");

        // Verify settle equals base (inverse)
        prop_assert_eq!(parsed.settle.as_ref(), Some(&base.to_uppercase()));

        // Verify contract type is Inverse
        prop_assert!(parsed.is_inverse(), "Inverse futures should have inverse contract type");
        prop_assert!(parsed.is_futures());
    }
}

#[cfg(test)]
mod futures_symbol_tests {
    use super::*;

    #[test]
    fn test_futures_symbol_basic() {
        let parsed = SymbolParser::parse("BTC/USDT:USDT-241231").unwrap();
        assert_eq!(parsed.base, "BTC");
        assert_eq!(parsed.quote, "USDT");
        assert_eq!(parsed.settle, Some("USDT".to_string()));
        assert!(parsed.expiry.is_some());
        let expiry = parsed.expiry.unwrap();
        assert_eq!(expiry.year, 24);
        assert_eq!(expiry.month, 12);
        assert_eq!(expiry.day, 31);
        assert_eq!(parsed.market_type(), SymbolMarketType::Futures);
    }

    #[test]
    fn test_futures_symbol_inverse() {
        let parsed = SymbolParser::parse("BTC/USD:BTC-250315").unwrap();
        assert_eq!(parsed.settle, Some("BTC".to_string()));
        assert!(parsed.is_inverse());
    }

    #[test]
    fn test_futures_symbol_invalid_date() {
        // Invalid month
        assert!(SymbolParser::parse("BTC/USDT:USDT-241301").is_err());
        // Invalid day
        assert!(SymbolParser::parse("BTC/USDT:USDT-241232").is_err());
    }
}

// ============================================================================
// Property 4: Symbol Parsing Correctness
// **Feature: unified-symbol-format, Property 4: Symbol Parsing Correctness**
// **Validates: Requirements 4.1, 4.3, 4.4**
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// **Feature: unified-symbol-format, Property 4: Symbol Parsing Correctness**
    /// **Validates: Requirements 4.1, 4.3, 4.4**
    ///
    /// *For any* valid unified symbol string:
    /// - The parser SHALL extract all components (base, quote, settle, expiry) correctly
    /// - The parser SHALL normalize currency codes to uppercase
    /// - The parser SHALL handle leading/trailing whitespace
    #[test]
    fn prop_parsing_correctness_all_symbol_types(
        base in arb_currency(),
        quote in arb_currency(),
        symbol_type in 0u8..3,  // 0=spot, 1=swap, 2=futures
        year in 20u8..=99,
        month in 1u8..=12,
        day in 1u8..=28,
        leading_spaces in 0usize..3,
        trailing_spaces in 0usize..3
    ) {
        let leading = " ".repeat(leading_spaces);
        let trailing = " ".repeat(trailing_spaces);

        let (symbol_str, expected_settle, expected_expiry) = match symbol_type {
            0 => {
                // Spot symbol
                let s = format!("{}{}/{}{}", leading, base, quote, trailing);
                (s, None, None)
            }
            1 => {
                // Swap symbol
                let s = format!("{}{}/{}:{}{}", leading, base, quote, quote, trailing);
                (s, Some(quote.to_uppercase()), None)
            }
            _ => {
                // Futures symbol
                let date_str = format!("{:02}{:02}{:02}", year, month, day);
                let s = format!("{}{}/{}:{}-{}{}", leading, base, quote, quote, date_str, trailing);
                let expiry = ExpiryDate::new(year, month, day).unwrap();
                (s, Some(quote.to_uppercase()), Some(expiry))
            }
        };

        // Parse the symbol
        let parsed = SymbolParser::parse(&symbol_str)
            .expect("Valid symbol should parse successfully");

        // Verify base and quote are correctly extracted and normalized
        prop_assert_eq!(&parsed.base, &base.to_uppercase());
        prop_assert_eq!(&parsed.quote, &quote.to_uppercase());

        // Verify settle
        prop_assert_eq!(parsed.settle, expected_settle);

        // Verify expiry
        match (parsed.expiry, expected_expiry) {
            (Some(p), Some(e)) => {
                prop_assert_eq!(p.year, e.year);
                prop_assert_eq!(p.month, e.month);
                prop_assert_eq!(p.day, e.day);
            }
            (None, None) => {}
            _ => prop_assert!(false, "Expiry mismatch"),
        }
    }

    /// **Feature: unified-symbol-format, Property 4: Lowercase Normalization**
    /// **Validates: Requirements 4.3**
    ///
    /// *For any* symbol with lowercase currency codes:
    /// - The parser SHALL normalize all currency codes to uppercase
    #[test]
    fn prop_parsing_normalizes_lowercase(
        base in arb_currency(),
        quote in arb_currency()
    ) {
        // Create lowercase symbol
        let symbol_str = format!("{}/{}", base.to_lowercase(), quote.to_lowercase());

        let parsed = SymbolParser::parse(&symbol_str)
            .expect("Lowercase symbol should parse successfully");

        // Verify normalization to uppercase
        prop_assert_eq!(&parsed.base, &base.to_uppercase());
        prop_assert_eq!(&parsed.quote, &quote.to_uppercase());
    }

    /// **Feature: unified-symbol-format, Property 4: Mixed Case Normalization**
    /// **Validates: Requirements 4.3**
    ///
    /// *For any* symbol with mixed case currency codes:
    /// - The parser SHALL normalize all currency codes to uppercase
    #[test]
    fn prop_parsing_normalizes_mixed_case(
        base in arb_currency(),
        quote in arb_currency()
    ) {
        // Create mixed case symbol (alternating case)
        let mixed_base: String = base.chars().enumerate()
            .map(|(i, c)| if i % 2 == 0 { c.to_lowercase().next().unwrap() } else { c.to_uppercase().next().unwrap() })
            .collect();
        let mixed_quote: String = quote.chars().enumerate()
            .map(|(i, c)| if i % 2 == 0 { c.to_uppercase().next().unwrap() } else { c.to_lowercase().next().unwrap() })
            .collect();

        let symbol_str = format!("{}/{}", mixed_base, mixed_quote);

        let parsed = SymbolParser::parse(&symbol_str)
            .expect("Mixed case symbol should parse successfully");

        // Verify normalization to uppercase
        prop_assert_eq!(&parsed.base, &base.to_uppercase());
        prop_assert_eq!(&parsed.quote, &quote.to_uppercase());
    }
}

#[cfg(test)]
mod parsing_correctness_tests {
    use super::*;

    #[test]
    fn test_parsing_extracts_all_components() {
        // Spot
        let spot = SymbolParser::parse("BTC/USDT").unwrap();
        assert_eq!(spot.base, "BTC");
        assert_eq!(spot.quote, "USDT");
        assert!(spot.settle.is_none());
        assert!(spot.expiry.is_none());

        // Swap
        let swap = SymbolParser::parse("ETH/USDT:USDT").unwrap();
        assert_eq!(swap.base, "ETH");
        assert_eq!(swap.quote, "USDT");
        assert_eq!(swap.settle, Some("USDT".to_string()));
        assert!(swap.expiry.is_none());

        // Futures
        let futures = SymbolParser::parse("SOL/USDT:USDT-250315").unwrap();
        assert_eq!(futures.base, "SOL");
        assert_eq!(futures.quote, "USDT");
        assert_eq!(futures.settle, Some("USDT".to_string()));
        assert!(futures.expiry.is_some());
    }

    #[test]
    fn test_parsing_handles_whitespace() {
        let parsed = SymbolParser::parse("  BTC/USDT  ").unwrap();
        assert_eq!(parsed.base, "BTC");
        assert_eq!(parsed.quote, "USDT");
    }
}

// ============================================================================
// Property 5: Invalid Symbol Rejection
// **Feature: unified-symbol-format, Property 5: Invalid Symbol Rejection**
// **Validates: Requirements 4.2**
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// **Feature: unified-symbol-format, Property 5: Empty Symbol Rejection**
    /// **Validates: Requirements 4.2**
    ///
    /// *For any* empty or whitespace-only string:
    /// - The parser SHALL return an error
    #[test]
    fn prop_rejects_empty_symbol(
        spaces in 0usize..10
    ) {
        let symbol_str = " ".repeat(spaces);
        let result = SymbolParser::parse(&symbol_str);
        prop_assert!(result.is_err(), "Empty/whitespace symbol should be rejected");
    }

    /// **Feature: unified-symbol-format, Property 5: Missing Slash Rejection**
    /// **Validates: Requirements 4.2**
    ///
    /// *For any* symbol without a forward slash:
    /// - The parser SHALL return an error
    #[test]
    fn prop_rejects_missing_slash(
        base in arb_currency(),
        quote in arb_currency()
    ) {
        // Symbol without slash
        let symbol_str = format!("{}{}", base, quote);
        let result = SymbolParser::parse(&symbol_str);
        prop_assert!(result.is_err(), "Symbol without slash should be rejected: {}", symbol_str);
    }

    /// **Feature: unified-symbol-format, Property 5: Multiple Colons Rejection**
    /// **Validates: Requirements 4.2**
    ///
    /// *For any* symbol with multiple colons:
    /// - The parser SHALL return an error
    #[test]
    fn prop_rejects_multiple_colons(
        base in arb_currency(),
        quote in arb_currency(),
        settle1 in arb_currency(),
        settle2 in arb_currency()
    ) {
        let symbol_str = format!("{}/{}:{}:{}", base, quote, settle1, settle2);
        let result = SymbolParser::parse(&symbol_str);
        prop_assert!(result.is_err(), "Symbol with multiple colons should be rejected");
    }

    /// **Feature: unified-symbol-format, Property 5: Invalid Currency Characters Rejection**
    /// **Validates: Requirements 4.2**
    ///
    /// *For any* symbol with special characters in currency codes:
    /// - The parser SHALL return an error
    #[test]
    fn prop_rejects_special_characters_in_base(
        base in arb_currency(),
        quote in arb_currency(),
        special_char in "[!@#$%^&*()\\-+=\\[\\]{}|;:'\",.<>?/\\\\]"
    ) {
        let invalid_base = format!("{}{}", base, special_char);
        let symbol_str = format!("{}/{}", invalid_base, quote);
        let result = SymbolParser::parse(&symbol_str);
        prop_assert!(result.is_err(), "Symbol with special char in base should be rejected: {}", symbol_str);
    }

    #[test]
    fn prop_rejects_special_characters_in_quote(
        base in arb_currency(),
        quote in arb_currency(),
        special_char in "[!@#$%^&*()\\-+=\\[\\]{}|;'\",.<>?\\\\]"
    ) {
        let invalid_quote = format!("{}{}", quote, special_char);
        let symbol_str = format!("{}/{}", base, invalid_quote);
        let result = SymbolParser::parse(&symbol_str);
        prop_assert!(result.is_err(), "Symbol with special char in quote should be rejected: {}", symbol_str);
    }

    /// **Feature: unified-symbol-format, Property 5: Empty Base/Quote Rejection**
    /// **Validates: Requirements 4.2**
    ///
    /// *For any* symbol with empty base or quote:
    /// - The parser SHALL return an error
    #[test]
    fn prop_rejects_empty_base(
        quote in arb_currency()
    ) {
        let symbol_str = format!("/{}", quote);
        let result = SymbolParser::parse(&symbol_str);
        prop_assert!(result.is_err(), "Symbol with empty base should be rejected");
    }

    #[test]
    fn prop_rejects_empty_quote(
        base in arb_currency()
    ) {
        let symbol_str = format!("{}/", base);
        let result = SymbolParser::parse(&symbol_str);
        prop_assert!(result.is_err(), "Symbol with empty quote should be rejected");
    }

    /// **Feature: unified-symbol-format, Property 5: Invalid Date Rejection**
    /// **Validates: Requirements 4.2**
    ///
    /// *For any* futures symbol with invalid date:
    /// - The parser SHALL return an error
    #[test]
    fn prop_rejects_invalid_date_month_zero(
        base in arb_currency(),
        quote in arb_currency(),
        year in 20u8..=99,
        day in 1u8..=28
    ) {
        let date_str = format!("{:02}00{:02}", year, day);  // month = 00
        let symbol_str = format!("{}/{}:{}-{}", base, quote, quote, date_str);
        let result = SymbolParser::parse(&symbol_str);
        prop_assert!(result.is_err(), "Futures with month=0 should be rejected");
    }

    #[test]
    fn prop_rejects_invalid_date_day_zero(
        base in arb_currency(),
        quote in arb_currency(),
        year in 20u8..=99,
        month in 1u8..=12
    ) {
        let date_str = format!("{:02}{:02}00", year, month);  // day = 00
        let symbol_str = format!("{}/{}:{}-{}", base, quote, quote, date_str);
        let result = SymbolParser::parse(&symbol_str);
        prop_assert!(result.is_err(), "Futures with day=0 should be rejected");
    }
}

#[cfg(test)]
mod invalid_symbol_tests {
    use super::*;

    #[test]
    fn test_rejects_empty_string() {
        assert!(SymbolParser::parse("").is_err());
    }

    #[test]
    fn test_rejects_whitespace_only() {
        assert!(SymbolParser::parse("   ").is_err());
    }

    #[test]
    fn test_rejects_no_slash() {
        assert!(SymbolParser::parse("BTCUSDT").is_err());
    }

    #[test]
    fn test_rejects_multiple_colons() {
        assert!(SymbolParser::parse("BTC/USDT:USDT:EXTRA").is_err());
    }

    #[test]
    fn test_rejects_special_characters() {
        assert!(SymbolParser::parse("BTC$/USDT").is_err());
        assert!(SymbolParser::parse("BTC/USDT@").is_err());
    }

    #[test]
    fn test_rejects_empty_base() {
        assert!(SymbolParser::parse("/USDT").is_err());
    }

    #[test]
    fn test_rejects_empty_quote() {
        assert!(SymbolParser::parse("BTC/").is_err());
    }

    #[test]
    fn test_rejects_invalid_date() {
        // Month > 12
        assert!(SymbolParser::parse("BTC/USDT:USDT-241301").is_err());
        // Day > 31
        assert!(SymbolParser::parse("BTC/USDT:USDT-241232").is_err());
        // Month = 0
        assert!(SymbolParser::parse("BTC/USDT:USDT-240001").is_err());
        // Day = 0
        assert!(SymbolParser::parse("BTC/USDT:USDT-240100").is_err());
    }
}

// ============================================================================
// Property 3: Futures Symbol Format Validation (date component)
// **Feature: unified-symbol-format, Property 3: Futures Symbol Format Validation**
// **Validates: Requirements 3.3, 3.4**
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// **Feature: unified-symbol-format, Property 3: Futures Symbol Format Validation (date component)**
    /// **Validates: Requirements 3.3, 3.4**
    ///
    /// *For any* valid year, month, and day values:
    /// - ExpiryDate::new() SHALL create a valid expiry date
    /// - The date SHALL format to YYMMDD format
    /// - Parsing the formatted string SHALL produce an equivalent ExpiryDate
    #[test]
    fn prop_expiry_date_validation_and_round_trip(
        year in 0u8..=99,
        month in 1u8..=12,
        day in 1u8..=28  // Conservative range to avoid month-specific day issues
    ) {
        // Create expiry date - should succeed for valid inputs
        let expiry = ExpiryDate::new(year, month, day)
            .expect("Valid date components should create ExpiryDate");

        // Verify components are stored correctly
        prop_assert_eq!(expiry.year, year);
        prop_assert_eq!(expiry.month, month);
        prop_assert_eq!(expiry.day, day);

        // Verify YYMMDD format
        let formatted = expiry.to_string();
        prop_assert_eq!(formatted.len(), 6, "Formatted date should be 6 characters");
        let expected_format = format!("{:02}{:02}{:02}", year, month, day);
        prop_assert_eq!(
            &formatted,
            &expected_format,
            "Format should be YYMMDD"
        );

        // Verify round-trip: parse the formatted string back
        let parsed: ExpiryDate = formatted.parse()
            .expect("Formatted date should be parseable");
        prop_assert_eq!(parsed.year, year);
        prop_assert_eq!(parsed.month, month);
        prop_assert_eq!(parsed.day, day);
        prop_assert_eq!(parsed, expiry, "Round-trip should produce equivalent ExpiryDate");
    }

    /// **Feature: unified-symbol-format, Property 3: Invalid Date Rejection**
    /// **Validates: Requirements 3.4**
    ///
    /// *For any* invalid month (0 or >12) or day (0 or >31):
    /// - ExpiryDate::new() SHALL return an error
    #[test]
    fn prop_expiry_date_rejects_invalid_month(
        year in 0u8..=99,
        invalid_month in prop_oneof![Just(0u8), 13u8..=255],
        day in 1u8..=28
    ) {
        let result = ExpiryDate::new(year, invalid_month, day);
        prop_assert!(
            result.is_err(),
            "Month {} should be rejected as invalid",
            invalid_month
        );
    }

    #[test]
    fn prop_expiry_date_rejects_invalid_day(
        year in 0u8..=99,
        month in 1u8..=12,
        invalid_day in prop_oneof![Just(0u8), 32u8..=255]
    ) {
        let result = ExpiryDate::new(year, month, invalid_day);
        prop_assert!(
            result.is_err(),
            "Day {} should be rejected as invalid",
            invalid_day
        );
    }

    /// **Feature: unified-symbol-format, Property 3: Futures Symbol with Expiry**
    /// **Validates: Requirements 3.1, 3.3, 3.4**
    ///
    /// *For any* valid base, quote, settle currency and valid expiry date:
    /// - Creating a futures symbol SHALL include the expiry in YYMMDD format
    /// - The symbol SHALL have market type Futures
    #[test]
    fn prop_futures_symbol_with_expiry(
        base in arb_currency(),
        quote in arb_currency(),
        year in 20u8..=99,
        month in 1u8..=12,
        day in 1u8..=28
    ) {
        let expiry = ExpiryDate::new(year, month, day)
            .expect("Valid date should create ExpiryDate");

        let symbol = ParsedSymbol::futures(
            base.clone(),
            quote.clone(),
            quote.clone(),  // Linear futures (settle == quote)
            expiry
        );

        // Verify market type is Futures
        prop_assert_eq!(symbol.market_type(), SymbolMarketType::Futures);
        prop_assert!(symbol.is_futures());
        prop_assert!(symbol.is_derivative());

        // Verify expiry is set
        prop_assert!(symbol.expiry.is_some());
        let symbol_expiry = symbol.expiry.unwrap();
        prop_assert_eq!(symbol_expiry.year, year);
        prop_assert_eq!(symbol_expiry.month, month);
        prop_assert_eq!(symbol_expiry.day, day);

        // Verify formatted string includes date suffix
        let formatted = symbol.to_string();
        let expected_date = format!("{:02}{:02}{:02}", year, month, day);
        prop_assert!(
            formatted.ends_with(&format!("-{}", expected_date)),
            "Futures symbol should end with date suffix: {}",
            formatted
        );

        // Verify format is BASE/QUOTE:SETTLE-YYMMDD
        let expected = format!(
            "{}/{}:{}-{}",
            base.to_uppercase(),
            quote.to_uppercase(),
            quote.to_uppercase(),
            expected_date
        );
        prop_assert_eq!(formatted, expected);
    }
}

#[cfg(test)]
mod expiry_date_tests {
    use super::*;

    #[test]
    fn test_expiry_date_boundary_values() {
        // Minimum valid values
        assert!(ExpiryDate::new(0, 1, 1).is_ok());

        // Maximum valid values
        assert!(ExpiryDate::new(99, 12, 31).is_ok());

        // Invalid month boundaries
        assert!(ExpiryDate::new(24, 0, 1).is_err());
        assert!(ExpiryDate::new(24, 13, 1).is_err());

        // Invalid day boundaries
        assert!(ExpiryDate::new(24, 1, 0).is_err());
        assert!(ExpiryDate::new(24, 1, 32).is_err());
    }

    #[test]
    fn test_expiry_date_parse_invalid_format() {
        // Too short
        assert!("24123".parse::<ExpiryDate>().is_err());

        // Too long
        assert!("2412311".parse::<ExpiryDate>().is_err());

        // Non-numeric
        assert!("abcdef".parse::<ExpiryDate>().is_err());

        // Invalid month in string
        assert!("241301".parse::<ExpiryDate>().is_err());

        // Invalid day in string
        assert!("241232".parse::<ExpiryDate>().is_err());
    }
}

// ============================================================================
// Property 6: Round-Trip Consistency
// **Feature: unified-symbol-format, Property 6: Round-Trip Consistency**
// **Validates: Requirements 5.1, 5.2, 5.3, 5.4**
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// **Feature: unified-symbol-format, Property 6: Round-Trip Consistency for Spot Symbols**
    /// **Validates: Requirements 5.1, 5.4**
    ///
    /// *For any* valid spot ParsedSymbol:
    /// - Formatting it to a string and parsing it back SHALL produce an equivalent ParsedSymbol
    /// - parse(format(symbol)) == symbol
    #[test]
    fn prop_round_trip_spot_symbol(
        base in arb_currency(),
        quote in arb_currency()
    ) {
        // Create a spot symbol
        let original = ParsedSymbol::spot(base.clone(), quote.clone());

        // Format to string
        let formatted = original.to_string();

        // Parse back
        let parsed = SymbolParser::parse(&formatted)
            .expect("Formatted spot symbol should be parseable");

        // Verify round-trip consistency using references
        prop_assert_eq!(&parsed.base, &original.base, "Base should match after round-trip");
        prop_assert_eq!(&parsed.quote, &original.quote, "Quote should match after round-trip");
        prop_assert_eq!(&parsed.settle, &original.settle, "Settle should match after round-trip");
        prop_assert_eq!(&parsed.expiry, &original.expiry, "Expiry should match after round-trip");
        prop_assert_eq!(parsed.market_type(), original.market_type(), "Market type should match after round-trip");

        // Verify the parsed symbol equals the original
        prop_assert!(parsed == original, "Round-trip should produce equivalent symbol");
    }

    /// **Feature: unified-symbol-format, Property 6: Round-Trip Consistency for Linear Swap Symbols**
    /// **Validates: Requirements 5.2, 5.4**
    ///
    /// *For any* valid linear swap ParsedSymbol:
    /// - Formatting it to a string and parsing it back SHALL produce an equivalent ParsedSymbol
    #[test]
    fn prop_round_trip_linear_swap_symbol(
        base in arb_currency(),
        quote in arb_currency()
    ) {
        // Create a linear swap symbol
        let original = ParsedSymbol::linear_swap(base.clone(), quote.clone());

        // Format to string
        let formatted = original.to_string();

        // Parse back
        let parsed = SymbolParser::parse(&formatted)
            .expect("Formatted linear swap symbol should be parseable");

        // Verify round-trip consistency using references
        prop_assert_eq!(&parsed.base, &original.base, "Base should match after round-trip");
        prop_assert_eq!(&parsed.quote, &original.quote, "Quote should match after round-trip");
        prop_assert_eq!(&parsed.settle, &original.settle, "Settle should match after round-trip");
        prop_assert_eq!(&parsed.expiry, &original.expiry, "Expiry should match after round-trip");
        prop_assert_eq!(parsed.market_type(), original.market_type(), "Market type should match after round-trip");
        prop_assert!(parsed.is_linear(), "Should remain linear after round-trip");

        // Verify the parsed symbol equals the original
        prop_assert!(parsed == original, "Round-trip should produce equivalent symbol");
    }

    /// **Feature: unified-symbol-format, Property 6: Round-Trip Consistency for Inverse Swap Symbols**
    /// **Validates: Requirements 5.2, 5.4**
    ///
    /// *For any* valid inverse swap ParsedSymbol:
    /// - Formatting it to a string and parsing it back SHALL produce an equivalent ParsedSymbol
    #[test]
    fn prop_round_trip_inverse_swap_symbol(
        base in arb_currency(),
        quote in arb_currency()
    ) {
        // Create an inverse swap symbol
        let original = ParsedSymbol::inverse_swap(base.clone(), quote.clone());

        // Format to string
        let formatted = original.to_string();

        // Parse back
        let parsed = SymbolParser::parse(&formatted)
            .expect("Formatted inverse swap symbol should be parseable");

        // Verify round-trip consistency using references
        prop_assert_eq!(&parsed.base, &original.base, "Base should match after round-trip");
        prop_assert_eq!(&parsed.quote, &original.quote, "Quote should match after round-trip");
        prop_assert_eq!(&parsed.settle, &original.settle, "Settle should match after round-trip");
        prop_assert_eq!(&parsed.expiry, &original.expiry, "Expiry should match after round-trip");
        prop_assert_eq!(parsed.market_type(), original.market_type(), "Market type should match after round-trip");
        prop_assert!(parsed.is_inverse(), "Should remain inverse after round-trip");

        // Verify the parsed symbol equals the original
        prop_assert!(parsed == original, "Round-trip should produce equivalent symbol");
    }

    /// **Feature: unified-symbol-format, Property 6: Round-Trip Consistency for Futures Symbols**
    /// **Validates: Requirements 5.3, 5.4**
    ///
    /// *For any* valid futures ParsedSymbol:
    /// - Formatting it to a string and parsing it back SHALL produce an equivalent ParsedSymbol
    #[test]
    fn prop_round_trip_futures_symbol(
        base in arb_currency(),
        quote in arb_currency(),
        year in 20u8..=99,
        month in 1u8..=12,
        day in 1u8..=28
    ) {
        // Create a futures symbol
        let expiry = ExpiryDate::new(year, month, day)
            .expect("Valid date should create ExpiryDate");
        let original = ParsedSymbol::futures(
            base.clone(),
            quote.clone(),
            quote.clone(),  // Linear futures
            expiry
        );

        // Format to string
        let formatted = original.to_string();

        // Parse back
        let parsed = SymbolParser::parse(&formatted)
            .expect("Formatted futures symbol should be parseable");

        // Verify round-trip consistency using references
        prop_assert_eq!(&parsed.base, &original.base, "Base should match after round-trip");
        prop_assert_eq!(&parsed.quote, &original.quote, "Quote should match after round-trip");
        prop_assert_eq!(&parsed.settle, &original.settle, "Settle should match after round-trip");
        prop_assert_eq!(parsed.market_type(), original.market_type(), "Market type should match after round-trip");

        // Verify expiry date round-trip
        prop_assert!(parsed.expiry.is_some(), "Expiry should be present after round-trip");
        let parsed_expiry = parsed.expiry.as_ref().unwrap();
        let original_expiry = original.expiry.as_ref().unwrap();
        prop_assert_eq!(parsed_expiry.year, original_expiry.year, "Year should match after round-trip");
        prop_assert_eq!(parsed_expiry.month, original_expiry.month, "Month should match after round-trip");
        prop_assert_eq!(parsed_expiry.day, original_expiry.day, "Day should match after round-trip");

        // Verify the parsed symbol equals the original
        prop_assert!(parsed == original, "Round-trip should produce equivalent symbol");
    }

    /// **Feature: unified-symbol-format, Property 6: Round-Trip Consistency for Inverse Futures Symbols**
    /// **Validates: Requirements 5.3, 5.4**
    ///
    /// *For any* valid inverse futures ParsedSymbol:
    /// - Formatting it to a string and parsing it back SHALL produce an equivalent ParsedSymbol
    #[test]
    fn prop_round_trip_inverse_futures_symbol(
        base in arb_currency(),
        quote in arb_currency(),
        year in 20u8..=99,
        month in 1u8..=12,
        day in 1u8..=28
    ) {
        // Create an inverse futures symbol (settle == base)
        let expiry = ExpiryDate::new(year, month, day)
            .expect("Valid date should create ExpiryDate");
        let original = ParsedSymbol::futures(
            base.clone(),
            quote.clone(),
            base.clone(),  // Inverse futures (settle == base)
            expiry
        );

        // Format to string
        let formatted = original.to_string();

        // Parse back
        let parsed = SymbolParser::parse(&formatted)
            .expect("Formatted inverse futures symbol should be parseable");

        // Verify round-trip consistency using references
        prop_assert_eq!(&parsed.base, &original.base, "Base should match after round-trip");
        prop_assert_eq!(&parsed.quote, &original.quote, "Quote should match after round-trip");
        prop_assert_eq!(&parsed.settle, &original.settle, "Settle should match after round-trip");
        prop_assert!(parsed.is_inverse(), "Should remain inverse after round-trip");

        // Verify the parsed symbol equals the original
        prop_assert!(parsed == original, "Round-trip should produce equivalent symbol");
    }

    /// **Feature: unified-symbol-format, Property 6: Round-Trip Consistency for Any Valid Symbol**
    /// **Validates: Requirements 5.1, 5.2, 5.3, 5.4**
    ///
    /// *For any* valid ParsedSymbol (spot, swap, or futures):
    /// - Formatting it to a string and parsing it back SHALL produce an equivalent ParsedSymbol
    /// - This is the comprehensive round-trip test covering all symbol types
    #[test]
    fn prop_round_trip_any_symbol(
        base in arb_currency(),
        quote in arb_currency(),
        symbol_type in 0u8..4,  // 0=spot, 1=linear swap, 2=inverse swap, 3=futures
        year in 20u8..=99,
        month in 1u8..=12,
        day in 1u8..=28
    ) {
        let original = match symbol_type {
            0 => ParsedSymbol::spot(base.clone(), quote.clone()),
            1 => ParsedSymbol::linear_swap(base.clone(), quote.clone()),
            2 => ParsedSymbol::inverse_swap(base.clone(), quote.clone()),
            _ => {
                let expiry = ExpiryDate::new(year, month, day)
                    .expect("Valid date should create ExpiryDate");
                ParsedSymbol::futures(base.clone(), quote.clone(), quote.clone(), expiry)
            }
        };

        // Format to string
        let formatted = original.to_string();

        // Parse back
        let parsed = SymbolParser::parse(&formatted)
            .expect("Formatted symbol should be parseable");

        // Verify round-trip consistency - check equality first
        prop_assert!(parsed == original, "Round-trip should produce equivalent symbol");

        // Verify market type is preserved
        prop_assert_eq!(
            parsed.market_type(),
            original.market_type(),
            "Market type should be preserved after round-trip"
        );

        // Verify contract type is preserved for derivatives
        if original.is_derivative() {
            prop_assert_eq!(
                parsed.contract_type(),
                original.contract_type(),
                "Contract type should be preserved after round-trip"
            );
        }
    }
}

#[cfg(test)]
mod round_trip_tests {
    use super::*;

    #[test]
    fn test_round_trip_spot() {
        let original = ParsedSymbol::spot("BTC".to_string(), "USDT".to_string());
        let formatted = original.to_string();
        let parsed = SymbolParser::parse(&formatted).unwrap();
        assert_eq!(parsed, original);
    }

    #[test]
    fn test_round_trip_linear_swap() {
        let original = ParsedSymbol::linear_swap("ETH".to_string(), "USDT".to_string());
        let formatted = original.to_string();
        let parsed = SymbolParser::parse(&formatted).unwrap();
        assert_eq!(parsed, original);
    }

    #[test]
    fn test_round_trip_inverse_swap() {
        let original = ParsedSymbol::inverse_swap("BTC".to_string(), "USD".to_string());
        let formatted = original.to_string();
        let parsed = SymbolParser::parse(&formatted).unwrap();
        assert_eq!(parsed, original);
    }

    #[test]
    fn test_round_trip_futures() {
        let expiry = ExpiryDate::new(24, 12, 31).unwrap();
        let original = ParsedSymbol::futures(
            "BTC".to_string(),
            "USDT".to_string(),
            "USDT".to_string(),
            expiry,
        );
        let formatted = original.to_string();
        let parsed = SymbolParser::parse(&formatted).unwrap();
        assert_eq!(parsed, original);
    }

    #[test]
    fn test_round_trip_preserves_market_type() {
        // Spot
        let spot = ParsedSymbol::spot("BTC".to_string(), "USDT".to_string());
        let parsed_spot = SymbolParser::parse(&spot.to_string()).unwrap();
        assert_eq!(parsed_spot.market_type(), SymbolMarketType::Spot);

        // Swap
        let swap = ParsedSymbol::linear_swap("BTC".to_string(), "USDT".to_string());
        let parsed_swap = SymbolParser::parse(&swap.to_string()).unwrap();
        assert_eq!(parsed_swap.market_type(), SymbolMarketType::Swap);

        // Futures
        let expiry = ExpiryDate::new(24, 12, 31).unwrap();
        let futures = ParsedSymbol::futures(
            "BTC".to_string(),
            "USDT".to_string(),
            "USDT".to_string(),
            expiry,
        );
        let parsed_futures = SymbolParser::parse(&futures.to_string()).unwrap();
        assert_eq!(parsed_futures.market_type(), SymbolMarketType::Futures);
    }

    #[test]
    fn test_round_trip_preserves_contract_type() {
        // Linear
        let linear = ParsedSymbol::linear_swap("BTC".to_string(), "USDT".to_string());
        let parsed_linear = SymbolParser::parse(&linear.to_string()).unwrap();
        assert!(parsed_linear.is_linear());

        // Inverse
        let inverse = ParsedSymbol::inverse_swap("BTC".to_string(), "USD".to_string());
        let parsed_inverse = SymbolParser::parse(&inverse.to_string()).unwrap();
        assert!(parsed_inverse.is_inverse());
    }
}

// ============================================================================
// Property 7: Market Type Detection
// **Feature: unified-symbol-format, Property 7: Market Type Detection**
// **Validates: Requirements 7.1, 7.2, 7.3**
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// **Feature: unified-symbol-format, Property 7: Market Type Detection - Spot**
    /// **Validates: Requirements 7.1**
    ///
    /// *For any* valid unified symbol without a colon (spot symbol):
    /// - The analyzer SHALL correctly identify the market type as Spot
    /// - is_spot() SHALL return true
    /// - is_swap() and is_futures() SHALL return false
    /// - is_derivative() SHALL return false
    #[test]
    fn prop_market_type_detection_spot(
        base in arb_currency(),
        quote in arb_currency()
    ) {
        // Create a spot symbol (no colon, no settle, no expiry)
        let symbol = ParsedSymbol::spot(base.clone(), quote.clone());

        // Verify market type is Spot
        prop_assert_eq!(
            symbol.market_type(),
            SymbolMarketType::Spot,
            "Symbol without settle should be Spot"
        );

        // Verify helper methods
        prop_assert!(symbol.is_spot(), "is_spot() should return true for spot symbol");
        prop_assert!(!symbol.is_swap(), "is_swap() should return false for spot symbol");
        prop_assert!(!symbol.is_futures(), "is_futures() should return false for spot symbol");
        prop_assert!(!symbol.is_derivative(), "is_derivative() should return false for spot symbol");

        // Verify by parsing the formatted string
        let formatted = symbol.to_string();
        prop_assert!(!formatted.contains(':'), "Spot symbol should not contain colon");

        let parsed = SymbolParser::parse(&formatted)
            .expect("Formatted spot symbol should be parseable");
        prop_assert_eq!(
            parsed.market_type(),
            SymbolMarketType::Spot,
            "Parsed spot symbol should have Spot market type"
        );
    }

    /// **Feature: unified-symbol-format, Property 7: Market Type Detection - Swap**
    /// **Validates: Requirements 7.2**
    ///
    /// *For any* valid unified symbol with a colon but no date suffix (swap symbol):
    /// - The analyzer SHALL correctly identify the market type as Swap (Perpetual)
    /// - is_swap() SHALL return true
    /// - is_spot() and is_futures() SHALL return false
    /// - is_derivative() SHALL return true
    #[test]
    fn prop_market_type_detection_swap(
        base in arb_currency(),
        quote in arb_currency()
    ) {
        // Create a swap symbol (has settle, no expiry)
        let symbol = ParsedSymbol::linear_swap(base.clone(), quote.clone());

        // Verify market type is Swap
        prop_assert_eq!(
            symbol.market_type(),
            SymbolMarketType::Swap,
            "Symbol with settle but no expiry should be Swap"
        );

        // Verify helper methods
        prop_assert!(!symbol.is_spot(), "is_spot() should return false for swap symbol");
        prop_assert!(symbol.is_swap(), "is_swap() should return true for swap symbol");
        prop_assert!(!symbol.is_futures(), "is_futures() should return false for swap symbol");
        prop_assert!(symbol.is_derivative(), "is_derivative() should return true for swap symbol");

        // Verify by parsing the formatted string
        let formatted = symbol.to_string();
        prop_assert!(formatted.contains(':'), "Swap symbol should contain colon");
        prop_assert!(!formatted.contains('-'), "Swap symbol should not contain date suffix");

        let parsed = SymbolParser::parse(&formatted)
            .expect("Formatted swap symbol should be parseable");
        prop_assert_eq!(
            parsed.market_type(),
            SymbolMarketType::Swap,
            "Parsed swap symbol should have Swap market type"
        );
    }

    /// **Feature: unified-symbol-format, Property 7: Market Type Detection - Swap (Inverse)**
    /// **Validates: Requirements 7.2**
    ///
    /// *For any* valid inverse swap symbol:
    /// - The analyzer SHALL correctly identify the market type as Swap
    #[test]
    fn prop_market_type_detection_inverse_swap(
        base in arb_currency(),
        quote in arb_currency()
    ) {
        // Create an inverse swap symbol
        let symbol = ParsedSymbol::inverse_swap(base.clone(), quote.clone());

        // Verify market type is Swap
        prop_assert_eq!(
            symbol.market_type(),
            SymbolMarketType::Swap,
            "Inverse swap should have Swap market type"
        );

        prop_assert!(symbol.is_swap(), "is_swap() should return true for inverse swap");
        prop_assert!(symbol.is_derivative(), "is_derivative() should return true for inverse swap");
    }

    /// **Feature: unified-symbol-format, Property 7: Market Type Detection - Futures**
    /// **Validates: Requirements 7.3**
    ///
    /// *For any* valid unified symbol with a colon and date suffix (futures symbol):
    /// - The analyzer SHALL correctly identify the market type as Futures
    /// - is_futures() SHALL return true
    /// - is_spot() and is_swap() SHALL return false
    /// - is_derivative() SHALL return true
    #[test]
    fn prop_market_type_detection_futures(
        base in arb_currency(),
        quote in arb_currency(),
        year in 20u8..=99,
        month in 1u8..=12,
        day in 1u8..=28
    ) {
        // Create a futures symbol (has settle and expiry)
        let expiry = ExpiryDate::new(year, month, day)
            .expect("Valid date should create ExpiryDate");
        let symbol = ParsedSymbol::futures(
            base.clone(),
            quote.clone(),
            quote.clone(),
            expiry
        );

        // Verify market type is Futures
        prop_assert_eq!(
            symbol.market_type(),
            SymbolMarketType::Futures,
            "Symbol with settle and expiry should be Futures"
        );

        // Verify helper methods
        prop_assert!(!symbol.is_spot(), "is_spot() should return false for futures symbol");
        prop_assert!(!symbol.is_swap(), "is_swap() should return false for futures symbol");
        prop_assert!(symbol.is_futures(), "is_futures() should return true for futures symbol");
        prop_assert!(symbol.is_derivative(), "is_derivative() should return true for futures symbol");

        // Verify by parsing the formatted string
        let formatted = symbol.to_string();
        prop_assert!(formatted.contains(':'), "Futures symbol should contain colon");
        prop_assert!(formatted.contains('-'), "Futures symbol should contain date suffix");

        let parsed = SymbolParser::parse(&formatted)
            .expect("Formatted futures symbol should be parseable");
        prop_assert_eq!(
            parsed.market_type(),
            SymbolMarketType::Futures,
            "Parsed futures symbol should have Futures market type"
        );
    }

    /// **Feature: unified-symbol-format, Property 7: Market Type Detection - Inverse Futures**
    /// **Validates: Requirements 7.3**
    ///
    /// *For any* valid inverse futures symbol:
    /// - The analyzer SHALL correctly identify the market type as Futures
    #[test]
    fn prop_market_type_detection_inverse_futures(
        base in arb_currency(),
        quote in arb_currency(),
        year in 20u8..=99,
        month in 1u8..=12,
        day in 1u8..=28
    ) {
        // Create an inverse futures symbol (settle == base)
        let expiry = ExpiryDate::new(year, month, day)
            .expect("Valid date should create ExpiryDate");
        let symbol = ParsedSymbol::futures(
            base.clone(),
            quote.clone(),
            base.clone(),  // Inverse: settle == base
            expiry
        );

        // Verify market type is Futures
        prop_assert_eq!(
            symbol.market_type(),
            SymbolMarketType::Futures,
            "Inverse futures should have Futures market type"
        );

        prop_assert!(symbol.is_futures(), "is_futures() should return true for inverse futures");
        prop_assert!(symbol.is_derivative(), "is_derivative() should return true for inverse futures");
    }

    /// **Feature: unified-symbol-format, Property 7: Market Type Detection - Comprehensive**
    /// **Validates: Requirements 7.1, 7.2, 7.3**
    ///
    /// *For any* valid ParsedSymbol:
    /// - The market type SHALL be correctly determined based on symbol structure
    /// - Exactly one of is_spot(), is_swap(), is_futures() SHALL return true
    #[test]
    fn prop_market_type_detection_comprehensive(
        base in arb_currency(),
        quote in arb_currency(),
        symbol_type in 0u8..4,
        year in 20u8..=99,
        month in 1u8..=12,
        day in 1u8..=28
    ) {
        let symbol = match symbol_type {
            0 => ParsedSymbol::spot(base.clone(), quote.clone()),
            1 => ParsedSymbol::linear_swap(base.clone(), quote.clone()),
            2 => ParsedSymbol::inverse_swap(base.clone(), quote.clone()),
            _ => {
                let expiry = ExpiryDate::new(year, month, day)
                    .expect("Valid date should create ExpiryDate");
                ParsedSymbol::futures(base.clone(), quote.clone(), quote.clone(), expiry)
            }
        };

        // Verify exactly one market type helper returns true
        let spot_count = if symbol.is_spot() { 1 } else { 0 };
        let swap_count = if symbol.is_swap() { 1 } else { 0 };
        let futures_count = if symbol.is_futures() { 1 } else { 0 };

        prop_assert_eq!(
            spot_count + swap_count + futures_count,
            1,
            "Exactly one of is_spot(), is_swap(), is_futures() should return true"
        );

        // Verify is_derivative() is consistent
        let expected_derivative = symbol.is_swap() || symbol.is_futures();
        prop_assert_eq!(
            symbol.is_derivative(),
            expected_derivative,
            "is_derivative() should be true iff is_swap() or is_futures()"
        );

        // Verify market_type() is consistent with helper methods
        match symbol.market_type() {
            SymbolMarketType::Spot => {
                prop_assert!(symbol.is_spot(), "market_type() Spot should match is_spot()");
            }
            SymbolMarketType::Swap => {
                prop_assert!(symbol.is_swap(), "market_type() Swap should match is_swap()");
            }
            SymbolMarketType::Futures => {
                prop_assert!(symbol.is_futures(), "market_type() Futures should match is_futures()");
            }
        }
    }
}

#[cfg(test)]
mod market_type_detection_tests {
    use super::*;

    #[test]
    fn test_market_type_spot() {
        let symbol = ParsedSymbol::spot("BTC".to_string(), "USDT".to_string());
        assert_eq!(symbol.market_type(), SymbolMarketType::Spot);
        assert!(symbol.is_spot());
        assert!(!symbol.is_swap());
        assert!(!symbol.is_futures());
        assert!(!symbol.is_derivative());
    }

    #[test]
    fn test_market_type_swap() {
        let symbol = ParsedSymbol::linear_swap("BTC".to_string(), "USDT".to_string());
        assert_eq!(symbol.market_type(), SymbolMarketType::Swap);
        assert!(!symbol.is_spot());
        assert!(symbol.is_swap());
        assert!(!symbol.is_futures());
        assert!(symbol.is_derivative());
    }

    #[test]
    fn test_market_type_futures() {
        let expiry = ExpiryDate::new(24, 12, 31).unwrap();
        let symbol = ParsedSymbol::futures(
            "BTC".to_string(),
            "USDT".to_string(),
            "USDT".to_string(),
            expiry,
        );
        assert_eq!(symbol.market_type(), SymbolMarketType::Futures);
        assert!(!symbol.is_spot());
        assert!(!symbol.is_swap());
        assert!(symbol.is_futures());
        assert!(symbol.is_derivative());
    }

    #[test]
    fn test_market_type_from_parsed_string() {
        // Spot
        let spot = SymbolParser::parse("BTC/USDT").unwrap();
        assert_eq!(spot.market_type(), SymbolMarketType::Spot);

        // Swap
        let swap = SymbolParser::parse("BTC/USDT:USDT").unwrap();
        assert_eq!(swap.market_type(), SymbolMarketType::Swap);

        // Futures
        let futures = SymbolParser::parse("BTC/USDT:USDT-241231").unwrap();
        assert_eq!(futures.market_type(), SymbolMarketType::Futures);
    }
}

// ============================================================================
// Property 8: Contract Type Detection
// **Feature: unified-symbol-format, Property 8: Contract Type Detection**
// **Validates: Requirements 7.4**
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// **Feature: unified-symbol-format, Property 8: Contract Type Detection - Spot Returns None**
    /// **Validates: Requirements 7.4**
    ///
    /// *For any* valid spot symbol:
    /// - contract_type() SHALL return None (spot symbols have no contract type)
    /// - is_linear() and is_inverse() SHALL return false
    #[test]
    fn prop_contract_type_spot_returns_none(
        base in arb_currency(),
        quote in arb_currency()
    ) {
        let symbol = ParsedSymbol::spot(base.clone(), quote.clone());

        // Spot symbols should have no contract type
        prop_assert!(
            symbol.contract_type().is_none(),
            "Spot symbol should have no contract type"
        );

        // is_linear() and is_inverse() should both return false for spot
        prop_assert!(!symbol.is_linear(), "is_linear() should return false for spot symbol");
        prop_assert!(!symbol.is_inverse(), "is_inverse() should return false for spot symbol");
    }

    /// **Feature: unified-symbol-format, Property 8: Contract Type Detection - Linear Swap**
    /// **Validates: Requirements 7.4**
    ///
    /// *For any* valid linear swap symbol (settle == quote):
    /// - contract_type() SHALL return Some(ContractType::Linear)
    /// - is_linear() SHALL return true
    /// - is_inverse() SHALL return false
    #[test]
    fn prop_contract_type_linear_swap(
        base in arb_currency(),
        quote in arb_currency()
    ) {
        // Create linear swap (settle == quote)
        let symbol = ParsedSymbol::linear_swap(base.clone(), quote.clone());

        // Verify settle equals quote (linear contract definition)
        prop_assert_eq!(
            symbol.settle.as_ref(),
            Some(&quote.to_uppercase()),
            "Linear swap should have settle == quote"
        );

        // Verify contract type is Linear
        prop_assert_eq!(
            symbol.contract_type(),
            Some(ContractType::Linear),
            "Linear swap should have Linear contract type"
        );

        // Verify helper methods
        prop_assert!(symbol.is_linear(), "is_linear() should return true for linear swap");
        prop_assert!(!symbol.is_inverse(), "is_inverse() should return false for linear swap");
    }

    /// **Feature: unified-symbol-format, Property 8: Contract Type Detection - Inverse Swap**
    /// **Validates: Requirements 7.4**
    ///
    /// *For any* valid inverse swap symbol (settle == base):
    /// - contract_type() SHALL return Some(ContractType::Inverse)
    /// - is_inverse() SHALL return true
    /// - is_linear() SHALL return false
    #[test]
    fn prop_contract_type_inverse_swap(
        base in arb_currency(),
        quote in arb_currency()
    ) {
        // Create inverse swap (settle == base)
        let symbol = ParsedSymbol::inverse_swap(base.clone(), quote.clone());

        // Verify settle equals base (inverse contract definition)
        prop_assert_eq!(
            symbol.settle.as_ref(),
            Some(&base.to_uppercase()),
            "Inverse swap should have settle == base"
        );

        // Verify contract type is Inverse
        prop_assert_eq!(
            symbol.contract_type(),
            Some(ContractType::Inverse),
            "Inverse swap should have Inverse contract type"
        );

        // Verify helper methods
        prop_assert!(!symbol.is_linear(), "is_linear() should return false for inverse swap");
        prop_assert!(symbol.is_inverse(), "is_inverse() should return true for inverse swap");
    }

    /// **Feature: unified-symbol-format, Property 8: Contract Type Detection - Linear Futures**
    /// **Validates: Requirements 7.4**
    ///
    /// *For any* valid linear futures symbol (settle == quote):
    /// - contract_type() SHALL return Some(ContractType::Linear)
    #[test]
    fn prop_contract_type_linear_futures(
        base in arb_currency(),
        quote in arb_currency(),
        year in 20u8..=99,
        month in 1u8..=12,
        day in 1u8..=28
    ) {
        let expiry = ExpiryDate::new(year, month, day)
            .expect("Valid date should create ExpiryDate");

        // Create linear futures (settle == quote)
        let symbol = ParsedSymbol::futures(
            base.clone(),
            quote.clone(),
            quote.clone(),  // Linear: settle == quote
            expiry
        );

        // Verify settle equals quote
        prop_assert_eq!(
            symbol.settle.as_ref(),
            Some(&quote.to_uppercase()),
            "Linear futures should have settle == quote"
        );

        // Verify contract type is Linear
        prop_assert_eq!(
            symbol.contract_type(),
            Some(ContractType::Linear),
            "Linear futures should have Linear contract type"
        );

        prop_assert!(symbol.is_linear(), "is_linear() should return true for linear futures");
        prop_assert!(!symbol.is_inverse(), "is_inverse() should return false for linear futures");
    }

    /// **Feature: unified-symbol-format, Property 8: Contract Type Detection - Inverse Futures**
    /// **Validates: Requirements 7.4**
    ///
    /// *For any* valid inverse futures symbol (settle == base):
    /// - contract_type() SHALL return Some(ContractType::Inverse)
    #[test]
    fn prop_contract_type_inverse_futures(
        base in arb_currency(),
        quote in arb_currency(),
        year in 20u8..=99,
        month in 1u8..=12,
        day in 1u8..=28
    ) {
        let expiry = ExpiryDate::new(year, month, day)
            .expect("Valid date should create ExpiryDate");

        // Create inverse futures (settle == base)
        let symbol = ParsedSymbol::futures(
            base.clone(),
            quote.clone(),
            base.clone(),  // Inverse: settle == base
            expiry
        );

        // Verify settle equals base
        prop_assert_eq!(
            symbol.settle.as_ref(),
            Some(&base.to_uppercase()),
            "Inverse futures should have settle == base"
        );

        // Verify contract type is Inverse
        prop_assert_eq!(
            symbol.contract_type(),
            Some(ContractType::Inverse),
            "Inverse futures should have Inverse contract type"
        );

        prop_assert!(!symbol.is_linear(), "is_linear() should return false for inverse futures");
        prop_assert!(symbol.is_inverse(), "is_inverse() should return true for inverse futures");
    }

    /// **Feature: unified-symbol-format, Property 8: Contract Type Detection - Comprehensive**
    /// **Validates: Requirements 7.4**
    ///
    /// *For any* valid derivative symbol (swap or futures):
    /// - contract_type() SHALL return Some(Linear) if settle == quote
    /// - contract_type() SHALL return Some(Inverse) if settle == base
    /// - Exactly one of is_linear() or is_inverse() SHALL return true for derivatives
    #[test]
    fn prop_contract_type_comprehensive(
        base in arb_currency(),
        quote in arb_currency(),
        symbol_type in 1u8..5,  // 1=linear swap, 2=inverse swap, 3=linear futures, 4=inverse futures
        year in 20u8..=99,
        month in 1u8..=12,
        day in 1u8..=28
    ) {
        let symbol = match symbol_type {
            1 => ParsedSymbol::linear_swap(base.clone(), quote.clone()),
            2 => ParsedSymbol::inverse_swap(base.clone(), quote.clone()),
            3 => {
                let expiry = ExpiryDate::new(year, month, day)
                    .expect("Valid date should create ExpiryDate");
                ParsedSymbol::futures(base.clone(), quote.clone(), quote.clone(), expiry)
            }
            _ => {
                let expiry = ExpiryDate::new(year, month, day)
                    .expect("Valid date should create ExpiryDate");
                ParsedSymbol::futures(base.clone(), quote.clone(), base.clone(), expiry)
            }
        };

        // All derivatives should have a contract type
        prop_assert!(
            symbol.contract_type().is_some(),
            "Derivative symbol should have a contract type"
        );

        // Verify exactly one of is_linear() or is_inverse() returns true
        let linear_count = if symbol.is_linear() { 1 } else { 0 };
        let inverse_count = if symbol.is_inverse() { 1 } else { 0 };

        prop_assert_eq!(
            linear_count + inverse_count,
            1,
            "Exactly one of is_linear() or is_inverse() should return true for derivatives"
        );

        // Verify contract type is consistent with settle currency
        let settle = symbol.settle.as_ref().expect("Derivative should have settle");
        if settle == &symbol.quote {
            prop_assert_eq!(
                symbol.contract_type(),
                Some(ContractType::Linear),
                "settle == quote should mean Linear contract"
            );
        } else if settle == &symbol.base {
            prop_assert_eq!(
                symbol.contract_type(),
                Some(ContractType::Inverse),
                "settle == base should mean Inverse contract"
            );
        }
    }

    /// **Feature: unified-symbol-format, Property 8: Contract Type Detection - Round Trip**
    /// **Validates: Requirements 7.4**
    ///
    /// *For any* valid derivative symbol:
    /// - Formatting and parsing back SHALL preserve the contract type
    #[test]
    fn prop_contract_type_round_trip(
        base in arb_currency(),
        quote in arb_currency(),
        is_linear in proptest::bool::ANY,
        is_futures in proptest::bool::ANY,
        year in 20u8..=99,
        month in 1u8..=12,
        day in 1u8..=28
    ) {
        let original = if is_futures {
            let expiry = ExpiryDate::new(year, month, day)
                .expect("Valid date should create ExpiryDate");
            if is_linear {
                ParsedSymbol::futures(base.clone(), quote.clone(), quote.clone(), expiry)
            } else {
                ParsedSymbol::futures(base.clone(), quote.clone(), base.clone(), expiry)
            }
        } else {
            if is_linear {
                ParsedSymbol::linear_swap(base.clone(), quote.clone())
            } else {
                ParsedSymbol::inverse_swap(base.clone(), quote.clone())
            }
        };

        let original_contract_type = original.contract_type();

        // Format and parse back
        let formatted = original.to_string();
        let parsed = SymbolParser::parse(&formatted)
            .expect("Formatted derivative symbol should be parseable");

        // Verify contract type is preserved
        prop_assert_eq!(
            parsed.contract_type(),
            original_contract_type,
            "Contract type should be preserved after round-trip"
        );

        // Verify is_linear() and is_inverse() are preserved
        prop_assert_eq!(
            parsed.is_linear(),
            original.is_linear(),
            "is_linear() should be preserved after round-trip"
        );
        prop_assert_eq!(
            parsed.is_inverse(),
            original.is_inverse(),
            "is_inverse() should be preserved after round-trip"
        );
    }
}

#[cfg(test)]
mod contract_type_detection_tests {
    use super::*;

    #[test]
    fn test_contract_type_spot_none() {
        let symbol = ParsedSymbol::spot("BTC".to_string(), "USDT".to_string());
        assert!(symbol.contract_type().is_none());
        assert!(!symbol.is_linear());
        assert!(!symbol.is_inverse());
    }

    #[test]
    fn test_contract_type_linear_swap() {
        let symbol = ParsedSymbol::linear_swap("BTC".to_string(), "USDT".to_string());
        assert_eq!(symbol.contract_type(), Some(ContractType::Linear));
        assert!(symbol.is_linear());
        assert!(!symbol.is_inverse());
    }

    #[test]
    fn test_contract_type_inverse_swap() {
        let symbol = ParsedSymbol::inverse_swap("BTC".to_string(), "USD".to_string());
        assert_eq!(symbol.contract_type(), Some(ContractType::Inverse));
        assert!(!symbol.is_linear());
        assert!(symbol.is_inverse());
    }

    #[test]
    fn test_contract_type_linear_futures() {
        let expiry = ExpiryDate::new(24, 12, 31).unwrap();
        let symbol = ParsedSymbol::futures(
            "BTC".to_string(),
            "USDT".to_string(),
            "USDT".to_string(),
            expiry,
        );
        assert_eq!(symbol.contract_type(), Some(ContractType::Linear));
        assert!(symbol.is_linear());
    }

    #[test]
    fn test_contract_type_inverse_futures() {
        let expiry = ExpiryDate::new(24, 12, 31).unwrap();
        let symbol = ParsedSymbol::futures(
            "BTC".to_string(),
            "USD".to_string(),
            "BTC".to_string(),
            expiry,
        );
        assert_eq!(symbol.contract_type(), Some(ContractType::Inverse));
        assert!(symbol.is_inverse());
    }

    #[test]
    fn test_contract_type_from_parsed_string() {
        // Linear swap
        let linear = SymbolParser::parse("BTC/USDT:USDT").unwrap();
        assert_eq!(linear.contract_type(), Some(ContractType::Linear));

        // Inverse swap
        let inverse = SymbolParser::parse("BTC/USD:BTC").unwrap();
        assert_eq!(inverse.contract_type(), Some(ContractType::Inverse));

        // Linear futures
        let linear_futures = SymbolParser::parse("BTC/USDT:USDT-241231").unwrap();
        assert_eq!(linear_futures.contract_type(), Some(ContractType::Linear));

        // Inverse futures
        let inverse_futures = SymbolParser::parse("BTC/USD:BTC-241231").unwrap();
        assert_eq!(inverse_futures.contract_type(), Some(ContractType::Inverse));
    }
}
