//! Bitget-specific endpoint router trait.
//!
//! This module provides the `BitgetEndpointRouter` trait for routing API requests
//! to the correct Bitget endpoints based on endpoint type (public/private).
//!
//! # Bitget API Structure
//!
//! Bitget uses a simple dual-endpoint structure:
//! - **REST**: `api.bitget.com` - Single REST endpoint for all operations
//! - **WebSocket Public**: `ws.bitget.com/v2/ws/public` - Public market data
//! - **WebSocket Private**: `ws.bitget.com/v2/ws/private` - Account data
//!
//! # Testnet/Sandbox Mode
//!
//! Bitget provides a separate testnet environment:
//! - REST: `api-testnet.bitget.com`
//! - WS Public: `ws-testnet.bitget.com/v2/ws/public`
//! - WS Private: `ws-testnet.bitget.com/v2/ws/private`
//!
//! # Example
//!
//! ```rust,no_run
//! use ccxt_exchanges::bitget::{Bitget, BitgetEndpointRouter};
//! use ccxt_core::types::EndpointType;
//! use ccxt_core::ExchangeConfig;
//!
//! let bitget = Bitget::new(ExchangeConfig::default()).unwrap();
//!
//! // Get REST endpoint
//! let rest_url = bitget.rest_endpoint();
//! assert!(rest_url.contains("api.bitget.com"));
//!
//! // Get WebSocket endpoint for public data
//! let ws_public = bitget.ws_endpoint(EndpointType::Public);
//! assert!(ws_public.contains("/v2/ws/public"));
//!
//! // Get WebSocket endpoint for private data
//! let ws_private = bitget.ws_endpoint(EndpointType::Private);
//! assert!(ws_private.contains("/v2/ws/private"));
//! ```

use ccxt_core::types::EndpointType;

/// Bitget-specific endpoint router trait.
///
/// This trait defines methods for obtaining the correct API endpoints for Bitget.
/// Bitget uses a simple structure with separate public and private WebSocket endpoints.
///
/// # Implementation Notes
///
/// - REST API uses a single domain for all operations
/// - WebSocket has separate endpoints for public and private data
/// - Testnet mode switches to completely different domains
///
/// # Sandbox/Testnet Mode
///
/// When sandbox mode is enabled (via `config.sandbox` or `options.testnet`):
/// - REST URL changes to `api-testnet.bitget.com`
/// - WebSocket URLs change to `ws-testnet.bitget.com`
pub trait BitgetEndpointRouter {
    /// Returns the REST API endpoint.
    ///
    /// Bitget uses a single REST domain for all market types and operations.
    /// The product type (spot, umcbl, dmcbl) is specified in the API path,
    /// not in the domain.
    ///
    /// # Returns
    ///
    /// The REST API base URL string.
    ///
    /// # Production vs Testnet
    ///
    /// - Production: `https://api.bitget.com`
    /// - Testnet: `https://api-testnet.bitget.com`
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use ccxt_exchanges::bitget::{Bitget, BitgetEndpointRouter};
    /// use ccxt_core::ExchangeConfig;
    ///
    /// let bitget = Bitget::new(ExchangeConfig::default()).unwrap();
    /// let url = bitget.rest_endpoint();
    /// assert_eq!(url, "https://api.bitget.com");
    /// ```
    fn rest_endpoint(&self) -> &'static str;

    /// Returns the WebSocket endpoint for a specific endpoint type.
    ///
    /// Bitget uses separate WebSocket URLs for public and private data:
    /// - `Public`: `/v2/ws/public` - Market data (no authentication)
    /// - `Private`: `/v2/ws/private` - Account data (authentication required)
    ///
    /// # Arguments
    ///
    /// * `endpoint_type` - The endpoint type (Public or Private)
    ///
    /// # Returns
    ///
    /// The complete WebSocket URL for the specified endpoint type.
    ///
    /// # Production vs Testnet
    ///
    /// Production:
    /// - Public: `wss://ws.bitget.com/v2/ws/public`
    /// - Private: `wss://ws.bitget.com/v2/ws/private`
    ///
    /// Testnet:
    /// - Public: `wss://ws-testnet.bitget.com/v2/ws/public`
    /// - Private: `wss://ws-testnet.bitget.com/v2/ws/private`
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use ccxt_exchanges::bitget::{Bitget, BitgetEndpointRouter};
    /// use ccxt_core::types::EndpointType;
    /// use ccxt_core::ExchangeConfig;
    ///
    /// let bitget = Bitget::new(ExchangeConfig::default()).unwrap();
    ///
    /// // Get WebSocket URL for public market data
    /// let ws_public = bitget.ws_endpoint(EndpointType::Public);
    /// assert!(ws_public.contains("/v2/ws/public"));
    ///
    /// // Get WebSocket URL for private account data
    /// let ws_private = bitget.ws_endpoint(EndpointType::Private);
    /// assert!(ws_private.contains("/v2/ws/private"));
    /// ```
    fn ws_endpoint(&self, endpoint_type: EndpointType) -> &str;
}

