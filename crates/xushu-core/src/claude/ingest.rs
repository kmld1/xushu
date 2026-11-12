use std::cmp::Ordering;
use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Duration, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use uuid::Uuid;
use xushu_hook_protocol::{
    is_link_or_reparse, project_root_fingerprint, HookEnvelope, HookEventType, MAX_SPOOL_FILE_BYTES,
};

use crate::db::{now_utc_rfc3339, Database};
use crate::error::CoreError;

use super::settings::read_snapshot;

use super::{
    ClaudeActivityEvent, ClaudeActivitySnapshot, ClaudeImportReport, ClaudeRuntimePaths,
    ClaudeSessionSummary, ClaudeToolCallSummary, ACTIVITY_EVENT_LIMIT, ACTIVITY_SESSION_LIMIT,
    ACTIVITY_TOOL_CALL_LIMIT, EVENT_RETENTION_DAYS, IMPORT_BATCH_LIMIT, MAX_EVENTS_PER_PROJECT,
    MAX_SESSIONS_PER_PROJECT,
};

#[derive(Debug, Clone)]
pub(crate) struct ProjectRow {
    pub id: String,
    pub name: String,
    pub root_path: String,
    pub archived_at: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct IntegrationRow {
    pub desired_enabled: bool,
    pub capture_generation: i64,
    pub project_root_identity: Option<String>,
    pub hook_version: Option<String>,
    pub hook_binary_path: Option<String>,
    pub hook_binary_digest: Option<String>,
    pub installed_at: Option<String>,
    pub uninstalled_at: Option<String>,
    pub last_import_at: Option<String>,
    pub last_error_code: Option<String>,
    pub last_error_at: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct ProjectCounts {
    pub event_count: i64,
    pub session_count: i64,
    pub running_tool_count: i64,
    pub last_event_at: Option<String>,
    pub last_event_type: Option<String>,
}

pub(crate) fn load_project(
    connection: &Connection,
    project_id: &str,
) -> Result<Option<ProjectRow>, CoreError> {
    connection
        .query_row(
            "SELECT id, name, root_path, archived_at FROM projects WHERE id = ?1",
            [project_id],
            |row| {
                Ok(ProjectRow {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    root_path: row.get(2)?,
                    archived_at: row.get(3)?,
                })
            },
        )
        .optional()
        .map_err(Into::into)
}

pub(crate) fn load_integration(
    connection: &Connection,
    project_id: &str,
) -> Result<Option<IntegrationRow>, CoreError> {
    connection
        .query_row(
            "SELECT desired_enabled, hook_version, hook_binary_path, hook_binary_digest,
                    installed_at, uninstalled_at, last_import_at, last_error_code, last_error_at,
                    capture_generation, project_root_identity
             FROM claude_integrations WHERE project_id = ?1",
            [project_id],
            |row| {
                Ok(IntegrationRow {
                    desired_enabled: row.get::<_, i64>(0)? == 1,
                    hook_version: row.get(1)?,
                    hook_binary_path: row.get(2)?,
                    hook_binary_digest: row.get(3)?,
                    installed_at: row.get(4)?,
                    uninstalled_at: row.get(5)?,
                    last_import_at: row.get(6)?,
                    last_error_code: row.get(7)?,
                    last_error_at: row.get(8)?,
                    capture_generation: row.get(9)?,
                    project_root_identity: row.get(10)?,
                })
            },
        )
        .optional()
        .map_err(Into::into)
}

pub(crate) fn project_counts(
    connection: &Connection,
    project_id: &str,
) -> Result<ProjectCounts, CoreError> {
    let event_count = connection.query_row(
        "SELECT COUNT(*) FROM claude_events WHERE project_id = ?1",
        [project_id],
        |row| row.get(0),
    )?;
    let session_count = connection.query_row(
        "SELECT COUNT(*) FROM claude_sessions WHERE project_id = ?1",
        [project_id],
        |row| row.get(0),
    )?;
    let running_tool_count = connection.query_row(
        "SELECT COUNT(*) FROM claude_tool_calls WHERE project_id = ?1 AND status = 'running'",
        [project_id],
        |row| row.get(0),
    )?;
    let last = connection
        .query_row(
            "SELECT event_type, occurred_at FROM claude_events
             WHERE project_id = ?1
             ORDER BY occurred_at DESC, rowid DESC LIMIT 1",
            [project_id],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
        )
        .optional()?;
    Ok(ProjectCounts {
        event_count,
        session_count,
        running_tool_count,
        last_event_at: last.as_ref().map(|(_, at)| at.clone()),
        last_event_type: last.map(|(event_type, _)| event_type),
    })
}

pub(crate) fn import_pending(
    database: &Database,
    runtime: &ClaudeRuntimePaths,
    project_id: &str,
) -> Result<ClaudeImportReport, CoreError> {
    let (project, integration) = {
        let connection = database.lock()?;
        (
            load_project(&connection, project_id)?.ok_or(CoreError::NotFound)?,
            load_integration(&connection, project_id)?,
        )
    };

    let project_dir = runtime.pending_project_dir(project_id);
    if is_link_or_reparse(runtime.inbox_dir())
        || is_link_or_reparse(&runtime.inbox_dir().join("pending"))
        || is_link_or_reparse(&project_dir)
    {
        let now = now_utc_rfc3339();
        update_import_status(
            database,
            project_id,
            &now,
            Some("spool_unsafe_path"),
            Some(&now),
        )?;
        return Ok(ClaudeImportReport {
            project_id: project_id.to_owned(),
            last_error_code: Some("spool_unsafe_path".to_owned()),
            ..ClaudeImportReport::default()
        });
    }
    let files = list_spool_files(&project_dir);
    let enabled = integration.as_ref().is_some_and(|row| row.desired_enabled);
    let generation = integration.as_ref().map_or(0, |row| row.capture_generation);
    let expected_fingerprint = if enabled && !files.is_empty() {
        let snapshot = read_snapshot(Path::new(&project.root_path), project_id).require_root(
            integration
                .as_ref()
                .and_then(|row| row.project_root_identity.as_deref()),
        );
        match runtime.salt() {
            Some(salt) if snapshot.error.is_none() && snapshot.has_generation(generation) => {
                Some(project_root_fingerprint(salt, &snapshot.project_root))
            }
            _ => {
                // Configuration may already have been removed while a database update is pending.
                // Preserve the files here. A committed disable or fresh generation will discard them.
                return Ok(ClaudeImportReport {
                    project_id: project_id.to_owned(),
                    remaining: files.len(),
                    last_error_code: Some("capture_unavailable".to_owned()),
                    ..ClaudeImportReport::default()
                });
            }
        }
    } else {
        None
    };

    let mut report = ClaudeImportReport {
        project_id: project_id.to_owned(),
        ..ClaudeImportReport::default()
    };
    let mut first_rejection: Option<&'static str> = None;

    for file in files.into_iter().take(IMPORT_BATCH_LIMIT) {
        match import_one_file(
            database,
            project_id,
            generation,
            expected_fingerprint.as_deref(),
            &file,
        ) {
            Ok(ImportDisposition::Imported) => {
                report.imported += 1;
                let _ = fs::remove_file(&file);
            }
            Ok(ImportDisposition::Duplicate) => {
                report.duplicates += 1;
                let _ = fs::remove_file(&file);
            }
            Ok(ImportDisposition::Ignored) => {
                report.ignored += 1;
                let _ = fs::remove_file(&file);
            }
            Ok(ImportDisposition::Pending) => {}
            Ok(ImportDisposition::Rejected(code)) => {
                report.rejected += 1;
                first_rejection.get_or_insert(code);
                let _ = fs::remove_file(&file);
            }
            Err(error) => {
                // 数据库或不可恢复错误：保留 spool 文件，等待下一次导入。
                if let Err(status_error) = update_import_status(
                    database,
                    project_id,
                    &now_utc_rfc3339(),
                    Some(error.code()),
                    Some(&now_utc_rfc3339()),
                ) {
                    eprintln!(
                        "[xushu] Hook import status write failed: code={}",
                        status_error.code()
                    );
                }
                return Err(error);
            }
        }
    }

    report.remaining = count_spool_files(&project_dir);
    let now = now_utc_rfc3339();
    let error_code = first_rejection.map(str::to_owned);
    report.last_error_code = error_code.clone();
    update_import_status(
        database,
        project_id,
        &now,
        error_code.as_deref(),
        if error_code.is_some() {
            Some(&now)
        } else {
            None
        },
    )?;
    prune(database, project_id)?;
    Ok(report)
}

enum ImportDisposition {
    Imported,
    Duplicate,
    Ignored,
    Pending,
    Rejected(&'static str),
}

fn list_spool_files(directory: &std::path::Path) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(directory) else {
        return Vec::new();
    };
    let mut files: Vec<PathBuf> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|value| value.to_str()) == Some("json"))
        .collect();
    files.sort();
    files
}

