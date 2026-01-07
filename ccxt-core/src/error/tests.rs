#![allow(clippy::disallowed_methods)] // unwrap() is acceptable in tests
#![allow(clippy::uninlined_format_args)] // format!("{}", x) is acceptable in tests
#![allow(clippy::manual_string_new)] // "".to_string() is acceptable in tests
#![allow(clippy::redundant_closure)] // closures are acceptable in tests
#![allow(clippy::unit_cmp)] // () comparison is acceptable in tests
#![allow(clippy::ignored_unit_patterns)] // _ for () is acceptable in tests
#![allow(clippy::items_after_statements)] // items after statements is acceptable in tests
#![allow(clippy::assertions_on_constants)] // const assertions are acceptable in tests
#![allow(clippy::len_zero)] // len() > 0 is acceptable in tests
#![allow(clippy::io_other_error)] // io::Error::new is acceptable in tests

use super::convert::{MAX_ERROR_MESSAGE_LEN, truncate_message};
use super::*;
use std::time::Duration;

#[test]
fn test_exchange_error_details_display() {
    let details = ExchangeErrorDetails::new("400", "Bad Request");
    let display = format!("{details}");
    assert!(display.contains("400"));
    assert!(display.contains("Bad Request"));
}

#[test]
fn test_exchange_error_details_with_data() {
    let data = serde_json::json!({"error": "test"});
    let details = ExchangeErrorDetails::with_data("500", "Internal Error", data.clone());
    assert_eq!(details.code, "500");
    assert_eq!(details.message, "Internal Error");
    assert_eq!(details.data, Some(data));
}

#[test]
fn test_error_exchange_creation() {
    let err = Error::exchange("400", "Bad Request");
    if let Error::Exchange(details) = &err {
        assert_eq!(details.code, "400");
        assert_eq!(details.message, "Bad Request");
    } else {
        panic!("Expected Exchange variant");
    }
}

#[test]
fn test_error_exchange_string_code() {
    // Test that string codes (not just numeric) work
    let err = Error::exchange("INVALID_SYMBOL", "Symbol not found");
    if let Error::Exchange(details) = &err {
        assert_eq!(details.code, "INVALID_SYMBOL");
    } else {
        panic!("Expected Exchange variant");
    }
}

#[test]
fn test_error_authentication() {
    let err = Error::authentication("Invalid API key");
    assert!(matches!(err, Error::Authentication(_)));
    assert!(err.to_string().contains("Invalid API key"));
}

#[test]
fn test_error_rate_limit() {
    let err = Error::rate_limit("Too many requests", Some(Duration::from_secs(60)));
    if let Error::RateLimit {
        message,
        retry_after,
    } = &err
    {
        assert_eq!(message.as_ref(), "Too many requests");
        assert_eq!(*retry_after, Some(Duration::from_secs(60)));
    } else {
        panic!("Expected RateLimit variant");
    }
}

#[test]
fn test_error_market_not_found() {
    let err = Error::market_not_found("BTC/USDT");
    assert!(matches!(err, Error::MarketNotFound(_)));
    assert!(err.to_string().contains("BTC/USDT"));
}

#[test]
fn test_error_context() {
    let base = Error::network("Connection refused");
    let with_context = base.context("Failed to fetch ticker");

    assert!(matches!(with_context, Error::Context { .. }));
    assert!(with_context.to_string().contains("Failed to fetch ticker"));
}

#[test]
fn test_error_context_chain() {
    let base = Error::network("Connection refused");
    let ctx1 = base.context("Layer 1");
    let ctx2 = ctx1.context("Layer 2");

    // Check that report contains all layers
    let report = ctx2.report();
    assert!(report.contains("Layer 2"));
    assert!(report.contains("Layer 1"));
    assert!(report.contains("Connection refused"));
}

#[test]
fn test_error_root_cause() {
    let base = Error::network("Connection refused");
    let ctx1 = base.context("Layer 1");
    let ctx2 = ctx1.context("Layer 2");

    let root = ctx2.root_cause();
    assert!(matches!(root, Error::Network(_)));
}

#[test]
fn test_error_is_retryable() {
    // Retryable errors
    assert!(Error::rate_limit("test", None).is_retryable());
    assert!(Error::timeout("test").is_retryable());
    assert!(Error::from(NetworkError::Timeout).is_retryable());
    assert!(Error::from(NetworkError::ConnectionFailed("test".to_string())).is_retryable());

    // Non-retryable errors
    assert!(!Error::authentication("test").is_retryable());
    assert!(!Error::invalid_request("test").is_retryable());
    assert!(!Error::market_not_found("test").is_retryable());
}

#[test]
fn test_error_is_retryable_through_context() {
    let err = Error::rate_limit("test", Some(Duration::from_secs(30)))
        .context("Layer 1")
        .context("Layer 2");

    assert!(err.is_retryable());
    assert_eq!(err.retry_after(), Some(Duration::from_secs(30)));
}

#[test]
fn test_error_as_rate_limit() {
    let err = Error::rate_limit("Too many requests", Some(Duration::from_secs(60)));
    let (msg, retry) = err.as_rate_limit().unwrap();
    assert_eq!(msg, "Too many requests");
    assert_eq!(retry, Some(Duration::from_secs(60)));
}

#[test]
fn test_error_as_rate_limit_through_context() {
    let err =
        Error::rate_limit("Too many requests", Some(Duration::from_secs(60))).context("Wrapped");
    let (msg, retry) = err.as_rate_limit().unwrap();
    assert_eq!(msg, "Too many requests");
    assert_eq!(retry, Some(Duration::from_secs(60)));
}

#[test]
fn test_error_as_authentication() {
    let err = Error::authentication("Invalid key");
    assert_eq!(err.as_authentication(), Some("Invalid key"));
}

#[test]
fn test_error_as_authentication_through_context() {
    let err = Error::authentication("Invalid key").context("Wrapped");
    assert_eq!(err.as_authentication(), Some("Invalid key"));
}

#[test]
fn test_network_error_request_failed() {
    let err = NetworkError::RequestFailed {
        status: 404,
        message: "Not Found".to_string(),
    };
    assert!(err.to_string().contains("404"));
    assert!(err.to_string().contains("Not Found"));
}

#[test]
fn test_parse_error_missing_field() {
    let err = ParseError::missing_field("price");
    assert!(err.to_string().contains("price"));
}

#[test]
fn test_parse_error_invalid_value() {
    let err = ParseError::invalid_value("amount", "must be positive");
    let display = err.to_string();
    assert!(display.contains("amount"));
    assert!(display.contains("must be positive"));
}

#[test]
fn test_error_display() {
    let err = Error::exchange("400", "Bad Request");
    let display = format!("{}", err);
    assert!(display.contains("400"));
    assert!(display.contains("Bad Request"));
}

#[test]
fn test_context_ext_result() {
    let result: std::result::Result<(), Error> = Err(Error::network("test"));
    let with_context = ContextExt::context(result, "Operation failed");
    assert!(with_context.is_err());
    let err = with_context.unwrap_err();
    assert!(err.to_string().contains("Operation failed"));
}

#[test]
fn test_context_ext_option() {
    let opt: Option<i32> = None;
    let result = opt.context("Value not found");
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("Value not found"));
}

#[test]
fn test_from_serde_json_error() {
    let json_err = serde_json::from_str::<serde_json::Value>("invalid json").unwrap_err();
    let err: Error = json_err.into();
    assert!(matches!(err, Error::Parse(_)));
}

#[test]
fn test_from_network_error() {
    let network_err = NetworkError::Timeout;
    let err: Error = network_err.into();
    assert!(matches!(err, Error::Network(_)));
}

#[test]
fn test_from_order_error() {
    let order_err = OrderError::CreationFailed("test".to_string());
    let err: Error = order_err.into();
    assert!(matches!(err, Error::Order(_)));
}

#[test]
fn test_truncate_message() {
    let short = "short message".to_string();
    assert_eq!(truncate_message(short.clone()), short);

    let long = "x".repeat(2000);
    let truncated = truncate_message(long);
    assert!(truncated.len() < 2000);
    assert!(truncated.ends_with("... (truncated)"));
}

// Static assertion tests for Send + Sync
#[test]
fn error_is_send_sync_static() {
    fn assert_traits<T: Send + Sync + 'static + StdError>() {}
    assert_traits::<Error>();
    assert_traits::<NetworkError>();
    assert_traits::<ParseError>();
    assert_traits::<OrderError>();
}

