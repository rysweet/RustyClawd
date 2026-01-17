# Claude Code JavaScript Deminification Guide

## Overview

This guide documents the complete workflow for deminifying and analyzing Claude Code's JavaScript implementation to understand patterns for porting to Rust. Since Claude Code's npm package is minified, we need to deminify it before analysis.

**Target Audience**: Developers working on RustyClawd who want to learn from Claude Code's implementation.

**Key Use Cases**:
- Understanding how Claude Code implements specific features
- Finding patterns for thinking blocks, strict validation, etc.
- Comparing JavaScript implementations to Rust equivalents
- Learning from production-ready code

## Quick Start

```bash
# 1. Install deminification tools
npm install -g prettier js-beautify

# 2. Locate Claude Code installation
CLAUDE_CODE_PATH=$(readlink -f $(which claude))
CLAUDE_CODE_DIR=$(dirname "$CLAUDE_CODE_PATH")

# 3. Create research directory
mkdir -p ~/src/RustyClawd/docs/research
cd ~/src/RustyClawd/docs/research

# 4. Copy and deminify
cp "$CLAUDE_CODE_DIR/cli.js" claude-code-minified.js
prettier --write claude-code-minified.js --print-width 100

# 5. Search for patterns (examples below)
grep -n "ContentBlock" claude-code-minified.js | head -20
```

## Claude Code Package Location

Claude Code's JavaScript implementation is typically installed at:

```
/home/azureuser/.npm-global/lib/node_modules/@anthropic-ai/claude-code/
```

**Key Files**:
- `cli.js` - Main implementation (11MB minified, 489k lines deminified)
- `sdk-tools.d.ts` - TypeScript definitions
- `package.json` - Package metadata

**Finding Your Installation**:

```bash
# Method 1: Follow symlink
which claude
readlink -f $(which claude)

# Method 2: Direct npm location
npm root -g
ls $(npm root -g)/@anthropic-ai/claude-code/

# Method 3: Search
find ~/.npm-global -name "claude-code" -type d 2>/dev/null
```

## Deminification Tools Comparison

### Prettier (Recommended)

**Pros**:
- Better readability (more consistent formatting)
- Widely used in JavaScript community
- Opinionated (consistent results)
- Handles modern JavaScript well

**Cons**:
- Slower (19 seconds for 11MB file)
- Creates more lines (489k lines)

**Usage**:

```bash
# Single file
prettier --write file.js --print-width 100

# With options
prettier --write file.js \
  --print-width 100 \
  --tab-width 2 \
  --single-quote
```

**Output Quality**: ⭐⭐⭐⭐⭐ (Excellent)

### js-beautify

**Pros**:
- Faster (comparable to prettier)
- Fewer lines (465k lines)
- Good for quick exploration

**Cons**:
- Slightly less consistent formatting
- More compact (sometimes harder to read)

**Usage**:

```bash
# Single file (in-place)
js-beautify -r file.js

# With options
js-beautify file.js -o formatted.js \
  --indent-size 2 \
  --max-preserve-newlines 2
```

**Output Quality**: ⭐⭐⭐⭐ (Good)

### Recommendation

**Use Prettier** for:
- Deep analysis where readability matters
- Learning from implementation patterns
- Creating documentation

**Use js-beautify** for:
- Quick exploration
- Fast pattern searching
- When line count matters

## Complete Deminification Workflow

### Step 1: Setup

```bash
#!/bin/bash
# File: scripts/setup-deminification.sh

set -e

# Install tools
echo "Installing deminification tools..."
npm install -g prettier js-beautify

# Find Claude Code installation
CLAUDE_CODE_PATH=$(readlink -f $(which claude))
if [ -z "$CLAUDE_CODE_PATH" ]; then
  echo "Error: Claude Code not found in PATH"
  exit 1
fi

CLAUDE_CODE_DIR=$(dirname "$CLAUDE_CODE_PATH")
echo "Found Claude Code at: $CLAUDE_CODE_DIR"

# Create research directory
RESEARCH_DIR="$HOME/src/RustyClawd/docs/research"
mkdir -p "$RESEARCH_DIR"
echo "Research directory: $RESEARCH_DIR"

# Export for other scripts
export CLAUDE_CODE_DIR
export RESEARCH_DIR
```

### Step 2: Extract and Deminify

