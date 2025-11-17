//! Tool formatting module for TUI display
//!
//! Provides human-readable formatting of tool execution with visual indicators.

use serde_json::Value;

/// Get icon for a tool by name
pub fn get_tool_icon(tool_name: &str) -> &'static str {
    match tool_name {
        "Bash" => "🔧",
        "Read" => "📄",
        "Write" => "✏️",
        "Edit" => "✂️",
        "Glob" => "🔍",
        "Grep" => "🔎",
        "AskUserQuestion" => "❓",
        "Task" => "🤖",
        "TodoWrite" => "✅",
        "BashOutput" => "📋",
        "KillShell" => "🛑",
        "Skill" => "🎯",
        "SlashCommand" => "⚡",
        _ => "🔹",
    }
}

/// Format a tool call for display
///
/// Example: "🔧 Using Bash: git status"
pub fn format_tool_call(tool_name: &str, params: &Value) -> String {
    let icon = get_tool_icon(tool_name);
    let description = extract_tool_description(tool_name, params);

    format!("{} Using {}: {}", icon, tool_name, description)
}

/// Format tool parameters for display
///
/// Example: "Running command in /home/user/project..."
pub fn format_tool_params(tool_name: &str, params: &Value) -> String {
    match tool_name {
        "Bash" => {
            if let Some(cmd) = params.get("command").and_then(|v| v.as_str()) {
                format!("Command: {}", truncate(cmd, 80))
            } else {
                "Executing command...".to_string()
            }
        }
        "Read" => {
            let file_path = params
                .get("file_path")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            let offset = params.get("offset").and_then(|v| v.as_u64());
            let limit = params.get("limit").and_then(|v| v.as_u64());

            match (offset, limit) {
                (Some(off), Some(lim)) => {
                    format!(
                        "Reading {} (lines {} to {})",
                        truncate(file_path, 60),
                        off,
                        off + lim
                    )
                }
                (Some(off), None) => {
                    format!("Reading {} (from line {})", truncate(file_path, 60), off)
                }
                (None, Some(lim)) => {
                    format!("Reading {} (first {} lines)", truncate(file_path, 60), lim)
                }
                (None, None) => {
                    format!("Reading {}", truncate(file_path, 70))
                }
            }
        }
        "Write" => {
            let file_path = params
                .get("file_path")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            let content_len = params
                .get("content")
                .and_then(|v| v.as_str())
                .map(|s| s.len())
                .unwrap_or(0);

            format!(
                "Writing {} ({} bytes)",
                truncate(file_path, 60),
                content_len
            )
        }
        "Edit" => {
            let file_path = params
                .get("file_path")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            let old_str_len = params
                .get("old_string")
                .and_then(|v| v.as_str())
                .map(|s| s.len())
                .unwrap_or(0);
            let new_str_len = params
                .get("new_string")
                .and_then(|v| v.as_str())
                .map(|s| s.len())
                .unwrap_or(0);

            format!(
                "Editing {} (replacing {} chars with {})",
                truncate(file_path, 50),
                old_str_len,
                new_str_len
            )
        }
        "Glob" => {
            let pattern = params
                .get("pattern")
                .and_then(|v| v.as_str())
                .unwrap_or("*");
            let path = params.get("path").and_then(|v| v.as_str());

            if let Some(p) = path {
                format!("Searching in {} for {}", truncate(p, 40), pattern)
            } else {
                format!("Searching for {}", pattern)
            }
        }
        "Grep" => {
            let pattern = params
                .get("pattern")
                .and_then(|v| v.as_str())
                .unwrap_or(".*");
            let path = params.get("path").and_then(|v| v.as_str());

            if let Some(p) = path {
                format!(
                    "Searching {} for pattern: {}",
                    truncate(p, 40),
                    truncate(pattern, 30)
                )
            } else {
                format!("Searching for pattern: {}", truncate(pattern, 50))
            }
        }
        "Task" => {
            let agent_type = params
                .get("subagent_type")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            let description = params
                .get("description")
                .and_then(|v| v.as_str())
                .unwrap_or("");

            format!(
                "Running {} agent: {}",
                agent_type,
                truncate(description, 60)
            )
        }
        "AskUserQuestion" => {
            let question_count = params
                .get("questions")
                .and_then(|v| v.as_array())
                .map(|arr| arr.len())
                .unwrap_or(0);

            format!("Asking {} question(s)", question_count)
        }
        "TodoWrite" => {
            let todo_count = params
                .get("todos")
                .and_then(|v| v.as_array())
                .map(|arr| arr.len())
                .unwrap_or(0);

            format!("Updating todo list ({} items)", todo_count)
        }
        "BashOutput" => {
            let bash_id = params
                .get("bash_id")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");

            format!("Reading output from {}", bash_id)
        }
        "KillShell" => {
            let shell_id = params
                .get("shell_id")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");

            format!("Terminating shell {}", shell_id)
        }
        "Skill" => {
            let skill = params
                .get("skill")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");

            format!("Invoking skill: {}", skill)
        }
        "SlashCommand" => {
            let command = params
                .get("command")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");

            format!("Executing: {}", truncate(command, 70))
        }
        _ => "Processing...".to_string(),
    }
}

