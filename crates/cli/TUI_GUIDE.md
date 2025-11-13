# RustyClawd TUI Mode - Pirate Ship Edition

## Overview

The TUI (Terminal User Interface) mode provides a beautiful, interactive chat experience with Claude using ratatui. Features include:

- **Pirate Ship Banner**: ASCII art welcome screen with a sailing ship theme
- **Rust-Themed Colors**: Orange/rust color scheme throughout
- **Scrollable Messages**: View conversation history with Page Up/Down
- **Interactive Input**: Line editing with cursor support
- **Status Bar**: Shows connection status and mode

## Usage

### Starting TUI Mode

```bash
# Interactive mode with TUI
rusty --tui

# Or without TUI (classic mode)
rusty
```

### Keyboard Controls

- **Enter**: Submit message
- **Ctrl+C** or **Ctrl+D**: Exit
- **Backspace**: Delete character
- **Left/Right Arrow**: Move cursor
- **Home/End**: Jump to start/end of input
- **Page Up/Down**: Scroll messages

### Commands

- `/exit` or `/quit`: Exit the chat
- `/clear`: Clear conversation history
- `/help`: Show help message

## Features

### 1. Pirate Ship Banner
When you first start TUI mode, you'll see:
```
                    |>
                    |
                   /|\
                  / | \
                 /  |  \
                /   🦀   \
               /         \
        🌊 ~~~~~~~~~~~~~ 🌊
      🌊🌊 ~~~~~~~~~~~~~~~ 🌊🌊
```

### 2. Color Scheme
- **Rust Orange** (`#DEA584`): Borders, highlights, crab emoji
- **Rust Dark** (`#A52A2A`): Status bar background, ship mast
- **Rust Light** (`#FFC3A0`): Title text, welcome messages
- **Dark Background** (`#282832`): Message and input areas
- **Text Color** (`#E6E6E6`): Main text content

### 3. Layout

```
╔══════════════════════════════════════╗
║   🦀 RustyClawd - Rusty Edition 🦀  ║  <- Status Bar
║          ⛵ Ahoy matey! ⛵          ║
╚══════════════════════════════════════╝

╔══════════════════════════════════════╗
║ ⚓ Messages                           ║  <- Message Area
║                                      ║     (Scrollable)
║ You> Hello!                          ║
║   Hello!                             ║
║                                      ║
║ Claude> Hi there! How can I help?    ║
║   Hi there! How can I help?          ║
╚══════════════════════════════════════╝

╔══════════════════════════════════════╗
║ ✏️  Input                            ║  <- Input Area
║ You> [type here...]                  ║
╚══════════════════════════════════════╝
```

## Technical Details

### Architecture

The TUI is built with:
- **ratatui 0.29**: Terminal UI framework
- **crossterm 0.28**: Cross-platform terminal manipulation
- **Async support**: Works with tokio runtime

### Message Types

Messages are color-coded by role:
- **User**: Cyan (You>)
- **Claude**: Rust Orange (Claude>)
- **System**: Yellow (System>)

### State Management

The TUI maintains:
- Message history with scrolling
- Input buffer with cursor position
- Status information
- Banner visibility flag

## Integration

The TUI integrates seamlessly with the existing RustyClawd infrastructure:

```rust
// In main.rs
async fn run_interactive(&mut self) -> Result<()> {
    if self.cli.tui {
        tui::run_tui().await
    } else {
        interactive::run_interactive().await
    }
}
```

## Future Enhancements

Potential improvements:
- [ ] Claude API integration for real responses
- [ ] Tool use visualization
- [ ] Session persistence
- [ ] Syntax highlighting for code blocks
- [ ] Multiple conversation tabs
- [ ] Export conversation to file

## Troubleshooting

### Terminal not rendering correctly
Make sure your terminal supports:
- Unicode characters (for emojis)
- 256 colors or true color
- Terminal size at least 80x24

### Colors look wrong
Try a terminal that supports true color:
- iTerm2 (macOS)
- Windows Terminal (Windows)
- Alacritty (cross-platform)
- kitty (cross-platform)

## Credits

Built with love using Rust and inspired by the official Claude Code CLI.

Ahoy, matey! Fair winds and following seas! ⛵
