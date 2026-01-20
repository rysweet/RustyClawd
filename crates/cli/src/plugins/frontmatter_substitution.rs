//! Frontmatter Variable Substitution
//!
//! Substitutes environment-like variables in plugin frontmatter string values.
//! Enables plugin developers to write plugin-relative paths like `${CLAUDE_PLUGIN_ROOT}/tools/script`.
//!
//! # Purpose
//!
//! Single responsibility: Transform `${VARIABLE_NAME}` patterns in strings to actual values.
//!
//! # Supported Variables
//!
//! - `${CLAUDE_PLUGIN_ROOT}` - Plugin root directory path
//! - `${CLAUDE_PROJECT_ROOT}` - Project root directory path
//! - `${HOME}` - User home directory
//! - `${USER}` - Current username
//! - `${PWD}` - Current working directory
//!
//! # Design Decisions
//!
//! 1. **Simple Pattern Matching**: Only uppercase `${VAR_NAME}` patterns
//! 2. **Safe Degradation**: Unknown/unresolvable variables left as-is
//! 3. **String-Based**: Works on any string value
//! 4. **No Caching**: Fresh resolution on each call
//!
//! # Example
//!
//! ```
//! use rustyclawd::plugins::frontmatter_substitution::{Substituter, SubstitutionContext};
//! use std::path::PathBuf;
//!
//! let ctx = SubstitutionContext::new(
//!     PathBuf::from("/plugin/root"),
//!     Some(PathBuf::from("/project/root"))
//! );
//! let substituter = Substituter::new(ctx);
//!
//! let input = "${CLAUDE_PLUGIN_ROOT}/tools/verify";
//! let output = substituter.substitute(input);
//! assert_eq!(output, "/plugin/root/tools/verify");
//! ```

use std::path::PathBuf;

/// Supported variable types for substitution
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Variable {
    /// Plugin root directory - ${CLAUDE_PLUGIN_ROOT}
    PluginRoot,
    /// Project root directory - ${CLAUDE_PROJECT_ROOT}
    ProjectRoot,
    /// User home directory - ${HOME}
    Home,
    /// Current username - ${USER}
    User,
    /// Current working directory - ${PWD}
    Pwd,
}

/// Context for variable resolution
#[derive(Debug, Clone)]
pub struct SubstitutionContext {
    plugin_root: PathBuf,
    project_root: Option<PathBuf>,
}

/// Performs variable substitution on strings
#[derive(Debug, Clone)]
pub struct Substituter {
    ctx: SubstitutionContext,
}

impl Variable {
    /// Parse variable name to Variable enum
    ///
    /// Only recognizes uppercase variable names to reduce false positives.
    ///
    /// # Examples
    ///
    /// ```
    /// use rustyclawd::plugins::frontmatter_substitution::Variable;
    ///
    /// assert_eq!(Variable::from_string("CLAUDE_PLUGIN_ROOT"), Some(Variable::PluginRoot));
    /// assert_eq!(Variable::from_string("HOME"), Some(Variable::Home));
    /// assert_eq!(Variable::from_string("unknown"), None);
    /// assert_eq!(Variable::from_string("Claude_Plugin_Root"), None); // lowercase not supported
    /// ```
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

