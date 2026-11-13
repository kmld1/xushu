//! 跨会话恢复上下文。
//!
//! 恢复包完全由本地数据库、只读 Git 状态与独立验收结果确定性生成，不依赖 AI。
//! 快照记录生成时的指纹；当前数据变化时，旧快照会显示为过期，而不是继续冒充
//! 当前状态。记忆与决策只是上下文，不能覆盖任务状态或验收证据。

use std::fs;
use std::path::Path;

use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::code_state::{self, CodeState};
use crate::db::{now_utc_rfc3339, Database};
use crate::decision::DecisionStatus;
use crate::error::CoreError;
use crate::memory::{MemoryCategory, MemoryConfirmationStatus, MemorySourceType};
use crate::models::{CompletionStatus, Project, Task, TaskPriority, TaskStatus};
use crate::project::ProjectRepository;
use crate::task::{criteria_with_completion, TaskRepository};
use crate::validation::validate_uuid;
use crate::verification::VerificationResult;

const ACTIVE_SECTION_LIMIT: usize = 5;
const DONE_SECTION_LIMIT: usize = 10;
const VERIFICATION_LIMIT: usize = 20;
const MEMORY_LIMIT: i64 = 100;
const DECISION_LIMIT: i64 = 50;
const NOTICE_LIMIT: i64 = 10;
const SNAPSHOT_RETENTION: i64 = 20;
const FILES_TO_READ_LIMIT: usize = 8;
const MARKDOWN_CONTENT_MAX: usize = 4_000;
const MAX_SNAPSHOT_MARKDOWN_BYTES: usize = 4 * 1024 * 1024;
const MAX_SNAPSHOT_PACKAGE_BYTES: usize = 8 * 1024 * 1024;
const TASK_CANDIDATE_LIMIT: i64 = 200;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecoveryGitState {
    Available,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecoveryNextStepKind {
    UserSelected,
    Suggested,
    NeedsConfirmation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecoveryFreshnessState {
    Fresh,
    Stale,
    NoSnapshot,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecoveryProject {
    pub id: String,
    pub name: String,
    pub root_path: String,
    pub archived: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecoveryGit {
    pub state: RecoveryGitState,
    pub checked_at: Option<String>,
    pub branch: Option<String>,
    pub head: Option<String>,
    pub dirty: bool,
    pub detached: bool,
    pub unborn: bool,
    pub workspace_fingerprint: Option<String>,
    pub fingerprint_complete: bool,
    pub changed_file_count: usize,
    pub note: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecoveryTaskItem {
    pub id: String,
    pub title: String,
    pub status: TaskStatus,
    pub priority: TaskPriority,
    pub revision: i64,
    pub block_reason: Option<String>,
    pub updated_at: String,
    pub required_count: i64,
    pub satisfied_required_count: i64,
    pub blockers: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct RecoveryTasks {
    pub in_progress: Vec<RecoveryTaskItem>,
    pub ready: Vec<RecoveryTaskItem>,
    pub blocked: Vec<RecoveryTaskItem>,
    pub awaiting_verification: Vec<RecoveryTaskItem>,
    pub done_with_valid_evidence: Vec<RecoveryTaskItem>,
    pub done_with_invalid_evidence: Vec<RecoveryTaskItem>,
    pub draft_count: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecoveryVerificationItem {
    pub task_id: String,
    pub task_title: String,
    pub criterion_id: String,
    pub criterion_description: String,
    pub status: VerificationResult,
    pub execution_status: Option<String>,
    pub run_id: Option<String>,
    pub evidence_id: Option<String>,
    pub started_at: Option<String>,
    pub finished_at: Option<String>,
    pub exit_code: Option<i32>,
    pub stale_reason: Option<String>,
    pub detail: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecoveryClaude {
    pub has_activity: bool,
    pub event_count: i64,
    pub session_count: i64,
    pub last_event_at: Option<String>,
    pub last_event_type: Option<String>,
    pub last_session_ref: Option<String>,
    pub last_session_started_at: Option<String>,
    pub last_session_last_event_at: Option<String>,
    pub last_session_ended_at: Option<String>,
    pub last_session_end_reason: Option<String>,
    pub last_session_failed_tool_count: i64,
    pub last_session_running_tool_count: i64,
    pub last_session_unknown_tool_count: i64,
    pub end_confirmed: bool,
    pub note: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecoveryMemoryItem {
    pub id: String,
    pub category: MemoryCategory,
    pub title: String,
    pub content: String,
    pub source_type: MemorySourceType,
    pub confirmation_status: MemoryConfirmationStatus,
    pub revision: i64,
    pub updated_at: String,
    pub linked_task_id: Option<String>,
    pub linked_task_title: Option<String>,
    pub linked_task_status: Option<TaskStatus>,
    pub linked_task_archived: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecoveryDecisionItem {
    pub id: String,
    pub title: String,
    pub status: DecisionStatus,
    pub task_id: Option<String>,
    pub task_title: Option<String>,
    pub problem: String,
    pub context: String,
    pub alternatives: Vec<String>,
    pub decision: String,
    pub rationale: String,
    pub risks: String,
    pub revision: i64,
    pub updated_at: String,
    pub confirmed_at: Option<String>,
    pub supersedes_title: Option<String>,
    pub superseded_by_title: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecoveryDecisionNotice {
    pub id: String,
    pub title: String,
    pub status: DecisionStatus,
    pub revision: i64,
    pub task_id: Option<String>,
    pub task_title: Option<String>,
    pub superseded_by_title: Option<String>,
    pub updated_at: String,
    pub note: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecoveryNextStep {
    pub kind: RecoveryNextStepKind,
    pub task_id: Option<String>,
    pub task_title: Option<String>,
    pub task_status: Option<TaskStatus>,
    // Drafts are counted, but their revisions are absent from the task lists.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub draft_task_revision: Option<i64>,
    pub reason: String,
    pub requires_confirmation: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecoveryConflict {
    pub kind: String,
    pub subject: String,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecoverySources {
    pub database_read_at: String,
    pub git_checked_at: Option<String>,
    pub task_candidate_count: i64,
    pub active_task_count: i64,
    pub done_task_count: i64,
    pub draft_task_count: i64,
    pub memory_confirmed_active_count: i64,
    pub memory_unconfirmed_active_count: i64,
    pub memory_archived_count: i64,
    pub decision_confirmed_count: i64,
    pub decision_draft_count: i64,
    pub decision_superseded_count: i64,
    pub decision_deprecated_count: i64,
    pub verification_run_count: i64,
    pub claude_event_count: i64,
    pub claude_session_count: i64,
    pub included_limits: Vec<String>,
    pub exclusions: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecoveryPackage {
    pub project: RecoveryProject,
    pub generated_at: String,
    pub fingerprint: String,
    pub git: RecoveryGit,
    pub tasks: RecoveryTasks,
    pub verification: Vec<RecoveryVerificationItem>,
    pub claude: RecoveryClaude,
    pub memories: Vec<RecoveryMemoryItem>,
    pub unconfirmed_memories: Vec<RecoveryMemoryItem>,
    pub decisions: Vec<RecoveryDecisionItem>,
    pub superseded_decisions: Vec<RecoveryDecisionNotice>,
    pub next_step: RecoveryNextStep,
    pub files_to_read: Vec<String>,
    pub conflicts: Vec<RecoveryConflict>,
    pub missing_information: Vec<String>,
    pub sources: RecoverySources,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecoverySnapshot {
    pub id: String,
    pub project_id: String,
    pub generated_at: String,
    pub fingerprint: String,
    pub markdown: String,
    pub package: RecoveryPackage,
    pub task_count: i64,
    pub memory_count: i64,
    pub decision_count: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecoverySnapshotSummary {
    pub id: String,
    pub project_id: String,
    pub generated_at: String,
    pub fingerprint: String,
    pub task_count: i64,
    pub memory_count: i64,
    pub decision_count: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecoveryFreshness {
    pub state: RecoveryFreshnessState,
    pub checked_at: String,
    pub changed: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecoveryOverview {
    pub latest: Option<RecoverySnapshot>,
    pub freshness: RecoveryFreshness,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecoveryPreference {
    pub project_id: String,
    pub next_task_id: Option<String>,
    pub updated_at: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetNextTaskInput {
    pub project_id: String,
    pub task_id: Option<String>,
}

pub struct RecoveryService<'a> {
    database: &'a Database,
}

impl<'a> RecoveryService<'a> {
    pub fn new(database: &'a Database) -> Self {
        Self { database }
    }

    pub fn generate(&self, project_id: &str) -> Result<RecoverySnapshot, CoreError> {
        let package = build_package(self.database, project_id)?;
        let snapshot = RecoverySnapshot {
            id: Uuid::new_v4().to_string(),
            project_id: package.project.id.clone(),
            generated_at: package.generated_at.clone(),
            fingerprint: package.fingerprint.clone(),
            markdown: build_markdown(&package),
            task_count: count_included_tasks(&package.tasks),
            memory_count: (package.memories.len() + package.unconfirmed_memories.len()) as i64,
            decision_count: (package.decisions.len() + package.superseded_decisions.len()) as i64,
            package,
        };
        persist_snapshot(self.database, &snapshot)?;
        Ok(snapshot)
    }

    pub fn overview(&self, project_id: &str) -> Result<RecoveryOverview, CoreError> {
        let project_id = validate_uuid(project_id, "项目 ID")?;
        {
            let connection = self.database.lock()?;
            require_project(&connection, &project_id)?;
        }

        let latest = load_latest_snapshot(self.database, &project_id)?;
        let checked_at = now_utc_rfc3339();
        let freshness = match &latest {
            None => RecoveryFreshness {
                state: RecoveryFreshnessState::NoSnapshot,
                checked_at,
                changed: Vec::new(),
            },
            Some(snapshot) => {
                let current = build_package(self.database, &project_id)?;
                if current.fingerprint == snapshot.fingerprint {
                    RecoveryFreshness {
                        state: RecoveryFreshnessState::Fresh,
                        checked_at,
                        changed: Vec::new(),
                    }
                } else {
                    RecoveryFreshness {
                        state: RecoveryFreshnessState::Stale,
                        checked_at,
                        changed: changed_areas(&snapshot.package, &current)?,
                    }
                }
            }
        };

        Ok(RecoveryOverview { latest, freshness })
    }

    pub fn list_snapshots(
        &self,
        project_id: &str,
    ) -> Result<Vec<RecoverySnapshotSummary>, CoreError> {
        let project_id = validate_uuid(project_id, "项目 ID")?;
        let connection = self.database.lock()?;
        require_project(&connection, &project_id)?;
        let mut statement = connection.prepare(
            "SELECT id, project_id, generated_at, fingerprint, task_count, memory_count,
                    decision_count
             FROM recovery_snapshots
             WHERE project_id = ?1
             ORDER BY generated_at DESC, rowid DESC
             LIMIT ?2",
        )?;
        let rows = statement.query_map(params![project_id, SNAPSHOT_RETENTION], |row| {
            Ok(RecoverySnapshotSummary {
                id: row.get(0)?,
                project_id: row.get(1)?,
                generated_at: row.get(2)?,
                fingerprint: row.get(3)?,
                task_count: row.get(4)?,
                memory_count: row.get(5)?,
                decision_count: row.get(6)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    pub fn get_snapshot(
        &self,
        project_id: &str,
        snapshot_id: &str,
    ) -> Result<RecoverySnapshot, CoreError> {
        let project_id = validate_uuid(project_id, "项目 ID")?;
        let snapshot_id = validate_uuid(snapshot_id, "恢复快照 ID")?;
        load_snapshot(self.database, &project_id, &snapshot_id)?.ok_or(CoreError::NotFound)
    }

    pub fn set_next_task(&self, input: SetNextTaskInput) -> Result<RecoveryPreference, CoreError> {
        let project_id = validate_uuid(&input.project_id, "项目 ID")?;
        let task_id = input
            .task_id
            .as_deref()
            .map(|value| validate_uuid(value, "任务 ID"))
            .transpose()?;

        self.database.with_transaction(|transaction| {
            let project = require_project(transaction, &project_id)?;
            if project.archived_at.is_some() {
                return Err(CoreError::Validation(
                    "项目已归档，不能设置下一步任务".to_owned(),
                ));
            }
            if let Some(task_id) = task_id.as_deref() {
                let task = TaskRepository::new(transaction)
                    .find_task(task_id)?
                    .ok_or(CoreError::NotFound)?;
                if task.project_id != project_id {
                    return Err(CoreError::Validation("下一步任务不属于当前项目".to_owned()));
                }
                if task.archived_at.is_some() {
                    return Err(CoreError::Validation("已归档任务不能作为下一步".to_owned()));
                }
                if matches!(task.status, TaskStatus::Done | TaskStatus::Cancelled) {
                    return Err(CoreError::Validation(
                        "已结束任务不能作为下一步；请选择仍在推进的任务".to_owned(),
                    ));
                }
            }

            let now = now_utc_rfc3339();
            transaction.execute(
                "INSERT INTO project_recovery_preferences (project_id, next_task_id, updated_at)
                 VALUES (?1, ?2, ?3)
                 ON CONFLICT(project_id) DO UPDATE SET
                    next_task_id = excluded.next_task_id,
                    updated_at = excluded.updated_at",
                params![project_id, task_id, now],
            )?;
            Ok(RecoveryPreference {
                project_id,
                next_task_id: task_id,
                updated_at: now,
            })
        })
    }
}

fn build_package(database: &Database, project_id: &str) -> Result<RecoveryPackage, CoreError> {
    let project_id = validate_uuid(project_id, "项目 ID")?;
    let project = {
        let connection = database.lock()?;
        require_project(&connection, &project_id)?
    };

    let code_state_result = code_state::capture(Path::new(&project.root_path));
    let git_checked_at = now_utc_rfc3339();
    let (git, current_code_state) = match code_state_result {
        Ok(state) => (git_from_code_state(&state, &git_checked_at), Some(state)),
        Err(error) => (
            RecoveryGit {
                state: RecoveryGitState::Unknown,
                checked_at: Some(git_checked_at.clone()),
                branch: None,
                head: None,
                dirty: false,
                detached: false,
                unborn: false,
                workspace_fingerprint: None,
                fingerprint_complete: false,
                changed_file_count: 0,
                note: Some(git_unknown_note(&error)),
            },
            None,
        ),
    };

    let database_read_at = now_utc_rfc3339();
    let connection = database.lock()?;
    let task_counts = load_task_counts(&connection, &project_id)?;
    let candidates =
        TaskRepository::new(&connection).recovery_candidates(&project_id, TASK_CANDIDATE_LIMIT)?;
    let (mut tasks, verification) =
        evaluate_tasks(&connection, &candidates, current_code_state.as_ref())?;
    tasks.draft_count = task_counts.draft;
    let (memory_confirmed_active, memory_unconfirmed_active, memory_archived) =
        count_memories(&connection, &project_id)?;
    let decision_counts = count_decisions(&connection, &project_id)?;
    let memories = load_memories(
        &connection,
        &project_id,
        MemoryConfirmationStatus::Confirmed,
        MEMORY_LIMIT,
    )?;
    let unconfirmed_memories = load_memories(
        &connection,
        &project_id,
        MemoryConfirmationStatus::Unconfirmed,
        MEMORY_LIMIT,
    )?;
    let decisions = load_decisions(&connection, &project_id, DECISION_LIMIT)?;
    let superseded_decisions = load_decision_notices(&connection, &project_id, NOTICE_LIMIT)?;
    let claude = load_claude(&connection, &project_id)?;
    let preference = load_preference(&connection, &project_id)?;
    let verification_run_count: i64 = connection.query_row(
        "SELECT COUNT(*)
         FROM verification_runs r
         JOIN tasks t ON t.id = r.task_id
         WHERE t.project_id = ?1",
        [&project_id],
        |row| row.get(0),
    )?;
    drop(connection);

    let files_to_read = discover_files(Path::new(&project.root_path));
    let (next_step, preference_conflict) = decide_next_step(
        database,
        &project_id,
        preference
            .as_ref()
            .and_then(|preference| preference.next_task_id.as_deref()),
        &tasks,
    )?;
    let mut conflicts = build_conflicts(&tasks, &memories, &unconfirmed_memories);
    if let Some(conflict) = preference_conflict {
        conflicts.push(conflict);
    }
    let missing_information = build_missing_information(
        &project,
        &git,
        &tasks,
        task_counts.total,
        &verification,
        &claude,
        &memories,
        &unconfirmed_memories,
        memory_archived,
        &decisions,
        decision_counts.draft,
        &files_to_read,
    );

    let sources = RecoverySources {
        database_read_at,
        git_checked_at: git.checked_at.clone(),
        task_candidate_count: task_counts.total,
        active_task_count: task_counts.draft
            + task_counts.ready
            + task_counts.running
            + task_counts.awaiting
            + task_counts.blocked,
        done_task_count: task_counts.done,
        draft_task_count: task_counts.draft,
        memory_confirmed_active_count: memory_confirmed_active,
        memory_unconfirmed_active_count: memory_unconfirmed_active,
        memory_archived_count: memory_archived,
        decision_confirmed_count: decision_counts.confirmed,
        decision_draft_count: decision_counts.draft,
        decision_superseded_count: decision_counts.superseded,
        decision_deprecated_count: decision_counts.deprecated,
        verification_run_count,
        claude_event_count: claude.event_count,
        claude_session_count: claude.session_count,
        included_limits: vec![
            format!(
                "任务：按最近更新最多检查 {TASK_CANDIDATE_LIMIT} 条；执行中/就绪/阻塞/待验证各最多 {ACTIVE_SECTION_LIMIT} 条；最近完成最多 {DONE_SECTION_LIMIT} 条。"
            ),
            format!("验证：最多 {VERIFICATION_LIMIT} 条最新记录。"),
            format!("记忆：已确认与未确认各最多 {MEMORY_LIMIT} 条。"),
            format!(
                "决策：已确认最多 {DECISION_LIMIT} 条；取代/废弃提示最多 {NOTICE_LIMIT} 条。"
            ),
            format!(
                "恢复包中的长文本最多保留 {MARKDOWN_CONTENT_MAX} 字符；完整内容请在项目上下文中查看。"
            ),
        ],
        exclusions: vec![
            "未导入的 Claude Code 本地收件箱事件不在恢复包内。".to_owned(),
            "不包含原始 Hook JSON、Prompt、工具输入输出、transcript 或密钥。".to_owned(),
            "不包含未记录在 XUSHU 中的项目文件内容。".to_owned(),
            "记忆与决策文本不代表真实验收结果。".to_owned(),
        ],
    };

    let mut package = RecoveryPackage {
        project: RecoveryProject {
            id: project.id,
            name: project.name,
            root_path: project.root_path,
            archived: project.archived_at.is_some(),
            created_at: project.created_at,
            updated_at: project.updated_at,
        },
        generated_at: now_utc_rfc3339(),
        fingerprint: String::new(),
        git,
        tasks,
        verification,
        claude,
        memories,
        unconfirmed_memories,
        decisions,
        superseded_decisions,
        next_step,
        files_to_read,
        conflicts,
        missing_information,
        sources,
    };
    package.fingerprint = fingerprint_package(&package)?;
    Ok(package)
}

fn git_from_code_state(state: &CodeState, checked_at: &str) -> RecoveryGit {
    RecoveryGit {
        state: RecoveryGitState::Available,
        checked_at: Some(checked_at.to_owned()),
        branch: state.branch.clone(),
        head: state.head.clone(),
        dirty: state.dirty,
        detached: state.detached,
        unborn: state.unborn,
        workspace_fingerprint: state.fingerprint.clone(),
        fingerprint_complete: state.fingerprint_complete,
        changed_file_count: state.changed_file_count,
        note: state.note.clone(),
    }
}

fn git_unknown_note(error: &CoreError) -> String {
    match error {
        CoreError::Validation(message) if message == "项目路径不存在或不可访问" => {
            "项目路径不存在或不可访问".to_owned()
        }
        CoreError::Validation(message) if message == "项目路径不是目录" => {
            "项目路径不是目录".to_owned()
        }
        CoreError::Git(_) => "无法读取 Git 状态；请确认项目路径与 Git 可用性".to_owned(),
        _ => "无法读取 Git 状态".to_owned(),
    }
}

fn require_project(connection: &Connection, project_id: &str) -> Result<Project, CoreError> {
    ProjectRepository::new(connection)
        .find_by_id(project_id)?
        .ok_or(CoreError::NotFound)
}

#[derive(Debug, Default)]
struct TaskCounts {
    total: i64,
    draft: i64,
    ready: i64,
    running: i64,
    awaiting: i64,
    blocked: i64,
    done: i64,
}

fn load_task_counts(connection: &Connection, project_id: &str) -> Result<TaskCounts, CoreError> {
    let mut counts = TaskCounts::default();
    let mut statement = connection.prepare(
        "SELECT status, COUNT(*)
         FROM tasks
         WHERE project_id = ?1 AND archived_at IS NULL
         GROUP BY status",
    )?;
    let rows = statement.query_map([project_id], |row| {
        Ok((row.get::<_, TaskStatus>(0)?, row.get::<_, i64>(1)?))
    })?;
    for row in rows {
        let (status, count) = row?;
        counts.total += count;
        match status {
            TaskStatus::Draft => counts.draft += count,
            TaskStatus::Ready => counts.ready += count,
            TaskStatus::Running => counts.running += count,
            TaskStatus::AwaitingVerification => counts.awaiting += count,
            TaskStatus::Blocked => counts.blocked += count,
            TaskStatus::Done => counts.done += count,
            TaskStatus::Cancelled => {}
        }
    }
    Ok(counts)
}

fn evaluate_tasks(
    connection: &Connection,
    candidates: &[Task],
    current_code_state: Option<&CodeState>,
) -> Result<(RecoveryTasks, Vec<RecoveryVerificationItem>), CoreError> {
    let mut tasks = RecoveryTasks::default();
    let mut verification = Vec::new();
    let mut done_count = 0usize;

    for task in candidates {
        match task.status {
            TaskStatus::Draft => {
                tasks.draft_count += 1;
                continue;
            }
            TaskStatus::Cancelled => continue,
            TaskStatus::Done => {
                if done_count >= DONE_SECTION_LIMIT {
                    continue;
                }
                done_count += 1;
            }
            TaskStatus::Running => {
                if tasks.in_progress.len() >= ACTIVE_SECTION_LIMIT {
                    continue;
                }
            }
            TaskStatus::Ready => {
                if tasks.ready.len() >= ACTIVE_SECTION_LIMIT {
                    continue;
                }
            }
            TaskStatus::Blocked => {
                if tasks.blocked.len() >= ACTIVE_SECTION_LIMIT {
                    continue;
                }
            }
            TaskStatus::AwaitingVerification => {
                if tasks.awaiting_verification.len() >= ACTIVE_SECTION_LIMIT {
                    continue;
                }
            }
        }

        let (views, completion) = criteria_with_completion(connection, task, current_code_state)?;
        let item = task_item(task, &completion);
        match task.status {
            TaskStatus::Running => tasks.in_progress.push(item),
            TaskStatus::Ready => tasks.ready.push(item),
            TaskStatus::Blocked => tasks.blocked.push(item),
            TaskStatus::AwaitingVerification => tasks.awaiting_verification.push(item),
            TaskStatus::Done if completion.can_complete => {
                tasks.done_with_valid_evidence.push(item)
            }
            TaskStatus::Done => tasks.done_with_invalid_evidence.push(item),
            _ => {}
        }

        for view in views {
            let Some(verification_view) = view.verification else {
                continue;
            };
            let Some(run) = verification_view.latest_run else {
                continue;
            };
            verification.push(RecoveryVerificationItem {
                task_id: task.id.clone(),
                task_title: task.title.clone(),
                criterion_id: view.criterion.id.clone(),
                criterion_description: view.criterion.description.clone(),
                status: verification_view.status,
                execution_status: Some(run.execution_status),
                run_id: Some(run.id),
                evidence_id: run.evidence_id,
                started_at: Some(run.started_at),
                finished_at: run.finished_at,
                exit_code: run.exit_code,
                stale_reason: verification_view.stale_reason,
                detail: verification_view.detail,
            });
        }
    }

    verification.sort_by(|left, right| {
        right
            .started_at
            .cmp(&left.started_at)
            .then_with(|| right.run_id.cmp(&left.run_id))
    });
    verification.truncate(VERIFICATION_LIMIT);
    Ok((tasks, verification))
}

fn task_item(task: &Task, completion: &CompletionStatus) -> RecoveryTaskItem {
    RecoveryTaskItem {
        id: task.id.clone(),
        title: task.title.clone(),
        status: task.status,
        priority: task.priority,
        revision: task.revision,
        block_reason: task.block_reason.clone(),
        updated_at: task.updated_at.clone(),
        required_count: completion.required_count,
        satisfied_required_count: completion.satisfied_required_count,
        blockers: completion.blockers.clone(),
    }
}

fn count_memories(connection: &Connection, project_id: &str) -> Result<(i64, i64, i64), CoreError> {
    let mut confirmed = 0;
    let mut unconfirmed = 0;
    let mut statement = connection.prepare(
        "SELECT confirmation_status, COUNT(*)
         FROM project_memories
         WHERE project_id = ?1 AND validity_status = 'active'
         GROUP BY confirmation_status",
    )?;
    let rows = statement.query_map([project_id], |row| {
        Ok((
            row.get::<_, MemoryConfirmationStatus>(0)?,
            row.get::<_, i64>(1)?,
        ))
    })?;
    for row in rows {
        let (status, count) = row?;
        match status {
            MemoryConfirmationStatus::Confirmed => confirmed = count,
            MemoryConfirmationStatus::Unconfirmed => unconfirmed = count,
        }
    }
    let archived: i64 = connection.query_row(
        "SELECT COUNT(*) FROM project_memories
         WHERE project_id = ?1 AND validity_status = 'archived'",
        [project_id],
        |row| row.get(0),
    )?;
    Ok((confirmed, unconfirmed, archived))
}

#[derive(Debug, Default)]
struct DecisionCounts {
    confirmed: i64,
    draft: i64,
    superseded: i64,
    deprecated: i64,
}

fn count_decisions(connection: &Connection, project_id: &str) -> Result<DecisionCounts, CoreError> {
    let mut counts = DecisionCounts::default();
    let mut statement = connection
        .prepare("SELECT status, COUNT(*) FROM decisions WHERE project_id = ?1 GROUP BY status")?;
    let rows = statement.query_map([project_id], |row| {
        Ok((row.get::<_, DecisionStatus>(0)?, row.get::<_, i64>(1)?))
    })?;
    for row in rows {
        let (status, count) = row?;
        match status {
            DecisionStatus::Confirmed => counts.confirmed = count,
            DecisionStatus::Draft => counts.draft = count,
            DecisionStatus::Superseded => counts.superseded = count,
            DecisionStatus::Deprecated => counts.deprecated = count,
        }
    }
    Ok(counts)
}

fn load_memories(
    connection: &Connection,
    project_id: &str,
    confirmation: MemoryConfirmationStatus,
    limit: i64,
) -> Result<Vec<RecoveryMemoryItem>, CoreError> {
    let mut statement = connection.prepare(
        "SELECT m.id, m.category, m.title, m.content, m.source_type,
                m.confirmation_status, m.revision, m.updated_at, m.linked_task_id,
                t.title, t.status, t.archived_at
         FROM project_memories m
         LEFT JOIN tasks t ON t.id = m.linked_task_id
         WHERE m.project_id = ?1
           AND m.validity_status = 'active'
           AND m.confirmation_status = ?2
         ORDER BY m.updated_at DESC, m.id DESC
         LIMIT ?3",
    )?;
    let rows = statement.query_map(params![project_id, confirmation, limit], |row| {
        let linked_task_archived: Option<String> = row.get(11)?;
        let content: String = row.get(3)?;
        Ok(RecoveryMemoryItem {
            id: row.get(0)?,
            category: row.get(1)?,
            title: row.get(2)?,
            content: bounded_value(content, MARKDOWN_CONTENT_MAX),
            source_type: row.get(4)?,
            confirmation_status: row.get(5)?,
            revision: row.get(6)?,
            updated_at: row.get(7)?,
            linked_task_id: row.get(8)?,
            linked_task_title: row.get(9)?,
            linked_task_status: row.get(10)?,
            linked_task_archived: linked_task_archived.is_some(),
        })
    })?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

fn load_decisions(
    connection: &Connection,
    project_id: &str,
    limit: i64,
) -> Result<Vec<RecoveryDecisionItem>, CoreError> {
    let mut statement = connection.prepare(
        "SELECT d.id, d.title, d.status, d.task_id, t.title, d.problem, d.context,
                d.alternatives_json, d.decision, d.rationale, d.risks, d.revision,
                d.updated_at, d.confirmed_at, old.title, new.title
         FROM decisions d
         LEFT JOIN tasks t ON t.id = d.task_id
         LEFT JOIN decisions old ON old.id = d.supersedes_id
         LEFT JOIN decisions new ON new.id = d.superseded_by_id
         WHERE d.project_id = ?1 AND d.status = 'confirmed'
         ORDER BY d.updated_at DESC, d.id DESC
         LIMIT ?2",
    )?;
    let rows = statement.query_map(params![project_id, limit], |row| {
        let alternatives_json: String = row.get(7)?;
        let problem: String = row.get(5)?;
        let context: String = row.get(6)?;
        let decision: String = row.get(8)?;
        let rationale: String = row.get(9)?;
        let risks: String = row.get(10)?;
        Ok(RecoveryDecisionItem {
            id: row.get(0)?,
            title: row.get(1)?,
            status: row.get(2)?,
            task_id: row.get(3)?,
            task_title: row.get(4)?,
            problem: bounded_value(problem, MARKDOWN_CONTENT_MAX),
            context: bounded_value(context, MARKDOWN_CONTENT_MAX),
            alternatives: parse_alternatives(&alternatives_json)?
                .into_iter()
                .map(|alternative| bounded_value(alternative, MARKDOWN_CONTENT_MAX))
                .collect(),
            decision: bounded_value(decision, MARKDOWN_CONTENT_MAX),
            rationale: bounded_value(rationale, MARKDOWN_CONTENT_MAX),
            risks: bounded_value(risks, MARKDOWN_CONTENT_MAX),
            revision: row.get(11)?,
            updated_at: row.get(12)?,
            confirmed_at: row.get(13)?,
            supersedes_title: row.get(14)?,
            superseded_by_title: row.get(15)?,
        })
    })?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

fn load_decision_notices(
    connection: &Connection,
    project_id: &str,
    limit: i64,
) -> Result<Vec<RecoveryDecisionNotice>, CoreError> {
    let mut statement = connection.prepare(
        "SELECT d.id, d.title, d.status, d.revision, d.task_id, t.title,
                new.title, d.updated_at
         FROM decisions d
         LEFT JOIN tasks t ON t.id = d.task_id
         LEFT JOIN decisions new ON new.id = d.superseded_by_id
         WHERE d.project_id = ?1 AND d.status IN ('superseded', 'deprecated')
         ORDER BY d.updated_at DESC, d.id DESC
         LIMIT ?2",
    )?;
    let rows = statement.query_map(params![project_id, limit], |row| {
        let status: DecisionStatus = row.get(2)?;
        let note = match status {
            DecisionStatus::Superseded => "已被新决策取代，不再作为当前约束。".to_owned(),
            DecisionStatus::Deprecated => "已废弃，不再作为当前约束。".to_owned(),
            _ => String::new(),
        };
        Ok(RecoveryDecisionNotice {
            id: row.get(0)?,
            title: row.get(1)?,
            status,
            revision: row.get(3)?,
            task_id: row.get(4)?,
            task_title: row.get(5)?,
            superseded_by_title: row.get(6)?,
            updated_at: row.get(7)?,
            note,
        })
    })?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

fn parse_alternatives(value: &str) -> rusqlite::Result<Vec<String>> {
    serde_json::from_str(value).map_err(|_| rusqlite::Error::InvalidQuery)
}

type SessionRow = (
    String,
    String,
    String,
    String,
    Option<String>,
    Option<String>,
);

fn load_claude(connection: &Connection, project_id: &str) -> Result<RecoveryClaude, CoreError> {
    let event_count: i64 = connection.query_row(
        "SELECT COUNT(*) FROM claude_events WHERE project_id = ?1",
        [project_id],
        |row| row.get(0),
    )?;
    let session_count: i64 = connection.query_row(
        "SELECT COUNT(*) FROM claude_sessions WHERE project_id = ?1",
        [project_id],
        |row| row.get(0),
    )?;
    let last_event: Option<(String, String)> = connection
        .query_row(
            "SELECT event_type, occurred_at
             FROM claude_events
             WHERE project_id = ?1
             ORDER BY occurred_at DESC, rowid DESC
             LIMIT 1",
            [project_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?;
    let last_session: Option<SessionRow> = connection
        .query_row(
            "SELECT id, session_hash, started_at, last_event_at, ended_at, end_reason
             FROM claude_sessions
             WHERE project_id = ?1
             ORDER BY last_event_at DESC, rowid DESC
             LIMIT 1",
            [project_id],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                ))
            },
        )
        .optional()?;

    let (
        last_session_ref,
        last_session_started_at,
        last_session_last_event_at,
        last_session_ended_at,
        last_session_end_reason,
        failed,
        running,
        unknown,
    ) = match &last_session {
        Some((id, hash, started_at, last_event_at, ended_at, end_reason)) => {
            let mut failed = 0;
            let mut running = 0;
            let mut unknown = 0;
            let mut statement = connection.prepare(
                "SELECT status, COUNT(*)
                 FROM claude_tool_calls
                 WHERE session_id = ?1
                 GROUP BY status",
            )?;
            let rows = statement.query_map([id], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
            })?;
            for row in rows {
                let (status, count) = row?;
                match status.as_str() {
                    "failed" => failed = count,
                    "running" => running = count,
                    "unknown" => unknown = count,
                    _ => {}
                }
            }
            (
                Some(hash.chars().take(12).collect::<String>()),
                Some(started_at.clone()),
                Some(last_event_at.clone()),
                ended_at.clone(),
                end_reason.clone(),
                failed,
                running,
                unknown,
            )
        }
        None => (None, None, None, None, None, 0, 0, 0),
    };

    let end_confirmed = last_session_ended_at.is_some();
    let note = if session_count == 0 {
        "尚无已导入的 Claude Code 事件；未见失败事件不代表成功。".to_owned()
    } else if !end_confirmed {
        "最近会话结束未确认；Hook 事件不能作为任务完成证据。".to_owned()
    } else {
        "最近会话已结束；Stop/SessionEnd 只表示活动结束，不是验收证据。".to_owned()
    };

    Ok(RecoveryClaude {
        has_activity: event_count > 0 || session_count > 0,
        event_count,
        session_count,
        last_event_at: last_event.as_ref().map(|event| event.1.clone()),
        last_event_type: last_event.map(|event| event.0),
        last_session_ref,
        last_session_started_at,
        last_session_last_event_at,
        last_session_ended_at,
        last_session_end_reason,
        last_session_failed_tool_count: failed,
        last_session_running_tool_count: running,
        last_session_unknown_tool_count: unknown,
        end_confirmed,
        note: Some(note),
    })
}

fn load_preference(
    connection: &Connection,
    project_id: &str,
) -> Result<Option<RecoveryPreference>, CoreError> {
    connection
        .query_row(
            "SELECT next_task_id, updated_at
             FROM project_recovery_preferences
             WHERE project_id = ?1",
            [project_id],
            |row| {
                Ok(RecoveryPreference {
                    project_id: project_id.to_owned(),
                    next_task_id: row.get(0)?,
                    updated_at: row.get(1)?,
                })
            },
        )
        .optional()
        .map_err(Into::into)
}

fn decide_next_step(
    database: &Database,
    project_id: &str,
    preferred_task_id: Option<&str>,
    tasks: &RecoveryTasks,
) -> Result<(RecoveryNextStep, Option<RecoveryConflict>), CoreError> {
    if let Some(task_id) = preferred_task_id {
        let task = {
            let connection = database.lock()?;
            TaskRepository::new(&connection).find_task(task_id)?
        };
        match task {
            Some(task)
                if task.project_id == project_id
                    && task.archived_at.is_none()
                    && !matches!(task.status, TaskStatus::Done | TaskStatus::Cancelled) =>
            {
                return Ok((
                    RecoveryNextStep {
                        kind: RecoveryNextStepKind::UserSelected,
                        task_id: Some(task.id),
                        task_title: Some(task.title),
                        task_status: Some(task.status),
                        draft_task_revision: (task.status == TaskStatus::Draft)
                            .then_some(task.revision),
                        reason: "你在 XUSHU 中明确指定的下一步任务。".to_owned(),
                        requires_confirmation: false,
                    },
                    None,
                ));
            }
            other => {
                let subject = other
                    .map(|task| task.title)
                    .unwrap_or_else(|| task_id.to_owned());
                return Ok((
                    RecoveryNextStep {
                        kind: RecoveryNextStepKind::NeedsConfirmation,
                        task_id: None,
                        task_title: None,
                        task_status: None,
                        draft_task_revision: None,
                        reason: "你指定的下一步任务已不可用；请重新指定。".to_owned(),
                        requires_confirmation: true,
                    },
                    Some(RecoveryConflict {
                        kind: "next_task_unavailable".to_owned(),
                        subject,
                        detail: "指定的下一步任务不存在、已归档或已结束。".to_owned(),
                    }),
                ));
            }
        }
    }

    if let Some(task) = tasks.blocked.first() {
        return Ok((
            suggested_next_step(
                task,
                task.block_reason
                    .clone()
                    .map(|reason| format!("任务已阻塞：{reason}"))
                    .unwrap_or_else(|| "任务已阻塞；请先补充原因并解除阻塞。".to_owned()),
            ),
            None,
        ));
    }
    if let Some(task) = tasks.awaiting_verification.first() {
        let reason = task
            .blockers
            .first()
            .cloned()
            .unwrap_or_else(|| "任务等待验收；请核对必需验收条件。".to_owned());
        return Ok((
            suggested_next_step(task, format!("任务等待验收：{reason}")),
            None,
        ));
    }
    if let Some(task) = tasks.in_progress.first() {
        return Ok((
            suggested_next_step(task, "任务正在执行；完成后进入待验证。".to_owned()),
            None,
        ));
    }
    if let Some(task) = tasks.ready.first() {
        return Ok((
            suggested_next_step(task, "任务已就绪，可以开始执行。".to_owned()),
            None,
        ));
    }

    let reason = if tasks.draft_count > 0 {
        format!(
            "存在 {} 个草稿任务，但尚无就绪、执行或待验收任务；请人工确认下一步。",
            tasks.draft_count
        )
    } else {
        "没有足够依据推断下一步；请人工确认或创建任务。".to_owned()
    };
    Ok((
        RecoveryNextStep {
            kind: RecoveryNextStepKind::NeedsConfirmation,
            task_id: None,
            task_title: None,
            task_status: None,
            draft_task_revision: None,
            reason,
            requires_confirmation: true,
        },
        None,
    ))
}

fn suggested_next_step(task: &RecoveryTaskItem, reason: String) -> RecoveryNextStep {
    RecoveryNextStep {
        kind: RecoveryNextStepKind::Suggested,
        task_id: Some(task.id.clone()),
        task_title: Some(task.title.clone()),
        task_status: Some(task.status),
        draft_task_revision: None,
        reason,
        requires_confirmation: true,
    }
}

fn discover_files(root: &Path) -> Vec<String> {
    if !root.is_dir() {
        return Vec::new();
    }

    let mut files = Vec::new();
    for relative in [
        "CLAUDE.md",
        "AGENTS.md",
        "README.md",
        "MASTER_DEVELOPMENT_PLAN.md",
        "docs/architecture/DEPENDENCIES.md",
        "docs/design/DESIGN_SYSTEM.md",
    ] {
        if root.join(relative).is_file() {
            files.push(relative.to_owned());
        }
    }

    let phases = root.join("docs").join("phases");
    if let Ok(entries) = fs::read_dir(&phases) {
        let mut reports = entries
            .flatten()
            .filter_map(|entry| {
                let path = entry.path();
                if path.extension().and_then(|value| value.to_str()) != Some("md") {
                    return None;
                }
                let metadata = entry.metadata().ok()?;
                if !metadata.is_file() {
                    return None;
                }
                let modified = metadata.modified().unwrap_or(std::time::UNIX_EPOCH);
                Some((
                    modified,
                    format!("docs/phases/{}", entry.file_name().to_string_lossy()),
                ))
            })
            .collect::<Vec<_>>();
        reports.sort_by_key(|left| std::cmp::Reverse(left.0));
        for (_, path) in reports.into_iter().take(2) {
            if !files.contains(&path) {
                files.push(path);
            }
        }
    }

    files.truncate(FILES_TO_READ_LIMIT);
    files
}

fn build_conflicts(
    tasks: &RecoveryTasks,
    memories: &[RecoveryMemoryItem],
    unconfirmed_memories: &[RecoveryMemoryItem],
) -> Vec<RecoveryConflict> {
    let mut conflicts = Vec::new();
    for task in &tasks.done_with_invalid_evidence {
        let detail = if task.blockers.is_empty() {
            "任务已完成，但当前没有可验证的有效证据。".to_owned()
        } else {
            task.blockers.join("；")
        };
        conflicts.push(RecoveryConflict {
            kind: "done_evidence_invalid".to_owned(),
            subject: task.title.clone(),
            detail,
        });
    }

    for memory in memories.iter().chain(unconfirmed_memories.iter()) {
        let inactive_task =
            memory.linked_task_archived || memory.linked_task_status == Some(TaskStatus::Cancelled);
        if inactive_task {
            conflicts.push(RecoveryConflict {
                kind: "memory_linked_task_inactive".to_owned(),
                subject: memory.title.clone(),
                detail: "记忆关联的任务已归档或取消；请人工复核该记忆是否仍然有效。".to_owned(),
            });
        }
    }
    conflicts
}

#[allow(clippy::too_many_arguments)]
fn build_missing_information(
    project: &Project,
    git: &RecoveryGit,
    tasks: &RecoveryTasks,
    total_tasks: i64,
    verification: &[RecoveryVerificationItem],
    claude: &RecoveryClaude,
    memories: &[RecoveryMemoryItem],
    unconfirmed_memories: &[RecoveryMemoryItem],
    archived_memories: i64,
    decisions: &[RecoveryDecisionItem],
    draft_decisions: i64,
    files_to_read: &[String],
) -> Vec<String> {
    let mut missing = Vec::new();
    if git.state == RecoveryGitState::Unknown {
        missing.push(format!(
            "无法读取 Git 状态：{}",
            git.note
                .as_deref()
                .unwrap_or("UNKNOWN；不得使用旧数据冒充实时状态")
        ));
    }
    if total_tasks == 0 {
        missing.push("尚无任务记录，无法推断开发进度。".to_owned());
    }
    if verification.is_empty() {
        missing.push("尚无独立验证运行记录；任何完成声明都需要证据。".to_owned());
    }
    if claude.event_count == 0 {
        missing.push("尚无已导入的 Claude Code 活动元数据；缺失事件不能视为成功。".to_owned());
    } else if !claude.end_confirmed {
        missing.push("最近 Claude Code 会话结束未确认；不要视为完成。".to_owned());
    }
    if memories.is_empty() {
        missing.push("尚无已确认的项目记忆。".to_owned());
    }
    if !unconfirmed_memories.is_empty() {
        missing.push(format!(
            "有 {} 条记忆尚未确认，未作为权威上下文。",
            unconfirmed_memories.len()
        ));
    }
    if archived_memories > 0 {
        missing.push(format!("有 {archived_memories} 条已归档记忆未进入恢复包。"));
    }
    if decisions.is_empty() {
        missing.push("尚无已确认的开发决策。".to_owned());
    }
    if draft_decisions > 0 {
        missing.push(format!(
            "有 {draft_decisions} 条草稿决策未确认，未进入恢复包。"
        ));
    }
    if files_to_read.is_empty() {
        missing.push("未在项目根目录发现常见协作文件。".to_owned());
    }
    if project.archived_at.is_some() {
        missing.push("项目已归档；恢复包只读，不能新增记忆或决策。".to_owned());
    }
    if tasks.draft_count > 0 && tasks.ready.is_empty() && tasks.in_progress.is_empty() {
        missing.push(format!(
            "有 {} 个草稿任务尚未进入就绪或执行。",
            tasks.draft_count
        ));
    }
    missing
}

fn count_included_tasks(tasks: &RecoveryTasks) -> i64 {
    (tasks.in_progress.len()
        + tasks.ready.len()
        + tasks.blocked.len()
        + tasks.awaiting_verification.len()
        + tasks.done_with_valid_evidence.len()
        + tasks.done_with_invalid_evidence.len()) as i64
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
struct FingerprintTask {
    id: String,
    status: TaskStatus,
    revision: i64,
    block_reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
struct FingerprintMemory {
    id: String,
    revision: i64,
    confirmation_status: MemoryConfirmationStatus,
    validity_status: String,
    linked_task_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
struct FingerprintDecision {
    id: String,
    status: DecisionStatus,
    revision: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
struct FingerprintVerification {
    task_id: String,
    criterion_id: String,
    run_id: Option<String>,
    status: VerificationResult,
    execution_status: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
struct FingerprintState {
    project_id: String,
    project_updated_at: String,
    project_archived: bool,
    git_state: RecoveryGitState,
    git_branch: Option<String>,
    git_head: Option<String>,
    git_dirty: bool,
    git_detached: bool,
    git_unborn: bool,
    workspace_fingerprint: Option<String>,
    tasks: Vec<FingerprintTask>,
    memories: Vec<FingerprintMemory>,
    decisions: Vec<FingerprintDecision>,
    verification: Vec<FingerprintVerification>,
    task_candidate_count: i64,
    active_task_count: i64,
    done_task_count: i64,
    draft_task_count: i64,
    memory_confirmed_active_count: i64,
    memory_unconfirmed_active_count: i64,
    memory_archived_count: i64,
    decision_confirmed_count: i64,
    decision_draft_count: i64,
    decision_superseded_count: i64,
    decision_deprecated_count: i64,
    verification_run_count: i64,
    claude_event_count: i64,
    claude_session_count: i64,
    claude_last_event_at: Option<String>,
    claude_last_event_type: Option<String>,
    claude_last_session_ref: Option<String>,
    claude_end_confirmed: bool,
    claude_failed_tool_count: i64,
    claude_running_tool_count: i64,
    claude_unknown_tool_count: i64,
    next_step_kind: RecoveryNextStepKind,
    next_step_task_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    next_step_draft_revision: Option<i64>,
    files_to_read: Vec<String>,
}

fn fingerprint_state(package: &RecoveryPackage) -> FingerprintState {
    let mut tasks = Vec::new();
    for task in all_task_items(&package.tasks) {
        tasks.push(FingerprintTask {
            id: task.id.clone(),
            status: task.status,
            revision: task.revision,
            block_reason: task.block_reason.clone(),
        });
    }
    tasks.sort_by(|left, right| left.id.cmp(&right.id));

    let mut memories = package
        .memories
        .iter()
        .chain(package.unconfirmed_memories.iter())
        .map(|memory| FingerprintMemory {
            id: memory.id.clone(),
            revision: memory.revision,
            confirmation_status: memory.confirmation_status,
            validity_status: "active".to_owned(),
            linked_task_id: memory.linked_task_id.clone(),
        })
        .collect::<Vec<_>>();
    memories.sort_by(|left, right| left.id.cmp(&right.id));

    let mut decisions = package
        .decisions
        .iter()
        .map(|decision| FingerprintDecision {
            id: decision.id.clone(),
            status: decision.status,
            revision: decision.revision,
        })
        .chain(
            package
                .superseded_decisions
                .iter()
                .map(|notice| FingerprintDecision {
                    id: notice.id.clone(),
                    status: notice.status,
                    revision: notice.revision,
                }),
        )
        .collect::<Vec<_>>();
    decisions.sort_by(|left, right| left.id.cmp(&right.id));

    let mut verification = package
        .verification
        .iter()
        .map(|item| FingerprintVerification {
            task_id: item.task_id.clone(),
            criterion_id: item.criterion_id.clone(),
            run_id: item.run_id.clone(),
            status: item.status,
            execution_status: item.execution_status.clone(),
        })
        .collect::<Vec<_>>();
    verification.sort_by(|left, right| {
        left.task_id
            .cmp(&right.task_id)
            .then_with(|| left.criterion_id.cmp(&right.criterion_id))
            .then_with(|| left.run_id.cmp(&right.run_id))
    });

    let mut files = package.files_to_read.clone();
    files.sort();

    FingerprintState {
        project_id: package.project.id.clone(),
        project_updated_at: package.project.updated_at.clone(),
        project_archived: package.project.archived,
        git_state: package.git.state,
        git_branch: package.git.branch.clone(),
        git_head: package.git.head.clone(),
        git_dirty: package.git.dirty,
        git_detached: package.git.detached,
        git_unborn: package.git.unborn,
        workspace_fingerprint: package.git.workspace_fingerprint.clone(),
        tasks,
        memories,
        decisions,
        verification,
        task_candidate_count: package.sources.task_candidate_count,
        active_task_count: package.sources.active_task_count,
        done_task_count: package.sources.done_task_count,
        draft_task_count: package.sources.draft_task_count,
        memory_confirmed_active_count: package.sources.memory_confirmed_active_count,
        memory_unconfirmed_active_count: package.sources.memory_unconfirmed_active_count,
        memory_archived_count: package.sources.memory_archived_count,
        decision_confirmed_count: package.sources.decision_confirmed_count,
        decision_draft_count: package.sources.decision_draft_count,
        decision_superseded_count: package.sources.decision_superseded_count,
        decision_deprecated_count: package.sources.decision_deprecated_count,
        verification_run_count: package.sources.verification_run_count,
        claude_event_count: package.claude.event_count,
        claude_session_count: package.claude.session_count,
        claude_last_event_at: package.claude.last_event_at.clone(),
        claude_last_event_type: package.claude.last_event_type.clone(),
        claude_last_session_ref: package.claude.last_session_ref.clone(),
        claude_end_confirmed: package.claude.end_confirmed,
        claude_failed_tool_count: package.claude.last_session_failed_tool_count,
        claude_running_tool_count: package.claude.last_session_running_tool_count,
        claude_unknown_tool_count: package.claude.last_session_unknown_tool_count,
        next_step_kind: package.next_step.kind,
        next_step_task_id: package.next_step.task_id.clone(),
        next_step_draft_revision: package.next_step.draft_task_revision,
        files_to_read: files,
    }
}

fn fingerprint_package(package: &RecoveryPackage) -> Result<String, CoreError> {
    let state = fingerprint_state(package);
    let bytes = serde_json::to_vec(&state)
        .map_err(|error| CoreError::Recovery(format!("无法编码恢复指纹: {error}")))?;
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    Ok(format!("sha256:{}", hex(&hasher.finalize())))
}

fn changed_areas(old: &RecoveryPackage, new: &RecoveryPackage) -> Result<Vec<String>, CoreError> {
    let old_state = fingerprint_state(old);
    let new_state = fingerprint_state(new);
    let mut changed = Vec::new();

    if old_state.project_updated_at != new_state.project_updated_at
        || old_state.project_archived != new_state.project_archived
    {
        changed.push("项目信息已变化".to_owned());
    }
    if old_state.git_state != new_state.git_state
        || old_state.git_branch != new_state.git_branch
        || old_state.git_head != new_state.git_head
        || old_state.git_dirty != new_state.git_dirty
        || old_state.git_detached != new_state.git_detached
        || old_state.git_unborn != new_state.git_unborn
        || old_state.workspace_fingerprint != new_state.workspace_fingerprint
    {
        changed.push("Git 状态或工作区指纹已变化".to_owned());
    }
    if old_state.tasks != new_state.tasks {
        changed.push("任务状态或修订已变化".to_owned());
    }
    if old_state.memories != new_state.memories {
        changed.push("记忆已更新".to_owned());
    }
    if old_state.decisions != new_state.decisions {
        changed.push("决策已更新".to_owned());
    }
    if old_state.verification != new_state.verification {
        changed.push("验证结果已变化".to_owned());
    }
    if old_state.claude_last_event_at != new_state.claude_last_event_at
        || old_state.claude_last_event_type != new_state.claude_last_event_type
        || old_state.claude_last_session_ref != new_state.claude_last_session_ref
        || old_state.claude_end_confirmed != new_state.claude_end_confirmed
        || old_state.claude_event_count != new_state.claude_event_count
        || old_state.claude_session_count != new_state.claude_session_count
        || old_state.claude_failed_tool_count != new_state.claude_failed_tool_count
        || old_state.claude_running_tool_count != new_state.claude_running_tool_count
        || old_state.claude_unknown_tool_count != new_state.claude_unknown_tool_count
    {
        changed.push("Claude Code 活动已变化".to_owned());
    }
    if old_state.next_step_kind != new_state.next_step_kind
        || old_state.next_step_task_id != new_state.next_step_task_id
        || old_state.next_step_draft_revision != new_state.next_step_draft_revision
    {
        changed.push("下一步任务已变化".to_owned());
    }
    if old_state.files_to_read != new_state.files_to_read {
        changed.push("建议读取的项目文件已变化".to_owned());
    }
    if changed.is_empty() && old.fingerprint != new.fingerprint {
        changed.push("数据指纹已变化".to_owned());
    }
    Ok(changed)
}

fn all_task_items(tasks: &RecoveryTasks) -> impl Iterator<Item = &RecoveryTaskItem> {
    tasks
        .in_progress
        .iter()
        .chain(tasks.ready.iter())
        .chain(tasks.blocked.iter())
        .chain(tasks.awaiting_verification.iter())
        .chain(tasks.done_with_valid_evidence.iter())
        .chain(tasks.done_with_invalid_evidence.iter())
}

fn persist_snapshot(database: &Database, snapshot: &RecoverySnapshot) -> Result<(), CoreError> {
    if snapshot.markdown.len() > MAX_SNAPSHOT_MARKDOWN_BYTES {
        return Err(CoreError::Recovery(
            "恢复包正文超过本地大小上限；请归档部分记忆或决策后重试".to_owned(),
        ));
    }
    let package_json = serde_json::to_string(&snapshot.package)
        .map_err(|error| CoreError::Recovery(format!("无法编码恢复快照: {error}")))?;
    if package_json.len() > MAX_SNAPSHOT_PACKAGE_BYTES {
        return Err(CoreError::Recovery(
            "恢复包结构化数据超过本地大小上限；请归档部分记忆或决策后重试".to_owned(),
        ));
    }
    let created_at = now_utc_rfc3339();
    database.with_transaction(|transaction| {
        transaction.execute(
            "INSERT INTO recovery_snapshots (
                id, project_id, generated_at, fingerprint, markdown, package_json,
                task_count, memory_count, decision_count, created_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                snapshot.id,
                snapshot.project_id,
                snapshot.generated_at,
                snapshot.fingerprint,
                snapshot.markdown,
                package_json,
                snapshot.task_count,
                snapshot.memory_count,
                snapshot.decision_count,
                created_at
            ],
        )?;
        transaction.execute(
            "DELETE FROM recovery_snapshots
             WHERE project_id = ?1
               AND id NOT IN (
                   SELECT id FROM recovery_snapshots
                   WHERE project_id = ?1
                   ORDER BY generated_at DESC, rowid DESC
                   LIMIT ?2
               )",
            params![snapshot.project_id, SNAPSHOT_RETENTION],
        )?;
        Ok(())
    })
}

type SnapshotRaw = (
    String,
    String,
    String,
    String,
    String,
    String,
    i64,
    i64,
    i64,
);

fn load_latest_snapshot(
    database: &Database,
    project_id: &str,
) -> Result<Option<RecoverySnapshot>, CoreError> {
    let connection = database.lock()?;
    let raw: Option<SnapshotRaw> = connection
        .query_row(
            "SELECT id, project_id, generated_at, fingerprint, markdown, package_json,
                    task_count, memory_count, decision_count
             FROM recovery_snapshots
             WHERE project_id = ?1
             ORDER BY generated_at DESC, rowid DESC
             LIMIT 1",
            [project_id],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                    row.get(6)?,
                    row.get(7)?,
                    row.get(8)?,
                ))
            },
        )
        .optional()?;
    raw.map(snapshot_from_row).transpose()
}

fn load_snapshot(
    database: &Database,
    project_id: &str,
    snapshot_id: &str,
) -> Result<Option<RecoverySnapshot>, CoreError> {
    let connection = database.lock()?;
    let raw: Option<SnapshotRaw> = connection
        .query_row(
            "SELECT id, project_id, generated_at, fingerprint, markdown, package_json,
                    task_count, memory_count, decision_count
             FROM recovery_snapshots
             WHERE id = ?1 AND project_id = ?2",
            params![snapshot_id, project_id],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                    row.get(6)?,
                    row.get(7)?,
                    row.get(8)?,
                ))
            },
        )
        .optional()?;
    raw.map(snapshot_from_row).transpose()
}

fn snapshot_from_row(row: SnapshotRaw) -> Result<RecoverySnapshot, CoreError> {
    let (
        id,
        project_id,
        generated_at,
        fingerprint,
        markdown,
        package_json,
        task_count,
        memory_count,
        decision_count,
    ) = row;
    let package: RecoveryPackage = serde_json::from_str(&package_json)
        .map_err(|error| CoreError::Recovery(format!("恢复快照内容无效: {error}")))?;
    if package.fingerprint != fingerprint {
        return Err(CoreError::Recovery(
            "恢复快照指纹与内容不一致，请重新生成".to_owned(),
        ));
    }
    Ok(RecoverySnapshot {
        id,
        project_id,
        generated_at,
        fingerprint,
        markdown,
        package,
        task_count,
        memory_count,
        decision_count,
    })
}

fn build_markdown(package: &RecoveryPackage) -> String {
    let mut output = String::new();
    output.push_str(&format!(
        "# XUSHU 恢复上下文 — {}\n\n",
        package.project.name
    ));
    output.push_str(&format!("> 生成时间（UTC）：{}\n", package.generated_at));
    output.push_str(&format!("> 项目：{}\n", package.project.name));
    output.push_str(&format!(
        "> 项目路径：{}\n",
        code_span(&package.project.root_path)
    ));
    output.push_str(&format!(
        "> 项目状态：{}\n",
        if package.project.archived {
            "已归档（只读）"
        } else {
            "进行中"
        }
    ));
    output.push_str(&format!(
        "> 来源指纹：{}\n",
        code_span(&package.fingerprint)
    ));
    output
        .push_str("> 本恢复包由本地数据库、只读 Git 与独立验收结果确定性生成，不包含 AI 推断。\n");
    output.push_str(
        "> 记忆与决策不能替代任务状态、验收证据或完成门禁。分享前请检查本地路径与项目记忆内容。\n\n",
    );

    output.push_str("## 项目身份与开发约束\n\n");
    output.push_str(&format!("- 项目 ID：{}\n", code_span(&package.project.id)));
    output.push_str(&format!(
        "- 创建时间（UTC）：{}\n",
        package.project.created_at
    ));
    output.push_str(&format!(
        "- 更新时间（UTC）：{}\n",
        package.project.updated_at
    ));
    if package.memories.is_empty() {
        output.push_str("- 已确认记忆：无\n");
    } else {
        output.push_str(&format!(
            "- 已确认记忆：{} 条（见下方“已确认的项目记忆”）\n",
            package.memories.len()
        ));
    }
    output.push('\n');

    output.push_str("## Git 状态\n\n");
    match package.git.state {
        RecoveryGitState::Available => {
            output.push_str(&format!(
                "- 状态：可用（检查于 {}）\n",
                package.git.checked_at.as_deref().unwrap_or("UNKNOWN")
            ));
            output.push_str(&format!(
                "- 分支：{}\n",
                package.git.branch.as_deref().unwrap_or("UNKNOWN")
            ));
            output.push_str(&format!(
                "- HEAD：{}\n",
                package
                    .git
                    .head
                    .as_deref()
                    .map(code_span)
                    .unwrap_or_else(|| "UNKNOWN".to_owned())
            ));
            output.push_str(&format!(
                "- 工作区：{}\n",
                if package.git.dirty {
                    "存在未提交修改"
                } else {
                    "干净"
                }
            ));
            output.push_str(&format!(
                "- 工作区代码指纹：{}（{}）\n",
                package
                    .git
                    .workspace_fingerprint
                    .as_deref()
                    .map(code_span)
                    .unwrap_or_else(|| "UNKNOWN".to_owned()),
                if package.git.fingerprint_complete {
                    "完整"
                } else {
                    "不完整"
                }
            ));
            output.push_str(&format!(
                "- 变更文件数：{}\n",
                package.git.changed_file_count
            ));
            if let Some(note) = &package.git.note {
                output.push_str(&format!("- 说明：{note}\n"));
            }
        }
        RecoveryGitState::Unknown => {
            output.push_str(&format!(
                "- 状态：UNKNOWN（尝试检查于 {}）\n",
                package.git.checked_at.as_deref().unwrap_or("UNKNOWN")
            ));
            output.push_str(&format!(
                "- 说明：{}\n",
                package
                    .git
                    .note
                    .as_deref()
                    .unwrap_or("无法读取 Git 状态；不得使用旧数据冒充实时状态")
            ));
        }
    }
    output.push('\n');

    output.push_str("## 任务状态\n\n");
    push_task_section(&mut output, "执行中", &package.tasks.in_progress);
    push_task_section(&mut output, "已就绪", &package.tasks.ready);
    push_task_section(&mut output, "已阻塞", &package.tasks.blocked);
    push_task_section(&mut output, "待验证", &package.tasks.awaiting_verification);
    push_task_section(
        &mut output,
        "最近完成且证据有效",
        &package.tasks.done_with_valid_evidence,
    );
    push_task_section(
        &mut output,
        "最近完成但证据无效或已过期",
        &package.tasks.done_with_invalid_evidence,
    );
    if package.tasks.draft_count > 0 {
        output.push_str(&format!(
            "- 另有 {} 个草稿任务未列入恢复包。\n",
            package.tasks.draft_count
        ));
    }
    output.push('\n');

    output.push_str("## 独立验证结果\n\n");
    output.push_str("以下状态按生成时的任务修订、验证配置与代码状态计算；生成后可能已经过期。\n\n");
    if package.verification.is_empty() {
        output.push_str("尚无独立验证运行记录。\n\n");
    } else {
        for item in &package.verification {
            output.push_str(&format!(
                "- 任务「{}」· 条件「{}」：生成时状态 {}\n",
                item.task_title,
                item.criterion_description,
                verification_label(item.status)
            ));
            if let Some(execution) = &item.execution_status {
                output.push_str(&format!("  - 执行状态：{execution}\n"));
            }
            if let Some(started_at) = &item.started_at {
                output.push_str(&format!("  - 开始时间（UTC）：{started_at}\n"));
            }
            if let Some(exit_code) = item.exit_code {
                output.push_str(&format!("  - 退出码：{exit_code}\n"));
            }
            if let Some(evidence_id) = &item.evidence_id {
                output.push_str(&format!("  - 证据 ID：{}\n", code_span(evidence_id)));
            }
            if let Some(reason) = item.stale_reason.as_ref().or(item.detail.as_ref()) {
                output.push_str(&format!("  - 说明：{reason}\n"));
            }
        }
        output.push('\n');
    }

    output.push_str("## Claude Code 活动\n\n");
    output.push_str(
        "仅包含已导入的元数据；不包含 Prompt、命令、文件内容、工具输出或 transcript。\n\n",
    );
    output.push_str(&format!(
        "- 事件数：{}；会话数：{}\n",
        package.claude.event_count, package.claude.session_count
    ));
    if let Some(ref session_ref) = package.claude.last_session_ref {
        output.push_str(&format!("- 最近会话：{}\n", code_span(session_ref)));
        if let Some(started_at) = &package.claude.last_session_started_at {
            output.push_str(&format!("  - 开始时间（UTC）：{started_at}\n"));
        }
        if let Some(last_event_at) = &package.claude.last_session_last_event_at {
            output.push_str(&format!("  - 最近事件（UTC）：{last_event_at}\n"));
        }
        output.push_str(&format!(
            "  - 结束确认：{}\n",
            if package.claude.end_confirmed {
                "已确认"
            } else {
                "结束未确认"
            }
        ));
        if let Some(end_reason) = &package.claude.last_session_end_reason {
            output.push_str(&format!("  - 结束原因：{end_reason}\n"));
        }
        output.push_str(&format!(
            "  - 工具调用：失败 {}；未结束 {}；状态未知 {}\n",
            package.claude.last_session_failed_tool_count,
            package.claude.last_session_running_tool_count,
            package.claude.last_session_unknown_tool_count
        ));
    } else {
        output.push_str("- 最近会话：无已导入会话\n");
    }
    if let Some(last_event_at) = &package.claude.last_event_at {
        output.push_str(&format!(
            "- 最近事件类型：{}（{}）\n",
            package
                .claude
                .last_event_type
                .as_deref()
                .unwrap_or("UNKNOWN"),
            last_event_at
        ));
    }
    if let Some(note) = &package.claude.note {
        output.push_str(&format!("- 说明：{note}\n"));
    }
    output.push('\n');

    output.push_str("## 已确认的项目记忆\n\n");
    output.push_str(
        "这些内容由用户确认或人工创建，可用于理解开发上下文；不得覆盖真实任务与验收状态。\n\n",
    );
    if package.memories.is_empty() {
        output.push_str("尚无已确认的项目记忆。\n\n");
    } else {
        for category in MemoryCategory::ALL {
            let group = package
                .memories
                .iter()
                .filter(|memory| memory.category == category)
                .collect::<Vec<_>>();
            if group.is_empty() {
                continue;
            }
            output.push_str(&format!("### {}\n\n", memory_category_label(category)));
            for memory in group {
                push_memory_markdown(&mut output, memory);
            }
        }
    }

    if !package.unconfirmed_memories.is_empty() {
        output.push_str("## 未确认记忆（不是事实）\n\n");
        output.push_str("以下内容尚未经过人工确认，不得作为权威上下文或完成依据。\n\n");
        for memory in &package.unconfirmed_memories {
            push_memory_markdown(&mut output, memory);
        }
    }

    output.push_str("## 开发决策\n\n");
    if package.decisions.is_empty() {
        output.push_str("尚无已确认的开发决策。\n\n");
    } else {
        for decision in &package.decisions {
            output.push_str(&format!("### {}\n\n", decision.title));
            output.push_str(&format!(
                "- 状态：{}；修订 {}；更新时间（UTC）：{}\n",
                decision_status_label(decision.status),
                decision.revision,
                decision.updated_at
            ));
            if let Some(task_title) = &decision.task_title {
                output.push_str(&format!("- 关联任务：{task_title}\n"));
            }
            if let Some(supersedes_title) = &decision.supersedes_title {
                output.push_str(&format!("- 取代了：{supersedes_title}\n"));
            }
            if let Some(superseded_by_title) = &decision.superseded_by_title {
                output.push_str(&format!("- 被取代：{superseded_by_title}\n"));
            }
            output.push_str("\n**问题**\n\n");
            push_quote(&mut output, &decision.problem);
            output.push_str("\n**背景与约束**\n\n");
            push_quote(&mut output, &decision.context);
            if !decision.alternatives.is_empty() {
                output.push_str("\n**备选方案**\n\n");
                for alternative in &decision.alternatives {
                    output.push_str(&format!(
                        "- {}\n",
                        truncate_text(alternative, MARKDOWN_CONTENT_MAX)
                    ));
                }
            }
            output.push_str("\n**最终方案**\n\n");
            push_quote(&mut output, &decision.decision);
            output.push_str("\n**采用原因**\n\n");
            push_quote(&mut output, &decision.rationale);
            output.push_str("\n**已知代价与风险**\n\n");
            push_quote(&mut output, &decision.risks);
            output.push('\n');
        }
    }

    if !package.superseded_decisions.is_empty() {
        output.push_str("## 已被取代或废弃的决策\n\n");
        for notice in &package.superseded_decisions {
            output.push_str(&format!(
                "- {}（{}）：{}",
                notice.title,
                decision_status_label(notice.status),
                notice.note
            ));
            if let Some(superseded_by) = &notice.superseded_by_title {
                output.push_str(&format!(" 新决策：{superseded_by}。"));
            }
            output.push('\n');
        }
        output.push('\n');
    }

    output.push_str("## 下一步\n\n");
    output.push_str(&format!(
        "- 来源：{}\n",
        next_step_kind_label(package.next_step.kind)
    ));
    if let Some(task_title) = &package.next_step.task_title {
        output.push_str(&format!(
            "- 任务：{}（{}）\n",
            task_title,
            package
                .next_step
                .task_status
                .map(task_status_label)
                .unwrap_or("UNKNOWN")
        ));
    }
    output.push_str(&format!("- 依据：{}\n", package.next_step.reason));
    output.push_str(&format!(
        "- 需要人工确认：{}\n\n",
        if package.next_step.requires_confirmation {
            "是"
        } else {
            "否"
        }
    ));

    output.push_str("## 新会话应首先读取的项目文件\n\n");
    if package.files_to_read.is_empty() {
        output.push_str("未在项目根目录发现常见协作文件；请手动指定。\n");
    } else {
        for file in &package.files_to_read {
            output.push_str(&format!("- {}\n", code_span(file)));
        }
    }
    output.push_str(
        "\n任务详情、验收历史与证据保存在 XUSHU 本地数据库中，不在项目文件里；请以上述任务 ID 查询。\n\n",
    );

    output.push_str("## 冲突与需要复核的事项\n\n");
    if package.conflicts.is_empty() {
        output.push_str("未发现结构化冲突。\n\n");
    } else {
        for conflict in &package.conflicts {
            output.push_str(&format!(
                "- [{}] {}：{}\n",
                conflict.kind, conflict.subject, conflict.detail
            ));
        }
        output.push('\n');
    }

    output.push_str("## 缺失信息与过期风险\n\n");
    if package.missing_information.is_empty() {
        output.push_str("未记录到缺失信息。\n\n");
    } else {
        for item in &package.missing_information {
            output.push_str(&format!("- {item}\n"));
        }
        output.push('\n');
    }

    output.push_str("## 数据来源\n\n");
    output.push_str(&format!(
        "- 数据库读取时间（UTC）：{}\n",
        package.sources.database_read_at
    ));
    output.push_str(&format!(
        "- Git 检查时间（UTC）：{}\n",
        package
            .sources
            .git_checked_at
            .as_deref()
            .unwrap_or("UNKNOWN")
    ));
    output.push_str(&format!(
        "- 任务：全部 {} 条；进行中/未完成 {} 条；已完成 {} 条；草稿 {} 条\n",
        package.sources.task_candidate_count,
        package.sources.active_task_count,
        package.sources.done_task_count,
        package.sources.draft_task_count
    ));
    output.push_str(&format!(
        "- 记忆：已确认 {} 条；尚未确认 {} 条；已归档 {} 条\n",
        package.sources.memory_confirmed_active_count,
        package.sources.memory_unconfirmed_active_count,
        package.sources.memory_archived_count
    ));
    output.push_str(&format!(
        "- 决策：已确认 {} 条；草稿 {} 条；已被取代 {} 条；已废弃 {} 条\n",
        package.sources.decision_confirmed_count,
        package.sources.decision_draft_count,
        package.sources.decision_superseded_count,
        package.sources.decision_deprecated_count
    ));
    output.push_str(&format!(
        "- 验证运行：{} 条；Claude Code 事件：{} 条；会话：{} 个\n",
        package.sources.verification_run_count,
        package.sources.claude_event_count,
        package.sources.claude_session_count
    ));
    output.push_str("\n**包含上限**\n\n");
    for limit in &package.sources.included_limits {
        output.push_str(&format!("- {limit}\n"));
    }
    output.push_str("\n**排除项**\n\n");
    for exclusion in &package.sources.exclusions {
        output.push_str(&format!("- {exclusion}\n"));
    }

    output.push_str(
        "\n---\n\n本恢复包不是 Claude Code 自身的会话记忆；复制后仍需在新会话中明确读取上述文件，并重新核验真实状态。\n",
    );
    output
}

fn push_task_section(output: &mut String, title: &str, tasks: &[RecoveryTaskItem]) {
    output.push_str(&format!("### {title}\n\n"));
    if tasks.is_empty() {
        output.push_str("无。\n\n");
        return;
    }
    for task in tasks {
        output.push_str(&format!(
            "- {}（{}；修订 {}；优先级 {}）\n",
            task.title,
            task_status_label(task.status),
            task.revision,
            task_priority_label(task.priority)
        ));
        output.push_str(&format!(
            "  - 验收：{}/{} 必需条件已满足\n",
            task.satisfied_required_count, task.required_count
        ));
        if let Some(reason) = &task.block_reason {
            output.push_str(&format!("  - 阻塞原因：{reason}\n"));
        }
        for blocker in &task.blockers {
            output.push_str(&format!("  - 未满足：{blocker}\n"));
        }
    }
    output.push('\n');
}

fn push_memory_markdown(output: &mut String, memory: &RecoveryMemoryItem) {
    output.push_str(&format!("#### {}\n\n", memory.title));
    output.push_str(&format!(
        "- 来源：{}；状态：{}；修订 {}；更新时间（UTC）：{}\n",
        memory_source_label(memory.source_type),
        if memory.confirmation_status == MemoryConfirmationStatus::Confirmed {
            "已确认"
        } else {
            "未确认"
        },
        memory.revision,
        memory.updated_at
    ));
    if let Some(task_title) = &memory.linked_task_title {
        output.push_str(&format!(
            "- 关联任务：{}{}\n",
            task_title,
            if memory.linked_task_archived {
                "（已归档）"
            } else {
                ""
            }
        ));
    }
    output.push('\n');
    push_quote(output, &memory.content);
    output.push('\n');
}

fn push_quote(output: &mut String, value: &str) {
    let text = truncate_text(value, MARKDOWN_CONTENT_MAX);
    for line in text.lines() {
        output.push_str("> ");
        output.push_str(line);
        output.push('\n');
    }
    if text.is_empty() {
        output.push_str(">\n");
    }
}

fn bounded_value(value: String, max_chars: usize) -> String {
    let mut chars = value.chars();
    let prefix = chars
        .by_ref()
        .take(max_chars.saturating_sub(1))
        .collect::<String>();
    if chars.next().is_some() {
        format!("{prefix}…")
    } else {
        value
    }
}

fn truncate_text(value: &str, max_chars: usize) -> String {
    let mut chars = value.chars();
    let truncated = chars.by_ref().take(max_chars).collect::<String>();
    if chars.next().is_some() {
        format!("{truncated}\n\n…（内容已截断，完整内容见 XUSHU）")
    } else {
        truncated
    }
}

fn code_span(value: &str) -> String {
    format!("`{}`", value.replace('`', "'"))
}

fn verification_label(result: VerificationResult) -> &'static str {
    match result {
        VerificationResult::Pass => "通过（PASS）",
        VerificationResult::Fail => "失败（FAIL）",
        VerificationResult::NotRun => "未运行（NOT_RUN）",
        VerificationResult::Inconclusive => "无法判定（INCONCLUSIVE）",
        VerificationResult::Stale => "已过期（STALE）",
    }
}

fn task_status_label(status: TaskStatus) -> &'static str {
    match status {
        TaskStatus::Draft => "草稿",
        TaskStatus::Ready => "就绪",
        TaskStatus::Running => "执行中",
        TaskStatus::AwaitingVerification => "待验证",
        TaskStatus::Done => "已完成",
        TaskStatus::Blocked => "已阻塞",
        TaskStatus::Cancelled => "已取消",
    }
}

fn task_priority_label(priority: TaskPriority) -> &'static str {
    match priority {
        TaskPriority::Low => "低",
        TaskPriority::Medium => "中",
        TaskPriority::High => "高",
        TaskPriority::Urgent => "紧急",
    }
}

fn memory_category_label(category: MemoryCategory) -> &'static str {
    match category {
        MemoryCategory::Goal => "项目目标",
        MemoryCategory::Constraint => "开发约束",
        MemoryCategory::Architecture => "技术架构决策",
        MemoryCategory::Convention => "开发规范",
        MemoryCategory::Fact => "已确认的重要事实",
        MemoryCategory::Risk => "风险与已知问题",
        MemoryCategory::FollowUp => "后续待处理事项",
        MemoryCategory::Knowledge => "项目知识",
    }
}

fn memory_source_label(source: MemorySourceType) -> &'static str {
    match source {
        MemorySourceType::User => "用户",
        MemorySourceType::System => "XUSHU 系统",
        MemorySourceType::Git => "Git",
        MemorySourceType::Verification => "独立验证",
        MemorySourceType::Hook => "Claude Code Hook",
        MemorySourceType::Inference => "AI 推断（未确认）",
    }
}

fn decision_status_label(status: DecisionStatus) -> &'static str {
    match status {
        DecisionStatus::Draft => "草稿",
        DecisionStatus::Confirmed => "已确认",
        DecisionStatus::Superseded => "已被取代",
        DecisionStatus::Deprecated => "已废弃",
    }
}

fn next_step_kind_label(kind: RecoveryNextStepKind) -> &'static str {
    match kind {
        RecoveryNextStepKind::UserSelected => "用户明确指定",
        RecoveryNextStepKind::Suggested => "系统建议（需人工确认）",
        RecoveryNextStepKind::NeedsConfirmation => "需要人工确认",
    }
}

fn hex(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        use std::fmt::Write;
        let _ = write!(output, "{byte:02x}");
    }
    output
}
