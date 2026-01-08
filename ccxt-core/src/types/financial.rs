//! Financial calculation type definitions.
//!
//! Provides type-safe financial wrappers that prevent unit confusion and precision loss.
//! Uses the newtype pattern to implement zero-cost abstractions with compile-time type checking.
//!
//! # Examples
//!
//! ```rust
//! use ccxt_core::types::financial::{Price, Amount, Cost};
//! use rust_decimal_macros::dec;
//!
//! let price = Price::new(dec!(50000.0));
//! let amount = Amount::new(dec!(0.1));
//! let cost = price * amount;  // Type-safe: Price × Amount = Cost
//!
//! assert_eq!(cost.as_decimal(), dec!(5000.0));
//! ```

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

/// Price type (zero-cost wrapper).
///
/// Represents the price of an asset using `Decimal` for precision.
/// Provides type safety via the newtype pattern to prevent confusion with amounts or costs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Price(pub Decimal);

impl Price {
    /// Creates a new price instance.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use ccxt_core::types::financial::Price;
    /// use rust_decimal_macros::dec;
    ///
    /// let price = Price::new(dec!(50000.0));
    /// ```
    pub fn new(value: Decimal) -> Self {
        Self(value)
    }

    /// Returns the inner `Decimal` value.
    #[inline]
    pub fn as_decimal(&self) -> Decimal {
        self.0
    }

    /// Parses a price from a string (common exchange API format).
    ///
    /// # Errors
    ///
    /// Returns an error if the string cannot be parsed as a valid `Decimal`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use ccxt_core::types::financial::Price;
    ///
    /// let price = Price::parse("50000.50").unwrap();
    /// ```
    pub fn parse(s: &str) -> Result<Self, rust_decimal::Error> {
        s.parse::<Decimal>().map(Self)
    }

    /// Returns `true` if the price is zero.
    #[inline]
    pub fn is_zero(&self) -> bool {
        self.0.is_zero()
    }

    /// Returns `true` if the price is positive (greater than zero).
    #[inline]
    pub fn is_positive(&self) -> bool {
        self.0.is_sign_positive() && !self.0.is_zero()
    }
}

impl fmt::Display for Price {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<Decimal> for Price {
    fn from(value: Decimal) -> Self {
        Self(value)
    }
}

impl From<Price> for Decimal {
    fn from(price: Price) -> Self {
        price.0
    }
}

impl FromStr for Price {
    type Err = rust_decimal::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

/// Amount type (zero-cost wrapper).
///
/// Represents the quantity of an asset using `Decimal` for precision.
/// Provides type safety via the newtype pattern.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Amount(pub Decimal);

impl Amount {
    /// Creates a new amount instance.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use ccxt_core::types::financial::Amount;
    /// use rust_decimal_macros::dec;
    ///
    /// let amount = Amount::new(dec!(0.1));
    /// ```
    pub fn new(value: Decimal) -> Self {
        Self(value)
    }

    /// Returns the inner `Decimal` value.
    #[inline]
    pub fn as_decimal(&self) -> Decimal {
        self.0
    }

    /// Parses an amount from a string.
    ///
    /// # Errors
    ///
    /// Returns an error if the string cannot be parsed as a valid `Decimal`.
    pub fn parse(s: &str) -> Result<Self, rust_decimal::Error> {
        s.parse::<Decimal>().map(Self)
    }

    /// Returns `true` if the amount is zero.
    #[inline]
    pub fn is_zero(&self) -> bool {
        self.0.is_zero()
    }

    /// Returns `true` if the amount is positive (greater than zero).
    #[inline]
    pub fn is_positive(&self) -> bool {
        self.0.is_sign_positive() && !self.0.is_zero()
    }

    /// Returns the absolute value of the amount.
    #[inline]
    pub fn abs(&self) -> Self {
        Self(self.0.abs())
    }
}

impl fmt::Display for Amount {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<Decimal> for Amount {
    fn from(value: Decimal) -> Self {
        Self(value)
    }
}

impl From<Amount> for Decimal {
    fn from(amount: Amount) -> Self {
        amount.0
    }
}

/// Enables using `.sum()` on `Amount` iterators.
impl std::iter::Sum for Amount {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        Self(iter.map(|a| a.0).sum())
    }
}

/// Enables using `.sum()` on `&Amount` iterators.
impl<'a> std::iter::Sum<&'a Amount> for Amount {
    fn sum<I: Iterator<Item = &'a Self>>(iter: I) -> Self {
        Self(iter.map(|a| a.0).sum())
    }
}

impl FromStr for Amount {
    type Err = rust_decimal::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

/// Cost type (zero-cost wrapper).
///
/// Represents the cost or total value of a trade, typically the result of price × amount.
/// Provides type safety via the newtype pattern.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Cost(pub Decimal);

impl Cost {
    /// Creates a new cost instance.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use ccxt_core::types::financial::Cost;
    /// use rust_decimal_macros::dec;
    ///
    /// let cost = Cost::new(dec!(5000.0));
    /// ```
    pub fn new(value: Decimal) -> Self {
        Self(value)
    }

