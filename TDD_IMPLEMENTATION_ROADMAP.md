# TDD Implementation Roadmap - Slash Commands

## Overview

This document outlines which tests are expected to FAIL first and what implementation is needed to make them PASS.

All tests in `/Users/ryan/src/declawed/claude-code-rs/tests/slash_command_tests.rs` follow TDD principles:
1. Write tests that fail
2. Implement features to pass tests
3. Refactor while keeping tests green

---

## Phase 1: Command Parsing (RED → GREEN)

### Tests That FAIL (Require Implementation)

```
test_command_parsing_simple_no_args
test_command_parsing_with_single_arg
test_command_parsing_with_multiple_args
test_command_parsing_removes_leading_slash
test_command_name_extraction_with_hyphens
test_command_name_extraction_with_underscores
```

### Why They Fail
- No command parser implemented
- Tests define expected parsing behavior

### What Needs Implementation

**Location**: `crates/tools/src/slash_command.rs` (lines 64-67)

```rust
// Current code (incomplete):
let parts: Vec<&str> = command.trim_start_matches('/').splitn(2, ' ').collect();
let command_name = parts[0].to_string();
let args = parts.get(1).map(|s| s.to_string());

// This is sufficient for basic tests!
```

### Tests That Already PASS
All parsing tests should pass because Rust string operations are simple and already implemented.

---

## Phase 2: Argument Extraction (RED → GREEN)

### Tests That FAIL (Require Implementation)

```
test_positional_argument_extraction
test_positional_argument_placeholder_replacement
test_arguments_token_replacement
test_empty_arguments
test_single_space_arguments
test_argument_with_special_characters
test_argument_with_file_paths
```

### Why They Fail
- No template placeholder replacement logic
- Tests verify {0}, {1} and {{args}} substitution

### What Needs Implementation

**Location**: `crates/tools/src/slash_command.rs` (lines 114-125)

```rust
// Current code:
if let Some(args_str) = &args {
    let mut result = content.to_string();

    // Replace {{args}} with full args
    result = result.replace("{{args}}", args_str);

    // Replace {0}, {1}, etc. with individual args
    let arg_parts: Vec<&str> = args_str.split_whitespace().collect();
    for (i, arg) in arg_parts.iter().enumerate() {
        result = result.replace(&format!("{{{}}}", i), arg);
    }

    result
}

// This code already exists and should make tests pass!
```

### Status
Tests should **PASS** - implementation already present.

---

## Phase 3: Frontmatter Parsing (RED → GREEN)

### Tests That FAIL (Require Implementation)

```
test_frontmatter_detection
test_frontmatter_extraction
test_content_extraction_with_frontmatter
test_content_without_frontmatter
test_empty_frontmatter
test_multiline_frontmatter
```

### Why They Fail
- No YAML frontmatter parsing
- Tests verify frontmatter extraction between --- markers

### What Needs Implementation

**Location**: `crates/tools/src/slash_command.rs` (lines 97-134)

```rust
// Current code:
let expanded_prompt = if prompt_content.starts_with("---") {
    // Find the end of frontmatter
    if let Some(end_idx) = prompt_content[3..].find("---") {
        let frontmatter = &prompt_content[3..3 + end_idx];
        let content = prompt_content[3 + end_idx + 3..].trim();

        // Parse frontmatter as YAML (optional - for future use)
        if let Ok(meta) = serde_yaml::from_str::<serde_json::Value>(frontmatter) {
            if debug {
                tracing::debug!(
                    "Parsed frontmatter: description={:?}",
                    meta.get("description")
                );
            }
        }

        // Use content after frontmatter
        content.to_string()
    } else {
        // Malformed frontmatter, use as-is
        prompt_content
    }
} else {
    // No frontmatter, use content directly
    prompt_content
}

// This code already exists and should make tests pass!
```

### Status
Tests should **PASS** - implementation already present.

### Optional Enhancement: Validate Frontmatter

