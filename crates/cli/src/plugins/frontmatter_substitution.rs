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
#[path = "frontmatter_substitution_tests.rs"]
mod tests;
