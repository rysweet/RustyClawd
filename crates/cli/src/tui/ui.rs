//! TUI (Terminal User Interface) for RustyClawd
//!
//! A beautiful terminal interface using ratatui with:
//! - Pirate ship banner
//! - Scrollable message display area
//! - Input area with prompt
//! - Status bar
//! - Rust-colored theme (orange/rust colors)

use super::input_viewport;
use anyhow::Result;
use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyModifiers,
    },
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame, Terminal,
};
use std::io::{self, Stdout};
use unicode_segmentation::UnicodeSegmentation;

/// Rust-themed colors
const RUST_ORANGE: Color = Color::Rgb(222, 165, 132);
const RUST_DARK: Color = Color::Rgb(165, 42, 42);
const RUST_LIGHT: Color = Color::Rgb(255, 195, 160);
const RUST_BACKGROUND: Color = Color::Rgb(40, 40, 50);
const TEXT_COLOR: Color = Color::Rgb(230, 230, 230);

/// Message role
#[derive(Debug, Clone)]
pub enum MessageRole {
    User,
    Assistant,
    System,
}

/// Chat message
#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub role: MessageRole,
    pub content: String,
}

impl ChatMessage {
    pub fn user(content: String) -> Self {
        Self {
            role: MessageRole::User,
            content,
        }
    }

    pub fn assistant(content: String) -> Self {
        Self {
            role: MessageRole::Assistant,
            content,
        }
    }

    pub fn system(content: String) -> Self {
        Self {
            role: MessageRole::System,
            content,
        }
    }
}

/// TUI state
pub struct TuiState {
    /// Terminal instance
    terminal: Terminal<CrosstermBackend<Stdout>>,
    /// Chat messages
    messages: Vec<ChatMessage>,
    /// Current input
    input: String,
    /// Input cursor position
    cursor_position: usize,
    /// Scroll offset for messages
    scroll_offset: usize,
    /// Status message
    status: String,
    /// Whether to show the pirate banner
    show_banner: bool,
}

impl TuiState {
    /// Create a new TUI state
    pub fn new() -> Result<Self> {
        // Setup terminal
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)?;

