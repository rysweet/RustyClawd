# Interactive Mode Implementation Guide

**For**: Builder Agent
**Status**: Ready for Implementation
**Reference**: INTERACTIVE_MODE_ARCHITECTURE.md (read first!)

---

## Quick Start Checklist

- [ ] Read `INTERACTIVE_MODE_ARCHITECTURE.md` for complete design
- [ ] Review test suite: `crates/cli/tests/interactive_mode_tests.rs`
- [ ] Create module structure in `crates/cli/src/interactive/`
- [ ] Implement modules in order (Phase 1 → Phase 4)
- [ ] Run tests after each module
- [ ] All 54 tests must pass before merging

---

## Project Structure

```
crates/cli/
├── src/
│   ├── main.rs              (existing)
│   ├── interactive/         (NEW)
│   │   ├── mod.rs           (module exports)
│   │   ├── session.rs       (Phase 1)
│   │   ├── input.rs         (Phase 1)
│   │   ├── history.rs       (Phase 1)
│   │   ├── repl.rs          (Phase 1)
│   │   ├── dispatcher.rs    (Phase 2)
│   │   ├── response.rs      (Phase 2)
│   │   ├── command_history.rs (Phase 3)
│   │   ├── background.rs    (Phase 3)
│   │   ├── output.rs        (Phase 3)
│   │   └── types.rs         (shared types)
│   └── ...
└── tests/
    ├── interactive_mode_tests.rs (existing)
    └── ...
```

---

## Module Implementation Order

### Phase 1: Core REPL (Days 1-3)

#### 1.1 Create `crates/cli/src/interactive/types.rs`

**Purpose**: Shared types used across modules

```rust
// crates/cli/src/interactive/types.rs

use std::collections::HashMap;
use std::time::SystemTime;
use uuid::Uuid;

/// Unique session identifier
pub type SessionId = String;

/// Unique task identifier for background operations
pub type TaskId = String;

/// Role in conversation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Role {
    User,
    Assistant,
}

/// Type of input received from user
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InputType {
    Prompt,
    BashCommand,
    SlashCommand,
    MemoryShortcut,
    FileMention,
}

/// Status of interactive session
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionStatus {
    Active,
    Paused,
    Closed,
}

/// Result of processing user input
pub enum DispatchResult {
    ApiResponse(String),
    StreamingResponse(Box<dyn Stream<Item = Result<String>>>),
    CommandOutput(String),
    StatusMessage(String),
}

/// Parsed user input
#[derive(Debug, Clone)]
pub struct ParsedInput {
    pub input_type: InputType,
    pub content: String,
    pub raw_input: String,
    pub command_name: Option<String>,
    pub arguments: Vec<String>,
    pub file_path: Option<String>,
    pub is_multiline: bool,
}

/// Entry in session history
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub id: String,
    pub role: Role,
    pub content: String,
    pub timestamp: String,  // ISO 8601
    pub input_type: InputType,
    pub metadata: HistoryMetadata,
}

/// Additional metadata for history entry
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HistoryMetadata {
    pub tool_used: Option<String>,
    pub exit_code: Option<i32>,
    pub duration_ms: Option<u64>,
}

impl HistoryEntry {
    pub fn new(
        role: Role,
        content: String,
        input_type: InputType,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            role,
            content,
            timestamp: chrono::Utc::now().to_rfc3339(),
            input_type,
            metadata: Default::default(),
        }
    }

    pub fn with_metadata(mut self, metadata: HistoryMetadata) -> Self {
        self.metadata = metadata;
        self
    }
}

/// Processed input ready for dispatch
pub struct ProcessedInput {
    pub parsed: ParsedInput,
    pub result: DispatchResult,
}
```

**Add to Cargo.toml**:
```toml
[dependencies]
uuid = { version = "1.6", features = ["v4", "serde"] }
chrono = { version = "0.4", features = ["serde"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
tokio = { version = "1.0", features = ["full"] }
futures = "0.3"
anyhow = "1.0"
thiserror = "1.0"
```

