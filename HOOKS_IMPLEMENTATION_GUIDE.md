# Hooks System - Implementation Guide Based on Tests

This document shows how to use the test suite as a specification for implementing the hooks system.

---

## Part 1: Core Data Structures

Based on tests, implement these structures:

### Hook Type
```rust
enum HookType {
    Command(String),  // Command string to execute
    Prompt,          // LLM-based hook
}

struct Hook {
    hook_type: HookType,
    timeout_ms: u32,  // Default 60000 (60 seconds)
}
```

**Test Reference**: `test_hook_creation_command_type`, `test_hook_creation_prompt_type`

### Hook Matcher
```rust
enum HookMatcher {
    Exact(String),  // Exact tool name: "Write"
    Regex(String),  // Pattern: "Edit|Write" or "mcp__.*"
}

impl HookMatcher {
    fn matches(&self, tool_name: &str) -> bool {
        match self {
            HookMatcher::Exact(pattern) => tool_name == pattern,
            HookMatcher::Regex(pattern) => {
                // Use regex crate to match
                todo!()
            }
        }
    }
}
```

**Test Reference**: `test_matcher_exact_string`, `test_matcher_regex_pattern`

### Hook Configuration
```rust
struct HookConfig {
    matcher: HookMatcher,
    hooks: Vec<Hook>,
}

struct HooksConfiguration {
    session_start: Vec<HookConfig>,
    session_end: Vec<HookConfig>,
    pre_tool_use: Vec<HookConfig>,
    post_tool_use: Vec<HookConfig>,
    user_prompt_submit: Vec<HookConfig>,
    stop: Vec<HookConfig>,
    subagent_stop: Vec<HookConfig>,
    notification: Vec<HookConfig>,
    pre_compact: Vec<HookConfig>,
}
```

**Test Reference**: `test_hooks_configuration_all_events`

### Hook Execution Context
```rust
struct HookContext {
    session_id: String,
    transcript_path: String,
    cwd: String,
    permission_mode: String,
    hook_event_name: String,
}
```

**Test Reference**: `test_session_start_hook_event`

### Hook Output
```rust
struct HookOutput {
    continue_execution: Option<bool>,
    permission_decision: Option<PermissionDecision>,
    decision: Option<Decision>,
    additional_context: Option<String>,
}

enum PermissionDecision {
    Allow,   // Tool execution permitted
    Deny,    // Tool execution blocked
    Ask,     // User prompted
}

enum Decision {
    Approve, // Work complete
    Block,   // Continue working
}
```

**Test Reference**: `test_hook_output_permission_allow`, `test_hook_output_decision_approve`

---

## Part 2: Lifecycle Event System

Implement handlers for all 9 lifecycle events:

### 1. SessionStart Hook

**When**: Session begins
**Purpose**: Initialize environment

```rust
async fn trigger_session_start_hooks(
    config: &HooksConfiguration,
    context: &HookContext,
) -> Result<()> {
    for hook_config in &config.session_start {
        if should_trigger(&hook_config.matcher, "*") {
            execute_hooks(&hook_config.hooks, context).await?;
        }
    }
    Ok(())
}
```

**Test Reference**: `test_scenario_session_workflow`
**Real-World Pattern**: `test_scenario_environment_persistence`

Example: Load environment variables
```bash
source $CLAUDE_ENV_FILE
export CUSTOM_VAR=value
```

### 2. PreToolUse Hook

**When**: Before tool execution
**Purpose**: Validate/permit tool use

```rust
async fn trigger_pre_tool_use_hooks(
    config: &HooksConfiguration,
    context: &HookContext,
    tool_name: &str,
) -> Result<PermissionDecision> {
    for hook_config in &config.pre_tool_use {
        if hook_config.matcher.matches(tool_name) {
            let output = execute_hooks(&hook_config.hooks, context).await?;
            if let Some(decision) = output.permission_decision {
                return Ok(decision);
            }
        }
    }
    Ok(PermissionDecision::Allow) // Default allow
}
```

**Test Reference**: `test_scenario_permission_enforcement`

