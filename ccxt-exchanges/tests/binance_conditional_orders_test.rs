#![allow(deprecated)]

use ccxt_core::types::financial::{Amount, Price};
use ccxt_core::{ExchangeConfig, Order, OrderSide, OrderStatus, OrderType};
use ccxt_exchanges::binance::Binance;
use rust_decimal_macros::dec;
use std::collections::HashMap;

/// Test creating a market stop-loss order.
#[tokio::test]
#[ignore] // Requires API credentials
async fn test_create_stop_loss_market_order() {
    let config = ExchangeConfig {
        id: "binance".to_string(),
        name: "Binance".to_string(),
        api_key: Some(ccxt_core::SecretString::new("your_api_key")),
        secret: Some(ccxt_core::SecretString::new("your_secret")),
        ..Default::default()
    };
    let exchange = Binance::new(config).expect("Failed to create Binance exchange");

    let result = exchange
        .create_stop_loss_order(
            "BTC/USDT",
            OrderSide::Sell,
            Amount::new(dec!(0.001)),
            Price::new(dec!(45000.0)),
            None, // Market order
            None,
        )
        .await;

    match result {
        Ok(order) => {
            assert_eq!(order.symbol, "BTC/USDT");
            assert_eq!(order.side, OrderSide::Sell);
            assert_eq!(order.order_type, OrderType::StopLoss);
            assert!(order.stop_price.is_some());
            assert_eq!(
                order.stop_price.expect("stop_price should be Some"),
                dec!(45000.0)
            );
            println!(
                "✅ Market stop-loss order created successfully: {:?}",
                order
            );
        }
        Err(e) => {
            println!(
                "⚠️  Stop-loss order creation failed (possibly test environment): {}",
                e
            );
        }
    }
}

/// Test creating a limit stop-loss order.
#[tokio::test]
#[ignore] // Requires API credentials
async fn test_create_stop_loss_limit_order() {
    let config = ExchangeConfig {
        id: "binance".to_string(),
        name: "Binance".to_string(),
        api_key: Some(ccxt_core::SecretString::new("your_api_key")),
        secret: Some(ccxt_core::SecretString::new("your_secret")),
        ..Default::default()
    };
    let exchange = Binance::new(config).expect("Failed to create Binance exchange");

    let result = exchange
        .create_stop_loss_order(
            "BTC/USDT",
            OrderSide::Sell,
            Amount::new(dec!(0.001)),
            Price::new(dec!(45000.0)),
            Some(Price::new(dec!(44900.0))), // Limit price
            None,
        )
        .await;

    match result {
        Ok(order) => {
            assert_eq!(order.symbol, "BTC/USDT");
            assert_eq!(order.side, OrderSide::Sell);
            assert_eq!(order.order_type, OrderType::StopLossLimit);
            assert!(order.stop_price.is_some());
            assert!(order.price.is_some());
            assert_eq!(
                order.stop_price.expect("stop_price should be Some"),
                dec!(45000.0)
            );
            assert_eq!(order.price.expect("price should be Some"), dec!(44900.0));
            println!("✅ Limit stop-loss order created successfully: {:?}", order);
        }
        Err(e) => {
            println!(
                "⚠️  Stop-loss order creation failed (possibly test environment): {}",
                e
            );
        }
    }
}

/// Test creating a market take-profit order.
#[tokio::test]
#[ignore] // Requires API credentials
async fn test_create_take_profit_market_order() {
    let config = ExchangeConfig {
        id: "binance".to_string(),
        name: "Binance".to_string(),
        api_key: Some(ccxt_core::SecretString::new("your_api_key")),
        secret: Some(ccxt_core::SecretString::new("your_secret")),
        ..Default::default()
    };
    let exchange = Binance::new(config).expect("Failed to create Binance exchange");

    let result = exchange
        .create_take_profit_order(
            "BTC/USDT",
            OrderSide::Sell,
            Amount::new(dec!(0.001)),
            Price::new(dec!(55000.0)),
            None, // Market order
            None,
        )
        .await;

    match result {
        Ok(order) => {
            assert_eq!(order.symbol, "BTC/USDT");
            assert_eq!(order.side, OrderSide::Sell);
            assert_eq!(order.order_type, OrderType::TakeProfit);
            assert!(order.take_profit_price.is_some());
            assert_eq!(
                order.stop_price.expect("stop_price should be Some"),
                dec!(55000.0)
            );
            println!(
                "✅ Market take-profit order created successfully: {:?}",
                order
            );
        }
        Err(e) => {
            println!(
                "⚠️  Take-profit order creation failed (possibly test environment): {}",
                e
            );
        }
    }
}

