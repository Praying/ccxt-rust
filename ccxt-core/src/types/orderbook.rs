//! Orderbook type definitions

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, VecDeque};

use super::{Amount, Price, Symbol, Timestamp};

/// Maximum number of buffered messages for orderbook sync
const MAX_BUFFERED_MESSAGES: usize = 100;

/// Minimum resync interval in milliseconds
const MIN_RESYNC_INTERVAL_MS: i64 = 1000;

/// Orderbook delta update message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderBookDelta {
    /// Symbol
    pub symbol: Symbol,

    /// First update ID in event
    #[serde(rename = "U")]
    pub first_update_id: i64,

    /// Final update ID in event
    #[serde(rename = "u")]
    pub final_update_id: i64,

    /// Previous final update ID (futures only)
    #[serde(rename = "pu")]
    pub prev_final_update_id: Option<i64>,

    /// Event timestamp
    pub timestamp: Timestamp,

    /// Bid updates
    pub bids: Vec<OrderBookEntry>,

    /// Ask updates
    pub asks: Vec<OrderBookEntry>,
}

/// Order book entry (price level)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OrderBookEntry {
    /// Price level
    pub price: Price,

    /// Total amount at this price level
    pub amount: Amount,
}

impl OrderBookEntry {
    /// Create a new order book entry
    pub fn new(price: Price, amount: Amount) -> Self {
        Self { price, amount }
    }
}

/// Order book side (bids or asks)
pub type OrderBookSide = Vec<OrderBookEntry>;

/// Complete order book structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderBook {
    /// Exchange symbol
    pub symbol: Symbol,

    /// Timestamp in milliseconds
    pub timestamp: Timestamp,

    /// ISO8601 datetime string
    pub datetime: Option<String>,

    /// Nonce/sequence number for updates
    pub nonce: Option<i64>,

    /// Bid side (buyers) - sorted descending by price
    pub bids: OrderBookSide,

    /// Ask side (sellers) - sorted ascending by price
    pub asks: OrderBookSide,

    /// Raw exchange info
    #[serde(flatten)]
    pub info: HashMap<String, serde_json::Value>,

    /// Buffered delta messages waiting for snapshot
    #[serde(skip)]
    pub buffered_deltas: VecDeque<OrderBookDelta>,

    /// BTree map for efficient bid updates (price -> amount)
    #[serde(skip)]
    pub bids_map: BTreeMap<String, Decimal>,

    /// BTree map for efficient ask updates (price -> amount)
    #[serde(skip)]
    pub asks_map: BTreeMap<String, Decimal>,

    /// Whether orderbook is synchronized
    #[serde(skip)]
    pub is_synced: bool,

    /// Request resync flag (set when sequence gap detected)
    #[serde(skip)]
    pub needs_resync: bool,

    /// Last resync timestamp (for rate limiting resync attempts)
    #[serde(skip)]
    pub last_resync_time: i64,
}

impl OrderBook {
    /// Create a new empty order book
    pub fn new(symbol: Symbol, timestamp: Timestamp) -> Self {
        Self {
            symbol,
            timestamp,
            datetime: None,
            nonce: None,
            bids: Vec::new(),
            asks: Vec::new(),
            info: HashMap::new(),
            buffered_deltas: VecDeque::new(),
            bids_map: BTreeMap::new(),
            asks_map: BTreeMap::new(),
            is_synced: false,
            needs_resync: false,
            last_resync_time: 0,
        }
    }

    /// Get best bid (highest buy price)
    pub fn best_bid(&self) -> Option<&OrderBookEntry> {
        self.bids.first()
    }

    /// Get best ask (lowest sell price)
    pub fn best_ask(&self) -> Option<&OrderBookEntry> {
        self.asks.first()
    }

    /// Calculate bid-ask spread
    pub fn spread(&self) -> Option<Price> {
        match (self.best_bid(), self.best_ask()) {
            (Some(bid), Some(ask)) => Some(ask.price - bid.price),
            _ => None,
        }
    }

