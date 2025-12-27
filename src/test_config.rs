//! Test configuration management.
//!
//! Provides environment variable-based test configuration with support for:
//! - API key management for multiple exchanges
//! - Test control flags
//! - Test data configuration
//! - Logging level configuration

use ccxt_core::serde::Deserialize;
use std::env;

/// Test configuration loaded from environment variables.
#[derive(Debug, Clone, Deserialize)]
#[allow(clippy::unsafe_derive_deserialize)]
pub struct TestConfig {
    /// Enable private API tests (requires API credentials).
    #[serde(default)]
    pub enable_private_tests: bool,

    /// Enable integration tests.
    #[serde(default)]
    pub enable_integration_tests: bool,

    /// Test timeout in seconds.
    #[serde(default = "default_timeout")]
    pub test_timeout_seconds: u64,

    /// Binance exchange configuration.
    #[serde(default)]
    pub binance: ExchangeConfig,

    /// OKX exchange configuration (reserved for future use).
    #[serde(default)]
    pub okx: ExchangeConfig,
    /// Bybit exchange configuration (reserved for future use).
    #[serde(default)]
    pub bybit: ExchangeConfig,
    /// Bitget exchange configuration (reserved for future use).
    #[serde(default)]
    pub bitget: ExchangeConfig,

    /// Test data configuration.
    #[serde(default)]
    pub test: TestDataConfig,

    /// Logging level for `RUST_LOG` environment variable.
    #[serde(default = "default_log_level")]
    pub rust_log: String,
}

/// Exchange-specific configuration.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct ExchangeConfig {
    /// API key for exchange authentication.
    pub api_key: Option<String>,
    /// API secret for exchange authentication.
    pub api_secret: Option<String>,
    /// Use testnet instead of production environment.
    #[serde(default)]
    pub use_testnet: bool,
    /// Custom API endpoint URL (overrides default).
    pub api_endpoint: Option<String>,
}

/// Test data configuration for trading operations.
#[derive(Debug, Clone, Deserialize)]
pub struct TestDataConfig {
    /// Trading pair symbol for tests (e.g., "BTC/USDT").
    #[serde(default = "default_test_symbol")]
    pub symbol: String,
    /// Use mock data instead of live API calls.
    #[serde(default)]
    pub use_mock_data: bool,
    /// Order amount for test orders.
    #[serde(default = "default_test_amount")]
    pub amount: f64,
    /// Price offset percentage for test orders.
    #[serde(default = "default_price_offset")]
    pub price_offset_percent: f64,
}

impl Default for TestDataConfig {
    fn default() -> Self {
        Self {
            symbol: default_test_symbol(),
            use_mock_data: false,
            amount: default_test_amount(),
            price_offset_percent: default_price_offset(),
        }
    }
}

// Serde default value functions
fn default_timeout() -> u64 {
    30
}

fn default_log_level() -> String {
    "info".to_string()
}

fn default_test_symbol() -> String {
    "BTC/USDT".to_string()
}

fn default_test_amount() -> f64 {
    0.001
}

fn default_price_offset() -> f64 {
    5.0
}

impl TestConfig {
    /// Load configuration from environment variables.
    ///
    /// Attempts to load `.env` file automatically if present.
    ///
    /// # Errors
    ///
    /// Returns an error if environment variables cannot be deserialized into configuration.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ccxt_rust::test_config::TestConfig;
    ///
    /// dotenvy::dotenv().ok();
    /// let config = TestConfig::from_env().unwrap();
    /// ```
    pub fn from_env() -> Result<Self, envy::Error> {
        dotenvy::dotenv().ok();
        envy::from_env::<Self>()
    }

    /// Load configuration from a specific `.env` file.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the `.env` file
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be loaded or parsed.
    pub fn from_dotenv(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        dotenvy::from_filename(path)?;
        Ok(Self::from_env()?)
    }

    /// Check if private API tests should be skipped.
    ///
    /// # Returns
    ///
    /// `true` if private tests are disabled, `false` otherwise.
    #[must_use]
    pub fn should_skip_private_tests(&self) -> bool {
        !self.enable_private_tests
    }

    /// Check if integration tests should be skipped.
    ///
    /// # Returns
    ///
    /// `true` if integration tests are disabled, `false` otherwise.
    #[must_use]
    pub fn should_skip_integration_tests(&self) -> bool {
        !self.enable_integration_tests
    }

    /// Get Binance API credentials if configured.
    ///
    /// # Returns
    ///
    /// `Some((api_key, api_secret))` if both credentials are present, `None` otherwise.
    #[must_use]
    pub fn binance_credentials(&self) -> Option<(&str, &str)> {
        match (&self.binance.api_key, &self.binance.api_secret) {
            (Some(key), Some(secret)) => Some((key.as_str(), secret.as_str())),
            _ => None,
        }
    }

