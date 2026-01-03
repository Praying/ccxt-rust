//! WebSocket client module.
//!
//! Provides asynchronous WebSocket connection management, subscription handling,
//! and heartbeat maintenance for cryptocurrency exchange streaming APIs.
//!
//! # Features
//!
//! - **Exponential Backoff Reconnection**: Configurable retry delays with jitter
//!   to prevent thundering herd effects during reconnection.
//! - **Error Classification**: Distinguishes between transient (retryable) and
//!   permanent (non-retryable) errors for intelligent reconnection decisions.
//! - **Cancellation Support**: Graceful cancellation of long-running operations
//!   via [`CancellationToken`](tokio_util::sync::CancellationToken).
//! - **Subscription Limits**: Configurable maximum subscription count to prevent
//!   resource exhaustion.
//! - **Lock-Free Statistics**: Atomic counters for connection statistics to
//!   prevent deadlocks in async contexts.
//! - **Graceful Shutdown**: Clean shutdown with pending operation completion
//!   and resource cleanup.
//!
//! # Exponential Backoff Configuration
//!
//! The [`BackoffConfig`] struct controls how retry delays are calculated during
//! reconnection attempts. The formula used is:
//!
//! ```text
//! delay = min(base_delay * multiplier^attempt, max_delay) + jitter
//! ```
//!
//! Where jitter is a random value in the range `[0, delay * jitter_factor]`.
//!
//! ## Default Configuration
//!
//! | Parameter | Default Value | Description |
//! |-----------|---------------|-------------|
//! | `base_delay` | 1 second | Initial delay before first retry |
//! | `max_delay` | 60 seconds | Maximum delay cap |
//! | `jitter_factor` | 0.25 (25%) | Random jitter to prevent thundering herd |
//! | `multiplier` | 2.0 | Exponential growth factor |
//!
//! ## Example: Custom Backoff Configuration
//!
//! ```rust
//! use ccxt_core::ws_client::{WsConfig, BackoffConfig};
//! use std::time::Duration;
//!
//! let config = WsConfig {
//!     url: "wss://stream.example.com/ws".to_string(),
//!     backoff_config: BackoffConfig {
//!         base_delay: Duration::from_millis(500),  // Start with 500ms
//!         max_delay: Duration::from_secs(30),      // Cap at 30 seconds
//!         jitter_factor: 0.2,                      // 20% jitter
//!         multiplier: 2.0,                         // Double each attempt
//!     },
//!     ..Default::default()
//! };
//! ```
//!
//! ## Retry Delay Progression (with default config, no jitter)
//!
//! | Attempt | Delay |
//! |---------|-------|
//! | 0 | 1s |
//! | 1 | 2s |
//! | 2 | 4s |
//! | 3 | 8s |
//! | 4 | 16s |
//! | 5 | 32s |
//! | 6+ | 60s (capped) |
//!
//! # Cancellation Support
//!
//! Long-running operations like `connect`, `reconnect`, and `subscribe` can be
//! cancelled using a [`CancellationToken`](tokio_util::sync::CancellationToken).
//! This enables graceful shutdown and timeout handling.
//!
//! ## Example: Using CancellationToken
//!
//! ```rust,ignore
//! use ccxt_core::ws_client::{WsClient, WsConfig};
//! use tokio_util::sync::CancellationToken;
//! use std::time::Duration;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let client = WsClient::new(WsConfig {
//!         url: "wss://stream.example.com/ws".to_string(),
//!         ..Default::default()
//!     });
//!
//!     // Create a cancellation token
//!     let token = CancellationToken::new();
//!     let token_clone = token.clone();
//!
//!     // Set the token on the client
//!     client.set_cancel_token(token.clone()).await;
//!
//!     // Spawn a task to cancel after 10 seconds
//!     tokio::spawn(async move {
//!         tokio::time::sleep(Duration::from_secs(10)).await;
//!         println!("Cancelling connection...");
//!         token_clone.cancel();
//!     });
//!
//!     // Connect with cancellation support
//!     match client.connect_with_cancel(Some(token)).await {
//!         Ok(()) => println!("Connected successfully!"),
//!         Err(e) if e.as_cancelled().is_some() => {
//!             println!("Connection was cancelled");
//!         }
//!         Err(e) => println!("Connection failed: {}", e),
//!     }
//!
//!     Ok(())
//! }
//! ```
//!
//! ## Sharing CancellationToken
//!
//! The `CancellationToken` can be cloned and shared across multiple operations.
//! Cancelling any clone will cancel all operations using that token:
//!
//! ```rust,ignore
//! use tokio_util::sync::CancellationToken;
//!
//! let token = CancellationToken::new();
//!
//! // Clone for different operations
//! let connect_token = token.clone();
//! let reconnect_token = token.clone();
//!
//! // Cancelling the original cancels all clones
//! token.cancel();
//!
//! assert!(connect_token.is_cancelled());
//! assert!(reconnect_token.is_cancelled());
//! ```
//!
//! # Subscription Limits
//!
//! The [`WsConfig::max_subscriptions`] field limits the number of concurrent
//! subscriptions to prevent resource exhaustion. When the limit is reached,
//! new subscription attempts will fail with [`Error::ResourceExhausted`](crate::error::Error).
//!
//! ## Default Limit
//!
//! The default maximum is 100 subscriptions (see [`DEFAULT_MAX_SUBSCRIPTIONS`]).
//!
//! ## Example: Configuring Subscription Limits
//!
//! ```rust
//! use ccxt_core::ws_client::{WsClient, WsConfig};
//!
//! let client = WsClient::new(WsConfig {
//!     url: "wss://stream.example.com/ws".to_string(),
//!     max_subscriptions: 50,  // Limit to 50 subscriptions
//!     ..Default::default()
//! });
//!
//! // Check current capacity
//! assert_eq!(client.subscription_count(), 0);
//! assert_eq!(client.remaining_capacity(), 50);
//! ```
//!
//! ## Checking Capacity Before Subscribing
//!
//! ```rust,ignore
//! use ccxt_core::ws_client::{WsClient, WsConfig};
//!
//! let client = WsClient::new(WsConfig::default());
//!
//! // Check if there's room for more subscriptions
//! if client.remaining_capacity() > 0 {
//!     client.subscribe("ticker".to_string(), Some("BTC/USDT".to_string()), None).await?;
//! } else {
//!     println!("No subscription capacity available");
//! }
//! ```
//!
//! # Error Classification
//!
//! The [`WsErrorKind`] enum classifies WebSocket errors into two categories:
//!
//! - **Transient**: Temporary errors that may recover with retry (network issues,
//!   server unavailable, connection resets).
//! - **Permanent**: Errors that should not be retried (authentication failures,
//!   protocol errors, invalid credentials).
//!
//! ## Example: Handling Errors by Type
//!
//! ```rust
//! use ccxt_core::ws_client::{WsError, WsErrorKind};
//!
//! fn handle_error(error: &WsError) {
//!     if error.is_transient() {
//!         println!("Transient error, will retry: {}", error.message());
//!     } else {
//!         println!("Permanent error, stopping: {}", error.message());
//!     }
//! }
//! ```
//!
//! # Graceful Shutdown
//!
//! The [`WsClient::shutdown`] method performs a complete graceful shutdown:
//!
//! 1. Cancels all pending reconnection attempts
//! 2. Sends WebSocket close frame to the server
//! 3. Waits for pending operations to complete (with timeout)
//! 4. Clears all resources (subscriptions, channels, etc.)
//! 5. Emits a [`WsEvent::Shutdown`] event
//!
//! ## Example: Graceful Shutdown
//!
//! ```rust,ignore
//! use ccxt_core::ws_client::{WsClient, WsConfig, WsEvent};
//! use std::sync::Arc;
//!
//! let client = WsClient::new(WsConfig {
//!     url: "wss://stream.example.com/ws".to_string(),
//!     shutdown_timeout: 5000,  // 5 second timeout
//!     ..Default::default()
//! });
//!
//! // Set up event callback
//! client.set_event_callback(Arc::new(|event| {
//!     if let WsEvent::Shutdown = event {
//!         println!("Shutdown completed!");
//!     }
//! })).await;
//!
//! // Connect and do work...
//! client.connect().await?;
//!
//! // Gracefully shutdown
//! client.shutdown().await;
//! ```
//!
//! # Observability
//!
//! This module uses the `tracing` crate for structured logging. Key events:
//! - Connection establishment and disconnection
//! - Subscription and unsubscription events with stream names
//! - Message parsing failures with raw message preview (truncated)
//! - Reconnection attempts and outcomes with backoff delays
//! - Ping/pong heartbeat events
//! - Cancellation events
//! - Shutdown progress
//!
//! # Lock Ordering Rules
//!
//! To prevent deadlocks, locks in this module are acquired in a consistent order:
//!
//! 1. `cancel_token` - Cancellation token mutex
//! 2. `event_callback` - Event callback mutex
//! 3. `write_tx` - Write channel mutex
//! 4. `shutdown_tx` - Shutdown channel mutex
//!
//! **Important**: No locks are held across await points. All lock guards are
//! dropped before any async operations.

use crate::error::{Error, Result};
use dashmap::DashMap;
use futures_util::{SinkExt, StreamExt, stream::SplitSink};
use rand::Rng;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicI64, AtomicU8, AtomicU32, AtomicU64, Ordering};
use tokio::net::TcpStream;
use tokio::sync::{Mutex, RwLock, mpsc};
use tokio::task::JoinHandle;
use tokio::time::{Duration, interval};
use tokio_tungstenite::{
    MaybeTlsStream, WebSocketStream, connect_async, tungstenite::protocol::Message,
};
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, instrument, warn};

// ==================== WebSocket Error Classification ====================

/// WebSocket error classification.
///
/// This enum categorizes WebSocket errors into two types:
/// - `Transient`: Temporary errors that may recover with retry (network issues, server unavailable)
/// - `Permanent`: Errors that should not be retried (authentication failures, protocol errors)
///
/// # Example
///
/// ```rust
/// use ccxt_core::ws_client::WsErrorKind;
///
/// let kind = WsErrorKind::Transient;
/// assert!(kind.is_transient());
///
/// let kind = WsErrorKind::Permanent;
/// assert!(!kind.is_transient());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WsErrorKind {
    /// Transient errors that may recover with retry.
    ///
    /// Examples:
    /// - Network timeouts
    /// - Connection resets
    /// - Server unavailable (5xx errors)
    /// - Temporary connection failures
    Transient,

    /// Permanent errors that should not be retried.
    ///
    /// Examples:
    /// - Authentication failures (401/403)
    /// - Protocol errors
    /// - Invalid credentials
    /// - Invalid parameters
    Permanent,
}

impl WsErrorKind {
    /// Returns `true` if this is a transient error that may recover with retry.
    ///
    /// # Example
    ///
    /// ```rust
    /// use ccxt_core::ws_client::WsErrorKind;
    ///
    /// assert!(WsErrorKind::Transient.is_transient());
    /// assert!(!WsErrorKind::Permanent.is_transient());
    /// ```
    #[inline]
    #[must_use]
    pub fn is_transient(self) -> bool {
        matches!(self, Self::Transient)
    }

    /// Returns `true` if this is a permanent error that should not be retried.
    ///
    /// # Example
    ///
    /// ```rust
    /// use ccxt_core::ws_client::WsErrorKind;
    ///
    /// assert!(WsErrorKind::Permanent.is_permanent());
    /// assert!(!WsErrorKind::Transient.is_permanent());
    /// ```
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
///
/// This struct wraps WebSocket errors with additional metadata including:
/// - Error kind (transient or permanent)
/// - Human-readable message
/// - Optional source error for error chaining
///
/// # Example
///
/// ```rust
/// use ccxt_core::ws_client::{WsError, WsErrorKind};
///
/// // Create a transient error
/// let err = WsError::transient("Connection reset by peer");
/// assert!(err.is_transient());
/// assert_eq!(err.kind(), WsErrorKind::Transient);
///
/// // Create a permanent error
/// let err = WsError::permanent("Authentication failed: invalid API key");
/// assert!(!err.is_transient());
/// assert_eq!(err.kind(), WsErrorKind::Permanent);
/// ```
#[derive(Debug)]
pub struct WsError {
    /// Error kind (transient or permanent)
    kind: WsErrorKind,
    /// Human-readable error message
    message: String,
    /// Original error source (if any)
    source: Option<Box<dyn std::error::Error + Send + Sync + 'static>>,
}

impl WsError {
    /// Creates a new `WsError` with the specified kind and message.
    ///
    /// # Arguments
    ///
    /// * `kind` - The error classification (transient or permanent)
    /// * `message` - Human-readable error message
    ///
    /// # Example
    ///
    /// ```rust
    /// use ccxt_core::ws_client::{WsError, WsErrorKind};
    ///
    /// let err = WsError::new(WsErrorKind::Transient, "Connection timeout");
    /// assert!(err.is_transient());
    /// ```
    pub fn new(kind: WsErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
            source: None,
        }
    }

    /// Creates a new `WsError` with a source error.
    ///
    /// # Arguments
    ///
    /// * `kind` - The error classification (transient or permanent)
    /// * `message` - Human-readable error message
    /// * `source` - The underlying error that caused this error
    ///
    /// # Example
    ///
    /// ```rust
    /// use ccxt_core::ws_client::{WsError, WsErrorKind};
    /// use std::io;
    ///
    /// let io_err = io::Error::new(io::ErrorKind::ConnectionReset, "connection reset");
    /// let err = WsError::with_source(
    ///     WsErrorKind::Transient,
    ///     "Connection lost",
    ///     io_err
    /// );
    /// assert!(err.source().is_some());
    /// ```
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
    ///
    /// Transient errors are temporary and may recover with retry.
    ///
    /// # Arguments
    ///
    /// * `message` - Human-readable error message
    ///
    /// # Example
    ///
    /// ```rust
    /// use ccxt_core::ws_client::WsError;
    ///
    /// let err = WsError::transient("Network timeout");
    /// assert!(err.is_transient());
    /// ```
    pub fn transient(message: impl Into<String>) -> Self {
        Self::new(WsErrorKind::Transient, message)
    }

