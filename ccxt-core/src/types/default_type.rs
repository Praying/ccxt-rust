//! Default Type Definitions for Exchange Configuration
//!
//! This module provides type-safe configuration for selecting which market type
//! (spot, swap, futures, margin, option) an exchange client should use by default
//! when calling generic API methods.
//!
//! # Overview
//!
//! The types in this module enable:
//!
//! - [`DefaultType`]: Primary market type configuration (spot, swap, futures, margin, option)
//! - [`DefaultSubType`]: Secondary configuration for contract settlement (linear, inverse)
//!
//! # Example
//!
//! ```rust
//! use ccxt_core::types::default_type::{DefaultType, DefaultSubType};
//! use std::str::FromStr;
//!
//! // Create default types
//! let spot = DefaultType::Spot;
//! let swap = DefaultType::Swap;
//!
//! // Parse from string (case-insensitive)
//! let futures = DefaultType::from_str("FUTURES").unwrap();
//! let margin = DefaultType::from_str("margin").unwrap();
//!
//! // Display as lowercase
//! assert_eq!(spot.to_string(), "spot");
//! assert_eq!(swap.to_string(), "swap");
//!
//! // Check market characteristics
//! assert!(!spot.is_derivative());
//! assert!(swap.is_derivative());
//! assert!(futures.is_contract());
//! ```

use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

use super::market::MarketType;
use super::symbol::{ParsedSymbol, SymbolMarketType};
use crate::symbol::SymbolParser;

/// Error type for DefaultType parsing
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum DefaultTypeError {
    /// Invalid default type string
    #[error("Invalid default type '{0}'. Valid options are: spot, swap, futures, margin, option")]
    InvalidType(String),

    /// Invalid default sub-type string
    #[error("Invalid default sub-type '{0}'. Valid options are: linear, inverse")]
    InvalidSubType(String),
}

/// Default market type for exchange operations
///
/// This enum determines which API endpoints and market filters
/// to use when calling generic exchange methods.
///
/// # Variants
///
/// - `Spot`: Regular cryptocurrency trading without leverage or expiry
/// - `Swap`: Perpetual contracts with no expiry date (also called perpetual futures)
/// - `Futures`: Delivery contracts with a specific expiry date
/// - `Margin`: Spot trading with borrowed funds (leverage)
/// - `Option`: Options contracts (calls and puts)
///
/// # Example
///
/// ```rust
/// use ccxt_core::types::default_type::DefaultType;
///
/// let default = DefaultType::default();
/// assert_eq!(default, DefaultType::Spot);
///
/// assert!(DefaultType::Swap.is_derivative());
/// assert!(DefaultType::Futures.is_contract());
/// assert!(!DefaultType::Spot.is_derivative());
/// ```
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DefaultType {
    /// Spot market - regular cryptocurrency trading
    #[default]
    Spot,
    /// Swap market - perpetual contracts with no expiry
    Swap,
    /// Futures market - delivery contracts with expiry date
    Futures,
    /// Margin market - spot trading with borrowed funds
    Margin,
    /// Option market - options contracts (calls and puts)
    Option,
}

