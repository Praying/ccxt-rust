//! Binance Order Enhancement Example
//!
//! Demonstrates usage of three newly added order-related methods:
//! 1. `fetch_order_trades()` - Query specific trade records for an order
//! 2. `fetch_canceled_orders()` - Query cancelled orders
//! 3. `create_market_buy_order_with_cost()` - Create market buy order by cost amount

// Allow clippy warnings for example code - examples prioritize readability over strict linting
#![allow(clippy::option_as_ref_deref)]

use ccxt_core::types::{OrderSide, OrderType};
use ccxt_exchanges::prelude::*;
use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<()> {
    let config = ExchangeConfig {
        api_key: Some("YOUR_API_KEY".to_string()),
        secret: Some("YOUR_SECRET".to_string()),
        ..Default::default()
    };

    let exchange = Binance::new(config)?;

    println!("========================================");
    println!("Binance Order Enhancement Example");
    println!("========================================\n");

    // ========================================
    // Example 1: fetch_order_trades() - Query order trade records
    // ========================================
    println!("Example 1: Query Order Trade Records");
    println!("----------------------------------------");

    let order_id = "12345678"; // Replace with actual order ID
    let symbol = "BTC/USDT";

    match exchange
        .fetch_order_trades(order_id, symbol, None, None)
        .await
    {
        Ok(trades) => {
            println!(
                "✅ Successfully fetched trade records for order {}",
                order_id
            );
            println!("Number of trades: {}", trades.len());

            for (i, trade) in trades.iter().enumerate() {
                println!("\nTrade {}:", i + 1);
                println!(
                    "  Trade ID: {}",
                    trade.id.as_ref().map(|s| s.as_str()).unwrap_or("N/A")
                );
                println!("  Timestamp: {:?}", trade.timestamp);
                println!("  Price: {:?}", trade.price);
                println!("  Amount: {:?}", trade.amount);
                println!("  Cost: {:?}", trade.cost);
                println!("  Fee: {:?}", trade.fee);
                println!("  Side: {:?}", trade.side);
            }
        }
        Err(e) => {
            println!("❌ Query failed: {}", e);
        }
    }

    println!("\n");

    // ========================================
    // Example 2: fetch_canceled_orders() - Query cancelled orders
    // ========================================
    println!("Example 2: Query Cancelled Orders");
    println!("----------------------------------------");

    let symbol = "BTC/USDT";
    let limit = 10u64;

    match exchange
        .fetch_canceled_orders(symbol, Some(limit), None)
        .await
    {
        Ok(orders) => {
            println!("✅ Successfully fetched cancelled orders");
            println!("Number of orders: {}", orders.len());

            for (i, order) in orders.iter().enumerate() {
                println!("\nOrder {}:", i + 1);
                println!("  Order ID: {}", order.id.as_str());
                println!("  Symbol: {}", order.symbol);
                println!("  Type: {:?}", order.order_type);
                println!("  Side: {:?}", order.side);
                println!("  Price: {:?}", order.price);
                println!("  Amount: {:?}", order.amount);
                println!("  Status: {:?}", order.status);
                println!("  Created: {:?}", order.timestamp);
            }
        }
        Err(e) => {
            println!("❌ Query failed: {}", e);
        }
    }

    println!("\n");

    // ========================================
    // Example 3: create_market_buy_order_with_cost() - Buy by amount
    // ========================================
    println!("Example 3: Create Market Buy Order by Cost");
    println!("----------------------------------------");

    let symbol = "BTC/USDT";
    let cost = 100.0; // Spend 100 USDT to buy BTC

    println!("Creating market buy order:");
    println!("  Symbol: {}", symbol);
    println!("  Cost Amount: {} USDT", cost);

    match exchange
        .create_market_buy_order_with_cost(symbol, cost, None)
        .await
    {
        Ok(order) => {
            println!("\n✅ Order created successfully");
            println!("  Order ID: {}", order.id.as_str());
            println!("  Symbol: {}", order.symbol);
            println!("  Type: {:?}", order.order_type);
            println!("  Side: {:?}", order.side);
            println!("  Cost: {:?}", order.cost);
            println!("  Filled: {:?}", order.filled);
            println!("  Status: {:?}", order.status);
            println!("  Created: {:?}", order.timestamp);
        }
        Err(e) => {
            println!("❌ Order creation failed: {}", e);
        }
    }

    println!("\n");

    // ========================================
    // Example 4: Use create_order() with cost parameter
    // ========================================
    println!("Example 4: Use create_order() with Cost Parameter");
    println!("----------------------------------------");

    let symbol = "ETH/USDT";
    let cost = 50.0; // Spend 50 USDT to buy ETH

    let mut params = HashMap::new();
    params.insert("cost".to_string(), cost.to_string());

    println!("Creating market buy order via create_order():");
    println!("  Symbol: {}", symbol);
    println!("  Cost Amount: {} USDT", cost);

    match exchange
        .create_order(
            symbol,
            OrderType::Market,
            OrderSide::Buy,
            cost, // Amount parameter will be ignored, using cost parameter instead
            None,
            Some(params),
        )
        .await
    {
        Ok(order) => {
            println!("\n✅ Order created successfully");
            println!("  Order ID: {}", order.id.as_str());
            println!("  Symbol: {}", order.symbol);
            println!("  Type: {:?}", order.order_type);
            println!("  Side: {:?}", order.side);
            println!("  Cost: {:?}", order.cost);
            println!("  Filled: {:?}", order.filled);
            println!("  Status: {:?}", order.status);
        }
        Err(e) => {
            println!("❌ Order creation failed: {}", e);
        }
    }

    println!("\n========================================");
    println!("Example execution complete");
    println!("========================================");

    Ok(())
}
