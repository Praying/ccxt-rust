//! Circuit Breaker module for preventing cascading failures.
//!
//! This module implements the Circuit Breaker pattern to protect against
//! continuously failing endpoints. When failures exceed a threshold, the
//! circuit "opens" and rejects requests immediately, allowing the system
//! to recover.
//!
//! # States
//!
//! The circuit breaker has three states:
//!
//! - **Closed**: Normal operation, requests are allowed
//! - **Open**: Blocking requests due to failures, requests are rejected immediately
//! - **HalfOpen**: Testing if the service has recovered, allows limited requests
//!
//! # State Transitions
//!
//! ```text
//! ┌─────────┐  failure_count >= threshold  ┌──────┐
//! │ Closed  │ ──────────────────────────▶  │ Open │
//! └─────────┘                              └──────┘
//!      ▲                                       │
//!      │ test request succeeds                 │ reset_timeout elapsed
//!      │                                       ▼
//!      │                                  ┌──────────┐
//!      └────────────────────────────────  │ HalfOpen │
//!                                         └──────────┘
//!                                              │
//!                                              │ test request fails
//!                                              ▼
//!                                         ┌──────┐
//!                                         │ Open │
//!                                         └──────┘
//! ```
//!
//! # Example
//!
//! ```rust
//! use ccxt_core::circuit_breaker::{CircuitBreaker, CircuitBreakerConfig, CircuitState};
//! use std::time::Duration;
//!
//! // Create a circuit breaker with custom configuration
//! let config = CircuitBreakerConfig {
//!     failure_threshold: 5,
//!     reset_timeout: Duration::from_secs(30),
//!     success_threshold: 1,
//! };
//! let breaker = CircuitBreaker::new(config);
//!
//! // Check if request is allowed
//! if breaker.allow_request().is_ok() {
//!     // Make the request
//!     // On success:
//!     breaker.record_success();
//!     // On failure:
//!     // breaker.record_failure();
//! }
//! ```

use crate::error::{ConfigValidationError, Error, Result, ValidationResult};
use std::sync::atomic::{AtomicI64, AtomicU8, AtomicU32, Ordering};
use std::time::Duration;
use tokio::sync::mpsc;
use tracing::{debug, info, warn};

/// Circuit breaker states.
///
/// The circuit breaker transitions between these states based on
/// request success/failure patterns.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum CircuitState {
    /// Normal operation, requests are allowed.
    /// Failures are counted, and if they exceed the threshold,
    /// the circuit transitions to Open.
    Closed = 0,

    /// Blocking requests due to failures.
    /// All requests are rejected immediately with a `ResourceExhausted` error.
    /// After `reset_timeout` elapses, transitions to HalfOpen.
    Open = 1,

    /// Testing if the service has recovered.
    /// Allows a limited number of test requests.
    /// If successful, transitions to Closed; if failed, transitions back to Open.
    HalfOpen = 2,
}

impl CircuitState {
    /// Converts a u8 value to a CircuitState.
    fn from_u8(value: u8) -> Self {
        match value {
            1 => CircuitState::Open,
            2 => CircuitState::HalfOpen,
            _ => CircuitState::Closed, // Default to Closed for invalid values (including 0)
        }
    }
}

/// Circuit breaker configuration.
///
/// # Example
///
/// ```rust
/// use ccxt_core::circuit_breaker::CircuitBreakerConfig;
/// use std::time::Duration;
///
/// let config = CircuitBreakerConfig {
///     failure_threshold: 5,
///     reset_timeout: Duration::from_secs(30),
///     success_threshold: 1,
/// };
/// ```
#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    /// Number of consecutive failures before opening the circuit.
    ///
    /// When the failure count reaches this threshold, the circuit
    /// transitions from Closed to Open state.
    ///
    /// Default: 5
    pub failure_threshold: u32,

    /// Time to wait in Open state before transitioning to HalfOpen.
    ///
    /// After this duration elapses, the circuit breaker will allow
    /// a test request to check if the service has recovered.
    ///
    /// Default: 30 seconds
    pub reset_timeout: Duration,

    /// Number of consecutive successes needed to close the circuit from HalfOpen.
    ///
    /// When in HalfOpen state, this many successful requests are required
    /// before transitioning back to Closed state.
    ///
    /// Default: 1
    pub success_threshold: u32,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            reset_timeout: Duration::from_secs(30),
            success_threshold: 1,
        }
    }
}

