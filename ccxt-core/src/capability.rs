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

use bitflags::bitflags;
use std::fmt;

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
    /// Can fetch market definitions
    FetchMarkets = 0,
    /// Can fetch currency definitions
    FetchCurrencies = 1,
    /// Can fetch single ticker
    FetchTicker = 2,
    /// Can fetch multiple tickers
    FetchTickers = 3,
    /// Can fetch order book
    FetchOrderBook = 4,
    /// Can fetch public trades
    FetchTrades = 5,
    /// Can fetch OHLCV candlestick data
    FetchOhlcv = 6,
    /// Can fetch exchange status
    FetchStatus = 7,
    /// Can fetch server time
    FetchTime = 8,

    // ==================== Trading (9-19) ====================
    /// Can create orders
    CreateOrder = 9,
    /// Can create market orders
    CreateMarketOrder = 10,
    /// Can create limit orders
    CreateLimitOrder = 11,
    /// Can cancel orders
    CancelOrder = 12,
    /// Can cancel all orders
    CancelAllOrders = 13,
    /// Can edit/modify orders
    EditOrder = 14,
    /// Can fetch single order
    FetchOrder = 15,
    /// Can fetch all orders
    FetchOrders = 16,
    /// Can fetch open orders
    FetchOpenOrders = 17,
    /// Can fetch closed orders
    FetchClosedOrders = 18,
    /// Can fetch canceled orders
    FetchCanceledOrders = 19,

    // ==================== Account (20-25) ====================
    /// Can fetch account balance
    FetchBalance = 20,
    /// Can fetch user's trade history
    FetchMyTrades = 21,
    /// Can fetch deposit history
    FetchDeposits = 22,
    /// Can fetch withdrawal history
    FetchWithdrawals = 23,
    /// Can fetch transaction history
    FetchTransactions = 24,
    /// Can fetch ledger entries
    FetchLedger = 25,

    // ==================== Funding (26-29) ====================
    /// Can fetch deposit address
    FetchDepositAddress = 26,
    /// Can create deposit address
    CreateDepositAddress = 27,
    /// Can withdraw funds
    Withdraw = 28,
    /// Can transfer between accounts
    Transfer = 29,

    // ==================== Margin Trading (30-36) ====================
    /// Can fetch borrow rate
    FetchBorrowRate = 30,
    /// Can fetch multiple borrow rates
    FetchBorrowRates = 31,
    /// Can fetch funding rate
    FetchFundingRate = 32,
    /// Can fetch multiple funding rates
    FetchFundingRates = 33,
    /// Can fetch positions
    FetchPositions = 34,
    /// Can set leverage
    SetLeverage = 35,
    /// Can set margin mode
    SetMarginMode = 36,

    // ==================== WebSocket (37-45) ====================
    /// WebSocket support available
    Websocket = 37,
    /// Can watch ticker updates
    WatchTicker = 38,
    /// Can watch multiple ticker updates
    WatchTickers = 39,
    /// Can watch order book updates
    WatchOrderBook = 40,
    /// Can watch trade updates
    WatchTrades = 41,
    /// Can watch OHLCV updates
    WatchOhlcv = 42,
    /// Can watch balance updates
    WatchBalance = 43,
    /// Can watch order updates
    WatchOrders = 44,
    /// Can watch user trade updates
    WatchMyTrades = 45,
}

impl Capability {
    /// Total number of defined capabilities
    pub const COUNT: usize = 46;

    /// Get the CCXT-style camelCase name for this capability
    ///
    /// # Example
    ///
    /// ```rust
    /// use ccxt_core::capability::Capability;
    ///
    /// assert_eq!(Capability::FetchTicker.as_ccxt_name(), "fetchTicker");
    /// assert_eq!(Capability::CreateOrder.as_ccxt_name(), "createOrder");
    /// ```
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
    ///
    /// # Example
    ///
    /// ```rust
    /// use ccxt_core::capability::Capability;
    ///
    /// assert_eq!(Capability::from_ccxt_name("fetchTicker"), Some(Capability::FetchTicker));
    /// assert_eq!(Capability::from_ccxt_name("unknown"), None);
    /// ```
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
}

impl fmt::Display for Capability {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_ccxt_name())
    }
}

// ============================================================================
// Capabilities Bitflags
// ============================================================================