impl DefaultType {
    /// All valid default type variants
    pub const ALL: &'static [DefaultType] = &[
        DefaultType::Spot,
        DefaultType::Swap,
        DefaultType::Futures,
        DefaultType::Margin,
        DefaultType::Option,
    ];

    /// Returns the string representation (lowercase)
    ///
    /// # Example
    ///
    /// ```rust
    /// use ccxt_core::types::default_type::DefaultType;
    ///
    /// assert_eq!(DefaultType::Spot.as_str(), "spot");
    /// assert_eq!(DefaultType::Swap.as_str(), "swap");
    /// ```
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Spot => "spot",
            Self::Swap => "swap",
            Self::Futures => "futures",
            Self::Margin => "margin",
            Self::Option => "option",
        }
    }

    /// Check if this type represents a derivative market
    ///
    /// Derivatives include swap, futures, margin, and option markets.
    ///
    /// # Example
    ///
    /// ```rust
    /// use ccxt_core::types::default_type::DefaultType;
    ///
    /// assert!(!DefaultType::Spot.is_derivative());
    /// assert!(DefaultType::Swap.is_derivative());
    /// assert!(DefaultType::Futures.is_derivative());
    /// assert!(DefaultType::Margin.is_derivative());
    /// assert!(DefaultType::Option.is_derivative());
    /// ```
    pub fn is_derivative(&self) -> bool {
        !matches!(self, Self::Spot)
    }

    /// Check if this type represents a contract market (swap, futures, option)
    ///
    /// Contract markets are those that trade derivative contracts rather than
    /// the underlying asset directly.
    ///
    /// # Example
    ///
    /// ```rust
    /// use ccxt_core::types::default_type::DefaultType;
    ///
    /// assert!(!DefaultType::Spot.is_contract());
    /// assert!(DefaultType::Swap.is_contract());
    /// assert!(DefaultType::Futures.is_contract());
    /// assert!(!DefaultType::Margin.is_contract());
    /// assert!(DefaultType::Option.is_contract());
    /// ```
    pub fn is_contract(&self) -> bool {
        matches!(self, Self::Swap | Self::Futures | Self::Option)
    }
}

impl fmt::Display for DefaultType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for DefaultType {
    type Err = DefaultTypeError;

    /// Parse a DefaultType from a string (case-insensitive)
    ///
    /// # Example
    ///
    /// ```rust
    /// use ccxt_core::types::default_type::DefaultType;
    /// use std::str::FromStr;
    ///
    /// assert_eq!(DefaultType::from_str("spot").unwrap(), DefaultType::Spot);
    /// assert_eq!(DefaultType::from_str("SWAP").unwrap(), DefaultType::Swap);
    /// assert_eq!(DefaultType::from_str("Futures").unwrap(), DefaultType::Futures);
    ///
    /// assert!(DefaultType::from_str("invalid").is_err());
    /// ```
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let lowercase = s.to_lowercase();
        match lowercase.as_str() {
            "spot" => Ok(Self::Spot),
            // Legacy: "future" typically meant perpetual futures
            "swap" | "future" => Ok(Self::Swap),
            // Legacy: "delivery" meant dated futures
            "futures" | "delivery" => Ok(Self::Futures),
            "margin" => Ok(Self::Margin),
            "option" => Ok(Self::Option),
            _ => Err(DefaultTypeError::InvalidType(s.to_string())),
        }
    }
}

impl From<&str> for DefaultType {
    /// Convert a string slice to DefaultType.
    ///
    /// Panics if the string is not a valid DefaultType.
    /// For fallible conversion, use `FromStr::from_str()` instead.
    ///
    /// # Example
    ///
    /// ```rust
    /// use ccxt_core::types::default_type::DefaultType;
    ///
    /// let dt: DefaultType = "spot".into();
    /// assert_eq!(dt, DefaultType::Spot);
    ///
    /// let dt: DefaultType = "swap".into();
    /// assert_eq!(dt, DefaultType::Swap);
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if the string is not a valid DefaultType value.
    fn from(s: &str) -> Self {
        Self::from_str(s).unwrap_or(Self::Spot)
    }
}

impl From<String> for DefaultType {
    /// Convert a String to DefaultType.
    ///
    /// Falls back to `Spot` if the string is not a valid DefaultType.
    /// For fallible conversion, use `FromStr::from_str()` instead.
    ///
    /// # Example
    ///
    /// ```rust
    /// use ccxt_core::types::default_type::DefaultType;
    ///
    /// let dt: DefaultType = "swap".to_string().into();
    /// assert_eq!(dt, DefaultType::Swap);
    /// ```
    fn from(s: String) -> Self {
        Self::from_str(&s).unwrap_or(Self::Spot)
    }
}

