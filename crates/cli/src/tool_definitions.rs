//! Tool definitions for the Claude API
//!
//! This module converts internal tool implementations to Anthropic API tool definitions.

use rustyclawd_core::client::ToolDefinition;
use serde_json::json;

/// Get all available tool definitions for the API.
///
/// When `CLAUDE_CODE_SIMPLE=1` is set, only core tools (Bash, Read, Write, Edit)
/// are returned.
pub fn get_all_tool_definitions() -> Vec<ToolDefinition> {
    if rustyclawd_core::simple_mode::is_active() {
        return get_simple_tool_definitions();
    }

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
        agent_output_tool_definition(),
        todowrite_tool_definition(),
        web_fetch_tool_definition(),
        web_search_tool_definition(),
    ]
}

/// Tool definitions for CLAUDE_CODE_SIMPLE mode (Bash, Read, Write, Edit only).
fn get_simple_tool_definitions() -> Vec<ToolDefinition> {
    vec![
        bash_tool_definition(),
        read_tool_definition(),
        write_tool_definition(),
        edit_tool_definition(),
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
        strict: None,
    }
}

/// BashOutput tool definition
fn bash_output_tool_definition() -> ToolDefinition {
    ToolDefinition {
        name: "BashOutput".to_string(),
        description: "Retrieve output from a running or completed background bash shell"
            .to_string(),
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
        strict: None,
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
        strict: None,
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
                },
                "pages": {
                    "type": "string",
                    "description": "Page range for PDF files (e.g., \"1-5\", \"3\", \"10-20\"). Only applicable to PDF files. Maximum 20 pages per request."
                }
            },
            "required": ["file_path"]
        }),
        strict: None,
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
        strict: None,
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
        strict: None,
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
        strict: None,
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
        strict: None,
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
        strict: None,
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
        strict: None,
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
        strict: None,
    }
}

/// Task tool definition (Agent orchestration)
fn task_tool_definition() -> ToolDefinition {
    ToolDefinition {
        name: "Task".to_string(),
        description: "Invoke specialized sub-agents for complex tasks with context isolation"
            .to_string(),
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
                },
                "run_in_background": {
                    "type": "boolean",
                    "description": "Run agent in background and return immediately with agent ID",
                    "default": false
                },
                "memory_scope": {
                    "type": "string",
                    "description": "Memory scope for agent memory operations (user, project, local). Defaults to agent definition frontmatter or local.",
                    "enum": ["user", "project", "local"]
                }
            },
            "required": ["subagent_type", "prompt", "description"]
        }),
        strict: None,
    }
}

/// AgentOutput tool definition
fn agent_output_tool_definition() -> ToolDefinition {
    ToolDefinition {
        name: "AgentOutput".to_string(),
        description: "Retrieve output from a background agent execution".to_string(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "agentId": {
                    "type": "string",
                    "description": "The agent ID to retrieve results for"
                },
                "block": {
                    "type": "boolean",
                    "description": "Whether to block until results are ready",
                    "default": true
                },
                "wait_up_to": {
                    "type": "number",
                    "description": "Maximum time to wait in seconds",
                    "default": 150,
                    "minimum": 0,
                    "maximum": 300
                }
            },
            "required": ["agentId"]
        }),
        strict: None,
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
        strict: None,
    }
}

/// WebFetch tool definition
fn web_fetch_tool_definition() -> ToolDefinition {
    ToolDefinition {
        name: "WebFetch".to_string(),
        description: "Fetches content from a URL, converts HTML to markdown, and processes with AI"
            .to_string(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "url": {
                    "type": "string",
                    "description": "The URL to fetch content from",
                    "format": "uri"
                },
                "prompt": {
                    "type": "string",
                    "description": "The prompt to run on the fetched content"
                }
            },
            "required": ["url", "prompt"]
        }),
        strict: None,
    }
}