impl CircuitBreakerConfig {
    /// Creates a new circuit breaker configuration with the given parameters.
    ///
    /// # Arguments
    ///
    /// * `failure_threshold` - Number of failures before opening the circuit
    /// * `reset_timeout` - Time to wait before testing recovery
    /// * `success_threshold` - Number of successes needed to close the circuit
    pub fn new(failure_threshold: u32, reset_timeout: Duration, success_threshold: u32) -> Self {
        Self {
            failure_threshold,
            reset_timeout,
            success_threshold,
        }
    }

    /// Validates the circuit breaker configuration.
    ///
    /// # Returns
    ///
    /// Returns `Ok(ValidationResult)` if the configuration is valid.
    /// Returns `Err(ConfigValidationError)` if the configuration is invalid.
    ///
    /// # Validation Rules
    ///
    /// - `failure_threshold` must be > 0
    /// - `success_threshold` must be > 0
    /// - `reset_timeout` must be >= 1 second
    ///
    /// # Example
    ///
    /// ```rust
    /// use ccxt_core::circuit_breaker::CircuitBreakerConfig;
    ///
    /// let config = CircuitBreakerConfig::default();
    /// assert!(config.validate().is_ok());
    /// ```
    pub fn validate(&self) -> std::result::Result<ValidationResult, ConfigValidationError> {
        let mut warnings = Vec::new();

        if self.failure_threshold == 0 {
            return Err(ConfigValidationError::invalid(
                "failure_threshold",
                "failure_threshold must be greater than 0",
            ));
        }

        if self.success_threshold == 0 {
            return Err(ConfigValidationError::invalid(
                "success_threshold",
                "success_threshold must be greater than 0",
            ));
        }

        if self.reset_timeout < Duration::from_secs(1) {
            warnings.push(format!(
                "reset_timeout {:?} is very short, may cause rapid state transitions",
                self.reset_timeout
            ));
        }

        Ok(ValidationResult::with_warnings(warnings))
    }
}

/// Circuit breaker events for observability.
///
/// These events are emitted when the circuit breaker state changes
/// or when significant actions occur.
#[derive(Debug, Clone)]
pub enum CircuitBreakerEvent {
    /// The circuit breaker state has changed.
    StateChanged {
        /// The previous state
        from: CircuitState,
        /// The new state
        to: CircuitState,
    },

    /// A request was rejected because the circuit is open.
    RequestRejected {
        /// The reason for rejection
        reason: String,
    },

    /// A failure was recorded.
    FailureRecorded {
        /// The current failure count
        count: u32,
    },

    /// A success was recorded.
    SuccessRecorded {
        /// The current success count (in HalfOpen state)
        count: u32,
    },
}

/// Circuit breaker for preventing cascading failures.
///
/// The circuit breaker monitors request success/failure patterns and
/// automatically blocks requests to failing endpoints, allowing the
/// system to recover.
///
/// # Thread Safety
///
/// This implementation uses atomic operations for all state management,
/// making it safe to use from multiple threads without external locking.
///
/// # Example
///
/// ```rust
/// use ccxt_core::circuit_breaker::{CircuitBreaker, CircuitBreakerConfig};
///
/// let breaker = CircuitBreaker::new(CircuitBreakerConfig::default());
///
/// // Before making a request
/// if let Err(e) = breaker.allow_request() {
///     // Circuit is open, handle the error
///     println!("Circuit is open: {}", e);
/// } else {
///     // Make the request
///     let success = true; // result of the request
///     if success {
///         breaker.record_success();
///     } else {
///         breaker.record_failure();
///     }
/// }
/// ```
#[derive(Debug)]
pub struct CircuitBreaker {
    /// Current state (stored as u8 for atomic operations)
    state: AtomicU8,

    /// Number of consecutive failures in Closed state
    failure_count: AtomicU32,

    /// Number of consecutive successes in HalfOpen state
    success_count: AtomicU32,

    /// Timestamp of the last failure (milliseconds since Unix epoch)
    last_failure_time: AtomicI64,

    /// Configuration
    config: CircuitBreakerConfig,

    /// Optional event sender for observability
    event_tx: Option<mpsc::UnboundedSender<CircuitBreakerEvent>>,
}

