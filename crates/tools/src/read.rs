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

    /// Page range for PDF files (e.g., "1-5", "3", "10-20"). Only for .pdf files. (v2.1.31)
    #[serde(default)]
    pub pages: Option<String>,
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

/// Parsed page range for PDF files
#[derive(Debug, Clone, PartialEq)]
pub struct PageRange {
    pub start: usize,
    pub end: usize,
}

/// Parse a page range string like "1-5", "3", "10-20"
pub fn parse_page_range(pages: &str) -> Result<PageRange, String> {
    let pages = pages.trim();
    if pages.is_empty() {
        return Err("Empty page range".to_string());
    }

    if let Some((start_str, end_str)) = pages.split_once('-') {
        let start: usize = start_str
            .trim()
            .parse()
            .map_err(|_| format!("Invalid start page: '{}'", start_str.trim()))?;
        let end: usize = end_str
            .trim()
            .parse()
            .map_err(|_| format!("Invalid end page: '{}'", end_str.trim()))?;
        if start == 0 || end == 0 {
            return Err("Page numbers must be 1 or greater".to_string());
        }
        if start > end {
            return Err(format!(
                "Start page {} is greater than end page {}",
                start, end
            ));
        }
        if end - start + 1 > 20 {
            return Err("Maximum 20 pages per request".to_string());
        }
        Ok(PageRange { start, end })
    } else {
        let page: usize = pages
            .parse()
            .map_err(|_| format!("Invalid page number: '{}'", pages))?;
        if page == 0 {
            return Err("Page numbers must be 1 or greater".to_string());
        }
        Ok(PageRange {
            start: page,
            end: page,
        })
    }
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

        let pages = params.pages.clone();

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
                    pages = ?pages,
                    "Reading file"
                );
            }

            // Handle PDF files with pages parameter
            if file_path.extension().and_then(|e| e.to_str()) == Some("pdf") {
                if let Some(ref page_range_str) = pages {
                    match parse_page_range(page_range_str) {
                        Ok(range) => {
                            yield ToolEvent::Result(ReadOutput {
                                content: format!(
                                    "[PDF file: {}]\n[Requested pages {}-{} of PDF]\n\
                                     Note: PDF text extraction requires a PDF library.\n\
                                     The pages parameter was parsed successfully (pages {}-{}).",
                                    params.file_path, range.start, range.end,
                                    range.start, range.end
                                ),
                                lines_read: 0,
                                file_path: params.file_path.clone(),
                            });
                            return;
                        }
                        Err(e) => {
                            yield ToolEvent::Error {
                                message: format!("Invalid pages parameter: {}", e),
                            };
                            return;
                        }
                    }
                } else {
                    yield ToolEvent::Result(ReadOutput {
                        content: format!(
                            "[PDF file: {}]\n\
                             Note: For large PDFs, use the 'pages' parameter (e.g., pages: \"1-5\").\n\
                             Maximum 20 pages per request.",
                            params.file_path
                        ),
                        lines_read: 0,
                        file_path: params.file_path.clone(),
                    });
                    return;
                }
            }

            // Warn if pages parameter used on non-PDF file
            if pages.is_some() {
                yield ToolEvent::Error {
                    message: "The 'pages' parameter is only applicable to PDF files (.pdf)".to_string(),
                };
                return;
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
            pages: None,
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
            pages: None,
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
            pages: None,
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
            pages: None,
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
            pages: None,
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
            pages: None,
        };
        let ctx = ToolContext::default();
        let stream = ReadTool.execute(params, &ctx).await.unwrap();
        let output: Vec<_> = stream.collect().await;
        // Empty file should succeed, not error
        assert!(output.iter().any(|e| matches!(e, ToolEvent::Result(_))));
    }

    // Page range parsing tests
    #[test]
    fn test_parse_page_range_single() {
        let range = parse_page_range("3").unwrap();
        assert_eq!(range, PageRange { start: 3, end: 3 });
    }

    #[test]
    fn test_parse_page_range_range() {
        let range = parse_page_range("1-5").unwrap();
        assert_eq!(range, PageRange { start: 1, end: 5 });
    }

    #[test]
    fn test_parse_page_range_with_spaces() {
        let range = parse_page_range(" 10 - 20 ").unwrap();
        assert_eq!(range, PageRange { start: 10, end: 20 });
    }

    #[test]
    fn test_parse_page_range_invalid_start() {
        assert!(parse_page_range("abc-5").is_err());
    }

    #[test]
    fn test_parse_page_range_reversed() {
        assert!(parse_page_range("10-5").is_err());
    }

    #[test]
    fn test_parse_page_range_too_many_pages() {
        assert!(parse_page_range("1-25").is_err());
    }

    #[test]
    fn test_parse_page_range_zero() {
        assert!(parse_page_range("0").is_err());
    }

    #[test]
    fn test_parse_page_range_empty() {
        assert!(parse_page_range("").is_err());
    }

    #[tokio::test]
    async fn test_read_pdf_without_pages() {
        // Create a file with .pdf extension
        let dir = tempfile::tempdir().unwrap();
        let pdf_path = dir.path().join("test.pdf");
        std::fs::write(&pdf_path, b"%PDF-1.4 fake content").unwrap();

        let params = ReadParams {
            file_path: pdf_path.to_str().unwrap().to_string(),
            offset: None,
            limit: None,
            pages: None,
        };
        let ctx = ToolContext::default();
        let stream = ReadTool.execute(params, &ctx).await.unwrap();
        let events: Vec<_> = stream.collect().await;

        let result = events
            .iter()
            .find_map(|e| match e {
                ToolEvent::Result(output) => Some(output),
                _ => None,
            })
            .expect("Should have result for PDF");
        assert!(result.content.contains("PDF file"));
        assert!(result.content.contains("pages"));
    }

    #[tokio::test]
    async fn test_read_pdf_with_valid_pages() {
        let dir = tempfile::tempdir().unwrap();
        let pdf_path = dir.path().join("test.pdf");
        std::fs::write(&pdf_path, b"%PDF-1.4 fake").unwrap();

        let params = ReadParams {
            file_path: pdf_path.to_str().unwrap().to_string(),
            offset: None,
            limit: None,
            pages: Some("1-5".to_string()),
        };
        let ctx = ToolContext::default();
        let stream = ReadTool.execute(params, &ctx).await.unwrap();
        let events: Vec<_> = stream.collect().await;

        let result = events
            .iter()
            .find_map(|e| match e {
                ToolEvent::Result(output) => Some(output),
                _ => None,
            })
            .expect("Should have result");
        assert!(result.content.contains("pages 1-5"));
    }

    #[tokio::test]
    async fn test_read_pages_on_non_pdf_errors() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "text content").unwrap();
        temp_file.flush().unwrap();

        let params = ReadParams {
            file_path: temp_file.path().to_str().unwrap().to_string(),
            offset: None,
            limit: None,
            pages: Some("1-3".to_string()),
        };
        let ctx = ToolContext::default();
        let stream = ReadTool.execute(params, &ctx).await.unwrap();
        let events: Vec<_> = stream.collect().await;

        assert!(events.iter().any(|e| matches!(e, ToolEvent::Error { .. })));
    }
}
