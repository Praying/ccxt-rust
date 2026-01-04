#![allow(dead_code, clippy::unnecessary_wraps)]

use ccxt_core::{
    Result,
    error::{Error, ParseError},
    types::{
        BorrowInterest, BorrowRate, BorrowRateHistory, MarginAdjustment, MarginLoan, MaxBorrowable,
        MaxTransferable,
    },
};
use serde_json::Value;

/// Parse borrow interest rate data from Binance margin API.
pub fn parse_borrow_rate(data: &Value, currency: &str, symbol: Option<&str>) -> Result<BorrowRate> {
    let rate = if let Some(rate_str) = data["dailyInterestRate"].as_str() {
        rate_str.parse::<f64>().unwrap_or(0.0)
    } else {
        data["dailyInterestRate"].as_f64().unwrap_or(0.0)
    };

    let timestamp = data["timestamp"]
        .as_i64()
        .or_else(|| {
            data["vipLevel"]
                .as_i64()
                .map(|_| chrono::Utc::now().timestamp_millis())
        })
        .unwrap_or_else(|| chrono::Utc::now().timestamp_millis());

    if let Some(sym) = symbol {
        Ok(BorrowRate::new_isolated(
            currency.to_string(),
            sym.to_string(),
            rate,
            timestamp,
            data.clone(),
        ))
    } else {
        Ok(BorrowRate::new_cross(
            currency.to_string(),
            rate,
            timestamp,
            data.clone(),
        ))
    }
}

/// Parse margin loan record data from Binance margin API.
pub fn parse_margin_loan(data: &Value) -> Result<MarginLoan> {
    let id = data["tranId"]
        .as_i64()
        .or_else(|| data["txId"].as_i64())
        .map(|id| id.to_string())
        .unwrap_or_default();

    let currency = data["asset"]
        .as_str()
        .ok_or_else(|| Error::from(ParseError::missing_field("asset")))?
        .to_string();

    let symbol = data["symbol"].as_str().map(ToString::to_string);

    let amount = if let Some(amount_str) = data["amount"].as_str() {
        amount_str.parse::<f64>().unwrap_or(0.0)
    } else {
        data["amount"].as_f64().unwrap_or(0.0)
    };

    let timestamp = data["timestamp"]
        .as_i64()
        .unwrap_or_else(|| chrono::Utc::now().timestamp_millis());

    let status = data["status"].as_str().unwrap_or("CONFIRMED").to_string();

    Ok(MarginLoan::new(
        id,
        currency,
        symbol,
        amount,
        timestamp,
        status,
        data.clone(),
    ))
}

/// Parse margin adjustment (transfer) data from Binance margin API.
pub fn parse_margin_adjustment(data: &Value) -> Result<MarginAdjustment> {
    let id = data["tranId"]
        .as_i64()
        .or_else(|| data["txId"].as_i64())
        .map(|id| id.to_string())
        .unwrap_or_default();

    let symbol = data["symbol"].as_str().map(ToString::to_string);

    let currency = data["asset"]
        .as_str()
        .ok_or_else(|| Error::from(ParseError::missing_field("asset")))?
        .to_string();

    let amount = if let Some(amount_str) = data["amount"].as_str() {
        amount_str.parse::<f64>().unwrap_or(0.0)
    } else {
        data["amount"].as_f64().unwrap_or(0.0)
    };

    let transfer_type = data["type"]
        .as_str()
        .or_else(|| data["transFrom"].as_str())
        .map_or("IN", |t| {
            if t.contains("MAIN") || t.eq("1") || t.eq("ROLL_IN") {
                "IN"
            } else {
                "OUT"
            }
        })
        .to_string();

    let timestamp = data["timestamp"]
        .as_i64()
        .unwrap_or_else(|| chrono::Utc::now().timestamp_millis());

    let status = data["status"].as_str().unwrap_or("SUCCESS").to_string();

    Ok(MarginAdjustment::new(
        id,
        symbol,
        currency,
        amount,
        transfer_type,
        timestamp,
        status,
        data.clone(),
    ))
}

/// Parse borrow interest accrual data from Binance margin API.
pub fn parse_borrow_interest(data: &Value) -> Result<BorrowInterest> {
    let id = data["txId"]
        .as_i64()
        .or_else(|| {
            data["isolatedSymbol"]
                .as_str()
                .map(|_| chrono::Utc::now().timestamp_millis())
        })
        .map(|id| id.to_string())
        .unwrap_or_default();

    let currency = data["asset"]
        .as_str()
        .ok_or_else(|| Error::from(ParseError::missing_field("asset")))?
        .to_string();

    let symbol = data["isolatedSymbol"].as_str().map(ToString::to_string);

    let interest = if let Some(interest_str) = data["interest"].as_str() {
        interest_str.parse::<f64>().unwrap_or(0.0)
    } else {
        data["interest"].as_f64().unwrap_or(0.0)
    };

    let interest_rate = if let Some(rate_str) = data["interestRate"].as_str() {
        rate_str.parse::<f64>().unwrap_or(0.0)
    } else {
        data["interestRate"].as_f64().unwrap_or(0.0)
    };

    let principal = if let Some(principal_str) = data["principal"].as_str() {
        principal_str.parse::<f64>().unwrap_or(0.0)
    } else {
        data["principal"].as_f64().unwrap_or(0.0)
    };

    let timestamp = data["interestAccuredTime"]
        .as_i64()
        .or_else(|| data["timestamp"].as_i64())
        .unwrap_or_else(|| chrono::Utc::now().timestamp_millis());

    Ok(BorrowInterest::new(
        id,
        currency,
        symbol,
        interest,
        interest_rate,
        principal,
        timestamp,
        data.clone(),
    ))
}

