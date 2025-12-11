//! Strongly typed parameter builder for ticker requests.
//!
//! This module provides a typed interface for constructing ticker request
//! parameters while preserving the ability to fall back to dynamic JSON values
//! when required.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// Strongly typed parameters accepted by ticker endpoints.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct TickerParams {
    /// When `true`, instructs the exchange to use the rolling window ticker
    /// endpoint instead of the default 24h ticker.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rolling: Option<bool>,

    /// Specifies the size of the rolling window in minutes (Binance specific).
    #[serde(rename = "windowSize", skip_serializing_if = "Option::is_none")]
    pub window_size: Option<u32>,

    /// Additional exchange-specific parameters.
    #[serde(flatten)]
    pub extras: Map<String, Value>,
}

impl TickerParams {
    /// Creates a new [`TickerParamsBuilder`].
    #[must_use]
    pub fn builder() -> TickerParamsBuilder {
        TickerParamsBuilder::default()
    }

    /// Inserts or overwrites an arbitrary extra parameter by key.
    pub fn insert_extra<K, V>(&mut self, key: K, value: V)
    where
        K: Into<String>,
        V: Into<Value>,
    {
        self.extras.insert(key.into(), value.into());
    }
}

/// Convenience builder for [`TickerParams`].
#[derive(Debug, Default)]
pub struct TickerParamsBuilder {
    inner: TickerParams,
}

impl TickerParamsBuilder {
    /// Creates a new builder instance.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the `rolling` flag.
    #[must_use]
    pub fn rolling(mut self, rolling: bool) -> Self {
        self.inner.rolling = Some(rolling);
        self
    }

    /// Sets the `window_size` parameter in minutes.
    #[must_use]
    pub fn window_size(mut self, window_size: u32) -> Self {
        self.inner.window_size = Some(window_size);
        self
    }

    /// Adds an arbitrary extra parameter by key.
    #[must_use]
    pub fn extra<K, V>(mut self, key: K, value: V) -> Self
    where
        K: Into<String>,
        V: Into<Value>,
    {
        self.inner.insert_extra(key, value);
        self
    }

    /// Finalises the builder and returns [`TickerParams`].
    #[must_use]
    pub fn build(self) -> TickerParams {
        self.inner
    }
}

impl From<TickerParamsBuilder> for TickerParams {
    fn from(builder: TickerParamsBuilder) -> Self {
        builder.build()
    }
}

/// Trait for values convertible into [`TickerParams`].
pub trait IntoTickerParams {
    /// Consumes `self` and produces [`TickerParams`].
    fn into_ticker_params(self) -> TickerParams;
}

impl IntoTickerParams for TickerParams {
    fn into_ticker_params(self) -> TickerParams {
        self
    }
}

impl IntoTickerParams for TickerParamsBuilder {
    fn into_ticker_params(self) -> TickerParams {
        self.build()
    }
}

impl IntoTickerParams for Value {
    fn into_ticker_params(self) -> TickerParams {
        match self {
            Value::Object(mut map) => {
                let mut params = TickerParams::default();

                if let Some(value) = map.remove("rolling") {
                    if let Some(flag) = value.as_bool() {
                        params.rolling = Some(flag);
                    } else {
                        params.extras.insert("rolling".to_string(), value);
                    }
                }

                if let Some(value) = map.remove("windowSize") {
                    match value {
                        Value::Number(num) if num.is_u64() => {
                            params.window_size = num.as_u64().map(|v| v as u32);
                        }
                        Value::String(text) => {
                            if let Ok(parsed) = text.parse::<u32>() {
                                params.window_size = Some(parsed);
                            } else {
                                params
                                    .extras
                                    .insert("windowSize".to_string(), Value::String(text));
                            }
                        }
                        other => {
                            params.extras.insert("windowSize".to_string(), other);
                        }
                    }
                }

                params.extras.extend(map);
                params
            }
            other => {
                let mut params = TickerParams::default();
                params.extras.insert("value".to_string(), other);
                params
            }
        }
    }
}

impl IntoTickerParams for Map<String, Value> {
    fn into_ticker_params(self) -> TickerParams {
        Value::Object(self).into_ticker_params()
    }
}

impl<S> IntoTickerParams for std::collections::HashMap<String, Value, S>
where
    S: std::hash::BuildHasher,
{
    fn into_ticker_params(self) -> TickerParams {
        let map: Map<String, Value> = self.into_iter().collect();
        Value::Object(map).into_ticker_params()
    }
}

impl IntoTickerParams for () {
    fn into_ticker_params(self) -> TickerParams {
        TickerParams::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn builder_sets_fields() {
        let params = TickerParams::builder()
            .rolling(true)
            .window_size(30)
            .extra("foo", 42)
            .build();

        assert_eq!(params.rolling, Some(true));
        assert_eq!(params.window_size, Some(30));
        assert_eq!(params.extras.get("foo"), Some(&json!(42)));
    }

    #[test]
    fn value_conversion_respects_known_fields() {
        let params = json!({
            "rolling": true,
            "windowSize": 15,
            "bar": "baz"
        })
        .into_ticker_params();

        assert_eq!(params.rolling, Some(true));
        assert_eq!(params.window_size, Some(15));
        assert_eq!(params.extras.get("bar"), Some(&json!("baz")));
    }

    #[test]
    fn unit_type_provides_default() {
        let params = ().into_ticker_params();
        assert!(params.rolling.is_none());
        assert!(params.extras.is_empty());
    }
}
