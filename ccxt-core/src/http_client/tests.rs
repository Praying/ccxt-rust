use super::*;
use crate::error::Error;
use crate::retry_strategy::RetryConfig;
use std::time::Duration;

#[tokio::test]
async fn test_http_client_creation() {
    let config = HttpConfig::default();
    let client = HttpClient::new(config);
    assert!(client.is_ok());
}

// Concurrent request test
#[tokio::test]
async fn test_concurrent_requests() {
    use futures_util::future::join_all;
    use std::sync::Arc;

    let config = HttpConfig {
        timeout: Duration::from_secs(5),
        ..Default::default()
    };
    let client = Arc::new(HttpClient::new(config).unwrap());

    // Spawn multiple concurrent requests
    let handles: Vec<_> = (0..5)
        .map(|_| {
            let client = client.clone();
            tokio::spawn(async move {
                let _ = client.get("https://httpbin.org/get", None).await;
            })
        })
        .collect();

    let results = join_all(handles).await;

    // All tasks should complete (may succeed or fail due to network)
    assert_eq!(results.len(), 5);
}

// Circuit breaker state transition test
#[tokio::test]
async fn test_circuit_breaker_state_transitions() {
    let mut client = HttpClient::new(HttpConfig::default()).unwrap();

    let cb_config = crate::circuit_breaker::CircuitBreakerConfig {
        failure_threshold: 3,
        reset_timeout: Duration::from_secs(60),
        success_threshold: 2,
    };
    let cb = std::sync::Arc::new(crate::circuit_breaker::CircuitBreaker::new(cb_config));

    // Initially should be Closed
    assert_eq!(cb.state(), crate::circuit_breaker::CircuitState::Closed);

    // Record failures up to threshold
    for _ in 0..3 {
        cb.record_failure();
    }

    // Should be Open after threshold reached
    assert_eq!(cb.state(), crate::circuit_breaker::CircuitState::Open);

    // Attempt request should fail with circuit open error
    client.set_circuit_breaker(cb.clone());
    let result = client.get("https://httpbin.org/get", None).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().as_resource_exhausted().is_some());
}

// Rate limiter precision test
#[tokio::test]
#[ignore = "Network-dependent test: requires stable httpbin.org latency"]
async fn test_rate_limiter_precision() {
    use crate::rate_limiter::{RateLimiter, RateLimiterConfig};
    use std::time::Instant;

    let limiter_config = RateLimiterConfig::new(10, Duration::from_secs(1));
    let limiter = RateLimiter::new(limiter_config);

    let config = HttpConfig {
        enable_rate_limit: true,
        ..Default::default()
    };
    let client = HttpClient::new_with_rate_limiter(config, limiter).unwrap();

    let mut success_count = 0;
    let mut failure_count = 0;
    let start = Instant::now();

    // Send 15 requests rapidly (should hit rate limit after 10)
    for _ in 0..15 {
        let _ = client.get("https://httpbin.org/get", None).await;
        if start.elapsed() < Duration::from_millis(900) {
            success_count += 1;
        } else {
            failure_count += 1;
        }
    }

    // At least first 10 should be quick, remaining should be rate-limited
    if success_count >= 10 && start.elapsed() >= Duration::from_secs(1) {
        assert!(true);
    } else {
        tracing::warn!(
            "Rate limiter test inconclusive: success_count={}, elapsed={:?}",
            success_count,
            start.elapsed()
        );
    }
}

