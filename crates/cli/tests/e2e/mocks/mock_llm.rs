//! MockLLM - Controllable LLM behavior for deterministic E2E testing
//!
//! This mock implements the same interface as the real Anthropic API client
//! but with queue-based, deterministic responses for testing.
//!
//! Philosophy:
//! - Single responsibility: Provide predictable LLM responses
//! - Zero-BS: Behaves like real API, just with controlled responses
//! - Self-contained: All mock logic in one module
//! - Regeneratable: Can be rebuilt from specification

use anyhow::{anyhow, Result};
use futures::stream::{self, Stream};
use rustyclawd_core::client::{
    ContentBlock, CreateMessageRequest, Message, MessageResponse, Role, StreamEvent, Usage,
};
use rustyclawd_core::client::types::{ContentBlockStart, ContentDelta, MessageDelta, MessageStart};
use std::collections::VecDeque;
use std::pin::Pin;
use std::sync::{Arc, Mutex};

/// Mock response type for queue
#[derive(Debug, Clone)]
pub enum MockResponse {
    /// Simple text response
    Text { content: String },

    /// Tool use response
    ToolUse {
        tool_name: String,
        tool_id: String,
        parameters: serde_json::Value,
    },

    /// Text response followed by tool use
    TextThenToolUse {
        text: String,
        tool_name: String,
        tool_id: String,
        parameters: serde_json::Value,
    },

    /// Error response
    Error { message: String },

    /// Streaming response (multiple chunks)
    Streaming { chunks: Vec<String> },
}

impl MockResponse {
    /// Create simple text response
    pub fn text(content: impl Into<String>) -> Self {
        Self::Text {
            content: content.into(),
        }
    }

    /// Create tool use response
    pub fn tool_use(
        tool_name: impl Into<String>,
        tool_id: impl Into<String>,
        parameters: serde_json::Value,
    ) -> Self {
        Self::ToolUse {
            tool_name: tool_name.into(),
            tool_id: tool_id.into(),
            parameters,
        }
    }

    /// Create text then tool use response
    pub fn text_then_tool_use(
        text: impl Into<String>,
        tool_name: impl Into<String>,
        tool_id: impl Into<String>,
        parameters: serde_json::Value,
    ) -> Self {
        Self::TextThenToolUse {
            text: text.into(),
            tool_name: tool_name.into(),
            tool_id: tool_id.into(),
            parameters,
        }
    }

    /// Create error response
    pub fn error(message: impl Into<String>) -> Self {
        Self::Error {
            message: message.into(),
        }
    }

    /// Create streaming response
    pub fn streaming(chunks: Vec<String>) -> Self {
        Self::Streaming { chunks }
    }
}

/// Recorded request for verification
#[derive(Debug, Clone)]
pub struct RecordedRequest {
    pub model: String,
    pub max_tokens: u32,
    pub system: Option<String>,
    pub messages: Vec<Message>,
    pub timestamp: std::time::Instant,
}

/// Internal state for MockLLM
struct MockLLMState {
    /// Queue of responses (FIFO)
    response_queue: VecDeque<MockResponse>,
    /// Recorded requests for verification
    recorded_requests: Vec<RecordedRequest>,
    /// Default model
    default_model: String,
}

/// MockLLM - Controllable LLM client for deterministic testing
///
/// Implements the same interface as the real Client but with queue-based responses.
///
/// # Example
///
/// ```no_run
/// use rustyclawd_cli::e2e::mocks::MockLLM;
/// use rustyclawd_core::client::{CreateMessageRequest, Message};
///
/// #[tokio::main]
/// async fn main() {
///     let mut mock = MockLLM::new();
///     mock.queue_response(MockResponse::text("Hello, world!"));
///
///     let request = CreateMessageRequest::new(
///         "claude-3-5-sonnet-20241022",
///         vec![Message::user("Hi")],
///         1024,
///     );
///
///     let response = mock.create_message(request).await.unwrap();
///     assert_eq!(response.content[0], ContentBlock::Text {
///         text: "Hello, world!".to_string()
///     });
/// }
/// ```
pub struct MockLLM {
    state: Arc<Mutex<MockLLMState>>,
}

