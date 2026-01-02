//! Symbol Type Definitions for Unified Symbol Format
//!
//! This module provides core type definitions for representing and working with unified
//! trading pair symbols across different market types (Spot, Swap, Futures) and
//! settlement currencies (Linear/Inverse).
//!
//! # Overview
//!
//! The types in this module form the foundation of the unified symbol system:
//!
//! - [`ParsedSymbol`]: The main structure representing a parsed trading pair symbol
//! - [`ExpiryDate`]: Futures contract expiry date in YYMMDD format
//! - [`SymbolMarketType`]: Enum for market types (Spot, Swap, Futures)
//! - [`ContractType`]: Enum for contract settlement types (Linear, Inverse)
//! - [`SymbolError`]: Error type for symbol-related operations
//!
//! # Symbol Format
//!
//! The unified symbol format follows the CCXT standard:
//!
//! | Market Type | Format | Example | Description |
//! |-------------|--------|---------|-------------|
//! | Spot | `BASE/QUOTE` | `BTC/USDT` | Direct exchange of assets |
//! | Linear Swap | `BASE/QUOTE:SETTLE` | `BTC/USDT:USDT` | Perpetual, settled in quote |
//! | Inverse Swap | `BASE/QUOTE:SETTLE` | `BTC/USD:BTC` | Perpetual, settled in base |
//! | Futures | `BASE/QUOTE:SETTLE-YYMMDD` | `BTC/USDT:USDT-241231` | Expiring contract |
//!
//! # Creating Symbols
//!
//! ## Spot Symbols
//!
//! ```rust
//! use ccxt_core::types::symbol::ParsedSymbol;
//!
//! let spot = ParsedSymbol::spot("BTC".to_string(), "USDT".to_string());
//! assert_eq!(spot.to_string(), "BTC/USDT");
//! assert!(spot.is_spot());
//! assert!(!spot.is_derivative());
//! ```
//!
//! ## Swap Symbols (Perpetuals)
//!
//! ```rust
//! use ccxt_core::types::symbol::{ParsedSymbol, ContractType};
//!
//! // Linear swap (USDT-margined)
//! let linear = ParsedSymbol::linear_swap("BTC".to_string(), "USDT".to_string());
//! assert_eq!(linear.to_string(), "BTC/USDT:USDT");
//! assert_eq!(linear.contract_type(), Some(ContractType::Linear));
//!
//! // Inverse swap (Coin-margined)
//! let inverse = ParsedSymbol::inverse_swap("BTC".to_string(), "USD".to_string());
//! assert_eq!(inverse.to_string(), "BTC/USD:BTC");
//! assert_eq!(inverse.contract_type(), Some(ContractType::Inverse));
//!
//! // Custom swap with explicit settlement
//! let custom = ParsedSymbol::swap("ETH".to_string(), "USDT".to_string(), "USDT".to_string());
//! assert!(custom.is_swap());
//! ```
//!
//! ## Futures Symbols
//!
//! ```rust
//! use ccxt_core::types::symbol::{ParsedSymbol, ExpiryDate};
//!
//! let expiry = ExpiryDate::new(24, 12, 31).unwrap();
//! let futures = ParsedSymbol::futures(
//!     "BTC".to_string(),
//!     "USDT".to_string(),
//!     "USDT".to_string(),
//!     expiry
//! );
//! assert_eq!(futures.to_string(), "BTC/USDT:USDT-241231");
//! assert!(futures.is_futures());
//! ```
//!
//! # Analyzing Symbols
//!
//! ```rust
//! use ccxt_core::types::symbol::{ParsedSymbol, SymbolMarketType, ContractType};
//!
//! let symbol = ParsedSymbol::linear_swap("ETH".to_string(), "USDT".to_string());
//!
//! // Market type detection
//! assert_eq!(symbol.market_type(), SymbolMarketType::Swap);
//! assert!(symbol.is_swap());
//! assert!(symbol.is_derivative());
//!
//! // Contract type detection
//! assert_eq!(symbol.contract_type(), Some(ContractType::Linear));
//! assert!(symbol.is_linear());
//! assert!(!symbol.is_inverse());
//! ```
//!
//! # Expiry Date Handling
//!
//! ```rust
//! use ccxt_core::types::symbol::ExpiryDate;
//! use std::str::FromStr;
//!
//! // Create with validation
//! let expiry = ExpiryDate::new(24, 12, 31).unwrap();
//! assert_eq!(expiry.to_string(), "241231");
//!
//! // Parse from string
//! let parsed = ExpiryDate::from_str("250315").unwrap();
//! assert_eq!(parsed.year, 25);
//! assert_eq!(parsed.month, 3);
//! assert_eq!(parsed.day, 15);
//!
//! // Invalid dates are rejected
//! assert!(ExpiryDate::new(24, 13, 1).is_err()); // month > 12
//! assert!(ExpiryDate::new(24, 1, 32).is_err()); // day > 31
//! ```
//!
//! # Case Normalization
//!
//! All currency codes are automatically normalized to uppercase:
//!
//! ```rust
//! use ccxt_core::types::symbol::ParsedSymbol;
//!
//! let symbol = ParsedSymbol::spot("btc".to_string(), "usdt".to_string());
//! assert_eq!(symbol.base, "BTC");
//! assert_eq!(symbol.quote, "USDT");
//! ```

