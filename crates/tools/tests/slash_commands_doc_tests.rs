//! Comprehensive test suite for ALL slash command features from documentation
//! https://code.claude.com/docs/en/slash-commands
//!
//! TDD APPROACH: These tests are FAILING by design - they define the specification
//! that the slash command implementation must satisfy.
//!
//! Test Coverage:
//! 1. Built-in Commands (30+ system commands)
//! 2. Custom Slash Commands (project & personal)
//! 3. Syntax and Parameters ($ARGUMENTS, $1, $2, etc.)
//! 4. Advanced Features (namespacing, bash integration, file references)
//! 5. Frontmatter Configuration (all fields)
//! 6. SlashCommand Tool Integration
//! 7. Plugin and MCP Commands
//! 8. Permission System
//! 9. Character Budget Limits
//! 10. Error Handling and Edge Cases

use std::path::PathBuf;
use tokio::fs;

// ============================================================================
// TEST FIXTURES
// ============================================================================

struct TestFixture {
    project_commands_dir: PathBuf,
    personal_commands_dir: PathBuf,
}

impl TestFixture {
    async fn new() -> Self {
        let project_commands_dir = PathBuf::from(".claude/commands_test");
        let personal_commands_dir = PathBuf::from("/tmp/.claude_test/commands");

        let _ = fs::create_dir_all(&project_commands_dir).await;
        let _ = fs::create_dir_all(&personal_commands_dir).await;

        TestFixture {
            project_commands_dir,
            personal_commands_dir,
        }
    }

    async fn create_command(&self, name: &str, content: &str, is_personal: bool) -> std::io::Result<PathBuf> {
        let dir = if is_personal {
            &self.personal_commands_dir
        } else {
            &self.project_commands_dir
        };
        let path = dir.join(format!("{}.md", name));
        fs::write(&path, content).await?;
        Ok(path)
    }

    async fn create_namespaced_command(&self, namespace: &str, name: &str, content: &str) -> std::io::Result<PathBuf> {
        let dir = self.project_commands_dir.join(namespace);
        let _ = fs::create_dir_all(&dir).await;
        let path = dir.join(format!("{}.md", name));
        fs::write(&path, content).await?;
        Ok(path)
    }

    async fn cleanup(&self) {
        let _ = fs::remove_dir_all(&self.project_commands_dir).await;
        let _ = fs::remove_dir_all(&self.personal_commands_dir).await;
    }
}

// ============================================================================
// 1. BUILT-IN COMMANDS - System Commands
// ============================================================================

mod builtin_commands {
    use super::*;

    #[test]
    fn test_help_command_exists() {
        // FAILING: /help must be a built-in command
        let cmd_name = "help";
        assert!(is_builtin_command(cmd_name), "/help is a built-in command");
    }

    #[test]
    fn test_clear_command_exists() {
        // FAILING: /clear must be a built-in command
        let cmd_name = "clear";
        assert!(is_builtin_command(cmd_name), "/clear is a built-in command");
    }

    #[test]
    fn test_cost_command_exists() {
        // FAILING: /cost must be a built-in command
        let cmd_name = "cost";
        assert!(is_builtin_command(cmd_name), "/cost is a built-in command");
    }

    #[test]
    fn test_model_command_exists() {
        // FAILING: /model must be a built-in command
        let cmd_name = "model";
        assert!(is_builtin_command(cmd_name), "/model is a built-in command");
    }

    #[test]
    fn test_export_command_exists() {
        // FAILING: /export must be a built-in command
        let cmd_name = "export";
        assert!(is_builtin_command(cmd_name), "/export is a built-in command");
    }

    #[test]
    fn test_sandbox_command_exists() {
        // FAILING: /sandbox must be a built-in command
        let cmd_name = "sandbox";
        assert!(is_builtin_command(cmd_name), "/sandbox is a built-in command");
    }

    #[test]
    fn test_vim_command_exists() {
        // FAILING: /vim must be a built-in command
        let cmd_name = "vim";
        assert!(is_builtin_command(cmd_name), "/vim is a built-in command");
    }

    #[test]
    fn test_builtin_commands_not_overridable() {
        // FAILING: Built-in commands cannot be overridden by custom commands
        let builtin = "help";
        assert!(!can_override_builtin(builtin), "Built-in commands are protected");
    }

    #[test]
    fn test_builtin_command_count() {
        // FAILING: There must be 30+ built-in commands
        let count = get_builtin_command_count();
        assert!(count >= 30, "Expected at least 30 built-in commands, got {}", count);
    }

    // Helper functions (these would fail to compile - that's the point!)
    fn is_builtin_command(_name: &str) -> bool {
        panic!("NOT IMPLEMENTED: Check if command is built-in")
    }

    fn can_override_builtin(_name: &str) -> bool {
        panic!("NOT IMPLEMENTED: Check if built-in can be overridden")
    }

    fn get_builtin_command_count() -> usize {
        panic!("NOT IMPLEMENTED: Get count of built-in commands")
    }
}

// ============================================================================
// 2. CUSTOM SLASH COMMANDS - Project and Personal Scopes
// ============================================================================

mod custom_commands {
    use super::*;

