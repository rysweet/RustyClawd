# Main CLI Integration Architecture

## Overview

This document defines the sophisticated orchestration layer for the Claude Code CLI main entry point. The new `main.rs` unifies 8 distinct systems into a cohesive CLI with proper lifecycle management, error handling, and graceful shutdown.

## Systems to Integrate

1. **Settings System** - 5-tier configuration hierarchy (default → user → project → cmdline → enterprise)
2. **Hooks System** - Lifecycle event handlers (SessionStart, SessionEnd, PreToolUse, PostToolUse, etc.)
3. **Plugins System** - Dynamic command/skill loading and execution
4. **Checkpoint System** - Session recovery and state persistence
5. **Interactive Mode** - REPL with streaming responses
6. **Slash Commands** - Command-like prompts with expansion
7. **Tools Suite** - 14 autonomous tools (bash, read, write, edit, glob, grep, etc.)
8. **Agent Tool** - Complex multi-step task orchestration

## Architecture Layers

```
┌─────────────────────────────────────────────────────────┐
│ CLI Entry Point (main.rs)                               │
├─────────────────────────────────────────────────────────┤
│                                                           │
│  Phase 1: STARTUP SEQUENCE                              │
│  ├─ Parse CLI arguments                                 │
│  ├─ Load settings hierarchy                             │
│  ├─ Initialize logging from settings                    │
│  ├─ Initialize hooks system                             │
│  ├─ Load plugins                                        │
│  ├─ Check for session checkpoint                        │
│  └─ Execute SessionStart hooks                          │
│                                                           │
│  Phase 2: MODE DETECTION & DISPATCH                     │
│  ├─ Determine mode: chat | tool | command              │
│  ├─ Dispatch to appropriate handler                     │
│  └─ Pass execution context & settings                   │
│                                                           │
│  Phase 3: LIFECYCLE MANAGEMENT                          │
│  ├─ Execute PreToolUse/PostToolUse hooks               │
│  ├─ Handle errors with recovery strategies             │
│  ├─ Save periodic checkpoints                          │
│  └─ Handle graceful shutdown                           │
│                                                           │
│  Phase 4: SHUTDOWN SEQUENCE                             │
│  ├─ Execute SessionEnd hooks                           │
│  ├─ Save final checkpoint if applicable                │
│  ├─ Cleanup resources                                  │
│  └─ Exit with proper status code                       │
│                                                           │
└─────────────────────────────────────────────────────────┘
```

## Startup Sequence (Phase 1)

### 1.1 CLI Argument Parsing

```rust
struct Cli {
    #[command(subcommand)]
    command: Commands,

    // Global flags
    #[arg(long)]
    debug: bool,

    #[arg(long)]
    settings: Option<String>,  // Custom settings path

    #[arg(long)]
    session: Option<String>,   // Session ID to resume

    #[arg(long)]
    checkpoint: Option<String>, // Checkpoint ID to restore
}

enum Commands {
    Chat,                    // Interactive mode
    Tool { name, params },  // Execute single tool
    Command { name, args }, // Execute slash command
    Plugin { plugin, cmd }, // Execute plugin command
    Agent { task_def },     // Execute agent task
}
```

### 1.2 Settings Initialization

**Sequence:**
1. Load settings hierarchy with precedence
2. Validate all settings
3. Apply security restrictions (enterprise policies)
4. Extract logging level
5. Setup tool permissions matrix

**Responsible Module:** `settings::SettingsLoader`

**Key Types:**
- `SettingsHierarchy` - 5-tier cascade
- `EffectiveSettings` - Merged result
- `PermissionMatrix` - Tool access control

### 1.3 Logging Initialization

**From effective settings:**
- Log level (debug, info, warn, error)
- Log format (json, text, compact)
- Output destination (stdout, file)
- Filter patterns

**Setup:**
```rust
initialize_logging(&effective_settings);
```

### 1.4 Hooks System Initialization

**Sequence:**
1. Load hooks configuration from `.claude/hooks.yaml`
2. Parse all hook definitions
3. Validate hook commands exist
4. Build hook registry indexed by event type

**Responsible Module:** `hooks::HooksSystem`

**Key Types:**
- `HookRegistry` - Indexed hook storage
- `HookEvent` - Enum of 9 lifecycle events
- `HookContext` - Execution context

