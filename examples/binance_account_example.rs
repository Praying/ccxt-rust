//! Binance Account Management Examples
//!
//! Demonstrates account management operations using ccxt-rust:
//! 1. Query balances for multiple account types (spot/margin/futures/funding)
//! 2. Internal account transfers
//! 3. Query transfer history
//! 4. Query maximum borrowable amount
//! 5. Query maximum transferable amount
//!
//! # Usage
//!
//! ```bash
//! # Set environment variables
//! export BINANCE_API_KEY="your_api_key"
//! export BINANCE_API_SECRET="your_api_secret"
//!
//! # Run the example
//! cargo run --example binance_account_example
//! ```

// Allow clippy warnings for example code - examples prioritize readability over strict linting
#![allow(clippy::disallowed_methods)]
#![allow(dead_code)]
use ccxt_exchanges::binance::Binance;
use ccxt_exchanges::prelude::ExchangeConfig;
use dotenvy::dotenv;
use std::collections::HashMap;
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();

    let api_key =
        env::var("BINANCE_API_KEY").expect("BINANCE_API_KEY environment variable not set");
    let secret =
        env::var("BINANCE_API_SECRET").expect("BINANCE_API_SECRET environment variable not set");

    let config = ExchangeConfig {
        api_key: Some(api_key),
        secret: Some(secret),
        ..Default::default()
    };
    let binance = Binance::new(config)?;

    println!("=== Binance Account Management Examples ===\n");

    example_fetch_spot_balance(&binance).await?;
    example_fetch_margin_balance(&binance).await?;
    example_fetch_isolated_margin_balance(&binance).await?;
    example_fetch_future_balance(&binance).await?;
    example_fetch_delivery_balance(&binance).await?;
    example_fetch_funding_balance(&binance).await?;

    // Commented out: requires real funds
    //example_internal_transfer(&binance).await?;
    //example_fetch_transfers(&binance).await?;

    example_fetch_cross_margin_max_borrowable(&binance).await?;
    example_fetch_isolated_margin_max_borrowable(&binance).await?;
    example_fetch_max_transferable(&binance).await?;
    example_complete_account_workflow(&binance).await?;

    Ok(())
}

/// Example 1: Fetch spot account balance
async fn example_fetch_spot_balance(binance: &Binance) -> Result<(), Box<dyn std::error::Error>> {
    println!("📊 Example 1: Fetch Spot Account Balance");
    println!("----------------------------------------");

    let mut params = HashMap::new();
    params.insert("type".to_string(), serde_json::json!("spot"));

    let balance = binance.fetch_balance(Some(params)).await?;

    println!("Spot Account Balance:");
    for (currency, entry) in balance.balances.iter().take(5) {
        if entry.total >= rust_decimal::Decimal::ZERO {
            println!(
                "  {} - Total: {}, Free: {}, Used: {}",
                currency, entry.total, entry.free, entry.used
            );
        }
    }
    println!("(Showing first 5 currencies with balance)\n");

    Ok(())
}

/// Example 2: Fetch cross margin account balance
async fn example_fetch_margin_balance(binance: &Binance) -> Result<(), Box<dyn std::error::Error>> {
    println!("📊 Example 2: Fetch Cross Margin Account Balance");
    println!("----------------------------------------");

    let mut params = HashMap::new();
    params.insert("type".to_string(), serde_json::json!("margin"));

    let balance = binance.fetch_balance(Some(params)).await?;

    println!("Cross Margin Account Balance:");
    for (currency, entry) in balance.balances.iter().take(5) {
        if entry.total >= rust_decimal::Decimal::ZERO {
            println!(
                "  {} - Total: {}, Free: {}, Used: {}",
                currency, entry.total, entry.free, entry.used
            );
        }
    }
    println!();

    Ok(())
}

/// Example 3: Fetch isolated margin account balance
async fn example_fetch_isolated_margin_balance(
    binance: &Binance,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("📊 Example 3: Fetch Isolated Margin Account Balance");
    println!("----------------------------------------");

    let mut params = HashMap::new();
    params.insert("type".to_string(), serde_json::json!("isolated"));
    params.insert("symbol".to_string(), serde_json::json!("BTC/USDT"));

    let balance = binance.fetch_balance(Some(params)).await?;

    println!("Isolated Margin Account Balance (BTC/USDT):");
    for (currency, entry) in balance.balances.iter() {
        println!(
            "  {} - Total: {}, Free: {}, Used: {}",
            currency, entry.total, entry.free, entry.used
        );
    }
    println!();

    Ok(())
}

