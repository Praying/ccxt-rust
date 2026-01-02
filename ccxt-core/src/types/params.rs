//! Parameter types for trait methods with builder pattern support.
//!
//! This module provides ergonomic parameter structs for the decomposed Exchange traits.
//! Each parameter type uses the builder pattern for flexible and readable API calls.
//!
//! # Example
//!
//! ```rust
//! use ccxt_core::types::params::{OhlcvParams, OrderParams, AccountType};
//! use ccxt_core::types::{Timeframe, OrderSide};
//! use rust_decimal_macros::dec;
//!
//! // OHLCV parameters with builder pattern
//! let params = OhlcvParams::new(Timeframe::H1)
//!     .since(1609459200000)
//!     .limit(100);
//!
//! // Order parameters with convenience constructors
//! let order = OrderParams::market_buy("BTC/USDT", dec!(0.01));
//! let limit_order = OrderParams::limit_sell("BTC/USDT", dec!(0.01), dec!(50000))
//!     .time_in_force(ccxt_core::types::TimeInForce::GTC)
//!     .client_id("my-order-123");
//! ```

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use super::order::{OrderSide, OrderType, TimeInForce};
use super::position::MarginType;

/// Account type for balance queries and transfers.
///
/// Represents different account types available on exchanges.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum AccountType {
    /// Spot trading account.
    #[default]
    Spot,
    /// Cross margin account.
    Margin,
    /// Isolated margin account.
    IsolatedMargin,
    /// USDT-margined futures account.
    Futures,
    /// Coin-margined futures account.
    Delivery,
    /// Funding/wallet account.
    Funding,
    /// Options trading account.
    Option,
}

impl std::fmt::Display for AccountType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            AccountType::Spot => "spot",
            AccountType::Margin => "margin",
            AccountType::IsolatedMargin => "isolated_margin",
            AccountType::Futures => "futures",
            AccountType::Delivery => "delivery",
            AccountType::Funding => "funding",
            AccountType::Option => "option",
        };
        write!(f, "{s}")
    }
}

impl std::str::FromStr for AccountType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "spot" | "main" => Ok(AccountType::Spot),
            "margin" | "cross" => Ok(AccountType::Margin),
            "isolated_margin" | "isolated" => Ok(AccountType::IsolatedMargin),
            "futures" | "future" | "swap" | "linear" | "umfuture" => Ok(AccountType::Futures),
            "delivery" | "inverse" | "cmfuture" => Ok(AccountType::Delivery),
            "funding" | "wallet" => Ok(AccountType::Funding),
            "option" | "options" => Ok(AccountType::Option),
            _ => Err(format!("Invalid account type: {s}")),
        }
    }
}

/// Margin mode for futures trading.
///
/// Type alias for `MarginType` to provide a more intuitive name in the context
/// of the new trait hierarchy.
pub type MarginMode = MarginType;

/// Price type for OHLCV data (futures-specific).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum PriceType {
    /// Contract/last price (default).
    #[default]
    Contract,
    /// Mark price.
    Mark,
    /// Index price.
    Index,
    /// Premium index.
    PremiumIndex,
}

impl std::fmt::Display for PriceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            PriceType::Contract => "contract",
            PriceType::Mark => "mark",
            PriceType::Index => "index",
            PriceType::PremiumIndex => "premiumIndex",
        };
        write!(f, "{s}")
    }
}

// ============================================================================
// OHLCV Parameters
// ============================================================================

/// Parameters for fetching OHLCV (candlestick) data.
///
/// Uses the builder pattern for ergonomic parameter construction.
///
/// # Example
///
/// ```rust
/// use ccxt_core::types::params::{OhlcvParams, PriceType};
/// use ccxt_core::types::Timeframe;
///
/// let params = OhlcvParams::new(Timeframe::H1)
///     .since(1609459200000)
///     .limit(100)
///     .until(1609545600000)
///     .price(PriceType::Mark);
/// ```
#[derive(Debug, Clone, Default)]
pub struct OhlcvParams {
    /// Timeframe for the candlesticks.
    pub timeframe: super::Timeframe,
    /// Start timestamp in milliseconds.
    pub since: Option<i64>,
    /// Maximum number of candles to return.
    pub limit: Option<u32>,
    /// End timestamp in milliseconds.
    pub until: Option<i64>,
    /// Price type (mark, index, premiumIndex for futures).
    pub price: Option<PriceType>,
}

