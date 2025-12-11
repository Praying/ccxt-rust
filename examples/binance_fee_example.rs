//! Binance fee management example.
//!
//! Demonstrates how to use the Binance exchange fee management APIs including:
//! - Trading fees (maker/taker rates)
//! - Funding rates for perpetual futures
//! - Funding rate history
//! - Leverage tier information
//!
//! # Environment Variables
//!
//! - `BINANCE_API_KEY`: API key for authenticated endpoints (optional)
//! - `BINANCE_SECRET`: API secret for authenticated endpoints (optional)
//!
//! # Examples
//!
//! ```bash
//! # Run without authentication (public endpoints only)
//! cargo run --example binance_fee_example
//!
//! # Run with authentication (all endpoints)
//! BINANCE_API_KEY=your_key BINANCE_SECRET=your_secret cargo run --example binance_fee_example
//! ```

use ccxt_core::ExchangeConfig;
use ccxt_exchanges::prelude::Binance;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let config = ExchangeConfig {
        api_key: std::env::var("BINANCE_API_KEY").ok(),
        secret: std::env::var("BINANCE_SECRET").ok(),
        sandbox: false,
        ..Default::default()
    };
    let binance = Binance::new(config)?;

    println!("=== Binance Fee Management Examples ===\n");

    // Example 1: Fetch trading fee for a single symbol (requires API key)
    if binance.base().config.api_key.is_some() {
        println!("1. Fetching BTC/USDT trading fee...");
        match binance.fetch_trading_fee("BTC/USDT", None).await {
            Ok(fee) => {
                println!("   Symbol: {}", fee.symbol);
                println!("   Maker: {}", fee.maker);
                println!("   Taker: {}", fee.taker);
                println!("   ✅ Success\n");
            }
            Err(e) => println!("   ❌ Error: {}\n", e),
        }

        println!("2. Fetching trading fees for all symbols...");
        match binance.fetch_trading_fees(None, None).await {
            Ok(fees) => {
                println!("   Retrieved {} trading pairs", fees.len());
                for fee in fees.iter().take(3) {
                    println!(
                        "   - {}: Maker={}, Taker={}",
                        fee.0, fee.1.maker, fee.1.taker
                    );
                }
                println!("   ✅ Success\n");
            }
            Err(e) => println!("   ❌ Error: {}\n", e),
        }
    } else {
        println!("⚠️  No API credentials set, skipping authenticated endpoints");
        println!("   Set environment variables: BINANCE_API_KEY and BINANCE_SECRET\n");
    }

    // Example 3: Fetch current funding rate (public API, futures market)
    println!("3. Fetching BTC/USDT:USDT perpetual contract funding rate...");
    match binance.fetch_funding_rate("BTC/USDT:USDT", None).await {
        Ok(rate) => {
            println!("   Symbol: {}", rate.symbol);
            if let Some(fr) = rate.funding_rate {
                println!("   Funding Rate: {}", fr);
            }
            if let Some(mp) = rate.mark_price {
                println!("   Mark Price: {}", mp);
            }
            println!("   ✅ Success\n");
        }
        Err(e) => println!("   ❌ Error: {}\n", e),
    }

    // Example 4: Fetch funding rate history
    println!("4. Fetching BTC/USDT:USDT perpetual contract funding rate history...");
    match binance
        .fetch_funding_rate_history("BTC/USDT:USDT", None, None, None)
        .await
    {
        Ok(history) => {
            println!("   Retrieved {} historical records", history.len());
            for record in history.iter().take(3) {
                println!("   - {}: Rate={:?}", record.symbol, record.funding_rate);
            }
            println!("   ✅ Success\n");
        }
        Err(e) => println!("   ❌ Error: {}\n", e),
    }

    // Example 5: Fetch funding rates for multiple symbols
    println!("5. Fetching funding rates for multiple perpetual contracts...");
    let symbols = vec!["BTC/USDT:USDT".to_string(), "ETH/USDT:USDT".to_string()];
    match binance.fetch_funding_rates(Some(symbols), None).await {
        Ok(rates) => {
            println!("   Retrieved {} contracts", rates.len());
            for (symbol, rate) in rates.iter() {
                println!("   - {}: Rate={:?}", symbol, rate.funding_rate);
            }
            println!("   ✅ Success\n");
        }
        Err(e) => println!("   ❌ Error: {}\n", e),
    }

    // Example 6: Fetch leverage tier information (requires API key)
    if binance.base().config.api_key.is_some() {
        println!("6. Fetching BTC/USDT:USDT perpetual contract leverage tiers...");
        let symbols = vec!["BTC/USDT:USDT".to_string()];
        match binance.fetch_leverage_tiers(Some(symbols), None).await {
            Ok(tiers_map) => {
                for (symbol, tiers) in tiers_map.iter() {
                    println!("   Symbol: {}", symbol);
                    println!("   Tier count: {}", tiers.len());
                    for tier in tiers.iter().take(3) {
                        println!(
                            "     - Tier {}: {}x leverage, Maintenance margin rate: {}",
                            tier.tier, tier.max_leverage, tier.maintenance_margin_rate
                        );
                    }
                }
                println!("   ✅ Success\n");
            }
            Err(e) => println!("   ❌ Error: {}\n", e),
        }
    }

    println!("=== Examples Complete ===");
    Ok(())
}
