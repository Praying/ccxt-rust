//! Bid-ask (best bid and ask prices) type definitions.
//!
//! Provides standardized structures for book ticker data.

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// Best bid and ask prices data structure.
///
/// Represents the current best bid and ask prices for a trading pair.
/// Uses `Decimal` for precise financial calculations.
///
/// # Examples
/// ```rust
/// use ccxt_core::types::BidAsk;
/// use rust_decimal_macros::dec;
///
/// let bid_ask = BidAsk {
///     symbol: "BTC/USDT".to_string(),
///     bid_price: dec!(50000),
///     bid_quantity: dec!(1.5),
///     ask_price: dec!(50010),
///     ask_quantity: dec!(2.0),
///     timestamp: 1637000000000,
/// };
///
/// println!("Spread: {}", bid_ask.spread());
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BidAsk {
    /// Trading pair symbol (e.g., "BTC/USDT").
    pub symbol: String,

    /// Best bid price.
    pub bid_price: Decimal,

    /// Best bid quantity.
    pub bid_quantity: Decimal,

    /// Best ask price.
    pub ask_price: Decimal,

    /// Best ask quantity.
    pub ask_quantity: Decimal,

    /// Data timestamp in milliseconds (Unix timestamp).
    pub timestamp: i64,
}

impl BidAsk {
    /// Creates a new BidAsk instance.
    ///
    /// # Arguments
    ///
    /// * `symbol` - Trading pair symbol
    /// * `bid_price` - Best bid price
    /// * `bid_quantity` - Best bid quantity
    /// * `ask_price` - Best ask price
    /// * `ask_quantity` - Best ask quantity
    /// * `timestamp` - Data timestamp
    pub fn new(
        symbol: String,
        bid_price: Decimal,
        bid_quantity: Decimal,
        ask_price: Decimal,
        ask_quantity: Decimal,
        timestamp: i64,
    ) -> Self {
        Self {
            symbol,
            bid_price,
            bid_quantity,
            ask_price,
            ask_quantity,
            timestamp,
        }
    }

    /// Calculates the bid-ask spread (absolute value).
    ///
    /// # Returns
    ///
    /// The spread (ask_price - bid_price).
    ///
    /// # Examples
    /// ```rust
    /// # use ccxt_core::types::BidAsk;
    /// # use rust_decimal_macros::dec;
    /// let bid_ask = BidAsk::new(
    ///     "BTC/USDT".to_string(),
    ///     dec!(50000), dec!(1.5),
    ///     dec!(50010), dec!(2.0),
    ///     1637000000000
    /// );
    /// assert_eq!(bid_ask.spread(), dec!(10));
    /// ```
    pub fn spread(&self) -> Decimal {
        self.ask_price - self.bid_price
    }

    /// Calculates the bid-ask spread as a percentage.
    ///
    /// # Returns
    ///
    /// The spread percentage, or Decimal::ZERO if bid_price is 0.
    ///
    /// # Examples
    /// ```rust
    /// # use ccxt_core::types::BidAsk;
    /// # use rust_decimal_macros::dec;
    /// let bid_ask = BidAsk::new(
    ///     "BTC/USDT".to_string(),
    ///     dec!(50000), dec!(1.5),
    ///     dec!(50100), dec!(2.0),
    ///     1637000000000
    /// );
    /// assert_eq!(bid_ask.spread_percent(), dec!(0.2));
    /// ```
    pub fn spread_percent(&self) -> Decimal {
        if self.bid_price.is_zero() {
            return Decimal::ZERO;
        }
        (self.spread() / self.bid_price) * Decimal::ONE_HUNDRED
    }

    /// Calculates the mid price (average of bid and ask prices).
    ///
    /// # Returns
    ///
    /// The mid price.
    ///
    /// # Examples
    /// ```rust
    /// # use ccxt_core::types::BidAsk;
    /// # use rust_decimal_macros::dec;
    /// let bid_ask = BidAsk::new(
    ///     "BTC/USDT".to_string(),
    ///     dec!(50000), dec!(1.5),
    ///     dec!(50100), dec!(2.0),
    ///     1637000000000
    /// );
    /// assert_eq!(bid_ask.mid_price(), dec!(50050));
    /// ```
    pub fn mid_price(&self) -> Decimal {
        (self.bid_price + self.ask_price) / Decimal::TWO
    }

    /// Calculates the total bid value (bid_price × bid_quantity).
    ///
    /// # Returns
    ///
    /// The total bid value.
    pub fn bid_value(&self) -> Decimal {
        self.bid_price * self.bid_quantity
    }

    /// Calculates the total ask value (ask_price × ask_quantity).
    ///
    /// # Returns
    ///
    /// The total ask value.
    pub fn ask_value(&self) -> Decimal {
        self.ask_price * self.ask_quantity
    }

