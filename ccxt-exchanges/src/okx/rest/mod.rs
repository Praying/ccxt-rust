//! OKX REST API implementation.
//!
//! Implements all REST API endpoint operations for the OKX exchange.

mod account;
mod market_data;
mod trading;

use super::{Okx, OkxAuth};
use ccxt_core::{Error, Result};
use reqwest::header::{HeaderMap, HeaderValue};
use serde_json::Value;
use std::collections::HashMap;
use tracing::debug;

impl Okx {
    /// Get the current timestamp in ISO 8601 format for OKX API.
    ///
    /// # Deprecated
    ///
    /// This method is deprecated. Use [`signed_request()`](Self::signed_request) instead.
    /// The `signed_request()` builder handles timestamp generation internally.
    #[deprecated(
        since = "0.1.0",
        note = "Use `signed_request()` builder instead which handles timestamps internally"
    )]
    #[allow(dead_code)]
    fn get_timestamp() -> String {
        chrono::Utc::now()
            .format("%Y-%m-%dT%H:%M:%S%.3fZ")
            .to_string()
    }

    /// Get the authentication instance if credentials are configured.
    pub fn get_auth(&self) -> Result<OkxAuth> {
        let config = &self.base().config;

        let api_key = config
            .api_key
            .as_ref()
            .ok_or_else(|| Error::authentication("API key is required"))?;
        let secret = config
            .secret
            .as_ref()
            .ok_or_else(|| Error::authentication("API secret is required"))?;
        let passphrase = config
            .password
            .as_ref()
            .ok_or_else(|| Error::authentication("Passphrase is required"))?;

        Ok(OkxAuth::new(
            api_key.expose_secret().to_string(),
            secret.expose_secret().to_string(),
            passphrase.expose_secret().to_string(),
        ))
    }

    /// Check that required credentials are configured.
    pub fn check_required_credentials(&self) -> Result<()> {
        self.base().check_required_credentials()?;
        if self.base().config.password.is_none() {
            return Err(Error::authentication("Passphrase is required for OKX"));
        }
        Ok(())
    }

    /// Build the API path for OKX V5 API.
    pub(crate) fn build_api_path(endpoint: &str) -> String {
        format!("/api/v5{}", endpoint)
    }

    /// Get the instrument type for API requests.
    ///
    /// Maps the configured `default_type` to OKX's instrument type (instType) parameter.
    /// OKX uses a unified V5 API, so this primarily affects market filtering.
    ///
    /// # Returns
    ///
    /// Returns the OKX instrument type string:
    /// - "SPOT" for spot trading
    /// - "MARGIN" for margin trading
    /// - "SWAP" for perpetual contracts
    /// - "FUTURES" for delivery contracts
    /// - "OPTION" for options trading
    pub fn get_inst_type(&self) -> &str {
        use ccxt_core::types::default_type::DefaultType;

        match self.options().default_type {
            DefaultType::Spot => "SPOT",
            DefaultType::Margin => "MARGIN",
            DefaultType::Swap => "SWAP",
            DefaultType::Futures => "FUTURES",
            DefaultType::Option => "OPTION",
        }
    }

    /// Make a public API request (no authentication required).
    pub(crate) async fn public_request(
        &self,
        method: &str,
        path: &str,
        params: Option<&HashMap<String, String>>,
    ) -> Result<Value> {
        let urls = self.urls();
        let mut url = format!("{}{}", urls.rest, path);

        if let Some(p) = params {
            if !p.is_empty() {
                let query: Vec<String> = p
                    .iter()
                    .map(|(k, v)| format!("{}={}", k, urlencoding::encode(v)))
                    .collect();
                url = format!("{}?{}", url, query.join("&"));
            }
        }

        debug!("OKX public request: {} {}", method, url);

        let mut headers = HeaderMap::new();
        if self.is_testnet_trading() {
            headers.insert("x-simulated-trading", HeaderValue::from_static("1"));
        }

        let response = match method.to_uppercase().as_str() {
            "GET" => {
                if headers.is_empty() {
                    self.base().http_client.get(&url, None).await?
                } else {
                    self.base().http_client.get(&url, Some(headers)).await?
                }
            }
            "POST" => {
                if headers.is_empty() {
                    self.base().http_client.post(&url, None, None).await?
                } else {
                    self.base()
                        .http_client
                        .post(&url, Some(headers), None)
                        .await?
                }
            }
            _ => {
                return Err(Error::invalid_request(format!(
                    "Unsupported HTTP method: {}",
                    method
                )));
            }
        };

        if super::error::is_error_response(&response) {
            return Err(super::error::parse_error(&response));
        }

        Ok(response)
    }

    /// Make a private API request (authentication required).
    ///
    /// # Deprecated
    ///
    /// This method is deprecated. Use [`signed_request()`](Self::signed_request) instead.
    /// The `signed_request()` builder provides a cleaner, more maintainable API for
    /// constructing authenticated requests.
    #[deprecated(
        since = "0.1.0",
        note = "Use `signed_request()` builder instead for cleaner, more maintainable code"
    )]
    #[allow(dead_code)]
    #[allow(deprecated)]
    async fn private_request(
        &self,
        method: &str,
        path: &str,
        params: Option<&HashMap<String, String>>,
        body: Option<&Value>,
    ) -> Result<Value> {
        self.check_required_credentials()?;

        let auth = self.get_auth()?;
        let urls = self.urls();
        let timestamp = Self::get_timestamp();

        let query_string = if let Some(p) = params {
            if p.is_empty() {
                String::new()
            } else {
                let query: Vec<String> = p
                    .iter()
                    .map(|(k, v)| format!("{}={}", k, urlencoding::encode(v)))
                    .collect();
                format!("?{}", query.join("&"))
            }
        } else {
            String::new()
        };

        let body_string = body
            .map(|b| serde_json::to_string(b).unwrap_or_default())
            .unwrap_or_default();

        let sign_path = format!("{}{}", path, query_string);
        let signature = auth.sign(&timestamp, method, &sign_path, &body_string);

        let mut headers = HeaderMap::new();
        auth.add_auth_headers(&mut headers, &timestamp, &signature);
        headers.insert("Content-Type", HeaderValue::from_static("application/json"));

        if self.is_testnet_trading() {
            headers.insert("x-simulated-trading", HeaderValue::from_static("1"));
        }

        let url = format!("{}{}{}", urls.rest, path, query_string);
        debug!("OKX private request: {} {}", method, url);

        let response = match method.to_uppercase().as_str() {
            "GET" => self.base().http_client.get(&url, Some(headers)).await?,
            "POST" => {
                let body_value = body.cloned();
                self.base()
                    .http_client
                    .post(&url, Some(headers), body_value)
                    .await?
            }
            "DELETE" => {
                self.base()
                    .http_client
                    .delete(&url, Some(headers), None)
                    .await?
            }
            _ => {
                return Err(Error::invalid_request(format!(
                    "Unsupported HTTP method: {}",
                    method
                )));
            }
        };

        if super::error::is_error_response(&response) {
            return Err(super::error::parse_error(&response));
        }

        Ok(response)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::disallowed_methods)]
    use super::*;

    #[test]
    fn test_build_api_path() {
        let _okx = Okx::builder().build().unwrap();
        let path = Okx::build_api_path("/public/instruments");
        assert_eq!(path, "/api/v5/public/instruments");
    }

    #[test]
    fn test_get_inst_type_spot() {
        let okx = Okx::builder().build().unwrap();
        let inst_type = okx.get_inst_type();
        assert_eq!(inst_type, "SPOT");
    }

    #[test]
    fn test_get_inst_type_margin() {
        use ccxt_core::types::default_type::DefaultType;
        let okx = Okx::builder()
            .default_type(DefaultType::Margin)
            .build()
            .unwrap();
        let inst_type = okx.get_inst_type();
        assert_eq!(inst_type, "MARGIN");
    }

    #[test]
    fn test_get_inst_type_swap() {
        use ccxt_core::types::default_type::DefaultType;
        let okx = Okx::builder()
            .default_type(DefaultType::Swap)
            .build()
            .unwrap();
        let inst_type = okx.get_inst_type();
        assert_eq!(inst_type, "SWAP");
    }

    #[test]
    fn test_get_inst_type_futures() {
        use ccxt_core::types::default_type::DefaultType;
        let okx = Okx::builder()
            .default_type(DefaultType::Futures)
            .build()
            .unwrap();
        let inst_type = okx.get_inst_type();
        assert_eq!(inst_type, "FUTURES");
    }

    #[test]
    fn test_get_inst_type_option() {
        use ccxt_core::types::default_type::DefaultType;
        let okx = Okx::builder()
            .default_type(DefaultType::Option)
            .build()
            .unwrap();
        let inst_type = okx.get_inst_type();
        assert_eq!(inst_type, "OPTION");
    }

    #[test]
    #[allow(deprecated)]
    fn test_get_timestamp() {
        let _okx = Okx::builder().build().unwrap();
        let ts = Okx::get_timestamp();

        assert!(ts.contains("T"));
        assert!(ts.contains("Z"));
        assert!(ts.len() > 20);
    }
}
