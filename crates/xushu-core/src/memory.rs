//! 项目长期记忆。
//!
//! 记忆是开发上下文，不是业务事实的替代品。已确认记忆可以进入恢复包；未确认
//! 记忆与已归档记忆默认不进入。任何记忆写入都不会修改任务状态、验收条件或证据。

use std::fmt;
use std::str::FromStr;

use rusqlite::types::{FromSql, FromSqlError, FromSqlResult, ToSql, ToSqlOutput, ValueRef};
use rusqlite::{params, Connection, OptionalExtension, Row};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::db::{now_utc_rfc3339, Database};
use crate::error::CoreError;
use crate::models::{Project, TaskStatus};
use crate::project::ProjectRepository;
use crate::task::TaskRepository;
use crate::validation::{optional_nonempty_text, required_text, validate_revision, validate_uuid};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryCategory {
    Goal,
    Constraint,
    Architecture,
    Convention,
    Fact,
    Risk,
    FollowUp,
    Knowledge,
}

impl MemoryCategory {
    pub const ALL: [Self; 8] = [
        Self::Goal,
        Self::Constraint,
        Self::Architecture,
        Self::Convention,
        Self::Fact,
        Self::Risk,
        Self::FollowUp,
        Self::Knowledge,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Goal => "goal",
            Self::Constraint => "constraint",
            Self::Architecture => "architecture",
            Self::Convention => "convention",
            Self::Fact => "fact",
            Self::Risk => "risk",
            Self::FollowUp => "follow_up",
            Self::Knowledge => "knowledge",
        }
    }
}

impl fmt::Display for MemoryCategory {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for MemoryCategory {
    type Err = CoreError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim() {
            "goal" => Ok(Self::Goal),
            "constraint" => Ok(Self::Constraint),
            "architecture" => Ok(Self::Architecture),
            "convention" => Ok(Self::Convention),
            "fact" => Ok(Self::Fact),
            "risk" => Ok(Self::Risk),
            "follow_up" => Ok(Self::FollowUp),
            "knowledge" => Ok(Self::Knowledge),
            _ => Err(CoreError::Validation(format!("无效记忆类别: {value}"))),
        }
    }
}

impl FromSql for MemoryCategory {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        value
            .as_str()?
            .parse()
            .map_err(|_| FromSqlError::InvalidType)
    }
}

impl ToSql for MemoryCategory {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        Ok(ToSqlOutput::Borrowed(ValueRef::Text(
            self.as_str().as_bytes(),
        )))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemorySourceType {
    User,
    System,
    Git,
    Verification,
    Hook,
    Inference,
}

impl MemorySourceType {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::User => "user",
            Self::System => "system",
            Self::Git => "git",
            Self::Verification => "verification",
            Self::Hook => "hook",
            Self::Inference => "inference",
        }
    }
}

impl fmt::Display for MemorySourceType {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for MemorySourceType {
    type Err = CoreError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim() {
            "user" => Ok(Self::User),
            "system" => Ok(Self::System),
            "git" => Ok(Self::Git),
            "verification" => Ok(Self::Verification),
            "hook" => Ok(Self::Hook),
            "inference" => Ok(Self::Inference),
            _ => Err(CoreError::Validation(format!("无效记忆来源: {value}"))),
        }
    }
}

impl FromSql for MemorySourceType {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        value
            .as_str()?
            .parse()
            .map_err(|_| FromSqlError::InvalidType)
    }
}

impl ToSql for MemorySourceType {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        Ok(ToSqlOutput::Borrowed(ValueRef::Text(
            self.as_str().as_bytes(),
        )))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryConfirmationStatus {
    Unconfirmed,
    Confirmed,
}

impl MemoryConfirmationStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Unconfirmed => "unconfirmed",
            Self::Confirmed => "confirmed",
        }
    }
}

