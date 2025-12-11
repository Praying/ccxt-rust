//! Stress test suite
//!
//! Tests concurrent performance, connection pool management, and memory leak detection.

use ccxt_core::ExchangeConfig;
use ccxt_exchanges::binance::Binance;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::task::JoinSet;

#[tokio::test]
#[ignore] // Requires network; run with: cargo test --test stress_test --ignored
async fn test_concurrent_ticker_fetches() {
    println!("🚀 Starting concurrent ticker test (100 concurrent requests)...");

    let config = ExchangeConfig::default();
    let exchange = Arc::new(Binance::new(config).unwrap());
    let symbols = vec![
        "BTC/USDT",
        "ETH/USDT",
        "BNB/USDT",
        "XRP/USDT",
        "ADA/USDT",
        "SOL/USDT",
        "DOT/USDT",
        "DOGE/USDT",
        "AVAX/USDT",
        "MATIC/USDT",
    ];

    let start = Instant::now();
    let mut tasks = JoinSet::new();

    // 100 concurrent tasks (10 per symbol)
    for i in 0..100 {
        let exchange_clone = Arc::clone(&exchange);
        let symbol = symbols[i % symbols.len()].to_string();

        tasks.spawn(async move {
            exchange_clone
                .fetch_ticker(&symbol, ccxt_core::types::TickerParams::default())
                .await
        });
    }

    let mut success_count = 0;
    let mut error_count = 0;

    while let Some(result) = tasks.join_next().await {
        match result {
            Ok(Ok(_)) => success_count += 1,
            Ok(Err(e)) => {
                error_count += 1;
                eprintln!("❌ Request failed: {:?}", e);
            }
            Err(e) => {
                error_count += 1;
                eprintln!("❌ Task failed: {:?}", e);
            }
        }
    }

    let duration = start.elapsed();

    println!("✅ Concurrent test completed:");
    println!("   - Total requests: 100");
    println!("   - Success: {}", success_count);
    println!("   - Failed: {}", error_count);
    println!("   - Total time: {:?}", duration);
    println!("   - Avg latency: {:?}", duration / 100);

    assert!(
        success_count >= 80,
        "Success rate too low: {}/100",
        success_count
    );
    assert!(
        duration < Duration::from_secs(5),
        "Avg latency too high: {:?}",
        duration
    );
}

#[tokio::test]
#[ignore]
async fn test_concurrent_orderbook_fetches() {
    println!("🚀 Starting concurrent orderbook test (50 concurrent requests)...");

    let config = ExchangeConfig::default();
    let exchange = Arc::new(Binance::new(config).unwrap());
    let symbols = vec!["BTC/USDT", "ETH/USDT", "BNB/USDT", "XRP/USDT", "ADA/USDT"];

    let start = Instant::now();
    let mut tasks = JoinSet::new();

    for i in 0..50 {
        let exchange_clone = Arc::clone(&exchange);
        let symbol = symbols[i % symbols.len()].to_string();

        tasks.spawn(async move { exchange_clone.fetch_order_book(&symbol, Some(10)).await });
    }

    let mut success_count = 0;
    let mut error_count = 0;

    while let Some(result) = tasks.join_next().await {
        match result {
            Ok(Ok(orderbook)) => {
                success_count += 1;
                assert!(
                    !orderbook.bids.is_empty(),
                    "Orderbook bids should not be empty"
                );
                assert!(
                    !orderbook.asks.is_empty(),
                    "Orderbook asks should not be empty"
                );
            }
            Ok(Err(e)) => {
                error_count += 1;
                eprintln!("❌ Request failed: {:?}", e);
            }
            Err(e) => {
                error_count += 1;
                eprintln!("❌ Task failed: {:?}", e);
            }
        }
    }

    let duration = start.elapsed();

    println!("✅ Concurrent orderbook test completed:");
    println!("   - Total requests: 50");
    println!("   - Success: {}", success_count);
    println!("   - Failed: {}", error_count);
    println!("   - Total time: {:?}", duration);
    println!("   - Avg latency: {:?}", duration / 50);

    assert!(
        success_count >= 40,
        "Success rate too low: {}/50",
        success_count
    );
}