use serde::{Deserialize, Serialize};
use std::fmt;
use std::hash::{Hash, Hasher};
use std::str::FromStr;

/// Error type for symbol-related operations
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SymbolError {
    /// Invalid symbol format
    #[error("Invalid symbol format: {0}")]
    InvalidFormat(String),

    /// Missing required component
    #[error("Missing required component: {0}")]
    MissingComponent(String),

    /// Invalid currency code
    #[error("Invalid currency code: {0}")]
    InvalidCurrency(String),

    /// Invalid expiry date
    #[error("Invalid expiry date: {year:02}{month:02}{day:02}")]
    InvalidExpiryDate {
        /// Year (2-digit)
        year: u8,
        /// Month (1-12)
        month: u8,
        /// Day (1-31)
        day: u8,
    },

    /// Unexpected date suffix in swap symbol
    #[error("Unexpected date suffix in swap symbol")]
    UnexpectedDateSuffix,

    /// Missing date suffix in futures symbol
    #[error("Missing date suffix in futures symbol")]
    MissingDateSuffix,
}

/// Represents a futures contract expiry date in YYMMDD format
///
/// # Example
///
/// ```rust
/// use ccxt_core::types::symbol::ExpiryDate;
///
/// let expiry = ExpiryDate::new(24, 12, 31).unwrap();
/// assert_eq!(expiry.to_string(), "241231");
/// ```
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ExpiryDate {
    /// 2-digit year (e.g., 24 for 2024)
    pub year: u8,
    /// Month (1-12)
    pub month: u8,
    /// Day (1-31)
    pub day: u8,
}

impl ExpiryDate {
    /// Create a new expiry date with validation
    ///
    /// # Arguments
    ///
    /// * `year` - 2-digit year (0-99)
    /// * `month` - Month (1-12)
    /// * `day` - Day (1-31)
    ///
    /// # Returns
    ///
    /// Returns `Ok(ExpiryDate)` if the date is valid, or `Err(SymbolError)` if invalid.
    ///
    /// # Example
    ///
    /// ```rust
    /// use ccxt_core::types::symbol::ExpiryDate;
    ///
    /// let valid = ExpiryDate::new(24, 12, 31);
    /// assert!(valid.is_ok());
    ///
    /// let invalid_month = ExpiryDate::new(24, 13, 1);
    /// assert!(invalid_month.is_err());
    /// ```
    pub fn new(year: u8, month: u8, day: u8) -> Result<Self, SymbolError> {
        // Validate month (1-12)
        if !(1..=12).contains(&month) {
            return Err(SymbolError::InvalidExpiryDate { year, month, day });
        }

        // Validate day (1-31)
        if !(1..=31).contains(&day) {
            return Err(SymbolError::InvalidExpiryDate { year, month, day });
        }

        Ok(Self { year, month, day })
    }
}

impl PartialEq for ExpiryDate {
    fn eq(&self, other: &Self) -> bool {
        self.year == other.year && self.month == other.month && self.day == other.day
    }
}

impl Eq for ExpiryDate {}

impl Hash for ExpiryDate {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.year.hash(state);
        self.month.hash(state);
        self.day.hash(state);
    }
}

impl Default for ExpiryDate {
    fn default() -> Self {
        Self {
            year: 0,
            month: 1,
            day: 1,
        }
    }
}

impl fmt::Display for ExpiryDate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:02}{:02}{:02}", self.year, self.month, self.day)
    }
}

impl FromStr for ExpiryDate {
    type Err = SymbolError;

