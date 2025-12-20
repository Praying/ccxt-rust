//! Mark price type definitions.
//!
//! This module provides standardized structures for futures contract mark prices,
//! used for calculating unrealized PnL and liquidation prices.

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// Mark price data structure.
///
/// Represents the mark price and related information for futures contracts.
///
/// # Examples
/// ```rust
/// use ccxt_core::types::MarkPrice;
/// use rust_decimal::Decimal;
/// use rust_decimal_macros::dec;
///
/// let mark_price = MarkPrice {
///     symbol: "BTC/USDT".to_string(),
///     mark_price: dec!(50000),
///     index_price: Some(dec!(49995)),
///     estimated_settle_price: Some(dec!(50005)),
///     last_funding_rate: Some(dec!(0.0001)),
///     next_funding_time: Some(1637000000000),
///     interest_rate: Some(dec!(0.0003)),
///     timestamp: 1637000000000,
/// };
///
/// println!("Basis (mark - index): {}", mark_price.basis().unwrap_or(Decimal::ZERO));
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MarkPrice {
    /// Trading symbol (e.g., "BTC/USDT").
    pub symbol: String,

    /// Mark price (used for unrealized PnL calculation).
    pub mark_price: Decimal,

    /// Index price (spot index).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub index_price: Option<Decimal>,

    /// Estimated settlement price.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub estimated_settle_price: Option<Decimal>,

    /// Latest funding rate.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_funding_rate: Option<Decimal>,

    /// Next funding time timestamp in milliseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_funding_time: Option<i64>,

    /// Interest rate.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interest_rate: Option<Decimal>,

    /// Data timestamp in milliseconds.
    pub timestamp: i64,
}

