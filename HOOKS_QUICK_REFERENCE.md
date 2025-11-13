# Hooks Quick Reference

## 9 Lifecycle Events

| Event | When | Use Case |
|-------|------|----------|
| **SessionStart** | Session begins | Initialize environment, load configs |
| **SessionEnd** | Session ends | Cleanup, save state |
| **PreToolUse** | Before tool runs | Validate, permission control, modify params |
| **PostToolUse** | After tool runs | Log results, validate output |
| **UserPromptSubmit** | User sends prompt | Validate, inject context |
| **Stop** | Agent finishes | Decide if work is complete |
| **SubagentStop** | Subagent finishes | Control subagent flow |
| **Notification** | System notification | Filter/route notifications |
| **PreCompact** | Before compaction | Save history, prepare for compression |

## Hook Types

| Type | Execution | Best For |
|------|-----------|----------|
| **command** | Bash script | Deterministic logic, file operations |
| **prompt** | Claude Haiku | Context-aware decisions, AI analysis |

## Common Patterns

### Permission Control (PreToolUse)
```json
{
  "PreToolUse": [{
    "matcher": "Bash",
    "hooks": [{
      "type": "command",
      "command": "check_safe_command.sh",
      "timeout": 5000
    }]
  }]
}
```

### Intelligent Continuation (Stop)
```json
{
  "Stop": [{
    "matcher": "*",
    "hooks": [{
      "type": "prompt",
      "prompt": "Is this task complete? $ARGUMENTS"
    }]
  }]
}
```

### Session Initialization (SessionStart)
```json
{
  "SessionStart": [{
    "matcher": "startup",
    "hooks": [{
      "type": "command",
      "command": "export MY_VAR=value >> $CLAUDE_ENV_FILE"
    }]
  }]
}
```

## Matchers Cheat Sheet

| Pattern | Matches |
|---------|---------|
| `"Write"` | Only Write tool |
| `"*"` or `""` | All tools |
| `"Edit\|Write"` | Edit OR Write |
| `"Notebook.*"` | NotebookEdit, NotebookRun, etc. |
| `"mcp__.*"` | All MCP tools |
| `"mcp__memory__.*"` | All memory server tools |

## Exit Codes

| Code | Meaning | Effect |
|------|---------|--------|
| 0 | Success | Continue execution |
| 2 | Blocking | Feed stderr to Claude |
| Other | Non-blocking error | Show stderr, continue |

## JSON Response Quick Reference

### PreToolUse
```json
{
  "permissionDecision": "allow|deny|ask",
  "permissionDecisionReason": "why",
  "hookSpecificOutput": {
    "updatedInput": { "modified": "params" }
  }
}
```

### Stop/SubagentStop
```json
{
  "decision": "approve|block",
  "reason": "explanation"
}
```

### PostToolUse/UserPromptSubmit
```json
{
  "decision": "block",
  "additionalContext": "inject this"
}
```

### All Events
```json
{
  "continue": false,
  "stopReason": "why stopped",
  "suppressOutput": true,
  "systemMessage": "warning to user"
}
```

## Environment Variables

| Variable | Available In | Purpose |
|----------|--------------|---------|
| `CLAUDE_PROJECT_DIR` | All | Project root path |
| `CLAUDE_CODE_REMOTE` | All | "true" if web |
| `CLAUDE_SESSION_ID` | All | Current session |
| `CLAUDE_TOOL_NAME` | Tool events | Which tool |
| `CLAUDE_ENV_FILE` | SessionStart only | Persist env vars |

## Common Commands

```bash
# Test hooks
cargo test --package rustyclawd-cli --test hooks_tests

# Build
cargo build --package rustyclawd-cli --lib

# Check specific event
cargo test --package rustyclawd-cli --test hooks_tests -- session_start
```

## File Locations

| File | Contains |
|------|----------|
| `~/.claude/settings.json` | User-level hooks |
| `.claude/settings.json` | Project hooks |
| `.claude/settings.local.json` | Local (uncommitted) |
| `crates/cli/src/hooks/` | Implementation |

## 5-Minute Setup

1. Create `.claude/settings.json`:
```json
{
  "hooks": {
    "PreToolUse": [{
      "matcher": "*",
      "hooks": [{
        "type": "command",
        "command": "echo 'Tool: $CLAUDE_TOOL_NAME'"
      }]
    }]
  }
}
```

2. Use in Rust:
```rust
use rustyclawd_cli::hooks::{HooksSystem, HookEvent, HookContext};

let mut hooks = HooksSystem::new();
hooks.load_from_file(".claude/settings.json").await?;
let results = hooks.execute_hooks(event, &context).await?;
```

## Troubleshooting

| Issue | Solution |
|-------|----------|
| Hook not firing | Check matcher pattern |
| Timeout | Increase timeout_ms |
| Permission denied | Check file permissions on command script |
| No output | Hook might be suppressed, check suppressOutput |
| Blocking when shouldn't | Check exit code (should be 0, not 2) |

## Best Practices

1. **Start Simple**: Begin with command hooks, add prompt hooks as needed
2. **Test Matchers**: Use specific matchers before wildcards
3. **Set Timeouts**: Always specify timeout for critical hooks
4. **Handle Errors**: Check exit codes in command hooks
5. **Document**: Add comments explaining complex hook logic
6. **Version Control**: Commit `.claude/settings.json`, ignore `.local.json`
7. **Security**: Validate all inputs in bash scripts (quote variables!)

## Next Steps

- See `HOOKS_IMPLEMENTATION_SUMMARY.md` for complete reference
- See `HOOKS_DELIVERY.md` for implementation details
- See `examples/comprehensive_hooks_example.json` for full example
- Read tests in `crates/cli/tests/hooks_tests.rs` for usage patterns
