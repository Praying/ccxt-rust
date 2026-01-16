use crate::error::Result;
use serde_json::Value;
use tracing::{debug, error, warn};

use super::builder::HttpClient;

impl HttpClient {
    pub(crate) async fn execute_with_retry<F, Fut>(&self, operation: F) -> Result<Value>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<Value>>,
    {
        let mut attempt = 0;
        loop {
            match operation().await {
                Ok(response) => {
                    debug!(attempt = attempt + 1, "Operation completed successfully");
                    return Ok(response);
                }
                Err(e) => {
                    let should_retry = self.retry_strategy().should_retry(&e, attempt);

                    if should_retry {
                        let delay = self.retry_strategy().calculate_delay(attempt, &e);

                        warn!(
                            attempt = attempt + 1,
                            delay_ms = %delay.as_millis(),
                            error = %e,
                            error_debug = ?e,
                            is_retryable = e.is_retryable(),
                            "Operation failed, retrying after delay"
                        );

                        tokio::time::sleep(delay).await;
                        attempt += 1;
                    } else {
                        error!(
                            attempt = attempt + 1,
                            error = %e,
                            error_debug = ?e,
                            is_retryable = e.is_retryable(),
                            "Operation failed, not retrying"
                        );
                        return Err(e);
                    }
                }
            }
        }
    }
}
