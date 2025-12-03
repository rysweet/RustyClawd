# Module Specification: MockLLM

**Module Type:** Test Infrastructure
**Layer:** E2E Test Mocks
**Purpose:** Controllable LLM behavior for deterministic E2E testing

---

## Philosophy

**Single Responsibility:** Provide predictable, queue-based LLM responses for testing

**Regeneratable:** Can be rebuilt from spec without breaking tests

**Self-Contained:** All mock LLM logic in one module

**Zero-BS:** Behaves like real API, just with controlled responses

---

## Public API (Studs)

### Main Type

```rust
/// Mock LLM client for deterministic testing
///
/// Implements ApiClient trait with queue-based response system.
/// Records all requests for verification.
pub struct MockLLM {
    // Private implementation
}
```

### Construction

```rust
impl MockLLM {
    /// Create new mock LLM with empty response queue
    pub fn new() -> Self;

    /// Create mock with predefined responses
    pub fn with_responses(responses: Vec<MockResponse>) -> Self;
}
```

### Response Configuration

```rust
impl MockLLM {
    /// Add text response to queue
    ///
    /// # Example
    /// ```rust
    /// mock.add_response("Analysis complete: 42 modules found");
    /// ```
    pub fn add_response(&mut self, text: &str);

    /// Add tool use response to queue
    ///
    /// # Example
    /// ```rust
    /// mock.add_tool_use("Read", json!({"file_path": "README.md"}));
    /// ```
    pub fn add_tool_use(&mut self, tool_name: &str, params: serde_json::Value);

    /// Add thinking block to response
    ///
    /// # Example
    /// ```rust
    /// mock.add_thinking("Let me analyze this code...");
    /// ```
    pub fn add_thinking(&mut self, text: &str);

    /// Add error response to queue
    ///
    /// # Example
    /// ```rust
    /// mock.add_error(ApiError::RateLimitError {
    ///     retry_after: 60
    /// });
    /// ```
    pub fn add_error(&mut self, error: ApiError);

    /// Add streaming response (chunks)
    ///
    /// # Example
    /// ```rust
    /// mock.add_streaming_response(vec![
    ///     "Hello",
    ///     " world",
    ///     "!"
    /// ]);
    /// ```
    pub fn add_streaming_response(&mut self, chunks: Vec<&str>);
}
```

### State Inspection

```rust
impl MockLLM {
    /// Get all recorded request messages
    pub fn get_requests(&self) -> &[RecordedRequest];

    /// Get last request sent to mock
    pub fn last_request(&self) -> Option<&RecordedRequest>;

    /// Check if specific system prompt was used
    pub fn used_system_prompt(&self, prompt: &str) -> bool;

    /// Get number of requests made
    pub fn request_count(&self) -> usize;

    /// Reset mock state (clear requests and responses)
    pub fn reset(&mut self);

    /// Check if response queue is empty
    pub fn is_queue_empty(&self) -> bool;
}
```

### ApiClient Implementation

```rust
#[async_trait]
impl ApiClient for MockLLM {
    async fn create_message(
        &self,
        request: CreateMessageRequest
    ) -> Result<Message>;

    async fn create_message_stream(
        &self,
        request: CreateMessageRequest
    ) -> Result<MessageStream>;

    async fn count_tokens(
        &self,
        request: CountTokensRequest
    ) -> Result<CountTokensResponse>;
}
```

---

## Internal Types

### MockResponse

```rust
/// Queued response from mock LLM
#[derive(Debug, Clone)]
pub enum MockResponse {
    /// Simple text response
    Text {
        content: String,
        stop_reason: StopReason,
    },

    /// Tool use response
    ToolUse {
        tool_name: String,
        tool_id: String,
        parameters: serde_json::Value,
    },

    /// Thinking block
    Thinking {
        content: String,
    },

    /// Error response
    Error {
        error: ApiError,
    },

