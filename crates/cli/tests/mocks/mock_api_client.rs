//! Mock API Client for testing
//!
//! Provides deterministic API responses without real API calls:
//! - Simulated streaming
//! - Configurable responses
//! - Error injection
//! - Tool use simulation

use futures::stream::{self, Stream};
use std::pin::Pin;
use std::sync::{Arc, Mutex};

/// Stream event type for mocking
#[derive(Debug, Clone, PartialEq)]
pub enum MockStreamEvent {
    /// Text content delta
    ContentDelta(String),
    /// Tool use start
    ToolUseStart { id: String, name: String },
    /// Tool use input delta
    ToolInputDelta(String),
    /// Tool use end
    ToolUseEnd,
    /// Message complete
    MessageComplete,
    /// Error occurred
    Error(String),
}

/// Mock API response
#[derive(Debug, Clone)]
pub struct MockResponse {
    /// Events to stream
    pub events: Vec<MockStreamEvent>,
    /// Delay between events (ms)
    pub delay_ms: u64,
}

impl MockResponse {
    /// Create simple text response
    pub fn text(content: &str) -> Self {
        Self {
            events: vec![
                MockStreamEvent::ContentDelta(content.to_string()),
                MockStreamEvent::MessageComplete,
            ],
            delay_ms: 10,
        }
    }

    /// Create chunked text response
    pub fn chunked_text(chunks: Vec<&str>) -> Self {
        let mut events: Vec<_> = chunks
            .into_iter()
            .map(|chunk| MockStreamEvent::ContentDelta(chunk.to_string()))
            .collect();
        events.push(MockStreamEvent::MessageComplete);

        Self {
            events,
            delay_ms: 10,
        }
    }

    /// Create response with tool use
    pub fn with_tool_use(tool_name: &str, tool_input: &str, response_text: &str) -> Self {
        Self {
            events: vec![
                MockStreamEvent::ToolUseStart {
                    id: "tool_123".to_string(),
                    name: tool_name.to_string(),
                },
                MockStreamEvent::ToolInputDelta(tool_input.to_string()),
                MockStreamEvent::ToolUseEnd,
                MockStreamEvent::ContentDelta(response_text.to_string()),
                MockStreamEvent::MessageComplete,
            ],
            delay_ms: 10,
        }
    }

    /// Create error response
    pub fn error(message: &str) -> Self {
        Self {
            events: vec![MockStreamEvent::Error(message.to_string())],
            delay_ms: 0,
        }
    }

    /// Set delay between events
    pub fn with_delay(mut self, delay_ms: u64) -> Self {
        self.delay_ms = delay_ms;
        self
    }
}

/// Mock API client for testing
pub struct MockApiClient {
    /// Queued responses
    responses: Arc<Mutex<Vec<MockResponse>>>,
    /// Call history
    call_history: Arc<Mutex<Vec<String>>>,
}

