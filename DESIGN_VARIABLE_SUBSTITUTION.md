# Design: ${CLAUDE_PLUGIN_ROOT} Variable Substitution in Plugin Frontmatter

**Issue**: ${CLAUDE_PLUGIN_ROOT} variables in agent/command frontmatter `allowed-tools` fields are not substituted with actual plugin root paths, causing tools to incorrectly require approval.

**Status**: Design Phase

---

## Problem Analysis

### Current State
1. Plugin system loads frontmatter from agent/command definitions
2. Frontmatter can include `allowed-tools` field with tool paths
3. Plugin creators use `${CLAUDE_PLUGIN_ROOT}` to reference files relative to plugin root
4. Variable is NOT substituted - remains as literal string `"${CLAUDE_PLUGIN_ROOT}/..."`
5. Tool system sees literal string, not resolved path, causing permission failures

### Root Cause
Frontmatter parsing stops at YAML deserialization. No variable substitution pass occurs before the frontmatter is used downstream.

### Impact
- Agent frontmatter with `allowed-tools: ["${CLAUDE_PLUGIN_ROOT}/tools/read-only"]` fails
- Tool permissions incorrectly reject valid allowed tools
- Plugin creators cannot use relative paths cleanly
- Forces absolute paths or workarounds in frontmatter

---

## Solution: Variable Substitution Module

### Module Name: `frontmatter_substitution`

**Location**: `crates/cli/src/plugins/frontmatter_substitution.rs`

**Single Responsibility**: Substitute environment variables and plugin-relative paths in frontmatter values.

### Design Principles

1. **Minimal Scope**: Only handles substitution, not parsing
2. **Reusable**: Works with any frontmatter structure
3. **Tested**: Comprehensive test coverage for all variable types
4. **Extensible**: Easy to add new variable types
5. **Non-destructive**: Preserves values that don't match patterns

### Supported Variables

```rust
// Pattern: ${VARIABLE_NAME}
// Examples:
${CLAUDE_PLUGIN_ROOT}     // Plugin root directory path
${CLAUDE_PROJECT_ROOT}    // Project root directory path
${HOME}                   // User home directory
${USER}                   // Current user
${PWD}                    // Current working directory
```

---

## Architecture

### Core Components

#### 1. Substitution Context

```rust
/// Context for variable substitution
pub struct SubstitutionContext {
    plugin_root: PathBuf,
    project_root: Option<PathBuf>,
    env_vars: HashMap<String, String>,
}
```

Provides lookup for all available variables.

#### 2. Variable Matcher

```rust
pub enum Variable {
    PluginRoot,
    ProjectRoot,
    Home,
    User,
    Pwd,
    Custom(String),
}

impl Variable {
    pub fn from_string(name: &str) -> Option<Self> { }
    pub fn resolve(&self, ctx: &SubstitutionContext) -> Option<String> { }
}
```

Identifies and resolves variables.

#### 3. Substitution Engine

```rust
pub struct Substituter {
    ctx: SubstitutionContext,
}

impl Substituter {
    pub fn new(ctx: SubstitutionContext) -> Self { }

    /// Substitute variables in a single string
    pub fn substitute(&self, value: &str) -> String { }

    /// Substitute variables in frontmatter
    pub fn substitute_frontmatter(&self, frontmatter: &mut FrontMatter) { }

    /// Substitute variables in all string values in a map
    pub fn substitute_map(&self, map: &mut HashMap<String, String>) { }
}
```

Main substitution logic.

#### 4. Pattern Matching

Use simple regex pattern: `\$\{([A-Z_]+)\}`

- Matches `${VARIABLE_NAME}` syntax
- Captures variable name
- Non-greedy to handle multiple variables in one string

### Data Flow

