//! Time utilities for CCXT
//!
//! This module provides comprehensive time-related utility functions for timestamp conversion,
//! date parsing, formatting, and validation. All timestamps are standardized to use `i64` type
//! representing milliseconds since Unix epoch unless otherwise specified.
//!
//! # Key Features
//!
//! - **Standardized i64 timestamps**: All timestamps use `i64` for consistency
//! - **Comprehensive validation**: Range checking and overflow protection
//! - **Migration support**: Conversion utilities for u64 to i64 migration
//! - **Multiple format support**: ISO 8601, space-separated, and custom formats
//! - **UTC timezone handling**: All operations in UTC for consistency
//! - **Error handling**: Proper error types for all failure modes
//!
//! # Timestamp Format Standard
//!
//! - **Type**: `i64`
//! - **Unit**: Milliseconds since Unix Epoch (January 1, 1970, 00:00:00 UTC)
//! - **Range**: 0 to ~292,277,026,596 (year ~294,276)
//! - **Validation**: Must be >= 0 for valid Unix timestamps
//!
//! # Example
//!
//! ```rust
//! use ccxt_core::time::{TimestampUtils, milliseconds, iso8601, parse_date};
//!
//! // Get current timestamp in milliseconds
//! let now = milliseconds();
//!
//! // Validate timestamp
//! let validated = TimestampUtils::validate_timestamp(now).unwrap();
//!
//! // Convert timestamp to ISO 8601 string
//! let iso_str = iso8601(validated).unwrap();
//!
//! // Parse ISO 8601 string back to timestamp
//! let parsed = parse_date(&iso_str).unwrap();
//! assert_eq!(validated, parsed);
//!
//! // Migration support: convert u64 to i64
//! let old_timestamp: u64 = 1704110400000;
//! let new_timestamp = TimestampUtils::u64_to_i64(old_timestamp).unwrap();
//! ```

use crate::error::{Error, ParseError, Result};
use chrono::{DateTime, NaiveDateTime, TimeZone, Utc};
use std::time::{SystemTime, UNIX_EPOCH};

/// Returns the current time in milliseconds since the Unix epoch
///
/// This function is now a wrapper around `TimestampUtils::now_ms()` for consistency.
/// Consider using `TimestampUtils::now_ms()` directly for new code.
///
/// # Example
///
/// ```rust
/// use ccxt_core::time::milliseconds;
///
/// let now = milliseconds();
/// assert!(now > 0);
/// ```
#[inline]
pub fn milliseconds() -> i64 {
    TimestampUtils::now_ms()
}

/// Returns the current time in seconds since the Unix epoch
///
/// This function is now a wrapper around `TimestampUtils` for consistency.
/// Consider using `TimestampUtils::ms_to_seconds(TimestampUtils::now_ms())` for new code.
///
/// # Example
///
/// ```rust
/// use ccxt_core::time::seconds;
///
/// let now = seconds();
/// assert!(now > 0);
/// ```
#[inline]
pub fn seconds() -> i64 {
    TimestampUtils::ms_to_seconds(TimestampUtils::now_ms())
}

/// Returns the current time in microseconds since the Unix epoch
///
/// # Example
///
/// ```rust
/// use ccxt_core::time::microseconds;
///
/// let now = microseconds();
/// assert!(now > 0);
/// ```
#[inline]
pub fn microseconds() -> i64 {
    Utc::now().timestamp_micros()
}

/// Converts a timestamp in milliseconds to an ISO 8601 formatted string
///
/// This function now uses `TimestampUtils::format_iso8601()` internally for consistency.
/// Consider using `TimestampUtils::format_iso8601()` directly for new code.
///
/// # Arguments
///
/// * `timestamp` - Timestamp in milliseconds since Unix epoch
///
/// # Returns
///
/// ISO 8601 formatted string in UTC timezone (e.g., "2024-01-01T12:00:00.000Z")
///
/// # Example
///
/// ```rust
/// use ccxt_core::time::iso8601;
///
/// let timestamp = 1704110400000; // 2024-01-01 12:00:00 UTC
/// let iso_str = iso8601(timestamp).unwrap();
/// assert_eq!(iso_str, "2024-01-01T12:00:00.000Z");
/// ```
pub fn iso8601(timestamp: i64) -> Result<String> {
    TimestampUtils::format_iso8601(timestamp)
}