impl OhlcvParams {
    /// Create new OHLCV parameters with the specified timeframe.
    pub fn new(timeframe: super::Timeframe) -> Self {
        Self {
            timeframe,
            ..Default::default()
        }
    }

    /// Set the start timestamp.
    pub fn since(mut self, ts: i64) -> Self {
        self.since = Some(ts);
        self
    }

    /// Set the maximum number of candles.
    pub fn limit(mut self, n: u32) -> Self {
        self.limit = Some(n);
        self
    }

    /// Set the end timestamp.
    pub fn until(mut self, ts: i64) -> Self {
        self.until = Some(ts);
        self
    }

    /// Set the price type (for futures).
    pub fn price(mut self, p: PriceType) -> Self {
        self.price = Some(p);
        self
    }
}

// ============================================================================
// OrderBook Parameters
// ============================================================================

/// Parameters for fetching order book data.
///
/// # Example
///
/// ```rust
/// use ccxt_core::types::params::OrderBookParams;
///
/// let params = OrderBookParams::default().limit(100);
/// ```
#[derive(Debug, Clone, Default)]
pub struct OrderBookParams {
    /// Maximum depth (number of price levels) to return.
    pub limit: Option<u32>,
}

impl OrderBookParams {
    /// Set the maximum depth.
    pub fn limit(mut self, n: u32) -> Self {
        self.limit = Some(n);
        self
    }
}

// ============================================================================
// Order Parameters
// ============================================================================

/// Parameters for creating orders.
///
/// Provides convenience constructors for common order types and a builder
/// pattern for advanced customization.
///
/// # Example
///
/// ```rust
/// use ccxt_core::types::params::OrderParams;
/// use ccxt_core::types::TimeInForce;
/// use rust_decimal_macros::dec;
///
/// // Market buy order
/// let order = OrderParams::market_buy("BTC/USDT", dec!(0.01));
///
/// // Limit sell order with options
/// let order = OrderParams::limit_sell("BTC/USDT", dec!(0.01), dec!(50000))
///     .time_in_force(TimeInForce::IOC)
///     .client_id("my-order-123")
///     .reduce_only();
/// ```
#[derive(Debug, Clone)]
pub struct OrderParams {
    /// Trading symbol (e.g., "BTC/USDT").
    pub symbol: String,
    /// Order side (buy/sell).
    pub side: OrderSide,
    /// Order type (market/limit/etc.).
    pub order_type: OrderType,
    /// Order amount/quantity.
    pub amount: Decimal,
    /// Limit price (required for limit orders).
    pub price: Option<Decimal>,
    /// Stop/trigger price (for stop orders).
    pub stop_price: Option<Decimal>,
    /// Time in force policy.
    pub time_in_force: Option<TimeInForce>,
    /// Client-assigned order ID.
    pub client_order_id: Option<String>,
    /// Reduce-only flag (futures).
    pub reduce_only: Option<bool>,
    /// Post-only flag (maker only).
    pub post_only: Option<bool>,
}

impl OrderParams {
    /// Create a market buy order.
    pub fn market_buy(symbol: &str, amount: Decimal) -> Self {
        Self {
            symbol: symbol.to_string(),
            side: OrderSide::Buy,
            order_type: OrderType::Market,
            amount,
            price: None,
            stop_price: None,
            time_in_force: None,
            client_order_id: None,
            reduce_only: None,
            post_only: None,
        }
    }

    /// Create a market sell order.
    pub fn market_sell(symbol: &str, amount: Decimal) -> Self {
        Self::market_buy(symbol, amount).side(OrderSide::Sell)
    }

