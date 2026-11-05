use std::fs::{self, File};
use std::io::Read;
use std::path::{Component, Path, PathBuf};
use std::time::Duration;

use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::error::CoreError;
use crate::git::run_git_raw;

const GIT_COMMAND_TIMEOUT: Duration = Duration::from_secs(10);
const GIT_STATUS_TIMEOUT: Duration = Duration::from_secs(30);
const GIT_OUTPUT_LIMIT: usize = 64 * 1024;
const GIT_STATUS_LIMIT: usize = 4 * 1024 * 1024;
const MAX_CHANGED_FILES: usize = 2000;
const MAX_TOTAL_FILE_BYTES: u64 = 64 * 1024 * 1024;
const MAX_SINGLE_FILE_BYTES: u64 = 8 * 1024 * 1024;

/// Git HEAD、分支与工作区内容指纹。
///
/// 指纹覆盖 `git status` 列出的每个变更/未跟踪文件的路径、状态与内容哈希，
/// 因此未提交修改不会被 `dirty=true` 的粗粒度布尔值掩盖。指纹不完整时
/// 验证引擎必须拒绝据此产生自动 PASS。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CodeState {
    pub head: Option<String>,
    pub branch: Option<String>,
    pub detached: bool,
    pub unborn: bool,
    pub dirty: bool,
    pub fingerprint: Option<String>,
    pub fingerprint_complete: bool,
    pub changed_file_count: usize,
    pub note: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct StatusEntry {
    status: String,
    path: String,
    original_path: Option<String>,
}

/// 采集项目根目录当前的只读代码状态。
pub fn capture(root: &Path) -> Result<CodeState, CoreError> {
    let root = root
        .canonicalize()
        .map_err(|_| CoreError::Git("项目路径不可访问".to_owned()))?;
    if !root.is_dir() {
        return Err(CoreError::Git("项目路径不是目录".to_owned()));
    }

    let branch_output = run_git_raw(
        &root,
        &["symbolic-ref", "--quiet", "--short", "HEAD"],
        GIT_COMMAND_TIMEOUT,
        GIT_OUTPUT_LIMIT,
    )?;
    let branch = if branch_output.success {
        nonempty_text(&branch_output.stdout)
    } else {
        None
    };

    let head_output = run_git_raw(
        &root,
        &["rev-parse", "--verify", "HEAD"],
        GIT_COMMAND_TIMEOUT,
        GIT_OUTPUT_LIMIT,
    )?;
    let head = if head_output.success {
        nonempty_text(&head_output.stdout)
    } else {
        None
    };

    let status_output = run_git_raw(
        &root,
        &["status", "--porcelain=v1", "-z", "--untracked-files=all"],
        GIT_STATUS_TIMEOUT,
        GIT_STATUS_LIMIT,
    )?;
    if !status_output.success {
        return Err(CoreError::Git(format!(
            "git status 失败: {}",
            String::from_utf8_lossy(&status_output.stderr).trim()
        )));
    }

    let mut notes = Vec::new();
    let mut complete = true;

    if status_output.stdout_truncated {
        complete = false;
        notes.push("git status 输出超过采集上限".to_owned());
    }

    let entries = match parse_status(&status_output.stdout) {
        Ok(entries) => entries,
        Err(note) => {
            complete = false;
            notes.push(note);
            Vec::new()
        }
    };

    if entries.len() > MAX_CHANGED_FILES {
        complete = false;
        notes.push(format!(
            "变更文件数量 {} 超过指纹上限 {MAX_CHANGED_FILES}",
            entries.len()
        ));
    }

    let mut hasher = Sha256::new();
    hasher.update(b"xushu-code-state-v1\0");
    hash_field(&mut hasher, head.as_deref().unwrap_or(""));
    hash_field(&mut hasher, branch.as_deref().unwrap_or(""));
    hash_field(
        &mut hasher,
        if branch.is_none() && head.is_some() {
            "detached"
        } else {
            "branch"
        },
    );
    hash_field(&mut hasher, if head.is_none() { "unborn" } else { "born" });

    let mut sorted: Vec<&StatusEntry> = entries.iter().take(MAX_CHANGED_FILES).collect();
    sorted.sort_by(|left, right| {
        left.path
            .cmp(&right.path)
            .then_with(|| left.status.cmp(&right.status))
    });

    let mut total_bytes = 0_u64;
    for entry in &sorted {
        hasher.update(entry.status.as_bytes());
        hasher.update(b"\0");
        hash_field(&mut hasher, &entry.path);

        if let Some(original) = &entry.original_path {
            hash_field(&mut hasher, original);
        }

        let content = hash_entry(
            &root,
            &entry.path,
            &mut total_bytes,
            &mut complete,
            &mut notes,
        );
        hash_field(&mut hasher, &content);
    }

    let fingerprint = format!("sha256:{}", hex(&hasher.finalize()));
    let unborn = head.is_none();
    let detached = branch.is_none() && head.is_some();
    let dirty = !entries.is_empty();

    Ok(CodeState {
        head,
        branch,
        detached,
        unborn,
        dirty,
        fingerprint: Some(fingerprint),
        fingerprint_complete: complete,
        changed_file_count: entries.len(),
        note: if notes.is_empty() {
            None
        } else {
            Some(notes.join("；"))
        },
    })
}