/// Example 4: Fetch USDT-margined futures account balance
async fn example_fetch_future_balance(binance: &Binance) -> Result<(), Box<dyn std::error::Error>> {
    println!("📊 Example 4: Fetch USDT-Margined Futures Account Balance");
    println!("----------------------------------------");

    let mut params = HashMap::new();
    params.insert("type".to_string(), serde_json::json!("future"));

    let balance = binance.fetch_balance(Some(params)).await?;

    println!("USDT-Margined Futures Account Balance:");
    for (currency, entry) in balance.balances.iter() {
        if entry.total >= rust_decimal::Decimal::ZERO {
            println!(
                "  {} - Total: {}, Free: {}, Used: {}",
                currency, entry.total, entry.free, entry.used
            );
        }
    }
    println!();

    Ok(())
}

/// Example 5: Fetch coin-margined futures account balance
async fn example_fetch_delivery_balance(
    binance: &Binance,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("📊 Example 5: Fetch Coin-Margined Futures Account Balance");
    println!("----------------------------------------");

    let mut params = HashMap::new();
    params.insert("type".to_string(), serde_json::json!("delivery"));

    let balance = binance.fetch_balance(Some(params)).await?;

    println!("Coin-Margined Futures Account Balance:");
    for (currency, entry) in balance.balances.iter() {
        if entry.total >= rust_decimal::Decimal::ZERO {
            println!(
                "  {} - Total: {}, Free: {}, Used: {}",
                currency, entry.total, entry.free, entry.used
            );
        }
    }
    println!();

    Ok(())
}

/// Example 6: Fetch funding account balance
async fn example_fetch_funding_balance(
    binance: &Binance,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("📊 Example 6: Fetch Funding Account Balance");
    println!("----------------------------------------");

    let mut params = HashMap::new();
    params.insert("type".to_string(), serde_json::json!("funding"));

    let balance = binance.fetch_balance(Some(params)).await?;

    println!("Funding Account Balance (includes savings, mining pool, etc.):");
    for (currency, entry) in balance.balances.iter().take(5) {
        if entry.total >= rust_decimal::Decimal::ZERO {
            println!(
                "  {} - Total: {}, Free: {}, Used: {}",
                currency, entry.total, entry.free, entry.used
            );
        }
    }
    println!("(Showing first 5 currencies with balance)\n");

    Ok(())
}

/// Example 7: Internal account transfer
async fn example_internal_transfer(binance: &Binance) -> Result<(), Box<dyn std::error::Error>> {
    println!("💸 Example 7: Internal Account Transfer");
    println!("----------------------------------------");

    let params = HashMap::new();

    let transfer = binance
        .transfer("USDT", 10.0, "spot", "future", Some(params))
        .await?;

    println!("Transfer Successful!");
    println!("  Transaction ID: {:?}", transfer.id);
    println!("  Currency: {}", transfer.currency);
    println!("  Amount: {}", transfer.amount);
    println!("  From Account: {:?}", transfer.from_account);
    println!("  To Account: {:?}", transfer.to_account);
    println!("  Timestamp: {}", transfer.timestamp);
    println!();

    Ok(())
}

/// Example 8: Fetch transfer history
async fn example_fetch_transfers(binance: &Binance) -> Result<(), Box<dyn std::error::Error>> {
    println!("📜 Example 8: Fetch Transfer History");
    println!("----------------------------------------");

    let mut params = HashMap::new();
    params.insert("type".to_string(), "MAIN_UMFUTURE".to_string());

    let transfers = binance
        .fetch_transfers(None, None, Some(10), Some(params))
        .await?;

    println!("Found {} transfer records:", transfers.len());
    for (i, transfer) in transfers.iter().enumerate() {
        println!(
            "  {}. {} {} - {:?} -> {:?} (ID: {:?})",
            i + 1,
            transfer.amount,
            transfer.currency,
            transfer.from_account,
            transfer.to_account,
            transfer.id
        );
    }
    println!();

    Ok(())
}

/// Example 9: Fetch cross margin max borrowable amount
async fn example_fetch_cross_margin_max_borrowable(
    binance: &Binance,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 Example 9: Fetch Cross Margin Max Borrowable Amount");
    println!("----------------------------------------");

    let params = HashMap::new();

    let max_borrowable = binance
        .fetch_cross_margin_max_borrowable("BTC", Some(params))
        .await?;

    println!("Cross Margin Max Borrowable:");
    println!("  Currency: {}", max_borrowable.currency);
    println!("  Max Borrowable Amount: {}", max_borrowable.amount);
    if let Some(ref symbol) = max_borrowable.symbol {
        println!("  Symbol: {}", symbol);
    }
    println!();

    Ok(())
}

