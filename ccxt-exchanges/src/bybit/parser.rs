//! Bybit data parser module.
//!
//! Converts Bybit API response data into standardized CCXT format structures.

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

/// Parse market data from Bybit exchange info.
///
/// Bybit uses `symbol` for instrument ID (e.g., "BTCUSDT").
///
/// # Arguments
///
/// * `data` - Bybit market data JSON object
///
/// # Returns
///
/// Returns a CCXT [`Market`] structure.
pub fn parse_market(data: &Value) -> Result<Market> {
    // Bybit uses "symbol" for the exchange-specific ID (e.g., "BTCUSDT")
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

    // Contract type - Bybit uses "contractType" for derivatives
    let contract_type = data["contractType"].as_str();
    let market_type = match contract_type {
        Some("LinearPerpetual" | "InversePerpetual") => MarketType::Swap,
        Some("LinearFutures" | "InverseFutures") => MarketType::Futures,
        _ => MarketType::Spot,
    };

    // Market status
    let status = data["status"].as_str().unwrap_or("Trading");
    let active = status == "Trading";

    // Parse precision - Bybit uses tickSize and basePrecision
    let price_precision = parse_decimal(data, "priceFilter").or_else(|| {
        data.get("priceFilter")
            .and_then(|pf| parse_decimal(pf, "tickSize"))
    });
    let amount_precision = parse_decimal(data, "lotSizeFilter").or_else(|| {
        data.get("lotSizeFilter")
            .and_then(|lf| parse_decimal(lf, "basePrecision"))
    });

    // Parse limits from lotSizeFilter
    let (min_amount, max_amount) = if let Some(lot_filter) = data.get("lotSizeFilter") {
        (
            parse_decimal(lot_filter, "minOrderQty"),
            parse_decimal(lot_filter, "maxOrderQty"),
        )
    } else {
        (None, None)
    };

    // Contract-specific fields
    let contract = market_type != MarketType::Spot;
    let linear = if contract {
        Some(contract_type == Some("LinearPerpetual") || contract_type == Some("LinearFutures"))
    } else {
        None
    };
    let inverse = if contract {
        Some(contract_type == Some("InversePerpetual") || contract_type == Some("InverseFutures"))
    } else {
        None
    };
    let contract_size = parse_decimal(data, "contractSize");

    // Settlement currency for derivatives
    let settle = data["settleCoin"].as_str().map(ToString::to_string);
    let settle_id = settle.clone();

    // Expiry for futures
    let expiry = parse_timestamp(data, "deliveryTime");
    let expiry_datetime = expiry.and_then(timestamp_to_datetime);

    // Build unified symbol format based on market type:
    // - Spot: BASE/QUOTE (e.g., "BTC/USDT")
    // - Swap: BASE/QUOTE:SETTLE (e.g., "BTC/USDT:USDT")
    // - Futures: BASE/QUOTE:SETTLE-YYMMDD (e.g., "BTC/USDT:USDT-241231")
    let symbol = match market_type {
        MarketType::Swap => {
            if let Some(ref s) = settle {
                format!("{}/{}:{}", base, quote, s)
            } else if linear == Some(true) {
                // Linear swaps settle in quote currency
                format!("{}/{}:{}", base, quote, quote)
            } else {
                // Inverse swaps settle in base currency
                format!("{}/{}:{}", base, quote, base)
            }
        }
        MarketType::Futures => {
            let settle_ccy = settle.clone().unwrap_or_else(|| {
                if linear == Some(true) {
                    quote.clone()
                } else {
                    base.clone()
                }
            });
            if let Some(exp_ts) = expiry {
                // Convert timestamp to YYMMDD format
                if let Some(dt) = chrono::DateTime::from_timestamp_millis(exp_ts) {
                    let year = (dt.format("%y").to_string().parse::<u8>()).unwrap_or(0);
                    let month = (dt.format("%m").to_string().parse::<u8>()).unwrap_or(1);
                    let day = (dt.format("%d").to_string().parse::<u8>()).unwrap_or(1);
                    format!(
                        "{}/{}:{}-{:02}{:02}{:02}",
                        base, quote, settle_ccy, year, month, day
                    )
                } else {
                    format!("{}/{}:{}", base, quote, settle_ccy)
                }
            } else {
                format!("{}/{}:{}", base, quote, settle_ccy)
            }
        }
        _ => format!("{}/{}", base, quote), // Spot and Option
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
        margin: contract,
        contract: Some(contract),
        linear,
        inverse,
        contract_size,
        expiry,
        expiry_datetime,
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
            cost: None,
            leverage: None,
        },
        maker: parse_decimal(data, "makerFeeRate"),
        taker: parse_decimal(data, "takerFeeRate"),
        percentage: Some(true),
        tier_based: Some(false),
        fee_side: Some("quote".to_string()),
        info: value_to_hashmap(data),
    })
}

