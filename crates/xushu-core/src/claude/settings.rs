use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

use serde::Serialize;
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use uuid::Uuid;
use xushu_hook_protocol::HookEventType;

use crate::error::CoreError;

use super::path_guard::{identity, redirected, regular_file_options, Directories};

pub(crate) const SETTINGS_DIR_NAME: &str = ".claude";
pub(crate) const SETTINGS_FILE_NAME: &str = "settings.local.json";
pub(crate) const SETTINGS_RELATIVE: &str = ".claude/settings.local.json";
pub(crate) const HOOK_TIMEOUT_SECONDS: u64 = 5;
pub(crate) const MAX_SETTINGS_BYTES: u64 = 1024 * 1024;
const BACKUP_KEEP: usize = 5;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SettingsError {
    ProjectUnavailable,
    UnsafePath,
    TooLarge,
    InvalidJson,
    InvalidStructure(&'static str),
    ReadFailed,
}

impl SettingsError {
    pub(crate) fn message(&self) -> &'static str {
        match self {
            Self::ProjectUnavailable => "项目目录不可访问",
            Self::UnsafePath => "项目路径已变化或包含重定向，已拒绝修改设置",
            Self::TooLarge => "项目设置文件超过 1 MiB，已拒绝读取或修改",
            Self::InvalidJson => "settings.local.json 不是有效 JSON，未做任何修改",
            Self::InvalidStructure(reason) => reason,
            Self::ReadFailed => "无法读取项目设置文件",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PlannedEntry {
    pub event: String,
    pub matcher: Option<String>,
    pub command: String,
    pub args: Vec<String>,
    pub timeout_seconds: u64,
}

#[derive(Debug, Clone)]
pub(crate) struct ManagedEntry {
    pub event: String,
    matcher: Option<Value>,
    handler: Value,
}

impl ManagedEntry {
    fn matches(&self, expected: &PlannedEntry) -> bool {
        self.event == expected.event
            && self.matcher == expected.matcher.as_ref().map(|matcher| json!(matcher))
            && self.handler == expected.handler()
    }
}

impl PlannedEntry {
    fn handler(&self) -> Value {
        json!({
            "type": "command",
            "command": self.command,
            "args": self.args,
            "timeout": self.timeout_seconds,
        })
    }
}

#[derive(Debug, Clone)]
pub(crate) struct SettingsSnapshot {
    authorized_root: PathBuf,
    pub project_root: PathBuf,
    pub root_identity: Option<String>,
    pub settings_dir: PathBuf,
    settings_dir_identity: Option<String>,
    pub settings_path: PathBuf,
    settings_identity: Option<String>,
    pub exists: bool,
    pub raw: Option<Vec<u8>>,
    pub document: Option<Value>,
    pub error: Option<SettingsError>,
    pub managed_entries: Vec<ManagedEntry>,
    pub other_group_count: usize,
}

impl SettingsSnapshot {
    fn project_unavailable(project_root: &Path) -> Self {
        let settings_dir = project_root.join(SETTINGS_DIR_NAME);
        Self {
            authorized_root: project_root.to_path_buf(),
            project_root: project_root.to_path_buf(),
            root_identity: None,
            settings_path: settings_dir.join(SETTINGS_FILE_NAME),
            settings_dir,
            settings_dir_identity: None,
            settings_identity: None,
            exists: false,
            raw: None,
            document: None,
            error: Some(SettingsError::ProjectUnavailable),
            managed_entries: Vec::new(),
            other_group_count: 0,
        }
    }

    fn with_error(mut self, error: SettingsError) -> Self {
        self.error = Some(error);
        self.document = None;
        self.managed_entries.clear();
        self.other_group_count = 0;
        self
    }

    pub fn require_root(self, expected: Option<&str>) -> Self {
        if self.error.is_none() && expected.is_some() && self.root_identity.as_deref() != expected {
            return self.with_error(SettingsError::UnsafePath);
        }
        self
    }

    pub fn has_generation(&self, generation: i64) -> bool {
        self.managed_entries.iter().any(|entry| {
            let Some(args) = entry.handler.get("args").and_then(Value::as_array) else {
                return false;
            };
            match args.as_slice() {
                [_, _, _, _, _, _] => generation == 0,
                [_, _, _, _, _, _, flag, value] => {
                    flag.as_str() == Some("--capture-generation")
                        && value.as_str().and_then(|value| value.parse().ok()) == Some(generation)
                }
                _ => false,
            }
        })
    }
}

pub(crate) fn read_snapshot(project_root: &Path, project_id: &str) -> SettingsSnapshot {
    let root = match Directories::open(project_root) {
        Ok(root) => root,
        Err(error) => {
            return SettingsSnapshot::project_unavailable(project_root)
                .with_error(path_error(&error, SettingsError::ProjectUnavailable));
        }
    };
    let settings_dir = project_root.join(SETTINGS_DIR_NAME);
    let settings_path = settings_dir.join(SETTINGS_FILE_NAME);
    let mut snapshot = SettingsSnapshot {
        authorized_root: project_root.to_owned(),
        project_root: root.canonical.clone(),
        root_identity: Some(root.identity.clone()),
        settings_dir,
        settings_dir_identity: None,
        settings_path,
        settings_identity: None,
        exists: false,
        raw: None,
        document: Some(json!({})),
        error: None,
        managed_entries: Vec::new(),
        other_group_count: 0,
    };

    let directory = match Directories::open(&snapshot.settings_dir) {
        Ok(directory) => directory,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return snapshot,
        Err(error) => return snapshot.with_error(path_error(&error, SettingsError::ReadFailed)),
    };
    snapshot.settings_dir_identity = Some(directory.identity.clone());
    let file = match open_settings(&snapshot.settings_path) {
        Ok(file) => file,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return snapshot,
        Err(error) => return snapshot.with_error(path_error(&error, SettingsError::ReadFailed)),
    };
    let Ok(file_identity) = identity(&file) else {
        return snapshot.with_error(SettingsError::ReadFailed);
    };
    let mut raw = Vec::new();
    if file
        .take(MAX_SETTINGS_BYTES + 1)
        .read_to_end(&mut raw)
        .is_err()
    {
        return snapshot.with_error(SettingsError::ReadFailed);
    }
    if raw.len() as u64 > MAX_SETTINGS_BYTES {
        return snapshot.with_error(SettingsError::TooLarge);
    }
    snapshot.exists = true;
    snapshot.settings_identity = Some(file_identity);
    let document = serde_json::from_slice::<Value>(&raw);
    snapshot.raw = Some(raw);
    let Ok(document) = document else {
        return snapshot.with_error(SettingsError::InvalidJson);
    };

    match analyze_document(&document, project_id) {
        Ok((managed_entries, other_group_count)) => {
            snapshot.document = Some(document);
            snapshot.managed_entries = managed_entries;
            snapshot.other_group_count = other_group_count;
            snapshot
        }
        Err(error) => snapshot.with_error(error),
    }
}

fn path_error(error: &io::Error, missing: SettingsError) -> SettingsError {
    match error.kind() {
        io::ErrorKind::NotFound => missing,
        io::ErrorKind::InvalidInput | io::ErrorKind::NotADirectory => SettingsError::UnsafePath,
        _ => SettingsError::ReadFailed,
    }
}

fn open_settings(path: &Path) -> io::Result<File> {
    let metadata = fs::symlink_metadata(path)?;
    if redirected(&metadata) || !metadata.is_file() {
        return Err(super::path_guard::unsafe_path());
    }
    let file = regular_file_options().read(true).open(path)?;
    let metadata = file.metadata()?;
    if redirected(&metadata) || !metadata.is_file() {
        return Err(super::path_guard::unsafe_path());
    }
    Ok(file)
}

fn analyze_document(
    document: &Value,
    project_id: &str,
) -> Result<(Vec<ManagedEntry>, usize), SettingsError> {
    let root = document.as_object().ok_or(SettingsError::InvalidStructure(
        "设置文件根节点必须是 JSON 对象",
    ))?;
    let Some(hooks) = root.get("hooks") else {
        return Ok((Vec::new(), 0));
    };
    let hooks = hooks.as_object().ok_or(SettingsError::InvalidStructure(
        "hooks 字段必须是 JSON 对象",
    ))?;

    let mut managed = Vec::new();
    let mut other_group_count = 0;
    for (event, groups) in hooks {
        let groups = groups
            .as_array()
            .ok_or(SettingsError::InvalidStructure("hooks 事件必须是数组"))?;
        for group in groups {
            let group = group.as_object().ok_or(SettingsError::InvalidStructure(
                "hooks matcher 组必须是 JSON 对象",
            ))?;
            let handlers = group.get("hooks").and_then(Value::as_array).ok_or(
                SettingsError::InvalidStructure("hooks matcher 组的 hooks 必须是数组"),
            )?;
            let mut has_other = false;
            for handler in handlers {
                let handler = handler.as_object().ok_or(SettingsError::InvalidStructure(
                    "Hook 处理器必须是 JSON 对象",
                ))?;
                if is_managed_handler(handler, project_id) {
                    managed.push(ManagedEntry {
                        event: event.clone(),
                        matcher: group.get("matcher").cloned(),
                        handler: Value::Object(handler.clone()),
                    });
                } else {
                    has_other = true;
                }
            }
            if has_other {
                other_group_count += 1;
            }
        }
    }
    Ok((managed, other_group_count))
}

fn is_managed_handler(handler: &Map<String, Value>, project_id: &str) -> bool {
    if handler.get("type").and_then(Value::as_str) != Some("command")
        || !handler.get("command").is_some_and(Value::is_string)
    {
        return false;
    }
    handler
        .get("args")
        .and_then(Value::as_array)
        .is_some_and(|args| {
            let mut args = args.iter();
            args.next().and_then(Value::as_str) == Some(xushu_hook_protocol::MANAGED_ARG)
                && args.next().and_then(Value::as_str) == Some(project_id)
                && args.all(Value::is_string)
        })
}

pub(crate) fn expected_entries(
    project_id: &str,
    project_root: &Path,
    hook_binary: &Path,
    inbox_dir: &Path,
    generation: i64,
) -> Vec<PlannedEntry> {
    let root = project_root.to_string_lossy().to_string();
    let inbox = inbox_dir.to_string_lossy().to_string();
    let command = hook_binary.to_string_lossy().to_string();
    HookEventType::ALL
        .iter()
        .map(|event| {
            let mut args = vec![
                xushu_hook_protocol::MANAGED_ARG.to_owned(),
                project_id.to_owned(),
                "--project-root".to_owned(),
                root.clone(),
                "--inbox".to_owned(),
                inbox.clone(),
            ];
            if generation > 0 {
                args.extend(["--capture-generation".to_owned(), generation.to_string()]);
            }
            PlannedEntry {
                event: event.claude_event_name().to_owned(),
                matcher: if *event == HookEventType::Stop {
                    None
                } else {
                    Some("*".to_owned())
                },
                command: command.clone(),
                args,
                timeout_seconds: HOOK_TIMEOUT_SECONDS,
            }
        })
        .collect()
}

pub(crate) fn exact_managed_count(snapshot: &SettingsSnapshot, expected: &[PlannedEntry]) -> usize {
    expected
        .iter()
        .filter(|expected| {
            snapshot
                .managed_entries
                .iter()
                .filter(|entry| entry.matches(expected))
                .count()
                == 1
        })
        .count()
}

pub(crate) fn merged_document(
    snapshot: &SettingsSnapshot,
    expected: &[PlannedEntry],
    project_id: &str,
) -> (Value, bool) {
    let original = snapshot.document.clone().unwrap_or_else(|| json!({}));
    let mut document = original.clone();
    let root = document
        .as_object_mut()
        .expect("settings snapshot validated root object");
    if !root.contains_key("hooks") || !root.get("hooks").is_some_and(Value::is_object) {
        root.insert("hooks".to_owned(), json!({}));
    }
    let hooks = root
        .get_mut("hooks")
        .and_then(Value::as_object_mut)
        .expect("hooks object inserted");

    remove_managed_from_hooks(hooks, project_id);

    for entry in expected {
        let mut group = Map::new();
        if let Some(matcher) = &entry.matcher {
            group.insert("matcher".to_owned(), json!(matcher));
        }
        group.insert("hooks".to_owned(), Value::Array(vec![entry.handler()]));
        let event = hooks
            .entry(entry.event.clone())
            .or_insert_with(|| Value::Array(Vec::new()));
        if !event.is_array() {
            *event = Value::Array(Vec::new());
        }
        event
            .as_array_mut()
            .expect("event array")
            .push(Value::Object(group));
    }

    let changed = document != original;
    (document, changed)
}

pub(crate) fn removed_document(snapshot: &SettingsSnapshot, project_id: &str) -> (Value, usize) {
    let original = snapshot.document.clone().unwrap_or_else(|| json!({}));
    let mut document = original.clone();
    let removed = document
        .get_mut("hooks")
        .and_then(Value::as_object_mut)
        .map(|hooks| remove_managed_from_hooks(hooks, project_id))
        .unwrap_or(0);
    (document, removed)
}

fn remove_managed_from_hooks(hooks: &mut Map<String, Value>, project_id: &str) -> usize {
    let mut removed = 0;
    let events: Vec<String> = hooks.keys().cloned().collect();
    for event in events {
        let Some(groups) = hooks.get_mut(&event).and_then(Value::as_array_mut) else {
            continue;
        };
        let mut kept_groups = Vec::with_capacity(groups.len());
        let removed_before_event = removed;
        for group in groups.iter() {
            let Some(group_object) = group.as_object() else {
                kept_groups.push(group.clone());
                continue;
            };
            let Some(handlers) = group_object.get("hooks").and_then(Value::as_array) else {
                kept_groups.push(group.clone());
                continue;
            };
            let mut kept_handlers = Vec::with_capacity(handlers.len());
            for handler in handlers {
                let is_managed = handler
                    .as_object()
                    .is_some_and(|object| is_managed_handler(object, project_id));
                if is_managed {
                    removed += 1;
                } else {
                    kept_handlers.push(handler.clone());
                }
            }
            if kept_handlers.is_empty() && !handlers.is_empty() {
                continue;
            }
            let mut new_group = group_object.clone();
            new_group.insert("hooks".to_owned(), Value::Array(kept_handlers));
            kept_groups.push(Value::Object(new_group));
        }
        if removed == removed_before_event {
            continue;
        }
        if kept_groups.is_empty() {
            hooks.remove(&event);
        } else {
            hooks.insert(event, Value::Array(kept_groups));
        }
    }
    removed
}

pub(crate) fn preview_token(snapshot: &SettingsSnapshot, expected: &[PlannedEntry]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"xushu-claude-hook-preview-v1");
    hasher.update(normalized_path(&snapshot.project_root).as_bytes());
    for identity in [
        &snapshot.root_identity,
        &snapshot.settings_dir_identity,
        &snapshot.settings_identity,
    ] {
        hasher.update([0]);
        if let Some(identity) = identity {
            hasher.update(identity.as_bytes());
        }
    }
    hasher.update([0]);
    match &snapshot.raw {
        Some(raw) => {
            hasher.update(b"file");
            hasher.update(raw);
        }
        None => hasher.update(b"missing"),
    }
    hasher.update([0]);
    if let Ok(serialized) = serde_json::to_vec(expected) {
        hasher.update(serialized);
    }
    to_hex(&hasher.finalize())
}

pub(crate) struct WrittenSettings {
    bytes: Vec<u8>,
    identity: String,
    directory: Directories,
}

pub(crate) struct MutationLock {
    _file: File,
    _directory: Directories,
}

pub(crate) fn lock_mutation(
    backup_dir: &Path,
    project_id: &str,
) -> Result<MutationLock, CoreError> {
    let _parent = Directories::open(backup_dir)
        .map_err(|_| CoreError::Claude("无法确认 Hook 设置备份目录，已拒绝修改".to_owned()))?;
    let directory = backup_dir.join(project_id);
    let directory_guard = ensure_directory(&directory)?;
    let path = directory.join(".settings.lock");
    if fs::symlink_metadata(&path).is_ok_and(|metadata| redirected(&metadata)) {
        return Err(CoreError::Claude(
            SettingsError::UnsafePath.message().to_owned(),
        ));
    }
    let mut options = regular_file_options();
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        use windows_sys::Win32::Storage::FileSystem::{FILE_SHARE_READ, FILE_SHARE_WRITE};
        options.share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE);
    }
    let lock = options
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(path)?;
    let metadata = lock.metadata()?;
    if redirected(&metadata) || !metadata.is_file() {
        return Err(CoreError::Claude(
            SettingsError::UnsafePath.message().to_owned(),
        ));
    }
    match lock.try_lock() {
        Ok(()) => Ok(MutationLock {
            _file: lock,
            _directory: directory_guard,
        }),
        Err(std::fs::TryLockError::WouldBlock) => Err(CoreError::Conflict(
            "另一进程正在修改该项目的 Hook 配置，请稍后重试".to_owned(),
        )),
        Err(std::fs::TryLockError::Error(error)) => Err(error.into()),
    }
}

