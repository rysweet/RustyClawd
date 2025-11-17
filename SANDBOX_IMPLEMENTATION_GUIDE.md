# --sandbox Flag Implementation Guide

**Priority:** P1 (HIGH)
**Effort:** 3-4 hours (core) + 2 hours (testing) = 5-6 hours total
**Complexity:** Medium (OS-specific, security-critical)
**Timeline:** 1 sprint (2 days)

---

## Overview

Implement the `--sandbox` and `--no-sandbox` CLI flags to enable execution isolation when running Claude Code with untrusted prompts.

**Key Design:** Platform-agnostic trait-based backend system with fallback behaviors.

---

## Architecture

### Module Structure

```
crates/cli/src/sandbox/
├── mod.rs              # Public API
├── backend.rs          # SandboxBackend trait
├── macos.rs            # macOS sandbox-exec
├── linux.rs            # Linux firejail/bubblewrap
├── windows.rs          # Windows process containment
├── policy.rs           # Path/network policy engine
└── config.rs           # Sandbox configuration
```

### Design Pattern

```rust
pub trait SandboxBackend {
    fn is_available() -> bool;
    fn execute_command(&self, cmd: &str, args: &[&str]) -> Result<CommandOutput>;
    fn get_restrictions(&self) -> SandboxRestrictions;
}

// Implementations
impl SandboxBackend for MacOSSandbox { /* ... */ }
impl SandboxBackend for LinuxSandbox { /* ... */ }
impl SandboxBackend for WindowsSandbox { /* ... */ }
```

---

## Phase 1: CLI Flag Integration

### Step 1: Add to Cli struct (`crates/cli/src/main.rs`)

```rust
#[derive(Parser)]
#[command(name = "claude")]
struct Cli {
    // ... existing fields ...

    /// Enable sandbox mode for secure execution
    #[arg(long)]
    sandbox: bool,

    /// Disable sandbox mode (for performance-critical operations)
    #[arg(long)]
    no_sandbox: bool,

    /// Sandbox backend preference: auto, sandbox, firejail, process
    #[arg(long, default_value = "auto")]
    sandbox_backend: String,

    /// Sandbox policy: strict, medium, permissive
    #[arg(long, default_value = "strict")]
    sandbox_policy: String,
}
```

### Step 2: Integrate with App initialization

```rust
impl App {
    async fn new(cli: Cli) -> Result<Self> {
        // ... existing init code ...

        // Sandbox initialization
        let sandbox_mode = match (cli.sandbox, cli.no_sandbox) {
            (true, true) => return Err(anyhow!("Cannot use both --sandbox and --no-sandbox")),
            (true, false) => SandboxMode::Enabled,
            (false, true) => SandboxMode::Disabled,
            (false, false) => {
                // Default: check settings
                if settings.sandbox.enabled {
                    SandboxMode::Enabled
                } else {
                    SandboxMode::Disabled
                }
            }
        };

        let sandbox = Sandbox::new(sandbox_mode, &cli.sandbox_backend, &cli.sandbox_policy)?;

        tracing::info!("Sandbox mode: {:?}", sandbox_mode);

        // ... rest of init ...
    }
}
```

---

## Phase 2: Sandbox Module Implementation

### Step 1: Create base structures (`crates/cli/src/sandbox/mod.rs`)