/// Parse ticker data from Bybit ticker response.
///
/// # Arguments
///
/// * `data` - Bybit ticker data JSON object
/// * `market` - Optional market information for symbol resolution
///
/// # Returns
///
/// Returns a CCXT [`Ticker`] structure.
pub fn parse_ticker(data: &Value, market: Option<&Market>) -> Result<Ticker> {
    let symbol = if let Some(m) = market {
        m.symbol.clone()
    } else {
        // Try to construct symbol from symbol field
        // Bybit uses concatenated format like "BTCUSDT"
        data["symbol"]
            .as_str()
            .map(ToString::to_string)
            .unwrap_or_default()
    };

    // Bybit uses different timestamp fields
    let timestamp = parse_timestamp(data, "time")
        .or_else(|| parse_timestamp(data, "timestamp"))
        .unwrap_or(0);

    Ok(Ticker {
        symbol,
        timestamp,
        datetime: timestamp_to_datetime(timestamp),
        high: parse_decimal(data, "highPrice24h").map(Price::new),
        low: parse_decimal(data, "lowPrice24h").map(Price::new),
        bid: parse_decimal(data, "bid1Price").map(Price::new),
        bid_volume: parse_decimal(data, "bid1Size").map(Amount::new),
        ask: parse_decimal(data, "ask1Price").map(Price::new),
        ask_volume: parse_decimal(data, "ask1Size").map(Amount::new),
        vwap: None,
        open: parse_decimal(data, "prevPrice24h").map(Price::new),
        close: parse_decimal(data, "lastPrice").map(Price::new),
        last: parse_decimal(data, "lastPrice").map(Price::new),
        previous_close: parse_decimal(data, "prevPrice24h").map(Price::new),
        change: parse_decimal(data, "price24hPcnt")
            .and_then(|pct| parse_decimal(data, "prevPrice24h").map(|prev| Price::new(prev * pct))),
        percentage: parse_decimal(data, "price24hPcnt").map(|p| p * Decimal::from(100)),
        average: None,
        base_volume: parse_decimal(data, "volume24h").map(Amount::new),
        quote_volume: parse_decimal(data, "turnover24h").map(Amount::new),
        info: value_to_hashmap(data),
    })
}

