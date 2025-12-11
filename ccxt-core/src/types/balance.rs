//! Balance type definitions

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Balance entry for a single currency
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BalanceEntry {
    /// Free balance available for trading
    pub free: Decimal,

    /// Used balance (in open orders)
    pub used: Decimal,

    /// Total balance (free + used)
    pub total: Decimal,
}

impl BalanceEntry {
    /// Create a new balance entry
    pub fn new(free: Decimal, used: Decimal) -> Self {
        Self {
            free,
            used,
            total: free + used,
        }
    }

    /// Create from total only
    pub fn from_total(total: Decimal) -> Self {
        Self {
            free: total,
            used: Decimal::ZERO,
            total,
        }
    }

    /// Apply delta to free balance
    ///
    /// # Arguments
    /// * `delta` - Amount to add (can be negative)
    pub fn apply_delta(&mut self, delta: Decimal) {
        self.free += delta;
        self.total = self.free + self.used;
    }

    /// Update from wallet balance (for futures)
    ///
    /// # Arguments
    /// * `wallet_balance` - Total wallet balance
    /// * `cross_wallet` - Cross wallet balance (optional)
    pub fn update_wallet_balance(
        &mut self,
        wallet_balance: Decimal,
        cross_wallet: Option<Decimal>,
    ) {
        self.total = wallet_balance;
        // Use cross_wallet if provided, otherwise keep existing free balance
        if let Some(cw) = cross_wallet {
            self.free = cw;
        }
        // Recalculate used balance
        self.used = self.total - self.free;
    }
}

/// Balance structure containing all currency balances
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Balance {
    /// Map of currency code to balance entry
    #[serde(flatten)]
    pub balances: HashMap<String, BalanceEntry>,

    /// Raw exchange info
    #[serde(skip)]
    pub info: HashMap<String, serde_json::Value>,
}

impl Balance {
    /// Create a new empty balance
    pub fn new() -> Self {
        Self {
            balances: HashMap::new(),
            info: HashMap::new(),
        }
    }

    /// Get balance for a specific currency
    pub fn get(&self, currency: &str) -> Option<&BalanceEntry> {
        self.balances.get(currency)
    }

    /// Get mutable balance for a specific currency
    pub fn get_mut(&mut self, currency: &str) -> Option<&mut BalanceEntry> {
        self.balances.get_mut(currency)
    }

    /// Set balance for a currency
    pub fn set(&mut self, currency: String, entry: BalanceEntry) {
        self.balances.insert(currency, entry);
    }

    /// Apply delta to a currency balance
    ///
    /// If currency doesn't exist, creates new entry with delta as free balance
    ///
    /// # Arguments
    /// * `currency` - Currency code
    /// * `delta` - Amount to add (can be negative)
    pub fn apply_delta(&mut self, currency: String, delta: Decimal) {
        if let Some(entry) = self.balances.get_mut(&currency) {
            entry.apply_delta(delta);
        } else {
            // Create new entry with delta as initial balance
            let entry = BalanceEntry {
                free: delta,
                used: Decimal::ZERO,
                total: delta,
            };
            self.balances.insert(currency, entry);
        }
    }

    /// Update balance from complete data (for outboundAccountPosition)
    ///
    /// # Arguments
    /// * `currency` - Currency code
    /// * `free` - Free balance
    /// * `locked` - Locked/used balance
    pub fn update_balance(&mut self, currency: String, free: Decimal, locked: Decimal) {
        let entry = BalanceEntry::new(free, locked);
        self.balances.insert(currency, entry);
    }

    /// Update wallet balance (for futures ACCOUNT_UPDATE)
    ///
    /// # Arguments
    /// * `currency` - Currency code
    /// * `wallet_balance` - Total wallet balance
    /// * `cross_wallet` - Cross wallet balance (optional)
    pub fn update_wallet(
        &mut self,
        currency: String,
        wallet_balance: Decimal,
        cross_wallet: Option<Decimal>,
    ) {
        if let Some(entry) = self.balances.get_mut(&currency) {
            entry.update_wallet_balance(wallet_balance, cross_wallet);
        } else {
            // Create new entry
            let free = cross_wallet.unwrap_or(wallet_balance);
            let used = wallet_balance - free;
            let entry = BalanceEntry {
                free,
                used,
                total: wallet_balance,
            };
            self.balances.insert(currency, entry);
        }
    }

    /// Merge another balance into this one
    ///
    /// # Arguments
    /// * `other` - Balance to merge from
    pub fn merge(&mut self, other: &Balance) {
        for (currency, entry) in &other.balances {
            self.balances.insert(currency.clone(), entry.clone());
        }
    }
}

impl Default for Balance {
    fn default() -> Self {
        Self::new()
    }
}

/// Maximum borrowable amount information.
///
/// Contains details about the maximum amount that can be borrowed for margin trading.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaxBorrowable {
    /// Currency code.
    pub currency: String,

    /// Maximum borrowable amount.
    pub amount: f64,

    /// Borrow limit (if applicable).
    pub borrow_limit: Option<f64>,

    /// Trading pair symbol (isolated margin only).
    pub symbol: Option<String>,

    /// Query timestamp in milliseconds.
    pub timestamp: i64,

    /// ISO 8601 datetime string.
    pub datetime: String,

    /// Raw exchange response data.
    pub info: serde_json::Value,
}

/// Maximum transferable amount information.
///
/// Contains details about the maximum amount that can be transferred between accounts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaxTransferable {
    /// Currency code.
    pub currency: String,

    /// Maximum transferable amount.
    pub amount: f64,

    /// Trading pair symbol (isolated margin only).
    pub symbol: Option<String>,

    /// Query timestamp in milliseconds.
    pub timestamp: i64,

    /// ISO 8601 datetime string.
    pub datetime: String,

    /// Raw exchange response data.
    pub info: serde_json::Value,
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_balance_entry() {
        let entry = BalanceEntry::new(dec!(10.5), dec!(2.5));
        assert_eq!(entry.free, dec!(10.5));
        assert_eq!(entry.used, dec!(2.5));
        assert_eq!(entry.total, dec!(13.0));
    }

    #[test]
    fn test_balance_operations() {
        let mut balance = Balance::new();
        balance.set("BTC".to_string(), BalanceEntry::new(dec!(1.5), dec!(0.5)));

        let btc = balance.get("BTC").unwrap();
        assert_eq!(btc.total, dec!(2.0));
    }
}
