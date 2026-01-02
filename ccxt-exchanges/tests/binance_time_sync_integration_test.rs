//! Integration tests for Binance Time Sync Optimization
//!
//! These tests verify the time synchronization functionality that reduces
//! network round-trips for signed API requests from 2 to 1.
//!
//! Run with: cargo test --test binance_time_sync_integration_test
//!
//! Note: Tests marked with #[ignore] require real API access.
//! Run them explicitly with:
//! cargo test --test binance_time_sync_integration_test -- --ignored

use ccxt_core::{ExchangeConfig, SecretString, time::TimestampUtils};
use ccxt_exchanges::binance::{Binance, BinanceOptions, TimeSyncConfig, TimeSyncManager};
use std::env;
use std::sync::Arc;
use std::time::Duration;

/// Load API credentials from environment variables.
fn get_api_credentials() -> ExchangeConfig {
    dotenvy::dotenv().ok();
    let api_key = env::var("BINANCE_API_KEY").ok().map(SecretString::new);
    let secret = env::var("BINANCE_API_SECRET").ok().map(SecretString::new);

    let mut config = ExchangeConfig::default();
    config.api_key = api_key;
    config.secret = secret;
    config
}

/// Check if API credentials are available
#[allow(dead_code)]
fn has_api_credentials() -> bool {
    dotenvy::dotenv().ok();
    env::var("BINANCE_API_KEY").is_ok() && env::var("BINANCE_API_SECRET").is_ok()
}

// ==================== Task 12.1: Time Sync with Real API ====================

/// Test that TimeSyncManager is not initialized before any sync
#[tokio::test]
async fn test_time_sync_not_initialized_initially() {
    let config = ExchangeConfig::default();
    let binance = Binance::new(config).unwrap();

    // TimeSyncManager should not be initialized before any sync
    assert!(
        !binance.time_sync().is_initialized(),
        "TimeSyncManager should not be initialized before sync"
    );
    assert!(
        binance.time_sync().needs_resync(),
        "Should need resync when not initialized"
    );
}

/// Test initial time synchronization on first signed request
///
/// Requirements: 2.1 - Initial time synchronization
#[tokio::test]
#[ignore = "Requires real API access"]
async fn test_initial_sync_on_first_signed_request() {
    let config = get_api_credentials();
    if config.api_key.is_none() || config.secret.is_none() {
        println!("⚠️  Skip test: API credentials not set");
        return;
    }

    let binance = Binance::new(config).unwrap();

    // Before any signed request, time sync should not be initialized
    assert!(
        !binance.time_sync().is_initialized(),
        "TimeSyncManager should not be initialized before first request"
    );

    // Make a signed request (fetch_balance triggers time sync)
    let result = binance.fetch_balance(None).await;

    // After the signed request, time sync should be initialized
    assert!(
        binance.time_sync().is_initialized(),
        "TimeSyncManager should be initialized after first signed request"
    );

    // The offset should be set (non-zero or zero depending on clock sync)
    let offset = binance.time_sync().get_offset();
    println!("Time offset after sync: {}ms", offset);

    // The offset should be within reasonable bounds (±30 seconds)
    assert!(
        offset.abs() < 30_000,
        "Time offset should be within ±30 seconds, got {}ms",
        offset
    );

    // The request should succeed (or fail for other reasons, not timestamp)
    if let Err(e) = result {
        // If it fails, it should not be a timestamp error
        assert!(
            !binance.is_timestamp_error(&e),
            "Request should not fail due to timestamp error after sync: {:?}",
            e
        );
    }
}

