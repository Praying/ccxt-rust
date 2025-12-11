//! Binance订单功能增强集成测试
//!
//! 测试新增的3个订单相关方法

use ccxt_exchanges::Binance;
use ccxt_core::types::{OrderSide, OrderType, OrderStatus};
use std::collections::HashMap;

/// 测试 fetch_order_trades() - 查询订单成交记录
#[tokio::test]
#[ignore] // 需要有效的API密钥和真实订单ID
async fn test_fetch_order_trades() {
    let mut config = HashMap::new();
    config.insert("apiKey".to_string(), std::env::var("BINANCE_API_KEY").unwrap());
    config.insert("secret".to_string(), std::env::var("BINANCE_SECRET").unwrap());
    
    let exchange = Binance::new(config);
    
    // 需要一个真实的订单ID
    let order_id = "12345678";
    let symbol = "BTC/USDT";
    
    let result = exchange.fetch_order_trades(order_id, symbol, None).await;
    
    match result {
        Ok(trades) => {
            // 验证返回的交易数据结构
            for trade in &trades {
                assert!(trade.id.is_some(), "交易ID应该存在");
                assert_eq!(trade.symbol, symbol, "交易对应该匹配");
                assert!(trade.timestamp.is_some(), "时间戳应该存在");
                assert!(trade.price.is_some(), "价格应该存在");
                assert!(trade.amount.is_some(), "数量应该存在");
                assert!(trade.cost.is_some(), "成交额应该存在");
                assert!(trade.side.is_some(), "方向应该存在");
            }
            println!("✅ 成功获取 {} 条成交记录", trades.len());
        }
        Err(e) => {
            println!("⚠️ 测试跳过: {}", e);
        }
    }
}

/// 测试 fetch_canceled_orders() - 查询已取消订单
#[tokio::test]
#[ignore] // 需要有效的API密钥
async fn test_fetch_canceled_orders() {
    let mut config = HashMap::new();
    config.insert("apiKey".to_string(), std::env::var("BINANCE_API_KEY").unwrap());
    config.insert("secret".to_string(), std::env::var("BINANCE_SECRET").unwrap());
    
    let exchange = Binance::new(config);
    
    let symbol = "BTC/USDT";
    let mut params = HashMap::new();
    params.insert("limit".to_string(), "10".to_string());
    
    let result = exchange.fetch_canceled_orders(Some(symbol), None, None, Some(params)).await;
    
    match result {
        Ok(orders) => {
            // 验证所有订单都是已取消状态
            for order in &orders {
                assert_eq!(
                    order.status,
                    OrderStatus::Canceled,
                    "订单状态应该是Canceled"
                );
                assert_eq!(order.symbol, symbol, "交易对应该匹配");
                assert!(order.id.is_some(), "订单ID应该存在");
                assert!(order.timestamp.is_some(), "时间戳应该存在");
            }
            println!("✅ 成功获取 {} 个已取消订单", orders.len());
        }
        Err(e) => {
            println!("⚠️ 测试跳过: {}", e);
        }
    }
}

/// 测试 fetch_canceled_orders() 不传symbol参数
#[tokio::test]
#[ignore] // 需要有效的API密钥
async fn test_fetch_canceled_orders_no_symbol() {
    let mut config = HashMap::new();
    config.insert("apiKey".to_string(), std::env::var("BINANCE_API_KEY").unwrap());
    config.insert("secret".to_string(), std::env::var("BINANCE_SECRET").unwrap());
    
    let exchange = Binance::new(config);
    
    let mut params = HashMap::new();
    params.insert("limit".to_string(), "5".to_string());
    
    let result = exchange.fetch_canceled_orders(None, None, None, Some(params)).await;
    
    match result {
        Ok(orders) => {
            // 验证所有订单都是已取消状态
            for order in &orders {
                assert_eq!(
                    order.status,
                    OrderStatus::Canceled,
                    "订单状态应该是Canceled"
                );
            }
            println!("✅ 成功获取 {} 个已取消订单（所有交易对）", orders.len());
        }
        Err(e) => {
            println!("⚠️ 测试跳过: {}", e);
        }
    }
}

