//! Property-based tests for the error system.
//!
//! These tests verify the correctness properties defined in the design document
//! for the error handling system using the proptest framework.
//!
//! **Feature: code-refactoring-improvements, Property 4: Error Variants Are Displayable**
//! **Validates: Requirements 5.3**

use ccxt_core::error::{Error, NetworkError, OrderError, ParseError};
use proptest::prelude::*;
use std::error::Error as StdError;
use std::time::Duration;

// ============================================================================
// Test Generators
// ============================================================================

/// Strategy for generating random error messages.
fn error_message_strategy() -> impl Strategy<Value = String> {
    "[a-zA-Z0-9 _-]{1,100}".prop_map(|s| s.to_string())
}

/// Strategy for generating random error codes.
fn error_code_strategy() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("-1000".to_string()),
        Just("-1001".to_string()),
        Just("-1121".to_string()),
        Just("400".to_string()),
        Just("401".to_string()),
        Just("403".to_string()),
        Just("404".to_string()),
        Just("429".to_string()),
        Just("500".to_string()),
        "[0-9]{1,5}".prop_map(|s| s.to_string()),
    ]
}

/// Strategy for generating optional retry durations.
fn retry_duration_strategy() -> impl Strategy<Value = Option<Duration>> {
    prop_oneof![
        Just(None),
        (1u64..3600).prop_map(|secs| Some(Duration::from_secs(secs))),
    ]
}

/// Strategy for generating all Error variants.
fn error_strategy() -> impl Strategy<Value = Error> {
    prop_oneof![
        // Exchange error
        (error_code_strategy(), error_message_strategy())
            .prop_map(|(code, msg)| Error::exchange(code, msg)),
        // Network error
        error_message_strategy().prop_map(Error::network),
        // Authentication error
        error_message_strategy().prop_map(|msg| Error::authentication(msg)),
        // Rate limit error
        (error_message_strategy(), retry_duration_strategy())
            .prop_map(|(msg, retry)| Error::rate_limit(msg, retry)),
        // Invalid request error
        error_message_strategy().prop_map(|msg| Error::invalid_request(msg)),
        // Insufficient balance error
        error_message_strategy().prop_map(|msg| Error::insufficient_balance(msg)),
        // Invalid order error
        error_message_strategy().prop_map(|msg| Error::InvalidOrder(msg.into())),
        // Order not found error
        error_message_strategy().prop_map(|msg| Error::OrderNotFound(msg.into())),
        // Market not found error
        error_message_strategy().prop_map(|msg| Error::market_not_found(msg)),
        // Timeout error
        error_message_strategy().prop_map(|msg| Error::timeout(msg)),
        // Not implemented error
        error_message_strategy().prop_map(|msg| Error::not_implemented(msg)),
        // WebSocket error
        error_message_strategy().prop_map(Error::websocket),
    ]
}

/// Strategy for generating NetworkError variants.
fn network_error_strategy() -> impl Strategy<Value = NetworkError> {
    prop_oneof![
        (400u16..600, error_message_strategy()).prop_map(|(status, msg)| {
            NetworkError::RequestFailed {
                status,
                message: msg,
            }
        }),
        // Use prop_map with unit to create Timeout (avoids Clone requirement)
        proptest::strategy::LazyJust::new(|| NetworkError::Timeout),
        error_message_strategy().prop_map(NetworkError::ConnectionFailed),
        error_message_strategy().prop_map(NetworkError::DnsResolution),
        error_message_strategy().prop_map(NetworkError::Ssl),
    ]
}

/// Strategy for generating OrderError variants.
fn order_error_strategy() -> impl Strategy<Value = OrderError> {
    prop_oneof![
        error_message_strategy().prop_map(OrderError::CreationFailed),
        error_message_strategy().prop_map(OrderError::CancellationFailed),
        error_message_strategy().prop_map(OrderError::ModificationFailed),
        error_message_strategy().prop_map(OrderError::InvalidParameters),
    ]
}

/// Strategy for generating ParseError variants.
fn parse_error_strategy() -> impl Strategy<Value = ParseError> {
    prop_oneof![
        // Use LazyJust to avoid Clone requirement
        proptest::strategy::LazyJust::new(|| ParseError::missing_field("test_field")),
        (error_message_strategy(), error_message_strategy())
            .prop_map(|(field, msg)| ParseError::invalid_value(field, msg)),
        error_message_strategy().prop_map(|msg| ParseError::timestamp_owned(msg)),
        (error_message_strategy(), error_message_strategy())
            .prop_map(|(field, msg)| ParseError::invalid_format(field, msg)),
    ]
}

