//! MarketData trait definition.
//!
//! The `MarketData` trait provides methods for fetching public market data
//! that doesn't require authentication. This includes markets, tickers,
//! order books, trades, and OHLCV candlestick data.
//!
//! # Timestamp Format
//!
//! All timestamp parameters and return values in this trait use the standardized format:
//! - **Type**: `i64`
//! - **Unit**: Milliseconds since Unix epoch (January 1, 1970, 00:00:00 UTC)
//! - **Range**: Supports dates from 1970 to approximately year 294,276
//!
//! # Object Safety
//!
//! This trait is designed to be object-safe, allowing for dynamic dispatch via
//! trait objects (`dyn MarketData`). All async methods return boxed futures
//! to maintain object safety.
//!
//! # Example
//!
//! ```rust,ignore
//! use ccxt_core::traits::MarketData;
//! use ccxt_core::types::{Timeframe, params::OhlcvParams};
//!
//! async fn fetch_data(exchange: &dyn MarketData) -> Result<(), ccxt_core::Error> {
//!     // Fetch markets
//!     let markets = exchange.fetch_markets().await?;
//!     
//!     // Fetch ticker
//!     let ticker = exchange.fetch_ticker("BTC/USDT").await?;
//!     
//!     // Fetch OHLCV with i64 timestamp parameters
//!     let since: i64 = chrono::Utc::now().timestamp_millis() - 3600000; // 1 hour ago
//!     let candles = exchange.fetch_ohlcv_with_params(
//!         "BTC/USDT",
//!         OhlcvParams::new(Timeframe::H1).since(since).limit(100)
//!     ).await?;
//!     
//!     Ok(())
//! }
//! ```

use async_trait::async_trait;
use std::collections::HashMap;

use crate::error::Result;
use crate::traits::PublicExchange;
use crate::types::{
    Market, Ohlcv, OrderBook, Ticker, Timeframe, Trade,
    params::{OhlcvParams, OrderBookParams},
};

/// Trait for fetching public market data.
///
/// This trait provides methods for accessing public market information
/// that doesn't require authentication. All methods are async and return
/// `Result<T>` for proper error handling.
///
/// # Timestamp Format
///
/// All timestamp parameters and fields in returned data structures use:
/// - **Type**: `i64`
/// - **Unit**: Milliseconds since Unix epoch (January 1, 1970, 00:00:00 UTC)
/// - **Example**: `1609459200000` represents January 1, 2021, 00:00:00 UTC
///
/// # Supertrait
///
/// Requires `PublicExchange` as a supertrait to access exchange metadata
/// and capabilities.
///
/// # Thread Safety
///
/// This trait requires `Send + Sync` bounds (inherited from `PublicExchange`)
/// to ensure safe usage across thread boundaries in async contexts.
#[async_trait]
pub trait MarketData: PublicExchange {
    // ========================================================================
    // Markets
    // ========================================================================

    /// Fetch all available markets from the exchange.
    ///
    /// Returns a vector of `Market` structures containing information about
    /// each trading pair including symbols, precision, limits, and fees.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let markets = exchange.fetch_markets().await?;
    /// for market in markets {
    ///     println!("{}: {} ({})", market.symbol, market.base, market.quote);
    /// }
    /// ```
    async fn fetch_markets(&self) -> Result<Vec<Market>>;

    /// Load and cache markets.
    ///
    /// This method fetches markets if not already cached, or returns the
    /// cached markets. Use `reload_markets()` to force a refresh.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// // First call loads from API
    /// let markets = exchange.load_markets().await?;
    ///
    /// // Subsequent calls use cache
    /// let markets = exchange.load_markets().await?;
    /// ```
    async fn load_markets(&self) -> Result<HashMap<String, Market>> {
        self.load_markets_with_reload(false).await
    }

