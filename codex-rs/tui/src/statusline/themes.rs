// 主题预设系统

use super::config::{CxLineConfig, SegmentItemConfig, SegmentsConfig};
use super::style::{AnsiColor, ColorConfig, IconConfig, StyleMode, TextStyleConfig, ansi16};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

/// 可用的预设主题名称
pub const THEME_NAMES: &[&str] = &[
    "default",
    "cometix",
    "minimal",
    "gruvbox",
    "nord",
    "powerline-dark",
    "powerline-light",
    "powerline-rose-pine",
    "powerline-tokyo-night",
];

/// 主题预设
pub struct ThemePresets;

impl ThemePresets {
    /// 获取主题目录路径
    pub fn themes_dir() -> Option<PathBuf> {
        dirs::home_dir().map(|h| h.join(".codex").join("cxline").join("themes"))
    }

    /// 确保主题目录和预设文件存在
    pub fn ensure_themes_exist() {
        if let Some(themes_dir) = Self::themes_dir() {
            if !themes_dir.exists() {
                let _ = fs::create_dir_all(&themes_dir);
            }

            for theme_name in THEME_NAMES {
                let theme_path = themes_dir.join(format!("{theme_name}.toml"));
                if !theme_path.exists()
                    && let Some(config) = Self::get_builtin(theme_name)
                    && let Ok(content) = toml::to_string_pretty(&config)
                {
                    let _ = fs::write(&theme_path, content);
                }
            }
        }
    }

    /// 从文件加载主题
    pub fn load_from_file(theme_name: &str) -> Option<CxLineConfig> {
        let themes_dir = Self::themes_dir()?;
        let theme_path = themes_dir.join(format!("{theme_name}.toml"));

        if !theme_path.exists() {
            return None;
        }

        let content = fs::read_to_string(&theme_path).ok()?;
        toml::from_str(&content).ok()
    }

    /// 获取主题（优先从文件加载，回退到内置预设）
    pub fn get_theme(theme_name: &str) -> CxLineConfig {
        if let Some(config) = Self::load_from_file(theme_name) {
            return config;
        }
        Self::get_builtin(theme_name).unwrap_or_else(Self::get_default)
    }