    /// Parse an expiry date from YYMMDD format
    ///
    /// # Example
    ///
    /// ```rust
    /// use ccxt_core::types::symbol::ExpiryDate;
    /// use std::str::FromStr;
    ///
    /// let expiry = ExpiryDate::from_str("241231").unwrap();
    /// assert_eq!(expiry.year, 24);
    /// assert_eq!(expiry.month, 12);
    /// assert_eq!(expiry.day, 31);
    /// ```
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.len() != 6 {
            return Err(SymbolError::InvalidFormat(format!(
                "Expiry date must be 6 characters (YYMMDD), got: {s}"
            )));
        }

        let year: u8 = s[0..2]
            .parse()
            .map_err(|_| SymbolError::InvalidFormat(format!("Invalid year in expiry: {s}")))?;

        let month: u8 = s[2..4]
            .parse()
            .map_err(|_| SymbolError::InvalidFormat(format!("Invalid month in expiry: {s}")))?;

        let day: u8 = s[4..6]
            .parse()
            .map_err(|_| SymbolError::InvalidFormat(format!("Invalid day in expiry: {s}")))?;

        Self::new(year, month, day)
    }
}

/// Market type derived from symbol structure
///
/// This enum represents the type of market based on the symbol format:
/// - `Spot`: No settlement currency (BASE/QUOTE)
/// - `Swap`: Has settlement currency but no expiry (BASE/QUOTE:SETTLE)
/// - `Futures`: Has settlement currency and expiry date (BASE/QUOTE:SETTLE-YYMMDD)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum SymbolMarketType {
    /// Spot market (BASE/QUOTE)
    #[default]
    Spot,
    /// Perpetual swap market (BASE/QUOTE:SETTLE)
    Swap,
    /// Futures market with expiry (BASE/QUOTE:SETTLE-YYMMDD)
    Futures,
}

impl fmt::Display for SymbolMarketType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Spot => write!(f, "spot"),
            Self::Swap => write!(f, "swap"),
            Self::Futures => write!(f, "futures"),
        }
    }
}

/// Contract settlement type for derivatives
///
/// - `Linear`: Settled in quote currency (USDT-margined)
/// - `Inverse`: Settled in base currency (Coin-margined)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ContractType {
    /// Linear contract - settled in quote currency (USDT-margined)
    /// Example: BTC/USDT:USDT (settle == quote)
    #[default]
    Linear,
    /// Inverse contract - settled in base currency (Coin-margined)
    /// Example: BTC/USD:BTC (settle == base)
    Inverse,
}

impl fmt::Display for ContractType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Linear => write!(f, "linear"),
            Self::Inverse => write!(f, "inverse"),
        }
    }
}

/// Represents a parsed unified symbol with all components
///
/// This struct holds all the components of a unified symbol:
/// - `base`: Base currency (e.g., "BTC")
/// - `quote`: Quote currency (e.g., "USDT")
/// - `settle`: Settlement currency for derivatives (e.g., "USDT" or "BTC")
/// - `expiry`: Expiry date for futures contracts
///
/// # Example
///
/// ```rust
/// use ccxt_core::types::symbol::{ParsedSymbol, ExpiryDate, SymbolMarketType};
///
/// // Spot symbol
/// let spot = ParsedSymbol::spot("BTC".to_string(), "USDT".to_string());
/// assert_eq!(spot.to_string(), "BTC/USDT");
///
/// // Linear swap
/// let swap = ParsedSymbol::linear_swap("BTC".to_string(), "USDT".to_string());
/// assert_eq!(swap.to_string(), "BTC/USDT:USDT");
///
/// // Futures with expiry
/// let expiry = ExpiryDate::new(24, 12, 31).unwrap();
/// let futures = ParsedSymbol::futures("BTC".to_string(), "USDT".to_string(), "USDT".to_string(), expiry);
/// assert_eq!(futures.to_string(), "BTC/USDT:USDT-241231");
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ParsedSymbol {
    /// Base currency (e.g., "BTC")
    pub base: String,
    /// Quote currency (e.g., "USDT")
    pub quote: String,
    /// Settlement currency for derivatives (e.g., "USDT" or "BTC")
    pub settle: Option<String>,
    /// Expiry date for futures contracts
    pub expiry: Option<ExpiryDate>,
}