---

#### 1.2 Implement `crates/cli/src/interactive/input.rs`

**Purpose**: Parse and classify user input

```rust
// crates/cli/src/interactive/input.rs

use super::types::{InputType, ParsedInput};

pub struct InputHandler;

impl InputHandler {
    /// Parse input string into structured ParsedInput
    pub fn parse(input: &str) -> ParsedInput {
        let trimmed = input.trim();

        // Empty or whitespace-only input
        if trimmed.is_empty() {
            return ParsedInput {
                input_type: InputType::Prompt,  // Will be skipped by caller
                content: String::new(),
                raw_input: input.to_string(),
                command_name: None,
                arguments: Vec::new(),
                file_path: None,
                is_multiline: false,
            };
        }

        let is_multiline = input.contains('\n') || input.contains('\\');

        // Bash command: !command
        if trimmed.starts_with('!') {
            return ParsedInput {
                input_type: InputType::BashCommand,
                content: trimmed[1..].trim().to_string(),
                raw_input: input.to_string(),
                command_name: None,
                arguments: Vec::new(),
                file_path: None,
                is_multiline,
            };
        }

        // Slash command: /command [arg1] [arg2]
        if trimmed.starts_with('/') {
            let parts: Vec<&str> = trimmed[1..].split_whitespace().collect();
            let command_name = parts.first().map(|s| s.to_string());
            let arguments = parts.iter().skip(1).map(|s| s.to_string()).collect();

            return ParsedInput {
                input_type: InputType::SlashCommand,
                content: trimmed[1..].to_string(),
                raw_input: input.to_string(),
                command_name,
                arguments,
                file_path: None,
                is_multiline,
            };
        }

        // Memory shortcut: #text
        if trimmed.starts_with('#') {
            return ParsedInput {
                input_type: InputType::MemoryShortcut,
                content: trimmed[1..].trim().to_string(),
                raw_input: input.to_string(),
                command_name: None,
                arguments: Vec::new(),
                file_path: None,
                is_multiline,
            };
        }

        // File mention: @path
        if trimmed.starts_with('@') {
            return ParsedInput {
                input_type: InputType::FileMention,
                content: trimmed.to_string(),
                raw_input: input.to_string(),
                command_name: None,
                arguments: Vec::new(),
                file_path: Some(trimmed[1..].to_string()),
                is_multiline,
            };
        }

        // Default: standard prompt
        ParsedInput {
            input_type: InputType::Prompt,
            content: trimmed.to_string(),
            raw_input: input.to_string(),
            command_name: None,
            arguments: Vec::new(),
            file_path: None,
            is_multiline,
        }
    }

    /// Check if input is complete (not waiting for multiline continuation)
    pub fn is_complete(input: &str) -> bool {
        // Complete if not ending with backslash (except at prompt)
        !input.trim_end().ends_with('\\')
    }

    /// Get continuation prompt for multiline input
    pub fn continuation_prompt() -> &'static str {
        "... "
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_standard_prompt() {
        let input = "explain this function";
        let parsed = InputHandler::parse(input);

        assert_eq!(parsed.input_type, InputType::Prompt);
        assert_eq!(parsed.content, "explain this function");
    }

    #[test]
    fn test_parse_bash_command() {
        let input = "!ls -la /tmp";
        let parsed = InputHandler::parse(input);

        assert_eq!(parsed.input_type, InputType::BashCommand);
        assert_eq!(parsed.content, "ls -la /tmp");
    }

    #[test]
    fn test_parse_slash_command_with_args() {
        let input = "/terminal-setup bash";
        let parsed = InputHandler::parse(input);

        assert_eq!(parsed.input_type, InputType::SlashCommand);
        assert_eq!(parsed.command_name, Some("terminal-setup".to_string()));
        assert_eq!(parsed.arguments, vec!["bash"]);
    }

    #[test]
    fn test_parse_memory_shortcut() {
        let input = "#remember this pattern";
        let parsed = InputHandler::parse(input);

        assert_eq!(parsed.input_type, InputType::MemoryShortcut);
        assert_eq!(parsed.content, "remember this pattern");
    }

    #[test]
    fn test_parse_file_mention() {
        let input = "@src/main.rs";
        let parsed = InputHandler::parse(input);

        assert_eq!(parsed.input_type, InputType::FileMention);
        assert_eq!(parsed.file_path, Some("src/main.rs".to_string()));
    }

    #[test]
    fn test_parse_empty_input() {
        let input = "";
        let parsed = InputHandler::parse(input);
        assert_eq!(parsed.content, "");
    }

    #[test]
    fn test_is_complete() {
        assert!(InputHandler::is_complete("hello world"));
        assert!(!InputHandler::is_complete("hello \\"));
        assert!(!InputHandler::is_complete("multi \\\nline \\"));
    }
}
```

