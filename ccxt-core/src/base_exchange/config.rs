//! Exchange configuration structures and builders

use crate::config::{ProxyConfig, RetryPolicy};
use crate::credentials::SecretString;
use serde_json::Value;
use std::collections::HashMap;
use std::time::Duration;

/// Exchange configuration
#[derive(Debug, Clone)]
pub struct ExchangeConfig {
    /// Exchange identifier
    pub id: String,
    /// Exchange display name
    pub name: String,
    /// API key for authentication (automatically zeroed on drop)
    pub api_key: Option<SecretString>,
    /// API secret for authentication (automatically zeroed on drop)
    pub secret: Option<SecretString>,
    /// Password (required by some exchanges, automatically zeroed on drop)
    pub password: Option<SecretString>,
    /// User ID (required by some exchanges)
    pub uid: Option<String>,
    /// Account ID
    pub account_id: Option<String>,
    /// Enable rate limiting
    pub enable_rate_limit: bool,
    /// Rate limit in requests per second
    pub rate_limit: u32,
    /// Request timeout (default: 30 seconds)
    pub timeout: Duration,
    /// TCP connection timeout (default: 10 seconds)
    pub connect_timeout: Duration,
    /// Retry policy
    pub retry_policy: Option<RetryPolicy>,
    /// Enable sandbox/testnet mode
    pub sandbox: bool,
    /// Custom user agent string
    pub user_agent: Option<String>,
    /// HTTP proxy configuration
    pub proxy: Option<ProxyConfig>,
    /// Enable verbose logging
    pub verbose: bool,
    /// Custom exchange-specific options
    pub options: HashMap<String, Value>,
    /// URL overrides for mocking/testing
    pub url_overrides: HashMap<String, String>,
}

impl Default for ExchangeConfig {
    fn default() -> Self {
        Self {
            id: String::new(),
            name: String::new(),
            api_key: None,
            secret: None,
            password: None,
            uid: None,
            account_id: None,
            enable_rate_limit: true,
            rate_limit: 10,
            timeout: Duration::from_secs(30),
            connect_timeout: Duration::from_secs(10),
            retry_policy: None,
            sandbox: false,
            user_agent: Some(format!("ccxt-rust/{}", env!("CARGO_PKG_VERSION"))),
            proxy: None,
            verbose: false,
            options: HashMap::new(),
            url_overrides: HashMap::new(),
        }
    }
}

impl ExchangeConfig {
    /// Create a new configuration builder
    ///
    /// # Example
    ///
    /// ```rust
    /// use ccxt_core::base_exchange::ExchangeConfig;
    ///
    /// let config = ExchangeConfig::builder()
    ///     .id("binance")
    ///     .name("Binance")
    ///     .api_key("your-api-key")
    ///     .secret("your-secret")
    ///     .sandbox(true)
    ///     .build();
    /// ```
    pub fn builder() -> ExchangeConfigBuilder {
        ExchangeConfigBuilder::default()
    }
}

/// Builder for `ExchangeConfig`
///
/// Provides a fluent API for constructing exchange configurations.
///
/// # Example
///
/// ```rust
/// use ccxt_core::base_exchange::ExchangeConfigBuilder;
/// use std::time::Duration;
///
/// let config = ExchangeConfigBuilder::new()
///     .id("binance")
///     .name("Binance")
///     .api_key("your-api-key")
///     .secret("your-secret")
///     .timeout(Duration::from_secs(60))
///     .build();
/// ```
#[derive(Debug, Clone, Default)]
pub struct ExchangeConfigBuilder {
    config: ExchangeConfig,
}

impl ExchangeConfigBuilder {
    /// Create a new builder with default configuration
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the exchange identifier
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.config.id = id.into();
        self
    }

    /// Set the exchange display name
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.config.name = name.into();
        self
    }

    /// Set the API key for authentication
    pub fn api_key(mut self, key: impl Into<String>) -> Self {
        self.config.api_key = Some(SecretString::new(key));
        self
    }

    /// Set the API secret for authentication
    pub fn secret(mut self, secret: impl Into<String>) -> Self {
        self.config.secret = Some(SecretString::new(secret));
        self
    }

    /// Set the password (required by some exchanges)
    pub fn password(mut self, password: impl Into<String>) -> Self {
        self.config.password = Some(SecretString::new(password));
        self
    }

    /// Set the user ID (required by some exchanges)
    pub fn uid(mut self, uid: impl Into<String>) -> Self {
        self.config.uid = Some(uid.into());
        self
    }

    /// Set the account ID
    pub fn account_id(mut self, account_id: impl Into<String>) -> Self {
        self.config.account_id = Some(account_id.into());
        self
    }

    /// Enable or disable rate limiting
    pub fn enable_rate_limit(mut self, enabled: bool) -> Self {
        self.config.enable_rate_limit = enabled;
        self
    }

    /// Set the rate limit in requests per second
    pub fn rate_limit(mut self, rate_limit: u32) -> Self {
        self.config.rate_limit = rate_limit;
        self
    }

    /// Set the request timeout
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.config.timeout = timeout;
        self
    }

    /// Set the TCP connection timeout
    pub fn connect_timeout(mut self, timeout: Duration) -> Self {
        self.config.connect_timeout = timeout;
        self
    }

    /// Set the TCP connection timeout in seconds (convenience method)
    pub fn connect_timeout_secs(mut self, seconds: u64) -> Self {
        self.config.connect_timeout = Duration::from_secs(seconds);
        self
    }

    /// Set the retry policy
    pub fn retry_policy(mut self, policy: RetryPolicy) -> Self {
        self.config.retry_policy = Some(policy);
        self
    }

    /// Enable or disable sandbox/testnet mode
    pub fn sandbox(mut self, enabled: bool) -> Self {
        self.config.sandbox = enabled;
        self
    }

    /// Set a custom user agent string
    pub fn user_agent(mut self, user_agent: impl Into<String>) -> Self {
        self.config.user_agent = Some(user_agent.into());
        self
    }

    /// Set the HTTP proxy configuration
    pub fn proxy(mut self, proxy: ProxyConfig) -> Self {
        self.config.proxy = Some(proxy);
        self
    }

    /// Set the HTTP proxy URL (convenience method)
    pub fn proxy_url(mut self, url: impl Into<String>) -> Self {
        self.config.proxy = Some(ProxyConfig::new(url));
        self
    }

    /// Enable or disable verbose logging
    pub fn verbose(mut self, enabled: bool) -> Self {
        self.config.verbose = enabled;
        self
    }

    /// Set a custom option
    pub fn option(mut self, key: impl Into<String>, value: Value) -> Self {
        self.config.options.insert(key.into(), value);
        self
    }

    /// Set multiple custom options
    pub fn options(mut self, options: HashMap<String, Value>) -> Self {
        self.config.options.extend(options);
        self
    }

    /// Set a URL override for a specific key (e.g., "public", "private")
    pub fn url_override(mut self, key: impl Into<String>, url: impl Into<String>) -> Self {
        self.config.url_overrides.insert(key.into(), url.into());
        self
    }

    /// Build the configuration
    pub fn build(self) -> ExchangeConfig {
        self.config
    }
}
