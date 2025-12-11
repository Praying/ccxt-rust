//! Binance-specific error types.
//!
//! This module provides module-specific error types for Binance WebSocket operations.
//! These errors can be converted to the core `Error` type while preserving
//! structured information for downcast capability.
//!
//! # Example
//!
//! ```rust
//! use ccxt_exchanges::binance::error::BinanceWsError;
//! use ccxt_core::error::Error as CoreError;
//!
//! // Create a Binance-specific error
//! let ws_err = BinanceWsError::subscription_failed("btcusdt@ticker", "Invalid stream");
//!
//! // Convert to core error for public API
//! let core_err: CoreError = ws_err.into();
//!
//! // Downcast back to access structured data
//! if let Some(binance_err) = core_err.downcast_websocket::<BinanceWsError>() {
//!     if let Some(stream) = binance_err.stream_name() {
//!         println!("Failed stream: {}", stream);
//!     }
//! }
//! ```

use ccxt_core::error::Error as CoreError;
use thiserror::Error;

#[cfg(test)]
use std::error::Error as StdError;

/// Binance WebSocket specific errors.
///
/// Implements `StdError + Send + Sync` for use with `CoreError::WebSocket`.
/// Uses `#[non_exhaustive]` to allow adding variants in future versions.
///
/// # Variants
///
/// - `MissingStream`: Stream name missing in WebSocket message
/// - `UnsupportedEvent`: Received an unsupported event type
/// - `SubscriptionFailed`: Stream subscription failed
/// - `Connection`: WebSocket connection error
/// - `Core`: Passthrough for core errors
#[derive(Error, Debug)]
#[non_exhaustive]
pub enum BinanceWsError {
    /// Stream name missing in message.
    ///
    /// This occurs when a WebSocket message doesn't contain the expected
    /// stream identifier, making it impossible to route the message.
    #[error("Stream name missing in message: {raw}")]
    MissingStream {
        /// The raw message content (truncated for display)
        raw: String,
    },

    /// Unsupported event type.
    ///
    /// This occurs when the WebSocket receives an event type that
    /// is not handled by the current implementation.
    #[error("Unsupported event type: {event}")]
    UnsupportedEvent {
        /// The event type that was not recognized
        event: String,
    },

    /// Subscription failed.
    ///
    /// This occurs when a stream subscription request is rejected
    /// by the Binance WebSocket server.
    #[error("Subscription failed for {stream}: {reason}")]
    SubscriptionFailed {
        /// The stream name that failed to subscribe
        stream: String,
        /// The reason for the failure
        reason: String,
    },

    /// WebSocket connection error.
    ///
    /// General connection-related errors that don't fit other categories.
    #[error("WebSocket connection error: {0}")]
    Connection(String),

    /// Core error passthrough.
    ///
    /// Allows wrapping core errors within Binance-specific error handling
    /// while maintaining the ability to extract the original error.
    #[error(transparent)]
    Core(#[from] CoreError),
}

impl BinanceWsError {
    /// Creates a new `MissingStream` error.
    ///
    /// # Arguments
    ///
    /// * `raw` - The raw message content (will be truncated if too long)
    ///
    /// # Example
    ///
    /// ```rust
    /// use ccxt_exchanges::binance::error::BinanceWsError;
    ///
    /// let err = BinanceWsError::missing_stream(r#"{"data": "unknown"}"#);
    /// ```
    pub fn missing_stream(raw: impl Into<String>) -> Self {
        let mut raw_str = raw.into();
        // Truncate long messages for display
        if raw_str.len() > 200 {
            raw_str.truncate(200);
            raw_str.push_str("...");
        }
        Self::MissingStream { raw: raw_str }
    }

    /// Creates a new `UnsupportedEvent` error.
    ///
    /// # Arguments
    ///
    /// * `event` - The unsupported event type
    ///
    /// # Example
    ///
    /// ```rust
    /// use ccxt_exchanges::binance::error::BinanceWsError;
    ///
    /// let err = BinanceWsError::unsupported_event("unknownEvent");
    /// ```
    pub fn unsupported_event(event: impl Into<String>) -> Self {
        Self::UnsupportedEvent {
            event: event.into(),
        }
    }

    /// Creates a new `SubscriptionFailed` error.
    ///
    /// # Arguments
    ///
    /// * `stream` - The stream name that failed
    /// * `reason` - The reason for the failure
    ///
    /// # Example
    ///
    /// ```rust
    /// use ccxt_exchanges::binance::error::BinanceWsError;
    ///
    /// let err = BinanceWsError::subscription_failed("btcusdt@ticker", "Invalid symbol");
    /// ```
    pub fn subscription_failed(stream: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::SubscriptionFailed {
            stream: stream.into(),
            reason: reason.into(),
        }
    }

    /// Creates a new `Connection` error.
    ///
    /// # Arguments
    ///
    /// * `message` - The connection error message
    ///
    /// # Example
    ///
    /// ```rust
    /// use ccxt_exchanges::binance::error::BinanceWsError;
    ///
    /// let err = BinanceWsError::connection("Failed to establish connection");
    /// ```
    pub fn connection(message: impl Into<String>) -> Self {
        Self::Connection(message.into())
    }