impl ParsedSymbol {
    /// Create a new spot symbol
    ///
    /// # Arguments
    ///
    /// * `base` - Base currency code (e.g., "BTC")
    /// * `quote` - Quote currency code (e.g., "USDT")
    ///
    /// # Example
    ///
    /// ```rust
    /// use ccxt_core::types::symbol::ParsedSymbol;
    ///
    /// let symbol = ParsedSymbol::spot("BTC".to_string(), "USDT".to_string());
    /// assert_eq!(symbol.to_string(), "BTC/USDT");
    /// ```
    pub fn spot(base: impl AsRef<str>, quote: impl AsRef<str>) -> Self {
        Self {
            base: base.as_ref().to_uppercase(),
            quote: quote.as_ref().to_uppercase(),
            settle: None,
            expiry: None,
        }
    }

    /// Create a new linear swap symbol (settled in quote currency)
    ///
    /// # Arguments
    ///
    /// * `base` - Base currency code (e.g., "BTC")
    /// * `quote` - Quote currency code (e.g., "USDT")
    ///
    /// # Example
    ///
    /// ```rust
    /// use ccxt_core::types::symbol::ParsedSymbol;
    ///
    /// let symbol = ParsedSymbol::linear_swap("BTC".to_string(), "USDT".to_string());
    /// assert_eq!(symbol.to_string(), "BTC/USDT:USDT");
    /// ```
    pub fn linear_swap(base: impl AsRef<str>, quote: impl AsRef<str>) -> Self {
        let quote_upper = quote.as_ref().to_uppercase();
        Self {
            base: base.as_ref().to_uppercase(),
            quote: quote_upper.clone(),
            settle: Some(quote_upper),
            expiry: None,
        }
    }

    /// Create a new inverse swap symbol (settled in base currency)
    ///
    /// # Arguments
    ///
    /// * `base` - Base currency code (e.g., "BTC")
    /// * `quote` - Quote currency code (e.g., "USD")
    ///
    /// # Example
    ///
    /// ```rust
    /// use ccxt_core::types::symbol::ParsedSymbol;
    ///
    /// let symbol = ParsedSymbol::inverse_swap("BTC".to_string(), "USD".to_string());
    /// assert_eq!(symbol.to_string(), "BTC/USD:BTC");
    /// ```
    pub fn inverse_swap(base: impl AsRef<str>, quote: impl AsRef<str>) -> Self {
        let base_upper = base.as_ref().to_uppercase();
        Self {
            base: base_upper.clone(),
            quote: quote.as_ref().to_uppercase(),
            settle: Some(base_upper),
            expiry: None,
        }
    }

    /// Create a new swap symbol with explicit settlement currency
    ///
    /// # Arguments
    ///
    /// * `base` - Base currency code
    /// * `quote` - Quote currency code
    /// * `settle` - Settlement currency code
    pub fn swap(base: impl AsRef<str>, quote: impl AsRef<str>, settle: impl AsRef<str>) -> Self {
        Self {
            base: base.as_ref().to_uppercase(),
            quote: quote.as_ref().to_uppercase(),
            settle: Some(settle.as_ref().to_uppercase()),
            expiry: None,
        }
    }

    /// Create a new futures symbol
    ///
    /// # Arguments
    ///
    /// * `base` - Base currency code
    /// * `quote` - Quote currency code
    /// * `settle` - Settlement currency code
    /// * `expiry` - Expiry date
    ///
    /// # Example
    ///
    /// ```rust
    /// use ccxt_core::types::symbol::{ParsedSymbol, ExpiryDate};
    ///
    /// let expiry = ExpiryDate::new(24, 12, 31).unwrap();
    /// let symbol = ParsedSymbol::futures("BTC".to_string(), "USDT".to_string(), "USDT".to_string(), expiry);
    /// assert_eq!(symbol.to_string(), "BTC/USDT:USDT-241231");
    /// ```
    pub fn futures(
        base: impl AsRef<str>,
        quote: impl AsRef<str>,
        settle: impl AsRef<str>,
        expiry: ExpiryDate,
    ) -> Self {
        Self {
            base: base.as_ref().to_uppercase(),
            quote: quote.as_ref().to_uppercase(),
            settle: Some(settle.as_ref().to_uppercase()),
            expiry: Some(expiry),
        }
    }

    /// Determine the market type from symbol structure
    ///
    /// - No settle and no expiry → Spot
    /// - Has settle but no expiry → Swap
    /// - Has settle and expiry → Futures
    pub fn market_type(&self) -> SymbolMarketType {
        match (&self.settle, &self.expiry) {
            (None, None) => SymbolMarketType::Spot,
            (Some(_), None) => SymbolMarketType::Swap,
            (Some(_) | None, Some(_)) => SymbolMarketType::Futures,
        }
    }

