//! Write tool - Write content to files
//!
//! Demonstrates:
//! - File writing with Tokio
//! - Atomic writes (write to temp, then rename)
//! - Directory creation
//! - Permission handling

use crate::{ToolContext, ToolEvent, ToolMetadata, ToolResult, ToolStream};
use async_stream::stream;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio::fs::{self, File};
use tokio::io::AsyncWriteExt;

/// Parameters for the Write tool
#[derive(Debug, Deserialize)]
pub struct WriteParams {
    /// Absolute path to write to
    pub file_path: String,

    /// Content to write
    pub content: String,
}

/// Output from Write tool
#[derive(Debug, Serialize)]
pub struct WriteOutput {
    /// Path written to
    pub file_path: String,

    /// Bytes written
    pub bytes_written: usize,

    /// Whether file was created (vs overwritten)
    pub created: bool,
}

/// The Write tool
pub struct WriteTool;

#[async_trait]
impl crate::Tool for WriteTool {
    type Params = WriteParams;
    type Output = WriteOutput;

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            name: "Write",
            description: "Writes content to a file (creates or overwrites)",
        }
    }

    async fn execute(
        &self,
        params: Self::Params,
        ctx: &ToolContext,
    ) -> ToolResult<ToolStream<Self::Output>> {
        let path = PathBuf::from(&params.file_path);
        let content = params.content.clone();
        let debug = ctx.debug;

        Ok(Box::pin(stream! {
            yield ToolEvent::Progress {
                step: format!("Writing to: {}", params.file_path),
                percentage: None,
            };

            // Check if file exists
            let file_exists = path.exists();

            // Create parent directory if needed
            if let Some(parent) = path.parent() {
                if !parent.exists() {
                    if let Err(e) = fs::create_dir_all(parent).await {
                        yield ToolEvent::Error {
                            message: format!("Failed to create parent directory: {}", e),
                        };
                        return;
                    }

                    if debug {
                        tracing::debug!("Created parent directory: {:?}", parent);
                    }
                }
            }

            // Write file atomically (write to temp, then rename)
            let temp_path = path.with_extension(format!("tmp_{}", std::process::id()));

            let mut file = match File::create(&temp_path).await {
                Ok(f) => f,
                Err(e) => {
                    yield ToolEvent::Error {
                        message: format!("Failed to create temp file: {}", e),
                    };
                    return;
                }
            };

            let bytes_written = match file.write_all(content.as_bytes()).await {
                Ok(_) => content.len(),
                Err(e) => {
                    yield ToolEvent::Error {
                        message: format!("Failed to write content: {}", e),
                    };
                    return;
                }
            };

            // Flush to ensure it's written
            if let Err(e) = file.flush().await {
                yield ToolEvent::Error {
                    message: format!("Failed to flush file: {}", e),
                };
                return;
            }

            drop(file); // Close file before rename

            // Atomic rename
            if let Err(e) = fs::rename(&temp_path, &path).await {
                yield ToolEvent::Error {
                    message: format!("Failed to rename file: {}", e),
                };
                return;
            }

            if debug {
                tracing::debug!(
                    bytes_written = bytes_written,
                    created = !file_exists,
                    "File written successfully"
                );
            }

            yield ToolEvent::Result(WriteOutput {
                file_path: params.file_path.clone(),
                bytes_written,
                created: !file_exists,
            });
        }))
    }

    fn is_read_only(&self) -> bool {
        false // Writing modifies system state
    }

    fn is_concurrency_safe(&self) -> bool {
        true // Each write is independent (unless writing same file)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Tool;
    use futures::StreamExt;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_write_new_file() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");

        let tool = WriteTool;
        let params = WriteParams {
            file_path: file_path.to_str().unwrap().to_string(),
            content: "Hello, Rust!".to_string(),
        };
        let ctx = ToolContext::default();

        let stream = tool.execute(params, &ctx).await.unwrap();
        let events: Vec<_> = stream.collect().await;

        let result = events
            .iter()
            .find_map(|e| match e {
                ToolEvent::Result(output) => Some(output),
                _ => None,
            })
            .unwrap();

        assert_eq!(result.bytes_written, "Hello, Rust!".len());
        assert!(result.created);

        // Verify file exists and has correct content
        let content = tokio::fs::read_to_string(&file_path).await.unwrap();
        assert_eq!(content, "Hello, Rust!");
    }

    #[tokio::test]
    async fn test_write_overwrite() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("existing.txt");

        // Create existing file
        tokio::fs::write(&file_path, "Old content").await.unwrap();

        let tool = WriteTool;
        let params = WriteParams {
            file_path: file_path.to_str().unwrap().to_string(),
            content: "New content".to_string(),
        };
        let ctx = ToolContext::default();

        let stream = tool.execute(params, &ctx).await.unwrap();
        let events: Vec<_> = stream.collect().await;

        let result = events
            .iter()
            .find_map(|e| match e {
                ToolEvent::Result(output) => Some(output),
                _ => None,
            })
            .unwrap();

        assert!(!result.created); // File already existed
        assert_eq!(result.bytes_written, "New content".len());

        // Verify content was overwritten
        let content = tokio::fs::read_to_string(&file_path).await.unwrap();
        assert_eq!(content, "New content");
    }

    #[tokio::test]
    async fn test_write_creates_parent_dir() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("subdir/nested/test.txt");

        let tool = WriteTool;
        let params = WriteParams {
            file_path: file_path.to_str().unwrap().to_string(),
            content: "Nested file".to_string(),
        };
        let ctx = ToolContext::default();

        let stream = tool.execute(params, &ctx).await.unwrap();
        let events: Vec<_> = stream.collect().await;

        let result = events
            .iter()
            .find_map(|e| match e {
                ToolEvent::Result(output) => Some(output),
                _ => None,
            })
            .unwrap();

        assert!(result.created);
        assert!(file_path.exists());

        let content = tokio::fs::read_to_string(&file_path).await.unwrap();
        assert_eq!(content, "Nested file");
    }

    #[tokio::test]
    async fn test_write_creates_deep_parent_directories() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("a/b/c/deep.txt");
        let params = WriteParams {
            file_path: file_path.to_str().unwrap().to_string(),
            content: "deep content".to_string(),
        };
        let ctx = ToolContext::default();
        let stream = WriteTool.execute(params, &ctx).await.unwrap();
        let output: Vec<_> = stream.collect().await;
        assert!(output.iter().any(|e| matches!(e, ToolEvent::Result(_))));
        let content = tokio::fs::read_to_string(&file_path).await.unwrap();
        assert_eq!(content, "deep content");
    }

    #[tokio::test]
    async fn test_write_empty_content() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("empty.txt");
        let params = WriteParams {
            file_path: file_path.to_str().unwrap().to_string(),
            content: String::new(),
        };
        let ctx = ToolContext::default();
        let stream = WriteTool.execute(params, &ctx).await.unwrap();
        let output: Vec<_> = stream.collect().await;
        assert!(output.iter().any(|e| matches!(e, ToolEvent::Result(_))));
        let content = tokio::fs::read_to_string(&file_path).await.unwrap();
        assert!(content.is_empty());
    }
}
