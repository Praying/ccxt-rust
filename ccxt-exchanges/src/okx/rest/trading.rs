//! Trading operations for OKX REST API.

use super::super::{Okx, parser};
use crate::okx::signed_request::HttpMethod;
use ccxt_core::{
    Error, ParseError, Result,
    types::{Amount, Order, OrderRequest, OrderSide, OrderType, Price, TimeInForce},
};
use tracing::warn;

impl Okx {
    /// Create a new order using the builder pattern.
    ///
    /// This is the preferred method for creating orders. It accepts an [`OrderRequest`]
    /// built using the builder pattern, which provides compile-time validation of
    /// required fields and a more ergonomic API.
    ///
    /// # Arguments
    ///
    /// * `request` - Order request built via [`OrderRequest::builder()`]
    ///
    /// # Returns
    ///
    /// Returns the created [`Order`] structure with order details.
    ///
    /// # Errors
    ///
    /// Returns an error if authentication fails, market is not found, or the API request fails.
    ///
    /// _Requirements: 2.2, 2.6_
    pub async fn create_order_v2(&self, request: OrderRequest) -> Result<Order> {
        let market = self.base().market(&request.symbol).await?;

        let path = Self::build_api_path("/trade/order");

        let mut map = serde_json::Map::new();
        map.insert(
            "instId".to_string(),
            serde_json::Value::String(market.id.clone()),
        );
        map.insert(
            "tdMode".to_string(),
            serde_json::Value::String(self.options().account_mode.clone()),
        );
        map.insert(
            "side".to_string(),
            serde_json::Value::String(match request.side {
                OrderSide::Buy => "buy".to_string(),
                OrderSide::Sell => "sell".to_string(),
            }),
        );
        map.insert(
            "ordType".to_string(),
            serde_json::Value::String(match request.order_type {
                OrderType::LimitMaker => "post_only".to_string(),
                OrderType::StopLoss
                | OrderType::StopMarket
                | OrderType::TakeProfit
                | OrderType::TrailingStop => "market".to_string(),
                _ => "limit".to_string(),
            }),
        );
        map.insert(
            "sz".to_string(),
            serde_json::Value::String(request.amount.to_string()),
        );

        if let Some(p) = request.price {
            if request.order_type == OrderType::Limit || request.order_type == OrderType::LimitMaker
            {
                map.insert("px".to_string(), serde_json::Value::String(p.to_string()));
            }
        }

        if request.time_in_force == Some(TimeInForce::PO) || request.post_only == Some(true) {
            // Already handled via ordType = "post_only" above
        }

        if let Some(client_id) = request.client_order_id {
            map.insert("clOrdId".to_string(), serde_json::Value::String(client_id));
        }

        if let Some(reduce_only) = request.reduce_only {
            map.insert(
                "reduceOnly".to_string(),
                serde_json::Value::Bool(reduce_only),
            );
        }

        if let Some(trigger) = request.trigger_price.or(request.stop_price) {
            map.insert(
                "triggerPx".to_string(),
                serde_json::Value::String(trigger.to_string()),
            );
        }

        if let Some(pos_side) = request.position_side {
            map.insert("posSide".to_string(), serde_json::Value::String(pos_side));
        }

        let body = serde_json::Value::Object(map);

        let response = self
            .signed_request(&path)
            .method(HttpMethod::Post)
            .body(body)
            .execute()
            .await?;

        let data = response
            .get("data")
            .ok_or_else(|| Error::from(ParseError::missing_field("data")))?;

        let orders = data.as_array().ok_or_else(|| {
            Error::from(ParseError::invalid_format(
                "data",
                "Expected array of orders",
            ))
        })?;

        if orders.is_empty() {
            return Err(Error::exchange("-1", "No order data returned"));
        }

        parser::parse_order(&orders[0], Some(&market))
    }

