# RustyClawd Test Infrastructure - Component Summary

## Overview
RustyClawd is an independent, unofficial Rust implementation of a CLI tool compatible with Claude Code. This document provides a comprehensive overview of the **test infrastructure** that validates the implementation through Test-Driven Development (TDD).

**Status:** Tests written FIRST before implementation (TDD approach)  
**Total Test Coverage:** 200+ tests across 3 validation phases

---

## Test Infrastructure Components

### 1. CLI Reference Test Suite (`cli_doc_tests.rs`)
**Purpose:** Comprehensive validation of command-line interface against official Claude Code documentation  
**Source:** https://code.claude.com/docs/en/cli-reference  
**Total Tests:** 150+ tests  

#### Coverage Areas:
- **Basic Commands (11 tests)**
  - Interactive REPL startup (`claude`)
  - REPL with initial prompt (`claude "query"`)
  - Print mode - SDK then exit (`claude -p "query"`)
  - Piped content processing (`cat file | claude -p`)
  - Session continuation (`claude -c`)
  - Session resumption (`claude -r "id"`)
  - Update command (`claude update`)
  - MCP management (`claude mcp`)

- **CLI Flags**
  - `--add-dir` - Directory access permissions (4 tests)
  - `--agents` - Subagent configuration (6 tests)
  - `--allowedTools` / `--disallowedTools` - Tool filtering (8 tests)
  - `-p` / `--print` - Print mode (2 tests)
  - `--system-prompt` / `--system-prompt-file` / `--append-system-prompt` (6 tests)
  - `--permission-mode` - Permission levels (4 tests)
  - `--dangerously-skip-permissions` - Skip permission checks (2 tests)
  - `--model` - Model selection (3 tests)
  - `--output-format` - Output formatting (6 tests)
  - `--input-format` - Input parsing (3 tests)

- **Session Management**
  - Continue recent conversation (2 tests)
  - Resume by session ID (3 tests)
  - List available sessions (1 test)

**Documentation:** See `CLI_FLAG_SPECIFICATION.md` and `CLI_TEST_COVERAGE.md`

---

### 2. Interactive Mode Test Suite (`interactive_doc_tests.rs`)
**Purpose:** Validate interactive REPL features and keyboard shortcuts  
**Source:** https://code.claude.com/docs/en/interactive-mode  
**Total Tests:** 100+ tests  

#### Coverage Areas:
- **Keyboard Shortcuts**
  - `Ctrl+C` - Cancel input/generation (2 tests)
  - `Ctrl+D` - Exit session (2 tests)
  - `Ctrl+L` - Clear screen (1 test)
  - `Ctrl+O` - Open current directory in editor (1 test)
  - `Ctrl+R` - Search command history (2 tests)
  - `Ctrl+V` - Paste mode toggle (2 tests)
  - `Up/Down` - Navigate command history (2 tests)
  - `Esc+Esc` - Clear current input (1 test)
  - `Tab/Shift+Tab` - Autocomplete navigation (4 tests)

