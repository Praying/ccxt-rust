#![allow(clippy::disallowed_methods)]
//! Property-based tests for Sandbox/Testnet Support.
//!
//! These tests verify correctness properties for sandbox/testnet URL selection
//! across all supported exchanges using proptest.
//!
//! **Feature: sandbox-testnet-support**

use proptest::prelude::*;

/// For any exchange instance and any sandbox configuration (true/false), the URLs
/// returned by `urls()` SHALL be consistent with the sandbox setting - testnet URLs
/// when sandbox is true, production URLs when sandbox is false.
mod sandbox_url_selection_consistency {
    use super::*;
    use ccxt_exchanges::binance::BinanceBuilder;
    use ccxt_exchanges::bitget::BitgetBuilder;
    use ccxt_exchanges::bybit::BybitBuilder;
    use ccxt_exchanges::hyperliquid::HyperLiquidBuilder;
    use ccxt_exchanges::okx::OkxBuilder;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Test Binance sandbox URL selection consistency
        /// When sandbox=true, URLs should contain testnet domains
        /// When sandbox=false, URLs should contain production domains
        #[test]
        fn prop_binance_sandbox_url_consistency(sandbox in any::<bool>()) {
            let binance = BinanceBuilder::new()
                .sandbox(sandbox)
                .build()
                .expect("Should build Binance instance");

            let urls = binance.urls();

            if sandbox {
                // Testnet URLs should contain testnet domains
                prop_assert!(
                    urls.public.contains("testnet.binance.vision"),
                    "Binance public REST URL should be testnet when sandbox=true, got: {}",
                    urls.public
                );
            } else {
                // Production URLs should contain production domains
                prop_assert!(
                    urls.public.contains("api.binance.com"),
                    "Binance public REST URL should be production when sandbox=false, got: {}",
                    urls.public
                );
            }

            // Verify is_sandbox() matches the configuration
            prop_assert_eq!(
                binance.is_sandbox(),
                sandbox,
                "is_sandbox() should match sandbox configuration"
            );
        }

        /// Test OKX sandbox URL selection consistency
        /// OKX uses same REST domain but different WebSocket URLs for demo mode
        #[test]
        fn prop_okx_sandbox_url_consistency(sandbox in any::<bool>()) {
            let okx = OkxBuilder::new()
                .sandbox(sandbox)
                .build()
                .expect("Should build OKX instance");

            let urls = okx.urls();

            // OKX REST URL should always be production domain (demo uses header)
            prop_assert!(
                urls.rest.contains("www.okx.com"),
                "OKX REST URL should always be production domain, got: {}",
                urls.rest
            );

            if sandbox {
                // Demo WebSocket URLs should contain demo endpoint
                prop_assert!(
                    urls.ws_public.contains("wspap.okx.com") || urls.ws_public.contains("brokerId=9999"),
                    "OKX WS URL should be demo when sandbox=true, got: {}",
                    urls.ws_public
                );
            } else {
                // Production WebSocket URLs
                prop_assert!(
                    urls.ws_public.contains("ws.okx.com"),
                    "OKX WS URL should be production when sandbox=false, got: {}",
                    urls.ws_public
                );
            }

            // Verify is_sandbox() matches the configuration
            prop_assert_eq!(
                okx.is_sandbox(),
                sandbox,
                "is_sandbox() should match sandbox configuration"
            );
        }

        /// Test Bybit sandbox URL selection consistency
        #[test]
        fn prop_bybit_sandbox_url_consistency(sandbox in any::<bool>()) {
            let bybit = BybitBuilder::new()
                .sandbox(sandbox)
                .build()
                .expect("Should build Bybit instance");

            let urls = bybit.urls();

            if sandbox {
                // Testnet URLs
                prop_assert!(
                    urls.rest.contains("api-testnet.bybit.com"),
                    "Bybit REST URL should be testnet when sandbox=true, got: {}",
                    urls.rest
                );
            } else {
                // Production URLs
                prop_assert!(
                    urls.rest.contains("api.bybit.com") && !urls.rest.contains("testnet"),
                    "Bybit REST URL should be production when sandbox=false, got: {}",
                    urls.rest
                );
            }

            // Verify is_sandbox() matches the configuration
            prop_assert_eq!(
                bybit.is_sandbox(),
                sandbox,
                "is_sandbox() should match sandbox configuration"
            );
        }

