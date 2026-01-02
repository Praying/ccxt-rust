// Allow clippy warnings for example code - examples prioritize readability over strict linting
#![allow(unused_variables)]
#![allow(deprecated)]

use ccxt_core::ExchangeConfig;
use ccxt_core::types::{Amount, Price};
use ccxt_core::types::{OrderSide, OrderType};
use ccxt_exchanges::binance::Binance;
use rust_decimal_macros::dec;
use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = ExchangeConfig {
        api_key: std::env::var("BINANCE_API_KEY").ok(),
        secret: std::env::var("BINANCE_API_SECRET").ok(),
        sandbox: true,
        ..Default::default()
    };

    let exchange = Binance::new(config)?;

    println!("=== Binance Conditional Orders Example ===\n");

    example_1_market_stop_loss(&exchange).await?;
    example_2_limit_stop_loss(&exchange).await?;
    example_3_market_take_profit(&exchange).await?;
    example_4_limit_take_profit(&exchange).await?;
    example_5_trailing_stop(&exchange).await?;
    example_6_trailing_stop_with_activation(&exchange).await?;
    example_7_create_order_method(&exchange).await?;
    example_8_complete_strategy(&exchange).await?;

    println!("\n=== All examples completed ===");
    Ok(())
}

/// Example 1: Create market stop-loss order
async fn example_1_market_stop_loss(exchange: &Binance) -> Result<(), Box<dyn std::error::Error>> {
    println!("--- Example 1: Market Stop-Loss Order ---");
    println!("Scenario: Sell 0.001 BTC at market price when BTC drops to 45000 USDT");

    match exchange
        .create_stop_loss_order(
            "BTC/USDT",
            OrderSide::Sell,
            Amount::new(dec!(0.001)),
            Price::new(dec!(45000.0)),
            None,
            None,
        )
        .await
    {
        Ok(order) => {
            println!("✅ Market stop-loss order created successfully:");
            println!("   Order ID: {}", order.id);
            println!("   Order Type: {:?}", order.order_type);
            println!("   Stop Price: {:?}", order.stop_price);
        }
        Err(e) => {
            println!("⚠️  Order creation failed: {}", e);
        }
    }
    println!();
    Ok(())
}

/// Example 2: Create limit stop-loss order
async fn example_2_limit_stop_loss(exchange: &Binance) -> Result<(), Box<dyn std::error::Error>> {
    println!("--- Example 2: Limit Stop-Loss Order ---");
    println!("Scenario: When BTC drops to 45000, sell at limit price 44900");

    match exchange
        .create_stop_loss_order(
            "BTC/USDT",
            OrderSide::Sell,
            Amount::new(dec!(0.001)),
            Price::new(dec!(45000.0)),
            Some(Price::new(dec!(44900.0))),
            None,
        )
        .await
    {
        Ok(order) => {
            println!("✅ Limit stop-loss order created successfully:");
            println!("   Stop Price: {:?}", order.stop_price);
            println!("   Limit Price: {:?}", order.price);
            println!("   Advantage: Controls minimum sell price");
        }
        Err(e) => {
            println!("⚠️  Order creation failed: {}", e);
        }
    }
    println!();
    Ok(())
}

/// Example 3: Create market take-profit order
async fn example_3_market_take_profit(
    exchange: &Binance,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("--- Example 3: Market Take-Profit Order ---");
    println!("Scenario: Sell at market price when BTC rises to 55000");

    match exchange
        .create_take_profit_order(
            "BTC/USDT",
            OrderSide::Sell,
            Amount::new(dec!(0.001)),
            Price::new(dec!(55000.0)),
            None,
            None,
        )
        .await
    {
        Ok(order) => {
            println!("✅ Market take-profit order created successfully:");
            println!("   Take-Profit Price: {:?}", order.take_profit_price);
            println!("   Advantage: Quick profit locking");
        }
        Err(e) => {
            println!("⚠️  Order creation failed: {}", e);
        }
    }
    println!();
    Ok(())
}

/// Example 4: Create limit take-profit order
async fn example_4_limit_take_profit(exchange: &Binance) -> Result<(), Box<dyn std::error::Error>> {
    println!("--- Example 4: Limit Take-Profit Order ---");
    println!("Scenario: When BTC rises to 55000, sell at limit price 55100");

    match exchange
        .create_take_profit_order(
            "BTC/USDT",
            OrderSide::Sell,
            Amount::new(dec!(0.001)),
            Price::new(dec!(55000.0)),
            Some(Price::new(dec!(55100.0))),
            None,
        )
        .await
    {
        Ok(order) => {
            println!("✅ Limit take-profit order created successfully:");
            println!("   Take-Profit Price: {:?}", order.take_profit_price);
            println!("   Limit Price: {:?}", order.price);
            println!("   Advantage: Potential for better execution price");
        }
        Err(e) => {
            println!("⚠️  Order creation failed: {}", e);
        }
    }
    println!();
    Ok(())
}

