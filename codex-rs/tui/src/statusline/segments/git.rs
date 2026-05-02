// Git Segment - displays git branch and status from async preview data

use crate::statusline::GitPreviewData;
use crate::statusline::StatusLineContext;
use crate::statusline::segment::Segment;
use crate::statusline::segment::SegmentData;
use crate::statusline::segment::SegmentId;
use std::path::Path;
use std::process::Command;

pub struct GitSegment;

impl GitSegment {
    /// Collect git info by running git commands. Only called from async
    /// `spawn_blocking` context via `collect_preview` — never on the render thread.
    fn get_git_info(&self, cwd: &Path) -> Option<GitInfo> {
        let wd = cwd.to_string_lossy();

        if !Command::new("git")
            .args(["--no-optional-locks", "rev-parse", "--git-dir"])
            .current_dir(wd.as_ref())
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
        {
            return None;
        }

        let branch = get_branch(&wd).unwrap_or_else(|| "detached".to_string());
        let status = get_status(&wd);
        let (ahead, behind) = get_ahead_behind(&wd);

        Some(GitInfo {
            branch,
            status,
            ahead,
            behind,
        })
    }

    /// Async-safe entry point: runs blocking git commands, returns preview data.
    /// Called exclusively from `tokio::task::spawn_blocking`.
    pub(crate) fn collect_preview(&self, cwd: &Path) -> Option<GitPreviewData> {
        let info = self.get_git_info(cwd)?;
        let status = match info.status {
            GitStatus::Clean => "✓",
            GitStatus::Dirty => "●",
            GitStatus::Conflicts => "⚠",
        };
        Some(GitPreviewData {
            branch: info.branch,
            status: status.to_string(),
            ahead: info.ahead,
            behind: info.behind,
        })
    }
}

impl Segment for GitSegment {
    fn collect(&self, ctx: &StatusLineContext) -> Option<SegmentData> {
        // @cometix: only render from async preview data — never run blocking
        // git commands on the render thread.
        let preview = ctx.git_preview.as_ref()?;
        if preview.branch.is_empty() && preview.status.is_empty() {
            return None;
        }
        let primary = preview.branch.clone();
        let mut parts = Vec::new();
        parts.push(preview.status.clone());
        if preview.ahead > 0 {
            parts.push(format!("↑{}", preview.ahead));
        }
        if preview.behind > 0 {
            parts.push(format!("↓{}", preview.behind));
        }
        Some(
            SegmentData::new(primary)
                .with_secondary(parts.join(" "))
                .with_metadata("branch", &preview.branch)
                .with_metadata("status", &preview.status)
                .with_metadata("ahead", preview.ahead.to_string())
                .with_metadata("behind", preview.behind.to_string()),
        )
    }

    fn id(&self) -> SegmentId {
        SegmentId::Git
    }
}

// --- internal helpers (blocking, only called from spawn_blocking) ---

#[derive(Debug)]
struct GitInfo {
    branch: String,
    status: GitStatus,
    ahead: u32,
    behind: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GitStatus {
    Clean,
    Dirty,
    Conflicts,
}

fn get_branch(wd: &str) -> Option<String> {
    if let Ok(o) = Command::new("git")
        .args(["--no-optional-locks", "branch", "--show-current"])
        .current_dir(wd)
        .output()
        && o.status.success()
    {
        let b = String::from_utf8(o.stdout).ok()?.trim().to_string();
        if !b.is_empty() {
            return Some(b);
        }
    }
    if let Ok(o) = Command::new("git")
        .args(["--no-optional-locks", "symbolic-ref", "--short", "HEAD"])
        .current_dir(wd)
        .output()
        && o.status.success()
    {
        let b = String::from_utf8(o.stdout).ok()?.trim().to_string();
        if !b.is_empty() {
            return Some(b);
        }
    }
    None
}

fn get_status(wd: &str) -> GitStatus {
    match Command::new("git")
        .args(["--no-optional-locks", "status", "--porcelain"])
        .current_dir(wd)
        .output()
    {
        Ok(o) if o.status.success() => {
            let text = String::from_utf8(o.stdout).unwrap_or_default();
            if text.trim().is_empty() {
                GitStatus::Clean
            } else if text.contains("UU") || text.contains("AA") || text.contains("DD") {
                GitStatus::Conflicts
            } else {
                GitStatus::Dirty
            }
        }
        _ => GitStatus::Clean,
    }
}

fn get_ahead_behind(wd: &str) -> (u32, u32) {
    let count = |range: &str| -> u32 {
        Command::new("git")
            .args(["--no-optional-locks", "rev-list", "--count", range])
            .current_dir(wd)
            .output()
            .ok()
            .filter(|o| o.status.success())
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .and_then(|s| s.trim().parse().ok())
            .unwrap_or(0)
    };
    (count("@{u}..HEAD"), count("HEAD..@{u}"))
}