bitflags! {
    /// Efficient bitflags representation of exchange capabilities
    ///
    /// Uses 64 bits to store all 46 capabilities, providing:
    /// - Compact storage (8 bytes vs 46+ bytes for booleans)
    /// - Fast set operations (union, intersection, difference)
    /// - Type-safe capability combinations
    ///
    /// # Predefined Sets
    ///
    /// - `MARKET_DATA`: All public market data capabilities
    /// - `TRADING`: All trading capabilities
    /// - `ACCOUNT`: All account-related capabilities
    /// - `FUNDING`: All funding capabilities
    /// - `MARGIN`: All margin/futures capabilities
    /// - `WEBSOCKET`: All WebSocket capabilities
    /// - `ALL`: All capabilities enabled
    /// - `NONE`: No capabilities
    ///
    /// # Example
    ///
    /// ```rust
    /// use ccxt_core::capability::Capabilities;
    ///
    /// let caps = Capabilities::MARKET_DATA | Capabilities::TRADING;
    /// assert!(caps.contains(Capabilities::FETCH_TICKER));
    /// assert!(caps.contains(Capabilities::CREATE_ORDER));
    /// ```
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub struct Capabilities: u64 {
        // ==================== Market Data (bits 0-8) ====================
        const FETCH_MARKETS     = 1 << 0;
        const FETCH_CURRENCIES  = 1 << 1;
        const FETCH_TICKER      = 1 << 2;
        const FETCH_TICKERS     = 1 << 3;
        const FETCH_ORDER_BOOK  = 1 << 4;
        const FETCH_TRADES      = 1 << 5;
        const FETCH_OHLCV       = 1 << 6;
        const FETCH_STATUS      = 1 << 7;
        const FETCH_TIME        = 1 << 8;

        // ==================== Trading (bits 9-19) ====================
        const CREATE_ORDER          = 1 << 9;
        const CREATE_MARKET_ORDER   = 1 << 10;
        const CREATE_LIMIT_ORDER    = 1 << 11;
        const CANCEL_ORDER          = 1 << 12;
        const CANCEL_ALL_ORDERS     = 1 << 13;
        const EDIT_ORDER            = 1 << 14;
        const FETCH_ORDER           = 1 << 15;
        const FETCH_ORDERS          = 1 << 16;
        const FETCH_OPEN_ORDERS     = 1 << 17;
        const FETCH_CLOSED_ORDERS   = 1 << 18;
        const FETCH_CANCELED_ORDERS = 1 << 19;

        // ==================== Account (bits 20-25) ====================
        const FETCH_BALANCE      = 1 << 20;
        const FETCH_MY_TRADES    = 1 << 21;
        const FETCH_DEPOSITS     = 1 << 22;
        const FETCH_WITHDRAWALS  = 1 << 23;
        const FETCH_TRANSACTIONS = 1 << 24;
        const FETCH_LEDGER       = 1 << 25;

        // ==================== Funding (bits 26-29) ====================
        const FETCH_DEPOSIT_ADDRESS  = 1 << 26;
        const CREATE_DEPOSIT_ADDRESS = 1 << 27;
        const WITHDRAW               = 1 << 28;
        const TRANSFER               = 1 << 29;

        // ==================== Margin Trading (bits 30-36) ====================
        const FETCH_BORROW_RATE   = 1 << 30;
        const FETCH_BORROW_RATES  = 1 << 31;
        const FETCH_FUNDING_RATE  = 1 << 32;
        const FETCH_FUNDING_RATES = 1 << 33;
        const FETCH_POSITIONS     = 1 << 34;
        const SET_LEVERAGE        = 1 << 35;
        const SET_MARGIN_MODE     = 1 << 36;

        // ==================== WebSocket (bits 37-45) ====================
        const WEBSOCKET        = 1 << 37;
        const WATCH_TICKER     = 1 << 38;
        const WATCH_TICKERS    = 1 << 39;
        const WATCH_ORDER_BOOK = 1 << 40;
        const WATCH_TRADES     = 1 << 41;
        const WATCH_OHLCV      = 1 << 42;
        const WATCH_BALANCE    = 1 << 43;
        const WATCH_ORDERS     = 1 << 44;
        const WATCH_MY_TRADES  = 1 << 45;

        // ==================== Category Presets ====================
        /// All public market data capabilities
        const MARKET_DATA = Self::FETCH_MARKETS.bits()
            | Self::FETCH_CURRENCIES.bits()
            | Self::FETCH_TICKER.bits()
            | Self::FETCH_TICKERS.bits()
            | Self::FETCH_ORDER_BOOK.bits()
            | Self::FETCH_TRADES.bits()
            | Self::FETCH_OHLCV.bits()
            | Self::FETCH_STATUS.bits()
            | Self::FETCH_TIME.bits();

        /// All trading capabilities
        const TRADING = Self::CREATE_ORDER.bits()
            | Self::CREATE_MARKET_ORDER.bits()
            | Self::CREATE_LIMIT_ORDER.bits()
            | Self::CANCEL_ORDER.bits()
            | Self::CANCEL_ALL_ORDERS.bits()
            | Self::EDIT_ORDER.bits()
            | Self::FETCH_ORDER.bits()
            | Self::FETCH_ORDERS.bits()
            | Self::FETCH_OPEN_ORDERS.bits()
            | Self::FETCH_CLOSED_ORDERS.bits()
            | Self::FETCH_CANCELED_ORDERS.bits();

        /// All account-related capabilities
        const ACCOUNT = Self::FETCH_BALANCE.bits()
            | Self::FETCH_MY_TRADES.bits()
            | Self::FETCH_DEPOSITS.bits()
            | Self::FETCH_WITHDRAWALS.bits()
            | Self::FETCH_TRANSACTIONS.bits()
            | Self::FETCH_LEDGER.bits();

        /// All funding capabilities
        const FUNDING = Self::FETCH_DEPOSIT_ADDRESS.bits()
            | Self::CREATE_DEPOSIT_ADDRESS.bits()
            | Self::WITHDRAW.bits()
            | Self::TRANSFER.bits();

        /// All margin/futures trading capabilities
        const MARGIN = Self::FETCH_BORROW_RATE.bits()
            | Self::FETCH_BORROW_RATES.bits()
            | Self::FETCH_FUNDING_RATE.bits()
            | Self::FETCH_FUNDING_RATES.bits()
            | Self::FETCH_POSITIONS.bits()
            | Self::SET_LEVERAGE.bits()
            | Self::SET_MARGIN_MODE.bits();

        /// All WebSocket capabilities
        const WEBSOCKET_ALL = Self::WEBSOCKET.bits()
            | Self::WATCH_TICKER.bits()
            | Self::WATCH_TICKERS.bits()
            | Self::WATCH_ORDER_BOOK.bits()
            | Self::WATCH_TRADES.bits()
            | Self::WATCH_OHLCV.bits()
            | Self::WATCH_BALANCE.bits()
            | Self::WATCH_ORDERS.bits()
            | Self::WATCH_MY_TRADES.bits();

        /// All REST API capabilities (no WebSocket)
        const REST_ALL = Self::MARKET_DATA.bits()
            | Self::TRADING.bits()
            | Self::ACCOUNT.bits()
            | Self::FUNDING.bits()
            | Self::MARGIN.bits();

        /// Public-only capabilities (no authentication required)
        const PUBLIC_ONLY = Self::MARKET_DATA.bits();

        /// All capabilities enabled
        const ALL = Self::REST_ALL.bits() | Self::WEBSOCKET_ALL.bits();
    }
}

impl Capabilities {
    /// Check if a capability is supported by CCXT-style name
    ///
    /// # Example
    ///
    /// ```rust
    /// use ccxt_core::capability::Capabilities;
    ///
    /// let caps = Capabilities::MARKET_DATA;
    /// assert!(caps.has("fetchTicker"));
    /// assert!(!caps.has("createOrder"));
    /// ```
    pub fn has(&self, capability: &str) -> bool {
        if let Some(cap) = Capability::from_ccxt_name(capability) {
            self.contains(Self::from(cap))
        } else {
            false
        }
    }

    /// Get a list of all supported capability names
    ///
    /// # Example
    ///
    /// ```rust
    /// use ccxt_core::capability::Capabilities;
    ///
    /// let caps = Capabilities::PUBLIC_ONLY;
    /// let names = caps.supported_capabilities();
    /// assert!(names.contains(&"fetchTicker"));
    /// ```
    pub fn supported_capabilities(&self) -> Vec<&'static str> {
        Capability::all()
            .iter()
            .filter(|cap| self.contains(Self::from(**cap)))
            .map(|cap| cap.as_ccxt_name())
            .collect()
    }

    /// Count the number of enabled capabilities
    #[inline]
    pub fn count(&self) -> u32 {
        self.bits().count_ones()
    }

    /// Create from an iterator of capabilities
    pub fn from_iter<I: IntoIterator<Item = Capability>>(iter: I) -> Self {
        let mut caps = Self::empty();
        for cap in iter {
            caps |= Self::from(cap);
        }
        caps
    }
}

impl From<Capability> for Capabilities {
    fn from(cap: Capability) -> Self {
        Self::from_bits_truncate(cap.bit_position())
    }
}

impl fmt::Display for Capabilities {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let caps = self.supported_capabilities();
        write!(f, "[{}]", caps.join(", "))
    }
}

// ============================================================================
// Capabilities Builder Macro
// ============================================================================

