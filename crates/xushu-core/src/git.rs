use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use crate::error::CoreError;
use crate::runner::{execute_command, ExecutionStatus, RunControl};

const GIT_TIMEOUT: Duration = Duration::from_secs(5);
const GIT_OUTPUT_LIMIT: usize = 1024 * 1024;

/// 只读 Git 状态。不包含文件列表或源码内容。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitStatus {
    pub root_path: PathBuf,
    pub branch: Option<String>,
    pub head: Option<String>,
    pub dirty: bool,
    pub detached: bool,
    pub unborn: bool,
}

pub(crate) struct GitRawOutput {
    pub success: bool,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub stdout_truncated: bool,
}

/// 以可执行文件 + 参数数组调用本机 `git`，不经过 shell。
///
/// 对每条命令设置 5 秒上限；stdout/stderr 由独立线程读取，避免大输出填满管道。
pub fn inspect(path: &Path) -> Result<GitStatus, CoreError> {
    let selected = path
        .canonicalize()
        .map_err(|_| CoreError::Validation("项目路径不存在或不可访问".to_owned()))?;

    if !selected.is_dir() {
        return Err(CoreError::Validation("项目路径不是目录".to_owned()));
    }

    let inside = run_git(&selected, &["rev-parse", "--is-inside-work-tree"])?;
    if !inside.success || inside.stdout != "true" {
        return Err(CoreError::Validation(
            "所选路径不是有效的 Git 工作树".to_owned(),
        ));
    }

    let top_level = run_git(&selected, &["rev-parse", "--show-toplevel"])?;
    if !top_level.success || top_level.stdout.is_empty() {
        return Err(CoreError::Git("无法读取 Git 根目录".to_owned()));
    }

    let root_path = PathBuf::from(top_level.stdout.trim());
    let root_path = root_path
        .canonicalize()
        .map_err(|_| CoreError::Git("Git 根目录不可访问".to_owned()))?;
    let root_path = normalize_display_path(root_path);

    let branch_output = run_git(&root_path, &["symbolic-ref", "--quiet", "--short", "HEAD"])?;
    let branch = if branch_output.success && !branch_output.stdout.is_empty() {
        Some(branch_output.stdout)
    } else {
        None
    };

    let head_output = run_git(&root_path, &["rev-parse", "--verify", "HEAD"])?;
    let head = if head_output.success && !head_output.stdout.is_empty() {
        Some(head_output.stdout)
    } else {
        None
    };

    let status_output = run_git(
        &root_path,
        &["status", "--porcelain=v1", "--untracked-files=normal"],
    )?;
    if !status_output.success {
        return Err(CoreError::Git(format!(
            "git status 失败: {}",
            status_output.stderr
        )));
    }

    let dirty = !status_output.stdout.is_empty();
    let unborn = head.is_none();
    let detached = branch.is_none() && head.is_some();

    Ok(GitStatus {
        root_path,
        branch,
        head,
        dirty,
        detached,
        unborn,
    })
}

struct GitOutput {
    success: bool,
    stdout: String,
    stderr: String,
}

fn run_git(path: &Path, args: &[&str]) -> Result<GitOutput, CoreError> {
    let output = run_git_raw(path, args, GIT_TIMEOUT, GIT_OUTPUT_LIMIT)?;
    Ok(GitOutput {
        success: output.success,
        stdout: String::from_utf8_lossy(&output.stdout).trim().to_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).trim().to_owned(),
    })
}

/// 运行只读 Git 命令；输出超过 `max_bytes` 时继续排空管道但只保留前缀。
pub(crate) fn run_git_raw(
    path: &Path,
    args: &[&str],
    timeout: Duration,
    max_bytes: usize,
) -> Result<GitRawOutput, CoreError> {
    let mut command = Command::new("git");
    command
        .args(["--no-optional-locks", "-c", "core.fsmonitor=false", "-C"])
        .arg(path)
        .args(args);
    let result = execute_command(command, timeout, max_bytes, &RunControl::new());
    if result.execution_status != ExecutionStatus::Completed {
        return Err(CoreError::Git(
            result.note.unwrap_or_else(|| "Git 命令未完成".to_owned()),
        ));
    }
    Ok(GitRawOutput {
        success: result.exit_code == Some(0),
        stdout: result.stdout,
        stderr: result.stderr,
        stdout_truncated: result.stdout_truncated,
    })
}

pub(crate) fn normalize_display_path(path: PathBuf) -> PathBuf {
    let text = path.to_string_lossy();
    if let Some(rest) = text.strip_prefix("\\\\?\\") {
        if !rest.starts_with("UNC\\") {
            return PathBuf::from(rest);
        }
    }
    path
}