fn parse_status(bytes: &[u8]) -> Result<Vec<StatusEntry>, String> {
    let fields: Vec<&[u8]> = bytes
        .split(|byte| *byte == 0)
        .filter(|field| !field.is_empty())
        .collect();

    let mut entries = Vec::new();
    let mut index = 0;
    while index < fields.len() {
        let field = fields[index];
        if field.len() < 4 {
            return Err("git status 条目格式无法解析".to_owned());
        }

        let status = String::from_utf8_lossy(&field[..2]).into_owned();
        let path = match std::str::from_utf8(&field[3..]) {
            Ok(path) => path.to_owned(),
            Err(_) => return Err("git status 包含非 UTF-8 路径".to_owned()),
        };

        let mut original_path = None;
        if status.contains('R') || status.contains('C') {
            index += 1;
            let original = fields
                .get(index)
                .ok_or_else(|| "git status 重命名条目缺少原路径".to_owned())?;
            original_path = Some(
                std::str::from_utf8(original)
                    .map_err(|_| "git status 包含非 UTF-8 路径".to_owned())?
                    .to_owned(),
            );
        }

        entries.push(StatusEntry {
            status,
            path,
            original_path,
        });
        index += 1;
    }

    Ok(entries)
}

fn hash_entry(
    root: &Path,
    raw_path: &str,
    total_bytes: &mut u64,
    complete: &mut bool,
    notes: &mut Vec<String>,
) -> String {
    let Some(relative) = safe_relative_path(raw_path) else {
        *complete = false;
        notes.push(format!("路径越界或格式无效: {raw_path}"));
        return "invalid-path".to_owned();
    };
    let absolute = root.join(&relative);

    let metadata = match fs::symlink_metadata(&absolute) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return "missing".to_owned();
        }
        Err(_) => {
            *complete = false;
            notes.push(format!("无法读取文件元数据: {raw_path}"));
            return "unreadable".to_owned();
        }
    };

    let file_type = metadata.file_type();
    if file_type.is_symlink() {
        return match fs::read_link(&absolute) {
            Ok(target) => format!("symlink:{}", target.to_string_lossy()),
            Err(_) => {
                *complete = false;
                notes.push(format!("无法读取符号链接: {raw_path}"));
                "symlink-unreadable".to_owned()
            }
        };
    }

    if file_type.is_dir() {
        *complete = false;
        notes.push(format!("目录内容未纳入指纹（可能是子模块）: {raw_path}"));
        return "directory".to_owned();
    }

    if !file_type.is_file() {
        *complete = false;
        notes.push(format!("不支持的文件类型: {raw_path}"));
        return "unsupported".to_owned();
    }

    if metadata.len() > MAX_SINGLE_FILE_BYTES {
        *complete = false;
        notes.push(format!("文件超过单文件指纹上限: {raw_path}"));
        return "too-large".to_owned();
    }

    if *total_bytes + metadata.len() > MAX_TOTAL_FILE_BYTES {
        *complete = false;
        notes.push("工作区变更内容超过指纹总字节上限".to_owned());
        return "over-budget".to_owned();
    }
    *total_bytes += metadata.len();

    match hash_file(&absolute) {
        Ok(digest) => format!("file:{digest}"),
        Err(_) => {
            *complete = false;
            notes.push(format!("无法读取文件内容: {raw_path}"));
            "unreadable".to_owned()
        }
    }
}

fn safe_relative_path(raw: &str) -> Option<PathBuf> {
    let path = Path::new(raw);
    if path.is_absolute() {
        return None;
    }

    let mut relative = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Normal(segment) => relative.push(segment),
            Component::CurDir => {}
            _ => return None,
        }
    }

    if relative.as_os_str().is_empty() {
        None
    } else {
        Some(relative)
    }
}

fn hash_file(path: &Path) -> std::io::Result<String> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];

    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }

    Ok(hex(&hasher.finalize()))
}

fn nonempty_text(bytes: &[u8]) -> Option<String> {
    let text = String::from_utf8_lossy(bytes).trim().to_owned();
    if text.is_empty() {
        None
    } else {
        Some(text)
    }
}

fn hash_field(hasher: &mut Sha256, value: &str) {
    hasher.update((value.len() as u64).to_le_bytes());
    hasher.update(value.as_bytes());
}

fn hex(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        use std::fmt::Write;
        let _ = write!(output, "{byte:02x}");
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_absolute_and_parent_paths() {
        assert!(safe_relative_path("../secret").is_none());
        assert!(safe_relative_path("C:/Windows").is_none());
        assert!(safe_relative_path("dir/../secret").is_none());
        assert_eq!(
            safe_relative_path("dir/file.txt"),
            Some(PathBuf::from("dir").join("file.txt"))
        );
    }

    #[test]
    fn parses_rename_status_with_nul_separator() {
        let raw = b" M b.txt\0R  renamed.txt\0a.txt\0?? new.txt\0";
        let entries = parse_status(raw).expect("parse");
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[1].path, "renamed.txt");
        assert_eq!(entries[1].original_path.as_deref(), Some("a.txt"));
    }
}
