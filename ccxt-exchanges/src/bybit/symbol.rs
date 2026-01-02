//! Bybit symbol converter implementation
//!
//! This module provides conversion between unified CCXT symbols and Bybit-specific
//! exchange IDs for spot, swap (perpetual), and futures markets.
//!
//! # Bybit Symbol Formats
//!
//! | Market Type | Unified Format | Bybit Format |
//! |-------------|----------------|--------------|
//! | Spot | BTC/USDT | BTCUSDT |
//! | Linear Swap | BTC/USDT:USDT | BTCUSDT |
//! | Inverse Swap | BTC/USD:BTC | BTCUSD |
//! | Linear Futures | BTC/USDT:USDT-241231 | BTCUSDT-31DEC24 |
//! | Inverse Futures | BTC/USD:BTC-241231 | BTCUSD-31DEC24 |
//!
//! # Example
//!
//! ```rust
//! use ccxt_core::types::symbol::{ParsedSymbol, ExpiryDate, SymbolMarketType};
//! use ccxt_exchanges::bybit::symbol::BybitSymbolConverter;
//!
//! // Convert spot symbol
//! let spot = ParsedSymbol::spot("BTC".to_string(), "USDT".to_string());
//! assert_eq!(BybitSymbolConverter::to_exchange_id(&spot), "BTCUSDT");
//!
//! // Convert linear swap symbol
//! let swap = ParsedSymbol::linear_swap("BTC".to_string(), "USDT".to_string());
//! assert_eq!(BybitSymbolConverter::to_exchange_id(&swap), "BTCUSDT");
//!
//! // Convert futures symbol
//! let expiry = ExpiryDate::new(24, 12, 31).unwrap();
//! let futures = ParsedSymbol::futures("BTC".to_string(), "USDT".to_string(), "USDT".to_string(), expiry);
//! assert_eq!(BybitSymbolConverter::to_exchange_id(&futures), "BTCUSDT-31DEC24");
//! ```

use ccxt_core::types::symbol::{ExpiryDate, ParsedSymbol, SymbolMarketType};

/// Month names for Bybit futures format
const MONTH_NAMES: [&str; 12] = [
    "JAN", "FEB", "MAR", "APR", "MAY", "JUN", "JUL", "AUG", "SEP", "OCT", "NOV", "DEC",
];

/// Bybit symbol converter
///
/// Provides methods to convert between unified CCXT symbols and Bybit-specific
/// exchange IDs.
pub struct BybitSymbolConverter;

impl BybitSymbolConverter {
    /// Convert a unified ParsedSymbol to Bybit exchange ID
    ///
    /// # Arguments
    ///
    /// * `parsed` - The parsed unified symbol
    ///
    /// # Returns
    ///
    /// Returns the Bybit-specific exchange ID string
    ///
    /// # Format Mapping
    ///
    /// - Spot: `BASE/QUOTE` → `BASEQUOTE` (e.g., "BTC/USDT" → "BTCUSDT")
    /// - Linear Swap: `BASE/QUOTE:QUOTE` → `BASEQUOTE` (e.g., "BTC/USDT:USDT" → "BTCUSDT")
    /// - Inverse Swap: `BASE/QUOTE:BASE` → `BASEQUOTE` (e.g., "BTC/USD:BTC" → "BTCUSD")
    /// - Linear Futures: `BASE/QUOTE:QUOTE-YYMMDD` → `BASEQUOTE-DDMMMYY` (e.g., "BTC/USDT:USDT-241231" → "BTCUSDT-31DEC24")
    /// - Inverse Futures: `BASE/QUOTE:BASE-YYMMDD` → `BASEQUOTE-DDMMMYY` (e.g., "BTC/USD:BTC-241231" → "BTCUSD-31DEC24")
    pub fn to_exchange_id(parsed: &ParsedSymbol) -> String {
        let base = &parsed.base;
        let quote = &parsed.quote;

        match parsed.market_type() {
            SymbolMarketType::Spot => {
                // Spot: BTCUSDT
                format!("{}{}", base, quote)
            }
            SymbolMarketType::Swap => {
                // Perpetual swap: BTCUSDT (both linear and inverse use same format)
                format!("{}{}", base, quote)
            }
            SymbolMarketType::Futures => {
                // Futures with expiry date: BTCUSDT-31DEC24
                if let Some(expiry) = parsed.expiry {
                    let date_str = Self::format_expiry_date(expiry);
                    format!("{}{}-{}", base, quote, date_str)
                } else {
                    // Fallback if no expiry (shouldn't happen for valid futures)
                    format!("{}{}", base, quote)
                }
            }
        }
    }