Examples:
- Bash tool: Require LLM approval
- Write tool: Check file permissions
- Edit tool: Validate target exists

### 3. PostToolUse Hook

**When**: After tool completes
**Purpose**: Analyze results

```rust
async fn trigger_post_tool_use_hooks(
    config: &HooksConfiguration,
    context: &HookContext,
    tool_name: &str,
    result: &ToolResult,
) -> Result<()> {
    for hook_config in &config.post_tool_use {
        if hook_config.matcher.matches(tool_name) {
            execute_hooks(&hook_config.hooks, context).await?;
        }
    }
    Ok(())
}
```

**Test Reference**: `test_scenario_post_execution_analysis`

Examples:
- Log command output
- Analyze errors
- Trigger follow-up actions

### 4. Stop Hook

**When**: Agent requests completion
**Purpose**: Decide if work is complete

```rust
async fn trigger_stop_hooks(
    config: &HooksConfiguration,
    context: &HookContext,
) -> Result<Decision> {
    for hook_config in &config.stop {
        if should_trigger(&hook_config.matcher, "*") {
            let output = execute_hooks(&hook_config.hooks, context).await?;
            if let Some(decision) = output.decision {
                return Ok(decision);
            }
        }
    }
    Ok(Decision::Approve) // Default approve
}
```

**Test Reference**: `test_scenario_completion_decision`

### 5. SessionEnd Hook

**When**: Session terminates
**Purpose**: Cleanup and logging

```rust
async fn trigger_session_end_hooks(
    config: &HooksConfiguration,
    context: &HookContext,
) -> Result<()> {
    for hook_config in &config.session_end {
        if should_trigger(&hook_config.matcher, "*") {
            execute_hooks(&hook_config.hooks, context).await?;
        }
    }
    Ok(())
}
```

**Test Reference**: `test_session_end_hook_event`

### 6. UserPromptSubmit Hook

**When**: Before processing user input
**Purpose**: Validate and enrich prompt

```rust
async fn trigger_user_prompt_submit_hooks(
    config: &HooksConfiguration,
    context: &HookContext,
) -> Result<Option<String>> {
    for hook_config in &config.user_prompt_submit {
        if should_trigger(&hook_config.matcher, "*") {
            let output = execute_hooks(&hook_config.hooks, context).await?;
            if let Some(additional_context) = output.additional_context {
                return Ok(Some(additional_context));
            }
        }
    }
    Ok(None)
}
```

**Test Reference**: `test_user_prompt_submit_hook_event`

### 7. SubagentStop Hook

**When**: Subagent requests stop
**Purpose**: Control subagent termination

```rust
async fn trigger_subagent_stop_hooks(
    config: &HooksConfiguration,
    context: &HookContext,
) -> Result<Decision> {
    for hook_config in &config.subagent_stop {
        if should_trigger(&hook_config.matcher, "*") {
            let output = execute_hooks(&hook_config.hooks, context).await?;
            if let Some(decision) = output.decision {
                return Ok(decision);
            }
        }
    }
    Ok(Decision::Approve)
}
```

**Test Reference**: `test_subagent_stop_hook_event`

### 8. Notification Hook

**When**: Alert or notification occurs
**Purpose**: Route or filter notifications

```rust
async fn trigger_notification_hooks(
    config: &HooksConfiguration,
    context: &HookContext,
) -> Result<bool> {
    for hook_config in &config.notification {
        if hook_config.matcher.matches("*") {
            let output = execute_hooks(&hook_config.hooks, context).await?;
            if let Some(continue_execution) = output.continue_execution {
                if !continue_execution {
                    return Ok(false);
                }
            }
        }
    }
    Ok(true)
}
```

**Test Reference**: `test_notification_hook_event`

### 9. PreCompact Hook

**When**: Before context compaction
**Purpose**: Prepare for compaction

```rust
async fn trigger_pre_compact_hooks(
    config: &HooksConfiguration,
    context: &HookContext,
) -> Result<()> {
    for hook_config in &config.pre_compact {
        if should_trigger(&hook_config.matcher, "*") {
            execute_hooks(&hook_config.hooks, context).await?;
        }
    }
    Ok(())
}
```