/// Macro for building Capabilities with a concise syntax
///
/// # Examples
///
/// ```rust
/// use ccxt_core::{capabilities, capability::Capabilities};
///
/// // Using predefined sets
/// let caps = capabilities!(MARKET_DATA, TRADING);
///
/// // Using individual capabilities
/// let caps = capabilities!(FETCH_TICKER, CREATE_ORDER, WEBSOCKET);
///
/// // Combining sets and individual capabilities
/// let caps = capabilities!(MARKET_DATA | FETCH_BALANCE | WEBSOCKET);
/// ```
#[macro_export]
macro_rules! capabilities {
    // Single capability or set
    ($cap:ident) => {
        $crate::capability::Capabilities::$cap
    };
    // Multiple capabilities with |
    ($cap:ident | $($rest:tt)+) => {
        $crate::capability::Capabilities::$cap | capabilities!($($rest)+)
    };
    // Multiple capabilities with ,
    ($cap:ident, $($rest:tt)+) => {
        $crate::capability::Capabilities::$cap | capabilities!($($rest)+)
    };
}

// ============================================================================
// ExchangeCapabilities Struct (Backward Compatible)
// ============================================================================

/// Exchange capabilities configuration
///
/// This struct provides a high-level API for working with exchange capabilities,
/// maintaining backward compatibility with the original boolean-field design
/// while using efficient bitflags internally.
///
/// # Example
///
/// ```rust
/// use ccxt_core::capability::ExchangeCapabilities;
///
/// let caps = ExchangeCapabilities::public_only();
/// assert!(caps.has("fetchTicker"));
/// assert!(!caps.has("createOrder"));
///
/// let caps = ExchangeCapabilities::all();
/// assert!(caps.fetch_ticker());
/// assert!(caps.create_order());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct ExchangeCapabilities {
    inner: Capabilities,
}

impl ExchangeCapabilities {
    /// Create with no capabilities enabled
    pub const fn none() -> Self {
        Self {
            inner: Capabilities::empty(),
        }
    }

    /// Create capabilities with all features enabled
    ///
    /// # Example
    ///
    /// ```rust
    /// use ccxt_core::capability::ExchangeCapabilities;
    ///
    /// let caps = ExchangeCapabilities::all();
    /// assert!(caps.fetch_ticker());
    /// assert!(caps.create_order());
    /// assert!(caps.websocket());
    /// ```
    pub const fn all() -> Self {
        Self {
            inner: Capabilities::ALL,
        }
    }

    /// Create capabilities for public API only (no authentication required)
    ///
    /// # Example
    ///
    /// ```rust
    /// use ccxt_core::capability::ExchangeCapabilities;
    ///
    /// let caps = ExchangeCapabilities::public_only();
    /// assert!(caps.fetch_ticker());
    /// assert!(!caps.create_order());
    /// ```
    pub const fn public_only() -> Self {
        Self {
            inner: Capabilities::PUBLIC_ONLY,
        }
    }

    /// Create from raw Capabilities bitflags
    pub const fn from_capabilities(caps: Capabilities) -> Self {
        Self { inner: caps }
    }

    /// Get the underlying Capabilities bitflags
    pub const fn as_capabilities(&self) -> Capabilities {
        self.inner
    }

    /// Check if a capability is supported by name
    ///
    /// This method allows checking capabilities using CCXT-style camelCase names.
    ///
    /// # Arguments
    ///
    /// * `capability` - The capability name in camelCase format
    ///
    /// # Example
    ///
    /// ```rust
    /// use ccxt_core::capability::ExchangeCapabilities;
    ///
    /// let caps = ExchangeCapabilities::all();
    /// assert!(caps.has("fetchTicker"));
    /// assert!(caps.has("createOrder"));
    /// assert!(!caps.has("unknownCapability"));
    /// ```
    pub fn has(&self, capability: &str) -> bool {
        self.inner.has(capability)
    }