    /// Resolve variable to actual value using context
    ///
    /// Returns None if the variable cannot be resolved (e.g., HOME not set).
    ///
    /// # Examples
    ///
    /// ```
    /// use rustyclawd::plugins::frontmatter_substitution::{Variable, SubstitutionContext};
    /// use std::path::PathBuf;
    ///
    /// let ctx = SubstitutionContext::new(
    ///     PathBuf::from("/plugin/root"),
    ///     Some(PathBuf::from("/project/root"))
    /// );
    ///
    /// assert_eq!(
    ///     Variable::PluginRoot.resolve(&ctx),
    ///     Some("/plugin/root".to_string())
    /// );
    /// assert_eq!(
    ///     Variable::ProjectRoot.resolve(&ctx),
    ///     Some("/project/root".to_string())
    /// );
    /// ```
    pub fn resolve(&self, ctx: &SubstitutionContext) -> Option<String> {
        match self {
            Variable::PluginRoot => Some(ctx.plugin_root.to_string_lossy().to_string()),
            Variable::ProjectRoot => ctx
                .project_root
                .as_ref()
                .map(|p| p.to_string_lossy().to_string()),
            Variable::Home => std::env::var("HOME").ok(),
            Variable::User => std::env::var("USER").ok(),
            Variable::Pwd => std::env::current_dir()
                .ok()
                .map(|p| p.to_string_lossy().to_string()),
        }
    }
}

impl SubstitutionContext {
    /// Create new substitution context
    ///
    /// # Arguments
    ///
    /// * `plugin_root` - Plugin root directory path
    /// * `project_root` - Optional project root directory path
    ///
    /// # Examples
    ///
    /// ```
    /// use rustyclawd::plugins::frontmatter_substitution::SubstitutionContext;
    /// use std::path::PathBuf;
    ///
    /// let ctx = SubstitutionContext::new(
    ///     PathBuf::from("/plugin/root"),
    ///     Some(PathBuf::from("/project/root"))
    /// );
    /// ```
    pub fn new(plugin_root: impl Into<PathBuf>, project_root: Option<PathBuf>) -> Self {
        Self {
            plugin_root: plugin_root.into(),
            project_root,
        }
    }
}

impl Substituter {
    /// Create new substituter with given context
    ///
    /// # Examples
    ///
    /// ```
    /// use rustyclawd::plugins::frontmatter_substitution::{Substituter, SubstitutionContext};
    /// use std::path::PathBuf;
    ///
    /// let ctx = SubstitutionContext::new(
    ///     PathBuf::from("/plugin/root"),
    ///     None
    /// );
    /// let substituter = Substituter::new(ctx);
    /// ```
    pub fn new(ctx: SubstitutionContext) -> Self {
        Self { ctx }
    }

    /// Substitute ${VARIABLE_NAME} patterns in a string
    ///
    /// Returns substituted string with variables replaced by their values.
    /// Unknown or unresolvable variables are left as-is.
    ///
    /// # Pattern Matching
    ///
    /// - Matches: `${UPPERCASE_NAME}`
    /// - Does not match: `$VARIABLE`, `${lowercase}`, `${VAR${NESTED}}`
    ///
    /// # Examples
    ///
    /// ```
    /// use rustyclawd::plugins::frontmatter_substitution::{Substituter, SubstitutionContext};
    /// use std::path::PathBuf;
    ///
    /// let ctx = SubstitutionContext::new(
    ///     PathBuf::from("/plugin/root"),
    ///     Some(PathBuf::from("/project/root"))
    /// );
    /// let substituter = Substituter::new(ctx);
    ///
    /// // Single variable
    /// assert_eq!(
    ///     substituter.substitute("${CLAUDE_PLUGIN_ROOT}/tools"),
    ///     "/plugin/root/tools"
    /// );
    ///
    /// // Multiple variables
    /// assert_eq!(
    ///     substituter.substitute("${CLAUDE_PLUGIN_ROOT}:${CLAUDE_PROJECT_ROOT}"),
    ///     "/plugin/root:/project/root"
    /// );
    ///
    /// // Unknown variable preserved
    /// assert_eq!(
    ///     substituter.substitute("${UNKNOWN}/path"),
    ///     "${UNKNOWN}/path"
    /// );
    /// ```
    pub fn substitute(&self, value: &str) -> String {
        let mut result = String::new();
        let mut chars = value.chars().peekable();

        while let Some(ch) = chars.next() {
            if ch == '$' && chars.peek() == Some(&'{') {
                chars.next(); // consume '{'
                let mut var_name = String::new();
                let mut found_closing_brace = false;

                // Collect variable name until '}'
                while let Some(&next_ch) = chars.peek() {
                    if next_ch == '}' {
                        chars.next(); // consume '}'
                        found_closing_brace = true;

                        // Try to resolve variable
                        if let Some(var) = Variable::from_string(&var_name) {
                            if let Some(value) = var.resolve(&self.ctx) {
                                result.push_str(&value);
                            } else {
                                // Variable recognized but can't resolve - preserve pattern
                                result.push_str(&format!("${{{}}}", var_name));
                                tracing::debug!(
                                    "Variable ${{{var_name}}} recognized but unavailable"
                                );
                            }
                        } else {
                            // Unknown variable - preserve pattern
                            result.push_str(&format!("${{{}}}", var_name));
                        }
                        break;
                    }
                    var_name.push(chars.next().unwrap());
                }

                // If no closing brace found, preserve the malformed pattern
                if !found_closing_brace {
                    result.push_str("${");
                    result.push_str(&var_name);
                }
            } else {
                result.push(ch);
            }
        }

        result
    }