    /// Calculate spread percentage
    pub fn spread_percentage(&self) -> Option<Decimal> {
        match (self.best_bid(), self.best_ask()) {
            (Some(bid), Some(ask)) if bid.price.as_decimal() > Decimal::ZERO => {
                let spread = ask.price.as_decimal() - bid.price.as_decimal();
                Some(spread / bid.price.as_decimal() * Decimal::from(100))
            }
            _ => None,
        }
    }

    /// Calculate mid price (average of best bid and ask)
    pub fn mid_price(&self) -> Option<Price> {
        match (self.best_bid(), self.best_ask()) {
            (Some(bid), Some(ask)) => {
                let sum = bid.price.as_decimal() + ask.price.as_decimal();
                Some(Price::new(sum / Decimal::from(2)))
            }
            _ => None,
        }
    }

    /// Get total bid volume
    pub fn bid_volume(&self) -> Amount {
        let total: Decimal = self
            .bids
            .iter()
            .map(|entry| entry.amount.as_decimal())
            .sum();
        Amount::new(total)
    }

    /// Get total ask volume
    pub fn ask_volume(&self) -> Amount {
        let total: Decimal = self
            .asks
            .iter()
            .map(|entry| entry.amount.as_decimal())
            .sum();
        Amount::new(total)
    }

    /// Sort bids in descending order (highest price first)
    pub fn sort_bids(&mut self) {
        self.bids.sort_by(|a, b| b.price.cmp(&a.price));
    }

    /// Sort asks in ascending order (lowest price first)
    pub fn sort_asks(&mut self) {
        self.asks.sort_by(|a, b| a.price.cmp(&b.price));
    }

    /// Sort both sides
    pub fn sort(&mut self) {
        self.sort_bids();
        self.sort_asks();
    }

    /// Limit order book to specified depth per side
    pub fn limit(&mut self, depth: usize) {
        self.bids.truncate(depth);
        self.asks.truncate(depth);
    }

    /// Update order book with new entries
    pub fn update(&mut self, bids: OrderBookSide, asks: OrderBookSide, timestamp: Timestamp) {
        self.bids = bids;
        self.asks = asks;
        self.timestamp = timestamp;
        self.sort();
    }

    /// Reset orderbook from snapshot
    pub fn reset_from_snapshot(
        &mut self,
        bids: OrderBookSide,
        asks: OrderBookSide,
        timestamp: Timestamp,
        nonce: Option<i64>,
    ) {
        self.bids = bids;
        self.asks = asks;
        self.timestamp = timestamp;
        self.nonce = nonce;

        // Rebuild maps
        self.bids_map.clear();
        self.asks_map.clear();

        for entry in &self.bids {
            self.bids_map
                .insert(entry.price.to_string(), entry.amount.as_decimal());
        }

        for entry in &self.asks {
            self.asks_map
                .insert(entry.price.to_string(), entry.amount.as_decimal());
        }

        self.sort();
        self.is_synced = true;
    }

    /// Buffer a delta update for later processing
    pub fn buffer_delta(&mut self, delta: OrderBookDelta) {
        if self.buffered_deltas.len() >= MAX_BUFFERED_MESSAGES {
            // Remove oldest message if buffer is full
            self.buffered_deltas.pop_front();
        }
        self.buffered_deltas.push_back(delta);
    }

