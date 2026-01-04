//! Leverage operations for Binance futures.

use super::super::super::{Binance, parser, signed_request::HttpMethod};
use ccxt_core::{
    Error, Result,
    types::{LeverageTier, MarketType},
};
use serde_json::Value;
use std::collections::{BTreeMap, HashMap};

impl Binance {
    /// Fetch leverage settings for multiple trading pairs.
    pub async fn fetch_leverages(
        &self,
        symbols: Option<Vec<String>>,
        params: Option<Value>,
    ) -> Result<BTreeMap<String, ccxt_core::types::Leverage>> {
        self.load_markets(false).await?;

        let mut params_map = if let Some(p) = params {
            serde_json::from_value::<BTreeMap<String, String>>(p).unwrap_or_default()
        } else {
            BTreeMap::new()
        };

        let market_type = params_map
            .remove("type")
            .or_else(|| params_map.remove("marketType"))
            .unwrap_or_else(|| "future".to_string());

        let sub_type = params_map
            .remove("subType")
            .unwrap_or_else(|| "linear".to_string());

        let is_portfolio_margin = params_map
            .remove("portfolioMargin")
            .and_then(|v| v.parse::<bool>().ok())
            .unwrap_or(false);

        let url = if market_type == "future" && sub_type == "linear" {
            if is_portfolio_margin {
                format!(
                    "{}/account",
                    self.urls().fapi_private.replace("/fapi/v1", "/papi/v1/um")
                )
            } else {
                format!("{}/symbolConfig", self.urls().fapi_private)
            }
        } else if market_type == "future" && sub_type == "inverse" {
            if is_portfolio_margin {
                format!(
                    "{}/account",
                    self.urls().dapi_private.replace("/dapi/v1", "/papi/v1/cm")
                )
            } else {
                format!("{}/account", self.urls().dapi_private)
            }
        } else {
            return Err(Error::invalid_request(
                "fetchLeverages() supports linear and inverse contracts only",
            ));
        };

        let response = self
            .signed_request(url)
            .params(params_map)
            .execute()
            .await?;

        let leverages_data = if let Some(positions) = response.get("positions") {
            positions.as_array().cloned().unwrap_or_default()
        } else if response.is_array() {
            response.as_array().cloned().unwrap_or_default()
        } else {
            vec![]
        };

        let mut leverages = BTreeMap::new();

        for item in leverages_data {
            if let Ok(leverage) = parser::parse_leverage(&item, None) {
                if let Some(ref filter_symbols) = symbols {
                    if filter_symbols.contains(&leverage.symbol) {
                        leverages.insert(leverage.symbol.clone(), leverage);
                    }
                } else {
                    leverages.insert(leverage.symbol.clone(), leverage);
                }
            }
        }

        Ok(leverages)
    }

    /// Fetch leverage settings for a single trading pair.
    pub async fn fetch_leverage(
        &self,
        symbol: &str,
        params: Option<Value>,
    ) -> Result<ccxt_core::types::Leverage> {
        let symbols = Some(vec![symbol.to_string()]);
        let leverages = self.fetch_leverages(symbols, params).await?;

        leverages.get(symbol).cloned().ok_or_else(|| {
            Error::exchange("404", format!("Leverage not found for symbol: {}", symbol))
        })
    }

