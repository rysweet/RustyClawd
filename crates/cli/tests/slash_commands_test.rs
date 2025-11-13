use claude_code_cli::commands::SlashCommands;

#[tokio::test]
async fn test_slash_commands_discovery() {
    // Initialize slash commands
    let slash_commands = SlashCommands::new().await.expect("Failed to initialize SlashCommands");

    // Get list of available commands
    let commands = slash_commands.list_commands();

    println!("Discovered commands:");
    for cmd in &commands {
        println!("  /{}", cmd);
    }

    // Verify we found some commands
    assert!(
        !commands.is_empty(),
        "Expected to find some slash commands in .claude/commands/"
    );

    // Check for specific test commands
    assert!(
        commands.contains(&"ultrathink".to_string()),
        "Expected to find 'ultrathink' command"
    );
    assert!(
        commands.contains(&"analyze".to_string()),
        "Expected to find 'analyze' command"
    );
    assert!(
        commands.contains(&"debug".to_string()),
        "Expected to find 'debug' command"
    );

    println!("\nAll tests passed! {} commands discovered.", commands.len());
}
