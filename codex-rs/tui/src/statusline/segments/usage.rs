// Usage Segment - 显示 Rate Limit 使用情况

use crate::statusline::StatusLineContext;
use crate::statusline::segment::Segment;
use crate::statusline::segment::SegmentData;
use crate::statusline::segment::SegmentId;

pub struct UsageSegment;

impl Segment for UsageSegment {
    fn collect(&self, ctx: &StatusLineContext) -> Option<SegmentData> {
        // 5h limit 用于百分比显示
        let hourly_percent = ctx.hourly_rate_limit_percent?;
        // Weekly limit 用于圆圈进度
        let weekly_percent = ctx.weekly_rate_limit_percent.unwrap_or(hourly_percent);

        // 格式化百分比 (显示 5h limit)
        let display = format!("{hourly_percent:.0}%");

        // 动态图标：根据周限使用率选择不同的圆形切片图标
        let dynamic_icon = get_circle_icon(weekly_percent / 100.0);

        let mut data = SegmentData::new(display)
            .with_metadata("hourly_percent", format!("{hourly_percent:.1}"))
            .with_metadata("weekly_percent", format!("{weekly_percent:.1}"))
            .with_metadata("dynamic_icon", dynamic_icon);

        // 添加周限重置时间
        if let Some(ref resets_at) = ctx.weekly_rate_limit_resets_at {
            data = data
                .with_secondary(format!("· {resets_at}"))
                .with_metadata("resets_at", resets_at);
        }

        Some(data)
    }

    fn id(&self) -> SegmentId {
        SegmentId::Usage
    }
}

/// 根据使用率获取圆形切片图标
/// 使用 Nerd Font Material Design Icons
fn get_circle_icon(utilization: f64) -> String {
    let percent = (utilization * 100.0) as u8;
    match percent {
        0..=12 => "\u{f0a9e}".to_string(),  // circle_slice_1
        13..=25 => "\u{f0a9f}".to_string(), // circle_slice_2
        26..=37 => "\u{f0aa0}".to_string(), // circle_slice_3
        38..=50 => "\u{f0aa1}".to_string(), // circle_slice_4
        51..=62 => "\u{f0aa2}".to_string(), // circle_slice_5
        63..=75 => "\u{f0aa3}".to_string(), // circle_slice_6
        76..=87 => "\u{f0aa4}".to_string(), // circle_slice_7
        _ => "\u{f0aa5}".to_string(),       // circle_slice_8 (full)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_circle_icon() {
        // 测试边界值
        assert_eq!(get_circle_icon(0.0), "\u{f0a9e}");
        assert_eq!(get_circle_icon(0.5), "\u{f0aa1}");
        assert_eq!(get_circle_icon(1.0), "\u{f0aa5}");
    }
}
