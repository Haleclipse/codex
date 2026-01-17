// CxLine 配置 Overlay
// 在主 TUI 的 Overlay 层中运行，不创建独立的 Terminal
// 参考 CCometixLine 的 UI 设计

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
use ratatui::widgets::List;
use ratatui::widgets::ListItem;
use ratatui::widgets::Paragraph;
use ratatui::widgets::Widget;

use crate::statusline::ColorPicker;
use crate::statusline::ColorTarget;
use crate::statusline::IconSelector;
use crate::statusline::NameInputDialog;
use crate::statusline::SeparatorEditor;
use crate::statusline::StatusLineContext;
use crate::statusline::config::CxLineConfig;
use crate::statusline::segment::SegmentId;
use crate::statusline::style::AnsiColor;
use crate::statusline::style::StyleMode;
use crate::statusline::themes::THEME_NAMES;
use crate::tui;
use crate::tui::TuiEvent;

/// 当前选中的面板
#[derive(Debug, Clone, PartialEq)]
enum Panel {
    SegmentList,
    Settings,
}

/// Settings 面板中的字段
#[derive(Debug, Clone, PartialEq)]
enum FieldSelection {
    Enabled,
    Icon,
    IconColor,
    TextColor,
    BackgroundColor,
    TextStyle,
    Options,
}

const FIELD_COUNT: usize = 7;

/// CxLine 配置 Overlay
pub(crate) struct CxlineOverlay {
    config: CxLineConfig,
    /// 原始配置（用于保存逻辑）
    original_config: CxLineConfig,
    /// 进入时的主题名称（用于判断主题是否变化）
    original_theme: String,
    /// Segment 显示顺序
    segment_order: Vec<SegmentId>,
    selected_segment: usize,
    selected_panel: Panel,
    selected_field: FieldSelection,
    is_done: bool,
    status_message: Option<String>,
    // 对话框组件
    color_picker: ColorPicker,
    icon_selector: IconSelector,
    separator_editor: SeparatorEditor,
    name_input_dialog: NameInputDialog,
}

impl CxlineOverlay {
    pub fn new(config: CxLineConfig) -> Self {
        let original_theme = config.theme.clone();
        let original_config = config.clone();
        Self {
            config,
            original_config,
            original_theme,
            segment_order: vec![
                SegmentId::Model,
                SegmentId::Directory,
                SegmentId::Git,
                SegmentId::Context,
                SegmentId::Usage,
            ],
            selected_segment: 0,
            selected_panel: Panel::SegmentList,
            selected_field: FieldSelection::Enabled,
            is_done: false,
            status_message: None,
            color_picker: ColorPicker::default(),
            icon_selector: IconSelector::default(),
            separator_editor: SeparatorEditor::default(),
            name_input_dialog: NameInputDialog::default(),
        }
    }

    /// 获取最终配置（只包含主题切换，如果主题真的变化了）
    pub fn config(&self) -> CxLineConfig {
        // 只有主题变化时才返回新配置，否则返回原始配置
        if self.config.theme != self.original_theme {
            // 创建一个新配置，应用新主题到原始配置
            let mut result = self.original_config.clone();
            result.apply_theme(&self.config.theme);
            result
        } else {
            self.original_config.clone()
        }
    }

    pub fn handle_event(&mut self, tui: &mut tui::Tui, event: TuiEvent) -> Result<()> {
        match event {
            TuiEvent::Key(key_event) => {
                self.handle_key_event(key_event)?;
                tui.frame_requester().schedule_frame();
                Ok(())
            }
            TuiEvent::Draw => {
                tui.draw(u16::MAX, |frame| {
                    self.render(frame.area(), frame.buffer_mut());
                })?;
                Ok(())
            }
            _ => Ok(()),
        }
    }

