// Directory Segment - 显示当前工作目录名称

use crate::statusline::StatusLineContext;
use crate::statusline::segment::{Segment, SegmentData, SegmentId};

pub struct DirectorySegment;

impl Segment for DirectorySegment {
    fn collect(&self, ctx: &StatusLineContext) -> Option<SegmentData> {
        let cwd = ctx.cwd;
        let dir_name = extract_directory_name(cwd);

        if dir_name.is_empty() {
            return None;
        }

        Some(SegmentData::new(&dir_name).with_metadata("full_path", cwd.to_string_lossy()))
    }

    fn id(&self) -> SegmentId {
        SegmentId::Directory
    }
}

/// 提取目录名称
/// 支持 Unix 和 Windows 路径
fn extract_directory_name(path: &std::path::Path) -> String {
    // 获取最后一个组件（目录名）
    path.file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| {
            // 如果是根目录，返回 "/"
            if path.as_os_str().is_empty() {
                String::new()
            } else {
                "/".to_string()
            }
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_extract_directory_name() {
        // Unix 路径测试
        assert_eq!(
            extract_directory_name(Path::new("/home/user/projects/codex")),
            "codex"
        );
        assert_eq!(extract_directory_name(Path::new("/home/user")), "user");

        // 根目录
        assert_eq!(extract_directory_name(Path::new("/")), "/");

        // 相对路径
        assert_eq!(extract_directory_name(Path::new("some/path")), "path");
    }
}
