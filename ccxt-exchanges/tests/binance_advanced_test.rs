//! Binance advanced feature integration tests.
//!
//! Test coverage for newly added API methods.

use ccxt_core::{ExchangeConfig, OrderStatus, Result};
use ccxt_exchanges::binance::Binance;

/// Test fetching currency information.
#[tokio::test]
#[ignore] // Requires API credentials
async fn test_fetch_currencies() -> Result<()> {
    let config = ExchangeConfig {
        api_key: std::env::var("BINANCE_API_KEY").ok(),
        secret: std::env::var("BINANCE_SECRET").ok(),
        ..Default::default()
    };

    let binance = Binance::new(config)?;
    let currencies = binance.fetch_currencies().await?;

    assert!(!currencies.is_empty(), "Should fetch currency information");
    println!("Fetched {} currencies", currencies.len());

    assert!(
        currencies.iter().any(|c| c.code == "BTC"),
        "Currency list should contain BTC"
    );

    Ok(())
}

/// Test fetching trading fees.
#[tokio::test]
#[ignore] // Requires API credentials
async fn test_fetch_trading_fees() -> Result<()> {
    let config = ExchangeConfig {
        api_key: std::env::var("BINANCE_API_KEY").ok(),
        secret: std::env::var("BINANCE_SECRET").ok(),
        ..Default::default()
    };

    let binance = Binance::new(config)?;
    let _ = binance.fetch_markets().await?;
    let fees = binance.fetch_trading_fees(None, None).await?;

    assert!(!fees.is_empty(), "Should fetch trading fee information");
    println!("Fetched fees for {} symbols", fees.len());

    Ok(())
}

/// Test fetching trading fee for a specific symbol.
#[tokio::test]
#[ignore] // Requires API credentials
async fn test_fetch_trading_fee() -> Result<()> {
    let config = ExchangeConfig {
        api_key: std::env::var("BINANCE_API_KEY").ok(),
        secret: std::env::var("BINANCE_SECRET").ok(),
        ..Default::default()
    };

    let binance = Binance::new(config)?;
    let _ = binance.fetch_markets().await?;
    let fee = binance.fetch_trading_fee("BTC/USDT", None).await?;

    assert!(
        fee.symbol == "BTC/USDT",
        "Fee object should correspond to BTC/USDT"
    );
    println!("BTC/USDT fee: {:?}", fee);

    Ok(())
}

/// Test fetching all orders.
#[tokio::test]
#[ignore] // Requires API credentials and order history
async fn test_fetch_orders() -> Result<()> {
    let config = ExchangeConfig {
        api_key: std::env::var("BINANCE_API_KEY").ok(),
        secret: std::env::var("BINANCE_SECRET").ok(),
        ..Default::default()
    };

    let binance = Binance::new(config)?;
    let _ = binance.fetch_markets().await?;

    let orders = binance
        .fetch_orders(Some("BTC/USDT"), None, Some(10))
        .await?;

    println!("Fetched {} orders", orders.len());

    for order in orders.iter().take(3) {
        assert!(!order.id.is_empty(), "Order ID should not be empty");
        assert!(!order.symbol.is_empty(), "Symbol should not be empty");
    }

    Ok(())
}

/// Test fetching closed orders.
#[tokio::test]
#[ignore] // Requires API credentials and order history
async fn test_fetch_closed_orders() -> Result<()> {
    let config = ExchangeConfig {
        api_key: std::env::var("BINANCE_API_KEY").ok(),
        secret: std::env::var("BINANCE_SECRET").ok(),
        ..Default::default()
    };

    let binance = Binance::new(config)?;
    let _ = binance.fetch_markets().await?;

    let orders = binance
        .fetch_closed_orders(Some("BTC/USDT"), None, Some(10))
        .await?;

    println!("Fetched {} closed orders", orders.len());

    for order in &orders {
        assert!(
            order.status == OrderStatus::Closed
                || order.status == OrderStatus::Canceled
                || order.status == OrderStatus::Rejected
                || order.status == OrderStatus::Expired,
            "Order status should be a closed type"
        );
    }

    Ok(())
}

/// Test batch order cancellation (simulated, not executed).
#[tokio::test]
#[ignore] // Requires API credentials and actual orders
async fn test_cancel_orders() -> Result<()> {
    let config = ExchangeConfig {
        api_key: std::env::var("BINANCE_API_KEY").ok(),
        secret: std::env::var("BINANCE_SECRET").ok(),
        ..Default::default()
    };

    let binance = Binance::new(config)?;
    let _ = binance.fetch_markets().await?;

    // Note: This test requires actual order IDs.
    // In practice, create orders first, then use the returned order IDs.
    let _order_ids = vec!["12345".to_string(), "67890".to_string()];

    // Uncomment for actual testing:
    // let result = binance.cancel_orders(order_ids, "BTC/USDT").await?;
    // assert!(!result.is_empty(), "Should return canceled orders");

    println!("Batch cancel orders test (requires actual order IDs)");

    Ok(())
}

/// Test canceling all orders (simulated, not executed).
#[tokio::test]
#[ignore] // Requires API credentials and actual orders
async fn test_cancel_all_orders() -> Result<()> {
    let config = ExchangeConfig {
        api_key: std::env::var("BINANCE_API_KEY").ok(),
        secret: std::env::var("BINANCE_SECRET").ok(),
        ..Default::default()
    };

    let binance = Binance::new(config)?;
    let _ = binance.fetch_markets().await?;

    // Uncomment for actual testing:
    // let result = binance.cancel_all_orders("BTC/USDT").await?;
    // println!("Canceled {} orders", result.len());

    println!("Cancel all orders test (requires actual orders)");

    Ok(())
}

/// Integration test: Complete order management workflow.
#[tokio::test]
#[ignore] // Requires API credentials and sufficient balance
async fn test_order_management_flow() -> Result<()> {
    let config = ExchangeConfig {
        api_key: std::env::var("BINANCE_API_KEY").ok(),
        secret: std::env::var("BINANCE_SECRET").ok(),
        ..Default::default()
    };

    let binance = Binance::new(config)?;

    let _ = binance.fetch_markets().await?;
    println!("✓ Markets loaded successfully");

    let fee = binance.fetch_trading_fee("BTC/USDT", None).await?;
    println!("✓ Trading fee fetched successfully: {:?}", fee);

    let open_orders = binance.fetch_open_orders(Some("BTC/USDT")).await?;
    println!("✓ Open orders count: {}", open_orders.len());

    let all_orders = binance
        .fetch_orders(Some("BTC/USDT"), None, Some(5))
        .await?;
    println!("✓ Historical orders count: {}", all_orders.len());

    let closed_orders = binance
        .fetch_closed_orders(Some("BTC/USDT"), None, Some(5))
        .await?;
    println!("✓ Closed orders count: {}", closed_orders.len());

    println!("\n=== Order management workflow test completed ===");

    Ok(())
}
