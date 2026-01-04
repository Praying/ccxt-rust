#![allow(dead_code)]

use super::value_to_hashmap;
use ccxt_core::{
    Result,
    error::{Error, ParseError},
    types::{
        OrderBook, OrderBookDelta, OrderBookEntry,
        financial::{Amount, Price},
    },
};
use rust_decimal::Decimal;
use serde_json::Value;

/// Parse orderbook data from Binance depth response.
pub fn parse_orderbook(data: &Value, symbol: String) -> Result<OrderBook> {
    let timestamp = data["T"]
        .as_i64()
        .or_else(|| data["E"].as_i64())
        .unwrap_or_else(|| chrono::Utc::now().timestamp_millis());

    let bids = parse_orderbook_side(&data["bids"])?;
    let asks = parse_orderbook_side(&data["asks"])?;

    Ok(OrderBook {
        symbol,
        timestamp,
        datetime: Some({
            chrono::DateTime::from_timestamp_millis(timestamp)
                .map(|dt| dt.to_rfc3339())
                .unwrap_or_default()
        }),
        nonce: data["lastUpdateId"].as_i64(),
        bids,
        asks,
        buffered_deltas: std::collections::VecDeque::new(),
        bids_map: std::collections::BTreeMap::new(),
        asks_map: std::collections::BTreeMap::new(),
        is_synced: false,
        needs_resync: false,
        last_resync_time: 0,
        info: value_to_hashmap(data),
    })
}

/// Parse WebSocket orderbook data from Binance depth stream.
pub fn parse_ws_orderbook(data: &Value, symbol: String) -> Result<OrderBook> {
    let timestamp = data["E"]
        .as_i64()
        .or_else(|| data["T"].as_i64())
        .unwrap_or_else(|| chrono::Utc::now().timestamp_millis());

    let bids = parse_orderbook_side(&data["b"])?;
    let asks = parse_orderbook_side(&data["a"])?;

    Ok(OrderBook {
        symbol,
        timestamp,
        datetime: Some(
            chrono::DateTime::from_timestamp_millis(timestamp)
                .map(|dt| dt.to_rfc3339())
                .unwrap_or_default(),
        ),
        nonce: data["u"].as_i64(),
        bids,
        asks,
        buffered_deltas: std::collections::VecDeque::new(),
        bids_map: std::collections::BTreeMap::new(),
        asks_map: std::collections::BTreeMap::new(),
        is_synced: false,
        needs_resync: false,
        last_resync_time: 0,
        info: value_to_hashmap(data),
    })
}

/// Parse WebSocket orderbook delta data from Binance diff depth stream.
pub fn parse_ws_orderbook_delta(data: &Value, symbol: String) -> Result<OrderBookDelta> {
    let first_update_id = data["U"]
        .as_i64()
        .ok_or_else(|| Error::from(ParseError::missing_field("U (first_update_id)")))?;

    let final_update_id = data["u"]
        .as_i64()
        .ok_or_else(|| Error::from(ParseError::missing_field("u (final_update_id)")))?;

    let prev_final_update_id = data["pu"].as_i64();

    let timestamp = data["E"]
        .as_i64()
        .unwrap_or_else(|| chrono::Utc::now().timestamp_millis());

    let bids = parse_orderbook_side_ws(&data["b"])?;
    let asks = parse_orderbook_side_ws(&data["a"])?;

    Ok(OrderBookDelta {
        symbol,
        first_update_id,
        final_update_id,
        prev_final_update_id,
        timestamp,
        bids,
        asks,
    })
}

/// Parse one side (bids or asks) of orderbook data.
pub fn parse_orderbook_side(data: &Value) -> Result<Vec<OrderBookEntry>> {
    let array = data
        .as_array()
        .ok_or_else(|| Error::from(ParseError::invalid_value("data", "orderbook side")))?;

    let mut result = Vec::new();

    for item in array {
        if let Some(arr) = item.as_array() {
            if arr.len() >= 2 {
                let price = arr[0]
                    .as_str()
                    .and_then(|s| s.parse::<f64>().ok())
                    .and_then(Decimal::from_f64_retain)
                    .ok_or_else(|| Error::from(ParseError::invalid_value("data", "price")))?;
                let amount = arr[1]
                    .as_str()
                    .and_then(|s| s.parse::<f64>().ok())
                    .and_then(Decimal::from_f64_retain)
                    .ok_or_else(|| Error::from(ParseError::invalid_value("data", "amount")))?;
                result.push(OrderBookEntry {
                    price: Price::new(price),
                    amount: Amount::new(amount),
                });
            }
        }
    }

    Ok(result)
}

/// Parse one side (bids or asks) of WebSocket orderbook delta data.
pub fn parse_orderbook_side_ws(data: &Value) -> Result<Vec<OrderBookEntry>> {
    let Some(array) = data.as_array() else {
        return Ok(Vec::new());
    };

    let mut result = Vec::with_capacity(array.len());

    for item in array {
        if let Some(arr) = item.as_array() {
            if arr.len() >= 2 {
                let price = arr[0]
                    .as_str()
                    .and_then(|s| s.parse::<f64>().ok())
                    .and_then(Decimal::from_f64_retain)
                    .ok_or_else(|| Error::from(ParseError::invalid_value("data", "price")))?;
                let amount = arr[1]
                    .as_str()
                    .and_then(|s| s.parse::<f64>().ok())
                    .and_then(Decimal::from_f64_retain)
                    .ok_or_else(|| Error::from(ParseError::invalid_value("data", "amount")))?;
                result.push(OrderBookEntry {
                    price: Price::new(price),
                    amount: Amount::new(amount),
                });
            }
        }
    }

    Ok(result)
}
