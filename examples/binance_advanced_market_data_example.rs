//! Binance Advanced Market Data Example
//!
//! Demonstrates OHLCV data, trading fees, and server time operations.
//!
//! # Usage
//!
//! ```bash
//! # Optional: Set API credentials for fee queries
//! export BINANCE_API_KEY="your_api_key"
//! export BINANCE_API_SECRET="your_api_secret"
//!
//! cargo run --example binance_advanced_market_data_example
//! ```

// Allow clippy warnings for example code - examples prioritize readability over strict linting
#![allow(clippy::disallowed_methods)]
#![allow(clippy::collapsible_if)]
#![allow(clippy::useless_format)]
#![allow(clippy::useless_vec)]

use ccxt_core::ExchangeConfig;
use ccxt_exchanges::binance::Binance;
use rust_decimal::Decimal;
use std::env;

#[tokio::main]
async fn main() {
    println!("=== Binance Advanced Market Data Example ===\n");

    let config = ExchangeConfig {
        api_key: env::var("BINANCE_API_KEY").ok(),
        secret: env::var("BINANCE_API_SECRET").ok(),
        sandbox: false,
        ..Default::default()
    };
    let client = Binance::new(config).unwrap();

    example_fetch_time(&client).await;
    example_fetch_ohlcv_basic(&client).await;
    example_fetch_ohlcv_timeframes(&client).await;
    example_fetch_ohlcv_historical(&client).await;
    example_analyze_ohlcv(&client).await;

    example_fetch_trading_fee(&client).await;
    example_fetch_trading_fees(&client).await;
    example_fetch_all_trading_fees(&client).await;
    example_compare_trading_fees(&client).await;

    example_time_sync_integration(&client).await;
    example_complete_market_analysis(&client).await;
    example_error_handling(&client).await;

    println!("\n=== All examples completed ===");
}

/// Example 1: Fetch server time
async fn example_fetch_time(client: &Binance) {
    println!("\n📍 Example 1: Fetch Server Time");
    match client.fetch_time().await {
        Ok(server_time) => {
            println!(
                "✅ Server time: {} ({}ms)",
                server_time.datetime, server_time.server_time
            );
            let offset = server_time.offset_from_local();
            println!(
                "   Local offset: {}ms {}",
                offset.abs(),
                if offset.abs() > 1000 { "⚠️" } else { "✓" }
            );
        }
        Err(e) => println!("❌ Failed: {}", e),
    }
}

/// Example 2: Fetch basic OHLCV data
async fn example_fetch_ohlcv_basic(client: &Binance) {
    println!("\n📊 Example 2: Fetch OHLCV Data");
    match client
        .fetch_ohlcv("BTC/USDT", "1h", None, Some(3), None)
        .await
    {
        Ok(ohlcvs) => {
            println!("✅ Fetched BTC/USDT 1h candles ({} bars)", ohlcvs.len());
            for (i, k) in ohlcvs.iter().enumerate() {
                println!(
                    "   #{} {}: O${} H${} L${} C${} V{} [{}]",
                    i + 1,
                    k.timestamp,
                    k.open,
                    k.high,
                    k.low,
                    k.close,
                    k.volume,
                    if k.is_bullish() { "Bull" } else { "Bear" }
                );
            }
        }
        Err(e) => println!("❌ Failed: {}", e),
    }
}

/// Example 3: Fetch different timeframes
async fn example_fetch_ohlcv_timeframes(client: &Binance) {
    println!("\n📊 Example 3: Different Timeframes");
    for tf in &["1m", "5m", "1h", "1d"] {
        if let Ok(data) = client
            .fetch_ohlcv("ETH/USDT", tf, None, Some(1), None)
            .await
        {
            if let Some(k) = data.first() {
                let change_percent = Decimal::from_f64_retain((k.close - k.open) / k.open)
                    .unwrap_or(Decimal::ZERO)
                    * Decimal::from(100);
                println!(
                    "   {} {}: ${} -> ${} ({:+.2}%)",
                    tf, k.timestamp, k.open, k.close, change_percent
                );
            }
        }
    }
}

