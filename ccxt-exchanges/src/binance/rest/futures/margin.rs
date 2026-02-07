//! Margin operations for Binance futures.

use super::super::super::{Binance, signed_request::HttpMethod};
use ccxt_core::{Error, ParseError, Result, types::MarketType};
use rust_decimal::Decimal;
use serde_json::Value;
use std::collections::{BTreeMap, HashMap};

impl Binance {
    /// Set margin mode for a trading pair.
    pub async fn set_margin_mode(
        &self,
        symbol: &str,
        margin_mode: &str,
        params: Option<HashMap<String, String>>,
    ) -> Result<HashMap<String, Value>> {
        let margin_type = match margin_mode.to_uppercase().as_str() {
            "ISOLATED" | "ISOLATED_MARGIN" => "ISOLATED",
            "CROSS" | "CROSSED" | "CROSS_MARGIN" => "CROSSED",
            _ => {
                return Err(Error::invalid_request(format!(
                    "Invalid margin mode: {}. Must be 'isolated' or 'cross'",
                    margin_mode
                )));
            }
        };

        self.load_markets(false).await?;
        let market = self.base().market(symbol).await?;

        if market.market_type != MarketType::Futures && market.market_type != MarketType::Swap {
            return Err(Error::invalid_request(
                "set_margin_mode() supports futures and swap markets only".to_string(),
            ));
        }

        let url = if market.linear.unwrap_or(true) {
            format!("{}/marginType", self.urls().fapi_private)
        } else if market.inverse.unwrap_or(false) {
            format!("{}/marginType", self.urls().dapi_private)
        } else {
            return Err(Error::invalid_request(
                "Unknown futures market type".to_string(),
            ));
        };

        let mut builder = self
            .signed_request(url)
            .method(HttpMethod::Post)
            .param("symbol", &market.id)
            .param("marginType", margin_type);

        if let Some(p) = params {
            let params_map: BTreeMap<String, String> = p.into_iter().collect();
            builder = builder.params(params_map);
        }

        let response = builder.execute().await?;

        let result: HashMap<String, Value> = serde_json::from_value(response).map_err(|e| {
            Error::from(ParseError::invalid_format(
                "data",
                format!("Failed to parse response: {}", e),
            ))
        })?;

        Ok(result)
    }

    /// Set position mode (hedge mode or one-way mode).
    pub async fn set_position_mode(&self, dual_side: bool, params: Option<Value>) -> Result<Value> {
        let use_dapi = params
            .as_ref()
            .and_then(|p| p.get("type"))
            .and_then(serde_json::Value::as_str)
            .is_some_and(|t| t == "delivery" || t == "coin_m");

        let url = if use_dapi {
            format!("{}/positionSide/dual", self.urls().dapi_private)
        } else {
            format!("{}/positionSide/dual", self.urls().fapi_private)
        };

        let builder = self
            .signed_request(url)
            .method(HttpMethod::Post)
            .param("dualSidePosition", dual_side)
            .merge_json_params(params);

        let data = builder.execute().await?;
        Ok(data)
    }

    /// Fetch current position mode.
    pub async fn fetch_position_mode(&self, params: Option<Value>) -> Result<bool> {
        let use_dapi = params
            .as_ref()
            .and_then(|p| p.get("type"))
            .and_then(serde_json::Value::as_str)
            .is_some_and(|t| t == "delivery" || t == "coin_m");

        let url = if use_dapi {
            format!("{}/positionSide/dual", self.urls().dapi_private)
        } else {
            format!("{}/positionSide/dual", self.urls().fapi_private)
        };

        let builder = self.signed_request(url).merge_json_params(params);

        let data = builder.execute().await?;

        if let Some(dual_side) = data.get("dualSidePosition") {
            if let Some(value) = dual_side.as_bool() {
                return Ok(value);
            }
            if let Some(value_str) = dual_side.as_str() {
                return Ok(value_str.to_lowercase() == "true");
            }
        }

        Err(Error::from(ParseError::invalid_format(
            "data",
            "Failed to parse position mode response",
        )))
    }

    /// Modify isolated position margin.
    pub async fn modify_isolated_position_margin(
        &self,
        symbol: &str,
        amount: Decimal,
        params: Option<Value>,
    ) -> Result<Value> {
        let market = self.base().market(symbol).await?;

        let url = if market.linear.unwrap_or(true) {
            format!("{}/positionMargin", self.urls().fapi_private)
        } else {
            format!("{}/positionMargin", self.urls().dapi_private)
        };

        let margin_type = if amount > Decimal::ZERO { "1" } else { "2" };

        let builder = self
            .signed_request(url)
            .method(HttpMethod::Post)
            .param("symbol", &market.id)
            .param("amount", amount.abs())
            .param("type", margin_type)
            .merge_json_params(params);

        let data = builder.execute().await?;
        Ok(data)
    }
}