    /// Get a list of all supported capability names
    pub fn supported_capabilities(&self) -> Vec<&'static str> {
        self.inner.supported_capabilities()
    }

    /// Count the number of enabled capabilities
    #[inline]
    pub fn count(&self) -> u32 {
        self.inner.count()
    }

    // ==================== Market Data Accessors ====================

    /// Can fetch market definitions
    #[inline]
    pub const fn fetch_markets(&self) -> bool {
        self.inner.contains(Capabilities::FETCH_MARKETS)
    }

    /// Can fetch currency definitions
    #[inline]
    pub const fn fetch_currencies(&self) -> bool {
        self.inner.contains(Capabilities::FETCH_CURRENCIES)
    }

    /// Can fetch single ticker
    #[inline]
    pub const fn fetch_ticker(&self) -> bool {
        self.inner.contains(Capabilities::FETCH_TICKER)
    }

    /// Can fetch multiple tickers
    #[inline]
    pub const fn fetch_tickers(&self) -> bool {
        self.inner.contains(Capabilities::FETCH_TICKERS)
    }

    /// Can fetch order book
    #[inline]
    pub const fn fetch_order_book(&self) -> bool {
        self.inner.contains(Capabilities::FETCH_ORDER_BOOK)
    }

    /// Can fetch public trades
    #[inline]
    pub const fn fetch_trades(&self) -> bool {
        self.inner.contains(Capabilities::FETCH_TRADES)
    }

    /// Can fetch OHLCV candlestick data
    #[inline]
    pub const fn fetch_ohlcv(&self) -> bool {
        self.inner.contains(Capabilities::FETCH_OHLCV)
    }

    /// Can fetch exchange status
    #[inline]
    pub const fn fetch_status(&self) -> bool {
        self.inner.contains(Capabilities::FETCH_STATUS)
    }

    /// Can fetch server time
    #[inline]
    pub const fn fetch_time(&self) -> bool {
        self.inner.contains(Capabilities::FETCH_TIME)
    }

    // ==================== Trading Accessors ====================

    /// Can create orders
    #[inline]
    pub const fn create_order(&self) -> bool {
        self.inner.contains(Capabilities::CREATE_ORDER)
    }

    /// Can create market orders
    #[inline]
    pub const fn create_market_order(&self) -> bool {
        self.inner.contains(Capabilities::CREATE_MARKET_ORDER)
    }

    /// Can create limit orders
    #[inline]
    pub const fn create_limit_order(&self) -> bool {
        self.inner.contains(Capabilities::CREATE_LIMIT_ORDER)
    }

    /// Can cancel orders
    #[inline]
    pub const fn cancel_order(&self) -> bool {
        self.inner.contains(Capabilities::CANCEL_ORDER)
    }

    /// Can cancel all orders
    #[inline]
    pub const fn cancel_all_orders(&self) -> bool {
        self.inner.contains(Capabilities::CANCEL_ALL_ORDERS)
    }

    /// Can edit/modify orders
    #[inline]
    pub const fn edit_order(&self) -> bool {
        self.inner.contains(Capabilities::EDIT_ORDER)
    }

    /// Can fetch single order
    #[inline]
    pub const fn fetch_order(&self) -> bool {
        self.inner.contains(Capabilities::FETCH_ORDER)
    }

    /// Can fetch all orders
    #[inline]
    pub const fn fetch_orders(&self) -> bool {
        self.inner.contains(Capabilities::FETCH_ORDERS)
    }

    /// Can fetch open orders
    #[inline]
    pub const fn fetch_open_orders(&self) -> bool {
        self.inner.contains(Capabilities::FETCH_OPEN_ORDERS)
    }

    /// Can fetch closed orders
    #[inline]
    pub const fn fetch_closed_orders(&self) -> bool {
        self.inner.contains(Capabilities::FETCH_CLOSED_ORDERS)
    }

    /// Can fetch canceled orders
    #[inline]
    pub const fn fetch_canceled_orders(&self) -> bool {
        self.inner.contains(Capabilities::FETCH_CANCELED_ORDERS)
    }

    // ==================== Account Accessors ====================

    /// Can fetch account balance
    #[inline]
    pub const fn fetch_balance(&self) -> bool {
        self.inner.contains(Capabilities::FETCH_BALANCE)
    }

    /// Can fetch user's trade history
    #[inline]
    pub const fn fetch_my_trades(&self) -> bool {
        self.inner.contains(Capabilities::FETCH_MY_TRADES)
    }

    /// Can fetch deposit history
    #[inline]
    pub const fn fetch_deposits(&self) -> bool {
        self.inner.contains(Capabilities::FETCH_DEPOSITS)
    }

    /// Can fetch withdrawal history
    #[inline]
    pub const fn fetch_withdrawals(&self) -> bool {
        self.inner.contains(Capabilities::FETCH_WITHDRAWALS)
    }

    /// Can fetch transaction history
    #[inline]
    pub const fn fetch_transactions(&self) -> bool {
        self.inner.contains(Capabilities::FETCH_TRANSACTIONS)
    }

    /// Can fetch ledger entries
    #[inline]
    pub const fn fetch_ledger(&self) -> bool {
        self.inner.contains(Capabilities::FETCH_LEDGER)
    }

    // ==================== Funding Accessors ====================

    /// Can fetch deposit address
    #[inline]
    pub const fn fetch_deposit_address(&self) -> bool {
        self.inner.contains(Capabilities::FETCH_DEPOSIT_ADDRESS)
    }

    /// Can create deposit address
    #[inline]
    pub const fn create_deposit_address(&self) -> bool {
        self.inner.contains(Capabilities::CREATE_DEPOSIT_ADDRESS)
    }

    /// Can withdraw funds
    #[inline]
    pub const fn withdraw(&self) -> bool {
        self.inner.contains(Capabilities::WITHDRAW)
    }

    /// Can transfer between accounts
    #[inline]
    pub const fn transfer(&self) -> bool {
        self.inner.contains(Capabilities::TRANSFER)
    }

    // ==================== Margin Trading Accessors ====================

    /// Can fetch borrow rate
    #[inline]
    pub const fn fetch_borrow_rate(&self) -> bool {
        self.inner.contains(Capabilities::FETCH_BORROW_RATE)
    }

    /// Can fetch multiple borrow rates
    #[inline]
    pub const fn fetch_borrow_rates(&self) -> bool {
        self.inner.contains(Capabilities::FETCH_BORROW_RATES)
    }

    /// Can fetch funding rate
    #[inline]
    pub const fn fetch_funding_rate(&self) -> bool {
        self.inner.contains(Capabilities::FETCH_FUNDING_RATE)
    }

    /// Can fetch multiple funding rates
    #[inline]
    pub const fn fetch_funding_rates(&self) -> bool {
        self.inner.contains(Capabilities::FETCH_FUNDING_RATES)
    }

    /// Can fetch positions
    #[inline]
    pub const fn fetch_positions(&self) -> bool {
        self.inner.contains(Capabilities::FETCH_POSITIONS)
    }

    /// Can set leverage
    #[inline]
    pub const fn set_leverage(&self) -> bool {
        self.inner.contains(Capabilities::SET_LEVERAGE)
    }

    /// Can set margin mode
    #[inline]
    pub const fn set_margin_mode(&self) -> bool {
        self.inner.contains(Capabilities::SET_MARGIN_MODE)
    }

    // ==================== WebSocket Accessors ====================

    /// WebSocket support available
    #[inline]
    pub const fn websocket(&self) -> bool {
        self.inner.contains(Capabilities::WEBSOCKET)
    }

    /// Can watch ticker updates
    #[inline]
    pub const fn watch_ticker(&self) -> bool {
        self.inner.contains(Capabilities::WATCH_TICKER)
    }

    /// Can watch multiple ticker updates
    #[inline]
    pub const fn watch_tickers(&self) -> bool {
        self.inner.contains(Capabilities::WATCH_TICKERS)
    }

    /// Can watch order book updates
    #[inline]
    pub const fn watch_order_book(&self) -> bool {
        self.inner.contains(Capabilities::WATCH_ORDER_BOOK)
    }

    /// Can watch trade updates
    #[inline]
    pub const fn watch_trades(&self) -> bool {
        self.inner.contains(Capabilities::WATCH_TRADES)
    }

    /// Can watch OHLCV updates
    #[inline]
    pub const fn watch_ohlcv(&self) -> bool {
        self.inner.contains(Capabilities::WATCH_OHLCV)
    }

    /// Can watch balance updates
    #[inline]
    pub const fn watch_balance(&self) -> bool {
        self.inner.contains(Capabilities::WATCH_BALANCE)
    }

    /// Can watch order updates
    #[inline]
    pub const fn watch_orders(&self) -> bool {
        self.inner.contains(Capabilities::WATCH_ORDERS)
    }

    /// Can watch user trade updates
    #[inline]
    pub const fn watch_my_trades(&self) -> bool {
        self.inner.contains(Capabilities::WATCH_MY_TRADES)
    }

    // ==================== Builder Method ====================

    /// Create a builder for ExchangeCapabilities
    ///
    /// # Example
    ///
    /// ```rust
    /// use ccxt_core::capability::ExchangeCapabilities;
    ///
    /// let caps = ExchangeCapabilities::builder()
    ///     .market_data()
    ///     .trading()
    ///     .websocket()
    ///     .build();
    /// assert!(caps.fetch_ticker());
    /// assert!(caps.create_order());
    /// assert!(caps.websocket());
    /// ```
    pub fn builder() -> ExchangeCapabilitiesBuilder {
        ExchangeCapabilitiesBuilder::new()
    }
}

impl From<Capabilities> for ExchangeCapabilities {
    fn from(caps: Capabilities) -> Self {
        Self::from_capabilities(caps)
    }
}

impl fmt::Display for ExchangeCapabilities {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ExchangeCapabilities({})", self.inner)
    }
}

// ============================================================================
// ExchangeCapabilities Builder
// ============================================================================

/// Builder for ExchangeCapabilities
///
/// Provides a fluent API for constructing capability sets.
///
/// # Example
///
/// ```rust
/// use ccxt_core::capability::ExchangeCapabilitiesBuilder;
///
/// let caps = ExchangeCapabilitiesBuilder::new()
///     .market_data()
///     .trading()
///     .capability(ccxt_core::capability::Capability::Websocket)
///     .build();
/// ```
#[derive(Debug, Clone, Default)]
pub struct ExchangeCapabilitiesBuilder {
    inner: Capabilities,
}

impl ExchangeCapabilitiesBuilder {
    /// Create a new builder with no capabilities
    pub fn new() -> Self {
        Self {
            inner: Capabilities::empty(),
        }
    }

    /// Add all market data capabilities
    pub fn market_data(mut self) -> Self {
        self.inner |= Capabilities::MARKET_DATA;
        self
    }

