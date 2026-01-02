//! Binance Order Management Example
//!
//! Demonstrates order management operations.
//!
//! Note: Some order methods (edit_order, fetch_canceled_and_closed_orders) are not yet
//! migrated to the new modular structure.

// Allow clippy warnings for example code - examples prioritize readability over strict linting
#![allow(clippy::disallowed_methods)]
#![allow(clippy::collapsible_if)]
#![allow(deprecated)]

use ccxt_core::types::{Amount, Price};
use ccxt_core::{ExchangeConfig, OrderSide, OrderType};
use ccxt_exchanges::binance::Binance;
use rust_decimal_macros::dec;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    let config = ExchangeConfig {
        api_key: std::env::var("BINANCE_API_KEY").ok(),
        secret: std::env::var("BINANCE_API_SECRET").ok(),
        ..Default::default()
    };

    let exchange = Binance::new(config)?;
    exchange.load_markets(false).await?;

    println!("=== Binance Order Management Example ===\n");

    // Example 1: Fetch open orders
    if exchange.base().config.api_key.is_some() {
        println!("--- Example 1: Fetch Open Orders ---");
        match exchange.fetch_open_orders(Some("BTC/USDT")).await {
            Ok(orders) => {
                println!("✓ Found {} open orders", orders.len());
                for order in orders.iter().take(5) {
                    println!("  Order ID: {}, Status: {:?}", order.id, order.status);
                }
            }
            Err(e) => println!("✗ Failed to fetch open orders: {}", e),
        }
        println!();

        // Example 2: Fetch closed orders
        println!("--- Example 2: Fetch Closed Orders ---");
        match exchange
            .fetch_closed_orders(Some("BTC/USDT"), None, Some(10))
            .await
        {
            Ok(orders) => {
                println!("✓ Found {} closed orders", orders.len());
                for order in orders.iter().take(5) {
                    println!("  Order ID: {}, Status: {:?}", order.id, order.status);
                }
            }
            Err(e) => println!("✗ Failed to fetch closed orders: {}", e),
        }
        println!();

        // Example 3: Create order (demonstration)
        println!("--- Example 3: Create Order (demonstration) ---");
        println!("Creating a limit buy order for BTC/USDT...");
        match exchange
            .create_order(
                "BTC/USDT",
                OrderType::Limit,
                OrderSide::Buy,
                Amount::new(dec!(0.001)),
                Some(Price::new(dec!(30000.0))), // Very low price, won't fill
                None,
            )
            .await
        {
            Ok(order) => {
                println!("✓ Order created successfully");
                println!("  Order ID: {}", order.id);
                println!("  Symbol: {}", order.symbol);
                println!("  Status: {:?}", order.status);
            }
            Err(e) => println!("✗ Failed to create order: {}", e),
        }
    } else {
        println!("⚠️  API credentials required for order management examples");
    }

    // Note about unmigrated methods
    println!("\nNote: Some order methods are not yet migrated to the new modular structure:");
    println!("  - edit_order");
    println!("  - edit_orders");
    println!("  - fetch_canceled_and_closed_orders");
    println!("      See rest_old.rs for the original implementations.");

    println!("\n=== Example Complete ===");
    Ok(())
}