impl MockLLM {
    /// Create new mock LLM with empty response queue
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(MockLLMState {
                response_queue: VecDeque::new(),
                recorded_requests: Vec::new(),
                default_model: "claude-3-5-sonnet-20241022".to_string(),
            })),
        }
    }

    /// Create mock with predefined responses
    pub fn with_responses(responses: Vec<MockResponse>) -> Self {
        let mock = Self::new();
        for response in responses {
            mock.queue_response(response);
        }
        mock
    }

    /// Queue a response (FIFO)
    pub fn queue_response(&self, response: MockResponse) {
        self.state.lock().unwrap().response_queue.push_back(response);
    }

    /// Queue multiple responses
    pub fn queue_responses(&self, responses: Vec<MockResponse>) {
        let mut state = self.state.lock().unwrap();
        for response in responses {
            state.response_queue.push_back(response);
        }
    }

    /// Get all recorded requests
    pub fn get_requests(&self) -> Vec<RecordedRequest> {
        self.state.lock().unwrap().recorded_requests.clone()
    }

    /// Get last request sent to mock
    pub fn last_request(&self) -> Option<RecordedRequest> {
        self.state.lock().unwrap().recorded_requests.last().cloned()
    }

    /// Check if specific system prompt was used
    pub fn used_system_prompt(&self, prompt: &str) -> bool {
        self.state
            .lock()
            .unwrap()
            .recorded_requests
            .iter()
            .any(|req| {
                req.system
                    .as_ref()
                    .map_or(false, |sys| sys.contains(prompt))
            })
    }

    /// Get number of requests made
    pub fn request_count(&self) -> usize {
        self.state.lock().unwrap().recorded_requests.len()
    }

    /// Reset mock state (clear requests and responses)
    pub fn reset(&self) {
        let mut state = self.state.lock().unwrap();
        state.response_queue.clear();
        state.recorded_requests.clear();
    }

    /// Check if response queue is empty
    pub fn is_queue_empty(&self) -> bool {
        self.state.lock().unwrap().response_queue.is_empty()
    }

    /// Create a message (non-streaming)
    ///
    /// This matches the real Client::create_message API
    pub async fn create_message(
        &self,
        request: CreateMessageRequest,
    ) -> Result<MessageResponse> {
        // Record the request
        let mut state = self.state.lock().unwrap();
        state.recorded_requests.push(RecordedRequest {
            model: request.model.clone(),
            max_tokens: request.max_tokens,
            system: request.system.clone(),
            messages: request.messages.clone(),
            timestamp: std::time::Instant::now(),
        });

        // Get next response from queue
        let response = state
            .response_queue
            .pop_front()
            .ok_or_else(|| anyhow!("MockLLM response queue is empty - did you forget to queue a response?"))?;

        drop(state); // Release lock before building response

        // Build MessageResponse based on MockResponse type
        match response {
            MockResponse::Text { content } => Ok(MessageResponse {
                id: format!("msg_{}", uuid::Uuid::new_v4()),
                type_field: "message".to_string(),
                role: Role::Assistant,
                content: vec![ContentBlock::Text { text: content }],
                model: request.model,
                stop_reason: Some("end_turn".to_string()),
                stop_sequence: None,
                usage: Usage {
                    input_tokens: 10,
                    output_tokens: 20,
                },
            }),

            MockResponse::ToolUse {
                tool_name,
                tool_id,
                parameters,
            } => Ok(MessageResponse {
                id: format!("msg_{}", uuid::Uuid::new_v4()),
                type_field: "message".to_string(),
                role: Role::Assistant,
                content: vec![ContentBlock::ToolUse {
                    id: tool_id,
                    name: tool_name,
                    input: parameters,
                }],
                model: request.model,
                stop_reason: Some("tool_use".to_string()),
                stop_sequence: None,
                usage: Usage {
                    input_tokens: 10,
                    output_tokens: 15,
                },
            }),

            MockResponse::TextThenToolUse {
                text,
                tool_name,
                tool_id,
                parameters,
            } => Ok(MessageResponse {
                id: format!("msg_{}", uuid::Uuid::new_v4()),
                type_field: "message".to_string(),
                role: Role::Assistant,
                content: vec![
                    ContentBlock::Text { text },
                    ContentBlock::ToolUse {
                        id: tool_id,
                        name: tool_name,
                        input: parameters,
                    },
                ],
                model: request.model,
                stop_reason: Some("tool_use".to_string()),
                stop_sequence: None,
                usage: Usage {
                    input_tokens: 10,
                    output_tokens: 25,
                },
            }),

            MockResponse::Error { message } => {
                Err(anyhow!("MockLLM error: {}", message))
            }

            MockResponse::Streaming { .. } => {
                Err(anyhow!("Streaming response queued but create_message called (use create_message_stream)"))
            }
        }
    }

    /// Create a message with streaming
    ///
    /// This matches the real Client::create_message_stream API
    pub async fn create_message_stream(
        &self,
        request: CreateMessageRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamEvent>> + Send>>> {
        // Record the request
        let mut state = self.state.lock().unwrap();
        state.recorded_requests.push(RecordedRequest {
            model: request.model.clone(),
            max_tokens: request.max_tokens,
            system: request.system.clone(),
            messages: request.messages.clone(),
            timestamp: std::time::Instant::now(),
        });

        // Get next response from queue
        let response = state
            .response_queue
            .pop_front()
            .ok_or_else(|| anyhow!("MockLLM response queue is empty"))?;

        drop(state); // Release lock

        // Build stream based on response type
        match response {
            MockResponse::Text { content } => {
                let events = vec![
                    StreamEvent::MessageStart {
                        message: MessageStart {
                            id: format!("msg_{}", uuid::Uuid::new_v4()),
                            type_field: "message".to_string(),
                            role: Role::Assistant,
                            content: vec![],
                            model: request.model,
                            stop_reason: None,
                            stop_sequence: None,
                            usage: Usage {
                                input_tokens: 10,
                                output_tokens: 0,
                            },
                        },
                    },
                    StreamEvent::ContentBlockStart {
                        index: 0,
                        content_block: ContentBlockStart::Text {
                            text: String::new(),
                        },
                    },
                    StreamEvent::ContentBlockDelta {
                        index: 0,
                        delta: ContentDelta::TextDelta { text: content },
                    },
                    StreamEvent::ContentBlockStop { index: 0 },
                    StreamEvent::MessageDelta {
                        delta: MessageDelta {
                            stop_reason: Some("end_turn".to_string()),
                            stop_sequence: None,
                        },
                        usage: Usage {
                            input_tokens: 0,
                            output_tokens: 20,
                        },
                    },
                    StreamEvent::MessageStop,
                ];

                Ok(Box::pin(stream::iter(events.into_iter().map(Ok))))
            }

            MockResponse::Streaming { chunks } => {
                let mut events = vec![
                    StreamEvent::MessageStart {
                        message: MessageStart {
                            id: format!("msg_{}", uuid::Uuid::new_v4()),
                            type_field: "message".to_string(),
                            role: Role::Assistant,
                            content: vec![],
                            model: request.model,
                            stop_reason: None,
                            stop_sequence: None,
                            usage: Usage {
                                input_tokens: 10,
                                output_tokens: 0,
                            },
                        },
                    },
                    StreamEvent::ContentBlockStart {
                        index: 0,
                        content_block: ContentBlockStart::Text {
                            text: String::new(),
                        },
                    },
                ];

                // Add delta for each chunk
                for chunk in chunks {
                    events.push(StreamEvent::ContentBlockDelta {
                        index: 0,
                        delta: ContentDelta::TextDelta { text: chunk },
                    });
                }

                events.push(StreamEvent::ContentBlockStop { index: 0 });
                events.push(StreamEvent::MessageDelta {
                    delta: MessageDelta {
                        stop_reason: Some("end_turn".to_string()),
                        stop_sequence: None,
                    },
                    usage: Usage {
                        input_tokens: 0,
                        output_tokens: 20,
                    },
                });
                events.push(StreamEvent::MessageStop);

                Ok(Box::pin(stream::iter(events.into_iter().map(Ok))))
            }

            _ => Err(anyhow!(
                "Non-streaming response queued for create_message_stream"
            )),
        }
    }
}