```bash
#!/bin/bash
# File: scripts/deminify-claude-code.sh

set -e

# Source setup
source ./setup-deminification.sh

cd "$RESEARCH_DIR"

# Copy original
echo "Copying minified file..."
cp "$CLAUDE_CODE_DIR/cli.js" claude-code-minified.js

# Deminify with prettier
echo "Deminifying with prettier (may take 20-30 seconds)..."
prettier --write claude-code-minified.js --print-width 100

# Also create js-beautify version for comparison
echo "Creating js-beautify version..."
cp "$CLAUDE_CODE_DIR/cli.js" claude-code-jsbeautify.js
js-beautify -r claude-code-jsbeautify.js

# Report
echo ""
echo "Deminification complete!"
echo "Prettier version: $(wc -l < claude-code-minified.js) lines"
echo "js-beautify version: $(wc -l < claude-code-jsbeautify.js) lines"
echo ""
echo "Files available at: $RESEARCH_DIR"
```

### Step 3: Create Search Index

```bash
#!/bin/bash
# File: scripts/index-patterns.sh

set -e

RESEARCH_DIR="$HOME/src/RustyClawd/docs/research"
cd "$RESEARCH_DIR"

# Create indices for common patterns
echo "Creating search indices..."

# ContentBlock patterns
grep -n "ContentBlock\|content_block" claude-code-minified.js > index-contentblock.txt
echo "ContentBlock patterns: $(wc -l < index-contentblock.txt) matches"

# Streaming patterns
grep -n "streaming\|StreamingEvent\|message_start\|content_block_start" \
  claude-code-minified.js > index-streaming.txt
echo "Streaming patterns: $(wc -l < index-streaming.txt) matches"

# Hook patterns
grep -n "hook\|Hook\|lifecycle" claude-code-minified.js > index-hooks.txt
echo "Hook patterns: $(wc -l < index-hooks.txt) matches"

# Tool execution
grep -n "tool_use\|ToolUse\|execute_tool" claude-code-minified.js > index-tools.txt
echo "Tool patterns: $(wc -l < index-tools.txt) matches"

# Session management
grep -n "session\|Session\|sessionId" claude-code-minified.js > index-session.txt
echo "Session patterns: $(wc -l < index-session.txt) matches"

# Thinking blocks
grep -n "thinking\|ThinkingBlock\|interleaved-thinking" \
  claude-code-minified.js > index-thinking.txt
echo "Thinking patterns: $(wc -l < index-thinking.txt) matches"

echo ""
echo "Indices created in: $RESEARCH_DIR/index-*.txt"
```

## Key Pattern Search Reference

### ContentBlock Types and Handling

**What to Search For**:
- ContentBlock type definitions
- ContentBlock event handling
- ContentBlock serialization

**Search Patterns**:

```bash
# Type definitions
grep -n "ContentBlock\|ContentBlockDelta\|ContentBlockStart" claude-code-minified.js

# Event handling
grep -n "content_block_start\|content_block_delta\|content_block_stop" claude-code-minified.js

# Specific block types
grep -n "TextContentBlock\|ToolUseBlock\|ReasoningContentBlock\|ThinkingBlock" \
  claude-code-minified.js
```

**Key Findings from Research**:

```javascript
// Line 126300-126306: ContentBlock event types
sX3 = "ContentBlocks",
tX3 = "ContentBlockDelta",
eX3 = "ContentBlockDeltaEvent",
AI3 = "ContentBlockStart",
QI3 = "ContentBlockStartEvent",
BI3 = "ContentBlockStopEvent",
GI3 = "ContentBlock",

// Line 126450-126451: Reasoning/Thinking blocks
hD3 = "ReasoningContentBlock",
gD3 = "ReasoningContentBlockDelta",

// Line 126485-126487: Tool use blocks
UV3 = "ToolUseBlock",
qV3 = "ToolUseBlockDelta",
NV3 = "ToolUseBlockStart",
```

**Rust Translation Pattern**:

```rust
// JavaScript enums become Rust enums
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ContentBlock {
    #[serde(rename = "text")]
    Text { text: String },

    #[serde(rename = "tool_use")]
    ToolUse { id: String, name: String, input: Value },

    #[serde(rename = "reasoning")]
    Reasoning { thinking: String },
}
```

### Streaming Event Parsing

**What to Search For**:
- Event type discrimination
- Stream parsing logic
- Event ordering validation

**Search Patterns**:

```bash
# Event types
grep -n "message_start\|content_block_start\|content_block_delta\|content_block_stop" \
  claude-code-minified.js

# Stream handling
grep -n "streaming.*true\|createStream\|parseStream" claude-code-minified.js

# Error handling
grep -n "Unexpected event order\|streaming error" claude-code-minified.js
```

**Key Findings**:

```javascript
// Line 185430-185433: Event type checking
X.event === "message_start" ||
X.event === "content_block_start" ||

// Line 186589-186606: Event handling switch
switch (event.type) {
  case "message_start": {
    // ...
  }
  case "content_block_start":
    // ...
}

// Line 186611: Order validation
if (!B) throw new cB(`Unexpected event order, got ${Q.type} before "message_start"`);
```

**Rust Translation Pattern**:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum StreamingEvent {
    #[serde(rename = "message_start")]
    MessageStart { message: MessageStart },

    #[serde(rename = "content_block_start")]
    ContentBlockStart { index: usize, content_block: ContentBlock },

    #[serde(rename = "content_block_delta")]
    ContentBlockDelta { index: usize, delta: ContentBlockDelta },

    #[serde(rename = "content_block_stop")]
    ContentBlockStop { index: usize },
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
                    return Err(Error::UnexpectedEventOrder);
                }
                Ok(())
            }
            // ...
        }
    }
}
```

### Hook Lifecycle Implementation

**What to Search For**:
- Hook registration system
- Hook execution timing
- Hook types and handlers

**Search Patterns**:

```bash
# Hook registration
grep -n "registeredHooks\|registerHook\|_hooks" claude-code-minified.js

# Hook types
grep -n "PreToolUse\|PostToolUse\|PostToolUseFailure" claude-code-minified.js

# Hook execution
grep -n "executeHook\|runHook\|triggerHook" claude-code-minified.js
```

**Key Findings**:

```javascript
// Line 1839: Hook storage
registeredHooks: null,

// Line 2218-2226: Hook registration
if (!C0.registeredHooks) C0.registeredHooks = {};
if (!C0.registeredHooks[G]) C0.registeredHooks[G] = [];
C0.registeredHooks[G].push(...B);

// Line 82243-82245: Hook types
"PreToolUse",
"PostToolUse",
"PostToolUseFailure",

// Line 62410-62414: Hook execution
if (!this._hooks[A]) this._hooks[A] = [];
this._hooks[A].push(Q);
if (this._hooks[A]) this._hooks[A].forEach((B) => B(...Q));
```

**Rust Translation Pattern**:

```rust
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum HookType {
    PreToolUse,
    PostToolUse,
    PostToolUseFailure,
    // ... other hook types
}

pub type HookCallback = Box<dyn Fn(&HookContext) -> Result<()>>;

pub struct HookRegistry {
    hooks: HashMap<HookType, Vec<HookCallback>>,
}

impl HookRegistry {
    pub fn new() -> Self {
        Self {
            hooks: HashMap::new(),
        }
    }

    pub fn register(&mut self, hook_type: HookType, callback: HookCallback) {
        self.hooks
            .entry(hook_type)
            .or_insert_with(Vec::new)
            .push(callback);
    }

    pub fn execute(&self, hook_type: &HookType, context: &HookContext) -> Result<()> {
        if let Some(callbacks) = self.hooks.get(hook_type) {
            for callback in callbacks {
                callback(context)?;
            }
        }
        Ok(())
    }
}
```

### Tool Execution Flow

**What to Search For**:
- Tool use block parsing
- Tool result formatting
- Tool error handling

**Search Patterns**:

```bash
# Tool use types
grep -n "tool_use.*id\|ToolUseBlock" claude-code-minified.js

# Tool results
grep -n "tool_result\|tool_use_id" claude-code-minified.js

# Tool execution
grep -n "server_tool_use\|mcp_tool_use" claude-code-minified.js
```

**Key Findings**:

```javascript
// Line 186269: Tool use type check
return A.type === "tool_use" || A.type === "server_tool_use" || A.type === "mcp_tool_use";

// Line 186744-186766: Tool result creation
let B = Q.content.filter((Z) => Z.type === "tool_use");
// ... processing ...
return { type: "tool_result", tool_use_id: Z.id, content: X };

