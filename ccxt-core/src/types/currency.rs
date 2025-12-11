//! Currency type definitions
//!
//! This module defines the `Currency` structure which represents a cryptocurrency
//! or fiat currency on an exchange.

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Precision mode for currency operations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PrecisionMode {
    /// Decimal places precision
    DecimalPlaces,
    /// Significant digits precision
    SignificantDigits,
    /// Tick size precision
    TickSize,
}

impl Default for PrecisionMode {
    fn default() -> Self {
        Self::DecimalPlaces
    }
}

impl std::fmt::Display for PrecisionMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DecimalPlaces => write!(f, "DECIMAL_PLACES"),
            Self::SignificantDigits => write!(f, "SIGNIFICANT_DIGITS"),
            Self::TickSize => write!(f, "TICK_SIZE"),
        }
    }
}

/// Min/Max range for limits
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MinMax {
    /// Minimum value
    pub min: Option<Decimal>,
    /// Maximum value
    pub max: Option<Decimal>,
}

impl MinMax {
    /// Create a new min/max range
    pub fn new(min: Option<Decimal>, max: Option<Decimal>) -> Self {
        Self { min, max }
    }

    /// Check if value is within range
    pub fn is_valid(&self, value: Decimal) -> bool {
        let min_ok = self.min.map_or(true, |min| value >= min);
        let max_ok = self.max.map_or(true, |max| value <= max);
        min_ok && max_ok
    }
}

impl Default for MinMax {
    fn default() -> Self {
        Self {
            min: None,
            max: None,
        }
    }
}

/// Currency network/chain information
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CurrencyNetwork {
    /// Network name (e.g., "ERC20", "TRC20", "BEP20")
    pub network: String,

    /// Network ID on exchange
    pub id: Option<String>,

    /// Network name on exchange
    pub name: Option<String>,

    /// Is network active for deposits/withdrawals
    pub active: bool,

    /// Deposit enabled
    pub deposit: bool,

    /// Withdrawal enabled
    pub withdraw: bool,

    /// Withdrawal fee
    pub fee: Option<Decimal>,

    /// Precision for this network
    pub precision: Option<Decimal>,

    /// Withdrawal limits
    pub limits: MinMax,

    /// Additional network info
    #[serde(flatten)]
    pub info: HashMap<String, serde_json::Value>,
}

impl CurrencyNetwork {
    /// Create a new currency network
    pub fn new(network: String) -> Self {
        Self {
            network,
            id: None,
            name: None,
            active: true,
            deposit: true,
            withdraw: true,
            fee: None,
            precision: None,
            limits: MinMax::default(),
            info: HashMap::new(),
        }
    }
}

/// Currency structure representing a tradable asset
///
/// This corresponds to Go's currency metadata and contains information
/// about a cryptocurrency or fiat currency on an exchange.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Currency {
    /// Currency code (e.g., "BTC", "USDT")
    pub code: String,

    /// Exchange-specific currency ID
    pub id: String,

    /// Full currency name
    pub name: Option<String>,

    /// Is currency active for trading
    pub active: bool,

    /// Deposit enabled
    pub deposit: bool,

    /// Withdrawal enabled
    pub withdraw: bool,

    /// Trading fee for this currency
    pub fee: Option<Decimal>,

    /// Precision (decimal places)
    pub precision: Option<Decimal>,

    /// Withdrawal/deposit limits
    pub limits: MinMax,

    /// Available networks for this currency
    pub networks: HashMap<String, CurrencyNetwork>,

    /// Currency type (crypto/fiat)
    #[serde(rename = "type")]
    pub currency_type: Option<String>,

    /// Raw exchange info
    #[serde(flatten)]
    pub info: HashMap<String, serde_json::Value>,
}

impl Currency {
    /// Create a new currency
    pub fn new(code: String, id: String) -> Self {
        Self {
            code: code.clone(),
            id,
            name: Some(code),
            active: true,
            deposit: true,
            withdraw: true,
            fee: None,
            precision: None,
            limits: MinMax::default(),
            networks: HashMap::new(),
            currency_type: Some("crypto".to_string()),
            info: HashMap::new(),
        }
    }

    /// Create a new cryptocurrency
    pub fn new_crypto(code: String, id: String, name: String) -> Self {
        Self {
            code,
            id,
            name: Some(name),
            active: true,
            deposit: true,
            withdraw: true,
            fee: None,
            precision: None,
            limits: MinMax::default(),
            networks: HashMap::new(),
            currency_type: Some("crypto".to_string()),
            info: HashMap::new(),
        }
    }

