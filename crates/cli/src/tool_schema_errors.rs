//! Schema error messages for tool parameter validation
//!
//! Provides educational error responses that teach the LLM the correct
//! parameter schema when a tool call fails deserialization.

use rustyclawd_core::client::ClientError;
use serde_json::json;

/// Create an educational error message that teaches Claude the correct schema
pub(crate) fn create_schema_error(tool_name: &str, error_msg: &str) -> ClientError {
    let (required_fields, optional_fields, example) = match tool_name {
        "Write" => (
            vec!["file_path", "content"],
            vec![],
            json!({
                "file_path": "/absolute/path/to/file.txt",
                "content": "The content to write to the file"
            }),
        ),
        "Read" => (
            vec!["file_path"],
            vec!["offset", "limit"],
            json!({
                "file_path": "/absolute/path/to/file.txt",
                "offset": 0,
                "limit": 100
            }),
        ),
        "Edit" => (
            vec!["file_path", "old_string", "new_string"],
            vec!["replace_all"],
            json!({
                "file_path": "/absolute/path/to/file.txt",
                "old_string": "text to replace",
                "new_string": "replacement text",
                "replace_all": false
            }),
        ),
        "Bash" => (
            vec!["command"],
            vec![
                "timeout",
                "description",
                "run_in_background",
                "dangerouslyDisableSandbox",
            ],
            json!({
                "command": "ls -la",
                "timeout": 120000,
                "description": "List files in directory"
            }),
        ),
        "Glob" => (
            vec!["pattern"],
            vec!["path"],
            json!({
                "pattern": "**/*.rs",
                "path": "/path/to/search"
            }),
        ),
        "Grep" => (
            vec!["pattern"],
            vec![
                "path",
                "output_mode",
                "glob",
                "type",
                "-i",
                "-n",
                "-A",
                "-B",
                "-C",
                "multiline",
                "head_limit",
                "offset",
            ],
            json!({
                "pattern": "search.*pattern",
                "path": "/path/to/search",
                "output_mode": "content"
            }),
        ),
        "BashOutput" => (
            vec!["bash_id"],
            vec!["filter"],
            json!({
                "bash_id": "shell_abc123",
                "filter": "ERROR.*"
            }),
        ),
        "KillShell" => (
            vec!["shell_id"],
            vec![],
            json!({
                "shell_id": "shell_abc123"
            }),
        ),
        "AskUserQuestion" => (
            vec!["questions"],
            vec!["answers"],
            json!({
                "questions": [{
                    "question": "What is your choice?",
                    "header": "choice",
                    "multiSelect": false,
                    "options": [
                        {"label": "Option 1", "description": "First option"},
                        {"label": "Option 2", "description": "Second option"}
                    ]
                }],
                "answers": {}
            }),
        ),
        "Skill" => (
            vec!["skill"],
            vec![],
            json!({
                "skill": "skill-name"
            }),
        ),
        "SlashCommand" => (
            vec!["command"],
            vec![],
            json!({
                "command": "/command-name arg1 arg2"
            }),
        ),
        "Task" => (
            vec!["subagent_type", "prompt", "description"],
            vec!["model", "resume", "run_in_background", "memory_scope"],
            json!({
                "subagent_type": "agent_name",
                "prompt": "Full task description for the agent",
                "description": "Brief task summary",
                "model": "sonnet",
                "run_in_background": false
            }),
        ),
        "AgentOutput" => (
            vec!["agent_id"],
            vec![],
            json!({
                "agent_id": "agent_builder_t1234567890"
            }),
        ),
        "TodoWrite" => (
            vec!["todos"],
            vec![],
            json!({
                "todos": [
                    {
                        "content": "Task description",
                        "status": "pending",
                        "activeForm": "Present continuous form"
                    },
                    {
                        "content": "Another task",
                        "status": "in_progress",
                        "activeForm": "Doing another task"
                    }
                ]
            }),
        ),
        "WebFetch" => (
            vec!["url", "prompt"],
            vec![],
            json!({
                "url": "https://example.com",
                "prompt": "Extract the main content from this page"
            }),
        ),
        "WebSearch" => (
            vec!["query"],
            vec!["allowed_domains", "blocked_domains"],
            json!({
                "query": "Rust programming language",
                "allowed_domains": [],
                "blocked_domains": []
            }),
        ),
        _ => (vec![], vec![], json!({})),
    };

    let error_response = json!({
        "error": format!("Parameter validation failed for {} tool: {}. Required fields: {:?}", tool_name, error_msg, required_fields),
        "details": error_msg,
        "required_fields": required_fields,
        "optional_fields": optional_fields,
        "example": example,
        "help": format!(
            "The {} tool requires these fields: {}. Please ensure all required fields are provided with the correct types.",
            tool_name,
            required_fields.join(", ")
        )
    });

    ClientError::ToolExecution(
        serde_json::to_string_pretty(&error_response).unwrap_or_else(|_| error_msg.to_string()),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_create_schema_error_for_task_tool() {
        let error = create_schema_error("Task", "Missing required field");
        let error_msg = match error {
            ClientError::ToolExecution(msg) => msg,
            _ => panic!("Expected ClientError::ToolExecution"),
        };

        // Parse the error message as JSON
        let error_json: serde_json::Value =
            serde_json::from_str(&error_msg).expect("Error message should be valid JSON");

        // Verify required fields are present
        assert!(error_json.get("error").is_some());
        assert!(error_json.get("required_fields").is_some());
        assert!(error_json.get("optional_fields").is_some());
        assert!(error_json.get("example").is_some());

        // Verify Task-specific required fields
        let required = error_json["required_fields"].as_array().unwrap();
        assert!(required.contains(&json!("subagent_type")));
        assert!(required.contains(&json!("prompt")));
        assert!(required.contains(&json!("description")));
    }

    #[test]
    fn test_task_schema_error_includes_optional_fields() {
        let error = create_schema_error("Task", "Test error");
        let error_msg = match error {
            ClientError::ToolExecution(msg) => msg,
            _ => panic!("Expected ClientError::ToolExecution"),
        };

        let error_json: serde_json::Value = serde_json::from_str(&error_msg).unwrap();
        let optional = error_json["optional_fields"].as_array().unwrap();

        assert!(optional.contains(&json!("model")));
        assert!(optional.contains(&json!("resume")));
    }

    #[test]
    fn test_task_schema_error_includes_example() {
        let error = create_schema_error("Task", "Test error");
        let error_msg = match error {
            ClientError::ToolExecution(msg) => msg,
            _ => panic!("Expected ClientError::ToolExecution"),
        };

        let error_json: serde_json::Value = serde_json::from_str(&error_msg).unwrap();
        let example = &error_json["example"];

        assert!(example.get("subagent_type").is_some());
        assert!(example.get("prompt").is_some());
        assert!(example.get("description").is_some());
        assert_eq!(example["subagent_type"], "agent_name");
    }

    #[test]
    fn test_all_schema_error_tools_include_task() {
        // Verify that Task is handled in create_schema_error
        let error = create_schema_error("Task", "test");
        let error_msg = match error {
            ClientError::ToolExecution(msg) => msg,
            _ => panic!("Expected ClientError::ToolExecution"),
        };

        let error_json: serde_json::Value = serde_json::from_str(&error_msg).unwrap();
        let required = error_json["required_fields"].as_array().unwrap();

        // Should not be empty (default case)
        assert!(
            !required.is_empty(),
            "Task tool should have specific schema error handling"
        );
    }

    #[test]
    fn test_task_schema_error_help_message() {
        let error = create_schema_error("Task", "test");
        let error_msg = match error {
            ClientError::ToolExecution(msg) => msg,
            _ => panic!("Expected ClientError::ToolExecution"),
        };

        let error_json: serde_json::Value = serde_json::from_str(&error_msg).unwrap();
        let help = error_json["help"].as_str().unwrap();

        assert!(
            help.contains("Task"),
            "Help message should mention Task tool"
        );
        assert!(
            help.contains("subagent_type"),
            "Help should list required fields"
        );
        assert!(help.contains("prompt"), "Help should list required fields");
        assert!(
            help.contains("description"),
            "Help should list required fields"
        );
    }
}