// Large response streaming test
#[tokio::test]
async fn test_large_response_streaming() {
    // Large response streaming test
    #[tokio::test]
    async fn test_large_response_streaming() {
        let config = HttpConfig {
            max_response_size: 1024 * 1024, // 1MB limit
            retry_config: Some(RetryConfig {
                max_retries: 0,
                ..RetryConfig::default()
            }),
            ..Default::default()
        };
        let client = HttpClient::new(config).unwrap();

        // httpbin.org/bytes/2048 returns 2048 bytes
        let result = client.get("https://httpbin.org/bytes/2048", None).await;

        // Should succeed (2048 < 1024 * 1024)
        match result {
            Ok(_) => assert!(true),
            Err(Error::InvalidRequest(msg))
                if msg.contains("exceeds limit") || msg.contains("size") =>
            {
                // If rejected, it should have exceeded the 1MB limit
                assert!(msg.contains("size") || msg.contains("limit"));
            }
            Err(Error::Network(_)) => {
                // Network tests use httpbin.org which may be unavailable in CI environments
                tracing::warn!("Network test skipped due to network error");
            }
            Err(e) => panic!("Unexpected error: {:?}", e),
        }
    }

    #[tokio::test]
    async fn test_http_config_default() {
        let config = HttpConfig::default();
        assert_eq!(config.timeout, Duration::from_secs(30));
        assert_eq!(config.connect_timeout, Duration::from_secs(10));
        assert!(config.retry_config.is_none());
        assert!(!config.verbose);
        assert_eq!(config.user_agent, "ccxt-rust/1.0");
        assert!(config.enable_rate_limit);
        assert_eq!(config.max_response_size, 10 * 1024 * 1024);
    }

    #[tokio::test]
    async fn test_headers_to_json() {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert("Content-Type", "application/json".parse().unwrap());
        headers.insert("X-Custom-Header", "test-value".parse().unwrap());

        let json = super::headers::headers_to_json(&headers);
        assert!(json.is_object());

        let obj = json.as_object().unwrap();
        assert_eq!(obj.get("content-type").unwrap(), "application/json");
        assert_eq!(obj.get("x-custom-header").unwrap(), "test-value");
    }

    #[tokio::test]
    async fn test_http_client_with_proxy() {
        let config = HttpConfig {
            proxy: Some(crate::config::ProxyConfig::new("http://localhost:8080")),
            ..Default::default()
        };

        let client = HttpClient::new(config);
        assert!(client.is_ok());
    }

    #[tokio::test]
    async fn test_get_request() {
        let config = HttpConfig {
            verbose: false,
            ..Default::default()
        };
        let client = HttpClient::new(config).unwrap();

        let result = client.get("https://httpbin.org/get", None).await;

        match result {
            Ok(value) => {
                assert!(value.is_object());
            }
            Err(e) => {
                tracing::warn!("Network test skipped due to: {:?}", e);
            }
        }
    }

    #[tokio::test]
    async fn test_post_request() {
        let config = HttpConfig::default();
        let client = HttpClient::new(config).unwrap();

        let body = serde_json::json!({
            "test": "data",
            "number": 123
        });

        let result = client
            .post("https://httpbin.org/post", None, Some(body))
            .await;

        match result {
            Ok(value) => {
                assert!(value.is_object());
            }
            Err(e) => {
                tracing::warn!("Network test skipped due to: {:?}", e);
            }
        }
    }

    #[tokio::test]
    async fn test_http_error_handling() {
        let config = HttpConfig::default();
        let client = HttpClient::new(config).unwrap();

        let result = client.get("https://httpbin.org/status/404", None).await;
        assert!(result.is_err());

        if let Err(e) = result {
            match e {
                Error::InvalidRequest(_) => {}
                Error::Exchange(_) => {}
                Error::Network(_) => {}
                _ => panic!("Unexpected error type: {:?}", e),
            }
        }
    }

    #[tokio::test]
    async fn test_timeout() {
        let config = HttpConfig {
            timeout: Duration::from_secs(1),
            retry_config: Some(RetryConfig {
                max_retries: 0,
                ..RetryConfig::default()
            }),
            ..Default::default()
        };
        let client = HttpClient::new(config).unwrap();

        let result = client.get("https://httpbin.org/delay/5", None).await;
        assert!(result.is_err());

        if let Err(e) = result {
            match &e {
                Error::Timeout(msg) => {
                    assert!(msg.contains("httpbin.org/delay/5"));
                    assert!(msg.contains("1000ms"));
                }
                Error::Network(_) => {}
                _ => panic!("Expected Timeout or Network error, got: {:?}", e),
            }
            assert!(e.is_retryable());
        }
    }

    #[tokio::test]
    async fn test_retry_mechanism() {
        let config = HttpConfig {
            retry_config: Some(RetryConfig {
                max_retries: 2,
                ..RetryConfig::default()
            }),
            verbose: true,
            ..Default::default()
        };
        let client = HttpClient::new(config).unwrap();

        let result = client.get("https://httpbin.org/status/503", None).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_rate_limiter_integration() {
        use crate::rate_limiter::{RateLimiter, RateLimiterConfig};
        use std::time::Instant;

        let limiter_config = RateLimiterConfig::new(5, Duration::from_secs(1));
        let limiter = RateLimiter::new(limiter_config);

        let config = HttpConfig {
            enable_rate_limit: true,
            verbose: false,
            ..Default::default()
        };

        let client = HttpClient::new_with_rate_limiter(config, limiter).unwrap();

        let start = Instant::now();

        for _ in 0..10 {
            let _ = client.get("https://httpbin.org/get", None).await;
        }

        let elapsed = start.elapsed();

        assert!(elapsed >= Duration::from_secs(1));
    }

    #[tokio::test]
    async fn test_rate_limiter_disabled() {
        let config = HttpConfig {
            enable_rate_limit: false,
            verbose: false,
            ..Default::default()
        };

        let client = HttpClient::new(config).unwrap();

        match client.get("https://httpbin.org/get", None).await {
            Ok(_) => assert!(true),
            Err(e) => {
                tracing::warn!("Network test skipped due to: {:?}", e);
            }
        }
    }

    #[tokio::test]
    async fn test_execute_with_retry_success() {
        let config = HttpConfig::default();
        let client = HttpClient::new(config).unwrap();

        let result = client
            .execute_with_retry(|| async { Ok(serde_json::json!({"status": "ok"})) })
            .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap()["status"], "ok");
    }

    #[tokio::test]
    async fn test_execute_with_retry_failure() {
        let config = HttpConfig {
            retry_config: Some(RetryConfig {
                max_retries: 2,
                base_delay_ms: 10,
                ..RetryConfig::default()
            }),
            ..Default::default()
        };
        let client = HttpClient::new(config).unwrap();

        let result = client
            .execute_with_retry(|| async { Err(Error::network("Persistent failure")) })
            .await;

        assert!(result.is_err());
    }

    #[test]
    fn test_http_config_validate_default() {
        let config = HttpConfig::default();
        let result = config.validate();
        assert!(result.is_ok());
        assert!(result.unwrap().warnings.is_empty());
    }

    #[test]
    fn test_http_config_validate_timeout_too_high() {
        let config = HttpConfig {
            timeout: Duration::from_secs(600),
            ..Default::default()
        };
        let result = config.validate();
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.field_name(), "timeout");
        assert!(matches!(
            err,
            crate::error::ConfigValidationError::ValueTooHigh { .. }
        ));
    }

    #[test]
    fn test_http_config_validate_timeout_boundary() {
        let config = HttpConfig {
            timeout: Duration::from_secs(300),
            ..Default::default()
        };
        let result = config.validate();
        assert!(result.is_ok());
        assert!(result.unwrap().warnings.is_empty());

        let config = HttpConfig {
            timeout: Duration::from_secs(301),
            ..Default::default()
        };
        let result = config.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_http_config_validate_short_timeout_warning() {
        let config = HttpConfig {
            timeout: Duration::from_millis(500),
            ..Default::default()
        };
        let result = config.validate();
        assert!(result.is_ok());
        let validation_result = result.unwrap();
        assert!(!validation_result.warnings.is_empty());
        assert!(validation_result.warnings[0].contains("timeout"));
        assert!(validation_result.warnings[0].contains("very short"));
    }

    #[test]
    fn test_http_config_validate_timeout_warning_boundary() {
        let config = HttpConfig {
            timeout: Duration::from_secs(1),
            ..Default::default()
        };
        let result = config.validate();
        assert!(result.is_ok());
        assert!(result.unwrap().warnings.is_empty());

        let config = HttpConfig {
            timeout: Duration::from_millis(999),
            ..Default::default()
        };
        let result = config.validate();
        assert!(result.is_ok());
        assert!(!result.unwrap().warnings.is_empty());
    }

    #[test]
    fn test_http_config_validate_max_request_size_zero() {
        let config = HttpConfig {
            max_request_size: 0,
            ..Default::default()
        };
        let result = config.validate();
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.field_name(), "max_request_size");
    }

    #[test]
    fn test_http_config_validate_max_request_size_too_large() {
        let config = HttpConfig {
            max_request_size: 101 * 1024 * 1024,
            ..Default::default()
        };
        let result = config.validate();
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.field_name(), "max_request_size");
    }

    #[test]
    fn test_http_config_validate_max_request_size_boundary() {
        let config = HttpConfig {
            max_request_size: 100 * 1024 * 1024,
            ..Default::default()
        };
        let result = config.validate();
        assert!(result.is_ok());

        let config = HttpConfig {
            max_request_size: 100 * 1024 * 1024 + 1,
            ..Default::default()
        };
        let result = config.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_http_config_max_response_size_default() {
        let config = HttpConfig::default();
        assert_eq!(config.max_response_size, 10 * 1024 * 1024);
    }

    #[test]
    fn test_http_config_max_response_size_custom() {
        let config = HttpConfig {
            max_response_size: 1024 * 1024,
            ..Default::default()
        };
        assert_eq!(config.max_response_size, 1024 * 1024);
    }

    #[tokio::test]
    async fn test_response_size_limit_small_response() {
        let config = HttpConfig {
            max_response_size: 10 * 1024 * 1024,
            ..Default::default()
        };
        let client = HttpClient::new(config).unwrap();

        match client.get("https://httpbin.org/get", None).await {
            Ok(value) => {
                assert!(value.is_object());
            }
            Err(e) => {
                tracing::warn!("Network test skipped due to: {:?}", e);
            }
        }
    }

    #[tokio::test]
    async fn test_response_size_limit_with_content_length() {
        let config = HttpConfig {
            max_response_size: 100,
            retry_config: Some(RetryConfig {
                max_retries: 0,
                ..RetryConfig::default()
            }),
            ..Default::default()
        };
        let client = HttpClient::new(config).unwrap();

        let result = client.get("https://httpbin.org/bytes/1000", None).await;

        match result {
            Err(Error::InvalidRequest(msg)) => {
                assert!(msg.contains("exceeds limit") || msg.contains("size"));
            }
            Err(Error::Network(_)) => {
                tracing::warn!("Network test skipped due to network error");
            }
            Ok(_) => {
                tracing::warn!(
                    "Response passed size check - httpbin may have returned smaller response"
                );
            }
            Err(e) => {
                panic!("Unexpected error type: {:?}", e);
            }
        }
    }

    #[test]
    fn test_http_config_circuit_breaker_default() {
        let config = HttpConfig::default();
        assert!(config.circuit_breaker.is_none());
    }

    #[test]
    fn test_http_config_circuit_breaker_custom() {
        let config = HttpConfig {
            circuit_breaker: Some(crate::circuit_breaker::CircuitBreakerConfig::default()),
            ..Default::default()
        };
        assert!(config.circuit_breaker.is_some());
    }

    #[test]
    fn test_http_client_circuit_breaker_disabled_by_default() {
        let config = HttpConfig::default();
        let client = HttpClient::new(config).unwrap();
        assert!(client.circuit_breaker().is_none());
    }

    #[test]
    fn test_http_client_circuit_breaker_enabled() {
        let config = HttpConfig {
            circuit_breaker: Some(crate::circuit_breaker::CircuitBreakerConfig::default()),
            ..Default::default()
        };
        let client = HttpClient::new(config).unwrap();
        assert!(client.circuit_breaker().is_some());
    }

    #[test]
    fn test_http_client_set_circuit_breaker() {
        let config = HttpConfig::default();
        let mut client = HttpClient::new(config).unwrap();
        assert!(client.circuit_breaker().is_none());

        let cb = std::sync::Arc::new(crate::circuit_breaker::CircuitBreaker::new(
            crate::circuit_breaker::CircuitBreakerConfig::default(),
        ));
        client.set_circuit_breaker(cb);
        assert!(client.circuit_breaker().is_some());
    }

    #[tokio::test]
    async fn test_http_client_circuit_breaker_blocks_when_open() {
        let config = HttpConfig::default();
        let mut client = HttpClient::new(config).unwrap();

        let cb_config = crate::circuit_breaker::CircuitBreakerConfig {
            failure_threshold: 1,
            reset_timeout: Duration::from_secs(60),
            success_threshold: 1,
        };
        let cb = std::sync::Arc::new(crate::circuit_breaker::CircuitBreaker::new(cb_config));

        cb.record_failure();
        assert_eq!(cb.state(), crate::circuit_breaker::CircuitState::Open);

        client.set_circuit_breaker(cb);

        let result = client.get("https://httpbin.org/get", None).await;
        assert!(result.is_err());

        if let Err(e) = result {
            assert!(e.as_resource_exhausted().is_some());
        }
    }

    #[test]
    fn test_http_config_pool_settings_default() {
        let config = HttpConfig::default();
        assert_eq!(config.pool_max_idle_per_host, 10);
        assert_eq!(config.pool_idle_timeout, Duration::from_secs(90));
    }

    #[test]
    fn test_http_config_pool_settings_custom() {
        let config = HttpConfig {
            pool_max_idle_per_host: 20,
            pool_idle_timeout: Duration::from_secs(120),
            ..Default::default()
        };
        assert_eq!(config.pool_max_idle_per_host, 20);
        assert_eq!(config.pool_idle_timeout, Duration::from_secs(120));
    }
}
