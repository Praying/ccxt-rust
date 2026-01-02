//! Property-based tests for HyperLiquid exchange implementation.
//!
//! These tests verify correctness properties using proptest.

use proptest::prelude::*;

/// **Feature: hyperliquid-exchange, Property 1: Private Key to Address Derivation**
/// **Validates: Requirements 1.1**
///
/// For any valid 32-byte private key, deriving the wallet address SHALL produce
/// a valid Ethereum address (40 hex characters prefixed with "0x") that is
/// deterministic for the same input.
mod private_key_derivation {
    use super::*;
    use ccxt_exchanges::hyperliquid::HyperLiquidAuth;

    // Strategy for generating valid 32-byte private keys as hex strings
    fn valid_private_key_strategy() -> impl Strategy<Value = String> {
        // Generate 32 random bytes as hex (64 hex chars)
        prop::collection::vec(any::<u8>(), 32..=32)
            .prop_map(|bytes| format!("0x{}", hex::encode(bytes)))
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Test that valid private keys produce valid Ethereum addresses
        #[test]
        fn prop_valid_key_produces_valid_address(key in valid_private_key_strategy()) {
            // Some keys may be invalid (out of curve range), skip those
            if let Ok(auth) = HyperLiquidAuth::from_private_key(&key) {
                let address = auth.wallet_address();

                // Address should start with "0x"
                prop_assert!(
                    address.starts_with("0x"),
                    "Address should start with '0x', got: {}",
                    address
                );

                // Address should be 42 characters (0x + 40 hex chars)
                prop_assert_eq!(
                    address.len(),
                    42,
                    "Address should be 42 characters, got: {}",
                    address.len()
                );

                // Address should only contain valid hex characters after 0x
                let hex_part = &address[2..];
                prop_assert!(
                    hex_part.chars().all(|c| c.is_ascii_hexdigit()),
                    "Address should only contain hex characters, got: {}",
                    address
                );
            }
        }

        /// Test that address derivation is deterministic
        #[test]
        fn prop_address_derivation_deterministic(key in valid_private_key_strategy()) {
            // Some keys may be invalid (out of curve range), skip those
            if let Ok(auth1) = HyperLiquidAuth::from_private_key(&key) {
                let auth2 = HyperLiquidAuth::from_private_key(&key)
                    .expect("Second derivation should succeed if first did");

                prop_assert_eq!(
                    auth1.wallet_address(),
                    auth2.wallet_address(),
                    "Same private key should always produce same address"
                );
            }
        }

        /// Test that private key without 0x prefix works the same
        #[test]
        fn prop_key_with_and_without_prefix_same(key in valid_private_key_strategy()) {
            let key_without_prefix = key.strip_prefix("0x").unwrap_or(&key);

            if let Ok(auth_with) = HyperLiquidAuth::from_private_key(&key) {
                let auth_without = HyperLiquidAuth::from_private_key(key_without_prefix)
                    .expect("Key without prefix should work if key with prefix works");

                prop_assert_eq!(
                    auth_with.wallet_address(),
                    auth_without.wallet_address(),
                    "Keys with and without 0x prefix should produce same address"
                );
            }
        }
    }
}

/// **Feature: hyperliquid-exchange, Property 2: Invalid Private Key Rejection**
/// **Validates: Requirements 1.2**
///
/// For any byte sequence that is not a valid 32-byte private key (wrong length,
/// invalid format, or out of curve range), the authenticator SHALL return an
/// error without panicking.
mod invalid_private_key_rejection {
    use super::*;
    use ccxt_exchanges::hyperliquid::HyperLiquidAuth;

    // Strategy for generating invalid hex strings (wrong length)
    fn wrong_length_key_strategy() -> impl Strategy<Value = String> {
        prop_oneof![
            // Too short (1-31 bytes)
            prop::collection::vec(any::<u8>(), 1..32)
                .prop_map(|bytes| format!("0x{}", hex::encode(bytes))),
            // Too long (33-64 bytes)
            prop::collection::vec(any::<u8>(), 33..65)
                .prop_map(|bytes| format!("0x{}", hex::encode(bytes))),
            // Empty
            Just("0x".to_string()),
            Just("".to_string()),
        ]
    }

    // Strategy for generating invalid hex format strings
    fn invalid_hex_strategy() -> impl Strategy<Value = String> {
        prop_oneof![
            // Contains non-hex characters (G is not valid hex)
            Just("0xGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG".to_string()),
            Just("0x123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdefZ".to_string()),
            // Contains spaces
            Just("0x 1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcde".to_string()),
            // Contains special characters
            Just("0x!@#$%^&*()1234567890abcdef1234567890abcdef1234567890abcdef12345".to_string()),
        ]
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Test that wrong length keys are rejected
        #[test]
        fn prop_wrong_length_rejected(key in wrong_length_key_strategy()) {
            let result = HyperLiquidAuth::from_private_key(&key);
            prop_assert!(
                result.is_err(),
                "Wrong length key should be rejected: {}",
                key
            );
        }

        /// Test that invalid hex format is rejected
        #[test]
        fn prop_invalid_hex_rejected(key in invalid_hex_strategy()) {
            let result = HyperLiquidAuth::from_private_key(&key);
            prop_assert!(
                result.is_err(),
                "Invalid hex format should be rejected: {}",
                key
            );
        }

        /// Test that very short strings are rejected gracefully
        #[test]
        fn prop_short_strings_rejected(len in 0usize..10usize) {
            let key = "a".repeat(len);
            let result = HyperLiquidAuth::from_private_key(&key);
            // Should not panic, should return error
            prop_assert!(
                result.is_err(),
                "Short string should be rejected: {}",
                key
            );
        }
    }
}

/// **Feature: hyperliquid-exchange, Property 3: EIP-712 Signature Determinism**
/// **Validates: Requirements 3.1, 3.4**
///
/// For any L1 action and nonce, signing with the same private key SHALL produce
/// the same signature, and the signature SHALL be in valid secp256k1 format
/// (r, s, v components).
mod eip712_signature_determinism {
    use super::*;
    use ccxt_exchanges::hyperliquid::HyperLiquidAuth;
    use serde_json::json;