/// Parses a date string and returns the timestamp in milliseconds since Unix epoch
///
/// Supports multiple date formats:
/// - ISO 8601: "2024-01-01T12:00:00.000Z" or "2024-01-01T12:00:00Z"
/// - Space-separated: "2024-01-01 12:00:00"
/// - Without timezone: "2024-01-01T12:00:00.389"
///
/// # Arguments
///
/// * `datetime` - Date string in one of the supported formats
///
/// # Returns
///
/// Timestamp in milliseconds since Unix epoch
///
/// # Example
///
/// ```rust
/// use ccxt_core::time::parse_date;
///
/// let ts1 = parse_date("2024-01-01T12:00:00.000Z").unwrap();
/// let ts2 = parse_date("2024-01-01 12:00:00").unwrap();
/// assert!(ts1 > 0);
/// assert!(ts2 > 0);
/// ```
pub fn parse_date(datetime: &str) -> Result<i64> {
    if datetime.is_empty() {
        return Err(ParseError::timestamp("Empty datetime string").into());
    }

    // List of supported date formats
    let formats = [
        "%Y-%m-%d %H:%M:%S",      // "2024-01-01 12:00:00"
        "%Y-%m-%dT%H:%M:%S%.3fZ", // "2024-01-01T12:00:00.000Z"
        "%Y-%m-%dT%H:%M:%SZ",     // "2024-01-01T12:00:00Z"
        "%Y-%m-%dT%H:%M:%S%.3f",  // "2024-01-01T12:00:00.389"
        "%Y-%m-%dT%H:%M:%S",      // "2024-01-01T12:00:00"
        "%Y-%m-%d %H:%M:%S%.3f",  // "2024-01-01 12:00:00.389"
    ];

    // Try parsing with each format
    for format in &formats {
        if let Ok(naive) = NaiveDateTime::parse_from_str(datetime, format) {
            let dt = Utc.from_utc_datetime(&naive);
            return Ok(dt.timestamp_millis());
        }
    }

    // Try parsing as RFC3339 (handles timezone offsets)
    if let Ok(dt) = DateTime::parse_from_rfc3339(datetime) {
        return Ok(dt.timestamp_millis());
    }

    Err(ParseError::timestamp_owned(format!("Unable to parse datetime: {}", datetime)).into())
}

/// Parses an ISO 8601 date string and returns the timestamp in milliseconds
///
/// This function handles various ISO 8601 formats and strips timezone offsets
/// like "+00:00" before parsing.
///
/// # Arguments
///
/// * `datetime` - ISO 8601 formatted date string
///
/// # Returns
///
/// Timestamp in milliseconds since Unix epoch
///
/// # Example
///
/// ```rust
/// use ccxt_core::time::parse_iso8601;
///
/// let ts = parse_iso8601("2024-01-01T12:00:00.000Z").unwrap();
/// assert!(ts > 0);
///
/// let ts2 = parse_iso8601("2024-01-01T12:00:00+00:00").unwrap();
/// assert!(ts2 > 0);
/// ```
pub fn parse_iso8601(datetime: &str) -> Result<i64> {
    if datetime.is_empty() {
        return Err(ParseError::timestamp("Empty datetime string").into());
    }

    // Remove "+00:00" or similar timezone offsets if present
    let cleaned = if datetime.contains("+0") {
        datetime.split('+').next().unwrap_or(datetime)
    } else {
        datetime
    };

    // Try RFC3339 format first
    if let Ok(dt) = DateTime::parse_from_rfc3339(cleaned) {
        return Ok(dt.timestamp_millis());
    }

    // Try parsing without timezone
    let formats = [
        "%Y-%m-%dT%H:%M:%S%.3f", // "2024-01-01T12:00:00.389"
        "%Y-%m-%d %H:%M:%S%.3f", // "2024-01-01 12:00:43.928"
        "%Y-%m-%dT%H:%M:%S",     // "2024-01-01T12:00:00"
        "%Y-%m-%d %H:%M:%S",     // "2024-01-01 12:00:00"
    ];

    for format in &formats {
        if let Ok(naive) = NaiveDateTime::parse_from_str(cleaned, format) {
            let dt = Utc.from_utc_datetime(&naive);
            return Ok(dt.timestamp_millis());
        }
    }

    Err(
        ParseError::timestamp_owned(format!("Unable to parse ISO 8601 datetime: {}", datetime))
            .into(),
    )
}

/// Formats a timestamp as "yyyy-MM-dd HH:mm:ss"
///
/// This function now includes validation using `TimestampUtils::validate_timestamp()`.
///
/// # Arguments
///
/// * `timestamp` - Timestamp in milliseconds since Unix epoch
/// * `separator` - Optional separator between date and time (defaults to " ")
///
/// # Returns
///
/// Formatted date string
///
/// # Example
///
/// ```rust
/// use ccxt_core::time::ymdhms;
///
/// let ts = 1704110400000; // 2024-01-01 12:00:00 UTC
/// let formatted = ymdhms(ts, None).unwrap();
/// assert_eq!(formatted, "2024-01-01 12:00:00");
///
/// let formatted_t = ymdhms(ts, Some("T")).unwrap();
/// assert_eq!(formatted_t, "2024-01-01T12:00:00");
/// ```
pub fn ymdhms(timestamp: i64, separator: Option<&str>) -> Result<String> {
    let validated = TimestampUtils::validate_timestamp(timestamp)?;

    let sep = separator.unwrap_or(" ");
    let secs = validated / 1000;
    let nsecs = ((validated % 1000) * 1_000_000) as u32;

    let datetime = DateTime::<Utc>::from_timestamp(secs, nsecs)
        .ok_or_else(|| Error::invalid_request(format!("Invalid timestamp: {}", validated)))?;

    Ok(format!(
        "{}{}{}",
        datetime.format("%Y-%m-%d"),
        sep,
        datetime.format("%H:%M:%S")
    ))
}

