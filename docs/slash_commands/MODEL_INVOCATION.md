# Slash Command Model Invocation

## Overview

RustyClawd slash commands can now be configured to be invoked by Claude via the SlashCommand tool, allowing the model to see the original command with arguments and decide how to process it.

## How It Works

### Local Execution (Default)

```
User types: /my-command arg1 arg2
    ↓
RustyClawd intercepts and expands template locally
    ↓
Sends expanded text to Claude
    ↓
Claude processes expanded text
```

### Model Invocation (Opt-In)

```
User types: /my-command arg1 arg2
    ↓
RustyClawd passes command through to Claude
    ↓
Claude sees "/my-command arg1 arg2" in message
    ↓
Claude invokes SlashCommand tool with full command
    ↓
Tool expands and returns result
```

## Configuration

Add `disable-model-invocation: false` to command frontmatter:

```markdown
---
description: My command description
disable-model-invocation: false
---

Process this request: $ARGUMENTS
```

## When to Use Model Invocation

**Use `disable-model-invocation: false` when**:
- Command needs context from conversation history
- Arguments should be interpreted by Claude
- Command behavior should adapt based on conversation
- Template placeholders aren't sufficient

**Use local execution (default) when**:
- Command has static template
- No conversation context needed
- Faster execution desired
- Template expansion is sufficient

## Examples

### Example 1: Context-Aware Command

```markdown
---
description: Analyze code with conversation context
disable-model-invocation: false
---

Analyze the following: $ARGUMENTS

Consider our previous discussion and coding standards.
```

### Example 2: Local Command (Default)

```markdown
---
description: Review PR
---

Review PR #$1 for code quality and correctness.
```

## Backward Compatibility

- **Default**: Commands without `disable-model-invocation` field default to `true` (local execution)
- **Existing commands**: Continue working as before
- **No breaking changes**: Opt-in feature only

## Testing

Test a command with model invocation:

```bash
# In command file: disable-model-invocation: false
/my-command test argument

# Verify Claude received: "/my-command test argument"
# Check tool execution logs
```

## Related Documentation

- [Slash Commands Overview](../README.md)
- [Custom Command Creation](./CUSTOM_COMMANDS.md)
- [SlashCommand Tool](../../crates/tools/src/slash_command.rs)
