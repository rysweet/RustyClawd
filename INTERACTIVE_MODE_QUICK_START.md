# Interactive Mode - Quick Start for Builder

**Read First**: This file
**Then Read**: `INTERACTIVE_MODE_ARCHITECTURE.md` (full design)
**Then Reference**: `INTERACTIVE_MODE_IMPLEMENTATION_GUIDE.md` (code examples)
**Test Source**: `crates/cli/tests/interactive_mode_tests.rs` (54 tests)

---

## What You're Building

An interactive REPL/chat mode for Claude Code where users can:

```
> chat with Claude in real-time
> execute bash commands with !bash
> use slash commands like /clear, /help
> navigate command history with arrow keys
> maintain persistent session history
> switch to background execution with &
```

---

## Architecture in One Page

```
Input (stdin)
    ↓
Input Parser (5 types: prompt, !bash, /slash, #memory, @file)
    ↓
Command Dispatcher (routes to processor)
    ├─ Prompts → Claude API (streaming)
    ├─ Bash → Tool executor (streaming output)
    ├─ Slash → Built-in handler (status message)
    ├─ Memory → Append to CLAUDE.md (confirm)
    └─ File → Show preview (confirm context)
    ↓
Session State (history, context, command history)
    ↓
Output Display (terminal with formatting)
    ↓
Persistent Storage (~/.claude/sessions/)
```

---

## Key Files You'll Create

| File | Purpose | Lines | Priority |
|------|---------|-------|----------|
| `types.rs` | Shared types (Role, InputType, etc) | ~100 | P1 |
| `input.rs` | Parse input into 5 types | ~150 | P1 |
| `history.rs` | Store/load session messages | ~200 | P1 |
| `session.rs` | Main session orchestrator | ~150 | P1 |
| `repl.rs` | REPL main loop | ~80 | P1 |
| `dispatcher.rs` | Route to processors | ~200 | P2 |
| `response.rs` | Stream API responses | ~150 | P2 |
| `command_history.rs` | Navigate history | ~100 | P3 |
| `background.rs` | Background task mgmt | ~150 | P3 |
| `output.rs` | Terminal display | ~100 | P3 |

**Total**: ~1,300 lines across 10 files

---

## Phase Breakdown

### Phase 1: Core REPL (Days 1-3)
**Files**: `types.rs`, `input.rs`, `history.rs`, `session.rs`, `repl.rs`
**Tests Passing**: 16/54 (input parsing, session, history)
**Deliverable**: Can start REPL, parse input, store history

```bash
cargo test --test interactive_mode_tests test_parse_
cargo test --test interactive_mode_tests test_session_history_
cargo test --test interactive_mode_tests test_session_
```

### Phase 2: API & Streaming (Days 4-5)
**Files**: `dispatcher.rs`, `response.rs`
**Tests Passing**: +20 (multi-turn, response handling)
**Deliverable**: Can chat with Claude, see streaming responses

```bash
cargo test --test interactive_mode_tests test_multi_turn_
cargo test --test interactive_mode_tests test_command_input_
```

### Phase 3: Advanced Features (Days 6-7)
**Files**: `command_history.rs`, `background.rs`, `output.rs`
**Tests Passing**: +14 (history nav, background, output)
**Deliverable**: Full-featured REPL with all bells and whistles

```bash
cargo test --test interactive_mode_tests test_command_history_
cargo test --test interactive_mode_tests test_background_
```

### Phase 4: Polish (Day 8)
**All Tests**: 54/54 passing
**Deliverable**: Production-ready interactive mode

---

## Critical Success Factors

### 1. Input Parsing is Foundation
Get these 5 types right FIRST - everything else depends on it:

```rust
"hello" → Prompt (send to Claude)
"!ls" → BashCommand (execute)
"/clear" → SlashCommand (execute handler)
"#note" → MemoryShortcut (append to file)
"@file.rs" → FileMention (file path)
```

**Test**: `cargo test --test interactive_mode_tests test_parse_`

### 2. Session History is Core
Must persist and restore reliably:

```rust
~/.claude/sessions/session_{hash}.json
// Stores all messages with metadata
```

**Test**: `cargo test --test interactive_mode_tests test_session_history_`

### 3. Streaming is UX Gold
Don't batch responses - stream chunks as they arrive:

```rust
// DON'T: Wait for full response then display
let response = api_call().await?;
println!("{}", response);

// DO: Display as chunks arrive
let stream = api_stream().await?;
while let Some(chunk) = stream.next().await {
    print!("{}", chunk);
    stdout().flush()?;
}
```

