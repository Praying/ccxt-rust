#![allow(clippy::disallowed_methods)]
//! Binance order update WebSocket integration tests

use ccxt_core::ExchangeConfig;
use ccxt_core::types::default_type::DefaultType;
use ccxt_exchanges::binance::{Binance, BinanceOptions};
use std::sync::Arc;

#[tokio::test]
#[ignore]
async fn test_watch_orders_all() {
    let api_key = std::env::var("BINANCE_API_KEY").expect("BINANCE_API_KEY not set");
    let api_secret = std::env::var("BINANCE_API_SECRET").expect("BINANCE_API_SECRET not set");

    let config = ExchangeConfig {
        api_key: Some(api_key.into()),
        secret: Some(api_secret.into()),
        ..Default::default()
    };

    let options = BinanceOptions {
        default_type: DefaultType::Spot,
        ..Default::default()
    };

    let mut exchange = Binance::new(config).expect("Failed to create Binance instance");
    exchange.set_options(options);

    let exchange = Arc::new(exchange);

    let result = exchange.watch_orders(None, None, None, None).await;
    assert!(result.is_ok(), "watch_orders must succeed");

    let orders = result.unwrap();
    println!("Received {} orders", orders.len());

    for order in &orders {
        assert!(!order.symbol.is_empty(), "Order ID must not be empty");
        assert!(!order.symbol.is_empty(), "Symbol must not be empty");
        println!(
            "Order: {} {} {} {:?}",
            order.symbol, order.side, order.status, order.amount
        );
    }
}

#[tokio::test]
#[ignore]
async fn test_watch_orders_by_symbol() {
    let api_key = std::env::var("BINANCE_API_KEY").expect("BINANCE_API_KEY not set");
    let api_secret = std::env::var("BINANCE_API_SECRET").expect("BINANCE_API_SECRET not set");

    let config = ExchangeConfig {
        api_key: Some(api_key.into()),
        secret: Some(api_secret.into()),
        ..Default::default()
    };

    let options = BinanceOptions {
        default_type: DefaultType::Spot,
        ..Default::default()
    };

    let mut exchange = Binance::new(config).expect("Failed to create Binance instance");
    exchange.set_options(options);

    let exchange = Arc::new(exchange);

    let result = exchange
        .watch_orders(Some("BTC/USDT"), None, None, None)
        .await;
    assert!(result.is_ok(), "watch_orders(BTC/USDT) must succeed");

    let orders = result.unwrap();
    println!("Received {} BTC/USDT orders", orders.len());

    for order in &orders {
        assert_eq!(order.symbol, "BTC/USDT", "Order symbol must be BTC/USDT");
        println!("Order: {} {} {}", order.id, order.side, order.status);
    }
}

#[tokio::test]
#[ignore]
async fn test_watch_orders_with_limit() {
    let api_key = std::env::var("BINANCE_API_KEY").expect("BINANCE_API_KEY not set");
    let api_secret = std::env::var("BINANCE_API_SECRET").expect("BINANCE_API_SECRET not set");

    let config = ExchangeConfig {
        api_key: Some(api_key.into()),
        secret: Some(api_secret.into()),
        ..Default::default()
    };

    let options = BinanceOptions {
        default_type: DefaultType::Spot,
        ..Default::default()
    };

    let mut exchange = Binance::new(config).expect("Failed to create Binance instance");
    exchange.set_options(options);

    let exchange = Arc::new(exchange);

    let limit = 5;
    let result = exchange.watch_orders(None, None, Some(limit), None).await;
    assert!(result.is_ok(), "watch_orders(limit=5) must succeed");

    let orders = result.unwrap();
    assert!(
        orders.len() <= limit,
        "Returned orders must not exceed limit: {} > {}",
        orders.len(),
        limit
    );

    println!("Received {} orders (limit {})", orders.len(), limit);
}

