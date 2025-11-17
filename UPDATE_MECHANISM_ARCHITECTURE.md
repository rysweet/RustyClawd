# Update Mechanism Architecture Specification

## Executive Summary

The Update Mechanism is a self-contained subsystem enabling safe, autonomous updates of the RustyClawd CLI. It follows the brick philosophy: modular components with clear contracts, recoverable from specification alone, and platform-agnostic design with Linux as primary implementation.

**Complexity Level**: 7-10 days (Expert)
**Scope**: ~2000-2500 LOC across 8-10 modules
**Key Principle**: Atomic operations with guaranteed rollback capability

---

## 1. Module Structure

```
crates/cli/src/update/
├── mod.rs                 # Module root, exports public API
├── version.rs             # Version detection & comparison
├── github_client.rs       # GitHub Releases API integration
├── downloader.rs          # Binary download with SHA256 verification
├── backup.rs              # Atomic backup & rollback management
├── installer.rs           # Platform-specific atomic replacement
├── scheduler.rs           # Auto-check scheduling (once per day)
├── state.rs               # Update state persistence
├── config.rs              # Configuration integration
└── error.rs               # Update-specific error types
```

### Module Dependencies

```
mod.rs (orchestrator)
├── version.rs (no deps)
├── github_client.rs (reqwest, serde)
├── downloader.rs (reqwest, sha2)
├── backup.rs (fs, paths)
├── installer.rs (fs, platform-specific)
├── scheduler.rs (chrono)
├── state.rs (serde, serde_json)
├── config.rs (settings hierarchy)
└── error.rs (thiserror, anyhow)
```

---

## 2. Version Detection Strategy

### Module: `version.rs`

**Purpose**: Parse, compare, and manage semantic versions using CARGO_PKG_VERSION

**Contract**:
- Input: Version string (e.g., "0.1.0")
- Output: Structured version with comparison operations
- Side Effects: None (pure computation)

**Implementation**:

```rust
pub struct Version {
    major: u32,
    minor: u32,
    patch: u32,
    metadata: Option<String>,  // e.g., "-beta.1"
}

impl Version {
    // Compile-time constant from CARGO_PKG_VERSION
    pub const fn current() -> Self;

    // Parse semantic version string
    pub fn parse(s: &str) -> Result<Self>;

    // Return true if self < other
    pub fn is_outdated(&self, other: &Version) -> bool;

    // Formatted display for user output
    pub fn to_string(&self) -> String;
}

impl Ord for Version { /* standard semver comparison */ }
```

**Key Design Decision**:
- Compile-time version extraction via macro: `env!("CARGO_PKG_VERSION")`
- Avoids runtime file reads for current version
- Supports pre-release identifiers (alpha, beta, rc)

**Test Requirements**:
- Parse standard semver (1.0.0, 0.1.0)
- Parse with metadata (1.0.0-beta.1)
- Comparison: 1.0.0 > 0.9.9, 1.0.0-beta < 1.0.0
- Parse malformed versions (error cases)

---

## 3. GitHub API Client

### Module: `github_client.rs`

**Purpose**: Fetch release information from GitHub Releases API

**Contract**:
- Input: GitHub owner/repo, token (optional for higher rate limit)
- Output: Vec<Release> with download URL, version, checksums
- Side Effects: HTTP requests, may fail due to network

**Design**:

```rust
pub struct GitHubClient {
    owner: String,
    repo: String,
    token: Option<String>,
    http_client: reqwest::Client,
}

pub struct Release {
    pub tag: String,              // e.g., "v0.2.0"
    pub version: Version,
    pub body: String,             // Release notes
    pub published_at: DateTime<Utc>,
    pub assets: Vec<ReleaseAsset>,
}

pub struct ReleaseAsset {
    pub name: String,             // e.g., "rusty-0.2.0-x86_64-unknown-linux-gnu"
    pub download_url: String,
    pub size: u64,
}

impl GitHubClient {
    // Construct with owner/repo
    pub async fn new(owner: &str, repo: &str) -> Result<Self>;

    // Set auth token for higher rate limits
    pub fn with_token(self, token: String) -> Self;

    // Fetch all releases, sorted newest first
    pub async fn list_releases(&self) -> Result<Vec<Release>>;

    // Fetch single release by tag
    pub async fn get_release(&self, tag: &str) -> Result<Release>;

    // Find asset matching platform/architecture
    pub async fn find_asset(
        &self,
        releases: &[Release],
        platform: &str,  // "x86_64-unknown-linux-gnu"
    ) -> Result<Option<ReleaseAsset>>;
}
```

