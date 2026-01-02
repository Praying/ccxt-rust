//! Property-based tests for financial types.
//!
//! These tests verify the correctness properties defined in the design document
//! for the financial types (Price, Amount, Cost) using the proptest framework.
//!
//! **Feature: code-refactoring-improvements**
//! **Validates: Requirements 5.1, 5.2**

use ccxt_core::types::financial::{Amount, Cost, Price};
use proptest::prelude::*;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use std::str::FromStr;

// ============================================================================
// Test Generators
// ============================================================================

/// Strategy for generating valid Price values.
/// Generates prices from 0.00000001 to 1,000,000 with up to 8 decimal places.
fn price_strategy() -> impl Strategy<Value = Price> {
    // Generate positive decimals in a reasonable range for financial values
    (1i64..=100_000_000_000_000i64).prop_map(|n| {
        // Create decimal with 8 decimal places (satoshi precision)
        let mut d = Decimal::from(n);
        d.set_scale(8).unwrap_or(());
        Price::new(d)
    })
}

/// Strategy for generating valid Amount values.
/// Generates amounts from 0.00000001 to 1,000,000 with up to 8 decimal places.
fn amount_strategy() -> impl Strategy<Value = Amount> {
    // Generate positive decimals in a reasonable range for financial values
    (1i64..=100_000_000_000_000i64).prop_map(|n| {
        // Create decimal with 8 decimal places
        let mut d = Decimal::from(n);
        d.set_scale(8).unwrap_or(());
        Amount::new(d)
    })
}

/// Strategy for generating valid Cost values.
fn cost_strategy() -> impl Strategy<Value = Cost> {
    (1i64..=100_000_000_000_000i64).prop_map(|n| {
        let mut d = Decimal::from(n);
        d.set_scale(8).unwrap_or(());
        Cost::new(d)
    })
}

// ============================================================================
// Property 2: Serialization Round-Trip Tests
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// **Feature: code-refactoring-improvements, Property 2: Serialization round-trip**
    /// **Validates: Requirements 5.2**
    ///
    /// *For any* valid Price value, serializing to JSON and deserializing
    /// SHALL produce a value equal to the original.
    #[test]
    fn prop_price_serialization_roundtrip(price in price_strategy()) {
        let json = serde_json::to_string(&price)
            .expect("Price should serialize to JSON");
        let deserialized: Price = serde_json::from_str(&json)
            .expect("Price should deserialize from JSON");
        prop_assert_eq!(price, deserialized,
            "Price serialization round-trip should preserve value");
    }

    /// **Feature: code-refactoring-improvements, Property 2: Serialization round-trip**
    /// **Validates: Requirements 5.2**
    ///
    /// *For any* valid Amount value, serializing to JSON and deserializing
    /// SHALL produce a value equal to the original.
    #[test]
    fn prop_amount_serialization_roundtrip(amount in amount_strategy()) {
        let json = serde_json::to_string(&amount)
            .expect("Amount should serialize to JSON");
        let deserialized: Amount = serde_json::from_str(&json)
            .expect("Amount should deserialize from JSON");
        prop_assert_eq!(amount, deserialized,
            "Amount serialization round-trip should preserve value");
    }

    /// **Feature: code-refactoring-improvements, Property 2: Serialization round-trip**
    /// **Validates: Requirements 5.2**
    ///
    /// *For any* valid Cost value, serializing to JSON and deserializing
    /// SHALL produce a value equal to the original.
    #[test]
    fn prop_cost_serialization_roundtrip(cost in cost_strategy()) {
        let json = serde_json::to_string(&cost)
            .expect("Cost should serialize to JSON");
        let deserialized: Cost = serde_json::from_str(&json)
            .expect("Cost should deserialize from JSON");
        prop_assert_eq!(cost, deserialized,
            "Cost serialization round-trip should preserve value");
    }
}

