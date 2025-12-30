//! Exchange trait implementation for Binance
//!
//! This module implements the unified `Exchange` trait from `ccxt-core` for Binance,
//! as well as the new decomposed traits (PublicExchange, MarketData, Trading, Account, Margin, Funding).
//!
//! # Backward Compatibility
//!
//! Binance implements both the legacy `Exchange` trait and the new modular traits.
//! This ensures that existing code using `dyn Exchange` continues to work, while
//! new code can use the more granular trait interfaces.
//!
//! # Trait Implementations
//!
//! - `Exchange`: Unified interface (backward compatible)
//! - `PublicExchange`: Metadata and capabilities
//! - `MarketData`: Public market data (via traits module)
//! - `Trading`: Order management (via traits module)
//! - `Account`: Balance and trade history (via traits module)
//! - `Margin`: Positions, leverage, funding (via traits module)
//! - `Funding`: Deposits, withdrawals, transfers (via traits module)

use async_trait::async_trait;
use ccxt_core::{
    Result,
    exchange::{Capability, Exchange, ExchangeCapabilities},
    traits::PublicExchange,
    types::{
        Amount, Balance, Market, Ohlcv, Order, OrderBook, OrderSide, OrderType, Price, Ticker,
        Timeframe, Trade,
    },
};
use rust_decimal::Decimal;
use std::collections::HashMap;
use std::sync::Arc;

use super::Binance;

// Re-export ExchangeExt for use in tests
#[cfg(test)]
use ccxt_core::exchange::ExchangeExt;

#[async_trait]
impl Exchange for Binance {
    // ==================== Metadata ====================

    fn id(&self) -> &str {
        "binance"
    }

    fn name(&self) -> &str {
        "Binance"
    }

