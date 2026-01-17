// 状态栏渲染引擎
// 参考 CCometixLine 的 statusline.rs

use super::config::CxLineConfig;
use super::segment::{SegmentData, SegmentId};
use super::style::{StyleMode, separators};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::WidgetRef;

/// Powerline 箭头字符
const POWERLINE_ARROW: &str = "\u{e0b0}";

/// 状态栏渲染器
pub struct StatusLineRenderer<'a> {
    config: &'a CxLineConfig,
    segments: Vec<(SegmentId, SegmentData)>,
}

impl<'a> StatusLineRenderer<'a> {
    pub fn new(config: &'a CxLineConfig) -> Self {
        Self {
            config,
            segments: Vec::new(),
        }
    }

    /// 添加 segment 数据
    pub fn add_segment(&mut self, id: SegmentId, data: SegmentData) {
        self.segments.push((id, data));
    }

    /// 渲染为 Line
    pub fn render_line(&self) -> Line<'static> {
        match self.config.style {
            StyleMode::Powerline => self.render_powerline(),
            _ => self.render_plain(),
        }
    }

    /// 渲染普通模式（Plain / NerdFont）
    fn render_plain(&self) -> Line<'static> {
        let mut spans: Vec<Span<'static>> = Vec::new();
        let separator = self.get_separator();
        let mut first = true;

        for (id, data) in self.segments.iter() {
            let segment_config = self.config.get_segment_config(*id);
            if !segment_config.enabled {
                continue;
            }

            if !first {
                spans.push(Span::raw(separator.to_string()).dim());
            }
            first = false;

            // 渲染图标
            let icon = self.get_icon(*id, data);
            if !icon.is_empty() {
                let mut icon_style = Style::default();
                if let Some(color) = segment_config.colors.icon_color() {
                    icon_style = icon_style.fg(color);
                }
                spans.push(Span::styled(format!("{icon} "), icon_style));
            }

            // 渲染主要内容
            let mut text_style = Style::default();
            if let Some(color) = segment_config.colors.text_color() {
                text_style = text_style.fg(color);
            }
            if segment_config.styles.text_bold {
                text_style = text_style.bold();
            }
            spans.push(Span::styled(data.primary.clone(), text_style));

            // 渲染次要内容
            if !data.secondary.is_empty() {
                spans.push(Span::styled(format!(" {}", data.secondary), text_style));
            }
        }

        Line::from(spans)
    }

    /// 渲染 Powerline 模式（带背景色和箭头过渡）
    fn render_powerline(&self) -> Line<'static> {
        let mut spans: Vec<Span<'static>> = Vec::new();

        // 收集启用的 segment
        let enabled_segments: Vec<_> = self
            .segments
            .iter()
            .filter(|(id, _)| self.config.get_segment_config(*id).enabled)
            .collect();

        let segment_count = enabled_segments.len();

        for (i, (id, data)) in enabled_segments.iter().enumerate() {
            let segment_config = self.config.get_segment_config(*id);

            // 获取背景色
            let bg_color = segment_config.colors.background_color();
            let text_color = segment_config.colors.text_color();
            let icon_color = segment_config.colors.icon_color();

            // 构建 segment 样式
            let mut segment_style = Style::default();
            if let Some(bg) = bg_color {
                segment_style = segment_style.bg(bg);
            }
            if let Some(fg) = text_color {
                segment_style = segment_style.fg(fg);
            }
            if segment_config.styles.text_bold {
                segment_style = segment_style.bold();
            }

            // 添加左边距
            spans.push(Span::styled(" ", segment_style));

            // 渲染图标
            let icon = self.get_icon(*id, data);
            if !icon.is_empty() {
                let mut icon_style = segment_style;
                if let Some(ic) = icon_color {
                    icon_style = icon_style.fg(ic);
                }
                spans.push(Span::styled(format!("{icon} "), icon_style));
            }

            // 渲染主要内容
            spans.push(Span::styled(data.primary.clone(), segment_style));

            // 渲染次要内容
            if !data.secondary.is_empty() {
                spans.push(Span::styled(format!(" {}", data.secondary), segment_style));
            }

            // 添加右边距
            spans.push(Span::styled(" ", segment_style));

            // 添加 Powerline 箭头过渡（最后一个 segment 不需要箭头）
            if i < segment_count - 1 {
                let next_segment_config = self.config.get_segment_config(enabled_segments[i + 1].0);
                let next_bg = next_segment_config.colors.background_color();

                let mut arrow_style = Style::default();
                if let Some(curr_bg) = bg_color {
                    arrow_style = arrow_style.fg(curr_bg);
                }
                if let Some(next_bg_color) = next_bg {
                    arrow_style = arrow_style.bg(next_bg_color);
                }
                spans.push(Span::styled(POWERLINE_ARROW, arrow_style));
            }
        }

        Line::from(spans)
    }

    /// 获取分隔符
    fn get_separator(&self) -> &'static str {
        match self.config.style {
            StyleMode::Powerline => separators::POWERLINE_THIN,
            _ => separators::SIMPLE,
        }
    }

    /// 获取图标
    fn get_icon(&self, id: SegmentId, data: &SegmentData) -> String {
        // 优先使用动态图标（从元数据）
        if let Some(dynamic_icon) = data.metadata.get("dynamic_icon") {
            return dynamic_icon.clone();
        }

        let segment_config = self.config.get_segment_config(id);
        segment_config.icon.get(self.config.style).to_string()
    }
}

/// 状态栏 Widget
pub struct StatusLineWidget<'a> {
    line: Line<'a>,
}

impl<'a> StatusLineWidget<'a> {
    pub fn new(line: Line<'a>) -> Self {
        Self { line }
    }

    pub fn from_renderer(renderer: &StatusLineRenderer<'_>) -> Self {
        Self {
            line: renderer.render_line(),
        }
    }
}

impl WidgetRef for StatusLineWidget<'_> {
    fn render_ref(&self, area: Rect, buf: &mut Buffer) {
        if area.height == 0 || area.width == 0 {
            return;
        }

        // 渲染状态栏内容
        let line = self.line.clone();
        buf.set_line(area.x, area.y, &line, area.width);
    }
}
