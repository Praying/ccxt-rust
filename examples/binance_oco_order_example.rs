//! Binance OCO Order Example
//!
//! Note: OCO order methods are not yet migrated to the new modular structure.
//! This is a placeholder example.
//!
//! # Running the Example
//!
//! ```bash
//! cargo run --example binance_oco_order_example
//! ```

use ccxt_core::ExchangeConfig;
use ccxt_exchanges::binance::Binance;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=================================");
    println!("Binance OCO Order Example");
    println!("=================================\n");

    let config = ExchangeConfig {
        api_key: std::env::var("BINANCE_API_KEY")
            .ok()
            .map(ccxt_core::SecretString::new),
        secret: std::env::var("BINANCE_API_SECRET")
            .ok()
            .map(ccxt_core::SecretString::new),
        ..Default::default()
    };

    let _binance = Binance::new(config)?;

    // Note: OCO order methods are not yet migrated to the new modular structure

    println!("Note: OCO order methods are not yet migrated to the new modular structure.");
    println!("      See rest_old.rs for the original implementations.\n");

    println!("Code examples (not executable until methods are migrated):");
    println!(
        r#"
// Create OCO order (One-Cancels-the-Other)
// This creates both a limit order and a stop-limit order
// When one is filled, the other is automatically cancelled

let result = binance.create_oco_order(
    "BTC/USDT",       // Symbol
    "sell",           // Side
    0.001,            // Amount
    50000.0,          // Price (limit order)
    45000.0,          // Stop price
    44900.0,          // Stop limit price
    None,             // Optional parameters
).await?;
"#
    );

    println!("\n=================================");
    println!("Example completed!");
    println!("=================================");

    Ok(())
}