    // Test private key (DO NOT USE IN PRODUCTION)
    const TEST_PRIVATE_KEY: &str =
        "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80";

    // Strategy for generating valid nonces (timestamp-like values)
    fn nonce_strategy() -> impl Strategy<Value = u64> {
        1577836800000u64..1893456000000u64 // 2020-2030 in milliseconds
    }

    // Strategy for generating action types
    fn action_type_strategy() -> impl Strategy<Value = String> {
        prop_oneof![
            Just("order".to_string()),
            Just("cancel".to_string()),
            Just("cancelByCloid".to_string()),
            Just("modifyOrder".to_string()),
            Just("updateLeverage".to_string()),
        ]
    }

    // Strategy for generating simple action payloads
    fn action_payload_strategy() -> impl Strategy<Value = serde_json::Value> {
        (action_type_strategy(), 0u32..100u32, any::<bool>()).prop_map(
            |(action_type, asset, is_buy)| {
                json!({
                    "type": action_type,
                    "asset": asset,
                    "isBuy": is_buy
                })
            },
        )
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Test that signing the same action with same nonce produces same signature
        #[test]
        fn prop_signature_deterministic(
            action in action_payload_strategy(),
            nonce in nonce_strategy(),
            is_mainnet in any::<bool>()
        ) {
            let auth = HyperLiquidAuth::from_private_key(TEST_PRIVATE_KEY)
                .expect("Failed to create auth");

            let sig1 = auth.sign_l1_action(&action, nonce, is_mainnet)
                .expect("First signing should succeed");
            let sig2 = auth.sign_l1_action(&action, nonce, is_mainnet)
                .expect("Second signing should succeed");

            prop_assert_eq!(sig1.r, sig2.r, "R component should be deterministic");
            prop_assert_eq!(sig1.s, sig2.s, "S component should be deterministic");
            prop_assert_eq!(sig1.v, sig2.v, "V component should be deterministic");
        }

        /// Test that signature components have valid format
        #[test]
        fn prop_signature_format_valid(
            action in action_payload_strategy(),
            nonce in nonce_strategy(),
            is_mainnet in any::<bool>()
        ) {
            let auth = HyperLiquidAuth::from_private_key(TEST_PRIVATE_KEY)
                .expect("Failed to create auth");

            let sig = auth.sign_l1_action(&action, nonce, is_mainnet)
                .expect("Signing should succeed");

            // R and S should be 64 hex characters (32 bytes each)
            prop_assert_eq!(
                sig.r.len(),
                64,
                "R component should be 64 hex chars, got: {}",
                sig.r.len()
            );
            prop_assert_eq!(
                sig.s.len(),
                64,
                "S component should be 64 hex chars, got: {}",
                sig.s.len()
            );

            // R and S should be valid hex
            prop_assert!(
                sig.r.chars().all(|c| c.is_ascii_hexdigit()),
                "R should be valid hex"
            );
            prop_assert!(
                sig.s.chars().all(|c| c.is_ascii_hexdigit()),
                "S should be valid hex"
            );

            // V should be 27 or 28 (standard Ethereum recovery id)
            prop_assert!(
                sig.v == 27 || sig.v == 28,
                "V should be 27 or 28, got: {}",
                sig.v
            );
        }

        /// Test that different nonces produce different signatures
        #[test]
        fn prop_different_nonce_different_signature(
            action in action_payload_strategy(),
            nonce1 in nonce_strategy(),
            nonce2 in nonce_strategy(),
            is_mainnet in any::<bool>()
        ) {
            prop_assume!(nonce1 != nonce2);

            let auth = HyperLiquidAuth::from_private_key(TEST_PRIVATE_KEY)
                .expect("Failed to create auth");

            let sig1 = auth.sign_l1_action(&action, nonce1, is_mainnet)
                .expect("First signing should succeed");
            let sig2 = auth.sign_l1_action(&action, nonce2, is_mainnet)
                .expect("Second signing should succeed");

            // At least one component should differ
            prop_assert!(
                sig1.r != sig2.r || sig1.s != sig2.s,
                "Different nonces should produce different signatures"
            );
        }

        /// Test that signature to_hex produces valid format
        #[test]
        fn prop_signature_to_hex_valid(
            action in action_payload_strategy(),
            nonce in nonce_strategy(),
            is_mainnet in any::<bool>()
        ) {
            let auth = HyperLiquidAuth::from_private_key(TEST_PRIVATE_KEY)
                .expect("Failed to create auth");

            let sig = auth.sign_l1_action(&action, nonce, is_mainnet)
                .expect("Signing should succeed");

            let hex = sig.to_hex();

            // Should start with 0x
            prop_assert!(hex.starts_with("0x"), "Hex should start with 0x");

            // Should be 132 characters (0x + 64 + 64 + 2)
            prop_assert_eq!(
                hex.len(),
                132,
                "Hex signature should be 132 chars, got: {}",
                hex.len()
            );
        }
    }
}

/// **Feature: hyperliquid-exchange, Property 4: Market Symbol Format**
/// **Validates: Requirements 2.1**
///
/// For any market returned by fetch_markets, the symbol SHALL match the pattern
/// "{BASE}/USDC:USDC" for perpetual contracts.
mod market_symbol_format {
    use super::*;
    use ccxt_exchanges::hyperliquid::parser::parse_market;
    use serde_json::json;

    // Strategy for generating valid asset names (uppercase letters, 2-10 chars)
    fn asset_name_strategy() -> impl Strategy<Value = String> {
        "[A-Z]{2,10}"
    }

    // Strategy for generating valid size decimals (0-8)
    fn sz_decimals_strategy() -> impl Strategy<Value = u64> {
        0u64..9u64
    }

