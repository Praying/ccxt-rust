//! Bitget data parser module.
//!
//! Converts Bitget API response data into standardized CCXT format structures.

use ccxt_core::{
    Result,
    error::{Error, ParseError},
    parser_utils::{parse_decimal, parse_timestamp, value_to_hashmap},
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

// Re-export for backward compatibility
pub use ccxt_core::parser_utils::{datetime_to_timestamp, timestamp_to_datetime};

// ============================================================================
// Market Data Parser Functions
// ============================================================================

/// Parse market data from Bitget exchange info.
///
/// # Arguments
///
/// * `data` - Bitget market data JSON object
///
/// # Returns
///
/// Returns a CCXT [`Market`] structure.
///
/// # Errors
///
/// Returns an error if required fields are missing or invalid.
pub fn parse_market(data: &Value) -> Result<Market> {
    // Bitget uses "symbol" for the exchange-specific ID (e.g., "BTCUSDT")
    let id = data["symbol"]
        .as_str()
        .ok_or_else(|| Error::from(ParseError::missing_field("symbol")))?
        .to_string();

    // Base and quote currencies
    let base = data["baseCoin"]
        .as_str()
        .ok_or_else(|| Error::from(ParseError::missing_field("baseCoin")))?
        .to_string();

    let quote = data["quoteCoin"]
        .as_str()
        .ok_or_else(|| Error::from(ParseError::missing_field("quoteCoin")))?
        .to_string();

    // Unified symbol format (e.g., "BTC/USDT")
    let symbol = format!("{}/{}", base, quote);

    // Market status
    let status = data["status"].as_str().unwrap_or("online");
    let active = status == "online";

    // Parse precision
    let price_precision = parse_decimal(data, "pricePrecision")
        .or_else(|| parse_decimal(data, "pricePlace"))
        .map(|p| {
            // Convert decimal places to tick size (e.g., 2 -> 0.01)
            if p.is_integer() {
                let places = p.to_string().parse::<i32>().unwrap_or(0);
                Decimal::new(1, places as u32)
            } else {
                p
            }
        });

    let amount_precision = parse_decimal(data, "quantityPrecision")
        .or_else(|| parse_decimal(data, "volumePlace"))
        .map(|p| {
            if p.is_integer() {
                let places = p.to_string().parse::<i32>().unwrap_or(0);
                Decimal::new(1, places as u32)
            } else {
                p
            }
        });

    // Parse limits
    let min_amount =
        parse_decimal(data, "minTradeNum").or_else(|| parse_decimal(data, "minTradeAmount"));
    let max_amount =
        parse_decimal(data, "maxTradeNum").or_else(|| parse_decimal(data, "maxTradeAmount"));
    let min_cost = parse_decimal(data, "minTradeUSDT");

    // Parse fees
    let maker_fee = parse_decimal(data, "makerFeeRate");
    let taker_fee = parse_decimal(data, "takerFeeRate");

    // Parse the symbol to get structured representation
    let parsed_symbol = ccxt_core::symbol::SymbolParser::parse(&symbol).ok();

    Ok(Market {
        id,
        symbol,
        parsed_symbol,
        base: base.clone(),
        quote: quote.clone(),
        settle: None,
        base_id: Some(base),
        quote_id: Some(quote),
        settle_id: None,
        market_type: MarketType::Spot,
        active,
        margin: false,
        contract: Some(false),
        linear: None,
        inverse: None,
        contract_size: None,
        expiry: None,
        expiry_datetime: None,
        strike: None,
        option_type: None,
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
            cost: Some(MinMax {
                min: min_cost,
                max: None,
            }),
            leverage: None,
        },
        maker: maker_fee,
        taker: taker_fee,
        percentage: Some(true),
        tier_based: Some(false),
        fee_side: Some("quote".to_string()),
        info: value_to_hashmap(data),
    })
}