    /// Determine if this is a linear or inverse contract
    ///
    /// Returns `None` for spot markets.
    /// - Linear: settle == quote
    /// - Inverse: settle == base
    pub fn contract_type(&self) -> Option<ContractType> {
        self.settle.as_ref().map(|settle| {
            if settle == &self.quote {
                ContractType::Linear
            } else if settle == &self.base {
                ContractType::Inverse
            } else {
                // Default to linear if settle doesn't match either
                ContractType::Linear
            }
        })
    }

    /// Check if this is a spot symbol
    pub fn is_spot(&self) -> bool {
        self.market_type() == SymbolMarketType::Spot
    }

    /// Check if this is a swap (perpetual) symbol
    pub fn is_swap(&self) -> bool {
        self.market_type() == SymbolMarketType::Swap
    }

    /// Check if this is a futures symbol
    pub fn is_futures(&self) -> bool {
        self.market_type() == SymbolMarketType::Futures
    }

    /// Check if this is a derivative (swap or futures)
    pub fn is_derivative(&self) -> bool {
        self.settle.is_some()
    }

    /// Check if this is a linear contract
    pub fn is_linear(&self) -> bool {
        self.contract_type() == Some(ContractType::Linear)
    }

    /// Check if this is an inverse contract
    pub fn is_inverse(&self) -> bool {
        self.contract_type() == Some(ContractType::Inverse)
    }
}

impl PartialEq for ParsedSymbol {
    fn eq(&self, other: &Self) -> bool {
        self.base == other.base
            && self.quote == other.quote
            && self.settle == other.settle
            && self.expiry == other.expiry
    }
}

impl Eq for ParsedSymbol {}

impl Hash for ParsedSymbol {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.base.hash(state);
        self.quote.hash(state);
        self.settle.hash(state);
        self.expiry.hash(state);
    }
}

