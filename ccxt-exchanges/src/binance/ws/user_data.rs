//! User data stream handling (balance, orders, positions)

use ccxt_core::error::{Error, Result};
use ccxt_core::types::{
    Balance, Fee, Order, OrderSide, OrderStatus, OrderType, Position, TakerOrMaker, Trade,
};
use rust_decimal::Decimal;
use serde_json::Value;
use std::collections::HashMap;
use std::str::FromStr;

/// Handles balance update messages
pub async fn handle_balance_message(
    message: &Value,
    account_type: &str,
    balances: &tokio::sync::RwLock<HashMap<String, Balance>>,
) -> Result<()> {
    let event_type = message
        .get("e")
        .and_then(|e| e.as_str())
        .ok_or_else(|| Error::invalid_request("Missing event type in balance message"))?;

    let mut balances_map = balances.write().await;
    let balance = balances_map
        .entry(account_type.to_string())
        .or_insert_with(Balance::new);

    match event_type {
        "balanceUpdate" => {
            let asset = message
                .get("a")
                .and_then(|a| a.as_str())
                .ok_or_else(|| Error::invalid_request("Missing asset in balanceUpdate"))?;

            let delta_str = message
                .get("d")
                .and_then(|d| d.as_str())
                .ok_or_else(|| Error::invalid_request("Missing delta in balanceUpdate"))?;

            let delta = Decimal::from_str(delta_str)
                .map_err(|e| Error::invalid_request(format!("Invalid delta value: {}", e)))?;

            balance.apply_delta(asset.to_string(), delta);
        }

        "outboundAccountPosition" => {
            if let Some(balances_array) = message.get("B").and_then(|b| b.as_array()) {
                for balance_item in balances_array {
                    let asset = balance_item
                        .get("a")
                        .and_then(|a| a.as_str())
                        .ok_or_else(|| Error::invalid_request("Missing asset in balance item"))?;

                    let free_str = balance_item
                        .get("f")
                        .and_then(|f| f.as_str())
                        .ok_or_else(|| Error::invalid_request("Missing free balance"))?;

                    let locked_str = balance_item
                        .get("l")
                        .and_then(|l| l.as_str())
                        .ok_or_else(|| Error::invalid_request("Missing locked balance"))?;

                    let free = Decimal::from_str(free_str).map_err(|e| {
                        Error::invalid_request(format!("Invalid free value: {}", e))
                    })?;

                    let locked = Decimal::from_str(locked_str).map_err(|e| {
                        Error::invalid_request(format!("Invalid locked value: {}", e))
                    })?;

                    balance.update_balance(asset.to_string(), free, locked);
                }
            }
        }

        "ACCOUNT_UPDATE" => {
            if let Some(account_data) = message.get("a") {
                if let Some(balances_array) = account_data.get("B").and_then(|b| b.as_array()) {
                    for balance_item in balances_array {
                        let asset =
                            balance_item
                                .get("a")
                                .and_then(|a| a.as_str())
                                .ok_or_else(|| {
                                    Error::invalid_request("Missing asset in balance item")
                                })?;

                        let wallet_balance_str = balance_item
                            .get("wb")
                            .and_then(|wb| wb.as_str())
                            .ok_or_else(|| Error::invalid_request("Missing wallet balance"))?;

                        let wallet_balance =
                            Decimal::from_str(wallet_balance_str).map_err(|e| {
                                Error::invalid_request(format!("Invalid wallet balance: {}", e))
                            })?;

                        let cross_wallet = balance_item
                            .get("cw")
                            .and_then(|cw| cw.as_str())
                            .and_then(|s| Decimal::from_str(s).ok());

                        balance.update_wallet(asset.to_string(), wallet_balance, cross_wallet);
                    }
                }
            }
        }

        _ => {
            return Err(Error::invalid_request(format!(
                "Unknown balance event type: {}",
                event_type
            )));
        }
    }

    Ok(())
}

