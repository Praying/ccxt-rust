//! Exchange Capabilities Module
//!
//! This module provides efficient representation and manipulation of exchange capabilities
//! using bitflags for compact storage (8 bytes instead of 46+ bytes) and type-safe operations.
//!
// Allow missing docs for bitflags-generated constants which are self-documenting by name
#![allow(missing_docs)]
//!
//! # Design
//!
//! The capability system uses a hybrid approach:
//! - `Capability` enum: Individual capability identifiers for type-safe API
//! - `Capabilities` bitflags: Efficient storage and set operations
//! - `ExchangeCapabilities` struct: High-level API with backward compatibility
//!
//! # Example
//!
//! ```rust
//! use ccxt_core::capability::{Capability, Capabilities, ExchangeCapabilities};
//!
//! // Using bitflags directly
//! let caps = Capabilities::MARKET_DATA | Capabilities::TRADING;
//! assert!(caps.contains(Capabilities::FETCH_TICKER));
//!
//! // Using the high-level API
//! let exchange_caps = ExchangeCapabilities::public_only();
//! assert!(exchange_caps.has("fetchTicker"));
//! assert!(!exchange_caps.has("createOrder"));
//! ```

use std::fmt;

mod exchange_caps;
mod flags;
mod macros;

pub use exchange_caps::{ExchangeCapabilities, ExchangeCapabilitiesBuilder, TraitCategory};
pub use flags::Capabilities;

// ============================================================================
// Capability Enum
// ============================================================================

/// Individual capability identifier
///
/// This enum provides type-safe capability names that can be converted to/from
/// bitflags and string representations. It supports all 46 exchange capabilities
/// organized into logical categories.
///
/// # Categories
///
/// - **Market Data**: Public API endpoints for market information
/// - **Trading**: Order management and execution
/// - **Account**: Balance and trade history
/// - **Funding**: Deposits, withdrawals, and transfers
/// - **Margin**: Margin/futures trading features
/// - **WebSocket**: Real-time data streaming
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Capability {
    // ==================== Market Data (0-8) ====================
    FetchMarkets = 0,
    FetchCurrencies = 1,
    FetchTicker = 2,
    FetchTickers = 3,
    FetchOrderBook = 4,
    FetchTrades = 5,
    FetchOhlcv = 6,
    FetchStatus = 7,
    FetchTime = 8,

    // ==================== Trading (9-19) ====================
    CreateOrder = 9,
    CreateMarketOrder = 10,
    CreateLimitOrder = 11,
    CancelOrder = 12,
    CancelAllOrders = 13,
    EditOrder = 14,
    FetchOrder = 15,
    FetchOrders = 16,
    FetchOpenOrders = 17,
    FetchClosedOrders = 18,
    FetchCanceledOrders = 19,

    // ==================== Account (20-25) ====================
    FetchBalance = 20,
    FetchMyTrades = 21,
    FetchDeposits = 22,
    FetchWithdrawals = 23,
    FetchTransactions = 24,
    FetchLedger = 25,

    // ==================== Funding (26-29) ====================
    FetchDepositAddress = 26,
    CreateDepositAddress = 27,
    Withdraw = 28,
    Transfer = 29,

    // ==================== Margin Trading (30-36) ====================
    FetchBorrowRate = 30,
    FetchBorrowRates = 31,
    FetchFundingRate = 32,
    FetchFundingRates = 33,
    FetchPositions = 34,
    SetLeverage = 35,
    SetMarginMode = 36,

    // ==================== WebSocket (37-45) ====================
    Websocket = 37,
    WatchTicker = 38,
    WatchTickers = 39,
    WatchOrderBook = 40,
    WatchTrades = 41,
    WatchOhlcv = 42,
    WatchBalance = 43,
    WatchOrders = 44,
    WatchMyTrades = 45,
}

impl Capability {
    /// Total number of defined capabilities
    pub const COUNT: usize = 46;

