//! Binance Market Data API Example
//!
//! Demonstrates how to use Binance exchange market data APIs, including:
//! 1. `fetch_currencies` - Get all currency information
//! 2. `fetch_recent_trades` - Get recent public trades
//! 3. `fetch_my_recent_trades` - Get my trade history
//! 4. `fetch_agg_trades` - Get aggregated trades
//! 5. `fetch_historical_trades` - Get historical trades
//! 6. `fetch_24hr_stats` - Get 24-hour statistics
//! 7. `fetch_trading_limits` - Get trading limits
//! 8. `fetch_bids_asks` - Get best bid/ask prices
//! 9. `fetch_last_prices` - Get latest prices
//! 10. `fetch_mark_price` - Get futures mark prices
//! 11. `fetch_time` - Get server time
//!
//! # Usage
//!
// Allow clippy warnings for example code - examples prioritize readability over strict linting
#![allow(clippy::disallowed_methods)]

//! 1. Create a `.env` file and set API credentials (required for private APIs)
//! 2. Run: `cargo run --example binance_market_data_example`

use ccxt_core::ExchangeConfig;
use ccxt_exchanges::binance::Binance;
use std::collections::HashMap;
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();

    let api_key = env::var("BINANCE_API_KEY").unwrap_or_default();
    let api_secret = env::var("BINANCE_API_SECRET").unwrap_or_default();

    let has_credentials = !api_key.is_empty() && !api_secret.is_empty();

    let config = ExchangeConfig {
        api_key: Some(ccxt_core::SecretString::new(api_key)),
        secret: Some(ccxt_core::SecretString::new(api_secret)),
        ..Default::default()
    };
    let exchange = Binance::new(config)?;
    exchange.load_markets(false).await?;

    println!("=== Binance Market Data API Example ===\n");

    // ==================== Public API Examples ====================

    // 1. Fetch recent public trades
    println!("1. 【fetch_recent_trades】Get BTC/USDT recent public trades");
    match exchange.fetch_recent_trades("BTC/USDT", Some(5)).await {
        Ok(trades) => {
            println!("   ✓ Successfully fetched {} trades", trades.len());
            if let Some(trade) = trades.first() {
                println!(
                    "   Latest trade: price={}, amount={}, side={}",
                    trade.price, trade.amount, trade.side
                );
            }
        }
        Err(e) => println!("   ✗ Error: {}", e),
    }
    println!();

    // 2. Fetch aggregated trades
    println!("2. 【fetch_agg_trades】Get BTC/USDT aggregated trades");
    let mut params = HashMap::new();
    params.insert("limit".to_string(), "10".to_string());

    match exchange
        .fetch_agg_trades("BTC/USDT", None, None, Some(params))
        .await
    {
        Ok(agg_trades) => {
            println!(
                "   ✓ Successfully fetched {} aggregated trades",
                agg_trades.len()
            );
            if let Some(agg) = agg_trades.first() {
                println!("   First aggregation:");
                println!("     - ID: {}", agg.agg_id);
                println!("     - Price: {}", agg.price);
                println!("     - Quantity: {}", agg.quantity);
                println!("     - Trade count: {}", agg.trade_count());
                println!("     - Side: {}", agg.side());
            }
        }
        Err(e) => println!("   ✗ Error: {}", e),
    }
    println!();

    // 3. Fetch 24-hour statistics
    println!("3. 【fetch_24hr_stats】Get BTC/USDT 24-hour statistics");
    match exchange.fetch_24hr_stats(Some("BTC/USDT")).await {
        Ok(stats) => {
            println!(
                "   ✓ Successfully fetched statistics for {} symbols",
                stats.len()
            );
            if let Some(stat) = stats.first() {
                println!("   {} statistics:", stat.symbol);
                println!("     - Open price: {:?}", stat.open_price);
                println!("     - Last price: {:?}", stat.last_price);
                println!("     - High price: {:?}", stat.high_price);
                println!("     - Low price: {:?}", stat.low_price);
                println!("     - Volume: {:?}", stat.volume);
                if let Some(change) = stat.price_change_percent {
                    println!("     - Price change: {}%", change);
                }
            }
        }
        Err(e) => println!("   ✗ Error: {}", e),
    }
    println!();

    // 4. Fetch best bid/ask prices
    println!("4. 【fetch_bids_asks】Get BTC/USDT best bid/ask");
    match exchange.fetch_bids_asks(Some("BTC/USDT")).await {
        Ok(bid_asks) => {
            println!("   ✓ Successfully fetched bid/ask prices");
            if let Some(bid_ask) = bid_asks.first() {
                println!("   BTC/USDT:");
                println!("     - Bid price: {}", bid_ask.bid_price);
                println!("     - Ask price: {}", bid_ask.ask_price);
                println!("     - Bid quantity: {}", bid_ask.bid_quantity);
                println!("     - Ask quantity: {}", bid_ask.ask_quantity);
                println!("     - Spread: {}", bid_ask.spread());
                println!("     - Mid price: {}", bid_ask.mid_price());
            }
        }
        Err(e) => println!("   ✗ Error: {}", e),
    }
    println!();

    // 5. Fetch trading limits
    println!("5. 【fetch_trading_limits】Get BTC/USDT trading limits");
    match exchange.fetch_trading_limits("BTC/USDT").await {
        Ok(limits) => {
            println!("   ✓ Successfully fetched trading limits");
            if let Some(price_limits) = &limits.price {
                println!("   Price limits:");
                if let Some(min) = price_limits.min {
                    println!("     - Min price: {}", min);
                }
                if let Some(max) = price_limits.max {
                    println!("     - Max price: {}", max);
                }
            }
            if let Some(amount_limits) = &limits.amount {
                println!("   Amount limits:");
                if let Some(min) = amount_limits.min {
                    println!("     - Min amount: {}", min);
                }
                if let Some(max) = amount_limits.max {
                    println!("     - Max amount: {}", max);
                }
            }
            if let Some(cost_limits) = &limits.cost {
                println!("   Cost limits:");
                if let Some(min) = cost_limits.min {
                    println!("     - Min cost: {}", min);
                }
            }
        }
        Err(e) => println!("   ✗ Error: {}", e),
    }
    println!();

    // 6. Fetch historical trades (requires API key)
    if has_credentials {
        println!("6. 【fetch_historical_trades】Get BTC/USDT historical trades (requires API key)");
        match exchange
            .fetch_historical_trades("BTC/USDT", Some(5), None, None)
            .await
        {
            Ok(trades) => {
                println!(
                    "   ✓ Successfully fetched {} historical trades",
                    trades.len()
                );
                if let Some(trade) = trades.first() {
                    println!(
                        "   Earliest trade: ID={}, price={}, amount={}",
                        trade.id.clone().unwrap_or_default(),
                        trade.price,
                        trade.amount
                    );
                }
            }
            Err(e) => println!("   ✗ Error: {}", e),
        }
        println!();
    } else {
        println!("6. 【fetch_historical_trades】Skipped (requires API key)\n");
    }

    // ==================== Private API Examples ====================

    if has_credentials {
        println!("=== Private API Examples (requires API credentials) ===\n");

        // 7. Fetch all currency information
        println!("7. 【fetch_currencies】Get all currency information");
        match exchange.fetch_currencies().await {
            Ok(currencies) => {
                println!("   ✓ Successfully fetched {} currencies", currencies.len());

                for currency in currencies.iter().take(3) {
                    println!("\n   Currency: {}", currency.id);
                    println!("     - Name: {:?}", currency.name);
                    println!("     - Deposit enabled: {}", currency.deposit);
                    println!("     - Withdraw enabled: {}", currency.withdraw);

                    if !currency.networks.is_empty() {
                        println!("     - Supported networks ({}):", currency.networks.len());
                        for (network_id, network) in currency.networks.iter().take(2) {
                            println!(
                                "       * {}: deposit={}, withdraw={}",
                                network_id, network.deposit, network.withdraw
                            );
                            if let Some(fee) = network.fee {
                                println!("         Withdraw fee: {}", fee);
                            }
                        }
                    }
                }

                println!(
                    "\n   (Showing only first 3 currencies, total: {})",
                    currencies.len()
                );
            }
            Err(e) => println!("   ✗ Error: {}", e),
        }
        println!();

        // 8. Fetch my recent trades
        println!("8. 【fetch_my_recent_trades】Get my recent trade history");
        match exchange
            .fetch_my_recent_trades("BTC/USDT", None, Some(5), None)
            .await
        {
            Ok(trades) => {
                if trades.is_empty() {
                    println!("   ℹ No recent trades found");
                } else {
                    println!("   ✓ Found {} recent trades:", trades.len());
                    for (i, trade) in trades.iter().take(3).enumerate() {
                        println!(
                            "   {}. {:?} {} @ {} ({})",
                            i + 1,
                            trade.side,
                            trade.amount,
                            trade.price,
                            trade.datetime.as_deref().unwrap_or("N/A")
                        );
                    }
                }
            }
            Err(e) => println!("   ✗ Error: {}", e),
        }
        println!();
    } else {
        println!("=== Private API Examples Skipped (requires API_KEY and API_SECRET) ===\n");
    }

    // ==================== Advanced Usage Examples ====================

    println!("=== Advanced Usage Examples ===\n");

    // 9. Batch fetch 24-hour statistics for multiple symbols
    println!("9. 【Batch Query】Get 24-hour statistics for multiple symbols");
    match exchange.fetch_24hr_stats(None).await {
        Ok(all_stats) => {
            println!(
                "   ✓ Successfully fetched statistics for {} symbols",
                all_stats.len()
            );

            let mut sorted_stats = all_stats.clone();
            sorted_stats.sort_by(|a, b| {
                let a_change = a.price_change_percent.unwrap_or_default();
                let b_change = b.price_change_percent.unwrap_or_default();
                b_change.partial_cmp(&a_change).unwrap()
            });

            println!("   Top 5 gainers:");
            for (i, stat) in sorted_stats.iter().take(5).enumerate() {
                if let Some(change) = stat.price_change_percent {
                    println!(
                        "     {}. {}: +{:.2}% (volume: {:?})",
                        i + 1,
                        stat.symbol,
                        change,
                        stat.volume
                    );
                }
            }
        }
        Err(e) => println!("   ✗ Error: {}", e),
    }
    println!();

    // 10. Query aggregated trades with time range using i64 timestamps
    println!("10. 【Time Range Query】Query aggregated trades for specific time period");
    let now: i64 = chrono::Utc::now().timestamp_millis();
    let one_hour_ago: i64 = now - 3600 * 1000;

    let mut time_params = HashMap::new();
    time_params.insert("startTime".to_string(), one_hour_ago.to_string());
    time_params.insert("endTime".to_string(), now.to_string());
    time_params.insert("limit".to_string(), "100".to_string());

    match exchange
        .fetch_agg_trades("ETH/USDT", None, None, Some(time_params))
        .await
    {
        Ok(agg_trades) => {
            println!(
                "   ✓ Fetched {} aggregated trades from past 1 hour",
                agg_trades.len()
            );
            if !agg_trades.is_empty() {
                let total_volume: rust_decimal::Decimal =
                    agg_trades.iter().map(|t| t.quantity).sum();
                println!("   Total volume: {}", total_volume);
            }
        }
        Err(e) => println!("   ✗ Error: {}", e),
    }
    println!();

    // 11. Query best bid/ask prices for multiple symbols
    println!("11. 【Query】Fetch all bid/ask prices");
    match exchange.fetch_bids_asks(None).await {
        Ok(bid_asks) => {
            println!("   ✓ Fetched bid/ask for {} symbols", bid_asks.len());
            // Show first 3 symbols
            for bid_ask in bid_asks.iter().take(3) {
                println!(
                    "   {}: bid={}, ask={}",
                    bid_ask.symbol, bid_ask.bid_price, bid_ask.ask_price
                );
            }
        }
        Err(e) => println!("   ✗ Error: {}", e),
    }
    println!();

    // ==================== Error Handling Examples ====================

    println!("=== Error Handling Examples ===\n");

    // 12. Invalid trading pair
    println!("12. 【Error Handling】Query invalid trading pair");
    match exchange.fetch_recent_trades("INVALID/PAIR", Some(5)).await {
        Ok(trades) => {
            println!("   Unexpected success: {} trades", trades.len());
        }
        Err(e) => {
            println!("   ✓ Expected error correctly caught: {}", e);
        }
    }
    println!();

    // 13. Query with excessive limit
    println!("13. 【Error Handling】Query with excessive limit parameter");
    match exchange.fetch_recent_trades("BTC/USDT", Some(10000)).await {
        Ok(trades) => {
            println!("   Success (may be limited): {} trades", trades.len());
        }
        Err(e) => {
            println!("   Error: {}", e);
        }
    }
    println!();

    println!("=== Example Complete ===");

    Ok(())
}
