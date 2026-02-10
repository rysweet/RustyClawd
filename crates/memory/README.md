# RustyClawd Memory System

High-performance, persistent memory system for AI agents with SQLite backend.

## Features

- **<50ms Operations**: Fast SQLite-based storage (typical: 2-15ms)
- **Scope Hierarchy**: Local > Project > User memory organization
- **Session Management**: Automatic session-based isolation
- **Thread-Safe**: Concurrent access with proper locking
- **Type-Safe API**: Rust's type system prevents errors
- **Zero-BS Implementation**: All functionality works, no stubs

## Philosophy

This crate follows RustyClawd's core principles:

- **Ruthless Simplicity**: Clean API without unnecessary abstractions
- **Modular Design**: Self-contained brick with clear public interface
- **Performance First**: Every operation optimized for <50ms
- **Quality over Speed**: Robust, well-tested implementation

## Quick Start

Add to your `Cargo.toml`:

```toml
[dependencies]
rustyclawd-memory = { path = "../memory" }
```

### Basic Usage

```rust
use rustyclawd_memory::{MemoryManager, MemoryType, MemoryScope};

// Create a memory manager
let manager = MemoryManager::new()?;

// Store a memory
let id = manager.store(
    "architect",
    "API Design Decision",
    "Use REST API with JSON responses",
    MemoryType::Decision,
    MemoryScope::Project,
)?;

// Retrieve important memories
let important = manager.important(Some(10))?;

// Search memories
let results = manager.search("API design", Some(10))?;
```

### With Session Management

```rust
// Create manager with session
let manager = MemoryManager::with_session("session-123")?;

// Session ID automatically attached to all memories
let id = manager.store(
    "agent1",
    "Session Context",
    "User is working on authentication system",
    MemoryType::Context,
    MemoryScope::Local,
)?;

// Retrieve session-specific memories
let memories = manager.retrieve_session_memories(None, None)?;
```

### Builder Pattern

```rust
use rustyclawd_memory::MemoryBuilder;

let id = MemoryBuilder::new(
    "architect",
    "Architecture Decision",
    "Use microservices pattern",
    MemoryType::Decision,
    MemoryScope::Project,
)
.importance(9)
.tag("architecture")
.tag("design")
.metadata("reason".to_string(), serde_json::json!("scalability"))
.store(&manager)?;
```

## Memory Types

- `Conversation`: Chat history and context
- `Decision`: Architectural and design decisions
- `Pattern`: Recognized code patterns and solutions
- `Context`: Session context and state
- `Learning`: Accumulated knowledge
- `Artifact`: Generated code, docs, etc.

## Memory Scopes

Hierarchy from highest to lowest priority:

1. `Local`: Session-specific memories
2. `Project`: Shared across sessions within a project
3. `User`: Shared across all projects for a user

## Database Location

Default: `~/.rustyclawd/memory.db`

Custom location:

```rust
let manager = MemoryManager::with_db_path("/custom/path/memory.db")?;
```

## API Reference

### MemoryManager

Core interface for memory operations.

#### Storage

- `store()` - Store a new memory
- `store_entry()` - Store a pre-built MemoryEntry
- `delete()` - Delete a memory by ID

#### Retrieval

- `get()` - Get specific memory by ID
- `retrieve()` - Query with filters
- `retrieve_session_memories()` - Get current session memories
- `search()` - Full-text search
- `recent()` - Get recent memories
- `important()` - Get high-importance memories (≥8)
- `by_scope()` - Filter by memory scope

#### Maintenance

- `cleanup_expired()` - Remove expired memories
- `stats()` - Get database statistics

#### Session Management

- `session_id()` - Get current session ID
- `set_session_id()` - Set session ID
- `clear_session_id()` - Clear session ID

### MemoryQuery

Flexible query builder for complex filtering:

```rust
let query = MemoryQuery::new()
    .agent_id("architect")
    .memory_type(MemoryType::Decision)
    .min_importance(8)
    .add_tag("api")
    .created_after(start_time)
    .limit(10);

let results = manager.retrieve(&query)?;
```

### MemoryBuilder