    /// Creates a transient error with a source.
    ///
    /// # Arguments
    ///
    /// * `message` - Human-readable error message
    /// * `source` - The underlying error that caused this error
    pub fn transient_with_source<E>(message: impl Into<String>, source: E) -> Self
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        Self::with_source(WsErrorKind::Transient, message, source)
    }

    /// Creates a permanent error.
    ///
    /// Permanent errors should not be retried as they indicate
    /// a fundamental issue that won't resolve with retries.
    ///
    /// # Arguments
    ///
    /// * `message` - Human-readable error message
    ///
    /// # Example
    ///
    /// ```rust
    /// use ccxt_core::ws_client::WsError;
    ///
    /// let err = WsError::permanent("Invalid API key");
    /// assert!(!err.is_transient());
    /// ```
    pub fn permanent(message: impl Into<String>) -> Self {
        Self::new(WsErrorKind::Permanent, message)
    }

    /// Creates a permanent error with a source.
    ///
    /// # Arguments
    ///
    /// * `message` - Human-readable error message
    /// * `source` - The underlying error that caused this error
    pub fn permanent_with_source<E>(message: impl Into<String>, source: E) -> Self
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        Self::with_source(WsErrorKind::Permanent, message, source)
    }

    /// Returns the error kind.
    ///
    /// # Example
    ///
    /// ```rust
    /// use ccxt_core::ws_client::{WsError, WsErrorKind};
    ///
    /// let err = WsError::transient("timeout");
    /// assert_eq!(err.kind(), WsErrorKind::Transient);
    /// ```
    #[inline]
    #[must_use]
    pub fn kind(&self) -> WsErrorKind {
        self.kind
    }

    /// Returns the error message.
    ///
    /// # Example
    ///
    /// ```rust
    /// use ccxt_core::ws_client::WsError;
    ///
    /// let err = WsError::transient("Connection timeout");
    /// assert_eq!(err.message(), "Connection timeout");
    /// ```
    #[inline]
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Returns `true` if this is a transient error.
    ///
    /// Transient errors may recover with retry.
    ///
    /// # Example
    ///
    /// ```rust
    /// use ccxt_core::ws_client::WsError;
    ///
    /// let err = WsError::transient("timeout");
    /// assert!(err.is_transient());
    ///
    /// let err = WsError::permanent("auth failed");
    /// assert!(!err.is_transient());
    /// ```
    #[inline]
    #[must_use]
    pub fn is_transient(&self) -> bool {
        self.kind.is_transient()
    }

    /// Returns `true` if this is a permanent error.
    ///
    /// Permanent errors should not be retried.
    ///
    /// # Example
    ///
    /// ```rust
    /// use ccxt_core::ws_client::WsError;
    ///
    /// let err = WsError::permanent("auth failed");
    /// assert!(err.is_permanent());
    ///
    /// let err = WsError::transient("timeout");
    /// assert!(!err.is_permanent());
    /// ```
    #[inline]
    #[must_use]
    pub fn is_permanent(&self) -> bool {
        self.kind.is_permanent()
    }

    /// Returns the source error, if any.
    ///
    /// # Example
    ///
    /// ```rust
    /// use ccxt_core::ws_client::WsError;
    /// use std::io;
    ///
    /// let io_err = io::Error::new(io::ErrorKind::ConnectionReset, "reset");
    /// let err = WsError::transient_with_source("Connection lost", io_err);
    /// assert!(err.source().is_some());
    /// ```
    #[must_use]
    pub fn source(&self) -> Option<&(dyn std::error::Error + Send + Sync + 'static)> {
        self.source.as_deref()
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

impl WsError {
    /// Classifies a tungstenite WebSocket error.
    ///
    /// This method analyzes the error type and classifies it as either
    /// transient (retryable) or permanent (non-retryable).
    ///
    /// # Classification Rules
    ///
    /// ## Transient Errors (retryable)
    /// - IO errors (network issues, connection resets)
    /// - `ConnectionClosed` (server closed connection)
    /// - `AlreadyClosed` (connection was already closed)
    /// - Server errors (5xx HTTP status codes)
    ///
    /// ## Permanent Errors (non-retryable)
    /// - Protocol errors (WebSocket protocol violations)
    /// - UTF-8 encoding errors
    /// - Authentication errors (401/403 HTTP status codes)
    /// - Client errors (4xx HTTP status codes, except 5xx)
    ///
    /// # Arguments
    ///
    /// * `err` - The tungstenite error to classify
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use ccxt_core::ws_client::WsError;
    /// use tokio_tungstenite::tungstenite::Error as TungError;
    ///
    /// // IO errors are transient
    /// let io_err = std::io::Error::new(std::io::ErrorKind::ConnectionReset, "reset");
    /// let ws_err = WsError::from_tungstenite(&TungError::Io(io_err));
    /// assert!(ws_err.is_transient());
    /// ```
    pub fn from_tungstenite(err: &tokio_tungstenite::tungstenite::Error) -> Self {
        use tokio_tungstenite::tungstenite::Error as TungError;

        match err {
            // Transient errors - network issues, temporary failures
            TungError::Io(io_err) => {
                let message = format!("IO error: {io_err}");
                Self::transient_with_source(
                    message,
                    std::io::Error::new(io_err.kind(), io_err.to_string()),
                )
            }

            TungError::ConnectionClosed => Self::transient("Connection closed by server"),

            TungError::AlreadyClosed => Self::transient("Connection already closed"),

            // Permanent errors - protocol violations, auth failures
            TungError::Protocol(protocol_err) => {
                let message = format!("Protocol error: {protocol_err}");
                Self::permanent(message)
            }

            TungError::Utf8(_) => Self::permanent("UTF-8 encoding error in WebSocket message"),

            TungError::Http(response) => {
                let status = response.status();
                let status_code = status.as_u16();

                if status_code == 401 || status_code == 403 {
                    // Authentication errors are permanent
                    Self::permanent(format!("Authentication error: HTTP {status}"))
                } else if status.is_server_error() {
                    // Server errors (5xx) are transient
                    Self::transient(format!("Server error: HTTP {status}"))
                } else {
                    // Other client errors (4xx) are permanent
                    Self::permanent(format!("HTTP error: {status}"))
                }
            }

            TungError::HttpFormat(http_err) => {
                Self::permanent(format!("HTTP format error: {http_err}"))
            }

            TungError::Url(url_err) => Self::permanent(format!("Invalid URL: {url_err}")),

            TungError::Tls(tls_err) => {
                // TLS errors could be transient (certificate issues during handshake)
                // or permanent (invalid certificates). We treat them as transient
                // to allow retry with potential certificate refresh.
                Self::transient(format!("TLS error: {tls_err}"))
            }

            TungError::Capacity(capacity_err) => {
                // Capacity errors (message too large) are permanent
                Self::permanent(format!("Capacity error: {capacity_err}"))
            }

            TungError::WriteBufferFull(msg) => {
                // Write buffer full is transient - can retry after buffer drains
                Self::transient(format!("Write buffer full: {msg:?}"))
            }

            TungError::AttackAttempt => {
                // Attack attempts are permanent - don't retry
                Self::permanent("Potential attack detected")
            }
        }
    }

    /// Classifies a generic error and wraps it in a `WsError`.
    ///
    /// This is a convenience method for wrapping arbitrary errors.
    /// By default, unknown errors are classified as transient.
    ///
    /// # Arguments
    ///
    /// * `err` - The error to classify
    ///
    /// # Example
    ///
    /// ```rust
    /// use ccxt_core::ws_client::WsError;
    /// use ccxt_core::error::Error;
    ///
    /// let err = Error::network("Connection failed");
    /// let ws_err = WsError::from_error(&err);
    /// assert!(ws_err.is_transient());
    /// ```
    pub fn from_error(err: &Error) -> Self {
        // Check for specific error types that indicate permanent failures
        if err.as_authentication().is_some() {
            return Self::permanent(format!("Authentication error: {err}"));
        }

        if err.as_cancelled().is_some() {
            // Cancelled is a special case - not really transient or permanent
            // but we treat it as permanent since retrying won't help
            return Self::permanent(format!("Operation cancelled: {err}"));
        }

        if err.as_resource_exhausted().is_some() {
            // Resource exhausted is permanent until resources are freed
            return Self::permanent(format!("Resource exhausted: {err}"));
        }

        // Default to transient for network and other errors
        Self::transient(format!("Error: {err}"))
    }
}

/// Exponential backoff configuration for reconnection.
///
/// This configuration controls how the WebSocket client calculates retry delays
/// when attempting to reconnect after a connection failure.
///
/// # Example
///
/// ```
/// use ccxt_core::ws_client::BackoffConfig;
/// use std::time::Duration;
///
/// let config = BackoffConfig {
///     base_delay: Duration::from_millis(500),
///     max_delay: Duration::from_secs(30),
///     jitter_factor: 0.2,
///     multiplier: 2.0,
/// };
/// ```
#[derive(Debug, Clone)]
pub struct BackoffConfig {
    /// Base delay for first retry (default: 1 second)
    ///
    /// This is the initial delay before the first reconnection attempt.
    pub base_delay: Duration,

    /// Maximum delay cap (default: 60 seconds)
    ///
    /// The calculated delay will never exceed this value (before jitter).
    pub max_delay: Duration,

    /// Jitter factor (0.0 - 1.0, default: 0.25 for 25%)
    ///
    /// Random jitter is added to prevent thundering herd effect.
    /// The jitter range is [0, delay * jitter_factor].
    pub jitter_factor: f64,

    /// Multiplier for exponential growth (default: 2.0)
    ///
    /// Each retry delay is multiplied by this factor.
    pub multiplier: f64,
}

impl Default for BackoffConfig {
    fn default() -> Self {
        Self {
            base_delay: Duration::from_secs(1),
            max_delay: Duration::from_secs(60),
            jitter_factor: 0.25,
            multiplier: 2.0,
        }
    }
}

/// Calculates retry delay with exponential backoff and jitter.
///
/// The backoff strategy uses the formula:
/// `min(base_delay * multiplier^attempt, max_delay) + jitter`
///
/// Where jitter is a random value in the range [0, delay * jitter_factor].
///
/// # Example
///
/// ```
/// use ccxt_core::ws_client::{BackoffConfig, BackoffStrategy};
///
/// let strategy = BackoffStrategy::new(BackoffConfig::default());
///
/// // First attempt (attempt = 0): ~1 second + jitter
/// let delay0 = strategy.calculate_delay(0);
///
/// // Second attempt (attempt = 1): ~2 seconds + jitter
/// let delay1 = strategy.calculate_delay(1);
///
/// // Third attempt (attempt = 2): ~4 seconds + jitter
/// let delay2 = strategy.calculate_delay(2);
/// ```
#[derive(Debug, Clone)]
pub struct BackoffStrategy {
    config: BackoffConfig,
}

impl BackoffStrategy {
    /// Creates a new backoff strategy with the given configuration.
    ///
    /// # Arguments
    ///
    /// * `config` - The backoff configuration to use
    pub fn new(config: BackoffConfig) -> Self {
        Self { config }
    }

    /// Creates a new backoff strategy with default configuration.
    pub fn with_defaults() -> Self {
        Self::new(BackoffConfig::default())
    }

    /// Returns a reference to the underlying configuration.
    pub fn config(&self) -> &BackoffConfig {
        &self.config
    }

    /// Calculates delay for the given attempt number.
    ///
    /// Formula: `min(base_delay * multiplier^attempt, max_delay) + jitter`
    ///
    /// # Arguments
    ///
    /// * `attempt` - The attempt number (0-indexed)
    ///
    /// # Returns
    ///
    /// The calculated delay duration including jitter.
    ///
    /// # Example
    ///
    /// ```
    /// use ccxt_core::ws_client::{BackoffConfig, BackoffStrategy};
    /// use std::time::Duration;
    ///
    /// let config = BackoffConfig {
    ///     base_delay: Duration::from_secs(1),
    ///     max_delay: Duration::from_secs(60),
    ///     jitter_factor: 0.0, // No jitter for predictable results
    ///     multiplier: 2.0,
    /// };
    /// let strategy = BackoffStrategy::new(config);
    ///
    /// // With no jitter, delays are exactly: 1s, 2s, 4s, 8s, ...
    /// assert_eq!(strategy.calculate_delay(0), Duration::from_secs(1));
    /// assert_eq!(strategy.calculate_delay(1), Duration::from_secs(2));
    /// assert_eq!(strategy.calculate_delay(2), Duration::from_secs(4));
    /// ```
    #[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
    pub fn calculate_delay(&self, attempt: u32) -> Duration {
        let base_ms = self.config.base_delay.as_millis() as f64;
        let multiplier = self.config.multiplier;
        let max_ms = self.config.max_delay.as_millis() as f64;

        // Calculate exponential delay: base_delay * multiplier^attempt
        let exponential_delay_ms = base_ms * multiplier.powi(attempt as i32);

        // Cap at max_delay
        let capped_delay_ms = exponential_delay_ms.min(max_ms);

        // Add jitter: random value in [0, delay * jitter_factor]
        let jitter_ms = if self.config.jitter_factor > 0.0 {
            let jitter_range = capped_delay_ms * self.config.jitter_factor;
            rand::rng().random::<f64>() * jitter_range
        } else {
            0.0
        };

        Duration::from_millis((capped_delay_ms + jitter_ms) as u64)
    }

    /// Calculates the base delay (without jitter) for the given attempt number.
    ///
    /// This is useful for testing or when you need predictable delay values.
    ///
    /// # Arguments
    ///
    /// * `attempt` - The attempt number (0-indexed)
    ///
    /// # Returns
    ///
    /// The calculated delay duration without jitter.
    #[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
    pub fn calculate_delay_without_jitter(&self, attempt: u32) -> Duration {
        let base_ms = self.config.base_delay.as_millis() as f64;
        let multiplier = self.config.multiplier;
        let max_ms = self.config.max_delay.as_millis() as f64;

        // Calculate exponential delay: base_delay * multiplier^attempt
        let exponential_delay_ms = base_ms * multiplier.powi(attempt as i32);

        // Cap at max_delay
        let capped_delay_ms = exponential_delay_ms.min(max_ms);

        Duration::from_millis(capped_delay_ms as u64)
    }
}

/// WebSocket connection state.
///
/// Uses `#[repr(u8)]` to enable atomic storage via `AtomicU8`.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WsConnectionState {
    /// Not connected
    Disconnected = 0,
    /// Establishing connection
    Connecting = 1,
    /// Successfully connected
    Connected = 2,
    /// Attempting to reconnect
    Reconnecting = 3,
    /// Error state
    Error = 4,
}

impl WsConnectionState {
    /// Converts a `u8` value to `WsConnectionState`.
    ///
    /// # Arguments
    ///
    /// * `value` - The u8 value to convert
    ///
    /// # Returns
    ///
    /// The corresponding `WsConnectionState`, defaulting to `Error` for unknown values.
    #[inline]
    pub fn from_u8(value: u8) -> Self {
        match value {
            0 => Self::Disconnected,
            1 => Self::Connecting,
            2 => Self::Connected,
            3 => Self::Reconnecting,
            _ => Self::Error,
        }
    }

    /// Converts the `WsConnectionState` to its `u8` representation.
    #[inline]
    pub fn as_u8(self) -> u8 {
        self as u8
    }
}

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

/// WebSocket connection configuration.
///
/// This struct contains all configuration options for WebSocket connections,
/// including connection timeouts, reconnection behavior, and resource limits.
///
/// # Example
///
/// ```rust
/// use ccxt_core::ws_client::{WsConfig, BackoffConfig};
/// use std::time::Duration;
///
/// let config = WsConfig {
///     url: "wss://stream.example.com/ws".to_string(),
///     max_subscriptions: 50,
///     backoff_config: BackoffConfig {
///         base_delay: Duration::from_millis(500),
///         max_delay: Duration::from_secs(30),
///         ..Default::default()
///     },
///     ..Default::default()
/// };
/// ```
#[derive(Debug, Clone)]
pub struct WsConfig {
    /// WebSocket server URL
    pub url: String,
    /// Connection timeout in milliseconds
    pub connect_timeout: u64,
    /// Ping interval in milliseconds
    pub ping_interval: u64,
    /// Reconnection delay in milliseconds (legacy, use `backoff_config` for exponential backoff)
    pub reconnect_interval: u64,
    /// Maximum reconnection attempts before giving up
    pub max_reconnect_attempts: u32,
    /// Enable automatic reconnection on disconnect
    pub auto_reconnect: bool,
    /// Enable message compression
    pub enable_compression: bool,
    /// Pong timeout in milliseconds
    ///
    /// Connection is considered dead if no pong received within this duration.
    pub pong_timeout: u64,
    /// Exponential backoff configuration for reconnection.
    ///
    /// This configuration controls how retry delays are calculated during
    /// reconnection attempts. Uses exponential backoff with jitter to prevent
    /// thundering herd effects.
    ///
    /// # Default
    ///
    /// - `base_delay`: 1 second
    /// - `max_delay`: 60 seconds
    /// - `jitter_factor`: 0.25 (25%)
    /// - `multiplier`: 2.0
    pub backoff_config: BackoffConfig,
    /// Maximum number of subscriptions allowed.
    ///
    /// When this limit is reached, new subscription attempts will fail with
    /// `Error::ResourceExhausted`. This prevents resource exhaustion from
    /// too many concurrent subscriptions.
    ///
    /// # Default
    ///
    /// 100 subscriptions (see `DEFAULT_MAX_SUBSCRIPTIONS`)
    pub max_subscriptions: usize,
    /// Shutdown timeout in milliseconds.
    ///
    /// Maximum time to wait for pending operations to complete during
    /// graceful shutdown. After this timeout, the shutdown will proceed
    /// regardless of pending operations.
    ///
    /// # Default
    ///
    /// 5000 milliseconds (5 seconds)
    pub shutdown_timeout: u64,
}

/// Default shutdown timeout in milliseconds.
pub const DEFAULT_SHUTDOWN_TIMEOUT: u64 = 5000;

impl Default for WsConfig {
    fn default() -> Self {
        Self {
            url: String::new(),
            connect_timeout: 10000,
            ping_interval: 30000,
            reconnect_interval: 5000,
            max_reconnect_attempts: 5,
            auto_reconnect: true,
            enable_compression: false,
            pong_timeout: 90000,
            backoff_config: BackoffConfig::default(),
            max_subscriptions: DEFAULT_MAX_SUBSCRIPTIONS,
            shutdown_timeout: DEFAULT_SHUTDOWN_TIMEOUT,
        }
    }
}

/// WebSocket subscription metadata.
#[derive(Debug, Clone)]
pub struct Subscription {
    channel: String,
    symbol: Option<String>,
    params: Option<HashMap<String, Value>>,
}

/// Default maximum number of subscriptions.
pub const DEFAULT_MAX_SUBSCRIPTIONS: usize = 100;

/// Subscription manager with capacity limits.
///
/// This struct manages WebSocket subscriptions with a configurable maximum limit
/// to prevent resource exhaustion. It uses `DashMap` for lock-free concurrent access.
///
/// # Example
///
/// ```rust
/// use ccxt_core::ws_client::SubscriptionManager;
///
/// let manager = SubscriptionManager::new(100);
/// assert_eq!(manager.count(), 0);
/// assert_eq!(manager.remaining_capacity(), 100);
/// assert_eq!(manager.max_subscriptions(), 100);
/// ```
#[derive(Debug)]
pub struct SubscriptionManager {
    /// Active subscriptions (lock-free)
    subscriptions: DashMap<String, Subscription>,
    /// Maximum allowed subscriptions
    max_subscriptions: usize,
}

impl SubscriptionManager {
    /// Creates a new subscription manager with the specified maximum capacity.
    ///
    /// # Arguments
    ///
    /// * `max_subscriptions` - Maximum number of subscriptions allowed
    ///
    /// # Example
    ///
    /// ```rust
    /// use ccxt_core::ws_client::SubscriptionManager;
    ///
    /// let manager = SubscriptionManager::new(50);
    /// assert_eq!(manager.max_subscriptions(), 50);
    /// ```
    pub fn new(max_subscriptions: usize) -> Self {
        Self {
            subscriptions: DashMap::new(),
            max_subscriptions,
        }
    }

    /// Creates a new subscription manager with the default maximum capacity (100).
    ///
    /// # Example
    ///
    /// ```rust
    /// use ccxt_core::ws_client::{SubscriptionManager, DEFAULT_MAX_SUBSCRIPTIONS};
    ///
    /// let manager = SubscriptionManager::with_default_capacity();
    /// assert_eq!(manager.max_subscriptions(), DEFAULT_MAX_SUBSCRIPTIONS);
    /// ```
    pub fn with_default_capacity() -> Self {
        Self::new(DEFAULT_MAX_SUBSCRIPTIONS)
    }

    /// Returns the maximum number of subscriptions allowed.
    ///
    /// # Example
    ///
    /// ```rust
    /// use ccxt_core::ws_client::SubscriptionManager;
    ///
    /// let manager = SubscriptionManager::new(75);
    /// assert_eq!(manager.max_subscriptions(), 75);
    /// ```
    #[inline]
    #[must_use]
    pub fn max_subscriptions(&self) -> usize {
        self.max_subscriptions
    }

    /// Attempts to add a subscription.
    ///
    /// Returns `Ok(())` if the subscription was added successfully, or
    /// `Err(Error::ResourceExhausted)` if the maximum capacity has been reached.
    ///
    /// If a subscription with the same key already exists, it will be replaced
    /// without counting against the capacity limit.
    ///
    /// # Arguments
    ///
    /// * `key` - Unique subscription key (typically "channel:symbol")
    /// * `subscription` - The subscription metadata
    ///
    /// # Errors
    ///
    /// Returns `Error::ResourceExhausted` if the subscription count has reached
    /// the maximum capacity and the key doesn't already exist.
    ///
    /// # Example
    ///
    /// ```rust
    /// use ccxt_core::ws_client::SubscriptionManager;
    ///
    /// let manager = SubscriptionManager::new(2);
    ///
    /// // First two subscriptions succeed
    /// // (Note: try_add requires internal Subscription type, this is conceptual)
    /// assert_eq!(manager.count(), 0);
    /// assert_eq!(manager.remaining_capacity(), 2);
    /// ```
    pub fn try_add(&self, key: String, subscription: Subscription) -> Result<()> {
        // If the key already exists, we're replacing, not adding
        if self.subscriptions.contains_key(&key) {
            self.subscriptions.insert(key, subscription);
            return Ok(());
        }

        // Check capacity before adding new subscription
        if self.subscriptions.len() >= self.max_subscriptions {
            return Err(Error::resource_exhausted(format!(
                "Maximum subscriptions ({}) reached",
                self.max_subscriptions
            )));
        }

        self.subscriptions.insert(key, subscription);
        Ok(())
    }

