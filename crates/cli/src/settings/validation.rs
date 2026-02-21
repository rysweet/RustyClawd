/// Validation logic for settings and configuration values
use crate::settings::types::Settings;
use std::path::Path;

/// Validate that a string value represents a valid URL
pub fn validate_url(url: &str) -> Result<(), String> {
    if !url.starts_with("https://") && !url.starts_with("http://") {
        return Err("URL must start with http:// or https://".to_string());
    }

    if url.len() < 8 {
        return Err("URL is too short".to_string());
    }

    Ok(())
}

/// Validate that a timeout value is within acceptable bounds
pub fn validate_timeout(secs: u64) -> Result<(), String> {
    if secs == 0 {
        return Err("Timeout must be greater than 0".to_string());
    }

    if secs > 3600 {
        return Err("Timeout must be less than 3600 seconds".to_string());
    }

    Ok(())
}

/// Validate that a cleanup period is within acceptable bounds
pub fn validate_cleanup_period(days: u32) -> Result<(), String> {
    if days == 0 {
        return Err("Cleanup period must be at least 1 day".to_string());
    }

    if days > 365 {
        return Err("Cleanup period must be at most 365 days".to_string());
    }

    Ok(())
}

/// Validate that a path exists and is readable
pub fn validate_path(path: &str) -> Result<(), String> {
    let p = Path::new(path);

    if !p.exists() {
        return Err(format!("Path does not exist: {}", path));
    }

    if !p.is_dir() && !p.is_file() {
        return Err(format!("Path is neither a file nor directory: {}", path));
    }

    Ok(())
}

/// Validate that a model name has a reasonable format
pub fn validate_model_name(model: &str) -> Result<(), String> {
    if model.is_empty() {
        return Err("Model name cannot be empty".to_string());
    }

    if model.len() > 256 {
        return Err("Model name is too long (max 256 characters)".to_string());
    }

    Ok(())
}

/// Validate environment variable key format
pub fn validate_env_var_key(key: &str) -> Result<(), String> {
    if key.is_empty() {
        return Err("Environment variable key cannot be empty".to_string());
    }

    if !key.chars().all(|c| c.is_alphanumeric() || c == '_') {
        return Err("Environment variable key contains invalid characters".to_string());
    }

    if key.chars().next().map(|c| c.is_numeric()).unwrap_or(false) {
        return Err("Environment variable key cannot start with a number".to_string());
    }

    Ok(())
}

/// Comprehensive validation for all settings
pub fn validate_all_settings(settings: &Settings) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();

    // Validate timeout
    if let Some(timeout) = settings.timeout_secs {
        if let Err(e) = validate_timeout(timeout) {
            errors.push(e);
        }
    }

    // Validate cleanup period (if explicitly set)
    if let Some(days) = settings.cleanup_period_days {
        if let Err(e) = validate_cleanup_period(days) {
            errors.push(e);
        }
    }

    // Validate API URL
    if let Some(ref url) = settings.api_url {
        if let Err(e) = validate_url(url) {
            errors.push(e);
        }
    }

    // Validate model name
    if let Some(ref model) = settings.model {
        if let Err(e) = validate_model_name(model) {
            errors.push(e);
        }
    }

    // Validate environment variable keys
    for key in settings.env_vars.keys() {
        if let Err(e) = validate_env_var_key(key) {
            errors.push(format!("Invalid environment variable key '{}': {}", key, e));
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_url_valid() {
        assert!(validate_url("https://api.anthropic.com").is_ok());
        assert!(validate_url("http://localhost:8000").is_ok());
    }

    #[test]
    fn test_validate_url_invalid() {
        assert!(validate_url("api.anthropic.com").is_err());
        assert!(validate_url("ftp://example.com").is_err());
    }

    #[test]
    fn test_validate_timeout() {
        assert!(validate_timeout(1).is_ok());
        assert!(validate_timeout(60).is_ok());
        assert!(validate_timeout(3600).is_ok());
        assert!(validate_timeout(0).is_err());
        assert!(validate_timeout(3601).is_err());
    }

    #[test]
    fn test_validate_cleanup_period() {
        assert!(validate_cleanup_period(1).is_ok());
        assert!(validate_cleanup_period(30).is_ok());
        assert!(validate_cleanup_period(365).is_ok());
        assert!(validate_cleanup_period(0).is_err());
        assert!(validate_cleanup_period(366).is_err());
    }

    #[test]
    fn test_validate_model_name() {
        assert!(validate_model_name("claude-3").is_ok());
        assert!(validate_model_name("gpt-4").is_ok());
        assert!(validate_model_name("").is_err());
    }

    #[test]
    fn test_validate_env_var_key() {
        assert!(validate_env_var_key("API_KEY").is_ok());
        assert!(validate_env_var_key("DEBUG_LEVEL").is_ok());
        assert!(validate_env_var_key("VAR_123").is_ok());
        assert!(validate_env_var_key("").is_err());
        assert!(validate_env_var_key("123_VAR").is_err());
        assert!(validate_env_var_key("KEY-NAME").is_err());
    }

    #[test]
    fn test_validate_all_settings_valid() {
        let settings = Settings::new()
            .with_timeout(120)
            .with_cleanup_period(30)
            .with_model("claude-3".to_string())
            .with_env_var("DEBUG".to_string(), "true".to_string());

        assert!(validate_all_settings(&settings).is_ok());
    }

    #[test]
    fn test_validate_all_settings_invalid() {
        let settings = Settings::new()
            .with_timeout(0) // Invalid
            .with_cleanup_period(366); // Invalid

        let result = validate_all_settings(&settings);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.len() >= 2);
    }
}
