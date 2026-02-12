//! Translation configuration overlay.
//!
//! Provides a full-screen UI for configuring translation settings.

use std::io::Result;

use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyEventKind;
use crossterm::event::KeyModifiers;
use ratatui::buffer::Buffer;
use ratatui::layout::Constraint;
use ratatui::layout::Layout;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::style::Modifier;
use ratatui::style::Style;
use ratatui::style::Stylize;
use ratatui::text::Line;
use ratatui::text::Span;
use ratatui::widgets::Block;
use ratatui::widgets::Borders;
use ratatui::widgets::Paragraph;
use ratatui::widgets::Widget;

use crate::translation::ProviderId;
use crate::translation::TranslationConfig;
use crate::tui;
use crate::tui::TuiEvent;

/// Supported target languages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetLanguage {
    ChineseSimplified,
    ChineseTraditional,
    Japanese,
    Korean,
    English,
    Spanish,
    French,
    German,
    Russian,
    Portuguese,
    Italian,
    Arabic,
    Hindi,
    Vietnamese,
    Thai,
}

impl TargetLanguage {
    const ALL: &'static [Self] = &[
        Self::ChineseSimplified,
        Self::ChineseTraditional,
        Self::Japanese,
        Self::Korean,
        Self::English,
        Self::Spanish,
        Self::French,
        Self::German,
        Self::Russian,
        Self::Portuguese,
        Self::Italian,
        Self::Arabic,
        Self::Hindi,
        Self::Vietnamese,
        Self::Thai,
    ];

    fn code(self) -> &'static str {
        match self {
            Self::ChineseSimplified => "zh-CN",
            Self::ChineseTraditional => "zh-TW",
            Self::Japanese => "ja",
            Self::Korean => "ko",
            Self::English => "en",
            Self::Spanish => "es",
            Self::French => "fr",
            Self::German => "de",
            Self::Russian => "ru",
            Self::Portuguese => "pt",
            Self::Italian => "it",
            Self::Arabic => "ar",
            Self::Hindi => "hi",
            Self::Vietnamese => "vi",
            Self::Thai => "th",
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::ChineseSimplified => "Chinese (Simplified)",
            Self::ChineseTraditional => "Chinese (Traditional)",
            Self::Japanese => "Japanese",
            Self::Korean => "Korean",
            Self::English => "English",
            Self::Spanish => "Spanish",
            Self::French => "French",
            Self::German => "German",
            Self::Russian => "Russian",
            Self::Portuguese => "Portuguese",
            Self::Italian => "Italian",
            Self::Arabic => "Arabic",
            Self::Hindi => "Hindi",
            Self::Vietnamese => "Vietnamese",
            Self::Thai => "Thai",
        }
    }

    #[allow(dead_code)]
    fn from_code(code: &str) -> Option<Self> {
        Self::ALL.iter().find(|l| l.code() == code).copied()
    }
}

/// Current selection in the overlay.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Selection {
    Enabled,
    Provider,
    ApiKey,
    Model,
    Language,
    BaseUrl,
    Timeout,
}

impl Selection {
    const ALL: &'static [Self] = &[
        Self::Enabled,
        Self::Provider,
        Self::ApiKey,
        Self::Model,
        Self::Language,
        Self::BaseUrl,
        Self::Timeout,
    ];

    fn next(self) -> Self {
        let idx = Self::ALL.iter().position(|s| *s == self).unwrap_or(0);
        Self::ALL[(idx + 1) % Self::ALL.len()]
    }

    fn prev(self) -> Self {
        let idx = Self::ALL.iter().position(|s| *s == self).unwrap_or(0);
        Self::ALL[(idx + Self::ALL.len() - 1) % Self::ALL.len()]
    }
}

/// Input mode for text fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InputMode {
    /// Normal navigation mode.
    Normal,
    /// Editing a text field.
    Editing,
}

/// Translation configuration overlay.
pub(crate) struct TranslateOverlay {
    /// Whether translation is enabled.
    enabled: bool,
    /// Selected provider.
    provider_id: ProviderId,
    /// Provider selection index.
    provider_index: usize,
    /// API key (stored in memory during editing).
    api_key: String,
    /// Model name override.
    model: String,
    /// Custom base URL.
    base_url: String,
    /// Timeout in milliseconds (as string for editing).
    timeout_ms: String,
    /// Selected target language.
    language: TargetLanguage,
    /// Language selection index.
    language_index: usize,
    /// Current selection.
    selection: Selection,
    /// Current input mode.
    input_mode: InputMode,
    /// Cursor position for text input.
    cursor_position: usize,
    /// Whether the overlay should close.
    is_done: bool,
    /// Status message to display.
    status_message: Option<String>,
    /// Whether config was modified.
    modified: bool,
}