### 4. Error Handling is Critical
Session must survive errors:

```rust
// DON'T: Panic/unwrap
let file = fs::read_to_string(path).unwrap();

// DO: Return error, show it, continue
let file = fs::read_to_string(path)
    .context("Failed to read config")?;
```

---

## Test-First Development

Run tests BEFORE coding to understand requirements:

```bash
# See what's expected
cargo test --test interactive_mode_tests -- --list

# See specific test code
cat crates/cli/tests/interactive_mode_tests.rs | grep -A 20 "test_parse_standard"

# Run and see failures
cargo test --test interactive_mode_tests test_parse_standard_prompt_input
```

The tests ARE the specification. Code to make them pass.

---

## Implementation Strategy

### Step 1: Understand Test Suite
```bash
# See how many tests there are
cargo test --test interactive_mode_tests 2>&1 | grep "test result"

# See test names
cargo test --test interactive_mode_tests -- --list --format terse | head -20
```

### Step 2: Implement Phase 1 (Core)
- Create `crates/cli/src/interactive/` directory
- Implement `types.rs` (shared types)
- Implement `input.rs` (input parsing)
- Implement `history.rs` (storage)
- Implement `session.rs` (main orchestrator)
- Implement `repl.rs` (main loop)

Run after each file:
```bash
cargo test --test interactive_mode_tests
```

### Step 3: Get Phase 1 Tests Green
- 9 tests for input parsing
- 4 tests for history
- 3 tests for session

All should pass with Phase 1 complete.

### Step 4: Implement Phase 2 (Integration)
- Implement `dispatcher.rs` (route commands)
- Implement `response.rs` (stream API responses)
- Integrate with `claude_code_core::client`

### Step 5: Continue to Phases 3-4
- Keep running tests
- Ensure no regressions
- All 54 tests passing

---

## Dependency Integration

### Existing Crates You'll Use

**API Client** (already exists):
```rust
use claude_code_core::client::{Client, CreateMessageRequest};

let client = Client::new(config);
let request = CreateMessageRequest::new(model, messages, max_tokens);
let stream = client.create_message_stream(request).await?;
```

**Tool Framework** (already exists):
```rust
use claude_code_tools::{BashTool, Tool};

let mut stream = BashTool.execute(params, &context).await?;
while let Some(event) = stream.next().await {
    // process event
}
```

**Core Types** (already exists):
```rust
use claude_code_core::{Message, Context, Role};

// Use these in your session
```

### New Dependencies to Add

```toml
[dependencies]
uuid = { version = "1.6", features = ["v4", "serde"] }
chrono = { version = "0.4", features = ["serde"] }
serde_json = "1.0"
dirs = "5.0"
fxhash = "0.2"
anyhow = "1.0"
thiserror = "1.0"
tokio = { version = "1.0", features = ["full"] }
futures = "0.3"
pin-project = "1.0"
```

---

## Code Style Guide

**Follow existing project patterns**:

```rust
// ✓ DO: Error handling with context
let content = fs::read_to_string(path)
    .context("Failed to read session history")?;

// ✓ DO: Async/await for I/O
pub async fn persist(&self) -> Result<()> {
    fs::write(&self.file_path, data).await?;
    Ok(())
}

// ✓ DO: Owned values for storage
pub fn add_message(&mut self, content: String) {
    self.messages.push(content);
}

// ✓ DO: References for reading
pub fn get_all(&self) -> Vec<&Message> {
    self.messages.iter().collect()
}

// ✗ DON'T: Panic on error
let content = fs::read_to_string(path).unwrap();

// ✗ DON'T: Unnecessary clones
let copy = self.history.clone();

// ✗ DON'T: Bare Result (add context)
fn process(input: &str) -> Result<Output> { ... }
```

---

## Testing Checklist

Before marking phase complete:

- [ ] Phase tests passing (cargo test)
- [ ] No compiler warnings
- [ ] No clippy warnings (cargo clippy)
- [ ] Code formatted (cargo fmt)
- [ ] Documentation added (doc comments on pub items)
- [ ] Tested with --nocapture to verify output
- [ ] Integration tests pass

```bash
# Full check
cargo fmt
cargo clippy --fix
cargo test --test interactive_mode_tests
```

---

## Debugging Session

### Enable Logs
```rust
use tracing::debug;

debug!("Session created: {}", self.session_id);
```

### Print in Tests
```rust
#[test]
fn test_something() {
    println!("Debug: {:?}", value);
    // Run with: cargo test -- --nocapture
}
```

