// ccxt-core/src/types/account.rs
//! Account configuration type definitions.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Account configuration information.
///
/// Contains configuration parameters for futures accounts, including multi-asset mode
/// and fee tier settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountConfig {
    /// Raw exchange response data.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub info: Option<Value>,

    /// Multi-asset margin mode (true = multi-asset enabled, false = single-asset mode).
    pub multi_assets_margin: bool,

    /// Fee tier level (0-9, higher tier means lower fees).
    pub fee_tier: i32,

    /// Whether trading is enabled.
    pub can_trade: bool,

    /// Whether deposits are enabled.
    pub can_deposit: bool,

    /// Whether withdrawals are enabled.
    pub can_withdraw: bool,

    /// Configuration update timestamp in milliseconds.
    pub update_time: i64,
}

impl Default for AccountConfig {
    fn default() -> Self {
        Self {
            info: None,
            multi_assets_margin: false,
            fee_tier: 0,
            can_trade: true,
            can_deposit: true,
            can_withdraw: true,
            update_time: 0,
        }
    }
}

/// Commission rate information.
///
/// Contains maker and taker commission rates for a specific trading pair.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommissionRate {
    /// Raw exchange response data.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub info: Option<Value>,

    /// Trading pair symbol.
    pub symbol: String,

    /// Maker commission rate (e.g., 0.0002 = 0.02%).
    pub maker_commission_rate: f64,

    /// Taker commission rate (e.g., 0.0004 = 0.04%).
    pub taker_commission_rate: f64,
}

impl Default for CommissionRate {
    fn default() -> Self {
        Self {
            info: None,
            symbol: String::new(),
            maker_commission_rate: 0.0,
            taker_commission_rate: 0.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_account_config_default() {
        let config = AccountConfig::default();
        assert!(!config.multi_assets_margin);
        assert_eq!(config.fee_tier, 0);
        assert!(config.can_trade);
        assert!(config.can_deposit);
        assert!(config.can_withdraw);
    }

    #[test]
    fn test_commission_rate_default() {
        let rate = CommissionRate::default();
        assert_eq!(rate.symbol, "");
        assert_eq!(rate.maker_commission_rate, 0.0);
        assert_eq!(rate.taker_commission_rate, 0.0);
    }
}
