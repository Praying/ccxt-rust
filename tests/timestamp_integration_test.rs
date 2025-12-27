//! Integration tests for timestamp handling across different exchange implementations
//!
//! These tests verify that timestamp handling is consistent across all exchange implementations
//! and that the i64 timestamp standardization works correctly end-to-end.

use ccxt_core::{
    Result,
    time::TimestampUtils,
    types::{
        Amount, Ohlcv, Order, OrderSide, OrderStatus, OrderType, Price, Timestamp, Trade,
        Transaction, TransactionStatus, TransactionType,
    },
};

/// Test that all exchanges handle i64 timestamps consistently in OHLCV data
#[tokio::test]
async fn test_ohlcv_timestamp_consistency() -> Result<()> {
    // Test with a known timestamp (January 1, 2023, 00:00:00 UTC)
    let test_timestamp: Timestamp = 1672531200000;

    // Create mock OHLCV data with i64 timestamp
    let ohlcv = Ohlcv {
        timestamp: test_timestamp,
        open: Price::new(rust_decimal::Decimal::from(50000)),
        high: Price::new(rust_decimal::Decimal::from(51000)),
        low: Price::new(rust_decimal::Decimal::from(49000)),
        close: Price::new(rust_decimal::Decimal::from(50500)),
        volume: Amount::new(rust_decimal::Decimal::from(100)),
    };

    // Verify timestamp is preserved correctly
    assert_eq!(ohlcv.timestamp, test_timestamp);

    // Verify timestamp is within reasonable bounds
    assert!(TimestampUtils::validate_timestamp(ohlcv.timestamp).is_ok());

    // Verify timestamp can be formatted correctly
    let formatted = TimestampUtils::format_iso8601(ohlcv.timestamp)?;
    assert!(formatted.contains("2023-01-01"));

    Ok(())
}

/// Test that all exchanges handle i64 timestamps consistently in Trade data
#[tokio::test]
async fn test_trade_timestamp_consistency() -> Result<()> {
    let test_timestamp: i64 = 1672531200000;

    // Create mock trade data with i64 timestamps
    let trade = Trade {
        id: Some("test_trade_123".to_string()),
        order: Some("order_456".to_string()),
        symbol: "BTC/USDT".to_string(),
        trade_type: Some(OrderType::Market),
        side: OrderSide::Buy,
        taker_or_maker: None,
        price: Price::new(rust_decimal::Decimal::from(50000)),
        amount: Amount::new(rust_decimal::Decimal::from(1)),
        cost: None,
        fee: None,
        timestamp: test_timestamp,
        datetime: Some("2023-01-01T00:00:00.000Z".to_string()),
        info: std::collections::HashMap::new(),
    };

    // Verify timestamp is i64 and valid
    assert_eq!(trade.timestamp, test_timestamp);

    // Verify timestamps are valid
    assert!(TimestampUtils::validate_timestamp(trade.timestamp).is_ok());

    Ok(())
}

/// Test that all exchanges handle i64 timestamps consistently in Order data
#[tokio::test]
async fn test_order_timestamp_consistency() -> Result<()> {
    let test_timestamp: i64 = 1672531200000;
    let updated_timestamp: i64 = test_timestamp + 60000; // 1 minute later

    // Create mock order data with i64 timestamps
    let order = Order {
        id: "order_123".to_string(),
        client_order_id: Some("client_456".to_string()),
        timestamp: Some(test_timestamp),
        datetime: Some("2023-01-01T00:00:00.000Z".to_string()),
        last_trade_timestamp: Some(updated_timestamp),
        symbol: "BTC/USDT".to_string(),
        order_type: OrderType::Limit,
        time_in_force: None,
        post_only: None,
        reduce_only: None,
        side: OrderSide::Buy,
        price: Some(rust_decimal::Decimal::from(50000)),
        stop_price: None,
        trigger_price: None,
        take_profit_price: None,
        stop_loss_price: None,
        trailing_delta: None,
        trailing_percent: None,
        activation_price: None,
        callback_rate: None,
        working_type: None,
        amount: rust_decimal::Decimal::from(1),
        filled: Some(rust_decimal::Decimal::ZERO),
        remaining: Some(rust_decimal::Decimal::from(1)),
        cost: None,
        average: None,
        status: OrderStatus::Open,
        fee: None,
        fees: None,
        trades: None,
        info: std::collections::HashMap::new(),
    };

    // Verify all timestamp fields are i64
    assert_eq!(order.timestamp, Some(test_timestamp));
    assert_eq!(order.last_trade_timestamp, Some(updated_timestamp));

    // Verify all timestamps are valid
    if let Some(ts) = order.timestamp {
        assert!(TimestampUtils::validate_timestamp(ts).is_ok());
    }
    if let Some(ts) = order.last_trade_timestamp {
        assert!(TimestampUtils::validate_timestamp(ts).is_ok());
    }

    Ok(())
}

