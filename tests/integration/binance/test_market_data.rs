//! Binance市场数据API集成测试
//! 
//! 测试覆盖：
//! 1. fetch_currencies - 币种信息查询
//! 2. fetch_recent_trades - 最近公开成交
//! 3. fetch_my_recent_trades - 我的成交记录
//! 4. fetch_agg_trades - 聚合成交
//! 5. fetch_historical_trades - 历史成交
//! 6. fetch_24hr_stats - 24小时统计
//! 7. fetch_trading_limits - 交易限制
//! 8. fetch_bids_asks - 最优买卖价

use ccxt_exchanges::binance::Binance;
use std::collections::HashMap;
use std::env;

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

/// 检查是否配置了API凭证
fn has_api_credentials() -> bool {
    env::var("BINANCE_API_KEY").is_ok() && env::var("BINANCE_API_SECRET").is_ok()
}

// ==================== 公开API测试 ====================

#[tokio::test]
async fn test_fetch_recent_trades() {
    let exchange = create_test_exchange();
    
    let result = exchange.fetch_recent_trades("BTC/USDT", Some(10), None).await;
    
    assert!(result.is_ok(), "fetch_recent_trades应该成功");
    let trades = result.unwrap();
    
    // 验证基本属性
    assert!(!trades.is_empty(), "应该返回至少一笔成交");
    assert!(trades.len() <= 10, "成交数量不应超过限制");
    
    // 验证第一笔成交的数据
    let trade = &trades[0];
    assert!(trade.price > rust_decimal::Decimal::ZERO, "价格应该大于0");
    assert!(trade.amount > rust_decimal::Decimal::ZERO, "数量应该大于0");
    assert!(!trade.side.is_empty(), "方向不应为空");
    assert!(trade.timestamp > 0, "时间戳应该大于0");
    
    println!("✓ fetch_recent_trades: 获取到 {} 笔成交", trades.len());
}

#[tokio::test]
async fn test_fetch_recent_trades_with_limit() {
    let exchange = create_test_exchange();
    
    // 测试限制为5
    let result = exchange.fetch_recent_trades("ETH/USDT", Some(5), None).await;
    assert!(result.is_ok());
    
    let trades = result.unwrap();
    assert!(trades.len() <= 5, "成交数量应该不超过5");
    
    println!("✓ fetch_recent_trades with limit: {} 笔成交", trades.len());
}

#[tokio::test]
async fn test_fetch_agg_trades() {
    let exchange = create_test_exchange();
    
    let mut params = HashMap::new();
    params.insert("limit".to_string(), serde_json::json!(20));
    
    let result = exchange.fetch_agg_trades("BTC/USDT", None, None, Some(params)).await;
    
    assert!(result.is_ok(), "fetch_agg_trades应该成功");
    let agg_trades = result.unwrap();
    
    assert!(!agg_trades.is_empty(), "应该返回聚合成交");
    assert!(agg_trades.len() <= 20, "聚合成交数量不应超过限制");
    
    // 验证聚合成交数据
    let agg = &agg_trades[0];
    assert!(agg.agg_trade_id > 0, "聚合成交ID应该大于0");
    assert!(agg.price > rust_decimal::Decimal::ZERO, "价格应该大于0");
    assert!(agg.quantity > rust_decimal::Decimal::ZERO, "数量应该大于0");
    assert!(agg.first_trade_id > 0, "首个成交ID应该大于0");
    assert!(agg.last_trade_id >= agg.first_trade_id, "最后成交ID应该>=首个成交ID");
    assert!(agg.timestamp > 0, "时间戳应该大于0");
    
    // 验证计算方法
    assert!(agg.cost() > rust_decimal::Decimal::ZERO, "成本应该大于0");
    assert!(agg.trade_count() >= 1, "成交数量应该>=1");
    assert!(!agg.side().is_empty(), "方向不应为空");
    
    println!("✓ fetch_agg_trades: 获取到 {} 笔聚合成交", agg_trades.len());
}