---

#### 1.3 Implement `crates/cli/src/interactive/history.rs`

**Purpose**: Manage session message history

```rust
// crates/cli/src/interactive/history.rs

use super::types::{HistoryEntry, HistoryMetadata, InputType, Role};
use anyhow::{Result, Context};
use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use tokio::fs;

const MAX_HISTORY: usize = 10000;
const HISTORY_DIR: &str = ".claude/sessions";

pub struct SessionHistory {
    messages: VecDeque<HistoryEntry>,
    working_dir: PathBuf,
    session_file: PathBuf,
}

impl SessionHistory {
    /// Create new session history for given working directory
    pub async fn new(working_dir: PathBuf) -> Result<Self> {
        let session_file = Self::get_session_file(&working_dir);

        // Load existing history if available
        let messages = if session_file.exists() {
            Self::load_from_file(&session_file).await?
        } else {
            VecDeque::new()
        };

        Ok(Self {
            messages,
            working_dir,
            session_file,
        })
    }

    /// Add message to history
    pub fn add_message(
        &mut self,
        role: Role,
        content: String,
        input_type: InputType,
    ) -> Result<String> {
        let entry = HistoryEntry::new(role, content, input_type);
        let id = entry.id.clone();

        self.messages.push_back(entry);

        // Prune if exceeding max size
        if self.messages.len() > MAX_HISTORY {
            let to_remove = self.messages.len() - MAX_HISTORY + 100;
            for _ in 0..to_remove {
                self.messages.pop_front();
            }
        }

        Ok(id)
    }

    /// Get all messages
    pub fn get_all(&self) -> Vec<&HistoryEntry> {
        self.messages.iter().collect()
    }

    /// Get messages from specific turn onward
    pub fn get_from_turn(&self, turn: usize) -> Vec<&HistoryEntry> {
        self.messages
            .iter()
            .skip(turn)
            .collect()
    }

    /// Get last N messages (for context window)
    pub fn get_recent(&self, limit: usize) -> Vec<&HistoryEntry> {
        self.messages
            .iter()
            .rev()
            .take(limit)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect()
    }

    /// Clear all history
    pub fn clear(&mut self) {
        self.messages.clear();
    }

    /// Get message count
    pub fn len(&self) -> usize {
        self.messages.len()
    }

    /// Save history to disk
    pub async fn persist(&self) -> Result<()> {
        // Ensure directory exists
        let parent = self.session_file.parent().context("Invalid session file path")?;
        fs::create_dir_all(parent).await?;

        // Serialize to JSON
        let json = serde_json::to_string_pretty(&self.messages)?;

        // Write atomically (write to temp, then rename)
        let temp_file = self.session_file.with_extension("tmp");
        fs::write(&temp_file, json).await?;
        fs::rename(&temp_file, &self.session_file).await?;

        Ok(())
    }

    /// Load history from disk
    async fn load_from_file(path: &Path) -> Result<VecDeque<HistoryEntry>> {
        let json = fs::read_to_string(path).await?;
        let messages: Vec<HistoryEntry> = serde_json::from_str(&json)?;
        Ok(messages.into_iter().collect())
    }

    /// Get path to session file for given working directory
    fn get_session_file(working_dir: &Path) -> PathBuf {
        let home = dirs::home_dir().expect("Cannot determine home directory");
        let hash = format!("{:x}", fxhash::hash64(working_dir.to_string_lossy().as_bytes()));
        home.join(HISTORY_DIR)
            .join(format!("session_{}.json", hash))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_new_empty_history() {
        let history = SessionHistory::new(PathBuf::from(".")).await.unwrap();
        assert_eq!(history.len(), 0);
    }

    #[tokio::test]
    async fn test_add_message() {
        let mut history = SessionHistory::new(PathBuf::from(".")).await.unwrap();
        history.add_message(
            Role::User,
            "hello".to_string(),
            InputType::Prompt,
        ).unwrap();

        assert_eq!(history.len(), 1);
    }

    #[tokio::test]
    async fn test_get_all_preserves_order() {
        let mut history = SessionHistory::new(PathBuf::from(".")).await.unwrap();

        for i in 0..5 {
            history.add_message(
                Role::User,
                format!("message {}", i),
                InputType::Prompt,
            ).unwrap();
        }

        let all = history.get_all();
        assert_eq!(all.len(), 5);
        assert_eq!(all[0].content, "message 0");
        assert_eq!(all[4].content, "message 4");
    }

    #[tokio::test]
    async fn test_clear_history() {
        let mut history = SessionHistory::new(PathBuf::from(".")).await.unwrap();
        history.add_message(Role::User, "test".to_string(), InputType::Prompt).unwrap();
        assert_eq!(history.len(), 1);

        history.clear();
        assert_eq!(history.len(), 0);
    }
}
```

