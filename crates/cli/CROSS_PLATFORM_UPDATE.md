# Cross-Platform Update Mechanism

## Overview

The RustyClawd update mechanism supports automatic binary updates across Linux, macOS, and Windows platforms. It includes platform-specific handling for security features, file locking, and atomic replacement operations.

## Supported Platforms

### Linux
- **Architectures**: x86_64, aarch64
- **Target Triples**:
  - `x86_64-unknown-linux-gnu`
  - `aarch64-unknown-linux-gnu`
- **Binary Extension**: None
- **Atomic Replacement**: Uses POSIX `rename()` for atomic operations

### macOS
- **Architectures**: x86_64 (Intel), aarch64 (Apple Silicon)
- **Target Triples**:
  - `x86_64-apple-darwin`
  - `aarch64-apple-darwin`
- **Binary Extension**: None
- **Atomic Replacement**: Uses POSIX `rename()` for atomic operations
- **Special Handling**:
  - Automatic removal of Gatekeeper quarantine attributes
  - Uses `xattr -d com.apple.quarantine` to clear quarantine flags
  - Handles downloaded binaries that might trigger security warnings

### Windows
- **Architectures**: x86_64, aarch64
- **Target Triples**:
  - `x86_64-pc-windows-msvc`
  - `aarch64-pc-windows-msvc`
- **Binary Extension**: `.exe` (required)
- **Atomic Replacement**: Two-stage move operation
- **Special Handling**:
  - Locked file detection before replacement
  - Prevents replacement if binary is currently in use
  - Automatic rollback on failure

## Platform Detection

The update system automatically detects the current platform using `PlatformInfo`:

```rust
use rustyclawd::update::PlatformInfo;

let platform = PlatformInfo::current();
println!("OS: {}, Arch: {}", platform.os, platform.arch);
println!("Target: {}", platform.target_triple);
println!("Binary Extension: {:?}", platform.binary_extension);
```

## Binary Asset Selection

The GitHub client automatically selects the correct binary for the current platform:

```rust
use rustyclawd::update::{GitHubClient, Version};

let client = GitHubClient::new("rysweet", "RustyClawd");
let current_version = Version::current();

let update_info = client.get_update_info(&current_version).await?;
if let Some(info) = update_info {
    // Automatically finds the right binary for the current platform
    if let Some(asset_url) = info.get_asset_for_platform() {
        println!("Download URL: {}", asset_url);
    }
}
```

## Platform-Specific Features

### macOS: Gatekeeper Quarantine Removal

When a binary is downloaded on macOS, the system adds a quarantine attribute that triggers security warnings. The update mechanism automatically removes this:

```rust
// Automatically called during installation
Self::remove_quarantine_attribute(&binary_path)?;
```

This uses the `xattr` command to remove `com.apple.quarantine` attributes. If the command fails (e.g., xattr not available), the operation continues with a warning.

### Windows: Locked File Detection

Windows prevents replacing binaries that are currently executing. The update system checks for this before attempting replacement:

```rust
// Automatically called during atomic replacement
if Self::is_binary_locked(current_binary) {
    return Err(UpdateError::IoError(
        "Cannot replace binary: file is currently in use. Please close the application and try again.".to_string()
    ));
}
```

**Important**: On Windows, updates should be applied when the application is not running, or using a separate updater process that can replace the main binary.

### Windows: .exe Extension Handling

The asset selection automatically filters for `.exe` files on Windows:

```rust
// On Windows, only matches assets with .exe extension
let asset = info.get_asset_for_platform();
// Returns: "rusty-x86_64-pc-windows-msvc.exe"
```

## Atomic Replacement Strategies

### Unix-like Systems (Linux, macOS)

Uses POSIX `rename()` which is atomic on the same filesystem:

1. Copy new binary to temporary location in same directory
2. Set executable permissions (`chmod 755`)
3. (macOS only) Remove quarantine attribute
4. Atomic rename from temp location to target location

### Windows

Uses two-stage move operation:

1. Check if current binary is locked
2. Move current binary to temporary backup location
3. Move new binary to target location
4. On success: delete temporary backup
5. On failure: restore from temporary backup

### Fallback Platforms

For unsupported platforms, falls back to simple copy operation (non-atomic).

## Usage Examples