Fluent API for creating memories:

```rust
let entry = MemoryBuilder::new(agent_id, title, content, type, scope)
    .importance(9)
    .tag("important")
    .metadata(key, value)
    .expires_at(expiration_time)
    .build();
```

## Performance

All operations meet <50ms target:

- Storage: ~2-4ms
- Retrieval: ~2-3ms
- Search: ~2-5ms
- Query with filters: ~3-8ms

Based on testing with:
- 1000+ entries
- Multiple concurrent operations
- Complex queries

## Testing

Run tests:

```bash
cargo test -p rustyclawd-memory
```

Test coverage includes:

- Unit tests for types and core logic
- Integration tests for database operations
- End-to-end workflow tests
- Performance benchmarks

## Thread Safety

The memory system is fully thread-safe:

- Arc-wrapped database connection
- Mutex for concurrent access
- WAL mode for SQLite concurrency
- No data races or corruption

## Error Handling

All operations return `Result<T, anyhow::Error>`:

```rust
match manager.store(agent_id, title, content, type, scope) {
    Ok(id) => println!("Stored memory: {}", id),
    Err(e) => eprintln!("Failed to store: {}", e),
}
```

## Integration Examples

### Agent Memory

```rust
struct ArchitectAgent {
    memory: MemoryManager,
    agent_id: String,
}

impl ArchitectAgent {
    fn record_decision(&self, title: &str, content: &str) -> Result<String> {
        self.memory.store(
            &self.agent_id,
            title,
            content,
            MemoryType::Decision,
            MemoryScope::Project,
        )
    }

    fn recall_decisions(&self) -> Result<Vec<MemoryEntry>> {
        let query = MemoryQuery::new()
            .agent_id(&self.agent_id)
            .memory_type(MemoryType::Decision)
            .min_importance(7);

        self.memory.retrieve(&query)
    }
}
```

### Session Context

```rust
fn preserve_session_context(manager: &MemoryManager, summary: &str) -> Result<String> {
    manager.store(
        "session_manager",
        "Session Context Snapshot",
        summary,
        MemoryType::Context,
        MemoryScope::Local,
    )
}

fn restore_session_context(manager: &MemoryManager) -> Result<Vec<MemoryEntry>> {
    manager.retrieve_session_memories(
        Some("session_manager"),
        Some(MemoryType::Context),
    )
}
```

## Maintenance

### Cleanup

```rust
// Remove expired memories
let cleaned = manager.cleanup_expired()?;
println!("Cleaned up {} expired memories", cleaned);

// Get stats
let stats = manager.stats()?;
println!("Total entries: {}", stats.total_entries);
println!("Database size: {} bytes", stats.database_size_bytes);
```

### Expiration

Set expiration for temporary memories:

```rust
use chrono::Duration;

let entry = MemoryEntry::new(
    "temp_agent",
    "Temporary Context",
    "Short-lived data",
    MemoryType::Context,
    MemoryScope::Local,
)
.with_expiration(chrono::Utc::now() + Duration::hours(1));

manager.store_entry(entry)?;
```

## Architecture

### Module Structure

```
crates/memory/
├── src/
│   ├── lib.rs           # Public API
│   ├── types.rs         # Data models
│   ├── database.rs      # SQLite backend
│   └── manager.rs       # High-level interface
├── Cargo.toml           # Dependencies
└── README.md            # This file
```

### Database Schema

- `memory_entries`: Core memory storage
  - Indexed on: agent_id, session_id, memory_type, scope, importance, created_at
  - Full-text search on: title, content
  - Supports hierarchical organization via parent_id

- `schema_version`: Migration tracking

### Design Decisions

1. **SQLite over in-memory**: Persistence across sessions
2. **WAL mode**: Better concurrency
3. **Arc<Mutex<>>**: Thread-safe, simple
4. **Builder pattern**: Ergonomic API
5. **Type-safe enums**: Prevent invalid states

## Contributing

This crate follows RustyClawd development principles:

- Write tests first (TDD)
- Keep it simple
- No stubs or placeholders
- Performance matters

## License

Same as RustyClawd: MIT OR Apache-2.0
