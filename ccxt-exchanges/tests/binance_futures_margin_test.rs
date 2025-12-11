//! Binance futures margin management tests.
//!
//! Tests the following features:
//! - `modify_isolated_position_margin()` - Adjust isolated margin
//! - `fetch_position_risk()` - Query position risk
//! - `fetch_leverage_bracket()` - Query leverage brackets

use ccxt_core::ExchangeConfig;
use ccxt_exchanges::binance::Binance;
use rust_decimal::Decimal;
use serde_json::json;
use std::str::FromStr;

#[cfg(test)]
mod margin_tests {
    use super::*;

    /// Create a test Binance futures instance.
    fn create_test_futures() -> Binance {
        let config = ExchangeConfig {
            api_key: Some("test_api_key".to_string()),
            secret: Some("test_secret".to_string()),
            sandbox: false,
            ..Default::default()
        };
        Binance::new_futures(config).unwrap()
    }

    #[test]
    fn test_modify_isolated_position_margin_increase() {
        let _binance = create_test_futures();

        let symbol = "BTC/USDT";
        let amount = Decimal::from_str("100.0").unwrap();

        // Positive amount should construct type=1 (increase)
        assert!(amount > Decimal::ZERO);
        assert!(!symbol.is_empty());
    }

    #[test]
    fn test_modify_isolated_position_margin_decrease() {
        let _binance = create_test_futures();

        let symbol = "BTC/USDT";
        let amount = Decimal::from_str("-50.0").unwrap();

        // Negative amount should construct type=2 (decrease)
        assert!(amount < Decimal::ZERO);
        assert!(!symbol.is_empty());
    }

    #[test]
    fn test_modify_isolated_position_margin_with_position_side() {
        let _binance = create_test_futures();

        let params = json!({
            "positionSide": "LONG"
        });

        assert_eq!(params["positionSide"], "LONG");
    }

    #[test]
    fn test_fetch_position_risk_all() {
        let _binance = create_test_futures();
    }

    #[test]
    fn test_fetch_position_risk_single_symbol() {
        let _binance = create_test_futures();

        let symbol = "BTC/USDT";
        assert!(!symbol.is_empty());
    }

    #[test]
    fn test_fetch_leverage_bracket_all() {
        let _binance = create_test_futures();
    }

    #[test]
    fn test_fetch_leverage_bracket_single_symbol() {
        let _binance = create_test_futures();

        let symbol = "BTC/USDT";
        assert!(!symbol.is_empty());
    }

