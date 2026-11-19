//! Claude Code Hook 的最小事件协议。
//!
//! 本 crate 只负责“原始 Hook 输入 → 白名单事件信封”的过滤与校验，不访问
//! SQLite，也不依赖桌面端。`xushu-hook` CLI 与 `xushu-core` 导入器共用同一套
//! 字段规则，避免两边对隐私边界的理解发生偏移。
//!
//! 关键约束：
//! - 原始 stdin JSON 只在内存中短暂存在，绝不写入 spool、数据库或日志；
//! - 只保留事件类型、时间、伪名化会话/工具调用标识、工具名称与类别、成功/失败
//!   类别、耗时等最小元数据；
//! - 所有 ID 都使用带本地盐的 SHA-256 伪名化，spool 中不出现原始 ID。

use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Seek, Write};
use std::path::{Path, PathBuf};
use std::str::FromStr;

use chrono::{DateTime, SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const ENVELOPE_VERSION: u32 = 1;

/// 原始 Hook 输入上限。工具结果可能很大，但本产品只需要几个元数据字段；
/// 超过上限时安全丢弃，不解析、不落盘。
pub const MAX_HOOK_INPUT_BYTES: usize = 256 * 1024;
/// 单个 spool 信封上限；信封字段固定且很短。
pub const MAX_SPOOL_FILE_BYTES: u64 = 16 * 1024;
/// 每个项目最多保留的待导入事件数，避免 XUSHU 关闭期间无限积累。
pub const MAX_PENDING_EVENTS_PER_PROJECT: usize = 1000;
pub const MAX_SESSION_ID_CHARS: usize = 512;
pub const MAX_PROMPT_ID_CHARS: usize = 256;
pub const MAX_TOOL_USE_ID_CHARS: usize = 512;
pub const MAX_TOOL_NAME_CHARS: usize = 64;
pub const MAX_DURATION_MS: i64 = 24 * 60 * 60 * 1000;

pub const SALT_FILE_NAME: &str = ".xushu-salt";
pub const SALT_HEX_CHARS: usize = 64;
pub const MANAGED_ARG: &str = "--xushu-managed";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HookEventType {
    SessionStart,
    PreToolUse,
    PostToolUse,
    PostToolUseFailure,
    Stop,
    SessionEnd,
}

impl HookEventType {
    pub const ALL: [Self; 6] = [
        Self::SessionStart,
        Self::PreToolUse,
        Self::PostToolUse,
        Self::PostToolUseFailure,
        Self::Stop,
        Self::SessionEnd,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::SessionStart => "session_start",
            Self::PreToolUse => "pre_tool_use",
            Self::PostToolUse => "post_tool_use",
            Self::PostToolUseFailure => "post_tool_use_failure",
            Self::Stop => "stop",
            Self::SessionEnd => "session_end",
        }
    }

    /// Claude Code settings.json 中的事件名。
    pub const fn claude_event_name(self) -> &'static str {
        match self {
            Self::SessionStart => "SessionStart",
            Self::PreToolUse => "PreToolUse",
            Self::PostToolUse => "PostToolUse",
            Self::PostToolUseFailure => "PostToolUseFailure",
            Self::Stop => "Stop",
            Self::SessionEnd => "SessionEnd",
        }
    }

    pub const fn is_tool_event(self) -> bool {
        matches!(
            self,
            Self::PreToolUse | Self::PostToolUse | Self::PostToolUseFailure
        )
    }
}

impl FromStr for HookEventType {
    type Err = HookReject;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim() {
            "SessionStart" => Ok(Self::SessionStart),
            "PreToolUse" => Ok(Self::PreToolUse),
            "PostToolUse" => Ok(Self::PostToolUse),
            "PostToolUseFailure" => Ok(Self::PostToolUseFailure),
            "Stop" => Ok(Self::Stop),
            "SessionEnd" => Ok(Self::SessionEnd),
            _ => Err(HookReject::UnsupportedEvent),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolCategory {
    Shell,
    FileRead,
    FileWrite,
    Search,
    Web,
    Agent,
    Mcp,
    Task,
    Other,
}

impl ToolCategory {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Shell => "shell",
            Self::FileRead => "file_read",
            Self::FileWrite => "file_write",
            Self::Search => "search",
            Self::Web => "web",
            Self::Agent => "agent",
            Self::Mcp => "mcp",
            Self::Task => "task",
            Self::Other => "other",
        }
    }
}

