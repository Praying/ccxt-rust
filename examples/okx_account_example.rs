//! OKX Account Management Example
//!
//! Demonstrates account management operations using ccxt-rust:
//! 1. Query account balance
//! 2. Query trade history
//!
//! # Usage
//!
//! ```bash
//! # Set environment variables
//! export OKX_API_KEY="your_api_key"
//! export OKX_SECRET="your_secret"
//! export OKX_PASSPHRASE="your_passphrase"
//!
//! # Run the example
//! cargo run --example okx_account_example
//! ```

// Allow clippy warnings for example code
#![allow(clippy::disallowed_methods)]

use ccxt_exchanges::okx::OkxBuilder;
use dotenvy::dotenv;
use std::env;

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();
    println!("=== OKX Account Management Example ===\n");

    let api_key = env::var("OKX_API_KEY").unwrap_or_default();
    let secret = env::var("OKX_SECRET").unwrap_or_default();
    let passphrase = env::var("OKX_PASSPHRASE").unwrap_or_default();

    if api_key.is_empty() || secret.is_empty() || passphrase.is_empty() {
        println!("⚠️  Skipping example: OKX_API_KEY, OKX_SECRET, or OKX_PASSPHRASE not set");
        println!("   Please set these environment variables to run this example.");
        return Ok(());
    }

    // Initialize authenticated OKX exchange
    let exchange = OkxBuilder::new()
        .api_key(&api_key)
        .secret(&secret)
        .passphrase(&passphrase)
        .build()?;

    // 1. Fetch Account Balance
    println!("1. 【fetch_balance】Get account balance");
    match exchange.fetch_balance().await {
        Ok(balance) => {
            println!("   ✓ Successfully fetched balance");

            println!("   Non-zero balances:");
            let mut found_any = false;
            for (currency, entry) in balance.balances.iter() {
                if entry.total > rust_decimal::Decimal::ZERO {
                    println!(
                        "     - {}: Total={}, Free={}, Used={}",
                        currency, entry.total, entry.free, entry.used
                    );
                    found_any = true;
                }
            }
            if !found_any {
                println!("     (No non-zero balances found)");
            }
        }
        Err(e) => println!("   ✗ Error: {}", e),
    }
    println!();

    // 2. Fetch My Recent Trades
    println!("2. 【fetch_my_trades】Get my recent trades (BTC/USDT)");
    // Note: We need to load markets first for symbol resolution
    if let Err(e) = exchange.load_markets(false).await {
        println!("   ⚠️ Failed to load markets: {}", e);
    } else {
        match exchange.fetch_my_trades("BTC/USDT", None, Some(5)).await {
            Ok(trades) => {
                if trades.is_empty() {
                    println!("   ℹ No recent trades found");
                } else {
                    println!("   ✓ Found {} recent trades:", trades.len());
                    for (i, trade) in trades.iter().enumerate() {
                        println!(
                            "     {}. {:?} {} @ {} ({})",
                            i + 1,
                            trade.side,
                            trade.amount,
                            trade.price,
                            trade.datetime.as_deref().unwrap_or("N/A")
                        );
                    }
                }
            }
            Err(e) => println!("   ✗ Error: {}", e),
        }
    }
    println!();

    println!("=== Example Complete ===");

    Ok(())
}
