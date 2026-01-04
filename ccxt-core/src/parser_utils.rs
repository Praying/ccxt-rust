//! Common parser utilities for exchange data parsing.
//!
//! This module provides shared utility functions for parsing JSON data
//! from exchange API responses into Rust types.

use rust_decimal::Decimal;
use rust_decimal::prelude::{FromPrimitive, FromStr};
use serde_json::Value;
use std::collections::HashMap;

/// Parse a `Decimal` value from JSON (supports both string and number formats).
///
/// Empty strings are treated as `None`.
pub fn parse_decimal(data: &Value, key: &str) -> Option<Decimal> {
    data.get(key).and_then(|v| {
        if let Some(num) = v.as_f64() {
            Decimal::from_f64(num)
        } else if let Some(s) = v.as_str() {
            if s.is_empty() {
                None
            } else {
                Decimal::from_str(s).ok()
            }
        } else {
            None
        }
    })
}

/// Parse a timestamp from JSON (supports both string and number formats).
pub fn parse_timestamp(data: &Value, key: &str) -> Option<i64> {
    data.get(key).and_then(|v| {
        v.as_i64()
            .or_else(|| v.as_str().and_then(|s| s.parse::<i64>().ok()))
    })
}

/// Convert a JSON `Value` into a `HashMap<String, Value>`.
pub fn value_to_hashmap(data: &Value) -> HashMap<String, Value> {
    data.as_object()
        .map(|obj| obj.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
        .unwrap_or_default()
}

/// Convert millisecond timestamp to ISO8601 datetime string.
pub fn timestamp_to_datetime(timestamp: i64) -> Option<String> {
    chrono::DateTime::from_timestamp_millis(timestamp)
        .map(|dt| dt.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string())
}

/// Convert ISO8601 datetime string to millisecond timestamp.
pub fn datetime_to_timestamp(datetime: &str) -> Option<i64> {
    chrono::DateTime::parse_from_rfc3339(datetime)
        .ok()
        .map(|dt| dt.timestamp_millis())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_parse_decimal_from_string() {
        let data = json!({"price": "123.45"});
        assert_eq!(
            parse_decimal(&data, "price"),
            Some(Decimal::from_str("123.45").unwrap())
        );
    }

    #[test]
    fn test_parse_decimal_from_number() {
        let data = json!({"price": 123.45});
        assert!(parse_decimal(&data, "price").is_some());
    }

    #[test]
    fn test_parse_decimal_empty_string() {
        let data = json!({"price": ""});
        assert_eq!(parse_decimal(&data, "price"), None);
    }

    #[test]
    fn test_parse_decimal_missing_key() {
        let data = json!({"other": "123"});
        assert_eq!(parse_decimal(&data, "price"), None);
    }

    #[test]
    fn test_parse_timestamp_from_number() {
        let data = json!({"time": 1704110400000i64});
        assert_eq!(parse_timestamp(&data, "time"), Some(1704110400000));
    }

    #[test]
    fn test_parse_timestamp_from_string() {
        let data = json!({"time": "1704110400000"});
        assert_eq!(parse_timestamp(&data, "time"), Some(1704110400000));
    }

    #[test]
    fn test_value_to_hashmap() {
        let data = json!({"a": 1, "b": "two"});
        let map = value_to_hashmap(&data);
        assert_eq!(map.len(), 2);
        assert_eq!(map.get("a"), Some(&json!(1)));
    }

    #[test]
    fn test_timestamp_to_datetime() {
        let result = timestamp_to_datetime(1704110400000);
        assert!(result.is_some());
        assert!(result.unwrap().contains("2024-01-01"));
    }

    #[test]
    fn test_datetime_to_timestamp() {
        let result = datetime_to_timestamp("2024-01-01T12:00:00.000Z");
        assert!(result.is_some());
    }
}
