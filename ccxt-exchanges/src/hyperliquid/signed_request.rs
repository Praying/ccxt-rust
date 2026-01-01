//! Signed request builder for Hyperliquid API.
//!
//! This module provides a builder pattern for creating authenticated Hyperliquid exchange actions,
//! encapsulating the common EIP-712 signing workflow used across all authenticated endpoints.
//!
//! # Overview
//!
//! Unlike HMAC-based exchanges (Binance, OKX, Bitget, Bybit), Hyperliquid uses EIP-712 typed data
//! signing with Ethereum private keys. The `HyperliquidSignedRequestBuilder` centralizes:
//! - Private key validation
//! - Nonce generation (millisecond timestamp)
//! - EIP-712 signature generation (r, s, v components)
//! - Request body construction with signature
//! - HTTP request execution
//!
//! # Example
//!
//! ```no_run
//! # use ccxt_exchanges::hyperliquid::HyperLiquid;
//! # use serde_json::json;
//! # async fn example() -> ccxt_core::Result<()> {
//! let hyperliquid = HyperLiquid::builder()
//!     .private_key("0x...")
//!     .testnet(true)
//!     .build()?;
//!
//! // Create an order action
//! let action = json!({
//!     "type": "order",
//!     "orders": [{"a": 0, "b": true, "p": "50000", "s": "0.001", "r": false, "t": {"limit": {"tif": "Gtc"}}}],
//!     "grouping": "na"
//! });
//!
//! let response = hyperliquid.signed_action(action)
//!     .execute()
//!     .await?;
//! # Ok(())
//! # }
//! ```

use super::{HyperLiquid, error};
use ccxt_core::{Error, Result};
use serde_json::{Map, Value};

/// Builder for creating authenticated Hyperliquid exchange actions.
///
/// This builder encapsulates the EIP-712 signing workflow:
/// 1. Private key validation
/// 2. Nonce generation (millisecond timestamp)
/// 3. EIP-712 signature generation via `HyperLiquidAuth.sign_l1_action()`
/// 4. Request body construction with action, nonce, signature, and optional vault address
/// 5. HTTP POST request execution to `/exchange` endpoint
///
/// # Hyperliquid Signature Format
///
/// Hyperliquid uses EIP-712 typed data signing:
/// - Domain: HyperliquidSignTransaction
/// - Chain ID: 42161 (mainnet) or 421614 (testnet)
/// - Signature components: r (32 bytes hex), s (32 bytes hex), v (recovery id)
///
/// # Example
///
/// ```no_run
/// # use ccxt_exchanges::hyperliquid::HyperLiquid;
/// # use serde_json::json;
/// # async fn example() -> ccxt_core::Result<()> {
/// let hyperliquid = HyperLiquid::builder()
///     .private_key("0x...")
///     .testnet(true)
///     .build()?;
///
/// let action = json!({"type": "cancel", "cancels": [{"a": 0, "o": 12345}]});
///
/// let response = hyperliquid.signed_action(action)
///     .nonce(1234567890000)  // Optional: override auto-generated nonce
///     .execute()
///     .await?;
/// # Ok(())
/// # }
/// ```
pub struct HyperliquidSignedRequestBuilder<'a> {
    /// Reference to the HyperLiquid exchange instance
    hyperliquid: &'a HyperLiquid,
    /// The action to be signed and executed
    action: Value,
    /// Optional nonce override (defaults to current timestamp in milliseconds)
    nonce: Option<u64>,
}

impl<'a> HyperliquidSignedRequestBuilder<'a> {
    /// Creates a new signed action builder.
    ///
    /// # Arguments
    ///
    /// * `hyperliquid` - Reference to the HyperLiquid exchange instance
    /// * `action` - The action JSON to be signed and executed
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use ccxt_exchanges::hyperliquid::HyperLiquid;
    /// # use ccxt_exchanges::hyperliquid::signed_request::HyperliquidSignedRequestBuilder;
    /// # use serde_json::json;
    /// let hyperliquid = HyperLiquid::builder()
    ///     .private_key("0x...")
    ///     .testnet(true)
    ///     .build()
    ///     .unwrap();
    ///
    /// let action = json!({"type": "order", "orders": [], "grouping": "na"});
    /// let builder = HyperliquidSignedRequestBuilder::new(&hyperliquid, action);
    /// ```
    pub fn new(hyperliquid: &'a HyperLiquid, action: Value) -> Self {
        Self {
            hyperliquid,
            action,
            nonce: None,
        }
    }

