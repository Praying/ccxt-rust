//! Bitget symbol converter implementation.
//!
//! This module provides conversion between unified CCXT symbols and Bitget-specific
//! exchange IDs for spot, swap (perpetual), and futures markets.
//!
//! # Bitget Symbol Formats
//!
//! | Market Type | Unified Format | Bitget Format | Product Type |
//! |-------------|----------------|---------------|--------------|
//! | Spot | BTC/USDT | BTCUSDT | spot |
//! | Linear Swap | BTC/USDT:USDT | BTCUSDT | USDT-FUTURES |
//! | Inverse Swap | BTC/USD:BTC | BTCUSD | COIN-FUTURES |
//!
//! # Example
//!
//! ```rust
//! use ccxt_exchanges::bitget::symbol::BitgetSymbolConverter;
//!
//! assert_eq!(BitgetSymbolConverter::unified_to_exchange("BTC/USDT"), "BTCUSDT");
//! assert_eq!(BitgetSymbolConverter::unified_to_exchange("BTC/USDT:USDT"), "BTCUSDT");
//! assert_eq!(BitgetSymbolConverter::product_type_from_symbol("BTC/USDT:USDT"), "USDT-FUTURES");
//! assert_eq!(BitgetSymbolConverter::product_type_from_symbol("BTC/USDT"), "spot");
//! ```

/// Bitget symbol converter.
///
/// Provides methods to convert between unified CCXT symbols and Bitget-specific
/// exchange IDs, and to determine the appropriate product type.
pub struct BitgetSymbolConverter;

impl BitgetSymbolConverter {
    /// Convert a unified CCXT symbol to Bitget exchange ID.
    ///
    /// Strips the settlement currency suffix and removes separators:
    /// - "BTC/USDT" → "BTCUSDT"
    /// - "BTC/USDT:USDT" → "BTCUSDT"
    /// - "BTC/USD:BTC" → "BTCUSD"
    /// - "BTC/USDT:USDT-241231" → "BTCUSDT"
    pub fn unified_to_exchange(symbol: &str) -> String {
        // Strip settlement part (everything after ':')
        let base_quote = if let Some(pos) = symbol.find(':') {
            &symbol[..pos]
        } else {
            symbol
        };

        // Remove the '/' separator
        base_quote.replace('/', "")
    }

