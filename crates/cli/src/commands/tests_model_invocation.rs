//! TDD Tests for slash command model invocation feature (Issue #126)
//!
//! Tests the ability for slash commands to declare whether they should be
//! intercepted and executed locally or passed through to Claude for model invocation.
//!
//! Testing pyramid: 60% unit, 30% integration, 10% E2E
//!
//! Note: This module is #[cfg(test)] - test-only code

use super::*;
use crate::commands::loader::{FrontMatter, LoadedCommand};
use crate::commands::registry::Registry;
use std::path::PathBuf;

// =============================================================================
// UNIT TESTS (60%) - Test individual components
// =============================================================================

mod unit_tests {
    use super::*;

    #[test]
    fn test_get_command_metadata_returns_none_for_nonexistent_command() {
        // This test will fail until we implement get_command_metadata()
        let slash_commands = SlashCommands {
            parser: CommandParser::new(),
            registry: Registry::new(PathBuf::from(".test")),
            executor: Executor::new(),
        };

        let metadata = slash_commands.get_command_metadata("nonexistent");
        assert!(metadata.is_none());
    }

    #[test]
    fn test_get_command_metadata_returns_default_true_for_disable_model_invocation() {
        // This test will fail until we implement get_command_metadata()
        // and CommandMetadata struct
        let mut registry = Registry::new(PathBuf::from(".test"));

        // Register a command WITHOUT disable-model-invocation field
        let cmd = LoadedCommand {
            name: "local-command".to_string(),
            frontmatter: FrontMatter {
                description: Some("A local command".to_string()),
                ..Default::default()
            },
            content: "Execute locally".to_string(),
        };
        registry.register(cmd).unwrap();

        let slash_commands = SlashCommands {
            parser: CommandParser::new(),
            registry,
            executor: Executor::new(),
        };

        let metadata = slash_commands.get_command_metadata("local-command");
        assert!(metadata.is_some());

        let meta = metadata.unwrap();
        // Default should be true (local execution)
        assert!(meta.disable_model_invocation);
    }

    #[test]
    fn test_get_command_metadata_returns_explicit_false_value() {
        // This test will fail until CommandMetadata struct is created
        let mut registry = Registry::new(PathBuf::from(".test"));

        // Register a command WITH disable-model-invocation: false
        let cmd = LoadedCommand {
            name: "model-invokable".to_string(),
            frontmatter: FrontMatter {
                description: Some("Can be invoked by model".to_string()),
                disable_model_invocation: Some(false),
                ..Default::default()
            },
            content: "Pass to model".to_string(),
        };
        registry.register(cmd).unwrap();

        let slash_commands = SlashCommands {
            parser: CommandParser::new(),
            registry,
            executor: Executor::new(),
        };

        let metadata = slash_commands.get_command_metadata("model-invokable");
        assert!(metadata.is_some());

        let meta = metadata.unwrap();
        assert!(!meta.disable_model_invocation);
    }

    #[test]
    fn test_get_command_metadata_returns_explicit_true_value() {
        // This test will fail until CommandMetadata struct is created
        let mut registry = Registry::new(PathBuf::from(".test"));

        // Register a command WITH disable-model-invocation: true
        let cmd = LoadedCommand {
            name: "explicitly-local".to_string(),
            frontmatter: FrontMatter {
                description: Some("Explicitly local only".to_string()),
                disable_model_invocation: Some(true),
                ..Default::default()
            },
            content: "Must execute locally".to_string(),
        };
        registry.register(cmd).unwrap();

        let slash_commands = SlashCommands {
            parser: CommandParser::new(),
            registry,
            executor: Executor::new(),
        };

        let metadata = slash_commands.get_command_metadata("explicitly-local");
        assert!(metadata.is_some());

        let meta = metadata.unwrap();
        assert!(meta.disable_model_invocation);
    }

