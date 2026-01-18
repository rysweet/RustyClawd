# Hook Lifecycle Integration Guide

## Overview

RustyClawd Claude Code implements a comprehensive hook lifecycle system that enables custom event handling at critical points throughout the CLI session and tool execution. This system allows developers, DevOps engineers, and power users to inject custom logic into the Claude Code workflow without modifying the core application.

### The Six Core Hook Events

The implementation provides six primary hook integration points:

1. **UserPromptSubmit** - Fires when a user submits a prompt
2. **PreToolUse** - Fires before a tool is executed (can block execution)
3. **PostToolUse** - Fires after a tool completes execution
4. **Stop** - Fires when checking if work is complete (can approve/block exit)
5. **SubagentStop** - Fires when a subagent signals completion (can approve/block subagent exit)
6. **PermissionRequest** - Fires when a tool requires permission and user would be prompted (can auto-approve/deny)

Additional hooks (SessionStart, SessionEnd, Notification, PreCompact) provide session and notification lifecycle control.

### Why Hooks Matter

Hooks enable powerful automation patterns:

- **Security & Compliance**: Block dangerous tool usage, audit execution, enforce organizational policies
- **Validation**: Validate user input before Claude processes it, validate tool parameters before execution
- **Enrichment**: Add context to prompts, enrich tool execution with metadata
- **Notification**: Integrate with external systems (Slack, monitoring platforms, logging systems)
- **Custom Workflows**: Implement domain-specific logic without modifying Claude Code

---

## Hook Descriptions

### UserPromptSubmit

**Fires**: When the user submits a prompt in the interactive CLI

**Context Provided**:
- `session_id` - Unique session identifier
- `transcript_path` - Path to the session transcript
- `cwd` - Current working directory
- `user_prompt` - The exact text the user submitted
- `hook_event_name` - "UserPromptSubmit"

**Blocking Model**: Non-blocking by default. Returning `continue: false` stops the prompt from being processed.

**Use Cases**:
- Validate prompt content (block malicious patterns, PII, secrets)
- Enrich prompts with additional context
- Route prompts to different models based on content
- Log all user input for audit trails
- Integrate with custom prompt validation systems

**Example Hook Output**:
```json
{
  "continue": true,
  "systemMessage": "Prompt validated and enhanced with project context"
}
```

---

### PreToolUse

**Fires**: Immediately before Claude Code executes a tool

**Context Provided**:
- `session_id` - Unique session identifier
- `transcript_path` - Path to the session transcript
- `cwd` - Current working directory
- `tool_name` - Name of the tool being executed (e.g., "Bash", "Read", "Write")
- `tool_params` - Complete parameters as JSON object
- `hook_event_name` - "PreToolUse"

**Blocking Model**: Three-way decision system:
- `allow` - Execute the tool immediately
- `deny` - Block execution and return error to Claude
- `ask` - Prompt the user to approve/deny the tool

**Permission Decision Logic**:
1. Hook executes and returns decision
2. If `ask`: User is presented with tool execution request
3. If user approves: Tool executes
4. If user denies or hook returns `deny`: Tool is blocked and Claude receives error

**Use Cases**:
- Restrict dangerous tool combinations (e.g., block Bash during specific conditions)
- Implement role-based access control (RBAC) for tools
- Add interactive approval workflows for sensitive operations
- Transform tool parameters for security/safety (sanitize file paths, etc.)
- Log all tool execution for compliance
- Rate-limit tool usage

**Example Hook Output - Allow**:
```json
{
  "permissionDecision": "allow",
  "permissionDecisionReason": "Bash execution allowed for build task"
}
```

**Example Hook Output - Ask**:
```json
{
  "permissionDecision": "ask",
  "permissionDecisionReason": "Requesting approval for file write operation"
}
```

**Example Hook Output - Deny**:
```json
{
  "permissionDecision": "deny",
  "permissionDecisionReason": "File paths outside project root are not permitted",
  "systemMessage": "This operation is blocked by policy"
}
```

**Example Hook Output - Transform Parameters**:
```json
{
  "permissionDecision": "allow",
  "hookSpecificOutput": {
    "updatedInput": {
      "command": "echo 'sanitized command'"
    }
  }
}
```

---

### PostToolUse

**Fires**: After a tool completes execution, before the result returns to Claude

**Context Provided**:
- `session_id` - Unique session identifier
- `transcript_path` - Path to the session transcript
- `cwd` - Current working directory
- `tool_name` - Name of the tool that executed
- `tool_params` - The parameters that were used
- `tool_result` - The complete result from tool execution (stdout, stderr, status)
- `hook_event_name` - "PostToolUse"

**Blocking Model**: Non-blocking inspection by default. Returning `continue: false` stops further processing.

**Use Cases**:
- Filter or sanitize tool output before Claude sees it (remove secrets, PII)
- Validate tool results for correctness or compliance
- Enrich results with additional context
- Send tool execution events to external systems (monitoring, logging)
- Suppress output from transcript (for sensitive operations)
- Transform results for better Claude understanding

**Example Hook Output**:
```json
{
  "continue": true,
  "suppressOutput": false,
  "additionalContext": "Note: This deployment succeeded with 5 warnings"
}
```

**Example Hook Output - Sanitize**:
```json
{
  "continue": true,
  "suppressOutput": false,
  "hookSpecificOutput": {
    "updatedInput": {
      "stdout": "API_KEY=***REDACTED***"
    }
  }
}
```

---

### Stop

**Fires**: When the user requests to stop/exit, or Claude signals completion

