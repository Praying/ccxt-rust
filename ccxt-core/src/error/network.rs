//! Network-related error types.

use std::error::Error as StdError;
use thiserror::Error;

/// Encapsulated network errors hiding implementation details.
///
/// This type wraps all network-related errors without exposing third-party
/// library types (like `reqwest::Error`) in the public API. This ensures
/// API stability even when underlying HTTP libraries change.
///
/// # Retryable Errors
///
/// The following variants are considered retryable:
/// - [`NetworkError::Timeout`] - Request timed out, may succeed on retry
/// - [`NetworkError::ConnectionFailed`] - Connection failed, may be transient
///
/// # Example
///
/// ```rust
/// use ccxt_core::error::{Error, NetworkError};
///
/// fn handle_network_error(err: NetworkError) {
///     match &err {
///         NetworkError::RequestFailed { status, message } => {
///             println!("HTTP {}: {}", status, message);
///         }
///         NetworkError::Timeout => {
///             println!("Request timed out, consider retrying");
///         }
///         NetworkError::ConnectionFailed(msg) => {
///             println!("Connection failed: {}", msg);
///         }
///         _ => println!("Network error: {}", err),
///     }
/// }
/// ```
#[derive(Error, Debug)]
#[non_exhaustive]
pub enum NetworkError {
    /// Request failed with HTTP status code.
    #[error("Request failed with status {status}: {message}")]
    RequestFailed {
        /// HTTP status code
        status: u16,
        /// Error message
        message: String,
    },

    /// Request timed out.
    #[error("Request timeout")]
    Timeout,

    /// Connection failed.
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),

    /// DNS resolution failed.
    #[error("DNS resolution failed: {0}")]
    DnsResolution(String),

    /// SSL/TLS error.
    #[error("SSL/TLS error: {0}")]
    Ssl(String),

    /// Opaque transport error for underlying issues.
    /// Uses `Box<dyn StdError>` to hide implementation details while preserving the source.
    #[error("Transport error")]
    Transport(#[source] Box<dyn StdError + Send + Sync + 'static>),
}
