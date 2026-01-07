//! Base exchange implementation
//!
//! Provides core functionality for all exchange implementations:
//! - Market data caching
//! - API configuration management
//! - Common request/response handling
//! - Authentication and signing
//! - Rate limiting

mod config;
mod market_cache;
mod requests;

pub use config::{ExchangeConfig, ExchangeConfigBuilder};
pub use market_cache::MarketCache;
pub use requests::RequestUtils;

use crate::error::{Error, ParseError, Result};
use crate::exchange::ExchangeCapabilities;
use crate::http_client::{HttpClient, HttpConfig};
use crate::rate_limiter::{RateLimiter, RateLimiterConfig};
#[allow(clippy::wildcard_imports)]
use crate::types::*;
use rust_decimal::Decimal;
use rust_decimal::prelude::{FromStr, ToPrimitive};
use serde_json::Value;
use std::collections::HashMap;
use std::future::Future;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{Mutex, RwLock};
use tracing::{debug, info, warn};

/// Base exchange implementation
#[derive(Debug)]
pub struct BaseExchange {
    /// Exchange configuration
    pub config: ExchangeConfig,
    /// HTTP client for API requests
    pub http_client: HttpClient,
    /// Thread-safe market data cache
    pub market_cache: Arc<RwLock<MarketCache>>,
    /// Mutex to serialize market loading operations
    pub market_loading_lock: Arc<Mutex<()>>,
    /// Exchange capability flags
    pub capabilities: ExchangeCapabilities,
    /// API endpoint URLs
    pub urls: HashMap<String, String>,
    /// Timeframe mappings
    pub timeframes: HashMap<String, String>,
    /// Precision mode for price/amount formatting
    pub precision_mode: PrecisionMode,
}

impl BaseExchange {
    /// Creates a new exchange instance
    pub fn new(config: ExchangeConfig) -> Result<Self> {
        info!("Initializing exchange: {}", config.id);

        if config.timeout.is_zero() {
            return Err(Error::invalid_request("timeout cannot be zero"));
        }
        if config.connect_timeout.is_zero() {
            return Err(Error::invalid_request("connect_timeout cannot be zero"));
        }

        if config.timeout > Duration::from_secs(300) {
            warn!(
                timeout_secs = config.timeout.as_secs(),
                "Request timeout exceeds 5 minutes"
            );
        }

        let http_config = HttpConfig {
            timeout: config.timeout,
            connect_timeout: config.connect_timeout,
            #[allow(deprecated, clippy::map_unwrap_or)]
            max_retries: config.retry_policy.map(|p| p.max_retries).unwrap_or(3),
            verbose: false,
            user_agent: config
                .user_agent
                .clone()
                .unwrap_or_else(|| format!("ccxt-rust/{}", env!("CARGO_PKG_VERSION"))),
            return_response_headers: false,
            proxy: config.proxy.clone(),
            enable_rate_limit: true,
            retry_config: config
                .retry_policy
                .map(|p| crate::retry_strategy::RetryConfig {
                    max_retries: p.max_retries,
                    #[allow(clippy::cast_possible_truncation)]
                    base_delay_ms: p.delay.as_millis() as u64,
                    strategy_type: crate::retry_strategy::RetryStrategyType::Fixed,
                    ..crate::retry_strategy::RetryConfig::default()
                }),
            max_response_size: 10 * 1024 * 1024, // 10MB default
            circuit_breaker: None,               // Disabled by default for backward compatibility
            pool_max_idle_per_host: 10,          // Default: 10 idle connections per host
            pool_idle_timeout: Duration::from_secs(90), // Default: 90 seconds
        };

        let mut http_client = HttpClient::new(http_config)?;

        if config.enable_rate_limit {
            let rate_config =
                RateLimiterConfig::new(config.rate_limit, Duration::from_millis(1000));
            let limiter = RateLimiter::new(rate_config);
            http_client.set_rate_limiter(limiter);
        }

        Ok(Self {
            config,
            http_client,
            market_cache: Arc::new(RwLock::new(MarketCache::default())),
            market_loading_lock: Arc::new(Mutex::new(())),
            capabilities: ExchangeCapabilities::default(),
            urls: HashMap::new(),
            timeframes: HashMap::new(),
            precision_mode: PrecisionMode::DecimalPlaces,
        })
    }

