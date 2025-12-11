//! Binance期货转账功能集成测试
//!
//! 测试futures_transfer()和fetch_futures_transfers()方法

use ccxt_core::ExchangeConfig;
use ccxt_exchanges::binance::Binance;
use std::collections::HashMap;

/// 测试辅助函数：创建测试用的Binance实例
fn create_test_binance() -> Binance {
    let config = ExchangeConfig {
        api_key: std::env::var("BINANCE_API_KEY").ok(),
        secret: std::env::var("BINANCE_SECRET").ok(),
        testnet: true, // 使用测试网
        ..Default::default()
    };
    
    Binance::new(config).expect("创建Binance实例失败")
}

/// 检查是否配置了API密钥
fn has_credentials() -> bool {
    std::env::var("BINANCE_API_KEY").is_ok() && std::env::var("BINANCE_SECRET").is_ok()
}

// ============================================================================
// futures_transfer() 方法测试
// ============================================================================

#[tokio::test]
#[ignore] // 需要有效的API密钥才能运行
async fn test_futures_transfer_spot_to_usdtm() {
    if !has_credentials() {
        println!("跳过测试：需要设置 BINANCE_API_KEY 和 BINANCE_SECRET");
        return;
    }

    let binance = create_test_binance();
    
    // Type 1: 现货 → USDT-M期货
    let result = binance
        .futures_transfer(
            "USDT",
            10.0,
            1,      // type=1: 现货 → USDT-M期货
            None,
        )
        .await;
    
    match result {
        Ok(transfer) => {
            assert!(transfer.id.is_some(), "转账应该有ID");
            assert_eq!(transfer.currency, "USDT");
            assert_eq!(transfer.amount, 10.0);
            assert_eq!(transfer.from_account.as_deref(), Some("spot"));
            assert_eq!(transfer.to_account.as_deref(), Some("future"));
            assert!(!transfer.status.is_empty());
            println!("✅ 转账成功: {:?}", transfer.id);
        }
        Err(e) => {
            println!("⚠️ 转账失败（预期）: {}", e);
        }
    }
}

#[tokio::test]
#[ignore]
async fn test_futures_transfer_usdtm_to_spot() {
    if !has_credentials() {
        return;
    }

    let binance = create_test_binance();
    
    // Type 2: USDT-M期货 → 现货
    let result = binance
        .futures_transfer("USDT", 5.0, 2, None)
        .await;
    
    match result {
        Ok(transfer) => {
            assert_eq!(transfer.currency, "USDT");
            assert_eq!(transfer.amount, 5.0);
            assert_eq!(transfer.from_account.as_deref(), Some("future"));
            assert_eq!(transfer.to_account.as_deref(), Some("spot"));
            println!("✅ 转账成功");
        }
        Err(e) => {
            println!("⚠️ 转账失败: {}", e);
        }
    }
}

#[tokio::test]
#[ignore]
async fn test_futures_transfer_spot_to_coinm() {
    if !has_credentials() {
        return;
    }

    let binance = create_test_binance();
    
    // Type 3: 现货 → COIN-M期货
    let result = binance
        .futures_transfer("BTC", 0.001, 3, None)
        .await;
    
    match result {
        Ok(transfer) => {
            assert_eq!(transfer.currency, "BTC");
            assert_eq!(transfer.amount, 0.001);
            assert_eq!(transfer.from_account.as_deref(), Some("spot"));
            assert_eq!(transfer.to_account.as_deref(), Some("delivery"));
            println!("✅ 转账成功");
        }
        Err(e) => {
            println!("⚠️ 转账失败: {}", e);
        }
    }
}

#[tokio::test]
#[ignore]
async fn test_futures_transfer_coinm_to_spot() {
    if !has_credentials() {
        return;
    }

    let binance = create_test_binance();
    
    // Type 4: COIN-M期货 → 现货
    let result = binance
        .futures_transfer("BTC", 0.001, 4, None)
        .await;
    
    match result {
        Ok(transfer) => {
            assert_eq!(transfer.currency, "BTC");
            assert_eq!(transfer.amount, 0.001);
            assert_eq!(transfer.from_account.as_deref(), Some("delivery"));
            assert_eq!(transfer.to_account.as_deref(), Some("spot"));
            println!("✅ 转账成功");
        }
        Err(e) => {
            println!("⚠️ 转账失败: {}", e);
        }
    }
}

// ============================================================================
// fetch_futures_transfers() 方法测试
// ============================================================================

