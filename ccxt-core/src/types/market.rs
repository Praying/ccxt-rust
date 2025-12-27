//! Market type definitions
//!
//! This module defines the `Market` structure which represents a trading pair
//! on a cryptocurrency exchange, along with its associated metadata.

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::symbol::ParsedSymbol;
use super::{MinMax, Symbol};
use crate::symbol::SymbolParser;

/// Market type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MarketType {
    /// Spot market
    Spot,
    /// Futures market
    Futures,
    /// Swap/Perpetual market
    Swap,
    /// Options market
    Option,
}

impl std::fmt::Display for MarketType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Spot => write!(f, "spot"),
            Self::Futures => write!(f, "futures"),
            Self::Swap => write!(f, "swap"),
            Self::Option => write!(f, "option"),
        }
    }
}

/// Market precision settings
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MarketPrecision {
    /// Price precision (decimal places or tick size)
    pub price: Option<Decimal>,
    /// Amount precision (decimal places or step size)
    pub amount: Option<Decimal>,
    /// Base currency precision
    pub base: Option<u32>,
    /// Quote currency precision
    pub quote: Option<u32>,
}

impl Default for MarketPrecision {
    fn default() -> Self {
        Self {
            price: None,
            amount: None,
            base: None,
            quote: None,
        }
    }
}

/// Market limits for order parameters
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MarketLimits {
    /// Amount limits
    pub amount: Option<MinMax>,
    /// Price limits
    pub price: Option<MinMax>,
    /// Cost (amount * price) limits
    pub cost: Option<MinMax>,
    /// Leverage limits (for margin/futures)
    pub leverage: Option<MinMax>,
}

impl Default for MarketLimits {
    fn default() -> Self {
        Self {
            amount: None,
            price: None,
            cost: None,
            leverage: None,
        }
    }
}

/// Market structure representing a trading pair
///
/// This corresponds to Go's `MarketInterface` struct and contains all
/// metadata about a trading pair including symbols, currencies, precision,
/// limits, and exchange-specific information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Market {
    /// Exchange-specific market ID
    pub id: String,

    /// Unified symbol (e.g., "BTC/USDT")
    pub symbol: Symbol,

    /// Parsed symbol structure with all components
    /// This provides structured access to base, quote, settle, and expiry
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parsed_symbol: Option<ParsedSymbol>,

    /// Base currency code (e.g., "BTC")
    pub base: String,

    /// Quote currency code (e.g., "USDT")
    pub quote: String,

    /// Settle currency code (for futures/swaps)
    pub settle: Option<String>,

    /// Base currency ID on exchange
    pub base_id: Option<String>,

    /// Quote currency ID on exchange
    pub quote_id: Option<String>,

    /// Settle currency ID on exchange
    pub settle_id: Option<String>,

    /// Market type (spot, futures, swap, option)
    #[serde(rename = "type")]
    pub market_type: MarketType,

    /// Is market active for trading
    pub active: bool,

    /// Is margin trading allowed
    pub margin: bool,

    /// Contract type (inverse, linear, etc.)
    pub contract: Option<bool>,

    /// Is linear contract
    pub linear: Option<bool>,

    /// Is inverse contract
    pub inverse: Option<bool>,

    /// Contract size (for futures/swaps)
    pub contract_size: Option<Decimal>,

    /// Expiry timestamp (for futures/options)
    pub expiry: Option<i64>,

    /// Expiry datetime string
    pub expiry_datetime: Option<String>,

    /// Strike price (for options)
    pub strike: Option<Decimal>,

    /// Option type (call/put)
    pub option_type: Option<String>,

    /// Precision settings
    pub precision: MarketPrecision,

    /// Limits for orders
    pub limits: MarketLimits,

    /// Maker fee rate
    pub maker: Option<Decimal>,

    /// Taker fee rate
    pub taker: Option<Decimal>,

    /// Percentage (true) or fixed (false) fees
    pub percentage: Option<bool>,

    /// Tier-based fees
    pub tier_based: Option<bool>,

    /// Fee side (get/give/base/quote)
    pub fee_side: Option<String>,

    /// Raw exchange info
    #[serde(flatten)]
    pub info: HashMap<String, serde_json::Value>,
}

