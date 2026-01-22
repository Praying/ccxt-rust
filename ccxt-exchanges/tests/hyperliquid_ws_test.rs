use ccxt_core::{ExchangeConfig, ws_exchange::WsExchange};
use ccxt_exchanges::hyperliquid::{HyperLiquid, HyperLiquidOptions};
use futures_util::StreamExt;
use std::time::Duration;
use tokio::time::timeout;

#[tokio::test]
async fn test_hyperliquid_ws_connection_lifecycle() {
    // Skip if no internet connection or in CI environment that blocks external connections
    if std::env::var("CI").is_ok() {
        return;
    }

    let config = ExchangeConfig::default();
    // Use testnet for safety
    let options = HyperLiquidOptions {
        testnet: true,
        ..Default::default()
    };

    let exchange = HyperLiquid::new_with_options(config, options, None).unwrap();

    // 1. Initial State
    assert!(!exchange.ws_is_connected());

    // 2. Connect
    let connect_result = timeout(Duration::from_secs(10), exchange.ws_connect()).await;
    assert!(connect_result.is_ok(), "Connection timed out");
    assert!(connect_result.unwrap().is_ok(), "Connection failed");

    assert!(exchange.ws_is_connected());

    // 3. Disconnect
    let disconnect_result = exchange.ws_disconnect().await;
    assert!(disconnect_result.is_ok());

    assert!(!exchange.ws_is_connected());
}

#[tokio::test]
async fn test_hyperliquid_ws_public_streams() {
    if std::env::var("CI").is_ok() {
        return;
    }

    let config = ExchangeConfig::default();
    let options = HyperLiquidOptions {
        testnet: true,
        ..Default::default()
    };

    let exchange = HyperLiquid::new_with_options(config, options, None).unwrap();
    exchange.ws_connect().await.unwrap();

    // Give connection a moment to stabilize
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Test OrderBook Subscription
    let mut orderbook_stream = exchange.watch_order_book("ETH", None).await.unwrap();
    let orderbook_item = timeout(Duration::from_secs(10), orderbook_stream.next())
        .await
        .ok()
        .flatten();
    assert!(matches!(orderbook_item, Some(Ok(_))));

    // Test Trades Subscription
    let mut trades_stream = exchange.watch_trades("ETH").await.unwrap();
    let trades_item = timeout(Duration::from_secs(10), trades_stream.next())
        .await
        .ok()
        .flatten();
    assert!(matches!(trades_item, Some(Ok(trades)) if !trades.is_empty()));

    // Test Ticker Subscription
    let mut ticker_stream = exchange.watch_ticker("ETH").await.unwrap();
    let ticker_item = timeout(Duration::from_secs(10), ticker_stream.next())
        .await
        .ok()
        .flatten();
    assert!(matches!(ticker_item, Some(Ok(_))));

    exchange.ws_disconnect().await.unwrap();
}

#[tokio::test]
async fn test_hyperliquid_ws_private_streams() {
    if std::env::var("CI").is_ok() {
        return;
    }

    // Requires API key/private key to derive address, or explicit address
    // Since we can't easily mock auth here without credentials, we'll test the logic
    // but expect authentication error if no credentials provided, which is correct behavior.

    let config = ExchangeConfig::default();
    let options = HyperLiquidOptions {
        testnet: true,
        ..Default::default()
    };

    let exchange = HyperLiquid::new_with_options(config, options, None).unwrap();

    // Should fail without auth
    let result = exchange.watch_orders(None).await;
    assert!(result.is_err());

    let result = exchange.watch_my_trades(None).await;
    assert!(result.is_err());
}
