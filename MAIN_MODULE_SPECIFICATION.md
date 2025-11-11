# Module Specification: New main.rs Architecture

## Module Organization

```
crates/cli/src/
├── main.rs              (NEW - 600-800 lines)
│   ├── Phase 1: Startup
│   ├── Phase 2: Mode Dispatch
│   ├── Phase 3: Lifecycle
│   └── Phase 4: Shutdown
│
├── context.rs           (NEW - 200 lines)
│   └── ExecutionContext struct
│
├── error.rs             (NEW - 150 lines)
│   └── CliError enum + strategies
│
└── [existing modules unchanged]
```

## Module: main.rs

### Purpose
Single unified CLI entry point that orchestrates all systems (settings, hooks, plugins, checkpoints, interactive mode, tools).

### Contract

**Inputs:**
- CLI arguments (from clap::Parser)
- Environment variables (CLAUDE_*, standard unix)
- Filesystem (`.claude/` config, plugins, hooks, checkpoints)
- API (via claude-code-core client)

**Outputs:**
- Exit code (0, 1, 2, 10-22, 130, 143)
- Stdout/stderr (formatted per mode)
- Checkpoints saved to `.claude/sessions/`
- Log output per settings configuration

**Side Effects:**
- Executes hooks at lifecycle events
- Creates/modifies files via tools
- Saves checkpoints
- Handles signals
- Streams responses from API

### Dependencies

```toml
[dependencies]
# Core CLI
clap = { workspace = true }
tokio = { workspace = true }

# Existing modules
claude_code_tools = { path = "../tools" }
claude_code_core = { path = "../core" }

# This crate's modules
crate::checkpoint
crate::hooks
crate::interactive
crate::plugins
crate::settings
crate::commands

# Utilities
anyhow = { workspace = true }
tracing = { workspace = true }
tracing-subscriber = { workspace = true }
serde_json = { workspace = true }
futures = { workspace = true }
signal-hook-tokio = "0.3"
```

### Implementation Notes

1. **No global state** - Everything in ExecutionContext
2. **Composable error handling** - Errors escalate with proper context
3. **Hook non-blocking** - Hook failures don't crash execution
4. **Checkpoint on error** - Always save state before fatal exit
5. **Single session per invocation** - One process = one session
6. **Streaming first** - All tool outputs stream, never buffer fully

### Structure

```rust
// ============================================================================
// MAIN ENTRY POINT
// ============================================================================

#[tokio::main]
async fn main() -> Result<()> {
    run_cli().await
}

async fn run_cli() -> Result<()> {
    // Phase 1: Startup
    // Phase 2: Mode Dispatch
    // Phase 3: Lifecycle (within handlers)
    // Phase 4: Shutdown (via cleanup guard)
}

// ============================================================================
// PHASE 1: STARTUP SEQUENCE
// ============================================================================

async fn startup_phase() -> Result<ExecutionContext> {
    // 1.1 Parse arguments
    // 1.2 Load settings
    // 1.3 Initialize logging
    // 1.4 Initialize hooks
    // 1.5 Initialize plugins
    // 1.6 Check checkpoints
    // 1.7 Execute SessionStart hooks
}

// ============================================================================
// PHASE 2: MODE DISPATCH
// ============================================================================

async fn mode_dispatch(ctx: ExecutionContext) -> Result<()> {
    match ctx.command {
        Commands::Chat => chat_mode(ctx).await,
        Commands::Tool { .. } => tool_mode(ctx).await,
        Commands::Command { .. } => command_mode(ctx).await,
        Commands::Plugin { .. } => plugin_mode(ctx).await,
        Commands::Agent { .. } => agent_mode(ctx).await,
    }
}

// ============================================================================
// PHASE 4: SHUTDOWN SEQUENCE (via RAII guard)
// ============================================================================

struct ShutdownGuard {
    ctx: Option<ExecutionContext>,
}

impl Drop for ShutdownGuard {
    fn drop(&mut self) {
        // Execute SessionEnd hooks
        // Save final checkpoint
        // Cleanup resources
    }
}
```

### Key Functions

#### `async fn startup_phase(cli: &Cli) -> Result<ExecutionContext>`

**Responsibility:** Initialize all systems in correct sequence

**Steps:**
1. Initialize logging (early, before anything else)
2. Load settings hierarchy from SettingsLoader
3. Validate merged settings
4. Initialize HooksSystem from `.claude/hooks.yaml`
5. Initialize PluginLoader with discovery
6. Check for checkpoint/session recovery
7. Create ExecutionContext
8. Execute SessionStart hooks with context
9. Return ExecutionContext

