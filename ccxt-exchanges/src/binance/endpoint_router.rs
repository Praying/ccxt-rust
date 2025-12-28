//! Binance-specific endpoint router trait.
//!
//! This module provides the `BinanceEndpointRouter` trait for routing API requests
//! to the correct Binance domain based on market characteristics.
//!
//! # Binance API Domains
//!
//! Binance has a complex multi-domain structure:
//! - **Spot**: `api.binance.com` - Spot trading and margin
//! - **Linear Futures (FAPI)**: `fapi.binance.com` - USDT-margined perpetuals/futures
//! - **Inverse Futures (DAPI)**: `dapi.binance.com` - Coin-margined perpetuals/futures
//! - **Options (EAPI)**: `eapi.binance.com` - Options trading
//! - **Portfolio Margin (PAPI)**: `papi.binance.com` - Portfolio margin API
//!
//! # Example
//!
//! ```rust,no_run
//! use ccxt_exchanges::binance::{Binance, BinanceEndpointRouter};
//! use ccxt_core::types::{EndpointType, Market, MarketType};
//! use ccxt_core::ExchangeConfig;
//!
//! let binance = Binance::new(ExchangeConfig::default()).unwrap();
//!
//! // Get REST endpoint for a spot market
//! let spot_market = Market::new_spot(
//!     "BTCUSDT".to_string(),
//!     "BTC/USDT".to_string(),
//!     "BTC".to_string(),
//!     "USDT".to_string(),
//! );
//! let url = binance.rest_endpoint(&spot_market, EndpointType::Public);
//! assert!(url.contains("api.binance.com"));
//!
//! // Get default REST endpoint based on exchange options
//! let default_url = binance.default_rest_endpoint(EndpointType::Public);
//! ```

use ccxt_core::types::{EndpointType, Market, MarketType};

/// Binance-specific endpoint router trait.
///
/// This trait defines methods for obtaining the correct API endpoints based on
/// market characteristics. Binance has multiple API domains for different market
/// types (spot, linear futures, inverse futures, options).
///
/// # Implementation Notes
///
/// The routing logic follows these rules:
/// - **Spot markets**: Use `api.binance.com` endpoints
/// - **Linear Swap/Futures**: Use `fapi.binance.com` endpoints (USDT-margined)
/// - **Inverse Swap/Futures**: Use `dapi.binance.com` endpoints (Coin-margined)
/// - **Option markets**: Use `eapi.binance.com` endpoints
///
/// When sandbox mode is enabled, testnet URLs are returned instead.
pub trait BinanceEndpointRouter {
    /// Returns the REST API endpoint for a specific market.
    ///
    /// This method routes to the correct Binance domain based on the market's
    /// `market_type`, `linear`, and `inverse` fields.
    ///
    /// # Arguments
    ///
    /// * `market` - Reference to the market object containing type information
    /// * `endpoint_type` - Whether this is a public or private endpoint
    ///
    /// # Returns
    ///
    /// The REST API base URL string for the given market.
    ///
    /// # Routing Logic
    ///
    /// | Market Type | Linear | Inverse | Domain |
    /// |-------------|--------|---------|--------|
    /// | Spot | - | - | api.binance.com |
    /// | Swap/Futures | true | false | fapi.binance.com |
    /// | Swap/Futures | false | true | dapi.binance.com |
    /// | Option | - | - | eapi.binance.com |
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use ccxt_exchanges::binance::{Binance, BinanceEndpointRouter};
    /// use ccxt_core::types::{EndpointType, Market, MarketType};
    /// use ccxt_core::ExchangeConfig;
    /// use rust_decimal_macros::dec;
    ///
    /// let binance = Binance::new(ExchangeConfig::default()).unwrap();
    ///
    /// // Linear futures market
    /// let linear_market = Market::new_swap(
    ///     "BTCUSDT".to_string(),
    ///     "BTC/USDT:USDT".to_string(),
    ///     "BTC".to_string(),
    ///     "USDT".to_string(),
    ///     "USDT".to_string(),
    ///     dec!(1.0),
    /// );
    /// let url = binance.rest_endpoint(&linear_market, EndpointType::Public);
    /// assert!(url.contains("fapi.binance.com"));
    /// ```
    fn rest_endpoint(&self, market: &Market, endpoint_type: EndpointType) -> String;

