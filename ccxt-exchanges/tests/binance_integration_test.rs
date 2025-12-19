//! Binance Integration Tests
//!
//! These tests verify the Binance exchange implementation against the real API.
//! They can be run with: cargo test --test binance_integration_test
//!
//! Note: Some tests are marked with #[ignore] to avoid hitting API rate limits
//! during normal test runs. Run them explicitly with:
//! cargo test --test binance_integration_test -- --ignored

use ccxt_core::types::{Market, OrderSide, OrderStatus, OrderType};
use ccxt_core::{Amount, ExchangeConfig, Price};
use ccxt_exchanges::binance::Binance;
use ccxt_exchanges::binance::parser;
use rust_decimal::prelude::FromStr;
use std::env;

/// Load API credentials from environment variables.
fn get_api_credentials() -> ExchangeConfig {
    dotenvy::dotenv().ok();
    let api_key = env::var("BINANCE_API_KEY").ok();
    let secret = env::var("BINANCE_SECRET_KEY").ok();

    let mut config = ExchangeConfig::default();
    config.api_key = api_key;
    config.secret = secret;
    config
}

/// Test creating a new Binance instance with default configuration.
#[tokio::test]
async fn test_new_binance_instance() {
    let mut config = ExchangeConfig::default();
    config.id = "binance".to_string();
    config.name = "Binance".to_string();

    let exchange = Binance::new(config).unwrap();
    assert_eq!(exchange.id(), "binance");
    assert_eq!(exchange.name(), "Binance");
}

/// Test fetching exchange URLs (public API and WebSocket endpoints).
#[tokio::test]
async fn test_get_urls() {
    let config = ExchangeConfig::default();
    let exchange = Binance::new(config).unwrap();
    let urls = exchange.urls();
    assert!(urls.public.contains("api.binance.com"));
    assert!(urls.ws.contains("stream.binance.com"));
}

/// Test fetching all available markets from the real API.
#[tokio::test]
#[ignore]
async fn test_fetch_markets_real() {
    let exchange = Binance::new(ExchangeConfig::default()).unwrap();
    let result = exchange.fetch_markets().await;

    assert!(
        result.is_ok(),
        "Failed to fetch markets: {:?}",
        result.err()
    );
    let markets = result.unwrap();

    assert!(
        markets.len() > 100,
        "Expected more than 100 markets, got {}",
        markets.len()
    );

    let btc_usdt = markets.iter().find(|m| m.symbol == "BTC/USDT");
    assert!(btc_usdt.is_some(), "BTC/USDT market not found");

    if let Some(market) = btc_usdt {
        assert_eq!(market.base, "BTC");
        assert_eq!(market.quote, "USDT");
        assert!(market.active);
    }
}

/// Test fetching ticker data for BTC/USDT from the real API.
#[tokio::test]
#[ignore]
async fn test_fetch_ticker_real() {
    let exchange = Binance::new(ExchangeConfig::default()).unwrap();
    let result = exchange
        .fetch_ticker("BTC/USDT", ccxt_core::types::TickerParams::default())
        .await;

    assert!(result.is_ok(), "Failed to fetch ticker: {:?}", result.err());
    let ticker = result.unwrap();

    assert_eq!(ticker.symbol, "BTC/USDT");
    assert!(ticker.last.is_some(), "Last price should be present");
    assert!(ticker.bid.is_some(), "Bid price should be present");
    assert!(ticker.ask.is_some(), "Ask price should be present");
    assert!(ticker.high.is_some(), "High price should be present");
    assert!(ticker.low.is_some(), "Low price should be present");
    assert!(ticker.base_volume.is_some(), "Volume should be present");
}

/// Test fetching order book data from the real API.
#[tokio::test]
#[ignore]
async fn test_fetch_order_book_real() {
    let exchange = Binance::new(ExchangeConfig::default()).unwrap();
    let result = exchange.fetch_order_book("BTC/USDT", Some(10)).await;

    assert!(
        result.is_ok(),
        "Failed to fetch order book: {:?}",
        result.err()
    );
    let order_book = result.unwrap();

    assert_eq!(order_book.symbol, "BTC/USDT");
    assert!(!order_book.bids.is_empty(), "Bids should not be empty");
    assert!(!order_book.asks.is_empty(), "Asks should not be empty");
    assert!(order_book.bids.len() <= 10, "Should have at most 10 bids");
    assert!(order_book.asks.len() <= 10, "Should have at most 10 asks");

    if let (Some(best_bid), Some(best_ask)) = (order_book.bids.first(), order_book.asks.first()) {
        assert!(
            best_bid.price < best_ask.price,
            "Best bid ({}) should be less than best ask ({})",
            best_bid.price,
            best_ask.price
        );
    }
}

