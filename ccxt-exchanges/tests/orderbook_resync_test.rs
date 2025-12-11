//! Orderbook automatic resynchronization mechanism tests

use ccxt_core::types::orderbook::{OrderBook, OrderBookDelta, OrderBookEntry};
use chrono::Utc;

#[test]
fn test_sequence_gap_detection_spot() {
    let mut orderbook =
        OrderBook::new("BTCUSDT".to_string(), chrono::Utc::now().timestamp_millis());

    orderbook.nonce = Some(100);
    orderbook.is_synced = true;

    // Gap detected: expected 101, got 105
    let delta = OrderBookDelta {
        symbol: "BTCUSDT".to_string(),
        first_update_id: 105,
        final_update_id: 106,
        prev_final_update_id: None,
        timestamp: chrono::Utc::now().timestamp_millis(),
        bids: vec![OrderBookEntry {
            price: "50000.0".parse().unwrap(),
            amount: "1.5".parse().unwrap(),
        }],
        asks: vec![OrderBookEntry {
            price: "50100.0".parse().unwrap(),
            amount: "2.0".parse().unwrap(),
        }],
    };

    let result = orderbook.apply_delta(&delta, false);

    assert!(result.is_err(), "Should detect sequence gap");
    assert!(orderbook.needs_resync, "Should set needs_resync flag");
    assert!(!orderbook.is_synced, "Should clear synced status");
    assert_eq!(
        orderbook.buffered_deltas.len(),
        0,
        "Failed apply should not buffer delta"
    );
}

#[test]
fn test_sequence_gap_detection_futures() {
    let mut orderbook = OrderBook::new("BTCUSDT".to_string(), Utc::now().timestamp_millis());

    orderbook.nonce = Some(100);
    orderbook.is_synced = true;

    // Sequence mismatch: pu=98, expected 100
    let delta = OrderBookDelta {
        symbol: "BTCUSDT".to_string(),
        first_update_id: 101,
        final_update_id: 102,
        prev_final_update_id: Some(98), // Mismatch
        timestamp: chrono::Utc::now().timestamp_millis(),
        bids: vec![OrderBookEntry {
            price: "50000.0".parse().unwrap(),
            amount: "1.5".parse().unwrap(),
        }],
        asks: vec![OrderBookEntry {
            price: "50100.0".parse().unwrap(),
            amount: "2.0".parse().unwrap(),
        }],
    };

    let result = orderbook.apply_delta(&delta, true);

    assert!(result.is_err(), "Should detect sequence mismatch");
    assert!(orderbook.needs_resync, "Should set needs_resync flag");
    assert!(!orderbook.is_synced, "Should clear synced status");
}

#[test]
fn test_resync_rate_limiting() {
    let mut orderbook = OrderBook::new("BTCUSDT".to_string(), Utc::now().timestamp_millis());

    orderbook.needs_resync = true;
    let base_time = Utc::now().timestamp_millis();
    orderbook.last_resync_time = base_time;

    assert!(
        !orderbook.should_resync(base_time + 500),
        "Should be rate-limited within 500ms"
    );

    assert!(
        orderbook.should_resync(base_time + 1000),
        "Should allow resync after 1 second"
    );

    assert!(
        orderbook.should_resync(base_time + 2000),
        "Should allow resync after 2 seconds"
    );
}

