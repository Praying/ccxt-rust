//! Symbol parser implementation
//!
//! This module provides parsing functionality for unified symbol strings,
//! converting them into `ParsedSymbol` structures.
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
//! use ccxt_core::symbol::{SymbolParser, ParsedSymbol};
//!
//! // Parse a spot symbol
//! let spot = SymbolParser::parse("BTC/USDT").unwrap();
//! assert!(spot.is_spot());
//!
//! // Parse a swap symbol
//! let swap = SymbolParser::parse("BTC/USDT:USDT").unwrap();
//! assert!(swap.is_swap());
//!
//! // Parse a futures symbol
//! let futures = SymbolParser::parse("BTC/USDT:USDT-241231").unwrap();
//! assert!(futures.is_futures());
//! ```

use super::error::SymbolError;
use crate::types::symbol::{ExpiryDate, ParsedSymbol};
use std::str::FromStr;

/// Symbol parser for unified symbol strings
///
/// Provides methods to parse and validate unified symbol strings into
/// `ParsedSymbol` structures.
pub struct SymbolParser;

impl SymbolParser {
    /// Parse a unified symbol string into a `ParsedSymbol`
    ///
    /// # Arguments
    ///
    /// * `symbol` - The unified symbol string to parse
    ///
    /// # Returns
    ///
    /// Returns `Ok(ParsedSymbol)` if parsing succeeds, or `Err(SymbolError)` if the
    /// symbol format is invalid.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use ccxt_core::symbol::SymbolParser;
    ///
    /// // Spot symbol
    /// let spot = SymbolParser::parse("BTC/USDT").unwrap();
    /// assert_eq!(spot.base, "BTC");
    /// assert_eq!(spot.quote, "USDT");
    /// assert!(spot.settle.is_none());
    ///
    /// // Swap symbol
    /// let swap = SymbolParser::parse("ETH/USDT:USDT").unwrap();
    /// assert_eq!(swap.settle, Some("USDT".to_string()));
    ///
    /// // Futures symbol
    /// let futures = SymbolParser::parse("BTC/USDT:USDT-241231").unwrap();
    /// assert!(futures.expiry.is_some());
    /// ```
    pub fn parse(symbol: &str) -> Result<ParsedSymbol, SymbolError> {
        // Trim whitespace
        let symbol = symbol.trim();

        // Check for empty symbol
        if symbol.is_empty() {
            return Err(SymbolError::EmptySymbol);
        }

        // Check for multiple colons
        let colon_count = symbol.chars().filter(|&c| c == ':').count();
        if colon_count > 1 {
            return Err(SymbolError::MultipleColons);
        }

        // Determine symbol type based on structure
        if colon_count == 0 {
            // Spot symbol: BASE/QUOTE
            Self::parse_spot(symbol)
        } else {
            // Derivative symbol: BASE/QUOTE:SETTLE or BASE/QUOTE:SETTLE-YYMMDD
            Self::parse_derivative(symbol)
        }
    }

    /// Parse a spot symbol (BASE/QUOTE format)
    ///
    /// # Arguments
    ///
    /// * `symbol` - The spot symbol string (already trimmed)
    fn parse_spot(symbol: &str) -> Result<ParsedSymbol, SymbolError> {
        // Validate no date suffix (should not contain hyphen followed by 6 digits)
        if Self::has_date_suffix(symbol) {
            return Err(SymbolError::InvalidFormat(
                "Spot symbol should not contain date suffix".to_string(),
            ));
        }

        // Split by forward slash
        let parts: Vec<&str> = symbol.split('/').collect();
        if parts.len() != 2 {
            return Err(SymbolError::InvalidFormat(format!(
                "Expected BASE/QUOTE format, got: {symbol}"
            )));
        }

        let base = parts[0].trim();
        let quote = parts[1].trim();

        // Validate currency codes
        Self::validate_currency(base)?;
        Self::validate_currency(quote)?;

        Ok(ParsedSymbol::spot(base, quote))
    }