### 1.5 Plugin System Initialization

**Sequence:**
1. Discover plugins from `~/.claude/plugins/` and `./.claude/plugins/`
2. Load plugin manifests
3. Validate manifest schemas
4. Initialize enabled plugins
5. Index commands and skills

**Responsible Module:** `plugins::PluginLoader`

**Key Types:**
- `PluginDiscovery` - Filesystem scanning
- `PluginManifest` - Plugin definition
- `PluginExecutor` - Command/skill execution

### 1.6 Checkpoint Recovery

**Sequence:**
1. Check for `--session` or `--checkpoint` flags
2. Query checkpoint storage for session metadata
3. If session exists and valid:
   - Load session state
   - Load conversation history
   - Load file snapshots
4. Restore state to working directory if `--checkpoint` specified

**Responsible Module:** `checkpoint::SessionLoader`

**Key Types:**
- `Session` - Session metadata
- `Checkpoint` - State snapshot
- `RestoreScope` - What to restore (Code | Conversation | Both)

### 1.7 SessionStart Hook Execution

**When:** After all initialization complete
**Context:** Available settings, plugins, session (if resuming)
**Possible Actions:**
- Validation of environment
- Setup of external services
- Logging/telemetry
- Warning on deprecated settings

**Error Handling:** Non-blocking (log and continue)

## Mode Detection and Dispatch (Phase 2)

### 2.1 Mode Routing Logic

```rust
match cli.command {
    Commands::Chat => {
        // INTERACTIVE MODE
        run_interactive_session(settings, hooks_system, plugins)
    }

    Commands::Tool { name, params } => {
        // TOOL MODE
        execute_single_tool(name, params, settings, hooks_system)
    }

    Commands::Command { name, args } => {
        // SLASH COMMAND MODE
        execute_slash_command(name, args, settings, plugins)
    }

    Commands::Plugin { plugin, cmd } => {
        // PLUGIN COMMAND MODE
        execute_plugin_command(plugin, cmd, settings)
    }

    Commands::Agent { task_def } => {
        // AGENT TASK MODE
        execute_agent_task(task_def, settings, all_tools)
    }
}
```

### 2.2 Interactive Mode (Chat)

**Responsibility:** `interactive::run_interactive_session()`

**Inputs:**
- `EffectiveSettings` - Config & permissions
- `HooksSystem` - Hook execution
- `PluginSystem` - Plugin commands
- Optional: `Session` (if resuming)

**Lifecycle:**
1. Execute `UserPromptSubmit` hooks on each input
2. Stream response from Claude
3. Execute `PostToolUse` hooks if tools used
4. Periodic checkpoint saving
5. On exit: save final checkpoint

**Output:** Full session saved with all checkpoints

### 2.3 Tool Mode (Single Tool)

**Responsibility:** Dispatch to specific tool with hooks

**Lifecycle:**
1. Validate tool name against permission matrix
2. Parse parameters from JSON/YAML
3. Execute `PreToolUse` hook
4. Execute tool with streaming
5. Execute `PostToolUse` hook
6. Checkpoint if applicable (esp. for edit tools)
7. Output result as JSON

**Error Handling:**
- Permission denied: Exit 403
- Invalid params: Exit 400
- Tool execution error: Exit 500
- Checkpoint save error: Warn but exit 0

### 2.4 Slash Command Mode

**Responsibility:** `commands::execute_slash_command()`

**Lifecycle:**
1. Parse command name and args
2. Load command from `.claude/commands/` directory
3. Expand variables and parameters
4. Execute expanded command (calls Agent tool)
5. Return result

**Permission Model:** Inherits settings-based restrictions

### 2.5 Plugin Command Mode

**Responsibility:** `plugins::PluginExecutor`

**Lifecycle:**
1. Verify plugin is loaded
2. Validate command in plugin manifest
3. Check permissions (settings matrix)
4. Execute plugin command
5. Return JSON result

### 2.6 Agent Task Mode

**Responsibility:** `tools::AgentTool`

**Lifecycle:**
1. Parse task definition (YAML/JSON)
2. Initialize agent with all 14 tools
3. Agent loops: perceive → decide → act
4. Save checkpoint after each major action
5. On completion: summarize and save

