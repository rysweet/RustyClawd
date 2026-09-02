//! Shared Anthropic configuration and model selection for CLI runtime modes.

use rustyclawd_core::client::{Backend, ClientResult, Config};

const DEFAULT_PRINT_ANTHROPIC_MODEL: &str = "claude-sonnet-4-6";
const DEFAULT_INTERACTIVE_ANTHROPIC_MODEL: &str = "claude-opus-4-6";
const DEFAULT_COPILOT_MODEL: &str = "claude-sonnet-4.6";

/// Runtime mode whose established Anthropic default should be preserved.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuntimeMode {
    Print,
    Interactive,
}

/// Credential-backed Anthropic configuration resolved for a runtime mode.
pub struct ResolvedAnthropicConfig {
    pub config: Config,
    pub model: String,
}

/// Resolve the complete Anthropic configuration shared by print and interactive modes.
pub async fn resolve_anthropic_config(
    cli_model: Option<&str>,
    settings_model: Option<&str>,
    settings_api_url: Option<&str>,
    runtime_mode: RuntimeMode,
) -> ClientResult<ResolvedAnthropicConfig> {
    let mut config = Config::from_default_location().await?;
    if let Some(api_url) = settings_api_url.and_then(non_empty) {
        config = config.with_api_url(api_url.to_string());
    }

    Ok(ResolvedAnthropicConfig {
        config,
        model: resolve_model(Backend::Anthropic, cli_model, settings_model, runtime_mode),
    })
}

/// Resolve the effective model using CLI, settings, environment, and backend defaults.
pub fn resolve_model(
    backend: Backend,
    cli_model: Option<&str>,
    settings_model: Option<&str>,
    runtime_mode: RuntimeMode,
) -> String {
    let environment_model = if backend == Backend::Anthropic {
        std::env::var("ANTHROPIC_MODEL").ok()
    } else {
        None
    };

    resolve_model_with_env(
        backend,
        cli_model,
        settings_model,
        environment_model.as_deref(),
        runtime_mode,
    )
}

/// Pure model resolver used by all runtime modes and resolver contract tests.
fn resolve_model_with_env(
    backend: Backend,
    cli_model: Option<&str>,
    settings_model: Option<&str>,
    anthropic_environment_model: Option<&str>,
    runtime_mode: RuntimeMode,
) -> String {
    let environment_model = match backend {
        Backend::Anthropic => anthropic_environment_model.and_then(non_empty),
        Backend::Copilot | Backend::AzureFoundry => None,
    };
    let configured_model = cli_model
        .and_then(non_empty)
        .or_else(|| settings_model.and_then(non_empty))
        .or(environment_model);

    match configured_model {
        Some(model) => resolve_model_name(model, backend),
        None => default_model(backend, runtime_mode).to_string(),
    }
}

/// Resolve a shorthand model alias for the selected backend.
pub fn resolve_model_name(model: &str, backend: Backend) -> String {
    match (backend, model) {
        (Backend::Copilot, "sonnet") => "claude-sonnet-4.6".to_string(),
        (Backend::Copilot, "opus") => "claude-opus-4.6".to_string(),
        (Backend::Copilot, "haiku") => "claude-haiku-4.5".to_string(),
        (_, "sonnet") => "claude-sonnet-4-6".to_string(),
        (_, "opus") => "claude-opus-4-6".to_string(),
        (_, "haiku") => "claude-haiku-4-5-20251001".to_string(),
        (_, custom) => custom.to_string(),
    }
}

fn default_model(backend: Backend, runtime_mode: RuntimeMode) -> &'static str {
    match backend {
        Backend::Anthropic => match runtime_mode {
            RuntimeMode::Print => DEFAULT_PRINT_ANTHROPIC_MODEL,
            RuntimeMode::Interactive => DEFAULT_INTERACTIVE_ANTHROPIC_MODEL,
        },
        Backend::Copilot | Backend::AzureFoundry => DEFAULT_COPILOT_MODEL,
    }
}

/// Whether an implicit Anthropic selection may fall back to Copilot.
pub fn copilot_fallback_eligible(explicit_provider: Option<Backend>) -> bool {
    explicit_provider.is_none()
}

