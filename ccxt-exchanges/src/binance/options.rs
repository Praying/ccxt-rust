//! Binance-specific options and configuration.

use ccxt_core::types::default_type::{DefaultSubType, DefaultType};
use serde::{Deserialize, Deserializer, Serialize};
use std::str::FromStr;

/// Binance-specific options.
///
/// # Example
///
/// ```rust
/// use ccxt_exchanges::binance::BinanceOptions;
/// use ccxt_core::types::default_type::{DefaultType, DefaultSubType};
///
/// let options = BinanceOptions {
///     default_type: DefaultType::Swap,
///     default_sub_type: Some(DefaultSubType::Linear),
///     ..Default::default()
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BinanceOptions {
    /// Enables time synchronization.
    pub adjust_for_time_difference: bool,
    /// Receive window in milliseconds.
    pub recv_window: u64,
    /// Default trading type (spot/margin/swap/futures/option).
    ///
    /// This determines which API endpoints to use for operations.
    /// Supports both `DefaultType` enum and string values for backward compatibility.
    #[serde(deserialize_with = "deserialize_default_type")]
    pub default_type: DefaultType,
    /// Default sub-type for contract settlement (linear/inverse).
    ///
    /// - `Linear`: USDT-margined contracts (FAPI)
    /// - `Inverse`: Coin-margined contracts (DAPI)
    ///
    /// Only applicable when `default_type` is `Swap`, `Futures`, or `Option`.
    /// Ignored for `Spot` and `Margin` types.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_sub_type: Option<DefaultSubType>,
    /// Enables testnet mode.
    pub test: bool,
    /// Time sync interval in seconds.
    ///
    /// Controls how often the time offset is refreshed when auto sync is enabled.
    /// Default: 30 seconds.
    #[serde(default = "default_sync_interval")]
    pub time_sync_interval_secs: u64,
    /// Enable automatic periodic time sync.
    ///
    /// When enabled, the time offset will be automatically refreshed
    /// based on `time_sync_interval_secs`.
    /// Default: true.
    #[serde(default = "default_auto_sync")]
    pub auto_time_sync: bool,
    /// Rate limit in requests per second.
    ///
    /// Default: 50.
    #[serde(default = "default_rate_limit")]
    pub rate_limit: u32,
}

fn default_sync_interval() -> u64 {
    30
}

fn default_auto_sync() -> bool {
    true
}

fn default_rate_limit() -> u32 {
    50
}

/// Custom deserializer for DefaultType that accepts both enum values and strings.
///
/// This provides backward compatibility with configurations that use string values
/// like "spot", "future", "swap", etc.
fn deserialize_default_type<'de, D>(deserializer: D) -> std::result::Result<DefaultType, D::Error>
where
    D: Deserializer<'de>,
{
    use serde::de::Error;

    // First try to deserialize as a string (for backward compatibility)
    // Then try as the enum directly
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum StringOrDefaultType {
        String(String),
        DefaultType(DefaultType),
    }

    match StringOrDefaultType::deserialize(deserializer)? {
        StringOrDefaultType::String(s) => {
            // Handle legacy "future" value (map to Swap for perpetuals)
            let lowercase = s.to_lowercase();
            let normalized = match lowercase.as_str() {
                "future" => "swap",      // Legacy: "future" typically meant perpetual futures
                "delivery" => "futures", // Legacy: "delivery" meant dated futures
                _ => lowercase.as_str(),
            };
            DefaultType::from_str(normalized).map_err(|e| D::Error::custom(e.to_string()))
        }
        StringOrDefaultType::DefaultType(dt) => Ok(dt),
    }
}

impl Default for BinanceOptions {
    fn default() -> Self {
        Self {
            adjust_for_time_difference: false,
            recv_window: 5000,
            default_type: DefaultType::default(), // Defaults to Spot
            default_sub_type: None,
            test: false,
            time_sync_interval_secs: default_sync_interval(),
            auto_time_sync: default_auto_sync(),
            rate_limit: default_rate_limit(),
        }
    }
}