/// Example 5: Create trailing stop order
async fn example_5_trailing_stop(exchange: &Binance) -> Result<(), Box<dyn std::error::Error>> {
    println!("--- Example 5: Trailing Stop Order ---");
    println!("Scenario: Follows price upward, triggers on 1.5% pullback");

    match exchange
        .create_trailing_stop_order(
            "BTC/USDT",
            OrderSide::Sell,
            Amount::new(dec!(0.001)),
            dec!(1.5),
            None,
            None,
        )
        .await
    {
        Ok(order) => {
            println!("✅ Trailing stop order created successfully:");
            println!("   Callback Rate: {:?}%", order.trailing_percent);
            println!("   How it works:");
            println!("     - Stop price follows automatically when price rises");
            println!("     - Triggers when price pulls back 1.5% from peak");
            println!("   Advantage: Maximizes profit while protecting gains");
        }
        Err(e) => {
            println!("⚠️  Order creation failed: {}", e);
        }
    }
    println!();
    Ok(())
}

/// Example 6: Create trailing stop order with activation price
async fn example_6_trailing_stop_with_activation(
    exchange: &Binance,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("--- Example 6: Trailing Stop with Activation Price ---");
    println!("Scenario: Start tracking after price reaches 52000");

    match exchange
        .create_trailing_stop_order(
            "BTC/USDT",
            OrderSide::Sell,
            Amount::new(dec!(0.001)),
            dec!(1.5),
            Some(Price::new(dec!(52000.0))),
            None,
        )
        .await
    {
        Ok(order) => {
            println!("✅ Trailing stop with activation price created successfully:");
            println!("   Activation Price: {:?}", order.activation_price);
            println!("   Callback Rate: {:?}%", order.trailing_percent);
            println!("   Use Case: Enable trailing stop after breakeven");
        }
        Err(e) => {
            println!("⚠️  Order creation failed: {}", e);
        }
    }
    println!();
    Ok(())
}

/// Example 7: Create conditional orders using create_order method
async fn example_7_create_order_method(
    exchange: &Binance,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("--- Example 7: Using create_order Method ---");
    let mut stop_params = HashMap::new();
    stop_params.insert("stopPrice".to_string(), "45000".to_string());

    match exchange
        .create_order(
            "BTC/USDT",
            OrderType::StopLossLimit,
            OrderSide::Sell,
            Amount::new(dec!(0.001)),
            Some(Price::new(dec!(44900.0))),
            Some(stop_params),
        )
        .await
    {
        Ok(order) => {
            println!("✅ Stop-loss order created via create_order successfully");
            println!("   Order ID: {}", order.id);
        }
        Err(e) => {
            println!("⚠️  Creation failed: {}", e);
        }
    }

    let mut trailing_params = HashMap::new();
    trailing_params.insert("trailingPercent".to_string(), "2.0".to_string());

    match exchange
        .create_order(
            "BTC/USDT",
            OrderType::TrailingStop,
            OrderSide::Sell,
            Amount::new(dec!(0.001)),
            None,
            Some(trailing_params),
        )
        .await
    {
        Ok(order) => {
            println!("✅ Trailing stop order created via create_order successfully");
            println!("   Order ID: {}", order.id);
        }
        Err(e) => {
            println!("⚠️  Creation failed: {}", e);
        }
    }

    println!();
    Ok(())
}

/// Example 8: Complete trading strategy
async fn example_8_complete_strategy(exchange: &Binance) -> Result<(), Box<dyn std::error::Error>> {
    println!("--- Example 8: Complete Trading Strategy ---");
    println!("Strategy: Buy at 50000, set stop-loss at 48000 and take-profit at 55000");

    println!("\nStep 1: Create limit buy order");
    match exchange
        .create_order(
            "BTC/USDT",
            OrderType::Limit,
            OrderSide::Buy,
            Amount::new(dec!(0.001)),
            Some(Price::new(dec!(50000.0))),
            None,
        )
        .await
    {
        Ok(order) => {
            println!("   ✅ Buy order created successfully, ID: {}", order.id);
        }
        Err(e) => {
            println!("   ⚠️  Buy order creation failed: {}", e);
            return Ok(());
        }
    }

    println!("\nStep 2: Create stop-loss order (risk control)");
    match exchange
        .create_stop_loss_order(
            "BTC/USDT",
            OrderSide::Sell,
            Amount::new(dec!(0.001)),
            Price::new(dec!(48000.0)),
            Some(Price::new(dec!(47900.0))),
            None,
        )
        .await
    {
        Ok(order) => {
            println!("   ✅ Stop-loss order created successfully");
            println!("   Maximum loss: ~4%");
        }
        Err(e) => {
            println!("   ⚠️  Stop-loss order creation failed: {}", e);
        }
    }

    println!("\nStep 3: Create take-profit order (profit target)");
    match exchange
        .create_take_profit_order(
            "BTC/USDT",
            OrderSide::Sell,
            Amount::new(dec!(0.001)),
            Price::new(dec!(55000.0)),
            Some(Price::new(dec!(55100.0))),
            None,
        )
        .await
    {
        Ok(order) => {
            println!("   ✅ Take-profit order created successfully");
            println!("   Target profit: ~10%");
        }
        Err(e) => {
            println!("   ⚠️  Take-profit order creation failed: {}", e);
        }
    }

    println!("\nStrategy Summary:");
    println!("   Buy Price: 50000 USDT");
    println!("   Stop-Loss: 48000 USDT (-4%)");
    println!("   Take-Profit: 55000 USDT (+10%)");
    println!("   Risk-Reward Ratio: 1:2.5");
    println!();
    Ok(())
}
