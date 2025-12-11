//! Binance Futures Position Mode Management Example
//!
//! Demonstrates position mode operations on Binance Futures:
//! - Query current position mode
//! - Set one-way or hedge position mode
//! - Impact of position modes on order placement

use ccxt_core::{ExchangeConfig, Result};
use ccxt_exchanges::binance::Binance;

#[tokio::main]
async fn main() -> Result<()> {
    let api_key = std::env::var("BINANCE_API_KEY").expect("BINANCE_API_KEY must be set");
    let secret = std::env::var("BINANCE_SECRET").expect("BINANCE_SECRET must be set");

    let config = ExchangeConfig {
        api_key: Some(api_key),
        secret: Some(secret),
        sandbox: true,
        ..Default::default()
    };

    let binance = Binance::new_futures(config)?;
    println!("=== Binance Futures Position Mode Management ===\n");

    example_fetch_position_mode(&binance).await?;
    example_set_single_position_mode(&binance).await?;
    example_set_dual_position_mode(&binance).await?;
    example_position_mode_workflow(&binance).await?;

    Ok(())
}

/// Fetch and display the current position mode.
///
/// Queries whether hedge mode (dual-side) or one-way mode is currently active.
async fn example_fetch_position_mode(binance: &Binance) -> Result<()> {
    println!("--- Example 1: Query Current Position Mode ---");

    let dual_side = binance.fetch_position_mode(None).await?;

    if dual_side {
        println!("✓ Current mode: Hedge Mode (Dual-side)");
        println!("  - Can hold both long and short positions simultaneously");
        println!("  - Must specify positionSide: LONG or SHORT when placing orders");
    } else {
        println!("✓ Current mode: One-way Mode");
        println!("  - Only one position direction allowed per symbol");
        println!("  - positionSide is fixed to BOTH");
    }

    println!();
    Ok(())
}

/// Set one-way position mode with verification.
///
/// Switches to one-way mode where only a single position direction is allowed
/// per symbol. Verifies the mode change after a 1-second delay.
async fn example_set_single_position_mode(binance: &Binance) -> Result<()> {
    println!("--- Example 2: Set One-way Position Mode ---");

    match binance.set_position_mode(false, None).await {
        Ok(result) => {
            println!("✓ Successfully set to one-way position mode");
            println!("  Response: {:?}", result);

            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

            let mode = binance.fetch_position_mode(None).await?;
            if !mode {
                println!("✓ Verification successful: One-way mode active");
            }
        }
        Err(e) => {
            println!("✗ Failed to set mode: {:?}", e);
            println!("  Possible reasons:");
            println!("  1. Cannot switch mode when positions exist");
            println!("  2. Insufficient API permissions");
        }
    }

    println!();
    Ok(())
}

/// Set hedge (dual-side) position mode with verification.
///
/// Switches to hedge mode where both long and short positions can be held
/// simultaneously. Verifies the mode change after a 1-second delay.
async fn example_set_dual_position_mode(binance: &Binance) -> Result<()> {
    println!("--- Example 3: Set Hedge (Dual-side) Position Mode ---");

    match binance.set_position_mode(true, None).await {
        Ok(result) => {
            println!("✓ Successfully set to hedge (dual-side) position mode");
            println!("  Response: {:?}", result);

            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

            let mode = binance.fetch_position_mode(None).await?;
            if mode {
                println!("✓ Verification successful: Hedge mode active");
            }
        }
        Err(e) => {
            println!("✗ Failed to set mode: {:?}", e);
            println!("  Possible reasons:");
            println!("  1. Cannot switch mode when positions exist");
            println!("  2. Insufficient API permissions");
        }
    }

    println!();
    Ok(())
}