/// Example 10: Fetch isolated margin max borrowable amount
async fn example_fetch_isolated_margin_max_borrowable(
    binance: &Binance,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 Example 10: Fetch Isolated Margin Max Borrowable Amount");
    println!("----------------------------------------");

    let params = HashMap::new();

    let max_borrowable = binance
        .fetch_isolated_margin_max_borrowable("USDT", "BTC/USDT", Some(params))
        .await?;

    println!("Isolated Margin Max Borrowable:");
    if let Some(ref symbol) = max_borrowable.symbol {
        println!("  Symbol: {}", symbol);
    }
    println!("  Currency: {}", max_borrowable.currency);
    println!("  Max Borrowable Amount: {}", max_borrowable.amount);
    println!();

    Ok(())
}

/// 示例11：查询最大可转账金额
async fn example_fetch_max_transferable(
    binance: &Binance,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 示例11：查询最大可转账金额");
    println!("----------------------------------------");

    let mut params = HashMap::new();
    params.insert("fromAccount".to_string(), serde_json::json!("spot"));
    params.insert("toAccount".to_string(), serde_json::json!("future"));

    // fetch_max_transferable需要2个参数：currency和params
    let max_transferable = binance.fetch_max_transferable("USDT", Some(params)).await?;

    println!("最大可转账金额（现货->合约）：");
    println!("  币种: {}", max_transferable.currency);
    println!("  最大可转金额: {}", max_transferable.amount);
    if let Some(ref symbol) = max_transferable.symbol {
        println!("  交易对: {}", symbol);
    }
    println!();

    Ok(())
}

/// Example 12: Complete account management workflow
async fn example_complete_account_workflow(
    binance: &Binance,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("🔄 Example 12: Complete Account Management Workflow");
    println!("========================================");

    println!("\nStep 1: Query All Account Balances");
    println!("------------------------");

    let account_types = vec!["spot", "margin", "future", "funding"];
    for account_type in account_types {
        let mut params = HashMap::new();
        params.insert("type".to_string(), serde_json::json!(account_type));

        match binance.fetch_balance(Some(params)).await {
            Ok(balance) => {
                let non_zero = balance
                    .balances
                    .iter()
                    .filter(|(_, v)| v.total > rust_decimal::Decimal::ZERO)
                    .count();
                println!(
                    "  ✅ {} account: {} currencies with balance",
                    account_type, non_zero
                );
            }
            Err(e) => println!("  ❌ {} account query failed: {}", account_type, e),
        }
    }

    println!("\nStep 2: Query Max Transferable Amount");
    println!("------------------------");

    let mut params = HashMap::new();
    params.insert("asset".to_string(), serde_json::json!("USDT"));
    params.insert("fromAccount".to_string(), serde_json::json!("spot"));
    params.insert("toAccount".to_string(), serde_json::json!("future"));

    match binance.fetch_max_transferable("USDT", Some(params)).await {
        Ok(max) => {
            println!("  ✅ Spot->Futures max transferable: {} USDT", max.amount);

            if max.amount > 10.0 {
                println!("\nStep 3: Execute Transfer Test");
                println!("------------------------");

                let transfer_params = HashMap::new();

                match binance
                    .transfer("USDT", 10.0, "spot", "future", Some(transfer_params))
                    .await
                {
                    Ok(transfer) => {
                        println!("  ✅ Transfer successful!");
                        println!("     Transaction ID: {:?}", transfer.id);
                        println!("     Amount: {} USDT", transfer.amount);
                    }
                    Err(e) => println!("  ❌ Transfer failed: {}", e),
                }
            } else {
                println!("\nStep 3: Skip transfer test (insufficient balance, need 10 USDT)");
            }
        }
        Err(e) => println!("  ❌ Query failed: {}", e),
    }

    println!("\nStep 4: Query Transfer History");
    println!("------------------------");

    let mut params = HashMap::new();
    params.insert("type".to_string(), "MAIN_UMFUTURE".to_string());

    match binance
        .fetch_transfers(None, None, Some(5), Some(params))
        .await
    {
        Ok(transfers) => {
            println!("  ✅ Found {} recent transfer records", transfers.len());
            for (i, transfer) in transfers.iter().enumerate() {
                println!(
                    "     {}. {} {} (ID: {:?})",
                    i + 1,
                    transfer.amount,
                    transfer.currency,
                    transfer.id
                );
            }
        }
        Err(e) => println!("  ❌ Query failed: {}", e),
    }

    println!("\nStep 5: Query Margin Borrowing Capacity");
    println!("------------------------");

    let params = HashMap::new();

    match binance
        .fetch_cross_margin_max_borrowable("BTC", Some(params))
        .await
    {
        Ok(max) => println!("  ✅ Cross margin max borrowable BTC: {}", max.amount),
        Err(e) => println!("  ❌ Query failed: {}", e),
    }

    println!("\n========================================");
    println!("🎉 Account management workflow demonstration completed!\n");

    Ok(())
}
