//! OHLCV request types with Builder pattern.
//!
//! This module provides a type-safe way to construct OHLCV (candlestick) data
//! requests using the Builder pattern with validation.
//!
//! # Examples
//!
//! ```rust
//! use ccxt_core::types::ohlcv_request::{OhlcvRequest, OhlcvRequestBuilder};
//!
//! // Create an OHLCV request using the builder
//! let request = OhlcvRequest::builder()
//!     .symbol("BTC/USDT")
//!     .timeframe("1h")
//!     .limit(100)
//!     .build()
//!     .expect("Valid request");
//!
//! assert_eq!(request.symbol, "BTC/USDT");
//! assert_eq!(request.timeframe, "1h");
//! assert_eq!(request.limit, Some(100));
//! ```

use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};

// ============================================================================
// OhlcvRequest struct
// ============================================================================

/// OHLCV request configuration built via builder pattern.
///
/// This struct contains all the fields needed to fetch OHLCV (candlestick)
/// data from an exchange. Use [`OhlcvRequest::builder()`] to construct
/// instances with validation.
///
/// # Fields
///
/// - `symbol` (required): Trading pair symbol (e.g., "BTC/USDT")
/// - `timeframe` (optional): Candlestick interval, defaults to "1h"
/// - `since` (optional): Start timestamp in milliseconds
/// - `limit` (optional): Maximum number of candles to return
/// - `until` (optional): End timestamp in milliseconds
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OhlcvRequest {
    /// Trading pair symbol (e.g., "BTC/USDT")
    pub symbol: String,

    /// Candlestick timeframe/interval (e.g., "1m", "5m", "1h", "1d")
    pub timeframe: String,

    /// Start timestamp in milliseconds (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub since: Option<i64>,

    /// Maximum number of candles to return (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,

    /// End timestamp in milliseconds (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub until: Option<i64>,
}

impl OhlcvRequest {
    /// Creates a new builder for constructing an OhlcvRequest.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use ccxt_core::types::ohlcv_request::OhlcvRequest;
    ///
    /// let request = OhlcvRequest::builder()
    ///     .symbol("BTC/USDT")
    ///     .build()
    ///     .expect("Valid request");
    ///
    /// // Timeframe defaults to "1h"
    /// assert_eq!(request.timeframe, "1h");
    /// ```
    pub fn builder() -> OhlcvRequestBuilder {
        OhlcvRequestBuilder::new()
    }
}

// ============================================================================
// OhlcvRequestBuilder
// ============================================================================

/// Builder for OhlcvRequest with validation.
///
/// The builder provides a fluent API for constructing OHLCV requests.
/// The `symbol` field is required, while all other fields have sensible
/// defaults or are optional.
///
/// # Default Values
///
/// - `timeframe`: "1h" (1 hour)
/// - `since`: None
/// - `limit`: None
/// - `until`: None
///
/// # Examples
///
/// ```rust
/// use ccxt_core::types::ohlcv_request::OhlcvRequestBuilder;
///
/// // Minimal request (only symbol required)
/// let request = OhlcvRequestBuilder::new()
///     .symbol("ETH/USDT")
///     .build()
///     .expect("Valid request");
///
/// // Full request with all options
/// let request = OhlcvRequestBuilder::new()
///     .symbol("BTC/USDT")
///     .timeframe("15m")
///     .since(1609459200000)
///     .limit(500)
///     .until(1609545600000)
///     .build()
///     .expect("Valid request");
/// ```
#[derive(Debug, Clone, Default)]
pub struct OhlcvRequestBuilder {
    symbol: Option<String>,
    timeframe: Option<String>,
    since: Option<i64>,
    limit: Option<u32>,
    until: Option<i64>,
}

impl OhlcvRequestBuilder {
    /// Creates a new OhlcvRequestBuilder with default values.
    pub fn new() -> Self {
        Self {
            symbol: None,
            timeframe: None,
            since: None,
            limit: None,
            until: None,
        }
    }

    /// Sets the trading symbol (required).
    ///
    /// # Arguments
    ///
    /// * `symbol` - Trading pair symbol (e.g., "BTC/USDT")
    ///
    /// # Examples
    ///
    /// ```rust
    /// use ccxt_core::types::ohlcv_request::OhlcvRequestBuilder;
    ///
    /// let builder = OhlcvRequestBuilder::new()
    ///     .symbol("BTC/USDT");
    /// ```
    pub fn symbol(mut self, symbol: impl Into<String>) -> Self {
        self.symbol = Some(symbol.into());
        self
    }