impl From<DefaultType> for MarketType {
    /// Convert DefaultType to MarketType
    ///
    /// Note: Margin maps to Spot since margin trading uses spot markets with leverage.
    fn from(dt: DefaultType) -> Self {
        match dt {
            // Margin is spot with leverage
            DefaultType::Spot | DefaultType::Margin => MarketType::Spot,
            DefaultType::Swap => MarketType::Swap,
            DefaultType::Futures => MarketType::Futures,
            DefaultType::Option => MarketType::Option,
        }
    }
}

impl TryFrom<MarketType> for DefaultType {
    type Error = DefaultTypeError;

    /// Convert MarketType to DefaultType
    ///
    /// Note: MarketType::Spot could be either DefaultType::Spot or DefaultType::Margin,
    /// so this conversion defaults to Spot.
    fn try_from(mt: MarketType) -> Result<Self, Self::Error> {
        match mt {
            MarketType::Spot => Ok(DefaultType::Spot),
            MarketType::Swap => Ok(DefaultType::Swap),
            MarketType::Futures => Ok(DefaultType::Futures),
            MarketType::Option => Ok(DefaultType::Option),
        }
    }
}

/// Resolves the effective market type for an API call
///
/// This function determines which market type to use based on:
/// 1. If a symbol is provided and explicitly specifies a market type (has settle currency),
///    the symbol's type takes precedence
/// 2. Otherwise, the configured default_type is converted to MarketType
///
/// # Arguments
///
/// * `symbol` - Optional symbol string to parse (e.g., "BTC/USDT" or "BTC/USDT:USDT")
/// * `default_type` - The configured default market type to use as fallback
///
/// # Returns
///
/// The resolved `MarketType` based on the symbol or default_type
///
/// # Example
///
/// ```rust
/// use ccxt_core::types::default_type::{DefaultType, resolve_market_type};
/// use ccxt_core::types::market::MarketType;
///
/// // Symbol with explicit type overrides default
/// let market_type = resolve_market_type(Some("BTC/USDT:USDT"), DefaultType::Spot);
/// assert_eq!(market_type, MarketType::Swap);
///
/// // Spot symbol uses default type
/// let market_type = resolve_market_type(Some("BTC/USDT"), DefaultType::Swap);
/// assert_eq!(market_type, MarketType::Swap);
///
/// // No symbol uses default type
/// let market_type = resolve_market_type(None, DefaultType::Futures);
/// assert_eq!(market_type, MarketType::Futures);
///
/// // Invalid symbol falls back to default type
/// let market_type = resolve_market_type(Some("invalid"), DefaultType::Spot);
/// assert_eq!(market_type, MarketType::Spot);
/// ```
pub fn resolve_market_type(symbol: Option<&str>, default_type: DefaultType) -> MarketType {
    // If no symbol provided, use default_type
    let Some(symbol_str) = symbol else {
        return MarketType::from(default_type);
    };

    // Try to parse the symbol
    let Ok(parsed) = SymbolParser::parse(symbol_str) else {
        // If parsing fails, fall back to default_type
        return MarketType::from(default_type);
    };

    // Check if the symbol explicitly specifies a market type (has settle currency)
    // A symbol with settle currency (e.g., "BTC/USDT:USDT") explicitly specifies its type
    if parsed.settle.is_some() {
        // Symbol explicitly specifies a type, use it
        match parsed.market_type() {
            SymbolMarketType::Spot => MarketType::Spot,
            SymbolMarketType::Swap => MarketType::Swap,
            SymbolMarketType::Futures => MarketType::Futures,
        }
    } else {
        // Symbol is a spot symbol (no settle), use default_type
        MarketType::from(default_type)
    }
}

/// Contract settlement type for derivatives
///
/// Determines whether contracts are settled in quote currency (linear/USDT-margined)
/// or base currency (inverse/coin-margined).
///
/// # Variants
///
/// - `Linear`: Settled in quote currency (USDT-margined)
/// - `Inverse`: Settled in base currency (coin-margined)
///
/// # Example
///
/// ```rust
/// use ccxt_core::types::default_type::DefaultSubType;
/// use std::str::FromStr;
///
/// let linear = DefaultSubType::default();
/// assert_eq!(linear, DefaultSubType::Linear);
///
/// let inverse = DefaultSubType::from_str("inverse").unwrap();
/// assert_eq!(inverse.to_string(), "inverse");
/// ```
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DefaultSubType {
    /// Linear contract - settled in quote currency (USDT-margined)
    #[default]
    Linear,
    /// Inverse contract - settled in base currency (coin-margined)
    Inverse,
}

