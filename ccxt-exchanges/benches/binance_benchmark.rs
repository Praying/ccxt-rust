use ccxt_core::ExchangeConfig;
use ccxt_exchanges::binance::Binance;
use ccxt_exchanges::binance::parser;
use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use serde_json::json;
use tokio::runtime::Runtime;
// ==================== API Call Performance Benchmarks ====================

fn bench_fetch_ticker(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let binance = Binance::new(ExchangeConfig::default()).unwrap();

    c.bench_function("fetch_ticker_btc_usdt", |b| {
        b.to_async(&rt).iter(|| async {
            let _ = binance
                .fetch_ticker(
                    black_box("BTC/USDT"),
                    ccxt_core::types::TickerParams::default(),
                )
                .await;
        });
    });
}

fn bench_fetch_order_book(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let binance = Binance::new(ExchangeConfig::default()).unwrap();

    let mut group = c.benchmark_group("fetch_order_book");

    for depth in [5, 10, 20, 50].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(depth), depth, |b, &depth| {
            b.to_async(&rt).iter(|| async {
                let _ = binance
                    .fetch_order_book(black_box("BTC/USDT"), Some(depth))
                    .await;
            });
        });
    }

    group.finish();
}

fn bench_fetch_trades(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let binance = Binance::new(ExchangeConfig::default()).unwrap();

    c.bench_function("fetch_trades_recent", |b| {
        b.to_async(&rt).iter(|| async {
            let _ = binance.fetch_trades(black_box("BTC/USDT"), Some(100)).await;
        });
    });
}

fn bench_fetch_ohlcv(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let binance = Binance::new(ExchangeConfig::default()).unwrap();

    let mut group = c.benchmark_group("fetch_ohlcv");

    for timeframe in ["1m", "5m", "1h", "1d"].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(timeframe),
            timeframe,
            |b, timeframe| {
                b.to_async(&rt).iter(|| async {
                    let _ = binance
                        .fetch_ohlcv(black_box("BTC/USDT"), timeframe, None, Some(100), None)
                        .await;
                });
            },
        );
    }

    group.finish();
}

// ==================== Data Parsing Performance Benchmarks ====================

fn bench_parse_market(c: &mut Criterion) {
    let data = json!({
        "symbol": "BTCUSDT",
        "status": "TRADING",
        "baseAsset": "BTC",
        "quoteAsset": "USDT",
        "baseAssetPrecision": 8,
        "quoteAssetPrecision": 8,
        "orderTypes": ["LIMIT", "MARKET"],
        "filters": [
            {
                "filterType": "PRICE_FILTER",
                "minPrice": "0.01",
                "maxPrice": "1000000.00",
                "tickSize": "0.01"
            },
            {
                "filterType": "LOT_SIZE",
                "minQty": "0.00001",
                "maxQty": "9000.00",
                "stepSize": "0.00001"
            }
        ]
    });

    c.bench_function("parse_market", |b| {
        b.iter(|| {
            let _ = parser::parse_market(black_box(&data));
        });
    });
}

fn bench_parse_ticker(c: &mut Criterion) {
    let data = json!({
        "symbol": "BTCUSDT",
        "lastPrice": "50000.00",
        "bidPrice": "49999.00",
        "askPrice": "50001.00",
        "highPrice": "51000.00",
        "lowPrice": "49000.00",
        "volume": "1000.5",
        "quoteVolume": "50000000.00"
    });

    c.bench_function("parse_ticker", |b| {
        b.iter(|| {
            let _ = parser::parse_ticker(black_box(&data), None::<&ccxt_core::Market>);
        });
    });
}

fn bench_parse_trade(c: &mut Criterion) {
    let data = json!({
        "id": 12345,
        "price": "50000.00",
        "qty": "0.1",
        "time": 1234567890000_i64,
        "isBuyerMaker": false
    });

    c.bench_function("parse_trade", |b| {
        b.iter(|| {
            let _ = parser::parse_trade(black_box(&data), None::<&ccxt_core::Market>);
        });
    });
}