    fn handle_key_event(&mut self, key_event: KeyEvent) -> Result<()> {
        if key_event.kind != KeyEventKind::Press && key_event.kind != KeyEventKind::Repeat {
            return Ok(());
        }

        // 优先处理对话框事件
        if self.color_picker.is_open {
            return self.handle_color_picker_key(key_event);
        }
        if self.icon_selector.is_open {
            return self.handle_icon_selector_key(key_event);
        }
        if self.separator_editor.is_open {
            return self.handle_separator_editor_key(key_event);
        }
        if self.name_input_dialog.is_open {
            return self.handle_name_input_key(key_event);
        }

        // Ctrl+S: 保存为新主题
        if key_event.modifiers.contains(KeyModifiers::CONTROL)
            && let KeyCode::Char('s') = key_event.code
        {
            self.name_input_dialog
                .open("Save as New Theme", "Enter theme name:");
            return Ok(());
        }

        // Shift+↑↓ 用于 Segment 排序
        if key_event.modifiers.contains(KeyModifiers::SHIFT) {
            match key_event.code {
                KeyCode::Up => {
                    self.move_segment_up();
                    return Ok(());
                }
                KeyCode::Down => {
                    self.move_segment_down();
                    return Ok(());
                }
                _ => {}
            }
        }

        match key_event.code {
            KeyCode::Esc | KeyCode::Char('q') => {
                self.is_done = true;
            }
            KeyCode::Up | KeyCode::Char('k') => self.move_selection(-1),
            KeyCode::Down | KeyCode::Char('j') => self.move_selection(1),
            KeyCode::Tab => self.switch_panel(),
            KeyCode::Enter | KeyCode::Char(' ') => self.toggle_current(),
            KeyCode::Left | KeyCode::Char('h') => self.adjust_current(-1),
            KeyCode::Right | KeyCode::Char('l') => self.adjust_current(1),
            KeyCode::Char('p') | KeyCode::Char('P') => self.cycle_theme(),
            KeyCode::Char('r') | KeyCode::Char('R') => self.reset_theme(),
            KeyCode::Char('w') | KeyCode::Char('W') => self.write_to_current_theme(),
            KeyCode::Char('s') | KeyCode::Char('S') => self.save_config(),
            KeyCode::Char('e') | KeyCode::Char('E') => self.open_separator_editor(),
            KeyCode::Char('1') => self.switch_to_theme(0),
            KeyCode::Char('2') => self.switch_to_theme(1),
            KeyCode::Char('3') => self.switch_to_theme(2),
            KeyCode::Char('4') => self.switch_to_theme(3),
            KeyCode::Char('5') => self.switch_to_theme(4),
            KeyCode::Char('6') => self.switch_to_theme(5),
            KeyCode::Char('7') => self.switch_to_theme(6),
            KeyCode::Char('8') => self.switch_to_theme(7),
            KeyCode::Char('9') => self.switch_to_theme(8),
            _ => {}
        }
        Ok(())
    }

