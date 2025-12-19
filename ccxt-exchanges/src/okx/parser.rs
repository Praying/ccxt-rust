//! OKX data parser module.
//!
//! Converts OKX API response data into standardized CCXT format structures.

use ccxt_core::{
    Result,
    error::{Error, ParseError},
    types::{
        Balance, BalanceEntry, Market, MarketLimits, MarketPrecision, MarketType, MinMax, OHLCV,
        Order, OrderBook, OrderBookEntry, OrderSide, OrderStatus, OrderType, Ticker, Trade,
        financial::{Amount, Cost, Price},
    },
};
use rust_decimal::Decimal;
use rust_decimal::prelude::{FromPrimitive, FromStr};
use serde_json::Value;
use std::collections::HashMap;

// ============================================================================
// Helper Functions - Type Conversion
// ============================================================================

/// Parse a `Decimal` value from JSON (supports both string and number formats).
fn parse_decimal(data: &Value, key: &str) -> Option<Decimal> {
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
fn parse_timestamp(data: &Value, key: &str) -> Option<i64> {
    data.get(key).and_then(|v| {
        v.as_i64()
            .or_else(|| v.as_str().and_then(|s| s.parse::<i64>().ok()))
    })
}

/// Convert a JSON `Value` into a `HashMap<String, Value>`.
fn value_to_hashmap(data: &Value) -> HashMap<String, Value> {
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

// ============================================================================
// Market Data Parser Functions
// ============================================================================

/// Parse market data from OKX exchange info.
///
/// OKX uses `instId` for instrument ID (e.g., "BTC-USDT").
///
/// # Arguments
///
/// * `data` - OKX market data JSON object
///
/// # Returns
///
/// Returns a CCXT [`Market`] structure.
pub fn parse_market(data: &Value) -> Result<Market> {
    // OKX uses "instId" for the exchange-specific ID (e.g., "BTC-USDT")
    let id = data["instId"]
        .as_str()
        .ok_or_else(|| Error::from(ParseError::missing_field("instId")))?
        .to_string();

    // Base and quote currencies
    let base = data["baseCcy"]
        .as_str()
        .ok_or_else(|| Error::from(ParseError::missing_field("baseCcy")))?
        .to_string();

    let quote = data["quoteCcy"]
        .as_str()
        .ok_or_else(|| Error::from(ParseError::missing_field("quoteCcy")))?
        .to_string();

    // Instrument type
    let inst_type = data["instType"].as_str().unwrap_or("SPOT");
    let market_type = match inst_type {
        "SPOT" => MarketType::Spot,
        "SWAP" => MarketType::Swap,
        "FUTURES" => MarketType::Futures,
        "OPTION" => MarketType::Option,
        _ => MarketType::Spot,
    };

    // Market status
    let state = data["state"].as_str().unwrap_or("live");
    let active = state == "live";

    // Parse precision - OKX uses tickSz and lotSz
    let price_precision = parse_decimal(data, "tickSz");
    let amount_precision = parse_decimal(data, "lotSz");

    // Parse limits
    let min_amount = parse_decimal(data, "minSz");
    let max_amount = parse_decimal(data, "maxLmtSz");

    // Contract-specific fields
    let contract = inst_type != "SPOT";
    let linear = if contract {
        Some(data["ctType"].as_str() == Some("linear"))
    } else {
        None
    };
    let inverse = if contract {
        Some(data["ctType"].as_str() == Some("inverse"))
    } else {
        None
    };
    let contract_size = parse_decimal(data, "ctVal");

    // Settlement currency for derivatives
    let settle = data["settleCcy"].as_str().map(|s| s.to_string());
    let settle_id = settle.clone();

    // Expiry for futures/options
    let expiry = parse_timestamp(data, "expTime");
    let expiry_datetime = expiry.and_then(timestamp_to_datetime);

    // Build unified symbol format based on market type:
    // - Spot: BASE/QUOTE (e.g., "BTC/USDT")
    // - Swap: BASE/QUOTE:SETTLE (e.g., "BTC/USDT:USDT")
    // - Futures: BASE/QUOTE:SETTLE-YYMMDD (e.g., "BTC/USDT:USDT-241231")
    let symbol = match market_type {
        MarketType::Spot => format!("{}/{}", base, quote),
        MarketType::Swap => {
            if let Some(ref s) = settle {
                format!("{}/{}:{}", base, quote, s)
            } else {
                // Fallback: use quote as settle for linear
                format!("{}/{}:{}", base, quote, quote)
            }
        }
        MarketType::Futures | MarketType::Option => {
            if let (Some(s), Some(exp_ts)) = (&settle, expiry) {
                // Convert timestamp to YYMMDD format
                if let Some(dt) = chrono::DateTime::from_timestamp_millis(exp_ts) {
                    let year = (dt.format("%y").to_string().parse::<u8>()).unwrap_or(0);
                    let month = (dt.format("%m").to_string().parse::<u8>()).unwrap_or(1);
                    let day = (dt.format("%d").to_string().parse::<u8>()).unwrap_or(1);
                    format!("{}/{}:{}-{:02}{:02}{:02}", base, quote, s, year, month, day)
                } else {
                    format!("{}/{}:{}", base, quote, s)
                }
            } else if let Some(ref s) = settle {
                format!("{}/{}:{}", base, quote, s)
            } else {
                format!("{}/{}", base, quote)
            }
        }
    };

    // Parse the symbol to get structured representation
    let parsed_symbol = ccxt_core::symbol::SymbolParser::parse(&symbol).ok();

    Ok(Market {
        id,
        symbol,
        parsed_symbol,
        base: base.clone(),
        quote: quote.clone(),
        settle,
        base_id: Some(base),
        quote_id: Some(quote),
        settle_id,
        market_type,
        active,
        margin: inst_type == "MARGIN",
        contract: Some(contract),
        linear,
        inverse,
        contract_size,
        expiry,
        expiry_datetime,
        strike: parse_decimal(data, "stk"),
        option_type: data["optType"].as_str().map(|s| s.to_string()),
        precision: MarketPrecision {
            price: price_precision,
            amount: amount_precision,
            base: None,
            quote: None,
        },
        limits: MarketLimits {
            amount: Some(MinMax {
                min: min_amount,
                max: max_amount,
            }),
            price: None,
            cost: None,
            leverage: None,
        },
        maker: parse_decimal(data, "makerFee"),
        taker: parse_decimal(data, "takerFee"),
        percentage: Some(true),
        tier_based: Some(false),
        fee_side: Some("quote".to_string()),
        info: value_to_hashmap(data),
    })
}

/// Parse ticker data from OKX ticker response.
///
/// # Arguments
///
/// * `data` - OKX ticker data JSON object
/// * `market` - Optional market information for symbol resolution
///
/// # Returns
///
/// Returns a CCXT [`Ticker`] structure.
pub fn parse_ticker(data: &Value, market: Option<&Market>) -> Result<Ticker> {
    let symbol = if let Some(m) = market {
        m.symbol.clone()
    } else {
        // Try to construct symbol from instId
        data["instId"]
            .as_str()
            .map(|s| s.replace('-', "/"))
            .unwrap_or_default()
    };

    // OKX uses "ts" for timestamp
    let timestamp = parse_timestamp(data, "ts").unwrap_or(0);

    Ok(Ticker {
        symbol,
        timestamp,
        datetime: timestamp_to_datetime(timestamp),
        high: parse_decimal(data, "high24h").map(Price::new),
        low: parse_decimal(data, "low24h").map(Price::new),
        bid: parse_decimal(data, "bidPx").map(Price::new),
        bid_volume: parse_decimal(data, "bidSz").map(Amount::new),
        ask: parse_decimal(data, "askPx").map(Price::new),
        ask_volume: parse_decimal(data, "askSz").map(Amount::new),
        vwap: None,
        open: parse_decimal(data, "open24h")
            .or_else(|| parse_decimal(data, "sodUtc0"))
            .map(Price::new),
        close: parse_decimal(data, "last").map(Price::new),
        last: parse_decimal(data, "last").map(Price::new),
        previous_close: None,
        change: None, // OKX doesn't provide direct change value
        percentage: parse_decimal(data, "sodUtc0").and_then(|open| {
            parse_decimal(data, "last").map(|last| {
                if open.is_zero() {
                    Decimal::ZERO
                } else {
                    ((last - open) / open) * Decimal::from(100)
                }
            })
        }),
        average: None,
        base_volume: parse_decimal(data, "vol24h")
            .or_else(|| parse_decimal(data, "volCcy24h"))
            .map(Amount::new),
        quote_volume: parse_decimal(data, "volCcy24h").map(Amount::new),
        info: value_to_hashmap(data),
    })
}

/// Parse orderbook data from OKX depth response.
///
/// # Arguments
///
/// * `data` - OKX orderbook data JSON object
/// * `symbol` - Trading pair symbol
///
/// # Returns
///
/// Returns a CCXT [`OrderBook`] structure with bids sorted in descending order
/// and asks sorted in ascending order.
pub fn parse_orderbook(data: &Value, symbol: String) -> Result<OrderBook> {
    let timestamp =
        parse_timestamp(data, "ts").unwrap_or_else(|| chrono::Utc::now().timestamp_millis());

    let mut bids = parse_orderbook_side(&data["bids"])?;
    let mut asks = parse_orderbook_side(&data["asks"])?;

    // Sort bids in descending order (highest price first)
    bids.sort_by(|a, b| b.price.cmp(&a.price));

    // Sort asks in ascending order (lowest price first)
    asks.sort_by(|a, b| a.price.cmp(&b.price));

    Ok(OrderBook {
        symbol,
        timestamp,
        datetime: timestamp_to_datetime(timestamp),
        nonce: None,
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

/// Parse one side (bids or asks) of orderbook data.
fn parse_orderbook_side(data: &Value) -> Result<Vec<OrderBookEntry>> {
    let Some(array) = data.as_array() else {
        return Ok(Vec::new());
    };

    let mut result = Vec::new();

    for item in array {
        if let Some(arr) = item.as_array() {
            // OKX format: [price, size, liquidated_orders, num_orders]
            if arr.len() >= 2 {
                let price = arr[0]
                    .as_str()
                    .and_then(|s| Decimal::from_str(s).ok())
                    .or_else(|| arr[0].as_f64().and_then(Decimal::from_f64))
                    .ok_or_else(|| Error::from(ParseError::invalid_value("data", "price")))?;

                let amount = arr[1]
                    .as_str()
                    .and_then(|s| Decimal::from_str(s).ok())
                    .or_else(|| arr[1].as_f64().and_then(Decimal::from_f64))
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

/// Parse trade data from OKX trade response.
///
/// # Arguments
///
/// * `data` - OKX trade data JSON object
/// * `market` - Optional market information for symbol resolution
///
/// # Returns
///
/// Returns a CCXT [`Trade`] structure.
pub fn parse_trade(data: &Value, market: Option<&Market>) -> Result<Trade> {
    let symbol = if let Some(m) = market {
        m.symbol.clone()
    } else {
        data["instId"]
            .as_str()
            .map(|s| s.replace('-', "/"))
            .unwrap_or_default()
    };

    let id = data["tradeId"].as_str().map(|s| s.to_string());

    let timestamp = parse_timestamp(data, "ts").unwrap_or(0);

    // OKX uses "side" field with "buy" or "sell" values
    let side = match data["side"].as_str() {
        Some("buy") | Some("Buy") | Some("BUY") => OrderSide::Buy,
        Some("sell") | Some("Sell") | Some("SELL") => OrderSide::Sell,
        _ => OrderSide::Buy, // Default to buy if not specified
    };

    let price = parse_decimal(data, "px").or_else(|| parse_decimal(data, "fillPx"));
    let amount = parse_decimal(data, "sz").or_else(|| parse_decimal(data, "fillSz"));

    let cost = match (price, amount) {
        (Some(p), Some(a)) => Some(p * a),
        _ => None,
    };

    Ok(Trade {
        id,
        order: data["ordId"].as_str().map(|s| s.to_string()),
        timestamp,
        datetime: timestamp_to_datetime(timestamp),
        symbol,
        trade_type: None,
        side,
        taker_or_maker: None,
        price: Price::new(price.unwrap_or(Decimal::ZERO)),
        amount: Amount::new(amount.unwrap_or(Decimal::ZERO)),
        cost: cost.map(Cost::new),
        fee: None,
        info: value_to_hashmap(data),
    })
}

/// Parse OHLCV (candlestick) data from OKX kline response.
///
/// # Arguments
///
/// * `data` - OKX OHLCV data JSON array
///
/// # Returns
///
/// Returns a CCXT [`OHLCV`] structure.
pub fn parse_ohlcv(data: &Value) -> Result<OHLCV> {
    // OKX returns OHLCV as array: [ts, o, h, l, c, vol, volCcy, volCcyQuote, confirm]
    let arr = data
        .as_array()
        .ok_or_else(|| Error::from(ParseError::invalid_format("data", "OHLCV array")))?;

    if arr.len() < 6 {
        return Err(Error::from(ParseError::invalid_format(
            "data",
            "OHLCV array with at least 6 elements",
        )));
    }

    let timestamp = arr[0]
        .as_str()
        .and_then(|s| s.parse::<i64>().ok())
        .or_else(|| arr[0].as_i64())
        .ok_or_else(|| Error::from(ParseError::invalid_value("data", "timestamp")))?;

    let open = arr[1]
        .as_str()
        .and_then(|s| s.parse::<f64>().ok())
        .or_else(|| arr[1].as_f64())
        .ok_or_else(|| Error::from(ParseError::invalid_value("data", "open")))?;

    let high = arr[2]
        .as_str()
        .and_then(|s| s.parse::<f64>().ok())
        .or_else(|| arr[2].as_f64())
        .ok_or_else(|| Error::from(ParseError::invalid_value("data", "high")))?;

    let low = arr[3]
        .as_str()
        .and_then(|s| s.parse::<f64>().ok())
        .or_else(|| arr[3].as_f64())
        .ok_or_else(|| Error::from(ParseError::invalid_value("data", "low")))?;

    let close = arr[4]
        .as_str()
        .and_then(|s| s.parse::<f64>().ok())
        .or_else(|| arr[4].as_f64())
        .ok_or_else(|| Error::from(ParseError::invalid_value("data", "close")))?;

    let volume = arr[5]
        .as_str()
        .and_then(|s| s.parse::<f64>().ok())
        .or_else(|| arr[5].as_f64())
        .ok_or_else(|| Error::from(ParseError::invalid_value("data", "volume")))?;

    Ok(OHLCV {
        timestamp,
        open,
        high,
        low,
        close,
        volume,
    })
}

// ============================================================================
// Order and Balance Parser Functions
// ============================================================================

/// Map OKX order status to CCXT OrderStatus.
///
/// OKX order states:
/// - live: Order is active
/// - partially_filled: Order is partially filled
/// - filled: Order is completely filled
/// - canceled: Order is canceled
/// - mmp_canceled: Order is canceled by MMP
///
/// # Arguments
///
/// * `status` - OKX order status string
///
/// # Returns
///
/// Returns the corresponding CCXT [`OrderStatus`].
pub fn parse_order_status(status: &str) -> OrderStatus {
    match status.to_lowercase().as_str() {
        "live" => OrderStatus::Open,
        "partially_filled" => OrderStatus::Open,
        "filled" => OrderStatus::Closed,
        "canceled" | "cancelled" => OrderStatus::Cancelled,
        "mmp_canceled" => OrderStatus::Cancelled,
        "expired" => OrderStatus::Expired,
        "rejected" => OrderStatus::Rejected,
        _ => OrderStatus::Open, // Default to Open for unknown statuses
    }
}

/// Parse order data from OKX order response.
///
/// # Arguments
///
/// * `data` - OKX order data JSON object
/// * `market` - Optional market information for symbol resolution
///
/// # Returns
///
/// Returns a CCXT [`Order`] structure.
pub fn parse_order(data: &Value, market: Option<&Market>) -> Result<Order> {
    let symbol = if let Some(m) = market {
        m.symbol.clone()
    } else {
        data["instId"]
            .as_str()
            .map(|s| s.replace('-', "/"))
            .unwrap_or_default()
    };

    let id = data["ordId"]
        .as_str()
        .ok_or_else(|| Error::from(ParseError::missing_field("ordId")))?
        .to_string();

    let timestamp = parse_timestamp(data, "cTime").or_else(|| parse_timestamp(data, "ts"));

    let status_str = data["state"].as_str().unwrap_or("live");
    let status = parse_order_status(status_str);

    // Parse order side
    let side = match data["side"].as_str() {
        Some("buy") | Some("Buy") | Some("BUY") => OrderSide::Buy,
        Some("sell") | Some("Sell") | Some("SELL") => OrderSide::Sell,
        _ => return Err(Error::from(ParseError::invalid_format("data", "side"))),
    };

    // Parse order type
    let order_type = match data["ordType"].as_str() {
        Some("market") | Some("Market") | Some("MARKET") => OrderType::Market,
        Some("limit") | Some("Limit") | Some("LIMIT") => OrderType::Limit,
        Some("post_only") => OrderType::LimitMaker,
        Some("fok") => OrderType::Limit, // Fill or Kill
        Some("ioc") => OrderType::Limit, // Immediate or Cancel
        _ => OrderType::Limit,           // Default to limit
    };

    let price = parse_decimal(data, "px");
    let amount =
        parse_decimal(data, "sz").ok_or_else(|| Error::from(ParseError::missing_field("sz")))?;
    let filled = parse_decimal(data, "accFillSz").or_else(|| parse_decimal(data, "fillSz"));
    let remaining = match filled {
        Some(f) => Some(amount - f),
        None => Some(amount),
    };

    let average = parse_decimal(data, "avgPx").or_else(|| parse_decimal(data, "fillPx"));

    // Calculate cost from filled amount and average price
    let cost = match (filled, average) {
        (Some(f), Some(avg)) => Some(f * avg),
        _ => None,
    };

    Ok(Order {
        id,
        client_order_id: data["clOrdId"].as_str().map(|s| s.to_string()),
        timestamp,
        datetime: timestamp.and_then(timestamp_to_datetime),
        last_trade_timestamp: parse_timestamp(data, "uTime"),
        status,
        symbol,
        order_type,
        time_in_force: data["ordType"].as_str().map(|s| match s {
            "fok" => "FOK".to_string(),
            "ioc" => "IOC".to_string(),
            "post_only" => "PO".to_string(),
            _ => "GTC".to_string(),
        }),
        side,
        price,
        average,
        amount,
        filled,
        remaining,
        cost,
        trades: None,
        fee: None,
        post_only: Some(data["ordType"].as_str() == Some("post_only")),
        reduce_only: data["reduceOnly"].as_bool(),
        trigger_price: parse_decimal(data, "triggerPx"),
        stop_price: parse_decimal(data, "slTriggerPx"),
        take_profit_price: parse_decimal(data, "tpTriggerPx"),
        stop_loss_price: parse_decimal(data, "slTriggerPx"),
        trailing_delta: None,
        trailing_percent: None,
        activation_price: None,
        callback_rate: None,
        working_type: None,
        fees: Some(Vec::new()),
        info: value_to_hashmap(data),
    })
}

/// Parse balance data from OKX account info.
///
/// # Arguments
///
/// * `data` - OKX account data JSON object
///
/// # Returns
///
/// Returns a CCXT [`Balance`] structure with all non-zero balances.
pub fn parse_balance(data: &Value) -> Result<Balance> {
    let mut balances = HashMap::new();

    // OKX returns balance in details array
    if let Some(details) = data["details"].as_array() {
        for detail in details {
            parse_balance_entry(detail, &mut balances)?;
        }
    } else if let Some(balances_array) = data.as_array() {
        // Handle array of balance objects
        for balance in balances_array {
            if let Some(details) = balance["details"].as_array() {
                for detail in details {
                    parse_balance_entry(detail, &mut balances)?;
                }
            } else {
                parse_balance_entry(balance, &mut balances)?;
            }
        }
    } else {
        // Handle single balance object
        parse_balance_entry(data, &mut balances)?;
    }

    Ok(Balance {
        balances,
        info: value_to_hashmap(data),
    })
}

/// Parse a single balance entry from OKX response.
fn parse_balance_entry(data: &Value, balances: &mut HashMap<String, BalanceEntry>) -> Result<()> {
    let currency = data["ccy"]
        .as_str()
        .or_else(|| data["currency"].as_str())
        .map(|s| s.to_string());

    if let Some(currency) = currency {
        // OKX uses different field names depending on account type
        let available = parse_decimal(data, "availBal")
            .or_else(|| parse_decimal(data, "availEq"))
            .or_else(|| parse_decimal(data, "cashBal"))
            .unwrap_or(Decimal::ZERO);

        let frozen = parse_decimal(data, "frozenBal")
            .or_else(|| parse_decimal(data, "ordFrozen"))
            .unwrap_or(Decimal::ZERO);

        let total = parse_decimal(data, "eq")
            .or_else(|| parse_decimal(data, "bal"))
            .unwrap_or(available + frozen);

        // Only include non-zero balances
        if total > Decimal::ZERO {
            balances.insert(
                currency,
                BalanceEntry {
                    free: available,
                    used: frozen,
                    total,
                },
            );
        }
    }

    Ok(())
}

// ============================================================================
// Unit Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;
    use serde_json::json;

    #[test]
    fn test_parse_market() {
        let data = json!({
            "instId": "BTC-USDT",
            "instType": "SPOT",
            "baseCcy": "BTC",
            "quoteCcy": "USDT",
            "state": "live",
            "tickSz": "0.01",
            "lotSz": "0.0001",
            "minSz": "0.0001"
        });

        let market = parse_market(&data).unwrap();
        assert_eq!(market.id, "BTC-USDT");
        assert_eq!(market.symbol, "BTC/USDT");
        assert_eq!(market.base, "BTC");
        assert_eq!(market.quote, "USDT");
        assert!(market.active);
        assert_eq!(market.market_type, MarketType::Spot);
    }

    #[test]
    fn test_parse_ticker() {
        let data = json!({
            "instId": "BTC-USDT",
            "last": "50000.00",
            "high24h": "51000.00",
            "low24h": "49000.00",
            "bidPx": "49999.00",
            "askPx": "50001.00",
            "vol24h": "1000.5",
            "ts": "1700000000000"
        });

        let ticker = parse_ticker(&data, None).unwrap();
        assert_eq!(ticker.symbol, "BTC/USDT");
        assert_eq!(ticker.last, Some(Price::new(dec!(50000.00))));
        assert_eq!(ticker.high, Some(Price::new(dec!(51000.00))));
        assert_eq!(ticker.low, Some(Price::new(dec!(49000.00))));
        assert_eq!(ticker.timestamp, 1700000000000);
    }

    #[test]
    fn test_parse_orderbook() {
        let data = json!({
            "bids": [
                ["50000.00", "1.5", "0", "1"],
                ["49999.00", "2.0", "0", "2"]
            ],
            "asks": [
                ["50001.00", "1.0", "0", "1"],
                ["50002.00", "3.0", "0", "2"]
            ],
            "ts": "1700000000000"
        });

        let orderbook = parse_orderbook(&data, "BTC/USDT".to_string()).unwrap();
        assert_eq!(orderbook.symbol, "BTC/USDT");
        assert_eq!(orderbook.bids.len(), 2);
        assert_eq!(orderbook.asks.len(), 2);
        assert_eq!(orderbook.bids[0].price, Price::new(dec!(50000.00)));
        assert_eq!(orderbook.asks[0].price, Price::new(dec!(50001.00)));
    }

    #[test]
    fn test_parse_trade() {
        let data = json!({
            "tradeId": "123456",
            "instId": "BTC-USDT",
            "side": "buy",
            "px": "50000.00",
            "sz": "0.5",
            "ts": "1700000000000"
        });

        let trade = parse_trade(&data, None).unwrap();
        assert_eq!(trade.id, Some("123456".to_string()));
        assert_eq!(trade.side, OrderSide::Buy);
        assert_eq!(trade.price, Price::new(dec!(50000.00)));
        assert_eq!(trade.amount, Amount::new(dec!(0.5)));
    }

    #[test]
    fn test_parse_ohlcv() {
        let data = json!([
            "1700000000000",
            "50000.00",
            "51000.00",
            "49000.00",
            "50500.00",
            "1000.5"
        ]);

        let ohlcv = parse_ohlcv(&data).unwrap();
        assert_eq!(ohlcv.timestamp, 1700000000000);
        assert_eq!(ohlcv.open, 50000.00);
        assert_eq!(ohlcv.high, 51000.00);
        assert_eq!(ohlcv.low, 49000.00);
        assert_eq!(ohlcv.close, 50500.00);
        assert_eq!(ohlcv.volume, 1000.5);
    }

    #[test]
    fn test_parse_order_status() {
        assert_eq!(parse_order_status("live"), OrderStatus::Open);
        assert_eq!(parse_order_status("partially_filled"), OrderStatus::Open);
        assert_eq!(parse_order_status("filled"), OrderStatus::Closed);
        assert_eq!(parse_order_status("canceled"), OrderStatus::Cancelled);
        assert_eq!(parse_order_status("mmp_canceled"), OrderStatus::Cancelled);
        assert_eq!(parse_order_status("expired"), OrderStatus::Expired);
        assert_eq!(parse_order_status("rejected"), OrderStatus::Rejected);
    }

    #[test]
    fn test_parse_order() {
        let data = json!({
            "ordId": "123456789",
            "instId": "BTC-USDT",
            "side": "buy",
            "ordType": "limit",
            "px": "50000.00",
            "sz": "0.5",
            "state": "live",
            "cTime": "1700000000000"
        });

        let order = parse_order(&data, None).unwrap();
        assert_eq!(order.id, "123456789");
        assert_eq!(order.side, OrderSide::Buy);
        assert_eq!(order.order_type, OrderType::Limit);
        assert_eq!(order.price, Some(dec!(50000.00)));
        assert_eq!(order.amount, dec!(0.5));
        assert_eq!(order.status, OrderStatus::Open);
    }

    #[test]
    fn test_parse_balance() {
        let data = json!({
            "details": [
                {
                    "ccy": "BTC",
                    "availBal": "1.5",
                    "frozenBal": "0.5",
                    "eq": "2.0"
                },
                {
                    "ccy": "USDT",
                    "availBal": "10000.00",
                    "frozenBal": "0",
                    "eq": "10000.00"
                }
            ]
        });

        let balance = parse_balance(&data).unwrap();
        let btc = balance.get("BTC").unwrap();
        assert_eq!(btc.free, dec!(1.5));
        assert_eq!(btc.used, dec!(0.5));
        assert_eq!(btc.total, dec!(2.0));

        let usdt = balance.get("USDT").unwrap();
        assert_eq!(usdt.free, dec!(10000.00));
        assert_eq!(usdt.total, dec!(10000.00));
    }

    #[test]
    fn test_timestamp_to_datetime() {
        let ts = 1700000000000i64;
        let dt = timestamp_to_datetime(ts).unwrap();
        assert!(dt.contains("2023-11-14"));
    }
}