**Error Handling:**
- Settings error: Fatal, exit 10
- Plugin error: Warn and continue, track in context
- Hook error: Log warn, non-fatal
- Checkpoint error: Warn, no checkpoint recovery

**Returns:** `ExecutionContext` fully initialized

#### `async fn mode_dispatch(mut ctx: ExecutionContext) -> Result<()>`

**Responsibility:** Route to appropriate handler based on CLI command

**Dispatch Table:**

| Command | Handler | Exit Codes |
|---------|---------|-----------|
| Chat | `interactive::run_interactive()` | 0, 1, 130 |
| Tool | `execute_tool()` | 0, 1, 20, 21, 22 |
| Command | `execute_slash_command()` | 0, 1, 2 |
| Plugin | `execute_plugin()` | 0, 1, 11, 20 |
| Agent | `execute_agent()` | 0, 1, 21, 22 |

**Each handler receives:**
- ExecutionContext (with settings, hooks, plugins, session)
- Specific parameters from CLI
- Returns Result<ExitCode>

#### `async fn chat_mode(ctx: ExecutionContext) -> Result<()>`

**Responsibility:** Interactive REPL session with Claude

**Flow:**
```
1. Resume session if --session specified
2. Enter REPL loop
   a. Read user input
   b. Execute UserPromptSubmit hooks
   c. Stream response from Claude (handles tool calls)
   d. Execute PostToolUse hooks if tools used
   e. Checkpoint every N turns
3. On exit: save final checkpoint
4. Execute SessionEnd hooks
```

**Integration Points:**
- Hooks: SessionStart, UserPromptSubmit, PostToolUse, SessionEnd
- Checkpoint: Resume + periodic saves
- Settings: Model selection, timeout, permissions

#### `async fn tool_mode(ctx: ExecutionContext, tool_name: String, params: String) -> Result<()>`

**Responsibility:** Execute single tool with proper hooks and permissions

**Flow:**
```
1. Parse parameters from JSON
2. Check permissions (PermissionMatrix)
3. Execute PreToolUse hook (can block)
4. Execute tool with streaming
5. Execute PostToolUse hook
6. Checkpoint if edit tool
7. Output result as JSON
```

**Permission Check:**
```rust
match ctx.permissions.check(&tool_name) {
    PermissionMode::Allow => {},
    PermissionMode::Deny => return Err(permission_denied()),
    PermissionMode::Ask => {
        ask_user_permission(&tool_name).await?
    }
}
```

**Exit Codes:**
- 0: Success
- 1: Generic error
- 20: Permission denied
- 21: Tool execution error
- 22: Parameter validation error

#### `async fn command_mode(ctx: ExecutionContext, cmd_name: String, args: Vec<String>) -> Result<()>`

**Responsibility:** Execute slash command

**Flow:**
```
1. Discover commands from .claude/commands/
2. Load command definition
3. Parse args and expand template
4. Execute via Agent tool
5. Return result
```

#### `async fn plugin_mode(ctx: ExecutionContext, plugin_id: String, cmd_name: String) -> Result<()>`

**Responsibility:** Execute plugin command

**Flow:**
```
1. Verify plugin is loaded
2. Find command in manifest
3. Check permissions
4. Execute command
5. Return JSON result
```

#### `async fn agent_mode(ctx: ExecutionContext, task_def: String) -> Result<()>`

**Responsibility:** Execute multi-step agent task

**Flow:**
```
1. Parse task definition
2. Initialize AgentTool with all tools
3. Run agent loop with streaming
4. Checkpoint after major actions
5. Summarize and return result
```

#### `async fn shutdown_phase(ctx: ExecutionContext, error: Option<&anyhow::Error>) -> Result<()>`

**Responsibility:** Clean shutdown with state preservation

**Steps:**
1. Execute SessionEnd hooks
   - Provide exit reason
   - Allow hooks to upload telemetry
   - Timeout after 5 seconds
2. Save final checkpoint
   - Mark as "paused" if graceful
   - Include exit reason
3. Cleanup resources
   - Flush logs
   - Kill background processes
   - Close connections

### Signal Handling

**Setup in startup:**
```rust
let (shutdown_tx, mut shutdown_rx) = tokio::sync::mpsc::channel(1);

// Register signal handlers
tokio::spawn(async move {
    let mut sigint = signal::unix::signal(signal::unix::SignalKind::interrupt())?;
    sigint.recv().await;
    let _ = shutdown_tx.send(()).await;
});

tokio::spawn(async move {
    let mut sigterm = signal::unix::signal(signal::unix::SignalKind::terminate())?;
    sigterm.recv().await;
    let _ = shutdown_tx.send(()).await;
});
```