/// Test that cached timestamp is used on subsequent requests
///
/// Requirements: 1.2 - Use cached time offset for signed requests
#[tokio::test]
#[ignore = "Requires real API access"]
async fn test_cached_timestamp_usage_on_subsequent_requests() {
    let config = get_api_credentials();
    if config.api_key.is_none() || config.secret.is_none() {
        println!("⚠️  Skip test: API credentials not set");
        return;
    }

    let binance = Binance::new(config).unwrap();

    // First request - triggers initial sync
    let _ = binance.fetch_balance(None).await;

    // Record the last sync time
    let first_sync_time = binance.time_sync().last_sync_time();
    assert!(first_sync_time > 0, "Last sync time should be set");

    // Wait a short time
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Second request - should use cached timestamp (no resync needed yet)
    let _ = binance.fetch_balance(None).await;

    // The last sync time should be the same (no resync triggered)
    let second_sync_time = binance.time_sync().last_sync_time();

    // Since sync interval is 30 seconds by default, no resync should occur
    assert_eq!(
        first_sync_time, second_sync_time,
        "No resync should occur within sync interval"
    );
}

/// Test get_signing_timestamp returns valid timestamp
#[tokio::test]
#[ignore = "Requires real API access"]
async fn test_get_signing_timestamp_returns_valid_timestamp() {
    let config = ExchangeConfig::default();
    let binance = Binance::new(config).unwrap();

    // Get signing timestamp (will trigger sync since not initialized)
    let timestamp = binance.get_signing_timestamp().await.unwrap();

    // Timestamp should be valid
    assert!(timestamp > 0, "Timestamp should be positive");
    assert!(
        TimestampUtils::validate_timestamp(timestamp).is_ok(),
        "Timestamp should be valid"
    );

    // Timestamp should be close to current time (within 30 seconds)
    let now = TimestampUtils::now_ms();
    let diff = (timestamp - now).abs();
    assert!(
        diff < 30_000,
        "Timestamp should be within 30 seconds of local time, diff: {}ms",
        diff
    );
}

/// Test sync_time method updates offset correctly
#[tokio::test]
#[ignore = "Requires real API access"]
async fn test_sync_time_updates_offset() {
    let config = ExchangeConfig::default();
    let binance = Binance::new(config).unwrap();

    // Before sync
    assert!(!binance.time_sync().is_initialized());

    // Perform sync
    binance.sync_time().await.unwrap();

    // After sync
    assert!(binance.time_sync().is_initialized());
    assert!(binance.time_sync().last_sync_time() > 0);

    // Offset should be reasonable
    let offset = binance.time_sync().get_offset();
    assert!(offset.abs() < 30_000, "Offset should be within ±30 seconds");
}

/// Test multiple sync calls update the offset
#[tokio::test]
#[ignore = "Requires real API access"]
async fn test_multiple_sync_calls() {
    let config = ExchangeConfig::default();
    let binance = Binance::new(config).unwrap();

    // First sync
    binance.sync_time().await.unwrap();
    let first_offset = binance.time_sync().get_offset();
    let first_sync_time = binance.time_sync().last_sync_time();

    // Wait a bit
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Second sync
    binance.sync_time().await.unwrap();
    let second_offset = binance.time_sync().get_offset();
    let second_sync_time = binance.time_sync().last_sync_time();

    // Sync time should be updated
    assert!(
        second_sync_time >= first_sync_time,
        "Second sync time should be >= first"
    );

    // Offsets should be similar (within 1 second)
    let offset_diff = (second_offset - first_offset).abs();
    assert!(
        offset_diff < 1000,
        "Offsets should be similar, diff: {}ms",
        offset_diff
    );
}

// ==================== Task 12.2: Backward Compatibility ====================

/// Test that fetch_time() still works independently
///
/// Requirements: 7.1 - fetch_time() should continue to work
#[tokio::test]
#[ignore = "Requires real API access"]
async fn test_fetch_time_works_independently() {
    let config = ExchangeConfig::default();
    let binance = Binance::new(config).unwrap();

    // fetch_time should work without any prior sync
    let server_time = binance.fetch_time().await.unwrap();

    // Server time should be valid
    assert!(
        server_time.server_time > 0,
        "Server time should be positive"
    );
    assert!(
        TimestampUtils::validate_timestamp(server_time.server_time).is_ok(),
        "Server time should be valid"
    );

    // Server time should be close to current time
    let now = TimestampUtils::now_ms();
    let diff = (server_time.server_time - now).abs();
    assert!(
        diff < 30_000,
        "Server time should be within 30 seconds of local time"
    );
}

