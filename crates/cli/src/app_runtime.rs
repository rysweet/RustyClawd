//! Application runtime methods for RustyClawd.
//!
//! Contains the `App` runtime lifecycle: `run()`, subcommand dispatch,
//! hook execution, interactive mode entry, stdin handling, and session saving.
//! This is a split `impl App` block -- the struct definition lives in main.rs.

use anyhow::{Context as AnyhowContext, Result};
use std::io::{self, IsTerminal, Read};

use super::cli_args::Commands;
use super::{hooks, interactive, mcp_commands, App};

impl App {
    /// Run the application
    pub(crate) async fn run(mut self) -> Result<()> {
        // Handle subcommands first
        if let Some(command) = &self.cli.command {
            return self.run_subcommand(command).await;
        }

        // Perform scheduled update check (if applicable)
        self.check_for_updates_on_startup().await;

        // Call SessionStart hook
        self.execute_session_start_hook().await?;

        // Determine mode: print mode (one-shot) or interactive
        let result = self.determine_and_run_mode().await;

        // Execute Stop hook before session end (checks if work is complete)
        self.execute_stop_hook().await?;

        // Call SessionEnd hook (even on error)
        self.execute_session_end_hook().await?;

        // Save session before exit
        self.save_session()?;

        result
    }

    /// Check for updates on startup (background, non-blocking)
    async fn check_for_updates_on_startup(&self) {
        use rustyclawd::update::GitHubClient;
        use rustyclawd::update::UpdateScheduler;
        use rustyclawd::update::Version;

        tracing::debug!("Checking if scheduled update check is needed");

        // Try to create scheduler and check if update check is needed
        match UpdateScheduler::new() {
            Ok(scheduler) => {
                if !scheduler.should_check_on_startup() {
                    tracing::debug!("Update check not needed at this time");
                    return;
                }

                tracing::info!("Performing scheduled background update check");

                // Spawn background task to perform check
                let current_version = Version::current();
                let client = GitHubClient::new("rysweet", "RustyClawd");

                // We'll do a simple non-blocking check here
                tokio::spawn(async move {
                    match client.get_update_info(&current_version).await {
                        Ok(Some(update_info)) => {
                            tracing::info!(
                                "Update available: {} -> {}",
                                current_version,
                                update_info.latest_version
                            );
                            // Note: In a full implementation, we might show a notification
                            // For now, we just log it
                        }
                        Ok(None) => {
                            tracing::debug!("Already at latest version");
                        }
                        Err(e) => {
                            // Don't warn if there are simply no releases available yet
                            // This is expected for repos that haven't published releases
                            use rustyclawd::update::error::UpdateError;
                            if !matches!(e, UpdateError::NoReleasesAvailable) {
                                tracing::warn!("Background update check failed: {}", e);
                            } else {
                                tracing::debug!("No releases available for update check");
                            }
                        }
                    }
                });
            }
            Err(e) => {
                tracing::warn!("Failed to initialize update scheduler: {}", e);
            }
        }
    }

    /// Run subcommands (update, mcp, agent)
    async fn run_subcommand(&self, command: &Commands) -> Result<()> {
        match command {
            Commands::Update {
                check,
                force,
                rollback,
            } => self.handle_update_command(*check, *force, *rollback).await,
            Commands::Mcp { args } => {
                // Handle MCP commands
                match mcp_commands::handle_cli_command(self.mcp_proxy.clone(), args).await {
                    Ok(output) => {
                        println!("{}", output);
                        Ok(())
                    }
                    Err(e) => {
                        eprintln!("Error: {}", e);
                        std::process::exit(1);
                    }
                }
            }
            Commands::Agent {
                agent_type,
                prompt,
                model,
            } => {
                self.handle_agent_command(agent_type, prompt, model.as_deref())
                    .await
            }
            // Agents and Auth are handled in main() before App initialization
            Commands::Agents | Commands::Auth { .. } => Ok(()),
        }
    }

