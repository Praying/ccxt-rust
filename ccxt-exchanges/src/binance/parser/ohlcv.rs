use ccxt_core::{
    Result,
    error::{Error, ParseError},
    types::{
        OHLCV, Ohlcv,
        financial::{Amount, Price},
    },
};
use rust_decimal::Decimal;
use serde_json::Value;
use std::str::FromStr;

/// Parse OHLCV (candlestick/kline) data from Binance market API.
pub fn parse_ohlcv(data: &Value) -> Result<ccxt_core::types::OHLCV> {
    if let Some(array) = data.as_array() {
        if array.len() < 6 {
            return Err(Error::from(ParseError::invalid_format(
                "data",
                "OHLCV array length insufficient",
            )));
        }

        let timestamp = array[0]
            .as_i64()
            .ok_or_else(|| Error::from(ParseError::invalid_format("data", "无效的时间戳")))?;

        let open = array[1]
            .as_str()
            .and_then(|s| s.parse::<f64>().ok())
            .or_else(|| array[1].as_f64())
            .ok_or_else(|| Error::from(ParseError::invalid_format("data", "Invalid open price")))?;

        let high = array[2]
            .as_str()
            .and_then(|s| s.parse::<f64>().ok())
            .or_else(|| array[2].as_f64())
            .ok_or_else(|| Error::from(ParseError::invalid_format("data", "Invalid high price")))?;

        let low = array[3]
            .as_str()
            .and_then(|s| s.parse::<f64>().ok())
            .or_else(|| array[3].as_f64())
            .ok_or_else(|| Error::from(ParseError::invalid_format("data", "Invalid low price")))?;

        let close = array[4]
            .as_str()
            .and_then(|s| s.parse::<f64>().ok())
            .or_else(|| array[4].as_f64())
            .ok_or_else(|| {
                Error::from(ParseError::invalid_format("data", "Invalid close price"))
            })?;

        let volume = array[5]
            .as_str()
            .and_then(|s| s.parse::<f64>().ok())
            .or_else(|| array[5].as_f64())
            .ok_or_else(|| Error::from(ParseError::invalid_format("data", "Invalid volume")))?;

        Ok(OHLCV::new(timestamp, open, high, low, close, volume))
    } else {
        Err(Error::from(ParseError::invalid_format(
            "data",
            "OHLCV data must be in array format",
        )))
    }
}

/// Parse multiple OHLCV (candlestick/kline) data entries from Binance market API.
pub fn parse_ohlcvs(data: &Value) -> Result<Vec<ccxt_core::types::OHLCV>> {
    if let Some(array) = data.as_array() {
        array.iter().map(|item| parse_ohlcv(item)).collect()
    } else {
        Err(Error::from(ParseError::invalid_format(
            "data",
            "OHLCV data list must be in array format",
        )))
    }
}

/// Parse WebSocket OHLCV (candlestick/kline) data from Binance streams.
pub fn parse_ws_ohlcv(data: &Value) -> Result<OHLCV> {
    let kline = data["k"]
        .as_object()
        .ok_or_else(|| Error::from(ParseError::missing_field("k")))?;

    let timestamp = kline
        .get("t")
        .and_then(serde_json::Value::as_i64)
        .ok_or_else(|| Error::from(ParseError::missing_field("t")))?;

    let open = kline
        .get("o")
        .and_then(serde_json::Value::as_str)
        .and_then(|s| s.parse::<f64>().ok())
        .ok_or_else(|| Error::from(ParseError::missing_field("o")))?;

    let high = kline
        .get("h")
        .and_then(serde_json::Value::as_str)
        .and_then(|s| s.parse::<f64>().ok())
        .ok_or_else(|| Error::from(ParseError::missing_field("h")))?;

    let low = kline
        .get("l")
        .and_then(serde_json::Value::as_str)
        .and_then(|s| s.parse::<f64>().ok())
        .ok_or_else(|| Error::from(ParseError::missing_field("l")))?;

    let close = kline
        .get("c")
        .and_then(serde_json::Value::as_str)
        .and_then(|s| s.parse::<f64>().ok())
        .ok_or_else(|| Error::from(ParseError::missing_field("c")))?;

    let volume = kline
        .get("v")
        .and_then(serde_json::Value::as_str)
        .and_then(|s| s.parse::<f64>().ok())
        .ok_or_else(|| Error::from(ParseError::missing_field("v")))?;

    Ok(OHLCV {
        timestamp,
        open,
        high,
        low,
        close,
        volume,
    })
}

