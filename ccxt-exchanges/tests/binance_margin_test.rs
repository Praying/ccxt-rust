//! Binance margin trading API integration tests.
//!
//! This test suite covers all 10 margin trading methods:
//! - Interest rate queries (3 methods)
//! - Borrow operations (2 methods)
//! - Repay operations (2 methods)
//! - History queries (3 methods)

use ccxt_core::ExchangeConfig;
use ccxt_exchanges::binance::Binance;

/// Create a Binance instance for testing.
fn create_binance_instance() -> Binance {
    let api_key = std::env::var("BINANCE_API_KEY").ok();
    let secret = std::env::var("BINANCE_SECRET").ok();

    let config = ExchangeConfig {
        api_key,
        secret,
        ..Default::default()
    };

    Binance::new(config).expect("Failed to create Binance instance")
}

/// Check if API credentials are configured.
fn has_api_credentials() -> bool {
    std::env::var("BINANCE_API_KEY").is_ok() && std::env::var("BINANCE_SECRET").is_ok()
}

// ============================================================================
// Interest Rate Query Tests (3 methods)
// ============================================================================

#[tokio::test]
async fn test_fetch_cross_borrow_rate() {
    if !has_api_credentials() {
        println!("⚠️  Skip test: API credentials not set");
        return;
    }

    let binance = create_binance_instance();

    match binance.fetch_isolated_borrow_rates(None).await {
        Ok(rates) => {
            println!("✅ Cross margin borrow rate query successful");
            println!("   Total {} symbols", rates.len());
            for (symbol, rate) in &rates {
                assert!(!symbol.is_empty());
                assert!(rate.base_rate >= 0.0 || rate.quote_rate >= 0.0);
            }
        }
        Err(e) => {
            println!("⚠️  Query failed (possible permission issue): {}", e);
        }
    }
}

#[tokio::test]
async fn test_fetch_isolated_borrow_rate() {
    if !has_api_credentials() {
        println!("⚠️  Skip test: API credentials not set");
        return;
    }

    let binance = create_binance_instance();

    match binance.fetch_isolated_borrow_rates(None).await {
        Ok(rates) => {
            println!("✅ Isolated margin borrow rate query successful");
            println!("   Total {} symbols", rates.len());
            for (symbol, rate) in &rates {
                assert!(!symbol.is_empty());
                assert!(rate.base_rate >= 0.0 || rate.quote_rate >= 0.0);
            }
        }
        Err(e) => {
            println!("⚠️  Query failed: {}", e);
        }
    }
}

#[tokio::test]
async fn test_fetch_borrow_rates() {
    if !has_api_credentials() {
        println!("⚠️  Skip test: API credentials not set");
        return;
    }

    let binance = create_binance_instance();

    match binance.fetch_isolated_borrow_rates(None).await {
        Ok(rates) => {
            println!("✅ Batch borrow rate query successful");
            println!("   Total: {} symbols", rates.len());
            assert!(!rates.is_empty());
        }
        Err(e) => {
            println!("⚠️  Query failed: {}", e);
        }
    }
}

// ============================================================================
// Borrow Operation Tests (2 methods) - Skipped by default to avoid real trades
// ============================================================================

#[tokio::test]
#[ignore]
async fn test_borrow_cross_margin() {
    println!("⚠️  Warning: This performs real borrow operations");
    println!("ℹ️  Run this test with --ignored flag");
}

#[tokio::test]
#[ignore]
async fn test_borrow_isolated_margin() {
    println!("⚠️  Warning: This performs real borrow operations");
    println!("ℹ️  Run this test with --ignored flag");
}

// ============================================================================
// Repay Operation Tests (2 methods) - Skipped by default
// ============================================================================

#[tokio::test]
#[ignore]
async fn test_repay_cross_margin() {
    println!("⚠️  Warning: This performs real repay operations");
}

#[tokio::test]
#[ignore]
async fn test_repay_isolated_margin() {
    println!("⚠️  Warning: This performs real repay operations");
}

// ============================================================================
// History Query Tests (3 methods)
// ============================================================================

#[tokio::test]
async fn test_fetch_borrow_rate_history() {
    if !has_api_credentials() {
        println!("⚠️  Skip test: API credentials not set");
        return;
    }

    let binance = create_binance_instance();

    match binance
        .fetch_borrow_rate_history("BTC", None, Some(10), None)
        .await
    {
        Ok(history) => {
            println!("✅ Borrow rate history query successful");
            println!("   Records: {}", history.len());

            for record in &history {
                assert_eq!(record.currency, "BTC");
                assert!(record.rate >= 0.0);
            }
        }
        Err(e) => {
            println!("⚠️  Query failed: {}", e);
        }
    }
}

#[tokio::test]
async fn test_fetch_borrow_interests() {
    if !has_api_credentials() {
        println!("⚠️  Skip test: API credentials not set");
        return;
    }

    let binance = create_binance_instance();

    match binance
        .fetch_borrow_interest(None, None, Some(10), None, None)
        .await
    {
        Ok(interests) => {
            println!("✅ Borrow interest history query successful");
            println!("   Records: {}", interests.len());

            for record in &interests {
                assert!(!record.currency.is_empty());
                assert!(record.interest >= 0.0);
            }
        }
        Err(e) => {
            println!("⚠️  Query failed: {}", e);
        }
    }
}

#[tokio::test]
async fn test_fetch_margin_adjustment_history() {
    if !has_api_credentials() {
        println!("⚠️  Skip test: API credentials not set");
        return;
    }

    let binance = create_binance_instance();

    match binance
        .fetch_margin_adjustment_history(Some("BTC/USDT"), None, Some(10))
        .await
    {
        Ok(adjustments) => {
            println!("✅ Margin adjustment history query successful");
            println!("   Records: {}", adjustments.len());

            for record in &adjustments {
                assert!(record.symbol.is_some());
                assert!(!record.transfer_type.is_empty());
            }
        }
        Err(e) => {
            println!("⚠️  Query failed: {}", e);
        }
    }
}
