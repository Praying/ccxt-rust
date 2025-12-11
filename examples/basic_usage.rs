//! Basic usage example for CCXT Rust
//!
//! This example demonstrates creating basic data structures
//! and working with the type system.

use anyhow::Result;
use ccxt_core::prelude::*;
// Logging is optional for basic example
// use ccxt_core::logging::{init_logging, LogConfig};
use rust_decimal_macros::dec;

fn main() -> Result<()> {
    // Initialize logging system (optional for this basic example)
    // Uncomment to see debug logs:
    // init_logging(LogConfig::development());

    println!("=== CCXT Rust Basic Usage Example ===\n");

    // Create a spot market
    let market = Market::new_spot(
        "btcusdt".to_string(),
        "BTC/USDT".to_string(),
        "BTC".to_string(),
        "USDT".to_string(),
    );
    println!("Created Market:");
    println!("  Symbol: {}", market.symbol);
    println!("  Type: {:?}", market.market_type);
    println!("  Base: {}", market.base);
    println!("  Quote: {}", market.quote);
    println!();

    // Create a limit buy order
    // Order::new(id, symbol, order_type, side, amount, price, status)
    let order = Order::new(
        "order-123".to_string(), // id
        "BTC/USDT".to_string(),  // symbol
        OrderType::Limit,        // order_type
        OrderSide::Buy,          // side
        dec!(0.1),               // amount
        Some(dec!(50000.0)),     // price (Option<Decimal>)
        OrderStatus::Open,       // status
    );
    println!("Created Order:");
    println!("  ID: {}", order.id);
    println!("  Symbol: {}", order.symbol);
    println!("  Type: {:?}", order.order_type);
    println!("  Side: {:?}", order.side);
    println!("  Price: {:?}", order.price); // 使用 {:?} 格式化 Option<Decimal>
    println!("  Amount: {}", order.amount);
    println!("  Status: {:?}", order.status);
    println!();

    // Create a ticker
    let mut ticker = Ticker::new(
        "BTC/USDT".to_string(),
        chrono::Utc::now().timestamp_millis(),
    );
    ticker.bid = Some(dec!(49950.0).into());
    ticker.ask = Some(dec!(50050.0).into());
    ticker.last = Some(dec!(50000.0).into());
    ticker.base_volume = Some(dec!(1234.5).into());

    println!("Created Ticker:");
    println!("  Symbol: {}", ticker.symbol);
    println!("  Bid: {:?}", ticker.bid);
    println!("  Ask: {:?}", ticker.ask);
    println!("  Last: {:?}", ticker.last);
    if let Some(spread) = ticker.spread() {
        println!("  Spread: {}", spread);
    }
    println!();

    // Create an order book
    let mut orderbook = OrderBook::new(
        "BTC/USDT".to_string(),
        chrono::Utc::now().timestamp_millis(),
    );

    orderbook.bids = vec![
        OrderBookEntry::new(dec!(50000.0).into(), dec!(1.0).into()),
        OrderBookEntry::new(dec!(49900.0).into(), dec!(2.0).into()),
        OrderBookEntry::new(dec!(49800.0).into(), dec!(1.5).into()),
    ];

    orderbook.asks = vec![
        OrderBookEntry::new(dec!(50100.0).into(), dec!(1.0).into()),
        OrderBookEntry::new(dec!(50200.0).into(), dec!(2.0).into()),
        OrderBookEntry::new(dec!(50300.0).into(), dec!(1.5).into()),
    ];

    println!("Created OrderBook:");
    println!("  Symbol: {}", orderbook.symbol);
    if let Some(best_bid) = orderbook.best_bid() {
        println!("  Best Bid: {} @ {}", best_bid.amount, best_bid.price);
    }
    if let Some(best_ask) = orderbook.best_ask() {
        println!("  Best Ask: {} @ {}", best_ask.amount, best_ask.price);
    }
    if let Some(spread) = orderbook.spread() {
        println!("  Spread: {}", spread);
    }
    println!("  Total Bid Volume: {}", orderbook.bid_volume());
    println!("  Total Ask Volume: {}", orderbook.ask_volume());
    println!();

    // Create a trade
    let trade = Trade::new(
        "BTC/USDT".to_string(),
        OrderSide::Buy,
        dec!(50000.0).into(),
        dec!(0.5).into(),
        chrono::Utc::now().timestamp_millis(),
    );
    println!("Created Trade:");
    println!("  Symbol: {}", trade.symbol);
    println!("  Side: {:?}", trade.side);
    println!("  Price: {}", trade.price);
    println!("  Amount: {}", trade.amount);
    println!("  Cost: {:?}", trade.cost);
    println!();

    // Create OHLCV data
    let ohlcv = Ohlcv::new(
        chrono::Utc::now().timestamp_millis(),
        dec!(49000.0).into(), // open
        dec!(51000.0).into(), // high
        dec!(48500.0).into(), // low
        dec!(50000.0).into(), // close
        dec!(1234.5).into(),  // volume
    );
    println!("Created OHLCV:");
    println!("  Open: {}", ohlcv.open);
    println!("  High: {}", ohlcv.high);
    println!("  Low: {}", ohlcv.low);
    println!("  Close: {}", ohlcv.close);
    println!("  Volume: {}", ohlcv.volume);
    println!();

    // Demonstrate timeframe conversion
    println!("Timeframe Conversions:");
    for timeframe in [Timeframe::M1, Timeframe::H1, Timeframe::D1] {
        println!(
            "  {} = {} ms = {} seconds",
            timeframe,
            timeframe.as_millis(),
            timeframe.as_seconds()
        );
    }
    println!();

    // Demonstrate error handling
    println!("Error Handling:");
    let err = Error::market_not_found("INVALID/PAIR");
    println!("  Market not found error: {}", err);

    let auth_err = Error::authentication("Invalid API key");
    println!("  Authentication error: {}", auth_err);
    println!();

    println!("=== Example completed successfully! ===");
    Ok(())
}
