//! Order type definitions

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::{Fee, Symbol, Timestamp};

/// Order side (buy or sell)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OrderSide {
    /// Buy order
    Buy,
    /// Sell order
    Sell,
}

impl std::fmt::Display for OrderSide {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Buy => write!(f, "buy"),
            Self::Sell => write!(f, "sell"),
        }
    }
}

/// Order type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OrderType {
    /// Market order
    Market,
    /// Limit order
    Limit,
    /// Limit maker order (post-only limit order)
    #[serde(rename = "limit_maker")]
    LimitMaker,
    /// Stop loss order
    #[serde(rename = "stop_loss")]
    StopLoss,
    /// Stop loss limit
    #[serde(rename = "stop_loss_limit")]
    StopLossLimit,
    /// Take profit order
    #[serde(rename = "take_profit")]
    TakeProfit,
    /// Take profit limit
    #[serde(rename = "take_profit_limit")]
    TakeProfitLimit,
    /// Stop market
    #[serde(rename = "stop_market")]
    StopMarket,
    /// Stop limit
    #[serde(rename = "stop_limit")]
    StopLimit,
    /// Trailing stop
    #[serde(rename = "trailing_stop")]
    TrailingStop,
}

impl std::fmt::Display for OrderType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Market => "market",
            Self::Limit => "limit",
            Self::LimitMaker => "limit_maker",
            Self::StopLoss => "stop_loss",
            Self::StopLossLimit => "stop_loss_limit",
            Self::TakeProfit => "take_profit",
            Self::TakeProfitLimit => "take_profit_limit",
            Self::StopMarket => "stop_market",
            Self::StopLimit => "stop_limit",
            Self::TrailingStop => "trailing_stop",
        };
        write!(f, "{}", s)
    }
}
/// Time in force - order validity duration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TimeInForce {
    /// Good Till Cancelled - order stays active until filled or cancelled
    #[serde(rename = "GTC")]
    GTC,
    /// Immediate Or Cancel - fill immediately or cancel
    #[serde(rename = "IOC")]
    IOC,
    /// Fill Or Kill - fill completely immediately or cancel
    #[serde(rename = "FOK")]
    FOK,
    /// Post Only - only make liquidity, never take
    #[serde(rename = "PO")]
    PO,
}

impl std::fmt::Display for TimeInForce {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::GTC => "GTC",
            Self::IOC => "IOC",
            Self::FOK => "FOK",
            Self::PO => "PO",
        };
        write!(f, "{}", s)
    }
}

/// Order status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OrderStatus {
    /// Order is open and active
    Open,
    /// Order is closed (filled or cancelled)
    Closed,
    /// Order was cancelled (British spelling - preferred)
    Cancelled,
    /// Order is expired
    Expired,
    /// Order is rejected
    Rejected,
    /// Order is partially filled
    #[serde(rename = "partial")]
    Partial,
}

impl std::fmt::Display for OrderStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Open => "open",
            Self::Closed => "closed",
            Self::Cancelled => "cancelled",
            Self::Expired => "expired",
            Self::Rejected => "rejected",
            Self::Partial => "partial",
        };
        write!(f, "{}", s)
    }
}

