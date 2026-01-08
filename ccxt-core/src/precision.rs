//! Financial-grade precision calculation utilities.
//!
//! Provides high-precision numerical operations for financial calculations:
//! - Rounding modes (Round, RoundUp, RoundDown)
//! - Precision counting modes (DecimalPlaces, SignificantDigits, TickSize)
//! - Number formatting and parsing
//! - `Decimal` type helper functions

use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;
use std::str::FromStr;

use crate::error::{Error, ParseError, Result};

/// Rounding mode for precision calculations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoundingMode {
    /// Standard rounding (rounds to nearest, ties away from zero)
    Round = 1,
    /// Round up (away from zero)
    RoundUp = 2,
    /// Round down (toward zero, truncate)
    RoundDown = 3,
}

/// Precision counting mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CountingMode {
    /// Count decimal places
    DecimalPlaces = 2,
    /// Count significant digits
    SignificantDigits = 3,
    /// Use minimum price tick size
    TickSize = 4,
}

/// Output padding mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaddingMode {
    /// No padding (remove trailing zeros)
    NoPadding = 5,
    /// Pad with zeros to precision
    PadWithZero = 6,
}

/// Converts a decimal number to string, handling scientific notation.
///
/// # Examples
///
/// ```
/// use ccxt_core::precision::number_to_string;
/// use rust_decimal::Decimal;
/// use std::str::FromStr;
///
/// let num = Decimal::from_str("0.00000123").unwrap();
/// assert_eq!(number_to_string(num), "0.00000123");
///
/// let large = Decimal::from_str("1234567890").unwrap();
/// assert_eq!(number_to_string(large), "1234567890");
/// ```
pub fn number_to_string(value: Decimal) -> String {
    let s = value.to_string();

    if s.contains('.') {
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    } else {
        s
    }
}

/// Extracts precision (number of decimal places) from a string representation.
///
/// # Examples
///
/// ```
/// use ccxt_core::precision::precision_from_string;
///
/// assert_eq!(precision_from_string("0.001"), 3);
/// assert_eq!(precision_from_string("0.01"), 2);
/// assert_eq!(precision_from_string("1.2345"), 4);
/// assert_eq!(precision_from_string("100"), 0);
/// assert_eq!(precision_from_string("1e-8"), 8);
/// ```
pub fn precision_from_string(s: &str) -> i32 {
    if (s.contains('e') || s.contains('E'))
        && let Some(e_pos) = s.find(['e', 'E'])
    {
        let exp_str = &s[e_pos + 1..];
        if let Ok(exp) = exp_str.parse::<i32>() {
            return -exp;
        }
    }

    let trimmed = s.trim_end_matches('0');
    if let Some(dot_pos) = trimmed.find('.') {
        #[allow(clippy::cast_possible_truncation)]
        let res = (trimmed.len() - dot_pos - 1) as i32;
        res
    } else {
        0
    }
}

/// Truncates a decimal to the specified number of decimal places without rounding.
///
/// # Examples
///
/// ```
/// use ccxt_core::precision::truncate_to_string;
/// use rust_decimal::Decimal;
/// use std::str::FromStr;
///
/// let num = Decimal::from_str("123.456789").unwrap();
/// assert_eq!(truncate_to_string(num, 2), "123.45");
/// assert_eq!(truncate_to_string(num, 4), "123.4567");
/// assert_eq!(truncate_to_string(num, 0), "123");
/// ```
pub fn truncate_to_string(value: Decimal, precision: i32) -> String {
    if precision <= 0 {
        return value.trunc().to_string();
    }

    let s = value.to_string();

    if let Some(dot_pos) = s.find('.') {
        let end_pos = std::cmp::min(dot_pos + 1 + precision as usize, s.len());
        s[..end_pos].to_string()
    } else {
        s
    }
}