/// Formats a timestamp as "yyyy-MM-dd"
///
/// This function now includes validation using `TimestampUtils::validate_timestamp()`.
///
/// # Arguments
///
/// * `timestamp` - Timestamp in milliseconds since Unix epoch
/// * `separator` - Optional separator between year, month, day (defaults to "-")
///
/// # Example
///
/// ```rust
/// use ccxt_core::time::yyyymmdd;
///
/// let ts = 1704110400000;
/// let formatted = yyyymmdd(ts, None).unwrap();
/// assert_eq!(formatted, "2024-01-01");
///
/// let formatted_slash = yyyymmdd(ts, Some("/")).unwrap();
/// assert_eq!(formatted_slash, "2024/01/01");
/// ```
pub fn yyyymmdd(timestamp: i64, separator: Option<&str>) -> Result<String> {
    let validated = TimestampUtils::validate_timestamp(timestamp)?;

    let sep = separator.unwrap_or("-");
    let secs = validated / 1000;
    let nsecs = ((validated % 1000) * 1_000_000) as u32;

    let datetime = DateTime::<Utc>::from_timestamp(secs, nsecs)
        .ok_or_else(|| Error::invalid_request(format!("Invalid timestamp: {}", validated)))?;

    Ok(format!(
        "{}{}{}{}{}",
        datetime.format("%Y"),
        sep,
        datetime.format("%m"),
        sep,
        datetime.format("%d")
    ))
}

/// Formats a timestamp as "yy-MM-dd"
///
/// This function now includes validation using `TimestampUtils::validate_timestamp()`.
///
/// # Arguments
///
/// * `timestamp` - Timestamp in milliseconds since Unix epoch
/// * `separator` - Optional separator between year, month, day (defaults to "")
///
/// # Example
///
/// ```rust
/// use ccxt_core::time::yymmdd;
///
/// let ts = 1704110400000;
/// let formatted = yymmdd(ts, None).unwrap();
/// assert_eq!(formatted, "240101");
///
/// let formatted_dash = yymmdd(ts, Some("-")).unwrap();
/// assert_eq!(formatted_dash, "24-01-01");
/// ```
pub fn yymmdd(timestamp: i64, separator: Option<&str>) -> Result<String> {
    let validated = TimestampUtils::validate_timestamp(timestamp)?;

    let sep = separator.unwrap_or("");
    let secs = validated / 1000;
    let nsecs = ((validated % 1000) * 1_000_000) as u32;

    let datetime = DateTime::<Utc>::from_timestamp(secs, nsecs)
        .ok_or_else(|| Error::invalid_request(format!("Invalid timestamp: {}", validated)))?;

    Ok(format!(
        "{}{}{}{}{}",
        datetime.format("%y"),
        sep,
        datetime.format("%m"),
        sep,
        datetime.format("%d")
    ))
}

/// Alias for `yyyymmdd` function
///
/// # Example
///
/// ```rust
/// use ccxt_core::time::ymd;
///
/// let ts = 1704110400000;
/// let formatted = ymd(ts, None).unwrap();
/// assert_eq!(formatted, "2024-01-01");
/// ```
#[inline]
pub fn ymd(timestamp: i64, separator: Option<&str>) -> Result<String> {
    yyyymmdd(timestamp, separator)
}

/// Comprehensive timestamp utilities for consistent i64 handling
///
/// This struct provides a collection of utility functions for working with i64 timestamps,
/// including validation, conversion, and formatting operations. All functions are designed
/// to work with milliseconds since Unix epoch.
///
/// # Design Principles
///
/// - **Type Safety**: All operations use i64 for consistency
/// - **Validation**: Range checking prevents invalid timestamps
/// - **Migration Support**: Conversion utilities for u64 to i64 migration
/// - **Error Handling**: Proper error types for all failure modes
///
/// # Example
///
/// ```rust
/// use ccxt_core::time::TimestampUtils;
///
/// // Get current timestamp
/// let now = TimestampUtils::now_ms();
///
/// // Validate timestamp
/// let validated = TimestampUtils::validate_timestamp(now).unwrap();
///
/// // Convert units
/// let seconds = TimestampUtils::ms_to_seconds(validated);
/// let back_to_ms = TimestampUtils::seconds_to_ms(seconds);
/// assert_eq!(validated, back_to_ms);
/// ```
pub struct TimestampUtils;

impl TimestampUtils {
    /// Maximum reasonable timestamp (year 2100) to prevent far-future timestamps
    pub const YEAR_2100_MS: i64 = 4_102_444_800_000;

    /// Minimum reasonable timestamp (year 1970) - Unix epoch start
    pub const UNIX_EPOCH_MS: i64 = 0;