    /// Removes a subscription by key.
    ///
    /// Returns the removed subscription if it existed, or `None` if not found.
    ///
    /// # Arguments
    ///
    /// * `key` - The subscription key to remove
    ///
    /// # Example
    ///
    /// ```rust
    /// use ccxt_core::ws_client::SubscriptionManager;
    ///
    /// let manager = SubscriptionManager::new(100);
    /// // After adding and removing a subscription:
    /// // let removed = manager.remove("ticker:BTC/USDT");
    /// ```
    pub fn remove(&self, key: &str) -> Option<Subscription> {
        self.subscriptions.remove(key).map(|(_, v)| v)
    }

    /// Returns the current number of active subscriptions.
    ///
    /// This operation is lock-free and thread-safe.
    ///
    /// # Example
    ///
    /// ```rust
    /// use ccxt_core::ws_client::SubscriptionManager;
    ///
    /// let manager = SubscriptionManager::new(100);
    /// assert_eq!(manager.count(), 0);
    /// ```
    #[inline]
    #[must_use]
    pub fn count(&self) -> usize {
        self.subscriptions.len()
    }

    /// Returns the remaining capacity for new subscriptions.
    ///
    /// This is calculated as `max_subscriptions - current_count`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use ccxt_core::ws_client::SubscriptionManager;
    ///
    /// let manager = SubscriptionManager::new(100);
    /// assert_eq!(manager.remaining_capacity(), 100);
    /// ```
    #[inline]
    #[must_use]
    pub fn remaining_capacity(&self) -> usize {
        self.max_subscriptions
            .saturating_sub(self.subscriptions.len())
    }

    /// Checks if a subscription exists for the given key.
    ///
    /// # Arguments
    ///
    /// * `key` - The subscription key to check
    ///
    /// # Example
    ///
    /// ```rust
    /// use ccxt_core::ws_client::SubscriptionManager;
    ///
    /// let manager = SubscriptionManager::new(100);
    /// assert!(!manager.contains("ticker:BTC/USDT"));
    /// ```
    #[inline]
    #[must_use]
    pub fn contains(&self, key: &str) -> bool {
        self.subscriptions.contains_key(key)
    }

    /// Returns a reference to the subscription for the given key, if it exists.
    ///
    /// # Arguments
    ///
    /// * `key` - The subscription key to look up
    #[must_use]
    pub fn get(&self, key: &str) -> Option<dashmap::mapref::one::Ref<'_, String, Subscription>> {
        self.subscriptions.get(key)
    }

    /// Clears all subscriptions.
    ///
    /// This removes all subscriptions from the manager, freeing all capacity.
    ///
    /// # Example
    ///
    /// ```rust
    /// use ccxt_core::ws_client::SubscriptionManager;
    ///
    /// let manager = SubscriptionManager::new(100);
    /// // After adding subscriptions:
    /// manager.clear();
    /// assert_eq!(manager.count(), 0);
    /// assert_eq!(manager.remaining_capacity(), 100);
    /// ```
    pub fn clear(&self) {
        self.subscriptions.clear();
    }

    /// Returns an iterator over all subscriptions.
    ///
    /// The iterator yields `(key, subscription)` pairs.
    pub fn iter(
        &self,
    ) -> impl Iterator<Item = dashmap::mapref::multiple::RefMulti<'_, String, Subscription>> {
        self.subscriptions.iter()
    }

    /// Collects all subscriptions into a vector.
    ///
    /// This is useful when you need to iterate over subscriptions while
    /// potentially modifying the manager.
    ///
    /// # Example
    ///
    /// ```rust
    /// use ccxt_core::ws_client::SubscriptionManager;
    ///
    /// let manager = SubscriptionManager::new(100);
    /// let subs = manager.collect_subscriptions();
    /// assert!(subs.is_empty());
    /// ```
    #[must_use]
    pub fn collect_subscriptions(&self) -> Vec<Subscription> {
        self.subscriptions
            .iter()
            .map(|entry| entry.value().clone())
            .collect()
    }

    /// Checks if the manager is at full capacity.
    ///
    /// # Example
    ///
    /// ```rust
    /// use ccxt_core::ws_client::SubscriptionManager;
    ///
    /// let manager = SubscriptionManager::new(100);
    /// assert!(!manager.is_full());
    /// ```
    #[inline]
    #[must_use]
    pub fn is_full(&self) -> bool {
        self.subscriptions.len() >= self.max_subscriptions
    }

    /// Checks if the manager has no subscriptions.
    ///
    /// # Example
    ///
    /// ```rust
    /// use ccxt_core::ws_client::SubscriptionManager;
    ///
    /// let manager = SubscriptionManager::new(100);
    /// assert!(manager.is_empty());
    /// ```
    #[inline]
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.subscriptions.is_empty()
    }
}

impl Default for SubscriptionManager {
    fn default() -> Self {
        Self::with_default_capacity()
    }
}

/// Type alias for WebSocket write half.
#[allow(dead_code)]
type WsWriter = SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>;

/// Async WebSocket client for exchange streaming APIs.
pub struct WsClient {
    config: WsConfig,
    /// Connection state (atomic for lock-free reads)
    state: Arc<AtomicU8>,
    /// Subscription manager with capacity limits for lock-free concurrent access.
    ///
    /// Uses `SubscriptionManager` to enforce subscription limits and prevent
    /// resource exhaustion. The manager uses `DashMap` internally for lock-free
    /// concurrent access.
    subscription_manager: SubscriptionManager,

    message_tx: mpsc::UnboundedSender<Value>,
    message_rx: Arc<RwLock<mpsc::UnboundedReceiver<Value>>>,

    write_tx: Arc<Mutex<Option<mpsc::UnboundedSender<Message>>>>,

    /// Reconnection counter (atomic for lock-free access)
    reconnect_count: AtomicU32,

    shutdown_tx: Arc<Mutex<Option<mpsc::UnboundedSender<()>>>>,

    /// Connection statistics (lock-free atomic access)
    stats: Arc<WsStats>,

    /// Cancellation token for graceful shutdown and operation cancellation.
    ///
    /// This token can be used to cancel long-running operations like connect,
    /// reconnect, and subscribe. When cancelled, operations will return
    /// `Error::Cancelled`.
    cancel_token: Arc<Mutex<Option<CancellationToken>>>,

    /// Optional event callback for connection lifecycle events.
    ///
    /// This callback is invoked for events like `Shutdown`, `Connected`, etc.
    /// The callback is stored as an `Arc` to allow sharing across tasks.
    event_callback: Arc<Mutex<Option<WsEventCallback>>>,
}

/// WebSocket connection statistics (lock-free).
///
/// This struct uses atomic types for all fields to enable lock-free concurrent access.
/// This prevents potential deadlocks when stats are accessed across await points.
///
/// # Thread Safety
///
/// All operations on `WsStats` are thread-safe and lock-free. Multiple tasks can
/// read and update statistics concurrently without blocking.
///
/// # Example
///
/// ```rust
/// use ccxt_core::ws_client::WsStats;
///
/// let stats = WsStats::new();
///
/// // Record a received message
/// stats.record_received(1024);
///
/// // Get a snapshot of current stats
/// let snapshot = stats.snapshot();
/// assert_eq!(snapshot.messages_received, 1);
/// assert_eq!(snapshot.bytes_received, 1024);
/// ```
#[derive(Debug)]
pub struct WsStats {
    /// Total messages received
    messages_received: AtomicU64,
    /// Total messages sent
    messages_sent: AtomicU64,
    /// Total bytes received
    bytes_received: AtomicU64,
    /// Total bytes sent
    bytes_sent: AtomicU64,
    /// Last message timestamp in milliseconds
    last_message_time: AtomicI64,
    /// Last ping timestamp in milliseconds
    last_ping_time: AtomicI64,
    /// Last pong timestamp in milliseconds
    last_pong_time: AtomicI64,
    /// Connection established timestamp in milliseconds
    connected_at: AtomicI64,
    /// Number of reconnection attempts
    reconnect_attempts: AtomicU32,
}

impl WsStats {
    /// Creates a new `WsStats` instance with all counters initialized to zero.
    ///
    /// # Example
    ///
    /// ```rust
    /// use ccxt_core::ws_client::WsStats;
    ///
    /// let stats = WsStats::new();
    /// let snapshot = stats.snapshot();
    /// assert_eq!(snapshot.messages_received, 0);
    /// ```
    pub fn new() -> Self {
        Self {
            messages_received: AtomicU64::new(0),
            messages_sent: AtomicU64::new(0),
            bytes_received: AtomicU64::new(0),
            bytes_sent: AtomicU64::new(0),
            last_message_time: AtomicI64::new(0),
            last_ping_time: AtomicI64::new(0),
            last_pong_time: AtomicI64::new(0),
            connected_at: AtomicI64::new(0),
            reconnect_attempts: AtomicU32::new(0),
        }
    }

    /// Records a received message.
    ///
    /// Increments the message count, adds to bytes received, and updates
    /// the last message timestamp.
    ///
    /// # Arguments
    ///
    /// * `bytes` - Number of bytes received in the message
    ///
    /// # Example
    ///
    /// ```rust
    /// use ccxt_core::ws_client::WsStats;
    ///
    /// let stats = WsStats::new();
    /// stats.record_received(512);
    /// stats.record_received(256);
    ///
    /// let snapshot = stats.snapshot();
    /// assert_eq!(snapshot.messages_received, 2);
    /// assert_eq!(snapshot.bytes_received, 768);
    /// ```
    pub fn record_received(&self, bytes: u64) {
        self.messages_received.fetch_add(1, Ordering::Relaxed);
        self.bytes_received.fetch_add(bytes, Ordering::Relaxed);
        self.last_message_time
            .store(chrono::Utc::now().timestamp_millis(), Ordering::Relaxed);
    }

    /// Records a sent message.
    ///
    /// Increments the message count and adds to bytes sent.
    ///
    /// # Arguments
    ///
    /// * `bytes` - Number of bytes sent in the message
    ///
    /// # Example
    ///
    /// ```rust
    /// use ccxt_core::ws_client::WsStats;
    ///
    /// let stats = WsStats::new();
    /// stats.record_sent(128);
    ///
    /// let snapshot = stats.snapshot();
    /// assert_eq!(snapshot.messages_sent, 1);
    /// assert_eq!(snapshot.bytes_sent, 128);
    /// ```
    pub fn record_sent(&self, bytes: u64) {
        self.messages_sent.fetch_add(1, Ordering::Relaxed);
        self.bytes_sent.fetch_add(bytes, Ordering::Relaxed);
    }

    /// Records a ping sent.
    ///
    /// Updates the last ping timestamp.
    ///
    /// # Example
    ///
    /// ```rust
    /// use ccxt_core::ws_client::WsStats;
    ///
    /// let stats = WsStats::new();
    /// stats.record_ping();
    ///
    /// let snapshot = stats.snapshot();
    /// assert!(snapshot.last_ping_time > 0);
    /// ```
    pub fn record_ping(&self) {
        self.last_ping_time
            .store(chrono::Utc::now().timestamp_millis(), Ordering::Relaxed);
    }

    /// Records a pong received.
    ///
    /// Updates the last pong timestamp.
    ///
    /// # Example
    ///
    /// ```rust
    /// use ccxt_core::ws_client::WsStats;
    ///
    /// let stats = WsStats::new();
    /// stats.record_pong();
    ///
    /// let snapshot = stats.snapshot();
    /// assert!(snapshot.last_pong_time > 0);
    /// ```
    pub fn record_pong(&self) {
        self.last_pong_time
            .store(chrono::Utc::now().timestamp_millis(), Ordering::Relaxed);
    }

    /// Records a connection established.
    ///
    /// Updates the connected_at timestamp.
    ///
    /// # Example
    ///
    /// ```rust
    /// use ccxt_core::ws_client::WsStats;
    ///
    /// let stats = WsStats::new();
    /// stats.record_connected();
    ///
    /// let snapshot = stats.snapshot();
    /// assert!(snapshot.connected_at > 0);
    /// ```
    pub fn record_connected(&self) {
        self.connected_at
            .store(chrono::Utc::now().timestamp_millis(), Ordering::Relaxed);
    }

    /// Increments the reconnection attempt counter.
    ///
    /// # Returns
    ///
    /// The new reconnection attempt count after incrementing.
    ///
    /// # Example
    ///
    /// ```rust
    /// use ccxt_core::ws_client::WsStats;
    ///
    /// let stats = WsStats::new();
    /// let count = stats.increment_reconnect_attempts();
    /// assert_eq!(count, 1);
    ///
    /// let count = stats.increment_reconnect_attempts();
    /// assert_eq!(count, 2);
    /// ```
    pub fn increment_reconnect_attempts(&self) -> u32 {
        self.reconnect_attempts.fetch_add(1, Ordering::Relaxed) + 1
    }

    /// Resets the reconnection attempt counter to zero.
    ///
    /// # Example
    ///
    /// ```rust
    /// use ccxt_core::ws_client::WsStats;
    ///
    /// let stats = WsStats::new();
    /// stats.increment_reconnect_attempts();
    /// stats.increment_reconnect_attempts();
    ///
    /// stats.reset_reconnect_attempts();
    ///
    /// let snapshot = stats.snapshot();
    /// assert_eq!(snapshot.reconnect_attempts, 0);
    /// ```
    pub fn reset_reconnect_attempts(&self) {
        self.reconnect_attempts.store(0, Ordering::Relaxed);
    }

    /// Returns the last pong timestamp.
    ///
    /// This is useful for checking connection health without creating a full snapshot.
    ///
    /// # Returns
    ///
    /// The last pong timestamp in milliseconds, or 0 if no pong has been received.
    ///
    /// # Example
    ///
    /// ```rust
    /// use ccxt_core::ws_client::WsStats;
    ///
    /// let stats = WsStats::new();
    /// assert_eq!(stats.last_pong_time(), 0);
    ///
    /// stats.record_pong();
    /// assert!(stats.last_pong_time() > 0);
    /// ```
    pub fn last_pong_time(&self) -> i64 {
        self.last_pong_time.load(Ordering::Relaxed)
    }

    /// Returns the last ping timestamp.
    ///
    /// This is useful for calculating latency without creating a full snapshot.
    ///
    /// # Returns
    ///
    /// The last ping timestamp in milliseconds, or 0 if no ping has been sent.
    pub fn last_ping_time(&self) -> i64 {
        self.last_ping_time.load(Ordering::Relaxed)
    }

    /// Creates an immutable snapshot of current statistics.
    ///
    /// The snapshot captures all statistics at a point in time. Note that since
    /// each field is read independently, the snapshot may not represent a perfectly
    /// consistent state if updates are happening concurrently. However, this is
    /// acceptable for statistics purposes.
    ///
    /// # Returns
    ///
    /// A `WsStatsSnapshot` containing the current values of all statistics.
    ///
    /// # Example
    ///
    /// ```rust
    /// use ccxt_core::ws_client::WsStats;
    ///
    /// let stats = WsStats::new();
    /// stats.record_received(100);
    /// stats.record_sent(50);
    ///
    /// let snapshot = stats.snapshot();
    /// assert_eq!(snapshot.messages_received, 1);
    /// assert_eq!(snapshot.bytes_received, 100);
    /// assert_eq!(snapshot.messages_sent, 1);
    /// assert_eq!(snapshot.bytes_sent, 50);
    /// ```
    pub fn snapshot(&self) -> WsStatsSnapshot {
        WsStatsSnapshot {
            messages_received: self.messages_received.load(Ordering::Relaxed),
            messages_sent: self.messages_sent.load(Ordering::Relaxed),
            bytes_received: self.bytes_received.load(Ordering::Relaxed),
            bytes_sent: self.bytes_sent.load(Ordering::Relaxed),
            last_message_time: self.last_message_time.load(Ordering::Relaxed),
            last_ping_time: self.last_ping_time.load(Ordering::Relaxed),
            last_pong_time: self.last_pong_time.load(Ordering::Relaxed),
            connected_at: self.connected_at.load(Ordering::Relaxed),
            reconnect_attempts: self.reconnect_attempts.load(Ordering::Relaxed),
        }
    }

    /// Resets all statistics to their default values.
    ///
    /// # Example
    ///
    /// ```rust
    /// use ccxt_core::ws_client::WsStats;
    ///
    /// let stats = WsStats::new();
    /// stats.record_received(100);
    /// stats.record_sent(50);
    ///
    /// stats.reset();
    ///
    /// let snapshot = stats.snapshot();
    /// assert_eq!(snapshot.messages_received, 0);
    /// assert_eq!(snapshot.bytes_received, 0);
    /// ```
    pub fn reset(&self) {
        self.messages_received.store(0, Ordering::Relaxed);
        self.messages_sent.store(0, Ordering::Relaxed);
        self.bytes_received.store(0, Ordering::Relaxed);
        self.bytes_sent.store(0, Ordering::Relaxed);
        self.last_message_time.store(0, Ordering::Relaxed);
        self.last_ping_time.store(0, Ordering::Relaxed);
        self.last_pong_time.store(0, Ordering::Relaxed);
        self.connected_at.store(0, Ordering::Relaxed);
        self.reconnect_attempts.store(0, Ordering::Relaxed);
    }
}

impl Default for WsStats {
    fn default() -> Self {
        Self::new()
    }
}

/// Immutable snapshot of WebSocket connection statistics.
///
/// This struct provides a point-in-time view of the connection statistics.
/// It is created by calling `WsStats::snapshot()` and can be safely cloned
/// and passed around without affecting the underlying atomic counters.
///
/// # Example
///
/// ```rust
/// use ccxt_core::ws_client::WsStats;
///
/// let stats = WsStats::new();
/// stats.record_received(1024);
///
/// let snapshot = stats.snapshot();
/// let snapshot_clone = snapshot.clone();
///
/// assert_eq!(snapshot.messages_received, snapshot_clone.messages_received);
/// ```
#[derive(Debug, Clone, Default)]
pub struct WsStatsSnapshot {
    /// Total messages received
    pub messages_received: u64,
    /// Total messages sent
    pub messages_sent: u64,
    /// Total bytes received
    pub bytes_received: u64,
    /// Total bytes sent
    pub bytes_sent: u64,
    /// Last message timestamp in milliseconds
    pub last_message_time: i64,
    /// Last ping timestamp in milliseconds
    pub last_ping_time: i64,
    /// Last pong timestamp in milliseconds
    pub last_pong_time: i64,
    /// Connection established timestamp in milliseconds
    pub connected_at: i64,
    /// Number of reconnection attempts
    pub reconnect_attempts: u32,
}