#[test]
fn error_size_is_reasonable() {
    let size = std::mem::size_of::<Error>();
    // Target: Error size ≤ 56 bytes on 64-bit systems
    assert!(
        size <= 56,
        "Error enum size {} exceeds 56 bytes, consider boxing large variants",
        size
    );
}

// ==================== Property-Based Tests ====================

#[cfg(test)]
mod property_tests {
    use super::*;
    use proptest::prelude::*;
    use std::thread;

    // **Feature: error-handling-refactoring, Property 1: Error Send + Sync Guarantee**
    // **Validates: Requirements 1.4, 9.2**
    //
    // This module verifies that all Error variants implement Send + Sync + 'static,
    // ensuring thread-safe error propagation across async boundaries.

    /// Strategy to generate arbitrary error codes (numeric, alphanumeric, special chars)
    fn arb_error_code() -> impl Strategy<Value = String> {
        prop_oneof![
            // Numeric codes
            (100u32..600).prop_map(|n| n.to_string()),
            // Alphanumeric codes
            "[A-Z_]{3,20}",
            // Mixed codes
            "[A-Za-z0-9_-]{1,30}",
        ]
    }

    /// Strategy to generate arbitrary error messages
    fn arb_error_message() -> impl Strategy<Value = String> {
        prop_oneof![
            Just("".to_string()),
            "[a-zA-Z0-9 .,!?-]{1,100}",
            // Unicode messages
            "\\PC{1,50}",
        ]
    }

    /// Strategy to generate arbitrary Duration values
    fn arb_duration() -> impl Strategy<Value = Duration> {
        (0u64..=u64::MAX / 2).prop_map(Duration::from_nanos)
    }

    /// Strategy to generate arbitrary optional Duration values
    fn arb_optional_duration() -> impl Strategy<Value = Option<Duration>> {
        prop_oneof![Just(None), arb_duration().prop_map(Some),]
    }

    /// Strategy to generate arbitrary Error variants
    fn arb_error() -> impl Strategy<Value = Error> {
        prop_oneof![
            // Exchange errors
            (arb_error_code(), arb_error_message())
                .prop_map(|(code, msg)| Error::exchange(code, msg)),
            // Authentication errors
            arb_error_message().prop_map(|msg| Error::authentication(msg)),
            // Rate limit errors
            (arb_error_message(), arb_optional_duration())
                .prop_map(|(msg, retry)| Error::rate_limit(msg, retry)),
            // Invalid request errors
            arb_error_message().prop_map(|msg| Error::invalid_request(msg)),
            // Market not found errors
            arb_error_message().prop_map(|msg| Error::market_not_found(msg)),
            // Timeout errors
            arb_error_message().prop_map(|msg| Error::timeout(msg)),
            // Not implemented errors
            arb_error_message().prop_map(|msg| Error::not_implemented(msg)),
            // Network errors
            arb_error_message().prop_map(|msg| Error::network(msg)),
            // WebSocket errors
            arb_error_message().prop_map(|msg| Error::websocket(msg)),
            // Insufficient balance errors
            arb_error_message().prop_map(|msg| Error::insufficient_balance(msg)),
        ]
    }

    /// Strategy to generate NetworkError variants
    fn arb_network_error() -> impl Strategy<Value = NetworkError> {
        prop_oneof![
            // Use prop_map with unit to avoid Clone requirement
            Just(()).prop_map(|_| NetworkError::Timeout),
            arb_error_message().prop_map(NetworkError::ConnectionFailed),
            arb_error_message().prop_map(NetworkError::DnsResolution),
            arb_error_message().prop_map(NetworkError::Ssl),
            (100u16..600, arb_error_message()).prop_map(|(status, msg)| {
                NetworkError::RequestFailed {
                    status,
                    message: msg,
                }
            }),
        ]
    }

    /// Strategy to generate ParseError variants
    fn arb_parse_error() -> impl Strategy<Value = ParseError> {
        prop_oneof![
            arb_error_message().prop_map(|msg| ParseError::MissingField(Cow::Owned(msg))),
            arb_error_message().prop_map(|msg| ParseError::Timestamp(Cow::Owned(msg))),
            (arb_error_message(), arb_error_message()).prop_map(|(field, msg)| {
                ParseError::InvalidValue {
                    field: Cow::Owned(field),
                    message: Cow::Owned(msg),
                }
            }),
            (arb_error_message(), arb_error_message()).prop_map(|(field, msg)| {
                ParseError::InvalidFormat {
                    field: Cow::Owned(field),
                    message: Cow::Owned(msg),
                }
            }),
        ]
    }

    /// Strategy to generate OrderError variants
    fn arb_order_error() -> impl Strategy<Value = OrderError> {
        prop_oneof![
            arb_error_message().prop_map(OrderError::CreationFailed),
            arb_error_message().prop_map(OrderError::CancellationFailed),
            arb_error_message().prop_map(OrderError::ModificationFailed),
            arb_error_message().prop_map(OrderError::InvalidParameters),
        ]
    }

    // ==================== Property 1: Error Send + Sync Guarantee ====================

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// **Feature: error-handling-refactoring, Property 1: Error Send + Sync Guarantee**
        ///
        /// *For any* Error instance, the type must implement both `Send` and `Sync` traits,
        /// ensuring thread-safe error propagation across async boundaries.
        ///
        /// **Validates: Requirements 1.4, 9.2**
        #[test]
        fn prop_error_is_send_sync(error in arb_error()) {
            // Compile-time assertion: Error must be Send + Sync + 'static
            fn assert_send_sync_static<T: Send + Sync + 'static>(_: &T) {}
            assert_send_sync_static(&error);

            // Runtime verification: Error can be sent across threads
            let error_string = error.to_string();
            let handle = thread::spawn(move || {
                // Error was successfully moved to another thread (Send)
                error.to_string()
            });
            let result = handle.join().expect("Thread should not panic");
            prop_assert_eq!(result, error_string);
        }

        /// **Feature: error-handling-refactoring, Property 12: Error Thread Safety with 'static Bound**
        ///
        /// *For any* Error instance, the type must implement `Send + Sync + 'static` and
        /// `std::error::Error`, ensuring it can be used with `anyhow` and across async task boundaries.
        ///
        /// **Validates: Requirements 1.4, 9.2**
        #[test]
        fn prop_error_thread_safety_with_static_bound(error in arb_error()) {
            // Compile-time assertion: Error must implement std::error::Error
            fn assert_std_error<T: StdError + Send + Sync + 'static>(_: &T) {}
            assert_std_error(&error);

            // Verify error can be boxed as dyn Error (required for anyhow compatibility)
            let boxed: Box<dyn StdError + Send + Sync + 'static> = Box::new(error);

            // Verify the boxed error can be sent across threads
            let handle = thread::spawn(move || {
                boxed.to_string()
            });
            let _ = handle.join().expect("Thread should not panic");
        }

        /// Property test for NetworkError Send + Sync guarantee
        #[test]
        fn prop_network_error_is_send_sync(error in arb_network_error()) {
            fn assert_send_sync_static<T: Send + Sync + 'static + StdError>(_: &T) {}
            assert_send_sync_static(&error);

            let error_string = error.to_string();
            let handle = thread::spawn(move || error.to_string());
            let result = handle.join().expect("Thread should not panic");
            prop_assert_eq!(result, error_string);
        }

        /// Property test for ParseError Send + Sync guarantee
        #[test]
        fn prop_parse_error_is_send_sync(error in arb_parse_error()) {
            fn assert_send_sync_static<T: Send + Sync + 'static + StdError>(_: &T) {}
            assert_send_sync_static(&error);

            let error_string = error.to_string();
            let handle = thread::spawn(move || error.to_string());
            let result = handle.join().expect("Thread should not panic");
            prop_assert_eq!(result, error_string);
        }

        /// Property test for OrderError Send + Sync guarantee
        #[test]
        fn prop_order_error_is_send_sync(error in arb_order_error()) {
            fn assert_send_sync_static<T: Send + Sync + 'static + StdError>(_: &T) {}
            assert_send_sync_static(&error);

            let error_string = error.to_string();
            let handle = thread::spawn(move || error.to_string());
            let result = handle.join().expect("Thread should not panic");
            prop_assert_eq!(result, error_string);
        }