impl fmt::Display for MemoryConfirmationStatus {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for MemoryConfirmationStatus {
    type Err = CoreError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim() {
            "unconfirmed" => Ok(Self::Unconfirmed),
            "confirmed" => Ok(Self::Confirmed),
            _ => Err(CoreError::Validation(format!("无效记忆确认状态: {value}"))),
        }
    }
}

impl FromSql for MemoryConfirmationStatus {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        value
            .as_str()?
            .parse()
            .map_err(|_| FromSqlError::InvalidType)
    }
}

impl ToSql for MemoryConfirmationStatus {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        Ok(ToSqlOutput::Borrowed(ValueRef::Text(
            self.as_str().as_bytes(),
        )))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryValidityStatus {
    Active,
    Archived,
}

impl MemoryValidityStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Archived => "archived",
        }
    }
}

impl fmt::Display for MemoryValidityStatus {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for MemoryValidityStatus {
    type Err = CoreError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim() {
            "active" => Ok(Self::Active),
            "archived" => Ok(Self::Archived),
            _ => Err(CoreError::Validation(format!("无效记忆有效性: {value}"))),
        }
    }
}

impl FromSql for MemoryValidityStatus {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        value
            .as_str()?
            .parse()
            .map_err(|_| FromSqlError::InvalidType)
    }
}

