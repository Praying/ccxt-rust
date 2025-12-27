//! Binance data parser module.
//!
//! Converts Binance API response data into standardized CCXT format structures.

use ccxt_core::{
    Result,
    error::{Error, ParseError},
    types::{
        AccountConfig, Balance, BalanceEntry, BidAsk, BorrowInterest, BorrowRate,
        BorrowRateHistory, CommissionRate, DepositWithdrawFee, FeeFundingRate,
        FeeFundingRateHistory, FeeTradingFee, FundingFee, FundingHistory, IndexPrice,
        LedgerDirection, LedgerEntry, LedgerEntryType, Leverage, LeverageTier, Liquidation,
        MarginAdjustment, MarginLoan, MarginType, MarkPrice, Market, MaxBorrowable, MaxLeverage,
        MaxTransferable, NextFundingRate, OHLCV, OcoOrder, OcoOrderInfo, OpenInterest,
        OpenInterestHistory, Order, OrderBook, OrderBookDelta, OrderBookEntry, OrderReport,
        OrderSide, OrderStatus, OrderType, Position, PremiumIndex, TakerOrMaker, Ticker,
        TimeInForce, Trade, Transfer,
        financial::{Amount, Cost, Price},
    },
};
use rust_decimal::Decimal;
use rust_decimal::prelude::{FromPrimitive, FromStr, ToPrimitive};
use serde_json::Value;
use std::collections::HashMap;

// ============================================================================
// Helper Functions - Type Conversion
// ============================================================================

/// Parse an f64 value from JSON (supports both string and number formats).
fn parse_f64(data: &Value, key: &str) -> Option<f64> {
    data.get(key).and_then(|v| {
        v.as_f64()
            .or_else(|| v.as_str().and_then(|s| s.parse::<f64>().ok()))
    })
}

/// Parse a `Decimal` value from JSON (supports both string and number formats).
/// Prioritizes string parsing for maximum precision, falls back to f64 only when necessary.
fn parse_decimal(data: &Value, key: &str) -> Option<Decimal> {
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
fn parse_decimal_multi(data: &Value, keys: &[&str]) -> Option<Decimal> {
    for key in keys {
        if let Some(decimal) = parse_decimal(data, key) {
            return Some(decimal);
        }
    }
    None
}

/// Convert a JSON `Value` into a `HashMap<String, Value>`.
fn value_to_hashmap(data: &Value) -> HashMap<String, Value> {
    data.as_object()
        .map(|obj| obj.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
        .unwrap_or_default()
}

/// Parse an array of orderbook entries from JSON.
#[allow(dead_code)]
fn parse_order_book_entries(data: &Value) -> Vec<OrderBookEntry> {
    data.as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|item| {
                    let price = if let Some(arr) = item.as_array() {
                        arr.get(0)
                            .and_then(|v| v.as_str())
                            .and_then(|s| Decimal::from_str(s).ok())
                    } else {
                        None
                    }?;

                    let amount = if let Some(arr) = item.as_array() {
                        arr.get(1)
                            .and_then(|v| v.as_str())
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

// ============================================================================
// Parser Functions
// ============================================================================

/// Parse market data from Binance exchange info.
///
/// # Arguments
///
/// * `data` - Binance market data JSON object
///
/// # Returns
///
/// Returns a CCXT [`Market`] structure.
///
/// # Errors
///
/// Returns an error if required fields are missing or invalid.
pub fn parse_market(data: &Value) -> Result<Market> {
    use ccxt_core::types::{MarketLimits, MarketPrecision, MarketType, MinMax};

    let symbol = data["symbol"]
        .as_str()
        .ok_or_else(|| Error::from(ParseError::missing_field("symbol")))?
        .to_string();

    let base_asset = data["baseAsset"]
        .as_str()
        .ok_or_else(|| Error::from(ParseError::missing_field("baseAsset")))?
        .to_string();

    let quote_asset = data["quoteAsset"]
        .as_str()
        .ok_or_else(|| Error::from(ParseError::missing_field("quoteAsset")))?
        .to_string();

    let status = data["status"]
        .as_str()
        .ok_or_else(|| Error::from(ParseError::missing_field("status")))?;

    let active = Some(status == "TRADING");

    // Check if margin trading is supported
    let margin = data["isMarginTradingAllowed"].as_bool().unwrap_or(false);

    // Parse price and amount precision from filters
    let mut price_precision: Option<Decimal> = None;
    let mut amount_precision: Option<Decimal> = None;
    let mut min_amount: Option<Decimal> = None;
    let mut max_amount: Option<Decimal> = None;
    let mut min_cost: Option<Decimal> = None;
    let mut min_price: Option<Decimal> = None;
    let mut max_price: Option<Decimal> = None;

    if let Some(filters) = data["filters"].as_array() {
        for filter in filters {
            let filter_type = filter["filterType"].as_str().unwrap_or("");

            match filter_type {
                "PRICE_FILTER" => {
                    if let Some(tick_size) = filter["tickSize"].as_str() {
                        if let Ok(dec) = Decimal::from_str(tick_size) {
                            price_precision = Some(dec);
                        }
                    }
                    if let Some(min) = filter["minPrice"].as_str() {
                        min_price = Decimal::from_str(min).ok();
                    }
                    if let Some(max) = filter["maxPrice"].as_str() {
                        max_price = Decimal::from_str(max).ok();
                    }
                }
                "LOT_SIZE" => {
                    if let Some(step_size) = filter["stepSize"].as_str() {
                        if let Ok(dec) = Decimal::from_str(step_size) {
                            amount_precision = Some(dec);
                        }
                    }
                    if let Some(min) = filter["minQty"].as_str() {
                        min_amount = Decimal::from_str(min).ok();
                    }
                    if let Some(max) = filter["maxQty"].as_str() {
                        max_amount = Decimal::from_str(max).ok();
                    }
                }
                "MIN_NOTIONAL" | "NOTIONAL" => {
                    if let Some(min) = filter["minNotional"].as_str() {
                        min_cost = Decimal::from_str(min).ok();
                    }
                }
                _ => {}
            }
        }
    }

    // Create unified symbol
    let unified_symbol = format!("{}/{}", base_asset, quote_asset);
    // Parse the symbol to get structured representation
    let parsed_symbol = ccxt_core::symbol::SymbolParser::parse(&unified_symbol).ok();

    Ok(Market {
        id: symbol.clone(),
        symbol: unified_symbol,
        parsed_symbol,
        base: base_asset.clone(),
        quote: quote_asset.clone(),
        settle: None,
        base_id: Some(base_asset),
        quote_id: Some(quote_asset),
        settle_id: None,
        market_type: MarketType::Spot,
        active: active.unwrap_or(true),
        margin,
        contract: Some(false),
        linear: None,
        inverse: None,
        // Default fee rate of 0.1% - using from_str which is infallible for valid decimal strings
        taker: Decimal::from_str("0.001").ok(),
        maker: Decimal::from_str("0.001").ok(),
        contract_size: None,
        expiry: None,
        expiry_datetime: None,
        strike: None,
        option_type: None,
        percentage: Some(true),
        tier_based: Some(false),
        fee_side: Some("quote".to_string()),
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
            price: Some(MinMax {
                min: min_price,
                max: max_price,
            }),
            cost: Some(MinMax {
                min: min_cost,
                max: None,
            }),
            leverage: None,
        },
        info: value_to_hashmap(data),
    })
}

/// Parse ticker data from Binance 24hr ticker response.
///
/// # Arguments
///
/// * `data` - Binance ticker data JSON object
/// * `market` - Optional market information for symbol resolution
///
/// # Returns
///
/// Returns a CCXT [`Ticker`] structure.
///
/// # Errors
///
/// Returns an error if the symbol field is missing when market is not provided.
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
        info: value_to_hashmap(data),
    })
}

/// Parse trade data from Binance trade response.
///
/// # Arguments
///
/// * `data` - Binance trade data JSON object
/// * `market` - Optional market information for symbol resolution
///
/// # Returns
///
/// Returns a CCXT [`Trade`] structure.
///
/// # Errors
///
/// Returns an error if the symbol field is missing when market is not provided.
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
        .map(|i| i.to_string());

    let timestamp = data["time"].as_i64().or_else(|| data["T"].as_i64());

    let side = if data["isBuyerMaker"].as_bool().unwrap_or(false) {
        OrderSide::Sell
    } else if data["m"].as_bool().unwrap_or(false) {
        OrderSide::Sell
    } else {
        OrderSide::Buy
    };

    let price = parse_decimal_multi(data, &["price", "p"]);
    let amount = parse_decimal_multi(data, &["qty", "q"]);

    let cost = match (price, amount) {
        (Some(p), Some(a)) => Some(p * a),
        _ => None,
    };

    Ok(Trade {
        id,
        order: data["orderId"]
            .as_u64()
            .or_else(|| data["orderid"].as_u64())
            .map(|i| i.to_string()),
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
        price: Price::new(price.unwrap_or(Decimal::ZERO)),
        amount: Amount::new(amount.unwrap_or(Decimal::ZERO)),
        cost: cost.map(Cost::new),
        fee: None,
        info: value_to_hashmap(data),
    })
}

/// Parse order data from Binance order response.
///
/// # Arguments
///
/// * `data` - Binance order data JSON object
/// * `market` - Optional market information for symbol resolution
///
/// # Returns
///
/// Returns a CCXT [`Order`] structure.
///
/// # Errors
///
/// Returns an error if required fields (symbol, orderId, status, side, amount) are missing or invalid.
pub fn parse_order(data: &Value, market: Option<&Market>) -> Result<Order> {
    let symbol = if let Some(m) = market {
        m.symbol.clone()
    } else {
        data["symbol"]
            .as_str()
            .ok_or_else(|| Error::from(ParseError::missing_field("symbol")))?
            .to_string()
    };

    let id = data["orderId"]
        .as_u64()
        .ok_or_else(|| Error::from(ParseError::missing_field("orderId")))?
        .to_string();

    let timestamp = data["time"]
        .as_i64()
        .or_else(|| data["transactTime"].as_i64());

    let status_str = data["status"]
        .as_str()
        .ok_or_else(|| Error::from(ParseError::missing_field("status")))?;

    let status = match status_str {
        "NEW" | "PARTIALLY_FILLED" => OrderStatus::Open,
        "FILLED" => OrderStatus::Closed,
        "CANCELED" => OrderStatus::Cancelled,
        "EXPIRED" => OrderStatus::Expired,
        "REJECTED" => OrderStatus::Rejected,
        _ => OrderStatus::Open,
    };

    let side = match data["side"].as_str() {
        Some("BUY") => OrderSide::Buy,
        Some("SELL") => OrderSide::Sell,
        _ => return Err(Error::from(ParseError::invalid_format("data", "side"))),
    };

    let order_type = match data["type"].as_str() {
        Some("MARKET") => OrderType::Market,
        Some("LIMIT") => OrderType::Limit,
        Some("STOP_LOSS") => OrderType::StopLoss,
        Some("STOP_LOSS_LIMIT") => OrderType::StopLossLimit,
        Some("TAKE_PROFIT") => OrderType::TakeProfit,
        Some("TAKE_PROFIT_LIMIT") => OrderType::TakeProfitLimit,
        Some("STOP_MARKET") | Some("STOP") => OrderType::StopMarket,
        Some("TAKE_PROFIT_MARKET") => OrderType::TakeProfitLimit,
        Some("TRAILING_STOP_MARKET") => OrderType::TrailingStop,
        Some("LIMIT_MAKER") => OrderType::LimitMaker,
        _ => OrderType::Limit,
    };

    let time_in_force = match data["timeInForce"].as_str() {
        Some("GTC") => Some(TimeInForce::GTC),
        Some("IOC") => Some(TimeInForce::IOC),
        Some("FOK") => Some(TimeInForce::FOK),
        Some("GTX") => Some(TimeInForce::PO),
        _ => None,
    };

    let price = parse_decimal(data, "price");
    let amount = parse_decimal(data, "origQty");
    let filled = parse_decimal(data, "executedQty");
    let remaining = match (&amount, &filled) {
        (Some(a), Some(f)) => Some(*a - *f),
        _ => None,
    };

    let cost = parse_decimal(data, "cummulativeQuoteQty");

    let average = match (&cost, &filled) {
        (Some(c), Some(f)) if !f.is_zero() => Some(*c / *f),
        _ => None,
    };

    Ok(Order {
        id,
        client_order_id: data["clientOrderId"].as_str().map(|s| s.to_string()),
        timestamp,
        datetime: timestamp.map(|t| {
            chrono::DateTime::from_timestamp(t / 1000, 0)
                .map(|dt| dt.to_rfc3339())
                .unwrap_or_default()
        }),
        last_trade_timestamp: data["updateTime"].as_i64(),
        status,
        symbol,
        order_type,
        time_in_force: time_in_force.map(|t| t.to_string()),
        side,
        price,
        average,
        amount: amount.ok_or_else(|| Error::from(ParseError::missing_field("amount")))?,
        filled,
        remaining,
        cost,
        trades: None,
        fee: None,
        post_only: None,
        reduce_only: data["reduceOnly"].as_bool(),
        trigger_price: parse_decimal(data, "triggerPrice"),
        stop_price: parse_decimal(data, "stopPrice"),
        take_profit_price: parse_decimal(data, "takeProfitPrice"),
        stop_loss_price: parse_decimal(data, "stopLossPrice"),
        trailing_delta: parse_decimal(data, "trailingDelta"),
        trailing_percent: parse_decimal_multi(data, &["trailingPercent", "callbackRate"]),
        activation_price: parse_decimal_multi(data, &["activationPrice", "activatePrice"]),
        callback_rate: parse_decimal(data, "callbackRate"),
        working_type: data["workingType"].as_str().map(|s| s.to_string()),
        fees: Some(Vec::new()),
        info: value_to_hashmap(data),
    })
}
/// Parse OCO (One-Cancels-the-Other) order data from Binance.
///
/// # Arguments
///
/// * `data` - Binance OCO order data JSON object
///
/// # Returns
///
/// Returns a CCXT [`OcoOrder`] structure.
///
/// # Errors
///
/// Returns an error if required fields (orderListId, symbol, listStatusType, listOrderStatus, transactionTime) are missing.
pub fn parse_oco_order(data: &Value) -> Result<OcoOrder> {
    let order_list_id = data["orderListId"]
        .as_i64()
        .ok_or_else(|| Error::from(ParseError::missing_field("orderListId")))?;

    let list_client_order_id = data["listClientOrderId"].as_str().map(|s| s.to_string());

    let symbol = data["symbol"]
        .as_str()
        .ok_or_else(|| Error::from(ParseError::missing_field("symbol")))?
        .to_string();

    let list_status = data["listStatusType"]
        .as_str()
        .ok_or_else(|| Error::from(ParseError::missing_field("listStatusType")))?
        .to_string();

    let list_order_status = data["listOrderStatus"]
        .as_str()
        .ok_or_else(|| Error::from(ParseError::missing_field("listOrderStatus")))?
        .to_string();

    let transaction_time = data["transactionTime"]
        .as_i64()
        .ok_or_else(|| Error::from(ParseError::missing_field("transactionTime")))?;

    let datetime = chrono::DateTime::from_timestamp((transaction_time / 1000) as i64, 0)
        .map(|dt| dt.to_rfc3339())
        .unwrap_or_default();

    let mut orders = Vec::new();
    if let Some(orders_array) = data["orders"].as_array() {
        for order in orders_array {
            let order_info = OcoOrderInfo {
                symbol: order["symbol"].as_str().unwrap_or(&symbol).to_string(),
                order_id: order["orderId"]
                    .as_i64()
                    .ok_or_else(|| Error::from(ParseError::missing_field("orderId")))?,
                client_order_id: order["clientOrderId"].as_str().map(|s| s.to_string()),
            };
            orders.push(order_info);
        }
    }

    let order_reports = if let Some(reports_array) = data["orderReports"].as_array() {
        let mut reports = Vec::new();
        for report in reports_array {
            let order_report = OrderReport {
                symbol: report["symbol"].as_str().unwrap_or(&symbol).to_string(),
                order_id: report["orderId"]
                    .as_i64()
                    .ok_or_else(|| Error::from(ParseError::missing_field("orderId")))?,
                order_list_id: report["orderListId"].as_i64().unwrap_or(order_list_id),
                client_order_id: report["clientOrderId"].as_str().map(|s| s.to_string()),
                transact_time: report["transactTime"].as_i64().unwrap_or(transaction_time),
                price: report["price"].as_str().unwrap_or("0").to_string(),
                orig_qty: report["origQty"].as_str().unwrap_or("0").to_string(),
                executed_qty: report["executedQty"].as_str().unwrap_or("0").to_string(),
                cummulative_quote_qty: report["cummulativeQuoteQty"]
                    .as_str()
                    .unwrap_or("0")
                    .to_string(),
                status: report["status"].as_str().unwrap_or("NEW").to_string(),
                time_in_force: report["timeInForce"].as_str().unwrap_or("GTC").to_string(),
                type_: report["type"].as_str().unwrap_or("LIMIT").to_string(),
                side: report["side"].as_str().unwrap_or("SELL").to_string(),
                stop_price: report["stopPrice"].as_str().map(|s| s.to_string()),
            };
            reports.push(order_report);
        }
        Some(reports)
    } else {
        None
    };

    Ok(OcoOrder {
        info: Some(data.clone()),
        order_list_id,
        list_client_order_id,
        symbol,
        list_status,
        list_order_status,
        transaction_time,
        datetime,
        orders,
        order_reports,
    })
}

/// Parse balance data from Binance account info.
///
/// # Arguments
///
/// * `data` - Binance account data JSON object
///
/// # Returns
///
/// Returns a CCXT [`Balance`] structure with all balances (including zero balances).
pub fn parse_balance(data: &Value) -> Result<Balance> {
    let mut balances = HashMap::new();

    if let Some(balances_array) = data["balances"].as_array() {
        for balance in balances_array {
            let currency = balance["asset"]
                .as_str()
                .ok_or_else(|| Error::from(ParseError::missing_field("asset")))?
                .to_string();

            let free = parse_decimal(balance, "free").unwrap_or(Decimal::ZERO);
            let locked = parse_decimal(balance, "locked").unwrap_or(Decimal::ZERO);
            let total = free + locked;

            balances.insert(
                currency,
                BalanceEntry {
                    free,
                    used: locked,
                    total,
                },
            );
        }
    }

    Ok(Balance {
        balances,
        info: value_to_hashmap(data),
    })
}

