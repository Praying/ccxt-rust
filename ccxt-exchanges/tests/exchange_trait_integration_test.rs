//! Exchange Trait Integration Tests
//!
//! These tests verify the unified Exchange trait implementation and trait object usage.
//! They test:
//! - Creating `Box<dyn Exchange>` from Binance (Task 17.1)
//! - Calling methods through trait objects (Task 17.1)
//! - Using multiple exchanges through unified interface (Task 17.2)
//!
//! Run with: cargo test --test exchange_trait_integration_test

use ccxt_core::ExchangeConfig;
use ccxt_core::exchange::{ArcExchange, BoxedExchange, Exchange};
use ccxt_exchanges::binance::{Binance, BinanceBuilder};
use std::sync::Arc;

// ============================================================================
// Task 17.1: Trait Object Usage Tests
// ============================================================================

/// Test creating a `Box<dyn Exchange>` from Binance instance.
///
/// Requirements: 1.2 - Allow instance to be used as `dyn Exchange` trait object
#[test]
fn test_binance_as_boxed_exchange() {
    let config = ExchangeConfig::default();
    let binance = Binance::new(config).expect("Should create Binance instance");

    // Create a boxed trait object
    let exchange: BoxedExchange = Box::new(binance);

    // Verify metadata methods work through trait object
    assert_eq!(exchange.id(), "binance");
    assert_eq!(exchange.name(), "Binance");
    assert_eq!(exchange.version(), "v3");
    assert!(exchange.certified());
    assert!(exchange.has_websocket());
}

/// Test creating an `Arc<dyn Exchange>` from Binance for shared ownership.
///
/// Requirements: 1.2 - Allow instance to be used as `dyn Exchange` trait object
#[test]
fn test_binance_as_arc_exchange() {
    let binance = BinanceBuilder::new()
        .build()
        .expect("Should create Binance instance");

    // Create an Arc-wrapped trait object
    let exchange: ArcExchange = Arc::new(binance);

    // Verify metadata methods work through Arc trait object
    assert_eq!(exchange.id(), "binance");
    assert_eq!(exchange.name(), "Binance");
    assert!(exchange.certified());

    // Test cloning Arc for shared ownership
    let exchange_clone = Arc::clone(&exchange);
    assert_eq!(exchange_clone.id(), "binance");
    assert_eq!(Arc::strong_count(&exchange), 2);
}

/// Test calling metadata methods through trait object.
///
/// Requirements: 1.3 - Execute exchange-specific implementation transparently
#[test]
fn test_trait_object_metadata_methods() {
    let binance = Binance::new(ExchangeConfig::default()).expect("Should create Binance instance");
    let exchange: &dyn Exchange = &binance;

    // Test all metadata methods
    assert_eq!(exchange.id(), "binance");
    assert_eq!(exchange.name(), "Binance");
    assert_eq!(exchange.version(), "v3");
    assert!(exchange.certified());
    assert!(exchange.has_websocket());
    assert_eq!(exchange.rate_limit(), 50.0);

    // Test timeframes
    let timeframes = exchange.timeframes();
    assert!(!timeframes.is_empty());
    assert!(timeframes.len() >= 10); // Binance supports many timeframes
}

/// Test calling capabilities() through trait object.
///
/// Requirements: 1.3 - Execute exchange-specific implementation transparently
#[test]
fn test_trait_object_capabilities() {
    let binance = Binance::new(ExchangeConfig::default()).expect("Should create Binance instance");
    let exchange: &dyn Exchange = &binance;

    let caps = exchange.capabilities();

    // Verify Binance capabilities
    assert!(caps.fetch_markets);
    assert!(caps.fetch_ticker);
    assert!(caps.fetch_tickers);
    assert!(caps.fetch_order_book);
    assert!(caps.fetch_trades);
    assert!(caps.fetch_ohlcv);
    assert!(caps.create_order);
    assert!(caps.cancel_order);
    assert!(caps.fetch_balance);
    assert!(caps.websocket);

    // Binance doesn't support order editing
    assert!(!caps.edit_order);
}

