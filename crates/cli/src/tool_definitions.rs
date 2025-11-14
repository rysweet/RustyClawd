//! Tool definitions for the Claude API
//!
//! This module converts internal tool implementations to Anthropic API tool definitions.

use rustyclawd_core::client::ToolDefinition;
use serde_json::json;

/// Get all available tool definitions for the API
pub fn get_all_tool_definitions() -> Vec<ToolDefinition> {
    vec![
        bash_tool_definition(),
        bash_output_tool_definition(),
        kill_shell_tool_definition(),
        read_tool_definition(),
        write_tool_definition(),
        edit_tool_definition(),
        glob_tool_definition(),
        grep_tool_definition(),
        ask_user_question_tool_definition(),
        skill_tool_definition(),
        slash_command_tool_definition(),
        task_tool_definition(),
        todowrite_tool_definition(),
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

/// BashOutput tool definition
fn bash_output_tool_definition() -> ToolDefinition {
    ToolDefinition {
        name: "BashOutput".to_string(),
        description: "Retrieve output from a running or completed background bash shell".to_string(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "bash_id": {
                    "type": "string",
                    "description": "The ID of the background shell to retrieve output from"
                },
                "filter": {
                    "type": "string",
                    "description": "Optional regular expression to filter output lines"
                }
            },
            "required": ["bash_id"]
        }),
    }
}

/// KillShell tool definition
fn kill_shell_tool_definition() -> ToolDefinition {
    ToolDefinition {
        name: "KillShell".to_string(),
        description: "Kills a running background bash shell by its ID".to_string(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "shell_id": {
                    "type": "string",
                    "description": "The ID of the background shell to kill"
                }
            },
            "required": ["shell_id"]
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

/// AskUserQuestion tool definition
fn ask_user_question_tool_definition() -> ToolDefinition {
    ToolDefinition {
        name: "AskUserQuestion".to_string(),
        description: "Ask the user questions and collect their answers interactively".to_string(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "questions": {
                    "type": "array",
                    "description": "List of questions to ask the user",
                    "items": {
                        "type": "object",
                        "properties": {
                            "question": {
                                "type": "string",
                                "description": "The question to ask"
                            },
                            "header": {
                                "type": "string",
                                "description": "Short label for the question (max 12 chars)"
                            },
                            "multiSelect": {
                                "type": "boolean",
                                "description": "Allow multiple option selection"
                            },
                            "options": {
                                "type": "array",
                                "description": "Available answer options",
                                "items": {
                                    "type": "object",
                                    "properties": {
                                        "label": {
                                            "type": "string",
                                            "description": "Option label"
                                        },
                                        "description": {
                                            "type": "string",
                                            "description": "Option description"
                                        }
                                    },
                                    "required": ["label", "description"]
                                }
                            }
                        },
                        "required": ["question", "header", "options", "multiSelect"]
                    }
                },
                "answers": {
                    "type": "object",
                    "description": "Optional pre-filled answers"
                }
            },
            "required": ["questions"]
        }),
    }
}

/// Skill tool definition
fn skill_tool_definition() -> ToolDefinition {
    ToolDefinition {
        name: "Skill".to_string(),
        description: "Execute skills from the skill registry".to_string(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "skill": {
                    "type": "string",
                    "description": "Name of the skill (loads from .claude/skills/{skill}.md)"
                }
            },
            "required": ["skill"]
        }),
    }
}

/// SlashCommand tool definition
fn slash_command_tool_definition() -> ToolDefinition {
    ToolDefinition {
        name: "SlashCommand".to_string(),
        description: "Execute slash commands within the main conversation".to_string(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "command": {
                    "type": "string",
                    "description": "The slash command to execute with its arguments (e.g., '/review-pr 123')"
                }
            },
            "required": ["command"]
        }),
    }
}

/// Task tool definition (Agent orchestration)
fn task_tool_definition() -> ToolDefinition {
    ToolDefinition {
        name: "Task".to_string(),
        description: "Invoke specialized sub-agents for complex tasks with context isolation".to_string(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "subagent_type": {
                    "type": "string",
                    "description": "Name of the agent (loads from .claude/agents/{subagent_type}.md)"
                },
                "prompt": {
                    "type": "string",
                    "description": "Full prompt/task for the agent to execute"
                },
                "description": {
                    "type": "string",
                    "description": "Brief 3-5 word description of the task"
                },
                "model": {
                    "type": "string",
                    "description": "Optional model override (haiku, sonnet, opus)"
                },
                "resume": {
                    "type": "string",
                    "description": "Optional agent ID to resume a previous execution"
                }
            },
            "required": ["subagent_type", "prompt", "description"]
        }),
    }
}

