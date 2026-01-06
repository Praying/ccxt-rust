//! 极端情况测试
//!
//! 测试系统在极端情况下的行为，包括网络故障、API限流、大数据量等。
//!
//! 运行测试：
//! ```bash
//! cargo test --test extreme_case_tests
//! ```

use ccxt_core::ExchangeConfig;
use ccxt_exchanges::binance::Binance;
use std::sync::Arc;
use tokio::time::{Duration, timeout};

/// 测试网络故障处理
#[tokio::test]
#[ignore = "需要网络访问"]
async fn test_network_timeout_handling() {
    let exchange =
        Binance::new(ExchangeConfig::default()).expect("Failed to create Binance exchange");

    // 设置很短的超时时间
    let result = timeout(Duration::from_millis(100), async {
        exchange
            .fetch_ticker("BTC/USDT", ccxt_core::types::TickerParams::default())
            .await
    })
    .await;

    // 应该返回超时错误
    assert!(result.is_err(), "Should timeout on slow network");
    println!("✅ 网络超时处理正确");
}

/// 测试API限流处理
#[tokio::test]
#[ignore = "需要网络访问"]
async fn test_rate_limit_handling() {
    let exchange =
        Binance::new(ExchangeConfig::default()).expect("Failed to create Binance exchange");

    // 快速发送多个请求以触发限流
    let symbols = vec![
        "BTC/USDT",
        "ETH/USDT",
        "BNB/USDT",
        "ADA/USDT",
        "DOT/USDT",
        "XRP/USDT",
        "LTC/USDT",
        "LINK/USDT",
        "BCH/USDT",
        "UNI/USDT",
    ];

    let mut rate_limit_errors = 0;
    for symbol in symbols {
        let result = exchange
            .fetch_ticker(symbol, ccxt_core::types::TickerParams::default())
            .await;
        if result.is_err() {
            rate_limit_errors += 1;
        }
    }

    // 应该有一些限流错误
    assert!(rate_limit_errors > 0, "Should encounter rate limiting");
    println!("✅ API限流处理正确，限流错误数：{}", rate_limit_errors);
}

/// 测试大数据量处理
#[tokio::test]
#[ignore = "需要网络访问"]
async fn test_large_order_book_handling() {
    let exchange =
        Binance::new(ExchangeConfig::default()).expect("Failed to create Binance exchange");

    // 请求深度订单簿
    let orderbook = exchange
        .fetch_order_book("BTC/USDT", Some(1000))
        .await
        .expect("Failed to fetch order book");

    // 应该返回深度订单簿
    assert!(!orderbook.bids.is_empty(), "Should have bids");
    assert!(!orderbook.asks.is_empty(), "Should have asks");
    assert!(
        orderbook.bids.len() >= 1000,
        "Should have at least 1000 bids"
    );
    assert!(
        orderbook.asks.len() >= 1000,
        "Should have at least 1000 asks"
    );
    println!(
        "✅ 大数据量处理正确，订单簿深度：{} bids, {} asks",
        orderbook.bids.len(),
        orderbook.asks.len()
    );
}

/// 测试无效输入处理
#[tokio::test]
#[ignore = "需要网络访问"]
async fn test_invalid_symbol_handling() {
    let exchange =
        Binance::new(ExchangeConfig::default()).expect("Failed to create Binance exchange");

    // 测试无效的交易对
    let invalid_symbols = vec!["", "INVALID", "BTC", "BTC/USDT/ETH", "BTC/USDT/INVALID"];

    for symbol in invalid_symbols {
        let result = exchange
            .fetch_ticker(symbol, ccxt_core::types::TickerParams::default())
            .await;
        assert!(
            result.is_err(),
            "Should return error for invalid symbol: {}",
            symbol
        );
    }

    println!("✅ 无效输入处理正确，所有无效符号都被正确拒绝");
}

/// 测试并发请求处理
#[tokio::test]
#[ignore = "需要网络访问"]
async fn test_concurrent_request_handling() {
    let exchange = Arc::new(
        Binance::new(ExchangeConfig::default()).expect("Failed to create Binance exchange"),
    );

    // 并发发送大量请求
    let symbols = vec![
        "BTC/USDT",
        "ETH/USDT",
        "BNB/USDT",
        "ADA/USDT",
        "DOT/USDT",
        "XRP/USDT",
        "LTC/USDT",
        "LINK/USDT",
        "BCH/USDT",
        "UNI/USDT",
        "SOL/USDT",
        "MATIC/USDT",
        "AVAX/USDT",
        "AAVE/USDT",
        "ATOM/USDT",
        "DOT/USDT",
        "NEAR/USDT",
        "FIL/USDT",
    ];
    let total_symbols = symbols.len();

    let futures: Vec<_> = symbols
        .into_iter()
        .map(|symbol| {
            let exchange = Arc::clone(&exchange);
            async move {
                exchange
                    .fetch_ticker(symbol, ccxt_core::types::TickerParams::default())
                    .await
            }
        })
        .collect();

    let results: Vec<Result<ccxt_core::Ticker, ccxt_core::Error>> =
        futures::future::join_all(futures).await;

    // 应该有一些成功的结果
    let success_count = results.iter().filter(|r| r.is_ok()).count();
    assert!(success_count > 0, "Should have some successful requests");
    assert!(
        success_count < total_symbols,
        "Not all requests should succeed"
    );
    println!(
        "✅ 并发请求处理正确，成功数：{}/{}",
        success_count, total_symbols
    );
}

