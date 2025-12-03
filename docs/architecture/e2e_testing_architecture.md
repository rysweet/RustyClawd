# E2E Testing Architecture for RustyClawd
**Goal:** Achieve TRUE 100% Claude Code Parity

**Version:** 1.0
**Date:** 2025-12-03
**Status:** Architecture Design Phase

---

## Executive Summary

This architecture defines a comprehensive end-to-end testing strategy for RustyClawd that validates the system works exactly like Claude Code from a user's perspective. The design follows the **Testing Pyramid** principle (60% unit, 30% integration, 10% E2E) and implements a **three-phase approach** that progressively increases test coverage from 85% → 95% → 100% parity.

**Key Philosophy:** Zero-BS implementation - tests must validate actual behavior, not just pass. Every test represents a real user workflow.

---

## Current State Analysis

### What We Have (Strong Foundation)

1. **Hooks System:** 100% coverage (45 integration tests)
2. **Unit Tests:** 666 tests, comprehensive coverage of components
3. **Integration Tests:** 48 tests, component interactions validated
4. **TUI Test Harness:** Existing `ratatui::TestBackend` infrastructure
5. **Mock Infrastructure:** Test helpers for API clients and tool execution

### Critical Gaps (Why We're at 70%, Not 100%)

1. **No SlashCommand TUI Integration:** Commands work in isolation, TUI works separately
2. **No Skills Execution Context:** Skills load but execution context not validated
3. **No Full Session E2E:** Complete workflows not tested end-to-end
4. **No Real Terminal Tests:** All tests use mocks, none use real PTY/tmux
5. **No Agentic Scenarios:** No declarative YAML-based test scenarios

---

## High-Level Architecture

### Three-Phase Approach

```
Phase 1: Critical E2E Tests (Programmatic)
├── Target: 85% Parity (24 hours)
├── Approach: Rust-based integration tests with TestBackend
├── Focus: SlashCommand+TUI, Skills+Context, Full Session
└── Foundation: Existing test infrastructure

Phase 2: Real Terminal E2E Tests (tmux)
├── Target: 95% Parity (16 hours)
├── Approach: Bash scripts + tmux for real terminal interaction
├── Focus: Actual rendering, key input, terminal output
└── Validation: Tests catch real rendering/interaction bugs

Phase 3: Agentic Test Scenarios (YAML)
├── Target: 100% Parity (16 hours)
├── Approach: Declarative YAML scenarios + Rust runner
├── Focus: Complex multi-step workflows, reproducible scenarios
└── Reusability: Scenarios serve as living documentation
```

### Architecture Principles

**1. Ruthless Simplicity**
- Start with tmux (simplest real terminal testing)
- Add complexity only when justified
- Prefer clarity over cleverness

**2. Zero-BS Implementation**
- No test stubs that always pass
- No mocked behavior that doesn't reflect reality
- Every test MUST validate actual end-to-end behavior

**3. Modular Design (Bricks & Studs)**
- Each test framework is an independent "brick"
- Clear contracts between test layers
- Components can be regenerated independently

**4. Quality Over Speed**
- Take full 56 hours if needed
- Fix bugs discovered during testing
- Ensure tests are maintainable and clear

---

## Detailed Phase Designs

## Phase 1: Critical E2E Tests (Programmatic)

### Architecture Overview

```
┌─────────────────────────────────────────────────┐
│         Programmatic E2E Test Suite             │
├─────────────────────────────────────────────────┤
│                                                 │
│  ┌───────────────────────────────────────────┐ │
│  │  Test 1: SlashCommand TUI Integration    │ │
│  │  - Mock LLM, Real TUI, Real SlashCommand  │ │
│  │  - Validates command expansion in TUI     │ │
│  └───────────────────────────────────────────┘ │
│                                                 │
│  ┌───────────────────────────────────────────┐ │
│  │  Test 2: Skills Execution in Context      │ │
│  │  - Mock LLM, Real Skills, Real Context    │ │
│  │  - Validates context propagation          │ │
│  └───────────────────────────────────────────┘ │
│                                                 │
│  ┌───────────────────────────────────────────┐ │
│  │  Test 3: Full Interactive Session E2E     │ │
│  │  - Complete workflow validation           │ │
│  │  - Multi-turn conversation with tools     │ │
│  └───────────────────────────────────────────┘ │
│                                                 │
└─────────────────────────────────────────────────┘
         │              │              │
         ▼              ▼              ▼
┌─────────────┐ ┌──────────────┐ ┌──────────┐
│  TestBackend│ │  MockLLM     │ │  Hooks   │
│  (ratatui)  │ │  Client      │ │  System  │
└─────────────┘ └──────────────┘ └──────────┘
```

### Test Infrastructure Components

**1. Enhanced Test Harness**
- Extend existing `TuiTestHarness` with interaction support
- Add event injection (keyboard, mouse)
- Add state capture and assertions
- Integration with existing `TestBackend`

**2. Mock LLM Client**
- Controllable streaming responses
- Tool use injection
- Deterministic behavior for testing
- Error simulation capabilities

**3. Session Test Framework**
- Start/stop full interactive sessions
- Inject user input
- Capture TUI state at each step
- Verify hook execution
- Validate tool invocation

### Test 1: SlashCommand TUI Integration

**Purpose:** Validate that slash commands work seamlessly in the TUI