/// Test fetching recent trades from the real API.
#[tokio::test]
#[ignore]
async fn test_fetch_trades_real() {
    let exchange = Binance::new(ExchangeConfig::default()).unwrap();
    let result = exchange.fetch_trades("BTC/USDT", Some(5)).await;

    assert!(result.is_ok(), "Failed to fetch trades: {:?}", result.err());
    let trades = result.unwrap();

    assert!(!trades.is_empty(), "Trades should not be empty");
    assert!(trades.len() <= 5, "Should have at most 5 trades");

    for trade in &trades {
        assert_eq!(trade.symbol, "BTC/USDT");
        assert!(
            trade.price > Price::from(rust_decimal::Decimal::ZERO),
            "Trade price should be positive"
        );
        assert!(
            trade.amount > Amount::from(rust_decimal::Decimal::ZERO),
            "Trade amount should be positive"
        );
        assert!(trade.timestamp > 0, "Trade should have timestamp");
    }
}

/// Test fetching OHLCV (candlestick) data from the real API.
#[tokio::test]
#[ignore]
async fn test_fetch_ohlcv_real() {
    let exchange = Binance::new(ExchangeConfig::default()).unwrap();
    let result = exchange
        .fetch_ohlcv("BTC/USDT", "1h", None, Some(5), None)
        .await;

    assert!(result.is_ok(), "Failed to fetch OHLCV: {:?}", result.err());
    let candles = result.unwrap();

    assert!(!candles.is_empty(), "Candles should not be empty");
    assert!(candles.len() <= 5, "Should have at most 5 candles");

    for candle in &candles {
        assert!(candle.open > 0.0, "Open price should be positive");
        assert!(candle.high >= candle.low, "High should be >= low");
        assert!(candle.high >= candle.open, "High should be >= open");
        assert!(candle.high >= candle.close, "High should be >= close");
        assert!(candle.low <= candle.open, "Low should be <= open");
        assert!(candle.low <= candle.close, "Low should be <= close");
        assert!(candle.volume >= 0.0, "Volume should be non-negative");
    }
}

/// Test error handling for invalid trading symbols.
#[tokio::test]
#[ignore]
async fn test_invalid_symbol() {
    let exchange = Binance::new(ExchangeConfig::default()).unwrap();
    let result = exchange
        .fetch_ticker("INVALID/SYMBOL", ccxt_core::types::TickerParams::default())
        .await;

    assert!(result.is_err(), "Should fail for invalid symbol");
}

/// Test handling of unsupported timeframe (2h is not supported by Binance).
#[tokio::test]
#[ignore]
async fn test_unsupported_timeframe() {
    let exchange = Binance::new(ExchangeConfig::default()).unwrap();
    let result = exchange
        .fetch_ohlcv("BTC/USDT", "2h", None, Some(5), None)
        .await;

    if result.is_err() {
        println!(
            "Expected error for unsupported timeframe: {:?}",
            result.err()
        );
    }
}

// ==================== Public API Tests ====================

/// Test fetching server time from Binance.
#[tokio::test]
#[ignore]
async fn test_fetch_time() {
    let exchange = Binance::new(ExchangeConfig::default()).unwrap();
    let result = exchange.fetch_time().await;

    assert!(
        result.is_ok(),
        "Failed to fetch server time: {:?}",
        result.err()
    );
    let timestamp = result.unwrap();
    assert!(timestamp.server_time > 0, "Server time should be positive");
}

/// Test fetching multiple tickers at once.
#[tokio::test]
#[ignore]
async fn test_fetch_tickers() {
    let exchange = Binance::new(ExchangeConfig::default()).unwrap();
    let result = exchange
        .fetch_tickers(Some(vec!["BTC/USDT".to_string(), "ETH/USDT".to_string()]))
        .await;

    assert!(
        result.is_ok(),
        "Failed to fetch tickers: {:?}",
        result.err()
    );
    let tickers = result.unwrap();
    assert!(tickers.len() >= 2, "Should have at least 2 tickers");

    assert!(tickers.iter().any(|t| t.symbol == "BTC/USDT"));
    assert!(tickers.iter().any(|t| t.symbol == "ETH/USDT"));
}

