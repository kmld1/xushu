use serde::Serialize;
use xushu_core::CoreError;

/// 可安全跨 IPC 返回的错误。消息不包含绝对路径、SQL 或凭据。
#[derive(Debug, Clone, Serialize)]
pub struct ApiError {
    pub code: String,
    pub message: String,
}

impl ApiError {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
        }
    }

    pub fn from_core(error: CoreError) -> Self {
        let code = error.code();

        match error {
            CoreError::Validation(message) => Self::new("VALIDATION_ERROR", message),
            CoreError::NotFound => Self::new("NOT_FOUND", "未找到指定记录"),
            CoreError::Conflict(message) => Self::new("CONFLICT", message),
            CoreError::InvalidTransition { from, to } => {
                Self::new("INVALID_TRANSITION", format!("不允许从 {from} 转换到 {to}"))
            }
            CoreError::TaskNotCompletable(message) => Self::new("TASK_NOT_COMPLETABLE", message),
            CoreError::Git(_) => Self::new(
                "GIT_ERROR",
                "Git 状态检查失败，请确认项目路径仍存在且是有效的 Git 工作树",
            ),
            CoreError::Unavailable => Self::new("DATABASE_UNAVAILABLE", "本地数据库尚未就绪"),
            CoreError::LockPoisoned => {
                Self::new("DATABASE_LOCK_ERROR", "本地数据库连接暂时不可用，请重试")
            }
            CoreError::SchemaTooNew { .. } => Self::new(
                "SCHEMA_TOO_NEW",
                "本地数据库版本高于当前应用支持的版本，请升级应用",
            ),
            CoreError::Database(_) | CoreError::Io(_) | CoreError::InvalidMigration(_) => {
                eprintln!("[xushu] internal database error: code={code}");
                Self::new("DATABASE_ERROR", "本地数据库操作失败，请稍后重试")
            }
        }
    }
}