/// Test that trait object can be passed to generic functions.
///
/// Requirements: 1.2, 1.3 - Unified interface usage
#[test]
fn test_trait_object_in_generic_function() {
    fn get_exchange_info(exchange: &dyn Exchange) -> String {
        format!(
            "{} ({}) v{} - certified: {}",
            exchange.name(),
            exchange.id(),
            exchange.version(),
            exchange.certified()
        )
    }

    let binance = Binance::new(ExchangeConfig::default()).expect("Should create Binance instance");
    let info = get_exchange_info(&binance);

    assert!(info.contains("Binance"));
    assert!(info.contains("binance"));
    assert!(info.contains("v3"));
    assert!(info.contains("certified: true"));
}

/// Test that boxed trait object can be stored in collections.
///
/// Requirements: 1.2 - Trait object usage
#[test]
fn test_boxed_exchange_in_collection() {
    let binance = Binance::new(ExchangeConfig::default()).expect("Should create Binance instance");

    // Store in a Vec of boxed trait objects
    let exchanges: Vec<BoxedExchange> = vec![Box::new(binance)];

    assert_eq!(exchanges.len(), 1);
    assert_eq!(exchanges[0].id(), "binance");
}

/// Test that Arc trait object can be shared across threads.
///
/// Requirements: 1.2 - Trait object usage with thread safety
#[test]
fn test_arc_exchange_thread_safety() {
    let binance = Binance::new(ExchangeConfig::default()).expect("Should create Binance instance");
    let exchange: ArcExchange = Arc::new(binance);

    // Spawn multiple threads that access the exchange
    let handles: Vec<_> = (0..4)
        .map(|_| {
            let ex = Arc::clone(&exchange);
            std::thread::spawn(move || {
                // Access exchange from different threads
                (ex.id().to_string(), ex.name().to_string(), ex.certified())
            })
        })
        .collect();

    // Verify all threads completed successfully
    for handle in handles {
        let (id, name, certified) = handle.join().expect("Thread should not panic");
        assert_eq!(id, "binance");
        assert_eq!(name, "Binance");
        assert!(certified);
    }
}

/// Test helper method `is_symbol_active` through trait object.
///
/// Requirements: 1.3 - Execute exchange-specific implementation transparently
#[tokio::test]
async fn test_trait_object_helper_methods() {
    let binance = Binance::new(ExchangeConfig::default()).expect("Should create Binance instance");
    let exchange: &dyn Exchange = &binance;

    // Without loading markets, is_symbol_active should return false
    assert!(!exchange.is_symbol_active("BTC/USDT").await);

    // markets() should return empty HashMap without loading
    let markets = exchange.markets().await;
    assert!(markets.is_empty());
}

/// Test that capabilities can be checked before calling methods.
///
/// Requirements: 1.3 - Execute exchange-specific implementation transparently
#[test]
fn test_capability_check_before_method_call() {
    fn check_and_describe_capabilities(exchange: &dyn Exchange) -> Vec<String> {
        let caps = exchange.capabilities();
        let mut supported = Vec::new();

        if caps.fetch_ticker {
            supported.push("fetch_ticker".to_string());
        }
        if caps.create_order {
            supported.push("create_order".to_string());
        }
        if caps.websocket {
            supported.push("websocket".to_string());
        }
        if caps.edit_order {
            supported.push("edit_order".to_string());
        }

        supported
    }

    let binance = Binance::new(ExchangeConfig::default()).expect("Should create Binance instance");
    let supported = check_and_describe_capabilities(&binance);

    assert!(supported.contains(&"fetch_ticker".to_string()));
    assert!(supported.contains(&"create_order".to_string()));
    assert!(supported.contains(&"websocket".to_string()));
    assert!(!supported.contains(&"edit_order".to_string())); // Binance doesn't support this
}

// ============================================================================
// Task 17.2: Multi-Exchange Scenario Tests
// ============================================================================

