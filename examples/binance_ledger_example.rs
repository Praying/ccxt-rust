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
use std::collections::HashMap;
use std::env;

#[tokio::main]
async fn main() -> Result<()> {
    let api_key = env::var("BINANCE_API_KEY").expect("Set BINANCE_API_KEY environment variable");
    let secret = env::var("BINANCE_SECRET").expect("Set BINANCE_SECRET environment variable");

    let exchange = ccxt_exchanges::binance::Binance::new(ExchangeConfig {
        api_key: Some(api_key),
        secret: Some(secret),
        ..Default::default()
    })?;

    println!("=== Binance Ledger Query Example ===\n");

    // ========================================================================
    // Example 1: Query USDT-M Futures Ledger (Last 10 Records)
    // ========================================================================
    println!("1. Query USDT-M Futures Ledger (Last 10 Records)");
    println!("{}", "-".repeat(60));

    let mut params = HashMap::new();
    params.insert("type".to_string(), "future".to_string());
    params.insert("limit".to_string(), "10".to_string());

    match exchange
        .fetch_ledger(None, None, Some(10), Some(params.clone()))
        .await
    {
        Ok(entries) => {
            println!("Query successful! {} records found\n", entries.len());
            for (i, entry) in entries.iter().enumerate() {
                println!("Record #{}", i + 1);
                println!("  ID: {}", entry.id);
                println!("  Time: {}", entry.datetime);
                println!("  Currency: {}", entry.currency);
                println!("  Amount: {}", entry.amount);
                println!("  Balance: {}", entry.after.unwrap_or(0.0));
                println!("  Direction: {:?}", entry.direction);
                println!("  Type: {:?}", entry.type_);
                if let Some(ref_id) = &entry.reference_id {
                    println!("  Reference ID: {}", ref_id);
                }
                println!();
            }
        }
        Err(e) => {
            eprintln!("Query failed: {}", e);
        }
    }

    // ========================================================================
    // Example 2: Query COIN-M Futures Ledger (Specific Currency)
    // ========================================================================
    println!("\n2. Query COIN-M Futures Ledger (BTC)");
    println!("{}", "-".repeat(60));

    params.clear();
    params.insert("type".to_string(), "delivery".to_string());
    params.insert("limit".to_string(), "5".to_string());

    match exchange
        .fetch_ledger(Some("BTC".to_string()), None, Some(5), Some(params.clone()))
        .await
    {
        Ok(entries) => {
            println!("Query successful! {} BTC records found\n", entries.len());
            for (i, entry) in entries.iter().enumerate() {
                println!(
                    "Record #{}: {} {} {:?} @ {}",
                    i + 1,
                    entry.amount,
                    entry.currency,
                    entry.type_,
                    entry.datetime
                );
            }
        }
        Err(e) => {
            eprintln!("Query failed: {}", e);
        }
    }

    // ========================================================================
    // Example 3: Query Options Ledger
    // ========================================================================
    println!("\n3. Query Options Ledger");
    println!("{}", "-".repeat(60));

    params.clear();
    params.insert("type".to_string(), "option".to_string());
    params.insert("limit".to_string(), "5".to_string());

    match exchange
        .fetch_ledger(None, None, Some(5), Some(params.clone()))
        .await
    {
        Ok(entries) => {
            println!(
                "Query successful! {} options records found\n",
                entries.len()
            );
            for entry in entries.iter() {
                println!(
                    "{}: {} {} {:?}",
                    entry.datetime, entry.amount, entry.currency, entry.type_
                );
            }
        }
        Err(e) => {
            eprintln!("Query failed: {}", e);
        }
    }

    // ========================================================================
    // Example 4: Time Range Query
    // ========================================================================
    println!("\n4. Query Ledger Within Time Range");
    println!("{}", "-".repeat(60));

    let now = chrono::Utc::now().timestamp_millis();
    let seven_days_ago = now - 7 * 24 * 60 * 60 * 1000;

    params.clear();
    params.insert("type".to_string(), "future".to_string());
    params.insert("startTime".to_string(), seven_days_ago.to_string());
    params.insert("endTime".to_string(), now.to_string());
    params.insert("limit".to_string(), "20".to_string());

    match exchange
        .fetch_ledger(None, None, Some(20), Some(params.clone()))
        .await
    {
        Ok(entries) => {
            println!(
                "Query successful! {} records in last 7 days\n",
                entries.len()
            );

            let mut type_counts: HashMap<String, usize> = HashMap::new();
            for entry in &entries {
                let type_name = format!("{:?}", entry.type_);
                *type_counts.entry(type_name).or_insert(0) += 1;
            }

            println!("Type Distribution:");
            for (entry_type, count) in type_counts.iter() {
                println!("  {}: {} entries", entry_type, count);
            }
        }
        Err(e) => {
            eprintln!("Query failed: {}", e);
        }
    }

    // ========================================================================
    // Example 5: Portfolio Margin Account Query
    // ========================================================================
    println!("\n5. Query Portfolio Margin Account");
    println!("{}", "-".repeat(60));

    params.clear();
    params.insert("type".to_string(), "future".to_string());
    params.insert("portfolioMargin".to_string(), "true".to_string());
    params.insert("limit".to_string(), "5".to_string());

    match exchange
        .fetch_ledger(None, None, Some(5), Some(params))
        .await
    {
        Ok(entries) => {
            println!(
                "Query successful! {} portfolio margin records",
                entries.len()
            );
            for entry in entries.iter() {
                println!("  {}: {} {}", entry.datetime, entry.amount, entry.currency);
            }
        }
        Err(e) => {
            eprintln!(
                "Query failed: {} (account may not have portfolio margin enabled)",
                e
            );
        }
    }

    // ========================================================================
    // Example 6: Unsupported Operation (Spot Ledger)
    // ========================================================================
    println!("\n6. Attempt to Query Spot Ledger (Expected to Fail)");
    println!("{}", "-".repeat(60));

    let mut spot_params = HashMap::new();
    spot_params.insert("type".to_string(), "spot".to_string());

    match exchange
        .fetch_ledger(None, None, Some(10), Some(spot_params))
        .await
    {
        Ok(_) => {
            println!("Unexpected success (should not happen)");
        }
        Err(e) => {
            println!("Expected error: {}", e);
            println!("(Spot wallet does not support ledger query)");
        }
    }

    println!("\n=== Example Complete ===");
    Ok(())
}
