//! Stream URL building and management functions

#![allow(dead_code)]

/// Binance WebSocket endpoints
pub const WS_BASE_URL: &str = "wss://stream.binance.com:9443/ws";
/// Binance testnet WebSocket endpoint
pub const WS_TESTNET_URL: &str = "wss://testnet.binance.vision/ws";

/// Builds a ticker stream name
pub fn ticker_stream(symbol: &str) -> String {
    format!("{}@ticker", symbol.to_lowercase())
}

/// Builds a mini ticker stream name
pub fn mini_ticker_stream(symbol: &str) -> String {
    format!("{}@miniTicker", symbol.to_lowercase())
}

/// Builds a trade stream name
pub fn trade_stream(symbol: &str) -> String {
    format!("{}@trade", symbol.to_lowercase())
}

/// Builds an aggregated trade stream name
pub fn agg_trade_stream(symbol: &str) -> String {
    format!("{}@aggTrade", symbol.to_lowercase())
}

/// Builds an order book depth stream name
pub fn orderbook_stream(symbol: &str, levels: u32, update_speed: &str) -> String {
    if update_speed == "100ms" {
        format!("{}@depth{}@100ms", symbol.to_lowercase(), levels)
    } else {
        format!("{}@depth{}", symbol.to_lowercase(), levels)
    }
}

/// Builds a diff order book stream name
pub fn orderbook_diff_stream(symbol: &str, update_speed: Option<&str>) -> String {
    if let Some(speed) = update_speed {
        if speed == "100ms" {
            format!("{}@depth@100ms", symbol.to_lowercase())
        } else {
            format!("{}@depth", symbol.to_lowercase())
        }
    } else {
        format!("{}@depth", symbol.to_lowercase())
    }
}

/// Builds a kline stream name
pub fn kline_stream(symbol: &str, interval: &str) -> String {
    format!("{}@kline_{}", symbol.to_lowercase(), interval)
}

/// Builds a mark price stream name
pub fn mark_price_stream(symbol: &str, use_1s_freq: bool) -> String {
    if use_1s_freq {
        format!("{}@markPrice@1s", symbol.to_lowercase())
    } else {
        format!("{}@markPrice", symbol.to_lowercase())
    }
}

/// Builds a book ticker stream name
pub fn book_ticker_stream(symbol: &str) -> String {
    format!("{}@bookTicker", symbol.to_lowercase())
}

/// Builds an all tickers stream name
pub fn all_tickers_stream() -> String {
    "!ticker@arr".to_string()
}

/// Builds an all mini tickers stream name
pub fn all_mini_tickers_stream() -> String {
    "!miniTicker@arr".to_string()
}

/// Builds a user data stream URL
pub fn user_data_stream_url(listen_key: &str) -> String {
    format!("wss://stream.binance.com:9443/ws/{}", listen_key)
}
