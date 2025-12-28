//! HyperLiquid-specific endpoint router trait.
//!
//! This module provides the `HyperLiquidEndpointRouter` trait for routing API requests
//! to the correct HyperLiquid endpoints.
//!
//! # HyperLiquid API Structure
//!
//! HyperLiquid uses the simplest endpoint structure among all supported exchanges:
//! - **REST**: `api.hyperliquid.xyz` - Single REST endpoint for all operations
//! - **WebSocket**: `api.hyperliquid.xyz/ws` - Single WebSocket endpoint
//!
//! Unlike centralized exchanges, HyperLiquid is a decentralized perpetual futures
//! exchange built on its own L1 blockchain. It uses:
//! - Ethereum wallet private keys for authentication (EIP-712 typed data signatures)
//! - Wallet addresses as account identifiers (no registration required)
//! - USDC as the sole settlement currency
//!
//! # Testnet/Sandbox Mode
//!
//! HyperLiquid provides a separate testnet environment:
//! - REST: `api.hyperliquid-testnet.xyz`
//! - WebSocket: `api.hyperliquid-testnet.xyz/ws`
//!
//! # Example
//!
//! ```rust,no_run
//! use ccxt_exchanges::hyperliquid::{HyperLiquid, HyperLiquidEndpointRouter};
//!
//! let exchange = HyperLiquid::builder()
//!     .testnet(false)
//!     .build()
//!     .unwrap();
//!
//! // Get REST endpoint
//! let rest_url = exchange.rest_endpoint();
//! assert!(rest_url.contains("api.hyperliquid.xyz"));
//!
//! // Get WebSocket endpoint
//! let ws_url = exchange.ws_endpoint();
//! assert!(ws_url.contains("api.hyperliquid.xyz/ws"));
//! ```

/// HyperLiquid-specific endpoint router trait.
///
/// This trait defines methods for obtaining the correct API endpoints for HyperLiquid.
/// HyperLiquid uses the simplest structure with a single REST and WebSocket endpoint.
///
/// # Implementation Notes
///
/// - REST API uses a single domain for all operations (`/info` for public, `/exchange` for private)
/// - WebSocket has a single endpoint for all data streams
/// - Authentication is handled via EIP-712 signatures, not separate endpoints
/// - Testnet mode switches to completely different domains
///
/// # Sandbox/Testnet Mode
///
/// When sandbox mode is enabled (via `config.sandbox` or `options.testnet`):
/// - REST URL changes to `api.hyperliquid-testnet.xyz`
/// - WebSocket URL changes to `api.hyperliquid-testnet.xyz/ws`
pub trait HyperLiquidEndpointRouter {
    /// Returns the REST API endpoint.
    ///
    /// HyperLiquid uses a single REST domain for all operations.
    /// The operation type is specified in the API path:
    /// - `/info` - Public data requests
    /// - `/exchange` - Private/authenticated requests
    ///
    /// # Returns
    ///
    /// The REST API base URL string.
    ///
    /// # Production vs Testnet
    ///
    /// - Production: `https://api.hyperliquid.xyz`
    /// - Testnet: `https://api.hyperliquid-testnet.xyz`
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use ccxt_exchanges::hyperliquid::{HyperLiquid, HyperLiquidEndpointRouter};
    ///
    /// let exchange = HyperLiquid::builder().build().unwrap();
    /// let url = exchange.rest_endpoint();
    /// assert_eq!(url, "https://api.hyperliquid.xyz");
    /// ```
    fn rest_endpoint(&self) -> &str;

    /// Returns the WebSocket endpoint.
    ///
    /// HyperLiquid uses a single WebSocket endpoint for all data streams.
    /// Unlike other exchanges, there is no separation between public and private
    /// WebSocket connections - authentication is handled at the message level.
    ///
    /// # Returns
    ///
    /// The complete WebSocket URL.
    ///
    /// # Production vs Testnet
    ///
    /// - Production: `wss://api.hyperliquid.xyz/ws`
    /// - Testnet: `wss://api.hyperliquid-testnet.xyz/ws`
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use ccxt_exchanges::hyperliquid::{HyperLiquid, HyperLiquidEndpointRouter};
    ///
    /// let exchange = HyperLiquid::builder().build().unwrap();
    /// let url = exchange.ws_endpoint();
    /// assert_eq!(url, "wss://api.hyperliquid.xyz/ws");
    /// ```
    fn ws_endpoint(&self) -> &str;
}

use super::HyperLiquid;

impl HyperLiquidEndpointRouter for HyperLiquid {
    fn rest_endpoint(&self) -> &str {
        // Return static reference based on sandbox mode
        // is_sandbox() checks both config.sandbox and options.testnet
        if self.is_sandbox() {
            "https://api.hyperliquid-testnet.xyz"
        } else {
            "https://api.hyperliquid.xyz"
        }
    }