impl MarkPrice {
    /// Creates a new mark price instance with all fields.
    ///
    /// # Arguments
    /// * `symbol` - Trading symbol
    /// * `mark_price` - Mark price
    /// * `index_price` - Index price
    /// * `estimated_settle_price` - Estimated settlement price
    /// * `last_funding_rate` - Latest funding rate
    /// * `next_funding_time` - Next funding time timestamp
    /// * `interest_rate` - Interest rate
    /// * `timestamp` - Data timestamp
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        symbol: String,
        mark_price: Decimal,
        index_price: Option<Decimal>,
        estimated_settle_price: Option<Decimal>,
        last_funding_rate: Option<Decimal>,
        next_funding_time: Option<i64>,
        interest_rate: Option<Decimal>,
        timestamp: i64,
    ) -> Self {
        Self {
            symbol,
            mark_price,
            index_price,
            estimated_settle_price,
            last_funding_rate,
            next_funding_time,
            interest_rate,
            timestamp,
        }
    }

    /// Creates a simplified mark price instance with only required fields.
    ///
    /// # Arguments
    /// * `symbol` - Trading symbol
    /// * `mark_price` - Mark price
    /// * `timestamp` - Data timestamp
    pub fn simple(symbol: String, mark_price: Decimal, timestamp: i64) -> Self {
        Self {
            symbol,
            mark_price,
            index_price: None,
            estimated_settle_price: None,
            last_funding_rate: None,
            next_funding_time: None,
            interest_rate: None,
            timestamp,
        }
    }

    /// Calculates the basis (mark price - index price).
    ///
    /// # Returns
    /// The basis value, or `None` if index price is not available.
    ///
    /// # Examples
    /// ```rust
    /// # use ccxt_core::types::MarkPrice;
    /// # use rust_decimal_macros::dec;
    /// let mark_price = MarkPrice::new(
    ///     "BTC/USDT".to_string(),
    ///     dec!(50000),
    ///     Some(dec!(49995)),
    ///     None, None, None, None,
    ///     1637000000000
    /// );
    /// assert_eq!(mark_price.basis(), Some(dec!(5)));
    /// ```
    pub fn basis(&self) -> Option<Decimal> {
        self.index_price.map(|ip| self.mark_price - ip)
    }

    /// Calculates the basis rate (percentage).
    ///
    /// # Returns
    /// The basis rate as a percentage, or `None` if index price is unavailable or zero.
    ///
    /// # Examples
    /// ```rust
    /// # use ccxt_core::types::MarkPrice;
    /// # use rust_decimal_macros::dec;
    /// let mark_price = MarkPrice::new(
    ///     "BTC/USDT".to_string(),
    ///     dec!(50100),
    ///     Some(dec!(50000)),
    ///     None, None, None, None,
    ///     1637000000000
    /// );
    /// assert_eq!(mark_price.basis_rate(), Some(dec!(0.2)));
    /// ```
    pub fn basis_rate(&self) -> Option<Decimal> {
        self.index_price.and_then(|ip| {
            if ip == Decimal::ZERO {
                None
            } else {
                Some((self.mark_price - ip) / ip * Decimal::ONE_HUNDRED)
            }
        })
    }

    /// Returns the funding rate as a percentage.
    ///
    /// # Returns
    /// The funding rate percentage, or `None` if not available.
    pub fn funding_rate_percent(&self) -> Option<Decimal> {
        self.last_funding_rate.map(|fr| fr * Decimal::ONE_HUNDRED)
    }

    /// Calculates the time until the next funding in seconds.
    ///
    /// # Returns
    /// The seconds until next funding, or `None` if not available.
    pub fn time_to_next_funding(&self) -> Option<i64> {
        self.next_funding_time
            .map(|nft| (nft - self.timestamp) / 1000)
    }

    /// Checks if the mark price deviation from index price exceeds a threshold.
    ///
    /// # Arguments
    /// * `threshold_percent` - Threshold percentage (e.g., dec!(1) for 1%)
    ///
    /// # Returns
    /// `true` if deviation exceeds threshold, `false` if index price is unavailable.
    pub fn is_deviation_excessive(&self, threshold_percent: Decimal) -> bool {
        self.basis_rate()
            .map(|rate| rate.abs() > threshold_percent)
            .unwrap_or(false)
    }

    /// Checks if the mark price is at a premium (mark price > index price).
    ///
    /// # Returns
    /// `true` if at premium, `false` if index price is unavailable.
    pub fn is_premium(&self) -> bool {
        self.basis().map(|b| b > Decimal::ZERO).unwrap_or(false)
    }

    /// Checks if the mark price is at a discount (mark price < index price).
    ///
    /// # Returns
    /// `true` if at discount, `false` if index price is unavailable.
    pub fn is_discount(&self) -> bool {
        self.basis().map(|b| b < Decimal::ZERO).unwrap_or(false)
    }

    /// Checks if the funding rate is positive.
    ///
    /// # Returns
    /// `true` if funding rate is positive, `false` if not available.
    pub fn is_funding_positive(&self) -> bool {
        self.last_funding_rate
            .map(|fr| fr > Decimal::ZERO)
            .unwrap_or(false)
    }

    /// Checks if the data is valid.
    ///
    /// # Returns
    /// `true` if mark price is greater than zero.
    pub fn is_valid(&self) -> bool {
        self.mark_price > Decimal::ZERO
    }
}