impl WsClient {
    /// Creates a new WebSocket client instance.
    ///
    /// # Arguments
    ///
    /// * `config` - WebSocket connection configuration
    ///
    /// # Returns
    ///
    /// A new `WsClient` instance ready to connect
    pub fn new(config: WsConfig) -> Self {
        let (message_tx, message_rx) = mpsc::unbounded_channel();
        let max_subscriptions = config.max_subscriptions;

        Self {
            config,
            state: Arc::new(AtomicU8::new(WsConnectionState::Disconnected.as_u8())),
            subscription_manager: SubscriptionManager::new(max_subscriptions),
            message_tx,
            message_rx: Arc::new(RwLock::new(message_rx)),
            write_tx: Arc::new(Mutex::new(None)),
            reconnect_count: AtomicU32::new(0),
            shutdown_tx: Arc::new(Mutex::new(None)),
            stats: Arc::new(WsStats::new()),
            cancel_token: Arc::new(Mutex::new(None)),
            event_callback: Arc::new(Mutex::new(None)),
        }
    }

    /// Sets the event callback for connection lifecycle events.
    ///
    /// The callback will be invoked for events like `Shutdown`, `Connected`, etc.
    /// This allows the application to react to connection state changes.
    ///
    /// # Arguments
    ///
    /// * `callback` - The event callback function
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use ccxt_core::ws_client::{WsClient, WsConfig, WsEvent};
    /// use std::sync::Arc;
    ///
    /// let client = WsClient::new(WsConfig::default());
    ///
    /// client.set_event_callback(Arc::new(|event| {
    ///     match event {
    ///         WsEvent::Shutdown => println!("Client shutdown"),
    ///         WsEvent::Connected => println!("Client connected"),
    ///         _ => {}
    ///     }
    /// })).await;
    /// ```
    pub async fn set_event_callback(&self, callback: WsEventCallback) {
        *self.event_callback.lock().await = Some(callback);
        debug!("Event callback set");
    }

    /// Clears the event callback.
    ///
    /// After calling this method, no events will be emitted to the callback.
    pub async fn clear_event_callback(&self) {
        *self.event_callback.lock().await = None;
        debug!("Event callback cleared");
    }

    /// Emits an event to the registered callback (if any).
    ///
    /// This method is used internally to notify the application about
    /// connection lifecycle events.
    ///
    /// # Arguments
    ///
    /// * `event` - The event to emit
    async fn emit_event(&self, event: WsEvent) {
        let callback = self.event_callback.lock().await;
        if let Some(ref cb) = *callback {
            // Clone the callback to release the lock before invoking
            let cb = Arc::clone(cb);
            drop(callback);
            // Invoke callback asynchronously to avoid blocking
            tokio::spawn(async move {
                cb(event);
            });
        }
    }

    /// Sets the cancellation token for this client.
    ///
    /// The cancellation token can be used to cancel long-running operations
    /// like connect, reconnect, and subscribe. When the token is cancelled,
    /// these operations will return `Error::Cancelled`.
    ///
    /// # Arguments
    ///
    /// * `token` - The cancellation token to use
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use ccxt_core::ws_client::{WsClient, WsConfig};
    /// use tokio_util::sync::CancellationToken;
    ///
    /// let client = WsClient::new(WsConfig::default());
    /// let token = CancellationToken::new();
    ///
    /// // Set the cancellation token
    /// client.set_cancel_token(token.clone()).await;
    ///
    /// // Later, cancel all operations
    /// token.cancel();
    /// ```
    pub async fn set_cancel_token(&self, token: CancellationToken) {
        *self.cancel_token.lock().await = Some(token);
        debug!("Cancellation token set");
    }

    /// Clears the cancellation token.
    ///
    /// After calling this method, operations will no longer be cancellable
    /// via the previously set token.
    pub async fn clear_cancel_token(&self) {
        *self.cancel_token.lock().await = None;
        debug!("Cancellation token cleared");
    }

    /// Returns a clone of the current cancellation token, if set.
    ///
    /// This is useful for sharing the token with other components or
    /// for checking if a token is currently set.
    ///
    /// # Returns
    ///
    /// `Some(CancellationToken)` if a token is set, `None` otherwise.
    pub async fn get_cancel_token(&self) -> Option<CancellationToken> {
        self.cancel_token.lock().await.clone()
    }

