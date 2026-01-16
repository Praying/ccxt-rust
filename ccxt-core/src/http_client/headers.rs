use reqwest::header::HeaderMap;
use serde_json::Value;

pub(crate) fn headers_to_json(headers: &HeaderMap) -> Value {
    let mut map = serde_json::Map::new();
    for (key, value) in headers {
        let key_str = key.as_str().to_string();
        let value_str = value.to_str().unwrap_or("").to_string();
        map.insert(key_str, Value::String(value_str));
    }
    Value::Object(map)
}