    /// Loads market data from the exchange
    pub async fn load_markets(&self, reload: bool) -> Result<Arc<HashMap<String, Arc<Market>>>> {
        let cache = self.market_cache.read().await;

        if cache.is_loaded() && !reload {
            debug!("Returning cached markets for {}", self.config.id);
            return Ok(cache.markets());
        }

        info!("Loading markets for {}", self.config.id);
        drop(cache);

        Err(Error::not_implemented(
            "load_markets must be implemented by exchange",
        ))
    }

    /// Loads market data with a custom loader function
    pub async fn load_markets_with_loader<F, Fut>(
        &self,
        reload: bool,
        loader: F,
    ) -> Result<Arc<HashMap<String, Arc<Market>>>>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<(Vec<Market>, Option<Vec<Currency>>)>>,
    {
        let _loading_guard = self.market_loading_lock.lock().await;

        {
            let cache = self.market_cache.read().await;
            if cache.is_loaded() && !reload {
                debug!(
                    "Returning cached markets for {} ({} markets)",
                    self.config.id,
                    cache.market_count()
                );
                return Ok(cache.markets());
            }
        }

        info!(
            "Loading markets for {} (reload: {})",
            self.config.id, reload
        );
        let (markets, currencies) = loader().await?;

        self.set_markets(markets, currencies).await?;

        let cache = self.market_cache.read().await;
        Ok(cache.markets())
    }

    /// Sets market and currency data in the cache
    pub async fn set_markets(
        &self,
        markets: Vec<Market>,
        currencies: Option<Vec<Currency>>,
    ) -> Result<Arc<HashMap<String, Arc<Market>>>> {
        let cache = self.market_cache.read().await;
        cache.set_markets(markets, currencies, &self.config.id)
    }

    /// Gets market information by trading symbol
    pub async fn market(&self, symbol: &str) -> Result<Arc<Market>> {
        let cache = self.market_cache.read().await;

        if !cache.is_loaded() {
            drop(cache);
            return Err(Error::exchange(
                "-1",
                "Markets not loaded. Call load_markets() first.",
            ));
        }

        cache
            .get_market(symbol)
            .ok_or_else(|| Error::bad_symbol(format!("Market {symbol} not found")))
    }

    /// Gets market information by exchange-specific market ID
    pub async fn market_by_id(&self, id: &str) -> Result<Arc<Market>> {
        let cache = self.market_cache.read().await;
        cache
            .get_market_by_id(id)
            .ok_or_else(|| Error::bad_symbol(format!("Market with id {id} not found")))
    }

    /// Gets currency information by currency code
    pub async fn currency(&self, code: &str) -> Result<Arc<Currency>> {
        let cache = self.market_cache.read().await;
        cache
            .get_currency(code)
            .ok_or_else(|| Error::bad_symbol(format!("Currency {code} not found")))
    }

    /// Gets all available trading symbols
    pub async fn symbols(&self) -> Result<Vec<String>> {
        let cache = self.market_cache.read().await;
        Ok(cache.symbols())
    }