/// Format a successful tool result
///
/// Example: "✓ Completed (exit code: 0)"
pub fn format_tool_success(tool_name: &str, result: &Value) -> String {
    match tool_name {
        "Bash" => {
            let exit_code = result
                .get("exit_code")
                .and_then(|v| v.as_i64())
                .unwrap_or(0);
            let stdout_len = result
                .get("stdout")
                .and_then(|v| v.as_str())
                .map(|s| s.len())
                .unwrap_or(0);

            format!(
                "✓ Completed (exit code: {}, {} bytes output)",
                exit_code, stdout_len
            )
        }
        "Read" => {
            let content_len = result
                .get("content")
                .and_then(|v| v.as_str())
                .map(|s| s.lines().count())
                .unwrap_or(0);

            format!("✓ Read {} lines", content_len)
        }
        "Write" => "✓ File written successfully".to_string(),
        "Edit" => "✓ File edited successfully".to_string(),
        "Glob" => {
            let files_count = result
                .get("files")
                .and_then(|v| v.as_array())
                .map(|arr| arr.len())
                .or_else(|| {
                    // Also check for "paths" field (alternative naming)
                    result
                        .get("paths")
                        .and_then(|v| v.as_array())
                        .map(|arr| arr.len())
                })
                .unwrap_or(0);

            format!("✓ Found {} file(s)", files_count)
        }
        "Grep" => {
            let matches_count = result
                .get("matches")
                .and_then(|v| v.as_array())
                .map(|arr| arr.len())
                .or_else(|| {
                    // Check for count field if available
                    result
                        .get("count")
                        .and_then(|v| v.as_u64())
                        .map(|n| n as usize)
                })
                .unwrap_or(0);

            format!("✓ Found {} match(es)", matches_count)
        }
        "Task" => "✓ Task completed successfully".to_string(),
        "AskUserQuestion" => "✓ User responded".to_string(),
        "TodoWrite" => "✓ Todo list updated".to_string(),
        "BashOutput" => {
            let output_len = result
                .get("output")
                .and_then(|v| v.as_str())
                .map(|s| s.len())
                .unwrap_or(0);

            format!("✓ Retrieved {} bytes", output_len)
        }
        "KillShell" => "✓ Shell terminated".to_string(),
        "Skill" => "✓ Skill execution completed".to_string(),
        "SlashCommand" => "✓ Command executed".to_string(),
        _ => "✓ Completed".to_string(),
    }
}

/// Format a tool error
///
/// Example: "✗ Failed: command not found"
pub fn format_tool_error(tool_name: &str, error: &str) -> String {
    let icon = get_tool_icon(tool_name);
    let first_line = error.lines().next().unwrap_or(error);

    format!("{} ✗ Failed: {}", icon, truncate(first_line, 80))
}

// Private helper functions