impl CircuitBreaker {
    /// Creates a new circuit breaker with the given configuration.
    ///
    /// # Arguments
    ///
    /// * `config` - Circuit breaker configuration
    ///
    /// # Example
    ///
    /// ```rust
    /// use ccxt_core::circuit_breaker::{CircuitBreaker, CircuitBreakerConfig};
    ///
    /// let breaker = CircuitBreaker::new(CircuitBreakerConfig::default());
    /// ```
    pub fn new(config: CircuitBreakerConfig) -> Self {
        Self {
            state: AtomicU8::new(CircuitState::Closed as u8),
            failure_count: AtomicU32::new(0),
            success_count: AtomicU32::new(0),
            last_failure_time: AtomicI64::new(0),
            config,
            event_tx: None,
        }
    }

    /// Creates a new circuit breaker with an event channel for observability.
    ///
    /// # Arguments
    ///
    /// * `config` - Circuit breaker configuration
    /// * `event_tx` - Channel sender for circuit breaker events
    ///
    /// # Example
    ///
    /// ```rust
    /// use ccxt_core::circuit_breaker::{CircuitBreaker, CircuitBreakerConfig, CircuitBreakerEvent};
    /// use tokio::sync::mpsc;
    ///
    /// let (tx, mut rx) = mpsc::unbounded_channel::<CircuitBreakerEvent>();
    /// let breaker = CircuitBreaker::with_events(CircuitBreakerConfig::default(), tx);
    ///
    /// // Events will be sent to the channel when state changes occur
    /// ```
    pub fn with_events(
        config: CircuitBreakerConfig,
        event_tx: mpsc::UnboundedSender<CircuitBreakerEvent>,
    ) -> Self {
        Self {
            state: AtomicU8::new(CircuitState::Closed as u8),
            failure_count: AtomicU32::new(0),
            success_count: AtomicU32::new(0),
            last_failure_time: AtomicI64::new(0),
            config,
            event_tx: Some(event_tx),
        }
    }

    /// Checks if a request should be allowed.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if the request is allowed.
    /// Returns `Err(Error::ResourceExhausted)` if the circuit is open.
    ///
    /// # State Transitions
    ///
    /// - **Closed**: Always allows requests
    /// - **Open**: Checks if `reset_timeout` has elapsed; if so, transitions to HalfOpen
    /// - **HalfOpen**: Allows requests (for testing recovery)
    ///
    /// # Example
    ///
    /// ```rust
    /// use ccxt_core::circuit_breaker::{CircuitBreaker, CircuitBreakerConfig};
    ///
    /// let breaker = CircuitBreaker::new(CircuitBreakerConfig::default());
    ///
    /// match breaker.allow_request() {
    ///     Ok(()) => println!("Request allowed"),
    ///     Err(e) => println!("Request blocked: {}", e),
    /// }
    /// ```
    #[allow(clippy::cast_possible_truncation)]
    pub fn allow_request(&self) -> Result<()> {
        let state = self.state();

        match state {
            CircuitState::Closed => {
                debug!("Circuit breaker: Closed state, allowing request");
                Ok(())
            }
            CircuitState::Open => {
                // Check if reset timeout has elapsed
                let last_failure = self.last_failure_time.load(Ordering::Acquire);
                let now = chrono::Utc::now().timestamp_millis();
                let elapsed_ms = (now - last_failure).max(0) as u64;
                let elapsed = Duration::from_millis(elapsed_ms);

                if elapsed >= self.config.reset_timeout {
                    // Safe truncation: reset_timeout is typically seconds/minutes, not years
                    let reset_timeout_ms =
                        self.config
                            .reset_timeout
                            .as_millis()
                            .min(u128::from(u64::MAX)) as u64;
                    info!(
                        elapsed_ms = elapsed_ms,
                        reset_timeout_ms = reset_timeout_ms,
                        "Circuit breaker: Reset timeout elapsed, transitioning to HalfOpen"
                    );
                    self.transition_to(CircuitState::HalfOpen);
                    Ok(())
                } else {
                    // Safe truncation: reset_timeout is typically seconds/minutes, not years
                    let reset_timeout_ms =
                        self.config
                            .reset_timeout
                            .as_millis()
                            .min(u128::from(u64::MAX)) as u64;
                    let remaining_ms = reset_timeout_ms.saturating_sub(elapsed_ms);
                    warn!(
                        remaining_ms = remaining_ms,
                        "Circuit breaker: Open state, rejecting request"
                    );
                    self.emit_event(CircuitBreakerEvent::RequestRejected {
                        reason: format!("Circuit breaker is open, retry after {remaining_ms}ms"),
                    });
                    Err(Error::resource_exhausted("Circuit breaker is open"))
                }
            }
            CircuitState::HalfOpen => {
                debug!("Circuit breaker: HalfOpen state, allowing test request");
                Ok(())
            }
        }
    }

