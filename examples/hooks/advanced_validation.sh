#!/bin/bash
# Advanced PreToolUse validation hook for dangerous commands
# This hook demonstrates permission control with JSON output

set -euo pipefail

# Check if this is a Bash tool
if [[ "$CLAUDE_TOOL_NAME" == "Bash" ]]; then
    # Read tool parameters from environment or stdin
    COMMAND="${TOOL_COMMAND:-}"

    # Check for dangerous commands
    if echo "$COMMAND" | grep -qE "(rm -rf|dd if=/dev/zero|mkfs|fdisk|:(){:|:&};:)"; then
        # Blocking error - dangerous command detected
        echo "DANGER: Potentially destructive command detected!" >&2
        echo '{"permissionDecision": "deny", "additionalContext": "Destructive command blocked by security hook"}'
        exit 0
    fi

    # Check for sudo commands - ask user
    if echo "$COMMAND" | grep -qE "^sudo "; then
        echo '{"permissionDecision": "ask", "additionalContext": "Command requires elevated privileges"}'
        exit 0
    fi

    # Safe command - allow
    echo '{"permissionDecision": "allow"}'
    exit 0
fi

# For non-Bash tools, allow by default
echo '{"permissionDecision": "allow"}'
exit 0