    /// Force reload markets from the API.
    ///
    /// Clears the cache and fetches fresh market data.
    async fn reload_markets(&self) -> Result<HashMap<String, Market>> {
        self.load_markets_with_reload(true).await
    }

    /// Internal method to load markets with optional reload.
    ///
    /// Implementations should cache markets and only fetch from the API
    /// when `reload` is `true` or the cache is empty.
    async fn load_markets_with_reload(&self, reload: bool) -> Result<HashMap<String, Market>>;

    /// Get a specific market by symbol.
    ///
    /// Returns the market information for the given symbol, or an error
    /// if the symbol is not found.
    async fn market(&self, symbol: &str) -> Result<Market>;

    /// Get all cached markets.
    ///
    /// Returns the currently cached markets without fetching from the API.
    async fn markets(&self) -> HashMap<String, Market>;

    /// Check if a symbol exists and is active.
    ///
    /// Returns `true` if the symbol exists in the market cache and is active.
    async fn has_symbol(&self, symbol: &str) -> bool {
        self.market(symbol).await.map(|m| m.active).unwrap_or(false)
    }

    // ========================================================================
    // Tickers
    // ========================================================================

    /// Fetch ticker for a single symbol.
    ///
    /// Returns current price and volume information for the specified symbol.
    ///
    /// # Arguments
    ///
    /// * `symbol` - The trading pair symbol (e.g., "BTC/USDT")
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let ticker = exchange.fetch_ticker("BTC/USDT").await?;
    /// println!("Last price: {:?}", ticker.last);
    /// ```
    async fn fetch_ticker(&self, symbol: &str) -> Result<Ticker>;

    /// Fetch tickers for multiple symbols.
    ///
    /// Returns tickers for the specified symbols. If `symbols` is empty,
    /// returns tickers for all available symbols.
    ///
    /// # Arguments
    ///
    /// * `symbols` - Slice of trading pair symbols
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// // Fetch specific symbols
    /// let tickers = exchange.fetch_tickers(&["BTC/USDT", "ETH/USDT"]).await?;
    ///
    /// // Fetch all tickers
    /// let all_tickers = exchange.fetch_all_tickers().await?;
    /// ```
    async fn fetch_tickers(&self, symbols: &[&str]) -> Result<Vec<Ticker>>;

    /// Fetch all tickers.
    ///
    /// Convenience method that calls `fetch_tickers` with an empty slice.
    async fn fetch_all_tickers(&self) -> Result<Vec<Ticker>> {
        self.fetch_tickers(&[]).await
    }

    // ========================================================================
    // Order Book
    // ========================================================================

    /// Fetch order book with default depth.
    ///
    /// Returns the current order book (bids and asks) for the specified symbol.
    ///
    /// # Arguments
    ///
    /// * `symbol` - The trading pair symbol
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let orderbook = exchange.fetch_order_book("BTC/USDT").await?;
    /// println!("Best bid: {:?}", orderbook.bids.first());
    /// println!("Best ask: {:?}", orderbook.asks.first());
    /// ```
    async fn fetch_order_book(&self, symbol: &str) -> Result<OrderBook> {
        self.fetch_order_book_with_params(symbol, OrderBookParams::default())
            .await
    }

    /// Fetch order book with custom depth.
    ///
    /// # Arguments
    ///
    /// * `symbol` - The trading pair symbol
    /// * `limit` - Maximum number of price levels to return
    async fn fetch_order_book_with_depth(&self, symbol: &str, limit: u32) -> Result<OrderBook> {
        self.fetch_order_book_with_params(symbol, OrderBookParams::default().limit(limit))
            .await
    }

    /// Fetch order book with full parameters.
    ///
    /// # Arguments
    ///
    /// * `symbol` - The trading pair symbol
    /// * `params` - Order book parameters including depth limit
    async fn fetch_order_book_with_params(
        &self,
        symbol: &str,
        params: OrderBookParams,
    ) -> Result<OrderBook>;

    // ========================================================================
    // Trades
    // ========================================================================