impl Default for MarkPrice {
    fn default() -> Self {
        Self {
            symbol: String::new(),
            mark_price: Decimal::ZERO,
            index_price: None,
            estimated_settle_price: None,
            last_funding_rate: None,
            next_funding_time: None,
            interest_rate: None,
            timestamp: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_mark_price_creation() {
        let mark_price = MarkPrice::new(
            "BTC/USDT".to_string(),
            dec!(50000),
            Some(dec!(49995)),
            Some(dec!(50005)),
            Some(dec!(0.0001)),
            Some(1637000000000),
            Some(dec!(0.0003)),
            1637000000000,
        );

        assert_eq!(mark_price.symbol, "BTC/USDT");
        assert_eq!(mark_price.mark_price, dec!(50000));
        assert_eq!(mark_price.index_price, Some(dec!(49995)));
        assert_eq!(mark_price.last_funding_rate, Some(dec!(0.0001)));
    }

    #[test]
    fn test_simple_creation() {
        let mark_price = MarkPrice::simple("BTC/USDT".to_string(), dec!(50000), 1637000000000);

        assert_eq!(mark_price.symbol, "BTC/USDT");
        assert_eq!(mark_price.mark_price, dec!(50000));
        assert_eq!(mark_price.index_price, None);
    }

    #[test]
    fn test_basis() {
        let mark_price = MarkPrice::new(
            "BTC/USDT".to_string(),
            dec!(50000),
            Some(dec!(49995)),
            None,
            None,
            None,
            None,
            1637000000000,
        );
        assert_eq!(mark_price.basis(), Some(dec!(5)));

        let no_index = MarkPrice::simple("BTC/USDT".to_string(), dec!(50000), 1637000000000);
        assert_eq!(no_index.basis(), None);
    }

    #[test]
    fn test_basis_rate() {
        let mark_price = MarkPrice::new(
            "BTC/USDT".to_string(),
            dec!(50100),
            Some(dec!(50000)),
            None,
            None,
            None,
            None,
            1637000000000,
        );
        assert_eq!(mark_price.basis_rate(), Some(dec!(0.2)));
    }

    #[test]
    fn test_funding_rate_percent() {
        let mark_price = MarkPrice::new(
            "BTC/USDT".to_string(),
            dec!(50000),
            None,
            None,
            Some(dec!(0.0001)),
            None,
            None,
            1637000000000,
        );
        assert_eq!(mark_price.funding_rate_percent(), Some(dec!(0.01)));
    }

    #[test]
    fn test_time_to_next_funding() {
        let mark_price = MarkPrice::new(
            "BTC/USDT".to_string(),
            dec!(50000),
            None,
            None,
            None,
            Some(1637008000000), // 8000秒后
            None,
            1637000000000,
        );
        assert_eq!(mark_price.time_to_next_funding(), Some(8000));
    }

    #[test]
    fn test_is_deviation_excessive() {
        let mark_price = MarkPrice::new(
            "BTC/USDT".to_string(),
            dec!(50600), // 1.2% 偏离
            Some(dec!(50000)),
            None,
            None,
            None,
            None,
            1637000000000,
        );
        assert!(mark_price.is_deviation_excessive(dec!(1)));
        assert!(!mark_price.is_deviation_excessive(dec!(2)));
    }

    #[test]
    fn test_is_premium() {
        let premium = MarkPrice::new(
            "BTC/USDT".to_string(),
            dec!(50100),
            Some(dec!(50000)),
            None,
            None,
            None,
            None,
            1637000000000,
        );
        assert!(premium.is_premium());
        assert!(!premium.is_discount());
    }

    #[test]
    fn test_is_discount() {
        let discount = MarkPrice::new(
            "BTC/USDT".to_string(),
            dec!(49900),
            Some(dec!(50000)),
            None,
            None,
            None,
            None,
            1637000000000,
        );
        assert!(discount.is_discount());
        assert!(!discount.is_premium());
    }

    #[test]
    fn test_is_funding_positive() {
        let positive = MarkPrice::new(
            "BTC/USDT".to_string(),
            dec!(50000),
            None,
            None,
            Some(dec!(0.0001)),
            None,
            None,
            1637000000000,
        );
        assert!(positive.is_funding_positive());

        let negative = MarkPrice::new(
            "BTC/USDT".to_string(),
            dec!(50000),
            None,
            None,
            Some(dec!(-0.0001)),
            None,
            None,
            1637000000000,
        );
        assert!(!negative.is_funding_positive());
    }

    #[test]
    fn test_is_valid() {
        let valid = MarkPrice::simple("BTC/USDT".to_string(), dec!(50000), 1637000000000);
        assert!(valid.is_valid());

        let invalid = MarkPrice::simple("BTC/USDT".to_string(), Decimal::ZERO, 1637000000000);
        assert!(!invalid.is_valid());
    }

    #[test]
    fn test_default() {
        let mark_price = MarkPrice::default();
        assert_eq!(mark_price.symbol, "");
        assert_eq!(mark_price.mark_price, Decimal::ZERO);
        assert_eq!(mark_price.index_price, None);
        assert!(!mark_price.is_valid());
    }
}
