//! Binance Advanced Features Example
//!
//! Demonstrates usage of newly added API methods.
//!
//! Note: Some methods like trading fee methods return raw JSON in the new modular structure.

use ccxt_core::{ExchangeConfig, Result};
use ccxt_exchanges::binance::Binance;

#[tokio::main]
async fn main() -> Result<()> {
    let config = ExchangeConfig {
        api_key: std::env::var("BINANCE_API_KEY").ok(),
        secret: std::env::var("BINANCE_SECRET").ok(),
        ..Default::default()
    };

    let binance = Binance::new(config)?;

    // 1. Load market data
    println!("=== Load Market Data ===");
    let markets = binance.fetch_markets().await?;
    println!("Loaded {} markets", markets.len());

    // 2. Get all currency information (requires API key)
    if binance.base().config.api_key.is_some() {
        println!("\n=== Get Currency Information ===");
        match binance.fetch_currencies().await {
            Ok(currencies) => {
                println!("Fetched {} currencies", currencies.len());
                for (i, currency) in currencies.iter().take(5).enumerate() {
                    println!("{}. {}", i + 1, currency.code);
                }
            }
            Err(e) => println!("Failed to fetch currencies: {}", e),
        }

        // 3. Get trading fees (returns HashMap<String, FeeTradingFee>)
        println!("\n=== Get Trading Fees ===");
        match binance.fetch_trading_fees(None, None).await {
            Ok(fees) => {
                println!("Fetched {} trading fees", fees.len());
                for (symbol, fee) in fees.iter().take(5) {
                    println!("  {}: maker={}, taker={}", symbol, fee.maker, fee.taker);
                }
            }
            Err(e) => println!("Failed to fetch trading fees: {}", e),
        }

        // 4. Get single trading pair fee (returns FeeTradingFee)
        println!("\n=== Get Single Trading Pair Fee ===");
        match binance.fetch_trading_fee("BTC/USDT", None).await {
            Ok(fee) => {
                println!("BTC/USDT fee: maker={}, taker={}", fee.maker, fee.taker);
            }
            Err(e) => println!("Failed to fetch fee: {}", e),
        }

        // 5. Get all orders
        println!("\n=== Get All Orders ===");
        match binance.fetch_orders(Some("BTC/USDT"), None, Some(10)).await {
            Ok(orders) => {
                println!("Fetched {} orders", orders.len());
                for (i, order) in orders.iter().enumerate() {
                    println!(
                        "{}. Order ID: {}, Status: {:?}, Type: {:?}",
                        i + 1,
                        order.id,
                        order.status,
                        order.order_type
                    );
                }
            }
            Err(e) => println!("Failed to fetch orders: {}", e),
        }

        // 6. Get closed orders
        println!("\n=== Get Closed Orders ===");
        match binance
            .fetch_closed_orders(Some("BTC/USDT"), None, Some(10))
            .await
        {
            Ok(orders) => {
                println!("Fetched {} closed orders", orders.len());
                for (i, order) in orders.iter().enumerate() {
                    println!(
                        "{}. Order ID: {}, Status: {:?}, Filled: {:?}",
                        i + 1,
                        order.id,
                        order.status,
                        order.filled
                    );
                }
            }
            Err(e) => println!("Failed to fetch closed orders: {}", e),
        }

        // 7. Batch cancel orders example (demonstration only, not executed)
        println!("\n=== Batch Cancel Orders (example code, not executed) ===");
        println!("// Cancel multiple orders");
        println!("let order_ids = vec![\"12345\".to_string(), \"67890\".to_string()];");
        println!("let result = binance.cancel_orders(order_ids, \"BTC/USDT\").await?;");

        // 8. Cancel all orders example (demonstration only, not executed)
        println!("\n=== Cancel All Orders (example code, not executed) ===");
        println!("// Cancel all orders for a specific trading pair");
        println!("let result = binance.cancel_all_orders(\"BTC/USDT\").await?;");
    } else {
        println!(
            "\nNote: Set BINANCE_API_KEY and BINANCE_SECRET environment variables to test private APIs"
        );
    }

    println!("\n=== Example Complete ===");
    Ok(())
}