#[tokio::test]
async fn test_fetch_agg_trades_with_time_range() {
    let exchange = create_test_exchange();
    
    let now = chrono::Utc::now().timestamp_millis();
    let one_hour_ago = now - 3600 * 1000;
    
    let mut params = HashMap::new();
    params.insert("startTime".to_string(), serde_json::json!(one_hour_ago));
    params.insert("endTime".to_string(), serde_json::json!(now));
    params.insert("limit".to_string(), serde_json::json!(100));
    
    let result = exchange.fetch_agg_trades("ETH/USDT", None, None, Some(params)).await;
    assert!(result.is_ok());
    
    let agg_trades = result.unwrap();
    println!("✓ fetch_agg_trades with time range: {} 笔聚合成交", agg_trades.len());
}

#[tokio::test]
async fn test_fetch_24hr_stats_single() {
    let exchange = create_test_exchange();
    
    let result = exchange.fetch_24hr_stats(Some("BTC/USDT"), None).await;
    
    assert!(result.is_ok(), "fetch_24hr_stats应该成功");
    let stats = result.unwrap();
    
    assert_eq!(stats.len(), 1, "单个交易对应该返回1个统计");
    
    // 验证统计数据
    let stat = &stats[0];
    assert_eq!(stat.symbol, "BTCUSDT", "交易对符号应该匹配");
    assert!(stat.open > rust_decimal::Decimal::ZERO, "开盘价应该大于0");
    assert!(stat.high > rust_decimal::Decimal::ZERO, "最高价应该大于0");
    assert!(stat.low > rust_decimal::Decimal::ZERO, "最低价应该大于0");
    assert!(stat.close > rust_decimal::Decimal::ZERO, "收盘价应该大于0");
    assert!(stat.volume > rust_decimal::Decimal::ZERO, "成交量应该大于0");
    assert!(stat.quote_volume > rust_decimal::Decimal::ZERO, "报价成交量应该大于0");
    assert!(stat.timestamp > 0, "时间戳应该大于0");
    
    // 验证价格关系
    assert!(stat.high >= stat.low, "最高价应该>=最低价");
    assert!(stat.close >= stat.low, "收盘价应该>=最低价");
    assert!(stat.close <= stat.high, "收盘价应该<=最高价");
    
    println!("✓ fetch_24hr_stats single: {} 涨跌幅={:.2}%", 
        stat.symbol,
        stat.price_change_percent.unwrap_or_default());
}

#[tokio::test]
async fn test_fetch_24hr_stats_all() {
    let exchange = create_test_exchange();
    
    let result = exchange.fetch_24hr_stats(None, None).await;
    
    assert!(result.is_ok(), "fetch_24hr_stats(all)应该成功");
    let stats = result.unwrap();
    
    assert!(stats.len() > 100, "所有交易对应该超过100个");
    
    // 验证包含BTCUSDT
    let btc_stat = stats.iter().find(|s| s.symbol == "BTCUSDT");
    assert!(btc_stat.is_some(), "应该包含BTCUSDT");
    
    println!("✓ fetch_24hr_stats all: 获取到 {} 个交易对统计", stats.len());
}

