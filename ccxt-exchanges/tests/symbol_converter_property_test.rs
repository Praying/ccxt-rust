//! Property-based tests for exchange-specific symbol converters.
//!
//! These tests verify the correctness properties defined in the design document
//! for converting between unified CCXT symbols and exchange-specific formats.

use ccxt_core::types::symbol::{ExpiryDate, ParsedSymbol, SymbolMarketType};
use ccxt_exchanges::binance::symbol::BinanceSymbolConverter;
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

/// Generator for valid linear futures ParsedSymbol
fn arb_linear_futures_symbol() -> impl Strategy<Value = ParsedSymbol> {
    (arb_currency(), arb_currency(), arb_expiry_date())
        .prop_map(|(base, quote, expiry)| ParsedSymbol::futures(base, quote.clone(), quote, expiry))
}

/// Generator for valid inverse futures ParsedSymbol
fn arb_inverse_futures_symbol() -> impl Strategy<Value = ParsedSymbol> {
    (arb_currency(), arb_currency(), arb_expiry_date())
        .prop_map(|(base, quote, expiry)| ParsedSymbol::futures(base.clone(), quote, base, expiry))
}

// ============================================================================
// Property 9: Binance Format Conversion
// **Feature: unified-symbol-format, Property 9: Binance Format Conversion**
// **Validates: Requirements 6.1**
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// **Feature: unified-symbol-format, Property 9: Binance Spot Symbol Conversion**
    /// **Validates: Requirements 6.1**
    ///
    /// *For any* valid spot ParsedSymbol:
    /// - Converting to Binance format SHALL produce `BASEQUOTE`
    /// - Converting back (with metadata) SHALL preserve the original symbol
    #[test]
    fn prop_binance_spot_conversion(symbol in arb_spot_symbol()) {
        // Convert to Binance format
        let exchange_id = BinanceSymbolConverter::to_exchange_id(&symbol);

        // Verify format: BASEQUOTE (no separators)
        let expected = format!("{}{}", symbol.base, symbol.quote);
        prop_assert_eq!(&exchange_id, &expected, "Spot symbol should be BASEQUOTE format");

        // Verify no special suffixes
        prop_assert!(
            !exchange_id.contains('_'),
            "Spot symbol should not contain underscore"
        );
        prop_assert!(
            !exchange_id.contains('/'),
            "Spot symbol should not contain slash"
        );
        prop_assert!(
            !exchange_id.contains(':'),
            "Spot symbol should not contain colon"
        );

        // Convert back and verify round-trip
        let parsed = BinanceSymbolConverter::from_exchange_id(
            &exchange_id,
            SymbolMarketType::Spot,
            None,
            &symbol.base,
            &symbol.quote,
        ).expect("Should convert back successfully");

        prop_assert_eq!(&parsed.base, &symbol.base);
        prop_assert_eq!(&parsed.quote, &symbol.quote);
        prop_assert!(parsed.settle.is_none());
        prop_assert!(parsed.expiry.is_none());
        prop_assert!(parsed.is_spot());
    }

    /// **Feature: unified-symbol-format, Property 9: Binance Linear Swap Conversion**
    /// **Validates: Requirements 6.1**
    ///
    /// *For any* valid linear swap ParsedSymbol:
    /// - Converting to Binance format SHALL produce `BASEQUOTE`
    /// - Converting back (with metadata) SHALL preserve the original symbol
    #[test]
    fn prop_binance_linear_swap_conversion(symbol in arb_linear_swap_symbol()) {
        // Convert to Binance format
        let exchange_id = BinanceSymbolConverter::to_exchange_id(&symbol);

        // Verify format: BASEQUOTE (same as spot for linear perpetuals)
        let expected = format!("{}{}", symbol.base, symbol.quote);
        prop_assert_eq!(&exchange_id, &expected, "Linear swap should be BASEQUOTE format");

        // Verify no PERP suffix (that's for inverse)
        prop_assert!(
            !exchange_id.ends_with("_PERP"),
            "Linear swap should not have _PERP suffix"
        );

        // Convert back and verify round-trip
        let settle = symbol.settle.as_ref().unwrap();
        let parsed = BinanceSymbolConverter::from_exchange_id(
            &exchange_id,
            SymbolMarketType::Swap,
            Some(settle),
            &symbol.base,
            &symbol.quote,
        ).expect("Should convert back successfully");

        prop_assert_eq!(&parsed.base, &symbol.base);
        prop_assert_eq!(&parsed.quote, &symbol.quote);
        prop_assert_eq!(&parsed.settle, &symbol.settle);
        prop_assert!(parsed.expiry.is_none());
        prop_assert!(parsed.is_swap());
    }

    /// **Feature: unified-symbol-format, Property 9: Binance Inverse Swap Conversion**
    /// **Validates: Requirements 6.1**
    ///
    /// *For any* valid inverse swap ParsedSymbol:
    /// - Converting to Binance format SHALL produce `BASEQUOTE_PERP`
    /// - Converting back (with metadata) SHALL preserve the original symbol
    #[test]
    fn prop_binance_inverse_swap_conversion(symbol in arb_inverse_swap_symbol()) {
        // Convert to Binance format
        let exchange_id = BinanceSymbolConverter::to_exchange_id(&symbol);

        // Verify format: BASEQUOTE_PERP
        let expected = format!("{}{}_PERP", symbol.base, symbol.quote);
        prop_assert_eq!(&exchange_id, &expected, "Inverse swap should be BASEQUOTE_PERP format");

        // Verify has PERP suffix
        prop_assert!(
            exchange_id.ends_with("_PERP"),
            "Inverse swap should have _PERP suffix"
        );

        // Verify is_perpetual helper
        prop_assert!(
            BinanceSymbolConverter::is_perpetual(&exchange_id),
            "is_perpetual should return true for inverse swap"
        );

        // Convert back and verify round-trip
        let settle = symbol.settle.as_ref().unwrap();
        let parsed = BinanceSymbolConverter::from_exchange_id(
            &exchange_id,
            SymbolMarketType::Swap,
            Some(settle),
            &symbol.base,
            &symbol.quote,
        ).expect("Should convert back successfully");

        prop_assert_eq!(&parsed.base, &symbol.base);
        prop_assert_eq!(&parsed.quote, &symbol.quote);
        prop_assert_eq!(&parsed.settle, &symbol.settle);
        prop_assert!(parsed.expiry.is_none());
        prop_assert!(parsed.is_swap());
    }

    /// **Feature: unified-symbol-format, Property 9: Binance Linear Futures Conversion**
    /// **Validates: Requirements 6.1**
    ///
    /// *For any* valid linear futures ParsedSymbol:
    /// - Converting to Binance format SHALL produce `BASEQUOTE_YYMMDD`
    /// - Converting back (with metadata) SHALL preserve the original symbol
    #[test]
    fn prop_binance_linear_futures_conversion(symbol in arb_linear_futures_symbol()) {
        // Convert to Binance format
        let exchange_id = BinanceSymbolConverter::to_exchange_id(&symbol);

        // Verify format: BASEQUOTE_YYMMDD
        let expiry = symbol.expiry.as_ref().unwrap();
        let expected = format!(
            "{}{}_{:02}{:02}{:02}",
            symbol.base, symbol.quote, expiry.year, expiry.month, expiry.day
        );
        prop_assert_eq!(&exchange_id, &expected, "Linear futures should be BASEQUOTE_YYMMDD format");

        // Verify is_futures helper
        prop_assert!(
            BinanceSymbolConverter::is_futures(&exchange_id),
            "is_futures should return true for futures"
        );

        // Verify not perpetual
        prop_assert!(
            !BinanceSymbolConverter::is_perpetual(&exchange_id),
            "is_perpetual should return false for futures"
        );

        // Convert back and verify round-trip
        let settle = symbol.settle.as_ref().unwrap();
        let parsed = BinanceSymbolConverter::from_exchange_id(
            &exchange_id,
            SymbolMarketType::Futures,
            Some(settle),
            &symbol.base,
            &symbol.quote,
        ).expect("Should convert back successfully");

        prop_assert_eq!(&parsed.base, &symbol.base);
        prop_assert_eq!(&parsed.quote, &symbol.quote);
        prop_assert_eq!(&parsed.settle, &symbol.settle);
        prop_assert!(parsed.is_futures());

        // Verify expiry date
        let parsed_expiry = parsed.expiry.as_ref().unwrap();
        prop_assert_eq!(parsed_expiry.year, expiry.year);
        prop_assert_eq!(parsed_expiry.month, expiry.month);
        prop_assert_eq!(parsed_expiry.day, expiry.day);
    }

    /// **Feature: unified-symbol-format, Property 9: Binance Inverse Futures Conversion**
    /// **Validates: Requirements 6.1**
    ///
    /// *For any* valid inverse futures ParsedSymbol:
    /// - Converting to Binance format SHALL produce `BASEQUOTE_YYMMDD`
    /// - Converting back (with metadata) SHALL preserve the original symbol
    #[test]
    fn prop_binance_inverse_futures_conversion(symbol in arb_inverse_futures_symbol()) {
        // Convert to Binance format
        let exchange_id = BinanceSymbolConverter::to_exchange_id(&symbol);

        // Verify format: BASEQUOTE_YYMMDD (same as linear futures)
        let expiry = symbol.expiry.as_ref().unwrap();
        let expected = format!(
            "{}{}_{:02}{:02}{:02}",
            symbol.base, symbol.quote, expiry.year, expiry.month, expiry.day
        );
        prop_assert_eq!(&exchange_id, &expected, "Inverse futures should be BASEQUOTE_YYMMDD format");

        // Verify is_futures helper
        prop_assert!(
            BinanceSymbolConverter::is_futures(&exchange_id),
            "is_futures should return true for inverse futures"
        );

        // Convert back and verify round-trip
        let settle = symbol.settle.as_ref().unwrap();
        let parsed = BinanceSymbolConverter::from_exchange_id(
            &exchange_id,
            SymbolMarketType::Futures,
            Some(settle),
            &symbol.base,
            &symbol.quote,
        ).expect("Should convert back successfully");

        prop_assert_eq!(&parsed.base, &symbol.base);
        prop_assert_eq!(&parsed.quote, &symbol.quote);
        prop_assert_eq!(&parsed.settle, &symbol.settle);
        prop_assert!(parsed.is_futures());
        prop_assert!(parsed.is_inverse());
    }
}

