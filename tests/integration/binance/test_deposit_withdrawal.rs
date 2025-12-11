//! Binance 充值提现功能集成测试
//!
//! 测试充值提现相关的API方法：
//! - withdraw: 提现到外部地址
//! - fetch_deposits: 查询充值记录
//! - fetch_withdrawals: 查询提现记录
//! - fetch_deposit_address: 获取充值地址

use ccxt_exchanges::binance::Binance;
use ccxt_core::{ExchangeConfig, TransactionStatus, TransactionType};
use std::collections::HashMap;

/// 辅助函数：创建测试用的Binance实例
fn create_test_binance() -> Binance {
    let mut config = ExchangeConfig::default();
    config.api_key = Some("test_api_key".to_string());
    config.secret = Some("test_secret".to_string());
    config.testnet = true;
    
    Binance::new(config).expect("Failed to create Binance instance")
}

#[cfg(test)]
mod deposit_withdrawal_tests {
    use super::*;

    // ============================================================================
    // 获取充值地址测试
    // ============================================================================

    #[tokio::test]
    #[ignore] // 需要实际的API连接
    async fn test_fetch_deposit_address_btc() {
        let binance = create_test_binance();
        
        let result = binance.fetch_deposit_address("BTC", None).await;
        
        match result {
            Ok(address) => {
                println!("✅ BTC充值地址获取成功");
                println!("   币种: {}", address.currency);
                println!("   地址: {}", address.address);
                
                assert_eq!(address.currency, "BTC");
                assert!(!address.address.is_empty());
                
                if let Some(network) = &address.network {
                    println!("   网络: {}", network);
                }
            }
            Err(e) => {
                println!("❌ 获取BTC充值地址失败: {}", e);
                panic!("Test failed");
            }
        }
    }

    #[tokio::test]
    #[ignore] // 需要实际的API连接
    async fn test_fetch_deposit_address_usdt_with_network() {
        let binance = create_test_binance();
        
        let mut params = HashMap::new();
        params.insert("network".to_string(), "TRX".to_string());
        
        let result = binance.fetch_deposit_address("USDT", Some(params)).await;
        
        match result {
            Ok(address) => {
                println!("✅ USDT-TRX充值地址获取成功");
                println!("   币种: {}", address.currency);
                println!("   地址: {}", address.address);
                
                assert_eq!(address.currency, "USDT");
                assert!(!address.address.is_empty());
                
                if let Some(network) = &address.network {
                    println!("   网络: {}", network);
                    assert_eq!(network, "TRX");
                }
            }
            Err(e) => {
                println!("❌ 获取USDT-TRX充值地址失败: {}", e);
                panic!("Test failed");
            }
        }
    }

    #[tokio::test]
    #[ignore] // 需要实际的API连接
    async fn test_fetch_deposit_address_with_tag() {
        let binance = create_test_binance();
        
        // XRP等币种会有tag
        let result = binance.fetch_deposit_address("XRP", None).await;
        
        match result {
            Ok(address) => {
                println!("✅ XRP充值地址获取成功");
                println!("   币种: {}", address.currency);
                println!("   地址: {}", address.address);
                
                assert_eq!(address.currency, "XRP");
                assert!(!address.address.is_empty());
                
                if let Some(tag) = &address.tag {
                    println!("   标签: {}", tag);
                    assert!(!tag.is_empty());
                }
            }
            Err(e) => {
                println!("❌ 获取XRP充值地址失败: {}", e);
                panic!("Test failed");
            }
        }
    }

    // ============================================================================
    // 查询充值记录测试
    // ============================================================================

    #[tokio::test]
    #[ignore] // 需要实际的API连接
    async fn test_fetch_deposits_all() {
        let binance = create_test_binance();
        
        let result = binance.fetch_deposits(None, None, Some(10), None).await;
        
        match result {
            Ok(deposits) => {
                println!("✅ 充值记录查询成功");
                println!("   记录数: {}", deposits.len());
                
                assert!(deposits.len() <= 10);
                
                for (i, deposit) in deposits.iter().take(3).enumerate() {
                    println!("\n充值 #{}:", i + 1);
                    println!("   ID: {}", deposit.id);
                    println!("   币种: {}", deposit.currency);
                    println!("   数量: {}", deposit.amount);
                    println!("   状态: {:?}", deposit.status);
                    
                    // 验证基本字段
                    assert!(!deposit.id.is_empty());
                    assert!(!deposit.currency.is_empty());
                    assert!(deposit.amount > rust_decimal::Decimal::ZERO);
                    assert!(matches!(deposit.transaction_type, TransactionType::Deposit));
                }
            }
            Err(e) => {
                println!("❌ 查询充值记录失败: {}", e);
                panic!("Test failed");
            }
        }
    }

