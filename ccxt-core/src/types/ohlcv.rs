//! OHLCV (candlestick) data type definitions.
//!
//! Provides standardized structures for candlestick chart data.

use serde::{Deserialize, Serialize};

/// OHLCV data structure (candlestick data).
///
/// Represents open, high, low, close prices and volume for a specific time period.
///
/// # Examples
/// ```rust
/// use ccxt_core::types::OHLCV;
///
/// let ohlcv = OHLCV {
///     timestamp: 1637000000000,
///     open: 50000.0,
///     high: 51000.0,
///     low: 49500.0,
///     close: 50500.0,
///     volume: 123.45,
/// };
///
/// println!("Candlestick data: {:?}", ohlcv);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OHLCV {
    /// Candlestick start time (Unix timestamp in milliseconds).
    pub timestamp: i64,

    /// Opening price.
    pub open: f64,

    /// Highest price.
    pub high: f64,

    /// Lowest price.
    pub low: f64,

    /// Closing price.
    pub close: f64,

    /// Trading volume (in base currency).
    pub volume: f64,
}

impl OHLCV {
    /// Creates a new OHLCV instance.
    ///
    /// # Arguments
    /// * `timestamp` - Candlestick timestamp in milliseconds
    /// * `open` - Opening price
    /// * `high` - Highest price
    /// * `low` - Lowest price
    /// * `close` - Closing price
    /// * `volume` - Trading volume
    pub fn new(timestamp: i64, open: f64, high: f64, low: f64, close: f64, volume: f64) -> Self {
        Self {
            timestamp,
            open,
            high,
            low,
            close,
            volume,
        }
    }

    /// Calculates the percentage change from open to close.
    ///
    /// Returns 0.0 if the opening price is zero to avoid division by zero.
    ///
    /// # Examples
    /// ```rust
    /// # use ccxt_core::types::OHLCV;
    /// let ohlcv = OHLCV::new(1637000000000, 100.0, 110.0, 95.0, 105.0, 1000.0);
    /// assert_eq!(ohlcv.change_percent(), 5.0);
    /// ```
    pub fn change_percent(&self) -> f64 {
        if self.open == 0.0 {
            return 0.0;
        }
        ((self.close - self.open) / self.open) * 100.0
    }

    /// Calculates the price range (high - low).
    pub fn price_range(&self) -> f64 {
        self.high - self.low
    }

    /// Calculates the amplitude percentage relative to the opening price.
    ///
    /// Returns 0.0 if the opening price is zero to avoid division by zero.
    pub fn amplitude_percent(&self) -> f64 {
        if self.open == 0.0 {
            return 0.0;
        }
        (self.price_range() / self.open) * 100.0
    }

    /// Returns `true` if this is a bullish candle (close >= open).
    pub fn is_bullish(&self) -> bool {
        self.close >= self.open
    }

    /// Returns `true` if this is a bearish candle (close < open).
    pub fn is_bearish(&self) -> bool {
        self.close < self.open
    }

    /// Calculates the body size (absolute difference between close and open).
    pub fn body_size(&self) -> f64 {
        (self.close - self.open).abs()
    }

    /// Calculates the upper shadow length (wick above the body).
    pub fn upper_shadow(&self) -> f64 {
        self.high - self.open.max(self.close)
    }

    /// Calculates the lower shadow length (wick below the body).
    pub fn lower_shadow(&self) -> f64 {
        self.open.min(self.close) - self.low
    }
}

impl Default for OHLCV {
    fn default() -> Self {
        Self {
            timestamp: 0,
            open: 0.0,
            high: 0.0,
            low: 0.0,
            close: 0.0,
            volume: 0.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ohlcv_creation() {
        let ohlcv = OHLCV::new(1637000000000, 100.0, 110.0, 95.0, 105.0, 1000.0);
        assert_eq!(ohlcv.timestamp, 1637000000000);
        assert_eq!(ohlcv.open, 100.0);
        assert_eq!(ohlcv.high, 110.0);
        assert_eq!(ohlcv.low, 95.0);
        assert_eq!(ohlcv.close, 105.0);
        assert_eq!(ohlcv.volume, 1000.0);
    }

    #[test]
    fn test_change_percent() {
        let ohlcv = OHLCV::new(1637000000000, 100.0, 110.0, 95.0, 105.0, 1000.0);
        assert_eq!(ohlcv.change_percent(), 5.0);

        let ohlcv_down = OHLCV::new(1637000000000, 100.0, 105.0, 90.0, 95.0, 1000.0);
        assert_eq!(ohlcv_down.change_percent(), -5.0);
    }

    #[test]
    fn test_price_range() {
        let ohlcv = OHLCV::new(1637000000000, 100.0, 110.0, 95.0, 105.0, 1000.0);
        assert_eq!(ohlcv.price_range(), 15.0);
    }

    #[test]
    fn test_amplitude_percent() {
        let ohlcv = OHLCV::new(1637000000000, 100.0, 110.0, 90.0, 105.0, 1000.0);
        assert_eq!(ohlcv.amplitude_percent(), 20.0);
    }

    #[test]
    fn test_is_bullish() {
        let bullish = OHLCV::new(1637000000000, 100.0, 110.0, 95.0, 105.0, 1000.0);
        assert!(bullish.is_bullish());

        let bearish = OHLCV::new(1637000000000, 100.0, 105.0, 90.0, 95.0, 1000.0);
        assert!(!bearish.is_bullish());
    }

    #[test]
    fn test_is_bearish() {
        let bearish = OHLCV::new(1637000000000, 100.0, 105.0, 90.0, 95.0, 1000.0);
        assert!(bearish.is_bearish());

        let bullish = OHLCV::new(1637000000000, 100.0, 110.0, 95.0, 105.0, 1000.0);
        assert!(!bullish.is_bearish());
    }

    #[test]
    fn test_body_size() {
        let ohlcv = OHLCV::new(1637000000000, 100.0, 110.0, 95.0, 105.0, 1000.0);
        assert_eq!(ohlcv.body_size(), 5.0);

        let ohlcv_down = OHLCV::new(1637000000000, 100.0, 105.0, 90.0, 95.0, 1000.0);
        assert_eq!(ohlcv_down.body_size(), 5.0);
    }

    #[test]
    fn test_shadows() {
        let ohlcv = OHLCV::new(1637000000000, 100.0, 110.0, 90.0, 105.0, 1000.0);
        assert_eq!(ohlcv.upper_shadow(), 5.0);
        assert_eq!(ohlcv.lower_shadow(), 10.0);
    }

    #[test]
    fn test_default() {
        let ohlcv = OHLCV::default();
        assert_eq!(ohlcv.timestamp, 0);
        assert_eq!(ohlcv.open, 0.0);
        assert_eq!(ohlcv.volume, 0.0);
    }
}
