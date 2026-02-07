//! Position management methods for OKX futures/swap.

use super::super::super::{Okx, parser};
use ccxt_core::{Error, ParseError, Result, types::Position};
use tracing::warn;

impl Okx {
    /// Map a unified symbol to OKX instType parameter.
    ///
    /// Returns the appropriate instType for the given symbol:
    /// - Symbols with `:` and expiry date → "FUTURES"
    /// - Symbols with `:` (perpetual) → "SWAP"
    /// - Otherwise → "SWAP" (default for contract queries)
    pub(crate) fn inst_type_from_symbol(symbol: &str) -> &'static str {
        if symbol.contains(':') {
            // Check if it has an expiry date (e.g., BTC/USDT:USDT-241231)
            if symbol.contains('-')
                && symbol
                    .rsplit('-')
                    .next()
                    .is_some_and(|s| s.len() == 6 && s.chars().all(|c| c.is_ascii_digit()))
            {
                "FUTURES"
            } else {
                "SWAP"
            }
        } else {
            "SWAP"
        }
    }

    /// Convert a unified symbol to OKX instId.
    ///
    /// Examples:
    /// - "BTC/USDT:USDT" → "BTC-USDT-SWAP"
    /// - "BTC/USD:BTC" → "BTC-USD-SWAP"
    /// - "BTC/USDT" → "BTC-USDT"
    pub(crate) fn symbol_to_inst_id(symbol: &str) -> String {
        if let Some(pos) = symbol.find(':') {
            let base_quote = &symbol[..pos];
            let settle_part = &symbol[pos + 1..];
            let base_quote_dash = base_quote.replace('/', "-");

            if let Some(dash_pos) = settle_part.find('-') {
                // Futures with expiry: BTC/USDT:USDT-241231 → BTC-USDT-241231
                let expiry = &settle_part[dash_pos + 1..];
                format!("{}-{}", base_quote_dash, expiry)
            } else {
                // Perpetual swap: BTC/USDT:USDT → BTC-USDT-SWAP
                format!("{}-SWAP", base_quote_dash)
            }
        } else {
            // Spot: BTC/USDT → BTC-USDT
            symbol.replace('/', "-")
        }
    }

    /// Fetch a single position for a symbol.
    ///
    /// Uses OKX GET `/api/v5/account/positions` endpoint.
    pub async fn fetch_position_impl(&self, symbol: &str) -> Result<Position> {
        let inst_id = Self::symbol_to_inst_id(symbol);
        let inst_type = Self::inst_type_from_symbol(symbol);

        let data = self
            .signed_request("/api/v5/account/positions")
            .param("instId", &inst_id)
            .param("instType", inst_type)
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
    /// Uses OKX GET `/api/v5/account/positions` endpoint.
    pub async fn fetch_positions_impl(&self, symbols: &[&str]) -> Result<Vec<Position>> {
        let inst_type = if symbols.is_empty() {
            // Fetch all — use the configured default type
            self.get_inst_type()
        } else {
            Self::inst_type_from_symbol(symbols[0])
        };

        let mut builder = self
            .signed_request("/api/v5/account/positions")
            .param("instType", inst_type);

        // OKX supports filtering by instId (comma-separated for up to 10)
        if !symbols.is_empty() && symbols.len() <= 10 {
            let inst_ids: Vec<String> =
                symbols.iter().map(|s| Self::symbol_to_inst_id(s)).collect();
            builder = builder.param("instId", inst_ids.join(","));
        }

        let data = builder.execute().await?;

        let positions_array = data["data"].as_array().ok_or_else(|| {
            Error::from(ParseError::invalid_format("data", "Expected data array"))
        })?;

        let mut positions = Vec::new();
        for position_data in positions_array {
            let inst_id = position_data["instId"].as_str().unwrap_or_default();
            // Derive symbol from instId for parsing
            let symbol_hint = Self::inst_id_to_symbol_hint(inst_id);

            match parser::parse_position(position_data, &symbol_hint) {
                Ok(position) => {
                    // Filter by requested symbols if specified
                    if symbols.is_empty() || symbols.contains(&position.symbol.as_str()) {
                        positions.push(position);
                    }
                }
                Err(e) => {
                    warn!(error = %e, inst_id = %inst_id, "Failed to parse OKX position");
                }
            }
        }

        Ok(positions)
    }

    /// Convert an OKX instId back to a rough unified symbol hint.
    ///
    /// This is a best-effort conversion used when market data isn't loaded.
    /// - "BTC-USDT-SWAP" → "BTC/USDT:USDT"
    /// - "BTC-USD-SWAP" → "BTC/USD:BTC"
    /// - "BTC-USDT-241231" → "BTC/USDT:USDT-241231"
    /// - "BTC-USDT" → "BTC/USDT"
    fn inst_id_to_symbol_hint(inst_id: &str) -> String {
        let parts: Vec<&str> = inst_id.split('-').collect();
        match parts.len() {
            2 => {
                // Spot: BTC-USDT → BTC/USDT
                format!("{}/{}", parts[0], parts[1])
            }
            3 if parts[2] == "SWAP" => {
                // Perpetual: BTC-USDT-SWAP → BTC/USDT:USDT or BTC/USD:BTC
                let settle = if parts[1] == "USD" {
                    parts[0]
                } else {
                    parts[1]
                };
                format!("{}/{}:{}", parts[0], parts[1], settle)
            }
            3 => {
                // Futures with expiry: BTC-USDT-241231 → BTC/USDT:USDT-241231
                let settle = if parts[1] == "USD" {
                    parts[0]
                } else {
                    parts[1]
                };
                format!("{}/{}:{}-{}", parts[0], parts[1], settle, parts[2])
            }
            _ => inst_id.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inst_type_from_symbol() {
        assert_eq!(Okx::inst_type_from_symbol("BTC/USDT:USDT"), "SWAP");
        assert_eq!(Okx::inst_type_from_symbol("BTC/USD:BTC"), "SWAP");
        assert_eq!(
            Okx::inst_type_from_symbol("BTC/USDT:USDT-241231"),
            "FUTURES"
        );
        assert_eq!(Okx::inst_type_from_symbol("BTC/USDT"), "SWAP");
    }

    #[test]
    fn test_symbol_to_inst_id() {
        assert_eq!(Okx::symbol_to_inst_id("BTC/USDT:USDT"), "BTC-USDT-SWAP");
        assert_eq!(Okx::symbol_to_inst_id("BTC/USD:BTC"), "BTC-USD-SWAP");
        assert_eq!(
            Okx::symbol_to_inst_id("BTC/USDT:USDT-241231"),
            "BTC-USDT-241231"
        );
        assert_eq!(Okx::symbol_to_inst_id("BTC/USDT"), "BTC-USDT");
        assert_eq!(Okx::symbol_to_inst_id("ETH/USD:ETH"), "ETH-USD-SWAP");
    }

    #[test]
    fn test_inst_id_to_symbol_hint() {
        assert_eq!(
            Okx::inst_id_to_symbol_hint("BTC-USDT-SWAP"),
            "BTC/USDT:USDT"
        );
        assert_eq!(Okx::inst_id_to_symbol_hint("BTC-USD-SWAP"), "BTC/USD:BTC");
        assert_eq!(
            Okx::inst_id_to_symbol_hint("BTC-USDT-241231"),
            "BTC/USDT:USDT-241231"
        );
        assert_eq!(Okx::inst_id_to_symbol_hint("BTC-USDT"), "BTC/USDT");
    }
}