use super::Bitget;

impl BitgetEndpointRouter for Bitget {
    fn rest_endpoint(&self) -> &'static str {
        // Return static reference based on sandbox mode
        // This matches the pattern used by other exchanges
        if self.is_sandbox() {
            "https://api-testnet.bitget.com"
        } else {
            "https://api.bitget.com"
        }
    }

    fn ws_endpoint(&self, endpoint_type: EndpointType) -> &str {
        // Return static reference based on sandbox mode and endpoint type
        if self.is_sandbox() {
            match endpoint_type {
                EndpointType::Public => "wss://ws-testnet.bitget.com/v2/ws/public",
                EndpointType::Private => "wss://ws-testnet.bitget.com/v2/ws/private",
            }
        } else {
            match endpoint_type {
                EndpointType::Public => "wss://ws.bitget.com/v2/ws/public",
                EndpointType::Private => "wss://ws.bitget.com/v2/ws/private",
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::disallowed_methods)]
    use super::*;
    use crate::bitget::BitgetOptions;
    use ccxt_core::ExchangeConfig;

    fn create_test_bitget() -> Bitget {
        Bitget::new(ExchangeConfig::default()).unwrap()
    }

    fn create_sandbox_bitget() -> Bitget {
        let config = ExchangeConfig {
            sandbox: true,
            ..Default::default()
        };
        Bitget::new(config).unwrap()
    }

    // ==================== REST Endpoint Tests ====================

    #[test]
    fn test_rest_endpoint_production() {
        let bitget = create_test_bitget();
        let url = bitget.rest_endpoint();
        assert_eq!(url, "https://api.bitget.com");
        assert!(!url.contains("testnet"));
    }

    #[test]
    fn test_rest_endpoint_sandbox() {
        let bitget = create_sandbox_bitget();
        let url = bitget.rest_endpoint();
        assert_eq!(url, "https://api-testnet.bitget.com");
    }

    // ==================== WebSocket Public Endpoint Tests ====================

    #[test]
    fn test_ws_endpoint_public_production() {
        let bitget = create_test_bitget();
        let url = bitget.ws_endpoint(EndpointType::Public);
        assert_eq!(url, "wss://ws.bitget.com/v2/ws/public");
        assert!(!url.contains("testnet"));
    }

    #[test]
    fn test_ws_endpoint_public_sandbox() {
        let bitget = create_sandbox_bitget();
        let url = bitget.ws_endpoint(EndpointType::Public);
        assert_eq!(url, "wss://ws-testnet.bitget.com/v2/ws/public");
    }

    // ==================== WebSocket Private Endpoint Tests ====================

    #[test]
    fn test_ws_endpoint_private_production() {
        let bitget = create_test_bitget();
        let url = bitget.ws_endpoint(EndpointType::Private);
        assert_eq!(url, "wss://ws.bitget.com/v2/ws/private");
        assert!(!url.contains("testnet"));
    }

    #[test]
    fn test_ws_endpoint_private_sandbox() {
        let bitget = create_sandbox_bitget();
        let url = bitget.ws_endpoint(EndpointType::Private);
        assert_eq!(url, "wss://ws-testnet.bitget.com/v2/ws/private");
    }

    // ==================== Testnet Option Tests ====================

    #[test]
    fn test_rest_endpoint_with_testnet_option() {
        let config = ExchangeConfig::default();
        let options = BitgetOptions {
            testnet: true,
            ..Default::default()
        };
        let bitget = Bitget::new_with_options(config, options).unwrap();

        let url = bitget.rest_endpoint();
        assert_eq!(url, "https://api-testnet.bitget.com");
    }

    #[test]
    fn test_ws_endpoint_public_with_testnet_option() {
        let config = ExchangeConfig::default();
        let options = BitgetOptions {
            testnet: true,
            ..Default::default()
        };
        let bitget = Bitget::new_with_options(config, options).unwrap();

        let url = bitget.ws_endpoint(EndpointType::Public);
        assert_eq!(url, "wss://ws-testnet.bitget.com/v2/ws/public");
    }

    #[test]
    fn test_ws_endpoint_private_with_testnet_option() {
        let config = ExchangeConfig::default();
        let options = BitgetOptions {
            testnet: true,
            ..Default::default()
        };
        let bitget = Bitget::new_with_options(config, options).unwrap();

        let url = bitget.ws_endpoint(EndpointType::Private);
        assert_eq!(url, "wss://ws-testnet.bitget.com/v2/ws/private");
    }

    // ==================== Consistency Tests ====================

    #[test]
    fn test_rest_endpoint_consistency_with_urls() {
        let bitget = create_test_bitget();
        let router_url = bitget.rest_endpoint();
        let urls_rest = bitget.urls().rest;
        assert_eq!(router_url, urls_rest);
    }

    #[test]
    fn test_ws_public_endpoint_consistency_with_urls() {
        let bitget = create_test_bitget();
        let router_url = bitget.ws_endpoint(EndpointType::Public);
        let urls_ws_public = bitget.urls().ws_public;
        assert_eq!(router_url, urls_ws_public);
    }

    #[test]
    fn test_ws_private_endpoint_consistency_with_urls() {
        let bitget = create_test_bitget();
        let router_url = bitget.ws_endpoint(EndpointType::Private);
        let urls_ws_private = bitget.urls().ws_private;
        assert_eq!(router_url, urls_ws_private);
    }

    #[test]
    fn test_sandbox_rest_endpoint_consistency_with_urls() {
        let bitget = create_sandbox_bitget();
        let router_url = bitget.rest_endpoint();
        let urls_rest = bitget.urls().rest;
        assert_eq!(router_url, urls_rest);
    }

    #[test]
    fn test_sandbox_ws_public_endpoint_consistency_with_urls() {
        let bitget = create_sandbox_bitget();
        let router_url = bitget.ws_endpoint(EndpointType::Public);
        let urls_ws_public = bitget.urls().ws_public;
        assert_eq!(router_url, urls_ws_public);
    }

    #[test]
    fn test_sandbox_ws_private_endpoint_consistency_with_urls() {
        let bitget = create_sandbox_bitget();
        let router_url = bitget.ws_endpoint(EndpointType::Private);
        let urls_ws_private = bitget.urls().ws_private;
        assert_eq!(router_url, urls_ws_private);
    }

    // ==================== Endpoint Type Tests ====================

    #[test]
    fn test_endpoint_type_public_is_public() {
        assert!(EndpointType::Public.is_public());
        assert!(!EndpointType::Public.is_private());
    }

    #[test]
    fn test_endpoint_type_private_is_private() {
        assert!(EndpointType::Private.is_private());
        assert!(!EndpointType::Private.is_public());
    }

    // ==================== URL Format Tests ====================

    #[test]
    fn test_rest_endpoint_uses_https() {
        let bitget = create_test_bitget();
        let url = bitget.rest_endpoint();
        assert!(url.starts_with("https://"));
    }

    #[test]
    fn test_ws_endpoint_uses_wss() {
        let bitget = create_test_bitget();

        let ws_public = bitget.ws_endpoint(EndpointType::Public);
        assert!(ws_public.starts_with("wss://"));

        let ws_private = bitget.ws_endpoint(EndpointType::Private);
        assert!(ws_private.starts_with("wss://"));
    }

    #[test]
    fn test_ws_endpoint_contains_v2_path() {
        let bitget = create_test_bitget();

        let ws_public = bitget.ws_endpoint(EndpointType::Public);
        assert!(ws_public.contains("/v2/ws/"));

        let ws_private = bitget.ws_endpoint(EndpointType::Private);
        assert!(ws_private.contains("/v2/ws/"));
    }
}
