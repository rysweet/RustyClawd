# Implementation Roadmap: New main.rs

## Overview

This document provides step-by-step implementation guidance for the builder agent. The work is organized into 5 phases that build progressively.

## Phase A: Foundation (1-2 days)

### A.1 Create Module Skeleton

**File:** `/crates/cli/src/context.rs`

```rust
use std::time::Instant;
use anyhow::Result;

pub struct ExecutionContext {
    // Configuration
    pub settings: crate::settings::EffectiveSettings,
    pub permissions: PermissionMatrix,

    // CLI command
    pub command: Commands,

    // TODO: Add remaining fields
}

pub struct PermissionMatrix {
    // TODO: Implement permission checking
}

impl ExecutionContext {
    pub fn can_execute_tool(&self, tool_name: &str) -> Result<()> {
        // TODO: Check against permission matrix
        Ok(())
    }
}
```

**Deliverable:** Type definitions compile, ready for expansion

### A.2 Create Error Module

**File:** `/crates/cli/src/error.rs`

```rust
use std::fmt;

#[derive(Debug)]
pub enum CliError {
    Settings(String),
    Hooks(String),
    Plugins(String),
    Checkpoint(String),
    PermissionDenied(String),
    ToolError { tool: String, error: String },
    ApiError(String),
}

impl CliError {
    pub fn exit_code(&self) -> i32 {
        match self {
            CliError::Settings(_) => 10,
            CliError::Hooks(_) => 11,
            CliError::Plugins(_) => 11,
            CliError::Checkpoint(_) => 12,
            CliError::PermissionDenied(_) => 20,
            CliError::ToolError { .. } => 21,
            CliError::ApiError(_) => 22,
        }
    }

    pub fn should_checkpoint(&self) -> bool {
        !matches!(self, CliError::Settings(_) | CliError::Plugins(_))
    }
}

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Fmt) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl std::error::Error for CliError {}
```

**Deliverable:** Error type with exit code strategy

### A.3 Update lib.rs Exports

**File:** `/crates/cli/src/lib.rs`

```rust
pub mod checkpoint;
pub mod commands;
pub mod context;  // NEW
pub mod error;    // NEW
pub mod hooks;
pub mod interactive;
pub mod plugins;
pub mod settings;

pub use context::ExecutionContext;
pub use error::CliError;
```

**Deliverable:** Modules accessible from main.rs

### A.4 Create Stub main.rs

**File:** `/crates/cli/src/main.rs` (replaced)

```rust
use anyhow::Result;
use clap::Parser;
use crate::context::ExecutionContext;
use crate::error::CliError;

#[derive(Parser)]
#[command(name = "claude-code")]
struct Cli {
    #[arg(short, long)]
    debug: bool,

    #[arg(long)]
    session: Option<String>,

    #[arg(long)]
    checkpoint: Option<String>,

    // TODO: Add remaining fields
}

#[tokio::main]
async fn main() {
    if let Err(e) = run_cli().await {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

async fn run_cli() -> Result<()> {
    let cli = Cli::parse();

    // Phase 1: Startup
    tracing_subscriber::fmt().init();
    eprintln!("TODO: Implement startup phase");

    // Phase 2: Mode dispatch
    eprintln!("TODO: Implement mode dispatch");

    Ok(())
}
```

**Deliverable:** Compiles and runs (prints TODO)

### A.5 Add Test Infrastructure

**File:** `/tests/integration_main.rs`

```rust
#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    fn setup_test_env() -> PathBuf {
        // Create temp directory with .claude structure
        todo!()
    }

    #[tokio::test]
    async fn test_cli_help() {
        // Basic smoke test
        todo!()
    }
}
```

**Deliverable:** Test harness ready

## Phase B: Initialization Systems (2-3 days)

### B.1 Implement Settings Loading in main.rs

**Code Location:** `main.rs::startup_phase()`

