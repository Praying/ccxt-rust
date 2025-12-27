//! Transfer-related type definitions.
//!
//! Provides data structures for inter-account transfers, transfer history queries,
//! and deposit/withdrawal fee information.

use serde::{Deserialize, Serialize};

/// Transfer record between accounts.
///
/// Represents a complete inter-account transfer including source account,
/// destination account, and transfer amount.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Transfer {
    /// Transfer ID.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,

    /// Timestamp in milliseconds.
    pub timestamp: i64,

    /// ISO 8601 datetime string.
    pub datetime: String,

    /// Currency code.
    pub currency: String,

    /// Transfer amount.
    pub amount: f64,

    /// Source account type (e.g., spot, margin, futures, funding, option).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from_account: Option<String>,

    /// Destination account type (e.g., spot, margin, futures, funding, option).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub to_account: Option<String>,

    /// Transfer status (e.g., pending, confirmed, failed).
    pub status: String,

    /// Raw exchange response data.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub info: Option<serde_json::Value>,
}

impl Transfer {
    /// Creates a new transfer record.
    pub fn new(
        id: Option<String>,
        timestamp: i64,
        datetime: String,
        currency: String,
        amount: f64,
        from_account: Option<String>,
        to_account: Option<String>,
        status: String,
    ) -> Self {
        Self {
            id,
            timestamp,
            datetime,
            currency,
            amount,
            from_account,
            to_account,
            status,
            info: None,
        }
    }
}

/// Deposit and withdrawal fee information.
///
/// Contains deposit and withdrawal fee details for a specific currency.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DepositWithdrawFee {
    /// Currency code.
    pub currency: String,

    /// Withdrawal fee amount.
    pub withdraw_fee: f64,

    /// Minimum withdrawal amount.
    pub withdraw_min: f64,

    /// Maximum withdrawal amount.
    pub withdraw_max: f64,

    /// Whether deposits are enabled.
    pub deposit_enable: bool,

    /// Whether withdrawals are enabled.
    pub withdraw_enable: bool,

    /// List of supported networks.
    pub networks: Vec<NetworkInfo>,

    /// Raw exchange response data.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub info: Option<serde_json::Value>,
}

/// Blockchain network information.
///
/// Describes deposit and withdrawal details for a specific blockchain network.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkInfo {
    /// Network identifier (e.g., BTC, ETH, TRC20, BEP20).
    pub network: String,

    /// Network name (e.g., Bitcoin, Ethereum).
    pub name: String,

    /// Withdrawal fee for this network.
    pub withdraw_fee: f64,

    /// Minimum withdrawal amount for this network.
    pub withdraw_min: f64,

    /// Maximum withdrawal amount for this network.
    pub withdraw_max: f64,

    /// Whether deposits are enabled on this network.
    pub deposit_enable: bool,

    /// Whether withdrawals are enabled on this network.
    pub withdraw_enable: bool,

    /// Number of confirmations required for deposit credit.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deposit_confirmations: Option<u32>,

    /// Number of confirmations required for withdrawal unlock.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub withdraw_confirmations: Option<u32>,
}

impl NetworkInfo {
    /// Creates a new network information record.
    pub fn new(
        network: String,
        name: String,
        withdraw_fee: f64,
        withdraw_min: f64,
        withdraw_max: f64,
        deposit_enable: bool,
        withdraw_enable: bool,
    ) -> Self {
        Self {
            network,
            name,
            withdraw_fee,
            withdraw_min,
            withdraw_max,
            deposit_enable,
            withdraw_enable,
            deposit_confirmations: None,
            withdraw_confirmations: None,
        }
    }
}

/// Transfer type for inter-account transfers.
///
/// Supported transfer types between different Binance account types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransferType {
    /// Spot account to USDⓈ-M futures account.
    #[serde(rename = "MAIN_UMFUTURE")]
    MainToUmFuture,

    /// USDⓈ-M futures account to spot account.
    #[serde(rename = "UMFUTURE_MAIN")]
    UmFutureToMain,

    /// Spot account to COIN-M futures account.
    #[serde(rename = "MAIN_CMFUTURE")]
    MainToCmFuture,

    /// COIN-M futures account to spot account.
    #[serde(rename = "CMFUTURE_MAIN")]
    CmFutureToMain,

    /// Spot account to cross margin account.
    #[serde(rename = "MAIN_MARGIN")]
    MainToMargin,

    /// Cross margin account to spot account.
    #[serde(rename = "MARGIN_MAIN")]
    MarginToMain,

    /// Spot account to isolated margin account.
    #[serde(rename = "MAIN_ISOLATED_MARGIN")]
    MainToIsolatedMargin,

    /// Isolated margin account to spot account.
    #[serde(rename = "ISOLATED_MARGIN_MAIN")]
    IsolatedMarginToMain,

    /// Spot account to funding account.
    #[serde(rename = "MAIN_FUNDING")]
    MainToFunding,

    /// Funding account to spot account.
    #[serde(rename = "FUNDING_MAIN")]
    FundingToMain,

    /// USDⓈ-M futures account to funding account.
    #[serde(rename = "UMFUTURE_FUNDING")]
    UmFutureToFunding,

    /// Funding account to USDⓈ-M futures account.
    #[serde(rename = "FUNDING_UMFUTURE")]
    FundingToUmFuture,

    /// Cross margin account to funding account.
    #[serde(rename = "MARGIN_FUNDING")]
    MarginToFunding,

    /// Funding account to cross margin account.
    #[serde(rename = "FUNDING_MARGIN")]
    FundingToMargin,

    /// Spot account to options account.
    #[serde(rename = "MAIN_OPTION")]
    MainToOption,

    /// Options account to spot account.
    #[serde(rename = "OPTION_MAIN")]
    OptionToMain,
}

