//! Phase 6：一致性备份、灾难恢复、数据完整性与隐私边界。
//!
//! 备份只包含 SQLite 数据库快照；Hook Spool、伪名化 Salt、项目 `.claude`
//! 配置、导出报告与 WebView 本地偏好不在备份范围内。恢复不会修改 Claude Code
//! Hooks 配置，也不会改变任务状态、验收证据或完成门禁。

mod integrity;
mod manifest;
mod overview;
mod paths;
mod restore;
mod service;

pub use integrity::{
    DataCounts, DataIntegrityReport, DataIntegrityService, IntegrityCheck, IntegrityStatus,
};
pub use manifest::{
    BackupFileEntry, BackupManifest, BackupOrigin, BACKUP_FORMAT_VERSION, MAX_BACKUP_BYTES,
    MAX_MANIFEST_BYTES, PAYLOAD_FILE_NAME,
};
pub use overview::{
    BackupEncryptionInfo, DataAsset, DataProtectionOverview, DataProtectionService,
    HooksPrivacyOverview, RestoreOperationSummary, RetentionSummary,
};
pub use paths::lock_data_directory;
pub use restore::{
    cleanup_workspaces, recover_interrupted_restore, PerformRestoreInput, RestoreDataChange,
    RestoreDataSummary, RestoreHooksReview, RestoreJournal, RestoreOutcome, RestorePlan,
    RestorePreflight, RestorePreflightInput, RestorePreflightOutcome, RestoreService,
    RestoreStartupNotice, RollbackReport, RESTORE_CONFIRMATION_TEXT, RESTORE_TOKEN_TTL_MINUTES,
};
pub use service::{
    AutomaticBackupOutcome, BackupIntegrityStatus, BackupPolicy, BackupRecord, BackupService,
    BackupVerification, CleanupOutcome, UpdateBackupPolicyInput,
};
