use std::path::Path;

use rusqlite::{params, Connection, OptionalExtension, Row};
use uuid::Uuid;

use crate::code_state::{self, CodeState};
use crate::db::{now_utc_rfc3339, Database};
use crate::error::CoreError;
use crate::models::{
    AcceptanceCriterion, ArchiveTaskInput, CompletionStatus, ConfirmManualInput,
    CreateCriterionInput, CreateTaskInput, CriterionKind, CriterionView, ManualConfirmation,
    RemoveCriterionInput, ReopenTaskInput, Task, TaskCounts, TaskDetail, TaskFilter, TaskPriority,
    TaskStatus, TaskTransition, TransitionTaskInput, UpdateCriterionInput, UpdateTaskInput,
    WorkspaceSummary,
};
use crate::project::ProjectRepository;
use crate::state_machine;
use crate::validation::{
    optional_nonempty_text, optional_text, required_text, search_text, validate_revision,
    validate_uuid,
};
use crate::verification::{self, VerificationQueueItem, VerificationResult};

const TASK_TITLE_MAX: usize = 200;
const TASK_DESCRIPTION_MAX: usize = 10_000;
const CRITERION_DESCRIPTION_MAX: usize = 1_000;
const REASON_MAX: usize = 1_000;
const NOTE_MAX: usize = 1_000;
const LIST_LIMIT: i64 = 500;

pub(crate) struct TaskRepository<'a> {
    connection: &'a Connection,
}

impl<'a> TaskRepository<'a> {
    pub(crate) fn new(connection: &'a Connection) -> Self {
        Self { connection }
    }