```rust
async fn startup_phase(cli: &Cli) -> Result<ExecutionContext> {
    // Step 1: Initialize logging early
    let log_level = if cli.debug { "debug" } else { "info" };
    tracing_subscriber::fmt()
        .with_env_filter(log_level)
        .init();

    // Step 2: Load settings hierarchy
    let loader = crate::settings::SettingsLoader::new();
    let hierarchy = loader.load_hierarchy()
        .map_err(|e| anyhow::anyhow!(CliError::Settings(e.to_string())))?;

    let settings = hierarchy.merge();
    settings.validate()
        .map_err(|e| anyhow::anyhow!(CliError::Settings(e.to_string())))?;

    // Step 3: Create ExecutionContext (stub)
    let ctx = ExecutionContext {
        settings,
        command: todo!(),
        // ... other fields
    };

    Ok(ctx)
}
```

**Deliverable:** Settings load from hierarchy with proper error handling

### B.2 Implement Hooks System Initialization

**Code Location:** `main.rs::startup_phase()`

```rust
// After settings loaded:
let mut hooks_system = crate::hooks::HooksSystem::new();

// Load from .claude/hooks.yaml if exists
if let Some(hooks_path) = settings.hooks_file_path() {
    if hooks_path.exists() {
        hooks_system.load_from_file(hooks_path.to_str().unwrap()).await
            .map_err(|e| anyhow::anyhow!(CliError::Hooks(e.to_string())))?;
    }
}

tracing::debug!("Hooks system initialized");
```

**Deliverable:** Hooks system loads and doesn't crash on missing file

### B.3 Implement Plugin Loading

**Code Location:** `main.rs::startup_phase()`

```rust
// After hooks initialized:
let mut plugin_loader = crate::plugins::PluginLoader::new();

// Discover plugins
let discovery = crate::plugins::PluginDiscovery::new(
    crate::plugins::DEFAULT_PLUGINS_DIR
);

let plugins = discovery.discover_all()
    .map_err(|e| anyhow::anyhow!(CliError::Plugins(e.to_string())))?;

for plugin in plugins {
    plugin_loader.register(plugin);
}

// Load enabled plugins
for plugin_metadata in plugin_loader.list_plugins() {
    if plugin_metadata.enabled {
        plugin_loader.load(&plugin_metadata.id)
            .map_err(|e| anyhow::anyhow!(CliError::Plugins(e.to_string())))?;
    }
}

tracing::debug!("Plugin system initialized with {} plugins",
    plugin_loader.list_plugins().len());
```

**Deliverable:** Plugins discover and load gracefully

### B.4 Implement Checkpoint System Initialization

**Code Location:** `main.rs::startup_phase()`

```rust
// Load checkpoint system
let checkpoint_saver = crate::checkpoint::SessionSaver::default()
    .map_err(|e| anyhow::anyhow!(CliError::Checkpoint(e.to_string())))?;

let checkpoint_loader = crate::checkpoint::SessionLoader::default()
    .map_err(|e| anyhow::anyhow!(CliError::Checkpoint(e.to_string())))?;

// Check for session recovery
let session = if let Some(session_id) = &cli.session {
    let recovered = checkpoint_loader.resume_session(
        session_id,
        50 // Max messages to keep
    ).map_err(|e| anyhow::anyhow!(CliError::Checkpoint(e.to_string())))?;

    tracing::info!("Resumed session: {}", session_id);
    Some(recovered)
} else {
    None
};

// If checkpoint specified, restore from it
if let Some(checkpoint_id) = &cli.checkpoint {
    if let Some(ref mut session) = session {
        session.restore_checkpoint(
            checkpoint_id,
            crate::checkpoint::RestoreScope::Both
        ).map_err(|e| anyhow::anyhow!(CliError::Checkpoint(e.to_string())))?;

        tracing::info!("Restored checkpoint: {}", checkpoint_id);
    }
}
```

**Deliverable:** Session recovery works when flags provided

### B.5 Implement Permission Matrix

**Code Location:** `context.rs`

```rust
pub struct PermissionMatrix {
    permissions: std::collections::HashMap<String, ToolPermission>,
}

impl PermissionMatrix {
    pub fn from_settings(settings: &EffectiveSettings) -> Self {
        let permissions = settings.permissions.clone(); // From settings
        Self { permissions }
    }

    pub fn check(&self, tool_name: &str) -> Result<(), String> {
        match self.permissions.get(tool_name) {
            Some(perm) => {
                if perm.mode == PermissionMode::Allow {
                    Ok(())
                } else {
                    Err(format!("Tool {} not permitted", tool_name))
                }
            }
            None => Ok(()), // Default allow if not specified
        }
    }
}
```