## Lifecycle Management (Phase 3)

### 3.1 Hook Integration Points

**Pre-Execution Hooks:**

```rust
hooks_system.execute_hooks(
    HookEvent::PreToolUse,
    HookContext {
        tool_name: "bash",
        params: params_json,
        session: current_session.clone(),
        settings: effective_settings.clone(),
    }
).await?;
```

**Post-Execution Hooks:**

```rust
hooks_system.execute_hooks(
    HookEvent::PostToolUse,
    HookContext {
        tool_name: "bash",
        result: tool_result.clone(),
        exit_code: Some(0),
        duration_ms: elapsed_ms,
    }
).await?;
```

**Other Lifecycle Hooks:**
- `Stop` - Check if user wants to exit
- `PreCompact` - Before conversation history compaction
- `Notification` - Filter notifications before display

### 3.2 Error Recovery Strategy

**Categories:**

1. **Recoverable Errors** (continue):
   - Hook execution failure (non-blocking)
   - Plugin command not found (warn)
   - Tool permission denied (log, skip)

2. **Blocking Errors** (save checkpoint, exit):
   - Settings validation failure
   - Plugin manifest invalid
   - API communication error
   - Tool execution timeout

3. **Fatal Errors** (cleanup, exit):
   - Unrecoverable IO error
   - Memory exhaustion
   - Signal (Ctrl+C)

**Error Flow:**
```
Error Occurs
  ↓
Categorize (Recoverable/Blocking/Fatal)
  ↓
If Blocking: Save Checkpoint
  ↓
If Blocking/Fatal: Execute SessionEnd Hooks
  ↓
Cleanup Resources
  ↓
Exit with appropriate code (0/1/>1)
```

### 3.3 Checkpoint Persistence

**Automatic checkpoints:**
- After successful edit operations
- Every N minutes during long sessions (configurable)
- Before exiting on error
- Manual: `/checkpoint "description"` slash command

**Checkpoint includes:**
- Current working directory
- Conversation history
- File snapshots (for edited files)
- Environment variables
- Active plugins/hooks state

### 3.4 Signal Handling

**Signals to handle:**
- `SIGINT` (Ctrl+C) - Graceful shutdown
- `SIGTERM` - Graceful shutdown
- `SIGHUP` - Reload configuration

**Flow:**
```
Signal Received
  ↓
Initiate graceful shutdown
  ↓
Finish current operation (with timeout)
  ↓
Save checkpoint
  ↓
Execute SessionEnd hooks
  ↓
Cleanup
  ↓
Exit with code 130 (SIGINT) or 143 (SIGTERM)
```

## Shutdown Sequence (Phase 4)

### 4.1 SessionEnd Hook Execution

**When:** Before any cleanup
**Context:** Final session state, exit reason, duration
**Typical Uses:**
- Upload telemetry
- Log session summary
- Trigger cleanup scripts
- Archive session

**Error Handling:** Timeout after 5 seconds, continue with cleanup

### 4.2 Final Checkpoint Save

**When:** SessionEnd hooks complete
**What:** Save final session state with exit reason
**Special Case:** If graceful shutdown, mark as "paused"

### 4.3 Resource Cleanup

1. **Close API connections** - Flush pending streams
2. **Kill background processes** - From process registry
3. **Flush logs** - Final writes to log files
4. **Close checkpoint storage** - Database connections
5. **Unload plugins** - Call cleanup hooks

### 4.4 Exit Code Strategy

```
0   - Normal exit or graceful completion
1   - Generic error
2   - Misuse of command/invalid args
10  - Settings/configuration error
11  - Plugin error
12  - Checkpoint error
20  - Permission denied
21  - Tool execution error
22  - API error
130 - SIGINT (Ctrl+C)
143 - SIGTERM
```

## Type System

### Core ExecutionContext

```rust
pub struct ExecutionContext {
    // Configuration
    pub settings: EffectiveSettings,
    pub permissions: PermissionMatrix,

    // Lifecycle
    pub hooks: HooksSystem,
    pub session: Option<Session>,
    pub checkpoint_saver: SessionSaver,
    pub checkpoint_loader: SessionLoader,

    // Plugin ecosystem
    pub plugins: PluginLoader,
    pub commands: SlashCommandRegistry,

    // Execution state
    pub current_tool: Option<String>,
    pub session_started_at: Instant,
    pub last_checkpoint_at: Instant,
    pub error_count: u32,
}
```