    #[tokio::test]
    async fn test_project_command_location() {
        // FAILING: Project commands must be in .claude/commands/
        let fixture = TestFixture::new().await;
        let content = "Project command content";

        let path = fixture.create_command("project-cmd", content, false).await.unwrap();

        assert!(path.to_string_lossy().contains(".claude/commands"));
        assert!(command_exists("project-cmd", CommandScope::Project).await);

        fixture.cleanup().await;
    }

    #[tokio::test]
    async fn test_personal_command_location() {
        // FAILING: Personal commands must be in ~/.claude/commands/
        let fixture = TestFixture::new().await;
        let content = "Personal command content";

        let _path = fixture.create_command("personal-cmd", content, true).await.unwrap();

        assert!(command_exists("personal-cmd", CommandScope::Personal).await);

        fixture.cleanup().await;
    }

    #[tokio::test]
    async fn test_command_name_derives_from_filename() {
        // FAILING: Command name must be filename without .md extension
        let fixture = TestFixture::new().await;

        let _ = fixture.create_command("my-awesome-command", "Content", false).await;

        let cmd_name = get_command_name_from_file("my-awesome-command.md");
        assert_eq!(cmd_name, "my-awesome-command");

        fixture.cleanup().await;
    }

    #[tokio::test]
    async fn test_command_must_be_markdown() {
        // FAILING: Command files must have .md extension
        let fixture = TestFixture::new().await;

        assert!(validate_command_extension("command.md"));
        assert!(!validate_command_extension("command.txt"));
        assert!(!validate_command_extension("command"));

        fixture.cleanup().await;
    }

    // Helper types and functions
    enum CommandScope {
        Project,
        Personal,
    }

    async fn command_exists(_name: &str, _scope: CommandScope) -> bool {
        panic!("NOT IMPLEMENTED: Check if command exists in scope")
    }

    fn get_command_name_from_file(_filename: &str) -> String {
        panic!("NOT IMPLEMENTED: Extract command name from filename")
    }

    fn validate_command_extension(_filename: &str) -> bool {
        panic!("NOT IMPLEMENTED: Validate command file extension")
    }
}

// ============================================================================
// 3. ARGUMENT HANDLING - $ARGUMENTS and Positional ($1, $2, $3)
// ============================================================================

mod argument_handling {
    use super::*;

    #[tokio::test]
    async fn test_arguments_placeholder_all_args() {
        // FAILING: $ARGUMENTS must capture everything passed to command
        let fixture = TestFixture::new().await;
        let content = "Process these: $ARGUMENTS";

        let _ = fixture.create_command("process", content, false).await;

        let expanded = expand_command("process", Some("foo bar baz")).await.unwrap();
        assert_eq!(expanded, "Process these: foo bar baz");

        fixture.cleanup().await;
    }

    #[tokio::test]
    async fn test_positional_argument_1() {
        // FAILING: $1 must access first positional argument
        let fixture = TestFixture::new().await;
        let content = "Review PR #$1";

        let _ = fixture.create_command("review", content, false).await;

        let expanded = expand_command("review", Some("123")).await.unwrap();
        assert_eq!(expanded, "Review PR #123");

        fixture.cleanup().await;
    }

    #[tokio::test]
    async fn test_positional_argument_2() {
        // FAILING: $2 must access second positional argument
        let fixture = TestFixture::new().await;
        let content = "PR #$1 priority $2";

        let _ = fixture.create_command("review", content, false).await;

        let expanded = expand_command("review", Some("456 high")).await.unwrap();
        assert_eq!(expanded, "PR #456 priority high");

        fixture.cleanup().await;
    }

    #[tokio::test]
    async fn test_positional_argument_3() {
        // FAILING: $3 must access third positional argument
        let fixture = TestFixture::new().await;
        let content = "PR #$1 priority $2 assigned to $3";

        let _ = fixture.create_command("review", content, false).await;

        let expanded = expand_command("review", Some("789 high alice")).await.unwrap();
        assert_eq!(expanded, "PR #789 priority high assigned to alice");

        fixture.cleanup().await;
    }

    #[tokio::test]
    async fn test_multiple_positional_arguments() {
        // FAILING: Support multiple positional arguments ($1-$9 at minimum)
        let fixture = TestFixture::new().await;
        let content = "$1 $2 $3 $4 $5";

        let _ = fixture.create_command("multi", content, false).await;

        let expanded = expand_command("multi", Some("a b c d e")).await.unwrap();
        assert_eq!(expanded, "a b c d e");

        fixture.cleanup().await;
    }

    #[tokio::test]
    async fn test_positional_with_defaults() {
        // FAILING: Positional args can have defaults if not provided
        let fixture = TestFixture::new().await;
        let content = "Priority: ${1:-medium}";

        let _ = fixture.create_command("task", content, false).await;

        let expanded_no_arg = expand_command("task", None).await.unwrap();
        assert_eq!(expanded_no_arg, "Priority: medium");

        let expanded_with_arg = expand_command("task", Some("high")).await.unwrap();
        assert_eq!(expanded_with_arg, "Priority: high");

        fixture.cleanup().await;
    }

    #[tokio::test]
    async fn test_arguments_appear_anywhere_in_prompt() {
        // FAILING: Arguments can be placed anywhere in the prompt
        let fixture = TestFixture::new().await;
        let content = "Start $1 middle $2 end $3";

        let _ = fixture.create_command("order", content, false).await;

        let expanded = expand_command("order", Some("A B C")).await.unwrap();
        assert_eq!(expanded, "Start A middle B end C");

        fixture.cleanup().await;
    }