    /// Applies rate limiting if enabled (deprecated, now handled by HttpClient)
    #[deprecated(
        since = "0.2.0",
        note = "Rate limiting is now handled internally by HttpClient. This method is a no-op."
    )]
    pub fn throttle(&self) -> Result<()> {
        Ok(())
    }

    /// Checks that required API credentials are configured
    pub fn check_required_credentials(&self) -> Result<()> {
        if self.config.api_key.is_none() {
            return Err(Error::authentication("API key is required"));
        }
        if self.config.secret.is_none() {
            return Err(Error::authentication("API secret is required"));
        }
        Ok(())
    }

    /// Gets a nonce value (current timestamp in milliseconds)
    pub fn nonce(&self) -> i64 {
        crate::time::milliseconds()
    }

    /// Builds a URL query string from parameters
    pub fn build_query_string(&self, params: &HashMap<String, Value>) -> String {
        RequestUtils::build_query_string(params)
    }

    /// Parses a JSON response string
    pub fn parse_json(&self, response: &str) -> Result<Value> {
        RequestUtils::parse_json(response)
    }

    /// Handles HTTP error responses
    pub fn handle_http_error(&self, status_code: u16, response: &str) -> Error {
        RequestUtils::handle_http_error(status_code, response)
    }

    /// Safely extracts a string value from a JSON object
    pub fn safe_string(&self, dict: &Value, key: &str) -> Option<String> {
        RequestUtils::safe_string(dict, key)
    }

    /// Safely extracts an integer value from a JSON object
    pub fn safe_integer(&self, dict: &Value, key: &str) -> Option<i64> {
        RequestUtils::safe_integer(dict, key)
    }

    /// Safely extracts a float value from a JSON object
    pub fn safe_float(&self, dict: &Value, key: &str) -> Option<f64> {
        RequestUtils::safe_float(dict, key)
    }

    /// Safely extracts a boolean value from a JSON object
    pub fn safe_bool(&self, dict: &Value, key: &str) -> Option<bool> {
        RequestUtils::safe_bool(dict, key)
    }

    /// Parses raw ticker data from exchange API response
    pub fn parse_ticker(&self, ticker_data: &Value, market: Option<&Market>) -> Result<Ticker> {
        let symbol = if let Some(m) = market {
            m.symbol.clone()
        } else {
            self.safe_string(ticker_data, "symbol")
                .ok_or_else(|| ParseError::missing_field("symbol"))?
        };

        let timestamp = self.safe_integer(ticker_data, "timestamp").unwrap_or(0);

        Ok(Ticker {
            symbol,
            timestamp,
            datetime: self.safe_string(ticker_data, "datetime"),
            high: self.safe_decimal(ticker_data, "high").map(Price::new),
            low: self.safe_decimal(ticker_data, "low").map(Price::new),
            bid: self.safe_decimal(ticker_data, "bid").map(Price::new),
            bid_volume: self.safe_decimal(ticker_data, "bidVolume").map(Amount::new),
            ask: self.safe_decimal(ticker_data, "ask").map(Price::new),
            ask_volume: self.safe_decimal(ticker_data, "askVolume").map(Amount::new),
            vwap: self.safe_decimal(ticker_data, "vwap").map(Price::new),
            open: self.safe_decimal(ticker_data, "open").map(Price::new),
            close: self.safe_decimal(ticker_data, "close").map(Price::new),
            last: self.safe_decimal(ticker_data, "last").map(Price::new),
            previous_close: self
                .safe_decimal(ticker_data, "previousClose")
                .map(Price::new),
            change: self.safe_decimal(ticker_data, "change").map(Price::new),
            percentage: self.safe_decimal(ticker_data, "percentage"),
            average: self.safe_decimal(ticker_data, "average").map(Price::new),
            base_volume: self
                .safe_decimal(ticker_data, "baseVolume")
                .map(Amount::new),
            quote_volume: self
                .safe_decimal(ticker_data, "quoteVolume")
                .map(Amount::new),
            info: HashMap::new(),
        })
    }

    /// Parses raw trade data from exchange API response
    pub fn parse_trade(&self, trade_data: &Value, market: Option<&Market>) -> Result<Trade> {
        let symbol = if let Some(m) = market {
            m.symbol.clone()
        } else {
            self.safe_string(trade_data, "symbol")
                .ok_or_else(|| ParseError::missing_field("symbol"))?
        };

        let side = self
            .safe_string(trade_data, "side")
            .and_then(|s| match s.to_lowercase().as_str() {
                "buy" => Some(OrderSide::Buy),
                "sell" => Some(OrderSide::Sell),
                _ => None,
            })
            .ok_or_else(|| ParseError::missing_field("side"))?;

        let trade_type =
            self.safe_string(trade_data, "type")
                .and_then(|t| match t.to_lowercase().as_str() {
                    "limit" => Some(OrderType::Limit),
                    "market" => Some(OrderType::Market),
                    _ => None,
                });

        let taker_or_maker = self.safe_string(trade_data, "takerOrMaker").and_then(|s| {
            match s.to_lowercase().as_str() {
                "taker" => Some(TakerOrMaker::Taker),
                "maker" => Some(TakerOrMaker::Maker),
                _ => None,
            }
        });

        Ok(Trade {
            id: self.safe_string(trade_data, "id"),
            order: self.safe_string(trade_data, "orderId"),
            timestamp: self.safe_integer(trade_data, "timestamp").unwrap_or(0),
            datetime: self.safe_string(trade_data, "datetime"),
            symbol,
            trade_type,
            side,
            taker_or_maker,
            price: Price::new(
                self.safe_decimal(trade_data, "price")
                    .unwrap_or(Decimal::ZERO),
            ),
            amount: Amount::new(
                self.safe_decimal(trade_data, "amount")
                    .unwrap_or(Decimal::ZERO),
            ),
            cost: self.safe_decimal(trade_data, "cost").map(Cost::new),
            fee: None,
            info: if let Some(obj) = trade_data.as_object() {
                obj.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
            } else {
                HashMap::new()
            },
        })
    }

    /// Parses raw order data from exchange API response
    pub fn parse_order(&self, order_data: &Value, market: Option<&Market>) -> Result<Order> {
        let symbol = if let Some(m) = market {
            m.symbol.clone()
        } else {
            self.safe_string(order_data, "symbol")
                .ok_or_else(|| ParseError::missing_field("symbol"))?
        };

        let order_type = self
            .safe_string(order_data, "type")
            .and_then(|t| match t.to_lowercase().as_str() {
                "limit" => Some(OrderType::Limit),
                "market" => Some(OrderType::Market),
                _ => None,
            })
            .unwrap_or(OrderType::Limit);

        let side = self
            .safe_string(order_data, "side")
            .and_then(|s| match s.to_lowercase().as_str() {
                "buy" => Some(OrderSide::Buy),
                "sell" => Some(OrderSide::Sell),
                _ => None,
            })
            .unwrap_or(OrderSide::Buy);

        let status_str = self
            .safe_string(order_data, "status")
            .unwrap_or_else(|| "open".to_string());
        #[allow(clippy::match_same_arms)]
        let status = match status_str.to_lowercase().as_str() {
            "open" => OrderStatus::Open,
            "closed" => OrderStatus::Closed,
            "canceled" | "cancelled" => OrderStatus::Cancelled,
            "expired" => OrderStatus::Expired,
            "rejected" => OrderStatus::Rejected,
            _ => OrderStatus::Open,
        };

        let id = self
            .safe_string(order_data, "id")
            .unwrap_or_else(|| format!("order_{}", chrono::Utc::now().timestamp_millis()));
        let amount = self
            .safe_decimal(order_data, "amount")
            .unwrap_or(Decimal::ZERO);

        Ok(Order {
            id,
            client_order_id: self.safe_string(order_data, "clientOrderId"),
            timestamp: self.safe_integer(order_data, "timestamp"),
            datetime: self.safe_string(order_data, "datetime"),
            last_trade_timestamp: self.safe_integer(order_data, "lastTradeTimestamp"),
            symbol,
            order_type,
            time_in_force: self.safe_string(order_data, "timeInForce"),
            post_only: self
                .safe_string(order_data, "postOnly")
                .and_then(|s| s.parse::<bool>().ok()),
            reduce_only: self
                .safe_string(order_data, "reduceOnly")
                .and_then(|s| s.parse::<bool>().ok()),
            side,
            price: self.safe_decimal(order_data, "price"),
            stop_price: self.safe_decimal(order_data, "stopPrice"),
            trigger_price: self.safe_decimal(order_data, "triggerPrice"),
            take_profit_price: self.safe_decimal(order_data, "takeProfitPrice"),
            stop_loss_price: self.safe_decimal(order_data, "stopLossPrice"),
            average: self.safe_decimal(order_data, "average"),
            amount,
            filled: self.safe_decimal(order_data, "filled"),
            remaining: self.safe_decimal(order_data, "remaining"),
            cost: self.safe_decimal(order_data, "cost"),
            status,
            fee: None,
            fees: None,
            trades: None,
            trailing_delta: self.safe_decimal(order_data, "trailingDelta"),
            trailing_percent: self.safe_decimal(order_data, "trailingPercent"),
            activation_price: self.safe_decimal(order_data, "activationPrice"),
            callback_rate: self.safe_decimal(order_data, "callbackRate"),
            working_type: self.safe_string(order_data, "workingType"),
            info: if let Some(obj) = order_data.as_object() {
                obj.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
            } else {
                HashMap::new()
            },
        })
    }

    /// Parses raw balance data from exchange API response
    pub fn parse_balance(&self, balance_data: &Value) -> Result<Balance> {
        let mut balance = Balance::new();

        if let Some(obj) = balance_data.as_object() {
            for (currency, balance_info) in obj {
                if currency == "timestamp" || currency == "datetime" || currency == "info" {
                    continue;
                }
                let free = self
                    .safe_decimal(balance_info, "free")
                    .unwrap_or(Decimal::ZERO);
                let used = self
                    .safe_decimal(balance_info, "used")
                    .unwrap_or(Decimal::ZERO);
                let total = self
                    .safe_decimal(balance_info, "total")
                    .unwrap_or(free + used);

                let entry = BalanceEntry { free, used, total };
                balance.set(currency.clone(), entry);
            }
        }

        if let Some(obj) = balance_data.as_object() {
            balance.info = obj.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
        }

        Ok(balance)
    }

    /// Parses raw order book data from exchange API response
    pub fn parse_order_book(
        &self,
        orderbook_data: &Value,
        timestamp: Option<i64>,
    ) -> Result<OrderBook> {
        let mut bids_side = OrderBookSide::new();
        let mut asks_side = OrderBookSide::new();

        if let Some(bids_array) = orderbook_data.get("bids").and_then(|v| v.as_array()) {
            for bid in bids_array {
                if let Some(arr) = bid.as_array() {
                    #[allow(clippy::collapsible_if)]
                    if arr.len() >= 2 {
                        let price = self.safe_decimal_from_value(&arr[0]);
                        let amount = self.safe_decimal_from_value(&arr[1]);
                        if let (Some(p), Some(a)) = (price, amount) {
                            bids_side.push(OrderBookEntry {
                                price: Price::new(p),
                                amount: Amount::new(a),
                            });
                        }
                    }
                }
            }
        }

        if let Some(asks_array) = orderbook_data.get("asks").and_then(|v| v.as_array()) {
            for ask in asks_array {
                if let Some(arr) = ask.as_array() {
                    #[allow(clippy::collapsible_if)]
                    if arr.len() >= 2 {
                        let price = self.safe_decimal_from_value(&arr[0]);
                        let amount = self.safe_decimal_from_value(&arr[1]);
                        if let (Some(p), Some(a)) = (price, amount) {
                            asks_side.push(OrderBookEntry {
                                price: Price::new(p),
                                amount: Amount::new(a),
                            });
                        }
                    }
                }
            }
        }

        Ok(OrderBook {
            symbol: self
                .safe_string(orderbook_data, "symbol")
                .unwrap_or_default(),
            bids: bids_side,
            asks: asks_side,
            timestamp: timestamp
                .or_else(|| self.safe_integer(orderbook_data, "timestamp"))
                .unwrap_or(0),
            datetime: self.safe_string(orderbook_data, "datetime"),
            nonce: self.safe_integer(orderbook_data, "nonce"),
            info: if let Some(obj) = orderbook_data.as_object() {
                obj.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
            } else {
                HashMap::new()
            },
            buffered_deltas: std::collections::VecDeque::new(),
            bids_map: std::collections::BTreeMap::new(),
            asks_map: std::collections::BTreeMap::new(),
            is_synced: false,
            needs_resync: false,
            last_resync_time: 0,
        })
    }

    fn safe_decimal(&self, data: &Value, key: &str) -> Option<Decimal> {
        data.get(key).and_then(|v| self.safe_decimal_from_value(v))
    }

    #[allow(clippy::unused_self)]
    fn safe_decimal_from_value(&self, value: &Value) -> Option<Decimal> {
        match value {
            Value::Number(n) => {
                if let Some(f) = n.as_f64() {
                    Decimal::from_f64_retain(f)
                } else {
                    None
                }
            }
            Value::String(s) => Decimal::from_str(s).ok(),
            _ => None,
        }
    }

    /// Calculates trading fee for a given order
    pub async fn calculate_fee(
        &self,
        symbol: &str,
        _order_type: OrderType,
        _side: OrderSide,
        amount: Decimal,
        price: Decimal,
        taker_or_maker: Option<&str>,
    ) -> Result<Fee> {
        let market = self.market(symbol).await?;

        let rate = if let Some(tom) = taker_or_maker {
            if tom == "taker" {
                market.taker.unwrap_or(Decimal::ZERO)
            } else {
                market.maker.unwrap_or(Decimal::ZERO)
            }
        } else {
            market.taker.unwrap_or(Decimal::ZERO)
        };

        let cost = amount * price;
        let fee_cost = cost * rate;

        Ok(Fee {
            currency: market.quote.clone(),
            cost: fee_cost,
            rate: Some(rate),
        })
    }

    /// Converts an amount to the precision required by the market
    pub async fn amount_to_precision(&self, symbol: &str, amount: Decimal) -> Result<Decimal> {
        let market = self.market(symbol).await?;
        match market.precision.amount {
            Some(precision_value) => Ok(self.round_to_precision(amount, precision_value)),
            None => Ok(amount),
        }
    }

    /// Converts a price to the precision required by the market
    pub async fn price_to_precision(&self, symbol: &str, price: Decimal) -> Result<Decimal> {
        let market = self.market(symbol).await?;
        match market.precision.price {
            Some(precision_value) => Ok(self.round_to_precision(price, precision_value)),
            None => Ok(price),
        }
    }

    /// Converts a cost to the precision required by the market
    pub async fn cost_to_precision(&self, symbol: &str, cost: Decimal) -> Result<Decimal> {
        let market = self.market(symbol).await?;
        match market.precision.price {
            Some(precision_value) => Ok(self.round_to_precision(cost, precision_value)),
            None => Ok(cost),
        }
    }

    #[allow(clippy::unused_self)]
    fn round_to_precision(&self, value: Decimal, precision_value: Decimal) -> Decimal {
        if precision_value < Decimal::ONE {
            let steps = (value / precision_value).round();
            steps * precision_value
        } else {
            let digits = precision_value.to_u32().unwrap_or(8);
            let multiplier = Decimal::from(10_i64.pow(digits));
            let scaled = value * multiplier;
            let rounded = scaled.round();
            rounded / multiplier
        }
    }

    /// Calculates the cost of a trade
    pub async fn calculate_cost(
        &self,
        symbol: &str,
        amount: Decimal,
        price: Decimal,
    ) -> Result<Decimal> {
        let _market = self.market(symbol).await?;
        Ok(amount * price)
    }
}

