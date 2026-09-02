//! CLI contract tests for Anthropic gateway model and provider precedence.

use serde_json::Value;
use std::fs;
use std::process::Output;
use tempfile::TempDir;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const AUTH_TOKEN: &str = "synthetic-cli-auth-token-1780";
const COPILOT_TOKEN: &str = "synthetic-cli-copilot-token-1780";
const ENV_MODEL: &str = "synthetic-anthropic-env-model-1780";
const CLI_MODEL: &str = "synthetic-anthropic-cli-model-1780";
const SETTINGS_MODEL: &str = "synthetic-anthropic-settings-model-1780";
const SETTINGS_FIXTURE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/anthropic_gateway_settings.json"
);

async fn run_print(server: &MockServer, extra_args: &[&str]) -> Output {
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "msg_synthetic_cli_1780",
            "type": "message",
            "role": "assistant",
            "content": [{"type": "text", "text": "synthetic response"}],
            "model": ENV_MODEL,
            "stop_reason": "end_turn",
            "usage": {"input_tokens": 1, "output_tokens": 1}
        })))
        .expect(1)
        .mount(server)
        .await;

    let binary = assert_cmd::cargo::cargo_bin!("rusty");
    let server_uri = server.uri();
    let args = extra_args
        .iter()
        .map(|arg| arg.to_string())
        .collect::<Vec<_>>();
    tokio::task::spawn_blocking(move || {
        std::process::Command::new(binary)
            .args(["--print", "--provider", "anthropic"])
            .args(args)
            .arg("synthetic prompt")
            .env_remove("ANTHROPIC_API_KEY")
            .env_remove("CLAUDE_MODEL")
            .env_remove("CLAUDE_API_URL")
            .env("ANTHROPIC_AUTH_TOKEN", AUTH_TOKEN)
            .env("ANTHROPIC_BASE_URL", server_uri)
            .env("ANTHROPIC_MODEL", ENV_MODEL)
            .output()
            .expect("run rusty print mode")
    })
    .await
    .expect("join rusty process")
}

async fn only_request_body(server: &MockServer) -> Value {
    let requests = server
        .received_requests()
        .await
        .expect("wiremock request recording");
    assert_eq!(requests.len(), 1, "expected exactly one Anthropic request");
    serde_json::from_slice(&requests[0].body).expect("request body should be JSON")
}

#[tokio::test]
async fn print_mode_implicitly_falls_back_to_copilot_without_provider() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/models"))
        .respond_with(ResponseTemplate::new(200).set_body_string("[]"))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "chatcmpl-synthetic-fallback",
            "model": "claude-sonnet-4.6",
            "choices": [{
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": "synthetic Copilot fallback response"
                },
                "finish_reason": "stop"
            }],
            "usage": {
                "prompt_tokens": 1,
                "completion_tokens": 1,
                "total_tokens": 2
            }
        })))
        .expect(1)
        .mount(&server)
        .await;

    let isolated_home = TempDir::new().expect("create isolated home");
    let binary = assert_cmd::cargo::cargo_bin!("rusty");
    let server_uri = server.uri();
    let isolated_path = isolated_home.path().to_path_buf();
    let output = tokio::task::spawn_blocking(move || {
        std::process::Command::new(binary)
            .args(["--print", "synthetic prompt"])
            .current_dir(&isolated_path)
            .env_clear()
            .env("HOME", &isolated_path)
            .env("XDG_CONFIG_HOME", &isolated_path)
            .env("ANTHROPIC_MODEL", "synthetic-anthropic-only-model")
            .env("GITHUB_TOKEN", COPILOT_TOKEN)
            .env("GITHUB_COPILOT_ENDPOINT", server_uri)
            .output()
            .expect("run rusty with implicit Copilot fallback")
    })
    .await
    .expect("join rusty process");

    assert!(
        output.status.success(),
        "print mode failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("synthetic Copilot fallback response"));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("No Anthropic credential found"));
    assert!(stderr.contains("Using GitHub Copilot backend"));

    let requests = server
        .received_requests()
        .await
        .expect("wiremock request recording");
    let chat_request = requests
        .iter()
        .find(|request| request.url.path() == "/chat/completions")
        .expect("Copilot chat request");
    let body: Value =
        serde_json::from_slice(&chat_request.body).expect("Copilot request body should be JSON");
    assert_eq!(body["model"], "claude-sonnet-4.6");
    assert_ne!(body["model"], "synthetic-anthropic-only-model");
}

#[tokio::test]
async fn print_mode_fallback_failure_retains_context_and_redacts_token() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/models"))
        .respond_with(
            ResponseTemplate::new(403)
                .set_body_string(format!("Copilot rejected credential {COPILOT_TOKEN}")),
        )
        .expect(1)
        .mount(&server)
        .await;

    let isolated_home = TempDir::new().expect("create isolated home");
    let binary = assert_cmd::cargo::cargo_bin!("rusty");
    let server_uri = server.uri();
    let isolated_path = isolated_home.path().to_path_buf();
    let output = tokio::task::spawn_blocking(move || {
        std::process::Command::new(binary)
            .args(["--print", "synthetic prompt"])
            .current_dir(&isolated_path)
            .env_clear()
            .env("HOME", &isolated_path)
            .env("XDG_CONFIG_HOME", &isolated_path)
            .env("GITHUB_TOKEN", COPILOT_TOKEN)
            .env("GITHUB_COPILOT_ENDPOINT", server_uri)
            .output()
            .expect("run rusty with failing Copilot fallback")
    })
    .await
    .expect("join rusty process");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Anthropic API key not found"));
    assert!(stderr.contains("Or use GitHub Copilot instead"));
    assert!(stderr.contains("Copilot credential validation failed (HTTP 403"));
    assert!(stderr.contains("Copilot rejected credential [REDACTED_API_KEY]"));
    assert!(!stderr.contains(COPILOT_TOKEN));
    server.verify().await;
}