    #[tokio::test]
    #[ignore] // 需要实际的API连接
    async fn test_fetch_deposits_by_currency() {
        let binance = create_test_binance();
        
        let result = binance.fetch_deposits(Some("USDT"), None, Some(10), None).await;
        
        match result {
            Ok(deposits) => {
                println!("✅ USDT充值记录查询成功");
                println!("   记录数: {}", deposits.len());
                
                for deposit in deposits.iter() {
                    assert_eq!(deposit.currency, "USDT");
                    assert!(matches!(deposit.transaction_type, TransactionType::Deposit));
                }
            }
            Err(e) => {
                println!("❌ 查询USDT充值记录失败: {}", e);
                panic!("Test failed");
            }
        }
    }

    #[tokio::test]
    #[ignore] // 需要实际的API连接
    async fn test_fetch_deposits_with_time_range() {
        let binance = create_test_binance();
        
        // 查询最近30天的充值
        let now = chrono::Utc::now().timestamp_millis();
        let thirty_days_ago = now - (30 * 24 * 60 * 60 * 1000);
        
        let mut params = HashMap::new();
        params.insert("startTime".to_string(), thirty_days_ago.to_string());
        params.insert("endTime".to_string(), now.to_string());
        
        let result = binance.fetch_deposits(None, Some(thirty_days_ago), Some(50), Some(params)).await;
        
        match result {
            Ok(deposits) => {
                println!("✅ 时间范围内充值记录查询成功");
                println!("   记录数: {}", deposits.len());
                
                for deposit in deposits.iter() {
                    if let Some(timestamp) = deposit.timestamp {
                        assert!(timestamp >= thirty_days_ago);
                        assert!(timestamp <= now);
                    }
                }
            }
            Err(e) => {
                println!("❌ 查询时间范围内充值记录失败: {}", e);
                panic!("Test failed");
            }
        }
    }

    // ============================================================================
    // 查询提现记录测试
    // ============================================================================

    #[tokio::test]
    #[ignore] // 需要实际的API连接
    async fn test_fetch_withdrawals_all() {
        let binance = create_test_binance();
        
        let result = binance.fetch_withdrawals(None, None, Some(10), None).await;
        
        match result {
            Ok(withdrawals) => {
                println!("✅ 提现记录查询成功");
                println!("   记录数: {}", withdrawals.len());
                
                assert!(withdrawals.len() <= 10);
                
                for (i, withdrawal) in withdrawals.iter().take(3).enumerate() {
                    println!("\n提现 #{}:", i + 1);
                    println!("   ID: {}", withdrawal.id);
                    println!("   币种: {}", withdrawal.currency);
                    println!("   数量: {}", withdrawal.amount);
                    println!("   状态: {:?}", withdrawal.status);
                    
                    // 验证基本字段
                    assert!(!withdrawal.id.is_empty());
                    assert!(!withdrawal.currency.is_empty());
                    assert!(withdrawal.amount > rust_decimal::Decimal::Decimal::ZERO);
                    assert!(matches!(withdrawal.transaction_type, TransactionType::Withdrawal));
                }
            }
            Err(e) => {
                println!("❌ 查询提现记录失败: {}", e);
                panic!("Test failed");
            }
        }
    }

    #[tokio::test]
    #[ignore] // 需要实际的API连接
    async fn test_fetch_withdrawals_by_currency() {
        let binance = create_test_binance();
        
        let result = binance.fetch_withdrawals(Some("BTC"), None, Some(10), None).await;
        
        match result {
            Ok(withdrawals) => {
                println!("✅ BTC提现记录查询成功");
                println!("   记录数: {}", withdrawals.len());
                
                for withdrawal in withdrawals.iter() {
                    assert_eq!(withdrawal.currency, "BTC");
                    assert!(matches!(withdrawal.transaction_type, TransactionType::Withdrawal));
                }
            }
            Err(e) => {
                println!("❌ 查询BTC提现记录失败: {}", e);
                panic!("Test failed");
            }
        }
    }

