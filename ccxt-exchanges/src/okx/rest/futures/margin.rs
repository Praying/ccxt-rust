//! Margin mode operations for OKX futures/swap.

use super::super::super::{Okx, signed_request::HttpMethod};
use ccxt_core::{Error, Result};

impl Okx {
    /// Set margin mode (cross or isolated) for a symbol.
    ///
    /// Uses OKX POST `/api/v5/account/set-position-mode` for position mode,
    /// and the leverage endpoint implicitly sets margin mode.
    ///
    /// Note: OKX sets margin mode per-instrument via the set-leverage endpoint's
    /// `mgnMode` parameter. This method provides a dedicated interface.
    pub async fn set_margin_mode_impl(&self, symbol: &str, mode: &str) -> Result<()> {
        let inst_id = Self::symbol_to_inst_id(symbol);

        let mgn_mode = match mode.to_lowercase().as_str() {
            "isolated" => "isolated",
            "cross" | "crossed" => "cross",
            _ => {
                return Err(Error::invalid_request(format!(
                    "Invalid margin mode: {}. Must be 'isolated' or 'cross'",
                    mode
                )));
            }
        };

        // OKX sets margin mode through the set-leverage endpoint
        // We need to get current leverage first, then re-set it with the new margin mode
        let current_leverage = self.get_leverage_impl(symbol).await.unwrap_or(10);

        let mut builder = self
            .signed_request("/api/v5/account/set-leverage")
            .method(HttpMethod::Post)
            .param("instId", &inst_id)
            .param("lever", current_leverage.to_string())
            .param("mgnMode", mgn_mode);

        if mgn_mode == "isolated" {
            builder = builder.param("posSide", "net");
        }

        let data = builder.execute().await?;

        // Check for nested error
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
}