Add to Cargo.toml:
```toml
[dependencies]
dirs = "5.0"
fxhash = "0.2"
```

---

#### 1.4 Implement `crates/cli/src/interactive/session.rs`

**Purpose**: Main session orchestrator

```rust
// crates/cli/src/interactive/session.rs

use super::types::{
    DispatchResult, HistoryEntry, InputType, ProcessedInput, Role,
    SessionId, SessionStatus,
};
use super::input::InputHandler;
use super::history::SessionHistory;
use anyhow::{Result, Context};
use std::path::{Path, PathBuf};
use uuid::Uuid;

pub struct InteractiveSession {
    session_id: SessionId,
    working_dir: PathBuf,
    history: SessionHistory,
    status: SessionStatus,
}

impl InteractiveSession {
    /// Create new session in current working directory
    pub async fn new() -> Result<Self> {
        let working_dir = std::env::current_dir()?;
        Self::new_in_dir(working_dir).await
    }

    /// Create new session in specific directory
    pub async fn new_in_dir<P: AsRef<Path>>(dir: P) -> Result<Self> {
        let working_dir = dir.as_ref().to_path_buf();
        let history = SessionHistory::new(working_dir.clone()).await?;

        Ok(Self {
            session_id: Uuid::new_v4().to_string(),
            working_dir,
            history,
            status: SessionStatus::Active,
        })
    }

    /// Get current session status
    pub fn status(&self) -> SessionStatus {
        self.status
    }

    /// Get working directory
    pub fn working_dir(&self) -> &Path {
        &self.working_dir
    }

    /// Change working directory
    pub async fn set_working_dir<P: AsRef<Path>>(&mut self, dir: P) -> Result<()> {
        self.working_dir = dir.as_ref().to_path_buf();
        Ok(())
    }

    /// Get session history
    pub fn history(&self) -> &SessionHistory {
        &self.history
    }

    /// Get mutable session history
    pub fn history_mut(&mut self) -> &mut SessionHistory {
        &mut self.history
    }

    /// Process user input - main entry point for REPL
    pub async fn process_input(&mut self, input: &str) -> Result<ProcessedInput> {
        // Parse input
        let parsed = InputHandler::parse(input);

        // Skip empty input
        if parsed.content.is_empty() && parsed.input_type == InputType::Prompt {
            return Ok(ProcessedInput {
                parsed,
                result: DispatchResult::StatusMessage("Skipped empty input".to_string()),
            });
        }

        // Add to history (but not for empty inputs)
        if !parsed.content.is_empty() {
            self.history.add_message(
                Role::User,
                parsed.raw_input.clone(),
                parsed.input_type,
            )?;
        }

        // TODO: Route to dispatcher based on input type
        // For now, return placeholder
        Ok(ProcessedInput {
            parsed,
            result: DispatchResult::StatusMessage(
                "Input processed (dispatcher not yet implemented)".to_string()
            ),
        })
    }

    /// Save session state to disk
    pub async fn persist(&self) -> Result<()> {
        self.history.persist().await?;
        Ok(())
    }

    /// Close session cleanly
    pub async fn close(&mut self) -> Result<()> {
        self.persist().await?;
        self.status = SessionStatus::Closed;
        Ok(())
    }

    /// Get session ID
    pub fn session_id(&self) -> &str {
        &self.session_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_session_creation() {
        let session = InteractiveSession::new().await.unwrap();
        assert_eq!(session.status(), SessionStatus::Active);
        assert!(session.working_dir().exists());
    }

    #[tokio::test]
    async fn test_process_input_adds_to_history() {
        let mut session = InteractiveSession::new().await.unwrap();
        session.process_input("hello world").await.unwrap();

        assert_eq!(session.history().len(), 1);
    }

    #[tokio::test]
    async fn test_close_session() {
        let mut session = InteractiveSession::new().await.unwrap();
        session.close().await.unwrap();

        assert_eq!(session.status(), SessionStatus::Closed);
    }
}
```

