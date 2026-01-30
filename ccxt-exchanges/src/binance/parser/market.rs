use super::value_to_hashmap;
use ccxt_core::{
    Result,
    error::{Error, ParseError},
    types::{Market, MarketLimits, MarketPrecision, MarketType, MinMax},
};
use rust_decimal::Decimal;
use rust_decimal::prelude::FromStr;
use serde_json::Value;

/// Parse market data from Binance exchange info.
pub fn parse_market(data: &Value) -> Result<Market> {
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

    let active = status == "TRADING";
    let margin = data["isMarginTradingAllowed"].as_bool().unwrap_or(false);

    // Check for contract type to determine if this is a futures/swap market
    let contract_type = data["contractType"].as_str();
    let is_contract = contract_type.is_some();

    let market_type = if let Some(ct) = contract_type {
        if ct == "PERPETUAL" {
            MarketType::Swap
        } else {
            MarketType::Futures
        }
    } else {
        MarketType::Spot
    };

    let mut linear = None;
    let mut inverse = None;
    let mut settle_id = None;
    let mut settle = None;

    if is_contract {
        if let Some(margin_asset) = data["marginAsset"].as_str() {
            settle_id = Some(margin_asset.to_string());
            settle = Some(margin_asset.to_string()); // In Binance, asset ID is usually the code

            if margin_asset == quote_asset {
                linear = Some(true);
                inverse = Some(false);
            } else if margin_asset == base_asset {
                linear = Some(false);
                inverse = Some(true);
            }
        }
    }

    let unified_symbol = if is_contract {
        if let Some(s) = &settle {
            // For now simplified logic: base/quote:settle
            // TODO: Add expiration handling for Futures (vs Swap)
            format!("{}/{}:{}", base_asset, quote_asset, s)
        } else {
            format!("{}/{}", base_asset, quote_asset)
        }
    } else {
        format!("{}/{}", base_asset, quote_asset)
    };

    let parsed_symbol = ccxt_core::symbol::SymbolParser::parse(&unified_symbol).ok();

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

    let unified_symbol = if is_contract {
        if let Some(s) = &settle {
            format!("{}/{}:{}", base_asset, quote_asset, s)
        } else {
            format!("{}/{}", base_asset, quote_asset)
        }
    } else {
        format!("{}/{}", base_asset, quote_asset)
    };

    let parsed_symbol = ccxt_core::symbol::SymbolParser::parse(&unified_symbol).ok();

    Ok(Market {
        id: symbol.clone(),
        symbol: unified_symbol,
        parsed_symbol,
        base: base_asset.clone(),
        quote: quote_asset.clone(),
        base_id: Some(base_asset),
        quote_id: Some(quote_asset),
        settle_id,
        settle,
        market_type,
        active,
        margin,
        contract: Some(is_contract),
        linear,
        inverse,
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

/// Parse currency information from Binance API response.
pub fn parse_currency(data: &Value) -> Result<ccxt_core::types::Currency> {
    use ccxt_core::types::{Currency, CurrencyNetwork, MinMax};
    use std::collections::HashMap;

    let code = data["coin"]
        .as_str()
        .ok_or_else(|| Error::from(ParseError::missing_field("coin")))?
        .to_string();

    let id = code.clone();
    let name = data["name"].as_str().map(ToString::to_string);
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
                name: network_data["name"].as_str().map(ToString::to_string),
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

    if !networks.is_empty() && global_fee.is_none() {
        if let Some(first) = networks.values().next() {
            global_fee = first.fee;
            global_precision = first.precision;
            global_limits = first.limits.clone();
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
pub fn parse_currencies(data: &Value) -> Result<Vec<ccxt_core::types::Currency>> {
    if let Some(array) = data.as_array() {
        array.iter().map(parse_currency).collect()
    } else {
        Ok(vec![parse_currency(data)?])
    }
}
