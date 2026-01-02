//! OKX-specific endpoint router trait.
//!
//! This module provides the `OkxEndpointRouter` trait for routing API requests
//! to the correct OKX endpoints based on channel type.
//!
//! # OKX API Structure
//!
//! OKX uses a unified V5 API with the following endpoint structure:
//! - **REST**: `www.okx.com` - Single unified REST endpoint for all market types
//! - **WebSocket Public**: `ws.okx.com:8443/ws/v5/public` - Public market data
//! - **WebSocket Private**: `ws.okx.com:8443/ws/v5/private` - Account data
//! - **WebSocket Business**: `ws.okx.com:8443/ws/v5/business` - Trade execution
//!
//! # Demo Trading Mode
//!
//! OKX uses a unique approach for demo trading:
//! - REST API uses the **same production domain** (`www.okx.com`)
//! - Demo mode is indicated by the `x-simulated-trading: 1` header
//! - WebSocket URLs switch to demo domain (`wspap.okx.com:8443`)
//!
//! # Channel Types
//!
//! OKX WebSocket has three channel types:
//! - `Public` - Market data (tickers, orderbooks, trades)
//! - `Private` - Account data (positions, orders, balances)
//! - `Business` - Trade execution and advanced features
//!
//! # Example
//!
//! ```rust,no_run
//! use ccxt_exchanges::okx::{Okx, OkxEndpointRouter, OkxChannelType};
//! use ccxt_core::ExchangeConfig;
//!
//! let okx = Okx::new(ExchangeConfig::default()).unwrap();
//!
//! // Get REST endpoint (unified for all market types)
//! let rest_url = okx.rest_endpoint();
//! assert!(rest_url.contains("okx.com"));
//!
//! // Get WebSocket endpoint for public channel
//! let ws_public = okx.ws_endpoint(OkxChannelType::Public);
//! assert!(ws_public.contains("/ws/v5/public"));
//!
//! // Get WebSocket endpoint for private channel
//! let ws_private = okx.ws_endpoint(OkxChannelType::Private);
//! assert!(ws_private.contains("/ws/v5/private"));
//!
//! // Check if demo trading mode is enabled
//! let is_demo = okx.is_demo_trading();
//! ```

/// OKX WebSocket channel type.
///
/// OKX uses different WebSocket channels for different types of data:
/// - `Public` - Market data streams (no authentication required)
/// - `Private` - Account data streams (authentication required)
/// - `Business` - Trade execution streams (authentication required)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OkxChannelType {
    /// Public channel for market data (tickers, orderbooks, trades).
    ///
    /// No authentication required. Used for:
    /// - Real-time ticker updates
    /// - Order book snapshots and updates
    /// - Public trade streams
    /// - Candlestick/OHLCV data
    Public,

    /// Private channel for account data.
    ///
    /// Authentication required. Used for:
    /// - Account balance updates
    /// - Position updates
    /// - Order status updates
    Private,

    /// Business channel for trade execution.
    ///
    /// Authentication required. Used for:
    /// - Advanced order types
    /// - Algo orders
    /// - Grid trading
    /// - Copy trading
    Business,
}

impl std::fmt::Display for OkxChannelType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OkxChannelType::Public => write!(f, "public"),
            OkxChannelType::Private => write!(f, "private"),
            OkxChannelType::Business => write!(f, "business"),
        }
    }
}