**Test Flow:**
```rust
#[tokio::test]
async fn test_slash_command_tui_integration() {
    // 1. Setup: Create test session with mock LLM
    let mut session = TestSession::new()
        .with_mock_llm()
        .with_real_tui()
        .build()
        .await;

    // 2. Action: User types /analyze src/
    session.send_input("/analyze src/").await;

    // 3. Verify: SlashCommandTool invoked
    assert!(session.tool_was_invoked("SlashCommand"));

    // 4. Verify: Command expanded in LLM context
    let context = session.get_llm_context();
    assert!(context.contains("Analyze the codebase"));

    // 5. Verify: TUI updated with command prompt
    assert!(session.tui_contains("/analyze src/"));

    // 6. Mock: LLM response
    session.inject_llm_response("Analysis: Found 42 modules").await;

    // 7. Verify: Response rendered in TUI
    assert!(session.tui_contains("42 modules"));
}
```

**Key Validations:**
- SlashCommandTool correctly invoked
- Command prompt expanded and sent to LLM
- TUI displays command input
- TUI renders LLM response
- Session state remains consistent

### Test 2: Skills Execution in Context

**Purpose:** Validate that skills execute with full conversation context

**Test Flow:**
```rust
#[tokio::test]
async fn test_skills_execution_in_context() {
    // 1. Setup: Create test skill file
    let skill_dir = TestSkillEnvironment::new()
        .with_skill("test-analyzer", "Perform deep analysis")
        .build();

    // 2. Setup: Session with conversation history
    let mut session = TestSession::new()
        .with_mock_llm()
        .with_skill_dir(skill_dir.path())
        .build()
        .await;

    // 3. Action: Establish context with prior messages
    session.add_conversation_turn(
        "User: What's in main.rs?",
        "Assistant: main.rs contains the entry point..."
    ).await;

    // 4. Action: Invoke skill via natural language
    session.send_input("Use test-analyzer skill on main.rs").await;

    // 5. Verify: SkillTool invoked
    assert!(session.tool_was_invoked("Skill"));

    // 6. Verify: Skill has access to prior context
    let tool_context = session.get_tool_context("Skill");
    assert!(tool_context.contains("entry point"));

    // 7. Verify: Skill prompt injected into LLM context
    let llm_context = session.get_llm_context();
    assert!(llm_context.contains("Perform deep analysis"));
}
```

**Key Validations:**
- Skills load correctly from disk
- Skills receive full conversation context
- Skill prompts injected into LLM context
- Tool parameters passed correctly
- Context preserved across turns

### Test 3: Full Interactive Session E2E

**Purpose:** Validate complete workflow from startup to shutdown

**Test Flow:**
```rust
#[tokio::test]
async fn test_full_interactive_session_e2e() {
    // 1. Setup: Real hooks, real tools, mock LLM
    let hooks = HooksSystem::new_with_real_hooks();
    let mut session = InteractiveSession::builder()
        .with_hooks(hooks)
        .with_mock_llm()
        .build()
        .await
        .unwrap();

    // 2. Verify: SessionStart hook fired
    assert!(hooks.hook_fired("SessionStart"));

    // 3. Action: User sends message requesting file read
    session.send_input("Please read README.md").await;

    // 4. Verify: UserPromptSubmit hook
    assert!(hooks.hook_fired("UserPromptSubmit"));

    // 5. Mock: LLM responds with Read tool use
    session.inject_llm_tool_use(
        "Read",
        json!({"file_path": "README.md"})
    ).await;

    // 6. Verify: PreToolUse hook
    assert!(hooks.hook_fired("PreToolUse"));

    // 7. Action: Tool executes (real Read tool)
    let tool_result = session.wait_for_tool_result().await;
    assert!(tool_result.is_ok());

    // 8. Verify: PostToolUse hook
    assert!(hooks.hook_fired("PostToolUse"));

    // 9. Mock: LLM final response
    session.inject_llm_response("The README explains...").await;

    // 10. Verify: Response displayed in TUI
    assert!(session.tui_contains("README explains"));

    // 11. Action: User exits
    session.send_input("/exit").await;

    // 12. Verify: Stop + SessionEnd hooks
    assert!(hooks.hook_fired("Stop"));
    assert!(hooks.hook_fired("SessionEnd"));
}
```

**Key Validations:**
- All hooks fire in correct order
- Tools execute with real implementations
- Multi-turn conversation state preserved
- TUI updates reflect conversation flow
- Clean session startup and shutdown

### Test Infrastructure Modules

**Module 1: TestSession**
- File: `crates/cli/tests/helpers/test_session.rs`
- Purpose: Orchestrate full session testing
- Key APIs:
  - `TestSession::new()` - Create test session
  - `send_input()` - Inject user input
  - `inject_llm_response()` - Mock LLM response
  - `tool_was_invoked()` - Check tool execution
  - `tui_contains()` - Verify TUI state

**Module 2: MockLLM**
- File: `crates/cli/tests/mocks/mock_llm.rs`
- Purpose: Controllable LLM behavior for tests
- Key APIs:
  - `MockLLM::new()` - Create mock
  - `add_response()` - Queue response
  - `add_tool_use()` - Queue tool use
  - `simulate_error()` - Test error handling

**Module 3: TestSkillEnvironment**
- File: `crates/cli/tests/helpers/test_skill_env.rs`
- Purpose: Create temporary skill directories for testing
- Key APIs:
  - `with_skill()` - Add skill to test environment
  - `with_frontmatter()` - Configure skill metadata
  - `cleanup()` - Remove test files

---

## Phase 2: Real Terminal E2E Tests (tmux)

### Architecture Overview

