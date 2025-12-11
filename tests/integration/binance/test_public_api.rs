//! Binance公开API集成测试
//!
//! 测试所有不需要认证的公开API端点

// Allow clippy warnings for test code
#![allow(clippy::to_string_in_format_args)]
#![allow(unused_imports)]

use anyhow::Context;

// 导入测试辅助工具 - 修正路径
use crate::common::assertions::assert_reasonable_timestamp;
use crate::common::helpers::{
    create_binance, ensure_markets_loaded, format_price, format_price_opt, format_volume,
    get_test_symbol, get_test_symbols, init_test_config, print_test_separator, print_test_step,
    print_test_summary, retry_with_backoff, should_skip_integration_tests,
};
use ccxt_core::types::market::MarketType;

/// Level 1: 基础连通性测试 - fetch_markets
#[tokio::test]
async fn test_fetch_markets() -> anyhow::Result<()> {
    let start = std::time::Instant::now();
    print_test_separator("Binance: fetch_markets");

    let config = init_test_config();

    // 检查是否跳过集成测试
    if should_skip_integration_tests(&config) {
        println!("⏭ Skipping integration test (ENABLE_INTEGRATION_TESTS=false)");
        return Ok(());
    }

    let binance = create_binance(&config)?;

    print_test_step("Fetching all markets...");
    let markets = binance
        .fetch_markets()
        .await
        .context("fetch_markets should succeed")?;

    print_test_step(&format!("Retrieved {} markets", markets.len()));

    // 验证市场数量合理
    assert!(markets.len() > 100, "Should have at least 100 markets");
    assert!(markets.len() < 10000, "Should have less than 10000 markets");

    // 验证几个主要交易对存在
    let symbols: Vec<String> = markets.iter().map(|m| m.symbol.to_string()).collect();
    assert!(
        symbols.contains(&"BTC/USDT".to_string()),
        "Should contain BTC/USDT"
    );
    assert!(
        symbols.contains(&"ETH/USDT".to_string()),
        "Should contain ETH/USDT"
    );

    // 验证第一个市场的数据完整性
    if let Some(first_market) = markets.first() {
        // 暂时注释掉宏调用，使用手动验证
        // assert_valid_market!(first_market);
        assert!(!first_market.symbol.to_string().is_empty());
        assert!(first_market.active);
        println!("  Sample market: {}", first_market.symbol.to_string());
    }

    // 统计市场类型
    let spot_count = markets
        .iter()
        .filter(|m| matches!(m.market_type, MarketType::Spot))
        .count();
    let future_count = markets
        .iter()
        .filter(|m| matches!(m.market_type, MarketType::Futures))
        .count();
    println!(
        "  Spot markets: {}, Future markets: {}",
        spot_count, future_count
    );

    let duration = start.elapsed().as_millis() as u64;
    print_test_summary("fetch_markets", true, duration);
    Ok(())
}

/// Level 1: 基础连通性测试 - fetch_ticker
#[tokio::test]
async fn test_fetch_ticker() -> anyhow::Result<()> {
    let start = std::time::Instant::now();
    print_test_separator("Binance: fetch_ticker");

    let config = init_test_config();
    if should_skip_integration_tests(&config) {
        println!("⏭ Skipping integration test");
        return Ok(());
    }

    let binance = create_binance(&config)?;

    // 先加载市场数据
    print_test_step("Loading markets...");
    binance
        .fetch_markets()
        .await
        .context("Failed to load markets")?;

    let symbol = get_test_symbol();

    print_test_step(&format!("Fetching ticker for {}...", symbol));
    let ticker = binance
        .fetch_ticker(symbol, ccxt_core::types::TickerParams::default())
        .await
        .context("fetch_ticker should succeed")?;

    // 使用自定义断言宏验证ticker数据
    // 暂时注释掉宏调用，使用手动验证
    // assert_valid_ticker!(ticker, symbol);
    assert_eq!(ticker.symbol, symbol);
    assert!(ticker.timestamp > 0);

    // 打印ticker详细信息
    println!("  Last: {}", format_price_opt(ticker.last));
    if let (Some(bid), Some(ask)) = (ticker.bid, ticker.ask) {
        println!("  Bid/Ask: {} / {}", format_price(bid), format_price(ask));
        let spread_pct = ((ask - bid) / bid) * rust_decimal_macros::dec!(100.0);
        println!("  Spread: {:.2}%", spread_pct);
    }
    if let Some(volume) = ticker.base_volume {
        println!("  24h Volume: {}", format_volume(volume));
    }
    if let (Some(high), Some(low)) = (ticker.high, ticker.low) {
        println!(
            "  24h High/Low: {} / {}",
            format_price(high),
            format_price(low)
        );
    }

    // 验证时间戳合理性（不超过5分钟）
    assert_reasonable_timestamp(ticker.timestamp, 300);

    let duration = start.elapsed().as_millis() as u64;
    print_test_summary("fetch_ticker", true, duration);
    Ok(())
}