    /// Parse a derivative symbol (swap or futures)
    ///
    /// # Arguments
    ///
    /// * `symbol` - The derivative symbol string (already trimmed)
    fn parse_derivative(symbol: &str) -> Result<ParsedSymbol, SymbolError> {
        // Split by colon to separate BASE/QUOTE from SETTLE[-YYMMDD]
        let colon_parts: Vec<&str> = symbol.split(':').collect();
        if colon_parts.len() != 2 {
            return Err(SymbolError::InvalidFormat(format!(
                "Expected BASE/QUOTE:SETTLE format, got: {symbol}"
            )));
        }

        let base_quote_part = colon_parts[0].trim();
        let settle_part = colon_parts[1].trim();

        // Parse BASE/QUOTE
        let slash_parts: Vec<&str> = base_quote_part.split('/').collect();
        if slash_parts.len() != 2 {
            return Err(SymbolError::InvalidFormat(format!(
                "Expected BASE/QUOTE format before colon, got: {base_quote_part}"
            )));
        }

        let base = slash_parts[0].trim();
        let quote = slash_parts[1].trim();

        // Validate base and quote
        Self::validate_currency(base)?;
        Self::validate_currency(quote)?;

        // Check if settle part contains date suffix
        if let Some(hyphen_pos) = settle_part.rfind('-') {
            let potential_date = &settle_part[hyphen_pos + 1..];

            // Check if it looks like a date (6 digits)
            if potential_date.len() == 6 && potential_date.chars().all(|c| c.is_ascii_digit()) {
                // Futures symbol: BASE/QUOTE:SETTLE-YYMMDD
                let settle = &settle_part[..hyphen_pos];
                Self::validate_currency(settle)?;

                let expiry = ExpiryDate::from_str(potential_date).map_err(|e| {
                    SymbolError::InvalidDateFormat(format!("{potential_date}: {e}"))
                })?;

                return Ok(ParsedSymbol::futures(base, quote, settle, expiry));
            }
        }

        // Swap symbol: BASE/QUOTE:SETTLE (no date suffix)
        Self::validate_currency(settle_part)?;

        Ok(ParsedSymbol::swap(base, quote, settle_part))
    }

    /// Validate a symbol string without full parsing
    ///
    /// # Arguments
    ///
    /// * `symbol` - The symbol string to validate
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if the symbol is valid, or `Err(SymbolError)` if invalid.
    pub fn validate(symbol: &str) -> Result<(), SymbolError> {
        Self::parse(symbol).map(|_| ())
    }

    /// Check if a string has a date suffix pattern (-YYMMDD)
    fn has_date_suffix(s: &str) -> bool {
        if let Some(hyphen_pos) = s.rfind('-') {
            let potential_date = &s[hyphen_pos + 1..];
            potential_date.len() == 6 && potential_date.chars().all(|c| c.is_ascii_digit())
        } else {
            false
        }
    }

    /// Validate a currency code
    ///
    /// Currency codes must be:
    /// - Non-empty
    /// - 1-10 characters long
    /// - Alphanumeric (letters and digits only)
    fn validate_currency(code: &str) -> Result<(), SymbolError> {
        if code.is_empty() {
            return Err(SymbolError::MissingComponent("currency code".to_string()));
        }

        if code.len() > 10 {
            return Err(SymbolError::InvalidCurrency(format!(
                "Currency code too long: {code}"
            )));
        }

        if !code.chars().all(|c| c.is_ascii_alphanumeric()) {
            return Err(SymbolError::InvalidCurrency(format!(
                "Currency code contains invalid characters: {code}"
            )));
        }

        Ok(())
    }
}

