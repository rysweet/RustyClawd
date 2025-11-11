# Main CLI Integration - Quick Reference

## File Map

```
/crates/cli/src/
├── main.rs                  (REPLACE: 700-900 lines)
│   ├── main()              - Entry point
│   ├── run_cli()           - 4-phase orchestration
│   ├── startup_phase()     - Phase 1: Initialize all systems
│   ├── mode_dispatch()     - Phase 2: Route to handler
│   ├── chat_mode()         - Chat handler
│   ├── tool_mode()         - Tool handler
│   ├── command_mode()      - Command handler
│   ├── plugin_mode()       - Plugin handler
│   ├── agent_mode()        - Agent handler
│   ├── shutdown_phase()    - Phase 4: Cleanup
│   ├── execute_pre_tool_hooks()
│   ├── execute_post_tool_hooks()
│   ├── execute_session_end_hooks()
│   └── [unit tests]
│
├── context.rs              (NEW: 200 lines)
│   ├── ExecutionContext    - Main state struct
│   ├── PermissionMatrix    - Permission checking
│   └── [unit tests]
│
├── error.rs                (NEW: 150 lines)
│   ├── CliError            - Error enum
│   ├── impl Display        - Formatting
│   └── impl Error
│
├── lib.rs                  (UPDATE: 2 lines)
│   └── pub mod context;
│   └── pub mod error;
│
└── [existing modules unchanged]

/Cargo.toml                 (UPDATE: 1 line)
    └── signal-hook-tokio = "0.3"

/tests/integration_main.rs  (NEW: 500+ lines)
    └── All integration tests
```

## Execution Flow Diagram

```
User Input
    │
    ├─ cargo run -- --debug --session my-session chat
    │
main()
    │
tokio::main async block
    ├─ run_cli() spawned
    │
    PHASE 1: STARTUP
    ├─ Cli::parse()                          // Parse args
    ├─ tracing::init()                       // Init logging
    ├─ SettingsLoader::load_hierarchy()      // Load: default → user → project → cmdline → enterprise
    ├─ settings.validate()                   // Validate merged settings
    ├─ HooksSystem::new()                    // Create hooks
    ├─ hooks.load_from_file()                // Load .claude/hooks.yaml
    ├─ PluginDiscovery::discover_all()       // Find plugins
    ├─ PluginLoader::load(enabled_plugins)   // Load enabled
    ├─ SessionLoader::resume_session()       // If --session flag (optional)
    ├─ checkpoint_loader.load()              // If --checkpoint flag (optional)
    ├─ ExecutionContext::new()               // Combine all into context
    └─ hooks.execute_hooks(SessionStart)     // Execute SessionStart hooks
         │
         PHASE 2: MODE DISPATCH
         ├─ match command {
         │
         │  Chat =>
         │  ├─ chat_mode(ctx)
         │  ├─ interactive::run_interactive()
         │  └─ checkpoint_saver.save_session()
         │
         │  Tool { name, params } =>
         │  ├─ tool_mode(ctx, name, params)
         │  ├─ permissions.check(name)
         │  ├─ hooks.execute(PreToolUse)
         │  ├─ execute_tool(name, params)    [STREAM OUTPUT]
         │  ├─ hooks.execute(PostToolUse)
         │  └─ maybe checkpoint_saver.save()
         │
         │  Command { name, args } =>
         │  ├─ command_mode(ctx, name, args)
         │  ├─ load_command(name)
         │  ├─ expand_template(args)
         │  └─ execute_via_agent()
         │
         │  Plugin { plugin, cmd, params } =>
         │  ├─ plugin_mode(ctx, plugin, cmd)
         │  ├─ get_plugin_metadata(plugin)
         │  ├─ plugins.execute_command()
         │  └─ output_json_result()
         │
         │  Agent { task } =>
         │  ├─ agent_mode(ctx, task)
         │  ├─ parse_task_def(task)
         │  ├─ agent_tool.execute(task)
         │  └─ checkpoint_saver.save()
         │  }
         │
         PHASE 3: LIFECYCLE (within handlers)
         ├─ Loop: Read → PreToolUse → Execute → PostToolUse
         ├─ Every N minutes: checkpoint_saver.save()
         ├─ On command: checkpoint_saver.save()
         └─ Permission checks before each tool
         │
         PHASE 4: SHUTDOWN (on exit or error)
         ├─ hooks.execute(SessionEnd)        [timeout 5s]
         ├─ checkpoint_saver.save_session()  [final]
         ├─ Kill background processes
         ├─ Flush logs
         └─ Return ExitCode
             │
             RETURN TO MAIN
             └─ main() exits with code
```

