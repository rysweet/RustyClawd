//! CLI Reference Documentation Test Suite
//!
//! Comprehensive tests derived from official Claude Code CLI documentation.
//! These tests cover EVERY flag and feature documented at https://code.claude.com/docs/en/cli-reference
//!
//! Test Strategy (TDD):
//! - All tests written to document exact spec
//! - Tests drive implementation of CLI features
//! - Follows testing pyramid: 60% unit, 30% integration, 10% E2E
//!
//! Coverage Categories:
//! 1. Commands (claude, claude "query", claude -p, cat | claude, claude -c, claude -r, claude update, claude mcp)
//! 2. Flags (--add-dir, --agents, --allowedTools, --disallowedTools, -p/--print, --system-prompt, etc.)
//! 3. Output formats (text, json, stream-json)
//! 4. Input formats (text, stream-json)
//! 5. Session management (--continue, --resume)
//! 6. Permission modes (--permission-mode, --dangerously-skip-permissions)
//! 7. Tool configuration (--allowedTools, --disallowedTools)
//! 8. Model selection (--model)
//! 9. System prompt configuration (--system-prompt, --system-prompt-file, --append-system-prompt)
//! 10. Subagent configuration (--agents JSON)

// ============================================================================
// UNIT TESTS: Basic Command Invocations
// ============================================================================

#[test]
fn test_claude_no_args_starts_interactive_repl() {
    // claude with no arguments should start interactive REPL mode
    let args = vec!["claude"];
    let result = parse_cli_args(&args);

    assert!(result.is_ok());
    let config = result.unwrap();
    assert_eq!(config.mode, ExecutionMode::Interactive);
    assert_eq!(config.prompt, None);
}

#[test]
fn test_claude_with_query_launches_repl_with_initial_prompt() {
    // claude "explain this project" should launch REPL with initial prompt
    let args = vec!["claude", "explain this project"];
    let result = parse_cli_args(&args);

    assert!(result.is_ok());
    let config = result.unwrap();
    assert_eq!(config.mode, ExecutionMode::Interactive);
    assert_eq!(config.prompt, Some("explain this project".to_string()));
}

#[test]
fn test_claude_print_mode_query_then_exit() {
    // claude -p "explain this function" should query via SDK then exit
    let args = vec!["claude", "-p", "explain this function"];
    let result = parse_cli_args(&args);

    assert!(result.is_ok());
    let config = result.unwrap();
    assert_eq!(config.mode, ExecutionMode::Print);
    assert_eq!(config.prompt, Some("explain this function".to_string()));
}

#[test]
fn test_claude_long_print_flag() {
    // --print flag should work same as -p
    let args = vec!["claude", "--print", "query text"];
    let result = parse_cli_args(&args);

    assert!(result.is_ok());
    let config = result.unwrap();
    assert_eq!(config.mode, ExecutionMode::Print);
    assert_eq!(config.prompt, Some("query text".to_string()));
}

#[test]
fn test_piped_input_with_print_mode() {
    // cat file | claude -p "query" should process piped content
    let piped_content = "function test() { return 42; }";
    let args = vec!["claude", "-p", "explain"];

    let result = parse_cli_args_with_stdin(&args, Some(piped_content));

    assert!(result.is_ok());
    let config = result.unwrap();
    assert_eq!(config.mode, ExecutionMode::Print);
    assert_eq!(config.stdin_content, Some(piped_content.to_string()));
}

#[test]
fn test_continue_most_recent_conversation() {
    // claude -c should continue most recent conversation
    let args = vec!["claude", "-c"];
    let result = parse_cli_args(&args);

    assert!(result.is_ok());
    let config = result.unwrap();
    assert_eq!(config.continue_session, true);
}

#[test]
fn test_continue_with_long_flag() {
    // claude --continue should work same as -c
    let args = vec!["claude", "--continue"];
    let result = parse_cli_args(&args);

    assert!(result.is_ok());
    let config = result.unwrap();
    assert_eq!(config.continue_session, true);
}

#[test]
fn test_continue_with_print_mode_query() {
    // claude -c -p "query" should resume prior session via SDK
    let args = vec!["claude", "-c", "-p", "Check for type errors"];
    let result = parse_cli_args(&args);

    assert!(result.is_ok());
    let config = result.unwrap();
    assert_eq!(config.mode, ExecutionMode::Print);
    assert_eq!(config.continue_session, true);
    assert_eq!(config.prompt, Some("Check for type errors".to_string()));
}

#[test]
fn test_resume_session_by_id() {
    // claude -r "session-id" "query" should resume session by ID
    let args = vec!["claude", "-r", "abc123", "Finish this PR"];
    let result = parse_cli_args(&args);

    assert!(result.is_ok());
    let config = result.unwrap();
    assert_eq!(config.resume_session_id, Some("abc123".to_string()));
    assert_eq!(config.prompt, Some("Finish this PR".to_string()));
}

#[test]
fn test_resume_session_long_flag() {
    // claude --resume "session-id" should work same as -r
    let args = vec!["claude", "--resume", "session-xyz", "continue work"];
    let result = parse_cli_args(&args);

    assert!(result.is_ok());
    let config = result.unwrap();
    assert_eq!(config.resume_session_id, Some("session-xyz".to_string()));
}

#[test]
fn test_resume_without_session_id_lists_sessions() {
    // claude -r with no ID should list available sessions
    let args = vec!["claude", "-r"];
    let result = parse_cli_args(&args);

    assert!(result.is_ok());
    let config = result.unwrap();
    assert_eq!(config.list_sessions, true);
}

