//! Parsing-related error types.

use std::borrow::Cow;
use thiserror::Error;

/// Errors related to parsing exchange responses.
///
/// This type handles all parsing failures including JSON deserialization,
/// decimal number parsing, timestamp parsing, and missing/invalid fields.
///
/// # Memory Optimization
///
/// Uses `Cow<'static, str>` for field names and messages to avoid allocation
/// when using static strings. Use the helper constructors for ergonomic creation:
///
/// ```rust
/// use ccxt_core::error::ParseError;
///
/// // Zero allocation (static string)
/// let err = ParseError::missing_field("price");
///
/// // Allocation only when needed (dynamic string)
/// let field_name = format!("field_{}", 42);
/// let err = ParseError::missing_field_owned(field_name);
///
/// // Invalid value with context
/// let err = ParseError::invalid_value("amount", "must be positive");
/// ```
///
/// # Example
///
/// ```rust
/// use ccxt_core::error::{Error, ParseError, Result};
///
/// fn parse_price(json: &serde_json::Value) -> Result<f64> {
///     json.get("price")
///         .and_then(|v| v.as_f64())
///         .ok_or_else(|| Error::from(ParseError::missing_field("price")))
/// }
/// ```
#[derive(Error, Debug)]
#[non_exhaustive]
pub enum ParseError {
    /// Failed to parse decimal number.
    #[error("Failed to parse decimal: {0}")]
    Decimal(#[from] rust_decimal::Error),

    /// Failed to deserialize JSON.
    #[error("Failed to deserialize JSON: {0}")]
    Json(#[from] serde_json::Error),

    /// Failed to parse timestamp.
    #[error("Failed to parse timestamp: {0}")]
    Timestamp(Cow<'static, str>),

    /// Missing required field in response.
    #[error("Missing required field: {0}")]
    MissingField(Cow<'static, str>),

    /// Invalid value for a field.
    #[error("Invalid value for '{field}': {message}")]
    InvalidValue {
        /// Field name
        field: Cow<'static, str>,
        /// Error message
        message: Cow<'static, str>,
    },

    /// Invalid format for a field.
    #[error("Invalid format for '{field}': {message}")]
    InvalidFormat {
        /// Field name
        field: Cow<'static, str>,
        /// Error message
        message: Cow<'static, str>,
    },
}

impl ParseError {
    /// Creates a `MissingField` error with a static string (no allocation).
    #[must_use]
    pub fn missing_field(field: &'static str) -> Self {
        Self::MissingField(Cow::Borrowed(field))
    }

    /// Creates a `MissingField` error with a dynamic string.
    #[must_use]
    pub fn missing_field_owned(field: String) -> Self {
        Self::MissingField(Cow::Owned(field))
    }

    /// Creates an `InvalidValue` error.
    pub fn invalid_value(
        field: impl Into<Cow<'static, str>>,
        message: impl Into<Cow<'static, str>>,
    ) -> Self {
        Self::InvalidValue {
            field: field.into(),
            message: message.into(),
        }
    }

    /// Creates a `Timestamp` error with a static string (no allocation).
    #[must_use]
    pub fn timestamp(message: &'static str) -> Self {
        Self::Timestamp(Cow::Borrowed(message))
    }

    /// Creates a `Timestamp` error with a dynamic string.
    #[must_use]
    pub fn timestamp_owned(message: String) -> Self {
        Self::Timestamp(Cow::Owned(message))
    }

    /// Creates an `InvalidFormat` error.
    pub fn invalid_format(
        field: impl Into<Cow<'static, str>>,
        message: impl Into<Cow<'static, str>>,
    ) -> Self {
        Self::InvalidFormat {
            field: field.into(),
            message: message.into(),
        }
    }
}
