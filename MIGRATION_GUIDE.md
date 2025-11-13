# Migration Guide: CLI Breaking Changes

## Overview

This guide helps you migrate from the old `rusty` CLI to the new spec-compliant version that matches the official Claude Code CLI.

---

## Quick Reference: Flag Changes

### Renamed Flags

| Old Flag | New Flag | Notes |
|----------|----------|-------|
| `--allowed-tools` | `--allowedTools` | Now uses camelCase to match official spec |
| `--disallowed-tools` | `--disallowedTools` | Now uses camelCase to match official spec |
| `--debug` | `--verbose` | Renamed to match official spec |

**Migration Example:**
```bash
# Old
rusty --debug --allowed-tools Read,Write "query"

# New
rusty --verbose --allowedTools Read --allowedTools Write "query"
```

---

## Removed Flags

The following flags are **no longer available** as they are not part of the official Claude Code CLI spec:

### Sampling Parameters (REMOVED)

```bash
# ❌ REMOVED - No longer available
--max-tokens <NUM>
--temperature <NUM>
--top-p <NUM>
--top-k <NUM>
--stop-sequences <SEQUENCES>
```

**Why removed**: The official Claude Code CLI doesn't expose these low-level API parameters. These are managed internally.

**Alternative**: These parameters are set to sensible defaults:
- Max tokens: 4096
- Other parameters: Anthropic defaults

---

### Directory/Working Options (CHANGED)

```bash
# ❌ REMOVED
--working-directory <DIR>

# ✅ USE INSTEAD
--add-dir <DIR>
```

**Migration:**
```bash
# Old
rusty --working-directory /path/to/project "query"

# New
rusty --add-dir /path/to/project "query"

# Multiple directories (new feature!)
rusty --add-dir /path/to/project --add-dir /path/to/lib "query"
```

---

### Feature Toggles (REMOVED)

```bash
# ❌ REMOVED - Tools are always enabled in official spec
--no-tools

# ❌ REMOVED - Streaming is always enabled in official spec
--no-stream

# ❌ REMOVED - Internal configuration, not user-facing
--checkpoint-limit <NUM>

# ❌ REMOVED - Not part of official spec
--tui
```

**Why removed**: The official Claude Code CLI always enables tools and uses them intelligently based on context. Streaming is the standard behavior.

**Alternative for tool control**: Use `--disallowedTools` to prevent specific tools from being used:

```bash
# Instead of --no-tools, disallow all editing tools
rusty --disallowedTools Edit --disallowedTools Write "analyze this code"
```

---

## New Features

### System Prompt from File

```bash
# Load custom system prompt from file
rusty --system-prompt-file ./my-prompt.txt "query"
```

### Subcommands

```bash
# Update to latest version
rusty update

# Configure MCP servers
rusty mcp
```

### Agent Configuration

```bash
# Define custom subagents
rusty --agents '{"reviewer":{"description":"Code review agent","prompt":"You are a code reviewer"}}' "query"
```

### Advanced Permissions

```bash
# Specify permission mode
rusty --permission-mode plan "query"

# Skip permissions (use with caution!)
rusty --dangerously-skip-permissions "query"

# Use MCP tool for permissions
rusty --permission-prompt-tool mcp_auth_tool "query"
```

### Turn Limiting

```bash
# Limit to 5 agentic turns in non-interactive mode
rusty -p --max-turns 5 "complex task"
```

### Multiple Input/Output Formats

```bash
# Stream JSON output with partial messages
rusty -p --output-format stream-json --include-partial-messages "query"

# Accept streaming JSON input
rusty --input-format stream-json < input.jsonl
```

---

## Common Migration Scenarios

### Scenario 1: Simple Query with Debug Output

**Before:**
```bash
rusty --debug -p "explain this code"
```

**After:**
```bash
rusty --verbose -p "explain this code"
```

---

### Scenario 2: Custom Model with Temperature

**Before:**
```bash
rusty --model sonnet --temperature 0.7 -p "creative writing task"
```

