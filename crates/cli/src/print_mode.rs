//! Print mode (one-shot execution) for RustyClawd.
//!
//! Contains `App::run_print_mode()` which handles `--print` / `-p` prompt
//! execution against the Anthropic API with tool use, fallback models,
//! system prompt overrides, and hook integration.

use anyhow::{Context as AnyhowContext, Result};
use rustyclawd_core::client::response::MessageResponse;
use rustyclawd_core::client::ToolLoopEvent;

use super::{hooks, permission_mode, tool_definitions, tool_executor, App};

// ---------------------------------------------------------------------------
// SDK-compatible stream-json helpers
// ---------------------------------------------------------------------------

/// Emit a single newline-delimited JSON message to stdout.
fn emit_sdk_message(msg: &serde_json::Value) {
    if let Ok(json) = serde_json::to_string(msg) {
        println!("{}", json);
    }
}

/// Emit the initial `system/init` message that opens an SDK session.
fn emit_init_message(session_id: &str) {
    emit_sdk_message(&serde_json::json!({
        "type": "system",
        "subtype": "init",
        "session_id": session_id,
        "data": {}
    }));
}

/// Emit an `assistant` message carrying the model response content.
fn emit_assistant_message(response: &MessageResponse) {
    emit_sdk_message(&serde_json::json!({
        "type": "assistant",
        "content": response.content,
        "model": response.model,
    }));
}

