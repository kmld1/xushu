use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use chrono::{SecondsFormat, Utc};
use uuid::Uuid;
use xushu_core::{
    backup::{
        cleanup_workspaces, lock_data_directory, recover_interrupted_restore,
        AutomaticBackupOutcome, BackupPolicy, BackupRecord, BackupService, BackupVerification,
        CleanupOutcome, DataIntegrityReport, DataIntegrityService, DataProtectionOverview,
        DataProtectionService, PerformRestoreInput, RestoreJournal, RestoreOutcome, RestorePlan,
        RestorePreflight, RestorePreflightInput, RestoreService, RestoreStartupNotice,
        RollbackReport, UpdateBackupPolicyInput,
    },
    code_state, runner, AcceptanceReport, ClaudeActionInput, ClaudeActivitySnapshot,
    ClaudeHookPreview, ClaudeImportReport, ClaudeIntegrationService, ClaudeIntegrationStatus,
    ClaudeRuntimePaths, CoreError, Database, InstallClaudeHooksInput, RunControl,
    StartVerificationInput, VerificationRun, VerificationService,
};

use crate::error::ApiError;
use crate::logging;

/// 应用级状态。数据库初始化失败时仍允许窗口启动，并通过 IPC 返回可理解的
/// `DATABASE_UNAVAILABLE` 错误，避免无提示闪退。
#[derive(Clone)]
pub struct AppState {
    database: Arc<Mutex<Option<Database>>>,
    startup_error: Option<ApiError>,
    controls: Arc<Mutex<HashMap<String, Arc<RunControl>>>>,
    exports_directory: Arc<PathBuf>,
    data_directory: Arc<PathBuf>,
    claude_runtime: Option<Arc<ClaudeRuntimePaths>>,
    claude_startup_error: Option<ApiError>,
    claude_mutation: Arc<Mutex<()>>,
    backup_mutation: Arc<Mutex<()>>,
    backup_cancel: Arc<AtomicBool>,
    restore_plans: Arc<Mutex<HashMap<String, RestorePlan>>>,
    startup_restore_notice: Arc<Mutex<Option<RestoreStartupNotice>>>,
    _data_directory_lock: Option<Arc<fs::File>>,
}

