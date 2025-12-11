//! Bitget-specific error handling.
//!
//! This module provides error parsing for Bitget API responses,
//! mapping Bitget error codes to ccxt-core Error types.

use ccxt_core::error::Error;
use serde_json::Value;
use std::time::Duration;

/// Bitget error codes and their meanings.
///
/// Reference: https://www.bitget.com/api-doc/common/error-code
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BitgetErrorCode {
    /// Invalid API key (40001)
    InvalidApiKey,
    /// Invalid signature (40002)
    InvalidSignature,
    /// Rate limit exceeded (40003)
    RateLimitExceeded,
    /// Invalid request parameters (40004)
    InvalidRequest,
    /// Insufficient funds (40005)
    InsufficientFunds,
    /// Bad symbol / Invalid trading pair (40006)
    BadSymbol,
    /// Order not found (40007)
    OrderNotFound,
    /// Unknown error code
    Unknown(i64),
}

impl BitgetErrorCode {
    /// Parses a Bitget error code string into a BitgetErrorCode.
    pub fn from_code(code: &str) -> Self {
        match code.parse::<i64>() {
            Ok(40001) => BitgetErrorCode::InvalidApiKey,
            Ok(40002) => BitgetErrorCode::InvalidSignature,
            Ok(40003) => BitgetErrorCode::RateLimitExceeded,
            Ok(40004) => BitgetErrorCode::InvalidRequest,
            Ok(40005) => BitgetErrorCode::InsufficientFunds,
            Ok(40006) => BitgetErrorCode::BadSymbol,
            Ok(40007) => BitgetErrorCode::OrderNotFound,
            Ok(n) => BitgetErrorCode::Unknown(n),
            Err(_) => BitgetErrorCode::Unknown(0),
        }
    }

    /// Returns the numeric code for this error.
    pub fn code(&self) -> i64 {
        match self {
            BitgetErrorCode::InvalidApiKey => 40001,
            BitgetErrorCode::InvalidSignature => 40002,
            BitgetErrorCode::RateLimitExceeded => 40003,
            BitgetErrorCode::InvalidRequest => 40004,
            BitgetErrorCode::InsufficientFunds => 40005,
            BitgetErrorCode::BadSymbol => 40006,
            BitgetErrorCode::OrderNotFound => 40007,
            BitgetErrorCode::Unknown(n) => *n,
        }
    }
}

/// Parses a Bitget API error response and converts it to a ccxt-core Error.
///
/// Bitget API responses follow this format:
/// ```json
/// {
///     "code": "40001",
///     "msg": "Invalid API key",
///     "data": null
/// }
/// ```
///
/// # Arguments
///
/// * `response` - The JSON response from Bitget API
///
/// # Returns
///
/// A ccxt-core Error mapped from the Bitget error code.
///
/// # Example
///
/// ```rust
/// use ccxt_exchanges::bitget::error::parse_error;
/// use serde_json::json;
///
/// let response = json!({
///     "code": "40001",
///     "msg": "Invalid API key"
/// });
///
/// let error = parse_error(&response);
/// assert!(error.as_authentication().is_some());
/// ```
pub fn parse_error(response: &Value) -> Error {
    let code = response
        .get("code")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");

    let msg = response
        .get("msg")
        .and_then(|v| v.as_str())
        .unwrap_or("Unknown error");

    let error_code = BitgetErrorCode::from_code(code);

    match error_code {
        BitgetErrorCode::InvalidApiKey | BitgetErrorCode::InvalidSignature => {
            Error::authentication(msg.to_string())
        }
        BitgetErrorCode::RateLimitExceeded => {
            // Bitget typically suggests waiting 1 second on rate limit
            Error::rate_limit(msg.to_string(), Some(Duration::from_secs(1)))
        }
        BitgetErrorCode::InvalidRequest => Error::invalid_request(msg.to_string()),
        BitgetErrorCode::InsufficientFunds => Error::insufficient_balance(msg.to_string()),
        BitgetErrorCode::BadSymbol => Error::bad_symbol(msg.to_string()),
        BitgetErrorCode::OrderNotFound => Error::exchange(code, msg),
        BitgetErrorCode::Unknown(_) => Error::exchange(code, msg),
    }
}

/// Checks if a Bitget API response indicates an error.
///
/// Bitget uses "00000" as the success code. Any other code indicates an error.
///
/// # Arguments
///
/// * `response` - The JSON response from Bitget API
///
/// # Returns
///
/// `true` if the response indicates an error, `false` otherwise.
pub fn is_error_response(response: &Value) -> bool {
    response
        .get("code")
        .and_then(|v| v.as_str())
        .map(|code| code != "00000")
        .unwrap_or(true)
}

/// Extracts the error code from a Bitget API response.
///
/// # Arguments
///
/// * `response` - The JSON response from Bitget API
///
/// # Returns
///
/// The error code as a string, or "unknown" if not found.
pub fn extract_error_code(response: &Value) -> &str {
    response
        .get("code")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
}

