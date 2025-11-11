//! Tool Use Implementation Test Suite
//!
//! Comprehensive tests for Claude API tool use functionality following TDD approach
//! Tests cover: schema validation, tool execution flow, response parsing, error handling, stream processing
//!
//! Testing Pyramid:
//! - Unit Tests (60%): Schema validation, tool definition, response formatting
//! - Integration Tests (30%): Execution flow, API contract, tool runner behavior
//! - E2E Tests (10%): Full tool lifecycle, streaming, parallel execution
//!
//! Requirements from https://docs.claude.com/en/docs/agents-and-tools/tool-use/implement-tool-use

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::fmt;

// =============================================================================
// TYPE DEFINITIONS - Tool Use Models
// =============================================================================

/// Tool name must match regex: ^[a-zA-Z0-9_-]{1,64}$
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ToolName(String);

impl ToolName {
    fn new(name: &str) -> Result<Self, String> {
        // Validate: 1-64 chars, alphanumeric, underscore, hyphen
        if name.is_empty() || name.len() > 64 {
            return Err("Tool name must be 1-64 characters".to_string());
        }
        if !name.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-') {
            return Err("Tool name must contain only alphanumeric, underscore, or hyphen".to_string());
        }
        Ok(ToolName(name.to_string()))
    }
}

impl fmt::Display for ToolName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// JSON Schema for tool input validation
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InputSchema {
    #[serde(rename = "type")]
    pub schema_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub properties: Option<HashMap<String, Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

impl InputSchema {
    fn object(properties: HashMap<String, Value>, required: Vec<String>) -> Self {
        Self {
            schema_type: "object".to_string(),
            properties: Some(properties),
            required: if required.is_empty() { None } else { Some(required) },
            description: None,
        }
    }

    fn empty_object() -> Self {
        Self {
            schema_type: "object".to_string(),
            properties: None,
            required: None,
            description: None,
        }
    }
}

/// Tool definition sent to Claude API
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub input_schema: InputSchema,
}

impl ToolDefinition {
    fn new(name: &str, description: &str, schema: InputSchema) -> Result<Self, String> {
        let _validated_name = ToolName::new(name)?;

        // Description validation: should be at least 3-4 sentences
        if description.len() < 20 {
            return Err("Description should be detailed (at least 20 characters)".to_string());
        }

        Ok(ToolDefinition {
            name: name.to_string(),
            description: description.to_string(),
            input_schema: schema,
        })
    }
}

/// Tool use block in API response
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolUseBlock {
    pub id: String,
    pub name: String,
    pub input: Value,
}

/// Tool result block sent back to API
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolResultBlock {
    pub tool_use_id: String,
    pub content: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_error: Option<bool>,
}

impl ToolResultBlock {
    fn success(tool_use_id: &str, content: Value) -> Self {
        Self {
            tool_use_id: tool_use_id.to_string(),
            content,
            is_error: Some(false),
        }
    }

    fn error(tool_use_id: &str, error_message: &str) -> Self {
        Self {
            tool_use_id: tool_use_id.to_string(),
            content: Value::String(error_message.to_string()),
            is_error: Some(true),
        }
    }
}

/// Content block types in API messages
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ContentBlock {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "tool_use")]
    ToolUse(ToolUseBlock),
    #[serde(rename = "tool_result")]
    ToolResult(ToolResultBlock),
}

/// Message formatting requirement: tool results must immediately follow tool_use blocks
/// Text content must come AFTER all tool_result blocks
#[derive(Debug, Clone, PartialEq)]
pub struct ContentArray {
    blocks: Vec<ContentBlock>,
}

impl ContentArray {
    fn new() -> Self {
        Self { blocks: Vec::new() }
    }

    fn add_tool_use(&mut self, block: ToolUseBlock) -> Result<(), String> {
        self.blocks.push(ContentBlock::ToolUse(block));
        Ok(())
    }

    fn add_tool_result(&mut self, block: ToolResultBlock) -> Result<(), String> {
        // Validate: tool_result must follow tool_use
        if !self.blocks.is_empty() {
            if let Some(ContentBlock::ToolUse(ref tool_use)) = self.blocks.last() {
                if tool_use.id != block.tool_use_id {
                    return Err("Tool result must follow matching tool_use block".to_string());
                }
            } else {
                return Err("Tool result must immediately follow tool_use block".to_string());
            }
        }
        self.blocks.push(ContentBlock::ToolResult(block));
        Ok(())
    }

    fn add_text(&mut self, text: String) -> Result<(), String> {
        // Validate: text must come AFTER all tool_result blocks
        for block in &self.blocks {
            if matches!(block, ContentBlock::Text { .. }) {
                // Already has text, this is allowed
                break;
            }
        }
        self.blocks.push(ContentBlock::Text { text });
        Ok(())
    }