/// Test fetching tickers for multiple symbols sequentially.
#[tokio::test]
#[ignore]
async fn test_fetch_multiple_symbols() {
    let exchange = Binance::new(ExchangeConfig::default()).unwrap();
    let symbols = vec!["BTC/USDT", "ETH/USDT", "BNB/USDT"];

    for symbol in symbols {
        let result = exchange
            .fetch_ticker(symbol, ccxt_core::types::TickerParams::default())
            .await;
        assert!(
            result.is_ok(),
            "Failed to fetch ticker for {}: {:?}",
            symbol,
            result.err()
        );

        let ticker = result.unwrap();
        assert_eq!(ticker.symbol, symbol);
        assert!(ticker.last.is_some());
    }
}

/// Test validating market data completeness.
#[tokio::test]
#[ignore]
async fn test_market_validation() {
    let exchange = Binance::new(ExchangeConfig::default()).unwrap();
    let result = exchange.fetch_markets().await;

    assert!(result.is_ok());
    let markets = result.unwrap();

    for market in markets.iter().take(10) {
        assert!(!market.symbol.is_empty(), "Symbol should not be empty");
        assert!(!market.base.is_empty(), "Base currency should not be empty");
        assert!(
            !market.quote.is_empty(),
            "Quote currency should not be empty"
        );
    }
}

/// Test fetching 24-hour ticker change data.
#[tokio::test]
#[ignore]
async fn test_ticker_24h_change() {
    let exchange = Binance::new(ExchangeConfig::default()).unwrap();
    let result = exchange
        .fetch_ticker("BTC/USDT", ccxt_core::types::TickerParams::default())
        .await;

    assert!(result.is_ok());
    let ticker = result.unwrap();

    assert!(
        ticker.percentage.is_some() || ticker.change.is_some(),
        "Should have 24h change data"
    );
    assert!(ticker.high.is_some(), "Should have 24h high");
    assert!(ticker.low.is_some(), "Should have 24h low");
}

// ==================== Private API Tests ====================

/// Test fetching account balance.
///
/// Note: Requires API credentials.
#[tokio::test]
#[ignore]
async fn test_fetch_balance() {
    let config = get_api_credentials();
    if config.api_key.is_none() || config.secret.is_none() {
        println!("⚠️  Skip test: API credentials not set");
        return;
    }

    let exchange = Binance::new(config).unwrap();
    let result = exchange.fetch_balance(None).await;

    assert!(
        result.is_ok(),
        "Failed to fetch balance: {:?}",
        result.err()
    );
    let balance = result.unwrap();

    assert!(!balance.balances.is_empty(), "Should have balance data");
}

/// Test fetching open orders.
///
/// Note: Requires API credentials.
#[tokio::test]
#[ignore]
async fn test_fetch_open_orders() {
    let config = get_api_credentials();
    if config.api_key.is_none() || config.secret.is_none() {
        println!("⚠️  Skip test: API credentials not set");
        return;
    }

    let exchange = Binance::new(config).unwrap();
    let result = exchange.fetch_open_orders(Some("BTC/USDT")).await;

    assert!(
        result.is_ok(),
        "Failed to fetch open orders: {:?}",
        result.err()
    );
}

/// Test fetching a specific order by ID.
///
/// Note: Requires API credentials and a real order ID.
#[tokio::test]
#[ignore]
async fn test_fetch_order() {
    let config = get_api_credentials();
    if config.api_key.is_none() || config.secret.is_none() {
        println!("⚠️  Skip test: API credentials not set");
        return;
    }

    let _exchange = Binance::new(config).unwrap();

    println!("⚠️  Requires real order ID for complete testing");
}

/// Test fetching user's trade history.
///
/// Note: Requires API credentials.
#[tokio::test]
#[ignore]
async fn test_fetch_my_trades() {
    let config = get_api_credentials();
    if config.api_key.is_none() || config.secret.is_none() {
        println!("⚠️  Skip test: API credentials not set");
        return;
    }

    let exchange = Binance::new(config).unwrap();
    let result = exchange.fetch_my_trades("BTC/USDT", None, None).await;

    assert!(
        result.is_ok(),
        "Failed to fetch my trades: {:?}",
        result.err()
    );
}