    /// Establishes connection to the WebSocket server.
    ///
    /// Returns immediately if already connected. Automatically starts message
    /// processing loop and resubscribes to previous channels on success.
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Connection timeout exceeded
    /// - Network error occurs
    /// - Server rejects connection
    #[instrument(
        name = "ws_connect",
        skip(self),
        fields(url = %self.config.url, timeout_ms = self.config.connect_timeout)
    )]
    pub async fn connect(&self) -> Result<()> {
        // Lock-free state check
        if self.state() == WsConnectionState::Connected {
            info!("WebSocket already connected");
            return Ok(());
        }

        // Lock-free state update
        self.set_state(WsConnectionState::Connecting);

        let url = self.config.url.clone();
        info!("Initiating WebSocket connection");

        match tokio::time::timeout(
            Duration::from_millis(self.config.connect_timeout),
            connect_async(&url),
        )
        .await
        {
            Ok(Ok((ws_stream, response))) => {
                info!(
                    status = response.status().as_u16(),
                    "WebSocket connection established successfully"
                );

                self.set_state(WsConnectionState::Connected);
                // Lock-free reconnect count reset
                self.reconnect_count.store(0, Ordering::Release);

                // Lock-free stats update
                self.stats.record_connected();

                self.start_message_loop(ws_stream).await;

                self.resubscribe_all().await?;

                Ok(())
            }
            Ok(Err(e)) => {
                error!(
                    error = %e,
                    error_debug = ?e,
                    "WebSocket connection failed"
                );
                self.set_state(WsConnectionState::Error);
                Err(Error::network(format!("WebSocket connection failed: {e}")))
            }
            Err(_) => {
                error!(
                    timeout_ms = self.config.connect_timeout,
                    "WebSocket connection timeout exceeded"
                );
                self.set_state(WsConnectionState::Error);
                Err(Error::timeout("WebSocket connection timeout"))
            }
        }
    }

    /// Establishes connection to the WebSocket server with cancellation support.
    ///
    /// This method is similar to [`connect`](Self::connect), but accepts an optional
    /// `CancellationToken` that can be used to cancel the connection attempt.
    ///
    /// If no token is provided, the method will use the client's internal token
    /// (if set via [`set_cancel_token`](Self::set_cancel_token)).
    ///
    /// # Arguments
    ///
    /// * `cancel_token` - Optional cancellation token. If `None`, uses the client's
    ///   internal token (if set).
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Connection timeout exceeded
    /// - Network error occurs
    /// - Server rejects connection
    /// - Operation was cancelled via the cancellation token
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use ccxt_core::ws_client::{WsClient, WsConfig};
    /// use tokio_util::sync::CancellationToken;
    ///
    /// let client = WsClient::new(WsConfig {
    ///     url: "wss://stream.example.com/ws".to_string(),
    ///     ..Default::default()
    /// });
    ///
    /// let token = CancellationToken::new();
    /// let token_clone = token.clone();
    ///
    /// // Spawn a task to cancel after 5 seconds
    /// tokio::spawn(async move {
    ///     tokio::time::sleep(Duration::from_secs(5)).await;
    ///     token_clone.cancel();
    /// });
    ///
    /// // Connect with cancellation support
    /// match client.connect_with_cancel(Some(token)).await {
    ///     Ok(()) => println!("Connected!"),
    ///     Err(e) if e.as_cancelled().is_some() => println!("Connection cancelled"),
    ///     Err(e) => println!("Connection failed: {}", e),
    /// }
    /// ```
    #[instrument(
        name = "ws_connect_with_cancel",
        skip(self, cancel_token),
        fields(url = %self.config.url, timeout_ms = self.config.connect_timeout)
    )]
    pub async fn connect_with_cancel(&self, cancel_token: Option<CancellationToken>) -> Result<()> {
        // Use provided token, or fall back to client's internal token, or create a new one
        let token = if let Some(t) = cancel_token {
            t
        } else {
            let internal_token = self.cancel_token.lock().await;
            internal_token
                .clone()
                .unwrap_or_else(CancellationToken::new)
        };

        // Lock-free state check
        if self.state() == WsConnectionState::Connected {
            info!("WebSocket already connected");
            return Ok(());
        }

        // Lock-free state update
        self.set_state(WsConnectionState::Connecting);

        let url = self.config.url.clone();
        info!("Initiating WebSocket connection with cancellation support");

        // Use tokio::select! to race between connection and cancellation
        tokio::select! {
            biased;

            // Check for cancellation first
            () = token.cancelled() => {
                warn!("WebSocket connection cancelled");
                self.set_state(WsConnectionState::Disconnected);
                Err(Error::cancelled("WebSocket connection cancelled"))
            }

            // Attempt connection with timeout
            result = tokio::time::timeout(
                Duration::from_millis(self.config.connect_timeout),
                connect_async(&url),
            ) => {
                match result {
                    Ok(Ok((ws_stream, response))) => {
                        info!(
                            status = response.status().as_u16(),
                            "WebSocket connection established successfully"
                        );

                        self.set_state(WsConnectionState::Connected);
                        // Lock-free reconnect count reset
                        self.reconnect_count.store(0, Ordering::Release);

                        // Lock-free stats update
                        self.stats.record_connected();

                        self.start_message_loop(ws_stream).await;

                        self.resubscribe_all().await?;

                        Ok(())
                    }
                    Ok(Err(e)) => {
                        error!(
                            error = %e,
                            error_debug = ?e,
                            "WebSocket connection failed"
                        );
                        self.set_state(WsConnectionState::Error);
                        Err(Error::network(format!("WebSocket connection failed: {e}")))
                    }
                    Err(_) => {
                        error!(
                            timeout_ms = self.config.connect_timeout,
                            "WebSocket connection timeout exceeded"
                        );
                        self.set_state(WsConnectionState::Error);
                        Err(Error::timeout("WebSocket connection timeout"))
                    }
                }
            }
        }
    }

    /// Closes the WebSocket connection gracefully.
    ///
    /// Sends shutdown signal to background tasks and clears internal state.
    #[instrument(name = "ws_disconnect", skip(self))]
    pub async fn disconnect(&self) -> Result<()> {
        info!("Initiating WebSocket disconnect");

        if let Some(tx) = self.shutdown_tx.lock().await.as_ref() {
            let _ = tx.send(());
            debug!("Shutdown signal sent to background tasks");
        }

        *self.write_tx.lock().await = None;

        // Lock-free state update
        self.set_state(WsConnectionState::Disconnected);

        info!("WebSocket disconnected successfully");
        Ok(())
    }

    /// Gracefully shuts down the WebSocket client.
    ///
    /// This method performs a complete shutdown of the WebSocket client:
    /// 1. Cancels all pending reconnection attempts
    /// 2. Sends WebSocket close frame to the server
    /// 3. Waits for pending operations to complete (with timeout)
    /// 4. Clears all resources (subscriptions, channels, etc.)
    /// 5. Emits a `Shutdown` event
    ///
    /// # Behavior
    ///
    /// - If a cancellation token is set, it will be cancelled to stop any
    ///   in-progress reconnection attempts.
    /// - The method waits up to `shutdown_timeout` milliseconds for pending
    ///   operations to complete before proceeding with cleanup.
    /// - All subscriptions are cleared during shutdown.
    /// - The connection state is set to `Disconnected`.
    ///
    /// # Errors
    ///
    /// This method does not return errors. All cleanup operations are
    /// performed on a best-effort basis.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use ccxt_core::ws_client::{WsClient, WsConfig, WsEvent};
    /// use std::sync::Arc;
    ///
    /// let client = WsClient::new(WsConfig {
    ///     url: "wss://stream.example.com/ws".to_string(),
    ///     shutdown_timeout: 5000, // 5 seconds
    ///     ..Default::default()
    /// });
    ///
    /// // Set up event callback to know when shutdown completes
    /// client.set_event_callback(Arc::new(|event| {
    ///     if let WsEvent::Shutdown = event {
    ///         println!("Shutdown completed");
    ///     }
    /// })).await;
    ///
    /// // Connect and do work...
    /// client.connect().await?;
    ///
    /// // Gracefully shutdown
    /// client.shutdown().await;
    /// ```
    #[instrument(name = "ws_shutdown", skip(self), fields(timeout_ms = self.config.shutdown_timeout))]
    pub async fn shutdown(&self) {
        info!("Initiating graceful shutdown");

        // 1. Cancel any pending reconnection attempts (Requirements 7.2)
        {
            let token_guard = self.cancel_token.lock().await;
            if let Some(ref token) = *token_guard {
                info!("Cancelling pending reconnection attempts");
                token.cancel();
            }
        }

        // 2. Set state to disconnected to prevent new operations
        self.set_state(WsConnectionState::Disconnected);

        // 3. Send WebSocket close frame (Requirements 7.3)
        {
            let write_tx_guard = self.write_tx.lock().await;
            if let Some(ref tx) = *write_tx_guard {
                info!("Sending WebSocket close frame");
                // Send close frame - ignore errors as connection may already be closed
                let _ = tx.send(Message::Close(None));
            }
        }

        // 4. Wait for pending operations to complete with timeout (Requirements 7.4)
        let shutdown_timeout = Duration::from_millis(self.config.shutdown_timeout);
        let shutdown_result = tokio::time::timeout(shutdown_timeout, async {
            // Signal background tasks to stop
            if let Some(tx) = self.shutdown_tx.lock().await.as_ref() {
                let _ = tx.send(());
                debug!("Shutdown signal sent to background tasks");
            }

            // Give tasks a moment to process the shutdown signal
            tokio::time::sleep(Duration::from_millis(100)).await;
        })
        .await;

        match shutdown_result {
            Ok(()) => {
                debug!("Pending operations completed within timeout");
            }
            Err(_) => {
                warn!(
                    timeout_ms = self.config.shutdown_timeout,
                    "Shutdown timeout exceeded, proceeding with cleanup"
                );
            }
        }

        // 5. Clear resources (Requirements 7.1)
        {
            // Clear write channel
            *self.write_tx.lock().await = None;

            // Clear shutdown channel
            *self.shutdown_tx.lock().await = None;

            // Clear subscriptions using SubscriptionManager
            self.subscription_manager.clear();
            debug!("Subscriptions cleared");

            // Reset reconnect count
            self.reconnect_count.store(0, Ordering::Release);

            // Reset stats
            self.stats.reset();
            debug!("Statistics reset");
        }

        info!("Shutdown cleanup completed");

        // 6. Emit Shutdown event (Requirements 7.5)
        self.emit_event(WsEvent::Shutdown).await;

        info!("Graceful shutdown completed");
    }

    /// Attempts to reconnect to the WebSocket server.
    ///
    /// Respects `max_reconnect_attempts` configuration and waits for
    /// `reconnect_interval` before attempting connection.
    ///
    /// # Errors
    ///
    /// Returns error if maximum reconnection attempts exceeded or connection fails.
    #[instrument(
        name = "ws_reconnect",
        skip(self),
        fields(
            max_attempts = self.config.max_reconnect_attempts,
            reconnect_interval_ms = self.config.reconnect_interval
        )
    )]
    pub async fn reconnect(&self) -> Result<()> {
        // Lock-free atomic increment and check
        let count = self.reconnect_count.fetch_add(1, Ordering::AcqRel) + 1;

        if count > self.config.max_reconnect_attempts {
            error!(
                attempts = count,
                max = self.config.max_reconnect_attempts,
                "Max reconnect attempts reached, giving up"
            );
            return Err(Error::network("Max reconnect attempts reached"));
        }

        warn!(
            attempt = count,
            max = self.config.max_reconnect_attempts,
            delay_ms = self.config.reconnect_interval,
            "Attempting WebSocket reconnection"
        );

        // Lock-free state update
        self.set_state(WsConnectionState::Reconnecting);

        tokio::time::sleep(Duration::from_millis(self.config.reconnect_interval)).await;

        self.connect().await
    }

    /// Attempts to reconnect to the WebSocket server with cancellation support.
    ///
    /// This method uses exponential backoff for retry delays and classifies errors
    /// to determine if reconnection should continue. It supports cancellation via
    /// an optional `CancellationToken`.
    ///
    /// # Arguments
    ///
    /// * `cancel_token` - Optional cancellation token. If `None`, uses the client's
    ///   internal token (if set via [`set_cancel_token`](Self::set_cancel_token)).
    ///
    /// # Behavior
    ///
    /// 1. Calculates retry delay using exponential backoff strategy
    /// 2. Waits for the calculated delay (can be cancelled)
    /// 3. Attempts to connect (can be cancelled)
    /// 4. On success: resets reconnect counter and returns `Ok(())`
    /// 5. On transient error: continues retry loop
    /// 6. On permanent error: stops retrying and returns error
    /// 7. On max attempts reached: returns error
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Maximum reconnection attempts exceeded
    /// - Permanent error occurs (authentication failure, protocol error)
    /// - Operation was cancelled via the cancellation token
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use ccxt_core::ws_client::{WsClient, WsConfig};
    /// use tokio_util::sync::CancellationToken;
    ///
    /// let client = WsClient::new(WsConfig {
    ///     url: "wss://stream.example.com/ws".to_string(),
    ///     max_reconnect_attempts: 5,
    ///     ..Default::default()
    /// });
    ///
    /// let token = CancellationToken::new();
    ///
    /// // Reconnect with cancellation support
    /// match client.reconnect_with_cancel(Some(token)).await {
    ///     Ok(()) => println!("Reconnected!"),
    ///     Err(e) if e.as_cancelled().is_some() => println!("Reconnection cancelled"),
    ///     Err(e) => println!("Reconnection failed: {}", e),
    /// }
    /// ```
    #[instrument(
        name = "ws_reconnect_with_cancel",
        skip(self, cancel_token),
        fields(
            max_attempts = self.config.max_reconnect_attempts,
        )
    )]
    pub async fn reconnect_with_cancel(
        &self,
        cancel_token: Option<CancellationToken>,
    ) -> Result<()> {
        // Use provided token, or fall back to client's internal token, or create a new one
        let token = if let Some(t) = cancel_token {
            t
        } else {
            let internal_token = self.cancel_token.lock().await;
            internal_token
                .clone()
                .unwrap_or_else(CancellationToken::new)
        };

        // Create backoff strategy from config
        let backoff = BackoffStrategy::new(self.config.backoff_config.clone());

        // Lock-free state update
        self.set_state(WsConnectionState::Reconnecting);

        loop {
            // Check for cancellation before each attempt
            if token.is_cancelled() {
                warn!("Reconnection cancelled before attempt");
                self.set_state(WsConnectionState::Disconnected);
                return Err(Error::cancelled("Reconnection cancelled"));
            }

            // Lock-free atomic increment and check
            let attempt = self.reconnect_count.fetch_add(1, Ordering::AcqRel);

            if attempt >= self.config.max_reconnect_attempts {
                error!(
                    attempts = attempt + 1,
                    max = self.config.max_reconnect_attempts,
                    "Max reconnect attempts reached, giving up"
                );
                self.set_state(WsConnectionState::Error);
                return Err(Error::network(format!(
                    "Max reconnect attempts ({}) reached",
                    self.config.max_reconnect_attempts
                )));
            }

            // Calculate delay using backoff strategy
            let delay = backoff.calculate_delay(attempt);

            #[allow(clippy::cast_possible_truncation)]
            {
                info!(
                    attempt = attempt + 1,
                    max = self.config.max_reconnect_attempts,
                    delay_ms = delay.as_millis() as u64,
                    "Attempting WebSocket reconnection with exponential backoff"
                );
            }

            // Wait for delay with cancellation support
            tokio::select! {
                biased;

                () = token.cancelled() => {
                    warn!("Reconnection cancelled during backoff delay");
                    self.set_state(WsConnectionState::Disconnected);
                    return Err(Error::cancelled("Reconnection cancelled during backoff"));
                }

                () = tokio::time::sleep(delay) => {
                    // Delay completed, proceed with connection attempt
                }
            }

            // Attempt connection with cancellation support
            match self.connect_with_cancel(Some(token.clone())).await {
                Ok(()) => {
                    info!(attempt = attempt + 1, "Reconnection successful");
                    // Reset reconnect count on success
                    self.reconnect_count.store(0, Ordering::Release);
                    return Ok(());
                }
                Err(e) => {
                    // Check if cancelled
                    if e.as_cancelled().is_some() {
                        warn!("Reconnection cancelled during connection attempt");
                        self.set_state(WsConnectionState::Disconnected);
                        return Err(e);
                    }

                    // Classify the error
                    let ws_error = WsError::from_error(&e);

                    if ws_error.is_permanent() {
                        error!(
                            attempt = attempt + 1,
                            error = %e,
                            "Permanent error during reconnection, stopping retry"
                        );
                        self.set_state(WsConnectionState::Error);
                        return Err(e);
                    }

                    // Transient error - log and continue retry loop
                    warn!(
                        attempt = attempt + 1,
                        error = %e,
                        "Transient error during reconnection, will retry"
                    );
                    // Continue to next iteration
                }
            }
        }
    }

    /// Returns the current reconnection attempt count (lock-free).
    #[inline]
    pub fn reconnect_count(&self) -> u32 {
        self.reconnect_count.load(Ordering::Acquire)
    }

    /// Resets the reconnection attempt counter to zero (lock-free).
    pub fn reset_reconnect_count(&self) {
        self.reconnect_count.store(0, Ordering::Release);
        debug!("Reconnect count reset");
    }

    /// Returns a snapshot of connection statistics.
    ///
    /// This method is lock-free and can be called from any context without
    /// blocking or risking deadlocks.
    ///
    /// # Returns
    ///
    /// A `WsStatsSnapshot` containing the current values of all statistics.
    pub fn stats(&self) -> WsStatsSnapshot {
        self.stats.snapshot()
    }

    /// Resets all connection statistics to default values.
    ///
    /// This method is lock-free and can be called from any context.
    pub fn reset_stats(&self) {
        self.stats.reset();
        debug!("Stats reset");
    }

    /// Calculates current connection latency in milliseconds.
    ///
    /// This method is lock-free and can be called from any context.
    ///
    /// # Returns
    ///
    /// Time difference between last pong and ping, or `None` if no data available.
    pub fn latency(&self) -> Option<i64> {
        let last_pong = self.stats.last_pong_time();
        let last_ping = self.stats.last_ping_time();
        if last_pong > 0 && last_ping > 0 {
            Some(last_pong - last_ping)
        } else {
            None
        }
    }

    /// Creates an automatic reconnection coordinator.
    ///
    /// # Returns
    ///
    /// A new [`AutoReconnectCoordinator`] instance for managing reconnection logic.
    pub fn create_auto_reconnect_coordinator(self: Arc<Self>) -> AutoReconnectCoordinator {
        AutoReconnectCoordinator::new(self)
    }

    /// Subscribes to a WebSocket channel.
    ///
    /// Subscription is persisted and automatically reestablished on reconnection.
    /// The subscription count is limited by `max_subscriptions` in `WsConfig`.
    ///
    /// # Arguments
    ///
    /// * `channel` - Channel name to subscribe to
    /// * `symbol` - Optional trading pair symbol
    /// * `params` - Optional additional subscription parameters
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Maximum subscription limit is reached (`Error::ResourceExhausted`)
    /// - Subscription message fails to send
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use ccxt_core::ws_client::{WsClient, WsConfig};
    ///
    /// let client = WsClient::new(WsConfig {
    ///     url: "wss://stream.example.com/ws".to_string(),
    ///     max_subscriptions: 50,
    ///     ..Default::default()
    /// });
    ///
    /// // Subscribe to a channel
    /// client.subscribe("ticker".to_string(), Some("BTC/USDT".to_string()), None).await?;
    ///
    /// // Check remaining capacity
    /// println!("Remaining capacity: {}", client.remaining_capacity());
    /// ```
    #[instrument(
        name = "ws_subscribe",
        skip(self, params),
        fields(channel = %channel, symbol = ?symbol)
    )]
    pub async fn subscribe(
        &self,
        channel: String,
        symbol: Option<String>,
        params: Option<HashMap<String, Value>>,
    ) -> Result<()> {
        let sub_key = Self::subscription_key(&channel, symbol.as_ref());
        let subscription = Subscription {
            channel: channel.clone(),
            symbol: symbol.clone(),
            params: params.clone(),
        };

        // Use SubscriptionManager to enforce capacity limits (Requirements 4.2)
        self.subscription_manager
            .try_add(sub_key.clone(), subscription)?;

        info!(subscription_key = %sub_key, "Subscription registered");

        // Lock-free state check
        let state = self.state();
        if state == WsConnectionState::Connected {
            self.send_subscribe_message(channel, symbol, params).await?;
            info!(subscription_key = %sub_key, "Subscription message sent");
        } else {
            debug!(
                subscription_key = %sub_key,
                state = ?state,
                "Subscription queued (not connected)"
            );
        }

        Ok(())
    }

    /// Unsubscribes from a WebSocket channel.
    ///
    /// Removes subscription from internal state and sends unsubscribe message if connected.
    /// The subscription slot is immediately freed for new subscriptions (Requirements 4.5).
    ///
    /// # Arguments
    ///
    /// * `channel` - Channel name to unsubscribe from
    /// * `symbol` - Optional trading pair symbol
    ///
    /// # Errors
    ///
    /// Returns error if unsubscribe message fails to send.
    #[instrument(
        name = "ws_unsubscribe",
        skip(self),
        fields(channel = %channel, symbol = ?symbol)
    )]
    pub async fn unsubscribe(&self, channel: String, symbol: Option<String>) -> Result<()> {
        let sub_key = Self::subscription_key(&channel, symbol.as_ref());

        // Use SubscriptionManager to remove subscription (Requirements 4.5)
        self.subscription_manager.remove(&sub_key);

        info!(subscription_key = %sub_key, "Subscription removed");

        // Lock-free state check
        let state = self.state();
        if state == WsConnectionState::Connected {
            self.send_unsubscribe_message(channel, symbol).await?;
            info!(subscription_key = %sub_key, "Unsubscribe message sent");
        }

        Ok(())
    }

    /// Receives the next available message from the WebSocket stream.
    ///
    /// # Returns
    ///
    /// The received JSON message, or `None` if the channel is closed.
    pub async fn receive(&self) -> Option<Value> {
        let mut rx = self.message_rx.write().await;
        rx.recv().await
    }

    /// Returns the current connection state (lock-free).
    #[inline]
    pub fn state(&self) -> WsConnectionState {
        WsConnectionState::from_u8(self.state.load(Ordering::Acquire))
    }

    /// Returns a reference to the WebSocket configuration.
    ///
    /// # Example
    ///
    /// ```rust
    /// use ccxt_core::ws_client::{WsClient, WsConfig};
    ///
    /// let config = WsConfig {
    ///     url: "wss://stream.example.com/ws".to_string(),
    ///     max_reconnect_attempts: 10,
    ///     ..Default::default()
    /// };
    /// let client = WsClient::new(config);
    ///
    /// assert_eq!(client.config().max_reconnect_attempts, 10);
    /// ```
    #[inline]
    pub fn config(&self) -> &WsConfig {
        &self.config
    }

    /// Sets the connection state (lock-free).
    ///
    /// This method is used internally and by the `AutoReconnectCoordinator`
    /// to update the connection state.
    ///
    /// # Arguments
    ///
    /// * `state` - The new connection state
    #[inline]
    pub fn set_state(&self, state: WsConnectionState) {
        self.state.store(state.as_u8(), Ordering::Release);
    }

    /// Checks whether the WebSocket is currently connected (lock-free).
    #[inline]
    pub fn is_connected(&self) -> bool {
        self.state() == WsConnectionState::Connected
    }

    /// Checks if subscribed to a specific channel (lock-free).
    ///
    /// # Arguments
    ///
    /// * `channel` - Channel name to check
    /// * `symbol` - Optional trading pair symbol
    ///
    /// # Returns
    ///
    /// `true` if subscribed to the channel, `false` otherwise.
    pub fn is_subscribed(&self, channel: &str, symbol: Option<&String>) -> bool {
        let sub_key = Self::subscription_key(channel, symbol);
        self.subscription_manager.contains(&sub_key)
    }

    /// Returns the number of active subscriptions (lock-free).
    ///
    /// This method delegates to the internal `SubscriptionManager` to get
    /// the current subscription count.
    ///
    /// # Example
    ///
    /// ```rust
    /// use ccxt_core::ws_client::{WsClient, WsConfig};
    ///
    /// let client = WsClient::new(WsConfig::default());
    /// assert_eq!(client.subscription_count(), 0);
    /// ```
    pub fn subscription_count(&self) -> usize {
        self.subscription_manager.count()
    }

    /// Returns the remaining capacity for new subscriptions (lock-free).
    ///
    /// This is calculated as `max_subscriptions - current_count`.
    /// Use this method to check if there's room for more subscriptions
    /// before attempting to subscribe.
    ///
    /// # Example
    ///
    /// ```rust
    /// use ccxt_core::ws_client::{WsClient, WsConfig};
    ///
    /// let client = WsClient::new(WsConfig {
    ///     max_subscriptions: 50,
    ///     ..Default::default()
    /// });
    /// assert_eq!(client.remaining_capacity(), 50);
    /// ```
    pub fn remaining_capacity(&self) -> usize {
        self.subscription_manager.remaining_capacity()
    }

    /// Sends a raw WebSocket message.
    ///
    /// # Arguments
    ///
    /// * `message` - WebSocket message to send
    ///
    /// # Errors
    ///
    /// Returns error if not connected or message transmission fails.
    #[instrument(name = "ws_send", skip(self, message))]
    pub async fn send(&self, message: Message) -> Result<()> {
        let tx = self.write_tx.lock().await;

        if let Some(sender) = tx.as_ref() {
            sender.send(message).map_err(|e| {
                error!(
                    error = %e,
                    "Failed to send WebSocket message"
                );
                Error::network(format!("Failed to send message: {e}"))
            })?;
            debug!("WebSocket message sent successfully");
            Ok(())
        } else {
            warn!("WebSocket not connected, cannot send message");
            Err(Error::network("WebSocket not connected"))
        }
    }

    /// Sends a text message over the WebSocket connection.
    ///
    /// # Arguments
    ///
    /// * `text` - Text content to send
    ///
    /// # Errors
    ///
    /// Returns error if not connected or transmission fails.
    #[instrument(name = "ws_send_text", skip(self, text), fields(text_len = text.len()))]
    pub async fn send_text(&self, text: String) -> Result<()> {
        self.send(Message::Text(text.into())).await
    }

    /// Sends a JSON-encoded message over the WebSocket connection.
    ///
    /// # Arguments
    ///
    /// * `json` - JSON value to serialize and send
    ///
    /// # Errors
    ///
    /// Returns error if serialization fails, not connected, or transmission fails.
    #[instrument(name = "ws_send_json", skip(self, json))]
    pub async fn send_json(&self, json: &Value) -> Result<()> {
        let text = serde_json::to_string(json).map_err(|e| {
            error!(error = %e, "Failed to serialize JSON for WebSocket");
            Error::from(e)
        })?;
        self.send_text(text).await
    }

    /// Generates a unique subscription key from channel and symbol.
    fn subscription_key(channel: &str, symbol: Option<&String>) -> String {
        match symbol {
            Some(s) => format!("{channel}:{s}"),
            None => channel.to_string(),
        }
    }

    /// Starts the WebSocket message processing loop.
    ///
    /// Spawns separate tasks for reading and writing messages, handling shutdown signals.
    async fn start_message_loop(&self, ws_stream: WebSocketStream<MaybeTlsStream<TcpStream>>) {
        let (write, mut read) = ws_stream.split();

        let (write_tx, mut write_rx) = mpsc::unbounded_channel::<Message>();
        *self.write_tx.lock().await = Some(write_tx.clone());

        let (shutdown_tx, mut shutdown_rx) = mpsc::unbounded_channel::<()>();
        *self.shutdown_tx.lock().await = Some(shutdown_tx);

        let state = Arc::clone(&self.state);
        let message_tx = self.message_tx.clone();
        let ping_interval_ms = self.config.ping_interval;

        info!("Starting WebSocket message loop");

        let write_handle = tokio::spawn(async move {
            let mut write = write;
            loop {
                tokio::select! {
                    Some(msg) = write_rx.recv() => {
                        if let Err(e) = write.send(msg).await {
                            error!(error = %e, "Failed to write message");
                            break;
                        }
                    }
                    _ = shutdown_rx.recv() => {
                        debug!("Write task received shutdown signal");
                        let _ = write.send(Message::Close(None)).await;
                        break;
                    }
                }
            }
            debug!("Write task terminated");
        });

        let state_clone = Arc::clone(&state);
        let ws_stats = Arc::clone(&self.stats);
        let read_handle = tokio::spawn(async move {
            debug!("Starting WebSocket read task");
            while let Some(msg_result) = read.next().await {
                match msg_result {
                    Ok(Message::Text(text)) => {
                        debug!(len = text.len(), "Received text message");

                        // Lock-free stats update
                        ws_stats.record_received(text.len() as u64);

                        match serde_json::from_str::<Value>(&text) {
                            Ok(json) => {
                                let _ = message_tx.send(json);
                            }
                            Err(e) => {
                                // Log parse failure with truncated raw message preview
                                let raw_preview: String = text.chars().take(200).collect();
                                warn!(
                                    error = %e,
                                    raw_message_preview = %raw_preview,
                                    raw_message_len = text.len(),
                                    "Failed to parse WebSocket text message as JSON"
                                );
                            }
                        }
                    }
                    Ok(Message::Binary(data)) => {
                        debug!(len = data.len(), "Received binary message");

                        // Lock-free stats update
                        ws_stats.record_received(data.len() as u64);

                        match String::from_utf8(data.to_vec()) {
                            Ok(text) => {
                                match serde_json::from_str::<Value>(&text) {
                                    Ok(json) => {
                                        let _ = message_tx.send(json);
                                    }
                                    Err(e) => {
                                        // Log parse failure with truncated raw message preview
                                        let raw_preview: String = text.chars().take(200).collect();
                                        warn!(
                                            error = %e,
                                            raw_message_preview = %raw_preview,
                                            raw_message_len = text.len(),
                                            "Failed to parse WebSocket binary message as JSON"
                                        );
                                    }
                                }
                            }
                            Err(e) => {
                                // Log UTF-8 decode failure with hex preview
                                let hex_preview: String = data
                                    .iter()
                                    .take(50)
                                    .map(|b| format!("{b:02x}"))
                                    .collect::<Vec<_>>()
                                    .join(" ");
                                warn!(
                                    error = %e,
                                    hex_preview = %hex_preview,
                                    data_len = data.len(),
                                    "Failed to decode WebSocket binary message as UTF-8"
                                );
                            }
                        }
                    }
                    Ok(Message::Ping(_)) => {
                        debug!("Received ping, auto-responding with pong");
                    }
                    Ok(Message::Pong(_)) => {
                        debug!("Received pong");

                        // Lock-free stats update
                        ws_stats.record_pong();
                    }
                    Ok(Message::Close(frame)) => {
                        info!(
                            close_frame = ?frame,
                            "Received WebSocket close frame"
                        );
                        // Lock-free state update
                        state_clone
                            .store(WsConnectionState::Disconnected.as_u8(), Ordering::Release);
                        break;
                    }
                    Err(e) => {
                        error!(
                            error = %e,
                            error_debug = ?e,
                            "WebSocket read error"
                        );
                        // Lock-free state update
                        state_clone.store(WsConnectionState::Error.as_u8(), Ordering::Release);
                        break;
                    }
                    _ => {
                        debug!("Received other WebSocket message type");
                    }
                }
            }
            debug!("WebSocket read task terminated");
        });

        if ping_interval_ms > 0 {
            let write_tx_clone = write_tx.clone();
            let ping_stats = Arc::clone(&self.stats);
            let ping_state = Arc::clone(&state);
            let pong_timeout_ms = self.config.pong_timeout;

            tokio::spawn(async move {
                let mut interval = interval(Duration::from_millis(ping_interval_ms));
                debug!(
                    interval_ms = ping_interval_ms,
                    timeout_ms = pong_timeout_ms,
                    "Starting ping task with timeout detection"
                );

                loop {
                    interval.tick().await;

                    let now = chrono::Utc::now().timestamp_millis();
                    // Lock-free stats read
                    let last_pong = ping_stats.last_pong_time();

                    if last_pong > 0 {
                        let elapsed = now - last_pong;
                        #[allow(clippy::cast_possible_wrap)]
                        if elapsed > pong_timeout_ms as i64 {
                            warn!(
                                elapsed_ms = elapsed,
                                timeout_ms = pong_timeout_ms,
                                "Pong timeout detected, marking connection as error"
                            );
                            // Lock-free state update
                            ping_state.store(WsConnectionState::Error.as_u8(), Ordering::Release);
                            break;
                        }
                    }

                    // Lock-free stats update
                    ping_stats.record_ping();

                    if write_tx_clone.send(Message::Ping(vec![].into())).is_err() {
                        debug!("Ping task: write channel closed");
                        break;
                    }
                    debug!("Sent ping");
                }
                debug!("Ping task terminated");
            });
        }

        tokio::spawn(async move {
            let _ = tokio::join!(write_handle, read_handle);
            info!("All WebSocket tasks completed");
        });
    }

    /// Sends a subscription message to the WebSocket server.
    #[instrument(
        name = "ws_send_subscribe",
        skip(self, params),
        fields(channel = %channel, symbol = ?symbol)
    )]
    async fn send_subscribe_message(
        &self,
        channel: String,
        symbol: Option<String>,
        params: Option<HashMap<String, Value>>,
    ) -> Result<()> {
        let msg = WsMessage::Subscribe {
            channel: channel.clone(),
            symbol: symbol.clone(),
            params,
        };

        let json = serde_json::to_value(&msg).map_err(|e| {
            error!(error = %e, "Failed to serialize subscribe message");
            Error::from(e)
        })?;

        debug!("Sending subscribe message to server");

        self.send_json(&json).await?;
        info!("Subscribe message sent successfully");
        Ok(())
    }

    /// Sends an unsubscribe message to the WebSocket server.
    #[instrument(
        name = "ws_send_unsubscribe",
        skip(self),
        fields(channel = %channel, symbol = ?symbol)
    )]
    async fn send_unsubscribe_message(
        &self,
        channel: String,
        symbol: Option<String>,
    ) -> Result<()> {
        let msg = WsMessage::Unsubscribe {
            channel: channel.clone(),
            symbol: symbol.clone(),
        };

        let json = serde_json::to_value(&msg).map_err(|e| {
            error!(error = %e, "Failed to serialize unsubscribe message");
            Error::from(e)
        })?;

        debug!("Sending unsubscribe message to server");

        self.send_json(&json).await?;
        info!("Unsubscribe message sent successfully");
        Ok(())
    }

    /// Resubscribes to all previously subscribed channels.
    async fn resubscribe_all(&self) -> Result<()> {
        // Collect subscriptions using SubscriptionManager to avoid holding reference during async calls
        let subs = self.subscription_manager.collect_subscriptions();

        for subscription in subs {
            self.send_subscribe_message(
                subscription.channel.clone(),
                subscription.symbol.clone(),
                subscription.params.clone(),
            )
            .await?;
        }
        Ok(())
    }
}
/// WebSocket connection event types.
///
/// These events are emitted during the WebSocket connection lifecycle to notify
/// listeners about state changes, reconnection attempts, and errors.
///
/// # Example
///
/// ```rust
/// use ccxt_core::ws_client::WsEvent;
/// use std::time::Duration;
///
/// fn handle_event(event: WsEvent) {
///     match event {
///         WsEvent::Connecting => println!("Connecting..."),
///         WsEvent::Connected => println!("Connected!"),
///         WsEvent::Disconnected => println!("Disconnected"),
///         WsEvent::Reconnecting { attempt, delay, error } => {
///             println!("Reconnecting (attempt {}), delay: {:?}, error: {:?}",
///                      attempt, delay, error);
///         }
///         WsEvent::ReconnectSuccess => println!("Reconnected successfully"),
///         WsEvent::ReconnectFailed { attempt, error, is_permanent } => {
///             println!("Reconnect failed (attempt {}): {}, permanent: {}",
///                      attempt, error, is_permanent);
///         }
///         WsEvent::ReconnectExhausted { total_attempts, last_error } => {
///             println!("All {} reconnect attempts exhausted: {}",
///                      total_attempts, last_error);
///         }
///         WsEvent::SubscriptionRestored => println!("Subscriptions restored"),
///         WsEvent::PermanentError { error } => {
///             println!("Permanent error (no retry): {}", error);
///         }
///         WsEvent::Shutdown => println!("Shutdown complete"),
///     }
/// }
/// ```
#[derive(Debug, Clone)]
pub enum WsEvent {
    /// Connection attempt started.
    ///
    /// Emitted when the client begins establishing a WebSocket connection.
    Connecting,

