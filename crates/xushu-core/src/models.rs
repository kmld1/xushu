use std::fmt;
use std::str::FromStr;

use rusqlite::types::{FromSql, FromSqlError, FromSqlResult, ToSql, ToSqlOutput, ValueRef};
use serde::{Deserialize, Serialize};

use crate::error::CoreError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Draft,
    Ready,
    Running,
    AwaitingVerification,
    Done,
    Blocked,
    Cancelled,
}

impl TaskStatus {
    pub const ALL: [Self; 7] = [
        Self::Draft,
        Self::Ready,
        Self::Running,
        Self::AwaitingVerification,
        Self::Done,
        Self::Blocked,
        Self::Cancelled,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::Ready => "ready",
            Self::Running => "running",
            Self::AwaitingVerification => "awaiting_verification",
            Self::Done => "done",
            Self::Blocked => "blocked",
            Self::Cancelled => "cancelled",
        }
    }
}

impl fmt::Display for TaskStatus {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for TaskStatus {
    type Err = CoreError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim() {
            "draft" => Ok(Self::Draft),
            "ready" => Ok(Self::Ready),
            "running" => Ok(Self::Running),
            "awaiting_verification" => Ok(Self::AwaitingVerification),
            "done" => Ok(Self::Done),
            "blocked" => Ok(Self::Blocked),
            "cancelled" => Ok(Self::Cancelled),
            _ => Err(CoreError::Validation(format!("无效任务状态: {value}"))),
        }
    }
}

impl FromSql for TaskStatus {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        value
            .as_str()?
            .parse()
            .map_err(|_| FromSqlError::InvalidType)
    }
}

impl ToSql for TaskStatus {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        Ok(ToSqlOutput::Borrowed(ValueRef::Text(
            self.as_str().as_bytes(),
        )))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskPriority {
    Low,
    Medium,
    High,
    Urgent,
}

impl TaskPriority {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
            Self::Urgent => "urgent",
        }
    }
}

impl fmt::Display for TaskPriority {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for TaskPriority {
    type Err = CoreError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim() {
            "low" => Ok(Self::Low),
            "medium" => Ok(Self::Medium),
            "high" => Ok(Self::High),
            "urgent" => Ok(Self::Urgent),
            _ => Err(CoreError::Validation(format!("无效任务优先级: {value}"))),
        }
    }
}

impl FromSql for TaskPriority {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        value
            .as_str()?
            .parse()
            .map_err(|_| FromSqlError::InvalidType)
    }
}

impl ToSql for TaskPriority {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        Ok(ToSqlOutput::Borrowed(ValueRef::Text(
            self.as_str().as_bytes(),
        )))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CriterionKind {
    Auto,
    Manual,
}

impl CriterionKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Manual => "manual",
        }
    }
}

impl fmt::Display for CriterionKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for CriterionKind {
    type Err = CoreError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim() {
            "auto" => Ok(Self::Auto),
            "manual" => Ok(Self::Manual),
            _ => Err(CoreError::Validation(format!("无效验收条件类型: {value}"))),
        }
    }
}

impl FromSql for CriterionKind {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        value
            .as_str()?
            .parse()
            .map_err(|_| FromSqlError::InvalidType)
    }
}

impl ToSql for CriterionKind {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        Ok(ToSqlOutput::Borrowed(ValueRef::Text(
            self.as_str().as_bytes(),
        )))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub id: String,
    pub name: String,
    pub root_path: String,
    pub created_at: String,
    pub updated_at: String,
    pub archived_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectGitStatus {
    pub project_id: String,
    pub root_path: String,
    pub branch: Option<String>,
    pub head: Option<String>,
    pub dirty: bool,
    pub detached: bool,
    pub unborn: bool,
    pub checked_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Task {
    pub id: String,
    pub project_id: String,
    pub title: String,
    pub description: String,
    pub priority: TaskPriority,
    pub status: TaskStatus,
    pub revision: i64,
    pub block_reason: Option<String>,
    pub archived_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AcceptanceCriterion {
    pub id: String,
    pub task_id: String,
    pub description: String,
    pub kind: CriterionKind,
    pub required: bool,
    pub revision: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskTransition {
    pub id: String,
    pub task_id: String,
    pub from_status: Option<TaskStatus>,
    pub to_status: TaskStatus,
    pub reason: Option<String>,
    pub task_revision: i64,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManualConfirmation {
    pub id: String,
    pub criterion_id: String,
    pub criterion_revision: i64,
    pub task_revision: i64,
    pub confirmed_at: String,
    pub note: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CriterionView {
    pub criterion: AcceptanceCriterion,
    pub satisfied: bool,
    pub confirmation: Option<ManualConfirmation>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CompletionStatus {
    pub can_complete: bool,
    pub required_count: i64,
    pub satisfied_required_count: i64,
    pub blockers: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskDetail {
    pub task: Task,
    pub criteria: Vec<CriterionView>,
    pub transitions: Vec<TaskTransition>,
    pub allowed_transitions: Vec<TaskStatus>,
    pub completion: CompletionStatus,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskCounts {
    pub total: i64,
    pub draft: i64,
    pub ready: i64,
    pub running: i64,
    pub awaiting_verification: i64,
    pub done: i64,
    pub blocked: i64,
    pub cancelled: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceSummary {
    pub project_count: i64,
    pub archived_project_count: i64,
    pub task_counts: TaskCounts,
    pub recent_tasks: Vec<Task>,
    pub blocked_tasks: Vec<Task>,
    pub awaiting_tasks: Vec<Task>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportProjectInput {
    pub root_path: String,
    pub name: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateProjectInput {
    pub project_id: String,
    pub name: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateTaskInput {
    pub project_id: String,
    pub title: String,
    pub description: Option<String>,
    pub priority: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateTaskInput {
    pub task_id: String,
    pub expected_revision: i64,
    pub title: String,
    pub description: Option<String>,
    pub priority: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArchiveTaskInput {
    pub task_id: String,
    pub expected_revision: i64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransitionTaskInput {
    pub task_id: String,
    pub expected_revision: i64,
    pub to_status: String,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReopenTaskInput {
    pub task_id: String,
    pub expected_revision: i64,
    pub reason: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateCriterionInput {
    pub task_id: String,
    pub expected_task_revision: i64,
    pub description: String,
    pub kind: String,
    pub required: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateCriterionInput {
    pub criterion_id: String,
    pub expected_task_revision: i64,
    pub expected_criterion_revision: i64,
    pub description: String,
    pub kind: String,
    pub required: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoveCriterionInput {
    pub criterion_id: String,
    pub expected_task_revision: i64,
    pub expected_criterion_revision: i64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfirmManualInput {
    pub criterion_id: String,
    pub expected_task_revision: i64,
    pub expected_criterion_revision: i64,
    pub note: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskFilter {
    pub project_id: Option<String>,
    pub status: Option<String>,
    pub search: Option<String>,
    pub include_archived: Option<bool>,
}
