# Implementation Guide: ${CLAUDE_PLUGIN_ROOT} Variable Substitution

## Quick Reference

**What**: Substitute ${VARIABLE_NAME} in plugin frontmatter tool paths
**Where**: New module `crates/cli/src/plugins/frontmatter_substitution.rs`
**When**: During plugin loading, after frontmatter YAML parsing
**Why**: Enable plugin-relative paths instead of absolute paths

---

## Step-by-Step Implementation

### Step 1: Create frontmatter_substitution.rs

**File**: `crates/cli/src/plugins/frontmatter_substitution.rs`

**Content Structure**:

```rust
use std::collections::HashMap;
use std::path::{Path, PathBuf};

// Variable enum - what can be substituted
pub enum Variable {
    PluginRoot,
    ProjectRoot,
    Home,
    User,
    Pwd,
}

// Context - where to look up values
pub struct SubstitutionContext {
    plugin_root: PathBuf,
    project_root: Option<PathBuf>,
}

// Substituter - does the work
pub struct Substituter {
    ctx: SubstitutionContext,
}

impl SubstitutionContext {
    pub fn new(plugin_root: impl Into<PathBuf>, project_root: Option<PathBuf>) -> Self { }
}

impl Substituter {
    pub fn new(ctx: SubstitutionContext) -> Self { }

    /// Main API: substitute variables in a string
    pub fn substitute(&self, value: &str) -> String { }

    /// Convenience: substitute in allowed_tools, disallowed_tools fields
    pub fn substitute_frontmatter(&self, frontmatter: &mut crate::commands::loader::FrontMatter) { }
}

impl Variable {
    pub fn from_string(name: &str) -> Option<Self> { }
    pub fn resolve(&self, ctx: &SubstitutionContext) -> Option<String> { }
}
```

### Step 2: Implement Pattern Matching

**Algorithm**:

1. Find all `${WORD}` patterns using regex or manual parsing
2. For each match, extract variable name
3. Resolve variable using context
4. Replace in original string

**Pattern**: `\$\{([A-Z_]+)\}`

```rust
// Example: "${CLAUDE_PLUGIN_ROOT}/tools"
// Regex captures: "CLAUDE_PLUGIN_ROOT"
// Resolve: "/actual/plugin/root"
// Result: "/actual/plugin/root/tools"
```

**Manual Implementation** (no regex dependency):

```rust
pub fn substitute(&self, value: &str) -> String {
    let mut result = String::new();
    let mut chars = value.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '$' && chars.peek() == Some(&'{') {
            chars.next(); // consume '{'
            let mut var_name = String::new();

            while let Some(&next_ch) = chars.peek() {
                if next_ch == '}' {
                    chars.next(); // consume '}'
                    if let Some(var) = Variable::from_string(&var_name) {
                        if let Some(value) = var.resolve(&self.ctx) {
                            result.push_str(&value);
                        } else {
                            result.push_str(&format!("${{{}}}", var_name));
                        }
                    } else {
                        result.push_str(&format!("${{{}}}", var_name));
                    }
                    break;
                }
                var_name.push(chars.next().unwrap());
            }
        } else {
            result.push(ch);
        }
    }

    result
}
```

### Step 3: Implement Variable Resolution

```rust
impl Variable {
    pub fn from_string(name: &str) -> Option<Self> {
        match name {
            "CLAUDE_PLUGIN_ROOT" => Some(Variable::PluginRoot),
            "CLAUDE_PROJECT_ROOT" => Some(Variable::ProjectRoot),
            "HOME" => Some(Variable::Home),
            "USER" => Some(Variable::User),
            "PWD" => Some(Variable::Pwd),
            _ => None,
        }
    }

    pub fn resolve(&self, ctx: &SubstitutionContext) -> Option<String> {
        match self {
            Variable::PluginRoot => {
                Some(ctx.plugin_root.to_string_lossy().to_string())
            }
            Variable::ProjectRoot => {
                ctx.project_root.as_ref().map(|p| p.to_string_lossy().to_string())
            }
            Variable::Home => std::env::var("HOME").ok(),
            Variable::User => std::env::var("USER").ok(),
            Variable::Pwd => std::env::current_dir().ok()
                .map(|p| p.to_string_lossy().to_string()),
        }
    }
}
```

### Step 4: Integrate with FrontMatter

```rust
pub fn substitute_frontmatter(&self, frontmatter: &mut FrontMatter) {
    // Substitute allowed_tools
    for tool in &mut frontmatter.allowed_tools {
        *tool = self.substitute(tool);
    }

    // Substitute description if present
    if let Some(desc) = &mut frontmatter.description {
        *desc = self.substitute(desc);
    }

    // Substitute argument_hint if present
    if let Some(hint) = &mut frontmatter.argument_hint {
        *hint = self.substitute(hint);
    }
}
```

### Step 5: Write Comprehensive Tests

**Test Categories**:

1. **Variable Resolution**
   - Each variable type resolves correctly
   - Returns None for unavailable variables

2. **Pattern Matching**
   - Matches `${VAR_NAME}` correctly
   - Multiple variables in one string
   - No match for $VAR (missing braces)
   - No match for ${var} (lowercase)

3. **Substitution**
   - Replaces correctly in strings
   - Preserves non-matching text
   - Handles empty strings
   - Unknown variables left as-is

