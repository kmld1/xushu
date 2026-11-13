//! 数据保护中心的数据资产清单、占用统计与隐私边界说明。

use std::fs;
use std::path::Path;

use chrono::Utc;
use rusqlite::Connection;
use serde::Serialize;

use crate::claude::path_guard::redirected;
use crate::claude::ClaudePrivacySnapshot;
use crate::db::{now_utc_rfc3339, Database, DatabaseHealth, MIGRATIONS};
use crate::error::CoreError;

use super::integrity::DataCounts;
use super::paths::{display_path, free_space_bytes};
use super::restore::RestoreStartupNotice;
use super::service::{database_file_sizes, is_due, BackupPolicy, BackupRecord, BackupService};

const RESTORE_HISTORY_LIMIT: i64 = 10;
const EXPORT_SCAN_LIMIT: usize = 2_000;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DataAsset {
    pub id: String,
    pub label: String,
    pub location: String,
    pub authoritative: bool,
    pub regenerable: bool,
    pub sensitive: bool,
    pub included_in_backup: bool,
    pub export_available: bool,
    pub export_note: String,
    pub special_recovery: bool,
    pub retention: String,
    pub impact_if_lost: String,
    pub count_label: String,
    pub size_bytes: Option<u64>,
    pub notes: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupEncryptionInfo {
    pub encrypted: bool,
    pub supported: bool,
    pub note: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RetentionSummary {
    pub hook_event_days: i64,
    pub hook_max_events_per_project: i64,
    pub hook_max_sessions_per_project: i64,
    pub recovery_snapshots_per_project: i64,
    pub backup_retention_count: i64,
    pub restore_workspaces: String,
    pub rollback_archives: String,
    pub backup_files: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HooksPrivacyOverview {
    pub event_count: i64,
    pub session_count: i64,
    pub integration_count: i64,
    pub desired_enabled_project_count: i64,
    pub pending_spool_events: usize,
    pub pending_spool_bytes: u64,
    pub salt_present: bool,
    pub salt_bytes: u64,
    pub inbox_available: bool,
    pub retention_days: i64,
    pub max_events_per_project: i64,
    pub collected_fields: Vec<String>,
    pub excluded_fields: Vec<String>,
    pub note: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RestoreOperationSummary {
    pub id: String,
    pub backup_id: String,
    pub backup_directory: String,
    pub started_at: String,
    pub finished_at: Option<String>,
    pub outcome: String,
    pub switch_verified: bool,
    pub rollback_status: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DataProtectionOverview {
    pub generated_at: String,
    pub app_version: String,
    pub schema_version: i64,
    pub data_directory: String,
    pub database: DatabaseHealth,
    pub database_size_bytes: u64,
    pub wal_size_bytes: u64,
    pub shm_size_bytes: u64,
    pub counts: DataCounts,
    pub backups: Vec<BackupRecord>,
    pub last_backup: Option<BackupRecord>,
    pub last_verified_backup: Option<BackupRecord>,
    pub backup_total_size_bytes: u64,
    pub policy: BackupPolicy,
    pub policy_target_free_bytes: Option<u64>,
    pub automatic_due: bool,
    pub restore_operations: Vec<RestoreOperationSummary>,
    pub startup_notice: Option<RestoreStartupNotice>,
    pub inventory: Vec<DataAsset>,
    pub hooks: HooksPrivacyOverview,
    pub retention: RetentionSummary,
    pub encryption: BackupEncryptionInfo,
    pub warnings: Vec<String>,
}

pub struct DataProtectionService<'a> {
    database: &'a Database,
    data_directory: &'a Path,
}

impl<'a> DataProtectionService<'a> {
    pub fn new(database: &'a Database, data_directory: &'a Path) -> Self {
        Self {
            database,
            data_directory,
        }
    }

    pub fn overview(
        &self,
        startup_notice: Option<RestoreStartupNotice>,
        hooks_runtime: ClaudePrivacySnapshot,
    ) -> Result<DataProtectionOverview, CoreError> {
        let health = self.database.health()?;
        let (main_size, wal_size, shm_size) = database_file_sizes(self.database.path());
        let backup_service = BackupService::new(self.database, self.data_directory);
        let backups = backup_service.list()?;
        let last_backup = backups.first().cloned();
        let last_verified_backup = backups
            .iter()
            .find(|record| {
                record.integrity_status == super::service::BackupIntegrityStatus::Verified
            })
            .cloned();
        let backup_total_size_bytes = backups
            .iter()
            .map(|record| record.data_size_bytes + record.manifest_size_bytes)
            .sum();
        let policy = backup_service.policy()?;
        let policy_target_free_bytes = policy
            .target_directory
            .as_deref()
            .filter(|target| Path::new(target).is_dir())
            .and_then(|target| free_space_bytes(Path::new(target)));
        let automatic_due = policy.automatic_enabled && is_due(&policy, Utc::now());

        let conn = self.database.lock()?;
        let counts = overview_counts(&conn)?;
        let restore_operations = load_restore_operations(&conn)?;
        let integration_count = count(&conn, "claude_integrations")?;
        let desired_enabled_project_count = conn.query_row(
            "SELECT COUNT(*) FROM claude_integrations WHERE desired_enabled = 1",
            [],
            |row| row.get(0),
        )?;
        drop(conn);

        let export_scan = scan_exports(&self.data_directory.join("exports"));
        let data_directory = display_path(
            &fs::canonicalize(self.data_directory)
                .unwrap_or_else(|_| self.data_directory.to_path_buf()),
        );

        let mut warnings = Vec::new();
        if backups.is_empty() {
            warnings.push("尚未创建数据库备份；本机故障可能导致不可恢复的数据丢失。".to_owned());
        } else if last_verified_backup.is_none() {
            warnings.push("已有备份记录，但没有任何备份通过完整性验证。".to_owned());
        }
        if let Some(record) = &last_backup {
            if record.integrity_status == super::service::BackupIntegrityStatus::Missing {
                warnings.push("最近一次备份目录不存在或被移动，恢复前必须重新定位。".to_owned());
            } else if record.integrity_status == super::service::BackupIntegrityStatus::Failed {
                warnings.push("最近一次备份未通过完整性验证，不能视为可靠备份。".to_owned());
            }
        }
        if let Some(code) = &policy.last_automatic_error_code {
            warnings.push(format!(
                "自动备份最近失败（错误代码 {code}）；旧备份未被自动删除，请检查目标目录与磁盘空间。"
            ));
        }
        if let Some(notice) = &startup_notice {
            warnings.push(format!("启动恢复检查：{}", notice.message));
        }
        if restore_operations
            .first()
            .is_some_and(|operation| operation.rollback_status == "failed")
        {
            warnings
                .push("最近一次恢复的回退状态为失败；请保留回退目录并进行人工检查。".to_owned());
        }
        warnings.push(
            "Phase 6 备份不加密。备份文件包含本地项目路径与用户录入内容，请不要上传到不受信任的位置。"
                .to_owned(),
        );

        let retention = RetentionSummary {
            hook_event_days: crate::claude::EVENT_RETENTION_DAYS,
            hook_max_events_per_project: crate::claude::MAX_EVENTS_PER_PROJECT,
            hook_max_sessions_per_project: crate::claude::MAX_SESSIONS_PER_PROJECT,
            recovery_snapshots_per_project: 20,
            backup_retention_count: policy.retention_count,
            restore_workspaces:
                "隔离恢复临时目录在应用启动时清理；只删除应用创建且仅含已知文件的目录。".to_owned(),
            rollback_archives: "恢复回退副本不自动删除，保留供人工核查。".to_owned(),
            backup_files:
                "仅自动清理已登记、清单匹配且仅含已知文件的 automatic 备份；manual 备份不自动删除。"
                    .to_owned(),
        };

        let hooks = HooksPrivacyOverview {
            event_count: counts.claude_events,
            session_count: counts.claude_sessions,
            integration_count,
            desired_enabled_project_count,
            pending_spool_events: hooks_runtime.pending_event_count,
            pending_spool_bytes: hooks_runtime.pending_bytes,
            salt_present: hooks_runtime.salt_present,
            salt_bytes: hooks_runtime.salt_bytes,
            inbox_available: hooks_runtime.inbox_available,
            retention_days: crate::claude::EVENT_RETENTION_DAYS,
            max_events_per_project: crate::claude::MAX_EVENTS_PER_PROJECT,
            collected_fields: vec![
                "事件类型与时间".to_owned(),
                "工具名称与工具类别".to_owned(),
                "带盐伪名化会话标识".to_owned(),
                "工具成功/失败状态与时长".to_owned(),
                "项目标识与去重标识".to_owned(),
            ],
            excluded_fields: vec![
                "用户原始 Prompt".to_owned(),
                "Claude Code transcript".to_owned(),
                "完整工具输入与输出".to_owned(),
                "命令原文与文件内容".to_owned(),
                "API Key、Token、密码与环境变量".to_owned(),
            ],
            note: "Hook 事件只是观察记录，不是验收证据，也不会更新任务完成门禁。".to_owned(),
        };

        let inventory = build_inventory(
            &counts,
            &hooks_runtime,
            integration_count,
            backups.len(),
            backup_total_size_bytes,
            export_scan.count,
            export_scan.size_bytes,
            main_size + wal_size + shm_size,
            policy.retention_count,
        );

        let encryption = BackupEncryptionInfo {
            encrypted: false,
            supported: false,
            note: "当前备份为未加密的本地 SQLite 快照。校验和不能防止他人在获得文件后读取内容；后续若增加加密，必须先有密钥管理与恢复 ADR。"
                .to_owned(),
        };

        Ok(DataProtectionOverview {
            generated_at: now_utc_rfc3339(),
            app_version: env!("CARGO_PKG_VERSION").to_owned(),
            schema_version: MIGRATIONS.last().map_or(0, |migration| migration.version),
            data_directory,
            database: health,
            database_size_bytes: main_size,
            wal_size_bytes: wal_size,
            shm_size_bytes: shm_size,
            counts,
            backups,
            last_backup,
            last_verified_backup,
            backup_total_size_bytes,
            policy,
            policy_target_free_bytes,
            automatic_due,
            restore_operations,
            startup_notice,
            inventory,
            hooks,
            retention,
            encryption,
            warnings,
        })
    }
}

fn overview_counts(conn: &Connection) -> Result<DataCounts, CoreError> {
    Ok(DataCounts {
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
    })
}

fn count(conn: &Connection, table: &str) -> Result<i64, CoreError> {
    Ok(
        conn.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
            row.get(0)
        })?,
    )
}

fn load_restore_operations(conn: &Connection) -> Result<Vec<RestoreOperationSummary>, CoreError> {
    let mut statement = conn.prepare(
        "SELECT id, backup_id, backup_directory, started_at, finished_at, outcome,
                switch_verified, rollback_status
         FROM restore_operations ORDER BY started_at DESC LIMIT ?1",
    )?;
    let rows = statement.query_map([RESTORE_HISTORY_LIMIT], |row| {
        Ok(RestoreOperationSummary {
            id: row.get(0)?,
            backup_id: row.get(1)?,
            backup_directory: row.get(2)?,
            started_at: row.get(3)?,
            finished_at: row.get(4)?,
            outcome: row.get(5)?,
            switch_verified: row.get::<_, i64>(6)? == 1,
            rollback_status: row.get(7)?,
        })
    })?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

struct ExportScan {
    count: usize,
    size_bytes: u64,
}

fn scan_exports(directory: &Path) -> ExportScan {
    let mut count = 0_usize;
    let mut size_bytes = 0_u64;
    let Ok(entries) = fs::read_dir(directory) else {
        return ExportScan { count, size_bytes };
    };
    for entry in entries.flatten().take(EXPORT_SCAN_LIMIT) {
        let path = entry.path();
        if let Ok(metadata) = fs::symlink_metadata(&path) {
            if metadata.is_file() && !redirected(&metadata) {
                count += 1;
                size_bytes += metadata.len();
            }
        }
    }
    ExportScan { count, size_bytes }
}

#[allow(clippy::too_many_arguments)]
fn build_inventory(
    counts: &DataCounts,
    hooks_runtime: &ClaudePrivacySnapshot,
    integration_count: i64,
    backup_count: usize,
    backup_size_bytes: u64,
    export_count: usize,
    export_size_bytes: u64,
    database_size_bytes: u64,
    backup_retention_count: i64,
) -> Vec<DataAsset> {
    let db_location = "应用数据目录 / xushu.db";
    vec![
        DataAsset {
            id: "projects_tasks".to_owned(),
            label: "项目与任务".to_owned(),
            location: db_location.to_owned(),
            authoritative: true,
            regenerable: false,
            sensitive: true,
            included_in_backup: true,
            export_available: true,
            export_note: "通过一致性备份导出；没有通用 SQL 或全库明文导出入口。".to_owned(),
            special_recovery: false,
            retention: "保留至用户显式归档或删除；归档不删除历史记录。".to_owned(),
            impact_if_lost: "项目路径、任务、状态与验收门禁上下文丢失。".to_owned(),
            count_label: format!("{} 项目 · {} 任务", counts.projects, counts.tasks),
            size_bytes: None,
            notes: "包含本地路径与用户输入，可能含敏感内容。".to_owned(),
        },
        DataAsset {
            id: "acceptance_state".to_owned(),
            label: "验收条件与状态历史".to_owned(),
            location: db_location.to_owned(),
            authoritative: true,
            regenerable: false,
            sensitive: false,
            included_in_backup: true,
            export_available: true,
            export_note: "验收报告可导出为 Markdown；备份包含完整关系。".to_owned(),
            special_recovery: false,
            retention: "随任务保留；没有自动删除。".to_owned(),
            impact_if_lost: "任务完成门禁与状态历史无法追溯。".to_owned(),
            count_label: format!("{} 条验收条件", counts.acceptance_criteria),
            size_bytes: None,
            notes: "不能通过 AI 文本或 Hook 事件重建。".to_owned(),
        },
        DataAsset {
            id: "verification_evidence".to_owned(),
            label: "验证记录与证据".to_owned(),
            location: db_location.to_owned(),
            authoritative: true,
            regenerable: false,
            sensitive: true,
            included_in_backup: true,
            export_available: true,
            export_note: "验收报告 Sheet 可复制/导出 Markdown；备份包含脱敏输出摘要。".to_owned(),
            special_recovery: false,
            retention: "随任务保留；过期证据仍保留并标记 STALE。".to_owned(),
            impact_if_lost: "无法证明任务在特定代码状态与时间下通过验收。".to_owned(),
            count_label: format!(
                "{} 次运行 · {} 条检查 · {} 份证据",
                counts.verification_runs, counts.verification_checks, counts.evidence
            ),
            size_bytes: None,
            notes: "输出已按白名单模式脱敏，但仍可能含路径与测试文本；不是绝对无敏感信息。"
                .to_owned(),
        },
        DataAsset {
            id: "claude_integration_state".to_owned(),
            label: "Claude Code 集成本地状态".to_owned(),
            location: db_location.to_owned(),
            authoritative: false,
            regenerable: true,
            sensitive: false,
            included_in_backup: true,
            export_available: false,
            export_note: "仅用于本地接入状态；不提供独立导出。".to_owned(),
            special_recovery: true,
            retention: "随项目保留；卸载后保留历史状态。".to_owned(),
            impact_if_lost: "XUSHU 忘记期望的启用状态；不会修改真实项目 Hooks 配置。".to_owned(),
            count_label: format!("{integration_count} 个项目有接入记录"),
            size_bytes: None,
            notes: "真实配置在项目 .claude/settings.local.json；恢复数据库不会改动它。".to_owned(),
        },
        DataAsset {
            id: "claude_project_hooks".to_owned(),
            label: "Claude Code 项目 Hooks 配置".to_owned(),
            location: "项目仓库 / .claude/settings.local.json（由项目自身持有）".to_owned(),
            authoritative: true,
            regenerable: true,
            sensitive: true,
            included_in_backup: false,
            export_available: false,
            export_note: "XUSHU 不导出项目配置文件。".to_owned(),
            special_recovery: true,
            retention: "由用户与 Claude Code 管理；XUSHU 不参与保留清理。".to_owned(),
            impact_if_lost: "Hook 采集停止；不影响 XUSHU 其他功能。".to_owned(),
            count_label: format!("{integration_count} 个项目有接入记录"),
            size_bytes: None,
            notes: "不进入数据库备份；恢复后不会自动重新安装或启用 Hooks。".to_owned(),
        },
        DataAsset {
            id: "claude_event_metadata".to_owned(),
            label: "Claude Code 事件元数据".to_owned(),
            location: db_location.to_owned(),
            authoritative: false,
            regenerable: false,
            sensitive: true,
            included_in_backup: true,
            export_available: false,
            export_note: "只在活动 Sheet 查看；不提供原始事件导出。".to_owned(),
            special_recovery: false,
            retention: format!(
                "最多 {} 天，每项目最多 {} 条事件、{} 个会话。",
                crate::claude::EVENT_RETENTION_DAYS,
                crate::claude::MAX_EVENTS_PER_PROJECT,
                crate::claude::MAX_SESSIONS_PER_PROJECT
            ),
            impact_if_lost: "失去观察历史；不影响任务验收证据。".to_owned(),
            count_label: format!(
                "{} 条事件 · {} 个会话",
                counts.claude_events, counts.claude_sessions
            ),
            size_bytes: None,
            notes: "只含白名单元数据与带盐伪名；不是完成证据。".to_owned(),
        },
        DataAsset {
            id: "hook_spool".to_owned(),
            label: "Hook 待导入 Spool".to_owned(),
            location: "应用数据目录 / hook-inbox / pending".to_owned(),
            authoritative: false,
            regenerable: false,
            sensitive: true,
            included_in_backup: false,
            export_available: false,
            export_note: "不导出；导入后应用会移除。".to_owned(),
            special_recovery: true,
            retention: "导入成功后移除；应用关闭期间可短暂缓冲。".to_owned(),
            impact_if_lost: "未导入的观察事件丢失；不影响任务与验收真相。".to_owned(),
            count_label: format!(
                "{} 条待导入 · {} 字节",
                hooks_runtime.pending_event_count, hooks_runtime.pending_bytes
            ),
            size_bytes: Some(hooks_runtime.pending_bytes),
            notes: "不属于数据库备份；恢复不会清理或重建 Spool。".to_owned(),
        },
        DataAsset {
            id: "pseudonym_salt".to_owned(),
            label: "伪名化 Salt".to_owned(),
            location: "应用数据目录 / hook-inbox".to_owned(),
            authoritative: false,
            regenerable: true,
            sensitive: true,
            included_in_backup: false,
            export_available: false,
            export_note: "绝不导出或显示内容。".to_owned(),
            special_recovery: true,
            retention: "随应用数据目录保留；删除后新事件无法与旧伪名关联。".to_owned(),
            impact_if_lost: "历史事件仍可读，但新会话哈希无法与旧会话哈希关联。".to_owned(),
            count_label: if hooks_runtime.salt_present {
                "已存在".to_owned()
            } else {
                "不存在".to_owned()
            },
            size_bytes: Some(hooks_runtime.salt_bytes),
            notes: "备份不包含 Salt；裁剪数据不承诺防篡改。".to_owned(),
        },
        DataAsset {
            id: "project_memories".to_owned(),
            label: "长期记忆与修订".to_owned(),
            location: db_location.to_owned(),
            authoritative: false,
            regenerable: false,
            sensitive: true,
            included_in_backup: true,
            export_available: false,
            export_note: "目前通过恢复包查看已确认记忆；没有独立导出入口。".to_owned(),
            special_recovery: true,
            retention: "归档保留历史；修订历史不删除。".to_owned(),
            impact_if_lost: "失去已确认开发上下文；不改变任务完成门禁。".to_owned(),
            count_label: format!(
                "{} 条记忆 · {} 条修订",
                counts.project_memories, counts.project_memory_revisions
            ),
            size_bytes: None,
            notes: "用户录入内容可能含秘密，正则脱敏无法保证全部识别。".to_owned(),
        },
        DataAsset {
            id: "decisions".to_owned(),
            label: "开发决策与修订".to_owned(),
            location: db_location.to_owned(),
            authoritative: false,
            regenerable: false,
            sensitive: false,
            included_in_backup: true,
            export_available: false,
            export_note: "目前通过恢复包查看已确认决策；没有独立导出入口。".to_owned(),
            special_recovery: true,
            retention: "追加式修订；已确认决策不可原地改写。".to_owned(),
            impact_if_lost: "失去决策依据与取代关系。".to_owned(),
            count_label: format!(
                "{} 条决策 · {} 条修订",
                counts.decisions, counts.decision_revisions
            ),
            size_bytes: None,
            notes: "决策不是事实，也不能替代验收证据。".to_owned(),
        },
        DataAsset {
            id: "recovery_snapshots".to_owned(),
            label: "恢复快照".to_owned(),
            location: db_location.to_owned(),
            authoritative: false,
            regenerable: true,
            sensitive: true,
            included_in_backup: true,
            export_available: false,
            export_note: "可复制恢复指令文本；不导出整份数据库。".to_owned(),
            special_recovery: true,
            retention: "每个项目保留最近 20 份。".to_owned(),
            impact_if_lost: "可从当前数据库重新生成；历史快照不可恢复。".to_owned(),
            count_label: format!("{} 份快照", counts.recovery_snapshots),
            size_bytes: None,
            notes: "包含本地路径与记忆副本；过期快照会明确标记 STALE。".to_owned(),
        },
        DataAsset {
            id: "backups".to_owned(),
            label: "备份文件".to_owned(),
            location: "用户选择的备份目录（可位于其他磁盘）".to_owned(),
            authoritative: false,
            regenerable: false,
            sensitive: true,
            included_in_backup: false,
            export_available: true,
            export_note: "备份本身就是可复制文件；未加密。".to_owned(),
            special_recovery: true,
            retention: format!(
                "当前策略保留 {backup_retention_count} 份自动备份；手动备份不自动删除。"
            ),
            impact_if_lost: "无法恢复备份时点之前的数据库状态。".to_owned(),
            count_label: format!("{backup_count} 份本地备份"),
            size_bytes: Some(backup_size_bytes),
            notes: "只登记 XUSHU 创建且清单可验证的目录；外部文件不会自动删除。".to_owned(),
        },
        DataAsset {
            id: "exports".to_owned(),
            label: "导出的验收报告".to_owned(),
            location: "应用数据目录 / exports".to_owned(),
            authoritative: false,
            regenerable: true,
            sensitive: true,
            included_in_backup: false,
            export_available: true,
            export_note: "报告文件可随时复制；可由数据库重新生成。".to_owned(),
            special_recovery: false,
            retention: "不自动清理；由用户自行管理。".to_owned(),
            impact_if_lost: "可从数据库重新导出。".to_owned(),
            count_label: format!("{export_count} 个文件"),
            size_bytes: Some(export_size_bytes),
            notes: "可能包含项目路径与脱敏后的测试输出。".to_owned(),
        },
        DataAsset {
            id: "ui_preferences".to_owned(),
            label: "界面偏好".to_owned(),
            location: "Tauri WebView2 本地存储（xushu-theme）".to_owned(),
            authoritative: false,
            regenerable: true,
            sensitive: false,
            included_in_backup: false,
            export_available: false,
            export_note: "不导出。".to_owned(),
            special_recovery: false,
            retention: "随 WebView 配置保留，直到用户清除。".to_owned(),
            impact_if_lost: "主题重置为跟随系统。".to_owned(),
            count_label: "主题设置".to_owned(),
            size_bytes: None,
            notes: "不包含业务事实。".to_owned(),
        },
        DataAsset {
            id: "logs".to_owned(),
            label: "日志与诊断".to_owned(),
            location: "未持久化到独立日志文件；仅 stderr 运行时诊断".to_owned(),
            authoritative: false,
            regenerable: true,
            sensitive: false,
            included_in_backup: false,
            export_available: false,
            export_note: "不提供日志导出。".to_owned(),
            special_recovery: false,
            retention: "进程退出即丢失。".to_owned(),
            impact_if_lost: "失去本次运行诊断线索。".to_owned(),
            count_label: "无持久化日志".to_owned(),
            size_bytes: None,
            notes: "诊断输出只包含错误代码与计数，不含路径、SQL 或敏感原文。".to_owned(),
        },
    ]
    .into_iter()
    .map(|mut asset| {
        if asset.id == "projects_tasks" {
            asset.size_bytes = Some(database_size_bytes);
        }
        asset
    })
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn inventory_marks_salt_and_spool_as_not_backed_up() {
        let directory = TempDir::new().expect("temp dir");
        let database = Database::open(directory.path().join("xushu.db")).expect("open");
        let runtime = ClaudePrivacySnapshot {
            pending_event_count: 3,
            pending_bytes: 300,
            salt_present: true,
            salt_bytes: 32,
            inbox_available: true,
        };
        let overview = DataProtectionService::new(&database, directory.path())
            .overview(None, runtime)
            .expect("overview");
        let spool = overview
            .inventory
            .iter()
            .find(|asset| asset.id == "hook_spool")
            .expect("spool asset");
        let salt = overview
            .inventory
            .iter()
            .find(|asset| asset.id == "pseudonym_salt")
            .expect("salt asset");
        assert!(!spool.included_in_backup);
        assert!(!salt.included_in_backup);
        assert!(salt.sensitive);
        assert_eq!(spool.count_label, "3 条待导入 · 300 字节");
        assert!(overview
            .warnings
            .iter()
            .any(|warning| warning.contains("尚未创建")));
        assert!(!overview.encryption.encrypted);
    }
}