#[cfg(test)]
mod binance_conversion_tests {
    use super::*;

    #[test]
    fn test_binance_spot_basic() {
        let symbol = ParsedSymbol::spot("BTC".to_string(), "USDT".to_string());
        let exchange_id = BinanceSymbolConverter::to_exchange_id(&symbol);
        assert_eq!(exchange_id, "BTCUSDT");
    }

    #[test]
    fn test_binance_linear_swap_basic() {
        let symbol = ParsedSymbol::linear_swap("BTC".to_string(), "USDT".to_string());
        let exchange_id = BinanceSymbolConverter::to_exchange_id(&symbol);
        assert_eq!(exchange_id, "BTCUSDT");
    }

    #[test]
    fn test_binance_inverse_swap_basic() {
        let symbol = ParsedSymbol::inverse_swap("BTC".to_string(), "USD".to_string());
        let exchange_id = BinanceSymbolConverter::to_exchange_id(&symbol);
        assert_eq!(exchange_id, "BTCUSD_PERP");
    }

    #[test]
    fn test_binance_futures_basic() {
        let expiry = ExpiryDate::new(24, 12, 31).unwrap();
        let symbol = ParsedSymbol::futures(
            "BTC".to_string(),
            "USDT".to_string(),
            "USDT".to_string(),
            expiry,
        );
        let exchange_id = BinanceSymbolConverter::to_exchange_id(&symbol);
        assert_eq!(exchange_id, "BTCUSDT_241231");
    }
}

