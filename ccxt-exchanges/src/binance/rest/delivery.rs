//! Binance delivery futures (DAPI) operations.
//!
//! This module is reserved for DAPI (coin-margined delivery futures) specific methods.
//!
//! # Note
//!
//! Most DAPI operations are handled automatically by the methods in the `futures` module.
//! When you call methods like `fetch_position`, `set_leverage`, or `fetch_funding_rate`
//! with a coin-margined symbol (e.g., "BTC/USD:BTC"), the implementation automatically
//! routes to the appropriate DAPI endpoint based on the market's `inverse` flag.
//!
//! This module exists for any DAPI-specific methods that don't have FAPI equivalents.
//! Currently, all common futures operations are unified in the `futures` module.

use super::super::Binance;

impl Binance {
    // DAPI-specific methods would go here if needed.
    // Currently, all common futures operations (positions, leverage, funding rates)
    // are handled by the unified methods in futures.rs which automatically
    // route to DAPI endpoints for inverse/coin-margined contracts.
}
