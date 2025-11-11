# Builder Agent Handoff: Main CLI Integration

## Quick Summary

You are implementing the **unified CLI orchestration layer** for Claude Code. This integrates 8 complex systems (settings, hooks, plugins, checkpoints, interactive mode, slash commands, 14 tools, agent) into a single `main.rs` entry point with proper lifecycle management.

**Scope:** 2 new files + updates to existing modules
**Complexity:** High (orchestration of many moving parts)
**Risk:** Medium (integration points between systems)
**Timeline:** 7-12 days focused work

## What You're Building

```
User runs: claude-code --debug --session my-session chat
                ↓
           main.rs Startup
    ├─ Load settings hierarchy
    ├─ Initialize hooks system
    ├─ Discover & load plugins
    ├─ Check for session recovery
    ├─ Execute SessionStart hooks
                ↓
           Mode Dispatch
    ├─ Route to chat_mode()
                ↓
           Lifecycle Management
    ├─ Hooks: PreToolUse, PostToolUse
    ├─ Checkpoints: save periodically
    ├─ Permissions: enforce access control
                ↓
           Graceful Shutdown
    ├─ Execute SessionEnd hooks
    ├─ Save final checkpoint
    ├─ Cleanup resources
    └─ Exit with proper code
```

## Files to Create/Modify

### New Files

1. **`/crates/cli/src/context.rs`** (NEW)
   - `ExecutionContext` struct (200 lines)
   - `PermissionMatrix` struct
   - Execution state tracking

2. **`/crates/cli/src/error.rs`** (NEW)
   - `CliError` enum
   - Exit code mapping
   - Error recovery strategies

### Modified Files

1. **`/crates/cli/src/main.rs`** (REPLACE)
   - Complete rewrite (700-900 lines)
   - 4 main phases: Startup, Dispatch, Lifecycle, Shutdown

2. **`/crates/cli/src/lib.rs`** (UPDATE)
   - Export new modules

3. **`/Cargo.toml`** (UPDATE)
   - Add `signal-hook-tokio = "0.3"`

4. **`/tests/integration_main.rs`** (NEW)
   - Full integration test suite

## Key Concepts

### Execution Phases

**Phase 1: Startup Sequence**
- Parse args → Load settings → Init hooks → Discover plugins → Check checkpoints → SessionStart hooks
- Builds `ExecutionContext` that flows through entire execution
- If any fatal error: save checkpoint (if possible) and exit

**Phase 2: Mode Dispatch**
- Route based on CLI command: `chat | tool | command | plugin | agent`
- Each handler receives `ExecutionContext`
- Each handler returns `Result<()>` with implicit exit code

**Phase 3: Lifecycle Management**
- Executed within each mode handler
- Pre/post hooks around tool execution
- Periodic checkpoint saving
- Permission enforcement
- Error escalation to shutdown

**Phase 4: Shutdown Sequence**
- Execute SessionEnd hooks (with 5-second timeout)
- Save final checkpoint (mark as "paused")
- Cleanup resources (flush logs, kill processes)
- Exit with proper code

### Critical Design Principles

1. **No Global State** - Everything in `ExecutionContext` passed explicitly
2. **Hooks Non-Blocking** - Hook failures never crash main flow
3. **Graceful Degradation** - Systems optional (plugin missing? just continue)
4. **Checkpoint on Error** - Always save state before fatal exit
5. **Streaming First** - Never buffer full tool output
6. **Single Session** - One process = one session, clean boundaries
7. **Explicit Error Handling** - Every error has recovery strategy

### ExecutionContext

The single source of truth for execution state:

```rust
pub struct ExecutionContext {
    // Configuration - loaded once at startup
    pub settings: EffectiveSettings,
    pub permissions: PermissionMatrix,
    pub command: Commands,

    // Systems - initialized at startup
    pub hooks: HooksSystem,
    pub checkpoint_saver: SessionSaver,
    pub checkpoint_loader: SessionLoader,
    pub plugins: PluginLoader,
    pub slash_commands: SlashCommandRegistry,

    // Session state - may resume or create new
    pub session: Option<Session>,
    pub session_id: String,
    pub session_created_at: Instant,

    // Execution tracking - mutated during run
    pub current_tool: Option<String>,
    pub tool_start_time: Option<Instant>,
    pub error_count: u32,
    pub checkpoint_count: u32,

    // Shutdown signal - for graceful termination
    pub shutdown_signal: Option<tokio::sync::mpsc::Receiver<()>>,
}
```

