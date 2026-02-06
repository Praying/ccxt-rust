//! Binance-specific error types.
//!
//! This module provides module-specific error types for Binance operations.
//! These errors can be converted to the core `Error` type while preserving
//! structured information for downcast capability.
//!
//! # Error Types
//!
//! - [`BinanceApiError`]: REST API errors with Binance error codes
//! - [`BinanceWsError`]: WebSocket-specific errors
//!
//! # Example
//!
//! ```rust
//! use ccxt_exchanges::binance::error::{BinanceApiError, BinanceWsError};
//! use ccxt_core::error::Error as CoreError;
//!
//! // Create a Binance API error
//! let api_err = BinanceApiError::new(-1121, "Invalid symbol.");
//! let core_err: CoreError = api_err.into();
//!
//! // Create a Binance WebSocket error
//! let ws_err = BinanceWsError::subscription_failed("btcusdt@ticker", "Invalid stream");
//! let core_err: CoreError = ws_err.into();
//! ```

use ccxt_core::error::Error as CoreError;
use thiserror::Error;

#[cfg(test)]
use std::error::Error as StdError;

/// Binance REST API error.
///
/// Represents errors returned by the Binance REST API in the format:
/// ```json
/// {"code": -1121, "msg": "Invalid symbol."}
/// ```
///
/// This error type preserves the original Binance error code and message,
/// and can be converted to the appropriate `CoreError` variant based on
/// the error code.
#[derive(Error, Debug, Clone)]
#[error("Binance API error {code}: {msg}")]
pub struct BinanceApiError {
    /// Binance error code (negative integer)
    pub code: i32,
    /// Error message from Binance
    pub msg: String,
}

impl BinanceApiError {
    /// Creates a new Binance API error.
    ///
    /// # Arguments
    ///
    /// * `code` - The Binance error code (typically negative)
    /// * `msg` - The error message from Binance
    ///
    /// # Example
    ///
    /// ```rust
    /// use ccxt_exchanges::binance::error::BinanceApiError;
    ///
    /// let err = BinanceApiError::new(-1121, "Invalid symbol.");
    /// assert_eq!(err.code, -1121);
    /// ```
    pub fn new(code: i32, msg: impl Into<String>) -> Self {
        Self {
            code,
            msg: msg.into(),
        }
    }

    /// Parses a Binance API error from JSON response.
    ///
    /// # Arguments
    ///
    /// * `json` - The JSON response from Binance API
    ///
    /// # Returns
    ///
    /// Returns `Some(BinanceApiError)` if the JSON contains `code` and `msg` fields,
    /// `None` otherwise.
    ///
    /// # Example
    ///
    /// ```rust
    /// use ccxt_exchanges::binance::error::BinanceApiError;
    /// use serde_json::json;
    ///
    /// let response = json!({"code": -1121, "msg": "Invalid symbol."});
    /// let err = BinanceApiError::from_json(&response);
    /// assert!(err.is_some());
    /// assert_eq!(err.unwrap().code, -1121);
    /// ```
    pub fn from_json(json: &serde_json::Value) -> Option<Self> {
        let code = json.get("code")?.as_i64()? as i32;
        let msg = json.get("msg")?.as_str()?.to_string();
        Some(Self { code, msg })
    }

    /// Returns true if this is a rate limit error.
    ///
    /// Rate limit errors have codes:
    /// - -1003: Too many requests
    /// - -1015: Too many orders
    pub fn is_rate_limit(&self) -> bool {
        matches!(self.code, -1003 | -1015)
    }

    /// Returns true if this is an IP ban error.
    ///
    /// IP ban errors have code -1003 with specific messages or HTTP 418 status.
    pub fn is_ip_banned(&self) -> bool {
        self.code == -1003 && self.msg.contains("banned")
    }

    /// Returns true if this is an authentication error.
    pub fn is_auth_error(&self) -> bool {
        matches!(self.code, -2014 | -2015 | -1022)
    }

    /// Returns true if this is an invalid symbol error.
    pub fn is_invalid_symbol(&self) -> bool {
        self.code == -1121
    }

