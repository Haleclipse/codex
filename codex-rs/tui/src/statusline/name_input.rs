// 名称输入对话框组件

use ratatui::buffer::Buffer;
use ratatui::layout::Constraint;
use ratatui::layout::Layout;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::style::Style;
use ratatui::widgets::Block;
use ratatui::widgets::Borders;
use ratatui::widgets::Clear;
use ratatui::widgets::Paragraph;
use ratatui::widgets::Widget;

#[derive(Debug, Clone, Default)]
pub struct NameInputDialog {
    pub is_open: bool,
    pub title: String,
    pub prompt: String,
    pub input: String,
}

impl NameInputDialog {
    pub fn open(&mut self, title: &str, prompt: &str) {
        self.is_open = true;
        self.title = title.to_string();
        self.prompt = prompt.to_string();
        self.input.clear();
    }

    pub fn close(&mut self) {
        self.is_open = false;
        self.input.clear();
    }

    pub fn input_char(&mut self, c: char) {
        if !c.is_control() && self.input.len() < 32 {
            self.input.push(c);
        }
    }

    pub fn backspace(&mut self) {
        self.input.pop();
    }

    pub fn get_input(&self) -> &str {
        &self.input
    }

    pub fn render(&self, area: Rect, buf: &mut Buffer) {
        if !self.is_open {
            return;
        }

        let popup_height = 8;
        let popup_width = 60;
        let popup_area = Rect {
            x: (area.width.saturating_sub(popup_width)) / 2,
            y: (area.height.saturating_sub(popup_height)) / 2,
            width: popup_width,
            height: popup_height,
        };

        Clear.render(popup_area, buf);

        let popup_block = Block::default()
            .borders(Borders::ALL)
            .title(self.title.as_str());
        let inner = popup_block.inner(popup_area);
        popup_block.render(popup_area, buf);

        let [input_area, help_area] =
            Layout::vertical([Constraint::Length(3), Constraint::Length(3)]).areas(inner);

        // Input
        let (input_text, input_style) = if self.input.is_empty() {
            (
                format!("> {} <", self.prompt),
                Style::default().fg(Color::DarkGray),
            )
        } else {
            (
                format!("> {} <", self.input),
                Style::default().fg(Color::Yellow),
            )
        };
        Paragraph::new(input_text)
            .style(input_style)
            .block(Block::default().borders(Borders::ALL).title("Name"))
            .render(input_area, buf);

        // Help
        Paragraph::new("[Enter] Confirm  [Esc] Cancel")
            .block(Block::default().borders(Borders::ALL))
            .render(help_area, buf);
    }
}