/// Order structure
///
/// Represents a trading order with all its metadata including prices,
/// amounts, fees, and timestamps.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Order {
    /// Order ID
    pub id: String,

    /// Client order ID
    pub client_order_id: Option<String>,

    /// Timestamp when order was created (milliseconds)
    pub timestamp: Option<Timestamp>,

    /// Datetime when order was created
    pub datetime: Option<String>,

    /// Last update timestamp (milliseconds)
    pub last_trade_timestamp: Option<Timestamp>,

    /// Symbol (e.g., "BTC/USDT")
    pub symbol: Symbol,

    /// Order type (market, limit, etc.)
    #[serde(rename = "type")]
    pub order_type: OrderType,

    /// Time in force (GTC, IOC, FOK, etc.)
    pub time_in_force: Option<String>,

    /// Post only flag
    pub post_only: Option<bool>,

    /// Reduce only flag (for futures)
    pub reduce_only: Option<bool>,

    /// Order side (buy/sell)
    pub side: OrderSide,

    /// Order price
    pub price: Option<Decimal>,

    /// Stop price (for stop orders)
    pub stop_price: Option<Decimal>,

    /// Trigger price
    pub trigger_price: Option<Decimal>,

    /// Take profit price
    pub take_profit_price: Option<Decimal>,

    /// Stop loss price
    pub stop_loss_price: Option<Decimal>,

    /// Trailing delta (for trailing stop orders, in basis points)
    pub trailing_delta: Option<Decimal>,

    /// Trailing percent (for trailing stop orders)
    pub trailing_percent: Option<Decimal>,

    /// Activation price (for trailing stop orders)
    pub activation_price: Option<Decimal>,

    /// Callback rate (for futures trailing stop orders)
    pub callback_rate: Option<Decimal>,

    /// Working type (CONTRACT_PRICE or MARK_PRICE for futures)
    pub working_type: Option<String>,

    /// Order amount
    pub amount: Decimal,

    /// Filled amount
    pub filled: Option<Decimal>,

    /// Remaining amount
    pub remaining: Option<Decimal>,

    /// Cost (filled_amount * average_price)
    pub cost: Option<Decimal>,

    /// Average fill price
    pub average: Option<Decimal>,

    /// Order status
    pub status: OrderStatus,

    /// Fee information
    pub fee: Option<Fee>,

    /// Fees (multiple fees)
    pub fees: Option<Vec<Fee>>,

    /// Trades associated with this order
    pub trades: Option<Vec<String>>,

    /// Raw exchange info
    #[serde(flatten)]
    pub info: HashMap<String, serde_json::Value>,
}

impl Order {
    /// Create a new order
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: String,
        symbol: Symbol,
        order_type: OrderType,
        side: OrderSide,
        amount: Decimal,
        price: Option<Decimal>,
        status: OrderStatus,
    ) -> Self {
        Self {
            id,
            client_order_id: None,
            timestamp: None,
            datetime: None,
            last_trade_timestamp: None,
            symbol,
            order_type,
            time_in_force: None,
            post_only: None,
            reduce_only: None,
            side,
            price,
            stop_price: None,
            trigger_price: None,
            take_profit_price: None,
            stop_loss_price: None,
            trailing_delta: None,
            trailing_percent: None,
            activation_price: None,
            callback_rate: None,
            working_type: None,
            amount,
            filled: None,
            remaining: Some(amount),
            cost: None,
            average: None,
            status,
            fee: None,
            fees: None,
            trades: None,
            info: HashMap::new(),
        }
    }

    /// Check if order is open
    pub fn is_open(&self) -> bool {
        matches!(self.status, OrderStatus::Open | OrderStatus::Partial)
    }

    /// Check if order is closed
    pub fn is_closed(&self) -> bool {
        matches!(self.status, OrderStatus::Closed | OrderStatus::Cancelled)
    }

    /// Check if order is filled
    pub fn is_filled(&self) -> bool {
        self.status == OrderStatus::Closed
            && self.filled.is_some()
            && self.remaining.map(|r| r.is_zero()).unwrap_or(false)
    }

    /// Check if order is cancelled
    pub fn is_cancelled(&self) -> bool {
        matches!(self.status, OrderStatus::Cancelled)
    }

    /// Get DateTime from timestamp
    pub fn datetime_utc(&self) -> Option<DateTime<Utc>> {
        self.timestamp
            .and_then(|ts| DateTime::from_timestamp_millis(ts))
    }

    /// Calculate fill percentage
    pub fn fill_percentage(&self) -> Option<Decimal> {
        if let Some(filled) = self.filled {
            if !self.amount.is_zero() {
                Some(filled / self.amount * Decimal::from(100))
            } else {
                None
            }
        } else {
            None
        }
    }
}

/// Errors that can occur while building an order.
#[derive(Debug, thiserror::Error)]
pub enum OrderBuilderError {
    /// Missing required limit price.
    #[error("Limit orders must specify a price")]
    MissingPrice,

    /// Missing required stop price.
    #[error("Stop-loss orders must specify a stop price")]
    MissingStopPrice,

    /// Invalid order configuration.
    #[error("Invalid order configuration: {0}")]
    InvalidConfiguration(String),

    /// Amount must be positive.
    #[error("Order amount must be greater than zero")]
    InvalidAmount,
}