    /// Connection established successfully.
    ///
    /// Emitted when the WebSocket handshake completes and the connection is ready.
    Connected,

    /// Connection closed.
    ///
    /// Emitted when the WebSocket connection is closed, either gracefully or due to an error.
    Disconnected,

    /// Reconnection in progress.
    ///
    /// Emitted before each reconnection attempt with details about the attempt.
    Reconnecting {
        /// Current reconnection attempt number (1-indexed).
        attempt: u32,
        /// Delay before this reconnection attempt (calculated by backoff strategy).
        delay: Duration,
        /// Error that triggered the reconnection (if any).
        error: Option<String>,
    },

    /// Reconnection succeeded.
    ///
    /// Emitted when a reconnection attempt successfully establishes a new connection.
    ReconnectSuccess,

    /// Single reconnection attempt failed.
    ///
    /// Emitted when a reconnection attempt fails. More attempts may follow
    /// unless `is_permanent` is true or max attempts is reached.
    ReconnectFailed {
        /// The attempt number that failed (1-indexed).
        attempt: u32,
        /// Error message describing the failure.
        error: String,
        /// Whether this is a permanent error that should not be retried.
        ///
        /// If `true`, no further reconnection attempts will be made.
        is_permanent: bool,
    },

    /// All reconnection attempts exhausted.
    ///
    /// Emitted when the maximum number of reconnection attempts has been reached
    /// without successfully reconnecting. No further automatic reconnection will occur.
    ReconnectExhausted {
        /// Total number of reconnection attempts made.
        total_attempts: u32,
        /// The last error encountered.
        last_error: String,
    },

    /// Subscriptions restored after reconnection.
    ///
    /// Emitted after a successful reconnection when all previous subscriptions
    /// have been re-established.
    SubscriptionRestored,

    /// Permanent error occurred (no retry).
    ///
    /// Emitted when a permanent error is encountered that cannot be recovered
    /// through retries (e.g., authentication failure, invalid credentials).
    PermanentError {
        /// Error message describing the permanent failure.
        error: String,
    },

    /// Shutdown completed.
    ///
    /// Emitted when the WebSocket client has completed its graceful shutdown
    /// process, including cancelling pending operations and closing connections.
    Shutdown,
}

impl WsEvent {
    /// Returns `true` if this is a `Connecting` event.
    #[inline]
    #[must_use]
    pub fn is_connecting(&self) -> bool {
        matches!(self, Self::Connecting)
    }

    /// Returns `true` if this is a `Connected` event.
    #[inline]
    #[must_use]
    pub fn is_connected(&self) -> bool {
        matches!(self, Self::Connected)
    }

    /// Returns `true` if this is a `Disconnected` event.
    #[inline]
    #[must_use]
    pub fn is_disconnected(&self) -> bool {
        matches!(self, Self::Disconnected)
    }

    /// Returns `true` if this is a `Reconnecting` event.
    #[inline]
    #[must_use]
    pub fn is_reconnecting(&self) -> bool {
        matches!(self, Self::Reconnecting { .. })
    }

    /// Returns `true` if this is a `ReconnectSuccess` event.
    #[inline]
    #[must_use]
    pub fn is_reconnect_success(&self) -> bool {
        matches!(self, Self::ReconnectSuccess)
    }

    /// Returns `true` if this is a `ReconnectFailed` event.
    #[inline]
    #[must_use]
    pub fn is_reconnect_failed(&self) -> bool {
        matches!(self, Self::ReconnectFailed { .. })
    }

    /// Returns `true` if this is a `ReconnectExhausted` event.
    #[inline]
    #[must_use]
    pub fn is_reconnect_exhausted(&self) -> bool {
        matches!(self, Self::ReconnectExhausted { .. })
    }

    /// Returns `true` if this is a `SubscriptionRestored` event.
    #[inline]
    #[must_use]
    pub fn is_subscription_restored(&self) -> bool {
        matches!(self, Self::SubscriptionRestored)
    }

    /// Returns `true` if this is a `PermanentError` event.
    #[inline]
    #[must_use]
    pub fn is_permanent_error(&self) -> bool {
        matches!(self, Self::PermanentError { .. })
    }

    /// Returns `true` if this is a `Shutdown` event.
    #[inline]
    #[must_use]
    pub fn is_shutdown(&self) -> bool {
        matches!(self, Self::Shutdown)
    }

    /// Returns `true` if this event indicates an error condition.
    ///
    /// This includes `ReconnectFailed`, `ReconnectExhausted`, and `PermanentError`.
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

    /// Returns `true` if this event indicates a terminal state.
    ///
    /// Terminal states are those where no further automatic recovery will occur:
    /// `ReconnectExhausted`, `PermanentError`, and `Shutdown`.
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

/// Automatic reconnection coordinator for WebSocket connections.
///
/// Monitors connection state and triggers reconnection attempts when disconnected.
/// Uses exponential backoff strategy for retry delays and classifies errors to
/// determine if reconnection should continue.
///
/// # Features
///
/// - **Exponential Backoff**: Uses configurable exponential backoff with jitter
///   to prevent thundering herd effects during reconnection.
/// - **Error Classification**: Distinguishes between transient and permanent errors,
///   stopping reconnection attempts for permanent errors.
/// - **Cancellation Support**: Supports graceful cancellation via `CancellationToken`.
/// - **Event Callbacks**: Notifies registered callbacks about reconnection events.
///
/// # Example
///
/// ```rust,ignore
/// use ccxt_core::ws_client::{WsClient, WsConfig, AutoReconnectCoordinator, WsEvent};
/// use std::sync::Arc;
///
/// let client = Arc::new(WsClient::new(WsConfig::default()));
/// let coordinator = client.clone().create_auto_reconnect_coordinator()
///     .with_callback(Arc::new(|event| {
///         match event {
///             WsEvent::Reconnecting { attempt, delay, .. } => {
///                 println!("Reconnecting attempt {} with delay {:?}", attempt, delay);
///             }
///             WsEvent::ReconnectSuccess => println!("Reconnected!"),
///             _ => {}
///         }
///     }));
///
/// coordinator.start().await;
/// ```
pub struct AutoReconnectCoordinator {
    /// The WebSocket client to manage reconnection for
    client: Arc<WsClient>,
    /// Whether auto-reconnect is enabled
    enabled: Arc<AtomicBool>,
    /// Handle to the reconnection task
    reconnect_task: Arc<Mutex<Option<JoinHandle<()>>>>,
    /// Optional event callback for reconnection events
    event_callback: Option<WsEventCallback>,
    /// Cancellation token for stopping reconnection
    cancel_token: Arc<Mutex<Option<CancellationToken>>>,
}

impl AutoReconnectCoordinator {
    /// Creates a new automatic reconnection coordinator.
    ///
    /// The coordinator uses the backoff configuration from the client's `WsConfig`.
    ///
    /// # Arguments
    ///
    /// * `client` - Arc reference to the WebSocket client
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use ccxt_core::ws_client::{WsClient, WsConfig, AutoReconnectCoordinator};
    /// use std::sync::Arc;
    ///
    /// let client = Arc::new(WsClient::new(WsConfig::default()));
    /// let coordinator = AutoReconnectCoordinator::new(client);
    /// ```
    pub fn new(client: Arc<WsClient>) -> Self {
        Self {
            client,
            enabled: Arc::new(AtomicBool::new(false)),
            reconnect_task: Arc::new(Mutex::new(None)),
            event_callback: None,
            cancel_token: Arc::new(Mutex::new(None)),
        }
    }

    /// Sets the event callback for reconnection events.
    ///
    /// # Arguments
    ///
    /// * `callback` - Event callback function
    ///
    /// # Returns
    ///
    /// Self for method chaining
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use ccxt_core::ws_client::{AutoReconnectCoordinator, WsEvent};
    /// use std::sync::Arc;
    ///
    /// let coordinator = coordinator.with_callback(Arc::new(|event| {
    ///     println!("Event: {:?}", event);
    /// }));
    /// ```
    pub fn with_callback(mut self, callback: WsEventCallback) -> Self {
        self.event_callback = Some(callback);
        self
    }

    /// Sets the cancellation token for stopping reconnection.
    ///
    /// When the token is cancelled, the reconnection loop will stop gracefully.
    ///
    /// # Arguments
    ///
    /// * `token` - Cancellation token
    ///
    /// # Returns
    ///
    /// Self for method chaining
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use ccxt_core::ws_client::AutoReconnectCoordinator;
    /// use tokio_util::sync::CancellationToken;
    ///
    /// let token = CancellationToken::new();
    /// let coordinator = coordinator.with_cancel_token(token.clone());
    ///
    /// // Later, to stop reconnection:
    /// token.cancel();
    /// ```
    pub fn with_cancel_token(self, token: CancellationToken) -> Self {
        // We need to store it synchronously during construction
        // The actual storage happens in start()
        let _ = self.cancel_token.try_lock().map(|mut guard| {
            *guard = Some(token);
        });
        self
    }

    /// Sets the cancellation token asynchronously.
    ///
    /// # Arguments
    ///
    /// * `token` - Cancellation token
    pub async fn set_cancel_token(&self, token: CancellationToken) {
        let mut guard = self.cancel_token.lock().await;
        *guard = Some(token);
    }

