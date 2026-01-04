//! Funding rate operations for Binance futures.

use super::super::super::{Binance, parser};
use ccxt_core::{
    Error, ParseError, Result,
    types::{FeeFundingRate, FeeFundingRateHistory, MarketType},
};
use serde_json::Value;
use std::collections::{BTreeMap, HashMap};
use std::fmt::Write;
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

        let mut request_url = format!("{}?", url);
        for (key, value) in &request_params {
            let _ = write!(request_url, "{}={}&", key, value);
        }

        let response = self.base().http_client.get(&request_url, None).await?;

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
            request_url.push('?');
            for (key, value) in &request_params {
                let _ = write!(request_url, "{}={}&", key, value);
            }
        }

        let response = self.base().http_client.get(&request_url, None).await?;

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

        let mut request_url = format!("{}?", url);
        for (key, value) in &request_params {
            let _ = write!(request_url, "{}={}&", key, value);
        }

        let response = self.base().http_client.get(&request_url, None).await?;

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
        self.check_required_credentials()?;
        self.load_markets(false).await?;

        let mut request_params = BTreeMap::new();

        if let Some(sym) = symbol {
            let market = self.base().market(sym).await?;
            request_params.insert("symbol".to_string(), market.id.clone());
        }

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

        let timestamp = self.get_signing_timestamp().await?;
        let auth = self.get_auth()?;
        let signed_params =
            auth.sign_with_timestamp(&request_params, timestamp, Some(self.options().recv_window))?;

        let query_string: String = signed_params
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<_>>()
            .join("&");

        let url = format!("{}/income?{}", self.urls().fapi_private, query_string);

        let mut headers = reqwest::header::HeaderMap::new();
        auth.add_auth_headers_reqwest(&mut headers);

        let data = self.base().http_client.get(&url, Some(headers)).await?;

        Ok(data)
    }
}