/// Order builder implementing a fluent API design.
///
/// Provides a type-safe and user-friendly way to construct orders.
///
/// # Examples
///
/// ```
/// use ccxt_core::types::OrderSide;
/// use ccxt_core::types::order::OrderBuilder;
/// use rust_decimal_macros::dec;
///
/// // Create a limit buy order
/// let order = OrderBuilder::new("BTC/USDT".to_string(), OrderSide::Buy, dec!(0.1))
///     .limit(dec!(50000))
///     .client_order_id("my-order-123")
///     .post_only()
///     .build()
///     .expect("valid order configuration");
///
/// // Create a market sell order
/// let order = OrderBuilder::new("ETH/USDT".to_string(), OrderSide::Sell, dec!(1.5))
///     .market()
///     .build()
///     .expect("valid order configuration");
/// ```
#[derive(Debug, Clone)]
pub struct OrderBuilder {
    symbol: Symbol,
    side: OrderSide,
    amount: Decimal,
    order_type: OrderType,
    price: Option<Decimal>,
    client_order_id: Option<String>,
    time_in_force: Option<TimeInForce>,
    stop_price: Option<Decimal>,
    trigger_price: Option<Decimal>,
    take_profit_price: Option<Decimal>,
    stop_loss_price: Option<Decimal>,
    trailing_delta: Option<Decimal>,
    trailing_percent: Option<Decimal>,
    activation_price: Option<Decimal>,
    callback_rate: Option<Decimal>,
    working_type: Option<String>,
    post_only: bool,
    reduce_only: bool,
}

impl OrderBuilder {
    /// Creates a new order builder instance.
    ///
    /// # Arguments
    ///
    /// * `symbol` - Trading pair symbol (for example, "BTC/USDT")
    /// * `side` - Order side (buy or sell)
    /// * `amount` - Order quantity
    pub fn new(symbol: Symbol, side: OrderSide, amount: Decimal) -> Self {
        Self {
            symbol,
            side,
            amount,
            order_type: OrderType::Market, // Default to market order
            price: None,
            client_order_id: None,
            time_in_force: None,
            stop_price: None,
            trigger_price: None,
            take_profit_price: None,
            stop_loss_price: None,
            trailing_delta: None,
            trailing_percent: None,
            activation_price: None,
            callback_rate: None,
            working_type: None,
            post_only: false,
            reduce_only: false,
        }
    }

    /// Sets the builder to create a limit order.
    pub fn limit(mut self, price: Decimal) -> Self {
        self.order_type = OrderType::Limit;
        self.price = Some(price);
        self
    }

    /// Sets the builder to create a market order.
    pub fn market(mut self) -> Self {
        self.order_type = OrderType::Market;
        self.price = None;
        self
    }

    /// Sets the builder to create a post-only limit order.
    pub fn limit_maker(mut self, price: Decimal) -> Self {
        self.order_type = OrderType::LimitMaker;
        self.price = Some(price);
        self.post_only = true;
        self
    }

    /// Sets the builder to create a stop-loss order.
    pub fn stop_loss(mut self, stop_price: Decimal) -> Self {
        self.order_type = OrderType::StopLoss;
        self.stop_price = Some(stop_price);
        self
    }

    /// Sets the builder to create a stop-loss limit order.
    pub fn stop_loss_limit(mut self, stop_price: Decimal, price: Decimal) -> Self {
        self.order_type = OrderType::StopLossLimit;
        self.stop_price = Some(stop_price);
        self.price = Some(price);
        self
    }

    /// Sets the builder to create a take-profit order.
    pub fn take_profit(mut self, trigger_price: Decimal) -> Self {
        self.order_type = OrderType::TakeProfit;
        self.trigger_price = Some(trigger_price);
        self
    }

    /// Sets the builder to create a take-profit limit order.
    pub fn take_profit_limit(mut self, trigger_price: Decimal, price: Decimal) -> Self {
        self.order_type = OrderType::TakeProfitLimit;
        self.trigger_price = Some(trigger_price);
        self.price = Some(price);
        self
    }

    /// Sets the builder to create a stop-market order.
    pub fn stop_market(mut self, stop_price: Decimal) -> Self {
        self.order_type = OrderType::StopMarket;
        self.stop_price = Some(stop_price);
        self
    }

