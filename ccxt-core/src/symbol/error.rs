//! Symbol error types for parsing and validation
//!
//! This module provides error types for symbol-related operations including
//! parsing, validation, and conversion errors.

use std::fmt;

/// Symbol-related errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SymbolError {
    /// Invalid symbol format
    InvalidFormat(String),

    /// Missing required component
    MissingComponent(String),

    /// Invalid currency code
    InvalidCurrency(String),

    /// Invalid expiry date
    InvalidExpiryDate {
        /// Year (2-digit)
        year: u8,
        /// Month (1-12)
        month: u8,
        /// Day (1-31)
        day: u8,
    },

    /// Unexpected date suffix in swap symbol
    UnexpectedDateSuffix,

    /// Missing date suffix in futures symbol
    MissingDateSuffix,

    /// Empty symbol string
    EmptySymbol,

    /// Multiple colons in symbol
    MultipleColons,

    /// Invalid date format in symbol
    InvalidDateFormat(String),
}

impl fmt::Display for SymbolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidFormat(msg) => write!(f, "Invalid symbol format: {msg}"),
            Self::MissingComponent(component) => {
                write!(f, "Missing required component: {component}")
            }
            Self::InvalidCurrency(code) => write!(f, "Invalid currency code: {code}"),
            Self::InvalidExpiryDate { year, month, day } => {
                write!(f, "Invalid expiry date: {year:02}{month:02}{day:02}")
            }
            Self::UnexpectedDateSuffix => write!(f, "Unexpected date suffix in swap symbol"),
            Self::MissingDateSuffix => write!(f, "Missing date suffix in futures symbol"),
            Self::EmptySymbol => write!(f, "Symbol string is empty"),
            Self::MultipleColons => write!(f, "Symbol contains multiple colons"),
            Self::InvalidDateFormat(msg) => write!(f, "Invalid date format: {msg}"),
        }
    }
}

impl std::error::Error for SymbolError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        assert_eq!(
            SymbolError::InvalidFormat("test".to_string()).to_string(),
            "Invalid symbol format: test"
        );
        assert_eq!(
            SymbolError::MissingComponent("base".to_string()).to_string(),
            "Missing required component: base"
        );
        assert_eq!(
            SymbolError::InvalidCurrency("123".to_string()).to_string(),
            "Invalid currency code: 123"
        );
        assert_eq!(
            SymbolError::InvalidExpiryDate {
                year: 24,
                month: 13,
                day: 1
            }
            .to_string(),
            "Invalid expiry date: 241301"
        );
        assert_eq!(
            SymbolError::UnexpectedDateSuffix.to_string(),
            "Unexpected date suffix in swap symbol"
        );
        assert_eq!(
            SymbolError::MissingDateSuffix.to_string(),
            "Missing date suffix in futures symbol"
        );
        assert_eq!(
            SymbolError::EmptySymbol.to_string(),
            "Symbol string is empty"
        );
        assert_eq!(
            SymbolError::MultipleColons.to_string(),
            "Symbol contains multiple colons"
        );
    }
}
