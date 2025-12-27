//! Binance Futures (Perpetual Contracts) Usage Example
//!
//! This example demonstrates how to use the Binance futures implementation
//! to trade perpetual contracts and manage positions.

// Allow clippy warnings for example code - examples prioritize readability over strict linting
#![allow(clippy::disallowed_methods)]

use ccxt_core::ExchangeConfig;
use ccxt_core::error::Result;
use ccxt_exchanges::binance::Binance;
use rust_decimal::prelude::FromStr;

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== Binance Futures (Perpetual Contracts) Example ===\n");

    // Initialize Binance Futures exchange
    let futures_config = ExchangeConfig::default();
    let futures_exchange = Binance::new_swap(futures_config)?;

    // Example 1: Fetch all futures markets
    println!("1. Fetching futures markets...");
    match futures_exchange.fetch_markets().await {
        Ok(markets) => {
            println!("   Found {} futures markets", markets.len());

            // Find perpetual futures (symbols ending with /USDT)
            let perpetuals: Vec<_> = markets
                .values()
                .filter(|m| m.symbol.contains("/USDT"))
                .take(10)
                .collect();

            println!("   First 10 USDT perpetual futures:");
            for market in perpetuals {
                println!("     - {}: {}/{}", market.symbol, market.base, market.quote);
                if let Some(precision) = market.precision.price {
                    println!("       Price precision: {}", precision);
                }
                if let Some(precision) = market.precision.amount {
                    println!("       Amount precision: {}", precision);
                }
            }
        }
        Err(e) => println!("   Error: {}", e),
    }
    println!();

    // Example 2: Fetch BTC/USDT perpetual futures ticker
    println!("2. Fetching BTC/USDT perpetual futures ticker...");
    match futures_exchange
        .fetch_ticker("BTC/USDT", ccxt_core::types::TickerParams::default())
        .await
    {
        Ok(ticker) => {
            println!("   Symbol: {}", ticker.symbol);
            println!("   Last: {:?}", ticker.last);
            println!("   24h Change: {:?}", ticker.percentage);
            println!("   24h Volume: {:?}", ticker.base_volume);
            println!("   High: {:?}", ticker.high);
            println!("   Low: {:?}", ticker.low);
        }
        Err(e) => println!("   Error: {}", e),
    }
    println!();

    // Example 3: Fetch BTC/USDT perpetual futures order book
    println!("3. Fetching BTC/USDT perpetual futures order book...");
    match futures_exchange
        .fetch_order_book("BTC/USDT", Some(20))
        .await
    {
        Ok(order_book) => {
            println!("   Symbol: {}", order_book.symbol);
            println!("   Bids: {} levels", order_book.bids.len());
            println!("   Asks: {} levels", order_book.asks.len());

            if let Some(best_bid) = order_book.bids.first() {
                println!("   Best bid: {} @ {}", best_bid.amount, best_bid.price);
            }

            if let Some(best_ask) = order_book.asks.first() {
                println!("   Best ask: {} @ {}", best_ask.amount, best_ask.price);
            }

            if let (Some(best_bid), Some(best_ask)) =
                (order_book.bids.first(), order_book.asks.first())
            {
                let spread = best_ask.price - best_bid.price;
                let spread_percent =
                    (spread / best_bid.price) * rust_decimal::Decimal::from_str("100.0").unwrap();
                println!("   Spread: {} ({:.4}%)", spread, spread_percent);
            }

            // Show top 5 bid and ask levels
            println!("   Top 5 bid levels:");
            for (i, bid) in order_book.bids.iter().take(5).enumerate() {
                println!("     {}. {} @ {}", i + 1, bid.amount, bid.price);
            }

            println!("   Top 5 ask levels:");
            for (i, ask) in order_book.asks.iter().take(5).enumerate() {
                println!("     {}. {} @ {}", i + 1, ask.amount, ask.price);
            }
        }
        Err(e) => println!("   Error: {}", e),
    }
    println!();

    // Example 4: Fetch recent trades (using new API)
    println!("4. Fetching recent BTC/USDT futures trades...");
    match futures_exchange.fetch_trades("BTC/USDT", Some(10)).await {
        Ok(trades) => {
            println!("   Found {} trades", trades.len());
            for (i, trade) in trades.iter().enumerate() {
                println!(
                    "   Trade {}: {} {} @ {} (side: {:?}, time: {})",
                    i + 1,
                    trade.amount,
                    trade.symbol,
                    trade.price,
                    trade.side,
                    trade.timestamp
                );
            }
        }
        Err(e) => println!("   Error: {}", e),
    }
    println!();

    // Example 5: Fetch OHLCV (candlestick) data
    println!("5. Fetching BTC/USDT 1h futures OHLCV data...");
    match futures_exchange
        .fetch_ohlcv("BTC/USDT", "1h", None, Some(5), None)
        .await
    {
        Ok(candles) => {
            println!("   Found {} candles", candles.len());
            for (i, candle) in candles.iter().enumerate() {
                // OHLCV is now a struct with fields
                println!(
                    "   Candle {}: O:{} H:{} L:{} C:{} V:{}",
                    i + 1,
                    candle.open,
                    candle.high,
                    candle.low,
                    candle.close,
                    candle.volume
                );
            }
        }
        Err(e) => println!("   Error: {}", e),
    }
    println!();

    // Example 6: Fetch funding rate (placeholder)
    println!("6. Fetching funding rate...");
    // Note: This would require implementing fetch_funding_rate method
    println!("   BTC/USDT funding rate: ~0.01% (example)");
    println!("   Note: Implement fetch_funding_rate method to get real rates");
    println!();

    // Example 7: Private API endpoints (requires API credentials)
    println!("7. Private futures API examples (requires credentials)...");

    // To use private endpoints, uncomment and set your credentials:
    // let private_futures_config = ExchangeConfig {
    //     api_key: Some("YOUR_API_KEY".to_string()),
    //     secret: Some("YOUR_SECRET".to_string()),
    //     ..Default::default()
    // };
    // let futures_exchange_private = Binance::new_futures(private_futures_config)?;

    // Set leverage for perpetual futures
    // match futures_exchange_private.set_leverage("BTC/USDT", 10).await {
    //     Ok(result) => {
    //         println!("   Leverage set: {:?}", result);
    //     }
    //     Err(e) => println!("   Error: {}", e),
    // }

    // Create a perpetual futures limit order
    // match futures_exchange_private.create_order(
    //     "BTC/USDT",
    //     "limit",
    //     "buy",
    //     0.01,
    //     Some(40000.0)
    // ).await {
    //     Ok(order) => {
    //         println!("   Futures order created:");
    //         println!("   ID: {:?}", order.id);
    //         println!("   Symbol: {}", order.symbol);
    //         println!("   Side: {:?}", order.side);
    //         println!("   Type: {:?}", order.type_);
    //         println!("   Amount: {}", order.amount);
    //         println!("   Price: {:?}", order.price);
    //     }
    //     Err(e) => println!("   Error: {}", e),
    // }

    // Create a perpetual futures market order
    // match futures_exchange_private.create_order(
    //     "ETH/USDT",
    //     "market",
    //     "sell",
    //     0.1,
    //     None
    // ).await {
    //     Ok(order) => {
    //         println!("   Futures market order created:");
    //         println!("   ID: {:?}", order.id);
    //         println!("   Symbol: {}", order.symbol);
    //         println!("   Side: {:?}", order.side);
    //         println!("   Type: {:?}", order.type_);
    //         println!("   Amount: {}", order.amount);
    //         println!("   Price: {:?}", order.price);
    //     }
    //     Err(e) => println!("   Error: {}", e),
    // }

    // Set position mode (hedge or one-way)
    // match futures_exchange_private.set_position_mode(true).await {
    //     Ok(result) => {
    //         println!("   Position mode set to hedge: {:?}", result);
    //     }
    //     Err(e) => println!("   Error: {}", e),
    // }

    // Fetch open positions
    // match futures_exchange_private.fetch_positions().await {
    //     Ok(positions) => {
    //         println!("   Current positions:");
    //         for position in positions {
    //             if position.size != 0.0 {
    //                 println!("   {}: {} @ {}",
    //                     position.symbol,
    //                     position.size,
    //                     position.entry_price.unwrap_or(0.0)
    //                 );
    //             }
    //         }
    //     }
    //     Err(e) => println!("   Error: {}", e),
    // }

    // Fetch account balance
    // match futures_exchange_private.fetch_balance().await {
    //     Ok(balance) => {
    //         println!("   Account balance:");
    //         for (currency, amount) in balance.free.iter() {
    //             if *amount > 0.0 {
    //                 println!("   {}: {} (free), {} (used)",
    //                     currency,
    //                     amount,
    //                     balance.used.get(currency).unwrap_or(&0.0)
    //                 );
    //             }
    //         }
    //     }
    //     Err(e) => println!("   Error: {}", e),
    // }

    // Cancel an order
    // match futures_exchange_private.cancel_order("12345", "BTC/USDT").await {
    //     Ok(order) => {
    //         println!("   Order cancelled: {:?}", order.id);
    //     }
    //     Err(e) => println!("   Error: {}", e),
    // }

    println!("   (Private futures API examples are commented out)");
    println!("   Set your credentials to test private futures endpoints");
    println!();

    // Example 8: Advanced features
    println!("8. Advanced features...");

    // Fetch multiple tickers
    let symbols = vec!["BTC/USDT", "ETH/USDT", "BNB/USDT"];
    println!("   Fetching multiple tickers...");
    for symbol in symbols {
        match futures_exchange
            .fetch_ticker(symbol, ccxt_core::types::TickerParams::default())
            .await
        {
            Ok(ticker) => {
                println!("   {}: {:?}", ticker.symbol, ticker.last);
            }
            Err(e) => println!("   {}: Error - {}", symbol, e),
        }
    }
    println!();

    // Example 9: WebSocket streams (commented out as they run continuously)
    /*
    println!("9. WebSocket ticker stream (press Ctrl+C to stop)...");
    futures_exchange.subscribe_ticker("BTC/USDT", |ticker| async move {
        println!("   Futures Ticker update: {} @ {:?}", ticker.symbol, ticker.last);
        Ok(())
    }).await?;

    // WebSocket order book stream
    println!("   Futures order book stream...");
    futures_exchange.subscribe_order_book("BTC/USDT", 10, |order_book| async move {
        if let (Some(best_bid), Some(best_ask)) = (order_book.bids.first(), order_book.asks.first()) {
            let spread = best_ask.price - best_bid.price;
            println!("   Order book update: {} @ {} (spread: {})",
                order_book.symbol, best_bid.price, spread);
        }
        Ok(())
    }).await?;

    // WebSocket trades stream
    println!("   Futures trades stream...");
    futures_exchange.subscribe_trades("BTC/USDT", |trades| async move {
        for trade in trades {
            println!("   Trade: {} {} @ {} ({})",
                trade.symbol, trade.amount, trade.price, trade.side);
        }
        Ok(())
    }).await?;
    */

    println!("=== Example completed ===");

    Ok(())
}
