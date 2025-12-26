//! Core type definitions for CCXT
//!
//! This module contains all the fundamental data structures used throughout the library,
//! including markets, orders, trades, balances, tickers, and order books.

use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;
use serde::{Deserialize, Serialize};

// Re-export submodules
pub mod account;
pub mod balance;
pub mod bid_ask;
pub mod currency;
/// Default type configuration for exchange operations
pub mod default_type;
pub mod fee;
/// Financial types with enhanced type safety (Price, Amount, Cost)
pub mod financial;
pub mod funding_rate;
/// Ledger entry types and related structures
pub mod ledger;
pub mod margin;
pub mod mark_price;
pub mod market;
pub mod market_data;
pub mod ohlcv;
pub mod order;
pub mod orderbook;
/// Parameter types for trait methods with builder pattern support
pub mod params;
pub mod position;
/// Risk management and position risk types
pub mod risk;
/// Symbol types for unified symbol format
pub mod symbol;
pub mod ticker;
pub mod ticker_params;
pub mod trade;
/// Transaction types for deposits and withdrawals
pub mod transaction;
pub mod transfer;

// Re-export commonly used types
pub use account::{AccountConfig, CommissionRate};
pub use balance::{Balance, BalanceEntry, MaxBorrowable, MaxTransferable};
pub use bid_ask::BidAsk;
pub use currency::{Currency, CurrencyNetwork, MinMax, PrecisionMode};
pub use default_type::{DefaultSubType, DefaultType, DefaultTypeError, resolve_market_type};
pub use fee::{
    FundingRate as FeeFundingRate, FundingRateHistory as FeeFundingRateHistory, LeverageTier,
    TradingFee as FeeTradingFee,
};
pub use funding_rate::{
    FundingFee, FundingHistory, FundingRate, FundingRateHistory, NextFundingRate,
};
pub use ledger::{LedgerDirection, LedgerEntry, LedgerEntryType};
pub use margin::{
    BorrowInterest, BorrowRate, BorrowRateHistory, IsolatedBorrowRate, MarginAdjustment,
    MarginLoan, MarginRepay,
};
pub use mark_price::MarkPrice;
pub use market::{
    BidAsk as MarketBidAsk, LastPrice, Market, MarketLimits, MarketPrecision, MarketType,
    ServerTime, Stats24hr, TradingFee,
};
pub use market_data::{IndexPrice, Liquidation, PremiumIndex};
pub use ohlcv::OHLCV;
pub use order::{
    BatchCancelResult, BatchOrderRequest, BatchOrderResult, BatchOrderUpdate,
    CancelAllOrdersResult, CancelReplaceResponse, OcoOrder, OcoOrderInfo, Order, OrderReport,
    OrderSide, OrderStatus, OrderType, TimeInForce,
};
pub use orderbook::{OrderBook, OrderBookDelta, OrderBookEntry, OrderBookSide};
pub use params::{
    AccountType, BalanceParams, LeverageParams, MarginMode, OhlcvParams, OrderBookParams,
    OrderParams, PriceType, TransferParams, WithdrawParams,
};
pub use position::{Leverage, MarginType, Position, PositionSide};
pub use risk::{MaxLeverage, OpenInterest, OpenInterestHistory};
pub use symbol::{ContractType, ExpiryDate, ParsedSymbol, SymbolMarketType};
pub use ticker::Ticker;
pub use ticker_params::{IntoTickerParams, TickerParams, TickerParamsBuilder};
pub use trade::{AggTrade, TakerOrMaker, Trade};
pub use transaction::{
    DepositAddress, Transaction, TransactionFee, TransactionStatus, TransactionType,
};
pub use transfer::{DepositWithdrawFee, NetworkInfo, Transfer, TransferType};

/// Type alias for timestamps (milliseconds since Unix epoch)
pub type Timestamp = i64;

/// Type alias for trading symbols (e.g., "BTC/USDT")
pub type Symbol = String;

// Re-export financial types with enhanced type safety
pub use financial::{Amount, Cost, Price};