### Hook Context

```rust
pub struct HookContext {
    pub event: HookEvent,
    pub settings: EffectiveSettings,
    pub session: Option<Session>,
    pub tool_name: Option<String>,
    pub params: Option<serde_json::Value>,
    pub result: Option<serde_json::Value>,
    pub error: Option<String>,
    pub duration_ms: Option<u64>,
}
```

## Error Handling Patterns

### Pattern 1: With Checkpoint on Error

```rust
if let Err(e) = execute_operation() {
    tracing::error!("Operation failed: {}", e);

    // Save checkpoint
    if let Err(cp_err) = checkpoint_saver.save_session(&session) {
        tracing::error!("Failed to save checkpoint: {}", cp_err);
    }

    // Execute error hook
    let _ = hooks_system.execute_hooks(
        HookEvent::SessionEnd,
        &HookContext { error: Some(e.to_string()), .. }
    ).await;

    return Err(e);
}
```

### Pattern 2: Permission-Based Execution

```rust
fn check_permission(tool_name: &str, matrix: &PermissionMatrix) -> Result<()> {
    match matrix.check(tool_name) {
        PermissionMode::Allow => Ok(()),
        PermissionMode::Deny => Err(anyhow!("Permission denied for {}", tool_name)),
        PermissionMode::Ask => {
            // Prompt user interactively
            ask_user_permission(tool_name)
        }
    }
}
```

### Pattern 3: Graceful Hook Failures

```rust
// Hooks should never fail the entire operation
match hooks_system.execute_hooks(event, context).await {
    Ok(results) => {
        for result in results {
            if !result.success {
                tracing::warn!("Hook failed: {:?}", result.error);
            }
        }
    }
    Err(e) => {
        tracing::warn!("Hook system error (non-fatal): {}", e);
    }
}
```

## Implementation Phases

### Phase A: Core Structure (foundation)
1. Parse arguments and initialize logging
2. Load and merge settings hierarchy
3. Define ExecutionContext struct
4. Simple error handling

### Phase B: Initialization Systems (bootstrap)
1. Initialize HooksSystem
2. Initialize PluginLoader
3. Checkpoint system integration
4. Signal handling setup

### Phase C: Mode Dispatch (routing)
1. Route to correct handler based on command
2. Pass ExecutionContext to each handler
3. Unified error handling per mode
4. Result serialization (JSON output)

### Phase D: Lifecycle Management (orchestration)
1. Pre/post-execution hooks
2. Checkpoint saves at lifecycle events
3. Error recovery and escalation
4. Permission checking

### Phase E: Shutdown Sequence (cleanup)
1. SessionEnd hooks
2. Final checkpoint save
3. Resource cleanup
4. Exit code handling

## Testing Strategy

### Unit Tests
- Settings loading and merging
- Permission matrix checks
- Hook registry and execution
- Checkpoint save/load

### Integration Tests
- Full startup → chat → shutdown
- Tool execution with hooks
- Error recovery scenarios
- Signal handling

### E2E Tests
- Complete session with checkpoint resume
- Plugin discovery and loading
- All mode types (chat, tool, command, agent)
- Permission enforcement

## Constraints & Assumptions

### Constraints
1. Single-threaded model within a session (no concurrent tool execution)
2. Hooks must complete within 5 seconds
3. Settings files must be valid JSON/YAML
4. Plugins must have valid manifests
5. All state changes must be checkpointable

### Assumptions
1. `.claude/` directory structure exists or can be created
2. API credentials are available (from Config)
3. Filesystem is writable for checkpoints
4. Process can receive Unix signals
5. No circular dependencies between plugins/hooks

## Future Extensions

1. **Plugin Hot Reload** - Reload plugins without restart
2. **Distributed Hooks** - Remote hook execution
3. **State Sync** - Cross-device session sync
4. **Metrics** - Builtin observability
5. **Audit Log** - All operations logged immutably
6. **Rate Limiting** - Per-tool rate control
7. **Caching** - Tool result caching
8. **Replay** - Session replay from checkpoint
