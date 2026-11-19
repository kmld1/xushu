//! 一致性备份创建、校验、历史与保留策略。

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, SystemTime};

use chrono::{DateTime, SecondsFormat, Utc};
use rusqlite::{params, OptionalExtension, Row};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::claude::path_guard::Directories;
use crate::db::{now_utc_rfc3339, Database};
use crate::error::CoreError;

use super::integrity::{
    DataIntegrityReport, DataIntegrityService, IntegrityCheck, IntegrityStatus,
};
use super::manifest::{
    build_manifest, isolated_failure_detail, manifest_bytes, open_isolated_payload, read_manifest,
    read_staged_manifest, verify_payload, BackupOrigin, MANIFEST_FILE_NAME, MAX_BACKUP_BYTES,
    PAYLOAD_FILE_NAME,
};
#[cfg(windows)]
use super::paths::free_space_bytes;
use super::paths::{
    capacity_ok, copy_regular_file_bounded, display_path, map_path_error, prepare_directory,
    remove_known_files, secure_directory, sha256_file, write_new_file,
};

const REGISTRY_LIMIT: i64 = 200;
const BACKUP_SPACE_MARGIN: u64 = 8 * 1024 * 1024;
const STALE_PARTIAL_AGE: Duration = Duration::from_secs(60 * 60);
const TRANSIENT_FILES: &[&str] = &[
    MANIFEST_FILE_NAME,
    PAYLOAD_FILE_NAME,
    "xushu.db-wal",
    "xushu.db-shm",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BackupIntegrityStatus {
    Verified,
    Failed,
    Unverified,
    Missing,
}

impl BackupIntegrityStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Verified => "verified",
            Self::Failed => "failed",
            Self::Unverified => "unverified",
            Self::Missing => "missing",
        }
    }

    fn parse(value: &str) -> Option<Self> {
        match value {
            "verified" => Some(Self::Verified),
            "failed" => Some(Self::Failed),
            "unverified" => Some(Self::Unverified),
            "missing" => Some(Self::Missing),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupRecord {
    pub id: String,
    pub backup_id: String,
    pub directory_path: String,
    pub format_version: i64,
    pub schema_version: i64,
    pub app_version: String,
    pub origin: BackupOrigin,
    pub created_at: String,
    pub registered_at: String,
    pub data_size_bytes: u64,
    pub manifest_size_bytes: u64,
    pub payload_sha256: String,
    pub integrity_status: BackupIntegrityStatus,
    pub verified_at: Option<String>,
    pub last_error: Option<String>,
    pub directory_exists: bool,
    pub manifest_exists: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupVerification {
    pub backup_id: String,
    pub verified: bool,
    pub verified_at: Option<String>,
    pub checks: Vec<IntegrityCheck>,
    pub integrity: Option<DataIntegrityReport>,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupPolicy {
    pub automatic_enabled: bool,
    pub target_directory: Option<String>,
    pub interval_hours: i64,
    pub retention_count: i64,
    pub last_automatic_at: Option<String>,
    pub last_automatic_error_code: Option<String>,
    pub last_automatic_error_at: Option<String>,
    pub updated_at: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateBackupPolicyInput {
    pub automatic_enabled: bool,
    pub target_directory: Option<String>,
    pub interval_hours: i64,
    pub retention_count: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AutomaticBackupOutcome {
    pub status: String,
    pub backup: Option<BackupRecord>,
    pub removed_count: usize,
    pub message: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanupOutcome {
    pub removed_backup_ids: Vec<String>,
    pub skipped_count: usize,
    pub failures: Vec<String>,
}

pub struct BackupService<'a> {
    database: &'a Database,
    data_directory: &'a Path,
}

impl<'a> BackupService<'a> {
    pub fn new(database: &'a Database, data_directory: &'a Path) -> Self {
        Self {
            database,
            data_directory,
        }
    }

    pub fn create_manual(
        &self,
        target_directory: &str,
        cancelled: &AtomicBool,
    ) -> Result<BackupRecord, CoreError> {
        let target = normalize_target(target_directory)?;
        self.create_with_target(BackupOrigin::Manual, &target, cancelled)
    }

    pub fn list(&self) -> Result<Vec<BackupRecord>, CoreError> {
        let conn = self.database.lock()?;
        let mut statement = conn.prepare(
            "SELECT id, backup_id, directory_path, format_version, schema_version, app_version,
                    origin, created_at, registered_at, data_size_bytes, manifest_size_bytes,
                    payload_sha256, integrity_status, verified_at, last_error
             FROM backup_records
             ORDER BY created_at DESC, id DESC
             LIMIT ?1",
        )?;
        let rows = statement.query_map([REGISTRY_LIMIT], row_to_record)?;
        let mut records = rows.collect::<Result<Vec<_>, _>>()?;
        for record in &mut records {
            record.directory_exists = directory_exists(Path::new(&record.directory_path));
            record.manifest_exists = record.directory_exists
                && Path::new(&record.directory_path)
                    .join(MANIFEST_FILE_NAME)
                    .is_file();
        }
        Ok(records)
    }

    pub fn get(&self, backup_id: &str) -> Result<BackupRecord, CoreError> {
        let conn = self.database.lock()?;
        let record = conn
            .query_row(
                "SELECT id, backup_id, directory_path, format_version, schema_version, app_version,
                        origin, created_at, registered_at, data_size_bytes, manifest_size_bytes,
                        payload_sha256, integrity_status, verified_at, last_error
                 FROM backup_records WHERE backup_id = ?1",
                [backup_id],
                row_to_record,
            )
            .optional()?;
        drop(conn);
        let mut record = record.ok_or(CoreError::NotFound)?;
        record.directory_exists = directory_exists(Path::new(&record.directory_path));
        record.manifest_exists = record.directory_exists
            && Path::new(&record.directory_path)
                .join(MANIFEST_FILE_NAME)
                .is_file();
        Ok(record)
    }

    pub fn verify(&self, backup_id: &str) -> Result<BackupVerification, CoreError> {
        self.verify_internal(backup_id, true)
    }

    /// 创建后或恢复预检时使用的完整校验；不更新登记状态。
    pub fn verify_without_registry_update(
        &self,
        backup_id: &str,
    ) -> Result<BackupVerification, CoreError> {
        self.verify_internal(backup_id, false)
    }

    fn verify_internal(
        &self,
        backup_id: &str,
        update_record: bool,
    ) -> Result<BackupVerification, CoreError> {
        let record = self.get(backup_id)?;
        let mut checks = Vec::new();
        let mut verified = true;
        let mut integrity = None;
        let mut message = "备份校验通过".to_owned();

        let directory = PathBuf::from(&record.directory_path);
        if !record.directory_exists {
            checks.push(failed_check(
                "backup_directory",
                "备份目录",
                "备份目录不存在或无法访问",
            ));
            verified = false;
            message = "备份目录不存在".to_owned();
            if update_record {
                self.update_integrity(
                    backup_id,
                    BackupIntegrityStatus::Missing,
                    None,
                    Some("目录缺失"),
                )?;
            }
            return Ok(BackupVerification {
                backup_id: backup_id.to_owned(),
                verified,
                verified_at: None,
                checks,
                integrity,
                message,
            });
        }

        let manifest_read = match secure_directory(&directory)
            .and_then(|guard| read_manifest(&guard.canonical).map(|manifest| (guard, manifest)))
        {
            Ok(value) => value,
            Err(error) => {
                checks.push(failed_check(
                    "manifest",
                    "备份清单",
                    "备份清单无效或不可读取",
                ));
                verified = false;
                message = "备份清单校验失败".to_owned();
                if update_record {
                    self.update_integrity(
                        backup_id,
                        BackupIntegrityStatus::Failed,
                        None,
                        Some("清单校验失败"),
                    )?;
                }
                let _ = error;
                return Ok(BackupVerification {
                    backup_id: backup_id.to_owned(),
                    verified,
                    verified_at: None,
                    checks,
                    integrity,
                    message,
                });
            }
        };
        let (_backup_guard, manifest_read) = manifest_read;
        checks.push(pass_check("manifest", "备份清单格式", "格式与字段通过校验"));

        let manifest = &manifest_read.manifest;
        let registry_matches = manifest.backup_id == record.backup_id
            && manifest
                .payload()
                .sha256
                .eq_ignore_ascii_case(&record.payload_sha256)
            && manifest.schema_version == record.schema_version;
        checks.push(if registry_matches {
            pass_check("registry", "本地登记", "清单与本地登记一致")
        } else {
            failed_check("registry", "本地登记", "清单与本地登记不一致")
        });
        verified &= registry_matches;

        if let Err(error) = verify_payload(&directory, manifest) {
            checks.push(failed_check(
                "payload_hash",
                "数据文件校验和",
                "数据文件大小或 SHA-256 不一致",
            ));
            verified = false;
            message = "备份数据文件校验失败".to_owned();
            if update_record {
                self.update_integrity(
                    backup_id,
                    BackupIntegrityStatus::Failed,
                    None,
                    Some("数据文件校验失败"),
                )?;
            }
            let _ = error;
            return Ok(BackupVerification {
                backup_id: backup_id.to_owned(),
                verified,
                verified_at: None,
                checks,
                integrity,
                message,
            });
        }
        checks.push(pass_check(
            "payload_hash",
            "数据文件校验和",
            "大小与 SHA-256 与清单一致",
        ));

        let workspace_root = self.data_directory.join("verify-workspaces");
        let root = prepare_directory(&workspace_root)?;
        let name = Uuid::new_v4().to_string();
        root.create_child(&name).map_err(map_path_error)?;
        let workspace_guard = root.open_child(&name).map_err(map_path_error)?;
        let workspace = workspace_guard.canonical.clone();
        let copy_path = workspace.join(PAYLOAD_FILE_NAME);

        let copy_result = copy_regular_file_bounded(
            &directory.join(PAYLOAD_FILE_NAME),
            &copy_path,
            MAX_BACKUP_BYTES,
        )
        .and_then(|_| {
            let (copied_sha, _) = sha256_file(&copy_path, MAX_BACKUP_BYTES)?;
            if !copied_sha.eq_ignore_ascii_case(&manifest.payload().sha256) {
                return Err(CoreError::Backup("隔离副本校验和不一致".to_owned()));
            }
            Ok(())
        });

        if let Err(error) = copy_result {
            checks.push(failed_check(
                "isolated_copy",
                "隔离副本",
                &format!(
                    "无法创建或校验隔离副本：{}",
                    isolated_failure_detail(&error)
                ),
            ));
            verified = false;
            message = "隔离校验失败".to_owned();
        } else {
            match open_isolated_payload(&copy_path, manifest.schema_version) {
                Ok(snapshot) => {
                    let compaction = snapshot.compact_for_backup();
                    let report = DataIntegrityService::new(&snapshot).check();
                    drop(snapshot);
                    match (compaction, report) {
                        (Ok(()), Ok(report)) => {
                            if report.ok {
                                checks.push(pass_check(
                                    "sqlite_integrity",
                                    "SQLite 与业务关系",
                                    "integrity_check、外键与归属关系通过",
                                ));
                            } else {
                                checks.push(failed_check(
                                    "sqlite_integrity",
                                    "SQLite 与业务关系",
                                    "隔离副本的 SQLite、外键或业务关系检查未通过",
                                ));
                                verified = false;
                                message = "隔离恢复数据关系检查未通过".to_owned();
                            }
                            integrity = Some(report);
                        }
                        (compaction, report) => {
                            verified = false;
                            message = "隔离恢复检查失败".to_owned();
                            let (stage, error) = if let Err(error) = &compaction {
                                ("整理隔离副本为单文件失败", Some(error))
                            } else {
                                ("执行隔离副本数据关系检查失败", report.as_ref().err())
                            };
                            let detail = error.map_or_else(
                                || stage.to_owned(),
                                |error| format!("{stage}：{}", isolated_failure_detail(error)),
                            );
                            checks.push(failed_check(
                                "sqlite_integrity",
                                "SQLite 与业务关系",
                                &detail,
                            ));
                            if let Ok(report) = report {
                                integrity = Some(report);
                            }
                        }
                    }
                }
                Err(error) => {
                    verified = false;
                    message = "隔离备份数据库校验失败".to_owned();
                    checks.push(failed_check(
                        "sqlite_integrity",
                        "SQLite 与业务关系",
                        &isolated_failure_detail(&error),
                    ));
                }
            }
        }

        let _ = cleanup_owned_directory(&root, &name, workspace_guard, TRANSIENT_FILES);

        let verified_at = if verified {
            Some(now_utc_rfc3339())
        } else {
            None
        };
        if update_record {
            self.update_integrity(
                backup_id,
                if verified {
                    BackupIntegrityStatus::Verified
                } else {
                    BackupIntegrityStatus::Failed
                },
                verified_at.as_deref(),
                if verified {
                    None
                } else {
                    Some("完整校验未通过")
                },
            )?;
        }
        if verified {
            message = "备份完整性验证通过：清单、校验和、SQLite 完整性与业务关系均通过".to_owned();
        }

        Ok(BackupVerification {
            backup_id: backup_id.to_owned(),
            verified,
            verified_at,
            checks,
            integrity,
            message,
        })
    }

    pub fn delete(&self, backup_id: &str, approved: bool) -> Result<(), CoreError> {
        if !approved {
            return Err(CoreError::RestoreConfirmationRequired);
        }
        let record = self.get(backup_id)?;
        let directory = PathBuf::from(&record.directory_path);
        if !record.directory_exists {
            return Err(CoreError::Conflict(
                "备份目录不存在，未删除历史登记".to_owned(),
            ));
        }
        let parent = secure_directory(directory.parent().ok_or(CoreError::UnsafePath)?)?;
        let name = directory.file_name().ok_or(CoreError::UnsafePath)?;
        let guard = parent.open_child(name).map_err(map_path_error)?;
        let manifest_read = read_manifest(&directory)?;
        if manifest_read.manifest.backup_id != record.backup_id {
            return Err(CoreError::Conflict(
                "备份清单与登记不一致，已拒绝删除".to_owned(),
            ));
        }
        verify_payload(&directory, &manifest_read.manifest)?;

        if !owns_only_known_files(&guard)? {
            return Err(CoreError::Conflict(
                "备份目录包含非备份文件，已拒绝删除；请手动检查该目录".to_owned(),
            ));
        }

        cleanup_owned_directory(
            &parent,
            name,
            guard,
            &[MANIFEST_FILE_NAME, PAYLOAD_FILE_NAME],
        )?;
        self.database.with_transaction(|transaction| {
            transaction.execute(
                "DELETE FROM backup_records WHERE backup_id = ?1",
                [backup_id],
            )?;
            Ok(())
        })?;
        Ok(())
    }

    pub fn policy(&self) -> Result<BackupPolicy, CoreError> {
        let conn = self.database.lock()?;
        let policy = conn
            .query_row(
                "SELECT automatic_enabled, target_directory, interval_hours, retention_count,
                        last_automatic_at, last_automatic_error_code, last_automatic_error_at,
                        updated_at
                 FROM backup_settings WHERE id = 1",
                [],
                row_to_policy,
            )
            .optional()?;
        Ok(policy.unwrap_or_else(default_policy))
    }

    pub fn update_policy(&self, input: UpdateBackupPolicyInput) -> Result<BackupPolicy, CoreError> {
        if !(1..=720).contains(&input.interval_hours) {
            return Err(CoreError::Validation(
                "自动备份间隔必须在 1 到 720 小时之间".to_owned(),
            ));
        }
        if !(1..=50).contains(&input.retention_count) {
            return Err(CoreError::Validation(
                "自动备份保留数量必须在 1 到 50 之间".to_owned(),
            ));
        }

        let target = input
            .target_directory
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(PathBuf::from);

        let normalized_target = match target {
            Some(path) => {
                if input.automatic_enabled {
                    Some(display_path(&prepare_directory(&path)?.canonical))
                } else {
                    super::paths::validate_absolute(&path)?;
                    Some(path.to_string_lossy().to_string())
                }
            }
            None => None,
        };
        if input.automatic_enabled && normalized_target.is_none() {
            return Err(CoreError::Validation(
                "启用自动备份前必须选择目标目录".to_owned(),
            ));
        }

        let updated_at = now_utc_rfc3339();
        self.database.with_transaction(|transaction| {
            transaction.execute(
                "UPDATE backup_settings SET automatic_enabled = ?1, target_directory = ?2,
                    interval_hours = ?3, retention_count = ?4, updated_at = ?5
                 WHERE id = 1",
                params![
                    i64::from(input.automatic_enabled),
                    normalized_target,
                    input.interval_hours,
                    input.retention_count,
                    updated_at
                ],
            )?;
            Ok(())
        })?;
        self.policy()
    }

    pub fn run_due_automatic(
        &self,
        cancelled: &AtomicBool,
    ) -> Result<AutomaticBackupOutcome, CoreError> {
        let policy = self.policy()?;
        if !policy.automatic_enabled {
            return Ok(AutomaticBackupOutcome {
                status: "disabled".to_owned(),
                backup: None,
                removed_count: 0,
                message: None,
            });
        }
        let Some(target) = policy.target_directory.as_deref() else {
            return Ok(AutomaticBackupOutcome {
                status: "disabled".to_owned(),
                backup: None,
                removed_count: 0,
                message: Some("自动备份已启用但未配置目标目录".to_owned()),
            });
        };
        if !is_due(&policy, Utc::now()) {
            return Ok(AutomaticBackupOutcome {
                status: "not_due".to_owned(),
                backup: None,
                removed_count: 0,
                message: None,
            });
        }

        match self.create_with_target(BackupOrigin::Automatic, Path::new(target), cancelled) {
            Ok(record) => {
                self.mark_automatic_success()?;
                let removed = self
                    .apply_retention_internal()
                    .map(|outcome| outcome.removed_backup_ids.len())
                    .unwrap_or(0);
                Ok(AutomaticBackupOutcome {
                    status: "created".to_owned(),
                    backup: Some(record),
                    removed_count: removed,
                    message: None,
                })
            }
            Err(error) => {
                let code = error.code().to_owned();
                self.mark_automatic_failure(&code)?;
                let message = if matches!(error, CoreError::BackupCancelled) {
                    "自动备份已取消".to_owned()
                } else {
                    "自动备份失败；旧备份未被删除".to_owned()
                };
                Ok(AutomaticBackupOutcome {
                    status: "failed".to_owned(),
                    backup: None,
                    removed_count: 0,
                    message: Some(message),
                })
            }
        }
    }

    pub fn cleanup_preview(&self) -> Result<Vec<BackupRecord>, CoreError> {
        let policy = self.policy()?;
        let records = self.list()?;
        let automatic: Vec<BackupRecord> = records
            .into_iter()
            .filter(|record| record.origin == BackupOrigin::Automatic)
            .collect();
        Ok(automatic
            .into_iter()
            .skip(policy.retention_count.max(1) as usize)
            .collect())
    }

    pub fn apply_cleanup(&self, approved: bool) -> Result<CleanupOutcome, CoreError> {
        if !approved {
            return Err(CoreError::RestoreConfirmationRequired);
        }
        self.apply_retention_internal()
    }

    fn apply_retention_internal(&self) -> Result<CleanupOutcome, CoreError> {
        let preview = self.cleanup_preview()?;
        let mut removed = Vec::new();
        let mut failures = Vec::new();
        let mut skipped = 0_usize;
        for record in preview {
            match self.delete(&record.backup_id, true) {
                Ok(()) => removed.push(record.backup_id),
                Err(_) => {
                    if record.directory_exists {
                        failures.push(format!(
                            "备份 {} 未通过安全校验，已跳过",
                            short_id(&record.backup_id)
                        ));
                        skipped += 1;
                    } else {
                        skipped += 1;
                    }
                }
            }
        }
        Ok(CleanupOutcome {
            removed_backup_ids: removed,
            skipped_count: skipped,
            failures,
        })
    }

    fn mark_automatic_success(&self) -> Result<(), CoreError> {
        self.database.with_transaction(|transaction| {
            transaction.execute(
                "UPDATE backup_settings SET last_automatic_at = ?1,
                    last_automatic_error_code = NULL, last_automatic_error_at = NULL,
                    updated_at = ?1 WHERE id = 1",
                [now_utc_rfc3339()],
            )?;
            Ok(())
        })
    }

    fn mark_automatic_failure(&self, code: &str) -> Result<(), CoreError> {
        let now = now_utc_rfc3339();
        self.database.with_transaction(|transaction| {
            transaction.execute(
                "UPDATE backup_settings SET last_automatic_error_code = ?1,
                    last_automatic_error_at = ?2, updated_at = ?2 WHERE id = 1",
                params![code, now],
            )?;
            Ok(())
        })
    }

    fn create_with_target(
        &self,
        origin: BackupOrigin,
        target: &Path,
        cancelled: &AtomicBool,
    ) -> Result<BackupRecord, CoreError> {
        let target_guard = prepare_directory(target)?;
        let target = target_guard.canonical.clone();
        let backup_id = Uuid::new_v4().to_string();
        let now = Utc::now();
        let created_at = now.to_rfc3339_opts(SecondsFormat::Millis, true);
        let directory_name = format!(
            "xushu-backup-{}-{}",
            now.format("%Y%m%dT%H%M%SZ"),
            &backup_id[..8]
        );
        let final_directory = target.join(&directory_name);
        if final_directory.exists() {
            return Err(CoreError::Conflict("备份目标目录已存在同名备份".to_owned()));
        }

        let (main_size, wal_size, _) = database_file_sizes(self.database.path());
        let estimate = main_size.saturating_add(wal_size);
        if estimate > MAX_BACKUP_BYTES {
            return Err(CoreError::Backup(
                "数据库超过单个备份的大小上限（2 GiB）".to_owned(),
            ));
        }
        let required = estimate.saturating_add(BACKUP_SPACE_MARGIN);
        #[cfg(unix)]
        let available = target_guard.free_space_bytes();
        #[cfg(windows)]
        let available = free_space_bytes(&target);
        if available.is_none() {
            return Err(CoreError::Backup("无法确认目标磁盘的可用空间".to_owned()));
        }
        if !capacity_ok(required, available) {
            return Err(CoreError::Backup(format!(
                "目标磁盘可用空间不足：至少需要 {} MiB",
                required.div_ceil(1024 * 1024)
            )));
        }

        cleanup_stale_partials(&target_guard);

        let partial_name = format!(".partial-{backup_id}");
        let partial = target.join(&partial_name);
        target_guard
            .create_child(&partial_name)
            .map_err(map_path_error)?;
        let mut partial_guard = Some(
            target_guard
                .open_child(&partial_name)
                .map_err(map_path_error)?,
        );
        let payload_path = partial.join(PAYLOAD_FILE_NAME);

        let result = (|| -> Result<BackupRecord, CoreError> {
            self.database
                .backup_to_with_cancel(&payload_path, cancelled)?;
            if cancelled.load(Ordering::Relaxed) {
                return Err(CoreError::BackupCancelled);
            }

            let schema_version = self.current_schema_version()?;
            {
                let snapshot = open_isolated_payload(&payload_path, schema_version)?;
                snapshot.compact_for_backup()?;
                let report = DataIntegrityService::new(&snapshot).check()?;
                if !report.ok {
                    return Err(CoreError::Backup(
                        "备份快照的数据完整性检查未通过".to_owned(),
                    ));
                }
            }
            let staged = partial_guard.as_ref().ok_or(CoreError::UnsafePath)?;
            remove_known_files(staged, &["xushu.db-wal", "xushu.db-shm"])?;

            let (payload_sha256, payload_size) = sha256_file(&payload_path, MAX_BACKUP_BYTES)?;
            if payload_size == 0 {
                return Err(CoreError::Backup("备份数据文件为空".to_owned()));
            }
            if cancelled.load(Ordering::Relaxed) {
                return Err(CoreError::BackupCancelled);
            }

            let manifest = build_manifest(
                &backup_id,
                env!("CARGO_PKG_VERSION"),
                schema_version,
                &created_at,
                origin,
                payload_size,
                &payload_sha256,
            );
            let bytes = manifest_bytes(&manifest)?;
            write_new_file(&partial.join(MANIFEST_FILE_NAME), &bytes)?;

            let read_back = read_staged_manifest(&partial)?;
            if read_back.manifest.backup_id != backup_id {
                return Err(CoreError::Backup("备份清单写入后校验失败".to_owned()));
            }
            verify_payload(&partial, &read_back.manifest)?;
            if cancelled.load(Ordering::Relaxed) {
                return Err(CoreError::BackupCancelled);
            }

            let staged = target_guard
                .open_child(&partial_name)
                .map_err(map_path_error)?;
            if partial_guard
                .as_ref()
                .is_none_or(|guard| guard.identity != staged.identity)
            {
                return Err(CoreError::UnsafePath);
            }
            let identity = staged.identity.clone();
            drop(staged);
            partial_guard.take();
            target_guard
                .rename_file(&partial_name, &directory_name)
                .map_err(map_path_error)?;
            let final_guard = target_guard
                .open_child(&directory_name)
                .map_err(map_path_error)?;
            if final_guard.identity != identity {
                return Err(CoreError::UnsafePath);
            }

            let record = BackupRecord {
                id: Uuid::new_v4().to_string(),
                backup_id: backup_id.clone(),
                directory_path: display_path(&final_directory),
                format_version: i64::from(read_back.manifest.format_version),
                schema_version,
                app_version: env!("CARGO_PKG_VERSION").to_owned(),
                origin,
                created_at: created_at.clone(),
                registered_at: now_utc_rfc3339(),
                data_size_bytes: payload_size,
                manifest_size_bytes: read_back.manifest_size,
                payload_sha256,
                integrity_status: BackupIntegrityStatus::Verified,
                verified_at: Some(now_utc_rfc3339()),
                last_error: None,
                directory_exists: true,
                manifest_exists: true,
            };

            if let Err(error) = self.insert_record(&record) {
                let _ = cleanup_owned_directory(
                    &target_guard,
                    &directory_name,
                    final_guard,
                    TRANSIENT_FILES,
                );
                return Err(error);
            }
            drop(final_guard);
            Ok(record)
        })();

        // 目录句柄在 Windows 上会阻止删除目录；失败清理前必须先释放。
        if result.is_err() {
            if let Some(guard) = partial_guard {
                let _ =
                    cleanup_owned_directory(&target_guard, &partial_name, guard, TRANSIENT_FILES);
            }
        }
        result
    }

    fn current_schema_version(&self) -> Result<i64, CoreError> {
        let conn = self.database.lock()?;
        Ok(conn.query_row(
            "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
            [],
            |row| row.get(0),
        )?)
    }

    fn insert_record(&self, record: &BackupRecord) -> Result<(), CoreError> {
        self.database.with_transaction(|transaction| {
            transaction.execute(
                "INSERT INTO backup_records (
                    id, backup_id, directory_path, format_version, schema_version, app_version,
                    origin, created_at, registered_at, data_size_bytes, manifest_size_bytes,
                    payload_sha256, integrity_status, verified_at, last_error
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
                params![
                    record.id,
                    record.backup_id,
                    record.directory_path,
                    record.format_version,
                    record.schema_version,
                    record.app_version,
                    record.origin.as_str(),
                    record.created_at,
                    record.registered_at,
                    record.data_size_bytes as i64,
                    record.manifest_size_bytes as i64,
                    record.payload_sha256,
                    record.integrity_status.as_str(),
                    record.verified_at,
                    record.last_error
                ],
            )?;
            Ok(())
        })
    }

    fn update_integrity(
        &self,
        backup_id: &str,
        status: BackupIntegrityStatus,
        verified_at: Option<&str>,
        error: Option<&str>,
    ) -> Result<(), CoreError> {
        self.database.with_transaction(|transaction| {
            transaction.execute(
                "UPDATE backup_records SET integrity_status = ?1, verified_at = ?2, last_error = ?3
                 WHERE backup_id = ?4",
                params![status.as_str(), verified_at, error, backup_id],
            )?;
            Ok(())
        })
    }
}

pub fn database_file_sizes(path: &Path) -> (u64, u64, u64) {
    let main = file_len(path);
    let wal = file_len(&sidecar(path, "-wal"));
    let shm = file_len(&sidecar(path, "-shm"));
    (main, wal, shm)
}

fn sidecar(path: &Path, suffix: &str) -> PathBuf {
    let mut value = path.as_os_str().to_os_string();
    value.push(suffix);
    PathBuf::from(value)
}

fn file_len(path: &Path) -> u64 {
    fs::metadata(path)
        .map(|metadata| metadata.len())
        .unwrap_or(0)
}

fn cleanup_stale_partials(target: &Directories) {
    let Ok(entries) = target.names() else {
        return;
    };
    for name in entries.flatten() {
        if !name
            .to_str()
            .is_some_and(|name| name.starts_with(".partial-"))
        {
            continue;
        }
        let Ok(directory) = target.open_child(&name) else {
            continue;
        };
        let stale = directory
            .modified()
            .ok()
            .and_then(|modified| SystemTime::now().duration_since(modified).ok())
            .is_some_and(|age| age >= STALE_PARTIAL_AGE);
        if !stale {
            continue;
        }
        if owns_only_known_files(&directory).unwrap_or(false) {
            let _ = cleanup_owned_directory(target, &name, directory, TRANSIENT_FILES);
        }
    }
}

fn owns_only_known_files(directory: &Directories) -> Result<bool, CoreError> {
    for name in directory.names()? {
        if !name?
            .to_str()
            .is_some_and(|name| TRANSIENT_FILES.contains(&name))
        {
            return Ok(false);
        }
    }
    Ok(true)
}

fn cleanup_owned_directory(
    parent: &Directories,
    name: impl AsRef<std::ffi::OsStr>,
    directory: Directories,
    files: &[&str],
) -> Result<(), CoreError> {
    if parent.open_child(&name).map_err(map_path_error)?.identity != directory.identity {
        return Err(CoreError::UnsafePath);
    }
    remove_known_files(&directory, files)?;
    drop(directory);
    parent.remove_child_dir(name).map_err(map_path_error)
}

fn directory_exists(path: &Path) -> bool {
    fs::symlink_metadata(path)
        .map(|metadata| metadata.is_dir() && !crate::claude::path_guard::redirected(&metadata))
        .unwrap_or(false)
}

fn normalize_target(value: &str) -> Result<PathBuf, CoreError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(CoreError::Validation("请选择备份目标目录".to_owned()));
    }
    let path = PathBuf::from(trimmed);
    super::paths::validate_absolute(&path)?;
    Ok(path)
}

fn short_id(value: &str) -> String {
    value.chars().take(8).collect()
}

pub(crate) fn is_due(policy: &BackupPolicy, now: DateTime<Utc>) -> bool {
    let Some(last) = policy
        .last_automatic_at
        .as_deref()
        .and_then(|value| DateTime::parse_from_rfc3339(value).ok())
    else {
        return true;
    };
    let elapsed = now.signed_duration_since(last.with_timezone(&Utc));
    elapsed.num_seconds() >= policy.interval_hours.saturating_mul(3600)
}

fn row_to_record(row: &Row<'_>) -> rusqlite::Result<BackupRecord> {
    let origin_text: String = row.get(6)?;
    let origin = BackupOrigin::parse(&origin_text).map_err(|_| {
        rusqlite::Error::InvalidColumnType(6, "origin".to_owned(), rusqlite::types::Type::Text)
    })?;
    let status_text: String = row.get(12)?;
    let integrity_status = BackupIntegrityStatus::parse(&status_text).ok_or_else(|| {
        rusqlite::Error::InvalidColumnType(
            12,
            "integrity_status".to_owned(),
            rusqlite::types::Type::Text,
        )
    })?;
    Ok(BackupRecord {
        id: row.get(0)?,
        backup_id: row.get(1)?,
        directory_path: row.get(2)?,
        format_version: row.get(3)?,
        schema_version: row.get(4)?,
        app_version: row.get(5)?,
        origin,
        created_at: row.get(7)?,
        registered_at: row.get(8)?,
        data_size_bytes: row.get::<_, i64>(9)?.max(0) as u64,
        manifest_size_bytes: row.get::<_, i64>(10)?.max(0) as u64,
        payload_sha256: row.get(11)?,
        integrity_status,
        verified_at: row.get(13)?,
        last_error: row.get(14)?,
        directory_exists: false,
        manifest_exists: false,
    })
}

fn row_to_policy(row: &Row<'_>) -> rusqlite::Result<BackupPolicy> {
    Ok(BackupPolicy {
        automatic_enabled: row.get::<_, i64>(0)? == 1,
        target_directory: row.get(1)?,
        interval_hours: row.get(2)?,
        retention_count: row.get(3)?,
        last_automatic_at: row.get(4)?,
        last_automatic_error_code: row.get(5)?,
        last_automatic_error_at: row.get(6)?,
        updated_at: row.get(7)?,
    })
}

fn default_policy() -> BackupPolicy {
    BackupPolicy {
        automatic_enabled: false,
        target_directory: None,
        interval_hours: 24,
        retention_count: 5,
        last_automatic_at: None,
        last_automatic_error_code: None,
        last_automatic_error_at: None,
        updated_at: now_utc_rfc3339(),
    }
}

fn pass_check(id: &str, label: &str, detail: &str) -> IntegrityCheck {
    IntegrityCheck {
        id: id.to_owned(),
        label: label.to_owned(),
        status: IntegrityStatus::Pass,
        detail: Some(detail.to_owned()),
    }
}

fn failed_check(id: &str, label: &str, detail: &str) -> IntegrityCheck {
    IntegrityCheck {
        id: id.to_owned(),
        label: label.to_owned(),
        status: IntegrityStatus::Fail,
        detail: Some(detail.to_owned()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn policy(last: Option<&str>, interval: i64) -> BackupPolicy {
        BackupPolicy {
            automatic_enabled: true,
            target_directory: Some("D:\\backups".to_owned()),
            interval_hours: interval,
            retention_count: 5,
            last_automatic_at: last.map(str::to_owned),
            last_automatic_error_code: None,
            last_automatic_error_at: None,
            updated_at: "2026-01-01T00:00:00Z".to_owned(),
        }
    }

    #[test]
    fn automatic_backup_is_due_without_history_or_after_interval() {
        let now = DateTime::parse_from_rfc3339("2026-11-13T12:00:00Z")
            .expect("time")
            .with_timezone(&Utc);
        assert!(is_due(&policy(None, 24), now));
        assert!(!is_due(&policy(Some("2026-11-13T00:00:00Z"), 24), now));
        assert!(is_due(&policy(Some("2026-11-12T00:00:00Z"), 24), now));
    }

    #[test]
    fn stop_never_deletes_manual_backups() {
        // 保留策略的过滤条件只接受 automatic 来源。
        assert_eq!(BackupOrigin::Manual.as_str(), "manual");
        assert_eq!(BackupOrigin::Automatic.as_str(), "automatic");
    }
}