/// Fee information
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Fee {
    /// Fee currency code
    pub currency: String,

    /// Fee cost
    pub cost: Decimal,

    /// Fee rate (if available)
    pub rate: Option<Decimal>,
}

impl Fee {
    /// Create a new fee
    pub fn new(currency: String, cost: Decimal) -> Self {
        Self {
            currency,
            cost,
            rate: None,
        }
    }

    /// Create a new fee with rate
    pub fn with_rate(currency: String, cost: Decimal, rate: Decimal) -> Self {
        Self {
            currency,
            cost,
            rate: Some(rate),
        }
    }
}

/// OHLCV (Open, High, Low, Close, Volume) candlestick data
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Ohlcv {
    /// Timestamp in milliseconds
    pub timestamp: Timestamp,

    /// Opening price
    pub open: Price,

    /// Highest price
    pub high: Price,

    /// Lowest price
    pub low: Price,

    /// Closing price
    pub close: Price,

    /// Volume traded
    pub volume: Amount,
}

impl Ohlcv {
    /// Create a new OHLCV entry
    pub fn new(
        timestamp: Timestamp,
        open: Price,
        high: Price,
        low: Price,
        close: Price,
        volume: Amount,
    ) -> Self {
        Self {
            timestamp,
            open,
            high,
            low,
            close,
            volume,
        }
    }

    /// Convert to array format [timestamp, open, high, low, close, volume]
    pub fn to_array(&self) -> [Decimal; 6] {
        [
            Decimal::from(self.timestamp),
            self.open.as_decimal().clone(),
            self.high.as_decimal().clone(),
            self.low.as_decimal().clone(),
            self.close.as_decimal().clone(),
            self.volume.as_decimal().clone(),
        ]
    }

    /// Create from array format [timestamp, open, high, low, close, volume]
    pub fn from_array(arr: [Decimal; 6]) -> Self {
        Self {
            timestamp: arr[0].to_i64().unwrap_or(0),
            open: Price::new(arr[1]),
            high: Price::new(arr[2]),
            low: Price::new(arr[3]),
            close: Price::new(arr[4]),
            volume: Amount::new(arr[5]),
        }
    }
}

/// Timeframe enum for OHLCV data
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, Default)]
pub enum Timeframe {
    /// 1 minute
    #[serde(rename = "1m")]
    M1,
    /// 3 minutes
    #[serde(rename = "3m")]
    M3,
    /// 5 minutes
    #[serde(rename = "5m")]
    M5,
    /// 15 minutes
    #[serde(rename = "15m")]
    M15,
    /// 30 minutes
    #[serde(rename = "30m")]
    M30,
    /// 1 hour
    #[serde(rename = "1h")]
    #[default]
    H1,
    /// 2 hours
    #[serde(rename = "2h")]
    H2,
    /// 4 hours
    #[serde(rename = "4h")]
    H4,
    /// 6 hours
    #[serde(rename = "6h")]
    H6,
    /// 8 hours
    #[serde(rename = "8h")]
    H8,
    /// 12 hours
    #[serde(rename = "12h")]
    H12,
    /// 1 day
    #[serde(rename = "1d")]
    D1,
    /// 3 days
    #[serde(rename = "3d")]
    D3,
    /// 1 week
    #[serde(rename = "1w")]
    W1,
    /// 1 month
    #[serde(rename = "1M")]
    Mon1,
}

impl Timeframe {
    /// Convert timeframe to milliseconds
    pub fn as_millis(&self) -> i64 {
        match self {
            Self::M1 => 60_000,
            Self::M3 => 180_000,
            Self::M5 => 300_000,
            Self::M15 => 900_000,
            Self::M30 => 1_800_000,
            Self::H1 => 3_600_000,
            Self::H2 => 7_200_000,
            Self::H4 => 14_400_000,
            Self::H6 => 21_600_000,
            Self::H8 => 28_800_000,
            Self::H12 => 43_200_000,
            Self::D1 => 86_400_000,
            Self::D3 => 259_200_000,
            Self::W1 => 604_800_000,
            Self::Mon1 => 2_592_000_000, // 30 days
        }
    }

