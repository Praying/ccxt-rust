use super::{parse_decimal, parse_decimal_multi, value_to_hashmap};
use ccxt_core::{
    Result,
    error::{Error, ParseError},
    types::{
        Market, Ticker,
        financial::{Amount, Price},
    },
};
use serde_json::Value;

/// Parse ticker data from Binance 24hr ticker response.
pub fn parse_ticker(data: &Value, market: Option<&Market>) -> Result<Ticker> {
    let symbol = if let Some(m) = market {
        m.symbol.clone()
    } else {
        data["symbol"]
            .as_str()
            .ok_or_else(|| Error::from(ParseError::missing_field("symbol")))?
            .to_string()
    };

    let timestamp = data["closeTime"].as_i64();

    Ok(Ticker {
        symbol,
        timestamp: timestamp.unwrap_or(0),
        datetime: timestamp.map(|t| {
            chrono::DateTime::from_timestamp(t / 1000, 0)
                .map(|dt| dt.to_rfc3339())
                .unwrap_or_default()
        }),
        high: parse_decimal(data, "highPrice").map(Price::new),
        low: parse_decimal(data, "lowPrice").map(Price::new),
        bid: parse_decimal(data, "bidPrice").map(Price::new),
        bid_volume: parse_decimal(data, "bidQty").map(Amount::new),
        ask: parse_decimal(data, "askPrice").map(Price::new),
        ask_volume: parse_decimal(data, "askQty").map(Amount::new),
        vwap: parse_decimal(data, "weightedAvgPrice").map(Price::new),
        open: parse_decimal(data, "openPrice").map(Price::new),
        close: parse_decimal(data, "lastPrice").map(Price::new),
        last: parse_decimal(data, "lastPrice").map(Price::new),
        previous_close: parse_decimal(data, "prevClosePrice").map(Price::new),
        change: parse_decimal(data, "priceChange").map(Price::new),
        percentage: parse_decimal(data, "priceChangePercent"),
        average: None,
        base_volume: parse_decimal(data, "volume").map(Amount::new),
        quote_volume: parse_decimal(data, "quoteVolume").map(Amount::new),
        funding_rate: None,
        open_interest: None,
        index_price: None,
        mark_price: None,
        info: value_to_hashmap(data),
    })
}

/// Parse WebSocket ticker data from Binance streams.
pub fn parse_ws_ticker(data: &Value, market: Option<&Market>) -> Result<Ticker> {
    let market_id = data["s"]
        .as_str()
        .or_else(|| data["symbol"].as_str())
        .ok_or_else(|| Error::from(ParseError::missing_field("symbol")))?;

    let symbol = if let Some(m) = market {
        m.symbol.clone()
    } else {
        market_id.to_string()
    };

    let event = data["e"].as_str().unwrap_or("bookTicker");

    if event == "markPriceUpdate" {
        let timestamp = data["E"].as_i64().unwrap_or(0);
        return Ok(Ticker {
            symbol,
            timestamp,
            datetime: Some(
                chrono::DateTime::from_timestamp_millis(timestamp)
                    .map(|dt| dt.to_rfc3339())
                    .unwrap_or_default(),
            ),
            high: None,
            low: None,
            bid: None,
            bid_volume: None,
            ask: None,
            ask_volume: None,
            vwap: None,
            open: None,
            close: parse_decimal(data, "p").map(Price::from),
            last: parse_decimal(data, "p").map(Price::from),
            previous_close: None,
            change: None,
            percentage: None,
            average: None,
            base_volume: None,
            quote_volume: None,
            funding_rate: parse_decimal(data, "r"),
            open_interest: None,
            index_price: parse_decimal(data, "i").map(Price::from),
            mark_price: parse_decimal(data, "p").map(Price::from),
            info: value_to_hashmap(data),
        });
    }

    let timestamp = if event == "bookTicker" {
        data["E"]
            .as_i64()
            .or_else(|| data["time"].as_i64())
            .unwrap_or_else(|| chrono::Utc::now().timestamp_millis())
    } else {
        data["C"]
            .as_i64()
            .or_else(|| data["E"].as_i64())
            .or_else(|| data["time"].as_i64())
            .unwrap_or_else(|| chrono::Utc::now().timestamp_millis())
    };

    let last = parse_decimal_multi(data, &["c", "price"]);

    Ok(Ticker {
        symbol,
        timestamp,
        datetime: Some(
            chrono::DateTime::from_timestamp_millis(timestamp)
                .map(|dt| dt.to_rfc3339())
                .unwrap_or_default(),
        ),
        high: parse_decimal(data, "h").map(Price::from),
        low: parse_decimal(data, "l").map(Price::from),
        bid: parse_decimal_multi(data, &["b", "bidPrice"]).map(Price::from),
        bid_volume: parse_decimal_multi(data, &["B", "bidQty"]).map(Amount::from),
        ask: parse_decimal_multi(data, &["a", "askPrice"]).map(Price::from),
        ask_volume: parse_decimal_multi(data, &["A", "askQty"]).map(Amount::from),
        vwap: parse_decimal(data, "w").map(Price::from),
        open: parse_decimal(data, "o").map(Price::from),
        close: last.map(Price::from),
        last: last.map(Price::from),
        previous_close: parse_decimal(data, "x").map(Price::from),
        change: parse_decimal(data, "p").map(Price::from),
        percentage: parse_decimal(data, "P"),
        average: None,
        base_volume: parse_decimal(data, "v").map(Amount::from),
        quote_volume: parse_decimal(data, "q").map(Amount::from),
        funding_rate: None,
        open_interest: None,
        index_price: None,
        mark_price: None,
        info: value_to_hashmap(data),
    })
}

