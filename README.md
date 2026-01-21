# RustyClawd

A Rust implementation of a CLI tool compatible with Claude Code.

## Legal Disclaimer

**IMPORTANT**: This is an independent, unofficial open-source project with no affiliation to Anthropic PBC. Not endorsed or sponsored by Anthropic.

"Claude" and "Claude Code" are trademarks of Anthropic PBC. This project provides tools that are compatible with Claude's API services. Users must comply with [Anthropic's Terms of Service](https://www.anthropic.com/legal/consumer-terms) when using this software with Claude services.

## About

RustyClawd is a Rust-based CLI tool that provides compatibility with Claude Code functionality. This project aims to explore Rust implementations of AI-powered development tools while maintaining compatibility with the Claude ecosystem.

## Features

- Terminal UI (TUI) for interactive sessions
- Hook system for workflow customization
- Plugin architecture for extensibility
- MCP (Model Context Protocol) support
- Command and slash command system
- Session state management
- Settings hierarchy and configuration

## Getting Started

### Prerequisites

- Rust 1.70 or higher
- Cargo

### Building

```bash
cargo build --release
```

### Running

```bash
cargo run
```

## Architecture

The project is organized into three main crates:

- **cli**: Command-line interface and TUI components
- **core**: Core client and API integration
- **tools**: Tool definitions and execution

## Development

### Running Tests

```bash
cargo test
```

### Pre-commit Hooks

This project uses pre-commit hooks for code quality:

```bash
pre-commit install
pre-commit run --all-files
```

## Contributing

Contributions are welcome! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## License

This project is dual-licensed under:
- MIT License
- Apache License 2.0

See [LICENSE-MIT](LICENSE-MIT) and [LICENSE-APACHE](LICENSE-APACHE) for details.

## Acknowledgments

This project provides a Rust implementation compatible with Claude Code APIs. Anthropic PBC creates Claude Code - this is an independent community project that aims to complement the Claude ecosystem.

## Support

For issues or questions about this project, please use the [GitHub issue tracker](https://github.com/rysweet/RustyClawd/issues).

For questions about Claude or Claude Code, please refer to [Anthropic's official documentation](https://docs.anthropic.com/).