        Ok(Self {
            terminal,
            messages: vec![],
            input: String::new(),
            cursor_position: 0,
            scroll_offset: 0,
            status: "Ready".to_string(),
            show_banner: true,
        })
    }

    /// Add a message to the chat
    pub fn add_message(&mut self, message: ChatMessage) {
        self.messages.push(message);
        // Auto-scroll to bottom
        self.scroll_to_bottom();
    }

    /// Set status message
    pub fn set_status(&mut self, status: String) {
        self.status = status;
    }

    /// Scroll to bottom of messages
    fn scroll_to_bottom(&mut self) {
        if self.messages.len() > 0 {
            self.scroll_offset = self.messages.len().saturating_sub(1);
        }
    }

    /// Handle keyboard input
    pub fn handle_input(&mut self) -> Result<Option<String>> {
        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                return Ok(self.handle_key_event(key));
            }
        }
        Ok(None)
    }

    /// Handle key events with Unicode-aware cursor positioning
    fn handle_key_event(&mut self, key: KeyEvent) -> Option<String> {
        match (key.code, key.modifiers) {
            // Ctrl+C - Exit
            (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
                return Some("/exit".to_string());
            }
            // Ctrl+D - Exit
            (KeyCode::Char('d'), KeyModifiers::CONTROL) => {
                return Some("/exit".to_string());
            }
            // Enter - Submit input
            (KeyCode::Enter, _) => {
                if !self.input.is_empty() {
                    let input = self.input.clone();
                    self.input.clear();
                    self.cursor_position = 0;
                    return Some(input);
                }
            }
            // Backspace - Delete character before cursor
            (KeyCode::Backspace, _) => {
                if self.cursor_position > 0 {
                    // Convert grapheme position to byte position
                    let graphemes: Vec<&str> = self.input.graphemes(true).collect();
                    if self.cursor_position <= graphemes.len() {
                        let byte_pos: usize = graphemes
                            .iter()
                            .take(self.cursor_position - 1)
                            .map(|g| g.len())
                            .sum();
                        let grapheme_len = graphemes[self.cursor_position - 1].len();
                        self.input
                            .replace_range(byte_pos..byte_pos + grapheme_len, "");
                        self.cursor_position -= 1;
                    }
                }
            }
            // Left arrow - Move cursor left
            (KeyCode::Left, _) => {
                if self.cursor_position > 0 {
                    self.cursor_position -= 1;
                }
            }
            // Right arrow - Move cursor right
            (KeyCode::Right, _) => {
                let text_len = self.input.graphemes(true).count();
                if self.cursor_position < text_len {
                    self.cursor_position += 1;
                }
            }
            // Home - Move to start
            (KeyCode::Home, _) => {
                self.cursor_position = 0;
            }
            // End - Move to end
            (KeyCode::End, _) => {
                self.cursor_position = self.input.graphemes(true).count();
            }
            // Page Up - Scroll messages up
            (KeyCode::PageUp, _) => {
                self.scroll_offset = self.scroll_offset.saturating_sub(5);
            }
            // Page Down - Scroll messages down
            (KeyCode::PageDown, _) => {
                self.scroll_offset =
                    (self.scroll_offset + 5).min(self.messages.len().saturating_sub(1));
            }
            // Character input - Insert at cursor position
            (KeyCode::Char(c), _) => {
                // Convert grapheme position to byte position
                let graphemes: Vec<&str> = self.input.graphemes(true).collect();
                if self.cursor_position <= graphemes.len() {
                    let byte_pos: usize = graphemes
                        .iter()
                        .take(self.cursor_position)
                        .map(|g| g.len())
                        .sum();
                    self.input.insert(byte_pos, c);
                    self.cursor_position += 1;
                }
            }
            _ => {}
        }
        None
    }

    /// Draw the TUI
    pub fn draw(&mut self) -> Result<()> {
        // Clone data needed for rendering to avoid borrowing issues
        let messages = self.messages.clone();
        let input = self.input.clone();
        let cursor_position = self.cursor_position;
        let scroll_offset = self.scroll_offset;
        let status = self.status.clone();
        let show_banner = self.show_banner;

        self.terminal.draw(|f| {
            Self::render_ui(
                f,
                &messages,
                &input,
                cursor_position,
                scroll_offset,
                &status,
                show_banner,
            );
        })?;
        Ok(())
    }

    /// Render the UI
    fn render_ui(
        f: &mut Frame,
        messages: &[ChatMessage],
        input: &str,
        cursor_position: usize,
        scroll_offset: usize,
        _status: &str,
        show_banner: bool,
    ) {
        let size = f.area();

        // Create main layout
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Status bar
                Constraint::Min(0),    // Messages area
                Constraint::Length(3), // Input area
            ])
            .split(size);

        // Render status bar
        Self::render_status_bar(f, chunks[0]);

        // Render messages area (with optional banner)
        Self::render_messages_area(f, chunks[1], messages, scroll_offset, show_banner);

        // Render input area
        Self::render_input_area(f, chunks[2], input, cursor_position);
    }

    /// Render status bar
    fn render_status_bar(f: &mut Frame, area: Rect) {
        let banner = vec![Line::from(vec![
            Span::styled(" 🦀 ", Style::default().fg(RUST_ORANGE)),
            Span::styled(
                "RustyClawd",
                Style::default()
                    .fg(RUST_ORANGE)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" - Rusty Edition ", Style::default().fg(RUST_LIGHT)),
            Span::styled("⛵", Style::default().fg(RUST_ORANGE)),
            Span::styled(
                " Ahoy matey! ",
                Style::default()
                    .fg(RUST_LIGHT)
                    .add_modifier(Modifier::ITALIC),
            ),
        ])];

        let status = Paragraph::new(banner)
            .style(Style::default().bg(RUST_DARK))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(RUST_ORANGE)),
            )
            .alignment(Alignment::Center);

        f.render_widget(status, area);
    }

    /// Render messages area
    fn render_messages_area(
        f: &mut Frame,
        area: Rect,
        messages: &[ChatMessage],
        scroll_offset: usize,
        show_banner: bool,
    ) {
        let mut lines = Vec::new();

        // Add pirate ship banner if enabled
        if show_banner && messages.is_empty() {
            lines.extend(Self::render_pirate_ship());
            lines.push(Line::from(""));
            lines.push(Line::from(vec![
                Span::styled(
                    "Welcome aboard! ",
                    Style::default().fg(RUST_LIGHT).add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    "Type your message below and press Enter to chat with Claude.",
                    Style::default().fg(TEXT_COLOR),
                ),
            ]));
            lines.push(Line::from(vec![
                Span::styled(
                    "Commands: ",
                    Style::default()
                        .fg(RUST_ORANGE)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled("/exit, /clear, /help", Style::default().fg(TEXT_COLOR)),
            ]));
        }

        // Add messages
        for message in messages {
            lines.push(Line::from(""));
            lines.extend(Self::format_message(message));
        }

        let messages_widget = Paragraph::new(lines)
            .style(Style::default().fg(TEXT_COLOR).bg(RUST_BACKGROUND))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(RUST_ORANGE))
                    .title(vec![
                        Span::styled("⚓ ", Style::default().fg(RUST_ORANGE)),
                        Span::styled(
                            "Messages",
                            Style::default().fg(RUST_LIGHT).add_modifier(Modifier::BOLD),
                        ),
                    ]),
            )
            .wrap(Wrap { trim: false })
            .scroll((scroll_offset as u16, 0));

        f.render_widget(messages_widget, area);
    }

    /// Render pirate ship ASCII art
    fn render_pirate_ship() -> Vec<Line<'static>> {
        vec![
            Line::from(vec![Span::styled(
                "                    |>",
                Style::default().fg(RUST_ORANGE),
            )]),
            Line::from(vec![Span::styled(
                "                    |",
                Style::default().fg(RUST_DARK),
            )]),
            Line::from(vec![Span::styled(
                "                   /|\\",
                Style::default().fg(RUST_DARK),
            )]),
            Line::from(vec![Span::styled(
                "                  / | \\",
                Style::default().fg(RUST_DARK),
            )]),
            Line::from(vec![Span::styled(
                "                 /  |  \\",
                Style::default().fg(RUST_DARK),
            )]),
            Line::from(vec![
                Span::styled("                /   ", Style::default().fg(RUST_DARK)),
                Span::styled("🦀", Style::default().fg(RUST_ORANGE)),
                Span::styled("   \\", Style::default().fg(RUST_DARK)),
            ]),
            Line::from(vec![Span::styled(
                "               /         \\",
                Style::default().fg(RUST_DARK),
            )]),
            Line::from(vec![
                Span::styled("        ", Style::default()),
                Span::styled("🌊", Style::default().fg(Color::Cyan)),
                Span::styled(" ~~~~~~~~~~~~~ ", Style::default().fg(Color::Blue)),
                Span::styled("🌊", Style::default().fg(Color::Cyan)),
            ]),
            Line::from(vec![
                Span::styled("      ", Style::default()),
                Span::styled("🌊🌊", Style::default().fg(Color::Cyan)),
                Span::styled(" ~~~~~~~~~~~~~~~ ", Style::default().fg(Color::Blue)),
                Span::styled("🌊🌊", Style::default().fg(Color::Cyan)),
            ]),
        ]
    }

    /// Format a message for display
    fn format_message(message: &ChatMessage) -> Vec<Line<'_>> {
        let (prefix, style) = match message.role {
            MessageRole::User => (
                "You",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            MessageRole::Assistant => (
                "Claude",
                Style::default()
                    .fg(RUST_ORANGE)
                    .add_modifier(Modifier::BOLD),
            ),
            MessageRole::System => (
                "System",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
        };

        let mut lines = Vec::new();

        // Add role prefix
        lines.push(Line::from(vec![Span::styled(
            format!("{}> ", prefix),
            style,
        )]));

        // Add message content (word-wrapped)
        for line in message.content.lines() {
            lines.push(Line::from(vec![Span::styled(
                format!("  {}", line),
                Style::default().fg(TEXT_COLOR),
            )]));
        }

        lines
    }

    /// Render input area with horizontal scrolling support
    fn render_input_area(f: &mut Frame, area: Rect, input: &str, cursor_position: usize) {
        // Calculate available width for text (subtract borders and prompt)
        // Border: 2 (left + right), Prompt: 5 ("You> ")
        let prompt_text = "You> ";
        let prompt_width = 5u16;
        let borders_width = 2u16;
        let available_width = area.width.saturating_sub(borders_width + prompt_width) as usize;

        // Calculate viewport
        let viewport = input_viewport::calculate_viewport(input, cursor_position, available_width);

        // Create input text with prompt and visible portion
        let input_text = vec![Line::from(vec![
            Span::styled(
                prompt_text,
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(&viewport.visible_text, Style::default().fg(TEXT_COLOR)),
        ])];

        let input_widget = Paragraph::new(input_text)
            .style(Style::default().bg(RUST_BACKGROUND))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(RUST_ORANGE))
                    .title(vec![
                        Span::styled("✏️  ", Style::default().fg(RUST_ORANGE)),
                        Span::styled(
                            "Input",
                            Style::default().fg(RUST_LIGHT).add_modifier(Modifier::BOLD),
                        ),
                    ]),
            );

        f.render_widget(input_widget, area);

        // Calculate cursor position with bounds checking
        let (cursor_x, cursor_y) = input_viewport::calculate_cursor_coords(
            &viewport,
            prompt_width + 1, // +1 for left border
            area.x,
            area.y + 1, // +1 for top border
        );

        // Ensure cursor stays within bounds
        let max_x = area.x + area.width.saturating_sub(1);
        let max_y = area.y + area.height.saturating_sub(1);
        let bounded_x = cursor_x.min(max_x);
        let bounded_y = cursor_y.min(max_y);

        f.set_cursor_position((bounded_x, bounded_y));
    }

    /// Cleanup terminal on drop
    pub fn cleanup(&mut self) -> Result<()> {
        disable_raw_mode()?;
        execute!(
            self.terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )?;
        self.terminal.show_cursor()?;
        Ok(())
    }
}

