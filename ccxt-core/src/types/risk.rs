//! Risk management type definitions.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Open interest data representing the total number of outstanding derivative contracts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenInterest {
    /// Raw API response data.
    pub info: Option<Value>,
    /// Trading pair symbol.
    pub symbol: String,
    /// Open interest amount (number of contracts).
    pub open_interest: f64,
    /// Notional value of open interest.
    pub open_interest_value: f64,
    /// Timestamp in milliseconds.
    pub timestamp: i64,
}

impl Default for OpenInterest {
    fn default() -> Self {
        Self {
            info: None,
            symbol: String::new(),
            open_interest: 0.0,
            open_interest_value: 0.0,
            timestamp: 0,
        }
    }
}

/// Historical open interest data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenInterestHistory {
    /// Raw API response data.
    pub info: Option<Value>,
    /// Trading pair symbol.
    pub symbol: String,
    /// Total open interest amount (number of contracts).
    pub sum_open_interest: f64,
    /// Total notional value.
    pub sum_open_interest_value: f64,
    /// Timestamp in milliseconds.
    pub timestamp: i64,
}

impl Default for OpenInterestHistory {
    fn default() -> Self {
        Self {
            info: None,
            symbol: String::new(),
            sum_open_interest: 0.0,
            sum_open_interest_value: 0.0,
            timestamp: 0,
        }
    }
}

/// Maximum available leverage information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaxLeverage {
    /// Raw API response data.
    pub info: Option<Value>,
    /// Trading pair symbol.
    pub symbol: String,
    /// Maximum leverage multiplier.
    pub max_leverage: i32,
    /// Notional value.
    pub notional: f64,
}

impl Default for MaxLeverage {
    fn default() -> Self {
        Self {
            info: None,
            symbol: String::new(),
            max_leverage: 0,
            notional: 0.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_open_interest_default() {
        let oi = OpenInterest::default();
        assert_eq!(oi.symbol, "");
        assert_eq!(oi.open_interest, 0.0);
        assert_eq!(oi.open_interest_value, 0.0);
        assert_eq!(oi.timestamp, 0);
    }

    #[test]
    fn test_open_interest_history_default() {
        let oih = OpenInterestHistory::default();
        assert_eq!(oih.symbol, "");
        assert_eq!(oih.sum_open_interest, 0.0);
        assert_eq!(oih.sum_open_interest_value, 0.0);
        assert_eq!(oih.timestamp, 0);
    }

    #[test]
    fn test_max_leverage_default() {
        let ml = MaxLeverage::default();
        assert_eq!(ml.symbol, "");
        assert_eq!(ml.max_leverage, 0);
        assert_eq!(ml.notional, 0.0);
    }

    #[test]
    fn test_open_interest_serialization() {
        let oi = OpenInterest {
            info: None,
            symbol: "BTC/USDT".to_string(),
            open_interest: 12345.67,
            open_interest_value: 500000000.0,
            timestamp: 1700000000000,
        };

        let json = serde_json::to_string(&oi).unwrap();
        assert!(json.contains("BTC/USDT"));
        assert!(json.contains("12345.67"));
    }
}
