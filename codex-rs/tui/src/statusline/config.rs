// 状态栏配置
// 配置文件位置：~/.codex/cxline/config.toml

use super::segment::SegmentId;
use super::style::ColorConfig;
use super::style::IconConfig;
use super::style::StyleMode;
use super::style::TextStyleConfig;
use super::themes::ThemePresets;
use serde::Deserialize;
use serde::Serialize;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

/// 状态栏配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CxLineConfig {
    /// 是否启用状态栏
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// 当前使用的主题名称
    #[serde(default = "default_theme")]
    pub theme: String,

    /// 样式模式
    #[serde(default)]
    pub style: StyleMode,

    /// 分隔符（仅 Plain/NerdFont 模式使用）
    #[serde(default = "default_separator")]
    pub separator: String,

    /// 各 segment 配置
    #[serde(default)]
    pub segments: SegmentsConfig,
}

fn default_true() -> bool {
    true
}

fn default_theme() -> String {
    "cometix".to_string()
}

fn default_separator() -> String {
    " │ ".to_string()
}

/// 各 segment 的配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SegmentsConfig {
    #[serde(default = "SegmentItemConfig::default_model")]
    pub model: SegmentItemConfig,

    #[serde(default = "SegmentItemConfig::default_directory")]
    pub directory: SegmentItemConfig,

    #[serde(default = "SegmentItemConfig::default_git")]
    pub git: SegmentItemConfig,

    #[serde(default = "SegmentItemConfig::default_context")]
    pub context: SegmentItemConfig,

    #[serde(default = "SegmentItemConfig::default_usage")]
    pub usage: SegmentItemConfig,
}

impl Default for SegmentsConfig {
    fn default() -> Self {
        let theme = ThemePresets::get_default();
        theme.segments
    }
}

/// 单个 segment 的配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SegmentItemConfig {
    /// Segment ID
    #[serde(default)]
    pub id: SegmentId,

    /// 是否启用
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// 图标配置
    #[serde(default)]
    pub icon: IconConfig,

    /// 颜色配置
    #[serde(default)]
    pub colors: ColorConfig,

    /// 文本样式配置
    #[serde(default)]
    pub styles: TextStyleConfig,

    /// 自定义选项
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub options: HashMap<String, serde_json::Value>,
}

impl SegmentItemConfig {
    pub fn default_model() -> Self {
        ThemePresets::get_default().segments.model
    }

    pub fn default_directory() -> Self {
        ThemePresets::get_default().segments.directory
    }

    pub fn default_git() -> Self {
        ThemePresets::get_default().segments.git
    }

    pub fn default_context() -> Self {
        ThemePresets::get_default().segments.context
    }

    pub fn default_usage() -> Self {
        ThemePresets::get_default().segments.usage
    }
}

impl Default for CxLineConfig {
    fn default() -> Self {
        ThemePresets::get_theme("cometix")
    }
}

impl CxLineConfig {
    /// 获取配置目录路径
    pub fn config_dir() -> Option<PathBuf> {
        dirs::home_dir().map(|home| home.join(".codex").join("cxline"))
    }

    /// 获取配置文件路径
    pub fn config_path() -> Option<PathBuf> {
        Self::config_dir().map(|dir| dir.join("config.toml"))
    }

    /// 获取主题目录路径
    pub fn themes_dir() -> Option<PathBuf> {
        Self::config_dir().map(|dir| dir.join("themes"))
    }

    /// 初始化配置目录和主题文件
    pub fn init() {
        // 确保配置目录存在
        if let Some(config_dir) = Self::config_dir() {
            let _ = fs::create_dir_all(&config_dir);
        }

        // 确保主题目录和预设文件存在
        ThemePresets::ensure_themes_exist();
    }

    /// 从文件加载配置
    pub fn load() -> Self {
        // 首先初始化目录结构
        Self::init();

        let Some(path) = Self::config_path() else {
            return Self::default();
        };

        if !path.exists() {
            let config = Self::default();
            // 首次运行时创建默认配置文件
            let _ = config.save();
            return config;
        }

        match fs::read_to_string(&path) {
            Ok(content) => match toml::from_str::<CxLineConfig>(&content) {
                Ok(config) => config,
                Err(e) => {
                    tracing::warn!("解析 cxline 配置失败: {}, 使用默认配置", e);
                    Self::default()
                }
            },
            Err(e) => {
                tracing::warn!("读取 cxline 配置失败: {}, 使用默认配置", e);
                Self::default()
            }
        }
    }

    /// 保存配置到文件
    pub fn save(&self) -> std::io::Result<()> {
        let Some(path) = Self::config_path() else {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "无法确定配置文件路径",
            ));
        };

        // 确保目录存在
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let content = toml::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))?;

        fs::write(&path, content)
    }

    /// 应用主题
    pub fn apply_theme(&mut self, theme_name: &str) {
        let theme = ThemePresets::get_theme(theme_name);
        self.theme = theme_name.to_string();
        self.style = theme.style;
        self.separator = theme.separator;
        self.segments = theme.segments;
    }

    /// 获取指定 segment 的配置
    pub fn get_segment_config(&self, id: SegmentId) -> &SegmentItemConfig {
        match id {
            SegmentId::Model => &self.segments.model,
            SegmentId::Directory => &self.segments.directory,
            SegmentId::Git => &self.segments.git,
            SegmentId::Context => &self.segments.context,
            SegmentId::Usage => &self.segments.usage,
        }
    }

    /// 获取指定 segment 的可变配置
    pub fn get_segment_config_mut(&mut self, id: SegmentId) -> &mut SegmentItemConfig {
        match id {
            SegmentId::Model => &mut self.segments.model,
            SegmentId::Directory => &mut self.segments.directory,
            SegmentId::Git => &mut self.segments.git,
            SegmentId::Context => &mut self.segments.context,
            SegmentId::Usage => &mut self.segments.usage,
        }
    }
}