/// Truncates a decimal to the specified number of decimal places, returning a `Decimal`.
///
/// # Examples
///
/// ```
/// use ccxt_core::precision::truncate;
/// use rust_decimal::Decimal;
/// use std::str::FromStr;
///
/// let num = Decimal::from_str("123.456789").unwrap();
/// let truncated = truncate(num, 2);
/// assert_eq!(truncated.to_string(), "123.45");
/// ```
pub fn truncate(value: Decimal, precision: i32) -> Decimal {
    let s = truncate_to_string(value, precision);
    Decimal::from_str(&s).unwrap_or(value)
}

/// Formats a decimal to a specific precision with configurable rounding and counting modes.
///
/// Core precision formatting function equivalent to Go's `DecimalToPrecision` method.
///
/// # Arguments
///
/// * `value` - The decimal value to format
/// * `rounding_mode` - Rounding behavior to apply
/// * `num_precision_digits` - Number of precision digits
/// * `counting_mode` - How to count precision (default: `DecimalPlaces`)
/// * `padding_mode` - Output padding behavior (default: `NoPadding`)
///
/// # Examples
///
/// ```
/// use ccxt_core::precision::{decimal_to_precision, RoundingMode, CountingMode, PaddingMode};
/// use rust_decimal::Decimal;
/// use std::str::FromStr;
///
/// let num = Decimal::from_str("123.456789").unwrap();
///
/// // Round to 2 decimal places
/// let result = decimal_to_precision(
///     num,
///     RoundingMode::Round,
///     2,
///     Some(CountingMode::DecimalPlaces),
///     Some(PaddingMode::NoPadding),
/// ).unwrap();
/// assert_eq!(result, "123.46");
///
/// // Truncate to 2 decimal places
/// let result = decimal_to_precision(
///     num,
///     RoundingMode::RoundDown,
///     2,
///     Some(CountingMode::DecimalPlaces),
///     Some(PaddingMode::NoPadding),
/// ).unwrap();
/// assert_eq!(result, "123.45");
/// ```
pub fn decimal_to_precision(
    value: Decimal,
    rounding_mode: RoundingMode,
    num_precision_digits: i32,
    counting_mode: Option<CountingMode>,
    padding_mode: Option<PaddingMode>,
) -> Result<String> {
    let counting_mode = counting_mode.unwrap_or(CountingMode::DecimalPlaces);
    let padding_mode = padding_mode.unwrap_or(PaddingMode::NoPadding);

    // Handle negative precision (round to powers of 10)
    if num_precision_digits < 0 {
        let to_nearest =
            Decimal::from_i128_with_scale(10_i128.pow(num_precision_digits.unsigned_abs()), 0);

        match rounding_mode {
            RoundingMode::Round => {
                let divided = value / to_nearest;
                let rounded = round_decimal(divided, 0, RoundingMode::Round);
                let result = to_nearest * rounded;
                return Ok(format_decimal(result, padding_mode));
            }
            RoundingMode::RoundDown => {
                let modulo = value % to_nearest;
                let result = value - modulo;
                return Ok(format_decimal(result, padding_mode));
            }
            RoundingMode::RoundUp => {
                let modulo = value % to_nearest;
                let result = if modulo.is_zero() {
                    value
                } else {
                    value - modulo + to_nearest
                };
                return Ok(format_decimal(result, padding_mode));
            }
        }
    }

    match counting_mode {
        CountingMode::DecimalPlaces => Ok(decimal_to_precision_decimal_places(
            value,
            rounding_mode,
            num_precision_digits,
            padding_mode,
        )),
        CountingMode::SignificantDigits => Ok(decimal_to_precision_significant_digits(
            value,
            rounding_mode,
            num_precision_digits,
            padding_mode,
        )),
        CountingMode::TickSize => {
            let tick_size = Decimal::from_str(&format!(
                "0.{}",
                "0".repeat((num_precision_digits - 1) as usize) + "1"
            ))
            .map_err(|e| {
                ParseError::invalid_format("tick_size", format!("Invalid tick size: {e}"))
            })?;

            decimal_to_precision_tick_size(value, rounding_mode, tick_size, padding_mode)
        }
    }
}

