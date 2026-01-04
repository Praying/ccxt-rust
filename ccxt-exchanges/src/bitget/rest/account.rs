//! Account operations for Bitget REST API.

use super::super::{Bitget, parser};
use ccxt_core::{
    Error, ParseError, Result,
    types::{Balance, Trade},
};
use tracing::warn;

impl Bitget {
    /// Fetch account balances.
    pub async fn fetch_balance(&self) -> Result<Balance> {
        let path = self.build_api_path("/account/assets");
        let response = self.signed_request(&path).execute().await?;

        let data = response
            .get("data")
            .ok_or_else(|| Error::from(ParseError::missing_field("data")))?;

        parser::parse_balance(data)
    }

    /// Fetch user's trade history.
    pub async fn fetch_my_trades(
        &self,
        symbol: &str,
        since: Option<i64>,
        limit: Option<u32>,
    ) -> Result<Vec<Trade>> {
        let market = self.base().market(symbol).await?;

        let path = self.build_api_path("/trade/fills");

        let actual_limit = limit.map_or(100, |l| l.min(500));

        let mut builder = self
            .signed_request(&path)
            .param("symbol", &market.id)
            .param("limit", actual_limit);

        if let Some(start_time) = since {
            builder = builder.param("startTime", start_time);
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
