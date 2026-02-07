//! Leverage operations for OKX futures/swap.

use super::super::super::{Okx, signed_request::HttpMethod};
use ccxt_core::{Error, ParseError, Result};

impl Okx {
    /// Set leverage for a symbol.
    ///
    /// Uses OKX POST `/api/v5/account/set-leverage` endpoint.
    ///
    /// # Arguments
    ///
    /// * `symbol` - Unified symbol (e.g., "BTC/USDT:USDT")
    /// * `leverage` - Leverage multiplier
    /// * `margin_mode` - "cross" or "isolated"
    pub async fn set_leverage_impl(
        &self,
        symbol: &str,
        leverage: u32,
        margin_mode: Option<&str>,
    ) -> Result<()> {
        let inst_id = Self::symbol_to_inst_id(symbol);
        let mgn_mode = margin_mode.unwrap_or("cross");

        let mut builder = self
            .signed_request("/api/v5/account/set-leverage")
            .method(HttpMethod::Post)
            .param("instId", &inst_id)
            .param("lever", leverage.to_string())
            .param("mgnMode", mgn_mode);

        // For isolated mode, we need to specify posSide
        if mgn_mode == "isolated" {
            // Set for both long and short sides
            builder = builder.param("posSide", "net");
        }

        let data = builder.execute().await?;

        // Check for nested error in data array
        if let Some(arr) = data["data"].as_array() {
            if let Some(first) = arr.first() {
                if let Some(code) = first["sCode"].as_str() {
                    if code != "0" {
                        let msg = first["sMsg"].as_str().unwrap_or("Unknown error");
                        return Err(Error::exchange(code, msg));
                    }
                }
            }
        }

        Ok(())
    }

    /// Get current leverage for a symbol.
    ///
    /// Uses OKX GET `/api/v5/account/leverage-info` endpoint.
    pub async fn get_leverage_impl(&self, symbol: &str) -> Result<u32> {
        let inst_id = Self::symbol_to_inst_id(symbol);

        let data = self
            .signed_request("/api/v5/account/leverage-info")
            .param("instId", &inst_id)
            .param("mgnMode", "cross")
            .execute()
            .await?;

        let leverage_array = data["data"].as_array().ok_or_else(|| {
            Error::from(ParseError::invalid_format("data", "Expected data array"))
        })?;

        if leverage_array.is_empty() {
            return Err(Error::from(ParseError::missing_field_owned(format!(
                "No leverage info found for symbol: {}",
                symbol
            ))));
        }

        let lever_str = leverage_array[0]["lever"]
            .as_str()
            .ok_or_else(|| Error::from(ParseError::missing_field("lever")))?;

        lever_str.parse::<f64>().map(|v| v as u32).map_err(|_| {
            Error::from(ParseError::invalid_value(
                "lever",
                format!("Cannot parse leverage: {}", lever_str),
            ))
        })
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_leverage_parse() {
        // Test that leverage string parsing works
        let lever_str = "10";
        let leverage: u32 = lever_str.parse::<f64>().unwrap() as u32;
        assert_eq!(leverage, 10);

        let lever_str = "5.00";
        let leverage: u32 = lever_str.parse::<f64>().unwrap() as u32;
        assert_eq!(leverage, 5);
    }
}
