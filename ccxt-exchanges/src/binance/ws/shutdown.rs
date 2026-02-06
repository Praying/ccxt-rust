//! WebSocket shutdown state management for Binance.
//!
//! Tracks whether the WebSocket client is shutting down or has completed shutdown.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

/// Tracks the shutdown lifecycle of a WebSocket connection.
pub struct ShutdownState {
    pub is_shutting_down: Arc<AtomicBool>,
    pub shutdown_complete: Arc<AtomicBool>,
}

impl ShutdownState {
    /// Creates a new shutdown state (not shutting down).
    pub fn new() -> Self {
        Self {
            is_shutting_down: Arc::new(AtomicBool::new(false)),
            shutdown_complete: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Marks the connection as shutting down.
    pub fn begin_shutdown(&self) {
        self.is_shutting_down.store(true, Ordering::Release);
    }

    /// Marks the shutdown as complete.
    pub fn complete_shutdown(&self) {
        self.shutdown_complete.store(true, Ordering::Release);
    }

    /// Returns true if shutdown has been initiated.
    pub fn is_shutting_down(&self) -> bool {
        self.is_shutting_down.load(Ordering::Acquire)
    }

    /// Returns true if shutdown has completed.
    pub fn is_complete(&self) -> bool {
        self.shutdown_complete.load(Ordering::Acquire)
    }

    /// Returns true if shutdown was not properly initiated (for Drop warning).
    pub fn needs_shutdown_warning(&self) -> bool {
        !self.is_complete() && !self.is_shutting_down()
    }
}

impl Default for ShutdownState {
    fn default() -> Self {
        Self::new()
    }
}
