//! Binance REST API implementation organized by market type and functionality.
//!
//! This module provides a modularized structure for Binance REST API operations,
//! organized into logical sub-modules by market type and functionality:
//!
//! - `market_data`: Public market data operations (ticker, orderbook, trades, OHLCV)
//! - `spot`: Spot trading operations (orders, balance)
//! - `futures`: Perpetual futures (FAPI) operations
//! - `delivery`: Delivery futures (DAPI) operations
//! - `margin`: Margin trading operations
//! - `account`: Account-related operations
//! - `funding`: Deposit and withdrawal operations
//!
//! # Usage
//!
//! All methods are implemented directly on the `Binance` struct, so you can call them
//! directly on any `Binance` instance:
//!
//! ```no_run
//! # use ccxt_exchanges::binance::Binance;
//! # use ccxt_core::ExchangeConfig;
//! # async fn example() -> ccxt_core::Result<()> {
//! let binance = Binance::new(ExchangeConfig::default())?;
//!
//! // Market data methods (from market_data.rs)
//! let ticker = binance.fetch_ticker("BTC/USDT", ()).await?;
//!
//! // Futures methods (from futures.rs)
//! // let position = binance.fetch_position("BTC/USDT:USDT", None).await?;
//!
//! // Margin methods (from margin.rs)
//! // let loan = binance.borrow_cross_margin("USDT", 100.0).await?;
//!
//! // Funding methods (from funding.rs)
//! // let address = binance.fetch_deposit_address("BTC", None).await?;
//! # Ok(())
//! # }
//! ```

pub mod account;
pub mod delivery;
pub mod funding;
pub mod futures;
pub mod margin;
pub mod market_data;
pub mod spot;
