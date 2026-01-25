//! Binance user trades API integration tests.
//!
//! Tests user trade history functionality including subscription and filtering.

use anyhow::Result;
use ccxt_core::{
    ExchangeConfig,
    types::Trade,
    types::order::OrderSide,
    types::{Amount, Cost, Price},
};
use ccxt_exchanges::binance::Binance;
use std::sync::Arc;

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[tokio::test]
    #[ignore]
    async fn test_watch_my_trades_all_symbols() -> Result<()> {
        let config = ExchangeConfig {
            api_key: std::env::var("BINANCE_API_KEY")
                .ok()
                .map(ccxt_core::SecretString::new),
            secret: std::env::var("BINANCE_API_SECRET")
                .ok()
                .map(ccxt_core::SecretString::new),
            ..Default::default()
        };

        let exchange = Arc::new(Binance::new(config)?);

        let trades = exchange.watch_my_trades(None, None, Some(10), None).await?;

        assert!(!trades.is_empty(), "Must return at least one trade");

        for trade in &trades {
            assert!(!trade.symbol.is_empty());
            assert!(
                trade.id.is_some() || trade.order.is_some(),
                "Trade ID or order ID must exist"
            );
            assert!(trade.timestamp > 0, "Timestamp must exist");
            assert!(
                trade.price.0 > rust_decimal::Decimal::ZERO,
                "Price must be positive"
            );
            assert!(
                trade.amount.0 > rust_decimal::Decimal::ZERO,
                "Amount must be positive"
            );
            assert!(matches!(trade.side, OrderSide::Buy | OrderSide::Sell));
        }

        println!("✓ Fetched my trades: {} trades", trades.len());
        Ok(())
    }

    #[tokio::test]
    #[ignore]
    async fn test_watch_my_trades_specific_symbol() -> Result<()> {
        let config = ExchangeConfig {
            api_key: std::env::var("BINANCE_API_KEY")
                .ok()
                .map(ccxt_core::SecretString::new),
            secret: std::env::var("BINANCE_API_SECRET")
                .ok()
                .map(ccxt_core::SecretString::new),
            ..Default::default()
        };

        let exchange = Arc::new(Binance::new(config)?);

        let trades = exchange
            .watch_my_trades(Some("BTC/USDT"), None, Some(5), None)
            .await?;

        assert!(!trades.is_empty(), "Must return at least one trade");

        for trade in &trades {
            assert_eq!(trade.symbol, "BTC/USDT");
            assert!(trade.timestamp > 0);
            assert!(trade.price.0 > rust_decimal::Decimal::ZERO);
            println!(
                "  Trade: {} {} {} @ ${}",
                trade.side, trade.amount, trade.symbol, trade.price
            );
        }

        println!("✓ Fetched BTC/USDT trades: {} trades", trades.len());
        Ok(())
    }

    #[tokio::test]
    #[ignore]
    async fn test_watch_my_trades_structure() -> Result<()> {
        let config = ExchangeConfig {
            api_key: std::env::var("BINANCE_API_KEY")
                .ok()
                .map(ccxt_core::SecretString::new),
            secret: std::env::var("BINANCE_API_SECRET")
                .ok()
                .map(ccxt_core::SecretString::new),
            ..Default::default()
        };

        let exchange = Arc::new(Binance::new(config)?);

        let trades = exchange.watch_my_trades(None, None, Some(1), None).await?;

        assert!(!trades.is_empty());

        let trade = &trades[0];

        println!("Trade structure:");
        println!("  ID: {:?}", trade.id);
        println!("  Order ID: {:?}", trade.order);
        println!("  Symbol: {}", trade.symbol);
        println!("  Side: {}", trade.side);
        println!("  Price: ${}", trade.price);
        println!("  Amount: {}", trade.amount);
        println!("  Total: ${}", trade.price.0 * trade.amount.0);
        println!(
            "  Timestamp: {} ({})",
            trade.timestamp,
            trade.datetime.as_deref().unwrap_or("N/A")
        );
        println!("  Order: {:?}", trade.order);

        assert!(!trade.symbol.is_empty());
        assert!(trade.timestamp > 0);
        assert!(trade.price.0 > rust_decimal::Decimal::ZERO);
        assert!(trade.amount.0 > rust_decimal::Decimal::ZERO);

        println!("✓ Trade structure validation passed");
        Ok(())
    }

    #[tokio::test]
    #[ignore]
    async fn test_watch_my_trades_with_since() -> Result<()> {
        let config = ExchangeConfig {
            api_key: std::env::var("BINANCE_API_KEY")
                .ok()
                .map(ccxt_core::SecretString::new),
            secret: std::env::var("BINANCE_API_SECRET")
                .ok()
                .map(ccxt_core::SecretString::new),
            ..Default::default()
        };

        let exchange = Arc::new(Binance::new(config)?);

        let since = chrono::Utc::now().timestamp_millis() - 24 * 60 * 60 * 1000;

        let trades = exchange
            .watch_my_trades(None, Some(since), Some(20), None)
            .await?;

        println!("✓ Fetched {} trades since specific time", trades.len());

        for trade in &trades {
            assert!(
                trade.timestamp >= since,
                "All trades must be after since time"
            );
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_trade_types() -> Result<()> {
        use rust_decimal::Decimal;

        let trade = Trade {
            id: Some("123456".to_string()),
            order: Some("987654".to_string()),
            symbol: "BTC/USDT".to_string(),
            trade_type: None,
            side: OrderSide::Buy,
            taker_or_maker: None,
            price: Price::from(Decimal::from(50000)),
            amount: Amount::from(Decimal::from_f64_retain(0.1).unwrap()),
            cost: Some(Cost::from(Decimal::from_f64_retain(5000.0).unwrap())),
            fee: None,
            timestamp: 1234567890000,
            datetime: Some("2023-01-01T00:00:00.000Z".to_string()),
            info: HashMap::new(),
        };

        assert_eq!(trade.symbol, "BTC/USDT");
        assert_eq!(trade.side, OrderSide::Buy);
        assert!(trade.price.0 > Decimal::ZERO);
        assert!(trade.amount.0 > Decimal::ZERO);
        assert!(trade.timestamp > 0);

        let calculated_cost = trade.price.0 * trade.amount.0;
        assert!(
            (trade.cost.unwrap().0 - calculated_cost).abs() < Decimal::new(1, 10),
            "Cost should match calculated value"
        );

        println!("✓ Trade data types validation passed");
        Ok(())
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;

    #[tokio::test]
    #[ignore]
    async fn test_complete_my_trades_workflow() -> Result<()> {
        if std::env::var("BINANCE_API_KEY").is_err() || std::env::var("BINANCE_API_SECRET").is_err()
        {
            println!("⚠️  Test skipped: API credentials not configured");
            return Ok(());
        }

        let config = ExchangeConfig {
            api_key: std::env::var("BINANCE_API_KEY")
                .ok()
                .map(ccxt_core::SecretString::new),
            secret: std::env::var("BINANCE_API_SECRET")
                .ok()
                .map(ccxt_core::SecretString::new),
            ..Default::default()
        };

        let exchange = Arc::new(Binance::new(config)?);

        println!("\n=== Complete My Trades Workflow Test ===\n");

        println!("1. Fetch all symbols trades...");
        let all_trades = exchange
            .clone()
            .watch_my_trades(None, None, Some(20), None)
            .await?;
        println!("   ✓ Fetched {} trades", all_trades.len());

        println!("2. Fetch BTC/USDT trades...");
        let btc_trades = exchange
            .clone()
            .watch_my_trades(Some("BTC/USDT"), None, Some(10), None)
            .await?;
        println!("   ✓ Fetched {} BTC/USDT trades", btc_trades.len());

        println!("\n3. Data analysis:");
        let mut total_volume = rust_decimal::Decimal::ZERO;
        let mut buy_count = 0;
        let mut sell_count = 0;

        for trade in &all_trades {
            total_volume += (trade.price * trade.amount).as_decimal();
            match trade.side {
                OrderSide::Buy => buy_count += 1,
                OrderSide::Sell => sell_count += 1,
            }
        }

        println!("   Total volume: ${}", total_volume);
        println!("   Buy count: {}", buy_count);
        println!("   Sell count: {}", sell_count);

        println!("\n=== Workflow Test Complete ===\n");
        Ok(())
    }
}