**Deliverable:** Permission checking works

### B.6 Implement SessionStart Hook Execution

**Code Location:** `main.rs::startup_phase()` (end)

```rust
// Execute SessionStart hook
let hook_context = crate::hooks::HookContext {
    event: crate::hooks::HookEvent::SessionStart,
    settings: settings.clone(),
    session: session.clone(),
    // ... other fields
};

match hooks_system.execute_hooks(
    crate::hooks::HookEvent::SessionStart,
    &hook_context,
).await {
    Ok(results) => {
        for result in results {
            if !result.success {
                tracing::warn!("SessionStart hook failed: {:?}", result);
            }
        }
    }
    Err(e) => {
        tracing::warn!("SessionStart hook error (non-fatal): {}", e);
    }
}

tracing::info!("Startup phase complete");
```

**Deliverable:** SessionStart hooks execute with proper error handling

### B.7 Complete ExecutionContext Creation

**Code Location:** `context.rs`

```rust
impl ExecutionContext {
    pub async fn new(
        cli: Cli,
        settings: EffectiveSettings,
        // ... other params from startup
    ) -> Result<Self> {
        Ok(ExecutionContext {
            settings,
            permissions: PermissionMatrix::from_settings(&settings),
            command: cli.command.clone(),
            hooks: hooks_system,
            checkpoint_saver,
            checkpoint_loader,
            plugins: plugin_loader,
            slash_commands: commands_registry,
            session,
            session_id: uuid::Uuid::new_v4().to_string(),
            session_created_at: Instant::now(),
            // ... other fields with defaults
        })
    }
}
```

**Deliverable:** ExecutionContext fully initialized from all systems

### B.8 Add Signal Handling

**Code Location:** `main.rs`

```rust
use signal_hook_tokio::Signals;
use signal_hook::consts::signal::*;

async fn setup_signal_handlers() -> Result<tokio::sync::mpsc::Receiver<()>> {
    let (tx, rx) = tokio::sync::mpsc::channel(1);

    tokio::spawn(async move {
        let mut signals = Signals::new(&[SIGINT, SIGTERM])?;
        if let Some(_sig) = signals.next().await {
            tracing::info!("Shutdown signal received");
            let _ = tx.send(()).await;
        }
        Ok::<_, anyhow::Error>(())
    });

    Ok(rx)
}
```

**Deliverable:** Signals are handled gracefully

## Phase C: Mode Dispatch (2-3 days)

### C.1 Update Cli Enum

**Code Location:** `main.rs`

```rust
#[derive(Parser)]
struct Cli {
    // ... existing ...

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Interactive chat session
    Chat {
        #[arg(long)]
        model: Option<String>,
    },

    /// Execute a tool
    Tool {
        /// Tool name (bash, read, write, edit, glob, grep, etc.)
        name: String,

        /// Parameters as JSON
        #[arg(long)]
        params: String,
    },

    /// Execute a slash command
    Command {
        /// Command name
        name: String,

        /// Arguments
        args: Vec<String>,
    },

    /// Execute plugin command
    Plugin {
        /// Plugin ID
        plugin: String,

        /// Command name
        command: String,

        /// Arguments
        #[arg(long)]
        params: Option<String>,
    },

    /// Execute agent task
    Agent {
        /// Task definition file
        #[arg(long)]
        task: String,
    },
}
```

**Deliverable:** CLI enum fully defined with all modes

### C.2 Implement Mode Dispatch Routing

**Code Location:** `main.rs::mode_dispatch()`

```rust
async fn mode_dispatch(ctx: ExecutionContext) -> Result<ExitCode> {
    let exit_code = match &ctx.command {
        Commands::Chat { .. } => {
            tracing::info!("Starting chat mode");
            chat_mode(ctx).await?;
            ExitCode::SUCCESS
        }

        Commands::Tool { name, params } => {
            tracing::info!("Executing tool: {}", name);
            tool_mode(ctx, name.clone(), params.clone()).await?;
            ExitCode::SUCCESS
        }

        Commands::Command { name, args } => {
            tracing::info!("Executing command: {}", name);
            command_mode(ctx, name.clone(), args.clone()).await?;
            ExitCode::SUCCESS
        }

        Commands::Plugin { plugin, command, params } => {
            tracing::info!("Executing plugin command: {}/{}", plugin, command);
            plugin_mode(ctx, plugin.clone(), command.clone(), params.clone()).await?;
            ExitCode::SUCCESS
        }

        Commands::Agent { task } => {
            tracing::info!("Executing agent task");
            agent_mode(ctx, task.clone()).await?;
            ExitCode::SUCCESS
        }
    };

    Ok(exit_code)
}
```