/// Test that all exchanges handle i64 timestamps consistently in Transaction data
#[tokio::test]
async fn test_transaction_timestamp_consistency() -> Result<()> {
    let test_timestamp: i64 = 1672531200000;
    let updated_timestamp: i64 = test_timestamp + 60000;

    // Create mock transaction data with i64 timestamps
    let transaction = Transaction {
        info: None,
        id: "tx_123".to_string(),
        txid: Some("blockchain_tx_456".to_string()),
        timestamp: Some(test_timestamp),
        datetime: Some("2023-01-01T00:00:00.000Z".to_string()),
        network: Some("BTC".to_string()),
        address: Some("bc1qxy2kgdygjrsqtzq2n0yrf2493p83kkfjhx0wlh".to_string()),
        address_to: None,
        address_from: None,
        tag: None,
        tag_to: None,
        tag_from: None,
        transaction_type: TransactionType::Deposit,
        amount: rust_decimal::Decimal::from(1),
        currency: "BTC".to_string(),
        status: TransactionStatus::Ok,
        updated: Some(updated_timestamp),
        internal: None,
        comment: None,
        fee: None,
    };

    // Verify all timestamp fields are i64
    assert_eq!(transaction.timestamp, Some(test_timestamp));
    assert_eq!(transaction.updated, Some(updated_timestamp));

    // Verify all timestamps are valid
    if let Some(ts) = transaction.timestamp {
        assert!(TimestampUtils::validate_timestamp(ts).is_ok());
    }
    if let Some(ts) = transaction.updated {
        assert!(TimestampUtils::validate_timestamp(ts).is_ok());
    }

    Ok(())
}

/// Test timestamp conversion utilities work correctly
#[tokio::test]
async fn test_timestamp_conversion_utilities() -> Result<()> {
    let current_time = TimestampUtils::now_ms();

    // Test that current time is reasonable (not negative, not too far in future)
    assert!(current_time > 0);
    assert!(current_time < 4_102_444_800_000); // Before year 2100

    // Test seconds to milliseconds conversion
    let seconds = 1672531200; // January 1, 2023 in seconds
    let millis = TimestampUtils::seconds_to_ms(seconds);
    assert_eq!(millis, 1672531200000);

    // Test milliseconds to seconds conversion
    let back_to_seconds = TimestampUtils::ms_to_seconds(millis);
    assert_eq!(back_to_seconds, seconds);

    // Test timestamp validation
    assert!(TimestampUtils::validate_timestamp(1672531200000).is_ok()); // Valid timestamp
    assert!(TimestampUtils::validate_timestamp(-1).is_err()); // Negative timestamp
    assert!(TimestampUtils::validate_timestamp(5_000_000_000_000).is_err()); // Too far in future

    // Test ISO8601 formatting
    let formatted = TimestampUtils::format_iso8601(1672531200000)?;
    assert!(formatted.contains("2023-01-01T00:00:00"));

    Ok(())
}

/// Test that timestamp parsing from strings works correctly
#[tokio::test]
async fn test_timestamp_parsing() -> Result<()> {
    // Test parsing integer timestamp strings
    let parsed = TimestampUtils::parse_timestamp("1672531200000")?;
    assert_eq!(parsed, 1672531200000);

    // Test parsing decimal timestamp strings (seconds with decimals)
    let parsed_decimal = TimestampUtils::parse_timestamp("1672531200.123")?;
    assert_eq!(parsed_decimal, 1672531200123);

    // Test that invalid strings return errors
    assert!(TimestampUtils::parse_timestamp("invalid").is_err());
    assert!(TimestampUtils::parse_timestamp("").is_err());

    Ok(())
}