/// Level 1: 基础连通性测试 - fetch_tickers（多个交易对）
#[tokio::test]
async fn test_fetch_tickers() -> anyhow::Result<()> {
    let start = std::time::Instant::now();
    print_test_separator("Binance: fetch_tickers");

    let config = init_test_config();
    if should_skip_integration_tests(&config) {
        println!("⏭ Skipping integration test");
        return Ok(());
    }

    let binance = create_binance(&config)?;

    // 确保市场数据已加载
    ensure_markets_loaded(&binance)
        .await
        .context("Failed to load markets")?;

    let symbols = get_test_symbols();

    print_test_step(&format!(
        "Fetching tickers for {} symbols...",
        symbols.len()
    ));
    let tickers = binance
        .fetch_tickers(Some(symbols.iter().map(|s| s.to_string()).collect()))
        .await
        .context("fetch_tickers should succeed")?;

    println!("  Retrieved {} tickers", tickers.len());
    assert!(
        tickers.len() >= symbols.len(),
        "Should return at least requested tickers"
    );

    // 验证每个请求的symbol都返回了数据
    for symbol in symbols {
        let ticker = tickers.iter().find(|t| t.symbol == *symbol);
        assert!(ticker.is_some(), "Should have ticker for {}", symbol);

        if let Some(t) = ticker {
            // assert_valid_ticker!(t, symbol);
            assert_eq!(t.symbol, symbol);
            println!("  {} - Last: {}", symbol, format_price_opt(t.last));
        }
    }

    let duration = start.elapsed().as_millis() as u64;
    print_test_summary("fetch_tickers", true, duration);
    Ok(())
}

/// Level 1: 基础连通性测试 - fetch_order_book
#[tokio::test]
async fn test_fetch_order_book() -> anyhow::Result<()> {
    let start = std::time::Instant::now();
    print_test_separator("Binance: fetch_order_book");

    let config = init_test_config();
    if should_skip_integration_tests(&config) {
        println!("⏭ Skipping integration test");
        return Ok(());
    }

    let binance = create_binance(&config)?;

    // 确保市场数据已加载
    ensure_markets_loaded(&binance)
        .await
        .context("Failed to load markets")?;

    let symbol = get_test_symbol();

    print_test_step(&format!("Fetching order book for {}...", symbol));
    let orderbook = binance
        .fetch_order_book(symbol, Some(20))
        .await
        .context("fetch_order_book should succeed")?;

    // 使用自定义断言宏验证orderbook数据（要求至少20档深度）
    // assert_valid_orderbook!(orderbook, symbol, 20);
    assert_eq!(orderbook.symbol, symbol);
    assert!(!orderbook.bids.is_empty());
    assert!(!orderbook.asks.is_empty());
    assert!(orderbook.bids.len() >= 20);
    assert!(orderbook.asks.len() >= 20);

    // 打印订单簿摘要
    use rust_decimal_macros::dec;
    let best_bid = orderbook.bids[0].price;
    let best_ask = orderbook.asks[0].price;
    let mid_price = (best_bid + best_ask) / dec!(2.0);
    let spread_bps = ((best_ask - best_bid) / mid_price) * dec!(10000.0);

    println!(
        "  Best Bid: {} ({} {})",
        format_price(best_bid),
        orderbook.bids[0].amount,
        symbol.split('/').next().unwrap_or("")
    );
    println!(
        "  Best Ask: {} ({} {})",
        format_price(best_ask),
        orderbook.asks[0].amount,
        symbol.split('/').next().unwrap_or("")
    );
    println!("  Mid Price: {}", format_price(mid_price));
    println!("  Spread: {:.2} bps", spread_bps);

    // 计算买卖盘深度
    use rust_decimal::prelude::*;
    let bid_depth: Decimal = orderbook
        .bids
        .iter()
        .take(10)
        .map(|l| {
            let amt: Decimal = l.amount.into();
            amt
        })
        .sum();
    let ask_depth: Decimal = orderbook
        .asks
        .iter()
        .take(10)
        .map(|l| {
            let amt: Decimal = l.amount.into();
            amt
        })
        .sum();
    println!("  Top 10 Bid Depth: {}", format_volume(bid_depth));
    println!("  Top 10 Ask Depth: {}", format_volume(ask_depth));

    // 验证时间戳
    assert_reasonable_timestamp(orderbook.timestamp, 300);

    let duration = start.elapsed().as_millis() as u64;
    print_test_summary("fetch_order_book", true, duration);
    Ok(())
}