---

#### 1.5 Create `crates/cli/src/interactive/mod.rs`

```rust
// crates/cli/src/interactive/mod.rs

pub mod types;
pub mod input;
pub mod history;
pub mod session;
pub mod repl;
pub mod dispatcher;
pub mod response;
pub mod command_history;
pub mod background;
pub mod output;

pub use session::InteractiveSession;
pub use types::{DispatchResult, InputType, ProcessedInput, SessionStatus};
pub use input::InputHandler;
pub use repl::run_interactive_session;
```

---

#### 1.6 Implement `crates/cli/src/interactive/repl.rs`

**Purpose**: Main REPL loop

```rust
// crates/cli/src/interactive/repl.rs

use super::session::InteractiveSession;
use anyhow::Result;
use std::io::{self, Write};

pub async fn run_interactive_session() -> Result<()> {
    println!("Claude Code Interactive Mode");
    println!("Type 'help' or '/help' for commands");
    println!("Press Ctrl+D to exit\n");

    let mut session = InteractiveSession::new().await?;

    loop {
        // 1. Display prompt
        let prompt = format!("Claude >> ");
        print!("{}", prompt);
        io::stdout().flush()?;

        // 2. Read input
        let mut input = String::new();
        let bytes_read = io::stdin().read_line(&mut input)?;

        // Check for EOF (Ctrl+D)
        if bytes_read == 0 {
            println!("\nGoodbye!");
            session.close().await?;
            break;
        }

        // Remove newline
        let input = input.trim_end();

        // 3. Process input
        match session.process_input(input).await {
            Ok(processed) => {
                // 4. Display result
                match &processed.result {
                    super::types::DispatchResult::ApiResponse(text) => {
                        println!("{}", text);
                    }
                    super::types::DispatchResult::CommandOutput(output) => {
                        println!("{}", output);
                    }
                    super::types::DispatchResult::StatusMessage(msg) => {
                        println!("✓ {}", msg);
                    }
                    super::types::DispatchResult::StreamingResponse(_) => {
                        println!("[Streaming not yet implemented]");
                    }
                }
            }
            Err(e) => {
                // 5. Handle error
                eprintln!("Error: {}", e);
            }
        }
    }

    Ok(())
}
```

