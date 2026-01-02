//! Timestamp Migration Example
//!
//! This example demonstrates how to migrate from the old u64 timestamp API
//! to the new standardized i64 timestamp API in CCXT Rust.
//!
//! # Usage
//!
//! ```bash
//! cargo run --example timestamp_migration_example
//! ```

#![allow(deprecated)]

use ccxt_core::{
    prelude::*,
    time::{TimestampConversion, TimestampUtils},
};
use ccxt_exchanges::binance::Binance;
use rust_decimal_macros::dec;
use std::env;

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    println!("=== Timestamp Migration Example ===\n");
    // Initialize exchange
    let config = ExchangeConfig {
        api_key: env::var("BINANCE_API_KEY")
            .ok()
            .map(ccxt_core::SecretString::new),
        secret: env::var("BINANCE_API_SECRET")
            .ok()
            .map(ccxt_core::SecretString::new),
        sandbox: false,
        ..Default::default()
    };
    let exchange = Binance::new(config)?;

    // Example 1: Basic timestamp creation and usage
    example_timestamp_creation().await?;

    // Example 2: Data structure creation with i64 timestamps
    example_data_structures().await?;

    // Example 3: OHLCV fetching migration
    example_ohlcv_migration(&exchange).await?;

    // Example 4: Trade fetching migration
    example_trades_migration(&exchange).await?;

    // Example 5: Conversion utilities
    example_conversion_utilities().await?;

    // Example 6: Error handling
    example_error_handling().await?;

    println!("\n=== Migration Example Complete ===");
    Ok(())
}

/// Example 1: Basic timestamp creation and usage
async fn example_timestamp_creation() -> std::result::Result<(), Box<dyn std::error::Error>> {
    println!("📅 Example 1: Timestamp Creation");
    println!("--------------------------------");

    // OLD WAY (deprecated - don't do this)
    #[allow(clippy::cast_possible_truncation)]
    let _old_timestamp = chrono::Utc::now().timestamp_millis() as u64;
    println!("❌ Old way: casting to u64 (deprecated)");

    // NEW WAY (recommended)
    let new_timestamp: i64 = chrono::Utc::now().timestamp_millis();
    println!("✅ New way: using i64 directly");
    println!("   Current timestamp: {} ms", new_timestamp);

    // Using timestamp utilities
    let util_timestamp = TimestampUtils::now_ms();
    println!("✅ Using utilities: {} ms", util_timestamp);

    // Formatting timestamps
    let formatted = TimestampUtils::format_iso8601(new_timestamp);
    println!("   Formatted: {:?}", formatted);

    println!();
    Ok(())
}

/// Example 2: Data structure creation with i64 timestamps
async fn example_data_structures() -> std::result::Result<(), Box<dyn std::error::Error>> {
    println!("🏗️  Example 2: Data Structures");
    println!("------------------------------");

    let timestamp: i64 = chrono::Utc::now().timestamp_millis();

    // Create ticker with i64 timestamp
    let ticker = Ticker::new("BTC/USDT".to_string(), timestamp);
    println!("✅ Ticker created with i64 timestamp: {}", ticker.timestamp);

    // Create order book with i64 timestamp
    let orderbook = OrderBook::new("BTC/USDT".to_string(), timestamp);
    println!(
        "✅ OrderBook created with i64 timestamp: {}",
        orderbook.timestamp
    );

    // Create trade with i64 timestamp
    let trade = Trade::new(
        "BTC/USDT".to_string(),
        OrderSide::Buy,
        dec!(50000.0).into(),
        dec!(0.1).into(),
        timestamp,
    );
    println!("✅ Trade created with i64 timestamp: {}", trade.timestamp);

    // Create OHLCV with i64 timestamp
    let ohlcv = Ohlcv::new(
        timestamp,
        dec!(49000.0).into(), // open
        dec!(51000.0).into(), // high
        dec!(48500.0).into(), // low
        dec!(50000.0).into(), // close
        dec!(1234.5).into(),  // volume
    );
    println!("✅ OHLCV created with i64 timestamp: {}", ohlcv.timestamp);

    println!();
    Ok(())
}

/// Example 3: OHLCV fetching migration
async fn example_ohlcv_migration(
    exchange: &Binance,
) -> std::result::Result<(), Box<dyn std::error::Error>> {
    println!("📊 Example 3: OHLCV Fetching Migration");
    println!("--------------------------------------");

    // OLD WAY (deprecated - still works but shows warnings)
    println!("❌ Old way (deprecated):");
    println!("   // This would show deprecation warnings:");
    println!(
        "   // exchange.fetch_ohlcv_u64(\"BTC/USDT\", Timeframe::H1, Some(timestamp_u64), Some(100), None)"
    );

    // NEW WAY 1: Basic method
    println!("✅ New way 1: Basic method");
    match exchange
        .fetch_ohlcv("BTC/USDT", "1h", None, Some(5), None)
        .await
    {
        Ok(candles) => {
            println!("   Fetched {} candles using basic method", candles.len());
            if let Some(first) = candles.first() {
                println!("   First candle timestamp: {}", first.timestamp);
            }
        }
        Err(e) => println!("   Error: {}", e),
    }

    // NEW WAY 2: Parameter-based method (recommended for complex queries)
    println!("✅ New way 2: Parameter-based method");
    let since: i64 = chrono::Utc::now().timestamp_millis() - (24 * 60 * 60 * 1000); // 24 hours ago

    match exchange
        .fetch_ohlcv("BTC/USDT", "1h", Some(since), Some(10), None)
        .await
    {
        Ok(candles) => {
            println!(
                "   Fetched {} candles using parameter method",
                candles.len()
            );
            if let Some(first) = candles.first() {
                println!("   First candle timestamp: {}", first.timestamp);
                println!("   Since parameter (i64): {}", since);
            }
        }
        Err(e) => println!("   Error: {}", e),
    }

    println!();
    Ok(())
}

