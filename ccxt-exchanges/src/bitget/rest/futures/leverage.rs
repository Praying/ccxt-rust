//! Leverage operations for Bitget futures/swap.

use super::super::super::signed_request::HttpMethod;
use super::super::super::{Bitget, symbol::BitgetSymbolConverter};
use ccxt_core::{Error, ParseError, Result};

impl Bitget {
    /// Set leverage for a symbol.
    ///
    /// Uses Bitget POST `/api/v2/mix/account/set-leverage` endpoint.
    pub async fn set_leverage_impl(
        &self,
        symbol: &str,
        leverage: u32,
        margin_mode: Option<&str>,
    ) -> Result<()> {
        let exchange_id = BitgetSymbolConverter::unified_to_exchange(symbol);
        let product_type = BitgetSymbolConverter::product_type_from_symbol(symbol);
        let margin_coin = Self::margin_coin_from_symbol(symbol);

        let hold_side = if margin_mode == Some("isolated") {
            // For isolated mode, set for both sides
            "long"
        } else {
            // For cross mode, no holdSide needed
            ""
        };

        let mut builder = self
            .signed_request("/api/v2/mix/account/set-leverage")
            .method(HttpMethod::Post)
            .param("symbol", &exchange_id)
            .param("productType", product_type)
            .param("marginCoin", margin_coin)
            .param("leverage", leverage.to_string());

        if !hold_side.is_empty() {
            builder = builder.param("holdSide", hold_side);
        }

        let _data = builder.execute().await?;

        // If isolated mode, also set for short side
        if margin_mode == Some("isolated") {
            self.signed_request("/api/v2/mix/account/set-leverage")
                .method(HttpMethod::Post)
                .param("symbol", &exchange_id)
                .param("productType", product_type)
                .param("marginCoin", margin_coin)
                .param("leverage", leverage.to_string())
                .param("holdSide", "short")
                .execute()
                .await?;
        }

        Ok(())
    }

    /// Get current leverage for a symbol.
    ///
    /// Uses Bitget GET `/api/v2/mix/account/account` endpoint and extracts leverage.
    pub async fn get_leverage_impl(&self, symbol: &str) -> Result<u32> {
        let exchange_id = BitgetSymbolConverter::unified_to_exchange(symbol);
        let product_type = BitgetSymbolConverter::product_type_from_symbol(symbol);
        let margin_coin = Self::margin_coin_from_symbol(symbol);

        let data = self
            .signed_request("/api/v2/mix/account/account")
            .param("symbol", &exchange_id)
            .param("productType", product_type)
            .param("marginCoin", margin_coin)
            .execute()
            .await?;

        // Try to extract leverage from the account data
        let lever_str = data["data"]["crossMarginLeverage"]
            .as_str()
            .or_else(|| data["data"]["fixedLongLeverage"].as_str())
            .ok_or_else(|| Error::from(ParseError::missing_field("leverage")))?;

        lever_str.parse::<f64>().map(|v| v as u32).map_err(|_| {
            Error::from(ParseError::invalid_value(
                "leverage",
                format!("Cannot parse leverage: {}", lever_str),
            ))
        })
    }
}
