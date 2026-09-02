//! Contract tests for Anthropic-compatible gateway environment variables.
//!
//! These tests cover the issue #1780 gateway configuration contract.

use futures::StreamExt;
use rustyclawd_core::client::{
    has_anthropic_env_credential, Client, ClientError, Config, CreateMessageRequest, Message,
    RetryConfig, StreamEvent,
};
use secrecy::ExposeSecret;
use serial_test::serial;
use std::ffi::OsString;
use std::path::PathBuf;
use std::time::Duration;
use tempfile::TempDir;
use wiremock::matchers::{body_json, header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const AUTH_TOKEN: &str = "synthetic-opaque-auth-token-1780";
const ESCAPED_AUTH_TOKEN: &str = r#"synthetic-opaque-"quoted"\auth-token-1780"#;
const API_KEY: &str = "sk-ant-synthetic-api-key-1780";
const OTHER_API_KEY: &str = "sk-ant-synthetic-other-key-1780";

struct IsolatedAnthropicEnv {
    saved: Vec<(&'static str, Option<OsString>)>,
    original_cwd: PathBuf,
    _isolated_cwd: TempDir,
}

impl IsolatedAnthropicEnv {
    fn new() -> Self {
        let original_cwd = std::env::current_dir().expect("test cwd should be available");
        let isolated_cwd = tempfile::tempdir().expect("isolated test cwd should be created");
        let names = [
            "ANTHROPIC_AUTH_TOKEN",
            "ANTHROPIC_API_KEY",
            "ANTHROPIC_BASE_URL",
            "ANTHROPIC_MODEL",
            "HOME",
        ];
        let saved = names
            .into_iter()
            .map(|name| (name, std::env::var_os(name)))
            .collect();
        let guard = Self {
            saved,
            original_cwd,
            _isolated_cwd: isolated_cwd,
        };

        std::env::set_current_dir(guard._isolated_cwd.path())
            .expect("test should enter the isolated cwd");

        for name in names {
            std::env::remove_var(name);
        }
        std::env::set_var(
            "HOME",
            "/synthetic/nonexistent/rustyclawd-anthropic-env-test-home",
        );
        guard
    }
}

impl Drop for IsolatedAnthropicEnv {
    fn drop(&mut self) {
        let _cwd_restored = std::env::set_current_dir(&self.original_cwd);
        for (name, value) in &self.saved {
            match value {
                Some(value) => std::env::set_var(name, value),
                None => std::env::remove_var(name),
            }
        }
    }
}

fn selected_secret(config: &Config) -> &str {
    config.api_key.expose_secret().expose()
}

#[test]
#[serial]
fn isolated_environment_restores_cwd_during_panic_unwind() {
    let original_cwd = std::env::current_dir().unwrap();

    let panic = std::panic::catch_unwind(|| {
        let _env = IsolatedAnthropicEnv::new();
        assert_ne!(std::env::current_dir().unwrap(), original_cwd);
        panic!("synthetic panic to exercise RAII restoration");
    });

    assert!(panic.is_err());
    assert_eq!(std::env::current_dir().unwrap(), original_cwd);
}

#[tokio::test]
#[serial]
async fn opaque_auth_token_is_accepted_without_api_key_format_validation() {
    let _env = IsolatedAnthropicEnv::new();
    std::env::set_var("ANTHROPIC_AUTH_TOKEN", AUTH_TOKEN);

    let config = Config::from_default_location()
        .await
        .expect("an opaque auth token should authenticate Anthropic-compatible gateways");

    assert_eq!(selected_secret(&config), AUTH_TOKEN);
}

#[tokio::test]
#[serial]
async fn auth_token_takes_precedence_over_anthropic_api_key() {
    let _env = IsolatedAnthropicEnv::new();
    std::env::set_var("ANTHROPIC_AUTH_TOKEN", AUTH_TOKEN);
    std::env::set_var("ANTHROPIC_API_KEY", OTHER_API_KEY);

    let config = Config::from_default_location().await.unwrap();

    assert_eq!(selected_secret(&config), AUTH_TOKEN);
}

#[tokio::test]
#[serial]
async fn blank_auth_token_falls_back_to_validated_anthropic_api_key() {
    for blank in ["", " \t\r\n "] {
        let _env = IsolatedAnthropicEnv::new();
        std::env::set_var("ANTHROPIC_AUTH_TOKEN", blank);
        std::env::set_var("ANTHROPIC_API_KEY", API_KEY);

        let config = Config::from_default_location().await.unwrap();

        assert_eq!(selected_secret(&config), API_KEY);
    }
}

#[tokio::test]
#[serial]
async fn fallback_anthropic_api_key_keeps_existing_format_validation() {
    let _env = IsolatedAnthropicEnv::new();
    std::env::set_var("ANTHROPIC_AUTH_TOKEN", "");
    std::env::set_var("ANTHROPIC_API_KEY", "synthetic-invalid-api-key-1780");

    let error = Config::from_default_location().await.unwrap_err();

    assert!(matches!(error, ClientError::InvalidApiKey));
}

#[tokio::test]
#[serial]
async fn whitespace_only_anthropic_credentials_are_treated_as_unset() {
    let _env = IsolatedAnthropicEnv::new();
    std::env::set_var("ANTHROPIC_AUTH_TOKEN", " \t ");
    std::env::set_var("ANTHROPIC_API_KEY", "\r\n");

    let error = Config::from_default_location().await.unwrap_err();

    assert!(matches!(error, ClientError::ApiKeyNotFound));
}

#[test]
#[serial]
fn credential_status_recognizes_only_trimmed_non_empty_values() {
    let _env = IsolatedAnthropicEnv::new();

    std::env::set_var("ANTHROPIC_AUTH_TOKEN", " \t ");
    std::env::set_var("ANTHROPIC_API_KEY", "\r\n");
    assert!(!has_anthropic_env_credential());

    std::env::set_var("ANTHROPIC_AUTH_TOKEN", " synthetic-status-token ");
    assert!(has_anthropic_env_credential());
}

#[test]
#[serial]
fn base_url_and_model_alone_are_not_credentials() {
    let _env = IsolatedAnthropicEnv::new();
    std::env::set_var(
        "ANTHROPIC_BASE_URL",
        "https://gateway.synthetic.invalid/anthropic",
    );
    std::env::set_var("ANTHROPIC_MODEL", "synthetic-model-without-credential");

    assert!(!has_anthropic_env_credential());
}

#[tokio::test]
#[serial]
async fn base_url_is_trimmed_and_normalized_without_a_messages_suffix() {
    let _env = IsolatedAnthropicEnv::new();
    std::env::set_var("ANTHROPIC_API_KEY", API_KEY);
    std::env::set_var(
        "ANTHROPIC_BASE_URL",
        "  https://gateway.synthetic.invalid/anthropic///  ",
    );

    let config = Config::from_default_location().await.unwrap();

    assert_eq!(
        config.api_url,
        "https://gateway.synthetic.invalid/anthropic"
    );
}

#[tokio::test]
#[serial]
async fn gateway_request_uses_exact_normalized_v1_messages_url_and_auth_token() {
    let _env = IsolatedAnthropicEnv::new();
    let server = MockServer::start().await;
    std::env::set_var("ANTHROPIC_AUTH_TOKEN", AUTH_TOKEN);
    std::env::set_var("ANTHROPIC_BASE_URL", format!("{}/gateway///", server.uri()));

    let request = CreateMessageRequest::new(
        "synthetic-anthropic-model-1780",
        vec![Message::user("synthetic request")],
        64,
    );
    Mock::given(method("POST"))
        .and(path("/gateway/v1/messages"))
        .and(header("x-api-key", AUTH_TOKEN))
        .and(body_json(serde_json::json!({
            "model": "synthetic-anthropic-model-1780",
            "max_tokens": 64,
            "messages": [{"role": "user", "content": "synthetic request"}],
            "stream": false
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "msg_synthetic_1780",
            "type": "message",
            "role": "assistant",
            "content": [{"type": "text", "text": "synthetic response"}],
            "model": "synthetic-anthropic-model-1780",
            "stop_reason": "end_turn",
            "usage": {"input_tokens": 1, "output_tokens": 1}
        })))
        .expect(1)
        .mount(&server)
        .await;

    let config = Config::from_default_location().await.unwrap();
    let client = Client::with_retry_config(
        config,
        RetryConfig {
            max_retries: 0,
            initial_delay: Duration::ZERO,
            max_delay: Duration::ZERO,
            jitter_factor: 0.0,
        },
    )
    .unwrap();

    client.create_message(request).await.unwrap();
}

#[tokio::test]
#[serial]
async fn echoed_opaque_credential_is_redacted_from_response_errors() {
    let _env = IsolatedAnthropicEnv::new();
    let server = MockServer::start().await;
    std::env::set_var("ANTHROPIC_AUTH_TOKEN", ESCAPED_AUTH_TOKEN);
    std::env::set_var("ANTHROPIC_BASE_URL", server.uri());

    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(ResponseTemplate::new(401).set_body_json(serde_json::json!({
            "error": {
                "message": format!(
                    "gateway rejected opaque credential {ESCAPED_AUTH_TOKEN}"
                )
            }
        })))
        .expect(2)
        .mount(&server)
        .await;

    let config = Config::from_default_location().await.unwrap();
    let client = Client::with_retry_config(
        config,
        RetryConfig {
            max_retries: 0,
            initial_delay: Duration::ZERO,
            max_delay: Duration::ZERO,
            jitter_factor: 0.0,
        },
    )
    .unwrap();
    let request = || {
        CreateMessageRequest::new(
            "synthetic-anthropic-model-1780",
            vec![Message::user("synthetic request")],
            64,
        )
    };

    let non_streaming_error = client
        .create_message(request())
        .await
        .expect_err("non-streaming response must be an error");
    let streaming_error = client
        .create_message_stream(request())
        .await
        .err()
        .expect("streaming response must be an error");

    for error in [non_streaming_error, streaming_error] {
        let rendered = format!("{error:?} {error}");
        assert!(matches!(error, ClientError::Unauthorized(_)));
        assert_secret_is_redacted(&rendered, ESCAPED_AUTH_TOKEN);
        assert!(rendered.contains("[REDACTED_API_KEY]"));
    }
}

#[tokio::test]
#[serial]
async fn successful_sse_error_event_redacts_active_opaque_credential() {
    let _env = IsolatedAnthropicEnv::new();
    let server = MockServer::start().await;
    std::env::set_var("ANTHROPIC_AUTH_TOKEN", ESCAPED_AUTH_TOKEN);
    std::env::set_var("ANTHROPIC_BASE_URL", server.uri());

    let event = serde_json::json!({
        "type": "error",
        "error": {
            "type": format!("authentication_error_{ESCAPED_AUTH_TOKEN}"),
            "message": format!("gateway rejected {ESCAPED_AUTH_TOKEN}")
        }
    });
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(format!("event: error\ndata: {event}\n\n")),
        )
        .expect(1)
        .mount(&server)
        .await;

    let client = gateway_client().await;
    let mut stream = client
        .create_message_stream(gateway_request())
        .await
        .expect("HTTP 200 SSE response should create a stream");
    let event = stream
        .next()
        .await
        .expect("SSE response should contain an event")
        .expect("valid SSE error event should parse");
    let StreamEvent::Error { error } = event else {
        panic!("expected an SSE error event");
    };
    let rendered = format!("{error:?} {}", error.message);

    assert_secret_is_redacted(&rendered, ESCAPED_AUTH_TOKEN);
    assert!(rendered.contains("[REDACTED_API_KEY]"));
}

#[tokio::test]
#[serial]
async fn successful_sse_parser_error_redacts_active_opaque_credential() {
    let _env = IsolatedAnthropicEnv::new();
    let server = MockServer::start().await;
    std::env::set_var("ANTHROPIC_AUTH_TOKEN", ESCAPED_AUTH_TOKEN);
    std::env::set_var("ANTHROPIC_BASE_URL", server.uri());

    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(format!("data: incomplete {ESCAPED_AUTH_TOKEN}")),
        )
        .expect(1)
        .mount(&server)
        .await;

    let client = gateway_client().await;
    let mut stream = client
        .create_message_stream(gateway_request())
        .await
        .expect("HTTP 200 SSE response should create a stream");
    let error = stream
        .next()
        .await
        .expect("incomplete SSE response should produce an error")
        .expect_err("incomplete SSE response must not parse");
    let rendered = format!("{error:?} {error}");

    assert_secret_is_redacted(&rendered, ESCAPED_AUTH_TOKEN);
    assert!(rendered.contains("[REDACTED_API_KEY]"));
}