    pub(crate) fn insert_task(&self, task: &Task) -> Result<(), CoreError> {
        self.connection.execute(
            "INSERT INTO tasks (
                id, project_id, title, description, priority, status, revision,
                block_reason, archived_at, created_at, updated_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                task.id,
                task.project_id,
                task.title,
                task.description,
                task.priority,
                task.status,
                task.revision,
                task.block_reason,
                task.archived_at,
                task.created_at,
                task.updated_at
            ],
        )?;
        Ok(())
    }

    pub(crate) fn find_task(&self, id: &str) -> Result<Option<Task>, CoreError> {
        self.connection
            .query_row(
                "SELECT id, project_id, title, description, priority, status, revision,
                        block_reason, archived_at, created_at, updated_at
                 FROM tasks WHERE id = ?1",
                [id],
                row_to_task,
            )
            .optional()
            .map_err(Into::into)
    }

    pub(crate) fn update_content(
        &self,
        id: &str,
        title: &str,
        description: &str,
        priority: TaskPriority,
        revision: i64,
        updated_at: &str,
    ) -> Result<usize, CoreError> {
        let changed = self.connection.execute(
            "UPDATE tasks
             SET title = ?2, description = ?3, priority = ?4, revision = ?5, updated_at = ?6
             WHERE id = ?1",
            params![id, title, description, priority, revision, updated_at],
        )?;
        Ok(changed)
    }

    pub(crate) fn update_status(
        &self,
        id: &str,
        status: TaskStatus,
        block_reason: Option<&str>,
        revision: i64,
        updated_at: &str,
    ) -> Result<usize, CoreError> {
        let changed = self.connection.execute(
            "UPDATE tasks
             SET status = ?2, block_reason = ?3, revision = ?4, updated_at = ?5
             WHERE id = ?1",
            params![id, status, block_reason, revision, updated_at],
        )?;
        Ok(changed)
    }

    pub(crate) fn set_archived(
        &self,
        id: &str,
        archived_at: Option<&str>,
        revision: i64,
        updated_at: &str,
    ) -> Result<usize, CoreError> {
        let changed = self.connection.execute(
            "UPDATE tasks
             SET archived_at = ?2, revision = ?3, updated_at = ?4
             WHERE id = ?1",
            params![id, archived_at, revision, updated_at],
        )?;
        Ok(changed)
    }

    pub(crate) fn bump_revision(
        &self,
        id: &str,
        revision: i64,
        updated_at: &str,
    ) -> Result<usize, CoreError> {
        let changed = self.connection.execute(
            "UPDATE tasks SET revision = ?2, updated_at = ?3 WHERE id = ?1",
            params![id, revision, updated_at],
        )?;
        Ok(changed)
    }

    pub(crate) fn touch(&self, id: &str, updated_at: &str) -> Result<usize, CoreError> {
        let changed = self.connection.execute(
            "UPDATE tasks SET updated_at = ?2 WHERE id = ?1",
            params![id, updated_at],
        )?;
        Ok(changed)
    }

    pub(crate) fn insert_transition(&self, transition: &TaskTransition) -> Result<(), CoreError> {
        self.connection.execute(
            "INSERT INTO task_transitions (
                id, task_id, from_status, to_status, reason, task_revision, created_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                transition.id,
                transition.task_id,
                transition.from_status,
                transition.to_status,
                transition.reason,
                transition.task_revision,
                transition.created_at
            ],
        )?;
        Ok(())
    }

    pub(crate) fn transitions_for_task(
        &self,
        task_id: &str,
    ) -> Result<Vec<TaskTransition>, CoreError> {
        let mut statement = self.connection.prepare(
            "SELECT id, task_id, from_status, to_status, reason, task_revision, created_at
             FROM task_transitions
             WHERE task_id = ?1
             ORDER BY rowid ASC",
        )?;
        let rows = statement.query_map([task_id], row_to_transition)?;
        let transitions = rows.collect::<Result<Vec<_>, _>>()?;
        Ok(transitions)
    }

    pub(crate) fn list(
        &self,
        project_id: Option<&str>,
        status: Option<TaskStatus>,
        include_archived: bool,
        search_pattern: Option<&str>,
    ) -> Result<Vec<Task>, CoreError> {
        let include_archived = if include_archived { 1_i64 } else { 0_i64 };
        let mut statement = self.connection.prepare(
            "SELECT t.id, t.project_id, t.title, t.description, t.priority, t.status,
                    t.revision, t.block_reason, t.archived_at, t.created_at, t.updated_at
             FROM tasks t
             JOIN projects p ON p.id = t.project_id
             WHERE (?1 IS NULL OR t.project_id = ?1)
               AND (?2 IS NULL OR t.status = ?2)
               AND (?3 = 1 OR (t.archived_at IS NULL AND p.archived_at IS NULL))
               AND (
                   ?4 IS NULL
                   OR t.title LIKE ?4 ESCAPE '\\'
                   OR t.description LIKE ?4 ESCAPE '\\'
               )
             ORDER BY t.updated_at DESC, t.id DESC
             LIMIT ?5",
        )?;
        let rows = statement.query_map(
            params![
                project_id,
                status,
                include_archived,
                search_pattern,
                LIST_LIMIT
            ],
            row_to_task,
        )?;
        let tasks = rows.collect::<Result<Vec<_>, _>>()?;
        Ok(tasks)
    }

    /// 恢复包使用：不因项目已归档而隐藏任务，但仍排除已归档任务。
    pub(crate) fn recovery_candidates(
        &self,
        project_id: &str,
        limit: i64,
    ) -> Result<Vec<Task>, CoreError> {
        let mut statement = self.connection.prepare(
            "SELECT id, project_id, title, description, priority, status, revision,
                    block_reason, archived_at, created_at, updated_at
             FROM tasks
             WHERE project_id = ?1 AND archived_at IS NULL
             ORDER BY updated_at DESC, id DESC
             LIMIT ?2",
        )?;
        let rows = statement.query_map(params![project_id, limit], row_to_task)?;
        let tasks = rows.collect::<Result<Vec<_>, _>>()?;
        Ok(tasks)
    }

    pub(crate) fn active_tasks(
        &self,
        status: Option<TaskStatus>,
        limit: i64,
    ) -> Result<Vec<Task>, CoreError> {
        let mut statement = self.connection.prepare(
            "SELECT t.id, t.project_id, t.title, t.description, t.priority, t.status,
                    t.revision, t.block_reason, t.archived_at, t.created_at, t.updated_at
             FROM tasks t
             JOIN projects p ON p.id = t.project_id
             WHERE t.archived_at IS NULL
               AND p.archived_at IS NULL
               AND (?1 IS NULL OR t.status = ?1)
             ORDER BY t.updated_at DESC, t.id DESC
             LIMIT ?2",
        )?;
        let rows = statement.query_map(params![status, limit], row_to_task)?;
        let tasks = rows.collect::<Result<Vec<_>, _>>()?;
        Ok(tasks)
    }

    pub(crate) fn counts(&self) -> Result<TaskCounts, CoreError> {
        let mut counts = TaskCounts::default();
        let mut statement = self.connection.prepare(
            "SELECT t.status, COUNT(*)
             FROM tasks t
             JOIN projects p ON p.id = t.project_id
             WHERE t.archived_at IS NULL AND p.archived_at IS NULL
             GROUP BY t.status",
        )?;
        let rows = statement.query_map([], |row| {
            Ok((row.get::<_, TaskStatus>(0)?, row.get::<_, i64>(1)?))
        })?;

        for row in rows {
            let (status, count) = row?;
            counts.total += count;
            match status {
                TaskStatus::Draft => counts.draft += count,
                TaskStatus::Ready => counts.ready += count,
                TaskStatus::Running => counts.running += count,
                TaskStatus::AwaitingVerification => counts.awaiting_verification += count,
                TaskStatus::Done => counts.done += count,
                TaskStatus::Blocked => counts.blocked += count,
                TaskStatus::Cancelled => counts.cancelled += count,
            }
        }

        Ok(counts)
    }

    pub(crate) fn project_archived(&self, project_id: &str) -> Result<bool, CoreError> {
        let archived: Option<Option<String>> = self
            .connection
            .query_row(
                "SELECT archived_at FROM projects WHERE id = ?1",
                [project_id],
                |row| row.get(0),
            )
            .optional()?;
        Ok(archived.flatten().is_some())
    }

    pub(crate) fn required_criterion_count(&self, task_id: &str) -> Result<i64, CoreError> {
        let count = self.connection.query_row(
            "SELECT COUNT(*) FROM acceptance_criteria WHERE task_id = ?1 AND required = 1",
            [task_id],
            |row| row.get(0),
        )?;
        Ok(count)
    }

    pub(crate) fn has_auto_criteria(&self, task_id: &str) -> Result<bool, CoreError> {
        let count: i64 = self.connection.query_row(
            "SELECT COUNT(*) FROM acceptance_criteria WHERE task_id = ?1 AND kind = 'auto'",
            [task_id],
            |row| row.get(0),
        )?;
        Ok(count > 0)
    }

    pub(crate) fn project_root(&self, project_id: &str) -> Result<String, CoreError> {
        self.connection
            .query_row(
                "SELECT root_path FROM projects WHERE id = ?1",
                [project_id],
                |row| row.get(0),
            )
            .map_err(Into::into)
    }

    pub(crate) fn project_name(&self, project_id: &str) -> Result<String, CoreError> {
        self.connection
            .query_row(
                "SELECT name FROM projects WHERE id = ?1",
                [project_id],
                |row| row.get(0),
            )
            .map_err(Into::into)
    }

    pub(crate) fn running_verification_count(&self, task_id: &str) -> Result<i64, CoreError> {
        let count = self.connection.query_row(
            "SELECT COUNT(*) FROM verification_runs
             WHERE task_id = ?1 AND execution_status = 'running'",
            [task_id],
            |row| row.get(0),
        )?;
        Ok(count)
    }

    pub(crate) fn criteria_for_task(
        &self,
        task_id: &str,
    ) -> Result<Vec<AcceptanceCriterion>, CoreError> {
        let mut statement = self.connection.prepare(
            "SELECT id, task_id, description, kind, required, revision, created_at, updated_at
             FROM acceptance_criteria
             WHERE task_id = ?1
             ORDER BY rowid ASC",
        )?;
        let rows = statement.query_map([task_id], row_to_criterion)?;
        let criteria = rows.collect::<Result<Vec<_>, _>>()?;
        Ok(criteria)
    }

    pub(crate) fn criterion_by_id(
        &self,
        id: &str,
    ) -> Result<Option<AcceptanceCriterion>, CoreError> {
        self.connection
            .query_row(
                "SELECT id, task_id, description, kind, required, revision, created_at, updated_at
                 FROM acceptance_criteria WHERE id = ?1",
                [id],
                row_to_criterion,
            )
            .optional()
            .map_err(Into::into)
    }

    pub(crate) fn insert_criterion(
        &self,
        criterion: &AcceptanceCriterion,
    ) -> Result<(), CoreError> {
        self.connection.execute(
            "INSERT INTO acceptance_criteria (
                id, task_id, description, kind, required, revision, created_at, updated_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                criterion.id,
                criterion.task_id,
                criterion.description,
                criterion.kind,
                criterion.required,
                criterion.revision,
                criterion.created_at,
                criterion.updated_at
            ],
        )?;
        Ok(())
    }

    pub(crate) fn update_criterion(
        &self,
        criterion: &AcceptanceCriterion,
    ) -> Result<usize, CoreError> {
        let changed = self.connection.execute(
            "UPDATE acceptance_criteria
             SET description = ?2, kind = ?3, required = ?4, revision = ?5, updated_at = ?6
             WHERE id = ?1 AND task_id = ?7",
            params![
                criterion.id,
                criterion.description,
                criterion.kind,
                criterion.required,
                criterion.revision,
                criterion.updated_at,
                criterion.task_id
            ],
        )?;
        Ok(changed)
    }

    pub(crate) fn delete_criterion(
        &self,
        criterion_id: &str,
        task_id: &str,
    ) -> Result<usize, CoreError> {
        let changed = self.connection.execute(
            "DELETE FROM acceptance_criteria WHERE id = ?1 AND task_id = ?2",
            params![criterion_id, task_id],
        )?;
        Ok(changed)
    }

    pub(crate) fn valid_confirmation(
        &self,
        criterion_id: &str,
        criterion_revision: i64,
        task_revision: i64,
    ) -> Result<Option<ManualConfirmation>, CoreError> {
        self.connection
            .query_row(
                "SELECT id, criterion_id, criterion_revision, task_revision, confirmed_at, note
                 FROM manual_confirmations
                 WHERE criterion_id = ?1 AND criterion_revision = ?2 AND task_revision = ?3
                 ORDER BY confirmed_at DESC
                 LIMIT 1",
                params![criterion_id, criterion_revision, task_revision],
                row_to_confirmation,
            )
            .optional()
            .map_err(Into::into)
    }

    pub(crate) fn insert_confirmation(
        &self,
        confirmation: &ManualConfirmation,
    ) -> Result<(), CoreError> {
        self.connection
            .execute(
                "INSERT INTO manual_confirmations (
                    id, criterion_id, criterion_revision, task_revision, confirmed_at, note
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    confirmation.id,
                    confirmation.criterion_id,
                    confirmation.criterion_revision,
                    confirmation.task_revision,
                    confirmation.confirmed_at,
                    confirmation.note
                ],
            )
            .map_err(map_confirmation_write_error)?;
        Ok(())
    }
}

