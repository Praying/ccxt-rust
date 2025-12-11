//! Bybit-specific error handling.
//!
//! This module provides error parsing for Bybit API responses,
//! mapping Bybit error codes to ccxt-core Error types.

use ccxt_core::error::Error;
use serde_json::Value;
use std::time::Duration;

/// Bybit error codes and their meanings.
///
/// Reference: https://bybit-exchange.github.io/docs/v5/error
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BybitErrorCode {
    /// Invalid request (10001)
    InvalidRequest,
    /// Invalid API key (10003)
    InvalidApiKey,
    /// Invalid signature (10004)
    InvalidSignature,
    /// Permission denied (10005)
    PermissionDenied,
    /// Rate limit exceeded (10006)
    RateLimitExceeded,
    /// IP not allowed (10010)
    IpNotAllowed,
    /// Bad symbol / Invalid symbol (10016)
    BadSymbol,
    /// Insufficient funds (110001)
    InsufficientFunds,
    /// Insufficient available balance (110007)
    InsufficientAvailableBalance,
    /// Order not found (110008)
    OrderNotFound,
    /// Unknown error code
    Unknown(i64),
}

impl BybitErrorCode {
    /// Parses a Bybit error code into a BybitErrorCode.
    pub fn from_code(code: i64) -> Self {
        match code {
            10001 => BybitErrorCode::InvalidRequest,
            10003 => BybitErrorCode::InvalidApiKey,
            10004 => BybitErrorCode::InvalidSignature,
            10005 => BybitErrorCode::PermissionDenied,
            10006 => BybitErrorCode::RateLimitExceeded,
            10010 => BybitErrorCode::IpNotAllowed,
            10016 => BybitErrorCode::BadSymbol,
            110001 => BybitErrorCode::InsufficientFunds,
            110007 => BybitErrorCode::InsufficientAvailableBalance,
            110008 => BybitErrorCode::OrderNotFound,
            n => BybitErrorCode::Unknown(n),
        }
    }

    /// Returns the numeric code for this error.
    pub fn code(&self) -> i64 {
        match self {
            BybitErrorCode::InvalidRequest => 10001,
            BybitErrorCode::InvalidApiKey => 10003,
            BybitErrorCode::InvalidSignature => 10004,
            BybitErrorCode::PermissionDenied => 10005,
            BybitErrorCode::RateLimitExceeded => 10006,
            BybitErrorCode::IpNotAllowed => 10010,
            BybitErrorCode::BadSymbol => 10016,
            BybitErrorCode::InsufficientFunds => 110001,
            BybitErrorCode::InsufficientAvailableBalance => 110007,
            BybitErrorCode::OrderNotFound => 110008,
            BybitErrorCode::Unknown(n) => *n,
        }
    }
}

/// Parses a Bybit API error response and converts it to a ccxt-core Error.
///
/// Bybit API responses follow this format:
/// ```json
/// {
///     "retCode": 10003,
///     "retMsg": "Invalid API key",
///     "result": {},
///     "time": 1234567890123
/// }
/// ```
///
/// # Arguments
///
/// * `response` - The JSON response from Bybit API
///
/// # Returns
///
/// A ccxt-core Error mapped from the Bybit error code.
///
/// # Example
///
/// ```rust
/// use ccxt_exchanges::bybit::error::parse_error;
/// use serde_json::json;
///
/// let response = json!({
///     "retCode": 10003,
///     "retMsg": "Invalid API key"
/// });
///
/// let error = parse_error(&response);
/// assert!(error.as_authentication().is_some());
/// ```
pub fn parse_error(response: &Value) -> Error {
    let code = response
        .get("retCode")
        .and_then(|v| v.as_i64())
        .unwrap_or(0);

    let msg = response
        .get("retMsg")
        .and_then(|v| v.as_str())
        .unwrap_or("Unknown error");

    let error_code = BybitErrorCode::from_code(code);

    match error_code {
        BybitErrorCode::InvalidApiKey
        | BybitErrorCode::InvalidSignature
        | BybitErrorCode::PermissionDenied
        | BybitErrorCode::IpNotAllowed => Error::authentication(msg.to_string()),
        BybitErrorCode::RateLimitExceeded => {
            // Bybit typically suggests waiting 1 second on rate limit
            Error::rate_limit(msg.to_string(), Some(Duration::from_secs(1)))
        }
        BybitErrorCode::InvalidRequest => Error::invalid_request(msg.to_string()),
        BybitErrorCode::InsufficientFunds | BybitErrorCode::InsufficientAvailableBalance => {
            Error::insufficient_balance(msg.to_string())
        }
        BybitErrorCode::BadSymbol => Error::bad_symbol(msg.to_string()),
        BybitErrorCode::OrderNotFound => Error::exchange(code.to_string(), msg),
        BybitErrorCode::Unknown(_) => Error::exchange(code.to_string(), msg),
    }
}

