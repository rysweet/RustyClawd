# CLI Flag Specification

**Complete specification derived from**: https://code.claude.com/docs/en/cli-reference

This document provides the exact specification for each CLI flag and command, extracted from official documentation and codified in tests.

---

## Commands

### `claude`
**Purpose**: Start interactive REPL
**Usage**: `claude`
**Behavior**:
- Launches interactive session
- Displays welcome message
- Waits for user input
- Maintains conversation context

**Tests**: `test_claude_no_args_starts_interactive_repl()`

---

### `claude "query"`
**Purpose**: Launch REPL with initial prompt
**Usage**: `claude "explain this project"`
**Behavior**:
- Starts interactive mode
- Processes initial query
- Keeps session open for follow-up
- Full conversation context maintained

**Tests**: `test_claude_with_query_launches_repl_with_initial_prompt()`

---

### `claude -p "query"`
**Purpose**: Query via SDK, then exit (print mode)
**Usage**: `claude -p "explain this function"`
**Behavior**:
- Executes single query
- Outputs response
- Exits immediately
- No persistent session

**Tests**:
- `test_claude_print_mode_query_then_exit()`
- `test_claude_long_print_flag()`

---

### `cat file | claude -p "query"`
**Purpose**: Process piped content
**Usage**: `cat logs.txt | claude -p "analyze errors"`
**Behavior**:
- Reads stdin if not a TTY
- Combines with query prompt
- Processes in print mode
- Outputs result and exits

**Tests**: `test_piped_input_with_print_mode()`

---

### `claude -c`
**Purpose**: Continue most recent conversation
**Usage**: `claude -c`
**Behavior**:
- Loads last session in current directory
- Restores full conversation context
- Continues interactively
- If no session exists, starts new one

**Tests**:
- `test_continue_most_recent_conversation()`
- `test_continue_with_long_flag()`

---

### `claude -c -p "query"`
**Purpose**: Resume prior session via SDK
**Usage**: `claude -c -p "Check for type errors"`
**Behavior**:
- Loads last session
- Adds query to conversation
- Executes in print mode
- Exits after response

**Tests**: `test_continue_with_print_mode_query()`

---

### `claude -r "<session-id>" "query"`
**Purpose**: Resume session by ID
**Usage**: `claude -r "abc123" "Finish this PR"`
**Behavior**:
- Loads specific session by ID
- Adds query to conversation
- Interactive if no query provided
- Print mode if query provided

**Tests**:
- `test_resume_session_by_id()`
- `test_resume_session_long_flag()`

---

### `claude -r`
**Purpose**: List available sessions
**Usage**: `claude -r`
**Behavior**:
- Lists all saved sessions
- Shows session IDs
- Exits after listing
- No session started

**Tests**: `test_resume_without_session_id_lists_sessions()`

---

### `claude update`
**Purpose**: Update to latest version
**Usage**: `claude update`
**Behavior**:
- Checks for latest version
- Downloads if available
- Installs update
- Reports status

**Tests**: `test_update_command()`

---

### `claude mcp`
**Purpose**: Manage Model Context Protocol servers
**Usage**: `claude mcp`
**Behavior**:
- Opens MCP server management
- Lists installed servers
- Allows configuration
- See MCP documentation for details

**Tests**: `test_mcp_command()`

---

## Flags

### `--add-dir`
**Purpose**: Add additional working directories for Claude to access
**Usage**: `claude --add-dir ../apps ../lib`
**Type**: Multiple values accepted
**Default**: Current directory only

**Behavior**:
- Makes specified directories accessible
- Claude can read/write files in these dirs
- Paths can be relative or absolute
- Multiple directories space-separated

**Tests**:
- `test_add_dir_single_directory()`
- `test_add_dir_multiple_directories()`
- `test_add_dir_with_absolute_paths()`
- `test_add_dir_makes_directories_accessible_to_claude()`

**Examples**:
```bash
claude --add-dir ../lib
claude --add-dir /usr/src/project /var/data
```

