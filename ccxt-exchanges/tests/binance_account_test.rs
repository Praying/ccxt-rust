//! Binance account management integration tests.
//!
//! This test suite covers the following account management features:
//! 1. Balance queries for multiple account types (8 account types)
//! 2. Internal account transfers
//! 3. Transfer history queries
//! 4. Maximum borrowable amount queries (cross/isolated margin)
//! 5. Maximum transferable amount queries
//!
//! Note: Some methods (transfer, fetch_transfers, fetch_cross_margin_max_borrowable,
//! fetch_isolated_margin_max_borrowable, fetch_max_transferable) are being migrated
//! to the new modular REST API structure.

use ccxt_core::ExchangeConfig;
use ccxt_core::error::Result;
use ccxt_core::types::AccountType;
use ccxt_exchanges::binance::Binance;

/// Create Binance client for testing.
///
/// Supports both production and testnet environments based on environment variables:
/// - `BINANCE_USE_TESTNET=true`: Uses testnet API keys and URLs
/// - `BINANCE_USE_TESTNET=false` or unset: Uses production API keys and URLs
///
/// Note: If testnet credentials are not set, falls back to production credentials.
fn create_binance_client() -> Binance {
    dotenvy::dotenv().ok();

    // Check if testnet mode is enabled
    let use_testnet = std::env::var("BINANCE_USE_TESTNET")
        .map(|v| v.to_lowercase() == "true")
        .unwrap_or(false);

    let (api_key, secret, sandbox) = if use_testnet {
        // Try testnet credentials first
        let testnet_key = std::env::var("BINANCE_TESTNET_API_KEY").unwrap_or_default();
        let testnet_secret = std::env::var("BINANCE_TESTNET_API_SECRET").unwrap_or_default();

        // If testnet credentials are empty, fall back to production
        if testnet_key.is_empty() || testnet_secret.is_empty() {
            eprintln!(
                "Warning: BINANCE_USE_TESTNET=true but testnet credentials not set, using production"
            );
            (
                std::env::var("BINANCE_API_KEY").unwrap_or_default(),
                std::env::var("BINANCE_API_SECRET").unwrap_or_default(),
                false,
            )
        } else {
            (testnet_key, testnet_secret, true)
        }
    } else {
        // Use production credentials
        (
            std::env::var("BINANCE_API_KEY").unwrap_or_default(),
            std::env::var("BINANCE_API_SECRET").unwrap_or_default(),
            false,
        )
    };

    let mut config = ExchangeConfig::default();
    config.api_key = Some(api_key);
    config.secret = Some(secret);
    config.sandbox = sandbox;

    Binance::new(config).unwrap()
}

#[tokio::test]
#[ignore] // Requires API credentials
async fn test_fetch_spot_balance() -> Result<()> {
    let binance = create_binance_client();

    let balance = binance.fetch_balance(Some(AccountType::Spot)).await?;

    assert!(
        !balance.balances.is_empty(),
        "Balance data should not be empty"
    );

    println!(
        "Spot balance query successful, currencies: {}",
        balance.balances.len()
    );

    if let Some(btc) = balance.balances.get("BTC") {
        assert!(btc.total >= btc.used + btc.free);
        println!(
            "BTC balance - total: {}, free: {}, used: {}",
            btc.total, btc.free, btc.used
        );
    }

    Ok(())
}

#[tokio::test]
#[ignore]
async fn test_fetch_margin_balance() -> Result<()> {
    let binance = create_binance_client();

    let balance = binance.fetch_balance(Some(AccountType::Margin)).await?;

    assert!(
        !balance.balances.is_empty(),
        "Margin balance should not be empty"
    );
    println!(
        "Margin account balance query successful, currencies: {}",
        balance.balances.len()
    );

    Ok(())
}

#[tokio::test]
#[ignore]
async fn test_fetch_isolated_margin_balance() -> Result<()> {
    let binance = create_binance_client();

    let balance = binance
        .fetch_balance(Some(AccountType::IsolatedMargin))
        .await?;

    assert!(
        !balance.balances.is_empty(),
        "Isolated margin balance should not be empty"
    );
    println!("Isolated margin account balance query successful");

    Ok(())
}

#[tokio::test]
#[ignore]
async fn test_fetch_future_balance() -> Result<()> {
    let binance = create_binance_client();

    let balance = binance.fetch_balance(Some(AccountType::Futures)).await?;

    assert!(
        !balance.balances.is_empty(),
        "Futures balance should not be empty"
    );
    println!("USDT-margined futures account balance query successful");

    Ok(())
}

#[tokio::test]
#[ignore]
async fn test_fetch_funding_balance() -> Result<()> {
    let binance = create_binance_client();

    let balance = binance.fetch_balance(Some(AccountType::Funding)).await?;

    assert!(
        !balance.balances.is_empty(),
        "Funding account balance should not be empty"
    );
    println!(
        "Funding account balance query successful, currencies: {}",
        balance.balances.len()
    );

    Ok(())
}

#[tokio::test]
#[ignore = "transfer method not yet migrated to new modular structure"]
async fn test_internal_transfer() {
    let _binance = create_binance_client();
    // TODO: Implement when transfer is available
}

#[tokio::test]
#[ignore = "fetch_transfers method not yet migrated to new modular structure"]
async fn test_fetch_transfers() {
    let _binance = create_binance_client();
    // TODO: Implement when fetch_transfers is available
}

#[tokio::test]
#[ignore = "fetch_cross_margin_max_borrowable method not yet migrated to new modular structure"]
async fn test_fetch_cross_margin_max_borrowable() {
    let _binance = create_binance_client();
    // TODO: Implement when fetch_cross_margin_max_borrowable is available
}

#[tokio::test]
#[ignore = "fetch_isolated_margin_max_borrowable method not yet migrated to new modular structure"]
async fn test_fetch_isolated_margin_max_borrowable() {
    let _binance = create_binance_client();
    // TODO: Implement when fetch_isolated_margin_max_borrowable is available
}

#[tokio::test]
#[ignore = "fetch_max_transferable method not yet migrated to new modular structure"]
async fn test_fetch_max_transferable() {
    let _binance = create_binance_client();
    // TODO: Implement when fetch_max_transferable is available
}

#[tokio::test]
#[ignore]
async fn test_multiple_account_types() -> Result<()> {
    let binance = create_binance_client();

    let account_types = vec![
        AccountType::Spot,
        AccountType::Margin,
        AccountType::Futures,
        AccountType::Funding,
    ];

    for account_type in account_types {
        let balance = binance.fetch_balance(Some(account_type)).await?;

        println!(
            "{} account balance query successful, currencies: {}",
            account_type,
            balance.balances.len()
        );
    }

    Ok(())
}

#[cfg(test)]
mod integration_tests {
    use super::*;

    /// Test complete account management workflow.
    #[tokio::test]
    #[ignore = "Account management methods not yet fully migrated to new modular structure"]
    async fn test_complete_account_workflow() {
        let _binance = create_binance_client();
        // TODO: Implement when all account management methods are available
    }
}