    /// Sets the candlestick timeframe/interval (optional, defaults to "1h").
    ///
    /// Common timeframes:
    /// - "1m", "3m", "5m", "15m", "30m" (minutes)
    /// - "1h", "2h", "4h", "6h", "8h", "12h" (hours)
    /// - "1d", "3d" (days)
    /// - "1w" (week)
    /// - "1M" (month)
    ///
    /// # Arguments
    ///
    /// * `timeframe` - Candlestick interval string
    ///
    /// # Examples
    ///
    /// ```rust
    /// use ccxt_core::types::ohlcv_request::OhlcvRequestBuilder;
    ///
    /// let builder = OhlcvRequestBuilder::new()
    ///     .symbol("BTC/USDT")
    ///     .timeframe("4h");
    /// ```
    pub fn timeframe(mut self, timeframe: impl Into<String>) -> Self {
        self.timeframe = Some(timeframe.into());
        self
    }

    /// Sets the start timestamp in milliseconds (optional).
    ///
    /// Only candles with timestamp >= since will be returned.
    ///
    /// # Arguments
    ///
    /// * `since` - Unix timestamp in milliseconds
    ///
    /// # Examples
    ///
    /// ```rust
    /// use ccxt_core::types::ohlcv_request::OhlcvRequestBuilder;
    ///
    /// let builder = OhlcvRequestBuilder::new()
    ///     .symbol("BTC/USDT")
    ///     .since(1609459200000); // 2021-01-01 00:00:00 UTC
    /// ```
    pub fn since(mut self, since: i64) -> Self {
        self.since = Some(since);
        self
    }

    /// Sets the maximum number of candles to return (optional).
    ///
    /// Most exchanges have a maximum limit (typically 500-1500).
    ///
    /// # Arguments
    ///
    /// * `limit` - Maximum number of candles
    ///
    /// # Examples
    ///
    /// ```rust
    /// use ccxt_core::types::ohlcv_request::OhlcvRequestBuilder;
    ///
    /// let builder = OhlcvRequestBuilder::new()
    ///     .symbol("BTC/USDT")
    ///     .limit(100);
    /// ```
    pub fn limit(mut self, limit: u32) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Sets the end timestamp in milliseconds (optional).
    ///
    /// Only candles with timestamp <= until will be returned.
    ///
    /// # Arguments
    ///
    /// * `until` - Unix timestamp in milliseconds
    ///
    /// # Examples
    ///
    /// ```rust
    /// use ccxt_core::types::ohlcv_request::OhlcvRequestBuilder;
    ///
    /// let builder = OhlcvRequestBuilder::new()
    ///     .symbol("BTC/USDT")
    ///     .until(1609545600000); // 2021-01-02 00:00:00 UTC
    /// ```
    pub fn until(mut self, until: i64) -> Self {
        self.until = Some(until);
        self
    }

    /// Builds the OhlcvRequest with validation.
    ///
    /// # Returns
    ///
    /// - `Ok(OhlcvRequest)` if the symbol is provided
    /// - `Err(Error)` if the symbol is missing
    ///
    /// # Default Values
    ///
    /// - `timeframe` defaults to "1h" if not specified
    ///
    /// # Examples
    ///
    /// ```rust
    /// use ccxt_core::types::ohlcv_request::OhlcvRequestBuilder;
    ///
    /// // Valid request
    /// let request = OhlcvRequestBuilder::new()
    ///     .symbol("BTC/USDT")
    ///     .build()
    ///     .expect("Valid request");
    ///
    /// // Invalid request (missing symbol)
    /// let result = OhlcvRequestBuilder::new()
    ///     .timeframe("1h")
    ///     .build();
    /// assert!(result.is_err());
    /// ```
    pub fn build(self) -> Result<OhlcvRequest> {
        let symbol = self
            .symbol
            .ok_or_else(|| Error::invalid_argument("symbol is required"))?;

        Ok(OhlcvRequest {
            symbol,
            timeframe: self.timeframe.unwrap_or_else(|| "1h".to_string()),
            since: self.since,
            limit: self.limit,
            until: self.until,
        })
    }
}