    /// Records a successful request.
    ///
    /// # State Transitions
    ///
    /// - **Closed**: Resets the failure count to 0
    /// - **HalfOpen**: Increments success count; if it reaches `success_threshold`,
    ///   transitions to Closed
    /// - **Open**: No effect
    ///
    /// # Example
    ///
    /// ```rust
    /// use ccxt_core::circuit_breaker::{CircuitBreaker, CircuitBreakerConfig};
    ///
    /// let breaker = CircuitBreaker::new(CircuitBreakerConfig::default());
    ///
    /// // After a successful request
    /// breaker.record_success();
    /// ```
    pub fn record_success(&self) {
        let state = self.state();

        match state {
            CircuitState::Closed => {
                // Reset failure count on success
                self.failure_count.store(0, Ordering::Release);
                debug!("Circuit breaker: Success recorded in Closed state, failure count reset");
            }
            CircuitState::HalfOpen => {
                let count = self.success_count.fetch_add(1, Ordering::AcqRel) + 1;
                debug!(
                    success_count = count,
                    threshold = self.config.success_threshold,
                    "Circuit breaker: Success recorded in HalfOpen state"
                );

                self.emit_event(CircuitBreakerEvent::SuccessRecorded { count });

                if count >= self.config.success_threshold {
                    info!(
                        success_count = count,
                        "Circuit breaker: Success threshold reached, transitioning to Closed"
                    );
                    self.transition_to(CircuitState::Closed);
                    self.failure_count.store(0, Ordering::Release);
                    self.success_count.store(0, Ordering::Release);
                }
            }
            CircuitState::Open => {
                // No effect in Open state
                debug!("Circuit breaker: Success recorded in Open state (no effect)");
            }
        }
    }

    /// Records a failed request.
    ///
    /// # State Transitions
    ///
    /// - **Closed**: Increments failure count; if it reaches `failure_threshold`,
    ///   transitions to Open
    /// - **HalfOpen**: Transitions immediately to Open (test request failed)
    /// - **Open**: No effect
    ///
    /// # Example
    ///
    /// ```rust
    /// use ccxt_core::circuit_breaker::{CircuitBreaker, CircuitBreakerConfig};
    ///
    /// let breaker = CircuitBreaker::new(CircuitBreakerConfig::default());
    ///
    /// // After a failed request
    /// breaker.record_failure();
    /// ```
    pub fn record_failure(&self) {
        let state = self.state();

        match state {
            CircuitState::Closed => {
                let count = self.failure_count.fetch_add(1, Ordering::AcqRel) + 1;
                debug!(
                    failure_count = count,
                    threshold = self.config.failure_threshold,
                    "Circuit breaker: Failure recorded in Closed state"
                );

                self.emit_event(CircuitBreakerEvent::FailureRecorded { count });

                if count >= self.config.failure_threshold {
                    warn!(
                        failure_count = count,
                        "Circuit breaker: Failure threshold reached, transitioning to Open"
                    );
                    self.transition_to(CircuitState::Open);
                    self.last_failure_time
                        .store(chrono::Utc::now().timestamp_millis(), Ordering::Release);
                }
            }
            CircuitState::HalfOpen => {
                warn!("Circuit breaker: Failure in HalfOpen state, transitioning back to Open");
                self.transition_to(CircuitState::Open);
                self.last_failure_time
                    .store(chrono::Utc::now().timestamp_millis(), Ordering::Release);
                self.success_count.store(0, Ordering::Release);
            }
            CircuitState::Open => {
                // No effect in Open state
                debug!("Circuit breaker: Failure recorded in Open state (no effect)");
            }
        }
    }

