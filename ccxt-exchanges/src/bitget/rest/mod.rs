//! Bitget REST API implementation.
//!
//! Implements all REST API endpoint operations for the Bitget exchange.

mod account;
pub mod futures;
mod market_data;
mod trading;

use super::{Bitget, BitgetAuth, error};
use ccxt_core::{Error, Result};
use reqwest::header::{HeaderMap, HeaderValue};
use serde_json::Value;
use std::collections::HashMap;
use tracing::debug;

impl Bitget {
    /// Get the current timestamp in milliseconds.
    #[deprecated(
        since = "0.1.0",
        note = "Use `signed_request()` builder instead which handles timestamps internally"
    )]
    #[allow(dead_code)]
    fn get_timestamp() -> String {
        chrono::Utc::now().timestamp_millis().to_string()
    }

    /// Get the authentication instance if credentials are configured.
    pub fn get_auth(&self) -> Result<BitgetAuth> {
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

        Ok(BitgetAuth::new(
            api_key.expose_secret().to_string(),
            secret.expose_secret().to_string(),
            passphrase.expose_secret().to_string(),
        ))
    }

    /// Check that required credentials are configured.
    pub fn check_required_credentials(&self) -> Result<()> {
        self.base().check_required_credentials()?;
        if self.base().config.password.is_none() {
            return Err(Error::authentication("Passphrase is required for Bitget"));
        }
        Ok(())
    }

    /// Build the API path with product type prefix.
    fn build_api_path(&self, endpoint: &str) -> String {
        let product_type = self.options().effective_product_type();
        match product_type {
            "umcbl" | "usdt-futures" | "dmcbl" | "coin-futures" => {
                format!("/api/v2/mix{}", endpoint)
            }
            _ => format!("/api/v2/spot{}", endpoint),
        }
    }

    /// Make a public API request (no authentication required).
    async fn public_request(
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

        debug!("Bitget public request: {} {}", method, url);

        let response = match method.to_uppercase().as_str() {
            "GET" => self.base().http_client.get(&url, None).await?,
            "POST" => self.base().http_client.post(&url, None, None).await?,
            _ => {
                return Err(Error::invalid_request(format!(
                    "Unsupported HTTP method: {}",
                    method
                )));
            }
        };

        if error::is_error_response(&response) {
            return Err(error::parse_error(&response));
        }

        Ok(response)
    }

    /// Make a private API request (authentication required).
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

        let body_string = match body {
            Some(b) => serde_json::to_string(b).map_err(|e| {
                ccxt_core::Error::from(ccxt_core::ParseError::invalid_format(
                    "request body",
                    format!("JSON serialization failed: {}", e),
                ))
            })?,
            None => String::new(),
        };

        let sign_path = format!("{}{}", path, query_string);
        let signature = auth.sign(&timestamp, method, &sign_path, &body_string);

        let mut headers = HeaderMap::new();
        auth.add_auth_headers(&mut headers, &timestamp, &signature);
        headers.insert("Content-Type", HeaderValue::from_static("application/json"));

        let url = format!("{}{}{}", urls.rest, path, query_string);
        debug!("Bitget private request: {} {}", method, url);

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

        if error::is_error_response(&response) {
            return Err(error::parse_error(&response));
        }

        Ok(response)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::disallowed_methods)]
    use super::*;
    use ccxt_core::types::default_type::{DefaultSubType, DefaultType};

    #[test]
    fn test_build_api_path_spot() {
        let bitget = Bitget::builder().build().unwrap();
        let path = bitget.build_api_path("/public/symbols");
        assert_eq!(path, "/api/v2/spot/public/symbols");
    }

    #[test]
    fn test_build_api_path_futures_legacy() {
        let bitget = Bitget::builder()
            .default_type(DefaultType::Swap)
            .default_sub_type(DefaultSubType::Linear)
            .build()
            .unwrap();
        let path = bitget.build_api_path("/public/symbols");
        assert_eq!(path, "/api/v2/mix/public/symbols");
    }

    #[test]
    fn test_build_api_path_with_default_type_spot() {
        let bitget = Bitget::builder()
            .default_type(DefaultType::Spot)
            .build()
            .unwrap();
        let path = bitget.build_api_path("/public/symbols");
        assert_eq!(path, "/api/v2/spot/public/symbols");
    }

    #[test]
    fn test_build_api_path_with_default_type_swap_linear() {
        let bitget = Bitget::builder()
            .default_type(DefaultType::Swap)
            .default_sub_type(DefaultSubType::Linear)
            .build()
            .unwrap();
        let path = bitget.build_api_path("/public/symbols");
        assert_eq!(path, "/api/v2/mix/public/symbols");
    }

    #[test]
    fn test_build_api_path_with_default_type_swap_inverse() {
        let bitget = Bitget::builder()
            .default_type(DefaultType::Swap)
            .default_sub_type(DefaultSubType::Inverse)
            .build()
            .unwrap();
        let path = bitget.build_api_path("/public/symbols");
        assert_eq!(path, "/api/v2/mix/public/symbols");
    }

    #[test]
    fn test_build_api_path_with_default_type_futures() {
        let bitget = Bitget::builder()
            .default_type(DefaultType::Futures)
            .build()
            .unwrap();
        let path = bitget.build_api_path("/public/symbols");
        assert_eq!(path, "/api/v2/mix/public/symbols");
    }

    #[test]
    fn test_build_api_path_with_default_type_margin() {
        let bitget = Bitget::builder()
            .default_type(DefaultType::Margin)
            .build()
            .unwrap();
        let path = bitget.build_api_path("/public/symbols");
        assert_eq!(path, "/api/v2/spot/public/symbols");
    }

    #[test]
    #[allow(deprecated)]
    fn test_get_timestamp() {
        let _bitget = Bitget::builder().build().unwrap();
        let ts = Bitget::get_timestamp();

        let parsed: i64 = ts.parse().unwrap();
        assert!(parsed > 0);

        let now = chrono::Utc::now().timestamp_millis();
        assert!((now - parsed).abs() < 1000);
    }
}