        /// Test Bitget sandbox URL selection consistency
        #[test]
        fn prop_bitget_sandbox_url_consistency(sandbox in any::<bool>()) {
            let bitget = BitgetBuilder::new()
                .sandbox(sandbox)
                .build()
                .expect("Should build Bitget instance");

            let urls = bitget.urls();

            if sandbox {
                // Testnet URLs
                prop_assert!(
                    urls.rest.contains("api-testnet.bitget.com"),
                    "Bitget REST URL should be testnet when sandbox=true, got: {}",
                    urls.rest
                );
            } else {
                // Production URLs
                prop_assert!(
                    urls.rest.contains("api.bitget.com") && !urls.rest.contains("testnet"),
                    "Bitget REST URL should be production when sandbox=false, got: {}",
                    urls.rest
                );
            }

            // Verify is_sandbox() matches the configuration
            prop_assert_eq!(
                bitget.is_sandbox(),
                sandbox,
                "is_sandbox() should match sandbox configuration"
            );
        }

        /// Test Hyperliquid sandbox URL selection consistency
        #[test]
        fn prop_hyperliquid_sandbox_url_consistency(sandbox in any::<bool>()) {
            let hyperliquid = HyperLiquidBuilder::new()
                .sandbox(sandbox)
                .build()
                .expect("Should build Hyperliquid instance");

            let urls = hyperliquid.urls();

            if sandbox {
                // Testnet URLs
                prop_assert!(
                    urls.rest.contains("hyperliquid-testnet.xyz"),
                    "Hyperliquid REST URL should be testnet when sandbox=true, got: {}",
                    urls.rest
                );
            } else {
                // Production URLs
                prop_assert!(
                    urls.rest.contains("api.hyperliquid.xyz") && !urls.rest.contains("testnet"),
                    "Hyperliquid REST URL should be production when sandbox=false, got: {}",
                    urls.rest
                );
            }

            // Verify is_sandbox() matches the configuration
            prop_assert_eq!(
                hyperliquid.is_sandbox(),
                sandbox,
                "is_sandbox() should match sandbox configuration"
            );
        }

        /// Test that all exchanges have consistent sandbox URL selection
        /// This is a comprehensive test that verifies all exchanges together
        #[test]
        fn prop_all_exchanges_sandbox_consistency(sandbox in any::<bool>()) {
            // Build all exchanges with the same sandbox setting
            let binance = BinanceBuilder::new().sandbox(sandbox).build().expect("Binance");
            let okx = OkxBuilder::new().sandbox(sandbox).build().expect("OKX");
            let bybit = BybitBuilder::new().sandbox(sandbox).build().expect("Bybit");
            let bitget = BitgetBuilder::new().sandbox(sandbox).build().expect("Bitget");
            let hyperliquid = HyperLiquidBuilder::new().sandbox(sandbox).build().expect("Hyperliquid");

            // All is_sandbox() methods should return the same value
            prop_assert_eq!(binance.is_sandbox(), sandbox, "Binance is_sandbox mismatch");
            prop_assert_eq!(okx.is_sandbox(), sandbox, "OKX is_sandbox mismatch");
            prop_assert_eq!(bybit.is_sandbox(), sandbox, "Bybit is_sandbox mismatch");
            prop_assert_eq!(bitget.is_sandbox(), sandbox, "Bitget is_sandbox mismatch");
            prop_assert_eq!(hyperliquid.is_sandbox(), sandbox, "Hyperliquid is_sandbox mismatch");

            // All URLs should be consistent with sandbox setting
            let binance_urls = binance.urls();
            let okx_urls = okx.urls();
            let bybit_urls = bybit.urls();
            let bitget_urls = bitget.urls();
            let hyperliquid_urls = hyperliquid.urls();

            if sandbox {
                // All testnet URLs should contain testnet indicators
                prop_assert!(binance_urls.public.contains("testnet"), "Binance testnet URL");
                // OKX uses same REST domain for demo (header-based)
                prop_assert!(bybit_urls.rest.contains("testnet"), "Bybit testnet URL");
                prop_assert!(bitget_urls.rest.contains("testnet"), "Bitget testnet URL");
                prop_assert!(hyperliquid_urls.rest.contains("testnet"), "Hyperliquid testnet URL");
            } else {
                // All production URLs should NOT contain testnet
                prop_assert!(!binance_urls.public.contains("testnet"), "Binance production URL");
                prop_assert!(!okx_urls.rest.contains("testnet"), "OKX production URL");
                prop_assert!(!bybit_urls.rest.contains("testnet"), "Bybit production URL");
                prop_assert!(!bitget_urls.rest.contains("testnet"), "Bitget production URL");
                prop_assert!(!hyperliquid_urls.rest.contains("testnet"), "Hyperliquid production URL");
            }
        }
    }
}

