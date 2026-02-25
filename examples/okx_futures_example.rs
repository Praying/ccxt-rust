//! OKX Futures (Swap) Example
//!
//! Demonstrates how to use OKX for perpetual swaps (futures):
//! 1. Initialize exchange with `DefaultType::Swap`
//! 2. Fetch swap markets
//! 3. Fetch swap ticker
//! 4. Fetch swap order book
//!
//! # Usage
//!
//! ```bash
//! cargo run --example okx_futures_example
//! ```

// Allow clippy warnings for example code
#![allow(clippy::disallowed_methods)]

use ccxt_core::DefaultType;
use ccxt_exchanges::okx::OkxBuilder;

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    println!("=== OKX Futures (Swap) Example ===\n");

    // Initialize OKX exchange for SWAP (Perpetual Futures)
    // This configures the exchange to default to 'SWAP' instrument type
    let exchange = OkxBuilder::new().default_type(DefaultType::Swap).build()?;

    // 1. Fetch Markets (Swaps)
    println!("1. 【fetch_markets】Get all swap markets");
    println!("   Fetching swap markets...");
    let markets = exchange.fetch_markets().await?;
    println!("   ✓ Found {} swap markets", markets.len());

    // Display sample swap markets (looking for USDT margined)
    let swaps: Vec<_> = markets
        .values()
        .filter(|m| m.quote == "USDT" && m.linear.unwrap_or(false))
        .take(5)
        .collect();

    println!("   Sample USDT-margined Swaps:");
    for market in swaps {
        println!(
            "     - {} ({}) Linear: {:?} Settle: {:?}",
            market.symbol, market.id, market.linear, market.settle
        );
    }
    println!();

    // Load markets for subsequent calls
    exchange.load_markets(false).await?;

    // 2. Fetch Swap Ticker
    // For OKX swaps, symbols are usually like "BTC/USDT:USDT" (unified) or "BTC-USDT-SWAP" (raw)
    // The parser handles "BTC/USDT:USDT".
    let symbol = "BTC/USDT:USDT";
    println!("2. 【fetch_ticker】Get {} ticker", symbol);
    match exchange.fetch_ticker(symbol).await {
        Ok(ticker) => {
            println!("   ✓ Successfully fetched ticker");
            println!("     - Symbol: {}", ticker.symbol);
            println!("     - Last Price: {:?}", ticker.last);
            println!("     - Mark Price: {:?}", ticker.mark_price); // OKX tickers often include mark price
            println!("     - Index Price: {:?}", ticker.index_price);
            println!("     - Open Interest: {:?}", ticker.open_interest);
        }
        Err(e) => {
            println!("   ✗ Error: {}", e);
            // Fallback to simpler symbol if specific swap symbol fails (though load_markets should fix this)
            println!("   Trying BTC/USDT...");
            match exchange.fetch_ticker("BTC/USDT").await {
                Ok(ticker) => println!("   ✓ Fetched BTC/USDT: {:?}", ticker.last),
                Err(e2) => println!("   ✗ Error: {}", e2),
            }
        }
    }
    println!();

    // 3. Fetch Swap Order Book
    println!("3. 【fetch_order_book】Get {} order book", symbol);
    match exchange.fetch_order_book(symbol, Some(5)).await {
        Ok(order_book) => {
            println!("   ✓ Successfully fetched order book");
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

    println!("=== Example Complete ===");

    Ok(())
}
