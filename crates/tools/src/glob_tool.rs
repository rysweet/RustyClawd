//! Glob tool - Find files by pattern
//!
//! Demonstrates:
//! - Pattern matching with glob
//! - Recursive file system traversal
//! - Streaming results
//! - Sorting by modification time

use crate::{ToolContext, ToolEvent, ToolMetadata, ToolResult, ToolStream};
use async_stream::stream;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Parameters for the Glob tool
#[derive(Debug, Deserialize)]
pub struct GlobParams {
    /// Glob pattern (e.g., "**/*.rs", "src/**/*.ts")
    pub pattern: String,

    /// Optional directory to search in (defaults to cwd)
    #[serde(default)]
    pub path: Option<String>,
}

/// Output from Glob tool
#[derive(Debug, Serialize)]
pub struct GlobOutput {
    /// Matching file paths (sorted by modification time, newest first)
    pub files: Vec<String>,

    /// Number of files found
    pub count: usize,
}

/// The Glob tool
pub struct GlobTool;

#[async_trait]
impl crate::Tool for GlobTool {
    type Params = GlobParams;
    type Output = GlobOutput;

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            name: "Glob",
            description: "Fast file pattern matching supporting glob patterns",
        }
    }

    async fn execute(
        &self,
        params: Self::Params,
        ctx: &ToolContext,
    ) -> ToolResult<ToolStream<Self::Output>> {
        let pattern = params.pattern.clone();
        let search_path = params
            .path
            .clone()
            .map(PathBuf::from)
            .unwrap_or_else(|| ctx.cwd.clone());
        let debug = ctx.debug;
        let cwd = ctx.cwd.clone();

        Ok(Box::pin(stream! {
            yield ToolEvent::Progress {
                step: format!("Searching for: {}", pattern),
                percentage: None,
            };

            // Build full pattern with search path
            let full_pattern = if search_path != cwd {
                format!("{}/{}", search_path.display(), pattern)
            } else {
                pattern.clone()
            };

            if debug {
                tracing::debug!(
                    pattern = %full_pattern,
                    search_path = ?search_path,
                    "Globbing for files"
                );
            }

            // Use glob crate for pattern matching
            let paths = match glob::glob(&full_pattern) {
                Ok(paths) => paths,
                Err(e) => {
                    yield ToolEvent::Error {
                        message: format!("Invalid glob pattern: {}", e),
                    };
                    return;
                }
            };

            // Collect all matching paths
            let mut files: Vec<(PathBuf, std::time::SystemTime)> = Vec::new();

            for entry in paths {
                match entry {
                    Ok(path) => {
                        // Get modification time for sorting
                        let mtime = tokio::fs::metadata(&path)
                            .await
                            .and_then(|m| m.modified())
                            .unwrap_or(std::time::SystemTime::UNIX_EPOCH);

                        files.push((path, mtime));
                    }
                    Err(e) => {
                        // Log error but continue
                        if debug {
                            tracing::warn!("Glob error: {}", e);
                        }
                    }
                }

                // Progress update every 100 files
                if files.len().is_multiple_of(100) {
                    yield ToolEvent::Progress {
                        step: format!("Found {} files...", files.len()),
                        percentage: None,
                    };
                }
            }

            // Sort by modification time (newest first)
            files.sort_by_key(|f| std::cmp::Reverse(f.1));

            // Convert to strings
            let file_paths: Vec<String> = files.iter()
                .map(|(p, _)| p.to_string_lossy().to_string())
                .collect();

            let count = file_paths.len();

            if debug {
                tracing::debug!(
                    files_found = count,
                    "Glob search complete"
                );
            }

            yield ToolEvent::Result(GlobOutput {
                files: file_paths,
                count,
            });
        }))
    }

    fn is_read_only(&self) -> bool {
        true // Glob only searches, doesn't modify
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

    #[tokio::test]
    async fn test_glob_basic_pattern() {
        let temp_dir = TempDir::new().unwrap();

        // Create test files
        std::fs::File::create(temp_dir.path().join("test.rs")).unwrap();
        std::fs::File::create(temp_dir.path().join("foo.txt")).unwrap();
        std::fs::File::create(temp_dir.path().join("bar.rs")).unwrap();

        let tool = GlobTool;
        let params = GlobParams {
            pattern: "*.rs".to_string(),
            path: Some(temp_dir.path().to_str().unwrap().to_string()),
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

        assert_eq!(result.count, 2);
        assert!(result.files.iter().any(|f| f.ends_with("test.rs")));
        assert!(result.files.iter().any(|f| f.ends_with("bar.rs")));
    }

    /// Helper: run glob and extract GlobOutput from the stream.
    async fn run_glob(params: GlobParams) -> GlobOutput {
        let tool = GlobTool;
        let ctx = ToolContext::default();
        let stream = tool.execute(params, &ctx).await.unwrap();
        let events: Vec<_> = stream.collect().await;
        events
            .into_iter()
            .find_map(|e| match e {
                ToolEvent::Result(output) => Some(output),
                _ => None,
            })
            .expect("Expected a Result event from GlobTool")
    }

    #[tokio::test]
    async fn test_glob_recursive_pattern() {
        let temp_dir = TempDir::new().unwrap();

        // Create nested directory structure
        let sub = temp_dir.path().join("sub");
        let subsub = sub.join("deep");
        std::fs::create_dir_all(&subsub).unwrap();

        std::fs::File::create(temp_dir.path().join("top.rs")).unwrap();
        std::fs::File::create(sub.join("mid.rs")).unwrap();
        std::fs::File::create(subsub.join("bottom.rs")).unwrap();
        std::fs::File::create(subsub.join("other.txt")).unwrap();

        let params = GlobParams {
            pattern: "**/*.rs".to_string(),
            path: Some(temp_dir.path().to_str().unwrap().to_string()),
        };

        let result = run_glob(params).await;
        assert_eq!(result.count, 3);
        assert!(result.files.iter().any(|f| f.ends_with("top.rs")));
        assert!(result.files.iter().any(|f| f.ends_with("mid.rs")));
        assert!(result.files.iter().any(|f| f.ends_with("bottom.rs")));
    }

    #[tokio::test]
    async fn test_glob_no_matches() {
        let temp_dir = TempDir::new().unwrap();
        std::fs::File::create(temp_dir.path().join("file.txt")).unwrap();

        let params = GlobParams {
            pattern: "*.zzz_nonexistent".to_string(),
            path: Some(temp_dir.path().to_str().unwrap().to_string()),
        };

        let result = run_glob(params).await;
        assert_eq!(result.count, 0);
        assert!(result.files.is_empty());
    }

    #[tokio::test]
    async fn test_glob_nonexistent_path() {
        let params = GlobParams {
            pattern: "*.rs".to_string(),
            path: Some("/tmp/nonexistent_glob_test_dir_12345".to_string()),
        };

        let result = run_glob(params).await;
        assert_eq!(result.count, 0);
        assert!(result.files.is_empty());
    }

    #[tokio::test]
    async fn test_glob_with_path_parameter() {
        let temp_dir = TempDir::new().unwrap();

        let subdir = temp_dir.path().join("mydir");
        std::fs::create_dir_all(&subdir).unwrap();
        std::fs::File::create(subdir.join("a.py")).unwrap();
        std::fs::File::create(subdir.join("b.py")).unwrap();
        std::fs::File::create(temp_dir.path().join("c.py")).unwrap();

        // Only search inside subdir via path parameter
        let params = GlobParams {
            pattern: "*.py".to_string(),
            path: Some(subdir.to_str().unwrap().to_string()),
        };

        let result = run_glob(params).await;
        assert_eq!(result.count, 2);
        assert!(result.files.iter().all(|f| f.contains("mydir")));
    }

    #[tokio::test]
    async fn test_glob_invalid_pattern() {
        let temp_dir = TempDir::new().unwrap();

        let tool = GlobTool;
        let params = GlobParams {
            pattern: "[".to_string(),
            path: Some(temp_dir.path().to_str().unwrap().to_string()),
        };
        let ctx = ToolContext::default();

        let stream = tool.execute(params, &ctx).await.unwrap();
        let events: Vec<_> = stream.collect().await;

        let has_error = events.iter().any(|e| matches!(e, ToolEvent::Error { .. }));
        assert!(
            has_error,
            "Invalid glob pattern should produce an error event"
        );
    }

    #[tokio::test]
    async fn test_glob_sorted_by_mtime() {
        let temp_dir = TempDir::new().unwrap();

        let old_path = temp_dir.path().join("old.txt");
        std::fs::File::create(&old_path).unwrap();

        std::thread::sleep(std::time::Duration::from_millis(50));

        let new_path = temp_dir.path().join("new.txt");
        std::fs::File::create(&new_path).unwrap();
        std::fs::write(&new_path, "fresh content").unwrap();

        let params = GlobParams {
            pattern: "*.txt".to_string(),
            path: Some(temp_dir.path().to_str().unwrap().to_string()),
        };

        let result = run_glob(params).await;
        assert_eq!(result.count, 2);
        assert!(
            result.files[0].ends_with("new.txt"),
            "Newest file should be first, got order: {:?}",
            result.files
        );
        assert!(
            result.files[1].ends_with("old.txt"),
            "Oldest file should be last, got order: {:?}",
            result.files
        );
    }

    #[test]
    fn test_glob_metadata() {
        let tool = GlobTool;
        let meta = tool.metadata();
        assert_eq!(meta.name, "Glob");
        assert!(
            meta.description.contains("pattern")
                || meta.description.contains("file")
                || meta.description.contains("glob"),
            "Description should mention file/pattern matching: {}",
            meta.description
        );
    }

    #[test]
    fn test_glob_read_only_and_concurrent() {
        let tool = GlobTool;
        assert!(tool.is_read_only(), "Glob should be read-only");
        assert!(
            tool.is_concurrency_safe(),
            "Glob should be concurrency-safe"
        );
    }
}