**API Endpoint Used**:
```
GET /repos/{owner}/{repo}/releases
GET /repos/{owner}/{repo}/releases/latest
GET /repos/{owner}/{repo}/releases/tags/{tag}
```

**Error Handling**:
- 404: Repository not found
- 401: Invalid token
- 403: Rate limited
- 500+: Server errors (retry strategy)

**Test Requirements**:
- Mock HTTP responses (use mockito crate)
- Parse GitHub API response format
- Handle rate limiting (X-RateLimit headers)
- Extract platform-specific asset

---

## 4. Binary Download & Verification

### Module: `downloader.rs`

**Purpose**: Download binary and verify SHA256 checksum

**Contract**:
- Input: Download URL, expected SHA256, destination path
- Output: Path to downloaded file (verified)
- Side Effects: Creates file, validates integrity

**Design**:

```rust
pub struct Downloader {
    http_client: reqwest::Client,
    timeout: Duration,
}

pub struct DownloadProgress {
    pub bytes_downloaded: u64,
    pub total_bytes: u64,
}

impl Downloader {
    pub fn new() -> Self;

    pub fn with_timeout(self, timeout: Duration) -> Self;

    // Download with progress callback
    pub async fn download<F>(
        &self,
        url: &str,
        destination: &Path,
        expected_sha256: Option<&str>,
        on_progress: F,
    ) -> Result<PathBuf>
    where
        F: Fn(DownloadProgress),
    {
        // 1. Stream response body to file
        // 2. Calculate SHA256 during download
        // 3. Verify checksum if provided
        // 4. Return path or error
    }

    // Resumable download (if supported by server)
    pub async fn download_resumable(
        &self,
        url: &str,
        destination: &Path,
        expected_sha256: Option<&str>,
    ) -> Result<PathBuf>;
}
```

**SHA256 Verification**:
- Stream-based: Calculate hash during download (zero extra I/O)
- If checksum fails: Delete partial file, return error
- Fallback: Re-download with exponential backoff (max 3 attempts)

**Progress Tracking**:
- Callback invoked every 1 MiB or 0.5 sec (whichever is more frequent)
- Allows TUI to display download progress

**Test Requirements**:
- Download valid file
- Verify correct SHA256
- Detect corrupted download (wrong hash)
- Resume incomplete download (if server supports Range header)
- Network timeout behavior

---

## 5. Atomic Replacement with Rollback

### Modules: `backup.rs`, `installer.rs`

#### 5.1 Backup Strategy (`backup.rs`)

**Purpose**: Maintain backup of current binary for rollback

**Contract**:
- Input: Current binary path
- Output: Backup metadata (version, timestamp, path)
- Side Effects: Creates backup in ~/.rusty/backups/

**Design**:

```rust
pub struct BackupMetadata {
    pub version: Version,
    pub timestamp: DateTime<Utc>,
    pub backup_path: PathBuf,
    pub original_path: PathBuf,
}

pub struct BackupManager {
    backup_dir: PathBuf,  // ~/.rusty/backups/
    max_backups: usize,   // Keep max 5 backups
}

impl BackupManager {
    pub fn new() -> Result<Self>;

    // Create backup of current binary, return metadata
    pub async fn backup_current(
        &self,
        binary_path: &Path,
        current_version: &Version,
    ) -> Result<BackupMetadata>;

    // List available backups (newest first)
    pub fn list_backups(&self) -> Result<Vec<BackupMetadata>>;

    // Clean old backups, keeping only max_backups
    pub fn cleanup_old_backups(&self) -> Result<()>;

    // Get backup metadata for specific version
    pub fn find_backup(&self, version: &Version) -> Result<Option<BackupMetadata>>;
}
```

**Backup Storage**:
```
~/.rusty/backups/
├── 0.1.0_20251116T143022Z.tar.gz   # Version_timestamp
├── 0.0.9_20251110T080000Z.tar.gz
└── backups.json                     # Metadata index
```

**Atomic Guarantee**: All backup operations are all-or-nothing using temporary files and rename

#### 5.2 Atomic Installer (`installer.rs`)

**Purpose**: Platform-specific atomic binary replacement with guaranteed rollback

