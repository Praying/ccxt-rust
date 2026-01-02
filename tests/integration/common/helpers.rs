//! 集成测试辅助函数
//!
//! 提供测试初始化、交易所实例创建、数据生成等辅助工具

// Allow clippy warnings for test helper code
#![allow(clippy::disallowed_methods)]
#![allow(dead_code)]

use anyhow::{Context, Result as AnyhowResult};
use ccxt_core::{
    ExchangeConfig,
    error::{Error as CcxtError, NetworkError},
    test_config::TestConfig,
};
use ccxt_exchanges::binance::Binance;
use std::time::Duration;

/// 初始化测试配置
///
/// 从环境变量加载测试配置，如果失败则使用默认配置
pub fn init_test_config() -> TestConfig {
    // 尝试加载环境变量，失败则使用默认配置
    TestConfig::from_env().unwrap_or_else(|e| {
        eprintln!("⚠️  警告: 无法加载测试配置: {}，使用默认配置", e);
        TestConfig::default()
    })
}

/// 检查测试是否应该跳过（基于环境配置）
pub fn should_skip_integration_tests(config: &TestConfig) -> bool {
    !config.enable_integration_tests
}

/// 创建Binance实例（用于公开API测试）
///
/// 不需要API凭据
pub fn create_binance(config: &TestConfig) -> AnyhowResult<Binance> {
    let exchange_config = ExchangeConfig {
        id: "binance".to_string(),
        name: "Binance".to_string(),
        sandbox: config.binance.use_testnet,
        api_key: None,
        secret: None,
        ..Default::default()
    };

    Binance::new(exchange_config).context("Failed to create Binance instance")
}

/// 创建带凭据的Binance实例（用于私有API测试）
///
/// 需要在环境变量中配置API_KEY和API_SECRET
pub fn create_binance_with_credentials(config: &TestConfig) -> AnyhowResult<Binance> {
    let (api_key, api_secret) = config
        .get_active_api_key("binance")
        .context("No Binance credentials configured")?;

    let exchange_config = ExchangeConfig {
        id: "binance".to_string(),
        name: "Binance".to_string(),
        sandbox: config.binance.use_testnet,
        api_key: Some(ccxt_core::SecretString::new(api_key)),
        secret: Some(ccxt_core::SecretString::new(api_secret)),
        ..Default::default()
    };

    Binance::new(exchange_config).context("Failed to create Binance instance with credentials")
}

/// 获取单个测试交易对
pub fn get_test_symbol() -> &'static str {
    "BTC/USDT"
}

/// 生成测试用的交易对列表
pub fn get_test_symbols() -> Vec<&'static str> {
    vec!["BTC/USDT", "ETH/USDT", "BNB/USDT"]
}

/// 重试机制：如果API调用失败，自动重试
///
/// 使用指数退避策略，每次重试延迟时间翻倍
pub async fn retry_with_backoff<F, Fut, T>(
    mut operation: F,
    max_retries: u32,
    initial_delay_ms: u64,
) -> Result<T, CcxtError>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T, CcxtError>>,
{
    let mut delay = initial_delay_ms;

    for attempt in 0..max_retries {
        match operation().await {
            Ok(result) => return Ok(result),
            Err(e) if attempt < max_retries - 1 => {
                println!(
                    "⚠️  尝试 {}/{} 失败: {:?}, {}ms后重试...",
                    attempt + 1,
                    max_retries,
                    e,
                    delay
                );
                tokio::time::sleep(Duration::from_millis(delay)).await;
                delay *= 2; // 指数退避
            }
            Err(e) => return Err(e),
        }
    }

    unreachable!("循环应该在max_retries内返回")
}

/// 打印测试结果摘要
///
/// 格式化输出测试名称、通过状态和执行时长
pub fn print_test_summary(test_name: &str, passed: bool, duration_ms: u64) {
    let status = if passed { "✅ PASSED" } else { "❌ FAILED" };

    println!("\n{} {} ({}ms)\n", status, test_name, duration_ms);
}

/// 打印测试分隔线
pub fn print_test_separator(test_name: &str) {
    println!("\n{}", "=".repeat(60));
    println!("  🧪 Running: {}", test_name);
    println!("{}\n", "=".repeat(60));
}

/// 打印测试子步骤
pub fn print_test_step(step: &str) {
    println!("  → {}", step);
}

/// 等待速率限制冷却
pub async fn wait_rate_limit(milliseconds: u64) {
    tokio::time::sleep(Duration::from_millis(milliseconds)).await;
}

/// 确保交易所的市场数据已加载
///
/// 这个函数会检查市场是否已加载，如果没有则自动加载
pub async fn ensure_markets_loaded(
    exchange: &ccxt_exchanges::binance::Binance,
) -> AnyhowResult<()> {
    // 尝试获取市场列表，如果失败则加载
    exchange
        .fetch_markets()
        .await
        .context("Failed to load markets")?;
    Ok(())
}

/// 格式化价格为字符串（用于输出）
pub fn format_price<T: Into<rust_decimal::Decimal>>(price: T) -> String {
    use rust_decimal::prelude::*;
    let price_dec = price.into();
    // Use unwrap_or for safe conversion - this is acceptable in test helpers
    // as it's only used for display purposes
    let price_f64 = price_dec.to_f64().unwrap_or(0.0);

    if price_f64 >= 1.0 {
        format!("{:.2}", price_f64)
    } else if price_f64 >= 0.01 {
        format!("{:.4}", price_f64)
    } else {
        format!("{:.8}", price_f64)
    }
}