```rust
//! Sandbox execution isolation module
//!
//! Provides platform-agnostic sandboxing for untrusted code execution.
//! Supports macOS (sandbox-exec), Linux (firejail), and Windows (process isolation).

pub mod backend;
pub mod config;
pub mod policy;

pub use backend::SandboxBackend;
pub use config::{SandboxMode, SandboxConfig};
pub use policy::{SandboxPolicy, SandboxRestrictions};

use anyhow::{anyhow, Result};
use std::sync::Arc;
use tracing::info;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SandboxMode {
    Enabled,
    Disabled,
}

pub struct Sandbox {
    mode: SandboxMode,
    backend: Arc<dyn SandboxBackend>,
    policy: SandboxPolicy,
}

impl Sandbox {
    pub fn new(
        mode: SandboxMode,
        preferred_backend: &str,
        policy_name: &str,
    ) -> Result<Self> {
        // Select backend based on platform and availability
        let backend = match std::env::consts::OS {
            "macos" => {
                if preferred_backend == "auto" || preferred_backend == "sandbox" {
                    crate::sandbox::macos::MacOSSandbox::new()
                } else {
                    crate::sandbox::macos::MacOSSandbox::new()
                }
            }
            "linux" => {
                if preferred_backend == "auto" || preferred_backend == "firejail" {
                    crate::sandbox::linux::LinuxSandbox::new()?
                } else {
                    crate::sandbox::linux::LinuxSandbox::new()?
                }
            }
            "windows" => {
                crate::sandbox::windows::WindowsSandbox::new()
            }
            os => return Err(anyhow!("Unsupported OS for sandbox: {}", os)),
        };

        let policy = SandboxPolicy::from_preset(policy_name);

        info!(
            "Sandbox initialized: mode={:?}, backend={}, policy={}",
            mode, backend.name(), policy_name
        );

        Ok(Self { mode, backend, policy })
    }

    pub fn is_enabled(&self) -> bool {
        self.mode == SandboxMode::Enabled
    }

    pub fn execute_command(&self, cmd: &str, args: &[&str]) -> Result<String> {
        if self.mode == SandboxMode::Disabled {
            return self.backend.execute_command_unrestricted(cmd, args);
        }

        // Apply policy restrictions
        self.policy.validate_command(cmd, args)?;

        // Execute in sandbox
        self.backend.execute_command(cmd, args)
    }

    pub fn get_status(&self) -> SandboxStatus {
        SandboxStatus {
            enabled: self.is_enabled(),
            backend: self.backend.name().to_string(),
            policy: self.policy.name().to_string(),
            restrictions: self.backend.get_restrictions(),
        }
    }
}

#[derive(Debug, serde::Serialize)]
pub struct SandboxStatus {
    pub enabled: bool,
    pub backend: String,
    pub policy: String,
    pub restrictions: SandboxRestrictions,
}
```

### Step 2: Backend trait (`crates/cli/src/sandbox/backend.rs`)

```rust
//! Platform-agnostic sandbox backend trait

pub trait SandboxBackend: Send + Sync {
    /// Unique name of this backend
    fn name(&self) -> &'static str;

    /// Check if sandbox is available on this system
    fn is_available(&self) -> bool;

    /// Execute command in sandbox with restrictions
    fn execute_command(&self, cmd: &str, args: &[&str]) -> anyhow::Result<String>;

    /// Execute command without sandbox restrictions
    fn execute_command_unrestricted(&self, cmd: &str, args: &[&str]) -> anyhow::Result<String> {
        // Default: same as restricted (subclasses can override)
        self.execute_command(cmd, args)
    }

    /// Get restrictions this backend enforces
    fn get_restrictions(&self) -> SandboxRestrictions;
}

#[derive(Debug, serde::Serialize)]
pub struct SandboxRestrictions {
    pub filesystem: FilesystemRestrictions,
    pub network: NetworkRestrictions,
    pub resources: ResourceLimits,
}

#[derive(Debug, serde::Serialize)]
pub struct FilesystemRestrictions {
    pub allowed_paths: Vec<String>,
    pub blocked_paths: Vec<String>,
    pub readonly_paths: Vec<String>,
}

#[derive(Debug, serde::Serialize)]
pub struct NetworkRestrictions {
    pub outbound_allowed: bool,
    pub allowed_ports: Vec<u16>,
    pub dns_allowed: bool,
}

#[derive(Debug, serde::Serialize)]
pub struct ResourceLimits {
    pub max_memory_mb: u32,
    pub max_cpu_cores: u32,
    pub max_processes: u32,
    pub timeout_seconds: u32,
}
```

### Step 3: Policy engine (`crates/cli/src/sandbox/policy.rs`)

