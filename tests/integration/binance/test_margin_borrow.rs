use ccxt_exchanges::binance::Binance;
use ccxt_core::error::Result;
use std::collections::HashMap;

/// 测试获取全仓借贷利率
#[tokio::test]
#[ignore] // 需要API密钥，默认跳过
async fn test_fetch_cross_borrow_rate() -> Result<()> {
    let binance = create_authenticated_client();
    
    // 测试BTC的全仓借贷利率
    let rate = binance.fetch_cross_borrow_rate("BTC", None).await?;
    
    // 验证返回的数据
    assert_eq!(rate.currency, "BTC");
    assert!(rate.rate > 0.0, "借贷利率应该大于0");
    assert!(rate.timestamp > 0, "时间戳应该大于0");
    assert!(rate.datetime.is_some(), "应该有日期时间字符串");
    
    println!("✅ BTC全仓借贷利率: {}", rate.rate);
    Ok(())
}

/// 测试获取逐仓借贷利率（单个交易对）
#[tokio::test]
#[ignore] // 需要API密钥，默认跳过
async fn test_fetch_isolated_borrow_rate() -> Result<()> {
    let mut binance = create_authenticated_client();
    
    // 测试BTC/USDT的逐仓借贷利率
    let rate = binance.fetch_isolated_borrow_rate("BTC/USDT", None).await?;
    
    // 验证返回的数据
    assert_eq!(rate.symbol, "BTC/USDT");
    assert_eq!(rate.base, "BTC");
    assert_eq!(rate.quote, "USDT");
    assert!(rate.base_rate > 0.0, "基础币种借贷利率应该大于0");
    assert!(rate.quote_rate > 0.0, "计价币种借贷利率应该大于0");
    assert!(rate.base_borrow_limit > 0.0, "基础币种借贷限额应该大于0");
    assert!(rate.quote_borrow_limit > 0.0, "计价币种借贷限额应该大于0");
    assert!(rate.timestamp > 0, "时间戳应该大于0");
    
    println!("✅ BTC/USDT逐仓借贷利率:");
    println!("   - BTC利率: {}", rate.base_rate);
    println!("   - USDT利率: {}", rate.quote_rate);
    Ok(())
}

/// 测试获取所有逐仓借贷利率
#[tokio::test]
#[ignore] // 需要API密钥，默认跳过
async fn test_fetch_isolated_borrow_rates() -> Result<()> {
    let binance = create_authenticated_client();
    
    // 获取所有逐仓借贷利率
    let rates = binance.fetch_isolated_borrow_rates(None).await?;
    
    // 验证返回的数据
    assert!(!rates.is_empty(), "应该返回至少一个交易对的借贷利率");
    
    // 检查第一个交易对的数据完整性
    let (symbol, rate) = rates.iter().next().unwrap();
    assert!(!symbol.is_empty(), "交易对符号不应为空");
    assert!(rate.base_rate >= 0.0, "基础币种利率应该>=0");
    assert!(rate.quote_rate >= 0.0, "计价币种利率应该>=0");
    
    println!("✅ 获取到 {} 个交易对的逐仓借贷利率", rates.len());
    
    // 显示前3个交易对的利率
    for (i, (sym, r)) in rates.iter().take(3).enumerate() {
        println!("   {}. {} - BTC利率: {}, USDT利率: {}", 
            i + 1, sym, r.base_rate, r.quote_rate);
    }
    
    Ok(())
}

/// 测试获取借贷利息历史（仅指定币种）
#[tokio::test]
#[ignore] // 需要API密钥，默认跳过
async fn test_fetch_borrow_interest_by_currency() -> Result<()> {
    let binance = create_authenticated_client();
    
    // 获取USDT的借贷利息历史
    let interests = binance.fetch_borrow_interest(
        Some("USDT"),
        None,
        None,
        Some(10),
        None
    ).await?;
    
    // 验证返回的数据
    if !interests.is_empty() {
        let interest = &interests[0];
        assert_eq!(interest.currency, "USDT");
        assert!(interest.interest >= 0.0, "利息应该>=0");
        assert!(interest.interest_rate >= 0.0, "利率应该>=0");
        assert!(interest.timestamp > 0, "时间戳应该大于0");
        
        println!("✅ 获取到 {} 条USDT借贷利息记录", interests.len());
        println!("   最新一条: 利息={}, 利率={}", 
            interest.interest, interest.interest_rate);
    } else {
        println!("⚠️  没有USDT借贷利息记录（可能未借贷）");
    }
    
    Ok(())
}

