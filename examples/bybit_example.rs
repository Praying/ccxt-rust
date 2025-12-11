//! Bybit Exchange Example
//!
//! This example demonstrates basic usage of the Bybit exchange implementation.
//!
//! Run with: cargo run --example bybit_example

use ccxt_exchanges::bybit::BybitBuilder;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    println!("=== Bybit Exchange Example ===\n");

    // Create a public-only instance (no authentication)
    let exchange = BybitBuilder::new()
        .testnet(true) // Use testnet environment for safety
        .build()?;

    println!("Exchange: {} ({})", exchange.name(), exchange.id());
    println!("Version: {}", exchange.version());
    println!("Testnet: {}", exchange.options().testnet);
    println!("Account Type: {}", exchange.options().account_type);
    println!();

    // Fetch and display markets
    println!("Fetching markets...");
    let markets = exchange.fetch_markets().await?;
    println!("Found {} markets\n", markets.len());

    // Display first 5 markets
    println!("Sample markets:");
    for market in markets.iter().take(5) {
        println!(
            "  {} - Base: {}, Quote: {}, Active: {}",
            market.symbol, market.base, market.quote, market.active
        );
    }
    println!();

    // Load markets for subsequent calls
    exchange.load_markets(false).await?;

    // Fetch BTC/USDT ticker
    println!("Fetching BTC/USDT ticker...");
    match exchange.fetch_ticker("BTC/USDT").await {
        Ok(ticker) => {
            println!("  Symbol: {}", ticker.symbol);
            if let Some(last) = ticker.last {
                println!("  Last Price: {}", last);
            }
            if let Some(high) = ticker.high {
                println!("  24h High: {}", high);
            }
            if let Some(low) = ticker.low {
                println!("  24h Low: {}", low);
            }
            println!("  Timestamp: {}", ticker.timestamp);
        }
        Err(e) => println!("  Error: {}", e),
    }
    println!();

    // Fetch order book
    println!("Fetching BTC/USDT order book...");
    match exchange.fetch_order_book("BTC/USDT", Some(5)).await {
        Ok(orderbook) => {
            println!("  Bids: {} levels", orderbook.bids.len());
            println!("  Asks: {} levels", orderbook.asks.len());
            if let Some(best_bid) = orderbook.bids.first() {
                println!("  Best Bid: {} @ {}", best_bid.amount, best_bid.price);
            }
            if let Some(best_ask) = orderbook.asks.first() {
                println!("  Best Ask: {} @ {}", best_ask.amount, best_ask.price);
            }
        }
        Err(e) => println!("  Error: {}", e),
    }
    println!();

    // Fetch recent trades
    println!("Fetching recent BTC/USDT trades...");
    match exchange.fetch_trades("BTC/USDT", Some(5)).await {
        Ok(trades) => {
            println!("  Found {} trades", trades.len());
            for trade in trades.iter().take(3) {
                println!(
                    "    {:?} {} @ {} ({})",
                    trade.side, trade.amount, trade.price, trade.timestamp
                );
            }
        }
        Err(e) => println!("  Error: {}", e),
    }
    println!();

    // Display exchange info
    println!("Exchange Info:");
    println!("  ID: {}", exchange.id());
    println!("  Name: {}", exchange.name());
    println!("  Version: {}", exchange.version());
    println!("  Certified: {}", exchange.certified());
    println!("  Pro (WebSocket): {}", exchange.pro());
    println!("  Rate Limit: {} req/s", exchange.rate_limit());
    println!();

    // Display supported timeframes
    println!("Supported Timeframes:");
    let timeframes = exchange.timeframes();
    let mut tf_list: Vec<_> = timeframes.keys().collect();
    tf_list.sort();
    for tf in tf_list.iter().take(6) {
        if let Some(value) = timeframes.get(*tf) {
            println!("  {} -> {}", tf, value);
        }
    }
    println!("  ... and {} more", timeframes.len().saturating_sub(6));
    println!();

    // Example with authentication (commented out for safety)
    /*
    println!("=== Authenticated Example ===\n");

    // WARNING: Never commit real API credentials!
    let api_key = std::env::var("BYBIT_API_KEY")
        .expect("Set BYBIT_API_KEY environment variable");
    let secret = std::env::var("BYBIT_SECRET")
        .expect("Set BYBIT_SECRET environment variable");

    let auth_exchange = BybitBuilder::new()
        .api_key(&api_key)
        .secret(&secret)
        .testnet(true) // Use testnet environment
        .recv_window(10000) // Custom receive window
        .build()?;

    // Fetch balance
    println!("Fetching balance...");
    let balance = auth_exchange.fetch_balance().await?;
    if let Some(usdt) = balance.get("USDT") {
        println!("  USDT Balance:");
        println!("    Total: {}", usdt.total);
        println!("    Free: {}", usdt.free);
        println!("    Used: {}", usdt.used);
    }

    // Fetch open orders
    println!("Fetching open orders...");
    let orders = auth_exchange.fetch_open_orders(Some("BTC/USDT"), None, None).await?;
    println!("  Found {} open orders", orders.len());
    for order in &orders {
        println!(
            "    {} - {:?} {:?} {} @ {:?}",
            order.id, order.side, order.order_type, order.amount, order.price
        );
    }
    */

    println!("=== Example Complete ===");

    Ok(())
}
