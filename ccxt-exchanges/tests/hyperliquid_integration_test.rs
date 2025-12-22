//! Integration tests for HyperLiquid exchange.
//!
//! These tests require network access and use the HyperLiquid testnet.
//! Run with: cargo test --test hyperliquid_integration_test -- --ignored

use ccxt_core::exchange::Exchange;
use ccxt_exchanges::hyperliquid::{HyperLiquid, HyperLiquidBuilder};

/// Test creating a HyperLiquid instance without authentication.
#[test]
fn test_create_public_instance() {
    let exchange = HyperLiquidBuilder::new()
        .testnet(true)
        .build()
        .expect("Failed to build HyperLiquid");

    assert_eq!(exchange.id(), "hyperliquid");
    assert_eq!(exchange.name(), "HyperLiquid");
    assert!(exchange.options().testnet);
    assert!(exchange.auth().is_none());
}

/// Test creating a HyperLiquid instance with authentication.
#[test]
fn test_create_authenticated_instance() {
    // Test private key (DO NOT USE IN PRODUCTION)
    let test_key = "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80";

    let exchange = HyperLiquidBuilder::new()
        .private_key(test_key)
        .testnet(true)
        .build()
        .expect("Failed to build HyperLiquid");

    assert!(exchange.auth().is_some());
    assert!(exchange.wallet_address().is_some());
}

/// Test that invalid private key is rejected.
#[test]
fn test_invalid_private_key_rejected() {
    let result = HyperLiquidBuilder::new()
        .private_key("invalid_key")
        .testnet(true)
        .build();

    assert!(result.is_err());
}

/// Test Exchange trait capabilities.
#[test]
fn test_exchange_capabilities() {
    let exchange = HyperLiquidBuilder::new()
        .testnet(true)
        .build()
        .expect("Failed to build HyperLiquid");

    let caps = exchange.capabilities();

    // Public API
    assert!(caps.fetch_markets());
    assert!(caps.fetch_ticker());
    assert!(caps.fetch_tickers());
    assert!(caps.fetch_order_book());
    assert!(caps.fetch_trades());
    assert!(caps.fetch_ohlcv());

    // Private API
    assert!(caps.create_order());
    assert!(caps.cancel_order());
    assert!(caps.fetch_open_orders());
    assert!(caps.fetch_balance());
    assert!(caps.fetch_positions());
    assert!(caps.set_leverage());

    // Not supported
    assert!(!caps.fetch_currencies());
    assert!(!caps.fetch_my_trades());
}

/// Test fetching markets from testnet.
#[tokio::test]
#[ignore] // Requires network access
async fn test_fetch_markets() {
    let exchange = HyperLiquidBuilder::new()
        .testnet(true)
        .build()
        .expect("Failed to build HyperLiquid");

    let markets = exchange
        .fetch_markets()
        .await
        .expect("Failed to fetch markets");

    assert!(!markets.is_empty(), "Should have at least one market");

    for market in markets.values() {
        // All HyperLiquid markets should be perpetual contracts
        assert!(
            market.symbol.ends_with("/USDC:USDC"),
            "Symbol should end with /USDC:USDC"
        );
        assert!(market.active, "Market should be active");
        assert!(market.margin, "Market should support margin");
        assert_eq!(market.contract, Some(true), "Market should be a contract");
    }
}

/// Test fetching ticker from testnet.
#[tokio::test]
#[ignore] // Requires network access
async fn test_fetch_ticker() {
    let exchange = HyperLiquidBuilder::new()
        .testnet(true)
        .build()
        .expect("Failed to build HyperLiquid");

    // First load markets
    exchange
        .load_markets(false)
        .await
        .expect("Failed to load markets");

    // Fetch BTC ticker
    let ticker = exchange
        .fetch_ticker("BTC/USDC:USDC")
        .await
        .expect("Failed to fetch ticker");

    assert_eq!(ticker.symbol, "BTC/USDC:USDC");
    assert!(ticker.last.is_some(), "Ticker should have last price");
    assert!(ticker.timestamp > 0, "Ticker should have valid timestamp");
}

/// Test fetching order book from testnet.
#[tokio::test]
#[ignore] // Requires network access
async fn test_fetch_order_book() {
    let exchange = HyperLiquidBuilder::new()
        .testnet(true)
        .build()
        .expect("Failed to build HyperLiquid");

    // First load markets
    exchange
        .load_markets(false)
        .await
        .expect("Failed to load markets");

    // Fetch BTC order book
    let orderbook = exchange
        .fetch_order_book("BTC/USDC:USDC", None)
        .await
        .expect("Failed to fetch order book");

    assert_eq!(orderbook.symbol, "BTC/USDC:USDC");
    assert!(
        !orderbook.bids.is_empty() || !orderbook.asks.is_empty(),
        "Order book should have entries"
    );

    // Verify sorting
    for i in 1..orderbook.bids.len() {
        assert!(
            orderbook.bids[i - 1].price >= orderbook.bids[i].price,
            "Bids should be sorted descending"
        );
    }
    for i in 1..orderbook.asks.len() {
        assert!(
            orderbook.asks[i - 1].price <= orderbook.asks[i].price,
            "Asks should be sorted ascending"
        );
    }
}

/// Test fetching trades from testnet.
#[tokio::test]
#[ignore] // Requires network access
async fn test_fetch_trades() {
    let exchange = HyperLiquidBuilder::new()
        .testnet(true)
        .build()
        .expect("Failed to build HyperLiquid");

    // First load markets
    exchange
        .load_markets(false)
        .await
        .expect("Failed to load markets");

    // Fetch BTC trades
    let trades = exchange
        .fetch_trades("BTC/USDC:USDC", Some(10))
        .await
        .expect("Failed to fetch trades");

    // Trades may be empty on testnet, but should not error
    for trade in &trades {
        assert!(trade.timestamp > 0, "Trade should have valid timestamp");
        assert!(
            trade.price.0 > rust_decimal::Decimal::ZERO,
            "Trade should have positive price"
        );
        assert!(
            trade.amount.0 > rust_decimal::Decimal::ZERO,
            "Trade should have positive amount"
        );
    }
}
