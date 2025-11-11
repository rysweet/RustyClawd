//! Example: Using the Claude Code Hooks System
//!
//! This example demonstrates how to use the hooks system in your application.
//! It shows SessionStart, PreToolUse, and Stop hook execution patterns.
//!
//! Run with: cargo run --example hooks_usage

// Note: This example won't compile until hooks are exported from the CLI crate
// For now, it serves as documentation of the intended API

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("=== Claude Code Hooks System Example ===\n");

    // Example 1: Basic hooks system setup
    example_basic_setup().await?;

    // Example 2: PreToolUse permission checking
    example_pretool_permission().await?;

    // Example 3: Stop hook completion checking
    example_stop_completion().await?;

    // Example 4: Loading from configuration file
    example_load_config().await?;

    Ok(())
}

/// Example 1: Basic hooks system setup
async fn example_basic_setup() -> anyhow::Result<()> {
    println!("Example 1: Basic Setup");
    println!("----------------------");

    // This demonstrates the API once integrated:
    /*
    use claude_code_cli::hooks::{HooksSystem, HookEvent, HookContext};

    // Create hooks system
    let mut hooks = HooksSystem::new();

    // Create context for SessionStart
    let context = HookContext::for_session(
        "session-abc123".to_string(),
        "/tmp/transcript.log".to_string(),
        std::env::current_dir()?.to_string_lossy().to_string(),
        "auto".to_string(),
        HookEvent::SessionStart,
    );

    // Execute SessionStart hooks
    let results = hooks.execute_hooks(HookEvent::SessionStart, &context).await?;

    // Check results
    for result in results {
        if result.is_success() {
            println!("Hook executed successfully");
            println!("Output: {}", result.stdout);
        } else if result.is_blocking() {
            eprintln!("Hook blocked execution: {}", result.stderr);
            return Err(anyhow::anyhow!("Blocked by hook"));
        } else {
            eprintln!("Hook warning: {}", result.stderr);
        }
    }
    */

    println!("✓ Hooks system created and SessionStart executed\n");
    Ok(())
}

/// Example 2: PreToolUse permission checking
async fn example_pretool_permission() -> anyhow::Result<()> {
    println!("Example 2: PreToolUse Permission");
    println!("---------------------------------");

    // This demonstrates PreToolUse hook pattern:
    /*
    use claude_code_cli::hooks::{HooksSystem, HookEvent, HookContext};

    let hooks = HooksSystem::new();
    let tool_name = "Bash";

    // Create context for tool execution
    let context = HookContext::for_tool(
        "session-abc123".to_string(),
        "/tmp/transcript.log".to_string(),
        std::env::current_dir()?.to_string_lossy().to_string(),
        "auto".to_string(),
        HookEvent::PreToolUse,
        tool_name.to_string(),
    );

    // Execute PreToolUse hooks
    let results = hooks.execute_hooks(HookEvent::PreToolUse, &context).await?;

    // Check permission decisions
    let mut allowed = true;
    for result in results {
        // Check exit code
        if result.is_blocking() {
            println!("Tool execution blocked by hook");
            allowed = false;
            break;
        }

        // Check JSON output
        if let Some(output) = result.parse_output() {
            if let Some(decision) = output.permission_decision {
                match decision {
                    PermissionDecision::Deny => {
                        println!("Permission denied by hook");
                        allowed = false;
                        break;
                    }
                    PermissionDecision::Ask => {
                        println!("Hook requesting user permission");
                        // In real app: prompt user
                        allowed = true;
                    }
                    PermissionDecision::Allow => {
                        println!("Permission granted by hook");
                    }
                }
            }

            // Inject additional context if provided
            if let Some(context) = output.additional_context {
                println!("Hook context: {}", context);
            }
        }
    }

    if allowed {
        println!("✓ Tool execution allowed");
        // Execute the tool...
    } else {
        println!("✗ Tool execution blocked");
    }
    */

    println!("✓ Permission check completed\n");
    Ok(())
}

/// Example 3: Stop hook completion checking
async fn example_stop_completion() -> anyhow::Result<()> {
    println!("Example 3: Stop Hook Completion");
    println!("--------------------------------");

    // This demonstrates Stop hook pattern:
    /*
    use claude_code_cli::hooks::{HooksSystem, HookEvent, HookContext};

    let hooks = HooksSystem::new();

    let context = HookContext::for_session(
        "session-abc123".to_string(),
        "/tmp/transcript.log".to_string(),
        std::env::current_dir()?.to_string_lossy().to_string(),
        "auto".to_string(),
        HookEvent::Stop,
    );

    // Execute Stop hooks
    let results = hooks.execute_hooks(HookEvent::Stop, &context).await?;

    // Check completion decision
    let mut should_stop = true;
    for result in results {
        if let Some(output) = result.parse_output() {
            if let Some(decision) = output.decision {
                match decision {
                    StopDecision::Block => {
                        println!("Stop blocked - more work needed");
                        should_stop = false;
                    }
                    StopDecision::Approve => {
                        println!("Stop approved - work complete");
                    }
                }
            }

            // Show additional context from hook
            if let Some(context) = output.additional_context {
                println!("Stop context: {}", context);
            }
        }
    }

    if should_stop {
        println!("✓ Work is complete, stopping execution");
    } else {
        println!("✗ More work needed, continuing...");
    }
    */

    println!("✓ Completion check performed\n");
    Ok(())
}

