#![allow(clippy::disallowed_methods)]
//! Binance orderbook WebSocket integration tests

use ccxt_core::{Amount, ExchangeConfig, Price, error::Result, types::Market};
use ccxt_exchanges::binance::Binance;
use serde_json::json;
use std::collections::HashMap;

#[tokio::test]
#[ignore]
async fn test_watch_order_book() -> Result<()> {
    let exchange = Binance::new(ExchangeConfig::default())?;

    let orderbook = exchange.watch_order_book("BTC/USDT", None, None).await?;

    assert_eq!(orderbook.symbol, "BTC/USDT");
    assert!(!orderbook.bids.is_empty(), "Orderbook must have bids");
    assert!(!orderbook.asks.is_empty(), "Orderbook must have asks");
    assert!(orderbook.timestamp > 0, "Orderbook must have timestamp");

    if let (Some(best_bid), Some(best_ask)) = (orderbook.best_bid(), orderbook.best_ask()) {
        assert!(
            best_bid.price < best_ask.price,
            "Best bid must be lower than best ask"
        );
    }

    for i in 0..orderbook.bids.len().saturating_sub(1) {
        assert!(
            orderbook.bids[i].price >= orderbook.bids[i + 1].price,
            "Bids must be sorted by descending price"
        );
    }

    for i in 0..orderbook.asks.len().saturating_sub(1) {
        assert!(
            orderbook.asks[i].price <= orderbook.asks[i + 1].price,
            "Asks must be sorted by ascending price"
        );
    }

    println!("✓ watch_order_book basic functionality");
    println!("  - Best bid: {:?}", orderbook.best_bid());
    println!("  - Best ask: {:?}", orderbook.best_ask());
    println!("  - Bid depth: {} levels", orderbook.bids.len());
    println!("  - Ask depth: {} levels", orderbook.asks.len());

    Ok(())
}

#[tokio::test]
#[ignore]
async fn test_watch_order_book_with_limit() -> Result<()> {
    let exchange = Binance::new(ExchangeConfig::default())?;

    let orderbook = exchange
        .watch_order_book("ETH/USDT", Some(100), None)
        .await?;

    assert_eq!(orderbook.symbol, "ETH/USDT");
    assert!(!orderbook.bids.is_empty());
    assert!(!orderbook.asks.is_empty());

    println!("✓ watch_order_book with limit");
    println!("  - Bid count: {}", orderbook.bids.len());
    println!("  - Ask count: {}", orderbook.asks.len());

    Ok(())
}

#[tokio::test]
#[ignore]
async fn test_watch_order_book_slow_update() -> Result<()> {
    let exchange = Binance::new(ExchangeConfig::default())?;

    let mut params = HashMap::new();
    params.insert("speed".to_string(), json!(1000));

    let orderbook = exchange
        .watch_order_book("BTC/USDT", None, Some(params))
        .await?;

    assert_eq!(orderbook.symbol, "BTC/USDT");
    assert!(!orderbook.bids.is_empty());
    assert!(!orderbook.asks.is_empty());

    println!("✓ watch_order_book with slow update");

    Ok(())
}

#[tokio::test]
#[ignore]
async fn test_watch_order_book_futures() -> Result<()> {
    let exchange = Binance::new(ExchangeConfig::default())?;

    let orderbook = exchange
        .watch_order_book("BTC/USDT:USDT", None, None)
        .await?;

    assert_eq!(orderbook.symbol, "BTC/USDT:USDT");
    assert!(!orderbook.bids.is_empty());
    assert!(!orderbook.asks.is_empty());

    println!("✓ watch_order_book futures market");
    println!("  - Best bid: {:?}", orderbook.best_bid());
    println!("  - Best ask: {:?}", orderbook.best_ask());

    Ok(())
}

#[tokio::test]
#[ignore]
async fn test_watch_order_books_multiple() -> Result<()> {
    let exchange = Binance::new(ExchangeConfig::default())?;

    let symbols = vec![
        "BTC/USDT".to_string(),
        "ETH/USDT".to_string(),
        "BNB/USDT".to_string(),
    ];

    let orderbooks = exchange
        .watch_order_books(symbols.clone(), None, None)
        .await?;

    assert_eq!(orderbooks.len(), symbols.len());

    for symbol in &symbols {
        let ob = orderbooks.get(symbol).expect("Must have orderbook");
        assert_eq!(&ob.symbol, symbol);
        assert!(!ob.bids.is_empty(), "{} must have bids", symbol);
        assert!(!ob.asks.is_empty(), "{} must have asks", symbol);

        println!("  {} - spread: {:?}", symbol, ob.spread());
    }

    println!("✓ watch_order_books multiple symbols");

    Ok(())
}

#[tokio::test]
#[ignore]
async fn test_watch_order_books_with_params() -> Result<()> {
    let exchange = Binance::new(ExchangeConfig::default())?;

    let symbols = vec!["BTC/USDT".to_string(), "ETH/USDT".to_string()];

    let mut params = HashMap::new();
    params.insert("speed".to_string(), json!(1000));

    let orderbooks = exchange
        .watch_order_books(symbols.clone(), Some(50), Some(params))
        .await?;

    assert_eq!(orderbooks.len(), symbols.len());

    for symbol in &symbols {
        let ob = orderbooks.get(symbol).expect("Must have orderbook");
        assert!(!ob.bids.is_empty());
        assert!(!ob.asks.is_empty());
    }

    println!("✓ watch_order_books with parameters");

    Ok(())
}

