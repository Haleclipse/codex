// Git Segment - 显示 Git 分支和状态
// 搬迁自 CCometixLine

use crate::statusline::GitPreviewData;
use crate::statusline::StatusLineContext;
use crate::statusline::segment::Segment;
use crate::statusline::segment::SegmentData;
use crate::statusline::segment::SegmentId;
use std::path::Path;
use std::process::Command;

/// Git 状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GitStatus {
    Clean,
    Dirty,
    Conflicts,
}

/// Git 信息
#[derive(Debug)]
pub struct GitInfo {
    pub branch: String,
    pub status: GitStatus,
    pub ahead: u32,
    pub behind: u32,
}

pub struct GitSegment;

impl GitSegment {
    fn get_git_info(&self, working_dir: &Path) -> Option<GitInfo> {
        let working_dir = working_dir.to_string_lossy();

        if !self.is_git_repository(&working_dir) {
            return None;
        }

        let branch = self
            .get_branch(&working_dir)
            .unwrap_or_else(|| "detached".to_string());
        let status = self.get_status(&working_dir);
        let (ahead, behind) = self.get_ahead_behind(&working_dir);

        Some(GitInfo {
            branch,
            status,
            ahead,
            behind,
        })
    }

    fn is_git_repository(&self, working_dir: &str) -> bool {
        Command::new("git")
            .args(["--no-optional-locks", "rev-parse", "--git-dir"])
            .current_dir(working_dir)
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }

    fn get_branch(&self, working_dir: &str) -> Option<String> {
        // 首先尝试 --show-current
        if let Ok(output) = Command::new("git")
            .args(["--no-optional-locks", "branch", "--show-current"])
            .current_dir(working_dir)
            .output()
            && output.status.success()
        {
            let branch = String::from_utf8(output.stdout).ok()?.trim().to_string();
            if !branch.is_empty() {
                return Some(branch);
            }
        }

        // 回退到 symbolic-ref
        if let Ok(output) = Command::new("git")
            .args(["--no-optional-locks", "symbolic-ref", "--short", "HEAD"])
            .current_dir(working_dir)
            .output()
            && output.status.success()
        {
            let branch = String::from_utf8(output.stdout).ok()?.trim().to_string();
            if !branch.is_empty() {
                return Some(branch);
            }
        }

        None
    }

    fn get_status(&self, working_dir: &str) -> GitStatus {
        let output = Command::new("git")
            .args(["--no-optional-locks", "status", "--porcelain"])
            .current_dir(working_dir)
            .output();

        match output {
            Ok(output) if output.status.success() => {
                let status_text = String::from_utf8(output.stdout).unwrap_or_default();

                if status_text.trim().is_empty() {
                    return GitStatus::Clean;
                }

                // 检查冲突标记
                if status_text.contains("UU")
                    || status_text.contains("AA")
                    || status_text.contains("DD")
                {
                    GitStatus::Conflicts
                } else {
                    GitStatus::Dirty
                }
            }
            _ => GitStatus::Clean,
        }
    }

    fn get_ahead_behind(&self, working_dir: &str) -> (u32, u32) {
        let ahead = self.get_commit_count(working_dir, "@{u}..HEAD");
        let behind = self.get_commit_count(working_dir, "HEAD..@{u}");
        (ahead, behind)
    }

    fn get_commit_count(&self, working_dir: &str, range: &str) -> u32 {
        let output = Command::new("git")
            .args(["--no-optional-locks", "rev-list", "--count", range])
            .current_dir(working_dir)
            .output();

        match output {
            Ok(output) if output.status.success() => String::from_utf8(output.stdout)
                .ok()
                .and_then(|s| s.trim().parse().ok())
                .unwrap_or(0),
            _ => 0,
        }
    }

    pub(crate) fn collect_preview(&self, cwd: &Path) -> Option<GitPreviewData> {
        let git_info = self.get_git_info(cwd)?;
        let status = match git_info.status {
            GitStatus::Clean => "✓",
            GitStatus::Dirty => "●",
            GitStatus::Conflicts => "⚠",
        };

        Some(GitPreviewData {
            branch: git_info.branch,
            status: status.to_string(),
            ahead: git_info.ahead,
            behind: git_info.behind,
        })
    }
}

impl Segment for GitSegment {
    fn collect(&self, ctx: &StatusLineContext) -> Option<SegmentData> {
        // 如果有预览数据，使用预览数据
        if let Some(preview) = &ctx.git_preview {
            if preview.branch.is_empty() && preview.status.is_empty() {
                return None;
            }
            let primary = preview.branch.clone();
            let mut status_parts = Vec::new();
            status_parts.push(preview.status.clone());
            if preview.ahead > 0 {
                status_parts.push(format!("↑{}", preview.ahead));
            }
            if preview.behind > 0 {
                status_parts.push(format!("↓{}", preview.behind));
            }
            let secondary = status_parts.join(" ");
            return Some(
                SegmentData::new(primary)
                    .with_secondary(secondary)
                    .with_metadata("branch", &preview.branch)
                    .with_metadata("status", &preview.status)
                    .with_metadata("ahead", preview.ahead.to_string())
                    .with_metadata("behind", preview.behind.to_string()),
            );
        }

        let git_info = self.get_git_info(ctx.cwd)?;

        let primary = git_info.branch.clone();
        let mut status_parts = Vec::new();

        // 状态符号
        match git_info.status {
            GitStatus::Clean => status_parts.push("✓".to_string()),
            GitStatus::Dirty => status_parts.push("●".to_string()),
            GitStatus::Conflicts => status_parts.push("⚠".to_string()),
        }

        // ahead/behind
        if git_info.ahead > 0 {
            status_parts.push(format!("↑{}", git_info.ahead));
        }
        if git_info.behind > 0 {
            status_parts.push(format!("↓{}", git_info.behind));
        }

        let secondary = status_parts.join(" ");

        Some(
            SegmentData::new(primary)
                .with_secondary(secondary)
                .with_metadata("branch", &git_info.branch)
                .with_metadata("status", format!("{:?}", git_info.status))
                .with_metadata("ahead", git_info.ahead.to_string())
                .with_metadata("behind", git_info.behind.to_string()),
        )
    }

    fn id(&self) -> SegmentId {
        SegmentId::Git
    }
}
