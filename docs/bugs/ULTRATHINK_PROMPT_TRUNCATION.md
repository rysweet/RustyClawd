# Prompt Truncation Bug Investigation

## Issue Summary

**Date:** 2025-02-10  
**Reporter:** User  
**Status:** Under Investigation

### Problem Description

When a user types a command starting with `/ultrathink`, followed by additional text/prompt, the AI assistant (Claude) does not receive the complete prompt. Only the `/ultrathink` portion is visible, causing the remainder of the input to be truncated or not delivered.

### Example

**User typed:**
```
/ultrathink there is a bug - we really should not reference Claude anywhere in the RustyClawd UI, but you do right at the startup
```

**AI received:**
```
Engage deep analysis mode. Think carefully through the problem step by step.
Consider edge cases, potential issues, and alternative approaches.
Provide a comprehensive analysis with detailed reasoning.

Engage deep analysis mode. Think carefully through the problem step by step.
Consider edge cases, potential issues, and alternative approaches.
Provide a comprehensive analysis with detailed reasoning.
```

Note: The actual task description was not visible to the AI.

### Technical Context

#### How SlashCommand Tool Works

1. **Command Definition:** `/ultrathink` is defined in `.claude/commands/amplihack/ultrathink.md`
2. **Tool Schema:** SlashCommand tool accepts `command` parameter as string (e.g., "/review-pr 123")
3. **Parsing:** Command parser splits on first space: command name vs. arguments
4. **Execution:** SlashCommandTool reads the markdown file and substitutes placeholders like `{{args}}` or `{TASK_DESCRIPTION}`

#### Code Flow

```
User Input: "/ultrathink <task>"
    ↓
Interactive Mode (interactive.rs:995-1020)
    ↓
SlashCommands.execute()
    ↓
CommandParser.parse() - Splits command and args
    ↓
SlashCommandTool.execute() - Expands template
    ↓
Returns expanded_prompt to conversation
```

### Investigation Points

#### 1. UI Input Capture Layer
**File:** `crates/cli/src/tui/input.rs`  
**Question:** Is the input field properly capturing multi-line or long inputs?  
**Status:** 🔍 Needs investigation

#### 2. Command Processing in Interactive Mode
**File:** `crates/cli/src/interactive.rs:995-1020`  
**Code:**
```rust
_ if input.starts_with('/') => {
    let command_name = input[1..].split_whitespace().next().unwrap_or("");
    
    if self.slash_commands.has_command(command_name) {
        // ... execution
    }
}
```
**Status:** ✅ Code looks correct

#### 3. Command Parser
**File:** `crates/cli/src/commands/parser.rs:58-88`  
**Status:** ✅ Parser correctly splits command and args

#### 4. Tool Schema and Execution
**File:** `crates/cli/src/tool_definitions.rs:336-351`  
**Schema:**
```json
{
  "command": {
    "type": "string",
    "description": "The slash command to execute with its arguments (e.g., '/review-pr 123')"
  }
}
```
**Status:** ✅ Schema is correct

#### 5. SlashCommand Tool Implementation
**File:** `crates/tools/src/slash_command.rs:64-67`  
**Code:**
```rust
let parts: Vec<&str> = command.trim_start_matches('/').splitn(2, ' ').collect();
let command_name = parts[0].to_string();
let args = parts.get(1).map(|s| s.to_string());
```
**Status:** ✅ Implementation is correct

### Hypotheses

#### Hypothesis 1: TUI Input Buffer Limitation
**Likelihood:** HIGH  
**Reasoning:** The repetition of text in what the AI saw suggests possible input buffering or display issue  
**Test:** Check if input field has character/line limits  
**Files to investigate:**
- `crates/cli/src/tui/input.rs`
- `crates/cli/src/tui/ui.rs`

#### Hypothesis 2: Clipboard or Terminal Paste Issue
**Likelihood:** MEDIUM  
**Reasoning:** Long paste operations might not be fully captured  
**Test:** Try typing vs. pasting the same command  
**Files to investigate:**
- Terminal emulator handling
- TUI paste event handling

#### Hypothesis 3: Async Race Condition
**Likelihood:** LOW  
**Reasoning:** Input processing might be interrupted before completion  
**Test:** Add logging to track full input capture  
**Files to investigate:**
- `crates/cli/src/interactive.rs` event loop

#### Hypothesis 4: API Request Truncation
**Likelihood:** LOW  
**Reasoning:** The full command might not be sent to Claude API  
**Test:** Add logging of outbound API requests  
**Files to investigate:**
- `crates/core/src/api.rs`
- Message construction before API call

### Reproduction Steps

1. Launch RustyClawd TUI: `cargo run`
2. Type: `/ultrathink <long text describing a task>`
3. Press Enter
4. Observe AI response - does it reference the full task?

### Debugging Steps

#### Step 1: Add Input Capture Logging
```rust
// In interactive.rs, around line 995
tracing::info!("FULL INPUT CAPTURED: {:?}", input);
tracing::info!("INPUT LENGTH: {}", input.len());
```

#### Step 2: Log Command Parsing
```rust
// In commands/parser.rs, after line 88
tracing::info!("Parsed command: name='{}', args_str='{:?}'", command_name, args_str);
```

#### Step 3: Log Tool Execution
```rust
// In slash_command.rs, after line 67
tracing::debug!("SlashCommand parts: command_name='{}', args='{:?}'", command_name, args);
```

#### Step 4: Log API Request Payload
```rust
// In api.rs or wherever messages are sent to Claude
tracing::info!("Sending to API: {} messages, total chars: {}", messages.len(), total_chars);
```

### Expected Behavior

When user types:
```
/ultrathink investigate the authentication flow
```

The AI should receive the expanded prompt from `ultrathink.md` with `{TASK_DESCRIPTION}` replaced by:
```
investigate the authentication flow
```

### Actual Behavior

The AI appears to receive a different prompt (possibly the frontmatter/metadata instead of the task).

### Next Steps

1. ✅ Document the bug comprehensively
2. 🔲 Add logging to trace full input flow
3. 🔲 Test with various input lengths
4. 🔲 Check TUI input handling code
5. 🔲 Verify API payload construction
6. 🔲 Create minimal reproduction test case
7. 🔲 Fix identified issue
8. 🔲 Add regression test

### Related Files

- `crates/cli/src/interactive.rs` - Main event loop
- `crates/cli/src/commands/parser.rs` - Command parsing
- `crates/cli/src/tui/input.rs` - Input capture
- `crates/tools/src/slash_command.rs` - Tool implementation
- `.claude/commands/amplihack/ultrathink.md` - Command definition

### Workaround

Until fixed, users can:
1. Use shorter commands/task descriptions
2. Type the task in multiple messages
3. Use regular chat instead of `/ultrathink` prefix

---

**Investigation Status:** 🔍 Active  
**Priority:** HIGH (affects core UX)  
**Assigned:** @AI Assistant

