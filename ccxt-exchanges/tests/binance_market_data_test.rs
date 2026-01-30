#![allow(clippy::disallowed_methods)]
//! Binance market data tests.
//!
//! Tests for market data methods including fetch_bids_asks, fetch_last_prices,
//! fetch_mark_price, and fetch_time.

use ccxt_core::Exchange;
use ccxt_core::ExchangeConfig;
use ccxt_core::error::Result;
use ccxt_exchanges::binance::Binance;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

fn create_test_binance() -> Binance {
    let config = ExchangeConfig {
        id: "binance".to_string(),
        name: "Binance".to_string(),
        api_key: Some(ccxt_core::SecretString::new("test_api_key")),
        secret: Some(ccxt_core::SecretString::new("test_api_secret")),
        ..Default::default()
    };
    Binance::new(config).expect("Failed to create Binance instance")
}

#[tokio::test]
#[ignore = "Requires network access to Binance API"]
async fn test_fetch_market_data() {
    let exchange = Binance::new(ExchangeConfig::default()).unwrap();
    exchange.load_markets(false).await.unwrap();

    let market = exchange.market("BTC/USDT").await.unwrap();
    assert_eq!(market.symbol, "BTC/USDT");
    assert!(market.active);

    let markets = exchange.markets().await;
    assert!(markets.contains_key("BTC/USDT"));
    assert!(markets.contains_key("ETH/USDT"));

    println!("Successfully fetched market data.");
}

#[cfg(test)]
mod tests {
    use super::*;
    use ccxt_core::types::{BidAsk, LastPrice, MarkPrice};

    #[tokio::test]
    #[ignore = "Requires network access to Binance API"]
    async fn test_fetch_time() {
        let binance = create_test_binance();
        let server_time = binance.fetch_time().await.unwrap();

        assert!(server_time.server_time > 0);
        assert!(!server_time.datetime.is_empty());
        println!(
            "Server time: {} ({})",
            server_time.server_time, server_time.datetime
        );
    }

    #[tokio::test]
    #[ignore = "Requires network access to Binance API"]
    async fn test_fetch_bids_asks_single_symbol() {
        let binance = create_test_binance();
        binance.load_markets(false).await.unwrap();

        let bid_asks = binance.fetch_bids_asks(Some("BTC/USDT")).await.unwrap();
        assert!(!bid_asks.is_empty());

        let bid_ask = &bid_asks[0];
        assert!(bid_ask.bid_price > Decimal::ZERO);
        assert!(bid_ask.ask_price > Decimal::ZERO);
        assert!(bid_ask.ask_price >= bid_ask.bid_price);
        println!(
            "BTC/USDT bid: {}, ask: {}",
            bid_ask.bid_price, bid_ask.ask_price
        );
    }

    #[tokio::test]
    #[ignore = "Requires network access to Binance API"]
    async fn test_fetch_bids_asks_all_symbols() {
        let binance = create_test_binance();
        binance.load_markets(false).await.unwrap();

        let bid_asks = binance.fetch_bids_asks(None).await.unwrap();
        assert!(!bid_asks.is_empty());
        println!("Total symbols with bid/ask: {}", bid_asks.len());
    }

    #[tokio::test]
    #[ignore = "Requires network access to Binance API"]
    async fn test_fetch_last_prices_single_symbol() {
        let binance = create_test_binance();
        binance.load_markets(false).await.unwrap();

        let prices = binance.fetch_last_prices(Some("BTC/USDT")).await.unwrap();
        assert!(!prices.is_empty());

        let price = &prices[0];
        assert!(price.price > Decimal::ZERO);
        println!("BTC/USDT last price: {}", price.price);
    }

    #[tokio::test]
    #[ignore = "Requires network access to Binance API"]
    async fn test_fetch_last_prices_all_symbols() {
        let binance = create_test_binance();
        binance.load_markets(false).await.unwrap();

        let prices = binance.fetch_last_prices(None).await.unwrap();
        assert!(!prices.is_empty());
        println!("Total symbols with prices: {}", prices.len());
    }

    #[tokio::test]
    #[ignore = "Requires network access to Binance API"]
    async fn test_fetch_mark_price_single_symbol() {
        let binance = create_test_binance();
        binance.load_markets(false).await.unwrap();

        // Note: fetch_mark_price is for futures markets
        let mark_prices = binance
            .fetch_mark_price(Some("BTC/USDT:USDT"))
            .await
            .unwrap();
        assert!(!mark_prices.is_empty());

        let mark_price = &mark_prices[0];
        assert!(mark_price.mark_price > Decimal::ZERO);
        println!("BTC/USDT:USDT mark price: {}", mark_price.mark_price);
        if let Some(funding_rate) = mark_price.last_funding_rate {
            println!("Funding rate: {}", funding_rate);
        }
    }

