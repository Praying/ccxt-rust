//! Binance data parser module.
//!
//! Converts Binance API response data into standardized CCXT format structures.

use rust_decimal::Decimal;
use rust_decimal::prelude::{FromPrimitive, FromStr};
use serde_json::Value;
use std::collections::HashMap;

// Re-export all parser functions
pub use balance::{parse_balance, parse_balance_with_type};
pub use funding::{parse_funding_rate, parse_funding_rate_history};
pub use margin::{parse_margin_adjustment, parse_margin_loan};
pub use market::{parse_currencies, parse_market};
pub use misc::{
    parse_mark_prices, parse_stats_24hr, parse_trading_fee, parse_trading_limits,
    parse_ws_mark_price,
};
pub use ohlcv::{
    parse_ohlcv_decimal, parse_ohlcvs, parse_ohlcvs_decimal, parse_ws_ohlcv, parse_ws_ohlcv_decimal,
};
pub use order::parse_order;
pub use orderbook::{parse_orderbook, parse_ws_orderbook};
pub use position::{parse_leverage, parse_leverage_tier, parse_position};
pub use ticker::{
    parse_bids_asks, parse_last_prices, parse_ticker, parse_ws_bid_ask, parse_ws_ticker,
};
pub use trade::{parse_agg_trade, parse_trade, parse_ws_trade};
pub use transaction::{parse_deposit_address, parse_deposit_withdraw_fees, parse_transaction};

mod balance;
mod funding;
mod margin;
mod market;
mod misc;
mod ohlcv;
mod order;
mod orderbook;
mod position;
mod ticker;
mod trade;
mod transaction;

// ============================================================================
// Helper Functions - Type Conversion
// ============================================================================

/// Parse an f64 value from JSON (supports both string and number formats).
pub(crate) fn parse_f64(data: &Value, key: &str) -> Option<f64> {
    data.get(key).and_then(|v| {
        v.as_f64()
            .or_else(|| v.as_str().and_then(|s| s.parse::<f64>().ok()))
    })
}

/// Parse a `Decimal` value from JSON (supports both string and number formats).
/// Prioritizes string parsing for maximum precision, falls back to f64 only when necessary.
pub(crate) fn parse_decimal(data: &Value, key: &str) -> Option<Decimal> {
    data.get(key).and_then(|v| {
        // Prioritize string parsing for maximum precision
        if let Some(s) = v.as_str() {
            Decimal::from_str(s).ok()
        } else if let Some(num) = v.as_f64() {
            // Fallback to f64 only when value is a JSON number
            Decimal::from_f64(num)
        } else {
            None
        }
    })
}

/// Parse a `Decimal` value from JSON, trying multiple keys in order.
/// Useful when different API responses use different field names for the same value.
pub(crate) fn parse_decimal_multi(data: &Value, keys: &[&str]) -> Option<Decimal> {
    for key in keys {
        if let Some(decimal) = parse_decimal(data, key) {
            return Some(decimal);
        }
    }
    None
}

/// Convert a JSON `Value` into a `HashMap<String, Value>`.
pub(crate) fn value_to_hashmap(data: &Value) -> HashMap<String, Value> {
    data.as_object()
        .map(|obj| obj.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
        .unwrap_or_default()
}

/// Parse an array of orderbook entries from JSON.
#[allow(dead_code)]
pub(crate) fn parse_order_book_entries(data: &Value) -> Vec<ccxt_core::types::OrderBookEntry> {
    use ccxt_core::types::{
        OrderBookEntry,
        financial::{Amount, Price},
    };

    data.as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|item| {
                    let price = if let Some(arr) = item.as_array() {
                        arr.first()
                            .and_then(serde_json::Value::as_str)
                            .and_then(|s| Decimal::from_str(s).ok())
                    } else {
                        None
                    }?;

                    let amount = if let Some(arr) = item.as_array() {
                        arr.get(1)
                            .and_then(serde_json::Value::as_str)
                            .and_then(|s| Decimal::from_str(s).ok())
                    } else {
                        None
                    }?;

                    Some(OrderBookEntry {
                        price: Price::new(price),
                        amount: Amount::new(amount),
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}