    /// Sets a custom nonce for the request.
    ///
    /// By default, the nonce is automatically generated from the current timestamp
    /// in milliseconds. Use this method to override the auto-generated nonce.
    ///
    /// # Arguments
    ///
    /// * `nonce` - The nonce value (typically timestamp in milliseconds)
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use ccxt_exchanges::hyperliquid::HyperLiquid;
    /// # use serde_json::json;
    /// # async fn example() -> ccxt_core::Result<()> {
    /// let hyperliquid = HyperLiquid::builder()
    ///     .private_key("0x...")
    ///     .testnet(true)
    ///     .build()?;
    ///
    /// let action = json!({"type": "order", "orders": [], "grouping": "na"});
    ///
    /// let response = hyperliquid.signed_action(action)
    ///     .nonce(1234567890000)
    ///     .execute()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn nonce(mut self, nonce: u64) -> Self {
        self.nonce = Some(nonce);
        self
    }

    /// Executes the signed action and returns the response.
    ///
    /// This method:
    /// 1. Validates that a private key is configured
    /// 2. Gets or generates the nonce (millisecond timestamp)
    /// 3. Signs the action using EIP-712 typed data signing
    /// 4. Constructs the request body with action, nonce, signature, and optional vault address
    /// 5. Executes the HTTP POST request to `/exchange` endpoint
    ///
    /// # Hyperliquid Signature Details
    ///
    /// - Uses EIP-712 typed data signing
    /// - Domain: HyperliquidSignTransaction, version 1
    /// - Chain ID: 42161 (mainnet) or 421614 (testnet)
    /// - Signature format: { r: "0x...", s: "0x...", v: 27|28 }
    ///
    /// # Returns
    ///
    /// Returns the raw `serde_json::Value` response for further parsing.
    ///
    /// # Errors
    ///
    /// - Returns authentication error if private key is missing
    /// - Returns network error if the request fails
    /// - Returns exchange error if the API returns an error response
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use ccxt_exchanges::hyperliquid::HyperLiquid;
    /// # use serde_json::json;
    /// # async fn example() -> ccxt_core::Result<()> {
    /// let hyperliquid = HyperLiquid::builder()
    ///     .private_key("0x...")
    ///     .testnet(true)
    ///     .build()?;
    ///
    /// let action = json!({
    ///     "type": "order",
    ///     "orders": [{"a": 0, "b": true, "p": "50000", "s": "0.001", "r": false, "t": {"limit": {"tif": "Gtc"}}}],
    ///     "grouping": "na"
    /// });
    ///
    /// let response = hyperliquid.signed_action(action)
    ///     .execute()
    ///     .await?;
    /// println!("Response: {:?}", response);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn execute(self) -> Result<Value> {
        // Step 1: Validate that private key is configured
        let auth = self
            .hyperliquid
            .auth()
            .ok_or_else(|| Error::authentication("Private key required for exchange actions"))?;

        // Step 2: Get or generate nonce
        let nonce = self.nonce.unwrap_or_else(|| get_current_nonce());

        // Step 3: Determine if mainnet or testnet
        let is_mainnet = !self.hyperliquid.options().testnet;

        // Step 4: Sign the action using EIP-712
        let signature = auth.sign_l1_action(&self.action, nonce, is_mainnet)?;

        // Step 5: Build signature object with r, s, v components
        let mut signature_map = Map::new();
        signature_map.insert("r".to_string(), Value::String(format!("0x{}", signature.r)));
        signature_map.insert("s".to_string(), Value::String(format!("0x{}", signature.s)));
        signature_map.insert("v".to_string(), Value::Number(signature.v.into()));

        // Step 6: Build request body
        let mut body_map = Map::new();
        body_map.insert("action".to_string(), self.action);
        body_map.insert("nonce".to_string(), Value::Number(nonce.into()));
        body_map.insert("signature".to_string(), Value::Object(signature_map));

        // Add vault address if configured
        if let Some(vault_address) = &self.hyperliquid.options().vault_address {
            body_map.insert(
                "vaultAddress".to_string(),
                Value::String(format!("0x{}", hex::encode(vault_address))),
            );
        }

        let body = Value::Object(body_map);

        // Step 7: Execute HTTP POST request
        let urls = self.hyperliquid.urls();
        let url = format!("{}/exchange", urls.rest);

        tracing::debug!("HyperLiquid signed action request: {:?}", body);

        let response = self
            .hyperliquid
            .base()
            .http_client
            .post(&url, None, Some(body))
            .await?;

        // Step 8: Check for error response
        if error::is_error_response(&response) {
            return Err(error::parse_error(&response));
        }

        Ok(response)
    }
}