pub fn classify_tool(name: &str) -> ToolCategory {
    match name {
        "Bash" | "PowerShell" => ToolCategory::Shell,
        "Read" => ToolCategory::FileRead,
        "Write" | "Edit" | "MultiEdit" | "NotebookEdit" => ToolCategory::FileWrite,
        "Glob" | "Grep" => ToolCategory::Search,
        "WebFetch" | "WebSearch" => ToolCategory::Web,
        "Agent" | "Task" => ToolCategory::Agent,
        "TaskCreate" | "TaskGet" | "TaskList" | "TaskOutput" | "TaskStop" | "TaskUpdate" => {
            ToolCategory::Task
        }
        value if value.starts_with("mcp__") => ToolCategory::Mcp,
        _ => ToolCategory::Other,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FailureKind {
    ExitCodeNonzero,
    CommandNotStarted,
    ToolError,
    Timeout,
    Interrupted,
    Unknown,
}

impl FailureKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ExitCodeNonzero => "exit_code_nonzero",
            Self::CommandNotStarted => "command_not_started",
            Self::ToolError => "tool_error",
            Self::Timeout => "timeout",
            Self::Interrupted => "interrupted",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionStartSource {
    Startup,
    Resume,
    Clear,
    Compact,
    Fork,
    Unknown,
}

impl SessionStartSource {
    pub fn from_optional(value: Option<&str>) -> Self {
        match value.map(str::trim) {
            Some("startup") => Self::Startup,
            Some("resume") => Self::Resume,
            Some("clear") => Self::Clear,
            Some("compact") => Self::Compact,
            Some("fork") => Self::Fork,
            _ => Self::Unknown,
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Startup => "startup",
            Self::Resume => "resume",
            Self::Clear => "clear",
            Self::Compact => "compact",
            Self::Fork => "fork",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionEndReason {
    Clear,
    Resume,
    Logout,
    PromptInputExit,
    Other,
    Unknown,
}

impl SessionEndReason {
    pub fn from_optional(value: Option<&str>) -> Self {
        match value.map(str::trim) {
            Some("clear") => Self::Clear,
            Some("resume") => Self::Resume,
            Some("logout") => Self::Logout,
            Some("prompt_input_exit") => Self::PromptInputExit,
            Some("other") => Self::Other,
            _ => Self::Unknown,
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Clear => "clear",
            Self::Resume => "resume",
            Self::Logout => "logout",
            Self::PromptInputExit => "prompt_input_exit",
            Self::Other => "other",
            Self::Unknown => "unknown",
        }
    }
}

/// 写入 spool 的最小事件。所有字段都是白名单元数据。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HookEnvelope {
    pub version: u32,
    pub delivery_id: String,
    pub project_id: String,
    #[serde(default, skip_serializing_if = "legacy_generation")]
    pub capture_generation: i64,
    pub project_fingerprint: String,
    pub captured_at: String,
    pub event_type: HookEventType,
    pub session_hash: String,
    pub prompt_hash: Option<String>,
    pub tool_use_hash: Option<String>,
    pub tool_name: Option<String>,
    pub tool_category: Option<ToolCategory>,
    pub failure_kind: Option<FailureKind>,
    pub is_interrupt: Option<bool>,
    pub duration_ms: Option<i64>,
    pub source: Option<SessionStartSource>,
    pub end_reason: Option<SessionEndReason>,
}

fn legacy_generation(generation: &i64) -> bool {
    *generation == 0
}

impl HookEnvelope {
    pub fn validate(&self) -> Result<(), HookReject> {
        if self.version != ENVELOPE_VERSION {
            return Err(HookReject::InvalidField("version"));
        }
        if !is_uuid(&self.delivery_id) {
            return Err(HookReject::InvalidField("deliveryId"));
        }
        if !is_uuid(&self.project_id) {
            return Err(HookReject::InvalidField("projectId"));
        }
        if self.capture_generation < 0 {
            return Err(HookReject::InvalidField("captureGeneration"));
        }
        if !is_hex_digest(&self.project_fingerprint) || !is_hex_digest(&self.session_hash) {
            return Err(HookReject::InvalidField("hash"));
        }
        if let Some(prompt_hash) = &self.prompt_hash {
            if !is_hex_digest(prompt_hash) {
                return Err(HookReject::InvalidField("promptHash"));
            }
        }
        if let Some(tool_hash) = &self.tool_use_hash {
            if !is_hex_digest(tool_hash) {
                return Err(HookReject::InvalidField("toolUseHash"));
            }
        }
        if DateTime::parse_from_rfc3339(&self.captured_at).is_err() {
            return Err(HookReject::InvalidField("capturedAt"));
        }
        if let Some(duration) = self.duration_ms {
            if !(0..=MAX_DURATION_MS).contains(&duration) {
                return Err(HookReject::InvalidField("durationMs"));
            }
        }
        if let Some(name) = &self.tool_name {
            if sanitize_tool_name(name).is_none() {
                return Err(HookReject::InvalidField("toolName"));
            }
        }

        let tool_fields_present = self.tool_use_hash.is_some()
            || self.tool_name.is_some()
            || self.tool_category.is_some()
            || self.failure_kind.is_some()
            || self.is_interrupt.is_some()
            || self.duration_ms.is_some();

        match self.event_type {
            HookEventType::SessionStart => {
                if self.source.is_none()
                    || tool_fields_present
                    || self.end_reason.is_some()
                    || self.prompt_hash.is_some()
                {
                    return Err(HookReject::InvalidField("sessionStart"));
                }
            }
            HookEventType::PreToolUse => {
                if self.tool_use_hash.is_none()
                    || self.tool_name.is_none()
                    || self.tool_category.is_none()
                    || self.failure_kind.is_some()
                    || self.is_interrupt.is_some()
                    || self.duration_ms.is_some()
                    || self.source.is_some()
                    || self.end_reason.is_some()
                {
                    return Err(HookReject::InvalidField("preToolUse"));
                }
            }
            HookEventType::PostToolUse => {
                if self.tool_use_hash.is_none()
                    || self.tool_name.is_none()
                    || self.tool_category.is_none()
                    || self.failure_kind.is_some()
                    || self.is_interrupt.is_some()
                    || self.source.is_some()
                    || self.end_reason.is_some()
                {
                    return Err(HookReject::InvalidField("postToolUse"));
                }
            }
            HookEventType::PostToolUseFailure => {
                if self.tool_use_hash.is_none()
                    || self.tool_name.is_none()
                    || self.tool_category.is_none()
                    || self.failure_kind.is_none()
                    || self.source.is_some()
                    || self.end_reason.is_some()
                {
                    return Err(HookReject::InvalidField("postToolUseFailure"));
                }
            }
            HookEventType::Stop => {
                if tool_fields_present || self.source.is_some() || self.end_reason.is_some() {
                    return Err(HookReject::InvalidField("stop"));
                }
            }
            HookEventType::SessionEnd => {
                if self.end_reason.is_none()
                    || tool_fields_present
                    || self.source.is_some()
                    || self.prompt_hash.is_some()
                {
                    return Err(HookReject::InvalidField("sessionEnd"));
                }
            }
        }

        Ok(())
    }

    /// 语义去重键。与 `deliveryId` 的单次投递幂等不同，这里用于防止同一工具
    /// 完成事件被重复投递后重复记账。Stop 使用 prompt_id 伪名化后的哈希；
    /// 老版本没有 prompt_id 时返回 `None`，由导入端的 delivery_id 保证幂等。
    pub fn semantic_dedupe_key(&self) -> Option<String> {
        match self.event_type {
            HookEventType::PreToolUse
            | HookEventType::PostToolUse
            | HookEventType::PostToolUseFailure => Some(format!(
                "tool|{}|{}|{}",
                self.session_hash,
                self.event_type.as_str(),
                self.tool_use_hash.as_deref().unwrap_or_default()
            )),
            HookEventType::Stop => self
                .prompt_hash
                .as_ref()
                .map(|prompt_hash| format!("stop|{}|{prompt_hash}", self.session_hash)),
            HookEventType::SessionEnd => Some(format!(
                "session_end|{}|{}",
                self.session_hash,
                self.end_reason
                    .map(SessionEndReason::as_str)
                    .unwrap_or("unknown")
            )),
            HookEventType::SessionStart => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HookReject {
    InputTooLarge,
    InvalidJson,
    UnsupportedEvent,
    MissingField(&'static str),
    InvalidField(&'static str),
    OutsideProject,
    ProjectUnavailable,
    SaltUnavailable,
}

impl HookReject {
    /// 只用于本地诊断与测试；不会包含输入内容或路径。
    pub const fn code(self) -> &'static str {
        match self {
            Self::InputTooLarge => "input_too_large",
            Self::InvalidJson => "invalid_json",
            Self::UnsupportedEvent => "unsupported_event",
            Self::MissingField(_) => "missing_field",
            Self::InvalidField(_) => "invalid_field",
            Self::OutsideProject => "outside_project",
            Self::ProjectUnavailable => "project_unavailable",
            Self::SaltUnavailable => "salt_unavailable",
        }
    }
}

impl std::fmt::Display for HookReject {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingField(field) => write!(formatter, "缺少字段: {field}"),
            Self::InvalidField(field) => write!(formatter, "字段无效: {field}"),
            other => formatter.write_str(other.code()),
        }
    }
}

impl std::error::Error for HookReject {}

pub struct FilterContext<'a> {
    pub project_id: &'a str,
    pub project_root: &'a Path,
    pub salt: &'a [u8],
}

/// 将原始 Hook stdin 过滤为最小信封。任何异常都返回 `Err`，调用方应安全退出 0。
pub fn filter_hook_input(
    raw: &[u8],
    context: &FilterContext<'_>,
) -> Result<HookEnvelope, HookReject> {
    if raw.is_empty() || raw.len() > MAX_HOOK_INPUT_BYTES {
        return Err(HookReject::InputTooLarge);
    }
    if !is_uuid(context.project_id) {
        return Err(HookReject::InvalidField("projectId"));
    }

    let value: serde_json::Value =
        serde_json::from_slice(raw).map_err(|_| HookReject::InvalidJson)?;
    let object = value.as_object().ok_or(HookReject::InvalidJson)?;

    let event_name = object
        .get("hook_event_name")
        .and_then(serde_json::Value::as_str)
        .ok_or(HookReject::MissingField("hook_event_name"))?;
    let event_type = HookEventType::from_str(event_name)?;

    let session_id = object
        .get("session_id")
        .and_then(serde_json::Value::as_str)
        .ok_or(HookReject::MissingField("session_id"))?;
    if session_id.is_empty() || session_id.chars().count() > MAX_SESSION_ID_CHARS {
        return Err(HookReject::InvalidField("session_id"));
    }

    let cwd = object
        .get("cwd")
        .and_then(serde_json::Value::as_str)
        .ok_or(HookReject::MissingField("cwd"))?;
    canonical_cwd_within(context.project_root, cwd)?;

    let prompt_hash = object
        .get("prompt_id")
        .and_then(serde_json::Value::as_str)
        .filter(|value| !value.is_empty() && value.chars().count() <= MAX_PROMPT_ID_CHARS)
        .map(|value| hash_token(context.salt, 0x02, value));

    let mut envelope = HookEnvelope {
        version: ENVELOPE_VERSION,
        delivery_id: uuid::Uuid::new_v4().to_string(),
        project_id: context.project_id.to_owned(),
        capture_generation: 0,
        project_fingerprint: project_root_fingerprint(
            context.salt,
            &canonical_project_root(context.project_root)?,
        ),
        captured_at: Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true),
        event_type,
        session_hash: hash_token(context.salt, 0x01, session_id),
        prompt_hash: None,
        tool_use_hash: None,
        tool_name: None,
        tool_category: None,
        failure_kind: None,
        is_interrupt: None,
        duration_ms: None,
        source: None,
        end_reason: None,
    };

    match event_type {
        HookEventType::SessionStart => {
            envelope.source = Some(SessionStartSource::from_optional(
                object.get("source").and_then(serde_json::Value::as_str),
            ));
            envelope.prompt_hash = None;
        }
        HookEventType::PreToolUse
        | HookEventType::PostToolUse
        | HookEventType::PostToolUseFailure => {
            envelope.prompt_hash = prompt_hash;
            let tool_name = object
                .get("tool_name")
                .and_then(serde_json::Value::as_str)
                .ok_or(HookReject::MissingField("tool_name"))?;
            let tool_name =
                sanitize_tool_name(tool_name).ok_or(HookReject::InvalidField("tool_name"))?;
            let tool_use_id = object
                .get("tool_use_id")
                .and_then(serde_json::Value::as_str)
                .ok_or(HookReject::MissingField("tool_use_id"))?;
            if tool_use_id.is_empty() || tool_use_id.chars().count() > MAX_TOOL_USE_ID_CHARS {
                return Err(HookReject::InvalidField("tool_use_id"));
            }

            envelope.tool_name = Some(tool_name.clone());
            envelope.tool_category = Some(classify_tool(&tool_name));
            envelope.tool_use_hash = Some(hash_token(context.salt, 0x03, tool_use_id));
            envelope.duration_ms = object
                .get("duration_ms")
                .and_then(serde_json::Value::as_i64)
                .filter(|duration| (0..=MAX_DURATION_MS).contains(duration));

            if event_type == HookEventType::PostToolUseFailure {
                let is_interrupt = object
                    .get("is_interrupt")
                    .and_then(serde_json::Value::as_bool);
                let error = object.get("error").and_then(serde_json::Value::as_str);
                envelope.is_interrupt = is_interrupt;
                envelope.failure_kind = Some(classify_failure(error, is_interrupt));
            }
        }
        HookEventType::Stop => {
            envelope.prompt_hash = prompt_hash;
        }
        HookEventType::SessionEnd => {
            envelope.end_reason = Some(SessionEndReason::from_optional(
                object.get("reason").and_then(serde_json::Value::as_str),
            ));
        }
    }

    envelope.validate()?;
    Ok(envelope)
}

fn canonical_project_root(project_root: &Path) -> Result<PathBuf, HookReject> {
    std::fs::canonicalize(project_root).map_err(|_| HookReject::ProjectUnavailable)
}

pub fn canonical_cwd_within(root: &Path, cwd: &str) -> Result<PathBuf, HookReject> {
    let root = std::fs::canonicalize(root).map_err(|_| HookReject::ProjectUnavailable)?;
    let cwd = std::fs::canonicalize(cwd).map_err(|_| HookReject::OutsideProject)?;
    if !cwd.starts_with(&root) {
        return Err(HookReject::OutsideProject);
    }
    Ok(cwd)
}

pub fn sanitize_tool_name(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() || trimmed.chars().count() > MAX_TOOL_NAME_CHARS {
        return None;
    }
    if !trimmed.chars().all(|character| {
        character.is_ascii_alphanumeric() || matches!(character, '_' | '-' | '.' | ':')
    }) {
        return None;
    }
    Some(trimmed.to_owned())
}

pub fn classify_failure(error: Option<&str>, is_interrupt: Option<bool>) -> FailureKind {
    if is_interrupt == Some(true) {
        return FailureKind::Interrupted;
    }
    let Some(error) = error else {
        return FailureKind::Unknown;
    };
    let first_line = error.lines().next().unwrap_or_default().trim();
    if first_line.is_empty() {
        return FailureKind::Unknown;
    }
    let lower = error.to_ascii_lowercase();
    if lower.contains("timed out") || lower.contains("timeout") {
        return FailureKind::Timeout;
    }
    if lower.contains("failed to start") || lower.contains("could not start") {
        return FailureKind::CommandNotStarted;
    }
    if let Some(rest) = first_line.strip_prefix("Exit code ") {
        if let Ok(code) = rest.trim().parse::<i32>() {
            return if code == 0 {
                FailureKind::ToolError
            } else {
                FailureKind::ExitCodeNonzero
            };
        }
    }
    FailureKind::ToolError
}

pub fn hash_token(salt: &[u8], domain: u8, value: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(salt);
    hasher.update([domain]);
    hasher.update(value.as_bytes());
    to_hex(&hasher.finalize())
}

pub fn project_root_fingerprint(salt: &[u8], project_root: &Path) -> String {
    hash_token(salt, 0x04, &normalize_root(project_root))
}

fn normalize_root(path: &Path) -> String {
    let value = path.to_string_lossy();
    #[cfg(windows)]
    {
        value.replace('/', "\\").to_lowercase()
    }
    #[cfg(not(windows))]
    {
        value.to_string()
    }
}

pub fn read_salt(inbox_dir: &Path) -> Result<Vec<u8>, HookReject> {
    let path = inbox_dir.join(SALT_FILE_NAME);
    if is_link_or_reparse(inbox_dir) || is_link_or_reparse(&path) || !path.is_file() {
        return Err(HookReject::SaltUnavailable);
    }
    let mut file = File::open(path).map_err(|_| HookReject::SaltUnavailable)?;
    file.lock_shared()
        .map_err(|_| HookReject::SaltUnavailable)?;
    read_salt_file(&mut file)
        .map_err(|_| HookReject::SaltUnavailable)?
        .ok_or(HookReject::SaltUnavailable)
}

pub fn initialize_salt(inbox_dir: &Path) -> io::Result<Vec<u8>> {
    if let Ok(salt) = read_salt(inbox_dir) {
        return Ok(salt);
    }
    let path = inbox_dir.join(SALT_FILE_NAME);
    if is_link_or_reparse(inbox_dir) || is_link_or_reparse(&path) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "redirected salt path",
        ));
    }
    let mut options = OpenOptions::new();
    options.read(true).write(true).create(true).truncate(false);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    // Readers take a shared lock, including on Windows volumes that reject rename.
    let mut file = options.open(path)?;
    file.lock()?;
    if let Some(salt) = read_salt_file(&mut file)? {
        return Ok(salt);
    }
    let generated = generate_salt_hex();
    file.rewind()?;
    file.set_len(0)?;
    file.write_all(generated.as_bytes())?;
    file.sync_all()?;
    Ok(decode_salt(&generated).expect("generated salt is valid hex"))
}

fn read_salt_file(file: &mut File) -> io::Result<Option<Vec<u8>>> {
    let mut bytes = Vec::new();
    file.take(SALT_HEX_CHARS as u64 + 3)
        .read_to_end(&mut bytes)?;
    if bytes.len() > SALT_HEX_CHARS + 2 {
        return Ok(None);
    }
    Ok(std::str::from_utf8(&bytes)
        .ok()
        .and_then(|content| decode_salt(content.trim())))
}

pub fn is_link_or_reparse(path: &Path) -> bool {
    let Ok(metadata) = fs::symlink_metadata(path) else {
        return false;
    };
    let is_symlink = metadata.file_type().is_symlink();
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0000_0400;
        is_symlink || metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
    }
    #[cfg(not(windows))]
    {
        is_symlink
    }
}

