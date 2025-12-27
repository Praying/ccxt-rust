//! Edge case and error condition tests for timestamp handling
//!
//! These tests verify that the timestamp system handles edge cases correctly,
//! including overflow/underflow scenarios and invalid input validation.

use ccxt_core::{Result, time::TimestampUtils};

/// Test timestamp overflow scenarios
#[tokio::test]
async fn test_timestamp_overflow_scenarios() -> Result<()> {
    // Test maximum valid timestamp (year 2100)
    let max_valid = TimestampUtils::YEAR_2100_MS; // January 1, 2100
    assert!(TimestampUtils::validate_timestamp(max_valid).is_ok());

    // Test timestamp beyond year 2100 (should fail)
    let beyond_max = TimestampUtils::YEAR_2100_MS + 1;
    assert!(TimestampUtils::validate_timestamp(beyond_max).is_err());

    // Test i64::MAX (should fail)
    assert!(TimestampUtils::validate_timestamp(i64::MAX).is_err());

    // Test very large timestamp (should fail)
    let very_large = 9_999_999_999_999i64;
    assert!(TimestampUtils::validate_timestamp(very_large).is_err());

    Ok(())
}

/// Test timestamp underflow scenarios
#[tokio::test]
async fn test_timestamp_underflow_scenarios() -> Result<()> {
    // Test minimum valid timestamp (Unix epoch)
    let min_valid = 0i64;
    assert!(TimestampUtils::validate_timestamp(min_valid).is_ok());

    // Test negative timestamps (should fail)
    assert!(TimestampUtils::validate_timestamp(-1).is_err());
    assert!(TimestampUtils::validate_timestamp(-1000).is_err());
    assert!(TimestampUtils::validate_timestamp(i64::MIN).is_err());

    Ok(())
}

/// Test invalid timestamp parsing scenarios
#[tokio::test]
async fn test_invalid_timestamp_parsing() {
    // Test empty string
    assert!(TimestampUtils::parse_timestamp("").is_err());

    // Test non-numeric strings
    assert!(TimestampUtils::parse_timestamp("invalid").is_err());
    assert!(TimestampUtils::parse_timestamp("abc123").is_err());
    assert!(TimestampUtils::parse_timestamp("not_a_number").is_err());

    // Test strings with special characters
    assert!(TimestampUtils::parse_timestamp("123.456.789").is_err());
    assert!(TimestampUtils::parse_timestamp("123,456").is_err());
    assert!(TimestampUtils::parse_timestamp("1e10").is_err()); // Scientific notation

    // Test strings with whitespace
    assert!(TimestampUtils::parse_timestamp(" 1672531200000 ").is_err());
    assert!(TimestampUtils::parse_timestamp("1672531200000\n").is_err());
    assert!(TimestampUtils::parse_timestamp("\t1672531200000").is_err());

    // Test very long strings
    let long_string = "1".repeat(100);
    assert!(TimestampUtils::parse_timestamp(&long_string).is_err());
}

/// Test valid edge case parsing scenarios
#[tokio::test]
async fn test_valid_edge_case_parsing() -> Result<()> {
    // Test Unix epoch
    let epoch = TimestampUtils::parse_timestamp("0")?;
    assert_eq!(epoch, 0);

    // Test decimal seconds (should convert to milliseconds)
    let decimal_seconds = TimestampUtils::parse_timestamp("1672531200.123")?;
    assert_eq!(decimal_seconds, 1672531200123);

    // Test decimal seconds with more precision (should truncate)
    let high_precision = TimestampUtils::parse_timestamp("1672531200.123456")?;
    assert_eq!(high_precision, 1672531200123);

    // Test integer milliseconds
    let millis = TimestampUtils::parse_timestamp("1672531200000")?;
    assert_eq!(millis, 1672531200000);

    Ok(())
}