    fn version(&self) -> &'static str {
        "v3"
    }

    fn certified(&self) -> bool {
        true
    }

    fn has_websocket(&self) -> bool {
        true
    }

    fn capabilities(&self) -> ExchangeCapabilities {
        // Binance supports almost all capabilities except:
        // - EditOrder: Binance doesn't support order editing
        // - FetchCanceledOrders: Binance doesn't have a separate API for canceled orders
        ExchangeCapabilities::builder()
            .all()
            .without_capability(Capability::EditOrder)
            .without_capability(Capability::FetchCanceledOrders)
            .build()
    }

    fn timeframes(&self) -> Vec<Timeframe> {
        vec![
            Timeframe::M1,
            Timeframe::M3,
            Timeframe::M5,
            Timeframe::M15,
            Timeframe::M30,
            Timeframe::H1,
            Timeframe::H2,
            Timeframe::H4,
            Timeframe::H6,
            Timeframe::H8,
            Timeframe::H12,
            Timeframe::D1,
            Timeframe::D3,
            Timeframe::W1,
            Timeframe::Mon1,
        ]
    }

    fn rate_limit(&self) -> u32 {
        50
    }

    // ==================== Market Data (Public API) ====================

    async fn fetch_markets(&self) -> Result<Vec<Market>> {
        let arc_markets = Binance::fetch_markets(self).await?;
        Ok(arc_markets.values().map(|v| (**v).clone()).collect())
    }

    async fn load_markets(&self, reload: bool) -> Result<Arc<HashMap<String, Arc<Market>>>> {
        Binance::load_markets(self, reload).await
    }

    async fn fetch_ticker(&self, symbol: &str) -> Result<Ticker> {
        // Delegate to existing implementation using default parameters
        Binance::fetch_ticker(self, symbol, ()).await
    }

    async fn fetch_tickers(&self, symbols: Option<&[String]>) -> Result<Vec<Ticker>> {
        // Convert slice to Vec for existing implementation
        let symbols_vec = symbols.map(|s| s.to_vec());
        Binance::fetch_tickers(self, symbols_vec).await
    }

    async fn fetch_order_book(&self, symbol: &str, limit: Option<u32>) -> Result<OrderBook> {
        // Delegate to existing implementation
        Binance::fetch_order_book(self, symbol, limit).await
    }

    async fn fetch_trades(&self, symbol: &str, limit: Option<u32>) -> Result<Vec<Trade>> {
        // Delegate to existing implementation
        Binance::fetch_trades(self, symbol, limit).await
    }

    async fn fetch_ohlcv(
        &self,
        symbol: &str,
        timeframe: Timeframe,
        since: Option<i64>,
        limit: Option<u32>,
    ) -> Result<Vec<Ohlcv>> {
        use ccxt_core::types::{Amount, Price};

        // Convert Timeframe enum to string for existing implementation
        let timeframe_str = timeframe.to_string();
        // Use i64 directly for the updated method signature
        let ohlcv_data =
            Binance::fetch_ohlcv(self, symbol, &timeframe_str, since, limit, None).await?;

        // Convert OHLCV to Ohlcv with proper type conversions
        Ok(ohlcv_data
            .into_iter()
            .map(|o| Ohlcv {
                timestamp: o.timestamp,
                open: Price::from(Decimal::try_from(o.open).unwrap_or_default()),
                high: Price::from(Decimal::try_from(o.high).unwrap_or_default()),
                low: Price::from(Decimal::try_from(o.low).unwrap_or_default()),
                close: Price::from(Decimal::try_from(o.close).unwrap_or_default()),
                volume: Amount::from(Decimal::try_from(o.volume).unwrap_or_default()),
            })
            .collect())
    }

    // ==================== Trading (Private API) ====================

    async fn create_order(
        &self,
        symbol: &str,
        order_type: OrderType,
        side: OrderSide,
        amount: Amount,
        price: Option<Price>,
    ) -> Result<Order> {
        // Direct delegation - no type conversion needed
        Binance::create_order(self, symbol, order_type, side, amount, price, None).await
    }

    async fn cancel_order(&self, id: &str, symbol: Option<&str>) -> Result<Order> {
        // Delegate to existing implementation
        // Note: Binance requires symbol for cancel_order
        let symbol_str = symbol.ok_or_else(|| {
            ccxt_core::Error::invalid_request("Symbol is required for cancel_order on Binance")
        })?;
        Binance::cancel_order(self, id, symbol_str).await
    }

    async fn cancel_all_orders(&self, symbol: Option<&str>) -> Result<Vec<Order>> {
        // Delegate to existing implementation
        // Note: Binance requires symbol for cancel_all_orders
        let symbol_str = symbol.ok_or_else(|| {
            ccxt_core::Error::invalid_request("Symbol is required for cancel_all_orders on Binance")
        })?;
        Binance::cancel_all_orders(self, symbol_str).await
    }

    async fn fetch_order(&self, id: &str, symbol: Option<&str>) -> Result<Order> {
        // Delegate to existing implementation
        // Note: Binance requires symbol for fetch_order
        let symbol_str = symbol.ok_or_else(|| {
            ccxt_core::Error::invalid_request("Symbol is required for fetch_order on Binance")
        })?;
        Binance::fetch_order(self, id, symbol_str).await
    }

    async fn fetch_open_orders(
        &self,
        symbol: Option<&str>,
        _since: Option<i64>,
        _limit: Option<u32>,
    ) -> Result<Vec<Order>> {
        // Delegate to existing implementation
        // Note: Binance's fetch_open_orders doesn't support since/limit parameters
        Binance::fetch_open_orders(self, symbol).await
    }

    async fn fetch_closed_orders(
        &self,
        symbol: Option<&str>,
        since: Option<i64>,
        limit: Option<u32>,
    ) -> Result<Vec<Order>> {
        // Delegate to existing implementation
        // Use i64 directly for since parameter
        Binance::fetch_closed_orders(self, symbol, since, limit).await
    }

    // ==================== Account (Private API) ====================

    async fn fetch_balance(&self) -> Result<Balance> {
        // Delegate to existing implementation
        Binance::fetch_balance(self, None).await
    }

    async fn fetch_my_trades(
        &self,
        symbol: Option<&str>,
        since: Option<i64>,
        limit: Option<u32>,
    ) -> Result<Vec<Trade>> {
        // Delegate to existing implementation
        // Note: Binance's fetch_my_trades requires a symbol
        let symbol_str = symbol.ok_or_else(|| {
            ccxt_core::Error::invalid_request("Symbol is required for fetch_my_trades on Binance")
        })?;
        // Use i64 directly for the updated method signature
        Binance::fetch_my_trades(self, symbol_str, since, limit).await
    }

    // ==================== Helper Methods ====================

    async fn market(&self, symbol: &str) -> Result<Arc<Market>> {
        // Use async read for async method
        let cache = self.base().market_cache.read().await;

        if !cache.loaded {
            return Err(ccxt_core::Error::exchange(
                "-1",
                "Markets not loaded. Call load_markets() first.",
            ));
        }

        cache
            .markets
            .get(symbol)
            .cloned()
            .ok_or_else(|| ccxt_core::Error::bad_symbol(format!("Market {} not found", symbol)))
    }

    async fn markets(&self) -> Arc<HashMap<String, Arc<Market>>> {
        let cache = self.base().market_cache.read().await;
        cache.markets.clone()
    }
}