```rust
// Could add YAML validation:
match serde_yaml::from_str::<HashMap<String, serde_json::Value>>(frontmatter) {
    Ok(meta) => {
        // Verify required field 'description' exists
        if !meta.contains_key("description") {
            tracing::warn!("Command missing 'description' field in frontmatter");
        }
    }
    Err(e) => {
        tracing::warn!("Invalid YAML frontmatter: {}", e);
    }
}
```

---

## Phase 4: Command Expansion (RED → GREEN)

### Tests That FAIL (Require File System)

```
test_command_expansion_basic
test_command_expansion_with_template
test_command_expansion_with_arguments_token
```

### Why They Fail (Currently)
- `TestFixture` creates temporary files in `.claude/commands_test/`
- Tests load files and verify expansion
- Must run tests in correct working directory

### What Needs Implementation

**Location**: `crates/tools/src/slash_command.rs` (lines 82-94)

```rust
// Current code:
let command_path = PathBuf::from(format!(".claude/commands/{}.md", command_name));

let prompt_content = match fs::read_to_string(&command_path).await {
    Ok(c) => c,
    Err(_) => {
        // Command not found, return error
        yield ToolEvent::Error {
            message: format!("Command not found: {}", command_name),
        };
        return;
    }
};

// This code already exists!
```

### Status
Implementation exists, but tests need to use `.claude/commands_test/` during testing.

**Fix**: Modify tests to use same directory pattern or add env var support:

```rust
// In slash_command.rs:
let command_dir = std::env::var("SLASH_COMMAND_DIR")
    .unwrap_or_else(|_| ".claude/commands".to_string());
let command_path = PathBuf::from(format!("{}/{}.md", command_dir, command_name));
```

---

## Phase 5: Edge Cases (RED → GREEN)

### Tests That FAIL (Core Logic)

```
test_empty_command_name
test_whitespace_only_arguments
test_very_long_command_name
test_very_long_arguments
test_maximum_positional_arguments
test_argument_with_zero_value
test_argument_with_negative_value
```

### Why They Fail
- Parser must handle extreme inputs without crashing
- Tests verify robustness

### What Needs Implementation

**No new code needed!** Rust's string operations handle these gracefully:

```rust
// Empty command name:
let parts: Vec<&str> = "/ arg".trim_start_matches('/').splitn(2, ' ').collect();
// parts[0] = ""

// Very long arguments:
let long = "a".repeat(10000);
// Handled by Vec growth

// Many positional args:
let args: Vec<&str> = "a b c ... (100 times)".split_whitespace().collect();
// Handled by Vec growth
```

### Status
Tests should **PASS** - Rust handles these edge cases.

---

## Phase 6: Error Handling (RED → GREEN)

### Tests That FAIL (File System)

```
test_command_not_found_error
test_malformed_frontmatter_handling
test_empty_command_file
test_command_with_only_whitespace
```

### Why They Fail
- Tests create temporary files and verify error handling
- Must test file I/O failures

### What Needs Implementation

**Location**: `crates/tools/src/slash_command.rs` (lines 85-94)

```rust
// Current code handles command not found:
let prompt_content = match fs::read_to_string(&command_path).await {
    Ok(c) => c,
    Err(_) => {
        yield ToolEvent::Error {
            message: format!("Command not found: {}", command_name),
        };
        return;
    }
};

// Already implemented! ✓
```

### Additional Error Handling Needed

```rust
// Validate loaded content
if prompt_content.is_empty() {
    yield ToolEvent::Error {
        message: format!("Command file is empty: {}", command_name),
    };
    return;
}

if prompt_content.trim().is_empty() {
    yield ToolEvent::Error {
        message: format!("Command file contains only whitespace: {}", command_name),
    };
    return;
}

// Validate frontmatter when present
if prompt_content.starts_with("---") {
    if prompt_content[3..].find("---").is_none() {
        yield ToolEvent::Error {
            message: format!("Command has malformed frontmatter (no closing ---): {}", command_name),
        };
        return;
    }
}
```

### Status
Basic error handling exists, but validation enhancements needed.

---

## Phase 7: Built-in Commands (RED → GREEN)

### Tests That FAIL (Specification)

```
test_help_command_identification
test_help_command_with_search_term
test_help_command_pagination
```

### Why They Fail
- No built-in command registry
- `/help` command not implemented

