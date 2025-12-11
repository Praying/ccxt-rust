//! Test configuration management utilities.
//!
//! Provides test environment configuration loading and management, supporting:
//! - Configuration loading from environment variables
//! - Configuration loading from `.env` files
//! - Multi-exchange API credential management
//! - Test data path management
//! - Performance benchmark configuration

use serde::Deserialize;
use std::env;
use std::path::PathBuf;

/// Configuration loading error types.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    /// Environment variable error
    #[error("Environment variable error: {0}")]
    EnvError(#[from] env::VarError),

    /// Configuration parsing error
    #[error("Configuration parsing error: {0}")]
    ParseError(String),

    /// File not found error
    #[error("File not found: {0}")]
    FileNotFound(String),
}

/// Main test configuration structure.
#[derive(Debug, Clone, Deserialize)]
pub struct TestConfig {
    /// Whether to skip private tests
    #[serde(default)]
    pub skip_private_tests: bool,

    /// Whether to enable integration tests
    #[serde(default)]
    pub enable_integration_tests: bool,

    /// Test timeout in milliseconds
    #[serde(default = "default_timeout")]
    pub test_timeout_ms: u64,

    /// Binance exchange configuration
    #[serde(default)]
    pub binance: ExchangeConfig,

    /// OKX exchange configuration
    #[serde(default)]
    pub okx: ExchangeConfig,

    /// Bybit exchange configuration
    #[serde(default)]
    pub bybit: ExchangeConfig,

    /// Kraken exchange configuration
    #[serde(default)]
    pub kraken: ExchangeConfig,

    /// KuCoin exchange configuration
    #[serde(default)]
    pub kucoin: ExchangeConfig,

    /// Hyperliquid exchange configuration
    #[serde(default)]
    pub hyperliquid: ExchangeConfig,

    /// Test data configuration
    #[serde(default)]
    pub test_data: TestDataConfig,

    /// Performance benchmark configuration
    #[serde(default)]
    pub benchmark: BenchmarkConfig,
}

fn default_timeout() -> u64 {
    30000
}

/// Exchange-specific configuration.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct ExchangeConfig {
    /// API key for production environment
    pub api_key: Option<String>,
    /// API secret for production environment
    pub api_secret: Option<String>,
    /// API key for testnet environment
    pub testnet_api_key: Option<String>,
    /// API secret for testnet environment
    pub testnet_api_secret: Option<String>,
    /// Whether to use testnet
    #[serde(default)]
    pub use_testnet: bool,
}

/// Test data configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct TestDataConfig {
    /// Test fixtures directory path
    #[serde(default = "default_fixtures_dir")]
    pub fixtures_dir: String,

    /// Logging level
    #[serde(default = "default_log_level")]
    pub log_level: String,
}

fn default_fixtures_dir() -> String {
    "tests/fixtures".to_string()
}

fn default_log_level() -> String {
    "info".to_string()
}

impl Default for TestDataConfig {
    fn default() -> Self {
        Self {
            fixtures_dir: default_fixtures_dir(),
            log_level: default_log_level(),
        }
    }
}

/// Performance benchmark configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct BenchmarkConfig {
    /// Benchmark sample size
    #[serde(default = "default_sample_size")]
    pub sample_size: usize,

    /// Number of warmup iterations
    #[serde(default = "default_warmup_iterations")]
    pub warmup_iterations: usize,
}

fn default_sample_size() -> usize {
    100
}

fn default_warmup_iterations() -> usize {
    10
}

impl Default for BenchmarkConfig {
    fn default() -> Self {
        Self {
            sample_size: default_sample_size(),
            warmup_iterations: default_warmup_iterations(),
        }
    }
}

