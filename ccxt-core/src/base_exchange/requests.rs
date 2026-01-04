//! Request handling utilities and helper methods

use crate::error::Error;
use serde_json::Value;
use std::collections::HashMap;

/// Request utility methods
pub struct RequestUtils;

impl RequestUtils {
    /// Builds a URL query string from parameters
    pub fn build_query_string(params: &HashMap<String, Value>) -> String {
        if params.is_empty() {
            return String::new();
        }

        let pairs: Vec<String> = params
            .iter()
            .map(|(k, v)| {
                let value_str = match v {
                    Value::String(s) => s.clone(),
                    Value::Number(n) => n.to_string(),
                    Value::Bool(b) => b.to_string(),
                    _ => v.to_string(),
                };
                format!("{}={}", k, urlencoding::encode(&value_str))
            })
            .collect();

        pairs.join("&")
    }

    /// Parses a JSON response string
    pub fn parse_json(response: &str) -> Result<Value, Error> {
        serde_json::from_str(response).map_err(|e| Error::invalid_request(e.to_string()))
    }

    /// Handles HTTP error responses by mapping status codes to appropriate errors
    pub fn handle_http_error(status_code: u16, response: &str) -> Error {
        match status_code {
            400 => Error::invalid_request(response.to_string()),
            401 | 403 => Error::authentication(response.to_string()),
            404 => Error::invalid_request(format!("Endpoint not found: {response}")),
            429 => Error::rate_limit(response.to_string(), None),
            500..=599 => Error::exchange(status_code.to_string(), response),
            _ => Error::network(format!("HTTP {status_code}: {response}")),
        }
    }

    /// Safely extracts a string value from a JSON object
    pub fn safe_string(dict: &Value, key: &str) -> Option<String> {
        dict.get(key)
            .and_then(|v| v.as_str())
            .map(ToString::to_string)
    }

    /// Safely extracts an integer value from a JSON object
    pub fn safe_integer(dict: &Value, key: &str) -> Option<i64> {
        dict.get(key).and_then(Value::as_i64)
    }

    /// Safely extracts a float value from a JSON object
    pub fn safe_float(dict: &Value, key: &str) -> Option<f64> {
        dict.get(key).and_then(Value::as_f64)
    }

    /// Safely extracts a boolean value from a JSON object
    pub fn safe_bool(dict: &Value, key: &str) -> Option<bool> {
        dict.get(key).and_then(Value::as_bool)
    }
}
