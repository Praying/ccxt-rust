use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// Transaction type for deposits and withdrawals.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum TransactionType {
    /// Deposit to exchange account.
    Deposit,
    /// Withdrawal from exchange account.
    Withdrawal,
}

/// Transaction processing status.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum TransactionStatus {
    /// Pending confirmation.
    Pending,
    /// Successfully completed.
    Ok,
    /// Failed to process.
    Failed,
    /// Canceled by user or exchange.
    Canceled,
}

/// Transaction fee information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionFee {
    /// Fee currency code.
    pub currency: String,
    /// Fee amount.
    pub cost: Decimal,
}

/// Transaction record for deposits and withdrawals.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    /// Raw exchange response data.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub info: Option<serde_json::Value>,
    /// Exchange-assigned transaction ID.
    pub id: String,
    /// Blockchain transaction hash.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub txid: Option<String>,
    /// Transaction timestamp in milliseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<i64>,
    /// ISO 8601 datetime string.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub datetime: Option<String>,
    /// Network or chain name (e.g., ERC20, TRC20).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub network: Option<String>,
    /// Blockchain address.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub address: Option<String>,
    /// Destination address for withdrawals.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub address_to: Option<String>,
    /// Source address for deposits.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub address_from: Option<String>,
    /// Address memo or tag.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tag: Option<String>,
    /// Destination address tag.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tag_to: Option<String>,
    /// Source address tag.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tag_from: Option<String>,
    /// Transaction type (deposit or withdrawal).
    #[serde(rename = "type")]
    pub transaction_type: TransactionType,
    /// Transaction amount.
    pub amount: Decimal,
    /// Currency code.
    pub currency: String,
    /// Transaction status.
    pub status: TransactionStatus,
    /// Last update timestamp in milliseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated: Option<i64>,
    /// Whether this is an internal transfer.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub internal: Option<bool>,
    /// Additional notes or comments.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
    /// Transaction fee details.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fee: Option<TransactionFee>,
}

impl Transaction {
    /// Creates a new transaction record.
    pub fn new(
        id: String,
        transaction_type: TransactionType,
        amount: Decimal,
        currency: String,
        status: TransactionStatus,
    ) -> Self {
        Self {
            info: None,
            id,
            txid: None,
            timestamp: None,
            datetime: None,
            network: None,
            address: None,
            address_to: None,
            address_from: None,
            tag: None,
            tag_to: None,
            tag_from: None,
            transaction_type,
            amount,
            currency,
            status,
            updated: None,
            internal: None,
            comment: None,
            fee: None,
        }
    }

    /// Returns `true` if this is a deposit transaction.
    pub fn is_deposit(&self) -> bool {
        matches!(self.transaction_type, TransactionType::Deposit)
    }

    /// Returns `true` if this is a withdrawal transaction.
    pub fn is_withdrawal(&self) -> bool {
        matches!(self.transaction_type, TransactionType::Withdrawal)
    }

    /// Returns `true` if this is an internal transfer.
    pub fn is_internal(&self) -> bool {
        self.internal.unwrap_or(false)
    }

    /// Returns `true` if the transaction is completed successfully.
    pub fn is_completed(&self) -> bool {
        matches!(self.status, TransactionStatus::Ok)
    }

    /// Returns `true` if the transaction is pending confirmation.
    pub fn is_pending(&self) -> bool {
        matches!(self.status, TransactionStatus::Pending)
    }

    /// Returns `true` if the transaction failed.
    pub fn is_failed(&self) -> bool {
        matches!(self.status, TransactionStatus::Failed)
    }

    /// Returns `true` if the transaction was canceled.
    pub fn is_canceled(&self) -> bool {
        matches!(self.status, TransactionStatus::Canceled)
    }
}

/// Deposit address information for receiving funds.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DepositAddress {
    /// Raw exchange response data.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub info: Option<serde_json::Value>,
    /// Currency code.
    pub currency: String,
    /// Network or chain name (e.g., ERC20, TRC20).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub network: Option<String>,
    /// Deposit address.
    pub address: String,
    /// Address memo or tag (required for some currencies).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tag: Option<String>,
}

impl DepositAddress {
    /// Creates a new deposit address.
    pub fn new(currency: String, address: String) -> Self {
        Self {
            info: None,
            currency,
            network: None,
            address,
            tag: None,
        }
    }

    /// Returns the full address string including tag if present.
    pub fn full_address(&self) -> String {
        if let Some(tag) = &self.tag {
            format!("{}:{}", self.address, tag)
        } else {
            self.address.clone()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_transaction_creation() {
        let transaction = Transaction::new(
            "test_id".to_string(),
            TransactionType::Deposit,
            dec!(100.5),
            "USDT".to_string(),
            TransactionStatus::Ok,
        );

        assert_eq!(transaction.id, "test_id");
        assert_eq!(transaction.amount, dec!(100.5));
        assert_eq!(transaction.currency, "USDT");
        assert!(transaction.is_deposit());
        assert!(!transaction.is_withdrawal());
        assert!(transaction.is_completed());
    }

    #[test]
    fn test_transaction_status_checks() {
        let mut transaction = Transaction::new(
            "test_id".to_string(),
            TransactionType::Withdrawal,
            dec!(50.0),
            "BTC".to_string(),
            TransactionStatus::Pending,
        );

        assert!(transaction.is_pending());
        assert!(!transaction.is_completed());
        assert!(!transaction.is_failed());
        assert!(!transaction.is_canceled());

        transaction.status = TransactionStatus::Ok;
        assert!(transaction.is_completed());
        assert!(!transaction.is_pending());
    }

    #[test]
    fn test_deposit_address_creation() {
        let address = DepositAddress::new(
            "BTC".to_string(),
            "1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa".to_string(),
        );

        assert_eq!(address.currency, "BTC");
        assert_eq!(address.address, "1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa");
        assert_eq!(address.full_address(), "1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa");
    }

    #[test]
    fn test_deposit_address_with_tag() {
        let mut address = DepositAddress::new(
            "XRP".to_string(),
            "rEb8TK3gBgk5auZkwc6sHnwrGVJH8DuaLh".to_string(),
        );
        address.tag = Some("108618262".to_string());

        assert_eq!(
            address.full_address(),
            "rEb8TK3gBgk5auZkwc6sHnwrGVJH8DuaLh:108618262"
        );
    }

    #[test]
    fn test_internal_transfer() {
        let mut transaction = Transaction::new(
            "test_id".to_string(),
            TransactionType::Deposit,
            dec!(100.0),
            "USDT".to_string(),
            TransactionStatus::Ok,
        );

        assert!(!transaction.is_internal());
        transaction.internal = Some(true);
        assert!(transaction.is_internal());
    }
}
