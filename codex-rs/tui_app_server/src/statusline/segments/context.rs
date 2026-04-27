// Context Segment - 显示上下文窗口使用情况

use crate::statusline::StatusLineContext;
use crate::statusline::segment::Segment;
use crate::statusline::segment::SegmentData;
use crate::statusline::segment::SegmentId;

pub struct ContextSegment;

impl Segment for ContextSegment {
    fn collect(&self, ctx: &StatusLineContext) -> Option<SegmentData> {
        // 如果有 token 数和窗口大小，计算使用占比
        // 使用占比 = (已使用 tokens / 窗口大小) * 100
        let used_percent = match (ctx.context_used_tokens, ctx.context_window_size) {
            (Some(used), Some(window)) if window > 0 => {
                Some((used as f64 / window as f64 * 100.0) as i64)
            }
            _ => None,
        };

        // 根据数据情况显示
        match (used_percent, ctx.context_used_tokens) {
            (Some(percent), Some(used_tokens)) => {
                // 格式: {percentage}% · {tokens} tokens
                let percentage_display = format!("{percent}%");
                let tokens_display = format!("{} tokens", format_tokens(used_tokens));
                let display = format!("{percentage_display} · {tokens_display}");
                Some(
                    SegmentData::new(display)
                        .with_metadata("percent", percent.to_string())
                        .with_metadata("tokens", used_tokens.to_string())
                        .with_metadata("type", "full"),
                )
            }
            (None, Some(used_tokens)) => {
                // 只有 token 数（没有窗口大小，无法计算百分比）
                let display = format!("{} tokens", format_tokens(used_tokens));
                Some(
                    SegmentData::new(display)
                        .with_metadata("tokens", used_tokens.to_string())
                        .with_metadata("type", "tokens"),
                )
            }
            _ => {
                // 没有数据时显示占位符
                Some(
                    SegmentData::new("- · - tokens".to_string())
                        .with_metadata("percent", "-".to_string())
                        .with_metadata("tokens", "-".to_string())
                        .with_metadata("type", "placeholder"),
                )
            }
        }
    }

    fn id(&self) -> SegmentId {
        SegmentId::Context
    }
}

/// 格式化 token 数量
fn format_tokens(tokens: i64) -> String {
    if tokens >= 1_000_000 {
        format!("{:.1}M", tokens as f64 / 1_000_000.0)
    } else if tokens >= 1_000 {
        format!("{:.1}k", tokens as f64 / 1_000.0)
    } else {
        tokens.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_tokens() {
        assert_eq!(format_tokens(500), "500");
        assert_eq!(format_tokens(1500), "1.5k");
        assert_eq!(format_tokens(150000), "150.0k");
        assert_eq!(format_tokens(1500000), "1.5M");
    }
}
