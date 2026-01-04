//! # Error Handling for CCXT Rust
//!
//! This module provides a comprehensive, production-grade error handling system for the
//! `ccxt-rust` library. It is designed following Rust community best practices and the
//! principles outlined in the [Error Handling Project Group](https://blog.rust-lang.org/inside-rust/2021/07/01/What-the-error-handling-project-group-is-working-towards.html).
//!
//! ## Design Philosophy
//!
//! The error handling system is built around these core principles:
//!
//! 1. **Type Safety**: Strongly-typed errors using `thiserror` for compile-time guarantees
//! 2. **API Stability**: All public enums use `#[non_exhaustive]` for forward compatibility
//! 3. **Zero Panic**: No `unwrap()` or `expect()` on recoverable error paths
//! 4. **Context Rich**: Full error chain support with context attachment
//! 5. **Performance**: Optimized memory layout using `Cow<'static, str>` and `Box`
//! 6. **Thread Safety**: All error types implement `Send + Sync + 'static`
//! 7. **Observability**: Integration with `tracing` for structured logging
//!
//! ## Error Hierarchy
//!
//! ```text
//! Error (main error type)
//! ├── Exchange      - Exchange-specific API errors
//! ├── Network       - Network/transport layer errors (via NetworkError)
//! ├── Parse         - Response parsing errors (via ParseError)
//! ├── Order         - Order management errors (via OrderError)
//! ├── Authentication - API key/signature errors
//! ├── RateLimit     - Rate limiting with retry information
//! ├── Timeout       - Operation timeout
//! ├── InvalidRequest - Invalid parameters
//! ├── MarketNotFound - Unknown trading pair
//! ├── WebSocket     - WebSocket communication errors
//! └── Context       - Error with additional context
//! ```
//!
//! ## Quick Start
//!
//! ### Basic Error Handling
//!
//! ```rust
//! use ccxt_core::error::{Error, Result};
//!
//! fn fetch_price(symbol: &str) -> Result<f64> {
//!     if symbol.is_empty() {
//!         return Err(Error::invalid_request("Symbol cannot be empty"));
//!     }
//!     // ... fetch price logic
//!     Ok(42000.0)
//! }
//! ```
//!
//! ### Adding Context to Errors
//!
//! ```rust
//! use ccxt_core::error::{Error, Result, ContextExt};
//!
//! fn process_order(order_id: &str) -> Result<()> {
//!     validate_order(order_id)
//!         .context("Failed to validate order")?;
//!
//!     submit_order(order_id)
//!         .with_context(|| format!("Failed to submit order {}", order_id))?;
//!
//!     Ok(())
//! }
//! # fn validate_order(_: &str) -> Result<()> { Ok(()) }
//! # fn submit_order(_: &str) -> Result<()> { Ok(()) }
//! ```
//!
//! ### Handling Specific Error Types
//!
//! ```rust
//! use ccxt_core::error::{Error, NetworkError};
//!
//! fn handle_error(err: Error) {
//!     // Check if error is retryable
//!     if err.is_retryable() {
//!         if let Some(duration) = err.retry_after() {
//!             println!("Retry after {:?}", duration);
//!         }
//!     }
//!
//!     // Check for specific error types through context layers
//!     if let Some(msg) = err.as_authentication() {
//!         println!("Auth error: {}", msg);
//!     }
//!
//!     // Get full error report
//!     println!("Error report:\n{}", err.report());
//! }
//! ```
//!
//! ### Creating Exchange Errors
//!
//! ```rust
//! use ccxt_core::error::Error;
//!
//! // Simple exchange error
//! let err = Error::exchange("-1121", "Invalid symbol");
//!
//! // Exchange error with raw response data
//! let err = Error::exchange_with_data(
//!     "400",
//!     "Bad Request",
//!     serde_json::json!({"code": -1121, "msg": "Invalid symbol"})
//! );
//! ```
//!
//! ## Memory Optimization
//!
//! The `Error` enum is optimized to be ≤56 bytes on 64-bit systems:
//!
//! - Large variants (`Exchange`, `Network`, `Parse`, `Order`, `WebSocket`, `Context`)
//!   are boxed to keep the enum size small
//! - String fields use `Cow<'static, str>` to avoid allocation for static strings
//! - Use `Error::authentication("static message")` for zero-allocation errors
//! - Use `Error::authentication(format!("dynamic {}", value))` when needed
//!
//! ## Feature Flags
//!
//! - `backtrace`: Enable backtrace capture in `ExchangeErrorDetails` for debugging
//!
//! ## Integration with anyhow
//!
//! For application-level code, errors can be converted to `anyhow::Error`:
//!
//! ```rust
//! use ccxt_core::error::Error;
//!
//! fn app_main() -> anyhow::Result<()> {
//!     let result: Result<(), Error> = Err(Error::timeout("Operation timed out"));
//!     result?; // Automatically converts to anyhow::Error
//!     Ok(())
//! }
//! ```

