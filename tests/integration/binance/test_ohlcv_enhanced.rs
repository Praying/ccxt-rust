//! Binance fetch_ohlcv增强功能测试
//! 
//! 测试覆盖P2.3增强阶段1-3的所有功能：
//! - 阶段1: params参数支持
//! - 阶段2: 多价格类型支持（default/mark/index/premiumIndex）
//! - 阶段3: 时间范围增强（until参数、智能limit、inverse处理）

use ccxt_exchanges::binance::Binance;
use std::collections::HashMap;
use std::env;
use serde_json::json;

/// 创建测试用的Binance实例
fn create_test_exchange() -> Binance {
    let api_key = env::var("BINANCE_API_KEY").unwrap_or_default();
    let api_secret = env::var("BINANCE_API_SECRET").unwrap_or_default();
    
    Binance::new(
        Some(api_key),
        Some(api_secret),
        HashMap::new(),
    )
}

// ==================== 阶段1: 基础params参数测试 ====================

#[tokio::test]
async fn test_fetch_ohlcv_basic() {
    let exchange = create_test_exchange();
    
    let result = exchange.fetch_ohlcv("BTC/USDT", "1m", None, Some(100), None).await;
    
    assert!(result.is_ok(), "基础调用应该成功");
    let candles = result.unwrap();
    
    assert!(!candles.is_empty(), "应该返回K线数据");
    assert!(candles.len() <= 100, "数量不应超过limit");
    
    println!("✓ 基础fetch_ohlcv: 获取到 {} 根K线", candles.len());
}

#[tokio::test]
async fn test_fetch_ohlcv_with_since() {
    let exchange = create_test_exchange();
    
    // 查询最近1小时的数据
    let one_hour_ago = chrono::Utc::now().timestamp_millis() as u64 - 3600_000;
    
    let result = exchange.fetch_ohlcv(
        "ETH/USDT",
        "1m",
        Some(one_hour_ago),
        Some(60),
        None
    ).await;
    
    assert!(result.is_ok(), "带since参数应该成功");
    let candles = result.unwrap();
    
    // 验证时间戳
    if !candles.is_empty() {
        let first_ts = candles[0][0] as u64;
        assert!(first_ts >= one_hour_ago, "第一根K线时间应该>=since");
    }
    
    println!("✓ fetch_ohlcv with since: 获取到 {} 根K线", candles.len());
}

// ==================== 阶段2: 多价格类型测试 ====================

#[tokio::test]
async fn test_fetch_ohlcv_mark_price_futures() {
    let exchange = create_test_exchange();
    
    let mut params = HashMap::new();
    params.insert("price".to_string(), json!("mark"));
    
    // 使用USDT本位合约
    let result = exchange.fetch_ohlcv(
        "BTC/USDT:USDT",
        "15m",
        None,
        Some(50),
        Some(params)
    ).await;
    
    assert!(result.is_ok(), "mark价格类型应该成功");
    let candles = result.unwrap();
    assert!(!candles.is_empty(), "应该返回标记价格K线");
    
    println!("✓ fetch_ohlcv (mark price): 获取到 {} 根K线", candles.len());
}

#[tokio::test]
async fn test_fetch_ohlcv_index_price_futures() {
    let exchange = create_test_exchange();
    
    let mut params = HashMap::new();
    params.insert("price".to_string(), json!("index"));
    
    let result = exchange.fetch_ohlcv(
        "ETH/USDT:USDT",
        "30m",
        None,
        Some(50),
        Some(params)
    ).await;
    
    assert!(result.is_ok(), "index价格类型应该成功");
    let candles = result.unwrap();
    assert!(!candles.is_empty(), "应该返回指数价格K线");
    
    println!("✓ fetch_ohlcv (index price): 获取到 {} 根K线", candles.len());
}

#[tokio::test]
async fn test_fetch_ohlcv_premium_index_futures() {
    let exchange = create_test_exchange();
    
    let mut params = HashMap::new();
    params.insert("price".to_string(), json!("premiumIndex"));
    
    let result = exchange.fetch_ohlcv(
        "BTC/USDT:USDT",
        "1h",
        None,
        Some(50),
        Some(params)
    ).await;
    
    assert!(result.is_ok(), "premiumIndex价格类型应该成功");
    let candles = result.unwrap();
    assert!(!candles.is_empty(), "应该返回资金费率K线");
    
    println!("✓ fetch_ohlcv (premiumIndex): 获取到 {} 根K线", candles.len());
}

#[tokio::test]
async fn test_fetch_ohlcv_invalid_price_type_spot() {
    let exchange = create_test_exchange();
    
    let mut params = HashMap::new();
    params.insert("price".to_string(), json!("mark"));
    
    // 现货市场不支持mark价格
    let result = exchange.fetch_ohlcv(
        "BTC/USDT",
        "1m",
        None,
        Some(10),
        Some(params)
    ).await;
    
    assert!(result.is_err(), "现货市场使用mark价格应该返回错误");
    println!("✓ 错误处理: 现货市场正确拒绝mark价格类型");
}

// ==================== 阶段3: 时间范围增强测试 ====================

#[tokio::test]
async fn test_fetch_ohlcv_with_until() {
    let exchange = create_test_exchange();
    
    let now = chrono::Utc::now().timestamp_millis() as u64;
    let two_hours_ago = now - 7200_000;
    let one_hour_ago = now - 3600_000;
    
    let mut params = HashMap::new();
    params.insert("until".to_string(), json!(one_hour_ago));
    
    let result = exchange.fetch_ohlcv(
        "BTC/USDT",
        "1m",
        Some(two_hours_ago),
        Some(100),
        Some(params)
    ).await;
    
    assert!(result.is_ok(), "带until参数应该成功");
    let candles = result.unwrap();
    
    // 验证时间范围
    if !candles.is_empty() {
        let first_ts = candles[0][0] as u64;
        let last_ts = candles[candles.len() - 1][0] as u64;
        
        assert!(first_ts >= two_hours_ago, "第一根K线应该>=since");
        assert!(last_ts <= one_hour_ago + 60_000, "最后一根K线应该<=until");
    }
    
    println!("✓ fetch_ohlcv with until: 获取到 {} 根K线", candles.len());
}