impl DefaultSubType {
    /// Returns the string representation (lowercase)
    ///
    /// # Example
    ///
    /// ```rust
    /// use ccxt_core::types::default_type::DefaultSubType;
    ///
    /// assert_eq!(DefaultSubType::Linear.as_str(), "linear");
    /// assert_eq!(DefaultSubType::Inverse.as_str(), "inverse");
    /// ```
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Linear => "linear",
            Self::Inverse => "inverse",
        }
    }

    /// Infer sub-type from a parsed symbol
    ///
    /// Returns `Some(Linear)` if settle equals quote currency,
    /// `Some(Inverse)` if settle equals base currency,
    /// or `None` if no settle currency is present (spot market).
    ///
    /// # Example
    ///
    /// ```rust
    /// use ccxt_core::types::default_type::DefaultSubType;
    /// use ccxt_core::types::symbol::ParsedSymbol;
    ///
    /// // Linear swap (USDT-margined)
    /// let linear_symbol = ParsedSymbol::linear_swap("BTC".to_string(), "USDT".to_string());
    /// assert_eq!(DefaultSubType::from_symbol(&linear_symbol), Some(DefaultSubType::Linear));
    ///
    /// // Inverse swap (coin-margined)
    /// let inverse_symbol = ParsedSymbol::inverse_swap("BTC".to_string(), "USD".to_string());
    /// assert_eq!(DefaultSubType::from_symbol(&inverse_symbol), Some(DefaultSubType::Inverse));
    ///
    /// // Spot (no sub-type)
    /// let spot_symbol = ParsedSymbol::spot("BTC".to_string(), "USDT".to_string());
    /// assert_eq!(DefaultSubType::from_symbol(&spot_symbol), None);
    /// ```
    pub fn from_symbol(symbol: &ParsedSymbol) -> Option<Self> {
        symbol.settle.as_ref().map(|settle| {
            if settle == &symbol.quote {
                Self::Linear
            } else if settle == &symbol.base {
                Self::Inverse
            } else {
                // Default to linear if settle doesn't match either
                Self::Linear
            }
        })
    }
}

impl fmt::Display for DefaultSubType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for DefaultSubType {
    type Err = DefaultTypeError;