**Contract**:
- Input: New binary path, current binary path, backup
- Output: Success or automatic rollback
- Side Effects: Replaces running binary (restart required)

**Design - Linux Implementation**:

```rust
pub trait InstallerStrategy: Send + Sync {
    // Perform atomic replacement, returns result or error
    async fn install_atomic(
        &self,
        new_binary: &Path,
        current_binary: &Path,
        backup_meta: &BackupMetadata,
    ) -> Result<()>;

    async fn rollback_atomic(
        &self,
        backup_meta: &BackupMetadata,
        current_binary: &Path,
    ) -> Result<()>;
}

pub struct LinuxInstaller;

impl InstallerStrategy for LinuxInstaller {
    async fn install_atomic(
        &self,
        new_binary: &Path,
        current_binary: &Path,
        backup_meta: &BackupMetadata,
    ) -> Result<()> {
        // 1. Verify permissions (must be writable)
        // 2. Create atomic temp directory
        // 3. Extract new binary to temp location
        // 4. Verify new binary is executable
        // 5. Create hard link to backup
        // 6. Atomic rename: new_temp -> current_binary
        //    (single syscall, guaranteed all-or-nothing)
        // 7. Verify replacement (test binary --version)
        // On error at any step: rollback via rename backup -> current_binary
    }
}

pub struct Installer {
    strategy: Box<dyn InstallerStrategy>,
    state: Arc<Mutex<InstallerState>>,
}
```

**Atomic Rename Strategy (Linux)**:

```
Before:
  /usr/local/bin/rusty          <- Running binary (locked by kernel)
  /tmp/rusty-new.XXXXXX         <- Downloaded binary

After atomic rename (single syscall):
  /usr/local/bin/rusty          <- New binary (old inode no longer exists)

Why this works:
- rename() is atomic at filesystem level
- Running processes continue on old inode (unlinked)
- New executions get new binary
- At process exit, old inode is freed
```

**Rollback Mechanism**:
```rust
pub async fn rollback(
    &self,
    binary_path: &Path,
    backup_meta: &BackupMetadata,
) -> Result<()> {
    // Extract backup
    // Atomic rename: backup -> binary_path
    // Verify old version restored
}
```

**Test Requirements**:
- Successful atomic replacement
- Automatic rollback on verification failure
- Backup integrity after installation
- Multiple sequential updates

---

## 6. Update State Persistence

### Module: `state.rs`

**Purpose**: Track update attempts, schedule next check, store metadata

**Contract**:
- Input: Version, timestamp, status
- Output: Structured state file
- Side Effects: Writes to ~/.rusty/update_state.json

**Design**:

```rust
#[derive(Serialize, Deserialize)]
pub struct UpdateState {
    pub current_version: Version,
    pub last_check: DateTime<Utc>,
    pub next_check: DateTime<Utc>,
    pub last_update: Option<DateTime<Utc>>,
    pub last_update_version: Option<Version>,
    pub last_failed_check: Option<UpdateCheckFailure>,
    pub automatic_update_enabled: bool,
}

#[derive(Serialize, Deserialize)]
pub struct UpdateCheckFailure {
    pub timestamp: DateTime<Utc>,
    pub error: String,
    pub retry_count: u32,
}

impl UpdateState {
    pub fn new(current_version: Version) -> Self;

    // Load from disk
    pub async fn load() -> Result<Self>;

    // Save to disk atomically
    pub async fn save(&self) -> Result<()>;

    // Check if daily check is due
    pub fn is_check_due(&self) -> bool;

    // Mark successful check, schedule next in 24h
    pub fn mark_checked(&mut self);

    // Record update completion
    pub fn mark_updated(&mut self, new_version: Version);

    // Record check failure with retry logic
    pub fn record_failure(&mut self, error: String);
}
```

**State File Location**: `~/.rusty/update_state.json`

**Example State File**:
```json
{
  "current_version": "0.1.0",
  "last_check": "2025-11-16T14:30:22Z",
  "next_check": "2025-11-17T14:30:22Z",
  "last_update": "2025-11-10T08:15:00Z",
  "last_update_version": "0.1.0",
  "automatic_update_enabled": true,
  "last_failed_check": null
}
```

**Test Requirements**:
- Serialize/deserialize state
- Persistence across restarts
- Handle missing state file gracefully
- Handle corrupted state file (reset to default)

---

## 7. Auto-Check Scheduling

### Module: `scheduler.rs`

**Purpose**: Check for updates once per 24 hours in background

