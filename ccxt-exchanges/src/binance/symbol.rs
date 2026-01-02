//! Binance symbol converter implementation
//!
//! This module provides conversion between unified CCXT symbols and Binance-specific
//! exchange IDs for spot, swap (perpetual), and futures markets.
//!
//! # Binance Symbol Formats
//!
//! | Market Type | Unified Format | Binance Format |
//! |-------------|----------------|----------------|
//! | Spot | BTC/USDT | BTCUSDT |
//! | Linear Swap | BTC/USDT:USDT | BTCUSDT |
//! | Inverse Swap | BTC/USD:BTC | BTCUSD_PERP |
//! | Linear Futures | BTC/USDT:USDT-241231 | BTCUSDT_241231 |
//! | Inverse Futures | BTC/USD:BTC-241231 | BTCUSD_241231 |
//!
//! # Example
//!
//! ```rust
//! use ccxt_core::types::symbol::{ParsedSymbol, ExpiryDate, SymbolMarketType};
//! use ccxt_exchanges::binance::symbol::BinanceSymbolConverter;
//!
//! // Convert spot symbol
//! let spot = ParsedSymbol::spot("BTC".to_string(), "USDT".to_string());
//! assert_eq!(BinanceSymbolConverter::to_exchange_id(&spot), "BTCUSDT");
//!
//! // Convert linear swap symbol
//! let swap = ParsedSymbol::linear_swap("BTC".to_string(), "USDT".to_string());
//! assert_eq!(BinanceSymbolConverter::to_exchange_id(&swap), "BTCUSDT");
//!
//! // Convert inverse swap symbol
//! let inverse = ParsedSymbol::inverse_swap("BTC".to_string(), "USD".to_string());
//! assert_eq!(BinanceSymbolConverter::to_exchange_id(&inverse), "BTCUSD_PERP");
//!
//! // Convert futures symbol
//! let expiry = ExpiryDate::new(24, 12, 31).unwrap();
//! let futures = ParsedSymbol::futures("BTC".to_string(), "USDT".to_string(), "USDT".to_string(), expiry);
//! assert_eq!(BinanceSymbolConverter::to_exchange_id(&futures), "BTCUSDT_241231");
//! ```

use ccxt_core::types::symbol::{ExpiryDate, ParsedSymbol, SymbolMarketType};

/// Binance symbol converter
///
/// Provides methods to convert between unified CCXT symbols and Binance-specific
/// exchange IDs.
pub struct BinanceSymbolConverter;

impl BinanceSymbolConverter {
    /// Convert a unified ParsedSymbol to Binance exchange ID
    ///
    /// # Arguments
    ///
    /// * `parsed` - The parsed unified symbol
    ///
    /// # Returns
    ///
    /// Returns the Binance-specific exchange ID string
    ///
    /// # Format Mapping
    ///
    /// - Spot: `BASE/QUOTE` → `BASEQUOTE` (e.g., "BTC/USDT" → "BTCUSDT")
    /// - Linear Swap: `BASE/QUOTE:QUOTE` → `BASEQUOTE` (e.g., "BTC/USDT:USDT" → "BTCUSDT")
    /// - Inverse Swap: `BASE/QUOTE:BASE` → `BASEQUOTE_PERP` (e.g., "BTC/USD:BTC" → "BTCUSD_PERP")
    /// - Linear Futures: `BASE/QUOTE:QUOTE-YYMMDD` → `BASEQUOTE_YYMMDD` (e.g., "BTC/USDT:USDT-241231" → "BTCUSDT_241231")
    /// - Inverse Futures: `BASE/QUOTE:BASE-YYMMDD` → `BASEQUOTE_YYMMDD` (e.g., "BTC/USD:BTC-241231" → "BTCUSD_241231")
    pub fn to_exchange_id(parsed: &ParsedSymbol) -> String {
        let base = &parsed.base;
        let quote = &parsed.quote;

        match parsed.market_type() {
            SymbolMarketType::Spot => {
                // Spot: BTCUSDT
                format!("{}{}", base, quote)
            }
            SymbolMarketType::Swap => {
                // Check if inverse (settle == base)
                if parsed.is_inverse() {
                    // Inverse perpetual: BTCUSD_PERP
                    format!("{}{}_PERP", base, quote)
                } else {
                    // Linear perpetual: BTCUSDT (same as spot format on Binance)
                    format!("{}{}", base, quote)
                }
            }
            SymbolMarketType::Futures => {
                // Futures with expiry date
                if let Some(ref expiry) = parsed.expiry {
                    let date_str =
                        format!("{:02}{:02}{:02}", expiry.year, expiry.month, expiry.day);
                    // Both linear and inverse futures use underscore + date
                    format!("{}{}_{}", base, quote, date_str)
                } else {
                    // Fallback if no expiry (shouldn't happen for valid futures)
                    format!("{}{}", base, quote)
                }
            }
        }
    }