#[tokio::test]
#[ignore]
async fn test_watch_order_books_futures_multiple() -> Result<()> {
    let exchange = Binance::new(ExchangeConfig::default())?;

    let symbols = vec!["BTC/USDT:USDT".to_string(), "ETH/USDT:USDT".to_string()];

    let orderbooks = exchange
        .watch_order_books(symbols.clone(), None, None)
        .await?;

    assert_eq!(orderbooks.len(), symbols.len());

    for symbol in &symbols {
        let ob = orderbooks.get(symbol).expect("应有期货订单簿");
        assert_eq!(&ob.symbol, symbol);
        assert!(!ob.bids.is_empty());
        assert!(!ob.asks.is_empty());
    }

    println!("✓ watch_order_books期货市场功能正常");

    Ok(())
}

#[tokio::test]
async fn test_watch_order_books_empty_symbols() {
    let exchange = Binance::new(ExchangeConfig::default()).unwrap();

    let result = exchange.watch_order_books(vec![], None, None).await;

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("cannot be empty"));

    println!("✓ watch_order_books空交易对列表错误处理正常");
}

#[tokio::test]
async fn test_watch_order_books_too_many_symbols() {
    let exchange = Binance::new(ExchangeConfig::default()).unwrap();

    // 创建超过200个交易对
    let symbols: Vec<String> = (0..201).map(|i| format!("SYMBOL{}/USDT", i)).collect();

    let result = exchange.watch_order_books(symbols, None, None).await;

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("max 200"));

    println!("✓ watch_order_books超限错误处理正常");
}

#[tokio::test]
async fn test_watch_order_book_invalid_speed() {
    let exchange = Binance::new(ExchangeConfig::default()).unwrap();

    // Manually populate markets to avoid network call
    let market = Market {
        id: "BTCUSDT".to_string(),
        symbol: "BTC/USDT".to_string(),
        ..Market::default()
    };
    exchange
        .base()
        .set_markets(vec![market], None)
        .await
        .unwrap();

    let mut params = HashMap::new();
    params.insert("speed".to_string(), json!(500)); // 无效速度

    let result = exchange
        .watch_order_book("BTC/USDT", None, Some(params))
        .await;

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("100 or 1000"));

    println!("✓ watch_order_book无效速度错误处理正常");
}

#[tokio::test]
#[ignore]
async fn test_orderbook_spread_calculation() -> Result<()> {
    let exchange = Binance::new(ExchangeConfig::default())?;

    let orderbook = exchange.watch_order_book("BTC/USDT", None, None).await?;

    // 测试价差计算
    if let Some(spread) = orderbook.spread() {
        assert!(
            spread > Price::from(rust_decimal::Decimal::ZERO),
            "Spread must be positive"
        );
        println!("  - 价差: {}", spread);
    }

    // 测试价差百分比
    if let Some(spread_pct) = orderbook.spread_percentage() {
        assert!(
            spread_pct > rust_decimal::Decimal::ZERO,
            "价差百分比应为正数"
        );
        println!("  - 价差百分比: {}%", spread_pct);
    }

    // 测试中间价
    if let Some(mid) = orderbook.mid_price() {
        assert!(
            mid > Price::from(rust_decimal::Decimal::ZERO),
            "中间价应为正数"
        );
        println!("  - 中间价: {}", mid);
    }

    println!("✓ 订单簿价差计算功能正常");

    Ok(())
}

#[tokio::test]
#[ignore]
async fn test_orderbook_depth_analysis() -> Result<()> {
    let exchange = Binance::new(ExchangeConfig::default())?;

    let orderbook = exchange.watch_order_book("ETH/USDT", None, None).await?;

    // 测试深度分析
    let total_bid_volume = orderbook.bid_volume();
    let total_ask_volume = orderbook.ask_volume();

    assert!(
        total_bid_volume > Amount::from(rust_decimal::Decimal::ZERO),
        "Total bid volume must be positive"
    );
    assert!(
        total_ask_volume > Amount::from(rust_decimal::Decimal::ZERO),
        "Total ask volume must be positive"
    );

    println!("  - Total bid volume: {}", total_bid_volume);
    println!("  - Total ask volume: {}", total_ask_volume);

    let total_volume = total_bid_volume + total_ask_volume;
    if total_volume > Amount::from(rust_decimal::Decimal::ZERO) {
        let imbalance = (total_ask_volume - total_bid_volume) / total_volume;
        println!("  - Order imbalance: {}", imbalance);
    }

    println!("✓ Orderbook depth analysis");

    Ok(())
}

#[tokio::test]
#[ignore]
async fn test_orderbook_update_sequence() -> Result<()> {
    let exchange = Binance::new(ExchangeConfig::default())?;

    let orderbook1 = exchange.watch_order_book("BTC/USDT", None, None).await?;
    let update_id1 = orderbook1.nonce.unwrap_or(0);

    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

    let orderbook2 = exchange.watch_order_book("BTC/USDT", None, None).await?;
    let update_id2 = orderbook2.nonce.unwrap_or(0);

    assert!(
        update_id2 >= update_id1,
        "Update ID must increase: {} -> {}",
        update_id1,
        update_id2
    );

    println!("✓ Orderbook update sequence");
    println!("  - Initial update ID: {}", update_id1);
    println!("  - Subsequent update ID: {}", update_id2);

    Ok(())
}
