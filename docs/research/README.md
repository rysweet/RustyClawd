# Claude Code Research Directory

This directory contains deminified Claude Code JavaScript files and search indices for analyzing implementation patterns.

## Contents

After running `../../scripts/analyze-claude-code.sh`, this directory will contain:

### Deminified Files
- `claude-code-minified.js` - Prettier-formatted version (~489k lines)
- `claude-code-jsbeautify.js` - js-beautify formatted version (~465k lines)

### Search Indices
- `index-contentblock.txt` - All ContentBlock-related code
- `index-streaming.txt` - Streaming event patterns
- `index-hooks.txt` - Hook lifecycle and registration
- `index-tools.txt` - Tool execution patterns
- `index-session.txt` - Session management code
- `index-thinking.txt` - Thinking block implementation

## Quick Start

1. **Generate deminified files and indices**:
   ```bash
   cd ~/src/RustyClawd
   ./scripts/analyze-claude-code.sh
   ```

2. **Search for specific patterns**:
   ```bash
   ./scripts/search-patterns.sh contentblocks
   ./scripts/search-patterns.sh thinking
   ./scripts/search-patterns.sh custom "your-pattern"
   ```

3. **Manual exploration**:
   ```bash
   # Search directly
   grep -n "pattern" claude-code-minified.js | head -20

   # Use indices
   cat index-contentblock.txt | grep "TextContentBlock"

   # Extract sections
   sed -n '126300,126400p' claude-code-minified.js
   ```

## File Comparison

| File | Lines | Tool | Best For |
|------|-------|------|----------|
| claude-code-minified.js | ~489k | prettier | Reading, understanding |
| claude-code-jsbeautify.js | ~465k | js-beautify | Quick searches |

## Common Workflows

### Understanding a Feature

1. Search for feature name: `grep -n "feature_name" claude-code-minified.js`
2. Find type definitions: Look for constants, enums
3. Trace execution: Follow method calls
4. Map to Rust: Use DEMINIFICATION_GUIDE.md

### Finding Implementation Details

1. Check relevant index: `cat index-contentblock.txt | grep "specific_term"`
2. Extract context: `grep -B 10 -A 10 "line_number" claude-code-minified.js`
3. Compare with Rust: Identify differences
4. Update implementation

### Verifying Behavior

1. Find JavaScript implementation
2. Understand the logic
3. Write equivalent Rust test
4. Verify parity

## Tips

### Better Navigation

Open in VS Code for:
- Syntax highlighting
- Find/replace
- Go to line
- Multi-cursor editing

```bash
code ~/src/RustyClawd/docs/research/
```

### Efficient Searching

```bash
# Case-insensitive
grep -i "pattern" claude-code-minified.js

# Multiple patterns
grep -E "pattern1|pattern2" claude-code-minified.js

# With context
grep -B 5 -A 5 "pattern" claude-code-minified.js

# Count matches
grep -c "pattern" claude-code-minified.js
```

### Extract Code Sections

```bash
# Lines 1000-2000
sed -n '1000,2000p' claude-code-minified.js > section.js

# Around a match
grep -n "pattern" claude-code-minified.js  # find line number
sed -n '1000,1100p' claude-code-minified.js  # extract around that line
```

## Resources

- [DEMINIFICATION_GUIDE.md](../DEMINIFICATION_GUIDE.md) - Complete guide
- [search-patterns.sh](../../scripts/search-patterns.sh) - Pattern search helper
- [analyze-claude-code.sh](../../scripts/analyze-claude-code.sh) - Setup automation

## Contributing

When you discover useful patterns:

1. Document them in findings.md (this directory)
2. Add search patterns to search-patterns.sh
3. Update DEMINIFICATION_GUIDE.md with examples
4. Share with the team

## Notes

- Files are regenerated each time you run analyze-claude-code.sh
- Keep findings.md for your research notes (not overwritten)
- Indices are quick references, always verify in source files