**Context Provided**:
- `session_id` - Unique session identifier
- `transcript_path` - Path to the session transcript
- `cwd` - Current working directory
- `hook_event_name` - "Stop"

**Blocking Model**: Two-way decision system:
- `approve` - Allow the stop/exit to proceed
- `block` - Prevent the stop/exit and continue the session

**Stop Decision Logic**:
1. Hook executes and returns decision
2. If `approve`: Session terminates normally
3. If `block`: User is notified, session continues
4. If multiple hooks: First `block` decision prevents exit

**Use Cases**:
- Enforce completion criteria (ensure all tests pass before exit)
- Cleanup validation (verify logs/artifacts are preserved before exit)
- Interactive confirmation for critical workflows
- Ensure session state is properly saved
- Prevent accidental exits during sensitive operations
- Track session completion for analytics

**Example Hook Output - Approve**:
```json
{
  "decision": "approve",
  "reason": "All validation checks passed"
}
```

**Example Hook Output - Block**:
```json
{
  "decision": "block",
  "reason": "Unit tests are still running. Cannot stop now.",
  "systemMessage": "Wait for tests to complete before exiting"
}
```

---

### SubagentStop

**Fires**: When a subagent signals that it wants to stop

**Context Provided**:
- `session_id` - Unique session identifier
- `transcript_path` - Path to the session transcript
- `cwd` - Current working directory
- `hook_event_name` - "SubagentStop"

**Blocking Model**: Two-way decision system (same as Stop):
- `approve` - Allow the subagent to stop
- `block` - Force the subagent to continue

**SubagentStop Decision Logic**:
1. Subagent signals completion
2. Hook executes before stopping subagent
3. If `approve`: Subagent terminates and control returns to parent
4. If `block`: Subagent continues executing
5. Multiple hooks: First `block` prevents stop

**Use Cases**:
- Enforce subagent output quality requirements
- Prevent premature subagent termination
- Implement subagent task validation
- Coordinate between parent and subagent workflows
- Ensure subagent state consistency
- Track subagent lifecycle for debugging

**Example Hook Output**:
```json
{
  "decision": "approve",
  "reason": "Subagent completed all assigned tasks"
}
```

---

### PermissionRequest

**Fires**: When a tool requires permission and user would be prompted for approval

**Context Provided**:
- `session_id` - Unique session identifier
- `transcript_path` - Path to the session transcript
- `cwd` - Current working directory
- `tool_name` - Name of the tool requesting permission (e.g., "Bash", "Write", "Edit")
- `tool_use_id` - Unique identifier for this tool invocation
- `tool_params` - Complete parameters as JSON object
- `hook_event_name` - "PermissionRequest"

**Blocking Model**: Three-way decision system:
- `allow` - Auto-approve the tool execution without user prompt
- `deny` - Block execution and return error to Claude
- `ask` - Fall through to normal interactive prompt (default if no hook configured)

**PermissionRequest Decision Logic**:
1. Tool execution requires permission (e.g., permission_mode is "ask")
2. Before showing user the permission prompt, PermissionRequest hook fires
3. Hook returns decision: allow, deny, or ask
4. If `allow`: Tool executes immediately without user interaction
5. If `deny`: Tool is blocked and Claude receives error message
6. If `ask` or hook not configured: Normal permission prompt is shown to user

**Use Cases**:
- Auto-approve safe commands based on patterns (e.g., read-only operations)
- Auto-deny dangerous commands (e.g., rm -rf, network access to unknown hosts)
- Implement custom security policies (RBAC, allowlists/blocklists)
- Speed up automated workflows by bypassing interactive prompts
- Integrate with external approval systems (ticket systems, security scanners)
- Log permission requests for audit trails

**Example Hook Output - Allow**:
```json
{
  "permissionDecision": "allow",
  "permissionDecisionReason": "Read-only file operation - auto-approved by policy"
}
```

**Example Hook Output - Deny**:
```json
{
  "permissionDecision": "deny",
  "permissionDecisionReason": "Network access to external host blocked by security policy",
  "systemMessage": "This operation is not permitted by your organization's security policy"
}
```

**Example Hook Output - Ask**:
```json
{
  "permissionDecision": "ask",
  "permissionDecisionReason": "Unrecognized operation - requires manual review"
}
```

**Example Configuration**:
```json
{
  "PermissionRequest": [
    {
      "matcher": "Bash",
      "hooks": [
        {
          "type": "command",
          "command": "scripts/auto-approve-bash.sh",
          "timeout": 5000
        }
      ]
    },
    {
      "matcher": "Write|Edit",
      "hooks": [
        {
          "type": "command",
          "command": "scripts/check-file-permissions.sh",
          "timeout": 3000
        }
      ]
    }
  ]
}
```

**Difference from PreToolUse**:
- **PreToolUse** fires for EVERY tool execution, regardless of permission mode
- **PermissionRequest** fires ONLY when a permission prompt would be shown
- Use PreToolUse for general validation/transformation/logging of all tools
- Use PermissionRequest specifically for automating permission decisions

---

## Configuration

### Configuration File Location

RustyClawd looks for hook configuration in this priority order:

1. `.claude/settings.json` - Modern standard location (recommended)
2. `.claude/hooks/config.json` - Legacy location (supported for backward compatibility)

The configuration loader automatically walks up parent directories to find configuration.

### Configuration Format

Hooks are configured using JSON with the following structure:

```json
{
  "SessionStart": [...],
  "SessionEnd": [...],
  "PreToolUse": [...],
  "PostToolUse": [...],
  "UserPromptSubmit": [...],
  "Stop": [...],
  "SubagentStop": [...],
  "Notification": [...],
  "PreCompact": [...],
  "PermissionRequest": [...]
}
```

