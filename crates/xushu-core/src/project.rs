use std::path::Path;

use rusqlite::{params, Connection, OptionalExtension, Row};
use uuid::Uuid;

use crate::db::{now_utc_rfc3339, Database};
use crate::error::CoreError;
use crate::git;
use crate::models::{ImportProjectInput, Project, ProjectGitStatus, UpdateProjectInput};
use crate::validation::{project_path, required_text, validate_uuid};

const PROJECT_NAME_MAX: usize = 120;

pub(crate) struct ProjectRepository<'a> {
    connection: &'a Connection,
}

impl<'a> ProjectRepository<'a> {
    pub(crate) fn new(connection: &'a Connection) -> Self {
        Self { connection }
    }

    pub(crate) fn insert(&self, project: &Project) -> Result<(), CoreError> {
        self.connection
            .execute(
                "INSERT INTO projects (
                    id, name, root_path, root_path_key, created_at, updated_at, archived_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    project.id,
                    project.name,
                    project.root_path,
                    root_path_key(&project.root_path),
                    project.created_at,
                    project.updated_at,
                    project.archived_at
                ],
            )
            .map_err(map_project_write_error)?;
        Ok(())
    }

    pub(crate) fn find_by_id(&self, id: &str) -> Result<Option<Project>, CoreError> {
        self.connection
            .query_row(
                "SELECT id, name, root_path, created_at, updated_at, archived_at
                 FROM projects WHERE id = ?1",
                [id],
                row_to_project,
            )
            .optional()
            .map_err(Into::into)
    }

    pub(crate) fn find_by_root_key(&self, key: &str) -> Result<Option<Project>, CoreError> {
        self.connection
            .query_row(
                "SELECT id, name, root_path, created_at, updated_at, archived_at
                 FROM projects WHERE root_path_key = ?1",
                [key],
                row_to_project,
            )
            .optional()
            .map_err(Into::into)
    }

    pub(crate) fn list(&self, include_archived: bool) -> Result<Vec<Project>, CoreError> {
        let sql = if include_archived {
            "SELECT id, name, root_path, created_at, updated_at, archived_at
             FROM projects ORDER BY updated_at DESC, id DESC"
        } else {
            "SELECT id, name, root_path, created_at, updated_at, archived_at
             FROM projects WHERE archived_at IS NULL ORDER BY updated_at DESC, id DESC"
        };

        let mut statement = self.connection.prepare(sql)?;
        let rows = statement.query_map([], row_to_project)?;
        let projects = rows.collect::<Result<Vec<_>, _>>()?;
        Ok(projects)
    }

    pub(crate) fn rename(
        &self,
        id: &str,
        name: &str,
        updated_at: &str,
    ) -> Result<usize, CoreError> {
        let changed = self.connection.execute(
            "UPDATE projects SET name = ?2, updated_at = ?3 WHERE id = ?1",
            params![id, name, updated_at],
        )?;
        Ok(changed)
    }

    pub(crate) fn archive(
        &self,
        id: &str,
        archived_at: &str,
        updated_at: &str,
    ) -> Result<usize, CoreError> {
        let changed = self.connection.execute(
            "UPDATE projects
             SET archived_at = ?2, updated_at = ?3
             WHERE id = ?1 AND archived_at IS NULL",
            params![id, archived_at, updated_at],
        )?;
        Ok(changed)
    }

    pub(crate) fn restore(&self, id: &str, updated_at: &str) -> Result<usize, CoreError> {
        let changed = self.connection.execute(
            "UPDATE projects
             SET archived_at = NULL, updated_at = ?2
             WHERE id = ?1 AND archived_at IS NOT NULL",
            params![id, updated_at],
        )?;
        Ok(changed)
    }

    pub(crate) fn counts(&self) -> Result<(i64, i64), CoreError> {
        let total: i64 = self
            .connection
            .query_row("SELECT COUNT(*) FROM projects", [], |row| row.get(0))?;
        let active: i64 = self.connection.query_row(
            "SELECT COUNT(*) FROM projects WHERE archived_at IS NULL",
            [],
            |row| row.get(0),
        )?;
        Ok((total, active))
    }
}

pub struct ProjectService<'a> {
    database: &'a Database,
}

impl<'a> ProjectService<'a> {
    pub fn new(database: &'a Database) -> Self {
        Self { database }
    }