pub(crate) fn write_document(
    snapshot: &SettingsSnapshot,
    document: &Value,
    backup_dir: &Path,
    project_id: &str,
) -> Result<WrittenSettings, CoreError> {
    if let Some(error) = &snapshot.error {
        return Err(CoreError::Claude(error.message().to_owned()));
    }

    let mut serialized = serde_json::to_vec_pretty(document)
        .map_err(|error| CoreError::Claude(format!("设置文件序列化失败: {error}")))?;
    serialized.push(b'\n');
    if serialized.len() as u64 > MAX_SETTINGS_BYTES {
        return Err(CoreError::Claude(
            "生成的 settings.local.json 超过 1 MiB，已拒绝写入".to_owned(),
        ));
    }

    let _root = pin_root(snapshot)?;
    if snapshot.settings_dir_identity.is_none() {
        // An external creator winning this race invalidates the preview.
        fs::create_dir(&snapshot.settings_dir)?;
    }
    let directory = pin_settings_dir(snapshot, snapshot.settings_dir_identity.as_deref())?;
    let check = || {
        check_current(
            snapshot,
            project_id,
            &directory.identity,
            snapshot.raw.as_deref(),
            snapshot.settings_identity.as_deref(),
        )
    };
    check()?;
    if let Some(raw) = &snapshot.raw {
        backup_settings(backup_dir, project_id, raw)?;
    }
    let identity = write_atomic(&snapshot.settings_path, &serialized, check)?;
    Ok(WrittenSettings {
        bytes: serialized,
        identity,
        directory,
    })
}