#[tokio::test]
#[serial]
async fn opaque_secret_is_redacted_from_debug_and_errors() {
    let _env = IsolatedAnthropicEnv::new();
    std::env::set_var("ANTHROPIC_AUTH_TOKEN", AUTH_TOKEN);
    std::env::set_var("ANTHROPIC_BASE_URL", "synthetic malformed url");

    let config = Config::from_default_location().await.unwrap();
    let debug = format!("{config:?}");
    let error = Client::new(config)
        .err()
        .map(|error| format!("{error:?} {error}"))
        .unwrap_or_default();

    assert!(!debug.contains(AUTH_TOKEN));
    assert!(!error.contains(AUTH_TOKEN));
}

fn gateway_request() -> CreateMessageRequest {
    CreateMessageRequest::new(
        "synthetic-anthropic-model-1780",
        vec![Message::user("synthetic request")],
        64,
    )
}

async fn gateway_client() -> Client {
    let config = Config::from_default_location().await.unwrap();
    Client::with_retry_config(
        config,
        RetryConfig {
            max_retries: 0,
            initial_delay: Duration::ZERO,
            max_delay: Duration::ZERO,
            jitter_factor: 0.0,
        },
    )
    .unwrap()
}

fn assert_secret_is_redacted(rendered: &str, secret: &str) {
    let json = serde_json::to_string(secret).unwrap();
    let json_escaped = json
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
        .unwrap();

    assert!(!rendered.contains(secret));
    assert!(!rendered.contains(json_escaped));
}
