//! Order request types with Builder pattern using typestate.
//!
//! This module provides a type-safe way to construct order requests using the
//! typestate pattern, which ensures at compile-time that all required fields
//! are set before building an order request.
//!
//! # Examples
//!
//! ```rust
//! use ccxt_core::types::order_request::{OrderRequest, OrderRequestBuilder};
//! use ccxt_core::types::{OrderSide, OrderType, TimeInForce};
//! use ccxt_core::types::financial::Amount;
//! use rust_decimal_macros::dec;
//!
//! // Create a market order using the builder
//! let request = OrderRequest::builder()
//!     .symbol("BTC/USDT")
//!     .side(OrderSide::Buy)
//!     .order_type(OrderType::Market)
//!     .amount(Amount::new(dec!(0.1)))
//!     .build();
//!
//! assert_eq!(request.symbol, "BTC/USDT");
//! assert_eq!(request.side, OrderSide::Buy);
//! ```

use crate::types::financial::{Amount, Price};
use crate::types::order::{OrderSide, OrderType, TimeInForce};
use serde::{Deserialize, Serialize};
use std::marker::PhantomData;

// ============================================================================
// OrderRequest struct
// ============================================================================

/// Order request configuration built via builder pattern.
///
/// This struct contains all the fields needed to create an order on an exchange.
/// Use [`OrderRequest::builder()`] to construct instances with compile-time
/// validation of required fields.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderRequest {
    /// Trading pair symbol (e.g., "BTC/USDT")
    pub symbol: String,

    /// Order side (buy or sell)
    pub side: OrderSide,

    /// Order type (market, limit, etc.)
    pub order_type: OrderType,

    /// Order amount/quantity
    pub amount: Amount,

    /// Order price (required for limit orders)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub price: Option<Price>,

    /// Stop price (for stop orders)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_price: Option<Price>,

    /// Time in force (GTC, IOC, FOK, etc.)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_in_force: Option<TimeInForce>,

    /// Client-specified order ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_order_id: Option<String>,

    /// Reduce-only flag (for futures)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reduce_only: Option<bool>,

    /// Post-only flag (maker only)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub post_only: Option<bool>,

    /// Trigger price (for conditional orders)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trigger_price: Option<Price>,

    /// Take profit price
    #[serde(skip_serializing_if = "Option::is_none")]
    pub take_profit_price: Option<Price>,

    /// Stop loss price
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_loss_price: Option<Price>,

    /// Trailing delta (in basis points)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trailing_delta: Option<rust_decimal::Decimal>,

    /// Trailing percent
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trailing_percent: Option<rust_decimal::Decimal>,

    /// Activation price (for trailing stop orders)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub activation_price: Option<Price>,

    /// Callback rate (for futures trailing stop orders)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub callback_rate: Option<rust_decimal::Decimal>,

    /// Working type (CONTRACT_PRICE or MARK_PRICE for futures)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub working_type: Option<String>,

    /// Position side (for hedge mode futures)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position_side: Option<String>,
}

impl OrderRequest {
    /// Creates a new builder for constructing an OrderRequest.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use ccxt_core::types::order_request::OrderRequest;
    /// use ccxt_core::types::{OrderSide, OrderType};
    /// use ccxt_core::types::financial::Amount;
    /// use rust_decimal_macros::dec;
    ///
    /// let request = OrderRequest::builder()
    ///     .symbol("BTC/USDT")
    ///     .side(OrderSide::Buy)
    ///     .order_type(OrderType::Market)
    ///     .amount(Amount::new(dec!(0.1)))
    ///     .build();
    /// ```
    pub fn builder() -> OrderRequestBuilder<Missing, Missing, Missing, Missing> {
        OrderRequestBuilder::new()
    }
}

// ============================================================================
// Typestate markers
// ============================================================================

/// Marker type indicating a required field has not been set.
#[derive(Debug, Clone, Copy, Default)]
pub struct Missing;

/// Marker type indicating a required field has been set with value T.
#[derive(Debug, Clone)]
pub struct Set<T>(pub T);

// ============================================================================
// OrderRequestBuilder with typestate pattern
// ============================================================================