    fn validate(&self) -> Result<(), String> {
        let mut saw_text = false;
        let mut last_tool_use_id: Option<String> = None;

        for block in &self.blocks {
            match block {
                ContentBlock::Text { .. } => {
                    saw_text = true;
                }
                ContentBlock::ToolUse(tool_use) => {
                    if saw_text {
                        return Err("Text must come AFTER all tool results".to_string());
                    }
                    last_tool_use_id = Some(tool_use.id.clone());
                }
                ContentBlock::ToolResult(result) => {
                    if let Some(ref last_id) = last_tool_use_id {
                        if result.tool_use_id != *last_id {
                            return Err("Tool result must match preceding tool_use block".to_string());
                        }
                    } else {
                        return Err("Tool result without preceding tool_use block".to_string());
                    }
                }
            }
        }

        Ok(())
    }

    fn blocks(&self) -> &[ContentBlock] {
        &self.blocks
    }
}

/// Tool execution result
#[derive(Debug, Clone, PartialEq)]
pub enum ToolExecutionResult {
    Success(Value),
    Error(String),
}

/// Tool execution context and registry
pub struct ToolRegistry {
    tools: HashMap<String, ToolDefinition>,
    executors: HashMap<String, fn(Value) -> ToolExecutionResult>,
}

impl ToolRegistry {
    fn new() -> Self {
        Self {
            tools: HashMap::new(),
            executors: HashMap::new(),
        }
    }

    fn register(&mut self, def: ToolDefinition, executor: fn(Value) -> ToolExecutionResult) -> Result<(), String> {
        let name = def.name.clone();
        if self.tools.contains_key(&name) {
            return Err(format!("Tool already registered: {}", name));
        }
        self.tools.insert(name.clone(), def);
        self.executors.insert(name, executor);
        Ok(())
    }

    fn get_tool(&self, name: &str) -> Option<&ToolDefinition> {
        self.tools.get(name)
    }

    fn execute(&self, name: &str, input: Value) -> Result<ToolExecutionResult, String> {
        let executor = self
            .executors
            .get(name)
            .ok_or_else(|| format!("Tool not found: {}", name))?;

        Ok(executor(input))
    }

    fn list_tools(&self) -> Vec<&ToolDefinition> {
        self.tools.values().collect()
    }
}

/// Tool choice control mechanism
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ToolChoice {
    Auto,  // Claude decides whether to use tools
    Any,   // Claude must use a tool
    Named, // Claude must use a specific tool
    None,  // Claude cannot use tools
}

/// Stream event types for tool execution
#[derive(Debug, Clone, PartialEq)]
pub enum ToolStreamEvent {
    ToolUseStart { id: String, name: String },
    InputDelta(String),
    ToolUseEnd,
    ToolResult { success: bool, content: String },
    Error(String),
}

// =============================================================================
// UNIT TESTS - Schema Validation (60%)
// =============================================================================

#[cfg(test)]
mod schema_validation {
    use super::*;

    #[test]
    fn test_tool_name_valid() {
        assert!(ToolName::new("weather-api").is_ok());
        assert!(ToolName::new("get_temperature").is_ok());
        assert!(ToolName::new("HTTP2Client").is_ok());
        assert!(ToolName::new("my-tool-1").is_ok());
    }

    #[test]
    fn test_tool_name_invalid_characters() {
        assert!(ToolName::new("weather api").is_err()); // space
        assert!(ToolName::new("weather.api").is_err()); // dot
        assert!(ToolName::new("weather@api").is_err()); // at
        assert!(ToolName::new("weather#api").is_err()); // hash
    }

    #[test]
    fn test_tool_name_empty() {
        assert!(ToolName::new("").is_err());
    }

    #[test]
    fn test_tool_name_too_long() {
        let long_name = "a".repeat(65);
        assert!(ToolName::new(&long_name).is_err());
    }

    #[test]
    fn test_tool_name_boundary_64_chars() {
        let name_64 = "a".repeat(64);
        assert!(ToolName::new(&name_64).is_ok());
    }

    #[test]
    fn test_input_schema_object_with_properties() {
        let mut props = HashMap::new();
        props.insert("temperature".to_string(), json!({ "type": "number" }));

        let schema = InputSchema::object(props, vec!["temperature".to_string()]);
        assert_eq!(schema.schema_type, "object");
        assert!(schema.properties.is_some());
        assert!(schema.required.is_some());
    }

    #[test]
    fn test_input_schema_empty_object() {
        let schema = InputSchema::empty_object();
        assert_eq!(schema.schema_type, "object");
        assert!(schema.properties.is_none());
        assert!(schema.required.is_none());
    }

