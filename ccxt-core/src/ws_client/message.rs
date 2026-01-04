//! WebSocket message types.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// WebSocket message types for exchange communication.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum WsMessage {
    /// Subscribe to a channel
    Subscribe {
        /// Channel name
        channel: String,
        /// Optional trading pair symbol
        symbol: Option<String>,
        /// Additional parameters
        params: Option<HashMap<String, Value>>,
    },
    /// Unsubscribe from a channel
    Unsubscribe {
        /// Channel name
        channel: String,
        /// Optional trading pair symbol
        symbol: Option<String>,
    },
    /// Ping message for keepalive
    Ping {
        /// Timestamp in milliseconds
        timestamp: i64,
    },
    /// Pong response to ping
    Pong {
        /// Timestamp in milliseconds
        timestamp: i64,
    },
    /// Authentication message
    Auth {
        /// API key
        api_key: String,
        /// HMAC signature
        signature: String,
        /// Timestamp in milliseconds
        timestamp: i64,
    },
    /// Custom message payload
    Custom(Value),
}