/// 测试内存压力测试
#[tokio::test]
#[ignore = "需要网络访问"]
async fn test_memory_pressure_handling() {
    // 创建多个exchange实例
    let exchanges: Vec<Arc<Binance>> = (0..10)
        .map(|_| {
            Arc::new(
                Binance::new(ExchangeConfig::default()).expect("Failed to create Binance exchange"),
            )
        })
        .collect();

    // 并发获取ticker
    let futures: Vec<_> = exchanges
        .iter()
        .map(|exchange| {
            let exchange = Arc::clone(exchange);
            async move {
                exchange
                    .fetch_ticker("BTC/USDT", ccxt_core::types::TickerParams::default())
                    .await
            }
        })
        .collect();

    let results: Vec<Result<ccxt_core::Ticker, ccxt_core::Error>> =
        futures::future::join_all(futures).await;

    // 应该有一些成功的结果
    let success_count = results.iter().filter(|r| r.is_ok()).count();
    assert!(success_count > 0, "Should have some successful requests");
    println!(
        "✅ 内存压力测试通过，成功数：{}/{}",
        success_count,
        results.len()
    );
}

/// 测试错误恢复
#[tokio::test]
#[ignore = "需要网络访问"]
async fn test_error_recovery() {
    let exchange =
        Binance::new(ExchangeConfig::default()).expect("Failed to create Binance exchange");

    // 发送多个请求，其中一些会失败
    let symbols = vec![
        "BTC/USDT",
        "INVALID_SYMBOL_1",
        "ETH/USDT",
        "INVALID_SYMBOL_2",
        "BNB/USDT",
    ];

    let mut success_count = 0;
    let mut error_count = 0;

    for symbol in symbols {
        let result = exchange
            .fetch_ticker(symbol, ccxt_core::types::TickerParams::default())
            .await;
        if result.is_ok() {
            success_count += 1;
        } else {
            error_count += 1;
        }
    }

    // 应该有一些成功的结果
    assert!(success_count > 0, "Should have some successful requests");
    assert!(error_count > 0, "Should have some failed requests");
    println!(
        "✅ 错误恢复测试通过，成功数：{}，错误数：{}",
        success_count, error_count
    );
}

/// 测试数据一致性
#[tokio::test]
#[ignore = "需要网络访问"]
async fn test_data_consistency() {
    let exchange =
        Binance::new(ExchangeConfig::default()).expect("Failed to create Binance exchange");

    // 多次获取同一个ticker，验证数据一致性
    let ticker1 = exchange
        .fetch_ticker("BTC/USDT", ccxt_core::types::TickerParams::default())
        .await
        .expect("Failed to fetch ticker 1");
    let ticker2 = exchange
        .fetch_ticker("BTC/USDT", ccxt_core::types::TickerParams::default())
        .await
        .expect("Failed to fetch ticker 2");
    let ticker3 = exchange
        .fetch_ticker("BTC/USDT", ccxt_core::types::TickerParams::default())
        .await
        .expect("Failed to fetch ticker 3");

    // 验证数据一致性
    assert_eq!(
        ticker1.symbol, ticker2.symbol,
        "Symbol should be consistent"
    );
    assert_eq!(
        ticker2.symbol, ticker3.symbol,
        "Symbol should be consistent"
    );

    let (last1, last2, last3) = (ticker1.last, ticker2.last, ticker3.last);
    // 价格可能会有变化，但应该比较接近
    if let (Some(price1), Some(price2), Some(price3)) = (last1, last2, last3) {
        let max_price = price1.max(price2).max(price3);
        let min_price = price1.min(price2).min(price3);
        let max_diff = max_price - min_price;
        let percent_diff = (max_diff / min_price) * rust_decimal_macros::dec!(100);

        // 价格差异不应该太大
        assert!(
            percent_diff < rust_decimal_macros::dec!(5),
            "Price should be consistent (diff < 5%)"
        );
    }

    println!("✅ 数据一致性测试通过");
}

/// 测试边界值处理
#[tokio::test]
#[ignore = "需要网络访问"]
async fn test_boundary_value_handling() {
    let exchange =
        Binance::new(ExchangeConfig::default()).expect("Failed to create Binance exchange");

    // 测试边界值
    let limit_cases = vec![
        (Some(0), "zero limit"),
        (Some(1), "one limit"),
        (Some(5), "small limit"),
        (Some(100), "normal limit"),
        (Some(1000), "large limit"),
    ];

    for (limit, _description) in limit_cases {
        let result = exchange.fetch_trades("BTC/USDT", limit).await;

        if limit == Some(0) {
            assert!(result.is_err(), "Should return error for zero limit");
        } else if let Ok(trades) = result
            && let Some(limit) = limit
        {
            assert!(trades.len() <= limit as usize, "Should not exceed limit");
        }
    }

    println!("✅ 边界值处理测试通过");
}