    /// Streaming response (multiple chunks)
    Streaming {
        chunks: Vec<String>,
        stop_reason: StopReason,
    },
}
```

### RecordedRequest

```rust
/// Captured request for verification
#[derive(Debug, Clone)]
pub struct RecordedRequest {
    pub model: String,
    pub max_tokens: u32,
    pub system: Vec<SystemMessage>,
    pub messages: Vec<Message>,
    pub tools: Option<Vec<Tool>>,
    pub temperature: Option<f32>,
    pub timestamp: std::time::Instant,
}
```

---

## Dependencies

### External Crates

```toml
[dependencies]
async-trait = "0.1"
serde_json = "1.0"
anyhow = "1.0"
tokio = { version = "1", features = ["sync"] }
```

### Internal Modules

- `crates/api_client/` - ApiClient trait, Message types
- `anthropic_sdk::types` - API types

---

## Implementation Notes

### Key Design Decisions

**1. Queue-Based Responses**
- Rationale: Simple FIFO queue matches request/response pattern
- Alternatives: Hash map by request, state machine
- Trade-off: Tests must queue responses in order

**2. Full Request Recording**
- Rationale: Enable thorough verification in tests
- Alternatives: Record only key fields
- Trade-off: Slightly higher memory usage

**3. Streaming Support**
- Rationale: Real API streams, tests should validate streaming
- Alternatives: Text-only responses
- Trade-off: More complex implementation

### Response Queue

```rust
// Internal state (private)
struct MockLLMState {
    response_queue: VecDeque<MockResponse>,
    recorded_requests: Vec<RecordedRequest>,
    default_model: String,
}
```

### Thread Safety

- Use `Arc<Mutex<MockLLMState>>` for shared state
- ApiClient trait methods take `&self`, need interior mutability
- Async-safe with tokio::sync::Mutex

### Error Handling

- Panic if queue empty when request made (test configuration error)
- Validate response types match API spec
- Provide clear error messages for debugging

---

## Test Requirements

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_queue_text_response() {
        let mut mock = MockLLM::new();
        mock.add_response("Hello");

        assert_eq!(mock.response_queue_len(), 1);
    }

    #[test]
    fn test_record_requests() {
        let mut mock = MockLLM::new();
        mock.add_response("Test response");

        // Simulate request
        let request = CreateMessageRequest {
            model: "claude-3-sonnet-20240229".to_string(),
            max_tokens: 1024,
            messages: vec![],
            system: vec![],
            tools: None,
            temperature: None,
        };

        let _ = mock.create_message(request).await;

        assert_eq!(mock.request_count(), 1);
    }

    #[test]
    #[should_panic(expected = "Response queue empty")]
    fn test_panic_on_empty_queue() {
        let mock = MockLLM::new();

        // No responses queued - should panic
        let request = CreateMessageRequest::default();
        let _ = mock.create_message(request).await;
    }
}
```

### Integration Tests

```rust
#[tokio::test]
async fn test_mock_with_api_client_trait() {
    let mut mock = MockLLM::new();
    mock.add_response("Test response");

    // Use as ApiClient
    let client: Box<dyn ApiClient> = Box::new(mock);

    let request = CreateMessageRequest {
        model: "claude-3-sonnet-20240229".to_string(),
        max_tokens: 1024,
        messages: vec![
            Message::user("Hello")
        ],
        system: vec![],
        tools: None,
        temperature: None,
    };

    let response = client.create_message(request).await.unwrap();

    assert_eq!(response.content[0].text(), "Test response");
}
```

---

## Usage Examples

### Example 1: Simple Text Response

```rust
#[tokio::test]
async fn test_simple_response() {
    let mut mock = MockLLM::new();
    mock.add_response("Hello, world!");

    let request = CreateMessageRequest {
        model: "claude-3-sonnet-20240229".to_string(),
        max_tokens: 1024,
        messages: vec![Message::user("Hi")],
        system: vec![],
        tools: None,
        temperature: None,
    };

    let response = mock.create_message(request).await.unwrap();

    assert_eq!(response.content[0].text(), "Hello, world!");
    assert_eq!(response.stop_reason, StopReason::EndTurn);
}
```

### Example 2: Tool Use Response