    #[test]
    fn test_command_metadata_contains_description() {
        // This test will fail until CommandMetadata includes description field
        let mut registry = Registry::new(PathBuf::from(".test"));

        let cmd = LoadedCommand {
            name: "test-command".to_string(),
            frontmatter: FrontMatter {
                description: Some("Test description".to_string()),
                disable_model_invocation: Some(false),
                ..Default::default()
            },
            content: "Content".to_string(),
        };
        registry.register(cmd).unwrap();

        let slash_commands = SlashCommands {
            parser: CommandParser::new(),
            registry,
            executor: Executor::new(),
        };

        let metadata = slash_commands.get_command_metadata("test-command").unwrap();
        assert_eq!(metadata.description.as_deref(), Some("Test description"));
    }
}

// =============================================================================
// INTEGRATION TESTS (30%) - Test component interactions
// =============================================================================

mod integration_tests {
    use super::*;

    #[test]
    fn test_interception_check_with_disable_model_invocation_true() {
        // This test will fail until we implement should_intercept_locally()
        // or equivalent method in interactive.rs
        let mut registry = Registry::new(PathBuf::from(".test"));

        let cmd = LoadedCommand {
            name: "local-only".to_string(),
            frontmatter: FrontMatter {
                description: Some("Local execution only".to_string()),
                disable_model_invocation: Some(true),
                ..Default::default()
            },
            content: "Execute locally".to_string(),
        };
        registry.register(cmd).unwrap();

        let slash_commands = SlashCommands {
            parser: CommandParser::new(),
            registry,
            executor: Executor::new(),
        };

        // Command with disable_model_invocation: true should be intercepted
        let should_intercept = slash_commands.should_intercept_locally("local-only");
        assert!(should_intercept);
    }

    #[test]
    fn test_interception_check_with_disable_model_invocation_false() {
        // This test will fail until we implement should_intercept_locally()
        let mut registry = Registry::new(PathBuf::from(".test"));

        let cmd = LoadedCommand {
            name: "model-command".to_string(),
            frontmatter: FrontMatter {
                description: Some("Can be invoked by model".to_string()),
                disable_model_invocation: Some(false),
                ..Default::default()
            },
            content: "Pass to model".to_string(),
        };
        registry.register(cmd).unwrap();

        let slash_commands = SlashCommands {
            parser: CommandParser::new(),
            registry,
            executor: Executor::new(),
        };

        // Command with disable_model_invocation: false should NOT be intercepted
        let should_intercept = slash_commands.should_intercept_locally("model-command");
        assert!(!should_intercept);
    }

    #[test]
    fn test_interception_defaults_to_local_when_field_absent() {
        // This test will fail until we implement should_intercept_locally()
        let mut registry = Registry::new(PathBuf::from(".test"));

        let cmd = LoadedCommand {
            name: "default-command".to_string(),
            frontmatter: FrontMatter {
                description: Some("No disable field".to_string()),
                ..Default::default()
            },
            content: "Content".to_string(),
        };
        registry.register(cmd).unwrap();

        let slash_commands = SlashCommands {
            parser: CommandParser::new(),
            registry,
            executor: Executor::new(),
        };

        // Commands without the field should default to local interception
        let should_intercept = slash_commands.should_intercept_locally("default-command");
        assert!(should_intercept);
    }

    #[test]
    fn test_unknown_commands_pass_through() {
        // This test will fail until we implement should_intercept_locally()
        let slash_commands = SlashCommands {
            parser: CommandParser::new(),
            registry: Registry::new(PathBuf::from(".test")),
            executor: Executor::new(),
        };

        // Unknown commands should NOT be intercepted (pass through to model)
        let should_intercept = slash_commands.should_intercept_locally("unknown");
        assert!(!should_intercept);
    }

    #[tokio::test]
    async fn test_registry_discovery_preserves_disable_model_invocation() {
        // This test will fail until FrontMatter properly deserializes the field
        
        use tokio::fs;

        let temp_dir = std::env::temp_dir().join("test_commands_126");
        fs::create_dir_all(&temp_dir).await.unwrap();

        // Create a test command file with disable-model-invocation: false
        let command_file = temp_dir.join("test.md");
        let content = r#"---
description: Test command
disable-model-invocation: false
---
Test content
"#;
        fs::write(&command_file, content).await.unwrap();

        // Discover commands
        let registry = Registry::discover(temp_dir.clone()).await.unwrap();

        // Verify the field was preserved
        let cmd = registry.get("test").unwrap();
        assert!(matches!(
            cmd.frontmatter.disable_model_invocation,
            Some(false)
        ));

        // Cleanup
        fs::remove_dir_all(&temp_dir).await.ok();
    }
}

