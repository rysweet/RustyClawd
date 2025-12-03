//! End-to-End Test Infrastructure
//!
//! This module contains E2E test infrastructure, including helpers and mocks.
//!
//! **Test Organization:**
//! - `test_slash_command_tui_integration.rs` - SlashCommand + TUI integration
//! - `test_skills_execution_context.rs` - Skills with conversation context
//! - `test_full_interactive_session.rs` - Complete session lifecycle
//!
//! **Helper Modules:**
//! - `helpers::TestSession` - Session orchestration (STUB)
//! - `helpers::TestSkillEnvironment` - Test skill setup (STUB)
//! - `mocks::MockLLM` - Controllable LLM client (STUB)
//!
//! **Status:** Tests are failing (expected) - waiting for implementation
//!
//! See: docs/architecture/e2e_testing_architecture.md

pub mod helpers;
pub mod mocks;

// Test files are separate - they import from this module