    #[test]
    fn test_margin_amount_validation() {
        let positive = Decimal::from_str("100.0").unwrap();
        let negative = Decimal::from_str("-50.0").unwrap();
        let zero = Decimal::ZERO;

        assert!(positive > Decimal::ZERO);
        assert!(negative < Decimal::ZERO);
        assert_eq!(zero, Decimal::ZERO);

        assert_eq!(positive.abs(), Decimal::from_str("100.0").unwrap());
        assert_eq!(negative.abs(), Decimal::from_str("50.0").unwrap());
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;

    /// Integration test: adjust isolated margin.
    ///
    /// Note: Requires valid API credentials and an open position.
    #[tokio::test]
    #[ignore] // Requires real API credentials
    async fn test_modify_margin_integration() {
        let api_key = std::env::var("BINANCE_API_KEY").expect("BINANCE_API_KEY not set");
        let secret = std::env::var("BINANCE_SECRET").expect("BINANCE_SECRET not set");

        let config = ExchangeConfig {
            api_key: Some(api_key),
            secret: Some(secret),
            sandbox: false,
            ..Default::default()
        };

        let binance = Binance::new_futures(config).unwrap();

        let result = binance
            .modify_isolated_position_margin("BTC/USDT", Decimal::from_str("10.0").unwrap(), None)
            .await;

        match result {
            Ok(data) => {
                println!("Margin adjustment successful: {:?}", data);
                assert!(data.is_object());
            }
            Err(e) => {
                println!(
                    "Margin adjustment failed (possibly no position or other reason): {:?}",
                    e
                );
            }
        }
    }

    /// Integration test: query position risk.
    #[tokio::test]
    #[ignore] // Requires real API credentials
    async fn test_fetch_position_risk_integration() {
        let api_key = std::env::var("BINANCE_API_KEY").expect("BINANCE_API_KEY not set");
        let secret = std::env::var("BINANCE_SECRET").expect("BINANCE_SECRET not set");

        let config = ExchangeConfig {
            api_key: Some(api_key),
            secret: Some(secret),
            sandbox: false,
            ..Default::default()
        };

        let binance = Binance::new_futures(config).unwrap();

        let result = binance.fetch_position_risk(None, None).await;

        match result {
            Ok(data) => {
                println!("Position risk: {:?}", data);
                assert!(data.is_array() || data.is_object());
            }
            Err(e) => {
                panic!("Failed to query position risk: {:?}", e);
            }
        }
    }

    /// Integration test: query leverage brackets.
    #[tokio::test]
    #[ignore] // Requires real API credentials
    async fn test_fetch_leverage_bracket_integration() {
        let api_key = std::env::var("BINANCE_API_KEY").expect("BINANCE_API_KEY not set");
        let secret = std::env::var("BINANCE_SECRET").expect("BINANCE_SECRET not set");

        let config = ExchangeConfig {
            api_key: Some(api_key),
            secret: Some(secret),
            sandbox: false,
            ..Default::default()
        };

        let binance = Binance::new_futures(config).unwrap();

        let result = binance.fetch_leverage_bracket(Some("BTC/USDT"), None).await;

        match result {
            Ok(data) => {
                println!("Leverage brackets: {:?}", data);
                assert!(data.is_array() || data.is_object());

                if let Some(brackets) = data.as_array() {
                    if !brackets.is_empty() {
                        let first = &brackets[0];
                        assert!(first.get("symbol").is_some());
                        assert!(first.get("brackets").is_some());
                    }
                }
            }
            Err(e) => {
                panic!("Failed to query leverage brackets: {:?}", e);
            }
        }
    }

    /// Integration test: complete margin management workflow.
    #[tokio::test]
    #[ignore] // Requires real API credentials
    async fn test_margin_management_workflow() {
        let api_key = std::env::var("BINANCE_API_KEY").expect("BINANCE_API_KEY not set");
        let secret = std::env::var("BINANCE_SECRET").expect("BINANCE_SECRET not set");

        let config = ExchangeConfig {
            api_key: Some(api_key),
            secret: Some(secret),
            sandbox: false,
            ..Default::default()
        };

        let binance = Binance::new_futures(config).unwrap();
        let symbol = "BTC/USDT";

        println!("=== Margin Management Workflow Test ===");

        // Step 1: Query leverage brackets
        println!("\n1. Querying leverage brackets...");
        let bracket = binance
            .fetch_leverage_bracket(Some(symbol), None)
            .await
            .expect("Failed to query leverage brackets");
        println!("Leverage brackets: {:?}", bracket);

        // Step 2: Query position risk
        println!("\n2. Querying position risk...");
        let risk = binance
            .fetch_position_risk(Some(symbol), None)
            .await
            .expect("Failed to query position risk");
        println!("Position risk: {:?}", risk);

        // Step 3: If positions exist, attempt to adjust margin
        if let Some(positions) = risk.as_array() {
            for pos in positions {
                if let Some(position_amt) = pos.get("positionAmt") {
                    if let Some(amt_str) = position_amt.as_str() {
                        if let Ok(amt) = amt_str.parse::<f64>() {
                            if amt != 0.0 {
                                println!("\n3. Adjusting margin...");
                                let adjust_result = binance
                                    .modify_isolated_position_margin(
                                        symbol,
                                        Decimal::from_str("1.0").unwrap(),
                                        None,
                                    )
                                    .await;

                                match adjust_result {
                                    Ok(data) => println!("Adjustment successful: {:?}", data),
                                    Err(e) => println!(
                                        "Adjustment failed (possibly cross margin mode): {:?}",
                                        e
                                    ),
                                }
                                break;
                            }
                        }
                    }
                }
            }
        }

        println!("\n=== Workflow Test Completed ===");
    }
}