pub struct TaskService<'a> {
    database: &'a Database,
}

impl<'a> TaskService<'a> {
    pub fn new(database: &'a Database) -> Self {
        Self { database }
    }

    pub fn create_task(&self, input: CreateTaskInput) -> Result<Task, CoreError> {
        let project_id = validate_uuid(&input.project_id, "项目 ID")?;
        let title = required_text(&input.title, "任务标题", TASK_TITLE_MAX, false)?;
        let description = optional_text(
            input.description.as_deref(),
            "任务描述",
            TASK_DESCRIPTION_MAX,
        )?;
        let priority: TaskPriority = input.priority.parse()?;

        self.database.with_transaction(|transaction| {
            ensure_project_active(transaction, &project_id)?;

            let now = now_utc_rfc3339();
            let task = Task {
                id: Uuid::new_v4().to_string(),
                project_id,
                title,
                description,
                priority,
                status: TaskStatus::Draft,
                revision: 1,
                block_reason: None,
                archived_at: None,
                created_at: now.clone(),
                updated_at: now.clone(),
            };
            let repository = TaskRepository::new(transaction);
            repository.insert_task(&task)?;
            repository.insert_transition(&TaskTransition {
                id: Uuid::new_v4().to_string(),
                task_id: task.id.clone(),
                from_status: None,
                to_status: TaskStatus::Draft,
                reason: Some("创建任务".to_owned()),
                task_revision: 1,
                created_at: now,
            })?;
            Ok(task)
        })
    }

