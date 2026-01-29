use crate::circuit_breaker::CircuitBreaker;
use crate::error::{Error, Result};
use crate::rate_limiter::RateLimiter;
use crate::retry_strategy::RetryStrategy;
use reqwest::Client;
use std::sync::Arc;

use super::config::HttpConfig;

/// HTTP client with retry and rate limiting support
#[derive(Debug, Clone)]
pub struct HttpClient {
    client: Client,
    config: HttpConfig,
    rate_limiter: Option<RateLimiter>,
    retry_strategy: RetryStrategy,
    circuit_breaker: Option<Arc<CircuitBreaker>>,
}

impl HttpClient {
    /// Creates a new HTTP client with the given configuration.
    ///
    /// # Arguments
    ///
    /// * `config` - HTTP client configuration
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing the initialized client or an error if client creation fails.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The proxy URL is invalid
    /// - The HTTP client cannot be built
    pub fn new(config: HttpConfig) -> Result<Self> {
        let mut builder = Client::builder()
            .timeout(config.timeout)
            .connect_timeout(config.connect_timeout)
            .pool_max_idle_per_host(config.pool_max_idle_per_host)
            .pool_idle_timeout(config.pool_idle_timeout)
            .gzip(true)
            .user_agent(&config.user_agent);

        if let Some(proxy_config) = &config.proxy {
            let mut proxy = reqwest::Proxy::all(&proxy_config.url)
                .map_err(|e| Error::network(format!("Invalid proxy URL: {e}")))?;

            if let (Some(username), Some(password)) =
                (&proxy_config.username, &proxy_config.password)
            {
                proxy = proxy.basic_auth(username, password);
            }
            builder = builder.proxy(proxy);
        }

        let client = builder
            .build()
            .map_err(|e| Error::network(format!("Failed to build HTTP client: {e}")))?;

        let retry_strategy = RetryStrategy::new(config.retry_config.clone().unwrap_or_default());

        let circuit_breaker = config
            .circuit_breaker
            .as_ref()
            .map(|cb_config| Arc::new(CircuitBreaker::new(cb_config.clone())));

        Ok(Self {
            client,
            config,
            rate_limiter: None,
            retry_strategy,
            circuit_breaker,
        })
    }

    /// Creates a new HTTP client with a custom rate limiter.
    ///
    /// # Arguments
    ///
    /// * `config` - HTTP client configuration
    /// * `rate_limiter` - Pre-configured rate limiter instance
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing the initialized client with rate limiter.
    ///
    /// # Errors
    ///
    /// Returns an error if client creation fails.
    pub fn new_with_rate_limiter(config: HttpConfig, rate_limiter: RateLimiter) -> Result<Self> {
        let mut client = Self::new(config)?;
        client.rate_limiter = Some(rate_limiter);
        Ok(client)
    }

    /// Sets the rate limiter for this client.
    ///
    /// # Arguments
    ///
    /// * `rate_limiter` - Rate limiter instance to use
    pub fn set_rate_limiter(&mut self, rate_limiter: RateLimiter) {
        self.rate_limiter = Some(rate_limiter);
    }

    /// Sets the retry strategy for this client.
    ///
    /// # Arguments
    ///
    /// * `strategy` - Retry strategy to use for failed requests
    pub fn set_retry_strategy(&mut self, strategy: RetryStrategy) {
        self.retry_strategy = strategy;
    }

    /// Sets the circuit breaker for this client.
    ///
    /// # Arguments
    ///
    /// * `circuit_breaker` - Circuit breaker instance to use
    ///
    /// # Example
    ///
    /// ```rust
    /// use ccxt_core::http_client::{HttpClient, HttpConfig};
    /// use ccxt_core::circuit_breaker::{CircuitBreaker, CircuitBreakerConfig};
    /// use std::sync::Arc;
    ///
    /// let mut client = HttpClient::new(HttpConfig::default()).unwrap();
    /// let cb = Arc::new(CircuitBreaker::new(CircuitBreakerConfig::default()));
    /// client.set_circuit_breaker(cb);
    /// ```
    pub fn set_circuit_breaker(&mut self, circuit_breaker: Arc<CircuitBreaker>) {
        self.circuit_breaker = Some(circuit_breaker);
    }

    /// Returns a reference to the circuit breaker, if configured.
    ///
    /// This can be used to check the circuit breaker state or manually
    /// record success/failure.
    pub fn circuit_breaker(&self) -> Option<&Arc<CircuitBreaker>> {
        self.circuit_breaker.as_ref()
    }

    /// Returns a reference to current HTTP configuration.
    pub fn config(&self) -> &HttpConfig {
        &self.config
    }

    /// Updates the HTTP configuration.
    ///
    /// # Arguments
    ///
    /// * `config` - New configuration to use
    pub fn set_config(&mut self, config: HttpConfig) {
        self.config = config;
    }

    /// Internal: Returns reference to the underlying reqwest client.
    pub(crate) fn client(&self) -> &Client {
        &self.client
    }

    /// Internal: Returns reference to the rate limiter, if configured.
    pub(crate) fn rate_limiter(&self) -> Option<&RateLimiter> {
        self.rate_limiter.as_ref()
    }

    /// Internal: Returns reference to the retry strategy.
    pub(crate) fn retry_strategy(&self) -> &RetryStrategy {
        &self.retry_strategy
    }
}
