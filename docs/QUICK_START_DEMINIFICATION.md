# Quick Start: Claude Code Deminification

**Goal**: Learn from Claude Code's JavaScript implementation by deminifying and analyzing it.

## 5-Minute Setup

```bash
# 1. Navigate to project
cd ~/src/RustyClawd

# 2. Run automated setup (installs tools, deminifies, creates indices)
./scripts/analyze-claude-code.sh

# 3. Search for patterns
./scripts/search-patterns.sh thinking
./scripts/search-patterns.sh contentblocks
```

## What This Gets You

After running the setup:

### Files Created
- **~/src/RustyClawd/docs/research/claude-code-minified.js** - Formatted JavaScript (~489k lines)
- **~/src/RustyClawd/docs/research/index-*.txt** - Search indices for common patterns

### Available Tools
- **analyze-claude-code.sh** - One-command setup
- **search-patterns.sh** - Quick pattern searches

### Documentation
- **DEMINIFICATION_GUIDE.md** - Complete reference (34KB)
- **docs/research/README.md** - Research directory guide
- **docs/research/findings.md** - Template for discoveries

## Common Use Cases

### 1. Understanding How Claude Code Implements Feature X

```bash
# Search for the feature
grep -n "feature_name" ~/src/RustyClawd/docs/research/claude-code-minified.js | head -20

# Find related code
grep -B 10 -A 10 "line_number" ~/src/RustyClawd/docs/research/claude-code-minified.js

# Map to Rust (see DEMINIFICATION_GUIDE.md for patterns)
```

### 2. Finding How Streaming Events Work

```bash
# Use pre-built search
./scripts/search-patterns.sh streaming

# Or search manually
cat ~/src/RustyClawd/docs/research/index-streaming.txt | grep "message_start"
```

### 3. Implementing a New Feature

1. Find similar feature in Claude Code
2. Extract implementation pattern
3. Translate to Rust using guide
4. Test for parity

## Next Steps

1. **Read the full guide**: [DEMINIFICATION_GUIDE.md](DEMINIFICATION_GUIDE.md)
2. **Run searches**: `./scripts/search-patterns.sh` for menu
3. **Document findings**: Add to `docs/research/findings.md`
4. **Implement in Rust**: Use translation patterns from guide

## Key Patterns to Search

| Pattern | Command | Use For |
|---------|---------|---------|
| ContentBlocks | `./scripts/search-patterns.sh contentblocks` | Block types, event handling |
| Streaming | `./scripts/search-patterns.sh streaming` | Event parsing, message flow |
| Hooks | `./scripts/search-patterns.sh hooks` | Hook lifecycle, registration |
| Tools | `./scripts/search-patterns.sh tools` | Tool execution, results |
| Session | `./scripts/search-patterns.sh session` | Session management |
| Thinking | `./scripts/search-patterns.sh thinking` | Thinking blocks, feature flags |

## Files Reference

| File | Purpose |
|------|---------|
| `DEMINIFICATION_GUIDE.md` | Complete guide (read this!) |
| `QUICK_START_DEMINIFICATION.md` | This file (quick reference) |
| `scripts/analyze-claude-code.sh` | Automated setup |
| `scripts/search-patterns.sh` | Pattern search helper |
| `docs/research/README.md` | Research directory guide |
| `docs/research/findings.md` | Document your discoveries |

## Tips

- **Use VS Code**: `code ~/src/RustyClawd/docs/research/` for better navigation
- **Check indices first**: Faster than grepping full file
- **Document findings**: Share knowledge with team
- **Refer to guide**: DEMINIFICATION_GUIDE.md has JS→Rust patterns

## Troubleshooting

### "Command not found: prettier"
Run: `npm install -g prettier js-beautify`

### "Deminified file not found"
Run: `./scripts/analyze-claude-code.sh` first

### "Search takes too long"
Use indices: `cat ~/src/RustyClawd/docs/research/index-*.txt | grep pattern`

## More Information

See [DEMINIFICATION_GUIDE.md](DEMINIFICATION_GUIDE.md) for:
- Detailed workflow steps
- JavaScript to Rust translation patterns
- Advanced search techniques
- Complete examples and references
