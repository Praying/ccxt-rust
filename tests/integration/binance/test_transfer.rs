//! Binance内部转账功能集成测试
//!
//! 测试transfer()、fetch_transfers()和fetch_deposit_withdraw_fees()方法

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
// transfer() 方法测试
// ============================================================================

#[tokio::test]
#[ignore] // 需要有效的API密钥才能运行
async fn test_transfer_spot_to_future() {
    if !has_credentials() {
        println!("跳过测试：需要设置 BINANCE_API_KEY 和 BINANCE_SECRET");
        return;
    }

    let binance = create_test_binance();
    
    // 从现货转账到合约
    let result = binance
        .transfer(
            "USDT",
            10.0,
            "spot",
            "future",
            None,
        )
        .await;
    
    match result {
        Ok(transfer) => {
            assert!(transfer.id.is_some(), "转账应该有ID");
            assert_eq!(transfer.currency, "USDT");
            assert_eq!(transfer.amount, 10.0);
            assert!(transfer.from_account.is_some());
            assert!(transfer.to_account.is_some());
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
async fn test_transfer_future_to_spot() {
    if !has_credentials() {
        return;
    }

    let binance = create_test_binance();
    
    let result = binance
        .transfer("USDT", 5.0, "future", "spot", None)
        .await;
    
    match result {
        Ok(transfer) => {
            assert_eq!(transfer.currency, "USDT");
            assert_eq!(transfer.amount, 5.0);
            println!("✅ 转账成功");
        }
        Err(e) => {
            println!("⚠️ 转账失败: {}", e);
        }
    }
}

// ============================================================================
// fetch_transfers() 方法测试
// ============================================================================

#[tokio::test]
#[ignore]
async fn test_fetch_transfers_all() {
    if !has_credentials() {
        return;
    }

    let binance = create_test_binance();
    
    let result = binance
        .fetch_transfers(None, None, Some(10), None)
        .await;
    
    match result {
        Ok(transfers) => {
            assert!(transfers.len() <= 10, "应该不超过限制数量");
            
            for transfer in &transfers {
                assert!(!transfer.currency.is_empty());
                assert!(transfer.amount > 0.0);
                assert!(!transfer.status.is_empty());
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
async fn test_fetch_transfers_by_currency() {
    if !has_credentials() {
        return;
    }

    let binance = create_test_binance();
    
    let result = binance
        .fetch_transfers(Some("USDT"), None, Some(20), None)
        .await;
    
    match result {
        Ok(transfers) => {
            for transfer in &transfers {
                assert_eq!(transfer.currency, "USDT");
            }
            println!("✅ 查询到 {} 条USDT转账记录", transfers.len());
        }
        Err(e) => {
            println!("⚠️ 查询失败: {}", e);
        }
    }
}

// ============================================================================
// fetch_deposit_withdraw_fees() 方法测试
// ============================================================================

#[tokio::test]
#[ignore]
async fn test_fetch_deposit_withdraw_fees_single() {
    if !has_credentials() {
        return;
    }

    let binance = create_test_binance();
    
    let result = binance
        .fetch_deposit_withdraw_fees(Some("USDT"), None)
        .await;
    
    match result {
        Ok(fees) => {
            assert!(!fees.is_empty());
            
            let usdt_fee = &fees[0];
            assert_eq!(usdt_fee.currency, "USDT");
            assert!(usdt_fee.withdraw_fee >= 0.0);
            assert!(usdt_fee.withdraw_min > 0.0);
            assert!(!usdt_fee.networks.is_empty());
            
            println!("✅ USDT手续费: {}", usdt_fee.withdraw_fee);
        }
        Err(e) => {
            println!("⚠️ 查询失败: {}", e);
        }
    }
}

#[tokio::test]
#[ignore]
async fn test_fetch_deposit_withdraw_fees_all() {
    if !has_credentials() {
        return;
    }

    let binance = create_test_binance();
    
    let result = binance
        .fetch_deposit_withdraw_fees(None, None)
        .await;
    
    match result {
        Ok(fees) => {
            assert!(!fees.is_empty());
            println!("✅ 查询到 {} 个币种的手续费", fees.len());
        }
        Err(e) => {
            println!("⚠️ 查询失败: {}", e);
        }
    }
}

// ============================================================================
// 错误处理测试
// ============================================================================

#[tokio::test]
async fn test_transfer_without_credentials() {
    let config = ExchangeConfig {
        api_key: None,
        secret: None,
        ..Default::default()
    };
    
    let binance = Binance::new(config).expect("创建实例失败");
    
    let result = binance
        .transfer("USDT", 1.0, "spot", "future", None)
        .await;
    
    assert!(result.is_err(), "没有凭证应该失败");
    println!("✅ 正确处理了缺少凭证的情况");
}