//! Binance watch_my_trades Example
//!
//! Demonstrates how to subscribe to user trade records via WebSocket.

// Allow clippy warnings for example code - examples prioritize readability over strict linting
#![allow(clippy::field_reassign_with_default)]

use ccxt_core::error::Result;
use ccxt_exchanges::binance::Binance;
use std::collections::HashMap;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<()> {
    let mut config = ccxt_core::ExchangeConfig::default();
    config.api_key = std::env::var("BINANCE_API_KEY").ok();
    config.secret = std::env::var("BINANCE_API_SECRET").ok();

    let exchange = Arc::new(Binance::new(config)?);

    println!("Starting subscription to user trade records...");

    // Subscribe to all trading pairs
    match exchange
        .clone()
        .watch_my_trades(None, None, Some(10), None)
        .await
    {
        Ok(trades) => {
            println!("\nReceived {} trade records:", trades.len());
            for trade in trades {
                println!(
                    "  {} - {} {} @ {} (order: {:?})",
                    trade.symbol, trade.side, trade.amount, trade.price, trade.order
                );
            }
        }
        Err(e) => {
            eprintln!("Failed to subscribe to trades: {:?}", e);
        }
    }

    // Subscribe to specific trading pair
    println!("\nSubscribing to BTC/USDT trades...");
    match exchange
        .clone()
        .watch_my_trades(Some("BTC/USDT"), None, Some(5), Some(HashMap::new()))
        .await
    {
        Ok(trades) => {
            println!("\nReceived {} BTC/USDT trade records:", trades.len());
            for trade in trades {
                println!(
                    "  {} {} @ {} (fee: {:?})",
                    trade.side, trade.amount, trade.price, trade.fee
                );
            }
        }
        Err(e) => {
            eprintln!("Failed to subscribe to BTC/USDT trades: {:?}", e);
        }
    }

    Ok(())
}