    #[tokio::test]
    async fn test_mixed_arguments_and_positional() {
        // FAILING: Can mix $ARGUMENTS and positional in same command
        let fixture = TestFixture::new().await;
        let content = "First arg: $1\nAll args: $ARGUMENTS";

        let _ = fixture.create_command("mixed", content, false).await;

        let expanded = expand_command("mixed", Some("foo bar baz")).await.unwrap();
        assert!(expanded.contains("First arg: foo"));
        assert!(expanded.contains("All args: foo bar baz"));

        fixture.cleanup().await;
    }

    async fn expand_command(_name: &str, _args: Option<&str>) -> Result<String, String> {
        panic!("NOT IMPLEMENTED: Expand command with arguments")
    }
}

// ============================================================================
// 4. ADVANCED FEATURES - Namespacing, Bash, File References
// ============================================================================

mod advanced_features {
    use super::*;

    // --- Namespacing Tests ---

    #[tokio::test]
    async fn test_namespace_subdirectory_structure() {
        // FAILING: Subdirectories create namespaces without affecting invocation
        let fixture = TestFixture::new().await;
        let content = "Namespaced command";

        let _ = fixture.create_namespaced_command("utils", "helper", content).await;

        // Command should still be invoked as /helper, not /utils:helper
        assert!(command_exists_in_namespace("helper", "utils").await);
        let invocation_name = get_invocation_name("utils", "helper");
        assert_eq!(invocation_name, "helper");

        fixture.cleanup().await;
    }

    #[tokio::test]
    async fn test_namespace_in_help_description() {
        // FAILING: Namespace appears in help as "(project:subdirectory)"
        let fixture = TestFixture::new().await;
        let content = "---\ndescription: Helper command\n---\nContent";

        let _ = fixture.create_namespaced_command("utils", "helper", content).await;

        let help_entry = get_help_entry("helper").await.unwrap();
        assert!(help_entry.contains("(project:utils)") || help_entry.contains("project:utils"));

        fixture.cleanup().await;
    }

    #[tokio::test]
    async fn test_personal_command_namespace_label() {
        // FAILING: Personal commands show "(user)" in help
        let fixture = TestFixture::new().await;
        let content = "---\ndescription: Personal command\n---\nContent";

        let _ = fixture.create_command("personal", content, true).await;

        let help_entry = get_help_entry("personal").await.unwrap();
        assert!(help_entry.contains("(user)"));

        fixture.cleanup().await;
    }

    // --- Bash Integration Tests ---

    #[tokio::test]
    async fn test_bash_prefix_execution() {
        // FAILING: Commands prefixed with ! execute shell operations
        let fixture = TestFixture::new().await;
        let content = "---\nallowed-tools:\n  - Bash:ls\n---\n!ls -la\nFiles listed above";

        let _ = fixture.create_command("listfiles", content, false).await;

        let result = execute_command("listfiles", None).await.unwrap();
        assert!(result.bash_executed);
        assert!(result.output.contains("Files listed above"));

        fixture.cleanup().await;
    }

    #[tokio::test]
    async fn test_bash_requires_allowed_tools() {
        // FAILING: Bash commands require allowed-tools frontmatter
        let fixture = TestFixture::new().await;
        let content = "!echo test"; // No allowed-tools

        let _ = fixture.create_command("unsafe", content, false).await;

        let result = execute_command("unsafe", None).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("allowed-tools"));