### Hook Lifecycle

8 critical hook points (9 events, but we start with these 8):

```
SessionStart
    ↓
    [REPL Loop / Tool Execution / Etc]
    ├─ UserPromptSubmit (when user types in chat)
    ├─ PreToolUse (before executing tool)
    ├─ PostToolUse (after tool returns)
    ├─ Stop (check if should exit)
    └─ PreCompact (before compacting history)
    ↓
SessionEnd (always executed, even on error)
```

### Permission Model

Settings define tool permissions:

```yaml
permissions:
  bash:
    mode: Allow
    patterns: []
  edit:
    mode: Ask
    patterns: ["*.rs"]  # Only ask for Rust files
  write:
    mode: Deny
```

In code:
```rust
match ctx.permissions.check("bash") {
    Ok(()) => { /* execute */ }
    Err(e) => { /* deny */ }
}
```

### Checkpoint Strategy

Automatic saving at:
- Every successful tool execution
- Every N minutes during interactive session
- On fatal error (before exit)
- Manual: `/checkpoint "description"` command

Each checkpoint captures:
- Conversation history
- File snapshots
- Working directory
- Environment variables
- Session metadata

### Signal Handling

Register handlers for SIGINT, SIGTERM:

```rust
// In startup_phase:
let (shutdown_tx, mut shutdown_rx) = tokio::sync::mpsc::channel(1);

tokio::spawn(async move {
    let mut signals = Signals::new(&[SIGINT, SIGTERM])?;
    if let Some(_) = signals.next().await {
        let _ = shutdown_tx.send(()).await;
    }
});
```

Then in mode handlers:
```rust
tokio::select! {
    result = run_operation() => result,
    _ = shutdown_rx.recv() => {
        // Graceful shutdown
        shutdown_phase().await
    }
}
```

## Implementation Strategy

### Start Simple, Build Up

1. **Phase A (Days 1-2): Foundation**
   - Create type definitions
   - Get it compiling
   - No complex logic yet

2. **Phase B (Days 2-4): Initialization**
   - Load settings
   - Init each system sequentially
   - Build ExecutionContext
   - Add signal handling

3. **Phase C (Days 4-7): Mode Dispatch**
   - Route to each handler
   - Get each mode working
   - Add basic error handling

4. **Phase D (Days 7-9): Lifecycle**
   - Hook execution
   - Permission checking
   - Checkpoint scheduling
   - Unified error handling

5. **Phase E (Days 9-12): Shutdown**
   - SessionEnd hooks
   - Final checkpoint
   - Resource cleanup
   - Proper exit codes

### Testing Strategy

**Phase A:** Compile tests
```bash
cargo build --bin claude-code
```

**Phase B:** Integration setup tests
```bash
cargo test --lib context::
cargo test --lib error::
```

**Phase C:** Mode dispatch tests
```bash
cargo test --test integration_main test_chat_mode
cargo test --test integration_main test_tool_mode
```

**Phase D:** Lifecycle tests
```bash
cargo test --test integration_main test_hooks
cargo test --test integration_main test_permissions
```

**Phase E:** Full E2E tests
```bash
cargo test --test integration_main
```

## Common Pitfalls to Avoid

### 1. Global State
**Wrong:**
```rust
static CONTEXT: Mutex<ExecutionContext> = ...;
```
**Right:**
```rust
async fn handler(ctx: ExecutionContext) { }
```

### 2. Hook Crashes
**Wrong:**
```rust
for hook in hooks {
    hook.execute().await?;  // ❌ One failure = crash
}
```
**Right:**
```rust
for hook in hooks {
    match hook.execute().await {
        Ok(_) => {},
        Err(e) => tracing::warn!("Hook failed: {}", e),  // ✓ Continue
    }
}
```