    /// Convert timeframe to seconds
    pub fn as_seconds(&self) -> i64 {
        self.as_millis() / 1000
    }
}

impl std::fmt::Display for Timeframe {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::M1 => "1m",
            Self::M3 => "3m",
            Self::M5 => "5m",
            Self::M15 => "15m",
            Self::M30 => "30m",
            Self::H1 => "1h",
            Self::H2 => "2h",
            Self::H4 => "4h",
            Self::H6 => "6h",
            Self::H8 => "8h",
            Self::H12 => "12h",
            Self::D1 => "1d",
            Self::D3 => "3d",
            Self::W1 => "1w",
            Self::Mon1 => "1M",
        };
        write!(f, "{}", s)
    }
}

/// Trading limits for a market
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TradingLimits {
    /// Minimum amount (deprecated, use amount.min)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min: Option<Amount>,

    /// Maximum amount (deprecated, use amount.max)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max: Option<Amount>,

    /// Amount/quantity limits
    #[serde(skip_serializing_if = "Option::is_none")]
    pub amount: Option<MinMax>,

    /// Price limits
    #[serde(skip_serializing_if = "Option::is_none")]
    pub price: Option<MinMax>,

    /// Cost (amount * price) limits
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost: Option<MinMax>,
}

impl TradingLimits {
    /// Create new trading limits (legacy)
    pub fn new(min: Option<Amount>, max: Option<Amount>) -> Self {
        Self {
            min,
            max,
            amount: None,
            price: None,
            cost: None,
        }
    }

    /// Create new trading limits with detailed constraints
    pub fn new_detailed(
        amount: Option<MinMax>,
        price: Option<MinMax>,
        cost: Option<MinMax>,
    ) -> Self {
        Self {
            min: None,
            max: None,
            amount,
            price,
            cost,
        }
    }

    /// Check if amount is within limits
    pub fn is_valid(&self, amount: Amount) -> bool {
        let min_ok = self.min.map_or(true, |min| amount >= min);
        let max_ok = self.max.map_or(true, |max| amount <= max);
        min_ok && max_ok
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_fee_creation() {
        let fee = Fee::new("USDT".to_string(), dec!(10.5));
        assert_eq!(fee.currency, "USDT");
        assert_eq!(fee.cost, dec!(10.5));
        assert_eq!(fee.rate, None);

        let fee_with_rate = Fee::with_rate("USDT".to_string(), dec!(10.5), dec!(0.001));
        assert_eq!(fee_with_rate.rate, Some(dec!(0.001)));
    }

    #[test]
    fn test_ohlcv_conversion() {
        let ohlcv = Ohlcv::new(
            1234567890,
            Price::from(dec!(100)),
            Price::from(dec!(110)),
            Price::from(dec!(95)),
            Price::from(dec!(105)),
            Amount::from(dec!(1000)),
        );

        let arr = ohlcv.to_array();
        assert_eq!(arr[0], Decimal::from(1234567890));
        assert_eq!(arr[1], dec!(100));
        assert_eq!(arr[5], dec!(1000));
    }

    #[test]
    fn test_timeframe_conversion() {
        assert_eq!(Timeframe::M1.as_millis(), 60_000);
        assert_eq!(Timeframe::H1.as_millis(), 3_600_000);
        assert_eq!(Timeframe::D1.as_millis(), 86_400_000);

        assert_eq!(Timeframe::M1.as_seconds(), 60);
        assert_eq!(Timeframe::H1.as_seconds(), 3_600);
    }

    #[test]
    fn test_trading_limits() {
        let limits = TradingLimits::new(
            Some(Amount::from(dec!(0.01))),
            Some(Amount::from(dec!(100))),
        );

        assert!(limits.is_valid(Amount::from(dec!(1.0))));
        assert!(limits.is_valid(Amount::from(dec!(0.01))));
        assert!(limits.is_valid(Amount::from(dec!(100))));
        assert!(!limits.is_valid(Amount::from(dec!(0.001))));
        assert!(!limits.is_valid(Amount::from(dec!(101))));
    }
}
