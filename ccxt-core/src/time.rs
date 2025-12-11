//! Time utilities for CCXT
//!
//! This module provides time-related utility functions for timestamp conversion,
//! date parsing, and formatting. All timestamps are in milliseconds since Unix epoch
//! unless otherwise specified.
//!
//! # Key Features
//!
//! - Millisecond/second/microsecond timestamp generation
//! - ISO 8601 date parsing and formatting
//! - Multiple date format support
//! - UTC timezone handling
//!
//! # Example
//!
//! ```rust
//! use ccxt_core::time::{milliseconds, iso8601, parse_date};
//!
//! // Get current timestamp in milliseconds
//! let now = milliseconds();
//!
//! // Convert timestamp to ISO 8601 string
//! let iso_str = iso8601(now).unwrap();
//!
//! // Parse ISO 8601 string back to timestamp
//! let parsed = parse_date(&iso_str).unwrap();
//! assert_eq!(now, parsed);
//! ```

use crate::error::{ParseError, Result};
use chrono::{DateTime, NaiveDateTime, TimeZone, Utc};

/// Returns the current time in milliseconds since the Unix epoch
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
    Utc::now().timestamp_millis()
}

/// Returns the current time in seconds since the Unix epoch
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
    milliseconds() / 1000
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
    if timestamp <= 0 {
        return Err(ParseError::timestamp("Invalid timestamp: must be positive").into());
    }

    // Convert milliseconds to seconds and nanoseconds
    let secs = timestamp / 1000;
    let nsecs = ((timestamp % 1000) * 1_000_000) as u32;

    let datetime = DateTime::<Utc>::from_timestamp(secs, nsecs)
        .ok_or_else(|| ParseError::timestamp_owned(format!("Invalid timestamp: {}", timestamp)))?;

    Ok(datetime.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string())
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
    if timestamp <= 0 {
        return Err(ParseError::timestamp("Invalid timestamp: must be positive").into());
    }

    let sep = separator.unwrap_or(" ");
    let secs = timestamp / 1000;
    let nsecs = ((timestamp % 1000) * 1_000_000) as u32;

    let datetime = DateTime::<Utc>::from_timestamp(secs, nsecs)
        .ok_or_else(|| ParseError::timestamp_owned(format!("Invalid timestamp: {}", timestamp)))?;

    Ok(format!(
        "{}{}{}",
        datetime.format("%Y-%m-%d"),
        sep,
        datetime.format("%H:%M:%S")
    ))
}

/// Formats a timestamp as "yyyy-MM-dd"
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
    if timestamp <= 0 {
        return Err(ParseError::timestamp("Invalid timestamp: must be positive").into());
    }

    let sep = separator.unwrap_or("-");
    let secs = timestamp / 1000;
    let nsecs = ((timestamp % 1000) * 1_000_000) as u32;

    let datetime = DateTime::<Utc>::from_timestamp(secs, nsecs)
        .ok_or_else(|| ParseError::timestamp_owned(format!("Invalid timestamp: {}", timestamp)))?;

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
    if timestamp <= 0 {
        return Err(ParseError::timestamp("Invalid timestamp: must be positive").into());
    }

    let sep = separator.unwrap_or("");
    let secs = timestamp / 1000;
    let nsecs = ((timestamp % 1000) * 1_000_000) as u32;

    let datetime = DateTime::<Utc>::from_timestamp(secs, nsecs)
        .ok_or_else(|| ParseError::timestamp_owned(format!("Invalid timestamp: {}", timestamp)))?;

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
}