#[cfg(test)]
#[allow(clippy::disallowed_methods)] // unwrap() is acceptable in tests
#[allow(clippy::default_trait_access)] // Default::default() is acceptable in tests
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_base_exchange_creation() {
        let config = ExchangeConfig {
            id: "test".to_string(),
            name: "Test Exchange".to_string(),
            ..Default::default()
        };
        let exchange = BaseExchange::new(config).unwrap();
        assert_eq!(exchange.config.id, "test");
        assert!(exchange.config.enable_rate_limit);
    }

    #[tokio::test]
    async fn test_market_cache() {
        let config = ExchangeConfig {
            id: "test".to_string(),
            ..Default::default()
        };
        let exchange = BaseExchange::new(config).unwrap();

        let markets = vec![Market {
            id: "btcusdt".to_string(),
            symbol: "BTC/USDT".to_string(),
            parsed_symbol: None,
            base: "BTC".to_string(),
            quote: "USDT".to_string(),
            active: true,
            market_type: MarketType::Spot,
            margin: false,
            settle: None,
            base_id: None,
            quote_id: None,
            settle_id: None,
            contract: None,
            linear: None,
            inverse: None,
            contract_size: None,
            expiry: None,
            expiry_datetime: None,
            strike: None,
            option_type: None,
            precision: Default::default(),
            limits: Default::default(),
            maker: None,
            taker: None,
            percentage: None,
            tier_based: None,
            fee_side: None,
            info: Default::default(),
        }];

        let _ = exchange.set_markets(markets, None).await.unwrap();
        let market = exchange.market("BTC/USDT").await.unwrap();
        assert_eq!(market.symbol, "BTC/USDT");

        let symbols = exchange.symbols().await.unwrap();
        assert_eq!(symbols.len(), 1);
    }

    #[test]
    fn test_build_query_string() {
        let config = ExchangeConfig::default();
        let exchange = BaseExchange::new(config).unwrap();

        let mut params = HashMap::new();
        params.insert("symbol".to_string(), Value::String("BTC/USDT".to_string()));
        params.insert("limit".to_string(), Value::Number(100.into()));

        let query = exchange.build_query_string(&params);
        assert!(query.contains("symbol="));
        assert!(query.contains("limit="));
    }
}
