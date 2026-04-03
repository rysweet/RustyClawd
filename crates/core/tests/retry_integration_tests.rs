//! Integration tests for retry logic with mock HTTP server
//!
//! These tests verify the retry behavior with actual HTTP requests
//! using wiremock to simulate various server responses.

use rustyclawd_core::client::{ApiKey, Client, Config, CreateMessageRequest, Message, RetryConfig};
use std::time::{Duration, Instant};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Helper to create a test client with custom config
fn create_test_client(api_url: &str, retry_config: RetryConfig) -> Client {
    let api_key = ApiKey::new("sk-ant-test123".to_string()).unwrap();
    let config = Config::new(api_key)
        .with_api_url(api_url.to_string())
        .with_timeout_secs(10); // Short timeout for tests
    Client::with_retry_config(config, retry_config).expect("Failed to build HTTP client")
}

/// Helper to create a test request
fn create_test_request() -> CreateMessageRequest {
    CreateMessageRequest::new(
        "claude-3-5-sonnet-20241022",
        vec![Message::user("Hello")],
        1024,
    )
}

/// Helper to create a successful response
fn success_response() -> ResponseTemplate {
    ResponseTemplate::new(200).set_body_json(serde_json::json!({
        "id": "msg_123",
        "type": "message",
        "role": "assistant",
        "content": [{"type": "text", "text": "Hello!"}],
        "model": "claude-3-5-sonnet-20241022",
        "stop_reason": "end_turn",
        "usage": {"input_tokens": 10, "output_tokens": 5}
    }))
}

#[tokio::test]
async fn test_retry_on_rate_limit_429() {
    let mock_server = MockServer::start().await;

    // First request: 429 with Retry-After
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(
            ResponseTemplate::new(429)
                .set_body_string("Rate limit exceeded")
                .append_header("retry-after", "1"),
        )
        .up_to_n_times(2)
        .mount(&mock_server)
        .await;

    // Third request: Success
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(success_response())
        .expect(1)
        .mount(&mock_server)
        .await;

    let retry_config = RetryConfig {
        max_retries: 3,
        initial_delay: Duration::from_millis(100),
        max_delay: Duration::from_secs(5),
        jitter_factor: 0.0, // No jitter for deterministic testing
    };

    let client = create_test_client(&mock_server.uri(), retry_config);
    let request = create_test_request();

    let start = Instant::now();
    let result = client.create_message(request).await;
    let elapsed = start.elapsed();

    // Should succeed after retries
    assert!(result.is_ok());

    // Should have taken at least 2 seconds (2 retries with 1s Retry-After)
    assert!(elapsed >= Duration::from_secs(2));
}