    /// Create a limit buy order.
    pub fn limit_buy(symbol: &str, amount: Decimal, price: Decimal) -> Self {
        Self {
            symbol: symbol.to_string(),
            side: OrderSide::Buy,
            order_type: OrderType::Limit,
            amount,
            price: Some(price),
            stop_price: None,
            time_in_force: Some(TimeInForce::GTC),
            client_order_id: None,
            reduce_only: None,
            post_only: None,
        }
    }

    /// Create a limit sell order.
    pub fn limit_sell(symbol: &str, amount: Decimal, price: Decimal) -> Self {
        Self::limit_buy(symbol, amount, price).side(OrderSide::Sell)
    }

    /// Set the order side.
    pub fn side(mut self, side: OrderSide) -> Self {
        self.side = side;
        self
    }

    /// Set the stop/trigger price.
    pub fn stop_price(mut self, price: Decimal) -> Self {
        self.stop_price = Some(price);
        self
    }

    /// Set the time in force policy.
    pub fn time_in_force(mut self, tif: TimeInForce) -> Self {
        self.time_in_force = Some(tif);
        self
    }

    /// Set the client order ID.
    pub fn client_id(mut self, id: &str) -> Self {
        self.client_order_id = Some(id.to_string());
        self
    }

    /// Mark the order as reduce-only.
    pub fn reduce_only(mut self) -> Self {
        self.reduce_only = Some(true);
        self
    }

    /// Mark the order as post-only (maker only).
    pub fn post_only(mut self) -> Self {
        self.post_only = Some(true);
        self
    }
}

// ============================================================================
// Balance Parameters
// ============================================================================

/// Parameters for fetching account balance.
///
/// # Example
///
/// ```rust
/// use ccxt_core::types::params::{BalanceParams, AccountType};
///
/// // Spot balance
/// let params = BalanceParams::spot();
///
/// // Futures balance for specific currencies
/// let params = BalanceParams::futures()
///     .currencies(&["BTC", "USDT"]);
/// ```
#[derive(Debug, Clone, Default)]
pub struct BalanceParams {
    /// Account type to query.
    pub account_type: Option<AccountType>,
    /// Filter by specific currencies.
    pub currencies: Option<Vec<String>>,
}

impl BalanceParams {
    /// Create parameters for spot account balance.
    pub fn spot() -> Self {
        Self {
            account_type: Some(AccountType::Spot),
            currencies: None,
        }
    }

    /// Create parameters for margin account balance.
    pub fn margin() -> Self {
        Self {
            account_type: Some(AccountType::Margin),
            currencies: None,
        }
    }

    /// Create parameters for futures account balance.
    pub fn futures() -> Self {
        Self {
            account_type: Some(AccountType::Futures),
            currencies: None,
        }
    }

    /// Filter by specific currencies.
    pub fn currencies(mut self, codes: &[&str]) -> Self {
        self.currencies = Some(codes.iter().map(std::string::ToString::to_string).collect());
        self
    }
}

// ============================================================================
// Leverage Parameters
// ============================================================================

/// Parameters for setting leverage.
///
/// # Example
///
/// ```rust
/// use ccxt_core::types::params::LeverageParams;
/// use ccxt_core::types::position::MarginType;
///
/// let params = LeverageParams::new("BTC/USDT:USDT", 10)
///     .margin_mode(MarginType::Isolated);
/// ```
#[derive(Debug, Clone)]
pub struct LeverageParams {
    /// Trading symbol.
    pub symbol: String,
    /// Leverage multiplier.
    pub leverage: u32,
    /// Margin mode (cross or isolated).
    pub margin_mode: Option<MarginMode>,
}

impl LeverageParams {
    /// Create new leverage parameters.
    pub fn new(symbol: &str, leverage: u32) -> Self {
        Self {
            symbol: symbol.to_string(),
            leverage,
            margin_mode: None,
        }
    }

    /// Set margin mode to cross.
    pub fn cross(mut self) -> Self {
        self.margin_mode = Some(MarginMode::Cross);
        self
    }