/// Parses a WebSocket order message
pub fn parse_ws_order(data: &serde_json::Map<String, Value>) -> Order {
    let symbol = data
        .get("s")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("");
    let order_id = data
        .get("i")
        .and_then(serde_json::Value::as_i64)
        .map(|id| id.to_string())
        .unwrap_or_default();
    let client_order_id = data
        .get("c")
        .and_then(serde_json::Value::as_str)
        .map(String::from);

    let status_str = data
        .get("X")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("NEW");
    let status = match status_str {
        "FILLED" => OrderStatus::Closed,
        "CANCELED" => OrderStatus::Cancelled,
        "REJECTED" => OrderStatus::Rejected,
        "EXPIRED" => OrderStatus::Expired,
        _ => OrderStatus::Open,
    };

    let side_str = data
        .get("S")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("BUY");
    let side = match side_str {
        "SELL" => OrderSide::Sell,
        _ => OrderSide::Buy,
    };

    let type_str = data
        .get("o")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("LIMIT");
    let order_type = match type_str {
        "MARKET" => OrderType::Market,
        "STOP_LOSS" => OrderType::StopLoss,
        "STOP_LOSS_LIMIT" => OrderType::StopLossLimit,
        "TAKE_PROFIT" => OrderType::TakeProfit,
        "TAKE_PROFIT_LIMIT" => OrderType::TakeProfitLimit,
        "LIMIT_MAKER" => OrderType::LimitMaker,
        _ => OrderType::Limit,
    };

    let amount = data
        .get("q")
        .and_then(serde_json::Value::as_str)
        .and_then(|s| Decimal::from_str(s).ok())
        .unwrap_or(Decimal::ZERO);

    let price = data
        .get("p")
        .and_then(serde_json::Value::as_str)
        .and_then(|s| Decimal::from_str(s).ok());

    let filled = data
        .get("z")
        .and_then(serde_json::Value::as_str)
        .and_then(|s| Decimal::from_str(s).ok());

    let cost = data
        .get("Z")
        .and_then(serde_json::Value::as_str)
        .and_then(|s| Decimal::from_str(s).ok());

    let remaining = filled.map(|fill| amount - fill);

    let average = match (filled, cost) {
        (Some(fill), Some(c)) if fill > Decimal::ZERO && c > Decimal::ZERO => Some(c / fill),
        _ => None,
    };

    let timestamp = data.get("T").and_then(serde_json::Value::as_i64);
    let last_trade_timestamp = data.get("T").and_then(serde_json::Value::as_i64);

    Order {
        id: order_id,
        client_order_id,
        timestamp,
        datetime: timestamp.map(|ts| {
            chrono::DateTime::from_timestamp_millis(ts)
                .map(|dt| dt.to_rfc3339())
                .unwrap_or_default()
        }),
        last_trade_timestamp,
        symbol: symbol.to_string(),
        order_type,
        side,
        price,
        average,
        amount,
        cost,
        filled,
        remaining,
        status,
        fee: None,
        fees: None,
        trades: None,
        time_in_force: data
            .get("f")
            .and_then(serde_json::Value::as_str)
            .map(String::from),
        post_only: None,
        reduce_only: None,
        stop_price: data
            .get("P")
            .and_then(serde_json::Value::as_str)
            .and_then(|s| Decimal::from_str(s).ok()),
        trigger_price: None,
        take_profit_price: None,
        stop_loss_price: None,
        trailing_delta: None,
        trailing_percent: None,
        activation_price: None,
        callback_rate: None,
        working_type: data
            .get("wt")
            .and_then(serde_json::Value::as_str)
            .map(String::from),
        info: data.iter().map(|(k, v)| (k.clone(), v.clone())).collect(),
    }
}

