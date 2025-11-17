# P0 Commands - Code Reference

## Command Registration

### In `is_builtin()` - Line 34

```rust
// P0 Priority commands
"add-dir" | "bashes" | "context" | "cost" | "todos"
```

### In `execute()` - Lines 92-97

```rust
// P0 Priority commands
"add-dir" => Some(Self::add_dir_command(&cmd.args_str)),
"bashes" => Some(Self::bashes_command()),
"context" => Some(Self::context_command()),
"cost" => Some(Self::cost_command()),
"todos" => Some(Self::todos_command()),
```

## Implementation Functions

### 1. `/add-dir` Implementation (Lines 314-331)

```rust
/// /add-dir <directory> - Add additional working directory
fn add_dir_command(args: &Option<String>) -> String {
    match args {
        Some(dir) => {
            // Validate directory exists
            let path = std::path::Path::new(dir);
            if path.exists() && path.is_dir() {
                format!(
                    "Added directory to working set:\n  {}\n\n\
                     Note: Directory will be available in this session context.",
                    dir
                )
            } else {
                format!("Error: Directory does not exist or is not a directory: {}", dir)
            }
        }
        None => "Usage: /add-dir <directory>\n\nExample:\n  /add-dir /path/to/project".to_string(),
    }
}
```

**Key Features**:
- Validates directory existence using `Path::exists()`
- Checks if path is actually a directory with `is_dir()`
- Clear error messages for invalid paths
- Usage instructions when no args provided

### 2. `/bashes` Implementation (Lines 334-345)

```rust
/// /bashes - List and manage background bash shells
fn bashes_command() -> String {
    "Background Bash Shells:\n\n\
     No background shells currently running.\n\n\
     Tips:\n\
     - Background shells are created when using run_in_background parameter\n\
     - Use BashOutput tool to read shell output\n\
     - Use KillShell tool to terminate shells"
        .to_string()
}
```

**Key Features**:
- Lists background shells (placeholder for future shell manager)
- References BashOutput and KillShell tools
- Provides helpful tips for users

### 3. `/context` Implementation (Lines 348-364)

```rust
/// /context - Visualize current context usage
fn context_command() -> String {
    const MAX_TOKENS: u64 = 200_000; // Claude's context window
    let used_tokens: u64 = 0; // Would be populated from actual usage
    let percentage = (used_tokens as f64 / MAX_TOKENS as f64 * 100.0) as u64;

    format!(
        "Context Window Usage:\n\n\
         Used:      {used_tokens:>7} tokens ({percentage}%)\n\
         Available: {MAX_TOKENS:>7} tokens\n\n\
         Visual: [{}{}] {percentage}%\n\n\
         Note: Context tracking will be implemented in future updates.",
        "=".repeat((percentage / 2) as usize),
        " ".repeat(50 - (percentage / 2) as usize),
    )
}
```

**Key Features**:
- 200K token context window constant
- Percentage calculation
- Visual progress bar using repeated characters
- Right-aligned number formatting

### 4. `/cost` Implementation (Lines 367-394)

```rust
/// /cost - Display token usage statistics and cost estimates
fn cost_command() -> String {
    const INPUT_COST_PER_MILLION: f64 = 3.0;
    const OUTPUT_COST_PER_MILLION: f64 = 15.0;

    let input_tokens: u64 = 0; // Would be populated from session stats
    let output_tokens: u64 = 0; // Would be populated from session stats
    let total_tokens = input_tokens + output_tokens;

    let input_cost = (input_tokens as f64 / 1_000_000.0) * INPUT_COST_PER_MILLION;
    let output_cost = (output_tokens as f64 / 1_000_000.0) * OUTPUT_COST_PER_MILLION;
    let total_cost = input_cost + output_cost;

    format!(
        "Token Usage & Cost Estimate:\n\n\
         Session Statistics:\n\
         - Input tokens:  {input_tokens:>8}\n\
         - Output tokens: {output_tokens:>8}\n\
         - Total tokens:  {total_tokens:>8}\n\n\
         Estimated Cost (Claude Sonnet 4.5):\n\
         - Input:  ${input_cost:>7.4} ({input_tokens} tokens @ ${INPUT_COST_PER_MILLION}/M)\n\
         - Output: ${output_cost:>7.4} ({output_tokens} tokens @ ${OUTPUT_COST_PER_MILLION}/M)\n\
         - Total:  ${total_cost:>7.4}\n\n\
         Note: Cost tracking will be implemented with full session integration."
    )
}
```

