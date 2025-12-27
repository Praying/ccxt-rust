//! Binance Account Management Examples
//!
//! Demonstrates account management operations using ccxt-rust:
//! 1. Query balances for multiple account types (spot/margin/futures/funding)
//! 2. Internal account transfers
//! 3. Query transfer history
//! 4. Query maximum borrowable amount
//! 5. Query maximum transferable amount
//!
//! # Usage
//!
//! ```bash
//! # Set environment variables
//! export BINANCE_API_KEY="your_api_key"
//! export BINANCE_API_SECRET="your_api_secret"
//!
//! # Run the example
//! cargo run --example binance_account_example
//! ```

// Allow clippy warnings for example code - examples prioritize readability over strict linting
#![allow(clippy::disallowed_methods)]
#![allow(dead_code)]
use ccxt_core::types::AccountType;
use ccxt_exchanges::binance::Binance;
use ccxt_exchanges::prelude::ExchangeConfig;
use dotenvy::dotenv;
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();

    let api_key =
        env::var("BINANCE_API_KEY").expect("BINANCE_API_KEY environment variable not set");
    let secret =
        env::var("BINANCE_API_SECRET").expect("BINANCE_API_SECRET environment variable not set");

    let config = ExchangeConfig {
        api_key: Some(api_key),
        secret: Some(secret),
        ..Default::default()
    };
    let binance = Binance::new(config)?;

    println!("=== Binance Account Management Examples ===\n");

    example_fetch_spot_balance(&binance).await?;
    example_fetch_margin_balance(&binance).await?;
    example_fetch_future_balance(&binance).await?;
    example_fetch_funding_balance(&binance).await?;

    println!("\n=== Examples Complete ===");

    Ok(())
}

/// Example 1: Fetch spot account balance
async fn example_fetch_spot_balance(binance: &Binance) -> Result<(), Box<dyn std::error::Error>> {
    println!("📊 Example 1: Fetch Spot Account Balance");
    println!("----------------------------------------");

    // Note: The current implementation uses account_type as a string parameter
    let balance = binance.fetch_balance(Some(AccountType::Spot)).await?;

    println!("Spot Account Balance:");
    for (currency, entry) in balance.balances.iter().take(5) {
        if entry.total >= rust_decimal::Decimal::ZERO {
            println!(
                "  {} - Total: {}, Free: {}, Used: {}",
                currency, entry.total, entry.free, entry.used
            );
        }
    }
    println!("(Showing first 5 currencies with balance)\n");

    Ok(())
}

/// Example 2: Fetch cross margin account balance
async fn example_fetch_margin_balance(binance: &Binance) -> Result<(), Box<dyn std::error::Error>> {
    println!("📊 Example 2: Fetch Cross Margin Account Balance");
    println!("----------------------------------------");

    let balance = binance.fetch_balance(Some(AccountType::Margin)).await?;

    println!("Cross Margin Account Balance:");
    for (currency, entry) in balance.balances.iter().take(5) {
        if entry.total >= rust_decimal::Decimal::ZERO {
            println!(
                "  {} - Total: {}, Free: {}, Used: {}",
                currency, entry.total, entry.free, entry.used
            );
        }
    }
    println!("(Showing first 5 currencies with balance)\n");

    Ok(())
}

/// Example 3: Fetch futures account balance
async fn example_fetch_future_balance(binance: &Binance) -> Result<(), Box<dyn std::error::Error>> {
    println!("📊 Example 3: Fetch Futures Account Balance");
    println!("----------------------------------------");

    let balance = binance.fetch_balance(Some(AccountType::Futures)).await?;

    println!("Futures Account Balance:");
    for (currency, entry) in balance.balances.iter().take(5) {
        if entry.total >= rust_decimal::Decimal::ZERO {
            println!(
                "  {} - Total: {}, Free: {}, Used: {}",
                currency, entry.total, entry.free, entry.used
            );
        }
    }
    println!("(Showing first 5 currencies with balance)\n");

    Ok(())
}

/// Example 4: Fetch funding account balance
async fn example_fetch_funding_balance(
    binance: &Binance,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("📊 Example 4: Fetch Funding Account Balance");
    println!("----------------------------------------");

    let balance = binance.fetch_balance(Some(AccountType::Funding)).await?;

    println!("Funding Account Balance:");
    for (currency, entry) in balance.balances.iter().take(5) {
        if entry.total >= rust_decimal::Decimal::ZERO {
            println!(
                "  {} - Total: {}, Free: {}, Used: {}",
                currency, entry.total, entry.free, entry.used
            );
        }
    }
    println!("(Showing first 5 currencies with balance)\n");

    Ok(())
}
