# Settings System API Contract

## Public API

The settings module exports the following public interface:

```rust
// Core types
pub use types::{Settings, SettingsLayer, PermissionMode, ToolPermission};

// Hierarchy management
pub use hierarchy::SettingsHierarchy;

// Configuration validation
pub use validation::{
    validate_all_settings, validate_cleanup_period, validate_env_var_key,
    validate_model_name, validate_path, validate_timeout, validate_url,
};

// Configuration loading
pub use loader::SettingsLoader;
```

## Detailed API

### Settings

Core configuration container.

#### Construction

```rust
// Default constructor
let settings = Settings::new();

// Builder pattern
let settings = Settings::new()
    .with_model("claude-3".to_string())
    .with_api_url("https://api.anthropic.com".to_string())
    .with_timeout(120)
    .with_cleanup_period(30)
    .with_permission("bash".to_string(), bash_perm)
    .with_env_var("DEBUG".to_string(), "true".to_string())
    .set_plugin("plugin-name".to_string(), true)
    .disable_bypass();
```

#### Public Methods

```rust
impl Settings {
    // Construction
    pub fn new() -> Self;
    pub fn default() -> Self;

    // Builder methods - return Self for chaining
    pub fn with_model(self, model: String) -> Self;
    pub fn with_api_url(self, url: String) -> Self;
    pub fn with_timeout(self, secs: u64) -> Self;
    pub fn with_cleanup_period(self, days: u32) -> Self;
    pub fn with_permission(self, tool: String, permission: ToolPermission) -> Self;
    pub fn with_env_var(self, key: String, value: String) -> Self;
    pub fn set_plugin(self, plugin: String, enabled: bool) -> Self;
    pub fn disable_bypass(self) -> Self;

    // Validation
    pub fn validate(&self) -> Result<(), String>;

    // Introspection
    pub fn is_empty(&self) -> bool;
}
```

#### Public Fields

```rust
pub struct Settings {
    pub model: Option<String>,                          // LLM model
    pub api_url: Option<String>,                        // API URL
    pub timeout_secs: Option<u64>,                      // Timeout in seconds
    pub cleanup_period_days: u32,                       // Cleanup period in days
    pub permissions: HashMap<String, ToolPermission>,   // Tool permissions
    pub env_vars: HashMap<String, String>,              // Environment variables
    pub disable_bypass_permissions: bool,               // Bypass prevention flag
    pub enabled_plugins: HashMap<String, bool>,         // Plugin states
}
```

#### Traits

- `Debug`: Full debug output
- `Clone`: Deep copy
- `PartialEq + Eq`: Equality comparison
- `Default`: Default values

---

### PermissionMode

Tool permission control.

```rust
pub enum PermissionMode {
    Allow,  // Always allow
    Ask,    // Ask user
    Deny,   // Always deny
}

impl PermissionMode {
    pub fn from_str(s: &str) -> Option<Self>;
    pub fn as_str(&self) -> &str;
}
```

---

### ToolPermission

Permissions for a specific tool.

```rust
pub struct ToolPermission {
    pub mode: PermissionMode,
    pub patterns: Vec<String>,  // Prefix patterns
}

impl ToolPermission {
    pub fn new(mode: PermissionMode, patterns: Vec<String>) -> Self;
}
```

#### Traits

- `Debug`: Full debug output
- `Clone`: Deep copy
- `PartialEq + Eq`: Equality comparison

---

### SettingsLayer

Configuration layer identifier.

```rust
pub enum SettingsLayer {
    Default = 0,
    UserGlobal = 1,
    ProjectShared = 2,
    ProjectLocal = 3,
    CommandLine = 4,
    EnterpriseManaged = 5,
}

impl SettingsLayer {
    pub fn priority(&self) -> u32;        // Get numeric priority
    pub fn name(&self) -> &str;           // Get layer name
}
```

#### Traits

- `Debug`: Full debug output
- `Clone + Copy`: Lightweight copying
- `PartialEq + Eq`: Equality
- `PartialOrd + Ord`: Priority ordering
- `Hash`: Hashmap key support

---

### SettingsHierarchy

