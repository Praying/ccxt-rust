//! Funding rate operations for Binance futures.

use super::super::super::{Binance, parser};
use ccxt_core::{
    Error, ParseError, Result,
    types::{FeeFundingRate, FeeFundingRateHistory, MarketType},
};
use serde_json::Value;
use std::collections::{BTreeMap, HashMap};
use tracing::warn;

impl Binance {
    /// Fetch current funding rate for a trading pair.
    pub async fn fetch_funding_rate(
        &self,
        symbol: &str,
        params: Option<HashMap<String, String>>,
    ) -> Result<FeeFundingRate> {
        self.load_markets(false).await?;
        let market = self.base().market(symbol).await?;

        if market.market_type != MarketType::Futures && market.market_type != MarketType::Swap {
            return Err(Error::invalid_request(
                "fetch_funding_rate() supports futures and swap markets only".to_string(),
            ));
        }

        let mut request_params = BTreeMap::new();
        request_params.insert("symbol".to_string(), market.id.clone());

        if let Some(p) = params {
            for (key, value) in p {
                request_params.insert(key, value);
            }
        }

        let url = if market.linear.unwrap_or(true) {
            format!("{}/premiumIndex", self.urls().fapi_public)
        } else {
            format!("{}/premiumIndex", self.urls().dapi_public)
        };

        let query = crate::binance::signed_request::build_query_string(&request_params);
        let request_url = if query.is_empty() {
            url
        } else {
            format!("{}?{}", url, query)
        };

        let response = self.public_get(&request_url, None).await?;

        let data = if market.linear.unwrap_or(true) {
            &response
        } else {
            response
                .as_array()
                .and_then(|arr| arr.first())
                .ok_or_else(|| {
                    Error::from(ParseError::invalid_format(
                        "data",
                        "COIN-M funding rate response should be an array with at least one element",
                    ))
                })?
        };

        parser::parse_funding_rate(data, Some(&market))
    }

    /// Fetch current funding rates for multiple trading pairs.
    pub async fn fetch_funding_rates(
        &self,
        symbols: Option<Vec<String>>,
        params: Option<BTreeMap<String, String>>,
    ) -> Result<BTreeMap<String, FeeFundingRate>> {
        self.load_markets(false).await?;

        let mut request_params = BTreeMap::new();

        if let Some(p) = params {
            for (key, value) in p {
                request_params.insert(key, value);
            }
        }

        let url = format!("{}/premiumIndex", self.urls().fapi_public);

        let mut request_url = url.clone();
        if !request_params.is_empty() {
            let query = crate::binance::signed_request::build_query_string(&request_params);
            request_url = format!("{}?{}", url, query);
        }

        let response = self.public_get(&request_url, None).await?;

        let mut rates = BTreeMap::new();

        if let Some(rates_array) = response.as_array() {
            for rate_data in rates_array {
                if let Ok(symbol_id) = rate_data["symbol"]
                    .as_str()
                    .ok_or_else(|| Error::from(ParseError::missing_field("symbol")))
                {
                    if let Ok(market) = self.base().market_by_id(symbol_id).await {
                        if let Some(ref syms) = symbols {
                            if !syms.contains(&market.symbol) {
                                continue;
                            }
                        }

                        if let Ok(rate) = parser::parse_funding_rate(rate_data, Some(&market)) {
                            rates.insert(market.symbol.clone(), rate);
                        }
                    }
                }
            }
        } else if let Ok(symbol_id) = response["symbol"]
            .as_str()
            .ok_or_else(|| Error::from(ParseError::missing_field("symbol")))
        {
            if let Ok(market) = self.base().market_by_id(symbol_id).await {
                if let Ok(rate) = parser::parse_funding_rate(&response, Some(&market)) {
                    rates.insert(market.symbol.clone(), rate);
                }
            }
        }

        Ok(rates)
    }

    /// Fetch funding rate history for a trading pair.
    pub async fn fetch_funding_rate_history(
        &self,
        symbol: &str,
        since: Option<i64>,
        limit: Option<u32>,
        params: Option<HashMap<String, String>>,
    ) -> Result<Vec<FeeFundingRateHistory>> {
        self.load_markets(false).await?;
        let market = self.base().market(symbol).await?;

        if market.market_type != MarketType::Futures && market.market_type != MarketType::Swap {
            return Err(Error::invalid_request(
                "fetch_funding_rate_history() supports futures and swap markets only".to_string(),
            ));
        }

        let mut request_params = BTreeMap::new();
        request_params.insert("symbol".to_string(), market.id.clone());

        if let Some(s) = since {
            request_params.insert("startTime".to_string(), s.to_string());
        }

        if let Some(l) = limit {
            request_params.insert("limit".to_string(), l.to_string());
        }

        if let Some(p) = params {
            for (key, value) in p {
                request_params.insert(key, value);
            }
        }

        let url = if market.linear.unwrap_or(true) {
            format!("{}/fundingRate", self.urls().fapi_public)
        } else {
            format!("{}/fundingRate", self.urls().dapi_public)
        };

        let query = crate::binance::signed_request::build_query_string(&request_params);
        let request_url = if query.is_empty() {
            url
        } else {
            format!("{}?{}", url, query)
        };

        let response = self.public_get(&request_url, None).await?;

        let history_array = response.as_array().ok_or_else(|| {
            Error::from(ParseError::invalid_format(
                "data",
                "Expected array of funding rate history",
            ))
        })?;

        let mut history = Vec::new();
        for history_data in history_array {
            match parser::parse_funding_rate_history(history_data, Some(&market)) {
                Ok(record) => history.push(record),
                Err(e) => {
                    warn!(error = %e, "Failed to parse funding rate history");
                }
            }
        }

        Ok(history)
    }

    /// Fetch funding payment history for a trading pair.
    pub async fn fetch_funding_history(
        &self,
        symbol: Option<&str>,
        since: Option<i64>,
        limit: Option<u32>,
        params: Option<HashMap<String, String>>,
    ) -> Result<Value> {
        self.load_markets(false).await?;

        let url = format!("{}/income", self.urls().fapi_private);

        let mut request = self.signed_request(url);

        if let Some(sym) = symbol {
            let market = self.base().market(sym).await?;
            request = request.param("symbol", market.id.clone());
        }

        request = request
            .optional_param("startTime", since)
            .optional_param("limit", limit);

        // Convert HashMap to serde_json::Value for merge_json_params
        let json_params = params.map(|p| {
            serde_json::Value::Object(
                p.into_iter()
                    .map(|(k, v)| (k, serde_json::Value::String(v)))
                    .collect(),
            )
        });

        let data = request.merge_json_params(json_params).execute().await?;

        Ok(data)
    }
}
