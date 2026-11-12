use std::io;

/// 核心领域层错误。
///
/// 这里保留足够的技术细节供 Rust 测试与诊断使用；跨 Tauri IPC 时由桌面层
/// 转换为不含路径、SQL 或凭据的结构化错误。
#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    #[error("数据库错误: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("文件系统错误: {0}")]
    Io(#[from] io::Error),

    #[error("数据库尚未就绪")]
    Unavailable,

    #[error("数据库连接锁已损坏")]
    LockPoisoned,

    #[error("{0}")]
    Validation(String),

    #[error("未找到指定记录")]
    NotFound,

    #[error("资源冲突: {0}")]
    Conflict(String),

    #[error("非法状态转换: {from} -> {to}")]
    InvalidTransition { from: String, to: String },

    #[error("任务尚不能完成：{0}")]
    TaskNotCompletable(String),

    #[error("Git 检查失败: {0}")]
    Git(String),

    #[error("需要用户明确批准后才能执行验证")]
    ApprovalRequired,

    #[error("验证运行错误: {0}")]
    Verification(String),

    #[error("Claude Code 接入错误: {0}")]
    Claude(String),

    #[error("数据库 schema 版本 {found} 高于当前应用支持的 {supported}")]
    SchemaTooNew { found: i64, supported: i64 },

    #[error("迁移配置无效: {0}")]
    InvalidMigration(String),
}

impl CoreError {
    /// 稳定的错误代码，供 IPC 与日志使用。
    pub fn code(&self) -> &'static str {
        match self {
            Self::Database(_) => "DATABASE_ERROR",
            Self::Io(_) => "IO_ERROR",
            Self::Unavailable => "DATABASE_UNAVAILABLE",
            Self::LockPoisoned => "DATABASE_LOCK_ERROR",
            Self::Validation(_) => "VALIDATION_ERROR",
            Self::NotFound => "NOT_FOUND",
            Self::Conflict(_) => "CONFLICT",
            Self::InvalidTransition { .. } => "INVALID_TRANSITION",
            Self::TaskNotCompletable(_) => "TASK_NOT_COMPLETABLE",
            Self::Git(_) => "GIT_ERROR",
            Self::ApprovalRequired => "APPROVAL_REQUIRED",
            Self::Verification(_) => "VERIFICATION_ERROR",
            Self::Claude(_) => "CLAUDE_INTEGRATION_ERROR",
            Self::SchemaTooNew { .. } => "SCHEMA_TOO_NEW",
            Self::InvalidMigration(_) => "MIGRATION_CONFIG_ERROR",
        }
    }
}
