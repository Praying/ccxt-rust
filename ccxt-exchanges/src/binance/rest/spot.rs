//! Binance spot trading operations.
//!
//! This module contains all spot trading methods including order creation,
//! cancellation, and order management.

use super::super::{Binance, constants::endpoints, parser, signed_request::HttpMethod};
use ccxt_core::{
    Error, ParseError, Result,
    types::{Amount, Order, OrderSide, OrderType, Price},
};
use rust_decimal::Decimal;
use std::collections::{BTreeMap, HashMap};
use tracing::warn;

impl Binance {
    /// Create a new order.
    ///
    /// # Arguments
    ///
    /// * `symbol` - Trading pair symbol.
    /// * `order_type` - Order type (Market, Limit, StopLoss, etc.).
    /// * `side` - Order side (Buy or Sell).
    /// * `amount` - Order quantity as [`Amount`] type.
    /// * `price` - Optional price as [`Price`] type (required for limit orders).
    /// * `params` - Additional parameters.
    ///
    /// # Returns
    ///
    /// Returns the created [`Order`] structure with order details.
    ///
    /// # Errors
    ///
    /// Returns an error if authentication fails, market is not found, or the API request fails.
    pub async fn create_order(
        &self,
        symbol: &str,
        order_type: OrderType,
        side: OrderSide,
        amount: Amount,
        price: Option<Price>,
        params: Option<HashMap<String, String>>,
    ) -> Result<Order> {
        let market = self.base().market(symbol).await?;
        let mut request_params = BTreeMap::new();

        request_params.insert("symbol".to_string(), market.id.clone());
        request_params.insert(
            "side".to_string(),
            match side {
                OrderSide::Buy => "BUY".to_string(),
                OrderSide::Sell => "SELL".to_string(),
            },
        );
        request_params.insert(
            "type".to_string(),
            match order_type {
                OrderType::Market => "MARKET".to_string(),
                OrderType::Limit => "LIMIT".to_string(),
                OrderType::StopLoss => "STOP_LOSS".to_string(),
                OrderType::StopLossLimit => "STOP_LOSS_LIMIT".to_string(),
                OrderType::TakeProfit => "TAKE_PROFIT".to_string(),
                OrderType::TakeProfitLimit => "TAKE_PROFIT_LIMIT".to_string(),
                OrderType::LimitMaker => "LIMIT_MAKER".to_string(),
                OrderType::StopMarket => "STOP_MARKET".to_string(),
                OrderType::StopLimit => "STOP_LIMIT".to_string(),
                OrderType::TrailingStop => "TRAILING_STOP_MARKET".to_string(),
            },
        );
        request_params.insert("quantity".to_string(), amount.to_string());

        if let Some(p) = price {
            request_params.insert("price".to_string(), p.to_string());
        }

        // Limit orders require timeInForce parameter
        if order_type == OrderType::Limit
            || order_type == OrderType::StopLossLimit
            || order_type == OrderType::TakeProfitLimit
        {
            if !request_params.contains_key("timeInForce") {
                request_params.insert("timeInForce".to_string(), "GTC".to_string());
            }
        }

        if let Some(extra) = params {
            for (k, v) in extra {
                request_params.insert(k, v);
            }
        }

        // Handle cost parameter for market buy orders (quoteOrderQty)
        if order_type == OrderType::Market && side == OrderSide::Buy {
            if let Some(cost_str) = request_params.get("cost") {
                request_params.insert("quoteOrderQty".to_string(), cost_str.clone());
                request_params.remove("quantity");
                request_params.remove("cost");
            }
        }

        // Handle conditional order parameters
        if matches!(
            order_type,
            OrderType::StopLoss
                | OrderType::StopLossLimit
                | OrderType::TakeProfit
                | OrderType::TakeProfitLimit
                | OrderType::StopMarket
        ) {
            if !request_params.contains_key("stopPrice") {
                if let Some(stop_loss) = request_params.get("stopLossPrice") {
                    request_params.insert("stopPrice".to_string(), stop_loss.clone());
                } else if let Some(take_profit) = request_params.get("takeProfitPrice") {
                    request_params.insert("stopPrice".to_string(), take_profit.clone());
                }
            }
        }

        // Trailing stop handling for spot market
        if order_type == OrderType::TrailingStop {
            if market.is_spot() {
                if !request_params.contains_key("trailingDelta") {
                    if let Some(percent_str) = request_params.get("trailingPercent") {
                        if let Ok(percent) = percent_str.parse::<Decimal>() {
                            // Convert percentage to basis points (e.g., 2.0% -> 200)
                            let delta = (percent * Decimal::from(100)).to_string();
                            // Parse as integer for the API
                            if let Ok(delta_int) = delta.parse::<i64>() {
                                request_params
                                    .insert("trailingDelta".to_string(), delta_int.to_string());
                                request_params.remove("trailingPercent");
                            }
                        }
                    }
                }
            }
        }

        let url = format!("{}{}", self.urls().private, endpoints::ORDER);
        let data = self
            .signed_request(url)
            .method(HttpMethod::Post)
            .params(request_params)
            .execute()
            .await?;

        parser::parse_order(&data, Some(&market))
    }