    /// Returns whether auto-reconnect is currently enabled.
    ///
    /// # Returns
    ///
    /// `true` if auto-reconnect is enabled, `false` otherwise
    #[inline]
    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::SeqCst)
    }

    /// Starts the automatic reconnection coordinator.
    ///
    /// Begins monitoring connection state and automatically reconnects on disconnect.
    /// Uses exponential backoff for retry delays.
    ///
    /// # Behavior
    ///
    /// 1. Monitors connection state every second
    /// 2. When disconnected or in error state, calculates delay using exponential backoff
    /// 3. Emits `Reconnecting` event before each attempt
    /// 4. Attempts reconnection
    /// 5. On success: resets reconnect counter, emits `ReconnectSuccess`, restores subscriptions
    /// 6. On transient error: waits for backoff delay and retries
    /// 7. On permanent error: emits `PermanentError` and stops
    /// 8. On max attempts: emits `ReconnectExhausted` and stops
    pub async fn start(&self) {
        if self.enabled.swap(true, Ordering::SeqCst) {
            info!("Auto-reconnect already started");
            return;
        }

        info!("Starting auto-reconnect coordinator with exponential backoff");

        let client = Arc::clone(&self.client);
        let enabled = Arc::clone(&self.enabled);
        let callback = self.event_callback.clone();
        let cancel_token = self.cancel_token.lock().await.clone();

        // Create backoff strategy from client config
        let backoff_config = client.config().backoff_config.clone();

        let handle = tokio::spawn(async move {
            Self::reconnect_loop(client, enabled, callback, backoff_config, cancel_token).await;
        });

        *self.reconnect_task.lock().await = Some(handle);
    }

    /// Stops the automatic reconnection coordinator.
    ///
    /// Halts monitoring and reconnection tasks. If a cancellation token was set,
    /// it will be cancelled to stop any in-progress reconnection attempts.
    pub async fn stop(&self) {
        if !self.enabled.swap(false, Ordering::SeqCst) {
            info!("Auto-reconnect already stopped");
            return;
        }

        info!("Stopping auto-reconnect coordinator");

        // Cancel any in-progress reconnection
        if let Some(token) = self.cancel_token.lock().await.as_ref() {
            token.cancel();
        }

        let mut task = self.reconnect_task.lock().await;
        if let Some(handle) = task.take() {
            handle.abort();
        }
    }

    /// Internal reconnection loop with exponential backoff.
    ///
    /// Continuously monitors connection state and triggers reconnection
    /// when `Error` or `Disconnected` state is detected. Uses exponential
    /// backoff for retry delays and classifies errors to determine if
    /// reconnection should continue.
    ///
    /// # Arguments
    ///
    /// * `client` - The WebSocket client
    /// * `enabled` - Atomic flag indicating if auto-reconnect is enabled
    /// * `callback` - Optional event callback
    /// * `backoff_config` - Configuration for exponential backoff
    /// * `cancel_token` - Optional cancellation token
    async fn reconnect_loop(
        client: Arc<WsClient>,
        enabled: Arc<AtomicBool>,
        callback: Option<WsEventCallback>,
        backoff_config: BackoffConfig,
        cancel_token: Option<CancellationToken>,
    ) {
        let mut check_interval = interval(Duration::from_secs(1));
        let backoff = BackoffStrategy::new(backoff_config);
        let mut last_error: Option<String> = None;

        loop {
            // Check for cancellation
            if cancel_token
                .as_ref()
                .is_some_and(CancellationToken::is_cancelled)
            {
                info!("Auto-reconnect cancelled via token");
                break;
            }

            check_interval.tick().await;

            if !enabled.load(Ordering::SeqCst) {
                debug!("Auto-reconnect disabled, exiting loop");
                break;
            }

            // Lock-free state check
            let state = client.state();

            if matches!(
                state,
                WsConnectionState::Disconnected | WsConnectionState::Error
            ) {
                // Lock-free reconnect count access
                let attempt = client.reconnect_count();

                // Check if max attempts reached
                let max_attempts = client.config().max_reconnect_attempts;
                if attempt >= max_attempts {
                    error!(
                        attempts = attempt,
                        max = max_attempts,
                        "Max reconnect attempts reached, stopping auto-reconnect"
                    );

                    if let Some(ref cb) = callback {
                        cb(WsEvent::ReconnectExhausted {
                            total_attempts: attempt,
                            last_error: last_error
                                .clone()
                                .unwrap_or_else(|| "Unknown error".to_string()),
                        });
                    }
                    break;
                }

                // Calculate delay using exponential backoff strategy
                let delay = backoff.calculate_delay(attempt);

                #[allow(clippy::cast_possible_truncation)]
                {
                    info!(
                        attempt = attempt + 1,
                        max = max_attempts,
                        delay_ms = delay.as_millis() as u64,
                        state = ?state,
                        "Connection lost, attempting reconnect with exponential backoff"
                    );
                }

                if let Some(ref cb) = callback {
                    cb(WsEvent::Reconnecting {
                        attempt: attempt + 1,
                        delay,
                        error: last_error.clone(),
                    });
                }

                // Wait for backoff delay with cancellation support
                if let Some(ref token) = cancel_token {
                    tokio::select! {
                        biased;

                        () = token.cancelled() => {
                            info!("Auto-reconnect cancelled during backoff delay");
                            break;
                        }

                        () = tokio::time::sleep(delay) => {
                            // Delay completed, proceed with reconnection
                        }
                    }
                } else {
                    tokio::time::sleep(delay).await;
                }

                // Check cancellation again after delay
                if cancel_token
                    .as_ref()
                    .is_some_and(CancellationToken::is_cancelled)
                {
                    info!("Auto-reconnect cancelled after backoff delay");
                    break;
                }

                // Attempt reconnection using connect() directly to avoid double-counting
                // The reconnect_count is managed by this loop
                client.set_state(WsConnectionState::Reconnecting);

                match client.connect().await {
                    Ok(()) => {
                        info!(attempt = attempt + 1, "Reconnection successful");

                        // Reset reconnect count on success (Requirements 1.4)
                        client.reset_reconnect_count();
                        last_error = None;

                        if let Some(ref cb) = callback {
                            cb(WsEvent::ReconnectSuccess);
                        }

                        // Restore subscriptions
                        match client.resubscribe_all().await {
                            Ok(()) => {
                                info!("Subscriptions restored");
                                if let Some(ref cb) = callback {
                                    cb(WsEvent::SubscriptionRestored);
                                }
                            }
                            Err(e) => {
                                error!(error = %e, "Failed to restore subscriptions");
                            }
                        }
                    }
                    Err(e) => {
                        let error_msg = e.to_string();
                        let ws_error = WsError::from_error(&e);
                        let is_permanent = ws_error.is_permanent();

                        error!(
                            attempt = attempt + 1,
                            error = %e,
                            is_permanent = is_permanent,
                            "Reconnection failed"
                        );

                        last_error = Some(error_msg.clone());

                        // Increment reconnect count for failed attempt
                        client.reconnect_count.fetch_add(1, Ordering::AcqRel);

                        if let Some(ref cb) = callback {
                            cb(WsEvent::ReconnectFailed {
                                attempt: attempt + 1,
                                error: error_msg,
                                is_permanent,
                            });
                        }

                        // Don't retry if it's a permanent error (Requirements 2.2, 2.3, 2.5)
                        if is_permanent {
                            if let Some(ref cb) = callback {
                                cb(WsEvent::PermanentError {
                                    error: e.to_string(),
                                });
                            }
                            break;
                        }

                        // For transient errors, the loop will continue and calculate
                        // a new backoff delay on the next iteration
                    }
                }
            }
        }

        info!("Auto-reconnect loop terminated");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==================== BackoffConfig Tests ====================

    #[test]
    fn test_backoff_config_default() {
        let config = BackoffConfig::default();
        assert_eq!(config.base_delay, Duration::from_secs(1));
        assert_eq!(config.max_delay, Duration::from_secs(60));
        assert!((config.jitter_factor - 0.25).abs() < f64::EPSILON);
        assert!((config.multiplier - 2.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_backoff_config_custom() {
        let config = BackoffConfig {
            base_delay: Duration::from_millis(500),
            max_delay: Duration::from_secs(30),
            jitter_factor: 0.1,
            multiplier: 3.0,
        };
        assert_eq!(config.base_delay, Duration::from_millis(500));
        assert_eq!(config.max_delay, Duration::from_secs(30));
        assert!((config.jitter_factor - 0.1).abs() < f64::EPSILON);
        assert!((config.multiplier - 3.0).abs() < f64::EPSILON);
    }

    // ==================== BackoffStrategy Tests ====================

    #[test]
    fn test_backoff_strategy_with_defaults() {
        let strategy = BackoffStrategy::with_defaults();
        assert_eq!(strategy.config().base_delay, Duration::from_secs(1));
        assert_eq!(strategy.config().max_delay, Duration::from_secs(60));
    }

    #[test]
    fn test_backoff_strategy_exponential_growth_no_jitter() {
        let config = BackoffConfig {
            base_delay: Duration::from_secs(1),
            max_delay: Duration::from_secs(60),
            jitter_factor: 0.0, // No jitter for predictable results
            multiplier: 2.0,
        };
        let strategy = BackoffStrategy::new(config);

        // Verify exponential growth: 1s, 2s, 4s, 8s, 16s, 32s, 60s (capped)
        assert_eq!(strategy.calculate_delay(0), Duration::from_secs(1));
        assert_eq!(strategy.calculate_delay(1), Duration::from_secs(2));
        assert_eq!(strategy.calculate_delay(2), Duration::from_secs(4));
        assert_eq!(strategy.calculate_delay(3), Duration::from_secs(8));
        assert_eq!(strategy.calculate_delay(4), Duration::from_secs(16));
        assert_eq!(strategy.calculate_delay(5), Duration::from_secs(32));
        // At attempt 6, 1 * 2^6 = 64s, but capped at 60s
        assert_eq!(strategy.calculate_delay(6), Duration::from_secs(60));
        // Further attempts stay at max
        assert_eq!(strategy.calculate_delay(10), Duration::from_secs(60));
    }

    #[test]
    fn test_backoff_strategy_max_delay_cap() {
        let config = BackoffConfig {
            base_delay: Duration::from_secs(10),
            max_delay: Duration::from_secs(30),
            jitter_factor: 0.0,
            multiplier: 2.0,
        };
        let strategy = BackoffStrategy::new(config);

        // 10s, 20s, 30s (capped), 30s (capped)
        assert_eq!(strategy.calculate_delay(0), Duration::from_secs(10));
        assert_eq!(strategy.calculate_delay(1), Duration::from_secs(20));
        assert_eq!(strategy.calculate_delay(2), Duration::from_secs(30)); // 40s capped to 30s
        assert_eq!(strategy.calculate_delay(3), Duration::from_secs(30)); // 80s capped to 30s
    }

    #[test]
    fn test_backoff_strategy_jitter_bounds() {
        let config = BackoffConfig {
            base_delay: Duration::from_secs(1),
            max_delay: Duration::from_secs(60),
            jitter_factor: 0.25,
            multiplier: 2.0,
        };
        let strategy = BackoffStrategy::new(config);

        // Run multiple times to test jitter randomness
        for _ in 0..100 {
            let delay = strategy.calculate_delay(0);
            // Base delay is 1s, jitter is [0, 0.25s]
            // So delay should be in [1s, 1.25s]
            assert!(delay >= Duration::from_secs(1));
            assert!(delay <= Duration::from_millis(1250));
        }

        for _ in 0..100 {
            let delay = strategy.calculate_delay(2);
            // Base delay is 4s, jitter is [0, 1s]
            // So delay should be in [4s, 5s]
            assert!(delay >= Duration::from_secs(4));
            assert!(delay <= Duration::from_secs(5));
        }
    }

    #[test]
    fn test_backoff_strategy_calculate_delay_without_jitter() {
        let config = BackoffConfig {
            base_delay: Duration::from_secs(1),
            max_delay: Duration::from_secs(60),
            jitter_factor: 0.25, // Has jitter configured
            multiplier: 2.0,
        };
        let strategy = BackoffStrategy::new(config);

        // calculate_delay_without_jitter should always return the same value
        assert_eq!(
            strategy.calculate_delay_without_jitter(0),
            Duration::from_secs(1)
        );
        assert_eq!(
            strategy.calculate_delay_without_jitter(1),
            Duration::from_secs(2)
        );
        assert_eq!(
            strategy.calculate_delay_without_jitter(2),
            Duration::from_secs(4)
        );
    }

    #[test]
    fn test_backoff_strategy_custom_multiplier() {
        let config = BackoffConfig {
            base_delay: Duration::from_secs(1),
            max_delay: Duration::from_secs(100),
            jitter_factor: 0.0,
            multiplier: 3.0, // Triple each time
        };
        let strategy = BackoffStrategy::new(config);

        // 1s, 3s, 9s, 27s, 81s, 100s (capped)
        assert_eq!(strategy.calculate_delay(0), Duration::from_secs(1));
        assert_eq!(strategy.calculate_delay(1), Duration::from_secs(3));
        assert_eq!(strategy.calculate_delay(2), Duration::from_secs(9));
        assert_eq!(strategy.calculate_delay(3), Duration::from_secs(27));
        assert_eq!(strategy.calculate_delay(4), Duration::from_secs(81));
        assert_eq!(strategy.calculate_delay(5), Duration::from_secs(100)); // 243s capped
    }

    #[test]
    fn test_backoff_strategy_millisecond_precision() {
        let config = BackoffConfig {
            base_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(10),
            jitter_factor: 0.0,
            multiplier: 2.0,
        };
        let strategy = BackoffStrategy::new(config);

        // 100ms, 200ms, 400ms, 800ms, 1600ms, ...
        assert_eq!(strategy.calculate_delay(0), Duration::from_millis(100));
        assert_eq!(strategy.calculate_delay(1), Duration::from_millis(200));
        assert_eq!(strategy.calculate_delay(2), Duration::from_millis(400));
        assert_eq!(strategy.calculate_delay(3), Duration::from_millis(800));
        assert_eq!(strategy.calculate_delay(4), Duration::from_millis(1600));
    }

    // ==================== WsConfig Tests ====================

    #[test]
    fn test_ws_config_default() {
        let config = WsConfig::default();
        assert_eq!(config.connect_timeout, 10000);
        assert_eq!(config.ping_interval, 30000);
        assert_eq!(config.reconnect_interval, 5000);
        assert_eq!(config.max_reconnect_attempts, 5);
        assert!(config.auto_reconnect);
        assert!(!config.enable_compression);
        assert_eq!(config.pong_timeout, 90000);
        // New fields added for WebSocket resilience improvements
        assert_eq!(config.max_subscriptions, DEFAULT_MAX_SUBSCRIPTIONS);
        assert_eq!(config.shutdown_timeout, DEFAULT_SHUTDOWN_TIMEOUT);
        // Verify backoff_config defaults
        assert_eq!(config.backoff_config.base_delay, Duration::from_secs(1));
        assert_eq!(config.backoff_config.max_delay, Duration::from_secs(60));
        assert!((config.backoff_config.jitter_factor - 0.25).abs() < f64::EPSILON);
        assert!((config.backoff_config.multiplier - 2.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_subscription_key() {
        let key1 = WsClient::subscription_key("ticker", Some(&"BTC/USDT".to_string()));
        assert_eq!(key1, "ticker:BTC/USDT");

        let key2 = WsClient::subscription_key("trades", None);
        assert_eq!(key2, "trades");
    }

    #[tokio::test]
    async fn test_ws_client_creation() {
        let config = WsConfig {
            url: "wss://example.com/ws".to_string(),
            ..Default::default()
        };

        let client = WsClient::new(config);
        // state() is now lock-free (no await needed)
        assert_eq!(client.state(), WsConnectionState::Disconnected);
        // is_connected() is now lock-free (no await needed)
        assert!(!client.is_connected());
    }

    #[tokio::test]
    async fn test_subscribe_adds_subscription() {
        let config = WsConfig {
            url: "wss://example.com/ws".to_string(),
            ..Default::default()
        };

        let client = WsClient::new(config);

        let result = client
            .subscribe("ticker".to_string(), Some("BTC/USDT".to_string()), None)
            .await;
        assert!(result.is_ok());

        // Use DashMap API (lock-free)
        assert_eq!(client.subscription_count(), 1);
        assert!(client.is_subscribed("ticker", Some(&"BTC/USDT".to_string())));
    }

    #[tokio::test]
    async fn test_unsubscribe_removes_subscription() {
        let config = WsConfig {
            url: "wss://example.com/ws".to_string(),
            ..Default::default()
        };

        let client = WsClient::new(config);

        client
            .subscribe("ticker".to_string(), Some("BTC/USDT".to_string()), None)
            .await
            .unwrap();

        let result = client
            .unsubscribe("ticker".to_string(), Some("BTC/USDT".to_string()))
            .await;
        assert!(result.is_ok());

        // Use DashMap API (lock-free)
        assert_eq!(client.subscription_count(), 0);
        assert!(!client.is_subscribed("ticker", Some(&"BTC/USDT".to_string())));
    }

    #[test]
    fn test_ws_message_serialization() {
        let msg = WsMessage::Subscribe {
            channel: "ticker".to_string(),
            symbol: Some("BTC/USDT".to_string()),
            params: None,
        };

        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"type\":\"subscribe\""));
        assert!(json.contains("\"channel\":\"ticker\""));
    }

    #[test]
    fn test_ws_connection_state_from_u8() {
        assert_eq!(
            WsConnectionState::from_u8(0),
            WsConnectionState::Disconnected
        );
        assert_eq!(WsConnectionState::from_u8(1), WsConnectionState::Connecting);
        assert_eq!(WsConnectionState::from_u8(2), WsConnectionState::Connected);
        assert_eq!(
            WsConnectionState::from_u8(3),
            WsConnectionState::Reconnecting
        );
        assert_eq!(WsConnectionState::from_u8(4), WsConnectionState::Error);
        // Unknown values default to Error
        assert_eq!(WsConnectionState::from_u8(5), WsConnectionState::Error);
        assert_eq!(WsConnectionState::from_u8(255), WsConnectionState::Error);
    }

    #[test]
    fn test_ws_connection_state_as_u8() {
        assert_eq!(WsConnectionState::Disconnected.as_u8(), 0);
        assert_eq!(WsConnectionState::Connecting.as_u8(), 1);
        assert_eq!(WsConnectionState::Connected.as_u8(), 2);
        assert_eq!(WsConnectionState::Reconnecting.as_u8(), 3);
        assert_eq!(WsConnectionState::Error.as_u8(), 4);
    }

    #[test]
    fn test_reconnect_count_lock_free() {
        let config = WsConfig {
            url: "wss://example.com/ws".to_string(),
            ..Default::default()
        };

        let client = WsClient::new(config);

        // Initial count should be 0
        assert_eq!(client.reconnect_count(), 0);

        // Reset should work
        client.reset_reconnect_count();
        assert_eq!(client.reconnect_count(), 0);
    }

    #[tokio::test]
    async fn test_concurrent_subscription_operations() {
        use std::sync::Arc;

        let config = WsConfig {
            url: "wss://example.com/ws".to_string(),
            ..Default::default()
        };

        let client = Arc::new(WsClient::new(config));

        // Spawn multiple tasks that add subscriptions concurrently
        let mut handles = vec![];

        for i in 0..10 {
            let client_clone = Arc::clone(&client);
            let handle = tokio::spawn(async move {
                let channel = format!("channel{}", i);
                let symbol = Some(format!("SYMBOL{}/USDT", i));
                client_clone.subscribe(channel, symbol, None).await.unwrap();
            });
            handles.push(handle);
        }

        // Wait for all tasks to complete
        for handle in handles {
            handle.await.unwrap();
        }

        // All subscriptions should be present
        assert_eq!(client.subscription_count(), 10);

        // Verify each subscription exists
        for i in 0..10 {
            assert!(
                client.is_subscribed(&format!("channel{}", i), Some(&format!("SYMBOL{}/USDT", i)))
            );
        }
    }

    #[tokio::test]
    async fn test_concurrent_state_access() {
        use std::sync::Arc;

        let config = WsConfig {
            url: "wss://example.com/ws".to_string(),
            ..Default::default()
        };

        let client = Arc::new(WsClient::new(config));

        // Spawn multiple tasks that read state concurrently
        let mut handles = vec![];

        for _ in 0..100 {
            let client_clone = Arc::clone(&client);
            let handle = tokio::spawn(async move {
                // Lock-free state access should not panic or deadlock
                let _ = client_clone.state();
                let _ = client_clone.is_connected();
            });
            handles.push(handle);
        }

        // Wait for all tasks to complete
        for handle in handles {
            handle.await.unwrap();
        }

        // State should still be Disconnected
        assert_eq!(client.state(), WsConnectionState::Disconnected);
    }

    #[tokio::test]
    async fn test_concurrent_subscription_add_remove() {
        use std::sync::Arc;

        let config = WsConfig {
            url: "wss://example.com/ws".to_string(),
            ..Default::default()
        };

        let client = Arc::new(WsClient::new(config));

        // Add some initial subscriptions
        for i in 0..5 {
            client
                .subscribe(
                    format!("channel{}", i),
                    Some(format!("SYM{}/USDT", i)),
                    None,
                )
                .await
                .unwrap();
        }

        // Spawn tasks that add and remove subscriptions concurrently
        let mut handles = vec![];

        // Add new subscriptions
        for i in 5..10 {
            let client_clone = Arc::clone(&client);
            let handle = tokio::spawn(async move {
                client_clone
                    .subscribe(
                        format!("channel{}", i),
                        Some(format!("SYM{}/USDT", i)),
                        None,
                    )
                    .await
                    .unwrap();
            });
            handles.push(handle);
        }

        // Remove some existing subscriptions
        for i in 0..3 {
            let client_clone = Arc::clone(&client);
            let handle = tokio::spawn(async move {
                client_clone
                    .unsubscribe(format!("channel{}", i), Some(format!("SYM{}/USDT", i)))
                    .await
                    .unwrap();
            });
            handles.push(handle);
        }

        // Wait for all tasks to complete
        for handle in handles {
            handle.await.unwrap();
        }

        // Should have 7 subscriptions (5 initial + 5 added - 3 removed)
        assert_eq!(client.subscription_count(), 7);
    }

    // ==================== WsErrorKind Tests ====================

    #[test]
    fn test_ws_error_kind_transient() {
        let kind = WsErrorKind::Transient;
        assert!(kind.is_transient());
        assert!(!kind.is_permanent());
        assert_eq!(kind.to_string(), "Transient");
    }

    #[test]
    fn test_ws_error_kind_permanent() {
        let kind = WsErrorKind::Permanent;
        assert!(!kind.is_transient());
        assert!(kind.is_permanent());
        assert_eq!(kind.to_string(), "Permanent");
    }

    #[test]
    fn test_ws_error_kind_equality() {
        assert_eq!(WsErrorKind::Transient, WsErrorKind::Transient);
        assert_eq!(WsErrorKind::Permanent, WsErrorKind::Permanent);
        assert_ne!(WsErrorKind::Transient, WsErrorKind::Permanent);
    }

    #[test]
    fn test_ws_error_kind_clone() {
        let kind = WsErrorKind::Transient;
        let cloned = kind;
        assert_eq!(kind, cloned);
    }

    // ==================== WsError Tests ====================

    #[test]
    fn test_ws_error_transient_creation() {
        let err = WsError::transient("Connection timeout");
        assert!(err.is_transient());
        assert!(!err.is_permanent());
        assert_eq!(err.kind(), WsErrorKind::Transient);
        assert_eq!(err.message(), "Connection timeout");
        assert!(err.source().is_none());
    }

    #[test]
    fn test_ws_error_permanent_creation() {
        let err = WsError::permanent("Invalid API key");
        assert!(!err.is_transient());
        assert!(err.is_permanent());
        assert_eq!(err.kind(), WsErrorKind::Permanent);
        assert_eq!(err.message(), "Invalid API key");
        assert!(err.source().is_none());
    }

    #[test]
    fn test_ws_error_new() {
        let err = WsError::new(WsErrorKind::Transient, "Custom error");
        assert_eq!(err.kind(), WsErrorKind::Transient);
        assert_eq!(err.message(), "Custom error");
    }

    #[test]
    fn test_ws_error_with_source() {
        let io_err = std::io::Error::new(std::io::ErrorKind::ConnectionReset, "reset");
        let err = WsError::with_source(WsErrorKind::Transient, "Connection lost", io_err);

        assert!(err.is_transient());
        assert_eq!(err.message(), "Connection lost");
        assert!(err.source().is_some());
    }

    #[test]
    fn test_ws_error_transient_with_source() {
        let io_err = std::io::Error::new(std::io::ErrorKind::TimedOut, "timeout");
        let err = WsError::transient_with_source("Network timeout", io_err);

        assert!(err.is_transient());
        assert!(err.source().is_some());
    }

    #[test]
    fn test_ws_error_permanent_with_source() {
        let io_err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "denied");
        let err = WsError::permanent_with_source("Access denied", io_err);

        assert!(err.is_permanent());
        assert!(err.source().is_some());
    }

    #[test]
    fn test_ws_error_display() {
        let err = WsError::transient("Connection timeout");
        let display = err.to_string();
        assert!(display.contains("Transient"));
        assert!(display.contains("Connection timeout"));
    }

    #[test]
    fn test_ws_error_display_permanent() {
        let err = WsError::permanent("Auth failed");
        let display = err.to_string();
        assert!(display.contains("Permanent"));
        assert!(display.contains("Auth failed"));
    }

    #[test]
    fn test_ws_error_std_error_trait() {
        let io_err = std::io::Error::new(std::io::ErrorKind::ConnectionReset, "reset");
        let err = WsError::transient_with_source("Connection lost", io_err);

        // Test std::error::Error trait
        let std_err: &dyn std::error::Error = &err;
        assert!(std_err.source().is_some());
    }

    // ==================== WsError::from_tungstenite Tests ====================

    #[test]
    fn test_ws_error_from_tungstenite_connection_closed() {
        use tokio_tungstenite::tungstenite::Error as TungError;

        let tung_err = TungError::ConnectionClosed;
        let ws_err = WsError::from_tungstenite(&tung_err);

        assert!(ws_err.is_transient());
        assert!(ws_err.message().contains("Connection closed"));
    }

    #[test]
    fn test_ws_error_from_tungstenite_already_closed() {
        use tokio_tungstenite::tungstenite::Error as TungError;

        let tung_err = TungError::AlreadyClosed;
        let ws_err = WsError::from_tungstenite(&tung_err);

        assert!(ws_err.is_transient());
        assert!(ws_err.message().contains("already closed"));
    }

    #[test]
    fn test_ws_error_from_tungstenite_io_error() {
        use tokio_tungstenite::tungstenite::Error as TungError;

        let io_err = std::io::Error::new(std::io::ErrorKind::ConnectionReset, "connection reset");
        let tung_err = TungError::Io(io_err);
        let ws_err = WsError::from_tungstenite(&tung_err);

        assert!(ws_err.is_transient());
        assert!(ws_err.message().contains("IO error"));
    }

    #[test]
    fn test_ws_error_from_tungstenite_utf8_error() {
        use tokio_tungstenite::tungstenite::Error as TungError;

        let tung_err = TungError::Utf8("invalid utf8".to_string());
        let ws_err = WsError::from_tungstenite(&tung_err);

        assert!(ws_err.is_permanent());
        assert!(ws_err.message().contains("UTF-8"));
    }

    #[test]
    fn test_ws_error_from_tungstenite_attack_attempt() {
        use tokio_tungstenite::tungstenite::Error as TungError;

        let tung_err = TungError::AttackAttempt;
        let ws_err = WsError::from_tungstenite(&tung_err);

        assert!(ws_err.is_permanent());
        assert!(ws_err.message().contains("attack"));
    }

    // ==================== WsError::from_error Tests ====================

    #[test]
    fn test_ws_error_from_error_authentication() {
        let err = Error::authentication("Invalid API key");
        let ws_err = WsError::from_error(&err);

        assert!(ws_err.is_permanent());
        assert!(ws_err.message().contains("Authentication"));
    }

    #[test]
    fn test_ws_error_from_error_cancelled() {
        let err = Error::cancelled("Operation cancelled");
        let ws_err = WsError::from_error(&err);

        assert!(ws_err.is_permanent());
        assert!(ws_err.message().contains("cancelled"));
    }

    #[test]
    fn test_ws_error_from_error_resource_exhausted() {
        let err = Error::resource_exhausted("Max subscriptions reached");
        let ws_err = WsError::from_error(&err);

        assert!(ws_err.is_permanent());
        assert!(ws_err.message().contains("exhausted"));
    }

    #[test]
    fn test_ws_error_from_error_network() {
        let err = Error::network("Connection failed");
        let ws_err = WsError::from_error(&err);

        // Network errors are transient by default
        assert!(ws_err.is_transient());
    }

    // ==================== SubscriptionManager Tests ====================

    #[test]
    fn test_subscription_manager_new() {
        let manager = SubscriptionManager::new(50);
        assert_eq!(manager.max_subscriptions(), 50);
        assert_eq!(manager.count(), 0);
        assert_eq!(manager.remaining_capacity(), 50);
        assert!(manager.is_empty());
        assert!(!manager.is_full());
    }

    #[test]
    fn test_subscription_manager_with_default_capacity() {
        let manager = SubscriptionManager::with_default_capacity();
        assert_eq!(manager.max_subscriptions(), DEFAULT_MAX_SUBSCRIPTIONS);
        assert_eq!(manager.count(), 0);
        assert_eq!(manager.remaining_capacity(), DEFAULT_MAX_SUBSCRIPTIONS);
    }

    #[test]
    fn test_subscription_manager_default_trait() {
        let manager = SubscriptionManager::default();
        assert_eq!(manager.max_subscriptions(), DEFAULT_MAX_SUBSCRIPTIONS);
    }

    #[test]
    fn test_subscription_manager_try_add_success() {
        let manager = SubscriptionManager::new(10);

        let subscription = Subscription {
            channel: "ticker".to_string(),
            symbol: Some("BTC/USDT".to_string()),
            params: None,
        };

        let result = manager.try_add("ticker:BTC/USDT".to_string(), subscription);
        assert!(result.is_ok());
        assert_eq!(manager.count(), 1);
        assert_eq!(manager.remaining_capacity(), 9);
        assert!(manager.contains("ticker:BTC/USDT"));
    }

    #[test]
    fn test_subscription_manager_try_add_at_capacity() {
        let manager = SubscriptionManager::new(2);

        // Add first subscription
        let sub1 = Subscription {
            channel: "ticker".to_string(),
            symbol: Some("BTC/USDT".to_string()),
            params: None,
        };
        assert!(manager.try_add("ticker:BTC/USDT".to_string(), sub1).is_ok());

        // Add second subscription
        let sub2 = Subscription {
            channel: "ticker".to_string(),
            symbol: Some("ETH/USDT".to_string()),
            params: None,
        };
        assert!(manager.try_add("ticker:ETH/USDT".to_string(), sub2).is_ok());

        // Third subscription should fail
        let sub3 = Subscription {
            channel: "ticker".to_string(),
            symbol: Some("SOL/USDT".to_string()),
            params: None,
        };
        let result = manager.try_add("ticker:SOL/USDT".to_string(), sub3);
        assert!(result.is_err());

        // Verify error is ResourceExhausted
        let err = result.unwrap_err();
        assert!(err.as_resource_exhausted().is_some());
        assert!(err.to_string().contains("Maximum subscriptions"));

        // Count should still be 2
        assert_eq!(manager.count(), 2);
        assert_eq!(manager.remaining_capacity(), 0);
        assert!(manager.is_full());
    }

    #[test]
    fn test_subscription_manager_try_add_replace_existing() {
        let manager = SubscriptionManager::new(2);

        // Fill to capacity
        let sub1 = Subscription {
            channel: "ticker".to_string(),
            symbol: Some("BTC/USDT".to_string()),
            params: None,
        };
        manager
            .try_add("ticker:BTC/USDT".to_string(), sub1)
            .unwrap();

        let sub2 = Subscription {
            channel: "ticker".to_string(),
            symbol: Some("ETH/USDT".to_string()),
            params: None,
        };
        manager
            .try_add("ticker:ETH/USDT".to_string(), sub2)
            .unwrap();

        assert!(manager.is_full());

        // Replacing existing key should succeed even at capacity
        let sub1_updated = Subscription {
            channel: "ticker".to_string(),
            symbol: Some("BTC/USDT".to_string()),
            params: Some(HashMap::new()), // Different params
        };
        let result = manager.try_add("ticker:BTC/USDT".to_string(), sub1_updated);
        assert!(result.is_ok());

        // Count should still be 2
        assert_eq!(manager.count(), 2);
    }

    #[test]
    fn test_subscription_manager_remove() {
        let manager = SubscriptionManager::new(10);

        let subscription = Subscription {
            channel: "ticker".to_string(),
            symbol: Some("BTC/USDT".to_string()),
            params: None,
        };
        manager
            .try_add("ticker:BTC/USDT".to_string(), subscription)
            .unwrap();

        assert_eq!(manager.count(), 1);
        assert_eq!(manager.remaining_capacity(), 9);

        // Remove the subscription
        let removed = manager.remove("ticker:BTC/USDT");
        assert!(removed.is_some());

        // Verify removal
        assert_eq!(manager.count(), 0);
        assert_eq!(manager.remaining_capacity(), 10);
        assert!(!manager.contains("ticker:BTC/USDT"));
        assert!(manager.is_empty());
    }

    #[test]
    fn test_subscription_manager_remove_nonexistent() {
        let manager = SubscriptionManager::new(10);

        let removed = manager.remove("nonexistent");
        assert!(removed.is_none());
        assert_eq!(manager.count(), 0);
    }

    #[test]
    fn test_subscription_manager_remove_frees_slot() {
        let manager = SubscriptionManager::new(2);

        // Fill to capacity
        let sub1 = Subscription {
            channel: "ticker".to_string(),
            symbol: Some("BTC/USDT".to_string()),
            params: None,
        };
        manager
            .try_add("ticker:BTC/USDT".to_string(), sub1)
            .unwrap();

        let sub2 = Subscription {
            channel: "ticker".to_string(),
            symbol: Some("ETH/USDT".to_string()),
            params: None,
        };
        manager
            .try_add("ticker:ETH/USDT".to_string(), sub2)
            .unwrap();

        assert!(manager.is_full());

        // Remove one subscription
        manager.remove("ticker:BTC/USDT");

        // Should now be able to add a new subscription
        let sub3 = Subscription {
            channel: "ticker".to_string(),
            symbol: Some("SOL/USDT".to_string()),
            params: None,
        };
        let result = manager.try_add("ticker:SOL/USDT".to_string(), sub3);
        assert!(result.is_ok());
        assert_eq!(manager.count(), 2);
    }

    #[test]
    fn test_subscription_manager_clear() {
        let manager = SubscriptionManager::new(10);

        // Add some subscriptions
        for i in 0..5 {
            let sub = Subscription {
                channel: format!("channel{}", i),
                symbol: Some(format!("SYM{}/USDT", i)),
                params: None,
            };
            manager
                .try_add(format!("channel{}:SYM{}/USDT", i, i), sub)
                .unwrap();
        }

        assert_eq!(manager.count(), 5);

        // Clear all
        manager.clear();

        assert_eq!(manager.count(), 0);
        assert_eq!(manager.remaining_capacity(), 10);
        assert!(manager.is_empty());
    }

    #[test]
    fn test_subscription_manager_get() {
        let manager = SubscriptionManager::new(10);

        let subscription = Subscription {
            channel: "ticker".to_string(),
            symbol: Some("BTC/USDT".to_string()),
            params: None,
        };
        manager
            .try_add("ticker:BTC/USDT".to_string(), subscription)
            .unwrap();

        // Get existing
        let got = manager.get("ticker:BTC/USDT");
        assert!(got.is_some());
        assert_eq!(got.unwrap().channel, "ticker");

        // Get nonexistent
        let got = manager.get("nonexistent");
        assert!(got.is_none());
    }

    #[test]
    fn test_subscription_manager_collect_subscriptions() {
        let manager = SubscriptionManager::new(10);

        for i in 0..3 {
            let sub = Subscription {
                channel: format!("channel{}", i),
                symbol: Some(format!("SYM{}/USDT", i)),
                params: None,
            };
            manager
                .try_add(format!("channel{}:SYM{}/USDT", i, i), sub)
                .unwrap();
        }

        let collected = manager.collect_subscriptions();
        assert_eq!(collected.len(), 3);
    }

    #[test]
    fn test_subscription_manager_concurrent_operations() {
        use std::sync::Arc;
        use std::thread;

        let manager = Arc::new(SubscriptionManager::new(100));
        let mut handles = vec![];

        // Spawn threads that add subscriptions concurrently
        for i in 0..10 {
            let manager_clone = Arc::clone(&manager);
            let handle = thread::spawn(move || {
                for j in 0..5 {
                    let sub = Subscription {
                        channel: format!("channel{}_{}", i, j),
                        symbol: Some(format!("SYM{}_{}/USDT", i, j)),
                        params: None,
                    };
                    let _ = manager_clone
                        .try_add(format!("channel{}_{}:SYM{}_{}/USDT", i, j, i, j), sub);
                }
            });
            handles.push(handle);
        }

        // Wait for all threads
        for handle in handles {
            handle.join().unwrap();
        }

        // Should have 50 subscriptions (10 threads * 5 each)
        assert_eq!(manager.count(), 50);
        assert_eq!(manager.remaining_capacity(), 50);
    }

    #[test]
    fn test_subscription_manager_concurrent_add_remove() {
        use std::sync::Arc;
        use std::thread;

        let manager = Arc::new(SubscriptionManager::new(100));

        // Pre-populate with some subscriptions
        for i in 0..20 {
            let sub = Subscription {
                channel: format!("channel{}", i),
                symbol: Some(format!("SYM{}/USDT", i)),
                params: None,
            };
            manager
                .try_add(format!("channel{}:SYM{}/USDT", i, i), sub)
                .unwrap();
        }

        let mut handles = vec![];

        // Spawn threads that add new subscriptions
        for i in 20..30 {
            let manager_clone = Arc::clone(&manager);
            let handle = thread::spawn(move || {
                let sub = Subscription {
                    channel: format!("channel{}", i),
                    symbol: Some(format!("SYM{}/USDT", i)),
                    params: None,
                };
                let _ = manager_clone.try_add(format!("channel{}:SYM{}/USDT", i, i), sub);
            });
            handles.push(handle);
        }

        // Spawn threads that remove existing subscriptions
        for i in 0..10 {
            let manager_clone = Arc::clone(&manager);
            let handle = thread::spawn(move || {
                manager_clone.remove(&format!("channel{}:SYM{}/USDT", i, i));
            });
            handles.push(handle);
        }

        // Wait for all threads
        for handle in handles {
            handle.join().unwrap();
        }

        // Should have 20 subscriptions (20 initial + 10 added - 10 removed)
        assert_eq!(manager.count(), 20);
    }

    // ==================== CancellationToken Tests ====================

    #[tokio::test]
    async fn test_ws_client_set_cancel_token() {
        let config = WsConfig {
            url: "wss://example.com/ws".to_string(),
            ..Default::default()
        };

        let client = WsClient::new(config);

        // Initially no token
        assert!(client.get_cancel_token().await.is_none());

        // Set a token
        let token = CancellationToken::new();
        client.set_cancel_token(token.clone()).await;

        // Token should be set
        let retrieved = client.get_cancel_token().await;
        assert!(retrieved.is_some());
    }

    #[tokio::test]
    async fn test_ws_client_clear_cancel_token() {
        let config = WsConfig {
            url: "wss://example.com/ws".to_string(),
            ..Default::default()
        };

        let client = WsClient::new(config);

        // Set a token
        let token = CancellationToken::new();
        client.set_cancel_token(token).await;
        assert!(client.get_cancel_token().await.is_some());

        // Clear the token
        client.clear_cancel_token().await;
        assert!(client.get_cancel_token().await.is_none());
    }

    #[tokio::test]
    async fn test_cancellation_token_sharing() {
        // Test that CancellationToken is properly shared (cloning shares state)
        let token = CancellationToken::new();
        let token_clone = token.clone();

        // Neither should be cancelled initially
        assert!(!token.is_cancelled());
        assert!(!token_clone.is_cancelled());

        // Cancel the clone
        token_clone.cancel();

        // Both should be cancelled (shared state)
        assert!(token.is_cancelled());
        assert!(token_clone.is_cancelled());
    }

    #[tokio::test]
    async fn test_cancellation_token_child() {
        // Test child token behavior
        let parent = CancellationToken::new();
        let child = parent.child_token();

        // Neither should be cancelled initially
        assert!(!parent.is_cancelled());
        assert!(!child.is_cancelled());

        // Cancel the parent
        parent.cancel();

        // Both should be cancelled
        assert!(parent.is_cancelled());
        assert!(child.is_cancelled());
    }

    #[tokio::test]
    async fn test_cancellation_token_child_independent() {
        // Test that cancelling child doesn't cancel parent
        let parent = CancellationToken::new();
        let child = parent.child_token();

        // Cancel the child
        child.cancel();

        // Child should be cancelled, parent should not
        assert!(!parent.is_cancelled());
        assert!(child.is_cancelled());
    }
}
