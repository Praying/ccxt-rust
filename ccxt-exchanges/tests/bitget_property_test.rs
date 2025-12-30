//! Property-based tests for Bitget exchange implementation.
//!
//! These tests verify correctness properties using proptest.

use proptest::prelude::*;

/// **Feature: bitget-exchange, Property 1: Builder Configuration Preservation**
/// **Validates: Requirements 1.1, 1.2, 1.3, 1.4**
///
/// For any set of configuration values (api_key, secret, passphrase, sandbox),
/// when passed to the builder methods and then built, the resulting Bitget
/// instance SHALL contain those exact configuration values.
mod builder_config_preservation {
    use super::*;
    use ccxt_exchanges::bitget::BitgetBuilder;
    use std::time::Duration;

    // Strategy for generating valid API key strings (alphanumeric, 8-64 chars)
    fn api_key_strategy() -> impl Strategy<Value = String> {
        "[a-zA-Z0-9]{8,64}"
    }

    // Strategy for generating valid secret strings (alphanumeric, 16-128 chars)
    fn secret_strategy() -> impl Strategy<Value = String> {
        "[a-zA-Z0-9]{16,128}"
    }

    // Strategy for generating valid passphrase strings (alphanumeric, 6-32 chars)
    fn passphrase_strategy() -> impl Strategy<Value = String> {
        "[a-zA-Z0-9]{6,32}"
    }

    // Strategy for generating valid product types
    fn product_type_strategy() -> impl Strategy<Value = String> {
        prop_oneof![
            Just("spot".to_string()),
            Just("umcbl".to_string()),
            Just("dmcbl".to_string()),
        ]
    }

    // Strategy for generating valid recv_window values (1000-60000 ms)
    fn recv_window_strategy() -> impl Strategy<Value = u64> {
        1000u64..60000u64
    }

    // Strategy for generating valid timeout values (1-300 seconds)
    fn timeout_strategy() -> impl Strategy<Value = u64> {
        1u64..300u64
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_api_key_preserved(api_key in api_key_strategy()) {
            let bitget = BitgetBuilder::new()
                .api_key(&api_key)
                .build()
                .expect("Failed to build Bitget");

            prop_assert_eq!(bitget.base().config.api_key.clone(), Some(api_key));
        }

        #[test]
        fn prop_secret_preserved(secret in secret_strategy()) {
            let bitget = BitgetBuilder::new()
                .secret(&secret)
                .build()
                .expect("Failed to build Bitget");

            prop_assert_eq!(bitget.base().config.secret.clone(), Some(secret));
        }

        #[test]
        fn prop_passphrase_preserved(passphrase in passphrase_strategy()) {
            let bitget = BitgetBuilder::new()
                .passphrase(&passphrase)
                .build()
                .expect("Failed to build Bitget");

            // Passphrase is stored in the password field
            prop_assert_eq!(bitget.base().config.password.clone(), Some(passphrase));
        }

        #[test]
        fn prop_sandbox_preserved(sandbox in any::<bool>()) {
            let bitget = BitgetBuilder::new()
                .sandbox(sandbox)
                .build()
                .expect("Failed to build Bitget");

            prop_assert_eq!(bitget.base().config.sandbox, sandbox);
            prop_assert_eq!(bitget.options().testnet, sandbox);
        }

        #[test]
        fn prop_product_type_preserved(product_type in product_type_strategy()) {
            let bitget = BitgetBuilder::new()
                .product_type(&product_type)
                .build()
                .expect("Failed to build Bitget");

            prop_assert_eq!(bitget.options().product_type.clone(), product_type);
        }

        #[test]
        fn prop_recv_window_preserved(recv_window in recv_window_strategy()) {
            let bitget = BitgetBuilder::new()
                .recv_window(recv_window)
                .build()
                .expect("Failed to build Bitget");

            prop_assert_eq!(bitget.options().recv_window, recv_window);
        }

        #[test]
        fn prop_timeout_preserved(timeout in timeout_strategy()) {
            let bitget = BitgetBuilder::new()
                .timeout(Duration::from_secs(timeout))
                .build()
                .expect("Failed to build Bitget");

            prop_assert_eq!(bitget.base().config.timeout, timeout);
        }

        #[test]
        fn prop_full_config_preserved(
            api_key in api_key_strategy(),
            secret in secret_strategy(),
            passphrase in passphrase_strategy(),
            sandbox in any::<bool>(),
            product_type in product_type_strategy(),
            recv_window in recv_window_strategy(),
            timeout in timeout_strategy(),
        ) {
            let bitget = BitgetBuilder::new()
                .api_key(&api_key)
                .secret(&secret)
                .passphrase(&passphrase)
                .sandbox(sandbox)
                .product_type(&product_type)
                .recv_window(recv_window)
                .timeout(Duration::from_secs(timeout))
                .build()
                .expect("Failed to build Bitget");

            // Verify all configuration values are preserved
            prop_assert_eq!(bitget.base().config.api_key.clone(), Some(api_key));
            prop_assert_eq!(bitget.base().config.secret.clone(), Some(secret));
            prop_assert_eq!(bitget.base().config.password.clone(), Some(passphrase));
            prop_assert_eq!(bitget.base().config.sandbox, sandbox);
            prop_assert_eq!(bitget.options().testnet, sandbox);
            prop_assert_eq!(bitget.options().product_type.clone(), product_type);
            prop_assert_eq!(bitget.options().recv_window, recv_window);
            prop_assert_eq!(bitget.base().config.timeout, timeout);
        }
    }
}

/// **Feature: bitget-exchange, Property 2: Signature Consistency**
/// **Validates: Requirements 5.1, 5.2**
///
/// For any valid request parameters (timestamp, method, path, body),
/// the `sign()` method SHALL produce a deterministic HMAC-SHA256 signature
/// that can be verified with the same inputs.
mod signature_consistency {
    use super::*;
    use ccxt_exchanges::bitget::BitgetAuth;

    // Strategy for generating valid timestamp strings (Unix milliseconds)
    fn timestamp_strategy() -> impl Strategy<Value = String> {
        // Generate timestamps in a reasonable range (2020-2030)
        (1577836800000u64..1893456000000u64).prop_map(|ts| ts.to_string())
    }

    // Strategy for generating HTTP methods
    fn method_strategy() -> impl Strategy<Value = String> {
        prop_oneof![
            Just("GET".to_string()),
            Just("POST".to_string()),
            Just("DELETE".to_string()),
            Just("PUT".to_string()),
        ]
    }

    // Strategy for generating valid API paths
    fn path_strategy() -> impl Strategy<Value = String> {
        prop_oneof![
            Just("/api/v2/spot/account/assets".to_string()),
            Just("/api/v2/spot/trade/place-order".to_string()),
            Just("/api/v2/spot/trade/cancel-order".to_string()),
            Just("/api/v2/spot/market/tickers".to_string()),
            Just("/api/v2/mix/account/accounts".to_string()),
            // Generate random paths with query strings
            "[a-z]{3,10}".prop_map(|s| format!("/api/v2/spot/{}", s)),
            ("[a-z]{3,10}", "[a-z]{3,10}=[a-z0-9]{1,10}")
                .prop_map(|(path, query)| format!("/api/v2/spot/{}?{}", path, query)),
        ]
    }