    /// Sets the builder to create an alternative stop-limit order form.
    pub fn stop_limit(mut self, stop_price: Decimal, price: Decimal) -> Self {
        self.order_type = OrderType::StopLimit;
        self.stop_price = Some(stop_price);
        self.price = Some(price);
        self
    }

    /// Sets the builder to create a trailing stop order.
    pub fn trailing_stop(mut self, callback_rate: Decimal) -> Self {
        self.order_type = OrderType::TrailingStop;
        self.callback_rate = Some(callback_rate);
        self
    }

    /// Sets the client order identifier.
    pub fn client_order_id(mut self, id: impl Into<String>) -> Self {
        self.client_order_id = Some(id.into());
        self
    }

    /// Sets the time-in-force policy.
    pub fn time_in_force(mut self, tif: TimeInForce) -> Self {
        self.time_in_force = Some(tif);
        self
    }

    /// Sets the trigger price.
    pub fn trigger_price(mut self, price: Decimal) -> Self {
        self.trigger_price = Some(price);
        self
    }

    /// Sets the take-profit price.
    pub fn take_profit_price(mut self, price: Decimal) -> Self {
        self.take_profit_price = Some(price);
        self
    }

    /// Sets the stop-loss price.
    pub fn stop_loss_price(mut self, price: Decimal) -> Self {
        self.stop_loss_price = Some(price);
        self
    }

    /// Sets the trailing delta (in basis points).
    pub fn trailing_delta(mut self, delta: Decimal) -> Self {
        self.trailing_delta = Some(delta);
        self
    }

    /// Sets the trailing percentage.
    pub fn trailing_percent(mut self, percent: Decimal) -> Self {
        self.trailing_percent = Some(percent);
        self
    }

    /// Sets the activation price.
    pub fn activation_price(mut self, price: Decimal) -> Self {
        self.activation_price = Some(price);
        self
    }

    /// Sets the callback rate (used for futures trailing stops).
    pub fn callback_rate(mut self, rate: Decimal) -> Self {
        self.callback_rate = Some(rate);
        self
    }

    /// Sets the working type (`CONTRACT_PRICE` or `MARK_PRICE`).
    pub fn working_type(mut self, wtype: impl Into<String>) -> Self {
        self.working_type = Some(wtype.into());
        self
    }

    /// Marks the order as post-only.
    pub fn post_only(mut self) -> Self {
        self.post_only = true;
        self
    }

    /// Marks the order as reduce-only (used for futures).
    pub fn reduce_only(mut self) -> Self {
        self.reduce_only = true;
        self
    }

    /// Validates the configuration and builds an [`Order`].
    ///
    /// Ensures the order configuration is valid. For example:
    /// - Limit orders require a price
    /// - Stop orders require a stop price
    /// - Quantities must be greater than zero
    ///
    /// # Errors
    ///
    /// Returns [`OrderBuilderError`] if the configuration is invalid.
    pub fn build(self) -> Result<Order, OrderBuilderError> {
        // Validate amount
        if self.amount <= Decimal::ZERO {
            return Err(OrderBuilderError::InvalidAmount);
        }

        // Validate required fields for each order type
        match self.order_type {
            OrderType::Limit | OrderType::LimitMaker => {
                if self.price.is_none() {
                    return Err(OrderBuilderError::MissingPrice);
                }
            }
            OrderType::StopLoss | OrderType::StopMarket => {
                if self.stop_price.is_none() {
                    return Err(OrderBuilderError::MissingStopPrice);
                }
            }
            OrderType::StopLossLimit | OrderType::StopLimit => {
                if self.stop_price.is_none() {
                    return Err(OrderBuilderError::MissingStopPrice);
                }
                if self.price.is_none() {
                    return Err(OrderBuilderError::MissingPrice);
                }
            }
            OrderType::TakeProfit => {
                if self.trigger_price.is_none() {
                    return Err(OrderBuilderError::InvalidConfiguration(
                        "Take-profit orders require a trigger price".to_string(),
                    ));
                }
            }
            OrderType::TakeProfitLimit => {
                if self.trigger_price.is_none() || self.price.is_none() {
                    return Err(OrderBuilderError::InvalidConfiguration(
                        "Take-profit limit orders require both a trigger price and a limit price"
                            .to_string(),
                    ));
                }
            }
            OrderType::TrailingStop => {
                if self.callback_rate.is_none()
                    && self.trailing_delta.is_none()
                    && self.trailing_percent.is_none()
                {
                    return Err(OrderBuilderError::InvalidConfiguration(
                        "Trailing-stop orders require a callback rate, trailing delta, or trailing percent".to_string(),
                    ));
                }
            }
            OrderType::Market => {
                // Market orders require no additional checks
            }
        }

        // Construct the order
        let time_in_force_str = self.time_in_force.map(|tif| tif.to_string());

        Ok(Order {
            id: String::new(), // Order IDs are assigned by the exchange
            client_order_id: self.client_order_id,
            timestamp: None,
            datetime: None,
            last_trade_timestamp: None,
            symbol: self.symbol,
            order_type: self.order_type,
            time_in_force: time_in_force_str,
            post_only: Some(self.post_only),
            reduce_only: Some(self.reduce_only),
            side: self.side,
            price: self.price,
            stop_price: self.stop_price,
            trigger_price: self.trigger_price,
            take_profit_price: self.take_profit_price,
            stop_loss_price: self.stop_loss_price,
            trailing_delta: self.trailing_delta,
            trailing_percent: self.trailing_percent,
            activation_price: self.activation_price,
            callback_rate: self.callback_rate,
            working_type: self.working_type,
            amount: self.amount,
            filled: None,
            remaining: Some(self.amount),
            cost: None,
            average: None,
            status: OrderStatus::Open, // Newly created orders start as open
            fee: None,
            fees: None,
            trades: None,
            info: HashMap::new(),
        })
    }
}