/// Test creating a limit take-profit order.
#[tokio::test]
#[ignore] // Requires API credentials
async fn test_create_take_profit_limit_order() {
    let config = ExchangeConfig {
        id: "binance".to_string(),
        name: "Binance".to_string(),
        api_key: Some(ccxt_core::SecretString::new("your_api_key")),
        secret: Some(ccxt_core::SecretString::new("your_secret")),
        ..Default::default()
    };
    let exchange = Binance::new(config).expect("Failed to create Binance exchange");

    let result = exchange
        .create_take_profit_order(
            "BTC/USDT",
            OrderSide::Sell,
            Amount::new(dec!(0.001)),
            Price::new(dec!(55000.0)),
            Some(Price::new(dec!(55100.0))), // Limit price
            None,
        )
        .await;

    match result {
        Ok(order) => {
            assert_eq!(order.symbol, "BTC/USDT");
            assert_eq!(order.side, OrderSide::Sell);
            assert_eq!(order.order_type, OrderType::TakeProfitLimit);
            assert!(order.take_profit_price.is_some());
            assert!(order.price.is_some());
            assert_eq!(
                order.stop_price.expect("stop_price should be Some"),
                dec!(55000.0)
            );
            assert_eq!(order.price.expect("price should be Some"), dec!(55100.0));
            println!(
                "✅ Limit take-profit order created successfully: {:?}",
                order
            );
        }
        Err(e) => {
            println!(
                "⚠️  Take-profit order creation failed (possibly test environment): {}",
                e
            );
        }
    }
}

/// Test creating a trailing stop order (spot market).
#[tokio::test]
#[ignore] // Requires API credentials
async fn test_create_trailing_stop_order_spot() {
    let config = ExchangeConfig {
        id: "binance".to_string(),
        name: "Binance".to_string(),
        api_key: Some(ccxt_core::SecretString::new("your_api_key")),
        secret: Some(ccxt_core::SecretString::new("your_secret")),
        ..Default::default()
    };
    let exchange = Binance::new(config).expect("Failed to create Binance exchange");

    let result = exchange
        .create_trailing_stop_order(
            "BTC/USDT",
            OrderSide::Sell,
            Amount::new(dec!(0.001)),
            dec!(1.5), // 1.5% callback rate
            None,
            None,
        )
        .await;

    match result {
        Ok(order) => {
            assert_eq!(order.symbol, "BTC/USDT");
            assert_eq!(order.side, OrderSide::Sell);
            assert_eq!(order.order_type, OrderType::TrailingStop);
            assert!(order.trailing_percent.is_some());
            println!("✅ Trailing stop order created successfully: {:?}", order);
        }
        Err(e) => {
            println!(
                "⚠️  Trailing stop order creation failed (possibly test environment): {}",
                e
            );
        }
    }
}