#[tokio::test]
async fn test_fetch_bids_asks_single() {
    let exchange = create_test_exchange();
    
    let result = exchange.fetch_bids_asks(
        Some(vec!["BTC/USDT".to_string()]), 
        None
    ).await;
    
    assert!(result.is_ok(), "fetch_bids_asks应该成功");
    let bids_asks = result.unwrap();
    
    assert_eq!(bids_asks.len(), 1, "单个交易对应该返回1个买卖价");
    
    // 验证买卖价数据
    let ba = &bids_asks[0];
    assert_eq!(ba.symbol, "BTCUSDT", "交易对符号应该匹配");
    assert!(ba.bid > rust_decimal::Decimal::ZERO, "买一价应该大于0");
    assert!(ba.ask > rust_decimal::Decimal::ZERO, "卖一价应该大于0");
    assert!(ba.ask > ba.bid, "卖一价应该大于买一价");
    
    // 验证辅助方法
    let spread = ba.spread();
    assert!(spread > rust_decimal::Decimal::ZERO, "价差应该大于0");
    
    let spread_pct = ba.spread_percentage();
    assert!(spread_pct > rust_decimal::Decimal::ZERO, "价差百分比应该大于0");
    assert!(spread_pct < rust_decimal::Decimal::from(10), "价差百分比应该<10%");
    
    let mid = ba.mid_price();
    assert!(mid > ba.bid, "中间价应该大于买一价");
    assert!(mid < ba.ask, "中间价应该小于卖一价");
    
    println!("✓ fetch_bids_asks single: {} 买={} 卖={} 价差={:.4}%", 
        ba.symbol, ba.bid, ba.ask, spread_pct);
}

#[tokio::test]
async fn test_fetch_bids_asks_multiple() {
    let exchange = create_test_exchange();
    
    let symbols = vec![
        "BTC/USDT".to_string(),
        "ETH/USDT".to_string(),
        "BNB/USDT".to_string(),
    ];
    
    let result = exchange.fetch_bids_asks(Some(symbols.clone()), None).await;
    
    assert!(result.is_ok(), "fetch_bids_asks(multiple)应该成功");
    let bids_asks = result.unwrap();
    
    assert_eq!(bids_asks.len(), 3, "应该返回3个交易对的买卖价");
    
    // 验证每个交易对
    for ba in &bids_asks {
        assert!(ba.ask > ba.bid, "{} 卖一价应该大于买一价", ba.symbol);
    }
    
    println!("✓ fetch_bids_asks multiple: 获取到 {} 个交易对", bids_asks.len());
}

#[tokio::test]
async fn test_fetch_bids_asks_all() {
    let exchange = create_test_exchange();
    
    let result = exchange.fetch_bids_asks(None, None).await;
    
    assert!(result.is_ok(), "fetch_bids_asks(all)应该成功");
    let bids_asks = result.unwrap();
    
    assert!(bids_asks.len() > 100, "所有交易对应该超过100个");
    
    println!("✓ fetch_bids_asks all: 获取到 {} 个交易对", bids_asks.len());
}

#[tokio::test]
async fn test_fetch_trading_limits() {
    let exchange = create_test_exchange();
    
    let result = exchange.fetch_trading_limits("BTC/USDT", None).await;
    
    assert!(result.is_ok(), "fetch_trading_limits应该成功");
    let limits = result.unwrap();
    
    // 验证价格限制
    assert!(limits.price_min.is_some(), "应该有最小价格限制");
    assert!(limits.price_max.is_some(), "应该有最大价格限制");
    
    if let (Some(min), Some(max)) = (limits.price_min, limits.price_max) {
        assert!(max > min, "最大价格应该大于最小价格");
    }
    
    // 验证数量限制
    assert!(limits.amount_min.is_some(), "应该有最小数量限制");
    assert!(limits.amount_max.is_some(), "应该有最大数量限制");
    
    if let (Some(min), Some(max)) = (limits.amount_min, limits.amount_max) {
        assert!(max > min, "最大数量应该大于最小数量");
    }
    
    // 验证金额限制
    assert!(limits.cost_min.is_some(), "应该有最小金额限制");
    
    println!("✓ fetch_trading_limits: 价格=[{:?}, {:?}], 数量=[{:?}, {:?}]",
        limits.price_min, limits.price_max,
        limits.amount_min, limits.amount_max);
}

// ==================== 需要API Key的测试 ====================