        fixture.cleanup().await;
    }

    #[tokio::test]
    async fn test_bash_output_in_context() {
        // FAILING: Bash output is included in command context
        let fixture = TestFixture::new().await;
        let content = "---\nallowed-tools:\n  - Bash:echo\n---\n!echo 'test output'\nAbove is the output";

        let _ = fixture.create_command("echotest", content, false).await;

        let result = execute_command("echotest", None).await.unwrap();
        assert!(result.output.contains("test output"));

        fixture.cleanup().await;
    }

    // --- File References Tests ---

    #[tokio::test]
    async fn test_at_prefix_includes_file() {
        // FAILING: @ prefix includes file contents
        let fixture = TestFixture::new().await;

        // Create a test file
        let test_file = "/tmp/test_include.js";
        fs::write(test_file, "function test() { return 42; }").await.unwrap();

        let content = "Review this file:\n@/tmp/test_include.js";
        let _ = fixture.create_command("review-file", content, false).await;

        let result = execute_command("review-file", None).await.unwrap();
        assert!(result.output.contains("function test()"));

        fixture.cleanup().await;
    }

    #[tokio::test]
    async fn test_multiple_file_references() {
        // FAILING: Can reference multiple files in one command
        let fixture = TestFixture::new().await;

        fs::write("/tmp/file1.js", "const a = 1;").await.unwrap();
        fs::write("/tmp/file2.js", "const b = 2;").await.unwrap();

        let content = "@/tmp/file1.js\n@/tmp/file2.js";
        let _ = fixture.create_command("review-multi", content, false).await;

        let result = execute_command("review-multi", None).await.unwrap();
        assert!(result.output.contains("const a = 1;"));
        assert!(result.output.contains("const b = 2;"));

        fixture.cleanup().await;
    }

    #[tokio::test]
    async fn test_file_reference_with_args() {
        // FAILING: File references work with argument substitution
        let fixture = TestFixture::new().await;
        let content = "Review @$1 for issues";

        let _ = fixture.create_command("review-arg", content, false).await;

        fs::write("/tmp/target.js", "code").await.unwrap();
        let result = execute_command("review-arg", Some("/tmp/target.js")).await.unwrap();
        assert!(result.output.contains("code"));

        fixture.cleanup().await;
    }

    // --- Extended Thinking Tests ---

    #[tokio::test]
    async fn test_extended_thinking_keyword_detection() {
        // FAILING: Commands with thinking keywords trigger extended thinking
        let fixture = TestFixture::new().await;
        let content = "Think carefully about this problem and analyze deeply.";

        let _ = fixture.create_command("think", content, false).await;

        let result = execute_command("think", None).await.unwrap();
        assert!(result.triggers_extended_thinking);

        fixture.cleanup().await;
    }

    #[tokio::test]
    async fn test_extended_thinking_keywords() {
        // FAILING: Multiple thinking keywords should be detected
        let thinking_keywords = vec!["think", "analyze", "consider", "deeply", "carefully"];

        for keyword in thinking_keywords {
            let content = format!("Please {} this problem", keyword);
            let triggers = detect_extended_thinking(&content);
            assert!(triggers, "Keyword '{}' should trigger extended thinking", keyword);
        }
    }

    // Helper functions
    async fn command_exists_in_namespace(_name: &str, _namespace: &str) -> bool {
        panic!("NOT IMPLEMENTED: Check command exists in namespace")
    }

    fn get_invocation_name(_namespace: &str, _name: &str) -> String {
        panic!("NOT IMPLEMENTED: Get command invocation name")
    }

    async fn get_help_entry(_name: &str) -> Result<String, String> {
        panic!("NOT IMPLEMENTED: Get help entry for command")
    }

    #[derive(Debug)]
    struct CommandResult {
        bash_executed: bool,
        output: String,
        triggers_extended_thinking: bool,
    }

    async fn execute_command(_name: &str, _args: Option<&str>) -> Result<CommandResult, String> {
        panic!("NOT IMPLEMENTED: Execute command")
    }

    fn detect_extended_thinking(_content: &str) -> bool {
        panic!("NOT IMPLEMENTED: Detect extended thinking keywords")
    }
}

// ============================================================================
// 5. FRONTMATTER CONFIGURATION - All Fields
// ============================================================================

mod frontmatter_config {
    use super::*;

    #[tokio::test]
    async fn test_description_field() {
        // FAILING: description field must be shown in /help
        let fixture = TestFixture::new().await;
        let content = "---\ndescription: Review a pull request\n---\nContent";

        let _ = fixture.create_command("review", content, false).await;

        let frontmatter = parse_frontmatter(content).unwrap();
        assert_eq!(frontmatter.description, Some("Review a pull request".to_string()));

        fixture.cleanup().await;
    }

    #[tokio::test]
    async fn test_allowed_tools_field() {
        // FAILING: allowed-tools field restricts available tools
        let fixture = TestFixture::new().await;
        let content = "---\nallowed-tools:\n  - Bash\n  - Grep\n---\nContent";

        let _ = fixture.create_command("restricted", content, false).await;

        let frontmatter = parse_frontmatter(content).unwrap();
        assert_eq!(frontmatter.allowed_tools, vec!["Bash", "Grep"]);

        fixture.cleanup().await;
    }

    #[tokio::test]
    async fn test_argument_hint_field() {
        // FAILING: argument-hint shown in auto-complete
        let fixture = TestFixture::new().await;
        let content = "---\nargument-hint: <pr-number> <priority>\n---\nContent";

        let _ = fixture.create_command("review", content, false).await;

        let frontmatter = parse_frontmatter(content).unwrap();
        assert_eq!(frontmatter.argument_hint, Some("<pr-number> <priority>".to_string()));

        fixture.cleanup().await;
    }

    #[tokio::test]
    async fn test_model_field() {
        // FAILING: model field overrides default model selection
        let fixture = TestFixture::new().await;
        let content = "---\nmodel: claude-sonnet-4.5\n---\nContent";

        let _ = fixture.create_command("sonnet-only", content, false).await;

        let frontmatter = parse_frontmatter(content).unwrap();
        assert_eq!(frontmatter.model, Some("claude-sonnet-4.5".to_string()));

        fixture.cleanup().await;
    }

    #[tokio::test]
    async fn test_disable_model_invocation_field() {
        // FAILING: disable-model-invocation prevents SlashCommand tool execution
        let fixture = TestFixture::new().await;
        let content = "---\ndisable-model-invocation: true\n---\nContent";

        let _ = fixture.create_command("no-invoke", content, false).await;

        let frontmatter = parse_frontmatter(content).unwrap();
        assert!(frontmatter.disable_model_invocation);

        fixture.cleanup().await;
    }

