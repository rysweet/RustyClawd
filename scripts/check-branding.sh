#!/bin/bash
# Brand validation script for RustyClawd
# Ensures no "Claude" branding appears in user-facing UI strings

set -e

echo "🔍 Checking for Claude branding in UI code..."

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Track if we found any violations
VIOLATIONS=0

# Directories to check (user-facing code)
UI_DIRS=(
    "crates/cli/src/tui"
    "crates/cli/src/main.rs"
    "crates/cli/src/interactive.rs"
)

# Allowed exceptions (these are OK to have "Claude" in them)
ALLOWED_PATTERNS=(
    "claude-sonnet"           # API model names
    "claude-opus"             # API model names
    "claude-haiku"            # API model names
    ".claude/"                # Directory paths (plugin spec)
    "// .*Claude"             # Comments
    "/\\* .*Claude"           # Block comments
    "claude_code"             # Internal variable names
    "#\\[test\\]"             # Test code (checked separately)
)

# Function to check a file
check_file() {
    local file=$1
    local matches
    
    # Search for "Claude" (case-sensitive) in string literals
    matches=$(grep -n '"[^"]*Claude[^"]*"' "$file" 2>/dev/null || true)
    
    if [ -n "$matches" ]; then
        # Filter out allowed patterns
        local filtered_matches=""
        while IFS= read -r line; do
            local is_allowed=false
            for pattern in "${ALLOWED_PATTERNS[@]}"; do
                if echo "$line" | grep -qE "$pattern"; then
                    is_allowed=true
                    break
                fi
            done
            
            if [ "$is_allowed" = false ]; then
                filtered_matches+="$line"$'\n'
            fi
        done <<< "$matches"
        
        # If we still have matches after filtering, report them
        if [ -n "$filtered_matches" ]; then
            echo -e "${RED}❌ Found Claude branding in: $file${NC}"
            echo "$filtered_matches"
            ((VIOLATIONS++))
        fi
    fi
}

# Check each directory/file
for target in "${UI_DIRS[@]}"; do
    if [ -f "$target" ]; then
        check_file "$target"
    elif [ -d "$target" ]; then
        while IFS= read -r -d '' file; do
            check_file "$file"
        done < <(find "$target" -name "*.rs" -print0)
    fi
done

# Report results
echo ""
if [ $VIOLATIONS -eq 0 ]; then
    echo -e "${GREEN}✅ No Claude branding violations found!${NC}"
    exit 0
else
    echo -e "${RED}❌ Found $VIOLATIONS file(s) with Claude branding violations${NC}"
    echo -e "${YELLOW}Please use 'RustyClawd' or generic terms like 'Assistant' instead${NC}"
    exit 1
fi
