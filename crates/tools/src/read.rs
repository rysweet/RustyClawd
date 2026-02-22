//! Read tool - Read files from the filesystem
//!
//! This tool demonstrates:
//! - File I/O with Tokio
//! - Line range handling (offset + limit)
//! - Streaming large files
//! - Error handling for missing files

use crate::{ToolContext, ToolEvent, ToolMetadata, ToolResult, ToolStream};
use async_stream::stream;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio::fs::File;
use tokio::io::{AsyncBufReadExt, BufReader};

/// Parameters for the Read tool
#[derive(Debug, Deserialize)]
pub struct ReadParams {
    /// Absolute path to the file to read
    pub file_path: String,

    /// Line number to start reading from (0-indexed)
    #[serde(default)]
    pub offset: Option<usize>,

    /// Number of lines to read
    #[serde(default)]
    pub limit: Option<usize>,
}

/// Output from the Read tool
#[derive(Debug, Serialize)]
pub struct ReadOutput {
    /// File contents (with line numbers in cat -n format)
    pub content: String,

    /// Total lines read
    pub lines_read: usize,

    /// File path that was read
    pub file_path: String,
}

/// The Read tool
pub struct ReadTool;

#[async_trait]
impl crate::Tool for ReadTool {
    type Params = ReadParams;
    type Output = ReadOutput;

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            name: "Read",
            description: "Reads files from the filesystem with optional line range",
        }
    }

    async fn execute(
        &self,
        params: Self::Params,
        ctx: &ToolContext,
    ) -> ToolResult<ToolStream<Self::Output>> {
        let file_path = PathBuf::from(&params.file_path);
        let offset = params.offset.unwrap_or(0);
        let limit = params.limit;
        let debug = ctx.debug;

        Ok(Box::pin(stream! {
            yield ToolEvent::Progress {
                step: format!("Reading file: {}", params.file_path),
                percentage: None,
            };

            if debug {
                tracing::debug!(
                    file_path = %params.file_path,
                    offset = offset,
                    limit = ?limit,
                    "Reading file"
                );
            }

            // Open file
            let file = match File::open(&file_path).await {
                Ok(f) => f,
                Err(e) => {
                    yield ToolEvent::Error {
                        message: format!("Failed to open file {}: {}", params.file_path, e),
                    };
                    return;
                }
            };

            let reader = BufReader::new(file);
            let mut lines = reader.lines();
            let mut content = String::new();
            let mut line_num = 0;
            let mut lines_read = 0;

            // Read lines
            while let Some(line_result) = lines.next_line().await.transpose() {
                let line = match line_result {
                    Ok(l) => l,
                    Err(e) => {
                        yield ToolEvent::Error {
                            message: format!("Error reading line {}: {}", line_num + 1, e),
                        };
                        return;
                    }
                };

                // Skip lines before offset
                if line_num < offset {
                    line_num += 1;
                    continue;
                }

                // Check limit
                if let Some(max_lines) = limit {
                    if lines_read >= max_lines {
                        break;
                    }
                }

                // Format as cat -n (line number + tab + content)
                content.push_str(&format!("{:6}→{}\n", line_num + 1, line));
                lines_read += 1;
                line_num += 1;

                // Progress update every 1000 lines
                if lines_read % 1000 == 0 {
                    yield ToolEvent::Progress {
                        step: format!("Read {} lines...", lines_read),
                        percentage: None,
                    };
                }
            }

            if debug {
                tracing::debug!(
                    lines_read = lines_read,
                    bytes = content.len(),
                    "File read complete"
                );
            }

            yield ToolEvent::Result(ReadOutput {
                content,
                lines_read,
                file_path: params.file_path.clone(),
            });
        }))
    }

    fn is_read_only(&self) -> bool {
        true // Reading doesn't modify system state
    }

    fn is_concurrency_safe(&self) -> bool {
        true // Multiple reads can happen concurrently
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
    async fn test_read_simple_file() {
        // Create temporary file
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "Line 1").unwrap();
        writeln!(temp_file, "Line 2").unwrap();
        writeln!(temp_file, "Line 3").unwrap();
        temp_file.flush().unwrap();

        let tool = ReadTool;
        let params = ReadParams {
            file_path: temp_file.path().to_str().unwrap().to_string(),
            offset: None,
            limit: None,
        };
        let ctx = ToolContext::default();

        let stream = tool.execute(params, &ctx).await.unwrap();
        let events: Vec<_> = stream.collect().await;

        // Find the result event
        let result_event = events
            .iter()
            .find_map(|e| match e {
                ToolEvent::Result(output) => Some(output),
                _ => None,
            })
            .expect("Should have result");

        assert_eq!(result_event.lines_read, 3);
        assert!(result_event.content.contains("Line 1"));
        assert!(result_event.content.contains("Line 2"));
        assert!(result_event.content.contains("Line 3"));
    }

    #[tokio::test]
    async fn test_read_with_offset() {
        let mut temp_file = NamedTempFile::new().unwrap();
        for i in 1..=10 {
            writeln!(temp_file, "Line {}", i).unwrap();
        }
        temp_file.flush().unwrap();

        let tool = ReadTool;
        let params = ReadParams {
            file_path: temp_file.path().to_str().unwrap().to_string(),
            offset: Some(5), // Start at line 5 (0-indexed)
            limit: None,
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

        assert_eq!(result.lines_read, 5); // Lines 5-9 (5 lines)
        assert!(result.content.contains("Line 6")); // Line 5 is 0-indexed
    }

    #[tokio::test]
    async fn test_read_with_limit() {
        let mut temp_file = NamedTempFile::new().unwrap();
        for i in 1..=100 {
            writeln!(temp_file, "Line {}", i).unwrap();
        }
        temp_file.flush().unwrap();

        let tool = ReadTool;
        let params = ReadParams {
            file_path: temp_file.path().to_str().unwrap().to_string(),
            offset: None,
            limit: Some(10),
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

        assert_eq!(result.lines_read, 10);
    }

    #[tokio::test]
    async fn test_read_nonexistent_file() {
        let tool = ReadTool;
        let params = ReadParams {
            file_path: "/nonexistent/file.txt".to_string(),
            offset: None,
            limit: None,
        };
        let ctx = ToolContext::default();

        let stream = tool.execute(params, &ctx).await.unwrap();
        let events: Vec<_> = stream.collect().await;

        // Should have an error event
        let has_error = events.iter().any(|e| matches!(e, ToolEvent::Error { .. }));
        assert!(has_error);
    }

    #[tokio::test]
    async fn test_read_file_not_found() {
        let tool = ReadTool;
        let params = ReadParams {
            file_path: "/tmp/nonexistent_read_test_file_12345.rs".to_string(),
            offset: None,
            limit: None,
        };
        let ctx = ToolContext::default();
        let stream = tool.execute(params, &ctx).await.unwrap();
        let output: Vec<_> = stream.collect().await;
        assert!(output.iter().any(|e| matches!(e, ToolEvent::Error { .. })));
    }

    #[tokio::test]
    async fn test_read_empty_file() {
        let temp_file = NamedTempFile::new().unwrap();
        let params = ReadParams {
            file_path: temp_file.path().to_str().unwrap().to_string(),
            offset: None,
            limit: None,
        };
        let ctx = ToolContext::default();
        let stream = ReadTool.execute(params, &ctx).await.unwrap();
        let output: Vec<_> = stream.collect().await;
        // Empty file should succeed, not error
        assert!(output.iter().any(|e| matches!(e, ToolEvent::Result(_))));
    }
}