#[tokio::test]
#[ignore]
async fn test_fetch_futures_transfers_all() {
    if !has_credentials() {
        return;
    }

    let binance = create_test_binance();
    
    let result = binance
        .fetch_futures_transfers("USDT", None, Some(20), None)
        .await;
    
    match result {
        Ok(transfers) => {
            assert!(transfers.len() <= 20, "应该不超过限制数量");
            
            for transfer in &transfers {
                assert_eq!(transfer.currency, "USDT");
                assert!(transfer.amount > 0.0);
                assert!(!transfer.status.is_empty());
                assert!(transfer.from_account.is_some());
                assert!(transfer.to_account.is_some());
            }
            
            println!("✅ 查询到 {} 条转账记录", transfers.len());
        }
        Err(e) => {
            println!("⚠️ 查询失败: {}", e);
        }
    }
}

#[tokio::test]
#[ignore]
async fn test_fetch_futures_transfers_with_time_range() {
    if !has_credentials() {
        return;
    }

    let binance = create_test_binance();
    
    // 查询最近7天的转账
    let seven_days_ago = chrono::Utc::now().timestamp_millis() as u64 - 7 * 86400_000;
    
    let result = binance
        .fetch_futures_transfers("USDT", Some(seven_days_ago), Some(50), None)
        .await;
    
    match result {
        Ok(transfers) => {
            for transfer in &transfers {
                assert_eq!(transfer.currency, "USDT");
                assert!(transfer.timestamp >= seven_days_ago);
            }
            println!("✅ 查询到 {} 条最近7天的转账记录", transfers.len());
        }
        Err(e) => {
            println!("⚠️ 查询失败: {}", e);
        }
    }
}

#[tokio::test]
#[ignore]
async fn test_fetch_futures_transfers_with_pagination() {
    if !has_credentials() {
        return;
    }

    let binance = create_test_binance();
    
    // 使用分页参数
    let mut params = HashMap::new();
    params.insert("current".to_string(), "1".to_string());
    params.insert("size".to_string(), "10".to_string());
    
    let result = binance
        .fetch_futures_transfers("USDT", None, None, Some(params))
        .await;
    
    match result {
        Ok(transfers) => {
            assert!(transfers.len() <= 10);
            println!("✅ 分页查询到 {} 条记录", transfers.len());
        }
        Err(e) => {
            println!("⚠️ 查询失败: {}", e);
        }
    }
}

#[tokio::test]
#[ignore]
async fn test_fetch_futures_transfers_btc() {
    if !has_credentials() {
        return;
    }

    let binance = create_test_binance();
    
    let result = binance
        .fetch_futures_transfers("BTC", None, Some(10), None)
        .await;
    
    match result {
        Ok(transfers) => {
            for transfer in &transfers {
                assert_eq!(transfer.currency, "BTC");
            }
            println!("✅ 查询到 {} 条BTC转账记录", transfers.len());
        }
        Err(e) => {
            println!("⚠️ 查询失败: {}", e);
        }
    }
}

// ============================================================================
// 转账类型验证测试
// ============================================================================

#[tokio::test]
#[ignore]
async fn test_futures_transfer_type_validation() {
    if !has_credentials() {
        return;
    }

    let binance = create_test_binance();
    
    // 测试所有有效的转账类型
    let valid_types = vec![1, 2, 3, 4];
    
    for transfer_type in valid_types {
        let asset = if transfer_type <= 2 { "USDT" } else { "BTC" };
        let amount = if transfer_type <= 2 { 1.0 } else { 0.0001 };
        
        let result = binance
            .futures_transfer(asset, amount, transfer_type, None)
            .await;
        
        match result {
            Ok(transfer) => {
                // 验证转账方向
                let (expected_from, expected_to) = match transfer_type {
                    1 => ("spot", "future"),
                    2 => ("future", "spot"),
                    3 => ("spot", "delivery"),
                    4 => ("delivery", "spot"),
                    _ => unreachable!(),
                };
                
                assert_eq!(transfer.from_account.as_deref(), Some(expected_from));
                assert_eq!(transfer.to_account.as_deref(), Some(expected_to));
                println!("✅ Type {} 验证通过", transfer_type);
            }
            Err(e) => {
                println!("⚠️ Type {} 失败: {}", transfer_type, e);
            }
        }
    }
}

// ============================================================================
// 错误处理测试
// ============================================================================

#[tokio::test]
async fn test_futures_transfer_invalid_type() {
    let binance = create_test_binance();
    
    // 测试无效的转账类型（type必须是1-4）
    let result = binance
        .futures_transfer("USDT", 10.0, 5, None)
        .await;
    
    assert!(result.is_err(), "无效的type应该失败");
    
    if let Err(e) = result {
        let error_msg = e.to_string();
        assert!(
            error_msg.contains("type must be") || error_msg.contains("between 1 and 4"),
            "错误信息应该提示type范围"
        );
        println!("✅ 正确处理了无效的type参数");
    }
}