    /// Returns the inner `Decimal` value.
    #[inline]
    pub fn as_decimal(&self) -> Decimal {
        self.0
    }

    /// Parses a cost from a string.
    ///
    /// # Errors
    ///
    /// Returns an error if the string cannot be parsed as a valid `Decimal`.
    pub fn parse(s: &str) -> Result<Self, rust_decimal::Error> {
        s.parse::<Decimal>().map(Self)
    }

    /// Returns `true` if the cost is zero.
    #[inline]
    pub fn is_zero(&self) -> bool {
        self.0.is_zero()
    }

    /// Returns `true` if the cost is positive (greater than zero).
    #[inline]
    pub fn is_positive(&self) -> bool {
        self.0.is_sign_positive() && !self.0.is_zero()
    }
}

impl Default for Cost {
    fn default() -> Self {
        Self(Decimal::ZERO)
    }
}

impl fmt::Display for Cost {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<Decimal> for Cost {
    fn from(value: Decimal) -> Self {
        Self(value)
    }
}

impl From<Cost> for Decimal {
    fn from(cost: Cost) -> Self {
        cost.0
    }
}

impl FromStr for Cost {
    type Err = rust_decimal::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

// ============================================================================
// Type-safe arithmetic operations
// ============================================================================

/// Price × Amount = Cost
///
/// This is the most fundamental relationship in financial calculations: price times amount equals cost.
/// Enforces this constraint through the type system to prevent unit errors.
impl std::ops::Mul<Amount> for Price {
    type Output = Cost;

    fn mul(self, rhs: Amount) -> Self::Output {
        Cost(self.0 * rhs.0)
    }
}

/// Amount × Price = Cost (multiplication commutativity)
impl std::ops::Mul<Price> for Amount {
    type Output = Cost;

    fn mul(self, rhs: Price) -> Self::Output {
        Cost(self.0 * rhs.0)
    }
}

/// Cost ÷ Amount = Price
///
/// Derives price from total cost and amount.
impl std::ops::Div<Amount> for Cost {
    type Output = Price;

    fn div(self, rhs: Amount) -> Self::Output {
        Price(self.0 / rhs.0)
    }
}

/// Cost ÷ Price = Amount
///
/// Derives amount from total cost and price.
impl std::ops::Div<Price> for Cost {
    type Output = Amount;

    fn div(self, rhs: Price) -> Self::Output {
        Amount(self.0 / rhs.0)
    }
}

// ============================================================================
// Same-type arithmetic operations
// ============================================================================

/// Price + Price = Price
impl std::ops::Add for Price {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}

/// Price - Price = Price
impl std::ops::Sub for Price {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0 - rhs.0)
    }
}

/// Amount + Amount = Amount
impl std::ops::Add for Amount {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}

/// Amount - Amount = Amount
impl std::ops::Sub for Amount {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0 - rhs.0)
    }
}

/// Cost + Cost = Cost
impl std::ops::Add for Cost {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}

/// Cost - Cost = Cost
impl std::ops::Sub for Cost {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0 - rhs.0)
    }
}

// ============================================================================
// Scalar operations
// ============================================================================

/// Price × Decimal = Price (scalar multiplication)
///
/// Used for multiplying prices by a factor, e.g., price adjustments.
impl std::ops::Mul<Decimal> for Price {
    type Output = Self;

    fn mul(self, rhs: Decimal) -> Self::Output {
        Self(self.0 * rhs)
    }
}

/// Decimal × Price = Price (scalar multiplication commutativity)
impl std::ops::Mul<Price> for Decimal {
    type Output = Price;

    fn mul(self, rhs: Price) -> Self::Output {
        Price(self * rhs.0)
    }
}

/// Price ÷ Decimal = Price (scalar division)
///
/// Used for dividing prices by a factor, e.g., calculating average prices.
impl std::ops::Div<Decimal> for Price {
    type Output = Self;

    fn div(self, rhs: Decimal) -> Self::Output {
        Self(self.0 / rhs)
    }
}

