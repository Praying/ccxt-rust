//! Binance Ledger Query Example
//!
//! Demonstrates how to query Binance ledger history records using ccxt-rust.
//!
//! # Important Notes
//!
//! - Only supports futures wallets (Options, USDT-M, COIN-M)
//! - Spot wallet does not support this functionality
//! - Requires API key and secret for access
//!
//! # Status
//!
//! This example is currently a placeholder. The `fetch_ledger` method
//! is being migrated to the new modular REST API structure.
//!
//! # Prerequisites
//!
//! Set the following environment variables:
//! - `BINANCE_API_KEY`: Your Binance API key
//! - `BINANCE_SECRET`: Your Binance API secret
//!
//! # Examples
//!
//! ```bash
//! # Set environment variables
//! export BINANCE_API_KEY="your_api_key"
//! export BINANCE_SECRET="your_secret"
//!
//! # Run the example
//! cargo run --example binance_ledger_example
//! ```

use ccxt_rust::prelude::*;
use std::env;

#[tokio::main]
async fn main() -> Result<()> {
    let api_key = env::var("BINANCE_API_KEY").ok();
    let secret = env::var("BINANCE_SECRET").ok();

    let _exchange = ccxt_exchanges::binance::Binance::new(ExchangeConfig {
        api_key,
        secret,
        ..Default::default()
    })?;

    println!("=== Binance Ledger Query Example ===\n");
    println!(
        "Note: The fetch_ledger method is being migrated to the new modular REST API structure."
    );
    println!("This example will be updated once the migration is complete.\n");

    // The fetch_ledger method will support:
    // - USDT-M Futures Ledger
    // - COIN-M Futures Ledger
    // - Options Ledger
    // - Time Range Queries
    // - Portfolio Margin Account Queries
    //
    // Note: Spot wallet does not support ledger queries.

    println!("Available ledger query types (when implemented):");
    println!("  1. USDT-M Futures Ledger (type: 'future')");
    println!("  2. COIN-M Futures Ledger (type: 'delivery')");
    println!("  3. Options Ledger (type: 'option')");
    println!("  4. Portfolio Margin Account (portfolioMargin: 'true')");
    println!("\nSpot wallet does NOT support ledger queries.");

    println!("\n=== Example Complete ===");
    Ok(())
}
