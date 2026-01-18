# Claude Code Hooks System

Complete implementation of the Claude Code hooks system for Rust. Provides lifecycle hooks for all 10 events with command and prompt-based execution.

## Architecture

```
hooks/
├── mod.rs          - Public API and HooksSystem interface
├── types.rs        - All type definitions (Hook, HookConfig, HookContext, etc.)
├── executor.rs     - Hook execution engine with async/timeout support
├── loader.rs       - Configuration loading (.claude/settings.json or .claude/hooks/config.json)
└── registry.rs     - Hook registration and retrieval
```

## Hook Types

### Command Hooks
Execute bash commands with environment variables and timeout:
- Run shell scripts
- Execute validation commands
- Perform system operations
- Capture stdout/stderr

### Prompt Hooks
Execute LLM-based analysis (placeholder implementation):
- Analyze context with AI
- Make intelligent decisions
- Generate dynamic responses
- Future: Full LLM integration

## Lifecycle Events (10 Total)

### 1. SessionStart
Called when a new session begins.
**Use cases:**
- Initialize environment variables
- Source `$CLAUDE_ENV_FILE` for persistence
- Set up session state
- Load configuration

### 2. SessionEnd
Called when a session ends.
**Use cases:**
- Clean up resources
- Save session data
- Generate reports
- Archive transcripts

### 3. PreToolUse
Called before tool execution (can block execution).
**Use cases:**
- Permission validation
- Parameter validation
- Security checks
- Rate limiting
**Returns:** `permissionDecision` (allow/deny/ask)

### 4. PermissionRequest
Called when a tool permission is about to be requested from the user (in Ask mode).
**Use cases:**
- Auto-approve trusted tools
- Auto-deny dangerous operations
- Custom permission logic based on context
- Bypass user prompts for known-safe operations
**Returns:** `decision` (approve/deny) - if no output, user is prompted

### 5. PostToolUse
Called after tool execution.
**Use cases:**
- Log results
- Analyze output
- Trigger follow-up actions
- Update metrics

### 6. UserPromptSubmit
Called when user submits a prompt.
**Use cases:**
- Preprocess prompts
- Add context
- Validate input
- Track history

### 7. Stop
Called when checking if work is complete.
**Use cases:**
- Verify completion criteria
- Check for errors
- Validate deliverables
- Approve/block completion
**Returns:** `decision` (approve/block)

### 8. SubagentStop
Called when a subagent stops.
**Use cases:**
- Control subagent lifecycle
- Validate subagent results
- Coordinate multi-agent work
**Returns:** `decision` (approve/block)

### 9. Notification
Called for notification filtering.
**Use cases:**
- Route notifications
- Filter noise
- Aggregate alerts
- Custom notification handling

### 10. PreCompact
Called before compacting conversation history.
**Use cases:**
- Archive full history
- Extract key information
- Prepare for compaction
- Generate summaries

## Hook Matchers

### Exact Match
```json
{
  "matcher": "Write"
}
```
Matches exactly "Write" tool.

### Wildcard
```json
{
  "matcher": "*"
}
```
Matches all tools/events.

### Regex Pattern
```json
{
  "matcher": "Edit|Write"
}
```
Matches tools containing "Edit" or "Write".

### MCP Tools
```json
{
  "matcher": "mcp__.*"
}
```
Matches all MCP server tools (format: `mcp__server__tool`).

## Configuration Format

Configuration can be placed in either location (priority order):
1. `.claude/settings.json` (amplihack standard, preferred)
2. `.claude/hooks/config.json` (legacy location, still supported)

Example configuration:

```json
{
  "SessionStart": [
    {
      "matcher": "*",
      "hooks": [
        {
          "type": "command",
          "command": "source $CLAUDE_ENV_FILE",
          "timeout": 60000
        }
      ]
    }
  ],
  "PreToolUse": [
    {
      "matcher": "Bash|Write",
      "hooks": [
        {
          "type": "prompt",
          "timeout": 60000
        },
        {
          "type": "command",
          "command": "./validate_tool.sh",
          "timeout": 30000
        }
      ]
    }
  ],
  "PermissionRequest": [
    {
      "matcher": "Read|Glob|Grep",
      "hooks": [
        {
          "type": "command",
          "command": "echo '{\"decision\": \"approve\"}'",
          "timeout": 5000
        }
      ]
    }
  ],
  "Stop": [
    {
      "matcher": "*",
      "hooks": [
        {
          "type": "prompt",
          "timeout": 60000
        }
      ]
    }
  ]
}
```

## Hook Environment Variables

All hooks receive these environment variables:

