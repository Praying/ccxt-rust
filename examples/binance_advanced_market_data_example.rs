//! Binance Advanced Market Data Example
//!
//! Demonstrates OHLCV data operations and other advanced market data methods.
//! Updated to use i64 timestamps and correct API methods.
//!
//! # Usage
//!
//! ```bash
//! cargo run --example binance_advanced_market_data_example
//! ```

// Allow clippy warnings for example code - examples prioritize readability over strict linting
#![allow(clippy::disallowed_methods)]
#![allow(clippy::collapsible_if)]
#![allow(clippy::useless_format)]
#![allow(clippy::useless_vec)]

use ccxt_core::ExchangeConfig;
use ccxt_exchanges::binance::Binance;
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

    // Fetch server time
    example_fetch_time(&client).await;

    example_fetch_ohlcv_basic(&client).await;
    example_fetch_ohlcv_timeframes(&client).await;
    example_fetch_ohlcv_historical(&client).await;
    example_analyze_ohlcv(&client).await;

    // Fetch last prices and mark prices
    example_fetch_last_prices(&client).await;
    example_fetch_mark_prices(&client).await;

    example_error_handling(&client).await;

    println!("\n=== All examples completed ===");
}

/// Example: Fetch server time
async fn example_fetch_time(client: &Binance) {
    println!("\n⏰ Fetch Server Time");
    match client.fetch_time().await {
        Ok(server_time) => {
            println!("   Server time: {} ms", server_time.server_time);
            println!("   Datetime: {}", server_time.datetime);
        }
        Err(e) => println!("   Error: {}", e),
    }
}

/// Example: Fetch last prices
async fn example_fetch_last_prices(client: &Binance) {
    println!("\n💰 Fetch Last Prices");
    client.load_markets(false).await.ok();

    // Single symbol
    match client.fetch_last_prices(Some("BTC/USDT")).await {
        Ok(prices) => {
            if let Some(price) = prices.first() {
                println!("   BTC/USDT last price: {}", price.price);
            }
        }
        Err(e) => println!("   Error: {}", e),
    }

    // All symbols (limited output)
    match client.fetch_last_prices(None).await {
        Ok(prices) => {
            println!("   Total symbols: {}", prices.len());
        }
        Err(e) => println!("   Error: {}", e),
    }
}

/// Example: Fetch mark prices (futures)
async fn example_fetch_mark_prices(client: &Binance) {
    println!("\n📈 Fetch Mark Prices (Futures)");
    client.load_markets(false).await.ok();

    match client.fetch_mark_price(None).await {
        Ok(mark_prices) => {
            println!("   Total futures symbols: {}", mark_prices.len());
            // Show first 3
            for mp in mark_prices.iter().take(3) {
                println!(
                    "   {}: mark={}, funding_rate={:?}",
                    mp.symbol, mp.mark_price, mp.last_funding_rate
                );
            }
        }
        Err(e) => println!("   Error: {}", e),
    }
}

/// Example 1: Fetch basic OHLCV data
async fn example_fetch_ohlcv_basic(client: &Binance) {
    println!("\n📊 Example 1: Fetch OHLCV Data");

    // Using the correct fetch_ohlcv method with i64 timestamps
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
                    if k.close > k.open { "Bull" } else { "Bear" }
                );
            }
        }
        Err(e) => println!("❌ Failed: {}", e),
    }
}

/// Example 2: Fetch different timeframes
async fn example_fetch_ohlcv_timeframes(client: &Binance) {
    println!("\n📊 Example 2: Different Timeframes");
    let timeframes = ["1m", "5m", "1h", "1d"];

    for tf in &timeframes {
        if let Ok(data) = client
            .fetch_ohlcv("ETH/USDT", tf, None, Some(1), None)
            .await
        {
            if let Some(k) = data.first() {
                let change_percent = if k.open > 0.0 {
                    (k.close - k.open) / k.open * 100.0
                } else {
                    0.0
                };
                println!(
                    "   {} {}: ${} -> ${} ({:+.2}%)",
                    tf, k.timestamp, k.open, k.close, change_percent
                );
            }
        }
    }
}

/// Example 3: Fetch historical data with since parameter
async fn example_fetch_ohlcv_historical(client: &Binance) {
    println!("\n📊 Example 3: Historical Data");

    // Using i64 timestamp (milliseconds since Unix epoch)
    let since: i64 = chrono::Utc::now().timestamp_millis() - (7 * 24 * 60 * 60 * 1000);

    match client
        .fetch_ohlcv("BNB/USDT", "1d", Some(since), Some(7), None)
        .await
    {
        Ok(data) => {
            if let (Some(first), Some(last)) = (data.first(), data.last()) {
                let change = if first.close > 0.0 {
                    (last.close - first.close) / first.close * 100.0
                } else {
                    0.0
                };
                println!(
                    "✅ BNB/USDT 7 days: ${} -> ${} ({:+.2}%)",
                    first.close, last.close, change
                );
            }
        }
        Err(e) => println!("❌ Failed: {}", e),
    }
}

/// Example 4: Analyze OHLCV data for market trend
async fn example_analyze_ohlcv(client: &Binance) {
    println!("\n📈 Example 4: OHLCV Analysis");

    match client
        .fetch_ohlcv("BTC/USDT", "1h", None, Some(20), None)
        .await
    {
        Ok(data) => {
            let bullish = data.iter().filter(|k| k.close > k.open).count();
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

/// Example 5: Error handling for invalid requests
async fn example_error_handling(client: &Binance) {
    println!("\n⚠️  Example 5: Error Handling");

    // Test invalid symbol
    match client
        .fetch_ohlcv("INVALID/PAIR", "1h", None, Some(1), None)
        .await
    {
        Ok(_) => println!("   Unexpected success"),
        Err(e) => println!("   ✓ Correctly caught error: {}", e),
    }

    // Test with basic fetch_ohlcv method (should work)
    match client
        .fetch_ohlcv("BTC/USDT", "1h", None, Some(1), None)
        .await
    {
        Ok(data) => println!("   ✓ Basic fetch_ohlcv works: {} candles", data.len()),
        Err(e) => println!("   ✗ Unexpected error: {}", e),
    }
}
