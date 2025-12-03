//! E2E Test Mocks
//!
//! This module provides mock implementations for E2E testing.
//!
//! Philosophy:
//! - Provide deterministic, controllable behavior for testing
//! - Match real API interfaces as closely as possible
//! - Enable comprehensive test coverage without external dependencies

pub mod mock_llm;

// Re-export for convenience
#[allow(unused_imports)]
pub use mock_llm::RecordedRequest;
pub use mock_llm::{MockLLM, MockResponse};
