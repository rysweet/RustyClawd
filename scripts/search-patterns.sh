#!/bin/bash
# Helper script for searching common patterns in deminified Claude Code
# Usage: ./scripts/search-patterns.sh <pattern-name>

set -e

RESEARCH_DIR="$HOME/src/RustyClawd/docs/research"

if [ ! -f "$RESEARCH_DIR/claude-code-minified.js" ]; then
    echo "Error: Deminified file not found"
    echo "Run ./scripts/analyze-claude-code.sh first"
    exit 1
fi

cd "$RESEARCH_DIR"

# Pattern definitions
search_contentblocks() {
    echo "=== ContentBlock Patterns ==="
    echo ""
    echo "Type Definitions:"
    grep -n "ContentBlock.*=" claude-code-minified.js | head -20
    echo ""
    echo "Event Handling:"
    grep -n "content_block_start\|content_block_delta\|content_block_stop" \
        claude-code-minified.js | head -15
}

search_streaming() {
    echo "=== Streaming Event Patterns ==="
    echo ""
    echo "Event Types:"
    grep -n "message_start\|content_block_start\|ping\|error" \
        claude-code-minified.js | head -20
    echo ""
    echo "Stream Processing:"
    grep -n "streaming.*true\|parseStream\|createStream" \
        claude-code-minified.js | head -10
}

search_hooks() {
    echo "=== Hook Lifecycle Patterns ==="
    echo ""
    echo "Hook Registration:"
    grep -n "registeredHooks\|registerHook\|_hooks\[" \
        claude-code-minified.js | head -20
    echo ""
    echo "Hook Types:"
    grep -n "PreToolUse\|PostToolUse\|PostToolUseFailure" \
        claude-code-minified.js | head -10
}

search_tools() {
    echo "=== Tool Execution Patterns ==="
    echo ""
    echo "Tool Use Types:"
    grep -n "tool_use.*type\|ToolUseBlock" claude-code-minified.js | head -15
    echo ""
    echo "Tool Results:"
    grep -n "tool_result\|tool_use_id" claude-code-minified.js | head -15
}

search_session() {
    echo "=== Session Management Patterns ==="
    echo ""
    echo "Session IDs:"
    grep -n "sessionId\|parentSessionId" claude-code-minified.js | head -15
    echo ""
    echo "Session State:"
    grep -n "sessionTrustAccepted\|sessionBypassPermissionsMode" \
        claude-code-minified.js | head -10
}

search_thinking() {
    echo "=== Thinking Block Patterns ==="
    echo ""
    echo "Feature Flags:"
    grep -n "interleaved-thinking\|thinking.*toggle" \
        claude-code-minified.js | head -10
    echo ""
    echo "Preservation:"
    grep -n "preserve_thinking" claude-code-minified.js | head -10
}

search_validation() {
    echo "=== Validation Patterns ==="
    echo ""
    echo "Strict Mode:"
    grep -n "strict.*validation\|validator" claude-code-minified.js | head -15
    echo ""
    echo "Schema Validation:"
    grep -n "schema\|zod\|validate" claude-code-minified.js | head -15
}

search_custom() {
    echo "=== Custom Search: $1 ==="
    echo ""
    grep -n "$1" claude-code-minified.js | head -30
}

# Main menu
show_menu() {
    echo "Claude Code Pattern Search"
    echo "=========================="
    echo ""
    echo "Available patterns:"
    echo "  1. contentblocks  - ContentBlock types and handling"
    echo "  2. streaming      - Streaming events and parsing"
    echo "  3. hooks          - Hook lifecycle and registration"
    echo "  4. tools          - Tool execution and results"
    echo "  5. session        - Session management"
    echo "  6. thinking       - Thinking blocks (interleaved-thinking)"
    echo "  7. validation     - Validation and schemas"
    echo "  8. custom         - Custom pattern search"
    echo "  9. all            - Show all patterns"
    echo ""
}

# Handle commands
case "$1" in
    contentblocks|1)
        search_contentblocks
        ;;
    streaming|2)
        search_streaming
        ;;
    hooks|3)
        search_hooks
        ;;
    tools|4)
        search_tools
        ;;
    session|5)
        search_session
        ;;
    thinking|6)
        search_thinking
        ;;
    validation|7)
        search_validation
        ;;
    custom|8)
        if [ -z "$2" ]; then
            echo "Error: Provide search pattern"
            echo "Usage: $0 custom 'pattern'"
            exit 1
        fi
        search_custom "$2"
        ;;
    all|9)
        search_contentblocks
        echo ""
        echo "---"
        echo ""
        search_streaming
        echo ""
        echo "---"
        echo ""
        search_hooks
        echo ""
        echo "---"
        echo ""
        search_tools
        echo ""
        echo "---"
        echo ""
        search_session
        echo ""
        echo "---"
        echo ""
        search_thinking
        ;;
    *)
        show_menu
        echo "Usage: $0 <pattern-name> [custom-pattern]"
        exit 1
        ;;
esac
