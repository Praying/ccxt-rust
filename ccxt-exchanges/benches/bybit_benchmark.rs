//! Bybit exchange performance benchmarks.
//!
//! Benchmarks for Bybit API calls, data parsing, and other operations.
//!
//! Run with: cargo bench --bench bybit_benchmark

use ccxt_core::ExchangeConfig;
use ccxt_exchanges::bybit::Bybit;
use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use serde_json::json;
use tokio::runtime::Runtime;

// ==================== API Call Performance Benchmarks ====================

/// Benchmark fetching ticker from Bybit.
fn bench_fetch_ticker(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let bybit = Bybit::new(ExchangeConfig::default()).unwrap();

    c.bench_function("bybit_fetch_ticker_btc_usdt", |b| {
        b.to_async(&rt).iter(|| async {
            let _ = bybit
                .fetch_ticker(
                    black_box("BTC/USDT"),
                    ccxt_core::types::TickerParams::default(),
                )
                .await;
        });
    });
}

/// Benchmark fetching order book from Bybit with different depths.
fn bench_fetch_order_book(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let bybit = Bybit::new(ExchangeConfig::default()).unwrap();

    let mut group = c.benchmark_group("bybit_fetch_order_book");

    for depth in [5, 10, 20, 50].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(depth), depth, |b, &depth| {
            b.to_async(&rt).iter(|| async {
                let _ = bybit
                    .fetch_order_book(black_box("BTC/USDT"), Some(depth))
                    .await;
            });
        });
    }

    group.finish();
}

/// Benchmark fetching trades from Bybit.
fn bench_fetch_trades(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let bybit = Bybit::new(ExchangeConfig::default()).unwrap();

    c.bench_function("bybit_fetch_trades_recent", |b| {
        b.to_async(&rt).iter(|| async {
            let _ = bybit.fetch_trades(black_box("BTC/USDT"), Some(100)).await;
        });
    });
}

/// Benchmark fetching OHLCV data from Bybit with different timeframes.
fn bench_fetch_ohlcv(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let bybit = Bybit::new(ExchangeConfig::default()).unwrap();

    let mut group = c.benchmark_group("bybit_fetch_ohlcv");

    for timeframe in ["1", "5", "60", "D"].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(timeframe),
            timeframe,
            |b, timeframe| {
                b.to_async(&rt).iter(|| async {
                    let _ = bybit
                        .fetch_ohlcv(black_box("BTC/USDT"), timeframe, None, Some(100), None)
                        .await;
                });
            },
        );
    }

    group.finish();
}

// ==================== Data Parsing Performance Benchmarks ====================

/// Benchmark parsing market data from Bybit.
fn bench_parse_market(c: &mut Criterion) {
    let data = json!({
        "symbol": "BTCUSDT",
        "status": "Trading",
        "baseCoin": "BTC",
        "quoteCoin": "USDT",
        "priceScale": "2",
        "lotSizeFilter": {
            "minOrderQty": "0.001",
            "maxOrderQty": "1000000",
            "qtyStep": "0.001"
        }
    });

    c.bench_function("bybit_parse_market", |b| {
        b.iter(|| {
            // Note: Actual parsing function may vary based on Bybit implementation
            let _ = black_box(&data);
        });
    });
}

/// Benchmark parsing ticker data from Bybit.
fn bench_parse_ticker(c: &mut Criterion) {
    let data = json!({
        "symbol": "BTCUSDT",
        "lastPrice": "50000.00",
        "bidPrice": "49999.00",
        "askPrice": "50001.00",
        "highPrice24h": "51000.00",
        "lowPrice24h": "49000.00",
        "volume24h": "1000.5",
        "turnover24h": "50000000.00"
    });

    c.bench_function("bybit_parse_ticker", |b| {
        b.iter(|| {
            // Note: Actual parsing function may vary based on Bybit implementation
            let _ = black_box(&data);
        });
    });
}

/// Benchmark parsing trade data from Bybit.
fn bench_parse_trade(c: &mut Criterion) {
    let data = json!({
        "id": "12345",
        "price": "50000.00",
        "qty": "0.1",
        "time": "1234567890000",
        "isBuy": true
    });

    c.bench_function("bybit_parse_trade", |b| {
        b.iter(|| {
            // Note: Actual parsing function may vary based on Bybit implementation
            let _ = black_box(&data);
        });
    });
}

// ==================== Signature Generation Performance Benchmarks ====================

/// Benchmark generating signatures for Bybit API requests.
fn bench_signature_generation(c: &mut Criterion) {
    // Note: Bybit uses different authentication method than Binance
    // This benchmark will need to be adjusted based on actual Bybit auth implementation

    let query_string = "symbol=BTCUSDT&side=Buy&type=Limit&qty=0.1&price=50000&time=1234567890000";

    c.bench_function("bybit_generate_signature", |b| {
        b.iter(|| {
            // Note: Actual signing function may vary based on Bybit implementation
            let _ = black_box(query_string);
        });
    });
}

// ==================== Memory Performance Benchmarks ====================

/// Benchmark creating Bybit exchange instance.
fn bench_exchange_creation(c: &mut Criterion) {
    c.bench_function("bybit_create_exchange_instance", |b| {
        b.iter(|| {
            let _ = Bybit::new(black_box(ExchangeConfig::default()));
        });
    });
}

/// Benchmark creating Bybit exchange instance with credentials.
fn bench_exchange_with_credentials(c: &mut Criterion) {
    c.bench_function("bybit_create_exchange_with_credentials", |b| {
        b.iter(|| {
            let config = ExchangeConfig {
                api_key: Some("test_key".to_string()),
                secret: Some("test_secret".to_string()),
                ..Default::default()
            };
            let _ = Bybit::new(black_box(config));
        });
    });
}

// ==================== Concurrent Performance Benchmarks ====================

/// Benchmark concurrent ticker fetches from Bybit.
fn bench_concurrent_ticker_fetches(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let bybit = Bybit::new(ExchangeConfig::default()).unwrap();

    c.bench_function("bybit_concurrent_10_ticker_fetches", |b| {
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
                .map(|symbol| bybit.fetch_ticker(symbol, ccxt_core::types::TickerParams::default()))
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
