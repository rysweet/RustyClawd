# Agent Tool - Multi-Agent Orchestration

## Overview

The Agent tool is the **MOST CRITICAL** tool in the system, enabling sophisticated multi-agent workflows through Claude API integration. It allows the main agent to delegate specialized tasks to sub-agents with isolated contexts.

## Key Features

1. **Agent Invocation**: Calls Claude API with specialized sub-agent prompts
2. **Context Forking**: Clones parent context and adds agent-specific system prompt
3. **Model Selection**: Supports haiku/sonnet/opus or custom model selection
4. **Streaming**: Real-time streaming of agent responses
5. **Resume Support**: Can resume previous agent executions
6. **Isolation**: Each agent operates in its own isolated context

## Architecture

```
Main Agent
    ↓
Agent Tool (Fork Context)
    ↓
Claude API (with agent system prompt)
    ↓
Sub-Agent Response
    ↓
Return to Main Agent
```

## Parameters

```rust
pub struct AgentParams {
    /// Brief 3-5 word description (for logging/UI)
    pub description: String,

    /// Full task/prompt for the agent to execute
    pub prompt: String,

    /// Agent type - loads from .claude/agents/{subagent_type}.md
    pub subagent_type: String,

    /// Optional model override ("haiku", "sonnet", "opus", or full model ID)
    pub model: Option<String>,

    /// Optional agent ID to resume previous execution
    pub resume: Option<String>,
}
```

## Agent Prompts

Agent prompts are markdown files stored in `.claude/agents/`:

```
.claude/
└── agents/
    ├── code_reviewer.md
    ├── test_writer.md
    ├── debugger.md
    └── optimizer.md
```

Each file contains the system prompt that defines the agent's:
- Role and purpose
- Capabilities and constraints
- Output format
- Guidelines and best practices

## Model Selection

The tool supports flexible model selection:

| Input | Resolved Model |
|-------|---------------|
| `"haiku"` | `claude-3-5-haiku-20241022` |
| `"sonnet"` | `claude-3-5-sonnet-20241022` |
| `"opus"` | `claude-opus-4-20250514` |
| `"claude-custom-..."` | Custom model ID (pass-through) |
| `None` | `claude-3-5-sonnet-20241022` (default) |

## Usage Examples

### Basic Agent Invocation

```rust
use claude_code_tools::{AgentTool, Tool, ToolContext};

let tool = AgentTool;

let params = AgentParams {
    description: "Review code quality".to_string(),
    prompt: "Review this function for bugs and improvements:\n\n```rust\n...\n```".to_string(),
    subagent_type: "code_reviewer".to_string(),
    model: Some("sonnet".to_string()),
    resume: None,
};

let ctx = ToolContext::default();
let mut stream = tool.execute(params, &ctx).await?;

while let Some(event) = stream.next().await {
    match event {
        ToolEvent::Result(output) => {
            println!("Agent response: {}", output.response);
        }
        ToolEvent::Error { message } => {
            eprintln!("Error: {}", message);
        }
        _ => {}
    }
}
```

### Using Different Models

```rust
// Fast, cost-effective for simple tasks
let params = AgentParams {
    model: Some("haiku".to_string()),
    // ... other fields
};

// Balanced performance (default)
let params = AgentParams {
    model: Some("sonnet".to_string()),
    // ... other fields
};

// Maximum capability for complex tasks
let params = AgentParams {
    model: Some("opus".to_string()),
    // ... other fields
};
```

### Resuming Agent Execution

```rust
// First execution
let params = AgentParams {
    description: "Long-running task".to_string(),
    prompt: "Start processing...".to_string(),
    subagent_type: "worker".to_string(),
    model: None,
    resume: None,
};

// ... get agent_id from output

// Resume later
let params = AgentParams {
    description: "Continue task".to_string(),
    prompt: "Continue from where we left off...".to_string(),
    subagent_type: "worker".to_string(),
    model: None,
    resume: Some(agent_id),
};
```

## Output

The tool returns `AgentOutput` with:

```rust
pub struct AgentOutput {
    /// Unique agent execution ID (for resuming)
    pub agent_id: String,

    /// Name of the invoked agent
    pub agent_name: String,

    /// Complete response text from the agent
    pub response: String,

    /// Model ID that was used
    pub model: String,

    /// Token usage statistics
    pub tokens_used: TokenUsage,
}

pub struct TokenUsage {
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub total_tokens: u32,
}
```

## Event Stream

The tool streams progress events during execution:

1. **Progress(10%)**: Loading agent prompt
2. **Progress(30%)**: Preparing context
3. **Progress(50%)**: Invoking agent
4. **Progress(70%)**: Agent responding (first content)
5. **Progress(80%)**: Receiving response (periodic updates)
6. **Progress(95%)**: Finalizing response
7. **Result**: Final output with complete response

