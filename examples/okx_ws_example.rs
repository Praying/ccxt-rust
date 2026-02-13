#![allow(clippy::disallowed_methods)]

use anyhow::Result;
use ccxt_core::logging::{LogConfig, init_logging};
use ccxt_core::prelude::Price;
use ccxt_exchanges::okx::Okx;
use ccxt_exchanges::prelude::*;
use futures::StreamExt;
use rust_decimal::Decimal;
use std::env;
use tokio::time::Duration;

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== OKX WebSocket Example ===\n");
    let mut config = LogConfig::development();
    config.show_span_events = false;
    init_logging(&config);

    let api_key = env::var("OKX_API_KEY").ok();
    let secret = env::var("OKX_SECRET").ok();
    let passphrase = env::var("OKX_PASSPHRASE").ok();

    let mut builder = Okx::builder();
    if let (Some(k), Some(s), Some(p)) = (&api_key, &secret, &passphrase) {
        builder = builder.api_key(k).secret(s).passphrase(p);
    }
    let exchange = anyhow::Context::context(builder.build(), "Failed to initialize OKX exchange")?;

    println!("Connecting to WebSocket...");
    exchange.ws_connect().await?;
    println!("Connected!");

    println!("1. 【watch_ticker】Monitor BTC/USDT");
    match exchange.watch_ticker("BTC/USDT").await {
        Ok(mut stream) => {
            println!("   ✓ Subscribed to BTC/USDT ticker");
            let mut count = 0;
            while let Some(result) = stream.next().await {
                match result {
                    Ok(ticker) => {
                        println!(
                            "     Update {}: {} Last: {}",
                            count + 1,
                            ticker.symbol,
                            ticker.last.unwrap_or(Price(Decimal::ZERO))
                        );
                        count += 1;
                        if count >= 3 {
                            break;
                        }
                    }
                    Err(e) => {
                        eprintln!("     Error: {}", e);
                        break;
                    }
                }
            }
        }
        Err(e) => eprintln!("   ✗ Error: {}", e),
    }
    println!();

    println!("2. 【watch_tickers】Monitor BTC/USDT and ETH/USDT");
    match exchange
        .watch_tickers(&["BTC/USDT".to_string(), "ETH/USDT".to_string()])
        .await
    {
        Ok(mut stream) => {
            println!("   ✓ Subscribed to multiple tickers");
            let mut count = 0;
            while let Some(result) = stream.next().await {
                match result {
                    Ok(tickers) => {
                        println!(
                            "     Update {}: Received {} tickers",
                            count + 1,
                            tickers.len()
                        );
                        for ticker in tickers {
                            println!(
                                "       - {}: {}",
                                ticker.symbol,
                                ticker.last.unwrap_or(Price(Decimal::ZERO))
                            );
                        }
                        count += 1;
                        if count >= 3 {
                            break;
                        }
                    }
                    Err(e) => {
                        eprintln!("     Error: {}", e);
                        break;
                    }
                }
            }
        }
        Err(e) => eprintln!("   ✗ Error: {}", e),
    }
    println!();

    println!("3. 【watch_order_book】Monitor BTC/USDT Order Book");
    match exchange.watch_order_book("BTC/USDT", Some(5)).await {
        Ok(mut stream) => {
            println!("   ✓ Subscribed to order book");
            let mut count = 0;
            while let Some(result) = stream.next().await {
                match result {
                    Ok(book) => {
                        println!(
                            "     Update {}: Bids={} Asks={} ",
                            count + 1,
                            book.bids.len(),
                            book.asks.len()
                        );
                        if let Some(best_bid) = book.bids.first() {
                            println!("       Best Bid: {} @ {}", best_bid.amount, best_bid.price);
                        }
                        count += 1;
                        if count >= 3 {
                            break;
                        }
                    }
                    Err(e) => {
                        eprintln!("     Error: {}", e);
                        break;
                    }
                }
            }
        }
        Err(e) => eprintln!("   ✗ Error: {}", e),
    }
    println!();

    println!("4. 【watch_trades】Monitor BTC/USDT Trades");
    match exchange.watch_trades("BTC/USDT").await {
        Ok(mut stream) => {
            println!("   ✓ Subscribed to trades");
            let mut count = 0;
            while let Some(result) = stream.next().await {
                match result {
                    Ok(trades) => {
                        println!(
                            "     Update {}: Received {} trades",
                            count + 1,
                            trades.len()
                        );
                        if let Some(trade) = trades.first() {
                            println!(
                                "       Latest: {:?} {} @ {}",
                                trade.side, trade.amount, trade.price
                            );
                        }
                        count += 1;
                        if count >= 3 {
                            break;
                        }
                    }
                    Err(e) => {
                        eprintln!("     Error: {}", e);
                        break;
                    }
                }
            }
        }
        Err(e) => eprintln!("   ✗ Error: {}", e),
    }
    println!();

    if api_key.is_some() {
        println!("5. 【Private Channels】Monitor Balance and Orders");

        println!("   Subscribing to balance updates...");
        match exchange.watch_balance().await {
            Ok(mut stream) => {
                println!("   ✓ Subscribed to balance");
                let timeout = tokio::time::timeout(Duration::from_secs(5), stream.next()).await;
                match timeout {
                    Ok(Some(Ok(balance))) => {
                        println!(
                            "     Received balance update. Total currencies: {}",
                            balance.balances.len()
                        );
                    }
                    Ok(Some(Err(e))) => eprintln!("     Error: {}", e),
                    Ok(None) => println!("     Stream ended"),
                    Err(_) => println!("     Timeout waiting for balance update"),
                }
            }
            Err(e) => eprintln!("   ✗ Error watching balance: {}", e),
        }

        println!("   Subscribing to order updates...");
        match exchange.watch_orders(None).await {
            Ok(mut stream) => {
                println!("   ✓ Subscribed to orders");
                let timeout = tokio::time::timeout(Duration::from_secs(5), stream.next()).await;
                match timeout {
                    Ok(Some(Ok(order))) => {
                        println!(
                            "     Received order update: {} {:?}",
                            order.id, order.status
                        );
                    }
                    Ok(Some(Err(e))) => eprintln!("     Error: {}", e),
                    Ok(None) => println!("     Stream ended"),
                    Err(_) => println!("     Timeout waiting for order update"),
                }
            }
            Err(e) => eprintln!("   ✗ Error watching orders: {}", e),
        }
    } else {
        println!("5. 【Private Channels】Skipped (requires API credentials)");
    }

    println!("=== Example Complete ===");
    exchange.ws_disconnect().await?;

    Ok(())
}