    /// Get the CCXT-style camelCase name for this capability
    pub const fn as_ccxt_name(&self) -> &'static str {
        match self {
            // Market Data
            Self::FetchMarkets => "fetchMarkets",
            Self::FetchCurrencies => "fetchCurrencies",
            Self::FetchTicker => "fetchTicker",
            Self::FetchTickers => "fetchTickers",
            Self::FetchOrderBook => "fetchOrderBook",
            Self::FetchTrades => "fetchTrades",
            Self::FetchOhlcv => "fetchOHLCV",
            Self::FetchStatus => "fetchStatus",
            Self::FetchTime => "fetchTime",
            // Trading
            Self::CreateOrder => "createOrder",
            Self::CreateMarketOrder => "createMarketOrder",
            Self::CreateLimitOrder => "createLimitOrder",
            Self::CancelOrder => "cancelOrder",
            Self::CancelAllOrders => "cancelAllOrders",
            Self::EditOrder => "editOrder",
            Self::FetchOrder => "fetchOrder",
            Self::FetchOrders => "fetchOrders",
            Self::FetchOpenOrders => "fetchOpenOrders",
            Self::FetchClosedOrders => "fetchClosedOrders",
            Self::FetchCanceledOrders => "fetchCanceledOrders",
            // Account
            Self::FetchBalance => "fetchBalance",
            Self::FetchMyTrades => "fetchMyTrades",
            Self::FetchDeposits => "fetchDeposits",
            Self::FetchWithdrawals => "fetchWithdrawals",
            Self::FetchTransactions => "fetchTransactions",
            Self::FetchLedger => "fetchLedger",
            // Funding
            Self::FetchDepositAddress => "fetchDepositAddress",
            Self::CreateDepositAddress => "createDepositAddress",
            Self::Withdraw => "withdraw",
            Self::Transfer => "transfer",
            // Margin
            Self::FetchBorrowRate => "fetchBorrowRate",
            Self::FetchBorrowRates => "fetchBorrowRates",
            Self::FetchFundingRate => "fetchFundingRate",
            Self::FetchFundingRates => "fetchFundingRates",
            Self::FetchPositions => "fetchPositions",
            Self::SetLeverage => "setLeverage",
            Self::SetMarginMode => "setMarginMode",
            // WebSocket
            Self::Websocket => "websocket",
            Self::WatchTicker => "watchTicker",
            Self::WatchTickers => "watchTickers",
            Self::WatchOrderBook => "watchOrderBook",
            Self::WatchTrades => "watchTrades",
            Self::WatchOhlcv => "watchOHLCV",
            Self::WatchBalance => "watchBalance",
            Self::WatchOrders => "watchOrders",
            Self::WatchMyTrades => "watchMyTrades",
        }
    }

    /// Parse a CCXT-style camelCase name into a Capability
    pub fn from_ccxt_name(name: &str) -> Option<Self> {
        match name {
            // Market Data
            "fetchMarkets" => Some(Self::FetchMarkets),
            "fetchCurrencies" => Some(Self::FetchCurrencies),
            "fetchTicker" => Some(Self::FetchTicker),
            "fetchTickers" => Some(Self::FetchTickers),
            "fetchOrderBook" => Some(Self::FetchOrderBook),
            "fetchTrades" => Some(Self::FetchTrades),
            "fetchOHLCV" => Some(Self::FetchOhlcv),
            "fetchStatus" => Some(Self::FetchStatus),
            "fetchTime" => Some(Self::FetchTime),
            // Trading
            "createOrder" => Some(Self::CreateOrder),
            "createMarketOrder" => Some(Self::CreateMarketOrder),
            "createLimitOrder" => Some(Self::CreateLimitOrder),
            "cancelOrder" => Some(Self::CancelOrder),
            "cancelAllOrders" => Some(Self::CancelAllOrders),
            "editOrder" => Some(Self::EditOrder),
            "fetchOrder" => Some(Self::FetchOrder),
            "fetchOrders" => Some(Self::FetchOrders),
            "fetchOpenOrders" => Some(Self::FetchOpenOrders),
            "fetchClosedOrders" => Some(Self::FetchClosedOrders),
            "fetchCanceledOrders" => Some(Self::FetchCanceledOrders),
            // Account
            "fetchBalance" => Some(Self::FetchBalance),
            "fetchMyTrades" => Some(Self::FetchMyTrades),
            "fetchDeposits" => Some(Self::FetchDeposits),
            "fetchWithdrawals" => Some(Self::FetchWithdrawals),
            "fetchTransactions" => Some(Self::FetchTransactions),
            "fetchLedger" => Some(Self::FetchLedger),
            // Funding
            "fetchDepositAddress" => Some(Self::FetchDepositAddress),
            "createDepositAddress" => Some(Self::CreateDepositAddress),
            "withdraw" => Some(Self::Withdraw),
            "transfer" => Some(Self::Transfer),
            // Margin
            "fetchBorrowRate" => Some(Self::FetchBorrowRate),
            "fetchBorrowRates" => Some(Self::FetchBorrowRates),
            "fetchFundingRate" => Some(Self::FetchFundingRate),
            "fetchFundingRates" => Some(Self::FetchFundingRates),
            "fetchPositions" => Some(Self::FetchPositions),
            "setLeverage" => Some(Self::SetLeverage),
            "setMarginMode" => Some(Self::SetMarginMode),
            // WebSocket
            "websocket" => Some(Self::Websocket),
            "watchTicker" => Some(Self::WatchTicker),
            "watchTickers" => Some(Self::WatchTickers),
            "watchOrderBook" => Some(Self::WatchOrderBook),
            "watchTrades" => Some(Self::WatchTrades),
            "watchOHLCV" => Some(Self::WatchOhlcv),
            "watchBalance" => Some(Self::WatchBalance),
            "watchOrders" => Some(Self::WatchOrders),
            "watchMyTrades" => Some(Self::WatchMyTrades),
            _ => None,
        }
    }

    /// Get all capabilities as an array
    pub const fn all() -> [Self; Self::COUNT] {
        [
            Self::FetchMarkets,
            Self::FetchCurrencies,
            Self::FetchTicker,
            Self::FetchTickers,
            Self::FetchOrderBook,
            Self::FetchTrades,
            Self::FetchOhlcv,
            Self::FetchStatus,
            Self::FetchTime,
            Self::CreateOrder,
            Self::CreateMarketOrder,
            Self::CreateLimitOrder,
            Self::CancelOrder,
            Self::CancelAllOrders,
            Self::EditOrder,
            Self::FetchOrder,
            Self::FetchOrders,
            Self::FetchOpenOrders,
            Self::FetchClosedOrders,
            Self::FetchCanceledOrders,
            Self::FetchBalance,
            Self::FetchMyTrades,
            Self::FetchDeposits,
            Self::FetchWithdrawals,
            Self::FetchTransactions,
            Self::FetchLedger,
            Self::FetchDepositAddress,
            Self::CreateDepositAddress,
            Self::Withdraw,
            Self::Transfer,
            Self::FetchBorrowRate,
            Self::FetchBorrowRates,
            Self::FetchFundingRate,
            Self::FetchFundingRates,
            Self::FetchPositions,
            Self::SetLeverage,
            Self::SetMarginMode,
            Self::Websocket,
            Self::WatchTicker,
            Self::WatchTickers,
            Self::WatchOrderBook,
            Self::WatchTrades,
            Self::WatchOhlcv,
            Self::WatchBalance,
            Self::WatchOrders,
            Self::WatchMyTrades,
        ]
    }

    /// Convert to bit position for bitflags
    #[inline]
    pub const fn bit_position(&self) -> u64 {
        1u64 << (*self as u8)
    }

    /// Get the trait category this capability belongs to
    pub const fn trait_category(&self) -> TraitCategory {
        match self {
            // Market Data
            Self::FetchMarkets
            | Self::FetchCurrencies
            | Self::FetchTicker
            | Self::FetchTickers
            | Self::FetchOrderBook
            | Self::FetchTrades
            | Self::FetchOhlcv
            | Self::FetchStatus
            | Self::FetchTime => TraitCategory::MarketData,

            // Trading
            Self::CreateOrder
            | Self::CreateMarketOrder
            | Self::CreateLimitOrder
            | Self::CancelOrder
            | Self::CancelAllOrders
            | Self::EditOrder
            | Self::FetchOrder
            | Self::FetchOrders
            | Self::FetchOpenOrders
            | Self::FetchClosedOrders
            | Self::FetchCanceledOrders => TraitCategory::Trading,

            // Account
            Self::FetchBalance
            | Self::FetchMyTrades
            | Self::FetchDeposits
            | Self::FetchWithdrawals
            | Self::FetchTransactions
            | Self::FetchLedger => TraitCategory::Account,

            // Funding
            Self::FetchDepositAddress
            | Self::CreateDepositAddress
            | Self::Withdraw
            | Self::Transfer => TraitCategory::Funding,

            // Margin
            Self::FetchBorrowRate
            | Self::FetchBorrowRates
            | Self::FetchFundingRate
            | Self::FetchFundingRates
            | Self::FetchPositions
            | Self::SetLeverage
            | Self::SetMarginMode => TraitCategory::Margin,

            // WebSocket
            Self::Websocket
            | Self::WatchTicker
            | Self::WatchTickers
            | Self::WatchOrderBook
            | Self::WatchTrades
            | Self::WatchOhlcv
            | Self::WatchBalance
            | Self::WatchOrders
            | Self::WatchMyTrades => TraitCategory::WebSocket,
        }
    }
}