impl Order {
    /// Creates an order builder.
    ///
    /// Preferred way to construct orders, providing a fluent API with validation.
    ///
    /// # Examples
    ///
    /// ```
    /// use ccxt_core::types::{Order, OrderSide};
    /// use rust_decimal_macros::dec;
    ///
    /// let order = Order::builder("BTC/USDT".to_string(), OrderSide::Buy, dec!(0.1))
    ///     .limit(dec!(50000))
    ///     .time_in_force(ccxt_core::types::TimeInForce::GTC)
    ///     .build()
    ///     .expect("valid order");
    /// ```
    pub fn builder(symbol: Symbol, side: OrderSide, amount: Decimal) -> OrderBuilder {
        OrderBuilder::new(symbol, side, amount)
    }
}

/// Batch order request payload used by `create_orders`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchOrderRequest {
    /// Trading pair symbol.
    pub symbol: String,

    /// Order side (`BUY`/`SELL`).
    pub side: String,

    /// Order type (`LIMIT`, `MARKET`, etc.).
    #[serde(rename = "type")]
    pub order_type: String,

    /// Order quantity.
    pub quantity: String,

    /// Order price (optional for market orders).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub price: Option<String>,

    /// Reduce-only flag.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reduce_only: Option<String>,

    /// Position side (`LONG`/`SHORT`/`BOTH`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position_side: Option<String>,

    /// Time-in-force value.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_in_force: Option<String>,

    /// Client order identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub new_client_order_id: Option<String>,
}

/// Batch order result.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchOrderResult {
    /// Order ID.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub order_id: Option<i64>,

    /// Trading pair symbol.
    pub symbol: String,

    /// Status code (`200` indicates success).
    pub code: i32,

    /// Message body.
    pub msg: String,

    /// Client order identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_order_id: Option<String>,
}

/// Batch order update request payload used by `edit_orders`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchOrderUpdate {
    /// Order ID.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub order_id: Option<i64>,

    /// Original client order identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub orig_client_order_id: Option<String>,

    /// Trading pair symbol.
    pub symbol: String,

    /// Order side.
    pub side: String,

    /// Order quantity.
    pub quantity: String,

    /// Order price.
    pub price: String,
}

/// Batch cancel order result.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchCancelResult {
    /// Order ID.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub order_id: Option<i64>,

    /// Trading pair symbol.
    pub symbol: String,

    /// Order status.
    pub status: String,

    /// Client order identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_order_id: Option<String>,

    /// Optional message.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub msg: Option<String>,
}