```rust
#[tokio::test]
async fn test_tool_use() {
    let mut mock = MockLLM::new();
    mock.add_tool_use("Read", json!({
        "file_path": "README.md"
    }));

    let request = CreateMessageRequest {
        model: "claude-3-sonnet-20240229".to_string(),
        max_tokens: 1024,
        messages: vec![Message::user("Read the README")],
        system: vec![],
        tools: Some(vec![/* tool definitions */]),
        temperature: None,
    };

    let response = mock.create_message(request).await.unwrap();

    // Verify tool use
    assert!(matches!(
        response.content[0],
        ContentBlock::ToolUse { name, .. } if name == "Read"
    ));
    assert_eq!(response.stop_reason, StopReason::ToolUse);
}
```

### Example 3: Multi-Turn Conversation

```rust
#[tokio::test]
async fn test_multi_turn() {
    let mut mock = MockLLM::new();
    mock.add_response("I'm Claude, an AI assistant");
    mock.add_response("I can help you with that!");

    // Turn 1
    let request1 = CreateMessageRequest {
        messages: vec![Message::user("Who are you?")],
        ..Default::default()
    };
    let response1 = mock.create_message(request1).await.unwrap();
    assert!(response1.content[0].text().contains("Claude"));

    // Turn 2
    let request2 = CreateMessageRequest {
        messages: vec![
            Message::user("Who are you?"),
            Message::assistant(response1.content),
            Message::user("Can you help me?")
        ],
        ..Default::default()
    };
    let response2 = mock.create_message(request2).await.unwrap();
    assert!(response2.content[0].text().contains("help"));
}
```

### Example 4: Streaming Response

```rust
#[tokio::test]
async fn test_streaming() {
    let mut mock = MockLLM::new();
    mock.add_streaming_response(vec![
        "Hello",
        " ",
        "world",
        "!"
    ]);

    let request = CreateMessageRequest {
        messages: vec![Message::user("Say hello")],
        ..Default::default()
    };

    let mut stream = mock.create_message_stream(request).await.unwrap();

    let mut accumulated = String::new();
    while let Some(delta) = stream.next().await {
        if let StreamEvent::ContentBlockDelta { delta, .. } = delta? {
            accumulated.push_str(&delta.text);
        }
    }

    assert_eq!(accumulated, "Hello world!");
}
```

### Example 5: Error Response

```rust
#[tokio::test]
async fn test_error_response() {
    let mut mock = MockLLM::new();
    mock.add_error(ApiError::RateLimitError {
        retry_after: 60
    });

    let request = CreateMessageRequest {
        messages: vec![Message::user("Test")],
        ..Default::default()
    };

    let result = mock.create_message(request).await;

    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err().downcast::<ApiError>().unwrap(),
        ApiError::RateLimitError { .. }
    ));
}
```

---

## Performance Considerations

**Memory Usage:**
- Queue size typically < 10 responses
- Each RecordedRequest ~1KB
- Total: < 100KB for typical test

**Response Time:**
- No network I/O (instant responses)
- Mutex contention minimal (single-threaded tests)
- Target: < 1ms per request

**Cleanup:**
- Reset between tests to avoid cross-test contamination
- No persistent state

---

## Future Enhancements

**Phase 2 (If Needed):**
- Response templates for common patterns
- Conditional responses based on request content
- Latency simulation for realistic timing
- Usage tracking (tokens, costs)

**NOT Planned:**
- Real API proxying (defeats purpose of mock)
- ML-based response generation (too complex)
- Multi-tenant support (not needed for tests)

---

## Contract Verification

**This module succeeds when:**

1. ✅ Implements ApiClient trait correctly
2. ✅ Queue-based responses work predictably
3. ✅ Records all requests for verification
4. ✅ Supports streaming responses
5. ✅ Supports tool use responses
6. ✅ Handles errors appropriately
7. ✅ Thread-safe for async tests
8. ✅ Zero external dependencies

**This module fails if:**

- Responses don't match real API format
- Queue behavior is unpredictable
- Request recording incomplete
- Memory leaks occur
- Tests become flaky

---

## See Also

- [TestSession Specification](test_session_spec.md)
- [E2E Testing Architecture](../architecture/e2e_testing_architecture.md)
- [ApiClient Trait](../../crates/api_client/src/lib.rs)