```
Plugin Loading
    ↓
Read frontmatter from file
    ↓
YAML parse → FrontMatter struct (current)
    ↓
[NEW] Substitution Pass
    - Create SubstitutionContext with plugin_root
    - Pass to Substituter
    - Substitute all string values in allowed_tools, disallowed_tools
    ↓
Use frontmatter downstream (unchanged API)
    ↓
Tool system sees resolved paths
```

---

## Integration Points

### 1. Commands Loader (agent/command frontmatter)

**File**: `crates/cli/src/commands/loader.rs`

**Method**: `CommandLoader::load_command()`

**Current**:
```rust
let (frontmatter, body) = self.parse_frontmatter(&content)?;
```

**New**:
```rust
let (mut frontmatter, body) = self.parse_frontmatter(&content)?;
let ctx = SubstitutionContext::new(plugin_root, project_root);
let substituter = Substituter::new(ctx);
substituter.substitute_frontmatter(&mut frontmatter);
```

### 2. Agent Discovery

**File**: `crates/cli/src/plugins/agent_discovery.rs`

**Method**: `AgentDiscovery::load_agent_from_file()`

**Current**:
```rust
Ok(Some(AgentDefinition {
    disallowed_tools: vec![],
    ...
}))
```

**New**: Pass plugin_root context to substitution when loading file-based agents.

### 3. Runtime Agents

**File**: `crates/cli/src/plugins/agent_discovery.rs`

**Method**: `RuntimeAgentDefinition::disallowed_tools` field

**Behavior**: Runtime agents pass through CLI JSON, no file-based substitution needed.
However, if runtime agents can contain `${...}` in their JSON, they should be substituted too.

---

## Implementation Sequence

### Phase 1: Core Module
1. Create `frontmatter_substitution.rs` with:
   - `Variable` enum
   - `SubstitutionContext` struct
   - `Substituter` struct
   - Comprehensive unit tests

### Phase 2: Integration - Commands
1. Update `CommandLoader::parse_frontmatter()` or create post-parse step
2. Integrate substitution after YAML parsing
3. Update tests to verify substitution

### Phase 3: Integration - Agents
1. Update `AgentDiscovery::load_agent_from_file()`
2. Pass plugin root context
3. Apply substitution to disallowed_tools
4. Update tests

### Phase 4: Testing
1. Unit tests for substitution logic
2. Integration tests with real plugin structures
3. Edge case tests (nested variables, partial matches, empty values)

---

## Code Specification

### Module: frontmatter_substitution

#### Location
```
crates/cli/src/plugins/frontmatter_substitution.rs
```

#### Public API

```rust
/// Variable types supported by substitution
pub enum Variable {
    PluginRoot,
    ProjectRoot,
    Home,
    User,
    Pwd,
}

/// Context for variable resolution
pub struct SubstitutionContext {
    plugin_root: PathBuf,
    project_root: Option<PathBuf>,
    // System env vars loaded on demand
}

/// Performs variable substitution
pub struct Substituter {
    ctx: SubstitutionContext,
}

impl SubstitutionContext {
    /// Create new context
    pub fn new(
        plugin_root: impl Into<PathBuf>,
        project_root: Option<PathBuf>,
    ) -> Self;
}

impl Substituter {
    /// Create new substituter
    pub fn new(ctx: SubstitutionContext) -> Self;

    /// Substitute ${VAR} patterns in a string
    /// Returns substituted string, leaves unmatched patterns as-is
    pub fn substitute(&self, value: &str) -> String;

    /// Substitute variables in FrontMatter allowed_tools field
    pub fn substitute_frontmatter(&self, frontmatter: &mut FrontMatter);
}

impl Variable {
    /// Parse variable name to Variable enum
    pub fn from_string(name: &str) -> Option<Self>;

    /// Resolve variable to actual value using context
    pub fn resolve(&self, ctx: &SubstitutionContext) -> Option<String>;
}
```

#### Tests