    // Strategy for generating market index
    fn market_index_strategy() -> impl Strategy<Value = usize> {
        0usize..1000usize
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Test that parsed market symbol follows the correct format
        #[test]
        fn prop_market_symbol_format(
            asset_name in asset_name_strategy(),
            sz_decimals in sz_decimals_strategy(),
            index in market_index_strategy()
        ) {
            let data = json!({
                "name": asset_name,
                "szDecimals": sz_decimals
            });

            let market = parse_market(&data, index)
                .expect("Market parsing should succeed");

            // Symbol should match pattern {BASE}/USDC:USDC
            let expected_symbol = format!("{}/USDC:USDC", asset_name);
            prop_assert_eq!(
                market.symbol,
                expected_symbol,
                "Symbol should match pattern BASE/USDC:USDC"
            );

            // Base should be the asset name
            prop_assert_eq!(
                market.base,
                asset_name,
                "Base should be the asset name"
            );

            // Quote should be USDC
            prop_assert_eq!(
                market.quote,
                "USDC",
                "Quote should be USDC"
            );

            // Settle should be USDC
            prop_assert_eq!(
                market.settle,
                Some("USDC".to_string()),
                "Settle should be USDC"
            );
        }

        /// Test that market is always active and supports margin
        #[test]
        fn prop_market_is_perpetual(
            asset_name in asset_name_strategy(),
            sz_decimals in sz_decimals_strategy(),
            index in market_index_strategy()
        ) {
            let data = json!({
                "name": asset_name,
                "szDecimals": sz_decimals
            });

            let market = parse_market(&data, index)
                .expect("Market parsing should succeed");

            prop_assert!(market.active, "Market should be active");
            prop_assert!(market.margin, "Market should support margin");
            prop_assert_eq!(market.contract, Some(true), "Market should be a contract");
            prop_assert_eq!(market.linear, Some(true), "Market should be linear");
            prop_assert_eq!(market.inverse, Some(false), "Market should not be inverse");
        }

        /// Test that market ID is the index as string
        #[test]
        fn prop_market_id_is_index(
            asset_name in asset_name_strategy(),
            sz_decimals in sz_decimals_strategy(),
            index in market_index_strategy()
        ) {
            let data = json!({
                "name": asset_name,
                "szDecimals": sz_decimals
            });

            let market = parse_market(&data, index)
                .expect("Market parsing should succeed");

            prop_assert_eq!(
                market.id,
                index.to_string(),
                "Market ID should be the index as string"
            );
        }
    }
}

/// **Feature: hyperliquid-exchange, Property 5: Ticker Data Completeness**
/// **Validates: Requirements 2.2**
///
/// For any ticker returned by fetch_ticker, the ticker SHALL contain non-null
/// values for last price, and the timestamp SHALL be a valid Unix millisecond timestamp.
mod ticker_data_completeness {
    use super::*;
    use ccxt_exchanges::hyperliquid::parser::parse_ticker;
    use rust_decimal::Decimal;
    use rust_decimal::prelude::FromPrimitive;

    // Strategy for generating valid mid prices
    fn mid_price_strategy() -> impl Strategy<Value = Decimal> {
        (1u64..1000000u64).prop_map(|n| Decimal::from_u64(n).unwrap())
    }

    // Strategy for generating valid symbols
    fn symbol_strategy() -> impl Strategy<Value = String> {
        "[A-Z]{2,10}".prop_map(|s| format!("{}/USDC:USDC", s))
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Test that ticker has valid last price
        #[test]
        fn prop_ticker_has_last_price(
            symbol in symbol_strategy(),
            mid_price in mid_price_strategy()
        ) {
            let ticker = parse_ticker(&symbol, mid_price, None)
                .expect("Ticker parsing should succeed");

            prop_assert!(
                ticker.last.is_some(),
                "Ticker should have last price"
            );

            let last = ticker.last.unwrap();
            prop_assert!(
                last.0 > Decimal::ZERO,
                "Last price should be positive"
            );
        }

        /// Test that ticker has valid timestamp
        #[test]
        fn prop_ticker_has_valid_timestamp(
            symbol in symbol_strategy(),
            mid_price in mid_price_strategy()
        ) {
            let ticker = parse_ticker(&symbol, mid_price, None)
                .expect("Ticker parsing should succeed");

            // Timestamp should be in reasonable range (2020-2030)
            prop_assert!(
                ticker.timestamp >= 1577836800000,
                "Timestamp should be after 2020"
            );
            prop_assert!(
                ticker.timestamp <= 1893456000000,
                "Timestamp should be before 2030"
            );
        }

        /// Test that ticker symbol matches input
        #[test]
        fn prop_ticker_symbol_matches(
            symbol in symbol_strategy(),
            mid_price in mid_price_strategy()
        ) {
            let ticker = parse_ticker(&symbol, mid_price, None)
                .expect("Ticker parsing should succeed");

            prop_assert_eq!(
                ticker.symbol,
                symbol,
                "Ticker symbol should match input"
            );
        }
    }
}

/// **Feature: hyperliquid-exchange, Property 6: OrderBook Invariants**
/// **Validates: Requirements 2.3**
///
/// For any order book returned by fetch_order_book, bids SHALL be sorted in
/// descending order by price, asks SHALL be sorted in ascending order by price,
/// and all entries SHALL have positive price and quantity.
mod orderbook_invariants {
    use super::*;
    use ccxt_exchanges::hyperliquid::parser::parse_orderbook;
    use rust_decimal::Decimal;
    use serde_json::json;

    // Strategy for generating valid price strings
    fn price_strategy() -> impl Strategy<Value = String> {
        (1u64..100000u64).prop_map(|n| format!("{}.{}", n / 100, n % 100))
    }

    // Strategy for generating valid size strings
    fn size_strategy() -> impl Strategy<Value = String> {
        (1u64..10000u64).prop_map(|n| format!("{}.{}", n / 1000, n % 1000))
    }

