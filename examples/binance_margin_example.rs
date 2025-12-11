//! Binance Margin Trading API Examples
//!
//! This example demonstrates Binance margin trading features:
//! - Query borrow interest rates
//! - Execute borrow operations
//! - Execute repay operations
//! - Query historical records
//!
//! # Usage
//!
//! ```bash
//! export BINANCE_API_KEY="your_api_key"
//! export BINANCE_SECRET="your_secret"
//! cargo run --example binance_margin_example
//! ```

use ccxt_core::ExchangeConfig;
use ccxt_exchanges::binance::Binance;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let api_key =
        std::env::var("BINANCE_API_KEY").expect("BINANCE_API_KEY environment variable must be set");
    let secret =
        std::env::var("BINANCE_SECRET").expect("BINANCE_SECRET environment variable must be set");

    println!("🚀 Binance Margin Trading API Examples");
    println!("{}", "=".repeat(60));

    let config = ExchangeConfig {
        api_key: Some(api_key),
        secret: Some(secret),
        ..Default::default()
    };
    let binance = Binance::new(config)?;

    // ========================================================================
    // Example 1: Query cross margin borrow rates
    // ========================================================================
    println!("\n📊 Example 1: Query Cross Margin Borrow Rates");
    println!("{}", "-".repeat(60));

    match binance.fetch_isolated_borrow_rates(None).await {
        Ok(rates) => {
            println!("✅ BTC Cross Margin Borrow Rates:");
            for (symbol, rate) in rates.iter() {
                println!("   Symbol: {}", symbol);
                println!("   Base Currency Daily Rate: {}%", rate.base_rate * 100.0);
                println!("   Quote Currency Daily Rate: {}%", rate.quote_rate * 100.0);
                println!("   Time: {:?}", rate.datetime);
            }
        }
        Err(e) => {
            println!("❌ Query failed: {}", e);
        }
    }

    // ========================================================================
    // Example 2: Query isolated margin borrow rates
    // ========================================================================
    println!("\n📊 Example 2: Query Isolated Margin Borrow Rates");
    println!("{}", "-".repeat(60));

    match binance.fetch_isolated_borrow_rates(None).await {
        Ok(rates) => {
            println!("✅ BTC/USDT Isolated Margin Borrow Rates:");
            for (symbol, rate) in rates.iter() {
                println!("   Symbol: {}", symbol);
                println!("   Base Currency Daily Rate: {}%", rate.base_rate * 100.0);
                println!("   Quote Currency Daily Rate: {}%", rate.quote_rate * 100.0);
                println!("   Time: {:?}", rate.datetime);
            }
        }
        Err(e) => {
            println!("❌ Query failed: {}", e);
        }
    }

    // ========================================================================
    // Example 3: Batch query borrow rates
    // ========================================================================
    println!("\n📊 Example 3: Batch Query Borrow Rates");
    println!("{}", "-".repeat(60));

    match binance
        .fetch_borrow_rate_history("BTC", None, Some(10), None)
        .await
    {
        Ok(history) => {
            println!("✅ BTC borrow rate history retrieved successfully (last 10 records)");
            for (i, record) in history.iter().enumerate().take(5) {
                println!(
                    "  Record {}: Currency={}, Rate={}, Timestamp={}",
                    i + 1,
                    record.currency,
                    record.rate,
                    record.timestamp
                );
            }
        }
        Err(e) => {
            println!("❌ Query failed: {}", e);
        }
    }

    // ========================================================================
    // Example 4: Query borrow rate history
    // ========================================================================
    println!("\n📊 Example 4: Query Borrow Rate History");
    println!("{}", "-".repeat(60));

    match binance
        .fetch_borrow_rate_history("BTC", None, Some(5), None)
        .await
    {
        Ok(history) => {
            println!("✅ BTC Borrow Rate History (last 5 records):");
            for record in history {
                println!(
                    "   Date: {} | Rate: {}%",
                    record.datetime,
                    record.rate * 100.0
                );
            }
        }
        Err(e) => {
            println!("❌ Query failed: {}", e);
        }
    }

    // ========================================================================
    // Example 5: Query borrow interest history (cross margin)
    // ========================================================================
    println!("\n📊 Example 5: Query Borrow Interest History (Cross Margin)");
    println!("{}", "-".repeat(60));

    match binance
        .fetch_borrow_interest(None, None, None, Some(5), None)
        .await
    {
        Ok(interests) => {
            println!("✅ Cross Margin Borrow Interest History (last 5 records):");
            for record in interests {
                println!(
                    "   Currency: {} | Principal: {} | Interest: {} | Rate: {}%",
                    record.currency,
                    record.principal,
                    record.interest,
                    record.interest_rate * 100.0
                );
            }
        }
        Err(e) => {
            println!("❌ Query failed: {}", e);
        }
    }

    // ========================================================================
    // Example 6: Query borrow interest history (isolated margin)
    // ========================================================================
    println!("\n📊 Example 6: Query Borrow Interest History (Isolated Margin)");
    println!("{}", "-".repeat(60));

    match binance
        .fetch_borrow_interest(None, Some("BTC/USDT"), None, Some(5), None)
        .await
    {
        Ok(interests) => {
            println!("✅ BTC/USDT Isolated Margin Borrow Interest History (last 5 records):");
            for record in interests {
                println!(
                    "   Symbol: {:?} | Currency: {} | Interest: {}",
                    record.symbol, record.currency, record.interest
                );
            }
        }
        Err(e) => {
            println!("❌ Query failed: {}", e);
        }
    }

    // ========================================================================
    // Example 7: Query margin adjustment history
    // ========================================================================
    println!("\n📊 Example 7: Query Margin Adjustment History");
    println!("{}", "-".repeat(60));

    match binance
        .fetch_margin_adjustment_history(Some("BTC/USDT"), None, Some(5))
        .await
    {
        Ok(adjustments) => {
            println!("✅ BTC/USDT Margin Adjustment History (last 5 records):");
            for record in adjustments {
                println!(
                    "   Type: {} | Amount: {} | Time: {}",
                    record.transfer_type, record.amount, record.datetime
                );
            }
        }
        Err(e) => {
            println!("❌ Query failed: {}", e);
        }
    }

    // ========================================================================
    // Example 8: Cross margin borrow (demo code, not executed by default)
    // ========================================================================
    println!("\n💰 Example 8: Cross Margin Borrow Operation (Demo Code)");
    println!("{}", "-".repeat(60));
    println!("⚠️  This operation will perform real borrowing, not executed by default");
    println!("ℹ️  To execute, uncomment the code below");

    /*
    let mut params = HashMap::new();
    // Optional: Set whether this is isolated margin borrowing
    // params.insert("isIsolated".to_string(), "FALSE".to_string());

    match binance.borrow_cross_margin("USDT", "10.0", 10.0, params).await {
        Ok(loan) => {
            println!("✅ Borrow successful:");
            println!("   Transaction ID: {:?}", loan.id);
            println!("   Currency: {}", loan.currency);
            println!("   Amount: {}", loan.amount);
            println!("   Time: {}", loan.datetime);
        }
        Err(e) => {
            println!("❌ Borrow failed: {}", e);
        }
    }
    */

    // ========================================================================
    // Example 9: Isolated margin borrow (demo code, not executed by default)
    // ========================================================================
    println!("\n💰 Example 9: Isolated Margin Borrow Operation (Demo Code)");
    println!("{}", "-".repeat(60));
    println!("⚠️  This operation will perform real borrowing, not executed by default");

    /*
    match binance.borrow_isolated_margin("BTC/USDT", "USDT", "10.0", 10.0, HashMap::new()).await {
        Ok(loan) => {
            println!("✅ Borrow successful:");
            println!("   Symbol: {:?}", loan.symbol);
            println!("   Currency: {}", loan.currency);
            println!("   Amount: {}", loan.amount);
        }
        Err(e) => {
            println!("❌ Borrow failed: {}", e);
        }
    }
    */

    // ========================================================================
    // Example 10: Cross margin repay (demo code, not executed by default)
    // ========================================================================
    println!("\n💸 Example 10: Cross Margin Repay Operation (Demo Code)");
    println!("{}", "-".repeat(60));
    println!("⚠️  This operation will perform real repayment, not executed by default");

    /*
    match binance.repay_cross_margin("USDT", "10.0", 10.0, HashMap::new()).await {
        Ok(repay) => {
            println!("✅ Repay successful:");
            println!("   Transaction ID: {:?}", repay.id);
            println!("   Currency: {}", repay.currency);
            println!("   Amount: {}", repay.amount);
        }
        Err(e) => {
            println!("❌ Repay failed: {}", e);
        }
    }
    */

    // ========================================================================
    // Example 11: Isolated margin repay (demo code, not executed by default)
    // ========================================================================
    println!("\n💸 Example 11: Isolated Margin Repay Operation (Demo Code)");
    println!("{}", "-".repeat(60));
    println!("⚠️  This operation will perform real repayment, not executed by default");

    /*
    match binance.repay_isolated_margin("BTC/USDT", "USDT", "10.0", 10.0, HashMap::new()).await {
        Ok(repay) => {
            println!("✅ Repay successful:");
            println!("   Symbol: {:?}", repay.symbol);
            println!("   Currency: {}", repay.currency);
            println!("   Amount: {}", repay.amount);
        }
        Err(e) => {
            println!("❌ Repay failed: {}", e);
        }
    }
    */

    println!();
    println!("{}", "=".repeat(60));
    println!("✨ Examples completed successfully!");
    println!();
    println!("💡 Tips:");
    println!("   1. Borrow/repay operations require sufficient account balance");
    println!("   2. Different VIP levels have different borrow rates");
    println!("   3. 全仓保证金共享整个账户的资产");
    println!("   4. 逐仓保证金只影响特定交易对");

    Ok(())
}