        /// Property test for Error with Context layers - Send + Sync guarantee
        #[test]
        fn prop_error_with_context_is_send_sync(
            base_error in arb_error(),
            context1 in arb_error_message(),
            context2 in arb_error_message()
        ) {
            let error_with_context = base_error
                .context(context1)
                .context(context2);

            fn assert_send_sync_static<T: Send + Sync + 'static + StdError>(_: &T) {}
            assert_send_sync_static(&error_with_context);

            let error_string = error_with_context.to_string();
            let handle = thread::spawn(move || error_with_context.to_string());
            let result = handle.join().expect("Thread should not panic");
            prop_assert_eq!(result, error_string);
        }

        /// Property test for Error converted from NetworkError - Send + Sync guarantee
        #[test]
        fn prop_error_from_network_error_is_send_sync(network_error in arb_network_error()) {
            let error: Error = network_error.into();

            fn assert_send_sync_static<T: Send + Sync + 'static + StdError>(_: &T) {}
            assert_send_sync_static(&error);

            let error_string = error.to_string();
            let handle = thread::spawn(move || error.to_string());
            let result = handle.join().expect("Thread should not panic");
            prop_assert_eq!(result, error_string);
        }

        /// Property test for Error converted from ParseError - Send + Sync guarantee
        #[test]
        fn prop_error_from_parse_error_is_send_sync(parse_error in arb_parse_error()) {
            let error: Error = parse_error.into();

            fn assert_send_sync_static<T: Send + Sync + 'static + StdError>(_: &T) {}
            assert_send_sync_static(&error);

            let error_string = error.to_string();
            let handle = thread::spawn(move || error.to_string());
            let result = handle.join().expect("Thread should not panic");
            prop_assert_eq!(result, error_string);
        }