// ==================== PublicExchange Trait Implementation ====================

#[async_trait]
impl PublicExchange for Binance {
    fn id(&self) -> &str {
        "binance"
    }

    fn name(&self) -> &str {
        "Binance"
    }

    fn version(&self) -> &'static str {
        "v3"
    }

    fn certified(&self) -> bool {
        true
    }

    fn capabilities(&self) -> ExchangeCapabilities {
        // Binance supports almost all capabilities except:
        // - EditOrder: Binance doesn't support order editing
        // - FetchCanceledOrders: Binance doesn't have a separate API for canceled orders
        ExchangeCapabilities::builder()
            .all()
            .without_capability(Capability::EditOrder)
            .without_capability(Capability::FetchCanceledOrders)
            .build()
    }

    fn timeframes(&self) -> Vec<Timeframe> {
        vec![
            Timeframe::M1,
            Timeframe::M3,
            Timeframe::M5,
            Timeframe::M15,
            Timeframe::M30,
            Timeframe::H1,
            Timeframe::H2,
            Timeframe::H4,
            Timeframe::H6,
            Timeframe::H8,
            Timeframe::H12,
            Timeframe::D1,
            Timeframe::D3,
            Timeframe::W1,
            Timeframe::Mon1,
        ]
    }

    fn rate_limit(&self) -> u32 {
        50
    }

    fn has_websocket(&self) -> bool {
        true
    }
}

// Helper methods for REST API operations
impl Binance {
    /// Check required authentication credentials.
    pub(crate) fn check_required_credentials(&self) -> ccxt_core::Result<()> {
        if self.base().config.api_key.is_none() || self.base().config.secret.is_none() {
            return Err(ccxt_core::Error::authentication(
                "API key and secret are required",
            ));
        }
        Ok(())
    }

    /// Get authenticator instance.
    pub(crate) fn get_auth(&self) -> ccxt_core::Result<super::auth::BinanceAuth> {
        let api_key = self
            .base()
            .config
            .api_key
            .as_ref()
            .ok_or_else(|| ccxt_core::Error::authentication("API key is required"))?
            .clone();

        let secret = self
            .base()
            .config
            .secret
            .as_ref()
            .ok_or_else(|| ccxt_core::Error::authentication("Secret is required"))?
            .clone();

        Ok(super::auth::BinanceAuth::new(api_key, secret))
    }

    // ==================== Time Sync Helper Methods ====================

    /// Gets the server timestamp for signing requests.
    ///
    /// This method uses the cached time offset if available, otherwise fetches
    /// the server time directly. It also triggers a background resync if needed.
    ///
    /// # Optimization
    ///
    /// By caching the time offset between local and server time, this method
    /// reduces the number of network round-trips for signed API requests from 2 to 1.
    /// Instead of fetching server time before every signed request, we calculate
    /// the server timestamp locally using: `server_timestamp = local_time + cached_offset`
    ///
    /// # Returns
    ///
    /// Returns the estimated server timestamp in milliseconds.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The time sync manager is not initialized and the server time fetch fails
    /// - Network errors occur during time synchronization
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use ccxt_exchanges::binance::Binance;
    /// # use ccxt_core::ExchangeConfig;
    /// # async fn example() -> ccxt_core::Result<()> {
    /// let binance = Binance::new(ExchangeConfig::default())?;
    ///
    /// // Get timestamp for signing (uses cached offset if available)
    /// let timestamp = binance.get_signing_timestamp().await?;
    /// println!("Server timestamp: {}", timestamp);
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// _Requirements: 1.2, 2.3, 6.4_
    pub async fn get_signing_timestamp(&self) -> ccxt_core::Result<i64> {
        // Check if we need to sync (not initialized or sync interval elapsed)
        if self.time_sync.needs_resync() {
            // Attempt to sync time
            if let Err(e) = self.sync_time().await {
                // If sync fails and we're not initialized, we must fail
                if !self.time_sync.is_initialized() {
                    return Err(e);
                }
                // If sync fails but we have a cached offset, log and continue
                tracing::warn!(
                    error = %e,
                    "Time sync failed, using cached offset"
                );
            }
        }

        // If still not initialized after sync attempt, fall back to direct fetch
        if !self.time_sync.is_initialized() {
            return self.fetch_time_raw().await;
        }

        // Return the estimated server timestamp using cached offset
        Ok(self.time_sync.get_server_timestamp())
    }