    /// Returns the WebSocket endpoint for a specific market.
    ///
    /// This method routes to the correct Binance WebSocket domain based on
    /// the market's type and settlement characteristics.
    ///
    /// # Arguments
    ///
    /// * `market` - Reference to the market object containing type information
    ///
    /// # Returns
    ///
    /// The WebSocket URL string for the given market.
    ///
    /// # Routing Logic
    ///
    /// | Market Type | Linear | Inverse | WebSocket Domain |
    /// |-------------|--------|---------|------------------|
    /// | Spot | - | - | stream.binance.com |
    /// | Swap/Futures | true | false | fstream.binance.com |
    /// | Swap/Futures | false | true | dstream.binance.com |
    /// | Option | - | - | nbstream.binance.com |
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use ccxt_exchanges::binance::{Binance, BinanceEndpointRouter};
    /// use ccxt_core::types::Market;
    /// use ccxt_core::ExchangeConfig;
    ///
    /// let binance = Binance::new(ExchangeConfig::default()).unwrap();
    ///
    /// let spot_market = Market::new_spot(
    ///     "BTCUSDT".to_string(),
    ///     "BTC/USDT".to_string(),
    ///     "BTC".to_string(),
    ///     "USDT".to_string(),
    /// );
    /// let ws_url = binance.ws_endpoint(&spot_market);
    /// assert!(ws_url.contains("stream.binance.com"));
    /// ```
    fn ws_endpoint(&self, market: &Market) -> String;

    /// Returns the default REST endpoint when no specific market is provided.
    ///
    /// This method uses the exchange's `default_type` and `default_sub_type`
    /// options to determine which endpoint to return.
    ///
    /// # Arguments
    ///
    /// * `endpoint_type` - Whether this is a public or private endpoint
    ///
    /// # Returns
    ///
    /// The default REST API base URL string.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use ccxt_exchanges::binance::{Binance, BinanceEndpointRouter, BinanceOptions};
    /// use ccxt_core::types::EndpointType;
    /// use ccxt_core::types::default_type::{DefaultType, DefaultSubType};
    /// use ccxt_core::ExchangeConfig;
    ///
    /// // Create a futures-focused Binance instance
    /// let options = BinanceOptions {
    ///     default_type: DefaultType::Swap,
    ///     default_sub_type: Some(DefaultSubType::Linear),
    ///     ..Default::default()
    /// };
    /// let binance = Binance::new_with_options(ExchangeConfig::default(), options).unwrap();
    ///
    /// let url = binance.default_rest_endpoint(EndpointType::Public);
    /// assert!(url.contains("fapi.binance.com"));
    /// ```
    fn default_rest_endpoint(&self, endpoint_type: EndpointType) -> String;

    /// Returns the default WebSocket endpoint when no specific market is provided.
    ///
    /// This method uses the exchange's `default_type` and `default_sub_type`
    /// options to determine which WebSocket endpoint to return.
    ///
    /// # Returns
    ///
    /// The default WebSocket URL string.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use ccxt_exchanges::binance::{Binance, BinanceEndpointRouter};
    /// use ccxt_core::ExchangeConfig;
    ///
    /// let binance = Binance::new(ExchangeConfig::default()).unwrap();
    /// let ws_url = binance.default_ws_endpoint();
    /// // Default is spot, so should be stream.binance.com
    /// assert!(ws_url.contains("stream.binance.com"));
    /// ```
    fn default_ws_endpoint(&self) -> String;

    /// Returns the SAPI (Spot API) endpoint.
    ///
    /// SAPI is used for Binance-specific spot trading features like:
    /// - Margin trading operations
    /// - Savings and staking
    /// - Sub-account management
    /// - Asset transfers
    ///
    /// # Returns
    ///
    /// The SAPI base URL string.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use ccxt_exchanges::binance::{Binance, BinanceEndpointRouter};
    /// use ccxt_core::ExchangeConfig;
    ///
    /// let binance = Binance::new(ExchangeConfig::default()).unwrap();
    /// let sapi_url = binance.sapi_endpoint();
    /// assert!(sapi_url.contains("sapi"));
    /// ```
    fn sapi_endpoint(&self) -> String;

