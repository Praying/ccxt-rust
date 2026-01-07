//! Market data caching structures and operations
//!
//! This module provides thread-safe caching for market and currency data.
//! It uses `DashMap` for concurrent access with minimal lock contention,
//! making it suitable for high-frequency trading scenarios.

use crate::error::Result;
use crate::types::{Currency, Market};
use dashmap::DashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tracing::info;

/// Market data cache structure with fine-grained locking.
///
/// Uses `DashMap` instead of `RwLock<HashMap>` to reduce lock contention
/// in high-concurrency scenarios. Each key-value pair has its own lock,
/// allowing concurrent reads and writes to different keys.
///
/// # Performance Characteristics
///
/// - Concurrent reads: O(1) with no blocking
/// - Concurrent writes to different keys: O(1) with no blocking
/// - Writes to the same key: Serialized per-key
///
/// # Example
///
/// ```rust
/// use ccxt_core::base_exchange::MarketCache;
///
/// let cache = MarketCache::default();
/// // Multiple threads can read different markets concurrently
/// // without blocking each other
/// ```
#[derive(Debug)]
pub struct MarketCache {
    /// Markets indexed by symbol (e.g., "BTC/USDT")
    /// Using DashMap for fine-grained concurrent access
    markets: DashMap<String, Arc<Market>>,
    /// Markets indexed by exchange-specific ID
    markets_by_id: DashMap<String, Arc<Market>>,
    /// Currencies indexed by code (e.g., "BTC")
    currencies: DashMap<String, Arc<Currency>>,
    /// Currencies indexed by exchange-specific ID
    currencies_by_id: DashMap<String, Arc<Currency>>,
    /// List of all trading pair symbols (cached for fast iteration)
    symbols: parking_lot::RwLock<Vec<String>>,
    /// List of all currency codes (cached for fast iteration)
    codes: parking_lot::RwLock<Vec<String>>,
    /// List of all market IDs (cached for fast iteration)
    ids: parking_lot::RwLock<Vec<String>>,
    /// Whether markets have been loaded
    loaded: AtomicBool,
}

impl Clone for MarketCache {
    fn clone(&self) -> Self {
        Self {
            markets: self.markets.clone(),
            markets_by_id: self.markets_by_id.clone(),
            currencies: self.currencies.clone(),
            currencies_by_id: self.currencies_by_id.clone(),
            symbols: parking_lot::RwLock::new(self.symbols.read().clone()),
            codes: parking_lot::RwLock::new(self.codes.read().clone()),
            ids: parking_lot::RwLock::new(self.ids.read().clone()),
            loaded: AtomicBool::new(self.loaded.load(Ordering::Acquire)),
        }
    }
}

impl Default for MarketCache {
    fn default() -> Self {
        Self {
            markets: DashMap::new(),
            markets_by_id: DashMap::new(),
            currencies: DashMap::new(),
            currencies_by_id: DashMap::new(),
            symbols: parking_lot::RwLock::new(Vec::new()),
            codes: parking_lot::RwLock::new(Vec::new()),
            ids: parking_lot::RwLock::new(Vec::new()),
            loaded: AtomicBool::new(false),
        }
    }
}

impl MarketCache {
    /// Creates a new empty market cache.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a new market cache with pre-allocated capacity.
    ///
    /// Use this when you know the approximate number of markets
    /// to avoid reallocations during loading.
    pub fn with_capacity(markets_capacity: usize, currencies_capacity: usize) -> Self {
        Self {
            markets: DashMap::with_capacity(markets_capacity),
            markets_by_id: DashMap::with_capacity(markets_capacity),
            currencies: DashMap::with_capacity(currencies_capacity),
            currencies_by_id: DashMap::with_capacity(currencies_capacity),
            symbols: parking_lot::RwLock::new(Vec::with_capacity(markets_capacity)),
            codes: parking_lot::RwLock::new(Vec::with_capacity(currencies_capacity)),
            ids: parking_lot::RwLock::new(Vec::with_capacity(markets_capacity)),
            loaded: AtomicBool::new(false),
        }
    }

    /// Returns whether markets have been loaded.
    #[inline]
    pub fn is_loaded(&self) -> bool {
        self.loaded.load(Ordering::Acquire)
    }

    /// Gets a market by symbol.
    ///
    /// This is a lock-free read operation.
    #[inline]
    pub fn get_market(&self, symbol: &str) -> Option<Arc<Market>> {
        self.markets.get(symbol).map(|r| Arc::clone(r.value()))
    }

    /// Gets a market by exchange-specific ID.
    ///
    /// This is a lock-free read operation.
    #[inline]
    pub fn get_market_by_id(&self, id: &str) -> Option<Arc<Market>> {
        self.markets_by_id.get(id).map(|r| Arc::clone(r.value()))
    }