    #[tokio::test]
    async fn test_all_frontmatter_fields_together() {
        // FAILING: All frontmatter fields work together
        let fixture = TestFixture::new().await;
        let content = "---\n\
            description: Complete example\n\
            allowed-tools:\n  - Bash\n  - Read\n\
            argument-hint: <file>\n\
            model: claude-sonnet-4.5\n\
            disable-model-invocation: false\n\
            ---\n\
            Content here";

        let _ = fixture.create_command("complete", content, false).await;

        let fm = parse_frontmatter(content).unwrap();
        assert_eq!(fm.description, Some("Complete example".to_string()));
        assert_eq!(fm.allowed_tools, vec!["Bash", "Read"]);
        assert_eq!(fm.argument_hint, Some("<file>".to_string()));
        assert_eq!(fm.model, Some("claude-sonnet-4.5".to_string()));
        assert!(!fm.disable_model_invocation);

        fixture.cleanup().await;
    }

    #[tokio::test]
    async fn test_empty_frontmatter() {
        // FAILING: Empty frontmatter should be valid
        let fixture = TestFixture::new().await;
        let content = "---\n---\nContent only";

        let _ = fixture.create_command("minimal", content, false).await;

        let fm = parse_frontmatter(content).unwrap();
        assert!(fm.description.is_none());
        assert!(fm.allowed_tools.is_empty());

        fixture.cleanup().await;
    }

    #[tokio::test]
    async fn test_no_frontmatter() {
        // FAILING: Commands without frontmatter should work
        let fixture = TestFixture::new().await;
        let content = "Just content, no frontmatter";

        let _ = fixture.create_command("plain", content, false).await;

        let result = parse_frontmatter(content);
        assert!(result.is_ok());

        fixture.cleanup().await;
    }

    // Frontmatter structure
    struct FrontMatter {
        description: Option<String>,
        allowed_tools: Vec<String>,
        argument_hint: Option<String>,
        model: Option<String>,
        disable_model_invocation: bool,
    }

    fn parse_frontmatter(_content: &str) -> Result<FrontMatter, String> {
        panic!("NOT IMPLEMENTED: Parse frontmatter")
    }
}

// ============================================================================
// 6. SLASHCOMMAND TOOL - Programmatic Invocation
// ============================================================================

mod slash_command_tool {
    use super::*;

    #[tokio::test]
    async fn test_tool_only_custom_commands() {
        // FAILING: SlashCommand tool only supports custom commands
        let fixture = TestFixture::new().await;

        let result = can_invoke_via_tool("help").await;
        assert!(!result, "Built-in commands cannot be invoked via SlashCommand tool");

        let _ = fixture.create_command("custom", "Custom content", false).await;
        let result = can_invoke_via_tool("custom").await;
        assert!(result, "Custom commands can be invoked via SlashCommand tool");

        fixture.cleanup().await;
    }

    #[tokio::test]
    async fn test_tool_requires_description() {
        // FAILING: SlashCommand tool requires description in frontmatter
        let fixture = TestFixture::new().await;

        let no_desc = "---\n---\nContent";
        let _ = fixture.create_command("nodesc", no_desc, false).await;
        assert!(!is_tool_discoverable("nodesc").await);

        let with_desc = "---\ndescription: Has description\n---\nContent";
        let _ = fixture.create_command("withdesc", with_desc, false).await;
        assert!(is_tool_discoverable("withdesc").await);

        fixture.cleanup().await;
    }

    #[tokio::test]
    async fn test_tool_character_budget_limit() {
        // FAILING: SlashCommand tool has 15,000 character budget limit
        let fixture = TestFixture::new().await;

        let budget = get_slash_command_tool_budget();
        assert_eq!(budget, 15_000);

        fixture.cleanup().await;
    }

    #[tokio::test]
    async fn test_tool_budget_env_var_override() {
        // FAILING: SLASH_COMMAND_TOOL_CHAR_BUDGET env var overrides budget
        std::env::set_var("SLASH_COMMAND_TOOL_CHAR_BUDGET", "20000");

        let budget = get_slash_command_tool_budget();
        assert_eq!(budget, 20_000);

        std::env::remove_var("SLASH_COMMAND_TOOL_CHAR_BUDGET");
    }

    #[tokio::test]
    async fn test_tool_can_be_disabled() {
        // FAILING: SlashCommand tool can be disabled entirely via permissions
        let fixture = TestFixture::new().await;

        set_tool_permission("SlashCommand", false).await;
        assert!(!is_tool_enabled("SlashCommand").await);

        set_tool_permission("SlashCommand", true).await;
        assert!(is_tool_enabled("SlashCommand").await);

        fixture.cleanup().await;
    }

    #[tokio::test]
    async fn test_tool_specific_command_permission() {
        // FAILING: Can restrict specific commands (SlashCommand:/commit)
        let fixture = TestFixture::new().await;

        let _ = fixture.create_command("commit", "---\ndescription: Commit\n---\nContent", false).await;

        set_command_permission("commit", false).await;
        assert!(!can_invoke_via_tool("commit").await);

        set_command_permission("commit", true).await;
        assert!(can_invoke_via_tool("commit").await);

        fixture.cleanup().await;
    }

    #[tokio::test]
    async fn test_tool_prefix_matching() {
        // FAILING: Permission rules support prefix matching (SlashCommand:/review-pr:*)
        let fixture = TestFixture::new().await;

        let _ = fixture.create_command("review-pr-quick", "---\ndescription: Quick review\n---\nContent", false).await;
        let _ = fixture.create_command("review-pr-detailed", "---\ndescription: Detailed review\n---\nContent", false).await;

        set_prefix_permission("review-pr", false).await;
        assert!(!can_invoke_via_tool("review-pr-quick").await);
        assert!(!can_invoke_via_tool("review-pr-detailed").await);

        fixture.cleanup().await;
    }

