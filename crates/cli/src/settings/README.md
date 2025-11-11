# Settings and Configuration System

A comprehensive 5-tier configuration hierarchy system for Claude Code with proper precedence, environment overrides, validation, and persistence.

## Overview

The settings system implements a hierarchical configuration model that allows settings to be defined at multiple levels, from system-wide defaults to command-line overrides. Each layer can inherit from and override lower layers.

## Hierarchy Tiers

Configuration is loaded in the following precedence order (lowest to highest):

1. **Default** - Built-in defaults in code
2. **User Global** - User's personal configuration (`~/.claude/config` or `~/.config/claude/config`)
3. **Project Shared** - Project-wide configuration (`.claude/config` in repo root)
4. **Project Local** - Local project overrides (`.claude/config.local` in repo root)
5. **Command Line** - Runtime CLI flags and environment variables
6. **Enterprise Managed** - System administrator enforced settings (`/etc/claude/config`)

Higher tiers override lower tiers for all settings except:
- **Permissions**: Accumulate across tiers (higher tier can add or override specific tools)
- **Environment Variables**: Accumulate and override
- **Bypass Disable Flag**: "Sticky" - once set to true, remains true

## Settings Types

### Core Settings

```rust
pub struct Settings {
    pub model: Option<String>,                           // LLM model to use
    pub api_url: Option<String>,                         // API endpoint
    pub timeout_secs: Option<u64>,                       // Operation timeout (1-3600 secs)
    pub cleanup_period_days: u32,                        // Temp file cleanup (1-365 days)
    pub permissions: HashMap<String, ToolPermission>,    // Tool access control
    pub env_vars: HashMap<String, String>,               // Environment variables
    pub disable_bypass_permissions: bool,                // Prevent permission bypass
    pub enabled_plugins: HashMap<String, bool>,          // Plugin settings
}
```

### Permission Modes

```rust
pub enum PermissionMode {
    Allow,  // Always allow access
    Ask,    // Ask user for permission
    Deny,   // Always deny access
}

pub struct ToolPermission {
    pub mode: PermissionMode,
    pub patterns: Vec<String>,  // Prefix patterns for command/path filtering
}
```

## Usage Examples

### Basic Configuration Loading

```rust
use crate::settings::{SettingsLoader, SettingsHierarchy};

// Create loader
let mut loader = SettingsLoader::new();
loader.set_project_root("/path/to/project".into());

// Load all configuration layers
let hierarchy = loader.load_hierarchy()?;

// Get effective configuration
let merged = hierarchy.merge();

println!("Using model: {:?}", merged.model);
println!("Timeout: {:?} seconds", merged.timeout_secs);
```

### Building Settings Programmatically

```rust
use crate::settings::{Settings, ToolPermission, PermissionMode};

let bash_perm = ToolPermission {
    mode: PermissionMode::Allow,
    patterns: vec!["ls".to_string(), "pwd".to_string()],
};

let settings = Settings::new()
    .with_model("claude-3-opus".to_string())
    .with_timeout(120)
    .with_api_url("https://api.anthropic.com".to_string())
    .with_permission("bash".to_string(), bash_perm)
    .with_env_var("DEBUG".to_string(), "true".to_string());

assert!(settings.validate().is_ok());
```

### Working with Hierarchy

```rust
use crate::settings::{SettingsHierarchy, SettingsLayer, Settings};

let mut hierarchy = SettingsHierarchy::new();

// Add user settings
hierarchy.add_layer(
    SettingsLayer::UserGlobal,
    Settings::new().with_model("claude-2".to_string()),
);

// Add project overrides
hierarchy.add_layer(
    SettingsLayer::ProjectLocal,
    Settings::new().with_timeout(90),
);

// Get effective configuration
let merged = hierarchy.merge();
assert_eq!(merged.model, Some("claude-2".to_string())); // From UserGlobal
assert_eq!(merged.timeout_secs, Some(90));             // From ProjectLocal
```

## Environment Variables

Configuration can be overridden using environment variables with the `CLAUDE_` prefix:

```bash
# Model selection
export CLAUDE_MODEL=claude-3-opus

# API configuration
export CLAUDE_API_URL=https://api.example.com

# Timeout (in seconds)
export CLAUDE_TIMEOUT_SECS=180

# Cleanup period (in days)
export CLAUDE_CLEANUP_PERIOD_DAYS=45

# Disable permission bypass
export CLAUDE_DISABLE_BYPASS_PERMISSIONS=true
```

Variable names are converted to lowercase with underscores (e.g., `CLAUDE_API_URL` -> `api_url`).

## Configuration Files

### User Global Configuration

Location: `~/.config/claude/config` or `~/.claude/config`

```toml
[settings]
model = "claude-3-opus"
api_url = "https://api.anthropic.com"
timeout_secs = 120
cleanup_period_days = 30

[permissions.bash]
mode = "allow"
patterns = ["ls", "pwd", "cd"]

[permissions.edit]
mode = "ask"
patterns = []
```

### Project Configuration

Shared config (`.claude/config`):
```toml
[settings]
model = "claude-3"
cleanup_period_days = 7
```

Local overrides (`.claude/config.local`):
```toml
[settings]
timeout_secs = 60

[env_vars]
PROJECT_ID = "proj-123"
DEBUG = "true"
```

### Enterprise Configuration

Location: `/etc/claude/config` (Unix) or `C:\ProgramData\Claude\config` (Windows)