    /// Format expiry date in Bybit format (DDMMMYY)
    ///
    /// Example: ExpiryDate { year: 24, month: 12, day: 31 } → "31DEC24"
    fn format_expiry_date(expiry: ExpiryDate) -> String {
        let month_name = MONTH_NAMES[(expiry.month - 1) as usize];
        format!("{:02}{}{:02}", expiry.day, month_name, expiry.year)
    }

    /// Parse expiry date from Bybit format (DDMMMYY)
    ///
    /// Example: "31DEC24" → ExpiryDate { year: 24, month: 12, day: 31 }
    fn parse_expiry_date(date_str: &str) -> Result<ExpiryDate, String> {
        if date_str.len() != 7 {
            return Err(format!("Invalid Bybit date format: {}", date_str));
        }

        let day: u8 = date_str[0..2]
            .parse()
            .map_err(|_| format!("Invalid day in expiry: {}", date_str))?;

        let month_str = &date_str[2..5];
        let month = MONTH_NAMES
            .iter()
            .position(|&m| m == month_str)
            .map(|i| (i + 1) as u8)
            .ok_or_else(|| format!("Invalid month in expiry: {}", month_str))?;

        let year: u8 = date_str[5..7]
            .parse()
            .map_err(|_| format!("Invalid year in expiry: {}", date_str))?;

        ExpiryDate::new(year, month, day).map_err(|e| format!("Invalid expiry date: {}", e))
    }

    /// Convert a Bybit exchange ID to a unified ParsedSymbol
    ///
    /// # Arguments
    ///
    /// * `exchange_id` - The Bybit-specific exchange ID
    /// * `market_type` - The market type (spot, swap, futures)
    /// * `settle` - Optional settlement currency (required for derivatives)
    /// * `base` - Base currency (required since Bybit IDs don't have separators)
    /// * `quote` - Quote currency (required since Bybit IDs don't have separators)
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

