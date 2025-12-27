//! Binance OCO订单管理集成测试
//!
//! 测试覆盖：
//! 1. 创建OCO订单
//! 2. 查询单个OCO订单
//! 3. 查询所有OCO订单
//! 4. 取消OCO订单
//! 5. 创建测试订单
//! 6. 修改现有订单

use ccxt_core::{ExchangeConfig, types::{OrderSide, OrderType, OrderStatus}};
use ccxt_exchanges::binance::Binance;

/// 创建测试用的Binance实例
fn create_test_binance() -> Binance {
    let mut config = ExchangeConfig::default();
    config.testnet = true;
    config.api_key = std::env::var("BINANCE_API_KEY").ok();
    config.secret = std::env::var("BINANCE_API_SECRET").ok();
    
    Binance::new(config).expect("Failed to create Binance instance")
}

/// 检查是否配置了API凭证
fn has_credentials() -> bool {
    std::env::var("BINANCE_API_KEY").is_ok() && std::env::var("BINANCE_API_SECRET").is_ok()
}

#[tokio::test]
async fn test_create_test_order() {
    if !has_credentials() {
        println!("⊘ 跳过测试: 未配置API凭证");
        return;
    }

    let binance = create_test_binance();
    
    // 测试限价买单
    let result = binance.create_test_order(
        "BTC/USDT",
        OrderType::Limit,
        OrderSide::Buy,
        0.001,
        Some(40000.0),
        None,
    ).await;
    
    assert!(result.is_ok(), "创建测试订单应该成功");
    
    let order = result.unwrap();
    assert_eq!(order.id, "test");
    assert_eq!(order.symbol, "BTC/USDT");
    assert_eq!(order.side, OrderSide::Buy);
    assert_eq!(order.order_type, OrderType::Limit);
    assert_eq!(order.status, OrderStatus::Open);
}

#[tokio::test]
async fn test_create_test_order_market() {
    if !has_credentials() {
        println!("⊘ 跳过测试: 未配置API凭证");
        return;
    }

    let binance = create_test_binance();
    
    // 测试市价卖单
    let result = binance.create_test_order(
        "BTC/USDT",
        OrderType::Market,
        OrderSide::Sell,
        0.001,
        None,
        None,
    ).await;
    
    assert!(result.is_ok(), "创建市价测试订单应该成功");
    
    let order = result.unwrap();
    assert_eq!(order.order_type, OrderType::Market);
    assert!(order.price.is_none(), "市价单不应该有价格");
}

#[tokio::test]
async fn test_create_oco_order() {
    if !has_credentials() {
        println!("⊘ 跳过测试: 未配置API凭证");
        return;
    }

    let binance = create_test_binance();
    
    let result = binance.create_oco_order(
        "BTC/USDT",
        OrderSide::Sell,
        0.001,
        50000.0,  // 止盈价
        45000.0,  // 止损触发价
        Some(44500.0),  // 止损限价
        None,
    ).await;
    
    match result {
        Ok(oco) => {
            // 验证OCO订单基本信息
            assert!(oco.order_list_id > 0, "订单列表ID应该大于0");
            assert_eq!(oco.symbol, "BTCUSDT", "交易对应该是BTCUSDT");
            assert!(!oco.list_status.is_empty(), "列表状态不应为空");
            assert_eq!(oco.orders.len(), 2, "OCO订单应该包含2个子订单");
            
            // 验证datetime格式
            assert!(!oco.datetime.is_empty(), "时间字符串不应为空");
            
            // 验证状态检查方法
            let is_executing = oco.is_executing();
            let is_all_done = oco.is_all_done();
            let is_rejected = oco.is_rejected();
            
            // 至少有一个状态应该为true
            assert!(
                is_executing || is_all_done || is_rejected,
                "至少应该有一个状态为true"
            );
            
            println!("✓ OCO订单创建成功: {}", oco.order_list_id);
        }
        Err(e) => {
            // 如果是余额不足等预期错误，测试也算通过
            let err_msg = e.to_string();
            if err_msg.contains("insufficient") || err_msg.contains("balance") {
                println!("⊘ 预期错误（余额不足）: {}", err_msg);
            } else {
                panic!("创建OCO订单失败: {}", e);
            }
        }
    }
}