/// Builder for OrderRequest with typestate pattern for required fields.
///
/// The typestate pattern ensures at compile-time that all required fields
/// (symbol, side, order_type, amount) are set before the `build()` method
/// becomes available.
///
/// # Type Parameters
///
/// * `Symbol` - Either `Missing` or `Set<String>` for the symbol field
/// * `Side` - Either `Missing` or `Set<OrderSide>` for the side field
/// * `Type` - Either `Missing` or `Set<OrderType>` for the order_type field
/// * `Amt` - Either `Missing` or `Set<Amount>` for the amount field
///
/// # Examples
///
/// ```rust
/// use ccxt_core::types::order_request::OrderRequestBuilder;
/// use ccxt_core::types::{OrderSide, OrderType, TimeInForce};
/// use ccxt_core::types::financial::{Amount, Price};
/// use rust_decimal_macros::dec;
///
/// // This compiles - all required fields are set
/// let request = OrderRequestBuilder::new()
///     .symbol("BTC/USDT")
///     .side(OrderSide::Buy)
///     .order_type(OrderType::Limit)
///     .amount(Amount::new(dec!(0.1)))
///     .price(Price::new(dec!(50000)))
///     .time_in_force(TimeInForce::GTC)
///     .build();
/// ```
#[derive(Debug, Clone)]
pub struct OrderRequestBuilder<Symbol, Side, Type, Amt> {
    symbol: Symbol,
    side: Side,
    order_type: Type,
    amount: Amt,
    price: Option<Price>,
    stop_price: Option<Price>,
    time_in_force: Option<TimeInForce>,
    client_order_id: Option<String>,
    reduce_only: Option<bool>,
    post_only: Option<bool>,
    trigger_price: Option<Price>,
    take_profit_price: Option<Price>,
    stop_loss_price: Option<Price>,
    trailing_delta: Option<rust_decimal::Decimal>,
    trailing_percent: Option<rust_decimal::Decimal>,
    activation_price: Option<Price>,
    callback_rate: Option<rust_decimal::Decimal>,
    working_type: Option<String>,
    position_side: Option<String>,
    _marker: PhantomData<(Symbol, Side, Type, Amt)>,
}

impl OrderRequestBuilder<Missing, Missing, Missing, Missing> {
    /// Creates a new OrderRequestBuilder with all required fields unset.
    pub fn new() -> Self {
        Self {
            symbol: Missing,
            side: Missing,
            order_type: Missing,
            amount: Missing,
            price: None,
            stop_price: None,
            time_in_force: None,
            client_order_id: None,
            reduce_only: None,
            post_only: None,
            trigger_price: None,
            take_profit_price: None,
            stop_loss_price: None,
            trailing_delta: None,
            trailing_percent: None,
            activation_price: None,
            callback_rate: None,
            working_type: None,
            position_side: None,
            _marker: PhantomData,
        }
    }
}