- `CLAUDE_SESSION_ID` - Unique session identifier
- `CLAUDE_TRANSCRIPT_PATH` - Path to transcript file
- `CLAUDE_CWD` - Current working directory
- `CLAUDE_PERMISSION_MODE` - Permission mode (auto/manual)
- `CLAUDE_HOOK_EVENT` - Event name (SessionStart, PreToolUse, etc.)
- `CLAUDE_TOOL_NAME` - Tool being executed (for tool events)
- `CLAUDE_ENV_FILE` - Path to environment file for persistence

## Hook Output

### Exit Codes
- `0` - Success (continue execution)
- `1` - Non-blocking error (continue with warning)
- `2` - Blocking error (halt execution)

### JSON Output (Optional)

Hooks can output JSON to stdout for advanced control:

```json
{
  "continue": true,
  "permissionDecision": "allow",
  "decision": "approve",
  "additionalContext": "Custom context to inject"
}
```

### Fields

#### `continue` (boolean)
- `true` - Continue execution
- `false` - Stop execution

#### `permissionDecision` (PreToolUse hooks)
- `"allow"` - Allow tool execution
- `"deny"` - Block tool execution
- `"ask"` - Prompt user for decision

#### `decision` (Stop/SubagentStop/PermissionRequest hooks)
- `"approve"` - Approve completion or auto-approve permission
- `"block"` - Block completion
- `"deny"` - Deny permission (PermissionRequest only)

#### `additionalContext` (string)
- Custom text to inject into conversation context

## Usage Examples

### Basic Usage

```rust
use claude_code_cli::hooks::{HooksSystem, HookEvent, HookContext};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Create hooks system
    let mut hooks = HooksSystem::new();

    // Load configuration (searches for .claude/settings.json or .claude/hooks/config.json)
    hooks.load_from_file(".claude/settings.json").await?;

    // Create context
    let context = HookContext::for_session(
        "session-123".to_string(),
        "/tmp/transcript.log".to_string(),
        "/home/user".to_string(),
        "auto".to_string(),
        HookEvent::SessionStart,
    );

    // Execute hooks
    let results = hooks.execute_hooks(HookEvent::SessionStart, &context).await?;

    // Check results
    for result in results {
        if result.is_blocking() {
            eprintln!("Hook blocked execution: {}", result.stderr);
            return Err(anyhow::anyhow!("Blocked by hook"));
        }
    }

    Ok(())
}
```

### PreToolUse Permission Check

```rust
use claude_code_cli::hooks::{HooksSystem, HookEvent, HookContext};

async fn check_tool_permission(
    hooks: &HooksSystem,
    tool_name: &str,
) -> anyhow::Result<bool> {
    let context = HookContext::for_tool(
        "session-123".to_string(),
        "/tmp/transcript.log".to_string(),
        "/home/user".to_string(),
        "auto".to_string(),
        HookEvent::PreToolUse,
        tool_name.to_string(),
    );

    let results = hooks.execute_hooks(HookEvent::PreToolUse, &context).await?;

    for result in results {
        // Check exit code
        if result.is_blocking() {
            return Ok(false); // Denied
        }

        // Check JSON output
        if let Some(output) = result.parse_output() {
            if let Some(decision) = output.permission_decision {
                match decision {
                    PermissionDecision::Deny => return Ok(false),
                    PermissionDecision::Ask => {
                        // Prompt user
                        return Ok(prompt_user(tool_name)?);
                    }
                    PermissionDecision::Allow => return Ok(true),
                }
            }
        }
    }

    Ok(true) // Default: allow
}
```

### PermissionRequest Auto-Approval

```rust
use claude_code_cli::hooks::{HooksSystem, HookEvent, HookContext, types::PermissionRequestDecision};

async fn check_permission_request(
    hooks: &HooksSystem,
    tool_name: &str,
    tool_params: &serde_json::Value,
) -> anyhow::Result<Option<bool>> {
    let context = HookContext::for_permission_request(
        "session-123".to_string(),
        "/tmp/transcript.log".to_string(),
        "/home/user".to_string(),
        "ask".to_string(),
        tool_name.to_string(),
        Some(tool_params.clone()),
    );

    let results = hooks.execute_hooks(HookEvent::PermissionRequest, &context).await?;

    for result in results {
        if let Some(output) = result.parse_output() {
            if let Some(decision) = output.decision {
                match decision {
                    PermissionRequestDecision::Approve => return Ok(Some(true)),
                    PermissionRequestDecision::Deny => return Ok(Some(false)),
                }
            }
        }
    }

    Ok(None) // No hook decision - prompt user
}
```