    /// Synchronizes local time with Binance server time.
    ///
    /// This method fetches the current server time from Binance and updates
    /// the cached time offset. The offset is calculated as:
    /// `offset = server_time - local_time`
    ///
    /// # When to Use
    ///
    /// - Called automatically by `get_signing_timestamp()` when resync is needed
    /// - Can be called manually to force a time synchronization
    /// - Useful after receiving timestamp-related errors from the API
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on successful synchronization.
    ///
    /// # Errors
    ///
    /// Returns an error if the server time fetch fails due to network issues.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use ccxt_exchanges::binance::Binance;
    /// # use ccxt_core::ExchangeConfig;
    /// # async fn example() -> ccxt_core::Result<()> {
    /// let binance = Binance::new(ExchangeConfig::default())?;
    ///
    /// // Manually sync time with server
    /// binance.sync_time().await?;
    ///
    /// // Check the current offset
    /// let offset = binance.time_sync().get_offset();
    /// println!("Time offset: {}ms", offset);
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// _Requirements: 2.1, 2.2, 6.3_
    pub async fn sync_time(&self) -> ccxt_core::Result<()> {
        let server_time = self.fetch_time_raw().await?;
        self.time_sync.update_offset(server_time);
        tracing::debug!(
            server_time = server_time,
            offset = self.time_sync.get_offset(),
            "Time synchronized with Binance server"
        );
        Ok(())
    }

    /// Checks if an error is related to timestamp validation.
    ///
    /// Binance returns specific error codes and messages when the request
    /// timestamp is outside the acceptable window (recvWindow). This method
    /// detects such errors to enable automatic retry with a fresh timestamp.
    ///
    /// # Binance Timestamp Error Codes
    ///
    /// | Error Code | Message | Meaning |
    /// |------------|---------|---------|
    /// | -1021 | "Timestamp for this request is outside of the recvWindow" | Timestamp too old or too new |
    /// | -1022 | "Signature for this request is not valid" | May be caused by wrong timestamp |
    ///
    /// # Arguments
    ///
    /// * `error` - The error to check
    ///
    /// # Returns
    ///
    /// Returns `true` if the error is related to timestamp validation, `false` otherwise.
    ///
    /// # Example
    ///
    /// ```rust
    /// use ccxt_exchanges::binance::Binance;
    /// use ccxt_core::{ExchangeConfig, Error};
    ///
    /// let binance = Binance::new(ExchangeConfig::default()).unwrap();
    ///
    /// // Simulate a timestamp error
    /// let err = Error::exchange("-1021", "Timestamp for this request is outside of the recvWindow");
    /// assert!(binance.is_timestamp_error(&err));
    ///
    /// // Non-timestamp error
    /// let err = Error::exchange("-1100", "Illegal characters found in parameter");
    /// assert!(!binance.is_timestamp_error(&err));
    /// ```
    ///
    /// _Requirements: 4.1_
    pub fn is_timestamp_error(&self, error: &ccxt_core::Error) -> bool {
        let error_str = error.to_string().to_lowercase();

        // Check for timestamp-related keywords in the error message
        let has_timestamp_keyword = error_str.contains("timestamp");
        let has_recv_window = error_str.contains("recvwindow");
        let has_ahead = error_str.contains("ahead");
        let has_behind = error_str.contains("behind");

        // Timestamp error if it mentions timestamp AND one of the time-related issues
        if has_timestamp_keyword && (has_recv_window || has_ahead || has_behind) {
            return true;
        }

        // Also check for specific Binance error codes
        // -1021: Timestamp for this request is outside of the recvWindow
        // -1022: Signature for this request is not valid (may be timestamp-related)
        if error_str.contains("-1021") {
            return true;
        }

        // Check for error code in Exchange variant
        if let ccxt_core::Error::Exchange(details) = error {
            if details.code == "-1021" {
                return true;
            }
        }

        false
    }

