//! Exchange capability constants and high-level API

use std::fmt;

use super::{Capabilities, Capability};

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
    pub const fn all() -> Self {
        Self {
            inner: Capabilities::ALL,
        }
    }

    /// Create capabilities for public API only (no authentication required)
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

    #[inline]
    pub const fn fetch_markets(&self) -> bool {
        self.inner.contains(Capabilities::FETCH_MARKETS)
    }

    #[inline]
    pub const fn fetch_currencies(&self) -> bool {
        self.inner.contains(Capabilities::FETCH_CURRENCIES)
    }

    #[inline]
    pub const fn fetch_ticker(&self) -> bool {
        self.inner.contains(Capabilities::FETCH_TICKER)
    }

    #[inline]
    pub const fn fetch_tickers(&self) -> bool {
        self.inner.contains(Capabilities::FETCH_TICKERS)
    }

    #[inline]
    pub const fn fetch_order_book(&self) -> bool {
        self.inner.contains(Capabilities::FETCH_ORDER_BOOK)
    }

    #[inline]
    pub const fn fetch_trades(&self) -> bool {
        self.inner.contains(Capabilities::FETCH_TRADES)
    }

    #[inline]
    pub const fn fetch_ohlcv(&self) -> bool {
        self.inner.contains(Capabilities::FETCH_OHLCV)
    }

    #[inline]
    pub const fn fetch_status(&self) -> bool {
        self.inner.contains(Capabilities::FETCH_STATUS)
    }

    #[inline]
    pub const fn fetch_time(&self) -> bool {
        self.inner.contains(Capabilities::FETCH_TIME)
    }

    // ==================== Trading Accessors ====================

    #[inline]
    pub const fn create_order(&self) -> bool {
        self.inner.contains(Capabilities::CREATE_ORDER)
    }

    #[inline]
    pub const fn create_market_order(&self) -> bool {
        self.inner.contains(Capabilities::CREATE_MARKET_ORDER)
    }

    #[inline]
    pub const fn create_limit_order(&self) -> bool {
        self.inner.contains(Capabilities::CREATE_LIMIT_ORDER)
    }

    #[inline]
    pub const fn cancel_order(&self) -> bool {
        self.inner.contains(Capabilities::CANCEL_ORDER)
    }

    #[inline]
    pub const fn cancel_all_orders(&self) -> bool {
        self.inner.contains(Capabilities::CANCEL_ALL_ORDERS)
    }

    #[inline]
    pub const fn edit_order(&self) -> bool {
        self.inner.contains(Capabilities::EDIT_ORDER)
    }

    #[inline]
    pub const fn fetch_order(&self) -> bool {
        self.inner.contains(Capabilities::FETCH_ORDER)
    }

    #[inline]
    pub const fn fetch_orders(&self) -> bool {
        self.inner.contains(Capabilities::FETCH_ORDERS)
    }

    #[inline]
    pub const fn fetch_open_orders(&self) -> bool {
        self.inner.contains(Capabilities::FETCH_OPEN_ORDERS)
    }

    #[inline]
    pub const fn fetch_closed_orders(&self) -> bool {
        self.inner.contains(Capabilities::FETCH_CLOSED_ORDERS)
    }

    #[inline]
    pub const fn fetch_canceled_orders(&self) -> bool {
        self.inner.contains(Capabilities::FETCH_CANCELED_ORDERS)
    }

    // ==================== Account Accessors ====================

    #[inline]
    pub const fn fetch_balance(&self) -> bool {
        self.inner.contains(Capabilities::FETCH_BALANCE)
    }

    #[inline]
    pub const fn fetch_my_trades(&self) -> bool {
        self.inner.contains(Capabilities::FETCH_MY_TRADES)
    }

    #[inline]
    pub const fn fetch_deposits(&self) -> bool {
        self.inner.contains(Capabilities::FETCH_DEPOSITS)
    }

    #[inline]
    pub const fn fetch_withdrawals(&self) -> bool {
        self.inner.contains(Capabilities::FETCH_WITHDRAWALS)
    }

    #[inline]
    pub const fn fetch_transactions(&self) -> bool {
        self.inner.contains(Capabilities::FETCH_TRANSACTIONS)
    }

    #[inline]
    pub const fn fetch_ledger(&self) -> bool {
        self.inner.contains(Capabilities::FETCH_LEDGER)
    }

    // ==================== Funding Accessors ====================

    #[inline]
    pub const fn fetch_deposit_address(&self) -> bool {
        self.inner.contains(Capabilities::FETCH_DEPOSIT_ADDRESS)
    }

    #[inline]
    pub const fn create_deposit_address(&self) -> bool {
        self.inner.contains(Capabilities::CREATE_DEPOSIT_ADDRESS)
    }

    #[inline]
    pub const fn withdraw(&self) -> bool {
        self.inner.contains(Capabilities::WITHDRAW)
    }

    #[inline]
    pub const fn transfer(&self) -> bool {
        self.inner.contains(Capabilities::TRANSFER)
    }

    // ==================== Margin Trading Accessors ====================

    #[inline]
    pub const fn fetch_borrow_rate(&self) -> bool {
        self.inner.contains(Capabilities::FETCH_BORROW_RATE)
    }

    #[inline]
    pub const fn fetch_borrow_rates(&self) -> bool {
        self.inner.contains(Capabilities::FETCH_BORROW_RATES)
    }

    #[inline]
    pub const fn fetch_funding_rate(&self) -> bool {
        self.inner.contains(Capabilities::FETCH_FUNDING_RATE)
    }

    #[inline]
    pub const fn fetch_funding_rates(&self) -> bool {
        self.inner.contains(Capabilities::FETCH_FUNDING_RATES)
    }

    #[inline]
    pub const fn fetch_positions(&self) -> bool {
        self.inner.contains(Capabilities::FETCH_POSITIONS)
    }

    #[inline]
    pub const fn set_leverage(&self) -> bool {
        self.inner.contains(Capabilities::SET_LEVERAGE)
    }

    #[inline]
    pub const fn set_margin_mode(&self) -> bool {
        self.inner.contains(Capabilities::SET_MARGIN_MODE)
    }

    // ==================== WebSocket Accessors ====================

    #[inline]
    pub const fn websocket(&self) -> bool {
        self.inner.contains(Capabilities::WEBSOCKET)
    }

    #[inline]
    pub const fn watch_ticker(&self) -> bool {
        self.inner.contains(Capabilities::WATCH_TICKER)
    }

    #[inline]
    pub const fn watch_tickers(&self) -> bool {
        self.inner.contains(Capabilities::WATCH_TICKERS)
    }

    #[inline]
    pub const fn watch_order_book(&self) -> bool {
        self.inner.contains(Capabilities::WATCH_ORDER_BOOK)
    }

    #[inline]
    pub const fn watch_trades(&self) -> bool {
        self.inner.contains(Capabilities::WATCH_TRADES)
    }

    #[inline]
    pub const fn watch_ohlcv(&self) -> bool {
        self.inner.contains(Capabilities::WATCH_OHLCV)
    }

    #[inline]
    pub const fn watch_balance(&self) -> bool {
        self.inner.contains(Capabilities::WATCH_BALANCE)
    }

    #[inline]
    pub const fn watch_orders(&self) -> bool {
        self.inner.contains(Capabilities::WATCH_ORDERS)
    }

    #[inline]
    pub const fn watch_my_trades(&self) -> bool {
        self.inner.contains(Capabilities::WATCH_MY_TRADES)
    }

    // ==================== Builder Method ====================

    /// Create a builder for ExchangeCapabilities
    pub fn builder() -> ExchangeCapabilitiesBuilder {
        ExchangeCapabilitiesBuilder::new()
    }

    // ==================== Presets ====================

    /// Create capabilities for a typical spot exchange
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
    pub const fn full_featured() -> Self {
        Self {
            inner: Capabilities::ALL,
        }
    }

    // ==================== Trait Implementation Checks ====================

    #[inline]
    pub const fn supports_market_data(&self) -> bool {
        self.inner
            .contains(TraitCategory::MarketData.minimum_capabilities())
    }

    #[inline]
    pub const fn supports_trading(&self) -> bool {
        self.inner
            .contains(TraitCategory::Trading.minimum_capabilities())
    }

    #[inline]
    pub const fn supports_account(&self) -> bool {
        self.inner
            .contains(TraitCategory::Account.minimum_capabilities())
    }

    #[inline]
    pub const fn supports_margin(&self) -> bool {
        self.inner
            .contains(TraitCategory::Margin.minimum_capabilities())
    }

    #[inline]
    pub const fn supports_funding(&self) -> bool {
        self.inner
            .contains(TraitCategory::Funding.minimum_capabilities())
    }

    #[inline]
    pub const fn supports_websocket(&self) -> bool {
        self.inner
            .contains(TraitCategory::WebSocket.minimum_capabilities())
    }

    #[inline]
    pub const fn supports_full_exchange(&self) -> bool {
        self.supports_market_data()
            && self.supports_trading()
            && self.supports_account()
            && self.supports_margin()
            && self.supports_funding()
    }

    pub const fn supports_trait(&self, category: TraitCategory) -> bool {
        match category {
            TraitCategory::PublicExchange => true,
            TraitCategory::MarketData => self.supports_market_data(),
            TraitCategory::Trading => self.supports_trading(),
            TraitCategory::Account => self.supports_account(),
            TraitCategory::Margin => self.supports_margin(),
            TraitCategory::Funding => self.supports_funding(),
            TraitCategory::WebSocket => self.supports_websocket(),
        }
    }

    pub fn supported_traits(&self) -> Vec<TraitCategory> {
        TraitCategory::all()
            .iter()
            .filter(|cat| self.supports_trait(**cat))
            .copied()
            .collect()
    }

    pub const fn capabilities_for_trait(&self, category: TraitCategory) -> Capabilities {
        Capabilities::from_bits_truncate(self.inner.bits() & category.capabilities().bits())
    }

    pub const fn trait_for_capability(capability: Capability) -> TraitCategory {
        capability.trait_category()
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

/// Builder for ExchangeCapabilities
#[derive(Debug, Clone, Default)]
pub struct ExchangeCapabilitiesBuilder {
    inner: Capabilities,
}

impl ExchangeCapabilitiesBuilder {
    pub fn new() -> Self {
        Self {
            inner: Capabilities::empty(),
        }
    }

    pub fn market_data(mut self) -> Self {
        self.inner |= Capabilities::MARKET_DATA;
        self
    }

    pub fn trading(mut self) -> Self {
        self.inner |= Capabilities::TRADING;
        self
    }

    pub fn account(mut self) -> Self {
        self.inner |= Capabilities::ACCOUNT;
        self
    }

    pub fn funding(mut self) -> Self {
        self.inner |= Capabilities::FUNDING;
        self
    }

    pub fn margin(mut self) -> Self {
        self.inner |= Capabilities::MARGIN;
        self
    }

    pub fn websocket_all(mut self) -> Self {
        self.inner |= Capabilities::WEBSOCKET_ALL;
        self
    }

    pub fn websocket(mut self) -> Self {
        self.inner |= Capabilities::WEBSOCKET;
        self
    }

    pub fn rest_all(mut self) -> Self {
        self.inner |= Capabilities::REST_ALL;
        self
    }

    pub fn all(mut self) -> Self {
        self.inner = Capabilities::ALL;
        self
    }

    pub fn capability(mut self, cap: Capability) -> Self {
        self.inner |= Capabilities::from(cap);
        self
    }

    pub fn capabilities<I: IntoIterator<Item = Capability>>(mut self, caps: I) -> Self {
        for cap in caps {
            self.inner |= Capabilities::from(cap);
        }
        self
    }

    pub fn raw(mut self, caps: Capabilities) -> Self {
        self.inner |= caps;
        self
    }

    pub fn without_capability(mut self, cap: Capability) -> Self {
        self.inner.remove(Capabilities::from(cap));
        self
    }

    pub fn without(mut self, caps: Capabilities) -> Self {
        self.inner.remove(caps);
        self
    }

    pub fn build(self) -> ExchangeCapabilities {
        ExchangeCapabilities { inner: self.inner }
    }
}

/// Trait category for capability-to-trait mapping
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TraitCategory {
    PublicExchange,
    MarketData,
    Trading,
    Account,
    Margin,
    Funding,
    WebSocket,
}

impl TraitCategory {
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

    pub const fn capabilities(&self) -> Capabilities {
        match self {
            Self::PublicExchange => Capabilities::empty(),
            Self::MarketData => Capabilities::MARKET_DATA,
            Self::Trading => Capabilities::TRADING,
            Self::Account => Capabilities::ACCOUNT,
            Self::Margin => Capabilities::MARGIN,
            Self::Funding => Capabilities::FUNDING,
            Self::WebSocket => Capabilities::WEBSOCKET_ALL,
        }
    }

    pub const fn minimum_capabilities(&self) -> Capabilities {
        match self {
            Self::PublicExchange => Capabilities::empty(),
            Self::MarketData => Capabilities::from_bits_truncate(
                Capabilities::FETCH_MARKETS.bits() | Capabilities::FETCH_TICKER.bits(),
            ),
            Self::Trading => Capabilities::from_bits_truncate(
                Capabilities::CREATE_ORDER.bits() | Capabilities::CANCEL_ORDER.bits(),
            ),
            Self::Account => Capabilities::FETCH_BALANCE,
            Self::Margin => Capabilities::FETCH_POSITIONS,
            Self::Funding => Capabilities::FETCH_DEPOSIT_ADDRESS,
            Self::WebSocket => Capabilities::WEBSOCKET,
        }
    }
}

impl fmt::Display for TraitCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}
