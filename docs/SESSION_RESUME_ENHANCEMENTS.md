# Session Resume Enhancements - Architectural Design

## Overview

This document specifies enhancements to RustyClawd's session resume functionality to support PR-to-session linking, memory-efficient loading, and improved user experience.

## Key Requirements

1. **PR-to-session linking mechanism** - Link sessions to GitHub PR numbers
2. **--from-pr CLI flag** - Resume session from PR number
3. **68% memory reduction via lazy loading** - Only load message content when needed
4. **Cycle detection and recovery** - Prevent infinite loops in session chains
5. **GitHub auto-linking** - Automatically detect and link PR context
6. **Resume hints on exit** - Show users how to resume their session

## Architecture

### Module Structure (Bricks & Studs Philosophy)

```
crates/cli/src/
├── session_index.rs         # NEW - Fast PR/session lookups
├── session_graph.rs         # NEW - Cycle detection in session chains
├── session_persistence.rs   # ENHANCED - PR linking integration
├── checkpoint/
│   ├── loader.rs            # ENHANCED - Lazy loading support
│   └── types.rs             # ENHANCED - PR metadata
└── main.rs                  # ENHANCED - --from-pr flag
```

### 1. Session Index Module (`session_index.rs`)

**Purpose**: Fast bidirectional lookups between PR numbers and session IDs

**Public API:**
```rust
pub struct SessionIndex {
    pr_to_session: HashMap<u64, Vec<String>>,
    session_to_pr: HashMap<String, u64>,
}

impl SessionIndex {
    pub fn new() -> Result<Self>;
    pub fn link_pr(&mut self, session_id: &str, pr_number: u64) -> Result<()>;
    pub fn find_sessions_by_pr(&self, pr_number: u64) -> Option<&[String]>;
    pub fn find_pr_by_session(&self, session_id: &str) -> Option<u64>;
    pub fn remove_session(&mut self, session_id: &str) -> Result<()>;
}
```

**Storage**: JSON file at `~/.config/claude/session_index.json`

**Structure**:
```json
{
  "pr_to_session": {
    "123": ["session-abc", "session-def"],
    "456": ["session-xyz"]
  },
  "session_to_pr": {
    "session-abc": 123,
    "session-def": 123,
    "session-xyz": 456
  },
  "last_updated": "2026-02-09T21:32:00Z"
}
```

### 2. Session Graph Module (`session_graph.rs`)

**Purpose**: Track session chains and detect cycles

**Public API:**
```rust
pub struct SessionGraph {
    edges: HashMap<String, String>, // child -> parent
    children: HashMap<String, Vec<String>>, // parent -> children
}

impl SessionGraph {
    pub fn new() -> Result<Self>;
    pub fn add_edge(&mut self, from: &str, to: &str) -> Result<()>;
    pub fn detect_cycle(&self, session_id: &str) -> Option<Vec<String>>;
    pub fn get_chain(&self, session_id: &str) -> Vec<String>;
    pub fn max_depth(&self, session_id: &str) -> usize;
}
```

**Cycle Detection Algorithm:**
- DFS with visited set
- Return cycle path when detected
- Maximum chain depth limit: 100 sessions

### 3. Enhanced Session Persistence

**New Fields in SessionInfo:**
```rust
pub struct SessionInfo {
    pub session_id: String,
    pub last_checkpoint_time: u64,
    pub age_hours: f64,
    pub checkpoint_count: usize,
    pub message_count: usize,
    pub pr_number: Option<u64>,        // NEW
    pub parent_session: Option<String>, // NEW
}
```

**New Methods:**
```rust
impl SessionPersistence {
    pub fn link_to_pr(&mut self, pr_number: u64) -> Result<()>;
    pub fn get_pr_number(&self) -> Option<u64>;
    pub fn set_parent_session(&mut self, parent_id: &str) -> Result<()>;
    pub fn detect_cycles(&self) -> Option<Vec<String>>;
}
```

### 4. Lazy Loading Implementation

**Current Memory Usage:**
- Full message content loaded: ~147 bytes/message average
- 1000 messages = ~147 KB

**Optimized Memory Usage (68% reduction):**
- Message metadata only: ~47 bytes/message
- Content loaded on-demand from checkpoint files
- 1000 messages = ~47 KB (68% reduction)

**Implementation in `checkpoint/types.rs`:**

```rust
/// Lazy-loadable checkpoint message
pub enum CheckpointMessage {
    /// Fully loaded message (in memory)
    Loaded {
        role: String,
        content: Vec<ContentBlock>,
        timestamp: u64,
    },
    /// Metadata only (content on disk)
    Lazy {
        role: String,
        content_offset: u64,      // Byte offset in checkpoint file
        content_length: u32,      // Content size in bytes
        timestamp: u64,
        checkpoint_path: PathBuf, // Path to checkpoint file
    },
}

impl CheckpointMessage {
    /// Load content if lazy
    pub fn ensure_loaded(&mut self) -> Result<()>;

    /// Get role without loading content
    pub fn role(&self) -> &str;

    /// Get timestamp without loading content
    pub fn timestamp(&self) -> u64;
}
```