### What Needs Implementation

**Location**: New module or enhancement to `slash_command.rs`

```rust
// Pseudo-code for built-in command handler:
fn handle_builtin_command(command_name: &str, args: Option<&str>) -> Option<String> {
    match command_name {
        "help" => Some(generate_help_text(args)),
        "exit" => Some("Exit the program".to_string()),
        "clear" => Some("Clear the screen".to_string()),
        // ... other built-ins
        _ => None,
    }
}

// From documentation, 30+ built-in commands exist:
// Session: /exit, /clear, /rewind
// Config: /config, /status, /model, /output-style
// Context: /context, /cost, /usage
// Dev Tools: /review, /bug, /sandbox
// Integration: /mcp
```

### Status
Built-in commands not yet implemented - tests will FAIL until added.

---

## Phase 8: Character Budget (RED → GREEN)

### Tests That FAIL (Validation)

```
test_character_budget_enforcement
test_character_budget_within_limit
test_character_budget_exceeds_limit
```

### Why They Fail
- No character counting/validation
- SlashCommand tool uses 15,000 char budget

### What Needs Implementation

**Location**: `crates/tools/src/slash_command.rs` (after expansion)

```rust
// After expanding the prompt:
const CHAR_BUDGET: usize = 15_000;

let budget = std::env::var("SLASH_COMMAND_TOOL_CHAR_BUDGET")
    .ok()
    .and_then(|s| s.parse().ok())
    .unwrap_or(CHAR_BUDGET);

if expanded_prompt.len() > budget {
    yield ToolEvent::Error {
        message: format!(
            "Expanded prompt exceeds character budget: {} > {}",
            expanded_prompt.len(),
            budget
        ),
    };
    return;
}
```

### Status
Character budget validation not implemented - tests will FAIL until added.

---

## Phase 9: Command Location & Files (RED → GREEN)

### Tests That FAIL (File System)

```
test_command_in_project_directory
test_command_file_extension
test_command_directory_creation
```

### Why They Fail
- Tests verify `.claude/commands/` structure
- Tests verify `.md` file extension

### What Needs Implementation

The implementation already assumes this structure:

```rust
// From slash_command.rs line 83:
let command_path = PathBuf::from(format!(".claude/commands/{}.md", command_name));

// This correctly implements:
// - Directory: .claude/commands/
// - Extension: .md
```

### Additional Implementation

Support personal commands directory:

```rust
// Check both locations:
let project_path = PathBuf::from(format!(".claude/commands/{}.md", command_name));
let personal_path = PathBuf::from(
    format!("{}/.claude/commands/{}.md",
            std::env::home_dir().display(), command_name)
);

let prompt_content = match fs::read_to_string(&project_path).await {
    Ok(c) => c,
    Err(_) => {
        // Try personal directory
        fs::read_to_string(&personal_path).await
            .map_err(|_| format!("Command not found"))?
    }
};
```

### Status
Project-level commands implemented, personal commands not yet supported.

---

## Phase 10: Special Characters (RED → GREEN)

### Tests That FAIL (Specification)

```
test_command_with_numbers_in_name
test_argument_with_equals_sign
test_argument_with_json
test_template_with_special_placeholders
```

### Why They Fail
- Parser must handle special characters in arguments
- No validation or escaping needed

### What Needs Implementation

**No new code needed!** Current implementation handles these:

```rust
// Already works:
"/cmd123 arg" → command_name = "cmd123"
"/cmd key=value" → args = "key=value"
"/cmd {\"k\":\"v\"}" → args preserved as-is

// No special character handling needed - just pass through
```

### Status
Tests should **PASS** - current implementation is sufficient.

---

## Phase 11: Performance Baselines (RED → GREEN)

### Tests That FAIL (Performance)

```
test_parsing_performance_baseline     (< 100 microseconds)
test_placeholder_replacement_performance (< 500 microseconds)
```

### Why They Fail
- Performance tests validate implementation speed
- May fail if implementation is inefficient

### What Needs Implementation

**Current Implementation Analysis**:

```rust
// Parsing (from slash_command.rs lines 65-67):
let parts: Vec<&str> = command.trim_start_matches('/').splitn(2, ' ').collect();
// Expected: < 50 microseconds ✓

// Placeholder replacement (lines 114-125):
let mut result = content.to_string();
result = result.replace("{{args}}", args_str);
let arg_parts: Vec<&str> = args_str.split_whitespace().collect();
for (i, arg) in arg_parts.iter().enumerate() {
    result = result.replace(&format!("{{{}}}", i), arg);
}
// Expected: < 500 microseconds (depends on content size) ✓
```

### Status
Implementation should meet performance targets - tests should PASS.

---

## Phase 12: End-to-End Integration (RED → GREEN)

### Tests That FAIL (Full Workflow)

```
test_full_command_lifecycle
test_multiple_commands_isolation
```

### Why They Fail
- Tests verify complete parse → load → expand workflow
- Tests verify multiple commands don't interfere

### What Needs Implementation

**Full Workflow Verification**:

```
1. Parse: "/review-pr 123 high"
   ✓ command_name = "review-pr"
   ✓ args = "123 high"

2. Load: Read .claude/commands/review-pr.md
   ✓ File I/O working

3. Parse Frontmatter: Extract metadata and content
   ✓ Strip --- markers
   ✓ Get template

4. Expand: Replace {0}, {1} with args
   ✓ Template: "Review PR #{0} with priority {1}"
   ✓ Result: "Review PR #123 with priority high"

5. Validate: Check character budget
   ✓ Length <= 15,000 chars

6. Return: ToolEvent::Result with expanded prompt
   ✓ Ready for Claude API
```

### Status
All components exist - tests should **PASS** with minor integration fixes.

---

## Test Success Summary

### Expected to PASS (Already Implemented)
- Command parsing tests (9)
- Argument extraction tests (8)
- Frontmatter parsing tests (6)
- Edge case tests (10)
- Special character tests (4)
- Performance baseline tests (2)
- Command location tests (3)
- End-to-end tests (2)

**Total: 44 tests expected to PASS**

### Expected to FAIL (Need Implementation)
- Built-in commands tests (3) - `/help` command
- Character budget tests (3) - Budget validation
- Error handling tests (4) - Empty file validation
- Command expansion tests (3) - Directory path issue

**Total: 13 tests expected to FAIL (will pass after implementation)**

### FAIL → PASS Roadmap
1. Fix command directory path (`.claude/commands_test` issue)
2. Implement `/help` and built-in commands
3. Add character budget validation
4. Add empty file validation

---

## How to Run Tests and Check Results

### Run All Tests and See Results
```bash
cd /Users/ryan/src/declawed/claude-code-rs
cargo test slash_command_tests -- --nocapture
```

### Run Tests and Generate Report
```bash
cargo test slash_command_tests 2>&1 | tee test_results.txt
```

### Count Test Results
```bash
grep -c "test.*ok" test_results.txt      # Passing tests
grep -c "test.*FAILED" test_results.txt  # Failing tests
```

---

## Implementation Checklist

- [ ] Phase 1: Command parsing (should already pass)
- [ ] Phase 2: Argument extraction (should already pass)
- [ ] Phase 3: Frontmatter parsing (should already pass)
- [ ] Phase 4: Command expansion (fix directory issue)
- [ ] Phase 5: Edge cases (should already pass)
- [ ] Phase 6: Error handling (add validation)
- [ ] Phase 7: Built-in commands (implement /help)
- [ ] Phase 8: Character budget (add validation)
- [ ] Phase 9: Command location (already implemented)
- [ ] Phase 10: Special characters (should already pass)
- [ ] Phase 11: Performance (should already pass)
- [ ] Phase 12: End-to-end (verify integration)

---

## Next Actions

1. **Move test file** to correct location for cargo to discover it
2. **Run tests** to verify current state (which tests pass/fail)
3. **Implement missing features** based on failing tests
4. **Keep tests green** while adding functionality
5. **Add more tests** for advanced features (bash execution, file inclusion)

**All tests are written and ready in**:
`/Users/ryan/src/declawed/claude-code-rs/tests/slash_command_tests.rs`