### 3. Duplicate Code
**Wrong:**
```rust
async fn chat_mode(ctx: &ExecutionContext) -> Result<()> {
    execute_hooks(PreToolUse)?;
    run_chat().await?;
    execute_hooks(PostToolUse)?;
}

async fn tool_mode(ctx: &ExecutionContext) -> Result<()> {
    execute_hooks(PreToolUse)?;
    run_tool().await?;
    execute_hooks(PostToolUse)?;
}
```
**Right:**
```rust
async fn execute_with_hooks(op: impl Fn() -> Result<()>) -> Result<()> {
    execute_hooks(PreToolUse)?;
    op().await?;
    execute_hooks(PostToolUse)?;
}
```

### 4. Forgetting Async
**Wrong:**
```rust
impl Drop for ShutdownGuard {
    fn drop(&mut self) {
        ctx.checkpoint_saver.save_session(&ctx.session)?;  // ❌ Can't await
    }
}
```
**Right:**
```rust
impl Drop for ShutdownGuard {
    fn drop(&mut self) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let _ = ctx.checkpoint_saver.save_session(&ctx.session).await;
        });
    }
}
```

### 5. Losing Context
**Wrong:**
```rust
async fn run_cli() -> Result<()> {
    let ctx = startup_phase().await?;
    mode_dispatch(ctx).await  // ctx moved
    // Can't access ctx here for shutdown
}
```
**Right:**
```rust
async fn run_cli() -> Result<()> {
    let mut ctx = startup_phase().await?;
    let _guard = ShutdownGuard { ctx: Some(ctx.clone()), error: None };
    mode_dispatch(&mut ctx).await
    // Shutdown runs in Drop even if error
}
```

## Integration Points

### With Interactive Mode
Interactive already handles chat loop. New main.rs:
- Loads settings before calling interactive
- Passes ExecutionContext to interactive
- Saves checkpoint after interactive returns

**File:** `/crates/cli/src/interactive.rs`
**Change:** Add parameter `ctx: ExecutionContext` to `run_interactive()`

### With Settings System
Settings hierarchy already built. New main.rs:
- Loads hierarchy in startup_phase
- Validates merged settings
- Extracts: model, timeout, permissions, log_level

**No changes needed to settings module**

### With Hooks System
Hooks system already loads from `.claude/hooks.yaml`. New main.rs:
- Initializes HooksSystem in startup
- Calls execute_hooks at 8 lifecycle points
- Non-blocking error handling for hooks

**No changes needed to hooks module**

### With Checkpoint System
Checkpoint system already handles save/load. New main.rs:
- Initializes SessionSaver/SessionLoader in startup
- Calls save_session at checkpoints
- Calls resume_session if --session flag

**No changes needed to checkpoint module**

### With Plugins System
Plugin system already handles discovery/loading. New main.rs:
- Initializes PluginLoader with discovery
- Dispatches plugin commands to PluginExecutor

**No changes needed to plugins module**

## FAQ for Builder

**Q: What if a system fails during startup?**
A: Exit with appropriate code (10-12 range) and don't try to checkpoint (may not be initialized)

**Q: What if a hook hangs?**
A: Timeout after 5 seconds. Log warning and continue. Non-blocking means main flow continues.

**Q: What if the user hits Ctrl+C?**
A: Signal handler sets shutdown flag. Mode handlers check it and call shutdown_phase. Exit with code 130.

**Q: What about cleanup if process is killed (SIGKILL)?**
A: Can't do anything. That's OS-level. Just make sure signal handlers work for SIGINT/SIGTERM.

**Q: Do I need to modify the tools themselves?**
A: No. Tools already stream via Tool trait. Main just calls them via execute_tool().

**Q: How do I handle tool parameter parsing?**
A: Parse params_json from CLI args as serde_json::Value, pass to tool. Tool handles deserialization.

**Q: What if settings file doesn't exist?**
A: Use defaults. Settings::default() should have sensible defaults for everything.

**Q: Do I need to implement the complete agent task format?**
A: No. Just parse task file and pass to AgentTool. Let AgentTool handle the logic.

## Quick Reference: Code Structure

