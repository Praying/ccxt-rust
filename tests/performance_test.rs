use ccxt_exchanges::binance::Binance;
use std::time::Instant;

#[tokio::test]
async fn test_load_markets_performance() {
    let exchange = Binance::builder().sandbox(true).build().unwrap();

    // Initial load
    let start = Instant::now();
    let _ = exchange.load_markets(true).await.unwrap();
    println!("Initial load: {:?}", start.elapsed());

    // Subsequent loads (should be cached but currently clones)
    let start = Instant::now();
    for _ in 0..100 {
        let _ = exchange.load_markets(false).await.unwrap();
    }
    println!("100 cached loads: {:?}", start.elapsed());
}