**Contract**:
- Input: Enable/disable flag from config
- Output: Async task handling background checks
- Side Effects: Periodic GitHub API calls

**Design**:

```rust
pub struct UpdateScheduler {
    enabled: bool,
    check_interval: Duration,  // 24 hours default
}

impl UpdateScheduler {
    pub fn new(enabled: bool) -> Self;

    // Start background check task
    // Returns JoinHandle for graceful shutdown
    pub fn start_background_check(
        self,
        state_path: &Path,
    ) -> tokio::task::JoinHandle<()>;

    // Check if update is due based on state
    pub async fn check_due(state_path: &Path) -> Result<bool>;

    // Perform check, update state, notify user if update available
    pub async fn perform_check() -> Result<UpdateCheckResult>;
}

pub struct UpdateCheckResult {
    pub current_version: Version,
    pub latest_version: Version,
    pub update_available: bool,
    pub release_notes: String,
}
```

**Scheduling Strategy**:
- Check runs at CLI startup IF 24h since last check
- Check runs asynchronously in background (non-blocking)
- User can force check: `rusty update --check`
- Results cached in state file

**Notification**:
- Display one-line message if update available
- Do not interrupt user interaction
- Suggestion: `rusty update` to install

**Test Requirements**:
- Background task scheduling
- Respects 24h interval
- Handles scheduler disable
- Mock time advancement

---

## 8. Configuration Integration

### Module: `config.rs`

**Purpose**: Integrate with existing settings hierarchy

**Contract**:
- Input: Settings from hierarchy
- Output: UpdateConfig with effective values
- Side Effects: Reads from settings files

**Design**:

```rust
pub struct UpdateConfig {
    pub enabled: bool,                    // Default: true
    pub auto_check: bool,                 // Default: true (once per 24h)
    pub auto_install: bool,               // Default: false (user must confirm)
    pub github_token: Option<String>,     // For higher rate limits
    pub backup_count: usize,              // Default: 5
    pub backup_dir: PathBuf,              // Default: ~/.rusty/backups
    pub repository: String,               // owner/repo format
    pub target_triple: String,            // e.g., x86_64-unknown-linux-gnu
}

impl UpdateConfig {
    // Load from settings hierarchy
    pub fn from_settings(settings: &Settings) -> Result<Self>;

    // Validate configuration
    pub fn validate(&self) -> Result<()>;

    // Load with platform-specific defaults
    pub fn with_platform_defaults() -> Self;
}
```

**Configuration Keys** (in ~/.claude/config):

```toml
[update]
enabled = true
auto_check = true
auto_install = false
github_token = "ghp_xxx..."  # Optional
backup_count = 5
repository = "anthropics/Claude-Code-Rust"  # Example

# Platform-specific overrides
[update.linux]
target_triple = "x86_64-unknown-linux-gnu"

[update.darwin]
target_triple = "x86_64-apple-darwin"
```

**Test Requirements**:
- Load from various config sources
- Validate all fields
- Apply platform-specific defaults
- Handle missing config (use defaults)

---

## 9. Error Handling

### Module: `error.rs`

**Purpose**: Unified error types for update operations

**Design**:

```rust
#[derive(thiserror::Error, Debug)]
pub enum UpdateError {
    #[error("Version parsing failed: {0}")]
    VersionParse(String),

    #[error("GitHub API error: {0}")]
    GitHubApi(String),

    #[error("Download failed: {0}")]
    DownloadFailed(String),

    #[error("SHA256 verification failed: expected {expected}, got {actual}")]
    ChecksumMismatch { expected: String, actual: String },

    #[error("Installation failed: {0}")]
    InstallationFailed(String),

    #[error("Rollback failed: {0}")]
    RollbackFailed(String),

    #[error("State management error: {0}")]
    StateError(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Platform not supported: {0}")]
    UnsupportedPlatform(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Network error: {0}")]
    Network(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

pub type UpdateResult<T> = Result<T, UpdateError>;
```

**Test Requirements**:
- Error messages are clear and actionable
- Error chain is preserved (backtraces)
- Serialization errors are properly wrapped

---

## 10. Public API Module (`mod.rs`)

**Purpose**: Orchestrate update flow, expose user-facing commands

**Design**:

