//! Binance OCO Order Management Example
//!
//! Demonstrates how to use Binance OCO (One-Cancels-the-Other) order functionality:
//! 1. Create OCO orders
//! 2. Query single OCO order
//! 3. Query all OCO orders
//! 4. Cancel OCO orders
//! 5. Create test orders
//! 6. Edit existing orders
//!
//! # Usage
//!
//! ```bash
//! export BINANCE_API_KEY="your_api_key"
//! export BINANCE_SECRET="your_secret"
//! cargo run --example binance_oco_order_example
//! ```

// Allow clippy warnings for example code - examples prioritize readability over strict linting
#![allow(clippy::disallowed_methods)]
#![allow(clippy::field_reassign_with_default)]

use ccxt_core::{
    ExchangeConfig,
    logging::{LogConfig, init_logging},
    types::{OrderSide, OrderType},
};
use ccxt_exchanges::binance::Binance;
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_logging(LogConfig::development());

    println!("=================================================");
    println!("  Binance OCO Order Management Example");
    println!("=================================================\n");

    let api_key =
        env::var("BINANCE_API_KEY").expect("Please set BINANCE_API_KEY environment variable");
    let secret =
        env::var("BINANCE_SECRET").expect("Please set BINANCE_SECRET environment variable");

    let mut config = ExchangeConfig::default();
    config.api_key = Some(api_key);
    config.secret = Some(secret);
    config.sandbox = true;

    let binance = Binance::new(config)?;
    println!("✓ Connected to Binance testnet\n");

    // Example 1: Create test order
    println!("📝 Example 1: Create Test Order");
    println!("────────────────────────────────────────────────");
    match binance
        .create_test_order(
            "BTC/USDT",
            OrderType::Limit,
            OrderSide::Buy,
            0.001,
            Some(40000.0),
            None,
        )
        .await
    {
        Ok(order) => {
            println!("✓ Test order validated");
            println!("  Order ID: {}", order.id);
            println!("  Symbol: {}", order.symbol);
            println!("  Side: {:?}", order.side);
            println!("  Amount: {}", order.amount);
        }
        Err(e) => println!("✗ Test order failed: {}", e),
    }
    println!();

    // Example 2: Create OCO order
    println!("📊 Example 2: Create OCO Order");
    println!("────────────────────────────────────────────────");
    println!("Scenario: Holding BTC, set take-profit at 50000, stop-loss at 45000\n");

    match binance
        .create_oco_order(
            "BTC/USDT",
            OrderSide::Sell,
            0.001,
            50000.0,
            45000.0,
            Some(44500.0),
            None,
        )
        .await
    {
        Ok(oco) => {
            println!("✓ OCO order created successfully");
            println!("  OCO Order List ID: {}", oco.order_list_id);
            println!("  Symbol: {}", oco.symbol);
            println!("  List Status: {}", oco.list_status);
            println!("  Created Time: {}", oco.datetime);
            println!("  Number of Orders: {}", oco.orders.len());

            for (i, order) in oco.orders.iter().enumerate() {
                println!("\n  Order #{}", i + 1);
                println!("    Order ID: {}", order.order_id);
                println!("    Symbol: {}", order.symbol);
            }

            let oco_order_list_id = oco.order_list_id;

            // Example 3: Query single OCO order
            println!("\n\n🔍 Example 3: Query Single OCO Order");
            println!("────────────────────────────────────────────────");
            match binance
                .fetch_oco_order(oco_order_list_id, "BTC/USDT", None)
                .await
            {
                Ok(fetched_oco) => {
                    println!("✓ Successfully queried OCO order");
                    println!("  Order List ID: {}", fetched_oco.order_list_id);
                    println!("  Status: {}", fetched_oco.list_status);
                    println!("  Is Executing: {}", fetched_oco.is_executing());
                    println!("  Is All Done: {}", fetched_oco.is_all_done());
                }
                Err(e) => println!("✗ Query failed: {}", e),
            }

            // Example 4: Cancel OCO order
            println!("\n\n❌ Example 4: Cancel OCO Order");
            println!("────────────────────────────────────────────────");
            match binance
                .cancel_oco_order(oco_order_list_id, "BTC/USDT", None)
                .await
            {
                Ok(cancelled_oco) => {
                    println!("✓ OCO order cancelled");
                    println!("  Order List ID: {}", cancelled_oco.order_list_id);
                    println!("  Status After Cancellation: {}", cancelled_oco.list_status);
                }
                Err(e) => println!("✗ Cancellation failed: {}", e),
            }
        }
        Err(e) => {
            println!("✗ Failed to create OCO order: {}", e);
            println!("  Tip: Ensure sufficient BTC balance in account");
        }
    }

    // Example 5: Query all OCO orders
    println!("\n\n📋 Example 5: Query All OCO Orders");
    println!("────────────────────────────────────────────────");
    match binance
        .fetch_oco_orders("BTC/USDT", None, Some(10), None)
        .await
    {
        Ok(oco_orders) => {
            println!("✓ Found {} OCO orders", oco_orders.len());
            for (i, oco) in oco_orders.iter().enumerate().take(3) {
                println!("\n  OCO Order #{}", i + 1);
                println!("    Order List ID: {}", oco.order_list_id);
                println!("    Symbol: {}", oco.symbol);
                println!("    Status: {}", oco.list_status);
            }
        }
        Err(e) => println!("✗ Query failed: {}", e),
    }

    // Example 6: Edit order
    println!("\n\n✏️  Example 6: Edit Existing Order");
    println!("────────────────────────────────────────────────");
    match binance
        .create_order(
            "BTC/USDT",
            OrderType::Limit,
            OrderSide::Buy,
            0.001,
            Some(40000.0),
            None,
        )
        .await
    {
        Ok(order) => {
            println!("✓ Created limit order successfully");
            println!("  Order ID: {}", order.id);
            println!("  Original Price: {:?}", order.price);

            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

            println!("\n  Modifying order price 40000 -> 39000...");
            match binance
                .edit_order(
                    &order.id,
                    "BTC/USDT",
                    OrderType::Limit,
                    OrderSide::Buy,
                    rust_decimal::Decimal::from_f64_retain(0.001).unwrap(),
                    Some(rust_decimal::Decimal::from(39000)),
                    None,
                )
                .await
            {
                Ok(new_order) => {
                    println!("✓ Order modified successfully");
                    println!("  New Order ID: {}", new_order.id);
                    println!("  New Price: {:?}", new_order.price);

                    println!("\n  Cleaning up test order...");
                    match binance.cancel_order(&new_order.id, "BTC/USDT").await {
                        Ok(_) => println!("  ✓ Test order cleaned up"),
                        Err(e) => println!("  ✗ Cleanup failed: {}", e),
                    }
                }
                Err(e) => {
                    println!("✗ Order modification failed: {}", e);
                    let _ = binance.cancel_order(&order.id, "BTC/USDT").await;
                }
            }
        }
        Err(e) => println!("✗ Failed to create limit order: {}", e),
    }

    println!("\n\n⚠️  Usage Tips");
    println!("────────────────────────────────────────────────");
    println!(
        "1. OCO orders are suitable for position management, setting take-profit and stop-loss simultaneously"
    );
    println!("2. Test orders only validate parameters, no actual order placement");
    println!("3. Order modification uses atomic operation (cancel + create)");
    println!("4. Recommend validating logic in testnet environment first");
    println!("\nExample execution complete!\n");

    Ok(())
}
