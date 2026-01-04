//! Error detail structures for various error types.

use serde_json::Value;

#[cfg(feature = "backtrace")]
use std::backtrace::Backtrace;

/// Details for exchange-specific errors.
///
/// Extracted to a separate struct and boxed to keep Error enum size small.
///
/// Note: `#[non_exhaustive]` allows adding fields in future versions without breaking changes.
///
/// # Example
///
/// ```rust
/// use ccxt_core::error::ExchangeErrorDetails;
///
/// let details = ExchangeErrorDetails::new("400", "Bad Request");
/// assert_eq!(details.code, "400");
/// ```
#[derive(Debug)]
#[non_exhaustive]
pub struct ExchangeErrorDetails {
    /// Error code as String to support all exchange formats (numeric, alphanumeric).
    pub code: String,
    /// Descriptive message from the exchange.
    pub message: String,
    /// Optional raw response data for debugging.
    pub data: Option<Value>,
    /// Backtrace captured at error creation (feature-gated).
    #[cfg(feature = "backtrace")]
    pub backtrace: Backtrace,
}

impl ExchangeErrorDetails {
    /// Creates a new `ExchangeErrorDetails` with the given code and message.
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            data: None,
            #[cfg(feature = "backtrace")]
            backtrace: Backtrace::capture(),
        }
    }

    /// Creates a new `ExchangeErrorDetails` with raw response data.
    pub fn with_data(code: impl Into<String>, message: impl Into<String>, data: Value) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            data: Some(data),
            #[cfg(feature = "backtrace")]
            backtrace: Backtrace::capture(),
        }
    }
}

impl std::fmt::Display for ExchangeErrorDetails {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} (code: {})", self.message, self.code)
    }
}