/// For any exchange instance with sandbox mode enabled, the WebSocket URLs SHALL
/// point to the testnet/demo WebSocket endpoints appropriate for that exchange.
mod websocket_url_selection_consistency {
    use super::*;
    use ccxt_exchanges::binance::BinanceBuilder;
    use ccxt_exchanges::bitget::BitgetBuilder;
    use ccxt_exchanges::bybit::BybitBuilder;
    use ccxt_exchanges::hyperliquid::HyperLiquidBuilder;
    use ccxt_exchanges::okx::OkxBuilder;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Test Binance WebSocket URL selection consistency
        #[test]
        fn prop_binance_ws_url_consistency(sandbox in any::<bool>()) {
            let binance = BinanceBuilder::new()
                .sandbox(sandbox)
                .build()
                .expect("Should build Binance instance");

            let urls = binance.urls();

            if sandbox {
                // Testnet WebSocket URLs
                prop_assert!(
                    urls.ws.contains("testnet.binance.vision"),
                    "Binance WS URL should be testnet when sandbox=true, got: {}",
                    urls.ws
                );
            } else {
                // Production WebSocket URLs
                prop_assert!(
                    urls.ws.contains("stream.binance.com"),
                    "Binance WS URL should be production when sandbox=false, got: {}",
                    urls.ws
                );
            }
        }

        /// Test OKX WebSocket URL selection consistency
        /// OKX uses different WebSocket URLs for demo mode
        #[test]
        fn prop_okx_ws_url_consistency(sandbox in any::<bool>()) {
            let okx = OkxBuilder::new()
                .sandbox(sandbox)
                .build()
                .expect("Should build OKX instance");

            let urls = okx.urls();

            if sandbox {
                // Demo WebSocket URLs should contain demo endpoint
                prop_assert!(
                    urls.ws_public.contains("wspap.okx.com") || urls.ws_public.contains("brokerId=9999"),
                    "OKX public WS URL should be demo when sandbox=true, got: {}",
                    urls.ws_public
                );
                prop_assert!(
                    urls.ws_private.contains("wspap.okx.com") || urls.ws_private.contains("brokerId=9999"),
                    "OKX private WS URL should be demo when sandbox=true, got: {}",
                    urls.ws_private
                );
            } else {
                // Production WebSocket URLs
                prop_assert!(
                    urls.ws_public.contains("ws.okx.com"),
                    "OKX public WS URL should be production when sandbox=false, got: {}",
                    urls.ws_public
                );
                prop_assert!(
                    urls.ws_private.contains("ws.okx.com"),
                    "OKX private WS URL should be production when sandbox=false, got: {}",
                    urls.ws_private
                );
            }
        }

        /// Test Bybit WebSocket URL selection consistency
        #[test]
        fn prop_bybit_ws_url_consistency(sandbox in any::<bool>()) {
            let bybit = BybitBuilder::new()
                .sandbox(sandbox)
                .build()
                .expect("Should build Bybit instance");

            let urls = bybit.urls();

            if sandbox {
                // Testnet WebSocket URLs
                prop_assert!(
                    urls.ws_public.contains("stream-testnet.bybit.com"),
                    "Bybit public WS URL should be testnet when sandbox=true, got: {}",
                    urls.ws_public
                );
                prop_assert!(
                    urls.ws_private.contains("stream-testnet.bybit.com"),
                    "Bybit private WS URL should be testnet when sandbox=true, got: {}",
                    urls.ws_private
                );
            } else {
                // Production WebSocket URLs
                prop_assert!(
                    urls.ws_public.contains("stream.bybit.com") && !urls.ws_public.contains("testnet"),
                    "Bybit public WS URL should be production when sandbox=false, got: {}",
                    urls.ws_public
                );
                prop_assert!(
                    urls.ws_private.contains("stream.bybit.com") && !urls.ws_private.contains("testnet"),
                    "Bybit private WS URL should be production when sandbox=false, got: {}",
                    urls.ws_private
                );
            }
        }