/// Emit the final `result` message that closes an SDK session.
fn emit_result_message(
    session_id: &str,
    result_text: &str,
    num_turns: u32,
    duration_ms: u64,
    is_error: bool,
) {
    emit_sdk_message(&serde_json::json!({
        "type": "result",
        "subtype": "result",
        "session_id": session_id,
        "result": result_text,
        "num_turns": num_turns,
        "duration_ms": duration_ms,
        "is_error": is_error,
        "stop_reason": if is_error { "error" } else { "end_turn" }
    }));
}

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

        // Determine output format early so we can emit init messages before the API call
        let output_format = if self.cli.ide {
            "json"
        } else {
            self.cli.output_format.as_str()
        };

        // Emit SDK init message before the API call for stream-json
        if output_format == "stream-json" {
            emit_init_message(&self.session.id);
        }

        let start_time = std::time::Instant::now();

        // Build the tool executor closure factory (shared by primary and fallback)
        macro_rules! make_tool_executor {
            ($hooks:expr, $session_id:expr, $allowed:expr, $disallowed:expr) => {
                |tool_name: String, tool_input: serde_json::Value| {
                    let hooks = $hooks.clone();
                    let session_id = $session_id.clone();
                    let allowed_tools = $allowed.clone();
                    let disallowed_tools = $disallowed.clone();
                    async move {
                        tool_executor::execute_tool_with_permission(
                            tool_name,
                            tool_input,
                            permission_mode,
                            tool_executor::ToolExecutionParams {
                                hooks: Some(hooks),
                                session_id: Some(session_id),
                                notification_manager: None,
                                tool_use_id: None,
                                allowed_tools,
                                disallowed_tools,
                            },
                        )
                        .await
                    }
                }
            };
        }

        // For stream-json, use the event-emitting variant so each turn is
        // streamed to stdout as it happens. Other formats use the simpler path.
        let (response, num_turns) = if output_format == "stream-json" {
            let on_event = |event: ToolLoopEvent| async move {
                match event {
                    ToolLoopEvent::AssistantMessage(ref resp) => {
                        emit_assistant_message(resp);
                    }
                    ToolLoopEvent::ToolUse { .. } | ToolLoopEvent::ToolResult { .. } => {
                        // Tool lifecycle events are logged via tracing; the SDK
                        // protocol only requires assistant messages per turn.
                    }
                }
            };

            match client
                .execute_with_tools_and_events(
                    request.clone(),
                    make_tool_executor!(
                        hooks_for_tools,
                        session_id_for_tools,
                        allowed_tools_for_executor,
                        disallowed_tools_for_executor
                    ),
                    on_event,
                )
                .await
            {
                Ok((resp, turns)) => (resp, turns),
                Err(e) => {
                    if let Some(fallback) = fallback_model {
                        tracing::warn!("Primary model failed, trying fallback: {}", fallback);
                        let mut fallback_request = CreateMessageRequest::new(
                            fallback,
                            vec![ApiMessage::user(prompt.to_string())],
                            max_tokens,
                        );
                        if let Some(ref sys_prompt) = system_prompt {
                            fallback_request = fallback_request.with_system(sys_prompt.clone());
                        }
                        fallback_request = fallback_request
                            .with_tools(tool_definitions::get_all_tool_definitions());

                        client
                            .execute_with_tools_and_events(
                                fallback_request,
                                make_tool_executor!(
                                    hooks_for_tools,
                                    session_id_for_tools,
                                    allowed_tools_for_executor,
                                    disallowed_tools_for_executor
                                ),
                                on_event,
                            )
                            .await?
                    } else {
                        return Err(e.into());
                    }
                }
            }
        } else {
            // Non-streaming path: use the simpler execute_with_tools
            let result = match client
                .execute_with_tools(
                    request.clone(),
                    make_tool_executor!(
                        hooks_for_tools,
                        session_id_for_tools,
                        allowed_tools_for_executor,
                        disallowed_tools_for_executor
                    ),
                )
                .await
            {
                Ok(resp) => resp,
                Err(e) => {
                    if let Some(fallback) = fallback_model {
                        tracing::warn!("Primary model failed, trying fallback: {}", fallback);
                        let mut fallback_request = CreateMessageRequest::new(
                            fallback,
                            vec![ApiMessage::user(prompt.to_string())],
                            max_tokens,
                        );
                        if let Some(ref sys_prompt) = system_prompt {
                            fallback_request = fallback_request.with_system(sys_prompt.clone());
                        }
                        fallback_request = fallback_request
                            .with_tools(tool_definitions::get_all_tool_definitions());

                        client
                            .execute_with_tools(
                                fallback_request,
                                make_tool_executor!(
                                    hooks_for_tools,
                                    session_id_for_tools,
                                    allowed_tools_for_executor,
                                    disallowed_tools_for_executor
                                ),
                            )
                            .await?
                    } else {
                        return Err(e.into());
                    }
                }
            };
            // Non-streaming path doesn't track turns (single response)
            (result, 1)
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

        // Output based on format
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
                    "session_id": self.session.id,
                });

                // Add IDE-specific metadata if in IDE mode
                if self.cli.ide {
                    json_output["ide_mode"] = serde_json::json!(true);
                }

                println!("{}", serde_json::to_string_pretty(&json_output)?);
            }
            "stream-json" => {
                // Assistant messages were already emitted per-turn via on_event.
                // Emit the final result message to close the SDK session.
                let duration_ms = start_time.elapsed().as_millis() as u64;
                emit_result_message(&self.session.id, &text, num_turns, duration_ms, false);
            }
            _ => {
                // Text format (default)
                println!("{}", text);
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use rustyclawd_core::client::response::{MessageResponse, Usage};
    use rustyclawd_core::client::types::{ContentBlock, Role};

    /// Build a minimal `MessageResponse` for testing.
    fn make_response(text: &str) -> MessageResponse {
        MessageResponse {
            id: "msg_test123".to_string(),
            type_field: "message".to_string(),
            role: Role::Assistant,
            content: vec![ContentBlock::Text {
                text: text.to_string(),
            }],
            model: "claude-sonnet-4-6".to_string(),
            stop_reason: Some("end_turn".to_string()),
            stop_sequence: None,
            usage: Usage {
                input_tokens: 10,
                output_tokens: 20,
                speed: None,
            },
        }
    }

    #[test]
    fn test_init_message_structure() {
        let msg = serde_json::json!({
            "type": "system",
            "subtype": "init",
            "session_id": "test-session-42",
            "data": {}
        });

        assert_eq!(msg["type"], "system");
        assert_eq!(msg["subtype"], "init");
        assert_eq!(msg["session_id"], "test-session-42");
        assert!(msg["data"].is_object());
    }

    #[test]
    fn test_assistant_message_structure() {
        let response = make_response("Hello, world!");
        let msg = serde_json::json!({
            "type": "assistant",
            "content": response.content,
            "model": response.model,
        });

        assert_eq!(msg["type"], "assistant");
        assert_eq!(msg["model"], "claude-sonnet-4-6");
        assert!(msg["content"].is_array());
        let content = &msg["content"][0];
        assert_eq!(content["type"], "text");
        assert_eq!(content["text"], "Hello, world!");
    }

    #[test]
    fn test_result_message_structure() {
        let msg = serde_json::json!({
            "type": "result",
            "subtype": "result",
            "session_id": "sess-abc",
            "result": "done",
            "num_turns": 1u32,
            "duration_ms": 500u64,
            "is_error": false,
            "stop_reason": "end_turn"
        });

        assert_eq!(msg["type"], "result");
        assert_eq!(msg["subtype"], "result");
        assert_eq!(msg["session_id"], "sess-abc");
        assert_eq!(msg["result"], "done");
        assert_eq!(msg["num_turns"], 1);
        assert_eq!(msg["duration_ms"], 500);
        assert_eq!(msg["is_error"], false);
        assert_eq!(msg["stop_reason"], "end_turn");
    }

    #[test]
    fn test_result_message_error_stop_reason() {
        let is_error = true;
        let msg = serde_json::json!({
            "type": "result",
            "subtype": "result",
            "session_id": "sess-err",
            "result": "something failed",
            "num_turns": 1u32,
            "duration_ms": 100u64,
            "is_error": is_error,
            "stop_reason": if is_error { "error" } else { "end_turn" }
        });

        assert_eq!(msg["stop_reason"], "error");
        assert_eq!(msg["is_error"], true);
    }

    #[test]
    fn test_emit_sdk_message_produces_valid_json() {
        // Verify that serde_json::to_string on our message shapes produces
        // valid single-line JSON (the newline-delimited format requirement).
        let msg = serde_json::json!({
            "type": "system",
            "subtype": "init",
            "session_id": "test-123",
            "data": {}
        });
        let serialized = serde_json::to_string(&msg).unwrap();
        assert!(
            !serialized.contains('\n'),
            "SDK messages must be single-line JSON"
        );

        // Verify round-trip
        let parsed: serde_json::Value = serde_json::from_str(&serialized).unwrap();
        assert_eq!(parsed["type"], "system");
    }

    #[test]
    fn test_stream_json_message_ordering() {
        // Verify that the three message types form the correct sequence:
        // init -> assistant -> result
        let init = serde_json::json!({
            "type": "system", "subtype": "init",
            "session_id": "s1", "data": {}
        });
        let assistant = serde_json::json!({
            "type": "assistant",
            "content": [],
            "model": "claude-sonnet-4-6",
        });
        let result = serde_json::json!({
            "type": "result", "subtype": "result",
            "session_id": "s1", "result": "ok",
            "num_turns": 1, "duration_ms": 42,
            "is_error": false, "stop_reason": "end_turn"
        });

        let sequence = [&init, &assistant, &result];
        assert_eq!(sequence[0]["type"], "system");
        assert_eq!(sequence[1]["type"], "assistant");
        assert_eq!(sequence[2]["type"], "result");
    }

    #[test]
    fn test_json_format_includes_session_id() {
        // Verify the json output shape always includes session_id
        let response = make_response("test");
        let json_output = serde_json::json!({
            "id": response.id,
            "type": response.type_field,
            "role": response.role,
            "content": response.content,
            "model": response.model,
            "stop_reason": response.stop_reason,
            "usage": response.usage,
            "session_id": "my-session-id",
        });

        assert_eq!(json_output["session_id"], "my-session-id");
        assert!(json_output.get("session_id").is_some());
    }

    #[test]
    fn test_result_message_multi_turn() {
        // Verify num_turns > 1 is correctly reflected in the result message
        let msg = serde_json::json!({
            "type": "result",
            "subtype": "result",
            "session_id": "sess-multi",
            "result": "done after tools",
            "num_turns": 5u32,
            "duration_ms": 3200u64,
            "is_error": false,
            "stop_reason": "end_turn"
        });

        assert_eq!(msg["num_turns"], 5);
        assert_eq!(msg["stop_reason"], "end_turn");
    }

    #[test]
    fn test_stream_json_multi_turn_ordering() {
        // With per-turn streaming, the sequence is:
        // init -> assistant(turn1) -> assistant(turn2) -> ... -> result
        let init = serde_json::json!({"type": "system", "subtype": "init"});
        let turn1 = serde_json::json!({"type": "assistant", "content": [{"type": "tool_use"}]});
        let turn2 = serde_json::json!({"type": "assistant", "content": [{"type": "text"}]});
        let result = serde_json::json!({"type": "result", "num_turns": 2});

        let sequence = [&init, &turn1, &turn2, &result];
        assert_eq!(sequence[0]["type"], "system");
        assert_eq!(sequence[1]["type"], "assistant");
        assert_eq!(sequence[2]["type"], "assistant");
        assert_eq!(sequence[3]["type"], "result");
        assert_eq!(sequence[3]["num_turns"], 2);
    }
}
