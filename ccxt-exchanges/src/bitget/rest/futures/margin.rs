//! Margin mode operations for Bitget futures/swap.

use super::super::super::signed_request::HttpMethod;
use super::super::super::{Bitget, symbol::BitgetSymbolConverter};
use ccxt_core::{Error, Result};

impl Bitget {
    /// Set margin mode (cross or isolated) for a symbol.
    ///
    /// Uses Bitget POST `/api/v2/mix/account/set-margin-mode` endpoint.
    pub async fn set_margin_mode_impl(&self, symbol: &str, mode: &str) -> Result<()> {
        let exchange_id = BitgetSymbolConverter::unified_to_exchange(symbol);
        let product_type = BitgetSymbolConverter::product_type_from_symbol(symbol);
        let margin_coin = Self::margin_coin_from_symbol(symbol);

        let margin_mode = match mode.to_lowercase().as_str() {
            "isolated" => "isolated",
            "cross" | "crossed" => "crossed",
            _ => {
                return Err(Error::invalid_request(format!(
                    "Invalid margin mode: {}. Must be 'isolated' or 'cross'",
                    mode
                )));
            }
        };

        self.signed_request("/api/v2/mix/account/set-margin-mode")
            .method(HttpMethod::Post)
            .param("symbol", &exchange_id)
            .param("productType", product_type)
            .param("marginCoin", margin_coin)
            .param("marginMode", margin_mode)
            .execute()
            .await?;

        Ok(())
    }
}
