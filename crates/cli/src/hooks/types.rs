//! Hook type definitions — re-export hub
//!
//! This module re-exports all hook types from their focused submodules:
//! - `event` — HookEvent enum and impls
//! - `context` — HookContext struct and factory methods
//! - `config` — HookConfig, HookMatcher, HookType, Hook, and output types
//!
//! All types are re-exported for backward compatibility and public API stability,
//! even if some are not currently imported via this hub within the crate.

// Re-export hub: allow unused imports since not all types may be consumed internally yet.
#[allow(unused_imports)]
pub use crate::hooks::config::{
    Hook, HookConfig, HookMatcher, HookOutput, HookResult, HookSpecificOutput, HookType,
    HooksConfiguration, PermissionDecision, StopDecision,
};
#[allow(unused_imports)]
pub use crate::hooks::context::{
    HookContext, NotificationType, SessionEndReason, SessionStartMatcher,
};
#[allow(unused_imports)]
pub use crate::hooks::event::HookEvent;