    /// Add all trading capabilities
    pub fn trading(mut self) -> Self {
        self.inner |= Capabilities::TRADING;
        self
    }

    /// Add all account capabilities
    pub fn account(mut self) -> Self {
        self.inner |= Capabilities::ACCOUNT;
        self
    }

    /// Add all funding capabilities
    pub fn funding(mut self) -> Self {
        self.inner |= Capabilities::FUNDING;
        self
    }

    /// Add all margin trading capabilities
    pub fn margin(mut self) -> Self {
        self.inner |= Capabilities::MARGIN;
        self
    }

    /// Add all WebSocket capabilities
    pub fn websocket_all(mut self) -> Self {
        self.inner |= Capabilities::WEBSOCKET_ALL;
        self
    }

    /// Add just the WebSocket base capability
    pub fn websocket(mut self) -> Self {
        self.inner |= Capabilities::WEBSOCKET;
        self
    }

    /// Add all REST API capabilities
    pub fn rest_all(mut self) -> Self {
        self.inner |= Capabilities::REST_ALL;
        self
    }

    /// Add all capabilities
    pub fn all(mut self) -> Self {
        self.inner = Capabilities::ALL;
        self
    }

    /// Add a specific capability
    pub fn capability(mut self, cap: Capability) -> Self {
        self.inner |= Capabilities::from(cap);
        self
    }

    /// Add multiple capabilities
    pub fn capabilities<I: IntoIterator<Item = Capability>>(mut self, caps: I) -> Self {
        for cap in caps {
            self.inner |= Capabilities::from(cap);
        }
        self
    }

    /// Add raw Capabilities bitflags
    pub fn raw(mut self, caps: Capabilities) -> Self {
        self.inner |= caps;
        self
    }

    /// Remove a specific capability
    pub fn without_capability(mut self, cap: Capability) -> Self {
        self.inner.remove(Capabilities::from(cap));
        self
    }

    /// Remove capabilities
    pub fn without(mut self, caps: Capabilities) -> Self {
        self.inner.remove(caps);
        self
    }

    /// Build the ExchangeCapabilities
    pub fn build(self) -> ExchangeCapabilities {
        ExchangeCapabilities { inner: self.inner }
    }
}

// ============================================================================
// Presets for Common Exchange Configurations
// ============================================================================

impl ExchangeCapabilities {
    /// Create capabilities for a typical spot exchange
    ///
    /// Includes: market data, trading, account, and basic WebSocket
    pub const fn spot_exchange() -> Self {
        Self {
            inner: Capabilities::from_bits_truncate(
                Capabilities::MARKET_DATA.bits()
                    | Capabilities::TRADING.bits()
                    | Capabilities::ACCOUNT.bits()
                    | Capabilities::WEBSOCKET.bits()
                    | Capabilities::WATCH_TICKER.bits()
                    | Capabilities::WATCH_ORDER_BOOK.bits()
                    | Capabilities::WATCH_TRADES.bits(),
            ),
        }
    }

    /// Create capabilities for a typical futures exchange
    ///
    /// Includes: spot capabilities + margin trading
    pub const fn futures_exchange() -> Self {
        Self {
            inner: Capabilities::from_bits_truncate(
                Capabilities::MARKET_DATA.bits()
                    | Capabilities::TRADING.bits()
                    | Capabilities::ACCOUNT.bits()
                    | Capabilities::MARGIN.bits()
                    | Capabilities::WEBSOCKET_ALL.bits(),
            ),
        }
    }

    /// Create capabilities for a full-featured exchange (like Binance)
    ///
    /// Includes: all capabilities
    pub const fn full_featured() -> Self {
        Self {
            inner: Capabilities::ALL,
        }
    }
}

// ============================================================================
// Trait-Capability Mapping
// ============================================================================

/// Trait category for capability-to-trait mapping.
///
/// This enum represents the decomposed trait hierarchy used in the exchange
/// trait system. Each variant corresponds to a specific trait that exchanges
/// can implement.
///
/// # Trait Hierarchy
///
/// ```text
/// PublicExchange (base trait - metadata, capabilities)
///     │
///     ├── MarketData (public market data)
///     ├── Trading (order management)
///     ├── Account (balance, trade history)
///     ├── Margin (positions, leverage, funding)
///     └── Funding (deposits, withdrawals, transfers)
///
/// FullExchange = PublicExchange + MarketData + Trading + Account + Margin + Funding
/// ```
///
/// # Capability Mapping
///
/// | Trait Category | Capabilities |
/// |----------------|--------------|
/// | `PublicExchange` | Base trait, always required |
/// | `MarketData` | FetchMarkets, FetchCurrencies, FetchTicker, FetchTickers, FetchOrderBook, FetchTrades, FetchOhlcv, FetchStatus, FetchTime |
/// | `Trading` | CreateOrder, CreateMarketOrder, CreateLimitOrder, CancelOrder, CancelAllOrders, EditOrder, FetchOrder, FetchOrders, FetchOpenOrders, FetchClosedOrders, FetchCanceledOrders |
/// | `Account` | FetchBalance, FetchMyTrades, FetchDeposits, FetchWithdrawals, FetchTransactions, FetchLedger |
/// | `Margin` | FetchBorrowRate, FetchBorrowRates, FetchFundingRate, FetchFundingRates, FetchPositions, SetLeverage, SetMarginMode |
/// | `Funding` | FetchDepositAddress, CreateDepositAddress, Withdraw, Transfer |
/// | `WebSocket` | Websocket, WatchTicker, WatchTickers, WatchOrderBook, WatchTrades, WatchOhlcv, WatchBalance, WatchOrders, WatchMyTrades |
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TraitCategory {
    /// Base trait providing exchange metadata and capabilities.
    /// All exchanges must implement this trait.
    PublicExchange,
    /// Public market data operations (ticker, orderbook, trades, OHLCV).
    /// Corresponds to the `MarketData` trait.
    MarketData,
    /// Order management operations (create, cancel, fetch orders).
    /// Corresponds to the `Trading` trait.
    Trading,
    /// Account-related operations (balance, trade history).
    /// Corresponds to the `Account` trait.
    Account,
    /// Margin/futures trading operations (positions, leverage, funding).
    /// Corresponds to the `Margin` trait.
    Margin,
    /// Deposit/withdrawal operations.
    /// Corresponds to the `Funding` trait.
    Funding,
    /// WebSocket real-time data streaming.
    /// Corresponds to the `WsExchange` trait.
    WebSocket,
}

impl TraitCategory {
    /// Get all trait categories.
    pub const fn all() -> [Self; 7] {
        [
            Self::PublicExchange,
            Self::MarketData,
            Self::Trading,
            Self::Account,
            Self::Margin,
            Self::Funding,
            Self::WebSocket,
        ]
    }

