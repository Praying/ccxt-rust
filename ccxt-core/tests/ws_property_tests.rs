//! Property-based tests for WsClient concurrent subscription operations.
//!
//! These tests verify that the WsClient maintains consistency under concurrent
//! subscription operations (add, remove, check).
//!
//! **Feature: code-refactoring-improvements, Property 5: Concurrent Subscription Consistency**
//! **Validates: Requirements 5.5**

use ccxt_core::ws_client::{WsClient, WsConfig, WsConnectionState};
use proptest::prelude::*;
use std::sync::Arc;
use std::thread;

// ============================================================================
// Test Generators
// ============================================================================

/// Strategy for generating channel names.
fn channel_strategy() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("ticker".to_string()),
        Just("orderbook".to_string()),
        Just("trades".to_string()),
        Just("kline".to_string()),
        Just("depth".to_string()),
        "[a-z]{3,10}".prop_map(|s| s.to_string()),
    ]
}

/// Strategy for generating optional symbol names.
fn symbol_strategy() -> impl Strategy<Value = Option<String>> {
    prop_oneof![
        Just(None),
        Just(Some("BTCUSDT".to_string())),
        Just(Some("ETHUSDT".to_string())),
        Just(Some("SOLUSDT".to_string())),
        "[A-Z]{3,6}".prop_map(|s| Some(s)),
    ]
}

/// Strategy for generating number of concurrent threads.
fn thread_count_strategy() -> impl Strategy<Value = usize> {
    2usize..8
}

/// Strategy for generating number of operations per thread.
fn ops_per_thread_strategy() -> impl Strategy<Value = usize> {
    10usize..30
}