**Deliverable:** All modes dispatch correctly

### C.3 Implement Chat Mode

**Code Location:** `main.rs::chat_mode()`

```rust
async fn chat_mode(mut ctx: ExecutionContext) -> Result<()> {
    // Delegate to existing interactive module
    let interactive_ctx = ConvertedContext {
        settings: ctx.settings.clone(),
        hooks: ctx.hooks.clone(),
        // ... convert ExecutionContext to what interactive needs
    };

    crate::interactive::run_interactive_with_context(interactive_ctx).await?;

    // Save final checkpoint
    if let Some(session) = &ctx.session {
        ctx.checkpoint_saver.save_session(session)
            .map_err(|e| anyhow::anyhow!(CliError::Checkpoint(e.to_string())))?;
    }

    Ok(())
}
```

**Deliverable:** Chat mode works (may require adapting interactive module)

### C.4 Implement Tool Mode

**Code Location:** `main.rs::tool_mode()`

```rust
async fn tool_mode(
    ctx: ExecutionContext,
    tool_name: String,
    params_json: String,
) -> Result<()> {
    // Check permission
    ctx.permissions.check(&tool_name)
        .map_err(|e| anyhow::anyhow!(CliError::PermissionDenied(e)))?;

    // Parse parameters
    let params: serde_json::Value = serde_json::from_str(&params_json)
        .map_err(|e| anyhow::anyhow!(CliError::ToolError {
            tool: tool_name.clone(),
            error: e.to_string(),
        }))?;

    // Execute PreToolUse hook
    // (non-blocking, log errors only)

    // Create tool context
    let tool_ctx = ToolContext::default();

    // Execute tool based on name
    // (dispatch to correct tool's execute_stream method)

    // Stream results
    // (print each ToolEvent as JSON)

    // Execute PostToolUse hook
    // (non-blocking)

    // Checkpoint if edit tool
    if matches!(tool_name.as_str(), "write" | "edit") {
        if let Some(session) = &ctx.session {
            ctx.checkpoint_saver.save_session(session)
                .map_err(|e| anyhow::anyhow!(CliError::Checkpoint(e.to_string())))?;
        }
    }

    Ok(())
}
```

**Deliverable:** Tool mode executes with hooks and checkpoints

### C.5 Implement Command Mode

**Code Location:** `main.rs::command_mode()`

```rust
async fn command_mode(
    ctx: ExecutionContext,
    cmd_name: String,
    args: Vec<String>,
) -> Result<()> {
    // Load command from slash_commands registry
    let cmd = ctx.slash_commands.get_command(&cmd_name)
        .ok_or_else(|| anyhow::anyhow!("Command not found: {}", cmd_name))?;

    // Expand template with args
    let expanded = expand_command_template(&cmd.template, &args)?;

    // Execute via Agent tool
    // (delegate to AgentTool)

    Ok(())
}
```

**Deliverable:** Slash commands execute

### C.6 Implement Plugin Mode

**Code Location:** `main.rs::plugin_mode()`

```rust
async fn plugin_mode(
    ctx: ExecutionContext,
    plugin_id: String,
    command: String,
    params: Option<String>,
) -> Result<()> {
    // Get plugin metadata
    let metadata = ctx.plugins.get_metadata(&plugin_id)
        .ok_or_else(|| anyhow::anyhow!("Plugin not found: {}", plugin_id))?;

    // Verify command exists in manifest
    let _cmd_def = metadata.manifest.commands.iter()
        .find(|c| c.name == command)
        .ok_or_else(|| anyhow::anyhow!("Command not found in plugin"))?;

    // Check permissions
    // (tool-level check for plugin execution)

    // Parse params if provided
    let params_json = if let Some(p) = params {
        serde_json::from_str(&p)?
    } else {
        serde_json::json!({})
    };

    // Execute plugin command
    let mut executor = crate::plugins::PluginExecutor::new();
    executor.register(metadata);
    let result = executor.execute_command(&plugin_id, &command, params_json)
        .map_err(|e| anyhow::anyhow!(CliError::Plugins(e.to_string())))?;

    // Output result as JSON
    println!("{}", serde_json::to_string_pretty(&result)?);

    Ok(())
}
```