/// Example 4: Fetch historical data with since parameter
async fn example_fetch_ohlcv_historical(client: &Binance) {
    println!("\n📊 Example 4: Historical Data");
    let since = chrono::Utc::now().timestamp_millis() - (7 * 24 * 60 * 60 * 1000);
    match client
        .fetch_ohlcv("BNB/USDT", "1d", Some(since), Some(7), None)
        .await
    {
        Ok(data) => {
            if let (Some(first), Some(last)) = (data.first(), data.last()) {
                let change = Decimal::from_f64_retain((last.close - first.close) / first.close)
                    .unwrap_or(Decimal::ZERO)
                    * Decimal::from(100);
                println!(
                    "✅ BNB/USDT 7 days: ${} -> ${} ({:+.2}%)",
                    first.close, last.close, change
                );
            }
        }
        Err(e) => println!("❌ Failed: {}", e),
    }
}

/// Example 5: Analyze OHLCV data for market trend
async fn example_analyze_ohlcv(client: &Binance) {
    println!("\n📈 Example 5: OHLCV Analysis");
    match client
        .fetch_ohlcv("BTC/USDT", "1h", None, Some(20), None)
        .await
    {
        Ok(data) => {
            let bullish = data.iter().filter(|k| k.is_bullish()).count();
            let bearish = data.len() - bullish;
            let trend = match (bullish, bearish) {
                (b, _) if b > bearish * 2 => "Strong uptrend 📈",
                (_, b) if b > bullish * 2 => "Strong downtrend 📉",
                (b, _) if b > bearish => "Mild uptrend ↗️",
                _ => "Mild downtrend ↘️",
            };
            println!(
                "✅ 20 candles: {} bullish, {} bearish => {}",
                bullish, bearish, trend
            );
        }
        Err(e) => println!("❌ Failed: {}", e),
    }
}

/// Example 6: Fetch single trading pair fee
async fn example_fetch_trading_fee(client: &Binance) {
    println!("\n💰 Example 6: Single Trading Pair Fee");
    if client.base().config.api_key.is_none() {
        println!("⚠️  API key required");
        return;
    }
    match client.fetch_trading_fee("BTC/USDT", None).await {
        Ok(fee) => {
            println!(
                "✅ {}: Maker {:.4}% Taker {:.4}%",
                fee.symbol,
                fee.maker * Decimal::from(100),
                fee.taker * Decimal::from(100)
            );
            let trade = Decimal::from(10000);
            let diff = (fee.taker - fee.maker) * trade;
            println!(
                "   $10k trade: Maker ${:.2} Taker ${:.2} (Save ${:.2})",
                trade * fee.maker,
                trade * fee.taker,
                diff
            );
        }
        Err(e) => println!("❌ Failed: {}", e),
    }
}

