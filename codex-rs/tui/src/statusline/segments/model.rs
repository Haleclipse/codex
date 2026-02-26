// Model Segment - 显示当前模型名称

use crate::statusline::StatusLineContext;
use crate::statusline::segment::Segment;
use crate::statusline::segment::SegmentData;
use crate::statusline::segment::SegmentId;
use codex_protocol::openai_models::ReasoningEffort;

pub struct ModelSegment;

impl Segment for ModelSegment {
    fn collect(&self, ctx: &StatusLineContext) -> Option<SegmentData> {
        let model_name = ctx.model_name;
        if model_name.is_empty() {
            return None;
        }

        // 简化模型名称显示
        let display_name = simplify_model_name(model_name);

        // Append reasoning effort suffix if present
        let display_name = if let Some(effort) = ctx.reasoning_effort {
            let effort_suffix = reasoning_effort_suffix(effort);
            if effort_suffix.is_empty() {
                display_name
            } else {
                format!("{display_name} {effort_suffix}")
            }
        } else {
            display_name
        };

        Some(SegmentData::new(display_name).with_metadata("model_id", model_name))
    }

    fn id(&self) -> SegmentId {
        SegmentId::Model
    }
}

/// Get short suffix for reasoning effort level
fn reasoning_effort_suffix(effort: ReasoningEffort) -> &'static str {
    match effort {
        ReasoningEffort::None => "",
        ReasoningEffort::Minimal => "·min",
        ReasoningEffort::Low => "·lo",
        ReasoningEffort::Medium => "·med",
        ReasoningEffort::High => "·hi",
        ReasoningEffort::XHigh => "·xhi",
    }
}

/// 简化模型名称
/// 例如：gpt-4o-2024-08-06 -> gpt-4o
///       claude-3-5-sonnet-20241022 -> claude-3.5-sonnet
fn simplify_model_name(name: &str) -> String {
    // 移除日期后缀
    let name = if let Some(pos) = name.rfind("-20") {
        // 检查是否是日期格式 -YYYYMMDD 或 -YYYY-MM-DD
        let suffix = &name[pos..];
        if suffix.len() >= 9
            && suffix[1..]
                .chars()
                .take(8)
                .all(|c| c.is_ascii_digit() || c == '-')
        {
            &name[..pos]
        } else {
            name
        }
    } else {
        name
    };

    // 常见模型名称映射（与 model_presets.rs 保持一致）
    match name {
        // 当前模型
        "gpt-5.3-codex" => "GPT 5.3 Codex".to_string(),
        "gpt-5.2-codex" => "GPT 5.2 Codex".to_string(),
        "gpt-5.1-codex-max" => "GPT 5.1 Codex Max".to_string(),
        "gpt-5.1-codex-mini" => "GPT 5.1 Codex Mini".to_string(),
        "gpt-5.2" => "GPT 5.2".to_string(),
        // Deprecated 模型
        "gpt-5-codex" => "GPT 5 Codex".to_string(),
        "gpt-5-codex-mini" => "GPT 5 Codex Mini".to_string(),
        "gpt-5.1-codex" => "GPT 5.1 Codex".to_string(),
        "gpt-5" => "GPT 5".to_string(),
        "gpt-5.1" => "GPT 5.1".to_string(),
        _ => name.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simplify_model_name() {
        // 测试日期后缀移除
        assert_eq!(
            simplify_model_name("gpt-5.2-codex-2025-01-15"),
            "GPT 5.2 Codex"
        );
        assert_eq!(
            simplify_model_name("gpt-5.1-codex-max-20250101"),
            "GPT 5.1 Codex Max"
        );
        // 测试模型名称映射
        assert_eq!(simplify_model_name("gpt-5.2-codex"), "GPT 5.2 Codex");
        assert_eq!(
            simplify_model_name("gpt-5.1-codex-max"),
            "GPT 5.1 Codex Max"
        );
        assert_eq!(simplify_model_name("gpt-5"), "GPT 5");
        // 测试无映射的模型
        assert_eq!(simplify_model_name("custom-model"), "custom-model");
    }
}
