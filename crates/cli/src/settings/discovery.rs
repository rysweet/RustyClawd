/// File discovery logic for configuration files.
///
/// Finds config files at standard locations:
/// - User global: ~/.config/claude/config (or XDG_CONFIG_HOME)
/// - Project shared: <project>/.claude/config
/// - Project local: <project>/.claude/config.local
/// - Enterprise: /etc/claude/config (Unix) or C:\ProgramData\Claude\config (Windows)
use std::env;
use std::path::{Path, PathBuf};

use crate::settings::parser::parse_settings_from_file;
use crate::settings::types::Settings;

/// Get the user config directory
pub fn get_user_config_dir() -> Result<PathBuf, String> {
    if let Ok(config_home) = env::var("XDG_CONFIG_HOME") {
        Ok(PathBuf::from(config_home).join("claude"))
    } else if let Ok(home) = env::var("HOME") {
        Ok(PathBuf::from(home).join(".config").join("claude"))
    } else {
        #[cfg(windows)]
        if let Ok(appdata) = env::var("APPDATA") {
            Ok(PathBuf::from(appdata).join("Claude"))
        } else {
            Err("Could not determine config directory".to_string())
        }

        #[cfg(not(windows))]
        Err("Could not determine config directory".to_string())
    }
}

/// Get the user config file path
pub fn get_user_config_path() -> Result<PathBuf, String> {
    let config_dir = get_user_config_dir()?;
    Ok(config_dir.join("config"))
}

/// Load user global settings from ~/.claude/config
pub fn load_user_global_settings() -> Result<Settings, String> {
    let config_path = get_user_config_path()?;

    if !config_path.exists() {
        return Err(format!("User config not found: {:?}", config_path));
    }

    parse_settings_from_file(&config_path)
}

/// Load project shared settings from .claude/config
pub fn load_project_shared_settings(project_root: &Path) -> Result<Settings, String> {
    let config_path = project_root.join(".claude").join("config");

    if !config_path.exists() {
        return Err(format!(
            "Project shared config not found: {:?}",
            config_path
        ));
    }

    parse_settings_from_file(&config_path)
}

/// Load project local settings from .claude/config.local
pub fn load_project_local_settings(project_root: &Path) -> Result<Settings, String> {
    let config_path = project_root.join(".claude").join("config.local");

    if !config_path.exists() {
        return Err(format!("Project local config not found: {:?}", config_path));
    }

    parse_settings_from_file(&config_path)
}

/// Load enterprise settings from /etc/claude/config
pub fn load_enterprise_settings() -> Result<Settings, String> {
    #[cfg(unix)]
    let config_path = Path::new("/etc/claude/config");

    #[cfg(windows)]
    let config_path = Path::new("C:\\ProgramData\\Claude\\config");

    if !config_path.exists() {
        return Err(format!("Enterprise config not found: {:?}", config_path));
    }

    parse_settings_from_file(config_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_user_config_dir() {
        let result = get_user_config_dir();
        assert!(result.is_ok());

        let path = result.unwrap();
        assert!(
            path.to_string_lossy().contains("claude") || path.to_string_lossy().contains("Claude")
        );
    }
}
