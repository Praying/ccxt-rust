//! Binance Futures Transfer Example
//!
//! Demonstrates how to transfer assets between spot and futures accounts using
//! Binance's Futures Transfer API.
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
use std::collections::HashMap;
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    println!("=========================================");
    println!("Binance Futures Transfer Example");
    println!("=========================================\n");

    let api_key = env::var("BINANCE_API_KEY").ok();
    let secret = env::var("BINANCE_SECRET").ok();

    if api_key.is_none() || secret.is_none() {
        println!("⚠️  Warning: API keys not set, demonstrating code structure only");
        println!("Please set environment variables: BINANCE_API_KEY and BINANCE_SECRET");
        println!("\nContinuing with code demonstration...\n");
    }

    let config = ExchangeConfig {
        api_key,
        secret,
        ..Default::default()
    };

    let binance = Binance::new(config)?;

    // ============================================================================
    // Example 1: Transfer from Spot to USDT-M Futures
    // ============================================================================
    println!("📤 Example 1: Spot → USDT-M Futures (type=1)");
    println!("----------------------------------------");

    if binance.base().config.api_key.is_some() {
        println!("Transferring 50 USDT from Spot to USDT-M Futures account...");

        match binance.futures_transfer("USDT", 50.0, 1, None).await {
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
        println!("⚠️  Skipped (requires API keys)");
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
    }

    println!();

    // ============================================================================
    // Example 2: Transfer from USDT-M Futures to Spot
    // ============================================================================
    println!("📥 Example 2: USDT-M Futures → Spot (type=2)");
    println!("----------------------------------------");

    if binance.base().config.api_key.is_some() {
        println!("Transferring 30 USDT from USDT-M Futures to Spot account...");

        match binance.futures_transfer("USDT", 30.0, 2, None).await {
            Ok(transfer) => {
                println!("✅ Transfer successful!");
                println!("  Transfer ID: {:?}", transfer.id);
                println!("  Currency: {}", transfer.currency);
                println!("  Amount: {}", transfer.amount);
                println!("  From: {:?}", transfer.from_account);
                println!("  To: {:?}", transfer.to_account);
                println!("  Status: {}", transfer.status);
            }
            Err(e) => {
                println!("❌ Transfer failed: {}", e);
            }
        }
    } else {
        println!("⚠️  Skipped (requires API keys)");
        println!("Code example:");
        println!(
            r#"
// Transfer from USDT-M Futures back to Spot
let transfer = binance.futures_transfer(
    "USDT",
    30.0,
    2,       // type=2: USDT-M Futures → Spot
    None,
).await?;
        "#
        );
    }

    println!();

    // ============================================================================
    // Example 3: Transfer from Spot to COIN-M Futures
    // ============================================================================
    println!("📤 Example 3: Spot → COIN-M Futures (type=3)");
    println!("----------------------------------------");

    if binance.base().config.api_key.is_some() {
        println!("Transferring 0.01 BTC from Spot to COIN-M Futures account...");

        match binance.futures_transfer("BTC", 0.01, 3, None).await {
            Ok(transfer) => {
                println!("✅ Transfer successful!");
                println!("  Transfer ID: {:?}", transfer.id);
                println!("  Currency: {}", transfer.currency);
                println!("  Amount: {}", transfer.amount);
                println!("  From: {:?}", transfer.from_account);
                println!("  To: {:?}", transfer.to_account);
            }
            Err(e) => {
                println!("❌ Transfer failed: {}", e);
            }
        }
    } else {
        println!("⚠️  Skipped (requires API keys)");
        println!("Code example:");
        println!(
            r#"
// Transfer BTC to COIN-M Futures
let transfer = binance.futures_transfer(
    "BTC",
    0.01,
    3,       // type=3: Spot → COIN-M Futures
    None,
).await?;
        "#
        );
    }

    println!();

    // ============================================================================
    // Example 4: Transfer from COIN-M Futures to Spot
    // ============================================================================
    println!("📥 示例4: COIN-M期货转出到现货账户 (type=4)");
    println!("----------------------------------------");

    println!("Code example:");
    println!(
        r#"
// Transfer from COIN-M Futures back to Spot
let transfer = binance.futures_transfer(
    "BTC",
    0.005,
    4,       // type=4: COIN-M Futures → Spot
    None,
).await?;

println!("Transfer ID: {{:?}}", transfer.id);
    "#
    );

    println!();

    // ============================================================================
    // Example 5: Transfer Type Descriptions
    // ============================================================================
    println!("📋 Example 5: Futures Transfer Type Descriptions");
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

    // ============================================================================
    // Example 6: Query Futures Transfer History
    // ============================================================================
    println!("📜 Example 6: Query Futures Transfer History");
    println!("----------------------------------------");

    if binance.base().config.api_key.is_some() {
        println!("Querying recent USDT futures transfer records...");

        match binance
            .fetch_futures_transfers("USDT", None, Some(20), None)
            .await
        {
            Ok(transfers) => {
                println!(
                    "✅ Query successful! Found {} transfer records",
                    transfers.len()
                );

                if transfers.is_empty() {
                    println!("  No futures transfer records found");
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
                        if let Some(id) = &transfer.id {
                            println!("    Transfer ID: {}", id);
                        }
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
        println!("⚠️  Skipped (requires API keys)");
        println!("Code example:");
        println!(
            r#"
// Query recent 30 BTC futures transfers
let transfers = binance.fetch_futures_transfers(
    "BTC",     // Asset
    None,      // No time limit
    Some(30),  // Max 30 records
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
    // Example 7: Query Futures Transfers by Time Range
    // ============================================================================
    println!("📅 Example 7: Query Futures Transfers by Time Range");
    println!("----------------------------------------");

    if binance.base().config.api_key.is_some() {
        let seven_days_ago = chrono::Utc::now().timestamp_millis() as u64 - 7 * 86400_000;

        println!("Querying USDT futures transfers from the last 7 days...");

        match binance
            .fetch_futures_transfers("USDT", Some(seven_days_ago), Some(50), None)
            .await
        {
            Ok(transfers) => {
                println!("✅ Found {} records", transfers.len());

                let mut to_futures = 0;
                let mut from_futures = 0;
                let mut total_to_futures = 0.0;
                let mut total_from_futures = 0.0;

                for transfer in &transfers {
                    match transfer.to_account.as_ref().map(|s| s.as_str()) {
                        Some("future") | Some("delivery") => {
                            to_futures += 1;
                            total_to_futures += transfer.amount;
                        }
                        _ => {
                            from_futures += 1;
                            total_from_futures += transfer.amount;
                        }
                    }
                }

                if !transfers.is_empty() {
                    println!("\n  Statistics:");
                    println!(
                        "    To Futures: {} transfers, total {}",
                        to_futures, total_to_futures
                    );
                    println!(
                        "    From Futures: {} transfers, total {}",
                        from_futures, total_from_futures
                    );
                    println!("    Net Inflow: {}", total_to_futures - total_from_futures);
                }
            }
            Err(e) => {
                println!("❌ Query failed: {}", e);
            }
        }
    } else {
        println!("⚠️  Skipped (requires API keys)");
        println!("Code example:");
        println!(
            r#"
use chrono::Utc;

// Query futures transfers from the last 30 days
let thirty_days_ago = Utc::now().timestamp_millis() as u64 - 30 * 86400_000;

let transfers = binance.fetch_futures_transfers(
    "USDT",                // Asset
    Some(thirty_days_ago), // Start from 30 days ago
    Some(100),             // Max 100 records
    None,
).await?;

// Calculate net inflow
let net_flow: f64 = transfers.iter()
    .map(|t| {{
        match t.to_account.as_ref().map(|s| s.as_str()) {{
            Some("future") | Some("delivery") => t.amount,
            _ => -t.amount,
        }}
    }})
    .sum();

println!("Futures account net inflow: {{}}", net_flow);
        "#
        );
    }

    println!();

    // ============================================================================
    // Example 8: Advanced Query - Using Additional Parameters
    // ============================================================================
    println!("🔍 Example 8: Advanced Query - Using Additional Parameters");
    println!("----------------------------------------");

    println!("Code example:");
    println!(
        r#"
use std::collections::HashMap;

// Paginated query using additional parameters
let mut params = HashMap::new();
params.insert("current".to_string(), "2".to_string());  // Page 2
params.insert("size".to_string(), "50".to_string());    // 50 per page

let transfers = binance.fetch_futures_transfers(
    "USDT",
    None,
    None,
    Some(params),
).await?;

println!("Page 2, total {{}} records", transfers.len());
    "#
    );

    println!();

    // ============================================================================
    // Example 9: Error Handling
    // ============================================================================
    println!("⚠️  Example 9: Error Handling");
    println!("----------------------------------------");

    println!("Common errors and handling methods:");
    println!(
        r#"
1. Invalid transfer type (type must be 1-4)
   match binance.futures_transfer("USDT", 100.0, 5, None).await {{
       Err(e) if e.to_string().contains("type must be") => {{
           println!("Invalid transfer type");
       }}
       Ok(transfer) => {{ /* Handle success */ }}
       Err(e) => {{ /* Other errors */ }}
   }}

2. Insufficient balance
   match binance.futures_transfer("USDT", 10000.0, 1, None).await {{
       Err(e) if e.to_string().contains("insufficient") => {{
           println!("Insufficient balance");
       }}
       Ok(transfer) => {{ /* Handle success */ }}
       Err(e) => {{ /* Other errors */ }}
   }}

3. Asset does not exist
   match binance.futures_transfer("INVALID", 100.0, 1, None).await {{
       Err(e) if e.to_string().contains("asset") => {{
           println!("Asset does not exist");
       }}
       Ok(transfer) => {{ /* Handle success */ }}
       Err(e) => {{ /* Other errors */ }}
   }}
    "#
    );

    println!();

    println!("=================================");
    println!("Example execution completed!");
    println!("=================================");

    Ok(())
}