### Check File System
```bash
# Where history files are stored
ls -la ~/.claude/sessions/

# See what was saved
cat ~/.claude/sessions/session_*.json | jq .
```

---

## Common Pitfalls

### 1. Forgetting Async
```rust
// ❌ WRONG - won't compile
pub fn load_history() -> Result<Vec<Message>> {
    let content = fs::read_to_string(path)?;  // Error: sync in async context
}

// ✓ RIGHT
pub async fn load_history() -> Result<Vec<Message>> {
    let content = fs::read_to_string(path).await?;
}
```

### 2. Clone vs Reference
```rust
// ❌ WRONG - unnecessary clone
pub fn get_messages(&self) -> Vec<Message> {
    self.messages.clone()  // Copies all data!
}

// ✓ RIGHT - return references
pub fn get_messages(&self) -> Vec<&Message> {
    self.messages.iter().collect()
}
```

### 3. Panicking on Error
```rust
// ❌ WRONG - crashes on error
let file = fs::read_to_string(path).unwrap();

// ✓ RIGHT - returns error for caller to handle
let file = fs::read_to_string(path)
    .context("Failed to read")?;
```

### 4. Missing Serialization
```rust
// ❌ WRONG - can't serialize
pub struct Message {
    content: String,
}

// ✓ RIGHT - add derive
#[derive(Serialize, Deserialize)]
pub struct Message {
    content: String,
}
```

---

## Success Metrics

### Completion Criteria

**Phase 1** (Days 1-3):
- ✓ 16/54 tests passing
- ✓ Can parse all 5 input types
- ✓ Can create/close sessions
- ✓ Can store/restore history

**Phase 2** (Days 4-5):
- ✓ +20 tests passing (36/54 total)
- ✓ Can send prompts to Claude API
- ✓ Can stream responses to terminal
- ✓ Can execute bash commands

**Phase 3** (Days 6-7):
- ✓ +14 tests passing (50/54 total)
- ✓ Can navigate command history
- ✓ Can spawn background tasks
- ✓ Can toggle verbose output

**Phase 4** (Day 8):
- ✓ 54/54 tests passing
- ✓ All error cases handled
- ✓ No compiler warnings
- ✓ Production ready

---

## Getting Unstuck

### If Tests Fail
```bash
# See detailed error
cargo test --test interactive_mode_tests test_name -- --nocapture

# Check specific test code
grep -A 30 "#\[test\]" crates/cli/tests/interactive_mode_tests.rs | grep -A 30 "test_name"

# Look at test setup
cargo test --test interactive_mode_tests -- --nocapture --test-threads=1
```

### If Compilation Fails
```bash
# See full error details
cargo build 2>&1 | head -50

# Check types
cargo check

# Fix formatting
cargo fmt

# Fix clippy
cargo clippy --fix
```

### If You're Lost
1. Read the architecture doc again
2. Look at the test that's failing
3. Check the implementation guide for code patterns
4. Review existing similar code in project

---

## Quick Reference: File Locations

```
Architecture: INTERACTIVE_MODE_ARCHITECTURE.md ← START HERE
Implementation: INTERACTIVE_MODE_IMPLEMENTATION_GUIDE.md
Quick Ref: INTERACTIVE_MODE_QUICK_START.md (this file)
Tests: crates/cli/tests/interactive_mode_tests.rs
Code: crates/cli/src/interactive/*.rs (what you'll create)
```

---

## Final Checklist Before Submission

- [ ] All 54 tests pass
- [ ] `cargo test` passes completely
- [ ] `cargo fmt` passes
- [ ] `cargo clippy` shows no errors
- [ ] No compiler warnings
- [ ] Documentation added (doc comments)
- [ ] Code reviewed against style guide
- [ ] Interactive mode works end-to-end
- [ ] Can chat with Claude
- [ ] Can execute bash commands
- [ ] Session history persists
- [ ] Error handling complete

---

## Timeline

**Days 1-3**: Phase 1 (Core REPL)
- Input parsing, session, history
- 16 tests passing

**Days 4-5**: Phase 2 (API Integration)
- Dispatcher, response handling
- 36 tests passing

**Days 6-7**: Phase 3 (Advanced Features)
- Command history, background, output
- 50 tests passing

**Day 8**: Phase 4 (Polish & Testing)
- Error handling, edge cases
- 54 tests passing
- **DONE**

---

**You're ready to start! Go build something awesome!**

Next: Read `INTERACTIVE_MODE_ARCHITECTURE.md` for complete design