mod context;
mod convert;
mod details;
mod network;
mod order;
mod parse;

use std::borrow::Cow;
use std::error::Error as StdError;
use std::fmt;
use std::time::Duration;
use thiserror::Error;

// Re-export all public types for backward compatibility
pub use context::ContextExt;
#[allow(deprecated)]
pub use context::ErrorContext;
pub use details::ExchangeErrorDetails;
pub use network::NetworkError;
pub use order::OrderError;
pub use parse::ParseError;

/// Result type alias for all CCXT operations.
pub type Result<T> = std::result::Result<T, Error>;

/// The primary error type for the `ccxt-rust` library.
///
/// Design constraints:
/// - All large variants are boxed to keep enum size ≤ 56 bytes
/// - Uses `Cow<'static, str>` for zero-allocation static strings
/// - Verify with: `assert!(std::mem::size_of::<Error>() <= 56);`
///
/// # Example
///
/// ```rust
/// use ccxt_core::error::Error;
///
/// let err = Error::authentication("Invalid API key");
/// assert!(err.to_string().contains("Invalid API key"));
/// ```
#[derive(Error, Debug)]
#[non_exhaustive]
pub enum Error {
    /// Exchange-specific errors returned by the exchange API.
    /// Boxed to reduce enum size (`ExchangeErrorDetails` is large).
    #[error("Exchange error: {0}")]
    Exchange(Box<ExchangeErrorDetails>),

    /// Network-related errors encapsulating transport layer issues.
    /// Boxed to reduce enum size.
    #[error("Network error: {0}")]
    Network(Box<NetworkError>),