impl Default for Market {
    fn default() -> Self {
        Self {
            id: String::new(),
            symbol: String::new(),
            parsed_symbol: None,
            base: String::new(),
            quote: String::new(),
            settle: None,
            base_id: None,
            quote_id: None,
            settle_id: None,
            market_type: MarketType::Spot,
            active: false,
            margin: false,
            contract: None,
            linear: None,
            inverse: None,
            contract_size: None,
            expiry: None,
            expiry_datetime: None,
            strike: None,
            option_type: None,
            precision: MarketPrecision::default(),
            limits: MarketLimits::default(),
            maker: None,
            taker: None,
            percentage: None,
            tier_based: None,
            fee_side: None,
            info: HashMap::new(),
        }
    }
}

impl Market {
    /// Create a new spot market
    ///
    /// The symbol is automatically parsed to populate the `parsed_symbol` field.
    /// If parsing fails, `parsed_symbol` will be `None`.
    pub fn new_spot(id: String, symbol: Symbol, base: String, quote: String) -> Self {
        // Try to parse the symbol, but don't fail if it doesn't parse
        let parsed_symbol = SymbolParser::parse(&symbol).ok();

        Self {
            id,
            symbol,
            parsed_symbol,
            base,
            quote,
            settle: None,
            base_id: None,
            quote_id: None,
            settle_id: None,
            market_type: MarketType::Spot,
            active: true,
            margin: false,
            contract: Some(false),
            linear: None,
            inverse: None,
            contract_size: None,
            expiry: None,
            expiry_datetime: None,
            strike: None,
            option_type: None,
            precision: MarketPrecision::default(),
            limits: MarketLimits::default(),
            maker: None,
            taker: None,
            percentage: Some(true),
            tier_based: Some(false),
            fee_side: None,
            info: HashMap::new(),
        }
    }

    /// Create a new futures market
    ///
    /// The symbol is automatically parsed to populate the `parsed_symbol` field.
    /// If parsing fails, `parsed_symbol` will be `None`.
    pub fn new_futures(
        id: String,
        symbol: Symbol,
        base: String,
        quote: String,
        settle: String,
        contract_size: Decimal,
    ) -> Self {
        // Try to parse the symbol, but don't fail if it doesn't parse
        let parsed_symbol = SymbolParser::parse(&symbol).ok();

        // Determine linear/inverse from parsed symbol or settle currency
        let (linear, inverse) = if let Some(ref ps) = parsed_symbol {
            (Some(ps.is_linear()), Some(ps.is_inverse()))
        } else {
            // Fallback: linear if settle == quote, inverse if settle == base
            let is_linear = settle == quote;
            let is_inverse = settle == base;
            (Some(is_linear), Some(is_inverse))
        };

        Self {
            id,
            symbol,
            parsed_symbol,
            base,
            quote,
            settle: Some(settle),
            base_id: None,
            quote_id: None,
            settle_id: None,
            market_type: MarketType::Futures,
            active: true,
            margin: false,
            contract: Some(true),
            linear,
            inverse,
            contract_size: Some(contract_size),
            expiry: None,
            expiry_datetime: None,
            strike: None,
            option_type: None,
            precision: MarketPrecision::default(),
            limits: MarketLimits::default(),
            maker: None,
            taker: None,
            percentage: Some(true),
            tier_based: Some(false),
            fee_side: None,
            info: HashMap::new(),
        }
    }

    /// Create a new swap (perpetual) market
    ///
    /// The symbol is automatically parsed to populate the `parsed_symbol` field.
    /// If parsing fails, `parsed_symbol` will be `None`.
    pub fn new_swap(
        id: String,
        symbol: Symbol,
        base: String,
        quote: String,
        settle: String,
        contract_size: Decimal,
    ) -> Self {
        // Try to parse the symbol, but don't fail if it doesn't parse
        let parsed_symbol = SymbolParser::parse(&symbol).ok();

        // Determine linear/inverse from parsed symbol or settle currency
        let (linear, inverse) = if let Some(ref ps) = parsed_symbol {
            (Some(ps.is_linear()), Some(ps.is_inverse()))
        } else {
            // Fallback: linear if settle == quote, inverse if settle == base
            let is_linear = settle == quote;
            let is_inverse = settle == base;
            (Some(is_linear), Some(is_inverse))
        };

        Self {
            id,
            symbol,
            parsed_symbol,
            base,
            quote,
            settle: Some(settle),
            base_id: None,
            quote_id: None,
            settle_id: None,
            market_type: MarketType::Swap,
            active: true,
            margin: false,
            contract: Some(true),
            linear,
            inverse,
            contract_size: Some(contract_size),
            expiry: None,
            expiry_datetime: None,
            strike: None,
            option_type: None,
            precision: MarketPrecision::default(),
            limits: MarketLimits::default(),
            maker: None,
            taker: None,
            percentage: Some(true),
            tier_based: Some(false),
            fee_side: None,
            info: HashMap::new(),
        }
    }