impl Default for TestConfig {
    fn default() -> Self {
        Self {
            skip_private_tests: false,
            enable_integration_tests: false,
            test_timeout_ms: default_timeout(),
            binance: ExchangeConfig::default(),
            okx: ExchangeConfig::default(),
            bybit: ExchangeConfig::default(),
            kraken: ExchangeConfig::default(),
            kucoin: ExchangeConfig::default(),
            hyperliquid: ExchangeConfig::default(),
            test_data: TestDataConfig::default(),
            benchmark: BenchmarkConfig::default(),
        }
    }
}

impl TestConfig {
    /// Loads configuration from environment variables with `CCXT_` prefix.
    ///
    /// # Errors
    ///
    /// Returns [`ConfigError`] if exchange configuration loading fails.
    pub fn from_env() -> Result<Self, ConfigError> {
        let mut config = TestConfig::default();

        if let Ok(val) = env::var("CCXT_SKIP_PRIVATE_TESTS") {
            config.skip_private_tests = val.parse().unwrap_or(false);
        }
        if let Ok(val) = env::var("CCXT_ENABLE_INTEGRATION_TESTS") {
            config.enable_integration_tests = val.parse().unwrap_or(false);
        }
        if let Ok(val) = env::var("CCXT_TEST_TIMEOUT_MS") {
            config.test_timeout_ms = val.parse().unwrap_or(default_timeout());
        }

        config.binance = Self::load_exchange_config("BINANCE")?;
        config.okx = Self::load_exchange_config("OKX")?;
        config.bybit = Self::load_exchange_config("BYBIT")?;
        config.kraken = Self::load_exchange_config("KRAKEN")?;
        config.kucoin = Self::load_exchange_config("KUCOIN")?;
        config.hyperliquid = Self::load_exchange_config("HYPERLIQUID")?;

        if let Ok(val) = env::var("CCXT_FIXTURES_DIR") {
            config.test_data.fixtures_dir = val;
        }
        if let Ok(val) = env::var("CCXT_LOG_LEVEL") {
            config.test_data.log_level = val;
        }

        if let Ok(val) = env::var("CCXT_BENCH_SAMPLE_SIZE") {
            config.benchmark.sample_size = val.parse().unwrap_or(default_sample_size());
        }
        if let Ok(val) = env::var("CCXT_BENCH_WARMUP_ITERATIONS") {
            config.benchmark.warmup_iterations = val.parse().unwrap_or(default_warmup_iterations());
        }

        Ok(config)
    }

    /// Loads configuration from a specified `.env` file.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the `.env` file
    ///
    /// # Errors
    ///
    /// Returns [`ConfigError::FileNotFound`] if the file does not exist.
    #[cfg(feature = "test-utils")]
    pub fn from_dotenv(path: &str) -> Result<Self, ConfigError> {
        dotenvy::from_filename(path)
            .map_err(|e| ConfigError::FileNotFound(format!("{}: {}", path, e)))?;
        Self::from_env()
    }

    /// Loads configuration from the default `.env` file.
    ///
    /// # Errors
    ///
    /// Returns [`ConfigError`] if environment variable loading fails.
    #[cfg(feature = "test-utils")]
    pub fn from_default_dotenv() -> Result<Self, ConfigError> {
        dotenvy::dotenv().ok();
        Self::from_env()
    }

    /// Loads configuration for a single exchange from environment variables.
    fn load_exchange_config(exchange: &str) -> Result<ExchangeConfig, ConfigError> {
        let mut config = ExchangeConfig::default();

        let api_key_var = format!("{}_API_KEY", exchange);
        let api_secret_var = format!("{}_API_SECRET", exchange);
        let testnet_key_var = format!("{}_TESTNET_API_KEY", exchange);
        let testnet_secret_var = format!("{}_TESTNET_API_SECRET", exchange);
        let use_testnet_var = format!("{}_USE_TESTNET", exchange);

        config.api_key = env::var(&api_key_var).ok();
        config.api_secret = env::var(&api_secret_var).ok();
        config.testnet_api_key = env::var(&testnet_key_var).ok();
        config.testnet_api_secret = env::var(&testnet_secret_var).ok();

        if let Ok(val) = env::var(&use_testnet_var) {
            config.use_testnet = val.parse().unwrap_or(false);
        }

        Ok(config)
    }