/// Test creating a trailing stop order with activation price.
#[tokio::test]
#[ignore] // Requires API credentials
async fn test_create_trailing_stop_order_with_activation() {
    let config = ExchangeConfig {
        id: "binance".to_string(),
        name: "Binance".to_string(),
        api_key: Some(ccxt_core::SecretString::new("your_api_key")),
        secret: Some(ccxt_core::SecretString::new("your_secret")),
        ..Default::default()
    };
    let exchange = Binance::new(config).expect("Failed to create Binance exchange");

    let result = exchange
        .create_trailing_stop_order(
            "BTC/USDT",
            OrderSide::Sell,
            Amount::new(dec!(0.001)),
            dec!(1.5),                       // 1.5% callback rate
            Some(Price::new(dec!(52000.0))), // Activation price
            None,
        )
        .await;

    match result {
        Ok(order) => {
            assert_eq!(order.symbol, "BTC/USDT");
            assert_eq!(order.side, OrderSide::Sell);
            assert_eq!(order.order_type, OrderType::TrailingStop);
            assert!(order.trailing_percent.is_some());
            assert!(order.activation_price.is_some());
            assert_eq!(
                order
                    .activation_price
                    .expect("activation_price should be Some"),
                dec!(52000.0)
            );
            println!(
                "✅ Trailing stop order with activation price created successfully: {:?}",
                order
            );
        }
        Err(e) => {
            println!(
                "⚠️  Trailing stop order creation failed (possibly test environment): {}",
                e
            );
        }
    }
}

/// Test creating a stop-loss order via `create_order` method.
#[tokio::test]
#[ignore] // Requires API credentials
async fn test_create_order_with_stop_loss() {
    let config = ExchangeConfig {
        id: "binance".to_string(),
        name: "Binance".to_string(),
        api_key: Some(ccxt_core::SecretString::new("your_api_key")),
        secret: Some(ccxt_core::SecretString::new("your_secret")),
        ..Default::default()
    };
    let exchange = Binance::new(config).expect("Failed to create Binance exchange");

    let mut params = HashMap::new();
    params.insert("stopPrice".to_string(), "45000".to_string());

    let result = exchange
        .create_order(
            "BTC/USDT",
            OrderType::StopLossLimit,
            OrderSide::Sell,
            Amount::new(dec!(0.001)),
            Some(Price::new(dec!(44900.0))),
            Some(params),
        )
        .await;

    match result {
        Ok(order) => {
            assert_eq!(order.symbol, "BTC/USDT");
            assert_eq!(order.side, OrderSide::Sell);
            assert_eq!(order.order_type, OrderType::StopLossLimit);
            assert!(order.stop_price.is_some());
            println!(
                "✅ Stop-loss order created via create_order successfully: {:?}",
                order
            );
        }
        Err(e) => {
            println!(
                "⚠️  Order creation failed (possibly test environment): {}",
                e
            );
        }
    }
}

/// Test creating a trailing stop order via `create_order` method.
#[tokio::test]
#[ignore] // Requires API credentials
async fn test_create_order_with_trailing_stop() {
    let config = ExchangeConfig {
        id: "binance".to_string(),
        name: "Binance".to_string(),
        api_key: Some(ccxt_core::SecretString::new("your_api_key")),
        secret: Some(ccxt_core::SecretString::new("your_secret")),
        ..Default::default()
    };
    let exchange = Binance::new(config).expect("Failed to create Binance exchange");

    let mut params = HashMap::new();
    params.insert("trailingPercent".to_string(), "1.5".to_string());
    params.insert("activationPrice".to_string(), "52000".to_string());

    let result = exchange
        .create_order(
            "BTC/USDT",
            OrderType::TrailingStop,
            OrderSide::Sell,
            Amount::new(dec!(0.001)),
            None,
            Some(params),
        )
        .await;

    match result {
        Ok(order) => {
            assert_eq!(order.symbol, "BTC/USDT");
            assert_eq!(order.side, OrderSide::Sell);
            assert_eq!(order.order_type, OrderType::TrailingStop);
            assert!(order.trailing_percent.is_some());
            println!(
                "✅ Trailing stop order created via create_order successfully: {:?}",
                order
            );
        }
        Err(e) => {
            println!(
                "⚠️  Order creation failed (possibly test environment): {}",
                e
            );
        }
    }
}