    /// Fetch recent public trades.
    ///
    /// Returns recent trades for the specified symbol.
    ///
    /// # Arguments
    ///
    /// * `symbol` - The trading pair symbol
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let trades = exchange.fetch_trades("BTC/USDT").await?;
    /// for trade in trades {
    ///     println!("{}: {} @ {}", trade.side, trade.amount, trade.price);
    /// }
    /// ```
    async fn fetch_trades(&self, symbol: &str) -> Result<Vec<Trade>> {
        self.fetch_trades_with_limit(symbol, None, None).await
    }

    /// Fetch recent trades with limit only.
    ///
    /// # Arguments
    ///
    /// * `symbol` - The trading pair symbol
    /// * `limit` - Maximum number of trades to return
    async fn fetch_trades_with_limit_only(
        &self,
        symbol: &str,
        limit: Option<u32>,
    ) -> Result<Vec<Trade>> {
        self.fetch_trades_with_limit(symbol, None, limit).await
    }

    /// Fetch recent trades with limit and optional timestamp filtering.
    ///
    /// Returns recent trades for the specified symbol, optionally filtered by timestamp.
    ///
    /// # Arguments
    ///
    /// * `symbol` - The trading pair symbol (e.g., "BTC/USDT")
    /// * `since` - Optional start timestamp in milliseconds (i64) since Unix epoch
    /// * `limit` - Maximum number of trades to return
    ///
    /// # Timestamp Format
    ///
    /// The `since` parameter uses `i64` milliseconds since Unix epoch:
    /// - `1609459200000` = January 1, 2021, 00:00:00 UTC
    /// - `chrono::Utc::now().timestamp_millis()` = Current time
    /// - `chrono::Utc::now().timestamp_millis() - 3600000` = 1 hour ago
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// // Fetch recent trades without timestamp filter
    /// let trades = exchange.fetch_trades_with_limit("BTC/USDT", None, Some(100)).await?;
    ///
    /// // Fetch trades from the last hour
    /// let since = chrono::Utc::now().timestamp_millis() - 3600000;
    /// let trades = exchange.fetch_trades_with_limit("BTC/USDT", Some(since), Some(50)).await?;
    /// ```
    async fn fetch_trades_with_limit(
        &self,
        symbol: &str,
        since: Option<i64>,
        limit: Option<u32>,
    ) -> Result<Vec<Trade>>;

    // ========================================================================
    // Deprecated u64 Wrapper Methods (Backward Compatibility)
    // ========================================================================