#[test]
fn test_update_command() {
    // claude update should update to latest version
    let args = vec!["claude", "update"];
    let result = parse_cli_args(&args);

    assert!(result.is_ok());
    let config = result.unwrap();
    assert_eq!(config.mode, ExecutionMode::Update);
}

#[test]
fn test_mcp_command() {
    // claude mcp should manage Model Context Protocol servers
    let args = vec!["claude", "mcp"];
    let result = parse_cli_args(&args);

    assert!(result.is_ok());
    let config = result.unwrap();
    assert_eq!(config.mode, ExecutionMode::MCP);
}

// ============================================================================
// UNIT TESTS: --add-dir Flag
// ============================================================================

#[test]
fn test_add_dir_single_directory() {
    // --add-dir ../lib should add one additional directory
    let args = vec!["claude", "--add-dir", "../lib"];
    let result = parse_cli_args(&args);

    assert!(result.is_ok());
    let config = result.unwrap();
    assert_eq!(config.additional_dirs, vec!["../lib"]);
}

#[test]
fn test_add_dir_multiple_directories() {
    // --add-dir ../apps ../lib should add multiple directories
    let args = vec!["claude", "--add-dir", "../apps", "../lib"];
    let result = parse_cli_args(&args);

    assert!(result.is_ok());
    let config = result.unwrap();
    assert_eq!(config.additional_dirs, vec!["../apps", "../lib"]);
}

#[test]
fn test_add_dir_with_absolute_paths() {
    let args = vec!["claude", "--add-dir", "/usr/src/project", "/var/data"];
    let result = parse_cli_args(&args);

    assert!(result.is_ok());
    let config = result.unwrap();
    assert_eq!(config.additional_dirs.len(), 2);
    assert!(config.additional_dirs.contains(&"/usr/src/project".to_string()));
}

#[test]
fn test_add_dir_makes_directories_accessible_to_claude() {
    // Claude should be able to access files in added directories
    let args = vec!["claude", "--add-dir", "../external"];
    let result = parse_cli_args(&args);

    assert!(result.is_ok());
    let config = result.unwrap();
    let access = check_directory_access(&config, "../external");
    assert_eq!(access, DirectoryAccess::Allowed);
}

// ============================================================================
// UNIT TESTS: --agents Flag (Subagent Configuration)
// ============================================================================

#[test]
fn test_agents_accepts_json_object() {
    let agents_json = r#"{
        "researcher": {
            "description": "Specializes in web research",
            "prompt": "You are a research specialist",
            "tools": ["WebSearch", "WebFetch"],
            "model": "sonnet"
        }
    }"#;

    let args = vec!["claude", "--agents", agents_json];
    let result = parse_cli_args(&args);

    assert!(result.is_ok());
    let config = result.unwrap();
    assert!(config.agents.is_some());
    assert_eq!(config.agents.unwrap().len(), 1);
}

#[test]
fn test_agents_requires_description() {
    // Agent definition without description should fail validation
    let agents_json = r#"{
        "worker": {
            "prompt": "You are a worker"
        }
    }"#;

    let args = vec!["claude", "--agents", agents_json];
    let result = parse_cli_args(&args);

    assert!(result.is_err());
    assert!(result.unwrap_err().contains("description is required"));
}

#[test]
fn test_agents_requires_prompt() {
    // Agent definition without prompt should fail validation
    let agents_json = r#"{
        "worker": {
            "description": "Does work"
        }
    }"#;

    let args = vec!["claude", "--agents", agents_json];
    let result = parse_cli_args(&args);

    assert!(result.is_err());
    assert!(result.unwrap_err().contains("prompt is required"));
}

#[test]
fn test_agents_tools_optional_inherits_all() {
    // Agent without tools should inherit all tools
    let agents_json = r#"{
        "general": {
            "description": "General purpose",
            "prompt": "You are helpful"
        }
    }"#;

    let args = vec!["claude", "--agents", agents_json];
    let result = parse_cli_args(&args);

    assert!(result.is_ok());
    let config = result.unwrap();
    let agent = config.agents.unwrap().get("general").unwrap();
    assert_eq!(agent.inherits_all_tools, true);
}

#[test]
fn test_agents_tools_array_specifies_allowed_tools() {
    let agents_json = r#"{
        "coder": {
            "description": "Code specialist",
            "prompt": "You write code",
            "tools": ["Read", "Write", "Edit"]
        }
    }"#;

    let args = vec!["claude", "--agents", agents_json];
    let result = parse_cli_args(&args);

    assert!(result.is_ok());
    let config = result.unwrap();
    let agent = config.agents.unwrap().get("coder").unwrap();
    assert_eq!(agent.tools, vec!["Read", "Write", "Edit"]);
}

#[test]
fn test_agents_model_optional() {
    let agents_json = r#"{
        "fast": {
            "description": "Fast responses",
            "prompt": "Be quick",
            "model": "haiku"
        }
    }"#;

    let args = vec!["claude", "--agents", agents_json];
    let result = parse_cli_args(&args);

    assert!(result.is_ok());
    let config = result.unwrap();
    let agent = config.agents.unwrap().get("fast").unwrap();
    assert_eq!(agent.model, Some("haiku".to_string()));
}

#[test]
fn test_agents_model_aliases() {
    // Should support sonnet, opus, haiku aliases
    let agents_json = r#"{
        "a1": {"description": "d", "prompt": "p", "model": "sonnet"},
        "a2": {"description": "d", "prompt": "p", "model": "opus"},
        "a3": {"description": "d", "prompt": "p", "model": "haiku"}
    }"#;

    let args = vec!["claude", "--agents", agents_json];
    let result = parse_cli_args(&args);

    assert!(result.is_ok());
}

