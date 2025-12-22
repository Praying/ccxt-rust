//! Bybit Integration Tests
//!
//! These tests verify the Bybit exchange implementation against the real API.
//! They can be run with: cargo test --test bybit_integration_test
//!
//! Note: Tests are marked with #[ignore] to avoid hitting API rate limits
//! during normal test runs. Run them explicitly with:
//! cargo test --test bybit_integration_test -- --ignored
//!
//! _Requirements: 4.1, 4.3, 5.1_

use ccxt_core::ExchangeConfig;
use ccxt_exchanges::bybit::{Bybit, BybitBuilder};
use std::env;

/// Load API credentials from environment variables.
fn get_api_credentials() -> (Option<String>, Option<String>) {
    dotenvy::dotenv().ok();
    let api_key = env::var("BYBIT_API_KEY").ok();
    let secret = env::var("BYBIT_SECRET").ok();
    (api_key, secret)
}

// ==================== Instance Creation Tests ====================

/// Test creating a new Bybit instance with default configuration.
#[test]
fn test_new_bybit_instance() {
    let config = ExchangeConfig {
        id: "bybit".to_string(),
        name: "Bybit".to_string(),
        ..Default::default()
    };

    let exchange = Bybit::new(config).unwrap();
    assert_eq!(exchange.id(), "bybit");
    assert_eq!(exchange.name(), "Bybit");
    assert_eq!(exchange.version(), "v5");
}

/// Test creating Bybit instance using builder pattern.
#[test]
fn test_bybit_builder() {
    let exchange = BybitBuilder::new()
        .testnet(false)
        .build()
        .expect("Failed to build Bybit");

    assert_eq!(exchange.id(), "bybit");
    assert_eq!(exchange.name(), "Bybit");
    assert!(!exchange.options().testnet);
}

/// Test creating Bybit instance with testnet mode.
#[test]
fn test_bybit_testnet_mode() {
    let exchange = BybitBuilder::new()
        .testnet(true)
        .build()
        .expect("Failed to build Bybit");

    assert!(exchange.options().testnet);
    let urls = exchange.urls();
    assert!(urls.rest.contains("api-testnet.bybit.com"));
    assert!(urls.ws_public.contains("stream-testnet.bybit.com"));
}

/// Test Bybit URLs configuration.
#[test]
fn test_bybit_urls() {
    let exchange = BybitBuilder::new().build().unwrap();
    let urls = exchange.urls();

    assert!(urls.rest.contains("api.bybit.com"));
    assert!(urls.ws_public.contains("stream.bybit.com"));
    assert!(urls.ws_private.contains("stream.bybit.com"));
}

/// Test Bybit timeframes.
#[test]
fn test_bybit_timeframes() {
    let exchange = BybitBuilder::new().build().unwrap();
    let timeframes = exchange.timeframes();

    assert!(timeframes.contains_key("1m"));
    assert!(timeframes.contains_key("5m"));
    assert!(timeframes.contains_key("1h"));
    assert!(timeframes.contains_key("1d"));
    assert_eq!(timeframes.len(), 13);
}

/// Test Bybit account type configuration.
#[test]
fn test_bybit_account_type() {
    let exchange = BybitBuilder::new()
        .account_type("SPOT")
        .build()
        .expect("Failed to build Bybit");

    assert_eq!(exchange.options().account_type, "SPOT");
}

/// Test Bybit recv_window configuration.
#[test]
fn test_bybit_recv_window() {
    let exchange = BybitBuilder::new()
        .recv_window(10000)
        .build()
        .expect("Failed to build Bybit");

    assert_eq!(exchange.options().recv_window, 10000);
}

// ==================== Public API Tests ====================

