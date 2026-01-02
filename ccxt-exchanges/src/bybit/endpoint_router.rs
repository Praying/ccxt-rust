//! Bybit-specific endpoint router trait.
//!
//! This module provides the `BybitEndpointRouter` trait for routing API requests
//! to the correct Bybit endpoints based on market category.
//!
//! # Bybit API Structure
//!
//! Bybit V5 API uses a unified REST domain with category-based WebSocket paths:
//! - **REST**: `api.bybit.com` - Single unified REST endpoint
//! - **WebSocket Public**: `stream.bybit.com/v5/public/{category}` - Category-specific paths
//! - **WebSocket Private**: `stream.bybit.com/v5/private` - Single private endpoint
//!
//! # Categories
//!
//! Bybit uses the following categories:
//! - `spot` - Spot trading
//! - `linear` - USDT-margined perpetuals and futures
//! - `inverse` - Coin-margined perpetuals and futures
//! - `option` - Options trading
//!
//! # Example
//!
//! ```rust,no_run
//! use ccxt_exchanges::bybit::{Bybit, BybitEndpointRouter};
//! use ccxt_core::ExchangeConfig;
//!
//! let bybit = Bybit::new(ExchangeConfig::default()).unwrap();
//!
//! // Get REST endpoint (unified for all categories)
//! let rest_url = bybit.rest_endpoint();
//! assert!(rest_url.contains("api.bybit.com"));
//!
//! // Get WebSocket endpoint for linear category
//! let ws_url = bybit.ws_public_endpoint("linear");
//! assert!(ws_url.contains("/v5/public/linear"));
//!
//! // Get private WebSocket endpoint
//! let ws_private_url = bybit.ws_private_endpoint();
//! assert!(ws_private_url.contains("/v5/private"));
//! ```

/// Bybit-specific endpoint router trait.
///
/// This trait defines methods for obtaining the correct API endpoints for Bybit.
/// Unlike Binance which has multiple REST domains, Bybit uses a unified REST
/// endpoint with category-based WebSocket paths.
///
/// # Implementation Notes
///
/// - REST API uses a single unified domain for all market types
/// - WebSocket public endpoints are differentiated by category path suffix
/// - WebSocket private endpoint is unified for all authenticated streams
/// - Sandbox/testnet mode switches to testnet domains
pub trait BybitEndpointRouter {
    /// Returns the REST API endpoint.
    ///
    /// Bybit uses a unified REST domain for all market types. The category
    /// is specified as a query parameter in API requests, not in the URL.
    ///
    /// # Returns
    ///
    /// The REST API base URL string.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use ccxt_exchanges::bybit::{Bybit, BybitEndpointRouter};
    /// use ccxt_core::ExchangeConfig;
    ///
    /// let bybit = Bybit::new(ExchangeConfig::default()).unwrap();
    /// let url = bybit.rest_endpoint();
    /// assert!(url.contains("api.bybit.com"));
    /// ```
    fn rest_endpoint(&self) -> &'static str;

    /// Returns the public WebSocket endpoint for a specific category.
    ///
    /// Bybit V5 API uses different WebSocket paths for different categories:
    /// - `spot`: `/v5/public/spot`
    /// - `linear`: `/v5/public/linear`
    /// - `inverse`: `/v5/public/inverse`
    /// - `option`: `/v5/public/option`
    ///
    /// # Arguments
    ///
    /// * `category` - The market category ("spot", "linear", "inverse", "option")
    ///
    /// # Returns
    ///
    /// The complete WebSocket URL for the specified category.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use ccxt_exchanges::bybit::{Bybit, BybitEndpointRouter};
    /// use ccxt_core::ExchangeConfig;
    ///
    /// let bybit = Bybit::new(ExchangeConfig::default()).unwrap();
    ///
    /// // Get WebSocket URL for linear perpetuals
    /// let ws_url = bybit.ws_public_endpoint("linear");
    /// assert!(ws_url.contains("/v5/public/linear"));
    ///
    /// // Get WebSocket URL for spot trading
    /// let ws_spot_url = bybit.ws_public_endpoint("spot");
    /// assert!(ws_spot_url.contains("/v5/public/spot"));
    /// ```
    fn ws_public_endpoint(&self, category: &str) -> String;

