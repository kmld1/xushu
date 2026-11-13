//! Claude Code 项目级 Hooks 接入。
//!
//! 三层状态严格分离：
//! - XUSHU 数据库中的期望状态（`desired_enabled`）；
//! - 项目 `.claude/settings.local.json` 中的实际安装状态（每次读取重新计算）；
//! - 最近一次真实 Hook 事件是否被成功导入（spool 与事件表）。
//!
//! 本模块从不修改任务状态、验收证据或完成门禁。Hook 事件只是观察记录。

mod ingest;
pub(crate) mod path_guard;
mod service;
mod settings;

use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use xushu_hook_protocol::{initialize_salt, is_link_or_reparse, SALT_FILE_NAME};

use crate::error::CoreError;

pub use service::ClaudeIntegrationService;

/// 面向隐私与数据保护页面的 Hook 运行时摘要；不包含 Salt 内容或事件原文。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaudePrivacySnapshot {
    pub pending_event_count: usize,
    pub pending_bytes: u64,
    pub salt_present: bool,
    pub salt_bytes: u64,
    pub inbox_available: bool,
}

pub const EVENT_RETENTION_DAYS: i64 = 30;
pub const MAX_EVENTS_PER_PROJECT: i64 = 5_000;
pub const MAX_SESSIONS_PER_PROJECT: i64 = 500;
pub const ACTIVITY_EVENT_LIMIT: i64 = 100;
pub const ACTIVITY_TOOL_CALL_LIMIT: i64 = 100;
pub const ACTIVITY_SESSION_LIMIT: i64 = 20;
pub const IMPORT_BATCH_LIMIT: usize = 200;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ClaudeInstallState {
    NotConfigured,
    Disabled,
    Installed,
    Partial,
    NeedsRepair,
    ConfigurationError,
    ReceiverUnavailable,
    ProjectUnavailable,
    Desynchronized,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaudeIntegrationStatus {
    pub project_id: String,
    pub project_name: String,
    pub project_archived: bool,
    pub desired_enabled: bool,
    pub has_record: bool,
    pub install_state: ClaudeInstallState,
    pub state_detail: Option<String>,
    pub settings_file: String,
    pub settings_exists: bool,
    pub managed_hook_count: usize,
    pub expected_hook_count: usize,
    pub other_hook_group_count: usize,
    pub hook_version: Option<String>,
    pub installed_at: Option<String>,
    pub receiver_changed: bool,
    pub hook_binary_available: bool,
    pub inbox_available: bool,
    pub pending_event_count: usize,
    pub last_event_at: Option<String>,
    pub last_event_type: Option<String>,
    pub last_import_at: Option<String>,
    pub uninstalled_at: Option<String>,
    pub last_error_code: Option<String>,
    pub last_error_at: Option<String>,
    pub event_count: i64,
    pub session_count: i64,
    pub running_tool_count: i64,
    pub retention_days: i64,
    pub max_events: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaudeHookPlannedEntry {
    pub event: String,
    pub matcher: Option<String>,
    pub command: String,
    pub args: Vec<String>,
    pub timeout_seconds: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaudeHookPreview {
    pub project_id: String,
    pub project_name: String,
    pub settings_file: String,
    pub settings_exists: bool,
    pub will_create_file: bool,
    pub managed_entries: Vec<ClaudeHookPlannedEntry>,
    pub managed_count_before: usize,
    pub other_hook_group_count: usize,
    pub backup_will_be_created: bool,
    pub receiver_available: bool,
    pub inbox_available: bool,
    pub preview_token: String,
    pub warnings: Vec<String>,
    pub blocked_reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaudeSessionSummary {
    pub id: String,
    pub session_ref: String,
    pub started_at: String,
    pub last_event_at: String,
    pub ended_at: Option<String>,
    pub start_source: String,
    pub end_reason: Option<String>,
    pub start_observed: bool,
    pub activity_state: String,
    pub event_count: i64,
    pub tool_call_count: i64,
    pub running_tool_count: i64,
    pub failed_tool_count: i64,
    pub unknown_tool_count: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaudeActivityEvent {
    pub id: String,
    pub session_id: String,
    pub session_ref: String,
    pub event_type: String,
    pub occurred_at: String,
    pub tool_name: Option<String>,
    pub tool_category: Option<String>,
    pub status: String,
    pub failure_kind: Option<String>,
    pub duration_ms: Option<i64>,
    pub source: Option<String>,
    pub end_reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaudeToolCallSummary {
    pub id: String,
    pub session_id: String,
    pub session_ref: String,
    pub tool_name: String,
    pub tool_category: String,
    pub status: String,
    pub failure_kind: Option<String>,
    pub started_at: Option<String>,
    pub finished_at: Option<String>,
    pub duration_ms: Option<i64>,
    pub start_observed: bool,
    pub finish_observed: bool,
    pub conflict: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaudeActivitySnapshot {
    pub project_id: String,
    pub generated_at: String,
    pub pending_event_count: usize,
    pub sessions: Vec<ClaudeSessionSummary>,
    pub events: Vec<ClaudeActivityEvent>,
    pub tool_calls: Vec<ClaudeToolCallSummary>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaudeImportReport {
    pub project_id: String,
    pub imported: usize,
    pub duplicates: usize,
    pub rejected: usize,
    pub ignored: usize,
    pub remaining: usize,
    pub last_error_code: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallClaudeHooksInput {
    pub project_id: String,
    pub preview_token: String,
    pub approved: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaudeActionInput {
    pub project_id: String,
    pub approved: bool,
}

/// 桌面端应用数据目录中的 Hook 运行时路径与盐。
#[derive(Clone)]
pub struct ClaudeRuntimePaths {
    inbox_dir: PathBuf,
    backup_dir: PathBuf,
    hook_binary: Option<PathBuf>,
    hook_version: String,
    salt: Option<Vec<u8>>,
}

impl ClaudeRuntimePaths {
    pub fn prepare(
        data_directory: &Path,
        hook_binary: Option<PathBuf>,
        hook_version: impl Into<String>,
    ) -> Result<Self, CoreError> {
        let inbox_dir = data_directory.join("hook-inbox");
        let backup_dir = data_directory.join("hook-backups");
        fs::create_dir_all(data_directory)?;
        let _data = path_guard::Directories::open(data_directory)?;
        create_runtime_directory(&backup_dir)?;
        let salt = match prepare_inbox(&inbox_dir) {
            Ok(salt) => Some(salt),
            Err(error) => {
                eprintln!("[xushu] Hook inbox unavailable: code={}", error.code());
                None
            }
        };

        Ok(Self {
            inbox_dir,
            backup_dir,
            hook_binary,
            hook_version: hook_version.into(),
            salt,
        })
    }

    pub(crate) fn inbox_dir(&self) -> &Path {
        &self.inbox_dir
    }

    pub(crate) fn backup_dir(&self) -> &Path {
        &self.backup_dir
    }

    pub(crate) fn hook_binary(&self) -> Option<&Path> {
        self.hook_binary.as_deref()
    }

    pub(crate) fn hook_version(&self) -> &str {
        &self.hook_version
    }

    pub(crate) fn salt(&self) -> Option<&[u8]> {
        self.salt.as_deref()
    }

    pub(crate) fn pending_project_dir(&self, project_id: &str) -> PathBuf {
        self.inbox_dir.join("pending").join(project_id)
    }

    pub(crate) fn pending_count(&self, project_id: &str) -> usize {
        let pending = self.pending_project_dir(project_id);
        if is_link_or_reparse(&self.inbox_dir)
            || is_link_or_reparse(&self.inbox_dir.join("pending"))
            || is_link_or_reparse(&pending)
        {
            return 0;
        }
        count_json_files(&pending)
    }

    pub(crate) fn hook_binary_available(&self) -> bool {
        self.hook_binary.as_ref().is_some_and(|path| path.is_file())
    }

    pub(crate) fn inbox_available(&self) -> bool {
        let salt_path = self.inbox_dir.join(SALT_FILE_NAME);
        self.salt.is_some()
            && [
                self.inbox_dir.clone(),
                self.inbox_dir.join("pending"),
                self.inbox_dir.join("tmp"),
            ]
            .iter()
            .all(|path| path.is_dir() && !is_link_or_reparse(path))
            && salt_path.is_file()
            && !is_link_or_reparse(&salt_path)
    }

    pub(crate) fn hook_binary_digest(&self) -> Option<String> {
        let path = self.hook_binary.as_ref()?;
        let bytes = fs::read(path).ok()?;
        let mut hasher = Sha256::new();
        hasher.update(bytes);
        Some(to_hex(&hasher.finalize()))
    }

    /// 只扫描应用自有 Spool 的目录项与文件大小，不读取事件内容。
    pub fn privacy_snapshot(&self) -> ClaudePrivacySnapshot {
        let salt_path = self.inbox_dir.join(SALT_FILE_NAME);
        let salt_present = salt_path.is_file() && !is_link_or_reparse(&salt_path);
        let salt_bytes = if salt_present {
            fs::metadata(&salt_path).map(|m| m.len()).unwrap_or(0)
        } else {
            0
        };
        let inbox_available = self.inbox_available();
        let pending_root = self.inbox_dir.join("pending");
        let mut pending_event_count = 0_usize;
        let mut pending_bytes = 0_u64;
        if !is_link_or_reparse(&pending_root) {
            if let Ok(projects) = fs::read_dir(&pending_root) {
                for project in projects.flatten() {
                    let path = project.path();
                    if is_link_or_reparse(&path) || !path.is_dir() {
                        continue;
                    }
                    if let Ok(entries) = fs::read_dir(&path) {
                        for entry in entries.flatten() {
                            let entry_path = entry.path();
                            if is_link_or_reparse(&entry_path) {
                                continue;
                            }
                            let is_json = entry_path
                                .extension()
                                .and_then(|value| value.to_str())
                                .is_some_and(|value| value == "json");
                            if !is_json {
                                continue;
                            }
                            pending_event_count += 1;
                            pending_bytes +=
                                entry.metadata().map(|metadata| metadata.len()).unwrap_or(0);
                        }
                    }
                }
            }
        }
        ClaudePrivacySnapshot {
            pending_event_count,
            pending_bytes,
            salt_present,
            salt_bytes,
            inbox_available,
        }
    }
}

fn create_runtime_directory(path: &Path) -> Result<path_guard::Directories, CoreError> {
    match fs::create_dir(path) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
        Err(error) => return Err(error.into()),
    }
    Ok(path_guard::Directories::open(path)?)
}

fn prepare_inbox(inbox: &Path) -> Result<Vec<u8>, CoreError> {
    let _inbox = create_runtime_directory(inbox)?;
    let _pending = create_runtime_directory(&inbox.join("pending"))?;
    let temporary = inbox.join("tmp");
    let _temporary = create_runtime_directory(&temporary)?;
    clean_temporary_files(&temporary);
    Ok(initialize_salt(inbox)?)
}

fn clean_temporary_files(directory: &Path) {
    let Ok(entries) = fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let stale = entry.metadata().ok().is_some_and(|metadata| {
            metadata.is_file()
                && metadata
                    .modified()
                    .ok()
                    .and_then(|time| time.elapsed().ok())
                    .is_some_and(|age| age >= Duration::from_secs(60))
        });
        if stale && path.extension().and_then(|value| value.to_str()) == Some("tmp") {
            let _ = fs::remove_file(path);
        }
    }
}

fn count_json_files(directory: &Path) -> usize {
    let Ok(entries) = fs::read_dir(directory) else {
        return 0;
    };
    entries
        .flatten()
        .filter(|entry| entry.path().extension().and_then(|value| value.to_str()) == Some("json"))
        .count()
}

fn to_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0x0f) as usize] as char);
    }
    output
}
