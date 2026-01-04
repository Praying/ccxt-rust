use ccxt_core::{
    Result,
    error::{Error, ParseError},
    types::OHLCV,
};
use serde_json::Value;

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