    /// Cancel an order.
    ///
    /// # Arguments
    ///
    /// * `id` - Order ID.
    /// * `symbol` - Trading pair symbol.
    ///
    /// # Returns
    ///
    /// Returns the cancelled [`Order`] information.
    ///
    /// # Errors
    ///
    /// Returns an error if authentication fails, market is not found, or the API request fails.
    pub async fn cancel_order(&self, id: &str, symbol: &str) -> Result<Order> {
        let market = self.base().market(symbol).await?;
        let url = format!("{}{}", self.urls().private, endpoints::ORDER);

        let data = self
            .signed_request(url)
            .method(HttpMethod::Delete)
            .param("symbol", &market.id)
            .param("orderId", id)
            .execute()
            .await?;

        parser::parse_order(&data, Some(&market))
    }

    /// Fetch order details.
    ///
    /// # Arguments
    ///
    /// * `id` - Order ID.
    /// * `symbol` - Trading pair symbol.
    ///
    /// # Returns
    ///
    /// Returns the [`Order`] information.
    ///
    /// # Errors
    ///
    /// Returns an error if authentication fails, market is not found, or the API request fails.
    pub async fn fetch_order(&self, id: &str, symbol: &str) -> Result<Order> {
        let market = self.base().market(symbol).await?;
        let url = format!("{}{}", self.urls().private, endpoints::ORDER);

        let data = self
            .signed_request(url)
            .param("symbol", &market.id)
            .param("orderId", id)
            .execute()
            .await?;

        parser::parse_order(&data, Some(&market))
    }

    /// Fetch open (unfilled) orders.
    ///
    /// # Arguments
    ///
    /// * `symbol` - Optional trading pair symbol. If `None`, fetches all open orders.
    ///
    /// # Returns
    ///
    /// Returns a vector of open [`Order`] structures.
    ///
    /// # Errors
    ///
    /// Returns an error if authentication fails or the API request fails.
    pub async fn fetch_open_orders(&self, symbol: Option<&str>) -> Result<Vec<Order>> {
        let market = if let Some(sym) = symbol {
            Some(self.base().market(sym).await?)
        } else {
            None
        };

        let url = format!("{}{}", self.urls().private, endpoints::OPEN_ORDERS);

        let data = self
            .signed_request(url)
            .optional_param("symbol", market.as_ref().map(|m| &m.id))
            .execute()
            .await?;

        let orders_array = data.as_array().ok_or_else(|| {
            Error::from(ParseError::invalid_format(
                "data",
                "Expected array of orders",
            ))
        })?;

        let mut orders = Vec::new();
        for order_data in orders_array {
            match parser::parse_order(order_data, market.as_ref().map(|v| &**v)) {
                Ok(order) => orders.push(order),
                Err(e) => {
                    warn!(error = %e, "Failed to parse order");
                }
            }
        }

        Ok(orders)
    }

    /// Fetch closed (completed) orders.
    ///
    /// # Arguments
    ///
    /// * `symbol` - Optional trading pair symbol.
    /// * `since` - Optional start timestamp (milliseconds).
    /// * `limit` - Optional limit on number of orders (default 500, max 1000).
    ///
    /// # Returns
    ///
    /// Returns a vector of closed [`Order`] structures.
    ///
    /// # Errors
    ///
    /// Returns an error if authentication fails or the API request fails.
    pub async fn fetch_closed_orders(
        &self,
        symbol: Option<&str>,
        since: Option<i64>,
        limit: Option<u32>,
    ) -> Result<Vec<Order>> {
        let all_orders = self.fetch_orders(symbol, since, None).await?;

        let mut closed_orders: Vec<Order> = all_orders
            .into_iter()
            .filter(|order| order.status == ccxt_core::types::OrderStatus::Closed)
            .collect();

        if let Some(l) = limit {
            closed_orders.truncate(l as usize);
        }

        Ok(closed_orders)
    }

