//! Position management methods for Binance futures.

use super::super::super::{Binance, parser};
use ccxt_core::{Error, ParseError, Result, types::Position};
use serde_json::Value;
use tracing::warn;

impl Binance {
    /// Fetch a single position for a trading pair.
    pub async fn fetch_position(&self, symbol: &str, params: Option<Value>) -> Result<Position> {
        let market = self.base().market(symbol).await?;

        let url = if market.linear.unwrap_or(true) {
            format!("{}/positionRisk", self.urls().fapi_private)
        } else {
            format!("{}/positionRisk", self.urls().dapi_private)
        };

        let data = self
            .signed_request(url)
            .param("symbol", &market.id)
            .merge_json_params(params)
            .execute()
            .await?;

        let positions_array = data.as_array().ok_or_else(|| {
            Error::from(ParseError::invalid_format(
                "data",
                "Expected array of positions",
            ))
        })?;

        for position_data in positions_array {
            if let Some(pos_symbol) = position_data["symbol"].as_str() {
                if pos_symbol == market.id {
                    return parser::parse_position(position_data, Some(&market));
                }
            }
        }

        Err(Error::from(ParseError::missing_field_owned(format!(
            "Position not found for symbol: {}",
            symbol
        ))))
    }

    /// Fetch all positions.
    pub async fn fetch_positions(
        &self,
        symbols: Option<Vec<String>>,
        params: Option<Value>,
    ) -> Result<Vec<Position>> {
        let use_coin_m = params
            .as_ref()
            .and_then(|p| p.get("type"))
            .and_then(serde_json::Value::as_str)
            .is_some_and(|t| t == "delivery" || t == "coin_m");

        let url = if use_coin_m {
            format!("{}/positionRisk", self.urls().dapi_private)
        } else {
            format!("{}/positionRisk", self.urls().fapi_private)
        };

        let data = self
            .signed_request(url)
            .merge_json_params(params)
            .execute()
            .await?;

        let positions_array = data.as_array().ok_or_else(|| {
            Error::from(ParseError::invalid_format(
                "data",
                "Expected array of positions",
            ))
        })?;

        let markets_by_id = {
            let cache = self.base().market_cache.read().await;
            cache.markets_by_id.clone()
        };

        let mut positions = Vec::new();
        for position_data in positions_array {
            if let Some(binance_symbol) = position_data["symbol"].as_str() {
                if let Some(market) = markets_by_id.get(binance_symbol) {
                    match parser::parse_position(position_data, Some(market)) {
                        Ok(position) => {
                            if position.contracts.unwrap_or(0.0) > 0.0 {
                                if let Some(ref syms) = symbols {
                                    if syms.contains(&position.symbol) {
                                        positions.push(position);
                                    }
                                } else {
                                    positions.push(position);
                                }
                            }
                        }
                        Err(e) => {
                            warn!(
                                error = %e,
                                symbol = %binance_symbol,
                                "Failed to parse position"
                            );
                        }
                    }
                }
            }
        }

        Ok(positions)
    }

    /// Fetch position risk information.
    pub async fn fetch_positions_risk(
        &self,
        symbols: Option<Vec<String>>,
        params: Option<Value>,
    ) -> Result<Vec<Position>> {
        self.fetch_positions(symbols, params).await
    }

    /// Fetch position risk information (raw JSON).
    pub async fn fetch_position_risk(
        &self,
        symbol: Option<&str>,
        params: Option<Value>,
    ) -> Result<Value> {
        let market_id = if let Some(sym) = symbol {
            let market = self.base().market(sym).await?;
            Some(market.id.clone())
        } else {
            None
        };

        let url = format!("{}/positionRisk", self.urls().fapi_private);

        self.signed_request(url)
            .optional_param("symbol", market_id)
            .merge_json_params(params)
            .execute()
            .await
    }
}