```rust
// main.rs structure
#[tokio::main]
async fn main() -> ExitCode {
    // Wrap in guard for cleanup
    match run_cli().await {
        Ok(code) => code,
        Err(e) => {
            eprintln!("Error: {}", e);
            ExitCode::FAILURE
        }
    }
}

async fn run_cli() -> Result<ExitCode> {
    // Phase 1
    let mut ctx = startup_phase().await?;

    // Phase 2 & 3 (lifecycle inside mode handlers)
    mode_dispatch(&mut ctx).await
        .map_err(|e| {
            // Phase 4 (on error)
            shutdown_phase(&mut ctx, &e).await?;
            e
        })?;

    // Phase 4 (on success)
    shutdown_phase(&mut ctx, None).await?;

    Ok(ExitCode::SUCCESS)
}

async fn startup_phase() -> Result<ExecutionContext> {
    // 1. Parse args (already done)
    // 2. Init logging
    // 3. Load settings
    // 4. Init hooks
    // 5. Load plugins
    // 6. Check checkpoints
    // 7. SessionStart hooks
    // 8. Return ExecutionContext
}

async fn mode_dispatch(ctx: &mut ExecutionContext) -> Result<ExitCode> {
    match &ctx.command {
        Commands::Chat => chat_mode(ctx).await,
        Commands::Tool { name, params } => tool_mode(ctx, name, params).await,
        // ... other modes ...
    }
}

async fn chat_mode(ctx: &mut ExecutionContext) -> Result<ExitCode> {
    // Delegate to interactive module
    // (may have hooks internally)
}

async fn tool_mode(ctx: &mut ExecutionContext, name: &str, params: &str) -> Result<ExitCode> {
    // 1. Check permission
    // 2. PreToolUse hook
    // 3. Execute tool (streaming)
    // 4. PostToolUse hook
    // 5. Checkpoint if needed
}

async fn shutdown_phase(ctx: &mut ExecutionContext, error: Option<&Error>) -> Result<()> {
    // 1. SessionEnd hooks (timeout 5s)
    // 2. Save final checkpoint
    // 3. Cleanup resources
}
```

## Deliverables Checklist

Phase A:
- [ ] context.rs compiles
- [ ] error.rs compiles
- [ ] main.rs skeleton compiles
- [ ] lib.rs exports new modules

Phase B:
- [ ] Settings load in startup_phase
- [ ] Hooks system initializes
- [ ] Plugins discover and load
- [ ] Checkpoints load for recovery
- [ ] SessionStart hooks execute
- [ ] Signal handlers installed

Phase C:
- [ ] CLI command enum defined
- [ ] All 5 modes dispatch correctly
- [ ] Chat mode works
- [ ] Tool mode works
- [ ] Command mode works
- [ ] Plugin mode works
- [ ] Agent mode works

Phase D:
- [ ] PreToolUse hooks execute
- [ ] PostToolUse hooks execute
- [ ] Permission checking works
- [ ] Checkpoint scheduling works
- [ ] All errors handled gracefully

Phase E:
- [ ] SessionEnd hooks execute
- [ ] Final checkpoint saved
- [ ] Resources cleaned up
- [ ] All exit codes correct
- [ ] Graceful shutdown on Ctrl+C
- [ ] Integration tests pass

## Success Criteria

```bash
# Builds without errors
cargo build --bin claude-code

# Runs all unit tests
cargo test --lib

# Runs all integration tests
cargo test --test integration_main

# Manual smoke test - chat mode
cargo run --bin claude-code -- chat

# Manual smoke test - tool mode
cargo run --bin claude-code -- tool read --params '{"file_path": "/etc/hostname"}'

# Manual smoke test - session recovery
cargo run --bin claude-code -- --session test-session chat
# Exit and resume
cargo run --bin claude-code -- --session test-session chat
```

You have comprehensive documentation in:
- `MAIN_INTEGRATION_ARCHITECTURE.md` - High-level design
- `MAIN_MODULE_SPECIFICATION.md` - Detailed module contracts
- `MAIN_IMPLEMENTATION_ROADMAP.md` - Step-by-step implementation

Good luck! The work is well-scoped and the documentation is thorough. Start with Phase A and don't rush.
