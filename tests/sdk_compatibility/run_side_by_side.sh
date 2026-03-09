#!/bin/bash
# Side-by-side SDK compatibility comparison
# Run this from a regular terminal (NOT inside Claude Code)
#
# Usage: ./tests/sdk_compatibility/run_side_by_side.sh
#
# Requires: ANTHROPIC_API_KEY, claude binary, target/release/rusty binary
set -e

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m'

if [ -z "$ANTHROPIC_API_KEY" ]; then
    echo -e "${RED}ERROR: ANTHROPIC_API_KEY not set${NC}"
    exit 1
fi

RUSTY="$(dirname "$0")/../../target/release/rusty"
if [ ! -f "$RUSTY" ]; then
    echo -e "${RED}ERROR: Build RustyClawd first: cargo build --release${NC}"
    exit 1
fi

PROMPT="What is 2+2? Answer with just the number."
TMPDIR=$(mktemp -d)

echo -e "${CYAN}=== Claude Agent SDK Side-by-Side Comparison ===${NC}"
echo ""

# Run Claude Code
echo -e "${YELLOW}Running Claude Code...${NC}"
claude -p --output-format stream-json "$PROMPT" 2>/dev/null > "$TMPDIR/claude.jsonl" || true

# Run RustyClawd
echo -e "${YELLOW}Running RustyClawd...${NC}"
"$RUSTY" --print --output-format stream-json "$PROMPT" 2>/dev/null > "$TMPDIR/rusty.jsonl" || true

echo ""
echo -e "${CYAN}=== Claude Code Output ===${NC}"
if [ -s "$TMPDIR/claude.jsonl" ]; then
    while IFS= read -r line; do
        echo "$line" | python3 -m json.tool 2>/dev/null || echo "$line"
    done < "$TMPDIR/claude.jsonl"
else
    echo -e "${RED}(no output - Claude Code may not work with API key auth)${NC}"
fi

echo ""
echo -e "${CYAN}=== RustyClawd Output ===${NC}"
if [ -s "$TMPDIR/rusty.jsonl" ]; then
    # Filter out non-JSON lines (tracing output)
    grep '^{' "$TMPDIR/rusty.jsonl" | while IFS= read -r line; do
        echo "$line" | python3 -m json.tool 2>/dev/null || echo "$line"
    done
else
    echo -e "${RED}(no output)${NC}"
fi

echo ""
echo -e "${CYAN}=== Format Comparison ===${NC}"

# Extract message types from both
CLAUDE_TYPES=$(grep '^{' "$TMPDIR/claude.jsonl" 2>/dev/null | python3 -c "
import json, sys
for line in sys.stdin:
    try:
        msg = json.loads(line)
        t = msg.get('type','?')
        st = msg.get('subtype','')
        sid = 'yes' if 'session_id' in msg else 'no'
        ptui = 'yes' if 'parent_tool_use_id' in msg else 'no'
        print(f'  {t}/{st}: session_id={sid} parent_tool_use_id={ptui}')
    except: pass
" 2>/dev/null)

RUSTY_TYPES=$(grep '^{' "$TMPDIR/rusty.jsonl" 2>/dev/null | python3 -c "
import json, sys
for line in sys.stdin:
    try:
        msg = json.loads(line)
        t = msg.get('type','?')
        st = msg.get('subtype','')
        sid = 'yes' if 'session_id' in msg else 'no'
        ptui = 'yes' if 'parent_tool_use_id' in msg else 'no'
        print(f'  {t}/{st}: session_id={sid} parent_tool_use_id={ptui}')
    except: pass
" 2>/dev/null)

echo -e "${YELLOW}Claude Code message types:${NC}"
if [ -n "$CLAUDE_TYPES" ]; then
    echo "$CLAUDE_TYPES"
else
    echo "  (none)"
fi

echo -e "${YELLOW}RustyClawd message types:${NC}"
if [ -n "$RUSTY_TYPES" ]; then
    echo "$RUSTY_TYPES"
else
    echo "  (none)"
fi

# Cleanup
rm -rf "$TMPDIR"

echo ""
echo -e "${GREEN}Done.${NC}"