    /// Returns the current state of the circuit breaker.
    ///
    /// # Example
    ///
    /// ```rust
    /// use ccxt_core::circuit_breaker::{CircuitBreaker, CircuitBreakerConfig, CircuitState};
    ///
    /// let breaker = CircuitBreaker::new(CircuitBreakerConfig::default());
    /// assert_eq!(breaker.state(), CircuitState::Closed);
    /// ```
    pub fn state(&self) -> CircuitState {
        CircuitState::from_u8(self.state.load(Ordering::Acquire))
    }

    /// Returns the current failure count.
    ///
    /// This is primarily useful for monitoring and debugging.
    pub fn failure_count(&self) -> u32 {
        self.failure_count.load(Ordering::Acquire)
    }

    /// Returns the current success count (in HalfOpen state).
    ///
    /// This is primarily useful for monitoring and debugging.
    pub fn success_count(&self) -> u32 {
        self.success_count.load(Ordering::Acquire)
    }

    /// Returns a reference to the configuration.
    pub fn config(&self) -> &CircuitBreakerConfig {
        &self.config
    }

    /// Resets the circuit breaker to its initial state (Closed).
    ///
    /// This can be useful for testing or manual intervention.
    ///
    /// # Example
    ///
    /// ```rust
    /// use ccxt_core::circuit_breaker::{CircuitBreaker, CircuitBreakerConfig};
    ///
    /// let breaker = CircuitBreaker::new(CircuitBreakerConfig::default());
    /// // ... some operations that opened the circuit ...
    /// breaker.reset();
    /// // Circuit is now Closed again
    /// ```
    pub fn reset(&self) {
        let old_state = self.state();
        self.state
            .store(CircuitState::Closed as u8, Ordering::Release);
        self.failure_count.store(0, Ordering::Release);
        self.success_count.store(0, Ordering::Release);
        self.last_failure_time.store(0, Ordering::Release);

        if old_state != CircuitState::Closed {
            info!(
                from = ?old_state,
                "Circuit breaker: Manual reset to Closed state"
            );
            self.emit_event(CircuitBreakerEvent::StateChanged {
                from: old_state,
                to: CircuitState::Closed,
            });
        }
    }

    /// Transitions to a new state and emits an event.
    fn transition_to(&self, new_state: CircuitState) {
        let old_state = self.state();
        self.state.store(new_state as u8, Ordering::Release);

        self.emit_event(CircuitBreakerEvent::StateChanged {
            from: old_state,
            to: new_state,
        });
    }

    /// Emits an event if an event channel is configured.
    fn emit_event(&self, event: CircuitBreakerEvent) {
        if let Some(ref tx) = self.event_tx {
            // Ignore send errors (receiver may have been dropped)
            let _ = tx.send(event);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_circuit_state_from_u8() {
        assert_eq!(CircuitState::from_u8(0), CircuitState::Closed);
        assert_eq!(CircuitState::from_u8(1), CircuitState::Open);
        assert_eq!(CircuitState::from_u8(2), CircuitState::HalfOpen);
        assert_eq!(CircuitState::from_u8(255), CircuitState::Closed); // Invalid defaults to Closed
    }

    #[test]
    fn test_circuit_breaker_config_default() {
        let config = CircuitBreakerConfig::default();
        assert_eq!(config.failure_threshold, 5);
        assert_eq!(config.reset_timeout, Duration::from_secs(30));
        assert_eq!(config.success_threshold, 1);
    }

    #[test]
    fn test_circuit_breaker_config_new() {
        let config = CircuitBreakerConfig::new(10, Duration::from_secs(60), 2);
        assert_eq!(config.failure_threshold, 10);
        assert_eq!(config.reset_timeout, Duration::from_secs(60));
        assert_eq!(config.success_threshold, 2);
    }

    #[test]
    fn test_circuit_breaker_config_validate_default() {
        let config = CircuitBreakerConfig::default();
        let result = config.validate();
        assert!(result.is_ok());
        assert!(result.unwrap().warnings.is_empty());
    }

    #[test]
    fn test_circuit_breaker_config_validate_zero_failure_threshold() {
        let config = CircuitBreakerConfig {
            failure_threshold: 0,
            ..Default::default()
        };
        let result = config.validate();
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.field_name(), "failure_threshold");
    }

    #[test]
    fn test_circuit_breaker_config_validate_zero_success_threshold() {
        let config = CircuitBreakerConfig {
            success_threshold: 0,
            ..Default::default()
        };
        let result = config.validate();
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.field_name(), "success_threshold");
    }