    /// Apply a delta update (incremental update)
    pub fn apply_delta(&mut self, delta: &OrderBookDelta, is_futures: bool) -> Result<(), String> {
        // Validate sequence based on market type
        if is_futures {
            // Validate sequence for futures markets
            if let Some(current_nonce) = self.nonce {
                // For futures: pu should equal previous u
                if let Some(pu) = delta.prev_final_update_id
                    && pu != current_nonce
                {
                    // Sequence mismatch detected - mark for resync
                    self.needs_resync = true;
                    self.is_synced = false;
                    tracing::warn!(
                        "Futures sequence mismatch for {}: expected pu = {}, got {}. Marking for resync.",
                        self.symbol,
                        current_nonce,
                        pu
                    );
                    return Err(format!(
                        "Futures sequence mismatch: expected pu = {current_nonce}, got {pu}"
                    ));
                }

                // Skip if outdated
                if delta.final_update_id < current_nonce {
                    return Ok(());
                }
            }
        } else {
            // Validate sequence for spot markets
            if let Some(current_nonce) = self.nonce {
                // For spot: U should be current_nonce + 1
                if delta.first_update_id > current_nonce + 1 {
                    // Sequence gap detected - mark for resync
                    self.needs_resync = true;
                    self.is_synced = false;
                    tracing::warn!(
                        "Sequence gap detected for {}: expected U <= {}, got {}. Marking for resync.",
                        self.symbol,
                        current_nonce + 1,
                        delta.first_update_id
                    );
                    return Err(format!(
                        "Gap in updates: expected U <= {}, got {}",
                        current_nonce + 1,
                        delta.first_update_id
                    ));
                }

                // Skip if this update is outdated
                if delta.final_update_id <= current_nonce {
                    return Ok(());
                }
            }
        }

        // Apply bid updates
        for entry in &delta.bids {
            let price_str = entry.price.to_string();
            if entry.amount == Amount::new(Decimal::ZERO) {
                // Remove price level
                self.bids_map.remove(&price_str);
            } else {
                // Update price level
                self.bids_map.insert(price_str, entry.amount.as_decimal());
            }
        }

        // Apply ask updates
        for entry in &delta.asks {
            let price_str = entry.price.to_string();
            if entry.amount == Amount::new(Decimal::ZERO) {
                // Remove price level
                self.asks_map.remove(&price_str);
            } else {
                // Update price level
                self.asks_map.insert(price_str, entry.amount.as_decimal());
            }
        }

        // Update metadata
        self.nonce = Some(delta.final_update_id);
        self.timestamp = delta.timestamp;

        // Rebuild sorted vectors from maps
        self.rebuild_sides();

        Ok(())
    }

    /// Rebuild bids and asks vectors from maps
    fn rebuild_sides(&mut self) {
        // Rebuild bids (descending order)
        self.bids = self
            .bids_map
            .iter()
            .rev()
            .map(|(price_str, amount)| {
                OrderBookEntry::new(
                    Price::new(price_str.parse::<Decimal>().unwrap_or(Decimal::ZERO)),
                    Amount::new(*amount),
                )
            })
            .collect();

        // Rebuild asks (ascending order)
        self.asks = self
            .asks_map
            .iter()
            .map(|(price_str, amount)| {
                OrderBookEntry::new(
                    Price::new(price_str.parse::<Decimal>().unwrap_or(Decimal::ZERO)),
                    Amount::new(*amount),
                )
            })
            .collect();
    }

    /// Process buffered deltas after receiving snapshot
    pub fn process_buffered_deltas(&mut self, is_futures: bool) -> Result<usize, String> {
        let mut processed = 0;
        let snapshot_nonce = self.nonce.ok_or("No snapshot nonce available")?;

        // Process deltas in order
        while let Some(delta) = self.buffered_deltas.pop_front() {
            // Check if delta is valid for this snapshot
            if is_futures {
                // Futures: skip if u < snapshot nonce
                if delta.final_update_id < snapshot_nonce {
                    continue;
                }

                // First valid delta: pu (if present) should match snapshot_nonce
                if processed == 0
                    && let Some(pu) = delta.prev_final_update_id
                    && pu != snapshot_nonce
                {
                    return Err(format!(
                        "First futures delta invalid: pu ({pu}) != lastUpdateId ({snapshot_nonce})"
                    ));
                }
            } else {
                // Spot: skip if u <= snapshot nonce
                if delta.final_update_id <= snapshot_nonce {
                    continue;
                }

                // First valid delta: U should be <= snapshot_nonce + 1
                if processed == 0 && delta.first_update_id > snapshot_nonce + 1 {
                    return Err(format!(
                        "First delta invalid: U ({}) > lastUpdateId + 1 ({})",
                        delta.first_update_id,
                        snapshot_nonce + 1
                    ));
                }
            }

            // Apply the delta
            self.apply_delta(&delta, is_futures)?;
            processed += 1;
        }

        Ok(processed)
    }