// Line 241688: Finding tool use by ID
if (D.type === "tool_use" && "id" in D && D.id === A) {
```

**Rust Translation Pattern**:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ContentBlock {
    #[serde(rename = "tool_use")]
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
    },
    #[serde(rename = "tool_result")]
    ToolResult {
        tool_use_id: String,
        content: Vec<ToolResultContent>,
    },
    // ... other variants
}

impl Message {
    pub fn extract_tool_uses(&self) -> Vec<&ContentBlock> {
        self.content
            .iter()
            .filter(|block| matches!(block, ContentBlock::ToolUse { .. }))
            .collect()
    }

    pub fn create_tool_result(tool_use_id: String, content: Vec<ToolResultContent>) -> ContentBlock {
        ContentBlock::ToolResult {
            tool_use_id,
            content,
        }
    }
}
```

### Session Management

**What to Search For**:
- Session ID generation
- Session state tracking
- Parent session relationships

**Search Patterns**:

```bash
# Session IDs
grep -n "sessionId\|parentSessionId\|sessionCounter" claude-code-minified.js

# Session state
grep -n "sessionTrustAccepted\|sessionBypassPermissionsMode" claude-code-minified.js

# Session lifecycle
grep -n "sessionIngressToken\|teleportedSessionInfo" claude-code-minified.js
```

**Key Findings**:

```javascript
// Line 1818-1819: Session ID initialization
sessionId: Xf0(),  // UUID generation
parentSessionId: void 0,

// Line 1849-1853: Session ID management
return C0.sessionId;
if (A.setCurrentAsParent) C0.parentSessionId = C0.sessionId;
return ((C0.sessionId = Xf0()), C0.sessionId);

// Line 2018-2049: Session metrics
C0.sessionCounter = Q("claude_code.session.count", {
  description: "Count of CLI sessions started",
```

**Rust Translation Pattern**:

```rust
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct SessionManager {
    session_id: Uuid,
    parent_session_id: Option<Uuid>,
    trust_accepted: bool,
    bypass_permissions: bool,
    ingress_token: Option<String>,
}

impl SessionManager {
    pub fn new() -> Self {
        Self {
            session_id: Uuid::new_v4(),
            parent_session_id: None,
            trust_accepted: false,
            bypass_permissions: false,
            ingress_token: None,
        }
    }

    pub fn new_session(&mut self, set_current_as_parent: bool) -> Uuid {
        if set_current_as_parent {
            self.parent_session_id = Some(self.session_id);
        }
        self.session_id = Uuid::new_v4();
        self.session_id
    }

    pub fn session_id(&self) -> Uuid {
        self.session_id
    }

    pub fn parent_session_id(&self) -> Option<Uuid> {
        self.parent_session_id
    }
}
```

### Thinking Blocks (Interleaved Thinking)

**What to Search For**:
- Thinking block feature flags
- Thinking block rendering
- Thinking preservation options

**Search Patterns**:

```bash
# Feature flags
grep -n "interleaved-thinking\|thinking.*toggle\|preserve_thinking" claude-code-minified.js

# Rendering
grep -n "ThinkingBlock\|renderThinking" claude-code-minified.js
```

**Key Findings**:

```javascript
// Line 1723-1742: Thinking feature flag
Gf0 = "interleaved-thinking-2025-05-14",
// ... in feature flags array ...
"interleaved-thinking-2025-05-14",
"interleaved-thinking-2025-05-14",

// Line 86767: Keyboard shortcut
"meta+t": "chat:thinkingToggle",

// Line 92007: Preserve thinking preference
let Y = Z && wY("preserve_thinking", "enabled", !1);
```

**Rust Translation Pattern**:

```rust
// Feature flag for thinking blocks
const INTERLEAVED_THINKING_FLAG: &str = "interleaved-thinking-2025-05-14";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPreferences {
    preserve_thinking: bool,
    thinking_toggle_enabled: bool,
}

impl Default for UserPreferences {
    fn default() -> Self {
        Self {
            preserve_thinking: false,
            thinking_toggle_enabled: true,
        }
    }
}

// In rendering logic
if preferences.preserve_thinking && has_feature_flag(INTERLEAVED_THINKING_FLAG) {
    // Include thinking blocks in output
} else {
    // Filter out thinking blocks
}
```

## JavaScript to Rust Translation Guide

### Type System Mapping

| JavaScript | Rust | Notes |
|------------|------|-------|
| `string` | `String` | Use `&str` for borrowed strings |
| `number` | `i32`, `f64`, etc. | Choose based on range |
| `boolean` | `bool` | - |
| `null`/`undefined` | `Option<T>` | Use `None` |
| `Array<T>` | `Vec<T>` | - |
| `object` | `struct` | Use `#[derive(Serialize, Deserialize)]` |
| Union types | `enum` | Use `#[serde(tag = "type")]` |
| `Map<K,V>` | `HashMap<K,V>` | - |
| `Set<T>` | `HashSet<T>` | - |

### Pattern Translations

#### 1. Promises to async/await

**JavaScript**:
```javascript
async function fetchData() {
  const response = await fetch(url);
  return response.json();
}

fetchData()
  .then(data => console.log(data))
  .catch(err => console.error(err));
```

**Rust**:
```rust
async fn fetch_data() -> Result<Value> {
    let response = reqwest::get(url).await?;
    let data = response.json().await?;
    Ok(data)
}

match fetch_data().await {
    Ok(data) => println!("{:?}", data),
    Err(err) => eprintln!("Error: {}", err),
}
```

#### 2. Classes to Structs + Implementations

**JavaScript**:
```javascript
class StreamParser {
  constructor(config) {
    this.config = config;
    this._buffer = [];
  }

  parse(event) {
    this._buffer.push(event);
    return this.processBuffer();
  }

  processBuffer() {
    // Implementation
  }
}
```

**Rust**:
```rust
pub struct StreamParser {
    config: Config,
    buffer: Vec<Event>,
}

impl StreamParser {
    pub fn new(config: Config) -> Self {
        Self {
            config,
            buffer: Vec::new(),
        }
    }

    pub fn parse(&mut self, event: Event) -> Result<ProcessedEvent> {
        self.buffer.push(event);
        self.process_buffer()
    }

    fn process_buffer(&self) -> Result<ProcessedEvent> {
        // Implementation
    }
}
```

#### 3. Interfaces to Traits

**JavaScript (TypeScript)**:
```typescript
interface EventHandler {
  handleEvent(event: Event): void;
  canHandle(event: Event): boolean;
}

class ToolUseHandler implements EventHandler {
  handleEvent(event: Event): void {
    // Implementation
  }

  canHandle(event: Event): boolean {
    return event.type === "tool_use";
  }
}
```

**Rust**:
```rust
pub trait EventHandler {
    fn handle_event(&self, event: &Event) -> Result<()>;
    fn can_handle(&self, event: &Event) -> bool;
}

pub struct ToolUseHandler;

impl EventHandler for ToolUseHandler {
    fn handle_event(&self, event: &Event) -> Result<()> {
        // Implementation
        Ok(())
    }

    fn can_handle(&self, event: &Event) -> bool {
        event.event_type == "tool_use"
    }
}
```

#### 4. Union Types to Enums

**JavaScript (TypeScript)**:
```typescript
type ContentBlock =
  | { type: "text"; text: string }
  | { type: "tool_use"; id: string; name: string; input: any }
  | { type: "tool_result"; tool_use_id: string; content: string };

function processBlock(block: ContentBlock) {
  switch (block.type) {
    case "text":
      return block.text;
    case "tool_use":
      return executeTool(block);
    case "tool_result":
      return formatResult(block);
  }
}
```

**Rust**:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ContentBlock {
    #[serde(rename = "text")]
    Text { text: String },

    #[serde(rename = "tool_use")]
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
    },

    #[serde(rename = "tool_result")]
    ToolResult {
        tool_use_id: String,
        content: String,
    },
}