    pub fn update_task(&self, input: UpdateTaskInput) -> Result<Task, CoreError> {
        let task_id = validate_uuid(&input.task_id, "任务 ID")?;
        validate_revision(input.expected_revision, "任务修订号")?;
        let title = required_text(&input.title, "任务标题", TASK_TITLE_MAX, false)?;
        let description = optional_text(
            input.description.as_deref(),
            "任务描述",
            TASK_DESCRIPTION_MAX,
        )?;
        let priority: TaskPriority = input.priority.parse()?;

        self.database.with_transaction(|transaction| {
            let repository = TaskRepository::new(transaction);
            let task = require_task(&repository, &task_id)?;
            ensure_project_active(transaction, &task.project_id)?;
            ensure_task_editable(&task)?;
            check_task_revision(&task, input.expected_revision)?;

            let revision = task.revision + 1;
            let now = now_utc_rfc3339();
            repository.update_content(&task.id, &title, &description, priority, revision, &now)?;

            Ok(Task {
                title,
                description,
                priority,
                revision,
                updated_at: now,
                ..task
            })
        })
    }

    pub fn archive_task(&self, input: ArchiveTaskInput) -> Result<Task, CoreError> {
        let task_id = validate_uuid(&input.task_id, "任务 ID")?;
        validate_revision(input.expected_revision, "任务修订号")?;

        self.database.with_transaction(|transaction| {
            let repository = TaskRepository::new(transaction);
            let task = require_task(&repository, &task_id)?;
            ensure_project_active(transaction, &task.project_id)?;
            if task.archived_at.is_some() {
                return Ok(task);
            }
            check_task_revision(&task, input.expected_revision)?;

            let now = now_utc_rfc3339();
            let revision = task.revision + 1;
            repository.set_archived(&task.id, Some(&now), revision, &now)?;
            Ok(Task {
                archived_at: Some(now.clone()),
                revision,
                updated_at: now,
                ..task
            })
        })
    }

    pub fn restore_task(&self, input: ArchiveTaskInput) -> Result<Task, CoreError> {
        let task_id = validate_uuid(&input.task_id, "任务 ID")?;
        validate_revision(input.expected_revision, "任务修订号")?;

        self.database.with_transaction(|transaction| {
            let repository = TaskRepository::new(transaction);
            let task = require_task(&repository, &task_id)?;
            ensure_project_active(transaction, &task.project_id)?;
            if task.archived_at.is_none() {
                return Ok(task);
            }
            check_task_revision(&task, input.expected_revision)?;

            let now = now_utc_rfc3339();
            let revision = task.revision + 1;
            repository.set_archived(&task.id, None, revision, &now)?;
            Ok(Task {
                archived_at: None,
                revision,
                updated_at: now,
                ..task
            })
        })
    }