    /// Get the display name for this trait category.
    pub const fn name(&self) -> &'static str {
        match self {
            Self::PublicExchange => "PublicExchange",
            Self::MarketData => "MarketData",
            Self::Trading => "Trading",
            Self::Account => "Account",
            Self::Margin => "Margin",
            Self::Funding => "Funding",
            Self::WebSocket => "WebSocket",
        }
    }

    /// Get the capabilities associated with this trait category.
    pub const fn capabilities(&self) -> Capabilities {
        match self {
            Self::PublicExchange => Capabilities::empty(), // Base trait, no specific capabilities
            Self::MarketData => Capabilities::MARKET_DATA,
            Self::Trading => Capabilities::TRADING,
            Self::Account => Capabilities::ACCOUNT,
            Self::Margin => Capabilities::MARGIN,
            Self::Funding => Capabilities::FUNDING,
            Self::WebSocket => Capabilities::WEBSOCKET_ALL,
        }
    }

    /// Get the minimum required capabilities for this trait.
    ///
    /// Returns the capabilities that MUST be present for an exchange
    /// to be considered as implementing this trait.
    pub const fn minimum_capabilities(&self) -> Capabilities {
        match self {
            Self::PublicExchange => Capabilities::empty(),
            // MarketData requires at least fetch_markets and fetch_ticker
            Self::MarketData => Capabilities::from_bits_truncate(
                Capabilities::FETCH_MARKETS.bits() | Capabilities::FETCH_TICKER.bits(),
            ),
            // Trading requires at least create_order and cancel_order
            Self::Trading => Capabilities::from_bits_truncate(
                Capabilities::CREATE_ORDER.bits() | Capabilities::CANCEL_ORDER.bits(),
            ),
            // Account requires at least fetch_balance
            Self::Account => Capabilities::FETCH_BALANCE,
            // Margin requires at least fetch_positions
            Self::Margin => Capabilities::FETCH_POSITIONS,
            // Funding requires at least fetch_deposit_address
            Self::Funding => Capabilities::FETCH_DEPOSIT_ADDRESS,
            // WebSocket requires the base websocket capability
            Self::WebSocket => Capabilities::WEBSOCKET,
        }
    }
}

impl fmt::Display for TraitCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

impl Capability {
    /// Get the trait category this capability belongs to.
    ///
    /// # Example
    ///
    /// ```rust
    /// use ccxt_core::capability::{Capability, TraitCategory};
    ///
    /// assert_eq!(Capability::FetchTicker.trait_category(), TraitCategory::MarketData);
    /// assert_eq!(Capability::CreateOrder.trait_category(), TraitCategory::Trading);
    /// assert_eq!(Capability::FetchBalance.trait_category(), TraitCategory::Account);
    /// ```
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

impl ExchangeCapabilities {
    // ========================================================================
    // Trait Implementation Checks
    // ========================================================================

    /// Check if the exchange supports the MarketData trait.
    ///
    /// Returns `true` if the exchange has the minimum capabilities required
    /// to implement the `MarketData` trait (fetch_markets and fetch_ticker).
    ///
    /// # Example
    ///
    /// ```rust
    /// use ccxt_core::capability::ExchangeCapabilities;
    ///
    /// let caps = ExchangeCapabilities::public_only();
    /// assert!(caps.supports_market_data());
    ///
    /// let caps = ExchangeCapabilities::none();
    /// assert!(!caps.supports_market_data());
    /// ```
    #[inline]
    pub const fn supports_market_data(&self) -> bool {
        self.inner
            .contains(TraitCategory::MarketData.minimum_capabilities())
    }

    /// Check if the exchange supports the Trading trait.
    ///
    /// Returns `true` if the exchange has the minimum capabilities required
    /// to implement the `Trading` trait (create_order and cancel_order).
    ///
    /// # Example
    ///
    /// ```rust
    /// use ccxt_core::capability::ExchangeCapabilities;
    ///
    /// let caps = ExchangeCapabilities::all();
    /// assert!(caps.supports_trading());
    ///
    /// let caps = ExchangeCapabilities::public_only();
    /// assert!(!caps.supports_trading());
    /// ```
    #[inline]
    pub const fn supports_trading(&self) -> bool {
        self.inner
            .contains(TraitCategory::Trading.minimum_capabilities())
    }

    /// Check if the exchange supports the Account trait.
    ///
    /// Returns `true` if the exchange has the minimum capabilities required
    /// to implement the `Account` trait (fetch_balance).
    ///
    /// # Example
    ///
    /// ```rust
    /// use ccxt_core::capability::ExchangeCapabilities;
    ///
    /// let caps = ExchangeCapabilities::all();
    /// assert!(caps.supports_account());
    ///
    /// let caps = ExchangeCapabilities::public_only();
    /// assert!(!caps.supports_account());
    /// ```
    #[inline]
    pub const fn supports_account(&self) -> bool {
        self.inner
            .contains(TraitCategory::Account.minimum_capabilities())
    }

    /// Check if the exchange supports the Margin trait.
    ///
    /// Returns `true` if the exchange has the minimum capabilities required
    /// to implement the `Margin` trait (fetch_positions).
    ///
    /// # Example
    ///
    /// ```rust
    /// use ccxt_core::capability::ExchangeCapabilities;
    ///
    /// let caps = ExchangeCapabilities::futures_exchange();
    /// assert!(caps.supports_margin());
    ///
    /// let caps = ExchangeCapabilities::spot_exchange();
    /// assert!(!caps.supports_margin());
    /// ```
    #[inline]
    pub const fn supports_margin(&self) -> bool {
        self.inner
            .contains(TraitCategory::Margin.minimum_capabilities())
    }

    /// Check if the exchange supports the Funding trait.
    ///
    /// Returns `true` if the exchange has the minimum capabilities required
    /// to implement the `Funding` trait (fetch_deposit_address).
    ///
    /// # Example
    ///
    /// ```rust
    /// use ccxt_core::capability::ExchangeCapabilities;
    ///
    /// let caps = ExchangeCapabilities::all();
    /// assert!(caps.supports_funding());
    ///
    /// let caps = ExchangeCapabilities::public_only();
    /// assert!(!caps.supports_funding());
    /// ```
    #[inline]
    pub const fn supports_funding(&self) -> bool {
        self.inner
            .contains(TraitCategory::Funding.minimum_capabilities())
    }

    /// Check if the exchange supports WebSocket.
    ///
    /// Returns `true` if the exchange has the base WebSocket capability.
    ///
    /// # Example
    ///
    /// ```rust
    /// use ccxt_core::capability::ExchangeCapabilities;
    ///
    /// let caps = ExchangeCapabilities::all();
    /// assert!(caps.supports_websocket());
    ///
    /// let caps = ExchangeCapabilities::public_only();
    /// assert!(!caps.supports_websocket());
    /// ```
    #[inline]
    pub const fn supports_websocket(&self) -> bool {
        self.inner
            .contains(TraitCategory::WebSocket.minimum_capabilities())
    }