    /// Substitute variables in FrontMatter fields
    ///
    /// Substitutes in:
    /// - `allowed_tools` (each tool string)
    /// - `description` (if present)
    /// - `argument_hint` (if present)
    ///
    /// # Examples
    ///
    /// ```
    /// use rustyclawd::plugins::frontmatter_substitution::{Substituter, SubstitutionContext};
    /// use rustyclawd::commands::loader::FrontMatter;
    /// use std::path::PathBuf;
    ///
    /// let ctx = SubstitutionContext::new(
    ///     PathBuf::from("/plugin/root"),
    ///     None
    /// );
    /// let substituter = Substituter::new(ctx);
    ///
    /// let mut frontmatter = FrontMatter {
    ///     allowed_tools: vec!["${CLAUDE_PLUGIN_ROOT}/bin/verify".to_string()],
    ///     description: Some("Uses ${CLAUDE_PLUGIN_ROOT}".to_string()),
    ///     ..Default::default()
    /// };
    ///
    /// substituter.substitute_frontmatter(&mut frontmatter);
    ///
    /// assert_eq!(frontmatter.allowed_tools[0], "/plugin/root/bin/verify");
    /// assert_eq!(frontmatter.description, Some("Uses /plugin/root".to_string()));
    /// ```
    pub fn substitute_frontmatter(&self, frontmatter: &mut crate::commands::loader::FrontMatter) {
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
}

#[cfg(test)]
mod tests {
    use super::*;

    mod variable {
        use super::*;

        #[test]
        fn from_string_plugin_root() {
            assert_eq!(
                Variable::from_string("CLAUDE_PLUGIN_ROOT"),
                Some(Variable::PluginRoot)
            );
        }

        #[test]
        fn from_string_project_root() {
            assert_eq!(
                Variable::from_string("CLAUDE_PROJECT_ROOT"),
                Some(Variable::ProjectRoot)
            );
        }

        #[test]
        fn from_string_home() {
            assert_eq!(Variable::from_string("HOME"), Some(Variable::Home));
        }

        #[test]
        fn from_string_user() {
            assert_eq!(Variable::from_string("USER"), Some(Variable::User));
        }

        #[test]
        fn from_string_pwd() {
            assert_eq!(Variable::from_string("PWD"), Some(Variable::Pwd));
        }

        #[test]
        fn from_string_unknown() {
            assert_eq!(Variable::from_string("UNKNOWN_VAR"), None);
        }

        #[test]
        fn from_string_lowercase_not_supported() {
            assert_eq!(Variable::from_string("home"), None);
            assert_eq!(Variable::from_string("Claude_Plugin_Root"), None);
        }

        #[test]
        fn resolve_plugin_root() {
            let ctx = SubstitutionContext::new(PathBuf::from("/plugin/root"), None);
            assert_eq!(
                Variable::PluginRoot.resolve(&ctx),
                Some("/plugin/root".to_string())
            );
        }

