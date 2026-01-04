use super::{parse_decimal, parse_f64};
use ccxt_core::{
    Result,
    error::{Error, ParseError},
    types::{Leverage, LeverageTier, MarginType, Market, Position},
};
use rust_decimal::Decimal;
use serde_json::Value;

/// Parse position data from Binance futures position risk.
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

    let contracts = parse_f64(data, "positionAmt").map(f64::abs);
    let contract_size = Some(1.0);
    let entry_price = parse_f64(data, "entryPrice");
    let mark_price = parse_f64(data, "markPrice");
    let notional = parse_f64(data, "notional").map(f64::abs);
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
        .map(str::to_lowercase);
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
        realized_pnl: None,
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
pub fn parse_leverage(data: &Value, _market: Option<&Market>) -> Result<Leverage> {
    let market_id = data
        .get("symbol")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("");

    let margin_mode =
        if let Some(isolated) = data.get("isolated").and_then(serde_json::Value::as_bool) {
            Some(if isolated {
                MarginType::Isolated
            } else {
                MarginType::Cross
            })
        } else {
            data.get("marginType")
                .and_then(serde_json::Value::as_str)
                .map(|margin_type| {
                    if margin_type == "crossed" {
                        MarginType::Cross
                    } else {
                        MarginType::Isolated
                    }
                })
        };

    let side = data
        .get("positionSide")
        .and_then(serde_json::Value::as_str)
        .map(str::to_lowercase);

    let leverage_value = data.get("leverage").and_then(serde_json::Value::as_i64);

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

/// Parse leverage tier from Binance API response.
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