/// 测试获取借贷利息历史（指定交易对，逐仓）
#[tokio::test]
#[ignore] // 需要API密钥，默认跳过
async fn test_fetch_borrow_interest_isolated() -> Result<()> {
    let binance = create_authenticated_client();
    
    // 获取BTC/USDT逐仓的BTC借贷利息历史
    let mut params = HashMap::new();
    params.insert("isolatedSymbol".to_string(), "BTCUSDT".to_string());
    
    let interests = binance.fetch_borrow_interest(
        Some("BTC"),
        Some("BTC/USDT"),
        None,
        Some(5),
        Some(params)
    ).await?;
    
    // 验证返回的数据
    if !interests.is_empty() {
        let interest = &interests[0];
        assert_eq!(interest.currency, "BTC");
        assert!(interest.interest >= 0.0, "利息应该>=0");
        assert!(interest.amount >= 0.0, "借贷本金应该>=0");
        
        println!("✅ 获取到 {} 条BTC/USDT逐仓借贷利息记录", interests.len());
    } else {
        println!("⚠️  没有BTC/USDT逐仓借贷利息记录");
    }
    
    Ok(())
}

/// 测试获取借贷利率历史
#[tokio::test]
#[ignore] // 需要API密钥，默认跳过
async fn test_fetch_borrow_rate_history() -> Result<()> {
    let binance = create_authenticated_client();
    
    // 获取BTC的借贷利率历史
    let history = binance.fetch_borrow_rate_history(
        "BTC",
        None,
        Some(30),
        None
    ).await?;
    
    // 验证返回的数据
    assert!(!history.is_empty(), "应该返回至少一条历史记录");
    assert!(history.len() <= 30, "返回的记录数不应超过limit");
    
    let first = &history[0];
    assert_eq!(first.currency, "BTC");
    assert!(first.rate > 0.0, "借贷利率应该大于0");
    assert!(first.timestamp > 0, "时间戳应该大于0");
    
    println!("✅ 获取到 {} 条BTC借贷利率历史记录", history.len());
    println!("   最新利率: {}", first.rate);
    
    Ok(())
}

/// 测试借贷利率历史的时间范围查询
#[tokio::test]
#[ignore] // 需要API密钥，默认跳过
async fn test_fetch_borrow_rate_history_with_time_range() -> Result<()> {
    let binance = create_authenticated_client();
    
    // 计算7天前的时间戳
    let now = chrono::Utc::now().timestamp_millis() as u64;
    let seven_days_ago = now - 7 * 24 * 60 * 60 * 1000;
    
    // 获取过去7天的借贷利率历史
    let history = binance.fetch_borrow_rate_history(
        "USDT",
        Some(seven_days_ago),
        Some(50),
        None
    ).await?;
    
    // 验证返回的数据
    assert!(!history.is_empty(), "应该返回至少一条历史记录");
    
    // 验证所有记录的时间戳都在指定范围内
    for item in &history {
        assert!(item.timestamp >= seven_days_ago, 
            "时间戳应该在指定范围内");
        assert_eq!(item.currency, "USDT");
    }
    
    println!("✅ 获取到 {} 条过去7天的USDT借贷利率历史", history.len());
    if !history.is_empty() {
        println!("   最早: {} - 利率: {}", 
            history.last().unwrap().datetime.as_deref().unwrap_or("N/A"),
            history.last().unwrap().rate);
        println!("   最新: {} - 利率: {}", 
            history[0].datetime.as_deref().unwrap_or("N/A"),
            history[0].rate);
    }
    
    Ok(())
}