impl TransferType {
    /// Parses transfer type from account type strings.
    pub fn from_accounts(from: &str, to: &str) -> Option<Self> {
        match (from.to_uppercase().as_str(), to.to_uppercase().as_str()) {
            ("SPOT" | "MAIN", "FUTURES" | "UMFUTURE") => Some(Self::MainToUmFuture),
            ("FUTURES" | "UMFUTURE", "SPOT" | "MAIN") => Some(Self::UmFutureToMain),
            ("SPOT" | "MAIN", "DELIVERY" | "CMFUTURE") => Some(Self::MainToCmFuture),
            ("DELIVERY" | "CMFUTURE", "SPOT" | "MAIN") => Some(Self::CmFutureToMain),
            ("SPOT" | "MAIN", "MARGIN") => Some(Self::MainToMargin),
            ("MARGIN", "SPOT" | "MAIN") => Some(Self::MarginToMain),
            ("SPOT" | "MAIN", "ISOLATED_MARGIN") => Some(Self::MainToIsolatedMargin),
            ("ISOLATED_MARGIN", "SPOT" | "MAIN") => Some(Self::IsolatedMarginToMain),
            ("SPOT" | "MAIN", "FUNDING") => Some(Self::MainToFunding),
            ("FUNDING", "SPOT" | "MAIN") => Some(Self::FundingToMain),
            ("FUTURES" | "UMFUTURE", "FUNDING") => Some(Self::UmFutureToFunding),
            ("FUNDING", "FUTURES" | "UMFUTURE") => Some(Self::FundingToUmFuture),
            ("MARGIN", "FUNDING") => Some(Self::MarginToFunding),
            ("FUNDING", "MARGIN") => Some(Self::FundingToMargin),
            ("SPOT" | "MAIN", "OPTION") => Some(Self::MainToOption),
            ("OPTION", "SPOT" | "MAIN") => Some(Self::OptionToMain),
            _ => None,
        }
    }

    /// Returns the string representation of the transfer type.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::MainToUmFuture => "MAIN_UMFUTURE",
            Self::UmFutureToMain => "UMFUTURE_MAIN",
            Self::MainToCmFuture => "MAIN_CMFUTURE",
            Self::CmFutureToMain => "CMFUTURE_MAIN",
            Self::MainToMargin => "MAIN_MARGIN",
            Self::MarginToMain => "MARGIN_MAIN",
            Self::MainToIsolatedMargin => "MAIN_ISOLATED_MARGIN",
            Self::IsolatedMarginToMain => "ISOLATED_MARGIN_MAIN",
            Self::MainToFunding => "MAIN_FUNDING",
            Self::FundingToMain => "FUNDING_MAIN",
            Self::UmFutureToFunding => "UMFUTURE_FUNDING",
            Self::FundingToUmFuture => "FUNDING_UMFUTURE",
            Self::MarginToFunding => "MARGIN_FUNDING",
            Self::FundingToMargin => "FUNDING_MARGIN",
            Self::MainToOption => "MAIN_OPTION",
            Self::OptionToMain => "OPTION_MAIN",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transfer_type_from_accounts() {
        assert_eq!(
            TransferType::from_accounts("spot", "futures"),
            Some(TransferType::MainToUmFuture)
        );
        assert_eq!(
            TransferType::from_accounts("MAIN", "MARGIN"),
            Some(TransferType::MainToMargin)
        );
        assert_eq!(TransferType::from_accounts("invalid", "accounts"), None);
    }

    #[test]
    fn test_transfer_type_as_str() {
        assert_eq!(TransferType::MainToUmFuture.as_str(), "MAIN_UMFUTURE");
        assert_eq!(TransferType::MarginToMain.as_str(), "MARGIN_MAIN");
    }
}