    /// Returns the Portfolio Margin API (PAPI) endpoint.
    ///
    /// PAPI is used for portfolio margin trading which allows cross-margining
    /// across spot, futures, and options positions.
    ///
    /// # Returns
    ///
    /// The PAPI base URL string.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use ccxt_exchanges::binance::{Binance, BinanceEndpointRouter};
    /// use ccxt_core::ExchangeConfig;
    ///
    /// let binance = Binance::new(ExchangeConfig::default()).unwrap();
    /// let papi_url = binance.papi_endpoint();
    /// assert!(papi_url.contains("papi"));
    /// ```
    fn papi_endpoint(&self) -> String;
}

use super::Binance;

use ccxt_core::types::default_type::{DefaultSubType, DefaultType};

impl BinanceEndpointRouter for Binance {
    fn rest_endpoint(&self, market: &Market, endpoint_type: EndpointType) -> String {
        let urls = self.urls();

        match market.market_type {
            MarketType::Spot => match endpoint_type {
                EndpointType::Public => urls.public.clone(),
                EndpointType::Private => urls.private.clone(),
            },
            MarketType::Swap | MarketType::Futures => {
                // Determine linear/inverse from market fields
                // Default to linear (true) if not specified
                let is_linear = market.linear.unwrap_or(true);

                if is_linear {
                    match endpoint_type {
                        EndpointType::Public => urls.fapi_public.clone(),
                        EndpointType::Private => urls.fapi_private.clone(),
                    }
                } else {
                    match endpoint_type {
                        EndpointType::Public => urls.dapi_public.clone(),
                        EndpointType::Private => urls.dapi_private.clone(),
                    }
                }
            }
            MarketType::Option => match endpoint_type {
                EndpointType::Public => urls.eapi_public.clone(),
                EndpointType::Private => urls.eapi_private.clone(),
            },
        }
    }

    fn ws_endpoint(&self, market: &Market) -> String {
        let urls = self.urls();

        match market.market_type {
            MarketType::Spot => urls.ws.clone(),
            MarketType::Swap | MarketType::Futures => {
                // Determine linear/inverse from market fields
                // Default to linear (true) if not specified
                let is_linear = market.linear.unwrap_or(true);

                if is_linear {
                    urls.ws_fapi.clone()
                } else {
                    urls.ws_dapi.clone()
                }
            }
            MarketType::Option => urls.ws_eapi.clone(),
        }
    }

    fn default_rest_endpoint(&self, endpoint_type: EndpointType) -> String {
        let urls = self.urls();
        let options = self.options();

        match options.default_type {
            DefaultType::Spot => match endpoint_type {
                EndpointType::Public => urls.public.clone(),
                EndpointType::Private => urls.private.clone(),
            },
            DefaultType::Margin => urls.sapi.clone(),
            DefaultType::Swap | DefaultType::Futures => {
                match options.default_sub_type {
                    Some(DefaultSubType::Inverse) => match endpoint_type {
                        EndpointType::Public => urls.dapi_public.clone(),
                        EndpointType::Private => urls.dapi_private.clone(),
                    },
                    _ => match endpoint_type {
                        // Default to FAPI (Linear)
                        EndpointType::Public => urls.fapi_public.clone(),
                        EndpointType::Private => urls.fapi_private.clone(),
                    },
                }
            }
            DefaultType::Option => match endpoint_type {
                EndpointType::Public => urls.eapi_public.clone(),
                EndpointType::Private => urls.eapi_private.clone(),
            },
        }
    }

    fn default_ws_endpoint(&self) -> String {
        let urls = self.urls();
        let options = self.options();

        match options.default_type {
            DefaultType::Swap | DefaultType::Futures => {
                // Check sub-type for FAPI vs DAPI selection
                match options.default_sub_type {
                    Some(DefaultSubType::Inverse) => urls.ws_dapi.clone(),
                    _ => urls.ws_fapi.clone(), // Default to FAPI (Linear) if not specified
                }
            }
            DefaultType::Option => urls.ws_eapi.clone(),
            _ => urls.ws.clone(), // Spot and Margin use standard WebSocket
        }
    }

    fn sapi_endpoint(&self) -> String {
        self.urls().sapi.clone()
    }