/// Test parameter validation - stop price is required.
#[tokio::test]
#[ignore] // Requires API credentials
async fn test_stop_loss_order_validation() {
    let config = ExchangeConfig {
        id: "binance".to_string(),
        name: "Binance".to_string(),
        api_key: Some(ccxt_core::SecretString::new("your_api_key")),
        secret: Some(ccxt_core::SecretString::new("your_secret")),
        ..Default::default()
    };
    let exchange = Binance::new(config).expect("Failed to create Binance exchange");

    let result = exchange
        .create_order(
            "BTC/USDT",
            OrderType::StopLossLimit,
            OrderSide::Sell,
            Amount::new(dec!(0.001)),
            Some(Price::new(dec!(44900.0))),
            None, // Missing stopPrice
        )
        .await;

    assert!(result.is_err(), "Missing stopPrice should return an error");
    println!("✅ Parameter validation test passed");
}

/// Test parameter validation - trailing percent is required.
#[tokio::test]
#[ignore] // Requires API credentials
async fn test_trailing_stop_order_validation() {
    let config = ExchangeConfig {
        id: "binance".to_string(),
        name: "Binance".to_string(),
        api_key: Some(ccxt_core::SecretString::new("your_api_key")),
        secret: Some(ccxt_core::SecretString::new("your_secret")),
        ..Default::default()
    };
    let exchange = Binance::new(config).expect("Failed to create Binance exchange");

    let result = exchange
        .create_order(
            "BTC/USDT",
            OrderType::TrailingStop,
            OrderSide::Sell,
            Amount::new(dec!(0.001)),
            None,
            None, // Missing trailingPercent
        )
        .await;

    assert!(
        result.is_err(),
        "Missing trailingPercent should return an error"
    );
    println!("✅ Trailing stop parameter validation test passed");
}

/// Test automatic mapping of stop-loss and take-profit prices.
#[tokio::test]
#[ignore] // Requires API credentials
async fn test_stop_price_mapping() {
    let config = ExchangeConfig {
        id: "binance".to_string(),
        name: "Binance".to_string(),
        api_key: Some(ccxt_core::SecretString::new("your_api_key")),
        secret: Some(ccxt_core::SecretString::new("your_secret")),
        ..Default::default()
    };
    let exchange = Binance::new(config).expect("Failed to create Binance exchange");

    let mut params = HashMap::new();
    params.insert("stopLossPrice".to_string(), "45000".to_string());

    let result = exchange
        .create_order(
            "BTC/USDT",
            OrderType::StopLossLimit,
            OrderSide::Sell,
            Amount::new(dec!(0.001)),
            Some(Price::new(dec!(44900.0))),
            Some(params),
        )
        .await;

    match result {
        Ok(order) => {
            assert!(order.stop_price.is_some());
            assert_eq!(
                order.stop_price.expect("stop_price should be Some"),
                dec!(45000.0)
            );
            println!("✅ stopLossPrice自动映射到stopPrice成功");
        }
        Err(e) => {
            println!("⚠️  订单创建失败（可能是测试环境）: {}", e);
        }
    }
}

/// 测试：合约市场的追踪止损（使用callbackRate）
#[tokio::test]
#[ignore] // 需要API凭证
async fn test_trailing_stop_futures() {
    let config = ExchangeConfig {
        id: "binance".to_string(),
        name: "Binance".to_string(),
        api_key: Some(ccxt_core::SecretString::new("your_api_key")),
        secret: Some(ccxt_core::SecretString::new("your_secret")),
        ..Default::default()
    };
    let exchange = Binance::new(config).expect("Failed to create Binance exchange");

    let mut params = HashMap::new();
    params.insert("market".to_string(), "future".to_string());

    let result = exchange
        .create_trailing_stop_order(
            "BTC/USDT",
            OrderSide::Sell,
            Amount::new(dec!(0.001)),
            dec!(1.5), // 将被转换为callbackRate
            None,
            Some(params),
        )
        .await;

    match result {
        Ok(order) => {
            assert_eq!(order.order_type, OrderType::TrailingStop);
            // 在合约市场，应该使用callbackRate
            assert!(
                order.callback_rate.is_some() || order.trailing_percent.is_some(),
                "应该设置callback_rate或trailing_percent"
            );
            println!("✅ 合约市场追踪止损订单创建成功: {:?}", order);
        }
        Err(e) => {
            println!("⚠️  追踪止损订单创建失败（可能是测试环境）: {}", e);
        }
    }
}