fn count_spool_files(directory: &std::path::Path) -> usize {
    list_spool_files(directory).len()
}

fn import_one_file(
    database: &Database,
    project_id: &str,
    observed_generation: i64,
    expected_fingerprint: Option<&str>,
    file: &std::path::Path,
) -> Result<ImportDisposition, CoreError> {
    if is_link_or_reparse(file) || !file.is_file() {
        return Ok(ImportDisposition::Rejected("spool_unavailable"));
    }
    let handle = match File::open(file) {
        Ok(handle) => handle,
        #[cfg(windows)]
        Err(error) if error.raw_os_error() == Some(32) => return Ok(ImportDisposition::Pending),
        Err(_) => return Ok(ImportDisposition::Rejected("spool_unavailable")),
    };
    let mut bytes = Vec::new();
    if handle
        .take(MAX_SPOOL_FILE_BYTES + 1)
        .read_to_end(&mut bytes)
        .is_err()
    {
        return Ok(ImportDisposition::Rejected("spool_unavailable"));
    }
    if bytes.len() as u64 > MAX_SPOOL_FILE_BYTES {
        return Ok(ImportDisposition::Rejected("spool_too_large"));
    }
    let envelope: HookEnvelope = match serde_json::from_slice(&bytes) {
        Ok(envelope) => envelope,
        Err(_) => return Ok(ImportDisposition::Rejected("invalid_envelope")),
    };
    if envelope.validate().is_err() {
        return Ok(ImportDisposition::Rejected("invalid_envelope"));
    }
    if envelope.project_id != project_id {
        return Ok(ImportDisposition::Rejected("project_mismatch"));
    }
    let now = now_utc_rfc3339();
    database.with_transaction(|transaction| {
        let integration = load_integration(transaction, project_id)?;
        let Some(integration) = integration else {
            return Ok(ImportDisposition::Ignored);
        };
        if integration.capture_generation != observed_generation {
            return Ok(ImportDisposition::Pending);
        }
        if !integration.desired_enabled {
            return Ok(ImportDisposition::Ignored);
        }
        if expected_fingerprint != Some(envelope.project_fingerprint.as_str()) {
            return Ok(ImportDisposition::Rejected("project_mismatch"));
        }
        if envelope.capture_generation != observed_generation {
            return Ok(ImportDisposition::Ignored);
        }
        apply_envelope(transaction, project_id, &envelope, &now)
    })
}

