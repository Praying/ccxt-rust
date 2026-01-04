//! WebSocket subscription management.

use crate::error::{Error, Result};
use dashmap::DashMap;
use serde_json::Value;
use std::collections::HashMap;

/// WebSocket subscription metadata.
#[derive(Debug, Clone)]
pub struct Subscription {
    pub(crate) channel: String,
    pub(crate) symbol: Option<String>,
    pub(crate) params: Option<HashMap<String, Value>>,
}

/// Subscription manager with capacity limits.
#[derive(Debug)]
pub struct SubscriptionManager {
    subscriptions: DashMap<String, Subscription>,
    max_subscriptions: usize,
}

impl SubscriptionManager {
    /// Creates a new subscription manager with the specified maximum capacity.
    pub fn new(max_subscriptions: usize) -> Self {
        Self {
            subscriptions: DashMap::new(),
            max_subscriptions,
        }
    }

    /// Creates a new subscription manager with the default maximum capacity (100).
    pub fn with_default_capacity() -> Self {
        Self::new(super::config::DEFAULT_MAX_SUBSCRIPTIONS)
    }

    /// Returns the maximum number of subscriptions allowed.
    #[inline]
    #[must_use]
    pub fn max_subscriptions(&self) -> usize {
        self.max_subscriptions
    }

    /// Attempts to add a subscription.
    pub fn try_add(&self, key: String, subscription: Subscription) -> Result<()> {
        if self.subscriptions.contains_key(&key) {
            self.subscriptions.insert(key, subscription);
            return Ok(());
        }

        if self.subscriptions.len() >= self.max_subscriptions {
            return Err(Error::resource_exhausted(format!(
                "Maximum subscriptions ({}) reached",
                self.max_subscriptions
            )));
        }

        self.subscriptions.insert(key, subscription);
        Ok(())
    }

    /// Removes a subscription by key.
    pub fn remove(&self, key: &str) -> Option<Subscription> {
        self.subscriptions.remove(key).map(|(_, v)| v)
    }

    /// Returns the current number of active subscriptions.
    #[inline]
    #[must_use]
    pub fn count(&self) -> usize {
        self.subscriptions.len()
    }

    /// Returns the remaining capacity for new subscriptions.
    #[inline]
    #[must_use]
    pub fn remaining_capacity(&self) -> usize {
        self.max_subscriptions
            .saturating_sub(self.subscriptions.len())
    }

    /// Checks if a subscription exists for the given key.
    #[inline]
    #[must_use]
    pub fn contains(&self, key: &str) -> bool {
        self.subscriptions.contains_key(key)
    }

    /// Returns a reference to the subscription for the given key, if it exists.
    #[must_use]
    pub fn get(&self, key: &str) -> Option<dashmap::mapref::one::Ref<'_, String, Subscription>> {
        self.subscriptions.get(key)
    }

    /// Clears all subscriptions.
    pub fn clear(&self) {
        self.subscriptions.clear();
    }

    /// Returns an iterator over all subscriptions.
    pub fn iter(
        &self,
    ) -> impl Iterator<Item = dashmap::mapref::multiple::RefMulti<'_, String, Subscription>> {
        self.subscriptions.iter()
    }

    /// Collects all subscriptions into a vector.
    #[must_use]
    pub fn collect_subscriptions(&self) -> Vec<Subscription> {
        self.subscriptions
            .iter()
            .map(|entry| entry.value().clone())
            .collect()
    }

    /// Checks if the manager is at full capacity.
    #[inline]
    #[must_use]
    pub fn is_full(&self) -> bool {
        self.subscriptions.len() >= self.max_subscriptions
    }

    /// Checks if the manager has no subscriptions.
    #[inline]
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.subscriptions.is_empty()
    }
}

impl Default for SubscriptionManager {
    fn default() -> Self {
        Self::with_default_capacity()
    }
}
