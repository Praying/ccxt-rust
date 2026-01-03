//! Binance WebSocket Resilience Example
//!
//! This example demonstrates the enhanced WebSocket resilience features:
//!
//! 1. **Exponential Backoff Configuration**: Configure retry delays with jitter
//!    to prevent thundering herd effects during reconnection.
//!
//! 2. **CancellationToken Support**: Gracefully cancel long-running operations
//!    like connect, reconnect, and subscribe.
//!
//! 3. **Event Callback Registration**: Monitor connection lifecycle events
//!    including reconnection attempts, successes, and failures.
//!
//! 4. **Subscription Limits**: Configure maximum subscription count to prevent
//!    resource exhaustion.
//!
//! 5. **Graceful Shutdown**: Clean shutdown with pending operation completion.
//!
//! # Running the Example
//!
//! ```bash
//! cargo run --example binance_websocket_reconnect_example
//! ```
//!
//! # Features Demonstrated
//!
//! - Custom `BackoffConfig` for exponential backoff with jitter
//! - `CancellationToken` for operation cancellation
//! - Event callbacks for connection state monitoring
//! - Subscription capacity management
//! - Graceful shutdown with timeout

use ccxt_core::ws_client::{BackoffConfig, WsClient, WsConfig, WsEvent};
use std::sync::Arc;
use std::time::Duration;
use tokio_util::sync::CancellationToken;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing for observability
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .init();

    println!("=== Binance WebSocket Resilience Example ===\n");

    // Run all examples
    example_backoff_configuration();
    example_cancellation_token().await;
    example_event_callbacks().await;
    example_subscription_limits();
    example_graceful_shutdown().await;
    example_auto_reconnect_coordinator().await;

    println!("\n=== All Examples Complete ===");
    Ok(())
}

/// Example 1: Exponential Backoff Configuration
///
/// Demonstrates how to configure the exponential backoff strategy for
/// reconnection attempts.
fn example_backoff_configuration() {
    println!("--- Example 1: Exponential Backoff Configuration ---\n");

    // Create a custom backoff configuration
    let backoff_config = BackoffConfig {
        // Start with 500ms delay for first retry
        base_delay: Duration::from_millis(500),
        // Cap maximum delay at 30 seconds
        max_delay: Duration::from_secs(30),
        // Add 20% random jitter to prevent thundering herd
        jitter_factor: 0.2,
        // Double the delay for each subsequent attempt
        multiplier: 2.0,
    };

    println!("Custom BackoffConfig:");
    println!("  base_delay: {:?}", backoff_config.base_delay);
    println!("  max_delay: {:?}", backoff_config.max_delay);
    println!("  jitter_factor: {}", backoff_config.jitter_factor);
    println!("  multiplier: {}", backoff_config.multiplier);
    println!();

    // Create WsConfig with custom backoff
    let config = WsConfig {
        url: "wss://stream.binance.com:9443/ws".to_string(),
        backoff_config,
        max_reconnect_attempts: 10,
        ..Default::default()
    };

    println!("WsConfig created with custom backoff:");
    println!("  url: {}", config.url);
    println!(
        "  max_reconnect_attempts: {}",
        config.max_reconnect_attempts
    );
    println!();

    // Show retry delay progression (without jitter for clarity)
    println!("Retry delay progression (without jitter):");
    let strategy = ccxt_core::ws_client::BackoffStrategy::new(BackoffConfig {
        base_delay: Duration::from_millis(500),
        max_delay: Duration::from_secs(30),
        jitter_factor: 0.0, // No jitter for predictable output
        multiplier: 2.0,
    });

    for attempt in 0..8 {
        let delay = strategy.calculate_delay(attempt);
        println!("  Attempt {}: {:?}", attempt, delay);
    }
    println!();
}