/// 格式化Option<价格类型>为字符串
///
/// 支持任何可以转换为 Decimal 的类型，包括 Price、Amount、Cost 和 Decimal 本身
pub fn format_price_opt<T: Into<rust_decimal::Decimal>>(price: Option<T>) -> String {
    match price {
        Some(p) => format_price(p),
        None => "N/A".to_string(),
    }
}

/// 格式化成交量
pub fn format_volume<T: Into<rust_decimal::Decimal>>(volume: T) -> String {
    use rust_decimal::prelude::*;
    let volume_dec = volume.into();
    // Use unwrap_or for safe conversion - this is acceptable in test helpers
    // as it's only used for display purposes
    let volume_f64 = volume_dec.to_f64().unwrap_or(0.0);

    if volume_f64 >= 1_000_000.0 {
        format!("{:.2}M", volume_f64 / 1_000_000.0)
    } else if volume_f64 >= 1_000.0 {
        format!("{:.2}K", volume_f64 / 1_000.0)
    } else {
        format!("{:.4}", volume_f64)
    }
}

/// 格式化Option<Decimal>成交量为字符串
pub fn format_volume_opt(volume: Option<rust_decimal::Decimal>) -> String {
    match volume {
        Some(v) => format_volume(v),
        None => "N/A".to_string(),
    }
}

/// 计算价格变化百分比
pub fn calculate_price_change_percent(old_price: f64, new_price: f64) -> f64 {
    if old_price == 0.0 {
        return 0.0;
    }
    ((new_price - old_price) / old_price) * 100.0
}

/// 验证交易对格式是否正确
pub fn is_valid_symbol_format(symbol: &str) -> bool {
    symbol.contains('/') && symbol.split('/').count() == 2
}

/// 从交易对中提取基础货币和报价货币
pub fn parse_symbol(symbol: &str) -> Option<(String, String)> {
    let parts: Vec<&str> = symbol.split('/').collect();
    if parts.len() == 2 {
        Some((parts[0].to_string(), parts[1].to_string()))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_test_symbols() {
        let symbols = get_test_symbols();
        assert!(!symbols.is_empty());
        assert!(symbols.contains(&"BTC/USDT"));
    }

    #[test]
    fn test_get_test_symbol() {
        let symbol = get_test_symbol();
        assert!(!symbol.is_empty());
        assert!(symbol.contains('/'));
    }

    #[test]
    fn test_format_price() {
        use rust_decimal_macros::dec;
        assert_eq!(format_price(dec!(50000.12)), "50000.12");
        assert_eq!(format_price(dec!(0.5)), "0.5000");
        assert_eq!(format_price(dec!(0.00001234)), "0.00001234");
    }

    #[test]
    fn test_format_volume() {
        use rust_decimal_macros::dec;
        assert_eq!(format_volume(dec!(1500000)), "1.50M");
        assert_eq!(format_volume(dec!(2500)), "2.50K");
        assert_eq!(format_volume(dec!(123.456)), "123.4560");
    }

    #[test]
    fn test_calculate_price_change_percent() {
        assert_eq!(calculate_price_change_percent(100.0, 110.0), 10.0);
        assert_eq!(calculate_price_change_percent(100.0, 90.0), -10.0);
        assert_eq!(calculate_price_change_percent(0.0, 100.0), 0.0);
    }

    #[test]
    fn test_is_valid_symbol_format() {
        assert!(is_valid_symbol_format("BTC/USDT"));
        assert!(is_valid_symbol_format("ETH/BTC"));
        assert!(!is_valid_symbol_format("BTCUSDT"));
        assert!(!is_valid_symbol_format("BTC/USDT/EUR"));
    }

    #[test]
    fn test_parse_symbol() {
        let (base, quote) = parse_symbol("BTC/USDT").unwrap();
        assert_eq!(base, "BTC");
        assert_eq!(quote, "USDT");

        assert!(parse_symbol("INVALID").is_none());
        assert!(parse_symbol("BTC/USDT/EUR").is_none());
    }

    #[tokio::test]
    async fn test_retry_with_backoff_success() {
        use std::sync::{Arc, Mutex};
        let attempt = Arc::new(Mutex::new(0));
        let attempt_clone = attempt.clone();

        let result = retry_with_backoff(
            move || {
                let attempt = attempt_clone.clone();
                async move {
                    let mut a = attempt.lock().expect("lock poisoned");
                    *a += 1;
                    if *a < 3 {
                        Err(CcxtError::Network(Box::new(
                            NetworkError::ConnectionFailed("Temporary error".to_string()),
                        )))
                    } else {
                        Ok(42)
                    }
                }
            },
            5,
            10,
        )
        .await;

        assert!(result.is_ok());
        assert_eq!(result.expect("should succeed"), 42);
        assert_eq!(*attempt.lock().expect("lock poisoned"), 3);
    }

    #[tokio::test]
    async fn test_retry_with_backoff_failure() {
        let result: Result<i32, CcxtError> = retry_with_backoff(
            || async {
                Err(CcxtError::Network(Box::new(
                    NetworkError::ConnectionFailed("Permanent error".to_string()),
                )))
            },
            2,
            10,
        )
        .await;

        assert!(result.is_err());
    }

    #[test]
    fn test_init_test_config() {
        // 这个测试只验证函数可以被调用，不验证具体内容
        // 因为配置内容依赖于环境
        let _config = init_test_config();
        // 只要不panic就算成功
    }
}