    /// 导入一个已存在的本地 Git 工作树。Git 检查在数据库事务外执行。
    pub fn import(&self, input: ImportProjectInput) -> Result<Project, CoreError> {
        let raw_path = project_path(&input.root_path)?;
        let inspection = git::inspect(Path::new(&raw_path))?;
        let root_path = inspection.root_path.to_string_lossy().to_string();
        let name = match input.name {
            Some(name) => required_text(&name, "项目名称", PROJECT_NAME_MAX, false)?,
            None => default_project_name(&root_path)?,
        };
        let key = root_path_key(&root_path);

        self.database.with_transaction(|transaction| {
            let repository = ProjectRepository::new(transaction);
            if let Some(existing) = repository.find_by_root_key(&key)? {
                let suffix = if existing.archived_at.is_some() {
                    "（已归档，可恢复）"
                } else {
                    ""
                };
                return Err(CoreError::Conflict(format!(
                    "该 Git 项目已导入：{}{suffix}",
                    existing.name
                )));
            }

            let now = now_utc_rfc3339();
            let project = Project {
                id: Uuid::new_v4().to_string(),
                name,
                root_path,
                created_at: now.clone(),
                updated_at: now,
                archived_at: None,
            };
            repository.insert(&project)?;
            Ok(project)
        })
    }

    pub fn list(&self, include_archived: bool) -> Result<Vec<Project>, CoreError> {
        let connection = self.database.lock()?;
        ProjectRepository::new(&connection).list(include_archived)
    }

    pub fn get(&self, id: &str) -> Result<Project, CoreError> {
        let id = validate_uuid(id, "项目 ID")?;
        let connection = self.database.lock()?;
        ProjectRepository::new(&connection)
            .find_by_id(&id)?
            .ok_or(CoreError::NotFound)
    }

    pub fn rename(&self, input: UpdateProjectInput) -> Result<Project, CoreError> {
        let id = validate_uuid(&input.project_id, "项目 ID")?;
        let name = required_text(&input.name, "项目名称", PROJECT_NAME_MAX, false)?;

        self.database.with_transaction(|transaction| {
            let repository = ProjectRepository::new(transaction);
            let existing = repository.find_by_id(&id)?.ok_or(CoreError::NotFound)?;
            let now = now_utc_rfc3339();
            repository.rename(&id, &name, &now)?;
            Ok(Project {
                name,
                updated_at: now,
                ..existing
            })
        })
    }

    pub fn archive(&self, id: &str) -> Result<Project, CoreError> {
        let id = validate_uuid(id, "项目 ID")?;
        self.database.with_transaction(|transaction| {
            let repository = ProjectRepository::new(transaction);
            let existing = repository.find_by_id(&id)?.ok_or(CoreError::NotFound)?;
            if existing.archived_at.is_some() {
                return Ok(existing);
            }
            let now = now_utc_rfc3339();
            repository.archive(&id, &now, &now)?;
            Ok(Project {
                archived_at: Some(now.clone()),
                updated_at: now,
                ..existing
            })
        })
    }

    pub fn restore(&self, id: &str) -> Result<Project, CoreError> {
        let id = validate_uuid(id, "项目 ID")?;
        self.database.with_transaction(|transaction| {
            let repository = ProjectRepository::new(transaction);
            let existing = repository.find_by_id(&id)?.ok_or(CoreError::NotFound)?;
            if existing.archived_at.is_none() {
                return Ok(existing);
            }
            let now = now_utc_rfc3339();
            repository.restore(&id, &now)?;
            Ok(Project {
                archived_at: None,
                updated_at: now,
                ..existing
            })
        })
    }

    pub fn git_status(&self, id: &str) -> Result<ProjectGitStatus, CoreError> {
        let id = validate_uuid(id, "项目 ID")?;
        let project = {
            let connection = self.database.lock()?;
            ProjectRepository::new(&connection)
                .find_by_id(&id)?
                .ok_or(CoreError::NotFound)?
        };

        let inspection = git::inspect(Path::new(&project.root_path))?;

        Ok(ProjectGitStatus {
            project_id: project.id,
            root_path: inspection.root_path.to_string_lossy().to_string(),
            branch: inspection.branch,
            head: inspection.head,
            dirty: inspection.dirty,
            detached: inspection.detached,
            unborn: inspection.unborn,
            checked_at: now_utc_rfc3339(),
        })
    }
}

fn row_to_project(row: &Row<'_>) -> rusqlite::Result<Project> {
    Ok(Project {
        id: row.get(0)?,
        name: row.get(1)?,
        root_path: row.get(2)?,
        created_at: row.get(3)?,
        updated_at: row.get(4)?,
        archived_at: row.get(5)?,
    })
}

fn default_project_name(root_path: &str) -> Result<String, CoreError> {
    let name = Path::new(root_path)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("未命名项目")
        .trim();

    if name.is_empty() {
        return Ok("未命名项目".to_owned());
    }

    let truncated: String = name.chars().take(PROJECT_NAME_MAX).collect();
    required_text(&truncated, "项目名称", PROJECT_NAME_MAX, false)
}

fn root_path_key(root_path: &str) -> String {
    root_path.replace('/', "\\").to_lowercase()
}

fn map_project_write_error(error: rusqlite::Error) -> CoreError {
    if let rusqlite::Error::SqliteFailure(failure, _) = &error {
        if failure.code == rusqlite::ErrorCode::ConstraintViolation {
            return CoreError::Conflict("该 Git 根目录已导入".to_owned());
        }
    }
    CoreError::Database(error)
}
