//! Symbol formatter implementation
//!
//! This module provides formatting functionality for converting `ParsedSymbol`
//! structures back into unified symbol strings.
//!
//! # Symbol Format
//!
//! The unified symbol format follows the CCXT standard:
//! - **Spot**: `BASE/QUOTE` (e.g., "BTC/USDT")
//! - **Perpetual Swap**: `BASE/QUOTE:SETTLE` (e.g., "BTC/USDT:USDT")
//! - **Futures**: `BASE/QUOTE:SETTLE-YYMMDD` (e.g., "BTC/USDT:USDT-241231")
//!
//! # Example
//!
//! ```rust
//! use ccxt_core::symbol::{ParsedSymbol, SymbolFormatter};
//! use ccxt_core::types::symbol::ExpiryDate;
//!
//! // Format a spot symbol
//! let spot = ParsedSymbol::spot("BTC".to_string(), "USDT".to_string());
//! assert_eq!(SymbolFormatter::format(&spot), "BTC/USDT");
//!
//! // Format a swap symbol
//! let swap = ParsedSymbol::linear_swap("BTC".to_string(), "USDT".to_string());
//! assert_eq!(SymbolFormatter::format(&swap), "BTC/USDT:USDT");
//!
//! // Format a futures symbol
//! let expiry = ExpiryDate::new(24, 12, 31).unwrap();
//! let futures = ParsedSymbol::futures("BTC".to_string(), "USDT".to_string(), "USDT".to_string(), expiry);
//! assert_eq!(SymbolFormatter::format(&futures), "BTC/USDT:USDT-241231");
//! ```

use crate::types::symbol::ParsedSymbol;

/// Symbol formatter for converting ParsedSymbol to unified symbol strings
///
/// Provides methods to format `ParsedSymbol` structures into their
/// canonical unified symbol string representation.
pub struct SymbolFormatter;

impl SymbolFormatter {
    /// Format a `ParsedSymbol` into a unified symbol string
    ///
    /// # Arguments
    ///
    /// * `parsed` - The parsed symbol to format
    ///
    /// # Returns
    ///
    /// Returns the unified symbol string in the appropriate format:
    /// - Spot: `BASE/QUOTE`
    /// - Swap: `BASE/QUOTE:SETTLE`
    /// - Futures: `BASE/QUOTE:SETTLE-YYMMDD`
    ///
    /// # Examples
    ///
    /// ```rust
    /// use ccxt_core::symbol::{ParsedSymbol, SymbolFormatter};
    /// use ccxt_core::types::symbol::ExpiryDate;
    ///
    /// // Spot symbol
    /// let spot = ParsedSymbol::spot("BTC".to_string(), "USDT".to_string());
    /// assert_eq!(SymbolFormatter::format(&spot), "BTC/USDT");
    ///
    /// // Linear swap symbol
    /// let swap = ParsedSymbol::linear_swap("ETH".to_string(), "USDT".to_string());
    /// assert_eq!(SymbolFormatter::format(&swap), "ETH/USDT:USDT");
    ///
    /// // Inverse swap symbol
    /// let inverse = ParsedSymbol::inverse_swap("BTC".to_string(), "USD".to_string());
    /// assert_eq!(SymbolFormatter::format(&inverse), "BTC/USD:BTC");
    ///
    /// // Futures symbol
    /// let expiry = ExpiryDate::new(24, 12, 31).unwrap();
    /// let futures = ParsedSymbol::futures("BTC".to_string(), "USDT".to_string(), "USDT".to_string(), expiry);
    /// assert_eq!(SymbolFormatter::format(&futures), "BTC/USDT:USDT-241231");
    /// ```
    pub fn format(parsed: &ParsedSymbol) -> String {
        // Use the Display implementation which already handles all cases
        parsed.to_string()
    }

    /// Format a spot symbol from base and quote currencies
    ///
    /// # Arguments
    ///
    /// * `base` - Base currency code
    /// * `quote` - Quote currency code
    ///
    /// # Returns
    ///
    /// Returns the formatted spot symbol string `BASE/QUOTE`
    pub fn format_spot(base: &str, quote: &str) -> String {
        format!("{}/{}", base.to_uppercase(), quote.to_uppercase())
    }

