# Phase 3: YAML Scenario Tests

**Status:** FAILING (Expected) - Waiting for scenario runner implementation

This directory contains declarative YAML-based test scenarios that describe complete user workflows. Scenarios are human-readable, reusable, and serve as living documentation.

## Why YAML Scenarios?

YAML scenarios provide:
- **Declarative:** Describe WHAT, not HOW
- **Readable:** Non-developers can understand and write tests
- **Reusable:** Same scenario, multiple test runs
- **Documentation:** Scenarios document expected behavior
- **Maintainable:** Easy to update when workflows change

## Scenario Files

### Core Workflows

#### `multi_turn_conversation.yaml`
Multi-turn conversation with context preservation and tool use:
- Three-turn conversation
- Read tool execution
- Context preserved across turns
- Write tool with context
- Verification

**Tags:** `conversation`, `context`, `core-workflow`, `tools`

#### `slash_command_workflow.yaml`
Complete slash command workflow:
- `/analyze` command execution
- Command expansion
- Result display
- Clean exit

**Tags:** `slash-command`, `analyze`, `core-workflow`

#### `skills_integration.yaml`
Skills with conversation context:
- Skill file creation
- Context establishment
- Skill invocation
- Context usage verification

**Tags:** `skills`, `context`, `integration`

### Error Handling

#### `error_handling.yaml`
Error handling and recovery:
- Invalid slash command → error → recovery
- Tool failure → error → recovery
- System stability after errors

**Tags:** `error-handling`, `recovery`, `robustness`

### Agentic Workflows

#### `agentic_task.yaml`
Complex multi-step agentic task:
- Multi-file analysis
- Context-based reasoning
- Document creation
- Verification
- Full workflow completion

**Tags:** `agentic`, `multi-step`, `complex`, `tools`

## Scenario Structure

```yaml
scenario:
  name: "Human-readable name"
  description: "What this scenario tests"
  type: tui
  tags: [tag1, tag2]
  status: "pending_implementation"

  environment:
    terminal_size: { width: 120, height: 40 }
    timeout: 60s

  steps:
    - action: launch
      description: "What this step does"
      target: "cargo run --bin rustyclawd"
      timeout: 10s

    - action: wait_for_text
      description: "What we're waiting for"
      contains: "expected text"
      timeout: 5s

    - action: send_input
      description: "User action"
      text: "input text"
      submit: true

  assertions:
    - type: text_present
      value: "expected output"
      description: "Why this matters"

  notes: |
    Additional context about the scenario
```

## Running Scenarios

**Current Status:** Scenario runner not implemented. All scenarios will fail.

**Once implemented:**
```bash
# Run single scenario
cargo run --bin scenario_runner run --file multi_turn_conversation.yaml

# Run all scenarios
cargo run --bin scenario_runner run --dir tests/e2e/scenarios/

# Run scenarios with specific tag
cargo run --bin scenario_runner run --dir tests/e2e/scenarios/ --tag core-workflow

# Verbose output
cargo run --bin scenario_runner run --file multi_turn_conversation.yaml --verbose
```

## Scenario Actions

**Available Actions:**
- `launch` - Start program
- `wait_for_text` - Wait for specific text
- `send_input` - Send user input
- `capture_screenshot` - Save terminal state
- `sleep` - Wait (use sparingly)
- `ensure_file` - Create/verify file exists
- `remove_file` - Clean up file

**Available Assertions:**
- `text_present` - Text should appear
- `text_not_present` - Text should NOT appear
- `exit_clean` - Clean exit
- `file_exists` - File created

## Implementation Order

1. **Create scenario runner crate** (Task 3.1)
   - `tests/e2e/scenarios/runner/` Cargo project
   - YAML parser (`parser.rs`)
   - Step executor (`executor.rs`)
   - Assertion evaluator (`assertions.rs`)
   - CLI entry point (`main.rs`)

2. **Verify scenarios parse** (Task 3.2)
   - All YAML scenarios parse successfully
   - Schema validation works

3. **Execute scenarios** (Tasks 3.2-3.3)
   - Scenarios run via tmux
   - All assertions evaluate correctly
   - Reports generated

4. **CI Integration**
   - Add to GitHub Actions
   - Artifact collection on failure

## Success Criteria

Phase 3 succeeds when:
- [ ] Scenario runner implemented
- [ ] All 5 scenarios parse successfully
- [ ] All 5 scenarios execute successfully
- [ ] All assertions pass
- [ ] Scenarios run in CI
- [ ] TRUE 100% parity achieved

## Writing New Scenarios

1. **Create YAML file**
   - Use existing scenarios as templates
   - Follow structure guidelines

2. **Test locally**
   ```bash
   cargo run --bin scenario_runner run --file your_scenario.yaml
   ```

3. **Add tags**
   - Categorize for easy filtering
   - Use existing tag conventions

4. **Document**
   - Clear descriptions
   - Explain assertions

## Scenario Categories

**By Type:**
- Core workflows (`core/`)
- Tool execution (`tools/`)
- Error handling (`errors/`)
- Agentic workflows (`agentic/`)

**By Complexity:**
- Simple (1-3 steps)
- Medium (4-8 steps)
- Complex (9+ steps, multiple tools)

## Documentation

- **Architecture:** `docs/architecture/e2e_testing_architecture.md`
- **Development Guide:** `docs/testing/E2E_TEST_DEVELOPMENT.md`
- **Scenario Examples:** All `.yaml` files in this directory

## Next Steps

After Phase 3:
- Final validation of TRUE 100% parity
- Documentation completion
- Comprehensive testing report

**Target:** TRUE 100% Claude Code parity after Phase 3
