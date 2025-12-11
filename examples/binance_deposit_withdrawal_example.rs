//! Binance Deposit and Withdrawal Example
//!
//! Demonstrates usage of Binance deposit and withdrawal APIs:
//! - `withdraw`: Withdraw funds to external address
//! - `fetch_deposits`: Query deposit history
//! - `fetch_withdrawals`: Query withdrawal history
//! - `fetch_deposit_address`: Get deposit address for a currency
//!
//! # Prerequisites
//!
//! Set valid API credentials in environment variables:
//! - `BINANCE_API_KEY`
//! - `BINANCE_SECRET`
//!
// Allow clippy warnings for example code - examples prioritize readability over strict linting
#![allow(clippy::field_reassign_with_default)]
#![allow(clippy::collapsible_if)]

//! # Examples
//!
//! Run this example:
//! ```bash
//! cargo run --example binance_deposit_withdrawal_example
//! ```

use ccxt_core::ExchangeConfig;
use ccxt_exchanges::binance::Binance;
use std::collections::HashMap;
use std::env;

#[tokio::main]
async fn main() -> ccxt_core::Result<()> {
    let api_key = env::var("BINANCE_API_KEY").unwrap_or_else(|_| "your-api-key".to_string());
    let secret = env::var("BINANCE_SECRET").unwrap_or_else(|_| "your-secret".to_string());

    let mut config = ExchangeConfig::default();
    config.api_key = Some(api_key);
    config.secret = Some(secret);

    let binance = Binance::new(config)?;

    println!("=== Binance Deposit and Withdrawal Example ===\n");

    // ============================================================================
    // 1. Fetch deposit addresses
    // ============================================================================
    println!("📍 1. Fetch Deposit Addresses");
    println!("{}", "=".repeat(60));

    match binance.fetch_deposit_address("BTC", None).await {
        Ok(addr) => {
            println!("✅ BTC Deposit Address:");
            println!("   Currency: {}", addr.currency);
            println!("   Address: {}", addr.address);
            if let Some(network) = &addr.network {
                println!("   Network: {}", network);
            }
            if let Some(tag) = &addr.tag {
                println!("   Tag: {}", tag);
            }
        }
        Err(e) => println!("❌ Failed to fetch BTC deposit address: {}", e),
    }

    println!();

    let mut params = HashMap::new();
    params.insert("network".to_string(), "TRX".to_string());

    match binance.fetch_deposit_address("USDT", Some(params)).await {
        Ok(addr) => {
            println!("✅ USDT-TRX Deposit Address:");
            println!("   Currency: {}", addr.currency);
            println!("   Address: {}", addr.address);
            if let Some(network) = &addr.network {
                println!("   Network: {}", network);
            }
        }
        Err(e) => println!("❌ Failed to fetch USDT-TRX deposit address: {}", e),
    }

    println!("\n");

    // ============================================================================
    // 2. Query deposit history
    // ============================================================================
    println!("💰 2. Query Deposit History");
    println!("{}", "=".repeat(60));

    match binance.fetch_deposits(None, None, Some(100), None).await {
        Ok(deposits) => {
            println!("✅ Total Deposits: {}", deposits.len());
            for (i, deposit) in deposits.iter().take(5).enumerate() {
                println!("\nDeposit #{}:", i + 1);
                println!("   ID: {}", deposit.id);
                println!("   Currency: {}", deposit.currency);
                println!("   Amount: {}", deposit.amount);
                println!("   Status: {:?}", deposit.status);
                if let Some(txid) = &deposit.txid {
                    println!("   TX Hash: {}", txid);
                }
                if let Some(network) = &deposit.network {
                    println!("   Network: {}", network);
                }
                if let Some(address) = &deposit.address {
                    println!("   Address: {}", address);
                }
                if let Some(timestamp) = deposit.timestamp {
                    println!("   Timestamp: {}", timestamp);
                }
                if let Some(fee) = &deposit.fee {
                    println!("   Fee: {} {}", fee.cost, fee.currency);
                }
            }

            if deposits.len() > 5 {
                println!("\n   ... {} more records not shown", deposits.len() - 5);
            }
        }
        Err(e) => println!("❌ Failed to fetch deposits: {}", e),
    }

    println!();

    match binance
        .fetch_deposits(Some("BTC"), None, Some(10), None)
        .await
    {
        Ok(deposits) => {
            println!("✅ BTC Deposits: {}", deposits.len());
            for deposit in deposits.iter().take(3) {
                println!("   - {} BTC ({})", deposit.amount, deposit.id);
            }
        }
        Err(e) => println!("❌ Failed to fetch BTC deposits: {}", e),
    }

    println!("\n");

    // ============================================================================
    // 3. Query withdrawal history
    // ============================================================================
    println!("💸 3. Query Withdrawal History");
    println!("{}", "=".repeat(60));

    match binance.fetch_withdrawals(None, None, Some(100), None).await {
        Ok(withdrawals) => {
            println!("✅ Total Withdrawals: {}", withdrawals.len());
            for (i, withdrawal) in withdrawals.iter().take(5).enumerate() {
                println!("\nWithdrawal #{}:", i + 1);
                println!("   ID: {}", withdrawal.id);
                println!("   Currency: {}", withdrawal.currency);
                println!("   Amount: {}", withdrawal.amount);
                println!("   Status: {:?}", withdrawal.status);
                if let Some(txid) = &withdrawal.txid {
                    println!("   TX Hash: {}", txid);
                }
                if let Some(network) = &withdrawal.network {
                    println!("   Network: {}", network);
                }
                if let Some(address) = &withdrawal.address {
                    println!("   Address: {}", address);
                }
                if let Some(timestamp) = withdrawal.timestamp {
                    println!("   Timestamp: {}", timestamp);
                }
                if let Some(fee) = &withdrawal.fee {
                    println!("   Fee: {} {}", fee.cost, fee.currency);
                }
                if let Some(internal) = withdrawal.internal {
                    if internal {
                        println!("   Type: Internal transfer");
                    }
                }
            }

            if withdrawals.len() > 5 {
                println!("\n   ... {} more records not shown", withdrawals.len() - 5);
            }
        }
        Err(e) => println!("❌ Failed to fetch withdrawals: {}", e),
    }

    println!();

    match binance
        .fetch_withdrawals(Some("USDT"), None, Some(10), None)
        .await
    {
        Ok(withdrawals) => {
            println!("✅ USDT Withdrawals: {}", withdrawals.len());
            for withdrawal in withdrawals.iter().take(3) {
                println!("   - {} USDT ({})", withdrawal.amount, withdrawal.id);
            }
        }
        Err(e) => println!("❌ Failed to fetch USDT withdrawals: {}", e),
    }

    println!("\n");

    // ============================================================================
    // 4. Withdrawal operations (demonstration only, not executed)
    // ============================================================================
    println!("🚀 4. Withdrawal Operations (Demo)");
    println!("{}", "=".repeat(60));

    println!(
        "⚠️  Warning: Withdrawal operations require caution. The following are example codes only and will not be executed.\n"
    );

    println!("Example 1: Basic withdrawal");
    println!("```rust");
    println!("let tx = binance.withdraw(");
    println!("    \"USDT\",");
    println!("    \"100.0\",");
    println!("    \"TXxxxxxxxxxxxxxxxxxxxxxxxxxxx\",");
    println!("    None");
    println!(").await?;");
    println!("println!(\"Withdrawal ID: {{}}\", tx.id);");
    println!("```\n");

    println!("Example 2: Withdrawal with specific network");
    println!("```rust");
    println!("let mut params = HashMap::new();");
    println!("params.insert(\"network\".to_string(), \"TRX\".to_string());");
    println!("let tx = binance.withdraw(");
    println!("    \"USDT\",");
    println!("    \"100.0\",");
    println!("    \"TXxxxxxxxxxxxxxxxxxxxxxxxxxxx\",");
    println!("    Some(params)");
    println!(").await?;");
    println!("```\n");

    println!("Example 3: Withdrawal with tag (e.g., XRP)");
    println!("```rust");
    println!("let mut params = HashMap::new();");
    println!("params.insert(\"tag\".to_string(), \"123456\".to_string());");
    println!("let tx = binance.withdraw(");
    println!("    \"XRP\",");
    println!("    \"50.0\",");
    println!("    \"rXXXXXXXXXXXXXXXXXXXXXXXXXXX\",");
    println!("    Some(params)");
    println!(").await?;");
    println!("```");

    println!("\n");

    // ============================================================================
    // Statistics
    // ============================================================================
    println!("📊 Statistics");
    println!("{}", "=".repeat(60));

    match binance.fetch_deposits(None, None, Some(1000), None).await {
        Ok(deposits) => {
            let mut currency_stats: HashMap<String, (usize, rust_decimal::Decimal)> =
                HashMap::new();

            for deposit in deposits.iter() {
                let entry = currency_stats
                    .entry(deposit.currency.clone())
                    .or_insert((0, rust_decimal::Decimal::ZERO));
                entry.0 += 1;
                entry.1 += deposit.amount;
            }

            println!("Deposit statistics (by currency):");
            for (currency, (count, total)) in currency_stats.iter() {
                println!("   {}: {} transactions, Total {}", currency, count, total);
            }
        }
        Err(e) => println!("❌ Failed to fetch deposit statistics: {}", e),
    }

    println!("\n");
    println!("✅ Deposit and withdrawal example completed!");

    Ok(())
}
