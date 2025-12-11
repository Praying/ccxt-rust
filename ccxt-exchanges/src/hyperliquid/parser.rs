//! HyperLiquid data parser module.
//!
//! Converts HyperLiquid API response data into standardized CCXT format structures.

use ccxt_core::{
    Result,
    error::{Error, ParseError},
    types::{
        Balance, BalanceEntry, Market, MarketLimits, MarketPrecision, MarketType, MinMax, Order,
        OrderBook, OrderBookEntry, OrderSide, OrderStatus, OrderType, Ticker, Trade,
        financial::{Amount, Cost, Price},
    },
};
use rust_decimal::Decimal;
use rust_decimal::prelude::{FromPrimitive, FromStr};
use serde_json::Value;
use std::collections::HashMap;

// ============================================================================
// Helper Functions
// ============================================================================

/// Parse a `Decimal` value from JSON.
pub fn parse_decimal(data: &Value, key: &str) -> Option<Decimal> {
    data.get(key).and_then(|v| {
        if let Some(num) = v.as_f64() {
            Decimal::from_f64(num)
        } else if let Some(s) = v.as_str() {
            Decimal::from_str(s).ok()
        } else {
            None
        }
    })
}

/// Parse a timestamp from JSON.
pub fn parse_timestamp(data: &Value, key: &str) -> Option<i64> {
    data.get(key).and_then(|v| {
        v.as_i64()
            .or_else(|| v.as_str().and_then(|s| s.parse::<i64>().ok()))
    })
}

/// Convert millisecond timestamp to ISO8601 datetime string.
pub fn timestamp_to_datetime(timestamp: i64) -> Option<String> {
    chrono::DateTime::from_timestamp_millis(timestamp)
        .map(|dt| dt.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string())
}

