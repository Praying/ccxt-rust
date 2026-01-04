//! Context attachment trait and implementations.

use crate::error::{Error, Result};
use std::fmt;

/// Extension trait for ergonomic error context attachment.
///
/// This trait provides methods to add context to errors, making it easier
/// to understand where and why an error occurred. It works with both
/// `Result<T, E>` and `Option<T>` types.
///
/// # When to Use
///
/// - Use `context()` when you have a static context message
/// - Use `with_context()` when the context message is expensive to compute
///   (it's only evaluated on error)
///
/// # Library vs Application Code
///
/// - **Library code**: Use this trait for adding context within the library
/// - **Application code**: Consider using `anyhow::Context` for richer error handling
///
/// # Examples
///
/// ## Adding Context to Results
///
/// ```rust
/// use ccxt_core::error::{Error, Result, ContextExt};
///
/// fn fetch_ticker(symbol: &str) -> Result<f64> {
///     // Static context (always evaluated)
///     let data = fetch_raw_data()
///         .context("Failed to fetch raw data")?;
///
///     // Lazy context (only evaluated on error)
///     parse_price(&data)
///         .with_context(|| format!("Failed to parse price for {}", symbol))
/// }
/// # fn fetch_raw_data() -> Result<String> { Ok("{}".to_string()) }
/// # fn parse_price(_: &str) -> Result<f64> { Ok(42.0) }
/// ```
///
/// ## Adding Context to Options
///
/// ```rust
/// use ccxt_core::error::{Result, ContextExt};
///
/// fn get_required_field(json: &serde_json::Value) -> Result<&str> {
///     json.get("field")
///         .and_then(|v| v.as_str())
///         .context("Missing required field 'field'")
/// }
/// ```
pub trait ContextExt<T, E> {
    /// Adds context to an error.
    fn context<C>(self, context: C) -> Result<T>
    where
        C: fmt::Display + Send + Sync + 'static;

    /// Adds lazy context to an error (only evaluated on error).
    fn with_context<C, F>(self, f: F) -> Result<T>
    where
        C: fmt::Display + Send + Sync + 'static,
        F: FnOnce() -> C;
}

impl<T, E> ContextExt<T, E> for std::result::Result<T, E>
where
    E: Into<Error>,
{
    fn context<C>(self, context: C) -> Result<T>
    where
        C: fmt::Display + Send + Sync + 'static,
    {
        self.map_err(|e| e.into().context(context.to_string()))
    }

    fn with_context<C, F>(self, f: F) -> Result<T>
    where
        C: fmt::Display + Send + Sync + 'static,
        F: FnOnce() -> C,
    {
        self.map_err(|e| e.into().context(f().to_string()))
    }
}

impl<T> ContextExt<T, Error> for Option<T> {
    fn context<C>(self, context: C) -> Result<T>
    where
        C: fmt::Display + Send + Sync + 'static,
    {
        self.ok_or_else(|| Error::generic(context.to_string()))
    }

    fn with_context<C, F>(self, f: F) -> Result<T>
    where
        C: fmt::Display + Send + Sync + 'static,
        F: FnOnce() -> C,
    {
        self.ok_or_else(|| Error::generic(f().to_string()))
    }
}

/// Helper trait for adding context to errors (legacy alias for ContextExt).
#[deprecated(since = "0.2.0", note = "Use ContextExt instead")]
pub trait ErrorContext<T>: Sized {
    /// Add context to an error
    fn context(self, context: impl fmt::Display) -> Result<T>;
}

#[allow(deprecated)]
impl<T, E: Into<Error>> ErrorContext<T> for std::result::Result<T, E> {
    fn context(self, context: impl fmt::Display) -> Result<T> {
        self.map_err(|e| e.into().context(context.to_string()))
    }
}