    /// Executes a signed request with automatic timestamp error recovery.
    ///
    /// This method wraps a signed request operation and automatically handles
    /// timestamp-related errors by resyncing time and retrying the request once.
    ///
    /// # Error Recovery Flow
    ///
    /// 1. Execute the request with the current timestamp
    /// 2. If the request fails with a timestamp error:
    ///    a. Resync time with the server
    ///    b. Retry the request with a fresh timestamp
    /// 3. If the retry also fails, return the error
    ///
    /// # Retry Limit
    ///
    /// To prevent infinite retry loops, this method limits automatic retries to
    /// exactly 1 retry per request. If the retry also fails, the error is returned.
    ///
    /// # Type Parameters
    ///
    /// * `T` - The return type of the request
    /// * `F` - The async function that performs the signed request
    ///
    /// # Arguments
    ///
    /// * `request_fn` - An async function that takes a timestamp (i64) and returns
    ///   a `Result<T>`. This function should perform the actual signed API request.
    ///
    /// # Returns
    ///
    /// Returns `Ok(T)` on success, or `Err` if the request fails after retry.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use ccxt_exchanges::binance::Binance;
    /// # use ccxt_core::ExchangeConfig;
    /// # async fn example() -> ccxt_core::Result<()> {
    /// let binance = Binance::new(ExchangeConfig::default())?;
    ///
    /// // Execute a signed request with automatic retry on timestamp error
    /// let result = binance.execute_signed_request_with_retry(|timestamp| {
    ///     Box::pin(async move {
    ///         // Perform signed request using the provided timestamp
    ///         // ... actual request logic here ...
    ///         Ok(())
    ///     })
    /// }).await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// _Requirements: 4.1, 4.2, 4.3, 4.4_
    pub async fn execute_signed_request_with_retry<T, F, Fut>(
        &self,
        request_fn: F,
    ) -> ccxt_core::Result<T>
    where
        F: Fn(i64) -> Fut,
        Fut: std::future::Future<Output = ccxt_core::Result<T>>,
    {
        // Get the initial timestamp
        let timestamp = self.get_signing_timestamp().await?;

        // Execute the request
        match request_fn(timestamp).await {
            Ok(result) => Ok(result),
            Err(e) if self.is_timestamp_error(&e) => {
                // Log the timestamp error and retry
                tracing::warn!(
                    error = %e,
                    "Timestamp error detected, resyncing time and retrying request"
                );

                // Force resync time with server
                if let Err(sync_err) = self.sync_time().await {
                    tracing::error!(
                        error = %sync_err,
                        "Failed to resync time after timestamp error"
                    );
                    // Return the original error if sync fails
                    return Err(e);
                }

                // Get fresh timestamp after resync
                let new_timestamp = self.time_sync.get_server_timestamp();

                tracing::debug!(
                    old_timestamp = timestamp,
                    new_timestamp = new_timestamp,
                    offset = self.time_sync.get_offset(),
                    "Retrying request with fresh timestamp"
                );

                // Retry the request once with the new timestamp
                request_fn(new_timestamp).await
            }
            Err(e) => Err(e),
        }
    }

