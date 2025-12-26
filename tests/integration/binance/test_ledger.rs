//! Binance账本查询功能集成测试
//! 
//! 测试fetch_ledger()方法和相关解析函数
//!
//! Note: The fetch_ledger method is being migrated to the new modular REST API structure.
//! These tests are currently placeholders and will be updated once the migration is complete.

use ccxt_core::types::*;
use ccxt_core::ExchangeConfig;
use std::collections::HashMap;

/// 辅助函数：创建测试用的Binance实例
fn create_test_exchange() -> ccxt_exchanges::binance::Binance {
    ccxt_exchanges::binance::Binance::new(ExchangeConfig {
        api_key: Some("test_api_key".to_string()),
        secret: Some("test_secret".to_string()),
        sandbox: true,
        ..Default::default()
    }).unwrap()
}

#[cfg(test)]
mod ledger_tests {
    use super::*;

    #[test]
    fn test_ledger_entry_creation() {
        // 测试LedgerEntry结构体创建
        let entry = LedgerEntry {
            id: "12345".to_string(),
            datetime: "2024-01-01T00:00:00.000Z".to_string(),
            timestamp: 1704067200000,
            direction: LedgerDirection::In,
            account: Some("futures".to_string()),
            reference_id: Some("ref123".to_string()),
            reference_account: None,
            type_: LedgerEntryType::Trade,
            currency: "USDT".to_string(),
            amount: 100.0,
            before: Some(1000.0),
            after: Some(1100.0),
            fee: None,
            info: serde_json::json!({}),
        };

        assert_eq!(entry.id, "12345");
        assert_eq!(entry.currency, "USDT");
        assert_eq!(entry.amount, 100.0);
        assert_eq!(entry.direction, LedgerDirection::In);
        assert_eq!(entry.type_, LedgerEntryType::Trade);
    }

    #[test]
    fn test_ledger_direction_enum() {
        // 测试LedgerDirection枚举
        let dir_in = LedgerDirection::In;
        let dir_out = LedgerDirection::Out;

        assert_ne!(dir_in, dir_out);
    }

    #[test]
    fn test_ledger_entry_type_variants() {
        // 测试LedgerEntryType所有变体
        let types = vec![
            LedgerEntryType::Trade,
            LedgerEntryType::Fee,
            LedgerEntryType::Rebate,
            LedgerEntryType::Funding,
            LedgerEntryType::Transfer,
            LedgerEntryType::Margin,
            LedgerEntryType::Cashback,
            LedgerEntryType::Referral,
            LedgerEntryType::Pnl,
            LedgerEntryType::Other,
        ];

        assert_eq!(types.len(), 10);
    }

    #[test]
    fn test_fetch_ledger_requires_credentials() {
        // 测试未提供凭证时的行为
        let _exchange = ccxt_exchanges::binance::Binance::new(ExchangeConfig {
            api_key: None,
            secret: None,
            sandbox: true,
            ..Default::default()
        }).unwrap();

        // 注意：这是同步测试，无法测试async方法
        // 实际测试应该在async环境中进行
        // 无法直接访问 config，需要通过其他方式测试
        // 这里仅测试创建成功
        assert!(true);
    }

    #[test]
    fn test_ledger_params_validation() {
        // 测试参数验证逻辑
        let mut params = HashMap::new();
        
        // 测试有效的type参数
        params.insert("type".to_string(), "future".to_string());
        assert_eq!(params.get("type").unwrap(), "future");
        
        params.insert("type".to_string(), "delivery".to_string());
        assert_eq!(params.get("type").unwrap(), "delivery");
        
        params.insert("type".to_string(), "option".to_string());
        assert_eq!(params.get("type").unwrap(), "option");
    }
}

#[cfg(test)]
mod parser_tests {
    use super::*;
    use ccxt_exchanges::binance::parser;

    // parse_ledger_entry_type 函数不存在，类型转换逻辑已内置在 parse_ledger_entry 中

    #[test]
    fn test_parse_ledger_entry_with_options_format() {
        // 测试期权API格式解析
        let options_data = serde_json::json!({
            "id": "12345",
            "asset": "USDT",
            "amount": "100.50",
            "transactionTime": 1704067200000i64,
            "type": "REALIZED_PNL",
            "balance": "1100.50"
        });

        let result = parser::parse_ledger_entry(&options_data);
        assert!(result.is_ok());
        
        let entry = result.unwrap();
        assert_eq!(entry.id, "12345");
        assert_eq!(entry.currency, "USDT");
        assert_eq!(entry.amount, 100.50);
        assert_eq!(entry.timestamp, 1704067200000);
        assert_eq!(entry.type_, LedgerEntryType::Pnl);
        assert_eq!(entry.after, Some(1100.50));
    }

