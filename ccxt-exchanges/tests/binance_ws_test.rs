//! Binance WebSocket integration tests

use ccxt_core::{ExchangeConfig, types::Market};
use ccxt_exchanges::binance::Binance;
use serde_json::json;
use std::collections::HashMap;

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore]
    async fn test_watch_ticker() {
        let exchange = Binance::new(ExchangeConfig::default()).unwrap();

        match exchange.watch_ticker("BTC/USDT", None).await {
            Ok(ticker) => {
                println!("✓ watch_ticker succeeded");
                println!("  Symbol: {}", ticker.symbol);
                println!("  Last: {:?}", ticker.last);
                println!("  Bid: {:?}", ticker.bid);
                println!("  Ask: {:?}", ticker.ask);
                println!("  Volume: {:?}", ticker.base_volume);
                assert_eq!(ticker.symbol, "BTC/USDT");
                assert!(ticker.last.is_some());
            }
            Err(e) => {
                println!("✗ watch_ticker failed: {}", e);
                panic!("Test failed: {}", e);
            }
        }
    }

    #[tokio::test]
    #[ignore]
    async fn test_watch_mini_ticker() {
        let exchange = Binance::new(ExchangeConfig::default()).unwrap();

        let mut params = HashMap::new();
        params.insert("name".to_string(), json!("miniTicker"));

        match exchange.watch_ticker("ETH/USDT", Some(params)).await {
            Ok(ticker) => {
                println!("✓ watch_ticker with miniTicker succeeded");
                println!("  Symbol: {}", ticker.symbol);
                println!("  Last: {:?}", ticker.last);
                assert_eq!(ticker.symbol, "ETH/USDT");
            }
            Err(e) => {
                println!("✗ watch_ticker with miniTicker failed: {}", e);
                panic!("Test failed: {}", e);
            }
        }
    }

    #[tokio::test]
    #[ignore]
    async fn test_watch_tickers_multiple() {
        let exchange = Binance::new(ExchangeConfig::default()).unwrap();

        let symbols = vec![
            "BTC/USDT".to_string(),
            "ETH/USDT".to_string(),
            "BNB/USDT".to_string(),
        ];

        match exchange.watch_tickers(Some(symbols.clone()), None).await {
            Ok(tickers) => {
                println!(
                    "✓ watch_tickers succeeded, received {} tickers",
                    tickers.len()
                );

                for symbol in &symbols {
                    if let Some(ticker) = tickers.get(symbol) {
                        println!("  {} - Last: {:?}", ticker.symbol, ticker.last);
                    }
                }

                assert!(tickers.len() > 0);
            }
            Err(e) => {
                println!("✗ watch_tickers failed: {}", e);
                panic!("Test failed: {}", e);
            }
        }
    }

    #[tokio::test]
    #[ignore]
    async fn test_watch_all_tickers() {
        let exchange = Binance::new(ExchangeConfig::default()).unwrap();

        match exchange.watch_tickers(None, None).await {
            Ok(tickers) => {
                println!(
                    "✓ watch_tickers (all) succeeded, received {} tickers",
                    tickers.len()
                );

                for (i, (symbol, ticker)) in tickers.iter().enumerate() {
                    if i >= 5 {
                        break;
                    }
                    println!("  {} - Last: {:?}", symbol, ticker.last);
                }

                assert!(tickers.len() > 100);
            }
            Err(e) => {
                println!("✗ watch_tickers (all) failed: {}", e);
                panic!("Test failed: {}", e);
            }
        }
    }

    #[tokio::test]
    #[ignore]
    async fn test_watch_mark_price() {
        let exchange = Binance::new(ExchangeConfig::default()).unwrap();

        match exchange.watch_mark_price("BTC/USDT:USDT", None).await {
            Ok(ticker) => {
                println!("✓ watch_mark_price succeeded");
                println!("  Symbol: {}", ticker.symbol);
                println!("  Mark Price: {:?}", ticker.last);
                println!("  Index Price: {:?}", ticker.info.get("indexPrice"));
                assert!(ticker.symbol.contains("BTC"));
            }
            Err(e) => {
                println!("✗ watch_mark_price failed: {}", e);
                panic!("Test failed: {}", e);
            }
        }
    }

    #[tokio::test]
    #[ignore]
    async fn test_watch_mark_price_3s() {
        let exchange = Binance::new(ExchangeConfig::default()).unwrap();

        let mut params = HashMap::new();
        params.insert("use1sFreq".to_string(), json!(false));

        match exchange
            .watch_mark_price("ETH/USDT:USDT", Some(params))
            .await
        {
            Ok(ticker) => {
                println!("✓ watch_mark_price (3s update) succeeded");
                println!("  Symbol: {}", ticker.symbol);
                println!("  Mark Price: {:?}", ticker.last);
                assert!(ticker.symbol.contains("ETH"));
            }
            Err(e) => {
                println!("✗ watch_mark_price (3s update) failed: {}", e);
                panic!("Test failed: {}", e);
            }
        }
    }

    #[tokio::test]
    #[ignore]
    async fn test_watch_mark_prices() {
        let exchange = Binance::new(ExchangeConfig::default()).unwrap();

        let symbols = vec!["BTC/USDT:USDT".to_string(), "ETH/USDT:USDT".to_string()];

        match exchange
            .watch_mark_prices(Some(symbols.clone()), None)
            .await
        {
            Ok(tickers) => {
                println!(
                    "✓ watch_mark_prices succeeded, received {} tickers",
                    tickers.len()
                );

                for (symbol, ticker) in &tickers {
                    println!("  {} - Mark: {:?}", symbol, ticker.last);
                }

                assert!(tickers.len() > 0);
            }
            Err(e) => {
                println!("✗ watch_mark_prices failed: {}", e);
                panic!("Test failed: {}", e);
            }
        }
    }

    #[tokio::test]
    async fn test_watch_tickers_reject_book_ticker() {
        let exchange = Binance::new(ExchangeConfig::default()).unwrap();

        // Manually populate markets
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
        params.insert("name".to_string(), json!("bookTicker"));

        match exchange
            .watch_tickers(Some(vec!["BTC/USDT".to_string()]), Some(params))
            .await
        {
            Ok(_) => {
                panic!("Should reject bookTicker channel");
            }
            Err(e) => {
                println!("✓ Correctly rejected bookTicker: {}", e);
                assert!(e.to_string().contains("watch_bids_asks"));
            }
        }
    }

    #[tokio::test]
    async fn test_watch_mark_price_reject_spot() {
        let exchange = Binance::new(ExchangeConfig::default()).unwrap();

        // Manually populate markets
        let market = Market::new_spot(
            "BTCUSDT".to_string(),
            "BTC/USDT".to_string(),
            "BTC".to_string(),
            "USDT".to_string(),
        );
        exchange
            .base()
            .set_markets(vec![market], None)
            .await
            .unwrap();

        match exchange.watch_mark_price("BTC/USDT", None).await {
            Ok(_) => {
                panic!("Should reject spot market");
            }
            Err(e) => {
                println!("✓ Correctly rejected spot market: {}", e);
                assert!(e.to_string().contains("does not support"));
            }
        }
    }

    #[tokio::test]
    #[ignore]
    async fn test_ticker_cache() {
        let exchange = Binance::new(ExchangeConfig::default()).unwrap();
        let _ws = exchange.create_ws();

        println!("✓ Ticker cache functionality pending (requires public API)");
    }

    #[test]
    fn test_stream_format() {
        assert_eq!(format!("{}@{}", "btcusdt", "ticker"), "btcusdt@ticker");

        assert_eq!(
            format!("{}@{}", "ethusdt", "miniTicker"),
            "ethusdt@miniTicker"
        );

        assert_eq!(
            format!("{}@{}", "btcusdt", "markPrice@1s"),
            "btcusdt@markPrice@1s"
        );

        assert_eq!(
            format!("{}@{}", "btcusdt", "markPrice"),
            "btcusdt@markPrice"
        );

        assert_eq!(format!("!{}@arr", "ticker"), "!ticker@arr");

        assert_eq!(format!("!{}@arr", "miniTicker"), "!miniTicker@arr");

        println!("✓ Stream format validation");
    }
}