**Test Reference**: `test_pre_compact_hook_event`

---

## Part 3: Hook Execution Engine

### Execute Command Hook

```rust
async fn execute_command_hook(
    command: &str,
    context: &HookContext,
    timeout_ms: u32,
) -> Result<HookOutput> {
    let timeout = Duration::from_millis(timeout_ms as u64);

    let output = tokio::time::timeout(
        timeout,
        tokio::process::Command::new("sh")
            .arg("-c")
            .arg(command)
            .env("SESSION_ID", &context.session_id)
            .env("TRANSCRIPT_PATH", &context.transcript_path)
            .env("CWD", &context.cwd)
            .env("HOOK_EVENT_NAME", &context.hook_event_name)
            .output()
    ).await??;

    // Parse exit code
    match output.status.code() {
        Some(0) => {
            // Success - parse stdout as JSON if possible
            let stdout = String::from_utf8_lossy(&output.stdout);
            parse_hook_output(&stdout)
        }
        Some(1) => {
            // Non-blocking error
            let stderr = String::from_utf8_lossy(&output.stderr);
            eprintln!("Hook warning: {}", stderr);
            Ok(HookOutput::default())
        }
        Some(2) => {
            // Blocking error
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(HookError::BlockingError(stderr.to_string()).into())
        }
        _ => Err("Hook execution failed".into()),
    }
}
```

**Test Reference**: `test_hook_result_success`, `test_hook_result_blocking_error`

### Execute Prompt Hook

```rust
async fn execute_prompt_hook(
    context: &HookContext,
    timeout_ms: u32,
) -> Result<HookOutput> {
    // Send context to Claude Haiku
    let prompt = format!(
        "Hook event: {}\nSession: {}\nCWD: {}\n\nMake a decision.",
        context.hook_event_name,
        context.session_id,
        context.cwd
    );

    let timeout = Duration::from_millis(timeout_ms as u64);

    let response = tokio::time::timeout(
        timeout,
        call_claude_haiku(&prompt)
    ).await??;

    // Parse response as JSON
    parse_hook_output(&response)
}
```

**Test Reference**: `test_hook_creation_prompt_type`

### Parallel Execution & Deduplication

```rust
async fn execute_hooks(
    hooks: &[Hook],
    context: &HookContext,
) -> Result<HookOutput> {
    // Deduplicate identical commands
    let mut unique_hooks = Vec::new();
    let mut seen_commands = HashSet::new();

    for hook in hooks {
        if let HookType::Command(cmd) = &hook.hook_type {
            if !seen_commands.contains(cmd) {
                seen_commands.insert(cmd.clone());
                unique_hooks.push(hook);
            }
        } else {
            unique_hooks.push(hook);
        }
    }

    // Execute all hooks in parallel
    let futures: Vec<_> = unique_hooks
        .iter()
        .map(|hook| execute_single_hook(hook, context))
        .collect();

    let results = futures::future::join_all(futures).await;

    // Combine results (first decision wins)
    combine_hook_outputs(results)
}
```

**Test Reference**: `test_scenario_parallel_hook_execution`, `test_scenario_deduplication`

---

## Part 4: Configuration Loading

### Parse from JSON