```
┌─────────────────────────────────────────────────┐
│           tmux-based E2E Test Suite             │
├─────────────────────────────────────────────────┤
│                                                 │
│  ┌───────────────────────────────────────────┐ │
│  │  Bash Test Scripts                        │ │
│  │  - Launch RustyClawd in tmux session      │ │
│  │  - Send real keystrokes                   │ │
│  │  - Capture terminal output                │ │
│  │  - Validate rendering                     │ │
│  └───────────────────────────────────────────┘ │
│                                                 │
│  ┌───────────────────────────────────────────┐ │
│  │  tmux Test Framework (helpers)            │ │
│  │  - Session management                     │ │
│  │  - Input injection                        │ │
│  │  - Output capture                         │ │
│  │  - Cleanup utilities                      │ │
│  └───────────────────────────────────────────┘ │
│                                                 │
└─────────────────────────────────────────────────┘
         │              │              │
         ▼              ▼              ▼
┌─────────────┐ ┌──────────────┐ ┌──────────┐
│  tmux       │ │  RustyClawd  │ │  Real    │
│  session    │ │  Binary      │ │  Terminal│
└─────────────┘ └──────────────┘ └──────────┘
```

### Why tmux Over Microsoft tui-test

**Decision:** Use tmux as primary real terminal testing framework

**Reasoning:**
1. **Simplicity:** Bash scripts, no additional dependencies
2. **Availability:** tmux standard on Linux/macOS (CI environment)
3. **Control:** Direct control over terminal, no abstraction layer
4. **Debugging:** Easy to run tests manually and observe behavior
5. **Philosophy:** "Start with simplest solution" - tmux is simpler than tui-test

**When to Consider tui-test:**
- If tmux proves insufficient for complex rendering tests
- If cross-platform Windows support becomes critical
- If deterministic timing becomes an issue

### tmux Test Framework

**Core Components:**

```bash
# tests/e2e/tmux/framework.sh

# Session Management
start_rustyclawd_session() {
    local session_name="$1"
    local timeout="${2:-10}"  # seconds

    # Start RustyClawd in detached tmux session
    tmux new-session -d -s "$session_name" "cargo run --bin claude -- --test-mode"

    # Wait for startup with timeout
    for i in $(seq 1 "$timeout"); do
        if tmux capture-pane -t "$session_name" -p | grep -q "Welcome"; then
            return 0
        fi
        sleep 1
    done

    echo "ERROR: RustyClawd failed to start within ${timeout}s"
    return 1
}

# Input Injection
send_command() {
    local session_name="$1"
    local command="$2"
    local wait_time="${3:-1}"  # seconds

    # Send keys to session
    tmux send-keys -t "$session_name" "$command" C-m
    sleep "$wait_time"
}

# Output Capture
capture_output() {
    local session_name="$1"
    tmux capture-pane -t "$session_name" -p
}

# State Validation
verify_output_contains() {
    local session_name="$1"
    local expected="$2"
    local output=$(capture_output "$session_name")

    if echo "$output" | grep -q "$expected"; then
        return 0
    else
        echo "ERROR: Expected '$expected' not found in output:"
        echo "$output"
        return 1
    fi
}

# Cleanup
cleanup_session() {
    local session_name="$1"
    tmux kill-session -t "$session_name" 2>/dev/null || true
}

# Error Handling
trap_cleanup() {
    local session_name="$1"
    trap "cleanup_session $session_name" EXIT INT TERM
}
```

### Example tmux Test: Slash Command E2E

```bash
#!/bin/bash
# tests/e2e/tmux/test_slash_command.sh

set -euo pipefail

# Import framework
source "$(dirname "$0")/framework.sh"

# Test configuration
SESSION="rustyclawd-slash-test-$$"
trap_cleanup "$SESSION"

# Test: Slash command execution in real terminal
test_slash_command_analyze() {
    echo "Test: /analyze command in TUI"

    # 1. Start RustyClawd
    start_rustyclawd_session "$SESSION" || exit 1

    # 2. Verify welcome message
    if ! verify_output_contains "$SESSION" "Welcome"; then
        echo "FAIL: Welcome message not shown"
        exit 1
    fi

    # 3. Send slash command
    send_command "$SESSION" "/analyze src/" 3

    # 4. Verify command processing indicator
    if ! verify_output_contains "$SESSION" "Analyzing"; then
        echo "FAIL: Command not processed"
        exit 1
    fi

    # 5. Wait for completion
    sleep 5

    # 6. Verify results displayed
    if ! verify_output_contains "$SESSION" "codebase"; then
        echo "FAIL: Analysis results not shown"
        exit 1
    fi

    echo "PASS: /analyze command works in TUI"
}

# Run test
test_slash_command_analyze

# Cleanup handled by trap
echo "✅ All tmux slash command tests passed"
```

### Test Suite Structure

```
tests/e2e/tmux/
├── framework.sh                    # Shared helper functions
├── test_slash_commands.sh          # Slash command tests
├── test_skills.sh                  # Skills execution tests
├── test_multi_turn_conversation.sh # Multi-turn workflow
├── test_tool_execution.sh          # Tool use workflow
├── test_error_handling.sh          # Error recovery
└── run_all.sh                      # Test runner
```

### Integration with CI/CD

```yaml
# .github/workflows/e2e-tests.yml
name: E2E Tests

on: [push, pull_request]

jobs:
  tmux-tests:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3

      - name: Install dependencies
        run: |
          sudo apt-get update
          sudo apt-get install -y tmux

      - name: Build RustyClawd
        run: cargo build --release

      - name: Run tmux E2E tests
        run: |
          cd tests/e2e/tmux
          bash run_all.sh

      - name: Upload test artifacts
        if: failure()
        uses: actions/upload-artifact@v3
        with:
          name: tmux-test-logs
          path: tests/e2e/tmux/logs/
```

---

## Phase 3: Agentic Test Scenarios (YAML)

### Architecture Overview