impl fmt::Display for ParsedSymbol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}", self.base, self.quote)?;

        if let Some(ref settle) = self.settle {
            write!(f, ":{settle}")?;

            if let Some(ref expiry) = self.expiry {
                write!(f, "-{expiry}")?;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expiry_date_creation() {
        let expiry = ExpiryDate::new(24, 12, 31).unwrap();
        assert_eq!(expiry.year, 24);
        assert_eq!(expiry.month, 12);
        assert_eq!(expiry.day, 31);
    }

    #[test]
    fn test_expiry_date_validation() {
        // Valid dates
        assert!(ExpiryDate::new(24, 1, 1).is_ok());
        assert!(ExpiryDate::new(24, 12, 31).is_ok());
        assert!(ExpiryDate::new(99, 6, 15).is_ok());

        // Invalid month
        assert!(ExpiryDate::new(24, 0, 1).is_err());
        assert!(ExpiryDate::new(24, 13, 1).is_err());

        // Invalid day
        assert!(ExpiryDate::new(24, 1, 0).is_err());
        assert!(ExpiryDate::new(24, 1, 32).is_err());
    }

    #[test]
    fn test_expiry_date_display() {
        let expiry = ExpiryDate::new(24, 12, 31).unwrap();
        assert_eq!(expiry.to_string(), "241231");

        let expiry2 = ExpiryDate::new(25, 1, 5).unwrap();
        assert_eq!(expiry2.to_string(), "250105");
    }

    #[test]
    fn test_expiry_date_from_str() {
        let expiry: ExpiryDate = "241231".parse().unwrap();
        assert_eq!(expiry.year, 24);
        assert_eq!(expiry.month, 12);
        assert_eq!(expiry.day, 31);

        // Invalid format
        assert!("24123".parse::<ExpiryDate>().is_err());
        assert!("2412311".parse::<ExpiryDate>().is_err());
        assert!("abcdef".parse::<ExpiryDate>().is_err());

        // Invalid date values
        assert!("241301".parse::<ExpiryDate>().is_err()); // month > 12
        assert!("241232".parse::<ExpiryDate>().is_err()); // day > 31
    }

    #[test]
    fn test_parsed_symbol_spot() {
        let symbol = ParsedSymbol::spot("BTC".to_string(), "USDT".to_string());
        assert_eq!(symbol.base, "BTC");
        assert_eq!(symbol.quote, "USDT");
        assert!(symbol.settle.is_none());
        assert!(symbol.expiry.is_none());
        assert_eq!(symbol.market_type(), SymbolMarketType::Spot);
        assert!(symbol.is_spot());
        assert!(!symbol.is_derivative());
        assert_eq!(symbol.to_string(), "BTC/USDT");
    }

    #[test]
    fn test_parsed_symbol_linear_swap() {
        let symbol = ParsedSymbol::linear_swap("BTC".to_string(), "USDT".to_string());
        assert_eq!(symbol.base, "BTC");
        assert_eq!(symbol.quote, "USDT");
        assert_eq!(symbol.settle, Some("USDT".to_string()));
        assert!(symbol.expiry.is_none());
        assert_eq!(symbol.market_type(), SymbolMarketType::Swap);
        assert!(symbol.is_swap());
        assert!(symbol.is_derivative());
        assert_eq!(symbol.contract_type(), Some(ContractType::Linear));
        assert!(symbol.is_linear());
        assert_eq!(symbol.to_string(), "BTC/USDT:USDT");
    }

    #[test]
    fn test_parsed_symbol_inverse_swap() {
        let symbol = ParsedSymbol::inverse_swap("BTC".to_string(), "USD".to_string());
        assert_eq!(symbol.base, "BTC");
        assert_eq!(symbol.quote, "USD");
        assert_eq!(symbol.settle, Some("BTC".to_string()));
        assert!(symbol.expiry.is_none());
        assert_eq!(symbol.market_type(), SymbolMarketType::Swap);
        assert!(symbol.is_swap());
        assert!(symbol.is_derivative());
        assert_eq!(symbol.contract_type(), Some(ContractType::Inverse));
        assert!(symbol.is_inverse());
        assert_eq!(symbol.to_string(), "BTC/USD:BTC");
    }

    #[test]
    fn test_parsed_symbol_futures() {
        let expiry = ExpiryDate::new(24, 12, 31).unwrap();
        let symbol = ParsedSymbol::futures(
            "BTC".to_string(),
            "USDT".to_string(),
            "USDT".to_string(),
            expiry,
        );
        assert_eq!(symbol.base, "BTC");
        assert_eq!(symbol.quote, "USDT");
        assert_eq!(symbol.settle, Some("USDT".to_string()));
        assert!(symbol.expiry.is_some());
        assert_eq!(symbol.market_type(), SymbolMarketType::Futures);
        assert!(symbol.is_futures());
        assert!(symbol.is_derivative());
        assert_eq!(symbol.contract_type(), Some(ContractType::Linear));
        assert_eq!(symbol.to_string(), "BTC/USDT:USDT-241231");
    }

    #[test]
    fn test_parsed_symbol_case_normalization() {
        let symbol = ParsedSymbol::spot("btc".to_string(), "usdt".to_string());
        assert_eq!(symbol.base, "BTC");
        assert_eq!(symbol.quote, "USDT");

        let swap = ParsedSymbol::linear_swap("eth".to_string(), "usdt".to_string());
        assert_eq!(swap.base, "ETH");
        assert_eq!(swap.quote, "USDT");
        assert_eq!(swap.settle, Some("USDT".to_string()));
    }

    #[test]
    fn test_symbol_market_type_display() {
        assert_eq!(SymbolMarketType::Spot.to_string(), "spot");
        assert_eq!(SymbolMarketType::Swap.to_string(), "swap");
        assert_eq!(SymbolMarketType::Futures.to_string(), "futures");
    }

    #[test]
    fn test_contract_type_display() {
        assert_eq!(ContractType::Linear.to_string(), "linear");
        assert_eq!(ContractType::Inverse.to_string(), "inverse");
    }

    #[test]
    fn test_parsed_symbol_equality() {
        let s1 = ParsedSymbol::spot("BTC".to_string(), "USDT".to_string());
        let s2 = ParsedSymbol::spot("BTC".to_string(), "USDT".to_string());
        let s3 = ParsedSymbol::spot("ETH".to_string(), "USDT".to_string());

        assert_eq!(s1, s2);
        assert_ne!(s1, s3);
    }

    #[test]
    fn test_parsed_symbol_hash() {
        use std::collections::HashSet;

        let mut set = HashSet::new();
        set.insert(ParsedSymbol::spot("BTC".to_string(), "USDT".to_string()));
        set.insert(ParsedSymbol::spot("BTC".to_string(), "USDT".to_string())); // duplicate
        set.insert(ParsedSymbol::spot("ETH".to_string(), "USDT".to_string()));

        assert_eq!(set.len(), 2);
    }
}
