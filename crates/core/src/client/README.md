# Anthropic API Client

A production-ready, secure Rust client for the Anthropic API with streaming support.

## Features

- **Secure API Key Handling**: Uses `zeroize` and `secrecy` to prevent key leakage
- **Server-Sent Events (SSE) Streaming**: Real-time message streaming with proper SSE parsing
- **Type Safety**: Comprehensive type definitions matching the Anthropic API
- **Error Sanitization**: Automatic redaction of API keys from error messages
- **Timeout Management**: Configurable request timeouts
- **HTTP/2 Support**: Modern HTTP client with connection pooling

## Quick Start

### Load API Key and Create Client

```rust
use claude_code_core::client::{Client, Config};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load from ~/.claude-msec-k
    let config = Config::from_default_location().await?;
    let client = Client::new(config);

    Ok(())
}
```

### Non-Streaming Request

```rust
use claude_code_core::client::{CreateMessageRequest, Message};

let request = CreateMessageRequest::new(
    "claude-3-haiku-20240307",
    vec![Message::user("Hello, Claude!")],
    1024,
);

let response = client.create_message(request).await?;

for block in response.content {
    match block {
        ContentBlock::Text { text } => println!("{}", text),
    }
}
```

### Streaming Request

```rust
use futures::StreamExt;

let request = CreateMessageRequest::new(
    "claude-3-haiku-20240307",
    vec![Message::user("Count to 10")],
    1024,
)
.with_stream(true);

let mut stream = client.create_message_stream(request).await?;

while let Some(result) = stream.next().await {
    match result? {
        StreamEvent::ContentBlockDelta { delta, .. } => {
            let ContentDelta::TextDelta { text } = delta;
            print!("{}", text);
        }
        _ => {}
    }
}
```

## Security Features

### API Key Protection

The client implements multiple layers of security for API key handling:

1. **Zeroization**: Keys are automatically zeroed in memory when dropped
2. **Secret Wrapper**: Keys are wrapped in `Secret<T>` to prevent accidental logging
3. **Debug Sanitization**: Custom Debug implementations never show keys
4. **Error Sanitization**: API keys are automatically redacted from error messages

```rust
// API key never appears in logs or debug output
let key = ApiKey::from_file("~/.claude-msec-k").await?;
println!("{:?}", key);  // Output: "ApiKey([REDACTED])"
```

### File Permissions Validation

On Unix systems, the client validates that API key files have secure permissions:

```bash
chmod 600 ~/.claude-msec-k  # Required: owner read/write only
```

## Architecture

### Module Structure

```
client/
├── mod.rs       - Main Client implementation
├── config.rs    - Configuration and API key loading
├── types.rs     - Request/response type definitions
├── stream.rs    - SSE stream parsing
└── error.rs     - Error types with sanitization
```

### Type Hierarchy

```
Client
  └── uses Config
        └── contains Secret<ApiKey>
              └── wraps ApiKey (with Zeroize)

CreateMessageRequest
  └── contains Vec<Message>

MessageResponse
  └── contains Vec<ContentBlock>
  └── contains Usage

StreamEvent (enum)
  ├── MessageStart
  ├── ContentBlockDelta
  ├── MessageDelta
  └── MessageStop
```

## Examples

See the `examples/` directory for complete, runnable examples:

- `simple_test.rs` - Basic non-streaming request
- `stream_test.rs` - Real-time streaming with text output
- `api_test.rs` - Comprehensive test suite

Run examples:

```bash
cargo run --example simple_test
cargo run --example stream_test
```

## Configuration Options

```rust
let config = Config::from_default_location().await?
    .with_api_url("https://api.anthropic.com".to_string())
    .with_api_version("2023-06-01".to_string())
    .with_timeout_secs(120);

let client = Client::new(config);
```

## Error Handling

All errors are sanitized to prevent API key leakage:

```rust
match client.create_message(request).await {
    Ok(response) => { /* handle response */ }
    Err(e) => {
        // API keys are automatically redacted
        eprintln!("Error: {}", e);

        // Get sanitized message explicitly
        let safe_msg = e.sanitized_message();
    }
}
```

## API Key File Format

The API key file should contain only the key:

```
sk-ant-your-key-here
```

No extra whitespace or comments. The file should have 600 permissions.

## Model IDs

Common model IDs (check Anthropic docs for current models):

- `claude-3-haiku-20240307` - Fast, cost-effective
- `claude-3-sonnet-20240229` - Balanced performance
- `claude-3-opus-20240229` - Most capable

## Testing

Run the test suite:

```bash
cargo test -p claude-code-core
```

Run with real API:

```bash
cargo run --example simple_test
```

## Performance

- Minimal allocations in hot paths
- Connection pooling via reqwest
- Zero-copy SSE parsing where possible
- Efficient buffering for streaming

## Dependencies

Key dependencies:

- `reqwest` - HTTP client with streaming
- `futures` - Stream trait implementation
- `bytes` - Efficient byte handling
- `zeroize` - Secure memory clearing
- `secrecy` - Secret wrapper type
- `pin-project` - Pin projection for streams

## Safety Guarantees

1. **Memory Safety**: No unsafe code in client implementation
2. **API Key Safety**: Keys never leak through Debug, Display, or errors
3. **Stream Safety**: Proper Pin projection for async streams
4. **Thread Safety**: All types are Send + Sync where appropriate

## Limitations

- Currently only supports text content (no images/tools yet)
- SSE parsing assumes UTF-8 encoding
- No retry logic (caller must implement)
- No rate limiting (caller must implement)

## Future Enhancements

Potential improvements:

- [ ] Tool/function calling support
- [ ] Image/multimodal content
- [ ] Automatic retry with exponential backoff
- [ ] Rate limiting
- [ ] Request batching
- [ ] Response caching
- [ ] Metrics/observability

## License

MIT OR Apache-2.0 (matches workspace license)
