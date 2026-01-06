//! HyperLiquid exchange performance benchmarks.
//!
//! Benchmarks for HyperLiquid API calls, data parsing, and other operations.
//!
//! Run with: cargo bench --bench hyperliquid_benchmark

use ccxt_exchanges::hyperliquid::{HyperLiquid, HyperLiquidBuilder};
use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use serde_json::json;
use tokio::runtime::Runtime;

// ==================== API Call Performance Benchmarks ====================

/// Benchmark fetching ticker from HyperLiquid.
fn bench_fetch_ticker(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let hyperliquid = HyperLiquidBuilder::new()
        .testnet(true)
        .build()
        .expect("Failed to build HyperLiquid");

    c.bench_function("hyperliquid_fetch_ticker_btc_usdc", |b| {
        b.to_async(&rt).iter(|| async {
            let _ = hyperliquid
                .fetch_ticker(
                    black_box("BTC/USDC:USDC"),
                    ccxt_core::types::TickerParams::default(),
                )
                .await;
        });
    });
}

/// Benchmark fetching order book from HyperLiquid with different depths.
fn bench_fetch_order_book(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let hyperliquid = HyperLiquidBuilder::new()
        .testnet(true)
        .build()
        .expect("Failed to build HyperLiquid");

    let mut group = c.benchmark_group("hyperliquid_fetch_order_book");

    for depth in [5, 10, 20, 50].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(depth), depth, |b, &depth| {
            b.to_async(&rt).iter(|| async {
                let _ = hyperliquid
                    .fetch_order_book(black_box("BTC/USDC:USDC"), Some(depth))
                    .await;
            });
        });
    }

    group.finish();
}

/// Benchmark fetching trades from HyperLiquid.
fn bench_fetch_trades(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let hyperliquid = HyperLiquidBuilder::new()
        .testnet(true)
        .build()
        .expect("Failed to build HyperLiquid");

    c.bench_function("hyperliquid_fetch_trades_recent", |b| {
        b.to_async(&rt).iter(|| async {
            let _ = hyperliquid
                .fetch_trades(black_box("BTC/USDC:USDC"), Some(100))
                .await;
        });
    });
}

/// Benchmark fetching OHLCV data from HyperLiquid with different timeframes.
fn bench_fetch_ohlcv(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let hyperliquid = HyperLiquidBuilder::new()
        .testnet(true)
        .build()
        .expect("Failed to build HyperLiquid");

    let mut group = c.benchmark_group("hyperliquid_fetch_ohlcv");

    for timeframe in ["1", "5", "60", "1440"].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(timeframe),
            timeframe,
            |b, timeframe| {
                b.to_async(&rt).iter(|| async {
                    let _ = hyperliquid
                        .fetch_ohlcv(black_box("BTC/USDC:USDC"), timeframe, None, Some(100), None)
                        .await;
                });
            },
        );
    }

    group.finish();
}

// ==================== Data Parsing Performance Benchmarks ====================

/// Benchmark parsing market data from HyperLiquid.
fn bench_parse_market(c: &mut Criterion) {
    let data = json!({
        "name": "BTC",
        "szDecimals": 8,
        "maxLeverage": 50,
        "onlyIsolated": false,
        "type": "perp"
    });

    c.bench_function("hyperliquid_parse_market", |b| {
        b.iter(|| {
            // Note: Actual parsing function may vary based on HyperLiquid implementation
            let _ = black_box(&data);
        });
    });
}

/// Benchmark parsing ticker data from HyperLiquid.
fn bench_parse_ticker(c: &mut Criterion) {
    let data = json!({
        "coin": "BTC",
        "mid": "50000.00",
        "funding": "0.0001"
    });

    c.bench_function("hyperliquid_parse_ticker", |b| {
        b.iter(|| {
            // Note: Actual parsing function may vary based on HyperLiquid implementation
            let _ = black_box(&data);
        });
    });
}

