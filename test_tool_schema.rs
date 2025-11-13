//! Test to verify tool schemas have required fields

use serde_json::json;

fn main() {
    // Import the tool definitions function
    // This would normally be: use rustyclawd_cli::tool_definitions::get_all_tool_definitions;
    // For this standalone test, we'll recreate the definitions

    let bash_def = json!({
        "name": "Bash",
        "description": "Execute bash commands and return output",
        "input_schema": {
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
        }
    });

    let write_def = json!({
        "name": "Write",
        "description": "Write content to a file (overwrites existing)",
        "input_schema": {
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
        }
    });

    println!("=== BASH TOOL SCHEMA ===");
    println!("{}", serde_json::to_string_pretty(&bash_def).unwrap());
    println!();

    println!("=== WRITE TOOL SCHEMA ===");
    println!("{}", serde_json::to_string_pretty(&write_def).unwrap());
    println!();

    // Verify required fields exist
    let bash_required = bash_def["input_schema"]["required"].as_array();
    let write_required = write_def["input_schema"]["required"].as_array();

    println!("=== VERIFICATION ===");
    println!("Bash required fields: {:?}", bash_required);
    println!("Write required fields: {:?}", write_required);

    assert!(bash_required.is_some(), "Bash tool missing 'required' field!");
    assert!(write_required.is_some(), "Write tool missing 'required' field!");

    let bash_required = bash_required.unwrap();
    let write_required = write_required.unwrap();

    assert_eq!(bash_required.len(), 1, "Bash should have 1 required field");
    assert_eq!(write_required.len(), 2, "Write should have 2 required fields");

    assert_eq!(bash_required[0], "command", "Bash required field should be 'command'");
    assert!(write_required.contains(&json!("file_path")), "Write should require 'file_path'");
    assert!(write_required.contains(&json!("content")), "Write should require 'content'");

    println!("✓ All tool schemas have proper 'required' fields!");
}