    async fn can_invoke_via_tool(_name: &str) -> bool {
        panic!("NOT IMPLEMENTED: Check if command can be invoked via tool")
    }

    async fn is_tool_discoverable(_name: &str) -> bool {
        panic!("NOT IMPLEMENTED: Check if command is discoverable by tool")
    }

    fn get_slash_command_tool_budget() -> usize {
        panic!("NOT IMPLEMENTED: Get SlashCommand tool character budget")
    }

    async fn set_tool_permission(_tool: &str, _enabled: bool) {
        panic!("NOT IMPLEMENTED: Set tool permission")
    }

    async fn is_tool_enabled(_tool: &str) -> bool {
        panic!("NOT IMPLEMENTED: Check if tool is enabled")
    }

    async fn set_command_permission(_command: &str, _enabled: bool) {
        panic!("NOT IMPLEMENTED: Set command-specific permission")
    }

    async fn set_prefix_permission(_prefix: &str, _enabled: bool) {
        panic!("NOT IMPLEMENTED: Set prefix-based permission")
    }
}

// ============================================================================
// 7. PLUGIN AND MCP COMMANDS
// ============================================================================

mod plugin_mcp_commands {
    use super::*;

    #[test]
    fn test_plugin_command_namespace_format() {
        // FAILING: Plugin commands use /plugin-name:command-name format
        let cmd = "/amplihack:ultrathink";

        let parsed = parse_plugin_command(cmd).unwrap();
        assert_eq!(parsed.plugin_name, "amplihack");
        assert_eq!(parsed.command_name, "ultrathink");
    }

    #[test]
    fn test_plugin_command_parsing() {
        // FAILING: Parse plugin commands correctly
        let cmd = "/my-plugin:do-thing arg1 arg2";

        let parsed = parse_plugin_command(cmd).unwrap();
        assert_eq!(parsed.plugin_name, "my-plugin");
        assert_eq!(parsed.command_name, "do-thing");
        assert_eq!(parsed.args, Some("arg1 arg2".to_string()));
    }

    #[test]
    fn test_mcp_command_format() {
        // FAILING: MCP commands use /mcp__<server>__<prompt> format
        let cmd = "/mcp__github__create-issue";

        let parsed = parse_mcp_command(cmd).unwrap();
        assert_eq!(parsed.server_name, "github");
        assert_eq!(parsed.prompt_name, "create-issue");
    }

    #[test]
    fn test_mcp_command_with_underscores() {
        // FAILING: MCP commands handle underscores in server/prompt names
        let cmd = "/mcp__my_server__my_prompt";

        let parsed = parse_mcp_command(cmd).unwrap();
        assert_eq!(parsed.server_name, "my_server");
        assert_eq!(parsed.prompt_name, "my_prompt");
    }

    #[test]
    fn test_mcp_no_wildcard_permissions() {
        // FAILING: MCP permission rules don't support wildcards
        let permission = "mcp__github__*";
        assert!(!is_valid_mcp_permission(permission));

        let permission = "mcp__github__create-issue";
        assert!(is_valid_mcp_permission(permission));
    }

    #[test]
    fn test_mcp_dynamic_discovery() {
        // FAILING: MCP commands are dynamically discovered from servers
        let commands = discover_mcp_commands("test-server");
        assert!(commands.is_ok());
    }

    #[test]
    fn test_plugin_integration() {
        // FAILING: Plugin commands integrate seamlessly once installed
        let plugin_name = "test-plugin";
        assert!(is_plugin_installed(plugin_name));

        let commands = get_plugin_commands(plugin_name);
        assert!(!commands.is_empty());
    }

    struct PluginCommand {
        plugin_name: String,
        command_name: String,
        args: Option<String>,
    }

    struct McpCommand {
        server_name: String,
        prompt_name: String,
    }

    fn parse_plugin_command(_cmd: &str) -> Result<PluginCommand, String> {
        panic!("NOT IMPLEMENTED: Parse plugin command")
    }

    fn parse_mcp_command(_cmd: &str) -> Result<McpCommand, String> {
        panic!("NOT IMPLEMENTED: Parse MCP command")
    }

    fn is_valid_mcp_permission(_permission: &str) -> bool {
        panic!("NOT IMPLEMENTED: Validate MCP permission")
    }

    fn discover_mcp_commands(_server: &str) -> Result<Vec<String>, String> {
        panic!("NOT IMPLEMENTED: Discover MCP commands")
    }

    fn is_plugin_installed(_name: &str) -> bool {
        panic!("NOT IMPLEMENTED: Check if plugin is installed")
    }

    fn get_plugin_commands(_name: &str) -> Vec<String> {
        panic!("NOT IMPLEMENTED: Get plugin commands")
    }
}

// ============================================================================
// 8. ERROR HANDLING AND EDGE CASES
// ============================================================================

mod error_handling {
    use super::*;

    #[tokio::test]
    async fn test_command_not_found() {
        // FAILING: Proper error when command doesn't exist
        let result = execute_command("nonexistent").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found"));
    }