/// Helper to parse a JSON value (string or number) directly to Decimal.
fn parse_decimal_field(value: &Value, field_name: &str) -> Result<Decimal> {
    if let Some(s) = value.as_str() {
        Decimal::from_str(s).map_err(|_| {
            Error::from(ParseError::invalid_format(
                field_name.to_string(),
                format!("Cannot parse '{}' as Decimal", s),
            ))
        })
    } else if let Some(n) = value.as_f64() {
        // Fallback for numeric JSON values - convert via string to preserve digits
        Decimal::from_str(&n.to_string()).map_err(|_| {
            Error::from(ParseError::invalid_format(
                field_name.to_string(),
                format!("Cannot parse number {} as Decimal", n),
            ))
        })
    } else {
        Err(Error::from(ParseError::invalid_format(
            field_name.to_string(),
            "Expected string or number value",
        )))
    }
}

/// Parse a single OHLCV entry from Binance REST API directly to Decimal-based `Ohlcv`.
///
/// This avoids the f64 intermediate representation, preserving full precision
/// for financial data like "50000.12345678".
pub fn parse_ohlcv_decimal(data: &Value) -> Result<Ohlcv> {
    let array = data.as_array().ok_or_else(|| {
        Error::from(ParseError::invalid_format(
            "data",
            "OHLCV data must be in array format",
        ))
    })?;

    if array.len() < 6 {
        return Err(Error::from(ParseError::invalid_format(
            "data",
            "OHLCV array length insufficient",
        )));
    }

    let timestamp = array[0]
        .as_i64()
        .ok_or_else(|| Error::from(ParseError::invalid_format("timestamp", "Invalid timestamp")))?;

    let open = parse_decimal_field(&array[1], "open")?;
    let high = parse_decimal_field(&array[2], "high")?;
    let low = parse_decimal_field(&array[3], "low")?;
    let close = parse_decimal_field(&array[4], "close")?;
    let volume = parse_decimal_field(&array[5], "volume")?;

    Ok(Ohlcv {
        timestamp,
        open: Price::from(open),
        high: Price::from(high),
        low: Price::from(low),
        close: Price::from(close),
        volume: Amount::from(volume),
    })
}

/// Parse multiple OHLCV entries from Binance REST API directly to Decimal-based `Ohlcv`.
pub fn parse_ohlcvs_decimal(data: &Value) -> Result<Vec<Ohlcv>> {
    let array = data.as_array().ok_or_else(|| {
        Error::from(ParseError::invalid_format(
            "data",
            "OHLCV data list must be in array format",
        ))
    })?;

    array.iter().map(parse_ohlcv_decimal).collect()
}

