//! WebSocket data cache for Binance.
//!
//! Consolidates all cached market and account data into a single component,
//! reducing the number of fields on `BinanceWs`.

use ccxt_core::types::{
    Balance, BidAsk, MarkPrice, OHLCV, Order, OrderBook, Position, Ticker, Trade,
};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};

/// Consolidated data cache for all WebSocket streams.
pub struct WsDataCache {
    pub tickers: Arc<Mutex<HashMap<String, Ticker>>>,
    pub bids_asks: Arc<Mutex<HashMap<String, BidAsk>>>,
    pub mark_prices: Arc<Mutex<HashMap<String, MarkPrice>>>,
    pub orderbooks: Arc<Mutex<HashMap<String, OrderBook>>>,
    pub trades: Arc<Mutex<HashMap<String, VecDeque<Trade>>>>,
    pub ohlcvs: Arc<Mutex<HashMap<String, VecDeque<OHLCV>>>>,
    pub balances: Arc<RwLock<HashMap<String, Balance>>>,
    pub orders: Arc<RwLock<HashMap<String, HashMap<String, Order>>>>,
    pub my_trades: Arc<RwLock<HashMap<String, VecDeque<Trade>>>>,
    pub positions: Arc<RwLock<HashMap<String, HashMap<String, Position>>>>,
}

impl WsDataCache {
    /// Creates a new empty data cache.
    pub fn new() -> Self {
        Self {
            tickers: Arc::new(Mutex::new(HashMap::new())),
            bids_asks: Arc::new(Mutex::new(HashMap::new())),
            mark_prices: Arc::new(Mutex::new(HashMap::new())),
            orderbooks: Arc::new(Mutex::new(HashMap::new())),
            trades: Arc::new(Mutex::new(HashMap::new())),
            ohlcvs: Arc::new(Mutex::new(HashMap::new())),
            balances: Arc::new(RwLock::new(HashMap::new())),
            orders: Arc::new(RwLock::new(HashMap::new())),
            my_trades: Arc::new(RwLock::new(HashMap::new())),
            positions: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl Default for WsDataCache {
    fn default() -> Self {
        Self::new()
    }
}