/// Parse ticker data from Bitget ticker response.
///
/// # Arguments
///
/// * `data` - Bitget ticker data JSON object
/// * `market` - Optional market information for symbol resolution
///
/// # Returns
///
/// Returns a CCXT [`Ticker`] structure.
pub fn parse_ticker(data: &Value, market: Option<&Market>) -> Result<Ticker> {
    let symbol = if let Some(m) = market {
        m.symbol.clone()
    } else {
        // Try to construct symbol from instId or symbol field
        data["symbol"]
            .as_str()
            .or_else(|| data["instId"].as_str())
            .map(ToString::to_string)
            .unwrap_or_default()
    };

    // Bitget uses "ts" for timestamp
    let timestamp = parse_timestamp(data, "ts")
        .or_else(|| parse_timestamp(data, "timestamp"))
        .unwrap_or(0);

    Ok(Ticker {
        symbol,
        timestamp,
        datetime: timestamp_to_datetime(timestamp),
        high: parse_decimal(data, "high24h")
            .or_else(|| parse_decimal(data, "high"))
            .map(Price::new),
        low: parse_decimal(data, "low24h")
            .or_else(|| parse_decimal(data, "low"))
            .map(Price::new),
        bid: parse_decimal(data, "bidPr")
            .or_else(|| parse_decimal(data, "bestBid"))
            .map(Price::new),
        bid_volume: parse_decimal(data, "bidSz")
            .or_else(|| parse_decimal(data, "bestBidSize"))
            .map(Amount::new),
        ask: parse_decimal(data, "askPr")
            .or_else(|| parse_decimal(data, "bestAsk"))
            .map(Price::new),
        ask_volume: parse_decimal(data, "askSz")
            .or_else(|| parse_decimal(data, "bestAskSize"))
            .map(Amount::new),
        vwap: None,
        open: parse_decimal(data, "open24h")
            .or_else(|| parse_decimal(data, "open"))
            .map(Price::new),
        close: parse_decimal(data, "lastPr")
            .or_else(|| parse_decimal(data, "last"))
            .or_else(|| parse_decimal(data, "close"))
            .map(Price::new),
        last: parse_decimal(data, "lastPr")
            .or_else(|| parse_decimal(data, "last"))
            .map(Price::new),
        previous_close: None,
        change: parse_decimal(data, "change24h")
            .or_else(|| parse_decimal(data, "change"))
            .map(Price::new),
        percentage: parse_decimal(data, "changeUtc24h")
            .or_else(|| parse_decimal(data, "changePercentage")),
        average: None,
        base_volume: parse_decimal(data, "baseVolume")
            .or_else(|| parse_decimal(data, "vol24h"))
            .map(Amount::new),
        quote_volume: parse_decimal(data, "quoteVolume")
            .or_else(|| parse_decimal(data, "usdtVolume"))
            .map(Amount::new),
        info: value_to_hashmap(data),
    })
}