// ============================================================================
// Property 10: OKX Format Conversion
// **Feature: unified-symbol-format, Property 10: OKX Format Conversion**
// **Validates: Requirements 6.2**
// ============================================================================

use ccxt_exchanges::okx::symbol::OkxSymbolConverter;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// **Feature: unified-symbol-format, Property 10: OKX Spot Symbol Conversion**
    /// **Validates: Requirements 6.2**
    ///
    /// *For any* valid spot ParsedSymbol:
    /// - Converting to OKX format SHALL produce `BASE-QUOTE`
    /// - Converting back (with metadata) SHALL preserve the original symbol
    #[test]
    fn prop_okx_spot_conversion(symbol in arb_spot_symbol()) {
        // Convert to OKX format
        let exchange_id = OkxSymbolConverter::to_exchange_id(&symbol);

        // Verify format: BASE-QUOTE
        let expected = format!("{}-{}", symbol.base, symbol.quote);
        prop_assert_eq!(&exchange_id, &expected, "Spot symbol should be BASE-QUOTE format");

        // Verify is_spot helper
        prop_assert!(
            OkxSymbolConverter::is_spot(&exchange_id),
            "is_spot should return true for spot"
        );

        // Verify not swap or futures
        prop_assert!(
            !OkxSymbolConverter::is_swap(&exchange_id),
            "is_swap should return false for spot"
        );
        prop_assert!(
            !OkxSymbolConverter::is_futures(&exchange_id),
            "is_futures should return false for spot"
        );

        // Convert back and verify round-trip
        let parsed = OkxSymbolConverter::from_exchange_id(
            &exchange_id,
            SymbolMarketType::Spot,
            None,
        ).expect("Should convert back successfully");

        prop_assert_eq!(&parsed.base, &symbol.base);
        prop_assert_eq!(&parsed.quote, &symbol.quote);
        prop_assert!(parsed.settle.is_none());
        prop_assert!(parsed.expiry.is_none());
        prop_assert!(parsed.is_spot());
    }

    /// **Feature: unified-symbol-format, Property 10: OKX Linear Swap Conversion**
    /// **Validates: Requirements 6.2**
    ///
    /// *For any* valid linear swap ParsedSymbol:
    /// - Converting to OKX format SHALL produce `BASE-QUOTE-SWAP`
    /// - Converting back (with metadata) SHALL preserve the original symbol
    #[test]
    fn prop_okx_linear_swap_conversion(symbol in arb_linear_swap_symbol()) {
        // Convert to OKX format
        let exchange_id = OkxSymbolConverter::to_exchange_id(&symbol);

        // Verify format: BASE-QUOTE-SWAP
        let expected = format!("{}-{}-SWAP", symbol.base, symbol.quote);
        prop_assert_eq!(&exchange_id, &expected, "Linear swap should be BASE-QUOTE-SWAP format");

        // Verify is_swap helper
        prop_assert!(
            OkxSymbolConverter::is_swap(&exchange_id),
            "is_swap should return true for swap"
        );

        // Convert back and verify round-trip
        let settle = symbol.settle.as_ref().unwrap();
        let parsed = OkxSymbolConverter::from_exchange_id(
            &exchange_id,
            SymbolMarketType::Swap,
            Some(settle),
        ).expect("Should convert back successfully");

        prop_assert_eq!(&parsed.base, &symbol.base);
        prop_assert_eq!(&parsed.quote, &symbol.quote);
        prop_assert_eq!(&parsed.settle, &symbol.settle);
        prop_assert!(parsed.expiry.is_none());
        prop_assert!(parsed.is_swap());
    }

    /// **Feature: unified-symbol-format, Property 10: OKX Inverse Swap Conversion**
    /// **Validates: Requirements 6.2**
    ///
    /// *For any* valid inverse swap ParsedSymbol:
    /// - Converting to OKX format SHALL produce `BASE-QUOTE-SWAP`
    /// - Converting back (with metadata) SHALL preserve the original symbol
    #[test]
    fn prop_okx_inverse_swap_conversion(symbol in arb_inverse_swap_symbol()) {
        // Convert to OKX format
        let exchange_id = OkxSymbolConverter::to_exchange_id(&symbol);

        // Verify format: BASE-QUOTE-SWAP (same as linear for OKX)
        let expected = format!("{}-{}-SWAP", symbol.base, symbol.quote);
        prop_assert_eq!(&exchange_id, &expected, "Inverse swap should be BASE-QUOTE-SWAP format");

        // Verify is_swap helper
        prop_assert!(
            OkxSymbolConverter::is_swap(&exchange_id),
            "is_swap should return true for inverse swap"
        );

        // Convert back and verify round-trip
        let settle = symbol.settle.as_ref().unwrap();
        let parsed = OkxSymbolConverter::from_exchange_id(
            &exchange_id,
            SymbolMarketType::Swap,
            Some(settle),
        ).expect("Should convert back successfully");

        prop_assert_eq!(&parsed.base, &symbol.base);
        prop_assert_eq!(&parsed.quote, &symbol.quote);
        prop_assert_eq!(&parsed.settle, &symbol.settle);
        prop_assert!(parsed.expiry.is_none());
        prop_assert!(parsed.is_swap());
    }

    /// **Feature: unified-symbol-format, Property 10: OKX Linear Futures Conversion**
    /// **Validates: Requirements 6.2**
    ///
    /// *For any* valid linear futures ParsedSymbol:
    /// - Converting to OKX format SHALL produce `BASE-QUOTE-YYMMDD`
    /// - Converting back (with metadata) SHALL preserve the original symbol
    #[test]
    fn prop_okx_linear_futures_conversion(symbol in arb_linear_futures_symbol()) {
        // Convert to OKX format
        let exchange_id = OkxSymbolConverter::to_exchange_id(&symbol);

        // Verify format: BASE-QUOTE-YYMMDD
        let expiry = symbol.expiry.as_ref().unwrap();
        let expected = format!(
            "{}-{}-{:02}{:02}{:02}",
            symbol.base, symbol.quote, expiry.year, expiry.month, expiry.day
        );
        prop_assert_eq!(&exchange_id, &expected, "Linear futures should be BASE-QUOTE-YYMMDD format");

        // Verify is_futures helper
        prop_assert!(
            OkxSymbolConverter::is_futures(&exchange_id),
            "is_futures should return true for futures"
        );

        // Verify not swap
        prop_assert!(
            !OkxSymbolConverter::is_swap(&exchange_id),
            "is_swap should return false for futures"
        );

        // Convert back and verify round-trip
        let settle = symbol.settle.as_ref().unwrap();
        let parsed = OkxSymbolConverter::from_exchange_id(
            &exchange_id,
            SymbolMarketType::Futures,
            Some(settle),
        ).expect("Should convert back successfully");

        prop_assert_eq!(&parsed.base, &symbol.base);
        prop_assert_eq!(&parsed.quote, &symbol.quote);
        prop_assert_eq!(&parsed.settle, &symbol.settle);
        prop_assert!(parsed.is_futures());

        // Verify expiry date
        let parsed_expiry = parsed.expiry.as_ref().unwrap();
        prop_assert_eq!(parsed_expiry.year, expiry.year);
        prop_assert_eq!(parsed_expiry.month, expiry.month);
        prop_assert_eq!(parsed_expiry.day, expiry.day);
    }

    /// **Feature: unified-symbol-format, Property 10: OKX Inverse Futures Conversion**
    /// **Validates: Requirements 6.2**
    ///
    /// *For any* valid inverse futures ParsedSymbol:
    /// - Converting to OKX format SHALL produce `BASE-QUOTE-YYMMDD`
    /// - Converting back (with metadata) SHALL preserve the original symbol
    #[test]
    fn prop_okx_inverse_futures_conversion(symbol in arb_inverse_futures_symbol()) {
        // Convert to OKX format
        let exchange_id = OkxSymbolConverter::to_exchange_id(&symbol);

        // Verify format: BASE-QUOTE-YYMMDD (same as linear futures)
        let expiry = symbol.expiry.as_ref().unwrap();
        let expected = format!(
            "{}-{}-{:02}{:02}{:02}",
            symbol.base, symbol.quote, expiry.year, expiry.month, expiry.day
        );
        prop_assert_eq!(&exchange_id, &expected, "Inverse futures should be BASE-QUOTE-YYMMDD format");

        // Verify is_futures helper
        prop_assert!(
            OkxSymbolConverter::is_futures(&exchange_id),
            "is_futures should return true for inverse futures"
        );

        // Convert back and verify round-trip
        let settle = symbol.settle.as_ref().unwrap();
        let parsed = OkxSymbolConverter::from_exchange_id(
            &exchange_id,
            SymbolMarketType::Futures,
            Some(settle),
        ).expect("Should convert back successfully");

        prop_assert_eq!(&parsed.base, &symbol.base);
        prop_assert_eq!(&parsed.quote, &symbol.quote);
        prop_assert_eq!(&parsed.settle, &symbol.settle);
        prop_assert!(parsed.is_futures());
        prop_assert!(parsed.is_inverse());
    }
}

