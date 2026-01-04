//! WebSocket connection state and statistics.

use std::sync::atomic::{AtomicI64, AtomicU32, AtomicU64, Ordering};

/// WebSocket connection state.
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

/// WebSocket connection statistics (lock-free).
#[derive(Debug)]
pub struct WsStats {
    messages_received: AtomicU64,
    messages_sent: AtomicU64,
    bytes_received: AtomicU64,
    bytes_sent: AtomicU64,
    last_message_time: AtomicI64,
    last_ping_time: AtomicI64,
    last_pong_time: AtomicI64,
    connected_at: AtomicI64,
    reconnect_attempts: AtomicU32,
}

impl WsStats {
    /// Creates a new `WsStats` instance with all counters initialized to zero.
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
    pub fn record_received(&self, bytes: u64) {
        self.messages_received.fetch_add(1, Ordering::Relaxed);
        self.bytes_received.fetch_add(bytes, Ordering::Relaxed);
        self.last_message_time
            .store(chrono::Utc::now().timestamp_millis(), Ordering::Relaxed);
    }

    /// Records a sent message.
    pub fn record_sent(&self, bytes: u64) {
        self.messages_sent.fetch_add(1, Ordering::Relaxed);
        self.bytes_sent.fetch_add(bytes, Ordering::Relaxed);
    }

    /// Records a ping sent.
    pub fn record_ping(&self) {
        self.last_ping_time
            .store(chrono::Utc::now().timestamp_millis(), Ordering::Relaxed);
    }

    /// Records a pong received.
    pub fn record_pong(&self) {
        self.last_pong_time
            .store(chrono::Utc::now().timestamp_millis(), Ordering::Relaxed);
    }

    /// Records a connection established.
    pub fn record_connected(&self) {
        self.connected_at
            .store(chrono::Utc::now().timestamp_millis(), Ordering::Relaxed);
    }

    /// Increments the reconnection attempt counter.
    pub fn increment_reconnect_attempts(&self) -> u32 {
        self.reconnect_attempts.fetch_add(1, Ordering::Relaxed) + 1
    }

    /// Resets the reconnection attempt counter to zero.
    pub fn reset_reconnect_attempts(&self) {
        self.reconnect_attempts.store(0, Ordering::Relaxed);
    }

    /// Returns the last pong timestamp.
    pub fn last_pong_time(&self) -> i64 {
        self.last_pong_time.load(Ordering::Relaxed)
    }

    /// Returns the last ping timestamp.
    pub fn last_ping_time(&self) -> i64 {
        self.last_ping_time.load(Ordering::Relaxed)
    }

    /// Creates an immutable snapshot of current statistics.
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