## State Machines

### Execution Phases

```
STARTUP ──→ DISPATCH ──→ LIFECYCLE ──→ SHUTDOWN
   │           │            │            │
   │           │            │            └─→ ExitCode
   │           │            │
   │           │            └─→ OnError ──→ ErrorShutdown
   │           │
   │           └─→ OnPanic ──────────────→ ErrorShutdown
   │
   └─→ OnError ──────────────────────────→ ErrorShutdown

ErrorShutdown:
  - SessionEnd hooks (timeout)
  - Save checkpoint (mark as "paused")
  - Cleanup
  - Exit with error code
```

### Hook Event Sequence

```
SessionStart
    ↓
[Handler Processing]
    ├─ (Chat: REPL loop)
    │  ├─ Read input
    │  ├─ UserPromptSubmit hook
    │  ├─ Stream response
    │  ├─ (Tool called)
    │  │  ├─ PreToolUse hook
    │  │  ├─ Execute tool
    │  │  └─ PostToolUse hook
    │  └─ Loop back
    │
    ├─ (Tool: Single execution)
    │  ├─ PreToolUse hook
    │  ├─ Execute tool
    │  └─ PostToolUse hook
    │
    └─ (Agent: Multi-step)
       ├─ PreToolUse hook
       ├─ Execute tool
       ├─ PostToolUse hook
       └─ Loop in agent
    ↓
[Periodic]
  - Stop hook (should exit?)
  - PreCompact hook (before compacting history)
    ↓
SessionEnd (always)
```

## Key Data Structures

### ExecutionContext (core state)

```rust
pub struct ExecutionContext {
    // ╔════════════════════════════════════════╗
    // ║ CONFIGURATION (loaded at startup)      ║
    // ╚════════════════════════════════════════╝
    settings: EffectiveSettings,                // All 5-tier settings merged
    permissions: PermissionMatrix,              // Tool access control
    command: Commands,                          // Which mode to run

    // ╔════════════════════════════════════════╗
    // ║ SYSTEMS (initialized at startup)       ║
    // ╚════════════════════════════════════════╝
    hooks: HooksSystem,                         // Lifecycle hooks
    checkpoint_saver: SessionSaver,             // Save state to disk
    checkpoint_loader: SessionLoader,           // Load state from disk
    plugins: PluginLoader,                      // Plugin registry
    slash_commands: SlashCommandRegistry,       // Slash command registry

    // ╔════════════════════════════════════════╗
    // ║ SESSION STATE (may resume or create)   ║
    // ╚════════════════════════════════════════╝
    session: Option<Session>,                   // Conversation + checkpoints
    session_id: String,                         // Unique session identifier
    session_created_at: Instant,                // Session start time

    // ╔════════════════════════════════════════╗
    // ║ EXECUTION TRACKING (mutated during run)║
    // ╚════════════════════════════════════════╝
    current_tool: Option<String>,               // What tool running now?
    tool_start_time: Option<Instant>,           // When did tool start?
    error_count: u32,                           // How many errors so far?
    checkpoint_count: u32,                      // Checkpoints since last save

    // ╔════════════════════════════════════════╗
    // ║ SHUTDOWN (signal handling)             ║
    // ╚════════════════════════════════════════╝
    shutdown_signal: Option<Receiver<()>>,     // Ctrl+C signal
}
```

### CliError (exit code mapping)

```rust
pub enum CliError {
    // Startup errors
    Settings(String)        → exit 10
    Hooks(String)           → exit 11
    Plugins(String)         → exit 11
    Checkpoint(String)      → exit 12

    // Execution errors
    PermissionDenied(String) → exit 20
    ToolError { .. }        → exit 21
    ApiError(String)        → exit 22

    // Signals
    SignalInterrupt         → exit 130
    SignalTerminate         → exit 143
}
```