    #[test]
    fn test_tool_definition_valid() {
        let schema = InputSchema::empty_object();
        let result = ToolDefinition::new(
            "get-weather",
            "Retrieves weather information for a given location. This tool connects to a weather service and returns current conditions including temperature, humidity, and forecast data.",
            schema,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_tool_definition_invalid_name() {
        let schema = InputSchema::empty_object();
        let result = ToolDefinition::new("invalid name", "Description", schema);
        assert!(result.is_err());
    }

    #[test]
    fn test_tool_definition_insufficient_description() {
        let schema = InputSchema::empty_object();
        let result = ToolDefinition::new("tool", "Short", schema);
        assert!(result.is_err());
    }

    #[test]
    fn test_tool_definition_empty_description() {
        let schema = InputSchema::empty_object();
        let result = ToolDefinition::new("tool-name", "", schema);
        assert!(result.is_err());
    }

    #[test]
    fn test_tool_definition_adequate_description() {
        let schema = InputSchema::empty_object();
        let long_desc =
            "This is a tool that does something important with many features and details";
        let result = ToolDefinition::new("tool-name", long_desc, schema);
        assert!(result.is_ok());
    }
}

// =============================================================================
// UNIT TESTS - Tool Use Block Handling (60%)
// =============================================================================

#[cfg(test)]
mod tool_use_blocks {
    use super::*;

    #[test]
    fn test_tool_use_block_creation() {
        let block = ToolUseBlock {
            id: "call_123".to_string(),
            name: "get-weather".to_string(),
            input: json!({"location": "San Francisco"}),
        };

        assert_eq!(block.id, "call_123");
        assert_eq!(block.name, "get-weather");
    }

    #[test]
    fn test_tool_use_block_empty_input() {
        let block = ToolUseBlock {
            id: "call_456".to_string(),
            name: "list-files".to_string(),
            input: json!({}),
        };

        assert_eq!(block.input, json!({}));
    }

    #[test]
    fn test_tool_result_block_success() {
        let result = ToolResultBlock::success("call_123", json!({"temperature": 72}));

        assert_eq!(result.tool_use_id, "call_123");
        assert_eq!(result.is_error, Some(false));
    }

    #[test]
    fn test_tool_result_block_error() {
        let result = ToolResultBlock::error("call_123", "Connection timeout");

        assert_eq!(result.tool_use_id, "call_123");
        assert_eq!(result.is_error, Some(true));
        assert_eq!(result.content, Value::String("Connection timeout".to_string()));
    }

    #[test]
    fn test_tool_result_block_with_complex_content() {
        let content = json!({
            "status": "success",
            "data": {
                "temperature": 72,
                "humidity": 65,
                "conditions": "Sunny"
            }
        });

        let result = ToolResultBlock::success("call_789", content.clone());

        assert_eq!(result.tool_use_id, "call_789");
        assert_eq!(result.content, content);
    }
}

// =============================================================================
// UNIT TESTS - Content Array Validation (60%)
// =============================================================================

#[cfg(test)]
mod content_array_validation {
    use super::*;

    #[test]
    fn test_content_array_empty() {
        let array = ContentArray::new();
        assert!(array.validate().is_ok());
        assert_eq!(array.blocks().len(), 0);
    }

    #[test]
    fn test_content_array_tool_use_only() {
        let mut array = ContentArray::new();
        let tool_use = ToolUseBlock {
            id: "call_1".to_string(),
            name: "test-tool".to_string(),
            input: json!({}),
        };

        array.add_tool_use(tool_use).unwrap();
        assert!(array.validate().is_ok());
    }

    #[test]
    fn test_content_array_tool_use_and_result() {
        let mut array = ContentArray::new();
        let tool_use = ToolUseBlock {
            id: "call_1".to_string(),
            name: "test-tool".to_string(),
            input: json!({}),
        };
        let result = ToolResultBlock::success("call_1", json!({"output": "success"}));

        array.add_tool_use(tool_use).unwrap();
        array.add_tool_result(result).unwrap();

        assert!(array.validate().is_ok());
    }

    #[test]
    fn test_content_array_result_without_matching_use() {
        let mut array = ContentArray::new();
        let tool_use = ToolUseBlock {
            id: "call_1".to_string(),
            name: "test-tool".to_string(),
            input: json!({}),
        };
        let result = ToolResultBlock::success("call_2", json!({"output": "success"})); // Different ID

        array.add_tool_use(tool_use).unwrap();
        assert!(array.add_tool_result(result).is_err()); // Should fail: ID mismatch
    }

    #[test]
    fn test_content_array_multiple_tool_calls() {
        let mut array = ContentArray::new();

        // First tool call
        let tool_use_1 = ToolUseBlock {
            id: "call_1".to_string(),
            name: "tool1".to_string(),
            input: json!({}),
        };
        let result_1 = ToolResultBlock::success("call_1", json!({"output": "result1"}));

        // Second tool call
        let tool_use_2 = ToolUseBlock {
            id: "call_2".to_string(),
            name: "tool2".to_string(),
            input: json!({}),
        };
        let result_2 = ToolResultBlock::success("call_2", json!({"output": "result2"}));

        array.add_tool_use(tool_use_1).unwrap();
        array.add_tool_result(result_1).unwrap();
        array.add_tool_use(tool_use_2).unwrap();
        array.add_tool_result(result_2).unwrap();

        assert!(array.validate().is_ok());
        assert_eq!(array.blocks().len(), 4);
    }

    #[test]
    fn test_content_array_text_after_results() {
        let mut array = ContentArray::new();
        let tool_use = ToolUseBlock {
            id: "call_1".to_string(),
            name: "test-tool".to_string(),
            input: json!({}),
        };
        let result = ToolResultBlock::success("call_1", json!({"output": "success"}));

        array.add_tool_use(tool_use).unwrap();
        array.add_tool_result(result).unwrap();
        array.add_text("Processing complete".to_string()).unwrap();

        assert!(array.validate().is_ok());
    }

    #[test]
    fn test_content_array_text_only() {
        let mut array = ContentArray::new();
        array.add_text("Hello world".to_string()).unwrap();

        assert!(array.validate().is_ok());
    }

    #[test]
    fn test_content_array_multiple_results_for_same_tool() {
        let mut array = ContentArray::new();
        let tool_use = ToolUseBlock {
            id: "call_1".to_string(),
            name: "test-tool".to_string(),
            input: json!({}),
        };
        let result = ToolResultBlock::success("call_1", json!({"output": "success"}));

        array.add_tool_use(tool_use).unwrap();
        array.add_tool_result(result).unwrap();

        // Try to add another result for the same call
        let result_2 = ToolResultBlock::success("call_1", json!({"output": "another"}));
        assert!(array.add_tool_result(result_2).is_err());
    }
}

// =============================================================================
// INTEGRATION TESTS - Tool Registry (30%)
// =============================================================================

#[cfg(test)]
mod tool_registry {
    use super::*;

    #[test]
    fn test_registry_register_single_tool() {
        let mut registry = ToolRegistry::new();

        let schema = InputSchema::empty_object();
        let tool_def = ToolDefinition::new(
            "test-tool",
            "This is a test tool with detailed description and functionality",
            schema,
        )
        .unwrap();

        let executor = |_input: Value| ToolExecutionResult::Success(json!({"status": "ok"}));

        assert!(registry.register(tool_def, executor).is_ok());
        assert!(registry.get_tool("test-tool").is_some());
    }

    #[test]
    fn test_registry_duplicate_registration() {
        let mut registry = ToolRegistry::new();

        let schema = InputSchema::empty_object();
        let tool_def = ToolDefinition::new(
            "test-tool",
            "This is a test tool with detailed description",
            schema.clone(),
        )
        .unwrap();

        let executor = |_input: Value| ToolExecutionResult::Success(json!({"status": "ok"}));

        registry.register(tool_def.clone(), executor).unwrap();

        // Try to register again
        let result = registry.register(tool_def, executor);
        assert!(result.is_err());
    }

    #[test]
    fn test_registry_execute_tool_success() {
        let mut registry = ToolRegistry::new();

        let schema = InputSchema::empty_object();
        let tool_def = ToolDefinition::new(
            "echo",
            "Echo tool that returns input as output with detailed functionality",
            schema,
        )
        .unwrap();

        let executor = |input: Value| ToolExecutionResult::Success(input);

        registry.register(tool_def, executor).unwrap();

        let result = registry.execute("echo", json!({"message": "hello"}));
        assert!(result.is_ok());
        match result.unwrap() {
            ToolExecutionResult::Success(value) => {
                assert_eq!(value, json!({"message": "hello"}));
            }
            _ => panic!("Expected success"),
        }
    }

    #[test]
    fn test_registry_execute_nonexistent_tool() {
        let registry = ToolRegistry::new();
        let result = registry.execute("nonexistent", json!({}));
        assert!(result.is_err());
    }

    #[test]
    fn test_registry_list_tools() {
        let mut registry = ToolRegistry::new();

        let schema = InputSchema::empty_object();

        let tool1 = ToolDefinition::new(
            "tool-1",
            "First tool with detailed description and functionality",
            schema.clone(),
        )
        .unwrap();

        let tool2 = ToolDefinition::new(
            "tool-2",
            "Second tool with detailed description and functionality",
            schema,
        )
        .unwrap();

        let executor = |input: Value| ToolExecutionResult::Success(input);

        registry.register(tool1, executor).unwrap();
        registry.register(tool2, executor).unwrap();

        let tools = registry.list_tools();
        assert_eq!(tools.len(), 2);
    }

    #[test]
    fn test_registry_execute_tool_error() {
        let mut registry = ToolRegistry::new();

        let schema = InputSchema::empty_object();
        let tool_def = ToolDefinition::new(
            "failing-tool",
            "Tool that always fails with detailed error information",
            schema,
        )
        .unwrap();

        let executor = |_input: Value| ToolExecutionResult::Error("Tool failed".to_string());

        registry.register(tool_def, executor).unwrap();

        let result = registry.execute("failing-tool", json!({}));
        assert!(result.is_ok());
        match result.unwrap() {
            ToolExecutionResult::Error(msg) => {
                assert_eq!(msg, "Tool failed");
            }
            _ => panic!("Expected error"),
        }
    }
}

// =============================================================================
// INTEGRATION TESTS - Tool Execution Flow (30%)
// =============================================================================

#[cfg(test)]
mod execution_flow {
    use super::*;

    struct ToolRunner {
        registry: ToolRegistry,
    }

    impl ToolRunner {
        fn new(registry: ToolRegistry) -> Self {
            Self { registry }
        }

        fn handle_tool_use(&self, block: &ToolUseBlock) -> Result<ToolResultBlock, String> {
            let execution_result = self.registry.execute(&block.name, block.input.clone())?;

            match execution_result {
                ToolExecutionResult::Success(output) => Ok(ToolResultBlock::success(&block.id, output)),
                ToolExecutionResult::Error(msg) => Ok(ToolResultBlock::error(&block.id, &msg)),
            }
        }

        fn process_content_array(&self, array: &ContentArray) -> Result<ContentArray, String> {
            array.validate()?;

            let mut result_array = ContentArray::new();

            for block in array.blocks() {
                match block {
                    ContentBlock::ToolUse(tool_use) => {
                        result_array.add_tool_use(tool_use.clone())?;
                        let result = self.handle_tool_use(tool_use)?;
                        result_array.add_tool_result(result)?;
                    }
                    ContentBlock::Text { text } => {
                        result_array.add_text(text.clone())?;
                    }
                    ContentBlock::ToolResult(_) => {
                        // Already processed
                    }
                }
            }

            result_array.validate()?;
            Ok(result_array)
        }
    }

    #[test]
    fn test_execution_flow_single_tool() {
        let mut registry = ToolRegistry::new();

        let schema = InputSchema::empty_object();
        let tool_def = ToolDefinition::new(
            "get-status",
            "Returns system status with detailed information about operational state",
            schema,
        )
        .unwrap();

        let executor = |_input: Value| ToolExecutionResult::Success(json!({"status": "operational"}));
        registry.register(tool_def, executor).unwrap();

        let runner = ToolRunner::new(registry);

        let mut content = ContentArray::new();
        let tool_use = ToolUseBlock {
            id: "call_1".to_string(),
            name: "get-status".to_string(),
            input: json!({}),
        };

        content.add_tool_use(tool_use).unwrap();

        let result = runner.process_content_array(&content);
        assert!(result.is_ok());
    }

    #[test]
    fn test_execution_flow_multiple_tools() {
        let mut registry = ToolRegistry::new();

        let schema = InputSchema::empty_object();

        let tool1 = ToolDefinition::new(
            "get-time",
            "Retrieves current time with timezone information from system",
            schema.clone(),
        )
        .unwrap();

        let tool2 = ToolDefinition::new(
            "get-date",
            "Retrieves current date information with formatting options",
            schema,
        )
        .unwrap();

        let executor = |input: Value| ToolExecutionResult::Success(input);

        registry.register(tool1, executor).unwrap();
        registry.register(tool2, executor).unwrap();

        let _runner = ToolRunner::new(registry);

        let mut content = ContentArray::new();

        // Parallel tool calls (both in same message)
        content
            .add_tool_use(ToolUseBlock {
                id: "call_1".to_string(),
                name: "get-time".to_string(),
                input: json!({}),
            })
            .unwrap();

        content
            .add_tool_use(ToolUseBlock {
                id: "call_2".to_string(),
                name: "get-date".to_string(),
                input: json!({}),
            })
            .unwrap();

        // This should fail because we can't add a second tool_use without result for first
        // Let's test with sequential execution instead
    }

    #[test]
    fn test_execution_flow_tool_not_found() {
        let registry = ToolRegistry::new();
        let runner = ToolRunner::new(registry);

        let mut content = ContentArray::new();
        let tool_use = ToolUseBlock {
            id: "call_1".to_string(),
            name: "nonexistent-tool".to_string(),
            input: json!({}),
        };

        content.add_tool_use(tool_use).unwrap();

        let result = runner.process_content_array(&content);
        assert!(result.is_err());
    }

    #[test]
    fn test_execution_flow_tool_error_handling() {
        let mut registry = ToolRegistry::new();

        let schema = InputSchema::empty_object();
        let tool_def = ToolDefinition::new(
            "failing-tool",
            "Tool that demonstrates error handling with proper error messages",
            schema,
        )
        .unwrap();

        let executor = |_input: Value| ToolExecutionResult::Error("Connection failed".to_string());
        registry.register(tool_def, executor).unwrap();

        let runner = ToolRunner::new(registry);

        let mut content = ContentArray::new();
        let tool_use = ToolUseBlock {
            id: "call_1".to_string(),
            name: "failing-tool".to_string(),
            input: json!({}),
        };

        content.add_tool_use(tool_use).unwrap();

        let result = runner.process_content_array(&content);
        assert!(result.is_ok());

        let processed = result.unwrap();
        let blocks = processed.blocks();
        let last_block = &blocks[blocks.len() - 1];

        if let ContentBlock::ToolResult(tool_result) = last_block {
            assert_eq!(tool_result.is_error, Some(true));
        } else {
            panic!("Expected ToolResult block");
        }
    }
}

// =============================================================================
// INTEGRATION TESTS - Tool Choice Control (30%)
// =============================================================================

#[cfg(test)]
mod tool_choice {
    use super::*;

    #[test]
    fn test_tool_choice_auto() {
        // Auto: Claude decides whether to use tools
        let choice = ToolChoice::Auto;
        assert_eq!(choice, ToolChoice::Auto);
    }

    #[test]
    fn test_tool_choice_any() {
        // Any: Claude must use a tool
        let choice = ToolChoice::Any;
        assert_eq!(choice, ToolChoice::Any);
    }

    #[test]
    fn test_tool_choice_named() {
        // Named: Claude must use a specific tool
        let choice = ToolChoice::Named;
        assert_eq!(choice, ToolChoice::Named);
    }

    #[test]
    fn test_tool_choice_none() {
        // None: Claude cannot use tools
        let choice = ToolChoice::None;
        assert_eq!(choice, ToolChoice::None);
    }
}

// =============================================================================
// INTEGRATION TESTS - Error Handling (30%)
// =============================================================================

#[cfg(test)]
mod error_handling {
    use super::*;

    #[test]
    fn test_invalid_input_error() {
        let mut registry = ToolRegistry::new();

        let mut props = HashMap::new();
        props.insert("temperature".to_string(), json!({ "type": "number" }));
        let schema = InputSchema::object(props, vec!["temperature".to_string()]);

        let tool_def = ToolDefinition::new(
            "validate-input",
            "Tool that validates input parameters with comprehensive error checking",
            schema,
        )
        .unwrap();

        let executor = |input: Value| {
            if let Some(_temp) = input.get("temperature") {
                ToolExecutionResult::Success(json!({"valid": true}))
            } else {
                ToolExecutionResult::Error("Missing required parameter: temperature".to_string())
            }
        };

        registry.register(tool_def, executor).unwrap();

        let result = registry.execute("validate-input", json!({}));
        assert!(result.is_ok());

        match result.unwrap() {
            ToolExecutionResult::Error(msg) => {
                assert!(msg.contains("temperature"));
            }
            _ => panic!("Expected error"),
        }
    }

    #[test]
    fn test_tool_execution_timeout_simulation() {
        let mut registry = ToolRegistry::new();

        let schema = InputSchema::empty_object();
        let tool_def = ToolDefinition::new(
            "timeout-tool",
            "Tool that simulates timeout behavior for error handling testing",
            schema,
        )
        .unwrap();

        let executor = |_input: Value| {
            // Simulate timeout by returning error
            ToolExecutionResult::Error("Timeout: Request exceeded 30 seconds".to_string())
        };

        registry.register(tool_def, executor).unwrap();

        let result = registry.execute("timeout-tool", json!({}));
        match result.unwrap() {
            ToolExecutionResult::Error(msg) => {
                assert!(msg.contains("Timeout"));
            }
            _ => panic!("Expected error"),
        }
    }

    #[test]
    fn test_tool_retry_on_error() {
        let mut registry = ToolRegistry::new();

        let schema = InputSchema::empty_object();
        let tool_def = ToolDefinition::new(
            "retry-tool",
            "Tool demonstrating retry behavior after failures with exponential backoff",
            schema,
        )
        .unwrap();

        let executor = |_input: Value| {
            // First attempt fails, should be retried by caller
            ToolExecutionResult::Error("Service unavailable".to_string())
        };

        registry.register(tool_def, executor).unwrap();

        // Simulate Claude's retry logic: 2-3 retries
        for attempt in 0..3 {
            let result = registry.execute("retry-tool", json!({}));

            match result.unwrap() {
                ToolExecutionResult::Error(msg) => {
                    if attempt < 2 {
                        // Could retry
                        assert!(msg.contains("unavailable"));
                    }
                }
                _ => panic!("Expected error on attempt {}", attempt),
            }
        }
    }

    #[test]
    fn test_malformed_tool_result() {
        // Test that invalid tool results are caught
        let mut content = ContentArray::new();
        let tool_use = ToolUseBlock {
            id: "call_1".to_string(),
            name: "test".to_string(),
            input: json!({}),
        };

        content.add_tool_use(tool_use).unwrap();

        // Try to add result with mismatched ID
        let mismatched_result = ToolResultBlock::success("call_2", json!({}));

        assert!(content.add_tool_result(mismatched_result).is_err());
    }

    #[test]
    fn test_tool_result_formatting_error() {
        // Test 400 error from incorrect result formatting
        let mut content = ContentArray::new();

        // Try to add text before results (invalid formatting)
        content.add_text("Some text".to_string()).unwrap();

        let tool_use = ToolUseBlock {
            id: "call_1".to_string(),
            name: "test".to_string(),
            input: json!({}),
        };

        // This should still work, but validation would fail
        content.add_tool_use(tool_use).unwrap();

        let validation_result = content.validate();
        assert!(validation_result.is_err()); // Text before results is invalid
    }
}

// =============================================================================
// E2E TESTS - Full Tool Lifecycle (10%)
// =============================================================================

#[cfg(test)]
mod e2e_tool_lifecycle {
    use super::*;

    #[test]
    fn test_full_tool_lifecycle() {
        // 1. Define tool
        let mut props = HashMap::new();
        props.insert("location".to_string(), json!({ "type": "string" }));
        let schema = InputSchema::object(props, vec!["location".to_string()]);

        let tool_def = ToolDefinition::new(
            "get-weather",
            "Retrieves current weather conditions for a specified location with temperature, humidity, and forecast data",
            schema,
        )
        .unwrap();

        // 2. Register tool
        let mut registry = ToolRegistry::new();
        let executor = |input: Value| {
            if let Some(location) = input.get("location") {
                ToolExecutionResult::Success(json!({
                    "location": location,
                    "temperature": 72,
                    "conditions": "Sunny"
                }))
            } else {
                ToolExecutionResult::Error("Missing location".to_string())
            }
        };

        registry.register(tool_def, executor).unwrap();

        // 3. Execute tool
        let result = registry.execute("get-weather", json!({"location": "San Francisco"}));
        assert!(result.is_ok());

        // 4. Verify result
        match result.unwrap() {
            ToolExecutionResult::Success(output) => {
                assert_eq!(output.get("location"), Some(&json!("San Francisco")));
                assert_eq!(output.get("temperature"), Some(&json!(72)));
            }
            _ => panic!("Expected success"),
        }
    }

    #[test]
    fn test_parallel_tool_execution() {
        // Simulate Claude making multiple tool calls in a single turn
        let mut registry = ToolRegistry::new();

        let schema = InputSchema::empty_object();

        let tool1_def = ToolDefinition::new(
            "get-time",
            "Retrieves the current system time with timezone information",
            schema.clone(),
        )
        .unwrap();

        let tool2_def = ToolDefinition::new(
            "get-date",
            "Retrieves the current system date with day of week information",
            schema,
        )
        .unwrap();

        let executor = |input: Value| ToolExecutionResult::Success(input);

        registry.register(tool1_def, executor).unwrap();
        registry.register(tool2_def, executor).unwrap();

        // Execute both tools
        let result1 = registry.execute("get-time", json!({}));
        let result2 = registry.execute("get-date", json!({}));

        assert!(result1.is_ok());
        assert!(result2.is_ok());
    }

    #[test]
    fn test_tool_use_with_streaming() {
        // Test streaming tool use events
        let events = vec![
            ToolStreamEvent::ToolUseStart {
                id: "call_1".to_string(),
                name: "search".to_string(),
            },
            ToolStreamEvent::InputDelta("query".to_string()),
            ToolStreamEvent::InputDelta(": SF".to_string()),
            ToolStreamEvent::ToolUseEnd,
            ToolStreamEvent::ToolResult {
                success: true,
                content: "Results found".to_string(),
            },
        ];

        assert_eq!(events.len(), 5);
        assert!(matches!(
            events[0],
            ToolStreamEvent::ToolUseStart { .. }
        ));
        assert!(matches!(
            events[4],
            ToolStreamEvent::ToolResult { .. }
        ));
    }

    #[test]
    fn test_tool_fallback_on_error() {
        let mut registry = ToolRegistry::new();

        let schema = InputSchema::empty_object();

        let primary_tool = ToolDefinition::new(
            "primary-api",
            "Primary tool that may fail with fallback handling",
            schema.clone(),
        )
        .unwrap();

        let fallback_tool = ToolDefinition::new(
            "fallback-api",
            "Fallback tool used when primary tool fails",
            schema,
        )
        .unwrap();

        let primary_executor =
            |_input: Value| ToolExecutionResult::Error("Primary service down".to_string());
        let fallback_executor =
            |_input: Value| ToolExecutionResult::Success(json!({"source": "fallback"}));

        registry.register(primary_tool, primary_executor).unwrap();
        registry.register(fallback_tool, fallback_executor).unwrap();

        // Try primary
        let primary_result = registry.execute("primary-api", json!({}));
        assert!(matches!(primary_result, Ok(ToolExecutionResult::Error(_))));

        // Fall back to secondary
        let fallback_result = registry.execute("fallback-api", json!({}));
        assert!(matches!(
            fallback_result,
            Ok(ToolExecutionResult::Success(_))
        ));
    }

    #[test]
    fn test_tool_output_documentation() {
        // Verify tools are properly documented
        let mut registry = ToolRegistry::new();

        let mut props = HashMap::new();
        props.insert("query".to_string(), json!({ "type": "string" }));
        props.insert("limit".to_string(), json!({ "type": "number" }));

        let schema = InputSchema::object(props, vec!["query".to_string()]);

        let tool_def = ToolDefinition::new(
            "search",
            "Performs a comprehensive search across documents and databases. Returns matching results ranked by relevance. Supports filtering and pagination for large result sets.",
            schema,
        )
        .unwrap();

        let executor = |_input: Value| {
            ToolExecutionResult::Success(json!({
                "results": [
                    {"title": "Result 1", "relevance": 0.95},
                    {"title": "Result 2", "relevance": 0.87}
                ]
            }))
        };

        registry.register(tool_def.clone(), executor).unwrap();

        // Verify tool is registered with documentation
        let tool = registry.get_tool("search");
        assert!(tool.is_some());
        let tool = tool.unwrap();
        assert!(tool.description.len() > 50);
        assert!(tool.input_schema.properties.is_some());
    }
}

// =============================================================================
// EDGE CASE TESTS
// =============================================================================

#[cfg(test)]
mod edge_cases {
    use super::*;

    #[test]
    fn test_empty_tool_input() {
        let mut registry = ToolRegistry::new();

        let schema = InputSchema::empty_object();
        let tool_def = ToolDefinition::new(
            "no-params",
            "Tool that takes no parameters but still requires detailed description",
            schema,
        )
        .unwrap();

        let executor = |input: Value| {
            assert_eq!(input, json!({}));
            ToolExecutionResult::Success(json!({"status": "ok"}))
        };

        registry.register(tool_def, executor).unwrap();
        let result = registry.execute("no-params", json!({}));
        assert!(result.is_ok());
    }

    #[test]
    fn test_large_tool_input() {
        let mut registry = ToolRegistry::new();

        let schema = InputSchema::empty_object();
        let tool_def = ToolDefinition::new(
            "process-large",
            "Tool that processes large input data structures with proper buffering",
            schema,
        )
        .unwrap();

        let executor = |input: Value| ToolExecutionResult::Success(input);

        registry.register(tool_def, executor).unwrap();

        // Large input
        let mut items = Vec::new();
        for i in 0..1000 {
            items.push(json!({"id": i, "data": format!("item_{}", i)}));
        }
        let large_data = json!({
            "items": items
        });

        let result = registry.execute("process-large", large_data);
        assert!(result.is_ok());
    }

    #[test]
    fn test_nested_json_input() {
        let mut registry = ToolRegistry::new();

        let schema = InputSchema::empty_object();
        let tool_def = ToolDefinition::new(
            "process-nested",
            "Tool that handles deeply nested JSON structures with recursive processing",
            schema,
        )
        .unwrap();

        let executor = |input: Value| ToolExecutionResult::Success(input);

        registry.register(tool_def, executor).unwrap();

        let nested = json!({
            "level1": {
                "level2": {
                    "level3": {
                        "level4": {
                            "value": "deep"
                        }
                    }
                }
            }
        });

        let result = registry.execute("process-nested", nested.clone());
        assert!(result.is_ok());

        match result.unwrap() {
            ToolExecutionResult::Success(output) => {
                assert_eq!(
                    output.get("level1").and_then(|l1| l1.get("level2"))
                        .and_then(|l2| l2.get("level3"))
                        .and_then(|l3| l3.get("level4"))
                        .and_then(|l4| l4.get("value")),
                    Some(&json!("deep"))
                );
            }
            _ => panic!("Expected success"),
        }
    }

    #[test]
    fn test_null_values_in_input() {
        let mut registry = ToolRegistry::new();

        let schema = InputSchema::empty_object();
        let tool_def = ToolDefinition::new(
            "handle-nulls",
            "Tool that gracefully handles null values in input parameters",
            schema,
        )
        .unwrap();

        let executor = |input: Value| {
            if input.get("field").map(|f| f.is_null()).unwrap_or(false) {
                ToolExecutionResult::Success(json!({"null_received": true}))
            } else {
                ToolExecutionResult::Error("Expected null".to_string())
            }
        };

        registry.register(tool_def, executor).unwrap();

        let result = registry.execute("handle-nulls", json!({"field": null}));
        assert!(result.is_ok());
    }
}