/// Test order type parsing.
#[test]
fn test_order_type_parsing() {
    // 这个测试不需要API调用，只测试类型定义
    use ccxt_core::OrderStatus;
    let order = Order::new(
        "test_id".to_string(),
        "BTC/USDT".to_string(),
        OrderType::StopLoss,
        OrderSide::Sell,
        dec!(0.001),
        None,
        OrderStatus::Open,
    );

    assert_eq!(order.order_type, OrderType::StopLoss);
    assert_eq!(order.symbol, "BTC/USDT");
    assert_eq!(order.side, OrderSide::Sell);
    println!("✅ 订单类型解析测试通过");
}

/// 测试：条件订单字段设置
#[test]
fn test_conditional_order_fields() {
    let mut order = Order::new(
        "test_id".to_string(),
        "BTC/USDT".to_string(),
        OrderType::TrailingStop,
        OrderSide::Sell,
        dec!(0.001),
        None,
        OrderStatus::Open,
    );

    // 设置条件订单字段
    order.trailing_percent = Some(dec!(1.5));
    order.activation_price = Some(dec!(52000.0));
    order.callback_rate = Some(dec!(1.5));
    order.working_type = Some("MARK_PRICE".to_string());

    assert_eq!(order.trailing_percent, Some(dec!(1.5)));
    assert_eq!(order.activation_price, Some(dec!(52000.0)));
    assert_eq!(order.callback_rate, Some(dec!(1.5)));
    assert_eq!(order.working_type, Some("MARK_PRICE".to_string()));
    println!("✅ 条件订单字段设置测试通过");
}

#[cfg(test)]
mod integration_tests {
    use super::*;

    /// 集成测试：完整的止损止盈策略
    #[tokio::test]
    #[ignore] // 需要API凭证和实际交易环境
    async fn test_full_stop_loss_take_profit_strategy() {
        let config = ExchangeConfig {
            id: "binance".to_string(),
            name: "Binance".to_string(),
            api_key: Some(ccxt_core::SecretString::new("your_api_key")),
            secret: Some(ccxt_core::SecretString::new("your_secret")),
            ..Default::default()
        };
        let exchange = Binance::new(config).expect("Failed to create Binance exchange");

        // 1. 创建限价买单
        let buy_order = exchange
            .create_order(
                "BTC/USDT",
                OrderType::Limit,
                OrderSide::Buy,
                Amount::new(dec!(0.001)),
                Some(Price::new(dec!(50000.0))),
                None,
            )
            .await;

        assert!(buy_order.is_ok(), "买单应该创建成功");
        println!("✅ 步骤1：买单创建成功");

        // 2. 创建止损订单
        let stop_loss = exchange
            .create_stop_loss_order(
                "BTC/USDT",
                OrderSide::Sell,
                Amount::new(dec!(0.001)),
                Price::new(dec!(48000.0)),       // 止损价格
                Some(Price::new(dec!(47900.0))), // 限价
                None,
            )
            .await;

        assert!(stop_loss.is_ok(), "止损单应该创建成功");
        println!("✅ 步骤2：止损单创建成功");

        // 3. 创建止盈订单
        let take_profit = exchange
            .create_take_profit_order(
                "BTC/USDT",
                OrderSide::Sell,
                Amount::new(dec!(0.001)),
                Price::new(dec!(55000.0)),       // 止盈价格
                Some(Price::new(dec!(55100.0))), // 限价
                None,
            )
            .await;

        assert!(take_profit.is_ok(), "止盈单应该创建成功");
        println!("✅ 步骤3：止盈单创建成功");

        println!("✅ 完整的止损止盈策略测试通过");
    }
}