/// Test using multiple exchanges through unified interface.
///
/// Requirements: 1.1 - Use different exchanges through unified interface
#[test]
fn test_multiple_exchanges_unified_interface() {
    // Create multiple Binance instances with different configurations
    // (In a real scenario, these would be different exchanges like Coinbase, Kraken, etc.)
    let binance_spot = BinanceBuilder::new()
        .default_type("spot")
        .build()
        .expect("Should create spot Binance");

    let binance_futures = BinanceBuilder::new()
        .default_type("future")
        .build()
        .expect("Should create futures Binance");

    // Store them as trait objects
    let exchanges: Vec<BoxedExchange> = vec![Box::new(binance_spot), Box::new(binance_futures)];

    // Iterate and use unified interface
    for exchange in &exchanges {
        assert_eq!(exchange.id(), "binance");
        assert!(exchange.capabilities().fetch_ticker);
        assert!(exchange.capabilities().websocket);
    }

    assert_eq!(exchanges.len(), 2);
}

/// Test exchange-agnostic function that works with any Exchange implementation.
///
/// Requirements: 1.1 - Write exchange-agnostic trading code
#[test]
fn test_exchange_agnostic_function() {
    /// An exchange-agnostic function that summarizes exchange capabilities
    fn summarize_exchange(exchange: &dyn Exchange) -> ExchangeSummary {
        let caps = exchange.capabilities();
        ExchangeSummary {
            id: exchange.id().to_string(),
            name: exchange.name().to_string(),
            version: exchange.version().to_string(),
            certified: exchange.certified(),
            has_websocket: exchange.has_websocket(),
            can_trade: caps.create_order && caps.cancel_order,
            can_fetch_market_data: caps.fetch_ticker && caps.fetch_order_book,
            timeframe_count: exchange.timeframes().len(),
        }
    }

    #[derive(Debug)]
    struct ExchangeSummary {
        id: String,
        name: String,
        version: String,
        certified: bool,
        has_websocket: bool,
        can_trade: bool,
        can_fetch_market_data: bool,
        timeframe_count: usize,
    }

    let binance = Binance::new(ExchangeConfig::default()).expect("Should create Binance instance");
    let summary = summarize_exchange(&binance);

    assert_eq!(summary.id, "binance");
    assert_eq!(summary.name, "Binance");
    assert_eq!(summary.version, "v3");
    assert!(summary.certified);
    assert!(summary.has_websocket);
    assert!(summary.can_trade);
    assert!(summary.can_fetch_market_data);
    assert!(summary.timeframe_count > 0);
}

/// Test managing multiple exchanges in a registry pattern.
///
/// Requirements: 1.1 - Use different exchanges through unified interface
#[test]
fn test_exchange_registry_pattern() {
    use std::collections::HashMap;

    struct ExchangeRegistry {
        exchanges: HashMap<String, BoxedExchange>,
    }

    impl ExchangeRegistry {
        fn new() -> Self {
            Self {
                exchanges: HashMap::new(),
            }
        }

        fn register(&mut self, exchange: BoxedExchange) {
            let id = exchange.id().to_string();
            self.exchanges.insert(id, exchange);
        }

        fn get(&self, id: &str) -> Option<&dyn Exchange> {
            self.exchanges.get(id).map(|e| e.as_ref())
        }

        fn list_ids(&self) -> Vec<&str> {
            self.exchanges.keys().map(|s| s.as_str()).collect()
        }

        fn count(&self) -> usize {
            self.exchanges.len()
        }
    }

    let mut registry = ExchangeRegistry::new();

    // Register exchanges
    let binance = Binance::new(ExchangeConfig::default()).expect("Should create Binance");
    registry.register(Box::new(binance));

    // Verify registry operations
    assert_eq!(registry.count(), 1);
    assert!(registry.list_ids().contains(&"binance"));

    // Get and use exchange through registry
    let exchange = registry.get("binance").expect("Should find binance");
    assert_eq!(exchange.name(), "Binance");
    assert!(exchange.certified());
}