```toml
[settings]
model = "enterprise-model"
disable_bypass_permissions = true

[permissions.bash]
mode = "deny"
patterns = ["rm", "dd", "mkfs"]
```

## Validation

Settings validation ensures:

- **Timeout**: Must be between 1 and 3600 seconds
- **Cleanup Period**: Must be between 1 and 365 days
- **API URL**: Must start with `http://` or `https://`
- **Model Name**: Must not be empty (max 256 characters)
- **Environment Variables**: Must have valid identifier format

```rust
let settings = Settings::new()
    .with_timeout(0)  // Invalid!
    .with_cleanup_period(366);  // Invalid!

match settings.validate() {
    Ok(()) => println!("Valid configuration"),
    Err(e) => eprintln!("Configuration error: {}", e),
}

// Or validate multiple settings at once
use crate::settings::validation::validate_all_settings;

match validate_all_settings(&settings) {
    Ok(()) => println!("All valid"),
    Err(errors) => {
        for error in errors {
            eprintln!("Error: {}", error);
        }
    }
}
```

## Merging Rules

When multiple layers are present, the merge process follows these rules:

### Simple Values (Model, Timeout, API URL)
- Highest priority layer wins
- If not set in higher layer, inherited from lower

### Collections (Permissions, Environment Variables, Plugins)
- Accumulated across all layers
- Higher priority overrides duplicate keys

### Special Cases
- **Bypass Disable Flag**: "Sticky" flag - once true, remains true
- **Cleanup Period**: Uses layer's value if different from default (30 days)

Example:

```
UserGlobal:
  - model: "claude-1"
  - timeout: 60
  - env: {API_KEY: "key1"}

ProjectLocal:
  - timeout: 90
  - env: {DEBUG: "true"}

Merged Result:
  - model: "claude-1"      (from UserGlobal)
  - timeout: 90            (overridden by ProjectLocal)
  - env: {
      API_KEY: "key1",     (from UserGlobal)
      DEBUG: "true"        (from ProjectLocal)
    }
```

## API Reference

### SettingsHierarchy

```rust
pub struct SettingsHierarchy {
    // Add/remove layers
    pub fn add_layer(&mut self, layer: SettingsLayer, settings: Settings);
    pub fn remove_layer(&mut self, layer: SettingsLayer) -> Option<Settings>;

    // Query layers
    pub fn get_layer(&self, layer: SettingsLayer) -> Option<&Settings>;
    pub fn has_layer(&self, layer: SettingsLayer) -> bool;
    pub fn active_layers(&self) -> Vec<SettingsLayer>;

    // Merge into effective configuration
    pub fn merge(&self) -> Settings;

    // Utilities
    pub fn clear(&mut self);
    pub fn layer_count(&self) -> usize;
    pub fn is_empty(&self) -> bool;
    pub fn summary(&self) -> String;
}
```

### SettingsLoader

```rust
pub struct SettingsLoader {
    pub fn new() -> Self;
    pub fn with_project_root(root: PathBuf) -> Self;
    pub fn set_project_root(&mut self, root: PathBuf);

    // Load configuration
    pub fn load_hierarchy(&self) -> Result<SettingsHierarchy, String>;
    pub fn load_env_overrides(&self) -> HashMap<String, String>;

    // Utilities
    pub fn get_user_config_dir() -> Result<PathBuf, String>;
    pub fn get_user_config_path() -> Result<PathBuf, String>;
}
```

### Validation Functions

```rust
pub fn validate_url(url: &str) -> Result<(), String>;
pub fn validate_timeout(secs: u64) -> Result<(), String>;
pub fn validate_cleanup_period(days: u32) -> Result<(), String>;
pub fn validate_path(path: &str) -> Result<(), String>;
pub fn validate_model_name(model: &str) -> Result<(), String>;
pub fn validate_env_var_key(key: &str) -> Result<(), String>;
pub fn validate_all_settings(settings: &Settings) -> Result<(), Vec<String>>;
```

## Testing

The system includes 56 comprehensive tests organized by category:

- **Unit Tests (45 tests)**
  - Configuration loading (10 tests)
  - Validation (13 tests)
  - Layer precedence (7 tests)
  - Edge cases (11 tests)
  - Error handling (4 tests)

- **Integration Tests (13 tests)**
  - Multi-layer merging
  - Permission accumulation
  - Environment variable override
  - Plugin settings merge

- **E2E Tests (6 tests)**
  - Enterprise lockdown scenarios
  - User-to-project settings flow
  - Command-line override cascades
  - Full workflow scenarios

Run tests with:
```bash
cargo test --test settings_tests
```

## Architecture

### Module Structure

- **types.rs** - Core data structures (Settings, PermissionMode, SettingsLayer)
- **hierarchy.rs** - Multi-layer configuration management
- **validation.rs** - Setting validation logic
- **loader.rs** - Configuration loading from files and environment
- **mod.rs** - Public API and integration tests

### Design Principles

1. **Immutable Configuration**: Settings are built-then-frozen
2. **Layer Isolation**: Each layer is independent
3. **Explicit Precedence**: Layer priority is well-defined
4. **Validation Before Use**: All settings must pass validation
5. **Graceful Degradation**: Missing configuration files don't break loading

## Future Enhancements

- [ ] TOML/JSON file parsing for configuration files
- [ ] Configuration reloading without restart
- [ ] Configuration validation schemas
- [ ] Settings export/import functionality
- [ ] Configuration audit logging
- [ ] Dynamic settings updates via API

## License

Educational project demonstrating Rust patterns for configuration management.