Each event array contains hook configurations with matchers and hook definitions.

### Basic Configuration

A basic hook configuration for a single event:

```json
{
  "UserPromptSubmit": [
    {
      "matcher": "*",
      "hooks": [
        {
          "type": "command",
          "command": "scripts/validate-prompt.sh",
          "timeout": 5000
        }
      ]
    }
  ]
}
```

---

## Configuration Examples

### Example 1: Basic Validation Hook

Validate bash commands before execution:

```json
{
  "PreToolUse": [
    {
      "matcher": "Bash",
      "hooks": [
        {
          "type": "command",
          "command": "scripts/validate-bash-command.sh",
          "timeout": 5000
        }
      ]
    }
  ]
}
```

Validation script (`scripts/validate-bash-command.sh`):
```bash
#!/bin/bash
set -e

# Parse the tool parameters from environment
tool_params="$CLAUDE_HOOK_TOOL_PARAMS"

# Check for dangerous patterns
if echo "$tool_params" | grep -E "rm -rf|:(){:|&" > /dev/null; then
  cat <<EOF
{
  "permissionDecision": "ask",
  "permissionDecisionReason": "Potentially dangerous command detected"
}
EOF
else
  cat <<EOF
{
  "permissionDecision": "allow"
}
EOF
fi
```

### Example 2: Multi-Tool Permission System

Different rules for different tools:

```json
{
  "PreToolUse": [
    {
      "matcher": "Write|Edit",
      "hooks": [
        {
          "type": "command",
          "command": "scripts/validate-file-write.sh",
          "timeout": 3000
        }
      ]
    },
    {
      "matcher": "Bash",
      "hooks": [
        {
          "type": "command",
          "command": "scripts/validate-bash.sh",
          "timeout": 5000
        }
      ]
    },
    {
      "matcher": "mcp__.*",
      "hooks": [
        {
          "type": "command",
          "command": "scripts/validate-mcp-tool.sh",
          "timeout": 3000
        }
      ]
    }
  ]
}
```

### Example 3: LLM-Powered Decision Hooks

Use Claude itself to make decisions:

```json
{
  "PreToolUse": [
    {
      "matcher": "Bash",
      "hooks": [
        {
          "type": "prompt",
          "timeout": 10000
        }
      ]
    }
  ]
}
```

The LLM hook automatically receives context as JSON and responds with decisions. Default prompt template is provided, or use custom:

```json
{
  "PreToolUse": [
    {
      "matcher": "*",
      "hooks": [
        {
          "type": "prompt",
          "prompt": "Analyze this tool execution request and respond with approval: $ARGUMENTS",
          "timeout": 10000
        }
      ]
    }
  ]
}
```

### Example 4: Audit and Logging

Log all tool usage to external system:

```json
{
  "PostToolUse": [
    {
      "matcher": "*",
      "hooks": [
        {
          "type": "command",
          "command": "scripts/send-to-logging-service.sh",
          "timeout": 5000
        }
      ]
    }
  ]
}
```

Script example:
```bash
#!/bin/bash

# Extract tool information from environment
TOOL_NAME="$CLAUDE_TOOL_NAME"
SESSION_ID="$CLAUDE_SESSION_ID"
TIMESTAMP=$(date -u +"%Y-%m-%dT%H:%M:%SZ")

# Send to external logging service
curl -s -X POST "https://logging.internal/api/events" \
  -H "Content-Type: application/json" \
  -d "{
    \"event_type\": \"tool_execution\",
    \"tool_name\": \"$TOOL_NAME\",
    \"session_id\": \"$SESSION_ID\",
    \"timestamp\": \"$TIMESTAMP\",
    \"user\": \"$(whoami)\"
  }" > /dev/null

# Return success to continue
echo '{"continue": true}'
```

### Example 5: Output Sanitization

Remove secrets from tool output:

```json
{
  "PostToolUse": [
    {
      "matcher": "*",
      "hooks": [
        {
          "type": "command",
          "command": "scripts/sanitize-output.sh",
          "timeout": 3000
        }
      ]
    }
  ]
}
```

Script example:
```bash
#!/bin/bash

# Read tool output from stdin (passed via CLAUDE_TOOL_RESULT env var)
# In practice, this is provided through hook context

output="$CLAUDE_TOOL_RESULT"

# Redact common secret patterns
redacted=$(echo "$output" | \
  sed -E 's/(API_KEY|TOKEN|PASSWORD)=[^ ]*/\1=***REDACTED***/g')

cat <<EOF
{
  "continue": true,
  "suppressOutput": false,
  "additionalContext": "Output has been sanitized for security"
}
EOF
```

### Example 6: Stop Approval Workflow

Require confirmation before exiting:

```json
{
  "Stop": [
    {
      "matcher": "*",
      "hooks": [
        {
          "type": "command",
          "command": "scripts/validate-stop.sh",
          "timeout": 30000
        }
      ]
    }
  ]
}
```

Script example:
```bash
#!/bin/bash

# Check if all critical tasks are complete
if ! test -f ".workflow/all-tests-passed"; then
  cat <<EOF
{
  "decision": "block",
  "reason": "Tests have not completed successfully",
  "systemMessage": "Run 'npm test' before exiting"
}
EOF
  exit 0
fi

# Check build artifacts
if ! test -f "dist/bundle.js"; then
  cat <<EOF
{
  "decision": "block",
  "reason": "Build has not completed",
  "systemMessage": "Run 'npm run build' before exiting"
}
EOF
  exit 0
fi

# All checks passed
cat <<EOF
{
  "decision": "approve",
  "reason": "All workflow checks passed"
}
EOF
```

