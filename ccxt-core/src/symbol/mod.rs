//! Unified Symbol Format Module
//!
//! This module provides comprehensive parsing, formatting, and analysis functionality for
//! unified trading pair symbols across different market types (Spot, Swap, Futures) and
//! settlement currencies (Linear/Inverse).
//!
//! # Overview
//!
//! The unified symbol format follows the CCXT standard, providing a consistent way to
//! represent trading pairs across all supported cryptocurrency exchanges. This enables
//! seamless interoperability and simplifies cross-exchange trading operations.
//!
//! # Symbol Format
//!
//! | Market Type | Format | Example |
//! |-------------|--------|---------|
//! | Spot | `BASE/QUOTE` | `BTC/USDT` |
//! | Linear Swap | `BASE/QUOTE:SETTLE` (settle = quote) | `BTC/USDT:USDT` |
//! | Inverse Swap | `BASE/QUOTE:SETTLE` (settle = base) | `BTC/USD:BTC` |
//! | Futures | `BASE/QUOTE:SETTLE-YYMMDD` | `BTC/USDT:USDT-241231` |
//!
//! # Components
//!
//! - [`SymbolParser`]: Parse unified symbol strings into [`ParsedSymbol`] structures
//! - [`SymbolFormatter`]: Format [`ParsedSymbol`] back to unified symbol strings
//! - [`SymbolError`]: Error types for symbol parsing and validation
//! - [`ParsedSymbol`]: Structured representation of a parsed symbol
//! - [`ExpiryDate`]: Futures contract expiry date in YYMMDD format
//! - [`SymbolMarketType`]: Market type enum (Spot, Swap, Futures)
//! - [`ContractType`]: Contract settlement type (Linear, Inverse)
//!
//! # Quick Start
//!
//! ## Parsing Symbols
//!
//! ```rust
//! use ccxt_core::symbol::{SymbolParser, ParsedSymbol};
//!
//! // Parse a spot symbol
//! let spot = SymbolParser::parse("BTC/USDT").unwrap();
//! assert!(spot.is_spot());
//! assert_eq!(spot.base, "BTC");
//! assert_eq!(spot.quote, "USDT");
//!
//! // Parse a perpetual swap symbol
//! let swap = SymbolParser::parse("BTC/USDT:USDT").unwrap();
//! assert!(swap.is_swap());
//! assert!(swap.is_linear());
//!
//! // Parse a futures symbol
//! let futures = SymbolParser::parse("BTC/USDT:USDT-241231").unwrap();
//! assert!(futures.is_futures());
//! assert!(futures.expiry.is_some());
//! ```
//!
//! ## Using FromStr Trait
//!
//! ```rust
//! use ccxt_core::symbol::ParsedSymbol;
//!
//! // Parse using the FromStr trait
//! let symbol: ParsedSymbol = "ETH/USDT".parse().unwrap();
//! assert_eq!(symbol.base, "ETH");
//! assert_eq!(symbol.quote, "USDT");
//! ```
//!
//! ## Creating Symbols Programmatically
//!
//! ```rust
//! use ccxt_core::symbol::{ParsedSymbol, ExpiryDate};
//!
//! // Create a spot symbol
//! let spot = ParsedSymbol::spot("BTC".to_string(), "USDT".to_string());
//! assert_eq!(spot.to_string(), "BTC/USDT");
//!
//! // Create a linear swap symbol
//! let swap = ParsedSymbol::linear_swap("ETH".to_string(), "USDT".to_string());
//! assert_eq!(swap.to_string(), "ETH/USDT:USDT");
//!
//! // Create an inverse swap symbol
//! let inverse = ParsedSymbol::inverse_swap("BTC".to_string(), "USD".to_string());
//! assert_eq!(inverse.to_string(), "BTC/USD:BTC");
//!
//! // Create a futures symbol
//! let expiry = ExpiryDate::new(24, 12, 31).unwrap();
//! let futures = ParsedSymbol::futures(
//!     "BTC".to_string(),
//!     "USDT".to_string(),
//!     "USDT".to_string(),
//!     expiry
//! );
//! assert_eq!(futures.to_string(), "BTC/USDT:USDT-241231");
//! ```
//!
//! ## Formatting Symbols
//!
//! ```rust
//! use ccxt_core::symbol::{SymbolFormatter, ParsedSymbol};
//!
//! let symbol = ParsedSymbol::linear_swap("BTC".to_string(), "USDT".to_string());
//!
//! // Using SymbolFormatter
//! let formatted = SymbolFormatter::format(&symbol);
//! assert_eq!(formatted, "BTC/USDT:USDT");
//!
//! // Using Display trait (equivalent)
//! assert_eq!(symbol.to_string(), "BTC/USDT:USDT");
//!
//! // Using helper methods
//! assert_eq!(SymbolFormatter::format_spot("BTC", "USDT"), "BTC/USDT");
//! assert_eq!(SymbolFormatter::format_swap("BTC", "USDT", "USDT"), "BTC/USDT:USDT");
//! ```
//!
//! ## Analyzing Symbols
//!
//! ```rust
//! use ccxt_core::symbol::{SymbolParser, SymbolMarketType, ContractType};
//!
//! let symbol = SymbolParser::parse("BTC/USDT:USDT").unwrap();
//!
//! // Check market type
//! assert_eq!(symbol.market_type(), SymbolMarketType::Swap);
//! assert!(symbol.is_swap());
//! assert!(symbol.is_derivative());
//!
//! // Check contract type
//! assert_eq!(symbol.contract_type(), Some(ContractType::Linear));
//! assert!(symbol.is_linear());
//! ```
//!
//! # Error Handling
//!
//! ```rust
//! use ccxt_core::symbol::{SymbolParser, SymbolError};
//!
//! // Invalid format
//! let result = SymbolParser::parse("BTCUSDT");
//! assert!(result.is_err());
//!
//! // Invalid date in futures symbol
//! let result = SymbolParser::parse("BTC/USDT:USDT-241301"); // month 13
//! assert!(result.is_err());
//!
//! // Validate without parsing
//! assert!(SymbolParser::validate("BTC/USDT").is_ok());
//! assert!(SymbolParser::validate("invalid").is_err());
//! ```
//!
//! # Round-Trip Consistency
//!
//! The symbol module guarantees round-trip consistency: parsing a formatted symbol
//! and formatting it again produces the same result.
//!
//! ```rust
//! use ccxt_core::symbol::{SymbolParser, SymbolFormatter, ParsedSymbol};
//!
//! let original = "BTC/USDT:USDT-241231";
//! let parsed = SymbolParser::parse(original).unwrap();
//! let formatted = SymbolFormatter::format(&parsed);
//! let reparsed = SymbolParser::parse(&formatted).unwrap();
//!
//! assert_eq!(parsed, reparsed);
//! assert_eq!(formatted, original);
//! ```

pub mod error;
pub mod formatter;
pub mod parser;

// Re-export main types for convenience
pub use error::SymbolError;
pub use formatter::SymbolFormatter;
pub use parser::SymbolParser;

// Re-export types from the types module for convenience
pub use crate::types::symbol::{ContractType, ExpiryDate, ParsedSymbol, SymbolMarketType};