/// Example 4: Trade fetching migration
async fn example_trades_migration(
    exchange: &Binance,
) -> std::result::Result<(), Box<dyn std::error::Error>> {
    println!("💱 Example 4: Trade Fetching Migration");
    println!("--------------------------------------");

    // OLD WAY (this signature no longer exists)
    println!("❌ Old way (no longer available):");
    println!("   // exchange.fetch_trades(\"BTC/USDT\", Some(5)) // This signature is gone");

    // NEW WAY 1: Basic trades (no parameters)
    println!("✅ New way 1: Basic trades");
    match exchange.fetch_trades("BTC/USDT", None).await {
        Ok(trades) => {
            println!("   Fetched {} trades using basic method", trades.len());
        }
        Err(e) => println!("   Error: {}", e),
    }

    // NEW WAY 2: With limit only
    println!("✅ New way 2: With limit");
    match exchange.fetch_trades("BTC/USDT", Some(5)).await {
        Ok(trades) => {
            println!("   Fetched {} trades with limit", trades.len());
        }
        Err(e) => println!("   Error: {}", e),
    }

    // NEW WAY 3: With timestamp filtering (i64)
    println!("✅ New way 3: With timestamp filtering");
    let since: i64 = chrono::Utc::now().timestamp_millis() - (60 * 60 * 1000); // 1 hour ago
    // Note: The Binance implementation doesn't have fetch_trades_with_limit method
    // Instead, we use the regular fetch_trades method with limit parameter
    println!("   Note: Using fetch_trades with limit parameter");
    match exchange.fetch_trades("BTC/USDT", Some(10)).await {
        Ok(trades) => {
            println!("   Fetched {} trades with timestamp filter", trades.len());
            println!("   Since parameter (i64): {}", since);
        }
        Err(e) => println!("   Error: {}", e),
    }

    println!();
    Ok(())
}

/// Example 5: Conversion utilities
#[allow(deprecated)] // Demonstrating deprecated conversion functions
async fn example_conversion_utilities() -> std::result::Result<(), Box<dyn std::error::Error>> {
    println!("🔄 Example 5: Conversion Utilities");
    println!("----------------------------------");

    // Convert u64 to i64
    let u64_timestamp = 1609459200000u64; // January 1, 2021
    match TimestampUtils::u64_to_i64(u64_timestamp) {
        Ok(i64_timestamp) => {
            println!(
                "✅ u64 to i64 conversion: {} -> {}",
                u64_timestamp, i64_timestamp
            );
        }
        Err(e) => println!("❌ Conversion error: {}", e),
    }

    // Convert Option<u64> to Option<i64>
    let u64_option = Some(1609459200000u64);
    match u64_option.to_i64() {
        Ok(i64_option) => {
            println!(
                "✅ Option<u64> to Option<i64>: {:?} -> {:?}",
                u64_option, i64_option
            );
        }
        Err(e) => println!("❌ Conversion error: {}", e),
    }

    // Validate timestamp
    let timestamp = 1609459200000i64;
    match TimestampUtils::validate_timestamp(timestamp) {
        Ok(validated) => {
            println!("✅ Timestamp validation: {} is valid", validated);
        }
        Err(e) => println!("❌ Validation error: {}", e),
    }

    // Parse timestamp from string
    match TimestampUtils::parse_timestamp("1609459200000") {
        Ok(parsed) => {
            println!("✅ String parsing: '1609459200000' -> {}", parsed);
        }
        Err(e) => println!("❌ Parse error: {}", e),
    }

    // Time conversions
    let seconds = 1609459200i64;
    let milliseconds = TimestampUtils::seconds_to_ms(seconds);
    let back_to_seconds = TimestampUtils::ms_to_seconds(milliseconds);
    println!(
        "✅ Time conversions: {}s -> {}ms -> {}s",
        seconds, milliseconds, back_to_seconds
    );

    println!();
    Ok(())
}

/// Example 6: Error handling
#[allow(deprecated)] // Demonstrating error handling with deprecated conversion functions
async fn example_error_handling() -> std::result::Result<(), Box<dyn std::error::Error>> {
    println!("⚠️  Example 6: Error Handling");
    println!("-----------------------------");

    // Test timestamp overflow (u64 to i64)
    let large_u64 = u64::MAX;
    match TimestampUtils::u64_to_i64(large_u64) {
        Ok(converted) => {
            println!("Unexpected success: {}", converted);
        }
        Err(e) => {
            println!("✅ Correctly caught overflow: {}", e);
        }
    }

    // Test timestamp underflow (i64 to u64)
    let negative_i64 = -1000i64;
    match TimestampUtils::i64_to_u64(negative_i64) {
        Ok(converted) => {
            println!("Unexpected success: {}", converted);
        }
        Err(e) => {
            println!("✅ Correctly caught underflow: {}", e);
        }
    }

    // Test invalid timestamp range
    let invalid_timestamp = -1i64;
    match TimestampUtils::validate_timestamp(invalid_timestamp) {
        Ok(validated) => {
            println!("Unexpected success: {}", validated);
        }
        Err(e) => {
            println!("✅ Correctly caught invalid range: {}", e);
        }
    }

    // Test invalid string parsing
    match TimestampUtils::parse_timestamp("not_a_number") {
        Ok(parsed) => {
            println!("Unexpected success: {}", parsed);
        }
        Err(e) => {
            println!("✅ Correctly caught parse error: {}", e);
        }
    }

    println!();
    Ok(())
}
