#![allow(dead_code)]

use super::{parse_decimal, parse_f64};
use ccxt_core::{
    Result,
    error::{Error, ParseError},
    types::{
        FeeFundingRate, FeeFundingRateHistory, FundingFee, FundingHistory, Market, NextFundingRate,
    },
};
use serde_json::Value;

/// Parse funding rate data from Binance futures API.
pub fn parse_funding_rate(data: &Value, market: Option<&Market>) -> Result<FeeFundingRate> {
    let symbol = if let Some(m) = market {
        m.symbol.clone()
    } else {
        data["symbol"]
            .as_str()
            .ok_or_else(|| Error::from(ParseError::missing_field("symbol")))?
            .to_string()
    };

    let funding_rate =
        parse_decimal(data, "lastFundingRate").or_else(|| parse_decimal(data, "fundingRate"));

    let mark_price = parse_decimal(data, "markPrice");
    let index_price = parse_decimal(data, "indexPrice");
    let interest_rate = parse_decimal(data, "interestRate");

    let next_funding_time = data["nextFundingTime"].as_i64();
    let funding_timestamp = next_funding_time.or_else(|| data["fundingTime"].as_i64());

    Ok(FeeFundingRate {
        info: data.clone(),
        symbol,
        mark_price,
        index_price,
        interest_rate,
        estimated_settle_price: None,
        funding_rate,
        funding_timestamp,
        funding_datetime: funding_timestamp.map(|t| {
            chrono::DateTime::from_timestamp(t / 1000, 0)
                .map(|dt| dt.to_rfc3339())
                .unwrap_or_default()
        }),
        next_funding_rate: None,
        next_funding_timestamp: None,
        next_funding_datetime: None,
        previous_funding_rate: None,
        previous_funding_timestamp: None,
        previous_funding_datetime: None,
        timestamp: None,
        datetime: None,
        interval: None,
    })
}

/// Parse funding rate history data from Binance futures API.
pub fn parse_funding_rate_history(
    data: &Value,
    market: Option<&Market>,
) -> Result<FeeFundingRateHistory> {
    let symbol = if let Some(m) = market {
        m.symbol.clone()
    } else {
        data["symbol"]
            .as_str()
            .ok_or_else(|| Error::from(ParseError::missing_field("symbol")))?
            .to_string()
    };

    let funding_rate = parse_decimal(data, "fundingRate");
    let funding_time = data["fundingTime"].as_i64();

    Ok(FeeFundingRateHistory {
        info: data.clone(),
        symbol,
        funding_rate,
        funding_timestamp: funding_time,
        funding_datetime: funding_time.map(|t| {
            chrono::DateTime::from_timestamp(t / 1000, 0)
                .map(|dt| dt.to_rfc3339())
                .unwrap_or_default()
        }),
        timestamp: funding_time,
        datetime: funding_time.map(|t| {
            chrono::DateTime::from_timestamp(t / 1000, 0)
                .map(|dt| dt.to_rfc3339())
                .unwrap_or_default()
        }),
    })
}

/// Parse funding fee history data from Binance futures API.
pub fn parse_funding_history(data: &Value, market: Option<&Market>) -> Result<FundingHistory> {
    let symbol = if let Some(m) = market {
        m.symbol.clone()
    } else {
        data["symbol"]
            .as_str()
            .ok_or_else(|| Error::from(ParseError::missing_field("symbol")))?
            .to_string()
    };

    let id = data["tranId"]
        .as_u64()
        .or_else(|| data["id"].as_u64())
        .map(|v| v.to_string());

    let amount = parse_f64(data, "income");
    let code = data["asset"].as_str().map(ToString::to_string);
    let timestamp = data["time"].as_i64();

    Ok(FundingHistory {
        info: data.clone(),
        id,
        symbol,
        code,
        amount,
        timestamp,
        datetime: timestamp.map(|t| {
            chrono::DateTime::from_timestamp(t / 1000, 0)
                .map(|dt| dt.to_rfc3339())
                .unwrap_or_default()
        }),
    })
}

/// Parse funding fee data from Binance futures API.
pub fn parse_funding_fee(data: &Value, market: Option<&Market>) -> Result<FundingFee> {
    let symbol = if let Some(m) = market {
        m.symbol.clone()
    } else {
        data["symbol"]
            .as_str()
            .ok_or_else(|| Error::from(ParseError::missing_field("symbol")))?
            .to_string()
    };

    let income = parse_f64(data, "income").unwrap_or(0.0);
    let asset = data["asset"]
        .as_str()
        .ok_or_else(|| Error::from(ParseError::missing_field("asset")))?
        .to_string();

    let time = data["time"]
        .as_i64()
        .ok_or_else(|| Error::from(ParseError::missing_field("time")))?;

    let funding_rate = parse_f64(data, "fundingRate");
    let mark_price = parse_f64(data, "markPrice");

    let datetime = Some(
        chrono::DateTime::from_timestamp(time / 1000, 0)
            .map(|dt| dt.to_rfc3339())
            .unwrap_or_default(),
    );

    Ok(FundingFee {
        info: data.clone(),
        symbol,
        income,
        asset,
        time,
        datetime,
        funding_rate,
        mark_price,
    })
}

/// Parse next funding rate data from Binance premium index.
pub fn parse_next_funding_rate(data: &Value, market: &Market) -> Result<NextFundingRate> {
    let symbol = market.symbol.clone();

    let mark_price = parse_f64(data, "markPrice")
        .ok_or_else(|| Error::from(ParseError::missing_field("markPrice")))?;

    let index_price = parse_f64(data, "indexPrice");

    let current_funding_rate = parse_f64(data, "lastFundingRate").unwrap_or(0.0);

    let next_funding_rate = parse_f64(data, "interestRate")
        .or_else(|| parse_f64(data, "estimatedSettlePrice"))
        .unwrap_or(current_funding_rate);

    let next_funding_time = data["nextFundingTime"]
        .as_i64()
        .ok_or_else(|| Error::from(ParseError::missing_field("nextFundingTime")))?;

    let next_funding_datetime = Some(
        chrono::DateTime::from_timestamp(next_funding_time / 1000, 0)
            .map(|dt| dt.to_rfc3339())
            .unwrap_or_default(),
    );

    Ok(NextFundingRate {
        info: data.clone(),
        symbol,
        mark_price,
        index_price,
        current_funding_rate,
        next_funding_rate,
        next_funding_time,
        next_funding_datetime,
    })
}
