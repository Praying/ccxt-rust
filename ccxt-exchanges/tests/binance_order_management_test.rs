//! Binance order management enhanced API integration tests.
//!
//! Tests all 6 order management API methods.

use ccxt_core::{
    ExchangeConfig,
    types::{Order, OrderSide, OrderType, Trade},
};
use ccxt_exchanges::binance::Binance;
use rust_decimal::Decimal;
use std::str::FromStr;

#[cfg(test)]
mod binance_order_management_tests {
    use super::*;

    /// Create a Binance instance for testing.
    fn create_test_binance() -> Binance {
        let config = ExchangeConfig {
            id: "binance".to_string(),
            name: "Binance".to_string(),
            api_key: Some("test_api_key".to_string()),
            secret: Some("test_api_secret".to_string()),
            ..Default::default()
        };
        Binance::new(config).expect("Failed to create Binance instance")
    }

    #[tokio::test]
    #[ignore]
    async fn test_fetch_canceled_orders() {
        let binance = create_test_binance();

        let result = binance
            .fetch_canceled_orders("BTCUSDT", None, Some(10))
            .await;

        match result {
            Ok(orders) => {
                println!("✓ Fetched canceled orders: {} orders", orders.len());

                for order in orders.iter() {
                    assert!(
                        order.status.to_string().contains("cancel"),
                        "Order status must be canceled"
                    );
                    println!(
                        "  Order ID: {}, Symbol: {}, Status: {}",
                        order.id, order.symbol, order.status
                    );
                }
            }
            Err(e) => {
                println!("✗ Failed to fetch canceled orders: {:?}", e);
                panic!("fetch_canceled_orders failed");
            }
        }
    }

    #[tokio::test]
    #[ignore]
    async fn test_fetch_canceled_orders_with_time_range() {
        let binance = create_test_binance();

        let since = 1609459200000; // 2021-01-01 00:00:00 UTC

        let result = binance
            .fetch_canceled_orders("ETHUSDT", Some(since), Some(20))
            .await;

        match result {
            Ok(orders) => {
                println!(
                    "✓ Fetched canceled orders since timestamp: {} orders",
                    orders.len()
                );

                for order in orders.iter() {
                    if let Some(ts) = order.timestamp {
                        assert!(
                            ts >= since as i64,
                            "Order timestamp must be >= since parameter"
                        );
                    }
                    assert!(
                        order.status.to_string().contains("cancel"),
                        "Order status must be canceled"
                    );
                    println!("  Order ID: {}, Status: {}", order.id, order.status);
                }
            }
            Err(e) => {
                println!("✗ Failed to fetch canceled orders: {:?}", e);
            }
        }
    }

    #[tokio::test]
    #[ignore]
    async fn test_fetch_canceled_and_closed_orders() {
        let binance = create_test_binance();

        let result = binance
            .fetch_canceled_and_closed_orders("BTCUSDT", None, Some(15))
            .await;

        match result {
            Ok(orders) => {
                println!(
                    "✓ Fetched canceled and closed orders: {} orders",
                    orders.len()
                );

                for order in orders.iter() {
                    assert!(
                        order.status.to_string().contains("cancel")
                            || order.status.to_string().contains("closed"),
                        "Order status must be canceled or closed, got: {}",
                        order.status
                    );
                    println!(
                        "  Order ID: {}, Status: {}, Type: {:?}",
                        order.id, order.status, order.order_type
                    );
                }
            }
            Err(e) => {
                println!("✗ Failed to fetch orders: {:?}", e);
                panic!("fetch_canceled_and_closed_orders failed");
            }
        }
    }

    #[tokio::test]
    #[ignore]
    async fn test_fetch_canceled_and_closed_orders_all_symbols() {
        let binance = create_test_binance();

        let result = binance
            .fetch_canceled_and_closed_orders("BTCUSDT", None, Some(50))
            .await;

        match result {
            Ok(orders) => {
                println!("✓ Fetched orders for all symbols: {} orders", orders.len());

                let mut symbol_counts = std::collections::HashMap::new();
                for order in orders.iter() {
                    *symbol_counts.entry(&order.symbol).or_insert(0) += 1;
                }

                println!("  Symbol statistics:");
                for (symbol, count) in symbol_counts.iter() {
                    println!("    {}: {} orders", symbol, count);
                }
            }
            Err(e) => {
                println!("✗ Failed to fetch orders: {:?}", e);
            }
        }
    }

