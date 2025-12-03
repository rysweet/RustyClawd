//! E2E Test Helpers
//!
//! This module provides helper utilities for E2E testing.
//!
//! Philosophy:
//! - Provide test-friendly interfaces to complex systems
//! - Enable comprehensive test coverage with minimal boilerplate
//! - Self-contained, regeneratable modules

pub mod test_session;
pub mod test_skill_env;

// Re-exports for convenience
pub use test_session::TestSession;
#[allow(unused_imports)]
pub use test_skill_env::TestSkillEnvironment;