## Error Handling

The tool handles various error conditions:

- **Agent prompt not found**: If `.claude/agents/{type}.md` doesn't exist
- **API configuration errors**: If Claude API config is invalid
- **Stream errors**: Network issues, API errors
- **Agent execution errors**: Runtime errors during agent execution

All errors are returned as `ToolEvent::Error` events in the stream.

## Testing

### Unit Tests

```bash
# Run all agent tests
cargo test --package claude-code-tools agent

# Run specific test
cargo test --package claude-code-tools test_model_resolution
```

### Integration Tests

Integration tests require a valid API key:

```bash
# Set API key
export ANTHROPIC_API_KEY="your-key-here"

# Run integration tests
cargo test --package claude-code-tools agent --ignored
```

### Example Program

```bash
# Run the example
export ANTHROPIC_API_KEY="your-key-here"
cargo run --example agent_example
```

## Best Practices

### 1. Agent Design

- **Single Responsibility**: Each agent should have one clear purpose
- **Clear Contracts**: Define inputs, outputs, and constraints
- **Isolation**: Agents should be self-contained
- **Composability**: Design agents to work together

### 2. Model Selection

- **Haiku**: Simple tasks, fast responses, cost-effective
- **Sonnet**: Default choice, balanced performance
- **Opus**: Complex reasoning, difficult problems

### 3. Prompt Engineering

- **Be Specific**: Clear, detailed task descriptions
- **Provide Context**: Include relevant information
- **Set Expectations**: Define output format and constraints
- **Include Examples**: Show desired output format

### 4. Error Handling

- **Check for Errors**: Always handle `ToolEvent::Error`
- **Retry Logic**: Implement retry for transient failures
- **Fallback**: Have fallback strategies for critical tasks
- **Logging**: Log agent executions for debugging

## Performance Considerations

### Token Usage

- Input tokens: System prompt + user prompt + context
- Output tokens: Agent response length
- Total cost: Based on model pricing

### Streaming Benefits

- **Real-time feedback**: See responses as they're generated
- **Early termination**: Can stop if response is sufficient
- **Progress tracking**: Monitor long-running operations

### Optimization Tips

1. **Use Haiku for simple tasks**: 10x cheaper than Sonnet
2. **Keep prompts concise**: Reduce input token usage
3. **Reuse agents**: Resume instead of creating new agents
4. **Batch similar tasks**: Process multiple items in one call

## Example Agent Prompts

### Code Reviewer Agent

```markdown
# Code Reviewer Agent

You are a specialized code review agent. Your role is to:
- Identify bugs and potential issues
- Suggest improvements and optimizations
- Check for best practices and patterns
- Highlight security concerns

## Output Format

Always structure your review as:
1. **Summary**: Overall assessment
2. **Issues**: Specific problems found
3. **Recommendations**: Suggested improvements
4. **Security**: Any security concerns
```

### Test Writer Agent

```markdown
# Test Writer Agent

You are a test generation specialist. Your role is to:
- Write comprehensive unit tests
- Cover edge cases and error conditions
- Follow testing best practices
- Generate clear, maintainable test code

## Guidelines

1. Test both happy paths and error cases
2. Use descriptive test names
3. Include assertions with clear messages
4. Document complex test scenarios
```

## Integration with Main System

The Agent tool is designed to integrate seamlessly with the main Claude Code system:

1. **Tool Discovery**: Registered in tool registry
2. **Schema Generation**: JSON schema for parameters
3. **Stream Processing**: Compatible with existing event system
4. **Context Management**: Works with conversation context

## Future Enhancements

Potential improvements for the Agent tool:

1. **Parallel Agents**: Run multiple agents concurrently
2. **Agent Memory**: Persistent memory across invocations
3. **Tool Access**: Allow agents to use other tools
4. **Agent Networks**: Multi-level agent hierarchies
5. **Caching**: Cache agent responses for identical prompts

## Troubleshooting

### Agent Prompt Not Found

**Error**: `Agent prompt not found: {type}`

**Solution**: Ensure `.claude/agents/{type}.md` exists in the working directory

### API Key Issues

**Error**: `Failed to load API config`

**Solution**: Set `ANTHROPIC_API_KEY` environment variable or create `~/.claude-msec-k` file

### Stream Errors

**Error**: `Stream error: ...`

**Solution**: Check network connectivity, API key validity, and rate limits

### Token Limit Exceeded

**Error**: `HTTP 400: max_tokens_to_sample too large`

**Solution**: Reduce prompt size or use a smaller model

## References

- [Anthropic API Documentation](https://docs.anthropic.com/claude/reference)
- [Claude Models Overview](https://docs.anthropic.com/claude/docs/models-overview)
- [Tool System Architecture](../README.md)
- [Example Code](../../examples/agent_example.rs)
