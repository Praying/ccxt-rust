//! HyperLiquid REST API implementation.
//!
//! Implements all REST API endpoint operations for the HyperLiquid exchange.

use super::{HyperLiquid, error, parser};
use ccxt_core::{
    Error, ParseError, Result,
    types::{
        Amount, Balance, Market, Order, OrderBook, OrderSide, OrderType, Price, Ticker, Trade,
    },
};
use rust_decimal::Decimal;
use serde_json::{Map, Value};
use std::{collections::HashMap, sync::Arc};
use tracing::{debug, info, warn};

impl HyperLiquid {
    // ============================================================================
    // Helper Methods
    // ============================================================================

    /// Make a public info API request.
    pub(crate) async fn info_request(&self, request_type: &str, payload: Value) -> Result<Value> {
        let urls = self.urls();
        let url = format!("{}/info", urls.rest);

        // Build the request body by merging type with payload
        let body = if let Value::Object(map) = payload {
            let mut obj = serde_json::Map::new();
            obj.insert("type".to_string(), Value::String(request_type.to_string()));
            for (k, v) in map {
                obj.insert(k, v);
            }
            Value::Object(obj)
        } else {
            let mut map = serde_json::Map::new();
            map.insert(
                "type".to_string(),
                serde_json::Value::String(request_type.to_string()),
            );
            serde_json::Value::Object(map)
        };

        debug!("HyperLiquid info request: {} {:?}", request_type, body);

        let response = self.base().http_client.post(&url, None, Some(body)).await?;

        if error::is_error_response(&response) {
            return Err(error::parse_error(&response));
        }

        Ok(response)
    }

    /// Make an exchange action request (requires authentication).
    ///
    /// # Deprecated
    ///
    /// This method is deprecated. Use `signed_action()` builder instead:
    ///
    /// ```no_run
    /// # use ccxt_exchanges::hyperliquid::HyperLiquid;
    /// # use serde_json::json;
    /// # async fn example() -> ccxt_core::Result<()> {
    /// let exchange = HyperLiquid::builder()
    ///     .private_key("0x...")
    ///     .testnet(true)
    ///     .build()?;
    ///
    /// let action = json!({"type": "order", "orders": [], "grouping": "na"});
    /// let response = exchange.signed_action(action).execute().await?;
    /// # Ok(())
    /// # }
    /// ```
    #[deprecated(since = "0.2.0", note = "Use signed_action() builder instead")]
    pub async fn exchange_request(&self, action: Value, nonce: u64) -> Result<Value> {
        self.signed_action(action).nonce(nonce).execute().await
    }