---

**Phase 1 Tests**: Run and ensure passing:
```bash
cargo test --test interactive_mode_tests test_parse_
cargo test --test interactive_mode_tests test_session_history_
cargo test --test interactive_mode_tests test_session_
```

**Expected**: 16/54 tests passing after Phase 1

---

### Phase 2: API Integration (Days 4-5)

Once Phase 1 is solid, implement:

#### 2.1 `crates/cli/src/interactive/dispatcher.rs`
- Route input to correct processor
- Handle slash commands
- Call API for prompts
- Execute bash for commands

#### 2.2 `crates/cli/src/interactive/response.rs`
- Stream responses from Claude API
- Handle SSE events
- Display in terminal
- Record in history

**Key Integration**:
```rust
// From existing API client
use claude_code_core::client::{Client, CreateMessageRequest};

// Create streaming request
let request = CreateMessageRequest::new(
    "claude-3-5-sonnet-20241022",
    context.get_messages(),
    4096,
);

// Stream response
let stream = client.create_message_stream(request).await?;
```

---

### Phase 3: Advanced Features (Days 6-7)

#### 3.1 `crates/cli/src/interactive/command_history.rs`
- Navigate command history (arrow keys)
- Reverse search (Ctrl+R)

#### 3.2 `crates/cli/src/interactive/background.rs`
- Background task execution
- Task ID tracking
- Output buffering

#### 3.3 `crates/cli/src/interactive/output.rs`
- Verbose mode toggle (Ctrl+O)
- Tool detail formatting
- Terminal output control

---

### Phase 4: Polish & Testing (Days 8)

- Ensure all 54 tests pass
- Handle edge cases
- Performance optimization
- Documentation

---

## Testing Guide

### Running Specific Test Categories

```bash
# Input parsing tests
cargo test --test interactive_mode_tests test_parse_

# Session tests
cargo test --test interactive_mode_tests test_session_

# History tests
cargo test --test interactive_mode_tests test_history_

# Single test
cargo test --test interactive_mode_tests test_parse_standard_prompt_input

# With output
cargo test --test interactive_mode_tests test_parse_standard_prompt_input -- --nocapture

# Debug (single-threaded)
cargo test --test interactive_mode_tests -- --test-threads=1

# All tests
cargo test --test interactive_mode_tests
```

### Test Coverage Goal

```
Phase 1 (16 tests):
  ✓ Input parsing (9)
  ✓ Session history (4)
  ✓ Session creation (3)

Phase 2 (20 tests):
  ✓ Multi-turn conversation (7)
  ✓ Command dispatch (6)
  ✓ Response handling (7)

Phase 3 (14 tests):
  ✓ Command history navigation (4)
  ✓ Background tasks (5)
  ✓ Output control (5)

Phase 4 (4 tests):
  ✓ E2E sessions (3)
  ✓ Session recovery (1)
```

---

## Common Patterns

### Pattern 1: Async/Await in Rust

```rust
// DON'T: Blocking
let result = expensive_operation();

// DO: Async with await
let result = expensive_operation().await?;

// DO: Streaming
let mut stream = api_call().await?;
while let Some(item) = stream.next().await {
    process(item);
}
```

### Pattern 2: Error Handling

```rust
// DON'T: Unwrap (panics on error)
let file = fs::read_to_string("path").unwrap();

// DO: Return error with context
let file = fs::read_to_string("path")
    .context("Failed to read config")?;

// DO: Custom error type
return Err(anyhow::anyhow!("Invalid input: {}", input));
```

### Pattern 3: Owned vs Borrowed

```rust
// DON'T: Clone unnecessarily
let content = self.history.get_all().clone();

// DO: Use references
for entry in self.history.get_all() {
    println!("{}", entry.content);
}

// DO: Explicit clone when needed
let owned = entry.clone();
```