/// Price ÷ Price = Decimal (price ratio)
///
/// Used for calculating price ratios, spread percentages, etc.
impl std::ops::Div<Price> for Price {
    type Output = Decimal;

    fn div(self, rhs: Price) -> Self::Output {
        self.0 / rhs.0
    }
}

/// Amount × Decimal = Amount (scalar multiplication)
///
/// Used for multiplying amounts by a factor.
impl std::ops::Mul<Decimal> for Amount {
    type Output = Self;

    fn mul(self, rhs: Decimal) -> Self::Output {
        Self(self.0 * rhs)
    }
}

/// Decimal × Amount = Amount (scalar multiplication commutativity)
impl std::ops::Mul<Amount> for Decimal {
    type Output = Amount;

    fn mul(self, rhs: Amount) -> Self::Output {
        Amount(self * rhs.0)
    }
}

/// Amount ÷ Decimal = Amount (scalar division)
///
/// Used for dividing amounts by a factor.
impl std::ops::Div<Decimal> for Amount {
    type Output = Self;

    fn div(self, rhs: Decimal) -> Self::Output {
        Self(self.0 / rhs)
    }
}

/// Amount ÷ Amount = Decimal (amount ratio)
impl std::ops::Div<Amount> for Amount {
    type Output = Decimal;

    fn div(self, rhs: Amount) -> Self::Output {
        self.0 / rhs.0
    }
}

/// Cost × Decimal = Cost (scalar multiplication)
impl std::ops::Mul<Decimal> for Cost {
    type Output = Self;

    fn mul(self, rhs: Decimal) -> Self::Output {
        Self(self.0 * rhs)
    }
}

/// Decimal × Cost = Cost (scalar multiplication commutativity)
impl std::ops::Mul<Cost> for Decimal {
    type Output = Cost;

    fn mul(self, rhs: Cost) -> Self::Output {
        Cost(self * rhs.0)
    }
}

/// Cost ÷ Decimal = Cost (scalar division)
impl std::ops::Div<Decimal> for Cost {
    type Output = Self;

    fn div(self, rhs: Decimal) -> Self::Output {
        Self(self.0 / rhs)
    }
}

/// Divides two [`Cost`] values, yielding a ratio as [`Decimal`].
impl std::ops::Div<Cost> for Cost {
    type Output = Decimal;

    fn div(self, rhs: Cost) -> Self::Output {
        self.0 / rhs.0
    }
}

// ============================================================================
// Constant Comparison Methods
// ============================================================================

impl Price {
    /// Zero price constant.
    pub const ZERO: Self = Self(Decimal::ZERO);

    /// Checks if the price is greater than the given [`Decimal`] value.
    #[inline]
    pub fn gt(&self, other: Decimal) -> bool {
        self.0 > other
    }

    /// Checks if the price is less than the given [`Decimal`] value.
    #[inline]
    pub fn lt(&self, other: Decimal) -> bool {
        self.0 < other
    }

    /// Checks if the price is greater than or equal to the given [`Decimal`] value.
    #[inline]
    pub fn ge(&self, other: Decimal) -> bool {
        self.0 >= other
    }

    /// Checks if the price is less than or equal to the given [`Decimal`] value.
    #[inline]
    pub fn le(&self, other: Decimal) -> bool {
        self.0 <= other
    }
}

impl Amount {
    /// Zero amount constant.
    pub const ZERO: Self = Self(Decimal::ZERO);

    /// Checks if the amount is greater than the given [`Decimal`] value.
    #[inline]
    pub fn gt(&self, other: Decimal) -> bool {
        self.0 > other
    }

    /// Checks if the amount is less than the given [`Decimal`] value.
    #[inline]
    pub fn lt(&self, other: Decimal) -> bool {
        self.0 < other
    }

    /// Checks if the amount is greater than or equal to the given [`Decimal`] value.
    #[inline]
    pub fn ge(&self, other: Decimal) -> bool {
        self.0 >= other
    }

    /// Checks if the amount is less than or equal to the given [`Decimal`] value.
    #[inline]
    pub fn le(&self, other: Decimal) -> bool {
        self.0 <= other
    }
}

impl Cost {
    /// Zero cost constant.
    pub const ZERO: Self = Self(Decimal::ZERO);

    /// Checks if the cost is greater than the given [`Decimal`] value.
    #[inline]
    pub fn gt(&self, other: Decimal) -> bool {
        self.0 > other
    }

    /// Checks if the cost is less than the given [`Decimal`] value.
    #[inline]
    pub fn lt(&self, other: Decimal) -> bool {
        self.0 < other
    }

