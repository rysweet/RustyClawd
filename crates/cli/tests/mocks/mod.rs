//! Mock implementations for TUI testing
//!
//! Provides deterministic mocks for:
//! - API client (streaming responses)
//! - Tool executor (tool calls)
//! - Event generation

pub mod mock_api_client;
pub mod mock_tool_executor;

pub use mock_api_client::{MockApiClient, MockResponse, MockStreamEvent};