/// Parse orderbook data from Bybit depth response.
///
/// # Arguments
///
/// * `data` - Bybit orderbook data JSON object
/// * `symbol` - Trading pair symbol
///
/// # Returns
///
/// Returns a CCXT [`OrderBook`] structure with bids sorted in descending order
/// and asks sorted in ascending order.
pub fn parse_orderbook(data: &Value, symbol: String) -> Result<OrderBook> {
    let timestamp = parse_timestamp(data, "ts")
        .or_else(|| parse_timestamp(data, "time"))
        .unwrap_or_else(|| chrono::Utc::now().timestamp_millis());

    let mut bids = parse_orderbook_side(&data["b"])?;
    let mut asks = parse_orderbook_side(&data["a"])?;

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
            // Bybit format: [price, size]
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

/// Parse trade data from Bybit trade response.
///
/// # Arguments
///
/// * `data` - Bybit trade data JSON object
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

    let id = data["execId"]
        .as_str()
        .or_else(|| data["id"].as_str())
        .map(ToString::to_string);

    let timestamp = parse_timestamp(data, "time")
        .or_else(|| parse_timestamp(data, "T"))
        .unwrap_or(0);

    // Bybit uses "side" field with "Buy" or "Sell" values
    let side = match data["side"].as_str() {
        Some("Sell" | "sell" | "SELL") => OrderSide::Sell,
        _ => OrderSide::Buy, // Default to buy if not specified
    };

    let price = parse_decimal(data, "price").or_else(|| parse_decimal(data, "execPrice"));
    let amount = parse_decimal(data, "size")
        .or_else(|| parse_decimal(data, "execQty"))
        .or_else(|| parse_decimal(data, "qty"));

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

/// Parse OHLCV (candlestick) data from Bybit kline response.
///
/// # Arguments
///
/// * `data` - Bybit OHLCV data JSON array or object
///
/// # Returns
///
/// Returns a CCXT [`OHLCV`] structure.
pub fn parse_ohlcv(data: &Value) -> Result<OHLCV> {
    // Bybit returns OHLCV as array: [startTime, openPrice, highPrice, lowPrice, closePrice, volume, turnover]
    // or as object with named fields
    if let Some(arr) = data.as_array() {
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
    } else {
        // Handle object format
        let timestamp = parse_timestamp(data, "startTime")
            .or_else(|| parse_timestamp(data, "openTime"))
            .ok_or_else(|| Error::from(ParseError::missing_field("startTime")))?;

        let open = data["openPrice"]
            .as_str()
            .and_then(|s| s.parse::<f64>().ok())
            .or_else(|| data["openPrice"].as_f64())
            .or_else(|| data["open"].as_str().and_then(|s| s.parse::<f64>().ok()))
            .or_else(|| data["open"].as_f64())
            .ok_or_else(|| Error::from(ParseError::missing_field("openPrice")))?;

        let high = data["highPrice"]
            .as_str()
            .and_then(|s| s.parse::<f64>().ok())
            .or_else(|| data["highPrice"].as_f64())
            .or_else(|| data["high"].as_str().and_then(|s| s.parse::<f64>().ok()))
            .or_else(|| data["high"].as_f64())
            .ok_or_else(|| Error::from(ParseError::missing_field("highPrice")))?;

        let low = data["lowPrice"]
            .as_str()
            .and_then(|s| s.parse::<f64>().ok())
            .or_else(|| data["lowPrice"].as_f64())
            .or_else(|| data["low"].as_str().and_then(|s| s.parse::<f64>().ok()))
            .or_else(|| data["low"].as_f64())
            .ok_or_else(|| Error::from(ParseError::missing_field("lowPrice")))?;

        let close = data["closePrice"]
            .as_str()
            .and_then(|s| s.parse::<f64>().ok())
            .or_else(|| data["closePrice"].as_f64())
            .or_else(|| data["close"].as_str().and_then(|s| s.parse::<f64>().ok()))
            .or_else(|| data["close"].as_f64())
            .ok_or_else(|| Error::from(ParseError::missing_field("closePrice")))?;

        let volume = data["volume"]
            .as_str()
            .and_then(|s| s.parse::<f64>().ok())
            .or_else(|| data["volume"].as_f64())
            .ok_or_else(|| Error::from(ParseError::missing_field("volume")))?;

        Ok(OHLCV {
            timestamp,
            open,
            high,
            low,
            close,
            volume,
        })
    }
}

// ============================================================================
// Order and Balance Parser Functions
// ============================================================================

/// Map Bybit order status to CCXT OrderStatus.
///
/// Bybit order states:
/// - New: Order is active
/// - PartiallyFilled: Order is partially filled
/// - Filled: Order is completely filled
/// - Cancelled: Order is canceled
/// - Rejected: Order is rejected
/// - PartiallyFilledCanceled: Partially filled then canceled
///
/// # Arguments
///
/// * `status` - Bybit order status string
///
/// # Returns
///
/// Returns the corresponding CCXT [`OrderStatus`].
pub fn parse_order_status(status: &str) -> OrderStatus {
    match status {
        "Filled" => OrderStatus::Closed,
        "Cancelled" | "Canceled" | "PartiallyFilledCanceled" | "Deactivated" => {
            OrderStatus::Cancelled
        }
        "Rejected" => OrderStatus::Rejected,
        "Expired" => OrderStatus::Expired,
        _ => OrderStatus::Open, // Default to Open for unknown statuses
    }
}

/// Parse order data from Bybit order response.
///
/// # Arguments
///
/// * `data` - Bybit order data JSON object
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
            .map(ToString::to_string)
            .unwrap_or_default()
    };

    let id = data["orderId"]
        .as_str()
        .ok_or_else(|| Error::from(ParseError::missing_field("orderId")))?
        .to_string();

    let timestamp =
        parse_timestamp(data, "createdTime").or_else(|| parse_timestamp(data, "createTime"));

    let status_str = data["orderStatus"].as_str().unwrap_or("New");
    let status = parse_order_status(status_str);

    // Parse order side
    let side = match data["side"].as_str() {
        Some("Buy" | "buy" | "BUY") => OrderSide::Buy,
        Some("Sell" | "sell" | "SELL") => OrderSide::Sell,
        _ => return Err(Error::from(ParseError::invalid_format("data", "side"))),
    };

    // Parse order type
    let order_type = match data["orderType"].as_str() {
        Some("Market" | "MARKET") => OrderType::Market,
        _ => OrderType::Limit, // Default to limit
    };

    let price = parse_decimal(data, "price");
    let amount =
        parse_decimal(data, "qty").ok_or_else(|| Error::from(ParseError::missing_field("qty")))?;
    let filled = parse_decimal(data, "cumExecQty");
    let remaining = match filled {
        Some(f) => Some(amount - f),
        None => Some(amount),
    };

    let average = parse_decimal(data, "avgPrice");

    // Calculate cost from filled amount and average price
    let cost = parse_decimal(data, "cumExecValue").or_else(|| match (filled, average) {
        (Some(f), Some(avg)) => Some(f * avg),
        _ => None,
    });

    Ok(Order {
        id,
        client_order_id: data["orderLinkId"].as_str().map(ToString::to_string),
        timestamp,
        datetime: timestamp.and_then(timestamp_to_datetime),
        last_trade_timestamp: parse_timestamp(data, "updatedTime"),
        status,
        symbol,
        order_type,
        time_in_force: data["timeInForce"].as_str().map(ToString::to_string),
        side,
        price,
        average,
        amount,
        filled,
        remaining,
        cost,
        trades: None,
        fee: None,
        post_only: data["timeInForce"].as_str().map(|s| s == "PostOnly"),
        reduce_only: data["reduceOnly"].as_bool(),
        trigger_price: parse_decimal(data, "triggerPrice"),
        stop_price: parse_decimal(data, "stopLoss"),
        take_profit_price: parse_decimal(data, "takeProfit"),
        stop_loss_price: parse_decimal(data, "stopLoss"),
        trailing_delta: None,
        trailing_percent: None,
        activation_price: None,
        callback_rate: None,
        working_type: None,
        fees: Some(Vec::new()),
        info: value_to_hashmap(data),
    })
}

