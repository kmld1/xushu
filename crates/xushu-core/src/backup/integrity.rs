//! 数据库结构与业务关系检查。
//!
//! SQLite `integrity_check` 通过只证明页面结构可读；这里额外检查外键、项目归属、
//! 版本历史与取代关系。检查结果不能替代任务验收证据，只用于备份可恢复性与
//! 本地数据健康诊断。

use rusqlite::Connection;
use serde::{Deserialize, Serialize};

use crate::db::{now_utc_rfc3339, Database};
use crate::error::CoreError;

pub const REQUIRED_TABLES: &[&str] = &[
    "schema_migrations",
    "projects",
    "tasks",
    "acceptance_criteria",
    "task_transitions",
    "manual_confirmations",
    "verification_specs",
    "verification_runs",
    "verification_checks",
    "evidence",
    "claude_integrations",
    "claude_sessions",
    "claude_tool_calls",
    "claude_events",
    "project_memories",
    "project_memory_revisions",
    "decisions",
    "decision_revisions",
    "project_recovery_preferences",
    "recovery_snapshots",
    "backup_records",
    "backup_settings",
    "restore_operations",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IntegrityStatus {
    Pass,
    Fail,
    NotRun,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IntegrityCheck {
    pub id: String,
    pub label: String,
    pub status: IntegrityStatus,
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DataCounts {
    pub projects: i64,
    pub tasks: i64,
    pub acceptance_criteria: i64,
    pub verification_runs: i64,
    pub verification_checks: i64,
    pub evidence: i64,
    pub claude_sessions: i64,
    pub claude_events: i64,
    pub project_memories: i64,
    pub project_memory_revisions: i64,
    pub decisions: i64,
    pub decision_revisions: i64,
    pub recovery_snapshots: i64,
    pub backup_records: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DataIntegrityReport {
    pub checked_at: String,
    pub ok: bool,
    pub schema_version: i64,
    pub expected_schema_version: i64,
    pub integrity_message: String,
    pub counts: DataCounts,
    pub checks: Vec<IntegrityCheck>,
}

pub struct DataIntegrityService<'a> {
    database: &'a Database,
}

impl<'a> DataIntegrityService<'a> {
    pub fn new(database: &'a Database) -> Self {
        Self { database }
    }

    pub fn check(&self) -> Result<DataIntegrityReport, CoreError> {
        let conn = self.database.lock()?;
        inspect_connection(&conn, crate::db::MIGRATIONS.last().map_or(0, |m| m.version))
    }
}

pub(crate) fn inspect_connection(
    conn: &Connection,
    expected_schema_version: i64,
) -> Result<DataIntegrityReport, CoreError> {
    let mut checks = Vec::new();

    let integrity_rows = {
        let mut statement = conn.prepare("PRAGMA integrity_check")?;
        let rows = statement.query_map([], |row| row.get::<_, String>(0))?;
        rows.collect::<Result<Vec<_>, _>>()?
    };
    let integrity_ok = integrity_rows.len() == 1 && integrity_rows[0].eq_ignore_ascii_case("ok");
    let integrity_message = if integrity_ok {
        "SQLite integrity_check = ok".to_owned()
    } else {
        let summary = integrity_rows
            .iter()
            .take(3)
            .cloned()
            .collect::<Vec<_>>()
            .join("; ");
        format!(
            "SQLite integrity_check 返回 {} 项问题: {summary}",
            integrity_rows.len()
        )
    };
    checks.push(IntegrityCheck {
        id: "sqlite_integrity".to_owned(),
        label: "SQLite 页面完整性".to_owned(),
        status: if integrity_ok {
            IntegrityStatus::Pass
        } else {
            IntegrityStatus::Fail
        },
        detail: Some(integrity_message.clone()),
    });

    let foreign_key_violations = {
        let mut statement = conn.prepare("PRAGMA foreign_key_check")?;
        let mut rows = statement.query([])?;
        let mut count = 0_i64;
        while rows.next()?.is_some() {
            count += 1;
        }
        count
    };
    checks.push(IntegrityCheck {
        id: "foreign_keys".to_owned(),
        label: "外键约束".to_owned(),
        status: if foreign_key_violations == 0 {
            IntegrityStatus::Pass
        } else {
            IntegrityStatus::Fail
        },
        detail: Some(format!("外键违规 {foreign_key_violations} 项")),
    });

    let schema_version: i64 = conn.query_row(
        "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
        [],
        |row| row.get(0),
    )?;
    checks.push(IntegrityCheck {
        id: "schema_version".to_owned(),
        label: "数据库版本".to_owned(),
        status: if schema_version == expected_schema_version {
            IntegrityStatus::Pass
        } else {
            IntegrityStatus::Fail
        },
        detail: Some(format!(
            "当前 v{schema_version}，预期 v{expected_schema_version}"
        )),
    });

    let existing_tables = {
        let mut statement = conn.prepare("SELECT name FROM sqlite_master WHERE type = 'table'")?;
        let rows = statement.query_map([], |row| row.get::<_, String>(0))?;
        rows.collect::<Result<Vec<_>, _>>()?
    };
    let missing_tables: Vec<&str> = REQUIRED_TABLES
        .iter()
        .copied()
        .filter(|table| !existing_tables.iter().any(|name| name == table))
        .collect();
    checks.push(IntegrityCheck {
        id: "required_tables".to_owned(),
        label: "必要数据表".to_owned(),
        status: if missing_tables.is_empty() {
            IntegrityStatus::Pass
        } else {
            IntegrityStatus::Fail
        },
        detail: Some(format!("缺失数据表 {} 项", missing_tables.len())),
    });

    if !integrity_ok || !missing_tables.is_empty() {
        return Ok(DataIntegrityReport {
            checked_at: now_utc_rfc3339(),
            ok: false,
            schema_version,
            expected_schema_version,
            integrity_message,
            counts: DataCounts::default(),
            checks,
        });
    }

    let push_count_check = |checks: &mut Vec<IntegrityCheck>, id: &str, label: &str, sql: &str| {
        let count: i64 = conn.query_row(sql, [], |row| row.get(0))?;
        checks.push(IntegrityCheck {
            id: id.to_owned(),
            label: label.to_owned(),
            status: if count == 0 {
                IntegrityStatus::Pass
            } else {
                IntegrityStatus::Fail
            },
            detail: Some(format!("发现 {count} 项不一致")),
        });
        Ok::<(), CoreError>(())
    };

    push_count_check(
        &mut checks,
        "task_ownership",
        "任务项目归属",
        "SELECT COUNT(*) FROM tasks t LEFT JOIN projects p ON p.id = t.project_id \
         WHERE p.id IS NULL",
    )?;
    push_count_check(
        &mut checks,
        "criterion_ownership",
        "验收条件任务归属",
        "SELECT COUNT(*) FROM acceptance_criteria c LEFT JOIN tasks t ON t.id = c.task_id \
         WHERE t.id IS NULL",
    )?;
    push_count_check(
        &mut checks,
        "verification_relationships",
        "验证运行关系",
        "SELECT COUNT(*) FROM verification_runs r \
         LEFT JOIN tasks t ON t.id = r.task_id \
         LEFT JOIN acceptance_criteria c ON c.id = r.criterion_id \
         WHERE t.id IS NULL OR (r.criterion_id IS NOT NULL AND c.id IS NULL) \
         OR (c.id IS NOT NULL AND c.task_id <> r.task_id)",
    )?;
    push_count_check(
        &mut checks,
        "verification_check_relationships",
        "验收检查关系",
        "SELECT COUNT(*) FROM verification_checks v \
         LEFT JOIN verification_runs r ON r.id = v.run_id \
         LEFT JOIN tasks t ON t.id = v.task_id \
         LEFT JOIN acceptance_criteria c ON c.id = v.criterion_id \
         LEFT JOIN evidence e ON e.id = v.evidence_id \
         WHERE r.id IS NULL OR t.id IS NULL \
         OR (v.criterion_id IS NOT NULL AND (c.id IS NULL OR c.task_id <> v.task_id)) \
         OR (v.evidence_id IS NOT NULL AND (e.id IS NULL OR e.task_id <> v.task_id))",
    )?;
    push_count_check(
        &mut checks,
        "evidence_relationships",
        "证据关系",
        "SELECT COUNT(*) FROM evidence e \
         LEFT JOIN tasks t ON t.id = e.task_id \
         LEFT JOIN verification_runs r ON r.id = e.run_id \
         WHERE t.id IS NULL OR (e.run_id IS NOT NULL AND (r.id IS NULL OR r.task_id <> e.task_id))",
    )?;
    push_count_check(
        &mut checks,
        "claude_relationships",
        "Claude 事件归属",
        "SELECT COUNT(*) FROM claude_tool_calls tc \
         LEFT JOIN claude_sessions s ON s.id = tc.session_id \
         LEFT JOIN projects p ON p.id = tc.project_id \
         WHERE s.id IS NULL OR p.id IS NULL OR s.project_id <> tc.project_id",
    )?;
    push_count_check(
        &mut checks,
        "claude_event_relationships",
        "Claude 事件会话归属",
        "SELECT COUNT(*) FROM claude_events e \
         LEFT JOIN claude_sessions s ON s.id = e.session_id \
         LEFT JOIN projects p ON p.id = e.project_id \
         WHERE s.id IS NULL OR p.id IS NULL OR s.project_id <> e.project_id",
    )?;
    push_count_check(
        &mut checks,
        "memory_relationships",
        "项目记忆归属",
        "SELECT COUNT(*) FROM project_memories m \
         LEFT JOIN projects p ON p.id = m.project_id \
         LEFT JOIN tasks t ON t.id = m.linked_task_id \
         LEFT JOIN evidence e ON e.id = m.linked_evidence_id \
         LEFT JOIN tasks et ON et.id = e.task_id \
         WHERE p.id IS NULL \
         OR (m.linked_task_id IS NOT NULL AND (t.id IS NULL OR t.project_id <> m.project_id)) \
         OR (m.linked_evidence_id IS NOT NULL AND (e.id IS NULL OR et.project_id <> m.project_id))",
    )?;
    push_count_check(
        &mut checks,
        "memory_revisions",
        "记忆修订历史",
        "SELECT COUNT(*) FROM project_memories m WHERE m.revision <> \
         (SELECT COALESCE(MAX(r.revision), 0) FROM project_memory_revisions r \
          WHERE r.memory_id = m.id)",
    )?;
    push_count_check(
        &mut checks,
        "decision_relationships",
        "开发决策归属",
        "SELECT COUNT(*) FROM decisions d \
         LEFT JOIN projects p ON p.id = d.project_id \
         LEFT JOIN tasks t ON t.id = d.task_id \
         WHERE p.id IS NULL OR (d.task_id IS NOT NULL AND (t.id IS NULL OR t.project_id <> d.project_id))",
    )?;
    push_count_check(
        &mut checks,
        "decision_revisions",
        "决策修订历史",
        "SELECT COUNT(*) FROM decisions d WHERE d.revision <> \
         (SELECT COALESCE(MAX(r.revision), 0) FROM decision_revisions r \
          WHERE r.decision_id = d.id)",
    )?;
    push_count_check(
        &mut checks,
        "decision_supersede_links",
        "决策取代关系",
        "SELECT COUNT(*) FROM decisions d \
         LEFT JOIN decisions old ON old.id = d.supersedes_id \
         LEFT JOIN decisions replacement ON replacement.id = d.superseded_by_id \
         WHERE (d.supersedes_id IS NOT NULL AND old.id IS NULL) \
         OR (d.superseded_by_id IS NOT NULL AND replacement.id IS NULL) \
         OR (d.status = 'draft' AND d.supersedes_id IS NOT NULL \
             AND (old.status <> 'confirmed' OR old.superseded_by_id IS NOT NULL)) \
         OR (d.status IN ('confirmed', 'deprecated') AND d.supersedes_id IS NOT NULL \
             AND (old.status <> 'superseded' OR old.superseded_by_id <> d.id)) \
         OR (d.status = 'superseded' AND \
             (replacement.supersedes_id <> d.id OR replacement.status NOT IN ('confirmed', 'deprecated')))",
    )?;
    push_count_check(
        &mut checks,
        "recovery_snapshot_relationships",
        "恢复快照归属",
        "SELECT COUNT(*) FROM recovery_snapshots s \
         LEFT JOIN projects p ON p.id = s.project_id WHERE p.id IS NULL",
    )?;
    push_count_check(
        &mut checks,
        "recovery_preference_relationships",
        "恢复下一步归属",
        "SELECT COUNT(*) FROM project_recovery_preferences pref \
         LEFT JOIN projects p ON p.id = pref.project_id \
         LEFT JOIN tasks t ON t.id = pref.next_task_id \
         WHERE p.id IS NULL OR (pref.next_task_id IS NOT NULL AND \
         (t.id IS NULL OR t.project_id <> pref.project_id))",
    )?;

    let counts = DataCounts {
        projects: count(conn, "projects")?,
        tasks: count(conn, "tasks")?,
        acceptance_criteria: count(conn, "acceptance_criteria")?,
        verification_runs: count(conn, "verification_runs")?,
        verification_checks: count(conn, "verification_checks")?,
        evidence: count(conn, "evidence")?,
        claude_sessions: count(conn, "claude_sessions")?,
        claude_events: count(conn, "claude_events")?,
        project_memories: count(conn, "project_memories")?,
        project_memory_revisions: count(conn, "project_memory_revisions")?,
        decisions: count(conn, "decisions")?,
        decision_revisions: count(conn, "decision_revisions")?,
        recovery_snapshots: count(conn, "recovery_snapshots")?,
        backup_records: count(conn, "backup_records")?,
    };

    let ok = checks
        .iter()
        .all(|check| check.status == IntegrityStatus::Pass);

    Ok(DataIntegrityReport {
        checked_at: now_utc_rfc3339(),
        ok,
        schema_version,
        expected_schema_version,
        integrity_message,
        counts,
        checks,
    })
}

pub(crate) fn count(conn: &Connection, table: &str) -> Result<i64, CoreError> {
    // table 来自本模块的常量或固定清单，不接受外部输入。
    let sql = format!("SELECT COUNT(*) FROM {table}");
    Ok(conn.query_row(&sql, [], |row| row.get(0))?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn temp_database() -> (TempDir, Database) {
        let directory = TempDir::new().expect("temp dir");
        let database = Database::open(directory.path().join("xushu.db")).expect("open");
        (directory, database)
    }

    #[test]
    fn fresh_database_passes_all_checks() {
        let (_directory, database) = temp_database();
        let report = DataIntegrityService::new(&database).check().expect("check");
        assert!(report.ok);
        assert_eq!(report.schema_version, 7);
        assert!(report
            .checks
            .iter()
            .all(|check| check.status == IntegrityStatus::Pass));
    }

    #[test]
    fn revision_history_gap_is_reported() {
        let (_directory, database) = temp_database();
        let database_path = database.path().to_path_buf();
        {
            let conn = rusqlite::Connection::open(&database_path).expect("raw connection");
            conn.execute_batch(
                "PRAGMA foreign_keys = OFF;
                 INSERT INTO projects (id, name, root_path, root_path_key, created_at, updated_at)
                 VALUES ('p1', 'P', 'C:\\p', 'c:\\p', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z');
                 INSERT INTO tasks (id, project_id, title, description, priority, status, revision, created_at, updated_at)
                 VALUES ('t1', 'p1', 'T', '', 'medium', 'draft', 1, '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z');
                 INSERT INTO project_memories (
                    id, project_id, category, title, content, source_type, confirmation_status,
                    validity_status, revision, created_at, updated_at, confirmed_at
                 ) VALUES ('m1', 'p1', 'fact', 'M', 'C', 'user', 'confirmed', 'active', 2,
                    '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z');
                 INSERT INTO project_memory_revisions (
                    id, memory_id, revision, category, title, content, source_type,
                    confirmation_status, validity_status, changed_at
                 ) VALUES ('r1', 'm1', 1, 'fact', 'M', 'C', 'user', 'confirmed', 'active',
                    '2026-01-01T00:00:00Z');",
            )
            .expect("insert inconsistent memory");
        }
        let report = DataIntegrityService::new(&database).check().expect("check");
        assert!(!report.ok);
        let check = report
            .checks
            .iter()
            .find(|check| check.id == "memory_revisions")
            .expect("memory check");
        assert_eq!(check.status, IntegrityStatus::Fail);
    }
}
