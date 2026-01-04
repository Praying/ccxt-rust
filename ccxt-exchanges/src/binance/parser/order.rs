#![allow(dead_code)]

use super::{parse_decimal, value_to_hashmap};
use ccxt_core::{
    Result,
    error::{Error, ParseError},
    types::{
        Market, OcoOrder, OcoOrderInfo, Order, OrderReport, OrderSide, OrderStatus, OrderType,
        TimeInForce,
    },
};
use serde_json::Value;

/// Parse order data from Binance order response.
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
        Some("STOP_LOSS") => OrderType::StopLoss,
        Some("STOP_LOSS_LIMIT") => OrderType::StopLossLimit,
        Some("TAKE_PROFIT") => OrderType::TakeProfit,
        Some("TAKE_PROFIT_LIMIT" | "TAKE_PROFIT_MARKET") => OrderType::TakeProfitLimit,
        Some("STOP_MARKET" | "STOP") => OrderType::StopMarket,
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
        client_order_id: data["clientOrderId"].as_str().map(ToString::to_string),
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
        trailing_percent: super::parse_decimal_multi(data, &["trailingPercent", "callbackRate"]),
        activation_price: super::parse_decimal_multi(data, &["activationPrice", "activatePrice"]),
        callback_rate: parse_decimal(data, "callbackRate"),
        working_type: data["workingType"].as_str().map(ToString::to_string),
        fees: Some(Vec::new()),
        info: value_to_hashmap(data),
    })
}

/// Parse OCO (One-Cancels-the-Other) order data from Binance.
pub fn parse_oco_order(data: &Value) -> Result<OcoOrder> {
    let order_list_id = data["orderListId"]
        .as_i64()
        .ok_or_else(|| Error::from(ParseError::missing_field("orderListId")))?;

    let list_client_order_id = data["listClientOrderId"].as_str().map(ToString::to_string);

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

    let datetime = chrono::DateTime::from_timestamp(transaction_time / 1000, 0)
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
                client_order_id: order["clientOrderId"].as_str().map(ToString::to_string),
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
                client_order_id: report["clientOrderId"].as_str().map(ToString::to_string),
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
                stop_price: report["stopPrice"].as_str().map(ToString::to_string),
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

/// Parse edit order response from Binance cancelReplace endpoint.
pub fn parse_edit_order_result(data: &Value, market: Option<&Market>) -> Result<Order> {
    let new_order_data = data.get("newOrderResponse").ok_or_else(|| {
        Error::from(ParseError::invalid_format(
            "data",
            "Missing newOrderResponse field",
        ))
    })?;

    parse_order(new_order_data, market)
}