    /// 保存配置为主题文件
    pub fn save_theme(theme_name: &str, config: &CxLineConfig) -> std::io::Result<()> {
        let themes_dir = Self::themes_dir()
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "无法确定主题目录"))?;

        // 确保目录存在
        fs::create_dir_all(&themes_dir)?;

        let theme_path = themes_dir.join(format!("{theme_name}.toml"));
        let content = toml::to_string_pretty(config)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))?;

        fs::write(&theme_path, content)
    }

    /// 获取内置预设主题
    pub fn get_builtin(theme_name: &str) -> Option<CxLineConfig> {
        match theme_name {
            "default" => Some(Self::get_default()),
            "cometix" => Some(Self::get_cometix()),
            "minimal" => Some(Self::get_minimal()),
            "gruvbox" => Some(Self::get_gruvbox()),
            "nord" => Some(Self::get_nord()),
            "powerline-dark" => Some(Self::get_powerline_dark()),
            "powerline-light" => Some(Self::get_powerline_light()),
            "powerline-rose-pine" => Some(Self::get_powerline_rose_pine()),
            "powerline-tokyo-night" => Some(Self::get_powerline_tokyo_night()),
            _ => None,
        }
    }

    /// Default 主题
    pub fn get_default() -> CxLineConfig {
        CxLineConfig {
            enabled: true,
            theme: "default".to_string(),
            style: StyleMode::Plain,
            separator: " │ ".to_string(),
            segments: SegmentsConfig {
                model: SegmentItemConfig {
                    id: super::segment::SegmentId::Model,
                    enabled: true,
                    icon: IconConfig::new("🤖", "\u{e26d}"),
                    colors: ColorConfig::new(ansi16::BRIGHT_CYAN, ansi16::BRIGHT_CYAN),
                    styles: TextStyleConfig::default(),
                    options: HashMap::new(),
                },
                directory: SegmentItemConfig {
                    id: super::segment::SegmentId::Directory,
                    enabled: true,
                    icon: IconConfig::new("📁", "\u{f024b}"),
                    colors: ColorConfig::new(ansi16::BRIGHT_YELLOW, ansi16::BRIGHT_GREEN),
                    styles: TextStyleConfig::default(),
                    options: HashMap::new(),
                },
                git: SegmentItemConfig {
                    id: super::segment::SegmentId::Git,
                    enabled: true,
                    icon: IconConfig::new("🌿", "\u{f02a2}"),
                    colors: ColorConfig::new(ansi16::BRIGHT_BLUE, ansi16::BRIGHT_BLUE),
                    styles: TextStyleConfig::default(),
                    options: HashMap::new(),
                },
                context: SegmentItemConfig {
                    id: super::segment::SegmentId::Context,
                    enabled: true,
                    icon: IconConfig::new("⚡️", "\u{f49b}"),
                    colors: ColorConfig::new(ansi16::BRIGHT_MAGENTA, ansi16::BRIGHT_MAGENTA),
                    styles: TextStyleConfig::default(),
                    options: HashMap::new(),
                },
                usage: SegmentItemConfig {
                    id: super::segment::SegmentId::Usage,
                    enabled: true,
                    icon: IconConfig::new("📊", "\u{f0a9e}"),
                    colors: ColorConfig::new(ansi16::BRIGHT_CYAN, ansi16::BRIGHT_CYAN),
                    styles: TextStyleConfig::default(),
                    options: HashMap::new(),
                },
            },
        }
    }

    /// Cometix 主题
    pub fn get_cometix() -> CxLineConfig {
        CxLineConfig {
            enabled: true,
            theme: "cometix".to_string(),
            style: StyleMode::NerdFont,
            separator: " │ ".to_string(),
            segments: SegmentsConfig {
                model: SegmentItemConfig {
                    id: super::segment::SegmentId::Model,
                    enabled: true,
                    icon: IconConfig::new("🤖", "\u{e26d}"),
                    colors: ColorConfig::new(ansi16::BRIGHT_CYAN, ansi16::BRIGHT_CYAN),
                    styles: TextStyleConfig { text_bold: true },
                    options: HashMap::new(),
                },
                directory: SegmentItemConfig {
                    id: super::segment::SegmentId::Directory,
                    enabled: true,
                    icon: IconConfig::new("📁", "\u{f024b}"),
                    colors: ColorConfig::new(ansi16::BRIGHT_YELLOW, ansi16::BRIGHT_GREEN),
                    styles: TextStyleConfig { text_bold: true },
                    options: HashMap::new(),
                },
                git: SegmentItemConfig {
                    id: super::segment::SegmentId::Git,
                    enabled: true,
                    icon: IconConfig::new("🌿", "\u{f02a2}"),
                    colors: ColorConfig::new(ansi16::BRIGHT_BLUE, ansi16::BRIGHT_BLUE),
                    styles: TextStyleConfig { text_bold: true },
                    options: HashMap::new(),
                },
                context: SegmentItemConfig {
                    id: super::segment::SegmentId::Context,
                    enabled: true,
                    icon: IconConfig::new("⚡️", "\u{f49b}"),
                    colors: ColorConfig::new(ansi16::BRIGHT_MAGENTA, ansi16::BRIGHT_MAGENTA),
                    styles: TextStyleConfig { text_bold: true },
                    options: HashMap::new(),
                },
                usage: SegmentItemConfig {
                    id: super::segment::SegmentId::Usage,
                    enabled: true,
                    icon: IconConfig::new("📊", "\u{f0a9e}"),
                    colors: ColorConfig::new(ansi16::BRIGHT_CYAN, ansi16::BRIGHT_CYAN),
                    styles: TextStyleConfig::default(),
                    options: HashMap::new(),
                },
            },
        }
    }

    /// Minimal 主题
    pub fn get_minimal() -> CxLineConfig {
        CxLineConfig {
            enabled: true,
            theme: "minimal".to_string(),
            style: StyleMode::Plain,
            separator: " │ ".to_string(),
            segments: SegmentsConfig {
                model: SegmentItemConfig {
                    id: super::segment::SegmentId::Model,
                    enabled: true,
                    icon: IconConfig::new("✽", "\u{f2d0}"),
                    colors: ColorConfig::new(ansi16::BRIGHT_CYAN, ansi16::BRIGHT_CYAN),
                    styles: TextStyleConfig::default(),
                    options: HashMap::new(),
                },
                directory: SegmentItemConfig {
                    id: super::segment::SegmentId::Directory,
                    enabled: true,
                    icon: IconConfig::new("◐", "\u{f024b}"),
                    colors: ColorConfig::new(ansi16::BRIGHT_YELLOW, ansi16::BRIGHT_GREEN),
                    styles: TextStyleConfig::default(),
                    options: HashMap::new(),
                },
                git: SegmentItemConfig {
                    id: super::segment::SegmentId::Git,
                    enabled: true,
                    icon: IconConfig::new("※", "\u{f02a2}"),
                    colors: ColorConfig::new(ansi16::BRIGHT_BLUE, ansi16::BRIGHT_BLUE),
                    styles: TextStyleConfig::default(),
                    options: HashMap::new(),
                },
                context: SegmentItemConfig {
                    id: super::segment::SegmentId::Context,
                    enabled: true,
                    icon: IconConfig::new("◐", "\u{f49b}"),
                    colors: ColorConfig::new(ansi16::BRIGHT_MAGENTA, ansi16::BRIGHT_MAGENTA),
                    styles: TextStyleConfig::default(),
                    options: HashMap::new(),
                },
                usage: SegmentItemConfig {
                    id: super::segment::SegmentId::Usage,
                    enabled: true,
                    icon: IconConfig::new("📊", "\u{f0a9e}"),
                    colors: ColorConfig::new(ansi16::BRIGHT_CYAN, ansi16::BRIGHT_CYAN),
                    styles: TextStyleConfig::default(),
                    options: HashMap::new(),
                },
            },
        }
    }

    /// Gruvbox 主题
    pub fn get_gruvbox() -> CxLineConfig {
        let gruvbox_orange = AnsiColor::c256(208);
        let gruvbox_green = AnsiColor::c256(142);
        let gruvbox_cyan = AnsiColor::c256(109);

        CxLineConfig {
            enabled: true,
            theme: "gruvbox".to_string(),
            style: StyleMode::NerdFont,
            separator: " │ ".to_string(),
            segments: SegmentsConfig {
                model: SegmentItemConfig {
                    id: super::segment::SegmentId::Model,
                    enabled: true,
                    icon: IconConfig::new("🤖", "\u{e26d}"),
                    colors: ColorConfig::new(gruvbox_orange, gruvbox_orange),
                    styles: TextStyleConfig { text_bold: true },
                    options: HashMap::new(),
                },
                directory: SegmentItemConfig {
                    id: super::segment::SegmentId::Directory,
                    enabled: true,
                    icon: IconConfig::new("📁", "\u{f024b}"),
                    colors: ColorConfig::new(gruvbox_green, gruvbox_green),
                    styles: TextStyleConfig { text_bold: true },
                    options: HashMap::new(),
                },
                git: SegmentItemConfig {
                    id: super::segment::SegmentId::Git,
                    enabled: true,
                    icon: IconConfig::new("🌿", "\u{f02a2}"),
                    colors: ColorConfig::new(gruvbox_cyan, gruvbox_cyan),
                    styles: TextStyleConfig { text_bold: true },
                    options: HashMap::new(),
                },
                context: SegmentItemConfig {
                    id: super::segment::SegmentId::Context,
                    enabled: true,
                    icon: IconConfig::new("⚡️", "\u{f49b}"),
                    colors: ColorConfig::new(ansi16::MAGENTA, ansi16::MAGENTA),
                    styles: TextStyleConfig { text_bold: true },
                    options: HashMap::new(),
                },
                usage: SegmentItemConfig {
                    id: super::segment::SegmentId::Usage,
                    enabled: true,
                    icon: IconConfig::new("📊", "\u{f0a9e}"),
                    colors: ColorConfig::new(ansi16::BRIGHT_CYAN, ansi16::BRIGHT_CYAN),
                    styles: TextStyleConfig::default(),
                    options: HashMap::new(),
                },
            },
        }
    }

    /// Nord 主题 (Powerline)
    pub fn get_nord() -> CxLineConfig {
        let nord_polar = AnsiColor::rgb(46, 52, 64);
        let bg_model = AnsiColor::rgb(136, 192, 208);
        let bg_dir = AnsiColor::rgb(163, 190, 140);
        let bg_git = AnsiColor::rgb(129, 161, 193);
        let bg_context = AnsiColor::rgb(180, 142, 173);
        let bg_usage = AnsiColor::rgb(235, 203, 139);

        CxLineConfig {
            enabled: true,
            theme: "nord".to_string(),
            style: StyleMode::Powerline,
            separator: "\u{e0b0}".to_string(),
            segments: SegmentsConfig {
                model: SegmentItemConfig {
                    id: super::segment::SegmentId::Model,
                    enabled: true,
                    icon: IconConfig::new("🤖", "\u{e26d}"),
                    colors: ColorConfig::new(nord_polar, nord_polar).with_background(bg_model),
                    styles: TextStyleConfig::default(),
                    options: HashMap::new(),
                },
                directory: SegmentItemConfig {
                    id: super::segment::SegmentId::Directory,
                    enabled: true,
                    icon: IconConfig::new("📁", "\u{f024b}"),
                    colors: ColorConfig::new(nord_polar, nord_polar).with_background(bg_dir),
                    styles: TextStyleConfig::default(),
                    options: HashMap::new(),
                },
                git: SegmentItemConfig {
                    id: super::segment::SegmentId::Git,
                    enabled: true,
                    icon: IconConfig::new("🌿", "\u{f02a2}"),
                    colors: ColorConfig::new(nord_polar, nord_polar).with_background(bg_git),
                    styles: TextStyleConfig::default(),
                    options: HashMap::new(),
                },
                context: SegmentItemConfig {
                    id: super::segment::SegmentId::Context,
                    enabled: true,
                    icon: IconConfig::new("⚡️", "\u{f49b}"),
                    colors: ColorConfig::new(nord_polar, nord_polar).with_background(bg_context),
                    styles: TextStyleConfig::default(),
                    options: HashMap::new(),
                },
                usage: SegmentItemConfig {
                    id: super::segment::SegmentId::Usage,
                    enabled: true,
                    icon: IconConfig::new("📊", "\u{f0a9e}"),
                    colors: ColorConfig::new(nord_polar, nord_polar).with_background(bg_usage),
                    styles: TextStyleConfig::default(),
                    options: HashMap::new(),
                },
            },
        }
    }

    /// Powerline Dark 主题
    pub fn get_powerline_dark() -> CxLineConfig {
        let white = AnsiColor::rgb(255, 255, 255);
        let light_gray = AnsiColor::rgb(209, 213, 219);

        let bg_model = AnsiColor::rgb(45, 45, 45);
        let bg_dir = AnsiColor::rgb(139, 69, 19);
        let bg_git = AnsiColor::rgb(64, 64, 64);
        let bg_context = AnsiColor::rgb(55, 65, 81);
        let bg_usage = AnsiColor::rgb(45, 50, 59);

        CxLineConfig {
            enabled: true,
            theme: "powerline-dark".to_string(),
            style: StyleMode::Powerline,
            separator: "\u{e0b0}".to_string(),
            segments: SegmentsConfig {
                model: SegmentItemConfig {
                    id: super::segment::SegmentId::Model,
                    enabled: true,
                    icon: IconConfig::new("🤖", "\u{e26d}"),
                    colors: ColorConfig::new(white, white).with_background(bg_model),
                    styles: TextStyleConfig::default(),
                    options: HashMap::new(),
                },
                directory: SegmentItemConfig {
                    id: super::segment::SegmentId::Directory,
                    enabled: true,
                    icon: IconConfig::new("📁", "\u{f024b}"),
                    colors: ColorConfig::new(white, white).with_background(bg_dir),
                    styles: TextStyleConfig::default(),
                    options: HashMap::new(),
                },
                git: SegmentItemConfig {
                    id: super::segment::SegmentId::Git,
                    enabled: true,
                    icon: IconConfig::new("🌿", "\u{f02a2}"),
                    colors: ColorConfig::new(white, white).with_background(bg_git),
                    styles: TextStyleConfig::default(),
                    options: HashMap::new(),
                },
                context: SegmentItemConfig {
                    id: super::segment::SegmentId::Context,
                    enabled: true,
                    icon: IconConfig::new("⚡️", "\u{f49b}"),
                    colors: ColorConfig::new(light_gray, light_gray).with_background(bg_context),
                    styles: TextStyleConfig::default(),
                    options: HashMap::new(),
                },
                usage: SegmentItemConfig {
                    id: super::segment::SegmentId::Usage,
                    enabled: true,
                    icon: IconConfig::new("📊", "\u{f0a9e}"),
                    colors: ColorConfig::new(light_gray, light_gray).with_background(bg_usage),
                    styles: TextStyleConfig::default(),
                    options: HashMap::new(),
                },
            },
        }
    }

    /// Powerline Light 主题
    pub fn get_powerline_light() -> CxLineConfig {
        let black = AnsiColor::rgb(0, 0, 0);
        let white = AnsiColor::rgb(255, 255, 255);

        let bg_model = AnsiColor::rgb(135, 206, 235);
        let bg_dir = AnsiColor::rgb(255, 107, 71);
        let bg_git = AnsiColor::rgb(79, 179, 217);
        let bg_context = AnsiColor::rgb(107, 114, 128);
        let bg_usage = AnsiColor::rgb(40, 167, 69);

        CxLineConfig {
            enabled: true,
            theme: "powerline-light".to_string(),
            style: StyleMode::Powerline,
            separator: "\u{e0b0}".to_string(),
            segments: SegmentsConfig {
                model: SegmentItemConfig {
                    id: super::segment::SegmentId::Model,
                    enabled: true,
                    icon: IconConfig::new("🤖", "\u{e26d}"),
                    colors: ColorConfig::new(black, black).with_background(bg_model),
                    styles: TextStyleConfig::default(),
                    options: HashMap::new(),
                },
                directory: SegmentItemConfig {
                    id: super::segment::SegmentId::Directory,
                    enabled: true,
                    icon: IconConfig::new("📁", "\u{f024b}"),
                    colors: ColorConfig::new(white, white).with_background(bg_dir),
                    styles: TextStyleConfig::default(),
                    options: HashMap::new(),
                },
                git: SegmentItemConfig {
                    id: super::segment::SegmentId::Git,
                    enabled: true,
                    icon: IconConfig::new("🌿", "\u{f02a2}"),
                    colors: ColorConfig::new(white, white).with_background(bg_git),
                    styles: TextStyleConfig::default(),
                    options: HashMap::new(),
                },
                context: SegmentItemConfig {
                    id: super::segment::SegmentId::Context,
                    enabled: true,
                    icon: IconConfig::new("⚡️", "\u{f49b}"),
                    colors: ColorConfig::new(white, white).with_background(bg_context),
                    styles: TextStyleConfig::default(),
                    options: HashMap::new(),
                },
                usage: SegmentItemConfig {
                    id: super::segment::SegmentId::Usage,
                    enabled: true,
                    icon: IconConfig::new("📊", "\u{f0a9e}"),
                    colors: ColorConfig::new(white, white).with_background(bg_usage),
                    styles: TextStyleConfig::default(),
                    options: HashMap::new(),
                },
            },
        }
    }

    /// Powerline Rose Pine 主题
    pub fn get_powerline_rose_pine() -> CxLineConfig {
        let rose = AnsiColor::rgb(235, 188, 186);
        let iris = AnsiColor::rgb(196, 167, 231);
        let foam = AnsiColor::rgb(156, 207, 216);
        let subtle = AnsiColor::rgb(224, 222, 244);
        let gold = AnsiColor::rgb(246, 193, 119);

        let bg_model = AnsiColor::rgb(25, 23, 36);
        let bg_dir = AnsiColor::rgb(38, 35, 58);
        let bg_git = AnsiColor::rgb(31, 29, 46);
        let bg_context = AnsiColor::rgb(82, 79, 103);
        let bg_usage = AnsiColor::rgb(35, 33, 54);

        CxLineConfig {
            enabled: true,
            theme: "powerline-rose-pine".to_string(),
            style: StyleMode::Powerline,
            separator: "\u{e0b0}".to_string(),
            segments: SegmentsConfig {
                model: SegmentItemConfig {
                    id: super::segment::SegmentId::Model,
                    enabled: true,
                    icon: IconConfig::new("🤖", "\u{e26d}"),
                    colors: ColorConfig::new(rose, rose).with_background(bg_model),
                    styles: TextStyleConfig::default(),
                    options: HashMap::new(),
                },
                directory: SegmentItemConfig {
                    id: super::segment::SegmentId::Directory,
                    enabled: true,
                    icon: IconConfig::new("📁", "\u{f024b}"),
                    colors: ColorConfig::new(iris, iris).with_background(bg_dir),
                    styles: TextStyleConfig::default(),
                    options: HashMap::new(),
                },
                git: SegmentItemConfig {
                    id: super::segment::SegmentId::Git,
                    enabled: true,
                    icon: IconConfig::new("🌿", "\u{f02a2}"),
                    colors: ColorConfig::new(foam, foam).with_background(bg_git),
                    styles: TextStyleConfig::default(),
                    options: HashMap::new(),
                },
                context: SegmentItemConfig {
                    id: super::segment::SegmentId::Context,
                    enabled: true,
                    icon: IconConfig::new("⚡️", "\u{f49b}"),
                    colors: ColorConfig::new(subtle, subtle).with_background(bg_context),
                    styles: TextStyleConfig::default(),
                    options: HashMap::new(),
                },
                usage: SegmentItemConfig {
                    id: super::segment::SegmentId::Usage,
                    enabled: true,
                    icon: IconConfig::new("📊", "\u{f0a9e}"),
                    colors: ColorConfig::new(gold, gold).with_background(bg_usage),
                    styles: TextStyleConfig::default(),
                    options: HashMap::new(),
                },
            },
        }
    }

    /// Powerline Tokyo Night 主题
    pub fn get_powerline_tokyo_night() -> CxLineConfig {
        let magenta = AnsiColor::rgb(252, 167, 234);
        let blue = AnsiColor::rgb(130, 170, 255);
        let green = AnsiColor::rgb(195, 232, 141);
        let lavender = AnsiColor::rgb(192, 202, 245);
        let orange = AnsiColor::rgb(224, 175, 104);

        let bg_model = AnsiColor::rgb(25, 27, 41);
        let bg_dir = AnsiColor::rgb(47, 51, 77);
        let bg_git = AnsiColor::rgb(30, 32, 48);
        let bg_context = AnsiColor::rgb(61, 89, 161);
        let bg_usage = AnsiColor::rgb(36, 40, 59);

        CxLineConfig {
            enabled: true,
            theme: "powerline-tokyo-night".to_string(),
            style: StyleMode::Powerline,
            separator: "\u{e0b0}".to_string(),
            segments: SegmentsConfig {
                model: SegmentItemConfig {
                    id: super::segment::SegmentId::Model,
                    enabled: true,
                    icon: IconConfig::new("🤖", "\u{e26d}"),
                    colors: ColorConfig::new(magenta, magenta).with_background(bg_model),
                    styles: TextStyleConfig::default(),
                    options: HashMap::new(),
                },
                directory: SegmentItemConfig {
                    id: super::segment::SegmentId::Directory,
                    enabled: true,
                    icon: IconConfig::new("📁", "\u{f024b}"),
                    colors: ColorConfig::new(blue, blue).with_background(bg_dir),
                    styles: TextStyleConfig::default(),
                    options: HashMap::new(),
                },
                git: SegmentItemConfig {
                    id: super::segment::SegmentId::Git,
                    enabled: true,
                    icon: IconConfig::new("🌿", "\u{f02a2}"),
                    colors: ColorConfig::new(green, green).with_background(bg_git),
                    styles: TextStyleConfig::default(),
                    options: HashMap::new(),
                },
                context: SegmentItemConfig {
                    id: super::segment::SegmentId::Context,
                    enabled: true,
                    icon: IconConfig::new("⚡️", "\u{f49b}"),
                    colors: ColorConfig::new(lavender, lavender).with_background(bg_context),
                    styles: TextStyleConfig::default(),
                    options: HashMap::new(),
                },
                usage: SegmentItemConfig {
                    id: super::segment::SegmentId::Usage,
                    enabled: true,
                    icon: IconConfig::new("📊", "\u{f0a9e}"),
                    colors: ColorConfig::new(orange, orange).with_background(bg_usage),
                    styles: TextStyleConfig::default(),
                    options: HashMap::new(),
                },
            },
        }
    }
}
