//! Mock Tool Executor for testing
//!
//! Provides deterministic tool execution without real execution:
//! - Configurable tool results
//! - Execution history tracking
//! - Error injection
//! - Tool call verification

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Mock tool execution result
#[derive(Debug, Clone, PartialEq)]
pub struct MockToolResult {
    /// Tool output
    pub output: String,
    /// Exit code (for bash-like tools)
    pub exit_code: i32,
    /// Execution time (ms)
    pub execution_time_ms: u64,
}

impl MockToolResult {
    /// Create success result
    pub fn success(output: &str) -> Self {
        Self {
            output: output.to_string(),
            exit_code: 0,
            execution_time_ms: 10,
        }
    }

    /// Create error result
    pub fn error(output: &str, exit_code: i32) -> Self {
        Self {
            output: output.to_string(),
            exit_code,
            execution_time_ms: 10,
        }
    }

    /// Set execution time
    pub fn with_execution_time(mut self, ms: u64) -> Self {
        self.execution_time_ms = ms;
        self
    }
}

/// Tool execution record
#[derive(Debug, Clone)]
pub struct ToolExecution {
    /// Tool name
    pub tool_name: String,
    /// Tool parameters (JSON string)
    pub parameters: String,
    /// Timestamp
    #[allow(dead_code)]
    pub timestamp: std::time::Instant,
}

/// Mock tool executor
pub struct MockToolExecutor {
    /// Configured results per tool
    results: Arc<Mutex<HashMap<String, Vec<MockToolResult>>>>,
    /// Execution history
    history: Arc<Mutex<Vec<ToolExecution>>>,
    /// Default result
    default_result: Arc<Mutex<MockToolResult>>,
}

impl MockToolExecutor {
    /// Create new mock executor
    pub fn new() -> Self {
        Self {
            results: Arc::new(Mutex::new(HashMap::new())),
            history: Arc::new(Mutex::new(Vec::new())),
            default_result: Arc::new(Mutex::new(MockToolResult::success("Default mock output"))),
        }
    }

    /// Set result for a specific tool
    pub fn set_tool_result(&self, tool_name: &str, result: MockToolResult) {
        self.results
            .lock()
            .unwrap()
            .entry(tool_name.to_string())
            .or_default()
            .push(result);
    }

    /// Set multiple results for a tool (will be consumed in order)
    pub fn set_tool_results(&self, tool_name: &str, results: Vec<MockToolResult>) {
        self.results
            .lock()
            .unwrap()
            .insert(tool_name.to_string(), results);
    }

    /// Set default result for unknown tools
    pub fn set_default_result(&self, result: MockToolResult) {
        *self.default_result.lock().unwrap() = result;
    }

    /// Execute a tool
    pub fn execute(&self, tool_name: &str, parameters: &str) -> MockToolResult {
        // Record execution
        self.history.lock().unwrap().push(ToolExecution {
            tool_name: tool_name.to_string(),
            parameters: parameters.to_string(),
            timestamp: std::time::Instant::now(),
        });

        // Get result
        let mut results = self.results.lock().unwrap();
        if let Some(tool_results) = results.get_mut(tool_name) {
            if !tool_results.is_empty() {
                return tool_results.remove(0);
            }
        }

        // Return default
        self.default_result.lock().unwrap().clone()
    }

    /// Get execution history
    pub fn execution_history(&self) -> Vec<ToolExecution> {
        self.history.lock().unwrap().clone()
    }

    /// Get execution count for a specific tool
    pub fn execution_count(&self, tool_name: &str) -> usize {
        self.history
            .lock()
            .unwrap()
            .iter()
            .filter(|exec| exec.tool_name == tool_name)
            .count()
    }

    /// Check if a tool was executed
    pub fn was_executed(&self, tool_name: &str) -> bool {
        self.execution_count(tool_name) > 0
    }

    /// Check if tool was executed with specific parameters
    pub fn was_executed_with(&self, tool_name: &str, params_substring: &str) -> bool {
        self.history
            .lock()
            .unwrap()
            .iter()
            .any(|exec| exec.tool_name == tool_name && exec.parameters.contains(params_substring))
    }