/// 测试借贷利率历史的limit限制
#[tokio::test]
#[ignore] // 需要API密钥，默认跳过
async fn test_fetch_borrow_rate_history_limit_constraint() -> Result<()> {
    let binance = create_authenticated_client();
    
    // Binance API要求limit不能超过92
    let history = binance.fetch_borrow_rate_history(
        "BTC",
        None,
        Some(92), // 最大值
        None
    ).await?;
    
    assert!(history.len() <= 92, "返回的记录数应该<=92");
    println!("✅ limit=92时，获取到 {} 条记录", history.len());
    
    Ok(())
}

/// 测试Portfolio Margin模式的借贷利息查询
#[tokio::test]
#[ignore] // 需要API密钥和Portfolio Margin权限，默认跳过
async fn test_fetch_borrow_interest_portfolio_margin() -> Result<()> {
    let binance = create_authenticated_client();
    
    // 使用Portfolio Margin模式
    let mut params = HashMap::new();
    params.insert("portfolioMargin".to_string(), "true".to_string());
    
    let interests = binance.fetch_borrow_interest(
        Some("BTC"),
        None,
        None,
        Some(10),
        Some(params)
    ).await?;
    
    // 验证返回的数据
    if !interests.is_empty() {
        println!("✅ Portfolio Margin模式: 获取到 {} 条借贷利息记录", 
            interests.len());
    } else {
        println!("⚠️  Portfolio Margin模式: 没有借贷利息记录");
    }
    
    Ok(())
}

/// 测试错误处理：无效的币种代码
#[tokio::test]
#[ignore] // 需要API密钥，默认跳过
async fn test_error_handling_invalid_currency() -> Result<()> {
    let binance = create_authenticated_client();
    
    // 使用无效的币种代码
    let result = binance.fetch_cross_borrow_rate("INVALID_COIN", None).await;
    
    // 应该返回错误
    assert!(result.is_err(), "应该返回错误");
    println!("✅ 正确处理了无效币种代码的错误");
    
    Ok(())
}

/// 测试错误处理：超出limit限制
#[tokio::test]
#[ignore] // 需要API密钥，默认跳过
async fn test_error_handling_exceed_limit() -> Result<()> {
    let binance = create_authenticated_client();
    
    // limit超过92会被自动限制
    let history = binance.fetch_borrow_rate_history(
        "BTC",
        None,
        Some(100), // 超过92
        None
    ).await?;
    
    // 应该返回最多92条记录
    assert!(history.len() <= 92, "应该自动限制为92条");
    println!("✅ 正确处理了超出limit限制的情况");
    
    Ok(())
}

// ==================== 辅助函数 ====================

/// 创建已认证的客户端
/// 
/// 注意：需要设置环境变量或修改此函数以提供真实的API密钥
fn create_authenticated_client() -> Binance {
    let api_key = std::env::var("BINANCE_API_KEY")
        .unwrap_or_else(|_| "YOUR_API_KEY".to_string());
    let api_secret = std::env::var("BINANCE_API_SECRET")
        .unwrap_or_else(|_| "YOUR_API_SECRET".to_string());
    
    Binance::new(
        Some(api_key),
        Some(api_secret),
        None,
    )
}

// ==================== 性能测试 ====================

/// 性能测试：批量查询借贷利率
#[tokio::test]
#[ignore] // 性能测试，默认跳过
async fn benchmark_fetch_borrow_rates() -> Result<()> {
    let binance = create_authenticated_client();
    
    let start = std::time::Instant::now();
    
    // 连续查询10次
    for i in 0..10 {
        let _ = binance.fetch_cross_borrow_rate("BTC", None).await?;
        println!("  第{}次查询完成", i + 1);
    }
    
    let duration = start.elapsed();
    let avg_time = duration.as_millis() / 10;
    
    println!("✅ 10次查询总耗时: {:?}", duration);
    println!("   平均耗时: {}ms", avg_time);
    
    Ok(())
}