pub(crate) fn restore_settings(
    snapshot: &SettingsSnapshot,
    written: &WrittenSettings,
    project_id: &str,
) -> Result<(), CoreError> {
    let _root = pin_root(snapshot)?;
    let directory = pin_settings_dir(snapshot, Some(&written.directory.identity))?;
    let check = || {
        check_current(
            snapshot,
            project_id,
            &directory.identity,
            Some(&written.bytes),
            Some(&written.identity),
        )
    };
    if snapshot.exists {
        let raw = snapshot
            .raw
            .as_ref()
            .ok_or_else(|| CoreError::Claude("备份内存不可用，无法恢复设置文件".to_owned()))?;
        write_atomic(&snapshot.settings_path, raw, check)?;
    } else {
        check()?;
        fs::remove_file(&snapshot.settings_path)?;
    }
    Ok(())
}

pub(super) fn pin_root(snapshot: &SettingsSnapshot) -> Result<Directories, CoreError> {
    let root = Directories::open(&snapshot.authorized_root)
        .map_err(|_| CoreError::Claude(SettingsError::UnsafePath.message().to_owned()))?;
    if snapshot.root_identity.as_deref() != Some(&root.identity) {
        return Err(CoreError::Claude(
            SettingsError::UnsafePath.message().to_owned(),
        ));
    }
    Ok(root)
}