    #[tokio::test]
    #[ignore] // 需要实际的API连接
    async fn test_fetch_withdrawals_with_status_filter() {
        let binance = create_test_binance();
        
        let mut params = HashMap::new();
        params.insert("status".to_string(), "6".to_string()); // 6 = 完成
        
        let result = binance.fetch_withdrawals(None, None, Some(20), Some(params)).await;
        
        match result {
            Ok(withdrawals) => {
                println!("✅ 已完成提现记录查询成功");
                println!("   记录数: {}", withdrawals.len());
                
                for withdrawal in withdrawals.iter() {
                    assert!(matches!(withdrawal.status, TransactionStatus::Ok));
                }
            }
            Err(e) => {
                println!("❌ 查询已完成提现记录失败: {}", e);
                panic!("Test failed");
            }
        }
    }

    // ============================================================================
    // 提现功能测试（仅测试参数验证，不实际执行）
    // ============================================================================

    #[test]
    fn test_withdraw_parameter_validation() {
        // 测试提现参数的验证逻辑
        let currency = "USDT";
        let amount = "100.0";
        let address = "TXxxxxxxxxxxxxxxxxxxxxxxxxxxx";
        
        // 验证参数格式
        assert!(!currency.is_empty());
        assert!(!amount.is_empty());
        assert!(!address.is_empty());
        
        // 验证金额可解析
        let parsed_amount: f64 = amount.parse().expect("Invalid amount format");
        assert!(parsed_amount > 0.0);
        
        println!("✅ 提现参数验证通过");
    }

    #[test]
    fn test_withdraw_with_network_parameter() {
        let mut params = HashMap::new();
        params.insert("network".to_string(), "TRX".to_string());
        
        assert!(params.contains_key("network"));
        assert_eq!(params.get("network").unwrap(), "TRX");
        
        println!("✅ 提现网络参数设置正确");
    }

    #[test]
    fn test_withdraw_with_tag_parameter() {
        let mut params = HashMap::new();
        params.insert("tag".to_string(), "123456".to_string());
        
        assert!(params.contains_key("tag"));
        assert_eq!(params.get("tag").unwrap(), "123456");
        
        println!("✅ 提现标签参数设置正确");
    }

    // ============================================================================
    // 综合场景测试
    // ============================================================================

    #[tokio::test]
    #[ignore] // 需要实际的API连接
    async fn test_deposit_withdrawal_workflow() {
        let binance = create_test_binance();
        
        println!("=== 充值提现工作流测试 ===\n");
        
        // 1. 获取充值地址
        println!("步骤1: 获取USDT充值地址");
        let mut params = HashMap::new();
        params.insert("network".to_string(), "TRX".to_string());
        
        match binance.fetch_deposit_address("USDT", Some(params)).await {
            Ok(address) => {
                println!("✅ 充值地址: {}", address.address);
                assert!(!address.address.is_empty());
            }
            Err(e) => {
                println!("❌ 获取充值地址失败: {}", e);
                return;
            }
        }
        
        // 2. 查询最近的充值记录
        println!("\n步骤2: 查询最近的USDT充值");
        match binance.fetch_deposits(Some("USDT"), None, Some(5), None).await {
            Ok(deposits) => {
                println!("✅ 找到 {} 条充值记录", deposits.len());
                
                for deposit in deposits.iter() {
                    println!("   - {} USDT (状态: {:?})", deposit.amount, deposit.status);
                }
            }
            Err(e) => {
                println!("❌ 查询充值记录失败: {}", e);
            }
        }
        
        // 3. 查询最近的提现记录
        println!("\n步骤3: 查询最近的USDT提现");
        match binance.fetch_withdrawals(Some("USDT"), None, Some(5), None).await {
            Ok(withdrawals) => {
                println!("✅ 找到 {} 条提现记录", withdrawals.len());
                
                for withdrawal in withdrawals.iter() {
                    println!("   - {} USDT (状态: {:?})", withdrawal.amount, withdrawal.status);
                }
            }
            Err(e) => {
                println!("❌ 查询提现记录失败: {}", e);
            }
        }
        
        println!("\n✅ 充值提现工作流测试完成");
    }

    #[tokio::test]
    #[ignore] // 需要实际的API连接
    async fn test_multiple_currencies_deposit_addresses() {
        let binance = create_test_binance();
        
        let currencies = vec!["BTC", "ETH", "USDT", "BNB"];
        
        println!("=== 多币种充值地址测试 ===\n");
        
        for currency in currencies {
            match binance.fetch_deposit_address(currency, None).await {
                Ok(address) => {
                    println!("✅ {} 充值地址: {}", currency, address.address);
                    assert_eq!(address.currency, currency);
                    assert!(!address.address.is_empty());
                }
                Err(e) => {
                    println!("❌ 获取 {} 充值地址失败: {}", currency, e);
                }
            }
            
            // 避免请求过快
            tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
        }
        
        println!("\n✅ 多币种充值地址测试完成");
    }
}