/// Example 2: CancellationToken Support
///
/// Demonstrates how to use CancellationToken to cancel long-running
/// WebSocket operations.
async fn example_cancellation_token() {
    println!("--- Example 2: CancellationToken Support ---\n");

    // Create a WebSocket client
    let client = WsClient::new(WsConfig {
        url: "wss://stream.binance.com:9443/ws".to_string(),
        connect_timeout: 5000,
        ..Default::default()
    });

    // Create a cancellation token
    let token = CancellationToken::new();
    println!("Created CancellationToken");

    // Clone the token for different uses
    let connect_token = token.clone();
    let reconnect_token = token.clone();
    println!("Cloned token for connect and reconnect operations");

    // Set the token on the client
    client.set_cancel_token(token.clone()).await;
    println!("Set cancellation token on client");

    // Demonstrate token sharing
    println!("\nToken sharing demonstration:");
    println!("  Original token cancelled: {}", token.is_cancelled());
    println!(
        "  Connect token cancelled: {}",
        connect_token.is_cancelled()
    );
    println!(
        "  Reconnect token cancelled: {}",
        reconnect_token.is_cancelled()
    );

    // Cancel the token
    token.cancel();
    println!("\nAfter cancelling original token:");
    println!("  Original token cancelled: {}", token.is_cancelled());
    println!(
        "  Connect token cancelled: {}",
        connect_token.is_cancelled()
    );
    println!(
        "  Reconnect token cancelled: {}",
        reconnect_token.is_cancelled()
    );

    println!("\nUsage pattern:");
    println!("```rust");
    println!("// Create token and set timeout");
    println!("let token = CancellationToken::new();");
    println!("let token_clone = token.clone();");
    println!();
    println!("// Spawn timeout task");
    println!("tokio::spawn(async move {{");
    println!("    tokio::time::sleep(Duration::from_secs(10)).await;");
    println!("    token_clone.cancel();");
    println!("}});");
    println!();
    println!("// Connect with cancellation support");
    println!("match client.connect_with_cancel(Some(token)).await {{");
    println!("    Ok(()) => println!(\"Connected!\"),");
    println!("    Err(e) if e.as_cancelled().is_some() => println!(\"Cancelled\"),");
    println!("    Err(e) => println!(\"Error: {{}}\", e),");
    println!("}}");
    println!("```");
    println!();
}

/// Example 3: Event Callback Registration
///
/// Demonstrates how to register event callbacks to monitor connection
/// lifecycle events.
async fn example_event_callbacks() {
    println!("--- Example 3: Event Callback Registration ---\n");

    println!("Available WsEvent types:");
    println!("  • WsEvent::Connecting - Connection attempt started");
    println!("  • WsEvent::Connected - Connection established");
    println!("  • WsEvent::Disconnected - Connection closed");
    println!("  • WsEvent::Reconnecting {{ attempt, delay, error }} - Reconnecting");
    println!("  • WsEvent::ReconnectSuccess - Reconnection succeeded");
    println!("  • WsEvent::ReconnectFailed {{ attempt, error, is_permanent }} - Failed");
    println!("  • WsEvent::ReconnectExhausted {{ total_attempts, last_error }} - Exhausted");
    println!("  • WsEvent::SubscriptionRestored - Subscriptions restored");
    println!("  • WsEvent::PermanentError {{ error }} - Permanent error");
    println!("  • WsEvent::Shutdown - Shutdown completed");
    println!();

    // Create a WebSocket client
    let client = WsClient::new(WsConfig::default());

    // Create an event callback
    let callback: Arc<dyn Fn(WsEvent) + Send + Sync> = Arc::new(|event| {
        match &event {
            WsEvent::Connecting => {
                println!("🔵 Event: Connecting...");
            }
            WsEvent::Connected => {
                println!("🟢 Event: Connected!");
            }
            WsEvent::Disconnected => {
                println!("🔴 Event: Disconnected");
            }
            WsEvent::Reconnecting {
                attempt,
                delay,
                error,
            } => {
                println!(
                    "🔄 Event: Reconnecting (attempt {}, delay: {:?}, error: {:?})",
                    attempt, delay, error
                );
            }
            WsEvent::ReconnectSuccess => {
                println!("✅ Event: Reconnection successful!");
            }
            WsEvent::ReconnectFailed {
                attempt,
                error,
                is_permanent,
            } => {
                println!(
                    "❌ Event: Reconnection failed (attempt {}) - {}, permanent: {}",
                    attempt, error, is_permanent
                );
            }
            WsEvent::ReconnectExhausted {
                total_attempts,
                last_error,
            } => {
                println!(
                    "💀 Event: All {} reconnect attempts exhausted - {}",
                    total_attempts, last_error
                );
            }
            WsEvent::SubscriptionRestored => {
                println!("📡 Event: Subscriptions restored");
            }
            WsEvent::PermanentError { error } => {
                println!("🚫 Event: Permanent error (no retry) - {}", error);
            }
            WsEvent::Shutdown => {
                println!("🛑 Event: Shutdown completed");
            }
        }

        // Event helper methods
        if event.is_error() {
            println!("   ⚠️  This is an error event");
        }
        if event.is_terminal() {
            println!("   ⛔ This is a terminal event (no recovery)");
        }
    });

    // Set the callback on the client
    client.set_event_callback(callback).await;
    println!("Event callback registered on client");

    // Demonstrate event creation
    println!("\nSimulating events:");
    let events = vec![
        WsEvent::Connecting,
        WsEvent::Connected,
        WsEvent::Reconnecting {
            attempt: 1,
            delay: Duration::from_secs(2),
            error: Some("Connection reset".to_string()),
        },
        WsEvent::ReconnectSuccess,
        WsEvent::Shutdown,
    ];

    for event in events {
        println!("\n  Event: {}", event);
        println!("    is_connecting: {}", event.is_connecting());
        println!("    is_connected: {}", event.is_connected());
        println!("    is_error: {}", event.is_error());
        println!("    is_terminal: {}", event.is_terminal());
    }
    println!();
}