impl Default for MockLLM {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for MockLLM {
    fn clone(&self) -> Self {
        Self {
            state: Arc::clone(&self.state),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_llm_simple_text_response() {
        let mock = MockLLM::new();
        mock.queue_response(MockResponse::text("Hello, world!"));

        let request = CreateMessageRequest::new(
            "claude-3-5-sonnet-20241022",
            vec![Message::user("Hi")],
            1024,
        );

        let response = mock.create_message(request).await.unwrap();

        assert_eq!(response.content.len(), 1);
        match &response.content[0] {
            ContentBlock::Text { text } => assert_eq!(text, "Hello, world!"),
            _ => panic!("Expected text content"),
        }
        assert_eq!(response.stop_reason, Some("end_turn".to_string()));
    }

    #[tokio::test]
    async fn test_mock_llm_tool_use_response() {
        let mock = MockLLM::new();
        mock.queue_response(MockResponse::tool_use(
            "Read",
            "tool_123",
            serde_json::json!({"file_path": "README.md"}),
        ));

        let request = CreateMessageRequest::new(
            "claude-3-5-sonnet-20241022",
            vec![Message::user("Read README")],
            1024,
        );

        let response = mock.create_message(request).await.unwrap();

        assert_eq!(response.content.len(), 1);
        match &response.content[0] {
            ContentBlock::ToolUse { id, name, input } => {
                assert_eq!(id, "tool_123");
                assert_eq!(name, "Read");
                assert_eq!(input["file_path"], "README.md");
            }
            _ => panic!("Expected tool use content"),
        }
        assert_eq!(response.stop_reason, Some("tool_use".to_string()));
    }

    #[tokio::test]
    async fn test_mock_llm_request_recording() {
        let mock = MockLLM::new();
        mock.queue_response(MockResponse::text("Response 1"));
        mock.queue_response(MockResponse::text("Response 2"));

        let _resp1 = mock
            .create_message(CreateMessageRequest::new(
                "claude-3-5-sonnet-20241022",
                vec![Message::user("First")],
                1024,
            ))
            .await
            .unwrap();

        let _resp2 = mock
            .create_message(CreateMessageRequest::new(
                "claude-3-5-sonnet-20241022",
                vec![Message::user("Second")],
                1024,
            ))
            .await
            .unwrap();

        assert_eq!(mock.request_count(), 2);

        let requests = mock.get_requests();
        assert_eq!(requests.len(), 2);
        assert_eq!(requests[0].messages[0].role, Role::User);
    }

    #[tokio::test]
    async fn test_mock_llm_empty_queue_error() {
        let mock = MockLLM::new();
        // Don't queue any responses

        let request = CreateMessageRequest::new(
            "claude-3-5-sonnet-20241022",
            vec![Message::user("Test")],
            1024,
        );

        let result = mock.create_message(request).await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("queue is empty"));
    }

    #[tokio::test]
    async fn test_mock_llm_system_prompt_tracking() {
        let mock = MockLLM::new();
        mock.queue_response(MockResponse::text("Response"));

        let request = CreateMessageRequest::new(
            "claude-3-5-sonnet-20241022",
            vec![Message::user("Test")],
            1024,
        )
        .with_system("You are a helpful assistant".to_string());

        let _response = mock.create_message(request).await.unwrap();

        assert!(mock.used_system_prompt("helpful assistant"));
        assert!(!mock.used_system_prompt("unhelpful"));
    }

    #[tokio::test]
    async fn test_mock_llm_reset() {
        let mock = MockLLM::new();
        mock.queue_response(MockResponse::text("Response"));

        let _resp = mock
            .create_message(CreateMessageRequest::new(
                "claude-3-5-sonnet-20241022",
                vec![Message::user("Test")],
                1024,
            ))
            .await
            .unwrap();

        assert_eq!(mock.request_count(), 1);
        assert!(mock.is_queue_empty());

        mock.reset();

        assert_eq!(mock.request_count(), 0);
        assert!(mock.is_queue_empty());
    }

    #[tokio::test]
    async fn test_mock_llm_with_responses_constructor() {
        let mock = MockLLM::with_responses(vec![
            MockResponse::text("First"),
            MockResponse::text("Second"),
        ]);

        let resp1 = mock
            .create_message(CreateMessageRequest::new(
                "claude-3-5-sonnet-20241022",
                vec![Message::user("1")],
                1024,
            ))
            .await
            .unwrap();

        match &resp1.content[0] {
            ContentBlock::Text { text } => assert_eq!(text, "First"),
            _ => panic!("Expected text"),
        }

        let resp2 = mock
            .create_message(CreateMessageRequest::new(
                "claude-3-5-sonnet-20241022",
                vec![Message::user("2")],
                1024,
            ))
            .await
            .unwrap();

        match &resp2.content[0] {
            ContentBlock::Text { text } => assert_eq!(text, "Second"),
            _ => panic!("Expected text"),
        }
    }
}