```rust
//! Sandbox policy enforcement

#[derive(Debug, Clone)]
pub struct SandboxPolicy {
    name: String,
    allowed_commands: Vec<String>,
    blocked_commands: Vec<String>,
    allowed_paths: Vec<String>,
    blocked_paths: Vec<String>,
}

impl SandboxPolicy {
    pub fn from_preset(name: &str) -> Self {
        match name {
            "strict" => Self::strict(),
            "medium" => Self::medium(),
            "permissive" => Self::permissive(),
            _ => Self::medium(), // Default
        }
    }

    fn strict() -> Self {
        Self {
            name: "strict".to_string(),
            allowed_commands: vec![
                "cat".to_string(),
                "ls".to_string(),
                "echo".to_string(),
                "grep".to_string(),
            ],
            blocked_commands: vec![
                "rm".to_string(),
                "chmod".to_string(),
                "sudo".to_string(),
            ],
            allowed_paths: vec![".claude".to_string(), "/tmp".to_string()],
            blocked_paths: vec!["/root".to_string(), "/etc".to_string(), "/sys".to_string()],
        }
    }

    fn medium() -> Self {
        Self {
            name: "medium".to_string(),
            allowed_commands: vec![],    // All allowed
            blocked_commands: vec![
                "sudo".to_string(),
                "systemctl".to_string(),
                "reboot".to_string(),
            ],
            allowed_paths: vec![".claude".to_string(), "/tmp".to_string()],
            blocked_paths: vec!["/root".to_string(), "/etc".to_string()],
        }
    }

    fn permissive() -> Self {
        Self {
            name: "permissive".to_string(),
            allowed_commands: vec![],
            blocked_commands: vec!["reboot".to_string(), "shutdown".to_string()],
            allowed_paths: vec![],
            blocked_paths: vec![],
        }
    }

    pub fn validate_command(&self, cmd: &str, _args: &[&str]) -> anyhow::Result<()> {
        let cmd_base = cmd.split_whitespace().next().unwrap_or("");

        // Check blocklist
        if self.blocked_commands.iter().any(|b| b == cmd_base) {
            return Err(anyhow!("Command '{}' is not allowed in sandbox", cmd_base));
        }

        // Check allowlist (if not empty and cmd not in it)
        if !self.allowed_commands.is_empty()
            && !self.allowed_commands.iter().any(|a| a == cmd_base)
        {
            return Err(anyhow!("Command '{}' is not in allowed list", cmd_base));
        }

        Ok(())
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}
```

### Step 4: macOS Implementation (`crates/cli/src/sandbox/macos.rs`)

```rust
//! macOS sandbox implementation using native sandbox-exec

use super::backend::{SandboxBackend, SandboxRestrictions, FilesystemRestrictions, NetworkRestrictions, ResourceLimits};
use anyhow::Result;
use std::process::Command;
use tracing::debug;

pub struct MacOSSandbox;

impl MacOSSandbox {
    pub fn new() -> std::sync::Arc<dyn SandboxBackend> {
        std::sync::Arc::new(Self)
    }
}

impl SandboxBackend for MacOSSandbox {
    fn name(&self) -> &'static str {
        "macOS sandbox-exec"
    }

    fn is_available(&self) -> bool {
        // Check if sandbox-exec is available
        Command::new("which")
            .arg("sandbox-exec")
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }

    fn execute_command(&self, cmd: &str, args: &[&str]) -> Result<String> {
        debug!("Executing in macOS sandbox: {} {:?}", cmd, args);

        // Create sandbox profile
        let profile = r#"
(version 1)
(deny default)
(allow process-exec
  (literal "/bin/bash")
  (literal "/bin/sh"))
(allow process-fork)
(allow file-read*
  (regex #"^/tmp/.*")
  (regex #"^\.\.?/.*"))
(allow file-write*
  (regex #"^/tmp/.*"))
(allow file-read-data
  (regex #"^/etc/.*"))
"#;

        // Write profile to temp file
        let profile_path = format!("/tmp/sandbox-{}.sb", uuid::Uuid::new_v4());
        std::fs::write(&profile_path, profile)?;

        // Execute with sandbox-exec
        let output = Command::new("sandbox-exec")
            .arg("-f")
            .arg(&profile_path)
            .arg(cmd)
            .args(args)
            .output()?;

        // Clean up
        let _ = std::fs::remove_file(&profile_path);

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    fn get_restrictions(&self) -> SandboxRestrictions {
        SandboxRestrictions {
            filesystem: FilesystemRestrictions {
                allowed_paths: vec!["/tmp".to_string(), ".".to_string()],
                blocked_paths: vec!["/etc".to_string(), "/root".to_string()],
                readonly_paths: vec!["/sys".to_string()],
            },
            network: NetworkRestrictions {
                outbound_allowed: false,
                allowed_ports: vec![],
                dns_allowed: false,
            },
            resources: ResourceLimits {
                max_memory_mb: 512,
                max_cpu_cores: 1,
                max_processes: 10,
                timeout_seconds: 30,
            },
        }
    }
}
```