    /// Convert a Binance exchange ID to a unified ParsedSymbol
    ///
    /// # Arguments
    ///
    /// * `exchange_id` - The Binance-specific exchange ID
    /// * `market_type` - The market type (spot, swap, futures)
    /// * `settle` - Optional settlement currency (required for derivatives)
    /// * `base` - Base currency (required since Binance IDs don't have separators)
    /// * `quote` - Quote currency (required since Binance IDs don't have separators)
    ///
    /// # Returns
    ///
    /// Returns `Ok(ParsedSymbol)` if conversion succeeds, or `Err` if the format is invalid
    pub fn from_exchange_id(
        exchange_id: &str,
        market_type: SymbolMarketType,
        settle: Option<&str>,
        base: &str,
        quote: &str,
    ) -> Result<ParsedSymbol, String> {
        let base = base.to_uppercase();
        let quote = quote.to_uppercase();

        match market_type {
            SymbolMarketType::Spot => Ok(ParsedSymbol::spot(base, quote)),
            SymbolMarketType::Swap => {
                let settle = settle
                    .map(str::to_uppercase)
                    .ok_or_else(|| "Settlement currency required for swap".to_string())?;
                Ok(ParsedSymbol::swap(base, quote, settle))
            }
            SymbolMarketType::Futures => {
                let settle = settle
                    .map(str::to_uppercase)
                    .ok_or_else(|| "Settlement currency required for futures".to_string())?;

                // Extract expiry date from exchange_id (format: BTCUSDT_241231)
                let expiry = Self::extract_expiry_from_exchange_id(exchange_id)?;
                Ok(ParsedSymbol::futures(base, quote, settle, expiry))
            }
        }
    }

    /// Extract expiry date from Binance exchange ID
    ///
    /// Binance futures format: BTCUSDT_241231
    fn extract_expiry_from_exchange_id(exchange_id: &str) -> Result<ExpiryDate, String> {
        // Find underscore and extract date part
        if let Some(underscore_pos) = exchange_id.rfind('_') {
            let date_part = &exchange_id[underscore_pos + 1..];

            // Check if it's a date (6 digits) and not "PERP"
            if date_part.len() == 6 && date_part.chars().all(|c| c.is_ascii_digit()) {
                let year: u8 = date_part[0..2]
                    .parse()
                    .map_err(|_| format!("Invalid year in expiry: {}", date_part))?;
                let month: u8 = date_part[2..4]
                    .parse()
                    .map_err(|_| format!("Invalid month in expiry: {}", date_part))?;
                let day: u8 = date_part[4..6]
                    .parse()
                    .map_err(|_| format!("Invalid day in expiry: {}", date_part))?;

                return ExpiryDate::new(year, month, day)
                    .map_err(|e| format!("Invalid expiry date: {}", e));
            }
        }

        Err(format!(
            "Could not extract expiry date from exchange ID: {}",
            exchange_id
        ))
    }

    /// Check if a Binance exchange ID represents a perpetual contract
    pub fn is_perpetual(exchange_id: &str) -> bool {
        exchange_id.ends_with("_PERP")
    }