```rust
#[cfg(test)]
mod tests {
    // Basic substitution
    test_substitute_plugin_root()
    test_substitute_project_root()
    test_substitute_home()
    test_substitute_user()
    test_substitute_pwd()

    // Multiple variables
    test_substitute_multiple_variables_in_one_string()
    test_substitute_multiple_fields()

    // Edge cases
    test_substitute_nested_variables_not_supported()
    test_substitute_unmatched_pattern_left_alone()
    test_substitute_empty_variable_name()
    test_substitute_malformed_pattern()
    test_substitute_empty_string()

    // Frontmatter integration
    test_substitute_frontmatter_allowed_tools()
    test_substitute_frontmatter_disallowed_tools()
    test_substitute_frontmatter_preserves_other_fields()

    // Real plugin scenarios
    test_real_plugin_with_allowed_tools_paths()
    test_real_plugin_with_mixed_absolute_and_relative()
}
```

---

## Testing Strategy

### Unit Tests (frontmatter_substitution module)
- Variable matching and resolution
- Edge cases (malformed, empty, nested)
- Multiple variables in one value
- Context resolution with missing optional paths

### Integration Tests
- Load agent file with substitution
- Load command file with substitution
- Verify downstream tool system receives resolved paths
- Plugin with disallowedTools containing variables

### Scenario Tests
1. **Plugin with relative tool paths**
   - Input: `allowed-tools: ["${CLAUDE_PLUGIN_ROOT}/tools/safe"]`
   - Output: `/actual/plugin/root/tools/safe`

2. **Mixed substitution**
   - Input: `allowed-tools: ["${CLAUDE_PLUGIN_ROOT}/tools", "/absolute/path", "builtin-tool"]`
   - Output: `["/actual/plugin/root/tools", "/absolute/path", "builtin-tool"]`

3. **Invalid pattern preservation**
   - Input: `["${INVALID_VAR}/path", "${CLAUDE_PLUGIN_ROOT}/valid"]`
   - Output: `["${INVALID_VAR}/path", "/actual/plugin/root/valid"]` (invalid left alone)

---

## Error Handling

**Philosophy**: Fail gracefully with logging.

- If plugin_root cannot be determined: log warning, leave `${CLAUDE_PLUGIN_ROOT}` as-is
- If HOME not available: log warning, leave `${HOME}` as-is
- If variable name unknown: leave pattern as-is (safe default)
- Never panic or abort - substitution is best-effort enhancement

---

## Backwards Compatibility

**Impact**: Zero breaking changes.

- Existing frontmatter without variables works unchanged
- New variable syntax is opt-in
- Non-matching patterns left as-is
- All downstream APIs unchanged

---

## Performance Considerations

**Substitution is lightweight:**
- Single regex pass per string value
- HashMap lookups for variable resolution
- No file I/O
- Runs once during plugin load (not hot path)

---

## Documentation

### For Plugin Developers
```markdown
# Variable Substitution in Frontmatter

You can use environment-like variables in frontmatter fields:

## Supported Variables

- ${CLAUDE_PLUGIN_ROOT} - Path to your plugin root directory
- ${CLAUDE_PROJECT_ROOT} - Path to project root
- ${HOME} - User home directory
- ${USER} - Current user
- ${PWD} - Current working directory

## Examples

### allowed-tools with plugin-relative paths

agents/secure.md:
---
allowed-tools:
  - "${CLAUDE_PLUGIN_ROOT}/bin/verify-script"
  - "Read"
  - "Grep"
---

Becomes:
---
allowed-tools:
  - "/path/to/plugin/bin/verify-script"
  - "Read"
  - "Grep"
---
```

---

## Summary

**Simplicity**: Single-responsibility module handling variable substitution.

**Integration**: Minimal changes to existing loader code (add post-parse substitution).

**Testing**: Comprehensive unit and integration tests.

**Impact**: Enables plugin developers to write cleaner, plugin-relative paths.

**No Breaking Changes**: Existing frontmatter works unchanged.