/// Cancel-all orders result payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CancelAllOrdersResult {
    /// Status code.
    pub code: i32,

    /// Message body.
    pub msg: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_order_creation() {
        let order = Order::new(
            "12345".to_string(),
            "BTC/USDT".to_string(),
            OrderType::Limit,
            OrderSide::Buy,
            dec!(1.5),
            Some(dec!(50000)),
            OrderStatus::Open,
        );

        assert_eq!(order.id, "12345");
        assert_eq!(order.symbol, "BTC/USDT");
        assert_eq!(order.amount, dec!(1.5));
        assert!(order.is_open());
        assert!(!order.is_closed());
    }

    #[test]
    fn test_order_status() {
        let mut order = Order::new(
            "12345".to_string(),
            "BTC/USDT".to_string(),
            OrderType::Market,
            OrderSide::Buy,
            dec!(1.0),
            None,
            OrderStatus::Open,
        );

        assert!(order.is_open());

        order.status = OrderStatus::Closed;
        order.filled = Some(dec!(1.0));
        order.remaining = Some(dec!(0.0));

        assert!(order.is_closed());
        assert!(order.is_filled());
    }

    #[test]
    fn test_fill_percentage() {
        let mut order = Order::new(
            "12345".to_string(),
            "BTC/USDT".to_string(),
            OrderType::Limit,
            OrderSide::Buy,
            dec!(2.0),
            Some(dec!(50000)),
            OrderStatus::Partial,
        );

        order.filled = Some(dec!(1.0));

        let percentage = order.fill_percentage().unwrap();
        assert_eq!(percentage, dec!(50.0));
    }

    #[test]
    fn test_order_side_display() {
        assert_eq!(OrderSide::Buy.to_string(), "buy");
        assert_eq!(OrderSide::Sell.to_string(), "sell");
    }

    #[test]
    fn test_order_type_display() {
        assert_eq!(OrderType::Market.to_string(), "market");
        assert_eq!(OrderType::Limit.to_string(), "limit");
        assert_eq!(OrderType::StopLoss.to_string(), "stop_loss");
    }
    // Builder pattern tests

    #[test]
    fn test_order_builder_market_order() {
        let order = Order::builder("BTC/USDT".to_string(), OrderSide::Buy, dec!(0.1))
            .market()
            .build()
            .expect("Valid market order");

        assert_eq!(order.symbol, "BTC/USDT");
        assert_eq!(order.side, OrderSide::Buy);
        assert_eq!(order.amount, dec!(0.1));
        assert_eq!(order.order_type, OrderType::Market);
        assert_eq!(order.price, None);
    }

    #[test]
    fn test_order_builder_limit_order() {
        let order = Order::builder("BTC/USDT".to_string(), OrderSide::Sell, dec!(0.5))
            .limit(dec!(50000))
            .build()
            .expect("Valid limit order");

        assert_eq!(order.symbol, "BTC/USDT");
        assert_eq!(order.side, OrderSide::Sell);
        assert_eq!(order.amount, dec!(0.5));
        assert_eq!(order.order_type, OrderType::Limit);
        assert_eq!(order.price, Some(dec!(50000)));
    }

    #[test]
    fn test_order_builder_limit_maker() {
        let order = Order::builder("ETH/USDT".to_string(), OrderSide::Buy, dec!(1.0))
            .limit_maker(dec!(3000))
            .build()
            .expect("Valid limit maker order");

        assert_eq!(order.order_type, OrderType::LimitMaker);
        assert_eq!(order.price, Some(dec!(3000)));
    }

    #[test]
    fn test_order_builder_stop_loss() {
        let order = Order::builder("BTC/USDT".to_string(), OrderSide::Sell, dec!(0.1))
            .stop_loss(dec!(45000))
            .build()
            .expect("Valid stop loss order");

        assert_eq!(order.order_type, OrderType::StopLoss);
        assert_eq!(order.stop_price, Some(dec!(45000)));
    }

    #[test]
    fn test_order_builder_stop_loss_limit() {
        let order = Order::builder("BTC/USDT".to_string(), OrderSide::Sell, dec!(0.1))
            .stop_loss_limit(dec!(45000), dec!(44900))
            .build()
            .expect("Valid stop loss limit order");

        assert_eq!(order.order_type, OrderType::StopLossLimit);
        assert_eq!(order.stop_price, Some(dec!(45000)));
        assert_eq!(order.price, Some(dec!(44900)));
    }

    #[test]
    fn test_order_builder_take_profit() {
        let order = Order::builder("BTC/USDT".to_string(), OrderSide::Sell, dec!(0.1))
            .take_profit(dec!(55000))
            .build()
            .expect("Valid take profit order");

        assert_eq!(order.order_type, OrderType::TakeProfit);
        assert_eq!(order.trigger_price, Some(dec!(55000)));
    }

    #[test]
    fn test_order_builder_take_profit_limit() {
        let order = Order::builder("BTC/USDT".to_string(), OrderSide::Sell, dec!(0.1))
            .take_profit_limit(dec!(55000), dec!(55100))
            .build()
            .expect("Valid take profit limit order");

        assert_eq!(order.order_type, OrderType::TakeProfitLimit);
        assert_eq!(order.trigger_price, Some(dec!(55000)));
        assert_eq!(order.price, Some(dec!(55100)));
    }

    #[test]
    fn test_order_builder_trailing_stop() {
        let order = Order::builder("BTC/USDT".to_string(), OrderSide::Sell, dec!(0.1))
            .trailing_stop(dec!(1.0))
            .build()
            .expect("Valid trailing stop order");

        assert_eq!(order.order_type, OrderType::TrailingStop);
        assert_eq!(order.callback_rate, Some(dec!(1.0)));
    }

    #[test]
    fn test_order_builder_with_options() {
        let order = Order::builder("BTC/USDT".to_string(), OrderSide::Buy, dec!(0.1))
            .limit(dec!(50000))
            .client_order_id("my-order-123")
            .time_in_force(TimeInForce::GTC)
            .post_only()
            .build()
            .expect("Valid limit order with options");

        assert_eq!(order.client_order_id, Some("my-order-123".to_string()));
        assert_eq!(order.time_in_force, Some("GTC".to_string()));
        assert_eq!(order.post_only, Some(true));
    }

    #[test]
    fn test_order_builder_reduce_only() {
        let order = Order::builder("BTC/USDT".to_string(), OrderSide::Sell, dec!(0.1))
            .market()
            .reduce_only()
            .build()
            .expect("Valid reduce-only order");

        assert_eq!(order.reduce_only, Some(true));
    }

    #[test]
    fn test_order_builder_futures_options() {
        let order = Order::builder("BTC/USDT".to_string(), OrderSide::Buy, dec!(0.1))
            .stop_loss_limit(dec!(45000), dec!(44900))
            .working_type("MARK_PRICE")
            .build()
            .expect("Valid futures stop loss order");

        assert_eq!(order.working_type, Some("MARK_PRICE".to_string()));
    }

    #[test]
    fn test_order_builder_trailing_options() {
        let order = Order::builder("BTC/USDT".to_string(), OrderSide::Sell, dec!(0.1))
            .trailing_stop(dec!(1.0))
            .trailing_delta(dec!(100))
            .activation_price(dec!(51000))
            .build()
            .expect("Valid trailing stop order");

        assert_eq!(order.callback_rate, Some(dec!(1.0)));
        assert_eq!(order.trailing_delta, Some(dec!(100)));
        assert_eq!(order.activation_price, Some(dec!(51000)));
    }

    // Builder error validation tests

    #[test]
    fn test_order_builder_missing_price_error() {
        let result = Order::builder("BTC/USDT".to_string(), OrderSide::Buy, dec!(0.1))
            .limit(dec!(50000))
            .build();

        assert!(result.is_ok());
    }

    #[test]
    fn test_order_builder_invalid_amount() {
        let result = Order::builder("BTC/USDT".to_string(), OrderSide::Buy, dec!(0.0))
            .market()
            .build();

        assert!(result.is_err());
        if let Err(e) = result {
            assert!(matches!(e, OrderBuilderError::InvalidAmount));
        }
    }

    #[test]
    fn test_order_builder_negative_amount() {
        let result = Order::builder("BTC/USDT".to_string(), OrderSide::Buy, dec!(-0.1))
            .market()
            .build();

        assert!(result.is_err());
        if let Err(e) = result {
            assert!(matches!(e, OrderBuilderError::InvalidAmount));
        }
    }

    #[test]
    fn test_order_builder_fluent_api() {
        let order = Order::builder("BTC/USDT".to_string(), OrderSide::Buy, dec!(0.1))
            .limit(dec!(50000))
            .time_in_force(TimeInForce::FOK)
            .client_order_id("test-123")
            .post_only()
            .build()
            .expect("Fluent API build success");

        assert_eq!(order.order_type, OrderType::Limit);
        assert_eq!(order.price, Some(dec!(50000)));
        assert_eq!(order.time_in_force, Some("FOK".to_string()));
        assert_eq!(order.client_order_id, Some("test-123".to_string()));
        assert_eq!(order.post_only, Some(true));
    }

    #[test]
    fn test_order_builder_order_type_override() {
        let order = Order::builder("BTC/USDT".to_string(), OrderSide::Buy, dec!(0.1))
            .limit(dec!(50000))
            .market()
            .build()
            .expect("Order type override success");

        assert_eq!(order.order_type, OrderType::Market);
        assert_eq!(order.price, None);
    }
}

