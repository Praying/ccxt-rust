//! Property-based tests for TimeSyncManager concurrent access.
//!
//! These tests verify that the TimeSyncManager maintains consistency
//! under concurrent access from multiple threads.
//!
//! **Feature: code-refactoring-improvements**
//! **Validates: Requirements 5.4**

use ccxt_exchanges::binance::time_sync::{TimeSyncConfig, TimeSyncManager};
use proptest::prelude::*;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

// ============================================================================
// Test Generators
// ============================================================================

/// Strategy for generating server time offsets (in milliseconds).
fn offset_strategy() -> impl Strategy<Value = i64> {
    // Generate offsets from -10 seconds to +10 seconds
    -10_000i64..10_000i64
}

/// Strategy for generating number of concurrent threads.
fn thread_count_strategy() -> impl Strategy<Value = usize> {
    2usize..10
}

/// Strategy for generating number of operations per thread.
fn ops_per_thread_strategy() -> impl Strategy<Value = usize> {
    10usize..50
}

/// Strategy for generating sync intervals.
fn sync_interval_strategy() -> impl Strategy<Value = Duration> {
    (1u64..120).prop_map(Duration::from_secs)
}

// ============================================================================
// Property Tests: Concurrent Access Consistency
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// **Feature: code-refactoring-improvements, Property: Concurrent access consistency**
    /// **Validates: Requirements 5.4**
    ///
    /// *For any* number of concurrent threads calling `get_server_timestamp()`,
    /// all calls SHALL complete without panic or data race.
    #[test]
    fn prop_concurrent_reads_complete_without_panic(
        thread_count in thread_count_strategy(),
        ops_per_thread in ops_per_thread_strategy(),
        initial_offset in offset_strategy()
    ) {
        let manager = Arc::new(TimeSyncManager::new());

        // Initialize with an offset
        let base_time = ccxt_core::time::TimestampUtils::now_ms();
        manager.update_offset(base_time + initial_offset);

        let mut handles = vec![];

        // Spawn reader threads
        for _ in 0..thread_count {
            let manager_clone = Arc::clone(&manager);
            let handle = thread::spawn(move || {
                let mut timestamps = Vec::with_capacity(ops_per_thread);
                for _ in 0..ops_per_thread {
                    let ts = manager_clone.get_server_timestamp();
                    timestamps.push(ts);
                }
                timestamps
            });
            handles.push(handle);
        }

        // All threads should complete successfully
        for handle in handles {
            let timestamps = handle.join().expect("Thread should not panic");
            // All timestamps should be positive
            for ts in timestamps {
                prop_assert!(ts > 0, "Timestamp should be positive: {}", ts);
            }
        }
    }

    /// **Feature: code-refactoring-improvements, Property: Concurrent writes consistency**
    /// **Validates: Requirements 5.4**
    ///
    /// *For any* number of concurrent threads calling `update_offset()`,
    /// the manager SHALL remain in a consistent state after all writes complete.
    #[test]
    fn prop_concurrent_writes_maintain_consistency(
        thread_count in thread_count_strategy(),
        ops_per_thread in ops_per_thread_strategy()
    ) {
        let manager = Arc::new(TimeSyncManager::new());

        let mut handles = vec![];

        // Spawn writer threads
        for i in 0..thread_count {
            let manager_clone = Arc::clone(&manager);
            let handle = thread::spawn(move || {
                for j in 0..ops_per_thread {
                    let server_time = ccxt_core::time::TimestampUtils::now_ms()
                        + (i * 100 + j) as i64;
                    manager_clone.update_offset(server_time);
                }
            });
            handles.push(handle);
        }

        // All threads should complete successfully
        for handle in handles {
            handle.join().expect("Thread should not panic");
        }

        // Manager should be in a consistent state
        prop_assert!(manager.is_initialized(), "Manager should be initialized after writes");
        prop_assert!(manager.last_sync_time() > 0, "Last sync time should be set");
    }

    /// **Feature: code-refactoring-improvements, Property: Mixed read/write consistency**
    /// **Validates: Requirements 5.4**
    ///
    /// *For any* mix of concurrent readers and writers, the manager SHALL
    /// maintain consistency without data races.
    #[test]
    fn prop_concurrent_read_write_consistency(
        reader_count in 2usize..8,
        writer_count in 1usize..4,
        ops_per_thread in ops_per_thread_strategy()
    ) {
        let manager = Arc::new(TimeSyncManager::new());

        // Initialize the manager
        manager.update_offset(ccxt_core::time::TimestampUtils::now_ms());

        let mut handles = vec![];

        // Spawn reader threads
        for _ in 0..reader_count {
            let manager_clone = Arc::clone(&manager);
            let handle = thread::spawn(move || {
                for _ in 0..ops_per_thread {
                    let _ = manager_clone.get_server_timestamp();
                    let _ = manager_clone.get_offset();
                    let _ = manager_clone.is_initialized();
                    let _ = manager_clone.needs_resync();
                }
            });
            handles.push(handle);
        }

        // Spawn writer threads
        for i in 0..writer_count {
            let manager_clone = Arc::clone(&manager);
            let handle = thread::spawn(move || {
                for j in 0..ops_per_thread {
                    let server_time = ccxt_core::time::TimestampUtils::now_ms()
                        + (i * 50 + j) as i64;
                    manager_clone.update_offset(server_time);
                }
            });
            handles.push(handle);
        }

        // All threads should complete successfully
        for handle in handles {
            handle.join().expect("Thread should not panic");
        }

        // Verify final state is consistent
        prop_assert!(manager.is_initialized());
        let offset = manager.get_offset();
        let last_sync = manager.last_sync_time();
        let timestamp = manager.get_server_timestamp();

        // Timestamp should be approximately last_sync + offset
        let expected_approx = last_sync + offset;
        let diff = (timestamp - expected_approx).abs();
        // Allow some tolerance for timing differences
        prop_assert!(
            diff < 1000,
            "Timestamp {} should be close to expected {} (diff: {})",
            timestamp, expected_approx, diff
        );
    }

    /// **Feature: code-refactoring-improvements, Property: Reset during concurrent access**
    /// **Validates: Requirements 5.4**
    ///
    /// *For any* concurrent access pattern including reset operations,
    /// the manager SHALL not panic or corrupt state.
    #[test]
    fn prop_reset_during_concurrent_access(
        thread_count in 2usize..6,
        ops_per_thread in 10usize..30
    ) {
        let manager = Arc::new(TimeSyncManager::new());
        manager.update_offset(ccxt_core::time::TimestampUtils::now_ms());

        let mut handles = vec![];

        // Spawn reader threads
        for _ in 0..thread_count {
            let manager_clone = Arc::clone(&manager);
            let handle = thread::spawn(move || {
                for _ in 0..ops_per_thread {
                    let _ = manager_clone.get_server_timestamp();
                    let _ = manager_clone.is_initialized();
                }
            });
            handles.push(handle);
        }

        // Spawn a reset thread
        {
            let manager_clone = Arc::clone(&manager);
            let handle = thread::spawn(move || {
                for _ in 0..5 {
                    manager_clone.reset();
                    thread::sleep(Duration::from_micros(100));
                    manager_clone.update_offset(ccxt_core::time::TimestampUtils::now_ms());
                }
            });
            handles.push(handle);
        }

        // All threads should complete successfully
        for handle in handles {
            handle.join().expect("Thread should not panic");
        }
    }

    /// **Feature: code-refactoring-improvements, Property: Configuration immutability**
    /// **Validates: Requirements 5.4**
    ///
    /// *For any* TimeSyncConfig, the configuration should remain unchanged
    /// during concurrent access to the manager.
    #[test]
    fn prop_config_immutable_during_concurrent_access(
        sync_interval in sync_interval_strategy(),
        thread_count in thread_count_strategy()
    ) {
        let config = TimeSyncConfig {
            sync_interval,
            auto_sync: true,
            max_offset_drift: 5000,
        };
        let expected_interval = config.sync_interval;

        let manager = Arc::new(TimeSyncManager::with_config(config));
        manager.update_offset(ccxt_core::time::TimestampUtils::now_ms());

        let mut handles = vec![];

        // Spawn threads that read config
        for _ in 0..thread_count {
            let manager_clone = Arc::clone(&manager);
            let expected = expected_interval;
            let handle = thread::spawn(move || {
                for _ in 0..50 {
                    let config = manager_clone.config();
                    assert_eq!(config.sync_interval, expected);
                    manager_clone.update_offset(ccxt_core::time::TimestampUtils::now_ms());
                }
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.join().expect("Thread should not panic");
        }

        // Config should still be the same
        prop_assert_eq!(manager.config().sync_interval, expected_interval);
    }
}

// ============================================================================
// Additional Property Tests
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// *For any* offset value, the offset should be correctly stored and retrieved.
    #[test]
    fn prop_offset_storage_retrieval(offset in offset_strategy()) {
        let manager = TimeSyncManager::new();
        let base_time = ccxt_core::time::TimestampUtils::now_ms();

        manager.update_offset(base_time + offset);

        let stored_offset = manager.get_offset();
        // The stored offset should be approximately equal to the input offset
        // (with some tolerance for timing)
        let diff = (stored_offset - offset).abs();
        prop_assert!(
            diff < 100,
            "Stored offset {} should be close to input offset {} (diff: {})",
            stored_offset, offset, diff
        );
    }

    /// *For any* server time, update_offset should mark the manager as initialized.
    #[test]
    fn prop_update_offset_initializes(server_time_offset in offset_strategy()) {
        let manager = TimeSyncManager::new();
        prop_assert!(!manager.is_initialized());

        let server_time = ccxt_core::time::TimestampUtils::now_ms() + server_time_offset;
        manager.update_offset(server_time);

        prop_assert!(manager.is_initialized());
    }

    /// *For any* initialized manager, get_server_timestamp should return a reasonable value.
    #[test]
    fn prop_server_timestamp_reasonable(offset in offset_strategy()) {
        let manager = TimeSyncManager::new();
        let base_time = ccxt_core::time::TimestampUtils::now_ms();

        manager.update_offset(base_time + offset);

        let timestamp = manager.get_server_timestamp();
        let current_time = ccxt_core::time::TimestampUtils::now_ms();

        // Timestamp should be within a reasonable range of current time + offset
        let expected_approx = current_time + offset;
        let diff = (timestamp - expected_approx).abs();

        prop_assert!(
            diff < 1000,
            "Timestamp {} should be close to expected {} (diff: {})",
            timestamp, expected_approx, diff
        );
    }
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn test_basic_concurrent_access() {
        let manager = Arc::new(TimeSyncManager::new());
        manager.update_offset(ccxt_core::time::TimestampUtils::now_ms());

        let mut handles = vec![];

        for _ in 0..4 {
            let m = Arc::clone(&manager);
            handles.push(thread::spawn(move || {
                for _ in 0..100 {
                    let _ = m.get_server_timestamp();
                }
            }));
        }

        for h in handles {
            h.join().unwrap();
        }

        assert!(manager.is_initialized());
    }
}
