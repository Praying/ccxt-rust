//! WebSocket error types and classification.

use crate::error::Error;

/// WebSocket error classification.
///
/// This enum categorizes WebSocket errors into two types:
/// - `Transient`: Temporary errors that may recover with retry (network issues, server unavailable)
/// - `Permanent`: Errors that should not be retried (authentication failures, protocol errors)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WsErrorKind {
    /// Transient errors that may recover with retry.
    Transient,
    /// Permanent errors that should not be retried.
    Permanent,
}

impl WsErrorKind {
    /// Returns `true` if this is a transient error that may recover with retry.
    #[inline]
    #[must_use]
    pub fn is_transient(self) -> bool {
        matches!(self, Self::Transient)
    }

    /// Returns `true` if this is a permanent error that should not be retried.
    #[inline]
    #[must_use]
    pub fn is_permanent(self) -> bool {
        matches!(self, Self::Permanent)
    }
}

impl std::fmt::Display for WsErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Transient => write!(f, "Transient"),
            Self::Permanent => write!(f, "Permanent"),
        }
    }
}

/// Extended WebSocket error with classification.
#[derive(Debug)]
pub struct WsError {
    kind: WsErrorKind,
    message: String,
    source: Option<Box<dyn std::error::Error + Send + Sync + 'static>>,
}

impl WsError {
    /// Creates a new `WsError` with the specified kind and message.
    pub fn new(kind: WsErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
            source: None,
        }
    }

    /// Creates a new `WsError` with a source error.
    pub fn with_source<E>(kind: WsErrorKind, message: impl Into<String>, source: E) -> Self
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        Self {
            kind,
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }

    /// Creates a transient error.
    pub fn transient(message: impl Into<String>) -> Self {
        Self::new(WsErrorKind::Transient, message)
    }

    /// Creates a transient error with a source.
    pub fn transient_with_source<E>(message: impl Into<String>, source: E) -> Self
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        Self::with_source(WsErrorKind::Transient, message, source)
    }

    /// Creates a permanent error.
    pub fn permanent(message: impl Into<String>) -> Self {
        Self::new(WsErrorKind::Permanent, message)
    }

    /// Creates a permanent error with a source.
    pub fn permanent_with_source<E>(message: impl Into<String>, source: E) -> Self
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        Self::with_source(WsErrorKind::Permanent, message, source)
    }

    /// Returns the error kind.
    #[inline]
    #[must_use]
    pub fn kind(&self) -> WsErrorKind {
        self.kind
    }

    /// Returns the error message.
    #[inline]
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Returns `true` if this is a transient error.
    #[inline]
    #[must_use]
    pub fn is_transient(&self) -> bool {
        self.kind.is_transient()
    }

    /// Returns `true` if this is a permanent error.
    #[inline]
    #[must_use]
    pub fn is_permanent(&self) -> bool {
        self.kind.is_permanent()
    }

    /// Returns the source error, if any.
    #[must_use]
    pub fn source(&self) -> Option<&(dyn std::error::Error + Send + Sync + 'static)> {
        self.source.as_deref()
    }

    /// Classifies a tungstenite WebSocket error.
    pub fn from_tungstenite(err: &tokio_tungstenite::tungstenite::Error) -> Self {
        use tokio_tungstenite::tungstenite::Error as TungError;

        match err {
            TungError::Io(io_err) => {
                let message = format!("IO error: {io_err}");
                Self::transient_with_source(
                    message,
                    std::io::Error::new(io_err.kind(), io_err.to_string()),
                )
            }
            TungError::ConnectionClosed => Self::transient("Connection closed by server"),
            TungError::AlreadyClosed => Self::transient("Connection already closed"),
            TungError::Protocol(protocol_err) => {
                Self::permanent(format!("Protocol error: {protocol_err}"))
            }
            TungError::Utf8(_) => Self::permanent("UTF-8 encoding error in WebSocket message"),
            TungError::Http(response) => {
                let status = response.status();
                let status_code = status.as_u16();
                if status_code == 401 || status_code == 403 {
                    Self::permanent(format!("Authentication error: HTTP {status}"))
                } else if status.is_server_error() {
                    Self::transient(format!("Server error: HTTP {status}"))
                } else {
                    Self::permanent(format!("HTTP error: {status}"))
                }
            }
            TungError::HttpFormat(http_err) => {
                Self::permanent(format!("HTTP format error: {http_err}"))
            }
            TungError::Url(url_err) => Self::permanent(format!("Invalid URL: {url_err}")),
            TungError::Tls(tls_err) => Self::transient(format!("TLS error: {tls_err}")),
            TungError::Capacity(capacity_err) => {
                Self::permanent(format!("Capacity error: {capacity_err}"))
            }
            TungError::WriteBufferFull(msg) => {
                Self::transient(format!("Write buffer full: {msg:?}"))
            }
            TungError::AttackAttempt => Self::permanent("Potential attack detected"),
        }
    }

    /// Classifies a generic error and wraps it in a `WsError`.
    pub fn from_error(err: &Error) -> Self {
        if err.as_authentication().is_some() {
            return Self::permanent(format!("Authentication error: {err}"));
        }
        if err.as_cancelled().is_some() {
            return Self::permanent(format!("Operation cancelled: {err}"));
        }
        if err.as_resource_exhausted().is_some() {
            return Self::permanent(format!("Resource exhausted: {err}"));
        }
        Self::transient(format!("Error: {err}"))
    }
}

impl std::fmt::Display for WsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {}", self.kind, self.message)
    }
}

impl std::error::Error for WsError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.source
            .as_ref()
            .map(|e| e.as_ref() as &(dyn std::error::Error + 'static))
    }
}