                // Extract expiry date from exchange_id (format: BTCUSDT-31DEC24)
                let expiry = Self::extract_expiry_from_exchange_id(exchange_id)?;
                Ok(ParsedSymbol::futures(base, quote, settle, expiry))
            }
        }
    }

    /// Extract expiry date from Bybit exchange ID
    ///
    /// Bybit futures format: BTCUSDT-31DEC24
    fn extract_expiry_from_exchange_id(exchange_id: &str) -> Result<ExpiryDate, String> {
        // Find hyphen and extract date part
        if let Some(hyphen_pos) = exchange_id.rfind('-') {
            let date_part = &exchange_id[hyphen_pos + 1..];
            return Self::parse_expiry_date(date_part);
        }

        Err(format!(
            "Could not extract expiry date from exchange ID: {}",
            exchange_id
        ))
    }

    /// Check if a Bybit exchange ID represents a futures contract (with expiry)
    pub fn is_futures(exchange_id: &str) -> bool {
        if let Some(hyphen_pos) = exchange_id.rfind('-') {
            let suffix = &exchange_id[hyphen_pos + 1..];
            // Bybit futures format: DDMMMYY (7 characters)
            suffix.len() == 7
                && suffix[0..2].chars().all(|c| c.is_ascii_digit())
                && suffix[5..7].chars().all(|c| c.is_ascii_digit())
        } else {
            false
        }
    }

    /// Check if a Bybit exchange ID represents a spot or perpetual (no hyphen)
    pub fn is_spot_or_perpetual(exchange_id: &str) -> bool {
        !exchange_id.contains('-')
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
        assert_eq!(BybitSymbolConverter::to_exchange_id(&symbol), "BTCUSDT");
    }

    #[test]
    fn test_spot_to_exchange_id_various_pairs() {
        let eth_usdt = ParsedSymbol::spot("ETH".to_string(), "USDT".to_string());
        assert_eq!(BybitSymbolConverter::to_exchange_id(&eth_usdt), "ETHUSDT");

        let btc_usdc = ParsedSymbol::spot("BTC".to_string(), "USDC".to_string());
        assert_eq!(BybitSymbolConverter::to_exchange_id(&btc_usdc), "BTCUSDC");

        let sol_btc = ParsedSymbol::spot("SOL".to_string(), "BTC".to_string());
        assert_eq!(BybitSymbolConverter::to_exchange_id(&sol_btc), "SOLBTC");
    }

    #[test]
    fn test_linear_swap_to_exchange_id() {
        let symbol = ParsedSymbol::linear_swap("BTC".to_string(), "USDT".to_string());
        assert_eq!(BybitSymbolConverter::to_exchange_id(&symbol), "BTCUSDT");
    }

    #[test]
    fn test_inverse_swap_to_exchange_id() {
        let symbol = ParsedSymbol::inverse_swap("BTC".to_string(), "USD".to_string());
        assert_eq!(BybitSymbolConverter::to_exchange_id(&symbol), "BTCUSD");
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
            BybitSymbolConverter::to_exchange_id(&symbol),
            "BTCUSDT-31DEC24"
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
            BybitSymbolConverter::to_exchange_id(&symbol),
            "BTCUSD-15MAR25"
        );
    }

    #[test]
    fn test_futures_date_formatting() {
        // Test various months
        let jan = ExpiryDate::new(25, 1, 5).unwrap();
        let symbol_jan = ParsedSymbol::futures(
            "ETH".to_string(),
            "USDT".to_string(),
            "USDT".to_string(),
            jan,
        );
        assert_eq!(
            BybitSymbolConverter::to_exchange_id(&symbol_jan),
            "ETHUSDT-05JAN25"
        );

        let jun = ExpiryDate::new(24, 6, 28).unwrap();
        let symbol_jun = ParsedSymbol::futures(
            "ETH".to_string(),
            "USDT".to_string(),
            "USDT".to_string(),
            jun,
        );
        assert_eq!(
            BybitSymbolConverter::to_exchange_id(&symbol_jun),
            "ETHUSDT-28JUN24"
        );
    }

    // ========================================================================
    // from_exchange_id tests
    // ========================================================================

    #[test]
    fn test_spot_from_exchange_id() {
        let parsed = BybitSymbolConverter::from_exchange_id(
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
        let parsed = BybitSymbolConverter::from_exchange_id(
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
        let parsed = BybitSymbolConverter::from_exchange_id(
            "BTCUSDT-31DEC24",
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
    fn test_is_futures() {
        assert!(BybitSymbolConverter::is_futures("BTCUSDT-31DEC24"));
        assert!(BybitSymbolConverter::is_futures("ETHUSDT-15MAR25"));
        assert!(!BybitSymbolConverter::is_futures("BTCUSDT"));
        assert!(!BybitSymbolConverter::is_futures("BTCUSD"));
    }

    #[test]
    fn test_is_spot_or_perpetual() {
        assert!(BybitSymbolConverter::is_spot_or_perpetual("BTCUSDT"));
        assert!(BybitSymbolConverter::is_spot_or_perpetual("ETHBTC"));
        assert!(!BybitSymbolConverter::is_spot_or_perpetual(
            "BTCUSDT-31DEC24"
        ));
    }

    #[test]
    fn test_format_expiry_date() {
        let expiry = ExpiryDate::new(24, 12, 31).unwrap();
        assert_eq!(BybitSymbolConverter::format_expiry_date(expiry), "31DEC24");

        let expiry2 = ExpiryDate::new(25, 1, 5).unwrap();
        assert_eq!(BybitSymbolConverter::format_expiry_date(expiry2), "05JAN25");
    }

    #[test]
    fn test_parse_expiry_date() {
        let expiry = BybitSymbolConverter::parse_expiry_date("31DEC24").unwrap();
        assert_eq!(expiry.year, 24);
        assert_eq!(expiry.month, 12);
        assert_eq!(expiry.day, 31);

        let expiry2 = BybitSymbolConverter::parse_expiry_date("05JAN25").unwrap();
        assert_eq!(expiry2.year, 25);
        assert_eq!(expiry2.month, 1);
        assert_eq!(expiry2.day, 5);
    }
}