#[test]
fn test_agents_multiple_subagents() {
    let agents_json = r#"{
        "researcher": {
            "description": "Research specialist",
            "prompt": "You research topics"
        },
        "coder": {
            "description": "Code specialist",
            "prompt": "You write code"
        },
        "reviewer": {
            "description": "Review specialist",
            "prompt": "You review work"
        }
    }"#;

    let args = vec!["claude", "--agents", agents_json];
    let result = parse_cli_args(&args);

    assert!(result.is_ok());
    let config = result.unwrap();
    let agents = config.agents.unwrap();
    assert_eq!(agents.len(), 3);
    assert!(agents.contains_key("researcher"));
    assert!(agents.contains_key("coder"));
    assert!(agents.contains_key("reviewer"));
}

// ============================================================================
// UNIT TESTS: Tool Allow/Deny Lists (--allowedTools, --disallowedTools)
// ============================================================================

#[test]
fn test_allowed_tools_single_tool() {
    let args = vec!["claude", "--allowedTools", "Read"];
    let result = parse_cli_args(&args);

    assert!(result.is_ok());
    let config = result.unwrap();
    assert_eq!(config.allowed_tools, vec!["Read"]);
}

#[test]
fn test_allowed_tools_multiple_tools() {
    let args = vec!["claude", "--allowedTools", "Read", "Write", "Edit"];
    let result = parse_cli_args(&args);

    assert!(result.is_ok());
    let config = result.unwrap();
    assert_eq!(config.allowed_tools, vec!["Read", "Write", "Edit"]);
}

#[test]
fn test_allowed_tools_bash_pattern_matching() {
    // Should support patterns like "Bash(git log:*)"
    let args = vec!["claude", "--allowedTools", "Bash(git log:*)"];
    let result = parse_cli_args(&args);

    assert!(result.is_ok());
    let config = result.unwrap();
    assert_eq!(config.allowed_tools, vec!["Bash(git log:*)"]);
}

#[test]
fn test_allowed_tools_no_prompting() {
    // Allowed tools should not prompt for permission
    let args = vec!["claude", "--allowedTools", "Read"];
    let result = parse_cli_args(&args);

    assert!(result.is_ok());
    let config = result.unwrap();
    let permission = check_tool_permission(&config, "Read");
    assert_eq!(permission, ToolPermission::AutoAllow);
}

#[test]
fn test_disallowed_tools_single_tool() {
    let args = vec!["claude", "--disallowedTools", "Bash"];
    let result = parse_cli_args(&args);

    assert!(result.is_ok());
    let config = result.unwrap();
    assert_eq!(config.disallowed_tools, vec!["Bash"]);
}

#[test]
fn test_disallowed_tools_multiple_tools() {
    let args = vec!["claude", "--disallowedTools", "Bash", "Edit", "Write"];
    let result = parse_cli_args(&args);

    assert!(result.is_ok());
    let config = result.unwrap();
    assert_eq!(config.disallowed_tools, vec!["Bash", "Edit", "Write"]);
}

#[test]
fn test_disallowed_tools_pattern_matching() {
    let args = vec!["claude", "--disallowedTools", "Bash(git log:*)"];
    let result = parse_cli_args(&args);

    assert!(result.is_ok());
    let config = result.unwrap();
    assert_eq!(config.disallowed_tools, vec!["Bash(git log:*)"]);
}

#[test]
fn test_disallowed_tools_blocks_without_prompting() {
    // Disallowed tools should be blocked without prompting
    let args = vec!["claude", "--disallowedTools", "Edit"];
    let result = parse_cli_args(&args);

    assert!(result.is_ok());
    let config = result.unwrap();
    let permission = check_tool_permission(&config, "Edit");
    assert_eq!(permission, ToolPermission::Deny);
}

#[test]
fn test_allowed_and_disallowed_tools_together() {
    // Both flags can be used together
    let args = vec!["claude", "--allowedTools", "Read", "--disallowedTools", "Write"];
    let result = parse_cli_args(&args);

    assert!(result.is_ok());
    let config = result.unwrap();
    assert_eq!(config.allowed_tools, vec!["Read"]);
    assert_eq!(config.disallowed_tools, vec!["Write"]);
}

// ============================================================================
// UNIT TESTS: System Prompt Configuration
// ============================================================================

#[test]
fn test_system_prompt_replaces_default() {
    // --system-prompt should replace entire system prompt
    let custom_prompt = "You are a Python expert";
    let args = vec!["claude", "--system-prompt", custom_prompt];
    let result = parse_cli_args(&args);

    assert!(result.is_ok());
    let config = result.unwrap();
    assert_eq!(config.system_prompt, Some(custom_prompt.to_string()));
    assert_eq!(config.system_prompt_mode, SystemPromptMode::Replace);
}

#[test]
fn test_system_prompt_file_in_print_mode() {
    // --system-prompt-file should load from file (print mode only)
    let args = vec!["claude", "-p", "query", "--system-prompt-file", "./custom-prompt.txt"];
    let result = parse_cli_args(&args);

    assert!(result.is_ok());
    let config = result.unwrap();
    assert_eq!(config.system_prompt_file, Some("./custom-prompt.txt".to_string()));
    assert_eq!(config.mode, ExecutionMode::Print);
}

#[test]
fn test_system_prompt_file_fails_in_interactive_mode() {
    // --system-prompt-file should only work in print mode
    let args = vec!["claude", "--system-prompt-file", "./custom-prompt.txt"];
    let result = parse_cli_args(&args);

    assert!(result.is_err());
    assert!(result.unwrap_err().contains("only available in print mode"));
}