/// Parse multiple borrow interest records from Binance API response.
pub fn parse_borrow_interests(data: &Value) -> Result<Vec<BorrowInterest>> {
    let mut interests = Vec::new();

    if let Some(array) = data.as_array() {
        for item in array {
            match parse_borrow_interest(item) {
                Ok(interest) => interests.push(interest),
                Err(e) => {
                    eprintln!("Failed to parse borrow interest: {}", e);
                }
            }
        }
    }

    Ok(interests)
}

/// Parse borrow rate history from Binance API response.
pub fn parse_borrow_rate_history(data: &Value, currency: &str) -> Result<BorrowRateHistory> {
    let timestamp = data["timestamp"]
        .as_i64()
        .or_else(|| data["time"].as_i64())
        .unwrap_or(0);

    let rate = data["hourlyInterestRate"]
        .as_str()
        .or_else(|| data["dailyInterestRate"].as_str())
        .or_else(|| data["rate"].as_str())
        .and_then(|s| s.parse::<f64>().ok())
        .or_else(|| data["hourlyInterestRate"].as_f64())
        .or_else(|| data["dailyInterestRate"].as_f64())
        .or_else(|| data["rate"].as_f64())
        .unwrap_or(0.0);

    let datetime = chrono::DateTime::from_timestamp_millis(timestamp)
        .map(|dt| dt.to_rfc3339())
        .unwrap_or_default();

    let symbol = data["symbol"].as_str().map(ToString::to_string);
    let vip_level = data["vipLevel"].as_i64().map(|v| v as i32);

    Ok(BorrowRateHistory {
        currency: currency.to_string(),
        symbol,
        rate,
        timestamp,
        datetime,
        vip_level,
        info: data.clone(),
    })
}

/// Parse isolated margin borrow rates from Binance API response.
pub fn parse_isolated_borrow_rates(
    data: &Value,
) -> Result<std::collections::HashMap<String, ccxt_core::types::IsolatedBorrowRate>> {
    use ccxt_core::types::IsolatedBorrowRate;
    use std::collections::HashMap;

    let mut rates_map = HashMap::new();

    if let Some(array) = data.as_array() {
        for item in array {
            let symbol = item["symbol"].as_str().unwrap_or("");
            let base = item["base"].as_str().unwrap_or("");
            let quote = item["quote"].as_str().unwrap_or("");

            let base_rate = item["dailyInterestRate"]
                .as_str()
                .and_then(|s| s.parse::<f64>().ok())
                .unwrap_or(0.0);

            let quote_rate = item["quoteDailyInterestRate"]
                .as_str()
                .and_then(|s| s.parse::<f64>().ok())
                .or_else(|| {
                    item["dailyInterestRate"]
                        .as_str()
                        .and_then(|s| s.parse::<f64>().ok())
                })
                .unwrap_or(0.0);

            let timestamp = item["timestamp"].as_i64().or_else(|| item["time"].as_i64());

            let datetime = timestamp.and_then(|ts| {
                chrono::DateTime::from_timestamp_millis(ts).map(|dt| dt.to_rfc3339())
            });

            let isolated_rate = IsolatedBorrowRate {
                symbol: symbol.to_string(),
                base: base.to_string(),
                base_rate,
                quote: quote.to_string(),
                quote_rate,
                period: 86400000,
                timestamp,
                datetime,
                info: item.clone(),
            };

            rates_map.insert(symbol.to_string(), isolated_rate);
        }
    }

    Ok(rates_map)
}

/// Parse maximum borrowable amount data from Binance margin API.
pub fn parse_max_borrowable(
    data: &Value,
    currency: &str,
    symbol: Option<String>,
) -> Result<MaxBorrowable> {
    let amount = if let Some(amount_str) = data["amount"].as_str() {
        amount_str.parse::<f64>().unwrap_or(0.0)
    } else {
        data["amount"].as_f64().unwrap_or(0.0)
    };

    let borrow_limit = if let Some(limit_str) = data["borrowLimit"].as_str() {
        Some(limit_str.parse::<f64>().unwrap_or(0.0))
    } else {
        data["borrowLimit"].as_f64()
    };

    let timestamp = chrono::Utc::now().timestamp_millis();
    let datetime = chrono::DateTime::from_timestamp_millis(timestamp)
        .map(|dt| dt.to_rfc3339())
        .unwrap_or_default();

    Ok(MaxBorrowable {
        currency: currency.to_string(),
        amount,
        borrow_limit,
        symbol,
        timestamp,
        datetime,
        info: data.clone(),
    })
}

/// Parse maximum transferable amount data from Binance margin API.
pub fn parse_max_transferable(
    data: &Value,
    currency: &str,
    symbol: Option<String>,
) -> Result<MaxTransferable> {
    let amount = if let Some(amount_str) = data["amount"].as_str() {
        amount_str.parse::<f64>().unwrap_or(0.0)
    } else {
        data["amount"].as_f64().unwrap_or(0.0)
    };

    let timestamp = chrono::Utc::now().timestamp_millis();
    let datetime = chrono::DateTime::from_timestamp_millis(timestamp)
        .map(|dt| dt.to_rfc3339())
        .unwrap_or_default();

    Ok(MaxTransferable {
        currency: currency.to_string(),
        amount,
        symbol,
        timestamp,
        datetime,
        info: data.clone(),
    })
}