    /// Fetch OHLCV candlestick data with u64 timestamps (deprecated).
    ///
    /// **DEPRECATED**: Use `fetch_ohlcv_with_params` with i64 timestamps instead.
    /// This method is provided for backward compatibility during migration.
    ///
    /// # Migration
    ///
    /// ```rust,ignore
    /// // Old code (deprecated)
    /// let candles = exchange.fetch_ohlcv_u64("BTC/USDT", Timeframe::H1, Some(1609459200000u64), Some(100), None).await?;
    ///
    /// // New code (recommended)
    /// let params = OhlcvParams::new(Timeframe::H1).since(1609459200000i64).limit(100);
    /// let candles = exchange.fetch_ohlcv_with_params("BTC/USDT", params).await?;
    /// ```
    #[deprecated(
        since = "0.1.0",
        note = "Use fetch_ohlcv_with_params with i64 timestamps. Convert using TimestampUtils::u64_to_i64()"
    )]
    async fn fetch_ohlcv_u64(
        &self,
        symbol: &str,
        timeframe: Timeframe,
        since: Option<u64>,
        limit: Option<u32>,
        until: Option<u64>,
    ) -> Result<Vec<Ohlcv>> {
        use crate::time::TimestampConversion;
        use crate::types::params::OhlcvParams;

        let since_i64 = since.to_i64()?;
        let until_i64 = until.to_i64()?;

        let mut params = OhlcvParams::new(timeframe);
        if let Some(ts) = since_i64 {
            params = params.since(ts);
        }
        if let Some(ts) = until_i64 {
            params = params.until(ts);
        }
        if let Some(n) = limit {
            params = params.limit(n);
        }

        self.fetch_ohlcv_with_params(symbol, params).await
    }

    /// Fetch recent trades with u64 timestamp filtering (deprecated).
    ///
    /// **DEPRECATED**: Use `fetch_trades_with_limit` with i64 timestamps instead.
    /// This method is provided for backward compatibility during migration.
    ///
    /// # Migration
    ///
    /// ```rust,ignore
    /// // Old code (deprecated)
    /// let trades = exchange.fetch_trades_u64("BTC/USDT", Some(1609459200000u64), Some(100)).await?;
    ///
    /// // New code (recommended)
    /// let trades = exchange.fetch_trades_with_limit("BTC/USDT", Some(1609459200000i64), Some(100)).await?;
    /// ```
    #[deprecated(
        since = "0.1.0",
        note = "Use fetch_trades_with_limit with i64 timestamps. Convert using TimestampUtils::u64_to_i64()"
    )]
    async fn fetch_trades_u64(
        &self,
        symbol: &str,
        since: Option<u64>,
        limit: Option<u32>,
    ) -> Result<Vec<Trade>> {
        use crate::time::TimestampConversion;

        let since_i64 = since.to_i64()?;
        self.fetch_trades_with_limit(symbol, since_i64, limit).await
    }

    // ========================================================================
    // OHLCV (Candlesticks)
    // ========================================================================

    /// Fetch OHLCV candlestick data.
    ///
    /// Returns candlestick data for the specified symbol and timeframe.
    ///
    /// # Arguments
    ///
    /// * `symbol` - The trading pair symbol
    /// * `timeframe` - The candlestick timeframe
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let candles = exchange.fetch_ohlcv("BTC/USDT", Timeframe::H1).await?;
    /// for candle in candles {
    ///     println!("O: {} H: {} L: {} C: {}",
    ///         candle.open, candle.high, candle.low, candle.close);
    /// }
    /// ```
    async fn fetch_ohlcv(&self, symbol: &str, timeframe: Timeframe) -> Result<Vec<Ohlcv>> {
        self.fetch_ohlcv_with_params(symbol, OhlcvParams::new(timeframe))
            .await
    }

    /// Fetch OHLCV with full parameters.
    ///
    /// Allows specifying additional parameters like start time, limit, and
    /// price type (for futures).
    ///
    /// # Arguments
    ///
    /// * `symbol` - The trading pair symbol (e.g., "BTC/USDT")
    /// * `params` - OHLCV parameters including timeframe, since, limit, until
    ///
    /// # Timestamp Format
    ///
    /// All timestamp parameters in `OhlcvParams` use `i64` milliseconds since Unix epoch:
    /// - `since`: Start time for historical data
    /// - `until`: End time for historical data
    /// - Returned `Ohlcv` structures have `timestamp` field as `i64`
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use ccxt_core::types::params::OhlcvParams;
    ///
    /// // Fetch recent candles
    /// let candles = exchange.fetch_ohlcv_with_params(
    ///     "BTC/USDT",
    ///     OhlcvParams::new(Timeframe::H1).limit(100)
    /// ).await?;
    ///
    /// // Fetch historical candles with timestamp range
    /// let since = chrono::Utc::now().timestamp_millis() - (7 * 24 * 60 * 60 * 1000); // 7 days ago
    /// let until = chrono::Utc::now().timestamp_millis();
    /// let candles = exchange.fetch_ohlcv_with_params(
    ///     "BTC/USDT",
    ///     OhlcvParams::new(Timeframe::D1)
    ///         .since(since)
    ///         .until(until)
    ///         .limit(7)
    /// ).await?;
    /// ```
    async fn fetch_ohlcv_with_params(
        &self,
        symbol: &str,
        params: OhlcvParams,
    ) -> Result<Vec<Ohlcv>>;
}