    /// Returns the stream name if this error is related to a specific stream.
    ///
    /// # Returns
    ///
    /// - `Some(&str)` - The stream name for `SubscriptionFailed` variant
    /// - `None` - For all other variants
    ///
    /// # Example
    ///
    /// ```rust
    /// use ccxt_exchanges::binance::error::BinanceWsError;
    ///
    /// let err = BinanceWsError::subscription_failed("btcusdt@ticker", "Invalid");
    /// assert_eq!(err.stream_name(), Some("btcusdt@ticker"));
    ///
    /// let err = BinanceWsError::connection("Failed");
    /// assert_eq!(err.stream_name(), None);
    /// ```
    pub fn stream_name(&self) -> Option<&str> {
        match self {
            Self::MissingStream { .. } => None,
            Self::UnsupportedEvent { .. } => None,
            Self::SubscriptionFailed { stream, .. } => Some(stream),
            Self::Connection(_) => None,
            Self::Core(_) => None,
        }
    }

    /// Returns the event type if this is an `UnsupportedEvent` error.
    ///
    /// # Returns
    ///
    /// - `Some(&str)` - The event type for `UnsupportedEvent` variant
    /// - `None` - For all other variants
    pub fn event_type(&self) -> Option<&str> {
        match self {
            Self::UnsupportedEvent { event } => Some(event),
            _ => None,
        }
    }

    /// Returns the raw message if this is a `MissingStream` error.
    ///
    /// # Returns
    ///
    /// - `Some(&str)` - The raw message for `MissingStream` variant
    /// - `None` - For all other variants
    pub fn raw_message(&self) -> Option<&str> {
        match self {
            Self::MissingStream { raw } => Some(raw),
            _ => None,
        }
    }
}

/// Conversion from `BinanceWsError` to `CoreError`.
///
/// This implementation allows Binance-specific errors to be returned from
/// public API methods that return `CoreError`. The conversion preserves
/// the original error for downcast capability.
///
/// # Conversion Rules
///
/// - `BinanceWsError::Core(core)` → Returns the inner `CoreError` directly
/// - All other variants → Boxed into `CoreError::WebSocket` for downcast
impl From<BinanceWsError> for CoreError {
    fn from(e: BinanceWsError) -> Self {
        match e {
            // Passthrough: extract the inner CoreError
            BinanceWsError::Core(core) => core,
            // Preserve the original error for downcast capability
            other => CoreError::WebSocket(Box::new(other)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_missing_stream_error() {
        let err = BinanceWsError::missing_stream(r#"{"data": "test"}"#);
        assert!(matches!(err, BinanceWsError::MissingStream { .. }));
        assert!(err.to_string().contains("Stream name missing"));
        assert_eq!(err.stream_name(), None);
        assert!(err.raw_message().is_some());
    }

    #[test]
    fn test_missing_stream_truncation() {
        let long_message = "x".repeat(300);
        let err = BinanceWsError::missing_stream(long_message);
        if let BinanceWsError::MissingStream { raw } = &err {
            assert!(raw.len() <= 203); // 200 + "..."
            assert!(raw.ends_with("..."));
        } else {
            panic!("Expected MissingStream variant");
        }
    }

    #[test]
    fn test_unsupported_event_error() {
        let err = BinanceWsError::unsupported_event("unknownEvent");
        assert!(matches!(err, BinanceWsError::UnsupportedEvent { .. }));
        assert!(err.to_string().contains("unknownEvent"));
        assert_eq!(err.event_type(), Some("unknownEvent"));
        assert_eq!(err.stream_name(), None);
    }

    #[test]
    fn test_subscription_failed_error() {
        let err = BinanceWsError::subscription_failed("btcusdt@ticker", "Invalid symbol");
        assert!(matches!(err, BinanceWsError::SubscriptionFailed { .. }));
        assert!(err.to_string().contains("btcusdt@ticker"));
        assert!(err.to_string().contains("Invalid symbol"));
        assert_eq!(err.stream_name(), Some("btcusdt@ticker"));
    }

    #[test]
    fn test_connection_error() {
        let err = BinanceWsError::connection("Connection refused");
        assert!(matches!(err, BinanceWsError::Connection(_)));
        assert!(err.to_string().contains("Connection refused"));
        assert_eq!(err.stream_name(), None);
    }

    #[test]
    fn test_core_passthrough() {
        let core_err = CoreError::authentication("Invalid API key");
        let ws_err = BinanceWsError::Core(core_err);
        assert!(matches!(ws_err, BinanceWsError::Core(_)));
    }

    #[test]
    fn test_conversion_to_core_error() {
        // Test non-Core variant conversion
        let ws_err = BinanceWsError::subscription_failed("btcusdt@ticker", "Invalid");
        let core_err: CoreError = ws_err.into();
        assert!(matches!(core_err, CoreError::WebSocket(_)));

        // Verify downcast works
        let downcast = core_err.downcast_websocket::<BinanceWsError>();
        assert!(downcast.is_some());
        if let Some(binance_err) = downcast {
            assert_eq!(binance_err.stream_name(), Some("btcusdt@ticker"));
        }
    }

    #[test]
    fn test_core_passthrough_conversion() {
        // Test Core variant passthrough
        let original = CoreError::authentication("Invalid API key");
        let ws_err = BinanceWsError::Core(original);
        let core_err: CoreError = ws_err.into();

        // Should be Authentication, not WebSocket
        assert!(matches!(core_err, CoreError::Authentication(_)));
        assert!(core_err.to_string().contains("Invalid API key"));
    }

    #[test]
    fn test_error_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<BinanceWsError>();
    }

    #[test]
    fn test_error_is_std_error() {
        fn assert_std_error<T: StdError>() {}
        assert_std_error::<BinanceWsError>();
    }
}
