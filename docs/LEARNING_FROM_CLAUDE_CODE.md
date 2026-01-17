# Learning from Claude Code's Implementation

**Quick Links**:
- [5-Minute Quick Start](QUICK_START_DEMINIFICATION.md)
- [Complete Guide](DEMINIFICATION_GUIDE.md)
- [Visual Workflow Diagrams](DEMINIFICATION_WORKFLOW_DIAGRAM.md)

## Overview

RustyClawd aims for 100% compatibility with Claude Code. To achieve this, we can learn from Claude Code's JavaScript implementation by deminifying and analyzing it. This directory contains everything you need to:

1. **Deminify** Claude Code's minified JavaScript
2. **Search** for specific patterns and implementations
3. **Translate** JavaScript patterns to Rust
4. **Implement** features with confidence
5. **Verify** parity with Claude Code

## Why This Matters

Claude Code's npm package is minified (11MB, difficult to read). By deminifying it, we can:

- **Understand** how features are implemented
- **Learn** patterns for thinking blocks, strict validation, etc.
- **Verify** our Rust implementation matches their behavior
- **Accelerate** development by learning from production code

## Getting Started (5 Minutes)

```bash
# 1. Run automated setup
cd ~/src/RustyClawd
./scripts/analyze-claude-code.sh

# 2. Try a search
./scripts/search-patterns.sh thinking

# 3. Explore the research directory
code docs/research/
```

That's it! You now have:
- Deminified JavaScript files (489k lines)
- Search indices for common patterns
- Ready-to-use search tools

## What You Get

### Documentation
- **DEMINIFICATION_GUIDE.md** (34KB) - Complete reference guide
- **QUICK_START_DEMINIFICATION.md** (3KB) - Get started in 5 minutes
- **DEMINIFICATION_WORKFLOW_DIAGRAM.md** (7KB) - 8 visual diagrams
- **research/README.md** (3.6KB) - Research directory guide
- **research/findings.md** (4.6KB) - Template for discoveries

### Automation
- **analyze-claude-code.sh** - One-command setup
- **search-patterns.sh** - Quick pattern searches

### Search Indices
After running `analyze-claude-code.sh`:
- `index-contentblock.txt` - ContentBlock patterns
- `index-streaming.txt` - Streaming events
- `index-hooks.txt` - Hook lifecycle
- `index-tools.txt` - Tool execution
- `index-session.txt` - Session management
- `index-thinking.txt` - Thinking blocks

## Common Use Cases

### 1. Understanding a Feature

```bash
# Find how thinking blocks work
./scripts/search-patterns.sh thinking

# Or search directly
grep -n "thinking" docs/research/claude-code-minified.js | head -20
```

### 2. Implementing New Feature

1. Search Claude Code for similar feature
2. Extract implementation pattern
3. Use translation guide (in DEMINIFICATION_GUIDE.md)
4. Implement in Rust
5. Test for parity

### 3. Debugging Behavior Differences

1. Find relevant code in Claude Code
2. Compare with RustyClawd implementation
3. Identify divergence
4. Fix and verify

## Key Patterns Documented

| Pattern | Location | Guide Section |
|---------|----------|---------------|
| ContentBlock types | Lines 126300-126306 | Key Pattern Search → ContentBlocks |
| Event ordering validation | Line 186611 | Key Pattern Search → Streaming |
| Hook registry | Lines 2218-2226 | Key Pattern Search → Hooks |
| Tool execution | Line 186269 | Key Pattern Search → Tools |
| Session management | Lines 1818-1819 | Key Pattern Search → Session |
| Thinking blocks | Lines 1723, 92007 | Key Pattern Search → Thinking |

## JavaScript → Rust Translation

Complete patterns documented in DEMINIFICATION_GUIDE.md:

- Promises → `async/await`
- Classes → `struct` + `impl`
- Interfaces → `trait`
- Union types → `enum` with `#[serde(tag = "type")]`
- Callbacks → `Box<dyn Fn>`
- Error handling → `Result<T, E>` + `thiserror`

Each pattern includes:
- Real JavaScript example from Claude Code
- Rust translation
- Notes about differences
- Best practices

## Tools Comparison

| Tool | Lines Output | Speed | Best For |
|------|--------------|-------|----------|
| prettier | 489,060 | ~19s | Deep analysis, learning |
| js-beautify | 464,791 | ~19s | Quick exploration |

**Both included** in the workflow for different use cases.

## Success Metrics

- ✅ **One-command setup** - No manual steps
- ✅ **45KB documentation** - Comprehensive guides
- ✅ **Real examples** - From Claude Code with line numbers
- ✅ **7 pattern searches** - Pre-built + custom
- ✅ **Team ready** - Onboarding reduced from hours to minutes

## File Structure

```
docs/
├── LEARNING_FROM_CLAUDE_CODE.md        # This file
├── DEMINIFICATION_GUIDE.md             # Complete guide (34KB)
├── QUICK_START_DEMINIFICATION.md       # 5-minute start
├── DEMINIFICATION_WORKFLOW_DIAGRAM.md  # Visual diagrams
└── research/
    ├── README.md                        # Research directory guide
    ├── findings.md                      # Discovery template
    ├── INVESTIGATION_SUMMARY.md         # Investigation report
    ├── claude-code-minified.js         # Created by script
    ├── claude-code-jsbeautify.js       # Created by script
    └── index-*.txt                      # Created by script

scripts/
├── analyze-claude-code.sh              # Automated setup
└── search-patterns.sh                  # Pattern searches
```

## Next Steps

### For New Team Members
1. Read [QUICK_START_DEMINIFICATION.md](QUICK_START_DEMINIFICATION.md)
2. Run `./scripts/analyze-claude-code.sh`
3. Try example searches
4. Read [DEMINIFICATION_GUIDE.md](DEMINIFICATION_GUIDE.md)

### For Feature Development
1. Search for similar feature in Claude Code
2. Extract pattern using search tools
3. Translate using guide
4. Implement in Rust
5. Test for parity

### For Documentation
1. Discover new patterns
2. Document in `docs/research/findings.md`
3. Share with team

## Tips

- **Use VS Code**: `code docs/research/` for better navigation
- **Check indices first**: Faster than grepping full file
- **Document findings**: Share knowledge with team
- **Refer to guide**: Complete JS→Rust patterns available

## Contributing

When you discover useful patterns:

1. Add them to `docs/research/findings.md`
2. Include JavaScript snippets with line numbers
3. Provide Rust translations
4. Share with the team

## Resources

- [Prettier Documentation](https://prettier.io/docs/en/options.html)
- [js-beautify Documentation](https://github.com/beautify-web/js-beautify)
- [Serde Documentation](https://serde.rs/)
- [Tokio Async Guide](https://tokio.rs/tokio/tutorial)

## Questions?

See the complete guide: [DEMINIFICATION_GUIDE.md](DEMINIFICATION_GUIDE.md)

It includes:
- Detailed workflow steps
- Tool comparison and selection
- Key pattern search reference
- JavaScript to Rust translation patterns
- Advanced search techniques
- Troubleshooting guide
- Tips and best practices