impl Default for OrderRequestBuilder<Missing, Missing, Missing, Missing> {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Required field setters (transition from Missing to Set)
// ============================================================================

impl<Side, Type, Amt> OrderRequestBuilder<Missing, Side, Type, Amt> {
    /// Sets the trading symbol (required).
    ///
    /// # Arguments
    ///
    /// * `symbol` - Trading pair symbol (e.g., "BTC/USDT")
    pub fn symbol(
        self,
        symbol: impl Into<String>,
    ) -> OrderRequestBuilder<Set<String>, Side, Type, Amt> {
        OrderRequestBuilder {
            symbol: Set(symbol.into()),
            side: self.side,
            order_type: self.order_type,
            amount: self.amount,
            price: self.price,
            stop_price: self.stop_price,
            time_in_force: self.time_in_force,
            client_order_id: self.client_order_id,
            reduce_only: self.reduce_only,
            post_only: self.post_only,
            trigger_price: self.trigger_price,
            take_profit_price: self.take_profit_price,
            stop_loss_price: self.stop_loss_price,
            trailing_delta: self.trailing_delta,
            trailing_percent: self.trailing_percent,
            activation_price: self.activation_price,
            callback_rate: self.callback_rate,
            working_type: self.working_type,
            position_side: self.position_side,
            _marker: PhantomData,
        }
    }
}

impl<Symbol, Type, Amt> OrderRequestBuilder<Symbol, Missing, Type, Amt> {
    /// Sets the order side (required).
    ///
    /// # Arguments
    ///
    /// * `side` - Order side (Buy or Sell)
    pub fn side(self, side: OrderSide) -> OrderRequestBuilder<Symbol, Set<OrderSide>, Type, Amt> {
        OrderRequestBuilder {
            symbol: self.symbol,
            side: Set(side),
            order_type: self.order_type,
            amount: self.amount,
            price: self.price,
            stop_price: self.stop_price,
            time_in_force: self.time_in_force,
            client_order_id: self.client_order_id,
            reduce_only: self.reduce_only,
            post_only: self.post_only,
            trigger_price: self.trigger_price,
            take_profit_price: self.take_profit_price,
            stop_loss_price: self.stop_loss_price,
            trailing_delta: self.trailing_delta,
            trailing_percent: self.trailing_percent,
            activation_price: self.activation_price,
            callback_rate: self.callback_rate,
            working_type: self.working_type,
            position_side: self.position_side,
            _marker: PhantomData,
        }
    }
}

impl<Symbol, Side, Amt> OrderRequestBuilder<Symbol, Side, Missing, Amt> {
    /// Sets the order type (required).
    ///
    /// # Arguments
    ///
    /// * `order_type` - Order type (Market, Limit, etc.)
    pub fn order_type(
        self,
        order_type: OrderType,
    ) -> OrderRequestBuilder<Symbol, Side, Set<OrderType>, Amt> {
        OrderRequestBuilder {
            symbol: self.symbol,
            side: self.side,
            order_type: Set(order_type),
            amount: self.amount,
            price: self.price,
            stop_price: self.stop_price,
            time_in_force: self.time_in_force,
            client_order_id: self.client_order_id,
            reduce_only: self.reduce_only,
            post_only: self.post_only,
            trigger_price: self.trigger_price,
            take_profit_price: self.take_profit_price,
            stop_loss_price: self.stop_loss_price,
            trailing_delta: self.trailing_delta,
            trailing_percent: self.trailing_percent,
            activation_price: self.activation_price,
            callback_rate: self.callback_rate,
            working_type: self.working_type,
            position_side: self.position_side,
            _marker: PhantomData,
        }
    }
}

impl<Symbol, Side, Type> OrderRequestBuilder<Symbol, Side, Type, Missing> {
    /// Sets the order amount (required).
    ///
    /// # Arguments
    ///
    /// * `amount` - Order quantity
    pub fn amount(self, amount: Amount) -> OrderRequestBuilder<Symbol, Side, Type, Set<Amount>> {
        OrderRequestBuilder {
            symbol: self.symbol,
            side: self.side,
            order_type: self.order_type,
            amount: Set(amount),
            price: self.price,
            stop_price: self.stop_price,
            time_in_force: self.time_in_force,
            client_order_id: self.client_order_id,
            reduce_only: self.reduce_only,
            post_only: self.post_only,
            trigger_price: self.trigger_price,
            take_profit_price: self.take_profit_price,
            stop_loss_price: self.stop_loss_price,
            trailing_delta: self.trailing_delta,
            trailing_percent: self.trailing_percent,
            activation_price: self.activation_price,
            callback_rate: self.callback_rate,
            working_type: self.working_type,
            position_side: self.position_side,
            _marker: PhantomData,
        }
    }
}

// ============================================================================
// Optional field setters (available at any state)
// ============================================================================

impl<Symbol, Side, Type, Amt> OrderRequestBuilder<Symbol, Side, Type, Amt> {
    /// Sets the order price (optional, required for limit orders).
    ///
    /// # Arguments
    ///
    /// * `price` - Order price
    pub fn price(mut self, price: Price) -> Self {
        self.price = Some(price);
        self
    }

    /// Sets the stop price (optional, for stop orders).
    ///
    /// # Arguments
    ///
    /// * `stop_price` - Stop trigger price
    pub fn stop_price(mut self, stop_price: Price) -> Self {
        self.stop_price = Some(stop_price);
        self
    }

