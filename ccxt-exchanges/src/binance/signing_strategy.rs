//! Binance signing strategy for the generic SignedRequestBuilder.
//!
//! Implements the SigningStrategy trait for Binance-specific authentication.

use crate::binance::Binance;
use async_trait::async_trait;
use ccxt_core::Result;
use ccxt_core::signed_request::{SigningContext, SigningStrategy};
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use std::sync::Arc;

/// Binance signing strategy implementing the generic SigningStrategy trait.
///
/// Handles Binance-specific signing requirements:
/// - Millisecond timestamp with server time offset
/// - HMAC-SHA256 signature (hex encoded)
/// - X-MBX-APIKEY header for authentication
pub struct BinanceSigningStrategy {
    /// Reference to the Binance exchange for accessing auth and time sync
    binance: Arc<Binance>,
}

impl BinanceSigningStrategy {
    /// Create a new Binance signing strategy.
    pub fn new(binance: Arc<Binance>) -> Self {
        Self { binance }
    }

    /// Create from a reference (clones the Arc internally).
    pub fn from_ref(binance: &Arc<Binance>) -> Self {
        Self {
            binance: binance.clone(),
        }
    }
}

#[async_trait]
impl SigningStrategy for BinanceSigningStrategy {
    async fn prepare_request(&self, ctx: &mut SigningContext) -> Result<()> {
        // Step 1: Validate credentials
        self.binance.check_required_credentials()?;

        // Step 2: Get signing timestamp (with server time offset)
        let timestamp = self.binance.get_signing_timestamp().await?;
        ctx.timestamp = timestamp.to_string();

        // Step 3: Add timestamp and recvWindow to params
        ctx.params
            .insert("timestamp".to_string(), ctx.timestamp.clone());
        ctx.params.insert(
            "recvWindow".to_string(),
            self.binance.options().recv_window.to_string(),
        );

        // Step 4: Get auth and compute signature
        let auth = self.binance.get_auth()?;
        let query_string = build_signing_string(&ctx.params);
        let signature = auth.sign(&query_string)?;

        // Step 5: Add signature to params
        ctx.signature = Some(signature.clone());
        ctx.params.insert("signature".to_string(), signature);

        Ok(())
    }

    fn add_auth_headers(&self, headers: &mut HeaderMap, _ctx: &SigningContext) {
        // Add X-MBX-APIKEY header
        if let Ok(auth) = self.binance.get_auth() {
            if let Ok(header_name) = HeaderName::from_bytes(b"X-MBX-APIKEY") {
                if let Ok(header_value) = HeaderValue::from_str(auth.api_key()) {
                    headers.insert(header_name, header_value);
                }
            }
        }
    }
}

/// Build query string for signing (without URL encoding).
fn build_signing_string(params: &std::collections::BTreeMap<String, String>) -> String {
    params
        .iter()
        .map(|(k, v)| format!("{}={}", k, v))
        .collect::<Vec<_>>()
        .join("&")
}

#[cfg(test)]
mod tests {
    use super::*;
    use ccxt_core::ExchangeConfig;

    #[test]
    fn test_signing_strategy_creation() {
        let config = ExchangeConfig::default();
        let binance = Arc::new(Binance::new(config).unwrap());
        let _strategy = BinanceSigningStrategy::new(binance);
        // Strategy created successfully
        assert!(true);
    }

    #[test]
    fn test_build_signing_string() {
        let mut params = std::collections::BTreeMap::new();
        params.insert("symbol".to_string(), "BTCUSDT".to_string());
        params.insert("side".to_string(), "BUY".to_string());
        params.insert("timestamp".to_string(), "1234567890".to_string());

        let query = build_signing_string(&params);
        // BTreeMap maintains alphabetical order
        assert_eq!(query, "side=BUY&symbol=BTCUSDT&timestamp=1234567890");
    }
}