    pub fn transition_task_status(&self, input: TransitionTaskInput) -> Result<Task, CoreError> {
        let task_id = validate_uuid(&input.task_id, "任务 ID")?;
        validate_revision(input.expected_revision, "任务修订号")?;
        let target: TaskStatus = input.to_status.parse()?;
        let reason = optional_nonempty_text(input.reason.as_deref(), "转换原因", REASON_MAX)?;

        if matches!(target, TaskStatus::Blocked | TaskStatus::Cancelled) && reason.is_none() {
            return Err(CoreError::Validation(
                "进入阻塞或取消状态必须填写原因".to_owned(),
            ));
        }

        // 完成门禁需要当前代码指纹。采集可能启动 Git 并读取变更文件，放到事务外，
        // 避免长时间持有写锁；事务内仍会重新校验任务修订与状态。
        let precomputed_code_state = if target == TaskStatus::Done {
            let auto_root = {
                let connection = self.database.lock()?;
                let repository = TaskRepository::new(&connection);
                let current = require_task(&repository, &task_id)?;
                if repository.has_auto_criteria(&current.id)? {
                    repository.project_root(&current.project_id).ok()
                } else {
                    None
                }
            };
            auto_root.and_then(|root| code_state::capture(Path::new(&root)).ok())
        } else {
            None
        };

        self.database.with_transaction(|transaction| {
            let repository = TaskRepository::new(transaction);
            let task = require_task(&repository, &task_id)?;
            ensure_project_active(transaction, &task.project_id)?;
            if task.archived_at.is_some() {
                return Err(CoreError::Validation("已归档任务不能转换状态".to_owned()));
            }
            check_task_revision(&task, input.expected_revision)?;
            state_machine::validate_transition(task.status, target)?;

            if target == TaskStatus::Ready && repository.required_criterion_count(&task.id)? == 0 {
                return Err(CoreError::Validation(
                    "进入就绪状态前至少需要一个必需验收条件".to_owned(),
                ));
            }

            if target == TaskStatus::Done {
                let (_, completion) =
                    criteria_with_completion(transaction, &task, precomputed_code_state.as_ref())?;
                if !completion.can_complete {
                    return Err(CoreError::TaskNotCompletable(
                        completion.blockers.join("；"),
                    ));
                }
            }

            // 离开待验证状态（除非直接完成）意味着返工/阻塞/取消，旧的人工确认
            // 不再适用于新修订，必须重新确认。
            let revision =
                if task.status == TaskStatus::AwaitingVerification && target != TaskStatus::Done {
                    task.revision + 1
                } else {
                    task.revision
                };

            let now = now_utc_rfc3339();
            let block_reason = if target == TaskStatus::Blocked {
                reason.clone()
            } else {
                None
            };
            repository.update_status(&task.id, target, block_reason.as_deref(), revision, &now)?;
            repository.insert_transition(&TaskTransition {
                id: Uuid::new_v4().to_string(),
                task_id: task.id.clone(),
                from_status: Some(task.status),
                to_status: target,
                reason,
                task_revision: revision,
                created_at: now.clone(),
            })?;

            Ok(Task {
                status: target,
                block_reason,
                revision,
                updated_at: now,
                ..task
            })
        })
    }

    pub fn reopen_task(&self, input: ReopenTaskInput) -> Result<Task, CoreError> {
        let task_id = validate_uuid(&input.task_id, "任务 ID")?;
        validate_revision(input.expected_revision, "任务修订号")?;
        let reason = required_text(&input.reason, "重新开启原因", REASON_MAX, true)?;

        self.database.with_transaction(|transaction| {
            let repository = TaskRepository::new(transaction);
            let task = require_task(&repository, &task_id)?;
            ensure_project_active(transaction, &task.project_id)?;
            if task.archived_at.is_some() {
                return Err(CoreError::Validation("已归档任务不能重新开启".to_owned()));
            }
            if task.status != TaskStatus::Cancelled {
                return Err(CoreError::Validation(
                    "只有已取消任务可以重新开启".to_owned(),
                ));
            }
            check_task_revision(&task, input.expected_revision)?;

            let now = now_utc_rfc3339();
            let revision = task.revision + 1;
            repository.update_status(&task.id, TaskStatus::Draft, None, revision, &now)?;
            repository.insert_transition(&TaskTransition {
                id: Uuid::new_v4().to_string(),
                task_id: task.id.clone(),
                from_status: Some(TaskStatus::Cancelled),
                to_status: TaskStatus::Draft,
                reason: Some(reason),
                task_revision: revision,
                created_at: now.clone(),
            })?;

            Ok(Task {
                status: TaskStatus::Draft,
                block_reason: None,
                revision,
                updated_at: now,
                ..task
            })
        })
    }

    pub fn get_task(&self, id: &str) -> Result<TaskDetail, CoreError> {
        let id = validate_uuid(id, "任务 ID")?;

        // 代码指纹需要启动 Git 并读取变更文件，不能持有数据库锁执行；否则会与
        // 后台验证的 finalize 写入相互等待。先短读决定是否需要采集，再在锁外采集。
        let auto_context = {
            let connection = self.database.lock()?;
            let repository = TaskRepository::new(&connection);
            let task = require_task(&repository, &id)?;
            if repository.has_auto_criteria(&task.id)? {
                Some(repository.project_root(&task.project_id).ok())
            } else {
                None
            }
        };
        let current_code_state = auto_context
            .flatten()
            .and_then(|root| code_state::capture(Path::new(&root)).ok());

        let connection = self.database.lock()?;
        let repository = TaskRepository::new(&connection);
        let task = require_task(&repository, &id)?;
        let project_archived = repository.project_archived(&task.project_id)?;
        let (criteria, completion) =
            criteria_with_completion(&connection, &task, current_code_state.as_ref())?;
        let transitions = repository.transitions_for_task(&task.id)?;
        let allowed_transitions = if task.archived_at.is_some() || project_archived {
            Vec::new()
        } else {
            state_machine::allowed_transitions(task.status)
        };

        Ok(TaskDetail {
            task,
            criteria,
            transitions,
            allowed_transitions,
            completion,
        })
    }