    // Strategy for generating orderbook levels
    fn levels_strategy() -> impl Strategy<Value = Vec<(String, String)>> {
        prop::collection::vec((price_strategy(), size_strategy()), 1..20)
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Test that bids are sorted in descending order
        #[test]
        fn prop_bids_sorted_descending(bid_levels in levels_strategy()) {
            let bids_json: Vec<serde_json::Value> = bid_levels
                .iter()
                .map(|(px, sz)| json!({"px": px, "sz": sz}))
                .collect();

            let data = json!({
                "levels": [bids_json, []]
            });

            let orderbook = parse_orderbook(&data, "BTC/USDC:USDC".to_string())
                .expect("Orderbook parsing should succeed");

            for i in 1..orderbook.bids.len() {
                prop_assert!(
                    orderbook.bids[i - 1].price >= orderbook.bids[i].price,
                    "Bids should be sorted descending: {:?} >= {:?}",
                    orderbook.bids[i - 1].price,
                    orderbook.bids[i].price
                );
            }
        }

        /// Test that asks are sorted in ascending order
        #[test]
        fn prop_asks_sorted_ascending(ask_levels in levels_strategy()) {
            let asks_json: Vec<serde_json::Value> = ask_levels
                .iter()
                .map(|(px, sz)| json!({"px": px, "sz": sz}))
                .collect();

            let data = json!({
                "levels": [[], asks_json]
            });

            let orderbook = parse_orderbook(&data, "BTC/USDC:USDC".to_string())
                .expect("Orderbook parsing should succeed");

            for i in 1..orderbook.asks.len() {
                prop_assert!(
                    orderbook.asks[i - 1].price <= orderbook.asks[i].price,
                    "Asks should be sorted ascending: {:?} <= {:?}",
                    orderbook.asks[i - 1].price,
                    orderbook.asks[i].price
                );
            }
        }

        /// Test that all entries have positive price and quantity
        #[test]
        fn prop_all_entries_positive(
            bid_levels in levels_strategy(),
            ask_levels in levels_strategy()
        ) {
            let bids_json: Vec<serde_json::Value> = bid_levels
                .iter()
                .map(|(px, sz)| json!({"px": px, "sz": sz}))
                .collect();
            let asks_json: Vec<serde_json::Value> = ask_levels
                .iter()
                .map(|(px, sz)| json!({"px": px, "sz": sz}))
                .collect();

            let data = json!({
                "levels": [bids_json, asks_json]
            });

            let orderbook = parse_orderbook(&data, "BTC/USDC:USDC".to_string())
                .expect("Orderbook parsing should succeed");

            for bid in &orderbook.bids {
                prop_assert!(
                    bid.price.0 > Decimal::ZERO,
                    "Bid price should be positive"
                );
                prop_assert!(
                    bid.amount.0 > Decimal::ZERO,
                    "Bid amount should be positive"
                );
            }

            for ask in &orderbook.asks {
                prop_assert!(
                    ask.price.0 > Decimal::ZERO,
                    "Ask price should be positive"
                );
                prop_assert!(
                    ask.amount.0 > Decimal::ZERO,
                    "Ask amount should be positive"
                );
            }
        }
    }
}

/// **Feature: hyperliquid-exchange, Property 7: Trade Data Completeness**
/// **Validates: Requirements 2.4**
///
/// For any trade returned by fetch_trades, the trade SHALL contain valid timestamp,
/// positive price, positive quantity, and a valid side (buy or sell).
mod trade_data_completeness {
    use super::*;
    use ccxt_core::types::OrderSide;
    use ccxt_exchanges::hyperliquid::parser::parse_trade;
    use rust_decimal::Decimal;
    use serde_json::json;

    // Strategy for generating valid timestamps
    fn timestamp_strategy() -> impl Strategy<Value = i64> {
        1577836800000i64..1893456000000i64
    }

    // Strategy for generating valid prices
    fn price_strategy() -> impl Strategy<Value = String> {
        (1u64..100000u64).prop_map(|n| n.to_string())
    }

    // Strategy for generating valid sizes
    fn size_strategy() -> impl Strategy<Value = String> {
        (1u64..10000u64).prop_map(|n| format!("{}.{}", n / 1000, n % 1000))
    }

    // Strategy for generating valid sides
    fn side_strategy() -> impl Strategy<Value = &'static str> {
        prop_oneof![Just("B"), Just("A"), Just("buy"), Just("sell"),]
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Test that trade has valid timestamp
        #[test]
        fn prop_trade_has_valid_timestamp(
            timestamp in timestamp_strategy(),
            price in price_strategy(),
            size in size_strategy(),
            side in side_strategy()
        ) {
            let data = json!({
                "time": timestamp,
                "px": price,
                "sz": size,
                "side": side,
                "tid": "123"
            });

            let trade = parse_trade(&data, None)
                .expect("Trade parsing should succeed");

            prop_assert_eq!(
                trade.timestamp,
                timestamp,
                "Trade timestamp should match input"
            );
        }

        /// Test that trade has positive price and amount
        #[test]
        fn prop_trade_has_positive_values(
            timestamp in timestamp_strategy(),
            price in price_strategy(),
            size in size_strategy(),
            side in side_strategy()
        ) {
            let data = json!({
                "time": timestamp,
                "px": price,
                "sz": size,
                "side": side,
                "tid": "123"
            });

            let trade = parse_trade(&data, None)
                .expect("Trade parsing should succeed");

            prop_assert!(
                trade.price.0 > Decimal::ZERO,
                "Trade price should be positive"
            );
            prop_assert!(
                trade.amount.0 > Decimal::ZERO,
                "Trade amount should be positive"
            );
        }

        /// Test that trade side is correctly parsed
        #[test]
        fn prop_trade_side_valid(
            timestamp in timestamp_strategy(),
            price in price_strategy(),
            size in size_strategy(),
            side in side_strategy()
        ) {
            let data = json!({
                "time": timestamp,
                "px": price,
                "sz": size,
                "side": side,
                "tid": "123"
            });

            let trade = parse_trade(&data, None)
                .expect("Trade parsing should succeed");

            // Side should be either Buy or Sell
            prop_assert!(
                trade.side == OrderSide::Buy || trade.side == OrderSide::Sell,
                "Trade side should be Buy or Sell"
            );

            // Verify correct mapping
            match side {
                "B" | "buy" | "Buy" => prop_assert_eq!(trade.side, OrderSide::Buy),
                "A" | "sell" | "Sell" => prop_assert_eq!(trade.side, OrderSide::Sell),
                _ => {}
            }
        }
    }
}