impl Drop for TuiState {
    fn drop(&mut self) {
        // Best effort cleanup
        let _ = self.cleanup();
    }
}

/// Run the TUI mode
pub async fn run_tui() -> Result<()> {
    let mut tui = TuiState::new()?;

    // Add welcome message
    tui.add_message(ChatMessage::system(
        "Welcome to RustyClawd! Type your message and press Enter.".to_string(),
    ));

    loop {
        // Draw UI
        tui.draw()?;

        // Handle input
        if let Some(input) = tui.handle_input()? {
            let input = input.trim();

            // Handle exit
            if input == "/exit" || input == "/quit" {
                break;
            }

            // Handle clear
            if input == "/clear" {
                tui.messages.clear();
                tui.add_message(ChatMessage::system("Conversation cleared.".to_string()));
                continue;
            }

            // Handle help
            if input == "/help" {
                tui.add_message(ChatMessage::system(
                    "Commands: /exit, /quit, /clear, /help\nPress Ctrl+C or Ctrl+D to exit."
                        .to_string(),
                ));
                continue;
            }

            // Add user message
            if !input.is_empty() {
                tui.add_message(ChatMessage::user(input.to_string()));

                tui.add_message(ChatMessage::system(
                    "Error: TUI mode requires Claude API integration. Use CLI mode instead."
                        .to_string(),
                ));
            }
        }
    }

    tui.cleanup()?;
    println!("Goodbye, matey! Fair winds and following seas! ⛵");

    Ok(())
}