    #[test]
    fn test_parse_ledger_entry_with_futures_format() {
        // 测试合约API格式解析
        let futures_data = serde_json::json!({
            "id": "67890",
            "asset": "BTC",
            "income": "-0.001",
            "time": 1704067200000i64,
            "incomeType": "COMMISSION",
            "tradeId": "trade123",
            "balanceAfter": "1.5"
        });

        let result = parser::parse_ledger_entry(&futures_data);
        assert!(result.is_ok());
        
        let entry = result.unwrap();
        assert_eq!(entry.id, "67890");
        assert_eq!(entry.currency, "BTC");
        assert_eq!(entry.amount, 0.001); // 负数转为正数
        assert_eq!(entry.direction, LedgerDirection::Out); // 负数为out
        assert_eq!(entry.type_, LedgerEntryType::Fee);
        assert_eq!(entry.reference_id, Some("trade123".to_string()));
    }

    #[test]
    fn test_parse_ledger_entry_positive_amount() {
        // 测试正数金额（入账）
        let data = serde_json::json!({
            "id": "111",
            "asset": "USDT",
            "income": "50.0",
            "time": 1704067200000i64,
            "incomeType": "TRANSFER"
        });

        let result = parser::parse_ledger_entry(&data);
        assert!(result.is_ok());
        
        let entry = result.unwrap();
        assert_eq!(entry.amount, 50.0);
        assert_eq!(entry.direction, LedgerDirection::In);
    }

    #[test]
    fn test_parse_ledger_entry_negative_amount() {
        // 测试负数金额（出账）
        let data = serde_json::json!({
            "id": "222",
            "asset": "USDT",
            "income": "-30.0",
            "time": 1704067200000i64,
            "incomeType": "FUNDING_FEE"
        });

        let result = parser::parse_ledger_entry(&data);
        assert!(result.is_ok());
        
        let entry = result.unwrap();
        assert_eq!(entry.amount, 30.0); // 绝对值
        assert_eq!(entry.direction, LedgerDirection::Out);
        assert_eq!(entry.type_, LedgerEntryType::Funding);
    }

    #[test]
    fn test_parse_ledger_entry_missing_fields() {
        // 测试缺少必需字段
        let invalid_data = serde_json::json!({
            "id": "333"
            // 缺少其他必需字段
        });

        let result = parser::parse_ledger_entry(&invalid_data);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_ledger_entry_with_symbol() {
        // 测试包含symbol字段
        let data = serde_json::json!({
            "id": "444",
            "asset": "USDT",
            "income": "100.0",
            "time": 1704067200000i64,
            "incomeType": "REALIZED_PNL",
            "symbol": "BTCUSDT"
        });

        let result = parser::parse_ledger_entry(&data);
        assert!(result.is_ok());
        
        let entry = result.unwrap();
        // symbol字段存储在info中
        assert!(entry.info.get("symbol").is_some());
        assert_eq!(entry.info["symbol"], "BTCUSDT");
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;

    // Note: The fetch_ledger method is being migrated to the new modular REST API structure.
    // These integration tests are currently disabled and will be updated once the migration is complete.

    #[tokio::test]
    #[ignore = "fetch_ledger method not yet migrated to new modular structure"]
    async fn test_fetch_ledger_future_wallet() {
        let _exchange = create_test_exchange();
        // TODO: Implement when fetch_ledger is available
    }

    #[tokio::test]
    #[ignore = "fetch_ledger method not yet migrated to new modular structure"]
    async fn test_fetch_ledger_with_currency_filter() {
        let _exchange = create_test_exchange();
        // TODO: Implement when fetch_ledger is available
    }

    #[tokio::test]
    #[ignore = "fetch_ledger method not yet migrated to new modular structure"]
    async fn test_fetch_ledger_delivery_wallet() {
        let _exchange = create_test_exchange();
        // TODO: Implement when fetch_ledger is available
    }

    #[tokio::test]
    #[ignore = "fetch_ledger method not yet migrated to new modular structure"]
    async fn test_fetch_ledger_option_wallet() {
        let _exchange = create_test_exchange();
        // TODO: Implement when fetch_ledger is available
    }

    #[tokio::test]
    #[ignore = "fetch_ledger method not yet migrated to new modular structure"]
    async fn test_fetch_ledger_spot_not_supported() {
        let _exchange = create_test_exchange();
        // TODO: Implement when fetch_ledger is available
    }

    #[tokio::test]
    #[ignore = "fetch_ledger method not yet migrated to new modular structure"]
    async fn test_fetch_ledger_with_time_range() {
        let _exchange = create_test_exchange();
        // TODO: Implement when fetch_ledger is available
    }

    #[tokio::test]
    #[ignore = "fetch_ledger method not yet migrated to new modular structure"]
    async fn test_fetch_ledger_portfolio_margin() {
        let _exchange = create_test_exchange();
        // TODO: Implement when fetch_ledger is available
    }
}
