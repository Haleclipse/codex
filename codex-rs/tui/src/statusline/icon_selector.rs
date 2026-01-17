// 图标选择器组件

use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Widget};

use super::color_picker::centered_rect;
use super::style::StyleMode;

#[derive(Debug, Clone, PartialEq)]
pub enum IconStyle {
    Plain,
    NerdFont,
}

#[derive(Debug, Clone)]
pub struct IconInfo {
    pub icon: &'static str,
    pub name: &'static str,
}

#[derive(Debug, Clone)]
pub struct IconSelector {
    pub is_open: bool,
    pub icon_style: IconStyle,
    pub selected_plain: usize,
    pub selected_nerd: usize,
    pub custom_input: String,
    pub editing_custom: bool,
    pub current_icon: Option<String>,
}

impl Default for IconSelector {
    fn default() -> Self {
        Self {
            is_open: false,
            icon_style: IconStyle::Plain,
            selected_plain: 0,
            selected_nerd: 0,
            custom_input: String::new(),
            editing_custom: false,
            current_icon: None,
        }
    }
}

impl IconSelector {
    pub fn open(&mut self, current_style: StyleMode) {
        self.is_open = true;
        self.icon_style = match current_style {
            StyleMode::Plain => IconStyle::Plain,
            StyleMode::NerdFont | StyleMode::Powerline => IconStyle::NerdFont,
        };
        self.editing_custom = false;
        self.custom_input.clear();
        self.update_current_icon();
    }

    pub fn close(&mut self) {
        self.is_open = false;
        self.editing_custom = false;
    }

    pub fn toggle_style(&mut self) {
        self.icon_style = match self.icon_style {
            IconStyle::Plain => IconStyle::NerdFont,
            IconStyle::NerdFont => IconStyle::Plain,
        };
        self.update_current_icon();
    }

    pub fn start_custom_input(&mut self) {
        self.editing_custom = true;
        self.custom_input.clear();
    }

    pub fn finish_custom_input(&mut self) -> bool {
        self.editing_custom = false;
        if !self.custom_input.is_empty() {
            self.current_icon = Some(self.custom_input.clone());
            return true;
        }
        false
    }

    pub fn input_char(&mut self, c: char) {
        if self.editing_custom {
            self.custom_input.push(c);
        }
    }

    pub fn backspace(&mut self) {
        if self.editing_custom {
            self.custom_input.pop();
        }
    }

    pub fn move_selection(&mut self, delta: i32) {
        if self.editing_custom {
            return;
        }

        match self.icon_style {
            IconStyle::Plain => {
                let icons = get_plain_icons();
                let new_selection =
                    (self.selected_plain as i32 + delta).clamp(0, icons.len() as i32 - 1) as usize;
                self.selected_plain = new_selection;
            }
            IconStyle::NerdFont => {
                let icons = get_nerd_font_icons();
                let new_selection =
                    (self.selected_nerd as i32 + delta).clamp(0, icons.len() as i32 - 1) as usize;
                self.selected_nerd = new_selection;
            }
        }
        self.update_current_icon();
    }

    fn update_current_icon(&mut self) {
        match self.icon_style {
            IconStyle::Plain => {
                let icons = get_plain_icons();
                if let Some(icon) = icons.get(self.selected_plain) {
                    self.current_icon = Some(icon.icon.to_string());
                }
            }
            IconStyle::NerdFont => {
                let icons = get_nerd_font_icons();
                if let Some(icon) = icons.get(self.selected_nerd) {
                    self.current_icon = Some(icon.icon.to_string());
                }
            }
        }
    }

    pub fn get_selected_icon(&self) -> Option<String> {
        self.current_icon.clone()
    }