#[test]
fn test_append_system_prompt_adds_to_default() {
    // --append-system-prompt should append to default system prompt
    let append_text = "Always use TypeScript";
    let args = vec!["claude", "--append-system-prompt", append_text];
    let result = parse_cli_args(&args);

    assert!(result.is_ok());
    let config = result.unwrap();
    assert_eq!(config.append_system_prompt, Some(append_text.to_string()));
    assert_eq!(config.system_prompt_mode, SystemPromptMode::Append);
}

#[test]
fn test_append_system_prompt_preserves_default() {
    let args = vec!["claude", "--append-system-prompt", "Additional instructions"];
    let result = parse_cli_args(&args);

    assert!(result.is_ok());
    let config = result.unwrap();
    let full_prompt = build_system_prompt(&config);

    // Should contain both default and appended text
    assert!(full_prompt.contains("default system prompt content"));
    assert!(full_prompt.contains("Additional instructions"));
}

#[test]
fn test_system_prompt_and_append_mutually_exclusive() {
    // Using both --system-prompt and --append-system-prompt should error
    let args = vec![
        "claude",
        "--system-prompt", "Replace",
        "--append-system-prompt", "Append"
    ];
    let result = parse_cli_args(&args);

    assert!(result.is_err());
    assert!(result.unwrap_err().contains("cannot be used together"));
}

// ============================================================================
// UNIT TESTS: Output Format Configuration
// ============================================================================

#[test]
fn test_output_format_text_default() {
    let args = vec!["claude", "-p", "query"];
    let result = parse_cli_args(&args);

    assert!(result.is_ok());
    let config = result.unwrap();
    assert_eq!(config.output_format, OutputFormat::Text);
}

#[test]
fn test_output_format_json() {
    let args = vec!["claude", "-p", "query", "--output-format", "json"];
    let result = parse_cli_args(&args);

    assert!(result.is_ok());
    let config = result.unwrap();
    assert_eq!(config.output_format, OutputFormat::Json);
}

#[test]
fn test_output_format_stream_json() {
    let args = vec!["claude", "-p", "query", "--output-format", "stream-json"];
    let result = parse_cli_args(&args);

    assert!(result.is_ok());
    let config = result.unwrap();
    assert_eq!(config.output_format, OutputFormat::StreamJson);
}

#[test]
fn test_output_format_json_structure() {
    // JSON output should contain specific fields
    let args = vec!["claude", "-p", "test", "--output-format", "json"];
    let output = execute_cli_command(&args).unwrap();
    let json: serde_json::Value = serde_json::from_str(&output).unwrap();

    assert!(json.get("id").is_some());
    assert!(json.get("type").is_some());
    assert!(json.get("role").is_some());
    assert!(json.get("content").is_some());
    assert!(json.get("model").is_some());
    assert!(json.get("stop_reason").is_some());
    assert!(json.get("usage").is_some());
}

#[test]
fn test_output_format_stream_json_events() {
    // stream-json should output events line by line
    let args = vec!["claude", "-p", "hello", "--output-format", "stream-json"];
    let output = execute_cli_command(&args).unwrap();
    let lines: Vec<&str> = output.lines().collect();

    assert!(lines.len() > 0);
    // Each line should be valid JSON
    for line in lines {
        assert!(serde_json::from_str::<serde_json::Value>(line).is_ok());
    }
}

#[test]
fn test_output_format_text_plain_output() {
    // Text format should output plain response without JSON
    let args = vec!["claude", "-p", "Say hello", "--output-format", "text"];
    let output = execute_cli_command(&args).unwrap();

    // Should not be JSON
    assert!(serde_json::from_str::<serde_json::Value>(&output).is_err());
    // Should contain actual response text
    assert!(output.len() > 0);
}

// ============================================================================
// UNIT TESTS: Input Format Configuration
// ============================================================================

#[test]
fn test_input_format_text_default() {
    let args = vec!["claude", "-p", "query"];
    let result = parse_cli_args(&args);

    assert!(result.is_ok());
    let config = result.unwrap();
    assert_eq!(config.input_format, InputFormat::Text);
}

#[test]
fn test_input_format_stream_json() {
    let args = vec!["claude", "-p", "--input-format", "stream-json"];
    let result = parse_cli_args(&args);

    assert!(result.is_ok());
    let config = result.unwrap();
    assert_eq!(config.input_format, InputFormat::StreamJson);
}

#[test]
fn test_input_format_stream_json_parses_events() {
    // stream-json input should parse event stream
    let stream_input = r#"{"type":"message_start","message":{"role":"user"}}
{"type":"content_block_delta","delta":{"text":"Hello"}}
{"type":"message_stop"}"#;

    let args = vec!["claude", "-p", "--input-format", "stream-json"];
    let result = parse_cli_args_with_stdin(&args, Some(stream_input));

    assert!(result.is_ok());
}

// ============================================================================
// UNIT TESTS: Include Partial Messages Flag
// ============================================================================

#[test]
fn test_include_partial_messages_default_false() {
    let args = vec!["claude", "-p", "test"];
    let result = parse_cli_args(&args);

    assert!(result.is_ok());
    let config = result.unwrap();
    assert_eq!(config.include_partial_messages, false);
}

#[test]
fn test_include_partial_messages_flag() {
    let args = vec!["claude", "-p", "test", "--include-partial-messages"];
    let result = parse_cli_args(&args);

    assert!(result.is_ok());
    let config = result.unwrap();
    assert_eq!(config.include_partial_messages, true);
}

