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
        api_key: Some("test_api_key".to_string()),
        secret: Some("test_api_secret".to_string()),
        ..Default::default()
    };
    Binance::new(config).expect("Failed to create Binance instance")
}
#[tokio::test]
#[ignore = "Requires network access to Binance API"]
async fn test_fetch_market_data() {
    let exchange = Binance::new(ExchangeConfig::default()).unwrap();
    exchange.load_markets(false).await.unwrap();

    // This call should fail to compile initially because market() is now async.
    // We will fix this by adding .await
    let market = exchange.market("BTC/USDT").await.unwrap();
    assert_eq!(market.symbol, "BTC/USDT");
    assert!(market.active);

    // This call should also be updated with .await
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
    #[ignore]
    async fn test_fetch_bids_asks_single_symbol() -> Result<()> {
        let binance = create_test_binance();

        let result = binance.fetch_bids_asks(Some("BTC/USDT")).await?;

        assert_eq!(result.len(), 1);
        let bid_ask = &result[0];

        assert_eq!(bid_ask.symbol, "BTC/USDT");
        assert!(bid_ask.bid_price > Decimal::ZERO, "Bid price must exist");
        assert!(bid_ask.ask_price > Decimal::ZERO, "Ask price must exist");
        assert!(
            bid_ask.bid_quantity > Decimal::ZERO,
            "Bid quantity must exist"
        );
        assert!(
            bid_ask.ask_quantity > Decimal::ZERO,
            "Ask quantity must exist"
        );
        assert!(
            bid_ask.bid_price < bid_ask.ask_price,
            "Bid must be less than ask"
        );

        println!("✓ Single symbol BBO test passed");
        Ok(())
    }

    #[tokio::test]
    #[ignore]
    async fn test_fetch_bids_asks_multiple_symbols() -> Result<()> {
        let binance = create_test_binance();

        let symbols = ["BTC/USDT", "ETH/USDT", "BNB/USDT"];
        let mut results = Vec::new();
        for symbol in &symbols {
            let result = binance.fetch_bids_asks(Some(*symbol)).await?;
            if !result.is_empty() {
                results.extend(result);
            }
        }

        assert_eq!(results.len(), symbols.len(), "Result count must match");

        for bid_ask in &results {
            assert!(
                symbols.contains(&bid_ask.symbol.as_str()),
                "Symbol must be in request list"
            );
            assert!(bid_ask.bid_price > Decimal::ZERO || bid_ask.ask_price > Decimal::ZERO);
        }

        println!("✓ Multiple symbols BBO test passed");
        Ok(())
    }

    #[tokio::test]
    #[ignore]
    async fn test_fetch_bids_asks_all_symbols() -> Result<()> {
        let binance = create_test_binance();

        let result = binance.fetch_bids_asks(None).await?;

        assert!(!result.is_empty(), "Must return at least one symbol");
        for bid_ask in result.iter().take(10) {
            assert!(!bid_ask.symbol.is_empty());
            assert!(
                bid_ask.bid_price > Decimal::ZERO || bid_ask.ask_price > Decimal::ZERO,
                "Must have bid or ask price"
            );
        }

        println!(
            "✓ All symbols BBO test passed (returned {} symbols)",
            result.len()
        );
        Ok(())
    }

    #[tokio::test]
    #[ignore]
    async fn test_fetch_last_prices_single_symbol() -> Result<()> {
        let binance = create_test_binance();

        let result = binance.fetch_last_prices(Some("BTC/USDT")).await?;

        assert!(result.len() == 1, "Must return one symbol price");
        let last_price = &result[0];

        assert_eq!(last_price.symbol, "BTC/USDT");
        assert!(last_price.price > Decimal::ZERO, "Price must exist");
        assert!(last_price.timestamp > 0, "Timestamp must exist");

        println!("✓ Single symbol last price test passed");
        Ok(())
    }

    #[tokio::test]
    #[ignore]
    async fn test_fetch_last_prices_multiple_symbols() -> Result<()> {
        let binance = create_test_binance();

        let symbols = ["BTC/USDT", "ETH/USDT", "BNB/USDT"];
        let mut results = Vec::new();
        for symbol in &symbols {
            let result = binance.fetch_last_prices(Some(*symbol)).await?;
            if !result.is_empty() {
                results.extend(result);
            }
        }

        assert_eq!(results.len(), symbols.len());

        for last_price in &results {
            assert!(symbols.contains(&last_price.symbol.as_str()));
            assert!(last_price.price > Decimal::ZERO);
            assert!(last_price.timestamp > 0);
        }

        println!("✓ Multiple symbols last price test passed");
        Ok(())
    }

    #[tokio::test]
    #[ignore]
    async fn test_fetch_mark_price_single_symbol() -> Result<()> {
        let binance = create_test_binance();

        let result = binance.fetch_mark_price(Some("BTC/USDT:USDT")).await?;

        assert!(result.len() == 1, "Must return one futures symbol");
        let mark_price = &result[0];

        assert_eq!(mark_price.symbol, "BTC/USDT:USDT");
        assert!(
            mark_price.mark_price > Decimal::ZERO,
            "Mark price must exist"
        );
        assert!(
            mark_price.last_funding_rate.is_some(),
            "Funding rate must exist"
        );
        assert!(mark_price.timestamp > 0, "Timestamp must exist");

        println!("✓ Single symbol mark price test passed");
        Ok(())
    }

    #[tokio::test]
    #[ignore]
    async fn test_bid_ask_spread_analysis() -> Result<()> {
        let binance = create_test_binance();

        let symbols = ["BTC/USDT", "ETH/USDT"];
        let mut results = Vec::new();
        for symbol in &symbols {
            let result = binance.fetch_bids_asks(Some(*symbol)).await?;
            results.extend(result);
        }

        for bid_ask in &results {
            let bid = bid_ask.bid_price;
            let ask = bid_ask.ask_price;
            let spread = ask - bid;
            let spread_percent = spread / bid * dec!(100);

            println!(
                "{} spread: ${:.2} ({:.4}%)",
                bid_ask.symbol, spread, spread_percent
            );

            assert!(spread > Decimal::ZERO, "Spread must be positive");
            assert!(
                spread_percent < dec!(10),
                "Spread percent must be reasonable"
            );
        }

        println!("✓ Bid-ask spread analysis test passed");
        Ok(())
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
    #[ignore]
    async fn test_complete_market_data_workflow() -> Result<()> {
        let binance = create_test_binance();

        println!("\n=== Complete Market Data Workflow Test ===\n");

        println!("1. Fetch bid-ask quotes...");
        let bid_asks = binance.fetch_bids_asks(Some("BTC/USDT")).await?;
        assert!(!bid_asks.is_empty());
        println!("   ✓ Fetched {} bid-ask quotes", bid_asks.len());

        println!("2. Fetch last prices...");
        let last_prices = binance.fetch_last_prices(Some("BTC/USDT")).await?;
        assert!(!last_prices.is_empty());
        println!("   ✓ Fetched {} last prices", last_prices.len());

        println!("3. Fetch futures mark prices...");
        let mark_prices = binance.fetch_mark_price(Some("BTC/USDT:USDT")).await?;
        assert!(!mark_prices.is_empty());
        println!("   ✓ Fetched {} mark prices", mark_prices.len());

        println!("4. Data consistency check...");
        let bid = bid_asks[0].bid_price;
        let ask = bid_asks[0].ask_price;
        let last = last_prices[0].price;

        assert!(
            last >= bid && last <= ask,
            "Last price must be between bid and ask"
        );
        println!("   ✓ Data consistency check passed");

        println!("\n=== Workflow Test Complete ===\n");
        Ok(())
    }
}