    /// Set leverage multiplier for a trading pair.
    pub async fn set_leverage(
        &self,
        symbol: &str,
        leverage: i64,
        params: Option<HashMap<String, String>>,
    ) -> Result<HashMap<String, Value>> {
        if !(1..=125).contains(&leverage) {
            return Err(Error::invalid_request(
                "Leverage must be between 1 and 125".to_string(),
            ));
        }

        self.load_markets(false).await?;
        let market = self.base().market(symbol).await?;

        if market.market_type != MarketType::Futures && market.market_type != MarketType::Swap {
            return Err(Error::invalid_request(
                "set_leverage() supports futures and swap markets only".to_string(),
            ));
        }

        let url = if market.linear.unwrap_or(true) {
            format!("{}/leverage", self.urls().fapi_private)
        } else if market.inverse.unwrap_or(false) {
            format!("{}/leverage", self.urls().dapi_private)
        } else {
            return Err(Error::invalid_request(
                "Unknown futures market type".to_string(),
            ));
        };

        let mut builder = self
            .signed_request(url)
            .method(HttpMethod::Post)
            .param("symbol", &market.id)
            .param("leverage", leverage);

        if let Some(p) = params {
            let params_map: BTreeMap<String, String> = p.into_iter().collect();
            builder = builder.params(params_map);
        }

        let response = builder.execute().await?;

        let result: HashMap<String, Value> = serde_json::from_value(response).map_err(|e| {
            Error::from(ccxt_core::ParseError::invalid_format(
                "data",
                format!("Failed to parse response: {}", e),
            ))
        })?;

        Ok(result)
    }

    /// Fetch leverage bracket information.
    pub async fn fetch_leverage_bracket(
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

        let url = format!("{}/leverageBracket", self.urls().fapi_private);

        self.signed_request(url)
            .optional_param("symbol", market_id)
            .merge_json_params(params)
            .execute()
            .await
    }

    /// Fetch leverage tier information for trading pairs.
    pub async fn fetch_leverage_tiers(
        &self,
        symbols: Option<Vec<String>>,
        params: Option<HashMap<String, String>>,
    ) -> Result<BTreeMap<String, Vec<LeverageTier>>> {
        self.load_markets(false).await?;

        let is_portfolio_margin = params
            .as_ref()
            .and_then(|p| p.get("portfolioMargin"))
            .is_some_and(|v| v == "true");

        let (market, market_id) = if let Some(syms) = &symbols {
            if let Some(first_symbol) = syms.first() {
                let m = self.base().market(first_symbol).await?;
                let id = m.id.clone();
                (Some(m), Some(id))
            } else {
                (None, None)
            }
        } else {
            (None, None)
        };

        let url = if let Some(ref m) = market {
            if is_portfolio_margin {
                if m.is_linear() {
                    format!("{}/um/leverageBracket", self.urls().papi)
                } else {
                    format!("{}/cm/leverageBracket", self.urls().papi)
                }
            } else if m.is_linear() {
                format!("{}/leverageBracket", self.urls().fapi_private)
            } else {
                format!("{}/v2/leverageBracket", self.urls().dapi_private)
            }
        } else {
            format!("{}/leverageBracket", self.urls().fapi_private)
        };

        let filtered_params: Option<BTreeMap<String, String>> = params.map(|p| {
            p.into_iter()
                .filter(|(k, _)| k != "portfolioMargin")
                .collect()
        });

        let response = self
            .signed_request(url)
            .optional_param("symbol", market_id)
            .params(filtered_params.unwrap_or_default())
            .execute()
            .await?;

        let mut tiers_map: BTreeMap<String, Vec<LeverageTier>> = BTreeMap::new();

        if let Some(symbols_array) = response.as_array() {
            for symbol_data in symbols_array {
                if let (Some(symbol_id), Some(brackets)) = (
                    symbol_data["symbol"].as_str(),
                    symbol_data["brackets"].as_array(),
                ) {
                    if let Ok(market) = self.base().market_by_id(symbol_id).await {
                        let mut tier_list = Vec::new();
                        for bracket in brackets {
                            if let Ok(tier) = parser::parse_leverage_tier(bracket, &market) {
                                tier_list.push(tier);
                            }
                        }

                        if !tier_list.is_empty() {
                            if let Some(ref filter_symbols) = symbols {
                                if filter_symbols.contains(&market.symbol) {
                                    tiers_map.insert(market.symbol.clone(), tier_list);
                                }
                            } else {
                                tiers_map.insert(market.symbol.clone(), tier_list);
                            }
                        }
                    }
                }
            }
        }

        Ok(tiers_map)
    }
}