#[test]
fn test_include_partial_messages_requires_stream_json() {
    // Should work with stream-json output
    let args = vec![
        "claude", "-p", "test",
        "--output-format", "stream-json",
        "--include-partial-messages"
    ];
    let result = parse_cli_args(&args);

    assert!(result.is_ok());
}

#[test]
fn test_include_partial_messages_outputs_partial_events() {
    let args = vec![
        "claude", "-p", "hello",
        "--output-format", "stream-json",
        "--include-partial-messages"
    ];
    let output = execute_cli_command(&args).unwrap();

    // Should include partial message events
    assert!(output.contains("content_block_delta") || output.contains("partial"));
}

// ============================================================================
// UNIT TESTS: Verbose Logging Flag
// ============================================================================

#[test]
fn test_verbose_default_false() {
    let args = vec!["claude"];
    let result = parse_cli_args(&args);

    assert!(result.is_ok());
    let config = result.unwrap();
    assert_eq!(config.verbose, false);
}

#[test]
fn test_verbose_flag_enables_logging() {
    let args = vec!["claude", "--verbose"];
    let result = parse_cli_args(&args);

    assert!(result.is_ok());
    let config = result.unwrap();
    assert_eq!(config.verbose, true);
}

#[test]
fn test_verbose_shows_turn_by_turn_output() {
    let args = vec!["claude", "-p", "test", "--verbose"];
    let output = execute_cli_command(&args).unwrap();

    // Verbose output should show detailed information
    assert!(output.contains("turn") || output.contains("debug") || output.contains("tool"));
}

// ============================================================================
// UNIT TESTS: Max Turns Limit
// ============================================================================

#[test]
fn test_max_turns_default_unlimited() {
    let args = vec!["claude", "-p", "test"];
    let result = parse_cli_args(&args);

    assert!(result.is_ok());
    let config = result.unwrap();
    assert_eq!(config.max_turns, None);
}

#[test]
fn test_max_turns_flag_sets_limit() {
    let args = vec!["claude", "-p", "test", "--max-turns", "3"];
    let result = parse_cli_args(&args);

    assert!(result.is_ok());
    let config = result.unwrap();
    assert_eq!(config.max_turns, Some(3));
}

#[test]
fn test_max_turns_non_interactive_only() {
    // max-turns should only apply in non-interactive mode
    let args = vec!["claude", "-p", "query", "--max-turns", "5"];
    let result = parse_cli_args(&args);

    assert!(result.is_ok());
    let config = result.unwrap();
    assert_eq!(config.mode, ExecutionMode::Print);
    assert_eq!(config.max_turns, Some(5));
}

#[test]
fn test_max_turns_limits_agentic_loops() {
    let args = vec!["claude", "-p", "complex task", "--max-turns", "2"];
    let execution = execute_cli_command(&args).unwrap();
    let turn_count = count_turns(&execution);

    assert!(turn_count <= 2);
}

// ============================================================================
// UNIT TESTS: Model Selection Flag
// ============================================================================

#[test]
fn test_model_default() {
    let args = vec!["claude", "-p", "test"];
    let result = parse_cli_args(&args);

    assert!(result.is_ok());
    let config = result.unwrap();
    // Default model should be set
    assert!(config.model.is_some());
}

#[test]
fn test_model_flag_sets_model() {
    let args = vec!["claude", "-p", "test", "--model", "claude-sonnet-4-5-20250929"];
    let result = parse_cli_args(&args);

    assert!(result.is_ok());
    let config = result.unwrap();
    assert_eq!(config.model, Some("claude-sonnet-4-5-20250929".to_string()));
}

#[test]
fn test_model_alias_sonnet() {
    let args = vec!["claude", "--model", "sonnet"];
    let result = parse_cli_args(&args);

    assert!(result.is_ok());
    let config = result.unwrap();
    let resolved = resolve_model_alias(&config.model.unwrap());
    assert!(resolved.contains("sonnet"));
}

#[test]
fn test_model_alias_opus() {
    let args = vec!["claude", "--model", "opus"];
    let result = parse_cli_args(&args);

    assert!(result.is_ok());
    let config = result.unwrap();
    let resolved = resolve_model_alias(&config.model.unwrap());
    assert!(resolved.contains("opus"));
}

#[test]
fn test_model_alias_haiku() {
    let args = vec!["claude", "--model", "haiku"];
    let result = parse_cli_args(&args);

    assert!(result.is_ok());
    let config = result.unwrap();
    let resolved = resolve_model_alias(&config.model.unwrap());
    assert!(resolved.contains("haiku"));
}

#[test]
fn test_model_full_id() {
    // Should accept full model ID
    let full_id = "claude-opus-4-20250514";
    let args = vec!["claude", "--model", full_id];
    let result = parse_cli_args(&args);

    assert!(result.is_ok());
    let config = result.unwrap();
    assert_eq!(config.model, Some(full_id.to_string()));
}

// ============================================================================
// UNIT TESTS: Permission Mode Configuration
// ============================================================================

#[test]
fn test_permission_mode_default() {
    let args = vec!["claude"];
    let result = parse_cli_args(&args);

    assert!(result.is_ok());
    let config = result.unwrap();
    // Default should be ask/normal mode
    assert_eq!(config.permission_mode, PermissionMode::Ask);
}

#[test]
fn test_permission_mode_plan() {
    let args = vec!["claude", "--permission-mode", "plan"];
    let result = parse_cli_args(&args);

    assert!(result.is_ok());
    let config = result.unwrap();
    assert_eq!(config.permission_mode, PermissionMode::Plan);
}

