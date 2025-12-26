//! Binance Deposit/Withdrawal Example
//!
//! Note: Deposit and withdrawal methods are not yet migrated to the new modular structure.
//! This is a placeholder example.
//!
//! # Running the Example
//!
//! ```bash
//! cargo run --example binance_deposit_withdrawal_example
//! ```

use ccxt_core::ExchangeConfig;
use ccxt_exchanges::binance::Binance;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=================================");
    println!("Binance Deposit/Withdrawal Example");
    println!("=================================\n");

    let config = ExchangeConfig {
        api_key: std::env::var("BINANCE_API_KEY").ok(),
        secret: std::env::var("BINANCE_SECRET").ok(),
        ..Default::default()
    };

    let _binance = Binance::new(config)?;

    // Note: Deposit and withdrawal methods are not yet migrated to the new modular structure:
    // - fetch_deposits
    // - fetch_withdrawals
    // - withdraw

    println!(
        "Note: Deposit and withdrawal methods are not yet migrated to the new modular structure."
    );
    println!("      See rest_old.rs for the original implementations.\n");

    println!("Code examples (not executable until methods are migrated):");
    println!(
        r#"
// Fetch deposits
let deposits = binance.fetch_deposits(
    Some("BTC"),   // Currency filter
    None,          // Since timestamp
    Some(100),     // Limit
    None,          // Optional parameters
).await?;

// Fetch withdrawals
let withdrawals = binance.fetch_withdrawals(
    Some("USDT"),  // Currency filter
    None,          // Since timestamp
    Some(100),     // Limit
    None,          // Optional parameters
).await?;

// Withdraw
let result = binance.withdraw(
    "USDT",                    // Currency
    100.0,                     // Amount
    "0x1234...",               // Address
    None,                      // Tag (for some currencies)
    None,                      // Optional parameters
).await?;
"#
    );

    println!("\n=================================");
    println!("Example completed!");
    println!("=================================");

    Ok(())
}