fn bench_parse_order(c: &mut Criterion) {
    let data = json!({
        "orderId": 12345,
        "symbol": "BTCUSDT",
        "price": "50000.00",
        "origQty": "0.1",
        "executedQty": "0.1",
        "status": "FILLED",
        "type": "LIMIT",
        "side": "BUY",
        "time": 1234567890000_i64
    });

    c.bench_function("parse_order", |b| {
        b.iter(|| {
            let _ = parser::parse_order(black_box(&data), None::<&ccxt_core::Market>);
        });
    });
}

fn bench_parse_multiple_markets(c: &mut Criterion) {
    // Create 100 market data entries
    let markets: Vec<_> = (0..100)
        .map(|i| {
            json!({
                "symbol": format!("SYM{}USDT", i),
                "status": "TRADING",
                "baseAsset": format!("SYM{}", i),
                "quoteAsset": "USDT",
                "baseAssetPrecision": 8,
                "quoteAssetPrecision": 8
            })
        })
        .collect();

    c.bench_function("parse_100_markets", |b| {
        b.iter(|| {
            for market_data in &markets {
                let _ = parser::parse_market(black_box(market_data));
            }
        });
    });
}

// ==================== Signature Generation Performance Benchmarks ====================

fn bench_signature_generation(c: &mut Criterion) {
    let auth = ccxt_exchanges::binance::auth::BinanceAuth::new("test_api_key", "test_secret_key");

    let query_string =
        "symbol=BTCUSDT&side=BUY&type=LIMIT&quantity=0.1&price=50000&timestamp=1234567890000";

    c.bench_function("generate_signature", |b| {
        b.iter(|| {
            let _ = auth.sign(black_box(query_string));
        });
    });
}

fn bench_signature_with_various_lengths(c: &mut Criterion) {
    let auth = ccxt_exchanges::binance::auth::BinanceAuth::new("test_api_key", "test_secret_key");

    let mut group = c.benchmark_group("signature_lengths");

    for length in [50, 100, 200, 500].iter() {
        let query = "a=b&".repeat(*length);
        group.bench_with_input(BenchmarkId::from_parameter(length), &query, |b, query| {
            b.iter(|| {
                let _ = auth.sign(black_box(query));
            });
        });
    }

    group.finish();
}

// ==================== Memory Performance Benchmarks ====================

fn bench_exchange_creation(c: &mut Criterion) {
    c.bench_function("create_exchange_instance", |b| {
        b.iter(|| {
            let _ = Binance::new(black_box(ExchangeConfig::default()));
        });
    });
}

fn bench_exchange_with_credentials(c: &mut Criterion) {
    c.bench_function("create_exchange_with_credentials", |b| {
        b.iter(|| {
            let mut config = ExchangeConfig::default();
            config.api_key = Some("test_key".to_string());
            config.secret = Some("test_secret".to_string());
            let _ = Binance::new(black_box(config));
        });
    });
}

// ==================== Concurrent Performance Benchmarks ====================

fn bench_concurrent_ticker_fetches(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let binance = Binance::new(ExchangeConfig::default()).unwrap();

    c.bench_function("concurrent_10_ticker_fetches", |b| {
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
                .map(|symbol| {
                    binance.fetch_ticker(symbol, ccxt_core::types::TickerParams::default())
                })
                .collect();

            let _ = futures::future::join_all(futures).await;
        });
    });
}

// Configure benchmark groups
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
        bench_parse_order,
        bench_parse_multiple_markets,

        // Signature benchmarks
        bench_signature_generation,
        bench_signature_with_various_lengths,

        // Memory benchmarks
        bench_exchange_creation,
        bench_exchange_with_credentials,

        // Concurrent benchmarks
        bench_concurrent_ticker_fetches
);

criterion_main!(benches);
