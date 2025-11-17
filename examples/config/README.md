# RustyClawd Configuration Examples

This directory contains example TOML configuration files for RustyClawd.

## Files

### minimal.toml
The absolute minimum configuration - just specifies the model to use.

### basic.toml
A basic configuration showing common settings including:
- Model and API configuration
- Timeout and cleanup settings
- Environment variables
- Basic tool permissions

### advanced.toml
A comprehensive configuration demonstrating all available Settings fields:
- Full API configuration
- Security settings (disable_bypass_permissions)
- Multiple environment variables
- Comprehensive tool permissions
- Plugin enable/disable settings

## Configuration File Locations

RustyClawd looks for configuration files in the following locations (in order of priority):

1. **Default** - Built-in defaults
2. **User Global** - `~/.config/claude/config.toml`
3. **Project Shared** - `.claude/config.toml` (checked into git)
4. **Project Local** - `.claude/config.local.toml` (gitignored)
5. **Command Line** - CLI flags and `CLAUDE_*` environment variables
6. **Enterprise Managed** - `/etc/claude/config.toml` (Unix) or `C:\ProgramData\Claude\config.toml` (Windows)

## Format Priority

When no file extension is specified, formats are tried in this order:
1. TOML (`.toml`)
2. YAML (`.yaml`, `.yml`) - Not yet implemented
3. JSON (`.json`)

## Settings Reference

### Top-Level Fields

- `model` (string) - LLM model identifier
- `api_url` (string) - API endpoint URL
- `timeout_secs` (integer) - Operation timeout in seconds (1-3600)
- `cleanup_period_days` (integer) - Temp file cleanup period (1-365)
- `disable_bypass_permissions` (boolean) - Prevent bypassing permission checks

### Tables

#### `[env_vars]`
Environment variables to set when running commands.

```toml
[env_vars]
PROJECT_NAME = "my-project"
RUST_LOG = "info"
```

#### `[permissions.<tool_name>]`
Tool-specific permission settings.

```toml
[permissions.bash]
mode = "ask"  # Options: "allow", "ask", "deny"
patterns = ["ls", "cat", "git"]
```

#### `[enabled_plugins]`
Enable or disable specific plugins.

```toml
[enabled_plugins]
github = true
gitlab = false
```

## Usage Examples

### Create a user global config
```bash
mkdir -p ~/.config/claude
cp minimal.toml ~/.config/claude/config.toml
```

### Create a project config
```bash
mkdir -p .claude
cp basic.toml .claude/config.toml
```

### Override with environment variables
```bash
export CLAUDE_MODEL="claude-3-opus"
export CLAUDE_TIMEOUT_SECS=180
rusty "hello"
```

## Validation

All configuration files are validated on load. Common validation rules:
- `timeout_secs` must be between 1 and 3600
- `cleanup_period_days` must be between 1 and 365
- `api_url` must start with `http://` or `https://`
- Permission modes must be one of: "allow", "ask", "deny"

## Error Messages

TOML parsing errors include line numbers for easy debugging:

```
Invalid TOML at line 5: expected a table key, found a newline
```