    /// Determine the Bitget product type from a unified symbol.
    ///
    /// Returns:
    /// - "USDT-FUTURES" for linear perpetual/futures (e.g., "BTC/USDT:USDT")
    /// - "COIN-FUTURES" for inverse perpetual/futures (e.g., "BTC/USD:BTC")
    /// - "spot" for spot markets (e.g., "BTC/USDT")
    pub fn product_type_from_symbol(symbol: &str) -> &'static str {
        if let Some(pos) = symbol.find(':') {
            let settle_part = &symbol[pos + 1..];
            // Strip expiry date if present
            let settle = if let Some(dash_pos) = settle_part.find('-') {
                &settle_part[..dash_pos]
            } else {
                settle_part
            };

            // Extract quote currency
            let base_quote = &symbol[..pos];
            let quote = if let Some(slash_pos) = base_quote.find('/') {
                &base_quote[slash_pos + 1..]
            } else {
                ""
            };

            // If settle == quote, it's linear (USDT-FUTURES)
            // If settle != quote (settle == base), it's inverse (COIN-FUTURES)
            if settle == quote {
                "USDT-FUTURES"
            } else {
                "COIN-FUTURES"
            }
        } else {
            "spot"
        }
    }

    /// Check if a symbol is a contract (has settlement currency).
    pub fn is_contract(symbol: &str) -> bool {
        symbol.contains(':')
    }

    /// Check if a symbol is a spot market.
    pub fn is_spot(symbol: &str) -> bool {
        !symbol.contains(':')
    }

    /// Convert a Bitget exchange ID back to a rough unified symbol hint.
    ///
    /// This is a best-effort conversion:
    /// - "BTCUSDT" with product_type "USDT-FUTURES" → "BTC/USDT:USDT"
    /// - "BTCUSD" with product_type "COIN-FUTURES" → "BTC/USD:BTC"
    /// - "BTCUSDT" with product_type "spot" → "BTC/USDT"
    pub fn exchange_to_unified_hint(exchange_id: &str, product_type: &str) -> String {
        // Try to split the exchange ID into base and quote
        // Common quote currencies in order of length (longest first to avoid partial matches)
        let quote_currencies = ["USDT", "USDC", "USD", "BTC", "ETH"];

        for quote in &quote_currencies {
            if exchange_id.ends_with(quote) && exchange_id.len() > quote.len() {
                let base = &exchange_id[..exchange_id.len() - quote.len()];
                return match product_type {
                    "USDT-FUTURES" | "usdt-futures" => {
                        format!("{}/{}:{}", base, quote, quote)
                    }
                    "COIN-FUTURES" | "coin-futures" => {
                        format!("{}/{}:{}", base, quote, base)
                    }
                    _ => {
                        format!("{}/{}", base, quote)
                    }
                };
            }
        }

        // Fallback: return as-is
        exchange_id.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // unified_to_exchange tests
    // ========================================================================

    #[test]
    fn test_spot_to_exchange() {
        assert_eq!(
            BitgetSymbolConverter::unified_to_exchange("BTC/USDT"),
            "BTCUSDT"
        );
        assert_eq!(
            BitgetSymbolConverter::unified_to_exchange("ETH/USDT"),
            "ETHUSDT"
        );
        assert_eq!(
            BitgetSymbolConverter::unified_to_exchange("SOL/BTC"),
            "SOLBTC"
        );
    }

    #[test]
    fn test_linear_swap_to_exchange() {
        assert_eq!(
            BitgetSymbolConverter::unified_to_exchange("BTC/USDT:USDT"),
            "BTCUSDT"
        );
        assert_eq!(
            BitgetSymbolConverter::unified_to_exchange("ETH/USDT:USDT"),
            "ETHUSDT"
        );
    }

    #[test]
    fn test_inverse_swap_to_exchange() {
        assert_eq!(
            BitgetSymbolConverter::unified_to_exchange("BTC/USD:BTC"),
            "BTCUSD"
        );
    }

    #[test]
    fn test_futures_with_expiry_to_exchange() {
        assert_eq!(
            BitgetSymbolConverter::unified_to_exchange("BTC/USDT:USDT-241231"),
            "BTCUSDT"
        );
    }

    // ========================================================================
    // product_type_from_symbol tests
    // ========================================================================

    #[test]
    fn test_product_type_spot() {
        assert_eq!(
            BitgetSymbolConverter::product_type_from_symbol("BTC/USDT"),
            "spot"
        );
        assert_eq!(
            BitgetSymbolConverter::product_type_from_symbol("ETH/BTC"),
            "spot"
        );
    }

    #[test]
    fn test_product_type_linear() {
        assert_eq!(
            BitgetSymbolConverter::product_type_from_symbol("BTC/USDT:USDT"),
            "USDT-FUTURES"
        );
        assert_eq!(
            BitgetSymbolConverter::product_type_from_symbol("ETH/USDT:USDT"),
            "USDT-FUTURES"
        );
    }

    #[test]
    fn test_product_type_inverse() {
        assert_eq!(
            BitgetSymbolConverter::product_type_from_symbol("BTC/USD:BTC"),
            "COIN-FUTURES"
        );
        assert_eq!(
            BitgetSymbolConverter::product_type_from_symbol("ETH/USD:ETH"),
            "COIN-FUTURES"
        );
    }

    #[test]
    fn test_product_type_futures_with_expiry() {
        assert_eq!(
            BitgetSymbolConverter::product_type_from_symbol("BTC/USDT:USDT-241231"),
            "USDT-FUTURES"
        );
    }

    // ========================================================================
    // Helper function tests
    // ========================================================================

    #[test]
    fn test_is_contract() {
        assert!(BitgetSymbolConverter::is_contract("BTC/USDT:USDT"));
        assert!(BitgetSymbolConverter::is_contract("BTC/USD:BTC"));
        assert!(!BitgetSymbolConverter::is_contract("BTC/USDT"));
    }

    #[test]
    fn test_is_spot() {
        assert!(BitgetSymbolConverter::is_spot("BTC/USDT"));
        assert!(!BitgetSymbolConverter::is_spot("BTC/USDT:USDT"));
    }

    // ========================================================================
    // exchange_to_unified_hint tests
    // ========================================================================

    #[test]
    fn test_exchange_to_unified_spot() {
        assert_eq!(
            BitgetSymbolConverter::exchange_to_unified_hint("BTCUSDT", "spot"),
            "BTC/USDT"
        );
    }

    #[test]
    fn test_exchange_to_unified_linear() {
        assert_eq!(
            BitgetSymbolConverter::exchange_to_unified_hint("BTCUSDT", "USDT-FUTURES"),
            "BTC/USDT:USDT"
        );
    }

    #[test]
    fn test_exchange_to_unified_inverse() {
        assert_eq!(
            BitgetSymbolConverter::exchange_to_unified_hint("BTCUSD", "COIN-FUTURES"),
            "BTC/USD:BTC"
        );
    }
}
