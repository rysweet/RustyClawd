//! Tool definitions for the Claude API
//!
//! This module converts internal tool implementations to Anthropic API tool definitions.

use claude_code_core::client::ToolDefinition;
use serde_json::json;

/// Get all available tool definitions for the API
pub fn get_all_tool_definitions() -> Vec<ToolDefinition> {
    vec![
        bash_tool_definition(),
        read_tool_definition(),
        write_tool_definition(),
        edit_tool_definition(),
        glob_tool_definition(),
        grep_tool_definition(),
    ]
}

/// Bash tool definition
fn bash_tool_definition() -> ToolDefinition {
    ToolDefinition {
        name: "Bash".to_string(),
        description: "Execute bash commands and return output".to_string(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "command": {
                    "type": "string",
                    "description": "The bash command to execute"
                },
                "timeout": {
                    "type": "integer",
                    "description": "Optional timeout in milliseconds (max 600000)",
                    "default": 120000
                },
                "description": {
                    "type": "string",
                    "description": "Optional description of what the command does"
                },
                "run_in_background": {
                    "type": "boolean",
                    "description": "Run the command in the background",
                    "default": false
                }
            },
            "required": ["command"]
        }),
    }
}

/// Read tool definition
fn read_tool_definition() -> ToolDefinition {
    ToolDefinition {
        name: "Read".to_string(),
        description: "Read contents of a file".to_string(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "file_path": {
                    "type": "string",
                    "description": "The absolute path to the file to read"
                },
                "offset": {
                    "type": "integer",
                    "description": "The line number to start reading from"
                },
                "limit": {
                    "type": "integer",
                    "description": "The number of lines to read"
                }
            },
            "required": ["file_path"]
        }),
    }
}

/// Write tool definition
fn write_tool_definition() -> ToolDefinition {
    ToolDefinition {
        name: "Write".to_string(),
        description: "Write content to a file (overwrites existing)".to_string(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "file_path": {
                    "type": "string",
                    "description": "The absolute path to the file to write"
                },
                "content": {
                    "type": "string",
                    "description": "The content to write to the file"
                }
            },
            "required": ["file_path", "content"]
        }),
    }
}

/// Edit tool definition
fn edit_tool_definition() -> ToolDefinition {
    ToolDefinition {
        name: "Edit".to_string(),
        description: "Perform exact string replacements in files".to_string(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "file_path": {
                    "type": "string",
                    "description": "The absolute path to the file to modify"
                },
                "old_string": {
                    "type": "string",
                    "description": "The text to replace"
                },
                "new_string": {
                    "type": "string",
                    "description": "The text to replace it with"
                },
                "replace_all": {
                    "type": "boolean",
                    "description": "Replace all occurrences (default false)",
                    "default": false
                }
            },
            "required": ["file_path", "old_string", "new_string"]
        }),
    }
}

/// Glob tool definition
fn glob_tool_definition() -> ToolDefinition {
    ToolDefinition {
        name: "Glob".to_string(),
        description: "Fast file pattern matching using glob patterns".to_string(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "pattern": {
                    "type": "string",
                    "description": "The glob pattern to match files against"
                },
                "path": {
                    "type": "string",
                    "description": "The directory to search in (defaults to current)"
                }
            },
            "required": ["pattern"]
        }),
    }
}

/// Grep tool definition
fn grep_tool_definition() -> ToolDefinition {
    ToolDefinition {
        name: "Grep".to_string(),
        description: "Search for patterns in files using ripgrep".to_string(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "pattern": {
                    "type": "string",
                    "description": "The regular expression pattern to search for"
                },
                "path": {
                    "type": "string",
                    "description": "File or directory to search in"
                },
                "glob": {
                    "type": "string",
                    "description": "Glob pattern to filter files"
                },
                "type": {
                    "type": "string",
                    "description": "File type to search (js, py, rust, etc.)"
                },
                "output_mode": {
                    "type": "string",
                    "enum": ["content", "files_with_matches", "count"],
                    "description": "Output mode",
                    "default": "files_with_matches"
                },
                "-i": {
                    "type": "boolean",
                    "description": "Case insensitive search"
                },
                "multiline": {
                    "type": "boolean",
                    "description": "Enable multiline mode",
                    "default": false
                }
            },
            "required": ["pattern"]
        }),
    }
}