    #[tokio::test]
    async fn test_malformed_frontmatter() {
        // FAILING: Handle malformed frontmatter gracefully
        let fixture = TestFixture::new().await;
        let content = "---\ninvalid yaml: [unclosed\n---\nContent";

        let _ = fixture.create_command("broken", content, false).await;

        let result = execute_command("broken").await;
        // Should either parse gracefully or return clear error
        if result.is_err() {
            assert!(result.unwrap_err().contains("frontmatter"));
        } else {
            assert!(result.is_ok());
        }

        fixture.cleanup().await;
    }

    #[tokio::test]
    async fn test_missing_closing_frontmatter() {
        // FAILING: Handle missing closing --- in frontmatter
        let fixture = TestFixture::new().await;
        let content = "---\ndescription: No closing marker\nContent";

        let _ = fixture.create_command("unclosed", content, false).await;

        let result = execute_command("unclosed").await;
        assert!(result.is_ok()); // Should use content as-is

        fixture.cleanup().await;
    }

    #[tokio::test]
    async fn test_empty_command_file() {
        // FAILING: Handle empty command files
        let fixture = TestFixture::new().await;
        let _ = fixture.create_command("empty", "", false).await;

        let result = execute_command("empty").await;
        assert!(result.is_ok());

        fixture.cleanup().await;
    }

    #[tokio::test]
    async fn test_command_exceeds_budget() {
        // FAILING: Error when expanded command exceeds character budget
        let fixture = TestFixture::new().await;
        let huge_content = "x".repeat(20_000);

        let _ = fixture.create_command("huge", &huge_content, false).await;

        let result = execute_command_via_tool("huge").await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("budget") || err.contains("limit"));

        fixture.cleanup().await;
    }

    #[tokio::test]
    async fn test_missing_argument() {
        // FAILING: Handle missing positional arguments
        let fixture = TestFixture::new().await;
        let content = "PR #$1 priority $2"; // Expects 2 args

        let _ = fixture.create_command("needs-args", content, false).await;

        // Call with only 1 arg
        let result = execute_command_with_args("needs-args", "123").await;
        // Should either leave $2 as-is or use empty string
        assert!(result.is_ok());

        fixture.cleanup().await;
    }

    #[tokio::test]
    async fn test_special_characters_in_arguments() {
        // FAILING: Handle special characters in arguments
        let fixture = TestFixture::new().await;
        let content = "Message: $1";

        let _ = fixture.create_command("msg", content, false).await;

        let special_chars = vec![
            "hello\nworld",           // newline
            "quote\"inside",          // quote
            "slash/path",             // slash
            "dollar$sign",            // dollar
            "back\\slash",            // backslash
        ];

        for special in special_chars {
            let result = execute_command_with_args("msg", special).await;
            assert!(result.is_ok());
        }

        fixture.cleanup().await;
    }

    #[tokio::test]
    async fn test_file_reference_not_found() {
        // FAILING: Handle file not found in @ reference
        let fixture = TestFixture::new().await;
        let content = "@/nonexistent/file.js";

        let _ = fixture.create_command("missing-file", content, false).await;

        let result = execute_command("missing-file").await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("not found") || err.contains("file"));

        fixture.cleanup().await;
    }

    #[tokio::test]
    async fn test_circular_command_reference() {
        // FAILING: Detect and prevent circular command references
        let fixture = TestFixture::new().await;
        let content_a = "Call /cmd-b";
        let content_b = "Call /cmd-a";

        let _ = fixture.create_command("cmd-a", content_a, false).await;
        let _ = fixture.create_command("cmd-b", content_b, false).await;

        let result = execute_command("cmd-a").await;
        // Should detect circular reference or hit recursion limit
        if result.is_ok() {
            assert!(!result.unwrap().output.is_empty());
        } else {
            assert!(result.is_err());
        }

        fixture.cleanup().await;
    }

    #[tokio::test]
    async fn test_bash_without_permission() {
        // FAILING: Error when bash used without allowed-tools
        let fixture = TestFixture::new().await;
        let content = "!echo 'no permission'";

        let _ = fixture.create_command("unsafe-bash", content, false).await;

        let result = execute_command("unsafe-bash").await;
        assert!(result.is_err());

        fixture.cleanup().await;
    }

    #[derive(Debug)]
    struct CommandResult {
        output: String,
    }

    async fn execute_command(_name: &str) -> Result<CommandResult, String> {
        panic!("NOT IMPLEMENTED: Execute command")
    }

    async fn execute_command_via_tool(_name: &str) -> Result<CommandResult, String> {
        panic!("NOT IMPLEMENTED: Execute command via SlashCommand tool")
    }

    async fn execute_command_with_args(_name: &str, _args: &str) -> Result<CommandResult, String> {
        panic!("NOT IMPLEMENTED: Execute command with args")
    }
}

// ============================================================================
// 9. COMPARISON: SLASH COMMANDS vs SKILLS
// ============================================================================

mod slash_vs_skills {
    #[test]
    fn test_slash_commands_single_file() {
        // FAILING: Slash commands are single-file prompts
        let cmd_structure = get_slash_command_structure();
        assert_eq!(cmd_structure.file_count, 1);
    }

    #[test]
    fn test_slash_commands_explicit_invocation() {
        // FAILING: Slash commands require explicit invocation
        let invocation_type = get_slash_command_invocation_type();
        assert_eq!(invocation_type, InvocationType::Explicit);
    }

