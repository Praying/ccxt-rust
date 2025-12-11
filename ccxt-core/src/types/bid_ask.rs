//! Bid-ask (best bid and ask prices) type definitions.
//!
//! Provides standardized structures for book ticker data.

use serde::{Deserialize, Serialize};

/// Best bid and ask prices data structure.
///
/// Represents the current best bid and ask prices for a trading pair.
///
/// # Examples
/// ```rust
/// use ccxt_core::types::BidAsk;
///
/// let bid_ask = BidAsk {
///     symbol: "BTC/USDT".to_string(),
///     bid_price: 50000.0,
///     bid_quantity: 1.5,
///     ask_price: 50010.0,
///     ask_quantity: 2.0,
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
    pub bid_price: f64,

    /// Best bid quantity.
    pub bid_quantity: f64,

    /// Best ask price.
    pub ask_price: f64,

    /// Best ask quantity.
    pub ask_quantity: f64,

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
        bid_price: f64,
        bid_quantity: f64,
        ask_price: f64,
        ask_quantity: f64,
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
    /// let bid_ask = BidAsk::new(
    ///     "BTC/USDT".to_string(),
    ///     50000.0, 1.5,
    ///     50010.0, 2.0,
    ///     1637000000000
    /// );
    /// assert_eq!(bid_ask.spread(), 10.0);
    /// ```
    pub fn spread(&self) -> f64 {
        self.ask_price - self.bid_price
    }

    /// Calculates the bid-ask spread as a percentage.
    ///
    /// # Returns
    ///
    /// The spread percentage, or 0.0 if bid_price is 0.
    ///
    /// # Examples
    /// ```rust
    /// # use ccxt_core::types::BidAsk;
    /// let bid_ask = BidAsk::new(
    ///     "BTC/USDT".to_string(),
    ///     50000.0, 1.5,
    ///     50100.0, 2.0,
    ///     1637000000000
    /// );
    /// assert_eq!(bid_ask.spread_percent(), 0.2);
    /// ```
    pub fn spread_percent(&self) -> f64 {
        if self.bid_price == 0.0 {
            return 0.0;
        }
        (self.spread() / self.bid_price) * 100.0
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
    /// let bid_ask = BidAsk::new(
    ///     "BTC/USDT".to_string(),
    ///     50000.0, 1.5,
    ///     50100.0, 2.0,
    ///     1637000000000
    /// );
    /// assert_eq!(bid_ask.mid_price(), 50050.0);
    /// ```
    pub fn mid_price(&self) -> f64 {
        (self.bid_price + self.ask_price) / 2.0
    }

    /// Calculates the total bid value (bid_price × bid_quantity).
    ///
    /// # Returns
    ///
    /// The total bid value.
    pub fn bid_value(&self) -> f64 {
        self.bid_price * self.bid_quantity
    }

    /// Calculates the total ask value (ask_price × ask_quantity).
    ///
    /// # Returns
    ///
    /// The total ask value.
    pub fn ask_value(&self) -> f64 {
        self.ask_price * self.ask_quantity
    }

    /// Calculates the bid-to-ask quantity ratio.
    ///
    /// # Returns
    ///
    /// The bid-to-ask quantity ratio (bid_quantity / ask_quantity), or 0.0 if ask_quantity is 0.
    pub fn quantity_ratio(&self) -> f64 {
        if self.ask_quantity == 0.0 {
            return 0.0;
        }
        self.bid_quantity / self.ask_quantity
    }

    /// Checks if the data is valid.
    ///
    /// # Returns
    ///
    /// Returns `true` if all prices and quantities are greater than 0 and ask_price >= bid_price.
    pub fn is_valid(&self) -> bool {
        self.bid_price > 0.0
            && self.bid_quantity > 0.0
            && self.ask_price > 0.0
            && self.ask_quantity > 0.0
            && self.ask_price >= self.bid_price
    }
}

impl Default for BidAsk {
    fn default() -> Self {
        Self {
            symbol: String::new(),
            bid_price: 0.0,
            bid_quantity: 0.0,
            ask_price: 0.0,
            ask_quantity: 0.0,
            timestamp: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bid_ask_creation() {
        let bid_ask = BidAsk::new(
            "BTC/USDT".to_string(),
            50000.0,
            1.5,
            50010.0,
            2.0,
            1637000000000,
        );

        assert_eq!(bid_ask.symbol, "BTC/USDT");
        assert_eq!(bid_ask.bid_price, 50000.0);
        assert_eq!(bid_ask.bid_quantity, 1.5);
        assert_eq!(bid_ask.ask_price, 50010.0);
        assert_eq!(bid_ask.ask_quantity, 2.0);
        assert_eq!(bid_ask.timestamp, 1637000000000);
    }

    #[test]
    fn test_spread() {
        let bid_ask = BidAsk::new(
            "BTC/USDT".to_string(),
            50000.0,
            1.5,
            50010.0,
            2.0,
            1637000000000,
        );
        assert_eq!(bid_ask.spread(), 10.0);
    }

    #[test]
    fn test_spread_percent() {
        let bid_ask = BidAsk::new(
            "BTC/USDT".to_string(),
            50000.0,
            1.5,
            50100.0,
            2.0,
            1637000000000,
        );
        assert_eq!(bid_ask.spread_percent(), 0.2);
    }

    #[test]
    fn test_mid_price() {
        let bid_ask = BidAsk::new(
            "BTC/USDT".to_string(),
            50000.0,
            1.5,
            50100.0,
            2.0,
            1637000000000,
        );
        assert_eq!(bid_ask.mid_price(), 50050.0);
    }

    #[test]
    fn test_bid_value() {
        let bid_ask = BidAsk::new(
            "BTC/USDT".to_string(),
            50000.0,
            1.5,
            50100.0,
            2.0,
            1637000000000,
        );
        assert_eq!(bid_ask.bid_value(), 75000.0);
    }

    #[test]
    fn test_ask_value() {
        let bid_ask = BidAsk::new(
            "BTC/USDT".to_string(),
            50000.0,
            1.5,
            50100.0,
            2.0,
            1637000000000,
        );
        assert_eq!(bid_ask.ask_value(), 100200.0);
    }

    #[test]
    fn test_quantity_ratio() {
        let bid_ask = BidAsk::new(
            "BTC/USDT".to_string(),
            50000.0,
            1.5,
            50100.0,
            2.0,
            1637000000000,
        );
        assert_eq!(bid_ask.quantity_ratio(), 0.75);
    }

    #[test]
    fn test_is_valid() {
        let valid = BidAsk::new(
            "BTC/USDT".to_string(),
            50000.0,
            1.5,
            50100.0,
            2.0,
            1637000000000,
        );
        assert!(valid.is_valid());

        let invalid = BidAsk::new(
            "BTC/USDT".to_string(),
            50100.0,
            1.5,
            50000.0,
            2.0,
            1637000000000,
        );
        assert!(!invalid.is_valid());

        let zero_price = BidAsk::new(
            "BTC/USDT".to_string(),
            0.0,
            1.5,
            50000.0,
            2.0,
            1637000000000,
        );
        assert!(!zero_price.is_valid());
    }

    #[test]
    fn test_default() {
        let bid_ask = BidAsk::default();
        assert_eq!(bid_ask.symbol, "");
        assert_eq!(bid_ask.bid_price, 0.0);
        assert_eq!(bid_ask.timestamp, 0);
        assert!(!bid_ask.is_valid());
    }
}
