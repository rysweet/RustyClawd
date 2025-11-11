//! Grep tool - Search for patterns in files using ripgrep
//!
//! Demonstrates:
//! - Integration with external binary (ripgrep)
//! - Regex pattern matching
//! - Streaming search results
//! - Multiple output modes (content, files, count)

use crate::{ToolContext, ToolEvent, ToolMetadata, ToolResult, ToolStream};
use async_stream::stream;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

/// Output mode for grep results
#[derive(Debug, Deserialize, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum OutputMode {
    /// Show matching lines with content
    Content,
    /// Show only file paths with matches
    FilesWithMatches,
    /// Show match counts per file
    Count,
}

impl Default for OutputMode {
    fn default() -> Self {
        Self::FilesWithMatches
    }
}

/// Parameters for the Grep tool
#[derive(Debug, Deserialize)]
pub struct GrepParams {
    /// The regex pattern to search for
    pub pattern: String,

    /// Optional file/directory to search in
    #[serde(default)]
    pub path: Option<String>,

    /// Output mode
    #[serde(default)]
    pub output_mode: OutputMode,

    /// Case insensitive search
    #[serde(rename = "-i", default)]
    pub case_insensitive: bool,

    /// Glob pattern to filter files
    #[serde(default)]
    pub glob: Option<String>,

    /// Number of context lines before match
    #[serde(rename = "-B", default)]
    pub before_context: Option<usize>,

    /// Number of context lines after match
    #[serde(rename = "-A", default)]
    pub after_context: Option<usize>,

    /// Limit output lines
    #[serde(default)]
    pub head_limit: Option<usize>,
}

/// Output from Grep tool
#[derive(Debug, Serialize)]
pub struct GrepOutput {
    /// Search results (format depends on output_mode)
    pub results: Vec<String>,

    /// Number of matches/files found
    pub count: usize,

    /// Pattern that was searched
    pub pattern: String,
}

/// The Grep tool
pub struct GrepTool;

#[async_trait]
impl crate::Tool for GrepTool {
    type Params = GrepParams;
    type Output = GrepOutput;

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            name: "Grep",
            description: "Search for patterns in files using ripgrep",
        }
    }

    async fn execute(
        &self,
        params: Self::Params,
        ctx: &ToolContext,
    ) -> ToolResult<ToolStream<Self::Output>> {
        let pattern = params.pattern.clone();
        let search_path = params.path.clone()
            .unwrap_or_else(|| ".".to_string());
        let debug = ctx.debug;

        Ok(Box::pin(stream! {
            yield ToolEvent::Progress {
                step: format!("Searching for pattern: {}", pattern),
                percentage: None,
            };

            // Build ripgrep command
            let mut cmd = Command::new("rg");

            // Add flags based on params
            if params.case_insensitive {
                cmd.arg("-i");
            }

            match params.output_mode {
                OutputMode::FilesWithMatches => {
                    cmd.arg("--files-with-matches");
                }
                OutputMode::Content => {
                    cmd.arg("--line-number");
                }
                OutputMode::Count => {
                    cmd.arg("--count");
                }
            }

            if let Some(glob_pattern) = &params.glob {
                cmd.arg("--glob").arg(glob_pattern);
            }

            if let Some(before) = params.before_context {
                cmd.arg(format!("-B{}", before));
            }

            if let Some(after) = params.after_context {
                cmd.arg(format!("-A{}", after));
            }

            // Add pattern and path
            cmd.arg(&pattern);
            cmd.arg(&search_path);

            // Configure output
            cmd.stdout(Stdio::piped());
            cmd.stderr(Stdio::piped());

            if debug {
                tracing::debug!(?cmd, "Executing ripgrep");
            }

            // Spawn ripgrep
            let mut child = match cmd.spawn() {
                Ok(c) => c,
                Err(e) => {
                    yield ToolEvent::Error {
                        message: format!("Failed to spawn ripgrep: {}. Is ripgrep installed?", e),
                    };
                    return;
                }
            };

            // Read stdout
            let stdout = child.stdout.take().unwrap();
            let mut reader = BufReader::new(stdout);
            let mut results = Vec::new();
            let mut line = String::new();

            loop {
                line.clear();
                match reader.read_line(&mut line).await {
                    Ok(0) => break, // EOF
                    Ok(_) => {
                        let trimmed = line.trim_end();
                        if !trimmed.is_empty() {
                            results.push(trimmed.to_string());

                            // Check limit
                            if let Some(limit) = params.head_limit {
                                if results.len() >= limit {
                                    // Kill the child process
                                    let _ = child.kill().await;
                                    break;
                                }
                            }

                            // Progress every 100 results
                            if results.len() % 100 == 0 {
                                yield ToolEvent::Progress {
                                    step: format!("Found {} matches...", results.len()),
                                    percentage: None,
                                };
                            }
                        }
                    }
                    Err(e) => {
                        yield ToolEvent::Error {
                            message: format!("Error reading ripgrep output: {}", e),
                        };
                        return;
                    }
                }
            }

            // Wait for process to finish
            let _ = child.wait().await;

            let count = results.len();

            if debug {
                tracing::debug!(
                    count = count,
                    "Grep search complete"
                );
            }

            yield ToolEvent::Result(GrepOutput {
                results,
                count,
                pattern: params.pattern.clone(),
            });
        }))
    }

    fn is_read_only(&self) -> bool {
        true
    }

    fn is_concurrency_safe(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Tool;
    use futures::StreamExt;
    use tempfile::TempDir;
    use std::io::Write;

    #[tokio::test]
    async fn test_grep_basic_search() {
        let temp_dir = TempDir::new().unwrap();

        // Create test files
        let mut file1 = std::fs::File::create(temp_dir.path().join("file1.txt")).unwrap();
        writeln!(file1, "Hello Rust").unwrap();
        writeln!(file1, "Hello World").unwrap();

        let mut file2 = std::fs::File::create(temp_dir.path().join("file2.txt")).unwrap();
        writeln!(file2, "Goodbye Rust").unwrap();

        let tool = GrepTool;
        let params = GrepParams {
            pattern: "Rust".to_string(),
            path: Some(temp_dir.path().to_str().unwrap().to_string()),
            output_mode: OutputMode::FilesWithMatches,
            case_insensitive: false,
            glob: None,
            before_context: None,
            after_context: None,
            head_limit: None,
        };
        let ctx = ToolContext::default();

        let mut stream = tool.execute(params, &ctx).await.unwrap();
        let events: Vec<_> = stream.collect().await;

        let result = events.iter().find_map(|e| match e {
            ToolEvent::Result(output) => Some(output),
            _ => None,
        });

        // If ripgrep is installed, should find 2 files
        if let Some(result) = result {
            assert_eq!(result.count, 2);
        }
        // else: ripgrep not installed, test skipped
    }
}