/// Extract a human-readable description from tool parameters
fn extract_tool_description(tool_name: &str, params: &Value) -> String {
    match tool_name {
        "Bash" => params
            .get("command")
            .and_then(|v| v.as_str())
            .map(|s| truncate(s, 50))
            .unwrap_or_else(|| "command".to_string()),
        "Read" => params
            .get("file_path")
            .and_then(|v| v.as_str())
            .map(|s| truncate(s, 50))
            .unwrap_or_else(|| "file".to_string()),
        "Write" => params
            .get("file_path")
            .and_then(|v| v.as_str())
            .map(|s| truncate(s, 50))
            .unwrap_or_else(|| "file".to_string()),
        "Edit" => params
            .get("file_path")
            .and_then(|v| v.as_str())
            .map(|s| truncate(s, 50))
            .unwrap_or_else(|| "file".to_string()),
        "Glob" => params
            .get("pattern")
            .and_then(|v| v.as_str())
            .unwrap_or("*")
            .to_string(),
        "Grep" => params
            .get("pattern")
            .and_then(|v| v.as_str())
            .unwrap_or("pattern")
            .to_string(),
        "Task" => params
            .get("description")
            .and_then(|v| v.as_str())
            .map(|s| truncate(s, 50))
            .unwrap_or_else(|| "task".to_string()),
        "AskUserQuestion" => "interactive question".to_string(),
        "TodoWrite" => "todo list".to_string(),
        "BashOutput" => params
            .get("bash_id")
            .and_then(|v| v.as_str())
            .unwrap_or("shell")
            .to_string(),
        "KillShell" => params
            .get("shell_id")
            .and_then(|v| v.as_str())
            .unwrap_or("shell")
            .to_string(),
        "Skill" => params
            .get("skill")
            .and_then(|v| v.as_str())
            .unwrap_or("skill")
            .to_string(),
        "SlashCommand" => params
            .get("command")
            .and_then(|v| v.as_str())
            .map(|s| truncate(s, 50))
            .unwrap_or_else(|| "command".to_string()),
        _ => tool_name.to_string(),
    }
}

/// Truncate a string to a maximum length, adding ellipsis if needed
fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_get_tool_icon() {
        assert_eq!(get_tool_icon("Bash"), "🔧");
        assert_eq!(get_tool_icon("Read"), "📄");
        assert_eq!(get_tool_icon("Write"), "✏️");
        assert_eq!(get_tool_icon("Unknown"), "🔹");
    }

    #[test]
    fn test_format_tool_call_bash() {
        let params = json!({
            "command": "git status"
        });
        let result = format_tool_call("Bash", &params);
        assert!(result.contains("🔧"));
        assert!(result.contains("Using Bash"));
        assert!(result.contains("git status"));
    }

    #[test]
    fn test_format_tool_params_bash() {
        let params = json!({
            "command": "ls -la"
        });
        let result = format_tool_params("Bash", &params);
        assert!(result.contains("Command"));
        assert!(result.contains("ls -la"));
    }

    #[test]
    fn test_format_tool_params_read_with_offset() {
        let params = json!({
            "file_path": "/path/to/file.txt",
            "offset": 100,
            "limit": 50
        });
        let result = format_tool_params("Read", &params);
        assert!(result.contains("/path/to/file.txt"));
        assert!(result.contains("100"));
        assert!(result.contains("150"));
    }

    #[test]
    fn test_format_tool_success_bash() {
        let result = json!({
            "exit_code": 0,
            "stdout": "output content"
        });
        let formatted = format_tool_success("Bash", &result);
        assert!(formatted.contains("✓"));
        assert!(formatted.contains("exit code: 0"));
    }

    #[test]
    fn test_format_tool_success_glob() {
        let result = json!({
            "files": ["file1.rs", "file2.rs", "file3.rs"]
        });
        let formatted = format_tool_success("Glob", &result);
        assert!(formatted.contains("✓"));
        assert!(formatted.contains("3 file(s)"));
    }

    #[test]
    fn test_format_tool_error() {
        let error = "command not found: invalid\nsome more details";
        let formatted = format_tool_error("Bash", error);
        assert!(formatted.contains("🔧"));
        assert!(formatted.contains("✗"));
        assert!(formatted.contains("command not found"));
        assert!(!formatted.contains("some more details")); // Should only show first line
    }

    #[test]
    fn test_truncate() {
        assert_eq!(truncate("short", 10), "short");
        assert_eq!(truncate("this is a very long string", 10), "this is...");
    }

    #[test]
    fn test_format_tool_params_task() {
        let params = json!({
            "subagent_type": "test_agent",
            "description": "Running a test task",
            "prompt": "Full prompt text"
        });
        let result = format_tool_params("Task", &params);
        assert!(result.contains("test_agent"));
        assert!(result.contains("Running a test task"));
    }

    #[test]
    fn test_format_tool_params_empty() {
        let params = json!({});
        // Should not panic on missing fields
        let _ = format_tool_params("Bash", &params);
        let _ = format_tool_params("Read", &params);
        let _ = format_tool_params("Write", &params);
    }
}