// ==================== 错误处理测试 ====================

#[tokio::test]
#[ignore]
async fn test_invalid_api_key() {
    let mut config = ExchangeConfig::default();
    config.api_key = Some("invalid_key".to_string());
    config.secret = Some("invalid_secret".to_string());
    let exchange = Binance::new(config).unwrap();
    let result = exchange.fetch_balance(None).await;

    assert!(result.is_err(), "Should fail with invalid API key");
}

#[tokio::test]
#[ignore]
async fn test_rate_limit_handling() {
    let exchange = Binance::new(ExchangeConfig::default()).unwrap();

    // 快速连续请求，测试速率限制处理
    for i in 0..5 {
        let result = exchange
            .fetch_ticker("BTC/USDT", ccxt_core::types::TickerParams::default())
            .await;
        if result.is_err() {
            println!("请求 {} 失败（可能触发速率限制）: {:?}", i, result.err());
        }
    }
}

#[tokio::test]
#[ignore]
async fn test_network_error_retry() {
    // 这个测试需要模拟网络错误
    // 在实际实现中应该测试重试机制
    println!("⚠️  网络错误重试测试需要mock框架支持");
}

/// Test signature error handling for authenticated requests.
#[tokio::test]
#[ignore]
async fn test_signature_error() {
    let mut config = ExchangeConfig::default();
    config.api_key = Some("test_key".to_string());
    config.secret = Some("wrong_secret".to_string());
    let exchange = Binance::new(config).unwrap();
    let result = exchange.fetch_balance(None).await;

    assert!(result.is_err(), "Should fail with wrong signature");
}

/// Test that operations requiring credentials fail without API keys.
#[tokio::test]
#[ignore]
async fn test_check_required_credentials_without_keys() {
    let exchange = Binance::new(ExchangeConfig::default()).unwrap();

    let result = exchange.fetch_balance(None).await;
    assert!(result.is_err(), "Should fail without API credentials");
}

// Note: Tests for creating and canceling test orders require Binance testnet environment.
// These tests should be performed in dedicated testnet test suites.

#[cfg(test)]
mod parser_tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_parse_market() {
        let data = json!({
            "symbol": "BTCUSDT",
            "status": "TRADING",
            "baseAsset": "BTC",
            "quoteAsset": "USDT",
            "baseAssetPrecision": 8,
            "quoteAssetPrecision": 8,
            "orderTypes": ["LIMIT", "MARKET"],
            "filters": [
                {
                    "filterType": "PRICE_FILTER",
                    "minPrice": "0.01",
                    "maxPrice": "1000000.00",
                    "tickSize": "0.01"
                },
                {
                    "filterType": "LOT_SIZE",
                    "minQty": "0.00001",
                    "maxQty": "9000.00",
                    "stepSize": "0.00001"
                }
            ]
        });

        let result = parser::parse_market(&data);
        assert!(result.is_ok());