    pub fn list_tasks(&self, filter: TaskFilter) -> Result<Vec<Task>, CoreError> {
        let project_id = match filter.project_id {
            Some(id) => Some(validate_uuid(&id, "项目 ID")?),
            None => None,
        };
        let status = match filter.status {
            Some(status) => Some(status.parse::<TaskStatus>()?),
            None => None,
        };
        let search = search_text(filter.search.as_deref())?;
        let search_pattern = search.map(|term| format!("%{}%", escape_like(&term)));

        let connection = self.database.lock()?;
        TaskRepository::new(&connection).list(
            project_id.as_deref(),
            status,
            filter.include_archived.unwrap_or(false),
            search_pattern.as_deref(),
        )
    }

    pub fn list_criteria(&self, task_id: &str) -> Result<Vec<AcceptanceCriterion>, CoreError> {
        let task_id = validate_uuid(task_id, "任务 ID")?;
        let connection = self.database.lock()?;
        let repository = TaskRepository::new(&connection);
        require_task(&repository, &task_id)?;
        repository.criteria_for_task(&task_id)
    }

    pub fn create_criterion(
        &self,
        input: CreateCriterionInput,
    ) -> Result<AcceptanceCriterion, CoreError> {
        let task_id = validate_uuid(&input.task_id, "任务 ID")?;
        validate_revision(input.expected_task_revision, "任务修订号")?;
        let description = required_text(
            &input.description,
            "验收条件",
            CRITERION_DESCRIPTION_MAX,
            true,
        )?;
        let kind: CriterionKind = input.kind.parse()?;

        self.database.with_transaction(|transaction| {
            let repository = TaskRepository::new(transaction);
            let task = require_task(&repository, &task_id)?;
            ensure_project_active(transaction, &task.project_id)?;
            ensure_task_editable(&task)?;
            check_task_revision(&task, input.expected_task_revision)?;

            let now = now_utc_rfc3339();
            let criterion = AcceptanceCriterion {
                id: Uuid::new_v4().to_string(),
                task_id: task.id.clone(),
                description,
                kind,
                required: input.required,
                revision: 1,
                created_at: now.clone(),
                updated_at: now.clone(),
            };
            repository.insert_criterion(&criterion)?;
            repository.bump_revision(&task.id, task.revision + 1, &now)?;
            Ok(criterion)
        })
    }

    pub fn update_criterion(
        &self,
        input: UpdateCriterionInput,
    ) -> Result<AcceptanceCriterion, CoreError> {
        let criterion_id = validate_uuid(&input.criterion_id, "验收条件 ID")?;
        validate_revision(input.expected_task_revision, "任务修订号")?;
        validate_revision(input.expected_criterion_revision, "验收条件修订号")?;
        let description = required_text(
            &input.description,
            "验收条件",
            CRITERION_DESCRIPTION_MAX,
            true,
        )?;
        let kind: CriterionKind = input.kind.parse()?;

        self.database.with_transaction(|transaction| {
            let repository = TaskRepository::new(transaction);
            let existing = repository
                .criterion_by_id(&criterion_id)?
                .ok_or(CoreError::NotFound)?;
            let task = require_task(&repository, &existing.task_id)?;
            ensure_project_active(transaction, &task.project_id)?;
            ensure_task_editable(&task)?;
            check_task_revision(&task, input.expected_task_revision)?;
            if existing.revision != input.expected_criterion_revision {
                return Err(CoreError::Conflict(format!(
                    "验收条件已被修改（当前 revision {}），请刷新后重试",
                    existing.revision
                )));
            }

            let now = now_utc_rfc3339();
            let criterion = AcceptanceCriterion {
                description,
                kind,
                required: input.required,
                revision: existing.revision + 1,
                updated_at: now.clone(),
                ..existing
            };
            repository.update_criterion(&criterion)?;
            repository.bump_revision(&task.id, task.revision + 1, &now)?;
            Ok(criterion)
        })
    }

    pub fn remove_criterion(&self, input: RemoveCriterionInput) -> Result<(), CoreError> {
        let criterion_id = validate_uuid(&input.criterion_id, "验收条件 ID")?;
        validate_revision(input.expected_task_revision, "任务修订号")?;
        validate_revision(input.expected_criterion_revision, "验收条件修订号")?;

        self.database.with_transaction(|transaction| {
            let repository = TaskRepository::new(transaction);
            let existing = repository
                .criterion_by_id(&criterion_id)?
                .ok_or(CoreError::NotFound)?;
            let task = require_task(&repository, &existing.task_id)?;
            ensure_project_active(transaction, &task.project_id)?;
            ensure_task_editable(&task)?;
            check_task_revision(&task, input.expected_task_revision)?;
            if existing.revision != input.expected_criterion_revision {
                return Err(CoreError::Conflict(format!(
                    "验收条件已被修改（当前 revision {}），请刷新后重试",
                    existing.revision
                )));
            }

            repository.delete_criterion(&criterion_id, &task.id)?;
            repository.bump_revision(&task.id, task.revision + 1, &now_utc_rfc3339())?;
            Ok(())
        })
    }