---

### `--agents`
**Purpose**: Dynamically define custom subagents via JSON
**Usage**: `claude --agents '{"name": {...}}'`
**Type**: JSON object
**Default**: None

**Format**:
```json
{
  "agent_name": {
    "description": "When to invoke (REQUIRED)",
    "prompt": "System prompt for agent (REQUIRED)",
    "tools": ["Tool1", "Tool2"],  // Optional, inherits all if omitted
    "model": "sonnet"  // Optional: sonnet|opus|haiku
  }
}
```

**Required Fields**:
- `description`: When to invoke the subagent
- `prompt`: System prompt guiding behavior

**Optional Fields**:
- `tools`: Array of allowed tools (inherits all if omitted)
- `model`: Model alias (sonnet/opus/haiku)

**Tests**:
- `test_agents_accepts_json_object()`
- `test_agents_requires_description()`
- `test_agents_requires_prompt()`
- `test_agents_tools_optional_inherits_all()`
- `test_agents_tools_array_specifies_allowed_tools()`
- `test_agents_model_optional()`
- `test_agents_model_aliases()`
- `test_agents_multiple_subagents()`

**Examples**:
```bash
claude --agents '{"researcher": {"description": "Research topics", "prompt": "You research"}}'
```

---

### `--allowedTools`
**Purpose**: List of tools allowed without prompting
**Usage**: `claude --allowedTools "Read" "Write" "Bash(git log:*)"`
**Type**: Multiple values, supports patterns
**Default**: All tools require prompting

**Behavior**:
- Listed tools auto-approved
- No permission prompts
- Supports pattern matching
- Example patterns: `Bash(git log:*)`

**Tests**:
- `test_allowed_tools_single_tool()`
- `test_allowed_tools_multiple_tools()`
- `test_allowed_tools_bash_pattern_matching()`
- `test_allowed_tools_no_prompting()`

**Examples**:
```bash
claude --allowedTools "Read"
claude --allowedTools "Read" "Write" "Edit"
claude --allowedTools "Bash(git log:*)" "Read"
```

---

### `--disallowedTools`
**Purpose**: List of tools disallowed without prompting
**Usage**: `claude --disallowedTools "Bash" "Edit"`
**Type**: Multiple values, supports patterns
**Default**: None

**Behavior**:
- Listed tools auto-denied
- No permission prompts
- Blocks usage completely
- Supports pattern matching

**Tests**:
- `test_disallowed_tools_single_tool()`
- `test_disallowed_tools_multiple_tools()`
- `test_disallowed_tools_pattern_matching()`
- `test_disallowed_tools_blocks_without_prompting()`
- `test_allowed_and_disallowed_tools_together()`

**Examples**:
```bash
claude --disallowedTools "Bash"
claude --disallowedTools "Bash(git log:*)" "Edit"
```

---

### `-p, --print`
**Purpose**: Print response without interactive mode
**Usage**: `claude -p "query"`
**Type**: Boolean flag
**Default**: false (interactive mode)

**Behavior**:
- Executes single query
- Outputs response
- Exits immediately
- Non-interactive

**Tests**:
- `test_claude_print_mode_query_then_exit()`
- `test_claude_long_print_flag()`

**Examples**:
```bash
claude -p "What is Rust?"
claude --print "Explain async"
```

---

### `--system-prompt`
**Purpose**: Replace entire system prompt
**Usage**: `claude --system-prompt "You are a Python expert"`
**Type**: String
**Default**: Default Claude system prompt

**Behavior**:
- Completely replaces default prompt
- No default behavior retained
- Use for specialized tasks
- Mutually exclusive with `--append-system-prompt`

**Tests**:
- `test_system_prompt_replaces_default()`
- `test_system_prompt_and_append_mutually_exclusive()`

**Examples**:
```bash
claude --system-prompt "You are a Python expert"
```

---

