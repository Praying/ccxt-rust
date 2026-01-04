//! WebSocket connection events.

use std::sync::Arc;
use std::time::Duration;

/// WebSocket connection event types.
#[derive(Debug, Clone)]
pub enum WsEvent {
    /// Connection attempt started.
    Connecting,
    /// Connection established successfully.
    Connected,
    /// Connection closed.
    Disconnected,
    /// Reconnection in progress.
    Reconnecting {
        /// Current reconnection attempt number (1-indexed).
        attempt: u32,
        /// Delay before this reconnection attempt.
        delay: Duration,
        /// Error that triggered the reconnection (if any).
        error: Option<String>,
    },
    /// Reconnection succeeded.
    ReconnectSuccess,
    /// Single reconnection attempt failed.
    ReconnectFailed {
        /// The attempt number that failed (1-indexed).
        attempt: u32,
        /// Error message describing the failure.
        error: String,
        /// Whether this is a permanent error that should not be retried.
        is_permanent: bool,
    },
    /// All reconnection attempts exhausted.
    ReconnectExhausted {
        /// Total number of reconnection attempts made.
        total_attempts: u32,
        /// The last error encountered.
        last_error: String,
    },
    /// Subscriptions restored after reconnection.
    SubscriptionRestored,
    /// Permanent error occurred (no retry).
    PermanentError {
        /// Error message describing the permanent failure.
        error: String,
    },
    /// Shutdown completed.
    Shutdown,
}

impl WsEvent {
    /// Returns true if this is a Connecting event.
    #[inline]
    #[must_use]
    pub fn is_connecting(&self) -> bool {
        matches!(self, Self::Connecting)
    }

    /// Returns true if this is a Connected event.
    #[inline]
    #[must_use]
    pub fn is_connected(&self) -> bool {
        matches!(self, Self::Connected)
    }

    /// Returns true if this is a Disconnected event.
    #[inline]
    #[must_use]
    pub fn is_disconnected(&self) -> bool {
        matches!(self, Self::Disconnected)
    }

    /// Returns true if this is a Reconnecting event.
    #[inline]
    #[must_use]
    pub fn is_reconnecting(&self) -> bool {
        matches!(self, Self::Reconnecting { .. })
    }

    /// Returns true if this is a ReconnectSuccess event.
    #[inline]
    #[must_use]
    pub fn is_reconnect_success(&self) -> bool {
        matches!(self, Self::ReconnectSuccess)
    }

    /// Returns true if this is a ReconnectFailed event.
    #[inline]
    #[must_use]
    pub fn is_reconnect_failed(&self) -> bool {
        matches!(self, Self::ReconnectFailed { .. })
    }

    /// Returns true if this is a ReconnectExhausted event.
    #[inline]
    #[must_use]
    pub fn is_reconnect_exhausted(&self) -> bool {
        matches!(self, Self::ReconnectExhausted { .. })
    }

    /// Returns true if this is a SubscriptionRestored event.
    #[inline]
    #[must_use]
    pub fn is_subscription_restored(&self) -> bool {
        matches!(self, Self::SubscriptionRestored)
    }

    /// Returns true if this is a PermanentError event.
    #[inline]
    #[must_use]
    pub fn is_permanent_error(&self) -> bool {
        matches!(self, Self::PermanentError { .. })
    }

    /// Returns true if this is a Shutdown event.
    #[inline]
    #[must_use]
    pub fn is_shutdown(&self) -> bool {
        matches!(self, Self::Shutdown)
    }

    /// Returns true if this is any error event.
    #[inline]
    #[must_use]
    pub fn is_error(&self) -> bool {
        matches!(
            self,
            Self::ReconnectFailed { .. }
                | Self::ReconnectExhausted { .. }
                | Self::PermanentError { .. }
        )
    }

    /// Returns true if this is a terminal event.
    #[inline]
    #[must_use]
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            Self::ReconnectExhausted { .. } | Self::PermanentError { .. } | Self::Shutdown
        )
    }
}

impl std::fmt::Display for WsEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Connecting => write!(f, "Connecting"),
            Self::Connected => write!(f, "Connected"),
            Self::Disconnected => write!(f, "Disconnected"),
            Self::Reconnecting {
                attempt,
                delay,
                error,
            } => {
                write!(f, "Reconnecting (attempt {attempt}, delay: {delay:?}")?;
                if let Some(err) = error {
                    write!(f, ", error: {err}")?;
                }
                write!(f, ")")
            }
            Self::ReconnectSuccess => write!(f, "ReconnectSuccess"),
            Self::ReconnectFailed {
                attempt,
                error,
                is_permanent,
            } => {
                write!(
                    f,
                    "ReconnectFailed (attempt {attempt}, error: {error}, permanent: {is_permanent})"
                )
            }
            Self::ReconnectExhausted {
                total_attempts,
                last_error,
            } => {
                write!(
                    f,
                    "ReconnectExhausted (attempts: {total_attempts}, last error: {last_error})"
                )
            }
            Self::SubscriptionRestored => write!(f, "SubscriptionRestored"),
            Self::PermanentError { error } => write!(f, "PermanentError: {error}"),
            Self::Shutdown => write!(f, "Shutdown"),
        }
    }
}

/// Event callback function type.
pub type WsEventCallback = Arc<dyn Fn(WsEvent) + Send + Sync>;