fn process_block(block: &ContentBlock) -> Result<String> {
    match block {
        ContentBlock::Text { text } => Ok(text.clone()),
        ContentBlock::ToolUse { id, name, input } => execute_tool(id, name, input),
        ContentBlock::ToolResult { tool_use_id, content } => format_result(tool_use_id, content),
    }
}
```

#### 5. Callbacks to Closures

**JavaScript**:
```javascript
class HookRegistry {
  constructor() {
    this.hooks = new Map();
  }

  register(hookType, callback) {
    if (!this.hooks.has(hookType)) {
      this.hooks.set(hookType, []);
    }
    this.hooks.get(hookType).push(callback);
  }

  execute(hookType, context) {
    const callbacks = this.hooks.get(hookType) || [];
    callbacks.forEach(cb => cb(context));
  }
}
```

**Rust**:
```rust
use std::collections::HashMap;

pub type HookCallback = Box<dyn Fn(&Context) -> Result<()>>;

pub struct HookRegistry {
    hooks: HashMap<String, Vec<HookCallback>>,
}

impl HookRegistry {
    pub fn new() -> Self {
        Self {
            hooks: HashMap::new(),
        }
    }

    pub fn register(&mut self, hook_type: String, callback: HookCallback) {
        self.hooks
            .entry(hook_type)
            .or_insert_with(Vec::new)
            .push(callback);
    }

