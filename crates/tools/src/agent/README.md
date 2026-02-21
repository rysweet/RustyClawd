# Agent Tool - Multi-Agent Orchestration

## Overview

The Agent tool is the **MOST CRITICAL** tool in the system, enabling sophisticated multi-agent workflows through Claude API integration. It allows the main agent to delegate specialized tasks to sub-agents with isolated contexts.

## Key Features

1. **Agent Invocation**: Calls Claude API with specialized sub-agent prompts
2. **Context Cloning**: Clones parent context and adds agent-specific system prompt
3. **Model Selection**: Supports haiku/sonnet/opus or custom model selection
4. **Streaming**: Real-time streaming of agent responses
5. **Resume Support**: Can resume previous agent executions
6. **Isolation**: Each agent operates in its own isolated context

## Architecture

```
Main Agent (with context A, B, C)
    ↓
Agent Tool (receives ToolContext copy, loads agent system prompt)
    ↓
Claude API (system=agent_prompt, user=provided_prompt)
    ↓
Sub-Agent Response (generates output independently)
    ↓
Return AgentOutput (response text only, no context changes)
    ↓
Main Agent (context unchanged: A, B, C)
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

## Context Management (Context Isolation & Cloning)

### How Agent Contexts Work

When you invoke an agent using the Task tool, the **ToolContext is cloned** but the agent operates in **complete isolation**. This is NOT traditional context forking - it's more accurately described as **context cloning and isolation**.

### Context Flow

```
Step 1: Parent Context
┌─────────────────────────────────────┐
│ cwd: /project                       │
│ debug: false                        │
│ metadata: {request_id: "123"}       │
│ execution_context: NonInteractive   │
└─────────────────────────────────────┘
         ↓
Step 2: Clone for Agent Execution
┌─────────────────────────────────────┐
│ cwd: /project (copied)              │  ← Same values
│ debug: false (copied)               │  ← Same values
│ metadata: {request_id: "123"} (cpy) │  ← Same values
│ execution_context: NonInteractive   │  ← Same values
└─────────────────────────────────────┘
         ↓
Step 3: Agent Execution
┌─────────────────────────────────────┐
│ Agent System Prompt:                │
│ (Loaded from .claude/agents/X.md)   │
│                                     │
│ User Prompt: (params.prompt)        │
│                                     │
│ API Call → Claude → Response        │
└─────────────────────────────────────┘
         ↓
Step 4: Return to Parent
┌─────────────────────────────────────┐
│ Parent Context: UNCHANGED           │
│ (Still has original values A, B, C) │
│                                     │
│ Agent Output:                       │
│ - agent_id                          │
│ - response (text only)              │
│ - model_id                          │
│ - tokens_used                       │
└─────────────────────────────────────┘
```

### Key Characteristics

#### 1. Context is NOT Truly "Forked"

**Context forking** (as known in process management) means child processes inherit and can modify parent state. **Agent contexts are NOT like this.**

Instead:
- ToolContext is **cloned** (shallow copy of struct fields)
- Agent receives the **clone**, not shared references
- Changes by the agent **cannot affect** parent context
- Parent context is **read-only** to the agent (via cloned fields)

#### 2. What the Agent Actually Receives

The agent tool passes:

```rust
// From parent ToolContext (cloned values):
pub struct ToolContext {
    pub cwd: std::path::PathBuf,        // File system path
    pub debug: bool,                    // Debug flag
    pub metadata: serde_json::Value,    // JSON metadata
    pub execution_context: ExecutionContext,  // TUI or NonInteractive
}

// NOT passed to agent:
// - Conversation history
// - Previous message exchanges
// - Parent execution state
// - Sibling agent outputs
```

#### 3. What the Agent Executes

```rust
// The agent makes a single API call:
let request = CreateMessageRequest::new(
    model_id,
    vec![Message::user(params.prompt)],  // ← User prompt only
    4096,
)
.with_system(agent_system_prompt)        // ← Agent role definition
.with_temperature(0.7);

// The agent responds based ONLY on:
// 1. Its system prompt (role/capabilities)
// 2. The provided user prompt
// 3. Its training knowledge
```

#### 4. What Returns to Parent

```rust
pub struct AgentOutput {
    pub agent_id: String,              // Unique ID for resumption
    pub agent_name: String,            // Agent type name
    pub response: String,              // Text response (only!)
    pub model: String,                 // Model used
    pub tokens_used: TokenUsage,       // Statistics
}

// NOTE: Only the response text is transferred back
// No modified context, no shared state changes
```

### When Agents Are Isolated (Good)

Use parallel agents when they need **independent perspectives** on the same problem:

```rust
// Agent 1: Analyze code quality
let result1 = invoke_agent("reviewer", "Review this function").await?;

// Agent 2: Analyze performance (independent, sees original context)
let result2 = invoke_agent("optimizer", "Optimize this function").await?;

// Agent 3: Analyze security (independent, sees original context)
let result3 = invoke_agent("security", "Check security of this function").await?;

// All agents started with identical contexts
// All operate independently
// Results combine at parent level
```

### When Agents Cannot Share (Limitation)

Agents **cannot** see each other's work in a pipeline:

```rust
// ❌ This doesn't work as you might expect:

// Agent 1 produces output
let output1 = invoke_agent("analyzer", "Analyze file").await?;