    #[test]
    fn test_circuit_breaker_config_validate_short_reset_timeout_warning() {
        let config = CircuitBreakerConfig {
            reset_timeout: Duration::from_millis(500),
            ..Default::default()
        };
        let result = config.validate();
        assert!(result.is_ok());
        let validation_result = result.unwrap();
        assert!(!validation_result.warnings.is_empty());
        assert!(validation_result.warnings[0].contains("reset_timeout"));
    }

    #[test]
    fn test_circuit_breaker_initial_state() {
        let breaker = CircuitBreaker::new(CircuitBreakerConfig::default());
        assert_eq!(breaker.state(), CircuitState::Closed);
        assert_eq!(breaker.failure_count(), 0);
        assert_eq!(breaker.success_count(), 0);
    }

    #[test]
    fn test_circuit_breaker_allow_request_closed() {
        let breaker = CircuitBreaker::new(CircuitBreakerConfig::default());
        assert!(breaker.allow_request().is_ok());
    }

    #[test]
    fn test_circuit_breaker_record_success_closed() {
        let breaker = CircuitBreaker::new(CircuitBreakerConfig::default());

        // Record some failures first
        breaker.record_failure();
        breaker.record_failure();
        assert_eq!(breaker.failure_count(), 2);

        // Success should reset failure count
        breaker.record_success();
        assert_eq!(breaker.failure_count(), 0);
        assert_eq!(breaker.state(), CircuitState::Closed);
    }

    #[test]
    fn test_circuit_breaker_transition_to_open() {
        let config = CircuitBreakerConfig {
            failure_threshold: 3,
            ..Default::default()
        };
        let breaker = CircuitBreaker::new(config);

        // Record failures up to threshold
        breaker.record_failure();
        assert_eq!(breaker.state(), CircuitState::Closed);
        breaker.record_failure();
        assert_eq!(breaker.state(), CircuitState::Closed);
        breaker.record_failure();

        // Should transition to Open after reaching threshold
        assert_eq!(breaker.state(), CircuitState::Open);
    }

    #[test]
    fn test_circuit_breaker_reject_when_open() {
        let config = CircuitBreakerConfig {
            failure_threshold: 1,
            reset_timeout: Duration::from_secs(60), // Long timeout
            ..Default::default()
        };
        let breaker = CircuitBreaker::new(config);

        // Open the circuit
        breaker.record_failure();
        assert_eq!(breaker.state(), CircuitState::Open);

        // Request should be rejected
        let result = breaker.allow_request();
        assert!(result.is_err());

        if let Err(e) = result {
            assert!(e.as_resource_exhausted().is_some());
        }
    }

    #[test]
    fn test_circuit_breaker_transition_to_half_open() {
        let config = CircuitBreakerConfig {
            failure_threshold: 1,
            reset_timeout: Duration::from_millis(10), // Very short timeout for testing
            ..Default::default()
        };
        let breaker = CircuitBreaker::new(config);

        // Open the circuit
        breaker.record_failure();
        assert_eq!(breaker.state(), CircuitState::Open);

        // Wait for reset timeout
        std::thread::sleep(Duration::from_millis(20));

        // Request should be allowed and state should transition to HalfOpen
        assert!(breaker.allow_request().is_ok());
        assert_eq!(breaker.state(), CircuitState::HalfOpen);
    }

    #[test]
    fn test_circuit_breaker_half_open_success_closes() {
        let config = CircuitBreakerConfig {
            failure_threshold: 1,
            reset_timeout: Duration::from_millis(10),
            success_threshold: 1,
        };
        let breaker = CircuitBreaker::new(config);

        // Open the circuit
        breaker.record_failure();
        assert_eq!(breaker.state(), CircuitState::Open);

        // Wait for reset timeout
        std::thread::sleep(Duration::from_millis(20));

        // Transition to HalfOpen
        assert!(breaker.allow_request().is_ok());
        assert_eq!(breaker.state(), CircuitState::HalfOpen);

        // Success should close the circuit
        breaker.record_success();
        assert_eq!(breaker.state(), CircuitState::Closed);
        assert_eq!(breaker.failure_count(), 0);
        assert_eq!(breaker.success_count(), 0);
    }