### Example 7: Permission Matrix with Role-Based Access

```json
{
  "PreToolUse": [
    {
      "matcher": "*",
      "hooks": [
        {
          "type": "command",
          "command": "scripts/rbac-check.sh",
          "timeout": 3000
        }
      ]
    }
  ]
}
```

Script example:
```bash
#!/bin/bash

TOOL_NAME="$CLAUDE_TOOL_NAME"
USER_ROLE=$(cat ~/.claude-config/user-role)

# Load permission matrix
declare -A PERMISSIONS
PERMISSIONS[admin]="*"
PERMISSIONS[developer]="Read|Edit|Bash"
PERMISSIONS[viewer]="Read"

allowed_tools="${PERMISSIONS[$USER_ROLE]}"

if [[ "$allowed_tools" == "*" ]]; then
  echo '{"permissionDecision": "allow"}'
  exit 0
fi

if echo "$TOOL_NAME" | grep -E "$allowed_tools" > /dev/null; then
  echo '{"permissionDecision": "allow"}'
else
  echo "{
    \"permissionDecision\": \"deny\",
    \"permissionDecisionReason\": \"Tool not allowed for role: $USER_ROLE\"
  }"
fi
```

---

## Integration Points

### Where Hooks Fire in the CLI

#### UserPromptSubmit Hook
- **Location**: User input processing loop (`cli/src/main.rs`)
- **Execution Phase**: After user input is received, before prompt is sent to Claude
- **Result Handling**: Non-blocking by default; `continue: false` stops processing

#### PreToolUse Hook
- **Location**: Tool execution engine (`cli/src/tool_executor.rs`)
- **Execution Phase**: After Claude selects a tool, before tool spawns
- **Result Handling**: Blocking (allow/deny/ask); determines tool execution path
- **User Interaction**: If `ask` decision, user is prompted with tool details

#### PostToolUse Hook
- **Location**: Tool execution engine (`cli/src/tool_executor.rs`)
- **Execution Phase**: After tool completes and output is captured
- **Result Handling**: Non-blocking inspection; can suppress/enrich output
- **Output Stream**: Affects what Claude sees in results

#### Stop Hook
- **Location**: Session termination handler (`cli/src/session.rs`)
- **Execution Phase**: When user requests exit or Claude signals completion
- **Result Handling**: Blocking (approve/block); prevents/allows session exit
- **Session State**: Fires before session cleanup

#### SubagentStop Hook
- **Location**: Subagent coordinator (`cli/src/subagent.rs`)
- **Execution Phase**: When subagent signals completion
- **Result Handling**: Blocking (approve/block); controls subagent lifecycle
- **Coordination**: Fires before subagent is terminated

### Hook Execution Model

All hooks follow this execution pattern:

1. **Event Trigger**: CLI event occurs (user input, tool execution, etc.)
2. **Context Creation**: Hook context is built with available information
3. **Matcher Selection**: Hooks are selected based on tool name matcher
4. **Parallel Execution**: All matching hooks for the event execute in parallel
5. **Decision Processing**: Hook results are aggregated and interpreted
6. **Action Taken**: Based on accumulated decisions, action is taken

---

## Permission Model Details

### PreToolUse Permission Decisions

#### Allow Decision

**Meaning**: Tool execution is permitted to proceed immediately.

**Response Format**:
```json
{
  "permissionDecision": "allow",
  "permissionDecisionReason": "Tool use is approved by policy",
  "systemMessage": "Executing Read tool"
}
```

**Behavior**:
- Tool executes immediately with original parameters
- Claude sees successful execution
- Optional `systemMessage` is shown to user

#### Deny Decision

**Meaning**: Tool execution is blocked entirely.

**Response Format**:
```json
{
  "permissionDecision": "deny",
  "permissionDecisionReason": "File path outside project root",
  "systemMessage": "This operation violates project boundaries"
}
```

**Behavior**:
- Tool does NOT execute
- Claude receives error message with reason
- Claude can retry with different tool or parameters
- User sees system message about why it was blocked

#### Ask Decision

**Meaning**: User approval is required before executing the tool.

**Response Format**:
```json
{
  "permissionDecision": "ask",
  "permissionDecisionReason": "File write operations require approval",
  "systemMessage": "Please review this operation"
}
```

**Behavior**:
- User is presented with tool execution request
- Request includes tool name, parameters, and system message
- User can approve or deny
- If approved: Tool executes with original parameters
- If denied: Claude receives error message
- System message provides context to user

### Permission Decision Aggregation

When multiple hooks return decisions:

1. **First Deny wins**: If any hook returns `deny`, execution is blocked
2. **Any Ask becomes Ask**: If any hook returns `ask` and none denied, user is prompted
3. **All Allow proceeds**: If all hooks return `allow`, execution proceeds
4. **Exit Code Interpretation**:
   - Exit code 0 or JSON with decision: Processed normally
   - Exit code 1: Non-blocking error (logged, execution continues)
   - Exit code 2: Blocking error (execution blocked)

---

## Exit Control Details

### Stop Hook Approval System

#### Approve Decision

**Meaning**: The session can terminate normally.

**Response Format**:
```json
{
  "decision": "approve",
  "reason": "All tasks completed successfully",
  "additionalContext": "Generated 3 test reports"
}
```

**Behavior**:
- Session termination proceeds
- Any `additionalContext` is logged
- Cleanup procedures run
- Session ends normally

#### Block Decision

**Meaning**: The session must continue; termination is prevented.

