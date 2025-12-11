//! Margin trading related types
//!
//! This module contains data structures for margin trading operations,
//! including borrow rates, loans, repayments, and historical records.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::Timestamp;

/// Borrow rate information for margin trading
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BorrowRate {
    /// Currency code (e.g., "BTC", "USDT")
    pub currency: String,

    /// Trading symbol for isolated margin (e.g., "BTC/USDT")
    pub symbol: Option<String>,

    /// Daily interest rate
    pub rate: f64,

    /// Timestamp in milliseconds
    pub timestamp: Timestamp,

    /// ISO 8601 datetime string
    pub datetime: String,

    /// Whether this is for isolated margin
    pub is_isolated: bool,

    /// Raw response data from the exchange
    pub info: Value,
}

impl BorrowRate {
    /// Create a new borrow rate for cross margin
    pub fn new_cross(currency: String, rate: f64, timestamp: Timestamp, info: Value) -> Self {
        let datetime = crate::time::iso8601(timestamp).unwrap_or_default();
        Self {
            currency,
            symbol: None,
            rate,
            timestamp,
            datetime,
            is_isolated: false,
            info,
        }
    }

    /// Create a new borrow rate for isolated margin
    pub fn new_isolated(
        currency: String,
        symbol: String,
        rate: f64,
        timestamp: Timestamp,
        info: Value,
    ) -> Self {
        let datetime = crate::time::iso8601(timestamp).unwrap_or_default();
        Self {
            currency,
            symbol: Some(symbol),
            rate,
            timestamp,
            datetime,
            is_isolated: true,
            info,
        }
    }
}

/// Isolated borrow rate information for margin trading
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct IsolatedBorrowRate {
    /// Trading symbol (e.g., "BTC/USDT")
    pub symbol: String,

    /// Base currency code
    pub base: String,

    /// Base currency daily interest rate
    pub base_rate: f64,

    /// Quote currency code
    pub quote: String,

    /// Quote currency daily interest rate
    pub quote_rate: f64,

    /// Period in milliseconds (typically 86400000 for 1 day)
    pub period: i64,

    /// Timestamp in milliseconds
    pub timestamp: Option<Timestamp>,

    /// ISO 8601 datetime string
    pub datetime: Option<String>,

    /// Raw response data from the exchange
    pub info: Value,
}

impl IsolatedBorrowRate {
    /// Create a new isolated borrow rate
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        symbol: String,
        base: String,
        base_rate: f64,
        quote: String,
        quote_rate: f64,
        period: i64,
        timestamp: Option<Timestamp>,
        info: Value,
    ) -> Self {
        let datetime = timestamp.and_then(|ts| crate::time::iso8601(ts).ok());
        Self {
            symbol,
            base,
            base_rate,
            quote,
            quote_rate,
            period,
            timestamp,
            datetime,
            info,
        }
    }
}

/// Margin loan record
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MarginLoan {
    /// Loan transaction ID
    pub id: String,

    /// Currency code
    pub currency: String,

    /// Trading symbol for isolated margin
    pub symbol: Option<String>,

    /// Loan amount
    pub amount: f64,

    /// Timestamp in milliseconds
    pub timestamp: Timestamp,

    /// ISO 8601 datetime string
    pub datetime: String,

    /// Whether this is for isolated margin
    pub is_isolated: bool,

    /// Loan status (e.g., "PENDING", "CONFIRMED", "FAILED")
    pub status: String,

    /// Raw response data from the exchange
    pub info: Value,
}

impl MarginLoan {
    /// Create a new margin loan record
    pub fn new(
        id: String,
        currency: String,
        symbol: Option<String>,
        amount: f64,
        timestamp: Timestamp,
        status: String,
        info: Value,
    ) -> Self {
        let is_isolated = symbol.is_some();
        let datetime = crate::time::iso8601(timestamp).unwrap_or_default();
        Self {
            id,
            currency,
            symbol,
            amount,
            timestamp,
            datetime,
            is_isolated,
            status,
            info,
        }
    }
}

/// Margin repayment record
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MarginRepay {
    /// Repayment transaction ID
    pub id: String,

    /// Currency code
    pub currency: String,

    /// Trading symbol for isolated margin
    pub symbol: Option<String>,

    /// Repayment amount
    pub amount: f64,

    /// Timestamp in milliseconds
    pub timestamp: Timestamp,

    /// ISO 8601 datetime string
    pub datetime: String,

    /// Whether this is for isolated margin
    pub is_isolated: bool,

    /// Repayment status (e.g., "PENDING", "CONFIRMED", "FAILED")
    pub status: String,

    /// Raw response data from the exchange
    pub info: Value,
}

impl MarginRepay {
    /// Create a new margin repayment record
    pub fn new(
        id: String,
        currency: String,
        symbol: Option<String>,
        amount: f64,
        timestamp: Timestamp,
        status: String,
        info: Value,
    ) -> Self {
        let is_isolated = symbol.is_some();
        let datetime = crate::time::iso8601(timestamp).unwrap_or_default();
        Self {
            id,
            currency,
            symbol,
            amount,
            timestamp,
            datetime,
            is_isolated,
            status,
            info,
        }
    }
}

/// Borrow rate history record
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BorrowRateHistory {
    /// Currency code
    pub currency: String,

    /// Trading symbol for isolated margin
    pub symbol: Option<String>,

    /// Daily interest rate
    pub rate: f64,

    /// Timestamp in milliseconds
    pub timestamp: Timestamp,

    /// ISO 8601 datetime string
    pub datetime: String,

    /// VIP level (if applicable)
    pub vip_level: Option<i32>,

    /// Raw response data from the exchange
    pub info: Value,
}

