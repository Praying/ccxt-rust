use crate::error::{Error, Result};
use reqwest::{Response, StatusCode};
use serde_json::Value;
use std::time::Duration;
use tracing::{debug, error, info, instrument, warn};

use super::builder::HttpClient;
use super::headers::headers_to_json;

impl HttpClient {
    #[instrument(name = "http_process_response_with_limit", skip(self, response), fields(status, url = %url
    ))]
    pub(crate) async fn process_response_with_limit(
        &self,
        response: Response,
        url: &str,
    ) -> Result<Value> {
        let status = response.status();
        let headers = response.headers().clone();
        let max_size = self.config().max_response_size;

        tracing::Span::current().record("status", status.as_u16());

        if let Some(content_length) = response.content_length()
            && content_length > max_size as u64
        {
            warn!(
                url = %url,
                content_length = content_length,
                max_size = max_size,
                "Response exceeds size limit (Content-Length check)"
            );
            return Err(Error::invalid_request(format!(
                "Response size {content_length} bytes exceeds limit {max_size} bytes"
            )));
        }

        let body_bytes = self
            .stream_response_with_limit(response, url, max_size)
            .await?;

        const BODY_PREVIEW_SIZE: usize = 200;

        let mut result: Value = if let Ok(value) = serde_json::from_slice(&body_bytes) {
            value
        } else {
            let body_text = String::from_utf8_lossy(&body_bytes).to_string();
            Value::String(body_text)
        };

        let body_preview: String = if body_bytes.len() <= BODY_PREVIEW_SIZE {
            String::from_utf8_lossy(&body_bytes).to_string()
        } else {
            String::from_utf8_lossy(&body_bytes[..BODY_PREVIEW_SIZE]).to_string()
        };

        debug!(
            status = %status,
            body_length = body_bytes.len(),
            body_preview = %body_preview,
            "HTTP response received"
        );

        if self.config().return_response_headers
            && let Value::Object(ref mut map) = result
        {
            let headers_value = headers_to_json(&headers);
            map.insert("responseHeaders".to_string(), headers_value);
        }

        if !status.is_success() {
            let body_text = String::from_utf8_lossy(&body_bytes).to_string();
            let err = Self::handle_http_error(status, &body_text, &result);
            error!(
                status = status.as_u16(),
                error = %err,
                body_preview = %body_preview,
                "HTTP error response"
            );
            return Err(err);
        }

        Ok(result)
    }

    async fn stream_response_with_limit(
        &self,
        response: Response,
        url: &str,
        max_size: usize,
    ) -> Result<Vec<u8>> {
        use futures_util::StreamExt;

        #[allow(clippy::cast_possible_truncation)]
        let initial_capacity = response
            .content_length()
            .map_or(64 * 1024, |len| std::cmp::min(len as usize, max_size));

        let mut stream = response.bytes_stream();
        let mut body = Vec::with_capacity(initial_capacity);
        let mut accumulated_size: usize = 0;

        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result.map_err(|e| {
                error!(
                    error = %e,
                    "Failed to read response chunk"
                );
                Error::network(format!("Failed to read response chunk: {e}"))
            })?;

            accumulated_size = accumulated_size.saturating_add(chunk.len());

            if accumulated_size > max_size {
                warn!(
                    url = %url,
                    accumulated_size = accumulated_size,
                    max_size = max_size,
                    "Response exceeds size limit during streaming"
                );
                return Err(Error::invalid_request(format!(
                    "Response size {accumulated_size} bytes exceeds limit {max_size} bytes (streaming)"
                )));
            }

            body.extend_from_slice(&chunk);
        }

        // Shrink buffer if we over-allocated significantly (more than 25% unused)
        // This helps with memory efficiency for responses smaller than expected
        if body.capacity() > body.len() + body.len() / 4 {
            body.shrink_to_fit();
        }

        Ok(body)
    }

    #[instrument(
        name = "http_handle_error",
        skip(body, result),
        fields(status = status.as_u16())
    )]
    fn handle_http_error(status: StatusCode, body: &str, result: &Value) -> Error {
        const BODY_PREVIEW_SIZE: usize = 200;
        let body_preview: String = body.chars().take(BODY_PREVIEW_SIZE).collect();

        match status {
            StatusCode::BAD_REQUEST => {
                info!(body_preview = %body_preview, "Bad request error");
                Error::invalid_request(body.to_string())
            }
            StatusCode::UNAUTHORIZED => {
                warn!("Authentication error: Unauthorized");
                Error::authentication("Unauthorized")
            }
            StatusCode::FORBIDDEN => {
                warn!("Authentication error: Forbidden");
                Error::authentication("Forbidden")
            }
            StatusCode::NOT_FOUND => {
                info!("Resource not found");
                Error::invalid_request("Not found")
            }
            StatusCode::TOO_MANY_REQUESTS => {
                let retry_after = if let Value::Object(map) = result {
                    if let Some(Value::Object(headers)) = map.get("responseHeaders") {
                        headers
                            .get("retry-after")
                            .and_then(|v| v.as_str())
                            .and_then(|s| s.parse::<u64>().ok())
                    } else {
                        None
                    }
                } else {
                    None
                };

                if let Some(seconds) = retry_after {
                    warn!(
                        retry_after_seconds = seconds,
                        "Rate limit exceeded with retry-after header"
                    );
                    Error::rate_limit(
                        format!("Rate limit exceeded, retry after {seconds} seconds"),
                        Some(Duration::from_secs(seconds)),
                    )
                } else {
                    warn!("Rate limit exceeded without retry-after header");
                    Error::rate_limit("Rate limit exceeded, please retry later", None)
                }
            }
            StatusCode::INTERNAL_SERVER_ERROR => {
                error!(body_preview = %body_preview, "Internal server error");
                Error::exchange("500", "Internal server error")
            }
            StatusCode::SERVICE_UNAVAILABLE => {
                error!(body_preview = %body_preview, "Service unavailable");
                Error::exchange("503", "Service unavailable")
            }
            StatusCode::GATEWAY_TIMEOUT => {
                error!("Gateway timeout");
                Error::from(crate::error::NetworkError::Timeout)
            }
            _ => {
                error!(
                    status = status.as_u16(),
                    body_preview = %body_preview,
                    "Unhandled HTTP error"
                );
                Error::network(format!("HTTP {status} error: {body}"))
            }
        }
    }
}
