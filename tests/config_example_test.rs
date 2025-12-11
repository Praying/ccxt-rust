// Integration test example demonstrating TestConfig usage
//
// This test shows how to:
// - Load test configuration from environment
// - Use skip_if! and require_credentials! macros
// - Load test fixtures from JSON files
// - Write integration tests for both public and private APIs

// Allow clippy warnings for test code
#![allow(clippy::disallowed_methods)]

use ccxt_core::test_config::TestConfig;
use serde_json::Value;
use std::fs;

#[test]
fn test_load_config_from_env() {
    // Load configuration from environment variables
    // This will use .env file if present, otherwise use defaults
    let config = TestConfig::from_env().expect("Failed to load test config");

    // Verify configuration structure
    assert!(config.test_timeout_ms > 0);
    assert!(!config.test_data.fixtures_dir.is_empty());

    println!("✓ Config loaded successfully");
    println!("  - Timeout: {}ms", config.test_timeout_ms);
    println!("  - Fixtures dir: {}", config.test_data.fixtures_dir);
    println!("  - Skip private tests: {}", config.skip_private_tests);
}

#[test]
fn test_skip_if_macro() {
    let config = TestConfig::from_env().expect("Failed to load test config");

    // Example: Skip test if private tests are disabled
    ccxt_core::skip_if!(
        config,
        config.should_skip_private_tests(),
        "Private tests are disabled"
    );

    // This code only runs if private tests are enabled
    println!("✓ Private tests are enabled, test continues...");
}

#[test]
fn test_has_credentials() {
    let config = TestConfig::from_env().expect("Failed to load test config");

    // Check if we have Binance credentials
    let has_binance = config.has_binance_credentials();
    println!("✓ Has Binance credentials: {}", has_binance);

    // Check if we have OKX credentials
    let has_okx = config.has_okx_credentials();
    println!("✓ Has OKX credentials: {}", has_okx);

    // Get active API key for Binance (respects testnet setting)
    if let Some((api_key, _secret)) = config.get_active_api_key("binance") {
        println!("✓ Active Binance API key: {}...", &api_key[..8]);
    }
}

#[test]
fn test_load_ticker_fixture() {
    let config = TestConfig::from_env().expect("Failed to load test config");

    // Get path to ticker fixture
    let fixture_path = config.get_fixture_path("tickers", "binance_btcusdt.json");

    // Load and parse fixture
    let fixture_content = fs::read_to_string(&fixture_path).expect("Failed to read ticker fixture");

    let ticker: Value =
        serde_json::from_str(&fixture_content).expect("Failed to parse ticker JSON");

    // Verify fixture structure
    assert_eq!(ticker["symbol"].as_str().unwrap(), "BTC/USDT");
    assert!(ticker["last"].as_f64().is_some());
    assert!(ticker["bid"].as_f64().is_some());
    assert!(ticker["ask"].as_f64().is_some());

    println!("✓ Ticker fixture loaded successfully");
    println!("  - Symbol: {}", ticker["symbol"]);
    println!("  - Last price: {}", ticker["last"]);
}

#[test]
fn test_load_orderbook_fixture() {
    let config = TestConfig::from_env().expect("Failed to load test config");

    let fixture_path = config.get_fixture_path("orderbooks", "binance_btcusdt_orderbook.json");
    let fixture_content =
        fs::read_to_string(&fixture_path).expect("Failed to read orderbook fixture");

    let orderbook: Value =
        serde_json::from_str(&fixture_content).expect("Failed to parse orderbook JSON");

    // Verify orderbook structure
    assert_eq!(orderbook["symbol"].as_str().unwrap(), "BTC/USDT");
    assert!(orderbook["bids"].as_array().is_some());
    assert!(orderbook["asks"].as_array().is_some());

    let bids = orderbook["bids"].as_array().unwrap();
    let asks = orderbook["asks"].as_array().unwrap();

    assert!(!bids.is_empty());
    assert!(!asks.is_empty());

    // Verify price ordering
    let best_bid = bids[0][0].as_f64().unwrap();
    let best_ask = asks[0][0].as_f64().unwrap();
    assert!(
        best_bid < best_ask,
        "Best bid should be lower than best ask"
    );

    println!("✓ Orderbook fixture loaded successfully");
    println!("  - Bids: {}", bids.len());
    println!("  - Asks: {}", asks.len());
    println!("  - Spread: {:.2}", best_ask - best_bid);
}