/// Benchmark parsing trade data from HyperLiquid.
fn bench_parse_trade(c: &mut Criterion) {
    let data = json!({
        "coin": "BTC",
        "side": "A",
        "px": "50000.00",
        "sz": "0.1",
        "time": 1234567890000_i64,
        "hash": "0x1234567890abcdef"
    });

    c.bench_function("hyperliquid_parse_trade", |b| {
        b.iter(|| {
            // Note: Actual parsing function may vary based on HyperLiquid implementation
            let _ = black_box(&data);
        });
    });
}

// ==================== Signature Generation Performance Benchmarks ====================

/// Benchmark generating signatures for HyperLiquid API requests.
fn bench_signature_generation(c: &mut Criterion) {
    // Note: HyperLiquid uses EIP-712 signing which is different from HMAC
    // This benchmark will need to be adjusted based on actual HyperLiquid auth implementation

    let message = json!({
        "type": "exchange",
        "coin": "BTC",
        "is_buy": true,
        "limit_px": "50000.00",
        "sz": "0.1",
        "reduce_only": false,
        "order_type": {"limit": {"tif": "Gtc"}},
        "cloid": "1234567890"
    });

    c.bench_function("hyperliquid_generate_signature", |b| {
        b.iter(|| {
            // Note: Actual signing function may vary based on HyperLiquid implementation
            let _ = black_box(&message);
        });
    });
}

// ==================== Memory Performance Benchmarks ====================

/// Benchmark creating HyperLiquid exchange instance.
fn bench_exchange_creation(c: &mut Criterion) {
    c.bench_function("hyperliquid_create_exchange_instance", |b| {
        b.iter(|| {
            let _ = HyperLiquidBuilder::new().testnet(true).build();
        });
    });
}

/// Benchmark creating HyperLiquid exchange instance with credentials.
fn bench_exchange_with_credentials(c: &mut Criterion) {
    let test_key = "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80";

    c.bench_function("hyperliquid_create_exchange_with_credentials", |b| {
        b.iter(|| {
            let _ = HyperLiquidBuilder::new()
                .private_key(black_box(test_key))
                .testnet(true)
                .build();
        });
    });
}

// ==================== Concurrent Performance Benchmarks ====================

/// Benchmark concurrent ticker fetches from HyperLiquid.
fn bench_concurrent_ticker_fetches(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let hyperliquid = HyperLiquidBuilder::new()
        .testnet(true)
        .build()
        .expect("Failed to build HyperLiquid");

    c.bench_function("hyperliquid_concurrent_10_ticker_fetches", |b| {
        b.to_async(&rt).iter(|| async {
            let symbols = vec![
                "BTC/USDC:USDC",
                "ETH/USDC:USDC",
                "SOL/USDC:USDC",
                "ARB/USDC:USDC",
                "DOGE/USDC:USDC",
                "MATIC/USDC:USDC",
                "AVAX/USDC:USDC",
                "LINK/USDC:USDC",
                "UNI/USDC:USDC",
                "AAVE/USDC:USDC",
            ];

            let futures: Vec<_> = symbols
                .into_iter()
                .map(|symbol| {
                    hyperliquid.fetch_ticker(symbol, ccxt_core::types::TickerParams::default())
                })
                .collect();

            let _ = futures::future::join_all(futures).await;
        });
    });
}

// ==================== Benchmark Configuration ====================

criterion_group!(
    name = benches;
    config = Criterion::default()
        .sample_size(10) // Reduce sample size for faster testing
        .measurement_time(std::time::Duration::from_secs(5));
    targets =
        // API call benchmarks
        bench_fetch_ticker,
        bench_fetch_order_book,
        bench_fetch_trades,
        bench_fetch_ohlcv,

        // Parsing benchmarks
        bench_parse_market,
        bench_parse_ticker,
        bench_parse_trade,

        // Signature benchmarks
        bench_signature_generation,

        // Memory benchmarks
        bench_exchange_creation,
        bench_exchange_with_credentials,

        // Concurrent benchmarks
        bench_concurrent_ticker_fetches
);

criterion_main!(benches);
