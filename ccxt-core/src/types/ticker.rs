//! Ticker type definitions

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::{Amount, Price, Symbol, Timestamp};

/// Ticker data structure
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Ticker {
    /// Exchange symbol
    pub symbol: Symbol,

    /// Timestamp in milliseconds
    pub timestamp: Timestamp,

    /// ISO8601 datetime string
    pub datetime: Option<String>,

    /// Highest price in 24h
    pub high: Option<Price>,

    /// Lowest price in 24h
    pub low: Option<Price>,

    /// Current best bid price
    pub bid: Option<Price>,

    /// Current best bid amount
    #[serde(rename = "bidVolume")]
    pub bid_volume: Option<Amount>,

    /// Current best ask price
    pub ask: Option<Price>,

    /// Current best ask amount
    #[serde(rename = "askVolume")]
    pub ask_volume: Option<Amount>,

    /// Volume-weighted average price
    pub vwap: Option<Price>,

    /// Opening price
    pub open: Option<Price>,

    /// Closing price (current price)
    pub close: Option<Price>,

    /// Last traded price
    pub last: Option<Price>,

    /// Last traded price before current last
    #[serde(rename = "previousClose")]
    pub previous_close: Option<Price>,

    /// Absolute change (last - open)
    pub change: Option<Price>,

    /// Relative change ((last - open) / open)
    pub percentage: Option<Decimal>,

    /// Average price (high + low) / 2
    pub average: Option<Price>,

    /// Base volume traded in 24h
    #[serde(rename = "baseVolume")]
    pub base_volume: Option<Amount>,

    /// Quote volume traded in 24h
    #[serde(rename = "quoteVolume")]
    pub quote_volume: Option<Amount>,

    /// Funding rate (for derivatives)
    #[serde(rename = "fundingRate")]
    pub funding_rate: Option<Decimal>,

    /// Open interest (for derivatives)
    #[serde(rename = "openInterest")]
    pub open_interest: Option<Decimal>,

    /// Index price (for derivatives)
    #[serde(rename = "indexPrice")]
    pub index_price: Option<Price>,

    /// Mark price (for derivatives)
    #[serde(rename = "markPrice")]
    pub mark_price: Option<Price>,

    /// Raw exchange info
    #[serde(flatten)]
    pub info: HashMap<String, serde_json::Value>,
}

impl Ticker {
    /// Creates a new ticker with required fields.
    ///
    /// # Arguments
    ///
    /// * `symbol` - Trading pair symbol
    /// * `timestamp` - Ticker timestamp in milliseconds
    pub fn new(symbol: Symbol, timestamp: Timestamp) -> Self {
        Self {
            symbol,
            timestamp,
            datetime: None,
            high: None,
            low: None,
            bid: None,
            bid_volume: None,
            ask: None,
            ask_volume: None,
            vwap: None,
            open: None,
            close: None,
            last: None,
            previous_close: None,
            change: None,
            percentage: None,
            average: None,
            base_volume: None,
            quote_volume: None,
            funding_rate: None,
            open_interest: None,
            index_price: None,
            mark_price: None,
            info: HashMap::new(),
        }
    }

    /// Calculates the bid-ask spread.
    ///
    /// # Returns
    ///
    /// Returns `Some(Price)` if both bid and ask are available, otherwise `None`.
    pub fn spread(&self) -> Option<Price> {
        match (self.bid, self.ask) {
            (Some(bid), Some(ask)) => Some(ask - bid),
            _ => None,
        }
    }

    /// Calculates the spread as a percentage of the bid price.
    ///
    /// # Returns
    ///
    /// Returns the spread percentage if both bid and ask are available and bid > 0.
    pub fn spread_percentage(&self) -> Option<Decimal> {
        match (self.bid, self.ask) {
            (Some(bid), Some(ask)) if bid.as_decimal() > Decimal::ZERO => {
                let spread = ask.as_decimal() - bid.as_decimal();
                Some(spread / bid.as_decimal() * Decimal::from(100))
            }
            _ => None,
        }
    }

    /// Checks if ticker data is stale based on a time threshold.
    ///
    /// # Arguments
    ///
    /// * `threshold_ms` - Maximum age in milliseconds before data is considered stale
    ///
    /// # Returns
    ///
    /// Returns `true` if ticker age exceeds the threshold.
    pub fn is_stale(&self, threshold_ms: i64) -> bool {
        let now = chrono::Utc::now().timestamp_millis();
        now - self.timestamp > threshold_ms
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_ticker_creation() {
        let ticker = Ticker::new("BTC/USDT".to_string(), 1234567890);
        assert_eq!(ticker.symbol, "BTC/USDT");
        assert_eq!(ticker.timestamp, 1234567890);
    }

    #[test]
    fn test_spread_calculation() {
        let mut ticker = Ticker::new("BTC/USDT".to_string(), 1234567890);
        ticker.bid = Some(Price::new(dec!(50000)));
        ticker.ask = Some(Price::new(dec!(50100)));

        assert_eq!(ticker.spread(), Some(Price::new(dec!(100))));
        assert_eq!(ticker.spread_percentage(), Some(dec!(0.2)));
    }

    #[test]
    fn test_stale_check() {
        let old_timestamp = chrono::Utc::now().timestamp_millis() - 10000;
        let ticker = Ticker::new("BTC/USDT".to_string(), old_timestamp);

        assert!(ticker.is_stale(5000)); // 5 seconds threshold
        assert!(!ticker.is_stale(20000)); // 20 seconds threshold
    }
}
