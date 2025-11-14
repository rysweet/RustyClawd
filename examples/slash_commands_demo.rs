//! Demonstration of the Slash Command Execution System
//!
//! This example shows how to:
//! 1. Create a slash command system
//! 2. Register custom commands
//! 3. Execute commands with arguments
//! 4. Handle built-in commands
//! 5. Track character budget

use rustyclawd::commands::*;
use std::path::PathBuf;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("╔════════════════════════════════════════════════╗");
    println!("║   Slash Command Execution System Demo         ║");
    println!("╚════════════════════════════════════════════════╝\n");

    // ========================================================================
    // SECTION 1: Create and discover commands
    // ========================================================================
    println!("1. Creating command system...");
    let registry = Registry::new(PathBuf::from(".demo_commands"));
    println!("   Registry created: {} commands available\n", registry.command_count());

    // ========================================================================
    // SECTION 2: Register some custom commands
    // ========================================================================
    println!("2. Registering custom commands...");

    let mut registry = registry;

    // Command 1: Review PR
    let review_cmd = CommandLoader::new().parse_frontmatter(
        "---\n\
         description: Review a pull request\n\
         model: claude-sonnet-4-5\n\
         ---\n\
         Please review pull request #{0} for the following aspects:\n\
         - Code quality and style\n\
         - Performance implications\n\
         - Security considerations\n\
         - Test coverage\n\
         - Documentation"
    ).unwrap();

    let cmd1 = rustyclawd::commands::loader::LoadedCommand {
        name: "review-pr".to_string(),
        frontmatter: review_cmd.0,
        content: review_cmd.1,
    };
    registry.register(cmd1)?;
    println!("   ✓ Registered: /review-pr <number>");

    // Command 2: Analyze code
    let cmd2 = rustyclawd::commands::loader::LoadedCommand {
        name: "analyze".to_string(),
        frontmatter: rustyclawd::commands::loader::FrontMatter::default(),
        content: "Analyze {0} for issues and suggest improvements".to_string(),
    };
    registry.register(cmd2)?;
    println!("   ✓ Registered: /analyze <file>");

    // Command 3: Document function
    let cmd3 = rustyclawd::commands::loader::LoadedCommand {
        name: "doc".to_string(),
        frontmatter: rustyclawd::commands::loader::FrontMatter {
            description: Some("Generate documentation".to_string()),
            ..Default::default()
        },
        content: "Generate comprehensive documentation for: {{args}}".to_string(),
    };
    registry.register(cmd3)?;
    println!("   ✓ Registered: /doc <items...>\n");

    // ========================================================================
    // SECTION 3: List all commands
    // ========================================================================
    println!("3. Available commands:");
    for cmd in registry.list_commands() {
        println!("   - /{}", cmd);
    }
    println!();

    // ========================================================================
    // SECTION 4: Execute commands
    // ========================================================================
    println!("4. Executing commands...\n");

    let executor = Executor::new();
    let parser = CommandParser::new();

    // Example 1: Execute /review-pr
    println!("   Command: /review-pr 789");
    let cmd = parser.parse("/review-pr 789")?;
    let result = executor.execute(&cmd, &registry).await?;
    println!("   Expanded (first 80 chars):");
    println!("   {}...\n", &result.expanded_prompt[..80.min(result.expanded_prompt.len())]);

    // Example 2: Execute /analyze with file
    println!("   Command: /analyze src/main.rs");
    let cmd = parser.parse("/analyze src/main.rs")?;
    let result = executor.execute(&cmd, &registry).await?;
    println!("   Result: {}\n", result.expanded_prompt);

    // Example 3: Execute /doc with multiple items
    println!("   Command: /doc function_name class_name module_name");
    let cmd = parser.parse("/doc function_name class_name module_name")?;
    let result = executor.execute(&cmd, &registry).await?;
    println!("   Result: {}\n", result.expanded_prompt);

    // ========================================================================
    // SECTION 5: Built-in commands
    // ========================================================================
    println!("5. Built-in commands:");

    for builtin_name in &["help", "exit", "clear"] {
        println!("   Command: /{}", builtin_name);
        let cmd = parser.parse(&format!("/{}", builtin_name))?;
        let result = executor.execute(&cmd, &registry).await?;
        println!("   Is built-in: {}", result.is_builtin);
        println!("   Response (first 60 chars): {}...\n",
            &result.expanded_prompt[..60.min(result.expanded_prompt.len())]);
    }

    // ========================================================================
    // SECTION 6: Character budget tracking
    // ========================================================================
    println!("6. Character budget tracking:");

    let cmd = parser.parse("/review-pr 999")?;
    let result = executor.execute(&cmd, &registry).await?;

    println!("   Expanded prompt size: {} characters", result.char_count());
    println!("   Budget limit: {} characters", commands::MAX_EXPANDED_CHARS);
    println!("   Budget used: {:.1}%", result.budget_percentage());
    println!("   Within budget: {}\n", result.is_within_budget());

    // ========================================================================
    // SECTION 7: Error handling
    // ========================================================================
    println!("7. Error handling examples:");

    // Invalid format
    match parser.parse("help") {
        Err(e) => println!("   ✓ Caught missing slash: {}", e),
        _ => println!("   ✗ Should have failed on missing slash"),
    }

    // Command not found
    let cmd = parser.parse("/nonexistent")?;
    match executor.execute(&cmd, &registry).await {
        Err(e) => println!("   ✓ Caught missing command: {}", e),
        _ => println!("   ✗ Should have failed on missing command"),
    }

    // ========================================================================
    // SECTION 8: Parser features
    // ========================================================================
    println!("\n8. Advanced parser features:");

    // Namespace support
    let cmd = parser.parse("/amplihack:ultrathink test mode")?;
    println!("   ✓ Namespace command: name={}, args={:?}", cmd.name, cmd.args);

    // Multiple arguments
    let cmd = parser.parse("/complex arg1 arg2 arg3 arg4 arg5")?;
    println!("   ✓ Multiple args: {} arguments parsed", cmd.args.len());

    // Command info
    println!("\n9. Command information:");
    println!("{}", registry.list_all_info());

    println!("╔════════════════════════════════════════════════╗");
    println!("║             Demo Complete!                    ║");
    println!("╚════════════════════════════════════════════════╝");

    Ok(())
}
