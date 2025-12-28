//! Endpoint type definitions for exchange API routing
//!
//! This module provides the `EndpointType` enum used to distinguish between
//! public and private API endpoints across all exchanges.

use serde::{Deserialize, Serialize};

/// Endpoint type enum for distinguishing between public and private API endpoints.
///
/// This enum is used by exchange-specific endpoint router traits to determine
/// which API endpoint to use for a given request.
///
/// # Examples
///
/// ```rust
/// use ccxt_core::types::EndpointType;
///
/// let public_endpoint = EndpointType::Public;
/// let private_endpoint = EndpointType::Private;
///
/// assert_ne!(public_endpoint, private_endpoint);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum EndpointType {
    /// Public API endpoint (no authentication required)
    ///
    /// Used for accessing publicly available data such as:
    /// - Market information
    /// - Ticker data
    /// - Order book data
    /// - Recent trades
    #[default]
    Public,

    /// Private API endpoint (authentication required)
    ///
    /// Used for accessing user-specific data and operations such as:
    /// - Account balances
    /// - Order management
    /// - Trade history
    /// - Withdrawals
    Private,
}

impl EndpointType {
    /// Returns `true` if this is a public endpoint.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use ccxt_core::types::EndpointType;
    ///
    /// assert!(EndpointType::Public.is_public());
    /// assert!(!EndpointType::Private.is_public());
    /// ```
    #[inline]
    pub const fn is_public(&self) -> bool {
        matches!(self, Self::Public)
    }

    /// Returns `true` if this is a private endpoint.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use ccxt_core::types::EndpointType;
    ///
    /// assert!(EndpointType::Private.is_private());
    /// assert!(!EndpointType::Public.is_private());
    /// ```
    #[inline]
    pub const fn is_private(&self) -> bool {
        matches!(self, Self::Private)
    }
}

impl std::fmt::Display for EndpointType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Public => write!(f, "public"),
            Self::Private => write!(f, "private"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_endpoint_type_equality() {
        assert_eq!(EndpointType::Public, EndpointType::Public);
        assert_eq!(EndpointType::Private, EndpointType::Private);
        assert_ne!(EndpointType::Public, EndpointType::Private);
    }

    #[test]
    fn test_endpoint_type_is_public() {
        assert!(EndpointType::Public.is_public());
        assert!(!EndpointType::Private.is_public());
    }

    #[test]
    fn test_endpoint_type_is_private() {
        assert!(EndpointType::Private.is_private());
        assert!(!EndpointType::Public.is_private());
    }

    #[test]
    fn test_endpoint_type_display() {
        assert_eq!(format!("{}", EndpointType::Public), "public");
        assert_eq!(format!("{}", EndpointType::Private), "private");
    }

    #[test]
    fn test_endpoint_type_default() {
        assert_eq!(EndpointType::default(), EndpointType::Public);
    }

    #[test]
    fn test_endpoint_type_clone() {
        let original = EndpointType::Private;
        let cloned = original.clone();
        assert_eq!(original, cloned);
    }

    #[test]
    fn test_endpoint_type_copy() {
        let original = EndpointType::Public;
        let copied = original;
        assert_eq!(original, copied);
    }

    #[test]
    fn test_endpoint_type_hash() {
        use std::collections::HashSet;

        let mut set = HashSet::new();
        set.insert(EndpointType::Public);
        set.insert(EndpointType::Private);
        set.insert(EndpointType::Public); // Duplicate

        assert_eq!(set.len(), 2);
        assert!(set.contains(&EndpointType::Public));
        assert!(set.contains(&EndpointType::Private));
    }

    #[test]
    fn test_endpoint_type_debug() {
        assert_eq!(format!("{:?}", EndpointType::Public), "Public");
        assert_eq!(format!("{:?}", EndpointType::Private), "Private");
    }
}