    /// Checks whether private tests should be skipped.
    pub fn should_skip_private_tests(&self) -> bool {
        self.skip_private_tests
    }

    /// Checks whether integration tests are enabled.
    pub fn is_integration_enabled(&self) -> bool {
        self.enable_integration_tests
    }

    /// Checks whether Binance credentials are available.
    pub fn has_binance_credentials(&self) -> bool {
        self.binance.has_credentials()
    }

    /// Checks whether OKX credentials are available.
    pub fn has_okx_credentials(&self) -> bool {
        self.okx.has_credentials()
    }

    /// Checks whether Bybit credentials are available.
    pub fn has_bybit_credentials(&self) -> bool {
        self.bybit.has_credentials()
    }

    /// Checks whether Kraken credentials are available.
    pub fn has_kraken_credentials(&self) -> bool {
        self.kraken.has_credentials()
    }

    /// Checks whether KuCoin credentials are available.
    pub fn has_kucoin_credentials(&self) -> bool {
        self.kucoin.has_credentials()
    }

    /// Checks whether Hyperliquid credentials are available.
    pub fn has_hyperliquid_credentials(&self) -> bool {
        self.hyperliquid.has_credentials()
    }

    /// Gets the active API credentials for the specified exchange.
    ///
    /// # Arguments
    ///
    /// * `exchange` - Exchange name (case-insensitive)
    ///
    /// # Returns
    ///
    /// Returns `Some((api_key, api_secret))` if credentials exist, `None` otherwise.
    pub fn get_active_api_key(&self, exchange: &str) -> Option<(String, String)> {
        let config = match exchange.to_lowercase().as_str() {
            "binance" => &self.binance,
            "okx" => &self.okx,
            "bybit" => &self.bybit,
            "kraken" => &self.kraken,
            "kucoin" => &self.kucoin,
            "hyperliquid" => &self.hyperliquid,
            _ => return None,
        };

        config.get_active_credentials()
    }

    /// Constructs the path to a test fixture file.
    ///
    /// # Arguments
    ///
    /// * `category` - Fixture category subdirectory
    /// * `filename` - Fixture filename
    ///
    /// # Returns
    ///
    /// Returns a [`PathBuf`] to the fixture file.
    pub fn get_fixture_path(&self, category: &str, filename: &str) -> PathBuf {
        PathBuf::from(&self.test_data.fixtures_dir)
            .join(category)
            .join(filename)
    }
}

impl ExchangeConfig {
    /// Checks whether any credentials are available.
    pub fn has_credentials(&self) -> bool {
        if self.use_testnet {
            self.testnet_api_key.is_some() && self.testnet_api_secret.is_some()
        } else {
            self.api_key.is_some() && self.api_secret.is_some()
        }
    }

    /// Gets the active credentials based on the `use_testnet` flag.
    ///
    /// # Returns
    ///
    /// Returns `Some((api_key, api_secret))` if credentials exist, `None` otherwise.
    pub fn get_active_credentials(&self) -> Option<(String, String)> {
        if self.use_testnet {
            match (&self.testnet_api_key, &self.testnet_api_secret) {
                (Some(key), Some(secret)) => Some((key.clone(), secret.clone())),
                _ => None,
            }
        } else {
            match (&self.api_key, &self.api_secret) {
                (Some(key), Some(secret)) => Some((key.clone(), secret.clone())),
                _ => None,
            }
        }
    }
}