fn apply_envelope(
    transaction: &rusqlite::Transaction<'_>,
    project_id: &str,
    envelope: &HookEnvelope,
    now: &str,
) -> Result<ImportDisposition, CoreError> {
    if event_exists(transaction, &envelope.delivery_id)? {
        return Ok(ImportDisposition::Duplicate);
    }
    let dedupe_key = envelope
        .semantic_dedupe_key()
        .map(|key| format!("{project_id}|{key}"));
    if let Some(key) = &dedupe_key {
        let exists: bool = transaction.query_row(
            "SELECT EXISTS(SELECT 1 FROM claude_events WHERE dedupe_key = ?1)",
            [key],
            |row| row.get(0),
        )?;
        if exists {
            return Ok(ImportDisposition::Duplicate);
        }
    }

    let session_id = ensure_session(transaction, project_id, envelope, now)?;
    let tool_call_id = if envelope.event_type.is_tool_event() {
        Some(upsert_tool_call(transaction, &session_id, envelope, now)?)
    } else {
        None
    };

    if envelope.event_type == HookEventType::SessionEnd {
        mark_running_tools_unknown(transaction, &session_id, now)?;
    }

    let inserted = transaction.execute(
        "INSERT OR IGNORE INTO claude_events (
            id, delivery_id, dedupe_key, project_id, session_id, tool_call_id,
            event_type, occurred_at, ingested_at, prompt_hash, tool_name, tool_category,
            failure_kind, is_interrupt, duration_ms, source, end_reason, created_at
         ) VALUES (
            ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18
         )",
        params![
            Uuid::new_v4().to_string(),
            envelope.delivery_id,
            dedupe_key,
            project_id,
            session_id,
            tool_call_id,
            envelope.event_type.as_str(),
            envelope.captured_at,
            now,
            envelope.prompt_hash,
            envelope.tool_name,
            envelope.tool_category.map(|category| category.as_str()),
            envelope.failure_kind.map(|kind| kind.as_str()),
            envelope.is_interrupt.map(|value| if value { 1 } else { 0 }),
            envelope.duration_ms,
            envelope.source.map(|source| source.as_str()),
            envelope.end_reason.map(|reason| reason.as_str()),
            now,
        ],
    )?;

    if inserted == 0 {
        return Ok(ImportDisposition::Duplicate);
    }
    Ok(ImportDisposition::Imported)
}