```rust
impl HooksConfiguration {
    pub fn from_json(json: &str) -> Result<Self> {
        let obj: serde_json::Value = serde_json::from_str(json)?;

        Ok(HooksConfiguration {
            session_start: parse_event_config(&obj, "SessionStart")?,
            session_end: parse_event_config(&obj, "SessionEnd")?,
            pre_tool_use: parse_event_config(&obj, "PreToolUse")?,
            post_tool_use: parse_event_config(&obj, "PostToolUse")?,
            user_prompt_submit: parse_event_config(&obj, "UserPromptSubmit")?,
            stop: parse_event_config(&obj, "Stop")?,
            subagent_stop: parse_event_config(&obj, "SubagentStop")?,
            notification: parse_event_config(&obj, "Notification")?,
            pre_compact: parse_event_config(&obj, "PreCompact")?,
        })
    }
}

fn parse_event_config(
    obj: &serde_json::Value,
    event: &str,
) -> Result<Vec<HookConfig>> {
    let mut configs = Vec::new();

    if let Some(items) = obj[event].as_array() {
        for item in items {
            let matcher = if let Some(m) = item["matcher"].as_str() {
                if m.contains("|") || m.contains("*") {
                    HookMatcher::Regex(m.to_string())
                } else {
                    HookMatcher::Exact(m.to_string())
                }
            } else {
                HookMatcher::Exact("*".to_string())
            };

            let mut hooks = Vec::new();
            if let Some(hook_items) = item["hooks"].as_array() {
                for hook_item in hook_items {
                    let hook_type = hook_item["type"].as_str().unwrap_or("command");
                    let hook = match hook_type {
                        "command" => {
                            let cmd = hook_item["command"]
                                .as_str()
                                .unwrap_or("")
                                .to_string();
                            Hook {
                                hook_type: HookType::Command(cmd),
                                timeout_ms: hook_item["timeout"]
                                    .as_u64()
                                    .unwrap_or(60000) as u32,
                            }
                        }
                        "prompt" => Hook {
                            hook_type: HookType::Prompt,
                            timeout_ms: hook_item["timeout"]
                                .as_u64()
                                .unwrap_or(60000) as u32,
                        },
                        _ => continue,
                    };
                    hooks.push(hook);
                }
            }

            configs.push(HookConfig { matcher, hooks });
        }
    }

    Ok(configs)
}
```

**Test Reference**: `test_parse_hook_configuration_json`

---

## Part 5: Integration Example

### Complete Hook Workflow

```rust
async fn process_user_request_with_hooks(
    config: &HooksConfiguration,
    user_input: &str,
    session_id: &str,
) -> Result<()> {
    let context = HookContext {
        session_id: session_id.to_string(),
        transcript_path: format!("/tmp/{}.log", session_id),
        cwd: std::env::current_dir()?.to_string_lossy().to_string(),
        permission_mode: "auto".to_string(),
        hook_event_name: String::new(),
    };

    // 1. SessionStart hooks (if first request)
    if is_first_request(session_id) {
        trigger_session_start_hooks(config, &context).await?;
    }

    // 2. UserPromptSubmit hooks
    let additional_context = trigger_user_prompt_submit_hooks(config, &context)
        .await?;
    let enriched_input = format!("{}\n{}", user_input,
        additional_context.unwrap_or_default());

    // 3. Process request (tool calls may occur)
    process_request(&enriched_input).await?;

    // 4. PreToolUse hooks (for each tool)
    // 5. PostToolUse hooks (for each tool)

    // 6. Stop hooks
    let decision = trigger_stop_hooks(config, &context).await?;

    if matches!(decision, Decision::Approve) {
        // 7. SessionEnd hooks
        trigger_session_end_hooks(config, &context).await?;
    }

    Ok(())
}
```

**Test Reference**: `test_scenario_session_workflow`

---

## Part 6: Error Handling

### Handle All Exit Codes

```rust
enum HookExitCode {
    Success = 0,           // Visible in transcript
    NonBlockingWarning = 1, // Shown to user
    BlockingError = 2,     // Fed to Claude
}

fn handle_hook_exit_code(code: i32) -> Result<HookOutput> {
    match code {
        0 => Ok(HookOutput::default()),
        1 => {
            eprintln!("Hook warning");
            Ok(HookOutput::default())
        }
        2 => Err("Hook blocked execution".into()),
        _ => Err("Unknown hook exit code".into()),
    }
}
```

**Test Reference**: `test_hook_result_success`, `test_hook_result_non_blocking_error`, `test_hook_result_blocking_error`

### Validate Input