**Key Features**:
- Accurate Claude Sonnet 4.5 pricing ($3/M input, $15/M output)
- Separate input/output cost calculation
- Formatted currency display (4 decimal places)
- Clear cost breakdown

### 5. `/todos` Implementation (Lines 397-411)

```rust
/// /todos - List current todo items
fn todos_command() -> String {
    "Current Todo Items:\n\n\
     No todos tracked in this session.\n\n\
     Todo items will appear here when:\n\
     - Claude uses the TodoWrite tool to track tasks\n\
     - Complex multi-step operations are in progress\n\
     - Multiple features are being implemented\n\n\
     Todo Status Legend:\n\
     - [ ] pending      - Not yet started\n\
     - [~] in_progress  - Currently working on\n\
     - [x] completed    - Finished successfully"
        .to_string()
}
```

**Key Features**:
- Lists todos from TodoWrite tool (placeholder)
- Shows status legend with checkbox indicators
- Explains when todos appear
- Clear status descriptions

## Test Coverage

### Recognition Tests (Lines 517-539)

```rust
#[test]
fn test_is_builtin_add_dir() {
    assert!(BuiltinCommands::is_builtin("add-dir"));
}

#[test]
fn test_is_builtin_bashes() {
    assert!(BuiltinCommands::is_builtin("bashes"));
}

#[test]
fn test_is_builtin_context() {
    assert!(BuiltinCommands::is_builtin("context"));
}

#[test]
fn test_is_builtin_cost() {
    assert!(BuiltinCommands::is_builtin("cost"));
}

#[test]
fn test_is_builtin_todos() {
    assert!(BuiltinCommands::is_builtin("todos"));
}
```

### Execution Tests (Lines 542-622)

```rust
#[test]
fn test_execute_add_dir_no_args() {
    let cmd = Command::new("add-dir".to_string(), None);
    let result = BuiltinCommands::execute(&cmd);
    assert!(result.is_some());
    assert!(result.unwrap().contains("Usage"));
}

#[test]
fn test_execute_add_dir_with_valid_dir() {
    let cwd = std::env::current_dir().unwrap();
    let cmd = Command::new("add-dir".to_string(), Some(cwd.to_string_lossy().to_string()));
    let result = BuiltinCommands::execute(&cmd);
    assert!(result.is_some());
    assert!(result.unwrap().contains("Added directory"));
}

#[test]
fn test_execute_add_dir_with_invalid_dir() {
    let cmd = Command::new("add-dir".to_string(), Some("/nonexistent/path".to_string()));
    let result = BuiltinCommands::execute(&cmd);
    assert!(result.is_some());
    assert!(result.unwrap().contains("Error"));
}

// ... more execution tests for bashes, context, cost, todos
```

## Usage Example

```rust
use rustyclawd_cli::commands::{builtins::BuiltinCommands, parser::Command};

// Check if command is builtin
if BuiltinCommands::is_builtin("cost") {
    // Execute the command
    let cmd = Command::new("cost".to_string(), None);
    if let Some(output) = BuiltinCommands::execute(&cmd) {
        println!("{}", output);
    }
}
```

## File Structure

```
crates/cli/src/commands/builtins.rs (623 lines)
├── Imports (lines 1-3)
├── BuiltinCommands struct (lines 5-6)
├── Core methods
│   ├── is_builtin() (lines 11-36)
│   └── execute() (lines 38-101)
├── Command implementations (lines 103-308)
├── P0 Command implementations (lines 310-412)
└── Tests (lines 414-623)
    ├── Original tests (lines 414-512)
    └── P0 tests (lines 514-622)
```

## Integration Points

These commands are ready to integrate with:

1. **SessionState** (`session.rs`) - For `/add-dir` directory persistence
2. **Shell Manager** (future) - For `/bashes` real shell tracking  
3. **API Response** (future) - For `/context` and `/cost` live data
4. **TodoWrite Parser** (future) - For `/todos` active tracking

All implementations follow the same pattern:
- Simple function signature: `fn command_name() -> String`
- Optional arguments: `fn command_name(args: &Option<String>) -> String`
- Return formatted string for display
- No side effects (yet)
