// 颜色选择器组件

use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Widget};

use super::style::AnsiColor;

#[derive(Debug, Clone, PartialEq)]
pub enum ColorPickerMode {
    Basic16,
    Extended256,
    RgbInput,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RgbField {
    Red,
    Green,
    Blue,
    Hex,
}

#[derive(Debug, Clone)]
pub struct RgbInput {
    pub r: String,
    pub g: String,
    pub b: String,
    pub hex: String,
    pub editing_field: RgbField,
}

impl Default for RgbInput {
    fn default() -> Self {
        Self {
            r: String::new(),
            g: String::new(),
            b: String::new(),
            hex: String::new(),
            editing_field: RgbField::Red,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ColorTarget {
    IconColor,
    TextColor,
    BackgroundColor,
}

#[derive(Debug, Clone)]
pub struct ColorPicker {
    pub is_open: bool,
    pub mode: ColorPickerMode,
    pub selected_basic: usize,
    pub selected_extended: usize,
    pub rgb_input: RgbInput,
    pub current_color: Option<AnsiColor>,
    pub target_field: ColorTarget,
    pub cached_basic_cols: usize,
    pub cached_extended_cols: usize,
}

impl Default for ColorPicker {
    fn default() -> Self {
        Self {
            is_open: false,
            mode: ColorPickerMode::Basic16,
            selected_basic: 0,
            selected_extended: 0,
            rgb_input: RgbInput::default(),
            current_color: None,
            target_field: ColorTarget::IconColor,
            cached_basic_cols: 8,
            cached_extended_cols: 8,
        }
    }
}

impl ColorPicker {
    pub fn open(&mut self, target: ColorTarget, current: Option<AnsiColor>) {
        self.is_open = true;
        self.target_field = target;
        self.mode = ColorPickerMode::Basic16;
        self.selected_basic = 0;
        self.selected_extended = 0;
        self.rgb_input = RgbInput::default();
        self.current_color = current;
    }

    pub fn close(&mut self) {
        self.is_open = false;
    }

    pub fn cycle_mode(&mut self) {
        self.mode = match self.mode {
            ColorPickerMode::Basic16 => ColorPickerMode::Extended256,
            ColorPickerMode::Extended256 => ColorPickerMode::RgbInput,
            ColorPickerMode::RgbInput => ColorPickerMode::Basic16,
        };
    }

    pub fn move_horizontal(&mut self, delta: i32) {
        match self.mode {
            ColorPickerMode::Basic16 => {
                let current = self.selected_basic;
                let new_selection = if delta > 0 {
                    if current < 15 { current + 1 } else { 0 }
                } else if current > 0 {
                    current - 1
                } else {
                    15
                };
                self.selected_basic = new_selection;
                self.current_color = Some(AnsiColor::c16(self.selected_basic as u8));
            }
            ColorPickerMode::Extended256 => {
                let current = self.selected_extended;
                let new_selection = if delta > 0 {
                    if current < 255 { current + 1 } else { 0 }
                } else if current > 0 {
                    current - 1
                } else {
                    255
                };
                self.selected_extended = new_selection;
                self.current_color = Some(AnsiColor::c256(self.selected_extended as u8));
            }
            ColorPickerMode::RgbInput => {
                self.rgb_input.editing_field = match (&self.rgb_input.editing_field, delta > 0) {
                    (RgbField::Red, true) => RgbField::Green,
                    (RgbField::Green, true) => RgbField::Blue,
                    (RgbField::Blue, true) => RgbField::Hex,
                    (RgbField::Hex, true) => RgbField::Red,
                    (RgbField::Red, false) => RgbField::Hex,
                    (RgbField::Green, false) => RgbField::Red,
                    (RgbField::Blue, false) => RgbField::Green,
                    (RgbField::Hex, false) => RgbField::Blue,
                };
            }
        }
    }

    pub fn move_vertical(&mut self, delta: i32) {
        match self.mode {
            ColorPickerMode::Basic16 => {
                let cols = self.cached_basic_cols;
                let current_row = self.selected_basic / cols;
                let current_col = self.selected_basic % cols;
                let total_rows = 16_usize.div_ceil(cols);

                let new_row = if delta > 0 {
                    if current_row + 1 < total_rows {
                        current_row + 1
                    } else {
                        current_row
                    }
                } else if current_row > 0 {
                    current_row - 1
                } else {
                    current_row
                };

                let new_selection = (new_row * cols + current_col).min(15);
                self.selected_basic = new_selection;
                self.current_color = Some(AnsiColor::c16(self.selected_basic as u8));
            }
            ColorPickerMode::Extended256 => {
                let cols = self.cached_extended_cols;
                let current_row = self.selected_extended / cols;
                let current_col = self.selected_extended % cols;
                let total_rows = 256_usize.div_ceil(cols);

                let new_row = if delta > 0 {
                    if current_row + 1 < total_rows {
                        current_row + 1
                    } else {
                        current_row
                    }
                } else if current_row > 0 {
                    current_row - 1
                } else {
                    current_row
                };

                let new_selection = (new_row * cols + current_col).min(255);
                self.selected_extended = new_selection;
                self.current_color = Some(AnsiColor::c256(self.selected_extended as u8));
            }
            ColorPickerMode::RgbInput => {}
        }
    }

    pub fn input_char(&mut self, c: char) {
        if self.mode != ColorPickerMode::RgbInput {
            return;
        }

        match self.rgb_input.editing_field {
            RgbField::Red => {
                if self.rgb_input.r.len() < 3 && c.is_ascii_digit() {
                    self.rgb_input.r.push(c);
                }
            }
            RgbField::Green => {
                if self.rgb_input.g.len() < 3 && c.is_ascii_digit() {
                    self.rgb_input.g.push(c);
                }
            }
            RgbField::Blue => {
                if self.rgb_input.b.len() < 3 && c.is_ascii_digit() {
                    self.rgb_input.b.push(c);
                }
            }
            RgbField::Hex => {
                if self.rgb_input.hex.len() < 6 && c.is_ascii_hexdigit() {
                    self.rgb_input.hex.push(c.to_ascii_uppercase());
                }
            }
        }
        self.update_rgb_color();
    }

    pub fn backspace(&mut self) {
        if self.mode != ColorPickerMode::RgbInput {
            return;
        }

        match self.rgb_input.editing_field {
            RgbField::Red => {
                self.rgb_input.r.pop();
            }
            RgbField::Green => {
                self.rgb_input.g.pop();
            }
            RgbField::Blue => {
                self.rgb_input.b.pop();
            }
            RgbField::Hex => {
                self.rgb_input.hex.pop();
            }
        }
        self.update_rgb_color();
    }

    fn update_rgb_color(&mut self) {
        if !self.rgb_input.hex.is_empty()
            && self.rgb_input.hex.len() == 6
            && let (Ok(r), Ok(g), Ok(b)) = (
                u8::from_str_radix(&self.rgb_input.hex[0..2], 16),
                u8::from_str_radix(&self.rgb_input.hex[2..4], 16),
                u8::from_str_radix(&self.rgb_input.hex[4..6], 16),
            )
        {
            self.current_color = Some(AnsiColor::rgb(r, g, b));
            return;
        }

        if let (Ok(r), Ok(g), Ok(b)) = (
            self.rgb_input.r.parse::<u8>(),
            self.rgb_input.g.parse::<u8>(),
            self.rgb_input.b.parse::<u8>(),
        ) {
            self.current_color = Some(AnsiColor::rgb(r, g, b));
        }
    }

    pub fn get_selected_color(&self) -> Option<AnsiColor> {
        self.current_color
    }

    pub fn render(&mut self, area: Rect, buf: &mut Buffer) {
        if !self.is_open {
            return;
        }

        let popup_area = centered_rect(60, 70, area);
        Clear.render(popup_area, buf);

        let popup_block = Block::default().borders(Borders::ALL).title("Color Picker");
        let inner = popup_block.inner(popup_area);
        popup_block.render(popup_area, buf);

        let [mode_area, content_area, preview_area, help_area] = Layout::vertical([
            Constraint::Length(3),
            Constraint::Min(8),
            Constraint::Length(3),
            Constraint::Length(3),
        ])
        .areas(inner);

        // Mode selector
        let mode_text = match self.mode {
            ColorPickerMode::Basic16 => "[•] Basic (16)  [ ] Extended (256)  [ ] RGB",
            ColorPickerMode::Extended256 => "[ ] Basic (16)  [•] Extended (256)  [ ] RGB",
            ColorPickerMode::RgbInput => "[ ] Basic (16)  [ ] Extended (256)  [•] RGB",
        };
        Paragraph::new(mode_text)
            .block(Block::default().borders(Borders::ALL).title("Mode"))
            .render(mode_area, buf);

        // Content
        match self.mode {
            ColorPickerMode::Basic16 => self.render_basic_colors(content_area, buf),
            ColorPickerMode::Extended256 => self.render_extended_colors(content_area, buf),
            ColorPickerMode::RgbInput => self.render_rgb_input(content_area, buf),
        }

        // Preview
        self.render_preview(preview_area, buf);

        // Help
        Paragraph::new("[Enter] Select  [Esc] Cancel  [Tab] Cycle Mode")
            .block(Block::default().borders(Borders::ALL))
            .render(help_area, buf);
    }

    fn render_basic_colors(&mut self, area: Rect, buf: &mut Buffer) {
        let block = Block::default()
            .borders(Borders::ALL)
            .title("Basic Colors (ANSI 16)");
        let inner = block.inner(area);
        block.render(area, buf);

        let available_width = inner.width as usize;
        let available_height = inner.height as usize;

        let colors_per_row = (available_width / 6).max(1);
        self.cached_basic_cols = colors_per_row;

        for color_index in 0..16 {
            let row = color_index / colors_per_row;
            let col = color_index % colors_per_row;

            let display_row = row * 2;
            if display_row >= available_height {
                break;
            }

            let x = inner.x + (col * 6) as u16;
            let y = inner.y + display_row as u16;

            let is_selected = color_index == self.selected_basic;
            let color = ansi16_to_color(color_index as u8);

            let text = if is_selected { "[ ██ ]" } else { "  ██  " };

            buf.set_string(x, y, text, Style::default().fg(color));
        }

        let rows_needed = 16_usize.div_ceil(colors_per_row);
        let display_rows_needed = rows_needed * 2;
        if available_height > display_rows_needed && self.selected_basic < 16 {
            let name = get_color_name(self.selected_basic as u8);
            buf.set_string(
                inner.x,
                inner.y + display_rows_needed as u16,
                format!("Selected: {} ({})", self.selected_basic, name),
                Style::default().fg(Color::Gray),
            );
        }
    }

    #[allow(clippy::disallowed_methods)] // 颜色选择器需要支持 256 色
    fn render_extended_colors(&mut self, area: Rect, buf: &mut Buffer) {
        let block = Block::default()
            .borders(Borders::ALL)
            .title("Extended Colors (256)");
        let inner = block.inner(area);
        block.render(area, buf);

        let available_width = inner.width as usize;
        let available_height = inner.height as usize;

        let colors_per_row = (available_width / 7).max(1);
        self.cached_extended_cols = colors_per_row;

        let logical_rows_available = if available_height > 3 {
            (available_height - 2) / 2
        } else {
            1
        };
        let colors_per_page = colors_per_row * logical_rows_available;

        let page_index = self.selected_extended / colors_per_page;
        let start_index = page_index * colors_per_page;
        let end_index = (start_index + colors_per_page).min(256);

        for i in 0..(end_index - start_index) {
            let color_index = start_index + i;
            if color_index >= 256 {
                break;
            }

            let row = i / colors_per_row;
            let col = i % colors_per_row;

            let display_row = row * 2;
            if display_row >= available_height.saturating_sub(2) {
                break;
            }

            let x = inner.x + (col * 7) as u16;
            let y = inner.y + display_row as u16;

            let is_selected = color_index == self.selected_extended;
            let color = Color::Indexed(color_index as u8);

            let text = if is_selected { "[ ██ ]" } else { "  ██  " };

            buf.set_string(x, y, text, Style::default().fg(color));
        }

        if available_height > 2 {
            buf.set_string(
                inner.x,
                inner.y + available_height.saturating_sub(1) as u16,
                format!(
                    "Selected: {} | Use ↑↓←→ to navigate",
                    self.selected_extended
                ),
                Style::default().fg(Color::Gray),
            );
        }
    }

    fn render_rgb_input(&self, area: Rect, buf: &mut Buffer) {
        let block = Block::default().borders(Borders::ALL).title("RGB Input");
        let inner = block.inner(area);
        block.render(area, buf);

        let format_field = |field: &RgbField, value: &str, current: &RgbField| -> String {
            if field == current {
                format!("> {value} <")
            } else {
                value.to_string()
            }
        };

        let rgb_text = format!(
            "R[{}]  G[{}]  B[{}]",
            format_field(
                &RgbField::Red,
                &self.rgb_input.r,
                &self.rgb_input.editing_field
            ),
            format_field(
                &RgbField::Green,
                &self.rgb_input.g,
                &self.rgb_input.editing_field
            ),
            format_field(
                &RgbField::Blue,
                &self.rgb_input.b,
                &self.rgb_input.editing_field
            ),
        );

        buf.set_string(inner.x, inner.y, &rgb_text, Style::default());

        let hex_text = format!(
            "Hex: #{}",
            format_field(
                &RgbField::Hex,
                &self.rgb_input.hex,
                &self.rgb_input.editing_field
            ),
        );

        if inner.height > 2 {
            buf.set_string(inner.x, inner.y + 2, &hex_text, Style::default());
        }
    }

    fn render_preview(&self, area: Rect, buf: &mut Buffer) {
        let preview_text = if let Some(color) = &self.current_color {
            match color {
                AnsiColor::Color16 { c16 } => {
                    format!("████ Color 16: {} ({})", c16, get_color_name(*c16))
                }
                AnsiColor::Color256 { c256 } => format!("████ Color 256: {c256}"),
                AnsiColor::Rgb { r, g, b } => format!("████ RGB: ({r}, {g}, {b})"),
            }
        } else {
            "████ No color selected".to_string()
        };

        let color = self
            .current_color
            .map(|c| c.to_ratatui_color())
            .unwrap_or(Color::White);

        Paragraph::new(preview_text)
            .style(Style::default().fg(color))
            .block(Block::default().borders(Borders::ALL).title("Preview"))
            .render(area, buf);
    }
}

// 辅助函数

pub fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(ratatui::layout::Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

pub fn ansi16_to_color(ansi: u8) -> Color {
    match ansi {
        0 => Color::Black,
        1 => Color::Red,
        2 => Color::Green,
        3 => Color::Yellow,
        4 => Color::Blue,
        5 => Color::Magenta,
        6 => Color::Cyan,
        7 => Color::White,
        8 => Color::DarkGray,
        9 => Color::LightRed,
        10 => Color::LightGreen,
        11 => Color::LightYellow,
        12 => Color::LightBlue,
        13 => Color::LightMagenta,
        14 => Color::LightCyan,
        15 => Color::Gray,
        _ => Color::White,
    }
}

pub fn get_color_name(ansi: u8) -> &'static str {
    match ansi {
        0 => "Black",
        1 => "Red",
        2 => "Green",
        3 => "Yellow",
        4 => "Blue",
        5 => "Magenta",
        6 => "Cyan",
        7 => "White",
        8 => "DarkGray",
        9 => "LightRed",
        10 => "LightGreen",
        11 => "LightYellow",
        12 => "LightBlue",
        13 => "LightMagenta",
        14 => "LightCyan",
        15 => "Gray",
        _ => "Unknown",
    }
}