    /// Set the parsed symbol from the symbol string
    ///
    /// This method attempts to parse the current symbol string and update
    /// the `parsed_symbol` field. Returns `true` if parsing succeeded.
    pub fn parse_symbol(&mut self) -> bool {
        match SymbolParser::parse(&self.symbol) {
            Ok(parsed) => {
                self.parsed_symbol = Some(parsed);
                true
            }
            Err(_) => false,
        }
    }

    /// Set the parsed symbol directly
    pub fn set_parsed_symbol(&mut self, parsed: ParsedSymbol) {
        self.parsed_symbol = Some(parsed);
    }

    /// Get the parsed symbol if available
    pub fn get_parsed_symbol(&self) -> Option<&ParsedSymbol> {
        self.parsed_symbol.as_ref()
    }

    /// Check if market is a spot market
    pub fn is_spot(&self) -> bool {
        self.market_type == MarketType::Spot
    }

    /// Check if market is a futures market
    pub fn is_futures(&self) -> bool {
        self.market_type == MarketType::Futures
    }

    /// Check if market is a swap/perpetual market
    pub fn is_swap(&self) -> bool {
        self.market_type == MarketType::Swap
    }

    /// Check if market is an options market
    pub fn is_option(&self) -> bool {
        self.market_type == MarketType::Option
    }

    /// Check if market is a contract (futures/swap/option)
    pub fn is_contract(&self) -> bool {
        self.contract.unwrap_or(false)
    }

    /// Check if market is a linear contract (settled in quote currency)
    /// Linear contracts: USDT-margined futures/swaps (e.g., BTC/USDT:USDT)
    pub fn is_linear(&self) -> bool {
        self.linear.unwrap_or(false)
    }

    /// Check if market is an inverse contract (settled in base currency)
    /// Inverse contracts: Coin-margined futures/swaps (e.g., BTC/USD:BTC)
    pub fn is_inverse(&self) -> bool {
        self.inverse.unwrap_or(false)
    }

    /// Get the settlement currency
    pub fn settlement_currency(&self) -> &str {
        if let Some(ref settle) = self.settle {
            settle
        } else if self.is_contract() {
            &self.quote
        } else {
            &self.quote
        }
    }
}
/// Latest price information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LastPrice {
    /// Trading symbol.
    pub symbol: String,
    /// Latest price.
    pub price: Decimal,
    /// Timestamp in milliseconds.
    pub timestamp: i64,
    /// Datetime string.
    pub datetime: String,
}

impl LastPrice {
    /// Creates a new last price instance.
    pub fn new(symbol: String, price: Decimal, timestamp: i64) -> Self {
        let datetime = chrono::DateTime::from_timestamp_millis(timestamp)
            .map(|dt| dt.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string())
            .unwrap_or_default();

        Self {
            symbol,
            price,
            timestamp,
            datetime,
        }
    }
}

/// Trading fee information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradingFee {
    /// Trading symbol.
    pub symbol: String,
    /// Maker fee rate (0.001 = 0.1%).
    pub maker: Decimal,
    /// Taker fee rate (0.001 = 0.1%).
    pub taker: Decimal,
    /// Timestamp in milliseconds.
    pub timestamp: Option<i64>,
    /// Datetime string.
    pub datetime: Option<String>,
}