#[tokio::test]
async fn test_retry_on_503_service_unavailable() {
    let mock_server = MockServer::start().await;

    // First request: 503
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(
            ResponseTemplate::new(503)
                .set_body_string("Service temporarily unavailable")
                .append_header("retry-after", "1"),
        )
        .up_to_n_times(1)
        .mount(&mock_server)
        .await;

    // Second request: Success
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(success_response())
        .expect(1)
        .mount(&mock_server)
        .await;

    let retry_config = RetryConfig {
        max_retries: 3,
        initial_delay: Duration::from_millis(100),
        max_delay: Duration::from_secs(5),
        jitter_factor: 0.0, // No jitter for deterministic testing
    };

    let client = create_test_client(&mock_server.uri(), retry_config);
    let request = create_test_request();

    let result = client.create_message(request).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_retry_on_500_server_error() {
    let mock_server = MockServer::start().await;

    // First request: 500
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(ResponseTemplate::new(500).set_body_string("Internal server error"))
        .up_to_n_times(2)
        .mount(&mock_server)
        .await;

    // Third request: Success
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(success_response())
        .expect(1)
        .mount(&mock_server)
        .await;

    let retry_config = RetryConfig {
        max_retries: 3,
        initial_delay: Duration::from_millis(100),
        max_delay: Duration::from_secs(5),
        jitter_factor: 0.0, // No jitter for deterministic testing
    };

    let client = create_test_client(&mock_server.uri(), retry_config);
    let request = create_test_request();

    let result = client.create_message(request).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_no_retry_on_400_bad_request() {
    let mock_server = MockServer::start().await;

    // Always return 400
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(ResponseTemplate::new(400).set_body_string("Bad request"))
        .expect(1) // Should only be called once
        .mount(&mock_server)
        .await;

    let retry_config = RetryConfig {
        max_retries: 3,
        initial_delay: Duration::from_millis(100),
        max_delay: Duration::from_secs(5),
        jitter_factor: 0.0, // No jitter for deterministic testing
    };

    let client = create_test_client(&mock_server.uri(), retry_config);
    let request = create_test_request();

    let result = client.create_message(request).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_retry_on_401_unauthorized() {
    // 401 is retryable because Azure AD tokens expire hourly.
    // The retry loop invalidates the cached token before each retry.
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(ResponseTemplate::new(401).set_body_string("Unauthorized"))
        .expect(4) // 1 initial + 3 retries
        .mount(&mock_server)
        .await;

    let retry_config = RetryConfig {
        max_retries: 3,
        initial_delay: Duration::from_millis(100),
        max_delay: Duration::from_secs(5),
        jitter_factor: 0.0, // No jitter for deterministic testing
    };

    let client = create_test_client(&mock_server.uri(), retry_config);
    let request = create_test_request();

    let result = client.create_message(request).await;
    assert!(result.is_err()); // Still fails after exhausting retries
}

#[tokio::test]
async fn test_no_retry_on_403_forbidden() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(ResponseTemplate::new(403).set_body_string("Forbidden"))
        .expect(1)
        .mount(&mock_server)
        .await;

    let retry_config = RetryConfig {
        max_retries: 3,
        initial_delay: Duration::from_millis(100),
        max_delay: Duration::from_secs(5),
        jitter_factor: 0.0, // No jitter for deterministic testing
    };

    let client = create_test_client(&mock_server.uri(), retry_config);
    let request = create_test_request();

    let result = client.create_message(request).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_max_retries_exceeded() {
    let mock_server = MockServer::start().await;

    // Always return 429
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(
            ResponseTemplate::new(429)
                .set_body_string("Rate limit exceeded")
                .append_header("retry-after", "1"),
        )
        .expect(4) // Initial + 3 retries
        .mount(&mock_server)
        .await;

    let retry_config = RetryConfig {
        max_retries: 3,
        initial_delay: Duration::from_millis(100),
        max_delay: Duration::from_secs(5),
        jitter_factor: 0.0, // No jitter for deterministic testing
    };

    let client = create_test_client(&mock_server.uri(), retry_config);
    let request = create_test_request();

    let result = client.create_message(request).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_exponential_backoff() {
    let mock_server = MockServer::start().await;

    // Fail 3 times, then succeed
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(ResponseTemplate::new(503).set_body_string("Service unavailable"))
        .up_to_n_times(3)
        .mount(&mock_server)
        .await;

    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(success_response())
        .expect(1)
        .mount(&mock_server)
        .await;

    let retry_config = RetryConfig {
        max_retries: 3,
        initial_delay: Duration::from_millis(500), // 500ms
        max_delay: Duration::from_secs(5),
        jitter_factor: 0.0, // No jitter for deterministic testing
    };

    let client = create_test_client(&mock_server.uri(), retry_config);
    let request = create_test_request();

    let start = Instant::now();
    let result = client.create_message(request).await;
    let elapsed = start.elapsed();

    assert!(result.is_ok());

    // Exponential backoff: 500ms, 1000ms, 2000ms = 3500ms total
    // Allow some tolerance for test execution overhead
    assert!(elapsed >= Duration::from_millis(3000));
    assert!(elapsed < Duration::from_millis(5000));
}

#[tokio::test]
async fn test_retry_respects_retry_after_header() {
    let mock_server = MockServer::start().await;

    // First request: 429 with Retry-After=2
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(
            ResponseTemplate::new(429)
                .set_body_string("Rate limit exceeded")
                .append_header("retry-after", "2"),
        )
        .up_to_n_times(1)
        .mount(&mock_server)
        .await;

    // Second request: Success
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(success_response())
        .expect(1)
        .mount(&mock_server)
        .await;

    let retry_config = RetryConfig {
        max_retries: 3,
        initial_delay: Duration::from_millis(100), // Should be overridden by Retry-After
        max_delay: Duration::from_secs(10),
        jitter_factor: 0.0, // No jitter for deterministic testing
    };

    let client = create_test_client(&mock_server.uri(), retry_config);
    let request = create_test_request();

    let start = Instant::now();
    let result = client.create_message(request).await;
    let elapsed = start.elapsed();

    assert!(result.is_ok());

    // Should wait at least 2 seconds (from Retry-After header)
    assert!(elapsed >= Duration::from_secs(2));
}

#[tokio::test]
async fn test_custom_retry_config() {
    let mock_server = MockServer::start().await;

    // Fail once, then succeed
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(ResponseTemplate::new(503).set_body_string("Service unavailable"))
        .up_to_n_times(1)
        .mount(&mock_server)
        .await;

    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(success_response())
        .expect(1)
        .mount(&mock_server)
        .await;

    // Custom config: 5 max retries, 200ms initial delay
    let retry_config = RetryConfig {
        max_retries: 5,
        initial_delay: Duration::from_millis(200),
        max_delay: Duration::from_secs(10),
        jitter_factor: 0.0, // No jitter for deterministic testing
    };

    let client = create_test_client(&mock_server.uri(), retry_config);
    let request = create_test_request();

    let result = client.create_message(request).await;
    assert!(result.is_ok());
}
