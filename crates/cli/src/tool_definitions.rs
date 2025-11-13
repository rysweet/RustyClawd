//! Tool definitions for the Claude API
//!
//! This module converts internal tool implementations to Anthropic API tool definitions.

use rustyclawd_core::client::ToolDefinition;
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Verify that all tool definitions have proper "required" fields in their schemas
    #[test]
    fn test_all_tools_have_required_fields() {
        let tools = get_all_tool_definitions();

        assert!(!tools.is_empty(), "Should have at least one tool");

        for tool in tools {
            let schema = &tool.input_schema;

            // Verify schema has "required" field
            let required = schema.get("required");
            assert!(
                required.is_some(),
                "Tool '{}' is missing 'required' field in input_schema",
                tool.name
            );

            // Verify it's an array
            let required_array = required.unwrap().as_array();
            assert!(
                required_array.is_some(),
                "Tool '{}' has 'required' field but it's not an array",
                tool.name
            );

            println!("✓ Tool '{}' has required fields: {:?}", tool.name, required_array.unwrap());
        }
    }

    /// Verify specific required fields for critical tools
    #[test]
    fn test_bash_tool_requires_command() {
        let bash = bash_tool_definition();
        let required = bash.input_schema["required"].as_array().unwrap();
        assert!(
            required.contains(&serde_json::json!("command")),
            "Bash tool must require 'command' parameter"
        );
    }

    #[test]
    fn test_write_tool_requires_file_path_and_content() {
        let write = write_tool_definition();
        let required = write.input_schema["required"].as_array().unwrap();
        assert!(
            required.contains(&serde_json::json!("file_path")),
            "Write tool must require 'file_path' parameter"
        );
        assert!(
            required.contains(&serde_json::json!("content")),
            "Write tool must require 'content' parameter"
        );
    }

    #[test]
    fn test_edit_tool_requires_all_parameters() {
        let edit = edit_tool_definition();
        let required = edit.input_schema["required"].as_array().unwrap();
        assert!(
            required.contains(&serde_json::json!("file_path")),
            "Edit tool must require 'file_path' parameter"
        );
        assert!(
            required.contains(&serde_json::json!("old_string")),
            "Edit tool must require 'old_string' parameter"
        );
        assert!(
            required.contains(&serde_json::json!("new_string")),
            "Edit tool must require 'new_string' parameter"
        );
    }

    #[test]
    fn test_read_tool_requires_file_path() {
        let read = read_tool_definition();
        let required = read.input_schema["required"].as_array().unwrap();
        assert!(
            required.contains(&serde_json::json!("file_path")),
            "Read tool must require 'file_path' parameter"
        );
    }

    #[test]
    fn test_glob_tool_requires_pattern() {
        let glob = glob_tool_definition();
        let required = glob.input_schema["required"].as_array().unwrap();
        assert!(
            required.contains(&serde_json::json!("pattern")),
            "Glob tool must require 'pattern' parameter"
        );
    }

    #[test]
    fn test_grep_tool_requires_pattern() {
        let grep = grep_tool_definition();
        let required = grep.input_schema["required"].as_array().unwrap();
        assert!(
            required.contains(&serde_json::json!("pattern")),
            "Grep tool must require 'pattern' parameter"
        );
    }

    /// Test that schemas serialize correctly for API transmission
    #[test]
    fn test_tool_schemas_serialize_correctly() {
        let tools = get_all_tool_definitions();

        for tool in tools {
            // Serialize to JSON (simulating API request)
            let serialized = serde_json::to_string(&tool).expect(&format!(
                "Tool '{}' should serialize to JSON",
                tool.name
            ));

            // Deserialize back
            let deserialized: ToolDefinition = serde_json::from_str(&serialized).expect(&format!(
                "Tool '{}' should deserialize from JSON",
                tool.name
            ));

            // Verify required fields survive serialization round-trip
            let required = deserialized.input_schema.get("required");
            assert!(
                required.is_some(),
                "Tool '{}' lost 'required' field during serialization",
                tool.name
            );

            println!("✓ Tool '{}' serialization verified", tool.name);
        }
    }
}