/// Example 7: Fetch multiple trading pair fees
async fn example_fetch_trading_fees(client: &Binance) {
    println!("\n💰 Example 7: Multiple Trading Pair Fees");
    if client.base().config.api_key.is_none() {
        println!("⚠️  API key required");
        return;
    }
    let symbols: Vec<String> = vec!["BTC/USDT", "ETH/USDT", "BNB/USDT"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    match client.fetch_trading_fees(Some(symbols), None).await {
        Ok(fees) => {
            println!("✅ Fetched {} trading pair fees", fees.len());
            for (symbol, fee) in fees {
                println!(
                    "   {}: M{:.4}% T{:.4}%",
                    symbol,
                    fee.maker * Decimal::from(100),
                    fee.taker * Decimal::from(100)
                );
            }
        }
        Err(e) => println!("❌ Failed: {}", e),
    }
}

/// Example 8: Fetch all trading pair fees
async fn example_fetch_all_trading_fees(client: &Binance) {
    println!("\n💰 Example 8: All Trading Pair Fees");
    if client.base().config.api_key.is_none() {
        println!("⚠️  API key required");
        return;
    }
    match client.fetch_trading_fees(None, None).await {
        Ok(fees) => {
            println!("✅ Total {} trading pairs", fees.len());
            println!(
                "   First 5: {}",
                fees.iter()
                    .take(5)
                    .map(|(symbol, _)| format!("{}", symbol))
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        }
        Err(e) => println!("❌ Failed: {}", e),
    }
}

/// Example 9: Compare trading fees across multiple pairs
async fn example_compare_trading_fees(client: &Binance) {
    println!("\n💰 Example 9: Compare Trading Fees");
    if client.base().config.api_key.is_none() {
        println!("⚠️  API key required");
        return;
    }
    let symbols: Vec<String> = vec!["BTC/USDT", "ETH/USDT", "BNB/USDT"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    match client.fetch_trading_fees(Some(symbols), None).await {
        Ok(fees) => {
            if let (Some(min), Some(max)) = (
                fees.iter().min_by_key(|(_, f)| f.maker),
                fees.iter().max_by_key(|(_, f)| f.maker),
            ) {
                println!(
                    "✅ Lowest maker fee: {} ({:.4}%)",
                    min.0,
                    min.1.maker * Decimal::from(100)
                );
                println!(
                    "   Highest maker fee: {} ({:.4}%)",
                    max.0,
                    max.1.maker * Decimal::from(100)
                );
            }
        }
        Err(e) => println!("❌ Failed: {}", e),
    }
}

/// Example 10: Integrate time sync with OHLCV fetching
async fn example_time_sync_integration(client: &Binance) {
    println!("\n🔄 Example 10: Time Sync + OHLCV");
    if let Ok(time) = client.fetch_time().await {
        let since = time.server_time - (2 * 60 * 60 * 1000);
        if let Ok(data) = client
            .fetch_ohlcv("BTC/USDT", "1h", Some(since), Some(2), None)
            .await
        {
            println!(
                "✅ Using server time {} fetched {} candles",
                time.datetime,
                data.len()
            );
        }
    }
}

/// Example 11: Complete market analysis workflow
async fn example_complete_market_analysis(client: &Binance) {
    println!("\n🎯 Example 11: Complete Market Analysis");
    let symbol = "BTC/USDT";

    let _ = client.fetch_time().await;

    if let Ok(data) = client.fetch_ohlcv(symbol, "1h", None, Some(24), None).await {
        println!(
            "✅ {} Step 1: Fetched 24h candles ({} bars)",
            symbol,
            data.len()
        );
        if let (Some(first), Some(last)) = (data.first(), data.last()) {
            let change = (last.close - first.close) / first.close * 100.0;
            println!("   24h change: {:+.2}%", change);
        }
    }

    if client.base().config.api_key.is_some() {
        if let Ok(fee) = client.fetch_trading_fee(symbol, None).await {
            println!(
                "   Step 2: Trading fees M{:.4}% T{:.4}%",
                fee.maker * Decimal::from(100),
                fee.taker * Decimal::from(100)
            );
        }
    }

    println!("   ✅ Market analysis completed");
}

/// Example 12: Error handling for invalid requests
async fn example_error_handling(client: &Binance) {
    println!("\n⚠️  Example 12: Error Handling");

    match client
        .fetch_ohlcv("INVALID/PAIR", "1h", None, Some(1), None)
        .await
    {
        Ok(_) => println!("   Unexpected success"),
        Err(e) => println!("   ✓ Correctly caught error: {}", e),
    }

    match client
        .fetch_ohlcv("BTC/USDT", "99h", None, Some(1), None)
        .await
    {
        Ok(_) => println!("   Unexpected success"),
        Err(e) => println!("   ✓ Correctly caught error: {}", e),
    }
}
