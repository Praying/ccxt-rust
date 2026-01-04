//! From implementations for converting between error types.

use crate::error::{Error, NetworkError, OrderError, ParseError};

/// Maximum length for error messages to prevent memory bloat from large HTTP responses.
pub(crate) const MAX_ERROR_MESSAGE_LEN: usize = 1024;

/// Truncates a string to a maximum length, adding "... (truncated)" if needed.
pub(crate) fn truncate_message(mut msg: String) -> String {
    if msg.len() > MAX_ERROR_MESSAGE_LEN {
        msg.truncate(MAX_ERROR_MESSAGE_LEN);
        msg.push_str("... (truncated)");
    }
    msg
}

impl From<NetworkError> for Error {
    fn from(e: NetworkError) -> Self {
        Error::Network(Box::new(e))
    }
}

impl From<Box<NetworkError>> for Error {
    fn from(e: Box<NetworkError>) -> Self {
        Error::Network(e)
    }
}

impl From<ParseError> for Error {
    fn from(e: ParseError) -> Self {
        Error::Parse(Box::new(e))
    }
}

impl From<Box<ParseError>> for Error {
    fn from(e: Box<ParseError>) -> Self {
        Error::Parse(e)
    }
}

impl From<OrderError> for Error {
    fn from(e: OrderError) -> Self {
        Error::Order(Box::new(e))
    }
}

impl From<Box<OrderError>> for Error {
    fn from(e: Box<OrderError>) -> Self {
        Error::Order(e)
    }
}

impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        Error::Parse(Box::new(ParseError::Json(e)))
    }
}

impl From<rust_decimal::Error> for Error {
    fn from(e: rust_decimal::Error) -> Self {
        Error::Parse(Box::new(ParseError::Decimal(e)))
    }
}

impl From<reqwest::Error> for NetworkError {
    fn from(e: reqwest::Error) -> Self {
        if e.is_timeout() {
            NetworkError::Timeout
        } else if e.is_connect() {
            NetworkError::ConnectionFailed(truncate_message(e.to_string()))
        } else if let Some(status) = e.status() {
            NetworkError::RequestFailed {
                status: status.as_u16(),
                message: truncate_message(e.to_string()),
            }
        } else {
            NetworkError::Transport(Box::new(e))
        }
    }
}

impl From<reqwest::Error> for Error {
    fn from(e: reqwest::Error) -> Self {
        Error::Network(Box::new(NetworkError::from(e)))
    }
}