```
┌─────────────────────────────────────────────────┐
│         YAML Scenario-Based Test Suite          │
├─────────────────────────────────────────────────┤
│                                                 │
│  ┌───────────────────────────────────────────┐ │
│  │  YAML Scenario Files                      │ │
│  │  - Declarative test definitions           │ │
│  │  - Reusable workflows                     │ │
│  │  - Human-readable documentation           │ │
│  └───────────────────────────────────────────┘ │
│                  │                              │
│                  ▼                              │
│  ┌───────────────────────────────────────────┐ │
│  │  Scenario Runner (Rust)                   │ │
│  │  - Parse YAML scenarios                   │ │
│  │  - Execute steps via tmux                 │ │
│  │  - Validate assertions                    │ │
│  │  - Report results                         │ │
│  └───────────────────────────────────────────┘ │
│                  │                              │
│                  ▼                              │
│  ┌───────────────────────────────────────────┐ │
│  │  Execution Engine                         │ │
│  │  - tmux session orchestration             │ │
│  │  - Screenshot capture                     │ │
│  │  - Assertion evaluation                   │ │
│  └───────────────────────────────────────────┘ │
│                                                 │
└─────────────────────────────────────────────────┘
```

### YAML Scenario Format

```yaml
# tests/e2e/scenarios/slash-command-workflow.yaml
scenario:
  name: "Slash Command Full Workflow"
  description: "Validate /analyze command from user input to completion"
  type: tui
  tags: [slash-command, analyze, core-workflow]

  environment:
    terminal_size:
      width: 100
      height: 30
    timeout: 30s

  steps:
    - action: launch
      description: "Start RustyClawd TUI"
      target: "cargo run --bin claude"
      timeout: 10s

    - action: wait_for_text
      description: "Wait for welcome message"
      contains: "Welcome"
      timeout: 5s

    - action: send_input
      description: "Type slash command"
      text: "/analyze src/"
      submit: true

    - action: wait_for_text
      description: "Command should be processing"
      contains: "Analyzing"
      timeout: 10s

    - action: wait_for_text
      description: "Should show analysis results"
      contains: "codebase"
      timeout: 20s

    - action: capture_screenshot
      description: "Save final state"
      filename: "slash-command-result.txt"

    - action: send_input
      description: "Exit cleanly"
      text: "/exit"
      submit: true

  assertions:
    - type: text_present
      value: "Welcome"
      description: "TUI shows welcome"

    - type: text_present
      value: "/analyze"
      description: "Command echoed in TUI"

    - type: text_present
      value: "codebase"
      description: "Results displayed"

    - type: exit_clean
      description: "Session exits without errors"
```

### Scenario Runner Architecture

**Core Components:**