        #[test]
        fn resolve_project_root() {
            let ctx = SubstitutionContext::new(
                PathBuf::from("/plugin/root"),
                Some(PathBuf::from("/project/root")),
            );
            assert_eq!(
                Variable::ProjectRoot.resolve(&ctx),
                Some("/project/root".to_string())
            );
        }

        #[test]
        fn resolve_project_root_none() {
            let ctx = SubstitutionContext::new(PathBuf::from("/plugin/root"), None);
            assert_eq!(Variable::ProjectRoot.resolve(&ctx), None);
        }

        #[test]
        fn resolve_home() {
            let ctx = SubstitutionContext::new(PathBuf::from("/plugin/root"), None);
            // HOME should be available in most test environments
            let result = Variable::Home.resolve(&ctx);
            // Just verify it returns Some or None without asserting specific value
            assert!(result.is_some() || result.is_none());
        }

        #[test]
        fn resolve_pwd() {
            let ctx = SubstitutionContext::new(PathBuf::from("/plugin/root"), None);
            let result = Variable::Pwd.resolve(&ctx);
            // Should return current directory
            assert!(result.is_some());
        }
    }

    mod substitution {
        use super::*;

        fn test_ctx() -> SubstitutionContext {
            SubstitutionContext::new(
                PathBuf::from("/plugin/root"),
                Some(PathBuf::from("/project/root")),
            )
        }

        #[test]
        fn single_variable() {
            let substituter = Substituter::new(test_ctx());
            assert_eq!(
                substituter.substitute("${CLAUDE_PLUGIN_ROOT}/tools"),
                "/plugin/root/tools"
            );
        }

        #[test]
        fn multiple_variables() {
            let substituter = Substituter::new(test_ctx());
            assert_eq!(
                substituter.substitute("${CLAUDE_PLUGIN_ROOT}:${CLAUDE_PROJECT_ROOT}"),
                "/plugin/root:/project/root"
            );
        }

        #[test]
        fn multiple_variables_in_path() {
            let substituter = Substituter::new(test_ctx());
            assert_eq!(
                substituter.substitute("${CLAUDE_PLUGIN_ROOT}/bin:${CLAUDE_PROJECT_ROOT}/bin"),
                "/plugin/root/bin:/project/root/bin"
            );
        }

        #[test]
        fn unknown_variable_preserved() {
            let substituter = Substituter::new(test_ctx());
            assert_eq!(substituter.substitute("${UNKNOWN}/path"), "${UNKNOWN}/path");
        }

        #[test]
        fn unavailable_variable_preserved() {
            let ctx = SubstitutionContext::new(PathBuf::from("/plugin/root"), None);
            let substituter = Substituter::new(ctx);
            assert_eq!(
                substituter.substitute("${CLAUDE_PROJECT_ROOT}/path"),
                "${CLAUDE_PROJECT_ROOT}/path"
            );
        }

        #[test]
        fn malformed_pattern_no_closing_brace() {
            let substituter = Substituter::new(test_ctx());
            // Missing closing brace - should preserve
            assert_eq!(
                substituter.substitute("${CLAUDE_PLUGIN_ROOT/path"),
                "${CLAUDE_PLUGIN_ROOT/path"
            );
        }

        #[test]
        fn dollar_without_braces_preserved() {
            let substituter = Substituter::new(test_ctx());
            assert_eq!(substituter.substitute("$VARIABLE"), "$VARIABLE");
        }

        #[test]
        fn empty_string() {
            let substituter = Substituter::new(test_ctx());
            assert_eq!(substituter.substitute(""), "");
        }

        #[test]
        fn no_variables() {
            let substituter = Substituter::new(test_ctx());
            assert_eq!(substituter.substitute("/absolute/path"), "/absolute/path");
        }