    #[test]
    fn test_circuit_breaker_half_open_failure_opens() {
        let config = CircuitBreakerConfig {
            failure_threshold: 1,
            reset_timeout: Duration::from_millis(10),
            success_threshold: 1,
        };
        let breaker = CircuitBreaker::new(config);

        // Open the circuit
        breaker.record_failure();
        assert_eq!(breaker.state(), CircuitState::Open);

        // Wait for reset timeout
        std::thread::sleep(Duration::from_millis(20));

        // Transition to HalfOpen
        assert!(breaker.allow_request().is_ok());
        assert_eq!(breaker.state(), CircuitState::HalfOpen);

        // Failure should reopen the circuit
        breaker.record_failure();
        assert_eq!(breaker.state(), CircuitState::Open);
    }

    #[test]
    fn test_circuit_breaker_multiple_successes_required() {
        let config = CircuitBreakerConfig {
            failure_threshold: 1,
            reset_timeout: Duration::from_millis(10),
            success_threshold: 3, // Require 3 successes
        };
        let breaker = CircuitBreaker::new(config);

        // Open the circuit
        breaker.record_failure();
        std::thread::sleep(Duration::from_millis(20));
        assert!(breaker.allow_request().is_ok());
        assert_eq!(breaker.state(), CircuitState::HalfOpen);

        // First success - still HalfOpen
        breaker.record_success();
        assert_eq!(breaker.state(), CircuitState::HalfOpen);
        assert_eq!(breaker.success_count(), 1);

        // Second success - still HalfOpen
        breaker.record_success();
        assert_eq!(breaker.state(), CircuitState::HalfOpen);
        assert_eq!(breaker.success_count(), 2);

        // Third success - should close
        breaker.record_success();
        assert_eq!(breaker.state(), CircuitState::Closed);
    }

    #[test]
    fn test_circuit_breaker_reset() {
        let config = CircuitBreakerConfig {
            failure_threshold: 1,
            ..Default::default()
        };
        let breaker = CircuitBreaker::new(config);

        // Open the circuit
        breaker.record_failure();
        assert_eq!(breaker.state(), CircuitState::Open);

        // Reset should close the circuit
        breaker.reset();
        assert_eq!(breaker.state(), CircuitState::Closed);
        assert_eq!(breaker.failure_count(), 0);
        assert_eq!(breaker.success_count(), 0);
    }

    #[tokio::test]
    async fn test_circuit_breaker_with_events() {
        let (tx, mut rx) = mpsc::unbounded_channel::<CircuitBreakerEvent>();
        let config = CircuitBreakerConfig {
            failure_threshold: 2,
            reset_timeout: Duration::from_millis(10),
            success_threshold: 1,
        };
        let breaker = CircuitBreaker::with_events(config, tx);

        // Record failures
        breaker.record_failure();
        breaker.record_failure(); // This should trigger state change

        // Check events
        let event1 = rx.recv().await.unwrap();
        assert!(matches!(
            event1,
            CircuitBreakerEvent::FailureRecorded { count: 1 }
        ));

        let event2 = rx.recv().await.unwrap();
        assert!(matches!(
            event2,
            CircuitBreakerEvent::FailureRecorded { count: 2 }
        ));

        let event3 = rx.recv().await.unwrap();
        assert!(matches!(
            event3,
            CircuitBreakerEvent::StateChanged {
                from: CircuitState::Closed,
                to: CircuitState::Open
            }
        ));
    }

    #[test]
    fn test_circuit_breaker_thread_safety() {
        use std::sync::Arc;
        use std::thread;

        let breaker = Arc::new(CircuitBreaker::new(CircuitBreakerConfig {
            failure_threshold: 100,
            ..Default::default()
        }));

        let mut handles = vec![];

        // Spawn multiple threads recording failures
        for _ in 0..10 {
            let breaker_clone = Arc::clone(&breaker);
            handles.push(thread::spawn(move || {
                for _ in 0..10 {
                    breaker_clone.record_failure();
                }
            }));
        }

        // Wait for all threads to complete
        for handle in handles {
            handle.join().unwrap();
        }

        // Should have recorded 100 failures and transitioned to Open
        assert_eq!(breaker.state(), CircuitState::Open);
    }
}