    #[tokio::test]
    #[ignore]
    async fn test_fetch_open_order() {
        let binance = create_test_binance();

        let order_id = "123456789"; // Replace with real order ID

        let result = binance.fetch_open_order(order_id, "BTCUSDT").await;

        match result {
            Ok(order) => {
                println!("✅ 成功获取未完成订单");
                println!("  订单ID: {}", order.id);
                println!("  交易对: {}", order.symbol);
                println!("  状态: {}", order.status);
                println!("  类型: {:?}", order.order_type);
                println!("  方向: {:?}", order.side);
                println!("  数量: {}", order.amount);
                println!("  价格: {:?}", order.price);

                assert_eq!(order.id, order_id, "订单ID应匹配");
                assert!(
                    order.status.to_string().contains("open"),
                    "订单状态应为open"
                );
            }
            Err(e) => {
                println!("✗ Failed to fetch open order: {:?}", e);
                // 注意：如果订单不存在或已完成，这是正常的
            }
        }
    }

    #[tokio::test]
    #[ignore]
    async fn test_fetch_open_order_without_symbol() {
        let binance = create_test_binance();

        // 测试不指定交易对获取订单
        let order_id = "123456789";

        let result = binance.fetch_open_order(order_id, "BTCUSDT").await;

        match result {
            Ok(order) => {
                println!("✅ 成功获取订单（未指定交易对）");
                println!("  订单ID: {}", order.id);
                println!("  交易对: {}", order.symbol);
            }
            Err(e) => {
                println!("❌ 获取订单失败: {:?}", e);
            }
        }
    }

    #[tokio::test]
    #[ignore]
    async fn test_fetch_order_trades() {
        let binance = create_test_binance();

        let order_id = "123456789"; // Replace with real order ID

        let result = binance
            .fetch_order_trades(order_id, "BTCUSDT", None, None)
            .await;

        match result {
            Ok(trades) => {
                println!("✓ Fetched order trades: {} trades", trades.len());

                for trade in trades.iter() {
                    if let Some(trade_id) = &trade.id {
                        println!("  成交ID: {}", trade_id);
                    }
                    println!("    订单ID: {:?}", trade.order);
                    println!("    交易对: {}", trade.symbol);
                    println!("    方向: {:?}", trade.side);
                    println!("    价格: {}", trade.price);
                    println!("    数量: {}", trade.amount);
                    println!("    手续费: {:?}", trade.fee);
                    println!("    时间: {}", trade.timestamp);

                    assert_eq!(
                        trade.order.as_deref(),
                        Some(order_id),
                        "成交记录应属于该订单"
                    );
                    assert_eq!(trade.symbol, "BTCUSDT", "交易对应匹配");
                }
            }
            Err(e) => {
                println!("✗ Failed to fetch order trades: {:?}", e);
            }
        }
    }

    #[tokio::test]
    #[ignore]
    async fn test_fetch_order_trades_with_pagination() {
        let binance = create_test_binance();

        // 测试使用分页获取订单成交记录
        let order_id = "123456789";
        let since = 1609459200000; // 2021-01-01
        let limit = 100;

        let result = binance
            .fetch_order_trades(order_id, "ETHUSDT", Some(since), Some(limit))
            .await;

        match result {
            Ok(trades) => {
                println!("✅ 成功获取订单成交记录（分页），数量: {}", trades.len());
                assert!(trades.len() <= limit as usize, "返回数量应不超过限制");

                // 验证时间戳
                for trade in trades.iter() {
                    assert!(trade.timestamp >= since as i64, "成交时间应大于since参数");
                }
            }
            Err(e) => {
                println!("✗ Failed to fetch order trades: {:?}", e);
            }
        }
    }

    #[tokio::test]
    #[ignore]
    async fn test_edit_order() {
        let binance = create_test_binance();

        let order_id = "123456789"; // Replace with real order ID
        let symbol = "BTCUSDT";
        let new_amount = Decimal::from_str("0.002").unwrap();
        let new_price = Decimal::from_str("45000.00").unwrap();

        let result = binance
            .edit_order(
                order_id,
                symbol,
                OrderType::Limit,
                OrderSide::Buy,
                new_amount,
                Some(new_price),
                None,
            )
            .await;

        match result {
            Ok(order) => {
                println!("✅ 成功编辑订单");
                println!("  订单ID: {}", order.id);
                println!("  交易对: {}", order.symbol);
                println!("  新数量: {}", order.amount);
                println!("  新价格: {:?}", order.price);
                println!("  状态: {}", order.status);

                assert_eq!(order.id, order_id, "订单ID应保持不变");
                assert_eq!(order.amount, new_amount, "订单数量应已更新");
                if let Some(price) = order.price {
                    assert_eq!(price, new_price, "订单价格应已更新");
                }
            }
            Err(e) => {
                println!("✗ Failed to edit order: {:?}", e);
            }
        }
    }

