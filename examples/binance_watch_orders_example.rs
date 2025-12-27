//! Binance WebSocket Order Updates Example
//!
//! Demonstrates how to use `watch_orders` to monitor real-time order updates.

use ccxt_core::ExchangeConfig;
use ccxt_core::types::default_type::DefaultType;
use ccxt_exchanges::binance::{Binance, BinanceOptions};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let api_key =
        std::env::var("BINANCE_API_KEY").expect("Set BINANCE_API_KEY environment variable");
    let api_secret =
        std::env::var("BINANCE_API_SECRET").expect("Set BINANCE_API_SECRET environment variable");

    let config = ExchangeConfig {
        api_key: Some(api_key),
        secret: Some(api_secret),
        ..Default::default()
    };

    let mut binance = Binance::new(config)?;

    let options = BinanceOptions {
        default_type: DefaultType::Spot,
        ..Default::default()
    };
    binance.set_options(options);

    let exchange = Arc::new(binance);

    println!("Starting to watch order updates...\n");

    println!("=== Example 1: Watch All Orders ===");
    match exchange.clone().watch_orders(None, None, None, None).await {
        Ok(orders) => {
            println!("Received {} order updates:", orders.len());
            for order in orders.iter().take(5) {
                println!("  Order ID: {}", order.id);
                println!("  Symbol: {}", order.symbol);
                println!("  Side: {}", order.side);
                println!("  Type: {}", order.order_type);
                println!("  Status: {}", order.status);
                println!("  Amount: {}", order.amount);
                if let Some(price) = order.price {
                    println!("  Price: {}", price);
                }
                if let Some(filled) = order.filled {
                    println!("  Filled: {}", filled);
                }
                println!("  ---");
            }
        }
        Err(e) => eprintln!("Failed to watch orders: {}", e),
    }

    println!("\n=== Example 2: Watch BTC/USDT Orders ===");
    match exchange
        .clone()
        .watch_orders(Some("BTC/USDT"), None, None, None)
        .await
    {
        Ok(orders) => {
            println!("Received {} BTC/USDT order updates:", orders.len());
            for order in &orders {
                println!(
                    "  Order ID: {} | Status: {} | Amount: {} | Filled: {}",
                    order.id,
                    order.status,
                    order.amount,
                    order.filled.unwrap_or_default()
                );
            }
        }
        Err(e) => eprintln!("Failed to watch BTC/USDT orders: {}", e),
    }

    println!("\n=== Example 3: Watch Recent 5 Orders ===");
    match exchange
        .clone()
        .watch_orders(None, None, Some(5), None)
        .await
    {
        Ok(orders) => {
            println!("Received recent {} orders:", orders.len());
            for (i, order) in orders.iter().enumerate() {
                println!(
                    "  {}. {} {} {} @ {:?}",
                    i + 1,
                    order.symbol,
                    order.side,
                    order.amount,
                    order.price
                );
            }
        }
        Err(e) => eprintln!("Failed to watch recent orders: {}", e),
    }

    println!("\n=== Example 4: Watch Orders Since Specified Time ===");
    let since: i64 = chrono::Utc::now().timestamp_millis() - 3600000;
    match exchange
        .clone()
        .watch_orders(None, Some(since), None, None)
        .await
    {
        Ok(orders) => {
            println!("Received {} orders in the last hour:", orders.len());
            for order in &orders {
                if let Some(ts) = order.timestamp {
                    let dt = chrono::DateTime::from_timestamp_millis(ts)
                        .map(|d| d.to_rfc3339())
                        .unwrap_or_default();
                    println!(
                        "  {} | {} | {} | {}",
                        dt, order.symbol, order.side, order.status
                    );
                }
            }
        }
        Err(e) => eprintln!("Failed to watch orders since specified time: {}", e),
    }

    println!("\nOrder watching examples completed successfully!");

    Ok(())
}