    pub fn confirm_manual_criterion(
        &self,
        input: ConfirmManualInput,
    ) -> Result<ManualConfirmation, CoreError> {
        let criterion_id = validate_uuid(&input.criterion_id, "验收条件 ID")?;
        validate_revision(input.expected_task_revision, "任务修订号")?;
        validate_revision(input.expected_criterion_revision, "验收条件修订号")?;
        let note = optional_nonempty_text(input.note.as_deref(), "确认备注", NOTE_MAX)?;

        self.database.with_transaction(|transaction| {
            let repository = TaskRepository::new(transaction);
            let criterion = repository
                .criterion_by_id(&criterion_id)?
                .ok_or(CoreError::NotFound)?;
            let task = require_task(&repository, &criterion.task_id)?;
            ensure_project_active(transaction, &task.project_id)?;
            if task.archived_at.is_some() {
                return Err(CoreError::Validation("已归档任务不能确认验收".to_owned()));
            }
            if task.status != TaskStatus::AwaitingVerification {
                return Err(CoreError::Validation(
                    "只能在待验证状态确认人工验收条件".to_owned(),
                ));
            }
            if criterion.kind != CriterionKind::Manual {
                return Err(CoreError::Validation(
                    "自动验收条件不能通过人工方式确认".to_owned(),
                ));
            }
            check_task_revision(&task, input.expected_task_revision)?;
            if criterion.revision != input.expected_criterion_revision {
                return Err(CoreError::Conflict(format!(
                    "验收条件已被修改（当前 revision {}），请刷新后重试",
                    criterion.revision
                )));
            }

            let now = now_utc_rfc3339();
            let confirmation = ManualConfirmation {
                id: Uuid::new_v4().to_string(),
                criterion_id: criterion.id,
                criterion_revision: criterion.revision,
                task_revision: task.revision,
                confirmed_at: now.clone(),
                note,
            };
            repository.insert_confirmation(&confirmation)?;
            repository.touch(&task.id, &now)?;
            Ok(confirmation)
        })
    }

    pub fn verification_queue(&self) -> Result<Vec<VerificationQueueItem>, CoreError> {
        let tasks = {
            let connection = self.database.lock()?;
            TaskRepository::new(&connection)
                .active_tasks(Some(TaskStatus::AwaitingVerification), 50)?
        };
        let mut items = Vec::with_capacity(tasks.len());

        for task in tasks {
            // 与 get_task 相同：Git 指纹采集必须在数据库锁之外执行。
            let (has_auto, project_root, project_name) = {
                let connection = self.database.lock()?;
                let repository = TaskRepository::new(&connection);
                let criteria = repository.criteria_for_task(&task.id)?;
                let has_auto = criteria
                    .iter()
                    .any(|criterion| criterion.kind == CriterionKind::Auto);
                let project_root = if has_auto {
                    repository.project_root(&task.project_id).ok()
                } else {
                    None
                };
                let project_name = repository.project_name(&task.project_id)?;
                (has_auto, project_root, project_name)
            };
            let current_code_state = if has_auto {
                project_root
                    .as_deref()
                    .and_then(|root| code_state::capture(Path::new(root)).ok())
            } else {
                None
            };

            let connection = self.database.lock()?;
            let repository = TaskRepository::new(&connection);
            // 指纹采集期间任务可能被修改；用当前任务重新评估，避免队列显示
            // 基于旧修订的“门禁已满足”。
            let fresh_task = require_task(&repository, &task.id)?;
            let (views, completion) =
                criteria_with_completion(&connection, &fresh_task, current_code_state.as_ref())?;
            let auto_total = views
                .iter()
                .filter(|view| view.criterion.kind == CriterionKind::Auto)
                .count() as i64;
            let auto_satisfied = views
                .iter()
                .filter(|view| view.criterion.kind == CriterionKind::Auto && view.satisfied)
                .count() as i64;
            let manual_pending = views
                .iter()
                .filter(|view| {
                    view.criterion.kind == CriterionKind::Manual
                        && view.criterion.required
                        && !view.satisfied
                })
                .count() as i64;
            let running_count = repository.running_verification_count(&fresh_task.id)?;
            drop(connection);

            items.push(VerificationQueueItem {
                task: fresh_task,
                project_name,
                completion,
                auto_total,
                auto_satisfied,
                manual_pending,
                running_count,
            });
        }

        Ok(items)
    }

    pub fn workspace_summary(&self) -> Result<WorkspaceSummary, CoreError> {
        let connection = self.database.lock()?;
        let project_repository = ProjectRepository::new(&connection);
        let repository = TaskRepository::new(&connection);

        let (project_total, project_active) = project_repository.counts()?;
        let task_counts = repository.counts()?;
        let recent_tasks = repository.active_tasks(None, 5)?;
        let blocked_tasks = repository.active_tasks(Some(TaskStatus::Blocked), 5)?;
        let awaiting_tasks = repository.active_tasks(Some(TaskStatus::AwaitingVerification), 5)?;

        Ok(WorkspaceSummary {
            project_count: project_active,
            archived_project_count: project_total - project_active,
            task_counts,
            recent_tasks,
            blocked_tasks,
            awaiting_tasks,
        })
    }
}

fn require_task(repository: &TaskRepository<'_>, id: &str) -> Result<Task, CoreError> {
    repository.find_task(id)?.ok_or(CoreError::NotFound)
}

