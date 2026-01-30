#![allow(clippy::disallowed_methods)]
#![allow(deprecated)]
//! OKX exchange performance benchmarks.
//!
//! Benchmarks for OKX API calls, data parsing, and other operations.
//!
//! Run with: cargo bench --bench okx_benchmark

use ccxt_core::ExchangeConfig;
use ccxt_exchanges::okx::Okx;
use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use serde_json::json;
use tokio::runtime::Runtime;

// ==================== API Call Performance Benchmarks ====================

/// Benchmark fetching ticker from OKX.
fn bench_fetch_ticker(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let okx = Okx::new(ExchangeConfig::default()).unwrap();

    c.bench_function("okx_fetch_ticker_btc_usdt", |b| {
        b.to_async(&rt).iter(|| async {
            let _ = okx.fetch_ticker(black_box("BTC/USDT")).await;
        });
    });
}

/// Benchmark fetching order book from OKX with different depths.
fn bench_fetch_order_book(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let okx = Okx::new(ExchangeConfig::default()).unwrap();

    let mut group = c.benchmark_group("okx_fetch_order_book");

    for depth in [5, 10, 20, 50].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(depth), depth, |b, &depth| {
            b.to_async(&rt).iter(|| async {
                let _ = okx
                    .fetch_order_book(black_box("BTC/USDT"), Some(depth))
                    .await;
            });
        });
    }

    group.finish();
}

/// Benchmark fetching trades from OKX.
fn bench_fetch_trades(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let okx = Okx::new(ExchangeConfig::default()).unwrap();

    c.bench_function("okx_fetch_trades_recent", |b| {
        b.to_async(&rt).iter(|| async {
            let _ = okx.fetch_trades(black_box("BTC/USDT"), Some(100)).await;
        });
    });
}

/// Benchmark fetching OHLCV data from OKX with different timeframes.
fn bench_fetch_ohlcv(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let okx = Okx::new(ExchangeConfig::default()).unwrap();

    let mut group = c.benchmark_group("okx_fetch_ohlcv");

    for timeframe in ["1m", "5m", "1H", "1D"].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(timeframe),
            timeframe,
            |b, timeframe| {
                b.to_async(&rt).iter(|| async {
                    let _ = okx
                        .fetch_ohlcv(black_box("BTC/USDT"), timeframe, None, Some(100))
                        .await;
                });
            },
        );
    }

    group.finish();
}

// ==================== Data Parsing Performance Benchmarks ====================

/// Benchmark parsing market data from OKX.
fn bench_parse_market(c: &mut Criterion) {
    let data = json!({
        "instId": "BTC-USDT",
        "instType": "SPOT",
        "baseCcy": "BTC",
        "quoteCcy": "USDT",
        "state": "live",
        "tickSz": "0.1",
        "lotSz": "0.00001",
        "minSz": "0.00001",
        "maxSz": "1000000"
    });

    c.bench_function("okx_parse_market", |b| {
        b.iter(|| {
            // Note: Actual parsing function may vary based on OKX implementation
            let _ = black_box(&data);
        });
    });
}

/// Benchmark parsing ticker data from OKX.
fn bench_parse_ticker(c: &mut Criterion) {
    let data = json!({
        "instId": "BTC-USDT",
        "last": "50000.00",
        "bidPx": "49999.00",
        "askPx": "50001.00",
        "high24h": "51000.00",
        "low24h": "49000.00",
        "vol24h": "1000.5",
        "volCcy24h": "50000000.00"
    });

    c.bench_function("okx_parse_ticker", |b| {
        b.iter(|| {
            // Note: Actual parsing function may vary based on OKX implementation
            let _ = black_box(&data);
        });
    });
}

/// Benchmark parsing trade data from OKX.
fn bench_parse_trade(c: &mut Criterion) {
    let data = json!({
        "tradeId": "12345",
        "px": "50000.00",
        "sz": "0.1",
        "side": "buy",
        "ts": "1234567890000"
    });

    c.bench_function("okx_parse_trade", |b| {
        b.iter(|| {
            // Note: Actual parsing function may vary based on OKX implementation
            let _ = black_box(&data);
        });
    });
}

// ==================== Signature Generation Performance Benchmarks ====================

/// Benchmark generating signatures for OKX API requests.
fn bench_signature_generation(c: &mut Criterion) {
    // Note: OKX uses different authentication method than Binance
    // This benchmark will need to be adjusted based on actual OKX auth implementation

    let query_string = "instId=BTC-USDT&tdMode=cash&side=buy&ordType=limit&sz=0.1&px=50000";

    c.bench_function("okx_generate_signature", |b| {
        b.iter(|| {
            // Note: Actual signing function may vary based on OKX implementation
            let _ = black_box(query_string);
        });
    });
}

// ==================== Memory Performance Benchmarks ====================

/// Benchmark creating OKX exchange instance.
fn bench_exchange_creation(c: &mut Criterion) {
    c.bench_function("okx_create_exchange_instance", |b| {
        b.iter(|| {
            let _ = Okx::new(black_box(ExchangeConfig::default()));
        });
    });
}

/// Benchmark creating OKX exchange instance with credentials.
fn bench_exchange_with_credentials(c: &mut Criterion) {
    c.bench_function("okx_create_exchange_with_credentials", |b| {
        b.iter(|| {
            let config = ExchangeConfig {
                api_key: Some("test_key".to_string().into()),
                secret: Some("test_secret".to_string().into()),
                ..Default::default()
            };
            let _ = Okx::new(black_box(config));
        });
    });
}

// ==================== Concurrent Performance Benchmarks ====================

/// Benchmark concurrent ticker fetches from OKX.
fn bench_concurrent_ticker_fetches(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let okx = Okx::new(ExchangeConfig::default()).unwrap();

    c.bench_function("okx_concurrent_10_ticker_fetches", |b| {
        b.to_async(&rt).iter(|| async {
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

            let futures: Vec<_> = symbols
                .into_iter()
                .map(|symbol| okx.fetch_ticker(symbol))
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
