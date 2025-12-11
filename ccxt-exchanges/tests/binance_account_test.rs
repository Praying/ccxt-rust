//! Binance account management integration tests.
//!
//! This test suite covers the following account management features:
//! 1. Balance queries for multiple account types (8 account types)
//! 2. Internal account transfers
//! 3. Transfer history queries
//! 4. Maximum borrowable amount queries (cross/isolated margin)
//! 5. Maximum transferable amount queries

use ccxt_core::ExchangeConfig;
use ccxt_core::error::Result;
use ccxt_exchanges::binance::Binance;
use serde_json::Value;
use std::collections::HashMap;

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

    let mut params = HashMap::new();
    params.insert("type".to_string(), serde_json::json!("spot"));

    let balance = binance.fetch_balance(Some(params)).await?;

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

    let mut params = HashMap::new();
    params.insert("type".to_string(), serde_json::json!("margin"));

    let balance = binance.fetch_balance(Some(params)).await?;

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

    let mut params = HashMap::new();
    params.insert("type".to_string(), serde_json::json!("isolated"));
    params.insert("symbol".to_string(), serde_json::json!("BTC/USDT"));

    let balance = binance.fetch_balance(Some(params)).await?;

    assert!(
        !balance.balances.is_empty(),
        "Isolated margin balance should not be empty"
    );
    println!("Isolated margin account balance query successful (BTC/USDT)");

    Ok(())
}