/// Implement FromStr for ParsedSymbol to enable string parsing
impl FromStr for ParsedSymbol {
    type Err = SymbolError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        SymbolParser::parse(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::symbol::SymbolMarketType;

    // ========================================================================
    // Spot Symbol Tests
    // ========================================================================

    #[test]
    fn test_parse_spot_basic() {
        let symbol = SymbolParser::parse("BTC/USDT").unwrap();
        assert_eq!(symbol.base, "BTC");
        assert_eq!(symbol.quote, "USDT");
        assert!(symbol.settle.is_none());
        assert!(symbol.expiry.is_none());
        assert_eq!(symbol.market_type(), SymbolMarketType::Spot);
    }

    #[test]
    fn test_parse_spot_lowercase() {
        let symbol = SymbolParser::parse("btc/usdt").unwrap();
        assert_eq!(symbol.base, "BTC");
        assert_eq!(symbol.quote, "USDT");
    }

    #[test]
    fn test_parse_spot_mixed_case() {
        let symbol = SymbolParser::parse("Btc/UsDt").unwrap();
        assert_eq!(symbol.base, "BTC");
        assert_eq!(symbol.quote, "USDT");
    }

    #[test]
    fn test_parse_spot_with_whitespace() {
        let symbol = SymbolParser::parse("  BTC/USDT  ").unwrap();
        assert_eq!(symbol.base, "BTC");
        assert_eq!(symbol.quote, "USDT");
    }

    #[test]
    fn test_parse_spot_numeric_currency() {
        let symbol = SymbolParser::parse("1INCH/USDT").unwrap();
        assert_eq!(symbol.base, "1INCH");
        assert_eq!(symbol.quote, "USDT");
    }

    // ========================================================================
    // Swap Symbol Tests
    // ========================================================================

    #[test]
    fn test_parse_linear_swap() {
        let symbol = SymbolParser::parse("BTC/USDT:USDT").unwrap();
        assert_eq!(symbol.base, "BTC");
        assert_eq!(symbol.quote, "USDT");
        assert_eq!(symbol.settle, Some("USDT".to_string()));
        assert!(symbol.expiry.is_none());
        assert_eq!(symbol.market_type(), SymbolMarketType::Swap);
        assert!(symbol.is_linear());
    }

    #[test]
    fn test_parse_inverse_swap() {
        let symbol = SymbolParser::parse("BTC/USD:BTC").unwrap();
        assert_eq!(symbol.base, "BTC");
        assert_eq!(symbol.quote, "USD");
        assert_eq!(symbol.settle, Some("BTC".to_string()));
        assert!(symbol.expiry.is_none());
        assert_eq!(symbol.market_type(), SymbolMarketType::Swap);
        assert!(symbol.is_inverse());
    }

    #[test]
    fn test_parse_swap_lowercase() {
        let symbol = SymbolParser::parse("eth/usdt:usdt").unwrap();
        assert_eq!(symbol.base, "ETH");
        assert_eq!(symbol.quote, "USDT");
        assert_eq!(symbol.settle, Some("USDT".to_string()));
    }

    // ========================================================================
    // Futures Symbol Tests
    // ========================================================================

    #[test]
    fn test_parse_futures_basic() {
        let symbol = SymbolParser::parse("BTC/USDT:USDT-241231").unwrap();
        assert_eq!(symbol.base, "BTC");
        assert_eq!(symbol.quote, "USDT");
        assert_eq!(symbol.settle, Some("USDT".to_string()));
        assert!(symbol.expiry.is_some());

        let expiry = symbol.expiry.unwrap();
        assert_eq!(expiry.year, 24);
        assert_eq!(expiry.month, 12);
        assert_eq!(expiry.day, 31);
        assert_eq!(symbol.market_type(), SymbolMarketType::Futures);
    }

    #[test]
    fn test_parse_futures_inverse() {
        let symbol = SymbolParser::parse("BTC/USD:BTC-250315").unwrap();
        assert_eq!(symbol.base, "BTC");
        assert_eq!(symbol.quote, "USD");
        assert_eq!(symbol.settle, Some("BTC".to_string()));
        assert!(symbol.expiry.is_some());

        let expiry = symbol.expiry.unwrap();
        assert_eq!(expiry.year, 25);
        assert_eq!(expiry.month, 3);
        assert_eq!(expiry.day, 15);
        assert!(symbol.is_inverse());
    }

    // ========================================================================
    // Error Cases
    // ========================================================================

    #[test]
    fn test_parse_empty_symbol() {
        let result = SymbolParser::parse("");
        assert!(matches!(result, Err(SymbolError::EmptySymbol)));
    }

    #[test]
    fn test_parse_whitespace_only() {
        let result = SymbolParser::parse("   ");
        assert!(matches!(result, Err(SymbolError::EmptySymbol)));
    }

    #[test]
    fn test_parse_multiple_colons() {
        let result = SymbolParser::parse("BTC/USDT:USDT:EXTRA");
        assert!(matches!(result, Err(SymbolError::MultipleColons)));
    }

    #[test]
    fn test_parse_missing_slash() {
        let result = SymbolParser::parse("BTCUSDT");
        assert!(matches!(result, Err(SymbolError::InvalidFormat(_))));
    }

    #[test]
    fn test_parse_invalid_date() {
        let result = SymbolParser::parse("BTC/USDT:USDT-241301"); // month 13
        assert!(matches!(result, Err(SymbolError::InvalidDateFormat(_))));
    }

    #[test]
    fn test_parse_invalid_currency_special_chars() {
        let result = SymbolParser::parse("BTC$/USDT");
        assert!(matches!(result, Err(SymbolError::InvalidCurrency(_))));
    }

    #[test]
    fn test_parse_empty_base() {
        let result = SymbolParser::parse("/USDT");
        assert!(matches!(result, Err(SymbolError::MissingComponent(_))));
    }

    #[test]
    fn test_parse_empty_quote() {
        let result = SymbolParser::parse("BTC/");
        assert!(matches!(result, Err(SymbolError::MissingComponent(_))));
    }

    // ========================================================================
    // FromStr Implementation Tests
    // ========================================================================

    #[test]
    fn test_from_str() {
        let symbol: ParsedSymbol = "BTC/USDT".parse().unwrap();
        assert_eq!(symbol.base, "BTC");
        assert_eq!(symbol.quote, "USDT");
    }

    // ========================================================================
    // Validate Tests
    // ========================================================================

    #[test]
    fn test_validate_valid_symbols() {
        assert!(SymbolParser::validate("BTC/USDT").is_ok());
        assert!(SymbolParser::validate("ETH/USDT:USDT").is_ok());
        assert!(SymbolParser::validate("BTC/USDT:USDT-241231").is_ok());
    }

    #[test]
    fn test_validate_invalid_symbols() {
        assert!(SymbolParser::validate("").is_err());
        assert!(SymbolParser::validate("BTCUSDT").is_err());
        assert!(SymbolParser::validate("BTC/USDT:USDT:EXTRA").is_err());
    }
}