#[tokio::test]
#[ignore]
async fn test_rate_limiter_enforcement() {
    println!("🚀 Testing rate limiter enforcement...");

    let config = ExchangeConfig::default();
    let exchange = Arc::new(Binance::new(config).unwrap());
    let start = Instant::now();

    // Rapidly send 20 consecutive requests
    let mut tasks = JoinSet::new();
    for i in 0..20 {
        let exchange_clone = Arc::clone(&exchange);
        tasks.spawn(async move {
            let request_start = Instant::now();
            let result = exchange_clone
                .fetch_ticker("BTC/USDT", ccxt_core::types::TickerParams::default())
                .await;
            (i, request_start.elapsed(), result.is_ok())
        });
    }

    let mut results = Vec::new();
    while let Some(result) = tasks.join_next().await {
        if let Ok(data) = result {
            results.push(data);
        }
    }

    let total_duration = start.elapsed();

    println!("✅ Rate limiting test completed:");
    println!("   - Total requests: 20");
    println!("   - Total time: {:?}", total_duration);

    // Binance weight limit ~1200/min; 20 requests should complete quickly
    assert!(
        total_duration < Duration::from_secs(10),
        "Rate limiting may be too strict"
    );

    let success_count = results.iter().filter(|(_, _, success)| *success).count();
    println!("   - Success rate: {}/20", success_count);
    assert!(
        success_count >= 18,
        "Success rate too low; rate limit may be triggered"
    );
}

#[tokio::test]
#[ignore]
async fn test_connection_pool() {
    println!("🚀 Testing connection pool management...");

    let config = ExchangeConfig::default();
    let exchange = Arc::new(Binance::new(config).unwrap());

    // First round: establish connections
    println!("   📝 First round (establishing connections)...");
    let start = Instant::now();
    let mut tasks = JoinSet::new();

    for _ in 0..10 {
        let exchange_clone = Arc::clone(&exchange);
        tasks.spawn(async move {
            exchange_clone
                .fetch_ticker("BTC/USDT", ccxt_core::types::TickerParams::default())
                .await
        });
    }

    let mut first_round_count = 0;
    while let Some(result) = tasks.join_next().await {
        if result.is_ok() {
            first_round_count += 1;
        }
    }
    let first_duration = start.elapsed();

    tokio::time::sleep(Duration::from_millis(100)).await;

    // Second round: reuse connections
    println!("   📝 Second round (reusing connections)...");
    let start = Instant::now();
    let mut tasks = JoinSet::new();

    for _ in 0..10 {
        let exchange_clone = Arc::clone(&exchange);
        tasks.spawn(async move {
            exchange_clone
                .fetch_ticker("ETH/USDT", ccxt_core::types::TickerParams::default())
                .await
        });
    }

    let mut second_round_count = 0;
    while let Some(result) = tasks.join_next().await {
        if result.is_ok() {
            second_round_count += 1;
        }
    }
    let second_duration = start.elapsed();

    println!("✅ Connection pool test completed:");
    println!(
        "   - First round: {} success, time {:?}",
        first_round_count, first_duration
    );
    println!(
        "   - Second round: {} success, time {:?}",
        second_round_count, second_duration
    );

    assert!(first_round_count >= 8, "First round success rate too low");
    assert!(second_round_count >= 8, "Second round success rate too low");

    // Second round typically faster (connections established)
    // Note: this may vary due to network conditions
    println!(
        "   - Speed improvement: {:?}",
        first_duration.saturating_sub(second_duration)
    );
}

