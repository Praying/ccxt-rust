//! Stream parsers for WebSocket message parsing
//!
//! This module provides the `StreamParser` trait and implementations for
//! parsing different types of WebSocket messages from Binance.

use crate::binance::parser;
use ccxt_core::error::Result;
use ccxt_core::types::{BidAsk, MarkPrice, Market, OHLCV, Ticker, Trade};
use serde_json::Value;

/// Trait for parsing WebSocket stream messages into typed data.
///
/// Implementations of this trait handle the conversion of raw JSON messages
/// from Binance WebSocket streams into strongly-typed Rust structures.
pub trait StreamParser {
    /// The output type produced by this parser
    type Output;

    /// Parse a WebSocket message into the output type.
    ///
    /// # Arguments
    /// * `message` - The raw JSON message from the WebSocket
    /// * `market` - Optional market information for symbol resolution
    ///
    /// # Returns
    /// The parsed data or an error if parsing fails
    fn parse(message: &Value, market: Option<&Market>) -> Result<Self::Output>;
}

/// Parser for ticker stream messages.
pub struct TickerParser;

impl StreamParser for TickerParser {
    type Output = Ticker;

    fn parse(message: &Value, market: Option<&Market>) -> Result<Self::Output> {
        parser::parse_ws_ticker(message, market)
    }
}

/// Parser for trade stream messages.
pub struct TradeParser;

impl StreamParser for TradeParser {
    type Output = Trade;

    fn parse(message: &Value, market: Option<&Market>) -> Result<Self::Output> {
        parser::parse_ws_trade(message, market)
    }
}

/// Parser for OHLCV (kline) stream messages.
pub struct OhlcvParser;

impl StreamParser for OhlcvParser {
    type Output = OHLCV;

    fn parse(message: &Value, _market: Option<&Market>) -> Result<Self::Output> {
        parser::parse_ws_ohlcv(message)
    }
}

/// Parser for mark price stream messages.
pub struct MarkPriceParser;

impl StreamParser for MarkPriceParser {
    type Output = MarkPrice;

    fn parse(message: &Value, _market: Option<&Market>) -> Result<Self::Output> {
        parser::parse_ws_mark_price(message)
    }
}

/// Parser for bid/ask (book ticker) stream messages.
pub struct BidAskParser;

impl StreamParser for BidAskParser {
    type Output = BidAsk;

    fn parse(message: &Value, _market: Option<&Market>) -> Result<Self::Output> {
        parser::parse_ws_bid_ask(message)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_ticker_parser() {
        let message = json!({
            "e": "24hrTicker",
            "s": "BTCUSDT",
            "c": "50000.00",
            "o": "49000.00",
            "h": "51000.00",
            "l": "48000.00",
            "v": "1000.00",
            "q": "50000000.00",
            "C": 1234567890000_i64
        });

        let result = TickerParser::parse(&message, None);
        assert!(result.is_ok());
        let ticker = result.unwrap();
        assert_eq!(ticker.symbol, "BTCUSDT");
    }

    #[test]
    fn test_bid_ask_parser() {
        let message = json!({
            "s": "BTCUSDT",
            "b": "50000.00",
            "B": "1.5",
            "a": "50001.00",
            "A": "2.0",
            "E": 1234567890000_i64
        });

        let result = BidAskParser::parse(&message, None);
        assert!(result.is_ok());
        let bid_ask = result.unwrap();
        assert_eq!(bid_ask.symbol, "BTCUSDT");
    }
}