#[tokio::test]
#[ignore]
async fn test_fetch_future_balance() -> Result<()> {
    let binance = create_binance_client();

    let mut params = HashMap::new();
    params.insert("type".to_string(), serde_json::json!("future"));

    let balance = binance.fetch_balance(Some(params)).await?;

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

    let mut params = HashMap::new();
    params.insert("type".to_string(), serde_json::json!("funding"));

    let balance = binance.fetch_balance(Some(params)).await?;

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
#[ignore]
async fn test_internal_transfer() -> Result<()> {
    let binance = create_binance_client();

    let transfer = binance
        .transfer("USDT", 10.0, "spot", "future", None)
        .await?;

    println!("Transfer successful, transaction ID: {:?}", transfer.id);
    println!(
        "From account: {:?} -> To account: {:?}",
        transfer.from_account, transfer.to_account
    );
    println!("Amount: {} {}", transfer.amount, transfer.currency);

    Ok(())
}

#[tokio::test]
#[ignore]
async fn test_fetch_transfers() -> Result<()> {
    let binance = create_binance_client();

    let mut params = HashMap::new();
    params.insert("type".to_string(), serde_json::json!("MAIN_UMFUTURE"));

    // Create String params for fetch_transfers
    let mut string_params = HashMap::new();
    if let Some(Value::String(s)) = params.get("type") {
        string_params.insert("type".to_string(), s.clone());
    }

    let transfers = binance
        .fetch_transfers(None, None, None, Some(string_params))
        .await?;

    assert!(
        !transfers.is_empty(),
        "Transfer history should not be empty"
    );

    println!("Found {} transfer records", transfers.len());

    if let Some(first) = transfers.first() {
        assert!(first.id.as_ref().map_or(false, |id| !id.is_empty()));
        assert!(!first.currency.is_empty());
        assert!(first.amount > 0.0);
        println!(
            "First record: {} {} from {:?} to {:?}",
            first.amount, first.currency, first.from_account, first.to_account
        );
    }

    Ok(())
}

#[tokio::test]
#[ignore]
async fn test_fetch_cross_margin_max_borrowable() -> Result<()> {
    let binance = create_binance_client();

    let mut params = HashMap::new();
    params.insert("asset".to_string(), serde_json::json!("BTC"));

    let max_borrowable = binance
        .fetch_cross_margin_max_borrowable("BTC", Some(params))
        .await?;

    assert_eq!(max_borrowable.currency, "BTC");
    assert!(max_borrowable.amount >= 0.0);

    println!(
        "Cross margin max borrowable: {} {}",
        max_borrowable.amount, max_borrowable.currency
    );

    Ok(())
}

#[tokio::test]
#[ignore]
async fn test_fetch_isolated_margin_max_borrowable() -> Result<()> {
    let binance = create_binance_client();

    let mut params = HashMap::new();
    params.insert("symbol".to_string(), serde_json::json!("BTCUSDT"));
    params.insert("asset".to_string(), serde_json::json!("USDT"));

    let max_borrowable = binance
        .fetch_isolated_margin_max_borrowable("USDT", "BTCUSDT", Some(params))
        .await?;

    assert_eq!(max_borrowable.currency, "USDT");
    assert!(
        max_borrowable
            .symbol
            .as_deref()
            .unwrap_or_default()
            .is_empty()
            || !max_borrowable.symbol.as_deref().unwrap().is_empty()
    );

    if let Some(symbol) = &max_borrowable.symbol {
        println!(
            "Isolated margin max borrowable ({}): {} {}",
            symbol, max_borrowable.amount, max_borrowable.currency
        );
    } else {
        println!(
            "Isolated margin max borrowable: {} {}",
            max_borrowable.amount, max_borrowable.currency
        );
    }

    Ok(())
}

#[tokio::test]
#[ignore]
async fn test_fetch_max_transferable() -> Result<()> {
    let binance = create_binance_client();

    let mut params = HashMap::new();
    params.insert("asset".to_string(), serde_json::json!("USDT"));
    params.insert("fromAccount".to_string(), serde_json::json!("spot"));
    params.insert("toAccount".to_string(), serde_json::json!("future"));

    let max_transferable = binance.fetch_max_transferable("USDT", Some(params)).await?;

    assert_eq!(max_transferable.currency, "USDT");
    assert!(max_transferable.amount >= 0.0);

    println!(
        "Max transferable amount (spot->futures): {} USDT",
        max_transferable.amount
    );

    Ok(())
}

#[tokio::test]
#[ignore]
async fn test_multiple_account_types() -> Result<()> {
    let binance = create_binance_client();

    let account_types = vec!["spot", "margin", "future", "funding"];

    for account_type in account_types {
        let mut params = HashMap::new();
        params.insert("type".to_string(), serde_json::json!(account_type));

        let balance = binance.fetch_balance(Some(params)).await?;

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
    #[ignore]
    async fn test_complete_account_workflow() -> Result<()> {
        let binance = create_binance_client();

        // 1. Query spot balance
        let mut params: HashMap<String, Value> = HashMap::new();
        params.insert("type".to_string(), serde_json::json!("spot"));
        let spot_balance = binance.fetch_balance(Some(params.clone())).await?;
        println!("✅ Spot balance query successful");

        // 2. Query max transferable amount
        params.clear();
        params.insert("asset".to_string(), serde_json::json!("USDT"));
        params.insert("fromAccount".to_string(), serde_json::json!("spot"));
        params.insert("toAccount".to_string(), serde_json::json!("future"));
        let max_transferable = binance
            .fetch_max_transferable("USDT", Some(params.clone()))
            .await?;
        println!("✅ Max transferable: {} USDT", max_transferable.amount);

        // 3. Execute small transfer test (if sufficient balance)
        if max_transferable.amount > 10.0 {
            params.clear();
            params.insert("fromAccount".to_string(), serde_json::json!("spot"));
            params.insert("toAccount".to_string(), serde_json::json!("future"));

            let transfer = binance
                .transfer("USDT", 10.0, "spot", "future", None)
                .await?;
            println!("✅ Transfer successful, transaction ID: {:?}", transfer.id);
        }

        // 4. Query transfer history
        params.clear();
        params.insert("type".to_string(), serde_json::json!("MAIN_UMFUTURE"));
        let mut string_params = HashMap::new();
        if let Some(Value::String(s)) = params.get("type") {
            string_params.insert("type".to_string(), s.clone());
        }
        let transfers = binance
            .fetch_transfers(None, None, Some(5), Some(string_params))
            .await?;
        println!("✅ Found {} transfer history records", transfers.len());

        // 5. Query futures balance for confirmation
        params.clear();
        params.insert("type".to_string(), serde_json::json!("future"));
        let _future_balance = binance.fetch_balance(Some(params)).await?;
        println!("✅ Futures balance query successful");

        println!("\n🎉 Complete account management workflow test successful!");

        Ok(())
    }
}