**After:**
```bash
# Temperature no longer configurable (uses Anthropic defaults)
rusty --model sonnet -p "creative writing task"
```

**Note**: Temperature is managed automatically by the API for best results.

---

### Scenario 3: Tool Restrictions

**Before:**
```bash
rusty --no-tools -p "just answer the question"
```

**After:**
```bash
# Tools are always available, but you can disallow specific ones
# For analysis-only queries, disallow editing tools:
rusty --disallowedTools Edit --disallowedTools Write -p "analyze this code"

# Or just rely on Claude to use tools appropriately
rusty -p "just answer the question"
```

---

### Scenario 4: Custom Working Directory

**Before:**
```bash
rusty --working-directory /path/to/project "analyze main.rs"
```

**After:**
```bash
rusty --add-dir /path/to/project "analyze main.rs"
```

---

### Scenario 5: Multiple Directories

**Before:**
```bash
# Not possible in old version
```

**After:**
```bash
# Now you can add multiple directories!
rusty --add-dir ./backend --add-dir ./frontend "check consistency"
```

---

### Scenario 6: Custom System Prompt

**Before:**
```bash
rusty --system-prompt "You are a Python expert" -p "query"
```

**After:**
```bash
# Same syntax works, plus new file-based option:
rusty --system-prompt "You are a Python expert" -p "query"

# Or load from file:
rusty --system-prompt-file ./expert-prompt.txt -p "query"
```

---

## Shell Scripting Changes

### Update Shell Scripts

If you have shell scripts using the old flags, here's a migration helper:

```bash
#!/bin/bash
# Old script
rusty --debug \
  --allowed-tools "Read,Write" \
  --working-directory "$PROJECT_DIR" \
  -p "analyze code"

# Migrated script
rusty --verbose \
  --allowedTools Read \
  --allowedTools Write \
  --add-dir "$PROJECT_DIR" \
  -p "analyze code"
```

### Update Aliases

```bash
# Old .bashrc/.zshrc
alias claudecode='rusty --debug'

# New .bashrc/.zshrc
alias claudecode='rusty --verbose'
```

---

## Programmatic Usage (SDK/Library)

If you're calling rusty from another program:

```python
# Old Python code
subprocess.run([
    "rusty",
    "--allowed-tools", "Read,Write",
    "--temperature", "0.7",
    "-p", "query"
])

# New Python code
subprocess.run([
    "rusty",
    "--allowedTools", "Read",
    "--allowedTools", "Write",
    # temperature removed - uses API defaults
    "-p", "query"
])
```

---

## Rollback Strategy

If you need to keep using the old CLI temporarily:

1. The old binary is still available in your git history
2. Pin to a specific commit before this change
3. Build the old version: `git checkout <old-commit> && cargo build`

However, we recommend migrating as soon as possible since the new version is spec-compliant with the official Claude Code CLI.

---

## Getting Help

### Check Available Flags
```bash
rusty --help
```

### Check Subcommands
```bash
rusty update --help
rusty mcp --help
```

### Verbose Mode
Use `--verbose` to see detailed logging:
```bash
rusty --verbose -p "query"
```

---

## Summary Checklist

- [ ] Replace `--debug` with `--verbose`
- [ ] Replace `--allowed-tools` with `--allowedTools`
- [ ] Replace `--disallowed-tools` with `--disallowedTools`
- [ ] Replace `--working-directory` with `--add-dir`
- [ ] Remove `--no-tools`, `--no-stream`, `--tui`
- [ ] Remove `--max-tokens`, `--temperature`, `--top-p`, `--top-k`, `--stop-sequences`
- [ ] Remove `--checkpoint-limit`
- [ ] Update shell scripts and aliases
- [ ] Update programmatic calls
- [ ] Test your workflows with the new CLI

---

## Need Help?

If you encounter issues during migration:
1. Check `rusty --help` for current flag documentation
2. Review the CLI_SPEC_COMPLIANCE.md document
3. Report issues with example commands that worked before but don't now