**Deliverable:** Plugin commands execute

### C.7 Implement Agent Mode

**Code Location:** `main.rs::agent_mode()`

```rust
async fn agent_mode(
    ctx: ExecutionContext,
    task_path: String,
) -> Result<()> {
    // Read task definition file
    let task_content = std::fs::read_to_string(&task_path)
        .map_err(|e| anyhow::anyhow!("Failed to read task: {}", e))?;

    // Parse as YAML/JSON
    let task_def: serde_json::Value = serde_yaml::from_str(&task_content)
        .or_else(|_| serde_json::from_str(&task_content))
        .map_err(|e| anyhow::anyhow!("Invalid task definition: {}", e))?;

    // Create AgentTool with all tools available
    let agent_tool = claude_code_tools::AgentTool;

    // Execute via tool interface
    // (stream results)

    Ok(())
}
```

**Deliverable:** Agent tasks run

### C.8 Add Graceful Error Handling to Dispatch

**Code Location:** `main.rs::run_cli()` (updated)

```rust
async fn run_cli() -> Result<(), ExitCode> {
    let result = async {
        let ctx = startup_phase().await
            .map_err(|e| {
                eprintln!("Startup error: {}", e);
                e
            })?;

        mode_dispatch(ctx).await
    }.await;

    match result {
        Ok(code) => Ok(code),
        Err(e) => {
            eprintln!("Error: {}", e);
            // TODO: Try to checkpoint here
            Err(ExitCode::FAILURE)
        }
    }
}
```

**Deliverable:** All errors handled with proper exit codes

## Phase D: Lifecycle Management (1-2 days)

### D.1 Implement PreToolUse Hook Execution

**Code Location:** New function `execute_pre_tool_hooks()`

```rust
async fn execute_pre_tool_hooks(
    hooks: &HooksSystem,
    tool_name: &str,
    params: &serde_json::Value,
) -> Result<()> {
    let context = HookContext {
        event: HookEvent::PreToolUse,
        tool_name: Some(tool_name.to_string()),
        params: Some(params.clone()),
        // ... other fields
    };

    match hooks.execute_hooks(HookEvent::PreToolUse, &context).await {
        Ok(results) => {
            for result in results {
                if !result.success {
                    tracing::warn!("PreToolUse hook failed: {:?}", result.error);
                }
            }
            Ok(())
        }
        Err(e) => {
            tracing::warn!("PreToolUse hook error: {}", e);
            Ok(()) // Non-blocking
        }
    }
}
```

**Deliverable:** Pre-tool hooks execute with timeout

### D.2 Implement PostToolUse Hook Execution

**Code Location:** New function `execute_post_tool_hooks()`

```rust
async fn execute_post_tool_hooks(
    hooks: &HooksSystem,
    tool_name: &str,
    result: &serde_json::Value,
    exit_code: Option<i32>,
    duration_ms: u64,
) -> Result<()> {
    let context = HookContext {
        event: HookEvent::PostToolUse,
        tool_name: Some(tool_name.to_string()),
        result: Some(result.clone()),
        exit_code,
        duration_ms: Some(duration_ms),
        // ... other fields
    };

    match hooks.execute_hooks(HookEvent::PostToolUse, &context).await {
        Ok(results) => {
            for result in results {
                if !result.success {
                    tracing::warn!("PostToolUse hook failed: {:?}", result.error);
                }
            }
            Ok(())
        }
        Err(e) => {
            tracing::warn!("PostToolUse hook error: {}", e);
            Ok(()) // Non-blocking
        }
    }
}
```

**Deliverable:** Post-tool hooks execute

### D.3 Implement Checkpoint Scheduling Logic

**Code Location:** `ExecutionContext` methods