    /// Checks if the cost is greater than or equal to the given [`Decimal`] value.
    #[inline]
    pub fn ge(&self, other: Decimal) -> bool {
        self.0 >= other
    }

    /// Checks if the cost is less than or equal to the given [`Decimal`] value.
    #[inline]
    pub fn le(&self, other: Decimal) -> bool {
        self.0 <= other
    }
}

// ============================================================================
// Utility Functions
// ============================================================================

/// Decimal parsing and formatting utilities.
///
/// Provides convenience methods for converting between strings and [`Decimal`] values,
/// commonly needed when interfacing with exchange APIs.
pub mod decimal_utils {
    use rust_decimal::Decimal;
    use std::str::FromStr;

    /// Parses a [`Decimal`] from a string slice.
    ///
    /// Returns `None` if parsing fails instead of panicking.
    ///
    /// # Arguments
    ///
    /// * `s` - String slice to parse
    ///
    /// # Returns
    ///
    /// * `Some(Decimal)` - Successfully parsed decimal value
    /// * `None` - Parsing failed due to invalid format
    ///
    /// # Examples
    ///
    /// ```rust
    /// use ccxt_core::types::financial::decimal_utils;
    ///
    /// let value = decimal_utils::parse_decimal("50000.50");
    /// assert!(value.is_some());
    ///
    /// let invalid = decimal_utils::parse_decimal("invalid");
    /// assert!(invalid.is_none());
    /// ```
    pub fn parse_decimal(s: &str) -> Option<Decimal> {
        Decimal::from_str(s).ok()
    }

    /// Parses a [`Decimal`] from an optional string slice.
    ///
    /// Convenience method for handling optional string fields from API responses.
    ///
    /// # Arguments
    ///
    /// * `s` - Optional string slice to parse
    ///
    /// # Returns
    ///
    /// * `Some(Decimal)` - Successfully parsed decimal value
    /// * `None` - Input was `None` or parsing failed
    pub fn parse_optional_decimal(s: Option<&str>) -> Option<Decimal> {
        s.and_then(parse_decimal)
    }

    /// Formats a [`Decimal`] as a string suitable for API requests.
    ///
    /// # Arguments
    ///
    /// * `value` - Decimal value to format
    ///
    /// # Returns
    ///
    /// String representation of the decimal value
    ///
    /// # Examples
    ///
    /// ```rust
    /// use ccxt_core::types::financial::decimal_utils;
    /// use rust_decimal_macros::dec;
    ///
    /// let s = decimal_utils::format_decimal(dec!(50000.50));
    /// assert_eq!(s, "50000.50");
    /// ```
    pub fn format_decimal(value: Decimal) -> String {
        value.to_string()
    }