pub fn decode_salt(value: &str) -> Option<Vec<u8>> {
    if value.len() != SALT_HEX_CHARS || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return None;
    }
    let mut bytes = Vec::with_capacity(SALT_HEX_CHARS / 2);
    for pair in value.as_bytes().as_chunks::<2>().0 {
        let high = hex_value(pair[0])?;
        let low = hex_value(pair[1])?;
        bytes.push((high << 4) | low);
    }
    Some(bytes)
}

pub fn generate_salt_hex() -> String {
    // UUID v4 使用操作系统随机源；两次拼接得到 32 字节随机盐。
    let first = uuid::Uuid::new_v4().simple().to_string();
    let second = uuid::Uuid::new_v4().simple().to_string();
    format!("{first}{second}")
}

fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
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

fn is_uuid(value: &str) -> bool {
    uuid::Uuid::parse_str(value).is_ok()
}

fn is_hex_digest(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

/// 供 CLI 在测试中使用的时间格式辅助。
pub fn utc_now_rfc3339() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_project() -> (tempfile::TempDir, PathBuf, Vec<u8>) {
        let directory = tempfile::TempDir::new().expect("temp dir");
        let project = directory.path().join("项目 with spaces");
        std::fs::create_dir_all(&project).expect("create project");
        let salt = decode_salt(&generate_salt_hex()).expect("salt");
        (directory, project, salt)
    }

    fn pre_tool_input(project: &Path, extra: &str) -> Vec<u8> {
        format!(
            r#"{{
                "session_id": "raw-session-123",
                "transcript_path": "C:\\Users\\secret\\transcript.jsonl",
                "cwd": "{}",
                "hook_event_name": "PreToolUse",
                "tool_name": "Bash",
                "tool_input": {{"command": "curl -H Authorization: SECRET_TOKEN https://example.com", "description": "secret"}},
                "tool_use_id": "toolu_raw_123",
                "prompt_id": "prompt-raw-456"
                {extra}
            }}"#,
            project.to_string_lossy().replace('\\', "\\\\")
        )
        .into_bytes()
    }

    #[test]
    fn filters_sensitive_fields_and_hashes_identifiers() {
        let (_directory, project, salt) = temp_project();
        let envelope = filter_hook_input(
            &pre_tool_input(&project, ""),
            &FilterContext {
                project_id: "11111111-1111-4111-8111-111111111111",
                project_root: &project,
                salt: &salt,
            },
        )
        .expect("filter hook input");

        let serialized = serde_json::to_string(&envelope).expect("serialize");
        assert!(!serialized.contains("SECRET_TOKEN"));
        assert!(!serialized.contains("raw-session-123"));
        assert!(!serialized.contains("toolu_raw_123"));
        assert!(!serialized.contains("prompt-raw-456"));
        assert!(!serialized.contains("transcript.jsonl"));
        assert!(!serialized.contains("C:\\\\Users"));
        assert_eq!(envelope.event_type, HookEventType::PreToolUse);
        assert_eq!(envelope.tool_category, Some(ToolCategory::Shell));
        assert_eq!(envelope.tool_name.as_deref(), Some("Bash"));
        assert!(envelope.tool_use_hash.is_some());
        assert!(envelope.prompt_hash.is_some());
    }

    #[test]
    fn rejects_input_from_outside_project() {
        let (directory, project, salt) = temp_project();
        let outside = directory.path().join("other");
        std::fs::create_dir_all(&outside).expect("outside");
        let raw = String::from_utf8(pre_tool_input(&project, "")).expect("utf8");
        let raw = raw.replace(
            &project.to_string_lossy().replace('\\', "\\\\"),
            &outside.to_string_lossy().replace('\\', "\\\\"),
        );
        let error = filter_hook_input(
            raw.as_bytes(),
            &FilterContext {
                project_id: "11111111-1111-4111-8111-111111111111",
                project_root: &project,
                salt: &salt,
            },
        )
        .expect_err("must reject");
        assert_eq!(error, HookReject::OutsideProject);
    }

    #[test]
    fn rejects_oversized_or_invalid_input() {
        let (_directory, project, salt) = temp_project();
        let context = FilterContext {
            project_id: "11111111-1111-4111-8111-111111111111",
            project_root: &project,
            salt: &salt,
        };
        let oversized = vec![b'{'; MAX_HOOK_INPUT_BYTES + 1];
        assert_eq!(
            filter_hook_input(&oversized, &context).expect_err("oversized"),
            HookReject::InputTooLarge
        );
        assert_eq!(
            filter_hook_input(b"{not json", &context).expect_err("invalid"),
            HookReject::InvalidJson
        );
    }

    #[test]
    fn classifies_failure_without_storing_error_text() {
        assert_eq!(
            classify_failure(Some("Exit code 1\nError: SECRET"), Some(false)),
            FailureKind::ExitCodeNonzero
        );
        assert_eq!(
            classify_failure(Some("Command timed out after 2m 0s"), None),
            FailureKind::Timeout
        );
        assert_eq!(
            classify_failure(Some("failed to start shell"), None),
            FailureKind::CommandNotStarted
        );
        assert_eq!(
            classify_failure(Some("anything"), Some(true)),
            FailureKind::Interrupted
        );
        assert_eq!(classify_failure(None, None), FailureKind::Unknown);
    }

    #[test]
    fn validates_envelopes_and_semantic_dedupe_keys() {
        let (_directory, project, salt) = temp_project();
        let envelope = filter_hook_input(
            &pre_tool_input(&project, ""),
            &FilterContext {
                project_id: "11111111-1111-4111-8111-111111111111",
                project_root: &project,
                salt: &salt,
            },
        )
        .expect("filter");
        envelope.validate().expect("valid envelope");
        assert!(envelope.semantic_dedupe_key().is_some());

        let mut broken = envelope.clone();
        broken.tool_name = Some("bad name with spaces".to_owned());
        assert!(broken.validate().is_err());
    }
}