/// Parse WebSocket OHLCV (kline) data directly to Decimal-based `Ohlcv`.
///
/// This avoids the f64 intermediate representation used by `parse_ws_ohlcv`.
pub fn parse_ws_ohlcv_decimal(data: &Value) -> Result<Ohlcv> {
    let kline = data["k"]
        .as_object()
        .ok_or_else(|| Error::from(ParseError::missing_field("k")))?;

    let timestamp = kline
        .get("t")
        .and_then(serde_json::Value::as_i64)
        .ok_or_else(|| Error::from(ParseError::missing_field("t")))?;

    let parse_kline_decimal = |key: &str| -> Result<Decimal> {
        let value = kline
            .get(key)
            .ok_or_else(|| Error::from(ParseError::missing_field_owned(key.to_string())))?;
        let s = value.as_str().ok_or_else(|| {
            Error::from(ParseError::invalid_format(
                key.to_string(),
                "Expected string value for kline field",
            ))
        })?;
        Decimal::from_str(s).map_err(|_| {
            Error::from(ParseError::invalid_format(
                key.to_string(),
                format!("Cannot parse '{}' as Decimal", s),
            ))
        })
    };

    let open = parse_kline_decimal("o")?;
    let high = parse_kline_decimal("h")?;
    let low = parse_kline_decimal("l")?;
    let close = parse_kline_decimal("c")?;
    let volume = parse_kline_decimal("v")?;

    Ok(Ohlcv {
        timestamp,
        open: Price::from(open),
        high: Price::from(high),
        low: Price::from(low),
        close: Price::from(close),
        volume: Amount::from(volume),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_parse_ohlcv_decimal_preserves_precision() {
        let data = json!([
            1637000000000i64,
            "50000.12345678",
            "51000.98765432",
            "49500.11111111",
            "50500.22222222",
            "123.45678901"
        ]);

        let ohlcv = parse_ohlcv_decimal(&data).unwrap();
        assert_eq!(ohlcv.timestamp, 1637000000000);
        assert_eq!(ohlcv.open.0, Decimal::from_str("50000.12345678").unwrap());
        assert_eq!(ohlcv.high.0, Decimal::from_str("51000.98765432").unwrap());
        assert_eq!(ohlcv.low.0, Decimal::from_str("49500.11111111").unwrap());
        assert_eq!(ohlcv.close.0, Decimal::from_str("50500.22222222").unwrap());
        assert_eq!(ohlcv.volume.0, Decimal::from_str("123.45678901").unwrap());
    }

    #[test]
    fn test_parse_ohlcvs_decimal_multiple_entries() {
        let data = json!([
            [
                1637000000000i64,
                "100.5",
                "110.0",
                "95.0",
                "105.0",
                "1000.0"
            ],
            [
                1637000060000i64,
                "105.0",
                "115.0",
                "100.0",
                "110.0",
                "2000.0"
            ]
        ]);

        let ohlcvs = parse_ohlcvs_decimal(&data).unwrap();
        assert_eq!(ohlcvs.len(), 2);
        assert_eq!(ohlcvs[0].open.0, Decimal::from_str("100.5").unwrap());
        assert_eq!(ohlcvs[1].open.0, Decimal::from_str("105.0").unwrap());
    }

    #[test]
    fn test_parse_ohlcv_decimal_invalid_string_returns_error() {
        let data = json!([
            1637000000000i64,
            "NaN",
            "51000.0",
            "49500.0",
            "50500.0",
            "123.0"
        ]);

        let result = parse_ohlcv_decimal(&data);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_ohlcv_decimal_empty_string_returns_error() {
        let data = json!([
            1637000000000i64,
            "",
            "51000.0",
            "49500.0",
            "50500.0",
            "123.0"
        ]);

        let result = parse_ohlcv_decimal(&data);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_ohlcv_decimal_insufficient_array() {
        let data = json!([1637000000000i64, "100.0", "110.0"]);

        let result = parse_ohlcv_decimal(&data);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_ohlcv_decimal_not_array() {
        let data = json!({"open": "100.0"});

        let result = parse_ohlcv_decimal(&data);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_ws_ohlcv_decimal_preserves_precision() {
        let data = json!({
            "e": "kline",
            "k": {
                "t": 1637000000000i64,
                "o": "50000.12345678",
                "h": "51000.98765432",
                "l": "49500.11111111",
                "c": "50500.22222222",
                "v": "123.45678901"
            }
        });

        let ohlcv = parse_ws_ohlcv_decimal(&data).unwrap();
        assert_eq!(ohlcv.timestamp, 1637000000000);
        assert_eq!(ohlcv.open.0, Decimal::from_str("50000.12345678").unwrap());
        assert_eq!(ohlcv.high.0, Decimal::from_str("51000.98765432").unwrap());
        assert_eq!(ohlcv.low.0, Decimal::from_str("49500.11111111").unwrap());
        assert_eq!(ohlcv.close.0, Decimal::from_str("50500.22222222").unwrap());
        assert_eq!(ohlcv.volume.0, Decimal::from_str("123.45678901").unwrap());
    }

    #[test]
    fn test_parse_ws_ohlcv_decimal_missing_field_returns_error() {
        let data = json!({
            "e": "kline",
            "k": {
                "t": 1637000000000i64,
                "o": "50000.0",
                "h": "51000.0"
                // missing l, c, v
            }
        });

        let result = parse_ws_ohlcv_decimal(&data);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_decimal_field_with_numeric_fallback() {
        let data = json!([1637000000000i64, 50000.5, 51000.0, 49500.0, 50500.0, 123.0]);

        let ohlcv = parse_ohlcv_decimal(&data).unwrap();
        assert_eq!(ohlcv.open.0, Decimal::from_str("50000.5").unwrap());
    }
}