```rust
// tests/scenario_runner/src/lib.rs

use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::Command;

/// Root scenario structure
#[derive(Debug, Deserialize)]
pub struct Scenario {
    pub scenario: ScenarioDefinition,
}

#[derive(Debug, Deserialize)]
pub struct ScenarioDefinition {
    pub name: String,
    pub description: String,
    pub r#type: String,
    pub tags: Vec<String>,
    pub environment: Environment,
    pub steps: Vec<Step>,
    pub assertions: Vec<Assertion>,
}

#[derive(Debug, Deserialize)]
pub struct Environment {
    pub terminal_size: TerminalSize,
    pub timeout: String,
}

#[derive(Debug, Deserialize)]
pub struct TerminalSize {
    pub width: u16,
    pub height: u16,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "action")]
pub enum Step {
    #[serde(rename = "launch")]
    Launch {
        description: String,
        target: String,
        timeout: String,
    },
    #[serde(rename = "wait_for_text")]
    WaitForText {
        description: String,
        contains: String,
        timeout: String,
    },
    #[serde(rename = "send_input")]
    SendInput {
        description: String,
        text: String,
        submit: bool,
    },
    #[serde(rename = "capture_screenshot")]
    CaptureScreenshot {
        description: String,
        filename: String,
    },
}

#[derive(Debug, Deserialize)]
pub struct Assertion {
    pub r#type: String,
    pub value: Option<String>,
    pub description: String,
}

/// Scenario executor
pub struct ScenarioRunner {
    tmux_session: String,
    output_dir: PathBuf,
}

impl ScenarioRunner {
    pub fn new(session_name: String, output_dir: PathBuf) -> Self {
        Self {
            tmux_session: session_name,
            output_dir,
        }
    }

    pub fn run_scenario(&mut self, scenario: &Scenario) -> Result<ScenarioResult> {
        println!("Running scenario: {}", scenario.scenario.name);

        let mut results = ScenarioResult::new(&scenario.scenario.name);

        // Execute steps
        for (idx, step) in scenario.scenario.steps.iter().enumerate() {
            println!("  Step {}: {:?}", idx + 1, step);

            match self.execute_step(step) {
                Ok(_) => results.add_step_pass(idx, &format!("{:?}", step)),
                Err(e) => {
                    results.add_step_fail(idx, &format!("{:?}", step), &e.to_string());
                    return Ok(results);  // Early exit on failure
                }
            }
        }

        // Evaluate assertions
        for assertion in &scenario.scenario.assertions {
            match self.evaluate_assertion(assertion) {
                Ok(true) => results.add_assertion_pass(&assertion.description),
                Ok(false) => results.add_assertion_fail(
                    &assertion.description,
                    "Assertion failed"
                ),
                Err(e) => results.add_assertion_fail(
                    &assertion.description,
                    &e.to_string()
                ),
            }
        }

        // Cleanup
        self.cleanup();

        Ok(results)
    }

    fn execute_step(&mut self, step: &Step) -> Result<()> {
        match step {
            Step::Launch { target, timeout, .. } => {
                self.launch_program(target, timeout)
            }
            Step::WaitForText { contains, timeout, .. } => {
                self.wait_for_text(contains, timeout)
            }
            Step::SendInput { text, submit, .. } => {
                self.send_input(text, *submit)
            }
            Step::CaptureScreenshot { filename, .. } => {
                self.capture_screenshot(filename)
            }
        }
    }

    fn launch_program(&self, target: &str, timeout: &str) -> Result<()> {
        // Parse target (e.g., "cargo run --bin claude")
        let parts: Vec<&str> = target.split_whitespace().collect();

        // Start in tmux
        let cmd = format!(
            "tmux new-session -d -s {} '{}'",
            self.tmux_session,
            target
        );

        Command::new("bash")
            .arg("-c")
            .arg(&cmd)
            .output()?;

        // Wait for startup
        std::thread::sleep(parse_duration(timeout)?);

        Ok(())
    }

    fn wait_for_text(&self, text: &str, timeout: &str) -> Result<()> {
        let timeout_duration = parse_duration(timeout)?;
        let start = std::time::Instant::now();

        loop {
            let output = self.capture_tmux_output()?;

            if output.contains(text) {
                return Ok(());
            }

            if start.elapsed() > timeout_duration {
                return Err(anyhow::anyhow!(
                    "Timeout waiting for text: '{}'", text
                ));
            }

            std::thread::sleep(std::time::Duration::from_millis(100));
        }
    }

    fn send_input(&self, text: &str, submit: bool) -> Result<()> {
        let suffix = if submit { " C-m" } else { "" };

        Command::new("tmux")
            .args(&["send-keys", "-t", &self.tmux_session, text])
            .arg(suffix)
            .output()?;

        std::thread::sleep(std::time::Duration::from_millis(500));
        Ok(())
    }

    fn capture_screenshot(&self, filename: &str) -> Result<()> {
        let output = self.capture_tmux_output()?;
        let path = self.output_dir.join(filename);

        std::fs::write(path, output)?;
        Ok(())
    }

    fn capture_tmux_output(&self) -> Result<String> {
        let output = Command::new("tmux")
            .args(&["capture-pane", "-t", &self.tmux_session, "-p"])
            .output()?;

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    fn evaluate_assertion(&self, assertion: &Assertion) -> Result<bool> {
        match assertion.r#type.as_str() {
            "text_present" => {
                let output = self.capture_tmux_output()?;
                Ok(output.contains(assertion.value.as_ref().unwrap()))
            }
            "exit_clean" => {
                // Check if session still exists
                let output = Command::new("tmux")
                    .args(&["has-session", "-t", &self.tmux_session])
                    .output()?;

                // Session should NOT exist (clean exit)
                Ok(!output.status.success())
            }
            _ => Err(anyhow::anyhow!("Unknown assertion type: {}", assertion.r#type)),
        }
    }

    fn cleanup(&self) {
        let _ = Command::new("tmux")
            .args(&["kill-session", "-t", &self.tmux_session])
            .output();
    }
}

#[derive(Debug)]
pub struct ScenarioResult {
    pub name: String,
    pub steps_passed: Vec<String>,
    pub steps_failed: Vec<(String, String)>,
    pub assertions_passed: Vec<String>,
    pub assertions_failed: Vec<(String, String)>,
}

impl ScenarioResult {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            steps_passed: Vec::new(),
            steps_failed: Vec::new(),
            assertions_passed: Vec::new(),
            assertions_failed: Vec::new(),
        }
    }

    pub fn is_success(&self) -> bool {
        self.steps_failed.is_empty() && self.assertions_failed.is_empty()
    }

    pub fn summary(&self) -> String {
        format!(
            "Scenario: {}\n\
             Steps: {}/{} passed\n\
             Assertions: {}/{} passed\n\
             Result: {}",
            self.name,
            self.steps_passed.len(),
            self.steps_passed.len() + self.steps_failed.len(),
            self.assertions_passed.len(),
            self.assertions_passed.len() + self.assertions_failed.len(),
            if self.is_success() { "✅ PASS" } else { "❌ FAIL" }
        )
    }
}
```

### Scenario Categories

**1. Core Workflow Scenarios**
- `slash-command-workflow.yaml` - Basic slash command execution
- `skills-workflow.yaml` - Skills invocation and execution
- `multi-turn-conversation.yaml` - Multi-turn conversation with tools
- `agent-subcommand-workflow.yaml` - Agent subcommand execution

**2. Tool Execution Scenarios**
- `read-tool-workflow.yaml` - Read tool with file operations
- `write-tool-workflow.yaml` - Write tool with file creation
- `bash-tool-workflow.yaml` - Bash tool with command execution
- `edit-tool-workflow.yaml` - Edit tool with file modifications

**3. Error Handling Scenarios**
- `error-recovery-workflow.yaml` - Error handling and recovery
- `invalid-command-workflow.yaml` - Invalid slash command handling
- `missing-skill-workflow.yaml` - Missing skill file handling
- `tool-error-workflow.yaml` - Tool execution error handling

**4. Complex Agentic Scenarios**
- `architect-builder-reviewer.yaml` - Multi-agent workflow
- `investigation-workflow.yaml` - Investigation workflow
- `ddd-workflow.yaml` - Document-driven development workflow
- `consensus-workflow.yaml` - Consensus decision-making

---

## Integration with Existing Infrastructure

### Leveraging Existing Components

**1. TUI Test Harness**
- Extend `TuiTestHarness` for E2E tests
- Add interaction capabilities (keyboard events)
- Integrate with MockLLM for controlled testing

**2. Mock Infrastructure**
- Use existing `MockApiClient` for LLM mocking
- Extend `MockToolExecutor` for tool testing
- Add MockHooksSystem for hook validation

**3. Test Helpers**
- Reuse existing `tests/helpers/` utilities
- Add E2E-specific helpers
- Maintain consistent patterns

### New Test Infrastructure