/// Test that fetch_time() does not affect cached offset
///
/// Requirements: 7.3 - fetch_time() should not affect cached offset
#[tokio::test]
#[ignore = "Requires real API access"]
async fn test_fetch_time_does_not_affect_cached_offset() {
    let config = ExchangeConfig::default();
    let binance = Binance::new(config).unwrap();

    // First, sync time to initialize the cache
    binance.sync_time().await.unwrap();
    let initial_offset = binance.time_sync().get_offset();
    let initial_sync_time = binance.time_sync().last_sync_time();

    // Call fetch_time multiple times
    for _ in 0..3 {
        let _ = binance.fetch_time().await.unwrap();
    }

    // The cached offset should remain unchanged
    let final_offset = binance.time_sync().get_offset();
    let final_sync_time = binance.time_sync().last_sync_time();

    assert_eq!(
        initial_offset, final_offset,
        "fetch_time() should not change cached offset"
    );
    assert_eq!(
        initial_sync_time, final_sync_time,
        "fetch_time() should not update last sync time"
    );
}

/// Test that existing API behavior is preserved
///
/// Requirements: 7.2 - Existing API behavior should be preserved
#[tokio::test]
#[ignore = "Requires real API access"]
async fn test_existing_api_behavior_preserved() {
    let config = ExchangeConfig::default();
    let binance = Binance::new(config).unwrap();

    // Load markets first (required for symbol-based operations)
    let markets = binance.load_markets(false).await;
    assert!(
        markets.is_ok(),
        "load_markets should work: {:?}",
        markets.err()
    );

    // Test public API methods still work
    let ticker = binance
        .fetch_ticker("BTC/USDT", ccxt_core::types::TickerParams::default())
        .await;
    assert!(
        ticker.is_ok(),
        "fetch_ticker should work: {:?}",
        ticker.err()
    );

    let orderbook = binance.fetch_order_book("BTC/USDT", Some(5)).await;
    assert!(
        orderbook.is_ok(),
        "fetch_order_book should work: {:?}",
        orderbook.err()
    );

    let trades = binance.fetch_trades("BTC/USDT", Some(5)).await;
    assert!(
        trades.is_ok(),
        "fetch_trades should work: {:?}",
        trades.err()
    );

    // Test that server time can still be fetched directly
    let server_time = binance.fetch_time().await;
    assert!(
        server_time.is_ok(),
        "fetch_time should work: {:?}",
        server_time.err()
    );
}

// ==================== TimeSyncManager Unit Tests ====================

/// Test TimeSyncManager configuration
#[test]
fn test_time_sync_manager_configuration() {
    // Default configuration
    let manager = TimeSyncManager::new();
    assert_eq!(manager.config().sync_interval, Duration::from_secs(30));
    assert!(manager.config().auto_sync);
    assert_eq!(manager.config().max_offset_drift, 5000);

    // Custom configuration
    let config = TimeSyncConfig {
        sync_interval: Duration::from_secs(60),
        auto_sync: false,
        max_offset_drift: 3000,
    };
    let manager = TimeSyncManager::with_config(config);
    assert_eq!(manager.config().sync_interval, Duration::from_secs(60));
    assert!(!manager.config().auto_sync);
    assert_eq!(manager.config().max_offset_drift, 3000);
}

/// Test TimeSyncManager offset calculation
#[test]
fn test_time_sync_manager_offset_calculation() {
    let manager = TimeSyncManager::new();

    // Simulate server time being 500ms ahead
    let local_time = TimestampUtils::now_ms();
    let server_time = local_time + 500;
    manager.update_offset(server_time);

    // Offset should be approximately 500ms
    let offset = manager.get_offset();
    assert!(
        offset >= 400 && offset <= 600,
        "Offset should be ~500ms, got {}",
        offset
    );

    // get_server_timestamp should return a value close to server_time
    let estimated = manager.get_server_timestamp();
    let diff = (estimated - server_time).abs();
    assert!(
        diff < 100,
        "Estimated server time should be close to actual, diff: {}ms",
        diff
    );
}