/// Formats decimal by counting decimal places.
fn decimal_to_precision_decimal_places(
    value: Decimal,
    rounding_mode: RoundingMode,
    decimal_places: i32,
    padding_mode: PaddingMode,
) -> String {
    let rounded = round_decimal(value, decimal_places, rounding_mode);
    format_decimal_with_places(rounded, decimal_places, padding_mode)
}

/// Formats decimal by counting significant digits.
fn decimal_to_precision_significant_digits(
    value: Decimal,
    rounding_mode: RoundingMode,
    sig_digits: i32,
    padding_mode: PaddingMode,
) -> String {
    if value.is_zero() {
        return "0".to_string();
    }

    let abs_value = value.abs();
    let value_f64 = abs_value.to_f64().unwrap_or(0.0);
    let log10 = value_f64.log10();
    #[allow(clippy::cast_possible_truncation)]
    let magnitude = log10.floor() as i32;

    let decimal_places = sig_digits - magnitude - 1;

    let rounded = round_decimal(value, decimal_places, rounding_mode);
    format_decimal_with_places(rounded, decimal_places, padding_mode)
}

/// Formats decimal to align with tick size increments.
fn decimal_to_precision_tick_size(
    value: Decimal,
    rounding_mode: RoundingMode,
    tick_size: Decimal,
    padding_mode: PaddingMode,
) -> Result<String> {
    if tick_size <= Decimal::ZERO {
        return Err(Error::invalid_request("Tick size must be positive"));
    }

    let ticks = match rounding_mode {
        RoundingMode::Round => (value / tick_size).round(),
        RoundingMode::RoundDown => (value / tick_size).floor(),
        RoundingMode::RoundUp => (value / tick_size).ceil(),
    };

    let result = ticks * tick_size;

    let tick_precision = precision_from_string(&tick_size.to_string());
    Ok(format_decimal_with_places(
        result,
        tick_precision,
        padding_mode,
    ))
}

/// Rounds a decimal value to the specified number of decimal places.
fn round_decimal(value: Decimal, decimal_places: i32, mode: RoundingMode) -> Decimal {
    if decimal_places < 0 {
        let scale = Decimal::from_i128_with_scale(10_i128.pow(decimal_places.unsigned_abs()), 0);
        let divided = value / scale;
        let rounded = match mode {
            RoundingMode::Round => divided.round(),
            RoundingMode::RoundDown => divided.floor(),
            RoundingMode::RoundUp => divided.ceil(),
        };
        return rounded * scale;
    }

    let scale = Decimal::from_i128_with_scale(10_i128.pow(decimal_places as u32), 0);
    let scaled = value * scale;

    let rounded = match mode {
        RoundingMode::Round => scaled.round(),
        RoundingMode::RoundDown => scaled.floor(),
        RoundingMode::RoundUp => scaled.ceil(),
    };

    rounded / scale
}

/// Formats decimal by removing trailing zeros.
fn format_decimal(value: Decimal, _padding_mode: PaddingMode) -> String {
    let s = value.to_string();

    if s.contains('.') {
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    } else {
        s
    }
}

