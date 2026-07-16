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
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

/// Output mode for grep results
#[derive(Debug, Deserialize, Clone, Copy)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum OutputMode {
    /// Show matching lines with content
    Content,
    /// Show only file paths with matches
    #[default]
    FilesWithMatches,
    /// Show match counts per file
    Count,
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
        let search_path = params.path.clone().unwrap_or_else(|| ".".to_string());
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
    use std::io::Write;
    use tempfile::TempDir;

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

        let stream = tool.execute(params, &ctx).await.unwrap();
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

    /// Helper: run grep and extract the GrepOutput from the stream.
    /// Returns None if ripgrep is not installed (error event instead of result).
    async fn run_grep(params: GrepParams) -> Option<GrepOutput> {
        let tool = GrepTool;
        let ctx = ToolContext::default();
        let stream = tool.execute(params, &ctx).await.unwrap();
        let events: Vec<_> = stream.collect().await;
        events.into_iter().find_map(|e| match e {
            ToolEvent::Result(output) => Some(output),
            _ => None,
        })
    }

    fn make_params(pattern: &str, path: &str) -> GrepParams {
        GrepParams {
            pattern: pattern.to_string(),
            path: Some(path.to_string()),
            output_mode: OutputMode::FilesWithMatches,
            case_insensitive: false,
            glob: None,
            before_context: None,
            after_context: None,
            head_limit: None,
        }
    }

    #[tokio::test]
    async fn test_grep_case_insensitive() {
        let temp_dir = TempDir::new().unwrap();
        let mut f = std::fs::File::create(temp_dir.path().join("a.txt")).unwrap();
        writeln!(f, "Hello WORLD").unwrap();
        writeln!(f, "hello world").unwrap();
        writeln!(f, "no match here").unwrap();

        let mut params = make_params("hello", temp_dir.path().to_str().unwrap());
        params.output_mode = OutputMode::Content;
        params.case_insensitive = true;

        if let Some(result) = run_grep(params).await {
            // Both "Hello WORLD" and "hello world" should match
            assert_eq!(result.count, 2);
            assert!(result.results.iter().any(|r| r.contains("Hello WORLD")));
            assert!(result.results.iter().any(|r| r.contains("hello world")));
        }
    }

    #[tokio::test]
    async fn test_grep_no_matches() {
        let temp_dir = TempDir::new().unwrap();
        let mut f = std::fs::File::create(temp_dir.path().join("a.txt")).unwrap();
        writeln!(f, "nothing interesting here").unwrap();

        let params = make_params("ZZZZNOTFOUND", temp_dir.path().to_str().unwrap());
        if let Some(result) = run_grep(params).await {
            assert_eq!(result.count, 0);
            assert!(result.results.is_empty());
        }
    }

    #[tokio::test]
    async fn test_grep_content_mode() {
        let temp_dir = TempDir::new().unwrap();
        let mut f = std::fs::File::create(temp_dir.path().join("code.rs")).unwrap();
        writeln!(f, "fn main() {{}}").unwrap();
        writeln!(f, "fn helper() {{}}").unwrap();
        writeln!(f, "let x = 42;").unwrap();

        let mut params = make_params("fn", temp_dir.path().to_str().unwrap());
        params.output_mode = OutputMode::Content;

        if let Some(result) = run_grep(params).await {
            assert_eq!(result.count, 2);
            // Content mode includes line numbers (rg --line-number)
            assert!(result.results.iter().any(|r| r.contains("fn main")));
            assert!(result.results.iter().any(|r| r.contains("fn helper")));
        }
    }

    #[tokio::test]
    async fn test_grep_count_mode() {
        let temp_dir = TempDir::new().unwrap();
        let mut f = std::fs::File::create(temp_dir.path().join("data.txt")).unwrap();
        writeln!(f, "apple").unwrap();
        writeln!(f, "banana").unwrap();
        writeln!(f, "apple pie").unwrap();

        let mut params = make_params("apple", temp_dir.path().to_str().unwrap());
        params.output_mode = OutputMode::Count;

        if let Some(result) = run_grep(params).await {
            // Count mode: rg --count produces "file:N" lines
            assert_eq!(result.count, 1); // one line of output: "data.txt:2"
            assert!(result.results[0].contains("2"));
        }
    }

    #[tokio::test]
    async fn test_grep_with_context() {
        let temp_dir = TempDir::new().unwrap();
        let mut f = std::fs::File::create(temp_dir.path().join("ctx.txt")).unwrap();
        writeln!(f, "line1").unwrap();
        writeln!(f, "line2").unwrap();
        writeln!(f, "MATCH").unwrap();
        writeln!(f, "line4").unwrap();
        writeln!(f, "line5").unwrap();

        let mut params = make_params("MATCH", temp_dir.path().to_str().unwrap());
        params.output_mode = OutputMode::Content;
        params.before_context = Some(1);
        params.after_context = Some(1);

        if let Some(result) = run_grep(params).await {
            // Should include line2 (before), MATCH, line4 (after)
            let joined = result.results.join("\n");
            assert!(joined.contains("line2"));
            assert!(joined.contains("MATCH"));
            assert!(joined.contains("line4"));
        }
    }

    #[tokio::test]
    async fn test_grep_with_head_limit() {
        let temp_dir = TempDir::new().unwrap();
        let mut f = std::fs::File::create(temp_dir.path().join("many.txt")).unwrap();
        for i in 0..50 {
            writeln!(f, "line {}", i).unwrap();
        }

        let mut params = make_params("line", temp_dir.path().to_str().unwrap());
        params.output_mode = OutputMode::Content;
        params.head_limit = Some(5);

        if let Some(result) = run_grep(params).await {
            assert_eq!(result.count, 5);
        }
    }

    #[tokio::test]
    async fn test_grep_with_glob_filter() {
        let temp_dir = TempDir::new().unwrap();
        let mut rs = std::fs::File::create(temp_dir.path().join("code.rs")).unwrap();
        writeln!(rs, "fn main() {{}}").unwrap();
        let mut txt = std::fs::File::create(temp_dir.path().join("notes.txt")).unwrap();
        writeln!(txt, "fn is a keyword").unwrap();

        let mut params = make_params("fn", temp_dir.path().to_str().unwrap());
        params.output_mode = OutputMode::FilesWithMatches;
        params.glob = Some("*.rs".to_string());

        if let Some(result) = run_grep(params).await {
            assert_eq!(result.count, 1);
            assert!(result.results[0].contains("code.rs"));
        }
    }

    #[tokio::test]
    async fn test_grep_nonexistent_path() {
        let params = make_params("anything", "/tmp/nonexistent_grep_test_dir_12345");
        // rg on a nonexistent path produces no results (exit code 2)
        let tool = GrepTool;
        let ctx = ToolContext::default();
        let stream = tool.execute(params, &ctx).await.unwrap();
        let events: Vec<_> = stream.collect().await;

        // Should get a result with 0 matches or an error event — either is acceptable
        let has_result = events.iter().any(|e| matches!(e, ToolEvent::Result(_)));
        let has_error = events.iter().any(|e| matches!(e, ToolEvent::Error { .. }));
        assert!(
            has_result || has_error,
            "Expected either a result or error event for nonexistent path"
        );
        if let Some(result) = events.iter().find_map(|e| match e {
            ToolEvent::Result(o) => Some(o),
            _ => None,
        }) {
            assert_eq!(result.count, 0);
        }
    }

    #[test]
    fn test_grep_metadata() {
        let tool = GrepTool;
        let meta = tool.metadata();
        assert_eq!(meta.name, "Grep");
        assert!(
            meta.description.contains("ripgrep")
                || meta.description.contains("pattern")
                || meta.description.contains("Search"),
            "Description should mention search functionality: {}",
            meta.description
        );
    }

    #[test]
    fn test_grep_read_only_and_concurrent() {
        let tool = GrepTool;
        assert!(tool.is_read_only(), "Grep should be read-only");
        assert!(
            tool.is_concurrency_safe(),
            "Grep should be concurrency-safe"
        );
    }
}
