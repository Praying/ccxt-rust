//! HyperLiquid error handling module.
//!
//! Provides error types and parsing for HyperLiquid API responses.

use ccxt_core::Error;
use serde_json::Value;

/// HyperLiquid-specific error codes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HyperLiquidErrorCode {
    /// Invalid signature
    InvalidSignature,
    /// Insufficient margin/balance
    InsufficientMargin,
    /// Order not found
    OrderNotFound,
    /// Invalid parameter
    InvalidParameter,
    /// Rate limited
    RateLimited,
    /// Server error
    ServerError,
    /// User not found
    UserNotFound,
    /// Invalid asset
    InvalidAsset,
    /// Position not found
    PositionNotFound,
    /// Order would cross
    OrderWouldCross,
    /// Reduce only violation
    ReduceOnlyViolation,
    /// Unknown error
    Unknown(String),
}

impl std::fmt::Display for HyperLiquidErrorCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidSignature => write!(f, "Invalid signature"),
            Self::InsufficientMargin => write!(f, "Insufficient margin"),
            Self::OrderNotFound => write!(f, "Order not found"),
            Self::InvalidParameter => write!(f, "Invalid parameter"),
            Self::RateLimited => write!(f, "Rate limited"),
            Self::ServerError => write!(f, "Server error"),
            Self::UserNotFound => write!(f, "User not found"),
            Self::InvalidAsset => write!(f, "Invalid asset"),
            Self::PositionNotFound => write!(f, "Position not found"),
            Self::OrderWouldCross => write!(f, "Order would cross"),
            Self::ReduceOnlyViolation => write!(f, "Reduce only violation"),
            Self::Unknown(msg) => write!(f, "{}", msg),
        }
    }
}

impl From<HyperLiquidErrorCode> for Error {
    fn from(code: HyperLiquidErrorCode) -> Self {
        match code {
            HyperLiquidErrorCode::InvalidSignature => Error::authentication("Invalid signature"),
            HyperLiquidErrorCode::InsufficientMargin => {
                Error::insufficient_balance("Insufficient margin")
            }
            HyperLiquidErrorCode::OrderNotFound => Error::invalid_request("Order not found"),
            HyperLiquidErrorCode::InvalidParameter => Error::invalid_request("Invalid parameter"),
            HyperLiquidErrorCode::RateLimited => Error::rate_limit("Rate limited", None),
            HyperLiquidErrorCode::ServerError => Error::exchange("-1", "Server error"),
            HyperLiquidErrorCode::UserNotFound => Error::authentication("User not found"),
            HyperLiquidErrorCode::InvalidAsset => Error::bad_symbol("Invalid asset"),
            HyperLiquidErrorCode::PositionNotFound => Error::invalid_request("Position not found"),
            HyperLiquidErrorCode::OrderWouldCross => Error::invalid_request("Order would cross"),
            HyperLiquidErrorCode::ReduceOnlyViolation => {
                Error::invalid_request("Reduce only violation")
            }
            HyperLiquidErrorCode::Unknown(msg) => Error::exchange("-1", &msg),
        }
    }
}

/// Checks if a response is an error response.
///
/// HyperLiquid returns errors in various formats:
/// - `{"error": "message"}`
/// - `{"status": "err", "response": "message"}`
///
/// # Arguments
///
/// * `response` - The JSON response to check.
///
/// # Returns
///
/// `true` if the response indicates an error.
pub fn is_error_response(response: &Value) -> bool {
    // Check for explicit error field
    if response.get("error").is_some() {
        return true;
    }

    // Check for status: err
    if let Some(status) = response.get("status") {
        if status.as_str() == Some("err") {
            return true;
        }
    }

    // Check for response containing error message
    if let Some(resp) = response.get("response") {
        if let Some(s) = resp.as_str() {
            if s.contains("error") || s.contains("Error") || s.contains("failed") {
                return true;
            }
        }
    }

    false
}

/// Parses an error response into a ccxt_core::Error.
///
/// # Arguments
///
/// * `response` - The JSON error response.
///
/// # Returns
///
/// A ccxt_core::Error with appropriate type and message.
pub fn parse_error(response: &Value) -> Error {
    // Try to extract error message
    let message = extract_error_message(response);

    // Map to error code
    let code = map_error_message(&message);

    code.into()
}