fn event_exists(
    transaction: &rusqlite::Transaction<'_>,
    delivery_id: &str,
) -> Result<bool, CoreError> {
    transaction
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM claude_events WHERE delivery_id = ?1)",
            [delivery_id],
            |row| row.get(0),
        )
        .map_err(Into::into)
}

fn ensure_session(
    transaction: &rusqlite::Transaction<'_>,
    project_id: &str,
    envelope: &HookEnvelope,
    now: &str,
) -> Result<String, CoreError> {
    let existing = transaction
        .query_row(
            "SELECT id, ended_at, last_event_at, start_observed
             FROM claude_sessions WHERE project_id = ?1 AND session_hash = ?2",
            params![project_id, envelope.session_hash],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, Option<String>>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, i64>(3)? == 1,
                ))
            },
        )
        .optional()?;

    let source = envelope
        .source
        .map(|source| source.as_str())
        .unwrap_or("unknown");
    let is_start = envelope.event_type == HookEventType::SessionStart;
    let is_end = envelope.event_type == HookEventType::SessionEnd;

    let Some((id, ended_at, last_event_at, start_observed)) = existing else {
        let id = Uuid::new_v4().to_string();
        let ended_at = is_end.then(|| envelope.captured_at.clone());
        let end_reason = is_end
            .then(|| envelope.end_reason.map(|reason| reason.as_str()))
            .flatten();
        transaction.execute(
            "INSERT INTO claude_sessions (
                id, project_id, session_hash, started_at, last_event_at, ended_at,
                start_source, end_reason, start_observed, created_at, updated_at
             ) VALUES (?1, ?2, ?3, ?4, ?4, ?5, ?6, ?7, ?8, ?9, ?9)",
            params![
                id,
                project_id,
                envelope.session_hash,
                envelope.captured_at,
                ended_at,
                source,
                end_reason,
                if is_start { 1 } else { 0 },
                now,
            ],
        )?;
        return Ok(id);
    };

    let last_event_at = later_timestamp(&last_event_at, &envelope.captured_at);
    let mut new_ended_at = ended_at.clone();
    let mut new_start_observed = start_observed;
    let mut reopen = false;

    if is_start && (ended_at.is_none() || timestamp_after(&envelope.captured_at, &ended_at)) {
        new_ended_at = None;
        new_start_observed = true;
        reopen = true;
    } else if is_start {
        new_start_observed = true;
    }
    if is_end {
        new_ended_at = Some(match &ended_at {
            Some(existing) => later_timestamp(existing, &envelope.captured_at),
            None => envelope.captured_at.clone(),
        });
    }

    let end_reason = if is_end {
        envelope.end_reason.map(|reason| reason.as_str())
    } else {
        None
    };

    transaction.execute(
        "UPDATE claude_sessions
         SET last_event_at = ?2,
             ended_at = ?3,
             start_observed = ?4,
             start_source = CASE WHEN ?5 THEN ?6 ELSE start_source END,
             end_reason = CASE WHEN ?7 THEN ?8 ELSE end_reason END,
             updated_at = ?9
         WHERE id = ?1",
        params![
            id,
            last_event_at,
            new_ended_at,
            if new_start_observed { 1 } else { 0 },
            if is_start { 1 } else { 0 },
            source,
            if is_end || reopen { 1 } else { 0 },
            end_reason,
            now,
        ],
    )?;

    Ok(id)
}