    pub fn execute(&self, hook_type: &str, context: &Context) -> Result<()> {
        if let Some(callbacks) = self.hooks.get(hook_type) {
            for callback in callbacks {
                callback(context)?;
            }
        }
        Ok(())
    }
}
```

#### 6. Error Handling

**JavaScript**:
```javascript
try {
  const result = await parseEvent(event);
  return result;
} catch (error) {
  if (error instanceof ValidationError) {
    console.error("Validation failed:", error.message);
    throw error;
  }
  console.error("Unexpected error:", error);
  throw new Error("Failed to parse event");
}
```

**Rust**:
```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ParseError {
    #[error("Validation failed: {0}")]
    Validation(String),

    #[error("Unexpected error: {0}")]
    Unexpected(String),
}

async fn parse_and_handle(event: &Event) -> Result<ParsedEvent, ParseError> {
    match parse_event(event).await {
        Ok(result) => Ok(result),
        Err(e) if e.is_validation_error() => {
            eprintln!("Validation failed: {}", e);
            Err(ParseError::Validation(e.to_string()))
        }
        Err(e) => {
            eprintln!("Unexpected error: {}", e);
            Err(ParseError::Unexpected("Failed to parse event".to_string()))
        }
    }
}
```

### Serde for JSON Handling

Claude Code uses JSON extensively. In Rust, use `serde_json`:

**JavaScript**:
```javascript
const contentBlock = {
  type: "tool_use",
  id: "toolu_123",
  name: "bash",
  input: { command: "ls -la" }
};

const json = JSON.stringify(contentBlock);
const parsed = JSON.parse(json);
```

**Rust**:
```rust
use serde::{Serialize, Deserialize};
use serde_json;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ContentBlock {
    #[serde(rename = "tool_use")]
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
    },
}

let content_block = ContentBlock::ToolUse {
    id: "toolu_123".to_string(),
    name: "bash".to_string(),
    input: serde_json::json!({ "command": "ls -la" }),
};

let json = serde_json::to_string(&content_block)?;
let parsed: ContentBlock = serde_json::from_str(&json)?;
```

### Common Patterns

#### Tagged Unions (Discriminated Unions)

**JavaScript**:
```javascript
// Type discrimination based on "type" field
if (block.type === "tool_use") {
  // TypeScript knows block has tool_use fields
}
```

**Rust**:
```rust
// Use serde's tag attribute for same behavior
#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Block {
    #[serde(rename = "tool_use")]
    ToolUse { id: String },
}

// Pattern matching handles discrimination
match block {
    Block::ToolUse { id } => println!("Tool use: {}", id),
}
```

#### Optional Fields

**JavaScript**:
```javascript
const config = {
  required: "value",
  optional: undefined,  // or missing
};
```

**Rust**:
```rust
#[derive(Serialize, Deserialize)]
pub struct Config {
    required: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    optional: Option<String>,
}
```

#### Default Values

**JavaScript**:
```javascript
function createConfig(options = {}) {
  return {
    timeout: options.timeout ?? 30,
    retries: options.retries ?? 3,
  };
}
```

**Rust**:
```rust
#[derive(Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "default_timeout")]
    timeout: u64,

    #[serde(default = "default_retries")]
    retries: u32,
}

fn default_timeout() -> u64 { 30 }
fn default_retries() -> u32 { 3 }

impl Default for Config {
    fn default() -> Self {
        Self {
            timeout: 30,
            retries: 3,
        }
    }
}
```

## Automation Scripts

### All-in-One Script

```bash
#!/bin/bash
# File: scripts/analyze-claude-code.sh
# Complete workflow for deminifying and analyzing Claude Code

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
RESEARCH_DIR="$HOME/src/RustyClawd/docs/research"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

echo_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

echo_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Check dependencies
check_dependencies() {
    echo_info "Checking dependencies..."

    if ! command -v npm &> /dev/null; then
        echo_error "npm not found. Please install Node.js"
        exit 1
    fi

    if ! command -v prettier &> /dev/null; then
        echo_warn "prettier not found. Installing..."
        npm install -g prettier
    fi

    if ! command -v js-beautify &> /dev/null; then
        echo_warn "js-beautify not found. Installing..."
        npm install -g js-beautify
    fi

    echo_info "All dependencies installed"
}