    fn ws_endpoint(&self) -> &str {
        // Return static reference based on sandbox mode
        // is_sandbox() checks both config.sandbox and options.testnet
        if self.is_sandbox() {
            "wss://api.hyperliquid-testnet.xyz/ws"
        } else {
            "wss://api.hyperliquid.xyz/ws"
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hyperliquid::HyperLiquidOptions;
    use ccxt_core::ExchangeConfig;

    fn create_test_hyperliquid() -> HyperLiquid {
        HyperLiquid::builder().build().unwrap()
    }

    fn create_testnet_hyperliquid() -> HyperLiquid {
        HyperLiquid::builder().testnet(true).build().unwrap()
    }

    fn create_sandbox_hyperliquid() -> HyperLiquid {
        let config = ExchangeConfig {
            sandbox: true,
            ..Default::default()
        };
        let options = HyperLiquidOptions::default();
        HyperLiquid::new_with_options(config, options, None).unwrap()
    }

    // ==================== REST Endpoint Tests ====================

    #[test]
    fn test_rest_endpoint_production() {
        let exchange = create_test_hyperliquid();
        let url = exchange.rest_endpoint();
        assert_eq!(url, "https://api.hyperliquid.xyz");
        assert!(!url.contains("testnet"));
    }

    #[test]
    fn test_rest_endpoint_testnet() {
        let exchange = create_testnet_hyperliquid();
        let url = exchange.rest_endpoint();
        assert_eq!(url, "https://api.hyperliquid-testnet.xyz");
    }

    #[test]
    fn test_rest_endpoint_sandbox() {
        let exchange = create_sandbox_hyperliquid();
        let url = exchange.rest_endpoint();
        assert_eq!(url, "https://api.hyperliquid-testnet.xyz");
    }

    // ==================== WebSocket Endpoint Tests ====================

    #[test]
    fn test_ws_endpoint_production() {
        let exchange = create_test_hyperliquid();
        let url = exchange.ws_endpoint();
        assert_eq!(url, "wss://api.hyperliquid.xyz/ws");
        assert!(!url.contains("testnet"));
    }

    #[test]
    fn test_ws_endpoint_testnet() {
        let exchange = create_testnet_hyperliquid();
        let url = exchange.ws_endpoint();
        assert_eq!(url, "wss://api.hyperliquid-testnet.xyz/ws");
    }

    #[test]
    fn test_ws_endpoint_sandbox() {
        let exchange = create_sandbox_hyperliquid();
        let url = exchange.ws_endpoint();
        assert_eq!(url, "wss://api.hyperliquid-testnet.xyz/ws");
    }

    // ==================== Consistency Tests ====================

    #[test]
    fn test_rest_endpoint_consistency_with_urls() {
        let exchange = create_test_hyperliquid();
        let router_url = exchange.rest_endpoint();
        let urls_rest = &exchange.urls().rest;
        assert_eq!(router_url, urls_rest);
    }

    #[test]
    fn test_ws_endpoint_consistency_with_urls() {
        let exchange = create_test_hyperliquid();
        let router_url = exchange.ws_endpoint();
        let urls_ws = &exchange.urls().ws;
        assert_eq!(router_url, urls_ws);
    }

    #[test]
    fn test_testnet_rest_endpoint_consistency_with_urls() {
        let exchange = create_testnet_hyperliquid();
        let router_url = exchange.rest_endpoint();
        let urls_rest = &exchange.urls().rest;
        assert_eq!(router_url, urls_rest);
    }

    #[test]
    fn test_testnet_ws_endpoint_consistency_with_urls() {
        let exchange = create_testnet_hyperliquid();
        let router_url = exchange.ws_endpoint();
        let urls_ws = &exchange.urls().ws;
        assert_eq!(router_url, urls_ws);
    }

    // ==================== URL Format Tests ====================

    #[test]
    fn test_rest_endpoint_uses_https() {
        let exchange = create_test_hyperliquid();
        let url = exchange.rest_endpoint();
        assert!(url.starts_with("https://"));
    }

    #[test]
    fn test_ws_endpoint_uses_wss() {
        let exchange = create_test_hyperliquid();
        let url = exchange.ws_endpoint();
        assert!(url.starts_with("wss://"));
    }

    #[test]
    fn test_ws_endpoint_contains_ws_path() {
        let exchange = create_test_hyperliquid();
        let url = exchange.ws_endpoint();
        assert!(url.ends_with("/ws"));
    }

    // ==================== Sandbox Mode Tests ====================

    #[test]
    fn test_is_sandbox_with_testnet_option() {
        let exchange = create_testnet_hyperliquid();
        assert!(exchange.is_sandbox());
    }

    #[test]
    fn test_is_sandbox_with_config_sandbox() {
        let exchange = create_sandbox_hyperliquid();
        assert!(exchange.is_sandbox());
    }

    #[test]
    fn test_is_not_sandbox_by_default() {
        let exchange = create_test_hyperliquid();
        assert!(!exchange.is_sandbox());
    }
}
