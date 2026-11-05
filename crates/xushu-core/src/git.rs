use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use crate::error::CoreError;

const GIT_TIMEOUT: Duration = Duration::from_secs(5);

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

struct GitOutput {
    success: bool,
    stdout: String,
    stderr: String,
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

fn run_git(path: &Path, args: &[&str]) -> Result<GitOutput, CoreError> {
    let mut child = Command::new("git")
        .arg("-C")
        .arg(path)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| CoreError::Git(format!("无法启动 git: {error}")))?;

    let mut stdout_pipe = child
        .stdout
        .take()
        .ok_or_else(|| CoreError::Git("无法读取 git stdout".to_owned()))?;
    let mut stderr_pipe = child
        .stderr
        .take()
        .ok_or_else(|| CoreError::Git("无法读取 git stderr".to_owned()))?;

    let stdout_handle = thread::spawn(move || {
        let mut buffer = Vec::new();
        let _ = stdout_pipe.read_to_end(&mut buffer);
        buffer
    });
    let stderr_handle = thread::spawn(move || {
        let mut buffer = Vec::new();
        let _ = stderr_pipe.read_to_end(&mut buffer);
        buffer
    });

    let deadline = Instant::now() + GIT_TIMEOUT;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if Instant::now() >= deadline => {
                let _ = child.kill();
                let _ = child.wait();
                let _ = stdout_handle.join();
                let _ = stderr_handle.join();
                return Err(CoreError::Git("Git 命令超时".to_owned()));
            }
            Ok(None) => thread::sleep(Duration::from_millis(20)),
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                let _ = stdout_handle.join();
                let _ = stderr_handle.join();
                return Err(CoreError::Git(format!("等待 Git 失败: {error}")));
            }
        }
    };

    let stdout = stdout_handle.join().unwrap_or_default();
    let stderr = stderr_handle.join().unwrap_or_default();

    Ok(GitOutput {
        success: status.success(),
        stdout: String::from_utf8_lossy(&stdout).trim().to_owned(),
        stderr: String::from_utf8_lossy(&stderr).trim().to_owned(),
    })
}

fn normalize_display_path(path: PathBuf) -> PathBuf {
    let text = path.to_string_lossy();
    if let Some(rest) = text.strip_prefix("\\\\?\\") {
        if !rest.starts_with("UNC\\") {
            return PathBuf::from(rest);
        }
    }
    path
}