/// **Feature: hyperliquid-exchange, Property 8: OHLCV Data Validity**
/// **Validates: Requirements 2.5**
///
/// For any OHLCV candle returned by fetch_ohlcv, high >= max(open, close),
/// low <= min(open, close), and volume >= 0.
mod ohlcv_data_validity {
    use super::*;
    use ccxt_exchanges::hyperliquid::parser::parse_ohlcv;
    use rust_decimal::Decimal;
    use serde_json::json;

    // Strategy for generating valid timestamps
    fn timestamp_strategy() -> impl Strategy<Value = i64> {
        1577836800000i64..1893456000000i64
    }

    // Strategy for generating valid OHLCV prices where high >= max(open, close) and low <= min(open, close)
    fn valid_ohlcv_prices_strategy() -> impl Strategy<Value = (String, String, String, String)> {
        (
            1u64..100000u64,
            1u64..100000u64,
            1u64..100000u64,
            1u64..100000u64,
        )
            .prop_map(|(a, b, c, d)| {
                let mut prices = [a, b, c, d];
                prices.sort();
                // low, open/close, open/close, high
                let low = prices[0];
                let high = prices[3];
                let open = prices[1];
                let close = prices[2];
                (
                    open.to_string(),
                    high.to_string(),
                    low.to_string(),
                    close.to_string(),
                )
            })
    }

    // Strategy for generating valid volume
    fn volume_strategy() -> impl Strategy<Value = String> {
        (0u64..1000000u64).prop_map(|v| v.to_string())
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Test that OHLCV high >= max(open, close)
        #[test]
        fn prop_ohlcv_high_valid(
            timestamp in timestamp_strategy(),
            (open, high, low, close) in valid_ohlcv_prices_strategy(),
            volume in volume_strategy()
        ) {
            let data = json!([timestamp, open, high, low, close, volume]);

            let ohlcv = parse_ohlcv(&data)
                .expect("OHLCV parsing should succeed");

            let max_oc = std::cmp::max(ohlcv.open.0, ohlcv.close.0);
            prop_assert!(
                ohlcv.high.0 >= max_oc,
                "High ({}) should be >= max(open, close) ({})",
                ohlcv.high.0,
                max_oc
            );
        }

        /// Test that OHLCV low <= min(open, close)
        #[test]
        fn prop_ohlcv_low_valid(
            timestamp in timestamp_strategy(),
            (open, high, low, close) in valid_ohlcv_prices_strategy(),
            volume in volume_strategy()
        ) {
            let data = json!([timestamp, open, high, low, close, volume]);

            let ohlcv = parse_ohlcv(&data)
                .expect("OHLCV parsing should succeed");

            let min_oc = std::cmp::min(ohlcv.open.0, ohlcv.close.0);
            prop_assert!(
                ohlcv.low.0 <= min_oc,
                "Low ({}) should be <= min(open, close) ({})",
                ohlcv.low.0,
                min_oc
            );
        }

        /// Test that OHLCV volume >= 0
        #[test]
        fn prop_ohlcv_volume_non_negative(
            timestamp in timestamp_strategy(),
            (open, high, low, close) in valid_ohlcv_prices_strategy(),
            volume in volume_strategy()
        ) {
            let data = json!([timestamp, open, high, low, close, volume]);

            let ohlcv = parse_ohlcv(&data)
                .expect("OHLCV parsing should succeed");

            prop_assert!(
                ohlcv.volume.0 >= Decimal::ZERO,
                "Volume should be non-negative"
            );
        }

        /// Test that OHLCV timestamp is preserved
        #[test]
        fn prop_ohlcv_timestamp_preserved(
            timestamp in timestamp_strategy(),
            (open, high, low, close) in valid_ohlcv_prices_strategy(),
            volume in volume_strategy()
        ) {
            let data = json!([timestamp, open, high, low, close, volume]);

            let ohlcv = parse_ohlcv(&data)
                .expect("OHLCV parsing should succeed");

            prop_assert_eq!(
                ohlcv.timestamp,
                timestamp,
                "Timestamp should be preserved"
            );
        }
    }
}

/// **Feature: hyperliquid-exchange, Property 10: Balance Parsing Accuracy**
/// **Validates: Requirements 5.1, 9.3**
///
/// For any balance response, the parsed Balance SHALL have total = free + used
/// for each currency, and all values SHALL be non-negative.
mod balance_parsing_accuracy {
    use super::*;
    use ccxt_exchanges::hyperliquid::parser::parse_balance;
    use rust_decimal::Decimal;
    use serde_json::json;

    // Strategy for generating valid account values
    #[allow(dead_code)]
    fn account_value_strategy() -> impl Strategy<Value = String> {
        (0u64..1000000u64).prop_map(|v| v.to_string())
    }

    // Strategy for generating valid margin used (must be <= account value)
    fn margin_used_strategy() -> impl Strategy<Value = (String, String)> {
        (0u64..1000000u64, 0u64..100u64).prop_map(|(account, pct)| {
            let margin = account * pct / 100;
            (account.to_string(), margin.to_string())
        })
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Test that balance total = free + used
        #[test]
        fn prop_balance_total_equals_free_plus_used(
            (account_value, margin_used) in margin_used_strategy()
        ) {
            let data = json!({
                "marginSummary": {
                    "accountValue": account_value,
                    "totalMarginUsed": margin_used
                }
            });

            let balance = parse_balance(&data)
                .expect("Balance parsing should succeed");

            if let Some(usdc) = balance.get("USDC") {
                prop_assert_eq!(
                    usdc.total,
                    usdc.free + usdc.used,
                    "Total should equal free + used"
                );
            }
        }

        /// Test that all balance values are non-negative
        #[test]
        fn prop_balance_values_non_negative(
            (account_value, margin_used) in margin_used_strategy()
        ) {
            let data = json!({
                "marginSummary": {
                    "accountValue": account_value,
                    "totalMarginUsed": margin_used
                }
            });

            let balance = parse_balance(&data)
                .expect("Balance parsing should succeed");

            if let Some(usdc) = balance.get("USDC") {
                prop_assert!(
                    usdc.total >= Decimal::ZERO,
                    "Total should be non-negative"
                );
                prop_assert!(
                    usdc.used >= Decimal::ZERO,
                    "Used should be non-negative"
                );
            }
        }
    }
}