```
tests/
├── e2e/                          # E2E test directory
│   ├── README.md                 # E2E testing guide
│   │
│   ├── programmatic/             # Phase 1: Rust-based E2E tests
│   │   ├── test_slash_tui.rs     # SlashCommand TUI integration
│   │   ├── test_skills_context.rs # Skills context execution
│   │   └── test_full_session.rs  # Full session E2E
│   │
│   ├── tmux/                     # Phase 2: tmux-based tests
│   │   ├── framework.sh          # Shared helpers
│   │   ├── test_slash_commands.sh
│   │   ├── test_skills.sh
│   │   ├── test_multi_turn.sh
│   │   ├── test_tool_execution.sh
│   │   ├── test_error_handling.sh
│   │   └── run_all.sh            # Test runner
│   │
│   ├── scenarios/                # Phase 3: YAML scenarios
│   │   ├── runner/               # Rust scenario runner
│   │   │   ├── Cargo.toml
│   │   │   └── src/
│   │   │       ├── main.rs       # CLI entry point
│   │   │       ├── lib.rs        # Core runner
│   │   │       ├── parser.rs     # YAML parsing
│   │   │       ├── executor.rs   # Step execution
│   │   │       └── assertions.rs # Assertion evaluation
│   │   │
│   │   ├── core/                 # Core workflow scenarios
│   │   │   ├── slash-command.yaml
│   │   │   ├── skills.yaml
│   │   │   └── multi-turn.yaml
│   │   │
│   │   ├── tools/                # Tool execution scenarios
│   │   │   ├── read-tool.yaml
│   │   │   ├── write-tool.yaml
│   │   │   └── bash-tool.yaml
│   │   │
│   │   ├── errors/               # Error handling scenarios
│   │   │   ├── error-recovery.yaml
│   │   │   └── invalid-command.yaml
│   │   │
│   │   └── agentic/              # Complex agentic scenarios
│   │       ├── architect-workflow.yaml
│   │       └── investigation.yaml
│   │
│   └── helpers/                  # Shared test helpers
│       ├── test_session.rs       # Session orchestration
│       ├── mock_llm.rs           # Enhanced MockLLM
│       ├── test_skill_env.rs     # Test skill environment
│       └── assertions.rs         # Custom assertions
│
├── helpers/                      # Existing shared helpers
│   └── ...                       # (already exists)
│
└── mocks/                        # Existing mocks
    └── ...                       # (already exists)
```

---

## Module Specifications

### Module 1: TestSession

**Purpose:** Orchestrate full interactive session testing

**File:** `tests/e2e/helpers/test_session.rs`

**Public API:**
```rust
pub struct TestSession {
    // Private fields
}

impl TestSession {
    /// Create new test session builder
    pub fn builder() -> TestSessionBuilder;

    /// Send user input to session
    pub async fn send_input(&mut self, input: &str) -> Result<()>;

    /// Inject mock LLM response
    pub async fn inject_llm_response(&mut self, response: &str) -> Result<()>;

    /// Inject mock LLM tool use
    pub async fn inject_llm_tool_use(
        &mut self,
        tool_name: &str,
        params: serde_json::Value
    ) -> Result<()>;

    /// Check if tool was invoked
    pub fn tool_was_invoked(&self, tool_name: &str) -> bool;

    /// Get LLM context messages
    pub fn get_llm_context(&self) -> Vec<String>;

    /// Check if TUI contains text
    pub fn tui_contains(&self, text: &str) -> bool;

    /// Get tool invocation context
    pub fn get_tool_context(&self, tool_name: &str) -> Option<String>;

    /// Add conversation turn
    pub async fn add_conversation_turn(
        &mut self,
        user_msg: &str,
        assistant_msg: &str
    ) -> Result<()>;

    /// Wait for tool result
    pub async fn wait_for_tool_result(&mut self) -> Result<ToolResult>;
}

pub struct TestSessionBuilder {
    // Configuration fields
}

impl TestSessionBuilder {
    pub fn with_mock_llm(self) -> Self;
    pub fn with_real_tui(self) -> Self;
    pub fn with_hooks(self, hooks: HooksSystem) -> Self;
    pub fn with_skill_dir(self, path: PathBuf) -> Self;
    pub async fn build(self) -> Result<TestSession>;
}
```

**Dependencies:**
- `crates/cli/src/interactive.rs` - Interactive session
- `tests/e2e/helpers/mock_llm.rs` - MockLLM
- `tests/tui_test_harness.rs` - TUI testing
- `crates/hooks/` - Hooks system

**Key Design Decisions:**
- Builder pattern for flexible configuration
- Async API for realistic session simulation
- Separate concerns: session orchestration vs TUI rendering vs LLM mocking

---

### Module 2: MockLLM (Enhanced)

**Purpose:** Controllable LLM behavior for deterministic testing

**File:** `tests/e2e/mocks/mock_llm.rs`

**Public API:**
```rust
pub struct MockLLM {
    // Private fields
}

impl MockLLM {
    /// Create new mock LLM client
    pub fn new() -> Self;

    /// Queue text response
    pub fn add_response(&mut self, response: &str);

    /// Queue tool use response
    pub fn add_tool_use(&mut self, tool_name: &str, params: serde_json::Value);

    /// Queue error response
    pub fn add_error(&mut self, error: ApiError);

    /// Set streaming behavior
    pub fn set_streaming(&mut self, enabled: bool);

    /// Get recorded messages
    pub fn get_messages(&self) -> &[Message];

    /// Clear state
    pub fn reset(&mut self);
}

impl ApiClient for MockLLM {
    async fn create_message(
        &self,
        request: CreateMessageRequest
    ) -> Result<Message>;

    async fn create_message_stream(
        &self,
        request: CreateMessageRequest
    ) -> Result<MessageStream>;
}
```