/// Parse balance data from Bybit account info.
///
/// # Arguments
///
/// * `data` - Bybit account data JSON object
///
/// # Returns
///
/// Returns a CCXT [`Balance`] structure with all non-zero balances.
pub fn parse_balance(data: &Value) -> Result<Balance> {
    let mut balances = HashMap::new();

    // Bybit returns balance in coin array
    if let Some(coins) = data["coin"].as_array() {
        for coin in coins {
            parse_balance_entry(coin, &mut balances);
        }
    } else if let Some(list) = data["list"].as_array() {
        // Handle list format from wallet balance endpoint
        for item in list {
            if let Some(coins) = item["coin"].as_array() {
                for coin in coins {
                    parse_balance_entry(coin, &mut balances);
                }
            }
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

/// Parse a single balance entry from Bybit response.
fn parse_balance_entry(data: &Value, balances: &mut HashMap<String, BalanceEntry>) {
    let currency = data["coin"]
        .as_str()
        .or_else(|| data["currency"].as_str())
        .map(ToString::to_string);

    if let Some(currency) = currency {
        // Bybit uses different field names depending on account type
        let available = parse_decimal(data, "availableToWithdraw")
            .or_else(|| parse_decimal(data, "free"))
            .or_else(|| parse_decimal(data, "walletBalance"))
            .unwrap_or(Decimal::ZERO);

        let frozen = parse_decimal(data, "locked")
            .or_else(|| parse_decimal(data, "frozen"))
            .unwrap_or(Decimal::ZERO);

        let total = parse_decimal(data, "walletBalance")
            .or_else(|| parse_decimal(data, "equity"))
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
            "status": "Trading",
            "lotSizeFilter": {
                "basePrecision": "0.0001",
                "minOrderQty": "0.0001",
                "maxOrderQty": "100"
            },
            "priceFilter": {
                "tickSize": "0.01"
            }
        });

        let market = parse_market(&data).unwrap();
        assert_eq!(market.id, "BTCUSDT");
        assert_eq!(market.symbol, "BTC/USDT");
        assert_eq!(market.base, "BTC");
        assert_eq!(market.quote, "USDT");
        assert!(market.active);
        assert_eq!(market.market_type, MarketType::Spot);
    }

    #[test]
    fn test_parse_ticker() {
        let data = json!({
            "symbol": "BTCUSDT",
            "lastPrice": "50000.00",
            "highPrice24h": "51000.00",
            "lowPrice24h": "49000.00",
            "bid1Price": "49999.00",
            "ask1Price": "50001.00",
            "volume24h": "1000.5",
            "time": "1700000000000"
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
            "b": [
                ["50000.00", "1.5"],
                ["49999.00", "2.0"]
            ],
            "a": [
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
            "execId": "123456",
            "symbol": "BTCUSDT",
            "side": "Buy",
            "price": "50000.00",
            "size": "0.5",
            "time": "1700000000000"
        });

        let trade = parse_trade(&data, None).unwrap();
        assert_eq!(trade.id, Some("123456".to_string()));
        assert_eq!(trade.side, OrderSide::Buy);
        assert_eq!(trade.price, Price::new(dec!(50000.00)));
        assert_eq!(trade.amount, Amount::new(dec!(0.5)));
    }

    #[test]
    fn test_parse_ohlcv_array() {
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
    fn test_parse_ohlcv_object() {
        let data = json!({
            "startTime": "1700000000000",
            "openPrice": "50000.00",
            "highPrice": "51000.00",
            "lowPrice": "49000.00",
            "closePrice": "50500.00",
            "volume": "1000.5"
        });

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
        assert_eq!(parse_order_status("New"), OrderStatus::Open);
        assert_eq!(parse_order_status("PartiallyFilled"), OrderStatus::Open);
        assert_eq!(parse_order_status("Filled"), OrderStatus::Closed);
        assert_eq!(parse_order_status("Cancelled"), OrderStatus::Cancelled);
        assert_eq!(parse_order_status("Rejected"), OrderStatus::Rejected);
        assert_eq!(parse_order_status("Expired"), OrderStatus::Expired);
    }

    #[test]
    fn test_parse_order() {
        let data = json!({
            "orderId": "123456789",
            "symbol": "BTCUSDT",
            "side": "Buy",
            "orderType": "Limit",
            "price": "50000.00",
            "qty": "0.5",
            "orderStatus": "New",
            "createdTime": "1700000000000"
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
            "coin": [
                {
                    "coin": "BTC",
                    "walletBalance": "2.0",
                    "availableToWithdraw": "1.5",
                    "locked": "0.5"
                },
                {
                    "coin": "USDT",
                    "walletBalance": "10000.00",
                    "availableToWithdraw": "10000.00",
                    "locked": "0"
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
