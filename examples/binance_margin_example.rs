//! Binance Margin Trading Example
//!
//! Note: Margin trading methods are not yet migrated to the new modular structure.
//! This is a placeholder example.
//!
//! # Running the Example
//!
//! ```bash
//! cargo run --example binance_margin_example
//! ```

use ccxt_core::ExchangeConfig;
use ccxt_exchanges::binance::Binance;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=================================");
    println!("Binance Margin Trading Example");
    println!("=================================\n");

    let config = ExchangeConfig {
        api_key: std::env::var("BINANCE_API_KEY").ok(),
        secret: std::env::var("BINANCE_SECRET").ok(),
        ..Default::default()
    };

    let _binance = Binance::new(config)?;

    // Note: Margin trading methods are not yet migrated to the new modular structure:
    // - borrow_margin
    // - repay_margin
    // - fetch_borrow_rate
    // - fetch_borrow_rates
    // - fetch_borrow_rate_history

    println!("Note: Margin trading methods are not yet migrated to the new modular structure.");
    println!("      See rest_old.rs for the original implementations.\n");

    println!("Code examples (not executable until methods are migrated):");
    println!(
        r#"
// Borrow margin
let result = binance.borrow_margin(
    "BTC",       // Asset
    0.1,         // Amount
    "BTC/USDT",  // Symbol
    None,        // Optional parameters
).await?;

// Repay margin
let result = binance.repay_margin(
    "BTC",       // Asset
    0.1,         // Amount
    "BTC/USDT",  // Symbol
    None,        // Optional parameters
).await?;

// Fetch borrow rate
let rate = binance.fetch_borrow_rate("BTC", None).await?;

// Fetch borrow rates
let rates = binance.fetch_borrow_rates(None).await?;
"#
    );

    println!("\n=================================");
    println!("Example completed!");
    println!("=================================");

    Ok(())
}