impl TradingFee {
    /// Creates a new trading fee instance.
    pub fn new(symbol: String, maker: Decimal, taker: Decimal, timestamp: Option<i64>) -> Self {
        let datetime = timestamp.and_then(|ts| {
            chrono::DateTime::from_timestamp_millis(ts)
                .map(|dt| dt.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string())
        });

        Self {
            symbol,
            maker,
            taker,
            timestamp,
            datetime,
        }
    }

    /// Calculates the difference between maker and taker fees.
    pub fn fee_difference(&self) -> Decimal {
        self.taker - self.maker
    }

    /// Converts maker fee rate to percentage.
    pub fn maker_percentage(&self) -> Decimal {
        self.maker * Decimal::ONE_HUNDRED
    }

    /// Converts taker fee rate to percentage.
    pub fn taker_percentage(&self) -> Decimal {
        self.taker * Decimal::ONE_HUNDRED
    }
}

/// Server time information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerTime {
    /// Server timestamp in milliseconds.
    pub server_time: i64,
    /// Datetime string.
    pub datetime: String,
}

impl ServerTime {
    /// Creates a new server time instance.
    pub fn new(server_time: i64) -> Self {
        let datetime = chrono::DateTime::from_timestamp_millis(server_time)
            .map(|dt| dt.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string())
            .unwrap_or_default();

        Self {
            server_time,
            datetime,
        }
    }

    /// Calculates the offset from local time in milliseconds.
    pub fn offset_from_local(&self) -> i64 {
        let local_time = chrono::Utc::now().timestamp_millis();
        self.server_time - local_time
    }
}

/// Best bid/ask price information (book ticker).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BidAsk {
    /// Trading symbol.
    pub symbol: String,
    /// Best bid price.
    pub bid_price: Decimal,
    /// Best bid quantity.
    pub bid_qty: Decimal,
    /// Best ask price.
    pub ask_price: Decimal,
    /// Best ask quantity.
    pub ask_qty: Decimal,
    /// Timestamp in milliseconds.
    pub timestamp: Option<i64>,
    /// Datetime string.
    pub datetime: Option<String>,
    /// Raw exchange data.
    #[serde(flatten)]
    pub info: HashMap<String, serde_json::Value>,
}

impl BidAsk {
    /// Creates a new bid/ask instance.
    pub fn new(
        symbol: String,
        bid_price: Decimal,
        bid_qty: Decimal,
        ask_price: Decimal,
        ask_qty: Decimal,
    ) -> Self {
        Self {
            symbol,
            bid_price,
            bid_qty,
            ask_price,
            ask_qty,
            timestamp: None,
            datetime: None,
            info: HashMap::new(),
        }
    }

    /// Calculates the bid-ask spread.
    pub fn spread(&self) -> Decimal {
        self.ask_price - self.bid_price
    }

    /// Calculates the spread as a percentage.
    pub fn spread_percentage(&self) -> Decimal {
        if self.bid_price > Decimal::ZERO {
            (self.ask_price - self.bid_price) / self.bid_price * Decimal::from(100)
        } else {
            Decimal::ZERO
        }
    }

    /// Calculates the mid price.
    pub fn mid_price(&self) -> Decimal {
        (self.bid_price + self.ask_price) / Decimal::from(2)
    }
}

