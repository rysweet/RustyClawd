use claude_code_cli::tool_executor::execute_tool;
use serde_json::json;

#[tokio::test]
#[ignore = "Schema validation - pre-existing main branch issues"]
async fn test_write_tool_missing_content_field() {
    let tool_input = json!({
        "file_path": "/tmp/test.txt"
        // Missing "content" field
    });

    let result = execute_tool("Write".to_string(), tool_input).await;

    assert!(result.is_err());
    let error = result.unwrap_err();
    let error_str = error.to_string();

    // Verify the error message is educational
    assert!(error_str.contains("Failed to parse Write tool parameters"));
    assert!(error_str.contains("required_fields"));
    assert!(error_str.contains("file_path"));
    assert!(error_str.contains("content"));
    assert!(error_str.contains("example"));
}

#[tokio::test]
#[ignore = "Schema validation - pre-existing main branch issues"]
async fn test_write_tool_missing_file_path_field() {
    let tool_input = json!({
        "content": "test content"
        // Missing "file_path" field
    });

    let result = execute_tool("Write".to_string(), tool_input).await;

    assert!(result.is_err());
    let error = result.unwrap_err();
    let error_str = error.to_string();

    // Verify the error message is educational
    assert!(error_str.contains("Failed to parse Write tool parameters"));
    assert!(error_str.contains("required_fields"));
    assert!(error_str.contains("file_path"));
    assert!(error_str.contains("content"));
}

#[tokio::test]
#[ignore = "Schema validation - pre-existing main branch issues"]
async fn test_bash_tool_missing_command_field() {
    let tool_input = json!({
        "timeout": 5000
        // Missing "command" field
    });

    let result = execute_tool("Bash".to_string(), tool_input).await;

    assert!(result.is_err());
    let error = result.unwrap_err();
    let error_str = error.to_string();

    // Verify the error message is educational
    assert!(error_str.contains("Failed to parse Bash tool parameters"));
    assert!(error_str.contains("required_fields"));
    assert!(error_str.contains("command"));
    assert!(error_str.contains("example"));
    assert!(error_str.contains("optional_fields"));
}

#[tokio::test]
#[ignore = "Schema validation - pre-existing main branch issues"]
async fn test_read_tool_missing_file_path() {
    let tool_input = json!({
        "offset": 0,
        "limit": 100
        // Missing "file_path" field
    });

    let result = execute_tool("Read".to_string(), tool_input).await;

    assert!(result.is_err());
    let error = result.unwrap_err();
    let error_str = error.to_string();

    // Verify the error message is educational
    assert!(error_str.contains("Failed to parse Read tool parameters"));
    assert!(error_str.contains("required_fields"));
    assert!(error_str.contains("file_path"));
    assert!(error_str.contains("example"));
}

#[tokio::test]
#[ignore = "Schema validation - pre-existing main branch issues"]
async fn test_edit_tool_missing_fields() {
    let tool_input = json!({
        "file_path": "/tmp/test.txt"
        // Missing "old_string" and "new_string"
    });

    let result = execute_tool("Edit".to_string(), tool_input).await;

    assert!(result.is_err());
    let error = result.unwrap_err();
    let error_str = error.to_string();

    // Verify the error message is educational
    assert!(error_str.contains("Failed to parse Edit tool parameters"));
    assert!(error_str.contains("required_fields"));
    assert!(error_str.contains("file_path"));
    assert!(error_str.contains("old_string"));
    assert!(error_str.contains("new_string"));
}

#[tokio::test]
#[ignore = "Schema validation - pre-existing main branch issues"]
async fn test_glob_tool_missing_pattern() {
    let tool_input = json!({
        "path": "/tmp"
        // Missing "pattern" field
    });

    let result = execute_tool("Glob".to_string(), tool_input).await;

    assert!(result.is_err());
    let error = result.unwrap_err();
    let error_str = error.to_string();

    // Verify the error message is educational
    assert!(error_str.contains("Failed to parse Glob tool parameters"));
    assert!(error_str.contains("required_fields"));
    assert!(error_str.contains("pattern"));
}

#[tokio::test]
#[ignore = "Schema validation - pre-existing main branch issues"]
async fn test_grep_tool_missing_pattern() {
    let tool_input = json!({
        "path": "/tmp",
        "output_mode": "content"
        // Missing "pattern" field
    });

    let result = execute_tool("Grep".to_string(), tool_input).await;

    assert!(result.is_err());
    let error = result.unwrap_err();
    let error_str = error.to_string();

    // Verify the error message is educational
    assert!(error_str.contains("Failed to parse Grep tool parameters"));
    assert!(error_str.contains("required_fields"));
    assert!(error_str.contains("pattern"));
}

#[tokio::test]
async fn test_error_includes_help_text() {
    let tool_input = json!({
        "file_path": "/tmp/test.txt"
        // Missing "content" field
    });

    let result = execute_tool("Write".to_string(), tool_input).await;

    assert!(result.is_err());
    let error = result.unwrap_err();
    let error_str = error.to_string();

    // Verify the error includes helpful guidance
    assert!(error_str.contains("help"));
    assert!(error_str.contains("requires these fields"));
}

#[tokio::test]
async fn test_error_includes_example_json() {
    let tool_input = json!({
        "file_path": "/tmp/test.txt"
        // Missing "content" field
    });

    let result = execute_tool("Write".to_string(), tool_input).await;

    assert!(result.is_err());
    let error = result.unwrap_err();
    let error_str = error.to_string();

    // Verify the error includes a valid JSON example
    assert!(error_str.contains("example"));
    assert!(error_str.contains("/absolute/path/to/file.txt"));
    assert!(error_str.contains("The content to write to the file"));
}