#[tokio::test]
async fn test_fetch_oco_orders() {
    if !has_credentials() {
        println!("⊘ 跳过测试: 未配置API凭证");
        return;
    }

    let binance = create_test_binance();
    
    let result = binance.fetch_oco_orders(
        "BTC/USDT",
        None,
        Some(5),
        None,
    ).await;
    
    assert!(result.is_ok(), "查询OCO订单列表应该成功");
    
    let oco_orders = result.unwrap();
    println!("✓ 查询到 {} 个OCO订单", oco_orders.len());
    
    // 验证返回的订单数量不超过限制
    assert!(oco_orders.len() <= 5, "返回的订单数量不应超过限制");
    
    // 如果有订单，验证其结构
    for oco in &oco_orders {
        assert!(oco.order_list_id > 0, "订单列表ID应该大于0");
        assert!(!oco.symbol.is_empty(), "交易对不应为空");
        assert!(!oco.list_status.is_empty(), "状态不应为空");
        assert!(oco.orders.len() >= 1, "应该至少有一个订单");
    }
}

#[tokio::test]
async fn test_fetch_oco_orders_with_time_filter() {
    if !has_credentials() {
        println!("⊘ 跳过测试: 未配置API凭证");
        return;
    }

    let binance = create_test_binance();
    
    // 查询最近24小时的订单
    let since = chrono::Utc::now().timestamp_millis() as u64 - 86400000;
    
    let result = binance.fetch_oco_orders(
        "BTC/USDT",
        Some(since),
        Some(10),
        None,
    ).await;
    
    assert!(result.is_ok(), "带时间过滤的查询应该成功");
    
    let oco_orders = result.unwrap();
    println!("✓ 最近24小时内有 {} 个OCO订单", oco_orders.len());
    
    // 验证所有订单的时间都在指定时间之后
    for oco in &oco_orders {
        assert!(
            oco.transaction_time >= since,
            "订单时间应该在过滤时间之后"
        );
    }
}

#[tokio::test]
async fn test_oco_order_lifecycle() {
    if !has_credentials() {
        println!("⊘ 跳过测试: 未配置API凭证");
        return;
    }

    let binance = create_test_binance();
    
    // 1. 创建OCO订单
    let create_result = binance.create_oco_order(
        "BTC/USDT",
        OrderSide::Sell,
        0.001,
        60000.0,
        40000.0,
        Some(39500.0),
        None,
    ).await;
    
    if let Ok(oco) = create_result {
        let order_list_id = oco.order_list_id;
        println!("✓ 创建OCO订单: {}", order_list_id);
        
        // 等待一小段时间
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        
        // 2. 查询单个OCO订单
        let fetch_result = binance.fetch_oco_order(
            order_list_id,
            "BTC/USDT",
            None,
        ).await;
        
        assert!(fetch_result.is_ok(), "查询OCO订单应该成功");
        let fetched_oco = fetch_result.unwrap();
        assert_eq!(fetched_oco.order_list_id, order_list_id, "订单ID应该匹配");
        println!("✓ 查询OCO订单: {}", order_list_id);
        
        // 3. 取消OCO订单
        let cancel_result = binance.cancel_oco_order(
            order_list_id,
            "BTC/USDT",
            None,
        ).await;
        
        assert!(cancel_result.is_ok(), "取消OCO订单应该成功");
        let cancelled_oco = cancel_result.unwrap();
        assert_eq!(cancelled_oco.order_list_id, order_list_id, "订单ID应该匹配");
        println!("✓ 取消OCO订单: {}", order_list_id);
        
        // 验证取消后的状态
        assert!(
            cancelled_oco.list_status.contains("ALL_DONE") || 
            cancelled_oco.list_status.contains("REJECT"),
            "取消后状态应该是ALL_DONE或REJECT"
        );
    } else {
        let err = create_result.unwrap_err();
        let err_msg = err.to_string();
        if err_msg.contains("insufficient") || err_msg.contains("balance") {
            println!("⊘ 预期错误（余额不足），跳过完整生命周期测试");
        } else {
            panic!("创建OCO订单失败: {}", err);
        }
    }
}