    /// Get current timestamp as nonce.
    ///
    /// # Deprecated
    ///
    /// This method is deprecated. The `signed_action()` builder automatically
    /// generates the nonce. Use `signed_action().nonce(value)` if you need
    /// to override the auto-generated nonce.
    #[deprecated(
        since = "0.2.0",
        note = "Use signed_action() builder which auto-generates nonce"
    )]
    pub fn get_nonce(&self) -> u64 {
        chrono::Utc::now().timestamp_millis() as u64
    }

    // ============================================================================
    // Public API Methods - Market Data
    // ============================================================================

    /// Fetch all trading markets.
    pub async fn fetch_markets(&self) -> Result<Arc<HashMap<String, Arc<Market>>>> {
        let response = self
            .info_request("meta", serde_json::Value::Object(serde_json::Map::new()))
            .await?;

        let universe = response["universe"]
            .as_array()
            .ok_or_else(|| Error::from(ParseError::missing_field("universe")))?;

        let mut markets = Vec::new();
        for (index, asset) in universe.iter().enumerate() {
            match parser::parse_market(asset, index) {
                Ok(market) => markets.push(market),
                Err(e) => {
                    warn!(error = %e, "Failed to parse market");
                }
            }
        }

        // Cache the markets and preserve ownership for the caller
        let result = self.base().set_markets(markets, None).await?;

        info!("Loaded {} markets for HyperLiquid", result.len());
        Ok(result)
    }

    /// Load and cache market data.
    pub async fn load_markets(&self, reload: bool) -> Result<Arc<HashMap<String, Arc<Market>>>> {
        // Acquire the loading lock to serialize concurrent load_markets calls
        // This prevents multiple tasks from making duplicate API calls
        let _loading_guard = self.base().market_loading_lock.lock().await;

        // Check cache status while holding the lock
        {
            let cache = self.base().market_cache.read().await;
            if cache.is_loaded() && !reload {
                debug!(
                    "Returning cached markets for HyperLiquid ({} markets)",
                    cache.market_count()
                );
                return Ok(cache.markets());
            }
        }

        info!("Loading markets for HyperLiquid (reload: {})", reload);
        let _markets = self.fetch_markets().await?;

        let cache = self.base().market_cache.read().await;
        Ok(cache.markets())
    }

    /// Fetch ticker for a single trading pair.
    pub async fn fetch_ticker(&self, symbol: &str) -> Result<Ticker> {
        let market = self.base().market(symbol).await?;
        let response = self
            .info_request("allMids", serde_json::Value::Object(serde_json::Map::new()))
            .await?;

        // Response is a map of asset name to mid price
        let mid_price = response[&market.base]
            .as_str()
            .and_then(|s| s.parse::<Decimal>().ok())
            .ok_or_else(|| Error::bad_symbol(format!("No ticker data for {}", symbol)))?;

        parser::parse_ticker(symbol, mid_price, Some(&market))
    }

    /// Fetch tickers for multiple trading pairs.
    pub async fn fetch_tickers(&self, symbols: Option<Vec<String>>) -> Result<Vec<Ticker>> {
        let response = self
            .info_request("allMids", serde_json::Value::Object(serde_json::Map::new()))
            .await?;

        let cache = self.base().market_cache.read().await;
        if !cache.is_loaded() {
            drop(cache);
            return Err(Error::exchange(
                "-1",
                "Markets not loaded. Call load_markets() first.",
            ));
        }
        drop(cache);

        let mut tickers = Vec::new();

        if let Some(obj) = response.as_object() {
            for (asset, price) in obj {
                if let Some(mid_price) = price.as_str().and_then(|s| s.parse::<Decimal>().ok()) {
                    let symbol = format!("{}/USDC:USDC", asset);

                    // Filter by requested symbols if provided
                    if let Some(ref syms) = symbols {
                        if !syms.contains(&symbol) {
                            continue;
                        }
                    }

                    if let Ok(ticker) = parser::parse_ticker(&symbol, mid_price, None) {
                        tickers.push(ticker);
                    }
                }
            }
        }

        Ok(tickers)
    }

    /// Fetch order book for a trading pair.
    pub async fn fetch_order_book(&self, symbol: &str, _limit: Option<u32>) -> Result<OrderBook> {
        let market = self.base().market(symbol).await?;

        let response = self
            .info_request("l2Book", {
                let mut map = serde_json::Map::new();
                map.insert(
                    "coin".to_string(),
                    serde_json::Value::String(market.base.clone()),
                );
                serde_json::Value::Object(map)
            })
            .await?;

        parser::parse_orderbook(&response, symbol.to_string())
    }

    /// Fetch recent public trades.
    pub async fn fetch_trades(&self, symbol: &str, limit: Option<u32>) -> Result<Vec<Trade>> {
        let market = self.base().market(symbol).await?;
        let limit = limit.unwrap_or(100).min(1000);

        let response = self
            .info_request("recentTrades", {
                let mut map = serde_json::Map::new();
                map.insert(
                    "coin".to_string(),
                    serde_json::Value::String(market.base.clone()),
                );
                map.insert("n".to_string(), serde_json::Value::Number(limit.into()));
                serde_json::Value::Object(map)
            })
            .await?;

        let trades_array = response
            .as_array()
            .ok_or_else(|| Error::from(ParseError::invalid_format("data", "Expected array")))?;

        let mut trades = Vec::new();
        for trade_data in trades_array {
            match parser::parse_trade(trade_data, Some(&market)) {
                Ok(trade) => trades.push(trade),
                Err(e) => {
                    warn!(error = %e, "Failed to parse trade");
                }
            }
        }

        Ok(trades)
    }

    /// Fetch OHLCV (candlestick) data.
    pub async fn fetch_ohlcv(
        &self,
        symbol: &str,
        timeframe: &str,
        since: Option<i64>,
        limit: Option<u32>,
    ) -> Result<Vec<ccxt_core::types::Ohlcv>> {
        let market = self.base().market(symbol).await?;
        let limit = limit.unwrap_or(500).min(5000) as i64;

        // Convert timeframe to HyperLiquid interval format
        let interval = match timeframe {
            "1m" => "1m",
            "5m" => "5m",
            "15m" => "15m",
            "30m" => "30m",
            "4h" => "4h",
            "1d" => "1d",
            "1w" => "1w",
            _ => "1h",
        };

        let now = chrono::Utc::now().timestamp_millis() as u64;
        let start_time = since.map_or(now - limit as u64 * 3600000, |s| s as u64); // Default to limit hours ago
        let end_time = now;

        let response = self
            .info_request("candleSnapshot", {
                let mut map = serde_json::Map::new();
                map.insert(
                    "coin".to_string(),
                    serde_json::Value::String(market.base.clone()),
                );
                map.insert(
                    "interval".to_string(),
                    serde_json::Value::String(interval.to_string()),
                );
                map.insert(
                    "startTime".to_string(),
                    serde_json::Value::Number(start_time.into()),
                );
                map.insert(
                    "endTime".to_string(),
                    serde_json::Value::Number(end_time.into()),
                );
                serde_json::Value::Object(map)
            })
            .await?;

        let candles_array = response
            .as_array()
            .ok_or_else(|| Error::from(ParseError::invalid_format("data", "Expected array")))?;

        let mut ohlcv_list = Vec::new();
        for candle_data in candles_array {
            match parser::parse_ohlcv(candle_data) {
                Ok(ohlcv) => ohlcv_list.push(ohlcv),
                Err(e) => {
                    warn!(error = %e, "Failed to parse OHLCV");
                }
            }
        }

        Ok(ohlcv_list)
    }

    /// Fetch current funding rate for a symbol.
    pub async fn fetch_funding_rate(&self, symbol: &str) -> Result<ccxt_core::types::FundingRate> {
        let market = self.base().market(symbol).await?;

        let response = self
            .info_request("meta", serde_json::Value::Object(serde_json::Map::new()))
            .await?;

        // Find the asset in the universe
        let universe = response["universe"]
            .as_array()
            .ok_or_else(|| Error::from(ParseError::missing_field("universe")))?;

        let asset_index: usize = market.id.parse().unwrap_or(0);
        let asset_data = universe
            .get(asset_index)
            .ok_or_else(|| Error::bad_symbol(format!("Asset not found: {}", symbol)))?;

        let funding_rate = asset_data["funding"]
            .as_str()
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(0.0);

        let timestamp = chrono::Utc::now().timestamp_millis();

        Ok(ccxt_core::types::FundingRate {
            info: serde_json::json!({}),
            symbol: symbol.to_string(),
            mark_price: None,
            index_price: None,
            interest_rate: None,
            estimated_settle_price: None,
            funding_rate: Some(funding_rate),
            funding_timestamp: None,
            funding_datetime: None,
            previous_funding_rate: None,
            previous_funding_timestamp: None,
            previous_funding_datetime: None,
            timestamp: Some(timestamp),
            datetime: parser::timestamp_to_datetime(timestamp),
        })
    }

    // ============================================================================
    // Private API Methods - Account
    // ============================================================================

    /// Fetch account balance.
    pub async fn fetch_balance(&self) -> Result<Balance> {
        let address = self
            .wallet_address()
            .ok_or_else(|| Error::authentication("Private key required to fetch balance"))?;

        let response = self
            .info_request("clearinghouseState", {
                let mut map = Map::new();
                map.insert("user".to_string(), Value::String(address.to_string()));
                Value::Object(map)
            })
            .await?;

        parser::parse_balance(&response)
    }

    /// Fetch open positions.
    pub async fn fetch_positions(
        &self,
        symbols: Option<Vec<String>>,
    ) -> Result<Vec<ccxt_core::types::Position>> {
        let address = self
            .wallet_address()
            .ok_or_else(|| Error::authentication("Private key required to fetch positions"))?;

        let response = self
            .info_request("clearinghouseState", {
                let mut map = Map::new();
                map.insert("user".to_string(), Value::String(address.to_string()));
                Value::Object(map)
            })
            .await?;

        let asset_positions = response["assetPositions"]
            .as_array()
            .ok_or_else(|| Error::from(ParseError::missing_field("assetPositions")))?;

        let mut positions = Vec::new();
        for pos_data in asset_positions {
            let position = pos_data.get("position").unwrap_or(pos_data);

            let coin = position["coin"].as_str().unwrap_or("");
            let symbol = format!("{}/USDC:USDC", coin);

            // Filter by symbols if provided
            if let Some(ref syms) = symbols {
                if !syms.contains(&symbol) {
                    continue;
                }
            }

            let szi = position["szi"]
                .as_str()
                .and_then(|s| s.parse::<f64>().ok())
                .unwrap_or(0.0);

            // Skip zero positions
            if szi.abs() < 1e-10 {
                continue;
            }

            let entry_px = position["entryPx"]
                .as_str()
                .and_then(|s| s.parse::<f64>().ok());
            let liquidation_px = position["liquidationPx"]
                .as_str()
                .and_then(|s| s.parse::<f64>().ok());
            let unrealized_pnl = position["unrealizedPnl"]
                .as_str()
                .and_then(|s| s.parse::<f64>().ok());
            let margin_used = position["marginUsed"]
                .as_str()
                .and_then(|s| s.parse::<f64>().ok());

            let leverage_info = position.get("leverage");
            let leverage = leverage_info
                .and_then(|l| l["value"].as_str())
                .and_then(|s| s.parse::<f64>().ok());
            let margin_mode = leverage_info
                .and_then(|l| l["type"].as_str())
                .map(|t| if t == "cross" { "cross" } else { "isolated" }.to_string());

            let side = if szi > 0.0 { "long" } else { "short" };

            positions.push(ccxt_core::types::Position {
                info: pos_data.clone(),
                id: None,
                symbol,
                side: Some(side.to_string()),
                position_side: None,
                dual_side_position: None,
                contracts: Some(szi.abs()),
                contract_size: Some(1.0),
                entry_price: entry_px,
                mark_price: None,
                notional: None,
                leverage,
                collateral: margin_used,
                initial_margin: margin_used,
                initial_margin_percentage: None,
                maintenance_margin: None,
                maintenance_margin_percentage: None,
                unrealized_pnl,
                realized_pnl: None,
                liquidation_price: liquidation_px,
                margin_ratio: None,
                margin_mode,
                hedged: None,
                percentage: None,
                timestamp: Some(chrono::Utc::now().timestamp_millis()),
                datetime: None,
            });
        }

        Ok(positions)
    }

    /// Fetch open orders.
    pub async fn fetch_open_orders(
        &self,
        symbol: Option<&str>,
        _since: Option<i64>,
        _limit: Option<u32>,
    ) -> Result<Vec<Order>> {
        let address = self
            .wallet_address()
            .ok_or_else(|| Error::authentication("Private key required to fetch orders"))?;

        let response = self
            .info_request("openOrders", {
                let mut map = Map::new();
                map.insert("user".to_string(), Value::String(address.to_string()));
                Value::Object(map)
            })
            .await?;

        let orders_array = response
            .as_array()
            .ok_or_else(|| Error::from(ParseError::invalid_format("data", "Expected array")))?;

        let mut orders = Vec::new();
        for order_data in orders_array {
            match parser::parse_order(order_data, None) {
                Ok(order) => {
                    // Filter by symbol if provided
                    if let Some(sym) = symbol {
                        if order.symbol != sym {
                            continue;
                        }
                    }
                    orders.push(order);
                }
                Err(e) => {
                    warn!(error = %e, "Failed to parse order");
                }
            }
        }

        Ok(orders)
    }

    // ============================================================================
    // Private API Methods - Order Management
    // ============================================================================

    /// Create a new order.
    ///
    /// # Arguments
    ///
    /// * `symbol` - Trading pair symbol.
    /// * `order_type` - Order type (Market, Limit, etc.).
    /// * `side` - Order side (Buy or Sell).
    /// * `amount` - Order quantity as [`Amount`] type.
    /// * `price` - Optional price as [`Price`] type (required for limit orders).
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
    ) -> Result<Order> {
        let market = self.base().market(symbol).await?;
        let asset_index: u32 = market.id.parse().unwrap_or(0);

        let is_buy = matches!(side, OrderSide::Buy);
        let limit_px = price.map_or_else(|| "0".to_string(), |p| p.to_string());

        let order_wire = if order_type == OrderType::Market {
            let mut limit_map = Map::new();
            limit_map.insert("tif".to_string(), Value::String("Ioc".to_string()));
            let mut map = Map::new();
            map.insert("limit".to_string(), Value::Object(limit_map));
            Value::Object(map)
        } else {
            let mut limit_map = Map::new();
            limit_map.insert("tif".to_string(), Value::String("Gtc".to_string()));
            let mut map = Map::new();
            map.insert("limit".to_string(), Value::Object(limit_map));
            Value::Object(map)
        };

        let action = {
            let mut order_map = Map::new();
            order_map.insert("a".to_string(), Value::Number(asset_index.into()));
            order_map.insert("b".to_string(), Value::Bool(is_buy));
            order_map.insert("p".to_string(), Value::String(limit_px));
            order_map.insert("s".to_string(), Value::String(amount.to_string()));
            order_map.insert("r".to_string(), Value::Bool(false));
            order_map.insert("t".to_string(), order_wire);

            let mut map = Map::new();
            map.insert("type".to_string(), Value::String("order".to_string()));
            map.insert(
                "orders".to_string(),
                Value::Array(vec![Value::Object(order_map)]),
            );
            map.insert("grouping".to_string(), Value::String("na".to_string()));
            Value::Object(map)
        };

        let response = self.signed_action(action).execute().await?;

        // Parse response
        if let Some(statuses) = response["response"]["data"]["statuses"].as_array() {
            if let Some(status) = statuses.first() {
                if let Some(resting) = status.get("resting") {
                    return parser::parse_order(resting, Some(&market));
                }
                if let Some(filled) = status.get("filled") {
                    return parser::parse_order(filled, Some(&market));
                }
            }
        }

        Err(Error::exchange("-1", "Failed to parse order response"))
    }

    /// Cancel an order.
    pub async fn cancel_order(&self, id: &str, symbol: &str) -> Result<Order> {
        let market = self.base().market(symbol).await?;
        let asset_index: u32 = market.id.parse().unwrap_or(0);
        let order_id: u64 = id
            .parse()
            .map_err(|_| Error::invalid_request("Invalid order ID format"))?;

        let action = {
            let mut cancel_map = Map::new();
            cancel_map.insert("a".to_string(), asset_index.into());
            cancel_map.insert("o".to_string(), order_id.into());

            let mut map = Map::new();
            map.insert("type".to_string(), Value::String("cancel".to_string()));
            map.insert(
                "cancels".to_string(),
                Value::Array(vec![Value::Object(cancel_map)]),
            );
            Value::Object(map)
        };

        let _response = self.signed_action(action).execute().await?;

        // Return a minimal order object indicating cancellation
        Ok(Order::new(
            id.to_string(),
            symbol.to_string(),
            OrderType::Limit,
            OrderSide::Buy, // Side is unknown for cancel response
            Decimal::ZERO,
            None,
            ccxt_core::types::OrderStatus::Cancelled,
        ))
    }

    /// Set leverage for a symbol.
    pub async fn set_leverage(&self, symbol: &str, leverage: u32, is_cross: bool) -> Result<()> {
        let market = self.base().market(symbol).await?;
        let asset_index: u32 = market.id.parse().unwrap_or(0);

        let leverage_type = if is_cross { "cross" } else { "isolated" };

        let action = {
            let mut map = Map::new();
            map.insert(
                "type".to_string(),
                Value::String("updateLeverage".to_string()),
            );
            map.insert("asset".to_string(), asset_index.into());
            map.insert("isCross".to_string(), is_cross.into());
            map.insert("leverage".to_string(), leverage.into());
            Value::Object(map)
        };

        let response = self.signed_action(action).execute().await?;

        // Check for success
        if error::is_error_response(&response) {
            return Err(error::parse_error(&response));
        }

        info!(
            "Set leverage for {} to {}x ({})",
            symbol, leverage, leverage_type
        );

        Ok(())
    }

    /// Cancel all orders for a symbol.
    pub async fn cancel_all_orders(&self, symbol: Option<&str>) -> Result<Vec<Order>> {
        let _address = self
            .wallet_address()
            .ok_or_else(|| Error::authentication("Private key required to cancel orders"))?;

        // First fetch open orders
        let open_orders = self.fetch_open_orders(symbol, None, None).await?;

        if open_orders.is_empty() {
            return Ok(Vec::new());
        }

        // Build cancel requests for all orders
        let mut cancels = Vec::new();
        for order in &open_orders {
            let market = self.base().market(&order.symbol).await?;
            let asset_index: u32 = market.id.parse().unwrap_or(0);
            let order_id: u64 = order.id.parse().unwrap_or(0);

            cancels.push(
                {
                    let mut map = Map::new();
                    map.insert("a".to_string(), asset_index.into());
                    map.insert("o".to_string(), order_id.into());
                    Ok(Value::Object(map))
                }
                .map_err(|e: serde_json::Error| Error::from(ParseError::from(e)))?,
            );
        }

        let action = {
            let mut map = Map::new();
            map.insert("type".to_string(), Value::String("cancel".to_string()));
            map.insert("cancels".to_string(), Value::Array(cancels));
            Value::Object(map)
        };

        let _response = self.signed_action(action).execute().await?;

        // Return the orders that were canceled
        let canceled_orders: Vec<Order> = open_orders
            .into_iter()
            .map(|mut o| {
                o.status = ccxt_core::types::OrderStatus::Cancelled;
                o
            })
            .collect();

        info!("Canceled {} orders", canceled_orders.len());

        Ok(canceled_orders)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[allow(deprecated)]
    fn test_get_nonce() {
        let exchange = HyperLiquid::builder().testnet(true).build().unwrap();

        let nonce1 = exchange.get_nonce();
        std::thread::sleep(std::time::Duration::from_millis(10));
        let nonce2 = exchange.get_nonce();

        assert!(nonce2 > nonce1);
    }
}