#[tokio::test]
#[ignore]
async fn test_memory_leak() {
    println!("🚀 Starting memory leak detection (1000 iterations)...");

    let config = ExchangeConfig::default();
    let exchange = Arc::new(Binance::new(config).unwrap());

    // Get initial memory info (if available)
    #[cfg(target_os = "linux")]
    fn get_memory_usage() -> Option<usize> {
        use std::fs;
        let status = fs::read_to_string("/proc/self/status").ok()?;
        for line in status.lines() {
            if line.starts_with("VmRSS:") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                return parts.get(1)?.parse().ok();
            }
        }
        None
    }

    #[cfg(not(target_os = "linux"))]
    fn get_memory_usage() -> Option<usize> {
        None
    }

    let initial_memory = get_memory_usage();
    let start = Instant::now();

    // Execute 1000 requests
    for i in 0..1000 {
        let result = exchange
            .fetch_ticker("BTC/USDT", ccxt_core::types::TickerParams::default())
            .await;

        if result.is_err() && i % 100 == 0 {
            eprintln!("⚠️  Iteration {} failed", i);
        }

        // Report progress every 100 iterations
        if i % 100 == 0 && i > 0 {
            let progress = (i as f64 / 1000.0) * 100.0;
            println!("   📊 Progress: {:.1}% ({}/1000)", progress, i);
        }

        // Small delay to avoid excessive request rate
        if i % 10 == 0 {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }

    let duration = start.elapsed();
    let final_memory = get_memory_usage();

    println!("✅ Memory leak detection completed:");
    println!("   - Iterations: 1000");
    println!("   - Total time: {:?}", duration);
    println!("   - Avg latency: {:?}", duration / 1000);

    if let (Some(initial), Some(final_mem)) = (initial_memory, final_memory) {
        let memory_increase = final_mem.saturating_sub(initial);
        let memory_increase_mb = memory_increase as f64 / 1024.0;
        println!("   - Memory growth: {:.2} MB", memory_increase_mb);

        assert!(
            memory_increase_mb < 50.0,
            "Memory leak detected: excessive growth ({:.2} MB)",
            memory_increase_mb
        );
    } else {
        println!("   ℹ️  Memory stats not available on current platform");
    }

    assert!(
        duration < Duration::from_secs(120),
        "Execution time too long: {:?}",
        duration
    );
}

#[tokio::test]
#[ignore]
async fn test_mixed_load() {
    println!("🚀 Starting mixed load test...");

    let config = ExchangeConfig::default();
    let exchange = Arc::new(Binance::new(config).unwrap());
    let start = Instant::now();
    let mut tasks = JoinSet::new();

    // 20 ticker requests
    for _ in 0..20 {
        let exchange_clone = Arc::clone(&exchange);
        tasks.spawn(async move {
            exchange_clone
                .fetch_ticker("BTC/USDT", ccxt_core::types::TickerParams::default())
                .await
                .map(|_| "ticker")
        });
    }

    // 10 orderbook requests
    for _ in 0..10 {
        let exchange_clone = Arc::clone(&exchange);
        tasks.spawn(async move {
            exchange_clone
                .fetch_order_book("ETH/USDT", Some(20))
                .await
                .map(|_| "orderbook")
        });
    }

    // 10 trade history requests
    for _ in 0..10 {
        let exchange_clone = Arc::clone(&exchange);
        tasks.spawn(async move {
            exchange_clone
                .fetch_trades("BNB/USDT", None)
                .await
                .map(|_| "trades")
        });
    }

    let mut results = std::collections::HashMap::new();
    results.insert("ticker", 0);
    results.insert("orderbook", 0);
    results.insert("trades", 0);

    while let Some(result) = tasks.join_next().await {
        if let Ok(Ok(op_type)) = result {
            *results.get_mut(op_type).unwrap() += 1;
        }
    }

    let duration = start.elapsed();

    println!("✅ Mixed load test completed:");
    println!("   - Ticker success: {}/20", results["ticker"]);
    println!("   - OrderBook success: {}/10", results["orderbook"]);
    println!("   - Trades success: {}/10", results["trades"]);
    println!("   - Total time: {:?}", duration);

    assert!(results["ticker"] >= 16, "Ticker success rate too low");
    assert!(results["orderbook"] >= 8, "OrderBook success rate too low");
    assert!(results["trades"] >= 8, "Trades success rate too low");
}