**Dependencies:**
- `crates/api_client/` - ApiClient trait
- `anthropic_sdk::types::Message` - Message types

**Key Design Decisions:**
- Implements real `ApiClient` trait for drop-in replacement
- Queue-based response system for deterministic behavior
- Records all messages for verification

---

### Module 3: TestSkillEnvironment

**Purpose:** Create temporary skill directories for testing

**File:** `tests/e2e/helpers/test_skill_env.rs`

**Public API:**
```rust
pub struct TestSkillEnvironment {
    // Private fields
}

impl TestSkillEnvironment {
    /// Create new test skill environment
    pub fn new() -> Self;

    /// Add skill with content
    pub fn with_skill(self, name: &str, prompt: &str) -> Self;

    /// Add skill with full frontmatter
    pub fn with_skill_full(
        self,
        name: &str,
        frontmatter: SkillFrontmatter,
        content: &str
    ) -> Self;

    /// Get path to skill directory
    pub fn path(&self) -> &Path;

    /// Build and finalize environment
    pub fn build(self) -> TestSkillEnvGuard;
}

pub struct TestSkillEnvGuard {
    // Private fields
}

impl Drop for TestSkillEnvGuard {
    fn drop(&mut self) {
        // Cleanup skill directory
    }
}
```

**Dependencies:**
- `tempfile` - Temporary directories
- `crates/cli/src/skills/` - Skill types

**Key Design Decisions:**
- RAII pattern with guard for automatic cleanup
- Builder pattern for flexible skill creation
- Temporary directories to avoid test pollution

---

### Module 4: tmux Test Framework

**Purpose:** Bash helpers for tmux-based testing

**File:** `tests/e2e/tmux/framework.sh`

**Public API:**
```bash
# Session Management
start_rustyclawd_session <session_name> [timeout]
cleanup_session <session_name>
trap_cleanup <session_name>

# Input Injection
send_command <session_name> <command> [wait_time]
send_keys <session_name> <keys>

# Output Capture
capture_output <session_name>
save_output <session_name> <filename>

# Validation
verify_output_contains <session_name> <expected>
verify_output_matches <session_name> <regex>
wait_for_text <session_name> <text> <timeout>

# Debugging
dump_session_info <session_name>
take_screenshot <session_name> <filename>
```

**Dependencies:**
- `tmux` - Terminal multiplexer
- `grep` - Text matching
- `bash` - Shell scripting

**Key Design Decisions:**
- Pure bash for simplicity
- Consistent error handling
- Timeout support on all wait operations
- Screenshot capability for debugging

---

### Module 5: Scenario Runner

**Purpose:** Parse and execute YAML test scenarios

**File:** `tests/e2e/scenarios/runner/src/lib.rs`

**Public API:**
```rust
pub struct ScenarioRunner {
    // Private fields
}

impl ScenarioRunner {
    /// Create new scenario runner
    pub fn new(session_name: String, output_dir: PathBuf) -> Self;

    /// Load scenario from YAML file
    pub fn load_scenario<P: AsRef<Path>>(path: P) -> Result<Scenario>;

    /// Run single scenario
    pub fn run_scenario(&mut self, scenario: &Scenario) -> Result<ScenarioResult>;

    /// Run all scenarios in directory
    pub fn run_directory<P: AsRef<Path>>(
        dir: P
    ) -> Result<Vec<ScenarioResult>>;
}

pub struct ScenarioResult {
    pub name: String,
    pub steps_passed: Vec<String>,
    pub steps_failed: Vec<(String, String)>,
    pub assertions_passed: Vec<String>,
    pub assertions_failed: Vec<(String, String)>,
}

impl ScenarioResult {
    pub fn is_success(&self) -> bool;
    pub fn summary(&self) -> String;
    pub fn detailed_report(&self) -> String;
}
```

**Dependencies:**
- `serde_yaml` - YAML parsing
- `tmux` (via Command) - Scenario execution
- `anyhow` - Error handling

**Key Design Decisions:**
- Rust for type safety and error handling
- tmux as execution backend
- Structured results for reporting

---

## Risk Assessment and Mitigation

### High-Priority Risks

**Risk 1: tmux Not Available in CI**
- **Impact:** Phase 2 tests cannot run
- **Probability:** Low (standard on Ubuntu runners)
- **Mitigation:**
  - Prerequisite check in CI workflow
  - Document fallback to programmatic tests only
  - Consider Microsoft tui-test as backup
- **Detection:** CI workflow fails with "tmux: command not found"

**Risk 2: Integration Tests Reveal Critical Bugs**
- **Impact:** Additional time required beyond 56 hours
- **Probability:** Medium (E2E tests often find integration issues)
- **Mitigation:**
  - Budget Task 1.4 (4 hours) for immediate bug fixes
  - Document discovered bugs for follow-up issues
  - Prioritize bugs by severity
- **Detection:** Test failures during Phase 1

**Risk 3: Test Framework Complexity Growth**
- **Impact:** Tests become hard to maintain, slow future development
- **Probability:** Medium (test frameworks tend to grow complex)
- **Mitigation:**
  - Apply "Ruthless Simplicity" to test code
  - Regular refactoring of test utilities
  - Clear documentation and examples
  - Code review for test additions
- **Detection:** Test maintenance time increasing

**Risk 4: Flaky Tests Due to Timing Issues**
- **Impact:** CI unreliable, developer frustration
- **Probability:** High (E2E tests inherently timing-dependent)
- **Mitigation:**
  - Generous timeouts in tests (10-30 seconds)
  - Retry logic (2-3 retries)
  - Wait-for-condition patterns (not fixed sleeps)
  - Clear failure messages indicating timing
- **Detection:** Intermittent test failures