#[derive(Debug)]
struct ToolCallState {
    id: String,
    status: String,
    started_at: Option<String>,
    finished_at: Option<String>,
    duration_ms: Option<i64>,
    start_observed: bool,
    finish_observed: bool,
    conflict: bool,
}

fn upsert_tool_call(
    transaction: &rusqlite::Transaction<'_>,
    session_id: &str,
    envelope: &HookEnvelope,
    now: &str,
) -> Result<String, CoreError> {
    let tool_use_hash = envelope
        .tool_use_hash
        .as_deref()
        .ok_or_else(|| CoreError::Validation("工具事件缺少 tool_use_id".to_owned()))?;
    let tool_name = envelope
        .tool_name
        .as_deref()
        .ok_or_else(|| CoreError::Validation("工具事件缺少工具名称".to_owned()))?;
    let tool_category = envelope
        .tool_category
        .map(|category| category.as_str())
        .ok_or_else(|| CoreError::Validation("工具事件缺少工具类别".to_owned()))?;
    let session_ended: bool = transaction.query_row(
        "SELECT ended_at IS NOT NULL FROM claude_sessions WHERE id = ?1",
        [session_id],
        |row| row.get::<_, i64>(0).map(|value| value == 1),
    )?;

    let existing = transaction
        .query_row(
            "SELECT id, status, started_at, finished_at, duration_ms,
                    start_observed, finish_observed, conflict
             FROM claude_tool_calls WHERE session_id = ?1 AND tool_use_hash = ?2",
            params![session_id, tool_use_hash],
            |row| {
                Ok(ToolCallState {
                    id: row.get(0)?,
                    status: row.get(1)?,
                    started_at: row.get(2)?,
                    finished_at: row.get(3)?,
                    duration_ms: row.get(4)?,
                    start_observed: row.get::<_, i64>(5)? == 1,
                    finish_observed: row.get::<_, i64>(6)? == 1,
                    conflict: row.get::<_, i64>(7)? == 1,
                })
            },
        )
        .optional()?;

    let Some(state) = existing else {
        let id = Uuid::new_v4().to_string();
        let (status, started_at, finished_at, start_observed, finish_observed, conflict) =
            match envelope.event_type {
                HookEventType::PreToolUse => (
                    if session_ended { "unknown" } else { "running" },
                    Some(envelope.captured_at.clone()),
                    None,
                    true,
                    false,
                    false,
                ),
                HookEventType::PostToolUse => (
                    "succeeded",
                    None,
                    Some(envelope.captured_at.clone()),
                    false,
                    true,
                    false,
                ),
                HookEventType::PostToolUseFailure => (
                    "failed",
                    None,
                    Some(envelope.captured_at.clone()),
                    false,
                    true,
                    false,
                ),
                _ => ("unknown", None, None, false, false, false),
            };
        transaction.execute(
            "INSERT INTO claude_tool_calls (
                id, project_id, session_id, tool_use_hash, tool_name, tool_category,
                status, failure_kind, started_at, finished_at, duration_ms,
                start_observed, finish_observed, conflict, created_at, updated_at
             ) VALUES (
                ?1, (SELECT project_id FROM claude_sessions WHERE id = ?2), ?2, ?3, ?4, ?5,
                ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?14
             )",
            params![
                id,
                session_id,
                tool_use_hash,
                tool_name,
                tool_category,
                status,
                envelope.failure_kind.map(|kind| kind.as_str()),
                started_at,
                finished_at,
                envelope.duration_ms,
                bool_to_int(start_observed),
                bool_to_int(finish_observed),
                bool_to_int(conflict),
                now,
            ],
        )?;
        return Ok(id);
    };

    let mut status = state.status.clone();
    let mut failure_kind: Option<&str> = envelope.failure_kind.map(|kind| kind.as_str());
    let mut finished_at = state.finished_at.clone();
    let mut finish_observed = state.finish_observed;
    let mut start_observed = state.start_observed;
    let mut started_at = state.started_at.clone();
    let mut conflict = state.conflict;

    match envelope.event_type {
        HookEventType::PreToolUse => {
            if started_at.is_none() && finished_at.is_none() {
                started_at = Some(envelope.captured_at.clone());
            }
            start_observed = true;
            if session_ended && status == "running" {
                status = "unknown".to_owned();
            }
            // 不把已经成功的调用降级回 running。
        }
        HookEventType::PostToolUse => {
            finished_at = Some(match &finished_at {
                Some(existing) => later_timestamp(existing, &envelope.captured_at),
                None => envelope.captured_at.clone(),
            });
            finish_observed = true;
            if status == "running" {
                status = "succeeded".to_owned();
            } else if status == "failed" || status == "unknown" {
                status = "unknown".to_owned();
                conflict = true;
            }
            failure_kind = None;
        }
        HookEventType::PostToolUseFailure => {
            finished_at = Some(match &finished_at {
                Some(existing) => later_timestamp(existing, &envelope.captured_at),
                None => envelope.captured_at.clone(),
            });
            finish_observed = true;
            if status == "running" {
                status = "failed".to_owned();
            } else if status == "succeeded" || status == "unknown" {
                status = "unknown".to_owned();
                conflict = true;
            }
        }
        _ => {}
    }

    let duration = state.duration_ms.or(envelope.duration_ms);
    transaction.execute(
        "UPDATE claude_tool_calls
         SET tool_name = ?2,
             tool_category = ?3,
             status = ?4,
             failure_kind = CASE WHEN ?5 IS NULL THEN failure_kind ELSE ?5 END,
             started_at = ?6,
             finished_at = ?7,
             duration_ms = ?8,
             start_observed = ?9,
             finish_observed = ?10,
             conflict = ?11,
             updated_at = ?12
         WHERE id = ?1",
        params![
            state.id,
            tool_name,
            tool_category,
            status,
            failure_kind,
            started_at,
            finished_at,
            duration,
            bool_to_int(start_observed),
            bool_to_int(finish_observed),
            bool_to_int(conflict),
            now,
        ],
    )?;

    Ok(state.id)
}

