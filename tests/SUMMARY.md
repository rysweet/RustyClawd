# RustyClawd - Project Summary

## Overview
RustyClawd is an independent, unofficial Rust implementation of a CLI tool compatible with Claude Code. This project is not affiliated with or endorsed by Anthropic PBC, but aims to explore Rust implementations of AI-powered development tools while maintaining compatibility with the Claude ecosystem.

## Core Features

### 1. Terminal User Interface (TUI)
Interactive terminal-based sessions for engaging with Claude AI.

### 2. Hook System
Customizable workflow system that allows users to extend and modify behavior at various points in the application lifecycle.

### 3. Plugin Architecture
Extensible design that enables developers to create and integrate custom functionality.

### 4. Model Context Protocol (MCP) Support
Native integration with MCP for enhanced AI interactions and context management.

### 5. Command System
Comprehensive command and slash command system for efficient interaction.

### 6. Session Management
Robust state management for maintaining conversation history and context across sessions.

### 7. Configuration System
Hierarchical settings system for flexible configuration at multiple levels.

## Technical Stack

### Requirements
- **Rust**: Version 1.70 or higher
- **Build Tool**: Cargo

### Architecture
The project is organized into three main crates:

- **`cli`** - Command-line interface and TUI components
- **`core`** - Core client and API integration logic
- **`tools`** - Tool definitions and execution framework

## Development Workflow

### Building
```bash
cargo build --release
```

### Running
```bash
cargo run
```

### Testing
```bash
cargo test
```

### Code Quality
Pre-commit hooks are configured to ensure code quality and consistency.

## Legal & Licensing

### License
Dual-licensed under:
- MIT License
- Apache License 2.0

### Important Compliance Note
Users must comply with Anthropic's Terms of Service when using this software with Claude services.

## Project Goals
- Explore Rust implementations of AI-powered development tools
- Maintain compatibility with the Claude ecosystem
- Provide a robust, performant alternative CLI implementation
- Foster community-driven development and extensibility

---

*For more detailed information, please refer to the full README.md file.*
