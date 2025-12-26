//! Binance Futures Transfer Example
//!
//! Demonstrates how to transfer assets between spot and futures accounts using
//! Binance's Futures Transfer API.
//!
//! Note: The futures_transfer method is being migrated to the new modular REST API structure.
//! This example shows the intended API usage.
//!
//! # Running the Example
//!
//! ```bash
//! # Set environment variables
//! export BINANCE_API_KEY="your_api_key"
//! export BINANCE_SECRET="your_secret"
//!
//! # Run the example
//! cargo run --example binance_futures_transfer_example
//! ```

// Allow clippy warnings for example code - examples prioritize readability over strict linting
#![allow(clippy::disallowed_methods)]
#![allow(clippy::inconsistent_digit_grouping)]
#![allow(clippy::option_as_ref_deref)]
#![allow(unused_imports)]

use ccxt_core::ExchangeConfig;
use ccxt_exchanges::binance::Binance;
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=========================================");
    println!("Binance Futures Transfer Example");
    println!("=========================================\n");

    let api_key = env::var("BINANCE_API_KEY").ok();
    let secret = env::var("BINANCE_SECRET").ok();

    if api_key.is_none() || secret.is_none() {
        println!("⚠️  Warning: API keys not set, demonstrating code structure only");
        println!("Please set environment variables: BINANCE_API_KEY and BINANCE_SECRET");
    }

    let config = ExchangeConfig {
        api_key,
        secret,
        ..Default::default()
    };

    let _binance = Binance::new(config)?;

    println!(
        "Note: The futures_transfer method is being migrated to the new modular REST API structure."
    );
    println!("This example shows the intended API usage.\n");

    // ============================================================================
    // Example 1: Transfer from Spot to USDT-M Futures
    // ============================================================================
    println!("📤 Example 1: Spot → USDT-M Futures (type=1)");
    println!("----------------------------------------");
    println!("Code example:");
    println!(
        r#"
let transfer = binance.futures_transfer(
    "USDT",  // Asset code
    50.0,    // Transfer amount
    1,       // type=1: Spot → USDT-M Futures
    None,    // Optional parameters
).await?;

println!("Transfer ID: {{:?}}", transfer.id);
println!("Status: {{}}", transfer.status);
        "#
    );

    println!();

    // ============================================================================
    // Example 2: Transfer Type Descriptions
    // ============================================================================
    println!("📋 Example 2: Futures Transfer Type Descriptions");
    println!("----------------------------------------");

    let transfer_types = vec![
        (1, "Spot → USDT-M Futures", "For USDT-margined contracts"),
        (
            2,
            "USDT-M Futures → Spot",
            "Withdraw funds from USDT contracts",
        ),
        (3, "Spot → COIN-M Futures", "For coin-margined contracts"),
        (
            4,
            "COIN-M Futures → Spot",
            "Withdraw funds from coin-margined contracts",
        ),
    ];

    for (type_code, description, note) in transfer_types {
        println!("  Type {}: {}", type_code, description);
        println!("         {}", note);
    }

    println!();

    println!("=================================");
    println!("Example execution completed!");
    println!("=================================");

    Ok(())
}