fn mark_running_tools_unknown(
    transaction: &rusqlite::Transaction<'_>,
    session_id: &str,
    now: &str,
) -> Result<(), CoreError> {
    transaction.execute(
        "UPDATE claude_tool_calls
         SET status = 'unknown', updated_at = ?2
         WHERE session_id = ?1 AND status = 'running'",
        params![session_id, now],
    )?;
    Ok(())
}

fn later_timestamp(left: &str, right: &str) -> String {
    match compare_timestamp(left, right) {
        Ordering::Greater => left.to_owned(),
        _ => right.to_owned(),
    }
}

fn timestamp_after(left: &str, right: &Option<String>) -> bool {
    match right {
        Some(right) => compare_timestamp(left, right) == Ordering::Greater,
        None => true,
    }
}

fn compare_timestamp(left: &str, right: &str) -> Ordering {
    match (
        DateTime::parse_from_rfc3339(left),
        DateTime::parse_from_rfc3339(right),
    ) {
        (Ok(left), Ok(right)) => left.cmp(&right),
        _ => left.cmp(right),
    }
}

fn update_import_status(
    database: &Database,
    project_id: &str,
    now: &str,
    error_code: Option<&str>,
    error_at: Option<&str>,
) -> Result<(), CoreError> {
    let connection = database.lock()?;
    connection.execute(
        "UPDATE claude_integrations
         SET last_import_at = ?2,
             last_error_code = ?3,
             last_error_at = ?4,
             last_checked_at = ?2,
             updated_at = ?2
         WHERE project_id = ?1",
        params![project_id, now, error_code, error_at],
    )?;
    Ok(())
}

