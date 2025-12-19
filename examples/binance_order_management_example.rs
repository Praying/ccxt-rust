//! Binance Order Management Example
//!
//! Demonstrates order editing operations including single and batch edits.

// Allow clippy warnings for example code - examples prioritize readability over strict linting
#![allow(clippy::disallowed_methods)]
#![allow(clippy::collapsible_if)]

use ccxt_core::{ExchangeConfig, OrderSide, OrderStatus, OrderType, Price};
use ccxt_exchanges::binance::Binance;
use rust_decimal_macros::dec;
use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = ExchangeConfig {
        api_key: std::env::var("BINANCE_API_KEY").ok(),
        secret: std::env::var("BINANCE_SECRET").ok(),
        sandbox: true,
        ..Default::default()
    };

    let mut exchange = Binance::new(config)?;

    println!("=== Binance Order Management Example ===\n");

    if let Err(e) = example_edit_order(&mut exchange).await {
        eprintln!("Example 5 error: {}", e);
    }

    if let Err(e) = example_edit_orders(&mut exchange).await {
        eprintln!("Example 6 error: {}", e);
    }

    println!("\n=== Example Complete ===");
    Ok(())
}

/// Example 5: Edit single order
async fn example_edit_order(exchange: &mut Binance) -> Result<(), Box<dyn std::error::Error>> {
    println!("--- Example 5: Edit Single Order ---");

    let order_id = "123456789";
    match exchange
        .edit_order(
            order_id,
            "BTC/USDT",
            OrderType::Limit,
            OrderSide::Buy,
            dec!(0.01),        // New amount
            Some(dec!(45000)), // New price
            None,
        )
        .await
    {
        Ok(order) => {
            println!("✓ Successfully edited order {}", order_id);
            println!("  New price: {:?}", order.price);
            println!("  New amount: {}", order.amount);
            println!("  Status: {:?}", order.status);
            println!("  Updated at: {:?}", order.timestamp);
        }
        Err(e) => println!("✗ Failed to edit order: {}", e),
    }

    // Note: amount is a required parameter, must pass current amount
    match exchange
        .edit_order(
            order_id,
            "BTC/USDT",
            OrderType::Limit,
            OrderSide::Buy,
            dec!(0.01),        // Keep amount unchanged
            Some(dec!(46000)), // New price
            None,
        )
        .await
    {
        Ok(order) => {
            println!("\n✓ Successfully modified order price");
            println!("  Order ID: {}", order.id);
            println!("  New price: {:?}", order.price);
        }
        Err(e) => println!("\n✗ Failed to modify price: {}", e),
    }

    // Note: price can be None, but amount is required
    match exchange
        .edit_order(
            order_id,
            "BTC/USDT",
            OrderType::Limit,
            OrderSide::Buy,
            dec!(0.02),        // New amount
            Some(dec!(45000)), // Keep price
            None,
        )
        .await
    {
        Ok(order) => {
            println!("\n✓ Successfully modified order amount");
            println!("  Order ID: {}", order.id);
            println!("  New amount: {}", order.amount);
        }
        Err(e) => println!("\n✗ Failed to modify amount: {}", e),
    }

    let mut params = HashMap::new();
    params.insert(
        "newClientOrderId".to_string(),
        "myNewOrderId123".to_string(),
    );
    params.insert(
        "cancelReplaceMode".to_string(),
        "STOP_ON_FAILURE".to_string(),
    );

    match exchange
        .edit_order(
            order_id,
            "BTC/USDT",
            OrderType::Limit,
            OrderSide::Buy,
            dec!(0.015),
            Some(dec!(47000)),
            Some(params),
        )
        .await
    {
        Ok(order) => {
            println!("\n✓ Successfully edited order with additional parameters");
            println!("  Order ID: {}", order.id);
            if let Some(client_order_id) = &order.client_order_id {
                println!("  Client Order ID: {}", client_order_id);
            }
        }
        Err(e) => println!("\n✗ Failed to edit order: {}", e),
    }

    println!();
    Ok(())
}