    /// Gets a currency by code.
    ///
    /// This is a lock-free read operation.
    #[inline]
    pub fn get_currency(&self, code: &str) -> Option<Arc<Currency>> {
        self.currencies.get(code).map(|r| Arc::clone(r.value()))
    }

    /// Gets a currency by exchange-specific ID.
    ///
    /// This is a lock-free read operation.
    #[inline]
    pub fn get_currency_by_id(&self, id: &str) -> Option<Arc<Currency>> {
        self.currencies_by_id.get(id).map(|r| Arc::clone(r.value()))
    }

    /// Returns the number of cached markets.
    #[inline]
    pub fn market_count(&self) -> usize {
        self.markets.len()
    }

    /// Returns the number of cached currencies.
    #[inline]
    pub fn currency_count(&self) -> usize {
        self.currencies.len()
    }

    /// Returns a copy of all symbol names.
    pub fn symbols(&self) -> Vec<String> {
        self.symbols.read().clone()
    }

    /// Returns a copy of all currency codes.
    pub fn codes(&self) -> Vec<String> {
        self.codes.read().clone()
    }

    /// Returns a copy of all market IDs.
    pub fn ids(&self) -> Vec<String> {
        self.ids.read().clone()
    }

    /// Returns all markets as a new HashMap.
    ///
    /// Note: This creates a new HashMap and clones all Arc references.
    /// For iteration, consider using `iter_markets()` instead.
    pub fn markets(&self) -> Arc<std::collections::HashMap<String, Arc<Market>>> {
        let map: std::collections::HashMap<String, Arc<Market>> = self
            .markets
            .iter()
            .map(|r| (r.key().clone(), Arc::clone(r.value())))
            .collect();
        Arc::new(map)
    }

    /// Iterates over all markets.
    ///
    /// This is more efficient than `markets()` when you only need to iterate.
    pub fn iter_markets(&self) -> impl Iterator<Item = (String, Arc<Market>)> + '_ {
        self.markets
            .iter()
            .map(|r| (r.key().clone(), Arc::clone(r.value())))
    }

    /// Checks if a market exists by symbol.
    #[inline]
    pub fn contains_market(&self, symbol: &str) -> bool {
        self.markets.contains_key(symbol)
    }

    /// Checks if a currency exists by code.
    #[inline]
    pub fn contains_currency(&self, code: &str) -> bool {
        self.currencies.contains_key(code)
    }

    /// Sets market and currency data in the cache.
    ///
    /// This operation clears existing data and replaces it with the new data.
    /// It's designed to be called during market loading/refresh.
    pub fn set_markets(
        &self,
        markets: Vec<Market>,
        currencies: Option<Vec<Currency>>,
        exchange_id: &str,
    ) -> Result<Arc<std::collections::HashMap<String, Arc<Market>>>> {
        // Clear existing market data
        self.markets.clear();
        self.markets_by_id.clear();

        // Pre-allocate vectors
        let mut symbols = Vec::with_capacity(markets.len());
        let mut ids = Vec::with_capacity(markets.len());

        // Insert markets
        for market in markets {
            symbols.push(market.symbol.clone());
            ids.push(market.id.clone());
            let arc_market = Arc::new(market);
            self.markets_by_id
                .insert(arc_market.id.clone(), Arc::clone(&arc_market));
            self.markets.insert(arc_market.symbol.clone(), arc_market);
        }

        // Update cached lists
        *self.symbols.write() = symbols;
        *self.ids.write() = ids;

        // Handle currencies if provided
        if let Some(currencies) = currencies {
            self.currencies.clear();
            self.currencies_by_id.clear();

            let mut codes = Vec::with_capacity(currencies.len());

            for currency in currencies {
                codes.push(currency.code.clone());
                let arc_currency = Arc::new(currency);
                self.currencies_by_id
                    .insert(arc_currency.id.clone(), Arc::clone(&arc_currency));
                self.currencies
                    .insert(arc_currency.code.clone(), arc_currency);
            }

            *self.codes.write() = codes;
        }

        self.loaded.store(true, Ordering::Release);

        info!(
            exchange = %exchange_id,
            markets = self.markets.len(),
            currencies = self.currencies.len(),
            "Market cache loaded"
        );

        Ok(self.markets())
    }

    /// Clears all cached data.
    pub fn clear(&self) {
        self.markets.clear();
        self.markets_by_id.clear();
        self.currencies.clear();
        self.currencies_by_id.clear();
        self.symbols.write().clear();
        self.codes.write().clear();
        self.ids.write().clear();
        self.loaded.store(false, Ordering::Release);
    }
}