    /// Handle update command with all subcommands
    async fn handle_update_command(&self, check: bool, force: bool, rollback: bool) -> Result<()> {
        use rustyclawd::update::{
            format_update_message, handle_check_updates, handle_install_update, handle_rollback,
        };

        tracing::info!(
            "Processing update command: check={}, force={}, rollback={}",
            check,
            force,
            rollback
        );

        // Determine which operation to perform
        if rollback {
            // Rollback to previous version
            match handle_rollback().await {
                Ok(result) => {
                    println!("{}", format_update_message(&result));
                    Ok(())
                }
                Err(e) => {
                    eprintln!("Update error: {}", e);
                    Err(e.into())
                }
            }
        } else if check {
            // Check for updates
            match handle_check_updates(force).await {
                Ok(result) => {
                    println!("{}", format_update_message(&result));
                    Ok(())
                }
                Err(e) => {
                    eprintln!("Update error: {}", e);
                    Err(e.into())
                }
            }
        } else {
            // Install update
            match handle_install_update().await {
                Ok(result) => {
                    println!("{}", format_update_message(&result));
                    Ok(())
                }
                Err(e) => {
                    eprintln!("Update error: {}", e);
                    Err(e.into())
                }
            }
        }
    }

    /// Handle agent command - invoke specialized agent with prompt from file
    async fn handle_agent_command(
        &self,
        agent_type: &str,
        prompt_file: &str,
        model: Option<&str>,
    ) -> Result<()> {
        use rustyclawd_tools::{AgentTool, Tool, ToolContext, ToolEvent};

        tracing::info!(
            "Invoking agent: type={}, prompt_file={}, model={:?}",
            agent_type,
            prompt_file,
            model
        );

        // Read prompt from file
        let prompt_content = std::fs::read_to_string(prompt_file)
            .with_context(|| format!("Failed to read prompt file: {}", prompt_file))?;

        // Create tool context
        let ctx = ToolContext {
            cwd: std::env::current_dir().unwrap_or_default(),
            debug: self.cli.verbose,
            metadata: serde_json::Value::Null,
            execution_context: rustyclawd_tools::ExecutionContext::NonInteractive,
            allowed_tools: self.cli.allowed_tools.clone(),
            disallowed_tools: self.cli.disallowed_tools.clone(),
        };

        // Create agent parameters
        let params = rustyclawd_tools::agent::AgentParams {
            description: format!("Agent invocation: {}", agent_type),
            prompt: prompt_content,
            subagent_type: agent_type.to_string(),
            model: model.map(|m| m.to_string()),
            resume: None,
            run_in_background: false,
            memory_scope: None,
        };

        // Execute agent tool
        let tool = AgentTool;
        let mut stream = tool
            .execute(params, &ctx)
            .await
            .with_context(|| format!("Failed to execute agent: {}", agent_type))?;

        // Process stream events
        use futures::StreamExt;
        while let Some(event) = stream.next().await {
            match event {
                ToolEvent::Result(output) => {
                    // Execute SubagentStop hook when agent completes
                    let context = hooks::HookContext::for_session(
                        self.session.id.clone(),
                        format!(".claude/sessions/{}/transcript.json", self.session.id),
                        std::env::current_dir()
                            .ok()
                            .and_then(|p| p.to_str().map(|s| s.to_string()))
                            .unwrap_or_default(),
                        "ask".to_string(),
                        hooks::HookEvent::SubagentStop,
                    );

                    match self
                        .hooks
                        .execute_hooks(hooks::HookEvent::SubagentStop, &context)
                        .await
                    {
                        Ok(results) => {
                            for result in results {
                                if let Some(hook_output) = result.parse_output() {
                                    // Check if hook is blocking subagent completion
                                    if let Some(decision) = hook_output.decision {
                                        if decision == hooks::types::StopDecision::Block {
                                            let reason = hook_output.reason.unwrap_or_else(|| {
                                                "Subagent stop blocked by hook".to_string()
                                            });
                                            return Err(anyhow::anyhow!(
                                                "Subagent completion blocked: {}",
                                                reason
                                            ));
                                        }
                                    }
                                }
                                if !result.is_success() {
                                    tracing::warn!("SubagentStop hook failed: {}", result.stderr);
                                }
                            }
                        }
                        Err(e) => {
                            tracing::warn!("Failed to execute SubagentStop hooks: {}", e);
                            // Non-blocking - continue with output
                        }
                    }

                    // Output the agent response
                    println!("\n=== Agent Response ===\n");
                    println!("{}", output.response);
                    println!("\n=== Metadata ===");
                    println!("Agent ID: {}", output.agent_id);
                    println!("Agent Name: {}", output.agent_name);
                    println!("Model: {}", output.model);
                    println!(
                        "Tokens: {} input, {} output, {} total",
                        output.tokens_used.input_tokens,
                        output.tokens_used.output_tokens,
                        output.tokens_used.total_tokens
                    );
                    return Ok(());
                }
                ToolEvent::Error { message } => {
                    eprintln!("Agent error: {}", message);
                    return Err(anyhow::anyhow!("Agent execution failed: {}", message));
                }
                ToolEvent::Progress { step, percentage } => {
                    if self.cli.verbose {
                        if let Some(pct) = percentage {
                            eprintln!("[{:.0}%] {}", pct, step);
                        } else {
                            eprintln!("{}", step);
                        }
                    }
                }
            }
        }

        Err(anyhow::anyhow!("Agent execution completed without result"))
    }