```rust
pub use self::error::{UpdateError, UpdateResult};
pub use self::version::Version;
pub use self::github_client::{GitHubClient, Release};
pub use self::config::UpdateConfig;

pub struct UpdateManager {
    config: UpdateConfig,
    github_client: GitHubClient,
    downloader: Downloader,
    backup_manager: BackupManager,
    installer: Installer,
    state: UpdateState,
}

impl UpdateManager {
    pub async fn new() -> UpdateResult<Self>;

    // Main flow: check for updates
    pub async fn check_updates(&self) -> UpdateResult<CheckResult>;

    // Main flow: perform update
    pub async fn update(&self) -> UpdateResult<UpdateResult>;

    // Main flow: rollback to previous version
    pub async fn rollback(&mut self) -> UpdateResult<()>;

    // Check if update is available without installing
    pub async fn check_only(&self) -> UpdateResult<bool>;

    // Get update information without performing action
    pub async fn get_update_info(&self) -> UpdateResult<Option<Release>>;
}

pub struct CheckResult {
    pub available: bool,
    pub current: Version,
    pub latest: Option<Version>,
    pub release_notes: Option<String>,
}

pub struct UpdateResult {
    pub success: bool,
    pub previous_version: Version,
    pub new_version: Version,
    pub timestamp: DateTime<Utc>,
}
```

---

## 11. Implementation Plan (4 Phases)

### Phase 1: Foundation (Days 1-2)
**Deliverable**: Version detection, GitHub client, error types

1. Create module structure and error types
2. Implement Version struct with semver parsing and comparison
3. Implement GitHubClient with API integration
4. Create configuration loading from settings hierarchy
5. Write comprehensive unit tests

**Success Criteria**:
- Version comparison works correctly
- GitHub API client fetches releases successfully
- All error types are well-typed

**Files to Create**:
- `crates/cli/src/update/mod.rs`
- `crates/cli/src/update/error.rs`
- `crates/cli/src/update/version.rs`
- `crates/cli/src/update/github_client.rs`
- `crates/cli/src/update/config.rs`

**Dependencies to Add**:
```toml
[dependencies]
sha2 = "0.10"           # SHA256 hashing
reqwest = { version = "0.11", features = ["stream"] }  # HTTP
serde = { workspace = true }
serde_json = { workspace = true }
chrono = { workspace = true }
thiserror = { workspace = true }
```

### Phase 2: Download & Backup (Days 3-4)
**Deliverable**: Binary download with verification, backup system

1. Implement Downloader with progress tracking
2. Implement BackupManager with atomic operations
3. Create UpdateState persistence
4. Integration tests for download + backup

**Success Criteria**:
- Download with SHA256 verification works
- Backup system creates restorable backups
- State persists across restarts

**Files to Create**:
- `crates/cli/src/update/downloader.rs`
- `crates/cli/src/update/backup.rs`
- `crates/cli/src/update/state.rs`

**Testing**:
- Mock download server
- Test backup restoration
- Test state file format

### Phase 3: Atomic Installation (Days 5-6)
**Deliverable**: Linux atomic replacement, rollback mechanism

1. Implement LinuxInstaller with atomic rename
2. Implement rollback logic
3. Platform abstraction for future macOS/Windows
4. Integration tests (test update + rollback flow)

**Success Criteria**:
- Binary replacement is atomic
- Rollback works correctly
- No data loss or inconsistency

**Files to Create**:
- `crates/cli/src/update/installer.rs`

**Critical Testing**:
- Atomic replacement verification
- Rollback after failed update
- Multiple sequential updates

### Phase 4: Scheduler & CLI Integration (Days 7-10)
**Deliverable**: Background scheduling, CLI commands, end-to-end integration

1. Implement UpdateScheduler
2. Integrate with main CLI: `rusty update`, `rusty update --check`, `rusty update --rollback`
3. Add background task to CLI startup
4. End-to-end testing

**Success Criteria**:
- Commands work as specified
- Background checks run correctly
- All workflows function end-to-end

**Files to Create**:
- `crates/cli/src/update/scheduler.rs`
- CLI command handlers in `crates/cli/src/commands/update.rs`

**Integration Points**:
- Add `Commands::Update` to main CLI parser
- Wire UpdateManager into InteractiveSession
- Add startup hook for background scheduler

---

## 12. Command Interface

### `rusty update`
Perform update to latest version
```bash
$ rusty update
Checking for updates...
Latest version: 0.2.0 (current: 0.1.0)
Release notes:
- Performance improvements
- Bug fixes

Update available. Proceed? (y/n): y
Downloading binary... [████████████████] 100%
Verifying checksum... OK
Creating backup of 0.1.0... OK
Installing new version... OK
Successfully updated to 0.2.0
Restart required: run 'rusty' to use the new version
```