/// OKX-specific endpoint router trait.
///
/// This trait defines methods for obtaining the correct API endpoints for OKX.
/// OKX uses a unified REST endpoint for all market types, with WebSocket
/// channels differentiated by channel type (public, private, business).
///
/// # Implementation Notes
///
/// - REST API uses a single unified domain for all market types
/// - Demo trading mode uses the same REST domain but adds a special header
/// - WebSocket endpoints are differentiated by channel type
/// - Demo mode WebSocket URLs use a different domain (`wspap.okx.com`)
///
/// # Demo Trading
///
/// OKX's demo trading mode is unique:
/// - REST requests use the production domain with `x-simulated-trading: 1` header
/// - WebSocket connections use demo-specific URLs with `brokerId=9999` parameter
pub trait OkxEndpointRouter {
    /// Returns the REST API endpoint.
    ///
    /// OKX uses a unified REST domain for all market types. The instrument
    /// type is specified as a query parameter in API requests, not in the URL.
    ///
    /// Note: For demo trading, the same URL is used but with the
    /// `x-simulated-trading: 1` header added to requests.
    ///
    /// # Returns
    ///
    /// The REST API base URL string.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use ccxt_exchanges::okx::{Okx, OkxEndpointRouter};
    /// use ccxt_core::ExchangeConfig;
    ///
    /// let okx = Okx::new(ExchangeConfig::default()).unwrap();
    /// let url = okx.rest_endpoint();
    /// assert_eq!(url, "https://www.okx.com");
    /// ```
    fn rest_endpoint(&self) -> &'static str;

    /// Returns the WebSocket endpoint for a specific channel type.
    ///
    /// OKX uses different WebSocket URLs for different channel types:
    /// - `Public`: `/ws/v5/public` - Market data
    /// - `Private`: `/ws/v5/private` - Account data
    /// - `Business`: `/ws/v5/business` - Trade execution
    ///
    /// In demo trading mode, the URLs switch to the demo domain
    /// (`wspap.okx.com`) with `brokerId=9999` parameter.
    ///
    /// # Arguments
    ///
    /// * `channel_type` - The WebSocket channel type (Public, Private, Business)
    ///
    /// # Returns
    ///
    /// The complete WebSocket URL for the specified channel type.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use ccxt_exchanges::okx::{Okx, OkxEndpointRouter, OkxChannelType};
    /// use ccxt_core::ExchangeConfig;
    ///
    /// let okx = Okx::new(ExchangeConfig::default()).unwrap();
    ///
    /// // Get WebSocket URL for public market data
    /// let ws_public = okx.ws_endpoint(OkxChannelType::Public);
    /// assert!(ws_public.contains("/ws/v5/public"));
    ///
    /// // Get WebSocket URL for private account data
    /// let ws_private = okx.ws_endpoint(OkxChannelType::Private);
    /// assert!(ws_private.contains("/ws/v5/private"));
    ///
    /// // Get WebSocket URL for business/trading
    /// let ws_business = okx.ws_endpoint(OkxChannelType::Business);
    /// assert!(ws_business.contains("/ws/v5/business"));
    /// ```
    fn ws_endpoint(&self, channel_type: OkxChannelType) -> &str;

    /// Returns whether demo trading mode is enabled.
    ///
    /// Demo trading mode is enabled when either:
    /// - `config.sandbox` is set to `true`
    /// - `options.testnet` is set to `true`
    ///
    /// When demo trading is enabled:
    /// - REST requests should include the `x-simulated-trading: 1` header
    /// - WebSocket URLs use the demo domain (`wspap.okx.com`)
    ///
    /// # Returns
    ///
    /// `true` if demo trading mode is enabled, `false` otherwise.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use ccxt_exchanges::okx::{Okx, OkxEndpointRouter};
    /// use ccxt_core::ExchangeConfig;
    ///
    /// // Production mode
    /// let okx = Okx::new(ExchangeConfig::default()).unwrap();
    /// assert!(!okx.is_demo_trading());
    ///
    /// // Demo mode
    /// let config = ExchangeConfig {
    ///     sandbox: true,
    ///     ..Default::default()
    /// };
    /// let okx_demo = Okx::new(config).unwrap();
    /// assert!(okx_demo.is_demo_trading());
    /// ```
    fn is_demo_trading(&self) -> bool;
}

use super::Okx;