**Response Format**:
```json
{
  "decision": "block",
  "reason": "Tests are still running",
  "systemMessage": "Session cannot stop: tests in progress"
}
```

**Behavior**:
- Termination request is cancelled
- Session continues running
- User sees system message
- Claude can continue work or user can force exit with signal

### Multiple Hook Decisions

When multiple hooks return decisions on Stop/SubagentStop:

1. **First Block wins**: If ANY hook returns `block`, stop is denied
2. **All must Approve**: ALL hooks must return `approve` to allow stop
3. **Blocking takes priority**: `block` decision overrides all `approve` decisions

---

## Security Considerations

### ⚠️ CRITICAL: Command Injection Risks

**Hook scripts receive user-controlled data through environment variables.** If your hook scripts use these variables unsafely, command injection attacks are possible.

#### Vulnerable Example (DO NOT USE):
```bash
#!/bin/bash
# DANGEROUS: Unquoted variable usage
eval "echo Processing: $CLAUDE_USER_PROMPT"
bash -c "log-prompt $CLAUDE_TOOL_PARAMS"
```

**Attack Scenario**:
```
User prompt: "; rm -rf /tmp/important; echo "
Hook executes: eval "echo Processing: ; rm -rf /tmp/important; echo "
Result: Code execution!
```

#### Safe Example (USE THIS):
```bash
#!/bin/bash
# SAFE: Properly quoted variables
echo "Processing: ${CLAUDE_USER_PROMPT}"
log-prompt "${CLAUDE_TOOL_PARAMS}"

# SAFER: Use base64-encoded context if available
if [ -n "$CLAUDE_USER_PROMPT_B64" ]; then
    decoded=$(echo "$CLAUDE_USER_PROMPT_B64" | base64 -d)
    echo "Processing: ${decoded}"
fi
```

#### Best Practices for Command Injection Prevention:
1. **Always quote environment variables** in shell scripts: `"$VAR"` not `$VAR`
2. **Avoid `eval`, `source`, or `bash -c`** with user-controlled data
3. **Use structured data** (parse JSON context instead of raw env vars)
4. **Validate and sanitize** all inputs before processing
5. **Prefer compiled languages** (Python, Node.js, Go) over shell scripts for complex logic

### 🔐 Information Disclosure Risks

**UserPromptSubmit and PostToolUse hooks receive sensitive data** that users may type or tools may return:

#### Sensitive Data Examples:
- API keys: `"Use API key sk-abc123xyz to connect"`
- Passwords: `"Login with password: MySecret123"`
- File contents: PostToolUse receives full `Read` tool outputs
- Database credentials: Tool parameters may contain connection strings

#### Protection Strategies:
```bash
#!/bin/bash
# Hook: UserPromptSubmit - Log prompts with redaction

PROMPT="${CLAUDE_USER_PROMPT}"

# Redact common secret patterns
REDACTED=$(echo "$PROMPT" | \
    sed -E 's/sk-[a-zA-Z0-9]{20,}/[REDACTED_API_KEY]/g' | \
    sed -E 's/ghp_[a-zA-Z0-9]{36}/[REDACTED_TOKEN]/g' | \
    sed -E 's/password[=: ]+[^ ]+/password=[REDACTED]/gi')

echo "$REDACTED" >> ~/.claude/prompt_audit.log
```

#### Recommendations:
1. **Never log raw prompts** to external services without redaction
2. **Implement pattern-based redaction** for API keys, tokens, passwords
3. **Minimize data retention** - rotate and purge logs regularly
4. **Secure hook outputs** - ensure log files have appropriate permissions (chmod 600)
5. **Audit your hooks** - review what data they capture and where it goes

### 🚨 Fail-Open Security Model

**RustyClawd hooks use a fail-open model**: if a hook crashes or times out, execution continues.

#### Behavior by Hook Type:
| Hook | Fail Behavior | Rationale |
|------|---------------|-----------|
| **PreToolUse** | Fail-open (tool executes) | Availability over security |
| **UserPromptSubmit** | Fail-open (prompt processes) | Avoid blocking user workflow |
| **PostToolUse** | Fail-open (tool result used) | Already executed, logging optional |
| **Stop** | Fail-open (session ends) | Always allow clean shutdown |

#### Security Implications:
```yaml
# You configure a security hook:
PreToolUse:
  - matcher: "Bash"
    hooks:
      - type: "command"
        command: "/opt/security/dangerous-command-scanner.sh"

# If the hook crashes, times out, or returns exit code 1:
# → Tool execution CONTINUES (fail-open)
# → Security check is BYPASSED

# Only exit code 2 blocks execution!
```

#### Recommendations:
1. **Test your hooks thoroughly** - ensure they're robust and don't crash
2. **Use exit code 2 for blocking** - exit 1 only warns, exit 2 blocks
3. **Monitor hook health** - track failure rates to detect bypasses
4. **Defense in depth** - don't rely solely on hooks for security
5. **Minimize timeout risks** - keep hooks fast (<1 second ideal)

### 🛡️ Exit Code Semantics (CRITICAL)

**Only exit code 2 blocks execution!** This is a common source of confusion.

```bash
#!/bin/bash
# Hook: PreToolUse - Block dangerous commands

if [[ "$CLAUDE_TOOL_NAME" == "Bash" ]]; then
    if echo "$CLAUDE_TOOL_PARAMS" | grep -q "rm -rf"; then
        echo "ERROR: Dangerous command detected!" >&2
        exit 1  # ❌ WRONG: This only logs a warning, doesn't block!
    fi
fi
```