### `rusty update --check`
Check for available updates without installing
```bash
$ rusty update --check
Checking for updates...
Update available: 0.2.0 (current: 0.1.0)
Run 'rusty update' to install
```

### `rusty update --rollback`
Rollback to previous version
```bash
$ rusty update --rollback
Available backups:
1. 0.1.0 (2025-11-16T14:30:22Z)
2. 0.0.9 (2025-11-10T08:15:00Z)

Select backup to restore (1-2): 1
Restoring 0.1.0... OK
Successfully rolled back to 0.1.0
Restart required: run 'rusty' to use the restored version
```

---

## 13. Cross-Platform Roadmap

### Linux (Phase 4)
- Primary implementation complete
- Atomic rename at filesystem level
- Shell script wrapper not required

### macOS (Future)
- Same LinuxInstaller approach (both use POSIX rename)
- Target triple: `x86_64-apple-darwin`, `aarch64-apple-darwin`
- Notarization check if installed via Apple ecosystem

### Windows (Future)
- Different strategy needed (cannot rename running binary)
- Approach: Use .new suffix, swap on next start
- Target triple: `x86_64-pc-windows-msvc`
- Requires Windows-specific InstallerStrategy

---

## 14. Testing Strategy

### Unit Tests (per module)
```
version.rs:
  - test_parse_semver
  - test_parse_prerelease
  - test_version_comparison
  - test_parse_invalid

github_client.rs:
  - test_list_releases (mocked)
  - test_get_release (mocked)
  - test_find_asset
  - test_rate_limit_handling

downloader.rs:
  - test_download_success
  - test_download_with_sha256
  - test_sha256_mismatch
  - test_resumable_download
  - test_timeout

backup.rs:
  - test_create_backup
  - test_list_backups
  - test_cleanup_old
  - test_corrupted_backup

installer.rs (Linux):
  - test_atomic_replacement
  - test_rollback
  - test_verify_permissions

scheduler.rs:
  - test_check_due
  - test_24h_interval
  - test_background_task
```

### Integration Tests
```
test_full_update_flow()
test_update_with_rollback()
test_failed_update_recovery()
test_concurrent_updates()
test_state_persistence()
```

### End-to-End Tests (require real binaries)
```
test_e2e_update_command()
test_e2e_rollback_command()
test_e2e_check_command()
```

---

## 15. Risk Analysis & Mitigations

| Risk | Impact | Mitigation |
|------|--------|-----------|
| Running binary replaced during execution | HIGH | Atomic rename at kernel level (Linux) |
| Checksum mismatch corrupts system | MEDIUM | Verify before replacing, keep backup |
| State file corruption | LOW | Atomic save, graceful fallback to defaults |
| Network failure during download | MEDIUM | Resumable downloads, retry logic |
| Insufficient disk space | LOW | Check free space before download |
| GitHub API rate limit | LOW | Optional auth token, cache releases |
| User kills process mid-update | MEDIUM | Atomic operations + backup rollback |

---

## 16. Performance Considerations

- **Download**: Streamed to avoid memory overhead
- **SHA256**: Calculated during download (single pass)
- **Backup**: Compressed .tar.gz to minimize disk usage
- **Atomic operations**: Single syscall (microseconds)
- **State file**: Small JSON (~500 bytes), async I/O
- **Background check**: Non-blocking, 100ms timeout for GitHub API

---

## 17. Security Considerations

- **SHA256 verification**: Mandatory for release binaries
- **GitHub HTTPS only**: No fallback to HTTP
- **Token storage**: In plaintext (user responsible for security)
- **Backup integrity**: Verify after extraction before use
- **Permissions**: Respect umask, maintain executable bit
- **No privilege escalation**: Update to user-owned binary only

---

## Summary

This architecture provides:
1. **Modularity**: Each component has single responsibility
2. **Atomicity**: Kernel-level guarantees for safe replacement
3. **Recoverability**: Backup + rollback mechanism
4. **Scalability**: Platform abstraction for future OSes
5. **Testability**: Clear contracts for mocking
6. **Simplicity**: ~2500 LOC, no external crate surprises

Implementation follows the brick philosophy: minimal components with clear contracts, regeneratable from specification alone.
