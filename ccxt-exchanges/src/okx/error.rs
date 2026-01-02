//! OKX-specific error handling.
//!
//! This module provides error parsing for OKX API responses,
//! mapping OKX error codes to ccxt-core Error types.

use ccxt_core::error::Error;
use serde_json::Value;
use std::time::Duration;

/// OKX error codes and their meanings.
///
/// Reference: https://www.okx.com/docs-v5/en/#error-code
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OkxErrorCode {
    /// Invalid request (50000)
    InvalidRequest,
    /// Invalid API key (50001)
    InvalidApiKey,
    /// Invalid signature (50002)
    InvalidSignature,
    /// Invalid timestamp (50003)
    InvalidTimestamp,
    /// Rate limit exceeded (50004)
    RateLimitExceeded,
    /// Invalid passphrase (50005)
    InvalidPassphrase,
    /// Bad symbol / Invalid instrument (50011)
    BadSymbol,
    /// General parameter error (51000)
    ParameterError,
    /// Insufficient funds (51001)
    InsufficientFunds,
    /// Order does not exist (51400)
    OrderNotFound,
    /// Insufficient balance (51008)
    InsufficientBalance,
    /// Unknown error code
    Unknown(i64),
}

impl OkxErrorCode {
    /// Parses an OKX error code string into an OkxErrorCode.
    pub fn from_code(code: &str) -> Self {
        match code.parse::<i64>() {
            Ok(50000) => OkxErrorCode::InvalidRequest,
            Ok(50001) => OkxErrorCode::InvalidApiKey,
            Ok(50002) => OkxErrorCode::InvalidSignature,
            Ok(50003) => OkxErrorCode::InvalidTimestamp,
            Ok(50004) => OkxErrorCode::RateLimitExceeded,
            Ok(50005) => OkxErrorCode::InvalidPassphrase,
            Ok(50011) => OkxErrorCode::BadSymbol,
            Ok(51000) => OkxErrorCode::ParameterError,
            Ok(51001) => OkxErrorCode::InsufficientFunds,
            Ok(51008) => OkxErrorCode::InsufficientBalance,
            Ok(51400) => OkxErrorCode::OrderNotFound,
            Ok(n) => OkxErrorCode::Unknown(n),
            Err(_) => OkxErrorCode::Unknown(0),
        }
    }

    /// Returns the numeric code for this error.
    pub fn code(&self) -> i64 {
        match self {
            OkxErrorCode::InvalidRequest => 50000,
            OkxErrorCode::InvalidApiKey => 50001,
            OkxErrorCode::InvalidSignature => 50002,
            OkxErrorCode::InvalidTimestamp => 50003,
            OkxErrorCode::RateLimitExceeded => 50004,
            OkxErrorCode::InvalidPassphrase => 50005,
            OkxErrorCode::BadSymbol => 50011,
            OkxErrorCode::ParameterError => 51000,
            OkxErrorCode::InsufficientFunds => 51001,
            OkxErrorCode::InsufficientBalance => 51008,
            OkxErrorCode::OrderNotFound => 51400,
            OkxErrorCode::Unknown(n) => *n,
        }
    }
}

/// Parses an OKX API error response and converts it to a ccxt-core Error.
///
/// OKX API responses follow this format:
/// ```json
/// {
///     "code": "50001",
///     "msg": "Invalid API key",
///     "data": []
/// }
/// ```
///
/// # Arguments
///
/// * `response` - The JSON response from OKX API
///
/// # Returns
///
/// A ccxt-core Error mapped from the OKX error code.
///
/// # Example
///
/// ```rust
/// use ccxt_exchanges::okx::error::parse_error;
/// use serde_json::json;
///
/// let response = json!({
///     "code": "50001",
///     "msg": "Invalid API key"
/// });
///
/// let error = parse_error(&response);
/// assert!(error.as_authentication().is_some());
/// ```
pub fn parse_error(response: &Value) -> Error {
    let code = response
        .get("code")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("unknown");

    let msg = response
        .get("msg")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("Unknown error");

    let error_code = OkxErrorCode::from_code(code);

    match error_code {
        OkxErrorCode::InvalidApiKey
        | OkxErrorCode::InvalidSignature
        | OkxErrorCode::InvalidTimestamp
        | OkxErrorCode::InvalidPassphrase => Error::authentication(msg.to_string()),
        OkxErrorCode::RateLimitExceeded => {
            // OKX typically suggests waiting 1 second on rate limit
            Error::rate_limit(msg.to_string(), Some(Duration::from_secs(1)))
        }
        OkxErrorCode::InvalidRequest | OkxErrorCode::ParameterError => {
            Error::invalid_request(msg.to_string())
        }
        OkxErrorCode::InsufficientFunds | OkxErrorCode::InsufficientBalance => {
            Error::insufficient_balance(msg.to_string())
        }
        OkxErrorCode::BadSymbol => Error::bad_symbol(msg.to_string()),
        OkxErrorCode::OrderNotFound | OkxErrorCode::Unknown(_) => Error::exchange(code, msg),
    }
}

