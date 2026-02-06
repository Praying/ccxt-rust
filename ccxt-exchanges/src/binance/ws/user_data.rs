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

/// Parses an ORDER_TRADE_UPDATE event into an Order.
///
/// This event is sent by Binance Futures (USDT-M and COIN-M) when an order
/// is created, updated, or filled. The order data is nested under the "o" field.
///
/// # Arguments
///
/// * `data` - The full ORDER_TRADE_UPDATE event payload
///
/// # Returns
///
/// An `Order` parsed from the nested order object.
pub fn parse_order_trade_update_to_order(data: &Value) -> Result<Order> {
    let order_data = data
        .get("o")
        .ok_or_else(|| Error::invalid_request("Missing order data 'o' in ORDER_TRADE_UPDATE"))?;

    let order_map = order_data
        .as_object()
        .ok_or_else(|| Error::invalid_request("ORDER_TRADE_UPDATE 'o' field is not an object"))?;

    Ok(parse_ws_order(order_map))
}

/// Parses an ORDER_TRADE_UPDATE event into a Trade.
///
/// This event is sent by Binance Futures when a trade execution occurs.
/// Only produces a trade when the execution type (field "x") is "TRADE".
///
/// # Arguments
///
/// * `data` - The full ORDER_TRADE_UPDATE event payload
///
/// # Returns
///
/// A `Trade` if the execution type is TRADE, or an error otherwise.
pub fn parse_order_trade_update_to_trade(data: &Value) -> Result<Trade> {
    let order_data = data
        .get("o")
        .ok_or_else(|| Error::invalid_request("Missing order data 'o' in ORDER_TRADE_UPDATE"))?;

    // Only produce a trade when execution type is TRADE
    let exec_type = order_data.get("x").and_then(|x| x.as_str()).unwrap_or("");

    if exec_type != "TRADE" {
        return Err(Error::invalid_request(format!(
            "ORDER_TRADE_UPDATE execution type is '{}', not 'TRADE'",
            exec_type
        )));
    }

    parse_ws_trade(order_data)
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // ==================== parse_ws_order tests ====================

    #[test]
    fn test_parse_ws_order_spot_execution_report() {
        let data = json!({
            "e": "executionReport",
            "s": "BTCUSDT",
            "i": 123456789,
            "c": "myOrder1",
            "S": "BUY",
            "o": "LIMIT",
            "X": "NEW",
            "q": "0.5",
            "p": "50000.00",
            "z": "0.0",
            "Z": "0.0",
            "T": 1700000000000_i64,
            "f": "GTC",
            "P": "0.0"
        });

        let map = data.as_object().unwrap();
        let order = parse_ws_order(map);

        assert_eq!(order.symbol, "BTCUSDT");
        assert_eq!(order.id, "123456789");
        assert_eq!(order.client_order_id, Some("myOrder1".to_string()));
        assert_eq!(order.side, OrderSide::Buy);
        assert_eq!(order.order_type, OrderType::Limit);
        assert_eq!(order.status, OrderStatus::Open);
        assert_eq!(order.amount, Decimal::from_str("0.5").unwrap());
        assert_eq!(order.price, Some(Decimal::from_str("50000.00").unwrap()));
        assert_eq!(order.timestamp, Some(1700000000000));
        assert_eq!(order.time_in_force, Some("GTC".to_string()));
    }

    #[test]
    fn test_parse_ws_order_filled() {
        let data = json!({
            "e": "executionReport",
            "s": "ETHUSDT",
            "i": 987654321,
            "S": "SELL",
            "o": "MARKET",
            "X": "FILLED",
            "q": "10.0",
            "p": "0.0",
            "z": "10.0",
            "Z": "30000.0",
            "T": 1700000001000_i64
        });

        let map = data.as_object().unwrap();
        let order = parse_ws_order(map);

        assert_eq!(order.symbol, "ETHUSDT");
        assert_eq!(order.side, OrderSide::Sell);
        assert_eq!(order.order_type, OrderType::Market);
        assert_eq!(order.status, OrderStatus::Closed);
        assert_eq!(order.filled, Some(Decimal::from_str("10.0").unwrap()));
        assert_eq!(order.cost, Some(Decimal::from_str("30000.0").unwrap()));
        // average = cost / filled = 30000 / 10 = 3000
        assert_eq!(order.average, Some(Decimal::from_str("3000").unwrap()));
        assert_eq!(order.remaining, Some(Decimal::ZERO));
    }

    #[test]
    fn test_parse_ws_order_cancelled() {
        let data = json!({
            "e": "executionReport",
            "s": "BTCUSDT",
            "i": 111,
            "S": "BUY",
            "o": "LIMIT",
            "X": "CANCELED",
            "q": "1.0",
            "p": "40000.0",
            "z": "0.0",
            "Z": "0.0",
            "T": 1700000002000_i64
        });

        let map = data.as_object().unwrap();
        let order = parse_ws_order(map);

        assert_eq!(order.status, OrderStatus::Cancelled);
    }

    #[test]
    fn test_parse_ws_order_all_statuses() {
        let statuses = vec![
            ("NEW", OrderStatus::Open),
            ("PARTIALLY_FILLED", OrderStatus::Open),
            ("FILLED", OrderStatus::Closed),
            ("CANCELED", OrderStatus::Cancelled),
            ("REJECTED", OrderStatus::Rejected),
            ("EXPIRED", OrderStatus::Expired),
        ];

        for (binance_status, expected) in statuses {
            let data = json!({
                "s": "BTCUSDT", "i": 1, "S": "BUY", "o": "LIMIT",
                "X": binance_status, "q": "1.0", "p": "50000.0",
                "z": "0.0", "Z": "0.0", "T": 1700000000000_i64
            });
            let map = data.as_object().unwrap();
            let order = parse_ws_order(map);
            assert_eq!(
                order.status, expected,
                "Status mismatch for {}",
                binance_status
            );
        }
    }

    #[test]
    fn test_parse_ws_order_all_types() {
        let types = vec![
            ("LIMIT", OrderType::Limit),
            ("MARKET", OrderType::Market),
            ("STOP_LOSS", OrderType::StopLoss),
            ("STOP_LOSS_LIMIT", OrderType::StopLossLimit),
            ("TAKE_PROFIT", OrderType::TakeProfit),
            ("TAKE_PROFIT_LIMIT", OrderType::TakeProfitLimit),
            ("LIMIT_MAKER", OrderType::LimitMaker),
        ];

        for (binance_type, expected) in types {
            let data = json!({
                "s": "BTCUSDT", "i": 1, "S": "BUY", "o": binance_type,
                "X": "NEW", "q": "1.0", "p": "50000.0",
                "z": "0.0", "Z": "0.0", "T": 1700000000000_i64
            });
            let map = data.as_object().unwrap();
            let order = parse_ws_order(map);
            assert_eq!(
                order.order_type, expected,
                "Type mismatch for {}",
                binance_type
            );
        }
    }

    // ==================== parse_ws_trade tests ====================

    #[test]
    fn test_parse_ws_trade_spot() {
        let data = json!({
            "e": "executionReport",
            "s": "BTCUSDT",
            "t": 100001,
            "T": 1700000000000_i64,
            "L": "50000.00",
            "l": "0.1",
            "Y": "5000.0",
            "S": "BUY",
            "o": "LIMIT",
            "i": 123456,
            "m": false,
            "n": "0.005",
            "N": "BNB"
        });

        let trade = parse_ws_trade(&data).unwrap();

        assert_eq!(trade.symbol, "BTCUSDT");
        assert_eq!(trade.id, Some("100001".to_string()));
        assert_eq!(trade.timestamp, 1700000000000);
        assert_eq!(
            trade.price,
            ccxt_core::types::financial::Price::from(Decimal::from_str("50000.00").unwrap())
        );
        assert_eq!(
            trade.amount,
            ccxt_core::types::financial::Amount::from(Decimal::from_str("0.1").unwrap())
        );
        assert_eq!(trade.side, OrderSide::Buy);
        assert_eq!(trade.taker_or_maker, Some(TakerOrMaker::Taker));
        assert_eq!(trade.order, Some("123456".to_string()));

        let fee = trade.fee.unwrap();
        assert_eq!(fee.cost, Decimal::from_str("0.005").unwrap());
        assert_eq!(fee.currency, "BNB");
    }

    #[test]
    fn test_parse_ws_trade_maker() {
        let data = json!({
            "s": "ETHUSDT",
            "t": 200001,
            "T": 1700000001000_i64,
            "L": "3000.00",
            "l": "1.0",
            "S": "SELL",
            "o": "LIMIT",
            "i": 789,
            "m": true
        });

        let trade = parse_ws_trade(&data).unwrap();

        assert_eq!(trade.side, OrderSide::Sell);
        assert_eq!(trade.taker_or_maker, Some(TakerOrMaker::Maker));
    }

    #[test]
    fn test_parse_ws_trade_cost_calculated() {
        // When "Y" (quote quantity) is missing, cost should be calculated as price * amount
        let data = json!({
            "s": "BTCUSDT",
            "T": 1700000000000_i64,
            "L": "50000.00",
            "l": "0.1",
            "S": "BUY",
            "i": 1
        });

        let trade = parse_ws_trade(&data).unwrap();
        // cost = 50000 * 0.1 = 5000
        assert_eq!(
            trade.cost,
            Some(ccxt_core::types::financial::Cost::from(
                Decimal::from_str("5000.0000").unwrap()
            ))
        );
    }

    #[test]
    fn test_parse_ws_trade_missing_symbol() {
        let data = json!({
            "T": 1700000000000_i64,
            "L": "50000.00",
            "l": "0.1"
        });

        let result = parse_ws_trade(&data);
        assert!(result.is_err());
    }

    // ==================== ORDER_TRADE_UPDATE tests ====================

    #[test]
    fn test_parse_order_trade_update_to_order() {
        let data = json!({
            "e": "ORDER_TRADE_UPDATE",
            "T": 1700000000000_i64,
            "o": {
                "s": "BTCUSDT",
                "i": 555666777,
                "c": "futuresOrder1",
                "S": "BUY",
                "o": "LIMIT",
                "X": "NEW",
                "q": "0.01",
                "p": "50000.00",
                "z": "0.0",
                "Z": "0.0",
                "T": 1700000000000_i64,
                "f": "GTC",
                "ps": "LONG",
                "wt": "CONTRACT_PRICE"
            }
        });

        let order = parse_order_trade_update_to_order(&data).unwrap();

        assert_eq!(order.symbol, "BTCUSDT");
        assert_eq!(order.id, "555666777");
        assert_eq!(order.client_order_id, Some("futuresOrder1".to_string()));
        assert_eq!(order.side, OrderSide::Buy);
        assert_eq!(order.order_type, OrderType::Limit);
        assert_eq!(order.status, OrderStatus::Open);
        assert_eq!(order.amount, Decimal::from_str("0.01").unwrap());
        assert_eq!(order.working_type, Some("CONTRACT_PRICE".to_string()));
    }

    #[test]
    fn test_parse_order_trade_update_missing_o_field() {
        let data = json!({
            "e": "ORDER_TRADE_UPDATE",
            "T": 1700000000000_i64
        });

        let result = parse_order_trade_update_to_order(&data);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_order_trade_update_to_trade_execution() {
        let data = json!({
            "e": "ORDER_TRADE_UPDATE",
            "T": 1700000000000_i64,
            "o": {
                "s": "BTCUSDT",
                "t": 300001,
                "T": 1700000000000_i64,
                "x": "TRADE",
                "X": "PARTIALLY_FILLED",
                "L": "50000.00",
                "l": "0.005",
                "Y": "250.0",
                "S": "BUY",
                "o": "LIMIT",
                "i": 555666777,
                "m": false,
                "n": "0.001",
                "N": "USDT"
            }
        });

        let trade = parse_order_trade_update_to_trade(&data).unwrap();

        assert_eq!(trade.symbol, "BTCUSDT");
        assert_eq!(trade.id, Some("300001".to_string()));
        assert_eq!(
            trade.price,
            ccxt_core::types::financial::Price::from(Decimal::from_str("50000.00").unwrap())
        );
        assert_eq!(
            trade.amount,
            ccxt_core::types::financial::Amount::from(Decimal::from_str("0.005").unwrap())
        );
        assert_eq!(trade.side, OrderSide::Buy);
        assert_eq!(trade.taker_or_maker, Some(TakerOrMaker::Taker));

        let fee = trade.fee.unwrap();
        assert_eq!(fee.cost, Decimal::from_str("0.001").unwrap());
        assert_eq!(fee.currency, "USDT");
    }

    #[test]
    fn test_parse_order_trade_update_to_trade_non_trade_execution() {
        // When execution type is not "TRADE", should return error
        let data = json!({
            "e": "ORDER_TRADE_UPDATE",
            "T": 1700000000000_i64,
            "o": {
                "s": "BTCUSDT",
                "x": "NEW",
                "X": "NEW",
                "S": "BUY",
                "o": "LIMIT",
                "i": 1
            }
        });

        let result = parse_order_trade_update_to_trade(&data);
        assert!(result.is_err());
    }

    // ==================== parse_ws_position tests ====================

    #[test]
    fn test_parse_ws_position_long() {
        let data = json!({
            "s": "BTCUSDT",
            "pa": "0.5",
            "ps": "LONG",
            "ep": "50000.00",
            "up": "1000.00",
            "cr": "500.00",
            "mt": "cross",
            "iw": "5000.00"
        });

        let position = parse_ws_position(&data).unwrap();

        assert_eq!(position.symbol, "BTCUSDT");
        assert_eq!(position.side, Some("long".to_string()));
        assert_eq!(position.contracts, Some(0.5));
        assert_eq!(position.hedged, Some(true));
        assert_eq!(position.entry_price, Some(50000.00));
        assert_eq!(position.unrealized_pnl, Some(1000.00));
        assert_eq!(position.realized_pnl, Some(500.00));
        assert_eq!(position.margin_mode, Some("cross".to_string()));
        assert_eq!(position.initial_margin, Some(5000.00));
    }

    #[test]
    fn test_parse_ws_position_short() {
        let data = json!({
            "s": "ETHUSDT",
            "pa": "-10.0",
            "ps": "SHORT",
            "ep": "3000.00",
            "up": "-200.00"
        });

        let position = parse_ws_position(&data).unwrap();

        assert_eq!(position.symbol, "ETHUSDT");
        assert_eq!(position.side, Some("short".to_string()));
        assert_eq!(position.contracts, Some(10.0));
        assert_eq!(position.hedged, Some(true));
    }

    #[test]
    fn test_parse_ws_position_both_positive() {
        // BOTH mode with positive amount -> long
        let data = json!({
            "s": "BTCUSDT",
            "pa": "1.0",
            "ps": "BOTH"
        });

        let position = parse_ws_position(&data).unwrap();

        assert_eq!(position.side, Some("long".to_string()));
        assert_eq!(position.hedged, Some(false));
    }

    #[test]
    fn test_parse_ws_position_both_negative() {
        // BOTH mode with negative amount -> short
        let data = json!({
            "s": "BTCUSDT",
            "pa": "-1.0",
            "ps": "BOTH"
        });

        let position = parse_ws_position(&data).unwrap();

        assert_eq!(position.side, Some("short".to_string()));
        assert_eq!(position.hedged, Some(false));
    }

    #[test]
    fn test_parse_ws_position_missing_symbol() {
        let data = json!({
            "pa": "1.0",
            "ps": "BOTH"
        });

        let result = parse_ws_position(&data);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_ws_position_missing_position_amount() {
        let data = json!({
            "s": "BTCUSDT",
            "ps": "BOTH"
        });

        let result = parse_ws_position(&data);
        assert!(result.is_err());
    }

    // ==================== handle_balance_message tests ====================

    #[tokio::test]
    async fn test_handle_balance_update() {
        let balances = tokio::sync::RwLock::new(HashMap::new());

        let msg = json!({
            "e": "balanceUpdate",
            "a": "BTC",
            "d": "0.5"
        });

        let result = handle_balance_message(&msg, "spot", &balances).await;
        assert!(result.is_ok());

        let balances_map = balances.read().await;
        assert!(balances_map.contains_key("spot"));
    }

    #[tokio::test]
    async fn test_handle_outbound_account_position() {
        let balances = tokio::sync::RwLock::new(HashMap::new());

        let msg = json!({
            "e": "outboundAccountPosition",
            "B": [
                {"a": "BTC", "f": "1.5", "l": "0.5"},
                {"a": "ETH", "f": "10.0", "l": "2.0"}
            ]
        });

        let result = handle_balance_message(&msg, "spot", &balances).await;
        assert!(result.is_ok());

        let balances_map = balances.read().await;
        assert!(balances_map.contains_key("spot"));
    }

    #[tokio::test]
    async fn test_handle_account_update_futures() {
        let balances = tokio::sync::RwLock::new(HashMap::new());

        let msg = json!({
            "e": "ACCOUNT_UPDATE",
            "a": {
                "B": [
                    {"a": "USDT", "wb": "10000.00", "cw": "9500.00"}
                ]
            }
        });

        let result = handle_balance_message(&msg, "future", &balances).await;
        assert!(result.is_ok());

        let balances_map = balances.read().await;
        assert!(balances_map.contains_key("future"));
    }

    #[tokio::test]
    async fn test_handle_unknown_event_type() {
        let balances = tokio::sync::RwLock::new(HashMap::new());

        let msg = json!({
            "e": "unknownEvent"
        });

        let result = handle_balance_message(&msg, "spot", &balances).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_handle_missing_event_type() {
        let balances = tokio::sync::RwLock::new(HashMap::new());

        let msg = json!({
            "data": "something"
        });

        let result = handle_balance_message(&msg, "spot", &balances).await;
        assert!(result.is_err());
    }
}