/// Example 4: Subscription Limits
///
/// Demonstrates how to configure and monitor subscription capacity.
fn example_subscription_limits() {
    println!("--- Example 4: Subscription Limits ---\n");

    // Create client with custom subscription limit
    let client = WsClient::new(WsConfig {
        url: "wss://stream.binance.com:9443/ws".to_string(),
        max_subscriptions: 50, // Limit to 50 subscriptions
        ..Default::default()
    });

    println!("Created client with max_subscriptions: 50");
    println!();

    // Check capacity
    println!("Initial capacity:");
    println!("  subscription_count(): {}", client.subscription_count());
    println!("  remaining_capacity(): {}", client.remaining_capacity());
    println!();

    // Default configuration
    let default_client = WsClient::new(WsConfig::default());
    println!("Default configuration:");
    println!(
        "  max_subscriptions: {} (DEFAULT_MAX_SUBSCRIPTIONS)",
        default_client.remaining_capacity()
    );
    println!();

    println!("Usage pattern:");
    println!("```rust");
    println!("// Check capacity before subscribing");
    println!("if client.remaining_capacity() > 0 {{");
    println!(
        "    client.subscribe(\"ticker\".to_string(), Some(\"BTC/USDT\".to_string()), None).await?;"
    );
    println!("}} else {{");
    println!("    println!(\"No subscription capacity available\");");
    println!("}}");
    println!();
    println!("// Handle ResourceExhausted error");
    println!("match client.subscribe(channel, symbol, params).await {{");
    println!("    Ok(()) => println!(\"Subscribed!\"),");
    println!("    Err(e) if e.as_resource_exhausted().is_some() => {{");
    println!("        println!(\"Max subscriptions reached: {{}}\", e);");
    println!("    }}");
    println!("    Err(e) => println!(\"Error: {{}}\", e),");
    println!("}}");
    println!("```");
    println!();
}