    /// Returns true if this is an insufficient balance error.
    pub fn is_insufficient_balance(&self) -> bool {
        matches!(self.code, -2010 | -2011)
    }

    /// Returns true if this is an order not found error.
    pub fn is_order_not_found(&self) -> bool {
        self.code == -2013
    }

    /// Converts this error to the appropriate `CoreError` variant.
    ///
    /// The mapping follows these rules:
    /// - Authentication errors (-2014, -2015, -1022) → `CoreError::Authentication`
    /// - Rate limit errors (-1003, -1015) → `CoreError::RateLimit`
    /// - Invalid symbol (-1121) → `CoreError::BadSymbol`
    /// - Insufficient balance (-2010, -2011) → `CoreError::InsufficientBalance`
    /// - Order not found (-2013) → `CoreError::OrderNotFound`
    /// - Invalid order errors → `CoreError::InvalidOrder`
    /// - Network errors (-1001) → `CoreError::Network`
    /// - Other errors → `CoreError::Exchange`
    pub fn to_core_error(&self) -> CoreError {
        use std::borrow::Cow;

        match self.code {
            // Authentication errors
            -2014 | -2015 | -1022 => CoreError::authentication(self.msg.clone()),
            // Rate limit errors
            -1003 | -1015 => CoreError::rate_limit(self.msg.clone(), None),
            // Invalid symbol
            -1121 => CoreError::bad_symbol(&self.msg),
            // Insufficient balance
            -2010 | -2011 => CoreError::insufficient_balance(self.msg.clone()),
            // Order not found
            -2013 => CoreError::OrderNotFound(Cow::Owned(self.msg.clone())),
            // Invalid order (various)
            -1102 | -1106 | -1111 | -1112 | -1114 | -1115 | -1116 | -1117 | -1118 => {
                CoreError::InvalidOrder(Cow::Owned(self.msg.clone()))
            }
            // Network/timeout errors
            -1001 => CoreError::network(&self.msg),
            // All other errors
            _ => CoreError::exchange(self.code.to_string(), &self.msg),
        }
    }
}