    /// Create a new fiat currency
    pub fn new_fiat(code: String, id: String, name: String) -> Self {
        Self {
            code,
            id,
            name: Some(name),
            active: true,
            deposit: true,
            withdraw: true,
            fee: None,
            precision: None,
            limits: MinMax::default(),
            networks: HashMap::new(),
            currency_type: Some("fiat".to_string()),
            info: HashMap::new(),
        }
    }

    /// Check if currency is active
    pub fn is_active(&self) -> bool {
        self.active
    }

    /// Check if deposit is enabled
    pub fn can_deposit(&self) -> bool {
        self.deposit && self.active
    }

    /// Check if withdrawal is enabled
    pub fn can_withdraw(&self) -> bool {
        self.withdraw && self.active
    }

    /// Check if currency is a cryptocurrency
    pub fn is_crypto(&self) -> bool {
        self.currency_type.as_deref() == Some("crypto")
    }

    /// Check if currency is a fiat currency
    pub fn is_fiat(&self) -> bool {
        self.currency_type.as_deref() == Some("fiat")
    }

    /// Add a network to this currency
    pub fn add_network(&mut self, network: CurrencyNetwork) {
        self.networks.insert(network.network.clone(), network);
    }

    /// Get a specific network
    pub fn get_network(&self, network: &str) -> Option<&CurrencyNetwork> {
        self.networks.get(network)
    }

    /// Get all available networks
    pub fn available_networks(&self) -> Vec<&CurrencyNetwork> {
        self.networks.values().filter(|n| n.active).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_precision_mode_display() {
        assert_eq!(PrecisionMode::DecimalPlaces.to_string(), "DECIMAL_PLACES");
        assert_eq!(
            PrecisionMode::SignificantDigits.to_string(),
            "SIGNIFICANT_DIGITS"
        );
        assert_eq!(PrecisionMode::TickSize.to_string(), "TICK_SIZE");
    }

    #[test]
    fn test_min_max_validation() {
        let limits = MinMax::new(Some(dec!(0.01)), Some(dec!(100)));

        assert!(limits.is_valid(dec!(1.0)));
        assert!(limits.is_valid(dec!(0.01)));
        assert!(limits.is_valid(dec!(100)));
        assert!(!limits.is_valid(dec!(0.001)));
        assert!(!limits.is_valid(dec!(101)));
    }

    #[test]
    fn test_currency_creation() {
        let currency = Currency::new("BTC".to_string(), "btc".to_string());

        assert_eq!(currency.code, "BTC");
        assert_eq!(currency.id, "btc");
        assert!(currency.is_active());
        assert!(currency.can_deposit());
        assert!(currency.can_withdraw());
        assert!(currency.is_crypto());
        assert!(!currency.is_fiat());
    }

    #[test]
    fn test_currency_crypto() {
        let currency =
            Currency::new_crypto("ETH".to_string(), "eth".to_string(), "Ethereum".to_string());

        assert_eq!(currency.name, Some("Ethereum".to_string()));
        assert!(currency.is_crypto());
    }

    #[test]
    fn test_currency_fiat() {
        let currency = Currency::new_fiat(
            "USD".to_string(),
            "usd".to_string(),
            "US Dollar".to_string(),
        );

        assert!(currency.is_fiat());
        assert!(!currency.is_crypto());
    }

    #[test]
    fn test_currency_network() {
        let mut currency = Currency::new("USDT".to_string(), "usdt".to_string());

        let mut network = CurrencyNetwork::new("ERC20".to_string());
        network.fee = Some(dec!(5.0));
        network.limits = MinMax::new(Some(dec!(10)), Some(dec!(10000)));

        currency.add_network(network);

        assert_eq!(currency.networks.len(), 1);
        assert!(currency.get_network("ERC20").is_some());

        let erc20 = currency.get_network("ERC20").unwrap();
        assert_eq!(erc20.fee, Some(dec!(5.0)));
        assert!(erc20.limits.is_valid(dec!(100)));
    }

    #[test]
    fn test_available_networks() {
        let mut currency = Currency::new("USDT".to_string(), "usdt".to_string());

        let mut erc20 = CurrencyNetwork::new("ERC20".to_string());
        erc20.active = true;
        currency.add_network(erc20);

        let mut trc20 = CurrencyNetwork::new("TRC20".to_string());
        trc20.active = false;
        currency.add_network(trc20);

        let mut bep20 = CurrencyNetwork::new("BEP20".to_string());
        bep20.active = true;
        currency.add_network(bep20);

        let available = currency.available_networks();
        assert_eq!(available.len(), 2);
    }
}