#[cfg(test)]
mod okx_conversion_tests {
    use super::*;

    #[test]
    fn test_okx_spot_basic() {
        let symbol = ParsedSymbol::spot("BTC".to_string(), "USDT".to_string());
        let exchange_id = OkxSymbolConverter::to_exchange_id(&symbol);
        assert_eq!(exchange_id, "BTC-USDT");
    }

    #[test]
    fn test_okx_linear_swap_basic() {
        let symbol = ParsedSymbol::linear_swap("BTC".to_string(), "USDT".to_string());
        let exchange_id = OkxSymbolConverter::to_exchange_id(&symbol);
        assert_eq!(exchange_id, "BTC-USDT-SWAP");
    }

    #[test]
    fn test_okx_inverse_swap_basic() {
        let symbol = ParsedSymbol::inverse_swap("BTC".to_string(), "USD".to_string());
        let exchange_id = OkxSymbolConverter::to_exchange_id(&symbol);
        assert_eq!(exchange_id, "BTC-USD-SWAP");
    }

    #[test]
    fn test_okx_futures_basic() {
        let expiry = ExpiryDate::new(24, 12, 31).unwrap();
        let symbol = ParsedSymbol::futures(
            "BTC".to_string(),
            "USDT".to_string(),
            "USDT".to_string(),
            expiry,
        );
        let exchange_id = OkxSymbolConverter::to_exchange_id(&symbol);
        assert_eq!(exchange_id, "BTC-USDT-241231");
    }
}

