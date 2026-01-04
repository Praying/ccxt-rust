//! Capability-related macros

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
