//! 追加式开发决策记录。
//!
//! 决策可以解释“为什么采用当前方案”，但不能修改任务状态、验收条件或证据。
//! 已确认决策不可原地改写；需要变化时创建新决策并显式取代旧决策，历史两个方向
//! 都保留引用。

use std::fmt;
use std::str::FromStr;

use rusqlite::types::{FromSql, FromSqlError, FromSqlResult, ToSql, ToSqlOutput, ValueRef};
use rusqlite::{params, Connection, OptionalExtension, Row};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::db::{now_utc_rfc3339, Database};
use crate::error::CoreError;
use crate::memory::LinkedTaskRef;
use crate::models::{Project, TaskStatus};
use crate::project::ProjectRepository;
use crate::task::TaskRepository;
use crate::validation::{optional_nonempty_text, required_text, validate_revision, validate_uuid};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DecisionStatus {
    Draft,
    Confirmed,
    Superseded,
    Deprecated,
}

impl DecisionStatus {
    pub const ALL: [Self; 4] = [
        Self::Draft,
        Self::Confirmed,
        Self::Superseded,
        Self::Deprecated,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::Confirmed => "confirmed",
            Self::Superseded => "superseded",
            Self::Deprecated => "deprecated",
        }
    }
}

impl fmt::Display for DecisionStatus {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for DecisionStatus {
    type Err = CoreError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim() {
            "draft" => Ok(Self::Draft),
            "confirmed" => Ok(Self::Confirmed),
            "superseded" => Ok(Self::Superseded),
            "deprecated" => Ok(Self::Deprecated),
            _ => Err(CoreError::Validation(format!("无效决策状态: {value}"))),
        }
    }
}

impl FromSql for DecisionStatus {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        value
            .as_str()?
            .parse()
            .map_err(|_| FromSqlError::InvalidType)
    }
}

