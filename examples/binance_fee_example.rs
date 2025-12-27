//! Binance fee management example.
//!
//! Note: Many fee-related methods (fetch_funding_rate, fetch_funding_rate_history,
//! fetch_funding_rates, fetch_leverage_tiers) are not yet migrated to the new
//! modular structure. This example demonstrates the available methods.
//!
//! # Environment Variables
//!
//! - `BINANCE_API_KEY`: API key for authenticated endpoints (optional)
//! - `BINANCE_API_SECRET`: API secret for authenticated endpoints (optional)
//!
//! # Examples
//!
//! ```bash
//! # Run with authentication
//! BINANCE_API_KEY=your_key BINANCE_API_SECRET=your_secret cargo run --example binance_fee_example
//! ```

use ccxt_core::ExchangeConfig;
use ccxt_exchanges::prelude::Binance;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let config = ExchangeConfig {
        api_key: std::env::var("BINANCE_API_KEY").ok(),
        secret: std::env::var("BINANCE_API_SECRET").ok(),
        sandbox: false,
        ..Default::default()
    };
    let binance = Binance::new(config)?;

    println!("=== Binance Fee Management Examples ===\n");

    // Example 1: Fetch trading fee for a single symbol (requires API key)
    // Returns typed FeeTradingFee struct
    if binance.base().config.api_key.is_some() {
        println!("1. Fetching BTC/USDT trading fee...");
        match binance.fetch_trading_fee("BTC/USDT", None).await {
            Ok(fee) => {
                println!("   Symbol: {}", fee.symbol);
                println!("   Maker fee: {}", fee.maker);
                println!("   Taker fee: {}", fee.taker);
                println!("   ✅ Success\n");
            }
            Err(e) => println!("   ❌ Error: {}\n", e),
        }

        println!("2. Fetching trading fees for all symbols...");
        match binance.fetch_trading_fees(None, None).await {
            Ok(fees) => {
                println!("   Total symbols: {}", fees.len());
                for (symbol, fee) in fees.iter().take(5) {
                    println!("   {}: maker={}, taker={}", symbol, fee.maker, fee.taker);
                }
                println!("   ✅ Success\n");
            }
            Err(e) => println!("   ❌ Error: {}\n", e),
        }
    } else {
        println!("⚠️  No API credentials set, skipping authenticated endpoints");
        println!("   Set environment variables: BINANCE_API_KEY and BINANCE_API_SECRET\n");
    }

    // Note: The following methods are not yet migrated to the new modular structure:
    // - fetch_funding_rate
    // - fetch_funding_rate_history
    // - fetch_funding_rates
    // - fetch_leverage_tiers
    println!("Note: Funding rate and leverage tier methods are not yet migrated.");
    println!("      See rest_old.rs for the original implementations.\n");

    println!("=== Examples Complete ===");
    Ok(())
}