Multi-layer configuration management.

```rust
pub struct SettingsHierarchy {
    layers: HashMap<SettingsLayer, Settings>,
}

impl SettingsHierarchy {
    // Construction
    pub fn new() -> Self;

    // Layer management
    pub fn add_layer(&mut self, layer: SettingsLayer, settings: Settings);
    pub fn remove_layer(&mut self, layer: SettingsLayer) -> Option<Settings>;
    pub fn get_layer(&self, layer: SettingsLayer) -> Option<&Settings>;
    pub fn get_layer_mut(&mut self, layer: SettingsLayer) -> Option<&mut Settings>;
    pub fn has_layer(&self, layer: SettingsLayer) -> bool;

    // Querying
    pub fn active_layers(&self) -> Vec<SettingsLayer>;
    pub fn layer_count(&self) -> usize;
    pub fn is_empty(&self) -> bool;

    // Operations
    pub fn merge(&self) -> Settings;              // Merge all layers
    pub fn clear(&mut self);                      // Clear all layers
    pub fn summary(&self) -> String;              // Debug summary
}
```

#### Merge Algorithm

```
1. Start with Settings::default()
2. For each layer in priority order (low to high):
   - If layer has model: override
   - If layer has api_url: override
   - If layer has timeout_secs: override
   - If layer has cleanup_period_days (≠ 30): override
   - For each permission: add/override
   - For each env_var: add/override
   - If layer has disable_bypass_permissions: set to true
   - For each plugin: add/override
3. Return merged Settings
```

#### Traits

- `Debug`: Full debug output
- `Clone`: Deep copy
- `Default`: Empty hierarchy

---

### SettingsLoader

Configuration loading from various sources.

```rust
pub struct SettingsLoader {
    project_root: Option<PathBuf>,
}

impl SettingsLoader {
    // Construction
    pub fn new() -> Self;
    pub fn with_project_root(root: PathBuf) -> Self;

    // Configuration
    pub fn set_project_root(&mut self, root: PathBuf);

    // Loading
    pub fn load_hierarchy(&self) -> Result<SettingsHierarchy, String>;
    pub fn load_env_overrides(&self) -> HashMap<String, String>;

    // Utilities
    pub fn parse_env_overrides(overrides: &HashMap<String, String>) -> Settings;
    pub fn get_user_config_dir() -> Result<PathBuf, String>;
    pub fn get_user_config_path() -> Result<PathBuf, String>;
}
```

#### Load Hierarchy Process

1. Create new hierarchy with Default layer
2. Load user global settings (if exists)
3. Load project shared settings (if project_root set)
4. Load project local settings (if project_root set)
5. Load environment variable overrides
6. Load enterprise settings (if exists)
7. Return complete hierarchy

#### Environment Variable Parsing

```
CLAUDE_MODEL -> model
CLAUDE_API_URL -> api_url
CLAUDE_TIMEOUT_SECS | CLAUDE_TIMEOUT -> timeout_secs
CLAUDE_CLEANUP_PERIOD_DAYS | CLAUDE_CLEANUP_PERIOD -> cleanup_period_days
CLAUDE_DISABLE_BYPASS_PERMISSIONS | CLAUDE_DISABLE_BYPASS -> disable_bypass_permissions
```

#### Traits

- `Debug`: Full debug output
- `Default`: Uses `new()`

---

## Validation Functions

### validate_url

```rust
pub fn validate_url(url: &str) -> Result<(), String>
```

- Must start with `http://` or `https://`
- Must be at least 8 characters
- Returns error message if invalid

### validate_timeout

```rust
pub fn validate_timeout(secs: u64) -> Result<(), String>
```

- Must be > 0
- Must be < 3600
- Returns error message if invalid

### validate_cleanup_period

```rust
pub fn validate_cleanup_period(days: u32) -> Result<(), String>
```

- Must be > 0
- Must be < 366
- Returns error message if invalid

### validate_path

```rust
pub fn validate_path(path: &str) -> Result<(), String>
```

- Path must exist
- Must be file or directory
- Returns error message if invalid

### validate_model_name