### Basic Update Check

```rust
use rustyclawd::update::{GitHubClient, Version};

let client = GitHubClient::new("rysweet", "RustyClawd");
let current = Version::current();

if client.check_update(&current).await? {
    println!("Update available!");
}
```

### Full Update Installation

```rust
use rustyclawd::update::{BinaryInstaller, InstallerConfig};
use std::path::Path;

let installer = BinaryInstaller::new()?;
let result = installer.install_update(
    Path::new("/tmp/new_binary"),
    Path::new("/usr/local/bin/rusty")
)?;

if result.success {
    println!("Update installed successfully!");
    if let Some(backup) = result.backup_path {
        println!("Backup created at: {:?}", backup);
    }
}
```

### Platform-Specific Binary Naming

```rust
use rustyclawd::update::PlatformInfo;

let platform = PlatformInfo::current();
let binary_name = if let Some(ext) = &platform.binary_extension {
    format!("rusty{}", ext)  // Windows: "rusty.exe"
} else {
    "rusty".to_string()      // Unix: "rusty"
};
```

## Testing

The update mechanism includes comprehensive cross-platform tests:

```bash
# Run all update tests
cargo test --package rustyclawd-cli --lib update

# Run platform-specific tests
cargo test --package rustyclawd-cli --lib update::github_client::tests::test_platform_info
cargo test --package rustyclawd-cli --lib update::integration_tests::test_cross_platform
```

## Release Workflow

When creating GitHub releases, ensure you provide binaries for all supported platforms:

```
# Linux
rusty-x86_64-unknown-linux-gnu
rusty-aarch64-unknown-linux-gnu

# macOS
rusty-x86_64-apple-darwin
rusty-aarch64-apple-darwin

# Windows
rusty-x86_64-pc-windows-msvc.exe
rusty-aarch64-pc-windows-msvc.exe
```

The update system will automatically select the appropriate binary based on the platform.

## Security Considerations

### macOS
- Quarantine attributes are removed automatically
- Users may still need to approve the binary in System Preferences on first run
- Consider code signing for production releases

### Windows
- Updates cannot replace running binaries
- Consider using Windows Installer or a separate updater process
- Administrator privileges may be required for system-wide installations
- Consider code signing for production releases

### Linux
- Standard executable permissions are set (`755`)
- Updates to system directories require appropriate permissions
- Consider using package managers for system-wide installations

## Troubleshooting

### macOS: "Binary is damaged and can't be opened"
- The quarantine attribute removal should prevent this
- If it persists, manually run: `xattr -cr /path/to/rusty`

### Windows: "File is in use"
- Close all instances of the application
- Use Task Manager to ensure no processes are running
- Consider using a separate updater that runs after the main application closes

### Linux: Permission Denied
- Ensure you have write permissions to the installation directory
- Use `sudo` if updating system-wide installations
- Check file ownership and permissions

## Future Enhancements

Planned improvements for cross-platform support:

1. **Windows Service Mode**: Support updates for running services
2. **macOS Notarization**: Integrate with Apple notarization workflow
3. **Linux AppImage**: Support for self-updating AppImages
4. **Sandboxed Updates**: Support for updates in sandboxed environments
5. **Delta Updates**: Binary diffing for smaller update downloads

## API Reference

### PlatformInfo

```rust
pub struct PlatformInfo {
    pub target_triple: String,
    pub os: String,
    pub arch: String,
    pub binary_extension: Option<String>,
}
```

Methods:
- `current()` - Get current platform information
- `parse_target(&str)` - Parse a target triple

### Binary Installer

Platform-specific methods (automatically called):
- `remove_quarantine_attribute(&Path)` - macOS only
- `is_binary_locked(&Path)` - Windows only

### Update Info

```rust
pub fn get_asset_for_platform(&self) -> Option<String>
```

Automatically selects the correct binary asset for the current platform, including:
- Matching target triple
- Verifying `.exe` extension on Windows
- Returning download URL

## Contributing

When adding support for new platforms:

1. Update `get_platform_target()` in `github_client.rs`
2. Add platform-specific handling in `installer.rs`
3. Update `PlatformInfo::parse_target()` to recognize the platform
4. Add comprehensive tests in `mod.rs`
5. Update this documentation

## License

Same as RustyClawd main project.