impl fmt::Display for Capability {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_ccxt_name())
    }
}

// ============================================================================
// Unit Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capabilities;

    #[test]
    fn test_capability_count() {
        assert_eq!(Capability::COUNT, 46);
    }

    #[test]
    fn test_capability_ccxt_names() {
        assert_eq!(Capability::FetchTicker.as_ccxt_name(), "fetchTicker");
        assert_eq!(Capability::CreateOrder.as_ccxt_name(), "createOrder");
        assert_eq!(Capability::FetchOhlcv.as_ccxt_name(), "fetchOHLCV");
        assert_eq!(Capability::WatchOhlcv.as_ccxt_name(), "watchOHLCV");
    }

    #[test]
    fn test_capability_from_ccxt_name() {
        assert_eq!(
            Capability::from_ccxt_name("fetchTicker"),
            Some(Capability::FetchTicker)
        );
        assert_eq!(
            Capability::from_ccxt_name("createOrder"),
            Some(Capability::CreateOrder)
        );
        assert_eq!(Capability::from_ccxt_name("unknown"), None);
    }

    #[test]
    fn test_capabilities_bitflags() {
        let caps = Capabilities::FETCH_TICKER | Capabilities::CREATE_ORDER;
        assert!(caps.contains(Capabilities::FETCH_TICKER));
        assert!(caps.contains(Capabilities::CREATE_ORDER));
        assert!(!caps.contains(Capabilities::WEBSOCKET));
    }

    #[test]
    fn test_capabilities_presets() {
        assert!(Capabilities::MARKET_DATA.contains(Capabilities::FETCH_TICKER));
        assert!(Capabilities::MARKET_DATA.contains(Capabilities::FETCH_ORDER_BOOK));
        assert!(!Capabilities::MARKET_DATA.contains(Capabilities::CREATE_ORDER));

        assert!(Capabilities::TRADING.contains(Capabilities::CREATE_ORDER));
        assert!(Capabilities::TRADING.contains(Capabilities::CANCEL_ORDER));
        assert!(!Capabilities::TRADING.contains(Capabilities::FETCH_TICKER));
    }

    #[test]
    fn test_capabilities_has() {
        let caps = Capabilities::MARKET_DATA;
        assert!(caps.has("fetchTicker"));
        assert!(caps.has("fetchOrderBook"));
        assert!(!caps.has("createOrder"));
        assert!(!caps.has("unknownCapability"));
    }

    #[test]
    fn test_capabilities_count() {
        assert_eq!(Capabilities::empty().count(), 0);
        assert_eq!(Capabilities::FETCH_TICKER.count(), 1);
        assert_eq!(
            (Capabilities::FETCH_TICKER | Capabilities::CREATE_ORDER).count(),
            2
        );
        assert_eq!(Capabilities::MARKET_DATA.count(), 9);
    }

    #[test]
    fn test_capabilities_from_iter() {
        let caps = Capabilities::from_iter([
            Capability::FetchTicker,
            Capability::CreateOrder,
            Capability::Websocket,
        ]);
        assert!(caps.contains(Capabilities::FETCH_TICKER));
        assert!(caps.contains(Capabilities::CREATE_ORDER));
        assert!(caps.contains(Capabilities::WEBSOCKET));
        assert_eq!(caps.count(), 3);
    }

    #[test]
    fn test_exchange_capabilities_all() {
        let caps = ExchangeCapabilities::all();
        assert!(caps.fetch_ticker());
        assert!(caps.create_order());
        assert!(caps.websocket());
        assert!(caps.fetch_positions());
    }

    #[test]
    fn test_exchange_capabilities_public_only() {
        let caps = ExchangeCapabilities::public_only();
        assert!(caps.fetch_ticker());
        assert!(caps.fetch_order_book());
        assert!(!caps.create_order());
        assert!(!caps.fetch_balance());
    }

    #[test]
    fn test_exchange_capabilities_has() {
        let caps = ExchangeCapabilities::all();
        assert!(caps.has("fetchTicker"));
        assert!(caps.has("createOrder"));
        assert!(!caps.has("unknownCapability"));
    }

    #[test]
    fn test_exchange_capabilities_builder() {
        let caps = ExchangeCapabilities::builder()
            .market_data()
            .trading()
            .build();

        assert!(caps.fetch_ticker());
        assert!(caps.create_order());
        assert!(!caps.websocket());
    }

    #[test]
    fn test_exchange_capabilities_builder_with_capability() {
        let caps = ExchangeCapabilities::builder()
            .capability(Capability::FetchTicker)
            .capability(Capability::Websocket)
            .build();

        assert!(caps.fetch_ticker());
        assert!(caps.websocket());
        assert!(!caps.create_order());
    }

    #[test]
    fn test_exchange_capabilities_builder_without() {
        let caps = ExchangeCapabilities::builder()
            .all()
            .without_capability(Capability::Websocket)
            .build();

        assert!(caps.fetch_ticker());
        assert!(caps.create_order());
        assert!(!caps.websocket());
    }

    #[test]
    fn test_exchange_capabilities_presets() {
        let spot = ExchangeCapabilities::spot_exchange();
        assert!(spot.fetch_ticker());
        assert!(spot.create_order());
        assert!(spot.websocket());
        assert!(!spot.fetch_positions());

        let futures = ExchangeCapabilities::futures_exchange();
        assert!(futures.fetch_ticker());
        assert!(futures.create_order());
        assert!(futures.fetch_positions());
        assert!(futures.set_leverage());
    }

    #[test]
    fn test_capabilities_macro() {
        let caps = capabilities!(MARKET_DATA);
        assert!(caps.contains(Capabilities::FETCH_TICKER));

        let caps = capabilities!(FETCH_TICKER | CREATE_ORDER);
        assert!(caps.contains(Capabilities::FETCH_TICKER));
        assert!(caps.contains(Capabilities::CREATE_ORDER));

        let caps = capabilities!(MARKET_DATA, TRADING);
        assert!(caps.contains(Capabilities::FETCH_TICKER));
        assert!(caps.contains(Capabilities::CREATE_ORDER));
    }

    #[test]
    fn test_capability_bit_positions() {
        assert_eq!(Capability::FetchMarkets.bit_position(), 1 << 0);
        assert_eq!(Capability::FetchTicker.bit_position(), 1 << 2);
        assert_eq!(Capability::CreateOrder.bit_position(), 1 << 9);
        assert_eq!(Capability::Websocket.bit_position(), 1 << 37);
    }

    #[test]
    fn test_memory_efficiency() {
        assert_eq!(std::mem::size_of::<ExchangeCapabilities>(), 8);
        assert_eq!(std::mem::size_of::<Capabilities>(), 8);
    }

    #[test]
    fn test_trait_category_all() {
        let categories = TraitCategory::all();
        assert_eq!(categories.len(), 7);
        assert!(categories.contains(&TraitCategory::PublicExchange));
        assert!(categories.contains(&TraitCategory::MarketData));
        assert!(categories.contains(&TraitCategory::Trading));
        assert!(categories.contains(&TraitCategory::Account));
        assert!(categories.contains(&TraitCategory::Margin));
        assert!(categories.contains(&TraitCategory::Funding));
        assert!(categories.contains(&TraitCategory::WebSocket));
    }

    #[test]
    fn test_capability_trait_category() {
        assert_eq!(
            Capability::FetchTicker.trait_category(),
            TraitCategory::MarketData
        );
        assert_eq!(
            Capability::CreateOrder.trait_category(),
            TraitCategory::Trading
        );
        assert_eq!(
            Capability::FetchBalance.trait_category(),
            TraitCategory::Account
        );
    }

    #[test]
    fn test_supports_market_data() {
        let caps = ExchangeCapabilities::public_only();
        assert!(caps.supports_market_data());

        let caps = ExchangeCapabilities::none();
        assert!(!caps.supports_market_data());
    }

    #[test]
    fn test_supports_trading() {
        let caps = ExchangeCapabilities::all();
        assert!(caps.supports_trading());

        let caps = ExchangeCapabilities::public_only();
        assert!(!caps.supports_trading());
    }

    #[test]
    fn test_supports_full_exchange() {
        let caps = ExchangeCapabilities::all();
        assert!(caps.supports_full_exchange());

        let caps = ExchangeCapabilities::spot_exchange();
        assert!(!caps.supports_full_exchange());
    }
}
