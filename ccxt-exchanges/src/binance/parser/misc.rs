#![allow(dead_code, clippy::unnecessary_wraps)]

use super::{parse_decimal, value_to_hashmap};
use ccxt_core::{
    Result,
    error::{Error, ParseError},
    types::{
        AccountConfig, CommissionRate, FeeTradingFee, IndexPrice, LedgerDirection, LedgerEntry,
        LedgerEntryType, Liquidation, MarkPrice, Market, MaxLeverage, MinMax, OpenInterest,
        OpenInterestHistory, PremiumIndex, Stats24hr, TradingLimits,
    },
};
use rust_decimal::Decimal;
use rust_decimal::prelude::{FromPrimitive, FromStr, ToPrimitive};
use serde_json::Value;

/// Parse server time data from Binance API.
pub fn parse_server_time(data: &Value) -> Result<ccxt_core::types::ServerTime> {
    use ccxt_core::types::ServerTime;

    let server_time = data["serverTime"]
        .as_i64()
        .ok_or_else(|| Error::from(ParseError::missing_field("serverTime")))?;

    Ok(ServerTime::new(server_time))
}

/// Parse trading fee data from Binance API.
pub fn parse_trading_fee(data: &Value) -> Result<FeeTradingFee> {
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
pub fn parse_trading_fees(data: &Value) -> Result<Vec<FeeTradingFee>> {
    if let Some(array) = data.as_array() {
        array.iter().map(|item| parse_trading_fee(item)).collect()
    } else {
        Ok(vec![parse_trading_fee(data)?])
    }
}

/// Parse 24-hour statistics from Binance API response.
pub fn parse_stats_24hr(data: &Value) -> Result<ccxt_core::types::Stats24hr> {
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

    let open_price = data["openPrice"]
        .as_str()
        .and_then(|s| Decimal::from_str(s).ok());

    let high_price = data["highPrice"]
        .as_str()
        .and_then(|s| Decimal::from_str(s).ok());

    let low_price = data["lowPrice"]
        .as_str()
        .and_then(|s| Decimal::from_str(s).ok());

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

/// Parse trading limits from Binance API response.
pub fn parse_trading_limits(
    data: &Value,
    _symbol: String,
) -> Result<ccxt_core::types::TradingLimits> {
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

/// Parse account configuration from Binance futures account info.
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

/// Parse open interest data from Binance futures API.
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

/// Parse index price data from Binance futures API.
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

    let next_funding_time = data["nextFundingTime"].as_i64().unwrap_or(0);

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

/// Parse mark price data from Binance futures API.
pub fn parse_mark_price(data: &Value) -> Result<ccxt_core::types::MarkPrice> {
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
pub fn parse_mark_prices(data: &Value) -> Result<Vec<ccxt_core::types::MarkPrice>> {
    if let Some(array) = data.as_array() {
        array.iter().map(|item| parse_mark_price(item)).collect()
    } else {
        Ok(vec![parse_mark_price(data)?])
    }
}

/// Parse WebSocket mark price data from Binance futures streams.
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

/// Parse ledger entry from Binance API response.
pub fn parse_ledger_entry(data: &Value) -> Result<LedgerEntry> {
    let id = data["tranId"]
        .as_i64()
        .or_else(|| data["id"].as_i64())
        .map(|v| v.to_string())
        .or_else(|| data["tranId"].as_str().map(ToString::to_string))
        .or_else(|| data["id"].as_str().map(ToString::to_string))
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
