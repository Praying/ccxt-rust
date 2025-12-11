//! Logging system integration tests.
//!
//! Tests log configuration, output formatting, and environment variable control.

use ccxt_core::logging::{LogConfig, LogFormat, LogLevel, try_init_logging};
use std::sync::Once;

static INIT: Once = Once::new();

/// Ensure logging system is initialized only once across tests.
fn setup_logging(config: LogConfig) {
    INIT.call_once(|| {
        let _ = try_init_logging(config);
    });
}

#[test]
fn test_log_config_default() {
    let config = LogConfig::default();
    assert_eq!(config.level, LogLevel::Info);
    assert_eq!(config.format, LogFormat::Pretty);
    assert!(config.show_time);
}

#[test]
fn test_log_config_development() {
    let config = LogConfig::development();
    assert_eq!(config.level, LogLevel::Debug);
    assert_eq!(config.format, LogFormat::Pretty);
    assert!(config.show_time);
    assert!(config.show_target);
    assert!(config.show_span_events);
}

#[test]
fn test_log_config_production() {
    let config = LogConfig::production();
    assert_eq!(config.level, LogLevel::Info);
    assert_eq!(config.format, LogFormat::Json);
    assert!(config.show_time);
    assert!(config.show_thread_ids);
}

#[test]
fn test_log_config_test() {
    let config = LogConfig::test();
    assert_eq!(config.level, LogLevel::Warn);
    assert_eq!(config.format, LogFormat::Compact);
    assert!(!config.show_span_events);
}

#[test]
fn test_log_level_conversion() {
    use tracing::Level;

    assert_eq!(Level::from(LogLevel::Trace), Level::TRACE);
    assert_eq!(Level::from(LogLevel::Debug), Level::DEBUG);
    assert_eq!(Level::from(LogLevel::Info), Level::INFO);
    assert_eq!(Level::from(LogLevel::Warn), Level::WARN);
    assert_eq!(Level::from(LogLevel::Error), Level::ERROR);
}

#[test]
fn test_init_logging_success() {
    // Use test configuration to avoid interfering with other tests
    setup_logging(LogConfig::test());

    // Logging system already initialized; subsequent calls should fail silently using try_init_logging
    try_init_logging(LogConfig::test());
}

#[test]
fn test_custom_log_config() {
    let config = LogConfig {
        level: LogLevel::Debug,
        format: LogFormat::Json,
        show_time: true,
        show_thread_ids: false,
        show_target: true,
        show_span_events: false,
    };

    assert_eq!(config.level, LogLevel::Debug);
    assert_eq!(config.format, LogFormat::Json);
    assert!(config.show_time);
    assert!(!config.show_thread_ids);
    assert!(config.show_target);
    assert!(!config.show_span_events);
}

#[test]
fn test_log_output_with_tracing_macros() {
    use tracing::{debug, error, info, warn};

    setup_logging(LogConfig::test());

    // These log calls should not panic
    info!("Test info message");
    warn!("Test warning message");
    error!("Test error message");
    debug!("Test debug message");
}

#[test]
fn test_structured_logging() {
    use tracing::info;

    setup_logging(LogConfig::test());

    // Test structured logging does not panic
    info!(user_id = 123, action = "login", "User action");

    info!(symbol = "BTC/USDT", price = 50000.0, "Price update");
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    use tracing::{debug, error, info, warn};

    #[test]
    fn test_log_levels_hierarchy() {
        setup_logging(LogConfig::test());

        // Test different log levels
        error!("This is an error");
        warn!("This is a warning");
        info!("This is info");
        debug!("This is debug");
    }

    #[test]
    fn test_log_with_context() {
        setup_logging(LogConfig::test());

        info!(
            method = "GET",
            url = "https://api.example.com",
            status = 200,
            "HTTP request completed"
        );
    }

    #[test]
    fn test_error_logging_with_details() {
        setup_logging(LogConfig::test());

        let error_msg = "Connection timeout";
        let retry_count = 3;

        error!(
            error = %error_msg,
            retry_count = retry_count,
            "Request failed after retries"
        );
    }
}