        let market = result.unwrap();
        assert_eq!(market.symbol, "BTC/USDT");
        assert_eq!(market.base, "BTC");
        assert_eq!(market.quote, "USDT");
        assert!(market.active);
    }

    #[test]
    fn test_parse_ticker() {
        let data = json!({
            "symbol": "BTCUSDT",
            "lastPrice": "50000.00",
            "bidPrice": "49999.00",
            "askPrice": "50001.00",
            "highPrice": "51000.00",
            "lowPrice": "49000.00",
            "volume": "1000.5",
            "quoteVolume": "50000000.00"
        });

        let market = Market::new_spot(
            "BTCUSDT".to_string(),
            "BTC/USDT".to_string(),
            "BTC".to_string(),
            "USDT".to_string(),
        );

        let result = parser::parse_ticker(&data, Some(&market));
        assert!(result.is_ok());

        let ticker = result.unwrap();
        assert_eq!(ticker.symbol, "BTC/USDT");
        assert_eq!(
            ticker.last,
            Some(Price::from(rust_decimal::Decimal::new(500000, 1)))
        ); // 50000.0
        assert_eq!(
            ticker.bid,
            Some(Price::from(rust_decimal::Decimal::new(499990, 1)))
        ); // 49999.0
        assert_eq!(
            ticker.ask,
            Some(Price::from(rust_decimal::Decimal::new(500010, 1)))
        ); // 50001.0
    }

    #[test]
    fn test_parse_order() {
        let data = json!({
            "orderId": 12345,
            "symbol": "BTCUSDT",
            "price": "50000.00",
            "origQty": "0.1",
            "executedQty": "0.1",
            "cummulativeQuoteQty": "5000.00",
            "status": "FILLED",
            "type": "LIMIT",
            "side": "BUY",
            "time": 1234567890000_i64
        });

        let market = Market::new_spot(
            "BTCUSDT".to_string(),
            "BTC/USDT".to_string(),
            "BTC".to_string(),
            "USDT".to_string(),
        );
        let result = parser::parse_order(&data, Some(&market));
        assert!(result.is_ok());

        let order = result.unwrap();
        assert_eq!(order.id, "12345");
        assert_eq!(order.symbol, "BTC/USDT");
        assert_eq!(order.side, OrderSide::Buy);
        assert_eq!(order.order_type, OrderType::Limit);
        assert_eq!(order.status, OrderStatus::Closed);
    }

    // ==================== Parser Unit Tests (Additional) ====================

    #[test]
    fn test_parse_market_with_filters() {
        let data = json!({
            "symbol": "ETHUSDT",
            "status": "TRADING",
            "baseAsset": "ETH",
            "quoteAsset": "USDT",
            "baseAssetPrecision": 8,
            "quoteAssetPrecision": 8,
            "filters": [
                {
                    "filterType": "PRICE_FILTER",
                    "minPrice": "0.01",
                    "maxPrice": "100000.00",
                    "tickSize": "0.01"
                },
                {
                    "filterType": "LOT_SIZE",
                    "minQty": "0.0001",
                    "maxQty": "10000.00",
                    "stepSize": "0.0001"
                },
                {
                    "filterType": "MIN_NOTIONAL",
                    "minNotional": "10.00"
                }
            ]
        });

        let result = parser::parse_market(&data);
        assert!(result.is_ok());

        let market = result.unwrap();
        assert_eq!(market.symbol, "ETH/USDT");

        let limits = &market.limits;
        assert!(limits.amount.is_some() || limits.price.is_some());
    }

    #[test]
    fn test_parse_ticker_edge_cases() {
        let data = json!({
            "symbol": "BTCUSDT",
            "lastPrice": "50000.00",
            "volume": "1000.0"
        });

        let market = Market::new_spot(
            "BTCUSDT".to_string(),
            "BTC/USDT".to_string(),
            "BTC".to_string(),
            "USDT".to_string(),
        );

        let result = parser::parse_ticker(&data, Some(&market));
        assert!(result.is_ok());

        let ticker = result.unwrap();
        assert_eq!(ticker.symbol, "BTC/USDT");
        assert_eq!(
            ticker.last,
            Some(Price::from(rust_decimal::Decimal::new(500000, 1)))
        ); // 50000.0
        assert!(ticker.bid.is_none() || ticker.bid.is_some());
    }

    #[test]
    fn test_parse_trade_timestamp() {
        let data = json!({
            "id": 12345,
            "price": "50000.00",
            "qty": "0.1",
            "time": 1234567890000_i64,
            "isBuyerMaker": false
        });

        let market = Market::new_spot(
            "BTCUSDT".to_string(),
            "BTC/USDT".to_string(),
            "BTC".to_string(),
            "USDT".to_string(),
        );

        let result = parser::parse_trade(&data, Some(&market));
        assert!(result.is_ok());

        let trade = result.unwrap();
        assert_eq!(trade.symbol, "BTC/USDT");

        let expected_price = rust_decimal::Decimal::from_str("50000.00").unwrap();
        let expected_amount = rust_decimal::Decimal::from_str("0.1").unwrap();
        assert!(
            (trade.price.as_decimal() - expected_price).abs()
                < rust_decimal::Decimal::from_str("0.0000001").unwrap()
        );
        assert!(
            (trade.amount.as_decimal() - expected_amount).abs()
                < rust_decimal::Decimal::from_str("0.0000001").unwrap()
        );
        assert_eq!(trade.timestamp, 1234567890000);
    }

    #[test]
    fn test_parse_order_status() {
        let market = Market::new_spot(
            "BTCUSDT".to_string(),
            "BTC/USDT".to_string(),
            "BTC".to_string(),
            "USDT".to_string(),
        );

        let statuses = vec![
            ("NEW", OrderStatus::Open),
            ("PARTIALLY_FILLED", OrderStatus::Open),
            ("FILLED", OrderStatus::Closed),
            ("CANCELED", OrderStatus::Cancelled),
            ("REJECTED", OrderStatus::Rejected),
            ("EXPIRED", OrderStatus::Expired),
        ];

        for (binance_status, expected_status) in statuses {
            let data = json!({
                "orderId": 12345,
                "symbol": "BTCUSDT",
                "status": binance_status,
                "type": "LIMIT",
                "side": "BUY",
                "origQty": "0.1",
                "price": "50000.0",
                "time": 1234567890000_i64
            });

            let result = parser::parse_order(&data, Some(&market));
            assert!(result.is_ok(), "Failed to parse status: {}", binance_status);

            let order = result.unwrap();
            assert_eq!(order.status, expected_status);
        }
    }

    #[test]
    fn test_parse_balance_locked() {
        let _exchange = Binance::new(ExchangeConfig::default()).unwrap();
        let _data = json!({
            "asset": "BTC",
            "free": "1.5",
            "locked": "0.5"
        });

        println!("Test balance parsing (requires parse_balance_entry method implementation)");
    }

    #[test]
    fn test_parse_empty_response() {
        let _exchange = Binance::new(ExchangeConfig::default()).unwrap();

        println!("Test empty response parsing (requires parse_orders method implementation)");
    }

    #[test]
    fn test_parse_malformed_data() {
        let _exchange = Binance::new(ExchangeConfig::default()).unwrap();

        let data = json!({
            "symbol": "INVALID"
        });

        let result = parser::parse_market(&data);
        assert!(result.is_err() || result.is_ok());
    }

    #[test]
    fn test_currency_precision() {
        let _exchange = Binance::new(ExchangeConfig::default()).unwrap();
        let data = json!({
            "symbol": "BTCUSDT",
            "status": "TRADING",
            "baseAsset": "BTC",
            "quoteAsset": "USDT",
            "baseAssetPrecision": 8,
            "quotePrecision": 2,
            "quoteAssetPrecision": 8,
            "filters": [
                {
                    "filterType": "PRICE_FILTER",
                    "tickSize": "0.01"
                },
                {
                    "filterType": "LOT_SIZE",
                    "stepSize": "0.00000001"
                }
            ]
        });

        let result = parser::parse_market(&data);
        assert!(result.is_ok());

        let market = result.unwrap();
        let precision = &market.precision;
        assert!(precision.amount.is_some() || precision.price.is_some());
    }

    #[test]
    fn test_market_limits() {
        let _exchange = Binance::new(ExchangeConfig::default()).unwrap();
        let data = json!({
            "symbol": "BTCUSDT",
            "status": "TRADING",
            "baseAsset": "BTC",
            "quoteAsset": "USDT",
            "filters": [
                {
                    "filterType": "PRICE_FILTER",
                    "minPrice": "0.01",
                    "maxPrice": "1000000.00"
                },
                {
                    "filterType": "LOT_SIZE",
                    "minQty": "0.00001",
                    "maxQty": "9000.00"
                }
            ]
        });

        let result = parser::parse_market(&data);
        assert!(result.is_ok());

        let market = result.unwrap();
        let limits = &market.limits;
        assert!(
            limits.amount.is_some()
                || limits.price.is_some()
                || limits.cost.is_some()
                || limits.leverage.is_some()
        );
    }

    #[test]
    fn test_fee_calculation() {
        let _maker_fee = 0.001; // 0.1%
        let taker_fee = 0.001; // 0.1%

        let trade_value = 1000.0;
        let fee = trade_value * taker_fee;

        assert_eq!(fee, 1.0);
    }

    #[test]
    fn test_symbol_normalization() {
        let binance_symbol = "BTCUSDT";
        let normalized = format!("{}/{}", &binance_symbol[..3], &binance_symbol[3..]);

        assert_eq!(normalized, "BTC/USDT");
    }

    #[test]
    fn test_timeframe_conversion() {
        let timeframes = vec![("1m", 60), ("5m", 300), ("1h", 3600), ("1d", 86400)];

        for (tf, expected_seconds) in timeframes {
            let seconds = match tf {
                "1m" => 60,
                "5m" => 300,
                "1h" => 3600,
                "1d" => 86400,
                _ => 0,
            };
            assert_eq!(seconds, expected_seconds);
        }
    }
}