    /// Converts an `f64` to a [`Decimal`].
    ///
    /// # Arguments
    ///
    /// * `value` - Floating-point value to convert
    ///
    /// # Returns
    ///
    /// * `Some(Decimal)` - Successfully converted value
    /// * `None` - Conversion failed (e.g., NaN, infinity)
    ///
    /// # Warning
    ///
    /// Conversion from `f64` to [`Decimal`] may introduce precision errors due to
    /// floating-point representation limitations. Avoid using `f64` for financial
    /// calculations when possible; prefer parsing from strings instead.
    pub fn from_f64(value: f64) -> Option<Decimal> {
        Decimal::try_from(value).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_price_creation() {
        let price = Price::new(dec!(50000.0));
        assert_eq!(price.as_decimal(), dec!(50000.0));
        assert!(price.is_positive());
        assert!(!price.is_zero());
    }

    #[test]
    fn test_amount_creation() {
        let amount = Amount::new(dec!(0.1));
        assert_eq!(amount.as_decimal(), dec!(0.1));
        assert!(amount.is_positive());
    }

    #[test]
    fn test_cost_creation() {
        let cost = Cost::new(dec!(5000.0));
        assert_eq!(cost.as_decimal(), dec!(5000.0));
    }

    #[test]
    fn test_price_multiply_amount() {
        let price = Price::new(dec!(50000.0));
        let amount = Amount::new(dec!(0.1));
        let cost = price * amount;

        assert_eq!(cost.as_decimal(), dec!(5000.0));
    }

    #[test]
    fn test_amount_multiply_price() {
        let amount = Amount::new(dec!(0.1));
        let price = Price::new(dec!(50000.0));
        let cost = amount * price;

        assert_eq!(cost.as_decimal(), dec!(5000.0));
    }

    #[test]
    fn test_cost_divide_amount() {
        let cost = Cost::new(dec!(5000.0));
        let amount = Amount::new(dec!(0.1));
        let price = cost / amount;

        assert_eq!(price.as_decimal(), dec!(50000.0));
    }

    #[test]
    fn test_cost_divide_price() {
        let cost = Cost::new(dec!(5000.0));
        let price = Price::new(dec!(50000.0));
        let amount = cost / price;

        assert_eq!(amount.as_decimal(), dec!(0.1));
    }

    #[test]
    fn test_price_addition() {
        let price1 = Price::new(dec!(50000.0));
        let price2 = Price::new(dec!(1000.0));
        let total = price1 + price2;

        assert_eq!(total.as_decimal(), dec!(51000.0));
    }

    #[test]
    fn test_amount_subtraction() {
        let amount1 = Amount::new(dec!(1.0));
        let amount2 = Amount::new(dec!(0.3));
        let remaining = amount1 - amount2;

        assert_eq!(remaining.as_decimal(), dec!(0.7));
    }

    #[test]
    fn test_from_str() {
        let price = Price::from_str("50000.50").unwrap();
        assert_eq!(price.as_decimal(), dec!(50000.50));

        let amount = Amount::from_str("0.123456").unwrap();
        assert_eq!(amount.as_decimal(), dec!(0.123456));
    }

    #[test]
    fn test_display() {
        let price = Price::new(dec!(50000.50));
        assert_eq!(format!("{}", price), "50000.50");

        let amount = Amount::new(dec!(0.1));
        assert_eq!(format!("{}", amount), "0.1");
    }

    #[test]
    fn test_decimal_utils() {
        use decimal_utils::*;

        let value = parse_decimal("50000.50");
        assert!(value.is_some());
        assert_eq!(value.unwrap(), dec!(50000.50));

        let invalid = parse_decimal("invalid");
        assert!(invalid.is_none());

        let formatted = format_decimal(dec!(50000.50));
        assert_eq!(formatted, "50000.50");
    }

    #[test]
    fn test_serde_serialization() {
        use serde_json;

        let price = Price::new(dec!(50000.0));
        let json = serde_json::to_string(&price).unwrap();
        assert_eq!(json, "\"50000.0\"");

        let deserialized: Price = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, price);
    }

    #[test]
    fn test_zero_check() {
        let zero_price = Price::new(dec!(0));
        assert!(zero_price.is_zero());
        assert!(!zero_price.is_positive());

        let positive_price = Price::new(dec!(100));
        assert!(!positive_price.is_zero());
        assert!(positive_price.is_positive());
    }

    #[test]
    fn test_amount_abs() {
        let negative = Amount::new(dec!(-0.5));
        let positive = negative.abs();
        assert_eq!(positive.as_decimal(), dec!(0.5));
    }
    #[test]
    fn test_price_scalar_multiplication() {
        let price = Price::new(dec!(50000.0));
        let doubled = price * dec!(2);
        assert_eq!(doubled.as_decimal(), dec!(100000.0));

        // test commutativity
        let doubled2 = dec!(2) * price;
        assert_eq!(doubled2.as_decimal(), dec!(100000.0));
    }

    #[test]
    fn test_price_scalar_division() {
        let price = Price::new(dec!(50000.0));
        let half = price / dec!(2);
        assert_eq!(half.as_decimal(), dec!(25000.0));
    }

    #[test]
    fn test_price_ratio() {
        let price1 = Price::new(dec!(51000.0));
        let price2 = Price::new(dec!(50000.0));
        let ratio = price1 / price2;
        assert_eq!(ratio, dec!(1.02));
    }

    #[test]
    fn test_amount_scalar_operations() {
        let amount = Amount::new(dec!(1.0));

        let doubled = amount * dec!(2);
        assert_eq!(doubled.as_decimal(), dec!(2.0));

        let half = amount / dec!(2);
        assert_eq!(half.as_decimal(), dec!(0.5));

        let ratio = amount / Amount::new(dec!(0.5));
        assert_eq!(ratio, dec!(2.0));
    }

    #[test]
    fn test_spread_calculation() {
        let bid = Price::new(dec!(50000.0));
        let ask = Price::new(dec!(50100.0));

        let spread = ask - bid;
        assert_eq!(spread.as_decimal(), dec!(100.0));

        // Spread percentage: (ask - bid) / bid * 100
        let spread_pct = (ask - bid) / bid * dec!(100);
        assert_eq!(spread_pct, dec!(0.2));
    }