#[tokio::test]
async fn test_fetch_historical_trades() {
    if !has_api_credentials() {
        println!("⊘ 跳过 fetch_historical_trades: 需要API凭证");
        return;
    }
    
    let exchange = create_test_exchange();
    
    let result = exchange.fetch_historical_trades("BTC/USDT", Some(10), None, None).await;
    
    assert!(result.is_ok(), "fetch_historical_trades应该成功");
    let trades = result.unwrap();
    
    assert!(!trades.is_empty(), "应该返回历史成交");
    assert!(trades.len() <= 10, "成交数量不应超过限制");
    
    // 验证成交数据
    let trade = &trades[0];
    assert!(trade.id.is_some(), "应该有成交ID");
    assert!(trade.price > rust_decimal::Decimal::ZERO, "价格应该大于0");
    assert!(trade.amount > rust_decimal::Decimal::ZERO, "数量应该大于0");
    
    println!("✓ fetch_historical_trades: 获取到 {} 笔历史成交", trades.len());
}

// ==================== 私有API测试 ====================

#[tokio::test]
async fn test_fetch_currencies() {
    if !has_api_credentials() {
        println!("⊘ 跳过 fetch_currencies: 需要API凭证");
        return;
    }
    
    let exchange = create_test_exchange();
    
    let result = exchange.fetch_currencies(None).await;
    
    assert!(result.is_ok(), "fetch_currencies应该成功");
    let currencies = result.unwrap();
    
    assert!(!currencies.is_empty(), "应该返回币种列表");
    assert!(currencies.len() > 50, "币种数量应该超过50个");
    
    // 查找BTC
    let btc = currencies.iter().find(|c| c.id == "BTC");
    assert!(btc.is_some(), "应该包含BTC");
    
    if let Some(btc) = btc {
        assert_eq!(btc.code, "BTC", "币种代码应该匹配");
        assert!(!btc.name.is_empty(), "币种名称不应为空");
        
        // 验证网络信息
        if let Some(networks) = &btc.networks {
            assert!(!networks.is_empty(), "BTC应该有网络信息");
        }
    }
    
    println!("✓ fetch_currencies: 获取到 {} 个币种", currencies.len());
}

#[tokio::test]
async fn test_fetch_my_recent_trades() {
    if !has_api_credentials() {
        println!("⊘ 跳过 fetch_my_recent_trades: 需要API凭证");
        return;
    }
    
    let exchange = create_test_exchange();
    
    let mut params = HashMap::new();
    params.insert("limit".to_string(), serde_json::json!(10));
    
    let result = exchange.fetch_my_recent_trades(
        "BTC/USDT",
        None,
        None,
        Some(params)
    ).await;
    
    // 注意：这个测试可能返回空列表（如果没有成交记录）
    match result {
        Ok(trades) => {
            if trades.is_empty() {
                println!("ℹ fetch_my_recent_trades: 暂无成交记录");
            } else {
                println!("✓ fetch_my_recent_trades: 获取到 {} 笔成交", trades.len());
                
                // 验证第一笔成交
                let trade = &trades[0];
                assert!(trade.id.is_some(), "应该有成交ID");
                assert!(trade.price > rust_decimal::Decimal::ZERO, "价格应该大于0");
                assert!(trade.amount > rust_decimal::Decimal::ZERO, "数量应该大于0");
            }
        }
        Err(e) => {
            let err_msg = e.to_string();
            if err_msg.contains("permission") || err_msg.contains("IP") {
                println!("⚠ fetch_my_recent_trades: API权限不足");
            } else {
                panic!("Unexpected error: {}", e);
            }
        }
    }
}

// ==================== 错误处理测试 ====================

#[tokio::test]
async fn test_invalid_symbol() {
    let exchange = create_test_exchange();
    
    let result = exchange.fetch_recent_trades("INVALID/PAIR", Some(5), None).await;
    assert!(result.is_err(), "无效交易对应该返回错误");
    
    println!("✓ 错误处理: 无效交易对被正确捕获");
}