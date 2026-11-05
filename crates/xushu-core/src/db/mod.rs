mod migrations;

use std::fs;
use std::path::Path;
use std::sync::{Mutex, MutexGuard};
use std::time::Duration;

use chrono::{SecondsFormat, Utc};
use rusqlite::Connection;
use serde::Serialize;

use crate::error::CoreError;

pub use migrations::{run_migrations, Migration, MigrationReport, MIGRATIONS};

/// 本地 SQLite 数据库。
///
/// 连接由 `Mutex` 串行化，避免把非 `Sync` 的 `rusqlite::Connection` 直接暴露给
/// 多线程运行时。数据库文件路径只在 Rust 内部使用，不通过 IPC 返回。
pub struct Database {
    conn: Mutex<Connection>,
    file_name: String,
}

impl Database {
    /// 打开数据库、启用安全 pragma 并执行内置迁移。
    pub fn open(path: impl AsRef<Path>) -> Result<Self, CoreError> {
        let path = path.as_ref();

        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)?;
            }
        }

        let conn = Connection::open(path)?;
        Self::configure(&conn)?;

        let file_name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("xushu.db")
            .to_owned();

        let database = Self {
            conn: Mutex::new(conn),
            file_name,
        };
        database.migrate()?;
        Ok(database)
    }

    fn configure(conn: &Connection) -> Result<(), CoreError> {
        conn.busy_timeout(Duration::from_secs(5))?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;

        let journal_mode: String =
            conn.query_row("PRAGMA journal_mode = WAL", [], |row| row.get(0))?;

        if !journal_mode.eq_ignore_ascii_case("wal") {
            return Err(CoreError::Validation(format!(
                "无法启用 SQLite WAL，当前 journal_mode={journal_mode}"
            )));
        }

        Ok(())
    }

    /// 幂等执行内置迁移。
    pub fn migrate(&self) -> Result<MigrationReport, CoreError> {
        let mut conn = self.lock()?;
        run_migrations(&mut conn, MIGRATIONS)
    }

    /// 运行 SQLite 完整性检查并返回可展示的健康状态。
    pub fn health(&self) -> Result<DatabaseHealth, CoreError> {
        let conn = self.lock()?;

        let integrity: String = conn.query_row("PRAGMA integrity_check", [], |row| row.get(0))?;

        let mut foreign_key_statement = conn.prepare("PRAGMA foreign_key_check")?;
        let mut foreign_key_rows = foreign_key_statement.query([])?;
        let foreign_key_violations = foreign_key_rows.next()?.is_some();

        let foreign_keys_enabled: i64 =
            conn.query_row("PRAGMA foreign_keys", [], |row| row.get(0))?;
        let journal_mode: String = conn.query_row("PRAGMA journal_mode", [], |row| row.get(0))?;
        let schema_version: i64 = conn.query_row(
            "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
            [],
            |row| row.get(0),
        )?;
        let record_count: i64 =
            conn.query_row("SELECT COUNT(*) FROM persistence_probes", [], |row| {
                row.get(0)
            })?;

        let integrity_ok = integrity.eq_ignore_ascii_case("ok");
        let foreign_keys_enabled = foreign_keys_enabled == 1;

        Ok(DatabaseHealth {
            ok: integrity_ok && foreign_keys_enabled && !foreign_key_violations,
            integrity_ok,
            foreign_keys_enabled,
            foreign_key_violations,
            journal_mode,
            schema_version,
            record_count,
            database_file: self.file_name.clone(),
        })
    }

    /// 在事务中执行操作；闭包返回错误时不提交，由 SQLite 回滚。
    pub fn with_transaction<T>(
        &self,
        operation: impl FnOnce(&rusqlite::Transaction<'_>) -> Result<T, CoreError>,
    ) -> Result<T, CoreError> {
        let mut conn = self.lock()?;
        let transaction = conn.transaction()?;
        let value = operation(&transaction)?;
        transaction.commit()?;
        Ok(value)
    }

    pub(crate) fn lock(&self) -> Result<MutexGuard<'_, Connection>, CoreError> {
        self.conn.lock().map_err(|_| CoreError::LockPoisoned)
    }
}

pub(crate) fn now_utc_rfc3339() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
}

/// 数据库健康状态；字段不包含绝对路径或连接字符串。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DatabaseHealth {
    pub ok: bool,
    pub integrity_ok: bool,
    pub foreign_keys_enabled: bool,
    pub foreign_key_violations: bool,
    pub journal_mode: String,
    pub schema_version: i64,
    pub record_count: i64,
    pub database_file: String,
}