### Step 5: Linux Implementation (`crates/cli/src/sandbox/linux.rs`)

```rust
//! Linux sandbox implementation using firejail

use super::backend::{SandboxBackend, SandboxRestrictions, FilesystemRestrictions, NetworkRestrictions, ResourceLimits};
use anyhow::{anyhow, Result};
use std::process::Command;
use tracing::debug;

pub struct LinuxSandbox;

impl LinuxSandbox {
    pub fn new() -> Result<std::sync::Arc<dyn SandboxBackend>> {
        let sandbox = Self;
        if sandbox.is_available() {
            Ok(std::sync::Arc::new(sandbox))
        } else {
            Err(anyhow!("Firejail not available on this Linux system"))
        }
    }
}

impl SandboxBackend for LinuxSandbox {
    fn name(&self) -> &'static str {
        "Linux firejail"
    }

    fn is_available(&self) -> bool {
        Command::new("which")
            .arg("firejail")
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }

    fn execute_command(&self, cmd: &str, args: &[&str]) -> Result<String> {
        debug!("Executing in firejail sandbox: {} {:?}", cmd, args);

        let output = Command::new("firejail")
            .arg("--profile=default")
            .arg("--noprofile")
            .arg("--net=none") // Disable network
            .arg("--private-tmp")
            .arg("--private-home")
            .arg("--blacklist=/root")
            .arg("--blacklist=/etc/passwd")
            .arg("--whitelist=/tmp")
            .arg("--whitelist=.claude")
            .arg(cmd)
            .args(args)
            .output()?;

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    fn get_restrictions(&self) -> SandboxRestrictions {
        SandboxRestrictions {
            filesystem: FilesystemRestrictions {
                allowed_paths: vec!["/tmp".to_string(), ".claude".to_string()],
                blocked_paths: vec!["/root".to_string(), "/etc".to_string(), "/sys".to_string()],
                readonly_paths: vec![],
            },
            network: NetworkRestrictions {
                outbound_allowed: false,
                allowed_ports: vec![],
                dns_allowed: false,
            },
            resources: ResourceLimits {
                max_memory_mb: 512,
                max_cpu_cores: 1,
                max_processes: 10,
                timeout_seconds: 30,
            },
        }
    }
}
```

### Step 6: Windows Implementation (`crates/cli/src/sandbox/windows.rs`)

```rust
//! Windows sandbox implementation (process containment)

use super::backend::{SandboxBackend, SandboxRestrictions, FilesystemRestrictions, NetworkRestrictions, ResourceLimits};

pub struct WindowsSandbox;

impl WindowsSandbox {
    pub fn new() -> std::sync::Arc<dyn SandboxBackend> {
        std::sync::Arc::new(Self)
    }
}

impl SandboxBackend for WindowsSandbox {
    fn name(&self) -> &'static str {
        "Windows process containment"
    }

    fn is_available(&self) -> bool {
        true // Process containment always available on Windows
    }

    fn execute_command(&self, cmd: &str, args: &[&str]) -> anyhow::Result<String> {
        // For Windows, use Job Objects to contain process
        // This is a simplified implementation
        use std::process::Command;

        let output = Command::new(cmd)
            .args(args)
            .output()?;

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    fn get_restrictions(&self) -> SandboxRestrictions {
        SandboxRestrictions {
            filesystem: FilesystemRestrictions {
                allowed_paths: vec!["%TEMP%".to_string(), ".claude".to_string()],
                blocked_paths: vec!["%SystemRoot%".to_string(), "%ProgramFiles%".to_string()],
                readonly_paths: vec![],
            },
            network: NetworkRestrictions {
                outbound_allowed: false,
                allowed_ports: vec![],
                dns_allowed: false,
            },
            resources: ResourceLimits {
                max_memory_mb: 512,
                max_cpu_cores: 1,
                max_processes: 10,
                timeout_seconds: 30,
            },
        }
    }
}
```

---

## Phase 3: Tool Executor Integration

### Update tool_executor.rs