    /// Cancel all open orders.
    ///
    /// # Arguments
    ///
    /// * `symbol` - Trading pair symbol.
    ///
    /// # Returns
    ///
    /// Returns a vector of cancelled [`Order`] structures.
    ///
    /// # Errors
    ///
    /// Returns an error if authentication fails, market is not found, or the API request fails.
    pub async fn cancel_all_orders(&self, symbol: &str) -> Result<Vec<Order>> {
        let market = self.base().market(symbol).await?;
        let url = format!("{}{}", self.urls().private, endpoints::OPEN_ORDERS);

        let data = self
            .signed_request(url)
            .method(HttpMethod::Delete)
            .param("symbol", &market.id)
            .execute()
            .await?;

        let orders_array = data.as_array().ok_or_else(|| {
            Error::from(ParseError::invalid_format(
                "data",
                "Expected array of orders",
            ))
        })?;

        let mut orders = Vec::new();
        for order_data in orders_array {
            match parser::parse_order(order_data, Some(&market)) {
                Ok(order) => orders.push(order),
                Err(e) => {
                    warn!(error = %e, "Failed to parse order");
                }
            }
        }

        Ok(orders)
    }

    /// Cancel multiple orders.
    ///
    /// # Arguments
    ///
    /// * `ids` - Vector of order IDs to cancel.
    /// * `symbol` - Trading pair symbol.
    ///
    /// # Returns
    ///
    /// Returns a vector of cancelled [`Order`] structures.
    ///
    /// # Errors
    ///
    /// Returns an error if authentication fails, market is not found, or the API request fails.
    pub async fn cancel_orders(&self, ids: Vec<String>, symbol: &str) -> Result<Vec<Order>> {
        let market = self.base().market(symbol).await?;

        let order_ids_json = serde_json::to_string(&ids).map_err(|e| {
            Error::from(ParseError::invalid_format(
                "data",
                format!("Failed to serialize order IDs: {}", e),
            ))
        })?;

        let url = format!("{}{}", self.urls().private, endpoints::OPEN_ORDERS);

        let data = self
            .signed_request(url)
            .method(HttpMethod::Delete)
            .param("symbol", &market.id)
            .param("orderIdList", order_ids_json)
            .execute()
            .await?;

        let orders_array = data.as_array().ok_or_else(|| {
            Error::from(ParseError::invalid_format(
                "data",
                "Expected array of orders",
            ))
        })?;

        let mut orders = Vec::new();
        for order_data in orders_array {
            match parser::parse_order(order_data, Some(&market)) {
                Ok(order) => orders.push(order),
                Err(e) => {
                    warn!(error = %e, "Failed to parse order");
                }
            }
        }

        Ok(orders)
    }

    /// Fetch all orders (historical and current).
    ///
    /// # Arguments
    ///
    /// * `symbol` - Optional trading pair symbol.
    /// * `since` - Optional start timestamp (milliseconds).
    /// * `limit` - Optional limit on number of orders (default 500, max 1000).
    ///
    /// # Returns
    ///
    /// Returns a vector of [`Order`] structures.
    ///
    /// # Errors
    ///
    /// Returns an error if authentication fails or the API request fails.
    pub async fn fetch_orders(
        &self,
        symbol: Option<&str>,
        since: Option<i64>,
        limit: Option<u32>,
    ) -> Result<Vec<Order>> {
        let market = if let Some(sym) = symbol {
            Some(self.base().market(sym).await?)
        } else {
            None
        };

        let url = format!("{}{}", self.urls().private, endpoints::ALL_ORDERS);

        let data = self
            .signed_request(url)
            .optional_param("symbol", market.as_ref().map(|m| &m.id))
            .optional_param("startTime", since)
            .optional_param("limit", limit)
            .execute()
            .await?;

        let orders_array = data.as_array().ok_or_else(|| {
            Error::from(ParseError::invalid_format(
                "data",
                "Expected array of orders",
            ))
        })?;

        let mut orders = Vec::new();
        for order_data in orders_array {
            match parser::parse_order(order_data, market.as_ref().map(|v| &**v)) {
                Ok(order) => orders.push(order),
                Err(e) => {
                    warn!(error = %e, "Failed to parse order");
                }
            }
        }

        Ok(orders)
    }