#[test]
fn test_reset_for_resync() {
    let mut orderbook = OrderBook::new("BTCUSDT".to_string(), Utc::now().timestamp_millis());

    orderbook.bids = vec![OrderBookEntry::new(
        "50000.0".parse().unwrap(),
        "1.5".parse().unwrap(),
    )];
    orderbook.asks = vec![OrderBookEntry::new(
        "50100.0".parse().unwrap(),
        "2.0".parse().unwrap(),
    )];
    orderbook.nonce = Some(100);
    orderbook.is_synced = true;
    orderbook.needs_resync = true;

    let delta = OrderBookDelta {
        symbol: "BTCUSDT".to_string(),
        first_update_id: 101,
        final_update_id: 102,
        prev_final_update_id: None,
        timestamp: chrono::Utc::now().timestamp_millis(),
        bids: vec![],
        asks: vec![],
    };
    orderbook.buffer_delta(delta);

    let buffered_count = orderbook.buffered_deltas.len();
    assert_eq!(buffered_count, 1, "Should have 1 buffered delta");

    orderbook.reset_for_resync();

    assert_eq!(orderbook.bids.len(), 0, "Bids should be cleared");
    assert_eq!(orderbook.asks.len(), 0, "Asks should be cleared");
    assert_eq!(orderbook.nonce, None, "Nonce should be reset");
    assert!(!orderbook.is_synced, "Synced status should be false");
    assert!(!orderbook.needs_resync, "needs_resync should be reset");
    assert_eq!(
        orderbook.buffered_deltas.len(),
        buffered_count,
        "Buffered deltas should be preserved for post-resync processing"
    );
}

#[test]
fn test_mark_resync_initiated() {
    let mut orderbook = OrderBook::new("BTCUSDT".to_string(), Utc::now().timestamp_millis());

    let initial_time = orderbook.last_resync_time;
    assert_eq!(initial_time, 0, "Initial resync time should be 0");

    let current_time = Utc::now().timestamp_millis();
    orderbook.mark_resync_initiated(current_time);

    assert_eq!(
        orderbook.last_resync_time, current_time,
        "Should update last_resync_time"
    );
}

#[test]
fn test_normal_sequence_no_resync() {
    let mut orderbook = OrderBook::new("BTCUSDT".to_string(), Utc::now().timestamp_millis());

    orderbook.nonce = Some(100);
    orderbook.is_synced = true;

    let delta = OrderBookDelta {
        symbol: "BTCUSDT".to_string(),
        first_update_id: 101,
        final_update_id: 102,
        prev_final_update_id: None,
        timestamp: chrono::Utc::now().timestamp_millis(),
        bids: vec![OrderBookEntry {
            price: "50000.0".parse().unwrap(),
            amount: "1.5".parse().unwrap(),
        }],
        asks: vec![OrderBookEntry {
            price: "50100.0".parse().unwrap(),
            amount: "2.0".parse().unwrap(),
        }],
    };

    let result = orderbook.apply_delta(&delta, false);

    assert!(result.is_ok(), "Normal sequence should apply successfully");
    assert!(!orderbook.needs_resync, "Should not set needs_resync flag");
    assert!(orderbook.is_synced, "Should maintain synced status");
    assert_eq!(
        orderbook.nonce,
        Some(102),
        "Nonce should update to final_update_id"
    );
}

#[test]
fn test_buffered_deltas_preserved_during_resync() {
    let mut orderbook = OrderBook::new("BTCUSDT".to_string(), Utc::now().timestamp_millis());

    for i in 1..=5 {
        let delta = OrderBookDelta {
            symbol: "BTCUSDT".to_string(),
            first_update_id: i * 10,
            final_update_id: i * 10 + 1,
            prev_final_update_id: None,
            timestamp: chrono::Utc::now().timestamp_millis(),
            bids: vec![],
            asks: vec![],
        };
        orderbook.buffer_delta(delta);
    }

    assert_eq!(
        orderbook.buffered_deltas.len(),
        5,
        "Should have 5 buffered deltas"
    );

    orderbook.needs_resync = true;
    orderbook.reset_for_resync();

    assert_eq!(
        orderbook.buffered_deltas.len(),
        5,
        "Buffer should preserve all deltas after resync"
    );
}

#[test]
fn test_should_resync_when_flag_false() {
    let orderbook = OrderBook::new("BTCUSDT".to_string(), Utc::now().timestamp_millis());

    let current_time = Utc::now().timestamp_millis();
    assert!(
        !orderbook.should_resync(current_time),
        "Should not resync when needs_resync is false"
    );
}