/// Extracts the error message from a Bitget API response.
///
/// # Arguments
///
/// * `response` - The JSON response from Bitget API
///
/// # Returns
///
/// The error message as a string, or "Unknown error" if not found.
pub fn extract_error_message(response: &Value) -> &str {
    response
        .get("msg")
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
            "code": "40001",
            "msg": "Invalid API key"
        });

        let error = parse_error(&response);
        assert!(error.as_authentication().is_some());
        assert!(error.to_string().contains("Invalid API key"));
    }

    #[test]
    fn test_parse_authentication_error_invalid_signature() {
        let response = json!({
            "code": "40002",
            "msg": "Invalid signature"
        });

        let error = parse_error(&response);
        assert!(error.as_authentication().is_some());
        assert!(error.to_string().contains("Invalid signature"));
    }

    #[test]
    fn test_parse_rate_limit_error() {
        let response = json!({
            "code": "40003",
            "msg": "Rate limit exceeded"
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
            "code": "40004",
            "msg": "Invalid request parameters"
        });

        let error = parse_error(&response);
        let display = error.to_string();
        assert!(display.contains("Invalid request"));
    }

    #[test]
    fn test_parse_insufficient_funds_error() {
        let response = json!({
            "code": "40005",
            "msg": "Insufficient balance"
        });

        let error = parse_error(&response);
        let display = error.to_string();
        assert!(display.contains("Insufficient balance"));
    }

    #[test]
    fn test_parse_bad_symbol_error() {
        let response = json!({
            "code": "40006",
            "msg": "Invalid trading pair"
        });

        let error = parse_error(&response);
        let display = error.to_string();
        assert!(display.contains("Bad symbol"));
    }

    #[test]
    fn test_parse_order_not_found_error() {
        let response = json!({
            "code": "40007",
            "msg": "Order not found"
        });

        let error = parse_error(&response);
        let display = error.to_string();
        assert!(display.contains("Order not found"));
    }

    #[test]
    fn test_parse_unknown_error() {
        let response = json!({
            "code": "99999",
            "msg": "Some unknown error"
        });

        let error = parse_error(&response);
        let display = error.to_string();
        assert!(display.contains("Some unknown error"));
    }

    #[test]
    fn test_parse_error_missing_code() {
        let response = json!({
            "msg": "Error without code"
        });

        let error = parse_error(&response);
        // Should default to exchange error with "unknown" code
        let display = error.to_string();
        assert!(display.contains("Error without code"));
    }

    #[test]
    fn test_parse_error_missing_message() {
        let response = json!({
            "code": "40001"
        });

        let error = parse_error(&response);
        // Should default to "Unknown error" message
        let display = error.to_string();
        assert!(display.contains("Unknown error"));
    }

    #[test]
    fn test_is_error_response_success() {
        let response = json!({
            "code": "00000",
            "msg": "success",
            "data": {}
        });

        assert!(!is_error_response(&response));
    }

    #[test]
    fn test_is_error_response_error() {
        let response = json!({
            "code": "40001",
            "msg": "Invalid API key"
        });

        assert!(is_error_response(&response));
    }

    #[test]
    fn test_is_error_response_missing_code() {
        let response = json!({
            "msg": "No code field"
        });

        // Missing code should be treated as error
        assert!(is_error_response(&response));
    }

    #[test]
    fn test_extract_error_code() {
        let response = json!({
            "code": "40001",
            "msg": "Invalid API key"
        });

        assert_eq!(extract_error_code(&response), "40001");
    }

    #[test]
    fn test_extract_error_code_missing() {
        let response = json!({
            "msg": "No code"
        });

        assert_eq!(extract_error_code(&response), "unknown");
    }

    #[test]
    fn test_extract_error_message() {
        let response = json!({
            "code": "40001",
            "msg": "Invalid API key"
        });

        assert_eq!(extract_error_message(&response), "Invalid API key");
    }

    #[test]
    fn test_extract_error_message_missing() {
        let response = json!({
            "code": "40001"
        });

        assert_eq!(extract_error_message(&response), "Unknown error");
    }

    #[test]
    fn test_bitget_error_code_from_code() {
        assert_eq!(
            BitgetErrorCode::from_code("40001"),
            BitgetErrorCode::InvalidApiKey
        );
        assert_eq!(
            BitgetErrorCode::from_code("40002"),
            BitgetErrorCode::InvalidSignature
        );
        assert_eq!(
            BitgetErrorCode::from_code("40003"),
            BitgetErrorCode::RateLimitExceeded
        );
        assert_eq!(
            BitgetErrorCode::from_code("40004"),
            BitgetErrorCode::InvalidRequest
        );
        assert_eq!(
            BitgetErrorCode::from_code("40005"),
            BitgetErrorCode::InsufficientFunds
        );
        assert_eq!(
            BitgetErrorCode::from_code("40006"),
            BitgetErrorCode::BadSymbol
        );
        assert_eq!(
            BitgetErrorCode::from_code("40007"),
            BitgetErrorCode::OrderNotFound
        );
        assert_eq!(
            BitgetErrorCode::from_code("99999"),
            BitgetErrorCode::Unknown(99999)
        );
        assert_eq!(
            BitgetErrorCode::from_code("invalid"),
            BitgetErrorCode::Unknown(0)
        );
    }

    #[test]
    fn test_bitget_error_code_code() {
        assert_eq!(BitgetErrorCode::InvalidApiKey.code(), 40001);
        assert_eq!(BitgetErrorCode::InvalidSignature.code(), 40002);
        assert_eq!(BitgetErrorCode::RateLimitExceeded.code(), 40003);
        assert_eq!(BitgetErrorCode::InvalidRequest.code(), 40004);
        assert_eq!(BitgetErrorCode::InsufficientFunds.code(), 40005);
        assert_eq!(BitgetErrorCode::BadSymbol.code(), 40006);
        assert_eq!(BitgetErrorCode::OrderNotFound.code(), 40007);
        assert_eq!(BitgetErrorCode::Unknown(12345).code(), 12345);
    }
}