/// 24-hour statistics data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Stats24hr {
    /// Trading symbol.
    pub symbol: String,
    /// Price change.
    #[serde(rename = "priceChange")]
    pub price_change: Option<Decimal>,
    /// Price change percentage.
    #[serde(rename = "priceChangePercent")]
    pub price_change_percent: Option<Decimal>,
    /// Weighted average price.
    #[serde(rename = "weightedAvgPrice")]
    pub weighted_avg_price: Option<Decimal>,
    /// Previous close price.
    #[serde(rename = "prevClosePrice")]
    pub prev_close_price: Option<Decimal>,
    /// Latest price.
    #[serde(rename = "lastPrice")]
    pub last_price: Option<Decimal>,
    /// Latest trade quantity.
    #[serde(rename = "lastQty")]
    pub last_qty: Option<Decimal>,
    /// Best bid price.
    #[serde(rename = "bidPrice")]
    pub bid_price: Option<Decimal>,
    /// Best bid quantity.
    #[serde(rename = "bidQty")]
    pub bid_qty: Option<Decimal>,
    /// Best ask price.
    #[serde(rename = "askPrice")]
    pub ask_price: Option<Decimal>,
    /// Best ask quantity.
    #[serde(rename = "askQty")]
    pub ask_qty: Option<Decimal>,
    /// Open price.
    #[serde(rename = "openPrice")]
    pub open_price: Option<Decimal>,
    /// High price.
    #[serde(rename = "highPrice")]
    pub high_price: Option<Decimal>,
    /// Low price.
    #[serde(rename = "lowPrice")]
    pub low_price: Option<Decimal>,
    /// Volume (base currency).
    pub volume: Option<Decimal>,
    /// Quote volume (quote currency).
    #[serde(rename = "quoteVolume")]
    pub quote_volume: Option<Decimal>,
    /// Open time.
    #[serde(rename = "openTime")]
    pub open_time: Option<i64>,
    /// Close time.
    #[serde(rename = "closeTime")]
    pub close_time: Option<i64>,
    /// First trade ID.
    #[serde(rename = "firstId")]
    pub first_id: Option<i64>,
    /// Last trade ID.
    #[serde(rename = "lastId")]
    pub last_id: Option<i64>,
    /// Number of trades.
    pub count: Option<i64>,
    /// Raw exchange data.
    #[serde(flatten)]
    pub info: HashMap<String, serde_json::Value>,
}

impl Stats24hr {
    /// Creates a new 24-hour statistics instance.
    pub fn new(symbol: String) -> Self {
        Self {
            symbol,
            price_change: None,
            price_change_percent: None,
            weighted_avg_price: None,
            prev_close_price: None,
            last_price: None,
            last_qty: None,
            bid_price: None,
            bid_qty: None,
            ask_price: None,
            ask_qty: None,
            open_price: None,
            high_price: None,
            low_price: None,
            volume: None,
            quote_volume: None,
            open_time: None,
            close_time: None,
            first_id: None,
            last_id: None,
            count: None,
            info: HashMap::new(),
        }
    }

    /// Calculates the price change percentage if not provided.
    pub fn calculate_price_change_percent(&self) -> Option<Decimal> {
        if let Some(pct) = self.price_change_percent {
            return Some(pct);
        }

        if let (Some(last), Some(open)) = (self.last_price, self.open_price) {
            if open > Decimal::ZERO {
                return Some((last - open) / open * Decimal::from(100));
            }
        }

        None
    }

    /// Calculates the price change if not provided.
    pub fn calculate_price_change(&self) -> Option<Decimal> {
        if let Some(change) = self.price_change {
            return Some(change);
        }

        if let (Some(last), Some(open)) = (self.last_price, self.open_price) {
            return Some(last - open);
        }

        None
    }

    /// Checks if the price change is positive.
    pub fn is_positive(&self) -> bool {
        self.price_change.map_or(false, |c| c > Decimal::ZERO)
    }

    /// Checks if the price change is negative.
    pub fn is_negative(&self) -> bool {
        self.price_change.map_or(false, |c| c < Decimal::ZERO)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_market_creation_spot() {
        let market = Market::new_spot(
            "BTCUSDT".to_string(),
            "BTC/USDT".to_string(),
            "BTC".to_string(),
            "USDT".to_string(),
        );

        assert_eq!(market.symbol, "BTC/USDT");
        assert!(market.is_spot());
        assert!(!market.is_futures());
        assert!(!market.is_contract());
    }

    #[test]
    fn test_market_creation_futures() {
        let market = Market::new_futures(
            "BTCUSDT_PERP".to_string(),
            "BTC/USDT:USDT".to_string(),
            "BTC".to_string(),
            "USDT".to_string(),
            "USDT".to_string(),
            dec!(1.0),
        );

        assert_eq!(market.symbol, "BTC/USDT:USDT");
        assert!(!market.is_spot());
        assert!(market.is_futures());
        assert!(market.is_contract());
        assert_eq!(market.settlement_currency(), "USDT");
    }

    #[test]
    fn test_market_type_display() {
        assert_eq!(MarketType::Spot.to_string(), "spot");
        assert_eq!(MarketType::Futures.to_string(), "futures");
        assert_eq!(MarketType::Swap.to_string(), "swap");
    }
}