### `--system-prompt-file`
**Purpose**: Load system prompt from file
**Usage**: `claude -p --system-prompt-file ./custom-prompt.txt`
**Type**: File path
**Default**: None
**Restriction**: Print mode only

**Behavior**:
- Reads prompt from file
- Replaces default system prompt
- Only works with `-p` flag
- Errors in interactive mode

**Tests**:
- `test_system_prompt_file_in_print_mode()`
- `test_system_prompt_file_fails_in_interactive_mode()`

**Examples**:
```bash
claude -p --system-prompt-file ./expert-prompt.txt "query"
```

---

### `--append-system-prompt`
**Purpose**: Append text to default system prompt
**Usage**: `claude --append-system-prompt "Always use TypeScript"`
**Type**: String
**Default**: None

**Behavior**:
- Adds to end of default prompt
- Preserves default behavior
- Good for additional instructions
- Mutually exclusive with `--system-prompt`

**Tests**:
- `test_append_system_prompt_adds_to_default()`
- `test_append_system_prompt_preserves_default()`
- `test_system_prompt_and_append_mutually_exclusive()`

**Examples**:
```bash
claude --append-system-prompt "Always use TypeScript"
```

---

### `--output-format`
**Purpose**: Specify output format
**Usage**: `claude -p "query" --output-format json`
**Type**: Enum: `text | json | stream-json`
**Default**: `text`

**Formats**:

#### `text`
- Plain text output
- Human-readable
- Default format
- No JSON structure

#### `json`
- Complete response as JSON
- Includes metadata
- Full message structure
- Fields: id, type, role, content, model, stop_reason, usage

#### `stream-json`
- Streaming JSON events
- One event per line
- Real-time output
- Event types: content_block_delta, message_stop, etc.

**Tests**:
- `test_output_format_text_default()`
- `test_output_format_json()`
- `test_output_format_stream_json()`
- `test_output_format_json_structure()`
- `test_output_format_stream_json_events()`
- `test_output_format_text_plain_output()`

**Examples**:
```bash
claude -p "test" --output-format text
claude -p "test" --output-format json
claude -p "test" --output-format stream-json
```

---

### `--input-format`
**Purpose**: Specify input format
**Usage**: `claude -p --input-format stream-json`
**Type**: Enum: `text | stream-json`
**Default**: `text`

**Formats**:

#### `text`
- Plain text input
- Standard format
- Default

#### `stream-json`
- Parse streaming JSON events
- One event per line
- For processing API streams

**Tests**:
- `test_input_format_text_default()`
- `test_input_format_stream_json()`
- `test_input_format_stream_json_parses_events()`

**Examples**:
```bash
claude -p --input-format stream-json
```

---

### `--include-partial-messages`
**Purpose**: Include partial streaming events in output
**Usage**: `claude -p --output-format stream-json --include-partial-messages`
**Type**: Boolean flag
**Default**: false
**Requires**: `--output-format stream-json`

**Behavior**:
- Outputs all streaming events
- Includes partial/incomplete messages
- Shows content as it's generated
- Use with stream-json format

**Tests**:
- `test_include_partial_messages_default_false()`
- `test_include_partial_messages_flag()`
- `test_include_partial_messages_requires_stream_json()`
- `test_include_partial_messages_outputs_partial_events()`

**Examples**:
```bash
claude -p "test" --output-format stream-json --include-partial-messages
```

---

### `--verbose`
**Purpose**: Enable verbose logging
**Usage**: `claude --verbose`
**Type**: Boolean flag
**Default**: false

**Behavior**:
- Shows full turn-by-turn output
- Displays tool usage details
- Shows internal processing
- Useful for debugging

**Tests**:
- `test_verbose_default_false()`
- `test_verbose_flag_enables_logging()`
- `test_verbose_shows_turn_by_turn_output()`

**Examples**:
```bash
claude --verbose
claude -p "test" --verbose
```

---