/// Level 1: 基础连通性测试 - fetch_trades
#[tokio::test]
async fn test_fetch_trades() -> anyhow::Result<()> {
    let start = std::time::Instant::now();
    print_test_separator("Binance: fetch_trades");

    let config = init_test_config();
    if should_skip_integration_tests(&config) {
        println!("⏭ Skipping integration test");
        return Ok(());
    }

    let binance = create_binance(&config)?;

    // 确保市场数据已加载
    ensure_markets_loaded(&binance)
        .await
        .context("Failed to load markets")?;

    let symbol = get_test_symbol();

    print_test_step(&format!("Fetching recent trades for {}...", symbol));
    let trades = binance
        .fetch_trades(symbol, Some(50))
        .await
        .context("fetch_trades should succeed")?;

    println!("  Retrieved {} trades", trades.len());
    assert!(!trades.is_empty(), "Should return at least one trade");
    assert!(trades.len() <= 50, "Should not exceed requested limit");

    // 验证前几笔交易
    for (i, trade) in trades.iter().take(3).enumerate() {
        // assert_valid_trade!(trade, symbol);
        assert_eq!(trade.symbol, symbol);
        assert!(trade.timestamp > 0);
        if i == 0 {
            println!(
                "  Latest trade: {} {} @ {} ({})",
                trade.side,
                trade.amount,
                format_price(trade.price),
                trade.timestamp
            );
        }
    }

    // 验证交易按时间排序（最新的在前）
    for i in 1..trades.len().min(10) {
        assert!(
            trades[i - 1].timestamp >= trades[i].timestamp,
            "Trades should be sorted by timestamp (newest first)"
        );
    }

    // 统计买卖方向
    use ccxt_core::types::order::OrderSide;
    let buys = trades
        .iter()
        .filter(|t| matches!(t.side, OrderSide::Buy))
        .count();
    let sells = trades
        .iter()
        .filter(|t| matches!(t.side, OrderSide::Sell))
        .count();
    println!("  Buy/Sell: {} / {}", buys, sells);

    let duration = start.elapsed().as_millis() as u64;
    print_test_summary("fetch_trades", true, duration);
    Ok(())
}

/// Level 1: 基础连通性测试 - fetch_ohlcv
#[tokio::test]
async fn test_fetch_ohlcv() -> anyhow::Result<()> {
    let start = std::time::Instant::now();
    print_test_separator("Binance: fetch_ohlcv");

    let config = init_test_config();
    if should_skip_integration_tests(&config) {
        println!("⏭ Skipping integration test");
        return Ok(());
    }

    let binance = create_binance(&config)?;

    // 确保市场数据已加载
    ensure_markets_loaded(&binance)
        .await
        .context("Failed to load markets")?;

    let symbol = get_test_symbol();
    let timeframe = "1m";
    let limit = 100;

    print_test_step(&format!("Fetching OHLCV for {} ({})", symbol, timeframe));
    let ohlcv = binance
        .fetch_ohlcv(symbol, timeframe, None, Some(limit), None)
        .await
        .context("fetch_ohlcv should succeed")?;

    println!("  Retrieved {} candles", ohlcv.len());
    assert!(!ohlcv.is_empty(), "Should return at least one candle");
    assert!(
        ohlcv.len() <= limit as usize,
        "Should not exceed requested limit"
    );

    // 验证OHLCV数据结构
    use rust_decimal::prelude::*;
    for (i, candle) in ohlcv.iter().take(3).enumerate() {
        // OHLCV是结构体，有字段: timestamp, open, high, low, close, volume
        let timestamp = candle.timestamp;
        let open = candle.open;
        let high = candle.high;
        let low = candle.low;
        let close = candle.close;
        let volume = candle.volume;

        // 验证OHLC关系
        assert!(high >= open, "High should be >= open");
        assert!(high >= close, "High should be >= close");
        assert!(low <= open, "Low should be <= open");
        assert!(low <= close, "Low should be <= close");
        assert!(high >= low, "High should be >= low");
        assert!(volume >= 0.0, "Volume should be non-negative");

        if i == 0 {
            println!(
                "  Latest candle: O:{:.2} H:{:.2} L:{:.2} C:{:.2} V:{:.2}",
                open, high, low, close, volume
            );
        }

        // 验证时间戳合理性
        assert_reasonable_timestamp(timestamp, 86400); // 24小时内
    }

    // 验证时间序列按升序排列
    for i in 1..ohlcv.len().min(10) {
        let prev_ts = ohlcv[i - 1].timestamp;
        let curr_ts = ohlcv[i].timestamp;
        assert!(
            curr_ts >= prev_ts,
            "OHLCV should be sorted by timestamp (ascending)"
        );
    }

    let duration = start.elapsed().as_millis() as u64;
    print_test_summary("fetch_ohlcv", true, duration);
    Ok(())
}

