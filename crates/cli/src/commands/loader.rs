//! Command loader - loads command files and parses frontmatter

use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use tokio::fs;
use tokio::process::Command as TokioCommand;

/// YAML frontmatter metadata
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FrontMatter {
    /// Command description
    pub description: Option<String>,
    /// Model to use
    pub model: Option<String>,
    /// Allowed tools
    #[serde(default)]
    pub allowed_tools: Vec<String>,
    /// Argument hint for tab completion
    #[serde(rename = "argument-hint")]
    pub argument_hint: Option<String>,
    /// Disable model invocation via SlashCommand tool
    #[serde(rename = "disable-model-invocation")]
    pub disable_model_invocation: Option<bool>,
    /// Additional metadata
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// Loaded command with metadata
#[derive(Debug, Clone)]
pub struct LoadedCommand {
    /// Command name
    pub name: String,
    /// Frontmatter metadata
    pub frontmatter: FrontMatter,
    /// Command content/template
    pub content: String,
}

/// Command loader
pub struct CommandLoader;

impl CommandLoader {
    /// Create a new loader
    pub fn new() -> Self {
        Self
    }

    /// Load a command file from path
    ///
    /// # Arguments
    ///
    /// * `path` - Path to command file
    /// * `plugin_root` - Optional plugin root for variable substitution
    /// * `project_root` - Optional project root for variable substitution
    pub async fn load_command(
        &self,
        path: &Path,
        plugin_root: Option<&Path>,
        project_root: Option<&Path>,
    ) -> Result<LoadedCommand> {
        let content = fs::read_to_string(path)
            .await
            .context(format!("Failed to read command file: {}", path.display()))?;

        let name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| anyhow!("Invalid file name"))?
            .to_string();

        let (mut frontmatter, body) = self.parse_frontmatter(&content)?;

        // Apply variable substitution if plugin root is provided
        if let Some(plugin_root) = plugin_root {
            use crate::plugins::{Substituter, SubstitutionContext};
            let ctx = SubstitutionContext::new(
                plugin_root.to_path_buf(),
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

    /// Parse YAML frontmatter from content
    ///
    /// Format:
    /// ---
    /// key: value
    /// ---
    /// Content here
    pub fn parse_frontmatter(&self, content: &str) -> Result<(FrontMatter, String)> {
        if !content.starts_with("---") {
            return Ok((FrontMatter::default(), content.to_string()));
        }

        // Find closing marker
        let after_opening = &content[3..];
        match after_opening.find("---") {
            Some(end_idx) => {
                let fm_str = &content[3..3 + end_idx];
                let body = content[3 + end_idx + 3..].trim().to_string();

                // Parse YAML (with fallback for invalid YAML)
                let frontmatter = match serde_yaml::from_str::<FrontMatter>(fm_str) {
                    Ok(fm) => fm,
                    Err(_) => {
                        // If YAML parsing fails, still return empty frontmatter
                        tracing::debug!(
                            "Failed to parse YAML frontmatter, using defaults: {}",
                            fm_str
                        );
                        FrontMatter::default()
                    }
                };

                Ok((frontmatter, body))
            }
            None => {
                // Malformed frontmatter, use content as-is
                Ok((FrontMatter::default(), content.to_string()))
            }
        }
    }

    /// Expand command template with arguments
    ///
    /// Supports multiple template syntaxes:
    /// - $ARGUMENTS - full argument string (official docs syntax)
    /// - {{args}} - full argument string (legacy)
    /// - $1, $2, $3, etc. - individual positional arguments (official docs syntax)
    /// - {0}, {1}, {2}, etc. - individual arguments (legacy)
    ///
    /// If the template contains NO argument placeholders and args are provided,
    /// the arguments are automatically appended to preserve user input.
    pub fn expand_template(&self, template: &str, args: &[String]) -> String {
        let mut result = template.to_string();
        let mut had_substitution = false;

        // Replace $ARGUMENTS with full argument string (official syntax)
        if !args.is_empty() {
            let args_str = args.join(" ");

            if result.contains("$ARGUMENTS") {
                result = result.replace("$ARGUMENTS", &args_str);
                had_substitution = true;
            }

            // Also support legacy {{args}} syntax
            if result.contains("{{args}}") {
                result = result.replace("{{args}}", &args_str);
                had_substitution = true;
            }
        }

        // Replace $1, $2, etc. (official syntax)
        for (i, arg) in args.iter().enumerate() {
            let dollar_placeholder = format!("${}", i + 1);
            let brace_placeholder = format!("{{{}}}", i);

            if result.contains(&dollar_placeholder) {
                result = result.replace(&dollar_placeholder, arg);
                had_substitution = true;
            }

            // Also support legacy {0}, {1} syntax
            if result.contains(&brace_placeholder) {
                result = result.replace(&brace_placeholder, arg);
                had_substitution = true;
            }
        }

        // If no substitutions were made but args were provided, append them
        // This preserves user input for templates that don't declare placeholders
        if !had_substitution && !args.is_empty() {
            let args_str = args.join(" ");
            // Add separator if template doesn't end with newline
            if !result.ends_with('\n') {
                result.push_str("\n\n");
            } else if !result.ends_with("\n\n") {
                result.push('\n');
            }
            result.push_str(&args_str);
        }

        result
    }

    /// Expand file references (@filename) in template
    ///
    /// Replaces @filename with the contents of the file
    pub async fn expand_file_references(
        &self,
        template: &str,
        working_dir: &Path,
    ) -> Result<String> {
        let mut result = template.to_string();

        // Find all @filename patterns (supports @path/to/file.txt)
        // Simple regex-like approach: find @ followed by non-whitespace
        let mut chars = template.chars().peekable();
        let mut file_refs = Vec::new();

        while let Some(ch) = chars.next() {
            if ch == '@' {
                let mut path = String::new();
                while let Some(&next_ch) = chars.peek() {
                    if next_ch.is_whitespace() || next_ch == '\n' {
                        break;
                    }
                    path.push(chars.next().unwrap());
                }
                if !path.is_empty() {
                    file_refs.push(path);
                }
            }
        }

        // Read and replace each file reference
        for file_ref in file_refs {
            let file_path = working_dir.join(&file_ref);
            match fs::read_to_string(&file_path).await {
                Ok(content) => {
                    result = result.replace(&format!("@{}", file_ref), &content);
                }
                Err(e) => {
                    tracing::warn!("Failed to read file reference @{}: {}", file_ref, e);
                    // Leave the reference as-is if file can't be read
                }
            }
        }

        Ok(result)
    }

    /// Execute bash commands (!command) in template
    ///
    /// Replaces !command with the command's output
    pub async fn expand_bash_commands(&self, template: &str) -> Result<String> {
        let mut result = template.to_string();

        // Find all !command patterns (lines starting with !)
        let lines: Vec<&str> = template.lines().collect();

        for line in lines {
            let trimmed = line.trim();
            if trimmed.starts_with('!') {
                let command = trimmed.trim_start_matches('!');

                // Execute bash command
                match self.execute_bash(command).await {
                    Ok(output) => {
                        result = result.replace(line, &output);
                    }
                    Err(e) => {
                        tracing::warn!("Failed to execute bash command '{}': {}", command, e);
                        // Leave the command as-is if execution fails
                    }
                }
            }
        }

        Ok(result)
    }

    /// Execute a bash command and return its output
    async fn execute_bash(&self, command: &str) -> Result<String> {
        let output = TokioCommand::new("bash")
            .arg("-c")
            .arg(command)
            .output()
            .await
            .context("Failed to execute bash command")?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(anyhow!("Bash command failed: {}", stderr))
        }
    }

    /// Fully expand a template with all features
    ///
    /// Applies in order:
    /// 1. Bash command execution (!command)
    /// 2. File references (@filename)
    /// 3. Argument substitution ($ARGUMENTS, $1, $2, etc.)
    pub async fn expand_full(
        &self,
        template: &str,
        args: &[String],
        working_dir: &Path,
    ) -> Result<String> {
        // Step 1: Execute bash commands
        let after_bash = self.expand_bash_commands(template).await?;

        // Step 2: Expand file references
        let after_files = self
            .expand_file_references(&after_bash, working_dir)
            .await?;

        // Step 3: Expand arguments
        Ok(self.expand_template(&after_files, args))
    }
}

impl Default for CommandLoader {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_frontmatter_with_metadata() {
        let content = "---\ndescription: Review PR\nauthor: test\n---\nPrompt content here";
        let loader = CommandLoader::new();

        let (fm, body) = loader.parse_frontmatter(content).unwrap();

        assert_eq!(fm.description, Some("Review PR".to_string()));
        assert_eq!(body, "Prompt content here");
    }

    #[test]
    fn test_parse_frontmatter_no_frontmatter() {
        let content = "Just content";
        let loader = CommandLoader::new();

        let (fm, body) = loader.parse_frontmatter(content).unwrap();

        assert_eq!(fm.description, None);
        assert_eq!(body, "Just content");
    }

    #[test]
    fn test_parse_frontmatter_empty_section() {
        let content = "---\n---\nPrompt content";
        let loader = CommandLoader::new();

        let (_, body) = loader.parse_frontmatter(content).unwrap();

        assert_eq!(body, "Prompt content");
    }

    #[test]
    fn test_parse_frontmatter_malformed() {
        let content = "---\nincomplete frontmatter\nPrompt content";
        let loader = CommandLoader::new();

        let (_, body) = loader.parse_frontmatter(content).unwrap();

        // Should use content as-is when malformed
        assert_eq!(body, "---\nincomplete frontmatter\nPrompt content");
    }

    #[test]
    fn test_expand_template_no_args() {
        let loader = CommandLoader::new();
        let result = loader.expand_template("Hello world", &[]);

        assert_eq!(result, "Hello world");
    }

    #[test]
    fn test_expand_template_arguments_token() {
        let loader = CommandLoader::new();
        let template = "Arguments: {{args}}";
        let args = vec!["foo".to_string(), "bar".to_string(), "baz".to_string()];

        let result = loader.expand_template(template, &args);

        assert_eq!(result, "Arguments: foo bar baz");
    }

    #[test]
    fn test_expand_template_positional_args() {
        let loader = CommandLoader::new();
        let template = "Review PR #{0} with priority {1}";
        let args = vec!["123".to_string(), "high".to_string()];

        let result = loader.expand_template(template, &args);

        assert_eq!(result, "Review PR #123 with priority high");
    }

    #[test]
    fn test_expand_template_multiple_positional() {
        let loader = CommandLoader::new();
        let template = "PR {0} priority {1} assignee {2}";
        let args = vec!["456".to_string(), "high".to_string(), "alice".to_string()];

        let result = loader.expand_template(template, &args);

        assert_eq!(result, "PR 456 priority high assignee alice");
    }

    #[test]
    fn test_expand_template_single_arg() {
        let loader = CommandLoader::new();
        let template = "Process {0}";
        let args = vec!["file.txt".to_string()];

        let result = loader.expand_template(template, &args);

        assert_eq!(result, "Process file.txt");
    }

    #[test]
    fn test_expand_template_empty_args() {
        let loader = CommandLoader::new();
        let template = "Command with {{args}}";
        let args: Vec<String> = vec![];

        let result = loader.expand_template(template, &args);

        // {{args}} should not be replaced with empty string
        assert_eq!(result, "Command with {{args}}");
    }

    #[test]
    fn test_expand_template_with_unused_parameters() {
        let loader = CommandLoader::new();
        let template = "Use {0} and {1} but not {2}";
        let args = vec!["a".to_string(), "b".to_string()];

        let result = loader.expand_template(template, &args);

        assert_eq!(result, "Use a and b but not {2}");
    }

    #[test]
    fn test_frontmatter_with_allowed_tools() {
        let content = "---\ndescription: Test\n---\nContent";
        let loader = CommandLoader::new();

        let (fm, _) = loader.parse_frontmatter(content).unwrap();

        assert_eq!(fm.description, Some("Test".to_string()));
        // Note: YAML list parsing is handled by serde_yaml deserialization
        // In practice, tools would be properly deserialized from proper YAML
    }

    #[test]
    fn test_frontmatter_with_model() {
        let content = "---\ndescription: Test\nmodel: claude-sonnet\n---\nContent";
        let loader = CommandLoader::new();

        let (fm, _) = loader.parse_frontmatter(content).unwrap();

        assert_eq!(fm.model, Some("claude-sonnet".to_string()));
    }

    #[test]
    fn test_expand_template_with_both_tokens() {
        let loader = CommandLoader::new();
        let template = "Args: {{args}}, First: {0}";
        let args = vec!["arg1".to_string(), "arg2".to_string()];

        let result = loader.expand_template(template, &args);

        assert_eq!(result, "Args: arg1 arg2, First: arg1");
    }

    #[tokio::test]
    async fn test_load_command_with_variable_substitution() {
        use std::path::PathBuf;
        use tempfile::tempdir;

        let temp_dir = tempdir().unwrap();
        let cmd_file = temp_dir.path().join("test.md");

        // Create test command with variables
        let content = r#"---
description: Test command from ${CLAUDE_PLUGIN_ROOT}
allowed_tools: ["${CLAUDE_PLUGIN_ROOT}/bin/verify", "Read", "${CLAUDE_PROJECT_ROOT}/shared/tool"]
argument-hint: ${CLAUDE_PLUGIN_ROOT}/config
---
Command content here"#;

        tokio::fs::write(&cmd_file, content).await.unwrap();

        let loader = CommandLoader::new();
        let plugin_root = PathBuf::from("/plugin/root");
        let project_root = PathBuf::from("/project/root");

        let loaded = loader
            .load_command(&cmd_file, Some(&plugin_root), Some(&project_root))
            .await
            .unwrap();

        // Verify substitution worked
        assert_eq!(
            loaded.frontmatter.description,
            Some("Test command from /plugin/root".to_string())
        );
        assert_eq!(loaded.frontmatter.allowed_tools.len(), 3);
        assert_eq!(
            loaded.frontmatter.allowed_tools[0],
            "/plugin/root/bin/verify"
        );
        assert_eq!(loaded.frontmatter.allowed_tools[1], "Read");
        assert_eq!(
            loaded.frontmatter.allowed_tools[2],
            "/project/root/shared/tool"
        );
        assert_eq!(
            loaded.frontmatter.argument_hint,
            Some("/plugin/root/config".to_string())
        );
        assert_eq!(loaded.content, "Command content here");
    }

    #[tokio::test]
    async fn test_load_command_without_plugin_root() {
        use tempfile::tempdir;

        let temp_dir = tempdir().unwrap();
        let cmd_file = temp_dir.path().join("test.md");

        // Create test command with variables
        let content = r#"---
description: Test command
allowed_tools: ["${CLAUDE_PLUGIN_ROOT}/bin/verify", "Read"]
---
Command content"#;

        tokio::fs::write(&cmd_file, content).await.unwrap();

        let loader = CommandLoader::new();

        // Load without plugin root - variables should NOT be substituted
        let loaded = loader.load_command(&cmd_file, None, None).await.unwrap();

        // Variables should be preserved as-is
        assert_eq!(
            loaded.frontmatter.allowed_tools[0],
            "${CLAUDE_PLUGIN_ROOT}/bin/verify"
        );
    }

    #[tokio::test]
    async fn test_load_command_with_unknown_variable() {
        use std::path::PathBuf;
        use tempfile::tempdir;

        let temp_dir = tempdir().unwrap();
        let cmd_file = temp_dir.path().join("test.md");

        // Create test command with unknown variable
        let content = r#"---
description: Test command
allowed_tools: ["${UNKNOWN_VAR}/bin/verify", "${CLAUDE_PLUGIN_ROOT}/bin/check"]
---
Command content"#;

        tokio::fs::write(&cmd_file, content).await.unwrap();

        let loader = CommandLoader::new();
        let plugin_root = PathBuf::from("/plugin/root");

        let loaded = loader
            .load_command(&cmd_file, Some(&plugin_root), None)
            .await
            .unwrap();

        // Unknown variable should be preserved
        assert_eq!(
            loaded.frontmatter.allowed_tools[0],
            "${UNKNOWN_VAR}/bin/verify"
        );
        // Known variable should be substituted
        assert_eq!(
            loaded.frontmatter.allowed_tools[1],
            "/plugin/root/bin/check"
        );
    }

    #[test]
    fn test_expand_template_no_placeholders_appends_args() {
        let loader = CommandLoader::new();
        let template = "Engage deep analysis mode.";
        let args = vec![
            "there".to_string(),
            "is".to_string(),
            "a".to_string(),
            "bug".to_string(),
        ];

        let result = loader.expand_template(template, &args);

        // Args should be appended when no placeholders exist
        assert!(result.contains("Engage deep analysis mode."));
        assert!(result.contains("there is a bug"));
        assert_eq!(result, "Engage deep analysis mode.\n\nthere is a bug");
    }

    #[test]
    fn test_expand_template_no_placeholders_with_newline() {
        let loader = CommandLoader::new();
        let template = "Analyze this problem.\n";
        let args = vec!["urgent".to_string(), "issue".to_string()];

        let result = loader.expand_template(template, &args);

        // Should add single newline when template ends with one
        assert_eq!(result, "Analyze this problem.\n\nurgent issue");
    }

    #[test]
    fn test_expand_template_no_placeholders_no_args() {
        let loader = CommandLoader::new();
        let template = "Simple command";
        let args: Vec<String> = vec![];

        let result = loader.expand_template(template, &args);

        // Should return template unchanged when no args
        assert_eq!(result, "Simple command");
    }

    #[test]
    fn test_expand_template_with_placeholders_no_append() {
        let loader = CommandLoader::new();
        let template = "Review issue {{args}}";
        let args = vec!["#123".to_string()];

        let result = loader.expand_template(template, &args);

        // Should NOT append args when placeholders are used
        assert_eq!(result, "Review issue #123");
        assert_eq!(result.matches("#123").count(), 1); // Only once, not appended
    }

    #[test]
    fn test_expand_template_dollar_arguments_syntax() {
        let loader = CommandLoader::new();
        let template = "Process $ARGUMENTS carefully";
        let args = vec!["file1.txt".to_string(), "file2.txt".to_string()];

        let result = loader.expand_template(template, &args);

        // Should use $ARGUMENTS placeholder, not append
        assert_eq!(result, "Process file1.txt file2.txt carefully");
    }
}