impl From<BinanceApiError> for CoreError {
    fn from(e: BinanceApiError) -> Self {
        e.to_core_error()
    }
}

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
            Self::MissingStream { .. }
            | Self::UnsupportedEvent { .. }
            | Self::Connection(_)
            | Self::Core(_) => None,
            Self::SubscriptionFailed { stream, .. } => Some(stream),
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
        assert_send_sync::<BinanceApiError>();
    }

    #[test]
    fn test_error_is_std_error() {
        fn assert_std_error<T: StdError>() {}
        assert_std_error::<BinanceWsError>();
        assert_std_error::<BinanceApiError>();
    }

    // ==================== BinanceApiError Tests ====================

    #[test]
    fn test_binance_api_error_new() {
        let err = BinanceApiError::new(-1121, "Invalid symbol.");
        assert_eq!(err.code, -1121);
        assert_eq!(err.msg, "Invalid symbol.");
        assert!(err.to_string().contains("-1121"));
        assert!(err.to_string().contains("Invalid symbol."));
    }

    #[test]
    fn test_binance_api_error_from_json() {
        let json = serde_json::json!({"code": -1121, "msg": "Invalid symbol."});
        let err = BinanceApiError::from_json(&json);
        assert!(err.is_some());
        let err = err.unwrap();
        assert_eq!(err.code, -1121);
        assert_eq!(err.msg, "Invalid symbol.");
    }

    #[test]
    fn test_binance_api_error_from_json_missing_fields() {
        // Missing code
        let json = serde_json::json!({"msg": "Invalid symbol."});
        assert!(BinanceApiError::from_json(&json).is_none());

        // Missing msg
        let json = serde_json::json!({"code": -1121});
        assert!(BinanceApiError::from_json(&json).is_none());

        // Empty object
        let json = serde_json::json!({});
        assert!(BinanceApiError::from_json(&json).is_none());
    }

    #[test]
    fn test_binance_api_error_is_rate_limit() {
        let err = BinanceApiError::new(-1003, "Too many requests");
        assert!(err.is_rate_limit());

        let err = BinanceApiError::new(-1015, "Too many orders");
        assert!(err.is_rate_limit());

        let err = BinanceApiError::new(-1121, "Invalid symbol");
        assert!(!err.is_rate_limit());
    }

    #[test]
    fn test_binance_api_error_is_auth_error() {
        let err = BinanceApiError::new(-2014, "API-key format invalid");
        assert!(err.is_auth_error());

        let err = BinanceApiError::new(-2015, "Invalid API-key, IP, or permissions");
        assert!(err.is_auth_error());

        let err = BinanceApiError::new(-1022, "Signature for this request is not valid");
        assert!(err.is_auth_error());

        let err = BinanceApiError::new(-1121, "Invalid symbol");
        assert!(!err.is_auth_error());
    }

    #[test]
    fn test_binance_api_error_is_invalid_symbol() {
        let err = BinanceApiError::new(-1121, "Invalid symbol.");
        assert!(err.is_invalid_symbol());

        let err = BinanceApiError::new(-1003, "Too many requests");
        assert!(!err.is_invalid_symbol());
    }

    #[test]
    fn test_binance_api_error_is_insufficient_balance() {
        let err = BinanceApiError::new(-2010, "Account has insufficient balance");
        assert!(err.is_insufficient_balance());

        let err = BinanceApiError::new(-2011, "Unknown order sent");
        assert!(err.is_insufficient_balance());

        let err = BinanceApiError::new(-1121, "Invalid symbol");
        assert!(!err.is_insufficient_balance());
    }

    #[test]
    fn test_binance_api_error_is_order_not_found() {
        let err = BinanceApiError::new(-2013, "Order does not exist");
        assert!(err.is_order_not_found());

        let err = BinanceApiError::new(-1121, "Invalid symbol");
        assert!(!err.is_order_not_found());
    }

    #[test]
    fn test_binance_api_error_to_core_error_auth() {
        let err = BinanceApiError::new(-2015, "Invalid API-key");
        let core_err: CoreError = err.into();
        assert!(matches!(core_err, CoreError::Authentication(_)));
    }

    #[test]
    fn test_binance_api_error_to_core_error_rate_limit() {
        let err = BinanceApiError::new(-1003, "Too many requests");
        let core_err: CoreError = err.into();
        assert!(matches!(core_err, CoreError::RateLimit { .. }));
    }

    #[test]
    fn test_binance_api_error_to_core_error_invalid_symbol() {
        let err = BinanceApiError::new(-1121, "Invalid symbol.");
        let core_err: CoreError = err.into();
        // bad_symbol returns InvalidRequest variant
        assert!(matches!(core_err, CoreError::InvalidRequest(_)));
    }

    #[test]
    fn test_binance_api_error_to_core_error_insufficient_funds() {
        let err = BinanceApiError::new(-2010, "Account has insufficient balance");
        let core_err: CoreError = err.into();
        assert!(matches!(core_err, CoreError::InsufficientBalance(_)));
    }

    #[test]
    fn test_binance_api_error_to_core_error_order_not_found() {
        let err = BinanceApiError::new(-2013, "Order does not exist");
        let core_err: CoreError = err.into();
        assert!(matches!(core_err, CoreError::OrderNotFound(_)));
    }

    #[test]
    fn test_binance_api_error_to_core_error_invalid_order() {
        let err = BinanceApiError::new(-1102, "Mandatory parameter was not sent");
        let core_err: CoreError = err.into();
        assert!(matches!(core_err, CoreError::InvalidOrder(_)));
    }

    #[test]
    fn test_binance_api_error_to_core_error_network() {
        let err = BinanceApiError::new(-1001, "Internal error; unable to process your request");
        let core_err: CoreError = err.into();
        assert!(matches!(core_err, CoreError::Network(_)));
    }

    #[test]
    fn test_binance_api_error_to_core_error_exchange() {
        let err = BinanceApiError::new(-9999, "Unknown error");
        let core_err: CoreError = err.into();
        assert!(matches!(core_err, CoreError::Exchange(_)));
    }
}