    /// Check if Binance credentials are configured.
    ///
    /// # Returns
    ///
    /// `true` if both API key and secret are present, `false` otherwise.
    #[must_use]
    pub fn has_binance_credentials(&self) -> bool {
        self.binance_credentials().is_some()
    }

    /// Initialize the tracing logging system.
    ///
    /// Sets `RUST_LOG` environment variable if not already set, then initializes
    /// the tracing subscriber.
    pub fn init_logging(&self) {
        if env::var("RUST_LOG").is_err() {
            unsafe {
                env::set_var("RUST_LOG", &self.rust_log);
            }
        }
        tracing_subscriber::fmt::init();
    }
}

/// Conditionally skip a test based on a runtime condition.
///
/// Prints a warning message and returns early from the test function if the condition is true.
///
/// # Examples
///
/// ```no_run
/// use ccxt_rust::skip_if;
/// # use ccxt_rust::test_config::TestConfig;
///
/// #[tokio::test]
/// async fn test_private_api() {
///     let config = TestConfig::from_env().unwrap();
///     skip_if!(config.should_skip_private_tests(), "Private API tests disabled");
/// }
/// ```
#[macro_export]
macro_rules! skip_if {
    ($condition:expr, $reason:expr) => {
        if $condition {
            println!("⚠️  Skipping test: {}", $reason);
            return;
        }
    };
}

/// Skip test if required exchange credentials are not configured.
///
/// Dynamically checks for credentials using the `has_{exchange}_credentials()` method.
///
/// # Examples
///
/// ```no_run
/// use ccxt_rust::require_credentials;
/// # use ccxt_rust::test_config::TestConfig;
///
/// #[tokio::test]
/// async fn test_binance_private() {
///     let config = TestConfig::from_env().unwrap();
///     require_credentials!(config, binance);
/// }
/// ```
#[macro_export]
macro_rules! require_credentials {
    ($config:expr, $exchange:ident) => {
        if !$config.paste::paste! { [<has_ $exchange _credentials>] }() {
            println!(
                "⚠️  Skipping test: Missing {} API credentials",
                stringify!($exchange).to_uppercase()
            );
            return;
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = TestConfig {
            enable_private_tests: false,
            enable_integration_tests: false,
            test_timeout_seconds: default_timeout(),
            binance: ExchangeConfig::default(),
            okx: ExchangeConfig::default(),
            bybit: ExchangeConfig::default(),
            bitget: ExchangeConfig::default(),
            test: TestDataConfig::default(),
            rust_log: default_log_level(),
        };

        assert!(!config.enable_private_tests);
        assert!(!config.enable_integration_tests);
        assert_eq!(config.test_timeout_seconds, 30);
        assert_eq!(config.test.symbol, "BTC/USDT");
        assert_eq!(config.rust_log, "info");
    }

    #[test]
    fn test_exchange_config_default() {
        let config = ExchangeConfig::default();
        assert!(config.api_key.is_none());
        assert!(config.api_secret.is_none());
        assert!(!config.use_testnet);
    }

    #[test]
    fn test_has_credentials() {
        let mut config = TestConfig {
            enable_private_tests: true,
            enable_integration_tests: false,
            test_timeout_seconds: 30,
            binance: ExchangeConfig {
                api_key: Some("test_key".to_string()),
                api_secret: Some("test_secret".to_string()),
                use_testnet: true,
                api_endpoint: None,
            },
            okx: ExchangeConfig::default(),
            bybit: ExchangeConfig::default(),
            bitget: ExchangeConfig::default(),
            test: TestDataConfig::default(),
            rust_log: "debug".to_string(),
        };

        assert!(config.has_binance_credentials());
        assert_eq!(
            config.binance_credentials(),
            Some(("test_key", "test_secret"))
        );

        config.binance.api_key = None;
        assert!(!config.has_binance_credentials());
    }

    #[test]
    fn test_should_skip_tests() {
        let config = TestConfig {
            enable_private_tests: false,
            enable_integration_tests: true,
            test_timeout_seconds: 30,
            binance: ExchangeConfig::default(),
            okx: ExchangeConfig::default(),
            bybit: ExchangeConfig::default(),
            bitget: ExchangeConfig::default(),
            test: TestDataConfig::default(),
            rust_log: "info".to_string(),
        };

        assert!(config.should_skip_private_tests());
        assert!(!config.should_skip_integration_tests());
    }
}