    /// Determine which mode to run based on CLI arguments and stdin
    async fn determine_and_run_mode(&mut self) -> Result<()> {
        // Handle --list-models early (before any mode selection)
        if self.cli.list_models {
            return self.run_print_mode("").await;
        }

        // If --input-format stream-json, use the SDK bidirectional protocol
        // instead of reading stdin as a raw prompt.
        if self.cli.input_format == "stream-json" {
            return self.run_print_mode_stream_input().await;
        }

        // Check for piped stdin first
        let stdin_input = Self::read_stdin_if_piped()?;

        // Determine if we have a prompt
        let prompt_text = if !self.cli.prompt.is_empty() {
            // Join all positional args as the prompt
            Some(self.cli.prompt.join(" "))
        } else {
            // Use stdin as prompt
            stdin_input.clone()
        };

        // If print_mode flag is set or we have a prompt, run in print mode
        if self.cli.print_mode || prompt_text.is_some() {
            if let Some(prompt) = prompt_text {
                return self.run_print_mode(&prompt).await;
            } else {
                // -p flag with no prompt
                return Err(anyhow::anyhow!("Print mode requires a prompt"));
            }
        }

        // No prompt and no -p flag = interactive mode
        self.run_interactive().await
    }

    /// Read from stdin if it's piped (not a TTY)
    fn read_stdin_if_piped() -> Result<Option<String>> {
        let stdin = io::stdin();

        // Check if stdin is a terminal (TTY) or piped
        if stdin.is_terminal() {
            // It's a TTY, not piped - return None
            return Ok(None);
        }

        // Stdin is piped - read all content
        let mut buffer = String::new();
        stdin.lock().read_to_string(&mut buffer)?;

        if buffer.trim().is_empty() {
            Ok(None)
        } else {
            Ok(Some(buffer.trim().to_string()))
        }
    }

    /// Execute SessionStart hook
    async fn execute_session_start_hook(&self) -> Result<()> {
        let context = hooks::HookContext::for_session(
            self.session.id.clone(),
            format!(".claude/sessions/{}/transcript.json", self.session.id),
            std::env::current_dir()
                .ok()
                .and_then(|p| p.to_str().map(|s| s.to_string()))
                .unwrap_or_default(),
            "ask".to_string(), // Default permission mode
            hooks::HookEvent::SessionStart,
        );

        match self
            .hooks
            .execute_hooks(hooks::HookEvent::SessionStart, &context)
            .await
        {
            Ok(results) => {
                for result in results {
                    if !result.is_success() {
                        tracing::warn!("SessionStart hook failed: {}", result.stderr);
                    }
                }
                Ok(())
            }
            Err(e) => {
                tracing::warn!("Failed to execute SessionStart hooks: {}", e);
                Ok(()) // Don't fail startup if hooks fail
            }
        }
    }