    fn papi_endpoint(&self) -> String {
        self.urls().papi.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ccxt_core::ExchangeConfig;
    use rust_decimal_macros::dec;

    fn create_test_binance() -> Binance {
        Binance::new(ExchangeConfig::default()).unwrap()
    }

    fn create_sandbox_binance() -> Binance {
        let config = ExchangeConfig {
            sandbox: true,
            ..Default::default()
        };
        Binance::new(config).unwrap()
    }

    // ==================== REST Endpoint Tests ====================

    #[test]
    fn test_rest_endpoint_spot_public() {
        let binance = create_test_binance();
        let market = Market::new_spot(
            "BTCUSDT".to_string(),
            "BTC/USDT".to_string(),
            "BTC".to_string(),
            "USDT".to_string(),
        );

        let url = binance.rest_endpoint(&market, EndpointType::Public);
        assert!(url.contains("api.binance.com"));
    }

    #[test]
    fn test_rest_endpoint_spot_private() {
        let binance = create_test_binance();
        let market = Market::new_spot(
            "BTCUSDT".to_string(),
            "BTC/USDT".to_string(),
            "BTC".to_string(),
            "USDT".to_string(),
        );

        let url = binance.rest_endpoint(&market, EndpointType::Private);
        assert!(url.contains("api.binance.com"));
    }

    #[test]
    fn test_rest_endpoint_linear_swap_public() {
        let binance = create_test_binance();
        let market = Market::new_swap(
            "BTCUSDT".to_string(),
            "BTC/USDT:USDT".to_string(),
            "BTC".to_string(),
            "USDT".to_string(),
            "USDT".to_string(),
            dec!(1.0),
        );

        let url = binance.rest_endpoint(&market, EndpointType::Public);
        assert!(url.contains("fapi.binance.com"));
    }

    #[test]
    fn test_rest_endpoint_linear_swap_private() {
        let binance = create_test_binance();
        let market = Market::new_swap(
            "BTCUSDT".to_string(),
            "BTC/USDT:USDT".to_string(),
            "BTC".to_string(),
            "USDT".to_string(),
            "USDT".to_string(),
            dec!(1.0),
        );

        let url = binance.rest_endpoint(&market, EndpointType::Private);
        assert!(url.contains("fapi.binance.com"));
    }

    #[test]
    fn test_rest_endpoint_inverse_swap_public() {
        let binance = create_test_binance();
        let mut market = Market::new_swap(
            "BTCUSD_PERP".to_string(),
            "BTC/USD:BTC".to_string(),
            "BTC".to_string(),
            "USD".to_string(),
            "BTC".to_string(),
            dec!(100.0),
        );
        // Ensure inverse is set correctly
        market.linear = Some(false);
        market.inverse = Some(true);

        let url = binance.rest_endpoint(&market, EndpointType::Public);
        assert!(url.contains("dapi.binance.com"));
    }

    #[test]
    fn test_rest_endpoint_inverse_swap_private() {
        let binance = create_test_binance();
        let mut market = Market::new_swap(
            "BTCUSD_PERP".to_string(),
            "BTC/USD:BTC".to_string(),
            "BTC".to_string(),
            "USD".to_string(),
            "BTC".to_string(),
            dec!(100.0),
        );
        market.linear = Some(false);
        market.inverse = Some(true);

        let url = binance.rest_endpoint(&market, EndpointType::Private);
        assert!(url.contains("dapi.binance.com"));
    }

    #[test]
    fn test_rest_endpoint_option_public() {
        let binance = create_test_binance();
        let mut market = Market::default();
        market.id = "BTC-250328-100000-C".to_string();
        market.symbol = "BTC/USDT:USDT-250328-100000-C".to_string();
        market.market_type = MarketType::Option;

        let url = binance.rest_endpoint(&market, EndpointType::Public);
        assert!(url.contains("eapi.binance.com"));
    }

    #[test]
    fn test_rest_endpoint_option_private() {
        let binance = create_test_binance();
        let mut market = Market::default();
        market.id = "BTC-250328-100000-C".to_string();
        market.symbol = "BTC/USDT:USDT-250328-100000-C".to_string();
        market.market_type = MarketType::Option;

        let url = binance.rest_endpoint(&market, EndpointType::Private);
        assert!(url.contains("eapi.binance.com"));
    }

    // ==================== WebSocket Endpoint Tests ====================

    #[test]
    fn test_ws_endpoint_spot() {
        let binance = create_test_binance();
        let market = Market::new_spot(
            "BTCUSDT".to_string(),
            "BTC/USDT".to_string(),
            "BTC".to_string(),
            "USDT".to_string(),
        );

        let url = binance.ws_endpoint(&market);
        assert!(url.contains("stream.binance.com"));
    }

    #[test]
    fn test_ws_endpoint_linear_swap() {
        let binance = create_test_binance();
        let market = Market::new_swap(
            "BTCUSDT".to_string(),
            "BTC/USDT:USDT".to_string(),
            "BTC".to_string(),
            "USDT".to_string(),
            "USDT".to_string(),
            dec!(1.0),
        );

        let url = binance.ws_endpoint(&market);
        assert!(url.contains("fstream.binance.com"));
    }

    #[test]
    fn test_ws_endpoint_inverse_swap() {
        let binance = create_test_binance();
        let mut market = Market::new_swap(
            "BTCUSD_PERP".to_string(),
            "BTC/USD:BTC".to_string(),
            "BTC".to_string(),
            "USD".to_string(),
            "BTC".to_string(),
            dec!(100.0),
        );
        market.linear = Some(false);
        market.inverse = Some(true);

        let url = binance.ws_endpoint(&market);
        assert!(url.contains("dstream.binance.com"));
    }

    #[test]
    fn test_ws_endpoint_option() {
        let binance = create_test_binance();
        let mut market = Market::default();
        market.market_type = MarketType::Option;

        let url = binance.ws_endpoint(&market);
        assert!(url.contains("nbstream.binance.com"));
    }

    // ==================== Default Endpoint Tests ====================

    #[test]
    fn test_default_rest_endpoint_spot() {
        let binance = create_test_binance();

        let url = binance.default_rest_endpoint(EndpointType::Public);
        assert!(url.contains("api.binance.com"));
    }

    #[test]
    fn test_default_ws_endpoint_spot() {
        let binance = create_test_binance();

        let url = binance.default_ws_endpoint();
        assert!(url.contains("stream.binance.com"));
    }

    // ==================== SAPI and PAPI Tests ====================

    #[test]
    fn test_sapi_endpoint() {
        let binance = create_test_binance();

        let url = binance.sapi_endpoint();
        assert!(url.contains("sapi"));
        assert!(url.contains("api.binance.com"));
    }

    #[test]
    fn test_papi_endpoint() {
        let binance = create_test_binance();

        let url = binance.papi_endpoint();
        assert!(url.contains("papi"));
    }

    // ==================== Sandbox Mode Tests ====================

    #[test]
    fn test_sandbox_rest_endpoint_spot() {
        let binance = create_sandbox_binance();
        let market = Market::new_spot(
            "BTCUSDT".to_string(),
            "BTC/USDT".to_string(),
            "BTC".to_string(),
            "USDT".to_string(),
        );

        let url = binance.rest_endpoint(&market, EndpointType::Public);
        assert!(url.contains("testnet"));
    }

    #[test]
    fn test_sandbox_ws_endpoint_spot() {
        let binance = create_sandbox_binance();
        let market = Market::new_spot(
            "BTCUSDT".to_string(),
            "BTC/USDT".to_string(),
            "BTC".to_string(),
            "USDT".to_string(),
        );

        let url = binance.ws_endpoint(&market);
        assert!(url.contains("testnet"));
    }

    #[test]
    fn test_sandbox_rest_endpoint_linear_swap() {
        let binance = create_sandbox_binance();
        let market = Market::new_swap(
            "BTCUSDT".to_string(),
            "BTC/USDT:USDT".to_string(),
            "BTC".to_string(),
            "USDT".to_string(),
            "USDT".to_string(),
            dec!(1.0),
        );

        let url = binance.rest_endpoint(&market, EndpointType::Public);
        assert!(url.contains("testnet"));
    }

    // ==================== Edge Case Tests ====================

    #[test]
    fn test_swap_defaults_to_linear_when_not_specified() {
        let binance = create_test_binance();
        let mut market = Market::default();
        market.market_type = MarketType::Swap;
        // linear and inverse are None

        let url = binance.rest_endpoint(&market, EndpointType::Public);
        // Should default to linear (fapi)
        assert!(url.contains("fapi.binance.com"));
    }

    #[test]
    fn test_futures_defaults_to_linear_when_not_specified() {
        let binance = create_test_binance();
        let mut market = Market::default();
        market.market_type = MarketType::Futures;
        // linear and inverse are None

        let url = binance.rest_endpoint(&market, EndpointType::Public);
        // Should default to linear (fapi)
        assert!(url.contains("fapi.binance.com"));
    }
}
