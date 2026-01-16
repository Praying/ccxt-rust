//! HTTP client abstraction layer
//!
//! Provides a unified HTTP request interface with support for:
//! - Automatic retry mechanism with configurable strategies
//! - Timeout control per request
//! - Request and response size limits for DoS protection
//! - Circuit breaker integration for resilience
//! - Rate limiting integration
//!
//! # Example
//!
//! ```rust,no_run
//! use ccxt_core::http_client::{HttpClient, HttpConfig};
//!
//! let config = HttpConfig::default();
//! let client = HttpClient::new(config)?;
//!
//! let response = client.get("https://example.com/api", None).await?;
//! ```
//!
//! # Features
//!
//! - **Retry**: Automatic retry with exponential backoff for transient failures
//! - **Circuit Breaker**: Prevents cascading failures by blocking requests to failing endpoints
//! - **Rate Limiting**: Optional rate limiting for API quota management
//! - **Size Limits**: Protection against oversized requests/responses

mod builder;
mod config;
mod headers;
mod request;
mod response;
mod retry;

#[cfg(test)]
mod tests;

pub use builder::HttpClient;
pub use config::HttpConfig;