    /// Get current timestamp in milliseconds as i64
    ///
    /// Returns the current system time as milliseconds since Unix epoch.
    /// This is the primary function for getting current timestamps in the library.
    ///
    /// # Example
    ///
    /// ```rust
    /// use ccxt_core::time::TimestampUtils;
    ///
    /// let now = TimestampUtils::now_ms();
    /// assert!(now > 1_600_000_000_000); // After 2020
    /// ```
    pub fn now_ms() -> i64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_millis() as i64
    }

    /// Convert seconds to milliseconds
    ///
    /// # Arguments
    ///
    /// * `seconds` - Timestamp in seconds since Unix epoch
    ///
    /// # Returns
    ///
    /// Timestamp in milliseconds since Unix epoch
    ///
    /// # Example
    ///
    /// ```rust
    /// use ccxt_core::time::TimestampUtils;
    ///
    /// let seconds = 1704110400; // 2024-01-01 12:00:00 UTC
    /// let milliseconds = TimestampUtils::seconds_to_ms(seconds);
    /// assert_eq!(milliseconds, 1704110400000);
    /// ```
    pub fn seconds_to_ms(seconds: i64) -> i64 {
        seconds.saturating_mul(1000)
    }

    /// Convert milliseconds to seconds
    ///
    /// # Arguments
    ///
    /// * `ms` - Timestamp in milliseconds since Unix epoch
    ///
    /// # Returns
    ///
    /// Timestamp in seconds since Unix epoch
    ///
    /// # Example
    ///
    /// ```rust
    /// use ccxt_core::time::TimestampUtils;
    ///
    /// let milliseconds = 1704110400000; // 2024-01-01 12:00:00 UTC
    /// let seconds = TimestampUtils::ms_to_seconds(milliseconds);
    /// assert_eq!(seconds, 1704110400);
    /// ```
    pub fn ms_to_seconds(ms: i64) -> i64 {
        ms / 1000
    }

    /// Validate timestamp is within reasonable bounds
    ///
    /// Ensures the timestamp is:
    /// - Not negative (valid Unix timestamp)
    /// - Not too far in the future (before year 2100)
    ///
    /// # Arguments
    ///
    /// * `timestamp` - Timestamp in milliseconds to validate
    ///
    /// # Returns
    ///
    /// The validated timestamp if valid, or an error if invalid
    ///
    /// # Errors
    ///
    /// - `Error::InvalidRequest` if timestamp is negative
    /// - `Error::InvalidRequest` if timestamp is too far in future
    ///
    /// # Example
    ///
    /// ```rust
    /// use ccxt_core::time::TimestampUtils;
    ///
    /// // Valid timestamp
    /// let valid = TimestampUtils::validate_timestamp(1704110400000).unwrap();
    /// assert_eq!(valid, 1704110400000);
    ///
    /// // Invalid timestamp (negative)
    /// let invalid = TimestampUtils::validate_timestamp(-1);
    /// assert!(invalid.is_err());
    /// ```
    pub fn validate_timestamp(timestamp: i64) -> Result<i64> {
        if timestamp < Self::UNIX_EPOCH_MS {
            return Err(Error::invalid_request("Timestamp cannot be negative"));
        }

        if timestamp > Self::YEAR_2100_MS {
            return Err(Error::invalid_request(
                "Timestamp too far in future (after year 2100)",
            ));
        }

        Ok(timestamp)
    }

    /// Parse timestamp from string (handles various formats)
    ///
    /// Attempts to parse a timestamp from string format, supporting:
    /// - Integer strings (milliseconds): "1704110400000"
    /// - Decimal strings (seconds with fractional part): "1704110400.123"
    /// - Scientific notation: "1.7041104e12"
    ///
    /// # Arguments
    ///
    /// * `s` - String representation of timestamp
    ///
    /// # Returns
    ///
    /// Parsed and validated timestamp in milliseconds
    ///
    /// # Errors
    ///
    /// - `Error::Parse` if string cannot be parsed as number
    /// - `Error::InvalidRequest` if parsed timestamp is invalid
    ///
    /// # Example
    ///
    /// ```rust
    /// use ccxt_core::time::TimestampUtils;
    ///
    /// // Parse integer milliseconds
    /// let ts1 = TimestampUtils::parse_timestamp("1704110400000").unwrap();
    /// assert_eq!(ts1, 1704110400000);
    ///
    /// // Parse decimal seconds
    /// let ts2 = TimestampUtils::parse_timestamp("1704110400.123").unwrap();
    /// assert_eq!(ts2, 1704110400123);
    /// ```
    pub fn parse_timestamp(s: &str) -> Result<i64> {
        if s.is_empty() {
            return Err(Error::invalid_request("Empty timestamp string"));
        }

        // Try parsing as i64 first (milliseconds)
        if let Ok(ts) = s.parse::<i64>() {
            return Self::validate_timestamp(ts);
        }

        // Try parsing as f64 (seconds with fractional part)
        if let Ok(ts_f64) = s.parse::<f64>() {
            if ts_f64.is_finite() {
                let ts = (ts_f64 * 1000.0) as i64;
                return Self::validate_timestamp(ts);
            }
        }

        Err(Error::invalid_request(format!(
            "Invalid timestamp format: {}",
            s
        )))
    }

    /// Format timestamp as ISO 8601 string
    ///
    /// Converts a timestamp to ISO 8601 format with millisecond precision.
    /// Always uses UTC timezone and includes milliseconds.
    ///
    /// # Arguments
    ///
    /// * `timestamp` - Timestamp in milliseconds since Unix epoch
    ///
    /// # Returns
    ///
    /// ISO 8601 formatted string (e.g., "2024-01-01T12:00:00.000Z")
    ///
    /// # Example
    ///
    /// ```rust
    /// use ccxt_core::time::TimestampUtils;
    ///
    /// let timestamp = 1704110400000; // 2024-01-01 12:00:00 UTC
    /// let iso_str = TimestampUtils::format_iso8601(timestamp).unwrap();
    /// assert_eq!(iso_str, "2024-01-01T12:00:00.000Z");
    /// ```
    pub fn format_iso8601(timestamp: i64) -> Result<String> {
        let validated = Self::validate_timestamp(timestamp)?;

        let secs = validated / 1000;
        let nsecs = ((validated % 1000) * 1_000_000) as u32;

        let datetime = DateTime::<Utc>::from_timestamp(secs, nsecs)
            .ok_or_else(|| Error::invalid_request(format!("Invalid timestamp: {}", validated)))?;

        Ok(datetime.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string())
    }

    /// Check if a timestamp represents a reasonable date
    ///
    /// Validates that the timestamp represents a date between 1970 and 2100,
    /// which covers all reasonable use cases for cryptocurrency trading.
    ///
    /// # Arguments
    ///
    /// * `timestamp` - Timestamp in milliseconds to check
    ///
    /// # Returns
    ///
    /// `true` if timestamp is reasonable, `false` otherwise
    ///
    /// # Example
    ///
    /// ```rust
    /// use ccxt_core::time::TimestampUtils;
    ///
    /// assert!(TimestampUtils::is_reasonable_timestamp(1704110400000)); // 2024
    /// assert!(!TimestampUtils::is_reasonable_timestamp(-1)); // Before 1970
    /// ```
    pub fn is_reasonable_timestamp(timestamp: i64) -> bool {
        (Self::UNIX_EPOCH_MS..=Self::YEAR_2100_MS).contains(&timestamp)
    }

    /// Convert u64 to i64 with overflow checking
    ///
    /// This function is provided for migration support when converting from
    /// old u64 timestamp code to new i64 timestamp code. It performs overflow
    /// checking to ensure the conversion is safe.
    ///
    /// # Arguments
    ///
    /// * `timestamp` - u64 timestamp to convert
    ///
    /// # Returns
    ///
    /// Converted i64 timestamp if within valid range
    ///
    /// # Errors
    ///
    /// - `Error::InvalidRequest` if u64 value exceeds i64::MAX
    ///
    /// # Example
    ///
    /// ```rust
    /// use ccxt_core::time::TimestampUtils;
    ///
    /// let old_timestamp: u64 = 1704110400000;
    /// let new_timestamp = TimestampUtils::u64_to_i64(old_timestamp).unwrap();
    /// assert_eq!(new_timestamp, 1704110400000i64);
    /// ```
    #[deprecated(since = "0.1.0", note = "Use i64 timestamps directly")]
    pub fn u64_to_i64(timestamp: u64) -> Result<i64> {
        if timestamp > i64::MAX as u64 {
            return Err(Error::invalid_request(format!(
                "Timestamp overflow: {} exceeds maximum i64 value",
                timestamp
            )));
        }
        let converted = timestamp as i64;
        Self::validate_timestamp(converted)
    }

    /// Convert i64 to u64 with underflow checking
    ///
    /// This function is provided for backward compatibility when interfacing
    /// with legacy code that expects u64 timestamps. It performs underflow
    /// checking to ensure the conversion is safe.
    ///
    /// # Arguments
    ///
    /// * `timestamp` - i64 timestamp to convert
    ///
    /// # Returns
    ///
    /// Converted u64 timestamp if non-negative
    ///
    /// # Errors
    ///
    /// - `Error::InvalidRequest` if i64 value is negative
    ///
    /// # Example
    ///
    /// ```rust
    /// use ccxt_core::time::TimestampUtils;
    ///
    /// let new_timestamp: i64 = 1704110400000;
    /// let old_timestamp = TimestampUtils::i64_to_u64(new_timestamp).unwrap();
    /// assert_eq!(old_timestamp, 1704110400000u64);
    /// ```
    pub fn i64_to_u64(timestamp: i64) -> Result<u64> {
        if timestamp < 0 {
            return Err(Error::invalid_request(
                "Cannot convert negative timestamp to u64",
            ));
        }
        Ok(timestamp as u64)
    }

    /// Get timestamp for start of day (00:00:00 UTC)
    ///
    /// Given a timestamp, returns the timestamp for the start of that day in UTC.
    ///
    /// # Arguments
    ///
    /// * `timestamp` - Any timestamp within the target day
    ///
    /// # Returns
    ///
    /// Timestamp for 00:00:00 UTC of the same day
    ///
    /// # Example
    ///
    /// ```rust
    /// use ccxt_core::time::TimestampUtils;
    ///
    /// let timestamp = 1704110400000; // 2024-01-01 12:00:00 UTC
    /// let start_of_day = TimestampUtils::start_of_day(timestamp).unwrap();
    /// // Should be 2024-01-01 00:00:00 UTC
    /// ```
    pub fn start_of_day(timestamp: i64) -> Result<i64> {
        let validated = Self::validate_timestamp(timestamp)?;

        let secs = validated / 1000;
        let datetime = DateTime::<Utc>::from_timestamp(secs, 0)
            .ok_or_else(|| Error::invalid_request(format!("Invalid timestamp: {}", validated)))?;

        let start_of_day = datetime
            .date_naive()
            .and_hms_opt(0, 0, 0)
            .ok_or_else(|| Error::invalid_request("Failed to create start of day"))?;

        let start_of_day_utc = Utc.from_utc_datetime(&start_of_day);
        Ok(start_of_day_utc.timestamp_millis())
    }

    /// Get timestamp for end of day (23:59:59.999 UTC)
    ///
    /// Given a timestamp, returns the timestamp for the end of that day in UTC.
    ///
    /// # Arguments
    ///
    /// * `timestamp` - Any timestamp within the target day
    ///
    /// # Returns
    ///
    /// Timestamp for 23:59:59.999 UTC of the same day
    ///
    /// # Example
    ///
    /// ```rust
    /// use ccxt_core::time::TimestampUtils;
    ///
    /// let timestamp = 1704110400000; // 2024-01-01 12:00:00 UTC
    /// let end_of_day = TimestampUtils::end_of_day(timestamp).unwrap();
    /// // Should be 2024-01-01 23:59:59.999 UTC
    /// ```
    pub fn end_of_day(timestamp: i64) -> Result<i64> {
        let validated = Self::validate_timestamp(timestamp)?;

        let secs = validated / 1000;
        let datetime = DateTime::<Utc>::from_timestamp(secs, 0)
            .ok_or_else(|| Error::invalid_request(format!("Invalid timestamp: {}", validated)))?;

        let end_of_day = datetime
            .date_naive()
            .and_hms_milli_opt(23, 59, 59, 999)
            .ok_or_else(|| Error::invalid_request("Failed to create end of day"))?;

        let end_of_day_utc = Utc.from_utc_datetime(&end_of_day);
        Ok(end_of_day_utc.timestamp_millis())
    }
}