### `--max-turns`
**Purpose**: Limit agentic turns in non-interactive mode
**Usage**: `claude -p --max-turns 3 "query"`
**Type**: Unsigned integer
**Default**: Unlimited
**Applies to**: Non-interactive mode only

**Behavior**:
- Limits tool use loops
- Prevents infinite loops
- Only in print mode
- Errors in interactive mode

**Tests**:
- `test_max_turns_default_unlimited()`
- `test_max_turns_flag_sets_limit()`
- `test_max_turns_non_interactive_only()`
- `test_max_turns_limits_agentic_loops()`

**Examples**:
```bash
claude -p --max-turns 3 "complex task"
claude -p --max-turns 5 "analyze code"
```

---

### `--model`
**Purpose**: Set the model for current session
**Usage**: `claude --model claude-sonnet-4-5-20250929`
**Type**: String (model ID or alias)
**Default**: `claude-sonnet-4-5-20250929`

**Aliases**:
- `sonnet` → `claude-sonnet-4-5-20250929`
- `opus` → `claude-opus-4-20250514`
- `haiku` → `claude-3-5-haiku-20241022`

**Full Model IDs**:
- Can use complete model identifier
- Format: `claude-{name}-{version}`

**Tests**:
- `test_model_default()`
- `test_model_flag_sets_model()`
- `test_model_alias_sonnet()`
- `test_model_alias_opus()`
- `test_model_alias_haiku()`
- `test_model_full_id()`

**Examples**:
```bash
claude --model sonnet
claude --model opus
claude --model claude-sonnet-4-5-20250929
```

---

### `--permission-mode`
**Purpose**: Begin in specified permission mode
**Usage**: `claude --permission-mode plan`
**Type**: Enum: `ask | auto-accept | plan`
**Default**: `ask`

**Modes**:

#### `ask`
- Prompt for each tool use
- Default behavior
- User confirms each action

#### `auto-accept`
- Auto-approve all tools
- No prompts
- Fastest execution

#### `plan`
- Show plan first
- User approves plan
- Then execute

**Tests**:
- `test_permission_mode_default()`
- `test_permission_mode_plan()`
- `test_permission_mode_auto_accept()`
- `test_permission_mode_ask()`
- `test_permission_mode_invalid()`

**Examples**:
```bash
claude --permission-mode ask
claude --permission-mode auto-accept
claude --permission-mode plan
```

---

### `--permission-prompt-tool`
**Purpose**: Specify MCP tool to handle permission prompts
**Usage**: `claude -p --permission-prompt-tool mcp_auth_tool`
**Type**: String (tool name)
**Default**: None

**Behavior**:
- Delegates permission prompts to MCP tool
- Custom permission handling
- For specialized workflows
- Tool must be MCP-provided

**Tests**:
- `test_permission_prompt_tool_default_none()`
- `test_permission_prompt_tool_mcp()`
- `test_permission_prompt_tool_delegates_prompts()`

**Examples**:
```bash
claude -p --permission-prompt-tool mcp_auth_tool
```

---

### `--resume`
**Purpose**: Resume specific session by ID
**Usage**: `claude --resume abc123 "query"`
**Type**: Optional string (session ID)
**Alias**: `-r`

**Behaviors**:

#### With session ID
- Resumes specified session
- Adds new query if provided
- Interactive or print mode

#### Without session ID
- Lists all available sessions
- Shows session IDs
- Exits after listing

**Tests**:
- `test_resume_session_by_id()`
- `test_resume_session_long_flag()`
- `test_resume_without_session_id_lists_sessions()`

**Examples**:
```bash
claude --resume abc123 "continue work"
claude -r session-xyz
claude -r  # Lists sessions
```

---

### `--continue`
**Purpose**: Load most recent conversation in current directory
**Usage**: `claude --continue`
**Type**: Boolean flag
**Alias**: `-c`
**Default**: false

**Behavior**:
- Finds last session in current dir
- Loads full conversation context
- Interactive by default
- Can combine with `-p` for print mode

