use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use xushu_core::{
    code_state, runner, AcceptanceReport, ClaudeActionInput, ClaudeActivitySnapshot,
    ClaudeHookPreview, ClaudeImportReport, ClaudeIntegrationService, ClaudeIntegrationStatus,
    ClaudeRuntimePaths, CoreError, Database, InstallClaudeHooksInput, RunControl,
    StartVerificationInput, VerificationRun, VerificationService,
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
    claude_runtime: Option<Arc<ClaudeRuntimePaths>>,
    claude_startup_error: Option<ApiError>,
    claude_mutation: Arc<Mutex<()>>,
}

impl AppState {
    pub fn initialize(data_directory: &Path) -> Self {
        let exports_directory = data_directory.join("exports");
        let hook_binary = resolve_hook_binary();
        let claude_runtime = match ClaudeRuntimePaths::prepare(
            data_directory,
            hook_binary,
            env!("CARGO_PKG_VERSION"),
        ) {
            Ok(runtime) => Some(Arc::new(runtime)),
            Err(error) => {
                eprintln!(
                    "[xushu] claude hook runtime initialization failed: code={}",
                    error.code()
                );
                None
            }
        };
        let claude_startup_error = if claude_runtime.is_none() {
            Some(ApiError::new(
                "CLAUDE_UNAVAILABLE",
                "Claude Code Hook 接收器初始化失败，请检查应用数据目录权限后重启应用",
            ))
        } else {
            None
        };

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
                if let Some(runtime) = &claude_runtime {
                    let reports =
                        ClaudeIntegrationService::new(&database, runtime).import_pending_all();
                    let imported: usize = reports.iter().map(|report| report.imported).sum();
                    if imported > 0 {
                        eprintln!("[xushu] imported {imported} pending Claude Code hook event(s)");
                    }
                }
                Self {
                    database: Arc::new(Mutex::new(Some(database))),
                    startup_error: None,
                    controls: Arc::new(Mutex::new(HashMap::new())),
                    exports_directory: Arc::new(exports_directory),
                    claude_runtime,
                    claude_startup_error,
                    claude_mutation: Arc::new(Mutex::new(())),
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
                    claude_runtime,
                    claude_startup_error,
                    claude_mutation: Arc::new(Mutex::new(())),
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

    fn claude_runtime(&self) -> Result<Arc<ClaudeRuntimePaths>, ApiError> {
        if let Some(error) = &self.claude_startup_error {
            return Err(error.clone());
        }
        self.claude_runtime.clone().ok_or_else(|| {
            ApiError::new(
                "CLAUDE_UNAVAILABLE",
                "Claude Code Hook 接收器尚未就绪，请重启应用",
            )
        })
    }

    pub fn get_claude_status(&self, project_id: &str) -> Result<ClaudeIntegrationStatus, ApiError> {
        let runtime = self.claude_runtime()?;
        self.with_database(|database| {
            ClaudeIntegrationService::new(database, &runtime).refresh(project_id)
        })
    }

    pub fn get_claude_activity(
        &self,
        project_id: &str,
    ) -> Result<ClaudeActivitySnapshot, ApiError> {
        let runtime = self.claude_runtime()?;
        self.with_database(|database| {
            ClaudeIntegrationService::new(database, &runtime).activity(project_id)
        })
    }

    pub fn preview_claude_hooks(&self, project_id: &str) -> Result<ClaudeHookPreview, ApiError> {
        let runtime = self.claude_runtime()?;
        self.with_database(|database| {
            ClaudeIntegrationService::new(database, &runtime).preview(project_id)
        })
    }

    pub fn install_claude_hooks(
        &self,
        input: InstallClaudeHooksInput,
    ) -> Result<ClaudeIntegrationStatus, ApiError> {
        let runtime = self.claude_runtime()?;
        let _guard = self.claude_mutation.lock().map_err(|_| {
            ApiError::new(
                "CLAUDE_UNAVAILABLE",
                "Claude Code 接入状态暂时不可用，请重试",
            )
        })?;
        self.with_database(|database| {
            ClaudeIntegrationService::new(database, &runtime).install(input)
        })
    }

    pub fn disable_claude_hooks(
        &self,
        input: ClaudeActionInput,
    ) -> Result<ClaudeIntegrationStatus, ApiError> {
        let runtime = self.claude_runtime()?;
        let _guard = self.claude_mutation.lock().map_err(|_| {
            ApiError::new(
                "CLAUDE_UNAVAILABLE",
                "Claude Code 接入状态暂时不可用，请重试",
            )
        })?;
        self.with_database(|database| {
            ClaudeIntegrationService::new(database, &runtime).disable(input)
        })
    }

    pub fn uninstall_claude_hooks(
        &self,
        input: ClaudeActionInput,
    ) -> Result<ClaudeIntegrationStatus, ApiError> {
        let runtime = self.claude_runtime()?;
        let _guard = self.claude_mutation.lock().map_err(|_| {
            ApiError::new(
                "CLAUDE_UNAVAILABLE",
                "Claude Code 接入状态暂时不可用，请重试",
            )
        })?;
        self.with_database(|database| {
            ClaudeIntegrationService::new(database, &runtime).uninstall(input)
        })
    }

    pub fn refresh_claude_events(&self, project_id: &str) -> Result<ClaudeImportReport, ApiError> {
        let runtime = self.claude_runtime()?;
        self.with_database(|database| {
            ClaudeIntegrationService::new(database, &runtime).import_pending(project_id)
        })
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

fn resolve_hook_binary() -> Option<PathBuf> {
    if let Some(configured) = std::env::var_os("XUSHU_HOOK_BIN") {
        let path = PathBuf::from(configured);
        if path.is_file() {
            return Some(path);
        }
    }

    let executable = std::env::current_exe().ok()?;
    let directory = executable.parent()?;
    let names: &[&str] = if cfg!(windows) {
        &["xushu-hook.exe", "xushu-hook-x86_64-pc-windows-msvc.exe"]
    } else {
        &["xushu-hook"]
    };
    names
        .iter()
        .map(|name| directory.join(name))
        .find(|path| path.is_file())
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::TempDir;

    use super::*;

    #[test]
    fn database_and_claude_runtime_failures_are_reported_without_panicking() {
        let directory = TempDir::new().expect("temp dir");
        let not_a_directory = directory.path().join("not-a-directory");
        fs::write(&not_a_directory, b"x").expect("write file");

        let state = AppState::initialize(&not_a_directory);
        let error = state
            .with_database(|_| Ok::<_, CoreError>(()))
            .expect_err("database must be unavailable");
        assert_eq!(error.code, "DATABASE_UNAVAILABLE");

        let error = state
            .get_claude_status("11111111-1111-4111-8111-111111111111")
            .expect_err("claude runtime must be unavailable");
        assert_eq!(error.code, "CLAUDE_UNAVAILABLE");
    }
}
