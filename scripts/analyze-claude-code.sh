#!/bin/bash
# Complete workflow for deminifying and analyzing Claude Code
# Usage: ./scripts/analyze-claude-code.sh

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
RESEARCH_DIR="$HOME/src/RustyClawd/docs/research"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

echo_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

echo_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Check dependencies
check_dependencies() {
    echo_info "Checking dependencies..."

    if ! command -v npm &> /dev/null; then
        echo_error "npm not found. Please install Node.js"
        exit 1
    fi

    if ! command -v prettier &> /dev/null; then
        echo_warn "prettier not found. Installing..."
        npm install -g prettier
    fi

    if ! command -v js-beautify &> /dev/null; then
        echo_warn "js-beautify not found. Installing..."
        npm install -g js-beautify
    fi

    echo_info "All dependencies installed"
}

# Find Claude Code installation
find_claude_code() {
    echo_info "Locating Claude Code installation..."

    CLAUDE_CODE_PATH=$(readlink -f $(which claude) 2>/dev/null)
    if [ -z "$CLAUDE_CODE_PATH" ]; then
        echo_error "Claude Code not found in PATH"
        echo_error "Please install Claude Code first"
        exit 1
    fi

    CLAUDE_CODE_DIR=$(dirname "$CLAUDE_CODE_PATH")
    echo_info "Found Claude Code at: $CLAUDE_CODE_DIR"

    if [ ! -f "$CLAUDE_CODE_DIR/cli.js" ]; then
        echo_error "cli.js not found at expected location"
        exit 1
    fi
}

# Setup research directory
setup_research_dir() {
    echo_info "Setting up research directory..."
    mkdir -p "$RESEARCH_DIR"
    cd "$RESEARCH_DIR"
    echo_info "Research directory: $RESEARCH_DIR"
}

# Deminify files
deminify_files() {
    echo_info "Deminifying Claude Code..."

    # Copy original
    cp "$CLAUDE_CODE_DIR/cli.js" claude-code-minified.js
    echo_info "Copied minified file"

    # Deminify with prettier
    echo_info "Running prettier (this may take 20-30 seconds)..."
    prettier --write claude-code-minified.js --print-width 100 2>&1 | grep -v "^$" || true

    # Create js-beautify version
    echo_info "Running js-beautify..."
    cp "$CLAUDE_CODE_DIR/cli.js" claude-code-jsbeautify.js
    js-beautify -r claude-code-jsbeautify.js 2>&1 | grep -v "^$" || true

    # Report
    PRETTIER_LINES=$(wc -l < claude-code-minified.js)
    BEAUTIFY_LINES=$(wc -l < claude-code-jsbeautify.js)

    echo_info "Deminification complete!"
    echo_info "  Prettier version: $PRETTIER_LINES lines"
    echo_info "  js-beautify version: $BEAUTIFY_LINES lines"
}

# Create search indices
create_indices() {
    echo_info "Creating search indices..."

    # ContentBlock patterns
    grep -n "ContentBlock\|content_block" claude-code-minified.js > index-contentblock.txt || true
    echo_info "  ContentBlock: $(wc -l < index-contentblock.txt) matches"

    # Streaming patterns
    grep -n "streaming\|StreamingEvent\|message_start\|content_block_start" \
        claude-code-minified.js > index-streaming.txt || true
    echo_info "  Streaming: $(wc -l < index-streaming.txt) matches"

    # Hook patterns
    grep -n "hook\|Hook\|lifecycle" claude-code-minified.js > index-hooks.txt || true
    echo_info "  Hooks: $(wc -l < index-hooks.txt) matches"

    # Tool execution
    grep -n "tool_use\|ToolUse\|execute_tool" claude-code-minified.js > index-tools.txt || true
    echo_info "  Tools: $(wc -l < index-tools.txt) matches"

    # Session management
    grep -n "session\|Session\|sessionId" claude-code-minified.js > index-session.txt || true
    echo_info "  Session: $(wc -l < index-session.txt) matches"

    # Thinking blocks
    grep -n "thinking\|ThinkingBlock\|interleaved-thinking" \
        claude-code-minified.js > index-thinking.txt || true
    echo_info "  Thinking: $(wc -l < index-thinking.txt) matches"

    echo_info "Indices created in: $RESEARCH_DIR/index-*.txt"
}

# Interactive search helper
interactive_search() {
    echo ""
    echo_info "Research directory ready: $RESEARCH_DIR"
    echo ""
    echo "Available files:"
    echo "  - claude-code-minified.js (prettier formatted)"
    echo "  - claude-code-jsbeautify.js (js-beautify formatted)"
    echo "  - index-*.txt (search indices)"
    echo ""
    echo "Example searches:"
    echo "  grep -n 'pattern' claude-code-minified.js | head -20"
    echo "  cat index-contentblock.txt | grep 'TextContentBlock'"
    echo "  less +/ContentBlock claude-code-minified.js"
    echo ""

    # Offer to open in editor
    read -p "Open research directory in VS Code? (y/n) " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        if command -v code &> /dev/null; then
            code "$RESEARCH_DIR"
        else
            echo_warn "VS Code not found in PATH"
        fi
    fi
}

# Main execution
main() {
    echo_info "Claude Code Deminification and Analysis"
    echo_info "========================================"
    echo ""

    check_dependencies
    find_claude_code
    setup_research_dir
    deminify_files
    create_indices
    interactive_search

    echo ""
    echo_info "Analysis setup complete!"
}

# Run
main
