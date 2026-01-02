//! Structured logging system.
//!
//! Provides tracing-based structured logging with support for:
//! - Multi-level logging (TRACE, DEBUG, INFO, WARN, ERROR)
//! - Structured fields
//! - Environment variable configuration
//! - JSON and formatted output

use tracing::Level;
use tracing_subscriber::{
    EnvFilter, Layer,
    fmt::{self, format::FmtSpan},
    layer::SubscriberExt,
    util::SubscriberInitExt,
};

/// Log level.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    /// Trace level: most detailed debugging information.
    Trace,
    /// Debug level: detailed debugging information.
    Debug,
    /// Info level: important business events.
    Info,
    /// Warn level: potential issues.
    Warn,
    /// Error level: error information.
    Error,
}

impl From<LogLevel> for Level {
    fn from(level: LogLevel) -> Self {
        match level {
            LogLevel::Trace => Level::TRACE,
            LogLevel::Debug => Level::DEBUG,
            LogLevel::Info => Level::INFO,
            LogLevel::Warn => Level::WARN,
            LogLevel::Error => Level::ERROR,
        }
    }
}

impl std::fmt::Display for LogLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LogLevel::Trace => write!(f, "trace"),
            LogLevel::Debug => write!(f, "debug"),
            LogLevel::Info => write!(f, "info"),
            LogLevel::Warn => write!(f, "warn"),
            LogLevel::Error => write!(f, "error"),
        }
    }
}

/// Log format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogFormat {
    /// Human-readable formatted output.
    Pretty,
    /// Compact format.
    Compact,
    /// JSON format for production environments.
    Json,
}

/// Log configuration.
#[derive(Debug, Clone)]
pub struct LogConfig {
    /// Log level.
    pub level: LogLevel,
    /// Log format.
    pub format: LogFormat,
    /// Whether to show timestamps.
    pub show_time: bool,
    /// Whether to show thread IDs.
    pub show_thread_ids: bool,
    /// Whether to show target module.
    pub show_target: bool,
    /// Whether to show span events (function enter/exit).
    pub show_span_events: bool,
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            level: LogLevel::Info,
            format: LogFormat::Pretty,
            show_time: true,
            show_thread_ids: false,
            show_target: true,
            show_span_events: false,
        }
    }
}

impl LogConfig {
    /// Creates a log configuration for development environments.
    pub fn development() -> Self {
        Self {
            level: LogLevel::Debug,
            format: LogFormat::Pretty,
            show_time: true,
            show_thread_ids: false,
            show_target: true,
            show_span_events: true,
        }
    }

    /// Creates a log configuration for production environments.
    pub fn production() -> Self {
        Self {
            level: LogLevel::Info,
            format: LogFormat::Json,
            show_time: true,
            show_thread_ids: true,
            show_target: true,
            show_span_events: false,
        }
    }

    /// Creates a log configuration for test environments.
    pub fn test() -> Self {
        Self {
            level: LogLevel::Warn,
            format: LogFormat::Compact,
            show_time: false,
            show_thread_ids: false,
            show_target: false,
            show_span_events: false,
        }
    }
}

/// Initializes the logging system.
///
/// # Arguments
///
/// * `config` - The logging configuration.
///
/// # Examples
///
/// ```no_run
/// use ccxt_core::logging::{init_logging, LogConfig};
///
/// // Use default configuration
/// init_logging(&LogConfig::default());
///
/// // Or use development configuration
/// init_logging(&LogConfig::development());
/// ```
pub fn init_logging(config: &LogConfig) {
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        EnvFilter::new(format!(
            "ccxt_core={},ccxt_exchanges={}",
            config.level, config.level
        ))
    });

    match config.format {
        LogFormat::Pretty => {
            let fmt_layer = fmt::layer()
                .pretty()
                .with_timer(fmt::time::time())
                .with_thread_ids(config.show_thread_ids)
                .with_target(config.show_target)
                .with_span_events(if config.show_span_events {
                    FmtSpan::ENTER | FmtSpan::CLOSE
                } else {
                    FmtSpan::NONE
                })
                .with_filter(env_filter);

            tracing_subscriber::registry().with(fmt_layer).init();
        }
        LogFormat::Compact => {
            let fmt_layer = fmt::layer()
                .compact()
                .with_timer(fmt::time::time())
                .with_thread_ids(config.show_thread_ids)
                .with_target(config.show_target)
                .with_span_events(if config.show_span_events {
                    FmtSpan::ENTER | FmtSpan::CLOSE
                } else {
                    FmtSpan::NONE
                })
                .with_filter(env_filter);

            tracing_subscriber::registry().with(fmt_layer).init();
        }
        LogFormat::Json => {
            let fmt_layer = fmt::layer()
                .json()
                .with_timer(fmt::time::time())
                .with_thread_ids(config.show_thread_ids)
                .with_target(config.show_target)
                .with_span_events(if config.show_span_events {
                    FmtSpan::ENTER | FmtSpan::CLOSE
                } else {
                    FmtSpan::NONE
                })
                .with_filter(env_filter);

            tracing_subscriber::registry().with(fmt_layer).init();
        }
    }
}