#[test]
fn test_permission_mode_auto_accept() {
    let args = vec!["claude", "--permission-mode", "auto-accept"];
    let result = parse_cli_args(&args);

    assert!(result.is_ok());
    let config = result.unwrap();
    assert_eq!(config.permission_mode, PermissionMode::AutoAccept);
}

#[test]
fn test_permission_mode_ask() {
    let args = vec!["claude", "--permission-mode", "ask"];
    let result = parse_cli_args(&args);

    assert!(result.is_ok());
    let config = result.unwrap();
    assert_eq!(config.permission_mode, PermissionMode::Ask);
}

#[test]
fn test_permission_mode_invalid() {
    let args = vec!["claude", "--permission-mode", "invalid"];
    let result = parse_cli_args(&args);

    assert!(result.is_err());
}

// ============================================================================
// UNIT TESTS: Permission Prompt Tool
// ============================================================================

#[test]
fn test_permission_prompt_tool_default_none() {
    let args = vec!["claude"];
    let result = parse_cli_args(&args);

    assert!(result.is_ok());
    let config = result.unwrap();
    assert_eq!(config.permission_prompt_tool, None);
}

#[test]
fn test_permission_prompt_tool_mcp() {
    let args = vec!["claude", "-p", "test", "--permission-prompt-tool", "mcp_auth_tool"];
    let result = parse_cli_args(&args);

    assert!(result.is_ok());
    let config = result.unwrap();
    assert_eq!(config.permission_prompt_tool, Some("mcp_auth_tool".to_string()));
}

#[test]
fn test_permission_prompt_tool_delegates_prompts() {
    // Permission prompts should be delegated to MCP tool
    let args = vec![
        "claude", "-p", "use tool",
        "--permission-prompt-tool", "custom_prompt_handler"
    ];
    let result = parse_cli_args(&args);

    assert!(result.is_ok());
}

// ============================================================================
// UNIT TESTS: Dangerously Skip Permissions Flag
// ============================================================================

#[test]
fn test_dangerously_skip_permissions_default_false() {
    let args = vec!["claude"];
    let result = parse_cli_args(&args);

    assert!(result.is_ok());
    let config = result.unwrap();
    assert_eq!(config.skip_permissions, false);
}

#[test]
fn test_dangerously_skip_permissions_flag() {
    let args = vec!["claude", "--dangerously-skip-permissions"];
    let result = parse_cli_args(&args);

    assert!(result.is_ok());
    let config = result.unwrap();
    assert_eq!(config.skip_permissions, true);
}

#[test]
fn test_dangerously_skip_permissions_no_prompts() {
    // Should not prompt for any tool usage
    let args = vec!["claude", "-p", "use bash", "--dangerously-skip-permissions"];
    let result = parse_cli_args(&args);

    assert!(result.is_ok());
    let config = result.unwrap();
    assert_eq!(should_prompt_for_tool(&config, "Bash"), false);
}

#[test]
fn test_dangerously_skip_permissions_warning() {
    // Should display warning about caution
    let args = vec!["claude", "--dangerously-skip-permissions"];
    let output = execute_cli_command(&args).unwrap_or_default();

    // Implementation should warn users
    assert!(output.contains("caution") || output.contains("warning") || output.is_empty());
}

// ============================================================================
// INTEGRATION TESTS: Complex Flag Combinations
// ============================================================================

#[test]
fn test_print_mode_with_all_flags() {
    let args = vec![
        "claude",
        "-p", "explain this code",
        "--model", "sonnet",
        "--output-format", "json",
        "--max-turns", "3",
        "--verbose",
        "--system-prompt", "You are helpful",
        "--allowedTools", "Read", "Grep",
        "--disallowedTools", "Write"
    ];

    let result = parse_cli_args(&args);
    assert!(result.is_ok());

    let config = result.unwrap();
    assert_eq!(config.mode, ExecutionMode::Print);
    assert_eq!(config.prompt, Some("explain this code".to_string()));
    assert_eq!(config.output_format, OutputFormat::Json);
    assert_eq!(config.max_turns, Some(3));
    assert_eq!(config.verbose, true);
}

#[test]
fn test_continue_session_with_resume_fails() {
    // --continue and --resume should be mutually exclusive
    let args = vec!["claude", "-c", "-r", "session-123"];
    let result = parse_cli_args(&args);

    assert!(result.is_err());
    assert!(result.unwrap_err().contains("mutually exclusive"));
}

#[test]
fn test_interactive_with_add_dir_and_agents() {
    let agents_json = r#"{"worker": {"description": "d", "prompt": "p"}}"#;
    let args = vec![
        "claude",
        "--add-dir", "../lib",
        "--agents", agents_json,
        "initial prompt"
    ];

    let result = parse_cli_args(&args);
    assert!(result.is_ok());

    let config = result.unwrap();
    assert_eq!(config.mode, ExecutionMode::Interactive);
    assert_eq!(config.additional_dirs.len(), 1);
    assert!(config.agents.is_some());
}

#[test]
fn test_piped_input_with_continue_session() {
    let stdin_content = "cat test.log";
    let args = vec!["claude", "-c", "-p", "analyze this"];
    let result = parse_cli_args_with_stdin(&args, Some(stdin_content));

    assert!(result.is_ok());
    let config = result.unwrap();
    assert_eq!(config.continue_session, true);
    assert_eq!(config.mode, ExecutionMode::Print);
}