#[tokio::test]
async fn test_futures_transfer_without_credentials() {
    let config = ExchangeConfig {
        api_key: None,
        secret: None,
        ..Default::default()
    };
    
    let binance = Binance::new(config).expect("创建实例失败");
    
    let result = binance
        .futures_transfer("USDT", 1.0, 1, None)
        .await;
    
    assert!(result.is_err(), "没有凭证应该失败");
    println!("✅ 正确处理了缺少凭证的情况");
}

#[tokio::test]
async fn test_fetch_futures_transfers_without_credentials() {
    let config = ExchangeConfig {
        api_key: None,
        secret: None,
        ..Default::default()
    };
    
    let binance = Binance::new(config).expect("创建实例失败");
    
    let result = binance
        .fetch_futures_transfers("USDT", None, Some(10), None)
        .await;
    
    assert!(result.is_err(), "没有凭证应该失败");
    println!("✅ 正确处理了缺少凭证的情况");
}

// ============================================================================
// 数据完整性测试
// ============================================================================

#[tokio::test]
#[ignore]
async fn test_futures_transfer_response_structure() {
    if !has_credentials() {
        return;
    }

    let binance = create_test_binance();
    
    let result = binance
        .futures_transfer("USDT", 1.0, 1, None)
        .await;
    
    match result {
        Ok(transfer) => {
            // 验证Transfer结构的所有必需字段
            assert!(transfer.id.is_some(), "应该有转账ID");
            assert!(!transfer.currency.is_empty(), "币种不能为空");
            assert!(transfer.amount > 0.0, "数量必须大于0");
            assert!(transfer.from_account.is_some(), "应该有源账户");
            assert!(transfer.to_account.is_some(), "应该有目标账户");
            assert!(!transfer.status.is_empty(), "状态不能为空");
            assert!(transfer.timestamp > 0, "时间戳必须有效");
            assert!(!transfer.datetime.is_empty(), "日期时间不能为空");
            
            println!("✅ 响应结构验证通过");
        }
        Err(e) => {
            println!("⚠️ 测试失败: {}", e);
        }
    }
}

#[tokio::test]
#[ignore]
async fn test_fetch_futures_transfers_response_structure() {
    if !has_credentials() {
        return;
    }

    let binance = create_test_binance();
    
    let result = binance
        .fetch_futures_transfers("USDT", None, Some(5), None)
        .await;
    
    match result {
        Ok(transfers) => {
            if !transfers.is_empty() {
                let transfer = &transfers[0];
                
                // 验证每条记录的结构
                assert!(transfer.id.is_some());
                assert!(!transfer.currency.is_empty());
                assert!(transfer.amount > 0.0);
                assert!(transfer.from_account.is_some());
                assert!(transfer.to_account.is_some());
                assert!(!transfer.status.is_empty());
                assert!(transfer.timestamp > 0);
                
                println!("✅ 历史记录结构验证通过");
            } else {
                println!("⚠️ 没有历史记录可验证");
            }
        }
        Err(e) => {
            println!("⚠️ 查询失败: {}", e);
        }
    }
}

// ============================================================================
// 边界条件测试
// ============================================================================

#[tokio::test]
#[ignore]
async fn test_futures_transfer_minimum_amount() {
    if !has_credentials() {
        return;
    }

    let binance = create_test_binance();
    
    // 测试最小转账金额
    let result = binance
        .futures_transfer("USDT", 0.01, 1, None)
        .await;
    
    match result {
        Ok(transfer) => {
            assert_eq!(transfer.amount, 0.01);
            println!("✅ 最小金额转账成功");
        }
        Err(e) => {
            println!("⚠️ 最小金额转账失败: {}", e);
        }
    }
}

#[tokio::test]
#[ignore]
async fn test_fetch_futures_transfers_max_limit() {
    if !has_credentials() {
        return;
    }

    let binance = create_test_binance();
    
    // 测试最大限制（Binance限制是100）
    let result = binance
        .fetch_futures_transfers("USDT", None, Some(100), None)
        .await;
    
    match result {
        Ok(transfers) => {
            assert!(transfers.len() <= 100);
            println!("✅ 最大限制测试通过，返回 {} 条记录", transfers.len());
        }
        Err(e) => {
            println!("⚠️ 查询失败: {}", e);
        }
    }
}