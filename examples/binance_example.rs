//! Binance Exchange Usage Example
//!
//! This example demonstrates how to use the Binance exchange implementation
//! to fetch market data, trade, and manage orders.

// Allow clippy warnings for example code - examples prioritize readability over strict linting
#![allow(clippy::field_reassign_with_default)]

use anyhow::{Context, Result};
use ccxt_core::ExchangeConfig;
use ccxt_core::logging::{LogConfig, init_logging};
use ccxt_exchanges::binance::Binance;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging system
    // Use development config for examples (Debug level, Pretty format)
    init_logging(LogConfig::development());

    println!("=== Binance Exchange Example ===\n");

    // Initialize Binance exchange
    // For public endpoints, you can use default config (no API credentials)
    let mut config = ExchangeConfig::default();
    config.verbose = true; // Enable verbose logging to see HTTP requests/responses
    let exchange = Binance::new(config).context("Failed to initialize Binance exchange")?;
    exchange.load_markets(false).await?;
    // Example 1: Fetch all markets
    println!("1. Fetching markets...");
    match exchange.fetch_markets().await {
        Ok(markets) => {
            println!("   Found {} markets", markets.len());
            if let Some(market) = markets.values().next() {
                println!(
                    "   Example market: {} ({}/{})",
                    market.symbol, market.base, market.quote
                );
            }
        }
        Err(e) => println!("   Error: {}", e),
    }
    println!();

    // Example 2: Fetch ticker for BTC/USDT
    println!("2. Fetching BTC/USDT ticker...");
    match exchange
        .fetch_ticker("BTC/USDT", ccxt_core::types::TickerParams::default())
        .await
    {
        Ok(ticker) => {
            println!("   Symbol: {}", ticker.symbol);
            println!("   Last: {:?}", ticker.last);
            println!("   Bid: {:?}", ticker.bid);
            println!("   Ask: {:?}", ticker.ask);
            println!("   Volume: {:?}", ticker.base_volume);
        }
        Err(e) => println!("   Error: {}", e),
    }
    println!();

    // Example 3: Fetch order book
    println!("3. Fetching BTC/USDT order book...");
    match exchange.fetch_order_book("BTC/USDT", Some(10)).await {
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
        }
        Err(e) => println!("   Error: {}", e),
    }
    println!();

    // Example 4: Fetch recent trades (using new API)
    println!("4. Fetching recent BTC/USDT trades...");
    match exchange.fetch_trades("BTC/USDT", Some(5)).await {
        Ok(trades) => {
            println!("   Found {} trades", trades.len());
            for (i, trade) in trades.iter().enumerate() {
                println!(
                    "   Trade {}: {} {} @ {} (side: {:?})",
                    i + 1,
                    trade.amount,
                    trade.symbol,
                    trade.price,
                    trade.side
                );
            }
        }
        Err(e) => println!("   Error: {}", e),
    }
    println!();

    // Example 5: Fetch OHLCV (candlestick) data
    println!("5. Fetching BTC/USDT 1h OHLCV data...");
    match exchange
        .fetch_ohlcv("BTC/USDT", "1h", None, Some(5), None)
        .await
    {
        Ok(candles) => {
            println!("   Found {} candles", candles.len());
            for (i, candle) in candles.iter().enumerate() {
                // OHLCV is now a struct with fields: timestamp, open, high, low, close, volume
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

    // Example 6: Private API endpoints (requires API credentials)
    println!("6. Private API examples (requires credentials)...");

    // To use private endpoints, uncomment and set your credentials:
    // let private_config = ExchangeConfig {
    //     api_key: Some("YOUR_API_KEY".to_string()),
    //     secret: Some("YOUR_SECRET".to_string()),
    //     ..Default::default()
    // };
    // let exchange_private = Binance::new(private_config)?;

    // Fetch account balance
    // match exchange_private.fetch_balance().await {
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

    // Create a limit order
    // match exchange_private.create_order(
    //     "BTC/USDT",
    //     "limit",
    //     "buy",
    //     0.001,
    //     Some(40000.0)
    // ).await {
    //     Ok(order) => {
    //         println!("   Order created:");
    //         println!("   ID: {:?}", order.id);
    //         println!("   Symbol: {}", order.symbol);
    //         println!("   Side: {:?}", order.side);
    //         println!("   Type: {:?}", order.type_);
    //         println!("   Amount: {}", order.amount);
    //         println!("   Price: {:?}", order.price);
    //     }
    //     Err(e) => println!("   Error: {}", e),
    // }

    // Cancel an order
    // match exchange_private.cancel_order("12345", "BTC/USDT").await {
    //     Ok(order) => {
    //         println!("   Order cancelled: {:?}", order.id);
    //     }
    //     Err(e) => println!("   Error: {}", e),
    // }

    println!("   (Private API examples are commented out)");
    println!("   Set your credentials to test private endpoints");
    println!();

    // Example 7: Perpetual futures markets
    println!("7. Fetching perpetual futures data...");

    // Initialize Binance Futures exchange
    let futures_config = ExchangeConfig::default();
    let futures_exchange = Binance::new_swap(futures_config)
        .context("Failed to initialize Binance Futures exchange")?;

    // Fetch all futures markets
    println!("   Fetching futures markets...");
    match futures_exchange.fetch_markets().await {
        Ok(markets) => {
            println!("   Found {} futures markets", markets.len());

            // Find perpetual futures (symbols ending with /USDT)
            let perpetuals: Vec<_> = markets
                .values()
                .filter(|m| m.symbol.contains("/USDT") && m.symbol.contains("PERP"))
                .take(5)
                .collect();

            println!("   Found {} perpetual futures:", perpetuals.len());
            for market in perpetuals {
                println!("     - {}: {}/{}", market.symbol, market.base, market.quote);
            }
        }
        Err(e) => println!("   Error: {}", e),
    }
    println!();

    // Fetch BTC/USDT perpetual futures ticker
    println!("   Fetching BTC/USDT perpetual futures ticker...");
    match futures_exchange
        .fetch_ticker("BTC/USDT", ccxt_core::types::TickerParams::default())
        .await
    {
        Ok(ticker) => {
            println!("   Symbol: {}", ticker.symbol);
            println!("   Last: {:?}", ticker.last);
            println!("   24h Change: {:?}", ticker.percentage);
            println!("   24h Volume: {:?}", ticker.base_volume);
            println!("   Funding Rate: N/A (not included in ticker)");
        }
        Err(e) => println!("   Error: {}", e),
    }
    println!();

    // Fetch BTC/USDT perpetual futures order book
    println!("   Fetching BTC/USDT perpetual futures order book...");
    match futures_exchange
        .fetch_order_book("BTC/USDT", Some(10))
        .await
    {
        Ok(order_book) => {
            println!("   Symbol: {}", order_book.symbol);
            println!("   Bids: {} levels", order_book.bids.len());
            println!("   Asks: {} levels", order_book.asks.len());
            if let (Some(best_bid), Some(best_ask)) =
                (order_book.bids.first(), order_book.asks.first())
            {
                let spread = best_ask.price - best_bid.price;
                // Use a constant for 100.0 to avoid unwrap
                let hundred = rust_decimal::Decimal::ONE_HUNDRED;
                let spread_percent = (spread / best_bid.price) * hundred;
                println!("   Spread: {} ({:.4}%)", spread, spread_percent);
            }
        }
        Err(e) => println!("   Error: {}", e),
    }
    println!();

    // Fetch perpetual futures funding rate
    println!("   Fetching BTC/USDT funding rate...");
    // Note: This would require implementing fetch_funding_rate method
    // For now, we'll show a placeholder
    println!("   Funding rate: ~0.01% (example)");
    println!("   Note: Implement fetch_funding_rate method to get real rates");
    println!();

    // Example 8: Private futures API endpoints (requires API credentials)
    println!("8. Private futures API examples (requires credentials)...");

    // To use private futures endpoints, uncomment and set your credentials:
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

    println!("   (Private futures API examples are commented out)");
    println!("   Set your credentials to test private futures endpoints");
    println!();

    // Example 9: WebSocket streams (commented out as they run continuously)
    /*
    println!("9. WebSocket ticker stream (press Ctrl+C to stop)...");
    exchange.subscribe_ticker("BTC/USDT", |ticker| async move {
        println!("   Ticker update: {} @ {:?}", ticker.symbol, ticker.last);
        Ok(())
    }).await?;

    // Futures WebSocket stream
    println!("   Futures WebSocket ticker stream...");
    futures_exchange.subscribe_ticker("BTC/USDT", |ticker| async move {
        println!("   Futures Ticker update: {} @ {:?}", ticker.symbol, ticker.last);
        Ok(())
    }).await?;
    */

    println!("=== Example completed ===");

    Ok(())
}