```rust
impl ExecutionContext {
    pub fn should_checkpoint(&self) -> bool {
        // Checkpoint every 5 minutes or every 10 turns
        let elapsed = self.session_created_at.elapsed();
        elapsed.as_secs() > 300 ||
            self.checkpoint_count > 10
    }

    pub fn checkpoint_completed(&mut self) {
        self.last_checkpoint_time = Instant::now();
        self.checkpoint_count = 0;
    }
}
```

**Deliverable:** Checkpoint scheduling logic

### D.4 Integrate Checkpoints into Tool Mode

**Code Location:** `main.rs::tool_mode()` (updated)

```rust
async fn tool_mode(mut ctx: ExecutionContext, tool_name: String, params_json: String) -> Result<()> {
    // ... existing code ...

    // After tool execution completes
    ctx.tool_start_time = None;
    ctx.error_count = 0;

    // Check if should checkpoint
    if ctx.should_checkpoint() {
        if let Some(session) = &ctx.session {
            match ctx.checkpoint_saver.save_session(session).await {
                Ok(_) => {
                    tracing::debug!("Checkpoint saved automatically");
                    ctx.checkpoint_completed();
                }
                Err(e) => {
                    tracing::warn!("Failed to save checkpoint: {}", e);
                }
            }
        }
    }

    Ok(())
}
```

**Deliverable:** Checkpoints save periodically

## Phase E: Shutdown Sequence (1-2 days)

### E.1 Create Shutdown Guard

**Code Location:** `context.rs`

```rust
pub struct ShutdownGuard {
    ctx: Option<ExecutionContext>,
    error: Option<anyhow::Error>,
}

impl Drop for ShutdownGuard {
    fn drop(&mut self) {
        // Run blocking shutdown operations
        // This runs on panic or normal exit
        if let Some(mut ctx) = self.ctx.take() {
            // Synchronous shutdown for tokio runtime shutdown
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let _ = shutdown_phase(&mut ctx, self.error.as_ref()).await;
            });
        }
    }
}
```

**Deliverable:** Shutdown runs even on panic

### E.2 Implement SessionEnd Hook Execution

**Code Location:** New function `execute_session_end_hooks()`

```rust
async fn execute_session_end_hooks(
    hooks: &HooksSystem,
    exit_reason: &str,
    duration_ms: u64,
) -> Result<()> {
    let context = HookContext {
        event: HookEvent::SessionEnd,
        // ... fields for summary
    };

    // Add timeout
    let result = tokio::time::timeout(
        Duration::from_secs(5),
        hooks.execute_hooks(HookEvent::SessionEnd, &context)
    ).await;

    match result {
        Ok(Ok(results)) => {
            for result in results {
                if !result.success {
                    tracing::warn!("SessionEnd hook failed: {:?}", result.error);
                }
            }
            Ok(())
        }
        Ok(Err(e)) => {
            tracing::warn!("SessionEnd hook error: {}", e);
            Ok(())
        }
        Err(_) => {
            tracing::warn!("SessionEnd hooks timed out");
            Ok(())
        }
    }
}
```

**Deliverable:** SessionEnd hooks with timeout

### E.3 Implement Final Checkpoint Save

**Code Location:** `shutdown_phase()`

```rust
async fn shutdown_phase(ctx: &mut ExecutionContext, error: Option<&anyhow::Error>) -> Result<()> {
    tracing::debug!("Entering shutdown phase");

    // Execute SessionEnd hooks
    let exit_reason = if error.is_some() {
        "error"
    } else {
        "normal"
    };

    execute_session_end_hooks(&ctx.hooks, exit_reason,
        ctx.session_created_at.elapsed().as_millis() as u64
    ).await?;

    // Save final checkpoint
    if let Some(session) = &ctx.session {
        match ctx.checkpoint_saver.save_session(session).await {
            Ok(_) => {
                tracing::info!("Final checkpoint saved");
            }
            Err(e) => {
                tracing::warn!("Failed to save final checkpoint: {}", e);
            }
        }
    }

    // Cleanup
    tracing::debug!("Shutdown phase complete");
    Ok(())
}
```

**Deliverable:** Final checkpoint saves on exit

### E.4 Integrate Shutdown into Main

**Code Location:** `main.rs::run_cli()` (updated)