/// Test conversion edge cases
#[tokio::test]
#[allow(deprecated)] // Testing deprecated conversion functions for edge cases
async fn test_conversion_edge_cases() -> Result<()> {
    // Test u64 to i64 conversion at boundary
    let max_safe_u64 = (i64::MAX as u64).min(TimestampUtils::YEAR_2100_MS as u64);
    let converted = TimestampUtils::u64_to_i64(max_safe_u64)?;
    assert_eq!(converted, max_safe_u64 as i64);

    // Test u64 overflow (should fail)
    let overflow_u64 = (i64::MAX as u64) + 1;
    assert!(TimestampUtils::u64_to_i64(overflow_u64).is_err());

    // Test i64 to u64 conversion at boundary
    let max_valid_i64 = TimestampUtils::YEAR_2100_MS;
    let converted_back = TimestampUtils::i64_to_u64(max_valid_i64)?;
    assert_eq!(converted_back, max_valid_i64 as u64);

    // Test negative i64 (should fail)
    assert!(TimestampUtils::i64_to_u64(-1).is_err());
    assert!(TimestampUtils::i64_to_u64(i64::MIN).is_err());

    Ok(())
}

/// Test seconds/milliseconds conversion edge cases
#[tokio::test]
async fn test_time_unit_conversion_edge_cases() {
    // Test seconds to milliseconds overflow
    let max_seconds = i64::MAX / 1000;
    let converted = TimestampUtils::seconds_to_ms(max_seconds);
    assert_eq!(converted, max_seconds * 1000);

    // Test milliseconds to seconds precision loss
    let millis_with_remainder = 1672531200123i64;
    let seconds = TimestampUtils::ms_to_seconds(millis_with_remainder);
    assert_eq!(seconds, 1672531200); // Should truncate, not round

    // Test zero conversions
    assert_eq!(TimestampUtils::seconds_to_ms(0), 0);
    assert_eq!(TimestampUtils::ms_to_seconds(0), 0);

    // Test negative conversions (should work mathematically)
    assert_eq!(TimestampUtils::seconds_to_ms(-1), -1000);
    assert_eq!(TimestampUtils::ms_to_seconds(-1000), -1);
}

/// Test ISO8601 formatting edge cases
#[tokio::test]
async fn test_iso8601_formatting_edge_cases() -> Result<()> {
    // Test Unix epoch formatting
    let epoch_formatted = TimestampUtils::format_iso8601(0)?;
    assert_eq!(epoch_formatted, "1970-01-01T00:00:00.000Z");

    // Test millisecond precision
    let with_millis = TimestampUtils::format_iso8601(1672531200123)?;
    assert!(with_millis.contains(".123Z"));

    // Test year 2000 (Y2K)
    let y2k = 946684800000i64; // January 1, 2000
    let y2k_formatted = TimestampUtils::format_iso8601(y2k)?;
    assert!(y2k_formatted.contains("2000-01-01"));

    // Test leap year (February 29, 2024)
    let leap_day = 1709164800000i64; // February 29, 2024
    let leap_formatted = TimestampUtils::format_iso8601(leap_day)?;
    assert!(leap_formatted.contains("2024-02-29"));

    // Test invalid timestamp formatting (should fail validation first)
    assert!(TimestampUtils::format_iso8601(-1).is_err());
    assert!(TimestampUtils::format_iso8601(i64::MAX).is_err());

    Ok(())
}

/// Test timestamp validation boundary conditions
#[tokio::test]
async fn test_timestamp_validation_boundaries() {
    // Test exactly at boundaries
    assert!(TimestampUtils::validate_timestamp(0).is_ok()); // Unix epoch
    assert!(TimestampUtils::validate_timestamp(TimestampUtils::YEAR_2100_MS).is_ok()); // Year 2100

    // Test just outside boundaries
    assert!(TimestampUtils::validate_timestamp(-1).is_err()); // Just before Unix epoch
    assert!(TimestampUtils::validate_timestamp(TimestampUtils::YEAR_2100_MS + 1).is_err()); // Just after year 2100

    // Test reasonable timestamp ranges
    let year_2020 = 1577836800000i64; // January 1, 2020
    let year_2030 = 1893456000000i64; // January 1, 2030
    assert!(TimestampUtils::validate_timestamp(year_2020).is_ok());
    assert!(TimestampUtils::validate_timestamp(year_2030).is_ok());

    // Test current time should be valid
    let now = TimestampUtils::now_ms();
    assert!(TimestampUtils::validate_timestamp(now).is_ok());
}