## Command Syntax

```bash
# Interactive chat
claude-code chat

# Execute single tool
claude-code tool bash --params '{"command": "ls", "timeout": 5000}'
claude-code tool read --params '{"file_path": "/etc/hostname"}'
claude-code tool write --params '{"file_path": "/tmp/x", "content": "hello"}'
claude-code tool edit --params '{"file_path": "/tmp/x", "old_string": "x", "new_string": "y"}'
claude-code tool glob --params '{"pattern": "**/*.rs"}'
claude-code tool grep --params '{"pattern": "TODO", "path": "src"}'

# Execute slash command
claude-code command my-command arg1 arg2

# Execute plugin command
claude-code plugin com.example.plugin mycommand --params '{"key": "value"}'

# Execute agent task
claude-code agent --task /path/to/task.yaml

# With flags
--debug                    Enable debug logging
--session SESSION_ID       Resume session
--checkpoint CHECKPOINT_ID Restore from checkpoint
--settings /path/config    Custom settings file
```

## Hook Execution Pattern

```rust
// Non-blocking hook execution pattern (used everywhere):

match hooks.execute_hooks(HookEvent::PreToolUse, context).await {
    Ok(results) => {
        for result in results {
            if !result.success {
                tracing::warn!("Hook failed: {:?}", result.error);
                // Don't return error, continue execution
            }
        }
    }
    Err(e) => {
        tracing::warn!("Hook system error (non-fatal): {}", e);
        // Log and continue, never crash main flow
    }
}

// With timeout:

let result = tokio::time::timeout(
    Duration::from_secs(5),
    hooks.execute_hooks(event, context)
).await;

match result {
    Ok(Ok(results)) => { /* process */ }
    Ok(Err(e)) => tracing::warn!("Hook error: {}", e),
    Err(_) => tracing::warn!("Hook timeout (5s)"),
}
```

## Permission Checking Pattern

```rust
// Always check before tool execution:

// Option 1: Simple check
ctx.permissions.check("bash")?;  // Returns error if denied

// Option 2: With recovery
match ctx.permissions.check("bash") {
    Ok(()) => { /* execute */ }
    Err(e) => return Err(anyhow!(CliError::PermissionDenied(e))),
}

// Option 3: Ask user
match ctx.permissions.check("bash") {
    Ok(()) => { /* execute */ }
    Err(PermissionError::Denied) => return Err(anyhow!("Denied")),
    Err(PermissionError::Ask) => {
        if ask_user_permission("bash").await? {
            // execute
        } else {
            return Err(anyhow!("User denied"));
        }
    }
}
```

## Checkpoint Strategy

```rust
// When to save:

// 1. After successful write/edit operations
if matches!(tool_name, "write" | "edit") {
    ctx.checkpoint_saver.save_session(&ctx.session)?;
}

// 2. Periodically (every 5 minutes or 10 turns)
if ctx.should_checkpoint() {
    ctx.checkpoint_saver.save_session(&ctx.session)?;
    ctx.checkpoint_completed();
}

// 3. Before exiting on fatal error
match run_operation().await {
    Ok(_) => { /* continue */ }
    Err(e) => {
        ctx.checkpoint_saver.save_session(&ctx.session)?;
        return Err(e);
    }
}

// 4. On graceful shutdown
if let Err(e) = shutdown_phase(&mut ctx).await {
    ctx.checkpoint_saver.save_session(&ctx.session)?;
}
```

## Testing Quick Checks

```bash
# Unit tests
cargo test --lib context::
cargo test --lib error::
cargo test --lib main::

# Integration tests
cargo test --test integration_main

# Manual tests
cargo run -- chat
cargo run -- tool bash --params '{"command":"echo hello"}'

# With debug logging
RUST_LOG=debug cargo run -- --debug chat
```

## Common Code Patterns

### Pattern: Execute with Pre/Post Hooks

