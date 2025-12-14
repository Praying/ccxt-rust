//! Binance Futures Margin Management Example
//!
//! Demonstrates futures margin management operations including:
//! - Adjusting isolated position margin
//! - Querying position risk information
//! - Fetching leverage bracket configuration
//!
//! # Prerequisites
//!
//! Set the following environment variables:
//! - `BINANCE_API_KEY`: Your Binance API key
//! - `BINANCE_SECRET`: Your Binance API secret
//!
//! # Examples
//!
//! ```bash
//! export BINANCE_API_KEY="your_api_key"
//! export BINANCE_SECRET="your_secret"
//! cargo run --example binance_futures_margin_example
//! ```

// Allow clippy warnings for example code - examples prioritize readability over strict linting
#![allow(clippy::collapsible_if)]

use ccxt_core::ExchangeConfig;
use ccxt_exchanges::binance::Binance;
use rust_decimal_macros::dec;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let api_key =
        std::env::var("BINANCE_API_KEY").expect("BINANCE_API_KEY environment variable must be set");
    let secret =
        std::env::var("BINANCE_SECRET").expect("BINANCE_SECRET environment variable must be set");

    let config = ExchangeConfig {
        api_key: Some(api_key),
        secret: Some(secret),
        ..Default::default()
    };

    let binance = Binance::new_swap(config)?;
    println!("✓ Binance Futures API connected successfully\n");

    example_1_fetch_leverage_bracket(&binance).await?;
    example_2_fetch_position_risk(&binance).await?;
    example_3_modify_margin(&binance).await?;
    example_4_risk_management_workflow(&binance).await?;

    Ok(())
}

/// Example 1: Fetch leverage bracket information
///
/// Leverage brackets display maximum leverage multipliers for different position sizes.
async fn example_1_fetch_leverage_bracket(
    binance: &Binance,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Example 1: Fetch Leverage Bracket ===\n");

    let symbol = "BTC/USDT";
    println!("Fetching leverage bracket for {}...", symbol);

    let result = binance.fetch_leverage_bracket(Some(symbol), None).await?;

    println!("Response: {}\n", serde_json::to_string_pretty(&result)?);

    if let Some(brackets) = result.as_array() {
        if let Some(first) = brackets.first() {
            if let Some(bracket_list) = first.get("brackets").and_then(|b| b.as_array()) {
                println!("Leverage Bracket Details:");
                println!(
                    "{:<8} {:<15} {:<20} {:<20} {:<15}",
                    "Tier", "Max Leverage", "Notional Floor", "Notional Cap", "Maint. Margin %"
                );
                println!("{}", "-".repeat(80));

                for bracket in bracket_list {
                    let tier = bracket.get("bracket").and_then(|b| b.as_i64()).unwrap_or(0);
                    let leverage = bracket
                        .get("initialLeverage")
                        .and_then(|l| l.as_i64())
                        .unwrap_or(0);
                    let floor = bracket
                        .get("notionalFloor")
                        .and_then(|f| f.as_f64())
                        .unwrap_or(0.0);
                    let cap = bracket
                        .get("notionalCap")
                        .and_then(|c| c.as_f64())
                        .unwrap_or(0.0);
                    let maint_margin = bracket
                        .get("maintMarginRatio")
                        .and_then(|m| m.as_f64())
                        .unwrap_or(0.0);

                    println!(
                        "{:<8} {:<15} ${:<19.2} ${:<19.2} {:.2}%",
                        tier,
                        format!("{}x", leverage),
                        floor,
                        cap,
                        maint_margin * 100.0
                    );
                }
            }
        }
    }

    println!("\n✓ Leverage bracket query completed\n");
    Ok(())
}

/// Example 2: Fetch position risk information
///
/// Displays risk information for all positions including liquidation price and unrealized PnL.
async fn example_2_fetch_position_risk(
    binance: &Binance,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Example 2: Fetch Position Risk ===\n");

    println!("Querying all position risks...");
    let result = binance.fetch_position_risk(None, None).await?;

    println!("Response: {}\n", serde_json::to_string_pretty(&result)?);

    if let Some(positions) = result.as_array() {
        println!("Position Risk Details:");
        println!(
            "{:<12} {:<12} {:<15} {:<15} {:<15} {:<12}",
            "Symbol", "Side", "Position Amt", "Unrealized PnL", "Liq. Price", "Leverage"
        );
        println!("{}", "-".repeat(85));

        for pos in positions {
            let symbol = pos.get("symbol").and_then(|s| s.as_str()).unwrap_or("");
            let position_side = pos
                .get("positionSide")
                .and_then(|s| s.as_str())
                .unwrap_or("BOTH");
            let position_amt = pos
                .get("positionAmt")
                .and_then(|a| a.as_str())
                .and_then(|s| s.parse::<f64>().ok())
                .unwrap_or(0.0);
            let unrealized_profit = pos
                .get("unRealizedProfit")
                .and_then(|p| p.as_str())
                .and_then(|s| s.parse::<f64>().ok())
                .unwrap_or(0.0);
            let liquidation_price = pos
                .get("liquidationPrice")
                .and_then(|p| p.as_str())
                .and_then(|s| s.parse::<f64>().ok())
                .unwrap_or(0.0);
            let leverage = pos
                .get("leverage")
                .and_then(|l| l.as_str())
                .and_then(|s| s.parse::<i64>().ok())
                .unwrap_or(0);

            if position_amt != 0.0 {
                println!(
                    "{:<12} {:<12} {:<15.4} ${:<14.2} ${:<14.2} {}x",
                    symbol,
                    position_side,
                    position_amt,
                    unrealized_profit,
                    liquidation_price,
                    leverage
                );
            }
        }
    }

    println!("\n✓ Position risk query completed\n");
    Ok(())
}

