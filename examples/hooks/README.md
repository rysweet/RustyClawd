# Hooks Examples

This directory contains example hook configurations and scripts for the Claude Code hooks system.

## Files

### `config.json`
Basic hooks configuration demonstrating all 9 lifecycle events:
- SessionStart: Log session start time
- SessionEnd: Log session end time
- PreToolUse: Validate tools before execution (Bash, Write/Edit, MCP tools)
- PostToolUse: Log after tool execution
- UserPromptSubmit: Track user prompts
- Stop: LLM-based completion check
- SubagentStop: LLM-based subagent control
- Notification: Handle notifications
- PreCompact: Prepare for history compaction

### `advanced_validation.sh`
Advanced PreToolUse hook script that:
- Detects dangerous commands (rm -rf, dd, mkfs, etc.)
- Blocks destructive operations
- Asks user permission for sudo commands
- Returns JSON output with permission decisions

### `session_init.sh`
SessionStart hook script that:
- Logs session information
- Sets up project environment variables
- Persists environment to `$CLAUDE_ENV_FILE`
- Can be sourced by subsequent bash commands

### `amplihack_example.json`
Real-world hooks configuration for Amplihack integration:
- Environment initialization with project context
- Multi-hook validation (LLM + script-based)
- Execution logging
- Transcript backup before compaction
- Demonstrates both command and prompt hook types

## Usage

### 1. Copy configuration to your project

```bash
# Option 1: Use amplihack standard location (preferred)
cp examples/hooks/config.json .claude/settings.json

# Option 2: Use legacy hooks directory location (still supported)
mkdir -p .claude/hooks
cp examples/hooks/config.json .claude/hooks/config.json

# For amplihack-specific config
cp examples/hooks/amplihack_example.json .claude/settings.json
```

### 2. Make scripts executable

```bash
chmod +x examples/hooks/*.sh
```

### 3. Set up environment file

The hooks system uses `$CLAUDE_ENV_FILE` for environment persistence:

```bash
# Set in your shell profile
export CLAUDE_ENV_FILE="$HOME/.claude/env.sh"
```

### 4. Test hooks

```bash
# Run with hooks enabled
claude-code chat

# The SessionStart hooks will run automatically
# PreToolUse hooks run before each tool
# PostToolUse hooks run after each tool
```

## Hook Configuration Format

Each event can have multiple hook configurations:

```json
{
  "EventName": [
    {
      "matcher": "ToolName|Pattern",
      "hooks": [
        {
          "type": "command",
          "command": "bash command here",
          "timeout": 60000
        },
        {
          "type": "prompt",
          "timeout": 60000
        }
      ]
    }
  ]
}
```

### Matcher Patterns

- `"*"` - Match all tools
- `"ToolName"` - Match specific tool
- `"Tool1|Tool2"` - Match multiple tools (regex alternation)
- `"mcp__.*"` - Match all MCP tools
- `"prefix.*"` - Match tools with prefix

### Hook Types

- `command` - Execute bash command with environment variables
- `prompt` - Execute LLM-based analysis (requires LLM integration)

### Timeout

Timeout in milliseconds (default: 60000 = 60 seconds)

## Environment Variables

All hooks receive:

- `CLAUDE_SESSION_ID` - Session identifier
- `CLAUDE_TRANSCRIPT_PATH` - Transcript file path
- `CLAUDE_CWD` - Current working directory
- `CLAUDE_PERMISSION_MODE` - Permission mode
- `CLAUDE_HOOK_EVENT` - Event name
- `CLAUDE_TOOL_NAME` - Tool being executed (for tool events)
- `CLAUDE_ENV_FILE` - Environment persistence file

## Exit Codes

- `0` - Success (continue)
- `1` - Non-blocking error (warning)
- `2` - Blocking error (halt)

## JSON Output

Hooks can output JSON for advanced control:

```json
{
  "continue": true,
  "permissionDecision": "allow",
  "decision": "approve",
  "additionalContext": "Context to inject"
}
```

## Real-World Scenarios

### Scenario 1: Security Validation

Use PreToolUse hooks to validate dangerous operations:

```json
{
  "PreToolUse": [
    {
      "matcher": "Bash",
      "hooks": [
        {
          "type": "command",
          "command": "./validate_command.sh",
          "timeout": 30000
        }
      ]
    }
  ]
}
```

### Scenario 2: Execution Logging

Use PostToolUse hooks to log all commands:

```json
{
  "PostToolUse": [
    {
      "matcher": "*",
      "hooks": [
        {
          "type": "command",
          "command": "echo \"$(date): $CLAUDE_TOOL_NAME\" >> .claude/audit.log",
          "timeout": 5000
        }
      ]
    }
  ]
}
```

### Scenario 3: Smart Completion

Use Stop hooks to verify work is complete:

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

The LLM analyzes the conversation and decides if work is truly complete.

### Scenario 4: Environment Persistence

Use SessionStart to set up persistent environment:

```json
{
  "SessionStart": [
    {
      "matcher": "*",
      "hooks": [
        {
          "type": "command",
          "command": "source $CLAUDE_ENV_FILE && export PROJECT_ROOT=$(pwd)",
          "timeout": 60000
        }
      ]
    }
  ]
}
```

All subsequent Bash commands will have access to these variables.

## Testing Your Hooks

Test hooks independently:

```bash
# Set up environment
export CLAUDE_SESSION_ID="test-123"
export CLAUDE_CWD="$(pwd)"
export CLAUDE_TRANSCRIPT_PATH="/tmp/test-transcript.log"
export CLAUDE_PERMISSION_MODE="auto"
export CLAUDE_HOOK_EVENT="SessionStart"
export CLAUDE_ENV_FILE="/tmp/claude-env.sh"

# Test a hook script
./examples/hooks/session_init.sh

# Test validation hook
export CLAUDE_TOOL_NAME="Bash"
./examples/hooks/advanced_validation.sh
```

## Troubleshooting

### Hooks not running
- Check that `.claude/settings.json` or `.claude/hooks/config.json` exists
- Verify JSON syntax with `jq . .claude/settings.json` (or `.claude/hooks/config.json`)
- Check hook script permissions

### Hook timeouts
- Increase timeout in configuration
- Optimize hook scripts
- Use background processes for long-running tasks

### Permission denied
- Make hook scripts executable: `chmod +x script.sh`
- Check script shebang line: `#!/bin/bash`
- Verify file paths are correct

## Best Practices

1. **Keep hooks fast** - Use short timeouts and optimize scripts
2. **Fail gracefully** - Return exit code 0 with JSON for control flow
3. **Log errors** - Use stderr for error messages
4. **Test independently** - Test hooks as standalone scripts
5. **Use environment persistence** - Leverage `$CLAUDE_ENV_FILE`
6. **Validate inputs** - Check environment variables exist
7. **Document behavior** - Add comments to configuration
8. **Version control** - Track hooks in git

## Contributing

Feel free to contribute more example hooks!

Examples we'd love to see:
- Git commit hooks
- Test execution hooks
- Deployment validation
- API rate limiting
- Custom notification routing
- Multi-language project setup