**Correct Usage:**
```bash
#!/bin/bash
if echo "$CLAUDE_TOOL_PARAMS" | grep -q "rm -rf"; then
    echo '{"permissionDecision": "deny", "permissionDecisionReason": "Dangerous rm -rf detected"}' >&1
    exit 2  # ✅ CORRECT: This blocks execution
fi
```

#### Exit Code Reference:
- **0**: Success - allow operation
- **1**: Non-blocking error - log warning but continue
- **2**: Blocking error - deny operation (PreToolUse/UserPromptSubmit only)

### 📁 Path Traversal Risks

**Hooks execute in the working directory** specified by `$CLAUDE_CWD`. Malicious paths could escape intended directories.

#### Protection:
```bash
#!/bin/bash
# Validate CWD is within project boundaries
CWD=$(realpath "$CLAUDE_CWD" 2>/dev/null || echo "$CLAUDE_CWD")

if [[ "$CWD" != /home/user/projects/* ]]; then
    echo "ERROR: CWD outside allowed directory" >&2
    exit 2
fi

cd "$CWD" || exit 2
```

### ⏱️ Timeout and Resource Management

**Default hook timeout: 60 seconds.** Long-running hooks can cause:
- User experience degradation (tool waits for hook)
- Resource exhaustion (parallel hooks consume CPU/memory)
- Denial of service (intentional or accidental)

#### Best Practices:
```json
{
  "hooks": {
    "PreToolUse": [{
      "matcher": "Bash",
      "hooks": [{
        "type": "command",
        "command": "/usr/local/bin/quick-security-check.sh",
        "timeout": 5000  // 5 seconds (override 60s default)
      }]
    }]
  }
}
```

1. **Keep hooks fast**: Aim for <1 second, never >5 seconds
2. **Set explicit timeouts**: Override 60s default for critical hooks
3. **Avoid blocking I/O**: Don't wait for network requests or user input
4. **Test performance**: Measure hook execution time under load

### 🔒 Hook Configuration Security

**Hooks are configured in `.claude/settings.json` or `.claude/hooks.json`.** Protect these files:

```bash
# Secure hook configuration files
chmod 600 ~/.claude/settings.json
chmod 600 ~/.claude/hooks.json

# Secure hook scripts
chmod 700 ~/.claude/hooks/*.sh
```

#### Configuration Threats:
1. **Malicious user with write access** can add hooks that exfiltrate data
2. **Compromised editor/IDE** could modify hook configuration
3. **Supply chain attacks** through shared hook scripts

#### Mitigations:
1. **File permissions**: Restrict write access to configuration files
2. **Code review**: Audit hook scripts before deployment
3. **Integrity monitoring**: Track changes to `.claude/` directory
4. **Principle of least privilege**: Hooks run with your user permissions—limit what your account can do

### 📝 Security Checklist for Hook Developers

Before deploying a hook to production:

- [ ] **Input validation**: All user-controlled data is validated/sanitized
- [ ] **Proper quoting**: All environment variables are quoted in shell scripts
- [ ] **Exit codes**: Blocking behavior uses exit 2, warnings use exit 1
- [ ] **Timeout configured**: Explicit timeout set (<5 seconds ideal)
- [ ] **Error handling**: Hook fails gracefully (doesn't crash/hang)
- [ ] **Secrets redacted**: Sensitive data is filtered before logging
- [ ] **File permissions**: Hook scripts are chmod 700, configs are chmod 600
- [ ] **Tested failure modes**: Hook behavior verified when it times out/crashes
- [ ] **No external dependencies**: Or dependencies are vendored/validated
- [ ] **Audit logging**: Security decisions are logged for review

---

## Hook Development Guide

### Command Hook Development

Command hooks are executable scripts (bash, Python, Node.js, etc.) that:

1. **Receive Context**: Via environment variables
2. **Process Decision**: Logic to determine response
3. **Output Decision**: JSON to stdout
4. **Exit with Status**:
   - Exit 0: Success (hook output processed)
   - Exit 1: Non-blocking error (warning logged)
   - Exit 2: Blocking error (operation blocked)

#### Environment Variables Provided

All command hooks receive:
- `CLAUDE_SESSION_ID` - Unique session identifier
- `CLAUDE_TRANSCRIPT_PATH` - Path to transcript file
- `CLAUDE_CWD` - Current working directory
- `CLAUDE_PERMISSION_MODE` - Permission mode (auto/ask/always)
- `CLAUDE_HOOK_EVENT` - Event name (e.g., "PreToolUse")
- `CLAUDE_TOOL_NAME` - Tool name (for tool-related events)
- `CLAUDE_PROJECT_DIR` - Project root directory

#### Hook Context as JSON

Full context is available through the hook output as JSON. For command hooks, you can parse environment or read stdin (if configured).

### Prompt Hook Development

Prompt hooks use Claude to make decisions. The hook system:

1. **Serializes Context**: Hook context is converted to JSON
2. **Builds Prompt**: Uses custom prompt or default template
3. **Calls Claude**: Makes LLM API call with context
4. **Parses Response**: Extracts JSON decision from response

#### Default Prompt Template

If no custom prompt is provided, this template is used:

```
You are a hook execution assistant for Claude Code CLI.

Event: [EVENT_NAME]
Tool: [TOOL_NAME]

Context:
[FULL_CONTEXT_JSON]

Please analyze this event and respond with a JSON decision in one of these formats:

For Stop/SubagentStop events:
{"decision": "approve"} or {"decision": "block", "reason": "explanation"}

For PreToolUse events:
{"permissionDecision": "allow"} or {"permissionDecision": "deny"} or {"permissionDecision": "ask"}

For PostToolUse events:
{"decision": "block", "additionalContext": "context"}} or {"continue": true}

For UserPromptSubmit events:
{"decision": "block", "additionalContext": "context"}} or {"continue": true}

For other events:
{"continue": true} or {"continue": false, "stopReason": "reason"}

You can also include optional fields:
- "systemMessage": "warning to show user"
- "suppressOutput": true (to hide from transcript)

Respond ONLY with the JSON decision, no other text.
```

#### Custom Prompt Example

```json
{
  "PreToolUse": [
    {
      "matcher": "Bash",
      "hooks": [
        {
          "type": "prompt",
          "prompt": "This is a bash command in a RustyClawd session. Context: $ARGUMENTS\n\nDetermine if this is safe to execute. Respond with {\"permissionDecision\": \"allow\"} or {\"permissionDecision\": \"deny\", \"permissionDecisionReason\": \"reason\"}",
          "timeout": 10000
        }
      ]
    }
  ]
}
```

### Testing Hooks

#### Manual Testing

1. **Create test hook**:
```bash
#!/bin/bash
echo '{"permissionDecision": "allow", "permissionDecisionReason": "test"}'
exit 0
```

2. **Configure in settings.json**:
```json
{
  "PreToolUse": [
    {
      "matcher": "Read",
      "hooks": [
        {
          "type": "command",
          "command": "./test-hook.sh",
          "timeout": 5000
        }
      ]
    }
  ]
}
```

3. **Run and observe**:
```bash
claude code
# In the session, try: "Read /some/file"
# Watch for hook execution in logs
```

#### Debug Output

Hooks produce debug output in:
- Hook executor logs: `~/.claude-code/logs/hooks.log`
- CLI output: When running in verbose mode (`claude code -v`)

Enable debug logging:
```json
{
  "debug": {
    "hooks": true,
    "logLevel": "debug"
  }
}
```

---

## Complete Configuration Examples

### Example: Complete Development Workflow

```json
{
  "SessionStart": [
    {
      "matcher": "*",
      "hooks": [
        {
          "type": "command",
          "command": "scripts/init-session.sh",
          "timeout": 10000
        }
      ]
    }
  ],
  "PreToolUse": [
    {
      "matcher": "Write|Edit",
      "hooks": [
        {
          "type": "command",
          "command": "scripts/validate-file-write.sh",
          "timeout": 5000
        }
      ]
    },
    {
      "matcher": "Bash",
      "hooks": [
        {
          "type": "command",
          "command": "scripts/validate-bash-safe.sh",
          "timeout": 5000
        }
      ]
    }
  ],
  "PostToolUse": [
    {
      "matcher": "*",
      "hooks": [
        {
          "type": "command",
          "command": "scripts/log-execution.sh",
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
          "type": "command",
          "command": "scripts/pre-exit-checks.sh",
          "timeout": 30000
        }
      ]
    }
  ],
  "SessionEnd": [
    {
      "matcher": "*",
      "hooks": [
        {
          "type": "command",
          "command": "scripts/session-cleanup.sh",
          "timeout": 10000
        }
      ]
    }
  ]
}
```

### Example: Security-Focused Configuration

```json
{
  "PreToolUse": [
    {
      "matcher": "Bash",
      "hooks": [
        {
          "type": "command",
          "command": "security/check-bash-security.sh",
          "timeout": 5000
        },
        {
          "type": "prompt",
          "prompt": "Security review: Is this bash command safe? Context: $ARGUMENTS",
          "timeout": 15000
        }
      ]
    },
    {
      "matcher": "*",
      "hooks": [
        {
          "type": "command",
          "command": "security/rate-limit-check.sh",
          "timeout": 2000
        }
      ]
    }
  ],
  "PostToolUse": [
    {
      "matcher": "*",
      "hooks": [
        {
          "type": "command",
          "command": "security/audit-output.sh",
          "timeout": 5000
        }
      ]
    }
  ]
}
```

### Example: Enterprise Compliance

```json
{
  "UserPromptSubmit": [
    {
      "matcher": "*",
      "hooks": [
        {
          "type": "command",
          "command": "compliance/check-pii.sh",
          "timeout": 5000
        }
      ]
    }
  ],
  "PreToolUse": [
    {
      "matcher": "*",
      "hooks": [
        {
          "type": "command",
          "command": "compliance/tool-policy-check.sh",
          "timeout": 3000
        }
      ]
    }
  ],
  "PostToolUse": [
    {
      "matcher": "*",
      "hooks": [
        {
          "type": "command",
          "command": "compliance/send-to-siem.sh",
          "timeout": 5000
        },
        {
          "type": "command",
          "command": "compliance/sanitize-output.sh",
          "timeout": 3000
        }
      ]
    }
  ],
  "Stop": [
    {
      "matcher": "*",
      "hooks": [
        {
          "type": "command",
          "command": "compliance/archive-transcript.sh",
          "timeout": 10000
        }
      ]
    }
  ]
}
```

---

## Migration Guide

### Upgrading from Previous Versions

#### From Versions Without Hooks (< 0.9.0)

1. **No changes required** - Hooks are optional; absence of hook configuration doesn't affect operation
2. **Add hooks gradually** - Configure hooks only for the events you need
3. **Test incrementally** - Enable hooks one at a time

#### From Versions with Legacy Hook System

1. **Locate old hook configuration**:
   - Legacy: `.claude/hooks/config.json`
   - New: `.claude/settings.json`

2. **Migrate configuration**:
```bash
# The system automatically discovers both locations
# but prioritizes .claude/settings.json
# Consider updating if currently using legacy location
cp .claude/hooks/config.json .claude/settings.json
```

3. **Update hook scripts**:
   - Old environment variable names still work
   - New names are preferred (check environment section)
   - Add error handling for new features

4. **Test thoroughly**:
```bash
# Run with verbose logging to verify hooks fire
claude code -v
```

### Configuration Best Practices

#### Version Control

Include hook configurations in version control:

```bash
git add .claude/settings.json
git add scripts/hooks/*.sh
```

**Note**: Don't commit API keys or sensitive credentials. Use environment variables.

#### Documentation

Document your hooks for team members:

```markdown
# Hook Configuration

## PreToolUse: Bash Validation
- Checks for dangerous patterns (rm -rf, etc.)
- Blocks network operations outside whitelist
- Location: scripts/validate-bash.sh
```

#### Performance Considerations

- **Hook Timeouts**: Set appropriate timeouts (default 60s)
  - Quick checks: 2-5 seconds
  - LLM decisions: 10-30 seconds
  - Complex validations: 30+ seconds

- **Parallel Execution**: Multiple hooks for same event run in parallel
  - Optimize for concurrency
  - Avoid resource contention

- **Deduplication**: Identical hook commands are deduplicated
  - Prevents redundant execution
  - Reduces latency

#### Error Handling

Hooks should handle errors gracefully:

```bash
#!/bin/bash
set -e  # Exit on error

# Your logic here

# Always output valid JSON on success
echo '{"permissionDecision": "allow"}'
exit 0

# On error, output error decision
echo '{"permissionDecision": "deny", "permissionDecisionReason": "Validation failed"}' >&2
exit 1
```

#### Logging

Log hook execution for debugging:

```bash
#!/bin/bash
LOGFILE="$HOME/.claude-code/logs/my-hook.log"
mkdir -p "$(dirname "$LOGFILE")"
echo "[$(date -u +%Y-%m-%dT%H:%M:%SZ)] Hook executed for $CLAUDE_TOOL_NAME" >> "$LOGFILE"
```

---

## Troubleshooting

### Common Issues

#### Hook Not Executing

**Symptoms**: Hook configuration exists but doesn't fire

**Solutions**:
1. Check configuration file location (must be `.claude/settings.json` or `.claude/hooks/config.json`)
2. Verify JSON syntax (use `jq . .claude/settings.json` to validate)
3. Check matcher pattern matches actual tool name
4. Enable debug logging to see hook selection

#### Hook Timing Out

**Symptoms**: Operations hang or take unexpectedly long

**Solutions**:
1. Increase timeout in configuration
2. Check if hook script is blocking on I/O
3. Verify external service availability (for hooks calling APIs)
4. Run hook script manually to measure actual duration

#### Hook Returning Invalid JSON

**Symptoms**: Error about parsing hook output

**Solutions**:
1. Ensure hook outputs valid JSON to stdout
2. Check for debug output being sent to stdout (use stderr for logs)
3. Verify JSON structure matches expected format
4. Test hook response format

#### Permission Decision Not Applied

**Symptoms**: Hook allows/denies but tool still executes opposite

**Solutions**:
1. Verify hook exit code is 0 (success)
2. Check decision field name matches event type
3. Ensure JSON structure is correct
4. Check for multiple conflicting hooks

### Debug Mode

Enable verbose hook debugging:

```bash
# Run CLI with debug mode
RUST_LOG=debug claude code

# Or set in configuration
{
  "debug": {
    "hooks": true,
    "logLevel": "trace"
  }
}
```

### Log Locations

- **CLI Logs**: `~/.claude-code/logs/cli.log`
- **Hook Logs**: `~/.claude-code/logs/hooks.log`
- **Transcript**: Current session transcript in `.claude/transcripts/`

---

## API Reference

### Hook Context Structure

All hooks receive context as JSON with these fields:

```json
{
  "session_id": "string",
  "transcript_path": "string",
  "cwd": "string",
  "permission_mode": "auto|ask|always",
  "hook_event_name": "string",
  "tool_name": "string (optional)",
  "tool_params": "object (optional)",
  "tool_result": "object (optional)",
  "user_prompt": "string (optional)",
  "session_start_matcher": "startup|resume|clear|compact (optional)",
  "session_end_reason": "clear|logout|prompt_input_exit|other (optional)",
  "notification_type": "permission_prompt|idle_prompt|auth_success|elicitation_dialog (optional)"
}
```

### Hook Response Structures

#### Permission Decision Response
```json
{
  "permissionDecision": "allow|deny|ask",
  "permissionDecisionReason": "string",
  "systemMessage": "string",
  "hookSpecificOutput": {
    "updatedInput": "object"
  }
}
```

#### Stop Decision Response
```json
{
  "decision": "approve|block",
  "reason": "string",
  "systemMessage": "string"
}
```

#### Generic Response
```json
{
  "continue": "true|false",
  "stopReason": "string",
  "suppressOutput": "true|false",
  "systemMessage": "string",
  "additionalContext": "string"
}
```

---

## Summary

The RustyClawd hook lifecycle system provides a flexible, powerful mechanism for customizing Claude Code behavior at critical points. Whether you're implementing security policies, compliance requirements, or custom workflows, hooks enable you to:

- **Validate** operations before they execute
- **Enhance** Claude with additional context
- **Control** session and tool execution flow
- **Monitor** and log all activities
- **Integrate** with external systems

Start with simple hooks and gradually add complexity as your needs grow. The system is designed to be forgiving—hooks are optional, timeouts prevent hangs, and error handling ensures failed hooks don't break operations.

For questions or issues, refer to the troubleshooting section or check the debug logs in `~/.claude-code/logs/`.