/// Extension trait for Option<u64> to Option<i64> conversion
///
/// This trait provides convenient methods for converting Option<u64> timestamps
/// to Option<i64> timestamps during migration. It handles the conversion and
/// validation in a single operation.
///
/// # Example
///
/// ```rust
/// use ccxt_core::time::TimestampConversion;
///
/// let old_timestamp: Option<u64> = Some(1704110400000);
/// let new_timestamp = old_timestamp.to_i64().unwrap();
/// assert_eq!(new_timestamp, Some(1704110400000i64));
///
/// let none_timestamp: Option<u64> = None;
/// let converted = none_timestamp.to_i64().unwrap();
/// assert_eq!(converted, None);
/// ```
pub trait TimestampConversion {
    /// Convert Option<u64> to Option<i64> with validation
    fn to_i64(self) -> Result<Option<i64>>;
}

impl TimestampConversion for Option<u64> {
    #[allow(deprecated)]
    fn to_i64(self) -> Result<Option<i64>> {
        match self {
            Some(ts) => Ok(Some(TimestampUtils::u64_to_i64(ts)?)),
            None => Ok(None),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_milliseconds() {
        let now = milliseconds();
        assert!(now > 1_600_000_000_000); // After 2020
        assert!(now < 2_000_000_000_000); // Before 2033
    }

    #[test]
    fn test_seconds() {
        let now = seconds();
        assert!(now > 1_600_000_000); // After 2020
        assert!(now < 2_000_000_000); // Before 2033
    }

    #[test]
    fn test_microseconds() {
        let now = microseconds();
        assert!(now > 1_600_000_000_000_000); // After 2020
    }

    #[test]
    fn test_iso8601() {
        let timestamp = 1704110400000; // 2024-01-01 12:00:00 UTC
        let result = iso8601(timestamp).unwrap();
        assert_eq!(result, "2024-01-01T12:00:00.000Z");
    }

    #[test]
    fn test_iso8601_with_millis() {
        let timestamp = 1704110400123; // 2024-01-01 12:00:00.123 UTC
        let result = iso8601(timestamp).unwrap();
        assert_eq!(result, "2024-01-01T12:00:00.123Z");
    }

    #[test]
    fn test_iso8601_invalid() {
        let result = iso8601(-1);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_date_iso8601() {
        let result = parse_date("2024-01-01T12:00:00.000Z").unwrap();
        assert_eq!(result, 1704110400000);
    }

    #[test]
    fn test_parse_date_space_separated() {
        let result = parse_date("2024-01-01 12:00:00").unwrap();
        assert_eq!(result, 1704110400000);
    }

    #[test]
    fn test_parse_date_without_timezone() {
        let result = parse_date("2024-01-01T12:00:00.389").unwrap();
        assert_eq!(result, 1704110400389);
    }

    #[test]
    fn test_parse_iso8601() {
        let result = parse_iso8601("2024-01-01T12:00:00.000Z").unwrap();
        assert_eq!(result, 1704110400000);
    }

    #[test]
    fn test_parse_iso8601_with_offset() {
        let result = parse_iso8601("2024-01-01T12:00:00+00:00").unwrap();
        assert_eq!(result, 1704110400000);
    }

    #[test]
    fn test_parse_iso8601_space_separated() {
        let result = parse_iso8601("2024-01-01 12:00:00.389").unwrap();
        assert_eq!(result, 1704110400389);
    }

    #[test]
    fn test_ymdhms_default_separator() {
        let timestamp = 1704110400000;
        let result = ymdhms(timestamp, None).unwrap();
        assert_eq!(result, "2024-01-01 12:00:00");
    }

    #[test]
    fn test_ymdhms_custom_separator() {
        let timestamp = 1704110400000;
        let result = ymdhms(timestamp, Some("T")).unwrap();
        assert_eq!(result, "2024-01-01T12:00:00");
    }

    #[test]
    fn test_yyyymmdd_default_separator() {
        let timestamp = 1704110400000;
        let result = yyyymmdd(timestamp, None).unwrap();
        assert_eq!(result, "2024-01-01");
    }

    #[test]
    fn test_yyyymmdd_custom_separator() {
        let timestamp = 1704110400000;
        let result = yyyymmdd(timestamp, Some("/")).unwrap();
        assert_eq!(result, "2024/01/01");
    }

    #[test]
    fn test_yymmdd_no_separator() {
        let timestamp = 1704110400000;
        let result = yymmdd(timestamp, None).unwrap();
        assert_eq!(result, "240101");
    }

    #[test]
    fn test_yymmdd_with_separator() {
        let timestamp = 1704110400000;
        let result = yymmdd(timestamp, Some("-")).unwrap();
        assert_eq!(result, "24-01-01");
    }

    #[test]
    fn test_ymd_alias() {
        let timestamp = 1704110400000;
        let result = ymd(timestamp, None).unwrap();
        assert_eq!(result, "2024-01-01");
    }

    #[test]
    fn test_round_trip() {
        let original = 1704110400000;
        let iso_str = iso8601(original).unwrap();
        let parsed = parse_date(&iso_str).unwrap();
        assert_eq!(original, parsed);
    }

    // ==================== TimestampUtils Tests ====================

    #[test]
    fn test_timestamp_utils_now_ms() {
        let now = TimestampUtils::now_ms();
        assert!(now > 1_600_000_000_000); // After 2020
        assert!(now < 2_000_000_000_000); // Before 2033
    }

    #[test]
    fn test_timestamp_utils_seconds_to_ms() {
        let seconds = 1704110400; // 2024-01-01 12:00:00 UTC
        let milliseconds = TimestampUtils::seconds_to_ms(seconds);
        assert_eq!(milliseconds, 1704110400000);
    }

    #[test]
    fn test_timestamp_utils_ms_to_seconds() {
        let milliseconds = 1704110400000; // 2024-01-01 12:00:00 UTC
        let seconds = TimestampUtils::ms_to_seconds(milliseconds);
        assert_eq!(seconds, 1704110400);
    }

    #[test]
    fn test_timestamp_utils_validate_timestamp_valid() {
        let valid_timestamp = 1704110400000; // 2024-01-01 12:00:00 UTC
        let result = TimestampUtils::validate_timestamp(valid_timestamp).unwrap();
        assert_eq!(result, valid_timestamp);
    }

    #[test]
    fn test_timestamp_utils_validate_timestamp_negative() {
        let result = TimestampUtils::validate_timestamp(-1);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("negative"));
    }

    #[test]
    fn test_timestamp_utils_validate_timestamp_too_far_future() {
        let far_future = TimestampUtils::YEAR_2100_MS + 1;
        let result = TimestampUtils::validate_timestamp(far_future);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("too far in future")
        );
    }

    #[test]
    fn test_timestamp_utils_parse_timestamp_integer() {
        let result = TimestampUtils::parse_timestamp("1704110400000").unwrap();
        assert_eq!(result, 1704110400000);
    }

    #[test]
    fn test_timestamp_utils_parse_timestamp_decimal() {
        let result = TimestampUtils::parse_timestamp("1704110400.123").unwrap();
        assert_eq!(result, 1704110400123);
    }

    #[test]
    fn test_timestamp_utils_parse_timestamp_invalid() {
        let result = TimestampUtils::parse_timestamp("invalid");
        assert!(result.is_err());
    }

    #[test]
    fn test_timestamp_utils_parse_timestamp_empty() {
        let result = TimestampUtils::parse_timestamp("");
        assert!(result.is_err());
    }

    #[test]
    fn test_timestamp_utils_format_iso8601() {
        let timestamp = 1704110400000; // 2024-01-01 12:00:00 UTC
        let result = TimestampUtils::format_iso8601(timestamp).unwrap();
        assert_eq!(result, "2024-01-01T12:00:00.000Z");
    }

    #[test]
    fn test_timestamp_utils_format_iso8601_with_millis() {
        let timestamp = 1704110400123; // 2024-01-01 12:00:00.123 UTC
        let result = TimestampUtils::format_iso8601(timestamp).unwrap();
        assert_eq!(result, "2024-01-01T12:00:00.123Z");
    }

    #[test]
    fn test_timestamp_utils_is_reasonable_timestamp() {
        assert!(TimestampUtils::is_reasonable_timestamp(1704110400000)); // 2024
        assert!(TimestampUtils::is_reasonable_timestamp(0)); // Unix epoch
        assert!(!TimestampUtils::is_reasonable_timestamp(-1)); // Before epoch
        assert!(!TimestampUtils::is_reasonable_timestamp(
            TimestampUtils::YEAR_2100_MS + 1
        )); // Too far future
    }

    #[test]
    #[allow(deprecated)]
    fn test_timestamp_utils_u64_to_i64_valid() {
        let old_timestamp: u64 = 1704110400000;
        let result = TimestampUtils::u64_to_i64(old_timestamp).unwrap();
        assert_eq!(result, 1704110400000i64);
    }

    #[test]
    #[allow(deprecated)]
    fn test_timestamp_utils_u64_to_i64_overflow() {
        let overflow_timestamp = u64::MAX;
        let result = TimestampUtils::u64_to_i64(overflow_timestamp);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("overflow"));
    }