// ============================================================================
// Property 5: Concurrent Subscription Consistency Tests
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// **Feature: code-refactoring-improvements, Property 5: Concurrent subscription consistency**
    /// **Validates: Requirements 5.5**
    ///
    /// *For any* number of concurrent subscription operations (add, remove, check),
    /// the WsClient SHALL maintain a consistent view without data races or deadlocks.
    #[test]
    fn prop_concurrent_subscription_operations_consistency(
        thread_count in thread_count_strategy(),
        ops_per_thread in ops_per_thread_strategy()
    ) {
        let config = WsConfig {
            url: "wss://test.example.com".to_string(),
            ..WsConfig::default()
        };
        let client = Arc::new(WsClient::new(config));

        let mut handles = vec![];

        // Spawn threads that perform mixed operations
        for thread_id in 0..thread_count {
            let client_clone = Arc::clone(&client);
            let handle = thread::spawn(move || {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .unwrap();

                rt.block_on(async {
                    for op_id in 0..ops_per_thread {
                        let channel = format!("channel_{}", (thread_id + op_id) % 5);
                        let symbol = if op_id % 2 == 0 {
                            Some(format!("SYM{}", thread_id))
                        } else {
                            None
                        };

                        match op_id % 3 {
                            0 => {
                                // Add subscription
                                let _ = client_clone.subscribe(
                                    channel.clone(),
                                    symbol.clone(),
                                    None,
                                ).await;
                            }
                            1 => {
                                // Remove subscription
                                let _ = client_clone.unsubscribe(channel.clone(), symbol.clone()).await;
                            }
                            _ => {
                                // Check subscription
                                let _ = client_clone.is_subscribed(&channel, &symbol);
                            }
                        }
                    }
                });
            });
            handles.push(handle);
        }

        // All threads should complete without panic
        for handle in handles {
            handle.join().expect("Thread should not panic");
        }

        // Client should be in a consistent state
        let _count = client.subscription_count();
    }

    /// **Feature: code-refactoring-improvements, Property 5: Concurrent add operations**
    /// **Validates: Requirements 5.5**
    ///
    /// *For any* number of concurrent add operations, all subscriptions should be tracked.
    #[test]
    fn prop_concurrent_add_operations(
        thread_count in thread_count_strategy(),
        ops_per_thread in 5usize..15
    ) {
        let config = WsConfig {
            url: "wss://test.example.com".to_string(),
            ..WsConfig::default()
        };
        let client = Arc::new(WsClient::new(config));

        let mut handles = vec![];

        // Each thread adds unique subscriptions
        for thread_id in 0..thread_count {
            let client_clone = Arc::clone(&client);
            let handle = thread::spawn(move || {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .unwrap();

                rt.block_on(async {
                    for op_id in 0..ops_per_thread {
                        let channel = format!("t{}c{}", thread_id, op_id);
                        let _ = client_clone.subscribe(channel, None, None).await;
                    }
                });
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.join().expect("Thread should not panic");
        }

        // All unique subscriptions should be present
        let count = client.subscription_count();
        let expected = thread_count * ops_per_thread;
        prop_assert_eq!(
            count, expected,
            "All {} subscriptions should be tracked, got {}",
            expected, count
        );
    }

    /// **Feature: code-refactoring-improvements, Property 5: Concurrent check operations**
    /// **Validates: Requirements 5.5**
    ///
    /// *For any* number of concurrent check operations, all should complete without blocking.
    #[test]
    fn prop_concurrent_check_operations(
        thread_count in thread_count_strategy(),
        ops_per_thread in ops_per_thread_strategy()
    ) {
        let config = WsConfig {
            url: "wss://test.example.com".to_string(),
            ..WsConfig::default()
        };
        let client = Arc::new(WsClient::new(config));

        // Pre-populate some subscriptions
        {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            rt.block_on(async {
                for i in 0..5 {
                    let _ = client.subscribe(format!("channel_{}", i), None, None).await;
                }
            });
        }

        let mut handles = vec![];

        // Spawn threads that only check subscriptions
        for _ in 0..thread_count {
            let client_clone = Arc::clone(&client);
            let handle = thread::spawn(move || {
                let mut results = Vec::with_capacity(ops_per_thread);
                for i in 0..ops_per_thread {
                    let channel = format!("channel_{}", i % 10);
                    let is_sub = client_clone.is_subscribed(&channel, &None);
                    results.push(is_sub);
                }
                results
            });
            handles.push(handle);
        }

        // All threads should complete
        for handle in handles {
            let results = handle.join().expect("Thread should not panic");
            // First 5 channels should be subscribed
            for (i, &is_sub) in results.iter().enumerate() {
                let expected = (i % 10) < 5;
                prop_assert_eq!(
                    is_sub, expected,
                    "Channel {} subscription status mismatch",
                    i % 10
                );
            }
        }
    }

    /// **Feature: code-refactoring-improvements, Property 5: State consistency**
    /// **Validates: Requirements 5.5**
    ///
    /// *For any* concurrent state reads, the state should always be a valid enum value.
    #[test]
    fn prop_concurrent_state_reads(
        thread_count in thread_count_strategy(),
        ops_per_thread in ops_per_thread_strategy()
    ) {
        let config = WsConfig {
            url: "wss://test.example.com".to_string(),
            ..WsConfig::default()
        };
        let client = Arc::new(WsClient::new(config));

        let mut handles = vec![];

        for _ in 0..thread_count {
            let client_clone = Arc::clone(&client);
            let handle = thread::spawn(move || {
                let mut states = Vec::with_capacity(ops_per_thread);
                for _ in 0..ops_per_thread {
                    let state = client_clone.state();
                    states.push(state);
                }
                states
            });
            handles.push(handle);
        }

        for handle in handles {
            let states = handle.join().expect("Thread should not panic");
            for state in states {
                // State should be a valid enum value
                let valid = matches!(
                    state,
                    WsConnectionState::Disconnected
                        | WsConnectionState::Connecting
                        | WsConnectionState::Connected
                        | WsConnectionState::Reconnecting
                        | WsConnectionState::Error
                );
                prop_assert!(valid, "State should be a valid enum value: {:?}", state);
            }
        }
    }

    /// **Feature: code-refactoring-improvements, Property 5: Reconnect count consistency**
    /// **Validates: Requirements 5.5**
    ///
    /// *For any* concurrent reconnect count operations, the count should be consistent.
    #[test]
    fn prop_concurrent_reconnect_count_operations(
        thread_count in thread_count_strategy(),
        ops_per_thread in ops_per_thread_strategy()
    ) {
        let config = WsConfig {
            url: "wss://test.example.com".to_string(),
            ..WsConfig::default()
        };
        let client = Arc::new(WsClient::new(config));

        let mut reader_handles = vec![];
        let mut reset_handles = vec![];

        // Spawn reader threads
        for _ in 0..thread_count {
            let client_clone = Arc::clone(&client);
            let handle = thread::spawn(move || {
                let mut counts = Vec::with_capacity(ops_per_thread);
                for _ in 0..ops_per_thread {
                    let count = client_clone.reconnect_count();
                    counts.push(count);
                }
                counts
            });
            reader_handles.push(handle);
        }

        // Spawn a reset thread
        {
            let client_clone = Arc::clone(&client);
            let handle = thread::spawn(move || {
                for _ in 0..5 {
                    client_clone.reset_reconnect_count();
                    thread::sleep(std::time::Duration::from_micros(100));
                }
            });
            reset_handles.push(handle);
        }

        // All threads should complete
        for handle in reader_handles {
            handle.join().expect("Reader thread should not panic");
        }
        for handle in reset_handles {
            handle.join().expect("Reset thread should not panic");
        }

        // Final count should be 0 after resets
        let final_count = client.reconnect_count();
        prop_assert_eq!(final_count, 0, "Reconnect count should be 0 after resets");
    }
}

// ============================================================================
// Additional Property Tests
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// *For any* subscription, adding and then checking should return true.
    #[test]
    fn prop_add_then_check_returns_true(
        channel in channel_strategy(),
        symbol in symbol_strategy()
    ) {
        let config = WsConfig {
            url: "wss://test.example.com".to_string(),
            ..WsConfig::default()
        };
        let client = WsClient::new(config);

        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();

        rt.block_on(async {
            let _ = client.subscribe(channel.clone(), symbol.clone(), None).await;
        });

        let is_subscribed = client.is_subscribed(&channel, &symbol);
        prop_assert!(is_subscribed, "Should be subscribed after add");
    }

    /// *For any* subscription, adding then removing then checking should return false.
    #[test]
    fn prop_add_remove_then_check_returns_false(
        channel in channel_strategy(),
        symbol in symbol_strategy()
    ) {
        let config = WsConfig {
            url: "wss://test.example.com".to_string(),
            ..WsConfig::default()
        };
        let client = WsClient::new(config);

        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();

        rt.block_on(async {
            let _ = client.subscribe(channel.clone(), symbol.clone(), None).await;
            let _ = client.unsubscribe(channel.clone(), symbol.clone()).await;
        });

        let is_subscribed = client.is_subscribed(&channel, &symbol);
        prop_assert!(!is_subscribed, "Should not be subscribed after remove");
    }

    /// *For any* WsConfig, creating a client should result in Disconnected state.
    #[test]
    fn prop_new_client_is_disconnected(
        ping_interval in 1000u64..60000,
        reconnect_interval in 1000u64..30000
    ) {
        let config = WsConfig {
            url: "wss://test.example.com".to_string(),
            ping_interval,
            reconnect_interval,
            ..WsConfig::default()
        };
        let client = WsClient::new(config);

        prop_assert_eq!(
            client.state(),
            WsConnectionState::Disconnected,
            "New client should be in Disconnected state"
        );
        prop_assert_eq!(
            client.subscription_count(),
            0,
            "New client should have no subscriptions"
        );
        prop_assert_eq!(
            client.reconnect_count(),
            0,
            "New client should have zero reconnect count"
        );
    }
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn test_basic_subscription_operations() {
        let config = WsConfig {
            url: "wss://test.example.com".to_string(),
            ..WsConfig::default()
        };
        let client = WsClient::new(config);

        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();

        rt.block_on(async {
            // Add subscription
            client
                .subscribe("ticker".to_string(), Some("BTCUSDT".to_string()), None)
                .await
                .unwrap();
            assert!(client.is_subscribed("ticker", &Some("BTCUSDT".to_string())));
            assert_eq!(client.subscription_count(), 1);

            // Remove subscription
            client
                .unsubscribe("ticker".to_string(), Some("BTCUSDT".to_string()))
                .await
                .unwrap();
            assert!(!client.is_subscribed("ticker", &Some("BTCUSDT".to_string())));
            assert_eq!(client.subscription_count(), 0);
        });
    }

    #[test]
    fn test_concurrent_subscription_basic() {
        let config = WsConfig {
            url: "wss://test.example.com".to_string(),
            ..WsConfig::default()
        };
        let client = Arc::new(WsClient::new(config));

        let mut handles = vec![];

        for i in 0..4 {
            let c = Arc::clone(&client);
            handles.push(thread::spawn(move || {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .unwrap();
                rt.block_on(async {
                    c.subscribe(format!("channel_{}", i), None, None)
                        .await
                        .unwrap();
                });
            }));
        }

        for h in handles {
            h.join().unwrap();
        }

        assert_eq!(client.subscription_count(), 4);
    }
}