### Stop Hook Completion Check

```rust
async fn check_completion(hooks: &HooksSystem) -> anyhow::Result<bool> {
    let context = HookContext::for_session(
        "session-123".to_string(),
        "/tmp/transcript.log".to_string(),
        "/home/user".to_string(),
        "auto".to_string(),
        HookEvent::Stop,
    );

    let results = hooks.execute_hooks(HookEvent::Stop, &context).await?;

    for result in results {
        if let Some(output) = result.parse_output() {
            if let Some(decision) = output.decision {
                match decision {
                    StopDecision::Block => return Ok(false),
                    StopDecision::Approve => return Ok(true),
                }
            }
        }
    }

    Ok(true) // Default: approve
}
```

## Features

### Parallel Execution
Multiple hooks for the same event execute in parallel for performance.

### Deduplication
Identical hooks (same command/type) are automatically deduplicated.

### Timeout Protection
All hooks have configurable timeouts (default: 60 seconds).

### Error Handling
- Non-blocking errors (exit 1) log warnings but continue
- Blocking errors (exit 2) halt execution immediately
- Timeout errors are treated as non-blocking

### Environment Persistence
Use `$CLAUDE_ENV_FILE` in SessionStart hooks to persist environment variables across bash commands.

## Testing

The system includes 74 comprehensive tests covering:

- Hook configuration and validation (9 tests)
- Lifecycle events (9 tests)
- Hook execution and output (13 tests)
- Configuration system (5 tests)
- Custom hook registration (4 tests)
- Boundary conditions (9 tests)
- Error handling (7 tests)
- Full workflow scenarios (8 tests)
- JSON configuration parsing (9 tests)
- Additional integration tests (1 test)

Run tests:
```bash
cargo test --test hooks_tests
```

## Integration Points

### CLI Tool Execution
```rust
// Before tool execution
let results = hooks.execute_hooks(HookEvent::PreToolUse, &context).await?;
for result in results {
    if result.is_blocking() {
        return Err(anyhow::anyhow!("Tool execution blocked by hook"));
    }
}

// Execute tool
let output = tool.execute(params).await?;

// After tool execution
hooks.execute_hooks(HookEvent::PostToolUse, &context).await?;
```

### Interactive Mode
```rust
// Session start
hooks.execute_hooks(HookEvent::SessionStart, &context).await?;

// Main loop
loop {
    let prompt = read_user_input()?;

    // User prompt submit
    hooks.execute_hooks(HookEvent::UserPromptSubmit, &context).await?;

    // Process prompt...
}

// Session end
hooks.execute_hooks(HookEvent::SessionEnd, &context).await?;
```

## Real-World Use Cases (Amplihack)

Amplihack uses hooks extensively:

### SessionStart Hook
```bash
#!/bin/bash
# Source environment and load project context
source $CLAUDE_ENV_FILE
export PROJECT_ROOT=$(pwd)
export AMPLIHACK_MODE=enabled
echo "Amplihack environment initialized"
```

### PreToolUse Hook
```bash
#!/bin/bash
# Validate dangerous operations
if [[ "$CLAUDE_TOOL_NAME" == "Bash" ]]; then
    # Check for destructive commands
    if echo "$TOOL_PARAMS" | grep -E "(rm -rf|dd|mkfs)"; then
        echo '{"permissionDecision": "ask", "additionalContext": "Destructive command detected"}'
        exit 0
    fi
fi
echo '{"permissionDecision": "allow"}'
```

### PermissionRequest Hook (Auto-approve safe tools)
```bash
#!/bin/bash
# Auto-approve read-only operations
if [[ "$CLAUDE_TOOL_NAME" == "Read" ]] || [[ "$CLAUDE_TOOL_NAME" == "Glob" ]] || [[ "$CLAUDE_TOOL_NAME" == "Grep" ]]; then
    echo '{"decision": "approve"}'
    exit 0
fi

# No decision - let user decide
exit 0
```

This hook auto-approves read-only tools without prompting the user.

### Stop Hook (Prompt-based)
```json
{
  "Stop": [
    {
      "matcher": "*",
      "hooks": [
        {
          "type": "prompt",
          "timeout": 60000
        }
      ]
    }
  ]
}
```

The prompt hook uses LLM to analyze if work is truly complete before stopping.

## Future Enhancements

- Full LLM integration for prompt hooks
- Hook chaining and dependencies
- Conditional execution based on context
- Hook metrics and analytics
- Remote hook execution
- Hook templates and presets
- Dynamic hook registration at runtime
- Hook debugging mode

## License

MIT OR Apache-2.0