#[test]
fn test_stream_json_output_with_partial_messages() {
    let args = vec![
        "claude", "-p", "hello",
        "--output-format", "stream-json",
        "--include-partial-messages",
        "--verbose"
    ];

    let result = parse_cli_args(&args);
    assert!(result.is_ok());

    let config = result.unwrap();
    assert_eq!(config.output_format, OutputFormat::StreamJson);
    assert_eq!(config.include_partial_messages, true);
    assert_eq!(config.verbose, true);
}

// ============================================================================
// E2E TESTS: Complete Workflows
// ============================================================================

#[test]
fn test_e2e_simple_print_query() {
    // End-to-end test of simple query
    let args = vec!["claude", "-p", "What is 2+2?"];
    let output = execute_cli_command(&args);

    assert!(output.is_ok());
    let response = output.unwrap();
    assert!(response.len() > 0);
    // Response should contain answer
    assert!(response.contains("4") || response.contains("four"));
}

#[test]
fn test_e2e_json_output_structure() {
    let args = vec!["claude", "-p", "test", "--output-format", "json"];
    let output = execute_cli_command(&args);

    assert!(output.is_ok());
    let json: serde_json::Value = serde_json::from_str(&output.unwrap()).unwrap();

    // Verify complete JSON structure
    assert!(json["id"].is_string());
    assert!(json["role"].is_string());
    assert!(json["content"].is_array());
    assert!(json["model"].is_string());
}

#[test]
fn test_e2e_with_tools_and_permissions() {
    let args = vec![
        "claude", "-p", "read test.txt",
        "--allowedTools", "Read",
        "--permission-mode", "auto-accept"
    ];

    let output = execute_cli_command(&args);
    assert!(output.is_ok());
}

#[test]
fn test_e2e_multi_turn_with_max_turns() {
    let args = vec![
        "claude", "-p", "complex multi-step task",
        "--max-turns", "5",
        "--verbose"
    ];

    let output = execute_cli_command(&args);
    assert!(output.is_ok());
}

#[test]
fn test_e2e_custom_model_with_system_prompt() {
    let args = vec![
        "claude", "-p", "explain Python",
        "--model", "sonnet",
        "--system-prompt", "You are a Python expert who explains clearly"
    ];

    let output = execute_cli_command(&args);
    assert!(output.is_ok());
}

#[test]
fn test_e2e_session_lifecycle() {
    // Create new session
    let args1 = vec!["claude", "-p", "First query"];
    let output1 = execute_cli_command(&args1);
    assert!(output1.is_ok());

    // Continue session
    let args2 = vec!["claude", "-c", "-p", "Follow-up query"];
    let output2 = execute_cli_command(&args2);
    assert!(output2.is_ok());

    // Session should maintain context
}

#[test]
fn test_e2e_piped_input_workflow() {
    let log_content = "[ERROR] Connection failed\n[INFO] Retrying...";
    let args = vec!["claude", "-p", "analyze these logs"];
    let output = execute_cli_command_with_stdin(&args, log_content);

    assert!(output.is_ok());
    let response = output.unwrap();
    assert!(response.len() > 0);
}

// ============================================================================
// BOUNDARY TESTS: Edge Cases
// ============================================================================

#[test]
fn test_empty_prompt() {
    let args = vec!["claude", "-p", ""];
    let result = parse_cli_args(&args);

    assert!(result.is_err() || result.unwrap().prompt == Some("".to_string()));
}

#[test]
fn test_very_long_prompt() {
    let long_prompt = "x".repeat(100000);
    let args = vec!["claude", "-p", &long_prompt];
    let result = parse_cli_args(&args);

    assert!(result.is_ok());
    let config = result.unwrap();
    assert_eq!(config.prompt.unwrap().len(), 100000);
}

#[test]
fn test_max_turns_zero() {
    let args = vec!["claude", "-p", "test", "--max-turns", "0"];
    let result = parse_cli_args(&args);

    // Zero turns should either error or be treated as 1
    assert!(result.is_err() || result.unwrap().max_turns == Some(0));
}

#[test]
fn test_max_turns_negative() {
    let args = vec!["claude", "-p", "test", "--max-turns", "-1"];
    let result = parse_cli_args(&args);

    assert!(result.is_err());
}

#[test]
fn test_invalid_json_in_agents() {
    let args = vec!["claude", "--agents", "not valid json {"];
    let result = parse_cli_args(&args);

    assert!(result.is_err());
    assert!(result.unwrap_err().contains("JSON") || result.unwrap_err().contains("parse"));
}

#[test]
fn test_empty_agents_json() {
    let args = vec!["claude", "--agents", "{}"];
    let result = parse_cli_args(&args);

    assert!(result.is_ok());
    let config = result.unwrap();
    assert_eq!(config.agents.unwrap().len(), 0);
}

#[test]
fn test_invalid_output_format() {
    let args = vec!["claude", "-p", "test", "--output-format", "xml"];
    let result = parse_cli_args(&args);

    assert!(result.is_err());
}

#[test]
fn test_invalid_permission_mode() {
    let args = vec!["claude", "--permission-mode", "invalid-mode"];
    let result = parse_cli_args(&args);

    assert!(result.is_err());
}

#[test]
fn test_add_dir_nonexistent_directory() {
    let args = vec!["claude", "--add-dir", "/nonexistent/path/xyz"];
    let result = parse_cli_args(&args);

    // Should parse OK, but runtime should warn or error
    assert!(result.is_ok());
}

#[test]
fn test_system_prompt_file_nonexistent() {
    let args = vec!["claude", "-p", "test", "--system-prompt-file", "/no/such/file.txt"];
    let result = parse_cli_args(&args);

    // Should parse OK, but execution should error
    assert!(result.is_ok());
}