**During execution:**
```rust
tokio::select! {
    result = mode_dispatch(&mut ctx) => result,
    _ = shutdown_rx.recv() => {
        tracing::info!("Shutdown signal received");
        shutdown_phase(ctx, None).await?;
        std::process::exit(130); // SIGINT
    }
}
```

### Error Handling Strategy

**Error Type Hierarchy:**

```
CliError
├── StartupError (exit 10-12)
│   ├── SettingsError
│   ├── HooksError
│   ├── PluginError
│   └── CheckpointError
│
├── ExecutionError (exit 1, 20-22)
│   ├── PermissionDenied
│   ├── ToolError
│   └── ApiError
│
└── ShutdownError (exit 130-143)
    ├── SignalInterrupt
    └── Timeout
```

**Recovery Flow:**

```
Error Occurs
  │
  ├─ During Startup?
  │  └─ Cleanup & exit immediately (no checkpoint)
  │
  ├─ During Execution?
  │  ├─ Categorize (recoverable vs fatal)
  │  ├─ If fatal: save checkpoint, then exit
  │  └─ If recoverable: log and continue
  │
  └─ During Shutdown?
     └─ Log and ignore (already shutting down)
```

## Module: context.rs

### Purpose
Unified execution context passed through all phases and handlers.

### Contract

**Inputs:**
- CLI arguments
- Settings hierarchy
- Initialized systems

**Outputs:**
- ExecutionContext struct
- Used by all handlers

### Type Definition

```rust
pub struct ExecutionContext {
    // Configuration
    pub settings: EffectiveSettings,
    pub permissions: PermissionMatrix,

    // CLI command
    pub command: Commands,

    // Lifecycle systems
    pub hooks: HooksSystem,
    pub checkpoint_saver: SessionSaver,
    pub checkpoint_loader: SessionLoader,

    // Plugin ecosystem
    pub plugins: PluginLoader,
    pub slash_commands: SlashCommandRegistry,

    // Session state
    pub session: Option<Session>,
    pub session_id: String,
    pub session_created_at: Instant,

    // Execution state
    pub current_tool: Option<String>,
    pub tool_start_time: Option<Instant>,
    pub last_checkpoint_time: Instant,
    pub checkpoint_count: u32,
    pub error_count: u32,

    // Shutdown signal
    pub shutdown_signal: Option<tokio::sync::mpsc::Receiver<()>>,

    // Logging context
    pub log_level: tracing::Level,
}

impl ExecutionContext {
    /// Create new context
    pub async fn new(cli: Cli, settings: EffectiveSettings) -> Result<Self> { }

    /// Check if tool is permitted
    pub fn can_execute_tool(&self, tool_name: &str) -> Result<()> { }

    /// Start tool execution tracking
    pub fn start_tool(&mut self, tool_name: String) { }

    /// Finish tool execution tracking
    pub fn finish_tool(&mut self) -> Duration { }

    /// Should checkpoint now?
    pub fn should_checkpoint(&self) -> bool { }

    /// Mark checkpoint complete
    pub fn checkpoint_completed(&mut self) { }

    /// Increment error count
    pub fn record_error(&mut self) { }

    /// Check if should stop (for Stop hook)
    pub fn should_stop(&self) -> bool { }
}
```

## Module: error.rs

### Purpose
Unified error handling and exit code strategy.

### Contract

**Inputs:**
- Error context (during startup, execution, shutdown)
- Optional previous state

**Outputs:**
- CliError enum
- Formatted error messages
- Exit codes

### Type Definition

```rust
#[derive(Debug)]
pub enum CliError {
    // Startup errors (exit 10+)
    Settings(String),
    Hooks(String),
    Plugins(String),
    Checkpoint(String),

    // Execution errors (exit 20+)
    PermissionDenied(String),
    ToolError { tool: String, error: String },
    ApiError(String),
    InvalidParams(String),

    // Shutdown errors (exit 130+)
    Signal(String),
    Timeout(String),
}

impl CliError {
    pub fn exit_code(&self) -> i32 { }
    pub fn should_checkpoint(&self) -> bool { }
    pub fn message(&self) -> String { }
}
```

## Integration Points

### With Settings System
```
startup_phase()
  └─ load_settings_hierarchy()
     ├─ Read from disk
     ├─ Validate
     └─ Return EffectiveSettings
  └─ Extract: model, timeout, permissions, log_level
```