/// Example 6: Batch edit orders
async fn example_edit_orders(exchange: &mut Binance) -> Result<(), Box<dyn std::error::Error>> {
    println!("--- Example 6: Batch Edit Orders ---");

    let orders = vec![
        (
            "123456789",
            "BTC/USDT",
            OrderType::Limit,
            OrderSide::Buy,
            dec!(0.01),
            Some(dec!(45000)),
        ),
        (
            "987654321",
            "ETH/USDT",
            OrderType::Limit,
            OrderSide::Sell,
            dec!(0.5),
            Some(dec!(3000)),
        ),
        (
            "555666777",
            "BNB/USDT",
            OrderType::Limit,
            OrderSide::Buy,
            dec!(10),
            Some(dec!(350)),
        ),
    ];

    match exchange.edit_orders(orders).await {
        Ok(results) => {
            println!("✓ Successfully batch edited {} orders", results.len());

            for (i, order) in results.iter().enumerate() {
                println!("\n  Order {}:", i + 1);
                println!("    ID: {}", order.id);
                println!("    Symbol: {}", order.symbol);
                println!("    Price: {:?}", order.price);
                println!("    Amount: {}", order.amount);
                println!("    Status: {:?}", order.status);
            }
        }
        Err(e) => println!("✗ Failed to batch edit orders: {}", e),
    }

    let mixed_orders = vec![
        (
            "111222333",
            "BTC/USDT",
            OrderType::Limit,
            OrderSide::Buy,
            dec!(0.01),
            Some(dec!(46000)),
        ),
        (
            "444555666",
            "BTC/USDT",
            OrderType::Limit,
            OrderSide::Sell,
            dec!(0.02),
            Some(dec!(45000)),
        ),
    ];

    match exchange.edit_orders(mixed_orders).await {
        Ok(results) => {
            println!("\n✓ Successfully edited multiple orders for same symbol");
            println!("  Edited count: {}", results.len());

            let mut success_count = 0;
            for order in &results {
                if order.status == OrderStatus::Open {
                    success_count += 1;
                }
            }
            println!("  Successful: {} orders", success_count);
        }
        Err(e) => println!("\n✗ Batch edit failed: {}", e),
    }

    // Note: edit_orders does not directly support additional parameters, use edit_order instead
    println!("\nUsing cancelReplaceMode parameter requires calling edit_order individually");
    let mut params = HashMap::new();
    params.insert(
        "cancelReplaceMode".to_string(),
        "STOP_ON_FAILURE".to_string(),
    );

    match exchange
        .edit_order(
            "777888999",
            "BTC/USDT",
            OrderType::Limit,
            OrderSide::Buy,
            dec!(0.015),
            Some(dec!(45500)),
            Some(params),
        )
        .await
    {
        Ok(order) => {
            println!("\n✓ Successfully edited order with cancelReplaceMode parameter");
            println!("  Order ID: {}", order.id);
            println!("  Status: {:?}", order.status);
        }
        Err(e) => println!("\n✗ Edit failed: {}", e),
    }

    println!();
    Ok(())
}

// ============================================================================
// Advanced Use Cases
// ============================================================================

/// Advanced Scenario 1: Monitor and manage historical orders
#[allow(dead_code)]
async fn advanced_example_monitor_orders(
    exchange: &mut Binance,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("--- Advanced Scenario 1: Monitor and Manage Historical Orders ---");

    let one_day_ago = chrono::Utc::now().timestamp_millis() - 86400000;

    match exchange
        .fetch_canceled_and_closed_orders("BTC/USDT", Some(one_day_ago as u64), None)
        .await
    {
        Ok(orders) => {
            println!("✓ {} historical orders in the last 24 hours", orders.len());

            let mut stats = HashMap::new();
            stats.insert("canceled", 0);
            stats.insert("filled", 0);
            stats.insert("partial", 0);

            let mut total_volume = dec!(0);

            for order in &orders {
                match order.status {
                    OrderStatus::Cancelled => *stats.get_mut("canceled").unwrap() += 1,
                    OrderStatus::Closed => {
                        *stats.get_mut("filled").unwrap() += 1;
                        if let Some(filled) = order.filled {
                            if let Some(price) = order.price {
                                total_volume += filled * price;
                            }
                        }
                    }
                    OrderStatus::Partial => *stats.get_mut("partial").unwrap() += 1,
                    _ => {}
                }
            }

            println!("\nOrder Statistics:");
            println!("  Cancelled: {}", stats["canceled"]);
            println!("  Filled: {}", stats["filled"]);
            println!("  Partially Filled: {}", stats["partial"]);
            println!("  Total Volume: {} USDT", total_volume);
        }
        Err(e) => println!("✗ Failed to fetch historical orders: {}", e),
    }

    println!();
    Ok(())
}

