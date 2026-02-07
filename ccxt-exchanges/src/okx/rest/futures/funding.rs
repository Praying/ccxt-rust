//! Funding rate operations for OKX futures/swap.

use super::super::super::{Okx, parser};
use ccxt_core::{
    Error, ParseError, Result,
    types::{FundingRate, FundingRateHistory},
};
use std::collections::HashMap;
use tracing::warn;

impl Okx {
    /// Fetch current funding rate for a symbol.
    ///
    /// Uses OKX GET `/api/v5/public/funding-rate` (public endpoint).
    pub async fn fetch_funding_rate_impl(&self, symbol: &str) -> Result<FundingRate> {
        let inst_id = Self::symbol_to_inst_id(symbol);

        let mut params = HashMap::new();
        params.insert("instId".to_string(), inst_id);

        let data = self
            .public_request("GET", "/api/v5/public/funding-rate", Some(&params))
            .await?;

        let rates_array = data["data"].as_array().ok_or_else(|| {
            Error::from(ParseError::invalid_format("data", "Expected data array"))
        })?;

        if rates_array.is_empty() {
            return Err(Error::from(ParseError::missing_field_owned(format!(
                "No funding rate found for symbol: {}",
                symbol
            ))));
        }

        parser::parse_funding_rate(&rates_array[0], symbol)
    }

    /// Fetch funding rates for multiple symbols.
    ///
    /// Calls `fetch_funding_rate_impl` for each symbol.
    pub async fn fetch_funding_rates_impl(&self, symbols: &[&str]) -> Result<Vec<FundingRate>> {
        let mut rates = Vec::new();
        for symbol in symbols {
            match self.fetch_funding_rate_impl(symbol).await {
                Ok(rate) => rates.push(rate),
                Err(e) => {
                    warn!(error = %e, symbol = %symbol, "Failed to fetch funding rate");
                }
            }
        }
        Ok(rates)
    }

    /// Fetch funding rate history for a symbol.
    ///
    /// Uses OKX GET `/api/v5/public/funding-rate-history` (public endpoint).
    pub async fn fetch_funding_rate_history_impl(
        &self,
        symbol: &str,
        since: Option<i64>,
        limit: Option<u32>,
    ) -> Result<Vec<FundingRateHistory>> {
        let inst_id = Self::symbol_to_inst_id(symbol);

        let mut params = HashMap::new();
        params.insert("instId".to_string(), inst_id);

        if let Some(s) = since {
            params.insert("before".to_string(), s.to_string());
        }

        if let Some(l) = limit {
            params.insert("limit".to_string(), l.to_string());
        }

        let data = self
            .public_request("GET", "/api/v5/public/funding-rate-history", Some(&params))
            .await?;

        let history_array = data["data"].as_array().ok_or_else(|| {
            Error::from(ParseError::invalid_format("data", "Expected data array"))
        })?;

        let mut history = Vec::new();
        for item in history_array {
            match parser::parse_funding_rate_history(item, symbol) {
                Ok(record) => history.push(record),
                Err(e) => {
                    warn!(error = %e, "Failed to parse OKX funding rate history");
                }
            }
        }

        Ok(history)
    }
}