#[tokio::test]
async fn print_mode_uses_anthropic_model_as_anthropic_default() {
    let server = MockServer::start().await;

    let output = run_print(&server, &[]).await;

    assert!(
        output.status.success(),
        "print mode failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(only_request_body(&server).await["model"], ENV_MODEL);
}

#[tokio::test]
async fn cli_model_takes_precedence_over_anthropic_model_in_print_mode() {
    let server = MockServer::start().await;

    let output = run_print(
        &server,
        &["--settings", SETTINGS_FIXTURE, "--model", CLI_MODEL],
    )
    .await;

    assert!(
        output.status.success(),
        "print mode failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(only_request_body(&server).await["model"], CLI_MODEL);
}

#[tokio::test]
async fn settings_model_takes_precedence_over_anthropic_model_in_print_mode() {
    let server = MockServer::start().await;

    let output = run_print(&server, &["--settings", SETTINGS_FIXTURE]).await;

    assert!(
        output.status.success(),
        "print mode failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(only_request_body(&server).await["model"], SETTINGS_MODEL);
}

#[tokio::test]
async fn settings_api_url_wins_at_the_cli_http_boundary() {
    let environment_server = MockServer::start().await;
    let settings_server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "msg_settings_boundary_1780",
            "type": "message",
            "role": "assistant",
            "content": [{"type": "text", "text": "settings endpoint response"}],
            "model": SETTINGS_MODEL,
            "stop_reason": "end_turn",
            "usage": {"input_tokens": 1, "output_tokens": 1}
        })))
        .expect(1)
        .mount(&settings_server)
        .await;

    let temp_dir = TempDir::new().expect("create isolated settings directory");
    let settings_path = temp_dir.path().join("settings.json");
    fs::write(
        &settings_path,
        serde_json::json!({
            "api_url": settings_server.uri(),
            "model": SETTINGS_MODEL
        })
        .to_string(),
    )
    .expect("write settings fixture");

    let binary = assert_cmd::cargo::cargo_bin!("rusty");
    let output = tokio::task::spawn_blocking(move || {
        std::process::Command::new(binary)
            .args(["--print", "--provider", "anthropic", "--settings"])
            .arg(settings_path)
            .arg("synthetic prompt")
            .env_remove("ANTHROPIC_API_KEY")
            .env_remove("CLAUDE_MODEL")
            .env_remove("CLAUDE_API_URL")
            .env("ANTHROPIC_AUTH_TOKEN", AUTH_TOKEN)
            .env("ANTHROPIC_BASE_URL", environment_server.uri())
            .env("ANTHROPIC_MODEL", ENV_MODEL)
            .output()
            .expect("run rusty with settings API URL")
    })
    .await
    .expect("join rusty process");

    assert!(
        output.status.success(),
        "print mode failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        settings_server
            .received_requests()
            .await
            .expect("settings request recording")
            .len(),
        1
    );
}

#[test]
fn anthropic_environment_does_not_override_explicit_provider_selection() {
    let output = std::process::Command::new(assert_cmd::cargo::cargo_bin!("rusty"))
        .args(["--print", "--provider", "azure", "synthetic prompt"])
        .env_remove("ANTHROPIC_API_KEY")
        .env_remove("CLAUDE_MODEL")
        .env_remove("CLAUDE_API_URL")
        .env("ANTHROPIC_AUTH_TOKEN", AUTH_TOKEN)
        .env(
            "ANTHROPIC_BASE_URL",
            "https://gateway.synthetic.invalid/anthropic",
        )
        .env("ANTHROPIC_MODEL", ENV_MODEL)
        .output()
        .expect("run rusty with explicit Azure provider");
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(!output.status.success());
    assert!(stderr.contains("Azure AI Foundry requires --azure-endpoint"));
    assert!(!stderr.contains(AUTH_TOKEN));
    assert!(!stderr.contains(ENV_MODEL));
}

#[test]
fn auth_diagnostics_recognize_token_and_reject_blank_credentials() {
    let binary = assert_cmd::cargo::cargo_bin!("rusty");
    let configured = std::process::Command::new(binary)
        .args(["auth", "status"])
        .env_remove("ANTHROPIC_API_KEY")
        .env("ANTHROPIC_AUTH_TOKEN", " synthetic-diagnostic-token ")
        .output()
        .expect("run auth status with token");
    assert!(configured.status.success());
    assert!(String::from_utf8_lossy(&configured.stdout)
        .contains("Anthropic environment credential is configured"));

    let blank = std::process::Command::new(binary)
        .args(["auth", "status"])
        .env("ANTHROPIC_API_KEY", " \t ")
        .env("ANTHROPIC_AUTH_TOKEN", "\r\n")
        .output()
        .expect("run auth status with blank credentials");
    assert!(blank.status.success());
    assert!(String::from_utf8_lossy(&blank.stdout)
        .contains("Anthropic environment credential is NOT configured"));
}
