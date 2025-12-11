//! Funding rate type definitions
//!
//! This module contains type definitions for funding rates, funding rate history,
//! and funding fee records used in perpetual futures trading.

use serde::{Deserialize, Serialize};

/// Funding rate information.
///
/// Contains comprehensive funding rate data for perpetual contracts including
/// mark price, index price, and funding rate timestamps.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FundingRate {
    /// Raw exchange response data.
    pub info: serde_json::Value,

    /// Trading pair symbol.
    pub symbol: String,

    /// Mark price.
    pub mark_price: Option<f64>,

    /// Index price.
    pub index_price: Option<f64>,

    /// Interest rate.
    pub interest_rate: Option<f64>,

    /// Estimated settlement price.
    pub estimated_settle_price: Option<f64>,

    /// Current funding rate.
    pub funding_rate: Option<f64>,

    /// Next funding rate timestamp.
    pub funding_timestamp: Option<u64>,

    /// Next funding rate time in ISO 8601 format.
    pub funding_datetime: Option<String>,

    /// Previous funding rate.
    pub previous_funding_rate: Option<f64>,

    /// Previous funding rate timestamp.
    pub previous_funding_timestamp: Option<u64>,

    /// Previous funding rate time in ISO 8601 format.
    pub previous_funding_datetime: Option<String>,

    /// Data timestamp.
    pub timestamp: Option<u64>,

    /// Data time in ISO 8601 format.
    pub datetime: Option<String>,
}

impl Default for FundingRate {
    fn default() -> Self {
        Self {
            info: serde_json::Value::Null,
            symbol: String::new(),
            mark_price: None,
            index_price: None,
            interest_rate: None,
            estimated_settle_price: None,
            funding_rate: None,
            funding_timestamp: None,
            funding_datetime: None,
            previous_funding_rate: None,
            previous_funding_timestamp: None,
            previous_funding_datetime: None,
            timestamp: None,
            datetime: None,
        }
    }
}

/// Funding rate history record.
///
/// Contains a single historical funding rate data point.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FundingRateHistory {
    /// Raw exchange response data.
    pub info: serde_json::Value,

    /// Trading pair symbol.
    pub symbol: String,

    /// Funding rate.
    pub funding_rate: Option<f64>,

    /// Timestamp.
    pub timestamp: Option<u64>,

    /// Datetime string in ISO 8601 format.
    pub datetime: Option<String>,
}

impl Default for FundingRateHistory {
    fn default() -> Self {
        Self {
            info: serde_json::Value::Null,
            symbol: String::new(),
            funding_rate: None,
            timestamp: None,
            datetime: None,
        }
    }
}

/// Funding fee history record.
///
/// Contains historical funding fee payment information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FundingHistory {
    /// Raw exchange response data.
    pub info: serde_json::Value,

    /// Income ID.
    pub id: Option<String>,

    /// Trading pair symbol.
    pub symbol: String,

    /// Funding fee amount.
    pub amount: Option<f64>,

    /// Currency code.
    pub code: Option<String>,

    /// Timestamp.
    pub timestamp: Option<u64>,

    /// Datetime string in ISO 8601 format.
    pub datetime: Option<String>,
}

impl Default for FundingHistory {
    fn default() -> Self {
        Self {
            info: serde_json::Value::Null,
            id: None,
            symbol: String::new(),
            amount: None,
            code: None,
            timestamp: None,
            datetime: None,
        }
    }
}

/// Personal funding fee record.
///
/// Contains user-specific funding fee income or payment information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FundingFee {
    /// Raw exchange response data.
    pub info: serde_json::Value,

    /// Trading pair symbol.
    pub symbol: String,

    /// Funding fee income (positive = income, negative = payment).
    pub income: f64,

    /// Asset type (e.g., USDT).
    pub asset: String,

    /// Timestamp.
    pub time: u64,

    /// Datetime string in ISO 8601 format.
    pub datetime: Option<String>,

    /// Funding rate at that time.
    pub funding_rate: Option<f64>,

    /// Mark price.
    pub mark_price: Option<f64>,
}

impl Default for FundingFee {
    fn default() -> Self {
        Self {
            info: serde_json::Value::Null,
            symbol: String::new(),
            income: 0.0,
            asset: String::new(),
            time: 0,
            datetime: None,
            funding_rate: None,
            mark_price: None,
        }
    }
}

/// Next funding rate information.
///
/// Contains current and predicted next funding rate data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NextFundingRate {
    /// Raw exchange response data.
    pub info: serde_json::Value,

    /// Trading pair symbol.
    pub symbol: String,

    /// Mark price.
    pub mark_price: f64,

    /// Index price.
    pub index_price: Option<f64>,

    /// Current funding rate.
    pub current_funding_rate: f64,

    /// Next funding rate (predicted).
    pub next_funding_rate: f64,

    /// Next settlement timestamp.
    pub next_funding_time: u64,

    /// Next settlement time in ISO 8601 format.
    pub next_funding_datetime: Option<String>,
}

impl Default for NextFundingRate {
    fn default() -> Self {
        Self {
            info: serde_json::Value::Null,
            symbol: String::new(),
            mark_price: 0.0,
            index_price: None,
            current_funding_rate: 0.0,
            next_funding_rate: 0.0,
            next_funding_time: 0,
            next_funding_datetime: None,
        }
    }
}
