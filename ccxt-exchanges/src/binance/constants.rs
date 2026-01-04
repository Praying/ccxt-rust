//! Binance API constants.
//!
//! This module contains constants for API endpoints, order statuses, and other
//! fixed strings used throughout the Binance implementation.

use std::collections::HashMap;

/// API Endpoints
pub mod endpoints {
    /// Public API base URL
    pub const PUBLIC: &str = "https://api.binance.com/api/v3";
    /// SAPI (System API) base URL
    pub const SAPI: &str = "https://api.binance.com/sapi/v1";
    /// FAPI (Futures API) base URL
    pub const FAPI: &str = "https://fapi.binance.com/fapi/v1";
    /// DAPI (Delivery API) base URL
    pub const DAPI: &str = "https://dapi.binance.com/dapi/v1";

    /// Exchange Info endpoint
    pub const EXCHANGE_INFO: &str = "/exchangeInfo";
    /// 24hr Ticker endpoint
    pub const TICKER_24HR: &str = "/ticker/24hr";
    /// Price Ticker endpoint
    pub const TICKER_PRICE: &str = "/ticker/price";
    /// Order Book (Depth) endpoint
    pub const DEPTH: &str = "/depth";
    /// Recent Trades endpoint
    pub const TRADES: &str = "/trades";
    /// Aggregated Trades endpoint
    pub const AGG_TRADES: &str = "/aggTrades";
    /// Kline/Candlestick endpoint
    pub const KLINES: &str = "/klines";
    /// Rolling window ticker endpoint
    pub const TICKER_ROLLING: &str = "/ticker";
    /// Historical Trades endpoint
    pub const HISTORICAL_TRADES: &str = "/historicalTrades";
    /// System Status endpoint
    pub const SYSTEM_STATUS: &str = "/system/status";
    /// Server Time endpoint
    pub const TIME: &str = "/time";

    /// Order endpoint (create, cancel, fetch)
    pub const ORDER: &str = "/order";
    /// Open Orders endpoint
    pub const OPEN_ORDERS: &str = "/openOrders";
    /// All Orders endpoint
    pub const ALL_ORDERS: &str = "/allOrders";
}

/// Order Statuses
pub mod status {
    /// The order has been accepted by the engine.
    pub const NEW: &str = "NEW";
    /// A part of the order has been filled.
    pub const PARTIALLY_FILLED: &str = "PARTIALLY_FILLED";
    /// The order has been completed.
    pub const FILLED: &str = "FILLED";
    /// The order has been canceled by the user.
    pub const CANCELED: &str = "CANCELED";
    /// The order is currently pending cancelation.
    pub const PENDING_CANCEL: &str = "PENDING_CANCEL";
    /// The order was not accepted by the engine and not processed.
    pub const REJECTED: &str = "REJECTED";
    /// The order was canceled according to the order type's rules.
    pub const EXPIRED: &str = "EXPIRED";
}

/// Returns the supported timeframes for Binance.
pub fn timeframes() -> HashMap<String, String> {
    let mut timeframes = HashMap::new();
    timeframes.insert("1s".to_string(), "1s".to_string());
    timeframes.insert("1m".to_string(), "1m".to_string());
    timeframes.insert("3m".to_string(), "3m".to_string());
    timeframes.insert("5m".to_string(), "5m".to_string());
    timeframes.insert("15m".to_string(), "15m".to_string());
    timeframes.insert("30m".to_string(), "30m".to_string());
    timeframes.insert("1h".to_string(), "1h".to_string());
    timeframes.insert("2h".to_string(), "2h".to_string());
    timeframes.insert("4h".to_string(), "4h".to_string());
    timeframes.insert("6h".to_string(), "6h".to_string());
    timeframes.insert("8h".to_string(), "8h".to_string());
    timeframes.insert("12h".to_string(), "12h".to_string());
    timeframes.insert("1d".to_string(), "1d".to_string());
    timeframes.insert("3d".to_string(), "3d".to_string());
    timeframes.insert("1w".to_string(), "1w".to_string());
    timeframes.insert("1M".to_string(), "1M".to_string());
    timeframes
}
