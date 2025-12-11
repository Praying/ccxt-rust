//! Fee-related type definitions
//!
//! This module contains type definitions for trading fees, funding rates, and leverage tiers.

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Trading fee structure.
///
/// Contains maker and taker fee information for a specific market.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TradingFee {
    /// Raw API response data.
    pub info: Value,

    /// Market symbol.
    pub symbol: String,

    /// Maker fee rate (limit orders).
    pub maker: Decimal,

    /// Taker fee rate (market orders).
    pub taker: Decimal,

    /// Whether the fee is in percentage format.
    #[serde(default)]
    pub percentage: bool,

    /// Whether tier-based fees are applied.
    #[serde(default)]
    pub tier_based: bool,
}

impl TradingFee {
    /// Creates a new trading fee structure.
    pub fn new(symbol: String, maker: Decimal, taker: Decimal) -> Self {
        Self {
            info: Value::Null,
            symbol,
            maker,
            taker,
            percentage: true,
            tier_based: false,
        }
    }
}

/// Funding rate structure.
///
/// Contains funding rate information for perpetual contracts.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FundingRate {
    /// Raw API response data.
    pub info: Value,

    /// Market symbol.
    pub symbol: String,

    /// Mark price.
    pub mark_price: Option<Decimal>,

    /// Index price.
    pub index_price: Option<Decimal>,

    /// Interest rate.
    pub interest_rate: Option<Decimal>,

    /// Estimated settlement price.
    pub estimated_settle_price: Option<Decimal>,

    /// Current funding rate.
    pub funding_rate: Option<Decimal>,

    /// Funding rate timestamp in milliseconds.
    pub funding_timestamp: Option<i64>,

    /// Funding rate time in ISO 8601 format.
    pub funding_datetime: Option<String>,

    /// Next funding rate.
    pub next_funding_rate: Option<Decimal>,

    /// Next funding rate timestamp in milliseconds.
    pub next_funding_timestamp: Option<i64>,

    /// Next funding rate time in ISO 8601 format.
    pub next_funding_datetime: Option<String>,

    /// Previous funding rate.
    pub previous_funding_rate: Option<Decimal>,

    /// Previous funding rate timestamp in milliseconds.
    pub previous_funding_timestamp: Option<i64>,

    /// Previous funding rate time in ISO 8601 format.
    pub previous_funding_datetime: Option<String>,

    /// Data timestamp in milliseconds.
    pub timestamp: Option<i64>,

    /// Data time in ISO 8601 format.
    pub datetime: Option<String>,

    /// Funding rate interval (e.g., "8h").
    pub interval: Option<String>,
}

impl FundingRate {
    /// Creates a new funding rate structure.
    pub fn new(symbol: String) -> Self {
        Self {
            info: Value::Null,
            symbol,
            mark_price: None,
            index_price: None,
            interest_rate: None,
            estimated_settle_price: None,
            funding_rate: None,
            funding_timestamp: None,
            funding_datetime: None,
            next_funding_rate: None,
            next_funding_timestamp: None,
            next_funding_datetime: None,
            previous_funding_rate: None,
            previous_funding_timestamp: None,
            previous_funding_datetime: None,
            timestamp: None,
            datetime: None,
            interval: None,
        }
    }
}

/// Funding rate history structure.
///
/// Contains historical funding rate data points.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FundingRateHistory {
    /// Raw API response data.
    pub info: Value,

    /// Market symbol.
    pub symbol: String,

    /// Funding rate.
    pub funding_rate: Option<Decimal>,

    /// Funding rate timestamp in milliseconds.
    pub funding_timestamp: Option<i64>,

    /// Funding rate time in ISO 8601 format.
    pub funding_datetime: Option<String>,

    /// Data timestamp in milliseconds.
    pub timestamp: Option<i64>,

    /// Data time in ISO 8601 format.
    pub datetime: Option<String>,
}

impl FundingRateHistory {
    /// Creates a new funding rate history structure.
    pub fn new(symbol: String, funding_rate: Decimal, timestamp: i64) -> Self {
        Self {
            info: Value::Null,
            symbol,
            funding_rate: Some(funding_rate),
            funding_timestamp: Some(timestamp),
            funding_datetime: None,
            timestamp: Some(timestamp),
            datetime: None,
        }
    }
}

/// Leverage tier structure.
///
/// Describes leverage limits for different notional value ranges.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LeverageTier {
    /// Raw API response data.
    pub info: Value,

    /// Tier number.
    pub tier: i32,

    /// Market symbol.
    pub symbol: String,

    /// Currency (typically the quote currency).
    pub currency: String,

    /// Minimum notional value.
    pub min_notional: Decimal,

    /// Maximum notional value.
    pub max_notional: Decimal,

    /// Maintenance margin rate.
    pub maintenance_margin_rate: Decimal,

    /// Maximum leverage multiplier.
    pub max_leverage: i32,
}

impl LeverageTier {
    /// Creates a new leverage tier structure.
    pub fn new(
        tier: i32,
        symbol: String,
        currency: String,
        min_notional: Decimal,
        max_notional: Decimal,
        maintenance_margin_rate: Decimal,
        max_leverage: i32,
    ) -> Self {
        Self {
            info: Value::Null,
            tier,
            symbol,
            currency,
            min_notional,
            max_notional,
            maintenance_margin_rate,
            max_leverage,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_trading_fee_creation() {
        let fee = TradingFee::new("BTC/USDT".to_string(), dec!(0.001), dec!(0.001));

        assert_eq!(fee.symbol, "BTC/USDT");
        assert_eq!(fee.maker, dec!(0.001));
        assert_eq!(fee.taker, dec!(0.001));
        assert!(fee.percentage);
        assert!(!fee.tier_based);
    }

    #[test]
    fn test_funding_rate_creation() {
        let rate = FundingRate::new("BTC/USDT:USDT".to_string());

        assert_eq!(rate.symbol, "BTC/USDT:USDT");
        assert!(rate.funding_rate.is_none());
        assert!(rate.mark_price.is_none());
    }

    #[test]
    fn test_funding_rate_history_creation() {
        let history =
            FundingRateHistory::new("BTC/USDT:USDT".to_string(), dec!(0.0001), 1621267200000);

        assert_eq!(history.symbol, "BTC/USDT:USDT");
        assert_eq!(history.funding_rate, Some(dec!(0.0001)));
        assert_eq!(history.timestamp, Some(1621267200000));
    }

    #[test]
    fn test_leverage_tier_creation() {
        let tier = LeverageTier::new(
            1,
            "BTC/USDT:USDT".to_string(),
            "USDT".to_string(),
            dec!(0),
            dec!(50000),
            dec!(0.01),
            50,
        );

        assert_eq!(tier.tier, 1);
        assert_eq!(tier.symbol, "BTC/USDT:USDT");
        assert_eq!(tier.max_leverage, 50);
        assert_eq!(tier.maintenance_margin_rate, dec!(0.01));
    }
}