    /// Sets the time in force (optional).
    ///
    /// # Arguments
    ///
    /// * `tif` - Time in force (GTC, IOC, FOK, PO)
    pub fn time_in_force(mut self, tif: TimeInForce) -> Self {
        self.time_in_force = Some(tif);
        self
    }

    /// Sets the client order ID (optional).
    ///
    /// # Arguments
    ///
    /// * `id` - Client-specified order identifier
    pub fn client_order_id(mut self, id: impl Into<String>) -> Self {
        self.client_order_id = Some(id.into());
        self
    }

    /// Sets the reduce-only flag (optional, for futures).
    ///
    /// When true, the order will only reduce an existing position.
    pub fn reduce_only(mut self, reduce_only: bool) -> Self {
        self.reduce_only = Some(reduce_only);
        self
    }

    /// Sets the post-only flag (optional).
    ///
    /// When true, the order will only be placed if it would be a maker order.
    pub fn post_only(mut self, post_only: bool) -> Self {
        self.post_only = Some(post_only);
        self
    }

    /// Sets the trigger price (optional, for conditional orders).
    ///
    /// # Arguments
    ///
    /// * `trigger_price` - Price at which the order is triggered
    pub fn trigger_price(mut self, trigger_price: Price) -> Self {
        self.trigger_price = Some(trigger_price);
        self
    }

    /// Sets the take profit price (optional).
    ///
    /// # Arguments
    ///
    /// * `take_profit_price` - Take profit trigger price
    pub fn take_profit_price(mut self, take_profit_price: Price) -> Self {
        self.take_profit_price = Some(take_profit_price);
        self
    }

    /// Sets the stop loss price (optional).
    ///
    /// # Arguments
    ///
    /// * `stop_loss_price` - Stop loss trigger price
    pub fn stop_loss_price(mut self, stop_loss_price: Price) -> Self {
        self.stop_loss_price = Some(stop_loss_price);
        self
    }

    /// Sets the trailing delta in basis points (optional).
    ///
    /// # Arguments
    ///
    /// * `delta` - Trailing delta in basis points
    pub fn trailing_delta(mut self, delta: rust_decimal::Decimal) -> Self {
        self.trailing_delta = Some(delta);
        self
    }

    /// Sets the trailing percent (optional).
    ///
    /// # Arguments
    ///
    /// * `percent` - Trailing percentage
    pub fn trailing_percent(mut self, percent: rust_decimal::Decimal) -> Self {
        self.trailing_percent = Some(percent);
        self
    }

    /// Sets the activation price (optional, for trailing stop orders).
    ///
    /// # Arguments
    ///
    /// * `activation_price` - Price at which trailing stop activates
    pub fn activation_price(mut self, activation_price: Price) -> Self {
        self.activation_price = Some(activation_price);
        self
    }

    /// Sets the callback rate (optional, for futures trailing stop orders).
    ///
    /// # Arguments
    ///
    /// * `rate` - Callback rate percentage
    pub fn callback_rate(mut self, rate: rust_decimal::Decimal) -> Self {
        self.callback_rate = Some(rate);
        self
    }

    /// Sets the working type (optional, for futures).
    ///
    /// # Arguments
    ///
    /// * `working_type` - "CONTRACT_PRICE" or "MARK_PRICE"
    pub fn working_type(mut self, working_type: impl Into<String>) -> Self {
        self.working_type = Some(working_type.into());
        self
    }