/// Test comparing capabilities across multiple exchanges.
///
/// Requirements: 1.1 - Use different exchanges through unified interface
#[test]
fn test_compare_exchange_capabilities() {
    fn compare_capabilities(exchanges: &[&dyn Exchange]) -> CapabilityComparison {
        let mut comparison = CapabilityComparison {
            all_support_ticker: true,
            all_support_websocket: true,
            all_certified: true,
            exchange_count: exchanges.len(),
        };

        for exchange in exchanges {
            let caps = exchange.capabilities();
            comparison.all_support_ticker &= caps.fetch_ticker;
            comparison.all_support_websocket &= caps.websocket;
            comparison.all_certified &= exchange.certified();
        }

        comparison
    }

    #[derive(Debug)]
    struct CapabilityComparison {
        all_support_ticker: bool,
        all_support_websocket: bool,
        all_certified: bool,
        exchange_count: usize,
    }

    let binance1 = Binance::new(ExchangeConfig::default()).expect("Should create Binance 1");
    let binance2 = Binance::new(ExchangeConfig::default()).expect("Should create Binance 2");

    let exchanges: Vec<&dyn Exchange> = vec![&binance1, &binance2];
    let comparison = compare_capabilities(&exchanges);

    assert!(comparison.all_support_ticker);
    assert!(comparison.all_support_websocket);
    assert!(comparison.all_certified);
    assert_eq!(comparison.exchange_count, 2);
}

/// Test async operations through trait object (compile-time verification).
///
/// Requirements: 1.3 - Execute exchange-specific implementation transparently
#[tokio::test]
async fn test_async_trait_object_compilation() {
    // This test verifies that async methods can be called through trait objects
    // We don't actually call the API, just verify the code compiles correctly

    async fn _fetch_ticker_if_supported(
        exchange: &dyn Exchange,
        symbol: &str,
    ) -> Option<ccxt_core::Result<ccxt_core::Ticker>> {
        if exchange.capabilities().fetch_ticker {
            Some(exchange.fetch_ticker(symbol).await)
        } else {
            None
        }
    }

    let binance = Binance::new(ExchangeConfig::default()).expect("Should create Binance");

    // Verify the function compiles and can be called
    // We don't actually make the API call in this test
    let _exchange: &dyn Exchange = &binance;

    // Just verify the async function signature is correct
    assert!(binance.capabilities().fetch_ticker);

    // The actual API call would be:
    // let result = fetch_ticker_if_supported(&binance, "BTC/USDT").await;
}

// ============================================================================
// Additional Edge Case Tests
// ============================================================================

/// Test that trait object works with different builder configurations.
#[test]
fn test_trait_object_with_various_configs() {
    // Test with sandbox mode
    let sandbox_binance = BinanceBuilder::new()
        .sandbox(true)
        .build()
        .expect("Should create sandbox Binance");

    let exchange: &dyn Exchange = &sandbox_binance;
    assert_eq!(exchange.id(), "binance");

    // Test with custom timeout
    let custom_timeout_binance = BinanceBuilder::new()
        .timeout(60)
        .build()
        .expect("Should create Binance with custom timeout");

    let exchange: &dyn Exchange = &custom_timeout_binance;
    assert_eq!(exchange.id(), "binance");

    // Test with API credentials (won't actually authenticate, just config)
    let auth_binance = BinanceBuilder::new()
        .api_key("test-key")
        .secret("test-secret")
        .build()
        .expect("Should create Binance with credentials");

    let exchange: &dyn Exchange = &auth_binance;
    assert_eq!(exchange.id(), "binance");
    assert!(exchange.capabilities().fetch_balance); // Capability exists even without valid auth
}

/// Test that ExchangeCapabilities methods work correctly.
#[test]
fn test_exchange_capabilities_methods() {
    let binance = Binance::new(ExchangeConfig::default()).expect("Should create Binance");
    let caps = binance.capabilities();

    // Test has() method with various capability names
    assert!(caps.has("fetchTicker"));
    assert!(caps.has("fetchOrderBook"));
    assert!(caps.has("createOrder"));
    assert!(caps.has("websocket"));
    assert!(!caps.has("editOrder")); // Binance doesn't support this
    assert!(!caps.has("unknownCapability"));

    // Test supported_capabilities() method
    let supported = caps.supported_capabilities();
    assert!(supported.contains(&"fetchTicker"));
    assert!(supported.contains(&"createOrder"));
    assert!(supported.contains(&"websocket"));
}