    fn handle_color_picker_key(&mut self, key_event: KeyEvent) -> Result<()> {
        match key_event.code {
            KeyCode::Esc => {
                self.color_picker.close();
            }
            KeyCode::Enter => {
                if let Some(color) = self.color_picker.get_selected_color() {
                    self.apply_color(color);
                }
                self.color_picker.close();
            }
            KeyCode::Tab => {
                self.color_picker.cycle_mode();
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.color_picker.move_vertical(-1);
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.color_picker.move_vertical(1);
            }
            KeyCode::Left | KeyCode::Char('h') => {
                self.color_picker.move_horizontal(-1);
            }
            KeyCode::Right | KeyCode::Char('l') => {
                self.color_picker.move_horizontal(1);
            }
            KeyCode::Backspace => {
                self.color_picker.backspace();
            }
            KeyCode::Char(c) => {
                self.color_picker.input_char(c);
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_icon_selector_key(&mut self, key_event: KeyEvent) -> Result<()> {
        if self.icon_selector.editing_custom {
            match key_event.code {
                KeyCode::Esc => {
                    self.icon_selector.editing_custom = false;
                }
                KeyCode::Enter => {
                    if self.icon_selector.finish_custom_input() {
                        if let Some(icon) = self.icon_selector.get_selected_icon() {
                            self.apply_icon(icon);
                        }
                        self.icon_selector.close();
                    }
                }
                KeyCode::Backspace => {
                    self.icon_selector.backspace();
                }
                KeyCode::Char(c) => {
                    self.icon_selector.input_char(c);
                }
                _ => {}
            }
        } else {
            match key_event.code {
                KeyCode::Esc => {
                    self.icon_selector.close();
                }
                KeyCode::Enter => {
                    if let Some(icon) = self.icon_selector.get_selected_icon() {
                        self.apply_icon(icon);
                    }
                    self.icon_selector.close();
                }
                KeyCode::Tab => {
                    self.icon_selector.toggle_style();
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    self.icon_selector.move_selection(-1);
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    self.icon_selector.move_selection(1);
                }
                KeyCode::Char('c') | KeyCode::Char('C') => {
                    self.icon_selector.start_custom_input();
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn handle_separator_editor_key(&mut self, key_event: KeyEvent) -> Result<()> {
        match key_event.code {
            KeyCode::Esc => {
                self.separator_editor.close();
            }
            KeyCode::Enter => {
                let separator = self.separator_editor.get_separator();
                self.config.separator = separator;
                self.status_message = Some("Separator updated".to_string());
                self.separator_editor.close();
            }
            KeyCode::Tab => {
                self.separator_editor.clear_input();
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.separator_editor.move_preset_selection(-1);
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.separator_editor.move_preset_selection(1);
            }
            KeyCode::Backspace => {
                self.separator_editor.backspace();
            }
            KeyCode::Char(c) => {
                self.separator_editor.input_char(c);
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_name_input_key(&mut self, key_event: KeyEvent) -> Result<()> {
        match key_event.code {
            KeyCode::Esc => {
                self.name_input_dialog.close();
            }
            KeyCode::Enter => {
                let name = self.name_input_dialog.get_input().to_string();
                if !name.is_empty() {
                    self.save_as_new_theme(&name);
                }
                self.name_input_dialog.close();
            }
            KeyCode::Backspace => {
                self.name_input_dialog.backspace();
            }
            KeyCode::Char(c) => {
                self.name_input_dialog.input_char(c);
            }
            _ => {}
        }
        Ok(())
    }

    fn write_to_current_theme(&mut self) {
        use crate::statusline::themes::ThemePresets;

        let current_theme = self.config.theme.clone();
        match ThemePresets::save_theme(&current_theme, &self.config) {
            Ok(_) => {
                self.status_message = Some(format!("Wrote config to theme: {current_theme}"));
            }
            Err(e) => {
                self.status_message = Some(format!("Failed to write theme: {e}"));
            }
        }
    }

    fn save_as_new_theme(&mut self, theme_name: &str) {
        use crate::statusline::themes::ThemePresets;

        let mut new_config = self.config.clone();
        new_config.theme = theme_name.to_string();

        match ThemePresets::save_theme(theme_name, &new_config) {
            Ok(_) => {
                self.config.theme = theme_name.to_string();
                self.status_message = Some(format!("Saved as new theme: {theme_name}"));
            }
            Err(e) => {
                self.status_message = Some(format!("Failed to save theme: {e}"));
            }
        }
    }

    fn apply_color(&mut self, color: AnsiColor) {
        let id = self.segment_id_at(self.selected_segment);
        let segment_config = self.config.get_segment_config_mut(id);

        match self.color_picker.target_field {
            ColorTarget::IconColor => {
                segment_config.colors.icon = Some(color);
                self.status_message = Some("Icon color updated".to_string());
            }
            ColorTarget::TextColor => {
                segment_config.colors.text = Some(color);
                self.status_message = Some("Text color updated".to_string());
            }
            ColorTarget::BackgroundColor => {
                segment_config.colors.background = Some(color);
                self.status_message = Some("Background color updated".to_string());
            }
        }
    }

    fn apply_icon(&mut self, icon: String) {
        let id = self.segment_id_at(self.selected_segment);
        let style = self.config.style;
        let segment_config = self.config.get_segment_config_mut(id);

        match style {
            StyleMode::Plain => {
                segment_config.icon.plain = icon;
            }
            StyleMode::NerdFont | StyleMode::Powerline => {
                segment_config.icon.nerd_font = icon;
            }
        }
        self.status_message = Some("Icon updated".to_string());
    }

    fn open_separator_editor(&mut self) {
        self.separator_editor.open(&self.config.separator);
    }

    pub fn is_done(&self) -> bool {
        self.is_done
    }

    fn segment_count(&self) -> usize {
        self.segment_order.len()
    }

    fn segment_id_at(&self, index: usize) -> SegmentId {
        self.segment_order
            .get(index)
            .copied()
            .unwrap_or(SegmentId::Model)
    }

    fn segment_name(id: SegmentId) -> &'static str {
        match id {
            SegmentId::Model => "Model",
            SegmentId::Directory => "Directory",
            SegmentId::Git => "Git",
            SegmentId::Context => "Context Window",
            SegmentId::Usage => "Usage",
        }
    }

    fn move_selection(&mut self, delta: i32) {
        match self.selected_panel {
            Panel::SegmentList => {
                let new_selection = (self.selected_segment as i32 + delta)
                    .max(0)
                    .min((self.segment_count() - 1) as i32)
                    as usize;
                self.selected_segment = new_selection;
            }
            Panel::Settings => {
                let current_field = self.field_index();
                let new_field = (current_field as i32 + delta).clamp(0, FIELD_COUNT as i32 - 1);
                self.selected_field = self.index_to_field(new_field as usize);
            }
        }
    }

    fn field_index(&self) -> usize {
        match self.selected_field {
            FieldSelection::Enabled => 0,
            FieldSelection::Icon => 1,
            FieldSelection::IconColor => 2,
            FieldSelection::TextColor => 3,
            FieldSelection::BackgroundColor => 4,
            FieldSelection::TextStyle => 5,
            FieldSelection::Options => 6,
        }
    }

    fn index_to_field(&self, index: usize) -> FieldSelection {
        match index {
            0 => FieldSelection::Enabled,
            1 => FieldSelection::Icon,
            2 => FieldSelection::IconColor,
            3 => FieldSelection::TextColor,
            4 => FieldSelection::BackgroundColor,
            5 => FieldSelection::TextStyle,
            6 => FieldSelection::Options,
            _ => FieldSelection::Enabled,
        }
    }

    fn switch_panel(&mut self) {
        self.selected_panel = match self.selected_panel {
            Panel::SegmentList => Panel::Settings,
            Panel::Settings => Panel::SegmentList,
        };
    }

    fn move_segment_up(&mut self) {
        if self.selected_panel == Panel::SegmentList && self.selected_segment > 0 {
            self.segment_order
                .swap(self.selected_segment, self.selected_segment - 1);
            self.selected_segment -= 1;
            self.status_message = Some("Segment moved up".to_string());
        }
    }

    fn move_segment_down(&mut self) {
        if self.selected_panel == Panel::SegmentList
            && self.selected_segment < self.segment_count() - 1
        {
            self.segment_order
                .swap(self.selected_segment, self.selected_segment + 1);
            self.selected_segment += 1;
            self.status_message = Some("Segment moved down".to_string());
        }
    }

    fn reset_theme(&mut self) {
        self.config.apply_theme(&self.original_theme);
        self.status_message = Some(format!("Reset to: {}", self.original_theme));
    }

    fn toggle_current(&mut self) {
        match self.selected_panel {
            Panel::SegmentList => {
                let id = self.segment_id_at(self.selected_segment);
                let name = Self::segment_name(id);
                let segment_config = self.config.get_segment_config_mut(id);
                segment_config.enabled = !segment_config.enabled;
                let enabled = segment_config.enabled;
                self.status_message = Some(format!(
                    "{} {}",
                    name,
                    if enabled { "enabled" } else { "disabled" }
                ));
            }
            Panel::Settings => {
                self.adjust_current(1);
            }
        }
    }

    fn adjust_current(&mut self, _delta: i32) {
        if self.selected_panel != Panel::Settings {
            return;
        }

        let id = self.segment_id_at(self.selected_segment);
        let name = Self::segment_name(id);

        match self.selected_field {
            FieldSelection::Enabled => {
                let segment_config = self.config.get_segment_config_mut(id);
                segment_config.enabled = !segment_config.enabled;
                let enabled = segment_config.enabled;
                self.status_message = Some(format!(
                    "{} {}",
                    name,
                    if enabled { "enabled" } else { "disabled" }
                ));
            }
            FieldSelection::Icon => {
                let style = self.config.style;
                self.icon_selector.open(style);
            }
            FieldSelection::IconColor => {
                let current_color = self.config.get_segment_config(id).colors.icon;
                self.color_picker
                    .open(ColorTarget::IconColor, current_color);
            }
            FieldSelection::TextColor => {
                let current_color = self.config.get_segment_config(id).colors.text;
                self.color_picker
                    .open(ColorTarget::TextColor, current_color);
            }
            FieldSelection::BackgroundColor => {
                let current_color = self.config.get_segment_config(id).colors.background;
                self.color_picker
                    .open(ColorTarget::BackgroundColor, current_color);
            }
            FieldSelection::TextStyle => {
                let segment_config = self.config.get_segment_config_mut(id);
                segment_config.styles.text_bold = !segment_config.styles.text_bold;
                let bold = segment_config.styles.text_bold;
                self.status_message = Some(format!(
                    "{} bold {}",
                    name,
                    if bold { "enabled" } else { "disabled" }
                ));
            }
            FieldSelection::Options => {
                self.status_message = Some("Options editing not yet supported".to_string());
            }
        }
    }

    fn cycle_theme(&mut self) {
        let current_idx = THEME_NAMES
            .iter()
            .position(|&t| t == self.config.theme)
            .unwrap_or(0);
        let new_idx = (current_idx + 1) % THEME_NAMES.len();
        let new_theme = THEME_NAMES[new_idx];
        self.config.apply_theme(new_theme);
        self.status_message = Some(format!("Theme: {new_theme}"));
    }

    fn switch_to_theme(&mut self, index: usize) {
        if index < THEME_NAMES.len() {
            let theme_name = THEME_NAMES[index];
            self.config.apply_theme(theme_name);
            self.status_message = Some(format!("Theme: {theme_name}"));
        }
    }

    fn save_config(&mut self) {
        if let Err(e) = self.config.save() {
            self.status_message = Some(format!("Failed to save: {e}"));
        } else {
            // 保存成功后更新原始配置，这样 ESC 退出时不会重置
            self.original_config = self.config.clone();
            self.original_theme = self.config.theme.clone();
            self.status_message = Some("Configuration saved!".to_string());
        }
    }

    pub fn render(&mut self, area: Rect, buf: &mut Buffer) {
        ratatui::widgets::Clear.render(area, buf);

        // 计算 Theme Selector 高度（自适应换行）
        let theme_selector_height = self.calculate_theme_selector_height(area.width);

        let [
            title_area,
            preview_area,
            theme_area,
            content_area,
            help_area,
        ] = Layout::vertical([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(theme_selector_height),
            Constraint::Min(10),
            Constraint::Length(4),
        ])
        .areas(area);

        // 标题
        self.render_title(title_area, buf);

        // 预览
        self.render_preview(preview_area, buf);

        // 主题选择
        self.render_theme_selector(theme_area, buf);

        // 内容区域
        let [list_area, settings_area] =
            Layout::horizontal([Constraint::Percentage(30), Constraint::Percentage(70)])
                .areas(content_area);

        self.render_segment_list(list_area, buf);
        self.render_settings(settings_area, buf);

        // 帮助
        self.render_help(help_area, buf);

        // 渲染对话框（如果打开的话）
        self.color_picker.render(area, buf);
        self.icon_selector.render(area, buf);
        self.separator_editor.render(area, buf);
        self.name_input_dialog.render(area, buf);
    }

    fn calculate_theme_selector_height(&self, width: u16) -> u16 {
        let content_width = width.saturating_sub(4) as usize;
        let mut current_width = 0usize;
        let mut lines = 1usize;

        for (i, theme) in THEME_NAMES.iter().enumerate() {
            let marker = if self.config.theme == *theme {
                "[✓]"
            } else {
                "[ ]"
            };
            let theme_part = format!("{marker} {theme}");
            let separator_width = if i == 0 { 0 } else { 2 };
            let part_width = theme_part.chars().count() + separator_width;

            if current_width + part_width > content_width && current_width > 0 {
                lines += 1;
                current_width = theme_part.chars().count();
            } else {
                current_width += part_width;
            }
        }

        // 主题行 + 边框
        (lines as u16 + 2).min(5)
    }

    fn render_title(&self, area: Rect, buf: &mut Buffer) {
        let title = Paragraph::new("CxLine Configuration")
            .block(Block::default().borders(Borders::ALL))
            .style(Style::default().fg(Color::Cyan))
            .alignment(ratatui::layout::Alignment::Center);
        title.render(area, buf);
    }

    fn render_preview(&self, area: Rect, buf: &mut Buffer) {
        use crate::statusline::renderer::StatusLineRenderer;
        use crate::statusline::segment::Segment;
        use crate::statusline::segments::*;

        let ctx =
            StatusLineContext::new("gpt-5.2-codex", std::path::Path::new("/home/user/Cxline"))
                .with_context(Some(50000), Some(128000))
                .with_rate_limit(Some(25.0), Some("12:00".to_string()))
                .with_git_preview("main", "✓", 0, 0);

        // 按 segment_order 顺序构建预览
        let mut renderer = StatusLineRenderer::new(&self.config);
        for &segment_id in &self.segment_order {
            let segment_config = self.config.get_segment_config(segment_id);
            if !segment_config.enabled {
                continue;
            }

            let data = match segment_id {
                SegmentId::Model => ModelSegment.collect(&ctx),
                SegmentId::Directory => DirectorySegment.collect(&ctx),
                SegmentId::Git => GitSegment.collect(&ctx),
                SegmentId::Context => ContextSegment.collect(&ctx),
                SegmentId::Usage => UsageSegment.collect(&ctx),
            };

            if let Some(data) = data {
                renderer.add_segment(segment_id, data);
            }
        }

        let line = renderer.render_line();

        let block = Block::default().borders(Borders::ALL).title("Preview");
        let inner = block.inner(area);
        block.render(area, buf);

        buf.set_line(inner.x, inner.y, &line, inner.width);
    }

    fn render_theme_selector(&self, area: Rect, buf: &mut Buffer) {
        let block = Block::default().borders(Borders::ALL).title("Theme");
        let inner = block.inner(area);
        block.render(area, buf);

        // 主题列表（自适应换行）
        if inner.height > 0 {
            let content_width = inner.width as usize;
            let mut lines: Vec<Line> = Vec::new();
            let mut current_line_spans: Vec<Span> = Vec::new();
            let mut current_width = 0usize;

            for theme in THEME_NAMES.iter() {
                let is_current = self.config.theme == *theme;
                let marker = if is_current { "[✓]" } else { "[ ]" };
                let theme_part = format!("{marker} {theme}");
                let separator_width = if current_line_spans.is_empty() { 0 } else { 2 };
                let theme_part_len = theme_part.chars().count();
                let part_width = theme_part_len + separator_width;

                if current_width + part_width > content_width && !current_line_spans.is_empty() {
                    lines.push(Line::from(current_line_spans));
                    current_line_spans = Vec::new();
                    current_width = 0;
                }

                if !current_line_spans.is_empty() {
                    current_line_spans.push(Span::raw("  "));
                    current_width += 2;
                }

                let style = if is_current {
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };
                current_line_spans.push(Span::styled(theme_part, style));
                current_width += theme_part_len;
            }

            if !current_line_spans.is_empty() {
                lines.push(Line::from(current_line_spans));
            }

            for (idx, line) in lines.iter().enumerate() {
                let y = inner.y + idx as u16;
                if y < inner.y + inner.height {
                    buf.set_line(inner.x, y, line, inner.width);
                }
            }
        }
    }

    fn render_segment_list(&self, area: Rect, buf: &mut Buffer) {
        let items: Vec<ListItem> = (0..self.segment_count())
            .map(|i| {
                let id = self.segment_id_at(i);
                let is_selected =
                    i == self.selected_segment && self.selected_panel == Panel::SegmentList;
                let segment_config = self.config.get_segment_config(id);
                let enabled_marker = if segment_config.enabled { "●" } else { "○" };
                let name = Self::segment_name(id);

                if is_selected {
                    ListItem::new(Line::from(vec![
                        Span::styled("▶ ", Style::default().fg(Color::Cyan)),
                        Span::raw(format!("{enabled_marker} {name}")),
                    ]))
                } else {
                    ListItem::new(format!("  {enabled_marker} {name}"))
                }
            })
            .collect();

        let block = Block::default()
            .borders(Borders::ALL)
            .title("Segments")
            .border_style(if self.selected_panel == Panel::SegmentList {
                Style::default().fg(Color::Cyan)
            } else {
                Style::default()
            });

        let list = List::new(items).block(block);
        list.render(area, buf);
    }

    fn render_settings(&self, area: Rect, buf: &mut Buffer) {
        let id = self.segment_id_at(self.selected_segment);
        let segment_config = self.config.get_segment_config(id);
        let segment_name = Self::segment_name(id);

        // 获取颜色信息
        let icon_color = segment_config.colors.icon_color().unwrap_or(Color::White);
        let text_color = segment_config.colors.text_color().unwrap_or(Color::White);
        let bg_color = segment_config.colors.background_color();

        // 获取当前图标
        let current_icon = segment_config.icon.get(self.config.style);

        let create_field_line =
            |field: FieldSelection, spans: Vec<Span<'static>>| -> Line<'static> {
                let is_selected =
                    self.selected_panel == Panel::Settings && self.selected_field == field;
                let mut result_spans = vec![];

                if is_selected {
                    result_spans.push(Span::styled("▶ ", Style::default().fg(Color::Cyan)));
                } else {
                    result_spans.push(Span::raw("  "));
                }

                result_spans.extend(spans);
                Line::from(result_spans)
            };

        let lines = vec![
            Line::from(format!("{segment_name} Segment").bold()),
            Line::from(""),
            create_field_line(
                FieldSelection::Enabled,
                vec![Span::raw(format!(
                    "├─ Enabled: {}",
                    if segment_config.enabled { "✓" } else { "✗" }
                ))],
            ),
            create_field_line(
                FieldSelection::Icon,
                vec![
                    Span::raw("├─ Icon: "),
                    Span::styled(current_icon.to_string(), Style::default().fg(icon_color)),
                ],
            ),
            create_field_line(
                FieldSelection::IconColor,
                vec![
                    Span::raw("├─ Icon Color: "),
                    Span::styled("██", Style::default().fg(icon_color)),
                ],
            ),
            create_field_line(
                FieldSelection::TextColor,
                vec![
                    Span::raw("├─ Text Color: "),
                    Span::styled("██", Style::default().fg(text_color)),
                ],
            ),
            create_field_line(
                FieldSelection::BackgroundColor,
                vec![
                    Span::raw("├─ Background: "),
                    if let Some(bg) = bg_color {
                        Span::styled("██", Style::default().fg(bg))
                    } else {
                        Span::styled("--", Style::default().fg(Color::DarkGray))
                    },
                ],
            ),
            create_field_line(
                FieldSelection::TextStyle,
                vec![Span::raw(format!(
                    "├─ Bold: {}",
                    if segment_config.styles.text_bold {
                        "[✓]"
                    } else {
                        "[ ]"
                    }
                ))],
            ),
            create_field_line(
                FieldSelection::Options,
                vec![Span::raw(format!(
                    "└─ Options: {} items",
                    segment_config.options.len()
                ))],
            ),
        ];

        let block = Block::default()
            .borders(Borders::ALL)
            .title("Settings")
            .border_style(if self.selected_panel == Panel::Settings {
                Style::default().fg(Color::Cyan)
            } else {
                Style::default()
            });

        let paragraph = Paragraph::new(lines).block(block);
        paragraph.render(area, buf);
    }

    fn render_help(&self, area: Rect, buf: &mut Buffer) {
        let help_items: Vec<(&str, &str)> = vec![
            ("[Tab]", "Switch Panel"),
            ("[↑↓]", "Select"),
            ("[Shift+↑↓]", "Reorder"),
            ("[Enter]", "Toggle/Edit"),
            ("[1-9]", "Theme"),
            ("[P]", "Cycle Theme"),
            ("[R]", "Reset Theme"),
            ("[E]", "Edit Separator"),
            ("[W]", "Write Theme"),
            ("[Ctrl+S]", "Save Theme"),
            ("[S]", "Save Config"),
            ("[Esc]", "Quit"),
        ];

        let block = Block::default().borders(Borders::ALL).title("Help");
        let inner = block.inner(area);
        block.render(area, buf);

        // 构建帮助行（智能换行）
        let content_width = inner.width as usize;
        let mut lines: Vec<Line> = Vec::new();
        let mut current_line_spans: Vec<Span> = Vec::new();
        let mut current_width = 0usize;

        for (key, desc) in help_items.iter() {
            let item_width = key.chars().count() + desc.chars().count() + 1;
            let separator_width = if current_line_spans.is_empty() { 0 } else { 2 };
            let total_width = item_width + separator_width;

            if current_width + total_width > content_width && !current_line_spans.is_empty() {
                lines.push(Line::from(current_line_spans));
                current_line_spans = Vec::new();
                current_width = 0;
            }

            if !current_line_spans.is_empty() {
                current_line_spans.push(Span::raw("  "));
                current_width += 2;
            }

            current_line_spans.push(Span::styled(
                *key,
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ));
            current_line_spans.push(Span::styled(
                format!(" {desc}"),
                Style::default().fg(Color::Gray),
            ));
            current_width += item_width;
        }

        if !current_line_spans.is_empty() {
            lines.push(Line::from(current_line_spans));
        }

        // 添加状态消息
        if let Some(msg) = &self.status_message {
            lines.push(Line::from(Span::styled(
                msg.clone(),
                Style::default().fg(Color::Green),
            )));
        }

        for (idx, line) in lines.iter().enumerate() {
            let y = inner.y + idx as u16;
            if y < inner.y + inner.height {
                buf.set_line(inner.x, y, line, inner.width);
            }
        }
    }
}
