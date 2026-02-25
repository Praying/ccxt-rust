//! OKX Market Data API Example
//!
//! Demonstrates how to use OKX exchange market data APIs, including:
//! 1. `fetch_markets` - Get all supported markets
//! 2. `fetch_ticker` - Get specific ticker
//! 3. `fetch_tickers` - Get all tickers
//! 4. `fetch_order_book` - Get order book
//! 5. `fetch_trades` - Get recent trades
//! 6. `fetch_ohlcv` - Get candlestick data
//!
//! # Usage
//!
//! ```bash
//! cargo run --example okx_market_data_example
//! ```

// Allow clippy warnings for example code
#![allow(clippy::disallowed_methods)]

use ccxt_core::ExchangeConfig;
use ccxt_core::types::ohlcv_request::OhlcvRequest;
use ccxt_exchanges::okx::Okx;

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    println!("=== OKX Market Data API Example ===\n");

    // Initialize OKX exchange (public only, no credentials needed for market data)
    let config = ExchangeConfig::default();
    let exchange = Okx::new(config)?;

    // 1. Fetch Markets
    println!("1. 【fetch_markets】Get all supported markets");
    println!("   Fetching markets...");
    let markets = exchange.fetch_markets().await?;
    println!("   ✓ Found {} markets", markets.len());

    // Display some sample markets
    println!("   Sample markets:");
    for market in markets.values().take(5) {
        println!(
            "     - {} ({}) Type: {:?} Active: {}",
            market.symbol, market.id, market.market_type, market.active
        );
    }
    println!();

    // Load markets for subsequent calls (required for correct symbol resolution)
    exchange.load_markets(false).await?;

    // 2. Fetch Ticker
    println!("2. 【fetch_ticker】Get BTC/USDT ticker");
    match exchange.fetch_ticker("BTC/USDT").await {
        Ok(ticker) => {
            println!("   ✓ Successfully fetched BTC/USDT ticker");
            println!("     - Symbol: {}", ticker.symbol);
            println!("     - Last Price: {:?}", ticker.last);
            println!("     - High (24h): {:?}", ticker.high);
            println!("     - Low (24h): {:?}", ticker.low);
            println!("     - Volume (24h): {:?}", ticker.base_volume);
            println!("     - Timestamp: {}", ticker.timestamp);
        }
        Err(e) => println!("   ✗ Error: {}", e),
    }
    println!();

    // 3. Fetch Order Book
    println!("3. 【fetch_order_book】Get BTC/USDT order book");
    match exchange.fetch_order_book("BTC/USDT", Some(5)).await {
        Ok(order_book) => {
            println!("   ✓ Successfully fetched order book");
            println!("     - Bids: {} levels", order_book.bids.len());
            println!("     - Asks: {} levels", order_book.asks.len());

            if let Some(best_bid) = order_book.bids.first() {
                println!("     - Best Bid: {} @ {}", best_bid.amount, best_bid.price);
            }
            if let Some(best_ask) = order_book.asks.first() {
                println!("     - Best Ask: {} @ {}", best_ask.amount, best_ask.price);
            }
        }
        Err(e) => println!("   ✗ Error: {}", e),
    }
    println!();

    // 4. Fetch Recent Trades
    println!("4. 【fetch_trades】Get BTC/USDT recent trades");
    match exchange.fetch_trades("BTC/USDT", Some(5)).await {
        Ok(trades) => {
            println!("   ✓ Successfully fetched {} trades", trades.len());
            for (i, trade) in trades.iter().take(5).enumerate() {
                println!(
                    "     {}. {:?} {} @ {} (Time: {})",
                    i + 1,
                    trade.side,
                    trade.amount,
                    trade.price,
                    trade.datetime.as_deref().unwrap_or("N/A")
                );
            }
        }
        Err(e) => println!("   ✗ Error: {}", e),
    }
    println!();

    // 5. Fetch OHLCV
    println!("5. 【fetch_ohlcv】Get BTC/USDT 1h candles");
    let request = OhlcvRequest::builder()
        .symbol("BTC/USDT")
        .timeframe("1h")
        .limit(5)
        .build()?;

    match exchange.fetch_ohlcv_v2(request).await {
        Ok(ohlcvs) => {
            println!("   ✓ Successfully fetched {} candles", ohlcvs.len());
            for (i, ohlcv) in ohlcvs.iter().enumerate() {
                println!(
                    "     {}. Time: {}, Open: {}, High: {}, Low: {}, Close: {}, Vol: {}",
                    i + 1,
                    ohlcv.timestamp,
                    ohlcv.open,
                    ohlcv.high,
                    ohlcv.low,
                    ohlcv.close,
                    ohlcv.volume
                );
            }
        }
        Err(e) => println!("   ✗ Error: {}", e),
    }
    println!();

    println!("=== Example Complete ===");

    Ok(())
}