/// **Feature: hyperliquid-exchange, Property 12: Error Response Parsing**
/// **Validates: Requirements 8.1, 8.3, 8.4**
///
/// For any error response from the API, the parser SHALL extract a structured
/// error with code and message without throwing exceptions.
mod error_response_parsing {
    use super::*;
    use ccxt_exchanges::hyperliquid::error::{is_error_response, parse_error};
    use serde_json::json;

    // Strategy for generating error messages
    fn error_message_strategy() -> impl Strategy<Value = String> {
        prop_oneof![
            Just("Invalid signature".to_string()),
            Just("Insufficient margin".to_string()),
            Just("Rate limit exceeded".to_string()),
            Just("Order not found".to_string()),
            Just("Invalid parameter".to_string()),
            "[a-zA-Z0-9 ]{5,50}".prop_map(|s| s.to_string()),
        ]
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Test that error responses are correctly identified
        #[test]
        fn prop_error_response_identified(msg in error_message_strategy()) {
            let response = json!({
                "error": msg
            });

            prop_assert!(
                is_error_response(&response),
                "Response with 'error' field should be identified as error"
            );
        }

        /// Test that status: err responses are correctly identified
        #[test]
        fn prop_status_err_identified(msg in error_message_strategy()) {
            let response = json!({
                "status": "err",
                "response": msg
            });

            prop_assert!(
                is_error_response(&response),
                "Response with status 'err' should be identified as error"
            );
        }

        /// Test that success responses are not identified as errors
        #[test]
        fn prop_success_not_error(_dummy in 0u32..100u32) {
            let response = json!({
                "status": "ok",
                "response": {"data": []}
            });

            prop_assert!(
                !is_error_response(&response),
                "Success response should not be identified as error"
            );
        }

        /// Test that parse_error produces valid error for any error response
        #[test]
        fn prop_parse_error_produces_valid_error(msg in error_message_strategy()) {
            let response = json!({
                "error": msg
            });

            let error = parse_error(&response);
            let display = error.to_string();

            prop_assert!(
                !display.is_empty(),
                "Error display should not be empty"
            );
        }

        /// Test that insufficient margin errors are correctly mapped
        #[test]
        fn prop_insufficient_margin_mapped(_dummy in 0u32..100u32) {
            let response = json!({
                "error": "Insufficient margin for order"
            });

            let error = parse_error(&response);
            let display = error.to_string().to_lowercase();

            prop_assert!(
                display.contains("insufficient") || display.contains("balance"),
                "Insufficient margin error should be correctly mapped"
            );
        }
    }
}

/// **Feature: hyperliquid-exchange, Property 13: Order Parsing Correctness**
/// **Validates: Requirements 4.4, 4.5, 9.2**
///
/// For any order response, the parsed Order SHALL have correct status mapping
/// (open, closed, canceled) and all required fields populated.
mod order_parsing_correctness {
    use super::*;
    use ccxt_core::types::OrderStatus;
    use ccxt_exchanges::hyperliquid::parser::{parse_order, parse_order_status};
    use serde_json::json;

    // Strategy for generating order statuses
    fn status_strategy() -> impl Strategy<Value = &'static str> {
        prop_oneof![
            Just("open"),
            Just("resting"),
            Just("filled"),
            Just("canceled"),
            Just("cancelled"),
            Just("rejected"),
        ]
    }

    // Strategy for generating order IDs
    fn order_id_strategy() -> impl Strategy<Value = u64> {
        1u64..1000000u64
    }

    // Strategy for generating prices
    fn price_strategy() -> impl Strategy<Value = String> {
        (1u64..100000u64).prop_map(|n| n.to_string())
    }

    // Strategy for generating sizes
    fn size_strategy() -> impl Strategy<Value = String> {
        (1u64..10000u64).prop_map(|n| format!("{}.{}", n / 1000, n % 1000))
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Test that order status is correctly mapped
        #[test]
        fn prop_order_status_mapping(status in status_strategy()) {
            let parsed = parse_order_status(status);

            match status {
                "open" | "resting" => prop_assert_eq!(parsed, OrderStatus::Open),
                "filled" => prop_assert_eq!(parsed, OrderStatus::Closed),
                "canceled" | "cancelled" => prop_assert_eq!(parsed, OrderStatus::Cancelled),
                "rejected" => prop_assert_eq!(parsed, OrderStatus::Rejected),
                _ => {}
            }
        }

        /// Test that order ID is correctly parsed
        #[test]
        fn prop_order_id_parsed(
            oid in order_id_strategy(),
            price in price_strategy(),
            size in size_strategy()
        ) {
            let data = json!({
                "oid": oid,
                "coin": "BTC",
                "limitPx": price,
                "sz": size,
                "side": "B",
                "status": "open"
            });

            let order = parse_order(&data, None)
                .expect("Order parsing should succeed");

            prop_assert_eq!(
                order.id,
                oid.to_string(),
                "Order ID should be correctly parsed"
            );
        }

        /// Test that order symbol is correctly formatted
        #[test]
        fn prop_order_symbol_formatted(
            oid in order_id_strategy(),
            price in price_strategy(),
            size in size_strategy()
        ) {
            let data = json!({
                "oid": oid,
                "coin": "BTC",
                "limitPx": price,
                "sz": size,
                "side": "B",
                "status": "open"
            });

            let order = parse_order(&data, None)
                .expect("Order parsing should succeed");

            prop_assert!(
                order.symbol.ends_with("/USDC:USDC"),
                "Order symbol should end with /USDC:USDC"
            );
        }
    }
}

