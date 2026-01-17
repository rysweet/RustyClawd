# Research Findings

This file captures interesting discoveries and patterns found in Claude Code's implementation.

## Template for New Findings

```markdown
### [Feature Name]

**Date**: YYYY-MM-DD
**Researcher**: [Your name]
**Lines**: [Line numbers in deminified code]

**Discovery**:
Brief description of what was found

**JavaScript Implementation**:
\```javascript
// Relevant code snippet
\```

**Rust Translation**:
\```rust
// How this maps to Rust
\```

**Notes**:
- Additional observations
- Gotchas or edge cases
- Related patterns
```

## Example Findings

### ContentBlock Event Ordering Validation

**Date**: 2026-01-17
**Lines**: 186611

**Discovery**:
Claude Code validates streaming event order, throwing errors if events arrive out of sequence. Specifically, it checks that `message_start` occurs before any `content_block_*` events.

**JavaScript Implementation**:
```javascript
// Line 186611
if (!B) throw new cB(`Unexpected event order, got ${Q.type} before "message_start"`);
```

**Rust Translation**:
```rust
pub struct StreamingEventParser {
    message_started: bool,
}

impl StreamingEventParser {
    pub fn parse(&mut self, event: StreamingEvent) -> Result<()> {
        match event {
            StreamingEvent::MessageStart { .. } => {
                if self.message_started {
                    return Err(Error::UnexpectedEventOrder);
                }
                self.message_started = true;
                Ok(())
            }
            StreamingEvent::ContentBlockStart { .. } => {
                if !self.message_started {
                    return Err(Error::msg(
                        format!("Unexpected event order, got content_block_start before message_start")
                    ));
                }
                Ok(())
            }
            // ... other events
        }
    }
}
```

**Notes**:
- Strict validation prevents malformed streams
- State machine tracks message lifecycle
- Error messages should be descriptive for debugging

---

### Hook Registry Implementation

**Date**: 2026-01-17
**Lines**: 2218-2226, 62410-62414

**Discovery**:
Hooks are stored in a Map (object) keyed by hook type, with arrays of callback functions. Registration lazily creates arrays, and execution iterates all callbacks.

**JavaScript Implementation**:
```javascript
// Registration (lines 2218-2222)
if (!C0.registeredHooks) C0.registeredHooks = {};
if (!C0.registeredHooks[G]) C0.registeredHooks[G] = [];
C0.registeredHooks[G].push(...B);

// Execution (lines 62410-62414)
if (!this._hooks[A]) this._hooks[A] = [];
this._hooks[A].push(Q);
if (this._hooks[A]) this._hooks[A].forEach((B) => B(...Q));
```

**Rust Translation**:
```rust
use std::collections::HashMap;

pub type HookCallback = Box<dyn Fn(&HookContext) -> Result<()>>;

pub struct HookRegistry {
    hooks: HashMap<String, Vec<HookCallback>>,
}

impl HookRegistry {
    pub fn register(&mut self, hook_type: String, callback: HookCallback) {
        self.hooks
            .entry(hook_type)
            .or_insert_with(Vec::new)
            .push(callback);
    }

    pub fn execute(&self, hook_type: &str, context: &HookContext) -> Result<()> {
        if let Some(callbacks) = self.hooks.get(hook_type) {
            for callback in callbacks {
                callback(context)?;
            }
        }
        Ok(())
    }
}
```

**Notes**:
- Lazy initialization common in JavaScript, explicit in Rust
- Error handling differs: JS forEach continues on error, Rust `?` stops
- Consider whether to continue or stop on hook failure

---

### Thinking Block Feature Flag

**Date**: 2026-01-17
**Lines**: 1723, 92007

**Discovery**:
Thinking blocks controlled by feature flag "interleaved-thinking-2025-05-14" and user preference "preserve_thinking". Both must be enabled to show thinking blocks.

**JavaScript Implementation**:
```javascript
// Line 1723
Gf0 = "interleaved-thinking-2025-05-14",

// Line 92007
let Y = Z && wY("preserve_thinking", "enabled", !1);
```

**Rust Translation**:
```rust
const INTERLEAVED_THINKING_FLAG: &str = "interleaved-thinking-2025-05-14";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureFlags {
    pub interleaved_thinking: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPreferences {
    pub preserve_thinking: bool,
}

impl MessageRenderer {
    pub fn should_show_thinking(&self) -> bool {
        self.feature_flags.interleaved_thinking &&
        self.preferences.preserve_thinking
    }
}
```

**Notes**:
- Feature flags enable gradual rollout
- User preferences override feature flags
- Both checks required for full compatibility

---

## Add Your Findings Below

[Your discoveries here]