// ============================================================================
// Property 4: Display and Box<dyn Error> Conversion Tests
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// **Feature: code-refactoring-improvements, Property 4: Error Display**
    /// **Validates: Requirements 5.3**
    ///
    /// *For any* error variant, calling `Display::fmt()` SHALL produce a non-empty string.
    #[test]
    fn prop_error_display_non_empty(error in error_strategy()) {
        let display = format!("{}", error);
        prop_assert!(
            !display.is_empty(),
            "Error display should not be empty: {:?}",
            error
        );
    }

    /// **Feature: code-refactoring-improvements, Property 4: Error Box Conversion**
    /// **Validates: Requirements 5.3**
    ///
    /// *For any* error variant, the error SHALL be convertible to `Box<dyn std::error::Error>`.
    #[test]
    fn prop_error_box_conversion(error in error_strategy()) {
        // Convert to Box<dyn Error>
        let boxed: Box<dyn StdError + Send + Sync + 'static> = Box::new(error);

        // Verify the boxed error can be displayed
        let display = boxed.to_string();
        prop_assert!(
            !display.is_empty(),
            "Boxed error display should not be empty"
        );
    }

    /// **Feature: code-refactoring-improvements, Property 4: Error Debug**
    /// **Validates: Requirements 5.3**
    ///
    /// *For any* error variant, the Debug trait should produce a non-empty string.
    #[test]
    fn prop_error_debug_non_empty(error in error_strategy()) {
        let debug = format!("{:?}", error);
        prop_assert!(
            !debug.is_empty(),
            "Error debug should not be empty"
        );
    }

    /// **Feature: code-refactoring-improvements, Property 4: NetworkError Display**
    /// **Validates: Requirements 5.3**
    ///
    /// *For any* NetworkError variant, Display should produce a non-empty string.
    #[test]
    fn prop_network_error_display_non_empty(error in network_error_strategy()) {
        let display = format!("{}", error);
        prop_assert!(
            !display.is_empty(),
            "NetworkError display should not be empty: {:?}",
            error
        );
    }

    /// **Feature: code-refactoring-improvements, Property 4: OrderError Display**
    /// **Validates: Requirements 5.3**
    ///
    /// *For any* OrderError variant, Display should produce a non-empty string.
    #[test]
    fn prop_order_error_display_non_empty(error in order_error_strategy()) {
        let display = format!("{}", error);
        prop_assert!(
            !display.is_empty(),
            "OrderError display should not be empty: {:?}",
            error
        );
    }

    /// **Feature: code-refactoring-improvements, Property 4: ParseError Display**
    /// **Validates: Requirements 5.3**
    ///
    /// *For any* ParseError variant, Display should produce a non-empty string.
    #[test]
    fn prop_parse_error_display_non_empty(error in parse_error_strategy()) {
        let display = format!("{}", error);
        prop_assert!(
            !display.is_empty(),
            "ParseError display should not be empty: {:?}",
            error
        );
    }

    /// **Feature: code-refactoring-improvements, Property 4: Error Conversion from NetworkError**
    /// **Validates: Requirements 5.3**
    ///
    /// *For any* NetworkError, it should be convertible to Error.
    #[test]
    fn prop_network_error_to_error_conversion(error in network_error_strategy()) {
        let converted: Error = error.into();
        let display = format!("{}", converted);
        prop_assert!(
            !display.is_empty(),
            "Converted NetworkError display should not be empty"
        );
    }

    /// **Feature: code-refactoring-improvements, Property 4: Error Conversion from OrderError**
    /// **Validates: Requirements 5.3**
    ///
    /// *For any* OrderError, it should be convertible to Error.
    #[test]
    fn prop_order_error_to_error_conversion(error in order_error_strategy()) {
        let converted: Error = error.into();
        let display = format!("{}", converted);
        prop_assert!(
            !display.is_empty(),
            "Converted OrderError display should not be empty"
        );
    }

    /// **Feature: code-refactoring-improvements, Property 4: Error Conversion from ParseError**
    /// **Validates: Requirements 5.3**
    ///
    /// *For any* ParseError, it should be convertible to Error.
    #[test]
    fn prop_parse_error_to_error_conversion(error in parse_error_strategy()) {
        let converted: Error = error.into();
        let display = format!("{}", converted);
        prop_assert!(
            !display.is_empty(),
            "Converted ParseError display should not be empty"
        );
    }

    /// **Feature: code-refactoring-improvements, Property 4: Error Report Generation**
    /// **Validates: Requirements 5.3**
    ///
    /// *For any* error variant, the report() method should produce a non-empty string.
    #[test]
    fn prop_error_report_non_empty(error in error_strategy()) {
        let report = error.report();
        prop_assert!(
            !report.is_empty(),
            "Error report should not be empty"
        );
    }
}

// ============================================================================
// Additional Error Properties
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// *For any* error with context, the context should be preserved in display.
    #[test]
    fn prop_error_context_preserved(
        base_error in error_strategy(),
        context_msg in error_message_strategy()
    ) {
        let with_context = base_error.context(context_msg.clone());
        let display = format!("{}", with_context);

        prop_assert!(
            display.contains(&context_msg),
            "Context '{}' should be in display '{}'",
            context_msg, display
        );
    }

    /// *For any* rate limit error with retry_after, the retry duration should be retrievable.
    #[test]
    fn prop_rate_limit_retry_after_retrievable(
        msg in error_message_strategy(),
        secs in 1u64..3600
    ) {
        let duration = Duration::from_secs(secs);
        let error = Error::rate_limit(msg, Some(duration));

        let retrieved = error.retry_after();
        prop_assert_eq!(
            retrieved, Some(duration),
            "retry_after should return the set duration"
        );
    }

    /// *For any* retryable error, is_retryable() should return true.
    #[test]
    fn prop_retryable_errors_identified(
        msg in error_message_strategy(),
        retry in retry_duration_strategy()
    ) {
        // Rate limit errors are retryable
        let rate_limit = Error::rate_limit(msg.clone(), retry);
        prop_assert!(
            rate_limit.is_retryable(),
            "Rate limit error should be retryable"
        );

        // Timeout errors are retryable
        let timeout = Error::timeout(msg);
        prop_assert!(
            timeout.is_retryable(),
            "Timeout error should be retryable"
        );
    }
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn test_error_display_basic() {
        let err = Error::authentication("Invalid API key");
        assert!(err.to_string().contains("Invalid API key"));
    }

    #[test]
    fn test_error_box_conversion_basic() {
        let err = Error::timeout("Operation timed out");
        let boxed: Box<dyn StdError> = Box::new(err);
        assert!(!boxed.to_string().is_empty());
    }

    #[test]
    fn test_error_context_chain() {
        let err = Error::network("Connection refused").context("Failed to fetch ticker");
        let report = err.report();
        assert!(report.contains("Failed to fetch ticker"));
        assert!(report.contains("Connection refused"));
    }
}