/// WebSearch tool definition
fn web_search_tool_definition() -> ToolDefinition {
    ToolDefinition {
        name: "WebSearch".to_string(),
        description: "Searches the web using Claude's server-side tool and returns ranked results"
            .to_string(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "The search query to use (minimum 2 characters)",
                    "minLength": 2
                },
                "allowed_domains": {
                    "type": "array",
                    "description": "Only include search results from these domains",
                    "items": { "type": "string" }
                },
                "blocked_domains": {
                    "type": "array",
                    "description": "Never include search results from these domains",
                    "items": { "type": "string" }
                }
            },
            "required": ["query"]
        }),
        strict: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Comprehensive structural validation of all tool definitions.
    /// Verifies schema structure, required fields, property descriptions,
    /// and tool naming for every tool returned by get_all_tool_definitions().
    #[test]
    fn test_all_tools_have_valid_schemas() {
        // Ensure CLAUDE_CODE_SIMPLE is not set so we get all tools
        let _prev = std::env::var("CLAUDE_CODE_SIMPLE").ok();
        std::env::remove_var("CLAUDE_CODE_SIMPLE");
        let tools = get_all_tool_definitions();

        // We expect exactly 16 tools
        assert_eq!(tools.len(), 16, "Expected 16 tool definitions");

        // Verify no duplicate names
        let mut seen_names = std::collections::HashSet::new();
        for tool in &tools {
            assert!(
                seen_names.insert(&tool.name),
                "Duplicate tool name: '{}'",
                tool.name
            );
        }

        for tool in &tools {
            // Every tool must have a non-empty name and description
            assert!(!tool.name.is_empty(), "Tool has empty name");
            assert!(
                !tool.description.is_empty(),
                "Tool '{}' has empty description",
                tool.name
            );

            let schema = &tool.input_schema;

            // Schema must be type "object" with properties and required
            assert_eq!(
                schema["type"], "object",
                "Tool '{}' schema type must be 'object'",
                tool.name
            );

            let properties = schema["properties"].as_object().unwrap_or_else(|| {
                panic!("Tool '{}' must have 'properties' object", tool.name);
            });

            let required = schema["required"].as_array().unwrap_or_else(|| {
                panic!("Tool '{}' must have 'required' array", tool.name);
            });

            // Every required field must exist in properties
            for req in required {
                let req_str = req.as_str().unwrap();
                assert!(
                    properties.contains_key(req_str),
                    "Tool '{}' requires '{}' but it's not in properties",
                    tool.name,
                    req_str
                );
            }

            // Every property must have a description
            for (key, value) in properties {
                assert!(
                    value.get("description").and_then(|d| d.as_str()).is_some(),
                    "Tool '{}' property '{}' must have a string description",
                    tool.name,
                    key
                );
            }
        }
    }

    /// Test CLAUDE_CODE_SIMPLE mode returns only 4 core tools.
    #[test]
    fn test_simple_mode_returns_core_tools_only() {
        let tools = get_simple_tool_definitions();
        assert_eq!(tools.len(), 4, "Simple mode should have exactly 4 tools");

        let names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
        assert!(names.contains(&"Bash"));
        assert!(names.contains(&"Read"));
        assert!(names.contains(&"Write"));
        assert!(names.contains(&"Edit"));
        assert!(!names.contains(&"Grep"));
        assert!(!names.contains(&"Task"));
    }

    /// Test that schemas serialize correctly for API transmission
    #[test]
    fn test_tool_schemas_serialize_correctly() {
        let tools = get_all_tool_definitions();

        for tool in tools {
            // Serialize to JSON (simulating API request)
            let serialized = serde_json::to_string(&tool)
                .unwrap_or_else(|_| panic!("Tool '{}' should serialize to JSON", tool.name));

            // Deserialize back
            let deserialized: ToolDefinition = serde_json::from_str(&serialized)
                .unwrap_or_else(|_| panic!("Tool '{}' should deserialize from JSON", tool.name));

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