```rust
pub fn validate_model_name(model: &str) -> Result<(), String>
```

- Must not be empty
- Max 256 characters
- Returns error message if invalid

### validate_env_var_key

```rust
pub fn validate_env_var_key(key: &str) -> Result<(), String>
```

- Must not be empty
- Must contain only alphanumeric and underscore
- Cannot start with number
- Returns error message if invalid

### validate_all_settings

```rust
pub fn validate_all_settings(settings: &Settings) -> Result<(), Vec<String>>
```

- Validates all settings in a Settings struct
- Returns all validation errors (not just first)
- Useful for comprehensive validation

---

## Error Handling

All functions that can fail return `Result<T, E>`:

```rust
// Can fail
match settings.validate() {
    Ok(()) => { /* valid */ },
    Err(msg) => { /* handle error */ },
}

// Can fail
match loader.load_hierarchy() {
    Ok(hierarchy) => { /* use it */ },
    Err(msg) => { /* handle error */ },
}

// Can collect multiple errors
match validate_all_settings(&settings) {
    Ok(()) => { /* all valid */ },
    Err(errors) => {
        for error in errors {
            eprintln!("Validation error: {}", error);
        }
    }
}
```

---

## Thread Safety

- **Settings**: Safe to share across threads (immutable after construction)
- **SettingsHierarchy**: Requires `&mut` for modifications, readonly for merging
- **SettingsLoader**: Safe to reuse for multiple loads

---

## Serialization

Current implementation does not include serialization. To add:

```rust
// Add to Cargo.toml:
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
toml = "0.8"

// Add derives:
#[derive(Serialize, Deserialize)]
pub struct Settings { ... }
```

---

## Performance Characteristics

- **Settings::new()**: O(1)
- **Settings::validate()**: O(n) where n = number of env vars
- **SettingsHierarchy::add_layer()**: O(1)
- **SettingsHierarchy::merge()**: O(m*n) where m = layers, n = avg settings per layer
- **SettingsLoader::load_hierarchy()**: O(file I/O)

---

## Examples

### Example 1: Basic Usage

```rust
let settings = Settings::new()
    .with_model("claude-3".to_string())
    .with_timeout(60);

settings.validate()?;
println!("Model: {:?}", settings.model);
```

### Example 2: Hierarchy with Overrides

```rust
let mut hierarchy = SettingsHierarchy::new();

hierarchy.add_layer(
    SettingsLayer::UserGlobal,
    Settings::new().with_model("claude-2".to_string()),
);

hierarchy.add_layer(
    SettingsLayer::ProjectLocal,
    Settings::new().with_timeout(90),
);

let config = hierarchy.merge();
assert_eq!(config.model, Some("claude-2".to_string()));
assert_eq!(config.timeout_secs, Some(90));
```

### Example 3: Loading from Files

```rust
let loader = SettingsLoader::with_project_root(
    std::env::current_dir()?.into()
);

let hierarchy = loader.load_hierarchy()?;
let config = hierarchy.merge();
config.validate()?;
```

### Example 4: Environment Overrides

```rust
std::env::set_var("CLAUDE_MODEL", "claude-3-opus");
std::env::set_var("CLAUDE_TIMEOUT_SECS", "180");

let loader = SettingsLoader::new();
let overrides = loader.load_env_overrides();
let settings = SettingsLoader::parse_env_overrides(&overrides);

assert_eq!(settings.model, Some("claude-3-opus".to_string()));
assert_eq!(settings.timeout_secs, Some(180));
```

### Example 5: Permissions

```rust
use rustyclawd::settings::{PermissionMode, ToolPermission};

let bash_perm = ToolPermission {
    mode: PermissionMode::Allow,
    patterns: vec!["ls".to_string(), "pwd".to_string()],
};

let settings = Settings::new()
    .with_permission("bash".to_string(), bash_perm);

assert!(settings.permissions.contains_key("bash"));
```

---

## Documentation Links

- Full documentation: See `README.md`
- Implementation details: See `types.rs`, `hierarchy.rs`, `validation.rs`, `loader.rs`
- Tests: Run `cargo test --test settings_tests`