fn pin_settings_dir(
    snapshot: &SettingsSnapshot,
    expected: Option<&str>,
) -> Result<Directories, CoreError> {
    let directory = Directories::open(&snapshot.settings_dir)
        .map_err(|_| CoreError::Claude(SettingsError::UnsafePath.message().to_owned()))?;
    if expected.is_some_and(|expected| expected != directory.identity) {
        return Err(CoreError::Claude(
            SettingsError::UnsafePath.message().to_owned(),
        ));
    }
    Ok(directory)
}

fn check_current(
    snapshot: &SettingsSnapshot,
    project_id: &str,
    directory_identity: &str,
    raw: Option<&[u8]>,
    file_identity: Option<&str>,
) -> Result<(), CoreError> {
    let current = read_snapshot(&snapshot.authorized_root, project_id);
    if current.error.is_some()
        || current.root_identity != snapshot.root_identity
        || current.settings_dir_identity.as_deref() != Some(directory_identity)
        || current.raw.as_deref() != raw
        || current.settings_identity.as_deref() != file_identity
    {
        return Err(CoreError::Conflict(
            "项目设置路径或内容已变化，请检查设置和备份后重试".to_owned(),
        ));
    }
    Ok(())
}

fn write_atomic(
    path: &Path,
    bytes: &[u8],
    check: impl FnOnce() -> Result<(), CoreError>,
) -> Result<String, CoreError> {
    let parent = path
        .parent()
        .ok_or_else(|| CoreError::Claude("设置文件路径无效".to_owned()))?;
    let temporary = parent.join(format!(".settings.local.json.xushu-{}.tmp", Uuid::new_v4()));
    let mut file = regular_file_options()
        .write(true)
        .create_new(true)
        .open(&temporary)?;
    let result = (|| {
        file.write_all(bytes)?;
        file.sync_all()?;
        let identity = identity(&file)?;
        drop(file);
        check()?;
        fs::rename(&temporary, path)?;
        Ok(identity)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

fn backup_settings(backup_dir: &Path, project_id: &str, raw: &[u8]) -> Result<(), CoreError> {
    let _parent = Directories::open(backup_dir)?;
    let directory = backup_dir.join(project_id);
    let _directory = ensure_directory(&directory)?;
    let timestamp = chrono::Utc::now().format("%Y%m%dT%H%M%S%.3fZ");
    let path = directory.join(format!(
        "settings.local.json-{timestamp}-{}.bak",
        Uuid::new_v4()
    ));
    let mut file = regular_file_options()
        .write(true)
        .create_new(true)
        .open(&path)?;
    file.write_all(raw)?;
    file.sync_all()?;
    drop(file);
    prune_backups(&directory);
    Ok(())
}

fn prune_backups(directory: &Path) {
    let Ok(entries) = fs::read_dir(directory) else {
        return;
    };
    let mut backups: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| {
                    name.starts_with("settings.local.json-") && name.ends_with(".bak")
                })
        })
        .collect();
    backups.sort();
    if backups.len() <= BACKUP_KEEP {
        return;
    }
    for path in &backups[..backups.len() - BACKUP_KEEP] {
        let _ = fs::remove_file(path);
    }
}

fn ensure_directory(path: &Path) -> Result<Directories, CoreError> {
    match fs::create_dir(path) {
        Ok(()) => {}
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
        Err(error) => return Err(error.into()),
    }
    Directories::open(path)
        .map_err(|_| CoreError::Claude(SettingsError::UnsafePath.message().to_owned()))
}

fn normalized_path(path: &Path) -> String {
    let value = path.to_string_lossy().to_string();
    #[cfg(windows)]
    {
        value.replace('/', "\\").to_lowercase()
    }
    #[cfg(not(windows))]
    {
        value
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rollback_preserves_settings_changed_after_our_write() {
        let root = tempfile::TempDir::new().unwrap();
        let backup = root.path().join("backups");
        let project_id = "11111111-1111-4111-8111-111111111111";
        let original = read_snapshot(root.path(), project_id);
        let written = write_document(&original, &json!({}), &backup, project_id).unwrap();
        let external_edit = br#"{"permissions":{"deny":["Read"]}}"#;
        fs::write(&original.settings_path, external_edit).unwrap();

        assert!(restore_settings(&original, &written, project_id).is_err());
        assert_eq!(fs::read(&original.settings_path).unwrap(), external_edit);
    }
}