    /// Set margin mode to isolated.
    pub fn isolated(mut self) -> Self {
        self.margin_mode = Some(MarginMode::Isolated);
        self
    }

    /// Set the margin mode explicitly.
    pub fn margin_mode(mut self, mode: MarginMode) -> Self {
        self.margin_mode = Some(mode);
        self
    }
}

// ============================================================================
// Withdraw Parameters
// ============================================================================

/// Parameters for withdrawing funds.
///
/// # Example
///
/// ```rust
/// use ccxt_core::types::params::WithdrawParams;
/// use rust_decimal_macros::dec;
///
/// let params = WithdrawParams::new("USDT", dec!(100), "TAddress...")
///     .network("TRC20")
///     .tag("memo123");
/// ```
#[derive(Debug, Clone)]
pub struct WithdrawParams {
    /// Currency code to withdraw.
    pub currency: String,
    /// Amount to withdraw.
    pub amount: Decimal,
    /// Destination address.
    pub address: String,
    /// Address tag/memo (for certain chains).
    pub tag: Option<String>,
    /// Specific network (e.g., "ERC20", "TRC20").
    pub network: Option<String>,
    /// Custom withdrawal fee.
    pub fee: Option<Decimal>,
}

impl WithdrawParams {
    /// Create new withdrawal parameters.
    pub fn new(currency: &str, amount: Decimal, address: &str) -> Self {
        Self {
            currency: currency.to_string(),
            amount,
            address: address.to_string(),
            tag: None,
            network: None,
            fee: None,
        }
    }

    /// Set the address tag/memo.
    pub fn tag(mut self, tag: &str) -> Self {
        self.tag = Some(tag.to_string());
        self
    }

    /// Set the network.
    pub fn network(mut self, network: &str) -> Self {
        self.network = Some(network.to_string());
        self
    }

    /// Set a custom fee.
    pub fn fee(mut self, fee: Decimal) -> Self {
        self.fee = Some(fee);
        self
    }
}

// ============================================================================
// Transfer Parameters
// ============================================================================

/// Parameters for transferring funds between accounts.
///
/// # Example
///
/// ```rust
/// use ccxt_core::types::params::{TransferParams, AccountType};
/// use rust_decimal_macros::dec;
///
/// // Transfer from spot to futures
/// let params = TransferParams::spot_to_futures("USDT", dec!(1000));
///
/// // Custom transfer
/// let params = TransferParams::new("BTC", dec!(0.1), AccountType::Spot, AccountType::Margin);
/// ```
#[derive(Debug, Clone)]
pub struct TransferParams {
    /// Currency code to transfer.
    pub currency: String,
    /// Amount to transfer.
    pub amount: Decimal,
    /// Source account type.
    pub from_account: AccountType,
    /// Destination account type.
    pub to_account: AccountType,
}

impl TransferParams {
    /// Create new transfer parameters.
    pub fn new(currency: &str, amount: Decimal, from: AccountType, to: AccountType) -> Self {
        Self {
            currency: currency.to_string(),
            amount,
            from_account: from,
            to_account: to,
        }
    }

    /// Transfer from spot to futures account.
    pub fn spot_to_futures(currency: &str, amount: Decimal) -> Self {
        Self::new(currency, amount, AccountType::Spot, AccountType::Futures)
    }

    /// Transfer from futures to spot account.
    pub fn futures_to_spot(currency: &str, amount: Decimal) -> Self {
        Self::new(currency, amount, AccountType::Futures, AccountType::Spot)
    }

    /// Transfer from spot to margin account.
    pub fn spot_to_margin(currency: &str, amount: Decimal) -> Self {
        Self::new(currency, amount, AccountType::Spot, AccountType::Margin)
    }