    /// Format a swap symbol from base, quote, and settle currencies
    ///
    /// # Arguments
    ///
    /// * `base` - Base currency code
    /// * `quote` - Quote currency code
    /// * `settle` - Settlement currency code
    ///
    /// # Returns
    ///
    /// Returns the formatted swap symbol string `BASE/QUOTE:SETTLE`
    pub fn format_swap(base: &str, quote: &str, settle: &str) -> String {
        format!(
            "{}/{}:{}",
            base.to_uppercase(),
            quote.to_uppercase(),
            settle.to_uppercase()
        )
    }

    /// Format a futures symbol from base, quote, settle currencies and expiry date
    ///
    /// # Arguments
    ///
    /// * `base` - Base currency code
    /// * `quote` - Quote currency code
    /// * `settle` - Settlement currency code
    /// * `year` - 2-digit year (0-99)
    /// * `month` - Month (1-12)
    /// * `day` - Day (1-31)
    ///
    /// # Returns
    ///
    /// Returns the formatted futures symbol string `BASE/QUOTE:SETTLE-YYMMDD`
    pub fn format_futures(
        base: &str,
        quote: &str,
        settle: &str,
        year: u8,
        month: u8,
        day: u8,
    ) -> String {
        format!(
            "{}/{}:{}-{:02}{:02}{:02}",
            base.to_uppercase(),
            quote.to_uppercase(),
            settle.to_uppercase(),
            year,
            month,
            day
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::symbol::ExpiryDate;

    #[test]
    fn test_format_spot() {
        let symbol = ParsedSymbol::spot("BTC".to_string(), "USDT".to_string());
        assert_eq!(SymbolFormatter::format(&symbol), "BTC/USDT");
    }

    #[test]
    fn test_format_spot_lowercase_input() {
        let symbol = ParsedSymbol::spot("btc".to_string(), "usdt".to_string());
        assert_eq!(SymbolFormatter::format(&symbol), "BTC/USDT");
    }

    #[test]
    fn test_format_linear_swap() {
        let symbol = ParsedSymbol::linear_swap("BTC".to_string(), "USDT".to_string());
        assert_eq!(SymbolFormatter::format(&symbol), "BTC/USDT:USDT");
    }

    #[test]
    fn test_format_inverse_swap() {
        let symbol = ParsedSymbol::inverse_swap("BTC".to_string(), "USD".to_string());
        assert_eq!(SymbolFormatter::format(&symbol), "BTC/USD:BTC");
    }

    #[test]
    fn test_format_futures() {
        let expiry = ExpiryDate::new(24, 12, 31).unwrap();
        let symbol = ParsedSymbol::futures(
            "BTC".to_string(),
            "USDT".to_string(),
            "USDT".to_string(),
            expiry,
        );
        assert_eq!(SymbolFormatter::format(&symbol), "BTC/USDT:USDT-241231");
    }

    #[test]
    fn test_format_futures_inverse() {
        let expiry = ExpiryDate::new(25, 3, 15).unwrap();
        let symbol = ParsedSymbol::futures(
            "BTC".to_string(),
            "USD".to_string(),
            "BTC".to_string(),
            expiry,
        );
        assert_eq!(SymbolFormatter::format(&symbol), "BTC/USD:BTC-250315");
    }

    #[test]
    fn test_format_spot_helper() {
        assert_eq!(SymbolFormatter::format_spot("BTC", "USDT"), "BTC/USDT");
        assert_eq!(SymbolFormatter::format_spot("btc", "usdt"), "BTC/USDT");
    }

    #[test]
    fn test_format_swap_helper() {
        assert_eq!(
            SymbolFormatter::format_swap("BTC", "USDT", "USDT"),
            "BTC/USDT:USDT"
        );
        assert_eq!(
            SymbolFormatter::format_swap("btc", "usd", "btc"),
            "BTC/USD:BTC"
        );
    }

    #[test]
    fn test_format_futures_helper() {
        assert_eq!(
            SymbolFormatter::format_futures("BTC", "USDT", "USDT", 24, 12, 31),
            "BTC/USDT:USDT-241231"
        );
        assert_eq!(
            SymbolFormatter::format_futures("btc", "usd", "btc", 25, 1, 5),
            "BTC/USD:BTC-250105"
        );
    }

    #[test]
    fn test_format_futures_date_padding() {
        // Ensure single-digit months and days are zero-padded
        assert_eq!(
            SymbolFormatter::format_futures("ETH", "USDT", "USDT", 25, 1, 5),
            "ETH/USDT:USDT-250105"
        );
    }
}