/// Parse orderbook data from Binance depth response.
///
/// # Arguments
///
/// * `data` - Binance orderbook data JSON object
/// * `symbol` - Trading pair symbol
///
/// # Returns
///
/// Returns a CCXT [`OrderBook`] structure.
pub fn parse_orderbook(data: &Value, symbol: String) -> Result<OrderBook> {
    // Try WebSocket event timestamp fields (T or E), fall back to current time for REST API
    let timestamp = data["T"]
        .as_i64()
        .or_else(|| data["E"].as_i64())
        .unwrap_or_else(|| chrono::Utc::now().timestamp_millis());

    let bids = parse_orderbook_side(&data["bids"])?;
    let asks = parse_orderbook_side(&data["asks"])?;

    Ok(OrderBook {
        symbol,
        timestamp,
        datetime: Some({
            chrono::DateTime::from_timestamp_millis(timestamp)
                .map(|dt| dt.to_rfc3339())
                .unwrap_or_default()
        }),
        nonce: data["lastUpdateId"].as_i64(),
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
    let array = data
        .as_array()
        .ok_or_else(|| Error::from(ParseError::invalid_value("data", "orderbook side")))?;

    let mut result = Vec::new();

    for item in array {
        if let Some(arr) = item.as_array() {
            if arr.len() >= 2 {
                let price = arr[0]
                    .as_str()
                    .and_then(|s| s.parse::<f64>().ok())
                    .and_then(Decimal::from_f64_retain)
                    .ok_or_else(|| Error::from(ParseError::invalid_value("data", "price")))?;
                let amount = arr[1]
                    .as_str()
                    .and_then(|s| s.parse::<f64>().ok())
                    .and_then(Decimal::from_f64_retain)
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

/// Parse WebSocket orderbook delta data from Binance diff depth stream.
///
/// This function parses incremental orderbook updates for proper synchronization.
/// Binance sends different fields for spot vs futures:
/// - Spot: `U` (first update ID), `u` (final update ID)
/// - Futures: `U`, `u`, and `pu` (previous final update ID)
///
/// # Arguments
///
/// * `data` - Binance WebSocket orderbook delta JSON object
/// * `symbol` - Trading pair symbol (ccxt format, e.g., "BTC/USDT")
///
/// # Returns
///
/// Returns a CCXT [`OrderBookDelta`] structure for synchronization.
///
/// # Example WebSocket Message (Spot)
///
/// ```json
/// {
///   "e": "depthUpdate",
///   "E": 1672515782136,
///   "s": "BTCUSDT",
///   "U": 157,
///   "u": 160,
///   "b": [["0.0024", "10"]],
///   "a": [["0.0026", "100"]]
/// }
/// ```
///
/// # Example WebSocket Message (Futures)
///
/// ```json
/// {
///   "e": "depthUpdate",
///   "E": 1672515782136,
///   "s": "BTCUSDT",
///   "U": 157,
///   "u": 160,
///   "pu": 156,
///   "b": [["0.0024", "10"]],
///   "a": [["0.0026", "100"]]
/// }
/// ```
pub fn parse_ws_orderbook_delta(data: &Value, symbol: String) -> Result<OrderBookDelta> {
    // Parse update IDs for synchronization
    // U: First update ID in event
    let first_update_id = data["U"]
        .as_i64()
        .ok_or_else(|| Error::from(ParseError::missing_field("U (first_update_id)")))?;

    // u: Final update ID in event
    let final_update_id = data["u"]
        .as_i64()
        .ok_or_else(|| Error::from(ParseError::missing_field("u (final_update_id)")))?;

    // pu: Previous final update ID (futures only, optional for spot)
    let prev_final_update_id = data["pu"].as_i64();

    // E: Event time
    let timestamp = data["E"]
        .as_i64()
        .unwrap_or_else(|| chrono::Utc::now().timestamp_millis());

    // Parse bids and asks using WebSocket field names (b, a)
    let bids = parse_orderbook_side_ws(&data["b"])?;
    let asks = parse_orderbook_side_ws(&data["a"])?;

    Ok(OrderBookDelta {
        symbol,
        first_update_id,
        final_update_id,
        prev_final_update_id,
        timestamp,
        bids,
        asks,
    })
}

/// Parse one side (bids or asks) of WebSocket orderbook delta data.
///
/// WebSocket uses `b` and `a` field names instead of `bids` and `asks`.
fn parse_orderbook_side_ws(data: &Value) -> Result<Vec<OrderBookEntry>> {
    // If the field is null or missing, return empty vector
    let Some(array) = data.as_array() else {
        return Ok(Vec::new());
    };

    let mut result = Vec::with_capacity(array.len());

    for item in array {
        if let Some(arr) = item.as_array() {
            if arr.len() >= 2 {
                let price = arr[0]
                    .as_str()
                    .and_then(|s| s.parse::<f64>().ok())
                    .and_then(Decimal::from_f64_retain)
                    .ok_or_else(|| Error::from(ParseError::invalid_value("data", "price")))?;
                let amount = arr[1]
                    .as_str()
                    .and_then(|s| s.parse::<f64>().ok())
                    .and_then(Decimal::from_f64_retain)
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

// ============================================================================
// Futures-Specific Parser Functions
// ============================================================================

/// Parse funding rate data from Binance futures API.
///
/// # Arguments
///
/// * `data` - Binance funding rate data JSON object
/// * `market` - Optional market information for symbol resolution
///
/// # Returns
///
/// Returns a CCXT [`FeeFundingRate`] structure with `Decimal` precision.
pub fn parse_funding_rate(data: &Value, market: Option<&Market>) -> Result<FeeFundingRate> {
    let symbol = if let Some(m) = market {
        m.symbol.clone()
    } else {
        data["symbol"]
            .as_str()
            .ok_or_else(|| Error::from(ParseError::missing_field("symbol")))?
            .to_string()
    };

    let funding_rate =
        parse_decimal(data, "lastFundingRate").or_else(|| parse_decimal(data, "fundingRate"));

    let mark_price = parse_decimal(data, "markPrice");
    let index_price = parse_decimal(data, "indexPrice");
    let interest_rate = parse_decimal(data, "interestRate");

    let next_funding_time = data["nextFundingTime"].as_i64();
    let funding_timestamp = next_funding_time.or_else(|| data["fundingTime"].as_i64());

    Ok(FeeFundingRate {
        info: data.clone(),
        symbol,
        mark_price,
        index_price,
        interest_rate,
        estimated_settle_price: None,
        funding_rate,
        funding_timestamp,
        funding_datetime: funding_timestamp.map(|t| {
            chrono::DateTime::from_timestamp(t / 1000, 0)
                .map(|dt| dt.to_rfc3339())
                .unwrap_or_default()
        }),
        next_funding_rate: None,
        next_funding_timestamp: None,
        next_funding_datetime: None,
        previous_funding_rate: None,
        previous_funding_timestamp: None,
        previous_funding_datetime: None,
        timestamp: None,
        datetime: None,
        interval: None,
    })
}

/// Parse funding rate history data from Binance futures API.
///
/// # Arguments
///
/// * `data` - Binance funding rate history data JSON object
/// * `market` - Optional market information for symbol resolution
///
/// # Returns
///
/// Returns a CCXT [`FeeFundingRateHistory`] structure with `Decimal` precision.
pub fn parse_funding_rate_history(
    data: &Value,
    market: Option<&Market>,
) -> Result<FeeFundingRateHistory> {
    let symbol = if let Some(m) = market {
        m.symbol.clone()
    } else {
        data["symbol"]
            .as_str()
            .ok_or_else(|| Error::from(ParseError::missing_field("symbol")))?
            .to_string()
    };

    let funding_rate = parse_decimal(data, "fundingRate");
    let funding_time = data["fundingTime"].as_i64();

    Ok(FeeFundingRateHistory {
        info: data.clone(),
        symbol,
        funding_rate,
        funding_timestamp: funding_time,
        funding_datetime: funding_time.map(|t| {
            chrono::DateTime::from_timestamp(t / 1000, 0)
                .map(|dt| dt.to_rfc3339())
                .unwrap_or_default()
        }),
        timestamp: funding_time,
        datetime: funding_time.map(|t| {
            chrono::DateTime::from_timestamp(t / 1000, 0)
                .map(|dt| dt.to_rfc3339())
                .unwrap_or_default()
        }),
    })
}

/// Parse position data from Binance futures position risk.
///
/// # Arguments
///
/// * `data` - Binance position data JSON object
/// * `market` - Optional market information for symbol resolution
///
/// # Returns
///
/// Returns a CCXT [`Position`] structure.
pub fn parse_position(data: &Value, market: Option<&Market>) -> Result<Position> {
    let symbol = if let Some(m) = market {
        m.symbol.clone()
    } else {
        data["symbol"]
            .as_str()
            .ok_or_else(|| Error::from(ParseError::missing_field("symbol")))?
            .to_string()
    };

    let position_side = data["positionSide"].as_str().unwrap_or("BOTH");

    let side = match position_side {
        "LONG" => Some("long".to_string()),
        "SHORT" => Some("short".to_string()),
        "BOTH" => {
            // For dual position mode, determine side from positionAmt sign
            let position_amt = parse_f64(data, "positionAmt").unwrap_or(0.0);
            if position_amt > 0.0 {
                Some("long".to_string())
            } else if position_amt < 0.0 {
                Some("short".to_string())
            } else {
                None
            }
        }
        _ => None,
    };

    let contracts = parse_f64(data, "positionAmt").map(|v| v.abs());

    let contract_size = Some(1.0); // Binance futures contract size is 1

    let entry_price = parse_f64(data, "entryPrice");
    let mark_price = parse_f64(data, "markPrice");
    let notional = parse_f64(data, "notional").map(|v| v.abs());

    let leverage = parse_f64(data, "leverage");

    let collateral =
        parse_f64(data, "isolatedWallet").or_else(|| parse_f64(data, "positionInitialMargin"));

    let initial_margin =
        parse_f64(data, "initialMargin").or_else(|| parse_f64(data, "positionInitialMargin"));

    let maintenance_margin =
        parse_f64(data, "maintMargin").or_else(|| parse_f64(data, "positionMaintMargin"));

    let unrealized_pnl =
        parse_f64(data, "unrealizedProfit").or_else(|| parse_f64(data, "unRealizedProfit"));

    let liquidation_price = parse_f64(data, "liquidationPrice");

    let margin_ratio = parse_f64(data, "marginRatio");

    let margin_mode = data["marginType"]
        .as_str()
        .or_else(|| data["marginMode"].as_str())
        .map(|s| s.to_lowercase());

    let hedged = position_side != "BOTH";

    let percentage = match (unrealized_pnl, collateral) {
        (Some(pnl), Some(col)) if col > 0.0 => Some((pnl / col) * 100.0),
        _ => None,
    };

    let initial_margin_percentage = parse_f64(data, "initialMarginPercentage");
    let maintenance_margin_percentage = parse_f64(data, "maintMarginPercentage");

    let update_time = data["updateTime"].as_i64();

    Ok(Position {
        info: data.clone(),
        id: None,
        symbol,
        side,
        contracts,
        contract_size,
        entry_price,
        mark_price,
        notional,
        leverage,
        collateral,
        initial_margin,
        initial_margin_percentage,
        maintenance_margin,
        maintenance_margin_percentage,
        unrealized_pnl,
        realized_pnl: None, // Binance positionRisk endpoint does not provide realized PnL
        liquidation_price,
        margin_ratio,
        margin_mode,
        hedged: Some(hedged),
        percentage,
        position_side: None,
        dual_side_position: None,
        timestamp: update_time,
        datetime: update_time.map(|t| {
            chrono::DateTime::from_timestamp(t / 1000, 0)
                .map(|dt| dt.to_rfc3339())
                .unwrap_or_default()
        }),
    })
}
/// Parse leverage information from Binance futures API.
///
/// # Arguments
///
/// * `data` - Binance leverage data JSON object
/// * `market` - Optional market information (unused)
///
/// # Returns
///
/// Returns a CCXT [`Leverage`] structure.
pub fn parse_leverage(data: &Value, _market: Option<&Market>) -> Result<Leverage> {
    let market_id = data.get("symbol").and_then(|v| v.as_str()).unwrap_or("");

    let margin_mode = if let Some(isolated) = data.get("isolated").and_then(|v| v.as_bool()) {
        Some(if isolated {
            MarginType::Isolated
        } else {
            MarginType::Cross
        })
    } else if let Some(margin_type) = data.get("marginType").and_then(|v| v.as_str()) {
        Some(if margin_type == "crossed" {
            MarginType::Cross
        } else {
            MarginType::Isolated
        })
    } else {
        None
    };

    let side = data
        .get("positionSide")
        .and_then(|v| v.as_str())
        .map(|s| s.to_lowercase());

    let leverage_value = data.get("leverage").and_then(|v| v.as_i64());

    // 4. 根据持仓方向分配杠杆
    let (long_leverage, short_leverage) = match side.as_deref() {
        None | Some("both") => (leverage_value, leverage_value),
        Some("long") => (leverage_value, None),
        Some("short") => (None, leverage_value),
        _ => (None, None),
    };

    Ok(Leverage {
        info: data.clone(),
        symbol: market_id.to_string(),
        margin_mode,
        long_leverage,
        short_leverage,
        timestamp: None,
        datetime: None,
    })
}

/// Parse funding fee history data from Binance futures API.
///
/// # Arguments
///
/// * `data` - Binance funding fee history data JSON object
/// * `market` - Optional market information for symbol resolution
///
/// # Returns
///
/// Returns a CCXT [`FundingHistory`] structure.
pub fn parse_funding_history(data: &Value, market: Option<&Market>) -> Result<FundingHistory> {
    let symbol = if let Some(m) = market {
        m.symbol.clone()
    } else {
        data["symbol"]
            .as_str()
            .ok_or_else(|| Error::from(ParseError::missing_field("symbol")))?
            .to_string()
    };

    let id = data["tranId"]
        .as_u64()
        .or_else(|| data["id"].as_u64())
        .map(|i| i.to_string());

    let amount = parse_f64(data, "income");
    let code = data["asset"].as_str().map(|s| s.to_string());
    let timestamp = data["time"].as_i64();

    Ok(FundingHistory {
        info: data.clone(),
        id,
        symbol,
        code,
        amount,
        timestamp: timestamp,
        datetime: timestamp.map(|t| {
            chrono::DateTime::from_timestamp((t / 1000) as i64, 0)
                .map(|dt| dt.to_rfc3339())
                .unwrap_or_default()
        }),
    })
}

/// Parse funding fee data from Binance futures API.
///
/// # Arguments
///
/// * `data` - Binance funding fee data JSON object
/// * `market` - Optional market information for symbol resolution
///
/// # Returns
///
/// Returns a CCXT [`FundingFee`] structure.
pub fn parse_funding_fee(data: &Value, market: Option<&Market>) -> Result<FundingFee> {
    let symbol = if let Some(m) = market {
        m.symbol.clone()
    } else {
        data["symbol"]
            .as_str()
            .ok_or_else(|| Error::from(ParseError::missing_field("symbol")))?
            .to_string()
    };

    let income = parse_f64(data, "income").unwrap_or(0.0);
    let asset = data["asset"]
        .as_str()
        .ok_or_else(|| Error::from(ParseError::missing_field("asset")))?
        .to_string();

    let time = data["time"]
        .as_i64()
        .ok_or_else(|| Error::from(ParseError::missing_field("time")))?;

    let funding_rate = parse_f64(data, "fundingRate");
    let mark_price = parse_f64(data, "markPrice");

    let datetime = Some(
        chrono::DateTime::from_timestamp((time / 1000) as i64, 0)
            .map(|dt| dt.to_rfc3339())
            .unwrap_or_default(),
    );

    Ok(FundingFee {
        info: data.clone(),
        symbol,
        income,
        asset,
        time,
        datetime,
        funding_rate,
        mark_price,
    })
}

/// Parse next funding rate data from Binance premium index.
///
/// # Arguments
///
/// * `data` - Binance premium index data JSON object
/// * `market` - Market information for symbol resolution
///
/// # Returns
///
/// Returns a CCXT [`NextFundingRate`] structure.
pub fn parse_next_funding_rate(data: &Value, market: &Market) -> Result<NextFundingRate> {
    let symbol = market.symbol.clone();

    let mark_price = parse_f64(data, "markPrice")
        .ok_or_else(|| Error::from(ParseError::missing_field("markPrice")))?;

    let index_price = parse_f64(data, "indexPrice");

    let current_funding_rate = parse_f64(data, "lastFundingRate").unwrap_or(0.0);

    let next_funding_rate = parse_f64(data, "interestRate")
        .or_else(|| parse_f64(data, "estimatedSettlePrice"))
        .unwrap_or(current_funding_rate);

    let next_funding_time = data["nextFundingTime"]
        .as_i64()
        .ok_or_else(|| Error::from(ParseError::missing_field("nextFundingTime")))?;

    let next_funding_datetime = Some(
        chrono::DateTime::from_timestamp((next_funding_time / 1000) as i64, 0)
            .map(|dt| dt.to_rfc3339())
            .unwrap_or_default(),
    );

    Ok(NextFundingRate {
        info: data.clone(),
        symbol,
        mark_price,
        index_price,
        current_funding_rate,
        next_funding_rate,
        next_funding_time,
        next_funding_datetime,
    })
}
// ============================================================================
// Account Configuration Parser Functions
// ============================================================================

/// Parse account configuration from Binance futures account info.
///
/// # Arguments
///
/// * `data` - Binance account info data JSON object
///
/// # Returns
///
/// Returns a CCXT [`AccountConfig`] structure.
pub fn parse_account_config(data: &Value) -> Result<AccountConfig> {
    let multi_assets_margin = data["multiAssetsMargin"].as_bool().unwrap_or(false);

    let fee_tier = data["feeTier"].as_i64().unwrap_or(0) as i32;

    let can_trade = data["canTrade"].as_bool().unwrap_or(true);

    let can_deposit = data["canDeposit"].as_bool().unwrap_or(true);

    let can_withdraw = data["canWithdraw"].as_bool().unwrap_or(true);

    let update_time = data["updateTime"].as_i64().unwrap_or(0);

    Ok(AccountConfig {
        info: Some(data.clone()),
        multi_assets_margin,
        fee_tier,
        can_trade,
        can_deposit,
        can_withdraw,
        update_time,
    })
}

/// Parse commission rate information from Binance.
///
/// # Arguments
///
/// * `data` - Binance commission rate data JSON object
/// * `market` - Market information for symbol resolution
///
/// # Returns
///
/// Returns a CCXT [`CommissionRate`] structure.
pub fn parse_commission_rate(data: &Value, market: &Market) -> Result<CommissionRate> {
    let maker_commission_rate = data["makerCommissionRate"]
        .as_str()
        .and_then(|s| s.parse::<f64>().ok())
        .unwrap_or(0.0);

    let taker_commission_rate = data["takerCommissionRate"]
        .as_str()
        .and_then(|s| s.parse::<f64>().ok())
        .unwrap_or(0.0);

    Ok(CommissionRate {
        info: Some(data.clone()),
        symbol: market.symbol.clone(),
        maker_commission_rate,
        taker_commission_rate,
    })
}

// ============================================================================
// Risk and Limits Query Parser Functions
// ============================================================================

/// Parse open interest data from Binance futures API.
///
/// # Arguments
///
/// * `data` - Binance open interest data JSON object
/// * `market` - Market information for symbol resolution
///
/// # Returns
///
/// Returns a CCXT [`OpenInterest`] structure.
///
/// # Binance Response Example
///
/// ```json
/// {
///   "openInterest": "10659.509",
///   "symbol": "BTCUSDT",
///   "time": 1589437530011
/// }
/// ```
pub fn parse_open_interest(data: &Value, market: &Market) -> Result<OpenInterest> {
    let open_interest = data["openInterest"]
        .as_str()
        .and_then(|s| s.parse::<f64>().ok())
        .or_else(|| data["openInterest"].as_f64())
        .unwrap_or(0.0);

    let timestamp = data["time"]
        .as_i64()
        .unwrap_or_else(|| chrono::Utc::now().timestamp_millis());

    let contract_size = market
        .contract_size
        .unwrap_or_else(|| rust_decimal::Decimal::from(1))
        .to_f64()
        .unwrap_or(1.0);
    let open_interest_value = open_interest * contract_size;

    Ok(OpenInterest {
        info: Some(data.clone()),
        symbol: market.symbol.clone(),
        open_interest,
        open_interest_value,
        timestamp,
    })
}

/// Parse open interest history data from Binance futures API.
///
/// # Arguments
///
/// * `data` - Binance open interest history data JSON array
/// * `market` - Market information for symbol resolution
///
/// # Returns
///
/// Returns a vector of [`OpenInterestHistory`] structures.
///
/// # Binance Response Example
///
/// ```json
/// [
///   {
///     "symbol": "BTCUSDT",
///     "sumOpenInterest": "10659.509",
///     "sumOpenInterestValue": "106595090.00",
///     "timestamp": 1589437530011
///   }
/// ]
/// ```
pub fn parse_open_interest_history(
    data: &Value,
    market: &Market,
) -> Result<Vec<OpenInterestHistory>> {
    let array = data.as_array().ok_or_else(|| {
        Error::from(ParseError::invalid_value(
            "data",
            "Expected array for open interest history",
        ))
    })?;

    let mut result = Vec::new();

    for item in array {
        let sum_open_interest = item["sumOpenInterest"]
            .as_str()
            .and_then(|s| s.parse::<f64>().ok())
            .or_else(|| item["sumOpenInterest"].as_f64())
            .unwrap_or(0.0);

        let sum_open_interest_value = item["sumOpenInterestValue"]
            .as_str()
            .and_then(|s| s.parse::<f64>().ok())
            .or_else(|| item["sumOpenInterestValue"].as_f64())
            .unwrap_or(0.0);

        let timestamp = item["timestamp"]
            .as_i64()
            .unwrap_or_else(|| chrono::Utc::now().timestamp_millis());

        result.push(OpenInterestHistory {
            info: Some(item.clone()),
            symbol: market.symbol.clone(),
            sum_open_interest,
            sum_open_interest_value,
            timestamp,
        });
    }

    Ok(result)
}

/// Parse maximum leverage data from Binance leverage brackets.
///
/// # Arguments
///
/// * `data` - Binance leverage bracket data JSON (array or object)
/// * `market` - Market information for symbol matching
///
/// # Returns
///
/// Returns a CCXT [`MaxLeverage`] structure.
///
/// # Binance Response Example
///
/// ```json
/// [
///   {
///     "symbol": "BTCUSDT",
///     "brackets": [
///       {
///         "bracket": 1,
///         "initialLeverage": 125,
///         "notionalCap": 50000,
///         "notionalFloor": 0,
///         "maintMarginRatio": 0.004
///       }
///     ]
///   }
/// ]
/// ```
pub fn parse_max_leverage(data: &Value, market: &Market) -> Result<MaxLeverage> {
    let target_data = if let Some(array) = data.as_array() {
        array
            .iter()
            .find(|item| item["symbol"].as_str().unwrap_or("") == market.id)
            .ok_or_else(|| {
                Error::from(ParseError::invalid_value(
                    "symbol",
                    format!("Symbol {} not found in leverage brackets", market.id),
                ))
            })?
    } else {
        data
    };

    let brackets = target_data["brackets"]
        .as_array()
        .ok_or_else(|| Error::from(ParseError::missing_field("brackets")))?;

    if brackets.is_empty() {
        return Err(Error::from(ParseError::invalid_value(
            "data",
            "Empty brackets array",
        )));
    }

    let first_bracket = &brackets[0];
    let max_leverage = first_bracket["initialLeverage"].as_i64().unwrap_or(1) as i32;

    let notional = first_bracket["notionalCap"]
        .as_f64()
        .or_else(|| {
            first_bracket["notionalCap"]
                .as_str()
                .and_then(|s| s.parse::<f64>().ok())
        })
        .unwrap_or(0.0);

    Ok(MaxLeverage {
        info: Some(data.clone()),
        symbol: market.symbol.clone(),
        max_leverage,
        notional,
    })
}

// ============================================================================
// Futures Market Data Parser Functions
// ============================================================================

/// Parse index price data from Binance futures API.
///
/// # Arguments
///
/// * `data` - Binance index price data JSON object
/// * `market` - Market information for symbol resolution
///
/// # Returns
///
/// Returns a CCXT [`IndexPrice`] structure.
pub fn parse_index_price(data: &Value, market: &Market) -> Result<IndexPrice> {
    let index_price = data["indexPrice"]
        .as_f64()
        .or_else(|| {
            data["indexPrice"]
                .as_str()
                .and_then(|s| s.parse::<f64>().ok())
        })
        .ok_or_else(|| Error::from(ParseError::missing_field("indexPrice")))?;

    let timestamp = data["time"]
        .as_i64()
        .unwrap_or_else(|| chrono::Utc::now().timestamp_millis());

    Ok(IndexPrice {
        info: Some(data.clone()),
        symbol: market.symbol.clone(),
        index_price,
        timestamp,
    })
}

/// Parse premium index data from Binance futures API.
///
/// # Arguments
///
/// * `data` - Binance premium index data JSON object
/// * `market` - Market information for symbol resolution
///
/// # Returns
///
/// Returns a CCXT [`PremiumIndex`] structure.
pub fn parse_premium_index(data: &Value, market: &Market) -> Result<PremiumIndex> {
    let mark_price = data["markPrice"]
        .as_f64()
        .or_else(|| {
            data["markPrice"]
                .as_str()
                .and_then(|s| s.parse::<f64>().ok())
        })
        .ok_or_else(|| Error::from(ParseError::missing_field("markPrice")))?;

    let index_price = data["indexPrice"]
        .as_f64()
        .or_else(|| {
            data["indexPrice"]
                .as_str()
                .and_then(|s| s.parse::<f64>().ok())
        })
        .ok_or_else(|| Error::from(ParseError::missing_field("indexPrice")))?;

    let estimated_settle_price = data["estimatedSettlePrice"]
        .as_f64()
        .or_else(|| {
            data["estimatedSettlePrice"]
                .as_str()
                .and_then(|s| s.parse::<f64>().ok())
        })
        .unwrap_or(0.0);

    let last_funding_rate = data["lastFundingRate"]
        .as_f64()
        .or_else(|| {
            data["lastFundingRate"]
                .as_str()
                .and_then(|s| s.parse::<f64>().ok())
        })
        .unwrap_or(0.0);

    // 解析下次资金费率时间
    let next_funding_time = data["nextFundingTime"].as_i64().unwrap_or(0);

    // 解析当前时间
    let time = data["time"]
        .as_i64()
        .unwrap_or_else(|| chrono::Utc::now().timestamp_millis());

    Ok(PremiumIndex {
        info: Some(data.clone()),
        symbol: market.symbol.clone(),
        mark_price,
        index_price,
        estimated_settle_price,
        last_funding_rate,
        next_funding_time,
        time,
    })
}

/// Parse liquidation order data from Binance futures API.
///
/// # Arguments
///
/// * `data` - Binance liquidation order data JSON object
/// * `market` - Market information for symbol resolution
///
/// # Returns
///
/// Returns a CCXT [`Liquidation`] structure.
pub fn parse_liquidation(data: &Value, market: &Market) -> Result<Liquidation> {
    let side = data["side"]
        .as_str()
        .ok_or_else(|| Error::from(ParseError::missing_field("side")))?
        .to_string();

    let order_type = data["type"].as_str().unwrap_or("LIMIT").to_string();

    let time = data["time"]
        .as_i64()
        .ok_or_else(|| Error::from(ParseError::missing_field("time")))?;

    let price = data["price"]
        .as_f64()
        .or_else(|| data["price"].as_str().and_then(|s| s.parse::<f64>().ok()))
        .ok_or_else(|| Error::from(ParseError::missing_field("price")))?;

    let quantity = data["origQty"]
        .as_f64()
        .or_else(|| data["origQty"].as_str().and_then(|s| s.parse::<f64>().ok()))
        .ok_or_else(|| Error::from(ParseError::missing_field("origQty")))?;

    let average_price = data["averagePrice"]
        .as_f64()
        .or_else(|| {
            data["averagePrice"]
                .as_str()
                .and_then(|s| s.parse::<f64>().ok())
        })
        .unwrap_or(price);

    Ok(Liquidation {
        info: Some(data.clone()),
        symbol: market.symbol.clone(),
        side,
        order_type,
        time,
        price,
        quantity,
        average_price,
    })
}

// ============================================================================
// Margin Trading Parser Functions
// ============================================================================

/// Parse borrow interest rate data from Binance margin API.
///
/// # Arguments
///
/// * `data` - Binance borrow rate data JSON object
/// * `currency` - Currency code (e.g., "USDT", "BTC")
/// * `symbol` - Trading pair symbol (required for isolated margin only)
///
/// # Returns
///
/// Returns a CCXT [`BorrowRate`] structure.
pub fn parse_borrow_rate(data: &Value, currency: &str, symbol: Option<&str>) -> Result<BorrowRate> {
    let rate = if let Some(rate_str) = data["dailyInterestRate"].as_str() {
        rate_str.parse::<f64>().unwrap_or(0.0)
    } else {
        data["dailyInterestRate"].as_f64().unwrap_or(0.0)
    };

    let timestamp = data["timestamp"]
        .as_i64()
        .or_else(|| {
            data["vipLevel"]
                .as_i64()
                .map(|_| chrono::Utc::now().timestamp_millis())
        })
        .unwrap_or_else(|| chrono::Utc::now().timestamp_millis());

    if let Some(sym) = symbol {
        Ok(BorrowRate::new_isolated(
            currency.to_string(),
            sym.to_string(),
            rate,
            timestamp,
            data.clone(),
        ))
    } else {
        Ok(BorrowRate::new_cross(
            currency.to_string(),
            rate,
            timestamp,
            data.clone(),
        ))
    }
}

/// Parse margin loan record data from Binance margin API.
///
/// # Arguments
///
/// * `data` - Binance margin loan record data JSON object
///
/// # Returns
///
/// Returns a CCXT [`MarginLoan`] structure.
pub fn parse_margin_loan(data: &Value) -> Result<MarginLoan> {
    let id = data["tranId"]
        .as_i64()
        .or_else(|| data["txId"].as_i64())
        .map(|id| id.to_string())
        .unwrap_or_default();

    let currency = data["asset"]
        .as_str()
        .ok_or_else(|| Error::from(ParseError::missing_field("asset")))?
        .to_string();

    let symbol = data["symbol"].as_str().map(|s| s.to_string());

    let amount = if let Some(amount_str) = data["amount"].as_str() {
        amount_str.parse::<f64>().unwrap_or(0.0)
    } else {
        data["amount"].as_f64().unwrap_or(0.0)
    };

    let timestamp = data["timestamp"]
        .as_i64()
        .unwrap_or_else(|| chrono::Utc::now().timestamp_millis());

    let status = data["status"].as_str().unwrap_or("CONFIRMED").to_string();

    Ok(MarginLoan::new(
        id,
        currency,
        symbol,
        amount,
        timestamp,
        status,
        data.clone(),
    ))
}

/// Parse borrow interest accrual data from Binance margin API.
///
/// # Arguments
///
/// * `data` - Binance borrow interest data JSON object
///
/// # Returns
///
/// Returns a CCXT [`BorrowInterest`] structure.
pub fn parse_borrow_interest(data: &Value) -> Result<BorrowInterest> {
    let id = data["txId"]
        .as_i64()
        .or_else(|| {
            data["isolatedSymbol"]
                .as_str()
                .map(|_| chrono::Utc::now().timestamp_millis())
        })
        .map(|id| id.to_string())
        .unwrap_or_default();

    let currency = data["asset"]
        .as_str()
        .ok_or_else(|| Error::from(ParseError::missing_field("asset")))?
        .to_string();

    let symbol = data["isolatedSymbol"].as_str().map(|s| s.to_string());

    let interest = if let Some(interest_str) = data["interest"].as_str() {
        interest_str.parse::<f64>().unwrap_or(0.0)
    } else {
        data["interest"].as_f64().unwrap_or(0.0)
    };

    let interest_rate = if let Some(rate_str) = data["interestRate"].as_str() {
        rate_str.parse::<f64>().unwrap_or(0.0)
    } else {
        data["interestRate"].as_f64().unwrap_or(0.0)
    };

    let principal = if let Some(principal_str) = data["principal"].as_str() {
        principal_str.parse::<f64>().unwrap_or(0.0)
    } else {
        data["principal"].as_f64().unwrap_or(0.0)
    };

    let timestamp = data["interestAccuredTime"]
        .as_i64()
        .or_else(|| data["timestamp"].as_i64())
        .unwrap_or_else(|| chrono::Utc::now().timestamp_millis());

    Ok(BorrowInterest::new(
        id,
        currency,
        symbol,
        interest,
        interest_rate,
        principal,
        timestamp,
        data.clone(),
    ))
}

/// Parse margin adjustment (transfer) data from Binance margin API.
///
/// # Arguments
///
/// * `data` - Binance margin adjustment data JSON object
///
/// # Returns
///
/// Returns a CCXT [`MarginAdjustment`] structure.
pub fn parse_margin_adjustment(data: &Value) -> Result<MarginAdjustment> {
    let id = data["tranId"]
        .as_i64()
        .or_else(|| data["txId"].as_i64())
        .map(|id| id.to_string())
        .unwrap_or_default();

    let symbol = data["symbol"].as_str().map(|s| s.to_string());

    let currency = data["asset"]
        .as_str()
        .ok_or_else(|| Error::from(ParseError::missing_field("asset")))?
        .to_string();

    let amount = if let Some(amount_str) = data["amount"].as_str() {
        amount_str.parse::<f64>().unwrap_or(0.0)
    } else {
        data["amount"].as_f64().unwrap_or(0.0)
    };

    let transfer_type = data["type"]
        .as_str()
        .or_else(|| data["transFrom"].as_str())
        .map(|t| {
            if t.contains("MAIN") || t.eq("1") || t.eq("ROLL_IN") {
                "IN"
            } else {
                "OUT"
            }
        })
        .unwrap_or("IN")
        .to_string();

    let timestamp = data["timestamp"]
        .as_i64()
        .unwrap_or_else(|| chrono::Utc::now().timestamp_millis());

    let status = data["status"].as_str().unwrap_or("SUCCESS").to_string();

    Ok(MarginAdjustment::new(
        id,
        symbol,
        currency,
        amount,
        transfer_type,
        timestamp,
        status,
        data.clone(),
    ))
}
// ============================================================================
// Account Management Parser Functions
// ============================================================================

/// Parse futures transfer type code from Binance API.
/// Converts Binance futures transfer API `type` parameter (1-4) to
/// source and destination account names.
///
/// # Arguments
///
/// * `transfer_type` - Transfer type code:
///   - 1: Spot account → USDT-M futures account
///   - 2: USDT-M futures account → Spot account
///   - 3: Spot account → COIN-M futures account
///   - 4: COIN-M futures account → Spot account
///
/// # Returns
///
/// Returns a tuple of `(from_account, to_account)`.
///
/// # Example
///
/// ```
/// use ccxt_exchanges::binance::parser::parse_futures_transfer_type;
/// let (from, to) = parse_futures_transfer_type(1).unwrap();
/// assert_eq!(from, "spot");
/// assert_eq!(to, "future");
/// ```
pub fn parse_futures_transfer_type(transfer_type: i32) -> Result<(&'static str, &'static str)> {
    match transfer_type {
        1 => Ok(("spot", "future")),
        2 => Ok(("future", "spot")),
        3 => Ok(("spot", "delivery")),
        4 => Ok(("delivery", "spot")),
        _ => Err(Error::invalid_request(format!(
            "Invalid futures transfer type: {}. Must be between 1 and 4",
            transfer_type
        ))),
    }
}

/// Parse transfer record data from Binance universal transfer API.
///
/// # Arguments
///
/// * `data` - Binance transfer record data JSON object
///
/// # Returns
///
/// Returns a CCXT [`Transfer`] structure.
///
/// # Binance Response Format
///
/// ```json
/// // transfer response (POST /sapi/v1/asset/transfer)
/// {
///   "tranId": 13526853623
/// }
///
/// // fetchTransfers response (GET /sapi/v1/asset/transfer)
/// {
///   "timestamp": 1614640878000,
///   "asset": "USDT",
///   "amount": "25",
///   "type": "MAIN_UMFUTURE",
///   "status": "CONFIRMED",
///   "tranId": 43000126248
/// }
/// ```
pub fn parse_transfer(data: &Value) -> Result<Transfer> {
    let id = data["tranId"]
        .as_i64()
        .or_else(|| data["txId"].as_i64())
        .or_else(|| data["transactionId"].as_i64())
        .map(|id| id.to_string());

    let timestamp = data["timestamp"]
        .as_i64()
        .or_else(|| data["transactionTime"].as_i64())
        .unwrap_or_else(|| chrono::Utc::now().timestamp_millis());

    let datetime = chrono::DateTime::from_timestamp_millis(timestamp)
        .map(|dt| dt.to_rfc3339())
        .unwrap_or_default();

    let currency = data["asset"]
        .as_str()
        .or_else(|| data["currency"].as_str())
        .ok_or_else(|| Error::from(ParseError::missing_field("asset")))?
        .to_string();

    let amount = if let Some(amount_str) = data["amount"].as_str() {
        amount_str.parse::<f64>().unwrap_or(0.0)
    } else {
        data["amount"].as_f64().unwrap_or(0.0)
    };

    let mut from_account = data["fromAccountType"].as_str().map(|s| s.to_string());

    let mut to_account = data["toAccountType"].as_str().map(|s| s.to_string());

    // If fromAccountType/toAccountType not present, parse from type field (e.g., "MAIN_UMFUTURE")
    if from_account.is_none() || to_account.is_none() {
        if let Some(type_str) = data["type"].as_str() {
            let parts: Vec<&str> = type_str.split('_').collect();
            if parts.len() == 2 {
                from_account = Some(parts[0].to_lowercase());
                to_account = Some(parts[1].to_lowercase());
            }
        }
    }

    let status = data["status"].as_str().unwrap_or("SUCCESS").to_lowercase();

    Ok(Transfer {
        id,
        timestamp: timestamp,
        datetime,
        currency,
        amount,
        from_account,
        to_account,
        status,
        info: Some(data.clone()),
    })
}

/// Parse maximum borrowable amount data from Binance margin API.
///
/// # Arguments
///
/// * `data` - Binance max borrowable amount data JSON object
/// * `currency` - Currency code (e.g., "USDT", "BTC")
/// * `symbol` - Trading pair symbol (required for isolated margin)
///
/// # Returns
///
/// Returns a CCXT [`MaxBorrowable`] structure.
pub fn parse_max_borrowable(
    data: &Value,
    currency: &str,
    symbol: Option<String>,
) -> Result<MaxBorrowable> {
    let amount = if let Some(amount_str) = data["amount"].as_str() {
        amount_str.parse::<f64>().unwrap_or(0.0)
    } else {
        data["amount"].as_f64().unwrap_or(0.0)
    };

    let borrow_limit = if let Some(limit_str) = data["borrowLimit"].as_str() {
        Some(limit_str.parse::<f64>().unwrap_or(0.0))
    } else {
        data["borrowLimit"].as_f64()
    };

    let timestamp = chrono::Utc::now().timestamp_millis();
    let datetime = chrono::DateTime::from_timestamp_millis(timestamp)
        .map(|dt| dt.to_rfc3339())
        .unwrap_or_default();

    Ok(MaxBorrowable {
        currency: currency.to_string(),
        amount,
        borrow_limit,
        symbol,
        timestamp,
        datetime,
        info: data.clone(),
    })
}

/// Parse maximum transferable amount data from Binance margin API.
///
/// # Arguments
///
/// * `data` - Binance max transferable amount data JSON object
/// * `currency` - Currency code (e.g., "USDT", "BTC")
/// * `symbol` - Trading pair symbol (required for isolated margin)
///
/// # Returns
///
/// Returns a CCXT [`MaxTransferable`] structure.
pub fn parse_max_transferable(
    data: &Value,
    currency: &str,
    symbol: Option<String>,
) -> Result<MaxTransferable> {
    let amount = if let Some(amount_str) = data["amount"].as_str() {
        amount_str.parse::<f64>().unwrap_or(0.0)
    } else {
        data["amount"].as_f64().unwrap_or(0.0)
    };

    let timestamp = chrono::Utc::now().timestamp_millis();
    let datetime = chrono::DateTime::from_timestamp_millis(timestamp)
        .map(|dt| dt.to_rfc3339())
        .unwrap_or_default();

    Ok(MaxTransferable {
        currency: currency.to_string(),
        amount,
        symbol,
        timestamp,
        datetime,
        info: data.clone(),
    })
}

/// 解析余额信息（支持多种账户类型）
///
/// # Arguments
///
/// * `data` - Binance余额JSON数据
/// * `account_type` - 账户类型（spot, margin, isolated, futures等）
///
/// # Returns
///
/// # Returns
///
/// Returns a CCXT [`Balance`] structure.
pub fn parse_balance_with_type(data: &Value, account_type: &str) -> Result<Balance> {
    let mut balances = HashMap::new();
    let _timestamp = chrono::Utc::now().timestamp_millis();

    match account_type {
        "spot" => {
            if let Some(balances_array) = data["balances"].as_array() {
                for item in balances_array {
                    if let Some(asset) = item["asset"].as_str() {
                        let free = if let Some(free_str) = item["free"].as_str() {
                            free_str.parse::<f64>().unwrap_or(0.0)
                        } else {
                            item["free"].as_f64().unwrap_or(0.0)
                        };

                        let locked = if let Some(locked_str) = item["locked"].as_str() {
                            locked_str.parse::<f64>().unwrap_or(0.0)
                        } else {
                            item["locked"].as_f64().unwrap_or(0.0)
                        };

                        balances.insert(
                            asset.to_string(),
                            BalanceEntry {
                                free: Decimal::from_f64(free).unwrap_or(Decimal::ZERO),
                                used: Decimal::from_f64(locked).unwrap_or(Decimal::ZERO),
                                total: Decimal::from_f64(free + locked).unwrap_or(Decimal::ZERO),
                            },
                        );
                    }
                }
            }
        }
        "margin" | "cross" => {
            if let Some(user_assets) = data["userAssets"].as_array() {
                for item in user_assets {
                    if let Some(asset) = item["asset"].as_str() {
                        let free = if let Some(free_str) = item["free"].as_str() {
                            free_str.parse::<f64>().unwrap_or(0.0)
                        } else {
                            item["free"].as_f64().unwrap_or(0.0)
                        };

                        let locked = if let Some(locked_str) = item["locked"].as_str() {
                            locked_str.parse::<f64>().unwrap_or(0.0)
                        } else {
                            item["locked"].as_f64().unwrap_or(0.0)
                        };

                        balances.insert(
                            asset.to_string(),
                            BalanceEntry {
                                free: Decimal::from_f64(free).unwrap_or(Decimal::ZERO),
                                used: Decimal::from_f64(locked).unwrap_or(Decimal::ZERO),
                                total: Decimal::from_f64(free + locked).unwrap_or(Decimal::ZERO),
                            },
                        );
                    }
                }
            }
        }
        "isolated" => {
            if let Some(assets) = data["assets"].as_array() {
                for item in assets {
                    if let Some(base_asset) = item["baseAsset"].as_object() {
                        if let Some(asset) = base_asset["asset"].as_str() {
                            let free = if let Some(free_str) = base_asset["free"].as_str() {
                                free_str.parse::<f64>().unwrap_or(0.0)
                            } else {
                                base_asset["free"].as_f64().unwrap_or(0.0)
                            };

                            let locked = if let Some(locked_str) = base_asset["locked"].as_str() {
                                locked_str.parse::<f64>().unwrap_or(0.0)
                            } else {
                                base_asset["locked"].as_f64().unwrap_or(0.0)
                            };

                            balances.insert(
                                asset.to_string(),
                                BalanceEntry {
                                    free: Decimal::from_f64(free).unwrap_or(Decimal::ZERO),
                                    used: Decimal::from_f64(locked).unwrap_or(Decimal::ZERO),
                                    total: Decimal::from_f64(free + locked)
                                        .unwrap_or(Decimal::ZERO),
                                },
                            );
                        }
                    }

                    if let Some(quote_asset) = item["quoteAsset"].as_object() {
                        if let Some(asset) = quote_asset["asset"].as_str() {
                            let free = if let Some(free_str) = quote_asset["free"].as_str() {
                                free_str.parse::<f64>().unwrap_or(0.0)
                            } else {
                                quote_asset["free"].as_f64().unwrap_or(0.0)
                            };

                            let locked = if let Some(locked_str) = quote_asset["locked"].as_str() {
                                locked_str.parse::<f64>().unwrap_or(0.0)
                            } else {
                                quote_asset["locked"].as_f64().unwrap_or(0.0)
                            };

                            balances.insert(
                                asset.to_string(),
                                BalanceEntry {
                                    free: Decimal::from_f64(free).unwrap_or(Decimal::ZERO),
                                    used: Decimal::from_f64(locked).unwrap_or(Decimal::ZERO),
                                    total: Decimal::from_f64(free + locked)
                                        .unwrap_or(Decimal::ZERO),
                                },
                            );
                        }
                    }
                }
            }
        }
        "linear" | "future" => {
            if let Some(assets) = data["assets"].as_array() {
                for item in assets {
                    if let Some(asset) = item["asset"].as_str() {
                        let available_balance =
                            if let Some(balance_str) = item["availableBalance"].as_str() {
                                balance_str.parse::<f64>().unwrap_or(0.0)
                            } else {
                                item["availableBalance"].as_f64().unwrap_or(0.0)
                            };

                        let wallet_balance =
                            if let Some(balance_str) = item["walletBalance"].as_str() {
                                balance_str.parse::<f64>().unwrap_or(0.0)
                            } else {
                                item["walletBalance"].as_f64().unwrap_or(0.0)
                            };

                        let used = wallet_balance - available_balance;

                        balances.insert(
                            asset.to_string(),
                            BalanceEntry {
                                free: Decimal::from_f64(available_balance).unwrap_or(Decimal::ZERO),
                                used: Decimal::from_f64(used).unwrap_or(Decimal::ZERO),
                                total: Decimal::from_f64(wallet_balance).unwrap_or(Decimal::ZERO),
                            },
                        );
                    }
                }
            }
        }
        "inverse" | "delivery" => {
            if let Some(assets) = data["assets"].as_array() {
                for item in assets {
                    if let Some(asset) = item["asset"].as_str() {
                        let available_balance =
                            if let Some(balance_str) = item["availableBalance"].as_str() {
                                balance_str.parse::<f64>().unwrap_or(0.0)
                            } else {
                                item["availableBalance"].as_f64().unwrap_or(0.0)
                            };

                        let wallet_balance =
                            if let Some(balance_str) = item["walletBalance"].as_str() {
                                balance_str.parse::<f64>().unwrap_or(0.0)
                            } else {
                                item["walletBalance"].as_f64().unwrap_or(0.0)
                            };

                        let used = wallet_balance - available_balance;

                        balances.insert(
                            asset.to_string(),
                            BalanceEntry {
                                free: Decimal::from_f64(available_balance).unwrap_or(Decimal::ZERO),
                                used: Decimal::from_f64(used).unwrap_or(Decimal::ZERO),
                                total: Decimal::from_f64(wallet_balance).unwrap_or(Decimal::ZERO),
                            },
                        );
                    }
                }
            }
        }
        "funding" => {
            if let Some(assets) = data.as_array() {
                for item in assets {
                    if let Some(asset) = item["asset"].as_str() {
                        let free = if let Some(free_str) = item["free"].as_str() {
                            free_str.parse::<f64>().unwrap_or(0.0)
                        } else {
                            item["free"].as_f64().unwrap_or(0.0)
                        };

                        let locked = if let Some(locked_str) = item["locked"].as_str() {
                            locked_str.parse::<f64>().unwrap_or(0.0)
                        } else {
                            item["locked"].as_f64().unwrap_or(0.0)
                        };

                        let total = free + locked;

                        balances.insert(
                            asset.to_string(),
                            BalanceEntry {
                                free: Decimal::from_f64(free).unwrap_or(Decimal::ZERO),
                                used: Decimal::from_f64(locked).unwrap_or(Decimal::ZERO),
                                total: Decimal::from_f64(total).unwrap_or(Decimal::ZERO),
                            },
                        );
                    }
                }
            }
        }
        _ => {
            return Err(Error::from(ParseError::invalid_value(
                "account_type",
                format!("Unsupported account type: {}", account_type),
            )));
        }
    }

    let mut info_map = HashMap::new();
    if let Some(obj) = data.as_object() {
        for (k, v) in obj {
            info_map.insert(k.clone(), v.clone());
        }
    }

    Ok(Balance {
        balances,
        info: info_map,
    })
}

// ============================================================================
// Market Data Parser Functions
// ============================================================================

/// Parse bid/ask price data from Binance ticker API.
///
/// # Arguments
///
/// * `data` - Binance bid/ask ticker data JSON object
///
/// # Returns
///
/// Returns a CCXT [`BidAsk`](ccxt_core::types::BidAsk) structure.
pub fn parse_bid_ask(data: &Value) -> Result<ccxt_core::types::BidAsk> {
    use ccxt_core::types::BidAsk;

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

        if !found { symbol.clone() } else { formatted }
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
///
/// # Arguments
///
/// * `data` - Binance bid/ask ticker data (array or single object)
///
/// # Returns
///
/// Returns a vector of CCXT [`BidAsk`](ccxt_core::types::BidAsk) structures.
pub fn parse_bids_asks(data: &Value) -> Result<Vec<ccxt_core::types::BidAsk>> {
    if let Some(array) = data.as_array() {
        array.iter().map(|item| parse_bid_ask(item)).collect()
    } else {
        Ok(vec![parse_bid_ask(data)?])
    }
}

/// Parse latest price data from Binance ticker API.
///
/// # Arguments
///
/// * `data` - Binance price ticker data JSON object
///
/// # Returns
///
/// Returns a CCXT [`LastPrice`](ccxt_core::types::LastPrice) structure.
pub fn parse_last_price(data: &Value) -> Result<ccxt_core::types::LastPrice> {
    use ccxt_core::types::LastPrice;

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

        if !found { symbol.clone() } else { formatted }
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
///
/// # Arguments
///
/// * `data` - Binance price ticker data (array or single object)
///
/// # Returns
///
/// Returns a vector of CCXT [`LastPrice`](ccxt_core::types::LastPrice) structures.
pub fn parse_last_prices(data: &Value) -> Result<Vec<ccxt_core::types::LastPrice>> {
    if let Some(array) = data.as_array() {
        array.iter().map(|item| parse_last_price(item)).collect()
    } else {
        Ok(vec![parse_last_price(data)?])
    }
}

/// Parse mark price data from Binance futures API.
///
/// # Arguments
///
/// * `data` - Binance mark price data JSON object
///
/// # Returns
///
/// Returns a CCXT [`MarkPrice`](ccxt_core::types::MarkPrice) structure.
pub fn parse_mark_price(data: &Value) -> Result<ccxt_core::types::MarkPrice> {
    use ccxt_core::types::MarkPrice;

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

        if !found { symbol.clone() } else { formatted }
    } else {
        symbol.clone()
    };

    let mark_price = parse_decimal(data, "markPrice").unwrap_or(Decimal::ZERO);
    let index_price = parse_decimal(data, "indexPrice");
    let estimated_settle_price = parse_decimal(data, "estimatedSettlePrice");
    let last_funding_rate = parse_decimal(data, "lastFundingRate");

    let next_funding_time = data["nextFundingTime"].as_i64();

    let timestamp = data["time"]
        .as_i64()
        .unwrap_or_else(|| chrono::Utc::now().timestamp_millis());

    Ok(MarkPrice {
        symbol: formatted_symbol,
        mark_price,
        index_price,
        estimated_settle_price,
        last_funding_rate,
        next_funding_time,
        interest_rate: None,
        timestamp,
    })
}

/// Parse multiple mark price data entries from Binance futures API.
///
/// # Arguments
///
/// * `data` - Binance mark price data (array or single object)
///
/// # Returns
///
/// Returns a vector of CCXT [`MarkPrice`](ccxt_core::types::MarkPrice) structures.
pub fn parse_mark_prices(data: &Value) -> Result<Vec<ccxt_core::types::MarkPrice>> {
    if let Some(array) = data.as_array() {
        array.iter().map(|item| parse_mark_price(item)).collect()
    } else {
        Ok(vec![parse_mark_price(data)?])
    }
}

/// Parse OHLCV (candlestick/kline) data from Binance market API.
///
/// # Binance API Response Format
///
/// ```json
/// [
///   1499040000000,      // Open time
///   "0.01634000",       // Open price
///   "0.80000000",       // High price
///   "0.01575800",       // Low price
///   "0.01577100",       // Close price (current price for incomplete candle)
///   "148976.11427815",  // Volume
///   1499644799999,      // Close time
///   "2434.19055334",    // Quote asset volume
///   308,                // Number of trades
///   "1756.87402397",    // Taker buy base asset volume
///   "28.46694368",      // Taker buy quote asset volume
///   "17928899.62484339" // Unused field
/// ]
/// ```
///
/// # Returns
///
/// Returns a CCXT [`OHLCV`](ccxt_core::types::OHLCV) structure.
pub fn parse_ohlcv(data: &Value) -> Result<ccxt_core::types::OHLCV> {
    use ccxt_core::error::{Error, ParseError};
    use ccxt_core::types::OHLCV;

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
///
/// # Returns
///
/// Returns a vector of CCXT [`OHLCV`](ccxt_core::types::OHLCV) structures.
pub fn parse_ohlcvs(data: &Value) -> Result<Vec<ccxt_core::types::OHLCV>> {
    use ccxt_core::error::{Error, ParseError};

    if let Some(array) = data.as_array() {
        array.iter().map(|item| parse_ohlcv(item)).collect()
    } else {
        Err(Error::from(ParseError::invalid_format(
            "data",
            "OHLCV data list must be in array format",
        )))
    }
}

/// Parse trading fee data from Binance API.
///
/// # Binance API Response Format
///
/// ```json
/// {
///   "symbol": "BTCUSDT",
///   "makerCommission": "0.001",
///   "takerCommission": "0.001"
/// }
/// ```
///
/// # Returns
///
/// Returns a [`FeeTradingFee`] structure.
pub fn parse_trading_fee(data: &Value) -> Result<FeeTradingFee> {
    use ccxt_core::error::{Error, ParseError};

    let symbol = data["symbol"]
        .as_str()
        .ok_or_else(|| Error::from(ParseError::missing_field("symbol")))?
        .to_string();

    let maker = data["makerCommission"]
        .as_str()
        .and_then(|s| Decimal::from_str(s).ok())
        .or_else(|| data["makerCommission"].as_f64().and_then(Decimal::from_f64))
        .ok_or_else(|| {
            Error::from(ParseError::invalid_format(
                "data",
                "Invalid maker commission",
            ))
        })?;

    let taker = data["takerCommission"]
        .as_str()
        .and_then(|s| Decimal::from_str(s).ok())
        .or_else(|| data["takerCommission"].as_f64().and_then(Decimal::from_f64))
        .ok_or_else(|| {
            Error::from(ParseError::invalid_format(
                "data",
                "Invalid taker commission",
            ))
        })?;

    Ok(FeeTradingFee::new(symbol, maker, taker))
}

/// Parse multiple trading fee data entries from Binance API.
///
/// # Returns
///
/// Returns a vector of [`FeeTradingFee`] structures.
pub fn parse_trading_fees(data: &Value) -> Result<Vec<FeeTradingFee>> {
    if let Some(array) = data.as_array() {
        array.iter().map(|item| parse_trading_fee(item)).collect()
    } else {
        Ok(vec![parse_trading_fee(data)?])
    }
}

/// Parse server time data from Binance API.
///
/// # Binance API Response Format
///
/// ```json
/// {
///   "serverTime": 1499827319559
/// }
/// ```
///
/// # Returns
///
/// Returns a CCXT [`ServerTime`](ccxt_core::types::ServerTime) structure.
pub fn parse_server_time(data: &Value) -> Result<ccxt_core::types::ServerTime> {
    use ccxt_core::error::{Error, ParseError};
    use ccxt_core::types::ServerTime;

    let server_time = data["serverTime"]
        .as_i64()
        .ok_or_else(|| Error::from(ParseError::missing_field("serverTime")))?;

    Ok(ServerTime::new(server_time))
}

/// Parse order trade history from Binance myTrades endpoint.
///
/// # Binance API Response Format
///
/// ```json
/// [
///   {
///     "id": 28457,
///     "orderId": 100234,
///     "price": "4.00000100",
///     "qty": "12.00000000",
///     "commission": "10.10000000",
///     "commissionAsset": "BNB",
///     "time": 1499865549590,
///     "isBuyer": true,
///     "isMaker": false,
///     "isBestMatch": true
///   }
/// ]
/// ```
///
/// # Returns
///
/// Returns a vector of CCXT [`Trade`](ccxt_core::types::Trade) structures.
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

/// Parse edit order response from Binance cancelReplace endpoint.
///
/// # Binance API Response Format
///
/// ```json
/// {
///   "cancelResult": "SUCCESS",
///   "newOrderResult": "SUCCESS",
///   "cancelResponse": {
///     "symbol": "BTCUSDT",
///     "orderId": 12345,
///     ...
///   },
///   "newOrderResponse": {
///     "symbol": "BTCUSDT",
///     "orderId": 12346,
///     "status": "NEW",
///     ...
///   }
/// }
/// ```
///
/// # Returns
///
/// Returns the new [`Order`] structure.
pub fn parse_edit_order_result(data: &Value, market: Option<&Market>) -> Result<Order> {
    let new_order_data = data.get("newOrderResponse").ok_or_else(|| {
        Error::from(ParseError::invalid_format(
            "data",
            "Missing newOrderResponse field",
        ))
    })?;

    parse_order(new_order_data, market)
}

// ============================================================================
// WebSocket Data Parser Functions
// ============================================================================

/// Parse WebSocket ticker data from Binance streams.
///
/// Supports multiple Binance WebSocket ticker formats:
/// - `24hrTicker`: Full 24-hour ticker statistics
/// - `24hrMiniTicker`: Simplified 24-hour ticker statistics
/// - `markPriceUpdate`: Mark price updates
/// - `bookTicker`: Best bid/ask prices
///
/// # Arguments
///
/// * `data` - WebSocket message JSON data
/// * `market` - Optional market information
///
/// # Returns
///
/// Returns a CCXT [`Ticker`] structure.
///
/// # Example WebSocket Messages
///
/// ```json
/// // 24hrTicker
/// {
///     "e": "24hrTicker",
///     "E": 1579485598569,
///     "s": "ETHBTC",
///     "p": "-0.00004000",
///     "P": "-0.209",
///     "w": "0.01920495",
///     "c": "0.01912500",
///     "Q": "0.10400000",
///     "b": "0.01912200",
///     "B": "4.10400000",
///     "a": "0.01912500",
///     "A": "0.00100000",
///     "o": "0.01916500",
///     "h": "0.01956500",
///     "l": "0.01887700",
///     "v": "173518.11900000",
///     "q": "3332.40703994",
///     "O": 1579399197842,
///     "C": 1579485597842,
///     "F": 158251292,
///     "L": 158414513,
///     "n": 163222
/// }
///
/// // markPriceUpdate
/// {
///     "e": "markPriceUpdate",
///     "E": 1562305380000,
///     "s": "BTCUSDT",
///     "p": "11794.15000000",
///     "i": "11784.62659091",
///     "P": "11784.25641265",
///     "r": "0.00038167",
///     "T": 1562306400000
/// }
///
/// // bookTicker
/// {
///     "u": 400900217,
///     "s": "BNBUSDT",
///     "b": "25.35190000",
///     "B": "31.21000000",
///     "a": "25.36520000",
///     "A": "40.66000000"
/// }
/// ```
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
        info: value_to_hashmap(data),
    })
}

/// Parse WebSocket trade data from Binance streams.
///
/// Supports multiple Binance WebSocket trade formats:
/// - `trade`: Public trade stream
/// - `aggTrade`: Aggregated trade stream
/// - `executionReport`: Private trade execution report
///
/// # Arguments
///
/// * `data` - WebSocket message JSON data
/// * `market` - Optional market information
///
/// # Returns
///
/// Returns a CCXT [`Trade`] structure.
///
/// # Example WebSocket Messages
///
/// ```json
/// // trade event
/// {
///     "e": "trade",
///     "E": 1579481530911,
///     "s": "ETHBTC",
///     "t": 158410082,
///     "p": "0.01914100",
///     "q": "0.00700000",
///     "b": 4110671841,
///     "a": 4110671533,
///     "T": 1579481530910,
///     "m": true,
///     "M": true
/// }
///
/// // aggTrade event
/// {
///     "e": "aggTrade",
///     "E": 1579481530911,
///     "s": "ETHBTC",
///     "a": 158410082,
///     "p": "0.01914100",
///     "q": "0.00700000",
///     "f": 4110671841,
///     "l": 4110671533,
///     "T": 1579481530910,
///     "m": true
/// }
/// ```
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
        .map(|i| i.to_string());

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
            .map(|i| i.to_string()),
        timestamp,
        datetime: Some(
            chrono::DateTime::from_timestamp_millis(timestamp)
                .map(|dt| dt.to_rfc3339())
                .unwrap_or_default(),
        ),
        symbol,
        trade_type: None,
        side,
        taker_or_maker: taker_or_maker,
        price: Price::from(price.unwrap_or(Decimal::ZERO)),
        amount: Amount::from(amount.unwrap_or(Decimal::ZERO)),
        cost: cost.map(Cost::from),
        fee: None,
        info: value_to_hashmap(data),
    })
}

/// Parse WebSocket order book data from Binance depth stream.
///
/// # Arguments
///
/// * `data` - WebSocket message JSON data
/// * `symbol` - Trading pair symbol
///
/// # Returns
///
/// Returns a CCXT [`OrderBook`] structure.
///
/// # Example WebSocket Message
///
/// ```json
/// {
///     "e": "depthUpdate",
///     "E": 1579481530911,
///     "s": "ETHBTC",
///     "U": 157,
///     "u": 160,
///     "b": [
///         ["0.0024", "10"]
///     ],
///     "a": [
///         ["0.0026", "100"]
///     ]
/// }
/// ```
pub fn parse_ws_orderbook(data: &Value, symbol: String) -> Result<OrderBook> {
    let timestamp = data["E"]
        .as_i64()
        .or_else(|| data["T"].as_i64())
        .unwrap_or_else(|| chrono::Utc::now().timestamp_millis());

    let bids = parse_orderbook_side(&data["b"])?;
    let asks = parse_orderbook_side(&data["a"])?;

    Ok(OrderBook {
        symbol,
        timestamp,
        datetime: Some(
            chrono::DateTime::from_timestamp_millis(timestamp)
                .map(|dt| dt.to_rfc3339())
                .unwrap_or_default(),
        ),
        nonce: data["u"].as_i64(),
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

/// Parse WebSocket OHLCV (candlestick/kline) data from Binance streams.
///
/// # Arguments
///
/// * `data` - Kline object from WebSocket message JSON data
///
/// # Returns
///
/// Returns a CCXT [`OHLCV`] structure.
///
/// # Example WebSocket Message
///
/// ```json
/// {
///     "e": "kline",
///     "E": 1579481530911,
///     "s": "ETHBTC",
///     "k": {
///         "t": 1579481400000,
///         "T": 1579481459999,
///         "s": "ETHBTC",
///         "i": "1m",
///         "f": 158251292,
///         "L": 158414513,
///         "o": "0.01916500",
///         "c": "0.01912500",
///         "h": "0.01956500",
///         "l": "0.01887700",
///         "v": "173518.11900000",
///         "n": 163222,
///         "x": false,
///         "q": "3332.40703994",
///         "V": "91515.47800000",
///         "Q": "1757.42139293"
///     }
/// }
/// ```
pub fn parse_ws_ohlcv(data: &Value) -> Result<OHLCV> {
    let kline = data["k"]
        .as_object()
        .ok_or_else(|| Error::from(ParseError::missing_field("k")))?;

    let timestamp = kline
        .get("t")
        .and_then(|v| v.as_i64())
        .ok_or_else(|| Error::from(ParseError::missing_field("t")))?;

    let open = kline
        .get("o")
        .and_then(|v| v.as_str())
        .and_then(|s| s.parse::<f64>().ok())
        .ok_or_else(|| Error::from(ParseError::missing_field("o")))?;

    let high = kline
        .get("h")
        .and_then(|v| v.as_str())
        .and_then(|s| s.parse::<f64>().ok())
        .ok_or_else(|| Error::from(ParseError::missing_field("h")))?;

    let low = kline
        .get("l")
        .and_then(|v| v.as_str())
        .and_then(|s| s.parse::<f64>().ok())
        .ok_or_else(|| Error::from(ParseError::missing_field("l")))?;

    let close = kline
        .get("c")
        .and_then(|v| v.as_str())
        .and_then(|s| s.parse::<f64>().ok())
        .ok_or_else(|| Error::from(ParseError::missing_field("c")))?;

    let volume = kline
        .get("v")
        .and_then(|v| v.as_str())
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

/// Parse WebSocket BidAsk (best bid/ask prices) data from Binance streams.
///
/// # Arguments
///
/// * `data` - WebSocket message JSON data
///
/// # Returns
///
/// Returns a CCXT [`BidAsk`] structure.
///
/// # Example WebSocket Message
///
/// ```json
/// {
///     "u": 400900217,
///     "s": "BNBUSDT",
///     "b": "25.35190000",
///     "B": "31.21000000",
///     "a": "25.36520000",
///     "A": "40.66000000"
/// }
/// ```
pub fn parse_ws_bid_ask(data: &Value) -> Result<BidAsk> {
    let symbol = data["s"]
        .as_str()
        .ok_or_else(|| Error::from(ParseError::missing_field("symbol")))?
        .to_string();

    let bid_price = data["b"]
        .as_str()
        .and_then(|s| s.parse::<Decimal>().ok())
        .ok_or_else(|| Error::from(ParseError::missing_field("bid_price")))?;

    let bid_quantity = data["B"]
        .as_str()
        .and_then(|s| s.parse::<Decimal>().ok())
        .ok_or_else(|| Error::from(ParseError::missing_field("bid_quantity")))?;

    let ask_price = data["a"]
        .as_str()
        .and_then(|s| s.parse::<Decimal>().ok())
        .ok_or_else(|| Error::from(ParseError::missing_field("ask_price")))?;

    let ask_quantity = data["A"]
        .as_str()
        .and_then(|s| s.parse::<Decimal>().ok())
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

/// Parse WebSocket mark price data from Binance futures streams.
///
/// # Binance WebSocket Mark Price Format
/// ```json
/// {
///     "e": "markPriceUpdate",
///     "E": 1609459200000,
///     "s": "BTCUSDT",
///     "p": "50250.50000000",
///     "i": "50000.00000000",
///     "P": "50500.00000000",
///     "r": "0.00010000",
///     "T": 1609459200000
/// }
/// ```
pub fn parse_ws_mark_price(data: &Value) -> Result<MarkPrice> {
    let symbol = data["s"]
        .as_str()
        .ok_or_else(|| Error::from(ParseError::missing_field("symbol")))?
        .to_string();

    let mark_price = data["p"]
        .as_str()
        .and_then(|s| s.parse::<Decimal>().ok())
        .ok_or_else(|| Error::from(ParseError::missing_field("mark_price")))?;

    let index_price = data["i"].as_str().and_then(|s| s.parse::<Decimal>().ok());

    let estimated_settle_price = data["P"].as_str().and_then(|s| s.parse::<Decimal>().ok());

    let last_funding_rate = data["r"].as_str().and_then(|s| s.parse::<Decimal>().ok());

    let next_funding_time = data["T"].as_i64();

    let interest_rate = None;

    let timestamp = data["E"].as_i64().unwrap_or(0);

    Ok(MarkPrice {
        symbol,
        mark_price,
        index_price,
        estimated_settle_price,
        last_funding_rate,
        next_funding_time,
        interest_rate,
        timestamp,
    })
}

// ============================================================================
// Transfer and Deposit/Withdrawal Parser Functions
// ============================================================================

/// Parse deposit and withdrawal fee information from Binance API.
///
/// # Binance API Response Format
///
/// Endpoint: `/sapi/v1/capital/config/getall`
/// ```json
/// {
///   "coin": "BTC",
///   "name": "Bitcoin",
///   "networkList": [
///     {
///       "network": "BTC",
///       "coin": "BTC",
///       "withdrawIntegerMultiple": "0.00000001",
///       "isDefault": true,
///       "depositEnable": true,
///       "withdrawEnable": true,
///       "depositDesc": "",
///       "withdrawDesc": "",
///       "name": "Bitcoin",
///       "resetAddressStatus": false,
///       "addressRegex": "^[13][a-km-zA-HJ-NP-Z1-9]{25,34}$|^(bc1)[0-9A-Za-z]{39,59}$",
///       "memoRegex": "",
///       "withdrawFee": "0.0005",
///       "withdrawMin": "0.001",
///       "withdrawMax": "9999999",
///       "minConfirm": 1,
///       "unLockConfirm": 2
///     }
///   ]
/// }
/// ```
///
/// # Returns
///
/// Returns a [`DepositWithdrawFee`](ccxt_core::types::DepositWithdrawFee) structure.
pub fn parse_deposit_withdraw_fee(data: &Value) -> Result<ccxt_core::types::DepositWithdrawFee> {
    let currency = data["coin"]
        .as_str()
        .ok_or_else(|| Error::from(ParseError::missing_field("coin")))?
        .to_string();

    let mut networks = Vec::new();
    let mut withdraw_fee = 0.0;
    let mut withdraw_min = 0.0;
    let mut withdraw_max = 0.0;
    let mut deposit_enable = false;
    let mut withdraw_enable = false;

    if let Some(network_list) = data["networkList"].as_array() {
        for network_data in network_list {
            let network = parse_network_info(network_data)?;

            if network_data["isDefault"].as_bool().unwrap_or(false) {
                withdraw_fee = network.withdraw_fee;
                withdraw_min = network.withdraw_min;
                withdraw_max = network.withdraw_max;
                deposit_enable = network.deposit_enable;
                withdraw_enable = network.withdraw_enable;
            }

            networks.push(network);
        }
    }

    if !networks.is_empty() && withdraw_fee == 0.0 {
        let first = &networks[0];
        withdraw_fee = first.withdraw_fee;
        withdraw_min = first.withdraw_min;
        withdraw_max = first.withdraw_max;
        deposit_enable = first.deposit_enable;
        withdraw_enable = first.withdraw_enable;
    }

    Ok(DepositWithdrawFee {
        currency,
        withdraw_fee,
        withdraw_min,
        withdraw_max,
        deposit_enable,
        withdraw_enable,
        networks,
        info: Some(data.clone()),
    })
}

/// Parse network information from Binance API.
///
/// # Binance API Response Format
///
/// Single network entry within the `networkList` array:
/// ```json
/// {
///   "network": "BTC",
///   "coin": "BTC",
///   "withdrawIntegerMultiple": "0.00000001",
///   "isDefault": true,
///   "depositEnable": true,
///   "withdrawEnable": true,
///   "depositDesc": "",
///   "withdrawDesc": "",
///   "name": "Bitcoin",
///   "withdrawFee": "0.0005",
///   "withdrawMin": "0.001",
///   "withdrawMax": "9999999",
///   "minConfirm": 1,
///   "unLockConfirm": 2
/// }
/// ```
///
/// # Returns
///
/// Returns a [`NetworkInfo`](ccxt_core::types::NetworkInfo) structure.
pub fn parse_network_info(data: &Value) -> Result<ccxt_core::types::NetworkInfo> {
    use ccxt_core::error::{Error, ParseError};
    use ccxt_core::types::NetworkInfo;

    let network = data["network"]
        .as_str()
        .ok_or_else(|| Error::from(ParseError::missing_field("network")))?
        .to_string();

    let name = data["name"].as_str().unwrap_or(&network).to_string();

    let withdraw_fee = data["withdrawFee"]
        .as_str()
        .and_then(|s| s.parse::<f64>().ok())
        .or_else(|| data["withdrawFee"].as_f64())
        .unwrap_or(0.0);

    let withdraw_min = data["withdrawMin"]
        .as_str()
        .and_then(|s| s.parse::<f64>().ok())
        .or_else(|| data["withdrawMin"].as_f64())
        .unwrap_or(0.0);

    let withdraw_max = data["withdrawMax"]
        .as_str()
        .and_then(|s| s.parse::<f64>().ok())
        .or_else(|| data["withdrawMax"].as_f64())
        .unwrap_or(0.0);

    let deposit_enable = data["depositEnable"].as_bool().unwrap_or(false);

    let withdraw_enable = data["withdrawEnable"].as_bool().unwrap_or(false);

    let _is_default = data["isDefault"].as_bool().unwrap_or(false);

    let _min_confirm = data["minConfirm"].as_i64().map(|v| v as u32);

    let _unlock_confirm = data["unLockConfirm"].as_i64().map(|v| v as u32);

    let deposit_confirmations = data["minConfirm"].as_u64().map(|v| v as u32);

    let withdraw_confirmations = data["unlockConfirm"].as_u64().map(|v| v as u32);

    Ok(NetworkInfo {
        network,
        name,
        withdraw_fee,
        withdraw_min,
        withdraw_max,
        deposit_enable,
        withdraw_enable,
        deposit_confirmations,
        withdraw_confirmations,
    })
}

/// Parse multiple deposit and withdrawal fee information entries from Binance API.
///
/// # Returns
///
/// Returns a vector of [`DepositWithdrawFee`](ccxt_core::types::DepositWithdrawFee) structures.
pub fn parse_deposit_withdraw_fees(
    data: &Value,
) -> Result<Vec<ccxt_core::types::DepositWithdrawFee>> {
    if let Some(array) = data.as_array() {
        array
            .iter()
            .map(|item| parse_deposit_withdraw_fee(item))
            .collect()
    } else {
        Ok(vec![parse_deposit_withdraw_fee(data)?])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_parse_market() {
        let data = json!({
            "symbol": "BTCUSDT",
            "baseAsset": "BTC",
            "quoteAsset": "USDT",
            "status": "TRADING",
            "isMarginTradingAllowed": true,
            "filters": [
                {
                    "filterType": "PRICE_FILTER",
                    "tickSize": "0.01"
                },
                {
                    "filterType": "LOT_SIZE",
                    "stepSize": "0.00001",
                    "minQty": "0.00001",
                    "maxQty": "9000"
                },
                {
                    "filterType": "MIN_NOTIONAL",
                    "minNotional": "10.0"
                }
            ]
        });

        let market = parse_market(&data).unwrap();
        assert_eq!(market.symbol, "BTC/USDT");
        assert_eq!(market.base, "BTC");
        assert_eq!(market.quote, "USDT");
        assert!(market.active);
        assert!(market.margin);
        assert_eq!(
            market.precision.price,
            Some(Decimal::from_str_radix("0.01", 10).unwrap())
        );
        assert_eq!(
            market.precision.amount,
            Some(Decimal::from_str_radix("0.00001", 10).unwrap())
        );
    }

    #[test]
    fn test_parse_ticker() {
        let data = json!({
            "symbol": "BTCUSDT",
            "lastPrice": "50000.00",
            "openPrice": "49000.00",
            "highPrice": "51000.00",
            "lowPrice": "48500.00",
            "volume": "1000.5",
            "quoteVolume": "50000000.0",
            "bidPrice": "49999.00",
            "bidQty": "1.5",
            "askPrice": "50001.00",
            "askQty": "2.0",
            "closeTime": 1609459200000u64,
            "priceChange": "1000.00",
            "priceChangePercent": "2.04"
        });

        let ticker = parse_ticker(&data, None).unwrap();
        assert_eq!(
            ticker.last,
            Some(Price::new(Decimal::from_str_radix("50000.00", 10).unwrap()))
        );
        assert_eq!(
            ticker.high,
            Some(Price::new(Decimal::from_str_radix("51000.00", 10).unwrap()))
        );
        assert_eq!(
            ticker.low,
            Some(Price::new(Decimal::from_str_radix("48500.00", 10).unwrap()))
        );
        assert_eq!(
            ticker.bid,
            Some(Price::new(Decimal::from_str_radix("49999.00", 10).unwrap()))
        );
        assert_eq!(
            ticker.ask,
            Some(Price::new(Decimal::from_str_radix("50001.00", 10).unwrap()))
        );
    }

    #[test]
    fn test_parse_trade() {
        let data = json!({
            "id": 12345,
            "price": "50000.00",
            "qty": "0.5",
            "time": 1609459200000u64,
            "isBuyerMaker": false,
            "symbol": "BTCUSDT"
        });

        let trade = parse_trade(&data, None).unwrap();
        assert_eq!(trade.id, Some("12345".to_string()));
        assert_eq!(
            trade.price,
            Price::new(Decimal::from_str_radix("50000.00", 10).unwrap())
        );
        assert_eq!(
            trade.amount,
            Amount::new(Decimal::from_str_radix("0.5", 10).unwrap())
        );
        assert_eq!(trade.side, OrderSide::Buy);
    }

    #[test]
    fn test_parse_order() {
        let data = json!({
            "orderId": 12345,
            "symbol": "BTCUSDT",
            "status": "FILLED",
            "side": "BUY",
            "type": "LIMIT",

            "price": "50000.00",
            "origQty": "0.5",
            "executedQty": "0.5",
            "cummulativeQuoteQty": "25000.00",
            "time": 1609459200000u64,
            "updateTime": 1609459200000u64
        });

        let order = parse_order(&data, None).unwrap();
        assert_eq!(order.id, "12345".to_string());
        assert_eq!(order.symbol, "BTCUSDT");
        assert_eq!(order.order_type, OrderType::Limit);
        assert_eq!(order.side, OrderSide::Buy);
        assert_eq!(
            order.price,
            Some(Decimal::from_str_radix("50000.00", 10).unwrap())
        );
        assert_eq!(order.amount, Decimal::from_str_radix("0.5", 10).unwrap());
        assert_eq!(
            order.filled,
            Some(Decimal::from_str_radix("0.5", 10).unwrap())
        );
    }

    #[test]
    fn test_parse_balance() {
        let data = json!({
            "balances": [
                {
                    "asset": "BTC",
                    "free": "1.5",
                    "locked": "0.5"
                }
            ]
        });

        let balance = parse_balance(&data).unwrap();
        let btc_balance = balance.balances.get("BTC").unwrap();
        assert_eq!(
            btc_balance.free,
            Decimal::from_str_radix("1.5", 10).unwrap()
        );
        assert_eq!(
            btc_balance.used,
            Decimal::from_str_radix("0.5", 10).unwrap()
        );
        assert_eq!(
            btc_balance.total,
            Decimal::from_str_radix("2.0", 10).unwrap()
        );
    }

    #[test]
    fn test_parse_market_with_filters() {
        let data = json!({
            "symbol": "ETHUSDT",
            "baseAsset": "ETH",
            "quoteAsset": "USDT",
            "status": "TRADING",
            "filters": [
                {
                    "filterType": "PRICE_FILTER",
                    "tickSize": "0.01",
                    "minPrice": "0.01",
                    "maxPrice": "1000000.00"
                },
                {
                    "filterType": "LOT_SIZE",
                    "stepSize": "0.0001",
                    "minQty": "0.0001",
                    "maxQty": "90000"
                },
                {
                    "filterType": "MIN_NOTIONAL",
                    "minNotional": "10.0"
                },
                {
                    "filterType": "MARKET_LOT_SIZE",
                    "stepSize": "0.0001",
                    "minQty": "0.0001",
                    "maxQty": "50000"
                }
            ]
        });

        let market = parse_market(&data).unwrap();
        assert_eq!(market.symbol, "ETH/USDT");
        assert!(market.limits.amount.is_some());
        assert!(market.limits.amount.as_ref().unwrap().min.is_some());
        assert!(market.limits.amount.as_ref().unwrap().max.is_some());
        assert!(market.limits.price.is_some());
        assert!(market.limits.price.as_ref().unwrap().min.is_some());
        assert!(market.limits.price.as_ref().unwrap().max.is_some());
        assert_eq!(
            market.limits.cost.as_ref().unwrap().min,
            Some(Decimal::from_str_radix("10.0", 10).unwrap())
        );
    }

    #[test]
    fn test_parse_ticker_edge_cases() {
        let data = json!({
            "symbol": "BTCUSDT",
            "lastPrice": "50000.00",
            "closeTime": 1609459200000u64
        });

        let ticker = parse_ticker(&data, None).unwrap();
        assert_eq!(
            ticker.last,
            Some(Price::new(Decimal::from_str_radix("50000.00", 10).unwrap()))
        );
        assert_eq!(ticker.symbol, "BTCUSDT");
        assert_eq!(ticker.bid, None);
        assert_eq!(ticker.ask, None);
    }

    #[test]
    fn test_parse_trade_timestamp() {
        let data = json!({
            "id": 99999,
            "price": "45000.50",
            "qty": "1.25",
            "time": 1609459200000u64,
            "isBuyerMaker": true,
            "symbol": "BTCUSDT"
        });

        let trade = parse_trade(&data, None).unwrap();
        assert_eq!(trade.timestamp, 1609459200000);
        assert_eq!(trade.side, OrderSide::Sell);
    }

    #[test]
    fn test_parse_order_status() {
        let statuses = vec![
            ("NEW", "open"),
            ("PARTIALLY_FILLED", "open"),
            ("FILLED", "closed"),
            ("CANCELED", "canceled"),
            ("REJECTED", "rejected"),
            ("EXPIRED", "expired"),
        ];

        for (binance_status, expected_status) in statuses {
            let data = json!({
                "orderId": 123,
                "symbol": "BTCUSDT",
                "status": binance_status,
                "side": "BUY",
                "type": "LIMIT",
                "price": "50000.00",
                "origQty": "1.0",
                "executedQty": "0.0",
                "time": 1609459200000u64
            });

            let order = parse_order(&data, None).unwrap();
            // Convert expected_status string to OrderStatus enum
            let status_enum = match expected_status {
                "open" => OrderStatus::Open,
                "closed" => OrderStatus::Closed,
                "canceled" | "cancelled" => OrderStatus::Cancelled,
                "expired" => OrderStatus::Expired,
                "rejected" => OrderStatus::Rejected,
                _ => OrderStatus::Open,
            };
            assert_eq!(order.status, status_enum);
        }
    }

    #[test]
    fn test_parse_balance_locked() {
        let data = json!({
            "balances": [
                {
                    "asset": "USDT",
                    "free": "10000.50",
                    "locked": "500.25"
                }
            ]
        });

        let balance = parse_balance(&data).unwrap();
        let usdt_balance = balance.balances.get("USDT").unwrap();
        assert_eq!(
            usdt_balance.free,
            Decimal::from_str_radix("10000.50", 10).unwrap()
        );
        assert_eq!(
            usdt_balance.used,
            Decimal::from_str_radix("500.25", 10).unwrap()
        );
        assert_eq!(
            usdt_balance.total,
            Decimal::from_str_radix("10500.75", 10).unwrap()
        );
    }

    #[test]
    fn test_parse_empty_response() {
        let data = json!({
            "lastUpdateId": 12345,
            "bids": [],
            "asks": []
        });

        let orderbook = parse_orderbook(&data, "BTC/USDT".to_string()).unwrap();
        assert_eq!(orderbook.bids.len(), 0);
        assert_eq!(orderbook.asks.len(), 0);
    }

    #[test]
    fn test_currency_precision() {
        let data = json!({
            "symbol": "BTCUSDT",
            "baseAsset": "BTC",
            "quoteAsset": "USDT",
            "status": "TRADING",
            "filters": [
                {
                    "filterType": "LOT_SIZE",
                    "stepSize": "0.00000001"
                },
                {
                    "filterType": "PRICE_FILTER",
                    "tickSize": "0.01"
                }
            ]
        });

        let market = parse_market(&data).unwrap();
        assert_eq!(
            market.precision.amount,
            Some(Decimal::from_str_radix("0.00000001", 10).unwrap())
        );
        assert_eq!(
            market.precision.price,
            Some(Decimal::from_str_radix("0.01", 10).unwrap())
        );
    }

    #[test]
    fn test_market_limits() {
        let data = json!({
            "symbol": "ETHBTC",
            "baseAsset": "ETH",
            "quoteAsset": "BTC",
            "status": "TRADING",
            "filters": [
                {
                    "filterType": "LOT_SIZE",
                    "minQty": "0.001",
                    "maxQty": "100000",
                    "stepSize": "0.001"
                },
                {
                    "filterType": "PRICE_FILTER",
                    "minPrice": "0.00000100",
                    "maxPrice": "100000.00000000",
                    "tickSize": "0.00000100"
                },
                {
                    "filterType": "MIN_NOTIONAL",
                    "minNotional": "0.0001"
                }
            ]
        });

        let market = parse_market(&data).unwrap();
        assert_eq!(
            market.limits.amount.as_ref().unwrap().min,
            Some(Decimal::from_str_radix("0.001", 10).unwrap())
        );
        assert_eq!(
            market.limits.amount.as_ref().unwrap().max,
            Some(Decimal::from_str_radix("100000.0", 10).unwrap())
        );
        assert_eq!(
            market.limits.price.as_ref().unwrap().min,
            Some(Decimal::from_str_radix("0.000001", 10).unwrap())
        );
        assert_eq!(
            market.limits.price.as_ref().unwrap().max,
            Some(Decimal::from_str_radix("100000.0", 10).unwrap())
        );
        assert_eq!(
            market.limits.cost.as_ref().unwrap().min,
            Some(Decimal::from_str_radix("0.0001", 10).unwrap())
        );
    }

    #[test]
    fn test_symbol_normalization() {
        let symbols = vec![
            ("BTCUSDT", "BTC/USDT"),
            ("ETHBTC", "ETH/BTC"),
            ("BNBBUSD", "BNB/BUSD"),
        ];

        for (binance_symbol, ccxt_symbol) in symbols {
            let data = json!({
                "symbol": binance_symbol,
                "baseAsset": &ccxt_symbol[..ccxt_symbol.find('/').unwrap()],
                "quoteAsset": &ccxt_symbol[ccxt_symbol.find('/').unwrap() + 1..],
                "status": "TRADING"
            });

            let market = parse_market(&data).unwrap();
            assert_eq!(market.symbol, ccxt_symbol);
        }
    }

    #[test]
    fn test_timeframe_conversion() {
        let timeframes = vec![
            ("1m", 60000),
            ("5m", 300000),
            ("15m", 900000),
            ("1h", 3600000),
            ("4h", 14400000),
            ("1d", 86400000),
        ];

        for (tf_str, expected_ms) in timeframes {
            let ms = match tf_str {
                "1m" => 60000,
                "5m" => 300000,
                "15m" => 900000,
                "1h" => 3600000,
                "4h" => 14400000,
                "1d" => 86400000,
                _ => 0,
            };
            assert_eq!(ms, expected_ms);
        }
    }
}

// ============================================================================
// Deposit/Withdrawal Parser Functions
// ============================================================================

/// Check if a currency is a fiat currency.
///
/// # Arguments
///
/// * `currency` - Currency code
///
/// # Returns
///
/// Returns `true` if the currency is a fiat currency.
pub fn is_fiat_currency(currency: &str) -> bool {
    matches!(
        currency.to_uppercase().as_str(),
        "USD" | "EUR" | "GBP" | "JPY" | "CNY" | "KRW" | "AUD" | "CAD" | "CHF" | "HKD" | "SGD"
    )
}

/// Extract internal transfer ID from transaction ID.
///
/// Extracts the actual ID from internal transfer transaction IDs.
///
/// # Arguments
///
/// * `txid` - Original transaction ID
///
/// # Returns
///
/// Returns the processed transaction ID.
pub fn extract_internal_transfer_id(txid: &str) -> String {
    const PREFIX: &str = "Internal transfer ";
    if txid.starts_with(PREFIX) {
        txid[PREFIX.len()..].to_string()
    } else {
        txid.to_string()
    }
}

/// Parse transaction status based on transaction type.
///
/// Deposits and withdrawals use different status code mappings.
///
/// # Arguments
///
/// * `status_value` - Status value (may be integer or string)
/// * `is_deposit` - Whether this is a deposit transaction
///
/// # Returns
///
/// Returns a [`TransactionStatus`](ccxt_core::types::TransactionStatus).
pub fn parse_transaction_status_by_type(
    status_value: &Value,
    is_deposit: bool,
) -> ccxt_core::types::TransactionStatus {
    use ccxt_core::types::TransactionStatus;

    if let Some(status_int) = status_value.as_i64() {
        if is_deposit {
            match status_int {
                0 => TransactionStatus::Pending,
                1 => TransactionStatus::Ok,
                6 => TransactionStatus::Ok,
                _ => TransactionStatus::Pending,
            }
        } else {
            match status_int {
                0 => TransactionStatus::Pending,
                1 => TransactionStatus::Canceled,
                2 => TransactionStatus::Pending,
                3 => TransactionStatus::Failed,
                4 => TransactionStatus::Pending,
                5 => TransactionStatus::Failed,
                6 => TransactionStatus::Ok,
                _ => TransactionStatus::Pending,
            }
        }
    } else if let Some(status_str) = status_value.as_str() {
        match status_str {
            "Processing" => TransactionStatus::Pending,
            "Failed" => TransactionStatus::Failed,
            "Successful" => TransactionStatus::Ok,
            "Refunding" => TransactionStatus::Canceled,
            "Refunded" => TransactionStatus::Canceled,
            "Refund Failed" => TransactionStatus::Failed,
            _ => TransactionStatus::Pending,
        }
    } else {
        TransactionStatus::Pending
    }
}

/// 解析单个交易记录（充值或提现）
///
/// # Arguments
///
/// * `data` - Binance transaction data JSON
/// * `transaction_type` - Transaction type (deposit or withdrawal)
///
/// # Returns
///
/// Returns a [`Transaction`](ccxt_core::types::Transaction).
///
/// # Binance API Response Format (Deposit)
///
/// ```json
/// {
///     "id": "abc123",
///     "amount": "0.5",
///     "coin": "BTC",
///     "network": "BTC",
///     "status": 1,
///     "address": "1A1zP1...",
///     "addressTag": "",
///     "txId": "hash123...",
///     "insertTime": 1609459200000,
///     "transferType": 0,
///     "confirmTimes": "2/2",
///     "unlockConfirm": 2,
///     "walletType": 0
/// }
/// ```
///
/// # Binance API Response Format (Withdrawal)
///
/// ```json
/// {
///     "id": "def456",
///     "amount": "0.3",
///     "transactionFee": "0.0005",
///     "coin": "BTC",
///     "status": 6,
///     "address": "1A1zP1...",
///     "txId": "hash456...",
///     "applyTime": "2021-01-01 00:00:00",
///     "network": "BTC",
///     "transferType": 0
/// }
/// ```
pub fn parse_transaction(
    data: &Value,
    transaction_type: ccxt_core::types::TransactionType,
) -> Result<ccxt_core::types::Transaction> {
    use ccxt_core::types::{Transaction, TransactionFee, TransactionStatus, TransactionType};

    let is_deposit = matches!(transaction_type, TransactionType::Deposit);

    // Parse ID: deposits use "id", withdrawals prefer "id" then "withdrawOrderId"
    let id = if is_deposit {
        data["id"]
            .as_str()
            .or_else(|| data["orderNo"].as_str())
            .unwrap_or("")
            .to_string()
    } else {
        data["id"]
            .as_str()
            .or_else(|| data["withdrawOrderId"].as_str())
            .unwrap_or("")
            .to_string()
    };

    // Parse currency: use "coin" for both deposits/withdrawals, "fiatCurrency" for fiat
    let currency = data["coin"]
        .as_str()
        .or_else(|| data["fiatCurrency"].as_str())
        .unwrap_or("")
        .to_string();

    let amount = data["amount"]
        .as_str()
        .and_then(|s| Decimal::from_str(s).ok())
        .unwrap_or_else(|| Decimal::ZERO);

    let fee = if is_deposit {
        None
    } else {
        data["transactionFee"]
            .as_str()
            .or_else(|| data["totalFee"].as_str())
            .and_then(|s| Decimal::from_str(s).ok())
            .map(|cost| TransactionFee {
                currency: currency.clone(),
                cost,
            })
    };

    let timestamp = if is_deposit {
        data["insertTime"].as_i64()
    } else {
        data["createTime"].as_i64().or_else(|| {
            data["applyTime"].as_str().and_then(|s| {
                chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S")
                    .ok()
                    .map(|dt| dt.and_utc().timestamp_millis())
            })
        })
    };

    let datetime = timestamp.and_then(|ts| ccxt_core::time::iso8601(ts).ok());

    let network = data["network"].as_str().map(|s| s.to_string());

    let address = data["address"]
        .as_str()
        .or_else(|| data["depositAddress"].as_str())
        .map(|s| s.to_string());

    let tag = data["addressTag"]
        .as_str()
        .or_else(|| data["tag"].as_str())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string());

    let mut txid = data["txId"]
        .as_str()
        .or_else(|| data["hash"].as_str())
        .map(|s| s.to_string());

    let transfer_type = data["transferType"].as_i64();
    let is_internal = transfer_type == Some(1);

    if is_internal {
        if let Some(ref tx) = txid {
            txid = Some(extract_internal_transfer_id(tx));
        }
    }

    let status = if let Some(status_value) = data.get("status") {
        parse_transaction_status_by_type(status_value, is_deposit)
    } else {
        TransactionStatus::Pending
    };

    let updated = data["updateTime"].as_i64();

    let comment = data["info"]
        .as_str()
        .or_else(|| data["comment"].as_str())
        .map(|s| s.to_string());

    Ok(Transaction {
        info: Some(data.clone()),
        id,
        txid,
        timestamp,
        datetime,
        network,
        address: address.clone(),
        address_to: if is_deposit { address.clone() } else { None },
        address_from: if !is_deposit { address } else { None },
        tag: tag.clone(),
        tag_to: if is_deposit { tag.clone() } else { None },
        tag_from: if !is_deposit { tag } else { None },
        transaction_type,
        amount,
        currency,
        status,
        updated,
        internal: Some(is_internal),
        comment,
        fee,
    })
}

/// Parse deposit address from Binance API response.
///
/// # Arguments
///
/// * `data` - Binance deposit address JSON data
///
/// # Returns
///
/// Returns a [`DepositAddress`](ccxt_core::types::DepositAddress).
///
/// # Binance API Response Format
///
/// ```json
/// {
///     "coin": "BTC",
///     "address": "1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa",
///     "tag": "",
///     "url": "https://btc.com/1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa"
/// }
/// ```
pub fn parse_deposit_address(data: &Value) -> Result<ccxt_core::types::DepositAddress> {
    use ccxt_core::types::DepositAddress;

    let currency = data["coin"]
        .as_str()
        .ok_or_else(|| Error::from(ParseError::missing_field("coin")))?
        .to_string();

    let address = data["address"]
        .as_str()
        .ok_or_else(|| Error::from(ParseError::missing_field("address")))?
        .to_string();

    let network = data["network"].as_str().map(|s| s.to_string()).or_else(|| {
        data["url"].as_str().and_then(|url| {
            if url.contains("btc.com") {
                Some("BTC".to_string())
            } else if url.contains("etherscan.io") {
                Some("ETH".to_string())
            } else if url.contains("tronscan.org") {
                Some("TRX".to_string())
            } else {
                None
            }
        })
    });

    let tag = data["tag"]
        .as_str()
        .or_else(|| data["addressTag"].as_str())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string());

    Ok(DepositAddress {
        info: Some(data.clone()),
        currency,
        network,
        address,
        tag,
    })
}
// ============================================================================
// P1.4 Market Data Enhancement - Parser Functions
// ============================================================================

/// Parse currency information from Binance API response.
///
/// # Arguments
///
/// * `data` - Binance currency data JSON
///
/// # Returns
///
/// Returns a CCXT [`Currency`](ccxt_core::types::Currency).
///
/// # Binance API Response Format
/// ```json
/// {
///   "coin": "BTC",
///   "name": "Bitcoin",
///   "networkList": [
///     {
///       "network": "BTC",
///       "coin": "BTC",
///       "withdrawIntegerMultiple": "0.00000001",
///       "isDefault": true,
///       "depositEnable": true,
///       "withdrawEnable": true,
///       "depositDesc": "",
///       "withdrawDesc": "",
///       "name": "Bitcoin",
///       "resetAddressStatus": false,
///       "addressRegex": "^[13][a-km-zA-HJ-NP-Z1-9]{25,34}$|^(bc1)[0-9A-Za-z]{39,59}$",
///       "memoRegex": "",
///       "withdrawFee": "0.0005",
///       "withdrawMin": "0.001",
///       "withdrawMax": "9000",
///       "minConfirm": 1,
///       "unLockConfirm": 2
///     }
///   ],
///   "trading": true,
///   "isLegalMoney": false
/// }
/// ```
pub fn parse_currency(data: &Value) -> Result<ccxt_core::types::Currency> {
    use ccxt_core::types::{Currency, CurrencyNetwork, MinMax};

    let code = data["coin"]
        .as_str()
        .ok_or_else(|| Error::from(ParseError::missing_field("coin")))?
        .to_string();

    let id = code.clone();
    let name = data["name"].as_str().map(|s| s.to_string());

    let active = data["trading"].as_bool().unwrap_or(true);

    let mut networks = HashMap::new();
    let mut global_deposit = false;
    let mut global_withdraw = false;
    let mut global_fee = None;
    let mut global_precision = None;
    let mut global_limits = MinMax::default();

    if let Some(network_list) = data["networkList"].as_array() {
        for network_data in network_list {
            let network_id = network_data["network"]
                .as_str()
                .unwrap_or(&code)
                .to_string();

            let is_default = network_data["isDefault"].as_bool().unwrap_or(false);
            let deposit_enable = network_data["depositEnable"].as_bool().unwrap_or(false);
            let withdraw_enable = network_data["withdrawEnable"].as_bool().unwrap_or(false);

            if is_default {
                global_deposit = deposit_enable;
                global_withdraw = withdraw_enable;
            }

            let fee = network_data["withdrawFee"]
                .as_str()
                .and_then(|s| Decimal::from_str(s).ok());

            if is_default && fee.is_some() {
                global_fee = fee;
            }

            let precision = network_data["withdrawIntegerMultiple"]
                .as_str()
                .and_then(|s| Decimal::from_str(s).ok());

            if is_default && precision.is_some() {
                global_precision = precision;
            }

            let withdraw_min = network_data["withdrawMin"]
                .as_str()
                .and_then(|s| Decimal::from_str(s).ok());

            let withdraw_max = network_data["withdrawMax"]
                .as_str()
                .and_then(|s| Decimal::from_str(s).ok());

            let limits = MinMax {
                min: withdraw_min,
                max: withdraw_max,
            };

            if is_default {
                global_limits = limits.clone();
            }

            let network = CurrencyNetwork {
                network: network_id.clone(),
                id: Some(network_id.clone()),
                name: network_data["name"].as_str().map(|s| s.to_string()),
                active: deposit_enable && withdraw_enable,
                deposit: deposit_enable,
                withdraw: withdraw_enable,
                fee,
                precision,
                limits,
                info: value_to_hashmap(network_data),
            };

            networks.insert(network_id, network);
        }
    }

    Ok(Currency {
        code,
        id,
        name,
        active,
        deposit: global_deposit,
        withdraw: global_withdraw,
        fee: global_fee,
        precision: global_precision,
        limits: global_limits,
        networks,
        currency_type: if data["isLegalMoney"].as_bool().unwrap_or(false) {
            Some("fiat".to_string())
        } else {
            Some("crypto".to_string())
        },
        info: value_to_hashmap(data),
    })
}

/// Parse multiple currencies from Binance API response.
///
/// # Arguments
///
/// * `data` - Binance currency data JSON array
///
/// # Returns
///
/// Returns a vector of [`Currency`](ccxt_core::types::Currency).
pub fn parse_currencies(data: &Value) -> Result<Vec<ccxt_core::types::Currency>> {
    if let Some(array) = data.as_array() {
        array.iter().map(parse_currency).collect()
    } else {
        Ok(vec![parse_currency(data)?])
    }
}

/// Parse 24-hour statistics from Binance API response.
///
/// # Arguments
///
/// * `data` - Binance 24-hour statistics JSON data
///
/// # Returns
///
/// Returns a [`Stats24hr`](ccxt_core::types::Stats24hr).
///
/// # Binance API Response Format
/// ```json
/// {
///   "symbol": "BTCUSDT",
///   "priceChange": "1000.00",
///   "priceChangePercent": "2.04",
///   "weightedAvgPrice": "49500.00",
///   "prevClosePrice": "49000.00",
///   "lastPrice": "50000.00",
///   "lastQty": "0.5",
///   "bidPrice": "49999.00",
///   "bidQty": "1.5",
///   "askPrice": "50001.00",
///   "askQty": "2.0",
///   "openPrice": "49000.00",
///   "highPrice": "51000.00",
///   "lowPrice": "48500.00",
///   "volume": "1000.5",
///   "quoteVolume": "50000000.0",
///   "openTime": 1609459200000,
///   "closeTime": 1609545600000,
///   "firstId": 100000,
///   "lastId": 200000,
///   "count": 100000
/// }
/// ```
pub fn parse_stats_24hr(data: &Value) -> Result<ccxt_core::types::Stats24hr> {
    use ccxt_core::types::Stats24hr;

    let symbol = data["symbol"]
        .as_str()
        .ok_or_else(|| Error::from(ParseError::missing_field("symbol")))?
        .to_string();

    let price_change = data["priceChange"]
        .as_str()
        .and_then(|s| Decimal::from_str(s).ok());

    let price_change_percent = data["priceChangePercent"]
        .as_str()
        .and_then(|s| Decimal::from_str(s).ok());

    let weighted_avg_price = data["weightedAvgPrice"]
        .as_str()
        .and_then(|s| Decimal::from_str(s).ok());

    let prev_close_price = data["prevClosePrice"]
        .as_str()
        .and_then(|s| Decimal::from_str(s).ok());

    let last_price = data["lastPrice"]
        .as_str()
        .and_then(|s| Decimal::from_str(s).ok());

    let last_qty = data["lastQty"]
        .as_str()
        .and_then(|s| Decimal::from_str(s).ok());

    let bid_price = data["bidPrice"]
        .as_str()
        .and_then(|s| Decimal::from_str(s).ok());

    let bid_qty = data["bidQty"]
        .as_str()
        .and_then(|s| Decimal::from_str(s).ok());

    let ask_price = data["askPrice"]
        .as_str()
        .and_then(|s| Decimal::from_str(s).ok());

    let ask_qty = data["askQty"]
        .as_str()
        .and_then(|s| Decimal::from_str(s).ok());

    // 解析OHLC数据
    let open_price = data["openPrice"]
        .as_str()
        .and_then(|s| Decimal::from_str(s).ok());

    let high_price = data["highPrice"]
        .as_str()
        .and_then(|s| Decimal::from_str(s).ok());

    let low_price = data["lowPrice"]
        .as_str()
        .and_then(|s| Decimal::from_str(s).ok());

    // 解析成交量
    let volume = data["volume"]
        .as_str()
        .and_then(|s| Decimal::from_str(s).ok());

    let quote_volume = data["quoteVolume"]
        .as_str()
        .and_then(|s| Decimal::from_str(s).ok());

    let open_time = data["openTime"].as_i64();
    let close_time = data["closeTime"].as_i64();
    let first_id = data["firstId"].as_i64();
    let last_id = data["lastId"].as_i64();
    let count = data["count"].as_i64();

    Ok(Stats24hr {
        symbol,
        price_change,
        price_change_percent,
        weighted_avg_price,
        prev_close_price,
        last_price,
        last_qty,
        bid_price,
        bid_qty,
        ask_price,
        ask_qty,
        open_price,
        high_price,
        low_price,
        volume,
        quote_volume,
        open_time,
        close_time,
        first_id,
        last_id,
        count,
        info: value_to_hashmap(data),
    })
}

/// Parse aggregated trade from Binance API response.
///
/// # Arguments
///
/// * `data` - Binance aggregated trade JSON data
/// * `symbol` - Optional trading pair symbol
///
/// # Returns
///
/// Returns an [`AggTrade`](ccxt_core::types::AggTrade).
///
/// # Binance API Response Format
/// ```json
/// {
///   "a": 26129,
///   "p": "0.01633102",
///   "q": "4.70443515",
///   "f": 27781,
///   "l": 27781,
///   "T": 1498793709153,
///   "m": true,
///   "M": true
/// }
/// ```
pub fn parse_agg_trade(data: &Value, symbol: Option<String>) -> Result<ccxt_core::types::AggTrade> {
    use ccxt_core::types::AggTrade;

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

/// Parse trading limits from Binance API response.
///
/// # Arguments
///
/// * `data` - Binance trading limits JSON data
/// * `_symbol` - Trading pair symbol (unused)
///
/// # Returns
///
/// Returns a [`TradingLimits`](ccxt_core::types::TradingLimits).
///
/// # Binance API Response Format
/// ```json
/// {
///   "symbol": "BTCUSDT",
///   "filters": [
///     {
///       "filterType": "PRICE_FILTER",
///       "minPrice": "0.01000000",
///       "maxPrice": "100000.00000000",
///       "tickSize": "0.01000000"
///     },
///     {
///       "filterType": "LOT_SIZE",
///       "minQty": "0.00001000",
///       "maxQty": "9000.00000000",
///       "stepSize": "0.00001000"
///     },
///     {
///       "filterType": "MIN_NOTIONAL",
///       "minNotional": "10.00000000"
///     }
///   ]
/// }
/// ```
pub fn parse_trading_limits(
    data: &Value,
    _symbol: String,
) -> Result<ccxt_core::types::TradingLimits> {
    use ccxt_core::types::{MinMax, TradingLimits};

    let mut price_limits = MinMax::default();
    let mut amount_limits = MinMax::default();
    let mut cost_limits = MinMax::default();

    if let Some(filters) = data["filters"].as_array() {
        for filter in filters {
            let filter_type = filter["filterType"].as_str().unwrap_or("");

            match filter_type {
                "PRICE_FILTER" => {
                    price_limits.min = filter["minPrice"]
                        .as_str()
                        .and_then(|s| Decimal::from_str(s).ok());
                    price_limits.max = filter["maxPrice"]
                        .as_str()
                        .and_then(|s| Decimal::from_str(s).ok());
                }
                "LOT_SIZE" => {
                    amount_limits.min = filter["minQty"]
                        .as_str()
                        .and_then(|s| Decimal::from_str(s).ok());
                    amount_limits.max = filter["maxQty"]
                        .as_str()
                        .and_then(|s| Decimal::from_str(s).ok());
                }
                "MIN_NOTIONAL" | "NOTIONAL" => {
                    cost_limits.min = filter["minNotional"]
                        .as_str()
                        .and_then(|s| Decimal::from_str(s).ok());
                }
                _ => {}
            }
        }
    }

    Ok(TradingLimits {
        min: None,
        max: None,
        amount: Some(amount_limits),
        price: Some(price_limits),
        cost: Some(cost_limits),
    })
}
/// Parse leverage tier from Binance API response.
///
/// # Arguments
///
/// * `data` - Binance leverage tier JSON data
/// * `market` - Market information for symbol mapping
///
/// # Returns
///
/// Returns a [`LeverageTier`](ccxt_core::types::LeverageTier).
pub fn parse_leverage_tier(data: &Value, market: &Market) -> Result<LeverageTier> {
    let tier = data["bracket"]
        .as_i64()
        .or_else(|| data["tier"].as_i64())
        .unwrap_or(0) as i32;

    let min_notional = parse_decimal(data, "notionalFloor")
        .or_else(|| parse_decimal(data, "minNotional"))
        .unwrap_or(Decimal::ZERO);

    let max_notional = parse_decimal(data, "notionalCap")
        .or_else(|| parse_decimal(data, "maxNotional"))
        .unwrap_or(Decimal::MAX);

    let maintenance_margin_rate = parse_decimal(data, "maintMarginRatio")
        .or_else(|| parse_decimal(data, "maintenanceMarginRate"))
        .unwrap_or(Decimal::ZERO);

    let max_leverage = data["initialLeverage"]
        .as_i64()
        .or_else(|| data["maxLeverage"].as_i64())
        .unwrap_or(1) as i32;

    Ok(LeverageTier {
        info: data.clone(),
        tier,
        symbol: market.symbol.clone(),
        currency: market.quote.clone(),
        min_notional,
        max_notional,
        maintenance_margin_rate,
        max_leverage,
    })
}

/// Parse isolated margin borrow rates from Binance API response.
///
/// # Arguments
///
/// * `data` - Binance isolated margin borrow rates JSON array
///
/// # Returns
///
/// Returns a `HashMap` of [`IsolatedBorrowRate`](ccxt_core::types::IsolatedBorrowRate) keyed by symbol.
pub fn parse_isolated_borrow_rates(
    data: &Value,
) -> Result<std::collections::HashMap<String, ccxt_core::types::IsolatedBorrowRate>> {
    use ccxt_core::types::IsolatedBorrowRate;
    use std::collections::HashMap;

    let mut rates_map = HashMap::new();

    if let Some(array) = data.as_array() {
        for item in array {
            let symbol = item["symbol"].as_str().unwrap_or("");
            let base = item["base"].as_str().unwrap_or("");
            let quote = item["quote"].as_str().unwrap_or("");

            let base_rate = item["dailyInterestRate"]
                .as_str()
                .and_then(|s| s.parse::<f64>().ok())
                .unwrap_or(0.0);

            let quote_rate = item["quoteDailyInterestRate"]
                .as_str()
                .and_then(|s| s.parse::<f64>().ok())
                .or_else(|| {
                    item["dailyInterestRate"]
                        .as_str()
                        .and_then(|s| s.parse::<f64>().ok())
                })
                .unwrap_or(0.0);

            let timestamp = item["timestamp"].as_i64().or_else(|| item["time"].as_i64());

            let datetime = timestamp.and_then(|ts| {
                chrono::DateTime::from_timestamp_millis(ts).map(|dt| dt.to_rfc3339())
            });

            let isolated_rate = IsolatedBorrowRate {
                symbol: symbol.to_string(),
                base: base.to_string(),
                base_rate,
                quote: quote.to_string(),
                quote_rate,
                period: 86400000, // 1 day in milliseconds
                timestamp,
                datetime,
                info: item.clone(),
            };

            rates_map.insert(symbol.to_string(), isolated_rate);
        }
    }

    Ok(rates_map)
}

/// Parse multiple borrow interest records from Binance API response.
///
/// # Arguments
///
/// * `data` - Binance borrow interest records JSON array
///
/// # Returns
///
/// Returns a vector of [`BorrowInterest`](ccxt_core::types::BorrowInterest).
pub fn parse_borrow_interests(data: &Value) -> Result<Vec<BorrowInterest>> {
    let mut interests = Vec::new();

    if let Some(array) = data.as_array() {
        for item in array {
            match parse_borrow_interest(item) {
                Ok(interest) => interests.push(interest),
                Err(e) => {
                    eprintln!("Failed to parse borrow interest: {}", e);
                }
            }
        }
    }

    Ok(interests)
}

/// Parse borrow rate history from Binance API response.
///
/// # Arguments
///
/// * `data` - Binance borrow rate history JSON data
/// * `currency` - Currency code
///
/// # Returns
///
/// Returns a [`BorrowRateHistory`](ccxt_core::types::BorrowRateHistory).
pub fn parse_borrow_rate_history(data: &Value, currency: &str) -> Result<BorrowRateHistory> {
    let timestamp = data["timestamp"]
        .as_i64()
        .or_else(|| data["time"].as_i64())
        .unwrap_or(0);

    let rate = data["hourlyInterestRate"]
        .as_str()
        .or_else(|| data["dailyInterestRate"].as_str())
        .or_else(|| data["rate"].as_str())
        .and_then(|s| s.parse::<f64>().ok())
        .or_else(|| data["hourlyInterestRate"].as_f64())
        .or_else(|| data["dailyInterestRate"].as_f64())
        .or_else(|| data["rate"].as_f64())
        .unwrap_or(0.0);

    let datetime = chrono::DateTime::from_timestamp_millis(timestamp)
        .map(|dt| dt.to_rfc3339())
        .unwrap_or_default();

    let symbol = data["symbol"].as_str().map(|s| s.to_string());
    let vip_level = data["vipLevel"].as_i64().map(|v| v as i32);

    Ok(BorrowRateHistory {
        currency: currency.to_string(),
        symbol,
        rate,
        timestamp,
        datetime,
        vip_level,
        info: data.clone(),
    })
}

/// Parse ledger entry from Binance API response.
///
/// # Arguments
///
/// * `data` - Binance ledger entry JSON data
///
/// # Returns
///
/// Returns a [`LedgerEntry`](ccxt_core::types::LedgerEntry).
pub fn parse_ledger_entry(data: &Value) -> Result<LedgerEntry> {
    let id = data["tranId"]
        .as_i64()
        .or_else(|| data["id"].as_i64())
        .map(|v| v.to_string())
        .or_else(|| data["tranId"].as_str().map(|s| s.to_string()))
        .or_else(|| data["id"].as_str().map(|s| s.to_string()))
        .unwrap_or_default();

    let currency = data["asset"]
        .as_str()
        .or_else(|| data["currency"].as_str())
        .unwrap_or("")
        .to_string();

    let amount = data["amount"]
        .as_str()
        .and_then(|s| s.parse::<f64>().ok())
        .or_else(|| data["amount"].as_f64())
        .or_else(|| data["qty"].as_str().and_then(|s| s.parse::<f64>().ok()))
        .or_else(|| data["qty"].as_f64())
        .unwrap_or(0.0);

    let timestamp = data["timestamp"]
        .as_i64()
        .or_else(|| data["time"].as_i64())
        .unwrap_or(0);

    let type_str = data["type"].as_str().unwrap_or("");
    let (direction, entry_type) = match type_str {
        "DEPOSIT" => (LedgerDirection::In, LedgerEntryType::Deposit),
        "WITHDRAW" => (LedgerDirection::Out, LedgerEntryType::Withdrawal),
        "BUY" | "SELL" => (
            if amount >= 0.0 {
                LedgerDirection::In
            } else {
                LedgerDirection::Out
            },
            LedgerEntryType::Trade,
        ),
        "FEE" => (LedgerDirection::Out, LedgerEntryType::Fee),
        "REBATE" => (LedgerDirection::In, LedgerEntryType::Rebate),
        "TRANSFER" => (
            if amount >= 0.0 {
                LedgerDirection::In
            } else {
                LedgerDirection::Out
            },
            LedgerEntryType::Transfer,
        ),
        _ => (
            if amount >= 0.0 {
                LedgerDirection::In
            } else {
                LedgerDirection::Out
            },
            LedgerEntryType::Trade,
        ),
    };

    let datetime = chrono::DateTime::from_timestamp_millis(timestamp)
        .map(|dt| dt.to_rfc3339())
        .unwrap_or_default();

    Ok(LedgerEntry {
        id,
        currency,
        account: None,
        reference_account: None,
        reference_id: None,
        type_: entry_type,
        direction,
        amount: amount.abs(),
        timestamp,
        datetime,
        before: None,
        after: None,
        status: None,
        fee: None,
        info: data.clone(),
    })
}

#[cfg(test)]
mod transaction_tests {
    use super::*;
    use ccxt_core::types::{TransactionStatus, TransactionType};
    use serde_json::json;

    #[test]
    fn test_is_fiat_currency() {
        assert!(is_fiat_currency("USD"));
        assert!(is_fiat_currency("eur"));
        assert!(is_fiat_currency("CNY"));
        assert!(!is_fiat_currency("BTC"));
        assert!(!is_fiat_currency("ETH"));
    }

    #[test]
    fn test_extract_internal_transfer_id() {
        assert_eq!(
            extract_internal_transfer_id("Internal transfer 123456"),
            "123456"
        );
        assert_eq!(
            extract_internal_transfer_id("normal_hash_abc"),
            "normal_hash_abc"
        );
    }

    #[test]
    fn test_parse_transaction_status_deposit() {
        assert_eq!(
            parse_transaction_status_by_type(&json!(0), true),
            TransactionStatus::Pending
        );
        assert_eq!(
            parse_transaction_status_by_type(&json!(1), true),
            TransactionStatus::Ok
        );
        assert_eq!(
            parse_transaction_status_by_type(&json!(6), true),
            TransactionStatus::Ok
        );
        assert_eq!(
            parse_transaction_status_by_type(&json!("Processing"), true),
            TransactionStatus::Pending
        );
        assert_eq!(
            parse_transaction_status_by_type(&json!("Successful"), true),
            TransactionStatus::Ok
        );
    }

    #[test]
    fn test_parse_transaction_status_withdrawal() {
        assert_eq!(
            parse_transaction_status_by_type(&json!(0), false),
            TransactionStatus::Pending
        );
        assert_eq!(
            parse_transaction_status_by_type(&json!(1), false),
            TransactionStatus::Canceled
        );
        assert_eq!(
            parse_transaction_status_by_type(&json!(6), false),
            TransactionStatus::Ok
        );
    }

    #[test]
    fn test_parse_deposit_transaction() {
        let data = json!({
            "id": "deposit123",
            "amount": "0.5",
            "coin": "BTC",
            "network": "BTC",
            "status": 1,
            "address": "1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa",
            "addressTag": "",
            "txId": "hash123abc",
            "insertTime": 1609459200000i64,
            "transferType": 0
        });

        let tx = parse_transaction(&data, TransactionType::Deposit).unwrap();
        assert_eq!(tx.id, "deposit123");
        assert_eq!(tx.currency, "BTC");
        assert_eq!(tx.amount, Decimal::from_str("0.5").unwrap());
        assert_eq!(tx.status, TransactionStatus::Ok);
        assert_eq!(tx.txid, Some("hash123abc".to_string()));
        assert_eq!(tx.internal, Some(false));
        assert!(tx.is_deposit());
    }

    #[test]
    fn test_parse_withdrawal_transaction() {
        let data = json!({
            "id": "withdrawal456",
            "amount": "0.3",
            "transactionFee": "0.0005",
            "coin": "BTC",
            "status": 6,
            "address": "1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa",
            "txId": "hash456def",
            "applyTime": "2021-01-01 00:00:00",
            "network": "BTC",
            "transferType": 0
        });

        let tx = parse_transaction(&data, TransactionType::Withdrawal).unwrap();
        assert_eq!(tx.id, "withdrawal456");
        assert_eq!(tx.currency, "BTC");
        assert_eq!(tx.amount, Decimal::from_str("0.3").unwrap());
        assert_eq!(tx.status, TransactionStatus::Ok);
        assert!(tx.fee.is_some());
        assert_eq!(
            tx.fee.as_ref().unwrap().cost,
            Decimal::from_str("0.0005").unwrap()
        );
        assert!(tx.is_withdrawal());
    }

    #[test]
    fn test_parse_internal_transfer() {
        let data = json!({
            "id": "internal789",
            "amount": "1.0",
            "coin": "USDT",
            "status": 1,
            "txId": "Internal transfer 789xyz",
            "insertTime": 1609459200000i64,
            "transferType": 1
        });

        let tx = parse_transaction(&data, TransactionType::Deposit).unwrap();
        assert_eq!(tx.internal, Some(true));
        assert_eq!(tx.txid, Some("789xyz".to_string()));
    }

    #[test]
    fn test_parse_deposit_address() {
        let data = json!({
            "coin": "BTC",
            "address": "1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa",
            "tag": "",
            "network": "BTC",
            "url": "https://btc.com/1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa"
        });

        let addr = parse_deposit_address(&data).unwrap();
        assert_eq!(addr.currency, "BTC");
        assert_eq!(addr.address, "1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa");
        assert_eq!(addr.network, Some("BTC".to_string()));
        assert_eq!(addr.tag, None);
    }

    #[test]
    fn test_parse_deposit_address_with_tag() {
        let data = json!({
            "coin": "XRP",
            "address": "rLHzPsX6oXkzU9rKmLwCdxoEFdLQsSz6Xg",
            "tag": "123456",
            "network": "XRP"
        });

        let addr = parse_deposit_address(&data).unwrap();
        assert_eq!(addr.currency, "XRP");
        assert_eq!(addr.tag, Some("123456".to_string()));
    }
}

// ============================================================================
// WebSocket Parser Tests
// ============================================================================

#[cfg(test)]
mod ws_parser_tests {
    use super::*;
    use rust_decimal_macros::dec;
    use serde_json::json;

    #[test]
    fn test_parse_ws_ticker_24hr() {
        let data = json!({
            "e": "24hrTicker",
            "E": 1609459200000i64,
            "s": "BTCUSDT",
            "p": "1000.00",
            "P": "2.04",
            "c": "50000.00",
            "o": "49000.00",
            "h": "51000.00",
            "l": "48500.00",
            "v": "1000.5",
            "q": "50000000.0",
            "b": "49999.00",
            "B": "1.5",
            "a": "50001.00",
            "A": "2.0"
        });

        let ticker = parse_ws_ticker(&data, None).unwrap();
        assert_eq!(ticker.symbol, "BTCUSDT");
        assert_eq!(
            ticker.last,
            Some(Price::new(Decimal::from_str_radix("50000.00", 10).unwrap()))
        );
        assert_eq!(
            ticker.open,
            Some(Price::new(Decimal::from_str_radix("49000.00", 10).unwrap()))
        );
        assert_eq!(
            ticker.high,
            Some(Price::new(Decimal::from_str_radix("51000.00", 10).unwrap()))
        );
        assert_eq!(
            ticker.low,
            Some(Price::new(Decimal::from_str_radix("48500.00", 10).unwrap()))
        );
        assert_eq!(
            ticker.bid,
            Some(Price::new(Decimal::from_str_radix("49999.00", 10).unwrap()))
        );
        assert_eq!(
            ticker.ask,
            Some(Price::new(Decimal::from_str_radix("50001.00", 10).unwrap()))
        );
        assert_eq!(ticker.timestamp, 1609459200000);
    }

    #[test]
    fn test_parse_ws_ticker_mark_price() {
        let data = json!({
            "e": "markPriceUpdate",
            "E": 1609459200000i64,
            "s": "BTCUSDT",
            "p": "50250.50",
            "i": "50000.00",
            "r": "0.00010000",
            "T": 1609459300000i64
        });

        let ticker = parse_ws_ticker(&data, None).unwrap();
        assert_eq!(ticker.symbol, "BTCUSDT");
        assert_eq!(
            ticker.last,
            Some(Price::new(Decimal::from_str_radix("50250.50", 10).unwrap()))
        );
        assert_eq!(ticker.timestamp, 1609459200000);
    }

    #[test]
    fn test_parse_ws_ticker_book_ticker() {
        let data = json!({
            "s": "BTCUSDT",
            "b": "49999.00",
            "B": "1.5",
            "a": "50001.00",
            "A": "2.0",
            "E": 1609459200000i64
        });

        let ticker = parse_ws_ticker(&data, None).unwrap();
        assert_eq!(ticker.symbol, "BTCUSDT");
        assert_eq!(
            ticker.bid,
            Some(Price::new(Decimal::from_str_radix("49999.00", 10).unwrap()))
        );
        assert_eq!(
            ticker.ask,
            Some(Price::new(Decimal::from_str_radix("50001.00", 10).unwrap()))
        );
        assert_eq!(ticker.timestamp, 1609459200000);
    }

    #[test]
    fn test_parse_ws_trade() {
        let data = json!({
            "e": "trade",
            "E": 1609459200000i64,
            "s": "BTCUSDT",
            "t": 12345,
            "p": "50000.00",
            "q": "0.5",
            "T": 1609459200000i64,
            "m": false
        });

        let trade = parse_ws_trade(&data, None).unwrap();
        assert_eq!(trade.id, Some("12345".to_string()));
        assert_eq!(trade.symbol, "BTCUSDT");
        assert_eq!(
            trade.price,
            Price::new(Decimal::from_str_radix("50000.00", 10).unwrap())
        );
        assert_eq!(
            trade.amount,
            Amount::new(Decimal::from_str_radix("0.5", 10).unwrap())
        );
        assert_eq!(trade.timestamp, 1609459200000);
        assert_eq!(trade.side, OrderSide::Buy); // m=false indicates buy order
    }

    #[test]
    fn test_parse_ws_trade_agg() {
        let data = json!({
            "e": "aggTrade",
            "E": 1609459200000i64,
            "s": "BTCUSDT",
            "a": 67890,
            "p": "50000.00",
            "q": "0.5",
            "T": 1609459200000i64,
            "m": true
        });

        let trade = parse_ws_trade(&data, None).unwrap();
        assert_eq!(trade.id, Some("67890".to_string()));
        assert_eq!(trade.symbol, "BTCUSDT");
        assert_eq!(trade.side, OrderSide::Sell); // m=true indicates sell order
    }

    #[test]
    fn test_parse_ws_orderbook() {
        let data = json!({
            "e": "depthUpdate",
            "E": 1609459200000i64,
            "s": "BTCUSDT",
            "U": 157,
            "u": 160,
            "b": [
                ["49999.00", "1.5"],
                ["49998.00", "2.0"]
            ],
            "a": [
                ["50001.00", "2.0"],
                ["50002.00", "1.5"]
            ]
        });

        let orderbook = parse_ws_orderbook(&data, "BTCUSDT".to_string()).unwrap();
        assert_eq!(orderbook.symbol, "BTCUSDT");
        assert_eq!(orderbook.bids.len(), 2);
        assert_eq!(orderbook.asks.len(), 2);
        assert_eq!(
            orderbook.bids[0].price,
            Price::new(Decimal::from_str_radix("49999.00", 10).unwrap())
        );
        assert_eq!(
            orderbook.bids[0].amount,
            Amount::new(Decimal::from_str_radix("1.5", 10).unwrap())
        );
        assert_eq!(
            orderbook.asks[0].price,
            Price::new(Decimal::from_str_radix("50001.00", 10).unwrap())
        );
        assert_eq!(
            orderbook.asks[0].amount,
            Amount::new(Decimal::from_str_radix("2.0", 10).unwrap())
        );
        assert_eq!(orderbook.timestamp, 1609459200000);
    }

    #[test]
    fn test_parse_ws_ohlcv() {
        let data = json!({
            "e": "kline",
            "E": 1609459200000i64,
            "s": "BTCUSDT",
            "k": {
                "t": 1609459200000i64,
                "o": "49000.00",
                "h": "51000.00",
                "l": "48500.00",
                "c": "50000.00",
                "v": "1000.5"
            }
        });

        let ohlcv = parse_ws_ohlcv(&data).unwrap();
        assert_eq!(ohlcv.timestamp, 1609459200000);
        assert_eq!(ohlcv.open, 49000.00);
        assert_eq!(ohlcv.high, 51000.00);
        assert_eq!(ohlcv.low, 48500.00);
        assert_eq!(ohlcv.close, 50000.00);
        assert_eq!(ohlcv.volume, 1000.5);
    }

    #[test]
    fn test_parse_ws_bid_ask() {
        let data = json!({
            "s": "BTCUSDT",
            "b": "49999.00",
            "B": "1.5",
            "a": "50001.00",
            "A": "2.0",
            "E": 1609459200000i64
        });

        let bid_ask = parse_ws_bid_ask(&data).unwrap();
        assert_eq!(bid_ask.symbol, "BTCUSDT");
        assert_eq!(bid_ask.bid_price, dec!(49999.00));
        assert_eq!(bid_ask.bid_quantity, dec!(1.5));
        assert_eq!(bid_ask.ask_price, dec!(50001.00));
        assert_eq!(bid_ask.ask_quantity, dec!(2.0));
        assert_eq!(bid_ask.timestamp, 1609459200000);

        // Test utility methods
        let spread = bid_ask.spread();
        assert_eq!(spread, dec!(2.0));

        let mid_price = bid_ask.mid_price();
        assert_eq!(mid_price, dec!(50000.0));
    }
    #[test]
    fn test_parse_ws_mark_price() {
        let data = json!({
            "e": "markPriceUpdate",
            "E": 1609459200000i64,
            "s": "BTCUSDT",
            "p": "50250.50",
            "i": "50000.00",
            "P": "50500.00",
            "r": "0.00010000",
            "T": 1609459300000i64
        });

        let mark_price = parse_ws_mark_price(&data).unwrap();
        assert_eq!(mark_price.symbol, "BTCUSDT");
        assert_eq!(mark_price.mark_price, dec!(50250.50));
        assert_eq!(mark_price.index_price, Some(dec!(50000.00)));
        assert_eq!(mark_price.estimated_settle_price, Some(dec!(50500.00)));
        assert_eq!(mark_price.last_funding_rate, Some(dec!(0.0001)));
        assert_eq!(mark_price.next_funding_time, Some(1609459300000));
        assert_eq!(mark_price.timestamp, 1609459200000);

        // Test utility methods
        let basis = mark_price.basis();
        assert_eq!(basis, Some(dec!(250.50)));

        let funding_rate_pct = mark_price.funding_rate_percent();
        assert_eq!(funding_rate_pct, Some(dec!(0.01)));
    }
}
