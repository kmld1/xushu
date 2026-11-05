use std::fs;
use std::path::Path;
use std::sync::Mutex;

use xushu_core::{CoreError, Database};

use crate::error::ApiError;

/// 应用级状态。数据库初始化失败时仍允许窗口启动，并通过 IPC 返回可理解的
/// `DATABASE_UNAVAILABLE` 错误，避免无提示闪退。
pub struct AppState {
    database: Mutex<Option<Database>>,
    startup_error: Option<ApiError>,
}

impl AppState {
    pub fn initialize(data_directory: &Path) -> Self {
        let result = fs::create_dir_all(data_directory)
            .map_err(CoreError::from)
            .and_then(|()| Database::open(data_directory.join("xushu.db")));

        match result {
            Ok(database) => Self {
                database: Mutex::new(Some(database)),
                startup_error: None,
            },
            Err(error) => {
                eprintln!(
                    "[xushu] database initialization failed: code={}",
                    error.code()
                );
                Self {
                    database: Mutex::new(None),
                    startup_error: Some(ApiError::new(
                        "DATABASE_UNAVAILABLE",
                        "本地数据库初始化失败，请检查应用数据目录权限后重启应用",
                    )),
                }
            }
        }
    }

    pub fn with_database<T>(
        &self,
        operation: impl FnOnce(&Database) -> Result<T, CoreError>,
    ) -> Result<T, ApiError> {
        if let Some(error) = &self.startup_error {
            return Err(error.clone());
        }

        let guard = self.database.lock().map_err(|_| {
            ApiError::new("DATABASE_LOCK_ERROR", "本地数据库连接暂时不可用，请重试")
        })?;

        let database = guard.as_ref().ok_or_else(|| {
            ApiError::new("DATABASE_UNAVAILABLE", "本地数据库尚未就绪，请重启应用")
        })?;

        operation(database).map_err(ApiError::from_core)
    }
}
