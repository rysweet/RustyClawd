# Contributing to RustyClawd

Ahoy matey! Welcome aboard the RustyClawd ship! We be thrilled ye want to contribute to this here Rust implementation of Claude Code CLI. This guide will help ye get started on yer journey.

## Table of Contents

- [Getting Started](#getting-started)
- [Development Guidelines](#development-guidelines)
- [Project Philosophy](#project-philosophy)
- [Pull Request Process](#pull-request-process)
- [Testing Strategy](#testing-strategy)
- [Where to Get Help](#where-to-get-help)

---

## Getting Started

### Prerequisites

Before ye set sail, make sure ye have these tools installed:

**Required:**
- **Rust 1.75+** (stable toolchain)
  ```bash
  # Install or update Rust
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
  rustup update stable
  ```

**System Dependencies:**

- **Linux (Ubuntu/Debian)**:
  ```bash
  sudo apt install libssl-dev pkg-config
  ```

- **Linux (Fedora/RHEL)**:
  ```bash
  sudo dnf install openssl-devel pkg-config
  ```

- **macOS**:
  ```bash
  brew install openssl pkg-config
  ```

- **Windows**:
  - Install OpenSSL binaries from https://slproweb.com/products/Win32OpenSSL.html
  - Or use vcpkg: `vcpkg install openssl:x64-windows`

### Quick Start

Get up and running in minutes:

```bash
# 1. Clone the repository
git clone https://github.com/rysweet/RustyClawd
cd RustyClawd

# 2. Build the project
cargo build --release

# 3. Run tests to verify everything works
cargo test

# 4. Try running Claude Code
./target/release/claude --help

# 5. Create an alias for convenience (optional)
alias claude="$PWD/target/release/claude"
```

**Verify Installation:**
```bash
# Should show version info
./target/release/claude --version

# Try interactive mode
./target/release/claude
```

---

## Development Guidelines

### Code Style

We follow standard Rust conventions with strict enforcement:

**Formatting:**
- Use `rustfmt` for all code formatting
- Run before every commit: `cargo fmt`
- Configuration is in `rustfmt.toml` (if present)

**Linting:**
- Pass `clippy` with zero warnings
- Run: `cargo clippy -- -D warnings`
- Fix all suggestions before submitting PR

**Code Quality:**
- No compiler warnings allowed in CI
- Write idiomatic Rust code
- Prefer explicit error handling with `Result` types
- Avoid `unwrap()` and `expect()` in production code (use `?` operator)

**Example - Good Error Handling:**
```rust
// ❌ BAD - Panics on error
fn read_config() -> Config {
    let content = fs::read_to_string("config.json").unwrap();
    serde_json::from_str(&content).unwrap()
}

// ✅ GOOD - Returns Result
fn read_config() -> Result<Config> {
    let content = fs::read_to_string("config.json")
        .context("Failed to read config file")?;
    let config = serde_json::from_str(&content)
        .context("Failed to parse config")?;
    Ok(config)
}
```

### Testing Requirements

We follow the **testing pyramid** approach:

- **60% Unit Tests** - Fast, isolated tests of individual functions
- **30% Integration Tests** - Tests of multiple components working together
- **10% E2E Tests** - Complete workflow tests

**Requirements:**
- All tests must pass: `cargo test`
- New features require new tests
- Bug fixes should include regression tests
- Aim for ~85% code coverage

**Current Test Status:**
- Total tests: ~1,500 tests
- Pass rate: 100% (all tests passing)
- Test execution time: ~20 seconds

**Running Tests:**
```bash
# Run all tests
cargo test

# Run specific test
cargo test test_save_and_load_checkpoint

# Run tests for a specific module
cargo test session_persistence

# Run with output
cargo test -- --nocapture

# Run tests in specific crate
cargo test -p claude-tools
```

### Documentation

Clear documentation helps everyone:

**Public APIs:**
- All public functions, structs, and modules need rustdoc comments
- Include examples in doc comments where helpful
- Explain **why**, not just **what**

**Example:**
```rust
/// Saves a session checkpoint to disk for later resumption.
///
/// This allows users to pause work and resume later without losing
/// conversation context or tool execution history.
///
/// # Arguments
/// * `session_id` - Unique identifier for the session
/// * `state` - Current conversation state to persist
///
/// # Returns
/// Path to the saved checkpoint file
///
/// # Example
/// ```rust
/// let checkpoint = save_checkpoint("session-123", &state)?;
/// println!("Saved to: {}", checkpoint.display());
/// ```
pub fn save_checkpoint(session_id: &str, state: &State) -> Result<PathBuf> {
    // Implementation...
}
```

**Complex Logic:**
- Add inline comments for non-obvious code
- Explain algorithms and design decisions
- Reference relevant issues or RFCs

**Architectural Changes:**
- Update `docs/ARCHITECTURE.md` for structural changes
- Document new modules in the Module Structure section
- Update diagrams if architecture changes significantly

---

## Project Philosophy

RustyClawd follows a clear development philosophy focused on simplicity, quality, and modularity.

### Core Principles

For complete details, see [`.claude/context/PHILOSOPHY.md`](.claude/context/PHILOSOPHY.md). Here are the key principles:

#### 1. Ruthless Simplicity

> "Make things as simple as possible, but no simpler."

- **Start minimal** - Begin with the simplest solution that works
- **Avoid over-engineering** - Don't build for hypothetical future requirements
- **Question abstractions** - Every layer must justify its existence
- **Prefer clarity** - Simple, obvious code beats clever code

**Example:**
```rust
// ❌ BAD - Over-engineered with unnecessary abstraction
trait MessageProcessor {
    fn process(&self, msg: Message) -> ProcessedMessage;
}

struct StandardProcessor;
impl MessageProcessor for StandardProcessor { /* ... */ }

// ✅ GOOD - Direct and simple
fn process_message(msg: Message) -> ProcessedMessage {
    // Simple, obvious transformation
    ProcessedMessage {
        content: msg.content.trim().to_string(),
        timestamp: Utc::now(),
    }
}
```

#### 2. Zero-BS Implementation

> "Every function must work or not exist."

- **No TODOs** - Don't commit unimplemented functions
- **No stubs** - Every function must have working code
- **No dead code** - Remove unused functions and imports
- **Quality over speed** - Take time to do it right
- **No swallowed errors** - Handle errors transparently

**Example:**
```rust
// ❌ BAD - Stub that does nothing
fn validate_model(model: &str) -> Result<()> {
    todo!("Add model validation")
}

// ❌ BAD - Silent failure
fn validate_model(model: &str) -> Result<()> {
    Ok(()) // TODO: Actually validate
}

// ✅ GOOD - Working implementation
fn validate_model(model: &str) -> Result<()> {
    const VALID_MODELS: &[&str] = &[
        "claude-3-opus-20240229",
        "claude-3-sonnet-20240229",
        "claude-3-haiku-20240307",
    ];

    if VALID_MODELS.contains(&model) {
        Ok(())
    } else {
        Err(anyhow!("Invalid model: {}", model))
    }
}
```

#### 3. Modular Design (The Brick Philosophy)

> "Build self-contained modules that can be regenerated independently."

- **Single responsibility** - Each module does one thing well
- **Clear interfaces** - Public APIs are well-defined and stable
- **Self-contained** - Modules include their own tests and examples
- **Regeneratable** - Can rebuild from spec without breaking connections

**Module Structure:**
```
crates/
├── cli/           # User interface (TUI, commands)
├── core/          # Core types and Claude API client
└── tools/         # Tool implementations (Read, Write, Bash, etc.)
```

Each crate is a "brick" with clear boundaries and defined public APIs.

#### 4. Testing Philosophy

- **Test behavior** - Focus on what code does, not how it does it
- **Test at boundaries** - Test public APIs and module interfaces
- **Manual testability** - Design code that's easy to test manually
- **Fast feedback** - Tests should run quickly (< 30 seconds)

### Decision-Making Framework

When faced with implementation decisions, ask:

1. **Necessity**: "Do we actually need this right now?"
2. **Simplicity**: "What's the simplest way to solve this?"
3. **Modularity**: "Can this be a self-contained module?"
4. **Value**: "Does the complexity add proportional value?"
5. **Maintenance**: "How easy will this be to change later?"

### When to Embrace Complexity

Some areas **justify** additional complexity:

- **Security** - Never compromise on security fundamentals
- **Data integrity** - Ensure data consistency and reliability
- **Core UX** - Make primary user flows smooth and reliable
- **Error handling** - Make problems obvious and diagnosable

### When to Aggressively Simplify

Push for **extreme simplicity** in:

- **Internal abstractions** - Minimize layers between components
- **Future-proofing** - Resist solving non-existent problems
- **Edge cases** - Handle common cases well first
- **Framework usage** - Use only what you need

---

## Pull Request Process

### Creating a Pull Request

Follow these steps to create a high-quality PR:

#### 1. Create a Feature Branch

Use descriptive branch names:
```bash
git checkout -b feat/issue-123-add-session-resume
git checkout -b fix/issue-456-handle-timeout
git checkout -b docs/improve-architecture-guide
```

#### 2. Make Changes and Add Tests

- Implement your changes following the style guide
- Add tests for new functionality
- Update documentation as needed

#### 3. Run Pre-Commit Checks

**Always run these before committing:**

```bash
# Format code
cargo fmt

# Check for lint errors
cargo clippy -- -D warnings

# Run all tests
cargo test

# Optional: Run tests with coverage
cargo tarpaulin --out Html
```

All checks must pass! Fix any issues before proceeding.

#### 4. Commit with Conventional Commits

Use clear, descriptive commit messages:

**Format:** `<type>: <description>`

**Types:**
- `feat:` - New feature
- `fix:` - Bug fix
- `docs:` - Documentation changes
- `test:` - Test additions or changes
- `refactor:` - Code refactoring (no behavior change)
- `perf:` - Performance improvements
- `chore:` - Build process or tooling changes

**Examples:**
```bash
git commit -m "feat: Add session resume capability"
git commit -m "fix: Handle connection timeout in MCP proxy"
git commit -m "docs: Update hook lifecycle documentation"
git commit -m "test: Add integration tests for session persistence"
```

#### 5. Push and Create PR

```bash
# Push your branch
git push origin feat/issue-123-add-session-resume

# Create PR on GitHub with clear description
# Include:
# - What changed and why
# - Link to related issue (#123)
# - Testing performed
# - Screenshots (for UI changes)
```

### PR Requirements Checklist

Before submitting, ensure:

- [ ] All tests pass locally (`cargo test`)
- [ ] No clippy warnings (`cargo clippy -- -D warnings`)
- [ ] Code formatted (`cargo fmt --check`)
- [ ] Documentation updated (rustdoc, ARCHITECTURE.md)
- [ ] Linked to related issue (e.g., "Closes #123")
- [ ] Clear PR description with context
- [ ] Tests added for new functionality
- [ ] No `unwrap()` or `expect()` in production code
- [ ] **No Claude branding in UI strings** (run `./scripts/check-branding.sh`)

#### Branding Guidelines

**RustyClawd has its own brand identity separate from Claude Code.** When writing user-facing strings:

✅ **USE:**
- "RustyClawd" - For the application name
- "Assistant" - For the AI role in messages
- Generic terms like "AI", "the assistant", or "chatting"

❌ **AVOID:**
- "Claude" - Except in allowed contexts (see below)

**Allowed "Claude" contexts:**
- API model names (e.g., `"claude-sonnet-4-5"`)
- Directory paths (e.g., `.claude/` plugin spec)
- Internal variable names (e.g., `claude_client`)
- Comments and documentation (attribution/context)
- Debug/trace logging (internal only)

**Examples:**

```rust
// ❌ BAD - Claude branding in UI
println!("Welcome to Claude!");
format!("[{}] Claude: {}", timestamp, message);

// ✅ GOOD - RustyClawd branding
println!("Welcome to RustyClawd!");
format!("[{}] Assistant: {}", timestamp, message);

// ✅ ALLOWED - API model name
let model = "claude-sonnet-4-5";

// ✅ ALLOWED - Internal logging
tracing::debug!("Claude API returned status {}", status);
```

**Validation:**

Run the branding validation script before submitting:

```bash
# Check for Claude branding violations
./scripts/check-branding.sh

# Run branding validation tests
cargo test --package rustyclawd-cli --test branding_test
```

Both must pass with no violations before PR submission.

### Review Process

**What happens after you submit:**

1. **CI Checks Run Automatically**
   - Test suite execution
   - Format verification (`cargo fmt --check`)
   - Lint checking (`cargo clippy`)
   - Release build verification
   - All must pass ✅

2. **Code Review by Maintainers**
   - Philosophy compliance check
   - Architecture review
   - Code quality assessment
   - Test coverage evaluation

3. **Changes Requested (if needed)**
   - Address feedback
   - Push updates to same branch
   - CI runs again automatically

4. **Approval and Merge**
   - Once approved and CI passes
   - Squash and merge to `main`

**Tips for Faster Review:**
- Keep PRs focused and reasonably sized
- Respond promptly to feedback
- Write clear commit messages
- Include context in PR description
- Link to relevant issues or discussions

---

## Testing Strategy

### Test Organization

Tests are organized following Rust conventions:

**Unit Tests** (60% of tests):
- Located in same file with `#[cfg(test)]`
- Test individual functions and methods
- Fast execution (< 1ms per test)
- No external dependencies

**Integration Tests** (30% of tests):
- Located in `crates/*/tests/` directories
- Test module interactions
- May use test fixtures or helpers
- Test complete workflows

**End-to-End Tests** (10% of tests):
- Test complete user journeys
- Verify CLI commands work correctly
- May spawn actual processes
- Slowest but most realistic

### Example Test Structure

**Unit Test:**
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_model_name() {
        let result = parse_model_name("claude-3-opus-20240229");
        assert_eq!(result.family, "claude-3");
        assert_eq!(result.variant, "opus");
    }

    #[test]
    fn test_invalid_model_name_returns_error() {
        let result = parse_model_name("invalid-model");
        assert!(result.is_err());
    }
}
```

**Integration Test:**
```rust
// tests/session_persistence.rs
use claude_core::session::{Session, SessionManager};

#[tokio::test]
async fn test_save_and_resume_session() {
    let manager = SessionManager::new("test_sessions").await.unwrap();

    // Create and save session
    let mut session = Session::new("test-123");
    session.add_message("user", "Hello");
    manager.save(&session).await.unwrap();

    // Resume session
    let restored = manager.load("test-123").await.unwrap();
    assert_eq!(restored.messages().len(), 1);
    assert_eq!(restored.messages()[0].content, "Hello");
}
```

### Running Tests

```bash
# Run all tests
cargo test

# Run tests in specific crate
cargo test -p claude-core

# Run specific test module
cargo test session_persistence

# Run specific test
cargo test test_save_and_load_checkpoint

# Run with output (see println! statements)
cargo test -- --nocapture

# Run tests and show test names
cargo test -- --nocapture --test-threads=1

# Run ignored tests (long-running or flaky)
cargo test -- --ignored
```

### Writing Good Tests

**Naming Convention:**
```rust
#[test]
fn test_<what_is_being_tested>_<expected_behavior>() {
    // Test name describes: what + expected outcome
}

// Examples:
fn test_parse_model_returns_correct_family()
fn test_invalid_input_returns_error()
fn test_session_checkpoint_preserves_all_messages()
```

**Test Structure (Arrange-Act-Assert):**
```rust
#[test]
fn test_session_resume_restores_state() {
    // Arrange - Set up test data
    let session = Session::new("test-123");
    session.add_message("user", "Hello");

    // Act - Perform the action
    let checkpoint = session.save_checkpoint().unwrap();
    let restored = Session::from_checkpoint(&checkpoint).unwrap();

    // Assert - Verify results
    assert_eq!(restored.id(), session.id());
    assert_eq!(restored.messages().len(), session.messages().len());
}
```

**Clean Up Resources:**
```rust
#[test]
fn test_temporary_file_operations() {
    let temp_dir = tempdir().unwrap();
    let file_path = temp_dir.path().join("test.txt");

    // Test operations...

    // Cleanup happens automatically when temp_dir is dropped
}
```

---

## Where to Get Help

### Documentation

- **Architecture**: See [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) - 1,810 lines of detailed system documentation
- **Hook System**: See [`docs/HOOK_LIFECYCLE_INTEGRATION.md`](docs/HOOK_LIFECYCLE_INTEGRATION.md) - Complete hook lifecycle guide
- **Philosophy**: See [`.claude/context/PHILOSOPHY.md`](.claude/context/PHILOSOPHY.md) - Development principles and decision framework
- **Patterns**: See [`.claude/context/PATTERNS.md`](.claude/context/PATTERNS.md) - Proven design patterns and solutions

### Community

- **Issues**: [GitHub Issues](https://github.com/rysweet/RustyClawd/issues) - Report bugs or request features
- **Discussions**: [GitHub Discussions](https://github.com/rysweet/RustyClawd/discussions) - Ask questions and share ideas
- **Pull Requests**: Browse [existing PRs](https://github.com/rysweet/RustyClawd/pulls) to see examples

### Getting Unstuck

**Common Issues:**

1. **Build Failures**
   - Ensure Rust is updated: `rustup update stable`
   - Check system dependencies are installed
   - Try clean build: `cargo clean && cargo build`

2. **Test Failures**
   - Run with output: `cargo test -- --nocapture`
   - Check for environment-specific issues
   - Verify test data and fixtures exist

3. **Clippy Warnings**
   - Fix automatically where possible: `cargo clippy --fix`
   - Read warning message carefully for context
   - Check PATTERNS.md for approved patterns

4. **Documentation Questions**
   - Start with ARCHITECTURE.md for system overview
   - Check module-level rustdoc comments
   - Look at existing code for examples

**Still Stuck?**
- Open an issue with detailed description
- Include error messages and context
- Tag with `help wanted` or `question`

---

## Contributing Best Practices

### Code Review Guidelines

When reviewing PRs:
- Focus on behavior and correctness
- Check philosophy compliance
- Verify tests cover new code
- Suggest simplifications where appropriate
- Be constructive and respectful

### Communication

- Be clear and concise in issues and PRs
- Provide context for decisions
- Ask questions when requirements are unclear
- Respect the philosophy and existing patterns

### Continuous Improvement

- Update documentation when you learn something new
- Add patterns to PATTERNS.md when you discover solutions
- Refactor code to improve clarity
- Help other contributors in discussions

---

## Thank You!

Arrr, thank ye for takin' the time to contribute to RustyClawd! Every contribution, whether it be code, documentation, or bug reports, helps make this project better for everyone.

Remember:
- **Keep it simple** - Simplicity is a feature
- **Write tests** - Tests are documentation
- **Ask questions** - Unclear requirements lead to wrong implementations
- **Have fun** - Enjoy the journey!

Fair winds and following seas on yer development voyage! ⚓🦀
