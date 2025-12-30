use ccxt_exchanges::binance::Binance;
use std::time::Instant;

#[tokio::test]
async fn test_load_markets_performance() {
    let exchange = Binance::builder()
        .sandbox(true)
        .build()
        .expect("Failed to build Binance exchange instance");

    // Initial load
    let start = Instant::now();
    let _ = exchange
        .load_markets(true)
        .await
        .expect("Failed to load markets from Binance API");
    println!("Initial load: {:?}", start.elapsed());

    // Subsequent loads (should be cached but currently clones)
    let start = Instant::now();
    for _ in 0..100 {
        let _ = exchange
            .load_markets(false)
            .await
            .expect("Failed to reload markets from Binance API");
    }
    println!("100 cached loads: {:?}", start.elapsed());
}