    /// Create a stop-loss order.
    ///
    /// # Arguments
    ///
    /// * `symbol` - Trading pair symbol.
    /// * `side` - Order side (Buy/Sell).
    /// * `amount` - Order quantity as [`Amount`] type.
    /// * `stop_price` - Stop-loss trigger price as [`Price`] type.
    /// * `price` - Optional limit price as [`Price`] type (if `None`, creates market stop-loss order).
    /// * `params` - Optional additional parameters.
    ///
    /// # Returns
    ///
    /// Returns the created stop-loss [`Order`].
    ///
    /// # Errors
    ///
    /// Returns an error if authentication fails or the API request fails.
    pub async fn create_stop_loss_order(
        &self,
        symbol: &str,
        side: OrderSide,
        amount: Amount,
        stop_price: Price,
        price: Option<Price>,
        params: Option<HashMap<String, String>>,
    ) -> Result<Order> {
        let mut request_params = params.unwrap_or_default();

        request_params.insert("stopPrice".to_string(), stop_price.to_string());

        let order_type = if price.is_some() {
            OrderType::StopLossLimit
        } else {
            OrderType::StopLoss
        };

        self.create_order(
            symbol,
            order_type,
            side,
            amount,
            price,
            Some(request_params),
        )
        .await
    }

    /// Create a take-profit order.
    ///
    /// # Arguments
    ///
    /// * `symbol` - Trading pair symbol.
    /// * `side` - Order side (Buy/Sell).
    /// * `amount` - Order quantity as [`Amount`] type.
    /// * `take_profit_price` - Take-profit trigger price as [`Price`] type.
    /// * `price` - Optional limit price as [`Price`] type (if `None`, creates market take-profit order).
    /// * `params` - Optional additional parameters.
    ///
    /// # Returns
    ///
    /// Returns the created take-profit [`Order`].
    ///
    /// # Errors
    ///
    /// Returns an error if authentication fails or the API request fails.
    pub async fn create_take_profit_order(
        &self,
        symbol: &str,
        side: OrderSide,
        amount: Amount,
        take_profit_price: Price,
        price: Option<Price>,
        params: Option<HashMap<String, String>>,
    ) -> Result<Order> {
        let mut request_params = params.unwrap_or_default();

        request_params.insert("stopPrice".to_string(), take_profit_price.to_string());

        let order_type = if price.is_some() {
            OrderType::TakeProfitLimit
        } else {
            OrderType::TakeProfit
        };

        self.create_order(
            symbol,
            order_type,
            side,
            amount,
            price,
            Some(request_params),
        )
        .await
    }

    /// Create a trailing stop order.
    ///
    /// # Arguments
    ///
    /// * `symbol` - Trading pair symbol.
    /// * `side` - Order side (Buy/Sell).
    /// * `amount` - Order quantity as [`Amount`] type.
    /// * `trailing_percent` - Trailing percentage as [`Decimal`] (e.g., 2.0 for 2%).
    /// * `activation_price` - Optional activation price as [`Price`] type (not supported for spot markets).
    /// * `params` - Optional additional parameters.
    ///
    /// # Returns
    ///
    /// Returns the created trailing stop [`Order`].
    ///
    /// # Errors
    ///
    /// Returns an error if authentication fails or the API request fails.
    pub async fn create_trailing_stop_order(
        &self,
        symbol: &str,
        side: OrderSide,
        amount: Amount,
        trailing_percent: Decimal,
        activation_price: Option<Price>,
        params: Option<HashMap<String, String>>,
    ) -> Result<Order> {
        let mut request_params = params.unwrap_or_default();

        request_params.insert("trailingPercent".to_string(), trailing_percent.to_string());

        if let Some(activation) = activation_price {
            request_params.insert("activationPrice".to_string(), activation.to_string());
        }

        self.create_order(
            symbol,
            OrderType::TrailingStop,
            side,
            amount,
            None,
            Some(request_params),
        )
        .await
    }
}