/// Test fetching all available markets from the real API.
/// _Requirements: 4.1_
#[tokio::test]
#[ignore] // Requires network access
async fn test_fetch_markets() {
    let exchange = BybitBuilder::new().build().expect("Failed to build Bybit");
    let result = exchange.fetch_markets().await;

    assert!(
        result.is_ok(),
        "Failed to fetch markets: {:?}",
        result.err()
    );
    let markets = result.unwrap();

    assert!(
        markets.len() > 10,
        "Expected more than 10 markets, got {}",
        markets.len()
    );

    // Check for common trading pair
    let btc_usdt = markets.values().find(|m| m.symbol == "BTC/USDT");
    assert!(btc_usdt.is_some(), "BTC/USDT market not found");

    if let Some(market) = btc_usdt {
        assert_eq!(market.base, "BTC");
        assert_eq!(market.quote, "USDT");
        assert!(market.active);
    }
}

/// Test fetching ticker data for BTC/USDT from the real API.
/// _Requirements: 4.3_
#[tokio::test]
#[ignore] // Requires network access
async fn test_fetch_ticker() {
    let exchange = BybitBuilder::new().build().expect("Failed to build Bybit");

    // First load markets
    exchange
        .load_markets(false)
        .await
        .expect("Failed to load markets");

    let result = exchange.fetch_ticker("BTC/USDT").await;

    assert!(result.is_ok(), "Failed to fetch ticker: {:?}", result.err());
    let ticker = result.unwrap();

    assert_eq!(ticker.symbol, "BTC/USDT");
    assert!(ticker.last.is_some(), "Last price should be present");
    assert!(ticker.bid.is_some(), "Bid price should be present");
    assert!(ticker.ask.is_some(), "Ask price should be present");
    assert!(ticker.high.is_some(), "High price should be present");
    assert!(ticker.low.is_some(), "Low price should be present");
}

/// Test fetching order book data from the real API.
/// _Requirements: 5.1_
#[tokio::test]
#[ignore] // Requires network access
async fn test_fetch_order_book() {
    let exchange = BybitBuilder::new().build().expect("Failed to build Bybit");

    // First load markets
    exchange
        .load_markets(false)
        .await
        .expect("Failed to load markets");

    let result = exchange.fetch_order_book("BTC/USDT", Some(10)).await;

    assert!(
        result.is_ok(),
        "Failed to fetch order book: {:?}",
        result.err()
    );
    let order_book = result.unwrap();

    assert_eq!(order_book.symbol, "BTC/USDT");
    assert!(!order_book.bids.is_empty(), "Bids should not be empty");
    assert!(!order_book.asks.is_empty(), "Asks should not be empty");
    assert!(order_book.bids.len() <= 10, "Should have at most 10 bids");
    assert!(order_book.asks.len() <= 10, "Should have at most 10 asks");

    // Verify sorting: bids descending, asks ascending
    for i in 1..order_book.bids.len() {
        assert!(
            order_book.bids[i - 1].price >= order_book.bids[i].price,
            "Bids should be sorted descending"
        );
    }
    for i in 1..order_book.asks.len() {
        assert!(
            order_book.asks[i - 1].price <= order_book.asks[i].price,
            "Asks should be sorted ascending"
        );
    }

    if let (Some(best_bid), Some(best_ask)) = (order_book.bids.first(), order_book.asks.first()) {
        assert!(
            best_bid.price < best_ask.price,
            "Best bid ({}) should be less than best ask ({})",
            best_bid.price,
            best_ask.price
        );
    }
}

/// Test fetching recent trades from the real API.
#[tokio::test]
#[ignore] // Requires network access
async fn test_fetch_trades() {
    let exchange = BybitBuilder::new().build().expect("Failed to build Bybit");

    // First load markets
    exchange
        .load_markets(false)
        .await
        .expect("Failed to load markets");

    let result = exchange.fetch_trades("BTC/USDT", Some(5)).await;

    assert!(result.is_ok(), "Failed to fetch trades: {:?}", result.err());
    let trades = result.unwrap();

    assert!(!trades.is_empty(), "Trades should not be empty");
    assert!(trades.len() <= 5, "Should have at most 5 trades");

    for trade in &trades {
        assert_eq!(trade.symbol, "BTC/USDT");
        assert!(
            trade.price.0 > rust_decimal::Decimal::ZERO,
            "Trade price should be positive"
        );
        assert!(
            trade.amount.0 > rust_decimal::Decimal::ZERO,
            "Trade amount should be positive"
        );
        assert!(trade.timestamp > 0, "Trade should have timestamp");
    }
}