    /// Returns the private WebSocket endpoint.
    ///
    /// Bybit uses a single private WebSocket endpoint for all authenticated
    /// streams regardless of market category.
    ///
    /// # Returns
    ///
    /// The private WebSocket URL string.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use ccxt_exchanges::bybit::{Bybit, BybitEndpointRouter};
    /// use ccxt_core::ExchangeConfig;
    ///
    /// let bybit = Bybit::new(ExchangeConfig::default()).unwrap();
    /// let ws_private_url = bybit.ws_private_endpoint();
    /// assert!(ws_private_url.contains("/v5/private"));
    /// ```
    fn ws_private_endpoint(&self) -> &str;
}

use super::Bybit;

impl BybitEndpointRouter for Bybit {
    fn rest_endpoint(&self) -> &'static str {
        // Use the existing urls() method which already handles sandbox mode
        // We need to return a reference, so we'll use a static approach
        // based on sandbox mode
        if self.is_sandbox() {
            "https://api-testnet.bybit.com"
        } else {
            "https://api.bybit.com"
        }
    }

    fn ws_public_endpoint(&self, category: &str) -> String {
        let urls = self.urls();
        urls.ws_public_for_category(category)
    }

    fn ws_private_endpoint(&self) -> &str {
        // Similar to rest_endpoint, return static reference based on sandbox mode
        if self.is_sandbox() {
            "wss://stream-testnet.bybit.com/v5/private"
        } else {
            "wss://stream.bybit.com/v5/private"
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bybit::BybitOptions;
    use ccxt_core::ExchangeConfig;
    use ccxt_core::types::default_type::{DefaultSubType, DefaultType};

    fn create_test_bybit() -> Bybit {
        Bybit::new(ExchangeConfig::default()).unwrap()
    }

    fn create_sandbox_bybit() -> Bybit {
        let config = ExchangeConfig {
            sandbox: true,
            ..Default::default()
        };
        Bybit::new(config).unwrap()
    }

    // ==================== REST Endpoint Tests ====================

    #[test]
    fn test_rest_endpoint_production() {
        let bybit = create_test_bybit();
        let url = bybit.rest_endpoint();
        assert!(url.contains("api.bybit.com"));
        assert!(!url.contains("testnet"));
    }

    #[test]
    fn test_rest_endpoint_sandbox() {
        let bybit = create_sandbox_bybit();
        let url = bybit.rest_endpoint();
        assert!(url.contains("api-testnet.bybit.com"));
    }

    // ==================== WebSocket Public Endpoint Tests ====================

    #[test]
    fn test_ws_public_endpoint_spot() {
        let bybit = create_test_bybit();
        let url = bybit.ws_public_endpoint("spot");
        assert!(url.contains("stream.bybit.com"));
        assert!(url.ends_with("/v5/public/spot"));
    }

    #[test]
    fn test_ws_public_endpoint_linear() {
        let bybit = create_test_bybit();
        let url = bybit.ws_public_endpoint("linear");
        assert!(url.contains("stream.bybit.com"));
        assert!(url.ends_with("/v5/public/linear"));
    }

    #[test]
    fn test_ws_public_endpoint_inverse() {
        let bybit = create_test_bybit();
        let url = bybit.ws_public_endpoint("inverse");
        assert!(url.contains("stream.bybit.com"));
        assert!(url.ends_with("/v5/public/inverse"));
    }

    #[test]
    fn test_ws_public_endpoint_option() {
        let bybit = create_test_bybit();
        let url = bybit.ws_public_endpoint("option");
        assert!(url.contains("stream.bybit.com"));
        assert!(url.ends_with("/v5/public/option"));
    }

    #[test]
    fn test_ws_public_endpoint_sandbox_spot() {
        let bybit = create_sandbox_bybit();
        let url = bybit.ws_public_endpoint("spot");
        assert!(url.contains("stream-testnet.bybit.com"));
        assert!(url.ends_with("/v5/public/spot"));
    }

    #[test]
    fn test_ws_public_endpoint_sandbox_linear() {
        let bybit = create_sandbox_bybit();
        let url = bybit.ws_public_endpoint("linear");
        assert!(url.contains("stream-testnet.bybit.com"));
        assert!(url.ends_with("/v5/public/linear"));
    }

    // ==================== WebSocket Private Endpoint Tests ====================

    #[test]
    fn test_ws_private_endpoint_production() {
        let bybit = create_test_bybit();
        let url = bybit.ws_private_endpoint();
        assert!(url.contains("stream.bybit.com"));
        assert!(url.contains("/v5/private"));
        assert!(!url.contains("testnet"));
    }

    #[test]
    fn test_ws_private_endpoint_sandbox() {
        let bybit = create_sandbox_bybit();
        let url = bybit.ws_private_endpoint();
        assert!(url.contains("stream-testnet.bybit.com"));
        assert!(url.contains("/v5/private"));
    }

    // ==================== Category Path Construction Tests ====================

    #[test]
    fn test_ws_public_endpoint_path_format() {
        let bybit = create_test_bybit();

        // Verify all categories follow the /v5/public/{category} format
        let categories = ["spot", "linear", "inverse", "option"];
        for category in categories {
            let url = bybit.ws_public_endpoint(category);
            let expected_suffix = format!("/v5/public/{}", category);
            assert!(
                url.ends_with(&expected_suffix),
                "URL {} should end with {}",
                url,
                expected_suffix
            );
        }
    }

    // ==================== Testnet Option Tests ====================

    #[test]
    fn test_rest_endpoint_with_testnet_option() {
        let config = ExchangeConfig::default();
        let options = BybitOptions {
            testnet: true,
            ..Default::default()
        };
        let bybit = Bybit::new_with_options(config, options).unwrap();

        let url = bybit.rest_endpoint();
        assert!(url.contains("api-testnet.bybit.com"));
    }

    #[test]
    fn test_ws_private_endpoint_with_testnet_option() {
        let config = ExchangeConfig::default();
        let options = BybitOptions {
            testnet: true,
            ..Default::default()
        };
        let bybit = Bybit::new_with_options(config, options).unwrap();

        let url = bybit.ws_private_endpoint();
        assert!(url.contains("stream-testnet.bybit.com"));
    }

    // ==================== Integration with Default Type Tests ====================

    #[test]
    fn test_ws_public_endpoint_with_linear_default_type() {
        let config = ExchangeConfig::default();
        let options = BybitOptions {
            default_type: DefaultType::Swap,
            default_sub_type: Some(DefaultSubType::Linear),
            ..Default::default()
        };
        let bybit = Bybit::new_with_options(config, options).unwrap();

        // Use the category() method to get the correct category
        let category = bybit.category();
        assert_eq!(category, "linear");

        let url = bybit.ws_public_endpoint(category);
        assert!(url.ends_with("/v5/public/linear"));
    }

    #[test]
    fn test_ws_public_endpoint_with_inverse_default_type() {
        let config = ExchangeConfig::default();
        let options = BybitOptions {
            default_type: DefaultType::Swap,
            default_sub_type: Some(DefaultSubType::Inverse),
            ..Default::default()
        };
        let bybit = Bybit::new_with_options(config, options).unwrap();

        let category = bybit.category();
        assert_eq!(category, "inverse");

        let url = bybit.ws_public_endpoint(category);
        assert!(url.ends_with("/v5/public/inverse"));
    }
}