    #[tokio::test]
    #[ignore = "Requires network access to Binance API"]
    async fn test_fetch_mark_price_all_symbols() {
        let binance = create_test_binance();
        binance.load_markets(false).await.unwrap();

        let mark_prices = binance.fetch_mark_price(None).await.unwrap();
        assert!(!mark_prices.is_empty());
        println!(
            "Total futures symbols with mark prices: {}",
            mark_prices.len()
        );
    }

    #[tokio::test]
    #[ignore = "Requires network access to Binance API"]
    async fn test_bid_ask_spread_analysis() {
        let binance = create_test_binance();
        binance.load_markets(false).await.unwrap();

        let bid_asks = binance.fetch_bids_asks(Some("BTC/USDT")).await.unwrap();
        assert!(!bid_asks.is_empty());

        let bid_ask = &bid_asks[0];
        let spread = bid_ask.spread();
        let mid_price = bid_ask.mid_price();
        let spread_percent = bid_ask.spread_percent();

        assert!(spread >= Decimal::ZERO);
        assert!(mid_price > Decimal::ZERO);
        assert!(spread_percent >= Decimal::ZERO);

        println!(
            "Spread: {}, Mid price: {}, Spread %: {}",
            spread, mid_price, spread_percent
        );
    }

    #[tokio::test]
    async fn test_market_data_types() -> Result<()> {
        let bid_ask = BidAsk {
            symbol: "BTC/USDT".to_string(),
            bid_price: dec!(50000),
            ask_price: dec!(50100),
            bid_quantity: dec!(10),
            ask_quantity: dec!(15),
            timestamp: 1234567890000,
        };

        assert_eq!(bid_ask.symbol, "BTC/USDT");
        assert_eq!(bid_ask.spread(), dec!(100));
        assert_eq!(bid_ask.mid_price(), dec!(50050));
        assert!(bid_ask.spread_percent() > Decimal::ZERO);

        let last_price = LastPrice {
            symbol: "BTC/USDT".to_string(),
            price: dec!(50000),
            timestamp: 1234567890000,
            datetime: "2023-01-01T00:00:00.000Z".to_string(),
        };

        assert_eq!(last_price.symbol, "BTC/USDT");
        assert_eq!(last_price.price, dec!(50000));
        assert!(last_price.timestamp > 0);

        let mark_price = MarkPrice {
            symbol: "BTC/USDT:USDT".to_string(),
            mark_price: dec!(50050),
            index_price: Some(dec!(50000)),
            estimated_settle_price: None,
            last_funding_rate: Some(dec!(0.0001)),
            interest_rate: Some(dec!(0.0003)),
            next_funding_time: Some(1234567890000),
            timestamp: 1234567890000,
        };

        assert_eq!(mark_price.symbol, "BTC/USDT:USDT");
        assert_eq!(mark_price.mark_price, dec!(50050));
        assert!(mark_price.last_funding_rate.is_some());

        println!("✓ Market data types test passed");
        Ok(())
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;

    #[tokio::test]
    #[ignore = "Requires network access to Binance API"]
    async fn test_complete_market_data_workflow() {
        let binance = create_test_binance();
        binance.load_markets(false).await.unwrap();

        // Test fetch_time
        let server_time = binance.fetch_time().await.unwrap();
        assert!(server_time.server_time > 0);
        println!("1. Server time: {}", server_time.datetime);

        // Test fetch_bids_asks
        let bid_asks = binance.fetch_bids_asks(Some("BTC/USDT")).await.unwrap();
        assert!(!bid_asks.is_empty());
        println!(
            "2. BTC/USDT bid/ask: {} / {}",
            bid_asks[0].bid_price, bid_asks[0].ask_price
        );

        // Test fetch_last_prices
        let prices = binance.fetch_last_prices(Some("BTC/USDT")).await.unwrap();
        assert!(!prices.is_empty());
        println!("3. BTC/USDT last price: {}", prices[0].price);

        // Test fetch_mark_price (futures)
        let mark_prices = binance
            .fetch_mark_price(Some("BTC/USDT:USDT"))
            .await
            .unwrap();
        assert!(!mark_prices.is_empty());
        println!("4. BTC/USDT:USDT mark price: {}", mark_prices[0].mark_price);

        println!("✓ Complete market data workflow test passed");
    }
}
