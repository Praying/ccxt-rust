//! Logging System Usage Example
//!
//! Demonstrates how to configure and use the ccxt-rust structured logging system.
//!
//! # Usage
//!
//! ```bash
//! # Run with different log levels
//! RUST_LOG=debug cargo run --example logging_example
//! RUST_LOG=ccxt_core=trace cargo run --example logging_example
//! ```

// Allow clippy warnings for example code - examples prioritize readability over strict linting
#![allow(clippy::field_reassign_with_default)]

use ccxt_core::{
    ExchangeConfig,
    error::Result,
    logging::{LogConfig, LogFormat, LogLevel, init_logging},
};
use ccxt_exchanges::binance::Binance;

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== CCXT-Rust Logging System Example ===\n");

    println!("1. Using default log configuration (Info level, Pretty format)");
    init_logging(LogConfig::default());
    println!("   ✓ Logging system initialized\n");

    let mut config = ExchangeConfig::default();
    config.verbose = true;
    let exchange = Binance::new(config)?;

    println!("   Executing API call to generate log output...");
    match exchange
        .fetch_ticker("BTC/USDT", ccxt_core::types::TickerParams::default())
        .await
    {
        Ok(ticker) => {
            println!("   ✓ Fetched ticker: {} @ {:?}", ticker.symbol, ticker.last);
        }
        Err(e) => {
            println!("   ✗ Error: {}", e);
        }
    }
    println!();

    println!("2. Development environment configuration (Debug level, show span events)");
    println!("   Configuration:");
    println!("   - Log level: Debug");
    println!("   - Format: Pretty");
    println!("   - Show time: Yes");
    println!("   - Show target: Yes");
    println!("   - Show span events: Yes");
    println!();

    println!("3. Production environment configuration (Info level, JSON format)");
    println!("   Configuration:");
    println!("   - Log level: Info");
    println!("   - Format: Json");
    println!("   - Show time: Yes");
    println!("   - Show thread IDs: Yes");
    println!("   - Show target: Yes");
    println!("   - Show span events: No");
    println!();

    println!("4. Custom log configuration");
    let _custom_config = LogConfig {
        level: LogLevel::Warn,
        format: LogFormat::Compact,
        show_time: true,
        show_thread_ids: false,
        show_target: true,
        show_span_events: false,
    };
    println!("   Configuration:");
    println!("   - Log level: Warn (warnings and errors only)");
    println!("   - Format: Compact");
    println!();

    println!("5. Control logging with environment variables");
    println!("   You can override configuration by setting RUST_LOG:");
    println!();
    println!("   # Set global log level to debug");
    println!("   export RUST_LOG=debug");
    println!();
    println!("   # Show debug logs for ccxt_core module only");
    println!("   export RUST_LOG=ccxt_core=debug");
    println!();
    println!("   # Set different levels for different modules");
    println!("   export RUST_LOG=warn,ccxt_core::http_client=debug,ccxt_exchanges::binance=info");
    println!();
    println!("   # Filter specific log targets");
    println!("   export RUST_LOG=ccxt_core::http_client=debug");
    println!();

    println!("6. Log level descriptions");
    println!("   Log levels from most to least verbose:");
    println!();
    println!("   TRACE - Most detailed debugging information");
    println!("   └─ For: Tracing program execution flow");
    println!("   └─ Example: Function entry/exit, loop iterations");
    println!();
    println!("   DEBUG - Development debugging information");
    println!("   └─ For: Detailed information during development");
    println!("   └─ Example: HTTP request/response details, data parsing process");
    println!();
    println!("   INFO  - Important business events (production default)");
    println!("   └─ For: Recording critical business operations");
    println!("   └─ Example: API call success, order creation");
    println!();
    println!("   WARN  - Warning messages");
    println!("   └─ For: Potential issues or recoverable errors");
    println!("   └─ Example: Request retry, data parsing failure but processing continues");
    println!();
    println!("   ERROR - Error messages");
    println!("   └─ For: Serious errors and failed operations");
    println!("   └─ Example: API call failure, connection errors");
    println!();

    println!("7. Log output format descriptions");
    println!();
    println!("   Pretty format (recommended for development):");
    println!("   ```");
    println!("   2024-01-20T10:30:45.123Z DEBUG ccxt_core::http_client: HTTP request");
    println!("     method: GET");
    println!("     url: https://api.binance.com/api/v3/ticker/24hr");
    println!("   ```");
    println!();
    println!("   Compact format (space-saving):");
    println!("   ```");
    println!(
        "   2024-01-20T10:30:45.123Z DEBUG ccxt_core::http_client: HTTP request method=GET url=https://..."
    );
    println!("   ```");
    println!();
    println!("   JSON format (recommended for production, log aggregation friendly):");
    println!("   ```json");
    println!(
        "   {{\"timestamp\":\"2024-01-20T10:30:45.123Z\",\"level\":\"DEBUG\",\"target\":\"ccxt_core::http_client\",\"fields\":{{\"message\":\"HTTP request\",\"method\":\"GET\"}}}}"
    );
    println!("   ```");
    println!();

    println!("8. Best practices for logging");
    println!();
    println!("   ✓ Initialize logging system at application startup (beginning of main function)");
    println!("   ✓ Use LogConfig::development() for development");
    println!("   ✓ Use LogConfig::production() for production");
    println!("   ✓ Control verbose logging with verbose flag (ExchangeConfig::verbose)");
    println!(
        "   ✓ Dynamically adjust log levels with environment variables (no recompilation needed)"
    );
    println!("   ✓ Use JSON format in production for easier log analysis tool processing");
    println!("   ✓ Avoid low-level logging in high-frequency loops (performance consideration)");
    println!();

    println!("9. Complete application example code");
    println!();
    println!("```rust");
    println!("use ccxt_core::{{");
    println!("    error::Result,");
    println!("    logging::{{init_logging, LogConfig}},");
    println!("    ExchangeConfig,");
    println!("}};");
    println!("use ccxt_exchanges::binance::Binance;");
    println!();
    println!("#[tokio::main]");
    println!("async fn main() -> Result<()> {{");
    println!("    // 1. Initialize logging system");
    println!("    #[cfg(debug_assertions)]");
    println!("    init_logging(LogConfig::development());");
    println!("    ");
    println!("    #[cfg(not(debug_assertions))]");
    println!("    init_logging(LogConfig::production());");
    println!();
    println!("    // 2. Create exchange configuration");
    println!("    let config = ExchangeConfig {{");
    println!("        api_key: Some(\"YOUR_API_KEY\".to_string()),");
    println!("        secret: Some(\"YOUR_SECRET\".to_string()),");
    println!("        verbose: cfg!(debug_assertions), // Enable verbose logging in development");
    println!("        ..Default::default()");
    println!("    }};");
    println!();
    println!("    // 3. Create exchange instance");
    println!("    let exchange = Binance::new(config)?;");
    println!();
    println!("    // 4. Execute operations (logs automatically recorded)");
    println!(
        "    let ticker = exchange.fetch_ticker(\"BTC/USDT\", ccxt_core::types::TickerParams::default()).await?;"
    );
    println!("    println!(\"Price: {{:?}}\", ticker.last);");
    println!();
    println!("    Ok(())");
    println!("}}");
    println!("```");
    println!();

    println!("=== Example Complete ===");
    println!();
    println!("Tip: Try setting different RUST_LOG values when running this example:");
    println!("  RUST_LOG=debug cargo run --example logging_example");
    println!("  RUST_LOG=ccxt_core=trace cargo run --example logging_example");

    Ok(())
}