/// Example 3: Modify isolated position margin
///
/// Demonstrates how to add or reduce isolated margin for a position.
async fn example_3_modify_margin(binance: &Binance) -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Example 3: Modify Isolated Position Margin ===\n");

    let symbol = "BTC/USDT";

    println!("Querying current position...");
    let positions = binance.fetch_position_risk(Some(symbol), None).await?;

    if let Some(pos_array) = positions.as_array() {
        if let Some(pos) = pos_array.first() {
            let position_amt = pos
                .get("positionAmt")
                .and_then(|a| a.as_str())
                .and_then(|s| s.parse::<f64>().ok())
                .unwrap_or(0.0);
            let margin_type = pos
                .get("marginType")
                .and_then(|m| m.as_str())
                .unwrap_or("cross");

            if position_amt == 0.0 {
                println!("⚠️  No open position, skipping margin adjustment example");
            } else if margin_type == "cross" {
                println!("⚠️  Position is in cross margin mode, cannot adjust margin");
            } else {
                println!(
                    "Current position: {:.4}, margin type: {}",
                    position_amt, margin_type
                );

                println!("\nAdding 10 USDT margin...");
                match binance
                    .modify_isolated_position_margin(symbol, dec!(10.0), None)
                    .await
                {
                    Ok(result) => {
                        println!("✓ Margin added successfully");
                        println!("Response: {}", serde_json::to_string_pretty(&result)?);
                    }
                    Err(e) => {
                        println!("✗ Failed to add margin: {:?}", e);
                    }
                }

                println!("\nReducing 5 USDT margin...");
                match binance
                    .modify_isolated_position_margin(symbol, dec!(-5.0), None)
                    .await
                {
                    Ok(result) => {
                        println!("✓ Margin reduced successfully");
                        println!("Response: {}", serde_json::to_string_pretty(&result)?);
                    }
                    Err(e) => {
                        println!("✗ Failed to reduce margin: {:?}", e);
                    }
                }
            }
        }
    }

    println!("\n✓ Margin adjustment example completed\n");
    Ok(())
}

/// Example 4: Complete risk management workflow
///
/// Demonstrates a comprehensive risk management process.
async fn example_4_risk_management_workflow(
    binance: &Binance,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Example 4: Risk Management Workflow ===\n");

    let symbol = "BTC/USDT";

    println!("Step 1: Fetch leverage bracket...");
    let bracket = binance.fetch_leverage_bracket(Some(symbol), None).await?;

    let max_leverage = if let Some(brackets) = bracket.as_array() {
        brackets
            .first()
            .and_then(|b| b.get("brackets"))
            .and_then(|b| b.as_array())
            .and_then(|arr| arr.first())
            .and_then(|b| b.get("initialLeverage"))
            .and_then(|l| l.as_i64())
            .unwrap_or(1)
    } else {
        1
    };
    println!("✓ Maximum leverage: {}x\n", max_leverage);

    println!("Step 2: Fetch position risk...");
    let risk = binance.fetch_position_risk(Some(symbol), None).await?;

    if let Some(positions) = risk.as_array() {
        for pos in positions {
            let position_amt = pos
                .get("positionAmt")
                .and_then(|a| a.as_str())
                .and_then(|s| s.parse::<f64>().ok())
                .unwrap_or(0.0);

            if position_amt != 0.0 {
                let unrealized_profit = pos
                    .get("unRealizedProfit")
                    .and_then(|p| p.as_str())
                    .and_then(|s| s.parse::<f64>().ok())
                    .unwrap_or(0.0);
                let liquidation_price = pos
                    .get("liquidationPrice")
                    .and_then(|p| p.as_str())
                    .and_then(|s| s.parse::<f64>().ok())
                    .unwrap_or(0.0);
                let entry_price = pos
                    .get("entryPrice")
                    .and_then(|p| p.as_str())
                    .and_then(|s| s.parse::<f64>().ok())
                    .unwrap_or(0.0);
                let mark_price = pos
                    .get("markPrice")
                    .and_then(|p| p.as_str())
                    .and_then(|s| s.parse::<f64>().ok())
                    .unwrap_or(0.0);

                println!("Position Information:");
                println!("  Position amount: {:.4}", position_amt);
                println!("  Entry price: ${:.2}", entry_price);
                println!("  Mark price: ${:.2}", mark_price);
                println!("  Liquidation price: ${:.2}", liquidation_price);
                println!("  Unrealized PnL: ${:.2}", unrealized_profit);

                println!("\nStep 3: Risk assessment...");
                let distance_to_liquidation = if position_amt > 0.0 {
                    ((mark_price - liquidation_price) / mark_price) * 100.0
                } else {
                    ((liquidation_price - mark_price) / mark_price) * 100.0
                };

                println!("  Distance to liquidation: {:.2}%", distance_to_liquidation);

                if distance_to_liquidation < 5.0 {
                    println!(
                        "  ⚠️  Risk level: HIGH - Consider adding margin or reducing position"
                    );
                } else if distance_to_liquidation < 10.0 {
                    println!("  ⚠️  Risk level: MEDIUM - Monitor market closely");
                } else {
                    println!("  ✓ Risk level: LOW - Position relatively safe");
                }

                let margin_type = pos
                    .get("marginType")
                    .and_then(|m| m.as_str())
                    .unwrap_or("cross");

                if margin_type == "isolated" && distance_to_liquidation < 10.0 {
                    println!("\nStep 4: Risk control recommendations...");
                    println!("  Recommended action: Add margin to reduce liquidation risk");
                    println!("  Alternatively: Partially close position to reduce exposure");
                }
            } else {
                println!("✓ No open positions\n");
            }
        }
    }

    println!("\n✓ Risk management workflow completed\n");
    Ok(())
}