    /// Create a new order (deprecated).
    ///
    /// # Deprecated
    ///
    /// This method is deprecated. Use [`create_order_v2`](Self::create_order_v2) with
    /// [`OrderRequest::builder()`] instead for a more ergonomic API.
    ///
    /// # Arguments
    ///
    /// * `symbol` - Trading pair symbol.
    /// * `order_type` - Order type (Market, Limit).
    /// * `side` - Order side (Buy or Sell).
    /// * `amount` - Order quantity.
    /// * `price` - Optional price (required for limit orders).
    ///
    /// # Returns
    ///
    /// Returns the created [`Order`] structure with order details.
    #[deprecated(
        since = "0.2.0",
        note = "Use create_order_v2 with OrderRequest::builder() instead"
    )]
    pub async fn create_order(
        &self,
        symbol: &str,
        order_type: OrderType,
        side: OrderSide,
        amount: Amount,
        price: Option<Price>,
    ) -> Result<Order> {
        let market = self.base().market(symbol).await?;

        let path = Self::build_api_path("/trade/order");

        let mut map = serde_json::Map::new();
        map.insert(
            "instId".to_string(),
            serde_json::Value::String(market.id.clone()),
        );
        map.insert(
            "tdMode".to_string(),
            serde_json::Value::String(self.options().account_mode.clone()),
        );
        map.insert(
            "side".to_string(),
            serde_json::Value::String(match side {
                OrderSide::Buy => "buy".to_string(),
                OrderSide::Sell => "sell".to_string(),
            }),
        );
        map.insert(
            "ordType".to_string(),
            serde_json::Value::String(match order_type {
                OrderType::Market => "market".to_string(),
                OrderType::LimitMaker => "post_only".to_string(),
                _ => "limit".to_string(),
            }),
        );
        map.insert(
            "sz".to_string(),
            serde_json::Value::String(amount.to_string()),
        );

        if let Some(p) = price {
            if order_type == OrderType::Limit || order_type == OrderType::LimitMaker {
                map.insert("px".to_string(), serde_json::Value::String(p.to_string()));
            }
        }
        let body = serde_json::Value::Object(map);

        let response = self
            .signed_request(&path)
            .method(HttpMethod::Post)
            .body(body)
            .execute()
            .await?;

        let data = response
            .get("data")
            .ok_or_else(|| Error::from(ParseError::missing_field("data")))?;

        let orders = data.as_array().ok_or_else(|| {
            Error::from(ParseError::invalid_format(
                "data",
                "Expected array of orders",
            ))
        })?;

        if orders.is_empty() {
            return Err(Error::exchange("-1", "No order data returned"));
        }

        parser::parse_order(&orders[0], Some(&market))
    }

    /// Cancel an existing order.
    ///
    /// # Arguments
    ///
    /// * `id` - Order ID to cancel.
    /// * `symbol` - Trading pair symbol.
    ///
    /// # Returns
    ///
    /// Returns the canceled [`Order`] structure.
    pub async fn cancel_order(&self, id: &str, symbol: &str) -> Result<Order> {
        let market = self.base().market(symbol).await?;

        let path = Self::build_api_path("/trade/cancel-order");

        let mut map = serde_json::Map::new();
        map.insert(
            "instId".to_string(),
            serde_json::Value::String(market.id.clone()),
        );
        map.insert(
            "ordId".to_string(),
            serde_json::Value::String(id.to_string()),
        );
        let body = serde_json::Value::Object(map);

        let response = self
            .signed_request(&path)
            .method(HttpMethod::Post)
            .body(body)
            .execute()
            .await?;

        let data = response
            .get("data")
            .ok_or_else(|| Error::from(ParseError::missing_field("data")))?;

        let orders = data.as_array().ok_or_else(|| {
            Error::from(ParseError::invalid_format(
                "data",
                "Expected array of orders",
            ))
        })?;

        if orders.is_empty() {
            return Err(Error::exchange("-1", "No order data returned"));
        }

        parser::parse_order(&orders[0], Some(&market))
    }

    /// Fetch a single order by ID.
    ///
    /// # Arguments
    ///
    /// * `id` - Order ID to fetch.
    /// * `symbol` - Trading pair symbol.
    ///
    /// # Returns
    ///
    /// Returns the [`Order`] structure with current status.
    pub async fn fetch_order(&self, id: &str, symbol: &str) -> Result<Order> {
        let market = self.base().market(symbol).await?;

        let path = Self::build_api_path("/trade/order");

        let response = self
            .signed_request(&path)
            .param("instId", &market.id)
            .param("ordId", id)
            .execute()
            .await?;

        let data = response
            .get("data")
            .ok_or_else(|| Error::from(ParseError::missing_field("data")))?;

        let orders = data.as_array().ok_or_else(|| {
            Error::from(ParseError::invalid_format(
                "data",
                "Expected array of orders",
            ))
        })?;

        if orders.is_empty() {
            return Err(Error::exchange("51400", "Order not found"));
        }

        parser::parse_order(&orders[0], Some(&market))
    }

    /// Fetch open orders.
    ///
    /// # Arguments
    ///
    /// * `symbol` - Optional trading pair symbol. If None, fetches all open orders.
    /// * `since` - Optional start timestamp in milliseconds.
    /// * `limit` - Optional limit on number of orders (maximum: 100).
    ///
    /// # Returns
    ///
    /// Returns a vector of open [`Order`] structures.
    pub async fn fetch_open_orders(
        &self,
        symbol: Option<&str>,
        since: Option<i64>,
        limit: Option<u32>,
    ) -> Result<Vec<Order>> {
        let path = Self::build_api_path("/trade/orders-pending");

        let actual_limit = limit.map_or(100, |l| l.min(100));

        let market = if let Some(sym) = symbol {
            let m = self.base().market(sym).await?;
            Some(m)
        } else {
            None
        };

        let mut builder = self
            .signed_request(&path)
            .param("instType", self.get_inst_type())
            .param("limit", actual_limit);

        if let Some(ref m) = market {
            builder = builder.param("instId", &m.id);
        }

        if let Some(start_time) = since {
            builder = builder.param("begin", start_time);
        }

        let response = builder.execute().await?;

        let data = response
            .get("data")
            .ok_or_else(|| Error::from(ParseError::missing_field("data")))?;

        let orders_array = data.as_array().ok_or_else(|| {
            Error::from(ParseError::invalid_format(
                "data",
                "Expected array of orders",
            ))
        })?;

        let mut orders = Vec::new();
        for order_data in orders_array {
            match parser::parse_order(order_data, market.as_deref()) {
                Ok(order) => orders.push(order),
                Err(e) => {
                    warn!(error = %e, "Failed to parse open order");
                }
            }
        }

        Ok(orders)
    }

    /// Fetch closed orders.
    ///
    /// # Arguments
    ///
    /// * `symbol` - Optional trading pair symbol. If None, fetches all closed orders.
    /// * `since` - Optional start timestamp in milliseconds.
    /// * `limit` - Optional limit on number of orders (maximum: 100).
    ///
    /// # Returns
    ///
    /// Returns a vector of closed [`Order`] structures.
    pub async fn fetch_closed_orders(
        &self,
        symbol: Option<&str>,
        since: Option<i64>,
        limit: Option<u32>,
    ) -> Result<Vec<Order>> {
        let path = Self::build_api_path("/trade/orders-history");

        let actual_limit = limit.map_or(100, |l| l.min(100));

        let market = if let Some(sym) = symbol {
            let m = self.base().market(sym).await?;
            Some(m)
        } else {
            None
        };

        let mut builder = self
            .signed_request(&path)
            .param("instType", self.get_inst_type())
            .param("limit", actual_limit);

        if let Some(ref m) = market {
            builder = builder.param("instId", &m.id);
        }

        if let Some(start_time) = since {
            builder = builder.param("begin", start_time);
        }

        let response = builder.execute().await?;

        let data = response
            .get("data")
            .ok_or_else(|| Error::from(ParseError::missing_field("data")))?;

        let orders_array = data.as_array().ok_or_else(|| {
            Error::from(ParseError::invalid_format(
                "data",
                "Expected array of orders",
            ))
        })?;

        let mut orders = Vec::new();
        for order_data in orders_array {
            match parser::parse_order(order_data, market.as_deref()) {
                Ok(order) => orders.push(order),
                Err(e) => {
                    warn!(error = %e, "Failed to parse closed order");
                }
            }
        }

        Ok(orders)
    }
}