    #[test]
    fn test_skills_multi_file() {
        // FAILING: Skills handle multi-file capabilities
        let skill_structure = get_skill_structure();
        assert!(skill_structure.file_count > 1);
    }

    #[test]
    fn test_skills_automatic_discovery() {
        // FAILING: Skills use automatic discovery
        let invocation_type = get_skill_invocation_type();
        assert_eq!(invocation_type, InvocationType::Automatic);
    }

    #[test]
    fn test_use_case_distinction() {
        // FAILING: Clear distinction between slash commands and skills use cases
        let slash_use_case = get_slash_command_use_case();
        assert!(slash_use_case.contains("quick") || slash_use_case.contains("template"));

        let skill_use_case = get_skill_use_case();
        assert!(skill_use_case.contains("complex") || skill_use_case.contains("workflow"));
    }

    #[derive(Debug, PartialEq)]
    enum InvocationType {
        Explicit,
        Automatic,
    }

    struct Structure {
        file_count: usize,
    }

    fn get_slash_command_structure() -> Structure {
        panic!("NOT IMPLEMENTED: Get slash command structure")
    }

    fn get_slash_command_invocation_type() -> InvocationType {
        panic!("NOT IMPLEMENTED: Get slash command invocation type")
    }

    fn get_skill_structure() -> Structure {
        panic!("NOT IMPLEMENTED: Get skill structure")
    }

    fn get_skill_invocation_type() -> InvocationType {
        panic!("NOT IMPLEMENTED: Get skill invocation type")
    }

    fn get_slash_command_use_case() -> String {
        panic!("NOT IMPLEMENTED: Get slash command use case")
    }

    fn get_skill_use_case() -> String {
        panic!("NOT IMPLEMENTED: Get skill use case")
    }
}

// ============================================================================
// 10. PERFORMANCE AND BOUNDARIES
// ============================================================================

mod performance_boundaries {
    use super::*;

    #[tokio::test]
    async fn test_command_loading_performance() {
        // FAILING: Command loading should be fast (< 100ms for simple commands)
        let fixture = TestFixture::new().await;
        let content = "Simple command content";

        let _ = fixture.create_command("fast", content, false).await;

        let start = std::time::Instant::now();
        let _ = load_command("fast").await;
        let elapsed = start.elapsed();

        assert!(elapsed.as_millis() < 100);

        fixture.cleanup().await;
    }

    #[tokio::test]
    async fn test_large_command_library() {
        // FAILING: Support many commands (100+)
        let fixture = TestFixture::new().await;

        for i in 0..100 {
            let content = format!("Command {}", i);
            let _ = fixture.create_command(&format!("cmd{}", i), &content, false).await;
        }

        let all_commands = list_all_commands().await;
        assert!(all_commands.len() >= 100);

        fixture.cleanup().await;
    }

    #[tokio::test]
    async fn test_max_argument_count() {
        // FAILING: Support reasonable number of positional arguments (20+)
        let fixture = TestFixture::new().await;
        let placeholders: Vec<String> = (1..=20).map(|i| format!("${}", i)).collect();
        let content = placeholders.join(" ");

        let _ = fixture.create_command("many-args", &content, false).await;

        let args: Vec<String> = (1..=20).map(|i| format!("arg{}", i)).collect();
        let result = execute_command_with_args("many-args", &args.join(" ")).await;
        assert!(result.is_ok());

        fixture.cleanup().await;
    }

    #[tokio::test]
    async fn test_nested_namespace_depth() {
        // FAILING: Support nested namespace directories
        let fixture = TestFixture::new().await;

        let deep_path = "level1/level2/level3";
        let content = "Deep command";

        let dir = fixture.project_commands_dir.join(deep_path);
        let _ = fs::create_dir_all(&dir).await;
        let _ = fs::write(dir.join("deep.md"), content).await;

        assert!(command_exists_at_path(deep_path, "deep").await);

        fixture.cleanup().await;
    }

    #[tokio::test]
    async fn test_concurrent_command_execution() {
        // FAILING: Commands can be executed concurrently
        let fixture = TestFixture::new().await;

        for i in 0..5 {
            let content = format!("Concurrent command {}", i);
            let _ = fixture.create_command(&format!("concurrent{}", i), &content, false).await;
        }

        let mut handles = vec![];
        for i in 0..5 {
            let name = format!("concurrent{}", i);
            handles.push(tokio::spawn(async move {
                execute_command_async(&name).await
            }));
        }

        for handle in handles {
            assert!(handle.await.is_ok());
        }

        fixture.cleanup().await;
    }

    async fn load_command(_name: &str) -> Result<String, String> {
        panic!("NOT IMPLEMENTED: Load command")
    }

    async fn list_all_commands() -> Vec<String> {
        panic!("NOT IMPLEMENTED: List all commands")
    }

    async fn execute_command_with_args(_name: &str, _args: &str) -> Result<String, String> {
        panic!("NOT IMPLEMENTED: Execute command with args")
    }

    async fn command_exists_at_path(_path: &str, _name: &str) -> bool {
        panic!("NOT IMPLEMENTED: Check command exists at path")
    }

    async fn execute_command_async(_name: &str) -> Result<String, String> {
        panic!("NOT IMPLEMENTED: Execute command asynchronously")
    }
}