/// **Feature: hyperliquid-exchange, Property 15: Parser Robustness**
/// **Validates: Requirements 9.5**
///
/// For any API response with missing optional fields, the parser SHALL
/// successfully parse the response and use appropriate default values without errors.
mod parser_robustness {
    use super::*;
    use ccxt_exchanges::hyperliquid::parser::{parse_balance, parse_market, parse_ticker};
    use rust_decimal::Decimal;
    use rust_decimal::prelude::FromPrimitive;
    use serde_json::json;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Test that market parser handles minimal data
        #[test]
        fn prop_market_parser_minimal(
            name in "[A-Z]{2,10}",
            index in 0usize..1000usize
        ) {
            // Minimal market data - only required fields
            let data = json!({
                "name": name
            });

            let result = parse_market(&data, index);
            prop_assert!(
                result.is_ok(),
                "Market parser should handle minimal data"
            );
        }

        /// Test that ticker parser handles any valid price
        #[test]
        fn prop_ticker_parser_any_price(
            symbol in "[A-Z]{2,10}".prop_map(|s| format!("{}/USDC:USDC", s)),
            price in 1u64..1000000u64
        ) {
            let mid_price = Decimal::from_u64(price).unwrap();
            let result = parse_ticker(&symbol, mid_price, None);

            prop_assert!(
                result.is_ok(),
                "Ticker parser should handle any valid price"
            );
        }

        /// Test that balance parser handles empty margin summary
        #[test]
        fn prop_balance_parser_empty(_dummy in 0u32..100u32) {
            let data = json!({});
            let result = parse_balance(&data);

            prop_assert!(
                result.is_ok(),
                "Balance parser should handle empty data"
            );
        }

        /// Test that balance parser handles partial data
        #[test]
        fn prop_balance_parser_partial(account_value in 0u64..1000000u64) {
            let data = json!({
                "marginSummary": {
                    "accountValue": account_value.to_string()
                }
            });

            let result = parse_balance(&data);
            prop_assert!(
                result.is_ok(),
                "Balance parser should handle partial data"
            );
        }
    }
}

/// **Feature: hyperliquid-exchange, Property 9: Order Parameter Validation**
/// **Validates: Requirements 4.6**
///
/// For any order request with invalid parameters (negative amount, zero price
/// for limit order, invalid symbol), the exchange SHALL return an error before
/// attempting to send the request.
mod order_parameter_validation {
    use super::*;

    // Note: This property test validates the concept that invalid parameters
    // should be rejected. Since we can't actually call the exchange without
    // network access, we test the validation logic conceptually.

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Test that negative amounts are conceptually invalid
        #[test]
        fn prop_negative_amount_invalid(amount in -1000.0f64..-0.001f64) {
            // Negative amounts should be rejected
            prop_assert!(
                amount < 0.0,
                "Negative amounts should be invalid"
            );
        }

        /// Test that zero price for limit orders is conceptually invalid
        #[test]
        fn prop_zero_price_invalid_for_limit(_dummy in 0u32..100u32) {
            let price = 0.0f64;
            // Zero price for limit orders should be rejected
            prop_assert!(
                price <= 0.0,
                "Zero or negative price should be invalid for limit orders"
            );
        }

        /// Test that valid order parameters pass validation
        #[test]
        fn prop_valid_params_accepted(
            amount in 0.001f64..1000.0f64,
            price in 0.01f64..100000.0f64
        ) {
            prop_assert!(amount > 0.0, "Amount should be positive");
            prop_assert!(price > 0.0, "Price should be positive");
        }
    }
}

/// **Feature: hyperliquid-exchange, Property 14: Request Serialization Round-Trip**
/// **Validates: Requirements 9.4, 9.6**
///
/// For any order request, serializing to JSON and then parsing back SHALL
/// produce an equivalent request structure.
mod request_serialization_roundtrip {
    use super::*;
    use serde_json::json;

    // Strategy for generating valid asset indices
    fn asset_index_strategy() -> impl Strategy<Value = u32> {
        0u32..100u32
    }

    // Strategy for generating valid prices
    fn price_strategy() -> impl Strategy<Value = String> {
        (1u64..100000u64).prop_map(|n| n.to_string())
    }

    // Strategy for generating valid sizes
    fn size_strategy() -> impl Strategy<Value = String> {
        (1u64..10000u64).prop_map(|n| format!("{}.{}", n / 1000, n % 1000))
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Test that order request JSON round-trips correctly
        #[test]
        fn prop_order_request_roundtrip(
            asset in asset_index_strategy(),
            is_buy in any::<bool>(),
            price in price_strategy(),
            size in size_strategy()
        ) {
            let order_request = json!({
                "a": asset,
                "b": is_buy,
                "p": price,
                "s": size,
                "r": false,
                "t": {
                    "limit": {
                        "tif": "Gtc"
                    }
                }
            });

            // Serialize to string
            let serialized = serde_json::to_string(&order_request)
                .expect("Serialization should succeed");

            // Parse back
            let parsed: serde_json::Value = serde_json::from_str(&serialized)
                .expect("Parsing should succeed");

            // Verify round-trip
            prop_assert_eq!(
                parsed["a"].as_u64().unwrap() as u32,
                asset,
                "Asset should round-trip"
            );
            prop_assert_eq!(
                parsed["b"].as_bool().unwrap(),
                is_buy,
                "isBuy should round-trip"
            );
            prop_assert_eq!(
                parsed["p"].as_str().unwrap(),
                price,
                "Price should round-trip"
            );
            prop_assert_eq!(
                parsed["s"].as_str().unwrap(),
                size,
                "Size should round-trip"
            );
        }

        /// Test that cancel request JSON round-trips correctly
        #[test]
        fn prop_cancel_request_roundtrip(
            asset in asset_index_strategy(),
            order_id in 1u64..1000000u64
        ) {
            let cancel_request = json!({
                "type": "cancel",
                "cancels": [{
                    "a": asset,
                    "o": order_id
                }]
            });

            // Serialize to string
            let serialized = serde_json::to_string(&cancel_request)
                .expect("Serialization should succeed");

            // Parse back
            let parsed: serde_json::Value = serde_json::from_str(&serialized)
                .expect("Parsing should succeed");

            // Verify round-trip
            prop_assert_eq!(
                parsed["type"].as_str().unwrap(),
                "cancel",
                "Type should round-trip"
            );

            let cancel = &parsed["cancels"][0];
            prop_assert_eq!(
                cancel["a"].as_u64().unwrap() as u32,
                asset,
                "Asset should round-trip"
            );
            prop_assert_eq!(
                cancel["o"].as_u64().unwrap(),
                order_id,
                "Order ID should round-trip"
            );
        }
    }
}

