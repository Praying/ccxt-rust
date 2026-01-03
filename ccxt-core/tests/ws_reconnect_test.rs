//! WebSocket automatic reconnection mechanism integration tests.
//!
//! Test coverage:
//! - Heartbeat timeout detection
//! - Automatic reconnection coordinator
//! - Connection state transitions
//! - Subscription auto-recovery

use ccxt_core::ws_client::{WsClient, WsConfig, WsConnectionState, WsEvent};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::time::{Duration, sleep};

/// Test that `WsConfig` includes `pong_timeout` field.
#[test]
fn test_ws_config_has_pong_timeout() {
    let config = WsConfig::default();
    assert_eq!(
        config.pong_timeout, 90000,
        "Default pong timeout should be 90 seconds"
    );
}

/// Test that `WsConfig` allows custom `pong_timeout` configuration.
#[test]
fn test_ws_config_custom_pong_timeout() {
    let config = WsConfig {
        url: "wss://example.com".to_string(),
        connect_timeout: 10000,
        ping_interval: 30000,
        reconnect_interval: 5000,
        max_reconnect_attempts: 5,
        auto_reconnect: true,
        enable_compression: false,
        pong_timeout: 120000, // Custom timeout: 120 seconds
        ..Default::default()
    };

    assert_eq!(config.pong_timeout, 120000);
}

/// Test `AutoReconnectCoordinator` creation.
#[tokio::test]
async fn test_auto_reconnect_coordinator_creation() {
    let config = WsConfig {
        url: "wss://stream.binance.com:9443/ws".to_string(),
        connect_timeout: 10000,
        ping_interval: 30000,
        reconnect_interval: 5000,
        max_reconnect_attempts: 5,
        auto_reconnect: true,
        enable_compression: false,
        pong_timeout: 90000,
        ..Default::default()
    };

    let client = Arc::new(WsClient::new(config));
    let _coordinator = client.clone().create_auto_reconnect_coordinator();

    // Coordinator should be created successfully
    // Note: Not starting the coordinator, only testing creation
}

/// Test `AutoReconnectCoordinator` start and stop operations.
#[tokio::test]
async fn test_auto_reconnect_coordinator_start_stop() {
    let config = WsConfig {
        url: "wss://stream.binance.com:9443/ws".to_string(),
        connect_timeout: 10000,
        ping_interval: 30000,
        reconnect_interval: 5000,
        max_reconnect_attempts: 5,
        auto_reconnect: true,
        enable_compression: false,
        pong_timeout: 90000,
        ..Default::default()
    };

    let client = Arc::new(WsClient::new(config));
    let coordinator = client.clone().create_auto_reconnect_coordinator();

    // Start coordinator
    coordinator.start().await;
    sleep(Duration::from_millis(100)).await;

    // Stop coordinator
    coordinator.stop().await;
    sleep(Duration::from_millis(100)).await;

    // Test passes if start/stop operations don't crash
}

/// Test event callback mechanism.
#[tokio::test]
async fn test_event_callback() {
    let config = WsConfig {
        url: "wss://stream.binance.com:9443/ws".to_string(),
        connect_timeout: 10000,
        ping_interval: 30000,
        reconnect_interval: 5000,
        max_reconnect_attempts: 5,
        auto_reconnect: true,
        enable_compression: false,
        pong_timeout: 90000,
        ..Default::default()
    };

    let client = Arc::new(WsClient::new(config));

    // Set up event flags
    let connected_flag = Arc::new(AtomicBool::new(false));
    let disconnected_flag = Arc::new(AtomicBool::new(false));
    let reconnecting_flag = Arc::new(AtomicBool::new(false));

    let connected_clone = connected_flag.clone();
    let disconnected_clone = disconnected_flag.clone();
    let reconnecting_clone = reconnecting_flag.clone();

    // Create coordinator with callback
    let coordinator = client
        .clone()
        .create_auto_reconnect_coordinator()
        .with_callback(Arc::new(move |event| match event {
            WsEvent::Connected => {
                connected_clone.store(true, Ordering::SeqCst);
            }
            WsEvent::Disconnected => {
                disconnected_clone.store(true, Ordering::SeqCst);
            }
            WsEvent::Reconnecting { .. } => {
                reconnecting_clone.store(true, Ordering::SeqCst);
            }
            _ => {}
        }));

    // Start coordinator
    coordinator.start().await;
    sleep(Duration::from_millis(200)).await;

    // Stop coordinator
    coordinator.stop().await;

    // Note: Flags may not be set since we're not actually connecting
    // This test primarily verifies that the callback mechanism doesn't crash
}

/// Test `WsConnectionState` enum behavior.
#[test]
fn test_ws_connection_state_enum() {
    let state1 = WsConnectionState::Disconnected;
    let state2 = WsConnectionState::Connecting;
    let state3 = WsConnectionState::Connected;
    let _state4 = WsConnectionState::Reconnecting;
    let _state5 = WsConnectionState::Error;

    // Test PartialEq trait
    assert_eq!(state1, WsConnectionState::Disconnected);
    assert_ne!(state1, state2);

    // Test Copy trait (no need to clone since it's Copy)
    let state1_copy = state1;
    assert_eq!(state1, state1_copy);

    // Test Debug trait
    let debug_str = format!("{:?}", state3);
    assert!(debug_str.contains("Connected"));
}