    /// Check if the exchange supports all traits (FullExchange).
    ///
    /// Returns `true` if the exchange has the minimum capabilities required
    /// to implement all traits: MarketData, Trading, Account, Margin, and Funding.
    ///
    /// # Example
    ///
    /// ```rust
    /// use ccxt_core::capability::ExchangeCapabilities;
    ///
    /// let caps = ExchangeCapabilities::all();
    /// assert!(caps.supports_full_exchange());
    ///
    /// let caps = ExchangeCapabilities::spot_exchange();
    /// assert!(!caps.supports_full_exchange());
    /// ```
    #[inline]
    pub const fn supports_full_exchange(&self) -> bool {
        self.supports_market_data()
            && self.supports_trading()
            && self.supports_account()
            && self.supports_margin()
            && self.supports_funding()
    }

    /// Check if the exchange supports a specific trait category.
    ///
    /// # Arguments
    ///
    /// * `category` - The trait category to check
    ///
    /// # Example
    ///
    /// ```rust
    /// use ccxt_core::capability::{ExchangeCapabilities, TraitCategory};
    ///
    /// let caps = ExchangeCapabilities::all();
    /// assert!(caps.supports_trait(TraitCategory::MarketData));
    /// assert!(caps.supports_trait(TraitCategory::Trading));
    /// ```
    pub const fn supports_trait(&self, category: TraitCategory) -> bool {
        match category {
            TraitCategory::PublicExchange => true, // Always supported
            TraitCategory::MarketData => self.supports_market_data(),
            TraitCategory::Trading => self.supports_trading(),
            TraitCategory::Account => self.supports_account(),
            TraitCategory::Margin => self.supports_margin(),
            TraitCategory::Funding => self.supports_funding(),
            TraitCategory::WebSocket => self.supports_websocket(),
        }
    }

    /// Get a list of supported trait categories.
    ///
    /// Returns a vector of trait categories that this exchange supports
    /// based on its capabilities.
    ///
    /// # Example
    ///
    /// ```rust
    /// use ccxt_core::capability::{ExchangeCapabilities, TraitCategory};
    ///
    /// let caps = ExchangeCapabilities::public_only();
    /// let traits = caps.supported_traits();
    /// assert!(traits.contains(&TraitCategory::PublicExchange));
    /// assert!(traits.contains(&TraitCategory::MarketData));
    /// assert!(!traits.contains(&TraitCategory::Trading));
    /// ```
    pub fn supported_traits(&self) -> Vec<TraitCategory> {
        TraitCategory::all()
            .iter()
            .filter(|cat| self.supports_trait(**cat))
            .copied()
            .collect()
    }

    /// Get capabilities for a specific trait category.
    ///
    /// Returns the capabilities that are enabled for the specified trait category.
    ///
    /// # Arguments
    ///
    /// * `category` - The trait category to get capabilities for
    ///
    /// # Example
    ///
    /// ```rust
    /// use ccxt_core::capability::{ExchangeCapabilities, TraitCategory, Capabilities};
    ///
    /// let caps = ExchangeCapabilities::all();
    /// let market_caps = caps.capabilities_for_trait(TraitCategory::MarketData);
    /// assert!(market_caps.contains(Capabilities::FETCH_TICKER));
    /// ```
    pub const fn capabilities_for_trait(&self, category: TraitCategory) -> Capabilities {
        Capabilities::from_bits_truncate(self.inner.bits() & category.capabilities().bits())
    }

    /// Get the trait category for a specific capability.
    ///
    /// # Arguments
    ///
    /// * `capability` - The capability to get the trait category for
    ///
    /// # Example
    ///
    /// ```rust
    /// use ccxt_core::capability::{ExchangeCapabilities, Capability, TraitCategory};
    ///
    /// let category = ExchangeCapabilities::trait_for_capability(Capability::FetchTicker);
    /// assert_eq!(category, TraitCategory::MarketData);
    /// ```
    pub const fn trait_for_capability(capability: Capability) -> TraitCategory {
        capability.trait_category()
    }
}

// ============================================================================
// Unit Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

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
        // Market data should include all fetch* for public data
        assert!(Capabilities::MARKET_DATA.contains(Capabilities::FETCH_TICKER));
        assert!(Capabilities::MARKET_DATA.contains(Capabilities::FETCH_ORDER_BOOK));
        assert!(!Capabilities::MARKET_DATA.contains(Capabilities::CREATE_ORDER));