#[test]
fn test_load_trades_fixture() {
    let config = TestConfig::from_env().expect("Failed to load test config");

    let fixture_path = config.get_fixture_path("trades", "binance_btcusdt_trades.json");
    let fixture_content = fs::read_to_string(&fixture_path).expect("Failed to read trades fixture");

    let trades: Value =
        serde_json::from_str(&fixture_content).expect("Failed to parse trades JSON");

    let trades_array = trades.as_array().expect("Trades should be an array");
    assert!(!trades_array.is_empty());

    // Verify first trade structure
    let first_trade = &trades_array[0];
    assert!(first_trade["id"].as_str().is_some());
    assert!(first_trade["symbol"].as_str().is_some());
    assert!(first_trade["price"].as_f64().is_some());
    assert!(first_trade["amount"].as_f64().is_some());
    assert!(first_trade["side"].as_str().is_some());

    println!("✓ Trades fixture loaded successfully");
    println!("  - Trade count: {}", trades_array.len());
}

#[test]
fn test_load_markets_fixture() {
    let config = TestConfig::from_env().expect("Failed to load test config");

    let fixture_path = config.get_fixture_path("markets", "binance_markets.json");
    let fixture_content =
        fs::read_to_string(&fixture_path).expect("Failed to read markets fixture");

    let markets: Value =
        serde_json::from_str(&fixture_content).expect("Failed to parse markets JSON");

    let markets_array = markets.as_array().expect("Markets should be an array");
    assert!(!markets_array.is_empty());

    // Verify market structure
    let btc_market = markets_array
        .iter()
        .find(|m| m["symbol"].as_str() == Some("BTC/USDT"))
        .expect("BTC/USDT market should exist");

    assert_eq!(btc_market["base"].as_str().unwrap(), "BTC");
    assert_eq!(btc_market["quote"].as_str().unwrap(), "USDT");
    assert!(btc_market["active"].as_bool().unwrap());
    assert!(btc_market["precision"].is_object());
    assert!(btc_market["limits"].is_object());

    println!("✓ Markets fixture loaded successfully");
    println!("  - Market count: {}", markets_array.len());
}

#[test]
fn test_load_balance_fixture() {
    let config = TestConfig::from_env().expect("Failed to load test config");

    let fixture_path = config.get_fixture_path("balances", "binance_balance.json");
    let fixture_content =
        fs::read_to_string(&fixture_path).expect("Failed to read balance fixture");

    let balance: Value =
        serde_json::from_str(&fixture_content).expect("Failed to parse balance JSON");

    // Verify balance structure
    assert!(balance["free"].is_object());
    assert!(balance["used"].is_object());
    assert!(balance["total"].is_object());

    let free = balance["free"].as_object().unwrap();
    assert!(free.contains_key("BTC"));
    assert!(free.contains_key("USDT"));

    println!("✓ Balance fixture loaded successfully");
    println!("  - Assets: {}", free.len());
}

// Example of a test that requires credentials
#[test]
fn test_with_binance_credentials() {
    let config = TestConfig::from_env().expect("Failed to load test config");

    // This will skip the test if no Binance credentials are available
    ccxt_core::require_credentials!(config, binance);

    // At this point, we know we have Binance credentials
    let (api_key, api_secret) = config
        .get_active_api_key("binance")
        .expect("Should have Binance credentials");

    println!("✓ Test with Binance credentials");
    println!("  - API key length: {}", api_key.len());
    println!("  - Secret length: {}", api_secret.len());
    println!("  - Using testnet: {}", config.binance.use_testnet);
}