impl ToSql for MemoryValidityStatus {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        Ok(ToSqlOutput::Borrowed(ValueRef::Text(
            self.as_str().as_bytes(),
        )))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectMemory {
    pub id: String,
    pub project_id: String,
    pub category: MemoryCategory,
    pub title: String,
    pub content: String,
    pub source_type: MemorySourceType,
    pub confirmation_status: MemoryConfirmationStatus,
    pub validity_status: MemoryValidityStatus,
    pub linked_task_id: Option<String>,
    pub linked_evidence_id: Option<String>,
    pub revision: i64,
    pub created_at: String,
    pub updated_at: String,
    pub confirmed_at: Option<String>,
    pub archived_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LinkedTaskRef {
    pub id: String,
    pub title: String,
    pub status: TaskStatus,
    pub archived: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectMemoryView {
    pub memory: ProjectMemory,
    pub linked_task: Option<LinkedTaskRef>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectMemoryRevision {
    pub id: String,
    pub memory_id: String,
    pub revision: i64,
    pub category: MemoryCategory,
    pub title: String,
    pub content: String,
    pub source_type: MemorySourceType,
    pub confirmation_status: MemoryConfirmationStatus,
    pub validity_status: MemoryValidityStatus,
    pub linked_task_id: Option<String>,
    pub linked_evidence_id: Option<String>,
    pub change_note: Option<String>,
    pub changed_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryDetail {
    pub memory: ProjectMemoryView,
    pub revisions: Vec<ProjectMemoryRevision>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateMemoryInput {
    pub project_id: String,
    pub category: String,
    pub title: String,
    pub content: String,
    pub source_type: Option<String>,
    pub confirmed: Option<bool>,
    pub linked_task_id: Option<String>,
    pub linked_evidence_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateMemoryInput {
    pub project_id: String,
    pub memory_id: String,
    pub expected_revision: i64,
    pub category: String,
    pub title: String,
    pub content: String,
    pub linked_task_id: Option<String>,
    pub linked_evidence_id: Option<String>,
    pub change_note: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfirmMemoryInput {
    pub project_id: String,
    pub memory_id: String,
    pub expected_revision: i64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArchiveMemoryInput {
    pub project_id: String,
    pub memory_id: String,
    pub expected_revision: i64,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryFilter {
    pub project_id: String,
    pub category: Option<String>,
    pub include_archived: Option<bool>,
    pub task_id: Option<String>,
}

const MEMORY_TITLE_MAX: usize = 200;
const MEMORY_CONTENT_MAX: usize = 20_000;
const CHANGE_NOTE_MAX: usize = 1_000;
const LIST_LIMIT: i64 = 500;

pub struct MemoryService<'a> {
    database: &'a Database,
}

impl<'a> MemoryService<'a> {
    pub fn new(database: &'a Database) -> Self {
        Self { database }
    }

    pub fn create(&self, input: CreateMemoryInput) -> Result<ProjectMemoryView, CoreError> {
        let project_id = validate_uuid(&input.project_id, "项目 ID")?;
        let category: MemoryCategory = input.category.parse()?;
        let title = required_text(&input.title, "记忆标题", MEMORY_TITLE_MAX, false)?;
        let content = required_text(&input.content, "记忆内容", MEMORY_CONTENT_MAX, true)?;
        let source_type: MemorySourceType = input
            .source_type
            .as_deref()
            .unwrap_or(MemorySourceType::User.as_str())
            .parse()?;
        let confirmed = input
            .confirmed
            .unwrap_or(source_type != MemorySourceType::Inference);
        if confirmed && source_type == MemorySourceType::Inference {
            return Err(CoreError::Validation(
                "AI 推断不能直接创建为已确认记忆；请先保存为未确认，再由人工确认".to_owned(),
            ));
        }
        let linked_task_id = validate_optional_uuid(input.linked_task_id.as_deref(), "任务 ID")?;
        let linked_evidence_id =
            validate_optional_uuid(input.linked_evidence_id.as_deref(), "证据 ID")?;

        self.database.with_transaction(|transaction| {
            require_active_project(transaction, &project_id)?;
            validate_links(
                transaction,
                &project_id,
                linked_task_id.as_deref(),
                linked_evidence_id.as_deref(),
            )?;

            let now = now_utc_rfc3339();
            let memory = ProjectMemory {
                id: Uuid::new_v4().to_string(),
                project_id: project_id.clone(),
                category,
                title,
                content,
                source_type,
                confirmation_status: if confirmed {
                    MemoryConfirmationStatus::Confirmed
                } else {
                    MemoryConfirmationStatus::Unconfirmed
                },
                validity_status: MemoryValidityStatus::Active,
                linked_task_id,
                linked_evidence_id,
                revision: 1,
                created_at: now.clone(),
                updated_at: now.clone(),
                confirmed_at: confirmed.then(|| now.clone()),
                archived_at: None,
            };
            insert_memory(transaction, &memory)?;
            insert_revision(transaction, &memory, Some("创建"), &now)?;
            load_memory_view(transaction, &project_id, &memory.id)
        })
    }

    pub fn update(&self, input: UpdateMemoryInput) -> Result<ProjectMemoryView, CoreError> {
        let project_id = validate_uuid(&input.project_id, "项目 ID")?;
        let memory_id = validate_uuid(&input.memory_id, "记忆 ID")?;
        validate_revision(input.expected_revision, "记忆修订号")?;
        let category: MemoryCategory = input.category.parse()?;
        let title = required_text(&input.title, "记忆标题", MEMORY_TITLE_MAX, false)?;
        let content = required_text(&input.content, "记忆内容", MEMORY_CONTENT_MAX, true)?;
        let linked_task_id = validate_optional_uuid(input.linked_task_id.as_deref(), "任务 ID")?;
        let linked_evidence_id =
            validate_optional_uuid(input.linked_evidence_id.as_deref(), "证据 ID")?;
        let change_note =
            optional_nonempty_text(input.change_note.as_deref(), "变更说明", CHANGE_NOTE_MAX)?;

        self.database.with_transaction(|transaction| {
            require_active_project(transaction, &project_id)?;
            let existing =
                find_memory(transaction, &project_id, &memory_id)?.ok_or(CoreError::NotFound)?;
            check_memory_revision(&existing, input.expected_revision)?;
            validate_links(
                transaction,
                &project_id,
                linked_task_id.as_deref(),
                linked_evidence_id.as_deref(),
            )?;

            let now = now_utc_rfc3339();
            let confirmed_at =
                if existing.confirmation_status == MemoryConfirmationStatus::Confirmed {
                    Some(now.clone())
                } else {
                    None
                };
            let updated = ProjectMemory {
                category,
                title,
                content,
                linked_task_id,
                linked_evidence_id,
                revision: existing.revision + 1,
                updated_at: now.clone(),
                confirmed_at,
                ..existing
            };
            update_memory_row(transaction, &updated)?;
            insert_revision(
                transaction,
                &updated,
                change_note.as_deref().or(Some("更新记忆")),
                &now,
            )?;
            load_memory_view(transaction, &project_id, &updated.id)
        })
    }

    pub fn confirm(&self, input: ConfirmMemoryInput) -> Result<ProjectMemoryView, CoreError> {
        let project_id = validate_uuid(&input.project_id, "项目 ID")?;
        let memory_id = validate_uuid(&input.memory_id, "记忆 ID")?;
        validate_revision(input.expected_revision, "记忆修订号")?;

        self.database.with_transaction(|transaction| {
            require_active_project(transaction, &project_id)?;
            let existing =
                find_memory(transaction, &project_id, &memory_id)?.ok_or(CoreError::NotFound)?;
            check_memory_revision(&existing, input.expected_revision)?;
            if existing.confirmation_status == MemoryConfirmationStatus::Confirmed {
                return load_memory_view(transaction, &project_id, &memory_id);
            }

            let now = now_utc_rfc3339();
            let confirmed = ProjectMemory {
                confirmation_status: MemoryConfirmationStatus::Confirmed,
                confirmed_at: Some(now.clone()),
                revision: existing.revision + 1,
                updated_at: now.clone(),
                ..existing
            };
            update_memory_row(transaction, &confirmed)?;
            insert_revision(transaction, &confirmed, Some("人工确认"), &now)?;
            load_memory_view(transaction, &project_id, &memory_id)
        })
    }

    pub fn archive(&self, input: ArchiveMemoryInput) -> Result<ProjectMemoryView, CoreError> {
        let project_id = validate_uuid(&input.project_id, "项目 ID")?;
        let memory_id = validate_uuid(&input.memory_id, "记忆 ID")?;
        validate_revision(input.expected_revision, "记忆修订号")?;
        let reason = optional_nonempty_text(input.reason.as_deref(), "归档原因", CHANGE_NOTE_MAX)?;

        self.database.with_transaction(|transaction| {
            require_active_project(transaction, &project_id)?;
            let existing =
                find_memory(transaction, &project_id, &memory_id)?.ok_or(CoreError::NotFound)?;
            check_memory_revision(&existing, input.expected_revision)?;
            if existing.validity_status == MemoryValidityStatus::Archived {
                return load_memory_view(transaction, &project_id, &memory_id);
            }

            let now = now_utc_rfc3339();
            let archived = ProjectMemory {
                validity_status: MemoryValidityStatus::Archived,
                archived_at: Some(now.clone()),
                revision: existing.revision + 1,
                updated_at: now.clone(),
                ..existing
            };
            update_memory_row(transaction, &archived)?;
            let note = reason
                .as_deref()
                .map(|reason| format!("归档：{reason}"))
                .unwrap_or_else(|| "归档".to_owned());
            insert_revision(transaction, &archived, Some(&note), &now)?;
            load_memory_view(transaction, &project_id, &memory_id)
        })
    }

    pub fn restore(&self, input: ArchiveMemoryInput) -> Result<ProjectMemoryView, CoreError> {
        let project_id = validate_uuid(&input.project_id, "项目 ID")?;
        let memory_id = validate_uuid(&input.memory_id, "记忆 ID")?;
        validate_revision(input.expected_revision, "记忆修订号")?;
        let reason = optional_nonempty_text(input.reason.as_deref(), "恢复原因", CHANGE_NOTE_MAX)?;

        self.database.with_transaction(|transaction| {
            require_active_project(transaction, &project_id)?;
            let existing =
                find_memory(transaction, &project_id, &memory_id)?.ok_or(CoreError::NotFound)?;
            check_memory_revision(&existing, input.expected_revision)?;
            if existing.validity_status == MemoryValidityStatus::Active {
                return load_memory_view(transaction, &project_id, &memory_id);
            }

            let now = now_utc_rfc3339();
            let restored = ProjectMemory {
                validity_status: MemoryValidityStatus::Active,
                archived_at: None,
                revision: existing.revision + 1,
                updated_at: now.clone(),
                ..existing
            };
            update_memory_row(transaction, &restored)?;
            let note = reason
                .as_deref()
                .map(|reason| format!("恢复：{reason}"))
                .unwrap_or_else(|| "恢复".to_owned());
            insert_revision(transaction, &restored, Some(&note), &now)?;
            load_memory_view(transaction, &project_id, &memory_id)
        })
    }

    pub fn get(&self, project_id: &str, memory_id: &str) -> Result<MemoryDetail, CoreError> {
        let project_id = validate_uuid(project_id, "项目 ID")?;
        let memory_id = validate_uuid(memory_id, "记忆 ID")?;
        let connection = self.database.lock()?;
        let memory = load_memory_view(&connection, &project_id, &memory_id)?;
        let revisions = revisions_for(&connection, &memory_id)?;
        Ok(MemoryDetail { memory, revisions })
    }

    pub fn list(&self, filter: MemoryFilter) -> Result<Vec<ProjectMemoryView>, CoreError> {
        let project_id = validate_uuid(&filter.project_id, "项目 ID")?;
        let category = filter
            .category
            .as_deref()
            .map(MemoryCategory::from_str)
            .transpose()?;
        let task_id = validate_optional_uuid(filter.task_id.as_deref(), "任务 ID")?;
        let include_archived = filter.include_archived.unwrap_or(false);

        let connection = self.database.lock()?;
        require_project(&connection, &project_id)?;
        let mut statement = connection.prepare(
            "SELECT m.id, m.project_id, m.category, m.title, m.content, m.source_type,
                    m.confirmation_status, m.validity_status, m.linked_task_id,
                    m.linked_evidence_id, m.revision, m.created_at, m.updated_at,
                    m.confirmed_at, m.archived_at,
                    t.id, t.title, t.status, t.archived_at
             FROM project_memories m
             LEFT JOIN tasks t ON t.id = m.linked_task_id
             WHERE m.project_id = ?1
               AND (?2 IS NULL OR m.category = ?2)
               AND (?3 = 1 OR m.validity_status = 'active')
               AND (?4 IS NULL OR m.linked_task_id = ?4)
             ORDER BY m.updated_at DESC, m.id DESC
             LIMIT ?5",
        )?;
        let rows = statement.query_map(
            params![
                project_id,
                category,
                if include_archived { 1_i64 } else { 0_i64 },
                task_id,
                LIST_LIMIT
            ],
            row_to_memory_view,
        )?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }
}

fn validate_optional_uuid(value: Option<&str>, field: &str) -> Result<Option<String>, CoreError> {
    value.map(|value| validate_uuid(value, field)).transpose()
}

fn require_project(connection: &Connection, project_id: &str) -> Result<Project, CoreError> {
    ProjectRepository::new(connection)
        .find_by_id(project_id)?
        .ok_or(CoreError::NotFound)
}

fn require_active_project(connection: &Connection, project_id: &str) -> Result<Project, CoreError> {
    let project = require_project(connection, project_id)?;
    if project.archived_at.is_some() {
        return Err(CoreError::Validation(
            "项目已归档，不能修改记忆与决策；请先恢复项目".to_owned(),
        ));
    }
    Ok(project)
}

fn validate_links(
    connection: &Connection,
    project_id: &str,
    task_id: Option<&str>,
    evidence_id: Option<&str>,
) -> Result<(), CoreError> {
    if let Some(task_id) = task_id {
        let task = TaskRepository::new(connection)
            .find_task(task_id)?
            .ok_or(CoreError::NotFound)?;
        if task.project_id != project_id {
            return Err(CoreError::Validation("关联任务不属于当前项目".to_owned()));
        }
    }

    if let Some(evidence_id) = evidence_id {
        let owner: Option<String> = connection
            .query_row(
                "SELECT t.project_id
                 FROM evidence e
                 JOIN tasks t ON t.id = e.task_id
                 WHERE e.id = ?1",
                [evidence_id],
                |row| row.get(0),
            )
            .optional()?;
        if owner.as_deref() != Some(project_id) {
            return Err(CoreError::Validation("关联证据不属于当前项目".to_owned()));
        }
    }

    Ok(())
}

fn check_memory_revision(memory: &ProjectMemory, expected: i64) -> Result<(), CoreError> {
    if memory.revision != expected {
        return Err(CoreError::Conflict(format!(
            "记忆已被修改（当前 revision {}），请刷新后重试",
            memory.revision
        )));
    }
    Ok(())
}

fn insert_memory(connection: &Connection, memory: &ProjectMemory) -> Result<(), CoreError> {
    connection.execute(
        "INSERT INTO project_memories (
            id, project_id, category, title, content, source_type,
            confirmation_status, validity_status, linked_task_id, linked_evidence_id,
            revision, created_at, updated_at, confirmed_at, archived_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
        params![
            memory.id,
            memory.project_id,
            memory.category,
            memory.title,
            memory.content,
            memory.source_type,
            memory.confirmation_status,
            memory.validity_status,
            memory.linked_task_id,
            memory.linked_evidence_id,
            memory.revision,
            memory.created_at,
            memory.updated_at,
            memory.confirmed_at,
            memory.archived_at
        ],
    )?;
    Ok(())
}

fn update_memory_row(connection: &Connection, memory: &ProjectMemory) -> Result<(), CoreError> {
    connection.execute(
        "UPDATE project_memories
         SET category = ?3, title = ?4, content = ?5, linked_task_id = ?6,
             linked_evidence_id = ?7, confirmation_status = ?8, validity_status = ?9,
             revision = ?10, updated_at = ?11, confirmed_at = ?12, archived_at = ?13
         WHERE id = ?1 AND project_id = ?2",
        params![
            memory.id,
            memory.project_id,
            memory.category,
            memory.title,
            memory.content,
            memory.linked_task_id,
            memory.linked_evidence_id,
            memory.confirmation_status,
            memory.validity_status,
            memory.revision,
            memory.updated_at,
            memory.confirmed_at,
            memory.archived_at
        ],
    )?;
    Ok(())
}

fn insert_revision(
    connection: &Connection,
    memory: &ProjectMemory,
    change_note: Option<&str>,
    changed_at: &str,
) -> Result<(), CoreError> {
    connection.execute(
        "INSERT INTO project_memory_revisions (
            id, memory_id, revision, category, title, content, source_type,
            confirmation_status, validity_status, linked_task_id, linked_evidence_id,
            change_note, changed_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
        params![
            Uuid::new_v4().to_string(),
            memory.id,
            memory.revision,
            memory.category,
            memory.title,
            memory.content,
            memory.source_type,
            memory.confirmation_status,
            memory.validity_status,
            memory.linked_task_id,
            memory.linked_evidence_id,
            change_note,
            changed_at
        ],
    )?;
    Ok(())
}

fn find_memory(
    connection: &Connection,
    project_id: &str,
    memory_id: &str,
) -> Result<Option<ProjectMemory>, CoreError> {
    connection
        .query_row(
            "SELECT id, project_id, category, title, content, source_type,
                    confirmation_status, validity_status, linked_task_id,
                    linked_evidence_id, revision, created_at, updated_at,
                    confirmed_at, archived_at
             FROM project_memories
             WHERE id = ?1 AND project_id = ?2",
            params![memory_id, project_id],
            row_to_memory,
        )
        .optional()
        .map_err(Into::into)
}

fn load_memory_view(
    connection: &Connection,
    project_id: &str,
    memory_id: &str,
) -> Result<ProjectMemoryView, CoreError> {
    connection
        .query_row(
            "SELECT m.id, m.project_id, m.category, m.title, m.content, m.source_type,
                    m.confirmation_status, m.validity_status, m.linked_task_id,
                    m.linked_evidence_id, m.revision, m.created_at, m.updated_at,
                    m.confirmed_at, m.archived_at,
                    t.id, t.title, t.status, t.archived_at
             FROM project_memories m
             LEFT JOIN tasks t ON t.id = m.linked_task_id
             WHERE m.id = ?1 AND m.project_id = ?2",
            params![memory_id, project_id],
            row_to_memory_view,
        )
        .optional()?
        .ok_or(CoreError::NotFound)
}

fn revisions_for(
    connection: &Connection,
    memory_id: &str,
) -> Result<Vec<ProjectMemoryRevision>, CoreError> {
    let mut statement = connection.prepare(
        "SELECT id, memory_id, revision, category, title, content, source_type,
                confirmation_status, validity_status, linked_task_id, linked_evidence_id,
                change_note, changed_at
         FROM project_memory_revisions
         WHERE memory_id = ?1
         ORDER BY revision DESC",
    )?;
    let rows = statement.query_map([memory_id], row_to_revision)?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

fn row_to_memory(row: &Row<'_>) -> rusqlite::Result<ProjectMemory> {
    Ok(ProjectMemory {
        id: row.get(0)?,
        project_id: row.get(1)?,
        category: row.get(2)?,
        title: row.get(3)?,
        content: row.get(4)?,
        source_type: row.get(5)?,
        confirmation_status: row.get(6)?,
        validity_status: row.get(7)?,
        linked_task_id: row.get(8)?,
        linked_evidence_id: row.get(9)?,
        revision: row.get(10)?,
        created_at: row.get(11)?,
        updated_at: row.get(12)?,
        confirmed_at: row.get(13)?,
        archived_at: row.get(14)?,
    })
}

fn row_to_memory_view(row: &Row<'_>) -> rusqlite::Result<ProjectMemoryView> {
    let memory = ProjectMemory {
        id: row.get(0)?,
        project_id: row.get(1)?,
        category: row.get(2)?,
        title: row.get(3)?,
        content: row.get(4)?,
        source_type: row.get(5)?,
        confirmation_status: row.get(6)?,
        validity_status: row.get(7)?,
        linked_task_id: row.get(8)?,
        linked_evidence_id: row.get(9)?,
        revision: row.get(10)?,
        created_at: row.get(11)?,
        updated_at: row.get(12)?,
        confirmed_at: row.get(13)?,
        archived_at: row.get(14)?,
    };
    let linked_task_id: Option<String> = row.get(15)?;
    let linked_task_title: Option<String> = row.get(16)?;
    let linked_task_status: Option<TaskStatus> = row.get(17)?;
    let linked_task_archived: Option<String> = row.get(18)?;
    let linked_task = match (linked_task_id, linked_task_title, linked_task_status) {
        (Some(id), Some(title), Some(status)) => Some(LinkedTaskRef {
            id,
            title,
            status,
            archived: linked_task_archived.is_some(),
        }),
        _ => None,
    };
    Ok(ProjectMemoryView {
        memory,
        linked_task,
    })
}

fn row_to_revision(row: &Row<'_>) -> rusqlite::Result<ProjectMemoryRevision> {
    Ok(ProjectMemoryRevision {
        id: row.get(0)?,
        memory_id: row.get(1)?,
        revision: row.get(2)?,
        category: row.get(3)?,
        title: row.get(4)?,
        content: row.get(5)?,
        source_type: row.get(6)?,
        confirmation_status: row.get(7)?,
        validity_status: row.get(8)?,
        linked_task_id: row.get(9)?,
        linked_evidence_id: row.get(10)?,
        change_note: row.get(11)?,
        changed_at: row.get(12)?,
    })
}
