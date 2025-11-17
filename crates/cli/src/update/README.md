# Update Mechanism - Phase 1

Version detection and GitHub releases API client for RustyClawd.

## Overview

This module handles:
- **Version detection**: Getting the current binary version from compile-time metadata
- **Version parsing and comparison**: Semantic versioning (major.minor.patch)
- **GitHub Releases API**: Querying the latest release from the repository
- **Update configuration**: Managing auto-check settings and intervals

## Architecture

### Module Components

#### `version.rs` - Version Detection & Comparison
- `Version` struct: Represents semantic versions (major.minor.patch)
- `Version::current()`: Gets the current binary version from `env!("CARGO_PKG_VERSION")`
- `Version::parse()`: Parses version strings (handles "v" prefix and pre-release tags)
- Version comparison: Standard trait implementations (PartialOrd, PartialEq)

Key Features:
- Handles pre-release versions (e.g., "1.2.3-alpha")
- Handles build metadata (e.g., "1.2.3+build.1")
- Flexible parsing that strips pre-release and metadata for core version comparison

#### `github_client.rs` - API Client
- `GitHubClient`: Async HTTP client for GitHub Releases API
- `Release`: Represents a GitHub release with assets
- `ReleaseAsset`: Individual binary/asset information
- `UpdateInfo`: Summary of available update information

API Endpoints:
- `GET /repos/rysweet/RustyClawd/releases/latest` - Fetch latest release

Features:
- Automatic user-agent header
- 10-second timeout
- Cross-platform asset matching (Linux x86_64/aarch64, macOS, Windows)
- Error conversion from reqwest

#### `config.rs` - Update Configuration
- `UpdateConfig`: Manages update check settings
- `auto_check: bool` - Enable/disable automatic checking
- `check_interval_hours: u32` - Hours between checks (1-8760)
- `last_check_timestamp: u64` - Unix timestamp of last check

Features:
- `should_check_now()`: Determines if check should run
- `update_last_check()`: Updates timestamp to current time
- `interval_description()`: Human-readable interval text
- Validation of configuration values
- JSON serialization support for persistence

#### `error.rs` - Error Types
- `UpdateError` enum: All possible error conditions
- Automatic conversions from:
  - `std::io::Error`
  - `serde_json::Error`
  - `reqwest::Error`

Error Variants:
- `VersionParseFailed` - Version string parsing error
- `GitHubApiError` - API request failure
- `GitHubResponseParseFailed` - JSON parsing error
- `NetworkError` - Network communication error
- `Timeout` - Operation exceeded timeout
- `ConfigError` - Invalid configuration
- `AssetNotFound` - No matching binary for platform

## Usage

### Basic Version Checking

```rust
use rustyclawd::update::{Version, GitHubClient};

// Get current version
let current = Version::current();
println!("Current: {}", current.to_string());

// Create GitHub client
let client = GitHubClient::new("rysweet", "RustyClawd");

// Check for updates
if let Ok(Some(update_info)) = client.get_update_info(&current).await {
    println!("{}", update_info.summary());
    println!("Release notes: {}", update_info.release_notes.unwrap_or("None"));

    // Get platform-specific binary URL
    if let Some(url) = update_info.get_asset_for_platform() {
        println!("Download: {}", url);
    }
}
```

### Configuration Management

```rust
use rustyclawd::update::UpdateConfig;

let mut config = UpdateConfig::new();

// Configure checks
config.set_auto_check(true)?;
config.set_check_interval(24)?;

// Check if time has passed
if config.should_check_now() {
    // Perform update check
    config.update_last_check();
}

// Serialize for persistence
let json = serde_json::to_string(&config)?;
```

## Implementation Details

### Version Comparison

Versions are compared using Rust's standard `Ord` trait implementation, which compares:
1. Major version
2. Minor version (if major equal)
3. Patch version (if minor equal)

Pre-release and build metadata are stripped during parsing, so "1.2.3-alpha" == "1.2.3+build".

### GitHub API

- **Base URL**: `https://api.github.com`
- **Endpoint**: `/repos/rysweet/RustyClawd/releases/latest`
- **User-Agent**: `RustyClawd-Update-Client/1.0`
- **Timeout**: 10 seconds

Response format (Release):
```json
{
  "tag_name": "v1.2.3",
  "name": "Version 1.2.3",
  "body": "Release notes...",
  "draft": false,
  "prerelease": false,
  "assets": [
    {
      "name": "rusty-x86_64-unknown-linux-gnu",
      "browser_download_url": "https://...",
      "size": 1024
    }
  ]
}
```

### Configuration Persistence

The `UpdateConfig` is JSON-serializable, allowing it to be:
- Stored in configuration files
- Integrated with the settings system
- Synced across multiple runs

Validation ensures:
- `check_interval_hours` is between 1 and 8760 (1 year)
- If `auto_check` is true, interval must be > 0
- Configuration can be safely saved and restored

## Testing

All modules include comprehensive unit tests:
- 33 total tests
- Version parsing edge cases
- Configuration validation
- Error conversions
- Integration tests

Run tests:
```bash
cargo test -p rustyclawd-cli update:: --lib
```

## Future Enhancements (Phase 2+)

- Download and apply updates automatically
- Binary signature verification
- Rollback capability
- Update UI/notifications
- Windows/macOS specific handling
- Pre-release channel support
- Update scheduling (background daemon)
- Statistics/telemetry

## Integration Points

### Settings System

The `UpdateConfig` is designed to integrate with the existing settings hierarchy:
- Stored at user/project configuration level
- Respects settings layer precedence
- Can be overridden via command line or environment

Example integration:
```rust
// In Settings::with_update_config()
settings.update_config = UpdateConfig::from_settings(settings_layer);
```

### Current Limitations (Phase 1)

- Linux-focused (basic support for x86_64 and aarch64)
- No auto-download/apply (manual download only)
- No signature verification
- No UI/notifications
- Synchronous API checking only (async available)

## Files

- `/home/azureuser/src/RustyClawd/crates/cli/src/update/version.rs` - Version handling
- `/home/azureuser/src/RustyClawd/crates/cli/src/update/github_client.rs` - GitHub API client
- `/home/azureuser/src/RustyClawd/crates/cli/src/update/config.rs` - Configuration
- `/home/azureuser/src/RustyClawd/crates/cli/src/update/error.rs` - Error types
- `/home/azureuser/src/RustyClawd/crates/cli/src/update/mod.rs` - Module foundation

## Acceptance Criteria Status

- [x] Can detect current version from binary
- [x] Can query GitHub for latest release
- [x] Can compare versions (semver)
- [x] Config integrates with settings structure
- [x] All code compiles without errors
- [x] Basic tests pass (33/33)
- [x] No warnings in release build