fn prune(database: &Database, project_id: &str) -> Result<(), CoreError> {
    let cutoff = (Utc::now() - Duration::days(EVENT_RETENTION_DAYS))
        .to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
    database.with_transaction(|transaction| {
        transaction.execute(
            "DELETE FROM claude_events WHERE project_id = ?1 AND occurred_at < ?2",
            params![project_id, cutoff],
        )?;
        transaction.execute(
            "DELETE FROM claude_tool_calls
             WHERE project_id = ?1
               AND COALESCE(started_at, finished_at, created_at) < ?2",
            params![project_id, cutoff],
        )?;
        transaction.execute(
            "DELETE FROM claude_sessions WHERE project_id = ?1 AND last_event_at < ?2",
            params![project_id, cutoff],
        )?;
        transaction.execute(
            "DELETE FROM claude_events
             WHERE id IN (
                SELECT id FROM claude_events
                WHERE project_id = ?1
                ORDER BY occurred_at DESC, rowid DESC
                LIMIT -1 OFFSET ?2
             )",
            params![project_id, MAX_EVENTS_PER_PROJECT],
        )?;
        transaction.execute(
            "DELETE FROM claude_sessions
             WHERE id IN (
                SELECT id FROM claude_sessions
                WHERE project_id = ?1
                ORDER BY last_event_at DESC, rowid DESC
                LIMIT -1 OFFSET ?2
             )",
            params![project_id, MAX_SESSIONS_PER_PROJECT],
        )?;
        Ok(())
    })
}

