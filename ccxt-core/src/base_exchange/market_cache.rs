//! Market data caching structures and operations

use crate::error::Result;
use crate::types::{Currency, Market};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::info;

/// Market data cache structure
#[derive(Debug, Clone)]
pub struct MarketCache {
    /// Markets indexed by symbol (e.g., "BTC/USDT")
    pub markets: Arc<HashMap<String, Arc<Market>>>,
    /// Markets indexed by exchange-specific ID
    pub markets_by_id: HashMap<String, Arc<Market>>,
    /// Currencies indexed by code (e.g., "BTC")
    pub currencies: HashMap<String, Arc<Currency>>,
    /// Currencies indexed by exchange-specific ID
    pub currencies_by_id: HashMap<String, Arc<Currency>>,
    /// List of all trading pair symbols
    pub symbols: Vec<String>,
    /// List of all currency codes
    pub codes: Vec<String>,
    /// List of all market IDs
    pub ids: Vec<String>,
    /// Whether markets have been loaded
    pub loaded: bool,
}

impl Default for MarketCache {
    fn default() -> Self {
        Self {
            markets: Arc::new(HashMap::new()),
            markets_by_id: HashMap::new(),
            currencies: HashMap::new(),
            currencies_by_id: HashMap::new(),
            symbols: Vec::new(),
            codes: Vec::new(),
            ids: Vec::new(),
            loaded: false,
        }
    }
}

impl MarketCache {
    /// Sets market and currency data in the cache
    pub fn set_markets(
        &mut self,
        markets: Vec<Market>,
        currencies: Option<Vec<Currency>>,
        exchange_id: &str,
    ) -> Result<Arc<HashMap<String, Arc<Market>>>> {
        let mut markets_map = HashMap::new();
        self.markets_by_id.clear();
        self.symbols.clear();
        self.ids.clear();

        for market in markets {
            self.symbols.push(market.symbol.clone());
            self.ids.push(market.id.clone());
            let arc_market = Arc::new(market);
            self.markets_by_id
                .insert(arc_market.id.clone(), Arc::clone(&arc_market));
            markets_map.insert(arc_market.symbol.clone(), arc_market);
        }
        self.markets = Arc::new(markets_map);

        if let Some(currencies) = currencies {
            self.currencies.clear();
            self.currencies_by_id.clear();
            self.codes.clear();

            for currency in currencies {
                self.codes.push(currency.code.clone());
                let arc_currency = Arc::new(currency);
                self.currencies_by_id
                    .insert(arc_currency.id.clone(), Arc::clone(&arc_currency));
                self.currencies
                    .insert(arc_currency.code.clone(), arc_currency);
            }
        }

        self.loaded = true;
        info!(
            "Loaded {} markets and {} currencies for {}",
            self.markets.len(),
            self.currencies.len(),
            exchange_id
        );

        Ok(self.markets.clone())
    }
}