// =============================================================================
// E2E TESTS (10%) - Test complete workflows
// =============================================================================

mod e2e_tests {
    use super::*;

    #[tokio::test]
    async fn test_full_flow_model_invokable_command() {
        // This test will fail until the full integration is complete
        // Tests the complete flow from command file to interception decision
        use tokio::fs;

        let temp_dir = std::env::temp_dir().join("test_e2e_model_invoke");
        fs::create_dir_all(&temp_dir).await.unwrap();

        // Create a model-invokable command
        let command_file = temp_dir.join("ai-review.md");
        let content = r#"---
description: Review code with AI assistance
disable-model-invocation: false
---
Please review the following code: $ARGUMENTS
"#;
        fs::write(&command_file, content).await.unwrap();

        // Initialize slash command system
        let slash_commands = SlashCommands::with_commands_dir(temp_dir.clone())
            .await
            .unwrap();

        // Verify command exists
        assert!(slash_commands.has_command("ai-review"));

        // Verify metadata
        let metadata = slash_commands.get_command_metadata("ai-review").unwrap();
        assert!(!metadata.disable_model_invocation);

        // Verify interception behavior
        let should_intercept = slash_commands.should_intercept_locally("ai-review");
        assert!(!should_intercept); // Should pass through to model

        // Cleanup
        fs::remove_dir_all(&temp_dir).await.ok();
    }

    #[tokio::test]
    async fn test_full_flow_local_only_command() {
        // This test will fail until the full integration is complete
        use tokio::fs;

        let temp_dir = std::env::temp_dir().join("test_e2e_local_only");
        fs::create_dir_all(&temp_dir).await.unwrap();

        // Create a local-only command
        let command_file = temp_dir.join("local-test.md");
        let content = r#"---
description: Local execution only
disable-model-invocation: true
---
Run tests locally: $ARGUMENTS
"#;
        fs::write(&command_file, content).await.unwrap();

        // Initialize slash command system
        let slash_commands = SlashCommands::with_commands_dir(temp_dir.clone())
            .await
            .unwrap();

        // Verify command exists
        assert!(slash_commands.has_command("local-test"));

        // Verify metadata
        let metadata = slash_commands.get_command_metadata("local-test").unwrap();
        assert!(metadata.disable_model_invocation);

        // Verify interception behavior
        let should_intercept = slash_commands.should_intercept_locally("local-test");
        assert!(should_intercept); // Should be intercepted and executed locally

        // Cleanup
        fs::remove_dir_all(&temp_dir).await.ok();
    }

    #[tokio::test]
    async fn test_backward_compatibility_default_behavior() {
        // This test will fail until full integration is complete
        // Ensures existing commands without the field work as before
        use tokio::fs;

        let temp_dir = std::env::temp_dir().join("test_e2e_compat");
        fs::create_dir_all(&temp_dir).await.unwrap();

        // Create an old-style command without disable-model-invocation
        let command_file = temp_dir.join("legacy.md");
        let content = r#"---
description: Legacy command
---
This is a legacy command
"#;
        fs::write(&command_file, content).await.unwrap();

        // Initialize slash command system
        let slash_commands = SlashCommands::with_commands_dir(temp_dir.clone())
            .await
            .unwrap();

        // Verify backward compatibility - should default to local execution
        let metadata = slash_commands.get_command_metadata("legacy").unwrap();
        assert!(metadata.disable_model_invocation); // Default

        let should_intercept = slash_commands.should_intercept_locally("legacy");
        assert!(should_intercept); // Should intercept (backward compatible)

        // Cleanup
        fs::remove_dir_all(&temp_dir).await.ok();
    }
}