/// Checks if an OKX API response indicates an error.
///
/// OKX uses "0" as the success code. Any other code indicates an error.
///
/// # Arguments
///
/// * `response` - The JSON response from OKX API
///
/// # Returns
///
/// `true` if the response indicates an error, `false` otherwise.
pub fn is_error_response(response: &Value) -> bool {
    response.get("code").and_then(serde_json::Value::as_str) != Some("0")
}

/// Extracts the error code from an OKX API response.
///
/// # Arguments
///
/// * `response` - The JSON response from OKX API
///
/// # Returns
///
/// The error code as a string, or "unknown" if not found.
pub fn extract_error_code(response: &Value) -> &str {
    response
        .get("code")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("unknown")
}

/// Extracts the error message from an OKX API response.
///
/// # Arguments
///
/// * `response` - The JSON response from OKX API
///
/// # Returns
///
/// The error message as a string, or "Unknown error" if not found.
pub fn extract_error_message(response: &Value) -> &str {
    response
        .get("msg")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("Unknown error")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_parse_authentication_error_invalid_api_key() {
        let response = json!({
            "code": "50001",
            "msg": "Invalid API key"
        });

        let error = parse_error(&response);
        assert!(error.as_authentication().is_some());
        assert!(error.to_string().contains("Invalid API key"));
    }

    #[test]
    fn test_parse_authentication_error_invalid_signature() {
        let response = json!({
            "code": "50002",
            "msg": "Invalid signature"
        });

        let error = parse_error(&response);
        assert!(error.as_authentication().is_some());
        assert!(error.to_string().contains("Invalid signature"));
    }

    #[test]
    fn test_parse_authentication_error_invalid_passphrase() {
        let response = json!({
            "code": "50005",
            "msg": "Invalid passphrase"
        });

        let error = parse_error(&response);
        assert!(error.as_authentication().is_some());
        assert!(error.to_string().contains("Invalid passphrase"));
    }

    #[test]
    fn test_parse_rate_limit_error() {
        let response = json!({
            "code": "50004",
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
            "code": "50000",
            "msg": "Invalid request parameters"
        });

        let error = parse_error(&response);
        let display = error.to_string();
        assert!(display.contains("Invalid request"));
    }

    #[test]
    fn test_parse_parameter_error() {
        let response = json!({
            "code": "51000",
            "msg": "Parameter error"
        });

        let error = parse_error(&response);
        let display = error.to_string();
        assert!(display.contains("Parameter error"));
    }

    #[test]
    fn test_parse_insufficient_funds_error() {
        let response = json!({
            "code": "51001",
            "msg": "Insufficient balance"
        });

        let error = parse_error(&response);
        let display = error.to_string();
        assert!(display.contains("Insufficient balance"));
    }

    #[test]
    fn test_parse_insufficient_balance_error() {
        let response = json!({
            "code": "51008",
            "msg": "Insufficient balance for order"
        });

        let error = parse_error(&response);
        let display = error.to_string();
        assert!(display.contains("Insufficient balance"));
    }

    #[test]
    fn test_parse_bad_symbol_error() {
        let response = json!({
            "code": "50011",
            "msg": "Invalid instrument"
        });

        let error = parse_error(&response);
        let display = error.to_string();
        assert!(display.contains("Bad symbol"));
    }

    #[test]
    fn test_parse_order_not_found_error() {
        let response = json!({
            "code": "51400",
            "msg": "Order does not exist"
        });

        let error = parse_error(&response);
        let display = error.to_string();
        assert!(display.contains("Order does not exist"));
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
        let display = error.to_string();
        assert!(display.contains("Error without code"));
    }

    #[test]
    fn test_parse_error_missing_message() {
        let response = json!({
            "code": "50001"
        });

        let error = parse_error(&response);
        let display = error.to_string();
        assert!(display.contains("Unknown error"));
    }

    #[test]
    fn test_is_error_response_success() {
        let response = json!({
            "code": "0",
            "msg": "success",
            "data": []
        });

        assert!(!is_error_response(&response));
    }

    #[test]
    fn test_is_error_response_error() {
        let response = json!({
            "code": "50001",
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
            "code": "50001",
            "msg": "Invalid API key"
        });

        assert_eq!(extract_error_code(&response), "50001");
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
            "code": "50001",
            "msg": "Invalid API key"
        });

        assert_eq!(extract_error_message(&response), "Invalid API key");
    }

    #[test]
    fn test_extract_error_message_missing() {
        let response = json!({
            "code": "50001"
        });

        assert_eq!(extract_error_message(&response), "Unknown error");
    }

    #[test]
    fn test_okx_error_code_from_code() {
        assert_eq!(
            OkxErrorCode::from_code("50000"),
            OkxErrorCode::InvalidRequest
        );
        assert_eq!(
            OkxErrorCode::from_code("50001"),
            OkxErrorCode::InvalidApiKey
        );
        assert_eq!(
            OkxErrorCode::from_code("50002"),
            OkxErrorCode::InvalidSignature
        );
        assert_eq!(
            OkxErrorCode::from_code("50003"),
            OkxErrorCode::InvalidTimestamp
        );
        assert_eq!(
            OkxErrorCode::from_code("50004"),
            OkxErrorCode::RateLimitExceeded
        );
        assert_eq!(
            OkxErrorCode::from_code("50005"),
            OkxErrorCode::InvalidPassphrase
        );
        assert_eq!(OkxErrorCode::from_code("50011"), OkxErrorCode::BadSymbol);
        assert_eq!(
            OkxErrorCode::from_code("51000"),
            OkxErrorCode::ParameterError
        );
        assert_eq!(
            OkxErrorCode::from_code("51001"),
            OkxErrorCode::InsufficientFunds
        );
        assert_eq!(
            OkxErrorCode::from_code("51008"),
            OkxErrorCode::InsufficientBalance
        );
        assert_eq!(
            OkxErrorCode::from_code("51400"),
            OkxErrorCode::OrderNotFound
        );
        assert_eq!(
            OkxErrorCode::from_code("99999"),
            OkxErrorCode::Unknown(99999)
        );
        assert_eq!(OkxErrorCode::from_code("invalid"), OkxErrorCode::Unknown(0));
    }

    #[test]
    fn test_okx_error_code_code() {
        assert_eq!(OkxErrorCode::InvalidRequest.code(), 50000);
        assert_eq!(OkxErrorCode::InvalidApiKey.code(), 50001);
        assert_eq!(OkxErrorCode::InvalidSignature.code(), 50002);
        assert_eq!(OkxErrorCode::InvalidTimestamp.code(), 50003);
        assert_eq!(OkxErrorCode::RateLimitExceeded.code(), 50004);
        assert_eq!(OkxErrorCode::InvalidPassphrase.code(), 50005);
        assert_eq!(OkxErrorCode::BadSymbol.code(), 50011);
        assert_eq!(OkxErrorCode::ParameterError.code(), 51000);
        assert_eq!(OkxErrorCode::InsufficientFunds.code(), 51001);
        assert_eq!(OkxErrorCode::InsufficientBalance.code(), 51008);
        assert_eq!(OkxErrorCode::OrderNotFound.code(), 51400);
        assert_eq!(OkxErrorCode::Unknown(12345).code(), 12345);
    }
}