/// Demonstrate complete position mode workflow with educational output.
///
/// Provides comprehensive information including mode detection, feature
/// comparison, switching prerequisites, order placement examples, and best
/// practice recommendations.
async fn example_position_mode_workflow(binance: &Binance) -> Result<()> {
    println!("--- Example 4: Position Mode Workflow ---");

    println!("\nStep 1: Query Initial Position Mode");
    let initial_mode = binance.fetch_position_mode(None).await?;
    println!(
        "  Initial mode: {}",
        if initial_mode {
            "Hedge (dual-side)"
        } else {
            "One-way"
        }
    );

    println!("\nStep 2: Position Mode Overview");
    if initial_mode {
        println!("  Hedge Mode (Dual-side):");
        println!("  ✓ Hold both long and short positions simultaneously");
        println!("  ✓ Suitable for hedging strategies");
        println!("  ✓ Must specify positionSide when placing orders");
        println!("    - LONG: Long position");
        println!("    - SHORT: Short position");
    } else {
        println!("  One-way Mode:");
        println!("  ✓ Only one direction per symbol");
        println!("  ✓ Simple and intuitive for directional trading");
        println!("  ✓ positionSide fixed to BOTH");
    }

    println!("\nStep 3: Mode Switching Prerequisites");
    println!("  ⚠️  Before switching, ensure:");
    println!("  1. No open positions exist");
    println!("  2. No pending orders");
    println!("  3. API key has sufficient permissions");

    println!("\nStep 4: Order Usage by Mode");
    println!("\n  One-way Mode Order Example:");
    println!("  ```rust");
    println!("  // Open long");
    println!(
        "  binance.create_order(\"BTC/USDT:USDT\", \"market\", \"buy\", 0.001, None, None).await?;"
    );
    println!("  // Close long");
    println!(
        "  binance.create_order(\"BTC/USDT:USDT\", \"market\", \"sell\", 0.001, None, None).await?;"
    );
    println!("  ```");

    println!("\n  Hedge Mode Order Example:");
    println!("  ```rust");
    println!("  // Open long (must specify positionSide: LONG)");
    println!("  let params = json!({{\"positionSide\": \"LONG\"}});");
    println!(
        "  binance.create_order(\"BTC/USDT:USDT\", \"market\", \"buy\", 0.001, None, Some(params)).await?;"
    );
    println!("  ");
    println!("  // Open short (must specify positionSide: SHORT)");
    println!("  let params = json!({{\"positionSide\": \"SHORT\"}});");
    println!(
        "  binance.create_order(\"BTC/USDT:USDT\", \"market\", \"sell\", 0.001, None, Some(params)).await?;"
    );
    println!("  ```");

    println!("\nStep 5: Best Practice Recommendations");
    println!("  💡 One-way Mode Use Cases:");
    println!("     - Directional trend trading");
    println!("     - Simple long or short strategies");
    println!("     - Beginner traders");

    println!("\n  💡 Hedge Mode Use Cases:");
    println!("     - Grid trading");
    println!("     - Hedging strategies");
    println!("     - Simultaneous long/short arbitrage");
    println!("     - Advanced trading strategies");

    println!("\n  ⚠️  Risk Warning:");
    println!("     - Hedge mode increases operational complexity");
    println!("     - Requires careful position direction management");
    println!("     - Incorrect positionSide may open reverse position");

    println!();
    Ok(())
}

/// Safely switches position mode with comprehensive pre-flight checks.
///
/// Performs validation steps before attempting mode change:
/// 1. Verifies no open positions exist
/// 2. Checks for pending orders (simplified in example)
/// 3. Executes mode change
/// 4. Waits for propagation
/// 5. Validates new mode
///
/// # Arguments
///
/// * `binance` - Reference to the Binance futures client
/// * `dual_side` - Target mode: `true` for hedge mode, `false` for one-way mode
///
/// # Errors
///
/// Returns error if API calls fail or validation checks detect issues.
#[allow(dead_code)]
async fn safe_switch_position_mode(binance: &Binance, dual_side: bool) -> Result<()> {
    println!("--- Safe Position Mode Switch ---");

    println!("1. Check current positions...");
    let positions = binance.fetch_positions(None, None).await?;
    let has_position = positions.iter().any(|p| p.contracts.unwrap_or(0.0) > 0.0);

    if has_position {
        println!("✗ Positions detected, cannot switch mode");
        println!("  Please close all positions first");
        return Ok(());
    }
    println!("✓ No positions");

    println!("2. Check pending orders...");
    // NOTE: Complete implementation would iterate all symbols to check for open orders
    println!("✓ Manual confirmation recommended for open orders");

    println!("3. Switch position mode...");
    let result = binance.set_position_mode(dual_side, None).await?;
    println!("✓ Switch successful: {:?}", result);

    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

    println!("4. Verify new mode...");
    let mode = binance.fetch_position_mode(None).await?;
    if mode == dual_side {
        println!(
            "✓ Verification successful: Position mode updated to {}",
            if dual_side {
                "hedge mode"
            } else {
                "one-way mode"
            }
        );
    } else {
        println!("✗ Verification failed: Position mode not updated");
    }

    Ok(())
}