    /// Helper method to handle timestamp errors and trigger resync.
    ///
    /// This method checks if an error is a timestamp error and, if so,
    /// triggers a time resync. It's useful for manual error handling
    /// when you want more control over the retry logic.
    ///
    /// # Arguments
    ///
    /// * `error` - The error to check
    ///
    /// # Returns
    ///
    /// Returns `true` if the error was a timestamp error and resync was triggered,
    /// `false` otherwise.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use ccxt_exchanges::binance::Binance;
    /// # use ccxt_core::ExchangeConfig;
    /// # async fn example() -> ccxt_core::Result<()> {
    /// let binance = Binance::new(ExchangeConfig::default())?;
    ///
    /// // Manual error handling with resync
    /// let result = some_signed_request().await;
    /// if let Err(ref e) = result {
    ///     if binance.handle_timestamp_error_and_resync(e).await {
    ///         // Timestamp error detected and resync triggered
    ///         // You can now retry the request
    ///     }
    /// }
    /// # async fn some_signed_request() -> ccxt_core::Result<()> { Ok(()) }
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// _Requirements: 4.1, 4.2_
    pub async fn handle_timestamp_error_and_resync(&self, error: &ccxt_core::Error) -> bool {
        if self.is_timestamp_error(error) {
            tracing::warn!(
                error = %error,
                "Timestamp error detected, triggering time resync"
            );

            if let Err(sync_err) = self.sync_time().await {
                tracing::error!(
                    error = %sync_err,
                    "Failed to resync time after timestamp error"
                );
                return false;
            }

            tracing::debug!(
                offset = self.time_sync.get_offset(),
                "Time resync completed after timestamp error"
            );

            return true;
        }

        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ccxt_core::ExchangeConfig;

    #[test]
    fn test_binance_exchange_trait_metadata() {
        let config = ExchangeConfig::default();
        let binance = Binance::new(config).unwrap();

        // Test metadata methods via Exchange trait
        let exchange: &dyn Exchange = &binance;

        assert_eq!(exchange.id(), "binance");
        assert_eq!(exchange.name(), "Binance");
        assert_eq!(exchange.version(), "v3");
        assert!(exchange.certified());
        assert!(exchange.has_websocket());
    }

    #[test]
    fn test_binance_exchange_trait_capabilities() {
        let config = ExchangeConfig::default();
        let binance = Binance::new(config).unwrap();

        let exchange: &dyn Exchange = &binance;
        let caps = exchange.capabilities();

        assert!(caps.fetch_markets());
        assert!(caps.fetch_ticker());
        assert!(caps.create_order());
        assert!(caps.websocket());
        assert!(!caps.edit_order()); // Binance doesn't support order editing
    }

    #[test]
    fn test_binance_exchange_trait_timeframes() {
        let config = ExchangeConfig::default();
        let binance = Binance::new(config).unwrap();

        let exchange: &dyn Exchange = &binance;
        let timeframes = exchange.timeframes();

        assert!(!timeframes.is_empty());
        assert!(timeframes.contains(&Timeframe::M1));
        assert!(timeframes.contains(&Timeframe::H1));
        assert!(timeframes.contains(&Timeframe::D1));
    }

    #[test]
    fn test_binance_exchange_trait_object_safety() {
        let config = ExchangeConfig::default();
        let binance = Binance::new(config).unwrap();

        // Test that we can create a trait object
        let exchange: Box<dyn Exchange> = Box::new(binance);

        assert_eq!(exchange.id(), "binance");
        assert_eq!(exchange.rate_limit(), 50);
    }

    #[test]
    fn test_binance_exchange_ext_trait() {
        let config = ExchangeConfig::default();
        let binance = Binance::new(config).unwrap();

        // Test ExchangeExt methods
        let exchange: &dyn Exchange = &binance;

        // Binance supports all capabilities
        assert!(
            exchange.supports_market_data(),
            "Binance should support market data"
        );
        assert!(
            exchange.supports_trading(),
            "Binance should support trading"
        );
        assert!(
            exchange.supports_account(),
            "Binance should support account operations"
        );
        assert!(
            exchange.supports_margin(),
            "Binance should support margin operations"
        );
        assert!(
            exchange.supports_funding(),
            "Binance should support funding operations"
        );
    }

    #[test]
    fn test_binance_implements_both_exchange_and_public_exchange() {
        let config = ExchangeConfig::default();
        let binance = Binance::new(config).unwrap();

        // Test that Binance can be used as both Exchange and PublicExchange
        let exchange: &dyn Exchange = &binance;
        let public_exchange: &dyn PublicExchange = &binance;

        // Both should return the same values
        assert_eq!(exchange.id(), public_exchange.id());
        assert_eq!(exchange.name(), public_exchange.name());
        assert_eq!(exchange.version(), public_exchange.version());
        assert_eq!(exchange.certified(), public_exchange.certified());
        assert_eq!(exchange.rate_limit(), public_exchange.rate_limit());
        assert_eq!(exchange.has_websocket(), public_exchange.has_websocket());
        assert_eq!(exchange.timeframes(), public_exchange.timeframes());
    }

    // ==================== Time Sync Helper Tests ====================

    #[test]
    fn test_is_timestamp_error_with_recv_window_message() {
        let config = ExchangeConfig::default();
        let binance = Binance::new(config).unwrap();

        // Test with recvWindow error message
        let err = ccxt_core::Error::exchange(
            "-1021",
            "Timestamp for this request is outside of the recvWindow",
        );
        assert!(
            binance.is_timestamp_error(&err),
            "Should detect recvWindow timestamp error"
        );
    }

    #[test]
    fn test_is_timestamp_error_with_ahead_message() {
        let config = ExchangeConfig::default();
        let binance = Binance::new(config).unwrap();

        // Test with "ahead" error message
        let err = ccxt_core::Error::exchange("-1021", "Timestamp is ahead of server time");
        assert!(
            binance.is_timestamp_error(&err),
            "Should detect 'ahead' timestamp error"
        );
    }

    #[test]
    fn test_is_timestamp_error_with_behind_message() {
        let config = ExchangeConfig::default();
        let binance = Binance::new(config).unwrap();

        // Test with "behind" error message
        let err = ccxt_core::Error::exchange("-1021", "Timestamp is behind server time");
        assert!(
            binance.is_timestamp_error(&err),
            "Should detect 'behind' timestamp error"
        );
    }

    #[test]
    fn test_is_timestamp_error_with_error_code_only() {
        let config = ExchangeConfig::default();
        let binance = Binance::new(config).unwrap();

        // Test with error code -1021 in message
        let err = ccxt_core::Error::exchange("-1021", "Some error message");
        assert!(
            binance.is_timestamp_error(&err),
            "Should detect error code -1021"
        );
    }

    #[test]
    fn test_is_timestamp_error_non_timestamp_error() {
        let config = ExchangeConfig::default();
        let binance = Binance::new(config).unwrap();

        // Test with non-timestamp error
        let err = ccxt_core::Error::exchange("-1100", "Illegal characters found in parameter");
        assert!(
            !binance.is_timestamp_error(&err),
            "Should not detect non-timestamp error"
        );

        // Test with authentication error
        let err = ccxt_core::Error::authentication("Invalid API key");
        assert!(
            !binance.is_timestamp_error(&err),
            "Should not detect authentication error as timestamp error"
        );

        // Test with network error
        let err = ccxt_core::Error::network("Connection refused");
        assert!(
            !binance.is_timestamp_error(&err),
            "Should not detect network error as timestamp error"
        );
    }

    #[test]
    fn test_is_timestamp_error_case_insensitive() {
        let config = ExchangeConfig::default();
        let binance = Binance::new(config).unwrap();

        // Test case insensitivity
        let err = ccxt_core::Error::exchange(
            "-1021",
            "TIMESTAMP for this request is outside of the RECVWINDOW",
        );
        assert!(
            binance.is_timestamp_error(&err),
            "Should detect timestamp error case-insensitively"
        );
    }

    #[test]
    fn test_time_sync_manager_accessible() {
        let config = ExchangeConfig::default();
        let binance = Binance::new(config).unwrap();

        // Test that time_sync() returns a reference to the manager
        let time_sync = binance.time_sync();
        assert!(
            !time_sync.is_initialized(),
            "Time sync should not be initialized initially"
        );
        assert!(
            time_sync.needs_resync(),
            "Time sync should need resync initially"
        );
    }

    #[test]
    fn test_time_sync_manager_update_offset() {
        let config = ExchangeConfig::default();
        let binance = Binance::new(config).unwrap();

        // Simulate updating the offset
        let server_time = ccxt_core::time::TimestampUtils::now_ms() + 100;
        binance.time_sync().update_offset(server_time);

        assert!(
            binance.time_sync().is_initialized(),
            "Time sync should be initialized after update"
        );
        assert!(
            !binance.time_sync().needs_resync(),
            "Time sync should not need resync immediately after update"
        );

        // Offset should be approximately 100ms
        let offset = binance.time_sync().get_offset();
        assert!(
            offset >= 90 && offset <= 110,
            "Offset should be approximately 100ms, got {}",
            offset
        );
    }

    // ==================== Error Recovery Tests ====================

    #[tokio::test]
    async fn test_execute_signed_request_with_retry_success() {
        let config = ExchangeConfig::default();
        let binance = Binance::new(config).unwrap();

        // Initialize time sync
        let server_time = ccxt_core::time::TimestampUtils::now_ms();
        binance.time_sync().update_offset(server_time);

        // Test successful request (no retry needed)
        let result = binance
            .execute_signed_request_with_retry(|timestamp| async move {
                assert!(timestamp > 0, "Timestamp should be positive");
                Ok::<_, ccxt_core::Error>(42)
            })
            .await;

        assert!(result.is_ok(), "Request should succeed");
        assert_eq!(result.unwrap(), 42);
    }

    #[tokio::test]
    async fn test_execute_signed_request_with_retry_non_timestamp_error() {
        let config = ExchangeConfig::default();
        let binance = Binance::new(config).unwrap();

        // Initialize time sync
        let server_time = ccxt_core::time::TimestampUtils::now_ms();
        binance.time_sync().update_offset(server_time);

        // Test non-timestamp error (should not retry)
        let result = binance
            .execute_signed_request_with_retry(|_timestamp| async move {
                Err::<i32, _>(ccxt_core::Error::exchange("-1100", "Invalid parameter"))
            })
            .await;

        assert!(result.is_err(), "Request should fail");
        let err = result.unwrap_err();
        assert!(
            err.to_string().contains("-1100"),
            "Error should contain original error code"
        );
    }

    #[test]
    fn test_handle_timestamp_error_detection() {
        let config = ExchangeConfig::default();
        let binance = Binance::new(config).unwrap();

        // Test various timestamp error formats
        let timestamp_errors = vec![
            ccxt_core::Error::exchange(
                "-1021",
                "Timestamp for this request is outside of the recvWindow",
            ),
            ccxt_core::Error::exchange("-1021", "Timestamp is ahead of server time"),
            ccxt_core::Error::exchange("-1021", "Timestamp is behind server time"),
            ccxt_core::Error::exchange("-1021", "Some error with timestamp and recvwindow"),
        ];

        for err in timestamp_errors {
            assert!(
                binance.is_timestamp_error(&err),
                "Should detect timestamp error: {}",
                err
            );
        }

        // Test non-timestamp errors
        let non_timestamp_errors = vec![
            ccxt_core::Error::exchange("-1100", "Invalid parameter"),
            ccxt_core::Error::exchange("-1000", "Unknown error"),
            ccxt_core::Error::authentication("Invalid API key"),
            ccxt_core::Error::network("Connection refused"),
            ccxt_core::Error::timeout("Request timed out"),
        ];

        for err in non_timestamp_errors {
            assert!(
                !binance.is_timestamp_error(&err),
                "Should not detect as timestamp error: {}",
                err
            );
        }
    }

    // ==================== Property-Based Tests ====================

    mod property_tests {
        use super::*;
        use proptest::prelude::*;

        // Strategy to generate various ExchangeConfig configurations
        fn arb_exchange_config() -> impl Strategy<Value = ExchangeConfig> {
            (
                prop::bool::ANY,                                                      // sandbox
                prop::option::of(any::<u64>().prop_map(|n| format!("key_{}", n))),    // api_key
                prop::option::of(any::<u64>().prop_map(|n| format!("secret_{}", n))), // secret
            )
                .prop_map(|(sandbox, api_key, secret)| ExchangeConfig {
                    sandbox,
                    api_key,
                    secret,
                    ..Default::default()
                })
        }

        proptest! {
            #![proptest_config(ProptestConfig::with_cases(100))]

            /// **Feature: unified-exchange-trait, Property 8: Timeframes Non-Empty**
            ///
            /// *For any* exchange configuration, calling `timeframes()` should return
            /// a non-empty vector of valid `Timeframe` values.
            ///
            /// **Validates: Requirements 8.4**
            #[test]
            fn prop_timeframes_non_empty(config in arb_exchange_config()) {
                let binance = Binance::new(config).expect("Should create Binance instance");
                let exchange: &dyn Exchange = &binance;

                let timeframes = exchange.timeframes();

                // Property: timeframes should never be empty
                prop_assert!(!timeframes.is_empty(), "Timeframes should not be empty");

                // Property: all timeframes should be valid (no duplicates)
                let mut seen = std::collections::HashSet::new();
                for tf in &timeframes {
                    prop_assert!(
                        seen.insert(tf.clone()),
                        "Timeframes should not contain duplicates: {:?}",
                        tf
                    );
                }

                // Property: should contain common timeframes
                prop_assert!(
                    timeframes.contains(&Timeframe::M1),
                    "Should contain 1-minute timeframe"
                );
                prop_assert!(
                    timeframes.contains(&Timeframe::H1),
                    "Should contain 1-hour timeframe"
                );
                prop_assert!(
                    timeframes.contains(&Timeframe::D1),
                    "Should contain 1-day timeframe"
                );
            }

            /// **Feature: unified-exchange-trait, Property 7: Backward Compatibility**
            ///
            /// *For any* exchange configuration, metadata methods called through the Exchange trait
            /// should return the same values as calling them directly on Binance.
            ///
            /// **Validates: Requirements 3.2, 3.4**
            #[test]
            fn prop_backward_compatibility_metadata(config in arb_exchange_config()) {
                let binance = Binance::new(config).expect("Should create Binance instance");

                // Get trait object reference
                let exchange: &dyn Exchange = &binance;

                // Property: id() should be consistent
                prop_assert_eq!(
                    exchange.id(),
                    Binance::id(&binance),
                    "id() should be consistent between trait and direct call"
                );

                // Property: name() should be consistent
                prop_assert_eq!(
                    exchange.name(),
                    Binance::name(&binance),
                    "name() should be consistent between trait and direct call"
                );

                // Property: version() should be consistent
                prop_assert_eq!(
                    exchange.version(),
                    Binance::version(&binance),
                    "version() should be consistent between trait and direct call"
                );

                // Property: certified() should be consistent
                prop_assert_eq!(
                    exchange.certified(),
                    Binance::certified(&binance),
                    "certified() should be consistent between trait and direct call"
                );

                // Property: rate_limit() should be consistent
                prop_assert_eq!(
                    exchange.rate_limit(),
                    Binance::rate_limit(&binance),
                    "rate_limit() should be consistent between trait and direct call"
                );

                // Property: capabilities should be consistent
                let trait_caps = exchange.capabilities();
                prop_assert!(trait_caps.fetch_markets(), "Should support fetch_markets");
                prop_assert!(trait_caps.fetch_ticker(), "Should support fetch_ticker");
                prop_assert!(trait_caps.websocket(), "Should support websocket");
            }
        }
    }
}
