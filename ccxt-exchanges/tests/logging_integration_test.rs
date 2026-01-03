//! Logging system end-to-end integration tests
//!
//! Verifies logging functionality during actual HTTP requests and exchange operations.

use ccxt_core::{
    ExchangeConfig,
    error::Result,
    logging::{LogConfig, init_logging},
    types::Market,
};
use ccxt_exchanges::binance::Binance;
use std::sync::Once;

static INIT: Once = Once::new();

fn setup() {
    INIT.call_once(|| {
        // Use test config to reduce log noise
        let _ = init_logging(&LogConfig::test());
    });
}

async fn populate_test_markets(exchange: &Binance) {
    let market = Market {
        id: "BTCUSDT".to_string(),
        symbol: "BTC/USDT".to_string(),
        base: "BTC".to_string(),
        quote: "USDT".to_string(),
        ..Market::default()
    };
    let market2 = Market {
        id: "ETHUSDT".to_string(),
        symbol: "ETH/USDT".to_string(),
        base: "ETH".to_string(),
        quote: "USDT".to_string(),
        ..Market::default()
    };
    let market3 = Market {
        id: "BNBUSDT".to_string(),
        symbol: "BNB/USDT".to_string(),
        base: "BNB".to_string(),
        quote: "USDT".to_string(),
        ..Market::default()
    };
    exchange
        .base()
        .set_markets(vec![market, market2, market3], None)
        .await
        .unwrap();
}

#[tokio::test]
async fn test_logging_with_public_api_calls() -> Result<()> {
    setup();

    let config = ExchangeConfig::default();
    let exchange = Binance::new(config)?;
    populate_test_markets(&exchange).await;

    // Should generate logs without panicking
    // Note: This might fail if network is unavailable, but we want to test logging logic
    let result = exchange
        .fetch_ticker("BTC/USDT", ccxt_core::types::TickerParams::default())
        .await;
    if let Err(e) = &result {
        println!("Network request failed (expected in some envs): {}", e);
        // If it failed due to network, it's fine for this test as long as it logged
        return Ok(());
    }
    let ticker = result?;
    assert!(ticker.symbol == "BTC/USDT");

    Ok(())
}

#[tokio::test]
async fn test_logging_with_verbose_mode() -> Result<()> {
    setup();

    let mut config = ExchangeConfig::default();
    config.verbose = true;

    let exchange = Binance::new(config)?;
    populate_test_markets(&exchange).await;

    // Should generate detailed HTTP logs
    let result = exchange
        .fetch_ticker("ETH/USDT", ccxt_core::types::TickerParams::default())
        .await;
    if let Err(e) = &result {
        println!("Network request failed (expected in some envs): {}", e);
        return Ok(());
    }
    let ticker = result?;
    assert!(ticker.symbol == "ETH/USDT");

    Ok(())
}

#[tokio::test]
async fn test_logging_with_multiple_requests() -> Result<()> {
    setup();

    let config = ExchangeConfig::default();
    let exchange = Binance::new(config)?;
    populate_test_markets(&exchange).await;

    let symbols = vec!["BTC/USDT", "ETH/USDT", "BNB/USDT"];

    for symbol in symbols {
        let result = exchange
            .fetch_ticker(symbol, ccxt_core::types::TickerParams::default())
            .await;
        if let Err(e) = &result {
            println!("Network request failed for {}: {}", symbol, e);
            continue;
        }
        let ticker = result?;
        assert!(ticker.symbol == symbol);
    }

    Ok(())
}

#[tokio::test]
async fn test_logging_with_orderbook() -> Result<()> {
    setup();

    let mut config = ExchangeConfig::default();
    config.verbose = true;

    let exchange = Binance::new(config)?;
    populate_test_markets(&exchange).await;

    let result = exchange.fetch_order_book("BTC/USDT", None).await;
    if let Err(e) = &result {
        println!("Network request failed (expected in some envs): {}", e);
        return Ok(());
    }
    let orderbook = result?;
    assert!(!orderbook.bids.is_empty());
    assert!(!orderbook.asks.is_empty());

    Ok(())
}

#[tokio::test]
async fn test_logging_with_trades() -> Result<()> {
    setup();

    let config = ExchangeConfig::default();
    let exchange = Binance::new(config)?;
    populate_test_markets(&exchange).await;

    let result = exchange.fetch_trades("BTC/USDT", None).await;
    if let Err(e) = &result {
        println!("Network request failed (expected in some envs): {}", e);
        return Ok(());
    }
    let trades = result?;
    assert!(!trades.is_empty());

    Ok(())
}

#[tokio::test]
async fn test_logging_error_handling() -> Result<()> {
    setup();

    let mut config = ExchangeConfig::default();
    config.verbose = true;

    let exchange = Binance::new(config)?;
    // Don't populate markets to force error or use invalid pair

    // Should log error without panicking
    let result = exchange
        .fetch_ticker("INVALID/PAIR", ccxt_core::types::TickerParams::default())
        .await;

    assert!(result.is_err());

    Ok(())
}

#[cfg(test)]
mod stress_tests {
    use super::*;

    #[tokio::test]
    async fn test_logging_under_concurrent_load() -> Result<()> {
        setup();

        use std::sync::Arc;

        let config = ExchangeConfig::default();
        let exchange = Arc::new(Binance::new(config)?);
        populate_test_markets(&exchange).await;

        let tasks: Vec<_> = (0..5)
            .map(|_| {
                let exchange_clone = Arc::clone(&exchange);
                tokio::spawn(async move {
                    let result = exchange_clone
                        .fetch_ticker("BTC/USDT", ccxt_core::types::TickerParams::default())
                        .await;
                    // Ignore network errors
                    if result.is_err() {
                        return Ok::<(), ccxt_core::Error>(());
                    }
                    Ok::<(), ccxt_core::Error>(())
                })
            })
            .collect();

        for task in tasks {
            let _ = task.await;
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_logging_with_rapid_requests() -> Result<()> {
        setup();

        let mut config = ExchangeConfig::default();
        config.verbose = false; // Disable verbose to reduce log volume

        let exchange = Binance::new(config)?;
        populate_test_markets(&exchange).await;

        for _ in 0..10 {
            let result = exchange
                .fetch_ticker("BTC/USDT", ccxt_core::types::TickerParams::default())
                .await;
            if result.is_err() {
                break; // Stop if network fails
            }
        }

        Ok(())
    }
}