    /// Clear all buffered deltas
    pub fn clear_buffer(&mut self) {
        self.buffered_deltas.clear();
    }

    /// Get number of buffered deltas
    pub fn buffered_count(&self) -> usize {
        self.buffered_deltas.len()
    }

    /// Reset orderbook state for resync
    /// Clears all data but keeps symbol and preserves buffered deltas
    pub fn reset_for_resync(&mut self) {
        self.bids.clear();
        self.asks.clear();
        self.bids_map.clear();
        self.asks_map.clear();
        self.nonce = None;
        self.is_synced = false;
        self.needs_resync = false;
        // Keep buffered_deltas - they will be processed after new snapshot
        tracing::info!(
            "Reset orderbook for {} for resync, keeping {} buffered deltas",
            self.symbol,
            self.buffered_deltas.len()
        );
    }

    /// Check if resync is needed and rate limit is satisfied
    /// Rate limit: minimum 1 second between resyncs
    pub fn should_resync(&self, current_time: i64) -> bool {
        if !self.needs_resync {
            return false;
        }

        // Rate limit: at least 1000ms between resyncs
        current_time - self.last_resync_time >= MIN_RESYNC_INTERVAL_MS
    }

    /// Mark that resync has been initiated
    pub fn mark_resync_initiated(&mut self, current_time: i64) {
        self.last_resync_time = current_time;
        tracing::info!(
            "Resync initiated for {} at timestamp {}",
            self.symbol,
            current_time
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn create_test_orderbook() -> OrderBook {
        let mut ob = OrderBook::new("BTC/USDT".to_string(), 1234567890);

        ob.bids = vec![
            OrderBookEntry::new(Price::from(dec!(50000)), Amount::from(dec!(1.0))),
            OrderBookEntry::new(Price::from(dec!(49900)), Amount::from(dec!(2.0))),
            OrderBookEntry::new(Price::from(dec!(49800)), Amount::from(dec!(1.5))),
        ];

        ob.asks = vec![
            OrderBookEntry::new(Price::from(dec!(50100)), Amount::from(dec!(1.0))),
            OrderBookEntry::new(Price::from(dec!(50200)), Amount::from(dec!(2.0))),
            OrderBookEntry::new(Price::from(dec!(50300)), Amount::from(dec!(1.5))),
        ];

        ob
    }

    fn create_delta(first_u: i64, final_u: i64, prev_u: Option<i64>) -> OrderBookDelta {
        OrderBookDelta {
            symbol: "BTC/USDT".to_string(),
            first_update_id: first_u,
            final_update_id: final_u,
            prev_final_update_id: prev_u,
            timestamp: 1234567890,
            bids: vec![OrderBookEntry::new(
                Price::from(dec!(49950)),
                Amount::from(dec!(3.0)),
            )],
            asks: vec![OrderBookEntry::new(
                Price::from(dec!(50150)),
                Amount::from(dec!(2.5)),
            )],
        }
    }

    #[test]
    fn test_orderbook_creation() {
        let ob = create_test_orderbook();
        assert_eq!(ob.symbol, "BTC/USDT");
        assert_eq!(ob.bids.len(), 3);
        assert_eq!(ob.asks.len(), 3);
    }

    #[test]
    fn test_best_bid_ask() {
        let ob = create_test_orderbook();

        let best_bid = ob.best_bid().unwrap();
        assert_eq!(best_bid.price, Price::from(dec!(50000)));

        let best_ask = ob.best_ask().unwrap();
        assert_eq!(best_ask.price, Price::from(dec!(50100)));
    }

    #[test]
    fn test_spread_calculation() {
        let ob = create_test_orderbook();

        assert_eq!(ob.spread(), Some(Price::from(dec!(100))));
        assert_eq!(ob.spread_percentage(), Some(dec!(0.2)));
    }

    #[test]
    fn test_mid_price() {
        let ob = create_test_orderbook();
        assert_eq!(ob.mid_price(), Some(Price::from(dec!(50050))));
    }

    #[test]
    fn test_volume_calculation() {
        let ob = create_test_orderbook();

        assert_eq!(ob.bid_volume(), Amount::from(dec!(4.5)));
        assert_eq!(ob.ask_volume(), Amount::from(dec!(4.5)));
    }

    #[test]
    fn test_limit() {
        let mut ob = create_test_orderbook();
        ob.limit(2);

        assert_eq!(ob.bids.len(), 2);
        assert_eq!(ob.asks.len(), 2);
    }

    #[test]
    fn test_sorting() {
        let mut ob = OrderBook::new("BTC/USDT".to_string(), 1234567890);

        // Add unsorted bids
        ob.bids = vec![
            OrderBookEntry::new(Price::from(dec!(49800)), Amount::from(dec!(1.5))),
            OrderBookEntry::new(Price::from(dec!(50000)), Amount::from(dec!(1.0))),
            OrderBookEntry::new(Price::from(dec!(49900)), Amount::from(dec!(2.0))),
        ];

        // Add unsorted asks
        ob.asks = vec![
            OrderBookEntry::new(Price::from(dec!(50300)), Amount::from(dec!(1.5))),
            OrderBookEntry::new(Price::from(dec!(50100)), Amount::from(dec!(1.0))),
            OrderBookEntry::new(Price::from(dec!(50200)), Amount::from(dec!(2.0))),
        ];

        ob.sort();

        // Check bids are sorted descending
        assert_eq!(ob.bids[0].price, Price::from(dec!(50000)));
        assert_eq!(ob.bids[1].price, Price::from(dec!(49900)));
        assert_eq!(ob.bids[2].price, Price::from(dec!(49800)));

        // Check asks are sorted ascending
        assert_eq!(ob.asks[0].price, Price::from(dec!(50100)));
        assert_eq!(ob.asks[1].price, Price::from(dec!(50200)));
        assert_eq!(ob.asks[2].price, Price::from(dec!(50300)));
    }

    #[test]
    fn test_reset_from_snapshot() {
        let mut ob = OrderBook::new("BTC/USDT".to_string(), 1234567890);

        let bids = vec![
            OrderBookEntry::new(Price::from(dec!(50000)), Amount::from(dec!(1.0))),
            OrderBookEntry::new(Price::from(dec!(49900)), Amount::from(dec!(2.0))),
        ];

        let asks = vec![
            OrderBookEntry::new(Price::from(dec!(50100)), Amount::from(dec!(1.5))),
            OrderBookEntry::new(Price::from(dec!(50200)), Amount::from(dec!(2.5))),
        ];

        ob.reset_from_snapshot(bids, asks, 1234567900, Some(100));

        assert_eq!(ob.nonce, Some(100));
        assert_eq!(ob.timestamp, 1234567900);
        assert_eq!(ob.bids.len(), 2);
        assert_eq!(ob.asks.len(), 2);
        assert!(ob.is_synced);
        assert_eq!(ob.bids_map.len(), 2);
        assert_eq!(ob.asks_map.len(), 2);
    }

    #[test]
    fn test_buffer_delta() {
        let mut ob = OrderBook::new("BTC/USDT".to_string(), 1234567890);

        let delta1 = create_delta(1, 10, None);
        let delta2 = create_delta(11, 20, None);

        ob.buffer_delta(delta1);
        ob.buffer_delta(delta2);

        assert_eq!(ob.buffered_count(), 2);
    }

    #[test]
    fn test_apply_delta_spot() {
        let mut ob = OrderBook::new("BTC/USDT".to_string(), 1234567890);

        // Setup initial snapshot
        let bids = vec![OrderBookEntry::new(
            Price::from(dec!(50000)),
            Amount::from(dec!(1.0)),
        )];
        let asks = vec![OrderBookEntry::new(
            Price::from(dec!(50100)),
            Amount::from(dec!(1.0)),
        )];
        ob.reset_from_snapshot(bids, asks, 1234567890, Some(100));

        // Apply delta with U=101, u=110
        let delta = create_delta(101, 110, None);
        let result = ob.apply_delta(&delta, false);

        assert!(result.is_ok());
        assert_eq!(ob.nonce, Some(110));
        assert!(ob.bids_map.contains_key("49950"));
        assert!(ob.asks_map.contains_key("50150"));
    }

    #[test]
    fn test_apply_delta_futures() {
        let mut ob = OrderBook::new("BTC/USDT".to_string(), 1234567890);

        // Setup initial snapshot
        let bids = vec![OrderBookEntry::new(
            Price::from(dec!(50000)),
            Amount::from(dec!(1.0)),
        )];
        let asks = vec![OrderBookEntry::new(
            Price::from(dec!(50100)),
            Amount::from(dec!(1.0)),
        )];
        ob.reset_from_snapshot(bids, asks, 1234567890, Some(100));

        // Apply delta with pu=100, u=110
        let delta = create_delta(101, 110, Some(100));
        let result = ob.apply_delta(&delta, true);

        assert!(result.is_ok());
        assert_eq!(ob.nonce, Some(110));
    }

    #[test]
    fn test_apply_delta_remove_price_level() {
        let mut ob = OrderBook::new("BTC/USDT".to_string(), 1234567890);

        // Setup initial snapshot
        let bids = vec![
            OrderBookEntry::new(Price::from(dec!(50000)), Amount::from(dec!(1.0))),
            OrderBookEntry::new(Price::from(dec!(49900)), Amount::from(dec!(2.0))),
        ];
        let asks = vec![OrderBookEntry::new(
            Price::from(dec!(50100)),
            Amount::from(dec!(1.0)),
        )];
        ob.reset_from_snapshot(bids, asks, 1234567890, Some(100));

        // Create delta to remove a price level (amount = 0)
        let delta = OrderBookDelta {
            symbol: "BTC/USDT".to_string(),
            first_update_id: 101,
            final_update_id: 110,
            prev_final_update_id: None,
            timestamp: 1234567900,
            bids: vec![OrderBookEntry::new(
                Price::from(dec!(49900)),
                Amount::from(dec!(0)),
            )],
            asks: vec![],
        };

        let result = ob.apply_delta(&delta, false);
        assert!(result.is_ok());

        // Verify price level was removed
        assert!(!ob.bids_map.contains_key("49900"));
        assert_eq!(ob.bids.len(), 1);
    }

    #[test]
    fn test_delta_sequence_validation_spot() {
        let mut ob = OrderBook::new("BTC/USDT".to_string(), 1234567890);

        // Setup snapshot with nonce=100
        let bids = vec![OrderBookEntry::new(
            Price::from(dec!(50000)),
            Amount::from(dec!(1.0)),
        )];
        let asks = vec![OrderBookEntry::new(
            Price::from(dec!(50100)),
            Amount::from(dec!(1.0)),
        )];
        ob.reset_from_snapshot(bids, asks, 1234567890, Some(100));

        // Try to apply delta with gap (U=105 > nonce+1)
        let delta = create_delta(105, 110, None);
        let result = ob.apply_delta(&delta, false);

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Gap in updates"));
    }

    #[test]
    fn test_delta_sequence_validation_futures() {
        let mut ob = OrderBook::new("BTC/USDT".to_string(), 1234567890);

        // Setup snapshot with nonce=100
        let bids = vec![OrderBookEntry::new(
            Price::from(dec!(50000)),
            Amount::from(dec!(1.0)),
        )];
        let asks = vec![OrderBookEntry::new(
            Price::from(dec!(50100)),
            Amount::from(dec!(1.0)),
        )];
        ob.reset_from_snapshot(bids, asks, 1234567890, Some(100));

        // Try to apply delta with wrong pu
        let delta = create_delta(101, 110, Some(95));
        let result = ob.apply_delta(&delta, true);

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("sequence mismatch"));
    }

    #[test]
    fn test_process_buffered_deltas_spot() {
        let mut ob = OrderBook::new("BTC/USDT".to_string(), 1234567890);

        // Buffer some deltas before snapshot
        ob.buffer_delta(create_delta(95, 99, None));
        ob.buffer_delta(create_delta(101, 105, None));
        ob.buffer_delta(create_delta(106, 110, None));

        // Setup snapshot with nonce=100
        let bids = vec![OrderBookEntry::new(
            Price::from(dec!(50000)),
            Amount::from(dec!(1.0)),
        )];
        let asks = vec![OrderBookEntry::new(
            Price::from(dec!(50100)),
            Amount::from(dec!(1.0)),
        )];
        ob.reset_from_snapshot(bids, asks, 1234567890, Some(100));

        // Process buffered deltas
        let result = ob.process_buffered_deltas(false);

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 2); // Should process 2 deltas (skip first one with u=99)
        assert_eq!(ob.nonce, Some(110));
        assert_eq!(ob.buffered_count(), 0);
    }

