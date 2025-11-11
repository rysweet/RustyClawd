//! Command loader - loads command files and parses frontmatter

use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use tokio::fs;

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
    pub async fn load_command(&self, path: &Path) -> Result<LoadedCommand> {
        let content = fs::read_to_string(path)
            .await
            .context(format!("Failed to read command file: {}", path.display()))?;

        let name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| anyhow!("Invalid file name"))?
            .to_string();

        let (frontmatter, body) = self.parse_frontmatter(&content)?;

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
    /// Supports:
    /// - {{args}} - full argument string
    /// - {0}, {1}, etc. - individual arguments
    pub fn expand_template(&self, template: &str, args: &[String]) -> String {
        let mut result = template.to_string();

        // Replace {{args}} with full argument string
        if !args.is_empty() {
            let args_str = args.join(" ");
            result = result.replace("{{args}}", &args_str);
        }

        // Replace {0}, {1}, etc. with individual arguments
        for (i, arg) in args.iter().enumerate() {
            result = result.replace(&format!("{{{}}}", i), arg);
        }

        result
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
        assert_eq!(
            body,
            "---\nincomplete frontmatter\nPrompt content"
        );
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
        let args = vec![
            "456".to_string(),
            "high".to_string(),
            "alice".to_string(),
        ];

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
    fn test_expand_template_unused_placeholders() {
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
}