/// Test `WsEvent` enum behavior.
#[test]
fn test_ws_event_enum() {
    use std::time::Duration;

    let event1 = WsEvent::Connected;
    let _event2 = WsEvent::Disconnected;
    let event3 = WsEvent::Reconnecting {
        attempt: 1,
        delay: Duration::from_secs(5),
        error: Some("connection lost".to_string()),
    };
    let _event4 = WsEvent::ReconnectSuccess;
    let _event5 = WsEvent::ReconnectFailed {
        attempt: 1,
        error: "timeout".to_string(),
        is_permanent: false,
    };
    let _event6 = WsEvent::SubscriptionRestored;
    let _event7 = WsEvent::Connecting;
    let _event8 = WsEvent::ReconnectExhausted {
        total_attempts: 5,
        last_error: "max attempts reached".to_string(),
    };
    let _event9 = WsEvent::PermanentError {
        error: "authentication failed".to_string(),
    };
    let _event10 = WsEvent::Shutdown;

    // Test Clone trait
    let event1_clone = event1.clone();
    match (event1, event1_clone) {
        (WsEvent::Connected, WsEvent::Connected) => {}
        _ => panic!("Clone failed"),
    }

    // Test Debug trait
    let debug_str = format!("{:?}", event3);
    assert!(debug_str.contains("Reconnecting"));
    assert!(debug_str.contains("attempt"));

    // Test helper methods
    assert!(WsEvent::Connecting.is_connecting());
    assert!(WsEvent::Connected.is_connected());
    assert!(WsEvent::Disconnected.is_disconnected());
    assert!(
        WsEvent::Reconnecting {
            attempt: 1,
            delay: Duration::from_secs(1),
            error: None
        }
        .is_reconnecting()
    );
    assert!(WsEvent::ReconnectSuccess.is_reconnect_success());
    assert!(
        WsEvent::ReconnectFailed {
            attempt: 1,
            error: "test".to_string(),
            is_permanent: false
        }
        .is_reconnect_failed()
    );
    assert!(
        WsEvent::ReconnectExhausted {
            total_attempts: 5,
            last_error: "test".to_string()
        }
        .is_reconnect_exhausted()
    );
    assert!(WsEvent::SubscriptionRestored.is_subscription_restored());
    assert!(
        WsEvent::PermanentError {
            error: "test".to_string()
        }
        .is_permanent_error()
    );
    assert!(WsEvent::Shutdown.is_shutdown());

    // Test is_error
    assert!(!WsEvent::Connected.is_error());
    assert!(
        WsEvent::ReconnectFailed {
            attempt: 1,
            error: "test".to_string(),
            is_permanent: false
        }
        .is_error()
    );
    assert!(
        WsEvent::PermanentError {
            error: "test".to_string()
        }
        .is_error()
    );

    // Test is_terminal
    assert!(!WsEvent::Connected.is_terminal());
    assert!(WsEvent::Shutdown.is_terminal());
    assert!(
        WsEvent::PermanentError {
            error: "test".to_string()
        }
        .is_terminal()
    );
    assert!(
        WsEvent::ReconnectExhausted {
            total_attempts: 5,
            last_error: "test".to_string()
        }
        .is_terminal()
    );

    // Test Display trait
    let display_str = format!("{}", WsEvent::Connected);
    assert_eq!(display_str, "Connected");

    let display_str = format!(
        "{}",
        WsEvent::Reconnecting {
            attempt: 2,
            delay: Duration::from_secs(5),
            error: Some("network error".to_string()),
        }
    );
    assert!(display_str.contains("Reconnecting"));
    assert!(display_str.contains("attempt 2"));
}

#[cfg(test)]
mod integration_tests {
    use super::*;

    /// Integration test: Full reconnect flow (simulated).
    ///
    /// Note: This test does not actually connect to a server.
    #[tokio::test]
    async fn test_full_reconnect_flow_simulation() {
        let config = WsConfig {
            url: "wss://stream.binance.com:9443/ws".to_string(),
            connect_timeout: 5000,
            ping_interval: 30000,
            reconnect_interval: 2000,
            max_reconnect_attempts: 3,
            auto_reconnect: true,
            enable_compression: false,
            pong_timeout: 10000, // Short timeout for testing
            ..Default::default()
        };

        let client = Arc::new(WsClient::new(config));

        // Record events
        let events = Arc::new(tokio::sync::Mutex::new(Vec::new()));
        let events_clone = events.clone();

        let coordinator = client
            .clone()
            .create_auto_reconnect_coordinator()
            .with_callback(Arc::new(move |event| {
                let events = events_clone.clone();
                tokio::spawn(async move {
                    events.lock().await.push(format!("{:?}", event));
                });
            }));

        // Start coordinator
        coordinator.start().await;

        // Wait briefly to observe behavior
        sleep(Duration::from_millis(500)).await;

        // Stop coordinator
        coordinator.stop().await;

        // Check if events were recorded (may be empty since no actual connection)
        let recorded_events = events.lock().await;
        println!("Recorded events: {:?}", recorded_events);

        // Test passes if entire flow completes without crashing
    }
}