/// Checks if a Bybit API response indicates an error.
///
/// Bybit uses 0 as the success code. Any other code indicates an error.
///
/// # Arguments
///
/// * `response` - The JSON response from Bybit API
///
/// # Returns
///
/// `true` if the response indicates an error, `false` otherwise.
pub fn is_error_response(response: &Value) -> bool {
    response
        .get("retCode")
        .and_then(|v| v.as_i64())
        .map(|code| code != 0)
        .unwrap_or(true)
}

/// Extracts the error code from a Bybit API response.
///
/// # Arguments
///
/// * `response` - The JSON response from Bybit API
///
/// # Returns
///
/// The error code as an i64, or 0 if not found.
pub fn extract_error_code(response: &Value) -> i64 {
    response
        .get("retCode")
        .and_then(|v| v.as_i64())
        .unwrap_or(0)
}

/// Extracts the error message from a Bybit API response.
///
/// # Arguments
///
/// * `response` - The JSON response from Bybit API
///
/// # Returns
///
/// The error message as a string, or "Unknown error" if not found.
pub fn extract_error_message(response: &Value) -> &str {
    response
        .get("retMsg")
        .and_then(|v| v.as_str())
        .unwrap_or("Unknown error")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_parse_authentication_error_invalid_api_key() {
        let response = json!({
            "retCode": 10003,
            "retMsg": "Invalid API key"
        });

        let error = parse_error(&response);
        assert!(error.as_authentication().is_some());
        assert!(error.to_string().contains("Invalid API key"));
    }

    #[test]
    fn test_parse_authentication_error_invalid_signature() {
        let response = json!({
            "retCode": 10004,
            "retMsg": "Invalid signature"
        });

        let error = parse_error(&response);
        assert!(error.as_authentication().is_some());
        assert!(error.to_string().contains("Invalid signature"));
    }

    #[test]
    fn test_parse_authentication_error_permission_denied() {
        let response = json!({
            "retCode": 10005,
            "retMsg": "Permission denied"
        });

        let error = parse_error(&response);
        assert!(error.as_authentication().is_some());
        assert!(error.to_string().contains("Permission denied"));
    }

    #[test]
    fn test_parse_authentication_error_ip_not_allowed() {
        let response = json!({
            "retCode": 10010,
            "retMsg": "IP not allowed"
        });

        let error = parse_error(&response);
        assert!(error.as_authentication().is_some());
        assert!(error.to_string().contains("IP not allowed"));
    }

    #[test]
    fn test_parse_rate_limit_error() {
        let response = json!({
            "retCode": 10006,
            "retMsg": "Rate limit exceeded"
        });

        let error = parse_error(&response);
        assert!(error.as_rate_limit().is_some());
        let (msg, retry_after) = error.as_rate_limit().unwrap();
        assert!(msg.contains("Rate limit"));
        assert!(retry_after.is_some());
    }

    #[test]
    fn test_parse_invalid_request_error() {
        let response = json!({
            "retCode": 10001,
            "retMsg": "Invalid request parameters"
        });

        let error = parse_error(&response);
        let display = error.to_string();
        assert!(display.contains("Invalid request"));
    }

    #[test]
    fn test_parse_insufficient_funds_error() {
        let response = json!({
            "retCode": 110001,
            "retMsg": "Insufficient balance"
        });

        let error = parse_error(&response);
        let display = error.to_string();
        assert!(display.contains("Insufficient balance"));
    }

    #[test]
    fn test_parse_insufficient_available_balance_error() {
        let response = json!({
            "retCode": 110007,
            "retMsg": "Insufficient available balance"
        });

        let error = parse_error(&response);
        let display = error.to_string();
        assert!(display.contains("Insufficient"));
    }

    #[test]
    fn test_parse_bad_symbol_error() {
        let response = json!({
            "retCode": 10016,
            "retMsg": "Invalid symbol"
        });

        let error = parse_error(&response);
        let display = error.to_string();
        assert!(display.contains("Bad symbol"));
    }

    #[test]
    fn test_parse_order_not_found_error() {
        let response = json!({
            "retCode": 110008,
            "retMsg": "Order not found"
        });

        let error = parse_error(&response);
        let display = error.to_string();
        assert!(display.contains("Order not found"));
    }

    #[test]
    fn test_parse_unknown_error() {
        let response = json!({
            "retCode": 99999,
            "retMsg": "Some unknown error"
        });

        let error = parse_error(&response);
        let display = error.to_string();
        assert!(display.contains("Some unknown error"));
    }

    #[test]
    fn test_parse_error_missing_code() {
        let response = json!({
            "retMsg": "Error without code"
        });

        let error = parse_error(&response);
        let display = error.to_string();
        assert!(display.contains("Error without code"));
    }

    #[test]
    fn test_parse_error_missing_message() {
        let response = json!({
            "retCode": 10003
        });

        let error = parse_error(&response);
        let display = error.to_string();
        assert!(display.contains("Unknown error"));
    }

    #[test]
    fn test_is_error_response_success() {
        let response = json!({
            "retCode": 0,
            "retMsg": "OK",
            "result": {}
        });

        assert!(!is_error_response(&response));
    }

    #[test]
    fn test_is_error_response_error() {
        let response = json!({
            "retCode": 10003,
            "retMsg": "Invalid API key"
        });

        assert!(is_error_response(&response));
    }

    #[test]
    fn test_is_error_response_missing_code() {
        let response = json!({
            "retMsg": "No code field"
        });

        // Missing code should be treated as error
        assert!(is_error_response(&response));
    }

    #[test]
    fn test_extract_error_code() {
        let response = json!({
            "retCode": 10003,
            "retMsg": "Invalid API key"
        });

        assert_eq!(extract_error_code(&response), 10003);
    }

    #[test]
    fn test_extract_error_code_missing() {
        let response = json!({
            "retMsg": "No code"
        });

        assert_eq!(extract_error_code(&response), 0);
    }

    #[test]
    fn test_extract_error_message() {
        let response = json!({
            "retCode": 10003,
            "retMsg": "Invalid API key"
        });

        assert_eq!(extract_error_message(&response), "Invalid API key");
    }

    #[test]
    fn test_extract_error_message_missing() {
        let response = json!({
            "retCode": 10003
        });

        assert_eq!(extract_error_message(&response), "Unknown error");
    }

    #[test]
    fn test_bybit_error_code_from_code() {
        assert_eq!(
            BybitErrorCode::from_code(10001),
            BybitErrorCode::InvalidRequest
        );
        assert_eq!(
            BybitErrorCode::from_code(10003),
            BybitErrorCode::InvalidApiKey
        );
        assert_eq!(
            BybitErrorCode::from_code(10004),
            BybitErrorCode::InvalidSignature
        );
        assert_eq!(
            BybitErrorCode::from_code(10005),
            BybitErrorCode::PermissionDenied
        );
        assert_eq!(
            BybitErrorCode::from_code(10006),
            BybitErrorCode::RateLimitExceeded
        );
        assert_eq!(
            BybitErrorCode::from_code(10010),
            BybitErrorCode::IpNotAllowed
        );
        assert_eq!(BybitErrorCode::from_code(10016), BybitErrorCode::BadSymbol);
        assert_eq!(
            BybitErrorCode::from_code(110001),
            BybitErrorCode::InsufficientFunds
        );
        assert_eq!(
            BybitErrorCode::from_code(110007),
            BybitErrorCode::InsufficientAvailableBalance
        );
        assert_eq!(
            BybitErrorCode::from_code(110008),
            BybitErrorCode::OrderNotFound
        );
        assert_eq!(
            BybitErrorCode::from_code(99999),
            BybitErrorCode::Unknown(99999)
        );
    }

    #[test]
    fn test_bybit_error_code_code() {
        assert_eq!(BybitErrorCode::InvalidRequest.code(), 10001);
        assert_eq!(BybitErrorCode::InvalidApiKey.code(), 10003);
        assert_eq!(BybitErrorCode::InvalidSignature.code(), 10004);
        assert_eq!(BybitErrorCode::PermissionDenied.code(), 10005);
        assert_eq!(BybitErrorCode::RateLimitExceeded.code(), 10006);
        assert_eq!(BybitErrorCode::IpNotAllowed.code(), 10010);
        assert_eq!(BybitErrorCode::BadSymbol.code(), 10016);
        assert_eq!(BybitErrorCode::InsufficientFunds.code(), 110001);
        assert_eq!(BybitErrorCode::InsufficientAvailableBalance.code(), 110007);
        assert_eq!(BybitErrorCode::OrderNotFound.code(), 110008);
        assert_eq!(BybitErrorCode::Unknown(12345).code(), 12345);
    }
}