### With Hooks System
```
startup_phase()
  └─ load_hooks()
     ├─ Parse .claude/hooks.yaml
     ├─ Build registry
     └─ Return HooksSystem
  └─ Execute: SessionStart

mode_dispatch()
  └─ During execution:
     ├─ Execute: PreToolUse
     ├─ Execute: PostToolUse
     └─ Execute: Stop

shutdown_phase()
  └─ Execute: SessionEnd
```

### With Checkpoint System
```
startup_phase()
  └─ Check --session flag
     ├─ Load session metadata
     ├─ Restore file state if --checkpoint
     └─ Resume context

mode_dispatch()
  └─ Periodically:
     ├─ Check should_checkpoint()
     └─ Save session

shutdown_phase()
  └─ Save final checkpoint
     ├─ Mark as "paused"
     └─ Include exit reason
```

### With Plugins System
```
startup_phase()
  └─ discover_and_load_plugins()
     ├─ Scan filesystem
     ├─ Load manifests
     └─ Initialize enabled plugins

command_mode()
  └─ Execute plugin command
```

### With Interactive Mode
```
chat_mode()
  └─ interactive::run_interactive()
     ├─ Takes: ExecutionContext
     ├─ Returns: Result<()>
     └─ Executes: hooks, checkpoints internally
```

### With Tools Suite
```
tool_mode()
  └─ Execute specific tool
     ├─ Pre: check_permission()
     ├─ Pre: execute_hooks(PreToolUse)
     ├─ Execute: stream tool output
     ├─ Post: execute_hooks(PostToolUse)
     └─ Post: checkpoint if edit tool
```

## Testing Strategy

### Unit Tests (in main.rs)

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_startup_phase_success() { }

    #[tokio::test]
    async fn test_settings_loading_failure() { }

    #[tokio::test]
    async fn test_permission_check() { }

    #[tokio::test]
    async fn test_tool_execution_with_hooks() { }

    #[tokio::test]
    async fn test_checkpoint_on_error() { }

    #[tokio::test]
    async fn test_signal_handling() { }

    #[tokio::test]
    async fn test_hook_timeout() { }

    #[tokio::test]
    async fn test_exit_codes() { }
}
```

### Integration Tests (in tests/ directory)

```rust
// tests/integration_main.rs

#[tokio::test]
async fn test_full_chat_session() { }

#[tokio::test]
async fn test_tool_execution_pipeline() { }

#[tokio::test]
async fn test_session_recovery() { }

#[tokio::test]
async fn test_permission_enforcement() { }

#[tokio::test]
async fn test_plugin_loading_and_execution() { }

#[tokio::test]
async fn test_hook_lifecycle() { }

#[tokio::test]
async fn test_graceful_shutdown() { }
```

## Build & Deployment

### Cargo.toml Updates

```toml
[[bin]]
name = "claude-code"
path = "src/main.rs"

[dependencies]
signal-hook-tokio = "0.3"
# All existing deps
```

### Binary Invocation

```bash
# Interactive chat
cargo run --bin claude-code -- chat

# Execute single tool
cargo run --bin claude-code -- tool bash \
  --params '{"command": "ls -la"}'

# Execute slash command
cargo run --bin claude-code -- command mycommand arg1 arg2

# Execute plugin command
cargo run --bin claude-code -- plugin com.example.plugin mycommand

# Agent task
cargo run --bin claude-code -- agent --task-def task.yaml

# With options
cargo run --bin claude-code -- \
  --debug \
  --session my-session \
  --checkpoint checkpoint-123 \
  chat
```

## Key Design Decisions

### Decision 1: Single Process = Single Session
**Rationale:** Simplifies state management, clear lifecycle, no concurrency bugs
**Impact:** Each invocation is stateless, recovery via checkpoint

### Decision 2: Hooks Non-Blocking
**Rationale:** Hook failures shouldn't crash main flow
**Impact:** Hook errors logged but execution continues

### Decision 3: All I/O Streams
**Rationale:** Memory efficient, responsive, supports large operations
**Impact:** Never buffer full tool output, stream from start

### Decision 4: ExecutionContext Passed Everywhere
**Rationale:** Explicit dependency passing, no global state
**Impact:** Type-safe, easy to test, flexible initialization

### Decision 5: Checkpoint on Fatal Error
**Rationale:** Maximize recovery opportunities
**Impact:** Users can resume even after crashes

### Decision 6: Phase-Based Organization
**Rationale:** Clear separation of concerns, easy to reason about
**Impact:** Easy to test each phase independently
