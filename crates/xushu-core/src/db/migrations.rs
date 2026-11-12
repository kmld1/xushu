use rusqlite::{params, Connection};
use serde::Serialize;

use super::now_utc_rfc3339;
use crate::error::CoreError;

/// 一次 schema 迁移。
#[derive(Debug, Clone, Copy)]
pub struct Migration {
    pub version: i64,
    pub name: &'static str,
    pub sql: &'static str,
}

/// 当前应用内置的迁移列表。只追加，不修改已发布迁移。
pub const MIGRATIONS: &[Migration] = &[
    Migration {
        version: 1,
        name: "0001_create_persistence_probes",
        sql: include_str!("../../migrations/0001_create_persistence_probes.sql"),
    },
    Migration {
        version: 2,
        name: "0002_projects_tasks",
        sql: include_str!("../../migrations/0002_projects_tasks.sql"),
    },
    Migration {
        version: 3,
        name: "0003_verification_engine",
        sql: include_str!("../../migrations/0003_verification_engine.sql"),
    },
    Migration {
        version: 4,
        name: "0004_claude_hooks",
        sql: include_str!("../../migrations/0004_claude_hooks.sql"),
    },
    Migration {
        version: 5,
        name: "0005_claude_capture_boundary",
        sql: include_str!("../../migrations/0005_claude_capture_boundary.sql"),
    },
];

/// 一次迁移执行的结果。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MigrationReport {
    pub applied: Vec<i64>,
    pub current_version: i64,
}

/// 按版本顺序执行尚未应用的迁移。
///
/// 每个迁移在独立事务中执行：DDL、版本记录与 `PRAGMA user_version` 一起提交；
/// 任意语句失败都会回滚该迁移，不留下半完成结构。
pub fn run_migrations(
    conn: &mut Connection,
    migrations: &[Migration],
) -> Result<MigrationReport, CoreError> {
    ensure_migrations_table(conn)?;

    if migrations.iter().any(|migration| migration.version <= 0) {
        return Err(CoreError::InvalidMigration(
            "迁移版本必须为正整数".to_owned(),
        ));
    }
    if migrations
        .iter()
        .any(|migration| migration.name.trim().is_empty())
    {
        return Err(CoreError::InvalidMigration("迁移名称不能为空".to_owned()));
    }

    let mut ordered: Vec<&Migration> = migrations.iter().collect();
    ordered.sort_by_key(|migration| migration.version);

    if ordered
        .windows(2)
        .any(|pair| pair[0].version == pair[1].version)
    {
        return Err(CoreError::InvalidMigration("迁移版本不能重复".to_owned()));
    }

    let applied_versions = load_applied_versions(conn)?;
    let supported_version = ordered
        .last()
        .map(|migration| migration.version)
        .unwrap_or(0);

    if let Some(found) = applied_versions.iter().copied().max() {
        if found > supported_version {
            return Err(CoreError::SchemaTooNew {
                found,
                supported: supported_version,
            });
        }
    }

    let mut newly_applied = Vec::new();

    for migration in ordered {
        if applied_versions.contains(&migration.version) {
            continue;
        }

        let tx = conn.transaction()?;
        tx.execute_batch(migration.sql)?;
        tx.execute(
            "INSERT INTO schema_migrations (version, name, applied_at) VALUES (?1, ?2, ?3)",
            params![migration.version, migration.name, now_utc_rfc3339()],
        )?;
        tx.pragma_update(None, "user_version", migration.version)?;
        tx.commit()?;
        newly_applied.push(migration.version);
    }

    Ok(MigrationReport {
        applied: newly_applied,
        current_version: current_schema_version(conn)?,
    })
}

fn ensure_migrations_table(conn: &Connection) -> Result<(), CoreError> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_migrations (
            version INTEGER PRIMARY KEY NOT NULL,
            name TEXT NOT NULL,
            applied_at TEXT NOT NULL
        );",
    )?;
    Ok(())
}

fn load_applied_versions(conn: &Connection) -> Result<Vec<i64>, CoreError> {
    let mut statement = conn.prepare("SELECT version FROM schema_migrations ORDER BY version")?;
    let rows = statement.query_map([], |row| row.get::<_, i64>(0))?;
    let versions = rows.collect::<Result<Vec<_>, _>>()?;
    Ok(versions)
}

fn current_schema_version(conn: &Connection) -> Result<i64, CoreError> {
    let version = conn.query_row(
        "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
        [],
        |row| row.get(0),
    )?;
    Ok(version)
}