```rust
async fn execute_tool_with_hooks(
    ctx: &mut ExecutionContext,
    tool_name: &str,
    params: serde_json::Value,
) -> Result<ToolOutput> {
    // Pre hook
    execute_pre_tool_hooks(&ctx.hooks, tool_name, &params).await?;

    // Execute
    let result = execute_tool(tool_name, params).await?;

    // Post hook
    execute_post_tool_hooks(&ctx.hooks, tool_name, &result, None, elapsed_ms).await?;

    Ok(result)
}
```

### Pattern: Checkpoint After Operation

```rust
async fn save_with_checkpoint(ctx: &mut ExecutionContext) -> Result<()> {
    let result = do_something().await?;

    if ctx.should_checkpoint() {
        ctx.checkpoint_saver.save_session(&ctx.session).await?;
        ctx.checkpoint_completed();
    }

    Ok(result)
}
```

### Pattern: Graceful Error Recovery

```rust
match operation().await {
    Ok(result) => Ok(result),
    Err(e) => {
        tracing::error!("Operation failed: {}", e);

        // Save checkpoint for recovery
        if let Err(cp_err) = ctx.checkpoint_saver.save_session(&ctx.session).await {
            tracing::warn!("Failed to save checkpoint: {}", cp_err);
        }

        Err(e)
    }
}
```

### Pattern: Timeout with Fallback

```rust
let result = tokio::time::timeout(
    Duration::from_secs(5),
    do_something()
).await;

match result {
    Ok(Ok(value)) => value,
    Ok(Err(e)) => return Err(e),
    Err(_) => {
        tracing::warn!("Operation timeout, using default");
        Default::default()
    }
}
```

## Exit Code Reference

| Code | Meaning | Action |
|------|---------|--------|
| 0 | Success | Normal exit |
| 1 | Generic error | Unexpected error |
| 2 | Misuse | Invalid arguments |
| 10 | Settings error | Can't load config |
| 11 | Plugin error | Can't load plugins |
| 12 | Checkpoint error | Can't save state |
| 20 | Permission denied | Tool not allowed |
| 21 | Tool execution error | Tool failed |
| 22 | API error | Communication error |
| 130 | SIGINT | User pressed Ctrl+C |
| 143 | SIGTERM | Terminated by OS |

## Dependency Graph

```
main.rs
├── context.rs ─────────────────────┐
├── error.rs                         │
├── settings module ─────────────────┼──→ EffectiveSettings
├── hooks module ────────────────────┼──→ HooksSystem
├── plugins module ──────────────────┼──→ PluginLoader
├── checkpoint module ───────────────┼──→ SessionSaver/Loader
├── interactive module ──────────────┼──→ run_interactive()
├── commands module ─────────────────┼──→ SlashCommandRegistry
├── tools crate ─────────────────────┼──→ Tool trait + 14 tools
├── core crate ──────────────────────┼──→ Client, Config
└── signal-hook-tokio ───────────────┘

All modules flow through ExecutionContext
ExecutionContext depends on all systems being initialized
```

## Performance Considerations

| Operation | Cost | When | Notes |
|-----------|------|------|-------|
| Load settings | ~5ms | Startup | Serial, not cached |
| Discover plugins | ~50ms | Startup | Filesystem scan |
| Load plugins | ~100ms | Startup | Per plugin |
| Hook execution | ~10ms | Per hook | Add timeout |
| Checkpoint save | ~50ms | Periodic | Async, non-blocking |
| Permission check | <1ms | Per tool | Hash lookup |
| Shutdown | <5s | Exit | Has timeout |

## Debug Logging

```bash
# Enable debug logging
RUST_LOG=debug cargo run -- --debug chat

# Filter specific modules
RUST_LOG=claude_code_cli=debug cargo run -- chat

# Trace-level (very verbose)
RUST_LOG=trace cargo run -- --debug chat

# JSON output for parsing
RUST_LOG_JSON=1 cargo run -- chat | jq .
```

---

**Remember:** This is a sophisticated orchestration layer. Start with Phase A (foundation), understand each piece, then build up. Don't rush. Test after each phase. Reference the detailed docs when stuck.