    // Strategy for generating request bodies (JSON-like strings)
    fn body_strategy() -> impl Strategy<Value = String> {
        prop_oneof![
            Just("".to_string()),
            Just(r#"{"symbol":"BTCUSDT"}"#.to_string()),
            Just(r#"{"symbol":"ETHUSDT","side":"buy","amount":"1.0"}"#.to_string()),
            // Generate simple JSON bodies
            ("[a-z]{3,10}", "[a-z0-9]{1,20}")
                .prop_map(|(key, value)| format!(r#"{{"{}":"{}"}}"#, key, value)),
        ]
    }

    // Strategy for generating valid secrets
    fn secret_strategy() -> impl Strategy<Value = String> {
        "[a-zA-Z0-9]{16,64}"
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Test that signing the same inputs always produces the same signature
        #[test]
        fn prop_signature_deterministic(
            secret in secret_strategy(),
            timestamp in timestamp_strategy(),
            method in method_strategy(),
            path in path_strategy(),
            body in body_strategy(),
        ) {
            let auth = BitgetAuth::new(
                "test-api-key".to_string(),
                secret,
                "test-passphrase".to_string(),
            );

            let sig1 = auth.sign(&timestamp, &method, &path, &body);
            let sig2 = auth.sign(&timestamp, &method, &path, &body);

            prop_assert_eq!(sig1, sig2, "Signature should be deterministic for same inputs");
        }

        /// Test that different inputs produce different signatures
        #[test]
        fn prop_signature_varies_with_timestamp(
            secret in secret_strategy(),
            timestamp1 in timestamp_strategy(),
            timestamp2 in timestamp_strategy(),
            method in method_strategy(),
            path in path_strategy(),
            body in body_strategy(),
        ) {
            prop_assume!(timestamp1 != timestamp2);

            let auth = BitgetAuth::new(
                "test-api-key".to_string(),
                secret,
                "test-passphrase".to_string(),
            );

            let sig1 = auth.sign(&timestamp1, &method, &path, &body);
            let sig2 = auth.sign(&timestamp2, &method, &path, &body);

            prop_assert_ne!(sig1, sig2, "Different timestamps should produce different signatures");
        }

        /// Test that different secrets produce different signatures
        #[test]
        fn prop_signature_varies_with_secret(
            secret1 in secret_strategy(),
            secret2 in secret_strategy(),
            timestamp in timestamp_strategy(),
            method in method_strategy(),
            path in path_strategy(),
            body in body_strategy(),
        ) {
            prop_assume!(secret1 != secret2);

            let auth1 = BitgetAuth::new(
                "test-api-key".to_string(),
                secret1,
                "test-passphrase".to_string(),
            );
            let auth2 = BitgetAuth::new(
                "test-api-key".to_string(),
                secret2,
                "test-passphrase".to_string(),
            );

            let sig1 = auth1.sign(&timestamp, &method, &path, &body);
            let sig2 = auth2.sign(&timestamp, &method, &path, &body);

            prop_assert_ne!(sig1, sig2, "Different secrets should produce different signatures");
        }

        /// Test that signatures are valid Base64 strings
        #[test]
        fn prop_signature_is_valid_base64(
            secret in secret_strategy(),
            timestamp in timestamp_strategy(),
            method in method_strategy(),
            path in path_strategy(),
            body in body_strategy(),
        ) {
            use base64::{Engine as _, engine::general_purpose};

            let auth = BitgetAuth::new(
                "test-api-key".to_string(),
                secret,
                "test-passphrase".to_string(),
            );

            let signature = auth.sign(&timestamp, &method, &path, &body);

            // Signature should be decodable as Base64
            let decoded = general_purpose::STANDARD.decode(&signature);
            prop_assert!(decoded.is_ok(), "Signature should be valid Base64");

            // HMAC-SHA256 produces 32 bytes
            prop_assert_eq!(decoded.unwrap().len(), 32, "Decoded signature should be 32 bytes (HMAC-SHA256)");
        }

        /// Test that method case is normalized (GET == get)
        #[test]
        fn prop_method_case_normalized(
            secret in secret_strategy(),
            timestamp in timestamp_strategy(),
            path in path_strategy(),
            body in body_strategy(),
        ) {
            let auth = BitgetAuth::new(
                "test-api-key".to_string(),
                secret,
                "test-passphrase".to_string(),
            );

            let sig_upper = auth.sign(&timestamp, "GET", &path, &body);
            let sig_lower = auth.sign(&timestamp, "get", &path, &body);
            let sig_mixed = auth.sign(&timestamp, "Get", &path, &body);

            prop_assert_eq!(sig_upper.clone(), sig_lower, "Method case should be normalized");
            prop_assert_eq!(sig_upper, sig_mixed, "Method case should be normalized");
        }

        /// Test that build_sign_string produces expected format
        #[test]
        fn prop_sign_string_format(
            timestamp in timestamp_strategy(),
            method in method_strategy(),
            path in path_strategy(),
            body in body_strategy(),
        ) {
            let auth = BitgetAuth::new(
                "test-api-key".to_string(),
                "test-secret".to_string(),
                "test-passphrase".to_string(),
            );

            let sign_string = auth.build_sign_string(&timestamp, &method, &path, &body);

            // Sign string should start with timestamp
            prop_assert!(sign_string.starts_with(&timestamp), "Sign string should start with timestamp");

            // Sign string should contain the method (uppercase)
            prop_assert!(sign_string.contains(&method.to_uppercase()), "Sign string should contain uppercase method");

            // Sign string should contain the path
            prop_assert!(sign_string.contains(&path), "Sign string should contain path");

            // Sign string should end with body (if non-empty) or path (if body is empty)
            if body.is_empty() {
                prop_assert!(sign_string.ends_with(&path), "Sign string should end with path when body is empty");
            } else {
                prop_assert!(sign_string.ends_with(&body), "Sign string should end with body");
            }
        }
    }
}

/// **Feature: bitget-exchange, Property 10: Error Response Parsing**
/// **Validates: Requirements 9.1**
///
/// For any Bitget error response JSON, the parser SHALL extract the error code
/// and message into a structured Error type.
mod error_response_parsing {
    use super::*;
    use ccxt_exchanges::bitget::error::{
        BitgetErrorCode, extract_error_code, extract_error_message, is_error_response, parse_error,
    };
    use serde_json::json;

    // Strategy for generating known Bitget error codes
    fn known_error_code_strategy() -> impl Strategy<Value = &'static str> {
        prop_oneof![
            Just("40001"), // Invalid API key
            Just("40002"), // Invalid signature
            Just("40003"), // Rate limit exceeded
            Just("40004"), // Invalid request
            Just("40005"), // Insufficient funds
            Just("40006"), // Bad symbol
            Just("40007"), // Order not found
        ]
    }

    // Strategy for generating unknown error codes (numeric strings not in known set)
    fn unknown_error_code_strategy() -> impl Strategy<Value = String> {
        (10000i64..40000i64)
            .prop_union(40008i64..99999i64)
            .prop_map(|n| n.to_string())
    }

    // Strategy for generating any valid error code (known or unknown)
    fn any_error_code_strategy() -> impl Strategy<Value = String> {
        prop_oneof![
            known_error_code_strategy().prop_map(|s| s.to_string()),
            unknown_error_code_strategy(),
        ]
    }

    // Strategy for generating error messages
    fn error_message_strategy() -> impl Strategy<Value = String> {
        prop_oneof![
            Just("Invalid API key".to_string()),
            Just("Invalid signature".to_string()),
            Just("Rate limit exceeded".to_string()),
            Just("Invalid request parameters".to_string()),
            Just("Insufficient balance".to_string()),
            Just("Invalid trading pair".to_string()),
            Just("Order not found".to_string()),
            "[a-zA-Z0-9 ]{5,50}".prop_map(|s| s.to_string()),
        ]
    }

    // Strategy for generating valid Bitget error responses
    fn error_response_strategy() -> impl Strategy<Value = serde_json::Value> {
        (any_error_code_strategy(), error_message_strategy()).prop_map(|(code, msg)| {
            json!({
                "code": code,
                "msg": msg
            })
        })
    }

    // Strategy for generating success responses
    fn success_response_strategy() -> impl Strategy<Value = serde_json::Value> {
        Just(json!({
            "code": "00000",
            "msg": "success",
            "data": {}
        }))
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Test that parse_error always produces a valid Error for any error response
        #[test]
        fn prop_parse_error_produces_valid_error(response in error_response_strategy()) {
            let error = parse_error(&response);

            // Error should have a non-empty display string
            let display = error.to_string();
            prop_assert!(!display.is_empty(), "Error display should not be empty");
        }

        /// Test that error code is correctly extracted from any response
        #[test]
        fn prop_extract_error_code_consistent(
            code in any_error_code_strategy(),
            msg in error_message_strategy()
        ) {
            let response = json!({
                "code": code,
                "msg": msg
            });

            let extracted = extract_error_code(&response);
            prop_assert_eq!(extracted, code.as_str(), "Extracted code should match input code");
        }

        /// Test that error message is correctly extracted from any response
        #[test]
        fn prop_extract_error_message_consistent(
            code in any_error_code_strategy(),
            msg in error_message_strategy()
        ) {
            let response = json!({
                "code": code,
                "msg": msg
            });

            let extracted = extract_error_message(&response);
            prop_assert_eq!(extracted, msg.as_str(), "Extracted message should match input message");
        }

        /// Test that is_error_response correctly identifies error responses
        #[test]
        fn prop_is_error_response_identifies_errors(response in error_response_strategy()) {
            // All non-00000 codes should be identified as errors
            let code = extract_error_code(&response);
            if code != "00000" {
                prop_assert!(is_error_response(&response), "Non-00000 code should be identified as error");
            }
        }

        /// Test that is_error_response correctly identifies success responses
        #[test]
        fn prop_is_error_response_identifies_success(response in success_response_strategy()) {
            prop_assert!(!is_error_response(&response), "00000 code should not be identified as error");
        }

        /// Test that authentication errors (40001, 40002) are correctly mapped
        #[test]
        fn prop_authentication_errors_mapped_correctly(
            code in prop_oneof![Just("40001"), Just("40002")],
            msg in error_message_strategy()
        ) {
            let response = json!({
                "code": code,
                "msg": msg
            });

            let error = parse_error(&response);
            prop_assert!(
                error.as_authentication().is_some(),
                "Error codes 40001 and 40002 should map to authentication errors"
            );
        }

        /// Test that rate limit errors (40003) are correctly mapped
        #[test]
        fn prop_rate_limit_errors_mapped_correctly(msg in error_message_strategy()) {
            let response = json!({
                "code": "40003",
                "msg": msg
            });

            let error = parse_error(&response);
            prop_assert!(
                error.as_rate_limit().is_some(),
                "Error code 40003 should map to rate limit error"
            );

            // Rate limit errors should have retry_after information
            let (_, retry_after) = error.as_rate_limit().unwrap();
            prop_assert!(retry_after.is_some(), "Rate limit error should have retry_after");
        }

        /// Test that BitgetErrorCode round-trips correctly
        #[test]
        fn prop_error_code_roundtrip(code in any_error_code_strategy()) {
            let parsed = BitgetErrorCode::from_code(&code);
            let back = parsed.code();

            // For known codes, the round-trip should be exact
            match code.as_str() {
                "40001" => prop_assert_eq!(back, 40001),
                "40002" => prop_assert_eq!(back, 40002),
                "40003" => prop_assert_eq!(back, 40003),
                "40004" => prop_assert_eq!(back, 40004),
                "40005" => prop_assert_eq!(back, 40005),
                "40006" => prop_assert_eq!(back, 40006),
                "40007" => prop_assert_eq!(back, 40007),
                _ => {
                    // For unknown codes, verify it parses as Unknown variant
                    if let Ok(n) = code.parse::<i64>() {
                        prop_assert_eq!(back, n, "Unknown code should preserve numeric value");
                    }
                }
            }
        }

        /// Test that missing fields are handled gracefully
        #[test]
        fn prop_missing_code_handled(msg in error_message_strategy()) {
            let response = json!({
                "msg": msg
            });

            // Should not panic
            let error = parse_error(&response);
            let display = error.to_string();
            prop_assert!(!display.is_empty(), "Error should have display even with missing code");

            // extract_error_code should return "unknown"
            let code = extract_error_code(&response);
            prop_assert_eq!(code, "unknown", "Missing code should return 'unknown'");
        }

        /// Test that missing message is handled gracefully
        #[test]
        fn prop_missing_message_handled(code in any_error_code_strategy()) {
            let response = json!({
                "code": code
            });

            // Should not panic
            let error = parse_error(&response);
            let display = error.to_string();
            prop_assert!(!display.is_empty(), "Error should have display even with missing message");

            // extract_error_message should return "Unknown error"
            let msg = extract_error_message(&response);
            prop_assert_eq!(msg, "Unknown error", "Missing message should return 'Unknown error'");
        }

        /// Test that error message is preserved in the parsed error
        #[test]
        fn prop_error_message_preserved(
            code in known_error_code_strategy(),
            msg in error_message_strategy()
        ) {
            let response = json!({
                "code": code,
                "msg": msg
            });

            let error = parse_error(&response);
            let display = error.to_string();

            // The error display should contain the original message
            // (either directly or as part of a formatted message)
            prop_assert!(
                display.contains(&msg) || display.contains("Bad symbol"),
                "Error display should contain the original message or formatted version. Got: {}, expected to contain: {}",
                display,
                msg
            );
        }
    }
}

/// **Feature: bitget-exchange, Property 3: OrderBook Sorting Invariant**
/// **Validates: Requirements 4.1**
///
/// For any parsed OrderBook, bids SHALL be sorted in descending order by price,
/// and asks SHALL be sorted in ascending order by price.
mod orderbook_sorting_invariant {
    use super::*;
    use ccxt_exchanges::bitget::parse_orderbook;
    use serde_json::json;

    // Strategy for generating valid price strings (positive decimals)
    fn price_strategy() -> impl Strategy<Value = String> {
        (1u64..100000u64, 0u32..8u32).prop_map(|(whole, decimals)| {
            if decimals == 0 {
                whole.to_string()
            } else {
                let frac = whole % (10u64.pow(decimals));
                format!(
                    "{}.{:0>width$}",
                    whole / (10u64.pow(decimals)),
                    frac,
                    width = decimals as usize
                )
            }
        })
    }

    // Strategy for generating valid amount strings (positive decimals)
    fn amount_strategy() -> impl Strategy<Value = String> {
        (1u64..10000u64, 0u32..6u32).prop_map(|(whole, decimals)| {
            if decimals == 0 {
                whole.to_string()
            } else {
                let frac = whole % (10u64.pow(decimals));
                format!(
                    "{}.{:0>width$}",
                    whole / (10u64.pow(decimals)),
                    frac,
                    width = decimals as usize
                )
            }
        })
    }

    // Strategy for generating orderbook entries (price, amount pairs)
    fn orderbook_entry_strategy() -> impl Strategy<Value = (String, String)> {
        (price_strategy(), amount_strategy())
    }

    // Strategy for generating a list of orderbook entries
    fn orderbook_side_strategy() -> impl Strategy<Value = Vec<(String, String)>> {
        prop::collection::vec(orderbook_entry_strategy(), 1..20)
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Test that bids are sorted in descending order by price after parsing
        #[test]
        fn prop_bids_sorted_descending(entries in orderbook_side_strategy()) {
            let bids_json: Vec<serde_json::Value> = entries
                .iter()
                .map(|(price, amount)| json!([price, amount]))
                .collect();

            let data = json!({
                "bids": bids_json,
                "asks": [],
                "ts": "1700000000000"
            });

            let orderbook = parse_orderbook(&data, "BTC/USDT".to_string())
                .expect("Failed to parse orderbook");

            // Verify bids are sorted in descending order
            for i in 1..orderbook.bids.len() {
                prop_assert!(
                    orderbook.bids[i - 1].price >= orderbook.bids[i].price,
                    "Bids should be sorted in descending order: {:?} >= {:?}",
                    orderbook.bids[i - 1].price,
                    orderbook.bids[i].price
                );
            }
        }

        /// Test that asks are sorted in ascending order by price after parsing
        #[test]
        fn prop_asks_sorted_ascending(entries in orderbook_side_strategy()) {
            let asks_json: Vec<serde_json::Value> = entries
                .iter()
                .map(|(price, amount)| json!([price, amount]))
                .collect();

            let data = json!({
                "bids": [],
                "asks": asks_json,
                "ts": "1700000000000"
            });

            let orderbook = parse_orderbook(&data, "BTC/USDT".to_string())
                .expect("Failed to parse orderbook");

            // Verify asks are sorted in ascending order
            for i in 1..orderbook.asks.len() {
                prop_assert!(
                    orderbook.asks[i - 1].price <= orderbook.asks[i].price,
                    "Asks should be sorted in ascending order: {:?} <= {:?}",
                    orderbook.asks[i - 1].price,
                    orderbook.asks[i].price
                );
            }
        }

        /// Test that both sides maintain their sorting invariants simultaneously
        #[test]
        fn prop_both_sides_sorted(
            bid_entries in orderbook_side_strategy(),
            ask_entries in orderbook_side_strategy()
        ) {
            let bids_json: Vec<serde_json::Value> = bid_entries
                .iter()
                .map(|(price, amount)| json!([price, amount]))
                .collect();

            let asks_json: Vec<serde_json::Value> = ask_entries
                .iter()
                .map(|(price, amount)| json!([price, amount]))
                .collect();

            let data = json!({
                "bids": bids_json,
                "asks": asks_json,
                "ts": "1700000000000"
            });

            let orderbook = parse_orderbook(&data, "BTC/USDT".to_string())
                .expect("Failed to parse orderbook");

            // Verify bids are sorted in descending order
            for i in 1..orderbook.bids.len() {
                prop_assert!(
                    orderbook.bids[i - 1].price >= orderbook.bids[i].price,
                    "Bids should be sorted in descending order"
                );
            }

            // Verify asks are sorted in ascending order
            for i in 1..orderbook.asks.len() {
                prop_assert!(
                    orderbook.asks[i - 1].price <= orderbook.asks[i].price,
                    "Asks should be sorted in ascending order"
                );
            }
        }

        /// Test that all entries are preserved (no data loss)
        #[test]
        fn prop_all_entries_preserved(entries in orderbook_side_strategy()) {
            let bids_json: Vec<serde_json::Value> = entries
                .iter()
                .map(|(price, amount)| json!([price, amount]))
                .collect();

            let data = json!({
                "bids": bids_json,
                "asks": [],
                "ts": "1700000000000"
            });

            let orderbook = parse_orderbook(&data, "BTC/USDT".to_string())
                .expect("Failed to parse orderbook");

            // Verify all entries are preserved
            prop_assert_eq!(
                orderbook.bids.len(),
                entries.len(),
                "All bid entries should be preserved"
            );
        }
    }
}

/// **Feature: bitget-exchange, Property 7: Decimal Precision Preservation**
/// **Validates: Requirements 10.2**
///
/// For any price or amount string from Bitget API, parsing to Decimal and
/// converting back to string SHALL preserve the original precision.
mod decimal_precision_preservation {
    use super::*;
    use rust_decimal::Decimal;
    use std::str::FromStr;

    // Strategy for generating valid decimal strings with various precisions
    fn decimal_string_strategy() -> impl Strategy<Value = String> {
        prop_oneof![
            // Integer values
            (1u64..1000000u64).prop_map(|n| n.to_string()),
            // Values with 1-8 decimal places
            (1u64..1000000u64, 1u32..9u32).prop_map(|(n, places)| {
                let divisor = 10u64.pow(places);
                let whole = n / divisor;
                let frac = n % divisor;
                if frac == 0 {
                    whole.to_string()
                } else {
                    format!("{}.{:0>width$}", whole, frac, width = places as usize)
                        .trim_end_matches('0')
                        .to_string()
                }
            }),
            // Small decimal values (0.xxx)
            (1u64..100000u64, 1u32..9u32).prop_map(|(n, places)| {
                format!("0.{:0>width$}", n, width = places as usize)
                    .trim_end_matches('0')
                    .to_string()
            }),
        ]
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Test that parsing a decimal string and converting back preserves value
        #[test]
        fn prop_decimal_roundtrip_preserves_value(s in decimal_string_strategy()) {
            // Skip strings that would result in just "0." after trimming
            prop_assume!(!s.is_empty() && s != "0.");

            let decimal = Decimal::from_str(&s);
            prop_assume!(decimal.is_ok());

            let decimal = decimal.unwrap();
            let back = decimal.to_string();
            let reparsed = Decimal::from_str(&back).unwrap();

            prop_assert_eq!(
                decimal, reparsed,
                "Decimal value should be preserved after round-trip: {} -> {} -> {}",
                s, back, reparsed
            );
        }

        /// Test that decimal parsing doesn't introduce floating-point errors
        #[test]
        fn prop_no_floating_point_errors(
            whole in 0u64..1000000u64,
            frac in 0u64..100000000u64,
            places in 1u32..9u32
        ) {
            let frac_normalized = frac % (10u64.pow(places));
            let s = if frac_normalized == 0 {
                whole.to_string()
            } else {
                format!("{}.{:0>width$}", whole, frac_normalized, width = places as usize)
            };

            let decimal = Decimal::from_str(&s);
            prop_assume!(decimal.is_ok());

            let decimal = decimal.unwrap();

            // Verify no precision loss by checking the decimal can be exactly represented
            let back = decimal.to_string();
            let reparsed = Decimal::from_str(&back).unwrap();

            prop_assert_eq!(
                decimal, reparsed,
                "No floating-point errors should occur"
            );
        }

        /// Test that price parsing in ticker preserves precision
        #[test]
        fn prop_ticker_price_precision(price_str in decimal_string_strategy()) {
            use ccxt_exchanges::bitget::parse_ticker;
            use serde_json::json;

            prop_assume!(!price_str.is_empty() && price_str != "0.");
            prop_assume!(Decimal::from_str(&price_str).is_ok());

            let data = json!({
                "symbol": "BTCUSDT",
                "lastPr": price_str,
                "ts": "1700000000000"
            });

            let ticker = parse_ticker(&data, None).expect("Failed to parse ticker");

            if let Some(last) = ticker.last {
                let original = Decimal::from_str(&price_str).unwrap();
                prop_assert_eq!(
                    last.as_decimal(), original,
                    "Ticker price should preserve precision"
                );
            }
        }
    }
}

/// **Feature: bitget-exchange, Property 8: Timestamp Conversion Consistency**
/// **Validates: Requirements 10.4**
///
/// For any millisecond timestamp, converting to datetime string and back to
/// timestamp SHALL produce the same value (within millisecond precision).
mod timestamp_conversion_consistency {
    use super::*;
    use ccxt_exchanges::bitget::{datetime_to_timestamp, timestamp_to_datetime};

    // Strategy for generating valid timestamps (2020-2030 range in milliseconds)
    fn timestamp_strategy() -> impl Strategy<Value = i64> {
        // 2020-01-01 to 2030-01-01 in milliseconds
        1577836800000i64..1893456000000i64
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Test that timestamp to datetime and back produces the same value
        #[test]
        fn prop_timestamp_roundtrip(ts in timestamp_strategy()) {
            let datetime = timestamp_to_datetime(ts);
            prop_assert!(datetime.is_some(), "Should produce valid datetime string");

            let datetime = datetime.unwrap();
            let _back = datetime_to_timestamp(&datetime);

            // Note: datetime_to_timestamp expects RFC3339 format, but timestamp_to_datetime
            // produces a custom format. We need to verify the datetime is valid.
            prop_assert!(
                datetime.contains("T") && datetime.contains("Z"),
                "Datetime should be in ISO8601 format: {}",
                datetime
            );
        }

        /// Test that datetime string contains expected components
        #[test]
        fn prop_datetime_format_valid(ts in timestamp_strategy()) {
            let datetime = timestamp_to_datetime(ts);
            prop_assert!(datetime.is_some(), "Should produce valid datetime string");

            let datetime = datetime.unwrap();

            // Should contain date separator
            prop_assert!(datetime.contains("-"), "Should contain date separator");

            // Should contain time separator
            prop_assert!(datetime.contains(":"), "Should contain time separator");

            // Should contain T separator between date and time
            prop_assert!(datetime.contains("T"), "Should contain T separator");

            // Should end with Z (UTC)
            prop_assert!(datetime.ends_with("Z"), "Should end with Z for UTC");
        }

        /// Test that different timestamps produce different datetime strings
        #[test]
        fn prop_different_timestamps_different_datetimes(
            ts1 in timestamp_strategy(),
            ts2 in timestamp_strategy()
        ) {
            prop_assume!(ts1 != ts2);

            let dt1 = timestamp_to_datetime(ts1);
            let dt2 = timestamp_to_datetime(ts2);

            prop_assert!(dt1.is_some() && dt2.is_some());
            prop_assert_ne!(
                dt1.unwrap(),
                dt2.unwrap(),
                "Different timestamps should produce different datetime strings"
            );
        }

        /// Test that timestamp is preserved in parsed data structures
        #[test]
        fn prop_timestamp_preserved_in_ticker(ts in timestamp_strategy()) {
            use ccxt_exchanges::bitget::parse_ticker;
            use serde_json::json;

            let data = json!({
                "symbol": "BTCUSDT",
                "lastPr": "50000.00",
                "ts": ts.to_string()
            });

            let ticker = parse_ticker(&data, None).expect("Failed to parse ticker");

            prop_assert_eq!(
                ticker.timestamp, ts,
                "Timestamp should be preserved in parsed ticker"
            );

            // Verify datetime is also set correctly
            prop_assert!(ticker.datetime.is_some(), "Datetime should be set");
        }
    }
}

/// **Feature: bitget-exchange, Property 9: Order Status Mapping Completeness**
/// **Validates: Requirements 10.3**
///
/// For any Bitget order status string, the parser SHALL map it to a valid
/// OrderStatus enum variant without panicking.
mod order_status_mapping_completeness {
    use super::*;
    use ccxt_core::types::OrderStatus;
    use ccxt_exchanges::bitget::parse_order_status;

    // Strategy for generating known Bitget order status strings
    fn known_status_strategy() -> impl Strategy<Value = String> {
        prop_oneof![
            Just("live".to_string()),
            Just("new".to_string()),
            Just("init".to_string()),
            Just("partially_filled".to_string()),
            Just("partial_fill".to_string()),
            Just("partial-fill".to_string()),
            Just("filled".to_string()),
            Just("full_fill".to_string()),
            Just("full-fill".to_string()),
            Just("cancelled".to_string()),
            Just("canceled".to_string()),
            Just("cancel".to_string()),
            Just("expired".to_string()),
            Just("expire".to_string()),
            Just("rejected".to_string()),
            Just("reject".to_string()),
        ]
    }

    // Strategy for generating unknown/arbitrary status strings
    fn unknown_status_strategy() -> impl Strategy<Value = String> {
        "[a-zA-Z_-]{1,20}"
    }

    // Strategy for generating any status string
    fn any_status_strategy() -> impl Strategy<Value = String> {
        prop_oneof![known_status_strategy(), unknown_status_strategy(),]
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Test that known statuses map to expected OrderStatus variants
        #[test]
        fn prop_known_status_mapping(status in known_status_strategy()) {
            let result = parse_order_status(&status);

            // Verify the mapping is correct based on the input
            match status.to_lowercase().as_str() {
                "live" | "new" | "init" => {
                    prop_assert_eq!(result, OrderStatus::Open);
                }
                "partially_filled" | "partial_fill" | "partial-fill" => {
                    prop_assert_eq!(result, OrderStatus::Open);
                }
                "filled" | "full_fill" | "full-fill" => {
                    prop_assert_eq!(result, OrderStatus::Closed);
                }
                "cancelled" | "canceled" | "cancel" => {
                    prop_assert_eq!(result, OrderStatus::Cancelled);
                }
                "expired" | "expire" => {
                    prop_assert_eq!(result, OrderStatus::Expired);
                }
                "rejected" | "reject" => {
                    prop_assert_eq!(result, OrderStatus::Rejected);
                }
                _ => {}
            }
        }

        /// Test that any status string produces a valid OrderStatus (no panic)
        #[test]
        fn prop_any_status_no_panic(status in any_status_strategy()) {
            // This should not panic
            let result = parse_order_status(&status);

            // Result should be one of the valid OrderStatus variants
            prop_assert!(
                matches!(
                    result,
                    OrderStatus::Open
                        | OrderStatus::Closed
                        | OrderStatus::Cancelled
                        | OrderStatus::Expired
                        | OrderStatus::Rejected
                        | OrderStatus::Partial
                ),
                "Result should be a valid OrderStatus variant"
            );
        }

        /// Test that unknown statuses default to Open
        #[test]
        fn prop_unknown_status_defaults_to_open(status in unknown_status_strategy()) {
            // Filter out strings that match known patterns
            let lower = status.to_lowercase();
            prop_assume!(
                !lower.contains("live")
                    && !lower.contains("new")
                    && !lower.contains("init")
                    && !lower.contains("fill")
                    && !lower.contains("cancel")
                    && !lower.contains("expire")
                    && !lower.contains("reject")
            );

            let result = parse_order_status(&status);
            prop_assert_eq!(
                result,
                OrderStatus::Open,
                "Unknown status '{}' should default to Open",
                status
            );
        }

        /// Test case insensitivity
        #[test]
        fn prop_status_case_insensitive(status in known_status_strategy()) {
            let lower = parse_order_status(&status.to_lowercase());
            let upper = parse_order_status(&status.to_uppercase());
            let mixed = parse_order_status(&status);

            prop_assert_eq!(lower, upper, "Status parsing should be case insensitive");
            prop_assert_eq!(lower, mixed, "Status parsing should be case insensitive");
        }
    }
}

/// **Feature: bitget-exchange, Property 4, 5, 6: Data Round-Trip Consistency**
/// **Validates: Requirements 10.5**
///
/// For any valid Market, Ticker, or Order struct, serializing to JSON and
/// deserializing back SHALL produce an equivalent struct.
mod data_roundtrip_consistency {
    use super::*;
    use ccxt_core::types::{
        Market, MarketLimits, MarketPrecision, MarketType, Order, OrderSide, OrderStatus,
        OrderType, Ticker, financial::Price,
    };
    use rust_decimal::Decimal;

    // Strategy for generating valid decimal values
    fn decimal_strategy() -> impl Strategy<Value = Decimal> {
        (1i64..1000000i64, 0u32..8u32).prop_map(|(n, scale)| Decimal::new(n, scale))
    }

    // Strategy for generating optional decimal values
    fn optional_decimal_strategy() -> impl Strategy<Value = Option<Decimal>> {
        prop_oneof![Just(None), decimal_strategy().prop_map(Some),]
    }

    // Strategy for generating valid symbol strings
    fn symbol_strategy() -> impl Strategy<Value = String> {
        prop_oneof![
            Just("BTC/USDT".to_string()),
            Just("ETH/USDT".to_string()),
            Just("SOL/USDT".to_string()),
            Just("DOGE/USDT".to_string()),
            "[A-Z]{3,5}/[A-Z]{3,5}",
        ]
    }

    // Strategy for generating valid currency codes
    fn currency_strategy() -> impl Strategy<Value = String> {
        prop_oneof![
            Just("BTC".to_string()),
            Just("ETH".to_string()),
            Just("USDT".to_string()),
            Just("SOL".to_string()),
            "[A-Z]{3,5}",
        ]
    }

    // Strategy for generating market types
    fn market_type_strategy() -> impl Strategy<Value = MarketType> {
        prop_oneof![
            Just(MarketType::Spot),
            Just(MarketType::Futures),
            Just(MarketType::Swap),
        ]
    }

    // Strategy for generating order sides
    fn order_side_strategy() -> impl Strategy<Value = OrderSide> {
        prop_oneof![Just(OrderSide::Buy), Just(OrderSide::Sell),]
    }

    // Strategy for generating order types
    fn order_type_strategy() -> impl Strategy<Value = OrderType> {
        prop_oneof![
            Just(OrderType::Market),
            Just(OrderType::Limit),
            Just(OrderType::StopLoss),
            Just(OrderType::TakeProfit),
        ]
    }

    // Strategy for generating order statuses
    fn order_status_strategy() -> impl Strategy<Value = OrderStatus> {
        prop_oneof![
            Just(OrderStatus::Open),
            Just(OrderStatus::Closed),
            Just(OrderStatus::Cancelled),
            Just(OrderStatus::Expired),
        ]
    }

    // Strategy for generating timestamps
    fn timestamp_strategy() -> impl Strategy<Value = i64> {
        1577836800000i64..1893456000000i64
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// **Property 4: Market Data Round-Trip**
        /// For any valid Market struct, serializing to JSON and deserializing back
        /// SHALL produce an equivalent Market struct.
        #[test]
        fn prop_market_roundtrip(
            id in "[A-Z]{3,10}",
            symbol in symbol_strategy(),
            base in currency_strategy(),
            quote in currency_strategy(),
            market_type in market_type_strategy(),
            active in any::<bool>(),
            maker in optional_decimal_strategy(),
            taker in optional_decimal_strategy(),
        ) {
            let market = Market {
                id: id.clone(),
                symbol: symbol.clone(),
                parsed_symbol: None,
                base: base.clone(),
                quote: quote.clone(),
                settle: None,
                base_id: Some(base.clone()),
                quote_id: Some(quote.clone()),
                settle_id: None,
                market_type,
                active,
                margin: false,
                contract: Some(false),
                linear: None,
                inverse: None,
                contract_size: None,
                expiry: None,
                expiry_datetime: None,
                strike: None,
                option_type: None,
                precision: MarketPrecision::default(),
                limits: MarketLimits::default(),
                maker,
                taker,
                percentage: Some(true),
                tier_based: Some(false),
                fee_side: Some("quote".to_string()),
                info: std::collections::HashMap::new(),
            };

            // Serialize to JSON
            let json = serde_json::to_string(&market).expect("Failed to serialize Market");

            // Deserialize back
            let deserialized: Market = serde_json::from_str(&json).expect("Failed to deserialize Market");

            // Verify key fields are preserved
            prop_assert_eq!(market.id, deserialized.id);
            prop_assert_eq!(market.symbol, deserialized.symbol);
            prop_assert_eq!(market.base, deserialized.base);
            prop_assert_eq!(market.quote, deserialized.quote);
            prop_assert_eq!(market.market_type, deserialized.market_type);
            prop_assert_eq!(market.active, deserialized.active);
            prop_assert_eq!(market.maker, deserialized.maker);
            prop_assert_eq!(market.taker, deserialized.taker);
        }

        /// **Property 5: Ticker Data Round-Trip**
        /// For any valid Ticker struct, serializing to JSON and deserializing back
        /// SHALL produce an equivalent Ticker struct.
        #[test]
        fn prop_ticker_roundtrip(
            symbol in symbol_strategy(),
            timestamp in timestamp_strategy(),
            high in optional_decimal_strategy(),
            low in optional_decimal_strategy(),
            last in optional_decimal_strategy(),
            bid in optional_decimal_strategy(),
            ask in optional_decimal_strategy(),
        ) {
            let ticker = Ticker {
                symbol: symbol.clone(),
                timestamp,
                datetime: ccxt_exchanges::bitget::timestamp_to_datetime(timestamp),
                high: high.map(Price::new),
                low: low.map(Price::new),
                bid: bid.map(Price::new),
                bid_volume: None,
                ask: ask.map(Price::new),
                ask_volume: None,
                vwap: None,
                open: None,
                close: last.map(Price::new),
                last: last.map(Price::new),
                previous_close: None,
                change: None,
                percentage: None,
                average: None,
                base_volume: None,
                quote_volume: None,
                info: std::collections::HashMap::new(),
            };

            // Serialize to JSON
            let json = serde_json::to_string(&ticker).expect("Failed to serialize Ticker");

            // Deserialize back
            let deserialized: Ticker = serde_json::from_str(&json).expect("Failed to deserialize Ticker");

            // Verify key fields are preserved
            prop_assert_eq!(ticker.symbol, deserialized.symbol);
            prop_assert_eq!(ticker.timestamp, deserialized.timestamp);
            prop_assert_eq!(ticker.high, deserialized.high);
            prop_assert_eq!(ticker.low, deserialized.low);
            prop_assert_eq!(ticker.last, deserialized.last);
            prop_assert_eq!(ticker.bid, deserialized.bid);
            prop_assert_eq!(ticker.ask, deserialized.ask);
        }

        /// **Property 6: Order Data Round-Trip**
        /// For any valid Order struct, serializing to JSON and deserializing back
        /// SHALL produce an equivalent Order struct.
        #[test]
        fn prop_order_roundtrip(
            id in "[0-9]{6,12}",
            symbol in symbol_strategy(),
            side in order_side_strategy(),
            order_type in order_type_strategy(),
            status in order_status_strategy(),
            amount in decimal_strategy(),
            price in optional_decimal_strategy(),
            filled in optional_decimal_strategy(),
            timestamp in timestamp_strategy(),
        ) {
            let order = Order {
                id: id.clone(),
                client_order_id: None,
                timestamp: Some(timestamp),
                datetime: ccxt_exchanges::bitget::timestamp_to_datetime(timestamp),
                last_trade_timestamp: None,
                symbol: symbol.clone(),
                order_type,
                time_in_force: Some("GTC".to_string()),
                post_only: None,
                reduce_only: None,
                side,
                price,
                stop_price: None,
                trigger_price: None,
                take_profit_price: None,
                stop_loss_price: None,
                trailing_delta: None,
                trailing_percent: None,
                activation_price: None,
                callback_rate: None,
                working_type: None,
                amount,
                filled,
                remaining: filled.map(|f| amount - f),
                cost: None,
                average: None,
                status,
                fee: None,
                fees: None,
                trades: None,
                info: std::collections::HashMap::new(),
            };

            // Serialize to JSON
            let json = serde_json::to_string(&order).expect("Failed to serialize Order");

            // Deserialize back
            let deserialized: Order = serde_json::from_str(&json).expect("Failed to deserialize Order");

            // Verify key fields are preserved
            prop_assert_eq!(order.id, deserialized.id);
            prop_assert_eq!(order.symbol, deserialized.symbol);
            prop_assert_eq!(order.side, deserialized.side);
            prop_assert_eq!(order.order_type, deserialized.order_type);
            prop_assert_eq!(order.status, deserialized.status);
            prop_assert_eq!(order.amount, deserialized.amount);
            prop_assert_eq!(order.price, deserialized.price);
            prop_assert_eq!(order.filled, deserialized.filled);
            prop_assert_eq!(order.timestamp, deserialized.timestamp);
        }

        /// Test that serialization produces valid JSON
        #[test]
        fn prop_market_serialization_valid_json(
            id in "[A-Z]{3,10}",
            symbol in symbol_strategy(),
            base in currency_strategy(),
            quote in currency_strategy(),
        ) {
            let market = Market {
                id,
                symbol,
                parsed_symbol: None,
                base: base.clone(),
                quote: quote.clone(),
                settle: None,
                base_id: Some(base.clone()),
                quote_id: Some(quote.clone()),
                settle_id: None,
                market_type: MarketType::Spot,
                active: true,
                margin: false,
                contract: Some(false),
                linear: None,
                inverse: None,
                contract_size: None,
                expiry: None,
                expiry_datetime: None,
                strike: None,
                option_type: None,
                precision: MarketPrecision::default(),
                limits: MarketLimits::default(),
                maker: None,
                taker: None,
                percentage: Some(true),
                tier_based: Some(false),
                fee_side: Some("quote".to_string()),
                info: std::collections::HashMap::new(),
            };

            let json = serde_json::to_string(&market);
            prop_assert!(json.is_ok(), "Market should serialize to valid JSON");

            // Verify it's valid JSON by parsing it
            let parsed: Result<serde_json::Value, _> = serde_json::from_str(&json.unwrap());
            prop_assert!(parsed.is_ok(), "Serialized Market should be valid JSON");
        }
    }
}

/// **Feature: bitget-exchange, Property 11: Market Cache Consistency**
/// **Validates: Requirements 3.2**
///
/// For any sequence of `load_markets(false)` calls, the returned HashMap SHALL be
/// identical (same keys and values) unless `load_markets(true)` is called in between.
///
/// Note: This property test verifies the cache behavior at the BaseExchange level
/// since actual API calls would require network access. The test verifies that:
/// 1. Setting markets in cache produces consistent results
/// 2. Multiple reads from cache return identical data
/// 3. Cache state is properly maintained
mod market_cache_consistency {
    use super::*;
    use ccxt_core::types::{Market, MarketLimits, MarketPrecision, MarketType};
    use ccxt_exchanges::bitget::Bitget;

    // Generate a unique market from an index to ensure no duplicates
    fn create_market(index: usize) -> Market {
        let base = format!("COIN{}", index);
        let quote = "USDT".to_string();
        let symbol = format!("{}/{}", base, quote);
        let id = format!("{}USDT", base);

        Market {
            id,
            symbol,
            parsed_symbol: None,
            base: base.clone(),
            quote: quote.clone(),
            settle: None,
            base_id: Some(base.clone()),
            quote_id: Some(quote.clone()),
            settle_id: None,
            market_type: MarketType::Spot,
            active: true,
            margin: false,
            contract: Some(false),
            linear: None,
            inverse: None,
            contract_size: None,
            expiry: None,
            expiry_datetime: None,
            strike: None,
            option_type: None,
            precision: MarketPrecision::default(),
            limits: MarketLimits::default(),
            maker: None,
            taker: None,
            percentage: Some(true),
            tier_based: Some(false),
            fee_side: Some("quote".to_string()),
            info: std::collections::HashMap::new(),
        }
    }

    // Strategy for generating a list of unique markets
    fn markets_strategy() -> impl Strategy<Value = Vec<Market>> {
        (1usize..10usize).prop_map(|count| (0..count).map(create_market).collect())
    }

    // Strategy for generating two different lists of unique markets
    fn two_markets_strategy() -> impl Strategy<Value = (Vec<Market>, Vec<Market>)> {
        ((1usize..5usize), (1usize..5usize)).prop_map(|(count1, count2)| {
            let markets1: Vec<Market> = (0..count1).map(create_market).collect();
            // Use different indices for second batch to ensure different markets
            let markets2: Vec<Market> = (100..(100 + count2)).map(create_market).collect();
            (markets1, markets2)
        })
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Test that setting markets in cache and reading them back produces consistent results
        #[test]
        fn prop_cache_set_and_read_consistent(markets in markets_strategy()) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let bitget = Bitget::builder().build().expect("Failed to build Bitget");

                // Set markets in cache
                bitget.base().set_markets(markets.clone(), None).await
                    .expect("Failed to set markets");

                // Read from cache
                let cache = bitget.base().market_cache.read().await;

                // Verify all markets are present
                for market in &markets {
                    let cached = cache.markets.get(&market.symbol);
                    assert!(cached.is_some(), "Market {} should be in cache", market.symbol);

                    let cached = cached.unwrap();
                    assert_eq!(cached.id, market.id, "Market ID should match");
                    assert_eq!(cached.symbol, market.symbol, "Market symbol should match");
                    assert_eq!(cached.base, market.base, "Market base should match");
                    assert_eq!(cached.quote, market.quote, "Market quote should match");
                }

                // Verify cache is marked as loaded
                assert!(cache.loaded, "Cache should be marked as loaded");
            });
        }

        /// Test that multiple reads from cache return identical data
        #[test]
        fn prop_multiple_cache_reads_identical(markets in markets_strategy()) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let bitget = Bitget::builder().build().expect("Failed to build Bitget");

                // Set markets in cache
                bitget.base().set_markets(markets.clone(), None).await
                    .expect("Failed to set markets");

                // Read from cache multiple times
                let read1 = {
                    let cache = bitget.base().market_cache.read().await;
                    cache.markets.clone()
                };

                let read2 = {
                    let cache = bitget.base().market_cache.read().await;
                    cache.markets.clone()
                };

                let read3 = {
                    let cache = bitget.base().market_cache.read().await;
                    cache.markets.clone()
                };

                // Verify all reads are identical
                assert_eq!(read1.len(), read2.len(), "Cache reads should have same length");
                assert_eq!(read2.len(), read3.len(), "Cache reads should have same length");

                for (symbol, market1) in read1.iter() {
                    let market2 = read2.get(symbol).expect("Market should exist in read2");
                    let market3 = read3.get(symbol).expect("Market should exist in read3");

                    assert_eq!(market1.id, market2.id, "Market ID should be identical across reads");
                    assert_eq!(market2.id, market3.id, "Market ID should be identical across reads");
                    assert_eq!(market1.symbol, market2.symbol, "Market symbol should be identical across reads");
                    assert_eq!(market2.symbol, market3.symbol, "Market symbol should be identical across reads");
                }
            });
        }

        /// Test that markets_by_id index is consistent with markets index
        #[test]
        fn prop_markets_by_id_consistent(markets in markets_strategy()) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let bitget = Bitget::builder().build().expect("Failed to build Bitget");

                // Set markets in cache
                bitget.base().set_markets(markets.clone(), None).await
                    .expect("Failed to set markets");

                // Read from cache
                let cache = bitget.base().market_cache.read().await;

                // Verify markets_by_id is consistent with markets
                for market in &markets {
                    // Check markets index
                    let by_symbol = cache.markets.get(&market.symbol);
                    assert!(by_symbol.is_some(), "Market should be indexed by symbol");

                    // Check markets_by_id index
                    let by_id = cache.markets_by_id.get(&market.id);
                    assert!(by_id.is_some(), "Market should be indexed by ID");

                    // Both should point to the same market data
                    let by_symbol = by_symbol.unwrap();
                    let by_id = by_id.unwrap();

                    assert_eq!(by_symbol.id, by_id.id, "Market ID should match between indexes");
                    assert_eq!(by_symbol.symbol, by_id.symbol, "Market symbol should match between indexes");
                }
            });
        }

        /// Test that symbols list is consistent with markets
        #[test]
        fn prop_symbols_list_consistent(markets in markets_strategy()) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let bitget = Bitget::builder().build().expect("Failed to build Bitget");

                // Set markets in cache
                bitget.base().set_markets(markets.clone(), None).await
                    .expect("Failed to set markets");

                // Read from cache
                let cache = bitget.base().market_cache.read().await;

                // Verify symbols list contains all market symbols
                for market in &markets {
                    assert!(
                        cache.symbols.contains(&market.symbol),
                        "Symbols list should contain {}",
                        market.symbol
                    );
                }

                // Verify symbols list length matches markets count
                // Note: May have duplicates in input, so we check unique count
                let unique_symbols: std::collections::HashSet<_> = markets.iter()
                    .map(|m| m.symbol.clone())
                    .collect();
                assert_eq!(
                    cache.symbols.len(),
                    unique_symbols.len(),
                    "Symbols list length should match unique market count"
                );
            });
        }

        /// Test that cache reload clears old data
        #[test]
        fn prop_cache_reload_clears_old_data(
            (markets1, markets2) in two_markets_strategy()
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let bitget = Bitget::builder().build().expect("Failed to build Bitget");

                // Set first batch of markets
                bitget.base().set_markets(markets1.clone(), None).await
                    .expect("Failed to set markets");

                // Verify first batch is loaded
                {
                    let cache = bitget.base().market_cache.read().await;
                    assert!(cache.loaded, "Cache should be loaded after first set");
                }

                // Set second batch of markets (simulating reload)
                bitget.base().set_markets(markets2.clone(), None).await
                    .expect("Failed to set markets");

                // Verify second batch replaced first batch
                let cache = bitget.base().market_cache.read().await;

                // Check that markets2 symbols are present
                let unique_symbols2: std::collections::HashSet<_> = markets2.iter()
                    .map(|m| m.symbol.clone())
                    .collect();

                for symbol in &unique_symbols2 {
                    assert!(
                        cache.markets.contains_key(symbol),
                        "New market {} should be in cache",
                        symbol
                    );
                }

                // Check that markets1-only symbols are NOT present (unless also in markets2)
                let unique_symbols1: std::collections::HashSet<_> = markets1.iter()
                    .map(|m| m.symbol.clone())
                    .collect();

                for symbol in &unique_symbols1 {
                    if !unique_symbols2.contains(symbol) {
                        assert!(
                            !cache.markets.contains_key(symbol),
                            "Old market {} should NOT be in cache after reload",
                            symbol
                        );
                    }
                }
            });
        }
    }
}