impl TranslateOverlay {
    pub fn new(config: &TranslationConfig) -> Self {
        let enabled = config.enabled;

        // Find provider index
        let provider_id = config.effective_provider();
        let provider_index = ProviderId::ALL
            .iter()
            .position(|p| *p == provider_id)
            .unwrap_or(0);

        // Find language index
        let language_index = TargetLanguage::ALL
            .iter()
            .position(|l| l.code() == config.target_language)
            .unwrap_or(0);
        let language = TargetLanguage::ALL[language_index];

        let api_key = config.api_key.clone().unwrap_or_default();
        let model = config.model.clone().unwrap_or_default();
        let base_url = config.base_url.clone().unwrap_or_default();
        let timeout_ms = config
            .timeout_ms
            .map(|ms| ms.to_string())
            .unwrap_or_default();

        Self {
            enabled,
            provider_id,
            provider_index,
            api_key,
            model,
            base_url,
            timeout_ms,
            language,
            language_index,
            selection: Selection::Enabled,
            input_mode: InputMode::Normal,
            cursor_position: 0,
            is_done: false,
            status_message: None,
            modified: false,
        }
    }

    /// Get the configured translation settings.
    pub fn config(&self) -> TranslationConfig {
        TranslationConfig {
            enabled: self.enabled,
            target_language: self.language.code().to_string(),
            provider: self.provider_id.as_str().to_string(),
            api_key: if self.api_key.is_empty() {
                None
            } else {
                Some(self.api_key.clone())
            },
            model: if self.model.is_empty() {
                None
            } else {
                Some(self.model.clone())
            },
            base_url: if self.base_url.is_empty() {
                None
            } else {
                Some(self.base_url.clone())
            },
            timeout_ms: self
                .timeout_ms
                .trim()
                .parse::<u64>()
                .ok()
                .filter(|&ms| ms > 0),
        }
    }

    /// Check if the overlay should close.
    pub fn is_done(&self) -> bool {
        self.is_done
    }

    /// Check if config was modified.
    #[allow(dead_code)]
    pub fn is_modified(&self) -> bool {
        self.modified
    }

    /// Save configuration to file.
    fn save_config(&mut self) {
        let config = self.config();
        match config.save() {
            Ok(()) => {
                self.status_message = Some("Configuration saved".to_string());
            }
            Err(e) => {
                self.status_message = Some(format!("Failed to save: {e}"));
            }
        }
    }

    pub fn handle_event(&mut self, tui: &mut tui::Tui, event: TuiEvent) -> Result<()> {
        match event {
            TuiEvent::Key(key_event) => {
                self.handle_key_event(key_event)?;
                tui.frame_requester().schedule_frame();
            }
            TuiEvent::Paste(text) => {
                // Handle paste in editing mode
                if self.input_mode == InputMode::Editing {
                    self.handle_paste(&text);
                    tui.frame_requester().schedule_frame();
                }
            }
            TuiEvent::Draw => {
                tui.draw(u16::MAX, |frame| {
                    self.render(frame.area(), frame.buffer_mut());
                })?;
            }
        }
        Ok(())
    }

    fn handle_key_event(&mut self, key_event: KeyEvent) -> Result<()> {
        if key_event.kind != KeyEventKind::Press && key_event.kind != KeyEventKind::Repeat {
            return Ok(());
        }

        match self.input_mode {
            InputMode::Normal => self.handle_normal_mode(key_event),
            InputMode::Editing => self.handle_editing_mode(key_event),
        }
    }

