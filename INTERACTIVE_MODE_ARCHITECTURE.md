# Interactive Mode Architecture Specification

**Status**: DESIGN PHASE - Ready for Implementation
**Last Updated**: November 11, 2025
**Test Suite**: 54 tests (47 passing, 7 pending) in `/crates/cli/tests/interactive_mode_tests.rs`

---

## Executive Summary

This specification defines the complete architecture for Claude Code's interactive REPL/chat mode. The design follows these core principles:

- **Simplicity**: Minimal dependencies, clear control flow
- **Modularity**: Self-contained modules with clean interfaces
- **Testability**: All components independently testable
- **Streaming**: Real-time responses with async/await
- **Persistence**: Session history and state management

The architecture integrates with existing systems:
- **API Client**: Use `claude_code_core::client` for Claude API calls
- **Tool Framework**: Use `claude_code_tools::Tool` trait for command execution
- **Context Management**: Leverage `claude_code_core::Context` for conversation state

---

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────┐
│                    REPL LOOP                                │
│  (Interactive Session Entry Point)                          │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  ┌──────────────────────────────────────────────────────┐   │
│  │  INPUT HANDLER                                       │   │
│  │  ├─ Readline (history, completion)                  │   │
│  │  ├─ Input Parser (5 types)                          │   │
│  │  ├─ Command Recognition (!bash, /slash, #memory)   │   │
│  │  └─ Multiline Support (\ continuation)              │   │
│  └──────────────────────────────────────────────────────┘   │
│           ↓                                                  │
│  ┌──────────────────────────────────────────────────────┐   │
│  │  COMMAND DISPATCHER                                 │   │
│  │  ├─ Route to Processor                              │   │
│  │  ├─ Handle Slash Commands                           │   │
│  │  ├─ Execute Bash Commands                           │   │
│  │  └─ Send Prompts to Claude                          │   │
│  └──────────────────────────────────────────────────────┘   │
│           ↓                                                  │
│  ┌──────────────────────────────────────────────────────┐   │
│  │  SESSION STATE MANAGER                              │   │
│  │  ├─ Conversation Context                            │   │
│  │  ├─ Message History (per working directory)         │   │
│  │  ├─ Command History (for navigation)                │   │
│  │  ├─ Background Tasks                                │   │
│  │  └─ Output Controller                               │   │
│  └──────────────────────────────────────────────────────┘   │
│           ↓                                                  │
│  ┌──────────────────────────────────────────────────────┐   │
│  │  RESPONSE HANDLER                                   │   │
│  │  ├─ Stream Manager                                  │   │
│  │  ├─ Output Formatter                                │   │
│  │  ├─ Terminal Display                                │   │
│  │  └─ History Recorder                                │   │
│  └──────────────────────────────────────────────────────┘   │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

---

## Module Specification

### 1. Interactive Session Module

**Location**: `crates/cli/src/interactive/session.rs`

**Purpose**: Main orchestrator for the REPL loop

**Contract**:
```rust
pub struct InteractiveSession {
    context: ConversationContext,
    history: SessionHistory,
    command_history: CommandHistory,
    output_controller: OutputController,
    background_tasks: BackgroundTaskTracker,
    working_dir: PathBuf,
    state: SessionState,
}

impl InteractiveSession {
    /// Create new session in current working directory
    pub fn new() -> Result<Self>;

    /// Create session in specific directory
    pub fn new_in_dir(path: impl AsRef<Path>) -> Result<Self>;

    /// Process user input (main REPL entry point)
    pub async fn process_input(&mut self, input: &str) -> Result<ProcessedInput>;

    /// Get current session status
    pub fn status(&self) -> SessionStatus;

    /// Close session and cleanup resources
    pub async fn close(&mut self) -> Result<()>;

    /// Run complete interactive session (blocking REPL)
    pub async fn run(&mut self) -> Result<()>;
}

pub enum SessionStatus {
    Active,
    Paused,
    Closed,
}
```

**Key Responsibilities**:
1. Initialize session with working directory tracking
2. Main REPL loop implementation
3. Route input to appropriate processor
4. Maintain session state (active/closed)
5. Cleanup on exit (terminate background tasks)

**Dependencies**:
- `InputHandler` - Parse and classify input
- `CommandDispatcher` - Route commands
- `SessionHistory` - Store messages
- `ConversationContext` - Manage context
- `OutputController` - Display output

---

### 2. Input Handler Module

**Location**: `crates/cli/src/interactive/input.rs`

**Purpose**: Parse and classify user input into 5 types

**Contract**:
```rust
#[derive(Debug, Clone, PartialEq)]
pub enum InputType {
    Prompt,           // Standard text (send to Claude)
    BashCommand,      // !command
    SlashCommand,     // /command [args]
    MemoryShortcut,   // #text (append to CLAUDE.md)
    FileMention,      // @path (file autocomplete)
}

pub struct ParsedInput {
    pub input_type: InputType,
    pub content: String,        // Main content
    pub raw_input: String,      // Original input
    pub command_name: Option<String>,  // For slash/bash
    pub arguments: Vec<String>, // For slash commands
    pub file_path: Option<String>, // For file mention
    pub is_multiline: bool,     // \ continuation or \n present
}

pub struct InputHandler;

impl InputHandler {
    /// Parse input string into structured command
    pub fn parse(input: &str) -> ParsedInput;

    /// Check if input is complete (not waiting for multiline continuation)
    pub fn is_complete(input: &str) -> bool;
}

// Multiline support: input ending with \ triggers continuation prompt
// Example:
//   > define a function \
//   >   with multiple lines
```

**Input Type Rules**:

| Type | Prefix | Example | Behavior |
|------|--------|---------|----------|
| Prompt | None | "explain this" | Send to Claude |
| BashCommand | ! | "!ls -la" | Execute in shell |
| SlashCommand | / | "/clear" | Execute built-in |
| MemoryShortcut | # | "#remember this" | Append to CLAUDE.md |
| FileMention | @ | "@src/lib.rs" | Path completion |

**Edge Cases**:
- Empty input → Skip (don't add to history)
- Whitespace only → Skip
- Multiline with \ → Collect until complete
- Code blocks (newlines) → Treat as multiline prompt

---

### 3. Command Dispatcher Module

**Location**: `crates/cli/src/interactive/dispatcher.rs`

**Purpose**: Route parsed input to appropriate processor

**Contract**:
```rust
pub struct CommandDispatcher {
    api_client: Client,
    tool_executor: ToolExecutor,
}

impl CommandDispatcher {
    pub async fn dispatch(
        &self,
        parsed: ParsedInput,
        session: &InteractiveSession,
    ) -> Result<DispatchResult>;
}

pub enum DispatchResult {
    /// Response from Claude API
    ApiResponse(String),

    /// Output from command execution
    CommandOutput(String),

    /// Internal command result (status message)
    StatusMessage(String),

    /// Stream of chunks for real-time display
    StreamingResponse(Pin<Box<dyn Stream<Item = Result<String>>>>),
}
```

**Routing Logic**:

```
ParsedInput
  ├─ InputType::Prompt
  │   ├─ Check for special patterns (/#context, /@file)
  │   ├─ Build request with conversation context
  │   └─ Call API with streaming (see 4. Response Handler)
  │
  ├─ InputType::BashCommand
  │   ├─ Execute via bash tool
  │   ├─ Stream output to terminal
  │   └─ Record in history
  │
  ├─ InputType::SlashCommand
  │   ├─ Match command name
  │   ├─ Execute built-in handler
  │   └─ Return status message
  │
  ├─ InputType::MemoryShortcut
  │   ├─ Append to ~/.claude/memory.md
  │   └─ Confirm with user
  │
  └─ InputType::FileMention
      ├─ Expand to full path
      ├─ Preview file contents
      └─ Propose adding to context
```

**Slash Commands** (Built-in):

| Command | Args | Effect |
|---------|------|--------|
| /clear | - | Clear history & context |
| /terminal-setup | shell | Configure terminal |
| /memory | - | Show CLAUDE.md contents |
| /context | - | Show current context |
| /exit | - | Close session |
| /rewind | turn# | Go back to turn N |
| /verbose | - | Toggle verbose output |
| /help | - | Show help |

---

### 4. Session History Module

**Location**: `crates/cli/src/interactive/history.rs`

**Purpose**: Persistent storage of session messages

**Contract**:
```rust
pub struct SessionHistory {
    messages: VecDeque<HistoryEntry>,
    working_dir: PathBuf,
    file_path: PathBuf,
}

pub struct HistoryEntry {
    pub id: Uuid,
    pub role: Role,         // User or Assistant
    pub content: String,
    pub timestamp: SystemTime,
    pub input_type: InputType,
    pub metadata: HistoryMetadata,
}

pub struct HistoryMetadata {
    pub tool_used: Option<String>,  // "bash", "api", etc.
    pub exit_code: Option<i32>,
    pub duration_ms: Option<u64>,
}

impl SessionHistory {
    /// Add message to history
    pub fn add_message(
        &mut self,
        role: Role,
        content: String,
        input_type: InputType,
    ) -> Result<Uuid>;

    /// Get all messages in chronological order
    pub fn get_all(&self) -> Vec<&HistoryEntry>;

    /// Get messages from turn N onward (for context window)
    pub fn get_from_turn(&self, turn: usize) -> Vec<&HistoryEntry>;

    /// Clear history
    pub fn clear(&mut self) -> Result<()>;

    /// Save to persistent storage
    pub async fn persist(&self) -> Result<()>;

    /// Load from persistent storage
    pub async fn restore(working_dir: PathBuf) -> Result<Self>;
}
```

**Storage Format** (per working directory):

```
~/.claude/sessions/
├── work_project_path_hash.json    # Session history
├── memory.md                       # Shared memory file
└── settings.toml                   # Session settings
```

**History Entry JSON**:
```json
{
  "id": "uuid-v4",
  "role": "user|assistant",
  "content": "...",
  "timestamp": "2025-11-11T10:30:00Z",
  "input_type": "prompt|bash|slash",
  "metadata": {
    "tool_used": "bash",
    "exit_code": 0,
    "duration_ms": 250
  }
}
```

---

### 5. Command History Module

**Location**: `crates/cli/src/interactive/command_history.rs`

**Purpose**: Navigate command history (separate from session history)

**Contract**:
```rust
pub struct CommandHistory {
    commands: VecDeque<String>,
    position: usize,
    search_results: Option<Vec<String>>,
}

impl CommandHistory {
    /// Add command to history
    pub fn add_command(&mut self, cmd: String);

    /// Navigate up (Alt+Up or Ctrl+P)
    pub fn navigate_up(&mut self) -> Option<String>;

    /// Navigate down (Alt+Down or Ctrl+N)
    pub fn navigate_down(&mut self) -> Option<String>;

    /// Search history (Ctrl+R reverse search)
    pub fn search(&mut self, pattern: &str) -> Vec<String>;

    /// Cycle through search results
    pub fn next_search_result(&mut self) -> Option<String>;

    /// Clear all history
    pub fn clear(&mut self);
}
```

**Navigation Example**:
```
> cargo build
> cargo test
> cargo fmt

Press Up:  → Shows "cargo fmt"
Press Up:  → Shows "cargo test"
Press Up:  → Shows "cargo build"
Press Up:  → None (at beginning)
Press Down → Shows "cargo test"
Press Down → Shows "cargo fmt"
Press Down → None (at end)
```

---

### 6. Output Controller Module

**Location**: `crates/cli/src/interactive/output.rs`

**Purpose**: Format and display output in terminal

**Contract**:
```rust
pub struct OutputController {
    verbose_mode: bool,
    last_displayed: String,
}

impl OutputController {
    /// Format output for terminal display
    pub fn format_output(&self, output: &str) -> String;

    /// Format tool output with metadata (when verbose)
    pub fn format_tool_output(
        &self,
        tool: &str,
        duration_ms: u64,
        output: &str,
    ) -> String;

    /// Display streaming chunk in real-time
    pub async fn display_chunk(&mut self, chunk: &str) -> Result<()>;

    /// Toggle verbose mode (Ctrl+O)
    pub fn toggle_verbose(&mut self);

    /// Clear screen (Ctrl+L)
    pub async fn clear_screen(&mut self) -> Result<()>;

    /// Get last displayed output
    pub fn get_last_displayed(&self) -> &str;
}
```

**Output Modes**:

1. **Normal Mode** (default):
   ```
   > cargo test
   running 10 tests

   test result: ok. 10 passed; 0 failed
   ```

2. **Verbose Mode** (Ctrl+O toggle):
   ```
   > cargo test
   TOOL: bash
   DURATION: 2.345s

   running 10 tests

   test result: ok. 10 passed; 0 failed
   ```

---

### 7. Response Handler Module

**Location**: `crates/cli/src/interactive/response.rs`

**Purpose**: Stream responses from Claude API to terminal

**Contract**:
```rust
pub struct ResponseHandler {
    client: Client,
    output_controller: OutputController,
}

impl ResponseHandler {
    /// Stream response from Claude API
    pub async fn stream_response(
        &mut self,
        request: CreateMessageRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<String>>>>>;

    /// Handle streaming chunk (called for each SSE event)
    pub async fn handle_chunk(&mut self, chunk: String) -> Result<()>;

    /// Add response to history
    pub fn record_response(&self, response: String) -> Result<()>;
}

/// Represents a streaming response chunk
pub struct ResponseChunk {
    pub content: String,
    pub is_complete: bool,
}
```

**Streaming Flow**:

```
1. User sends prompt
   ↓
2. Create API request with context
   ↓
3. Call client.create_message_stream()
   ↓
4. For each SSE event:
   ├─ Parse ContentBlockDelta
   ├─ Extract text delta
   ├─ Display in terminal (real-time)
   ├─ Buffer in memory
   ↓
5. On message_stop event:
   ├─ Finalize response
   ├─ Add to session history
   ├─ Display usage stats (verbose)
   ↓
6. Back to prompt
```

**Integration with API Client**:
```rust
// Create streaming request
let request = CreateMessageRequest::new(
    "claude-3-5-sonnet-20241022",
    context.get_messages(),  // From ConversationContext
    max_tokens,
);

// Stream from client
let stream = client.create_message_stream(request).await?;

// Listen to stream
pin_mut!(stream);
while let Some(event) = stream.next().await {
    match event? {
        StreamEvent::ContentBlockDelta { delta } => {
            output_controller.display_chunk(&delta.text).await?;
        }
        StreamEvent::MessageStop => break,
        _ => {}
    }
}
```

---

### 8. Background Task Manager Module

**Location**: `crates/cli/src/interactive/background.rs`

**Purpose**: Execute commands without blocking REPL

**Contract**:
```rust
pub struct BackgroundTaskTracker {
    tasks: HashMap<TaskId, TaskState>,
}

pub enum TaskState {
    Running {
        start_time: SystemTime,
        output_buffer: Vec<String>,
    },
    Completed {
        exit_code: i32,
        output: String,
        duration: Duration,
    },
}

impl BackgroundTaskTracker {
    /// Register and start background task
    pub fn spawn_task(
        &mut self,
        command: String,
    ) -> Result<TaskId>;

    /// Check if task is running
    pub fn is_running(&self, task_id: &TaskId) -> bool;

    /// Get task output (may be incomplete if running)
    pub fn get_output(&self, task_id: &TaskId) -> Option<String>;

    /// Wait for task and get result
    pub async fn wait_for_task(&mut self, task_id: &TaskId) -> Result<String>;

    /// Cleanup completed tasks
    pub fn cleanup(&mut self);
}

pub type TaskId = String;  // UUID format: "bg-{uuid}"
```

**Background Command Syntax**:
```
> !cargo test &
Task bg-123e4567-e89b-12d3-a456-426614174000 started
Use 'bg show bg-123e4567' to check progress
Use 'bg wait bg-123e4567' to wait for completion

> Some work continues...
> bg show bg-123e4567
Status: running (2.3s)
  12 tests passed

> bg wait bg-123e4567
✓ Completed (3.2s)
test result: ok. 12 passed; 0 failed
```

---

## REPL Loop Implementation

**Location**: `crates/cli/src/interactive/repl.rs`

**Main Loop Structure**:

```rust
pub async fn run_interactive_session() -> Result<()> {
    // 1. Initialize
    let mut session = InteractiveSession::new()?;
    let mut readline = LineEditor::new()?;  // For history/completion

    println!("Claude Code Interactive Mode");
    println!("Type 'help' or /help for commands\n");

    loop {
        // 2. Read Input
        let prompt = format!("Claude {} >> ", session.working_dir.display());
        let input = readline.readline(&prompt)?;

        if input.is_empty() {
            continue;  // Skip empty input
        }

        // 3. Check for EOF (Ctrl+D)
        if input == "EOF" {
            println!("Goodbye!");
            session.close().await?;
            break;
        }

        // 4. Process Input
        match session.process_input(&input).await {
            Ok(processed) => {
                // 5. Display Result
                match processed.result {
                    DispatchResult::ApiResponse(text) => {
                        println!("{}", text);
                    }
                    DispatchResult::StreamingResponse(stream) => {
                        // Display chunks as they arrive
                        pin_mut!(stream);
                        while let Some(Ok(chunk)) = stream.next().await {
                            print!("{}", chunk);
                            stdout().flush()?;
                        }
                        println!();
                    }
                    DispatchResult::CommandOutput(output) => {
                        println!("{}", output);
                    }
                    DispatchResult::StatusMessage(msg) => {
                        println!("✓ {}", msg);
                    }
                }
            }
            Err(e) => {
                // 6. Handle Error (don't exit)
                eprintln!("Error: {}", e);
                session.error_recovery()?;
            }
        }

        // 7. Save input to command history (for navigation)
        readline.add_history_entry(input);
    }

    Ok(())
}
```

**Exit Conditions**:
- Ctrl+D (EOF) → Clean exit
- /exit command → Clean exit
- Error recovery timeout (3 failures) → Exit with error
- Panic → Display error and exit

---

## Input/Output Examples

### Example 1: Multi-turn Conversation

```
Claude >> Explain Rust ownership
Assistant: Rust's ownership system is...
[full response streams in real-time]

Claude >> Can I have multiple mutable references?
Assistant: No, Rust prevents this...

Claude >> Show me an example
Assistant: Here's an example:
```

### Example 2: Bash Command Execution

```
Claude >> !cargo test
Running: cargo test
running 45 tests
...
test result: ok. 45 passed; 0 failed

Claude >> That worked great!
Assistant: Great! Your tests...
```

### Example 3: Slash Commands

```
Claude >> /clear
✓ History cleared

Claude >> /context
Current context window (5 messages):
1. User: "Explain Rust"
2. Assistant: "Rust is..."
...

Claude >> /rewind 2
✓ Rewound to turn 2 (context reset)

Claude >> /exit
Goodbye!
```

### Example 4: History Navigation

```
Claude >> cargo test
[output]

Claude >> cargo fmt
[output]

Claude >> [Press Up Arrow]
Claude >> cargo fmt

Claude >> [Press Up Arrow]
Claude >> cargo test

Claude >> [Ctrl+R "cargo"]
3 matches found:
1. cargo test
2. cargo fmt
```

---

## State Persistence

### Session State File Format

**Location**: `~/.claude/sessions/{working_dir_hash}.json`

```json
{
  "session_id": "uuid-v4",
  "working_directory": "/path/to/project",
  "created_at": "2025-11-11T10:00:00Z",
  "last_updated": "2025-11-11T10:30:00Z",
  "settings": {
    "verbose_mode": false,
    "extended_thinking": false,
    "permission_mode": "normal"
  },
  "messages": [
    {
      "id": "uuid-v4",
      "role": "user",
      "content": "Explain Rust ownership",
      "timestamp": "2025-11-11T10:05:00Z",
      "input_type": "prompt"
    },
    {
      "id": "uuid-v4",
      "role": "assistant",
      "content": "Rust's ownership system...",
      "timestamp": "2025-11-11T10:05:02Z",
      "input_type": "api_response"
    }
  ]
}
```

### Recovery on Session Reopen

```
> claude-code interactive
...Loading previous session in /path/to/project
Session restored: 15 messages loaded
Ready to continue!

Claude >>
```

---

## Error Handling

### Error Recovery Strategy

| Error Type | Recovery |
|------------|----------|
| API Error | Retry with backoff (3x), show error, continue |
| Bash Error | Display exit code, continue |
| File Not Found | Show path, suggest correction, continue |
| Parse Error | Show input, explain expected format, continue |
| Permission Denied | Show error, offer sudo option, continue |
| Network Timeout | Retry, suggest check connection, continue |

### Error Display Format

```
❌ Error: File not found: /tmp/nonexistent.txt
   Suggestion: Did you mean '/tmp/existing.txt'?
   Type '/help' for available commands

Claude >>
```

---

## Terminal UI Components

### Prompt Display
```
Claude [project-name] >> [cursor here]
```

### Streaming Response Display
```
Claude >> Explain async Rust
Assistant: Async Rust allows...
  ↳ Text streams here in real-time
  ↳ Multiple lines appear as they arrive
  ↳ No waiting for complete response
[DONE]

Claude >>
```

### Verbose Output Header
```
Claude >> !cargo test

Tool Execution:
  Command: cargo test
  Duration: 2.345s
  Status: success (exit code 0)

Output:
  running 45 tests
  ...
```

---

## Integration Points

### With API Client (`claude_code_core::client`)

```rust
// Create message with conversation context
let request = CreateMessageRequest {
    model: "claude-3-5-sonnet-20241022".to_string(),
    messages: session.get_conversation_context().get_messages(),
    max_tokens: 4096,
    system: Some(vec![SystemMessage {
        type_: "text".to_string(),
        text: "You are a helpful programming assistant".to_string(),
        cache_control: None,
    }]),
};

// Stream response
let stream = client.create_message_stream(request).await?;
response_handler.stream_response(stream).await?;
```

### With Tool Framework (`claude_code_tools::Tool`)

```rust
// Execute bash command
let params = BashParams {
    command: "cargo test".to_string(),
    timeout: 120000,
    description: None,
    run_in_background: false,
};

let mut stream = BashTool.execute(params, &context).await?;
while let Some(event) = stream.next().await {
    match event {
        ToolEvent::Result(output) => {
            output_controller.display_output(&output).await?;
        }
        _ => {}
    }
}
```

### With Context Management (`claude_code_core::Context`)

```rust
// Build request context
let context = session.get_conversation_context();
let mut request = CreateMessageRequest::new(...);

// Add windowed message history
for msg in context.get_windowed_messages(1000) {
    request.messages.push(msg);
}
```

---

## Testing Strategy

**Test Categories** (from `crates/cli/tests/interactive_mode_tests.rs`):

1. **Input Parsing** (9 tests)
   - Each input type recognized correctly
   - Multiline handling
   - Empty/whitespace input

2. **Session Management** (6 tests)
   - Session creation/initialization
   - Input processing flow
   - Command execution
   - EOF handling

3. **History Management** (4 tests)
   - Per-directory separation
   - Chronological ordering
   - Clear command

4. **Command History Navigation** (4 tests)
   - Up/down arrow navigation
   - Reverse search (Ctrl+R)
   - Highlighting

5. **Multi-turn Conversations** (7 tests)
   - Context preservation
   - Turn counting
   - Message threading

6. **Output Control** (5 tests)
   - Verbose toggle (Ctrl+O)
   - Tool detail formatting
   - Background task tracking

7. **Session Continuity** (6 tests)
   - Error resilience
   - Screen clear (Ctrl+L) with history preservation
   - Session rewind (Esc+Esc)
   - Extended thinking toggle (Tab)
   - Permission mode switching (Shift+Tab)

8. **Command I/O** (6 tests)
   - Input echoing
   - Bash execution
   - Output in history
   - Background tasks

9. **E2E Sessions** (3 tests)
   - Full workflow
   - Session cleanup
   - All input modes

**Running Tests**:
```bash
# All tests
cargo test --test interactive_mode_tests

# With output
cargo test --test interactive_mode_tests -- --nocapture

# Single category
cargo test --test interactive_mode_tests test_parse_

# Debug (single-threaded)
cargo test --test interactive_mode_tests -- --test-threads=1
```

---

## Implementation Phases

### Phase 1: Core REPL (3-4 days)
**Priority**: CRITICAL
- Input parsing and command recognition
- Basic session lifecycle (create/close)
- Bash command execution
- Session history storage

**Deliverables**:
- `input.rs` - Input parser (5 types)
- `session.rs` - Session manager
- `history.rs` - History storage
- Tests for all

### Phase 2: API Integration (2-3 days)
**Priority**: HIGH
- Claude API streaming
- Conversation context building
- Response handling and display
- Streaming to terminal

**Deliverables**:
- `response.rs` - Response handler
- `dispatcher.rs` - Command routing
- Integration with `claude_code_core::client`

### Phase 3: Advanced Features (2 days)
**Priority**: MEDIUM
- Command history navigation
- Slash command execution
- Background task management
- Verbose output formatting

**Deliverables**:
- `command_history.rs` - History navigation
- `background.rs` - Background tasks
- `output.rs` - Output controller

### Phase 4: Polish & Testing (1-2 days)
**Priority**: MEDIUM
- Terminal UI refinements
- Error handling improvements
- Session recovery
- All 54 tests passing

---

## Key Design Decisions

### 1. Why Separate History Types?
- **Session History**: Conversation flow (important for context)
- **Command History**: Command navigation (separate concerns)
- Keeps code focused and testable

### 2. Why Streaming Over Batch?
- Better UX (no waiting for complete response)
- Lower latency perception
- Can display response while still generating
- Natural for interactive mode

### 3. Why Per-Directory Sessions?
- Different projects have different contexts
- Users expect history to be project-specific
- Simplifies persistence (one file per project)

### 4. Why Background Tasks?
- Long-running commands shouldn't block REPL
- Users can continue working while building/testing
- Matches shell behavior users expect

### 5. Why Memory Windowing?
- Prevents unbounded context growth
- Balances context size vs memory usage
- Leverages lessons from original JavaScript version

---

## Future Enhancements

### Post-MVP Features
1. **Interactive Completions**
   - Tab-completion for files, commands
   - Suggestion menu for slash commands

2. **Session Management**
   - List saved sessions
   - Resume old sessions
   - Fork session (branch for experimentation)

3. **Rich Terminal UI**
   - Syntax highlighting in output
   - Progress bars for long operations
   - Mouse support for navigation

4. **Advanced Context**
   - Context pinning (keep certain messages)
   - Auto-pruning when over limit
   - Context statistics display

5. **Plugins**
   - Custom slash commands
   - Output formatters
   - Pre/post-processing hooks

---

## Performance Targets

- **Startup Time**: < 100ms (from CLI to prompt)
- **First Character Display**: < 500ms after sending prompt
- **Input Echo**: < 50ms from keystroke to display
- **History Load**: < 500ms for 1000 messages
- **Memory Usage**: < 50MB baseline, < 10MB per 1000 messages

---

## Security Considerations

1. **API Key Handling**
   - Store in `~/.claude/config.toml`
   - Use `secrecy` crate for sensitive data
   - Never log API keys

2. **Command Execution**
   - Warn before executing dangerous commands
   - Respect `.gitignore` patterns
   - Don't auto-execute shell code

3. **File Access**
   - Respect file permissions
   - No arbitrary file modification without confirmation
   - Show diffs before applying edits

4. **Session Files**
   - Store in `~/.claude/` (user-only access)
   - Don't include sensitive data in history
   - Option to exclude from history (--secure flag)

---

## Conclusion

This specification provides a complete blueprint for implementing Claude Code's interactive mode. The architecture is:

- **Modular**: Each component has a single responsibility
- **Testable**: All 54 tests can pass with clean implementation
- **Extensible**: Easy to add new commands and features
- **Integrated**: Works seamlessly with existing API client and tools
- **Production-Ready**: Handles errors, persistence, and edge cases

**Next Step**: Builder agent implements each module following this spec.

---

**Specification Version**: 1.0
**Created**: November 11, 2025
**Status**: READY FOR IMPLEMENTATION