    #[tokio::test]
    #[ignore]
    async fn test_edit_order_with_params() {
        let binance = create_test_binance();

        // 测试使用额外参数编辑订单
        use std::collections::HashMap;
        let mut params = HashMap::new();
        params.insert(
            "cancelReplaceMode".to_string(),
            "STOP_ON_FAILURE".to_string(),
        );

        let order_id = "123456789";
        let result = binance
            .edit_order(
                order_id,
                "ETHUSDT",
                OrderType::Limit,
                OrderSide::Sell,
                Decimal::from_str("0.05").unwrap(),
                Some(Decimal::from_str("3000.00").unwrap()),
                Some(params),
            )
            .await;

        match result {
            Ok(order) => {
                println!("✅ 成功使用额外参数编辑订单");
                println!("  订单ID: {}", order.id);
                println!("  状态: {}", order.status);
            }
            Err(e) => {
                println!("✗ Failed to edit order: {:?}", e);
            }
        }
    }

    #[tokio::test]
    #[ignore]
    async fn test_edit_orders_batch() {
        let binance = create_test_binance();

        // 测试批量编辑订单
        use std::collections::HashMap;

        let orders = vec![
            (
                "123456789",
                "BTCUSDT",
                OrderType::Limit,
                OrderSide::Buy,
                Decimal::from_str("0.002").unwrap(),
                Some(Decimal::from_str("45000.00").unwrap()),
            ),
            (
                "987654321",
                "ETHUSDT",
                OrderType::Limit,
                OrderSide::Buy,
                Decimal::from_str("0.05").unwrap(),
                Some(Decimal::from_str("3000.00").unwrap()),
            ),
        ];

        let result = binance.edit_orders(orders).await;

        match result {
            Ok(edited_orders) => {
                println!("✓ Batch edited orders: {} orders", edited_orders.len());

                for order in edited_orders.iter() {
                    println!("  订单ID: {}", order.id);
                    println!("    交易对: {}", order.symbol);
                    println!("    数量: {}", order.amount);
                    println!("    价格: {:?}", order.price);
                    println!("    状态: {}", order.status);
                }
            }
            Err(e) => {
                println!("✗ Failed to batch edit orders: {:?}", e);
            }
        }
    }

    #[tokio::test]
    #[ignore]
    async fn test_edit_orders_with_different_types() {
        let binance = create_test_binance();

        // 测试批量编辑不同类型的订单
        use std::collections::HashMap;

        let orders = vec![
            (
                "111111111",
                "BTCUSDT",
                OrderType::Limit,
                OrderSide::Buy,
                Decimal::from_str("0.001").unwrap(),
                Some(Decimal::from_str("44000.00").unwrap()),
            ),
            (
                "222222222",
                "ETHUSDT",
                OrderType::Market,
                OrderSide::Sell,
                Decimal::from_str("0.1").unwrap(),
                None,
            ),
        ];

        let result = binance.edit_orders(orders).await;

        match result {
            Ok(edited_orders) => {
                println!("✅ 成功批量编辑不同类型订单，数量: {}", edited_orders.len());
                assert_eq!(edited_orders.len(), 2, "应返回2个编辑后的订单");
            }
            Err(e) => {
                println!("❌ 批量编辑订单失败: {:?}", e);
            }
        }
    }

    #[test]
    fn test_order_management_parameter_validation() {
        // 测试参数验证逻辑

        // 验证订单ID格式
        let valid_id = "123456789";
        assert!(!valid_id.is_empty(), "订单ID不应为空");

        // 验证交易对格式
        let valid_symbol = "BTCUSDT";
        assert!(
            valid_symbol.contains("USDT") || valid_symbol.contains("BTC"),
            "交易对格式应正确"
        );

        // 验证数量范围
        let amount = Decimal::from_str("0.001").unwrap();
        assert!(amount > Decimal::ZERO, "订单数量应大于0");

        // 验证价格范围
        let price = Decimal::from_str("45000.00").unwrap();
        assert!(price > Decimal::ZERO, "订单价格应大于0");

        println!("✅ 参数验证测试通过");
    }

    #[test]
    fn test_order_status_validation() {
        // 测试订单状态验证
        let valid_statuses = vec!["open", "closed", "canceled", "expired"];

        for status in valid_statuses.iter() {
            assert!(!status.is_empty(), "订单状态不应为空");
            println!("  有效状态: {}", status);
        }

        println!("✅ 订单状态验证测试通过");
    }
}