    /// Transfer from margin to spot account.
    pub fn margin_to_spot(currency: &str, amount: Decimal) -> Self {
        Self::new(currency, amount, AccountType::Margin, AccountType::Spot)
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Timeframe;
    use rust_decimal_macros::dec;

    #[test]
    fn test_account_type_display() {
        assert_eq!(AccountType::Spot.to_string(), "spot");
        assert_eq!(AccountType::Futures.to_string(), "futures");
        assert_eq!(AccountType::Margin.to_string(), "margin");
    }

    #[test]
    fn test_account_type_from_str() {
        assert_eq!("spot".parse::<AccountType>().unwrap(), AccountType::Spot);
        assert_eq!(
            "futures".parse::<AccountType>().unwrap(),
            AccountType::Futures
        );
        assert_eq!(
            "margin".parse::<AccountType>().unwrap(),
            AccountType::Margin
        );
        assert_eq!("main".parse::<AccountType>().unwrap(), AccountType::Spot);
    }

    #[test]
    fn test_ohlcv_params_builder() {
        let params = OhlcvParams::new(Timeframe::H1)
            .since(1609459200000)
            .limit(100)
            .until(1609545600000)
            .price(PriceType::Mark);

        assert_eq!(params.timeframe, Timeframe::H1);
        assert_eq!(params.since, Some(1609459200000));
        assert_eq!(params.limit, Some(100));
        assert_eq!(params.until, Some(1609545600000));
        assert_eq!(params.price, Some(PriceType::Mark));
    }

    #[test]
    fn test_order_book_params() {
        let params = OrderBookParams::default().limit(50);
        assert_eq!(params.limit, Some(50));
    }

    #[test]
    fn test_order_params_market_buy() {
        let params = OrderParams::market_buy("BTC/USDT", dec!(0.01));
        assert_eq!(params.symbol, "BTC/USDT");
        assert_eq!(params.side, OrderSide::Buy);
        assert_eq!(params.order_type, OrderType::Market);
        assert_eq!(params.amount, dec!(0.01));
        assert!(params.price.is_none());
    }

    #[test]
    fn test_order_params_limit_sell() {
        let params = OrderParams::limit_sell("ETH/USDT", dec!(1.0), dec!(3000))
            .time_in_force(TimeInForce::IOC)
            .client_id("test-123")
            .post_only();

        assert_eq!(params.symbol, "ETH/USDT");
        assert_eq!(params.side, OrderSide::Sell);
        assert_eq!(params.order_type, OrderType::Limit);
        assert_eq!(params.amount, dec!(1.0));
        assert_eq!(params.price, Some(dec!(3000)));
        assert_eq!(params.time_in_force, Some(TimeInForce::IOC));
        assert_eq!(params.client_order_id, Some("test-123".to_string()));
        assert_eq!(params.post_only, Some(true));
    }

    #[test]
    fn test_balance_params() {
        let params = BalanceParams::futures().currencies(&["BTC", "USDT"]);
        assert_eq!(params.account_type, Some(AccountType::Futures));
        assert_eq!(
            params.currencies,
            Some(vec!["BTC".to_string(), "USDT".to_string()])
        );
    }

    #[test]
    fn test_leverage_params() {
        let params = LeverageParams::new("BTC/USDT:USDT", 10).isolated();
        assert_eq!(params.symbol, "BTC/USDT:USDT");
        assert_eq!(params.leverage, 10);
        assert_eq!(params.margin_mode, Some(MarginMode::Isolated));
    }

    #[test]
    fn test_withdraw_params() {
        let params = WithdrawParams::new("USDT", dec!(100), "TAddress123")
            .network("TRC20")
            .tag("memo");

        assert_eq!(params.currency, "USDT");
        assert_eq!(params.amount, dec!(100));
        assert_eq!(params.address, "TAddress123");
        assert_eq!(params.network, Some("TRC20".to_string()));
        assert_eq!(params.tag, Some("memo".to_string()));
    }

    #[test]
    fn test_transfer_params() {
        let params = TransferParams::spot_to_futures("USDT", dec!(1000));
        assert_eq!(params.currency, "USDT");
        assert_eq!(params.amount, dec!(1000));
        assert_eq!(params.from_account, AccountType::Spot);
        assert_eq!(params.to_account, AccountType::Futures);
    }
}