    /// Sets the position side (optional, for hedge mode futures).
    ///
    /// # Arguments
    ///
    /// * `position_side` - "LONG", "SHORT", or "BOTH"
    pub fn position_side(mut self, position_side: impl Into<String>) -> Self {
        self.position_side = Some(position_side.into());
        self
    }
}

// ============================================================================
// Build method (only available when all required fields are set)
// ============================================================================

impl OrderRequestBuilder<Set<String>, Set<OrderSide>, Set<OrderType>, Set<Amount>> {
    /// Builds the final OrderRequest.
    ///
    /// This method is only available when all required fields (symbol, side,
    /// order_type, amount) have been set.
    ///
    /// # Returns
    ///
    /// A fully constructed `OrderRequest` with all specified fields.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use ccxt_core::types::order_request::OrderRequest;
    /// use ccxt_core::types::{OrderSide, OrderType};
    /// use ccxt_core::types::financial::Amount;
    /// use rust_decimal_macros::dec;
    ///
    /// let request = OrderRequest::builder()
    ///     .symbol("BTC/USDT")
    ///     .side(OrderSide::Buy)
    ///     .order_type(OrderType::Market)
    ///     .amount(Amount::new(dec!(0.1)))
    ///     .build();
    ///
    /// assert_eq!(request.symbol, "BTC/USDT");
    /// ```
    pub fn build(self) -> OrderRequest {
        OrderRequest {
            symbol: self.symbol.0,
            side: self.side.0,
            order_type: self.order_type.0,
            amount: self.amount.0,
            price: self.price,
            stop_price: self.stop_price,
            time_in_force: self.time_in_force,
            client_order_id: self.client_order_id,
            reduce_only: self.reduce_only,
            post_only: self.post_only,
            trigger_price: self.trigger_price,
            take_profit_price: self.take_profit_price,
            stop_loss_price: self.stop_loss_price,
            trailing_delta: self.trailing_delta,
            trailing_percent: self.trailing_percent,
            activation_price: self.activation_price,
            callback_rate: self.callback_rate,
            working_type: self.working_type,
            position_side: self.position_side,
        }
    }
}

// ============================================================================
// Default implementations for OrderRequest
// ============================================================================

impl OrderRequest {
    /// Returns true if this is a market order.
    pub fn is_market_order(&self) -> bool {
        matches!(self.order_type, OrderType::Market)
    }

    /// Returns true if this is a limit order.
    pub fn is_limit_order(&self) -> bool {
        matches!(self.order_type, OrderType::Limit | OrderType::LimitMaker)
    }

    /// Returns true if this is a stop order.
    pub fn is_stop_order(&self) -> bool {
        matches!(
            self.order_type,
            OrderType::StopLoss
                | OrderType::StopLossLimit
                | OrderType::StopMarket
                | OrderType::StopLimit
        )
    }

    /// Returns true if this is a take profit order.
    pub fn is_take_profit_order(&self) -> bool {
        matches!(
            self.order_type,
            OrderType::TakeProfit | OrderType::TakeProfitLimit
        )
    }

    /// Returns true if this is a trailing stop order.
    pub fn is_trailing_stop_order(&self) -> bool {
        matches!(self.order_type, OrderType::TrailingStop)
    }
}

// ============================================================================
// Unit Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_order_request_builder_market_order() {
        let request = OrderRequest::builder()
            .symbol("BTC/USDT")
            .side(OrderSide::Buy)
            .order_type(OrderType::Market)
            .amount(Amount::new(dec!(0.1)))
            .build();