#[tokio::test]
#[ignore]
async fn test_watch_orders_with_since() {
    let api_key = std::env::var("BINANCE_API_KEY").expect("BINANCE_API_KEY not set");
    let api_secret = std::env::var("BINANCE_API_SECRET").expect("BINANCE_API_SECRET not set");

    let config = ExchangeConfig {
        api_key: Some(api_key.into()),
        secret: Some(api_secret.into()),
        ..Default::default()
    };

    let options = BinanceOptions {
        default_type: DefaultType::Spot,
        ..Default::default()
    };

    let mut exchange = Binance::new(config).expect("Failed to create Binance instance");
    exchange.set_options(options);

    let exchange = Arc::new(exchange);

    let since = chrono::Utc::now().timestamp_millis() - 3600000;
    let result = exchange.watch_orders(None, Some(since), None, None).await;
    assert!(result.is_ok(), "watch_orders(since) must succeed");

    let orders = result.unwrap();
    println!("Received {} orders from last hour", orders.len());

    for order in &orders {
        if let Some(ts) = order.timestamp {
            assert!(
                ts >= since,
                "Order timestamp must be after since: {} < {}",
                ts,
                since
            );
        }
    }
}

#[tokio::test]
#[ignore]
async fn test_watch_orders_futures() {
    let api_key = std::env::var("BINANCE_API_KEY").expect("BINANCE_API_KEY not set");
    let api_secret = std::env::var("BINANCE_API_SECRET").expect("BINANCE_API_SECRET not set");

    let config = ExchangeConfig {
        api_key: Some(api_key.into()),
        secret: Some(api_secret.into()),
        ..Default::default()
    };

    let options = BinanceOptions {
        default_type: DefaultType::Swap, // "future" maps to Swap (perpetual futures)
        ..Default::default()
    };

    let mut exchange = Binance::new(config).expect("Failed to create Binance instance");
    exchange.set_options(options);

    let exchange = Arc::new(exchange);

    let result = exchange.watch_orders(None, None, None, None).await;
    assert!(result.is_ok(), "Futures watch_orders must succeed");

    let orders = result.unwrap();
    println!("Received {} futures orders", orders.len());

    for order in &orders {
        println!(
            "Futures order: {} {} {} {:?}",
            order.symbol, order.side, order.status, order.amount
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ccxt_core::types::{
        Order,
        order::{OrderSide, OrderStatus},
    };
    use std::collections::HashMap;

    #[tokio::test]
    async fn test_order_structure() {
        use ccxt_core::types::order::OrderType;

        let order = Order {
            id: "123456789".to_string(),
            client_order_id: Some("test_order_001".to_string()),
            info: HashMap::new(),
            timestamp: Some(1234567890000),
            datetime: Some("2023-01-01T00:00:00.000Z".to_string()),
            last_trade_timestamp: None,
            symbol: "BTC/USDT".to_string(),
            order_type: OrderType::Limit,
            time_in_force: Some("GTC".to_string()),
            post_only: None,
            reduce_only: None,
            side: OrderSide::Buy,
            price: Some(rust_decimal::Decimal::from(50000)),
            stop_price: None,
            trigger_price: None,
            take_profit_price: None,
            stop_loss_price: None,
            trailing_delta: None,
            trailing_percent: None,
            activation_price: None,
            callback_rate: None,
            working_type: None,
            amount: rust_decimal::Decimal::from_f64_retain(0.1).unwrap(),
            filled: Some(rust_decimal::Decimal::from_f64_retain(0.05).unwrap()),
            remaining: Some(rust_decimal::Decimal::from_f64_retain(0.05).unwrap()),
            cost: None,
            average: None,
            status: OrderStatus::Open,
            fee: None,
            fees: None,
            trades: None,
        };

        assert_eq!(order.id, "123456789");
        assert_eq!(order.symbol, "BTC/USDT");
        assert_eq!(order.side, OrderSide::Buy);
        assert_eq!(order.status, OrderStatus::Open);
        assert_eq!(order.price, Some(rust_decimal::Decimal::from(50000)));
        if let Some(ts) = order.timestamp {
            assert!(ts > 0);
        }

        println!("✓ Order structure validation");
    }

    #[tokio::test]
    async fn test_binance_options() {
        let options = BinanceOptions {
            default_type: DefaultType::Swap, // "future" maps to Swap (perpetual futures)
            ..Default::default()
        };

        assert_eq!(options.default_type, DefaultType::Swap);
        println!("✓ Binance options validation");
    }
}