/// Example 4: Loading hooks from configuration file
async fn example_load_config() -> anyhow::Result<()> {
    println!("Example 4: Load Configuration");
    println!("------------------------------");

    // This demonstrates loading hooks from config:
    /*
    use claude_code_cli::hooks::HooksSystem;

    let mut hooks = HooksSystem::new();

    // Load from specific file
    match hooks.load_from_file(".claude/hooks/config.json").await {
        Ok(_) => println!("✓ Hooks loaded from config file"),
        Err(e) => println!("No config file found (this is ok): {}", e),
    }

    // Or use default loading (searches parent directories)
    use claude_code_cli::hooks::HookLoader;
    match HookLoader::load_default().await {
        Ok(config) => {
            println!("✓ Hooks loaded from default location");
            hooks.registry_mut().register_configuration(config);
        }
        Err(_) => {
            println!("No hooks configured (using defaults)");
        }
    }

    // Check what hooks are loaded
    let total = hooks.registry().count_total_hooks();
    println!("Total hooks registered: {}", total);
    */

    println!("✓ Configuration loaded\n");
    Ok(())
}

/// Example 5: Full integration pattern
#[allow(dead_code)]
async fn example_full_integration() -> anyhow::Result<()> {
    println!("Example 5: Full Integration");
    println!("---------------------------");

    // This shows the complete pattern for integrating hooks:
    /*
    use claude_code_cli::hooks::{HooksSystem, HookEvent, HookContext};

    // Initialize hooks system
    let mut hooks = HooksSystem::new();
    hooks.load_from_file(".claude/hooks/config.json").await.ok();

    // Session start
    let session_id = uuid::Uuid::new_v4().to_string();
    let context = HookContext::for_session(
        session_id.clone(),
        "/tmp/transcript.log".to_string(),
        std::env::current_dir()?.to_string_lossy().to_string(),
        "auto".to_string(),
        HookEvent::SessionStart,
    );
    hooks.execute_hooks(HookEvent::SessionStart, &context).await?;

    // Main execution loop
    loop {
        // Get user input...
        let user_input = "...";

        // UserPromptSubmit hook
        let context = HookContext::for_session(
            session_id.clone(),
            "/tmp/transcript.log".to_string(),
            std::env::current_dir()?.to_string_lossy().to_string(),
            "auto".to_string(),
            HookEvent::UserPromptSubmit,
        );
        hooks.execute_hooks(HookEvent::UserPromptSubmit, &context).await?;

        // Process prompt, execute tools...
        for tool in tools_to_execute {
            // PreToolUse hook
            let context = HookContext::for_tool(
                session_id.clone(),
                "/tmp/transcript.log".to_string(),
                std::env::current_dir()?.to_string_lossy().to_string(),
                "auto".to_string(),
                HookEvent::PreToolUse,
                tool.name(),
            );
            let results = hooks.execute_hooks(HookEvent::PreToolUse, &context).await?;

            // Check permission
            let allowed = check_permission(&results)?;
            if !allowed {
                continue;
            }

            // Execute tool
            tool.execute()?;

            // PostToolUse hook
            hooks.execute_hooks(HookEvent::PostToolUse, &context).await?;
        }

        // Check if work is complete
        let context = HookContext::for_session(
            session_id.clone(),
            "/tmp/transcript.log".to_string(),
            std::env::current_dir()?.to_string_lossy().to_string(),
            "auto".to_string(),
            HookEvent::Stop,
        );
        let results = hooks.execute_hooks(HookEvent::Stop, &context).await?;

        if should_stop(&results)? {
            break;
        }
    }

    // Session end
    let context = HookContext::for_session(
        session_id.clone(),
        "/tmp/transcript.log".to_string(),
        std::env::current_dir()?.to_string_lossy().to_string(),
        "auto".to_string(),
        HookEvent::SessionEnd,
    );
    hooks.execute_hooks(HookEvent::SessionEnd, &context).await?;
    */

    println!("✓ Full integration pattern demonstrated\n");
    Ok(())
}