    /// Calculates the bid-to-ask quantity ratio.
    ///
    /// # Returns
    ///
    /// The bid-to-ask quantity ratio (bid_quantity / ask_quantity), or Decimal::ZERO if ask_quantity is 0.
    pub fn quantity_ratio(&self) -> Decimal {
        if self.ask_quantity.is_zero() {
            return Decimal::ZERO;
        }
        self.bid_quantity / self.ask_quantity
    }

    /// Checks if the data is valid.
    ///
    /// # Returns
    ///
    /// Returns `true` if all prices and quantities are greater than 0 and ask_price >= bid_price.
    pub fn is_valid(&self) -> bool {
        self.bid_price > Decimal::ZERO
            && self.bid_quantity > Decimal::ZERO
            && self.ask_price > Decimal::ZERO
            && self.ask_quantity > Decimal::ZERO
            && self.ask_price >= self.bid_price
    }
}

impl Default for BidAsk {
    fn default() -> Self {
        Self {
            symbol: String::new(),
            bid_price: Decimal::ZERO,
            bid_quantity: Decimal::ZERO,
            ask_price: Decimal::ZERO,
            ask_quantity: Decimal::ZERO,
            timestamp: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_bid_ask_creation() {
        let bid_ask = BidAsk::new(
            "BTC/USDT".to_string(),
            dec!(50000),
            dec!(1.5),
            dec!(50010),
            dec!(2.0),
            1637000000000,
        );

        assert_eq!(bid_ask.symbol, "BTC/USDT");
        assert_eq!(bid_ask.bid_price, dec!(50000));
        assert_eq!(bid_ask.bid_quantity, dec!(1.5));
        assert_eq!(bid_ask.ask_price, dec!(50010));
        assert_eq!(bid_ask.ask_quantity, dec!(2.0));
        assert_eq!(bid_ask.timestamp, 1637000000000);
    }

    #[test]
    fn test_spread() {
        let bid_ask = BidAsk::new(
            "BTC/USDT".to_string(),
            dec!(50000),
            dec!(1.5),
            dec!(50010),
            dec!(2.0),
            1637000000000,
        );
        assert_eq!(bid_ask.spread(), dec!(10));
    }

    #[test]
    fn test_spread_percent() {
        let bid_ask = BidAsk::new(
            "BTC/USDT".to_string(),
            dec!(50000),
            dec!(1.5),
            dec!(50100),
            dec!(2.0),
            1637000000000,
        );
        assert_eq!(bid_ask.spread_percent(), dec!(0.2));
    }

    #[test]
    fn test_mid_price() {
        let bid_ask = BidAsk::new(
            "BTC/USDT".to_string(),
            dec!(50000),
            dec!(1.5),
            dec!(50100),
            dec!(2.0),
            1637000000000,
        );
        assert_eq!(bid_ask.mid_price(), dec!(50050));
    }

    #[test]
    fn test_bid_value() {
        let bid_ask = BidAsk::new(
            "BTC/USDT".to_string(),
            dec!(50000),
            dec!(1.5),
            dec!(50100),
            dec!(2.0),
            1637000000000,
        );
        assert_eq!(bid_ask.bid_value(), dec!(75000));
    }

    #[test]
    fn test_ask_value() {
        let bid_ask = BidAsk::new(
            "BTC/USDT".to_string(),
            dec!(50000),
            dec!(1.5),
            dec!(50100),
            dec!(2.0),
            1637000000000,
        );
        assert_eq!(bid_ask.ask_value(), dec!(100200));
    }

    #[test]
    fn test_quantity_ratio() {
        let bid_ask = BidAsk::new(
            "BTC/USDT".to_string(),
            dec!(50000),
            dec!(1.5),
            dec!(50100),
            dec!(2.0),
            1637000000000,
        );
        assert_eq!(bid_ask.quantity_ratio(), dec!(0.75));
    }

    #[test]
    fn test_is_valid() {
        let valid = BidAsk::new(
            "BTC/USDT".to_string(),
            dec!(50000),
            dec!(1.5),
            dec!(50100),
            dec!(2.0),
            1637000000000,
        );
        assert!(valid.is_valid());

        let invalid = BidAsk::new(
            "BTC/USDT".to_string(),
            dec!(50100),
            dec!(1.5),
            dec!(50000),
            dec!(2.0),
            1637000000000,
        );
        assert!(!invalid.is_valid());

        let zero_price = BidAsk::new(
            "BTC/USDT".to_string(),
            Decimal::ZERO,
            dec!(1.5),
            dec!(50000),
            dec!(2.0),
            1637000000000,
        );
        assert!(!zero_price.is_valid());
    }

    #[test]
    fn test_default() {
        let bid_ask = BidAsk::default();
        assert_eq!(bid_ask.symbol, "");
        assert_eq!(bid_ask.bid_price, Decimal::ZERO);
        assert_eq!(bid_ask.timestamp, 0);
        assert!(!bid_ask.is_valid());
    }
}
