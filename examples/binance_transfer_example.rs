//! Binance internal transfer functionality example.
//!
//! Demonstrates how to use Binance's internal transfer API to move assets between
//! different account types (spot, futures, margin, funding).
//!
//! # Running the Example
//!
//! ```bash
//! # Set environment variables (or configure directly in code)
//! export BINANCE_API_KEY="your_api_key"
//! export BINANCE_SECRET="your_secret"
//!
//! # Run the example
//! cargo run --example binance_transfer_example
//! ```

// Allow clippy warnings for example code - examples prioritize readability over strict linting
#![allow(clippy::inconsistent_digit_grouping)]

use ccxt_core::ExchangeConfig;
use ccxt_exchanges::binance::Binance;
use std::collections::HashMap;
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    println!("=================================");
    println!("Binance Internal Transfer Example");
    println!("=================================\n");

    let api_key = env::var("BINANCE_API_KEY").ok();
    let secret = env::var("BINANCE_SECRET").ok();

    if api_key.is_none() || secret.is_none() {
        println!("⚠️  Warning: API credentials not set, demonstrating code structure only");
        println!("Set environment variables: BINANCE_API_KEY and BINANCE_SECRET");
        println!("\nContinuing with demo code...\n");
    }

    let config = ExchangeConfig {
        api_key,
        secret,
        ..Default::default()
    };

    let binance = Binance::new(config)?;

    // ============================================================================
    // Example 1: Execute Internal Transfer
    // ============================================================================
    println!("📤 Example 1: Execute Internal Transfer");
    println!("----------------------------------------");

    if binance.base().config.api_key.is_some() {
        println!("Transferring 100 USDT from spot to futures account...");

        match binance
            .transfer("USDT", 100.0, "spot", "future", None)
            .await
        {
            Ok(transfer) => {
                println!("✅ Transfer successful!");
                println!("  Transfer ID: {:?}", transfer.id);
                println!("  Currency: {}", transfer.currency);
                println!("  Amount: {}", transfer.amount);
                println!("  From: {:?}", transfer.from_account);
                println!("  To: {:?}", transfer.to_account);
                println!("  Status: {}", transfer.status);
                println!("  Time: {}", transfer.datetime);
            }
            Err(e) => {
                println!("❌ Transfer failed: {}", e);
            }
        }
    } else {
        println!("⚠️  Skipped (API key required)");
        println!("Code example:");
        println!(
            r#"
let transfer = binance.transfer(
    "USDT",      // Asset code
    100.0,       // Amount
    "spot",      // From spot account
    "future",    // To futures account
    None,        // Optional parameters
).await?;

println!("Transfer ID: {{:?}}", transfer.id);
println!("Status: {{}}", transfer.status);
        "#
        );
    }

    println!();

    // ============================================================================
    // Example 2: Different Transfer Types
    // ============================================================================
    println!("📤 Example 2: Different Transfer Types");
    println!("----------------------------------------");

    let transfer_examples = vec![
        ("Spot → Futures", "spot", "future"),
        ("Futures → Spot", "future", "spot"),
        ("Spot → Margin", "spot", "margin"),
        ("Margin → Spot", "margin", "spot"),
        ("Spot → Funding", "spot", "funding"),
        ("Funding → Spot", "funding", "spot"),
    ];

    for (desc, from, to) in transfer_examples {
        println!("  {} ({} → {})", desc, from, to);
    }

    println!("\nCode example:");
    println!(
        r#"
// Transfer from futures back to spot
let transfer = binance.transfer(
    "USDT",
    50.0,
    "future",    // From futures
    "spot",      // To spot
    None,
).await?;
    "#
    );

    println!();

    // ============================================================================
    // Example 3: Query Transfer History
    // ============================================================================
    println!("📜 Example 3: Query Transfer History");
    println!("----------------------------------------");

    if binance.base().config.api_key.is_some() {
        println!("Querying recent USDT transfer records...");

        match binance
            .fetch_transfers(Some("USDT"), None, Some(10), None)
            .await
        {
            Ok(transfers) => {
                println!(
                    "✅ Query successful! Found {} transfer records",
                    transfers.len()
                );

                if transfers.is_empty() {
                    println!("  No transfer records found");
                } else {
                    for (i, transfer) in transfers.iter().enumerate().take(5) {
                        println!("\n  Transfer #{}:", i + 1);
                        println!("    Time: {}", transfer.datetime);
                        println!("    Currency: {}", transfer.currency);
                        println!("    Amount: {}", transfer.amount);
                        println!(
                            "    Direction: {:?} → {:?}",
                            transfer.from_account, transfer.to_account
                        );
                        println!("    Status: {}", transfer.status);
                    }

                    if transfers.len() > 5 {
                        println!("\n  ... {} more records", transfers.len() - 5);
                    }
                }
            }
            Err(e) => {
                println!("❌ Query failed: {}", e);
            }
        }
    } else {
        println!("⚠️  Skipped (API key required)");
        println!("Code example:");
        println!(
            r#"
// Query recent 50 BTC transfers
let transfers = binance.fetch_transfers(
    Some("BTC"),   // BTC only
    None,          // No time limit
    Some(50),      // Max 50 records
    None,
).await?;

for transfer in transfers {{
    println!("{{}} {{}} from {{:?}} to {{:?}}",
        transfer.amount,
        transfer.currency,
        transfer.from_account,
        transfer.to_account
    );
}}
        "#
        );
    }

    println!();

    // ============================================================================
    // Example 4: Query Transfers by Time Range
    // ============================================================================
    println!("📅 Example 4: Query Transfers by Time Range");
    println!("----------------------------------------");

    if binance.base().config.api_key.is_some() {
        let one_day_ago = chrono::Utc::now().timestamp_millis() as u64 - 86400_000;

        println!("Querying transfer records from the last 24 hours...");

        match binance
            .fetch_transfers(None, Some(one_day_ago), Some(50), None)
            .await
        {
            Ok(transfers) => {
                println!("✅ Found {} records", transfers.len());

                let mut by_currency: HashMap<String, (usize, f64)> = HashMap::new();
                for transfer in &transfers {
                    let entry = by_currency
                        .entry(transfer.currency.clone())
                        .or_insert((0, 0.0));
                    entry.0 += 1;
                    entry.1 += transfer.amount;
                }

                if !by_currency.is_empty() {
                    println!("\n  Statistics by currency:");
                    for (currency, (count, total)) in by_currency {
                        println!("    {}: {} transfers, total {}", currency, count, total);
                    }
                }
            }
            Err(e) => {
                println!("❌ Query failed: {}", e);
            }
        }
    } else {
        println!("⚠️  Skipped (API key required)");
        println!("Code example:");
        println!(
            r#"
use chrono::Utc;

// Query transfers from the last 7 days
let seven_days_ago = Utc::now().timestamp_millis() as u64 - 7 * 86400_000;

let transfers = binance.fetch_transfers(
    None,                // All currencies
    Some(seven_days_ago), // Start from 7 days ago
    Some(100),           // Max 100 records
    None,
).await?;
        "#
        );
    }

    println!();

    // ============================================================================
    // Example 5: Query Deposit/Withdrawal Fees
    // ============================================================================
    println!("💰 Example 5: Query Deposit/Withdrawal Fees");
    println!("----------------------------------------");

    if binance.base().config.api_key.is_some() {
        println!("Querying USDT deposit/withdrawal fee information...");

        match binance
            .fetch_deposit_withdraw_fees(Some("USDT"), None)
            .await
        {
            Ok(fees) => {
                println!("✅ Query successful!");

                for fee in fees {
                    println!("\n  Currency: {}", fee.currency);
                    println!("  Withdrawal fee: {}", fee.withdraw_fee);
                    println!("  Min withdrawal: {}", fee.withdraw_min);
                    println!("  Max withdrawal: {}", fee.withdraw_max);
                    println!(
                        "  Deposit enabled: {}",
                        if fee.deposit_enable { "Yes" } else { "No" }
                    );
                    println!(
                        "  Withdrawal enabled: {}",
                        if fee.withdraw_enable { "Yes" } else { "No" }
                    );

                    if !fee.networks.is_empty() {
                        println!("\n  Supported networks:");
                        for network in &fee.networks {
                            println!("    {} ({})", network.network, network.name);
                            println!("      Fee: {}", network.withdraw_fee);
                            println!("      Min: {}", network.withdraw_min);
                            println!("      Max: {}", network.withdraw_max);
                        }
                    }
                }
            }
            Err(e) => {
                println!("❌ Query failed: {}", e);
            }
        }
    } else {
        println!("⚠️  Skipped (API key required)");
        println!("Code example:");
        println!(
            r#"
// Query fees for specific currency
let fees = binance.fetch_deposit_withdraw_fees(
    Some("BTC"),
    None,
).await?;

for fee in fees {{
    println!("{{}}: withdrawal fee = {{}}", fee.currency, fee.withdraw_fee);
    
    for network in &fee.networks {{
        println!("  Network {{}}: {{}}", network.network, network.withdraw_fee);
    }}
}}
        "#
        );
    }

    println!();

    // ============================================================================
    // Example 6: Query All Currency Fees
    // ============================================================================
    println!("💰 Example 6: Query All Currency Fees");
    println!("----------------------------------------");

    if binance.base().config.api_key.is_some() {
        println!("Querying deposit/withdrawal fees for all currencies...");

        match binance.fetch_deposit_withdraw_fees(None, None).await {
            Ok(fees) => {
                println!("✅ Query successful! Total {} currencies", fees.len());

                println!("\n  First 10 currencies:");
                for fee in fees.iter().take(10) {
                    println!(
                        "    {}: withdrawal fee {} (min {}, max {})",
                        fee.currency, fee.withdraw_fee, fee.withdraw_min, fee.withdraw_max
                    );
                }

                if fees.len() > 10 {
                    println!("\n  ... {} more currencies", fees.len() - 10);
                }
            }
            Err(e) => {
                println!("❌ Query failed: {}", e);
            }
        }
    } else {
        println!("⚠️  Skipped (API key required)");
        println!("Code example:");
        println!(
            r#"
// Query all currencies
let all_fees = binance.fetch_deposit_withdraw_fees(None, None).await?;

println!("Total {{}} currencies", all_fees.len());

// Find currency with lowest withdrawal fee
let min_fee = all_fees.iter()
    .filter(|f| f.withdraw_enable)
    .min_by(|a, b| a.withdraw_fee.partial_cmp(&b.withdraw_fee).unwrap());
        "#
        );
    }

    println!();
    println!("=================================");
    println!("Example completed!");
    println!("=================================");

    Ok(())
}
