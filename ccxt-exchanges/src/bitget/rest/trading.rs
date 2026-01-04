//! Trading operations for Bitget REST API.

use super::super::{Bitget, parser};
use ccxt_core::{
    Error, ParseError, Result,
    types::{Amount, Order, OrderRequest, OrderSide, OrderType, Price, TimeInForce},
};
use tracing::warn;

impl Bitget {
    /// Create a new order.
    pub async fn create_order_v2(&self, request: OrderRequest) -> Result<Order> {
        let market = self.base().market(&request.symbol).await?;

        let path = self.build_api_path("/trade/place-order");

        let mut map = serde_json::Map::new();
        map.insert(
            "symbol".to_string(),
            serde_json::Value::String(market.id.clone()),
        );
        map.insert(
            "side".to_string(),
            serde_json::Value::String(match request.side {
                OrderSide::Buy => "buy".to_string(),
                OrderSide::Sell => "sell".to_string(),
            }),
        );
        map.insert(
            "orderType".to_string(),
            serde_json::Value::String(match request.order_type {
                OrderType::LimitMaker => "limit_maker".to_string(),
                OrderType::Market
                | OrderType::StopLoss
                | OrderType::StopMarket
                | OrderType::TakeProfit
                | OrderType::TrailingStop => "market".to_string(),
                _ => "limit".to_string(),
            }),
        );
        map.insert(
            "size".to_string(),
            serde_json::Value::String(request.amount.to_string()),
        );

        let force = if let Some(tif) = request.time_in_force {
            match tif {
                TimeInForce::GTC => "gtc",
                TimeInForce::IOC => "ioc",
                TimeInForce::FOK => "fok",
                TimeInForce::PO => "post_only",
            }
        } else if request.post_only == Some(true) {
            "post_only"
        } else {
            "gtc"
        };
        map.insert(
            "force".to_string(),
            serde_json::Value::String(force.to_string()),
        );

        if let Some(p) = request.price {
            if request.order_type == OrderType::Limit || request.order_type == OrderType::LimitMaker
            {
                map.insert(
                    "price".to_string(),
                    serde_json::Value::String(p.to_string()),
                );
            }
        }

        if let Some(client_id) = request.client_order_id {
            map.insert(
                "clientOid".to_string(),
                serde_json::Value::String(client_id),
            );
        }

        if let Some(reduce_only) = request.reduce_only {
            map.insert(
                "reduceOnly".to_string(),
                serde_json::Value::Bool(reduce_only),
            );
        }

        if let Some(trigger) = request.trigger_price.or(request.stop_price) {
            map.insert(
                "triggerPrice".to_string(),
                serde_json::Value::String(trigger.to_string()),
            );
        }

        if let Some(tp) = request.take_profit_price {
            map.insert(
                "presetTakeProfitPrice".to_string(),
                serde_json::Value::String(tp.to_string()),
            );
        }

        if let Some(sl) = request.stop_loss_price {
            map.insert(
                "presetStopLossPrice".to_string(),
                serde_json::Value::String(sl.to_string()),
            );
        }

        let body = serde_json::Value::Object(map);

        let response = self
            .signed_request(&path)
            .method(crate::bitget::signed_request::HttpMethod::Post)
            .body(body)
            .execute()
            .await?;

        let data = response
            .get("data")
            .ok_or_else(|| Error::from(ParseError::missing_field("data")))?;

        parser::parse_order(data, Some(&market))
    }

    /// Create a new order (deprecated).
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

        let path = self.build_api_path("/trade/place-order");

        let mut map = serde_json::Map::new();
        map.insert(
            "symbol".to_string(),
            serde_json::Value::String(market.id.clone()),
        );
        map.insert(
            "side".to_string(),
            serde_json::Value::String(match side {
                OrderSide::Buy => "buy".to_string(),
                OrderSide::Sell => "sell".to_string(),
            }),
        );
        map.insert(
            "orderType".to_string(),
            serde_json::Value::String(match order_type {
                OrderType::Market => "market".to_string(),
                OrderType::LimitMaker => "limit_maker".to_string(),
                _ => "limit".to_string(),
            }),
        );
        map.insert(
            "size".to_string(),
            serde_json::Value::String(amount.to_string()),
        );
        map.insert(
            "force".to_string(),
            serde_json::Value::String("gtc".to_string()),
        );

