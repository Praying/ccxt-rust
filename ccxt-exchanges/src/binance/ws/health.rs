//! WebSocket health tracking for Binance.
//!
//! Tracks connection health metrics like message counts, latency, and uptime.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

/// Tracks WebSocket connection health metrics.
pub struct WsHealthTracker {
    pub messages_received: Arc<AtomicU64>,
    pub messages_dropped: Arc<AtomicU64>,
    pub last_message_time: Arc<AtomicU64>,
    pub connection_start_time: Arc<AtomicU64>,
}

impl WsHealthTracker {
    /// Creates a new health tracker with the current time as connection start.
    pub fn new() -> Self {
        Self {
            messages_received: Arc::new(AtomicU64::new(0)),
            messages_dropped: Arc::new(AtomicU64::new(0)),
            last_message_time: Arc::new(AtomicU64::new(0)),
            connection_start_time: Arc::new(AtomicU64::new(
                chrono::Utc::now().timestamp_millis() as u64
            )),
        }
    }

    /// Records a successfully received message.
    pub fn record_received(&self) {
        self.messages_received.fetch_add(1, Ordering::Relaxed);
        self.last_message_time.store(
            chrono::Utc::now().timestamp_millis() as u64,
            Ordering::Relaxed,
        );
    }

    /// Records a dropped message.
    pub fn record_dropped(&self) {
        self.messages_dropped.fetch_add(1, Ordering::Relaxed);
    }

    /// Returns the total number of messages received.
    pub fn total_received(&self) -> u64 {
        self.messages_received.load(Ordering::Relaxed)
    }

    /// Returns the total number of messages dropped.
    pub fn total_dropped(&self) -> u64 {
        self.messages_dropped.load(Ordering::Relaxed)
    }

    /// Returns the connection uptime in milliseconds.
    pub fn uptime_ms(&self) -> u64 {
        let now = chrono::Utc::now().timestamp_millis() as u64;
        now.saturating_sub(self.connection_start_time.load(Ordering::Relaxed))
    }
}

impl Default for WsHealthTracker {
    fn default() -> Self {
        Self::new()
    }
}