impl ToSql for DecisionStatus {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        Ok(ToSqlOutput::Borrowed(ValueRef::Text(
            self.as_str().as_bytes(),
        )))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Decision {
    pub id: String,
    pub project_id: String,
    pub task_id: Option<String>,
    pub title: String,
    pub problem: String,
    pub context: String,
    pub alternatives: Vec<String>,
    pub decision: String,
    pub rationale: String,
    pub risks: String,
    pub status: DecisionStatus,
    pub supersedes_id: Option<String>,
    pub superseded_by_id: Option<String>,
    pub revision: i64,
    pub created_at: String,
    pub updated_at: String,
    pub confirmed_at: Option<String>,
    pub deprecated_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DecisionView {
    pub decision: Decision,
    pub linked_task: Option<LinkedTaskRef>,
    pub supersedes_title: Option<String>,
    pub superseded_by_title: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DecisionRevision {
    pub id: String,
    pub decision_id: String,
    pub revision: i64,
    pub title: String,
    pub problem: String,
    pub context: String,
    pub alternatives: Vec<String>,
    pub decision: String,
    pub rationale: String,
    pub risks: String,
    pub status: DecisionStatus,
    pub supersedes_id: Option<String>,
    pub superseded_by_id: Option<String>,
    pub change_note: Option<String>,
    pub changed_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DecisionDetail {
    pub decision: DecisionView,
    pub revisions: Vec<DecisionRevision>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateDecisionInput {
    pub project_id: String,
    pub task_id: Option<String>,
    pub title: String,
    pub problem: String,
    pub context: String,
    #[serde(default)]
    pub alternatives: Vec<String>,
    pub decision: String,
    pub rationale: String,
    pub risks: String,
    pub supersedes_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateDecisionInput {
    pub project_id: String,
    pub decision_id: String,
    pub expected_revision: i64,
    pub task_id: Option<String>,
    pub title: String,
    pub problem: String,
    pub context: String,
    #[serde(default)]
    pub alternatives: Vec<String>,
    pub decision: String,
    pub rationale: String,
    pub risks: String,
    pub supersedes_id: Option<String>,
    pub change_note: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfirmDecisionInput {
    pub project_id: String,
    pub decision_id: String,
    pub expected_revision: i64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeprecateDecisionInput {
    pub project_id: String,
    pub decision_id: String,
    pub expected_revision: i64,
    pub reason: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DecisionFilter {
    pub project_id: String,
    pub status: Option<String>,
    pub task_id: Option<String>,
}

const DECISION_TITLE_MAX: usize = 200;
const DECISION_PROBLEM_MAX: usize = 4_000;
const DECISION_CONTEXT_MAX: usize = 10_000;
const ALTERNATIVE_MAX: usize = 2_000;
const MAX_ALTERNATIVES: usize = 20;
const DECISION_TEXT_MAX: usize = 4_000;
const DECISION_RATIONALE_MAX: usize = 10_000;
const DECISION_RISKS_MAX: usize = 10_000;
const CHANGE_NOTE_MAX: usize = 1_000;
const LIST_LIMIT: i64 = 500;

pub struct DecisionService<'a> {
    database: &'a Database,
}

impl<'a> DecisionService<'a> {
    pub fn new(database: &'a Database) -> Self {
        Self { database }
    }

    pub fn create(&self, input: CreateDecisionInput) -> Result<DecisionView, CoreError> {
        let project_id = validate_uuid(&input.project_id, "项目 ID")?;
        let task_id = validate_optional_uuid(input.task_id.as_deref(), "任务 ID")?;
        let supersedes_id =
            validate_optional_uuid(input.supersedes_id.as_deref(), "被取代决策 ID")?;
        let title = required_text(&input.title, "决策标题", DECISION_TITLE_MAX, false)?;
        let problem = required_text(&input.problem, "问题", DECISION_PROBLEM_MAX, true)?;
        let context = required_text(&input.context, "背景与约束", DECISION_CONTEXT_MAX, true)?;
        let decision = required_text(&input.decision, "最终方案", DECISION_TEXT_MAX, true)?;
        let rationale = required_text(&input.rationale, "决策理由", DECISION_RATIONALE_MAX, true)?;
        let risks = required_text(&input.risks, "已知代价与风险", DECISION_RISKS_MAX, true)?;
        let alternatives = validate_alternatives(input.alternatives)?;

        self.database.with_transaction(|transaction| {
            require_active_project(transaction, &project_id)?;
            validate_task_link(transaction, &project_id, task_id.as_deref())?;
            validate_supersedes(transaction, &project_id, None, supersedes_id.as_deref())?;

            let now = now_utc_rfc3339();
            let decision = Decision {
                id: Uuid::new_v4().to_string(),
                project_id: project_id.clone(),
                task_id,
                title,
                problem,
                context,
                alternatives,
                decision,
                rationale,
                risks,
                status: DecisionStatus::Draft,
                supersedes_id,
                superseded_by_id: None,
                revision: 1,
                created_at: now.clone(),
                updated_at: now.clone(),
                confirmed_at: None,
                deprecated_at: None,
            };
            insert_decision(transaction, &decision)?;
            insert_revision(transaction, &decision, Some("创建"), &now)?;
            load_decision_view(transaction, &project_id, &decision.id)
        })
    }

    pub fn update(&self, input: UpdateDecisionInput) -> Result<DecisionView, CoreError> {
        let project_id = validate_uuid(&input.project_id, "项目 ID")?;
        let decision_id = validate_uuid(&input.decision_id, "决策 ID")?;
        validate_revision(input.expected_revision, "决策修订号")?;
        let task_id = validate_optional_uuid(input.task_id.as_deref(), "任务 ID")?;
        let supersedes_id =
            validate_optional_uuid(input.supersedes_id.as_deref(), "被取代决策 ID")?;
        let title = required_text(&input.title, "决策标题", DECISION_TITLE_MAX, false)?;
        let problem = required_text(&input.problem, "问题", DECISION_PROBLEM_MAX, true)?;
        let context = required_text(&input.context, "背景与约束", DECISION_CONTEXT_MAX, true)?;
        let decision = required_text(&input.decision, "最终方案", DECISION_TEXT_MAX, true)?;
        let rationale = required_text(&input.rationale, "决策理由", DECISION_RATIONALE_MAX, true)?;
        let risks = required_text(&input.risks, "已知代价与风险", DECISION_RISKS_MAX, true)?;
        let alternatives = validate_alternatives(input.alternatives)?;
        let change_note =
            optional_nonempty_text(input.change_note.as_deref(), "变更说明", CHANGE_NOTE_MAX)?;

        self.database.with_transaction(|transaction| {
            require_active_project(transaction, &project_id)?;
            let existing = find_decision(transaction, &project_id, &decision_id)?
                .ok_or(CoreError::NotFound)?;
            check_decision_revision(&existing, input.expected_revision)?;
            if existing.status != DecisionStatus::Draft {
                return Err(CoreError::Validation(
                    "只有草稿决策可以编辑；已确认决策请创建取代决策或标记废弃".to_owned(),
                ));
            }
            validate_task_link(transaction, &project_id, task_id.as_deref())?;
            validate_supersedes(
                transaction,
                &project_id,
                Some(&decision_id),
                supersedes_id.as_deref(),
            )?;

            let now = now_utc_rfc3339();
            let updated = Decision {
                task_id,
                title,
                problem,
                context,
                alternatives,
                decision,
                rationale,
                risks,
                supersedes_id,
                revision: existing.revision + 1,
                updated_at: now.clone(),
                ..existing
            };
            update_decision_row(transaction, &updated)?;
            insert_revision(
                transaction,
                &updated,
                change_note.as_deref().or(Some("更新决策")),
                &now,
            )?;
            load_decision_view(transaction, &project_id, &decision_id)
        })
    }

    pub fn confirm(&self, input: ConfirmDecisionInput) -> Result<DecisionView, CoreError> {
        let project_id = validate_uuid(&input.project_id, "项目 ID")?;
        let decision_id = validate_uuid(&input.decision_id, "决策 ID")?;
        validate_revision(input.expected_revision, "决策修订号")?;

        self.database.with_transaction(|transaction| {
            require_active_project(transaction, &project_id)?;
            let existing = find_decision(transaction, &project_id, &decision_id)?
                .ok_or(CoreError::NotFound)?;
            check_decision_revision(&existing, input.expected_revision)?;
            if existing.status == DecisionStatus::Confirmed {
                return load_decision_view(transaction, &project_id, &decision_id);
            }
            if existing.status != DecisionStatus::Draft {
                return Err(CoreError::Validation("只有草稿决策可以确认".to_owned()));
            }

            let now = now_utc_rfc3339();
            if let Some(supersedes_id) = existing.supersedes_id.as_deref() {
                let old = find_decision(transaction, &project_id, supersedes_id)?
                    .ok_or(CoreError::NotFound)?;
                if old.status != DecisionStatus::Confirmed {
                    return Err(CoreError::Validation(
                        "只能取代仍然有效且已确认的决策".to_owned(),
                    ));
                }
                let superseded = Decision {
                    status: DecisionStatus::Superseded,
                    superseded_by_id: Some(existing.id.clone()),
                    revision: old.revision + 1,
                    updated_at: now.clone(),
                    ..old
                };
                update_decision_row(transaction, &superseded)?;
                insert_revision(transaction, &superseded, Some("被新决策取代"), &now)?;
            }

            let confirmed = Decision {
                status: DecisionStatus::Confirmed,
                confirmed_at: Some(now.clone()),
                revision: existing.revision + 1,
                updated_at: now.clone(),
                ..existing
            };
            update_decision_row(transaction, &confirmed)?;
            insert_revision(transaction, &confirmed, Some("确认决策"), &now)?;
            load_decision_view(transaction, &project_id, &decision_id)
        })
    }

    pub fn deprecate(&self, input: DeprecateDecisionInput) -> Result<DecisionView, CoreError> {
        let project_id = validate_uuid(&input.project_id, "项目 ID")?;
        let decision_id = validate_uuid(&input.decision_id, "决策 ID")?;
        validate_revision(input.expected_revision, "决策修订号")?;
        let reason = required_text(&input.reason, "废弃原因", CHANGE_NOTE_MAX, true)?;

        self.database.with_transaction(|transaction| {
            require_active_project(transaction, &project_id)?;
            let existing = find_decision(transaction, &project_id, &decision_id)?
                .ok_or(CoreError::NotFound)?;
            check_decision_revision(&existing, input.expected_revision)?;
            if existing.status == DecisionStatus::Deprecated {
                return load_decision_view(transaction, &project_id, &decision_id);
            }
            if existing.status == DecisionStatus::Superseded {
                return Err(CoreError::Validation(
                    "已被取代的决策不能再次标记废弃".to_owned(),
                ));
            }

            let now = now_utc_rfc3339();
            let deprecated = Decision {
                status: DecisionStatus::Deprecated,
                deprecated_at: Some(now.clone()),
                revision: existing.revision + 1,
                updated_at: now.clone(),
                ..existing
            };
            update_decision_row(transaction, &deprecated)?;
            insert_revision(
                transaction,
                &deprecated,
                Some(&format!("废弃：{reason}")),
                &now,
            )?;
            load_decision_view(transaction, &project_id, &decision_id)
        })
    }

    pub fn get(&self, project_id: &str, decision_id: &str) -> Result<DecisionDetail, CoreError> {
        let project_id = validate_uuid(project_id, "项目 ID")?;
        let decision_id = validate_uuid(decision_id, "决策 ID")?;
        let connection = self.database.lock()?;
        let decision = load_decision_view(&connection, &project_id, &decision_id)?;
        let revisions = revisions_for(&connection, &decision_id)?;
        Ok(DecisionDetail {
            decision,
            revisions,
        })
    }

    pub fn list(&self, filter: DecisionFilter) -> Result<Vec<DecisionView>, CoreError> {
        let project_id = validate_uuid(&filter.project_id, "项目 ID")?;
        let status = filter
            .status
            .as_deref()
            .map(DecisionStatus::from_str)
            .transpose()?;
        let task_id = validate_optional_uuid(filter.task_id.as_deref(), "任务 ID")?;

        let connection = self.database.lock()?;
        require_project(&connection, &project_id)?;
        let mut statement = connection.prepare(
            "SELECT d.id, d.project_id, d.task_id, d.title, d.problem, d.context,
                    d.alternatives_json, d.decision, d.rationale, d.risks, d.status,
                    d.supersedes_id, d.superseded_by_id, d.revision, d.created_at,
                    d.updated_at, d.confirmed_at, d.deprecated_at,
                    t.id, t.title, t.status, t.archived_at,
                    old.title, new.title
             FROM decisions d
             LEFT JOIN tasks t ON t.id = d.task_id
             LEFT JOIN decisions old ON old.id = d.supersedes_id
             LEFT JOIN decisions new ON new.id = d.superseded_by_id
             WHERE d.project_id = ?1
               AND (?2 IS NULL OR d.status = ?2)
               AND (?3 IS NULL OR d.task_id = ?3)
             ORDER BY d.updated_at DESC, d.id DESC
             LIMIT ?4",
        )?;
        let rows = statement.query_map(
            params![project_id, status, task_id, LIST_LIMIT],
            row_to_decision_view,
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

fn validate_task_link(
    connection: &Connection,
    project_id: &str,
    task_id: Option<&str>,
) -> Result<(), CoreError> {
    if let Some(task_id) = task_id {
        let task = TaskRepository::new(connection)
            .find_task(task_id)?
            .ok_or(CoreError::NotFound)?;
        if task.project_id != project_id {
            return Err(CoreError::Validation("关联任务不属于当前项目".to_owned()));
        }
    }
    Ok(())
}

fn validate_supersedes(
    connection: &Connection,
    project_id: &str,
    current_id: Option<&str>,
    supersedes_id: Option<&str>,
) -> Result<(), CoreError> {
    let Some(supersedes_id) = supersedes_id else {
        return Ok(());
    };
    if current_id == Some(supersedes_id) {
        return Err(CoreError::Validation("决策不能取代自身".to_owned()));
    }
    let old = find_decision(connection, project_id, supersedes_id)?.ok_or(CoreError::NotFound)?;
    if old.status != DecisionStatus::Confirmed {
        return Err(CoreError::Validation(
            "只能取代仍然有效且已确认的决策".to_owned(),
        ));
    }
    Ok(())
}

fn validate_alternatives(alternatives: Vec<String>) -> Result<Vec<String>, CoreError> {
    let mut validated = Vec::new();
    for alternative in alternatives {
        let alternative = alternative.trim();
        if alternative.is_empty() {
            continue;
        }
        if validated.len() >= MAX_ALTERNATIVES {
            return Err(CoreError::Validation(format!(
                "备选方案最多 {MAX_ALTERNATIVES} 个"
            )));
        }
        validated.push(required_text(
            alternative,
            "备选方案",
            ALTERNATIVE_MAX,
            true,
        )?);
    }
    Ok(validated)
}

fn alternatives_to_json(alternatives: &[String]) -> Result<String, CoreError> {
    serde_json::to_string(alternatives)
        .map_err(|error| CoreError::Recovery(format!("无法编码备选方案: {error}")))
}

fn parse_alternatives(value: &str) -> Result<Vec<String>, CoreError> {
    serde_json::from_str(value)
        .map_err(|error| CoreError::Recovery(format!("决策备选方案数据无效: {error}")))
}

fn check_decision_revision(decision: &Decision, expected: i64) -> Result<(), CoreError> {
    if decision.revision != expected {
        return Err(CoreError::Conflict(format!(
            "决策已被修改（当前 revision {}），请刷新后重试",
            decision.revision
        )));
    }
    Ok(())
}

fn insert_decision(connection: &Connection, decision: &Decision) -> Result<(), CoreError> {
    connection.execute(
        "INSERT INTO decisions (
            id, project_id, task_id, title, problem, context, alternatives_json,
            decision, rationale, risks, status, supersedes_id, superseded_by_id,
            revision, created_at, updated_at, confirmed_at, deprecated_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18)",
        params![
            decision.id,
            decision.project_id,
            decision.task_id,
            decision.title,
            decision.problem,
            decision.context,
            alternatives_to_json(&decision.alternatives)?,
            decision.decision,
            decision.rationale,
            decision.risks,
            decision.status,
            decision.supersedes_id,
            decision.superseded_by_id,
            decision.revision,
            decision.created_at,
            decision.updated_at,
            decision.confirmed_at,
            decision.deprecated_at
        ],
    )?;
    Ok(())
}

fn update_decision_row(connection: &Connection, decision: &Decision) -> Result<(), CoreError> {
    connection.execute(
        "UPDATE decisions
         SET task_id = ?3, title = ?4, problem = ?5, context = ?6,
             alternatives_json = ?7, decision = ?8, rationale = ?9, risks = ?10,
             status = ?11, supersedes_id = ?12, superseded_by_id = ?13,
             revision = ?14, updated_at = ?15, confirmed_at = ?16, deprecated_at = ?17
         WHERE id = ?1 AND project_id = ?2",
        params![
            decision.id,
            decision.project_id,
            decision.task_id,
            decision.title,
            decision.problem,
            decision.context,
            alternatives_to_json(&decision.alternatives)?,
            decision.decision,
            decision.rationale,
            decision.risks,
            decision.status,
            decision.supersedes_id,
            decision.superseded_by_id,
            decision.revision,
            decision.updated_at,
            decision.confirmed_at,
            decision.deprecated_at
        ],
    )?;
    Ok(())
}

fn insert_revision(
    connection: &Connection,
    decision: &Decision,
    change_note: Option<&str>,
    changed_at: &str,
) -> Result<(), CoreError> {
    connection.execute(
        "INSERT INTO decision_revisions (
            id, decision_id, revision, title, problem, context, alternatives_json,
            decision, rationale, risks, status, supersedes_id, superseded_by_id,
            change_note, changed_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
        params![
            Uuid::new_v4().to_string(),
            decision.id,
            decision.revision,
            decision.title,
            decision.problem,
            decision.context,
            alternatives_to_json(&decision.alternatives)?,
            decision.decision,
            decision.rationale,
            decision.risks,
            decision.status,
            decision.supersedes_id,
            decision.superseded_by_id,
            change_note,
            changed_at
        ],
    )?;
    Ok(())
}

fn find_decision(
    connection: &Connection,
    project_id: &str,
    decision_id: &str,
) -> Result<Option<Decision>, CoreError> {
    connection
        .query_row(
            "SELECT id, project_id, task_id, title, problem, context, alternatives_json,
                    decision, rationale, risks, status, supersedes_id, superseded_by_id,
                    revision, created_at, updated_at, confirmed_at, deprecated_at
             FROM decisions
             WHERE id = ?1 AND project_id = ?2",
            params![decision_id, project_id],
            row_to_decision,
        )
        .optional()
        .map_err(Into::into)
}

fn load_decision_view(
    connection: &Connection,
    project_id: &str,
    decision_id: &str,
) -> Result<DecisionView, CoreError> {
    connection
        .query_row(
            "SELECT d.id, d.project_id, d.task_id, d.title, d.problem, d.context,
                    d.alternatives_json, d.decision, d.rationale, d.risks, d.status,
                    d.supersedes_id, d.superseded_by_id, d.revision, d.created_at,
                    d.updated_at, d.confirmed_at, d.deprecated_at,
                    t.id, t.title, t.status, t.archived_at,
                    old.title, new.title
             FROM decisions d
             LEFT JOIN tasks t ON t.id = d.task_id
             LEFT JOIN decisions old ON old.id = d.supersedes_id
             LEFT JOIN decisions new ON new.id = d.superseded_by_id
             WHERE d.id = ?1 AND d.project_id = ?2",
            params![decision_id, project_id],
            row_to_decision_view,
        )
        .optional()?
        .ok_or(CoreError::NotFound)
}

fn revisions_for(
    connection: &Connection,
    decision_id: &str,
) -> Result<Vec<DecisionRevision>, CoreError> {
    let mut statement = connection.prepare(
        "SELECT id, decision_id, revision, title, problem, context, alternatives_json,
                decision, rationale, risks, status, supersedes_id, superseded_by_id,
                change_note, changed_at
         FROM decision_revisions
         WHERE decision_id = ?1
         ORDER BY revision DESC",
    )?;
    let rows = statement.query_map([decision_id], row_to_revision)?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

fn row_to_decision(row: &Row<'_>) -> rusqlite::Result<Decision> {
    let alternatives_json: String = row.get(6)?;
    Ok(Decision {
        id: row.get(0)?,
        project_id: row.get(1)?,
        task_id: row.get(2)?,
        title: row.get(3)?,
        problem: row.get(4)?,
        context: row.get(5)?,
        alternatives: parse_alternatives(&alternatives_json)
            .map_err(|_| rusqlite::Error::InvalidQuery)?,
        decision: row.get(7)?,
        rationale: row.get(8)?,
        risks: row.get(9)?,
        status: row.get(10)?,
        supersedes_id: row.get(11)?,
        superseded_by_id: row.get(12)?,
        revision: row.get(13)?,
        created_at: row.get(14)?,
        updated_at: row.get(15)?,
        confirmed_at: row.get(16)?,
        deprecated_at: row.get(17)?,
    })
}

fn row_to_decision_view(row: &Row<'_>) -> rusqlite::Result<DecisionView> {
    let alternatives_json: String = row.get(6)?;
    let decision = Decision {
        id: row.get(0)?,
        project_id: row.get(1)?,
        task_id: row.get(2)?,
        title: row.get(3)?,
        problem: row.get(4)?,
        context: row.get(5)?,
        alternatives: parse_alternatives(&alternatives_json)
            .map_err(|_| rusqlite::Error::InvalidQuery)?,
        decision: row.get(7)?,
        rationale: row.get(8)?,
        risks: row.get(9)?,
        status: row.get(10)?,
        supersedes_id: row.get(11)?,
        superseded_by_id: row.get(12)?,
        revision: row.get(13)?,
        created_at: row.get(14)?,
        updated_at: row.get(15)?,
        confirmed_at: row.get(16)?,
        deprecated_at: row.get(17)?,
    };
    let linked_task_id: Option<String> = row.get(18)?;
    let linked_task_title: Option<String> = row.get(19)?;
    let linked_task_status: Option<TaskStatus> = row.get(20)?;
    let linked_task_archived: Option<String> = row.get(21)?;
    let linked_task = match (linked_task_id, linked_task_title, linked_task_status) {
        (Some(id), Some(title), Some(status)) => Some(LinkedTaskRef {
            id,
            title,
            status,
            archived: linked_task_archived.is_some(),
        }),
        _ => None,
    };
    Ok(DecisionView {
        decision,
        linked_task,
        supersedes_title: row.get(22)?,
        superseded_by_title: row.get(23)?,
    })
}

fn row_to_revision(row: &Row<'_>) -> rusqlite::Result<DecisionRevision> {
    let alternatives_json: String = row.get(6)?;
    Ok(DecisionRevision {
        id: row.get(0)?,
        decision_id: row.get(1)?,
        revision: row.get(2)?,
        title: row.get(3)?,
        problem: row.get(4)?,
        context: row.get(5)?,
        alternatives: parse_alternatives(&alternatives_json)
            .map_err(|_| rusqlite::Error::InvalidQuery)?,
        decision: row.get(7)?,
        rationale: row.get(8)?,
        risks: row.get(9)?,
        status: row.get(10)?,
        supersedes_id: row.get(11)?,
        superseded_by_id: row.get(12)?,
        change_note: row.get(13)?,
        changed_at: row.get(14)?,
    })
}
