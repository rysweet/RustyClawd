#!/bin/bash
# SessionStart hook that sets up project environment
# Demonstrates environment persistence with $CLAUDE_ENV_FILE

set -euo pipefail

echo "=== Claude Code Session Initialization ==="
echo "Session ID: $CLAUDE_SESSION_ID"
echo "CWD: $CLAUDE_CWD"
echo "Transcript: $CLAUDE_TRANSCRIPT_PATH"

# Source existing environment if available
if [[ -n "${CLAUDE_ENV_FILE:-}" ]] && [[ -f "$CLAUDE_ENV_FILE" ]]; then
    echo "Loading existing environment from $CLAUDE_ENV_FILE"
    source "$CLAUDE_ENV_FILE"
fi

# Set up project-specific environment
export PROJECT_NAME="claude-code-rs"
export PROJECT_TYPE="rust"
export BUILD_TOOL="cargo"

# Save environment for future commands
if [[ -n "${CLAUDE_ENV_FILE:-}" ]]; then
    cat > "$CLAUDE_ENV_FILE" << 'EOF'
export PROJECT_NAME="claude-code-rs"
export PROJECT_TYPE="rust"
export BUILD_TOOL="cargo"
EOF
    echo "Environment saved to $CLAUDE_ENV_FILE"
fi

echo "Session initialization complete!"
exit 0