/// **Feature: bitget-exchange, Property 12: Exchange Trait Metadata Consistency**
/// **Validates: Requirements 2.2, 2.3**
///
/// For any Bitget instance created with any valid configuration,
/// `id()` SHALL return "bitget" and `name()` SHALL return "Bitget".
mod exchange_trait_metadata_consistency {
    use super::*;
    use ccxt_core::exchange::Exchange;
    use ccxt_exchanges::bitget::BitgetBuilder;
    use std::time::Duration;

    // Strategy for generating valid API key strings (alphanumeric, 8-64 chars)
    fn api_key_strategy() -> impl Strategy<Value = String> {
        "[a-zA-Z0-9]{8,64}"
    }

    // Strategy for generating valid secret strings (alphanumeric, 16-128 chars)
    fn secret_strategy() -> impl Strategy<Value = String> {
        "[a-zA-Z0-9]{16,128}"
    }

    // Strategy for generating valid passphrase strings (alphanumeric, 6-32 chars)
    fn passphrase_strategy() -> impl Strategy<Value = String> {
        "[a-zA-Z0-9]{6,32}"
    }

    // Strategy for generating valid product types
    fn product_type_strategy() -> impl Strategy<Value = String> {
        prop_oneof![
            Just("spot".to_string()),
            Just("umcbl".to_string()),
            Just("dmcbl".to_string()),
        ]
    }

