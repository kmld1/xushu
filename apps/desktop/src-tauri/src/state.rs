use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use xushu_core::{
    code_state, runner, AcceptanceReport, CoreError, Database, RunControl, StartVerificationInput,
    VerificationRun, VerificationService,
};

use crate::error::ApiError;

/// 应用级状态。数据库初始化失败时仍允许窗口启动，并通过 IPC 返回可理解的
/// `DATABASE_UNAVAILABLE` 错误，避免无提示闪退。
#[derive(Clone)]
pub struct AppState {
    database: Arc<Mutex<Option<Database>>>,
    startup_error: Option<ApiError>,
    controls: Arc<Mutex<HashMap<String, Arc<RunControl>>>>,
    exports_directory: Arc<PathBuf>,
}

impl AppState {
    pub fn initialize(data_directory: &Path) -> Self {
        let exports_directory = data_directory.join("exports");
        let result = fs::create_dir_all(data_directory)
            .map_err(CoreError::from)
            .and_then(|()| Database::open(data_directory.join("xushu.db")));

        match result {
            Ok(database) => {
                match VerificationService::new(&database).recover_interrupted() {
                    Ok(recovered) if recovered > 0 => {
                        eprintln!("[xushu] recovered {recovered} interrupted verification run(s)");
                    }
                    Err(error) => {
                        eprintln!(
                            "[xushu] verification recovery failed: code={}",
                            error.code()
                        );
                    }
                    _ => {}
                }
                Self {
                    database: Arc::new(Mutex::new(Some(database))),
                    startup_error: None,
                    controls: Arc::new(Mutex::new(HashMap::new())),
                    exports_directory: Arc::new(exports_directory),
                }
            }
            Err(error) => {
                eprintln!(
                    "[xushu] database initialization failed: code={}",
                    error.code()
                );
                Self {
                    database: Arc::new(Mutex::new(None)),
                    startup_error: Some(ApiError::new(
                        "DATABASE_UNAVAILABLE",
                        "本地数据库初始化失败，请检查应用数据目录权限后重启应用",
                    )),
                    controls: Arc::new(Mutex::new(HashMap::new())),
                    exports_directory: Arc::new(exports_directory),
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

    /// 准备并启动后台验证线程；数据库只写入 `running` 记录，进程在后台执行。
    pub fn start_verification(
        &self,
        input: StartVerificationInput,
    ) -> Result<VerificationRun, ApiError> {
        let prepared =
            self.with_database(|database| VerificationService::new(database).prepare_run(input))?;

        let control = RunControl::new();
        {
            let mut controls = self.controls.lock().map_err(|_| {
                ApiError::new("VERIFICATION_ERROR", "验证运行注册表暂时不可用，请重试")
            })?;
            controls.insert(prepared.run.id.clone(), control.clone());
        }

        let run = prepared.run.clone();
        let run_id = run.id.clone();
        let plan = prepared.plan.clone();
        let project_root = prepared.project_root.clone();
        let preflight_error = prepared.preflight_error.clone();
        let state = self.clone();

        std::thread::spawn(move || {
            let outcome = match preflight_error {
                Some(message) => runner::RunOutcome::start_failed(message),
                None => runner::execute(&plan, &control),
            };
            let end_code_state =
                code_state::capture(&project_root).map_err(|error| error.code().to_owned());
            let result = state.with_database(|database| {
                VerificationService::new(database).finalize_run(&run_id, outcome, end_code_state)
            });

            if let Err(error) = result {
                eprintln!("[xushu] verification finalize failed: code={}", error.code);
            }

            if let Ok(mut controls) = state.controls.lock() {
                controls.remove(&run_id);
            }
        });

        Ok(run)
    }

    pub fn cancel_verification(&self, run_id: &str) -> Result<(), ApiError> {
        let run =
            self.with_database(|database| VerificationService::new(database).get_run(run_id))?;
        if run.execution_status != "running" {
            return Err(ApiError::new("CONFLICT", "验证运行已经结束，无需取消"));
        }

        let control = self
            .controls
            .lock()
            .map_err(|_| ApiError::new("VERIFICATION_ERROR", "验证运行注册表暂时不可用"))?
            .get(run_id)
            .cloned();

        match control {
            Some(control) => {
                control.cancel();
                Ok(())
            }
            None => Err(ApiError::new(
                "CONFLICT",
                "未找到可取消的运行句柄；重启应用后该运行会被标记为中断",
            )),
        }
    }

    pub fn export_acceptance_report(&self, task_id: &str) -> Result<AcceptanceReport, ApiError> {
        self.with_database(|database| {
            VerificationService::new(database).export_report(task_id, &self.exports_directory)
        })
    }
}