/// Level 2: 错误处理测试 - 无效交易对
#[tokio::test]
async fn test_fetch_ticker_invalid_symbol() -> anyhow::Result<()> {
    let start = std::time::Instant::now();
    print_test_separator("Binance: fetch_ticker (invalid symbol)");

    let config = init_test_config();
    if should_skip_integration_tests(&config) {
        println!("⏭ Skipping integration test");
        return Ok(());
    }

    let binance = create_binance(&config)?;
    let invalid_symbol = "INVALID/SYMBOL";

    print_test_step(&format!(
        "Attempting to fetch ticker for invalid symbol: {}",
        invalid_symbol
    ));
    let result = binance
        .fetch_ticker(invalid_symbol, ccxt_core::types::TickerParams::default())
        .await;

    // 应该返回错误
    assert!(result.is_err(), "Should fail with invalid symbol");

    if let Err(e) = result {
        println!("  Expected error: {:?}", e);
        // 验证错误类型合理
        match e {
            ccxt_core::error::Error::MarketNotFound(_)
            | ccxt_core::error::Error::Exchange { .. } => {
                println!("  ✓ Error type is appropriate");
            }
            _ => panic!("Unexpected error type: {:?}", e),
        }
    }

    let duration = start.elapsed().as_millis() as u64;
    print_test_summary("fetch_ticker (invalid symbol)", true, duration);
    Ok(())
}

/// Level 2: 并发测试 - 同时获取多个ticker
#[tokio::test]
async fn test_concurrent_fetch_tickers() -> anyhow::Result<()> {
    let start = std::time::Instant::now();
    print_test_separator("Binance: Concurrent fetch_ticker");

    let config = init_test_config();
    if should_skip_integration_tests(&config) {
        println!("⏭ Skipping integration test");
        return Ok(());
    }

    let binance = std::sync::Arc::new(create_binance(&config)?);

    // 确保市场数据已加载
    ensure_markets_loaded(&binance)
        .await
        .context("Failed to load markets")?;

    let symbols = get_test_symbols();
    let symbol_count = symbols.len();

    print_test_step(&format!(
        "Fetching {} tickers concurrently...",
        symbol_count
    ));

    // 创建并发任务
    let mut tasks = vec![];
    for symbol in &symbols {
        let binance_clone = binance.clone();
        let symbol_owned = symbol.to_string();
        let task = tokio::spawn(async move {
            binance_clone
                .fetch_ticker(&symbol_owned, ccxt_core::types::TickerParams::default())
                .await
        });
        tasks.push((symbol, task));
    }

    // 等待所有任务完成
    let mut success_count = 0;
    for (symbol, task) in tasks {
        match task.await {
            Ok(Ok(ticker)) => {
                // assert_valid_ticker!(ticker, *symbol);
                assert_eq!(ticker.symbol, *symbol);
                println!("  ✓ {} - Last: {}", symbol, format_price_opt(ticker.last));
                success_count += 1;
            }
            Ok(Err(e)) => {
                println!("  ✗ {} failed: {:?}", symbol, e);
            }
            Err(e) => {
                println!("  ✗ {} task error: {:?}", symbol, e);
            }
        }
    }

    assert_eq!(
        success_count, symbol_count,
        "All concurrent requests should succeed"
    );

    let duration = start.elapsed().as_millis() as u64;
    print_test_summary("Concurrent fetch_ticker", true, duration);
    Ok(())
}

/// Level 2: 重试机制测试
#[tokio::test]
async fn test_fetch_with_retry() -> anyhow::Result<()> {
    let start = std::time::Instant::now();
    print_test_separator("Binance: fetch_ticker with retry");

    let config = init_test_config();
    if should_skip_integration_tests(&config) {
        println!("⏭ Skipping integration test");
        return Ok(());
    }

    let binance = create_binance(&config)?;

    // 确保市场数据已加载
    ensure_markets_loaded(&binance)
        .await
        .context("Failed to load markets")?;

    let symbol = get_test_symbol();

    print_test_step("Testing retry with backoff...");

    let ticker = retry_with_backoff(
        || async {
            binance
                .fetch_ticker(symbol, ccxt_core::types::TickerParams::default())
                .await
        },
        3,
        100,
    )
    .await
    .context("fetch_ticker with retry should succeed")?;

    // assert_valid_ticker!(ticker, symbol);
    assert_eq!(ticker.symbol, symbol);
    println!("  Successfully fetched ticker with retry mechanism");

    let duration = start.elapsed().as_millis() as u64;
    print_test_summary("fetch_ticker with retry", true, duration);
    Ok(())
}