/// Test error message quality for debugging
#[tokio::test]
async fn test_error_message_quality() {
    // Test that error messages are helpful for debugging
    match TimestampUtils::validate_timestamp(-1) {
        Err(e) => {
            let error_msg = format!("{}", e);
            assert!(error_msg.contains("negative") || error_msg.contains("invalid"));
        }
        Ok(_) => panic!("Expected error for negative timestamp"),
    }

    match TimestampUtils::validate_timestamp(i64::MAX) {
        Err(e) => {
            let error_msg = format!("{}", e);
            assert!(
                error_msg.contains("future")
                    || error_msg.contains("invalid")
                    || error_msg.contains("range")
            );
        }
        Ok(_) => panic!("Expected error for timestamp too far in future"),
    }

    match TimestampUtils::parse_timestamp("invalid") {
        Err(e) => {
            let error_msg = format!("{}", e);
            assert!(
                error_msg.contains("parse")
                    || error_msg.contains("invalid")
                    || error_msg.contains("format")
            );
        }
        Ok(_) => panic!("Expected error for invalid timestamp string"),
    }
}

/// Test concurrent timestamp operations (basic thread safety)
#[tokio::test]
async fn test_concurrent_timestamp_operations() -> Result<()> {
    use std::sync::Arc;
    use tokio::task;

    let test_timestamps = Arc::new(vec![
        0i64,
        1672531200000i64,
        1577836800000i64,
        1893456000000i64,
    ]);

    let mut handles = vec![];

    // Spawn multiple tasks to test concurrent access
    for _i in 0..10 {
        let timestamps = Arc::clone(&test_timestamps);
        let handle = task::spawn(async move {
            for &ts in timestamps.iter() {
                // Test validation
                assert!(TimestampUtils::validate_timestamp(ts).is_ok());

                // Test formatting
                let formatted = TimestampUtils::format_iso8601(ts)?;
                assert!(!formatted.is_empty());

                // Test conversions
                let seconds = TimestampUtils::ms_to_seconds(ts);
                let back_to_ms = TimestampUtils::seconds_to_ms(seconds);
                assert_eq!(back_to_ms / 1000 * 1000, ts / 1000 * 1000); // Account for precision loss
            }
            Ok::<(), ccxt_core::Error>(())
        });
        handles.push(handle);
    }

    // Wait for all tasks to complete
    for handle in handles {
        if let Err(e) = handle.await {
            return Err(ccxt_core::Error::network(format!("Task join error: {}", e)));
        }
    }

    Ok(())
}

/// Test memory usage with large timestamp operations
#[tokio::test]
async fn test_memory_efficiency() -> Result<()> {
    // Test that timestamp operations don't leak memory with large datasets
    let large_dataset: Vec<i64> = (0..10000)
        .map(|i| 1672531200000i64 + (i * 60000)) // Every minute for ~7 days
        .collect();

    for &timestamp in &large_dataset {
        // These operations should be efficient and not accumulate memory
        let _ = TimestampUtils::validate_timestamp(timestamp)?;
        let formatted = TimestampUtils::format_iso8601(timestamp)?;
        let _ = TimestampUtils::parse_timestamp(&timestamp.to_string())?;

        // Verify the formatted string is reasonable
        assert!(formatted.len() > 20 && formatted.len() < 30);
    }

    Ok(())
}

/// Test timestamp precision and accuracy
#[tokio::test]
async fn test_timestamp_precision() -> Result<()> {
    // Test that millisecond precision is maintained
    let base_timestamp = 1672531200000i64;

    for millis in 0..1000 {
        let timestamp_with_millis = base_timestamp + millis;

        // Validate
        assert!(TimestampUtils::validate_timestamp(timestamp_with_millis).is_ok());

        // Format and check millisecond precision
        let formatted = TimestampUtils::format_iso8601(timestamp_with_millis)?;
        let expected_millis = format!(".{:03}Z", millis);
        assert!(formatted.ends_with(&expected_millis));

        // Test round-trip through string parsing
        let parsed = TimestampUtils::parse_timestamp(&timestamp_with_millis.to_string())?;
        assert_eq!(parsed, timestamp_with_millis);
    }

    Ok(())
}