**Risk 5: Mock LLM Doesn't Reflect Real Behavior**
- **Impact:** Tests pass but real usage fails
- **Probability:** Low (mocks based on real API)
- **Mitigation:**
  - Document mock limitations clearly
  - Optional integration tests with real API (manual)
  - Regular validation against Claude Code behavior
  - Keep mocks simple and realistic
- **Detection:** User reports issues that tests don't catch

### Medium-Priority Risks

**Risk 6: tmux Tests Difficult to Debug**
- **Impact:** Slower development of tmux tests
- **Probability:** Medium
- **Mitigation:**
  - Screenshot capture on failure
  - Verbose logging mode
  - Manual test mode for step-through debugging
  - Clear error messages with context

**Risk 7: YAML Scenario Runner Complexity**
- **Impact:** Phase 3 takes longer than estimated
- **Probability:** Medium
- **Mitigation:**
  - Start with minimal feature set
  - Reuse tmux framework code
  - Simple YAML schema
  - Clear examples and documentation

**Risk 8: Cross-Platform Compatibility Issues**
- **Impact:** Tests work on Linux but fail on macOS
- **Probability:** Low (tmux similar on both)
- **Mitigation:**
  - Test on both platforms early
  - Document platform-specific quirks
  - Use portable bash constructs
  - CI matrix for Linux and macOS

### Low-Priority Risks

**Risk 9: Test Output Too Verbose**
- **Impact:** Hard to find failures in CI logs
- **Probability:** Low
- **Mitigation:**
  - Clear test result summaries
  - Collapsible CI output sections
  - Separate detailed logs from summaries

**Risk 10: Scenario Library Growth**
- **Impact:** Too many scenarios, slow test suite
- **Probability:** Low (controlled growth)
- **Mitigation:**
  - Categorize scenarios
  - Selective scenario execution
  - Regular scenario review

---

## Success Criteria

### Phase 1 Success (85% Parity)

**Quantitative:**
- 4 new E2E tests passing
- All 45 existing integration tests still passing
- Zero test flakiness (3 consecutive clean runs)

**Qualitative:**
- Manual verification: `/analyze` works in TUI
- Manual verification: Skills use conversation context
- Manual verification: Full session workflow works
- CI pipeline remains green

### Phase 2 Success (95% Parity)

**Quantitative:**
- 7+ tmux E2E tests passing
- tmux framework documented
- Tests run successfully in CI

**Qualitative:**
- Manual verification: Real terminal rendering correct
- Manual verification: Keyboard input handling correct
- Tests catch real rendering bugs
- Debugging workflow clear and documented

### Phase 3 Success (100% Parity)

**Quantitative:**
- YAML scenario framework complete
- 10+ scenarios passing
- Scenario runner documented

**Qualitative:**
- Manual verification: Complex workflows validated
- Scenarios serve as living documentation
- New scenarios easy to add
- TRUE 100% parity achieved

### Overall Success Criteria

**Definition of TRUE 100% Parity:**
A user switching from Claude Code to RustyClawd notices zero functional differences. Every workflow that works in Claude Code works identically in RustyClawd.

**Validation:**
1. All 3 phases complete
2. All tests passing consistently
3. Manual validation confirms parity
4. Documentation complete and accurate
5. CI/CD pipeline stable

---

## Philosophy Compliance Checkpoints

### Ruthless Simplicity
- ✅ Start with tmux (simplest real terminal testing)
- ✅ Bash scripts before TypeScript frameworks
- ✅ Reuse existing test infrastructure
- ✅ Add complexity only when justified

### Zero-BS Implementation
- ✅ Tests validate actual behavior, not stubs
- ✅ No tests that pass without real validation
- ✅ MockLLM based on real API behavior
- ✅ Real tools, real hooks, real TUI in tests

### Modular Design (Bricks & Studs)
- ✅ Each test framework is independent brick
- ✅ Clear contracts between test layers
- ✅ Components regeneratable from specs
- ✅ Test utilities in dedicated modules

### Quality Over Speed
- ✅ Full 56-hour budget allocated
- ✅ Bug fixing time included
- ✅ Test maintainability prioritized
- ✅ Documentation required for all phases

---

## Next Steps

**After Architecture Approval:**

1. **Builder Agent:** Implement Phase 1 (Critical E2E Tests)
2. **Builder Agent:** Implement Phase 2 (tmux Tests)
3. **Builder Agent:** Implement Phase 3 (YAML Scenarios)
4. **Reviewer Agent:** Validate philosophy compliance
5. **Tester Agent:** Execute all test suites
6. **Documentation:** Complete testing guides

**Implementation Sequence:**
1. Module specs → Builder implements → Tests pass
2. Each phase validated before moving to next
3. Bug fixes incorporated immediately
4. Continuous integration validation

---

## Conclusion

This architecture provides a comprehensive, philosophy-aligned path to achieving TRUE 100% Claude Code parity. By implementing three complementary test phases (programmatic, tmux, YAML scenarios), we systematically close all identified gaps and establish confidence that RustyClawd works exactly like Claude Code in practice.

The design prioritizes simplicity, maintainability, and real-world validation while remaining flexible enough to adapt as we discover new requirements during implementation.

**Timeline:** 56 hours (7 days) from architecture approval to 100% parity.

**Confidence Level:** High - Clear architecture, proven patterns, manageable risks.

**Ready for Implementation:** YES

---

**References:**
- Requirements: `/home/azureuser/src/RustyClawd/.claude/runtime/requirements/e2e_testing_requirements.md`
- Master Prompt: `/tmp/e2e_testing_master_prompt.md`
- Philosophy: `.claude/context/PHILOSOPHY.md`
- Patterns: `.claude/context/PATTERNS.md`