        if let Some(p) = price {
            if order_type == OrderType::Limit || order_type == OrderType::LimitMaker {
                map.insert(
                    "price".to_string(),
                    serde_json::Value::String(p.to_string()),
                );
            }
        }
        let body = serde_json::Value::Object(map);

        let response = self
            .signed_request(&path)
            .method(crate::bitget::signed_request::HttpMethod::Post)
            .body(body)
            .execute()
            .await?;

        let data = response
            .get("data")
            .ok_or_else(|| Error::from(ParseError::missing_field("data")))?;

        parser::parse_order(data, Some(&market))
    }

    /// Cancel an existing order.
    pub async fn cancel_order(&self, id: &str, symbol: &str) -> Result<Order> {
        let market = self.base().market(symbol).await?;

        let path = self.build_api_path("/trade/cancel-order");

        let mut map = serde_json::Map::new();
        map.insert(
            "symbol".to_string(),
            serde_json::Value::String(market.id.clone()),
        );
        map.insert(
            "orderId".to_string(),
            serde_json::Value::String(id.to_string()),
        );
        let body = serde_json::Value::Object(map);

        let response = self
            .signed_request(&path)
            .method(crate::bitget::signed_request::HttpMethod::Post)
            .body(body)
            .execute()
            .await?;

        let data = response
            .get("data")
            .ok_or_else(|| Error::from(ParseError::missing_field("data")))?;

        parser::parse_order(data, Some(&market))
    }

    /// Fetch a single order by ID.
    pub async fn fetch_order(&self, id: &str, symbol: &str) -> Result<Order> {
        let market = self.base().market(symbol).await?;

        let path = self.build_api_path("/trade/orderInfo");

        let response = self
            .signed_request(&path)
            .param("symbol", &market.id)
            .param("orderId", id)
            .execute()
            .await?;

        let data = response
            .get("data")
            .ok_or_else(|| Error::from(ParseError::missing_field("data")))?;

        let order_data = if data.is_array() {
            data.as_array()
                .and_then(|arr| arr.first())
                .ok_or_else(|| Error::exchange("40007", "Order not found"))?
        } else {
            data
        };

        parser::parse_order(order_data, Some(&market))
    }

    /// Fetch open orders.
    pub async fn fetch_open_orders(
        &self,
        symbol: Option<&str>,
        since: Option<i64>,
        limit: Option<u32>,
    ) -> Result<Vec<Order>> {
        let path = self.build_api_path("/trade/unfilled-orders");

        let market = if let Some(sym) = symbol {
            Some(self.base().market(sym).await?)
        } else {
            None
        };

        let actual_limit = limit.map_or(100, |l| l.min(500));

        let mut builder = self.signed_request(&path).param("limit", actual_limit);

        if let Some(m) = &market {
            builder = builder.param("symbol", &m.id);
        }

        if let Some(start_time) = since {
            builder = builder.param("startTime", start_time);
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
    pub async fn fetch_closed_orders(
        &self,
        symbol: Option<&str>,
        since: Option<i64>,
        limit: Option<u32>,
    ) -> Result<Vec<Order>> {
        let path = self.build_api_path("/trade/history-orders");

        let market = if let Some(sym) = symbol {
            Some(self.base().market(sym).await?)
        } else {
            None
        };

        let actual_limit = limit.map_or(100, |l| l.min(500));

        let mut builder = self.signed_request(&path).param("limit", actual_limit);

        if let Some(m) = &market {
            builder = builder.param("symbol", &m.id);
        }

        if let Some(start_time) = since {
            builder = builder.param("startTime", start_time);
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
