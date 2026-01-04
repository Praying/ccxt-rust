#![allow(dead_code)]

use super::{parse_decimal_multi, parse_f64, value_to_hashmap};
use ccxt_core::{
    Result,
    error::{Error, ParseError},
    types::{
        Market, OrderSide, TakerOrMaker, Trade,
        financial::{Amount, Cost, Price},
    },
};
use rust_decimal::Decimal;
use serde_json::Value;

/// Parse trade data from Binance trade response.
pub fn parse_trade(data: &Value, market: Option<&Market>) -> Result<Trade> {
    let symbol = if let Some(m) = market {
        m.symbol.clone()
    } else {
        data["symbol"]
            .as_str()
            .ok_or_else(|| Error::from(ParseError::missing_field("symbol")))?
            .to_string()
    };

    let id = data["id"]
        .as_u64()
        .or_else(|| data["a"].as_u64())
        .map(|v| v.to_string());

    let timestamp = data["time"].as_i64().or_else(|| data["T"].as_i64());

    let side = if data["isBuyerMaker"].as_bool().unwrap_or(false)
        || data["m"].as_bool().unwrap_or(false)
    {
        OrderSide::Sell
    } else {
        OrderSide::Buy
    };

    let price = parse_decimal_multi(data, &["price", "p"])
        .ok_or_else(|| Error::from(ParseError::missing_field("price")))?;
    let amount = parse_decimal_multi(data, &["qty", "q"])
        .ok_or_else(|| Error::from(ParseError::missing_field("amount")))?;

    let cost = Some(price * amount);

    Ok(Trade {
        id,
        order: data["orderId"]
            .as_u64()
            .or_else(|| data["orderid"].as_u64())
            .map(|v| v.to_string()),
        timestamp: timestamp.unwrap_or(0),
        datetime: timestamp.map(|t| {
            chrono::DateTime::from_timestamp(t / 1000, 0)
                .map(|dt| dt.to_rfc3339())
                .unwrap_or_default()
        }),
        symbol,
        trade_type: None,
        side,
        taker_or_maker: if data["isBuyerMaker"].as_bool().unwrap_or(false) {
            Some(TakerOrMaker::Maker)
        } else {
            Some(TakerOrMaker::Taker)
        },
        price: Price::new(price),
        amount: Amount::new(amount),
        cost: cost.map(Cost::new),
        fee: None,
        info: value_to_hashmap(data),
    })
}

/// Parse WebSocket trade data from Binance streams.
pub fn parse_ws_trade(data: &Value, market: Option<&Market>) -> Result<Trade> {
    let market_id = data["s"]
        .as_str()
        .or_else(|| data["symbol"].as_str())
        .ok_or_else(|| Error::from(ParseError::missing_field("symbol")))?;

    let symbol = if let Some(m) = market {
        m.symbol.clone()
    } else {
        market_id.to_string()
    };

    let id = data["t"]
        .as_u64()
        .or_else(|| data["a"].as_u64())
        .map(|v| v.to_string());

    let timestamp = data["T"].as_i64().unwrap_or(0);

    let price = parse_f64(data, "L")
        .or_else(|| parse_f64(data, "p"))
        .and_then(Decimal::from_f64_retain);

    let amount = parse_f64(data, "q").and_then(Decimal::from_f64_retain);

    let cost = match (price, amount) {
        (Some(p), Some(a)) => Some(p * a),
        _ => None,
    };

    let side = if data["m"].as_bool().unwrap_or(false) {
        OrderSide::Sell
    } else {
        OrderSide::Buy
    };

    let taker_or_maker = if data["m"].as_bool().unwrap_or(false) {
        Some(TakerOrMaker::Maker)
    } else {
        Some(TakerOrMaker::Taker)
    };

    Ok(Trade {
        id,
        order: data["orderId"]
            .as_u64()
            .or_else(|| data["orderid"].as_u64())
            .map(|v| v.to_string()),
        timestamp,
        datetime: Some(
            chrono::DateTime::from_timestamp_millis(timestamp)
                .map(|dt| dt.to_rfc3339())
                .unwrap_or_default(),
        ),
        symbol,
        trade_type: None,
        side,
        taker_or_maker,
        price: Price::from(price.unwrap_or(Decimal::ZERO)),
        amount: Amount::from(amount.unwrap_or(Decimal::ZERO)),
        cost: cost.map(Cost::from),
        fee: None,
        info: value_to_hashmap(data),
    })
}

/// Parse aggregated trade from Binance API response.
pub fn parse_agg_trade(data: &Value, symbol: Option<String>) -> Result<ccxt_core::types::AggTrade> {
    use ccxt_core::types::AggTrade;
    use rust_decimal::prelude::FromStr;

    let agg_id = data["a"]
        .as_i64()
        .ok_or_else(|| Error::from(ParseError::missing_field("a")))?;

    let price = data["p"]
        .as_str()
        .and_then(|s| Decimal::from_str(s).ok())
        .ok_or_else(|| Error::from(ParseError::missing_field("p")))?;

    let quantity = data["q"]
        .as_str()
        .and_then(|s| Decimal::from_str(s).ok())
        .ok_or_else(|| Error::from(ParseError::missing_field("q")))?;

    let first_trade_id = data["f"]
        .as_i64()
        .ok_or_else(|| Error::from(ParseError::missing_field("f")))?;

    let last_trade_id = data["l"]
        .as_i64()
        .ok_or_else(|| Error::from(ParseError::missing_field("l")))?;

    let timestamp = data["T"]
        .as_i64()
        .ok_or_else(|| Error::from(ParseError::missing_field("T")))?;

    let is_buyer_maker = data["m"].as_bool().unwrap_or(false);
    let is_best_match = data["M"].as_bool();

    Ok(AggTrade {
        agg_id,
        price,
        quantity,
        first_trade_id,
        last_trade_id,
        timestamp,
        is_buyer_maker,
        is_best_match,
        symbol,
    })
}

/// Parse order trade history from Binance myTrades endpoint.
pub fn parse_order_trades(
    data: &Value,
    market: Option<&Market>,
) -> Result<Vec<ccxt_core::types::Trade>> {
    if let Some(array) = data.as_array() {
        array.iter().map(|item| parse_trade(item, market)).collect()
    } else {
        Ok(vec![parse_trade(data, market)?])
    }
}