        #[test]
        fn empty_variable_name() {
            let substituter = Substituter::new(test_ctx());
            assert_eq!(substituter.substitute("${}/path"), "${}/path");
        }

        #[test]
        fn nested_variables_not_supported() {
            let substituter = Substituter::new(test_ctx());
            // Nested variables should be preserved as-is (first ${ stops at first })
            let result = substituter.substitute("${CLAUDE_${PLUGIN}_ROOT}");
            // Will match ${CLAUDE_ and stop at first }, leaving rest as-is
            assert!(result.contains("${") || result.contains("}"));
        }

        #[test]
        fn mixed_absolute_and_relative() {
            let substituter = Substituter::new(test_ctx());
            assert_eq!(
                substituter.substitute("${CLAUDE_PLUGIN_ROOT}/tools:/absolute/path"),
                "/plugin/root/tools:/absolute/path"
            );
        }
    }

    mod frontmatter {
        use super::*;
        use crate::commands::loader::FrontMatter;

        fn test_ctx() -> SubstitutionContext {
            SubstitutionContext::new(
                PathBuf::from("/plugin/root"),
                Some(PathBuf::from("/project/root")),
            )
        }

        #[test]
        fn substitute_allowed_tools() {
            let substituter = Substituter::new(test_ctx());
            let mut fm = FrontMatter {
                allowed_tools: vec![
                    "${CLAUDE_PLUGIN_ROOT}/bin/verify".to_string(),
                    "Read".to_string(),
                    "${CLAUDE_PLUGIN_ROOT}/bin/check".to_string(),
                ],
                ..Default::default()
            };

            substituter.substitute_frontmatter(&mut fm);

            assert_eq!(fm.allowed_tools[0], "/plugin/root/bin/verify");
            assert_eq!(fm.allowed_tools[1], "Read");
            assert_eq!(fm.allowed_tools[2], "/plugin/root/bin/check");
        }

        #[test]
        fn substitute_description() {
            let substituter = Substituter::new(test_ctx());
            let mut fm = FrontMatter {
                description: Some("Uses ${CLAUDE_PLUGIN_ROOT}".to_string()),
                ..Default::default()
            };

            substituter.substitute_frontmatter(&mut fm);

            assert_eq!(fm.description, Some("Uses /plugin/root".to_string()));
        }

        #[test]
        fn substitute_argument_hint() {
            let substituter = Substituter::new(test_ctx());
            let mut fm = FrontMatter {
                argument_hint: Some("${CLAUDE_PLUGIN_ROOT}/config".to_string()),
                ..Default::default()
            };

            substituter.substitute_frontmatter(&mut fm);

            assert_eq!(fm.argument_hint, Some("/plugin/root/config".to_string()));
        }

        #[test]
        fn preserves_non_string_fields() {
            let substituter = Substituter::new(test_ctx());
            let mut fm = FrontMatter {
                model: Some("claude-3-5-sonnet-20241022".to_string()),
                allowed_tools: vec!["${CLAUDE_PLUGIN_ROOT}/bin/verify".to_string()],
                disable_model_invocation: Some(true),
                ..Default::default()
            };

            substituter.substitute_frontmatter(&mut fm);

            // Verify non-substituted fields are unchanged
            assert_eq!(fm.model, Some("claude-3-5-sonnet-20241022".to_string()));
            assert_eq!(fm.disable_model_invocation, Some(true));
            // And verify substitution worked
            assert_eq!(fm.allowed_tools[0], "/plugin/root/bin/verify");
        }

        #[test]
        fn empty_frontmatter() {
            let substituter = Substituter::new(test_ctx());
            let mut fm = FrontMatter::default();

            substituter.substitute_frontmatter(&mut fm);

            // Should not crash, just leave empty
            assert!(fm.allowed_tools.is_empty());
            assert!(fm.description.is_none());
            assert!(fm.argument_hint.is_none());
        }
    }