    #[test]
    #[allow(deprecated)]
    fn test_timestamp_utils_i64_to_u64_valid() {
        let new_timestamp: i64 = 1704110400000;
        let result = TimestampUtils::i64_to_u64(new_timestamp).unwrap();
        assert_eq!(result, 1704110400000u64);
    }

    #[test]
    #[allow(deprecated)]
    fn test_timestamp_utils_i64_to_u64_negative() {
        let negative_timestamp: i64 = -1;
        let result = TimestampUtils::i64_to_u64(negative_timestamp);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("negative"));
    }

    #[test]
    fn test_timestamp_utils_start_of_day() {
        let timestamp = 1704110400000; // 2024-01-01 12:00:00 UTC
        let start_of_day = TimestampUtils::start_of_day(timestamp).unwrap();

        // Should be 2024-01-01 00:00:00 UTC = 1704067200000
        let expected = 1704067200000;
        assert_eq!(start_of_day, expected);
    }

    #[test]
    fn test_timestamp_utils_end_of_day() {
        let timestamp = 1704110400000; // 2024-01-01 12:00:00 UTC
        let end_of_day = TimestampUtils::end_of_day(timestamp).unwrap();

        // Should be 2024-01-01 23:59:59.999 UTC = 1704153599999
        let expected = 1704153599999;
        assert_eq!(end_of_day, expected);
    }

    // ==================== TimestampConversion Tests ====================

    #[test]
    fn test_timestamp_conversion_some() {
        let old_timestamp: Option<u64> = Some(1704110400000);
        let result = old_timestamp.to_i64().unwrap();
        assert_eq!(result, Some(1704110400000i64));
    }

    #[test]
    fn test_timestamp_conversion_none() {
        let old_timestamp: Option<u64> = None;
        let result = old_timestamp.to_i64().unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn test_timestamp_conversion_overflow() {
        let old_timestamp: Option<u64> = Some(u64::MAX);
        let result = old_timestamp.to_i64();
        assert!(result.is_err());
    }

    // ==================== Integration Tests ====================

    #[test]
    fn test_timestamp_round_trip_with_validation() {
        let original = 1704110400000;

        // Validate original
        let validated = TimestampUtils::validate_timestamp(original).unwrap();
        assert_eq!(validated, original);

        // Format to ISO 8601
        let iso_str = TimestampUtils::format_iso8601(validated).unwrap();

        // Parse back
        let parsed = parse_date(&iso_str).unwrap();
        assert_eq!(parsed, original);
    }

    #[test]
    fn test_migration_workflow() {
        // Simulate migration from u64 to i64
        let old_timestamp: u64 = 1704110400000;

        // Convert to i64
        #[allow(deprecated)]
        let new_timestamp = TimestampUtils::u64_to_i64(old_timestamp).unwrap();

        // Validate
        let validated = TimestampUtils::validate_timestamp(new_timestamp).unwrap();

        // Use in formatting
        let formatted = TimestampUtils::format_iso8601(validated).unwrap();
        assert_eq!(formatted, "2024-01-01T12:00:00.000Z");

        // Convert back for legacy code if needed
        #[allow(deprecated)]
        let back_to_u64 = TimestampUtils::i64_to_u64(validated).unwrap();
        assert_eq!(back_to_u64, old_timestamp);
    }

    #[test]
    fn test_edge_cases() {
        // Test Unix epoch
        let epoch = 0i64;
        let validated = TimestampUtils::validate_timestamp(epoch).unwrap();
        assert_eq!(validated, epoch);

        // Test year 2100 boundary
        let year_2100 = TimestampUtils::YEAR_2100_MS;
        let validated = TimestampUtils::validate_timestamp(year_2100).unwrap();
        assert_eq!(validated, year_2100);

        // Test just over year 2100 boundary
        let over_2100 = TimestampUtils::YEAR_2100_MS + 1;
        let result = TimestampUtils::validate_timestamp(over_2100);
        assert!(result.is_err());
    }

    #[test]
    fn test_consistency_between_functions() {
        let timestamp = 1704110400000;

        // Test that all formatting functions use the same validation
        let ymdhms_result = ymdhms(timestamp, None);
        let yyyymmdd_result = yyyymmdd(timestamp, None);
        let yymmdd_result = yymmdd(timestamp, None);
        let iso8601_result = iso8601(timestamp);

        assert!(ymdhms_result.is_ok());
        assert!(yyyymmdd_result.is_ok());
        assert!(yymmdd_result.is_ok());
        assert!(iso8601_result.is_ok());

        // Test that all fail for invalid timestamps
        let invalid = -1i64;
        assert!(ymdhms(invalid, None).is_err());
        assert!(yyyymmdd(invalid, None).is_err());
        assert!(yymmdd(invalid, None).is_err());
        assert!(iso8601(invalid).is_err());
    }
}
