//! Binance Margin Borrow Example
//!
//! Demonstrates Binance margin lending and borrowing features.
//!
//! # Prerequisites
//!
//! - Valid API key and secret
//! - Margin account enabled on Binance
//!
//! # Examples
//!
//! ```bash
//! cargo run --example binance_margin_borrow_example
//! ```

// Allow clippy warnings for example code - examples prioritize readability over strict linting
#![allow(clippy::disallowed_methods)]
#![allow(unused_mut)]

use ccxt_core::{ExchangeConfig, error::Result};
use ccxt_exchanges::binance::Binance;
use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== Binance Margin Borrow Features Example ===\n");

    let config = ExchangeConfig {
        api_key: Some("YOUR_API_KEY".to_string()),
        secret: Some("YOUR_API_SECRET".to_string()),
        ..Default::default()
    };
    let mut binance = Binance::new(config)?;

    // binance.set_sandbox_mode(true);

    println!("1. Query Cross Margin Borrow Rate");
    println!("   Function: Get cross margin borrow rate for specified currency");
    println!("   ⚠️  Note: fetch_cross_borrow_rate is deprecated, use fetch_borrow_rate_history");
    println!();

    println!("2. Query Single Isolated Margin Borrow Rate");
    println!("   Function: Get isolated margin borrow rate for specified symbol");
    println!(
        "   ⚠️  Note: fetch_isolated_borrow_rate is deprecated, use fetch_isolated_borrow_rates"
    );
    println!();

    println!("3. Query All Isolated Margin Borrow Rates");
    println!("   Function: Get isolated margin borrow rates for all symbols");
    match binance.fetch_isolated_borrow_rates(None).await {
        Ok(rates) => {
            println!("   ✅ Found borrow rates for {} symbols", rates.len());
            for (i, (symbol, rate)) in rates.iter().take(5).enumerate() {
                println!(
                    "      {}. {} - Base rate: {}, Quote rate: {}",
                    i + 1,
                    symbol,
                    rate.base_rate,
                    rate.quote_rate
                );
            }
            if rates.len() > 5 {
                println!("      ... {} more symbols", rates.len() - 5);
            }
        }
        Err(e) => println!("   ❌ Query failed: {}", e),
    }
    println!();

    println!("4. Query Borrow Interest History");
    println!("   Function: Get borrow interest history records");

    let mut params = HashMap::new();
    match binance
        .fetch_borrow_interest(Some("USDT"), None, None, Some(10), Some(params.clone()))
        .await
    {
        Ok(interests) => {
            println!("   ✅ USDT borrow interest history (last 10 records):");
            for (i, interest) in interests.iter().enumerate() {
                println!(
                    "      {}. {} - Interest: {} {}, Rate: {}",
                    i + 1,
                    interest.datetime,
                    interest.interest,
                    interest.currency,
                    interest.interest_rate
                );
            }
        }
        Err(e) => println!("   ❌ Query failed: {}", e),
    }
    println!();

    params.insert("isolatedSymbol".to_string(), "BTCUSDT".to_string());
    match binance
        .fetch_borrow_interest(Some("BTC"), Some("BTC/USDT"), None, Some(5), Some(params))
        .await
    {
        Ok(interests) => {
            println!("   ✅ BTC/USDT (isolated) BTC borrow interest history (last 5 records):");
            for (i, interest) in interests.iter().enumerate() {
                println!(
                    "      {}. {} - Interest: {} {}, Principal: {}",
                    i + 1,
                    interest.datetime,
                    interest.interest,
                    interest.currency,
                    interest.principal
                );
            }
        }
        Err(e) => println!("   ❌ Query failed: {}", e),
    }
    println!();

    println!("5. Query Borrow Rate History");
    println!("   Function: Get borrow rate history for a currency");
    match binance
        .fetch_borrow_rate_history("BTC", None, Some(30), None)
        .await
    {
        Ok(history_items) => {
            println!("   ✅ BTC borrow rate history (last 30 records):");
            for (i, item) in history_items.iter().enumerate() {
                println!(
                    "      {}. {} - Currency: {}, Daily rate: {}",
                    i + 1,
                    item.datetime,
                    item.currency,
                    item.rate
                );
            }
        }
        Err(e) => println!("   ❌ Query failed: {}", e),
    }
    println!();

    println!("=== Advanced Usage Examples ===\n");

    println!("6. Query Borrow Rate History with Time Range");
    println!("   Function: Query rate changes within specified time range");
    let now = chrono::Utc::now().timestamp_millis() as u64;
    let one_week_ago = now - 7 * 24 * 60 * 60 * 1000;

    match binance
        .fetch_borrow_rate_history("USDT", Some(one_week_ago), Some(50), None)
        .await
    {
        Ok(history_items) => {
            println!("   ✅ USDT borrow rate history for past 7 days:");
            println!("      Total {} records", history_items.len());
            if !history_items.is_empty() {
                let first = &history_items[0];
                let last = history_items.last().unwrap();
                println!("      Earliest: {} - Rate: {}", first.datetime, first.rate);
                println!("      Latest: {} - Rate: {}", last.datetime, last.rate);
            }
        }
        Err(e) => println!("   ❌ Query failed: {}", e),
    }
    println!();

    println!("7. Query Borrow Interest in Portfolio Margin Mode");
    println!("   Function: Query borrow interest under Portfolio Margin mode");
    let mut pm_params = HashMap::new();
    pm_params.insert("portfolioMargin".to_string(), "true".to_string());

    match binance
        .fetch_borrow_interest(Some("BTC"), None, None, Some(10), Some(pm_params))
        .await
    {
        Ok(interests) => {
            println!("   ✅ Portfolio Margin BTC borrow interest:");
            println!("      Total {} records", interests.len());
        }
        Err(e) => println!("   ❌ Query failed: {}", e),
    }
    println!();

    println!("=== Important Notes ===");
    println!("1. All queries require valid API key and secret");
    println!("2. fetch_borrow_rate_history() limit parameter cannot exceed 92");
    println!("3. Isolated margin requires creating an isolated margin account on Binance first");
    println!("4. Borrow rates change in real-time based on market conditions");
    println!("5. Portfolio Margin requires enabling this feature first");
    println!("6. Be aware of API rate limits when querying historical data");

    Ok(())
}