    /// Parse a DefaultSubType from a string (case-insensitive)
    ///
    /// # Example
    ///
    /// ```rust
    /// use ccxt_core::types::default_type::DefaultSubType;
    /// use std::str::FromStr;
    ///
    /// assert_eq!(DefaultSubType::from_str("linear").unwrap(), DefaultSubType::Linear);
    /// assert_eq!(DefaultSubType::from_str("INVERSE").unwrap(), DefaultSubType::Inverse);
    ///
    /// assert!(DefaultSubType::from_str("invalid").is_err());
    /// ```
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "linear" => Ok(Self::Linear),
            "inverse" => Ok(Self::Inverse),
            _ => Err(DefaultTypeError::InvalidSubType(s.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // DefaultType tests

    #[test]
    fn test_default_type_default() {
        assert_eq!(DefaultType::default(), DefaultType::Spot);
    }

    #[test]
    fn test_default_type_as_str() {
        assert_eq!(DefaultType::Spot.as_str(), "spot");
        assert_eq!(DefaultType::Swap.as_str(), "swap");
        assert_eq!(DefaultType::Futures.as_str(), "futures");
        assert_eq!(DefaultType::Margin.as_str(), "margin");
        assert_eq!(DefaultType::Option.as_str(), "option");
    }

    #[test]
    fn test_default_type_display() {
        assert_eq!(DefaultType::Spot.to_string(), "spot");
        assert_eq!(DefaultType::Swap.to_string(), "swap");
        assert_eq!(DefaultType::Futures.to_string(), "futures");
        assert_eq!(DefaultType::Margin.to_string(), "margin");
        assert_eq!(DefaultType::Option.to_string(), "option");
    }

    #[test]
    fn test_default_type_from_str_lowercase() {
        assert_eq!(DefaultType::from_str("spot").unwrap(), DefaultType::Spot);
        assert_eq!(DefaultType::from_str("swap").unwrap(), DefaultType::Swap);
        assert_eq!(
            DefaultType::from_str("futures").unwrap(),
            DefaultType::Futures
        );
        assert_eq!(
            DefaultType::from_str("margin").unwrap(),
            DefaultType::Margin
        );
        assert_eq!(
            DefaultType::from_str("option").unwrap(),
            DefaultType::Option
        );
    }

    #[test]
    fn test_default_type_from_str_uppercase() {
        assert_eq!(DefaultType::from_str("SPOT").unwrap(), DefaultType::Spot);
        assert_eq!(DefaultType::from_str("SWAP").unwrap(), DefaultType::Swap);
        assert_eq!(
            DefaultType::from_str("FUTURES").unwrap(),
            DefaultType::Futures
        );
        assert_eq!(
            DefaultType::from_str("MARGIN").unwrap(),
            DefaultType::Margin
        );
        assert_eq!(
            DefaultType::from_str("OPTION").unwrap(),
            DefaultType::Option
        );
    }

    #[test]
    fn test_default_type_from_str_mixed_case() {
        assert_eq!(DefaultType::from_str("Spot").unwrap(), DefaultType::Spot);
        assert_eq!(DefaultType::from_str("sWaP").unwrap(), DefaultType::Swap);
        assert_eq!(
            DefaultType::from_str("FuTuReS").unwrap(),
            DefaultType::Futures
        );
    }

    #[test]
    fn test_default_type_from_str_invalid() {
        let err = DefaultType::from_str("invalid").unwrap_err();
        assert!(matches!(err, DefaultTypeError::InvalidType(_)));
        assert!(err.to_string().contains("invalid"));
        assert!(err.to_string().contains("spot"));
        assert!(err.to_string().contains("swap"));
        assert!(err.to_string().contains("futures"));
        assert!(err.to_string().contains("margin"));
        assert!(err.to_string().contains("option"));
    }

    #[test]
    fn test_default_type_is_derivative() {
        assert!(!DefaultType::Spot.is_derivative());
        assert!(DefaultType::Swap.is_derivative());
        assert!(DefaultType::Futures.is_derivative());
        assert!(DefaultType::Margin.is_derivative());
        assert!(DefaultType::Option.is_derivative());
    }

    #[test]
    fn test_default_type_is_contract() {
        assert!(!DefaultType::Spot.is_contract());
        assert!(DefaultType::Swap.is_contract());
        assert!(DefaultType::Futures.is_contract());
        assert!(!DefaultType::Margin.is_contract());
        assert!(DefaultType::Option.is_contract());
    }

    #[test]
    fn test_default_type_all_variants() {
        assert_eq!(DefaultType::ALL.len(), 5);
        assert!(DefaultType::ALL.contains(&DefaultType::Spot));
        assert!(DefaultType::ALL.contains(&DefaultType::Swap));
        assert!(DefaultType::ALL.contains(&DefaultType::Futures));
        assert!(DefaultType::ALL.contains(&DefaultType::Margin));
        assert!(DefaultType::ALL.contains(&DefaultType::Option));
    }

    #[test]
    fn test_default_type_to_market_type() {
        assert_eq!(MarketType::from(DefaultType::Spot), MarketType::Spot);
        assert_eq!(MarketType::from(DefaultType::Swap), MarketType::Swap);
        assert_eq!(MarketType::from(DefaultType::Futures), MarketType::Futures);
        assert_eq!(MarketType::from(DefaultType::Margin), MarketType::Spot); // Margin -> Spot
        assert_eq!(MarketType::from(DefaultType::Option), MarketType::Option);
    }

    #[test]
    fn test_market_type_to_default_type() {
        assert_eq!(
            DefaultType::try_from(MarketType::Spot).unwrap(),
            DefaultType::Spot
        );
        assert_eq!(
            DefaultType::try_from(MarketType::Swap).unwrap(),
            DefaultType::Swap
        );
        assert_eq!(
            DefaultType::try_from(MarketType::Futures).unwrap(),
            DefaultType::Futures
        );
        assert_eq!(
            DefaultType::try_from(MarketType::Option).unwrap(),
            DefaultType::Option
        );
    }

    #[test]
    fn test_default_type_serialization() {
        let spot = DefaultType::Spot;
        let json = serde_json::to_string(&spot).unwrap();
        assert_eq!(json, "\"spot\"");

        let swap = DefaultType::Swap;
        let json = serde_json::to_string(&swap).unwrap();
        assert_eq!(json, "\"swap\"");
    }

    #[test]
    fn test_default_type_deserialization() {
        let spot: DefaultType = serde_json::from_str("\"spot\"").unwrap();
        assert_eq!(spot, DefaultType::Spot);

        let swap: DefaultType = serde_json::from_str("\"swap\"").unwrap();
        assert_eq!(swap, DefaultType::Swap);
    }

    // DefaultSubType tests

    #[test]
    fn test_default_sub_type_default() {
        assert_eq!(DefaultSubType::default(), DefaultSubType::Linear);
    }

    #[test]
    fn test_default_sub_type_as_str() {
        assert_eq!(DefaultSubType::Linear.as_str(), "linear");
        assert_eq!(DefaultSubType::Inverse.as_str(), "inverse");
    }

    #[test]
    fn test_default_sub_type_display() {
        assert_eq!(DefaultSubType::Linear.to_string(), "linear");
        assert_eq!(DefaultSubType::Inverse.to_string(), "inverse");
    }

    #[test]
    fn test_default_sub_type_from_str() {
        assert_eq!(
            DefaultSubType::from_str("linear").unwrap(),
            DefaultSubType::Linear
        );
        assert_eq!(
            DefaultSubType::from_str("inverse").unwrap(),
            DefaultSubType::Inverse
        );
        assert_eq!(
            DefaultSubType::from_str("LINEAR").unwrap(),
            DefaultSubType::Linear
        );
        assert_eq!(
            DefaultSubType::from_str("INVERSE").unwrap(),
            DefaultSubType::Inverse
        );
    }

    #[test]
    fn test_default_sub_type_from_str_invalid() {
        let err = DefaultSubType::from_str("invalid").unwrap_err();
        assert!(matches!(err, DefaultTypeError::InvalidSubType(_)));
        assert!(err.to_string().contains("invalid"));
        assert!(err.to_string().contains("linear"));
        assert!(err.to_string().contains("inverse"));
    }

    #[test]
    fn test_default_sub_type_from_symbol_linear() {
        let symbol = ParsedSymbol::linear_swap("BTC".to_string(), "USDT".to_string());
        assert_eq!(
            DefaultSubType::from_symbol(&symbol),
            Some(DefaultSubType::Linear)
        );
    }

    #[test]
    fn test_default_sub_type_from_symbol_inverse() {
        let symbol = ParsedSymbol::inverse_swap("BTC".to_string(), "USD".to_string());
        assert_eq!(
            DefaultSubType::from_symbol(&symbol),
            Some(DefaultSubType::Inverse)
        );
    }

    #[test]
    fn test_default_sub_type_from_symbol_spot() {
        let symbol = ParsedSymbol::spot("BTC".to_string(), "USDT".to_string());
        assert_eq!(DefaultSubType::from_symbol(&symbol), None);
    }

    #[test]
    fn test_default_sub_type_serialization() {
        let linear = DefaultSubType::Linear;
        let json = serde_json::to_string(&linear).unwrap();
        assert_eq!(json, "\"linear\"");

        let inverse = DefaultSubType::Inverse;
        let json = serde_json::to_string(&inverse).unwrap();
        assert_eq!(json, "\"inverse\"");
    }

    #[test]
    fn test_default_sub_type_deserialization() {
        let linear: DefaultSubType = serde_json::from_str("\"linear\"").unwrap();
        assert_eq!(linear, DefaultSubType::Linear);

        let inverse: DefaultSubType = serde_json::from_str("\"inverse\"").unwrap();
        assert_eq!(inverse, DefaultSubType::Inverse);
    }

    // resolve_market_type tests

    #[test]
    fn test_resolve_market_type_no_symbol() {
        // No symbol provided, should use default_type
        assert_eq!(
            resolve_market_type(None, DefaultType::Spot),
            MarketType::Spot
        );
        assert_eq!(
            resolve_market_type(None, DefaultType::Swap),
            MarketType::Swap
        );
        assert_eq!(
            resolve_market_type(None, DefaultType::Futures),
            MarketType::Futures
        );
        assert_eq!(
            resolve_market_type(None, DefaultType::Margin),
            MarketType::Spot // Margin maps to Spot
        );
        assert_eq!(
            resolve_market_type(None, DefaultType::Option),
            MarketType::Option
        );
    }

    #[test]
    fn test_resolve_market_type_spot_symbol_uses_default() {
        // Spot symbol (no settle) should use default_type
        assert_eq!(
            resolve_market_type(Some("BTC/USDT"), DefaultType::Spot),
            MarketType::Spot
        );
        assert_eq!(
            resolve_market_type(Some("BTC/USDT"), DefaultType::Swap),
            MarketType::Swap
        );
        assert_eq!(
            resolve_market_type(Some("ETH/BTC"), DefaultType::Futures),
            MarketType::Futures
        );
    }

    #[test]
    fn test_resolve_market_type_swap_symbol_overrides_default() {
        // Swap symbol (has settle, no expiry) should override default_type
        assert_eq!(
            resolve_market_type(Some("BTC/USDT:USDT"), DefaultType::Spot),
            MarketType::Swap
        );
        assert_eq!(
            resolve_market_type(Some("BTC/USD:BTC"), DefaultType::Spot),
            MarketType::Swap
        );
        assert_eq!(
            resolve_market_type(Some("ETH/USDT:USDT"), DefaultType::Futures),
            MarketType::Swap
        );
    }

    #[test]
    fn test_resolve_market_type_futures_symbol_overrides_default() {
        // Futures symbol (has settle and expiry) should override default_type
        assert_eq!(
            resolve_market_type(Some("BTC/USDT:USDT-241231"), DefaultType::Spot),
            MarketType::Futures
        );
        assert_eq!(
            resolve_market_type(Some("BTC/USD:BTC-250315"), DefaultType::Swap),
            MarketType::Futures
        );
    }

    #[test]
    fn test_resolve_market_type_invalid_symbol_uses_default() {
        // Invalid symbol should fall back to default_type
        assert_eq!(
            resolve_market_type(Some("invalid"), DefaultType::Spot),
            MarketType::Spot
        );
        assert_eq!(
            resolve_market_type(Some(""), DefaultType::Swap),
            MarketType::Swap
        );
        assert_eq!(
            resolve_market_type(Some("BTC"), DefaultType::Futures),
            MarketType::Futures
        );
        assert_eq!(
            resolve_market_type(Some("BTC/USDT:USDT:EXTRA"), DefaultType::Spot),
            MarketType::Spot
        );
    }

    #[test]
    fn test_resolve_market_type_case_insensitive_symbol() {
        // Symbol parsing should be case-insensitive
        assert_eq!(
            resolve_market_type(Some("btc/usdt:usdt"), DefaultType::Spot),
            MarketType::Swap
        );
        assert_eq!(
            resolve_market_type(Some("BTC/usdt"), DefaultType::Swap),
            MarketType::Swap
        );
    }
}