/// Parses a WebSocket trade message
pub fn parse_ws_trade(data: &Value) -> Result<Trade> {
    let symbol = data
        .get("s")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| Error::invalid_request("Missing symbol field".to_string()))?
        .to_string();

    let id = data
        .get("t")
        .and_then(serde_json::Value::as_i64)
        .map(|v| v.to_string());

    let timestamp = data
        .get("T")
        .and_then(serde_json::Value::as_i64)
        .unwrap_or(0);

    let price = data
        .get("L")
        .and_then(serde_json::Value::as_str)
        .and_then(|s| Decimal::from_str(s).ok())
        .unwrap_or(Decimal::ZERO);

    let amount = data
        .get("l")
        .and_then(serde_json::Value::as_str)
        .and_then(|s| Decimal::from_str(s).ok())
        .unwrap_or(Decimal::ZERO);

    let cost = data
        .get("Y")
        .and_then(serde_json::Value::as_str)
        .and_then(|s| Decimal::from_str(s).ok())
        .or_else(|| {
            if price > Decimal::ZERO && amount > Decimal::ZERO {
                Some(price * amount)
            } else {
                None
            }
        });

    let side = data
        .get("S")
        .and_then(serde_json::Value::as_str)
        .and_then(|s| match s.to_uppercase().as_str() {
            "BUY" => Some(OrderSide::Buy),
            "SELL" => Some(OrderSide::Sell),
            _ => None,
        })
        .unwrap_or(OrderSide::Buy);

    let trade_type = data
        .get("o")
        .and_then(serde_json::Value::as_str)
        .and_then(|s| match s.to_uppercase().as_str() {
            "LIMIT" => Some(OrderType::Limit),
            "MARKET" => Some(OrderType::Market),
            _ => None,
        });

    let order_id = data
        .get("i")
        .and_then(serde_json::Value::as_i64)
        .map(|v| v.to_string());

    let taker_or_maker = data
        .get("m")
        .and_then(serde_json::Value::as_bool)
        .map(|is_maker| {
            if is_maker {
                TakerOrMaker::Maker
            } else {
                TakerOrMaker::Taker
            }
        });

    let fee = if let Some(fee_cost_str) = data.get("n").and_then(serde_json::Value::as_str) {
        if let Ok(fee_cost) = Decimal::from_str(fee_cost_str) {
            let currency = data
                .get("N")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("UNKNOWN")
                .to_string();
            Some(Fee {
                currency,
                cost: fee_cost,
                rate: None,
            })
        } else {
            None
        }
    } else {
        None
    };

    let datetime = chrono::DateTime::from_timestamp_millis(timestamp)
        .map(|dt| dt.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string());

    let mut info = HashMap::new();
    if let Value::Object(map) = data {
        for (k, v) in map {
            info.insert(k.clone(), v.clone());
        }
    }

    Ok(Trade {
        id,
        order: order_id,
        symbol,
        trade_type,
        side,
        taker_or_maker,
        price: ccxt_core::types::financial::Price::from(price),
        amount: ccxt_core::types::financial::Amount::from(amount),
        cost: cost.map(ccxt_core::types::financial::Cost::from),
        fee,
        timestamp,
        datetime,
        info,
    })
}

/// Parses a WebSocket position payload
pub fn parse_ws_position(data: &Value) -> Result<Position> {
    let symbol = data["s"]
        .as_str()
        .ok_or_else(|| Error::invalid_request("Missing symbol field"))?
        .to_string();

    let position_amount_str = data["pa"]
        .as_str()
        .ok_or_else(|| Error::invalid_request("Missing position amount"))?;

    let position_amount = position_amount_str
        .parse::<f64>()
        .map_err(|e| Error::invalid_request(format!("Invalid position amount: {}", e)))?;

    let position_side = data["ps"]
        .as_str()
        .ok_or_else(|| Error::invalid_request("Missing position side"))?
        .to_uppercase();

    let (side, hedged) = if position_side == "BOTH" {
        let actual_side = if position_amount < 0.0 {
            "short"
        } else {
            "long"
        };
        (actual_side.to_string(), false)
    } else {
        (position_side.to_lowercase(), true)
    };

    let entry_price = data["ep"].as_str().and_then(|s| s.parse::<f64>().ok());
    let unrealized_pnl = data["up"].as_str().and_then(|s| s.parse::<f64>().ok());
    let realized_pnl = data["cr"].as_str().and_then(|s| s.parse::<f64>().ok());
    let margin_mode = data["mt"].as_str().map(ToString::to_string);
    let initial_margin = data["iw"].as_str().and_then(|s| s.parse::<f64>().ok());

    Ok(Position {
        info: data.clone(),
        id: None,
        symbol,
        side: Some(side),
        contracts: Some(position_amount.abs()),
        contract_size: None,
        entry_price,
        mark_price: None,
        notional: None,
        leverage: None,
        collateral: initial_margin,
        initial_margin,
        initial_margin_percentage: None,
        maintenance_margin: None,
        maintenance_margin_percentage: None,
        unrealized_pnl,
        realized_pnl,
        liquidation_price: None,
        margin_ratio: None,
        margin_mode,
        hedged: Some(hedged),
        percentage: None,
        position_side: None,
        dual_side_position: None,
        timestamp: Some(chrono::Utc::now().timestamp_millis()),
        datetime: Some(chrono::Utc::now().to_rfc3339()),
    })
}