- **Multiline Input Methods**
  - Backslash continuation (`\` + Enter)
  - Option+Enter, Shift+Enter, Ctrl+J
  - Paste detection (5 tests)

- **Quick Command Prefixes**
  - `#` - Inline comments
  - `/` - Slash commands
  - `!` - Bash commands
  - `@` - File references (8 tests)

- **Vim Editor Mode**
  - Mode activation and switching (3 tests)
  - Navigation commands (4 tests)
  - Editing operations (5 tests)

- **Command History**
  - Per-directory persistence (3 tests)
  - Search and navigation (4 tests)
  - History clearing (2 tests)

- **Background Bash Commands**
  - Async execution with `Ctrl+B` (6 tests)
  - Task ID management (3 tests)
  - Output retrieval (4 tests)
  - Auto-cleanup (2 tests)

---

### 3. Slash Command Test Suite (`slash_command_tests.rs`)
**Purpose:** Validate custom slash command system  
**Total Tests:** 40+ tests  

#### Coverage Areas:
- `/help` command and built-in commands (5 tests)
- Custom command loading and execution (8 tests)
- Command expansion with template variables (6 tests)
- Argument passing (positional and `$ARGUMENTS`) (7 tests)
- Frontmatter parsing (YAML metadata) (5 tests)
- Error handling and edge cases (9 tests)

**Features Tested:**
- Command file discovery (.claude/commands/)
- Markdown file parsing
- Variable substitution
- Argument validation
- Error messages

---

### 4. Tool Schema Tests (`test_tool_schema.rs`)
**Purpose:** Validate MCP tool definitions and signatures  
**Total Tests:** 15+ tests  

#### Coverage:
- Tool signature validation
- Parameter type checking
- Required vs. optional parameters
- Tool metadata (descriptions, examples)
- JSON schema compliance

---

### 5. Integration Test Scripts
**Purpose:** Cross-component validation through bash scripts  
**Location:** `method*.sh` files  

#### Test Methods:

##### Method 1: Tool Signature Validation (`method1_tool_signatures.sh`)
- Validates all tools exist and accept correct parameters
- Tests: Bash, Read, Write, Edit, Grep, Glob, Task tools
- Verifies JSON response structure

##### Method 2: Behavioral Equivalence (`method2_behavioral_equivalence.sh`)
- Compares outputs between implementations
- Tests identical inputs produce identical outputs
- Validates tool behavior consistency

##### Method 3: CLI Parity (`method3_cli_parity.sh`)
- Ensures CLI flags work identically
- Tests command-line argument parsing
- Validates subcommand routing

##### Method 4: Error Alignment (`method4_error_alignment.sh`)
- Verifies error messages match specification
- Tests error codes and exit status
- Validates error recovery

##### Method 5: Integration Workflows (`method5_integration_workflows.sh`)
- End-to-end workflow validation
- Tests multi-step operations
- Validates state persistence

---

### 6. E2E Test Suite
**Purpose:** End-to-end validation of complete user workflows  
**Location:** `e2e/` directory  
**Total Tests:** 29+ comprehensive scenarios  

#### Phase 1: Rust Programmatic Tests
**Location:** `e2e/` (Rust test files)  

##### Test Categories:
1. **Slash Command TUI Integration** (`test_slash_command_tui_integration.rs`)
   - 4 tests covering /analyze, /debug workflows
   - TUI interaction validation
   - Error handling

2. **Skills Execution Context** (`test_skills_execution_context.rs`)
   - 5 tests for skill loading and execution
   - Context propagation
   - Multi-turn preservation

3. **Full Interactive Session** (`test_full_interactive_session.rs`)
   - 6 tests for complete session lifecycle
   - Tool execution workflow
   - Hook execution order
   - Error recovery

**Infrastructure Stubs:**
- `TestSession` - Session simulation framework
- `TestSessionBuilder` - Fluent API for test setup
- `TestSkillEnvironment` - Skill testing environment
- `MockLLM` - LLM response mocking

#### Phase 2: tmux Terminal Tests
**Location:** `e2e/tmux/`  
**Framework:** Bash-based tmux interaction  

##### Test Scripts:
- `test_slash_command_e2e.sh` - Slash command workflows
- `test_skills_e2e.sh` - Skills integration
- `test_complex_workflow.sh` - Multi-step workflows
- `run_all.sh` - Full suite runner

**Framework Functions (13 total):**
- Session Management: start_rustyclawd_session, cleanup_session, trap_cleanup
- Input Injection: send_keys, send_command
- Output Verification: expect_output, expect_no_output, wait_for_output
- Tool Verification: assert_tool_executed, verify_file_content
- Utilities: capture_pane, set_timeout, debug_pane

**Features:**
- Real terminal simulation
- Keyboard shortcut testing
- Visual output verification
- Multi-pane scenarios

#### Phase 3: YAML Scenario Tests
**Location:** `e2e/scenarios/`  
**Format:** Declarative YAML test definitions  
**Runner:** Python-based scenario executor (`runner.py`)  

##### Scenario Categories:

**Core Workflows:**
- `multi_turn_conversation.yaml` - Context preservation
- `slash_command_workflow.yaml` - /analyze command flow
- `skills_integration.yaml` - Skill + context integration

**Agentic Features:**
- `agentic_task.yaml` - Task creation and execution
- `agent_teams_basic.yaml` - Multi-agent coordination
- `background_agents.yaml` - Async agent operations
- `runtime_agents.yaml` - Dynamic agent spawning

**Permission System:**
- `permission_mode_toggle.yaml` - Mode switching
- `permission_mode_cycling.yaml` - Cycle through modes

**Memory & State:**
- `memory_system_basic.yaml` - Memory persistence
- `session_resume_basic.yaml` - Session restoration
- `task_management_basic.yaml` - Task tracking
- `task_management_dependencies.yaml` - Dependent tasks

**Tool Chains:**
- `tool_chain_read_write.yaml` - Read → Write workflows
- `mcp_tools_basic.yaml` - MCP tool integration

**Performance Modes:**
- `fast_mode_basic.yaml` - Fast response mode
- `extended_thinking_basic.yaml` - Deep thinking mode
- `extended_thinking_cancel.yaml` - Cancel thinking

**Error Handling:**
- `error_handling.yaml` - Error recovery workflows
- `error_recovery_api_timeout.yaml` - Timeout handling

**Edge Cases:**
- `edge_case_empty_input.yaml` - Empty input handling
- `edge_case_unicode.yaml` - Unicode support
- `edge_case_long_input.yaml` - Large input handling
- `edge_case_terminal_resize.yaml` - Window resize

**Smoke Tests:**
- `smoke_cli_basic.yaml` - Basic CLI functionality
- `smoke_binary_launch.yaml` - Binary startup

**Stress Tests:**
- `stress_rapid_input.yaml` - Rapid command entry

**YAML Scenario Features:**
- Step-by-step workflow definition
- Input/output assertions
- Timing constraints
- Screenshot capture
- Tag-based filtering

---

### 7. Command Integration Tests (`commands_integration_tests.rs`)
**Purpose:** Cross-component command validation  
**Total Tests:** 20+ tests  

#### Coverage:
- Command composition
- Argument forwarding
- Output streaming
- Error propagation
- Command chaining

---

### 8. MCP Server Testing
**Purpose:** Model Context Protocol server validation  
**Script:** `mcp_serve_manual_test.sh`  

#### Tests:
- MCP server startup
- Tool discovery
- Request/response handling
- Server lifecycle management

---

## Test Strategy & Pyramid

### Testing Pyramid Distribution:
- **60% Unit Tests** - Individual component validation
- **30% Integration Tests** - Cross-component interaction
- **10% E2E Tests** - Complete user workflows

### TDD Approach:
1. **Write Tests First** - All tests written before implementation
2. **Explicit Failures** - Tests fail with clear "Not implemented" messages
3. **Specification as Tests** - Tests document exact expected behavior
4. **Incremental Implementation** - Build to make tests pass

### Test Execution:
```bash
# Run all unit tests
cargo test

# Run specific test suite
cargo test --test cli_doc_tests
cargo test --test interactive_doc_tests
cargo test --test slash_command_tests

# Run integration scripts
./method1_tool_signatures.sh
./method2_behavioral_equivalence.sh
./method3_cli_parity.sh
./method4_error_alignment.sh
./method5_integration_workflows.sh

# Run E2E tests
cd e2e/tmux && ./run_all.sh
cd e2e/scenarios && python runner.py
```

---

## Documentation Files

### Test Documentation:
- **`CLI_FLAG_SPECIFICATION.md`** - Complete CLI flag specification
- **`CLI_TEST_COVERAGE.md`** - Coverage report for CLI tests
- **`e2e/TEST_SUMMARY.md`** - E2E test suite summary
- **`e2e/scenarios/README.md`** - YAML scenario documentation
- **`e2e/scenarios/RUNNER_README.md`** - Runner implementation guide
- **`e2e/scenarios/PHASE3_IMPLEMENTATION_SUMMARY.md`** - Phase 3 status
- **`e2e/tmux/README.md`** - tmux framework documentation

### Agentic Configuration:
- **`agentic/permissions_search_tui.yaml`** - Permission system TUI config

---

## Test Status

### Current State:
✅ **Tests Written:** 200+ comprehensive tests  
⏳ **Tests Passing:** Awaiting implementation  
🎯 **Test-First:** All tests define specification before code  

### Purpose:
These tests serve as:
1. **Executable Specifications** - Define exact expected behavior
2. **Implementation Guide** - Clear roadmap for builders
3. **Regression Protection** - Prevent breaking changes
4. **Documentation** - Living docs of system capabilities

---

## For Developers

### Adding New Tests:
1. Identify feature from documentation
2. Write failing test with clear assertions
3. Mark test with `#[ignore]` if infra not ready
4. Include link to specification source
5. Add test to appropriate category in docs

### Test Structure:
```rust
#[test]
fn test_feature_name() {
    // Arrange: Setup test conditions
    let config = setup_test_config();
    
    // Act: Execute the feature
    let result = execute_feature(config);
    
    // Assert: Verify expected behavior
    assert_eq!(result.status, Expected::Behavior);
}
```

### Running Tests:
- All tests should run with `cargo test`
- Integration tests may require environment setup
- E2E tests may need external dependencies
- Check test output for "Not implemented" messages

---

## Project Goals

These tests support the core project objectives:
- ✅ **Specification Completeness** - Every documented feature has tests
- ✅ **TDD Discipline** - Tests written before implementation
- ✅ **Maintainability** - Clear test structure and documentation
- ✅ **Confidence** - Comprehensive validation at all levels
- ✅ **Compatibility** - Parity with Claude Code CLI behavior

---

*For implementation details, see individual test files and documentation.*  
*For Claude Code documentation, visit: https://code.claude.com/docs/*