/// Advanced Scenario 2: Intelligent order adjustment strategy
#[allow(dead_code)]
async fn advanced_example_smart_order_adjustment(
    exchange: &mut Binance,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("--- Advanced Scenario 2: Intelligent Order Adjustment Strategy ---");

    let symbol = "BTC/USDT";

    match exchange.fetch_open_orders(Some(symbol)).await {
        Ok(open_orders) => {
            println!("✓ Found {} open orders", open_orders.len());

            let ticker = exchange
                .fetch_ticker(symbol, ccxt_core::types::TickerParams::default())
                .await?;
            let current_price = ticker.last.unwrap_or(Price::from(dec!(0)));

            println!("\nCurrent market price: {} USDT", current_price);
            println!("\nAnalyzing orders for adjustment...");

            for order in &open_orders {
                if let Some(order_price) = order.price {
                    let order_price = Price::from(order_price);
                    let price_diff_pct = ((order_price - current_price).as_decimal()
                        / current_price.as_decimal()
                        * dec!(100))
                    .abs();

                    println!("\nOrder ID: {}", order.id);
                    println!("  Order Price: {} USDT", order_price);
                    println!("  Price Difference: {:.2}%", price_diff_pct);

                    if price_diff_pct > dec!(5) {
                        println!("  ⚠️  Price deviation exceeds 5%, adjustment recommended");

                        let new_price = match order.side {
                            OrderSide::Buy => current_price * dec!(0.98),
                            OrderSide::Sell => current_price * dec!(1.02),
                        };

                        println!("  Recommended new price: {} USDT", new_price);
                    } else {
                        println!("  ✓ Price within reasonable range");
                    }
                }
            }
        }
        Err(e) => println!("✗ Failed to fetch open orders: {}", e),
    }

    println!();
    Ok(())
}

/// Advanced Scenario 3: Batch order optimization
#[allow(dead_code)]
async fn advanced_example_batch_optimization(
    exchange: &mut Binance,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("--- Advanced Scenario 3: Batch Order Optimization ---");

    let symbols = vec!["BTC/USDT", "ETH/USDT", "BNB/USDT"];

    println!("Analyzing orders across multiple symbols...\n");

    for symbol in &symbols {
        match exchange.fetch_open_orders(Some(symbol)).await {
            Ok(orders) => {
                if !orders.is_empty() {
                    println!("Symbol: {}", symbol);
                    println!("  Open orders: {}", orders.len());

                    let mut buy_orders = 0;
                    let mut sell_orders = 0;

                    for order in &orders {
                        match order.side {
                            OrderSide::Buy => buy_orders += 1,
                            OrderSide::Sell => sell_orders += 1,
                        }
                    }

                    println!("  Buy orders: {}", buy_orders);
                    println!("  Sell orders: {}", sell_orders);

                    if buy_orders > sell_orders * 2 {
                        println!("  ⚠️  Buy orders dominate, consider rebalancing");
                    } else if sell_orders > buy_orders * 2 {
                        println!("  ⚠️  Sell orders dominate, consider rebalancing");
                    } else {
                        println!("  ✓ Order balance looks good");
                    }
                    println!();
                }
            }
            Err(e) => println!("✗ Failed to fetch orders for {}: {}\n", symbol, e),
        }
    }

    Ok(())
}