### Pattern 4: Option vs Result

```rust
// Option: value may or may not exist
pub fn get_last(&self) -> Option<&HistoryEntry>

// Result: operation may fail
pub fn process_input(&mut self, input: &str) -> Result<ProcessedInput>

// Handle Option
if let Some(entry) = self.history.get_last() {
    println!("{}", entry.content);
}

// Handle Result
match self.process_input(input) {
    Ok(result) => println!("{:?}", result),
    Err(e) => eprintln!("Error: {}", e),
}
```

---

## Debugging Tips

### Enable Logging

```rust
// In main.rs or test
use tracing_subscriber;

tracing_subscriber::fmt()
    .with_max_level(tracing::Level::DEBUG)
    .init();

// Then use in code
tracing::debug!("Session created: {:?}", session_id);
```

### Print Debug Info

```rust
// Use dbg! macro
let parsed = InputHandler::parse(input);
dbg!(&parsed);

// Use println! in tests
println!("History length: {}", session.history().len());

// Run with --nocapture
cargo test -- --nocapture
```

### Run Single Test with Output

```bash
cargo test --test interactive_mode_tests test_session_creation_and_initialization -- --nocapture
```

---

## Integration Checklist

- [ ] Phase 1: Core REPL working
  - [ ] Input parsing correct
  - [ ] Session creation working
  - [ ] History storage functional
  - [ ] All Phase 1 tests pass

- [ ] Phase 2: API Integration
  - [ ] Dispatcher routing working
  - [ ] API calls succeed
  - [ ] Response streaming functional
  - [ ] All Phase 2 tests pass

- [ ] Phase 3: Advanced Features
  - [ ] Command history navigation
  - [ ] Background task execution
  - [ ] Output formatting
  - [ ] All Phase 3 tests pass

- [ ] Phase 4: Polish
  - [ ] All 54 tests passing
  - [ ] Error handling complete
  - [ ] Edge cases handled
  - [ ] Performance acceptable

---

## Common Issues & Solutions

### Issue: "Module not found"
**Solution**: Ensure `mod.rs` exports the module:
```rust
// In mod.rs
pub mod input;
pub use input::InputHandler;
```

### Issue: "Lifetime mismatch"
**Solution**: Clone when needed:
```rust
// Instead of:
let ptr = &self.messages;

// Use:
let owned = self.messages.clone();
```

### Issue: "Test hangs"
**Solution**: Use `#[tokio::test]` for async:
```rust
#[tokio::test]
async fn test_name() {
    // async code
}
```

### Issue: "Serialization error"
**Solution**: Add serde derives:
```rust
#[derive(Serialize, Deserialize)]
pub struct MyType {
    field: String,
}
```

---

## Code Review Checklist

Before submitting for merge:

- [ ] All 54 tests pass
- [ ] No compiler warnings
- [ ] Error handling complete (no unwrap/panic in library)
- [ ] Documentation added (doc comments)
- [ ] Examples provided for public APIs
- [ ] Performance acceptable (< 100ms startup)
- [ ] Memory usage reasonable (< 50MB baseline)
- [ ] Consistent with existing code style
- [ ] Git history clean (logical commits)

---

## Next Steps After Implementation

1. **Integration Testing**
   - Test interactive mode end-to-end
   - Verify Claude API integration
   - Test all slash commands

2. **Performance Optimization**
   - Profile startup time
   - Optimize memory usage
   - Cache appropriate data

3. **Documentation**
   - User guide for interactive mode
   - Keyboard shortcuts reference
   - Example workflows

4. **Feature Parity**
   - Compare with JavaScript version
   - Add missing features
   - Fix any behavioral differences

---

**Status**: Ready for builder implementation
**Estimated Time**: 7-8 days for full implementation
**Tests Passing**: 0/54 initially → 54/54 at completion