/// Example 5: Graceful Shutdown
///
/// Demonstrates how to perform a graceful shutdown of the WebSocket client.
async fn example_graceful_shutdown() {
    println!("--- Example 5: Graceful Shutdown ---\n");

    // Create client with custom shutdown timeout
    let client = WsClient::new(WsConfig {
        url: "wss://stream.binance.com:9443/ws".to_string(),
        shutdown_timeout: 5000, // 5 second timeout
        ..Default::default()
    });

    println!("Created client with shutdown_timeout: 5000ms");
    println!();

    println!("Shutdown process:");
    println!("  1. Cancel all pending reconnection attempts");
    println!("  2. Send WebSocket close frame to server");
    println!("  3. Wait for pending operations (with timeout)");
    println!("  4. Clear all resources (subscriptions, channels)");
    println!("  5. Emit WsEvent::Shutdown event");
    println!();

    // Set up event callback to monitor shutdown
    let shutdown_received = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let shutdown_flag = shutdown_received.clone();

    client
        .set_event_callback(Arc::new(move |event| {
            if let WsEvent::Shutdown = event {
                shutdown_flag.store(true, std::sync::atomic::Ordering::SeqCst);
                println!("  ✓ Received Shutdown event");
            }
        }))
        .await;

    // Perform shutdown (without connecting first, just to demonstrate)
    println!("Calling client.shutdown()...");
    client.shutdown().await;
    println!("Shutdown completed");
    println!();

    println!("Usage pattern:");
    println!("```rust");
    println!("// Set up shutdown event handler");
    println!("client.set_event_callback(Arc::new(|event| {{");
    println!("    if let WsEvent::Shutdown = event {{");
    println!("        println!(\"Shutdown completed!\");");
    println!("    }}");
    println!("}})).await;");
    println!();
    println!("// Connect and do work...");
    println!("client.connect().await?;");
    println!(
        "client.subscribe(\"ticker\".to_string(), Some(\"BTC/USDT\".to_string()), None).await?;"
    );
    println!();
    println!("// Gracefully shutdown when done");
    println!("client.shutdown().await;");
    println!("```");
    println!();
}

/// Example 6: AutoReconnectCoordinator
///
/// Demonstrates how to use the AutoReconnectCoordinator for automatic
/// reconnection with exponential backoff.
async fn example_auto_reconnect_coordinator() {
    println!("--- Example 6: AutoReconnectCoordinator ---\n");

    // Create a WebSocket client with custom configuration
    let client = Arc::new(WsClient::new(WsConfig {
        url: "wss://stream.binance.com:9443/ws".to_string(),
        backoff_config: BackoffConfig {
            base_delay: Duration::from_secs(1),
            max_delay: Duration::from_secs(60),
            jitter_factor: 0.25,
            multiplier: 2.0,
        },
        max_reconnect_attempts: 5,
        ..Default::default()
    }));

    println!("Created WsClient with custom backoff configuration");
    println!();

    // Create AutoReconnectCoordinator
    let coordinator = client.clone().create_auto_reconnect_coordinator();
    println!("Created AutoReconnectCoordinator");

    // Add event callback
    let coordinator = coordinator.with_callback(Arc::new(|event| {
        println!("  Coordinator event: {}", event);
    }));
    println!("Added event callback to coordinator");

    // Add cancellation token
    let cancel_token = CancellationToken::new();
    let coordinator = coordinator.with_cancel_token(cancel_token.clone());
    println!("Added cancellation token to coordinator");
    println!();

    println!("Coordinator features:");
    println!("  • Monitors connection state automatically");
    println!("  • Triggers reconnection on disconnect/error");
    println!("  • Uses exponential backoff for retry delays");
    println!("  • Classifies errors (transient vs permanent)");
    println!("  • Supports graceful cancellation");
    println!("  • Emits events for all state changes");
    println!();

    println!("Usage pattern:");
    println!("```rust");
    println!("// Create client and coordinator");
    println!("let client = Arc::new(WsClient::new(config));");
    println!("let coordinator = client.clone().create_auto_reconnect_coordinator()");
    println!("    .with_callback(Arc::new(|event| {{");
    println!("        println!(\"Event: {{}}\", event);");
    println!("    }}))");
    println!("    .with_cancel_token(cancel_token.clone());");
    println!();
    println!("// Start automatic reconnection monitoring");
    println!("coordinator.start().await;");
    println!();
    println!("// Connect and subscribe");
    println!("client.connect().await?;");
    println!(
        "client.subscribe(\"ticker\".to_string(), Some(\"BTC/USDT\".to_string()), None).await?;"
    );
    println!();
    println!("// Process messages - coordinator handles reconnection automatically");
    println!("while let Some(msg) = client.receive().await {{");
    println!("    // Process message...");
    println!("}}");
    println!();
    println!("// Stop coordinator when done");
    println!("coordinator.stop().await;");
    println!("```");
    println!();

    // Check coordinator state
    println!("Coordinator state:");
    println!("  is_enabled: {}", coordinator.is_enabled());
    println!();
}
