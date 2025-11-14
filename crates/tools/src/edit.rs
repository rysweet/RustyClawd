//! Edit tool - Perform exact string replacements in files
//!
//! This is Claude Code's most sophisticated editing tool.
//! Demonstrates:
//! - Atomic file updates
//! - Exact string matching
//! - Replace all vs single occurrence
//! - Preserving indentation

use crate::{ToolContext, ToolEvent, ToolMetadata, ToolResult, ToolStream};
use async_stream::stream;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio::fs;

/// Parameters for the Edit tool
#[derive(Debug, Deserialize)]
pub struct EditParams {
    /// Absolute path to the file to modify
    pub file_path: String,

    /// The exact text to replace
    pub old_string: String,

    /// The text to replace it with
    pub new_string: String,

    /// Replace all occurrences (default: false)
    #[serde(default)]
    pub replace_all: bool,
}

/// Output from Edit tool
#[derive(Debug, Serialize)]
pub struct EditOutput {
    /// File path that was edited
    pub file_path: String,

    /// Number of replacements made
    pub replacements: usize,

    /// Total lines in file
    pub total_lines: usize,

    /// Bytes changed
    pub bytes_changed: usize,
}

/// The Edit tool
pub struct EditTool;

#[async_trait]
impl crate::Tool for EditTool {
    type Params = EditParams;
    type Output = EditOutput;

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            name: "Edit",
            description: "Performs exact string replacements in files",
        }
    }

    async fn execute(
        &self,
        params: Self::Params,
        ctx: &ToolContext,
    ) -> ToolResult<ToolStream<Self::Output>> {
        let path = PathBuf::from(&params.file_path);
        let old_string = params.old_string.clone();
        let new_string = params.new_string.clone();
        let replace_all = params.replace_all;
        let debug = ctx.debug;

        Ok(Box::pin(stream! {
            yield ToolEvent::Progress {
                step: format!("Reading file: {}", params.file_path),
                percentage: Some(10.0),
            };

            // Read file
            let content = match fs::read_to_string(&path).await {
                Ok(c) => c,
                Err(e) => {
                    yield ToolEvent::Error {
                        message: format!("Failed to read file: {}", e),
                    };
                    return;
                }
            };

            let original_bytes = content.len();
            let total_lines = content.lines().count();

            yield ToolEvent::Progress {
                step: "Searching for old_string...".to_string(),
                percentage: Some(30.0),
            };

            // Count occurrences
            let occurrences = content.matches(&old_string).count();

            if occurrences == 0 {
                yield ToolEvent::Error {
                    message: format!("old_string not found in file: {}", old_string),
                };
                return;
            }

            // Check uniqueness if not replace_all
            if !replace_all && occurrences > 1 {
                yield ToolEvent::Error {
                    message: format!(
                        "old_string appears {} times. Use replace_all: true or provide more context to make it unique",
                        occurrences
                    ),
                };
                return;
            }

            yield ToolEvent::Progress {
                step: format!("Replacing {} occurrence(s)...", occurrences),
                percentage: Some(60.0),
            };

            // Perform replacement
            let new_content = if replace_all {
                content.replace(&old_string, &new_string)
            } else {
                // Replace only first occurrence
                content.replacen(&old_string, &new_string, 1)
            };

            let new_bytes = new_content.len();
            let bytes_changed = if new_bytes > original_bytes {
                new_bytes - original_bytes
            } else {
                original_bytes - new_bytes
            };

            yield ToolEvent::Progress {
                step: "Writing changes...".to_string(),
                percentage: Some(80.0),
            };

            // Write atomically (temp + rename)
            let temp_path = path.with_extension("edit_tmp");

            if let Err(e) = fs::write(&temp_path, &new_content).await {
                yield ToolEvent::Error {
                    message: format!("Failed to write temp file: {}", e),
                };
                return;
            }

            if let Err(e) = fs::rename(&temp_path, &path).await {
                yield ToolEvent::Error {
                    message: format!("Failed to rename temp file: {}", e),
                };
                // Try to clean up temp file
                let _ = fs::remove_file(&temp_path).await;
                return;
            }

            if debug {
                tracing::debug!(
                    replacements = if replace_all { occurrences } else { 1 },
                    bytes_changed = bytes_changed,
                    "Edit completed successfully"
                );
            }

            yield ToolEvent::Result(EditOutput {
                file_path: params.file_path.clone(),
                replacements: if replace_all { occurrences } else { 1 },
                total_lines,
                bytes_changed,
            });
        }))
    }

    fn is_read_only(&self) -> bool {
        false
    }

    fn is_concurrency_safe(&self) -> bool {
        false // Editing same file concurrently would cause conflicts
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Tool;
    use futures::StreamExt;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn test_edit_single_replacement() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "Hello World").unwrap();
        writeln!(temp_file, "This is unique text").unwrap();
        temp_file.flush().unwrap();

        let tool = EditTool;
        let params = EditParams {
            file_path: temp_file.path().to_str().unwrap().to_string(),
            old_string: "Hello World".to_string(), // Unique string
            new_string: "Hello Rust".to_string(),
            replace_all: false,
        };
        let ctx = ToolContext::default();

        let mut stream = tool.execute(params, &ctx).await.unwrap();
        let events: Vec<_> = stream.collect().await;

        let result = events
            .iter()
            .find_map(|e| match e {
                ToolEvent::Result(output) => Some(output),
                _ => None,
            })
            .unwrap();

        assert_eq!(result.replacements, 1);

        // Verify replacement occurred
        let content = tokio::fs::read_to_string(temp_file.path()).await.unwrap();
        assert!(content.contains("Hello Rust"));
        assert!(!content.contains("Hello World"));
    }

    #[tokio::test]
    async fn test_edit_replace_all() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "foo bar foo").unwrap();
        writeln!(temp_file, "baz foo qux").unwrap();
        temp_file.flush().unwrap();

        let tool = EditTool;
        let params = EditParams {
            file_path: temp_file.path().to_str().unwrap().to_string(),
            old_string: "foo".to_string(),
            new_string: "FOO".to_string(),
            replace_all: true,
        };
        let ctx = ToolContext::default();

        let mut stream = tool.execute(params, &ctx).await.unwrap();
        let events: Vec<_> = stream.collect().await;

        let result = events
            .iter()
            .find_map(|e| match e {
                ToolEvent::Result(output) => Some(output),
                _ => None,
            })
            .unwrap();

        assert_eq!(result.replacements, 3);

        // Verify all replaced
        let content = tokio::fs::read_to_string(temp_file.path()).await.unwrap();
        assert!(!content.contains("foo"));
        assert_eq!(content.matches("FOO").count(), 3);
    }

    #[tokio::test]
    async fn test_edit_string_not_found() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "Hello World").unwrap();
        temp_file.flush().unwrap();

        let tool = EditTool;
        let params = EditParams {
            file_path: temp_file.path().to_str().unwrap().to_string(),
            old_string: "NotFound".to_string(),
            new_string: "Replacement".to_string(),
            replace_all: false,
        };
        let ctx = ToolContext::default();

        let mut stream = tool.execute(params, &ctx).await.unwrap();
        let events: Vec<_> = stream.collect().await;

        // Should have error event
        let has_error = events.iter().any(|e| matches!(e, ToolEvent::Error { .. }));
        assert!(has_error);
    }

    #[tokio::test]
    async fn test_edit_non_unique_without_replace_all() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "duplicate text").unwrap();
        writeln!(temp_file, "duplicate text").unwrap();
        temp_file.flush().unwrap();

        let tool = EditTool;
        let params = EditParams {
            file_path: temp_file.path().to_str().unwrap().to_string(),
            old_string: "duplicate".to_string(),
            new_string: "unique".to_string(),
            replace_all: false,
        };
        let ctx = ToolContext::default();

        let mut stream = tool.execute(params, &ctx).await.unwrap();
        let events: Vec<_> = stream.collect().await;

        // Should error on non-unique match
        let has_error = events.iter().any(|e| matches!(e, ToolEvent::Error { .. }));
        assert!(has_error);
    }
}
