//! Futures market data type definitions.
//!
//! Provides market data structures specific to futures trading, including index prices,
//! premium indices, and liquidation orders.

use serde::{Deserialize, Serialize};

/// Index price information.
///
/// Represents the index price for a trading pair, typically used as a reference
/// for mark price calculations in derivatives markets.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexPrice {
    /// Raw API response data.
    pub info: Option<serde_json::Value>,
    /// Trading pair symbol.
    pub symbol: String,
    /// Index price value.
    pub index_price: f64,
    /// Timestamp in milliseconds.
    pub timestamp: u64,
}

impl Default for IndexPrice {
    fn default() -> Self {
        Self {
            info: None,
            symbol: String::new(),
            index_price: 0.0,
            timestamp: 0,
        }
    }
}

/// Premium index information.
///
/// Contains comprehensive market data including mark price, index price,
/// and funding rate information for futures contracts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PremiumIndex {
    /// Raw API response data.
    pub info: Option<serde_json::Value>,
    /// Trading pair symbol.
    pub symbol: String,
    /// Mark price used for profit/loss calculation.
    pub mark_price: f64,
    /// Index price calculated from spot exchanges.
    pub index_price: f64,
    /// Estimated settlement price.
    pub estimated_settle_price: f64,
    /// Last applied funding rate.
    pub last_funding_rate: f64,
    /// Next funding rate settlement time in milliseconds.
    pub next_funding_time: u64,
    /// Current timestamp in milliseconds.
    pub time: u64,
}

impl Default for PremiumIndex {
    fn default() -> Self {
        Self {
            info: None,
            symbol: String::new(),
            mark_price: 0.0,
            index_price: 0.0,
            estimated_settle_price: 0.0,
            last_funding_rate: 0.0,
            next_funding_time: 0,
            time: 0,
        }
    }
}

/// Liquidation order information.
///
/// Represents an order that was forcefully closed due to insufficient margin.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Liquidation {
    /// Raw API response data.
    pub info: Option<serde_json::Value>,
    /// Trading pair symbol.
    pub symbol: String,
    /// Order side ("BUY" or "SELL").
    pub side: String,
    /// Order type ("LIMIT" or "MARKET").
    pub order_type: String,
    /// Liquidation timestamp in milliseconds.
    pub time: u64,
    /// Liquidation price.
    pub price: f64,
    /// Liquidation quantity.
    pub quantity: f64,
    /// Average execution price.
    pub average_price: f64,
}

impl Default for Liquidation {
    fn default() -> Self {
        Self {
            info: None,
            symbol: String::new(),
            side: String::new(),
            order_type: String::new(),
            time: 0,
            price: 0.0,
            quantity: 0.0,
            average_price: 0.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_index_price_default() {
        let index_price = IndexPrice::default();
        assert_eq!(index_price.symbol, "");
        assert_eq!(index_price.index_price, 0.0);
        assert_eq!(index_price.timestamp, 0);
    }

    #[test]
    fn test_premium_index_default() {
        let premium = PremiumIndex::default();
        assert_eq!(premium.symbol, "");
        assert_eq!(premium.mark_price, 0.0);
        assert_eq!(premium.index_price, 0.0);
        assert_eq!(premium.last_funding_rate, 0.0);
    }

    #[test]
    fn test_liquidation_default() {
        let liquidation = Liquidation::default();
        assert_eq!(liquidation.symbol, "");
        assert_eq!(liquidation.side, "");
        assert_eq!(liquidation.price, 0.0);
        assert_eq!(liquidation.quantity, 0.0);
    }

    #[test]
    fn test_index_price_serialization() {
        let index_price = IndexPrice {
            info: None,
            symbol: "BTCUSDT".to_string(),
            index_price: 50000.0,
            timestamp: 1234567890000,
        };

        let json = serde_json::to_string(&index_price).unwrap();
        assert!(json.contains("BTCUSDT"));
        assert!(json.contains("50000"));
    }
}
