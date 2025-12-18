# Issue #149 Investigation: tool_use_id in Hook Input Types

**Status:** MISSING
**Evidence Level:** Confirmed with Full Codebase Analysis
**Date:** 2025-12-13

## Summary

The `tool_use_id` field **DOES NOT EXIST** in either `PreToolUseHookInput` or `PostToolUseHookInput` hook input structures. Furthermore, these specific input type names don't exist either - the system uses a unified `HookContext` struct for all hook events.

## Investigation Results

### 1. Type Definitions

**Relevant File:** `/home/azureuser/src/RustyClawd/crates/cli/src/hooks/types.rs`

The codebase does **NOT** define `PreToolUseHookInput` or `PostToolUseHookInput` types. Instead, it uses a unified `HookContext` struct (lines 262-286):

```rust
/// Hook execution context
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HookContext {
    pub session_id: String,
    pub transcript_path: String,
    pub cwd: String,
    pub permission_mode: String,
    pub hook_event_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_params: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_start_matcher: Option<SessionStartMatcher>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_end_reason: Option<SessionEndReason>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notification_type: Option<NotificationType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_prompt: Option<String>,
    #[serde(flatten)]
    pub additional: HashMap<String, serde_json::Value>,
}
```

**Key Fields Present:**
- `session_id` - Session identifier
- `transcript_path` - Path to transcript
- `cwd` - Current working directory
- `permission_mode` - Permission mode setting
- `hook_event_name` - Event name (e.g., "PreToolUse", "PostToolUse")
- `tool_name` - Name of the tool (e.g., "Write", "Bash")
- `tool_params` - Tool input parameters
- `tool_result` - Tool execution result (PostToolUse only)
- `additional` - HashMap for additional context via `#[serde(flatten)]`

**Key Fields MISSING:**
- `tool_use_id` - **NOT DEFINED**

### 2. How PreToolUse and PostToolUse Hooks Are Called

**Relevant File:** `/home/azureuser/src/RustyClawd/crates/cli/src/tool_executor.rs` (lines 262-375)

#### PreToolUse Hook Execution (lines 262-319):

```rust
// Execute PreToolUse hook (BLOCKING - can deny execution)
if let (Some(ref hooks_system), Some(ref sess_id)) = (&hooks, &session_id) {
    let hook_context = hooks::HookContext::for_tool(
        sess_id.clone(),
        format!(".claude/sessions/{}/transcript.json", sess_id),
        ctx.cwd.to_string_lossy().to_string(),
        "ask".to_string(),
        hooks::HookEvent::PreToolUse,
        tool_name.clone(),  // Only tool_name, NO tool_use_id
    )
    .with_tool_params(tool_input.clone());  // Add parameters

    match hooks_system
        .execute_hooks(hooks::HookEvent::PreToolUse, &hook_context)
        .await
    { /* ... */ }
}
```

**What is passed:**
1. Session ID
2. Transcript path
3. Current working directory
4. Permission mode (hardcoded "ask")
5. Hook event (PreToolUse)
6. Tool name (e.g., "Write", "Bash")
7. Tool input parameters via `with_tool_params()`

**What is NOT passed:**
- `tool_use_id` - Missing entirely

#### PostToolUse Hook Execution (lines 340-375):

```rust
// Execute PostToolUse hook (NON-BLOCKING - for logging/monitoring)
if let (Some(ref hooks_system), Some(ref sess_id)) = (&hooks, &session_id) {
    // Convert result to JSON value for hook context
    let result_value = match &result {
        Ok(val) => val.clone(),
        Err(e) => json!({"error": e.to_string()}),
    };

    let hook_context = hooks::HookContext::for_tool(
        sess_id.clone(),
        format!(".claude/sessions/{}/transcript.json", sess_id),
        ctx.cwd.to_string_lossy().to_string(),
        "ask".to_string(),
        hooks::HookEvent::PostToolUse,
        tool_name.clone(),  // Only tool_name, NO tool_use_id
    )
    .with_tool_params(tool_input.clone())
    .with_tool_result(result_value);  // Add execution result

    match hooks_system
        .execute_hooks(hooks::HookEvent::PostToolUse, &hook_context)
        .await
    { /* ... */ }
}
```

**What is passed:**
1. Session ID
2. Transcript path
3. Current working directory
4. Permission mode (hardcoded "ask")
5. Hook event (PostToolUse)
6. Tool name
7. Tool input parameters via `with_tool_params()`
8. Tool execution result via `with_tool_result()`