# Find Claude Code installation
find_claude_code() {
    echo_info "Locating Claude Code installation..."

    CLAUDE_CODE_PATH=$(readlink -f $(which claude) 2>/dev/null)
    if [ -z "$CLAUDE_CODE_PATH" ]; then
        echo_error "Claude Code not found in PATH"
        echo_error "Please install Claude Code first"
        exit 1
    fi

    CLAUDE_CODE_DIR=$(dirname "$CLAUDE_CODE_PATH")
    echo_info "Found Claude Code at: $CLAUDE_CODE_DIR"

    if [ ! -f "$CLAUDE_CODE_DIR/cli.js" ]; then
        echo_error "cli.js not found at expected location"
        exit 1
    fi
}

# Setup research directory
setup_research_dir() {
    echo_info "Setting up research directory..."
    mkdir -p "$RESEARCH_DIR"
    cd "$RESEARCH_DIR"
    echo_info "Research directory: $RESEARCH_DIR"
}

# Deminify files
deminify_files() {
    echo_info "Deminifying Claude Code..."

    # Copy original
    cp "$CLAUDE_CODE_DIR/cli.js" claude-code-minified.js
    echo_info "Copied minified file"

    # Deminify with prettier
    echo_info "Running prettier (this may take 20-30 seconds)..."
    prettier --write claude-code-minified.js --print-width 100 2>&1 | grep -v "^$" || true

    # Create js-beautify version
    echo_info "Running js-beautify..."
    cp "$CLAUDE_CODE_DIR/cli.js" claude-code-jsbeautify.js
    js-beautify -r claude-code-jsbeautify.js 2>&1 | grep -v "^$" || true

    # Report
    PRETTIER_LINES=$(wc -l < claude-code-minified.js)
    BEAUTIFY_LINES=$(wc -l < claude-code-jsbeautify.js)

    echo_info "Deminification complete!"
    echo_info "  Prettier version: $PRETTIER_LINES lines"
    echo_info "  js-beautify version: $BEAUTIFY_LINES lines"
}

# Create search indices
create_indices() {
    echo_info "Creating search indices..."

    # ContentBlock patterns
    grep -n "ContentBlock\|content_block" claude-code-minified.js > index-contentblock.txt || true
    echo_info "  ContentBlock: $(wc -l < index-contentblock.txt) matches"

    # Streaming patterns
    grep -n "streaming\|StreamingEvent\|message_start\|content_block_start" \
        claude-code-minified.js > index-streaming.txt || true
    echo_info "  Streaming: $(wc -l < index-streaming.txt) matches"

    # Hook patterns
    grep -n "hook\|Hook\|lifecycle" claude-code-minified.js > index-hooks.txt || true
    echo_info "  Hooks: $(wc -l < index-hooks.txt) matches"

    # Tool execution
    grep -n "tool_use\|ToolUse\|execute_tool" claude-code-minified.js > index-tools.txt || true
    echo_info "  Tools: $(wc -l < index-tools.txt) matches"

    # Session management
    grep -n "session\|Session\|sessionId" claude-code-minified.js > index-session.txt || true
    echo_info "  Session: $(wc -l < index-session.txt) matches"

    # Thinking blocks
    grep -n "thinking\|ThinkingBlock\|interleaved-thinking" \
        claude-code-minified.js > index-thinking.txt || true
    echo_info "  Thinking: $(wc -l < index-thinking.txt) matches"

    echo_info "Indices created in: $RESEARCH_DIR/index-*.txt"
}

# Interactive search helper
interactive_search() {
    echo ""
    echo_info "Research directory ready: $RESEARCH_DIR"
    echo ""
    echo "Available files:"
    echo "  - claude-code-minified.js (prettier formatted)"
    echo "  - claude-code-jsbeautify.js (js-beautify formatted)"
    echo "  - index-*.txt (search indices)"
    echo ""
    echo "Example searches:"
    echo "  grep -n 'pattern' claude-code-minified.js | head -20"
    echo "  cat index-contentblock.txt | grep 'TextContentBlock'"
    echo "  less +/ContentBlock claude-code-minified.js"
    echo ""

    # Offer to open in editor
    read -p "Open research directory in VS Code? (y/n) " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        code "$RESEARCH_DIR"
    fi
}