/// Extracts the error message from a response.
fn extract_error_message(response: &Value) -> String {
    // Try "error" field
    if let Some(error) = response.get("error") {
        if let Some(s) = error.as_str() {
            return s.to_string();
        }
    }

    // Try "response" field
    if let Some(resp) = response.get("response") {
        if let Some(s) = resp.as_str() {
            return s.to_string();
        }
    }

    // Try "message" field
    if let Some(msg) = response.get("message") {
        if let Some(s) = msg.as_str() {
            return s.to_string();
        }
    }

    "Unknown error".to_string()
}

/// Maps an error message to an error code.
fn map_error_message(message: &str) -> HyperLiquidErrorCode {
    let lower = message.to_lowercase();

    if lower.contains("signature") || lower.contains("auth") || lower.contains("unauthorized") {
        HyperLiquidErrorCode::InvalidSignature
    } else if lower.contains("insufficient")
        || lower.contains("margin")
        || lower.contains("balance")
    {
        HyperLiquidErrorCode::InsufficientMargin
    } else if lower.contains("order") && lower.contains("not found") {
        HyperLiquidErrorCode::OrderNotFound
    } else if lower.contains("rate") || lower.contains("limit") || lower.contains("throttle") {
        HyperLiquidErrorCode::RateLimited
    } else if lower.contains("user") && lower.contains("not found") {
        HyperLiquidErrorCode::UserNotFound
    } else if lower.contains("asset") && (lower.contains("invalid") || lower.contains("not found"))
    {
        HyperLiquidErrorCode::InvalidAsset
    } else if lower.contains("position") && lower.contains("not found") {
        HyperLiquidErrorCode::PositionNotFound
    } else if lower.contains("cross") || lower.contains("would cross") {
        HyperLiquidErrorCode::OrderWouldCross
    } else if lower.contains("reduce only") || lower.contains("reduce-only") {
        HyperLiquidErrorCode::ReduceOnlyViolation
    } else if lower.contains("invalid") || lower.contains("parameter") {
        HyperLiquidErrorCode::InvalidParameter
    } else if lower.contains("server") || lower.contains("internal") {
        HyperLiquidErrorCode::ServerError
    } else {
        HyperLiquidErrorCode::Unknown(message.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_is_error_response_with_error_field() {
        let response = json!({"error": "Invalid signature"});
        assert!(is_error_response(&response));
    }

    #[test]
    fn test_is_error_response_with_status_err() {
        let response = json!({"status": "err", "response": "Something went wrong"});
        assert!(is_error_response(&response));
    }

    #[test]
    fn test_is_error_response_success() {
        let response = json!({"status": "ok", "response": {"data": []}});
        assert!(!is_error_response(&response));
    }

    #[test]
    fn test_parse_error_insufficient_margin() {
        let response = json!({"error": "Insufficient margin for order"});
        let error = parse_error(&response);
        assert!(error.to_string().contains("Insufficient"));
    }

    #[test]
    fn test_parse_error_invalid_signature() {
        let response = json!({"error": "Invalid signature"});
        let error = parse_error(&response);
        assert!(error.to_string().contains("signature") || error.to_string().contains("Signature"));
    }

    #[test]
    fn test_parse_error_rate_limited() {
        let response = json!({"error": "Rate limit exceeded"});
        let error = parse_error(&response);
        assert!(error.to_string().contains("Rate") || error.to_string().contains("rate"));
    }

    #[test]
    fn test_parse_error_unknown() {
        let response = json!({"error": "Some unknown error occurred"});
        let error = parse_error(&response);
        assert!(error.to_string().contains("unknown") || error.to_string().contains("Unknown"));
    }

    #[test]
    fn test_error_code_display() {
        assert_eq!(
            HyperLiquidErrorCode::InvalidSignature.to_string(),
            "Invalid signature"
        );
        assert_eq!(
            HyperLiquidErrorCode::InsufficientMargin.to_string(),
            "Insufficient margin"
        );
    }

    #[test]
    fn test_error_code_into_ccxt_error() {
        let code = HyperLiquidErrorCode::InsufficientMargin;
        let error: Error = code.into();
        // Just verify it converts without panic
        let _ = error.to_string();
    }
}