    /// Check if a Binance exchange ID represents a futures contract (with expiry)
    pub fn is_futures(exchange_id: &str) -> bool {
        if let Some(underscore_pos) = exchange_id.rfind('_') {
            let suffix = &exchange_id[underscore_pos + 1..];
            suffix.len() == 6 && suffix.chars().all(|c| c.is_ascii_digit())
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // to_exchange_id tests
    // ========================================================================

    #[test]
    fn test_spot_to_exchange_id() {
        let symbol = ParsedSymbol::spot("BTC".to_string(), "USDT".to_string());
        assert_eq!(BinanceSymbolConverter::to_exchange_id(&symbol), "BTCUSDT");
    }

    #[test]
    fn test_spot_to_exchange_id_various_pairs() {
        let eth_usdt = ParsedSymbol::spot("ETH".to_string(), "USDT".to_string());
        assert_eq!(BinanceSymbolConverter::to_exchange_id(&eth_usdt), "ETHUSDT");

        let btc_busd = ParsedSymbol::spot("BTC".to_string(), "BUSD".to_string());
        assert_eq!(BinanceSymbolConverter::to_exchange_id(&btc_busd), "BTCBUSD");

        let sol_btc = ParsedSymbol::spot("SOL".to_string(), "BTC".to_string());
        assert_eq!(BinanceSymbolConverter::to_exchange_id(&sol_btc), "SOLBTC");
    }

    #[test]
    fn test_linear_swap_to_exchange_id() {
        let symbol = ParsedSymbol::linear_swap("BTC".to_string(), "USDT".to_string());
        assert_eq!(BinanceSymbolConverter::to_exchange_id(&symbol), "BTCUSDT");
    }

    #[test]
    fn test_inverse_swap_to_exchange_id() {
        let symbol = ParsedSymbol::inverse_swap("BTC".to_string(), "USD".to_string());
        assert_eq!(
            BinanceSymbolConverter::to_exchange_id(&symbol),
            "BTCUSD_PERP"
        );
    }

    #[test]
    fn test_linear_futures_to_exchange_id() {
        let expiry = ExpiryDate::new(24, 12, 31).unwrap();
        let symbol = ParsedSymbol::futures(
            "BTC".to_string(),
            "USDT".to_string(),
            "USDT".to_string(),
            expiry,
        );
        assert_eq!(
            BinanceSymbolConverter::to_exchange_id(&symbol),
            "BTCUSDT_241231"
        );
    }

    #[test]
    fn test_inverse_futures_to_exchange_id() {
        let expiry = ExpiryDate::new(25, 3, 15).unwrap();
        let symbol = ParsedSymbol::futures(
            "BTC".to_string(),
            "USD".to_string(),
            "BTC".to_string(),
            expiry,
        );
        assert_eq!(
            BinanceSymbolConverter::to_exchange_id(&symbol),
            "BTCUSD_250315"
        );
    }

    #[test]
    fn test_futures_date_padding() {
        let expiry = ExpiryDate::new(25, 1, 5).unwrap();
        let symbol = ParsedSymbol::futures(
            "ETH".to_string(),
            "USDT".to_string(),
            "USDT".to_string(),
            expiry,
        );
        assert_eq!(
            BinanceSymbolConverter::to_exchange_id(&symbol),
            "ETHUSDT_250105"
        );
    }

    // ========================================================================
    // from_exchange_id tests
    // ========================================================================

    #[test]
    fn test_spot_from_exchange_id() {
        let parsed = BinanceSymbolConverter::from_exchange_id(
            "BTCUSDT",
            SymbolMarketType::Spot,
            None,
            "BTC",
            "USDT",
        )
        .unwrap();
        assert_eq!(parsed.base, "BTC");
        assert_eq!(parsed.quote, "USDT");
        assert!(parsed.settle.is_none());
        assert!(parsed.is_spot());
    }

    #[test]
    fn test_swap_from_exchange_id() {
        let parsed = BinanceSymbolConverter::from_exchange_id(
            "BTCUSDT",
            SymbolMarketType::Swap,
            Some("USDT"),
            "BTC",
            "USDT",
        )
        .unwrap();
        assert_eq!(parsed.base, "BTC");
        assert_eq!(parsed.quote, "USDT");
        assert_eq!(parsed.settle, Some("USDT".to_string()));
        assert!(parsed.is_swap());
    }

    #[test]
    fn test_futures_from_exchange_id() {
        let parsed = BinanceSymbolConverter::from_exchange_id(
            "BTCUSDT_241231",
            SymbolMarketType::Futures,
            Some("USDT"),
            "BTC",
            "USDT",
        )
        .unwrap();
        assert_eq!(parsed.base, "BTC");
        assert_eq!(parsed.quote, "USDT");
        assert_eq!(parsed.settle, Some("USDT".to_string()));
        assert!(parsed.is_futures());
        let expiry = parsed.expiry.unwrap();
        assert_eq!(expiry.year, 24);
        assert_eq!(expiry.month, 12);
        assert_eq!(expiry.day, 31);
    }

    // ========================================================================
    // Helper function tests
    // ========================================================================

    #[test]
    fn test_is_perpetual() {
        assert!(BinanceSymbolConverter::is_perpetual("BTCUSD_PERP"));
        assert!(BinanceSymbolConverter::is_perpetual("ETHUSD_PERP"));
        assert!(!BinanceSymbolConverter::is_perpetual("BTCUSDT"));
        assert!(!BinanceSymbolConverter::is_perpetual("BTCUSDT_241231"));
    }

    #[test]
    fn test_is_futures() {
        assert!(BinanceSymbolConverter::is_futures("BTCUSDT_241231"));
        assert!(BinanceSymbolConverter::is_futures("ETHUSDT_250315"));
        assert!(!BinanceSymbolConverter::is_futures("BTCUSDT"));
        assert!(!BinanceSymbolConverter::is_futures("BTCUSD_PERP"));
    }
}