```rust
fn validate_hook_type(hook_type: &str) -> Result<()> {
    match hook_type {
        "command" | "prompt" => Ok(()),
        _ => Err(format!("Invalid hook type: {}", hook_type).into()),
    }
}

fn validate_permission_decision(decision: &str) -> Result<()> {
    match decision {
        "allow" | "deny" | "ask" => Ok(()),
        _ => Err(format!("Invalid permission decision: {}", decision).into()),
    }
}

fn validate_completion_decision(decision: &str) -> Result<()> {
    match decision {
        "approve" | "block" => Ok(()),
        _ => Err(format!("Invalid completion decision: {}", decision).into()),
    }
}
```

**Test Reference**: `test_hook_type_validation_invalid`, `test_permission_decision_invalid_value`, `test_hook_decision_invalid_value`

---

## Part 7: Environment Persistence

### Support $CLAUDE_ENV_FILE

```rust
fn apply_environment_file(env_file_path: &str) -> Result<()> {
    if !std::path::Path::new(env_file_path).exists() {
        return Ok(()); // Not an error if doesn't exist
    }

    let content = std::fs::read_to_string(env_file_path)?;

    for line in content.lines() {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if let Some((key, value)) = line.split_once('=') {
            std::env::set_var(key, value);
        }
    }

    Ok(())
}

// In SessionStart hooks:
// source $CLAUDE_ENV_FILE
// This enables:
// export CUSTOM_VAR=value
// export API_KEY=secret
```

**Test Reference**: `test_scenario_environment_persistence`

---

## Part 8: MCP Tool Support

### Target MCP Tools

```rust
fn is_mcp_tool(tool_name: &str) -> bool {
    tool_name.starts_with("mcp__")
}

fn parse_mcp_tool(tool_name: &str) -> Option<(String, String)> {
    // Format: mcp__<server>__<tool>
    let parts: Vec<&str> = tool_name.split("__").collect();
    if parts.len() == 3 {
        Some((parts[1].to_string(), parts[2].to_string()))
    } else {
        None
    }
}

// In hook matchers:
// "mcp__.*" - matches all MCP tools
// "mcp__server_name__.*" - matches specific server
```

**Test Reference**: `test_custom_hook_registration_mcp_tool`, `test_scenario_mcp_tool_targeting`

---

## Testing Your Implementation

### Use the Test Suite to Validate

```bash
# Run all tests
cargo test --test hooks_tests

# Your implementation should:
# 1. Create all structures matching test definitions
# 2. Implement all lifecycle event handlers
# 3. Support both hook types (command, prompt)
# 4. Handle all decision types
# 5. Support timeout management
# 6. Parse JSON configuration

# Once implemented, tests will ensure:
✓ All events trigger correctly
✓ All decisions are respected
✓ Errors are handled properly
✓ Real-world patterns work
```

---

## Checklist for Implementation

- [ ] Define Hook, HookMatcher, HookConfig structures
- [ ] Define HooksConfiguration with all 9 events
- [ ] Implement HookContext and HookOutput
- [ ] Implement SessionStart hook handler
- [ ] Implement PreToolUse hook handler
- [ ] Implement PostToolUse hook handler
- [ ] Implement Stop hook handler
- [ ] Implement SessionEnd hook handler
- [ ] Implement UserPromptSubmit hook handler
- [ ] Implement SubagentStop hook handler
- [ ] Implement Notification hook handler
- [ ] Implement PreCompact hook handler
- [ ] Implement command hook execution
- [ ] Implement prompt hook execution
- [ ] Implement parallel execution
- [ ] Implement hook deduplication
- [ ] Implement JSON configuration loading
- [ ] Implement permission decision handling
- [ ] Implement completion decision handling
- [ ] Implement exit code handling
- [ ] Implement timeout management
- [ ] Implement environment file support
- [ ] Implement MCP tool targeting
- [ ] All 74 tests passing

---

## References

- **Full Test Suite**: `/Users/ryan/src/declawed/claude-code-rs/crates/cli/tests/hooks_tests.rs`
- **Coverage Report**: `/Users/ryan/src/declawed/claude-code-rs/HOOKS_TEST_COVERAGE.md`
- **Quick Start**: `/Users/ryan/src/declawed/claude-code-rs/HOOKS_QUICK_START.md`
- **Official Docs**: https://code.claude.com/docs/en/hooks