    /// Clear execution history
    pub fn clear_history(&self) {
        self.history.lock().unwrap().clear();
    }

    /// Reset executor (clear history and results)
    pub fn reset(&self) {
        self.history.lock().unwrap().clear();
        self.results.lock().unwrap().clear();
    }
}

impl Default for MockToolExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_tool_executor_basic() {
        let executor = MockToolExecutor::new();

        // Set result for bash tool
        executor.set_tool_result("bash", MockToolResult::success("Hello from bash"));

        // Execute
        let result = executor.execute("bash", r#"{"command":"echo hello"}"#);

        assert_eq!(result.output, "Hello from bash");
        assert_eq!(result.exit_code, 0);
    }

    #[test]
    fn test_mock_tool_executor_error() {
        let executor = MockToolExecutor::new();

        executor.set_tool_result("bash", MockToolResult::error("Command failed", 1));

        let result = executor.execute("bash", r#"{"command":"false"}"#);

        assert_eq!(result.output, "Command failed");
        assert_eq!(result.exit_code, 1);
    }

    #[test]
    fn test_execution_history() {
        let executor = MockToolExecutor::new();

        executor.execute("bash", r#"{"command":"ls"}"#);
        executor.execute("read", r#"{"path":"/tmp/test"}"#);
        executor.execute("bash", r#"{"command":"pwd"}"#);

        let history = executor.execution_history();
        assert_eq!(history.len(), 3);
        assert_eq!(history[0].tool_name, "bash");
        assert_eq!(history[1].tool_name, "read");
        assert_eq!(history[2].tool_name, "bash");
    }

    #[test]
    fn test_execution_count() {
        let executor = MockToolExecutor::new();

        executor.execute("bash", "{}");
        executor.execute("bash", "{}");
        executor.execute("read", "{}");

        assert_eq!(executor.execution_count("bash"), 2);
        assert_eq!(executor.execution_count("read"), 1);
        assert_eq!(executor.execution_count("write"), 0);
    }

    #[test]
    fn test_was_executed() {
        let executor = MockToolExecutor::new();

        executor.execute("bash", "{}");

        assert!(executor.was_executed("bash"));
        assert!(!executor.was_executed("read"));
    }

    #[test]
    fn test_was_executed_with() {
        let executor = MockToolExecutor::new();

        executor.execute("bash", r#"{"command":"echo hello"}"#);

        assert!(executor.was_executed_with("bash", "echo"));
        assert!(executor.was_executed_with("bash", "hello"));
        assert!(!executor.was_executed_with("bash", "goodbye"));
    }

    #[test]
    fn test_multiple_results() {
        let executor = MockToolExecutor::new();

        executor.set_tool_results(
            "bash",
            vec![
                MockToolResult::success("First"),
                MockToolResult::success("Second"),
                MockToolResult::success("Third"),
            ],
        );

        let result1 = executor.execute("bash", "{}");
        let result2 = executor.execute("bash", "{}");
        let result3 = executor.execute("bash", "{}");

        assert_eq!(result1.output, "First");
        assert_eq!(result2.output, "Second");
        assert_eq!(result3.output, "Third");
    }

    #[test]
    fn test_default_result() {
        let executor = MockToolExecutor::new();

        executor.set_default_result(MockToolResult::success("Default response"));

        let result = executor.execute("unknown_tool", "{}");

        assert_eq!(result.output, "Default response");
    }

    #[test]
    fn test_clear_history() {
        let executor = MockToolExecutor::new();

        executor.execute("bash", "{}");
        executor.execute("read", "{}");

        assert_eq!(executor.execution_history().len(), 2);

        executor.clear_history();

        assert_eq!(executor.execution_history().len(), 0);
    }

    #[test]
    fn test_reset() {
        let executor = MockToolExecutor::new();

        executor.set_tool_result("bash", MockToolResult::success("Test"));
        executor.execute("bash", "{}");

        assert_eq!(executor.execution_history().len(), 1);

        executor.reset();

        assert_eq!(executor.execution_history().len(), 0);
    }

    #[test]
    fn test_execution_time() {
        let result = MockToolResult::success("Test").with_execution_time(500);

        assert_eq!(result.execution_time_ms, 500);
    }
}