// ============================================================================
// Unit Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ohlcv_request_builder_minimal() {
        let request = OhlcvRequest::builder()
            .symbol("BTC/USDT")
            .build()
            .expect("Valid request");

        assert_eq!(request.symbol, "BTC/USDT");
        assert_eq!(request.timeframe, "1h"); // Default
        assert!(request.since.is_none());
        assert!(request.limit.is_none());
        assert!(request.until.is_none());
    }

    #[test]
    fn test_ohlcv_request_builder_with_all_fields() {
        let request = OhlcvRequest::builder()
            .symbol("ETH/USDT")
            .timeframe("15m")
            .since(1609459200000)
            .limit(500)
            .until(1609545600000)
            .build()
            .expect("Valid request");

        assert_eq!(request.symbol, "ETH/USDT");
        assert_eq!(request.timeframe, "15m");
        assert_eq!(request.since, Some(1609459200000));
        assert_eq!(request.limit, Some(500));
        assert_eq!(request.until, Some(1609545600000));
    }

    #[test]
    fn test_ohlcv_request_builder_missing_symbol() {
        let result = OhlcvRequest::builder().timeframe("1h").limit(100).build();

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("symbol is required"));
    }

    #[test]
    fn test_ohlcv_request_builder_default_timeframe() {
        let request = OhlcvRequest::builder()
            .symbol("BTC/USDT")
            .build()
            .expect("Valid request");

        assert_eq!(request.timeframe, "1h");
    }

    #[test]
    fn test_ohlcv_request_builder_custom_timeframe() {
        let request = OhlcvRequest::builder()
            .symbol("BTC/USDT")
            .timeframe("4h")
            .build()
            .expect("Valid request");

        assert_eq!(request.timeframe, "4h");
    }

    #[test]
    fn test_ohlcv_request_builder_any_order() {
        // Test that fields can be set in any order
        let request1 = OhlcvRequest::builder()
            .symbol("BTC/USDT")
            .timeframe("1h")
            .limit(100)
            .build()
            .expect("Valid request");

        let request2 = OhlcvRequest::builder()
            .limit(100)
            .timeframe("1h")
            .symbol("BTC/USDT")
            .build()
            .expect("Valid request");

        assert_eq!(request1.symbol, request2.symbol);
        assert_eq!(request1.timeframe, request2.timeframe);
        assert_eq!(request1.limit, request2.limit);
    }

    #[test]
    fn test_ohlcv_request_builder_with_since_only() {
        let request = OhlcvRequest::builder()
            .symbol("BTC/USDT")
            .since(1609459200000)
            .build()
            .expect("Valid request");

        assert_eq!(request.since, Some(1609459200000));
        assert!(request.until.is_none());
    }

    #[test]
    fn test_ohlcv_request_builder_with_until_only() {
        let request = OhlcvRequest::builder()
            .symbol("BTC/USDT")
            .until(1609545600000)
            .build()
            .expect("Valid request");

        assert!(request.since.is_none());
        assert_eq!(request.until, Some(1609545600000));
    }

    #[test]
    fn test_ohlcv_request_builder_with_time_range() {
        let request = OhlcvRequest::builder()
            .symbol("BTC/USDT")
            .since(1609459200000)
            .until(1609545600000)
            .build()
            .expect("Valid request");

        assert_eq!(request.since, Some(1609459200000));
        assert_eq!(request.until, Some(1609545600000));
    }

    #[test]
    fn test_ohlcv_request_serialization() {
        let request = OhlcvRequest::builder()
            .symbol("BTC/USDT")
            .timeframe("1h")
            .limit(100)
            .build()
            .expect("Valid request");

        // Test serialization
        let json = serde_json::to_string(&request).expect("Serialization should succeed");
        assert!(json.contains("BTC/USDT"));
        assert!(json.contains("1h"));
        assert!(json.contains("100"));

        // Test deserialization
        let deserialized: OhlcvRequest =
            serde_json::from_str(&json).expect("Deserialization should succeed");
        assert_eq!(deserialized.symbol, request.symbol);
        assert_eq!(deserialized.timeframe, request.timeframe);
        assert_eq!(deserialized.limit, request.limit);
    }

    #[test]
    fn test_ohlcv_request_builder_default() {
        let builder = OhlcvRequestBuilder::default();
        let result = builder.build();
        assert!(result.is_err()); // Symbol is required
    }

    #[test]
    fn test_ohlcv_request_various_timeframes() {
        let timeframes = vec!["1m", "5m", "15m", "30m", "1h", "4h", "1d", "1w", "1M"];

        for tf in timeframes {
            let request = OhlcvRequest::builder()
                .symbol("BTC/USDT")
                .timeframe(tf)
                .build()
                .expect("Valid request");

            assert_eq!(request.timeframe, tf);
        }
    }
}