        /// Test Bitget WebSocket URL selection consistency
        #[test]
        fn prop_bitget_ws_url_consistency(sandbox in any::<bool>()) {
            let bitget = BitgetBuilder::new()
                .sandbox(sandbox)
                .build()
                .expect("Should build Bitget instance");

            let urls = bitget.urls();

            if sandbox {
                // Testnet WebSocket URLs
                prop_assert!(
                    urls.ws_public.contains("ws-testnet.bitget.com"),
                    "Bitget public WS URL should be testnet when sandbox=true, got: {}",
                    urls.ws_public
                );
                prop_assert!(
                    urls.ws_private.contains("ws-testnet.bitget.com"),
                    "Bitget private WS URL should be testnet when sandbox=true, got: {}",
                    urls.ws_private
                );
            } else {
                // Production WebSocket URLs
                prop_assert!(
                    urls.ws_public.contains("ws.bitget.com") && !urls.ws_public.contains("testnet"),
                    "Bitget public WS URL should be production when sandbox=false, got: {}",
                    urls.ws_public
                );
                prop_assert!(
                    urls.ws_private.contains("ws.bitget.com") && !urls.ws_private.contains("testnet"),
                    "Bitget private WS URL should be production when sandbox=false, got: {}",
                    urls.ws_private
                );
            }
        }

        /// Test Hyperliquid WebSocket URL selection consistency
        #[test]
        fn prop_hyperliquid_ws_url_consistency(sandbox in any::<bool>()) {
            let hyperliquid = HyperLiquidBuilder::new()
                .sandbox(sandbox)
                .build()
                .expect("Should build Hyperliquid instance");

            let urls = hyperliquid.urls();

            if sandbox {
                // Testnet WebSocket URLs
                prop_assert!(
                    urls.ws.contains("hyperliquid-testnet.xyz"),
                    "Hyperliquid WS URL should be testnet when sandbox=true, got: {}",
                    urls.ws
                );
            } else {
                // Production WebSocket URLs
                prop_assert!(
                    urls.ws.contains("api.hyperliquid.xyz") && !urls.ws.contains("testnet"),
                    "Hyperliquid WS URL should be production when sandbox=false, got: {}",
                    urls.ws
                );
            }
        }

        /// Test that all exchanges have consistent WebSocket URL selection
        #[test]
        fn prop_all_exchanges_ws_consistency(sandbox in any::<bool>()) {
            // Build all exchanges with the same sandbox setting
            let binance = BinanceBuilder::new().sandbox(sandbox).build().expect("Binance");
            let okx = OkxBuilder::new().sandbox(sandbox).build().expect("OKX");
            let bybit = BybitBuilder::new().sandbox(sandbox).build().expect("Bybit");
            let bitget = BitgetBuilder::new().sandbox(sandbox).build().expect("Bitget");
            let hyperliquid = HyperLiquidBuilder::new().sandbox(sandbox).build().expect("Hyperliquid");

            let binance_urls = binance.urls();
            let okx_urls = okx.urls();
            let bybit_urls = bybit.urls();
            let bitget_urls = bitget.urls();
            let hyperliquid_urls = hyperliquid.urls();

            if sandbox {
                // All testnet WebSocket URLs should contain testnet indicators
                prop_assert!(binance_urls.ws.contains("testnet"), "Binance WS testnet");
                // OKX uses different demo WS endpoint
                prop_assert!(
                    okx_urls.ws_public.contains("wspap") || okx_urls.ws_public.contains("brokerId"),
                    "OKX WS demo"
                );
                prop_assert!(bybit_urls.ws_public.contains("testnet"), "Bybit WS testnet");
                prop_assert!(bitget_urls.ws_public.contains("testnet"), "Bitget WS testnet");
                prop_assert!(hyperliquid_urls.ws.contains("testnet"), "Hyperliquid WS testnet");
            } else {
                // All production WebSocket URLs should NOT contain testnet
                prop_assert!(!binance_urls.ws.contains("testnet"), "Binance WS production");
                prop_assert!(!okx_urls.ws_public.contains("testnet"), "OKX WS production");
                prop_assert!(!bybit_urls.ws_public.contains("testnet"), "Bybit WS production");
                prop_assert!(!bitget_urls.ws_public.contains("testnet"), "Bitget WS production");
                prop_assert!(!hyperliquid_urls.ws.contains("testnet"), "Hyperliquid WS production");
            }
        }
    }
}