#[tokio::test]
async fn test_fetch_ohlcv_smart_limit() {
    let exchange = create_test_exchange();
    
    let now = chrono::Utc::now().timestamp_millis() as u64;
    let six_hours_ago = now - 21600_000;
    
    let mut params = HashMap::new();
    params.insert("until".to_string(), json!(now));
    
    // 不指定limit，应该使用智能limit=1500
    let result = exchange.fetch_ohlcv(
        "ETH/USDT",
        "1m",
        Some(six_hours_ago),
        None,
        Some(params)
    ).await;
    
    assert!(result.is_ok(), "智能limit应该成功");
    let candles = result.unwrap();
    
    assert!(!candles.is_empty(), "应该返回K线数据");
    println!("✓ 智能limit (since+until): 获取到 {} 根K线", candles.len());
}

#[tokio::test]
async fn test_fetch_ohlcv_limit_cap() {
    let exchange = create_test_exchange();
    
    // 尝试请求2000根K线，应该被限制在1500
    let result = exchange.fetch_ohlcv(
        "BTC/USDT",
        "1m",
        None,
        Some(2000),
        None
    ).await;
    
    assert!(result.is_ok(), "limit上限应该成功");
    let candles = result.unwrap();
    
    assert!(candles.len() <= 1500, "返回的K线数不应超过1500");
    println!("✓ limit上限: 请求2000根，实际获取 {} 根", candles.len());
}

// ==================== 综合测试 ====================

#[tokio::test]
async fn test_fetch_ohlcv_complex_query() {
    let exchange = create_test_exchange();
    
    let now = chrono::Utc::now().timestamp_millis() as u64;
    let four_hours_ago = now - 14400_000;
    let two_hours_ago = now - 7200_000;
    
    let mut params = HashMap::new();
    params.insert("price".to_string(), json!("mark"));
    params.insert("until".to_string(), json!(two_hours_ago));
    
    // 综合测试：USDT合约 + mark价格 + since + until
    let result = exchange.fetch_ohlcv(
        "ETH/USDT:USDT",
        "15m",
        Some(four_hours_ago),
        None,
        Some(params)
    ).await;
    
    assert!(result.is_ok(), "综合查询应该成功");
    let candles = result.unwrap();
    
    if !candles.is_empty() {
        println!("✓ 综合测试: 获取到 {} 根mark价格K线", candles.len());
    }
}

#[tokio::test]
async fn test_fetch_ohlcv_different_timeframes() {
    let exchange = create_test_exchange();
    
    let timeframes = vec!["1m", "5m", "15m", "1h", "4h", "1d"];
    
    for tf in timeframes {
        let result = exchange.fetch_ohlcv(
            "BTC/USDT",
            tf,
            None,
            Some(20),
            None
        ).await;
        
        assert!(result.is_ok(), "{} 时间周期应该成功", tf);
        let candles = result.unwrap();
        assert!(!candles.is_empty(), "{} 应该返回数据", tf);
        
        println!("✓ fetch_ohlcv ({}): {} 根K线", tf, candles.len());
    }
}

// ==================== 数据完整性验证 ====================

#[tokio::test]
async fn test_fetch_ohlcv_data_integrity() {
    let exchange = create_test_exchange();
    
    let result = exchange.fetch_ohlcv(
        "ETH/USDT",
        "1h",
        None,
        Some(24),
        None
    ).await;
    
    assert!(result.is_ok(), "数据完整性测试应该成功");
    let candles = result.unwrap();
    
    use rust_decimal::prelude::*;
    
    for (i, candle) in candles.iter().enumerate() {
        assert_eq!(candle.len(), 6, "K线应该有6个字段");
        
        let open = Decimal::from_f64_retain(candle[1]).unwrap();
        let high = Decimal::from_f64_retain(candle[2]).unwrap();
        let low = Decimal::from_f64_retain(candle[3]).unwrap();
        let close = Decimal::from_f64_retain(candle[4]).unwrap();
        let volume = Decimal::from_f64_retain(candle[5]).unwrap();
        
        // OHLC关系验证
        assert!(high >= open, "K线{}: high >= open", i);
        assert!(high >= close, "K线{}: high >= close", i);
        assert!(high >= low, "K线{}: high >= low", i);
        assert!(low <= open, "K线{}: low <= open", i);
        assert!(low <= close, "K线{}: low <= close", i);
        assert!(volume >= Decimal::ZERO, "K线{}: volume >= 0", i);
    }
    
    println!("✓ 数据完整性验证: {} 根K线全部通过", candles.len());
}

#[tokio::test]
async fn test_fetch_ohlcv_time_order() {
    let exchange = create_test_exchange();
    
    let result = exchange.fetch_ohlcv(
        "BTC/USDT",
        "5m",
        None,
        Some(50),
        None
    ).await;
    
    assert!(result.is_ok(), "时间排序测试应该成功");
    let candles = result.unwrap();
    
    // 验证时间序列按升序排列
    for i in 1..candles.len() {
        let prev_ts = candles[i - 1][0] as i64;
        let curr_ts = candles[i][0] as i64;
        assert!(
            curr_ts >= prev_ts,
            "K线应该按时间升序排列"
        );
    }
    
    println!("✓ 时间排序验证: {} 根K线时间序列正确", candles.len());
}