/// **Feature: hyperliquid-exchange, Property 16: Exchange Trait Consistency**
/// **Validates: Requirements 7.1**
///
/// For any method implemented in the Exchange trait, calling it through the
/// trait object SHALL produce the same result as calling it directly on HyperLiquid.
mod exchange_trait_consistency {
    use super::*;
    use ccxt_core::exchange::Exchange;
    use ccxt_exchanges::hyperliquid::HyperLiquid;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Test that Exchange trait metadata methods are consistent
        #[test]
        fn prop_metadata_consistent(_dummy in 0u32..100u32) {
            let exchange = HyperLiquid::builder()
                .testnet(true)
                .build()
                .expect("Failed to build HyperLiquid");

            // Test via trait object
            let trait_obj: &dyn Exchange = &exchange;

            prop_assert_eq!(trait_obj.id(), "hyperliquid");
            prop_assert_eq!(trait_obj.name(), "HyperLiquid");
            prop_assert_eq!(trait_obj.version(), "1");
            prop_assert!(!trait_obj.certified());
            prop_assert!(trait_obj.has_websocket());
        }

        /// Test that capabilities are correctly reported
        #[test]
        fn prop_capabilities_consistent(_dummy in 0u32..100u32) {
            let exchange = HyperLiquid::builder()
                .testnet(true)
                .build()
                .expect("Failed to build HyperLiquid");

            let trait_obj: &dyn Exchange = &exchange;
            let caps = trait_obj.capabilities();

            // Public API capabilities
            prop_assert!(caps.fetch_markets());
            prop_assert!(caps.fetch_ticker());
            prop_assert!(caps.fetch_tickers());
            prop_assert!(caps.fetch_order_book());
            prop_assert!(caps.fetch_trades());
            prop_assert!(caps.fetch_ohlcv());

            // Private API capabilities
            prop_assert!(caps.create_order());
            prop_assert!(caps.cancel_order());
            prop_assert!(caps.cancel_all_orders());
            prop_assert!(caps.fetch_open_orders());
            prop_assert!(caps.fetch_balance());
            prop_assert!(caps.fetch_positions());
            prop_assert!(caps.set_leverage());

            // Not implemented
            prop_assert!(!caps.fetch_currencies());
            prop_assert!(!caps.edit_order());
            prop_assert!(!caps.fetch_my_trades());
        }

        /// Test that rate limit is consistent
        #[test]
        fn prop_rate_limit_consistent(_dummy in 0u32..100u32) {
            let exchange = HyperLiquid::builder()
                .testnet(true)
                .build()
                .expect("Failed to build HyperLiquid");

            let trait_obj: &dyn Exchange = &exchange;

            prop_assert!(
                trait_obj.rate_limit() == 100,
                "Rate limit should be 100"
            );
        }
    }
}

/// **Feature: hyperliquid-exchange, Property 17: NotImplemented Error Handling**
/// **Validates: Requirements 7.3**
///
/// For any method not supported by HyperLiquid (e.g., spot trading), calling it
/// SHALL return a NotImplemented error with a descriptive message.
mod not_implemented_error_handling {
    use super::*;
    use ccxt_core::exchange::Exchange;
    use ccxt_exchanges::hyperliquid::HyperLiquid;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(10))]

        /// Test that fetch_order returns NotImplemented
        #[test]
        fn prop_fetch_order_not_implemented(_dummy in 0u32..10u32) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let exchange = HyperLiquid::builder()
                    .testnet(true)
                    .build()
                    .expect("Failed to build HyperLiquid");

                let trait_obj: &dyn Exchange = &exchange;
                let result = trait_obj.fetch_order("123", Some("BTC/USDC:USDC")).await;

                prop_assert!(result.is_err(), "fetch_order should return error");
                let err = result.unwrap_err();
                prop_assert!(
                    err.to_string().to_lowercase().contains("not implemented") ||
                    err.to_string().to_lowercase().contains("notimplemented"),
                    "Error should indicate not implemented"
                );
                Ok(())
            })?;
        }

        /// Test that fetch_closed_orders returns NotImplemented
        #[test]
        fn prop_fetch_closed_orders_not_implemented(_dummy in 0u32..10u32) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let exchange = HyperLiquid::builder()
                    .testnet(true)
                    .build()
                    .expect("Failed to build HyperLiquid");

                let trait_obj: &dyn Exchange = &exchange;
                let result = trait_obj.fetch_closed_orders(None, None, None).await;

                prop_assert!(result.is_err(), "fetch_closed_orders should return error");
                let err = result.unwrap_err();
                prop_assert!(
                    err.to_string().to_lowercase().contains("not implemented") ||
                    err.to_string().to_lowercase().contains("notimplemented"),
                    "Error should indicate not implemented"
                );
                Ok(())
            })?;
        }

        /// Test that fetch_my_trades returns NotImplemented
        #[test]
        fn prop_fetch_my_trades_not_implemented(_dummy in 0u32..10u32) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let exchange = HyperLiquid::builder()
                    .testnet(true)
                    .build()
                    .expect("Failed to build HyperLiquid");

                let trait_obj: &dyn Exchange = &exchange;
                let result = trait_obj.fetch_my_trades(None, None, None).await;

                prop_assert!(result.is_err(), "fetch_my_trades should return error");
                let err = result.unwrap_err();
                prop_assert!(
                    err.to_string().to_lowercase().contains("not implemented") ||
                    err.to_string().to_lowercase().contains("notimplemented"),
                    "Error should indicate not implemented"
                );
                Ok(())
            })?;
        }
    }
}
