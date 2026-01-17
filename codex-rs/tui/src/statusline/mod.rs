// Codex TUI 状态栏模块
// 参考 CCometixLine 设计

pub mod color_picker;
pub mod config;
pub mod icon_selector;
pub mod name_input;
pub mod renderer;
pub mod segment;
pub mod segments;
pub mod separator_editor;
pub mod style;
pub mod themes;

use std::path::Path;

pub use color_picker::{ColorPicker, ColorTarget};
pub use config::CxLineConfig;
pub use icon_selector::IconSelector;
pub use name_input::NameInputDialog;
pub use renderer::{StatusLineRenderer, StatusLineWidget};
pub use segment::{Segment, SegmentData, SegmentId, SegmentStyle};
pub use separator_editor::SeparatorEditor;
pub use style::StyleMode;

/// Git 预览数据（用于配置页预览）
#[derive(Debug, Clone)]
pub struct GitPreviewData {
    pub branch: String,
    pub status: String,
    pub ahead: u32,
    pub behind: u32,
}

/// 状态栏数据上下文
/// 包含渲染状态栏所需的所有数据
pub struct StatusLineContext<'a> {
    /// 当前模型名称
    pub model_name: &'a str,

    /// 当前工作目录
    pub cwd: &'a Path,

    /// 已使用的 token 数
    pub context_used_tokens: Option<i64>,

    /// 上下文窗口大小（用于计算使用占比）
    pub context_window_size: Option<i64>,

    /// Rate limit 使用百分比
    pub rate_limit_percent: Option<f64>,

    /// Rate limit 重置时间
    pub rate_limit_resets_at: Option<String>,

    /// Git 预览数据（用于配置页预览，覆盖实际 git 检测）
    pub git_preview: Option<GitPreviewData>,
}

impl<'a> StatusLineContext<'a> {
    pub fn new(model_name: &'a str, cwd: &'a Path) -> Self {
        Self {
            model_name,
            cwd,
            context_used_tokens: None,
            context_window_size: None,
            rate_limit_percent: None,
            rate_limit_resets_at: None,
            git_preview: None,
        }
    }

    pub fn with_context(mut self, used_tokens: Option<i64>, window_size: Option<i64>) -> Self {
        self.context_used_tokens = used_tokens;
        self.context_window_size = window_size;
        self
    }

    pub fn with_rate_limit(mut self, percent: Option<f64>, resets_at: Option<String>) -> Self {
        self.rate_limit_percent = percent;
        self.rate_limit_resets_at = resets_at;
        self
    }

    /// 设置 Git 预览数据（用于配置页预览）
    pub fn with_git_preview(mut self, branch: &str, status: &str, ahead: u32, behind: u32) -> Self {
        self.git_preview = Some(GitPreviewData {
            branch: branch.to_string(),
            status: status.to_string(),
            ahead,
            behind,
        });
        self
    }
}

/// 构建状态栏
/// 收集所有 segment 数据并返回渲染器
pub fn build_statusline<'a>(
    config: &'a CxLineConfig,
    ctx: &StatusLineContext<'_>,
) -> StatusLineRenderer<'a> {
    use segments::*;

    let mut renderer = StatusLineRenderer::new(config);

    // Model segment
    if config.segments.model.enabled {
        let segment = ModelSegment;
        if let Some(data) = segment.collect(ctx) {
            renderer.add_segment(SegmentId::Model, data);
        }
    }

    // Directory segment
    if config.segments.directory.enabled {
        let segment = DirectorySegment;
        if let Some(data) = segment.collect(ctx) {
            renderer.add_segment(SegmentId::Directory, data);
        }
    }

    // Git segment
    if config.segments.git.enabled {
        let segment = GitSegment;
        if let Some(data) = segment.collect(ctx) {
            renderer.add_segment(SegmentId::Git, data);
        }
    }

    // Context segment
    if config.segments.context.enabled {
        let segment = ContextSegment;
        if let Some(data) = segment.collect(ctx) {
            renderer.add_segment(SegmentId::Context, data);
        }
    }

    // Usage segment
    if config.segments.usage.enabled {
        let segment = UsageSegment;
        if let Some(data) = segment.collect(ctx) {
            renderer.add_segment(SegmentId::Usage, data);
        }
    }

    renderer
}