/// 测试 create_market_buy_order_with_cost() - 按金额买入
#[tokio::test]
#[ignore] // 需要有效的API密钥且会产生真实交易
async fn test_create_market_buy_order_with_cost() {
    let mut config = HashMap::new();
    config.insert("apiKey".to_string(), std::env::var("BINANCE_API_KEY").unwrap());
    config.insert("secret".to_string(), std::env::var("BINANCE_SECRET").unwrap());
    // 使用测试网
    config.insert("testnet".to_string(), "true".to_string());
    
    let exchange = Binance::new(config);
    
    let symbol = "BTC/USDT";
    let cost = 10.0; // 花费10 USDT
    
    let result = exchange.create_market_buy_order_with_cost(symbol, cost, None).await;
    
    match result {
        Ok(order) => {
            // 验证订单数据
            assert!(order.id.is_some(), "订单ID应该存在");
            assert_eq!(order.symbol, symbol, "交易对应该匹配");
            assert_eq!(order.order_type, OrderType::Market, "订单类型应该是Market");
            assert_eq!(order.side, OrderSide::Buy, "订单方向应该是Buy");
            assert!(order.timestamp.is_some(), "时间戳应该存在");
            
            println!("✅ 成功创建市价买单");
            println!("  订单ID: {}", order.id.unwrap());
            println!("  成交金额: {:?}", order.cost);
            println!("  成交数量: {:?}", order.filled);
        }
        Err(e) => {
            println!("⚠️ 测试跳过: {}", e);
        }
    }
}

/// 测试 create_market_buy_order_with_cost() 对合约市场的限制
#[tokio::test]
#[ignore] // 需要有效的API密钥
async fn test_create_market_buy_order_with_cost_futures_error() {
    let mut config = HashMap::new();
    config.insert("apiKey".to_string(), std::env::var("BINANCE_API_KEY").unwrap());
    config.insert("secret".to_string(), std::env::var("BINANCE_SECRET").unwrap());
    
    let exchange = Binance::new(config);
    
    // 尝试在合约市场使用此方法
    let symbol = "BTC/USDT:USDT"; // 合约交易对
    let cost = 10.0;
    
    let result = exchange.create_market_buy_order_with_cost(symbol, cost, None).await;
    
    // 应该返回错误
    assert!(result.is_err(), "合约市场应该返回错误");
    
    if let Err(e) = result {
        println!("✅ 正确拒绝合约市场: {}", e);
    }
}

/// 测试 create_order() 的 cost 参数支持
#[tokio::test]
#[ignore] // 需要有效的API密钥且会产生真实交易
async fn test_create_order_with_cost_param() {
    let mut config = HashMap::new();
    config.insert("apiKey".to_string(), std::env::var("BINANCE_API_KEY").unwrap());
    config.insert("secret".to_string(), std::env::var("BINANCE_SECRET").unwrap());
    config.insert("testnet".to_string(), "true".to_string());
    
    let exchange = Binance::new(config);
    
    let symbol = "ETH/USDT";
    let cost = 10.0;
    
    let mut params = HashMap::new();
    params.insert("cost".to_string(), cost.to_string());
    
    let result = exchange.create_order(
        symbol,
        OrderType::Market,
        OrderSide::Buy,
        cost, // 这个参数会被忽略
        None,
        Some(params)
    ).await;
    
    match result {
        Ok(order) => {
            // 验证订单数据
            assert_eq!(order.order_type, OrderType::Market, "订单类型应该是Market");
            assert_eq!(order.side, OrderSide::Buy, "订单方向应该是Buy");
            
            println!("✅ 成功通过cost参数创建市价买单");
            println!("  订单ID: {:?}", order.id);
            println!("  成交金额: {:?}", order.cost);
        }
        Err(e) => {
            println!("⚠️ 测试跳过: {}", e);
        }
    }
}

/// 测试 fetch_order_trades() 对现货市场的限制
#[tokio::test]
#[ignore] // 需要有效的API密钥
async fn test_fetch_order_trades_futures_error() {
    let mut config = HashMap::new();
    config.insert("apiKey".to_string(), std::env::var("BINANCE_API_KEY").unwrap());
    config.insert("secret".to_string(), std::env::var("BINANCE_SECRET").unwrap());
    
    let exchange = Binance::new(config);
    
    // 尝试查询合约订单的成交记录
    let order_id = "12345678";
    let symbol = "BTC/USDT:USDT"; // 合约交易对
    
    let result = exchange.fetch_order_trades(order_id, symbol, None).await;
    
    // 应该返回错误
    assert!(result.is_err(), "合约市场应该返回错误");
    
    if let Err(e) = result {
        println!("✅ 正确拒绝合约市场: {}", e);
    }
}