        // Trading should include order operations
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
        assert!(!spot.fetch_positions()); // No margin trading

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
        // Verify bit positions match enum values
        assert_eq!(Capability::FetchMarkets.bit_position(), 1 << 0);
        assert_eq!(Capability::FetchTicker.bit_position(), 1 << 2);
        assert_eq!(Capability::CreateOrder.bit_position(), 1 << 9);
        assert_eq!(Capability::Websocket.bit_position(), 1 << 37);
    }

    #[test]
    fn test_memory_efficiency() {
        // ExchangeCapabilities should be 8 bytes (u64)
        assert_eq!(std::mem::size_of::<ExchangeCapabilities>(), 8);
        assert_eq!(std::mem::size_of::<Capabilities>(), 8);
    }

    // ========================================================================
    // Trait-Capability Mapping Tests
    // ========================================================================

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
    fn test_trait_category_names() {
        assert_eq!(TraitCategory::PublicExchange.name(), "PublicExchange");
        assert_eq!(TraitCategory::MarketData.name(), "MarketData");
        assert_eq!(TraitCategory::Trading.name(), "Trading");
        assert_eq!(TraitCategory::Account.name(), "Account");
        assert_eq!(TraitCategory::Margin.name(), "Margin");
        assert_eq!(TraitCategory::Funding.name(), "Funding");
        assert_eq!(TraitCategory::WebSocket.name(), "WebSocket");
    }

    #[test]
    fn test_trait_category_capabilities() {
        assert_eq!(
            TraitCategory::PublicExchange.capabilities(),
            Capabilities::empty()
        );
        assert_eq!(
            TraitCategory::MarketData.capabilities(),
            Capabilities::MARKET_DATA
        );
        assert_eq!(TraitCategory::Trading.capabilities(), Capabilities::TRADING);
        assert_eq!(TraitCategory::Account.capabilities(), Capabilities::ACCOUNT);
        assert_eq!(TraitCategory::Margin.capabilities(), Capabilities::MARGIN);
        assert_eq!(TraitCategory::Funding.capabilities(), Capabilities::FUNDING);
        assert_eq!(
            TraitCategory::WebSocket.capabilities(),
            Capabilities::WEBSOCKET_ALL
        );
    }

    #[test]
    fn test_capability_trait_category() {
        // Market Data
        assert_eq!(
            Capability::FetchTicker.trait_category(),
            TraitCategory::MarketData
        );
        assert_eq!(
            Capability::FetchMarkets.trait_category(),
            TraitCategory::MarketData
        );
        assert_eq!(
            Capability::FetchOhlcv.trait_category(),
            TraitCategory::MarketData
        );

        // Trading
        assert_eq!(
            Capability::CreateOrder.trait_category(),
            TraitCategory::Trading
        );
        assert_eq!(
            Capability::CancelOrder.trait_category(),
            TraitCategory::Trading
        );
        assert_eq!(
            Capability::FetchOpenOrders.trait_category(),
            TraitCategory::Trading
        );

        // Account
        assert_eq!(
            Capability::FetchBalance.trait_category(),
            TraitCategory::Account
        );
        assert_eq!(
            Capability::FetchMyTrades.trait_category(),
            TraitCategory::Account
        );

        // Margin
        assert_eq!(
            Capability::FetchPositions.trait_category(),
            TraitCategory::Margin
        );
        assert_eq!(
            Capability::SetLeverage.trait_category(),
            TraitCategory::Margin
        );
        assert_eq!(
            Capability::FetchFundingRate.trait_category(),
            TraitCategory::Margin
        );

        // Funding
        assert_eq!(
            Capability::FetchDepositAddress.trait_category(),
            TraitCategory::Funding
        );
        assert_eq!(
            Capability::Withdraw.trait_category(),
            TraitCategory::Funding
        );
        assert_eq!(
            Capability::Transfer.trait_category(),
            TraitCategory::Funding
        );

        // WebSocket
        assert_eq!(
            Capability::Websocket.trait_category(),
            TraitCategory::WebSocket
        );
        assert_eq!(
            Capability::WatchTicker.trait_category(),
            TraitCategory::WebSocket
        );
    }

    #[test]
    fn test_supports_market_data() {
        let caps = ExchangeCapabilities::public_only();
        assert!(caps.supports_market_data());

        let caps = ExchangeCapabilities::none();
        assert!(!caps.supports_market_data());

        // Only fetch_markets is not enough
        let caps = ExchangeCapabilities::builder()
            .capability(Capability::FetchMarkets)
            .build();
        assert!(!caps.supports_market_data());

        // Both fetch_markets and fetch_ticker are required
        let caps = ExchangeCapabilities::builder()
            .capability(Capability::FetchMarkets)
            .capability(Capability::FetchTicker)
            .build();
        assert!(caps.supports_market_data());
    }

    #[test]
    fn test_supports_trading() {
        let caps = ExchangeCapabilities::all();
        assert!(caps.supports_trading());

        let caps = ExchangeCapabilities::public_only();
        assert!(!caps.supports_trading());

        // Both create_order and cancel_order are required
        let caps = ExchangeCapabilities::builder()
            .capability(Capability::CreateOrder)
            .capability(Capability::CancelOrder)
            .build();
        assert!(caps.supports_trading());
    }

    #[test]
    fn test_supports_account() {
        let caps = ExchangeCapabilities::all();
        assert!(caps.supports_account());

        let caps = ExchangeCapabilities::public_only();
        assert!(!caps.supports_account());

        let caps = ExchangeCapabilities::builder()
            .capability(Capability::FetchBalance)
            .build();
        assert!(caps.supports_account());
    }

    #[test]
    fn test_supports_margin() {
        let caps = ExchangeCapabilities::futures_exchange();
        assert!(caps.supports_margin());

        let caps = ExchangeCapabilities::spot_exchange();
        assert!(!caps.supports_margin());

        let caps = ExchangeCapabilities::builder()
            .capability(Capability::FetchPositions)
            .build();
        assert!(caps.supports_margin());
    }

    #[test]
    fn test_supports_funding() {
        let caps = ExchangeCapabilities::all();
        assert!(caps.supports_funding());

        let caps = ExchangeCapabilities::public_only();
        assert!(!caps.supports_funding());

        let caps = ExchangeCapabilities::builder()
            .capability(Capability::FetchDepositAddress)
            .build();
        assert!(caps.supports_funding());
    }

    #[test]
    fn test_supports_websocket() {
        let caps = ExchangeCapabilities::all();
        assert!(caps.supports_websocket());

        let caps = ExchangeCapabilities::public_only();
        assert!(!caps.supports_websocket());

        let caps = ExchangeCapabilities::builder()
            .capability(Capability::Websocket)
            .build();
        assert!(caps.supports_websocket());
    }

    #[test]
    fn test_supports_full_exchange() {
        let caps = ExchangeCapabilities::all();
        assert!(caps.supports_full_exchange());

        let caps = ExchangeCapabilities::spot_exchange();
        assert!(!caps.supports_full_exchange()); // Missing margin and funding

        let caps = ExchangeCapabilities::futures_exchange();
        assert!(!caps.supports_full_exchange()); // Missing funding
    }

    #[test]
    fn test_supports_trait() {
        let caps = ExchangeCapabilities::all();
        assert!(caps.supports_trait(TraitCategory::PublicExchange));
        assert!(caps.supports_trait(TraitCategory::MarketData));
        assert!(caps.supports_trait(TraitCategory::Trading));
        assert!(caps.supports_trait(TraitCategory::Account));
        assert!(caps.supports_trait(TraitCategory::Margin));
        assert!(caps.supports_trait(TraitCategory::Funding));
        assert!(caps.supports_trait(TraitCategory::WebSocket));

        let caps = ExchangeCapabilities::public_only();
        assert!(caps.supports_trait(TraitCategory::PublicExchange));
        assert!(caps.supports_trait(TraitCategory::MarketData));
        assert!(!caps.supports_trait(TraitCategory::Trading));
        assert!(!caps.supports_trait(TraitCategory::Account));
        assert!(!caps.supports_trait(TraitCategory::Margin));
        assert!(!caps.supports_trait(TraitCategory::Funding));
        assert!(!caps.supports_trait(TraitCategory::WebSocket));
    }

    #[test]
    fn test_supported_traits() {
        let caps = ExchangeCapabilities::public_only();
        let traits = caps.supported_traits();
        assert!(traits.contains(&TraitCategory::PublicExchange));
        assert!(traits.contains(&TraitCategory::MarketData));
        assert!(!traits.contains(&TraitCategory::Trading));

        let caps = ExchangeCapabilities::all();
        let traits = caps.supported_traits();
        assert_eq!(traits.len(), 7); // All traits supported
    }

    #[test]
    fn test_capabilities_for_trait() {
        let caps = ExchangeCapabilities::all();

        let market_caps = caps.capabilities_for_trait(TraitCategory::MarketData);
        assert!(market_caps.contains(Capabilities::FETCH_TICKER));
        assert!(market_caps.contains(Capabilities::FETCH_ORDER_BOOK));
        assert!(!market_caps.contains(Capabilities::CREATE_ORDER));

        let trading_caps = caps.capabilities_for_trait(TraitCategory::Trading);
        assert!(trading_caps.contains(Capabilities::CREATE_ORDER));
        assert!(trading_caps.contains(Capabilities::CANCEL_ORDER));
        assert!(!trading_caps.contains(Capabilities::FETCH_TICKER));
    }

    #[test]
    fn test_trait_for_capability() {
        assert_eq!(
            ExchangeCapabilities::trait_for_capability(Capability::FetchTicker),
            TraitCategory::MarketData
        );
        assert_eq!(
            ExchangeCapabilities::trait_for_capability(Capability::CreateOrder),
            TraitCategory::Trading
        );
        assert_eq!(
            ExchangeCapabilities::trait_for_capability(Capability::FetchBalance),
            TraitCategory::Account
        );
    }

    #[test]
    fn test_trait_category_display() {
        assert_eq!(format!("{}", TraitCategory::MarketData), "MarketData");
        assert_eq!(format!("{}", TraitCategory::Trading), "Trading");
    }
}