**Lazy Loading Strategy:**
1. When listing sessions: Load metadata only
2. When resuming: Load last N messages fully (N=10 default)
3. On scroll/navigation: Load messages on-demand
4. Cache recently accessed messages (LRU cache, max 50 messages)

### 5. CLI Flag Implementation

**New Flag in main.rs:**
```rust
struct Cli {
    // ... existing fields ...

    /// Resume from session linked to PR number
    #[arg(long = "from-pr")]
    from_pr: Option<u64>,
}
```

**Execution Flow:**
```
1. Parse --from-pr flag
2. Look up sessions via SessionIndex
3. If multiple sessions found:
   - Show interactive selector
   - Default to most recent
4. If no sessions found:
   - Show error with hint to link manually
5. Resume selected session
6. Check for cycles before resume
```

### 6. GitHub Auto-Linking

**Detection Triggers:**
1. User mentions PR in prompt: "continue working on PR #123"
2. Branch name contains PR number: `pr-123-feature`
3. Git branch has upstream tracking to PR

**Implementation:**
```rust
pub struct GitHubAutoLinker {
    git_repo: Option<Repository>,
}

impl GitHubAutoLinker {
    pub fn detect_pr_from_prompt(prompt: &str) -> Option<u64>;
    pub fn detect_pr_from_branch() -> Result<Option<u64>>;
    pub fn suggest_link(&self, session_id: &str) -> Option<u64>;
}
```

**Heuristics:**
- Regex: `(?i)pr\s*#?(\d+)` in prompt
- Branch pattern: `pr-(\d+)-.*` or `feat/issue-(\d+)-.*`
- GitHub remote upstream branch name parsing

### 7. Resume Hints on Exit

**Display on session exit:**
```
Session saved successfully!

Resume options:
  claude --resume <session-id>
  claude --continue           (resume last session)
  claude --from-pr 123        (if linked to PR #123)

Session ID: abc123def
Linked to: PR #123
Duration: 45 minutes
Messages: 87
```

**Implementation in TUI exit handler:**
```rust
fn show_exit_hints(session_info: &SessionInfo) {
    println!("\nSession saved successfully!\n");
    println!("Resume options:");
    println!("  claude --resume {}", session_info.session_id);
    println!("  claude --continue");

    if let Some(pr) = session_info.pr_number {
        println!("  claude --from-pr {}  (linked to this session)", pr);
    }

    println!("\nSession ID: {}", session_info.session_id);
    if let Some(pr) = session_info.pr_number {
        println!("Linked to: PR #{}", pr);
    }
    println!("Duration: {}", format_duration(session_info.duration));
    println!("Messages: {}", session_info.message_count);
}
```

## Implementation Plan

### Phase 1: Core Infrastructure
1. Create `session_index.rs` with persistence
2. Create `session_graph.rs` with cycle detection
3. Add tests for both modules

### Phase 2: Lazy Loading
1. Update `checkpoint/types.rs` with lazy message enum
2. Implement content-on-demand loading in `checkpoint/loader.rs`
3. Add LRU cache for loaded messages
4. Performance tests

### Phase 3: CLI Integration
1. Add `--from-pr` flag to main.rs
2. Wire SessionIndex lookup into resume flow
3. Add cycle detection check before resume
4. Integration tests

### Phase 4: GitHub Auto-Linking
1. Implement GitHubAutoLinker
2. Hook into session creation
3. Prompt user to confirm auto-detected links

### Phase 5: UX Enhancements
1. Add resume hints to exit flow
2. Improve session selector UI
3. Add PR info to session list display

## Testing Strategy

### Unit Tests
- SessionIndex: Add/remove/lookup operations
- SessionGraph: Cycle detection, chain traversal
- Lazy loading: Load on demand, cache behavior

### Integration Tests
- End-to-end PR linking workflow
- Resume from PR with multiple sessions
- Cycle detection in real session chains

### Performance Tests
- Memory usage with 1000+ message sessions
- Lazy loading performance (target: <10ms per message load)
- SessionIndex lookup performance (target: <1ms)

## Migration Strategy

**Backward Compatibility:**
- Existing sessions work without PR links
- Lazy loading opt-in via feature flag initially
- SessionIndex created on first use

**Upgrade Path:**
1. Install new version
2. SessionIndex auto-created empty
3. Users can link existing sessions manually: `claude session link <id> --pr <num>`
4. New sessions auto-link if PR detected

## Success Criteria

1. ✅ 68% memory reduction measured in production
2. ✅ --from-pr flag resolves correct session 100% of time
3. ✅ Zero cycle-related crashes or infinite loops
4. ✅ Auto-linking accuracy >80% (measured on sample PRs)
5. ✅ Resume hints displayed on every exit
6. ✅ All tests passing (unit + integration + performance)

## Open Questions

1. Should we support linking to closed PRs?
   - **Decision: Yes, for historical reference**

2. How to handle session linked to PR that gets deleted?
   - **Decision: Keep link, show "(PR deleted)" in UI**

3. Maximum session chain depth?
   - **Decision: 100 sessions, error on exceed**

4. Should lazy loading be default or opt-in?
   - **Decision: Default for lists, disabled for active session**

## References

- Existing checkpoint system: `crates/cli/src/checkpoint/`
- Session persistence: `crates/cli/src/session_persistence.rs`
- CLI args: `crates/cli/src/main.rs`