impl OkxEndpointRouter for Okx {
    fn rest_endpoint(&self) -> &'static str {
        // OKX uses the same REST domain for both production and demo trading.
        // Demo mode is indicated by the `x-simulated-trading: 1` header,
        // not by a different URL.
        "https://www.okx.com"
    }

    fn ws_endpoint(&self, channel_type: OkxChannelType) -> &str {
        // OKX WebSocket URLs differ between production and demo mode
        if self.is_testnet_trading() {
            // Demo trading WebSocket URLs
            match channel_type {
                OkxChannelType::Public => "wss://wspap.okx.com:8443/ws/v5/public?brokerId=9999",
                OkxChannelType::Private => "wss://wspap.okx.com:8443/ws/v5/private?brokerId=9999",
                OkxChannelType::Business => "wss://wspap.okx.com:8443/ws/v5/business?brokerId=9999",
            }
        } else {
            // Production WebSocket URLs
            match channel_type {
                OkxChannelType::Public => "wss://ws.okx.com:8443/ws/v5/public",
                OkxChannelType::Private => "wss://ws.okx.com:8443/ws/v5/private",
                OkxChannelType::Business => "wss://ws.okx.com:8443/ws/v5/business",
            }
        }
    }

    fn is_demo_trading(&self) -> bool {
        // Delegate to the existing method that checks both config.sandbox and options.testnet
        self.is_testnet_trading()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::okx::OkxOptions;
    use ccxt_core::ExchangeConfig;

    fn create_test_okx() -> Okx {
        Okx::new(ExchangeConfig::default()).unwrap()
    }

    fn create_demo_okx() -> Okx {
        let config = ExchangeConfig {
            sandbox: true,
            ..Default::default()
        };
        Okx::new(config).unwrap()
    }

    // ==================== REST Endpoint Tests ====================

    #[test]
    fn test_rest_endpoint_production() {
        let okx = create_test_okx();
        let url = okx.rest_endpoint();
        assert_eq!(url, "https://www.okx.com");
    }

    #[test]
    fn test_rest_endpoint_demo() {
        let okx = create_demo_okx();
        let url = okx.rest_endpoint();
        // OKX uses the same REST domain for demo trading
        // Demo mode is indicated by header, not URL
        assert_eq!(url, "https://www.okx.com");
    }

    // ==================== WebSocket Public Endpoint Tests ====================

    #[test]
    fn test_ws_endpoint_public_production() {
        let okx = create_test_okx();
        let url = okx.ws_endpoint(OkxChannelType::Public);
        assert_eq!(url, "wss://ws.okx.com:8443/ws/v5/public");
        assert!(!url.contains("brokerId"));
    }

    #[test]
    fn test_ws_endpoint_public_demo() {
        let okx = create_demo_okx();
        let url = okx.ws_endpoint(OkxChannelType::Public);
        assert!(url.contains("wspap.okx.com"));
        assert!(url.contains("/ws/v5/public"));
        assert!(url.contains("brokerId=9999"));
    }

    // ==================== WebSocket Private Endpoint Tests ====================

    #[test]
    fn test_ws_endpoint_private_production() {
        let okx = create_test_okx();
        let url = okx.ws_endpoint(OkxChannelType::Private);
        assert_eq!(url, "wss://ws.okx.com:8443/ws/v5/private");
        assert!(!url.contains("brokerId"));
    }

    #[test]
    fn test_ws_endpoint_private_demo() {
        let okx = create_demo_okx();
        let url = okx.ws_endpoint(OkxChannelType::Private);
        assert!(url.contains("wspap.okx.com"));
        assert!(url.contains("/ws/v5/private"));
        assert!(url.contains("brokerId=9999"));
    }

    // ==================== WebSocket Business Endpoint Tests ====================

    #[test]
    fn test_ws_endpoint_business_production() {
        let okx = create_test_okx();
        let url = okx.ws_endpoint(OkxChannelType::Business);
        assert_eq!(url, "wss://ws.okx.com:8443/ws/v5/business");
        assert!(!url.contains("brokerId"));
    }

    #[test]
    fn test_ws_endpoint_business_demo() {
        let okx = create_demo_okx();
        let url = okx.ws_endpoint(OkxChannelType::Business);
        assert!(url.contains("wspap.okx.com"));
        assert!(url.contains("/ws/v5/business"));
        assert!(url.contains("brokerId=9999"));
    }

    // ==================== Demo Trading Mode Tests ====================

    #[test]
    fn test_is_demo_trading_false_by_default() {
        let okx = create_test_okx();
        assert!(!okx.is_demo_trading());
    }

    #[test]
    fn test_is_demo_trading_with_sandbox_config() {
        let okx = create_demo_okx();
        assert!(okx.is_demo_trading());
    }

    #[test]
    fn test_is_demo_trading_with_testnet_option() {
        let config = ExchangeConfig::default();
        let options = OkxOptions {
            testnet: true,
            ..Default::default()
        };
        let okx = Okx::new_with_options(config, options).unwrap();
        assert!(okx.is_demo_trading());
    }

    // ==================== Channel Type Display Tests ====================

    #[test]
    fn test_channel_type_display() {
        assert_eq!(format!("{}", OkxChannelType::Public), "public");
        assert_eq!(format!("{}", OkxChannelType::Private), "private");
        assert_eq!(format!("{}", OkxChannelType::Business), "business");
    }

    // ==================== Channel Type Equality Tests ====================

    #[test]
    fn test_channel_type_equality() {
        assert_eq!(OkxChannelType::Public, OkxChannelType::Public);
        assert_eq!(OkxChannelType::Private, OkxChannelType::Private);
        assert_eq!(OkxChannelType::Business, OkxChannelType::Business);
        assert_ne!(OkxChannelType::Public, OkxChannelType::Private);
        assert_ne!(OkxChannelType::Private, OkxChannelType::Business);
    }

    // ==================== All Channel Types Tests ====================

    #[test]
    fn test_all_channel_types_production() {
        let okx = create_test_okx();

        let channels = [
            (OkxChannelType::Public, "/ws/v5/public"),
            (OkxChannelType::Private, "/ws/v5/private"),
            (OkxChannelType::Business, "/ws/v5/business"),
        ];

        for (channel_type, expected_path) in channels {
            let url = okx.ws_endpoint(channel_type);
            assert!(
                url.contains(expected_path),
                "URL {} should contain {}",
                url,
                expected_path
            );
            assert!(
                url.contains("ws.okx.com"),
                "Production URL {} should contain ws.okx.com",
                url
            );
        }
    }

    #[test]
    fn test_all_channel_types_demo() {
        let okx = create_demo_okx();

        let channels = [
            (OkxChannelType::Public, "/ws/v5/public"),
            (OkxChannelType::Private, "/ws/v5/private"),
            (OkxChannelType::Business, "/ws/v5/business"),
        ];

        for (channel_type, expected_path) in channels {
            let url = okx.ws_endpoint(channel_type);
            assert!(
                url.contains(expected_path),
                "URL {} should contain {}",
                url,
                expected_path
            );
            assert!(
                url.contains("wspap.okx.com"),
                "Demo URL {} should contain wspap.okx.com",
                url
            );
            assert!(
                url.contains("brokerId=9999"),
                "Demo URL {} should contain brokerId=9999",
                url
            );
        }
    }

    // ==================== Consistency Tests ====================

    #[test]
    fn test_rest_endpoint_same_for_production_and_demo() {
        let okx_prod = create_test_okx();
        let okx_demo = create_demo_okx();

        // OKX uses the same REST domain for both modes
        assert_eq!(okx_prod.rest_endpoint(), okx_demo.rest_endpoint());
    }

    #[test]
    fn test_ws_endpoints_differ_for_production_and_demo() {
        let okx_prod = create_test_okx();
        let okx_demo = create_demo_okx();

        // WebSocket URLs should be different
        assert_ne!(
            okx_prod.ws_endpoint(OkxChannelType::Public),
            okx_demo.ws_endpoint(OkxChannelType::Public)
        );
        assert_ne!(
            okx_prod.ws_endpoint(OkxChannelType::Private),
            okx_demo.ws_endpoint(OkxChannelType::Private)
        );
        assert_ne!(
            okx_prod.ws_endpoint(OkxChannelType::Business),
            okx_demo.ws_endpoint(OkxChannelType::Business)
        );
    }
}