    #[test]
    fn test_mid_price_calculation() {
        let bid = Price::new(dec!(50000.0));
        let ask = Price::new(dec!(50100.0));

        let mid_price = (bid + ask) / dec!(2);
        assert_eq!(mid_price.as_decimal(), dec!(50050.0));
    }

    #[test]
    fn test_zero_constants() {
        assert_eq!(Price::ZERO, Price::new(dec!(0)));
        assert_eq!(Amount::ZERO, Amount::new(dec!(0)));
        assert_eq!(Cost::ZERO, Cost::new(dec!(0)));

        assert!(Price::ZERO.is_zero());
        assert!(Amount::ZERO.is_zero());
        assert!(Cost::ZERO.is_zero());
    }

    #[test]
    fn test_decimal_comparison() {
        let price = Price::new(dec!(50000.0));

        assert!(price.gt(Decimal::ZERO));
        assert!(price.gt(dec!(49999.0)));
        assert!(!price.gt(dec!(50001.0)));

        assert!(price.lt(dec!(50001.0)));
        assert!(!price.lt(dec!(49999.0)));

        assert!(price.ge(dec!(50000.0)));
        assert!(price.le(dec!(50000.0)));
    }

    #[test]
    fn test_cost_scalar_operations() {
        let cost = Cost::new(dec!(5000.0));

        let doubled = cost * dec!(2);
        assert_eq!(doubled.as_decimal(), dec!(10000.0));

        // Verify commutativity of multiplication
        let doubled2 = dec!(2) * cost;
        assert_eq!(doubled2.as_decimal(), dec!(10000.0));

        let half = cost / dec!(2);
        assert_eq!(half.as_decimal(), dec!(2500.0));

        let ratio = cost / Cost::new(dec!(2500.0));
        assert_eq!(ratio, dec!(2.0));
    }
}

// ============================================================================
// Property-Based Tests for Financial Types Unification
// ============================================================================

#[cfg(test)]
mod property_tests {
    use super::*;
    use proptest::prelude::*;
    use rust_decimal::Decimal;
    use rust_decimal_macros::dec;
    use std::str::FromStr;

    /// Strategy for generating valid Decimal values suitable for financial calculations.
    /// Avoids extreme values that could cause overflow or precision issues.
    fn decimal_strategy() -> impl Strategy<Value = Decimal> {
        // Generate decimals in a reasonable range for financial values
        // Using string-based generation to ensure precise decimal representation
        prop_oneof![
            // Integer values
            (-999_999_999i64..999_999_999i64).prop_map(Decimal::from),
            // Decimal values with various precisions (1-8 decimal places)
            (1u32..=8u32, -99_999_999i64..99_999_999i64).prop_map(|(scale, mantissa)| {
                let mut d = Decimal::from(mantissa);
                d.set_scale(scale).unwrap_or(());
                d
            }),
            // Small decimal values (0.xxx)
            (1i64..999_999i64, 1u32..=8u32).prop_map(|(mantissa, scale)| {
                let mut d = Decimal::from(mantissa);
                d.set_scale(scale).unwrap_or(());
                d
            }),
            // Common financial values
            Just(dec!(0)),
            Just(dec!(0.00000001)),
            Just(dec!(0.001)),
            Just(dec!(1.0)),
            Just(dec!(100.0)),
            Just(dec!(50000.0)),
            Just(dec!(99999.99999999)),
        ]
    }