// ============================================================================
// Property 11: Bybit Format Conversion
// **Feature: unified-symbol-format, Property 11: Bybit Format Conversion**
// **Validates: Requirements 6.3**
// ============================================================================

use ccxt_exchanges::bybit::symbol::BybitSymbolConverter;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// **Feature: unified-symbol-format, Property 11: Bybit Spot Symbol Conversion**
    /// **Validates: Requirements 6.3**
    ///
    /// *For any* valid spot ParsedSymbol:
    /// - Converting to Bybit format SHALL produce `BASEQUOTE`
    /// - Converting back (with metadata) SHALL preserve the original symbol
    #[test]
    fn prop_bybit_spot_conversion(symbol in arb_spot_symbol()) {
        // Convert to Bybit format
        let exchange_id = BybitSymbolConverter::to_exchange_id(&symbol);

        // Verify format: BASEQUOTE
        let expected = format!("{}{}", symbol.base, symbol.quote);
        prop_assert_eq!(&exchange_id, &expected, "Spot symbol should be BASEQUOTE format");

        // Verify is_spot_or_perpetual helper
        prop_assert!(
            BybitSymbolConverter::is_spot_or_perpetual(&exchange_id),
            "is_spot_or_perpetual should return true for spot"
        );

        // Verify not futures
        prop_assert!(
            !BybitSymbolConverter::is_futures(&exchange_id),
            "is_futures should return false for spot"
        );

        // Convert back and verify round-trip
        let parsed = BybitSymbolConverter::from_exchange_id(
            &exchange_id,
            SymbolMarketType::Spot,
            None,
            &symbol.base,
            &symbol.quote,
        ).expect("Should convert back successfully");

        prop_assert_eq!(&parsed.base, &symbol.base);
        prop_assert_eq!(&parsed.quote, &symbol.quote);
        prop_assert!(parsed.settle.is_none());
        prop_assert!(parsed.expiry.is_none());
        prop_assert!(parsed.is_spot());
    }

    /// **Feature: unified-symbol-format, Property 11: Bybit Linear Swap Conversion**
    /// **Validates: Requirements 6.3**
    ///
    /// *For any* valid linear swap ParsedSymbol:
    /// - Converting to Bybit format SHALL produce `BASEQUOTE`
    /// - Converting back (with metadata) SHALL preserve the original symbol
    #[test]
    fn prop_bybit_linear_swap_conversion(symbol in arb_linear_swap_symbol()) {
        // Convert to Bybit format
        let exchange_id = BybitSymbolConverter::to_exchange_id(&symbol);

        // Verify format: BASEQUOTE (same as spot for Bybit perpetuals)
        let expected = format!("{}{}", symbol.base, symbol.quote);
        prop_assert_eq!(&exchange_id, &expected, "Linear swap should be BASEQUOTE format");

        // Verify is_spot_or_perpetual helper
        prop_assert!(
            BybitSymbolConverter::is_spot_or_perpetual(&exchange_id),
            "is_spot_or_perpetual should return true for swap"
        );

        // Convert back and verify round-trip
        let settle = symbol.settle.as_ref().unwrap();
        let parsed = BybitSymbolConverter::from_exchange_id(
            &exchange_id,
            SymbolMarketType::Swap,
            Some(settle),
            &symbol.base,
            &symbol.quote,
        ).expect("Should convert back successfully");

        prop_assert_eq!(&parsed.base, &symbol.base);
        prop_assert_eq!(&parsed.quote, &symbol.quote);
        prop_assert_eq!(&parsed.settle, &symbol.settle);
        prop_assert!(parsed.expiry.is_none());
        prop_assert!(parsed.is_swap());
    }

    /// **Feature: unified-symbol-format, Property 11: Bybit Inverse Swap Conversion**
    /// **Validates: Requirements 6.3**
    ///
    /// *For any* valid inverse swap ParsedSymbol:
    /// - Converting to Bybit format SHALL produce `BASEQUOTE`
    /// - Converting back (with metadata) SHALL preserve the original symbol
    #[test]
    fn prop_bybit_inverse_swap_conversion(symbol in arb_inverse_swap_symbol()) {
        // Convert to Bybit format
        let exchange_id = BybitSymbolConverter::to_exchange_id(&symbol);

        // Verify format: BASEQUOTE (same as linear for Bybit)
        let expected = format!("{}{}", symbol.base, symbol.quote);
        prop_assert_eq!(&exchange_id, &expected, "Inverse swap should be BASEQUOTE format");

        // Verify is_spot_or_perpetual helper
        prop_assert!(
            BybitSymbolConverter::is_spot_or_perpetual(&exchange_id),
            "is_spot_or_perpetual should return true for inverse swap"
        );

        // Convert back and verify round-trip
        let settle = symbol.settle.as_ref().unwrap();
        let parsed = BybitSymbolConverter::from_exchange_id(
            &exchange_id,
            SymbolMarketType::Swap,
            Some(settle),
            &symbol.base,
            &symbol.quote,
        ).expect("Should convert back successfully");

        prop_assert_eq!(&parsed.base, &symbol.base);
        prop_assert_eq!(&parsed.quote, &symbol.quote);
        prop_assert_eq!(&parsed.settle, &symbol.settle);
        prop_assert!(parsed.expiry.is_none());
        prop_assert!(parsed.is_swap());
    }

    /// **Feature: unified-symbol-format, Property 11: Bybit Linear Futures Conversion**
    /// **Validates: Requirements 6.3**
    ///
    /// *For any* valid linear futures ParsedSymbol:
    /// - Converting to Bybit format SHALL produce `BASEQUOTE-DDMMMYY`
    /// - Converting back (with metadata) SHALL preserve the original symbol
    #[test]
    fn prop_bybit_linear_futures_conversion(symbol in arb_linear_futures_symbol()) {
        // Convert to Bybit format
        let exchange_id = BybitSymbolConverter::to_exchange_id(&symbol);

        // Verify format contains hyphen (futures indicator)
        prop_assert!(
            exchange_id.contains('-'),
            "Futures symbol should contain hyphen"
        );

        // Verify is_futures helper
        prop_assert!(
            BybitSymbolConverter::is_futures(&exchange_id),
            "is_futures should return true for futures"
        );

        // Verify not spot_or_perpetual
        prop_assert!(
            !BybitSymbolConverter::is_spot_or_perpetual(&exchange_id),
            "is_spot_or_perpetual should return false for futures"
        );

        // Convert back and verify round-trip
        let settle = symbol.settle.as_ref().unwrap();
        let parsed = BybitSymbolConverter::from_exchange_id(
            &exchange_id,
            SymbolMarketType::Futures,
            Some(settle),
            &symbol.base,
            &symbol.quote,
        ).expect("Should convert back successfully");

        prop_assert_eq!(&parsed.base, &symbol.base);
        prop_assert_eq!(&parsed.quote, &symbol.quote);
        prop_assert_eq!(&parsed.settle, &symbol.settle);
        prop_assert!(parsed.is_futures());

        // Verify expiry date
        let expiry = symbol.expiry.as_ref().unwrap();
        let parsed_expiry = parsed.expiry.as_ref().unwrap();
        prop_assert_eq!(parsed_expiry.year, expiry.year);
        prop_assert_eq!(parsed_expiry.month, expiry.month);
        prop_assert_eq!(parsed_expiry.day, expiry.day);
    }

    /// **Feature: unified-symbol-format, Property 11: Bybit Inverse Futures Conversion**
    /// **Validates: Requirements 6.3**
    ///
    /// *For any* valid inverse futures ParsedSymbol:
    /// - Converting to Bybit format SHALL produce `BASEQUOTE-DDMMMYY`
    /// - Converting back (with metadata) SHALL preserve the original symbol
    #[test]
    fn prop_bybit_inverse_futures_conversion(symbol in arb_inverse_futures_symbol()) {
        // Convert to Bybit format
        let exchange_id = BybitSymbolConverter::to_exchange_id(&symbol);

        // Verify format contains hyphen (futures indicator)
        prop_assert!(
            exchange_id.contains('-'),
            "Inverse futures symbol should contain hyphen"
        );

        // Verify is_futures helper
        prop_assert!(
            BybitSymbolConverter::is_futures(&exchange_id),
            "is_futures should return true for inverse futures"
        );

        // Convert back and verify round-trip
        let settle = symbol.settle.as_ref().unwrap();
        let parsed = BybitSymbolConverter::from_exchange_id(
            &exchange_id,
            SymbolMarketType::Futures,
            Some(settle),
            &symbol.base,
            &symbol.quote,
        ).expect("Should convert back successfully");

        prop_assert_eq!(&parsed.base, &symbol.base);
        prop_assert_eq!(&parsed.quote, &symbol.quote);
        prop_assert_eq!(&parsed.settle, &symbol.settle);
        prop_assert!(parsed.is_futures());
        prop_assert!(parsed.is_inverse());
    }
}

