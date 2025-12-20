//! HyperLiquid Exchange Example
//!
//! This example demonstrates basic usage of the HyperLiquid exchange implementation.
//!
//! Run with: cargo run --example hyperliquid_example

use ccxt_core::exchange::Exchange;
use ccxt_exchanges::hyperliquid::HyperLiquidBuilder;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    println!("=== HyperLiquid Exchange Example ===\n");

    // Create a public-only instance (no authentication)
    let exchange = HyperLiquidBuilder::new()
        .testnet(true) // Use testnet for safety
        .build()?;

    println!("Exchange: {} ({})", exchange.name(), exchange.id());
    println!("Testnet: {}", exchange.options().testnet);
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

    // Fetch BTC ticker
    println!("Fetching BTC/USDC:USDC ticker...");
    match exchange.fetch_ticker("BTC/USDC:USDC").await {
        Ok(ticker) => {
            println!("  Symbol: {}", ticker.symbol);
            if let Some(last) = ticker.last {
                println!("  Last Price: {}", last);
            }
            println!("  Timestamp: {}", ticker.timestamp);
        }
        Err(e) => println!("  Error: {}", e),
    }
    println!();

    // Fetch order book
    println!("Fetching BTC/USDC:USDC order book...");
    match exchange.fetch_order_book("BTC/USDC:USDC", Some(5)).await {
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
    println!("Fetching recent BTC/USDC:USDC trades...");
    match exchange.fetch_trades("BTC/USDC:USDC", Some(5)).await {
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

    // Display capabilities
    println!("Exchange Capabilities:");
    let caps = exchange.capabilities();
    println!("  fetch_markets: {}", caps.fetch_markets());
    println!("  fetch_ticker: {}", caps.fetch_ticker());
    println!("  fetch_order_book: {}", caps.fetch_order_book());
    println!("  fetch_trades: {}", caps.fetch_trades());
    println!("  fetch_ohlcv: {}", caps.fetch_ohlcv());
    println!("  create_order: {}", caps.create_order());
    println!("  cancel_order: {}", caps.cancel_order());
    println!("  fetch_balance: {}", caps.fetch_balance());
    println!("  fetch_positions: {}", caps.fetch_positions());
    println!("  set_leverage: {}", caps.set_leverage());
    println!();

    // Example with authentication (commented out for safety)
    /*
    println!("=== Authenticated Example ===\n");

    // WARNING: Never commit real private keys!
    let private_key = std::env::var("HYPERLIQUID_PRIVATE_KEY")
        .expect("Set HYPERLIQUID_PRIVATE_KEY environment variable");

    let auth_exchange = HyperLiquidBuilder::new()
        .private_key(&private_key)
        .testnet(true)
        .build()?;

    println!("Wallet Address: {}", auth_exchange.wallet_address().unwrap());

    // Fetch balance
    println!("Fetching balance...");
    let balance = auth_exchange.fetch_balance().await?;
    if let Some(usdc) = balance.get("USDC") {
        println!("  USDC Balance:");
        println!("    Total: {}", usdc.total);
        println!("    Free: {}", usdc.free);
        println!("    Used: {}", usdc.used);
    }

    // Fetch positions
    println!("Fetching positions...");
    let positions = auth_exchange.fetch_positions(None).await?;
    println!("  Found {} positions", positions.len());
    for pos in &positions {
        println!(
            "    {} - Size: {:?}, Entry: {:?}, PnL: {:?}",
            pos.symbol, pos.contracts, pos.entry_price, pos.unrealized_pnl
        );
    }
    */

    println!("=== Example Complete ===");

    Ok(())
}