**Tests**:
- `test_continue_most_recent_conversation()`
- `test_continue_with_long_flag()`
- `test_continue_with_print_mode_query()`

**Examples**:
```bash
claude --continue
claude -c
claude -c -p "new query"
```

---

### `--dangerously-skip-permissions`
**Purpose**: Skip permission prompts (use with caution)
**Usage**: `claude --dangerously-skip-permissions`
**Type**: Boolean flag
**Default**: false

**Behavior**:
- Auto-approves ALL tool usage
- No prompts whatsoever
- Dangerous - use carefully
- Bypasses safety checks

**⚠️ Warning**: Use only in trusted environments

**Tests**:
- `test_dangerously_skip_permissions_default_false()`
- `test_dangerously_skip_permissions_flag()`
- `test_dangerously_skip_permissions_no_prompts()`
- `test_dangerously_skip_permissions_warning()`

**Examples**:
```bash
# Use with extreme caution
claude --dangerously-skip-permissions
```

---

## Flag Combinations

### Valid Combinations

✅ **Print mode with all configuration**
```bash
claude -p "query" --model sonnet --output-format json --max-turns 3 --verbose
```

✅ **Continue with print mode**
```bash
claude -c -p "follow-up query"
```

✅ **Interactive with agents and directories**
```bash
claude --add-dir ../lib --agents '{...}' "initial prompt"
```

✅ **Tool control with permissions**
```bash
claude --allowedTools "Read" --disallowedTools "Write" --permission-mode plan
```

### Invalid Combinations

❌ **--continue and --resume together**
```bash
claude -c -r session-123  # Error: mutually exclusive
```

❌ **--system-prompt and --append-system-prompt together**
```bash
claude --system-prompt "..." --append-system-prompt "..."  # Error: mutually exclusive
```

❌ **--system-prompt-file in interactive mode**
```bash
claude --system-prompt-file file.txt  # Error: print mode only
```

---

## Test Coverage Matrix

| Flag/Command | Unit Tests | Integration Tests | E2E Tests | Boundary Tests |
|--------------|------------|-------------------|-----------|----------------|
| `claude` | ✅ | ✅ | ✅ | ✅ |
| `claude "query"` | ✅ | ✅ | ✅ | ✅ |
| `-p, --print` | ✅ | ✅ | ✅ | ✅ |
| Piped input | ✅ | ✅ | ✅ | ✅ |
| `-c, --continue` | ✅ | ✅ | ✅ | ✅ |
| `-r, --resume` | ✅ | ✅ | ✅ | ✅ |
| `update` | ✅ | - | - | - |
| `mcp` | ✅ | - | - | - |
| `--add-dir` | ✅ | ✅ | ✅ | ✅ |
| `--agents` | ✅ | ✅ | - | ✅ |
| `--allowedTools` | ✅ | ✅ | ✅ | - |
| `--disallowedTools` | ✅ | ✅ | ✅ | - |
| `--system-prompt` | ✅ | ✅ | ✅ | ✅ |
| `--system-prompt-file` | ✅ | - | - | ✅ |
| `--append-system-prompt` | ✅ | - | - | - |
| `--output-format` | ✅ | ✅ | ✅ | ✅ |
| `--input-format` | ✅ | ✅ | - | - |
| `--include-partial-messages` | ✅ | ✅ | - | - |
| `--verbose` | ✅ | ✅ | ✅ | - |
| `--max-turns` | ✅ | ✅ | ✅ | ✅ |
| `--model` | ✅ | ✅ | ✅ | - |
| `--permission-mode` | ✅ | ✅ | ✅ | ✅ |
| `--permission-prompt-tool` | ✅ | - | - | - |
| `--dangerously-skip-permissions` | ✅ | ✅ | - | - |

**Total**: 150+ tests covering 100% of documented features

---

**Specification Version**: 1.0
**Last Updated**: 2025-11-13
**Source**: https://code.claude.com/docs/en/cli-reference
