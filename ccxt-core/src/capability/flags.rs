//! Capability bitflags definitions

use bitflags::bitflags;
use std::fmt;

use super::Capability;

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
            .map(Capability::as_ccxt_name)
            .collect()
    }

    /// Count the number of enabled capabilities
    #[inline]
    pub fn count(&self) -> u32 {
        self.bits().count_ones()
    }

    /// Create from an iterator of capabilities
    // Lint: should_implement_trait
    // Reason: This method has different semantics than FromIterator - it's a convenience constructor
    #[allow(clippy::should_implement_trait)]
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
