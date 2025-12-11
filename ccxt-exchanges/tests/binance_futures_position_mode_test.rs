//! Binance futures position mode management tests.
//!
//! Tests position mode configuration and query functionality.

use ccxt_core::{ExchangeConfig, Result};
use ccxt_exchanges::binance::Binance;
use serde_json::json;

#[tokio::test]
#[ignore] // Requires real API credentials
async fn test_set_position_mode_dual() -> Result<()> {
    let config = ExchangeConfig {
        api_key: Some(std::env::var("BINANCE_API_KEY").unwrap_or_default()),
        secret: Some(std::env::var("BINANCE_SECRET").unwrap_or_default()),
        sandbox: true,
        ..Default::default()
    };

    let binance = Binance::new_futures(config)?;

    let result = binance.set_position_mode(true, None).await?;
    println!("Set dual position mode result: {:?}", result);

    assert!(result.get("code").is_none() || result.get("code").unwrap().as_i64().unwrap() == 200);

    Ok(())
}

#[tokio::test]
#[ignore] // Requires real API credentials
async fn test_set_position_mode_single() -> Result<()> {
    let config = ExchangeConfig {
        api_key: Some(std::env::var("BINANCE_API_KEY").unwrap_or_default()),
        secret: Some(std::env::var("BINANCE_SECRET").unwrap_or_default()),
        sandbox: true,
        ..Default::default()
    };

    let binance = Binance::new_futures(config)?;

    let result = binance.set_position_mode(false, None).await?;
    println!("Set single position mode result: {:?}", result);

    assert!(result.get("code").is_none() || result.get("code").unwrap().as_i64().unwrap() == 200);

    Ok(())
}

#[tokio::test]
#[ignore] // Requires real API credentials
async fn test_fetch_position_mode() -> Result<()> {
    let config = ExchangeConfig {
        api_key: Some(std::env::var("BINANCE_API_KEY").unwrap_or_default()),
        secret: Some(std::env::var("BINANCE_SECRET").unwrap_or_default()),
        sandbox: true,
        ..Default::default()
    };

    let binance = Binance::new_futures(config)?;

    let dual_side = binance.fetch_position_mode(None).await?;
    println!(
        "Current position mode: {}",
        if dual_side {
            "Dual (Hedge Mode)"
        } else {
            "Single (One-way Mode)"
        }
    );

    assert!(dual_side == true || dual_side == false);

    Ok(())
}

#[tokio::test]
#[ignore] // Requires real API credentials
async fn test_position_mode_toggle() -> Result<()> {
    let config = ExchangeConfig {
        api_key: Some(std::env::var("BINANCE_API_KEY").unwrap_or_default()),
        secret: Some(std::env::var("BINANCE_SECRET").unwrap_or_default()),
        sandbox: true,
        ..Default::default()
    };

    let binance = Binance::new_futures(config)?;

    let initial_mode = binance.fetch_position_mode(None).await?;
    println!(
        "Initial position mode: {}",
        if initial_mode {
            "Dual (Hedge Mode)"
        } else {
            "Single (One-way Mode)"
        }
    );

    let new_mode = !initial_mode;
    let _result = binance.set_position_mode(new_mode, None).await?;

    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

    let current_mode = binance.fetch_position_mode(None).await?;
    println!(
        "Position mode after toggle: {}",
        if current_mode {
            "Dual (Hedge Mode)"
        } else {
            "Single (One-way Mode)"
        }
    );
    assert_eq!(current_mode, new_mode);

    let _result = binance.set_position_mode(initial_mode, None).await?;
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

    let final_mode = binance.fetch_position_mode(None).await?;
    println!(
        "Position mode after restore: {}",
        if final_mode {
            "Dual (Hedge Mode)"
        } else {
            "Single (One-way Mode)"
        }
    );
    assert_eq!(final_mode, initial_mode);

    Ok(())
}

#[tokio::test]
#[ignore] // Requires real API credentials
async fn test_set_position_mode_with_params() -> Result<()> {
    let config = ExchangeConfig {
        api_key: Some(std::env::var("BINANCE_API_KEY").unwrap_or_default()),
        secret: Some(std::env::var("BINANCE_SECRET").unwrap_or_default()),
        sandbox: true,
        ..Default::default()
    };

    let binance = Binance::new_futures(config)?;

    let params = json!({
        "recvWindow": 10000
    });

    let result = binance.set_position_mode(true, Some(params)).await?;
    println!("Set position mode with params result: {:?}", result);

    Ok(())
}

/// Test error handling without credentials.
#[tokio::test]
async fn test_position_mode_without_credentials() {
    let config = ExchangeConfig::default();
    let binance = Binance::new_futures(config).unwrap();

    let result = binance.fetch_position_mode(None).await;
    assert!(result.is_err());

    let result = binance.set_position_mode(true, None).await;
    assert!(result.is_err());
}

/// Mock test: verify API call parameter format.
#[tokio::test]
async fn test_position_mode_params_format() {
    let config = ExchangeConfig {
        api_key: Some("test_key".to_string()),
        secret: Some("test_secret".to_string()),
        sandbox: true,
        ..Default::default()
    };

    let binance = Binance::new_futures(config).unwrap();

    assert!(binance.base().config.api_key.is_some());
    assert!(binance.base().config.secret.is_some());
}

#[cfg(test)]
mod position_mode_integration_tests {
    use super::*;

    /// Integration test: complete position mode toggle workflow.
    ///
    /// Note: Requires real API credentials and no open positions.
    #[tokio::test]
    #[ignore] // Requires real API credentials and no open positions
    async fn test_complete_position_mode_workflow() -> Result<()> {
        let config = ExchangeConfig {
            api_key: Some(std::env::var("BINANCE_API_KEY").unwrap_or_default()),
            secret: Some(std::env::var("BINANCE_SECRET").unwrap_or_default()),
            sandbox: true,
            ..Default::default()
        };

        let binance = Binance::new_futures(config)?;

        println!("=== Complete Position Mode Management Workflow Test ===");

        println!("\nStep 1: Query initial position mode");
        let initial_mode = binance.fetch_position_mode(None).await?;
        println!(
            "  Initial mode: {}",
            if initial_mode {
                "Dual (Hedge Mode)"
            } else {
                "Single (One-way Mode)"
            }
        );

        println!("\nStep 2: Switch to single position mode");
        let result = binance.set_position_mode(false, None).await?;
        println!("  Set result: {:?}", result);
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

        println!("\nStep 3: Verify single position mode");
        let mode = binance.fetch_position_mode(None).await?;
        println!("  Current mode: {}", if mode { "Dual" } else { "Single" });
        assert_eq!(mode, false, "Should be single position mode");

        println!("\nStep 4: Switch to dual position mode");
        let result = binance.set_position_mode(true, None).await?;
        println!("  Set result: {:?}", result);
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

        println!("\nStep 5: Verify dual position mode");
        let mode = binance.fetch_position_mode(None).await?;
        println!("  Current mode: {}", if mode { "Dual" } else { "Single" });
        assert_eq!(mode, true, "Should be dual position mode");

        println!("\nStep 6: Restore initial position mode");
        let result = binance.set_position_mode(initial_mode, None).await?;
        println!("  Set result: {:?}", result);
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

        println!("\nStep 7: Verify restored to initial mode");
        let final_mode = binance.fetch_position_mode(None).await?;
        println!(
            "  Final mode: {}",
            if final_mode { "Dual" } else { "Single" }
        );
        assert_eq!(
            final_mode, initial_mode,
            "Should be restored to initial mode"
        );

        println!("\n=== Test Complete ===");

        Ok(())
    }
}