/// Formats decimal to the specified number of decimal places.
fn format_decimal_with_places(
    value: Decimal,
    decimal_places: i32,
    padding_mode: PaddingMode,
) -> String {
    match padding_mode {
        PaddingMode::NoPadding => format_decimal(value, padding_mode),
        PaddingMode::PadWithZero => {
            if decimal_places > 0 {
                format!("{:.prec$}", value, prec = decimal_places as usize)
            } else {
                value.trunc().to_string()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_number_to_string() {
        let num = Decimal::from_str("0.00000123").unwrap();
        assert_eq!(number_to_string(num), "0.00000123");

        let large = Decimal::from_str("1234567890").unwrap();
        assert_eq!(number_to_string(large), "1234567890");

        let with_trailing = Decimal::from_str("123.4500").unwrap();
        assert_eq!(number_to_string(with_trailing), "123.45");
    }

    #[test]
    fn test_precision_from_string() {
        assert_eq!(precision_from_string("0.001"), 3);
        assert_eq!(precision_from_string("0.01"), 2);
        assert_eq!(precision_from_string("1.2345"), 4);
        assert_eq!(precision_from_string("100"), 0);
        assert_eq!(precision_from_string("1.0000"), 0);
    }

    #[test]
    fn test_truncate() {
        let num = Decimal::from_str("123.456789").unwrap();

        assert_eq!(truncate_to_string(num, 2), "123.45");
        assert_eq!(truncate_to_string(num, 4), "123.4567");
        assert_eq!(truncate_to_string(num, 0), "123");

        let truncated = truncate(num, 2);
        assert_eq!(truncated.to_string(), "123.45");
    }

    #[test]
    fn test_decimal_to_precision_round() {
        let num = Decimal::from_str("123.456").unwrap();

        let result = decimal_to_precision(
            num,
            RoundingMode::Round,
            2,
            Some(CountingMode::DecimalPlaces),
            Some(PaddingMode::NoPadding),
        )
        .unwrap();
        assert_eq!(result, "123.46");
    }

    #[test]
    fn test_decimal_to_precision_round_down() {
        let num = Decimal::from_str("123.456").unwrap();

        let result = decimal_to_precision(
            num,
            RoundingMode::RoundDown,
            2,
            Some(CountingMode::DecimalPlaces),
            Some(PaddingMode::NoPadding),
        )
        .unwrap();
        assert_eq!(result, "123.45");
    }

    #[test]
    fn test_decimal_to_precision_round_up() {
        let num = Decimal::from_str("123.451").unwrap();

        let result = decimal_to_precision(
            num,
            RoundingMode::RoundUp,
            2,
            Some(CountingMode::DecimalPlaces),
            Some(PaddingMode::NoPadding),
        )
        .unwrap();
        assert_eq!(result, "123.46");
    }

    #[test]
    fn test_decimal_to_precision_with_padding() {
        let num = Decimal::from_str("123.4").unwrap();

        let result = decimal_to_precision(
            num,
            RoundingMode::Round,
            3,
            Some(CountingMode::DecimalPlaces),
            Some(PaddingMode::PadWithZero),
        )
        .unwrap();
        assert_eq!(result, "123.400");
    }

    #[test]
    fn test_decimal_to_precision_negative_precision() {
        let num = Decimal::from_str("123.456").unwrap();

        let result = decimal_to_precision(
            num,
            RoundingMode::Round,
            -1,
            Some(CountingMode::DecimalPlaces),
            Some(PaddingMode::NoPadding),
        )
        .unwrap();
        assert_eq!(result, "120");
    }

    #[test]
    fn test_decimal_to_precision_tick_size() {
        let num = Decimal::from_str("123.456").unwrap();
        let tick = Decimal::from_str("0.05").unwrap();

        let result =
            decimal_to_precision_tick_size(num, RoundingMode::Round, tick, PaddingMode::NoPadding)
                .unwrap();

        // 123.456 / 0.05 = 2469.12, round = 2469, * 0.05 = 123.45
        assert_eq!(result, "123.45");
    }

    #[test]
    fn test_round_decimal() {
        let num = Decimal::from_str("123.456").unwrap();

        let rounded = round_decimal(num, 2, RoundingMode::Round);
        assert_eq!(rounded.to_string(), "123.46");

        let truncated = round_decimal(num, 2, RoundingMode::RoundDown);
        assert_eq!(truncated.to_string(), "123.45");
    }

    #[test]
    fn test_format_decimal() {
        let num = Decimal::from_str("123.4500").unwrap();
        let formatted = format_decimal(num, PaddingMode::NoPadding);
        assert_eq!(formatted, "123.45");

        let formatted_padded = format_decimal_with_places(num, 4, PaddingMode::PadWithZero);
        assert_eq!(formatted_padded, "123.4500");
    }
}