/// Test end-to-end timestamp flow with mock data
#[tokio::test]
async fn test_end_to_end_timestamp_flow() -> Result<()> {
    // Simulate the flow: API response -> parsing -> data structure -> validation

    // 1. Start with a timestamp from an API (as string, like real APIs return)
    let api_timestamp_str = "1672531200000";

    // 2. Parse it to i64 (like exchange parsers do)
    let parsed_timestamp = TimestampUtils::parse_timestamp(api_timestamp_str)?;

    // 3. Create data structures with the parsed timestamp
    let ohlcv = Ohlcv {
        timestamp: parsed_timestamp,
        open: Price::new(rust_decimal::Decimal::from(50000)),
        high: Price::new(rust_decimal::Decimal::from(51000)),
        low: Price::new(rust_decimal::Decimal::from(49000)),
        close: Price::new(rust_decimal::Decimal::from(50500)),
        volume: Amount::new(rust_decimal::Decimal::from(100)),
    };

    // 4. Validate the timestamp in the data structure
    assert!(TimestampUtils::validate_timestamp(ohlcv.timestamp).is_ok());

    // 5. Format it back for display/logging
    let formatted = TimestampUtils::format_iso8601(ohlcv.timestamp)?;
    assert!(formatted.contains("2023-01-01"));

    // 6. Verify the round-trip consistency
    assert_eq!(ohlcv.timestamp, parsed_timestamp);
    assert_eq!(parsed_timestamp, 1672531200000);

    Ok(())
}

/// Test that deprecated conversion functions still work for backward compatibility
#[tokio::test]
#[allow(deprecated)] // Testing deprecated conversion functions for backward compatibility
async fn test_backward_compatibility_conversions() -> Result<()> {
    // Test u64 to i64 conversion (deprecated but should still work)
    let u64_timestamp: u64 = 1672531200000;
    let converted = TimestampUtils::u64_to_i64(u64_timestamp)?;
    assert_eq!(converted, 1672531200000i64);

    // Test i64 to u64 conversion (deprecated but should still work)
    let i64_timestamp: i64 = 1672531200000;
    let converted_back = TimestampUtils::i64_to_u64(i64_timestamp)?;
    assert_eq!(converted_back, 1672531200000u64);

    // Test overflow handling
    let large_u64: u64 = u64::MAX;
    assert!(TimestampUtils::u64_to_i64(large_u64).is_err());

    // Test underflow handling
    let negative_i64: i64 = -1;
    assert!(TimestampUtils::i64_to_u64(negative_i64).is_err());

    Ok(())
}

/// Test timestamp consistency across different data types
#[tokio::test]
async fn test_cross_type_timestamp_consistency() -> Result<()> {
    let base_timestamp: i64 = 1672531200000;

    // Create different data types with the same timestamp
    let ohlcv = Ohlcv {
        timestamp: base_timestamp,
        open: Price::new(rust_decimal::Decimal::from(50000)),
        high: Price::new(rust_decimal::Decimal::from(51000)),
        low: Price::new(rust_decimal::Decimal::from(49000)),
        close: Price::new(rust_decimal::Decimal::from(50500)),
        volume: Amount::new(rust_decimal::Decimal::from(100)),
    };

    let trade = Trade {
        id: Some("test_trade".to_string()),
        order: None,
        symbol: "BTC/USDT".to_string(),
        trade_type: Some(OrderType::Market),
        side: OrderSide::Buy,
        taker_or_maker: None,
        price: Price::new(rust_decimal::Decimal::from(50000)),
        amount: Amount::new(rust_decimal::Decimal::from(1)),
        cost: None,
        fee: None,
        timestamp: base_timestamp,
        datetime: Some("2023-01-01T00:00:00.000Z".to_string()),
        info: std::collections::HashMap::new(),
    };

    let order = Order {
        id: "test_order".to_string(),
        client_order_id: None,
        timestamp: Some(base_timestamp),
        datetime: Some("2023-01-01T00:00:00.000Z".to_string()),
        last_trade_timestamp: None,
        symbol: "BTC/USDT".to_string(),
        order_type: OrderType::Limit,
        time_in_force: None,
        post_only: None,
        reduce_only: None,
        side: OrderSide::Buy,
        price: Some(rust_decimal::Decimal::from(50000)),
        stop_price: None,
        trigger_price: None,
        take_profit_price: None,
        stop_loss_price: None,
        trailing_delta: None,
        trailing_percent: None,
        activation_price: None,
        callback_rate: None,
        working_type: None,
        amount: rust_decimal::Decimal::from(1),
        filled: Some(rust_decimal::Decimal::ZERO),
        remaining: Some(rust_decimal::Decimal::from(1)),
        cost: None,
        average: None,
        status: OrderStatus::Open,
        fee: None,
        fees: None,
        trades: None,
        info: std::collections::HashMap::new(),
    };

    // Verify all timestamps are consistent
    assert_eq!(ohlcv.timestamp, base_timestamp);
    assert_eq!(trade.timestamp, base_timestamp);
    assert_eq!(order.timestamp, Some(base_timestamp));

    // Verify all are the same value
    assert_eq!(ohlcv.timestamp, trade.timestamp);
    if let Some(order_timestamp) = order.timestamp {
        assert_eq!(trade.timestamp, order_timestamp);
    }

    Ok(())
}