impl AppState {
    pub fn initialize(data_directory: &Path) -> Self {
        let exports_directory = data_directory.join("exports");
        let data_directory = Arc::new(data_directory.to_path_buf());
        let data_lock = fs::create_dir_all(&*data_directory)
            .map_err(CoreError::from)
            .and_then(|()| lock_data_directory(&data_directory));
        let claude_runtime = match &data_lock {
            Ok(_) => {
                if !cfg!(test) {
                    logging::init(&data_directory);
                }
                let hook_binary = resolve_hook_binary();
                match ClaudeRuntimePaths::prepare(
                    &data_directory,
                    hook_binary,
                    env!("CARGO_PKG_VERSION"),
                ) {
                    Ok(runtime) => Some(Arc::new(runtime)),
                    Err(error) => {
                        logging::record_error(
                            "claude_hook_runtime_initialization_failed",
                            error.code(),
                        );
                        None
                    }
                }
            }
            Err(error) => {
                eprintln!("[xushu] application_data_lock_failed code={}", error.code());
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

        let startup_restore_notice = match data_lock
            .as_ref()
            .ok()
            .map(|_| recover_interrupted_restore(&data_directory))
        {
            None => None,
            Some(Ok(notice)) => notice,
            Some(Err(error)) => {
                logging::record_error("restore_journal_recovery_failed", error.code());
                Some(RestoreStartupNotice::needs_manual(
                    data_directory
                        .join("restore-journal.json")
                        .to_string_lossy()
                        .to_string(),
                    "启动恢复检查失败；没有自动修改数据库，请人工检查应用数据目录。",
                    None,
                ))
            }
        };
        let pending_manual = startup_restore_notice
            .as_ref()
            .filter(|notice| notice.outcome == "needs_manual");
        let (data_lock, result) = match data_lock {
            Ok(lock) if pending_manual.is_none() => {
                let _ = cleanup_workspaces(data_directory.as_path());
                let result = Database::open(data_directory.join("xushu.db"));
                (Some(Arc::new(lock)), result)
            }
            Ok(lock) => (Some(Arc::new(lock)), Err(CoreError::Unavailable)),
            Err(error) => (None, Err(error)),
        };

        match result {
            Ok(database) => {
                match database.migrate() {
                    Ok(report) => logging::record(
                        "INFO",
                        &format!(
                            "database_ready schema={} applied={}",
                            report.current_version,
                            report.applied.len()
                        ),
                    ),
                    Err(error) => {
                        logging::record_error("database_migration_failed", error.code());
                    }
                }
                match VerificationService::new(&database).recover_interrupted() {
                    Ok(recovered) if recovered > 0 => {
                        logging::record(
                            "INFO",
                            &format!("verification_runs_recovered count={recovered}"),
                        );
                    }
                    Err(error) => {
                        logging::record_error("verification_recovery_failed", error.code());
                    }
                    _ => {}
                }
                if let Some(runtime) = &claude_runtime {
                    let reports =
                        ClaudeIntegrationService::new(&database, runtime).import_pending_all();
                    let imported: usize = reports.iter().map(|report| report.imported).sum();
                    if imported > 0 {
                        logging::record(
                            "INFO",
                            &format!("claude_events_imported count={imported}"),
                        );
                    }
                }
                match BackupService::new(&database, data_directory.as_path())
                    .run_due_automatic(&AtomicBool::new(false))
                {
                    Ok(outcome) if outcome.status == "created" => {
                        logging::record("INFO", "automatic_backup_created");
                    }
                    Ok(outcome) if outcome.status == "failed" => {
                        logging::record("WARN", "automatic_backup_failed");
                    }
                    Ok(_) => {}
                    Err(error) => {
                        logging::record_error("automatic_backup_check_failed", error.code());
                    }
                }
                Self {
                    database: Arc::new(Mutex::new(Some(database))),
                    startup_error: None,
                    controls: Arc::new(Mutex::new(HashMap::new())),
                    exports_directory: Arc::new(exports_directory),
                    data_directory,
                    claude_runtime,
                    claude_startup_error,
                    claude_mutation: Arc::new(Mutex::new(())),
                    backup_mutation: Arc::new(Mutex::new(())),
                    backup_cancel: Arc::new(AtomicBool::new(false)),
                    restore_plans: Arc::new(Mutex::new(HashMap::new())),
                    startup_restore_notice: Arc::new(Mutex::new(startup_restore_notice)),
                    _data_directory_lock: data_lock,
                }
            }
            Err(error) => {
                logging::record_error("database_initialization_failed", error.code());
                Self {
                    database: Arc::new(Mutex::new(None)),
                    startup_error: Some(match pending_manual {
                        Some(notice) => ApiError::new("RESTORE_NEEDS_MANUAL", &notice.message),
                        None if matches!(&error, CoreError::Conflict(_)) => {
                            ApiError::new("DATABASE_IN_USE", "另一个序枢进程正在使用此数据库")
                        }
                        None => ApiError::new(
                            "DATABASE_UNAVAILABLE",
                            "本地数据库初始化失败，请检查应用数据目录权限后重启应用",
                        ),
                    }),
                    controls: Arc::new(Mutex::new(HashMap::new())),
                    exports_directory: Arc::new(exports_directory),
                    data_directory,
                    claude_runtime,
                    claude_startup_error,
                    claude_mutation: Arc::new(Mutex::new(())),
                    backup_mutation: Arc::new(Mutex::new(())),
                    backup_cancel: Arc::new(AtomicBool::new(false)),
                    restore_plans: Arc::new(Mutex::new(HashMap::new())),
                    startup_restore_notice: Arc::new(Mutex::new(startup_restore_notice)),
                    _data_directory_lock: data_lock,
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
                logging::record_error("verification_finalize_failed", &error.code);
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

    pub fn get_data_protection_overview(&self) -> Result<DataProtectionOverview, ApiError> {
        let notice = self
            .startup_restore_notice
            .lock()
            .map_err(|_| ApiError::new("RESTORE_ERROR", "恢复状态暂时不可用，请重试"))?
            .clone();
        let hooks = self
            .claude_runtime
            .as_ref()
            .map(|runtime| runtime.privacy_snapshot())
            .unwrap_or_default();
        self.with_database(|database| {
            DataProtectionService::new(database, self.data_directory.as_path())
                .overview(notice, hooks)
        })
    }

    pub fn dismiss_restore_notice(&self) -> Result<(), ApiError> {
        *self
            .startup_restore_notice
            .lock()
            .map_err(|_| ApiError::new("RESTORE_ERROR", "恢复状态暂时不可用，请重试"))? = None;
        Ok(())
    }

    pub fn create_backup(&self, target_directory: String) -> Result<BackupRecord, ApiError> {
        let _guard = self.lock_backup_mutation()?;
        self.backup_cancel.store(false, Ordering::SeqCst);
        let cancel = self.backup_cancel.clone();
        let result = self.with_database(|database| {
            BackupService::new(database, self.data_directory.as_path())
                .create_manual(&target_directory, &cancel)
        });
        self.backup_cancel.store(false, Ordering::SeqCst);
        result
    }

    pub fn cancel_backup(&self) -> Result<(), ApiError> {
        self.backup_cancel.store(true, Ordering::SeqCst);
        Ok(())
    }

    pub fn list_backups(&self) -> Result<Vec<BackupRecord>, ApiError> {
        self.with_database(|database| {
            BackupService::new(database, self.data_directory.as_path()).list()
        })
    }

    pub fn verify_backup(&self, backup_id: String) -> Result<BackupVerification, ApiError> {
        self.with_database(|database| {
            BackupService::new(database, self.data_directory.as_path()).verify(&backup_id)
        })
    }

    pub fn delete_backup(&self, backup_id: String, approved: bool) -> Result<(), ApiError> {
        let _guard = self.lock_backup_mutation()?;
        self.with_database(|database| {
            BackupService::new(database, self.data_directory.as_path()).delete(&backup_id, approved)
        })
    }

    pub fn get_backup_policy(&self) -> Result<BackupPolicy, ApiError> {
        self.with_database(|database| {
            BackupService::new(database, self.data_directory.as_path()).policy()
        })
    }

    pub fn update_backup_policy(
        &self,
        input: UpdateBackupPolicyInput,
    ) -> Result<BackupPolicy, ApiError> {
        let _guard = self.lock_backup_mutation()?;
        self.with_database(|database| {
            BackupService::new(database, self.data_directory.as_path()).update_policy(input)
        })
    }

    pub fn preview_backup_cleanup(&self) -> Result<Vec<BackupRecord>, ApiError> {
        self.with_database(|database| {
            BackupService::new(database, self.data_directory.as_path()).cleanup_preview()
        })
    }

    pub fn apply_backup_cleanup(&self, approved: bool) -> Result<CleanupOutcome, ApiError> {
        let _guard = self.lock_backup_mutation()?;
        self.with_database(|database| {
            BackupService::new(database, self.data_directory.as_path()).apply_cleanup(approved)
        })
    }

    pub fn run_due_automatic_backup(&self) -> Result<AutomaticBackupOutcome, ApiError> {
        let _guard = self.lock_backup_mutation()?;
        self.backup_cancel.store(false, Ordering::SeqCst);
        let cancel = self.backup_cancel.clone();
        let result = self.with_database(|database| {
            BackupService::new(database, self.data_directory.as_path()).run_due_automatic(&cancel)
        });
        self.backup_cancel.store(false, Ordering::SeqCst);
        result
    }

    pub fn check_data_integrity(&self) -> Result<DataIntegrityReport, ApiError> {
        self.with_database(|database| DataIntegrityService::new(database).check())
    }

    pub fn preflight_restore(
        &self,
        input: RestorePreflightInput,
    ) -> Result<RestorePreflight, ApiError> {
        let _guard = self.lock_backup_mutation()?;
        self.purge_expired_plans();
        let mut outcome = self.with_database(|database| {
            RestoreService::new(database, self.data_directory.as_path()).preflight(input)
        })?;
        if let Some(plan) = outcome.plan.take() {
            let token = plan.token.clone();
            let mut plans = self
                .restore_plans
                .lock()
                .map_err(|_| ApiError::new("RESTORE_ERROR", "恢复预检状态暂时不可用，请重试"))?;
            if plans.len() >= 3 {
                drop(plans);
                RestoreService::cleanup_plan_workspace(&plan);
                return Err(ApiError::new(
                    "CONFLICT",
                    "已有多个未完成的恢复预检，请重启应用后重新开始",
                ));
            }
            plans.insert(token, plan);
        }
        Ok(outcome.preflight)
    }

    pub fn perform_restore(&self, input: PerformRestoreInput) -> Result<RestoreOutcome, ApiError> {
        let _guard = self.lock_backup_mutation()?;
        RestoreService::validates_confirmation(&input).map_err(ApiError::from_core)?;

        let plan = {
            let mut plans = self
                .restore_plans
                .lock()
                .map_err(|_| ApiError::new("RESTORE_ERROR", "恢复预检状态暂时不可用，请重试"))?;
            let plan = plans.remove(&input.token).ok_or_else(|| {
                ApiError::new("RESTORE_ERROR", "恢复预检已失效或已被使用，请重新开始")
            })?;
            // 一旦开始执行恢复，其他预检都基于旧状态，必须作废。
            let stale: Vec<RestorePlan> = plans.drain().map(|(_, value)| value).collect();
            drop(plans);
            for stale_plan in stale {
                RestoreService::cleanup_plan_workspace(&stale_plan);
            }
            plan
        };

        if Utc::now() > plan.expires_at {
            RestoreService::cleanup_plan_workspace(&plan);
            return Err(ApiError::new("RESTORE_ERROR", "恢复预检已过期，请重新开始"));
        }
        if let Err(error) = RestoreService::revalidate(&plan) {
            RestoreService::cleanup_plan_workspace(&plan);
            return Err(ApiError::from_core(error));
        }
        if let Err(error) = self.with_database(|database| {
            let service = RestoreService::new(database, self.data_directory.as_path());
            service.check_formal_restore_capacity(&plan)
        }) {
            RestoreService::cleanup_plan_workspace(&plan);
            return Err(error);
        }
        if !self
            .controls
            .lock()
            .map_err(|_| ApiError::new("VERIFICATION_ERROR", "验证运行注册表暂时不可用"))?
            .is_empty()
        {
            RestoreService::cleanup_plan_workspace(&plan);
            return Err(ApiError::new(
                "CONFLICT",
                "存在运行中的验收任务，不能执行数据库恢复",
            ));
        }

        let operation_id = Uuid::new_v4().to_string();
        let database_path = self.data_directory.join("xushu.db");
        let mut db_guard = self.database.lock().map_err(|_| {
            ApiError::new("DATABASE_LOCK_ERROR", "本地数据库连接暂时不可用，请重试")
        })?;
        if let Err(error) = db_guard
            .as_ref()
            .ok_or(CoreError::Unavailable)
            .and_then(|database| {
                RestoreService::new(database, self.data_directory.as_path())
                    .ensure_current_unchanged(&plan)
            })
        {
            RestoreService::cleanup_plan_workspace(&plan);
            return Err(ApiError::from_core(error));
        }
        let database = db_guard.take().ok_or_else(|| {
            ApiError::new("DATABASE_UNAVAILABLE", "本地数据库尚未就绪，不能执行恢复")
        })?;

        let mut journal = match RestoreService::new(&database, self.data_directory.as_path())
            .create_rollback(&operation_id)
        {
            Ok(journal) => journal,
            Err(error) => {
                *db_guard = Some(database);
                RestoreService::cleanup_plan_workspace(&plan);
                return Err(ApiError::from_core(error));
            }
        };

        if let Err(error) = database.checkpoint().and_then(|()| {
            RestoreService::new(&database, self.data_directory.as_path())
                .ensure_current_unchanged(&plan)
        }) {
            *db_guard = Some(database);
            let _ = RestoreService::finish_journal(
                self.data_directory.as_path(),
                &journal,
                "abandoned",
            );
            RestoreService::cleanup_plan_workspace(&plan);
            return Err(ApiError::from_core(error));
        }
        drop(database);

        if let Err(error) =
            RestoreService::begin_switch(self.data_directory.as_path(), &mut journal, &plan)
        {
            if let Ok(reopened) = Database::open(&database_path) {
                *db_guard = Some(reopened);
            }
            let _ = RestoreService::finish_journal(
                self.data_directory.as_path(),
                &journal,
                "abandoned",
            );
            RestoreService::cleanup_plan_workspace(&plan);
            return Err(ApiError::from_core(error));
        }

        if let Err(error) =
            RestoreService::switch_files(self.data_directory.as_path(), &journal, &plan)
        {
            logging::record_error("restore_switch_failed", error.code());
            return Ok(self.finish_failed_restore(db_guard, &journal, &plan, &operation_id, None));
        }

        let final_report = RestoreService::verify_restored_database(&database_path);
        match final_report {
            Ok(report) if report.ok => match Database::open(&database_path) {
                Ok(restored) => {
                    let hooks_review = RestoreService::hooks_review(&restored).ok();
                    let record = RestoreService::record_operation(
                        &restored,
                        &operation_id,
                        &journal,
                        "completed",
                        true,
                        "available",
                    )
                    .and_then(|()| restored.checkpoint());
                    if record.is_err() {
                        drop(restored);
                        return Ok(self.finish_failed_restore(
                            db_guard,
                            &journal,
                            &plan,
                            &operation_id,
                            Some(report),
                        ));
                    }
                    if RestoreService::finish_journal(
                        self.data_directory.as_path(),
                        &journal,
                        "completed",
                    )
                    .is_err()
                        && RestoreService::active_journal_stage(self.data_directory.as_path())
                            .as_deref()
                            != Some("completed")
                    {
                        drop(restored);
                        return Ok(self.finish_failed_restore(
                            db_guard,
                            &journal,
                            &plan,
                            &operation_id,
                            Some(report),
                        ));
                    }
                    *db_guard = Some(restored);
                    drop(db_guard);
                    RestoreService::cleanup_plan_workspace(&plan);
                    Ok(RestoreOutcome {
                        operation_id,
                        status: "completed".to_owned(),
                        switch_verified: true,
                        final_report: Some(report),
                        rollback: RollbackReport {
                            status: "available".to_owned(),
                            directory: Some(journal.rollback_directory()),
                            verified: true,
                            message: "恢复前的数据库一致性回退副本已保留".to_owned(),
                        },
                        hooks_review,
                        message: "恢复完成并通过最终完整性检查".to_owned(),
                        completed_at: now_iso(),
                        warnings: plan.preflight.warnings.clone(),
                    })
                }
                Err(error) => {
                    logging::record_error("restored_database_reopen_failed", error.code());
                    Ok(self.finish_failed_restore(
                        db_guard,
                        &journal,
                        &plan,
                        &operation_id,
                        Some(report),
                    ))
                }
            },
            Ok(report) => {
                logging::record("ERROR", "restored_database_final_validation_failed");
                Ok(self.finish_failed_restore(
                    db_guard,
                    &journal,
                    &plan,
                    &operation_id,
                    Some(report),
                ))
            }
            Err(error) => {
                logging::record_error("restored_database_final_check_failed", error.code());
                Ok(self.finish_failed_restore(db_guard, &journal, &plan, &operation_id, None))
            }
        }
    }

    fn finish_failed_restore(
        &self,
        mut db_guard: std::sync::MutexGuard<'_, Option<Database>>,
        journal: &RestoreJournal,
        plan: &RestorePlan,
        operation_id: &str,
        report: Option<DataIntegrityReport>,
    ) -> RestoreOutcome {
        let rollback = RestoreService::rollback_switch(self.data_directory.as_path(), journal);
        let recovered_database = if rollback.verified {
            Database::open(self.data_directory.join("xushu.db")).ok()
        } else {
            None
        };

        let verified = rollback.verified && recovered_database.is_some();
        let status = if verified {
            "rolled_back"
        } else {
            "rollback_failed"
        };
        if let Some(database) = recovered_database {
            let _ = RestoreService::record_operation(
                &database,
                operation_id,
                journal,
                status,
                false,
                &rollback.status,
            );
            *db_guard = Some(database);
            drop(db_guard);
        } else {
            drop(db_guard);
        }
        if verified {
            let _ = RestoreService::finish_journal(
                self.data_directory.as_path(),
                journal,
                "rolled_back",
            );
            RestoreService::cleanup_plan_workspace(plan);
        } else if let Ok(mut notice) = self.startup_restore_notice.lock() {
            *notice = Some(RestoreStartupNotice::needs_manual(
                self.data_directory
                    .join("restore-journal.json")
                    .to_string_lossy()
                    .into_owned(),
                "恢复失败且自动回退未完成，请保留恢复日志与回退目录并人工检查。",
                Some(journal.rollback_directory()),
            ));
        }

        let mut warnings = plan.preflight.warnings.clone();
        if !verified {
            warnings.push(
                "自动回退未通过完整性检查或回退失败；未删除原数据库或回退文件，请人工检查。"
                    .to_owned(),
            );
        }
        RestoreOutcome {
            operation_id: operation_id.to_owned(),
            status: status.to_owned(),
            switch_verified: false,
            final_report: report,
            rollback,
            hooks_review: None,
            message: if verified {
                "恢复未完成，已自动回退到恢复前的数据库并通过完整性检查".to_owned()
            } else {
                "恢复未完成且回退失败；应用数据库可能不可用，请人工检查回退目录".to_owned()
            },
            completed_at: now_iso(),
            warnings,
        }
    }

    fn lock_backup_mutation(&self) -> Result<std::sync::MutexGuard<'_, ()>, ApiError> {
        self.backup_mutation.lock().map_err(|_| {
            ApiError::new(
                "BACKUP_ERROR",
                "备份或恢复操作状态暂时不可用，请重启应用后重试",
            )
        })
    }

    fn purge_expired_plans(&self) {
        let now = Utc::now();
        if let Ok(mut plans) = self.restore_plans.lock() {
            let expired: Vec<String> = plans
                .iter()
                .filter(|(_, plan)| plan.expires_at <= now)
                .map(|(token, _)| token.clone())
                .collect();
            for token in expired {
                if let Some(plan) = plans.remove(&token) {
                    RestoreService::cleanup_plan_workspace(&plan);
                }
            }
        }
    }
}

fn now_iso() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
}

fn resolve_hook_binary() -> Option<PathBuf> {
    #[cfg(debug_assertions)]
    if let Some(configured) = std::env::var_os("XUSHU_HOOK_BIN") {
        let path = PathBuf::from(configured);
        return verify_hook_binary(&path).then_some(path);
    }

    let executable = std::env::current_exe().ok()?;
    let directory = executable.parent()?;
    let names: &[&str] = if cfg!(windows) {
        &["xushu-hook.exe", "xushu-hook-x86_64-pc-windows-msvc.exe"]
    } else {
        &["xushu-hook"]
    };
    let mut candidates = names
        .iter()
        .map(|name| directory.join(name))
        .filter(|path| path.is_file())
        .peekable();
    if candidates.peek().is_none() {
        logging::record_error("sidecar_probe_failed", "missing");
    }
    candidates.find(|path| verify_hook_binary(path))
}

/// 生产安装包必须能区分“sidecar 缺失、损坏、版本不一致”和可用接收器。
/// 只执行固定的 `--version` 参数，不经过 shell，也不接受用户输入。
fn verify_hook_binary(path: &Path) -> bool {
    if !path.is_file() {
        logging::record_error("sidecar_probe_failed", "missing");
        return false;
    }

    let mut command = Command::new(path);
    command
        .arg("--version")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }

    let output = match command.output() {
        Ok(output) => output,
        Err(_) => {
            logging::record_error("sidecar_probe_failed", "spawn_failed");
            return false;
        }
    };
    if !output.status.success() {
        logging::record_error("sidecar_probe_failed", "nonzero_exit");
        return false;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    if hook_version_matches(&stdout, env!("CARGO_PKG_VERSION")) {
        true
    } else {
        logging::record_error("sidecar_probe_failed", "version_mismatch");
        false
    }
}

fn hook_version_matches(output: &str, expected: &str) -> bool {
    output
        .trim()
        .strip_prefix("xushu-hook ")
        .is_some_and(|version| version == expected)
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::TempDir;

    use super::*;

    #[test]
    fn unresolved_startup_restore_never_creates_a_replacement_database() {
        let directory = TempDir::new().expect("temp dir");
        let journal = directory.path().join("restore-journal.json");
        fs::write(&journal, b"incomplete restore journal").expect("write journal");

        let state = AppState::initialize(directory.path());

        assert!(!directory.path().join("xushu.db").exists());
        assert!(journal.is_file());
        assert!(state.with_database(|_| Ok(())).is_err());
        assert_eq!(
            state
                .startup_restore_notice
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .outcome,
            "needs_manual"
        );
    }

    #[test]
    fn second_app_state_cannot_open_the_same_database() {
        let directory = TempDir::new().expect("temp dir");
        let first = AppState::initialize(directory.path());
        assert!(
            first
                .with_database(|database| database.health())
                .unwrap()
                .ok
        );
        let second = AppState::initialize(directory.path());
        let error = second
            .with_database(|database| database.health())
            .unwrap_err();
        assert_eq!(error.code, "DATABASE_IN_USE");
        drop(first);
        let reopened = AppState::initialize(directory.path());
        assert!(
            reopened
                .with_database(|database| database.health())
                .unwrap()
                .ok
        );
    }

    #[test]
    fn expired_and_concurrent_restore_requests_cannot_reuse_authorization() {
        let directory = TempDir::new().unwrap();
        let state = AppState::initialize(directory.path());
        let backup = state
            .create_backup(
                directory
                    .path()
                    .join("backups")
                    .to_string_lossy()
                    .into_owned(),
            )
            .unwrap();
        let preflight = || {
            state
                .preflight_restore(RestorePreflightInput {
                    backup_directory: backup.directory_path.clone(),
                })
                .unwrap()
                .token
                .unwrap()
        };
        let request = |token| PerformRestoreInput {
            token,
            confirmation_text: xushu_core::backup::RESTORE_CONFIRMATION_TEXT.to_owned(),
            acknowledged: true,
        };
        let expired = preflight();
        state
            .restore_plans
            .lock()
            .unwrap()
            .get_mut(&expired)
            .unwrap()
            .expires_at = Utc::now() - chrono::Duration::seconds(1);
        for _ in 0..2 {
            assert!(state.perform_restore(request(expired.clone())).is_err());
        }
        assert!(!directory.path().join("restore-journal.json").exists());

        let tokens = [preflight(), preflight()];
        let start = std::sync::Barrier::new(2);
        let outcomes = std::thread::scope(|scope| {
            let workers = tokens.map(|token| {
                let state = &state;
                let start = &start;
                scope.spawn(move || {
                    start.wait();
                    state.perform_restore(request(token))
                })
            });
            workers.map(|worker| worker.join().unwrap())
        });
        assert_eq!(outcomes.iter().filter(|result| result.is_ok()).count(), 1);
        let successful = outcomes.into_iter().find_map(Result::ok).unwrap();
        assert_eq!(successful.status, "completed");
        let overview = state.get_data_protection_overview().unwrap();
        assert_eq!(overview.restore_operations.len(), 1);
    }

    #[test]
    fn final_check_failure_rolls_back_or_keeps_manual_recovery_evidence() {
        for rollback_available in [true, false] {
            let directory = TempDir::new().unwrap();
            let root = directory.path();
            let state = AppState::initialize(root);
            let backup = state
                .create_backup(root.join("backups").to_string_lossy().into_owned())
                .unwrap();
            state
                .with_database(|database| {
                    database.with_transaction(|tx| {
                        tx.execute(
                            "UPDATE backup_settings SET retention_count = 19 WHERE id = 1",
                            [],
                        )?;
                        Ok(())
                    })
                })
                .unwrap();
            let mut guard = state.database.lock().unwrap();
            let database = guard.take().unwrap();
            let service = RestoreService::new(&database, root);
            let plan = service
                .preflight(RestorePreflightInput {
                    backup_directory: backup.directory_path,
                })
                .unwrap()
                .plan
                .unwrap();
            let operation = Uuid::new_v4().to_string();
            let mut journal = service.create_rollback(&operation).unwrap();
            database.checkpoint().unwrap();
            drop(database);
            RestoreService::begin_switch(root, &mut journal, &plan).unwrap();
            RestoreService::switch_files(root, &journal, &plan).unwrap();
            let live = root.join("xushu.db");
            fs::write(&live, b"invalid after switch").unwrap();
            assert!(RestoreService::verify_restored_database(&live).is_err());
            let evidence = PathBuf::from(journal.rollback_directory());
            if !rollback_available {
                fs::rename(
                    evidence.join("rollback-xushu.db"),
                    evidence.join("offline.db"),
                )
                .unwrap();
            }
            let outcome = state.finish_failed_restore(guard, &journal, &plan, &operation, None);
            assert!(!outcome.switch_verified);
            if rollback_available {
                assert_eq!(outcome.status, "rolled_back");
                assert_eq!(state.get_backup_policy().unwrap().retention_count, 19);
                assert_eq!(
                    state
                        .get_data_protection_overview()
                        .unwrap()
                        .restore_operations[0]
                        .outcome,
                    "rolled_back"
                );
            } else {
                assert_eq!(outcome.status, "rollback_failed");
                assert_eq!(outcome.rollback.status, "failed");
                assert!(state.get_backup_policy().is_err());
                assert!(root.join("restore-journal.json").is_file());
                assert!(plan.isolated_database.is_file());
                assert!(evidence.join("original-xushu.db").is_file());
                assert_eq!(fs::read(live).unwrap(), b"invalid after switch");
            }
        }
    }

    #[test]
    fn recovery_subprocess_fixture() {
        let Some(root) = std::env::var_os("XUSHU_TEST_RESTORE_PROCESS_ROOT") else {
            return;
        };
        let root = PathBuf::from(root);
        assert!(root.join("isolated-test.marker").is_file());
        let phase = fs::read_to_string(root.join("phase.txt")).unwrap();
        if let Some(expected) = phase.strip_prefix("reopen:") {
            let app = AppState::initialize(&root);
            let notice = app.startup_restore_notice.lock().unwrap().clone().unwrap();
            assert_eq!(notice.outcome, expected);
            if expected == "needs_manual" {
                assert_eq!(
                    app.get_backup_policy().unwrap_err().code,
                    "RESTORE_NEEDS_MANUAL"
                );
                assert!(root.join("restore-journal.json").is_file());
                assert_eq!(
                    fs::read(root.join("xushu.db")).unwrap(),
                    b"incomplete database"
                );
            } else {
                let health = app.with_database(|database| database.health()).unwrap();
                assert!(health.integrity_ok && !health.foreign_key_violations);
                assert_eq!(app.get_backup_policy().unwrap().retention_count, 17);
                assert!(!root.join("restore-journal.json").exists());
            }
            return;
        }

        let app = AppState::initialize(&root);
        let backup = app
            .create_backup(root.join("backups").to_string_lossy().into_owned())
            .unwrap();
        app.with_database(|database| {
            database.with_transaction(|tx| {
                tx.execute(
                    "UPDATE backup_settings SET retention_count=17 WHERE id=1",
                    [],
                )?;
                Ok(())
            })
        })
        .unwrap();
        let database = app.database.lock().unwrap().take().unwrap();
        let service = RestoreService::new(&database, &root);
        let plan = service
            .preflight(RestorePreflightInput {
                backup_directory: backup.directory_path,
            })
            .unwrap()
            .plan
            .unwrap();
        let mut journal = service
            .create_rollback(&Uuid::new_v4().to_string())
            .unwrap();
        database.checkpoint().unwrap();
        drop(database);
        if phase != "prepared" {
            RestoreService::begin_switch(&root, &mut journal, &plan).unwrap();
            let current = root.join("xushu.db");
            let rollback = PathBuf::from(journal.rollback_directory());
            if phase == "switched" {
                RestoreService::switch_files(&root, &journal, &plan).unwrap();
            } else {
                // Controlled interruption fixture at the first live rename and during copy.
                fs::rename(&current, rollback.join("original-xushu.db")).unwrap();
                fs::write(&current, b"incomplete database").unwrap();
                if phase == "rollback-failure" {
                    fs::rename(
                        rollback.join("rollback-xushu.db"),
                        rollback.join("unavailable.db"),
                    )
                    .unwrap();
                }
            }
        }
        fs::write(root.join("ready"), b"ready").unwrap();
        std::thread::sleep(std::time::Duration::from_secs(30));
        panic!("parent did not terminate the fixture process");
    }

    #[test]
    fn abrupt_restore_process_exit_recovers_on_a_fresh_startup() {
        use std::process::{Command, Stdio};
        use std::time::{Duration, Instant};

        for (phase, expected) in [
            ("prepared", "abandoned"),
            ("partial-copy", "rolled_back"),
            ("switched", "rolled_back"),
            ("rollback-failure", "needs_manual"),
        ] {
            let directory = TempDir::new().unwrap();
            let root = directory.path();
            fs::write(root.join("isolated-test.marker"), b"temporary fixture").unwrap();
            fs::write(root.join("phase.txt"), phase).unwrap();
            let run = || {
                let mut child = Command::new(std::env::current_exe().unwrap());
                child
                    .args([
                        "--exact",
                        "state::tests::recovery_subprocess_fixture",
                        "--nocapture",
                    ])
                    .env("XUSHU_TEST_RESTORE_PROCESS_ROOT", root)
                    .stdin(Stdio::null());
                child
            };
            let mut child = run().spawn().unwrap();
            let deadline = Instant::now() + Duration::from_secs(10);
            while !root.join("ready").exists() && Instant::now() < deadline {
                assert!(
                    child.try_wait().unwrap().is_none(),
                    "fixture exited early at {phase}"
                );
                std::thread::sleep(Duration::from_millis(20));
            }
            child.kill().unwrap();
            assert!(!child.wait().unwrap().success());
            assert!(
                root.join("ready").is_file(),
                "fixture did not reach {phase}"
            );
            fs::write(root.join("phase.txt"), format!("reopen:{expected}")).unwrap();
            let output = run().output().unwrap();
            assert!(
                output.status.success(),
                "{phase}: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            assert!(
                String::from_utf8_lossy(&output.stdout).contains("1 passed"),
                "{phase}"
            );
            eprintln!("[phase6.1] terminated {phase}; new process confirmed {expected}");
        }
    }

    #[test]
    fn restore_rejects_a_preflight_after_a_current_database_write() {
        let directory = TempDir::new().expect("temp dir");
        let state = AppState::initialize(directory.path());
        let backup = state
            .create_backup(
                directory
                    .path()
                    .join("backups")
                    .to_string_lossy()
                    .into_owned(),
            )
            .expect("create backup");
        let preflight = state
            .preflight_restore(RestorePreflightInput {
                backup_directory: backup.directory_path,
            })
            .expect("preflight");
        assert!(preflight.ready);
        let token = preflight.token.unwrap();
        state
            .with_database(|database| {
                database.with_transaction(|tx| {
                    tx.execute(
                        "UPDATE backup_settings SET retention_count = 9 WHERE id = 1",
                        [],
                    )?;
                    Ok(())
                })
            })
            .expect("change current database");

        for _ in 0..2 {
            assert!(state
                .perform_restore(PerformRestoreInput {
                    token: token.clone(),
                    confirmation_text: xushu_core::backup::RESTORE_CONFIRMATION_TEXT.to_owned(),
                    acknowledged: true,
                })
                .is_err());
        }
        assert_eq!(state.get_backup_policy().unwrap().retention_count, 9);
        assert!(!directory.path().join("restore-journal.json").exists());
    }

    #[test]
    fn sidecar_version_probe_accepts_only_the_exact_packaged_version() {
        assert!(hook_version_matches("xushu-hook 0.1.0\n", "0.1.0"));
        assert!(hook_version_matches("  xushu-hook 0.1.0  \r\n", "0.1.0"));
        assert!(!hook_version_matches("xushu-hook 0.0.9\n", "0.1.0"));
        assert!(!hook_version_matches("xushu-hook 0.1.0\n", "0.1.1"));
        assert!(!hook_version_matches("xushu-hook\n", "0.1.0"));
        assert!(!hook_version_matches("garbage", "0.1.0"));
        assert!(!hook_version_matches("", "0.1.0"));
        assert!(!hook_version_matches(
            "xushu-hook 0.1.0\nextra output",
            "0.1.0"
        ));
    }

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