pub(crate) fn activity(
    database: &Database,
    runtime: &ClaudeRuntimePaths,
    project_id: &str,
) -> Result<ClaudeActivitySnapshot, CoreError> {
    let connection = database.lock()?;
    let now = now_utc_rfc3339();
    let active_cutoff =
        (Utc::now() - Duration::minutes(10)).to_rfc3339_opts(chrono::SecondsFormat::Millis, true);

    let mut sessions_statement = connection.prepare(
        "SELECT id, session_hash, started_at, last_event_at, ended_at,
                start_source, end_reason, start_observed
         FROM claude_sessions
         WHERE project_id = ?1
         ORDER BY last_event_at DESC, rowid DESC
         LIMIT ?2",
    )?;
    let session_rows =
        sessions_statement.query_map(params![project_id, ACTIVITY_SESSION_LIMIT], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, Option<String>>(6)?,
                row.get::<_, i64>(7)? == 1,
            ))
        })?;
    let mut sessions = Vec::new();
    for row in session_rows {
        let (
            id,
            session_hash,
            started_at,
            last_event_at,
            ended_at,
            start_source,
            end_reason,
            start_observed,
        ) = row?;
        let event_count: i64 = connection.query_row(
            "SELECT COUNT(*) FROM claude_events WHERE session_id = ?1",
            [&id],
            |row| row.get(0),
        )?;
        let tool_call_count: i64 = connection.query_row(
            "SELECT COUNT(*) FROM claude_tool_calls WHERE session_id = ?1",
            [&id],
            |row| row.get(0),
        )?;
        let running_tool_count: i64 = connection.query_row(
            "SELECT COUNT(*) FROM claude_tool_calls WHERE session_id = ?1 AND status = 'running'",
            [&id],
            |row| row.get(0),
        )?;
        let failed_tool_count: i64 = connection.query_row(
            "SELECT COUNT(*) FROM claude_tool_calls WHERE session_id = ?1 AND status = 'failed'",
            [&id],
            |row| row.get(0),
        )?;
        let unknown_tool_count: i64 = connection.query_row(
            "SELECT COUNT(*) FROM claude_tool_calls WHERE session_id = ?1 AND status = 'unknown'",
            [&id],
            |row| row.get(0),
        )?;
        let activity_state = if ended_at.is_some() {
            "ended"
        } else if !start_observed {
            "unknown"
        } else if last_event_at >= active_cutoff {
            "active"
        } else {
            "unknown"
        };
        sessions.push(ClaudeSessionSummary {
            id,
            session_ref: short_ref(&session_hash),
            started_at,
            last_event_at,
            ended_at,
            start_source,
            end_reason,
            start_observed,
            activity_state: activity_state.to_owned(),
            event_count,
            tool_call_count,
            running_tool_count,
            failed_tool_count,
            unknown_tool_count,
        });
    }

    let mut events_statement = connection.prepare(
        "SELECT e.id, e.session_id, s.session_hash, e.event_type, e.occurred_at,
                e.tool_name, e.tool_category, e.failure_kind, e.duration_ms,
                e.source, e.end_reason, t.status, t.failure_kind
         FROM claude_events e
         JOIN claude_sessions s ON s.id = e.session_id
         LEFT JOIN claude_tool_calls t ON t.id = e.tool_call_id
         WHERE e.project_id = ?1
         ORDER BY e.occurred_at DESC, e.rowid DESC
         LIMIT ?2",
    )?;
    let event_rows =
        events_statement.query_map(params![project_id, ACTIVITY_EVENT_LIMIT], |row| {
            let event_type: String = row.get(3)?;
            let tool_status: Option<String> = row.get(11)?;
            let status = match event_type.as_str() {
                "session_start" | "pre_tool_use" | "stop" => "observed".to_owned(),
                "session_end" => "ended".to_owned(),
                _ => tool_status.unwrap_or_else(|| "unknown".to_owned()),
            };
            let tool_failure: Option<String> = row.get(12)?;
            let event_failure: Option<String> = row.get(7)?;
            let failure_kind = if event_type == "pre_tool_use" {
                None
            } else {
                tool_failure.or(event_failure)
            };
            Ok(ClaudeActivityEvent {
                id: row.get(0)?,
                session_id: row.get(1)?,
                session_ref: short_ref(&row.get::<_, String>(2)?),
                event_type,
                occurred_at: row.get(4)?,
                tool_name: row.get(5)?,
                tool_category: row.get(6)?,
                status,
                failure_kind,
                duration_ms: row.get(8)?,
                source: row.get(9)?,
                end_reason: row.get(10)?,
            })
        })?;
    let events = event_rows.collect::<Result<Vec<_>, _>>()?;

    let mut tools_statement = connection.prepare(
        "SELECT t.id, t.session_id, s.session_hash, t.tool_name, t.tool_category,
                t.status, t.failure_kind, t.started_at, t.finished_at, t.duration_ms,
                t.start_observed, t.finish_observed, t.conflict
         FROM claude_tool_calls t
         JOIN claude_sessions s ON s.id = t.session_id
         WHERE t.project_id = ?1
         ORDER BY COALESCE(t.started_at, t.finished_at, t.created_at) DESC, t.rowid DESC
         LIMIT ?2",
    )?;
    let tool_rows =
        tools_statement.query_map(params![project_id, ACTIVITY_TOOL_CALL_LIMIT], |row| {
            Ok(ClaudeToolCallSummary {
                id: row.get(0)?,
                session_id: row.get(1)?,
                session_ref: short_ref(&row.get::<_, String>(2)?),
                tool_name: row.get(3)?,
                tool_category: row.get(4)?,
                status: row.get(5)?,
                failure_kind: row.get(6)?,
                started_at: row.get(7)?,
                finished_at: row.get(8)?,
                duration_ms: row.get(9)?,
                start_observed: row.get::<_, i64>(10)? == 1,
                finish_observed: row.get::<_, i64>(11)? == 1,
                conflict: row.get::<_, i64>(12)? == 1,
            })
        })?;
    let tool_calls = tool_rows.collect::<Result<Vec<_>, _>>()?;

    Ok(ClaudeActivitySnapshot {
        project_id: project_id.to_owned(),
        generated_at: now,
        pending_event_count: runtime.pending_count(project_id),
        sessions,
        events,
        tool_calls,
    })
}

fn short_ref(hash: &str) -> String {
    hash.chars().take(12).collect()
}

fn bool_to_int(value: bool) -> i64 {
    if value {
        1
    } else {
        0
    }
}