        /// Property test for Error converted from OrderError - Send + Sync guarantee
        #[test]
        fn prop_error_from_order_error_is_send_sync(order_error in arb_order_error()) {
            let error: Error = order_error.into();

            fn assert_send_sync_static<T: Send + Sync + 'static + StdError>(_: &T) {}
            assert_send_sync_static(&error);

            let error_string = error.to_string();
            let handle = thread::spawn(move || error.to_string());
            let result = handle.join().expect("Thread should not panic");
            prop_assert_eq!(result, error_string);
        }
    }

    // ==================== Static Compile-Time Assertions ====================

    /// Static assertions that verify traits at compile time.
    /// These tests will fail to compile if the traits are not implemented.
    #[test]
    fn static_assert_error_traits() {
        // Compile-time assertions using const fn
        const fn assert_send<T: Send>() {}
        const fn assert_sync<T: Sync>() {}
        const fn assert_static<T: 'static>() {}
        const fn assert_std_error<T: StdError>() {}

        // Error type
        assert_send::<Error>();
        assert_sync::<Error>();
        assert_static::<Error>();
        assert_std_error::<Error>();

        // NetworkError type
        assert_send::<NetworkError>();
        assert_sync::<NetworkError>();
        assert_static::<NetworkError>();
        assert_std_error::<NetworkError>();

        // ParseError type
        assert_send::<ParseError>();
        assert_sync::<ParseError>();
        assert_static::<ParseError>();
        assert_std_error::<ParseError>();

        // OrderError type
        assert_send::<OrderError>();
        assert_sync::<OrderError>();
        assert_static::<OrderError>();
        assert_std_error::<OrderError>();

        // ExchangeErrorDetails type (not StdError, but should be Send + Sync)
        assert_send::<ExchangeErrorDetails>();
        assert_sync::<ExchangeErrorDetails>();
        assert_static::<ExchangeErrorDetails>();
    }

    /// Verify that Error can be used with anyhow (requires Send + Sync + 'static + StdError)
    #[test]
    fn static_assert_anyhow_compatibility() {
        fn can_convert_to_anyhow<E: StdError + Send + Sync + 'static>(_: E) -> anyhow::Error {
            anyhow::Error::msg("test")
        }

        // These should compile, proving anyhow compatibility
        let _ = can_convert_to_anyhow(Error::authentication("test"));
        let _ = can_convert_to_anyhow(NetworkError::Timeout);
        let _ = can_convert_to_anyhow(ParseError::missing_field("test"));
        let _ = can_convert_to_anyhow(OrderError::CreationFailed("test".to_string()));
    }

    /// Verify that Error can be used in async contexts (requires Send + 'static)
    #[test]
    fn static_assert_async_compatibility() {
        fn can_be_spawned<F: std::future::Future + Send + 'static>(_: F) {}

        // This should compile, proving async compatibility
        async fn returns_error() -> std::result::Result<(), Error> {
            Err(Error::authentication("test"))
        }

        can_be_spawned(returns_error());
    }

    // ==================== Property 5: HTTP Status Code Preservation ====================

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// **Feature: error-handling-refactoring, Property 5: HTTP Status Code Preservation**
        ///
        /// *For any* valid HTTP status code (100-599), creating a `NetworkError::RequestFailed`
        /// must preserve the exact status code value.
        ///
        /// **Validates: Requirements 3.3**
        #[test]
        fn prop_http_status_code_preservation(status in 100u16..600u16, message in arb_error_message()) {
            // Create NetworkError::RequestFailed with the given status code
            let network_error = NetworkError::RequestFailed {
                status,
                message: message.clone(),
            };

            // Verify the status code is preserved
            if let NetworkError::RequestFailed { status: preserved_status, message: preserved_message } = &network_error {
                prop_assert_eq!(
                    *preserved_status, status,
                    "HTTP status code {} was not preserved, got {}",
                    status, preserved_status
                );
                prop_assert_eq!(
                    preserved_message, &message,
                    "Error message was not preserved"
                );
            } else {
                prop_assert!(false, "Expected RequestFailed variant");
            }

            // Verify the status code appears in the Display output
            let display = network_error.to_string();
            prop_assert!(
                display.contains(&status.to_string()),
                "Status code {} not found in display output: {}",
                status, display
            );
        }

        /// **Feature: error-handling-refactoring, Property 5: HTTP Status Code Preservation**
        ///
        /// *For any* valid HTTP status code (100-599), when wrapped in Error::Network,
        /// the status code must still be accessible and preserved.
        ///
        /// **Validates: Requirements 3.3**
        #[test]
        fn prop_http_status_code_preservation_through_error(status in 100u16..600u16, message in arb_error_message()) {
            // Create NetworkError::RequestFailed and wrap in Error
            let network_error = NetworkError::RequestFailed {
                status,
                message: message.clone(),
            };
            let error: Error = network_error.into();

            // Verify the error is a Network variant
            if let Error::Network(boxed_network_error) = &error {
                if let NetworkError::RequestFailed { status: preserved_status, .. } = boxed_network_error.as_ref() {
                    prop_assert_eq!(
                        *preserved_status, status,
                        "HTTP status code {} was not preserved through Error wrapper, got {}",
                        status, preserved_status
                    );
                } else {
                    prop_assert!(false, "Expected RequestFailed variant inside Network");
                }
            } else {
                prop_assert!(false, "Expected Network variant");
            }

            // Verify the status code appears in the Error's Display output
            let display = error.to_string();
            prop_assert!(
                display.contains(&status.to_string()),
                "Status code {} not found in Error display output: {}",
                status, display
            );
        }

        /// **Feature: error-handling-refactoring, Property 5: HTTP Status Code Preservation**
        ///
        /// *For any* valid HTTP status code (100-599), when wrapped in Error::Network
        /// and then wrapped with context, the status code must still be preserved
        /// and accessible via root_cause().
        ///
        /// **Validates: Requirements 3.3**
        #[test]
        fn prop_http_status_code_preservation_through_context(
            status in 100u16..600u16,
            message in arb_error_message(),
            context in "[a-zA-Z0-9 ]{1,50}"
        ) {
            // Create NetworkError::RequestFailed, wrap in Error, then add context
            let network_error = NetworkError::RequestFailed {
                status,
                message: message.clone(),
            };
            let error: Error = network_error.into();
            let error_with_context = error.context(context);

            // Verify the status code is preserved in root_cause
            let root = error_with_context.root_cause();
            if let Error::Network(boxed_network_error) = root {
                if let NetworkError::RequestFailed { status: preserved_status, .. } = boxed_network_error.as_ref() {
                    prop_assert_eq!(
                        *preserved_status, status,
                        "HTTP status code {} was not preserved through context, got {}",
                        status, preserved_status
                    );
                } else {
                    prop_assert!(false, "Expected RequestFailed variant in root_cause");
                }
            } else {
                prop_assert!(false, "Expected Network variant in root_cause");
            }

            // Verify the status code appears in the report
            let report = error_with_context.report();
            prop_assert!(
                report.contains(&status.to_string()),
                "Status code {} not found in error report: {}",
                status, report
            );
        }
    }

    // ==================== Property 7: Error Context Chain Preservation ====================

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// **Feature: error-handling-refactoring, Property 7: Error Context Chain Preservation**
        ///
        /// *For any* base error and sequence of context strings, wrapping the error with
        /// `context()` calls must preserve the entire chain such that:
        /// - `source()` returns the previous error in the chain
        /// - `root_cause()` returns the original base error
        /// - `report()` contains all context strings and the base error message
        ///
        /// **Validates: Requirements 4.1, 4.2, 4.3, 4.4**
        #[test]
        fn prop_error_context_chain_preservation_single_context(
            base_error in arb_error(),
            context_str in "[a-zA-Z0-9 .,!?-]{1,100}"
        ) {
            // Capture the base error's string representation before wrapping
            let base_error_string = base_error.to_string();

            // Wrap with a single context
            let wrapped = base_error.context(context_str.clone());

            // 1. Verify source() returns the previous error in the chain
            let source = wrapped.source();
            prop_assert!(
                source.is_some(),
                "source() should return Some for Context variant"
            );
            let source_string = source.unwrap().to_string();
            prop_assert!(
                source_string.contains(&base_error_string) || base_error_string.contains(&source_string) || source_string == base_error_string,
                "source() should return the base error. Expected to contain '{}', got '{}'",
                base_error_string, source_string
            );

            // 2. Verify root_cause() returns the original base error
            let root = wrapped.root_cause();
            let root_string = root.to_string();
            prop_assert!(
                root_string == base_error_string || root_string.contains(&base_error_string) || base_error_string.contains(&root_string),
                "root_cause() should return the original base error. Expected '{}', got '{}'",
                base_error_string, root_string
            );

            // 3. Verify report() contains the context string
            let report = wrapped.report();
            prop_assert!(
                report.contains(&context_str),
                "report() should contain context string '{}'. Got: {}",
                context_str, report
            );

            // 4. Verify report() contains the base error message
            prop_assert!(
                report.contains(&base_error_string),
                "report() should contain base error message '{}'. Got: {}",
                base_error_string, report
            );

            // 5. Verify the wrapped error's Display shows the context
            let wrapped_display = wrapped.to_string();
            prop_assert!(
                wrapped_display.contains(&context_str),
                "Wrapped error Display should contain context '{}'. Got: {}",
                context_str, wrapped_display
            );
        }

        /// **Feature: error-handling-refactoring, Property 7: Error Context Chain Preservation**
        ///
        /// *For any* base error and multiple context strings, wrapping the error with
        /// multiple `context()` calls must preserve the entire chain.
        ///
        /// **Validates: Requirements 4.1, 4.2, 4.3, 4.4**
        #[test]
        fn prop_error_context_chain_preservation_multiple_contexts(
            base_error in arb_error(),
            context1 in "[a-zA-Z0-9]{5,20}",
            context2 in "[a-zA-Z0-9]{5,20}",
            context3 in "[a-zA-Z0-9]{5,20}"
        ) {
            // Capture the base error's string representation before wrapping
            let base_error_string = base_error.to_string();

            // Wrap with multiple contexts
            let wrapped = base_error
                .context(context1.clone())
                .context(context2.clone())
                .context(context3.clone());

            // 1. Verify source() chain exists
            // The outermost context's source should be the middle context
            let source1 = wrapped.source();
            prop_assert!(source1.is_some(), "First source() should return Some");

            let source2 = source1.unwrap().source();
            prop_assert!(source2.is_some(), "Second source() should return Some");

            let source3 = source2.unwrap().source();
            prop_assert!(source3.is_some(), "Third source() should return Some");

            // 2. Verify root_cause() returns the original base error
            let root = wrapped.root_cause();
            let root_string = root.to_string();
            prop_assert!(
                root_string == base_error_string || root_string.contains(&base_error_string) || base_error_string.contains(&root_string),
                "root_cause() should return the original base error. Expected '{}', got '{}'",
                base_error_string, root_string
            );

            // 3. Verify report() contains all context strings
            let report = wrapped.report();
            prop_assert!(
                report.contains(&context1),
                "report() should contain context1 '{}'. Got: {}",
                context1, report
            );
            prop_assert!(
                report.contains(&context2),
                "report() should contain context2 '{}'. Got: {}",
                context2, report
            );
            prop_assert!(
                report.contains(&context3),
                "report() should contain context3 '{}'. Got: {}",
                context3, report
            );

            // 4. Verify report() contains the base error message
            prop_assert!(
                report.contains(&base_error_string),
                "report() should contain base error message '{}'. Got: {}",
                base_error_string, report
            );

            // 5. Verify the outermost context is shown in Display
            let wrapped_display = wrapped.to_string();
            prop_assert!(
                wrapped_display.contains(&context3),
                "Wrapped error Display should contain outermost context '{}'. Got: {}",
                context3, wrapped_display
            );
        }

        /// **Feature: error-handling-refactoring, Property 7: Error Context Chain Preservation**
        ///
        /// *For any* base error and variable number of context strings (1-10),
        /// the context chain depth should match the number of context() calls.
        ///
        /// **Validates: Requirements 4.1, 4.2, 4.3, 4.4**
        #[test]
        fn prop_error_context_chain_depth(
            base_error in arb_error(),
            contexts in proptest::collection::vec("[a-zA-Z0-9]{3,15}", 1..=10)
        ) {
            // Capture the base error's string representation before wrapping
            let base_error_string = base_error.to_string();
            let num_contexts = contexts.len();

            // Wrap with all contexts
            let mut wrapped = base_error;
            for ctx in &contexts {
                wrapped = wrapped.context(ctx.clone());
            }

            // 1. Verify the chain depth by counting source() calls
            let mut depth = 0;
            let mut current: Option<&(dyn StdError + 'static)> = wrapped.source();
            while let Some(err) = current {
                depth += 1;
                current = err.source();
            }
            // The depth should be at least num_contexts (base error may or may not have source)
            prop_assert!(
                depth >= num_contexts,
                "Chain depth {} should be at least {} (number of contexts)",
                depth, num_contexts
            );

            // 2. Verify root_cause() returns the original base error
            let root = wrapped.root_cause();
            let root_string = root.to_string();
            prop_assert!(
                root_string == base_error_string || root_string.contains(&base_error_string) || base_error_string.contains(&root_string),
                "root_cause() should return the original base error. Expected '{}', got '{}'",
                base_error_string, root_string
            );

            // 3. Verify report() contains all context strings
            let report = wrapped.report();
            for ctx in &contexts {
                prop_assert!(
                    report.contains(ctx),
                    "report() should contain context '{}'. Got: {}",
                    ctx, report
                );
            }

            // 4. Verify report() contains the base error message
            prop_assert!(
                report.contains(&base_error_string),
                "report() should contain base error message '{}'. Got: {}",
                base_error_string, report
            );
        }

        /// **Feature: error-handling-refactoring, Property 7: Error Context Chain Preservation**
        ///
        /// *For any* error wrapped with context, the Context variant should preserve
        /// the source error exactly (not just its string representation).
        ///
        /// **Validates: Requirements 4.4**
        #[test]
        fn prop_error_context_preserves_source_error_variant(
            context_str in "[a-zA-Z0-9 ]{1,50}"
        ) {
            // Test with specific error variants to verify variant preservation

            // Test with RateLimit variant
            let rate_limit_err = Error::rate_limit("test rate limit", Some(Duration::from_secs(30)));
            let wrapped_rate_limit = rate_limit_err.context(context_str.clone());

            // The wrapped error should still be identifiable as a rate limit error
            prop_assert!(
                wrapped_rate_limit.as_rate_limit().is_some(),
                "as_rate_limit() should work through context"
            );
            prop_assert!(
                wrapped_rate_limit.is_retryable(),
                "is_retryable() should work through context for RateLimit"
            );
            prop_assert_eq!(
                wrapped_rate_limit.retry_after(),
                Some(Duration::from_secs(30)),
                "retry_after() should work through context"
            );

            // Test with Authentication variant
            let auth_err = Error::authentication("test auth error");
            let wrapped_auth = auth_err.context(context_str.clone());

            prop_assert!(
                wrapped_auth.as_authentication().is_some(),
                "as_authentication() should work through context"
            );

            // Test with Timeout variant
            let timeout_err = Error::timeout("test timeout");
            let wrapped_timeout = timeout_err.context(context_str.clone());

            prop_assert!(
                wrapped_timeout.is_retryable(),
                "is_retryable() should work through context for Timeout"
            );

            // Test with Network variant
            let network_err = Error::from(NetworkError::Timeout);
            let wrapped_network = network_err.context(context_str);

            prop_assert!(
                wrapped_network.is_retryable(),
                "is_retryable() should work through context for NetworkError::Timeout"
            );
        }
    }

    // ==================== Property 14: Enum Size Constraint ====================

    /// **Feature: error-handling-refactoring, Property 14: Enum Size Constraint**
    ///
    /// *For any* build configuration on 64-bit systems, `std::mem::size_of::<Error>()`
    /// must be less than or equal to 56 bytes to ensure efficient stack allocation
    /// and cache performance.
    ///
    /// **Validates: Performance optimization**
    ///
    /// Note: This is a static property that doesn't vary with input, but we verify
    /// it through property testing to ensure the constraint holds regardless of
    /// how Error variants are constructed.
    #[test]
    fn static_assert_error_size_constraint() {
        // The Error enum size is a compile-time constant
        const ERROR_SIZE: usize = std::mem::size_of::<Error>();
        const MAX_ALLOWED_SIZE: usize = 56;

        // Static assertion at compile time (will fail compilation if violated)
        const _: () = assert!(
            ERROR_SIZE <= MAX_ALLOWED_SIZE,
            // Note: const panic messages don't support formatting
        );

        // Runtime assertion with detailed message for debugging
        assert!(
            ERROR_SIZE <= MAX_ALLOWED_SIZE,
            "Error enum size {} bytes exceeds maximum allowed {} bytes. \
             Consider boxing large variants to reduce enum size.",
            ERROR_SIZE,
            MAX_ALLOWED_SIZE
        );

        // Also verify sub-error types are reasonably sized
        let network_error_size = std::mem::size_of::<NetworkError>();
        let parse_error_size = std::mem::size_of::<ParseError>();
        let order_error_size = std::mem::size_of::<OrderError>();

        // These are boxed in Error, so their size doesn't directly affect Error size,
        // but we still want them to be reasonable
        assert!(
            network_error_size <= 80,
            "NetworkError size {} bytes is unexpectedly large",
            network_error_size
        );
        assert!(
            parse_error_size <= 80,
            "ParseError size {} bytes is unexpectedly large",
            parse_error_size
        );
        assert!(
            order_error_size <= 48,
            "OrderError size {} bytes is unexpectedly large",
            order_error_size
        );
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// **Feature: error-handling-refactoring, Property 14: Enum Size Constraint**
        ///
        /// *For any* Error instance constructed with arbitrary data, the enum size
        /// must remain within the 56-byte constraint. This verifies that boxing
        /// strategy is effective regardless of the data stored in variants.
        ///
        /// **Validates: Performance optimization**
        #[test]
        fn prop_error_size_constraint_with_arbitrary_data(error in arb_error()) {
            // The size of the Error enum is constant regardless of the data inside
            // (because large data is boxed), but we verify this property holds
            // for all constructed variants
            const MAX_ALLOWED_SIZE: usize = 56;
            let error_size = std::mem::size_of_val(&error);

            // size_of_val returns the size of the enum itself, not the heap data
            // This should always equal size_of::<Error>()
            prop_assert!(
                error_size <= MAX_ALLOWED_SIZE,
                "Error size {} bytes exceeds {} bytes for variant: {:?}",
                error_size,
                MAX_ALLOWED_SIZE,
                std::mem::discriminant(&error)
            );

            // Verify the size is consistent with the type's size
            prop_assert_eq!(
                error_size,
                std::mem::size_of::<Error>(),
                "size_of_val should equal size_of::<Error>()"
            );
        }

        /// Property test verifying that wrapping errors in Context doesn't change enum size
        #[test]
        fn prop_error_size_with_context_layers(
            base_error in arb_error(),
            context1 in "[a-zA-Z0-9 ]{1,50}",
            context2 in "[a-zA-Z0-9 ]{1,50}",
            context3 in "[a-zA-Z0-9 ]{1,50}"
        ) {
            const MAX_ALLOWED_SIZE: usize = 56;

            // Wrap with multiple context layers
            let wrapped = base_error
                .context(context1)
                .context(context2)
                .context(context3);

            let wrapped_size = std::mem::size_of_val(&wrapped);

            // Size should remain constant regardless of context depth
            // (because Context variant boxes the source error)
            prop_assert!(
                wrapped_size <= MAX_ALLOWED_SIZE,
                "Error with context size {} bytes exceeds {} bytes",
                wrapped_size,
                MAX_ALLOWED_SIZE
            );

            prop_assert_eq!(
                wrapped_size,
                std::mem::size_of::<Error>(),
                "Wrapped error size should equal base Error size"
            );
        }

        /// Property test verifying NetworkError size is reasonable
        #[test]
        fn prop_network_error_size_constraint(error in arb_network_error()) {
            const MAX_ALLOWED_SIZE: usize = 80;
            let error_size = std::mem::size_of_val(&error);

            prop_assert!(
                error_size <= MAX_ALLOWED_SIZE,
                "NetworkError size {} bytes exceeds {} bytes",
                error_size,
                MAX_ALLOWED_SIZE
            );
        }

        /// Property test verifying ParseError size is reasonable
        #[test]
        fn prop_parse_error_size_constraint(error in arb_parse_error()) {
            const MAX_ALLOWED_SIZE: usize = 80;
            let error_size = std::mem::size_of_val(&error);

            prop_assert!(
                error_size <= MAX_ALLOWED_SIZE,
                "ParseError size {} bytes exceeds {} bytes",
                error_size,
                MAX_ALLOWED_SIZE
            );
        }

        /// Property test verifying OrderError size is reasonable
        #[test]
        fn prop_order_error_size_constraint(error in arb_order_error()) {
            const MAX_ALLOWED_SIZE: usize = 48;
            let error_size = std::mem::size_of_val(&error);

            prop_assert!(
                error_size <= MAX_ALLOWED_SIZE,
                "OrderError size {} bytes exceeds {} bytes",
                error_size,
                MAX_ALLOWED_SIZE
            );
        }
    }

    // ==================== Property 9: Error Display Non-Empty ====================

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// **Feature: error-handling-refactoring, Property 9: Error Display Non-Empty**
        ///
        /// *For any* Error variant with valid construction parameters, calling `to_string()`
        /// must return a non-empty string.
        ///
        /// **Validates: Requirements 8.1**
        #[test]
        fn prop_error_display_non_empty(error in arb_error()) {
            let display = error.to_string();
            prop_assert!(
                !display.is_empty(),
                "Error::to_string() returned empty string for error: {:?}",
                error
            );
            prop_assert!(
                display.trim().len() > 0,
                "Error::to_string() returned whitespace-only string for error: {:?}",
                error
            );
        }

        /// **Feature: error-handling-refactoring, Property 9: Error Display Non-Empty**
        ///
        /// *For any* NetworkError variant, calling `to_string()` must return a non-empty string.
        ///
        /// **Validates: Requirements 8.1**
        #[test]
        fn prop_network_error_display_non_empty(error in arb_network_error()) {
            let display = error.to_string();
            prop_assert!(
                !display.is_empty(),
                "NetworkError::to_string() returned empty string for error: {:?}",
                error
            );
        }

        /// **Feature: error-handling-refactoring, Property 9: Error Display Non-Empty**
        ///
        /// *For any* ParseError variant, calling `to_string()` must return a non-empty string.
        ///
        /// **Validates: Requirements 8.1**
        #[test]
        fn prop_parse_error_display_non_empty(error in arb_parse_error()) {
            let display = error.to_string();
            prop_assert!(
                !display.is_empty(),
                "ParseError::to_string() returned empty string for error: {:?}",
                error
            );
        }

        /// **Feature: error-handling-refactoring, Property 9: Error Display Non-Empty**
        ///
        /// *For any* OrderError variant, calling `to_string()` must return a non-empty string.
        ///
        /// **Validates: Requirements 8.1**
        #[test]
        fn prop_order_error_display_non_empty(error in arb_order_error()) {
            let display = error.to_string();
            prop_assert!(
                !display.is_empty(),
                "OrderError::to_string() returned empty string for error: {:?}",
                error
            );
        }

        /// **Feature: error-handling-refactoring, Property 9: Error Display Non-Empty**
        ///
        /// *For any* Error wrapped with context, calling `to_string()` must return a non-empty string.
        ///
        /// **Validates: Requirements 8.1**
        #[test]
        fn prop_error_with_context_display_non_empty(
            base_error in arb_error(),
            context in "[a-zA-Z0-9 ]{1,50}"
        ) {
            let wrapped = base_error.context(context);
            let display = wrapped.to_string();
            prop_assert!(
                !display.is_empty(),
                "Error with context::to_string() returned empty string for error: {:?}",
                wrapped
            );
        }
    }

    // ==================== Property 11: Retryable Error Classification ====================

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// **Feature: error-handling-refactoring, Property 11: Retryable Error Classification**
        ///
        /// *For any* error, the `is_retryable()` method must return `true` if and only if
        /// the error is one of: `NetworkError::Timeout`, `NetworkError::ConnectionFailed`,
        /// `RateLimit`, or `Timeout`. This must also work through Context layers.
        ///
        /// **Validates: Requirements 3.2 (implicit retry behavior)**
        #[test]
        fn prop_retryable_error_classification_rate_limit(
            message in arb_error_message(),
            retry_after in arb_optional_duration()
        ) {
            let error = Error::rate_limit(message, retry_after);
            prop_assert!(
                error.is_retryable(),
                "RateLimit error should be retryable"
            );
        }

        /// **Feature: error-handling-refactoring, Property 11: Retryable Error Classification**
        ///
        /// Timeout errors must be classified as retryable.
        ///
        /// **Validates: Requirements 3.2**
        #[test]
        fn prop_retryable_error_classification_timeout(message in arb_error_message()) {
            let error = Error::timeout(message);
            prop_assert!(
                error.is_retryable(),
                "Timeout error should be retryable"
            );
        }

        /// **Feature: error-handling-refactoring, Property 11: Retryable Error Classification**
        ///
        /// NetworkError::Timeout must be classified as retryable.
        ///
        /// **Validates: Requirements 3.2**
        #[test]
        fn prop_retryable_error_classification_network_timeout(_dummy in Just(())) {
            let error: Error = NetworkError::Timeout.into();
            prop_assert!(
                error.is_retryable(),
                "NetworkError::Timeout should be retryable"
            );
        }

        /// **Feature: error-handling-refactoring, Property 11: Retryable Error Classification**
        ///
        /// NetworkError::ConnectionFailed must be classified as retryable.
        ///
        /// **Validates: Requirements 3.2**
        #[test]
        fn prop_retryable_error_classification_connection_failed(message in arb_error_message()) {
            let error: Error = NetworkError::ConnectionFailed(message).into();
            prop_assert!(
                error.is_retryable(),
                "NetworkError::ConnectionFailed should be retryable"
            );
        }

        /// **Feature: error-handling-refactoring, Property 11: Retryable Error Classification**
        ///
        /// Authentication errors must NOT be classified as retryable.
        ///
        /// **Validates: Requirements 3.2**
        #[test]
        fn prop_non_retryable_error_classification_authentication(message in arb_error_message()) {
            let error = Error::authentication(message);
            prop_assert!(
                !error.is_retryable(),
                "Authentication error should NOT be retryable"
            );
        }

        /// **Feature: error-handling-refactoring, Property 11: Retryable Error Classification**
        ///
        /// InvalidRequest errors must NOT be classified as retryable.
        ///
        /// **Validates: Requirements 3.2**
        #[test]
        fn prop_non_retryable_error_classification_invalid_request(message in arb_error_message()) {
            let error = Error::invalid_request(message);
            prop_assert!(
                !error.is_retryable(),
                "InvalidRequest error should NOT be retryable"
            );
        }

        /// **Feature: error-handling-refactoring, Property 11: Retryable Error Classification**
        ///
        /// MarketNotFound errors must NOT be classified as retryable.
        ///
        /// **Validates: Requirements 3.2**
        #[test]
        fn prop_non_retryable_error_classification_market_not_found(message in arb_error_message()) {
            let error = Error::market_not_found(message);
            prop_assert!(
                !error.is_retryable(),
                "MarketNotFound error should NOT be retryable"
            );
        }

        /// **Feature: error-handling-refactoring, Property 11: Retryable Error Classification**
        ///
        /// Exchange errors must NOT be classified as retryable.
        ///
        /// **Validates: Requirements 3.2**
        #[test]
        fn prop_non_retryable_error_classification_exchange(
            code in arb_error_code(),
            message in arb_error_message()
        ) {
            let error = Error::exchange(code, message);
            prop_assert!(
                !error.is_retryable(),
                "Exchange error should NOT be retryable"
            );
        }

        /// **Feature: error-handling-refactoring, Property 11: Retryable Error Classification**
        ///
        /// Retryable errors wrapped in Context layers must still be classified as retryable.
        ///
        /// **Validates: Requirements 3.2**
        #[test]
        fn prop_retryable_error_through_context(
            message in arb_error_message(),
            retry_after in arb_optional_duration(),
            context1 in "[a-zA-Z0-9 ]{1,30}",
            context2 in "[a-zA-Z0-9 ]{1,30}"
        ) {
            let error = Error::rate_limit(message, retry_after)
                .context(context1)
                .context(context2);
            prop_assert!(
                error.is_retryable(),
                "RateLimit error wrapped in context should still be retryable"
            );
        }

        /// **Feature: error-handling-refactoring, Property 11: Retryable Error Classification**
        ///
        /// Non-retryable errors wrapped in Context layers must still NOT be classified as retryable.
        ///
        /// **Validates: Requirements 3.2**
        #[test]
        fn prop_non_retryable_error_through_context(
            message in arb_error_message(),
            context1 in "[a-zA-Z0-9 ]{1,30}",
            context2 in "[a-zA-Z0-9 ]{1,30}"
        ) {
            let error = Error::authentication(message)
                .context(context1)
                .context(context2);
            prop_assert!(
                !error.is_retryable(),
                "Authentication error wrapped in context should still NOT be retryable"
            );
        }

        /// **Feature: error-handling-refactoring, Property 11: Retryable Error Classification**
        ///
        /// NetworkError::RequestFailed must NOT be classified as retryable (it's a definitive failure).
        ///
        /// **Validates: Requirements 3.2**
        #[test]
        fn prop_non_retryable_error_classification_request_failed(
            status in 100u16..600u16,
            message in arb_error_message()
        ) {
            let error: Error = NetworkError::RequestFailed { status, message }.into();
            prop_assert!(
                !error.is_retryable(),
                "NetworkError::RequestFailed should NOT be retryable"
            );
        }

        /// **Feature: error-handling-refactoring, Property 11: Retryable Error Classification**
        ///
        /// NetworkError::DnsResolution must NOT be classified as retryable.
        ///
        /// **Validates: Requirements 3.2**
        #[test]
        fn prop_non_retryable_error_classification_dns_resolution(message in arb_error_message()) {
            let error: Error = NetworkError::DnsResolution(message).into();
            prop_assert!(
                !error.is_retryable(),
                "NetworkError::DnsResolution should NOT be retryable"
            );
        }

        /// **Feature: error-handling-refactoring, Property 11: Retryable Error Classification**
        ///
        /// NetworkError::Ssl must NOT be classified as retryable.
        ///
        /// **Validates: Requirements 3.2**
        #[test]
        fn prop_non_retryable_error_classification_ssl(message in arb_error_message()) {
            let error: Error = NetworkError::Ssl(message).into();
            prop_assert!(
                !error.is_retryable(),
                "NetworkError::Ssl should NOT be retryable"
            );
        }
    }

    // ==================== Unit Tests for From Implementations ====================

    /// Unit tests for From implementations to verify error information is preserved.
    /// **Validates: Requirements 9.4**
    #[test]
    fn test_from_network_error_preserves_info() {
        // Test Timeout
        let network_err = NetworkError::Timeout;
        let error: Error = network_err.into();
        assert!(matches!(error, Error::Network(_)));
        assert!(error.to_string().contains("timeout") || error.to_string().contains("Timeout"));

        // Test ConnectionFailed
        let network_err = NetworkError::ConnectionFailed("Connection refused".to_string());
        let error: Error = network_err.into();
        assert!(matches!(error, Error::Network(_)));
        assert!(error.to_string().contains("Connection refused"));

        // Test RequestFailed
        let network_err = NetworkError::RequestFailed {
            status: 404,
            message: "Not Found".to_string(),
        };
        let error: Error = network_err.into();
        assert!(matches!(error, Error::Network(_)));
        assert!(error.to_string().contains("404"));
        assert!(error.to_string().contains("Not Found"));

        // Test DnsResolution
        let network_err = NetworkError::DnsResolution("DNS lookup failed".to_string());
        let error: Error = network_err.into();
        assert!(matches!(error, Error::Network(_)));
        assert!(error.to_string().contains("DNS"));

        // Test Ssl
        let network_err = NetworkError::Ssl("Certificate expired".to_string());
        let error: Error = network_err.into();
        assert!(matches!(error, Error::Network(_)));
        assert!(error.to_string().contains("Certificate expired"));
    }

    #[test]
    fn test_from_parse_error_preserves_info() {
        // Test MissingField
        let parse_err = ParseError::missing_field("price");
        let error: Error = parse_err.into();
        assert!(matches!(error, Error::Parse(_)));
        assert!(error.to_string().contains("price"));

        // Test InvalidValue
        let parse_err = ParseError::invalid_value("amount", "must be positive");
        let error: Error = parse_err.into();
        assert!(matches!(error, Error::Parse(_)));
        assert!(error.to_string().contains("amount"));
        assert!(error.to_string().contains("must be positive"));

        // Test Timestamp
        let parse_err = ParseError::timestamp("invalid timestamp format");
        let error: Error = parse_err.into();
        assert!(matches!(error, Error::Parse(_)));
        assert!(error.to_string().contains("timestamp"));

        // Test InvalidFormat
        let parse_err = ParseError::invalid_format("date", "expected ISO 8601");
        let error: Error = parse_err.into();
        assert!(matches!(error, Error::Parse(_)));
        assert!(error.to_string().contains("date"));
        assert!(error.to_string().contains("ISO 8601"));
    }

    #[test]
    fn test_from_order_error_preserves_info() {
        // Test CreationFailed
        let order_err = OrderError::CreationFailed("Insufficient margin".to_string());
        let error: Error = order_err.into();
        assert!(matches!(error, Error::Order(_)));
        assert!(error.to_string().contains("Insufficient margin"));

        // Test CancellationFailed
        let order_err = OrderError::CancellationFailed("Order already filled".to_string());
        let error: Error = order_err.into();
        assert!(matches!(error, Error::Order(_)));
        assert!(error.to_string().contains("Order already filled"));

        // Test ModificationFailed
        let order_err = OrderError::ModificationFailed("Cannot modify filled order".to_string());
        let error: Error = order_err.into();
        assert!(matches!(error, Error::Order(_)));
        assert!(error.to_string().contains("Cannot modify"));

        // Test InvalidParameters
        let order_err = OrderError::InvalidParameters("Invalid quantity".to_string());
        let error: Error = order_err.into();
        assert!(matches!(error, Error::Order(_)));
        assert!(error.to_string().contains("Invalid quantity"));
    }

    #[test]
    fn test_from_serde_json_error_preserves_info() {
        let json_err = serde_json::from_str::<serde_json::Value>("{ invalid json }").unwrap_err();
        let error: Error = json_err.into();
        assert!(matches!(error, Error::Parse(_)));
        // The error message should contain some indication of JSON parsing failure
        let display = error.to_string();
        assert!(
            display.contains("JSON") || display.contains("json") || display.contains("parse"),
            "Expected JSON-related error message, got: {}",
            display
        );
    }

    #[test]
    fn test_from_rust_decimal_error_preserves_info() {
        use rust_decimal::Decimal;
        use std::str::FromStr;

        let decimal_err = Decimal::from_str("not_a_number").unwrap_err();
        let error: Error = decimal_err.into();
        assert!(matches!(error, Error::Parse(_)));
        // The error message should contain some indication of decimal parsing failure
        let display = error.to_string();
        assert!(
            display.contains("decimal") || display.contains("Decimal") || display.contains("parse"),
            "Expected decimal-related error message, got: {}",
            display
        );
    }

    #[test]
    fn test_from_boxed_network_error_preserves_info() {
        let network_err = Box::new(NetworkError::Timeout);
        let error: Error = network_err.into();
        assert!(matches!(error, Error::Network(_)));
        assert!(error.to_string().contains("timeout") || error.to_string().contains("Timeout"));
    }

    #[test]
    fn test_from_boxed_parse_error_preserves_info() {
        let parse_err = Box::new(ParseError::missing_field("symbol"));
        let error: Error = parse_err.into();
        assert!(matches!(error, Error::Parse(_)));
        assert!(error.to_string().contains("symbol"));
    }

    #[test]
    fn test_from_boxed_order_error_preserves_info() {
        let order_err = Box::new(OrderError::CreationFailed("Test failure".to_string()));
        let error: Error = order_err.into();
        assert!(matches!(error, Error::Order(_)));
        assert!(error.to_string().contains("Test failure"));
    }

    // ==================== Unit Tests for From<reqwest::Error> ====================
    // **Validates: Requirements 9.4**
    //
    // Note: reqwest::Error cannot be easily constructed directly in tests,
    // so we test the conversion logic by creating actual network errors
    // using tokio runtime.

    /// Test that reqwest timeout errors are converted to NetworkError::Timeout
    #[tokio::test]
    async fn test_from_reqwest_timeout_error() {
        // Create a client with a very short timeout
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_nanos(1))
            .build()
            .expect("Failed to build client");

        // Try to make a request that will timeout
        let result = client.get("https://httpbin.org/delay/10").send().await;

        if let Err(reqwest_err) = result {
            // Convert to NetworkError
            let network_err: NetworkError = reqwest_err.into();

            // Timeout errors should be converted to NetworkError::Timeout
            // Note: The actual error type depends on whether the timeout happens
            // during connection or during the request
            let is_timeout_or_connection = matches!(
                network_err,
                NetworkError::Timeout
                    | NetworkError::ConnectionFailed(_)
                    | NetworkError::Transport(_)
            );
            assert!(
                is_timeout_or_connection,
                "Expected Timeout, ConnectionFailed, or Transport variant, got: {:?}",
                network_err
            );
        }
        // If the request somehow succeeds (unlikely with 1ns timeout), that's also fine
    }

    /// Test that reqwest connection errors are converted to NetworkError::ConnectionFailed
    #[tokio::test]
    async fn test_from_reqwest_connection_error() {
        let client = reqwest::Client::new();

        // Try to connect to a non-existent server
        let result = client.get("http://127.0.0.1:1").send().await;

        if let Err(reqwest_err) = result {
            // Verify it's a connection error
            assert!(
                reqwest_err.is_connect(),
                "Expected connection error, got: {:?}",
                reqwest_err
            );

            // Convert to NetworkError
            let network_err: NetworkError = reqwest_err.into();

            // Connection errors should be converted to NetworkError::ConnectionFailed
            assert!(
                matches!(network_err, NetworkError::ConnectionFailed(_)),
                "Expected ConnectionFailed variant, got: {:?}",
                network_err
            );

            // Convert to Error and verify the chain
            let error: Error = network_err.into();
            assert!(matches!(error, Error::Network(_)));
        }
    }

    /// Test that the From<reqwest::Error> for Error conversion works correctly
    #[tokio::test]
    async fn test_from_reqwest_error_to_error() {
        let client = reqwest::Client::new();

        // Try to connect to a non-existent server
        let result = client.get("http://127.0.0.1:1").send().await;

        if let Err(reqwest_err) = result {
            // Convert directly to Error (not via NetworkError)
            let error: Error = reqwest_err.into();

            // Should be wrapped in Error::Network
            assert!(
                matches!(error, Error::Network(_)),
                "Expected Network variant, got: {:?}",
                error
            );

            // The error message should contain some connection-related info
            let display = error.to_string();
            assert!(!display.is_empty(), "Error display should not be empty");
        }
    }

    /// Test that reqwest errors preserve information through the conversion chain
    #[tokio::test]
    async fn test_reqwest_error_preserves_info_through_chain() {
        let client = reqwest::Client::new();

        // Try to connect to a non-existent server
        let result = client.get("http://127.0.0.1:1").send().await;

        if let Err(reqwest_err) = result {
            let _original_message = reqwest_err.to_string();

            // Convert to Error
            let error: Error = reqwest_err.into();

            // The error should be retryable (connection failures are retryable)
            assert!(
                error.is_retryable(),
                "Connection failed errors should be retryable"
            );

            // Add context and verify chain is preserved
            let error_with_context = error.context("Failed to fetch data");
            assert!(
                error_with_context.is_retryable(),
                "Retryable status should be preserved through context"
            );

            // Verify the report contains the original error info
            let report = error_with_context.report();
            assert!(
                report.contains("Failed to fetch data"),
                "Report should contain context"
            );
            // The original error info should be somewhere in the chain
            assert!(
                report.contains("Network")
                    || report.contains("Connection")
                    || report.contains("connect"),
                "Report should contain network error info, got: {}",
                report
            );
        }
    }

    /// Test truncate_message function for long error messages
    #[test]
    fn test_truncate_message_preserves_short_messages() {
        let short = "Short message".to_string();
        let result = truncate_message(short.clone());
        assert_eq!(result, short);
    }

    #[test]
    fn test_truncate_message_truncates_long_messages() {
        let long = "x".repeat(2000);
        let result = truncate_message(long);
        assert!(result.len() < 2000);
        assert!(result.len() <= MAX_ERROR_MESSAGE_LEN + 20); // Allow for "... (truncated)" suffix
        assert!(result.ends_with("... (truncated)"));
    }

    #[test]
    fn test_truncate_message_boundary() {
        // Test exactly at the boundary
        let exact = "x".repeat(MAX_ERROR_MESSAGE_LEN);
        let result = truncate_message(exact.clone());
        assert_eq!(result, exact); // Should not be truncated

        // Test one over the boundary
        let over = "x".repeat(MAX_ERROR_MESSAGE_LEN + 1);
        let result = truncate_message(over);
        assert!(result.ends_with("... (truncated)"));
    }

    // ==================== Comprehensive From Implementation Tests ====================
    // **Validates: Requirements 9.4**

    /// Test that all NetworkError variants convert correctly to Error
    #[test]
    fn test_all_network_error_variants_convert_to_error() {
        // Timeout
        let err: Error = NetworkError::Timeout.into();
        assert!(matches!(err, Error::Network(_)));
        assert!(err.to_string().to_lowercase().contains("timeout"));

        // ConnectionFailed
        let err: Error = NetworkError::ConnectionFailed("refused".to_string()).into();
        assert!(matches!(err, Error::Network(_)));
        assert!(err.to_string().contains("refused"));

        // DnsResolution
        let err: Error = NetworkError::DnsResolution("lookup failed".to_string()).into();
        assert!(matches!(err, Error::Network(_)));
        assert!(err.to_string().contains("lookup failed"));

        // Ssl
        let err: Error = NetworkError::Ssl("cert error".to_string()).into();
        assert!(matches!(err, Error::Network(_)));
        assert!(err.to_string().contains("cert error"));

        // RequestFailed
        let err: Error = NetworkError::RequestFailed {
            status: 500,
            message: "Internal Server Error".to_string(),
        }
        .into();
        assert!(matches!(err, Error::Network(_)));
        assert!(err.to_string().contains("500"));
        assert!(err.to_string().contains("Internal Server Error"));

        // Transport (with a simple error)
        let simple_err = std::io::Error::new(std::io::ErrorKind::Other, "transport error");
        let err: Error = NetworkError::Transport(Box::new(simple_err)).into();
        assert!(matches!(err, Error::Network(_)));
    }

    /// Test that all ParseError variants convert correctly to Error
    #[test]
    fn test_all_parse_error_variants_convert_to_error() {
        // MissingField (static)
        let err: Error = ParseError::missing_field("price").into();
        assert!(matches!(err, Error::Parse(_)));
        assert!(err.to_string().contains("price"));

        // MissingField (owned)
        let err: Error = ParseError::missing_field_owned("dynamic_field".to_string()).into();
        assert!(matches!(err, Error::Parse(_)));
        assert!(err.to_string().contains("dynamic_field"));

        // InvalidValue
        let err: Error = ParseError::invalid_value("amount", "negative value").into();
        assert!(matches!(err, Error::Parse(_)));
        assert!(err.to_string().contains("amount"));
        assert!(err.to_string().contains("negative value"));

        // Timestamp (static)
        let err: Error = ParseError::timestamp("invalid format").into();
        assert!(matches!(err, Error::Parse(_)));
        assert!(err.to_string().contains("timestamp"));

        // Timestamp (owned)
        let err: Error = ParseError::timestamp_owned("dynamic timestamp error".to_string()).into();
        assert!(matches!(err, Error::Parse(_)));
        assert!(err.to_string().contains("dynamic timestamp error"));

        // InvalidFormat
        let err: Error = ParseError::invalid_format("date", "expected YYYY-MM-DD").into();
        assert!(matches!(err, Error::Parse(_)));
        assert!(err.to_string().contains("date"));
        assert!(err.to_string().contains("YYYY-MM-DD"));

        // Json (via serde_json::Error)
        let json_err = serde_json::from_str::<serde_json::Value>("invalid").unwrap_err();
        let err: Error = ParseError::Json(json_err).into();
        assert!(matches!(err, Error::Parse(_)));

        // Decimal (via rust_decimal::Error)
        use rust_decimal::Decimal;
        use std::str::FromStr;
        let decimal_err = Decimal::from_str("not_a_number").unwrap_err();
        let err: Error = ParseError::Decimal(decimal_err).into();
        assert!(matches!(err, Error::Parse(_)));
    }

    /// Test that all OrderError variants convert correctly to Error
    #[test]
    fn test_all_order_error_variants_convert_to_error() {
        // CreationFailed
        let err: Error = OrderError::CreationFailed("insufficient funds".to_string()).into();
        assert!(matches!(err, Error::Order(_)));
        assert!(err.to_string().contains("insufficient funds"));

        // CancellationFailed
        let err: Error = OrderError::CancellationFailed("order not found".to_string()).into();
        assert!(matches!(err, Error::Order(_)));
        assert!(err.to_string().contains("order not found"));

        // ModificationFailed
        let err: Error = OrderError::ModificationFailed("order already filled".to_string()).into();
        assert!(matches!(err, Error::Order(_)));
        assert!(err.to_string().contains("order already filled"));

        // InvalidParameters
        let err: Error = OrderError::InvalidParameters("invalid quantity".to_string()).into();
        assert!(matches!(err, Error::Order(_)));
        assert!(err.to_string().contains("invalid quantity"));
    }

    /// Test that From implementations work with the ? operator
    #[test]
    fn test_from_implementations_with_question_mark() {
        fn parse_json() -> Result<serde_json::Value> {
            let value: serde_json::Value = serde_json::from_str("invalid")?;
            Ok(value)
        }

        let result = parse_json();
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, Error::Parse(_)));
    }

    /// Test that From implementations preserve error source chain
    #[test]
    fn test_from_implementations_preserve_source() {
        // serde_json::Error -> ParseError -> Error
        let json_err = serde_json::from_str::<serde_json::Value>("invalid").unwrap_err();
        let parse_err = ParseError::Json(json_err);
        let error: Error = parse_err.into();

        // The Error should have a source
        if let Error::Parse(boxed_parse) = &error {
            if let ParseError::Json(inner) = boxed_parse.as_ref() {
                // The inner serde_json::Error should be accessible
                assert!(!inner.to_string().is_empty());
            } else {
                panic!("Expected ParseError::Json variant");
            }
        } else {
            panic!("Expected Error::Parse variant");
        }

        // rust_decimal::Error -> ParseError -> Error
        use rust_decimal::Decimal;
        use std::str::FromStr;
        let decimal_err = Decimal::from_str("not_a_number").unwrap_err();
        let parse_err = ParseError::Decimal(decimal_err);
        let error: Error = parse_err.into();

        if let Error::Parse(boxed_parse) = &error {
            if let ParseError::Decimal(inner) = boxed_parse.as_ref() {
                assert!(!inner.to_string().is_empty());
            } else {
                panic!("Expected ParseError::Decimal variant");
            }
        } else {
            panic!("Expected Error::Parse variant");
        }
    }
}