/// Parse bid/ask price data from Binance ticker API.
pub fn parse_bid_ask(data: &Value) -> Result<ccxt_core::types::BidAsk> {
    use ccxt_core::types::BidAsk;
    use rust_decimal::Decimal;

    let symbol = data["symbol"]
        .as_str()
        .ok_or_else(|| Error::from(ParseError::missing_field("symbol")))?
        .to_string();

    let formatted_symbol = if symbol.len() >= 6 {
        let quote_currencies = ["USDT", "BUSD", "USDC", "BTC", "ETH", "BNB"];
        let mut found = false;
        let mut formatted = symbol.clone();

        for quote in &quote_currencies {
            if symbol.ends_with(quote) {
                let base = &symbol[..symbol.len() - quote.len()];
                formatted = format!("{}/{}", base, quote);
                found = true;
                break;
            }
        }

        if found { formatted } else { symbol.clone() }
    } else {
        symbol.clone()
    };

    let bid_price = parse_decimal(data, "bidPrice").unwrap_or(Decimal::ZERO);
    let bid_quantity = parse_decimal(data, "bidQty").unwrap_or(Decimal::ZERO);
    let ask_price = parse_decimal(data, "askPrice").unwrap_or(Decimal::ZERO);
    let ask_quantity = parse_decimal(data, "askQty").unwrap_or(Decimal::ZERO);

    let timestamp = data["time"]
        .as_i64()
        .unwrap_or_else(|| chrono::Utc::now().timestamp_millis());

    Ok(BidAsk {
        symbol: formatted_symbol,
        bid_price,
        bid_quantity,
        ask_price,
        ask_quantity,
        timestamp,
    })
}

/// Parse multiple bid/ask price data entries from Binance ticker API.
pub fn parse_bids_asks(data: &Value) -> Result<Vec<ccxt_core::types::BidAsk>> {
    if let Some(array) = data.as_array() {
        array.iter().map(|item| parse_bid_ask(item)).collect()
    } else {
        Ok(vec![parse_bid_ask(data)?])
    }
}

/// Parse latest price data from Binance ticker API.
pub fn parse_last_price(data: &Value) -> Result<ccxt_core::types::LastPrice> {
    use ccxt_core::types::LastPrice;
    use rust_decimal::Decimal;

    let symbol = data["symbol"]
        .as_str()
        .ok_or_else(|| Error::from(ParseError::missing_field("symbol")))?
        .to_string();

    let formatted_symbol = if symbol.len() >= 6 {
        let quote_currencies = ["USDT", "BUSD", "USDC", "BTC", "ETH", "BNB"];
        let mut found = false;
        let mut formatted = symbol.clone();

        for quote in &quote_currencies {
            if symbol.ends_with(quote) {
                let base = &symbol[..symbol.len() - quote.len()];
                formatted = format!("{}/{}", base, quote);
                found = true;
                break;
            }
        }

        if found { formatted } else { symbol.clone() }
    } else {
        symbol.clone()
    };

    let price = parse_decimal(data, "price").unwrap_or(Decimal::ZERO);

    let timestamp = data["time"]
        .as_i64()
        .unwrap_or_else(|| chrono::Utc::now().timestamp_millis());

    let datetime = chrono::DateTime::from_timestamp_millis(timestamp)
        .map(|dt| dt.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string())
        .unwrap_or_default();

    Ok(LastPrice {
        symbol: formatted_symbol,
        price,
        timestamp,
        datetime,
    })
}

/// Parse multiple latest price data entries from Binance ticker API.
pub fn parse_last_prices(data: &Value) -> Result<Vec<ccxt_core::types::LastPrice>> {
    if let Some(array) = data.as_array() {
        array.iter().map(|item| parse_last_price(item)).collect()
    } else {
        Ok(vec![parse_last_price(data)?])
    }
}

/// Parse WebSocket BidAsk (best bid/ask prices) data from Binance streams.
pub fn parse_ws_bid_ask(data: &Value) -> Result<ccxt_core::types::BidAsk> {
    use ccxt_core::types::BidAsk;
    use rust_decimal::Decimal;
    use rust_decimal::prelude::FromStr;

    let symbol = data["s"]
        .as_str()
        .ok_or_else(|| Error::from(ParseError::missing_field("symbol")))?
        .to_string();

    let bid_price = data["b"]
        .as_str()
        .and_then(|s| Decimal::from_str(s).ok())
        .ok_or_else(|| Error::from(ParseError::missing_field("bid_price")))?;

    let bid_quantity = data["B"]
        .as_str()
        .and_then(|s| Decimal::from_str(s).ok())
        .ok_or_else(|| Error::from(ParseError::missing_field("bid_quantity")))?;

    let ask_price = data["a"]
        .as_str()
        .and_then(|s| Decimal::from_str(s).ok())
        .ok_or_else(|| Error::from(ParseError::missing_field("ask_price")))?;

    let ask_quantity = data["A"]
        .as_str()
        .and_then(|s| Decimal::from_str(s).ok())
        .ok_or_else(|| Error::from(ParseError::missing_field("ask_quantity")))?;

    let timestamp = data["E"].as_i64().unwrap_or(0);

    Ok(BidAsk {
        symbol,
        bid_price,
        bid_quantity,
        ask_price,
        ask_quantity,
        timestamp,
    })
}
