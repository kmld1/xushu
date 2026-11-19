mod migrations;

#[cfg(windows)]
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, MutexGuard};
use std::time::{Duration, Instant};

use chrono::{SecondsFormat, Utc};
use rusqlite::backup::{Backup, StepResult};
use rusqlite::{config::DbConfig, Connection, OpenFlags};
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
    path: PathBuf,
}

impl Database {
    /// 打开数据库、启用安全 pragma 并执行内置迁移。
    pub fn open(path: impl AsRef<Path>) -> Result<Self, CoreError> {
        let path = path.as_ref();

        #[cfg(unix)]
        let conn = crate::backup::open_checked_sqlite(
            path,
            OpenFlags::default(),
            crate::backup::SqliteAccess::OpenOrCreate,
        )?;
        #[cfg(windows)]
        let conn = {
            if let Some(parent) = path.parent() {
                if !parent.as_os_str().is_empty() {
                    fs::create_dir_all(parent)?;
                }
            }
            Connection::open(path)?
        };
        Self::configure(&conn)?;
        Self::from_configured_connection(path, conn)
    }

    pub(crate) fn open_existing(path: &Path) -> Result<Self, CoreError> {
        let flags = OpenFlags::default() & !OpenFlags::SQLITE_OPEN_CREATE;
        #[cfg(unix)]
        let conn =
            crate::backup::open_checked_sqlite(path, flags, crate::backup::SqliteAccess::Existing)?;
        #[cfg(windows)]
        let conn = Connection::open_with_flags(path, flags)?;
        Self::configure(&conn)?;
        Self::from_configured_connection(path, conn)
    }

    pub(crate) fn from_configured_connection(
        path: &Path,
        conn: Connection,
    ) -> Result<Self, CoreError> {
        let file_name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("xushu.db")
            .to_owned();

        let database = Self {
            conn: Mutex::new(conn),
            file_name,
            path: path.to_path_buf(),
        };
        database.migrate()?;
        Ok(database)
    }

    pub(crate) fn configure(conn: &Connection) -> Result<(), CoreError> {
        conn.busy_timeout(Duration::from_secs(5))?;
        conn.set_db_config(DbConfig::SQLITE_DBCONFIG_TRUSTED_SCHEMA, false)?;
        conn.set_db_config(DbConfig::SQLITE_DBCONFIG_DEFENSIVE, true)?;
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

    /// 数据库主文件路径。仅 Rust 内部使用，不直接通过 IPC 暴露。
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// 将 WAL 内容合并回主数据库，用于受控关闭或文件切换前的准备。
    pub fn checkpoint(&self) -> Result<(), CoreError> {
        let conn = self.lock()?;
        let busy: i64 = conn.query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |row| row.get(0))?;
        if busy != 0 {
            return Err(CoreError::Conflict(
                "数据库 WAL 仍被其他连接占用".to_owned(),
            ));
        }
        Ok(())
    }

    /// Detect writes on this connection and commits made by other connections.
    pub(crate) fn change_stamp(&self) -> Result<(u64, i64), CoreError> {
        let conn = self.lock()?;
        let external_version = conn.query_row("PRAGMA data_version", [], |row| row.get(0))?;
        Ok((conn.total_changes(), external_version))
    }

    /// 将数据库整理为单一自包含文件（WAL 合并、切回 DELETE 日志模式）。
    ///
    /// 只用于隔离副本或备份快照，不用于正在被应用使用的正式数据库；正式数据库
    /// 保持 WAL 以支持并发读取。
    pub fn compact_for_backup(&self) -> Result<(), CoreError> {
        let conn = self.lock()?;
        conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")?;
        let journal_mode: String =
            conn.query_row("PRAGMA journal_mode = DELETE", [], |row| row.get(0))?;
        if !journal_mode.eq_ignore_ascii_case("delete") {
            return Err(CoreError::Backup(format!(
                "无法将备份副本切换为单文件模式，当前 journal_mode={journal_mode}"
            )));
        }
        Ok(())
    }

    /// 使用 SQLite Online Backup API 生成一致性快照。
    ///
    /// `destination` 必须是不存在的路径；调用方应写入临时目录，校验成功后再
    /// 原子改名。备份在源数据库运行期间可安全执行，不能通过复制 `.db` 文件替代。
    pub fn backup_to(&self, destination: &Path) -> Result<(), CoreError> {
        self.backup_to_with_cancel(destination, &AtomicBool::new(false))
    }

    /// 可取消的一致性快照。取消时返回 `BackupCancelled`，不保证目标文件有效，
    /// 调用方必须删除临时目录。
    pub fn backup_to_with_cancel(
        &self,
        destination: &Path,
        cancelled: &AtomicBool,
    ) -> Result<(), CoreError> {
        #[cfg(windows)]
        if destination.exists() {
            return Err(CoreError::Conflict("备份目标文件已存在".to_owned()));
        }

        let conn = self.lock()?;
        #[cfg(windows)]
        let mut destination_connection = Connection::open(destination)?;
        #[cfg(unix)]
        let mut destination_connection = crate::backup::open_checked_sqlite(
            destination,
            OpenFlags::default(),
            crate::backup::SqliteAccess::CreateNew,
        )?;
        destination_connection.busy_timeout(Duration::from_secs(5))?;
        destination_connection.pragma_update(None, "synchronous", "FULL")?;

        {
            let backup = Backup::new(&conn, &mut destination_connection)?;
            let mut lock_started: Option<Instant> = None;
            loop {
                if cancelled.load(Ordering::Relaxed) {
                    return Err(CoreError::BackupCancelled);
                }
                match backup.step(128)? {
                    StepResult::Done => break,
                    StepResult::More => {
                        lock_started = None;
                    }
                    StepResult::Busy | StepResult::Locked => {
                        let started = *lock_started.get_or_insert_with(Instant::now);
                        if started.elapsed() >= Duration::from_secs(30) {
                            return Err(CoreError::Backup(
                                "数据库持续被其他连接占用，无法完成一致性备份".to_owned(),
                            ));
                        }
                        std::thread::sleep(Duration::from_millis(10));
                    }
                    _ => {}
                }
            }
        }

        destination_connection.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")?;
        destination_connection.cache_flush()?;
        drop(destination_connection);
        Ok(())
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