```rust
#[tokio::main]
async fn main() -> ExitCode {
    let result = async {
        // Parse args early
        let cli = Cli::parse();

        // Create shutdown guard
        let ctx = startup_phase(&cli).await?;
        let mut guard = ShutdownGuard { ctx: Some(ctx), error: None };

        // Run mode dispatch
        if let Some(ref mut ctx) = &mut guard.ctx {
            match mode_dispatch(ctx.clone()).await {
                Ok(code) => return Ok(code),
                Err(e) => {
                    guard.error = Some(e.clone());
                    Err(e)
                }
            }
        }

        Ok(ExitCode::SUCCESS)
    }.await;

    match result {
        Ok(code) => code,
        Err(e) => {
            eprintln!("Fatal error: {}", e);
            ExitCode::FAILURE
        }
    }
}
```

**Deliverable:** Graceful shutdown with resource cleanup

### E.5 Implement Proper Exit Codes

**Code Location:** `main.rs` (final)

```rust
fn error_to_exit_code(err: &anyhow::Error) -> ExitCode {
    // Try to downcast to CliError
    if let Some(cli_err) = err.downcast_ref::<CliError>() {
        let code = cli_err.exit_code();
        ExitCode::from(code as u8)
    } else {
        ExitCode::FAILURE
    }
}
```

**Deliverable:** Exit codes match specification

### E.6 Add Comprehensive Tests

**File:** `/tests/integration_main.rs`

All tests from Phase A.5 specification

**Deliverable:** Full test coverage

## Acceptance Criteria

### Phase A Acceptance
- [ ] All modules compile without errors
- [ ] lib.rs exports new modules
- [ ] main.rs runs and prints TODO messages
- [ ] Types are well-defined

### Phase B Acceptance
- [ ] Settings load from hierarchy
- [ ] Hooks system initializes
- [ ] Plugins discover and load
- [ ] Checkpoints resume sessions
- [ ] SessionStart hooks execute
- [ ] Signal handlers installed

### Phase C Acceptance
- [ ] All 5 commands dispatch correctly
- [ ] Chat mode delegates to interactive
- [ ] Tool mode executes single tools
- [ ] Command mode runs slash commands
- [ ] Plugin mode executes plugin commands
- [ ] Agent mode runs agent tasks
- [ ] Error exit codes correct

### Phase D Acceptance
- [ ] PreToolUse hooks execute
- [ ] PostToolUse hooks execute
- [ ] Checkpoints save periodically
- [ ] No duplicate hooks
- [ ] Hook timeouts work
- [ ] Permission matrix enforced

### Phase E Acceptance
- [ ] SessionEnd hooks execute
- [ ] Final checkpoint saved on exit
- [ ] All resources cleaned up
- [ ] Exit codes correct
- [ ] Graceful shutdown on Ctrl+C
- [ ] Integration tests pass

## Risk Mitigation

### High Risk: Integration with Existing Modules
**Risk:** Module interfaces may have changed or be incompatible
**Mitigation:**
- Verify interfaces match specification
- Add integration tests early
- Use feature flags if needed

### Medium Risk: Signal Handling Complexity
**Risk:** Signal handling may miss edge cases
**Mitigation:**
- Test SIGINT and SIGTERM
- Test nested signal handling
- Add timeout for shutdown

### Medium Risk: Hook Execution Reliability
**Risk:** Hooks may panic or hang
**Mitigation:**
- Wrap each hook in catch_unwind
- Add 5-second timeout
- Log failures clearly

### Low Risk: Checkpoint IO Performance
**Risk:** Too many checkpoints slow down CLI
**Mitigation:**
- Implement checkpoint scheduling
- Make saves async
- Monitor performance

## Timeline Estimate

- Phase A: 1-2 days (foundation, types)
- Phase B: 2-3 days (initialization, systems)
- Phase C: 2-3 days (mode dispatch, routing)
- Phase D: 1-2 days (lifecycle, hooks)
- Phase E: 1-2 days (shutdown, cleanup)

**Total: 7-12 days of focused development**

## Success Metrics

1. All integration tests pass
2. Single `cargo build` succeeds
3. `claude-code chat` runs interactively
4. `claude-code tool bash --params '{...}'` executes tools
5. Session recovery works with checkpoints
6. All hooks execute at correct lifecycle points
7. Graceful shutdown on Ctrl+C
8. Proper exit codes for all error cases