    /// Strategy for generating positive Decimal values (for amounts and prices).
    fn positive_decimal_strategy() -> impl Strategy<Value = Decimal> {
        decimal_strategy().prop_filter("must be non-negative", |d| !d.is_sign_negative())
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        // ====================================================================
        // Property 1: Decimal to Amount/Price Conversion Round-Trip
        // ====================================================================
        // For any valid Decimal value, converting it to Amount (or Price) and
        // then back to Decimal using as_decimal() SHALL produce the original value.
        // **Feature: financial-types-unification, Property 1: Decimal to Amount/Price round-trip**
        // **Validates: Requirements 5.1, 5.2, 5.3**

        /// Test that Decimal -> Amount -> Decimal round-trip preserves the value
        #[test]
        fn prop_amount_decimal_roundtrip(value in decimal_strategy()) {
            let amount = Amount::new(value);
            let back = amount.as_decimal();
            prop_assert_eq!(back, value, "Amount round-trip should preserve value");
        }

        /// Test that Decimal -> Price -> Decimal round-trip preserves the value
        #[test]
        fn prop_price_decimal_roundtrip(value in decimal_strategy()) {
            let price = Price::new(value);
            let back = price.as_decimal();
            prop_assert_eq!(back, value, "Price round-trip should preserve value");
        }

        /// Test that Decimal -> Cost -> Decimal round-trip preserves the value
        #[test]
        fn prop_cost_decimal_roundtrip(value in decimal_strategy()) {
            let cost = Cost::new(value);
            let back = cost.as_decimal();
            prop_assert_eq!(back, value, "Cost round-trip should preserve value");
        }

        /// Test that From<Decimal> and Into<Decimal> are consistent
        #[test]
        fn prop_amount_from_into_consistent(value in decimal_strategy()) {
            let amount: Amount = value.into();
            let back: Decimal = amount.into();
            prop_assert_eq!(back, value, "Amount From/Into should be consistent");
        }

        /// Test that From<Decimal> and Into<Decimal> are consistent for Price
        #[test]
        fn prop_price_from_into_consistent(value in decimal_strategy()) {
            let price: Price = value.into();
            let back: Decimal = price.into();
            prop_assert_eq!(back, value, "Price From/Into should be consistent");
        }

        /// Test that From<Decimal> and Into<Decimal> are consistent for Cost
        #[test]
        fn prop_cost_from_into_consistent(value in decimal_strategy()) {
            let cost: Cost = value.into();
            let back: Decimal = cost.into();
            prop_assert_eq!(back, value, "Cost From/Into should be consistent");
        }

        // ====================================================================
        // Property 2: Serialization Produces Valid Decimal Strings
        // ====================================================================
        // For any valid Amount or Price value, calling to_string() SHALL produce
        // a string that:
        // - Does not contain scientific notation characters ('e' or 'E')
        // - Can be parsed back to the original Decimal value
        // **Feature: financial-types-unification, Property 2: Serialization produces valid decimal strings**
        // **Validates: Requirements 6.1, 6.2, 6.3**

        /// Test that Amount::to_string() produces valid decimal strings without scientific notation
        #[test]
        fn prop_amount_to_string_no_scientific_notation(value in decimal_strategy()) {
            let amount = Amount::new(value);
            let s = amount.to_string();

            // Should not contain scientific notation
            prop_assert!(
                !s.contains('e') && !s.contains('E'),
                "Amount string '{}' should not contain scientific notation",
                s
            );

            // Should be parseable back to the original value
            let parsed = Decimal::from_str(&s);
            prop_assert!(parsed.is_ok(), "Amount string '{}' should be parseable", s);
            prop_assert_eq!(
                parsed.unwrap(),
                value,
                "Parsed Amount should equal original value"
            );
        }

        /// Test that Price::to_string() produces valid decimal strings without scientific notation
        #[test]
        fn prop_price_to_string_no_scientific_notation(value in decimal_strategy()) {
            let price = Price::new(value);
            let s = price.to_string();

            // Should not contain scientific notation
            prop_assert!(
                !s.contains('e') && !s.contains('E'),
                "Price string '{}' should not contain scientific notation",
                s
            );

            // Should be parseable back to the original value
            let parsed = Decimal::from_str(&s);
            prop_assert!(parsed.is_ok(), "Price string '{}' should be parseable", s);
            prop_assert_eq!(
                parsed.unwrap(),
                value,
                "Parsed Price should equal original value"
            );
        }

        /// Test that Cost::to_string() produces valid decimal strings without scientific notation
        #[test]
        fn prop_cost_to_string_no_scientific_notation(value in decimal_strategy()) {
            let cost = Cost::new(value);
            let s = cost.to_string();

            // Should not contain scientific notation
            prop_assert!(
                !s.contains('e') && !s.contains('E'),
                "Cost string '{}' should not contain scientific notation",
                s
            );

            // Should be parseable back to the original value
            let parsed = Decimal::from_str(&s);
            prop_assert!(parsed.is_ok(), "Cost string '{}' should be parseable", s);
            prop_assert_eq!(
                parsed.unwrap(),
                value,
                "Parsed Cost should equal original value"
            );
        }

        /// Test that string round-trip preserves Amount values
        #[test]
        fn prop_amount_string_roundtrip(value in decimal_strategy()) {
            let amount = Amount::new(value);
            let s = amount.to_string();
            let parsed = Amount::from_str(&s);

            prop_assert!(parsed.is_ok(), "Amount string '{}' should be parseable", s);
            prop_assert_eq!(
                parsed.unwrap().as_decimal(),
                value,
                "Amount string round-trip should preserve value"
            );
        }

        /// Test that string round-trip preserves Price values
        #[test]
        fn prop_price_string_roundtrip(value in decimal_strategy()) {
            let price = Price::new(value);
            let s = price.to_string();
            let parsed = Price::from_str(&s);

            prop_assert!(parsed.is_ok(), "Price string '{}' should be parseable", s);
            prop_assert_eq!(
                parsed.unwrap().as_decimal(),
                value,
                "Price string round-trip should preserve value"
            );
        }

        // ====================================================================
        // Property 3: No Precision Loss in API Parameter Conversion
        // ====================================================================
        // For any valid Amount and Price values, the string representations
        // used in API requests SHALL preserve the full precision of the original values.
        // **Feature: financial-types-unification, Property 3: No precision loss in API parameter conversion**
        // **Validates: Requirements 1.4, 3.1**

        /// Test that Amount precision is preserved when converting to API string
        #[test]
        fn prop_amount_api_precision_preserved(value in positive_decimal_strategy()) {
            let amount = Amount::new(value);

            // Simulate API parameter conversion (what happens in create_order)
            let api_string = amount.to_string();

            // Parse back and verify precision is preserved
            let parsed = Decimal::from_str(&api_string).expect("Should parse");

            prop_assert_eq!(
                parsed,
                value,
                "Amount API string '{}' should preserve full precision of {}",
                api_string,
                value
            );
        }

        /// Test that Price precision is preserved when converting to API string
        #[test]
        fn prop_price_api_precision_preserved(value in positive_decimal_strategy()) {
            let price = Price::new(value);

            // Simulate API parameter conversion (what happens in create_order)
            let api_string = price.to_string();

            // Parse back and verify precision is preserved
            let parsed = Decimal::from_str(&api_string).expect("Should parse");

            prop_assert_eq!(
                parsed,
                value,
                "Price API string '{}' should preserve full precision of {}",
                api_string,
                value
            );
        }

        /// Test that Cost precision is preserved when converting to API string
        #[test]
        fn prop_cost_api_precision_preserved(value in positive_decimal_strategy()) {
            let cost = Cost::new(value);

            // Simulate API parameter conversion
            let api_string = cost.to_string();

            // Parse back and verify precision is preserved
            let parsed = Decimal::from_str(&api_string).expect("Should parse");

            prop_assert_eq!(
                parsed,
                value,
                "Cost API string '{}' should preserve full precision of {}",
                api_string,
                value
            );
        }

        /// Test that Price * Amount = Cost calculation preserves precision
        #[test]
        fn prop_price_amount_cost_precision(
            price_val in positive_decimal_strategy(),
            amount_val in positive_decimal_strategy()
        ) {
            // Skip if multiplication would overflow
            let price_f64 = price_val.to_string().parse::<f64>().unwrap_or(0.0);
            let amount_f64 = amount_val.to_string().parse::<f64>().unwrap_or(0.0);
            prop_assume!(price_f64 * amount_f64 < 1e20);

            let price = Price::new(price_val);
            let amount = Amount::new(amount_val);

            // Calculate cost
            let cost = price * amount;

            // Verify the calculation is correct
            let expected = price_val * amount_val;
            prop_assert_eq!(
                cost.as_decimal(),
                expected,
                "Price * Amount should equal expected Cost"
            );

            // Verify string representation preserves precision
            let cost_string = cost.to_string();
            let parsed = Decimal::from_str(&cost_string).expect("Should parse");
            prop_assert_eq!(
                parsed,
                expected,
                "Cost string should preserve calculation precision"
            );
        }

        /// Test that API parameter strings are valid for exchange APIs
        #[test]
        fn prop_api_strings_valid_format(value in positive_decimal_strategy()) {
            let amount = Amount::new(value);
            let price = Price::new(value);

            let amount_str = amount.to_string();
            let price_str = price.to_string();

            // API strings should only contain valid characters: digits, '.', and optional '-'
            let valid_chars = |c: char| c.is_ascii_digit() || c == '.' || c == '-';

            prop_assert!(
                amount_str.chars().all(valid_chars),
                "Amount string '{}' should only contain valid API characters",
                amount_str
            );

            prop_assert!(
                price_str.chars().all(valid_chars),
                "Price string '{}' should only contain valid API characters",
                price_str
            );

            // Should not have multiple decimal points
            prop_assert!(
                amount_str.matches('.').count() <= 1,
                "Amount string '{}' should have at most one decimal point",
                amount_str
            );

            prop_assert!(
                price_str.matches('.').count() <= 1,
                "Price string '{}' should have at most one decimal point",
                price_str
            );
        }
    }
}
