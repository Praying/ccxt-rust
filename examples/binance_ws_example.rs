//! Binance WebSocket example
//!
//! Demonstrates how to use Binance WebSocket features for real-time market data monitoring

// Allow clippy warnings for example code - examples prioritize readability over strict linting
#![allow(clippy::disallowed_methods)]

use anyhow::{Context, Result};
use ccxt_core::prelude::{Amount, ExchangeConfig, Price};
use ccxt_exchanges::binance::Binance;
use rust_decimal::Decimal;
use serde_json::json;
use std::collections::HashMap;
use tokio::time::{Duration, sleep};

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== Binance WebSocket Example ===\n");

    let exchange =
        Binance::new(ExchangeConfig::default()).context("Failed to initialize Binance exchange")?;

    // Example 1: Monitor single ticker
    println!("1. Monitor single ticker (BTC/USDT)");
    println!("   Connecting to WebSocket...");

    match exchange.watch_ticker("BTC/USDT", None).await {
        Ok(ticker) => {
            println!("   ✓ Connected successfully!");
            println!("   Symbol: {}", ticker.symbol);
            println!(
                "   Last Price: {}",
                ticker.last.unwrap_or(Price(Decimal::ZERO))
            );
            println!("   Bid: {}", ticker.bid.unwrap_or(Price(Decimal::ZERO)));
            println!("   Ask: {}", ticker.ask.unwrap_or(Price(Decimal::ZERO)));
            println!(
                "   24h Volume: {}",
                ticker.base_volume.unwrap_or(Amount(Decimal::ZERO))
            );
            println!(
                "   24h Change: {}%",
                ticker.percentage.unwrap_or(Decimal::ZERO)
            );
        }
        Err(e) => {
            eprintln!("   ✗ Error: {}", e);
        }
    }

    sleep(Duration::from_secs(2)).await;

    // Example 2: Use miniTicker (more lightweight)
    println!("\n2. Monitor using miniTicker (ETH/USDT)");

    let mut params = HashMap::new();
    params.insert("name".to_string(), json!("miniTicker"));

    match exchange.watch_ticker("ETH/USDT", Some(params)).await {
        Ok(ticker) => {
            println!("   ✓ MiniTicker connected successfully!");
            println!("   Symbol: {}", ticker.symbol);
            println!(
                "   Last Price: {}",
                ticker.last.unwrap_or(Price(Decimal::ZERO))
            );
            println!(
                "   24h Volume: {}",
                ticker.base_volume.unwrap_or(Amount(Decimal::ZERO))
            );
        }
        Err(e) => {
            eprintln!("   ✗ Error: {}", e);
        }
    }

    sleep(Duration::from_secs(2)).await;

    // Example 3: Monitor multiple tickers
    println!("\n3. Monitor multiple tickers");

    let symbols = vec![
        "BTC/USDT".to_string(),
        "ETH/USDT".to_string(),
        "BNB/USDT".to_string(),
    ];

    match exchange.watch_tickers(Some(symbols.clone()), None).await {
        Ok(tickers) => {
            println!("   ✓ Received {} ticker updates", tickers.len());

            for symbol in &symbols {
                if let Some(ticker) = tickers.get(symbol) {
                    println!(
                        "   {} - ${}",
                        ticker.symbol,
                        ticker.last.unwrap_or(Price(Decimal::ZERO))
                    );
                }
            }
        }
        Err(e) => {
            eprintln!("   ✗ Error: {}", e);
        }
    }

    sleep(Duration::from_secs(2)).await;

    // Example 4: Monitor all tickers (may take longer)
    println!("\n4. Monitor all tickers (first 10)");

    match exchange.watch_tickers(None, None).await {
        Ok(tickers) => {
            println!("   ✓ Received {} tickers", tickers.len());

            for (i, (symbol, ticker)) in tickers.iter().enumerate() {
                if i >= 10 {
                    break;
                }
                println!(
                    "   {} - ${:.2}",
                    symbol,
                    ticker.last.unwrap_or(Price(Decimal::ZERO))
                );
            }

            println!("   ... {} trading pairs total", tickers.len());
        }
        Err(e) => {
            eprintln!("   ✗ Error: {}", e);
        }
    }

    sleep(Duration::from_secs(2)).await;

    // Example 5: Monitor futures mark price (1-second updates)
    println!("\n5. Monitor futures mark price (BTC/USDT:USDT, 1-second updates)");

    let mut params = HashMap::new();
    params.insert("use1sFreq".to_string(), json!(true));

    match exchange
        .watch_mark_price("BTC/USDT:USDT", Some(params))
        .await
    {
        Ok(ticker) => {
            println!("   ✓ Mark price connected successfully!");
            println!("   Symbol: {}", ticker.symbol);
            println!(
                "   Mark Price: {}",
                ticker.last.unwrap_or(Price(Decimal::ZERO))
            );

            if !ticker.info.is_empty() {
                let info = &ticker.info;
                if let Some(index_price) = info.get("indexPrice") {
                    println!("   Index Price: {}", index_price);
                }
                if let Some(funding_rate) = info.get("fundingRate") {
                    println!("   Funding Rate: {}", funding_rate);
                }
            }
        }
        Err(e) => {
            eprintln!("   ✗ Error: {}", e);
        }
    }

    sleep(Duration::from_secs(2)).await;

    // Example 6: Monitor multiple futures mark prices
    println!("\n6. Monitor multiple futures mark prices");

    let futures_symbols = vec!["BTC/USDT:USDT".to_string(), "ETH/USDT:USDT".to_string()];

    match exchange
        .watch_mark_prices(Some(futures_symbols.clone()), None)
        .await
    {
        Ok(tickers) => {
            println!("   ✓ Received {} mark prices", tickers.len());

            for (symbol, ticker) in &tickers {
                println!(
                    "   {} - Mark: ${:.2}",
                    symbol,
                    ticker.last.unwrap_or(Price(Decimal::ZERO))
                );
            }
        }
        Err(e) => {
            eprintln!("   ✗ Error: {}", e);
        }
    }

    // Example 7: Continuous ticker monitoring instructions
    println!("\n7. Continuous ticker monitoring");
    println!("   Tip: Continuous monitoring requires the watch_ticker() method");
    println!("   This method maintains WebSocket connection and continuously pushes updates");
    println!("   Example code:");
    println!("   ```rust");
    println!("   loop {{");
    println!("       let ticker = exchange.watch_ticker(\"BTC/USDT\", None).await?;");
    println!("       println!(\"Price: {{}}\", ticker.last.unwrap_or(Price(Decimal::ZERO)));");
    println!("       // Process ticker data...");
    println!("   }}");
    println!("   ```");

    sleep(Duration::from_secs(2)).await;

    println!("\n=== Example Complete ===");
    println!("\nTips:");
    println!("  - ticker: Complete 24-hour statistics");
    println!("  - miniTicker: Lightweight version with fewer fields");
    println!("  - markPrice: Futures mark price with funding rate");
    println!("  - bookTicker: Best bid/ask prices, use watch_bids_asks()");
    println!("  - 1s frequency: Faster updates but more bandwidth");
    println!("  - 3s frequency: Default frequency, balanced performance");

    Ok(())
}