/// Parse orderbook data from Bitget depth response.
///
/// # Arguments
///
/// * `data` - Bitget orderbook data JSON object
/// * `symbol` - Trading pair symbol
///
/// # Returns
///
/// Returns a CCXT [`OrderBook`] structure with bids sorted in descending order
/// and asks sorted in ascending order.
pub fn parse_orderbook(data: &Value, symbol: String) -> Result<OrderBook> {
    let timestamp = parse_timestamp(data, "ts")
        .or_else(|| parse_timestamp(data, "timestamp"))
        .unwrap_or_else(|| chrono::Utc::now().timestamp_millis());

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
        nonce: parse_timestamp(data, "seqId"),
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

/// Parse trade data from Bitget trade response.
///
/// # Arguments
///
/// * `data` - Bitget trade data JSON object
/// * `market` - Optional market information for symbol resolution
///
/// # Returns
///
/// Returns a CCXT [`Trade`] structure.
pub fn parse_trade(data: &Value, market: Option<&Market>) -> Result<Trade> {
    let symbol = if let Some(m) = market {
        m.symbol.clone()
    } else {
        data["symbol"]
            .as_str()
            .map(ToString::to_string)
            .unwrap_or_default()
    };

    let id = data["tradeId"]
        .as_str()
        .or_else(|| data["id"].as_str())
        .map(ToString::to_string);

    let timestamp = parse_timestamp(data, "ts")
        .or_else(|| parse_timestamp(data, "timestamp"))
        .unwrap_or(0);

    // Bitget uses "side" field with "buy" or "sell" values
    let side = match data["side"].as_str() {
        Some("sell" | "Sell" | "SELL") => OrderSide::Sell,
        _ => OrderSide::Buy, // Default to buy if not specified
    };

    let price = parse_decimal(data, "price").or_else(|| parse_decimal(data, "fillPrice"));
    let amount = parse_decimal(data, "size")
        .or_else(|| parse_decimal(data, "amount"))
        .or_else(|| parse_decimal(data, "fillSize"));

    let cost = match (price, amount) {
        (Some(p), Some(a)) => Some(p * a),
        _ => None,
    };

    Ok(Trade {
        id,
        order: data["orderId"].as_str().map(ToString::to_string),
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

/// Parse OHLCV (candlestick) data from Bitget kline response.
///
/// # Arguments
///
/// * `data` - Bitget OHLCV data JSON array
///
/// # Returns
///
/// Returns a CCXT [`OHLCV`] structure.
pub fn parse_ohlcv(data: &Value) -> Result<OHLCV> {
    // Bitget returns OHLCV as array: [timestamp, open, high, low, close, volume, quoteVolume]
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

/// Map Bitget order status to CCXT OrderStatus.
///
/// # Arguments
///
/// * `status` - Bitget order status string
///
/// # Returns
///
/// Returns the corresponding CCXT [`OrderStatus`].
pub fn parse_order_status(status: &str) -> OrderStatus {
    match status.to_lowercase().as_str() {
        "filled" | "full_fill" | "full-fill" => OrderStatus::Closed,
        "cancelled" | "canceled" | "cancel" => OrderStatus::Cancelled,
        "expired" | "expire" => OrderStatus::Expired,
        "rejected" | "reject" => OrderStatus::Rejected,
        _ => OrderStatus::Open, // Default to Open for unknown statuses
    }
}

/// Parse order data from Bitget order response.
///
/// # Arguments
///
/// * `data` - Bitget order data JSON object
/// * `market` - Optional market information for symbol resolution
///
/// # Returns
///
/// Returns a CCXT [`Order`] structure.
pub fn parse_order(data: &Value, market: Option<&Market>) -> Result<Order> {
    let symbol = if let Some(m) = market {
        m.symbol.clone()
    } else {
        data["symbol"]
            .as_str()
            .or_else(|| data["instId"].as_str())
            .map(ToString::to_string)
            .unwrap_or_default()
    };

    let id = data["orderId"]
        .as_str()
        .ok_or_else(|| Error::from(ParseError::missing_field("orderId")))?
        .to_string();

    let timestamp = parse_timestamp(data, "cTime")
        .or_else(|| parse_timestamp(data, "createTime"))
        .or_else(|| parse_timestamp(data, "ts"));

    let status_str = data["status"]
        .as_str()
        .or_else(|| data["state"].as_str())
        .unwrap_or("live");
    let status = parse_order_status(status_str);

    // Parse order side
    let side = match data["side"].as_str() {
        Some("buy" | "Buy" | "BUY") => OrderSide::Buy,
        Some("sell" | "Sell" | "SELL") => OrderSide::Sell,
        _ => return Err(Error::from(ParseError::invalid_format("data", "side"))),
    };

    // Parse order type
    let order_type = match data["orderType"].as_str().or_else(|| data["type"].as_str()) {
        Some("market" | "Market" | "MARKET") => OrderType::Market,
        Some("limit_maker" | "post_only") => OrderType::LimitMaker,
        _ => OrderType::Limit, // Default to limit
    };

    let price = parse_decimal(data, "price").or_else(|| parse_decimal(data, "priceAvg"));
    let amount = parse_decimal(data, "size")
        .or_else(|| parse_decimal(data, "baseVolume"))
        .ok_or_else(|| Error::from(ParseError::missing_field("size")))?;
    let filled = parse_decimal(data, "fillSize").or_else(|| parse_decimal(data, "baseVolume"));
    let remaining = match filled {
        Some(f) => Some(amount - f),
        None => Some(amount),
    };

    let cost =
        parse_decimal(data, "fillNotionalUsd").or_else(|| parse_decimal(data, "quoteVolume"));

    let average = parse_decimal(data, "priceAvg").or_else(|| parse_decimal(data, "fillPrice"));

    Ok(Order {
        id,
        client_order_id: data["clientOid"]
            .as_str()
            .or_else(|| data["clientOrderId"].as_str())
            .map(ToString::to_string),
        timestamp,
        datetime: timestamp.and_then(timestamp_to_datetime),
        last_trade_timestamp: parse_timestamp(data, "uTime")
            .or_else(|| parse_timestamp(data, "updateTime")),
        status,
        symbol,
        order_type,
        time_in_force: data["timeInForce"]
            .as_str()
            .or_else(|| data["force"].as_str())
            .map(str::to_uppercase),
        side,
        price,
        average,
        amount,
        filled,
        remaining,
        cost,
        trades: None,
        fee: None,
        post_only: None,
        reduce_only: data["reduceOnly"].as_bool(),
        trigger_price: parse_decimal(data, "triggerPrice"),
        stop_price: parse_decimal(data, "stopPrice")
            .or_else(|| parse_decimal(data, "presetStopLossPrice")),
        take_profit_price: parse_decimal(data, "presetTakeProfitPrice"),
        stop_loss_price: parse_decimal(data, "presetStopLossPrice"),
        trailing_delta: None,
        trailing_percent: None,
        activation_price: None,
        callback_rate: None,
        working_type: None,
        fees: Some(Vec::new()),
        info: value_to_hashmap(data),
    })
}

/// Parse balance data from Bitget account info.
///
/// # Arguments
///
/// * `data` - Bitget account data JSON object
///
/// # Returns
///
/// Returns a CCXT [`Balance`] structure with all non-zero balances.
pub fn parse_balance(data: &Value) -> Result<Balance> {
    let mut balances = HashMap::new();

    // Handle array of balances (common Bitget response format)
    if let Some(balances_array) = data.as_array() {
        for balance in balances_array {
            parse_balance_entry(balance, &mut balances);
        }
    } else if let Some(balances_array) = data["data"].as_array() {
        // Handle wrapped response
        for balance in balances_array {
            parse_balance_entry(balance, &mut balances);
        }
    } else {
        // Handle single balance object
        parse_balance_entry(data, &mut balances);
    }

    Ok(Balance {
        balances,
        info: value_to_hashmap(data),
    })
}

/// Parse a single balance entry from Bitget response.
fn parse_balance_entry(data: &Value, balances: &mut HashMap<String, BalanceEntry>) {
    let currency = data["coin"]
        .as_str()
        .or_else(|| data["coinName"].as_str())
        .or_else(|| data["asset"].as_str())
        .map(ToString::to_string);

    if let Some(currency) = currency {
        let available = parse_decimal(data, "available")
            .or_else(|| parse_decimal(data, "free"))
            .unwrap_or(Decimal::ZERO);

        let frozen = parse_decimal(data, "frozen")
            .or_else(|| parse_decimal(data, "locked"))
            .or_else(|| parse_decimal(data, "lock"))
            .unwrap_or(Decimal::ZERO);

        let total = available + frozen;

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
            "symbol": "BTCUSDT",
            "baseCoin": "BTC",
            "quoteCoin": "USDT",
            "status": "online",
            "pricePrecision": "2",
            "quantityPrecision": "4",
            "minTradeNum": "0.0001",
            "makerFeeRate": "0.001",
            "takerFeeRate": "0.001"
        });

        let market = parse_market(&data).unwrap();
        assert_eq!(market.id, "BTCUSDT");
        assert_eq!(market.symbol, "BTC/USDT");
        assert_eq!(market.base, "BTC");
        assert_eq!(market.quote, "USDT");
        assert!(market.active);
    }

    #[test]
    fn test_parse_ticker() {
        let data = json!({
            "symbol": "BTCUSDT",
            "lastPr": "50000.00",
            "high24h": "51000.00",
            "low24h": "49000.00",
            "bidPr": "49999.00",
            "askPr": "50001.00",
            "baseVolume": "1000.5",
            "ts": "1700000000000"
        });

        let ticker = parse_ticker(&data, None).unwrap();
        assert_eq!(ticker.symbol, "BTCUSDT");
        assert_eq!(ticker.last, Some(Price::new(dec!(50000.00))));
        assert_eq!(ticker.high, Some(Price::new(dec!(51000.00))));
        assert_eq!(ticker.low, Some(Price::new(dec!(49000.00))));
        assert_eq!(ticker.timestamp, 1700000000000);
    }

    #[test]
    fn test_parse_orderbook() {
        let data = json!({
            "bids": [
                ["50000.00", "1.5"],
                ["49999.00", "2.0"]
            ],
            "asks": [
                ["50001.00", "1.0"],
                ["50002.00", "3.0"]
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
            "symbol": "BTCUSDT",
            "side": "buy",
            "price": "50000.00",
            "size": "0.5",
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
        assert_eq!(parse_order_status("cancelled"), OrderStatus::Cancelled);
        assert_eq!(parse_order_status("expired"), OrderStatus::Expired);
        assert_eq!(parse_order_status("rejected"), OrderStatus::Rejected);
    }

    #[test]
    fn test_parse_order() {
        let data = json!({
            "orderId": "123456789",
            "symbol": "BTCUSDT",
            "side": "buy",
            "orderType": "limit",
            "price": "50000.00",
            "size": "0.5",
            "status": "live",
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
        let data = json!([
            {
                "coin": "BTC",
                "available": "1.5",
                "frozen": "0.5"
            },
            {
                "coin": "USDT",
                "available": "10000.00",
                "frozen": "0"
            }
        ]);

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