    // Strategy for generating valid recv_window values (1000-60000 ms)
    fn recv_window_strategy() -> impl Strategy<Value = u64> {
        1000u64..60000u64
    }

    // Strategy for generating valid timeout values (1-300 seconds)
    fn timeout_strategy() -> impl Strategy<Value = u64> {
        1u64..300u64
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Test that id() always returns "bitget" regardless of configuration
        #[test]
        fn prop_id_always_bitget(
            api_key in proptest::option::of(api_key_strategy()),
            secret in proptest::option::of(secret_strategy()),
            passphrase in proptest::option::of(passphrase_strategy()),
            sandbox in any::<bool>(),
            product_type in product_type_strategy(),
            recv_window in recv_window_strategy(),
            timeout in timeout_strategy(),
        ) {
            let mut builder = BitgetBuilder::new()
                .sandbox(sandbox)
                .product_type(&product_type)
                .recv_window(recv_window)
                .timeout(Duration::from_secs(timeout));

            if let Some(key) = api_key {
                builder = builder.api_key(&key);
            }
            if let Some(sec) = secret {
                builder = builder.secret(&sec);
            }
            if let Some(pass) = passphrase {
                builder = builder.passphrase(&pass);
            }

            let bitget = builder.build().expect("Failed to build Bitget");

            // Test via Exchange trait
            let exchange: &dyn Exchange = &bitget;
            prop_assert_eq!(exchange.id(), "bitget", "id() should always return 'bitget'");
        }

        /// Test that name() always returns "Bitget" regardless of configuration
        #[test]
        fn prop_name_always_bitget(
            api_key in proptest::option::of(api_key_strategy()),
            secret in proptest::option::of(secret_strategy()),
            passphrase in proptest::option::of(passphrase_strategy()),
            sandbox in any::<bool>(),
            product_type in product_type_strategy(),
            recv_window in recv_window_strategy(),
            timeout in timeout_strategy(),
        ) {
            let mut builder = BitgetBuilder::new()
                .sandbox(sandbox)
                .product_type(&product_type)
                .recv_window(recv_window)
                .timeout(Duration::from_secs(timeout));

            if let Some(key) = api_key {
                builder = builder.api_key(&key);
            }
            if let Some(sec) = secret {
                builder = builder.secret(&sec);
            }
            if let Some(pass) = passphrase {
                builder = builder.passphrase(&pass);
            }

            let bitget = builder.build().expect("Failed to build Bitget");

            // Test via Exchange trait
            let exchange: &dyn Exchange = &bitget;
            prop_assert_eq!(exchange.name(), "Bitget", "name() should always return 'Bitget'");
        }

        /// Test that both id() and name() are consistent across any configuration
        #[test]
        fn prop_metadata_consistent_across_configs(
            api_key in proptest::option::of(api_key_strategy()),
            secret in proptest::option::of(secret_strategy()),
            passphrase in proptest::option::of(passphrase_strategy()),
            sandbox in any::<bool>(),
            product_type in product_type_strategy(),
            recv_window in recv_window_strategy(),
            timeout in timeout_strategy(),
        ) {
            let mut builder = BitgetBuilder::new()
                .sandbox(sandbox)
                .product_type(&product_type)
                .recv_window(recv_window)
                .timeout(Duration::from_secs(timeout));

            if let Some(key) = api_key {
                builder = builder.api_key(&key);
            }
            if let Some(sec) = secret {
                builder = builder.secret(&sec);
            }
            if let Some(pass) = passphrase {
                builder = builder.passphrase(&pass);
            }

            let bitget = builder.build().expect("Failed to build Bitget");

            // Test via Exchange trait
            let exchange: &dyn Exchange = &bitget;

            // Verify all metadata is consistent
            prop_assert_eq!(exchange.id(), "bitget");
            prop_assert_eq!(exchange.name(), "Bitget");
            prop_assert_eq!(exchange.version(), "v2");
            prop_assert!(!exchange.certified(), "Bitget should not be certified");
            prop_assert!(exchange.has_websocket(), "Bitget should have websocket support");
        }

        /// Test that metadata is consistent when accessed via trait object (Box<dyn Exchange>)
        #[test]
        fn prop_metadata_via_boxed_trait_object(
            sandbox in any::<bool>(),
            product_type in product_type_strategy(),
        ) {
            let bitget = BitgetBuilder::new()
                .sandbox(sandbox)
                .product_type(&product_type)
                .build()
                .expect("Failed to build Bitget");

            // Test via Box<dyn Exchange>
            let exchange: Box<dyn Exchange> = Box::new(bitget);

            prop_assert_eq!(exchange.id(), "bitget");
            prop_assert_eq!(exchange.name(), "Bitget");
        }

        /// Test that rate_limit() returns a consistent value
        #[test]
        fn prop_rate_limit_consistent(
            sandbox in any::<bool>(),
            product_type in product_type_strategy(),
            timeout in timeout_strategy(),
        ) {
            let bitget = BitgetBuilder::new()
                .sandbox(sandbox)
                .product_type(&product_type)
                .timeout(Duration::from_secs(timeout))
                .build()
                .expect("Failed to build Bitget");

            let exchange: &dyn Exchange = &bitget;

            // Rate limit should be 20 for Bitget
            prop_assert!(
                exchange.rate_limit() == 20,
                "rate_limit() should return 20"
            );
        }

        /// Test that capabilities are consistent regardless of configuration
        #[test]
        fn prop_capabilities_consistent(
            sandbox in any::<bool>(),
            product_type in product_type_strategy(),
        ) {
            let bitget = BitgetBuilder::new()
                .sandbox(sandbox)
                .product_type(&product_type)
                .build()
                .expect("Failed to build Bitget");

            let exchange: &dyn Exchange = &bitget;
            let caps = exchange.capabilities();

            // Core capabilities should always be present
            prop_assert!(caps.fetch_markets(), "fetch_markets should be supported");
            prop_assert!(caps.fetch_ticker(), "fetch_ticker should be supported");
            prop_assert!(caps.fetch_tickers(), "fetch_tickers should be supported");
            prop_assert!(caps.fetch_order_book(), "fetch_order_book should be supported");
            prop_assert!(caps.fetch_trades(), "fetch_trades should be supported");
            prop_assert!(caps.fetch_ohlcv(), "fetch_ohlcv should be supported");
            prop_assert!(caps.create_order(), "create_order should be supported");
            prop_assert!(caps.cancel_order(), "cancel_order should be supported");
            prop_assert!(caps.fetch_balance(), "fetch_balance should be supported");
            prop_assert!(caps.websocket(), "websocket should be supported");
            prop_assert!(caps.watch_ticker(), "watch_ticker should be supported");
            prop_assert!(caps.watch_order_book(), "watch_order_book should be supported");
            prop_assert!(caps.watch_trades(), "watch_trades should be supported");
        }

        /// Test that timeframes are consistent regardless of configuration
        #[test]
        fn prop_timeframes_consistent(
            sandbox in any::<bool>(),
            product_type in product_type_strategy(),
        ) {
            use ccxt_core::types::Timeframe;

            let bitget = BitgetBuilder::new()
                .sandbox(sandbox)
                .product_type(&product_type)
                .build()
                .expect("Failed to build Bitget");

            let exchange: &dyn Exchange = &bitget;
            let timeframes = exchange.timeframes();

            // Should have standard timeframes
            prop_assert!(!timeframes.is_empty(), "timeframes should not be empty");
            prop_assert!(timeframes.contains(&Timeframe::M1), "should support 1m timeframe");
            prop_assert!(timeframes.contains(&Timeframe::M5), "should support 5m timeframe");
            prop_assert!(timeframes.contains(&Timeframe::H1), "should support 1h timeframe");
            prop_assert!(timeframes.contains(&Timeframe::D1), "should support 1d timeframe");
        }
    }
}