4. **Edge Cases**
   - Nested ${} not supported - left as-is
   - Malformed patterns - left as-is
   - Multiple variables - all substituted
   - Partial paths work correctly

5. **Frontmatter Integration**
   - Works with real FrontMatter struct
   - Substitutes allowed_tools correctly
   - Preserves other fields

### Step 6: Update Module Exports

**File**: `crates/cli/src/plugins/mod.rs`

```rust
pub mod frontmatter_substitution;
pub use frontmatter_substitution::{Substituter, SubstitutionContext, Variable};
```

### Step 7: Integrate into CommandLoader

**File**: `crates/cli/src/commands/loader.rs`

**Add parameter to load_command**:

```rust
pub async fn load_command(
    &self,
    path: &Path,
    plugin_root: Option<&Path>,
    project_root: Option<&Path>,
) -> Result<LoadedCommand> {
    let content = fs::read_to_string(path)
        .await
        .context("Failed to read command file")?;

    let name = path
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| anyhow!("Invalid file name"))?
        .to_string();

    let (mut frontmatter, body) = self.parse_frontmatter(&content)?;

    // NEW: Apply variable substitution
    if let Some(plugin_root) = plugin_root {
        use crate::plugins::{Substituter, SubstitutionContext};
        let ctx = SubstitutionContext::new(
            plugin_root,
            project_root.map(|p| p.to_path_buf()),
        );
        let substituter = Substituter::new(ctx);
        substituter.substitute_frontmatter(&mut frontmatter);
    }

    Ok(LoadedCommand {
        name,
        frontmatter,
        content: body,
    })
}
```

### Step 8: Update AgentDiscovery (Optional)

**File**: `crates/cli/src/plugins/agent_discovery.rs`

**In load_agent_from_file**:

```rust
// After creating AgentDefinition but before returning
if let Some(plugin_root) = self.plugin_root {
    use crate::plugins::{Substituter, SubstitutionContext};
    let ctx = SubstitutionContext::new(plugin_root, None);
    let substituter = Substituter::new(ctx);
    // Note: disallowed_tools not currently loaded from frontmatter
    // but this shows the pattern if future enhancements add it
}
```

---

## Testing Plan

### Unit Test File Structure

```rust
#[cfg(test)]
mod tests {
    use super::*;

    // Variable Tests
    mod variable {
        #[test]
        fn from_string_plugin_root() { }
        #[test]
        fn from_string_project_root() { }
        #[test]
        fn from_string_home() { }
        #[test]
        fn from_string_user() { }
        #[test]
        fn from_string_pwd() { }
        #[test]
        fn from_string_unknown() { }

        #[test]
        fn resolve_plugin_root() { }
        #[test]
        fn resolve_project_root() { }
        #[test]
        fn resolve_home() { }
    }

    // Substitution Tests
    mod substitution {
        #[test]
        fn single_variable() { }
        #[test]
        fn multiple_variables() { }
        #[test]
        fn unknown_variable_preserved() { }
        #[test]
        fn malformed_pattern_preserved() { }
        #[test]
        fn empty_string() { }
        #[test]
        fn no_variables() { }
    }

    // FrontMatter Integration
    mod frontmatter {
        #[test]
        fn substitute_allowed_tools() { }
        #[test]
        fn substitute_description() { }
        #[test]
        fn preserves_non_string_fields() { }
    }

    // Scenarios
    mod scenarios {
        #[test]
        fn plugin_with_relative_tool_paths() { }
        #[test]
        fn plugin_with_mixed_paths() { }
    }
}
```

---

## Integration Checklist

- [ ] Create frontmatter_substitution.rs module
- [ ] Implement Variable enum
- [ ] Implement SubstitutionContext struct
- [ ] Implement Substituter with substitute() method
- [ ] Implement substitute_frontmatter() for FrontMatter integration
- [ ] Add comprehensive unit tests
- [ ] Update plugins/mod.rs to export public types
- [ ] Update CommandLoader to accept plugin_root parameter
- [ ] Integrate substitution call after frontmatter parse
- [ ] Update AgentDiscovery if needed
- [ ] Add integration tests
- [ ] Verify no regression in existing plugin loading

---

## Validation Criteria

1. **Correctness**: `${CLAUDE_PLUGIN_ROOT}/tool` → `/actual/root/tool`
2. **Graceful Degradation**: Unknown variables left as-is
3. **No Breaking Changes**: Existing frontmatter works unchanged
4. **Performance**: Substitution is fast (< 1ms per string)
5. **Test Coverage**: 100% of public API
6. **Documentation**: Clear examples for plugin developers

---

## Common Pitfalls to Avoid

1. ❌ **Case Sensitivity**: Use `${CLAUDE_PLUGIN_ROOT}` not `${Claude_Plugin_Root}`
2. ❌ **Nested Variables**: Don't try to support `${VAR${NESTED}}` - leave as-is
3. ❌ **File I/O**: Don't validate paths exist - just do string substitution
4. ❌ **Panic on Error**: Always degrade gracefully, never panic
5. ❌ **Over-Substitution**: Only substitute frontmatter, not command body content

---

## Quick Start Command

```bash
# Create the module
touch crates/cli/src/plugins/frontmatter_substitution.rs

# Add to mod.rs
echo "pub mod frontmatter_substitution;" >> crates/cli/src/plugins/mod.rs

# Run tests as you develop
cargo test frontmatter_substitution --lib
```