fn non_empty(value: &str) -> Option<&str> {
    let trimmed = value.trim();
    (!trimmed.is_empty()).then_some(trimmed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::ffi::OsString;

    struct SavedEnv {
        values: Vec<(&'static str, Option<OsString>)>,
    }

    impl SavedEnv {
        fn set(values: &[(&'static str, &str)]) -> Self {
            let names = [
                "ANTHROPIC_AUTH_TOKEN",
                "ANTHROPIC_API_KEY",
                "ANTHROPIC_BASE_URL",
                "ANTHROPIC_MODEL",
            ];
            let saved = names
                .iter()
                .map(|name| (*name, std::env::var_os(name)))
                .collect();
            for name in names {
                std::env::remove_var(name);
            }
            for (name, value) in values {
                std::env::set_var(name, value);
            }
            Self { values: saved }
        }
    }

    impl Drop for SavedEnv {
        fn drop(&mut self) {
            for (name, value) in &self.values {
                match value {
                    Some(value) => std::env::set_var(name, value),
                    None => std::env::remove_var(name),
                }
            }
        }
    }

    #[test]
    fn precedence_is_consistent_for_anthropic() {
        assert_eq!(
            resolve_model_with_env(
                Backend::Anthropic,
                Some("cli"),
                Some("settings"),
                Some("environment"),
                RuntimeMode::Print,
            ),
            "cli"
        );
        assert_eq!(
            resolve_model_with_env(
                Backend::Anthropic,
                None,
                Some("settings"),
                Some("environment"),
                RuntimeMode::Print,
            ),
            "settings"
        );
        assert_eq!(
            resolve_model_with_env(
                Backend::Anthropic,
                None,
                None,
                Some("environment"),
                RuntimeMode::Print,
            ),
            "environment"
        );
    }

    #[test]
    fn runtime_modes_preserve_their_distinct_anthropic_defaults() {
        assert_eq!(
            resolve_model_with_env(Backend::Anthropic, None, None, None, RuntimeMode::Print,),
            DEFAULT_PRINT_ANTHROPIC_MODEL
        );
        assert_eq!(
            resolve_model_with_env(
                Backend::Anthropic,
                None,
                None,
                None,
                RuntimeMode::Interactive,
            ),
            DEFAULT_INTERACTIVE_ANTHROPIC_MODEL
        );
    }

    #[test]
    fn precedence_ignores_blank_values_and_trims_the_selected_model() {
        assert_eq!(
            resolve_model_with_env(
                Backend::Anthropic,
                Some(" \t "),
                Some(" settings-model "),
                Some(" environment-model "),
                RuntimeMode::Print,
            ),
            "settings-model"
        );
        assert_eq!(
            resolve_model_with_env(
                Backend::Anthropic,
                None,
                Some("\r\n"),
                Some(" environment-model "),
                RuntimeMode::Interactive,
            ),
            "environment-model"
        );
    }

    #[test]
    fn aliases_are_backend_aware() {
        assert_eq!(
            resolve_model_name("opus", Backend::Anthropic),
            "claude-opus-4-6"
        );
        assert_eq!(
            resolve_model_name("opus", Backend::Copilot),
            "claude-opus-4.6"
        );
        assert_eq!(
            resolve_model_name("custom-model", Backend::AzureFoundry),
            "custom-model"
        );
    }

    #[test]
    fn anthropic_environment_model_does_not_leak_to_other_backends() {
        assert_eq!(
            resolve_model_with_env(
                Backend::Copilot,
                None,
                None,
                Some("anthropic-only"),
                RuntimeMode::Print,
            ),
            DEFAULT_COPILOT_MODEL
        );
        assert_eq!(
            resolve_model_with_env(
                Backend::AzureFoundry,
                None,
                None,
                Some("anthropic-only"),
                RuntimeMode::Interactive,
            ),
            DEFAULT_COPILOT_MODEL
        );
    }

    #[test]
    fn fallback_is_only_eligible_without_an_explicit_provider() {
        assert!(copilot_fallback_eligible(None));
        assert!(!copilot_fallback_eligible(Some(Backend::Anthropic)));
        assert!(!copilot_fallback_eligible(Some(Backend::Copilot)));
        assert!(!copilot_fallback_eligible(Some(Backend::AzureFoundry)));
    }

    #[tokio::test]
    #[serial]
    async fn anthropic_resolver_applies_settings_endpoint_over_environment() {
        let _env = SavedEnv::set(&[
            ("ANTHROPIC_AUTH_TOKEN", "synthetic-resolver-token"),
            (
                "ANTHROPIC_BASE_URL",
                "https://environment.synthetic.invalid/",
            ),
            ("ANTHROPIC_MODEL", "environment-model"),
        ]);

        let resolved = resolve_anthropic_config(
            None,
            Some("settings-model"),
            Some(" https://settings.synthetic.invalid/gateway/ "),
            RuntimeMode::Print,
        )
        .await
        .unwrap();

        assert_eq!(
            resolved.config.api_url,
            "https://settings.synthetic.invalid/gateway"
        );
        assert_eq!(resolved.model, "settings-model");
        assert_eq!(resolved.config.backend, Backend::Anthropic);
    }
}
