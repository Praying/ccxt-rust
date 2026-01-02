//! Secure credential types with automatic memory zeroization.
//!
//! All sensitive data is automatically cleared from memory when dropped,
//! preventing credential leakage through memory dumps or core files.
//!
//! # Security
//!
//! This module provides types that implement the `Zeroize` and `ZeroizeOnDrop`
//! traits from the `zeroize` crate. When these types go out of scope, their
//! memory is securely overwritten with zeros before being deallocated.
//!
//! # Example
//!
//! ```rust
//! use ccxt_core::credentials::SecretString;
//!
//! let api_key = SecretString::new("my-api-key");
//!
//! // Access the secret when needed
//! let key_value = api_key.expose_secret();
//!
//! // Debug output is redacted
//! println!("{:?}", api_key); // Prints: [REDACTED]
//!
//! // When api_key goes out of scope, memory is automatically zeroed
//! ```

use std::fmt;
use zeroize::{Zeroize, ZeroizeOnDrop};

/// A secure string that is automatically zeroed when dropped.
///
/// Use this for API keys, secrets, passwords, and other sensitive data.
/// The underlying memory is securely cleared when the value is dropped,
/// preventing credential leakage through memory inspection.
///
/// # Security Features
///
/// - Memory is zeroed on drop using the `zeroize` crate
/// - Debug and Display implementations are redacted to prevent accidental logging
/// - Clone creates a new zeroed copy (original remains secure)
///
/// # Example
///
/// ```rust
/// use ccxt_core::credentials::SecretString;
///
/// let secret = SecretString::new("my-secret-key");
///
/// // Use expose_secret() to access the value
/// assert_eq!(secret.expose_secret(), "my-secret-key");
///
/// // Debug output is safe
/// println!("{:?}", secret); // Prints: [REDACTED]
/// ```
#[derive(Clone, Zeroize, ZeroizeOnDrop, PartialEq, Eq)]
pub struct SecretString(String);

impl SecretString {
    /// Creates a new secret string.
    ///
    /// # Arguments
    ///
    /// * `value` - The secret value to store
    ///
    /// # Example
    ///
    /// ```rust
    /// use ccxt_core::credentials::SecretString;
    ///
    /// let secret = SecretString::new("api-key-12345");
    /// ```
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Returns the secret value.
    ///
    /// # Security
    ///
    /// Avoid storing the returned reference longer than necessary.
    /// The reference should be used immediately and not persisted.
    ///
    /// # Example
    ///
    /// ```rust
    /// use ccxt_core::credentials::SecretString;
    ///
    /// let secret = SecretString::new("my-key");
    /// let value = secret.expose_secret();
    /// // Use value immediately, don't store it
    /// ```
    #[inline]
    pub fn expose_secret(&self) -> &str {
        &self.0
    }

    /// Returns the secret as bytes.
    ///
    /// # Security
    ///
    /// Same security considerations as `expose_secret()` apply.
    #[inline]
    pub fn expose_secret_bytes(&self) -> &[u8] {
        self.0.as_bytes()
    }

    /// Returns the length of the secret string.
    #[inline]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns true if the secret string is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

// Prevent accidental logging of sensitive data
impl fmt::Debug for SecretString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("[REDACTED]")
    }
}

impl fmt::Display for SecretString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("[REDACTED]")
    }
}

// Convenient conversions
impl From<String> for SecretString {
    fn from(s: String) -> Self {
        Self::new(s)
    }
}

impl From<&str> for SecretString {
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}

/// Secure bytes that are automatically zeroed when dropped.
///
/// Use this for private keys, binary secrets, and other sensitive byte data.
///
/// # Example
///
/// ```rust
/// use ccxt_core::credentials::SecretBytes;
///
/// let private_key = SecretBytes::new(vec![0x01, 0x02, 0x03, 0x04]);
/// let bytes = private_key.expose_secret();
/// ```
#[derive(Clone, Zeroize, ZeroizeOnDrop, PartialEq, Eq)]
pub struct SecretBytes(Vec<u8>);

impl SecretBytes {
    /// Creates new secret bytes.
    ///
    /// # Arguments
    ///
    /// * `value` - The secret bytes to store
    pub fn new(value: impl Into<Vec<u8>>) -> Self {
        Self(value.into())
    }

    /// Creates secret bytes from a fixed-size array.
    ///
    /// # Arguments
    ///
    /// * `value` - The secret bytes array to store
    pub fn from_array<const N: usize>(value: [u8; N]) -> Self {
        Self(value.to_vec())
    }

    /// Returns the secret bytes.
    ///
    /// # Security
    ///
    /// Avoid storing the returned reference longer than necessary.
    #[inline]
    pub fn expose_secret(&self) -> &[u8] {
        &self.0
    }

    /// Returns the length of the secret bytes.
    #[inline]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns true if the secret bytes are empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl fmt::Debug for SecretBytes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[REDACTED {} bytes]", self.0.len())
    }
}

impl<const N: usize> From<[u8; N]> for SecretBytes {
    fn from(arr: [u8; N]) -> Self {
        Self::from_array(arr)
    }
}

impl From<Vec<u8>> for SecretBytes {
    fn from(v: Vec<u8>) -> Self {
        Self::new(v)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_secret_string_debug_redacted() {
        let secret = SecretString::new("my-api-key");
        assert_eq!(format!("{:?}", secret), "[REDACTED]");
    }

    #[test]
    fn test_secret_string_display_redacted() {
        let secret = SecretString::new("my-api-key");
        assert_eq!(format!("{}", secret), "[REDACTED]");
    }

    #[test]
    fn test_secret_string_expose() {
        let secret = SecretString::new("my-api-key");
        assert_eq!(secret.expose_secret(), "my-api-key");
    }

    #[test]
    fn test_secret_string_expose_bytes() {
        let secret = SecretString::new("test");
        assert_eq!(secret.expose_secret_bytes(), b"test");
    }

    #[test]
    fn test_secret_string_len() {
        let secret = SecretString::new("12345");
        assert_eq!(secret.len(), 5);
        assert!(!secret.is_empty());
    }

    #[test]
    fn test_secret_string_empty() {
        let secret = SecretString::new("");
        assert!(secret.is_empty());
    }

    #[test]
    fn test_secret_from_string() {
        let secret: SecretString = String::from("test").into();
        assert_eq!(secret.expose_secret(), "test");
    }

    #[test]
    fn test_secret_from_str() {
        let secret: SecretString = "test".into();
        assert_eq!(secret.expose_secret(), "test");
    }

    #[test]
    fn test_secret_bytes_debug_redacted() {
        let secret = SecretBytes::new(vec![1, 2, 3, 4, 5]);
        assert_eq!(format!("{:?}", secret), "[REDACTED 5 bytes]");
    }

    #[test]
    fn test_secret_bytes_expose() {
        let secret = SecretBytes::new(vec![1, 2, 3]);
        assert_eq!(secret.expose_secret(), &[1, 2, 3]);
    }

    #[test]
    fn test_secret_bytes_from_array() {
        let secret = SecretBytes::from_array([1u8, 2, 3, 4]);
        assert_eq!(secret.expose_secret(), &[1, 2, 3, 4]);
    }

    #[test]
    fn test_secret_bytes_len() {
        let secret = SecretBytes::new(vec![1, 2, 3]);
        assert_eq!(secret.len(), 3);
        assert!(!secret.is_empty());
    }

    #[test]
    fn test_secret_clone() {
        let secret1 = SecretString::new("test");
        let secret2 = secret1.clone();
        assert_eq!(secret1.expose_secret(), secret2.expose_secret());
    }
}
