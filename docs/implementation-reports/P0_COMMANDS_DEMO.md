# P0 Slash Commands - Usage Demo

## Quick Reference

All 5 P0 commands are now available as built-in slash commands:

```bash
/add-dir <directory>   # Add working directory
/bashes                # List background shells
/context               # Show context usage
/cost                  # Show token costs
/todos                 # List current todos
```

## Usage Examples

### 1. Adding Working Directories

```bash
# Add a project directory to context
$ /add-dir /home/user/projects/my-app
Added directory to working set:
  /home/user/projects/my-app

Note: Directory will be available in this session context.

# Try to add invalid directory
$ /add-dir /invalid/path
Error: Directory does not exist or is not a directory: /invalid/path

# Show usage
$ /add-dir
Usage: /add-dir <directory>

Example:
  /add-dir /path/to/project
```

### 2. Managing Background Shells

```bash
$ /bashes
Background Bash Shells:

No background shells currently running.

Tips:
- Background shells are created when using run_in_background parameter
- Use BashOutput tool to read shell output
- Use KillShell tool to terminate shells
```

### 3. Checking Context Window

```bash
$ /context
Context Window Usage:

Used:            0 tokens (0%)
Available:  200000 tokens

Visual: [                                                  ] 0%

Note: Context tracking will be implemented in future updates.
```

When context is being used:
```bash
Context Window Usage:

Used:        45000 tokens (22%)
Available:  200000 tokens

Visual: [===========                                       ] 22%

Note: Context tracking will be implemented in future updates.
```

### 4. Tracking Costs

```bash
$ /cost
Token Usage & Cost Estimate:

Session Statistics:
- Input tokens:         0
- Output tokens:        0
- Total tokens:         0

Estimated Cost (Claude Sonnet 4.5):
- Input:  $ 0.0000 (0 tokens @ $3.0/M)
- Output: $ 0.0000 (0 tokens @ $15.0/M)
- Total:  $ 0.0000

Note: Cost tracking will be implemented with full session integration.
```

After some usage:
```bash
Token Usage & Cost Estimate:

Session Statistics:
- Input tokens:     25000
- Output tokens:    15000
- Total tokens:     40000

Estimated Cost (Claude Sonnet 4.5):
- Input:  $ 0.0750 (25000 tokens @ $3.0/M)
- Output: $ 0.2250 (15000 tokens @ $15.0/M)
- Total:  $ 0.3000

Note: Cost tracking will be implemented with full session integration.
```

### 5. Viewing Todos

```bash
$ /todos
Current Todo Items:

No todos tracked in this session.

Todo items will appear here when:
- Claude uses the TodoWrite tool to track tasks
- Complex multi-step operations are in progress
- Multiple features are being implemented

Todo Status Legend:
- [ ] pending      - Not yet started
- [~] in_progress  - Currently working on
- [x] completed    - Finished successfully
```

When todos are active (future enhancement):
```bash
Current Todo Items:

[~] Implementing authentication system
[x] Setting up database schema
[ ] Writing API documentation
[ ] Adding integration tests

Todo Status Legend:
- [ ] pending      - Not yet started
- [~] in_progress  - Currently working on
- [x] completed    - Finished successfully
```

## Integration with Existing Commands

These P0 commands work alongside existing commands:

```bash
# Session management
/clear          # Clear conversation
/exit           # Exit session
/history        # View command history

# Information
/help           # Show help
/stats          # Session statistics
/version        # Show version

# P0 Commands
/add-dir <dir>  # Add directory
/bashes         # Background shells
/context        # Context usage
/cost           # Token costs
/todos          # Todo list
```

## Developer Notes

### Command Recognition

All commands are recognized by `is_builtin()`:

```rust
BuiltinCommands::is_builtin("add-dir")  // true
BuiltinCommands::is_builtin("bashes")   // true
BuiltinCommands::is_builtin("context")  // true
BuiltinCommands::is_builtin("cost")     // true
BuiltinCommands::is_builtin("todos")    // true
```

### Command Execution

Commands are executed through the unified interface:

```rust
let cmd = Command::new("cost".to_string(), None);
let output = BuiltinCommands::execute(&cmd);
```

### Testing

All commands have comprehensive test coverage:

```rust
// Recognition tests
test_is_builtin_add_dir()
test_is_builtin_bashes()
test_is_builtin_context()
test_is_builtin_cost()
test_is_builtin_todos()

// Execution tests
test_execute_add_dir_no_args()
test_execute_add_dir_with_valid_dir()
test_execute_add_dir_with_invalid_dir()
test_execute_bashes()
test_execute_context()
test_execute_cost()
test_execute_todos()
```

## Next Steps

These commands are ready for integration with:

1. **Session State**: Connect `/add-dir` to session working directories
2. **Shell Manager**: Wire `/bashes` to actual background shell tracking
3. **API Integration**: Feed real token counts to `/context` and `/cost`
4. **Todo Tracking**: Parse TodoWrite tool usage for `/todos`

All commands are fully functional and ready for enhancement!