/// Test TimeSyncManager needs_resync logic
#[test]
fn test_time_sync_manager_needs_resync() {
    // With auto_sync enabled
    let manager = TimeSyncManager::new();
    assert!(
        manager.needs_resync(),
        "Should need resync when not initialized"
    );

    manager.update_offset(TimestampUtils::now_ms());
    assert!(
        !manager.needs_resync(),
        "Should not need resync immediately after sync"
    );

    // With auto_sync disabled
    let config = TimeSyncConfig::manual_sync_only();
    let manager = TimeSyncManager::with_config(config);
    manager.update_offset(TimestampUtils::now_ms());
    assert!(
        !manager.needs_resync(),
        "Should never need auto resync when disabled"
    );
}

/// Test TimeSyncManager reset functionality
#[test]
fn test_time_sync_manager_reset() {
    let manager = TimeSyncManager::new();
    manager.update_offset(TimestampUtils::now_ms() + 1000);

    assert!(manager.is_initialized());
    assert!(manager.get_offset() != 0 || manager.last_sync_time() > 0);

    manager.reset();

    assert!(!manager.is_initialized());
    assert_eq!(manager.get_offset(), 0);
    assert_eq!(manager.last_sync_time(), 0);
}

/// Test Binance time sync configuration via options
#[test]
fn test_binance_time_sync_options() {
    let config = ExchangeConfig::default();
    let options = BinanceOptions {
        time_sync_interval_secs: 60,
        auto_time_sync: false,
        ..Default::default()
    };

    let binance = Binance::new_with_options(config, options).unwrap();

    assert_eq!(
        binance.time_sync().config().sync_interval,
        Duration::from_secs(60)
    );
    assert!(!binance.time_sync().config().auto_sync);
}

/// Test is_timestamp_error detection
#[test]
fn test_is_timestamp_error_detection() {
    let config = ExchangeConfig::default();
    let binance = Binance::new(config).unwrap();

    // Timestamp errors
    let err1 = ccxt_core::Error::exchange(
        "-1021",
        "Timestamp for this request is outside of the recvWindow",
    );
    assert!(binance.is_timestamp_error(&err1));

    let err2 = ccxt_core::Error::exchange("-1021", "Timestamp is ahead of server time");
    assert!(binance.is_timestamp_error(&err2));

    let err3 = ccxt_core::Error::exchange("-1021", "Timestamp is behind server time");
    assert!(binance.is_timestamp_error(&err3));

    // Non-timestamp errors
    let err4 = ccxt_core::Error::exchange("-1100", "Illegal characters found in parameter");
    assert!(!binance.is_timestamp_error(&err4));

    let err5 = ccxt_core::Error::exchange("-2010", "Insufficient balance");
    assert!(!binance.is_timestamp_error(&err5));
}

/// Test thread safety of TimeSyncManager
#[tokio::test]
async fn test_time_sync_thread_safety() {
    let manager = Arc::new(TimeSyncManager::new());
    let mut handles = vec![];

    // Spawn multiple reader tasks
    for _ in 0..5 {
        let manager_clone = Arc::clone(&manager);
        let handle = tokio::spawn(async move {
            for _ in 0..100 {
                let _ = manager_clone.get_server_timestamp();
                let _ = manager_clone.get_offset();
                let _ = manager_clone.is_initialized();
                let _ = manager_clone.needs_resync();
            }
        });
        handles.push(handle);
    }

    // Spawn writer tasks
    for i in 0..3 {
        let manager_clone = Arc::clone(&manager);
        let handle = tokio::spawn(async move {
            for j in 0..50 {
                let server_time = TimestampUtils::now_ms() + (i * 10 + j) as i64;
                manager_clone.update_offset(server_time);
            }
        });
        handles.push(handle);
    }

    // Wait for all tasks to complete
    for handle in handles {
        handle.await.unwrap();
    }

    // Manager should be in a consistent state
    assert!(manager.is_initialized());
    assert!(manager.last_sync_time() > 0);
}
