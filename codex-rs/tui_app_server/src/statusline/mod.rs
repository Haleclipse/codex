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

use codex_protocol::openai_models::ReasoningEffort;

pub use color_picker::ColorPicker;
pub use color_picker::ColorTarget;
pub use config::CxLineConfig;
pub use icon_selector::IconSelector;
pub use name_input::NameInputDialog;
pub use renderer::StatusLineRenderer;
pub use renderer::StatusLineWidget;
pub use segment::Segment;
pub use segment::SegmentData;
pub use segment::SegmentId;
pub use segment::SegmentStyle;
pub use separator_editor::SeparatorEditor;
pub use style::StyleMode;

/// Git 预览数据（用于配置页预览）
#[derive(Debug, Clone, PartialEq, Eq)]
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

    /// Reasoning effort level
    pub reasoning_effort: Option<ReasoningEffort>,

    /// 当前工作目录
    pub cwd: &'a Path,

    /// 已使用的 token 数
    pub context_used_tokens: Option<i64>,

    /// 上下文窗口大小（用于计算使用占比）
    pub context_window_size: Option<i64>,

    /// 5h Rate limit 使用百分比 (用于百分比数字显示)
    pub hourly_rate_limit_percent: Option<f64>,

    /// Weekly Rate limit 使用百分比 (用于圆圈进度条)
    pub weekly_rate_limit_percent: Option<f64>,

    /// Weekly Rate limit 重置时间
    pub weekly_rate_limit_resets_at: Option<String>,

    /// Git 预览数据（用于配置页预览，覆盖实际 git 检测）
    pub git_preview: Option<GitPreviewData>,
}

impl<'a> StatusLineContext<'a> {
    pub fn new(model_name: &'a str, cwd: &'a Path) -> Self {
        Self {
            model_name,
            reasoning_effort: None,
            cwd,
            context_used_tokens: None,
            context_window_size: None,
            hourly_rate_limit_percent: None,
            weekly_rate_limit_percent: None,
            weekly_rate_limit_resets_at: None,
            git_preview: None,
        }
    }

    pub fn with_reasoning_effort(mut self, effort: Option<ReasoningEffort>) -> Self {
        self.reasoning_effort = effort;
        self
    }

    pub fn with_context(mut self, used_tokens: Option<i64>, window_size: Option<i64>) -> Self {
        self.context_used_tokens = used_tokens;
        self.context_window_size = window_size;
        self
    }

    pub fn with_rate_limit(
        mut self,
        hourly_percent: Option<f64>,
        weekly_percent: Option<f64>,
        weekly_resets_at: Option<String>,
    ) -> Self {
        self.hourly_rate_limit_percent = hourly_percent;
        self.weekly_rate_limit_percent = weekly_percent;
        self.weekly_rate_limit_resets_at = weekly_resets_at;
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

impl GitPreviewData {
    pub fn empty() -> Self {
        Self {
            branch: String::new(),
            status: String::new(),
            ahead: 0,
            behind: 0,
        }
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

/// 异步更新用的 Git 预览数据收集（避免在 render 中执行 git 命令）
pub(crate) fn collect_git_preview(cwd: &Path) -> Option<GitPreviewData> {
    let segment = segments::GitSegment;
    segment.collect_preview(cwd)
}
