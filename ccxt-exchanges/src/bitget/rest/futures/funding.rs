//! Funding rate operations for Bitget futures/swap.

use super::super::super::{Bitget, parser, symbol::BitgetSymbolConverter};
use ccxt_core::{
    Error, ParseError, Result,
    types::{FundingRate, FundingRateHistory},
};
use std::collections::HashMap;
use tracing::warn;

impl Bitget {
    /// Fetch current funding rate for a symbol.
    ///
    /// Uses Bitget GET `/api/v2/mix/market/current-fund-rate` (public endpoint).
    pub async fn fetch_funding_rate_impl(&self, symbol: &str) -> Result<FundingRate> {
        let exchange_id = BitgetSymbolConverter::unified_to_exchange(symbol);
        let product_type = BitgetSymbolConverter::product_type_from_symbol(symbol);

        let mut params = HashMap::new();
        params.insert("symbol".to_string(), exchange_id);
        params.insert("productType".to_string(), product_type.to_string());

        let data = self
            .public_request("GET", "/api/v2/mix/market/current-fund-rate", Some(&params))
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
    pub async fn fetch_funding_rates_impl(&self, symbols: &[&str]) -> Result<Vec<FundingRate>> {
        let mut rates = Vec::new();
        for symbol in symbols {
            match self.fetch_funding_rate_impl(symbol).await {
                Ok(rate) => rates.push(rate),
                Err(e) => {
                    warn!(error = %e, symbol = %symbol, "Failed to fetch Bitget funding rate");
                }
            }
        }
        Ok(rates)
    }

    /// Fetch funding rate history for a symbol.
    ///
    /// Uses Bitget GET `/api/v2/mix/market/history-fund-rate` (public endpoint).
    pub async fn fetch_funding_rate_history_impl(
        &self,
        symbol: &str,
        since: Option<i64>,
        limit: Option<u32>,
    ) -> Result<Vec<FundingRateHistory>> {
        let exchange_id = BitgetSymbolConverter::unified_to_exchange(symbol);
        let product_type = BitgetSymbolConverter::product_type_from_symbol(symbol);

        let mut params = HashMap::new();
        params.insert("symbol".to_string(), exchange_id);
        params.insert("productType".to_string(), product_type.to_string());

        if let Some(l) = limit {
            params.insert("pageSize".to_string(), l.to_string());
        }

        if let Some(s) = since {
            params.insert("startTime".to_string(), s.to_string());
        }

        let data = self
            .public_request("GET", "/api/v2/mix/market/history-fund-rate", Some(&params))
            .await?;

        let history_array = data["data"].as_array().ok_or_else(|| {
            Error::from(ParseError::invalid_format("data", "Expected data array"))
        })?;

        let mut history = Vec::new();
        for item in history_array {
            match parser::parse_funding_rate_history(item, symbol) {
                Ok(record) => history.push(record),
                Err(e) => {
                    warn!(error = %e, "Failed to parse Bitget funding rate history");
                }
            }
        }

        Ok(history)
    }
}
