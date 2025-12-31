//! Binance API constants.
//!
//! This module contains constants for API endpoints, order statuses, and other
//! fixed strings used throughout the Binance implementation.

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
