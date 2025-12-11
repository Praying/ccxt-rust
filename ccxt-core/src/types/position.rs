//! Position type definitions for futures trading.
//!
//! Provides data structures for managing trading positions, including
//! margin types, position sides, leverage configuration, and position status.

use serde::{Deserialize, Serialize};

/// Margin type for futures positions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MarginType {
    /// Isolated margin: position uses only allocated margin.
    Isolated,
    /// Cross margin: position uses all available account balance.
    Cross,
}

impl Default for MarginType {
    fn default() -> Self {
        MarginType::Cross
    }
}

impl std::fmt::Display for MarginType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MarginType::Isolated => write!(f, "isolated"),
            MarginType::Cross => write!(f, "cross"),
        }
    }
}

impl std::str::FromStr for MarginType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "isolated" => Ok(MarginType::Isolated),
            "cross" | "crossed" => Ok(MarginType::Cross),
            _ => Err(format!("Invalid margin type: {}", s)),
        }
    }
}

/// Position side for dual-side position mode.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "UPPERCASE")]
pub enum PositionSide {
    /// Long position.
    Long,
    /// Short position.
    Short,
    /// Both sides (one-way position mode).
    Both,
}

impl Default for PositionSide {
    fn default() -> Self {
        PositionSide::Both
    }
}

impl std::fmt::Display for PositionSide {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PositionSide::Long => write!(f, "LONG"),
            PositionSide::Short => write!(f, "SHORT"),
            PositionSide::Both => write!(f, "BOTH"),
        }
    }
}

impl std::str::FromStr for PositionSide {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "LONG" => Ok(PositionSide::Long),
            "SHORT" => Ok(PositionSide::Short),
            "BOTH" => Ok(PositionSide::Both),
            _ => Err(format!("Invalid position side: {}", s)),
        }
    }
}

/// Trading position information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    /// Raw exchange response data.
    pub info: serde_json::Value,

    /// Position ID.
    pub id: Option<String>,

    /// Trading symbol (e.g., "BTC/USDT:USDT").
    pub symbol: String,

    /// Position direction: "long" or "short".
    pub side: Option<String>,

    /// Position side for dual-side mode.
    pub position_side: Option<PositionSide>,

    /// Whether dual-side position mode is enabled.
    pub dual_side_position: Option<bool>,

    /// Number of contracts (positive for long, negative for short).
    pub contracts: Option<f64>,

    /// Size of each contract.
    pub contract_size: Option<f64>,

    /// Entry price of the position.
    pub entry_price: Option<f64>,

    /// Current mark price.
    pub mark_price: Option<f64>,

    /// Notional value of the position.
    pub notional: Option<f64>,

    /// Leverage multiplier.
    pub leverage: Option<f64>,

    /// Collateral/margin amount.
    pub collateral: Option<f64>,

    /// Initial margin required.
    pub initial_margin: Option<f64>,

    /// Initial margin as a percentage.
    pub initial_margin_percentage: Option<f64>,

    /// Maintenance margin required.
    pub maintenance_margin: Option<f64>,

    /// Maintenance margin as a percentage.
    pub maintenance_margin_percentage: Option<f64>,

    /// Unrealized profit and loss.
    pub unrealized_pnl: Option<f64>,

    /// Realized profit and loss.
    pub realized_pnl: Option<f64>,

    /// Liquidation price.
    pub liquidation_price: Option<f64>,

    /// Margin ratio.
    pub margin_ratio: Option<f64>,

    /// Margin mode: "cross" or "isolated".
    pub margin_mode: Option<String>,

    /// Whether hedge mode is enabled.
    pub hedged: Option<bool>,

    /// Profit/loss as a percentage.
    pub percentage: Option<f64>,

    /// Timestamp in milliseconds.
    pub timestamp: Option<u64>,

    /// ISO 8601 datetime string.
    pub datetime: Option<String>,
}
/// Leverage configuration for a trading symbol.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Leverage {
    /// Raw exchange API response.
    pub info: serde_json::Value,

    /// Trading symbol.
    pub symbol: String,

    /// Margin mode (cross or isolated).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub margin_mode: Option<MarginType>,

    /// Leverage multiplier for long positions.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub long_leverage: Option<i64>,

    /// Leverage multiplier for short positions.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub short_leverage: Option<i64>,

    /// Timestamp in milliseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<i64>,

    /// ISO 8601 datetime string.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub datetime: Option<String>,
}

impl Default for Position {
    fn default() -> Self {
        Self {
            info: serde_json::Value::Null,
            id: None,
            symbol: String::new(),
            side: None,
            position_side: None,
            dual_side_position: None,
            contracts: None,
            contract_size: None,
            entry_price: None,
            mark_price: None,
            notional: None,
            leverage: None,
            collateral: None,
            initial_margin: None,
            initial_margin_percentage: None,
            maintenance_margin: None,
            maintenance_margin_percentage: None,
            unrealized_pnl: None,
            realized_pnl: None,
            liquidation_price: None,
            margin_ratio: None,
            margin_mode: None,
            hedged: None,
            percentage: None,
            timestamp: None,
            datetime: None,
        }
    }
}