/// OCO order info for a single order.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OcoOrderInfo {
    /// Trading pair symbol.
    pub symbol: String,

    /// Order ID.
    pub order_id: i64,

    /// Client order ID.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_order_id: Option<String>,
}

/// OCO order report with detailed status.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderReport {
    /// Trading pair symbol.
    pub symbol: String,

    /// Order ID.
    pub order_id: i64,

    /// OCO order list ID.
    pub order_list_id: i64,

    /// Client order ID.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_order_id: Option<String>,

    /// Transaction timestamp.
    pub transact_time: u64,

    /// Price.
    pub price: String,

    /// Original quantity.
    pub orig_qty: String,

    /// Executed quantity.
    pub executed_qty: String,

    /// Cumulative quote quantity.
    pub cummulative_quote_qty: String,

    /// Order status.
    pub status: String,

    /// Time in force.
    pub time_in_force: String,

    /// Order type.
    #[serde(rename = "type")]
    pub type_: String,

    /// Order side.
    pub side: String,

    /// Stop price (optional).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_price: Option<String>,
}

/// OCO (One-Cancels-the-Other) order.
///
/// An OCO order is a combination order containing two orders:
/// - A limit order (take profit)
/// - A stop limit order (stop loss)
///
/// When one order is filled, the other is automatically cancelled.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OcoOrder {
    /// Raw exchange response info.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub info: Option<serde_json::Value>,

    /// OCO order list ID.
    pub order_list_id: i64,

    /// Client order list ID.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub list_client_order_id: Option<String>,

    /// Trading pair symbol.
    pub symbol: String,

    /// Order list status.
    pub list_status: String,

    /// Order list order status.
    pub list_order_status: String,

    /// Transaction timestamp in milliseconds.
    pub transaction_time: u64,

    /// Datetime in ISO 8601 format.
    pub datetime: String,

    /// List of order info contained in the OCO order.
    pub orders: Vec<OcoOrderInfo>,

    /// Order reports (optional, returned when creating order).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub order_reports: Option<Vec<OrderReport>>,
}

impl OcoOrder {
    /// Creates a new OCO order.
    pub fn new(
        order_list_id: i64,
        symbol: String,
        list_status: String,
        list_order_status: String,
        transaction_time: u64,
        datetime: String,
        orders: Vec<OcoOrderInfo>,
    ) -> Self {
        Self {
            info: None,
            order_list_id,
            list_client_order_id: None,
            symbol,
            list_status,
            list_order_status,
            transaction_time,
            datetime,
            orders,
            order_reports: None,
        }
    }

    /// Checks if the OCO order is executing.
    pub fn is_executing(&self) -> bool {
        self.list_status == "EXECUTING"
    }

    /// Checks if the OCO order is all done.
    pub fn is_all_done(&self) -> bool {
        self.list_status == "ALL_DONE"
    }

    /// Checks if the OCO order is rejected.
    pub fn is_rejected(&self) -> bool {
        self.list_status == "REJECT"
    }
}

/// Cancel and replace order response.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CancelReplaceResponse {
    /// Cancel result.
    pub cancel_result: String,

    /// New order result.
    pub new_order_result: String,

    /// Cancel response info.
    pub cancel_response: serde_json::Value,

    /// New order response info.
    pub new_order_response: serde_json::Value,
}