    #[test]
    fn test_process_buffered_deltas_futures() {
        let mut ob = OrderBook::new("BTC/USDT".to_string(), 1234567890);

        // Buffer some deltas before snapshot
        ob.buffer_delta(create_delta(95, 99, Some(94)));
        ob.buffer_delta(create_delta(101, 105, Some(100)));
        ob.buffer_delta(create_delta(106, 110, Some(105)));

        // Setup snapshot with nonce=100
        let bids = vec![OrderBookEntry::new(
            Price::from(dec!(50000)),
            Amount::from(dec!(1.0)),
        )];
        let asks = vec![OrderBookEntry::new(
            Price::from(dec!(50100)),
            Amount::from(dec!(1.0)),
        )];
        ob.reset_from_snapshot(bids, asks, 1234567890, Some(100));

        // Process buffered deltas
        let result = ob.process_buffered_deltas(true);

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 2); // Should process 2 deltas (skip first one with u=99)
    }

    #[test]
    fn test_clear_buffer() {
        let mut ob = OrderBook::new("BTC/USDT".to_string(), 1234567890);

        ob.buffer_delta(create_delta(1, 10, None));
        ob.buffer_delta(create_delta(11, 20, None));

        assert_eq!(ob.buffered_count(), 2);

        ob.clear_buffer();
        assert_eq!(ob.buffered_count(), 0);
    }

    #[test]
    fn test_rebuild_sides() {
        let mut ob = OrderBook::new("BTC/USDT".to_string(), 1234567890);

        // Manually populate maps (maps store Decimal, not Amount)
        ob.bids_map.insert("50000".to_string(), dec!(1.0));
        ob.bids_map.insert("49900".to_string(), dec!(2.0));
        ob.asks_map.insert("50100".to_string(), dec!(1.5));
        ob.asks_map.insert("50200".to_string(), dec!(2.5));

        ob.rebuild_sides();

        // Check bids are sorted descending
        assert_eq!(ob.bids.len(), 2);
        assert_eq!(ob.bids[0].price, Price::from(dec!(50000)));
        assert_eq!(ob.bids[1].price, Price::from(dec!(49900)));

        // Check asks are sorted ascending
        assert_eq!(ob.asks.len(), 2);
        assert_eq!(ob.asks[0].price, Price::from(dec!(50100)));
        assert_eq!(ob.asks[1].price, Price::from(dec!(50200)));
    }
}