impl BorrowRateHistory {
    /// Create a new borrow rate history record
    pub fn new(
        currency: String,
        symbol: Option<String>,
        rate: f64,
        timestamp: Timestamp,
        vip_level: Option<i32>,
        info: Value,
    ) -> Self {
        let datetime = crate::time::iso8601(timestamp).unwrap_or_default();
        Self {
            currency,
            symbol,
            rate,
            timestamp,
            datetime,
            vip_level,
            info,
        }
    }
}

/// Borrow interest record
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BorrowInterest {
    /// Record ID
    pub id: String,

    /// Currency code
    pub currency: String,

    /// Trading symbol for isolated margin
    pub symbol: Option<String>,

    /// Interest amount
    pub interest: f64,

    /// Interest rate
    pub interest_rate: f64,

    /// Principal amount
    pub principal: f64,

    /// Timestamp in milliseconds
    pub timestamp: Timestamp,

    /// ISO 8601 datetime string
    pub datetime: String,

    /// Whether this is for isolated margin
    pub is_isolated: bool,

    /// Raw response data from the exchange
    pub info: Value,
}

impl BorrowInterest {
    /// Create a new borrow interest record
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: String,
        currency: String,
        symbol: Option<String>,
        interest: f64,
        interest_rate: f64,
        principal: f64,
        timestamp: Timestamp,
        info: Value,
    ) -> Self {
        let is_isolated = symbol.is_some();
        let datetime = crate::time::iso8601(timestamp).unwrap_or_default();
        Self {
            id,
            currency,
            symbol,
            interest,
            interest_rate,
            principal,
            timestamp,
            datetime,
            is_isolated,
            info,
        }
    }
}

/// Margin adjustment record (transfers between spot and margin accounts)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MarginAdjustment {
    /// Record ID
    pub id: String,

    /// Trading symbol (for isolated margin)
    pub symbol: Option<String>,

    /// Currency code
    pub currency: String,

    /// Adjustment amount
    pub amount: f64,

    /// Transfer type: "IN" (to margin) or "OUT" (from margin)
    pub transfer_type: String,

    /// Timestamp in milliseconds
    pub timestamp: Timestamp,

    /// ISO 8601 datetime string
    pub datetime: String,

    /// Transfer status
    pub status: String,

    /// Raw response data from the exchange
    pub info: Value,
}

impl MarginAdjustment {
    /// Create a new margin adjustment record
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: String,
        symbol: Option<String>,
        currency: String,
        amount: f64,
        transfer_type: String,
        timestamp: Timestamp,
        status: String,
        info: Value,
    ) -> Self {
        let datetime = crate::time::iso8601(timestamp).unwrap_or_default();
        Self {
            id,
            symbol,
            currency,
            amount,
            transfer_type,
            timestamp,
            datetime,
            status,
            info,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_borrow_rate_cross() {
        let rate = BorrowRate::new_cross(
            "USDT".to_string(),
            0.00055,
            1700000000000,
            json!({"test": "data"}),
        );

        assert_eq!(rate.currency, "USDT");
        assert_eq!(rate.symbol, None);
        assert_eq!(rate.rate, 0.00055);
        assert!(!rate.is_isolated);
    }

    #[test]
    fn test_borrow_rate_isolated() {
        let rate = BorrowRate::new_isolated(
            "BTC".to_string(),
            "BTC/USDT".to_string(),
            0.00065,
            1700000000000,
            json!({"test": "data"}),
        );

        assert_eq!(rate.currency, "BTC");
        assert_eq!(rate.symbol, Some("BTC/USDT".to_string()));
        assert_eq!(rate.rate, 0.00065);
        assert!(rate.is_isolated);
    }

    #[test]
    fn test_margin_loan() {
        let loan = MarginLoan::new(
            "12345".to_string(),
            "USDT".to_string(),
            None,
            1000.0,
            1700000000000,
            "CONFIRMED".to_string(),
            json!({"test": "data"}),
        );

        assert_eq!(loan.id, "12345");
        assert_eq!(loan.currency, "USDT");
        assert_eq!(loan.amount, 1000.0);
        assert_eq!(loan.status, "CONFIRMED");
        assert!(!loan.is_isolated);
    }

    #[test]
    fn test_margin_repay() {
        let repay = MarginRepay::new(
            "54321".to_string(),
            "USDT".to_string(),
            Some("BTC/USDT".to_string()),
            500.0,
            1700000000000,
            "CONFIRMED".to_string(),
            json!({"test": "data"}),
        );

        assert_eq!(repay.id, "54321");
        assert_eq!(repay.amount, 500.0);
        assert!(repay.is_isolated);
    }

    #[test]
    fn test_borrow_interest() {
        let interest = BorrowInterest::new(
            "99999".to_string(),
            "USDT".to_string(),
            None,
            0.55,
            0.00055,
            1000.0,
            1700000000000,
            json!({"test": "data"}),
        );

        assert_eq!(interest.interest, 0.55);
        assert_eq!(interest.interest_rate, 0.00055);
        assert_eq!(interest.principal, 1000.0);
    }

    #[test]
    fn test_margin_adjustment() {
        let adjustment = MarginAdjustment::new(
            "11111".to_string(),
            None,
            "USDT".to_string(),
            1000.0,
            "IN".to_string(),
            1700000000000,
            "SUCCESS".to_string(),
            json!({"test": "data"}),
        );

        assert_eq!(adjustment.transfer_type, "IN");
        assert_eq!(adjustment.amount, 1000.0);
        assert_eq!(adjustment.status, "SUCCESS");
    }
}
