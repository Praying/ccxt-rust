//! Binance internal transfer functionality example.
//!
//! Note: Transfer-related methods (transfer, fetch_transfers, fetch_deposit_withdraw_fees)
//! are not yet migrated to the new modular structure. This is a placeholder example.
//!
//! # Running the Example
//!
//! ```bash
//! cargo run --example binance_transfer_example
//! ```

use ccxt_core::ExchangeConfig;
use ccxt_exchanges::binance::Binance;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=================================");
    println!("Binance Internal Transfer Example");
    println!("=================================\n");

    let config = ExchangeConfig {
        api_key: std::env::var("BINANCE_API_KEY").ok(),
        secret: std::env::var("BINANCE_API_SECRET").ok(),
        ..Default::default()
    };

    let _binance = Binance::new(config)?;

    // Note: The following methods are not yet migrated to the new modular structure:
    // - transfer
    // - fetch_transfers
    // - fetch_deposit_withdraw_fees

    println!("Note: Transfer-related methods are not yet migrated to the new modular structure.");
    println!("      See rest_old.rs for the original implementations.\n");

    println!("Code examples (not executable until methods are migrated):");
    println!(
        r#"
// Transfer from spot to futures
let transfer = binance.transfer(
    "USDT",      // Asset code
    100.0,       // Amount
    "spot",      // From spot account
    "future",    // To futures account
    None,        // Optional parameters
).await?;

// Query transfer history
let transfers = binance.fetch_transfers(
    Some("USDT"),  // Currency filter
    None,          // Since timestamp
    Some(50),      // Limit
    None,          // Optional parameters
).await?;

// Query deposit/withdrawal fees
let fees = binance.fetch_deposit_withdraw_fees(
    Some("BTC"),   // Currency filter
    None,          // Optional parameters
).await?;
"#
    );

    println!("\n=================================");
    println!("Example completed!");
    println!("=================================");

    Ok(())
}