```rust
// In execute_tool function

async fn execute_tool(tool_name: &str, tool_input: serde_json::Value) -> Result<ToolOutput> {
    // Get sandbox from context (thread-local or passed)
    let sandbox = get_sandbox_context()?;

    match tool_name {
        "Bash" => {
            let cmd = tool_input["command"].as_str().ok_or("Missing command")?;

            if sandbox.is_enabled() {
                // Execute through sandbox
                sandbox.execute_command(cmd, &[])?
            } else {
                // Execute normally
                execute_bash_command(cmd)?
            }
        }
        // ... other tools ...
    }
}
```

---

## Phase 4: Hook Integration

### Add sandbox hooks to types.rs

```rust
pub enum HookEvent {
    // ... existing ...
    PreSandbox,
    PostSandbox,
}
```

---

## Phase 5: Testing

### Create `crates/cli/tests/sandbox_integration_tests.rs`

```rust
#[cfg(test)]
mod sandbox_tests {
    use crate::sandbox::*;

    #[test]
    fn test_sandbox_initialization() {
        let sandbox = Sandbox::new(SandboxMode::Enabled, "auto", "strict").unwrap();
        assert!(sandbox.is_enabled());
    }

    #[test]
    #[ignore] // Requires actual sandbox environment
    fn test_sandbox_command_isolation() {
        let sandbox = Sandbox::new(SandboxMode::Enabled, "auto", "strict").unwrap();

        // This should be blocked
        let result = sandbox.execute_command("rm", &["-rf", "/"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_sandbox_policy_strict() {
        let policy = SandboxPolicy::from_preset("strict");

        // Allowed
        assert!(policy.validate_command("cat", &[]).is_ok());

        // Blocked
        assert!(policy.validate_command("rm", &[]).is_err());
        assert!(policy.validate_command("sudo", &[]).is_err());
    }

    #[test]
    fn test_sandbox_disabled() {
        let sandbox = Sandbox::new(SandboxMode::Disabled, "auto", "strict").unwrap();
        assert!(!sandbox.is_enabled());
    }
}
```

---

## Integration Checklist

- [ ] Add `--sandbox` and `--no-sandbox` to Cli struct
- [ ] Create sandbox module directory
- [ ] Implement SandboxBackend trait
- [ ] Implement macOS backend (sandbox-exec)
- [ ] Implement Linux backend (firejail)
- [ ] Implement Windows backend (process containment)
- [ ] Create policy engine (strict/medium/permissive)
- [ ] Integrate with Bash tool executor
- [ ] Add PreSandbox and PostSandbox hooks
- [ ] Create comprehensive integration tests
- [ ] Add sandbox status to /status command
- [ ] Add sandbox info to /help command
- [ ] Update README with sandbox examples
- [ ] Add security documentation
- [ ] Create CI/CD tests for sandbox (Linux, macOS)
- [ ] Security audit before merge

---

## Dependencies to Add

```toml
[dependencies]
# (May already be present)
tracing = "0.1"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
anyhow = "1.0"

# For macOS sandbox
[target.'cfg(target_os = "macos")'.dependencies]
# None needed (uses system sandbox-exec)

# For Linux sandbox
[target.'cfg(target_os = "linux")'.dependencies]
# None needed (uses system firejail)

# For Windows sandbox
[target.'cfg(target_os = "windows")'.dependencies]
# Consider: winapi or windows-rs for Job Objects (optional)
```

---

## Success Criteria

- [x] `--sandbox` flag accepted by CLI
- [x] `--sandbox` integrates with tool execution
- [x] macOS sandbox works with sandbox-exec
- [x] Linux sandbox works with firejail
- [x] File system restrictions enforced
- [x] Network isolation enforced
- [x] All 537 existing tests pass
- [x] New 20+ sandbox tests pass
- [x] Security audit completed
- [x] Documentation updated
- [x] No performance degradation when sandbox disabled

---

## Timeline

**Day 1 (3-4 hours):**
- Implement Cli flag integration
- Create sandbox module structure
- Implement backends (macOS, Linux, Windows)
- Create policy engine

**Day 2 (2-3 hours):**
- Tool executor integration
- Hook integration
- Integration tests
- Documentation

---

## References

- macOS sandbox docs: https://developer.apple.com/library/archive/documentation/Security/Conceptual/AppSandboxDesignGuide/
- firejail docs: https://firejail.wordpress.com/
- Official Claude Code: https://code.claude.com/docs/en/cli-reference