/// Test fetching OHLCV (candlestick) data from the real API.
#[tokio::test]
#[ignore] // Requires network access
async fn test_fetch_ohlcv() {
    let exchange = BybitBuilder::new().build().expect("Failed to build Bybit");

    // First load markets
    exchange
        .load_markets(false)
        .await
        .expect("Failed to load markets");

    let result = exchange.fetch_ohlcv("BTC/USDT", "1h", None, Some(5)).await;

    assert!(result.is_ok(), "Failed to fetch OHLCV: {:?}", result.err());
    let candles = result.unwrap();

    assert!(!candles.is_empty(), "Candles should not be empty");
    assert!(candles.len() <= 5, "Should have at most 5 candles");

    for candle in &candles {
        assert!(candle.open > 0.0, "Open price should be positive");
        assert!(candle.high >= candle.low, "High should be >= low");
        assert!(candle.high >= candle.open, "High should be >= open");
        assert!(candle.high >= candle.close, "High should be >= close");
        assert!(candle.low <= candle.open, "Low should be <= open");
        assert!(candle.low <= candle.close, "Low should be <= close");
        assert!(candle.volume >= 0.0, "Volume should be non-negative");
    }
}

/// Test error handling for invalid trading symbols.
#[tokio::test]
#[ignore] // Requires network access
async fn test_invalid_symbol() {
    let exchange = BybitBuilder::new().build().expect("Failed to build Bybit");

    // First load markets
    exchange
        .load_markets(false)
        .await
        .expect("Failed to load markets");

    let result = exchange.fetch_ticker("INVALID/SYMBOL").await;

    assert!(result.is_err(), "Should fail for invalid symbol");
}

// ==================== Private API Tests ====================

/// Test fetching account balance.
///
/// Note: Requires API credentials.
#[tokio::test]
#[ignore] // Requires API credentials
async fn test_fetch_balance() {
    let (api_key, secret) = get_api_credentials();

    if api_key.is_none() || secret.is_none() {
        println!("⚠️  Skip test: API credentials not set");
        return;
    }

    let exchange = BybitBuilder::new()
        .api_key(api_key.unwrap())
        .secret(secret.unwrap())
        .build()
        .expect("Failed to build Bybit");

    let result = exchange.fetch_balance().await;

    assert!(
        result.is_ok(),
        "Failed to fetch balance: {:?}",
        result.err()
    );
}

/// Test fetching open orders.
///
/// Note: Requires API credentials.
#[tokio::test]
#[ignore] // Requires API credentials
async fn test_fetch_open_orders() {
    let (api_key, secret) = get_api_credentials();

    if api_key.is_none() || secret.is_none() {
        println!("⚠️  Skip test: API credentials not set");
        return;
    }

    let exchange = BybitBuilder::new()
        .api_key(api_key.unwrap())
        .secret(secret.unwrap())
        .build()
        .expect("Failed to build Bybit");

    // First load markets
    exchange
        .load_markets(false)
        .await
        .expect("Failed to load markets");

    let result = exchange
        .fetch_open_orders(Some("BTC/USDT"), None, None)
        .await;

    assert!(
        result.is_ok(),
        "Failed to fetch open orders: {:?}",
        result.err()
    );
}

/// Test that operations requiring credentials fail without API keys.
#[tokio::test]
#[ignore] // Requires network access
async fn test_check_required_credentials_without_keys() {
    let exchange = BybitBuilder::new().build().expect("Failed to build Bybit");

    let result = exchange.fetch_balance().await;
    assert!(result.is_err(), "Should fail without API credentials");
}

/// Test invalid API key handling.
#[tokio::test]
#[ignore] // Requires network access
async fn test_invalid_api_key() {
    let exchange = BybitBuilder::new()
        .api_key("invalid_key")
        .secret("invalid_secret")
        .build()
        .expect("Failed to build Bybit");

    let result = exchange.fetch_balance().await;
    assert!(result.is_err(), "Should fail with invalid API key");
}