impl MockApiClient {
    /// Create new mock client
    pub fn new() -> Self {
        Self {
            responses: Arc::new(Mutex::new(Vec::new())),
            call_history: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Queue a response
    pub fn queue_response(&self, response: MockResponse) {
        self.responses.lock().unwrap().push(response);
    }

    /// Queue multiple responses
    pub fn queue_responses(&self, responses: Vec<MockResponse>) {
        self.responses.lock().unwrap().extend(responses);
    }

    /// Get call history
    pub fn call_history(&self) -> Vec<String> {
        self.call_history.lock().unwrap().clone()
    }

    /// Clear call history
    pub fn clear_history(&self) {
        self.call_history.lock().unwrap().clear();
    }

    /// Send message and get streaming response
    pub async fn send_message(
        &self,
        prompt: String,
    ) -> Pin<Box<dyn Stream<Item = MockStreamEvent> + Send>> {
        // Record call
        self.call_history.lock().unwrap().push(prompt);

        // Get next response
        let response = self
            .responses
            .lock()
            .unwrap()
            .pop()
            .unwrap_or_else(|| MockResponse::text("Mock response"));

        // Create stream
        let events = response.events.clone();
        let delay_ms = response.delay_ms;

        Box::pin(stream::iter(events.into_iter().inspect(move |event| {
            if delay_ms > 0 {
                std::thread::sleep(std::time::Duration::from_millis(delay_ms));
            }
        })))
    }

    /// Send message with specific response (convenience method)
    pub async fn send_with_response(
        &self,
        prompt: String,
        response: MockResponse,
    ) -> Pin<Box<dyn Stream<Item = MockStreamEvent> + Send>> {
        self.queue_response(response);
        self.send_message(prompt).await
    }
}

impl Default for MockApiClient {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::StreamExt;

    #[tokio::test]
    async fn test_mock_client_simple_response() {
        let client = MockApiClient::new();
        client.queue_response(MockResponse::text("Hello, World!"));

        let mut stream = client.send_message("Test prompt".to_string()).await;
        let mut events = Vec::new();

        while let Some(event) = stream.next().await {
            events.push(event);
        }

        assert_eq!(events.len(), 2);
        assert_eq!(
            events[0],
            MockStreamEvent::ContentDelta("Hello, World!".to_string())
        );
        assert_eq!(events[1], MockStreamEvent::MessageComplete);
    }

    #[tokio::test]
    async fn test_mock_client_chunked_response() {
        let client = MockApiClient::new();
        client.queue_response(MockResponse::chunked_text(vec!["Hello", ", ", "World!"]));

        let mut stream = client.send_message("Test".to_string()).await;
        let mut events = Vec::new();

        while let Some(event) = stream.next().await {
            events.push(event);
        }

        assert_eq!(events.len(), 4); // 3 chunks + complete
        assert!(matches!(
            events[0],
            MockStreamEvent::ContentDelta(ref s) if s == "Hello"
        ));
        assert!(matches!(
            events[1],
            MockStreamEvent::ContentDelta(ref s) if s == ", "
        ));
        assert!(matches!(
            events[2],
            MockStreamEvent::ContentDelta(ref s) if s == "World!"
        ));
    }

    #[tokio::test]
    async fn test_mock_client_with_tool_use() {
        let client = MockApiClient::new();
        client.queue_response(MockResponse::with_tool_use(
            "bash",
            r#"{"command":"echo test"}"#,
            "Command executed successfully",
        ));

        let mut stream = client.send_message("Run echo".to_string()).await;
        let mut events = Vec::new();

        while let Some(event) = stream.next().await {
            events.push(event);
        }

        assert_eq!(events.len(), 5);
        assert!(matches!(events[0], MockStreamEvent::ToolUseStart { .. }));
        assert!(matches!(events[1], MockStreamEvent::ToolInputDelta(_)));
        assert_eq!(events[2], MockStreamEvent::ToolUseEnd);
    }

    #[tokio::test]
    async fn test_mock_client_error() {
        let client = MockApiClient::new();
        client.queue_response(MockResponse::error("API Error"));

        let mut stream = client.send_message("Test".to_string()).await;
        let mut events = Vec::new();

        while let Some(event) = stream.next().await {
            events.push(event);
        }

        assert_eq!(events.len(), 1);
        assert!(matches!(
            events[0],
            MockStreamEvent::Error(ref s) if s == "API Error"
        ));
    }

    #[tokio::test]
    async fn test_mock_client_call_history() {
        let client = MockApiClient::new();
        client.queue_response(MockResponse::text("Response 1"));
        client.queue_response(MockResponse::text("Response 2"));

        let _ = client.send_message("Prompt 1".to_string()).await;
        let _ = client.send_message("Prompt 2".to_string()).await;

        let history = client.call_history();
        assert_eq!(history.len(), 2);
        assert_eq!(history[0], "Prompt 1");
        assert_eq!(history[1], "Prompt 2");
    }

    #[tokio::test]
    async fn test_mock_client_multiple_responses() {
        let client = MockApiClient::new();
        client.queue_responses(vec![
            MockResponse::text("First"),
            MockResponse::text("Second"),
            MockResponse::text("Third"),
        ]);

        // Responses are popped in LIFO order
        let mut stream = client.send_message("Test 1".to_string()).await;
        let event = stream.next().await.unwrap();
        assert!(matches!(
            event,
            MockStreamEvent::ContentDelta(ref s) if s == "Third"
        ));
    }

    #[test]
    fn test_mock_response_builders() {
        let text_resp = MockResponse::text("Hello");
        assert_eq!(text_resp.events.len(), 2);

        let chunked = MockResponse::chunked_text(vec!["a", "b", "c"]);
        assert_eq!(chunked.events.len(), 4);

        let with_tool = MockResponse::with_tool_use("bash", "{}", "Done");
        assert_eq!(with_tool.events.len(), 5);

        let error = MockResponse::error("Failed");
        assert_eq!(error.events.len(), 1);
    }
}