# Main execution
main() {
    echo_info "Claude Code Deminification and Analysis"
    echo_info "========================================"
    echo ""

    check_dependencies
    find_claude_code
    setup_research_dir
    deminify_files
    create_indices
    interactive_search

    echo ""
    echo_info "Analysis setup complete!"
}

# Run
main
```

Make it executable:

```bash
chmod +x scripts/analyze-claude-code.sh
./scripts/analyze-claude-code.sh
```

## Tips for Effective Analysis

### 1. Use Context Around Matches

```bash
# Show 5 lines before and after match
grep -B 5 -A 5 "ContentBlock" claude-code-minified.js | less
```

### 2. Combine Patterns

```bash
# Find all ContentBlock definitions
grep -n "ContentBlock.*=" claude-code-minified.js | grep -v "function"
```

### 3. Extract Code Sections

```bash
# Extract lines 126300-126400 (ContentBlock definitions)
sed -n '126300,126400p' claude-code-minified.js > contentblock-defs.js
```

### 4. Compare Implementations

```bash
# Compare how prettier vs js-beautify formatted a section
diff <(sed -n '1000,1100p' claude-code-minified.js) \
     <(sed -n '1000,1100p' claude-code-jsbeautify.js)
```

### 5. Search Multiple Patterns

```bash
# Find all event types
grep -E "(message_start|content_block_start|content_block_delta|ping)" \
    claude-code-minified.js | head -30
```

### 6. Use Your Editor

Open in VS Code or your preferred editor for:
- Syntax highlighting
- Find and replace
- Multi-cursor editing
- Better navigation

```bash
code ~/src/RustyClawd/docs/research/
```

## Common Analysis Workflows

### Understanding a New Feature

1. **Search for feature name** in deminified code
2. **Find type definitions** (constants, enums, classes)
3. **Trace execution flow** (method calls, event handling)
4. **Identify data structures** (what gets passed around)
5. **Map to Rust equivalents** using translation guide

### Debugging Implementation Differences

1. **Compare behavior** between Claude Code and RustyClawd
2. **Search for relevant code** in deminified JavaScript
3. **Identify the difference** in logic or structure
4. **Update Rust implementation** to match
5. **Test to verify** parity restored

### Adding New Features

1. **Find similar features** in Claude Code
2. **Extract pattern** from JavaScript implementation
3. **Design Rust equivalent** using translation guide
4. **Implement with tests**
5. **Verify behavior matches** Claude Code

## Troubleshooting

### Prettier Takes Too Long

If prettier is slow or hangs:

```bash
# Use js-beautify instead (faster)
js-beautify -r claude-code-minified.js

# Or limit prettier to smaller sections
head -100000 claude-code-original.js > section1.js
prettier --write section1.js
```

### Can't Find Pattern

Try:
- Case-insensitive search: `grep -i "pattern"`
- Regex search: `grep -E "pattern1|pattern2"`
- Search in both files: `grep "pattern" claude-code-*.js`
- Check the indices: `cat index-*.txt | grep "pattern"`

### Memory Issues

If your editor struggles with large files:

```bash
# Split into smaller files
split -l 50000 claude-code-minified.js section-

# Search in sections
for file in section-*; do
    echo "=== $file ==="
    grep "pattern" "$file"
done
```

## Next Steps

After deminification and analysis:

1. **Document findings** in `/home/azureuser/src/RustyClawd/docs/research/findings.md`
2. **Create Rust implementations** based on patterns discovered
3. **Write tests** to verify parity
4. **Update this guide** with new patterns as you find them

## Contributing Back

If you discover useful patterns:

1. Add them to this guide under "Key Pattern Search Reference"
2. Include JavaScript snippets from Claude Code
3. Provide Rust translation examples
4. Submit PR to share with team

## Resources

- [Prettier Documentation](https://prettier.io/docs/en/options.html)
- [js-beautify Documentation](https://github.com/beautify-web/js-beautify)
- [Serde Documentation](https://serde.rs/)
- [Tokio Async Guide](https://tokio.rs/tokio/tutorial)
- [Rust Error Handling](https://doc.rust-lang.org/book/ch09-00-error-handling.html)

## Summary

This guide provides:
- Complete deminification workflow
- Key pattern search strategies
- JavaScript to Rust translation patterns
- Automation scripts for efficiency
- Real examples from Claude Code

Use this as your reference when learning from Claude Code's implementation to improve RustyClawd's parity and completeness.
