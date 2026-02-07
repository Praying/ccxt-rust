//! Position management methods for Bitget futures/swap.

use super::super::super::{Bitget, parser, symbol::BitgetSymbolConverter};
use ccxt_core::{Error, ParseError, Result, types::Position};
use tracing::warn;

impl Bitget {
    /// Fetch a single position for a symbol.
    ///
    /// Uses Bitget GET `/api/v2/mix/position/single-position` endpoint.
    pub async fn fetch_position_impl(&self, symbol: &str) -> Result<Position> {
        let exchange_id = BitgetSymbolConverter::unified_to_exchange(symbol);
        let product_type = BitgetSymbolConverter::product_type_from_symbol(symbol);

        let data = self
            .signed_request("/api/v2/mix/position/single-position")
            .param("symbol", &exchange_id)
            .param("productType", product_type)
            .param("marginCoin", Self::margin_coin_from_symbol(symbol))
            .execute()
            .await?;

        let positions_array = data["data"].as_array().ok_or_else(|| {
            Error::from(ParseError::invalid_format("data", "Expected data array"))
        })?;

        if positions_array.is_empty() {
            return Err(Error::from(ParseError::missing_field_owned(format!(
                "No position found for symbol: {}",
                symbol
            ))));
        }

        parser::parse_position(&positions_array[0], symbol)
    }

    /// Fetch positions for specific symbols, or all positions if empty.
    ///
    /// Uses Bitget GET `/api/v2/mix/position/all-position` endpoint.
    pub async fn fetch_positions_impl(&self, symbols: &[&str]) -> Result<Vec<Position>> {
        let product_type = if symbols.is_empty() {
            self.options().effective_product_type()
        } else {
            BitgetSymbolConverter::product_type_from_symbol(symbols[0])
        };

        let margin_coin = if symbols.is_empty() {
            "USDT".to_string()
        } else {
            Self::margin_coin_from_symbol(symbols[0]).to_string()
        };

        let data = self
            .signed_request("/api/v2/mix/position/all-position")
            .param("productType", product_type)
            .param("marginCoin", &margin_coin)
            .execute()
            .await?;

        let positions_array = data["data"].as_array().ok_or_else(|| {
            Error::from(ParseError::invalid_format("data", "Expected data array"))
        })?;

        let mut positions = Vec::new();
        for position_data in positions_array {
            let bitget_symbol = position_data["symbol"].as_str().unwrap_or_default();
            let symbol_hint =
                BitgetSymbolConverter::exchange_to_unified_hint(bitget_symbol, product_type);

            match parser::parse_position(position_data, &symbol_hint) {
                Ok(position) => {
                    if symbols.is_empty() || symbols.contains(&position.symbol.as_str()) {
                        positions.push(position);
                    }
                }
                Err(e) => {
                    warn!(error = %e, symbol = %bitget_symbol, "Failed to parse Bitget position");
                }
            }
        }

        Ok(positions)
    }

    /// Extract margin coin from a unified symbol.
    ///
    /// - "BTC/USDT:USDT" → "USDT"
    /// - "BTC/USD:BTC" → "BTC"
    /// - "BTC/USDT" → "USDT" (fallback to quote)
    pub(crate) fn margin_coin_from_symbol(symbol: &str) -> &str {
        if let Some(pos) = symbol.find(':') {
            let settle_part = &symbol[pos + 1..];
            // Strip expiry date if present
            if let Some(dash_pos) = settle_part.find('-') {
                &settle_part[..dash_pos]
            } else {
                settle_part
            }
        } else if let Some(pos) = symbol.find('/') {
            &symbol[pos + 1..]
        } else {
            "USDT"
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_margin_coin_from_symbol() {
        assert_eq!(Bitget::margin_coin_from_symbol("BTC/USDT:USDT"), "USDT");
        assert_eq!(Bitget::margin_coin_from_symbol("BTC/USD:BTC"), "BTC");
        assert_eq!(Bitget::margin_coin_from_symbol("BTC/USDT"), "USDT");
        assert_eq!(
            Bitget::margin_coin_from_symbol("BTC/USDT:USDT-241231"),
            "USDT"
        );
    }
}