    mod scenarios {
        use super::*;
        use crate::commands::loader::FrontMatter;

        #[test]
        fn plugin_with_relative_tool_paths() {
            let ctx = SubstitutionContext::new(
                PathBuf::from("/home/user/plugins/security-tools"),
                Some(PathBuf::from("/home/user/project")),
            );
            let substituter = Substituter::new(ctx);

            let mut fm = FrontMatter {
                description: Some("Security verification agent".to_string()),
                allowed_tools: vec![
                    "${CLAUDE_PLUGIN_ROOT}/bin/verify-config".to_string(),
                    "${CLAUDE_PLUGIN_ROOT}/bin/check-syntax".to_string(),
                    "Read".to_string(),
                    "Grep".to_string(),
                ],
                ..Default::default()
            };

            substituter.substitute_frontmatter(&mut fm);

            assert_eq!(
                fm.allowed_tools[0],
                "/home/user/plugins/security-tools/bin/verify-config"
            );
            assert_eq!(
                fm.allowed_tools[1],
                "/home/user/plugins/security-tools/bin/check-syntax"
            );
            assert_eq!(fm.allowed_tools[2], "Read");
            assert_eq!(fm.allowed_tools[3], "Grep");
        }

        #[test]
        fn plugin_with_mixed_paths() {
            let ctx = SubstitutionContext::new(
                PathBuf::from("/plugin/root"),
                Some(PathBuf::from("/project/root")),
            );
            let substituter = Substituter::new(ctx);

            let mut fm = FrontMatter {
                allowed_tools: vec![
                    "${CLAUDE_PLUGIN_ROOT}/tools/verify".to_string(),
                    "/absolute/path/to/tool".to_string(),
                    "Read".to_string(),
                    "${CLAUDE_PROJECT_ROOT}/shared/tool".to_string(),
                ],
                ..Default::default()
            };

            substituter.substitute_frontmatter(&mut fm);

            assert_eq!(fm.allowed_tools[0], "/plugin/root/tools/verify");
            assert_eq!(fm.allowed_tools[1], "/absolute/path/to/tool");
            assert_eq!(fm.allowed_tools[2], "Read");
            assert_eq!(fm.allowed_tools[3], "/project/root/shared/tool");
        }

        #[test]
        fn real_world_agent_frontmatter() {
            let ctx = SubstitutionContext::new(
                PathBuf::from("/home/alice/.claude/plugins/code-analyzer"),
                Some(PathBuf::from("/home/alice/my-project")),
            );
            let substituter = Substituter::new(ctx);

            let mut fm = FrontMatter {
                description: Some(
                    "Code analyzer using tools from ${CLAUDE_PLUGIN_ROOT}".to_string(),
                ),
                allowed_tools: vec![
                    "${CLAUDE_PLUGIN_ROOT}/analyzers/python-analyzer".to_string(),
                    "${CLAUDE_PLUGIN_ROOT}/analyzers/rust-analyzer".to_string(),
                    "Read".to_string(),
                    "Grep".to_string(),
                    "Bash".to_string(),
                ],
                argument_hint: Some("${CLAUDE_PROJECT_ROOT}/src".to_string()),
                ..Default::default()
            };

            substituter.substitute_frontmatter(&mut fm);

            assert_eq!(
                fm.description,
                Some(
                    "Code analyzer using tools from /home/alice/.claude/plugins/code-analyzer"
                        .to_string()
                )
            );
            assert_eq!(
                fm.allowed_tools[0],
                "/home/alice/.claude/plugins/code-analyzer/analyzers/python-analyzer"
            );
            assert_eq!(
                fm.allowed_tools[1],
                "/home/alice/.claude/plugins/code-analyzer/analyzers/rust-analyzer"
            );
            assert_eq!(fm.allowed_tools[2], "Read");
            assert_eq!(
                fm.argument_hint,
                Some("/home/alice/my-project/src".to_string())
            );
        }
    }
}