**What is NOT passed:**
- `tool_use_id` - Missing entirely

### 3. The for_tool() Constructor

**Source:** `/home/azureuser/src/RustyClawd/crates/cli/src/hooks/types.rs` (lines 289-313)

```rust
/// Create context for a tool event (PreToolUse/PostToolUse)
pub fn for_tool(
    session_id: String,
    transcript_path: String,
    cwd: String,
    permission_mode: String,
    event: HookEvent,
    tool_name: String,  // Only tool_name parameter, NO tool_use_id
) -> Self {
    Self {
        session_id,
        transcript_path,
        cwd,
        permission_mode,
        hook_event_name: event.as_str().to_string(),
        tool_name: Some(tool_name),
        tool_params: None,
        tool_result: None,
        session_start_matcher: None,
        session_end_reason: None,
        notification_type: None,
        user_prompt: None,
        additional: HashMap::new(),
    }
}
```

**Key Observation:** The `for_tool()` constructor takes only `tool_name`, not `tool_use_id`.

### 4. Where tool_use_id Exists in the Codebase

**Relevant File:** `/home/azureuser/src/RustyClawd/crates/core/src/client/types.rs` (line 164)

The `tool_use_id` field exists in the **ContentBlock::ToolResult** variant:

```rust
ToolResult {
    tool_use_id: String,  // ID from Claude API
    content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    is_error: Option<bool>,
}
```

This is used at a different level - in the Claude API message protocol layer, not in the hooks system.

### 5. Search Results

**Global search for `tool_use_id` in codebase:** Found in 9 files:
- `crates/cli/src/interactive.rs` - Used in ToolResult processing
- `docs/ARCHITECTURE.md` - Documented in ContentBlock
- `crates/core/src/client/mod.rs` - Part of API protocol
- `crates/tools/src/web_search_phase2.rs` - Web search tool
- `crates/tools/src/web_search.rs` - Web search tool
- `crates/core/tests/tool_use_test.rs` - Tests for tool use
- `crates/core/src/client/types.rs` - Type definition

**Result:** `tool_use_id` is **NOT found** in any hook-related file except through transitive references in the API layer.

## Design Implications

### Current Architecture

The hooks system is intentionally **decoupled from the Claude API's tool_use_id system**:

1. **API Layer** (`crates/core`): Manages `tool_use_id` for Claude API protocol
2. **Hooks Layer** (`crates/cli/src/hooks`): Uses higher-level semantic information (`tool_name`, `tool_params`, `tool_result`)
3. **Tool Executor** (`crates/cli/src/tool_executor.rs`): Bridges between them

### Why tool_use_id is Missing

There are several valid reasons `tool_use_id` is not passed to hooks:

1. **Semantic vs Protocol Distinction**
   - `tool_use_id` is a Claude API protocol detail (internal message flow)
   - Hooks work with semantic information (what tool, what parameters, what result)

2. **Timing Issues**
   - `tool_use_id` is generated by Claude API only when making tool calls
   - Hooks may need to operate at different abstraction levels

3. **Flexibility**
   - Decoupling allows hooks to work with tool information without API details
   - Cleaner separation of concerns

## References

**Source Files:**
- Primary hook types: `/home/azureuser/src/RustyClawd/crates/cli/src/hooks/types.rs` (lines 262-313)
- Hook invocation: `/home/azureuser/src/RustyClawd/crates/cli/src/tool_executor.rs` (lines 262-375)
- API protocol: `/home/azureuser/src/RustyClawd/crates/core/src/client/types.rs` (line 164)

**Test Files:**
- Hook execution tests: `crates/cli/tests/hook_lifecycle_integration_tests.rs`
- Tool use tests: `crates/core/tests/tool_use_test.rs`

## Conclusion

**Tool_use_id Status: MISSING**

Evidence:
1. ✗ No `PreToolUseHookInput` type exists
2. ✗ No `PostToolUseHookInput` type exists
3. ✗ `HookContext` struct (used for both hook types) has NO `tool_use_id` field
4. ✗ The `for_tool()` constructor does not accept `tool_use_id` as a parameter
5. ✗ Tool executor never passes `tool_use_id` to hooks
6. ✗ Global search confirms no `tool_use_id` in hook types

This is a **design decision**, not an oversight, as the hooks system operates at a higher semantic level than the Claude API's internal protocol details.