fn ensure_project_active(connection: &Connection, project_id: &str) -> Result<(), CoreError> {
    let archived: Option<Option<String>> = connection
        .query_row(
            "SELECT archived_at FROM projects WHERE id = ?1",
            [project_id],
            |row| row.get(0),
        )
        .optional()?;

    match archived {
        None => Err(CoreError::NotFound),
        Some(Some(_)) => Err(CoreError::Validation(
            "项目已归档，不能修改其任务；请先恢复项目".to_owned(),
        )),
        Some(None) => Ok(()),
    }
}

fn ensure_task_editable(task: &Task) -> Result<(), CoreError> {
    if task.archived_at.is_some() {
        return Err(CoreError::Validation("已归档任务不能修改".to_owned()));
    }
    if matches!(task.status, TaskStatus::Done | TaskStatus::Cancelled) {
        return Err(CoreError::Validation(
            "已结束任务不能修改；已取消任务可使用重新开启".to_owned(),
        ));
    }
    Ok(())
}

fn check_task_revision(task: &Task, expected: i64) -> Result<(), CoreError> {
    if task.revision != expected {
        return Err(CoreError::Conflict(format!(
            "任务已被修改（当前 revision {}），请刷新后重试",
            task.revision
        )));
    }
    Ok(())
}

pub(crate) fn criteria_with_completion(
    connection: &Connection,
    task: &Task,
    current_code_state: Option<&CodeState>,
) -> Result<(Vec<CriterionView>, CompletionStatus), CoreError> {
    let repository = TaskRepository::new(connection);
    let criteria = repository.criteria_for_task(&task.id)?;
    let mut views = Vec::with_capacity(criteria.len());
    let mut blockers = Vec::new();
    let mut required_count = 0_i64;
    let mut satisfied_required_count = 0_i64;

    for criterion in criteria {
        let (confirmation, verification, satisfied) = match criterion.kind {
            CriterionKind::Manual => {
                let confirmation = repository.valid_confirmation(
                    &criterion.id,
                    criterion.revision,
                    task.revision,
                )?;
                let satisfied = confirmation.is_some();
                (confirmation, None, satisfied)
            }
            CriterionKind::Auto => {
                let verification = verification::evaluate_criterion(
                    connection,
                    task,
                    &criterion,
                    current_code_state,
                )?;
                let satisfied = verification.status == VerificationResult::Pass;
                (None, Some(verification), satisfied)
            }
        };

        if criterion.required {
            required_count += 1;
            if satisfied {
                satisfied_required_count += 1;
            } else if let Some(verification) = &verification {
                blockers.push(verification::blocker_for(&criterion, verification));
            } else {
                blockers.push(format!("人工验收条件尚未确认：{}", criterion.description));
            }
        }

        views.push(CriterionView {
            criterion,
            satisfied,
            confirmation,
            verification,
        });
    }

    if required_count == 0 {
        blockers.insert(0, "至少需要一个必需验收条件".to_owned());
    }

    Ok((
        views,
        CompletionStatus {
            can_complete: blockers.is_empty(),
            required_count,
            satisfied_required_count,
            blockers,
        },
    ))
}

fn row_to_task(row: &Row<'_>) -> rusqlite::Result<Task> {
    Ok(Task {
        id: row.get(0)?,
        project_id: row.get(1)?,
        title: row.get(2)?,
        description: row.get(3)?,
        priority: row.get(4)?,
        status: row.get(5)?,
        revision: row.get(6)?,
        block_reason: row.get(7)?,
        archived_at: row.get(8)?,
        created_at: row.get(9)?,
        updated_at: row.get(10)?,
    })
}

fn row_to_criterion(row: &Row<'_>) -> rusqlite::Result<AcceptanceCriterion> {
    Ok(AcceptanceCriterion {
        id: row.get(0)?,
        task_id: row.get(1)?,
        description: row.get(2)?,
        kind: row.get(3)?,
        required: row.get(4)?,
        revision: row.get(5)?,
        created_at: row.get(6)?,
        updated_at: row.get(7)?,
    })
}

fn row_to_transition(row: &Row<'_>) -> rusqlite::Result<TaskTransition> {
    Ok(TaskTransition {
        id: row.get(0)?,
        task_id: row.get(1)?,
        from_status: row.get(2)?,
        to_status: row.get(3)?,
        reason: row.get(4)?,
        task_revision: row.get(5)?,
        created_at: row.get(6)?,
    })
}

fn row_to_confirmation(row: &Row<'_>) -> rusqlite::Result<ManualConfirmation> {
    Ok(ManualConfirmation {
        id: row.get(0)?,
        criterion_id: row.get(1)?,
        criterion_revision: row.get(2)?,
        task_revision: row.get(3)?,
        confirmed_at: row.get(4)?,
        note: row.get(5)?,
    })
}

fn escape_like(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

fn map_confirmation_write_error(error: rusqlite::Error) -> CoreError {
    if let rusqlite::Error::SqliteFailure(failure, _) = &error {
        if failure.code == rusqlite::ErrorCode::ConstraintViolation {
            return CoreError::Conflict("该验收条件在当前任务修订下已经确认".to_owned());
        }
    }
    CoreError::Database(error)
}
