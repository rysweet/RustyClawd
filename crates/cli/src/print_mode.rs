//! Print mode (one-shot execution) for RustyClawd.
//!
//! Contains `App::run_print_mode()` which handles `--print` / `-p` prompt
//! execution against the Anthropic API with tool use, fallback models,
//! system prompt overrides, and hook integration.

use anyhow::{Context as AnyhowContext, Result};

use super::{hooks, permission_mode, tool_definitions, tool_executor, App};

impl App {
    /// Run in print mode (one-shot execution) - matches Claude Code's behavior
    pub(crate) async fn run_print_mode(&mut self, prompt: &str) -> Result<()> {
        use rustyclawd_core::client::{
            Client, Config, CreateMessageRequest, Message as ApiMessage,
        };

        // Load API configuration
        let config = Config::from_default_location().await?;
        let client = Client::new(config)?;

        // Execute UserPromptSubmit hook BEFORE processing prompt
        let context = hooks::HookContext::for_user_prompt(
            self.session.id.clone(),
            format!(".claude/sessions/{}/transcript.json", self.session.id),
            std::env::current_dir()
                .ok()
                .and_then(|p| p.to_str().map(|s| s.to_string()))
                .unwrap_or_default(),
            "ask".to_string(),
            prompt.to_string(),
        );

        match self
            .hooks
            .execute_hooks(hooks::HookEvent::UserPromptSubmit, &context)
            .await
        {
            Ok(results) => {
                for result in results {
                    if result.is_blocking() {
                        return Err(anyhow::anyhow!("Prompt blocked by hook: {}", result.stderr));
                    }
                    if !result.is_success() {
                        eprintln!(
                            "\u{26a0}\u{fe0f}  Warning: UserPromptSubmit hook failed: {}",
                            result.stderr
                        );
                    }
                }
            }
            Err(e) => {
                eprintln!(
                    "\u{26a0}\u{fe0f}  Warning: Failed to execute UserPromptSubmit hooks: {}",
                    e
                );
                // Non-blocking - continue even if hook fails
            }
        }

        // Model configuration - use CLI override or default
        let model = self
            .cli
            .model
            .as_ref()
            .map(|m| match m.as_str() {
                "sonnet" => "claude-sonnet-4-6",
                "opus" => "claude-opus-4-6",
                "haiku" => "claude-haiku-4-5-20251001",
                custom => custom,
            })
            .unwrap_or("claude-sonnet-4-6")
            .to_string();

        // Fallback model configuration (if specified)
        let fallback_model = self
            .cli
            .fallback_model
            .as_ref()
            .map(|m| match m.as_str() {
                "sonnet" => "claude-sonnet-4-6",
                "opus" => "claude-opus-4-6",
                "haiku" => "claude-haiku-4-5-20251001",
                custom => custom,
            })
            .map(|s| s.to_string());

        let max_tokens = 4096u32; // Default max tokens (not configurable in official spec)

        // Build system prompt based on priority:
        // 1. CLAUDE_CODE_SIMPLE mode -> minimal system prompt
        // 2. --system-prompt > --system-prompt-file > --append-system-prompt
        let system_prompt = if rustyclawd_core::simple_mode::is_active() {
            // Simple mode uses a minimal system prompt, ignoring CLI overrides
            Some(rustyclawd_core::simple_mode::MINIMAL_SYSTEM_PROMPT.to_string())
        } else if let Some(ref prompt) = self.cli.system_prompt {
            // --system-prompt: replace entire system prompt
            Some(prompt.clone())
        } else if let Some(ref file_path) = self.cli.system_prompt_file {
            // --system-prompt-file: load from file
            Some(
                std::fs::read_to_string(file_path)
                    .with_context(|| format!("Failed to read system prompt file: {}", file_path))?,
            )
        } else if let Some(ref _append) = self.cli.append_system_prompt {
            // --append-system-prompt: append to default (would need default system prompt)
            // For now, just use the append text
            self.cli.append_system_prompt.clone()
        } else {
            None
        };

        // Create message request
        let messages = vec![ApiMessage::user(prompt.to_string())];
        let mut request = CreateMessageRequest::new(model, messages, max_tokens);

        // Apply system prompt if specified
        if let Some(ref sys_prompt) = system_prompt {
            request = request.with_system(sys_prompt.clone());
        }

        // Add tools (always enabled in official spec, controlled by allowedTools/disallowedTools)
        let tools = tool_definitions::get_all_tool_definitions();
        request = request.with_tools(tools);

        // Parse model capabilities if provided (JSON format)
        if let Some(ref capabilities_json) = self.cli.model_capabilities {
            tracing::info!("Applying custom model capabilities");
            // Parse and log capabilities (in real implementation would apply to request)
            match serde_json::from_str::<serde_json::Value>(capabilities_json) {
                Ok(caps) => tracing::debug!("Model capabilities: {:?}", caps),
                Err(e) => tracing::warn!("Failed to parse model capabilities: {}", e),
            }
        }

        // Use the tool execution loop (tools always enabled in official spec)
        // Pass hooks, session ID, and permission mode to tool executor
        let hooks_for_tools = std::sync::Arc::new(self.hooks.clone());
        let session_id_for_tools = self.session.id.clone();

        // Parse permission mode from CLI
        let permission_mode = self
            .cli
            .permission_mode
            .as_ref()
            .map(|s| match s.as_str() {
                "plan" => permission_mode::PermissionMode::Plan,
                "auto-accept" | "auto" => permission_mode::PermissionMode::AutoAccept,
                _ => permission_mode::PermissionMode::Ask,
            })
            .unwrap_or(permission_mode::PermissionMode::Ask);

        // Clone allowed/disallowed tools from CLI for tool executor
        let allowed_tools_for_executor = self.cli.allowed_tools.clone();
        let disallowed_tools_for_executor = self.cli.disallowed_tools.clone();

        let response = match client
            .execute_with_tools(request.clone(), |tool_name, tool_input| {
                let hooks = hooks_for_tools.clone();
                let session_id = session_id_for_tools.clone();
                let allowed_tools = allowed_tools_for_executor.clone();
                let disallowed_tools = disallowed_tools_for_executor.clone();
                async move {
                    tool_executor::execute_tool_with_permission(
                        tool_name,
                        tool_input,
                        permission_mode,
                        tool_executor::ToolExecutionParams {
                            hooks: Some(hooks),
                            session_id: Some(session_id),
                            notification_manager: None, // No notification manager in non-interactive mode
                            tool_use_id: None,          // No tool_use_id in non-interactive mode
                            allowed_tools,
                            disallowed_tools,
                        },
                    )
                    .await
                }
            })
            .await
        {
            Ok(resp) => resp,
            Err(e) => {
                if let Some(fallback) = fallback_model {
                    // Try fallback model if primary fails
                    tracing::warn!("Primary model failed, trying fallback: {}", fallback);

                    // Create new request with fallback model
                    let mut fallback_request = CreateMessageRequest::new(
                        fallback,
                        vec![ApiMessage::user(prompt.to_string())],
                        max_tokens,
                    );

                    // Copy system prompt if present
                    if let Some(ref sys_prompt) = system_prompt {
                        fallback_request = fallback_request.with_system(sys_prompt.clone());
                    }

                    // Add tools
                    fallback_request =
                        fallback_request.with_tools(tool_definitions::get_all_tool_definitions());

                    let hooks_fallback = hooks_for_tools.clone();
                    let session_id_fallback = session_id_for_tools.clone();
                    let allowed_tools_fallback = allowed_tools_for_executor.clone();
                    let disallowed_tools_fallback = disallowed_tools_for_executor.clone();

                    client
                        .execute_with_tools(fallback_request, |tool_name, tool_input| {
                            let hooks = hooks_fallback.clone();
                            let session_id = session_id_fallback.clone();
                            let allowed_tools = allowed_tools_fallback.clone();
                            let disallowed_tools = disallowed_tools_fallback.clone();
                            async move {
                                tool_executor::execute_tool_with_permission(
                                    tool_name,
                                    tool_input,
                                    permission_mode,
                                    tool_executor::ToolExecutionParams {
                                        hooks: Some(hooks),
                                        session_id: Some(session_id),
                                        notification_manager: None, // No notification manager in non-interactive mode
                                        tool_use_id: None, // No tool_use_id in non-interactive mode
                                        allowed_tools,
                                        disallowed_tools,
                                    },
                                )
                                .await
                            }
                        })
                        .await?
                } else {
                    return Err(e.into());
                }
            }
        };

        // Extract text from response
        let text = response
            .content
            .iter()
            .filter_map(|block| {
                if let rustyclawd_core::client::types::ContentBlock::Text { text } = block {
                    Some(text.as_str())
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
            .join("");

        // Output based on format (IDE mode forces JSON output)
        let output_format = if self.cli.ide {
            "json"
        } else {
            self.cli.output_format.as_str()
        };

        match output_format {
            "json" => {
                let mut json_output = serde_json::json!({
                    "id": response.id,
                    "type": response.type_field,
                    "role": response.role,
                    "content": response.content,
                    "model": response.model,
                    "stop_reason": response.stop_reason,
                    "usage": response.usage,
                });

                // Add IDE-specific metadata if in IDE mode
                if self.cli.ide {
                    json_output["ide_mode"] = serde_json::json!(true);
                    json_output["session_id"] = serde_json::json!(self.session.id);
                }

                println!("{}", serde_json::to_string_pretty(&json_output)?);
            }
            "stream-json" => {
                // For stream-json, output as JSON but with streaming markers if requested
                let json_output = serde_json::json!({
                    "id": response.id,
                    "type": response.type_field,
                    "role": response.role,
                    "content": response.content,
                    "model": response.model,
                    "stop_reason": response.stop_reason,
                    "usage": response.usage,
                });
                println!("{}", serde_json::to_string(&json_output)?);
            }
            _ => {
                // Text format (default)
                println!("{}", text);
            }
        }

        Ok(())
    }
}