/// Gets the current timestamp in milliseconds as a nonce.
fn get_current_nonce() -> u64 {
    chrono::Utc::now().timestamp_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // Test private key (DO NOT USE IN PRODUCTION)
    const TEST_PRIVATE_KEY: &str =
        "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80";

    #[test]
    fn test_builder_construction() {
        let hyperliquid = HyperLiquid::builder().testnet(true).build().unwrap();

        let action = json!({"type": "order", "orders": [], "grouping": "na"});
        let builder = HyperliquidSignedRequestBuilder::new(&hyperliquid, action.clone());

        assert_eq!(builder.action, action);
        assert!(builder.nonce.is_none());
    }

    #[test]
    fn test_builder_with_nonce() {
        let hyperliquid = HyperLiquid::builder().testnet(true).build().unwrap();

        let action = json!({"type": "order", "orders": [], "grouping": "na"});
        let builder =
            HyperliquidSignedRequestBuilder::new(&hyperliquid, action).nonce(1234567890000);

        assert_eq!(builder.nonce, Some(1234567890000));
    }

    #[test]
    fn test_builder_method_chaining() {
        let hyperliquid = HyperLiquid::builder().testnet(true).build().unwrap();

        let action = json!({"type": "cancel", "cancels": []});
        let builder =
            HyperliquidSignedRequestBuilder::new(&hyperliquid, action.clone()).nonce(9999999999999);

        assert_eq!(builder.action, action);
        assert_eq!(builder.nonce, Some(9999999999999));
    }

    #[test]
    fn test_get_current_nonce() {
        let nonce1 = get_current_nonce();
        std::thread::sleep(std::time::Duration::from_millis(10));
        let nonce2 = get_current_nonce();

        // Nonce should be increasing
        assert!(nonce2 > nonce1);

        // Nonce should be a reasonable timestamp (after year 2020)
        assert!(nonce1 > 1577836800000); // 2020-01-01 00:00:00 UTC
    }

    #[test]
    fn test_builder_with_authenticated_exchange() {
        let hyperliquid = HyperLiquid::builder()
            .private_key(TEST_PRIVATE_KEY)
            .testnet(true)
            .build()
            .unwrap();

        let action = json!({"type": "order", "orders": [], "grouping": "na"});
        let builder = HyperliquidSignedRequestBuilder::new(&hyperliquid, action);

        // Should have auth available
        assert!(builder.hyperliquid.auth().is_some());
    }

    #[test]
    fn test_builder_without_authentication() {
        let hyperliquid = HyperLiquid::builder().testnet(true).build().unwrap();

        let action = json!({"type": "order", "orders": [], "grouping": "na"});
        let builder = HyperliquidSignedRequestBuilder::new(&hyperliquid, action);

        // Should not have auth available
        assert!(builder.hyperliquid.auth().is_none());
    }

    #[tokio::test]
    async fn test_execute_without_credentials_returns_error() {
        let hyperliquid = HyperLiquid::builder().testnet(true).build().unwrap();

        let action = json!({"type": "order", "orders": [], "grouping": "na"});
        let result = hyperliquid.signed_action(action).execute().await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Private key required"));
    }

    #[test]
    fn test_different_action_types() {
        let hyperliquid = HyperLiquid::builder().testnet(true).build().unwrap();

        // Order action
        let order_action = json!({
            "type": "order",
            "orders": [{"a": 0, "b": true, "p": "50000", "s": "0.001", "r": false, "t": {"limit": {"tif": "Gtc"}}}],
            "grouping": "na"
        });
        let builder = HyperliquidSignedRequestBuilder::new(&hyperliquid, order_action.clone());
        assert_eq!(builder.action["type"], "order");

        // Cancel action
        let cancel_action = json!({
            "type": "cancel",
            "cancels": [{"a": 0, "o": 12345}]
        });
        let builder = HyperliquidSignedRequestBuilder::new(&hyperliquid, cancel_action.clone());
        assert_eq!(builder.action["type"], "cancel");

        // Update leverage action
        let leverage_action = json!({
            "type": "updateLeverage",
            "asset": 0,
            "isCross": true,
            "leverage": 10
        });
        let builder = HyperliquidSignedRequestBuilder::new(&hyperliquid, leverage_action.clone());
        assert_eq!(builder.action["type"], "updateLeverage");
    }
}