/// Attempts to initialize the logging system, ignoring duplicate initialization errors.
///
/// Suitable for test environments where multiple calls should not panic.
pub fn try_init_logging(config: &LogConfig) {
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        EnvFilter::new(format!(
            "ccxt_core={},ccxt_exchanges={}",
            config.level, config.level
        ))
    });

    let result = match config.format {
        LogFormat::Pretty => {
            let fmt_layer = fmt::layer()
                .pretty()
                .with_timer(fmt::time::time())
                .with_thread_ids(config.show_thread_ids)
                .with_target(config.show_target)
                .with_span_events(if config.show_span_events {
                    FmtSpan::ENTER | FmtSpan::CLOSE
                } else {
                    FmtSpan::NONE
                })
                .with_filter(env_filter);

            tracing_subscriber::registry().with(fmt_layer).try_init()
        }
        LogFormat::Compact => {
            let fmt_layer = fmt::layer()
                .compact()
                .with_timer(fmt::time::time())
                .with_thread_ids(config.show_thread_ids)
                .with_target(config.show_target)
                .with_span_events(if config.show_span_events {
                    FmtSpan::ENTER | FmtSpan::CLOSE
                } else {
                    FmtSpan::NONE
                })
                .with_filter(env_filter);

            tracing_subscriber::registry().with(fmt_layer).try_init()
        }
        LogFormat::Json => {
            let fmt_layer = fmt::layer()
                .json()
                .with_timer(fmt::time::time())
                .with_thread_ids(config.show_thread_ids)
                .with_target(config.show_target)
                .with_span_events(if config.show_span_events {
                    FmtSpan::ENTER | FmtSpan::CLOSE
                } else {
                    FmtSpan::NONE
                })
                .with_filter(env_filter);

            tracing_subscriber::registry().with(fmt_layer).try_init()
        }
    };

    let _ = result;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_level_conversion() {
        assert_eq!(Level::from(LogLevel::Trace), Level::TRACE);
        assert_eq!(Level::from(LogLevel::Debug), Level::DEBUG);
        assert_eq!(Level::from(LogLevel::Info), Level::INFO);
        assert_eq!(Level::from(LogLevel::Warn), Level::WARN);
        assert_eq!(Level::from(LogLevel::Error), Level::ERROR);
    }

    #[test]
    fn test_log_level_display() {
        assert_eq!(LogLevel::Trace.to_string(), "trace");
        assert_eq!(LogLevel::Debug.to_string(), "debug");
        assert_eq!(LogLevel::Info.to_string(), "info");
        assert_eq!(LogLevel::Warn.to_string(), "warn");
        assert_eq!(LogLevel::Error.to_string(), "error");
    }

    #[test]
    fn test_log_config_default() {
        let config = LogConfig::default();
        assert_eq!(config.level, LogLevel::Info);
        assert_eq!(config.format, LogFormat::Pretty);
        assert!(config.show_time);
        assert!(!config.show_thread_ids);
        assert!(config.show_target);
        assert!(!config.show_span_events);
    }

    #[test]
    fn test_log_config_development() {
        let config = LogConfig::development();
        assert_eq!(config.level, LogLevel::Debug);
        assert_eq!(config.format, LogFormat::Pretty);
        assert!(config.show_span_events);
    }

    #[test]
    fn test_log_config_production() {
        let config = LogConfig::production();
        assert_eq!(config.level, LogLevel::Info);
        assert_eq!(config.format, LogFormat::Json);
        assert!(config.show_thread_ids);
    }

    #[test]
    fn test_log_config_test() {
        let config = LogConfig::test();
        assert_eq!(config.level, LogLevel::Warn);
        assert_eq!(config.format, LogFormat::Compact);
        assert!(!config.show_time);
    }

    #[test]
    fn test_try_init_logging() {
        try_init_logging(&LogConfig::test());
        try_init_logging(&LogConfig::test());
    }
}
