//! Ledger entry type definitions.
//!
//! This module contains types for account ledger entries tracking
//! all balance changes including trades, fees, transfers, and settlements.

use serde::{Deserialize, Serialize};

/// Ledger entry direction.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum LedgerDirection {
    /// Incoming (credit).
    In,
    /// Outgoing (debit).
    Out,
}

/// Ledger entry type (CCXT standardized types).
///
/// Represents the type of transaction recorded in the ledger.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum LedgerEntryType {
    /// Trade execution.
    Trade,
    /// Trading fee.
    Fee,
    /// Account transfer.
    Transfer,
    /// Deposit.
    Deposit,
    /// Withdrawal.
    Withdrawal,
    /// Position settlement.
    Settlement,
    /// Cashback reward.
    Cashback,
    /// Trading rebate.
    Rebate,
    /// Referral reward.
    Referral,
    /// Commission payment.
    Commission,
}

/// Ledger entry structure.
///
/// Contains a single ledger record with balance change information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LedgerEntry {
    /// Raw exchange response data.
    pub info: serde_json::Value,

    /// Ledger entry ID.
    pub id: String,

    /// Timestamp in milliseconds.
    pub timestamp: i64,

    /// ISO 8601 datetime string.
    pub datetime: String,

    /// Transaction direction (in/out).
    pub direction: LedgerDirection,

    /// Currency code.
    pub currency: String,

    /// Amount (absolute value).
    pub amount: f64,

    /// Ledger entry type.
    #[serde(rename = "type")]
    pub type_: LedgerEntryType,

    /// Account type.
    pub account: Option<String>,

    /// Reference account.
    pub reference_account: Option<String>,

    /// Reference ID (order ID, trade ID, etc.).
    pub reference_id: Option<String>,

    /// Balance before transaction.
    pub before: Option<f64>,

    /// Balance after transaction.
    pub after: Option<f64>,

    /// Status.
    pub status: Option<String>,

    /// Fee amount.
    pub fee: Option<f64>,
}