#[cfg(test)]
mod bybit_conversion_tests {
    use super::*;

    #[test]
    fn test_bybit_spot_basic() {
        let symbol = ParsedSymbol::spot("BTC".to_string(), "USDT".to_string());
        let exchange_id = BybitSymbolConverter::to_exchange_id(&symbol);
        assert_eq!(exchange_id, "BTCUSDT");
    }

    #[test]
    fn test_bybit_linear_swap_basic() {
        let symbol = ParsedSymbol::linear_swap("BTC".to_string(), "USDT".to_string());
        let exchange_id = BybitSymbolConverter::to_exchange_id(&symbol);
        assert_eq!(exchange_id, "BTCUSDT");
    }

    #[test]
    fn test_bybit_inverse_swap_basic() {
        let symbol = ParsedSymbol::inverse_swap("BTC".to_string(), "USD".to_string());
        let exchange_id = BybitSymbolConverter::to_exchange_id(&symbol);
        assert_eq!(exchange_id, "BTCUSD");
    }

    #[test]
    fn test_bybit_futures_basic() {
        let expiry = ExpiryDate::new(24, 12, 31).unwrap();
        let symbol = ParsedSymbol::futures(
            "BTC".to_string(),
            "USDT".to_string(),
            "USDT".to_string(),
            expiry,
        );
        let exchange_id = BybitSymbolConverter::to_exchange_id(&symbol);
        assert_eq!(exchange_id, "BTCUSDT-31DEC24");
    }
}