    pub fn render(&self, area: Rect, buf: &mut Buffer) {
        if !self.is_open {
            return;
        }

        let popup_area = centered_rect(55, 65, area);
        Clear.render(popup_area, buf);

        let popup_block = Block::default()
            .borders(Borders::ALL)
            .title("Icon Selector");
        let inner = popup_block.inner(popup_area);
        popup_block.render(popup_area, buf);

        let [style_area, list_area, custom_area, help_area] = Layout::vertical([
            Constraint::Length(3),
            Constraint::Min(8),
            Constraint::Length(3),
            Constraint::Length(3),
        ])
        .areas(inner);

        // Style selector
        let style_text = match self.icon_style {
            IconStyle::Plain => "[•] Emoji  [ ] Nerd Font",
            IconStyle::NerdFont => "[ ] Emoji  [•] Nerd Font",
        };
        Paragraph::new(style_text)
            .block(Block::default().borders(Borders::ALL).title("Style"))
            .render(style_area, buf);

        // Icon list
        let block = Block::default()
            .borders(Borders::ALL)
            .title(match self.icon_style {
                IconStyle::Plain => "Emoji Icons",
                IconStyle::NerdFont => "Nerd Font Icons",
            });
        let list_inner = block.inner(list_area);
        block.render(list_area, buf);

        let icons = match self.icon_style {
            IconStyle::Plain => get_plain_icons(),
            IconStyle::NerdFont => get_nerd_font_icons(),
        };
        let selected = match self.icon_style {
            IconStyle::Plain => self.selected_plain,
            IconStyle::NerdFont => self.selected_nerd,
        };

        let visible_rows = list_inner.height as usize;
        let start_idx = selected.saturating_sub(visible_rows / 2);

        for (i, icon_info) in icons.iter().enumerate().skip(start_idx).take(visible_rows) {
            let y = list_inner.y + (i - start_idx) as u16;
            let is_selected = i == selected;
            let style = if is_selected {
                Style::default().add_modifier(Modifier::REVERSED)
            } else {
                Style::default()
            };
            let text = format!("{} {}", icon_info.icon, icon_info.name);
            buf.set_string(list_inner.x, y, &text, style);
        }

        // Custom input
        let custom_text = if self.editing_custom {
            format!("> {} <", self.custom_input)
        } else {
            "[c] to enter custom icon".to_string()
        };
        let custom_style = if self.editing_custom {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default()
        };
        Paragraph::new(custom_text)
            .style(custom_style)
            .block(Block::default().borders(Borders::ALL).title("Custom"))
            .render(custom_area, buf);

        // Help
        let help = if self.editing_custom {
            "[Enter] Confirm  [Esc] Cancel"
        } else {
            "[Enter] Select  [Tab] Switch  [c] Custom  [Esc] Cancel"
        };
        Paragraph::new(help)
            .block(Block::default().borders(Borders::ALL))
            .render(help_area, buf);
    }
}

pub fn get_plain_icons() -> Vec<IconInfo> {
    vec![
        IconInfo {
            icon: "🤖",
            name: "Robot (Model)",
        },
        IconInfo {
            icon: "💻",
            name: "Laptop",
        },
        IconInfo {
            icon: "📁",
            name: "Folder",
        },
        IconInfo {
            icon: "📂",
            name: "Open Folder",
        },
        IconInfo {
            icon: "📊",
            name: "Bar Chart",
        },
        IconInfo {
            icon: "🌿",
            name: "Branch (Git)",
        },
        IconInfo {
            icon: "🌱",
            name: "Seedling",
        },
        IconInfo {
            icon: "🔧",
            name: "Wrench",
        },
        IconInfo {
            icon: "⚡",
            name: "Lightning",
        },
        IconInfo {
            icon: "⭐",
            name: "Star",
        },
        IconInfo {
            icon: "✨",
            name: "Sparkles",
        },
        IconInfo {
            icon: "🔥",
            name: "Fire",
        },
        IconInfo {
            icon: "💎",
            name: "Gem",
        },
        IconInfo {
            icon: "✓",
            name: "Check",
        },
        IconInfo {
            icon: "●",
            name: "Circle",
        },
        IconInfo {
            icon: "▶",
            name: "Play",
        },
    ]
}

pub fn get_nerd_font_icons() -> Vec<IconInfo> {
    vec![
        IconInfo {
            icon: "\u{e26d}",
            name: "Robot (Model)",
        },
        IconInfo {
            icon: "\u{f02a2}",
            name: "Git Branch",
        },
        IconInfo {
            icon: "\u{f024b}",
            name: "Folder",
        },
        IconInfo {
            icon: "\u{f07b}",
            name: "Folder Open",
        },
        IconInfo {
            icon: "\u{f111}",
            name: "Circle",
        },
        IconInfo {
            icon: "\u{f135}",
            name: "Rocket",
        },
        IconInfo {
            icon: "\u{f49b}",
            name: "Chart",
        },
        IconInfo {
            icon: "\u{f0c9}",
            name: "List",
        },
        IconInfo {
            icon: "\u{f013}",
            name: "Cog",
        },
        IconInfo {
            icon: "\u{f015}",
            name: "Home",
        },
        IconInfo {
            icon: "\u{f0e7}",
            name: "Lightning",
        },
        IconInfo {
            icon: "\u{f121}",
            name: "Code",
        },
        IconInfo {
            icon: "\u{f126}",
            name: "Code Fork",
        },
        IconInfo {
            icon: "\u{f017}",
            name: "Clock",
        },
        IconInfo {
            icon: "\u{f080}",
            name: "Bar Chart",
        },
    ]
}