// ============================================================================
// Property 3: Arithmetic Precision Tests (Price × Amount = Cost)
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// **Feature: code-refactoring-improvements, Property 3: Arithmetic precision**
    /// **Validates: Requirements 5.1**
    ///
    /// *For any* valid Price `p` and Amount `a`, computing `Cost = p × a`
    /// SHALL maintain precision within 10^-8 tolerance.
    #[test]
    fn prop_price_amount_multiplication_precision(
        price in price_strategy(),
        amount in amount_strategy()
    ) {
        // Compute cost using type-safe multiplication
        let cost = price * amount;

        // Compute expected cost using raw decimals
        let expected = price.as_decimal() * amount.as_decimal();

        // Verify precision within 10^-8 tolerance
        let tolerance = dec!(0.00000001);
        let diff = (cost.as_decimal() - expected).abs();

        prop_assert!(
            diff <= tolerance,
            "Precision loss detected: diff={}, tolerance={}, price={}, amount={}, cost={}",
            diff, tolerance, price, amount, cost
        );
    }

    /// **Feature: code-refactoring-improvements, Property 3: Arithmetic precision**
    /// **Validates: Requirements 5.1**
    ///
    /// *For any* valid Price and Amount, multiplication should be commutative:
    /// Price × Amount = Amount × Price
    #[test]
    fn prop_multiplication_commutativity(
        price in price_strategy(),
        amount in amount_strategy()
    ) {
        let cost1 = price * amount;
        let cost2 = amount * price;

        prop_assert_eq!(
            cost1, cost2,
            "Multiplication should be commutative: {} * {} vs {} * {}",
            price, amount, amount, price
        );
    }

    /// **Feature: code-refactoring-improvements, Property 3: Arithmetic precision**
    /// **Validates: Requirements 5.1**
    ///
    /// *For any* valid Cost and Amount (non-zero), Cost ÷ Amount = Price
    /// should maintain precision.
    #[test]
    fn prop_cost_division_by_amount_precision(
        price in price_strategy(),
        amount in amount_strategy()
    ) {
        // Skip if amount is zero (would cause division by zero)
        prop_assume!(!amount.is_zero());

        // Compute cost
        let cost = price * amount;

        // Derive price back from cost / amount
        let derived_price = cost / amount;

        // Verify precision within tolerance
        let tolerance = dec!(0.00000001);
        let diff = (derived_price.as_decimal() - price.as_decimal()).abs();

        prop_assert!(
            diff <= tolerance,
            "Division precision loss: diff={}, original_price={}, derived_price={}",
            diff, price, derived_price
        );
    }

    /// **Feature: code-refactoring-improvements, Property 3: Arithmetic precision**
    /// **Validates: Requirements 5.1**
    ///
    /// *For any* valid Cost and Price (non-zero), Cost ÷ Price = Amount
    /// should maintain precision.
    #[test]
    fn prop_cost_division_by_price_precision(
        price in price_strategy(),
        amount in amount_strategy()
    ) {
        // Skip if price is zero (would cause division by zero)
        prop_assume!(!price.is_zero());

        // Compute cost
        let cost = price * amount;

        // Derive amount back from cost / price
        let derived_amount = cost / price;

        // Verify precision within tolerance
        let tolerance = dec!(0.00000001);
        let diff = (derived_amount.as_decimal() - amount.as_decimal()).abs();

        prop_assert!(
            diff <= tolerance,
            "Division precision loss: diff={}, original_amount={}, derived_amount={}",
            diff, amount, derived_amount
        );
    }
}

// ============================================================================
// Additional Financial Type Properties
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// *For any* valid Price, the Display trait should produce a parseable string.
    #[test]
    fn prop_price_display_parseable(price in price_strategy()) {
        let display_str = price.to_string();
        let parsed = Price::from_str(&display_str);
        prop_assert!(
            parsed.is_ok(),
            "Price display '{}' should be parseable",
            display_str
        );
        prop_assert_eq!(
            price, parsed.expect("parsed price should be Ok"),
            "Parsed price should equal original"
        );
    }

    /// *For any* valid Amount, the Display trait should produce a parseable string.
    #[test]
    fn prop_amount_display_parseable(amount in amount_strategy()) {
        let display_str = amount.to_string();
        let parsed = Amount::from_str(&display_str);
        prop_assert!(
            parsed.is_ok(),
            "Amount display '{}' should be parseable",
            display_str
        );
        prop_assert_eq!(
            amount, parsed.expect("parsed amount should be Ok"),
            "Parsed amount should equal original"
        );
    }

    /// *For any* valid Cost, the Display trait should produce a parseable string.
    #[test]
    fn prop_cost_display_parseable(cost in cost_strategy()) {
        let display_str = cost.to_string();
        let parsed = Cost::from_str(&display_str);
        prop_assert!(
            parsed.is_ok(),
            "Cost display '{}' should be parseable",
            display_str
        );
        prop_assert_eq!(
            cost, parsed.expect("parsed cost should be Ok"),
            "Parsed cost should equal original"
        );
    }

    /// *For any* two Prices, addition should be commutative.
    #[test]
    fn prop_price_addition_commutative(
        p1 in price_strategy(),
        p2 in price_strategy()
    ) {
        let sum1 = p1 + p2;
        let sum2 = p2 + p1;
        prop_assert_eq!(sum1, sum2, "Price addition should be commutative");
    }

    /// *For any* two Amounts, addition should be commutative.
    #[test]
    fn prop_amount_addition_commutative(
        a1 in amount_strategy(),
        a2 in amount_strategy()
    ) {
        let sum1 = a1 + a2;
        let sum2 = a2 + a1;
        prop_assert_eq!(sum1, sum2, "Amount addition should be commutative");
    }
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn test_basic_price_amount_multiplication() {
        let price = Price::new(dec!(50000.0));
        let amount = Amount::new(dec!(0.1));
        let cost = price * amount;
        assert_eq!(cost.as_decimal(), dec!(5000.0));
    }

    #[test]
    fn test_serialization_roundtrip_basic() {
        let price = Price::new(dec!(12345.67890123));
        let json = serde_json::to_string(&price).expect("serialization should succeed");
        let deserialized: Price =
            serde_json::from_str(&json).expect("deserialization should succeed");
        assert_eq!(price, deserialized);
    }
}