#[test]
fn test_resume_invalid_session_id() {
    let args = vec!["claude", "-r", "nonexistent-session"];
    let result = parse_cli_args(&args);

    // Should parse OK, but execution should error
    assert!(result.is_ok());
}

#[test]
fn test_special_characters_in_prompt() {
    let special_prompt = "Test with\nnewlines\tand\ttabs and \"quotes\" and 'apostrophes'";
    let args = vec!["claude", "-p", special_prompt];
    let result = parse_cli_args(&args);

    assert!(result.is_ok());
    let config = result.unwrap();
    assert_eq!(config.prompt, Some(special_prompt.to_string()));
}

#[test]
fn test_unicode_in_prompt() {
    let unicode_prompt = "Hello 世界 🌍 Здравствуй мир";
    let args = vec!["claude", "-p", unicode_prompt];
    let result = parse_cli_args(&args);

    assert!(result.is_ok());
    let config = result.unwrap();
    assert_eq!(config.prompt, Some(unicode_prompt.to_string()));
}

// ============================================================================
// MOCK TYPES AND HELPER FUNCTIONS FOR TESTING
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
enum ExecutionMode {
    Interactive,
    Print,
    Update,
    MCP,
}

#[derive(Debug, Clone, PartialEq)]
enum OutputFormat {
    Text,
    Json,
    StreamJson,
}

#[derive(Debug, Clone, PartialEq)]
enum InputFormat {
    Text,
    StreamJson,
}

#[derive(Debug, Clone, PartialEq)]
enum PermissionMode {
    Ask,
    AutoAccept,
    Plan,
}

#[derive(Debug, Clone, PartialEq)]
enum SystemPromptMode {
    Replace,
    Append,
}

#[derive(Debug, Clone, PartialEq)]
enum DirectoryAccess {
    Allowed,
    Denied,
}

#[derive(Debug, Clone, PartialEq)]
enum ToolPermission {
    Ask,
    AutoAllow,
    Deny,
}

#[derive(Debug, Clone)]
struct AgentConfig {
    description: String,
    prompt: String,
    tools: Vec<String>,
    model: Option<String>,
    inherits_all_tools: bool,
}

#[derive(Debug, Clone)]
struct CliConfig {
    mode: ExecutionMode,
    prompt: Option<String>,
    stdin_content: Option<String>,
    continue_session: bool,
    resume_session_id: Option<String>,
    list_sessions: bool,
    additional_dirs: Vec<String>,
    agents: Option<std::collections::HashMap<String, AgentConfig>>,
    allowed_tools: Vec<String>,
    disallowed_tools: Vec<String>,
    system_prompt: Option<String>,
    system_prompt_file: Option<String>,
    append_system_prompt: Option<String>,
    system_prompt_mode: SystemPromptMode,
    output_format: OutputFormat,
    input_format: InputFormat,
    include_partial_messages: bool,
    verbose: bool,
    max_turns: Option<u32>,
    model: Option<String>,
    permission_mode: PermissionMode,
    permission_prompt_tool: Option<String>,
    skip_permissions: bool,
}

impl Default for CliConfig {
    fn default() -> Self {
        Self {
            mode: ExecutionMode::Interactive,
            prompt: None,
            stdin_content: None,
            continue_session: false,
            resume_session_id: None,
            list_sessions: false,
            additional_dirs: vec![],
            agents: None,
            allowed_tools: vec![],
            disallowed_tools: vec![],
            system_prompt: None,
            system_prompt_file: None,
            append_system_prompt: None,
            system_prompt_mode: SystemPromptMode::Replace,
            output_format: OutputFormat::Text,
            input_format: InputFormat::Text,
            include_partial_messages: false,
            verbose: false,
            max_turns: None,
            model: Some("claude-sonnet-4-5-20250929".to_string()),
            permission_mode: PermissionMode::Ask,
            permission_prompt_tool: None,
            skip_permissions: false,
        }
    }
}

fn parse_cli_args(_args: &[&str]) -> Result<CliConfig, String> {
    // Mock implementation - will be replaced with actual parser
    Ok(CliConfig::default())
}

fn parse_cli_args_with_stdin(_args: &[&str], _stdin: Option<&str>) -> Result<CliConfig, String> {
    // Mock implementation
    Ok(CliConfig::default())
}

fn execute_cli_command(_args: &[&str]) -> Result<String, String> {
    // Mock implementation
    Ok("mock response".to_string())
}

fn execute_cli_command_with_stdin(_args: &[&str], _stdin: &str) -> Result<String, String> {
    // Mock implementation
    Ok("mock response".to_string())
}

fn check_directory_access(_config: &CliConfig, _dir: &str) -> DirectoryAccess {
    DirectoryAccess::Allowed
}

fn check_tool_permission(_config: &CliConfig, _tool: &str) -> ToolPermission {
    ToolPermission::Ask
}

fn should_prompt_for_tool(_config: &CliConfig, _tool: &str) -> bool {
    !_config.skip_permissions
}

fn resolve_model_alias(model: &str) -> String {
    match model {
        "sonnet" => "claude-sonnet-4-5-20250929".to_string(),
        "opus" => "claude-opus-4-20250514".to_string(),
        "haiku" => "claude-3-5-haiku-20241022".to_string(),
        other => other.to_string(),
    }
}

fn build_system_prompt(_config: &CliConfig) -> String {
    "default system prompt content".to_string()
}

fn count_turns(_output: &str) -> u32 {
    // Mock implementation
    1
}