/// Conditionally skips a test based on a condition.
///
/// # Examples
///
/// ```ignore
/// // Version 1: With explicit config and condition
/// skip_if!(config, config.skip_private_tests, "Private tests disabled");
///
/// // Version 2: Simplified version for private tests
/// skip_if!(private_tests);
/// ```
#[macro_export]
macro_rules! skip_if {
    ($config:expr, $condition:expr, $reason:expr) => {
        if $condition {
            println!("SKIPPED: {}", $reason);
            return;
        }
    };

    (private_tests) => {{
        #[cfg(feature = "test-utils")]
        {
            let config = $crate::test_config::TestConfig::from_default_dotenv().unwrap_or_default();
            if config.should_skip_private_tests() {
                println!("SKIPPED: Private tests are disabled");
                return;
            }
        }
        #[cfg(not(feature = "test-utils"))]
        {
            println!("SKIPPED: test-utils feature not enabled");
            return;
        }
    }};
}

/// Requires exchange credentials to run a test.
///
/// Skips the test if credentials are not available for the specified exchange.
///
/// # Examples
///
/// ```ignore
/// // Version 1: With explicit config
/// require_credentials!(config, binance);
///
/// // Version 2: Simplified version with auto-loading
/// require_credentials!(binance);
/// ```
#[macro_export]
macro_rules! require_credentials {
    ($config:expr, $exchange:ident) => {
        paste::paste! {
            if !$config.[<has_ $exchange _credentials>]() {
                println!("SKIPPED: No {} credentials", stringify!($exchange));
                return;
            }
        }
    };

    ($exchange:ident) => {{
        #[cfg(feature = "test-utils")]
        {
            let config = $crate::test_config::TestConfig::from_default_dotenv().unwrap_or_default();
            paste::paste! {
                if !config.[<has_ $exchange _credentials>]() {
                    println!("SKIPPED: No {} credentials", stringify!($exchange));
                    return;
                }
            }
        }
        #[cfg(not(feature = "test-utils"))]
        {
            println!("SKIPPED: test-utils feature not enabled");
            return;
        }
    }};
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = TestConfig::default();
        assert!(!config.skip_private_tests);
        assert!(!config.enable_integration_tests);
        assert_eq!(config.test_timeout_ms, 30000);
        assert_eq!(config.test_data.fixtures_dir, "tests/fixtures");
        assert_eq!(config.benchmark.sample_size, 100);
    }

    #[test]
    fn test_exchange_config_no_credentials() {
        let config = ExchangeConfig::default();
        assert!(!config.has_credentials());
        assert!(config.get_active_credentials().is_none());
    }

    #[test]
    fn test_exchange_config_with_credentials() {
        let config = ExchangeConfig {
            api_key: Some("test_key".to_string()),
            api_secret: Some("test_secret".to_string()),
            testnet_api_key: None,
            testnet_api_secret: None,
            use_testnet: false,
        };

        assert!(config.has_credentials());
        let (key, secret) = config.get_active_credentials().unwrap();
        assert_eq!(key, "test_key");
        assert_eq!(secret, "test_secret");
    }

    #[test]
    fn test_exchange_config_testnet() {
        let config = ExchangeConfig {
            api_key: Some("prod_key".to_string()),
            api_secret: Some("prod_secret".to_string()),
            testnet_api_key: Some("test_key".to_string()),
            testnet_api_secret: Some("test_secret".to_string()),
            use_testnet: true,
        };

        assert!(config.has_credentials());
        let (key, secret) = config.get_active_credentials().unwrap();
        assert_eq!(key, "test_key");
        assert_eq!(secret, "test_secret");
    }

    #[test]
    fn test_fixture_path() {
        let config = TestConfig::default();
        let path = config.get_fixture_path("tickers", "binance_btcusdt.json");
        assert_eq!(
            path.to_str().unwrap(),
            "tests/fixtures/tickers/binance_btcusdt.json"
        );
    }

    #[test]
    fn test_from_env_with_defaults() {
        // 测试在没有环境变量时使用默认值
        let config = TestConfig::from_env().unwrap();
        assert_eq!(config.test_timeout_ms, 30000);
        assert_eq!(config.test_data.fixtures_dir, "tests/fixtures");
    }
}