    /// Authentication errors (invalid API key, signature, etc.).
    #[error("Authentication error: {0}")]
    Authentication(Cow<'static, str>),

    /// Rate limit exceeded with optional retry information.
    #[error("Rate limit exceeded: {message}")]
    RateLimit {
        /// Error message
        message: Cow<'static, str>,
        /// Optional duration to wait before retrying
        retry_after: Option<Duration>,
    },

    /// Invalid request parameters.
    #[error("Invalid request: {0}")]
    InvalidRequest(Cow<'static, str>),

    /// Order-related errors. Boxed to reduce enum size.
    #[error("Order error: {0}")]
    Order(Box<OrderError>),

    /// Insufficient balance for an operation.
    #[error("Insufficient balance: {0}")]
    InsufficientBalance(Cow<'static, str>),

    /// Invalid order format or parameters.
    #[error("Invalid order: {0}")]
    InvalidOrder(Cow<'static, str>),

    /// Order not found on the exchange.
    #[error("Order not found: {0}")]
    OrderNotFound(Cow<'static, str>),

    /// Market symbol not found or not supported.
    #[error("Market not found: {0}")]
    MarketNotFound(Cow<'static, str>),

    /// Errors during response parsing. Boxed to reduce enum size.
    #[error("Parse error: {0}")]
    Parse(Box<ParseError>),

    /// WebSocket communication errors.
    /// Uses `Box<dyn StdError>` to preserve original error for downcast.
    #[error("WebSocket error: {0}")]
    WebSocket(#[source] Box<dyn StdError + Send + Sync + 'static>),

    /// Operation timeout.
    #[error("Timeout: {0}")]
    Timeout(Cow<'static, str>),

    /// Feature not implemented for this exchange.
    #[error("Not implemented: {0}")]
    NotImplemented(Cow<'static, str>),

    /// Operation was cancelled.
    ///
    /// This error is returned when an operation is cancelled via a `CancellationToken`
    /// or other cancellation mechanism. It indicates that the operation was intentionally
    /// aborted and did not complete.
    ///
    /// # Example
    ///
    /// ```rust
    /// use ccxt_core::error::Error;
    ///
    /// let err = Error::cancelled("WebSocket connection cancelled");
    /// assert!(err.to_string().contains("cancelled"));
    /// ```
    #[error("Cancelled: {0}")]
    Cancelled(Cow<'static, str>),

    /// Resource exhausted error.
    ///
    /// This error is returned when a resource limit has been reached, such as
    /// maximum number of WebSocket subscriptions, connection pool exhaustion,
    /// or other capacity limits.
    ///
    /// # Example
    ///
    /// ```rust
    /// use ccxt_core::error::Error;
    ///
    /// let err = Error::resource_exhausted("Maximum subscriptions (100) reached");
    /// assert!(err.to_string().contains("Resource exhausted"));
    /// ```
    #[error("Resource exhausted: {0}")]
    ResourceExhausted(Cow<'static, str>),

    /// Error with additional context, preserving the error chain.
    #[error("{context}")]
    Context {
        /// Context message describing what operation failed
        context: String,
        /// The underlying error
        #[source]
        source: Box<Error>,
    },
}

impl Error {
    // ==================== Constructor Methods ====================

    /// Creates a new exchange error.
    ///
    /// # Example
    ///
    /// ```rust
    /// use ccxt_core::error::Error;
    ///
    /// let err = Error::exchange("400", "Bad Request");
    /// ```
    pub fn exchange(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Exchange(Box::new(ExchangeErrorDetails::new(code, message)))
    }

    /// Creates a new exchange error with raw response data.
    pub fn exchange_with_data(
        code: impl Into<String>,
        message: impl Into<String>,
        data: serde_json::Value,
    ) -> Self {
        Self::Exchange(Box::new(ExchangeErrorDetails::with_data(
            code, message, data,
        )))
    }

    /// Creates a new rate limit error with optional retry duration.
    /// Accepts both `&'static str` (zero allocation) and `String`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use ccxt_core::error::Error;
    /// use std::time::Duration;
    ///
    /// // Zero allocation (static string):
    /// let err = Error::rate_limit("Too many requests", Some(Duration::from_secs(60)));
    ///
    /// // Allocation (dynamic string):
    /// let err = Error::rate_limit(format!("Rate limit: {}", 429), None);
    /// ```
    pub fn rate_limit(
        message: impl Into<Cow<'static, str>>,
        retry_after: Option<Duration>,
    ) -> Self {
        Self::RateLimit {
            message: message.into(),
            retry_after,
        }
    }

    /// Creates an authentication error.
    /// Accepts both `&'static str` (zero allocation) and `String`.
    pub fn authentication(msg: impl Into<Cow<'static, str>>) -> Self {
        Self::Authentication(msg.into())
    }

    /// Creates a generic error (for backwards compatibility).
    pub fn generic(msg: impl Into<Cow<'static, str>>) -> Self {
        Self::InvalidRequest(msg.into())
    }

    /// Creates a network error from a message.
    pub fn network(msg: impl Into<String>) -> Self {
        Self::Network(Box::new(NetworkError::ConnectionFailed(msg.into())))
    }

    /// Creates a market not found error.
    /// Accepts both `&'static str` (zero allocation) and `String`.
    pub fn market_not_found(symbol: impl Into<Cow<'static, str>>) -> Self {
        Self::MarketNotFound(symbol.into())
    }

    /// Creates a not implemented error.
    /// Accepts both `&'static str` (zero allocation) and `String`.
    pub fn not_implemented(feature: impl Into<Cow<'static, str>>) -> Self {
        Self::NotImplemented(feature.into())
    }

    /// Creates a cancelled error.
    ///
    /// Use this when an operation is cancelled via a `CancellationToken` or
    /// other cancellation mechanism.
    ///
    /// Accepts both `&'static str` (zero allocation) and `String`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use ccxt_core::error::Error;
    ///
    /// // Zero allocation (static string):
    /// let err = Error::cancelled("Operation cancelled by user");
    ///
    /// // Allocation (dynamic string):
    /// let err = Error::cancelled(format!("Connection {} cancelled", "ws-1"));
    /// ```
    pub fn cancelled(msg: impl Into<Cow<'static, str>>) -> Self {
        Self::Cancelled(msg.into())
    }

    /// Creates a resource exhausted error.
    ///
    /// Use this when a resource limit has been reached, such as maximum
    /// WebSocket subscriptions, connection pool exhaustion, or other
    /// capacity limits.
    ///
    /// Accepts both `&'static str` (zero allocation) and `String`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use ccxt_core::error::Error;
    ///
    /// // Zero allocation (static string):
    /// let err = Error::resource_exhausted("Maximum subscriptions reached");
    ///
    /// // Allocation (dynamic string):
    /// let err = Error::resource_exhausted(format!("Maximum subscriptions ({}) reached", 100));
    /// ```
    pub fn resource_exhausted(msg: impl Into<Cow<'static, str>>) -> Self {
        Self::ResourceExhausted(msg.into())
    }

    /// Creates an invalid request error.
    pub fn invalid_request(msg: impl Into<Cow<'static, str>>) -> Self {
        Self::InvalidRequest(msg.into())
    }

    /// Creates an invalid argument error (alias for `invalid_request`).
    pub fn invalid_argument(msg: impl Into<Cow<'static, str>>) -> Self {
        Self::InvalidRequest(msg.into())
    }

    /// Creates a bad symbol error (alias for `invalid_request`).
    pub fn bad_symbol(symbol: impl Into<String>) -> Self {
        let s = symbol.into();
        Self::InvalidRequest(Cow::Owned(format!("Bad symbol: {s}")))
    }

    /// Creates an insufficient balance error.
    pub fn insufficient_balance(msg: impl Into<Cow<'static, str>>) -> Self {
        Self::InsufficientBalance(msg.into())
    }

    /// Creates a timeout error.
    pub fn timeout(msg: impl Into<Cow<'static, str>>) -> Self {
        Self::Timeout(msg.into())
    }

    /// Creates a WebSocket error from a message string.
    pub fn websocket(msg: impl Into<String>) -> Self {
        Self::WebSocket(Box::new(SimpleError(msg.into())))
    }

    /// Creates a WebSocket error from any error type.
    pub fn websocket_error<E: StdError + Send + Sync + 'static>(err: E) -> Self {
        Self::WebSocket(Box::new(err))
    }

    // ==================== Context Methods ====================

    /// Attaches context to an existing error.
    ///
    /// # Example
    ///
    /// ```rust
    /// use ccxt_core::error::Error;
    ///
    /// let err = Error::network("Connection refused")
    ///     .context("Failed to fetch ticker for BTC/USDT");
    /// ```
    #[must_use]
    pub fn context(self, context: impl Into<String>) -> Self {
        Self::Context {
            context: context.into(),
            source: Box::new(self),
        }
    }

    // ==================== Chain Traversal Methods ====================

    /// Internal helper: creates an iterator that traverses the error chain.
    /// Automatically penetrates Context layers.
    fn iter_chain(&self) -> impl Iterator<Item = &Error> {
        std::iter::successors(Some(self), |err| match err {
            Error::Context { source, .. } => Some(source.as_ref()),
            _ => None,
        })
    }

    /// Returns the root cause of the error, skipping Context layers.
    #[must_use]
    pub fn root_cause(&self) -> &Error {
        self.iter_chain().last().unwrap_or(self)
    }

    /// Finds a specific error variant in the chain (penetrates Context layers).
    /// Useful for handling wrapped errors without manual unwrapping.
    pub fn find_variant<F>(&self, matcher: F) -> Option<&Error>
    where
        F: Fn(&Error) -> bool,
    {
        self.iter_chain().find(|e| matcher(e))
    }

    /// Generates a detailed error report with the full chain.
    ///
    /// # Example
    ///
    /// ```rust
    /// use ccxt_core::error::Error;
    ///
    /// let err = Error::network("Connection refused")
    ///     .context("Failed to fetch ticker");
    /// println!("{}", err.report());
    /// // Output:
    /// // Failed to fetch ticker
    /// // Caused by: Network error: Connection failed: Connection refused
    /// ```
    #[must_use]
    pub fn report(&self) -> String {
        use std::fmt::Write;
        let mut report = String::new();
        report.push_str(&self.to_string());

        let mut current: Option<&(dyn StdError + 'static)> = self.source();
        while let Some(err) = current {
            let _ = write!(report, "\nCaused by: {err}");
            current = err.source();
        }
        report
    }

    // ==================== Helper Methods (Context Penetrating) ====================

    /// Checks if this error is retryable (penetrates Context layers).
    ///
    /// Returns `true` for:
    /// - `NetworkError::Timeout`
    /// - `NetworkError::ConnectionFailed`
    /// - `RateLimit`
    /// - `Timeout`
    #[must_use]
    pub fn is_retryable(&self) -> bool {
        match self {
            Error::Network(ne) => matches!(
                ne.as_ref(),
                NetworkError::Timeout | NetworkError::ConnectionFailed(_)
            ),
            Error::RateLimit { .. } | Error::Timeout(_) => true,
            Error::Context { source, .. } => source.is_retryable(),
            _ => false,
        }
    }

    /// Returns the retry delay if this is a rate limit error (penetrates Context layers).
    #[must_use]
    pub fn retry_after(&self) -> Option<Duration> {
        match self {
            Error::RateLimit { retry_after, .. } => *retry_after,
            Error::Context { source, .. } => source.retry_after(),
            _ => None,
        }
    }

    /// Checks if this is a rate limit error (penetrates Context layers).
    /// Returns the message and optional retry duration.
    #[must_use]
    pub fn as_rate_limit(&self) -> Option<(&str, Option<Duration>)> {
        match self {
            Error::RateLimit {
                message,
                retry_after,
            } => Some((message.as_ref(), *retry_after)),
            Error::Context { source, .. } => source.as_rate_limit(),
            _ => None,
        }
    }

    /// Checks if this is an authentication error (penetrates Context layers).
    /// Returns the error message.
    #[must_use]
    pub fn as_authentication(&self) -> Option<&str> {
        match self {
            Error::Authentication(msg) => Some(msg.as_ref()),
            Error::Context { source, .. } => source.as_authentication(),
            _ => None,
        }
    }

    /// Checks if this is a cancelled error (penetrates Context layers).
    /// Returns the error message.
    ///
    /// # Example
    ///
    /// ```rust
    /// use ccxt_core::error::Error;
    ///
    /// let err = Error::cancelled("Operation cancelled");
    /// assert_eq!(err.as_cancelled(), Some("Operation cancelled"));
    ///
    /// // Works through context layers
    /// let wrapped = err.context("Wrapped error");
    /// assert_eq!(wrapped.as_cancelled(), Some("Operation cancelled"));
    /// ```
    #[must_use]
    pub fn as_cancelled(&self) -> Option<&str> {
        match self {
            Error::Cancelled(msg) => Some(msg.as_ref()),
            Error::Context { source, .. } => source.as_cancelled(),
            _ => None,
        }
    }

    /// Checks if this is a resource exhausted error (penetrates Context layers).
    /// Returns the error message.
    ///
    /// # Example
    ///
    /// ```rust
    /// use ccxt_core::error::Error;
    ///
    /// let err = Error::resource_exhausted("Maximum subscriptions reached");
    /// assert_eq!(err.as_resource_exhausted(), Some("Maximum subscriptions reached"));
    ///
    /// // Works through context layers
    /// let wrapped = err.context("Wrapped error");
    /// assert_eq!(wrapped.as_resource_exhausted(), Some("Maximum subscriptions reached"));
    /// ```
    #[must_use]
    pub fn as_resource_exhausted(&self) -> Option<&str> {
        match self {
            Error::ResourceExhausted(msg) => Some(msg.as_ref()),
            Error::Context { source, .. } => source.as_resource_exhausted(),
            _ => None,
        }
    }

    /// Attempts to downcast the WebSocket error to a specific type.
    #[must_use]
    pub fn downcast_websocket<T: StdError + 'static>(&self) -> Option<&T> {
        match self {
            Error::WebSocket(e) => e.downcast_ref::<T>(),
            Error::Context { source, .. } => source.downcast_websocket(),
            _ => None,
        }
    }
}

/// A simple error type for wrapping string messages.
/// Used internally for WebSocket errors created from strings.
#[derive(Debug)]
struct SimpleError(String);

impl fmt::Display for SimpleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl StdError for SimpleError {}

#[cfg(test)]
mod tests;
