//! Account operations for OKX REST API.

use super::super::{Okx, parser};
use ccxt_core::{
    Error, ParseError, Result,
    types::{Balance, Trade},
};
use std::collections::HashMap;
use tracing::warn;

impl Okx {
    /// Fetch account balances.
    ///
    /// # Returns
    ///
    /// Returns a [`Balance`] structure with all currency balances.
    ///
    /// # Errors
    ///
    /// Returns an error if authentication fails or the API request fails.
    pub async fn fetch_balance(&self) -> Result<Balance> {
        let path = Self::build_api_path("/account/balance");
        let response = self.signed_request(&path).execute().await?;

        let data = response
            .get("data")
            .ok_or_else(|| Error::from(ParseError::missing_field("data")))?;

        let balances = data.as_array().ok_or_else(|| {
            Error::from(ParseError::invalid_format(
                "data",
                "Expected array of balances",
            ))
        })?;

        if balances.is_empty() {
            return Ok(Balance {
                balances: HashMap::new(),
                info: HashMap::new(),
            });
        }

        parser::parse_balance(&balances[0])
    }

    /// Fetch user's trade history.
    ///
    /// # Arguments
    ///
    /// * `symbol` - Trading pair symbol.
    /// * `since` - Optional start timestamp in milliseconds.
    /// * `limit` - Optional limit on number of trades (maximum: 100).
    ///
    /// # Returns
    ///
    /// Returns a vector of [`Trade`] structures representing user's trade history.
    pub async fn fetch_my_trades(
        &self,
        symbol: &str,
        since: Option<i64>,
        limit: Option<u32>,
    ) -> Result<Vec<Trade>> {
        let market = self.base().market(symbol).await?;

        let path = Self::build_api_path("/trade/fills");

        let actual_limit = limit.map_or(100, |l| l.min(100));

        let mut builder = self
            .signed_request(&path)
            .param("instId", &market.id)
            .param("instType", self.get_inst_type())
            .param("limit", actual_limit);

        if let Some(start_time) = since {
            builder = builder.param("begin", start_time);
        }

        let response = builder.execute().await?;

        let data = response
            .get("data")
            .ok_or_else(|| Error::from(ParseError::missing_field("data")))?;

        let trades_array = data.as_array().ok_or_else(|| {
            Error::from(ParseError::invalid_format(
                "data",
                "Expected array of trades",
            ))
        })?;

        let mut trades = Vec::new();
        for trade_data in trades_array {
            match parser::parse_trade(trade_data, Some(&market)) {
                Ok(trade) => trades.push(trade),
                Err(e) => {
                    warn!(error = %e, "Failed to parse my trade");
                }
            }
        }

        Ok(trades)
    }
}