/// TodoWrite tool definition
fn todowrite_tool_definition() -> ToolDefinition {
    ToolDefinition {
        name: "TodoWrite".to_string(),
        description: "Manage structured task lists for tracking progress".to_string(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "todos": {
                    "type": "array",
                    "description": "List of tasks to manage",
                    "items": {
                        "type": "object",
                        "properties": {
                            "content": {
                                "type": "string",
                                "description": "Task description (what needs to be done)"
                            },
                            "status": {
                                "type": "string",
                                "enum": ["pending", "inprogress", "completed"],
                                "description": "Current status of the task"
                            },
                            "activeForm": {
                                "type": "string",
                                "description": "Present continuous form for in-progress display"
                            }
                        },
                        "required": ["content", "status", "activeForm"]
                    }
                }
            },
            "required": ["todos"]
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

    #[test]
    fn test_skill_tool_requires_skill() {
        let skill = skill_tool_definition();
        let required = skill.input_schema["required"].as_array().unwrap();
        assert!(
            required.contains(&serde_json::json!("skill")),
            "Skill tool must require 'skill' parameter"
        );
    }

    #[test]
    fn test_task_tool_requires_all_parameters() {
        let task = task_tool_definition();
        let required = task.input_schema["required"].as_array().unwrap();
        assert!(
            required.contains(&serde_json::json!("subagent_type")),
            "Task tool must require 'subagent_type' parameter"
        );
        assert!(
            required.contains(&serde_json::json!("prompt")),
            "Task tool must require 'prompt' parameter"
        );
        assert!(
            required.contains(&serde_json::json!("description")),
            "Task tool must require 'description' parameter"
        );
    }

    #[test]
    fn test_task_tool_has_correct_name() {
        let task = task_tool_definition();
        assert_eq!(task.name, "Task", "Task tool must be named 'Task'");
    }

    #[test]
    fn test_task_tool_has_description() {
        let task = task_tool_definition();
        assert!(!task.description.is_empty(), "Task tool must have a description");
        assert!(task.description.contains("agent"), "Task tool description should mention agents");
    }

    #[test]
    fn test_task_tool_has_optional_model() {
        let task = task_tool_definition();
        let properties = task.input_schema["properties"].as_object().unwrap();
        assert!(
            properties.contains_key("model"),
            "Task tool should have optional 'model' parameter"
        );
        let required = task.input_schema["required"].as_array().unwrap();
        assert!(
            !required.contains(&serde_json::json!("model")),
            "Task tool 'model' parameter should be optional"
        );
    }

    #[test]
    fn test_task_tool_has_optional_resume() {
        let task = task_tool_definition();
        let properties = task.input_schema["properties"].as_object().unwrap();
        assert!(
            properties.contains_key("resume"),
            "Task tool should have optional 'resume' parameter"
        );
        let required = task.input_schema["required"].as_array().unwrap();
        assert!(
            !required.contains(&serde_json::json!("resume")),
            "Task tool 'resume' parameter should be optional"
        );
    }

    #[test]
    fn test_task_tool_properties_have_descriptions() {
        let task = task_tool_definition();
        let properties = task.input_schema["properties"].as_object().unwrap();

        for (key, value) in properties {
            let description = value.get("description");
            assert!(
                description.is_some() && description.unwrap().is_string(),
                "Task tool property '{}' must have a string description",
                key
            );
        }
    }

    #[test]
    fn test_task_tool_schema_structure() {
        let task = task_tool_definition();

        // Check top-level schema structure
        assert_eq!(task.input_schema["type"], "object", "Task tool schema must be an object");
        assert!(task.input_schema.get("properties").is_some(), "Task tool must have properties");
        assert!(task.input_schema.get("required").is_some(), "Task tool must have required array");
    }

    #[test]
    fn test_all_tools_include_task() {
        let tools = get_all_tool_definitions();
        let has_task = tools.iter().any(|t| t.name == "Task");
        assert!(has_task, "get_all_tool_definitions() must include Task tool");
    }

    #[test]
    fn test_all_tools_include_ask_user_question() {
        let tools = get_all_tool_definitions();
        let has_ask = tools.iter().any(|t| t.name == "AskUserQuestion");
        assert!(has_ask, "get_all_tool_definitions() must include AskUserQuestion tool");
    }

    #[test]
    fn test_all_tools_include_skill() {
        let tools = get_all_tool_definitions();
        let has_skill = tools.iter().any(|t| t.name == "Skill");
        assert!(has_skill, "get_all_tool_definitions() must include Skill tool");
    }

    #[test]
    fn test_all_tools_include_slashcommand() {
        let tools = get_all_tool_definitions();
        let has_slashcommand = tools.iter().any(|t| t.name == "SlashCommand");
        assert!(has_slashcommand, "get_all_tool_definitions() must include SlashCommand tool");
    }

    #[test]
    fn test_all_tools_include_todowrite() {
        let tools = get_all_tool_definitions();
        let has_todowrite = tools.iter().any(|t| t.name == "TodoWrite");
        assert!(has_todowrite, "get_all_tool_definitions() must include TodoWrite tool");
    }

    #[test]
    fn test_task_tool_count_in_all_tools() {
        let tools = get_all_tool_definitions();
        let task_count = tools.iter().filter(|t| t.name == "Task").count();
        assert_eq!(task_count, 1, "Task tool should appear exactly once in all tool definitions");
    }

    #[test]
    fn test_task_tool_subagent_type_is_string() {
        let task = task_tool_definition();
        let properties = task.input_schema["properties"].as_object().unwrap();
        let subagent_type = properties.get("subagent_type").unwrap();
        assert_eq!(
            subagent_type["type"], "string",
            "Task tool subagent_type must be a string"
        );
    }

    #[test]
    fn test_task_tool_prompt_is_string() {
        let task = task_tool_definition();
        let properties = task.input_schema["properties"].as_object().unwrap();
        let prompt = properties.get("prompt").unwrap();
        assert_eq!(
            prompt["type"], "string",
            "Task tool prompt must be a string"
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