    fn handle_normal_mode(&mut self, key_event: KeyEvent) -> Result<()> {
        match key_event.code {
            KeyCode::Esc | KeyCode::Char('q') => {
                // Close without saving; user must press 's' to save
                self.is_done = true;
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.selection = self.selection.prev();
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.selection = self.selection.next();
            }
            KeyCode::Left | KeyCode::Char('h') => {
                self.adjust_current(-1);
            }
            KeyCode::Right | KeyCode::Char('l') => {
                self.adjust_current(1);
            }
            KeyCode::Enter => {
                self.enter_edit_mode();
            }
            KeyCode::Char(' ') => {
                if self.selection == Selection::Enabled {
                    self.enabled = !self.enabled;
                    self.modified = true;
                } else {
                    self.enter_edit_mode();
                }
            }
            KeyCode::Char('s') | KeyCode::Char('S') => {
                self.save_config();
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_editing_mode(&mut self, key_event: KeyEvent) -> Result<()> {
        match key_event.code {
            KeyCode::Esc => {
                self.input_mode = InputMode::Normal;
            }
            KeyCode::Enter => {
                self.input_mode = InputMode::Normal;
                self.modified = true;
            }
            KeyCode::Char(c) => {
                // Handle paste (Ctrl+V)
                if key_event.modifiers.contains(KeyModifiers::CONTROL) && c == 'v' {
                    // Clipboard paste is handled by terminal, just accept characters
                    self.insert_char(c);
                } else {
                    self.insert_char(c);
                }
            }
            KeyCode::Backspace => {
                self.delete_char_before_cursor();
            }
            KeyCode::Delete => {
                self.delete_char_at_cursor();
            }
            KeyCode::Left => {
                self.move_cursor_left();
            }
            KeyCode::Right => {
                self.move_cursor_right();
            }
            KeyCode::Home => {
                self.cursor_position = 0;
            }
            KeyCode::End => {
                let text = self.current_text();
                self.cursor_position = text.len();
            }
            _ => {}
        }
        Ok(())
    }

    fn enter_edit_mode(&mut self) {
        match self.selection {
            Selection::ApiKey | Selection::Model | Selection::BaseUrl | Selection::Timeout => {
                self.input_mode = InputMode::Editing;
                let text = self.current_text();
                self.cursor_position = text.len();
            }
            Selection::Enabled => {
                self.enabled = !self.enabled;
                self.modified = true;
            }
            _ => {}
        }
    }

    fn current_text(&self) -> &str {
        match self.selection {
            Selection::ApiKey => &self.api_key,
            Selection::Model => &self.model,
            Selection::BaseUrl => &self.base_url,
            Selection::Timeout => &self.timeout_ms,
            _ => "",
        }
    }

    fn current_text_mut(&mut self) -> &mut String {
        match self.selection {
            Selection::ApiKey => &mut self.api_key,
            Selection::Model => &mut self.model,
            Selection::BaseUrl => &mut self.base_url,
            Selection::Timeout => &mut self.timeout_ms,
            _ => unreachable!(),
        }
    }

    fn insert_char(&mut self, c: char) {
        let pos = self.cursor_position;
        let text = self.current_text_mut();
        if pos <= text.len() {
            text.insert(pos, c);
            self.cursor_position += 1;
        }
    }

    fn handle_paste(&mut self, pasted: &str) {
        // Normalize line endings and filter control characters
        let clean: String = pasted
            .replace("\r\n", "")
            .replace('\r', "")
            .replace('\n', "")
            .chars()
            .filter(|c| !c.is_control())
            .collect();

        if clean.is_empty() {
            return;
        }

        let pos = self.cursor_position;
        let text = self.current_text_mut();
        if pos <= text.len() {
            text.insert_str(pos, &clean);
            self.cursor_position += clean.len();
            self.modified = true;
        }
    }

    fn delete_char_before_cursor(&mut self) {
        if self.cursor_position > 0 {
            let pos = self.cursor_position - 1;
            let text = self.current_text_mut();
            text.remove(pos);
            self.cursor_position -= 1;
        }
    }

    fn delete_char_at_cursor(&mut self) {
        let pos = self.cursor_position;
        let text = self.current_text_mut();
        if pos < text.len() {
            text.remove(pos);
        }
    }

    fn move_cursor_left(&mut self) {
        if self.cursor_position > 0 {
            self.cursor_position -= 1;
        }
    }

    fn move_cursor_right(&mut self) {
        let text = self.current_text();
        if self.cursor_position < text.len() {
            self.cursor_position += 1;
        }
    }

    fn adjust_current(&mut self, delta: i32) {
        match self.selection {
            Selection::Enabled => {
                self.enabled = !self.enabled;
                self.modified = true;
            }
            Selection::Provider => {
                let len = ProviderId::ALL.len();
                self.provider_index = if delta > 0 {
                    (self.provider_index + 1) % len
                } else {
                    (self.provider_index + len - 1) % len
                };
                self.provider_id = ProviderId::ALL[self.provider_index];
                self.modified = true;
            }
            Selection::Language => {
                let len = TargetLanguage::ALL.len();
                self.language_index = if delta > 0 {
                    (self.language_index + 1) % len
                } else {
                    (self.language_index + len - 1) % len
                };
                self.language = TargetLanguage::ALL[self.language_index];
                self.modified = true;
            }
            _ => {}
        }
    }

    fn render(&self, area: Rect, buf: &mut Buffer) {
        // Clear background
        for y in area.top()..area.bottom() {
            for x in area.left()..area.right() {
                buf[(x, y)].set_char(' ').set_style(Style::default());
            }
        }

        // Full-screen block
        let block = Block::default()
            .title(" Translation Settings ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan));
        let inner = block.inner(area);
        block.render(area, buf);

        // Layout with spacing for all options
        let chunks = Layout::vertical([
            Constraint::Length(1), // [0]  Top padding
            Constraint::Length(3), // [1]  Enabled toggle
            Constraint::Length(1), // [2]  Spacing
            Constraint::Length(3), // [3]  Provider
            Constraint::Length(1), // [4]  Spacing
            Constraint::Length(3), // [5]  API Key
            Constraint::Length(1), // [6]  Spacing
            Constraint::Length(3), // [7]  Model
            Constraint::Length(1), // [8]  Spacing
            Constraint::Length(3), // [9]  Language
            Constraint::Length(1), // [10] Spacing
            Constraint::Length(3), // [11] Base URL
            Constraint::Length(1), // [12] Spacing
            Constraint::Length(3), // [13] Timeout
            Constraint::Length(2), // [14] Status
            Constraint::Min(1),    // [15] Help (at bottom)
        ])
        .split(inner);

        // Enabled toggle
        self.render_toggle(
            chunks[1],
            buf,
            "Translation",
            self.enabled,
            if self.enabled {
                "Translation is enabled"
            } else {
                "Translation is disabled"
            },
            self.selection == Selection::Enabled,
        );

        // Provider selection
        let provider_def = self.provider_id.definition();
        self.render_option_with_status(
            chunks[3],
            buf,
            "Provider",
            provider_def.name,
            provider_def.description,
            self.selection == Selection::Provider,
            self.api_key_status(),
        );

        // API Key input
        self.render_text_input(
            chunks[5],
            buf,
            "API Key",
            &self.api_key,
            true, // masked
            self.selection == Selection::ApiKey,
            self.input_mode == InputMode::Editing && self.selection == Selection::ApiKey,
            "Press Enter to edit",
        );

        // Model input
        self.render_text_input(
            chunks[7],
            buf,
            "Model",
            &self.model,
            false,
            self.selection == Selection::Model,
            self.input_mode == InputMode::Editing && self.selection == Selection::Model,
            &format!("Default: {}", provider_def.default_model),
        );

        // Language selection
        self.render_option(
            chunks[9],
            buf,
            "Target Language",
            self.language.name(),
            self.language.code(),
            self.selection == Selection::Language,
        );

        // Base URL input
        self.render_text_input(
            chunks[11],
            buf,
            "Base URL",
            &self.base_url,
            false,
            self.selection == Selection::BaseUrl,
            self.input_mode == InputMode::Editing && self.selection == Selection::BaseUrl,
            &format!("Default: {}", provider_def.default_base_url),
        );

        // Timeout input
        self.render_text_input(
            chunks[13],
            buf,
            "Timeout (ms)",
            &self.timeout_ms,
            false,
            self.selection == Selection::Timeout,
            self.input_mode == InputMode::Editing && self.selection == Selection::Timeout,
            "Default: 5000",
        );

        // Status message
        if let Some(msg) = &self.status_message {
            let status = Paragraph::new(Line::from(vec![
                Span::raw("  "),
                Span::styled(msg, Style::default().fg(Color::Green)),
            ]));
            status.render(chunks[14], buf);
        }

        // Help text at bottom
        let help = if self.input_mode == InputMode::Editing {
            Paragraph::new(vec![
                Line::from(""),
                Line::from(vec![
                    Span::styled("  Enter", Style::default().bold()),
                    Span::raw(" Confirm  "),
                    Span::styled("Esc", Style::default().bold()),
                    Span::raw(" Cancel  "),
                    Span::styled("←→", Style::default().bold()),
                    Span::raw(" Move cursor"),
                ])
                .dim(),
            ])
        } else {
            Paragraph::new(vec![
                Line::from(""),
                Line::from(vec![
                    Span::styled("  ↑↓/jk", Style::default().bold()),
                    Span::raw(" Navigate  "),
                    Span::styled("←→/hl", Style::default().bold()),
                    Span::raw(" Adjust  "),
                    Span::styled("Enter", Style::default().bold()),
                    Span::raw(" Edit  "),
                    Span::styled("s", Style::default().bold()),
                    Span::raw(" Save  "),
                    Span::styled("q", Style::default().bold()),
                    Span::raw(" Close"),
                ])
                .dim(),
            ])
        };
        help.render(chunks[15], buf);
    }

    fn api_key_status(&self) -> Option<(&'static str, Color)> {
        let provider_def = self.provider_id.definition();
        if !provider_def.requires_api_key {
            Some(("○ No Key Needed", Color::Gray))
        } else if !self.api_key.is_empty() {
            Some(("✓ Key Configured", Color::Green))
        } else {
            Some(("✗ Key Required", Color::Red))
        }
    }

    fn mask_api_key(key: &str) -> String {
        if key.len() <= 8 {
            "*".repeat(key.len())
        } else {
            format!("{}...{}", &key[..4], &key[key.len().saturating_sub(4)..])
        }
    }

    fn render_toggle(
        &self,
        area: Rect,
        buf: &mut Buffer,
        label: &str,
        value: bool,
        hint: &str,
        selected: bool,
    ) {
        let style = if selected {
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };

        let indicator = if selected { "▶ " } else { "  " };
        let toggle_value = if value { "[ON]" } else { "[OFF]" };
        let toggle_color = if value { Color::Green } else { Color::Red };

        let lines = vec![
            Line::from(vec![
                Span::styled(indicator, style),
                Span::styled(format!("{label}: "), style),
                Span::styled(toggle_value, Style::default().fg(toggle_color).bold()),
            ]),
            Line::from(vec![
                Span::raw("    "),
                Span::styled(hint, Style::default().dim()),
            ]),
        ];

        for (i, line) in lines.into_iter().enumerate() {
            if area.y + (i as u16) < area.bottom() {
                buf.set_line(area.x, area.y + (i as u16), &line, area.width);
            }
        }
    }

    fn render_option(
        &self,
        area: Rect,
        buf: &mut Buffer,
        label: &str,
        value: &str,
        hint: &str,
        selected: bool,
    ) {
        self.render_option_with_status(area, buf, label, value, hint, selected, None);
    }

    #[allow(clippy::too_many_arguments)]
    fn render_option_with_status(
        &self,
        area: Rect,
        buf: &mut Buffer,
        label: &str,
        value: &str,
        hint: &str,
        selected: bool,
        status: Option<(&str, Color)>,
    ) {
        let style = if selected {
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };

        let indicator = if selected { "▶ " } else { "  " };

        let mut spans = vec![
            Span::styled(indicator, style),
            Span::styled(format!("{label}: "), style),
            Span::raw("< "),
            Span::styled(value, Style::default().fg(Color::Yellow)),
            Span::raw(" >"),
        ];

        if let Some((status_text, status_color)) = status {
            spans.push(Span::raw("  "));
            spans.push(Span::styled(
                format!("[{status_text}]"),
                Style::default().fg(status_color),
            ));
        }

        let lines = vec![
            Line::from(spans),
            Line::from(vec![
                Span::raw("    "),
                Span::styled(hint, Style::default().dim()),
            ]),
        ];

        for (i, line) in lines.into_iter().enumerate() {
            if area.y + (i as u16) < area.bottom() {
                buf.set_line(area.x, area.y + (i as u16), &line, area.width);
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn render_text_input(
        &self,
        area: Rect,
        buf: &mut Buffer,
        label: &str,
        value: &str,
        masked: bool,
        selected: bool,
        editing: bool,
        hint: &str,
    ) {
        let style = if selected {
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };

        let indicator = if selected { "▶ " } else { "  " };

        let display_value = if editing {
            // Show full value while editing
            value.to_string()
        } else if masked && !value.is_empty() {
            Self::mask_api_key(value)
        } else if value.is_empty() {
            "(not set)".to_string()
        } else {
            value.to_string()
        };

        let value_style = if value.is_empty() {
            Style::default().dim()
        } else {
            Style::default().fg(Color::Yellow)
        };

        let mut spans = vec![
            Span::styled(indicator, style),
            Span::styled(format!("{label}: "), style),
            Span::raw("["),
            Span::styled(&display_value, value_style),
        ];

        // Show cursor if editing
        if editing {
            // Add cursor indicator
            spans.push(Span::styled("▏", Style::default().fg(Color::White)));
        }

        spans.push(Span::raw("]"));

        if editing {
            spans.push(Span::raw("  "));
            spans.push(Span::styled(
                "(editing)",
                Style::default().fg(Color::Yellow),
            ));
        }

        let lines = vec![
            Line::from(spans),
            Line::from(vec![
                Span::raw("    "),
                Span::styled(hint, Style::default().dim()),
            ]),
        ];

        for (i, line) in lines.into_iter().enumerate() {
            if area.y + (i as u16) < area.bottom() {
                buf.set_line(area.x, area.y + (i as u16), &line, area.width);
            }
        }
    }
}