    /// Execute Stop hook before session end
    async fn execute_stop_hook(&self) -> Result<()> {
        let context = hooks::HookContext::for_session(
            self.session.id.clone(),
            format!(".claude/sessions/{}/transcript.json", self.session.id),
            std::env::current_dir()
                .ok()
                .and_then(|p| p.to_str().map(|s| s.to_string()))
                .unwrap_or_default(),
            "ask".to_string(),
            hooks::HookEvent::Stop,
        );

        match self
            .hooks
            .execute_hooks(hooks::HookEvent::Stop, &context)
            .await
        {
            Ok(results) => {
                for result in results {
                    if let Some(output) = result.parse_output() {
                        // Check if hook is blocking session end
                        if let Some(decision) = output.decision {
                            if decision == hooks::types::StopDecision::Block {
                                let reason = output
                                    .reason
                                    .unwrap_or_else(|| "Session end blocked by hook".to_string());
                                tracing::warn!("Stop hook blocked session end: {}", reason);
                                // For non-interactive mode, we still exit but log the warning
                            }
                        }
                    }
                    if !result.is_success() {
                        tracing::warn!("Stop hook failed: {}", result.stderr);
                    }
                }
                Ok(())
            }
            Err(e) => {
                tracing::warn!("Failed to execute Stop hooks: {}", e);
                Ok(()) // Non-blocking - continue with shutdown
            }
        }
    }

    /// Execute SessionEnd hook
    async fn execute_session_end_hook(&self) -> Result<()> {
        let context = hooks::HookContext::for_session(
            self.session.id.clone(),
            format!(".claude/sessions/{}/transcript.json", self.session.id),
            std::env::current_dir()
                .ok()
                .and_then(|p| p.to_str().map(|s| s.to_string()))
                .unwrap_or_default(),
            "ask".to_string(),
            hooks::HookEvent::SessionEnd,
        );

        match self
            .hooks
            .execute_hooks(hooks::HookEvent::SessionEnd, &context)
            .await
        {
            Ok(results) => {
                for result in results {
                    if !result.is_success() {
                        tracing::warn!("SessionEnd hook failed: {}", result.stderr);
                    }
                }
                Ok(())
            }
            Err(e) => {
                tracing::warn!("Failed to execute SessionEnd hooks: {}", e);
                Ok(()) // Don't fail shutdown if hooks fail
            }
        }
    }

    /// Run interactive mode
    async fn run_interactive(&mut self) -> Result<()> {
        use rustyclawd_core::client::Backend;

        // Pass hooks and tool restrictions to interactive session
        let hooks = std::sync::Arc::new(self.hooks.clone());
        let allowed_tools = self.cli.allowed_tools.clone();
        let disallowed_tools = self.cli.disallowed_tools.clone();

        let backend = self
            .cli
            .provider
            .as_deref()
            .map(|p| {
                Backend::from_str_loose(p).ok_or_else(|| {
                    anyhow::anyhow!("Unknown provider '{}'. Use 'anthropic' or 'copilot'.", p)
                })
            })
            .transpose()?
            .unwrap_or(Backend::Anthropic);

        interactive::run_interactive_with_config(
            Some(hooks),
            allowed_tools,
            disallowed_tools,
            backend,
        )
        .await
    }

    /// Save session to disk
    fn save_session(&self) -> Result<()> {
        match self.session_saver.save_session(&self.session) {
            Ok(_) => {
                tracing::debug!("Session saved: {}", self.session.id);
                Ok(())
            }
            Err(e) => {
                tracing::warn!("Failed to save session: {}", e);
                Ok(()) // Don't fail if save fails
            }
        }
    }
}