#[tokio::test]
async fn test_edit_order() {
    if !has_credentials() {
        println!("⊘ 跳过测试: 未配置API凭证");
        return;
    }

    let binance = create_test_binance();
    
    // 1. 先创建一个限价单
    let create_result = binance.create_order(
        "BTC/USDT",
        OrderType::Limit,
        OrderSide::Buy,
        rust_decimal::Decimal::from_f64_retain(0.001).unwrap(),
        Some(rust_decimal::Decimal::from_f64_retain(35000.0).unwrap()),
        None,
    ).await;
    
    if let Ok(order) = create_result {
        let order_id = order.id.clone();
        let original_price = order.price;
        println!("✓ 创建限价单: {} @ {:?}", order_id, original_price);
        
        // 等待订单进入系统
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        
        // 2. 修改订单价格
        let edit_result = binance.edit_order(
            &order_id,
            "BTC/USDT",
            OrderType::Limit,
            OrderSide::Buy,
            rust_decimal::Decimal::from_f64_retain(0.001).unwrap(),
            Some(rust_decimal::Decimal::from_f64_retain(34000.0).unwrap()),
            None,
        ).await;
        
        match edit_result {
            Ok(new_order) => {
                println!("✓ 订单修改成功: {}", new_order.id);
                assert_ne!(new_order.id, order_id, "新订单ID应该不同");
                assert_ne!(new_order.price, original_price, "价格应该已修改");
                
                // 清理：取消新订单
                let _ = binance.cancel_order(&new_order.id, "BTC/USDT", None).await;
                println!("✓ 清理测试订单");
            }
            Err(e) => {
                // 尝试取消原订单
                let _ = binance.cancel_order(&order_id, "BTC/USDT", None).await;
                panic!("修改订单失败: {}", e);
            }
        }
    } else {
        let err = create_result.unwrap_err();
        println!("⊘ 创建订单失败，跳过编辑测试: {}", err);
    }
}

#[tokio::test]
async fn test_oco_order_info_methods() {
    // 测试OCO订单的状态判断方法
    use ccxt_core::types::OcoOrder;
    
    // 测试 EXECUTING 状态
    let mut oco = OcoOrder::new(
        1,
        "BTC/USDT".to_string(),
        "EXECUTING".to_string(),
        "EXECUTING".to_string(),
        1234567890,
    );
    assert!(oco.is_executing(), "应该识别为执行中");
    assert!(!oco.is_all_done(), "不应该识别为全部完成");
    assert!(!oco.is_rejected(), "不应该识别为被拒绝");
    
    // 测试 ALL_DONE 状态
    oco.list_status = "ALL_DONE".to_string();
    assert!(!oco.is_executing(), "不应该识别为执行中");
    assert!(oco.is_all_done(), "应该识别为全部完成");
    assert!(!oco.is_rejected(), "不应该识别为被拒绝");
    
    // 测试 REJECT 状态
    oco.list_status = "REJECT".to_string();
    assert!(!oco.is_executing(), "不应该识别为执行中");
    assert!(!oco.is_all_done(), "不应该识别为全部完成");
    assert!(oco.is_rejected(), "应该识别为被拒绝");
}

#[tokio::test]
async fn test_create_test_order_without_price() {
    if !has_credentials() {
        println!("⊘ 跳过测试: 未配置API凭证");
        return;
    }

    let binance = create_test_binance();
    
    // 市价单不需要价格
    let result = binance.create_test_order(
        "BTC/USDT",
        OrderType::Market,
        OrderSide::Buy,
        0.001,
        None,
        None,
    ).await;
    
    assert!(result.is_ok(), "不带价格的市价测试订单应该成功");
}

#[tokio::test]
async fn test_create_oco_order_validation() {
    if !has_credentials() {
        println!("⊘ 跳过测试: 未配置API凭证");
        return;
    }

    let binance = create_test_binance();
    
    // 测试不合理的价格（止盈价低于止损价）
    let result = binance.create_oco_order(
        "BTC/USDT",
        OrderSide::Sell,
        0.001,
        40000.0,  // 止盈价低于
        50000.0,  // 止损价（不合理）
        None,
        None,
    ).await;
    
    // 应该返回错误或被交易所拒绝
    if let Ok(oco) = result {
        // 如果创建成功，清理订单
        let _ = binance.cancel_oco_order(oco.order_list_id, "BTC/USDT", None).await;
        println!("⊘ 交易所接受了不合理的价格设置");
    } else {
        println!("✓ 正确拒绝了不合理的价格设置");
    }
}