/// Type alias for boxed MarketData trait object.
pub type BoxedMarketData = Box<dyn MarketData>;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capability::ExchangeCapabilities;

    // Mock implementation for testing trait object safety
    struct MockExchange;

    impl PublicExchange for MockExchange {
        fn id(&self) -> &str {
            "mock"
        }
        fn name(&self) -> &str {
            "Mock Exchange"
        }
        fn capabilities(&self) -> ExchangeCapabilities {
            ExchangeCapabilities::public_only()
        }
        fn timeframes(&self) -> Vec<Timeframe> {
            vec![Timeframe::H1, Timeframe::D1]
        }
    }

    #[async_trait]
    impl MarketData for MockExchange {
        async fn fetch_markets(&self) -> Result<Vec<Market>> {
            Ok(vec![])
        }

        async fn load_markets_with_reload(&self, _reload: bool) -> Result<HashMap<String, Market>> {
            Ok(HashMap::new())
        }

        async fn market(&self, _symbol: &str) -> Result<Market> {
            Err(crate::Error::invalid_request("Not found"))
        }

        async fn markets(&self) -> HashMap<String, Market> {
            HashMap::new()
        }

        async fn fetch_ticker(&self, symbol: &str) -> Result<Ticker> {
            Ok(Ticker::new(symbol.to_string(), 0))
        }

        async fn fetch_tickers(&self, _symbols: &[&str]) -> Result<Vec<Ticker>> {
            Ok(vec![])
        }

        async fn fetch_order_book_with_params(
            &self,
            symbol: &str,
            _params: OrderBookParams,
        ) -> Result<OrderBook> {
            Ok(OrderBook::new(symbol.to_string(), 0))
        }

        async fn fetch_trades_with_limit(
            &self,
            _symbol: &str,
            _since: Option<i64>,
            _limit: Option<u32>,
        ) -> Result<Vec<Trade>> {
            Ok(vec![])
        }

        async fn fetch_ohlcv_with_params(
            &self,
            _symbol: &str,
            _params: OhlcvParams,
        ) -> Result<Vec<Ohlcv>> {
            Ok(vec![])
        }
    }

    #[test]
    fn test_trait_object_safety() {
        // Verify trait is object-safe by creating a trait object
        let _exchange: BoxedMarketData = Box::new(MockExchange);
    }

    #[tokio::test]
    async fn test_default_implementations() {
        let exchange = MockExchange;

        // Test load_markets default
        let markets = exchange.load_markets().await.unwrap();
        assert!(markets.is_empty());

        // Test reload_markets default
        let markets = exchange.reload_markets().await.unwrap();
        assert!(markets.is_empty());

        // Test has_symbol default
        let has = exchange.has_symbol("BTC/USDT").await;
        assert!(!has);

        // Test fetch_all_tickers default
        let tickers = exchange.fetch_all_tickers().await.unwrap();
        assert!(tickers.is_empty());

        // Test fetch_order_book default
        let orderbook = exchange.fetch_order_book("BTC/USDT").await.unwrap();
        assert_eq!(orderbook.symbol, "BTC/USDT");

        // Test fetch_order_book_with_depth default
        let orderbook = exchange
            .fetch_order_book_with_depth("BTC/USDT", 100)
            .await
            .unwrap();
        assert_eq!(orderbook.symbol, "BTC/USDT");

        // Test fetch_trades default
        let trades = exchange.fetch_trades("BTC/USDT").await.unwrap();
        assert!(trades.is_empty());

        // Test fetch_ohlcv default
        let candles = exchange
            .fetch_ohlcv("BTC/USDT", Timeframe::H1)
            .await
            .unwrap();
        assert!(candles.is_empty());
    }
}