        assert_eq!(request.symbol, "BTC/USDT");
        assert_eq!(request.side, OrderSide::Buy);
        assert_eq!(request.order_type, OrderType::Market);
        assert_eq!(request.amount.as_decimal(), dec!(0.1));
        assert!(request.price.is_none());
        assert!(request.is_market_order());
    }

    #[test]
    fn test_order_request_builder_limit_order() {
        let request = OrderRequest::builder()
            .symbol("ETH/USDT")
            .side(OrderSide::Sell)
            .order_type(OrderType::Limit)
            .amount(Amount::new(dec!(1.5)))
            .price(Price::new(dec!(3000)))
            .build();

        assert_eq!(request.symbol, "ETH/USDT");
        assert_eq!(request.side, OrderSide::Sell);
        assert_eq!(request.order_type, OrderType::Limit);
        assert_eq!(request.amount.as_decimal(), dec!(1.5));
        assert_eq!(request.price.unwrap().as_decimal(), dec!(3000));
        assert!(request.is_limit_order());
    }

    #[test]
    fn test_order_request_builder_with_all_optional_fields() {
        let request = OrderRequest::builder()
            .symbol("BTC/USDT")
            .side(OrderSide::Buy)
            .order_type(OrderType::Limit)
            .amount(Amount::new(dec!(0.1)))
            .price(Price::new(dec!(50000)))
            .stop_price(Price::new(dec!(49000)))
            .time_in_force(TimeInForce::GTC)
            .client_order_id("my-order-123")
            .reduce_only(false)
            .post_only(true)
            .trigger_price(Price::new(dec!(51000)))
            .take_profit_price(Price::new(dec!(55000)))
            .stop_loss_price(Price::new(dec!(45000)))
            .trailing_delta(dec!(100))
            .trailing_percent(dec!(1.5))
            .activation_price(Price::new(dec!(52000)))
            .callback_rate(dec!(0.5))
            .working_type("MARK_PRICE")
            .position_side("LONG")
            .build();

        assert_eq!(request.symbol, "BTC/USDT");
        assert_eq!(request.price.unwrap().as_decimal(), dec!(50000));
        assert_eq!(request.stop_price.unwrap().as_decimal(), dec!(49000));
        assert_eq!(request.time_in_force, Some(TimeInForce::GTC));
        assert_eq!(request.client_order_id, Some("my-order-123".to_string()));
        assert_eq!(request.reduce_only, Some(false));
        assert_eq!(request.post_only, Some(true));
        assert_eq!(request.trigger_price.unwrap().as_decimal(), dec!(51000));
        assert_eq!(request.take_profit_price.unwrap().as_decimal(), dec!(55000));
        assert_eq!(request.stop_loss_price.unwrap().as_decimal(), dec!(45000));
        assert_eq!(request.trailing_delta, Some(dec!(100)));
        assert_eq!(request.trailing_percent, Some(dec!(1.5)));
        assert_eq!(request.activation_price.unwrap().as_decimal(), dec!(52000));
        assert_eq!(request.callback_rate, Some(dec!(0.5)));
        assert_eq!(request.working_type, Some("MARK_PRICE".to_string()));
        assert_eq!(request.position_side, Some("LONG".to_string()));
    }

    #[test]
    fn test_order_request_builder_any_order_of_required_fields() {
        // Test that required fields can be set in any order
        let request1 = OrderRequest::builder()
            .symbol("BTC/USDT")
            .side(OrderSide::Buy)
            .order_type(OrderType::Market)
            .amount(Amount::new(dec!(0.1)))
            .build();

        let request2 = OrderRequest::builder()
            .amount(Amount::new(dec!(0.1)))
            .order_type(OrderType::Market)
            .side(OrderSide::Buy)
            .symbol("BTC/USDT")
            .build();

        assert_eq!(request1.symbol, request2.symbol);
        assert_eq!(request1.side, request2.side);
        assert_eq!(request1.order_type, request2.order_type);
        assert_eq!(request1.amount.as_decimal(), request2.amount.as_decimal());
    }

    #[test]
    fn test_order_request_builder_optional_fields_before_required() {
        // Test that optional fields can be set before required fields
        let request = OrderRequest::builder()
            .price(Price::new(dec!(50000)))
            .time_in_force(TimeInForce::GTC)
            .symbol("BTC/USDT")
            .side(OrderSide::Buy)
            .order_type(OrderType::Limit)
            .amount(Amount::new(dec!(0.1)))
            .build();

        assert_eq!(request.symbol, "BTC/USDT");
        assert_eq!(request.price.unwrap().as_decimal(), dec!(50000));
        assert_eq!(request.time_in_force, Some(TimeInForce::GTC));
    }

    #[test]
    fn test_order_request_builder_stop_loss_order() {
        let request = OrderRequest::builder()
            .symbol("BTC/USDT")
            .side(OrderSide::Sell)
            .order_type(OrderType::StopLoss)
            .amount(Amount::new(dec!(0.5)))
            .stop_price(Price::new(dec!(45000)))
            .build();

        assert_eq!(request.order_type, OrderType::StopLoss);
        assert_eq!(request.stop_price.unwrap().as_decimal(), dec!(45000));
        assert!(request.is_stop_order());
    }

    #[test]
    fn test_order_request_builder_take_profit_order() {
        let request = OrderRequest::builder()
            .symbol("BTC/USDT")
            .side(OrderSide::Sell)
            .order_type(OrderType::TakeProfit)
            .amount(Amount::new(dec!(0.5)))
            .trigger_price(Price::new(dec!(55000)))
            .build();

        assert_eq!(request.order_type, OrderType::TakeProfit);
        assert_eq!(request.trigger_price.unwrap().as_decimal(), dec!(55000));
        assert!(request.is_take_profit_order());
    }

    #[test]
    fn test_order_request_builder_trailing_stop_order() {
        let request = OrderRequest::builder()
            .symbol("BTC/USDT")
            .side(OrderSide::Sell)
            .order_type(OrderType::TrailingStop)
            .amount(Amount::new(dec!(0.5)))
            .callback_rate(dec!(1.0))
            .activation_price(Price::new(dec!(52000)))
            .build();

        assert_eq!(request.order_type, OrderType::TrailingStop);
        assert_eq!(request.callback_rate, Some(dec!(1.0)));
        assert!(request.is_trailing_stop_order());
    }

    #[test]
    fn test_order_request_default_optional_fields() {
        let request = OrderRequest::builder()
            .symbol("BTC/USDT")
            .side(OrderSide::Buy)
            .order_type(OrderType::Market)
            .amount(Amount::new(dec!(0.1)))
            .build();

        // All optional fields should be None by default
        assert!(request.price.is_none());
        assert!(request.stop_price.is_none());
        assert!(request.time_in_force.is_none());
        assert!(request.client_order_id.is_none());
        assert!(request.reduce_only.is_none());
        assert!(request.post_only.is_none());
        assert!(request.trigger_price.is_none());
        assert!(request.take_profit_price.is_none());
        assert!(request.stop_loss_price.is_none());
        assert!(request.trailing_delta.is_none());
        assert!(request.trailing_percent.is_none());
        assert!(request.activation_price.is_none());
        assert!(request.callback_rate.is_none());
        assert!(request.working_type.is_none());
        assert!(request.position_side.is_none());
    }

    #[test]
    fn test_order_request_is_methods() {
        let market = OrderRequest::builder()
            .symbol("BTC/USDT")
            .side(OrderSide::Buy)
            .order_type(OrderType::Market)
            .amount(Amount::new(dec!(0.1)))
            .build();
        assert!(market.is_market_order());
        assert!(!market.is_limit_order());

        let limit = OrderRequest::builder()
            .symbol("BTC/USDT")
            .side(OrderSide::Buy)
            .order_type(OrderType::Limit)
            .amount(Amount::new(dec!(0.1)))
            .build();
        assert!(limit.is_limit_order());
        assert!(!limit.is_market_order());

        let limit_maker = OrderRequest::builder()
            .symbol("BTC/USDT")
            .side(OrderSide::Buy)
            .order_type(OrderType::LimitMaker)
            .amount(Amount::new(dec!(0.1)))
            .build();
        assert!(limit_maker.is_limit_order());

        let stop_loss = OrderRequest::builder()
            .symbol("BTC/USDT")
            .side(OrderSide::Sell)
            .order_type(OrderType::StopLoss)
            .amount(Amount::new(dec!(0.1)))
            .build();
        assert!(stop_loss.is_stop_order());

        let take_profit = OrderRequest::builder()
            .symbol("BTC/USDT")
            .side(OrderSide::Sell)
            .order_type(OrderType::TakeProfit)
            .amount(Amount::new(dec!(0.1)))
            .build();
        assert!(take_profit.is_take_profit_order());

        let trailing = OrderRequest::builder()
            .symbol("BTC/USDT")
            .side(OrderSide::Sell)
            .order_type(OrderType::TrailingStop)
            .amount(Amount::new(dec!(0.1)))
            .build();
        assert!(trailing.is_trailing_stop_order());
    }

    #[test]
    fn test_order_request_serialization() {
        let request = OrderRequest::builder()
            .symbol("BTC/USDT")
            .side(OrderSide::Buy)
            .order_type(OrderType::Limit)
            .amount(Amount::new(dec!(0.1)))
            .price(Price::new(dec!(50000)))
            .build();

        // Test serialization
        let json = serde_json::to_string(&request).expect("Serialization should succeed");
        assert!(json.contains("BTC/USDT"));
        assert!(json.contains("buy"));
        assert!(json.contains("limit"));

        // Test deserialization
        let deserialized: OrderRequest =
            serde_json::from_str(&json).expect("Deserialization should succeed");
        assert_eq!(deserialized.symbol, request.symbol);
        assert_eq!(deserialized.side, request.side);
        assert_eq!(deserialized.order_type, request.order_type);
    }
}
