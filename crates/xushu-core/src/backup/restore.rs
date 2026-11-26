//! 灾难恢复：只读预检、隔离验证、受控切换、回退与启动恢复。
//!
//! 正式恢复必须在用户明确确认后才执行。切换前创建当前数据库的一致性回退
//! 副本，并把切换状态写入应用数据目录中的恢复日志；进程异常退出后，下次启动
//! 会优先回退到原数据库，而不是假定恢复已经成功。

use std::fs;
use std::io::{Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use chrono::{DateTime, SecondsFormat, Utc};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::claude::path_guard::{Directories, FileAccess};
use crate::db::{now_utc_rfc3339, Database, MIGRATIONS};
use crate::error::CoreError;

use super::integrity::{
    DataIntegrityReport, DataIntegrityService, IntegrityCheck, IntegrityStatus,
};
use super::manifest::{
    isolated_failure_detail, open_isolated_payload, read_manifest_in, verify_payload_in,
    BackupManifest, BackupOrigin, MANIFEST_FILE_NAME, MAX_BACKUP_BYTES, PAYLOAD_FILE_NAME,
};
use super::paths::{
    atomic_replace_file, atomic_replace_in, capacity_ok, copy_bounded, display_path,
    free_space_bytes, map_path_error, private_data_directory, read_bounded, read_bounded_file,
    remove_known_files, secure_directory, sha256_file, sha256_reader, validate_absolute,
};
use super::service::database_file_sizes;

pub const RESTORE_TOKEN_TTL_MINUTES: i64 = 15;
const RESTORE_SPACE_MARGIN: u64 = 16 * 1024 * 1024;
const JOURNAL_FILE_NAME: &str = "restore-journal.json";
const JOURNAL_MAX_BYTES: u64 = 64 * 1024;
pub const RESTORE_CONFIRMATION_TEXT: &str = "恢复";
const ROLLBACK_FILE_NAME: &str = "rollback-xushu.db";
const ORIGINAL_PREFIX: &str = "original-xushu.db";

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RestorePreflightInput {
    pub backup_directory: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PerformRestoreInput {
    pub token: String,
    pub confirmation_text: String,
    pub acknowledged: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RestoreDataSummary {
    pub projects: i64,
    pub archived_projects: i64,
    pub tasks: i64,
    pub done_tasks: i64,
    pub criteria: i64,
    pub verification_runs: i64,
    pub evidence: i64,
    pub claude_events: i64,
    pub memories: i64,
    pub decisions: i64,
    pub recovery_snapshots: i64,
    pub project_names: Vec<String>,
    pub missing_project_root_count: i64,
    pub running_verification_count: i64,
    pub desired_hook_project_count: i64,
    pub database_size_bytes: u64,
    pub last_updated_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RestoreDataChange {
    pub label: String,
    pub count: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RestoreHooksReview {
    pub project_count: i64,
    pub settings_files_modified: bool,
    pub pending_spool_included: bool,
    pub salt_included: bool,
    pub note: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RestorePreflight {
    pub token: Option<String>,
    pub ready: bool,
    pub blocking_reason: Option<String>,
    pub backup_id: String,
    pub backup_directory: String,
    pub created_at: String,
    pub app_version: String,
    pub origin: BackupOrigin,
    pub format_version: u32,
    pub backup_schema_version: i64,
    pub current_schema_version: i64,
    pub schema_action: String,
    pub manifest_sha256: String,
    pub payload_sha256: String,
    pub payload_size_bytes: u64,
    pub current: RestoreDataSummary,
    pub incoming: Option<RestoreDataSummary>,
    pub changes_since_backup: Vec<RestoreDataChange>,
    pub checks: Vec<IntegrityCheck>,
    pub warnings: Vec<String>,
    pub effects: Vec<String>,
    pub hooks_review: RestoreHooksReview,
    pub expires_at: String,
}

#[derive(Debug, Clone)]
pub struct RestorePlan {
    pub token: String,
    pub backup_id: String,
    pub backup_directory: PathBuf,
    pub isolated_database: PathBuf,
    pub workspace_directory: PathBuf,
    pub manifest_sha256: String,
    pub payload_sha256: String,
    pub isolated_sha256: String,
    pub backup_schema_version: i64,
    pub expires_at: DateTime<Utc>,
    pub preflight: RestorePreflight,
    current_change_stamp: (u64, i64),
    workspace_identity: String,
}

#[derive(Debug, Clone)]
pub struct RestorePreflightOutcome {
    pub preflight: RestorePreflight,
    pub plan: Option<RestorePlan>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RollbackReport {
    pub status: String,
    pub directory: Option<String>,
    pub verified: bool,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RestoreOutcome {
    pub operation_id: String,
    pub status: String,
    pub switch_verified: bool,
    pub final_report: Option<DataIntegrityReport>,
    pub rollback: RollbackReport,
    pub hooks_review: Option<RestoreHooksReview>,
    pub message: String,
    pub completed_at: String,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RestoreStartupNotice {
    pub outcome: String,
    pub message: String,
    pub journal_file: String,
    pub rollback_directory: Option<String>,
    pub checked_at: String,
}

impl RestoreJournal {
    /// 面向 UI 的回退目录展示路径（去掉 Windows `\\?\` 前缀）。
    pub fn rollback_directory(&self) -> String {
        display_path(Path::new(&self.rollback_directory))
    }
}

impl RestoreStartupNotice {
    pub fn needs_manual(
        journal_file: String,
        message: impl Into<String>,
        rollback_directory: Option<String>,
    ) -> Self {
        Self {
            outcome: "needs_manual".to_owned(),
            message: message.into(),
            journal_file,
            rollback_directory,
            checked_at: now_utc_rfc3339(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RestoreJournal {
    restore_id: String,
    backup_id: String,
    backup_directory: String,
    rollback_directory: String,
    rollback_database: String,
    current_database: String,
    isolated_sha256: String,
    stage: String,
    started_at: String,
    updated_at: String,
}

pub struct RestoreService<'a> {
    database: &'a Database,
    data_directory: &'a Path,
}

impl<'a> RestoreService<'a> {
    pub fn new(database: &'a Database, data_directory: &'a Path) -> Self {
        Self {
            database,
            data_directory,
        }
    }

    pub fn preflight(
        &self,
        input: RestorePreflightInput,
    ) -> Result<RestorePreflightOutcome, CoreError> {
        let raw = input.backup_directory.trim();
        if raw.is_empty() {
            return Err(CoreError::Validation("请选择备份目录".to_owned()));
        }
        let requested = PathBuf::from(raw);
        validate_absolute(&requested)?;
        let backup_guard = secure_directory(&requested)?;
        let backup_directory = backup_guard.canonical.clone();

        let manifest_read = read_manifest_in(&backup_guard)?;
        let manifest = manifest_read.manifest.clone();
        let (payload_size, payload_sha256) = verify_payload_in(&backup_guard, &manifest)?;

        let current_change_stamp = self.database.change_stamp()?;
        let current_summary = self.current_summary()?;
        let changes_since_backup = self.changes_since_backup(&manifest.created_at)?;
        let current_schema_version = self.current_schema_version()?;
        let supported_schema = MIGRATIONS.last().map_or(0, |migration| migration.version);

        let mut checks = vec![
            pass_check("manifest", "备份清单格式", "格式与字段通过校验"),
            pass_check(
                "payload_hash",
                "数据文件校验和",
                "大小与 SHA-256 与清单一致",
            ),
        ];
        let mut warnings = base_warnings();
        let mut effects = base_effects(&manifest);
        let mut hooks_review = RestoreHooksReview {
            project_count: current_summary.desired_hook_project_count,
            settings_files_modified: false,
            pending_spool_included: false,
            salt_included: false,
            note: "恢复不会自动安装、停用或修改项目 .claude/settings.local.json；需要重新检查接入状态。"
                .to_owned(),
        };

        if manifest.schema_version > supported_schema {
            checks.push(failed_check(
                "schema_compatibility",
                "数据库版本兼容性",
                "备份数据库版本高于当前应用支持的版本，已拒绝降级恢复",
            ));
            return Ok(RestorePreflightOutcome {
                preflight: self.blocked_preflight(
                    &backup_directory,
                    &manifest,
                    &manifest_read.manifest_sha256,
                    &payload_sha256,
                    payload_size,
                    current_schema_version,
                    current_summary,
                    &changes_since_backup,
                    checks,
                    "备份数据库版本高于当前应用支持的版本。",
                    warnings,
                    effects,
                    hooks_review,
                ),
                plan: None,
            });
        }

        checks.push(pass_check(
            "schema_compatibility",
            "数据库版本兼容性",
            if manifest.schema_version == supported_schema {
                "备份版本与当前应用一致"
            } else {
                "备份版本可通过内置迁移升级到当前版本"
            },
        ));

        let required_space = payload_size.saturating_add(RESTORE_SPACE_MARGIN);
        let capacity_available = free_space_bytes(self.data_directory);
        if !capacity_ok(required_space, capacity_available) {
            let reason = if capacity_available.is_none() {
                "无法确认应用数据目录的可用空间，无法完成隔离恢复验证。"
            } else {
                "应用数据目录可用空间不足，无法完成隔离恢复验证。"
            };
            checks.push(failed_check("disk_space", "应用数据目录空间", reason));
            return Ok(RestorePreflightOutcome {
                preflight: self.blocked_preflight(
                    &backup_directory,
                    &manifest,
                    &manifest_read.manifest_sha256,
                    &payload_sha256,
                    payload_size,
                    current_schema_version,
                    current_summary,
                    &changes_since_backup,
                    checks,
                    reason,
                    warnings,
                    effects,
                    hooks_review,
                ),
                plan: None,
            });
        }
        checks.push(pass_check(
            "disk_space",
            "应用数据目录空间",
            "隔离副本所需空间充足",
        ));

        let token = Uuid::new_v4().to_string();
        let workspace_root = self.data_directory.join("restore-workspaces");
        let workspace_root_guard = private_data_directory(&workspace_root, true)?;
        let workspace_directory = workspace_root_guard.canonical.join(&token);
        workspace_root_guard
            .create_child(&token)
            .map_err(map_path_error)?;
        let workspace_guard = workspace_root_guard
            .open_child(&token)
            .map_err(map_path_error)?;
        let workspace_identity = workspace_guard.identity.clone();
        let isolated_database = workspace_directory.join(PAYLOAD_FILE_NAME);

        let isolated_result =
            (|| -> Result<(RestoreDataSummary, String, DataIntegrityReport), CoreError> {
                let workspace_payload = |access| {
                    workspace_guard
                        .open_file(PAYLOAD_FILE_NAME, access)
                        .map_err(map_path_error)
                };
                copy_bounded(
                    backup_guard
                        .open_file(PAYLOAD_FILE_NAME, FileAccess::Read)
                        .map_err(map_path_error)?,
                    || workspace_payload(FileAccess::CreateNew),
                    MAX_BACKUP_BYTES,
                )?;
                let (copied_sha, _) =
                    sha256_reader(&mut workspace_payload(FileAccess::Read)?, MAX_BACKUP_BYTES)?;
                if !copied_sha.eq_ignore_ascii_case(&payload_sha256) {
                    return Err(CoreError::Backup(
                        "隔离副本与备份数据文件校验和不一致".to_owned(),
                    ));
                }

                let isolated = open_isolated_payload(&isolated_database, manifest.schema_version)?;
                let compaction = isolated.compact_for_backup();
                let report = DataIntegrityService::new(&isolated).check();
                let summary = {
                    let guard = isolated.lock()?;
                    summarize_connection(&guard, file_len(&isolated_database))
                };
                drop(isolated);
                let report = report?;
                if !report.ok {
                    return Err(CoreError::Restore(
                        "隔离副本的 SQLite 或业务关系检查未通过".to_owned(),
                    ));
                }
                compaction?;
                remove_known_files(&workspace_guard, &["xushu.db-wal", "xushu.db-shm"])?;
                let (isolated_sha, _) =
                    sha256_reader(&mut workspace_payload(FileAccess::Read)?, MAX_BACKUP_BYTES)?;
                Ok((summary?, isolated_sha, report))
            })();

        let (incoming_summary, isolated_sha256, isolated_report) = match isolated_result {
            Ok(value) => value,
            Err(error) => {
                drop(workspace_guard);
                cleanup_owned_workspace(&workspace_directory, &workspace_identity);
                checks.push(failed_check(
                    "isolated_restore",
                    "隔离恢复验证",
                    &isolated_failure_detail(&error),
                ));
                return Ok(RestorePreflightOutcome {
                    preflight: self.blocked_preflight(
                        &backup_directory,
                        &manifest,
                        &manifest_read.manifest_sha256,
                        &payload_sha256,
                        payload_size,
                        current_schema_version,
                        current_summary,
                        &changes_since_backup,
                        checks,
                        "隔离恢复验证未通过，不能进入正式恢复。",
                        warnings,
                        effects,
                        hooks_review,
                    ),
                    plan: None,
                });
            }
        };

        checks.push(pass_check(
            "isolated_restore",
            "隔离恢复验证",
            "隔离副本通过迁移、完整性、外键与业务关系检查",
        ));
        for check in &isolated_report.checks {
            checks.push(IntegrityCheck {
                id: format!("isolated_{}", check.id),
                label: format!("隔离：{}", check.label),
                status: check.status,
                detail: check.detail.clone(),
            });
        }
        if incoming_summary.running_verification_count > 0 {
            warnings.push(format!(
                "备份中有 {} 个验证运行处于 running；恢复后会在下次启动时标记为中断。",
                incoming_summary.running_verification_count
            ));
        }
        if incoming_summary.missing_project_root_count > 0 {
            warnings.push(format!(
                "备份中有 {} 个项目的本地路径在当前机器不存在；恢复后需要重新定位或重新导入。",
                incoming_summary.missing_project_root_count
            ));
        }
        warnings.push(
            "备份仅包含 SQLite 数据库快照，不包含 Hook Spool、伪名化 Salt、项目 .claude 配置、导出报告或 WebView 本地偏好。"
                .to_owned(),
        );
        warnings.push(
            "恢复完成后不会自动重新安装或启用 Claude Code Hooks，必须人工重新检查接入状态。"
                .to_owned(),
        );
        hooks_review.project_count = incoming_summary.desired_hook_project_count;
        effects.push(
            "当前数据库会先保存为一致性回退副本；若切换或最终校验失败，将优先回退到原数据库。"
                .to_owned(),
        );

        let expires_at = Utc::now() + chrono::Duration::minutes(RESTORE_TOKEN_TTL_MINUTES);
        let preflight = RestorePreflight {
            token: Some(token.clone()),
            ready: true,
            blocking_reason: None,
            backup_id: manifest.backup_id.clone(),
            backup_directory: display_path(&backup_directory),
            created_at: manifest.created_at.clone(),
            app_version: manifest.app_version.clone(),
            origin: manifest.origin,
            format_version: manifest.format_version,
            backup_schema_version: manifest.schema_version,
            current_schema_version,
            schema_action: if manifest.schema_version == current_schema_version {
                "same".to_owned()
            } else {
                "upgrade".to_owned()
            },
            manifest_sha256: manifest_read.manifest_sha256.clone(),
            payload_sha256: payload_sha256.clone(),
            payload_size_bytes: payload_size,
            current: current_summary,
            incoming: Some(incoming_summary),
            changes_since_backup,
            checks,
            warnings,
            effects,
            hooks_review,
            expires_at: expires_at.to_rfc3339_opts(SecondsFormat::Millis, true),
        };

        let plan = RestorePlan {
            token,
            backup_id: manifest.backup_id,
            backup_directory,
            isolated_database,
            workspace_directory,
            manifest_sha256: manifest_read.manifest_sha256,
            payload_sha256,
            isolated_sha256,
            backup_schema_version: manifest.schema_version,
            expires_at,
            preflight: preflight.clone(),
            current_change_stamp,
            workspace_identity,
        };

        Ok(RestorePreflightOutcome {
            preflight,
            plan: Some(plan),
        })
    }

    #[allow(clippy::too_many_arguments)]
    fn blocked_preflight(
        &self,
        backup_directory: &Path,
        manifest: &BackupManifest,
        manifest_sha256: &str,
        payload_sha256: &str,
        payload_size: u64,
        current_schema_version: i64,
        current_summary: RestoreDataSummary,
        changes_since_backup: &[RestoreDataChange],
        checks: Vec<IntegrityCheck>,
        reason: &str,
        warnings: Vec<String>,
        effects: Vec<String>,
        hooks_review: RestoreHooksReview,
    ) -> RestorePreflight {
        RestorePreflight {
            token: None,
            ready: false,
            blocking_reason: Some(reason.to_owned()),
            backup_id: manifest.backup_id.clone(),
            backup_directory: display_path(backup_directory),
            created_at: manifest.created_at.clone(),
            app_version: manifest.app_version.clone(),
            origin: manifest.origin,
            format_version: manifest.format_version,
            backup_schema_version: manifest.schema_version,
            current_schema_version,
            schema_action: if manifest.schema_version == current_schema_version {
                "same".to_owned()
            } else if manifest.schema_version > current_schema_version {
                "incompatible".to_owned()
            } else {
                "upgrade".to_owned()
            },
            manifest_sha256: manifest_sha256.to_owned(),
            payload_sha256: payload_sha256.to_owned(),
            payload_size_bytes: payload_size,
            current: current_summary,
            incoming: None,
            changes_since_backup: changes_since_backup.to_vec(),
            checks,
            warnings,
            effects,
            hooks_review,
            expires_at: now_utc_rfc3339(),
        }
    }

    /// 正式恢复的峰值空间估算：隔离副本、回退副本、切换副本与当前数据库都位于
    /// 应用数据目录。预检只检查隔离副本；正式切换前必须再次确认容量。
    pub fn check_formal_restore_capacity(&self, plan: &RestorePlan) -> Result<(), CoreError> {
        let (main, wal, shm) = database_file_sizes(self.database.path());
        let current = main.saturating_add(wal).saturating_add(shm);
        let payload = plan.preflight.payload_size_bytes;
        let required = current
            .saturating_mul(2)
            .saturating_add(payload.saturating_mul(2))
            .saturating_add(RESTORE_SPACE_MARGIN);
        let available = free_space_bytes(self.data_directory);
        if available.is_none() {
            return Err(CoreError::Restore(
                "无法确认应用数据目录的可用空间，已停止正式恢复".to_owned(),
            ));
        }
        if !capacity_ok(required, available) {
            return Err(CoreError::Restore(format!(
                "应用数据目录空间不足：正式恢复至少需要约 {} MiB 用于隔离、回退与切换副本",
                required.div_ceil(1024 * 1024)
            )));
        }
        Ok(())
    }

    pub fn ensure_current_unchanged(&self, plan: &RestorePlan) -> Result<(), CoreError> {
        if self.database.change_stamp()? != plan.current_change_stamp {
            return Err(CoreError::Restore(
                "当前数据库在预检后发生变化，请重新预检并确认恢复影响".to_owned(),
            ));
        }
        self.assert_no_running_verification()
    }

    pub fn assert_no_running_verification(&self) -> Result<(), CoreError> {
        let conn = self.database.lock()?;
        let running: i64 = conn.query_row(
            "SELECT COUNT(*) FROM verification_runs WHERE execution_status = 'running'",
            [],
            |row| row.get(0),
        )?;
        if running > 0 {
            return Err(CoreError::Conflict(
                "存在运行中的验收任务，不能执行数据库恢复".to_owned(),
            ));
        }
        Ok(())
    }

    pub fn hooks_review(database: &Database) -> Result<RestoreHooksReview, CoreError> {
        let conn = database.lock()?;
        let project_count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM claude_integrations WHERE desired_enabled = 1",
            [],
            |row| row.get(0),
        )?;
        Ok(RestoreHooksReview {
            project_count,
            settings_files_modified: false,
            pending_spool_included: false,
            salt_included: false,
            note: "恢复不会自动安装、停用或修改项目 .claude/settings.local.json；需要重新检查接入状态。"
                .to_owned(),
        })
    }

    pub fn revalidate(plan: &RestorePlan) -> Result<(), CoreError> {
        if Utc::now() > plan.expires_at {
            return Err(CoreError::Restore(
                "恢复预检已过期，请重新执行预检".to_owned(),
            ));
        }
        let backup_guard = secure_directory(&plan.backup_directory)?;
        let workspace_guard = private_data_directory(&plan.workspace_directory, false)?;
        if workspace_guard.identity != plan.workspace_identity {
            return Err(CoreError::UnsafePath);
        }
        let manifest_read = read_manifest_in(&backup_guard)?;
        if manifest_read.manifest_sha256 != plan.manifest_sha256 {
            return Err(CoreError::Restore(
                "备份清单在预检后发生变化，请重新预检".to_owned(),
            ));
        }
        if manifest_read.manifest.backup_id != plan.backup_id
            || manifest_read.manifest.schema_version != plan.backup_schema_version
        {
            return Err(CoreError::Restore(
                "备份身份或数据库版本在预检后发生变化，请重新预检".to_owned(),
            ));
        }
        verify_payload_in(&backup_guard, &manifest_read.manifest)?;
        let mut payload = workspace_guard
            .open_file(PAYLOAD_FILE_NAME, FileAccess::Read)
            .map_err(map_path_error)?;
        let (isolated_sha, _) = sha256_reader(&mut payload, MAX_BACKUP_BYTES)?;
        if isolated_sha != plan.isolated_sha256 {
            return Err(CoreError::Restore(
                "隔离恢复副本在预检后发生变化，请重新预检".to_owned(),
            ));
        }
        Ok(())
    }

    pub fn validates_confirmation(input: &PerformRestoreInput) -> Result<(), CoreError> {
        if !input.acknowledged || input.confirmation_text.trim() != RESTORE_CONFIRMATION_TEXT {
            return Err(CoreError::RestoreConfirmationRequired);
        }
        Ok(())
    }

    pub fn create_rollback(&self, operation_id: &str) -> Result<RestoreJournal, CoreError> {
        let journal_path = self.data_directory.join(JOURNAL_FILE_NAME);
        match fs::symlink_metadata(journal_path) {
            Ok(_) => {
                return Err(CoreError::Restore(
                    "存在未处理的恢复日志；请先重启应用完成或回退上次恢复".to_owned(),
                ));
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(CoreError::Io(error)),
        }

        let rollback_root = self.data_directory.join("restore-rollback");
        let rollback_root_guard = private_data_directory(&rollback_root, true)?;
        let rollback_directory = rollback_root_guard.canonical.join(operation_id);
        rollback_root_guard
            .create_child(operation_id)
            .map_err(map_path_error)?;
        let rollback_guard = rollback_root_guard
            .open_child(operation_id)
            .map_err(map_path_error)?;
        let rollback_database = rollback_directory.join(ROLLBACK_FILE_NAME);

        let result = (|| -> Result<RestoreJournal, CoreError> {
            self.database.backup_to(&rollback_database)?;
            {
                let snapshot = Database::open_existing(&rollback_database)?;
                snapshot.compact_for_backup()?;
                let report = DataIntegrityService::new(&snapshot).check()?;
                if !report.ok {
                    return Err(CoreError::Restore(
                        "当前数据库回退副本未通过完整性检查".to_owned(),
                    ));
                }
            }
            remove_known_files(
                &rollback_guard,
                &["rollback-xushu.db-wal", "rollback-xushu.db-shm"],
            )?;
            let (_, rollback_size) = sha256_file(&rollback_database, MAX_BACKUP_BYTES)?;
            if rollback_size == 0 {
                return Err(CoreError::Restore("回退副本为空".to_owned()));
            }

            let current_database = self.database.path().to_path_buf();
            let now = now_utc_rfc3339();
            let journal = RestoreJournal {
                restore_id: operation_id.to_owned(),
                backup_id: String::new(),
                backup_directory: String::new(),
                rollback_directory: rollback_directory.to_string_lossy().to_string(),
                rollback_database: rollback_database.to_string_lossy().to_string(),
                current_database: current_database.to_string_lossy().to_string(),
                isolated_sha256: String::new(),
                stage: "prepared".to_owned(),
                started_at: now.clone(),
                updated_at: now,
            };
            write_journal(self.data_directory, &journal)?;
            Ok(journal)
        })();

        let rollback_identity = rollback_guard.identity.clone();
        drop(rollback_guard);
        if result.is_err() {
            cleanup_owned_workspace(&rollback_directory, &rollback_identity);
        }
        result
    }

    pub fn begin_switch(
        data_directory: &Path,
        journal: &mut RestoreJournal,
        plan: &RestorePlan,
    ) -> Result<(), CoreError> {
        journal.backup_id = plan.backup_id.clone();
        journal.backup_directory = display_path(&plan.backup_directory);
        journal.isolated_sha256 = plan.isolated_sha256.clone();
        journal.stage = "switching".to_owned();
        journal.updated_at = now_utc_rfc3339();
        write_journal(data_directory, journal)
    }

    pub fn switch_files(
        data_directory: &Path,
        journal: &RestoreJournal,
        plan: &RestorePlan,
    ) -> Result<u64, CoreError> {
        let current_database = PathBuf::from(&journal.current_database);
        let rollback_directory = PathBuf::from(&journal.rollback_directory);
        if current_database != data_directory.join(PAYLOAD_FILE_NAME)
            || plan.isolated_database != plan.workspace_directory.join(PAYLOAD_FILE_NAME)
        {
            return Err(CoreError::UnsafePath);
        }
        let data = private_data_directory(data_directory, false)?;
        let rollback = validate_rollback_directory(&data, &rollback_directory)?;
        let workspace = data
            .open_child("restore-workspaces")
            .and_then(|root| root.open_child(&plan.token))
            .map_err(map_path_error)?;
        if workspace.identity != plan.workspace_identity
            || private_data_directory(&workspace.canonical, false)?.identity != workspace.identity
        {
            return Err(CoreError::UnsafePath);
        }
        let mut source = workspace
            .open_file(PAYLOAD_FILE_NAME, FileAccess::Read)
            .map_err(map_path_error)?;
        if sha256_reader(&mut source, MAX_BACKUP_BYTES)?.0 != plan.isolated_sha256 {
            return Err(CoreError::Restore(
                "隔离恢复副本在预检后发生变化，请重新预检".to_owned(),
            ));
        }
        source.seek(SeekFrom::Start(0))?;

        for (name, suffix) in current_files(&data)? {
            data.move_file(&name, &rollback, format!("{ORIGINAL_PREFIX}{suffix}"))
                .map_err(map_path_error)?;
        }

        let copied = copy_bounded(
            source,
            || {
                data.open_file(PAYLOAD_FILE_NAME, FileAccess::CreateNew)
                    .map_err(map_path_error)
            },
            MAX_BACKUP_BYTES,
        )?;
        let mut installed = data
            .open_file(PAYLOAD_FILE_NAME, FileAccess::Read)
            .map_err(map_path_error)?;
        let (copied_sha, _) = sha256_reader(&mut installed, MAX_BACKUP_BYTES)?;
        if copied_sha != plan.isolated_sha256 {
            return Err(CoreError::Restore(
                "切换后的数据库校验和与隔离副本不一致".to_owned(),
            ));
        }

        let mut updated = journal.clone();
        updated.stage = "switched".to_owned();
        updated.updated_at = now_utc_rfc3339();
        atomic_replace_in(&data, JOURNAL_FILE_NAME.as_ref(), &journal_bytes(&updated)?)?;
        Ok(copied)
    }

    pub fn verify_restored_database(
        current_database: &Path,
    ) -> Result<DataIntegrityReport, CoreError> {
        let (_, size) = sha256_file(current_database, MAX_BACKUP_BYTES)?;
        if size == 0 {
            return Err(CoreError::Restore("数据库文件为空".to_owned()));
        }
        let database = Database::open_existing(current_database)?;
        let report = DataIntegrityService::new(&database).check()?;
        database.checkpoint()?;
        Ok(report)
    }

    pub fn rollback_switch(data_directory: &Path, journal: &RestoreJournal) -> RollbackReport {
        let current_database = PathBuf::from(&journal.current_database);
        let rollback_directory = PathBuf::from(&journal.rollback_directory);
        let directory_label = Some(display_path(Path::new(&journal.rollback_directory)));

        let result = (|| -> Result<(), CoreError> {
            if current_database != data_directory.join(PAYLOAD_FILE_NAME) {
                return Err(CoreError::UnsafePath);
            }
            let data = private_data_directory(data_directory, false)?;
            let rollback_guard = validate_rollback_directory(&data, &rollback_directory)?;
            let rollback_database = rollback_directory.join(ROLLBACK_FILE_NAME);
            if Path::new(&journal.rollback_database) != rollback_database {
                return Err(CoreError::UnsafePath);
            }
            verify_database_health(&rollback_database)?;
            let source = rollback_guard
                .open_file(ROLLBACK_FILE_NAME, FileAccess::Read)
                .map_err(map_path_error)?;

            let interrupted = interrupted_name();
            for (name, suffix) in current_files(&data)? {
                data.move_file(&name, &rollback_guard, format!("{interrupted}{suffix}"))
                    .map_err(map_path_error)?;
            }

            copy_bounded(
                source,
                || {
                    data.open_file(PAYLOAD_FILE_NAME, FileAccess::CreateNew)
                        .map_err(map_path_error)
                },
                MAX_BACKUP_BYTES,
            )?;

            let report = RestoreService::verify_restored_database(&current_database)?;
            if !report.ok {
                return Err(CoreError::Restore(
                    "回退后的数据库未通过完整性检查".to_owned(),
                ));
            }
            Ok(())
        })();

        match result {
            Ok(()) => RollbackReport {
                status: "used".to_owned(),
                directory: directory_label,
                verified: true,
                message: "已回退到恢复前的数据库，并通过完整性检查".to_owned(),
            },
            Err(error) => {
                let _ = error;
                RollbackReport {
                    status: "failed".to_owned(),
                    directory: directory_label,
                    verified: false,
                    message:
                        "回退失败；原数据库与回退副本仍保留在回退目录中，请勿删除，需要人工检查"
                            .to_owned(),
                }
            }
        }
    }

    pub fn finish_journal(
        data_directory: &Path,
        journal: &RestoreJournal,
        outcome: &str,
    ) -> Result<(), CoreError> {
        let mut updated = journal.clone();
        updated.stage = outcome.to_owned();
        updated.updated_at = now_utc_rfc3339();
        let guard = private_data_directory(data_directory, false)?;
        let destination = format!(
            "restore-journal-{outcome}-{}.json",
            short_id(&journal.restore_id)
        );
        let bytes = journal_bytes(&updated)?;
        atomic_replace_in(&guard, JOURNAL_FILE_NAME.as_ref(), &bytes)?;
        guard
            .rename_file(JOURNAL_FILE_NAME, destination)
            .map_err(map_path_error)?;
        Ok(())
    }

    pub fn active_journal_stage(data_directory: &Path) -> Option<String> {
        let bytes =
            read_bounded_file(&data_directory.join(JOURNAL_FILE_NAME), JOURNAL_MAX_BYTES).ok()?;
        serde_json::from_slice::<RestoreJournal>(&bytes)
            .ok()
            .map(|journal| journal.stage)
    }

    pub fn cleanup_plan_workspace(plan: &RestorePlan) {
        cleanup_owned_workspace(&plan.workspace_directory, &plan.workspace_identity);
    }

    fn current_schema_version(&self) -> Result<i64, CoreError> {
        let conn = self.database.lock()?;
        Ok(conn.query_row(
            "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
            [],
            |row| row.get(0),
        )?)
    }

    fn changes_since_backup(&self, backup_time: &str) -> Result<Vec<RestoreDataChange>, CoreError> {
        let conn = self.database.lock()?;
        let columns = [
            ("tasks", "updated_at", "任务更新"),
            ("projects", "updated_at", "项目更新"),
            ("acceptance_criteria", "updated_at", "验收条件更新"),
            ("verification_runs", "created_at", "验证运行"),
            ("project_memories", "updated_at", "记忆更新"),
            ("decisions", "updated_at", "决策更新"),
            ("recovery_snapshots", "generated_at", "恢复快照"),
            ("claude_events", "occurred_at", "Claude 事件"),
        ];
        let mut changes = Vec::new();
        for (table, column, label) in columns {
            let count: i64 = conn.query_row(
                &format!("SELECT COUNT(*) FROM {table} WHERE {column} >= ?1"),
                [backup_time],
                |row| row.get(0),
            )?;
            if count > 0 {
                changes.push(RestoreDataChange {
                    label: label.to_owned(),
                    count,
                });
            }
        }
        Ok(changes)
    }

    fn current_summary(&self) -> Result<RestoreDataSummary, CoreError> {
        let conn = self.database.lock()?;
        let (main, wal, shm) = database_file_sizes(self.database.path());
        summarize_connection(&conn, main + wal + shm)
    }
}

impl RestoreService<'_> {
    pub fn record_operation(
        database: &Database,
        operation_id: &str,
        journal: &RestoreJournal,
        outcome: &str,
        switch_verified: bool,
        rollback_status: &str,
    ) -> Result<(), CoreError> {
        let now = now_utc_rfc3339();
        database.with_transaction(|transaction| {
            transaction.execute(
                "INSERT OR REPLACE INTO restore_operations (
                    id, backup_id, backup_directory, started_at, finished_at, outcome,
                    switch_verified, rollback_status, detail_json, created_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, '{}', ?9)",
                rusqlite::params![
                    operation_id,
                    journal.backup_id,
                    journal.backup_directory,
                    journal.started_at,
                    now,
                    outcome,
                    i64::from(switch_verified),
                    rollback_status,
                    now
                ],
            )?;
            Ok(())
        })
    }
}

pub fn cleanup_workspaces(data_directory: &Path) -> Result<(), CoreError> {
    let data = private_data_directory(data_directory, false)?;
    for root_name in ["restore-workspaces", "verify-workspaces"] {
        cleanup_workspace_root(&data, root_name)?;
    }
    Ok(())
}

fn cleanup_workspace_root(data: &Directories, root_name: &str) -> Result<(), CoreError> {
    let root = match data.open_child(root_name) {
        Ok(root) => root,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(map_path_error(error)),
    };
    for entry in root.names()? {
        let name = entry?;
        if !name
            .to_str()
            .is_some_and(|name| Uuid::parse_str(name).is_ok())
        {
            continue;
        }
        let directory = match root.open_child(&name) {
            Ok(directory) => directory,
            Err(error)
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::InvalidInput
                        | std::io::ErrorKind::NotFound
                        | std::io::ErrorKind::NotADirectory
                ) =>
            {
                continue
            }
            Err(error) => return Err(map_path_error(error)),
        };
        let mut only_known = true;
        for child in directory.names()? {
            if !matches!(
                child?.to_str(),
                Some(PAYLOAD_FILE_NAME | "xushu.db-wal" | "xushu.db-shm" | "xushu.db-journal")
            ) {
                only_known = false;
                break;
            }
        }
        if only_known {
            cleanup_workspace(&root, &name, directory);
        }
    }
    Ok(())
}

pub fn recover_interrupted_restore(
    data_directory: &Path,
) -> Result<Option<RestoreStartupNotice>, CoreError> {
    let journal_path = data_directory.join(JOURNAL_FILE_NAME);
    let data_guard = private_data_directory(data_directory, false)?;
    let journal_label = display_path(&journal_path);
    let file = match data_guard.open_file(JOURNAL_FILE_NAME, FileAccess::Read) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        result => result,
    };
    let bytes = match file
        .map_err(map_path_error)
        .and_then(|file| read_bounded(file, JOURNAL_MAX_BYTES))
    {
        Ok(bytes) => bytes,
        Err(_) => {
            return Ok(Some(RestoreStartupNotice {
                outcome: "needs_manual".to_owned(),
                message: "恢复日志不可读取；没有自动修改数据库。请人工检查恢复日志与回退目录。"
                    .to_owned(),
                journal_file: journal_label,
                rollback_directory: None,
                checked_at: now_utc_rfc3339(),
            }));
        }
    };
    let journal: RestoreJournal = match serde_json::from_slice(&bytes) {
        Ok(journal) => journal,
        Err(_) => {
            return Ok(Some(RestoreStartupNotice {
                outcome: "needs_manual".to_owned(),
                message: "恢复日志格式无效；没有自动修改数据库。请人工检查。".to_owned(),
                journal_file: journal_label,
                rollback_directory: None,
                checked_at: now_utc_rfc3339(),
            }));
        }
    };

    let expected_current = data_directory.join(PAYLOAD_FILE_NAME);
    if Path::new(&journal.current_database) != expected_current {
        return Ok(Some(RestoreStartupNotice {
            outcome: "needs_manual".to_owned(),
            message: "恢复日志指向的数据库路径异常；没有自动修改数据库。".to_owned(),
            journal_file: journal_label,
            rollback_directory: Some(display_path(Path::new(&journal.rollback_directory))),
            checked_at: now_utc_rfc3339(),
        }));
    }

    if matches!(journal.stage.as_str(), "prepared" | "abandoned") {
        if verify_database_health(&expected_current).is_err() {
            return Ok(Some(RestoreStartupNotice::needs_manual(
                journal_label,
                "切换前的数据库缺失或未通过完整性检查，请保留回退副本并人工检查。",
                Some(journal.rollback_directory()),
            )));
        }
        let destination = format!(
            "restore-journal-abandoned-{}.json",
            short_id(&journal.restore_id)
        );
        data_guard
            .rename_file(JOURNAL_FILE_NAME, destination)
            .map_err(map_path_error)?;
        return Ok(Some(RestoreStartupNotice {
            outcome: "abandoned".to_owned(),
            message: "上次恢复在切换前中断；正式数据库没有被替换，回退副本已保留。".to_owned(),
            journal_file: journal_label,
            rollback_directory: Some(display_path(Path::new(&journal.rollback_directory))),
            checked_at: now_utc_rfc3339(),
        }));
    }

    if matches!(
        journal.stage.as_str(),
        "completed" | "rolled_back" | "recovered"
    ) {
        match verify_database_health(Path::new(&journal.current_database)) {
            Ok(()) => {
                let completed = journal.stage == "completed";
                let destination = format!(
                    "restore-journal-{}-{}.json",
                    journal.stage,
                    short_id(&journal.restore_id)
                );
                data_guard
                    .rename_file(JOURNAL_FILE_NAME, destination)
                    .map_err(map_path_error)?;
                return Ok(Some(RestoreStartupNotice {
                    outcome: if completed {
                        "completed"
                    } else {
                        "rolled_back"
                    }
                    .to_owned(),
                    message: "上次恢复操作已结束，当前数据库通过启动完整性检查。".to_owned(),
                    journal_file: journal_label,
                    rollback_directory: Some(display_path(Path::new(&journal.rollback_directory))),
                    checked_at: now_utc_rfc3339(),
                }));
            }
            Err(_) => {
                // 继续走回退流程。
            }
        }
    }

    let rollback = RestoreService::rollback_switch(data_directory, &journal);
    if rollback.verified {
        let record = Database::open_existing(&expected_current).and_then(|database| {
            RestoreService::record_operation(
                &database,
                &journal.restore_id,
                &journal,
                "rolled_back",
                false,
                "used",
            )?;
            database.checkpoint()
        });
        if record.is_err()
            || RestoreService::finish_journal(data_directory, &journal, "recovered").is_err()
        {
            return Ok(Some(RestoreStartupNotice::needs_manual(
                journal_label,
                "数据库已回退，但无法持久化恢复结果；请保留日志与回退副本。",
                Some(journal.rollback_directory()),
            )));
        }
        Ok(Some(RestoreStartupNotice {
            outcome: "rolled_back".to_owned(),
            message: "检测到未完成的恢复切换；已自动回退到原数据库并通过完整性检查。".to_owned(),
            journal_file: journal_label,
            rollback_directory: Some(display_path(Path::new(&journal.rollback_directory))),
            checked_at: now_utc_rfc3339(),
        }))
    } else {
        Ok(Some(RestoreStartupNotice {
            outcome: "needs_manual".to_owned(),
            message: "检测到未完成的恢复切换且自动回退失败；未删除任何回退文件，请人工检查。"
                .to_owned(),
            journal_file: journal_label,
            rollback_directory: Some(display_path(Path::new(&journal.rollback_directory))),
            checked_at: now_utc_rfc3339(),
        }))
    }
}

fn validate_rollback_directory(
    data: &Directories,
    rollback_directory: &Path,
) -> Result<Directories, CoreError> {
    let root = data
        .open_child("restore-rollback")
        .map_err(map_path_error)?;
    let requested_parent = rollback_directory.parent().ok_or(CoreError::UnsafePath)?;
    if private_data_directory(requested_parent, false)?.identity != root.identity {
        return Err(CoreError::UnsafePath);
    }
    let held = root
        .open_child(
            rollback_directory
                .file_name()
                .ok_or(CoreError::UnsafePath)?,
        )
        .map_err(map_path_error)?;
    if private_data_directory(&held.canonical, false)?.identity != held.identity {
        return Err(CoreError::UnsafePath);
    }
    Ok(held)
}

fn current_files(data: &Directories) -> Result<Vec<(String, &'static str)>, CoreError> {
    let mut files = Vec::new();
    for suffix in ["", "-wal", "-shm", "-journal"] {
        let name = format!("{PAYLOAD_FILE_NAME}{suffix}");
        let exists = data
            .sqlite_file_identity(&name)
            .map_err(map_path_error)?
            .is_some();
        if exists {
            files.push((name, suffix));
        }
    }
    Ok(files)
}

fn short_id(value: &str) -> String {
    value
        .chars()
        .take(8)
        .filter(|character| character.is_ascii_alphanumeric())
        .collect()
}

fn verify_database_health(path: &Path) -> Result<(), CoreError> {
    let report = RestoreService::verify_restored_database(path)?;
    if report.ok {
        Ok(())
    } else {
        Err(CoreError::Restore("数据库完整性检查未通过".to_owned()))
    }
}

fn write_journal(data_directory: &Path, journal: &RestoreJournal) -> Result<(), CoreError> {
    atomic_replace_file(
        &data_directory.join(JOURNAL_FILE_NAME),
        &journal_bytes(journal)?,
    )
}

fn journal_bytes(journal: &RestoreJournal) -> Result<Vec<u8>, CoreError> {
    serde_json::to_vec_pretty(journal)
        .map_err(|_| CoreError::Restore("无法序列化恢复日志".to_owned()))
}

fn summarize_connection(
    conn: &Connection,
    database_size_bytes: u64,
) -> Result<RestoreDataSummary, CoreError> {
    let projects = count(conn, "projects")?;
    let archived_projects = count_where(conn, "projects", "archived_at IS NOT NULL")?;
    let tasks = count(conn, "tasks")?;
    let done_tasks = count_where(conn, "tasks", "status = 'done'")?;
    let criteria = count(conn, "acceptance_criteria")?;
    let verification_runs = count(conn, "verification_runs")?;
    let evidence = count(conn, "evidence")?;
    let claude_events = count(conn, "claude_events")?;
    let memories = count(conn, "project_memories")?;
    let decisions = count(conn, "decisions")?;
    let recovery_snapshots = count(conn, "recovery_snapshots")?;
    let running_verification_count =
        count_where(conn, "verification_runs", "execution_status = 'running'")?;
    let desired_hook_project_count = conn.query_row(
        "SELECT COUNT(*) FROM claude_integrations WHERE desired_enabled = 1",
        [],
        |row| row.get(0),
    )?;
    let last_updated_at = conn.query_row(
        "SELECT MAX(value) FROM (
            SELECT MAX(updated_at) AS value FROM projects
            UNION ALL
            SELECT MAX(updated_at) AS value FROM tasks
         )",
        [],
        |row| row.get::<_, Option<String>>(0),
    )?;

    let project_names = {
        let mut statement =
            conn.prepare("SELECT name FROM projects ORDER BY updated_at DESC, id DESC LIMIT 5")?;
        let rows = statement.query_map([], |row| row.get::<_, String>(0))?;
        rows.collect::<Result<Vec<_>, _>>()?
    };

    let missing_project_root_count = {
        let mut statement = conn.prepare("SELECT root_path FROM projects")?;
        let rows = statement.query_map([], |row| row.get::<_, String>(0))?;
        let mut missing = 0_i64;
        for row in rows {
            let root = row?;
            if !Path::new(&root).is_dir() {
                missing += 1;
            }
        }
        missing
    };

    Ok(RestoreDataSummary {
        projects,
        archived_projects,
        tasks,
        done_tasks,
        criteria,
        verification_runs,
        evidence,
        claude_events,
        memories,
        decisions,
        recovery_snapshots,
        project_names,
        missing_project_root_count,
        running_verification_count,
        desired_hook_project_count,
        database_size_bytes,
        last_updated_at,
    })
}

fn count(conn: &Connection, table: &str) -> Result<i64, CoreError> {
    Ok(
        conn.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
            row.get(0)
        })?,
    )
}

fn count_where(conn: &Connection, table: &str, predicate: &str) -> Result<i64, CoreError> {
    Ok(conn.query_row(
        &format!("SELECT COUNT(*) FROM {table} WHERE {predicate}"),
        [],
        |row| row.get(0),
    )?)
}

fn base_warnings() -> Vec<String> {
    vec![
        "备份未加密，包含项目路径、任务、验证记录摘要、长期记忆与决策；请妥善保管，不要上传到不受信任的位置。"
            .to_owned(),
        "SHA-256 只证明文件内容与清单一致，不能证明备份来源可信；只恢复自己创建或明确信任的备份。"
            .to_owned(),
        "正式恢复会替换当前数据库。切换前会创建本地回退副本；切换期间请勿关闭应用。"
            .to_owned(),
    ]
}

fn base_effects(manifest: &BackupManifest) -> Vec<String> {
    vec![
        "当前数据库将被备份中的数据库替换；当前数据库会先保存为一致性回退副本。".to_owned(),
        "备份中的项目、任务、验收条件、验证记录、证据摘要、长期记忆、决策与恢复快照将按备份时点恢复。".to_owned(),
        format!(
            "备份创建时间：{}；备份 Schema：v{}；格式版本：v{}。",
            manifest.created_at, manifest.schema_version, manifest.format_version
        ),
        "Claude Code Hooks 配置文件、Hook Spool 与伪名化 Salt 不在替换范围内；恢复后必须人工重新检查接入状态。".to_owned(),
    ]
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

fn file_len(path: &Path) -> u64 {
    fs::metadata(path)
        .map(|metadata| metadata.len())
        .unwrap_or(0)
}

fn interrupted_name() -> String {
    let stamp = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0);
    format!("interrupted-{stamp}-xushu.db")
}

fn cleanup_owned_workspace(directory: &Path, expected_identity: &str) {
    let Some((parent, name)) = directory.parent().zip(directory.file_name()) else {
        return;
    };
    let Ok(parent) = private_data_directory(parent, false) else {
        return;
    };
    let Ok(directory) = parent.open_child(name) else {
        return;
    };
    if directory.identity == expected_identity {
        cleanup_workspace(&parent, name, directory);
    }
}

fn cleanup_workspace(parent: &Directories, name: &std::ffi::OsStr, directory: Directories) {
    for file in [
        PAYLOAD_FILE_NAME,
        "xushu.db-wal",
        "xushu.db-shm",
        "xushu.db-journal",
        MANIFEST_FILE_NAME,
    ] {
        let _ = directory.remove_file(file);
    }
    if parent
        .open_child(name)
        .is_ok_and(|current| current.identity == directory.identity)
    {
        drop(directory);
        let _ = parent.remove_child_dir(name);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn confirmation_requires_exact_text_and_acknowledgement() {
        assert!(
            RestoreService::validates_confirmation(&PerformRestoreInput {
                token: "t".to_owned(),
                confirmation_text: "恢复".to_owned(),
                acknowledged: true,
            })
            .is_ok()
        );
        assert!(
            RestoreService::validates_confirmation(&PerformRestoreInput {
                token: "t".to_owned(),
                confirmation_text: "恢复".to_owned(),
                acknowledged: false,
            })
            .is_err()
        );
        assert!(
            RestoreService::validates_confirmation(&PerformRestoreInput {
                token: "t".to_owned(),
                confirmation_text: "确认".to_owned(),
                acknowledged: true,
            })
            .is_err()
        );
    }

    #[test]
    fn cleanup_workspaces_ignores_unknown_entries() {
        let directory = tempfile::TempDir::new().expect("temp dir");
        let roots = directory.path().join("restore-workspaces");
        fs::create_dir(&roots).expect("create root");
        let owned = roots.join(Uuid::new_v4().to_string());
        fs::create_dir(&owned).expect("create owned");
        fs::write(owned.join(PAYLOAD_FILE_NAME), b"x").expect("write payload");
        let unknown = roots.join("unknown-directory");
        fs::create_dir(&unknown).expect("create unknown");
        fs::write(unknown.join("keep.txt"), b"keep").expect("write unknown");
        let verify_roots = directory.path().join("verify-workspaces");
        fs::create_dir(&verify_roots).expect("create verify root");
        let verify_owned = verify_roots.join(Uuid::new_v4().to_string());
        fs::create_dir(&verify_owned).expect("create verify owned");
        fs::write(verify_owned.join(PAYLOAD_FILE_NAME), b"x").expect("write verify payload");
        cleanup_workspaces(directory.path()).expect("cleanup");
        assert!(!owned.exists());
        assert!(!verify_owned.exists());
        assert!(unknown.exists());
    }

    #[cfg(unix)]
    #[test]
    fn restore_boundaries_reject_redirected_workspaces_and_outside_rollback() {
        use std::os::unix::fs::symlink;

        let fixture = tempfile::TempDir::new().unwrap();
        let data = fixture.path().join("data");
        let workspaces = data.join("restore-workspaces");
        let rollback = data.join("restore-rollback");
        let outside = fixture.path().join("outside");
        fs::create_dir_all(&workspaces).unwrap();
        fs::create_dir_all(&rollback).unwrap();
        fs::create_dir(&outside).unwrap();
        fs::write(outside.join(PAYLOAD_FILE_NAME), b"keep").unwrap();
        symlink(&outside, workspaces.join(Uuid::new_v4().to_string())).unwrap();
        symlink(&outside, rollback.join("redirected")).unwrap();

        cleanup_workspaces(&data).unwrap();
        let guard = secure_directory(&data).unwrap();
        assert!(matches!(
            validate_rollback_directory(&guard, &outside),
            Err(CoreError::UnsafePath)
        ));
        assert!(matches!(
            validate_rollback_directory(&guard, &rollback.join("redirected")),
            Err(CoreError::UnsafePath)
        ));
        assert_eq!(fs::read(outside.join(PAYLOAD_FILE_NAME)).unwrap(), b"keep");
    }

    #[test]
    fn journal_round_trip_and_startup_abandoned_notice() {
        let directory = tempfile::TempDir::new().expect("temp dir");
        let current = directory.path().join(PAYLOAD_FILE_NAME);
        let journal = RestoreJournal {
            restore_id: Uuid::new_v4().to_string(),
            backup_id: Uuid::new_v4().to_string(),
            backup_directory: "D:\\backup".to_owned(),
            rollback_directory: directory
                .path()
                .join("restore-rollback")
                .join("op")
                .to_string_lossy()
                .to_string(),
            rollback_database: directory
                .path()
                .join("restore-rollback")
                .join("op")
                .join(ROLLBACK_FILE_NAME)
                .to_string_lossy()
                .to_string(),
            current_database: current.to_string_lossy().to_string(),
            isolated_sha256: "a".repeat(64),
            stage: "prepared".to_owned(),
            started_at: now_utc_rfc3339(),
            updated_at: now_utc_rfc3339(),
        };
        write_journal(directory.path(), &journal).expect("write journal");
        let missing = recover_interrupted_restore(directory.path())
            .expect("check missing live database")
            .expect("notice");
        assert_eq!(missing.outcome, "needs_manual");
        assert!(!current.exists());
        assert!(directory.path().join(JOURNAL_FILE_NAME).is_file());

        drop(Database::open(&current).expect("create fixture database"));
        let notice = recover_interrupted_restore(directory.path())
            .expect("recover")
            .expect("notice");
        assert_eq!(notice.outcome, "abandoned");
        assert!(!directory.path().join(JOURNAL_FILE_NAME).exists());
    }
}
