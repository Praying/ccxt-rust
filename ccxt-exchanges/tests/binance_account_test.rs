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
use ccxt_exchanges::binance::Binance;

/// Create Binance client for testing.
fn create_binance_client() -> Binance {
    let api_key = std::env::var("BINANCE_API_KEY").unwrap_or_default();
    let secret = std::env::var("BINANCE_SECRET").unwrap_or_default();
    let mut config = ExchangeConfig::default();
    config.api_key = Some(api_key);
    config.secret = Some(secret);
    Binance::new(config).unwrap()
}

#[tokio::test]
#[ignore] // Requires API credentials
async fn test_fetch_spot_balance() -> Result<()> {
    let binance = create_binance_client();

    // Note: The current implementation uses account_type as a string parameter
    let balance = binance.fetch_balance(Some("spot")).await?;

    assert!(
        !balance.balances.is_empty(),
        "Balance data should not be empty"
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

    // Note: The current implementation uses account_type as a string parameter
    let balance = binance.fetch_balance(Some("margin")).await?;

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

    // Note: The current implementation uses account_type as a string parameter
    // Isolated margin with symbol is not yet supported in the simplified API
    let balance = binance.fetch_balance(Some("isolated")).await?;

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

    // Note: The current implementation uses account_type as a string parameter
    let balance = binance.fetch_balance(Some("future")).await?;

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

    // Note: The current implementation uses account_type as a string parameter
    let balance = binance.fetch_balance(Some("funding")).await?;

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

    let account_types = vec!["spot", "margin", "future", "funding"];

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