// Agent 2 wants to see Agent 1's analysis
// But can't - isolated context!
let output2 = invoke_agent("refactorer",
    "Refactor based on analysis").await?;  // Doesn't see output1

// ✓ Solution: Pass output1's response as part of user prompt:
let output2 = invoke_agent("refactorer",
    format!("Refactor based on this analysis:\n{}", output1.response)
).await?;
```

### Context Variables and Behavior

| Variable | Source | Passed to Agent? | Can Agent Modify? |
|----------|--------|-----------------|------------------|
| `cwd` | Parent | Yes (cloned) | No (agent receives copy) |
| `debug` | Parent | Yes (cloned) | No (agent receives copy) |
| `metadata` | Parent | Yes (cloned) | No (agent receives copy) |
| `execution_context` | Parent | Yes (cloned) | No (agent receives copy) |
| User prompt | Params | Yes (passed) | Agent responds to it |
| System prompt | `.claude/agents/*.md` | Yes (loaded fresh) | Agent uses as role |
| API responses | Claude | Yes (streamed) | Agent generates them |

### Examples

#### Example 1: Parallel Independent Analysis

```rust
// All agents see the same cwd, metadata, etc.
// All agents start fresh
// All operate independently

let code = r#"
fn calculate(x: i32) -> i32 {
    x * 2 + 1
}
"#;

// Parallel execution - all see same context
let review = invoke_agent("reviewer", format!("Review:\n{}", code)).await?;
let tests = invoke_agent("tester", format!("Write tests for:\n{}", code)).await?;
let perf = invoke_agent("optimizer", format!("Optimize:\n{}", code)).await?;

// Each agent produced independent analysis
// Parent combines: review.response + tests.response + perf.response
```

#### Example 2: Sequential Processing (Workaround)

```rust
// Agent 1: Analyze
let analysis = invoke_agent("analyzer", "Analyze error in logs").await?;

// Agent 2: Uses Agent 1's response as input
// (Not shared context - explicit prompt inclusion)
let solution = invoke_agent("fixer",
    format!("Given this analysis:\n{}\n\nSuggest a fix:", analysis.response)
).await?;

// Each invocation is independent
// Data flows through explicit prompts, not context sharing
```

#### Example 3: Resume Support

```rust
// First execution
let params1 = AgentParams {
    subagent_type: "worker".to_string(),
    prompt: "Start processing large dataset...".to_string(),
    resume: None,  // First run
    ..
};

let output1 = invoke_agent(params1).await?;
let agent_id = output1.agent_id;  // Save ID

// Later: Resume same agent
let params2 = AgentParams {
    subagent_type: "worker".to_string(),
    prompt: "Continue from previous state...".to_string(),
    resume: Some(agent_id),  // Resume by ID
    ..
};

let output2 = invoke_agent(params2).await?;

// Resume allows continuation BUT still starts fresh context
// Previous state must be encoded in the resumed prompt
```

### Implementation Details

The agent tool implementation confirms this behavior:

```rust
// From agent.rs line 134-211:

// 1. Clone the context
let cwd = ctx.cwd.clone();        // ← Cloned
let debug = ctx.debug;             // ← Copied (primitive)

// 2. Load agent system prompt (not from context)
let agent_system_prompt =
    Self::load_agent_prompt(&agent_type, &cwd).await?;

// 3. Create fresh API request
let messages = vec![
    Message::user(params.prompt.clone()),  // ← User prompt only
];

let request = CreateMessageRequest::new(model_id, messages, 4096)
    .with_system(agent_system_prompt)      // ← Agent role
    .with_temperature(0.7);

// 4. API call - starts fresh, agent responds independently
let mut event_stream = client.create_message_stream(request).await?;

// 5. Return only response text
yield ToolEvent::Result(AgentOutput {
    agent_id,
    agent_name: agent_type.clone(),
    response: response_text,              // ← Only text returned
    model: model_id,
    tokens_used: TokenUsage { ... },
});
```

### Common Misconceptions

| Misconception | Reality |
|---------------|---------|
| "Agents fork context like processes" | Agents receive cloned context fields, cannot modify them |
| "Agents see conversation history" | Agents only see the user prompt provided to them |
| "Agent changes persist in parent" | All changes are isolated; only response text returns |
| "Agents can modify metadata" | Agents receive copies; modifications don't affect parent |
| "Context forking enables agent communication" | Context isolation prevents this; use explicit prompts instead |
| "Multiple agents share state" | Each agent invocation is independent with cloned context |

### Best Practices for Context Isolation

1. **Treat agents as stateless**: Each invocation starts fresh
2. **Pass data explicitly**: Use prompts, not shared context
3. **Combine results manually**: Orchestrate agent outputs at parent level
4. **Use resume for long tasks**: Resume by ID, not context sharing
5. **Design for parallelization**: Since agents are isolated, they parallelize well
6. **Document dependencies**: If Agent B needs Agent A's output, make it explicit in prompts

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
| `"haiku"` | `claude-haiku-4-5-20251001` |
| `"sonnet"` | `claude-sonnet-4-6` |
| `"opus"` | `claude-opus-4-6` |
| `"claude-custom-..."` | Custom model ID (pass-through) |
| `None` | `claude-sonnet-4-6` (default) |

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