/// Convert a JSON `Value` into a `HashMap<String, Value>`.
fn value_to_hashmap(data: &Value) -> HashMap<String, Value> {
    data.as_object()
        .map(|obj| obj.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
        .unwrap_or_default()
}

// ============================================================================
// Market Parser
// ============================================================================

/// Parse market data from HyperLiquid meta response.
pub fn parse_market(data: &Value, index: usize) -> Result<Market> {
    let name = data["name"]
        .as_str()
        .ok_or_else(|| Error::from(ParseError::missing_field("name")))?;

    // HyperLiquid uses format like "BTC" for the asset name
    // We convert to unified format: "BTC/USDC:USDC"
    let symbol = format!("{}/USDC:USDC", name);
    let id = index.to_string();

    // Parse size decimals for precision
    let sz_decimals = data["szDecimals"].as_u64().unwrap_or(4) as u32;
    let amount_precision = Decimal::new(1, sz_decimals);

    // Price precision (HyperLiquid uses 5 significant figures typically)
    let price_precision = Decimal::new(1, 5);

    // Parse the symbol to get structured representation
    let parsed_symbol = ccxt_core::symbol::SymbolParser::parse(&symbol).ok();

    Ok(Market {
        id,
        symbol: symbol.clone(),
        parsed_symbol,
        base: name.to_string(),
        quote: "USDC".to_string(),
        settle: Some("USDC".to_string()),
        base_id: Some(name.to_string()),
        quote_id: Some("USDC".to_string()),
        settle_id: Some("USDC".to_string()),
        market_type: MarketType::Swap,
        active: true,
        margin: true,
        contract: Some(true),
        linear: Some(true),
        inverse: Some(false),
        contract_size: Some(Decimal::ONE),
        expiry: None,
        expiry_datetime: None,
        strike: None,
        option_type: None,
        precision: MarketPrecision {
            price: Some(price_precision),
            amount: Some(amount_precision),
            base: None,
            quote: None,
        },
        limits: MarketLimits {
            amount: Some(MinMax {
                min: Some(amount_precision),
                max: None,
            }),
            price: None,
            cost: Some(MinMax {
                min: Some(Decimal::new(10, 0)), // $10 minimum
                max: None,
            }),
            leverage: Some(MinMax {
                min: Some(Decimal::ONE),
                max: Some(Decimal::new(50, 0)),
            }),
        },
        maker: Some(Decimal::new(2, 4)), // 0.02%
        taker: Some(Decimal::new(5, 4)), // 0.05%
        percentage: Some(true),
        tier_based: Some(true),
        fee_side: Some("quote".to_string()),
        info: value_to_hashmap(data),
    })
}

// ============================================================================
// Ticker Parser
// ============================================================================

/// Parse ticker data from HyperLiquid all_mids response.
pub fn parse_ticker(symbol: &str, mid_price: Decimal, _market: Option<&Market>) -> Result<Ticker> {
    let timestamp = chrono::Utc::now().timestamp_millis();

    Ok(Ticker {
        symbol: symbol.to_string(),
        timestamp,
        datetime: timestamp_to_datetime(timestamp),
        high: None,
        low: None,
        bid: Some(Price::new(mid_price)),
        bid_volume: None,
        ask: Some(Price::new(mid_price)),
        ask_volume: None,
        vwap: None,
        open: None,
        close: Some(Price::new(mid_price)),
        last: Some(Price::new(mid_price)),
        previous_close: None,
        change: None,
        percentage: None,
        average: None,
        base_volume: None,
        quote_volume: None,
        info: HashMap::new(),
    })
}

// ============================================================================
// OrderBook Parser
// ============================================================================

/// Parse orderbook data from HyperLiquid L2 book response.
pub fn parse_orderbook(data: &Value, symbol: String) -> Result<OrderBook> {
    let timestamp = chrono::Utc::now().timestamp_millis();

    let mut bids = Vec::new();
    let mut asks = Vec::new();

    // Parse levels
    if let Some(levels) = data["levels"].as_array() {
        if levels.len() >= 2 {
            // First array is bids, second is asks
            if let Some(bid_levels) = levels[0].as_array() {
                for level in bid_levels {
                    if let (Some(px), Some(sz)) = (
                        level["px"].as_str().and_then(|s| Decimal::from_str(s).ok()),
                        level["sz"].as_str().and_then(|s| Decimal::from_str(s).ok()),
                    ) {
                        bids.push(OrderBookEntry {
                            price: Price::new(px),
                            amount: Amount::new(sz),
                        });
                    }
                }
            }

            if let Some(ask_levels) = levels[1].as_array() {
                for level in ask_levels {
                    if let (Some(px), Some(sz)) = (
                        level["px"].as_str().and_then(|s| Decimal::from_str(s).ok()),
                        level["sz"].as_str().and_then(|s| Decimal::from_str(s).ok()),
                    ) {
                        asks.push(OrderBookEntry {
                            price: Price::new(px),
                            amount: Amount::new(sz),
                        });
                    }
                }
            }
        }
    }

    // Sort bids descending, asks ascending
    bids.sort_by(|a, b| b.price.cmp(&a.price));
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

// ============================================================================
// Trade Parser
// ============================================================================

/// Parse trade data from HyperLiquid fill response.
pub fn parse_trade(data: &Value, market: Option<&Market>) -> Result<Trade> {
    let symbol = market
        .map(|m| m.symbol.clone())
        .unwrap_or_else(|| data["coin"].as_str().unwrap_or("").to_string());

    let timestamp = parse_timestamp(data, "time").unwrap_or(0);

    let side = match data["side"].as_str() {
        Some("B") | Some("buy") | Some("Buy") => OrderSide::Buy,
        Some("A") | Some("sell") | Some("Sell") => OrderSide::Sell,
        _ => OrderSide::Buy,
    };

    let price = parse_decimal(data, "px").unwrap_or(Decimal::ZERO);
    let amount = parse_decimal(data, "sz").unwrap_or(Decimal::ZERO);
    let cost = price * amount;

    Ok(Trade {
        id: data["tid"]
            .as_str()
            .or(data["hash"].as_str())
            .map(|s| s.to_string()),
        order: data["oid"].as_str().map(|s| s.to_string()),
        timestamp,
        datetime: timestamp_to_datetime(timestamp),
        symbol,
        trade_type: None,
        side,
        taker_or_maker: None,
        price: Price::new(price),
        amount: Amount::new(amount),
        cost: Some(Cost::new(cost)),
        fee: None,
        info: value_to_hashmap(data),
    })
}

// ============================================================================
// Order Parser
// ============================================================================

/// Parse order status from HyperLiquid status string.
pub fn parse_order_status(status: &str) -> OrderStatus {
    match status.to_lowercase().as_str() {
        "open" | "resting" => OrderStatus::Open,
        "filled" => OrderStatus::Closed,
        "canceled" | "cancelled" => OrderStatus::Canceled,
        "rejected" => OrderStatus::Rejected,
        _ => OrderStatus::Open,
    }
}

/// Parse order data from HyperLiquid order response.
pub fn parse_order(data: &Value, market: Option<&Market>) -> Result<Order> {
    let symbol = market.map(|m| m.symbol.clone()).unwrap_or_else(|| {
        data["coin"]
            .as_str()
            .map(|c| format!("{}/USDC:USDC", c))
            .unwrap_or_default()
    });

    let id = data["oid"]
        .as_u64()
        .map(|n| n.to_string())
        .or_else(|| data["oid"].as_str().map(|s| s.to_string()))
        .unwrap_or_default();

    let timestamp = parse_timestamp(data, "timestamp");

    let status_str = data["status"].as_str().unwrap_or("open");
    let status = parse_order_status(status_str);

    let side = match data["side"].as_str() {
        Some("B") | Some("buy") => OrderSide::Buy,
        Some("A") | Some("sell") => OrderSide::Sell,
        _ => {
            // Check isBuy field
            if data["isBuy"].as_bool().unwrap_or(true) {
                OrderSide::Buy
            } else {
                OrderSide::Sell
            }
        }
    };

    let order_type = match data["orderType"].as_str() {
        Some("Limit") | Some("limit") => OrderType::Limit,
        Some("Market") | Some("market") => OrderType::Market,
        _ => OrderType::Limit,
    };

    let price = parse_decimal(data, "limitPx").or_else(|| parse_decimal(data, "px"));
    let amount = parse_decimal(data, "sz")
        .or_else(|| parse_decimal(data, "origSz"))
        .unwrap_or(Decimal::ZERO);
    let filled = parse_decimal(data, "filledSz");
    let remaining = filled.map(|f| amount - f);

    Ok(Order {
        id,
        client_order_id: data["cloid"].as_str().map(|s| s.to_string()),
        timestamp,
        datetime: timestamp.and_then(timestamp_to_datetime),
        last_trade_timestamp: None,
        status,
        symbol,
        order_type,
        time_in_force: data["tif"].as_str().map(|s| s.to_uppercase()),
        side,
        price,
        average: parse_decimal(data, "avgPx"),
        amount,
        filled,
        remaining,
        cost: None,
        trades: None,
        fee: None,
        post_only: None,
        reduce_only: data["reduceOnly"].as_bool(),
        trigger_price: parse_decimal(data, "triggerPx"),
        stop_price: None,
        take_profit_price: None,
        stop_loss_price: None,
        trailing_delta: None,
        trailing_percent: None,
        activation_price: None,
        callback_rate: None,
        working_type: None,
        fees: Some(Vec::new()),
        info: value_to_hashmap(data),
    })
}

// ============================================================================
// OHLCV Parser
// ============================================================================

/// Parse OHLCV (candlestick) data from HyperLiquid candle response.
///
/// HyperLiquid returns candle data as arrays: [timestamp, open, high, low, close, volume]
pub fn parse_ohlcv(data: &Value) -> Result<ccxt_core::types::Ohlcv> {
    // Handle array format: [timestamp, open, high, low, close, volume]
    if let Some(arr) = data.as_array() {
        if arr.len() >= 6 {
            let timestamp = arr[0]
                .as_i64()
                .or_else(|| arr[0].as_str().and_then(|s| s.parse().ok()))
                .ok_or_else(|| Error::from(ParseError::missing_field("timestamp")))?;

            let open = parse_decimal_from_value(&arr[1])
                .ok_or_else(|| Error::from(ParseError::missing_field("open")))?;
            let high = parse_decimal_from_value(&arr[2])
                .ok_or_else(|| Error::from(ParseError::missing_field("high")))?;
            let low = parse_decimal_from_value(&arr[3])
                .ok_or_else(|| Error::from(ParseError::missing_field("low")))?;
            let close = parse_decimal_from_value(&arr[4])
                .ok_or_else(|| Error::from(ParseError::missing_field("close")))?;
            let volume = parse_decimal_from_value(&arr[5])
                .ok_or_else(|| Error::from(ParseError::missing_field("volume")))?;

            return Ok(ccxt_core::types::Ohlcv {
                timestamp,
                open: ccxt_core::types::financial::Price::new(open),
                high: ccxt_core::types::financial::Price::new(high),
                low: ccxt_core::types::financial::Price::new(low),
                close: ccxt_core::types::financial::Price::new(close),
                volume: ccxt_core::types::financial::Amount::new(volume),
            });
        }
    }

    // Handle object format
    let timestamp = parse_timestamp(data, "t")
        .or_else(|| parse_timestamp(data, "timestamp"))
        .ok_or_else(|| Error::from(ParseError::missing_field("timestamp")))?;

    let open = parse_decimal(data, "o")
        .or_else(|| parse_decimal(data, "open"))
        .ok_or_else(|| Error::from(ParseError::missing_field("open")))?;
    let high = parse_decimal(data, "h")
        .or_else(|| parse_decimal(data, "high"))
        .ok_or_else(|| Error::from(ParseError::missing_field("high")))?;
    let low = parse_decimal(data, "l")
        .or_else(|| parse_decimal(data, "low"))
        .ok_or_else(|| Error::from(ParseError::missing_field("low")))?;
    let close = parse_decimal(data, "c")
        .or_else(|| parse_decimal(data, "close"))
        .ok_or_else(|| Error::from(ParseError::missing_field("close")))?;
    let volume = parse_decimal(data, "v")
        .or_else(|| parse_decimal(data, "volume"))
        .ok_or_else(|| Error::from(ParseError::missing_field("volume")))?;

    Ok(ccxt_core::types::Ohlcv {
        timestamp,
        open: ccxt_core::types::financial::Price::new(open),
        high: ccxt_core::types::financial::Price::new(high),
        low: ccxt_core::types::financial::Price::new(low),
        close: ccxt_core::types::financial::Price::new(close),
        volume: ccxt_core::types::financial::Amount::new(volume),
    })
}

/// Helper to parse decimal from a JSON value directly
fn parse_decimal_from_value(v: &Value) -> Option<Decimal> {
    if let Some(num) = v.as_f64() {
        Decimal::from_f64(num)
    } else if let Some(s) = v.as_str() {
        Decimal::from_str(s).ok()
    } else {
        None
    }
}

// ============================================================================
// Balance Parser
// ============================================================================

/// Parse balance data from HyperLiquid user state response.
pub fn parse_balance(data: &Value) -> Result<Balance> {
    let mut balances = HashMap::new();

    // Parse margin summary
    if let Some(margin) = data.get("marginSummary") {
        let account_value = parse_decimal(margin, "accountValue").unwrap_or(Decimal::ZERO);
        let total_margin_used = parse_decimal(margin, "totalMarginUsed").unwrap_or(Decimal::ZERO);
        let available = account_value - total_margin_used;

        balances.insert(
            "USDC".to_string(),
            BalanceEntry {
                free: available,
                used: total_margin_used,
                total: account_value,
            },
        );
    }

    // Also check withdrawable
    if let Some(withdrawable) = data.get("withdrawable") {
        if let Some(w) = withdrawable
            .as_str()
            .and_then(|s| Decimal::from_str(s).ok())
        {
            if let Some(entry) = balances.get_mut("USDC") {
                entry.free = w;
            }
        }
    }

    Ok(Balance {
        balances,
        info: value_to_hashmap(data),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;
    use serde_json::json;

    #[test]
    fn test_parse_market() {
        let data = json!({
            "name": "BTC",
            "szDecimals": 4
        });

        let market = parse_market(&data, 0).unwrap();
        assert_eq!(market.symbol, "BTC/USDC:USDC");
        assert_eq!(market.base, "BTC");
        assert_eq!(market.quote, "USDC");
        assert!(market.active);
    }

    #[test]
    fn test_parse_ticker() {
        let ticker = parse_ticker("BTC/USDC:USDC", dec!(50000), None).unwrap();
        assert_eq!(ticker.symbol, "BTC/USDC:USDC");
        assert_eq!(ticker.last, Some(Price::new(dec!(50000))));
    }

    #[test]
    fn test_parse_orderbook() {
        let data = json!({
            "levels": [
                [{"px": "50000", "sz": "1.5"}, {"px": "49999", "sz": "2.0"}],
                [{"px": "50001", "sz": "1.0"}, {"px": "50002", "sz": "3.0"}]
            ]
        });

        let orderbook = parse_orderbook(&data, "BTC/USDC:USDC".to_string()).unwrap();
        assert_eq!(orderbook.bids.len(), 2);
        assert_eq!(orderbook.asks.len(), 2);
        // Bids should be sorted descending
        assert!(orderbook.bids[0].price >= orderbook.bids[1].price);
        // Asks should be sorted ascending
        assert!(orderbook.asks[0].price <= orderbook.asks[1].price);
    }

    #[test]
    fn test_parse_trade() {
        let data = json!({
            "coin": "BTC",
            "side": "B",
            "px": "50000",
            "sz": "0.5",
            "time": 1700000000000i64,
            "tid": "123456"
        });

        let trade = parse_trade(&data, None).unwrap();
        assert_eq!(trade.side, OrderSide::Buy);
        assert_eq!(trade.price, Price::new(dec!(50000)));
        assert_eq!(trade.amount, Amount::new(dec!(0.5)));
    }

    #[test]
    fn test_parse_order_status() {
        assert_eq!(parse_order_status("open"), OrderStatus::Open);
        assert_eq!(parse_order_status("filled"), OrderStatus::Closed);
        assert_eq!(parse_order_status("canceled"), OrderStatus::Canceled);
    }

    #[test]
    fn test_parse_balance() {
        let data = json!({
            "marginSummary": {
                "accountValue": "10000",
                "totalMarginUsed": "2000"
            },
            "withdrawable": "8000"
        });

        let balance = parse_balance(&data).unwrap();
        let usdc = balance.get("USDC").unwrap();
        assert_eq!(usdc.total, dec!(10000));
        assert_eq!(usdc.free, dec!(8000));
    }
}
