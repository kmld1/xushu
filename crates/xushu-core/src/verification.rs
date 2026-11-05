use std::fs;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::time::Duration;

use rusqlite::types::{FromSql, FromSqlError, FromSqlResult, ToSql, ToSqlOutput, ValueRef};
use rusqlite::{params, Connection, OptionalExtension, Row};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::code_state::{self, CodeState};
use crate::db::{now_utc_rfc3339, Database};
use crate::error::CoreError;
use crate::models::{AcceptanceCriterion, CriterionKind, Task, TaskStatus};
use crate::redaction;
use crate::runner::{ExecutionStatus, RunOutcome, RunPlan};
use crate::task::TaskService;
use crate::validation::{validate_revision, validate_uuid};

pub const SUCCESS_RULE_EXIT_CODE_ZERO: &str = "exit_code_zero";

const EXECUTABLE_MAX_CHARS: usize = 4096;
const ARGUMENT_MAX_CHARS: usize = 4096;
const MAX_ARGUMENTS: usize = 128;
const TIMEOUT_MIN_SECONDS: i64 = 1;
const TIMEOUT_MAX_SECONDS: i64 = 3600;
const OUTPUT_MIN_BYTES: i64 = 1024;
const OUTPUT_MAX_BYTES: i64 = 1_048_576;
const RUN_HISTORY_LIMIT: i64 = 100;

const BANNED_SHELLS: &[&str] = &[
    "cmd",
    "powershell",
    "pwsh",
    "sh",
    "bash",
    "zsh",
    "fish",
    "wsl",
    "cscript",
    "wscript",
    "mshta",
];

const BANNED_WINDOWS_EXTENSIONS: &[&str] = &["bat", "cmd", "ps1", "vbs", "vbe", "js", "jse", "wsf"];

// ---------------------------------------------------------------------------
// 公开类型
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VerificationResult {
    Pass,
    Fail,
    NotRun,
    Inconclusive,
    Stale,
}

impl VerificationResult {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Pass => "pass",
            Self::Fail => "fail",
            Self::NotRun => "not_run",
            Self::Inconclusive => "inconclusive",
            Self::Stale => "stale",
        }
    }
}

impl FromStr for VerificationResult {
    type Err = CoreError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim() {
            "pass" => Ok(Self::Pass),
            "fail" => Ok(Self::Fail),
            "not_run" => Ok(Self::NotRun),
            "inconclusive" => Ok(Self::Inconclusive),
            "stale" => Ok(Self::Stale),
            _ => Err(CoreError::Validation(format!("无效验证结果: {value}"))),
        }
    }
}

impl FromSql for VerificationResult {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        match value.as_str()? {
            "pass" => Ok(Self::Pass),
            "fail" => Ok(Self::Fail),
            "not_run" => Ok(Self::NotRun),
            "inconclusive" => Ok(Self::Inconclusive),
            "stale" => Ok(Self::Stale),
            _ => Err(FromSqlError::InvalidType),
        }
    }
}

impl ToSql for VerificationResult {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        Ok(ToSqlOutput::Borrowed(ValueRef::Text(
            self.as_str().as_bytes(),
        )))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VerificationSpec {
    pub id: String,
    pub criterion_id: String,
    pub executable: String,
    pub args: Vec<String>,
    pub cwd: String,
    pub timeout_seconds: i64,
    pub output_limit_bytes: i64,
    pub success_rule: String,
    pub revision: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveVerificationSpecInput {
    pub criterion_id: String,
    pub expected_task_revision: i64,
    pub expected_criterion_revision: i64,
    pub expected_spec_revision: Option<i64>,
    pub executable: String,
    #[serde(default)]
    pub args: Vec<String>,
    pub cwd: Option<String>,
    pub timeout_seconds: i64,
    pub output_limit_bytes: i64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartVerificationInput {
    pub criterion_id: String,
    pub expected_task_revision: i64,
    pub expected_criterion_revision: i64,
    pub expected_spec_revision: i64,
    pub approved: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VerificationTemplate {
    pub id: String,
    pub name: String,
    pub description: String,
    pub executable: String,
    pub args: Vec<String>,
    pub timeout_seconds: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VerificationPreview {
    pub criterion_id: String,
    pub criterion_description: String,
    pub task_id: String,
    pub task_title: String,
    pub executable: String,
    pub resolved_executable: Option<String>,
    pub args: Vec<String>,
    pub cwd: String,
    pub timeout_seconds: i64,
    pub output_limit_bytes: i64,
    pub success_rule: String,
    pub spec_revision: i64,
    pub task_revision: i64,
    pub criterion_revision: i64,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VerificationRun {
    pub id: String,
    pub task_id: String,
    pub criterion_id: Option<String>,
    pub criterion_description: String,
    pub spec_id: Option<String>,
    pub spec_revision: i64,
    pub task_revision: i64,
    pub criterion_revision: i64,
    pub executable: String,
    pub resolved_executable: Option<String>,
    pub args: Vec<String>,
    pub cwd: String,
    pub timeout_seconds: i64,
    pub output_limit_bytes: i64,
    pub success_rule: String,
    pub approved_at: String,
    pub started_at: String,
    pub finished_at: Option<String>,
    pub execution_status: String,
    pub exit_code: Option<i32>,
    pub duration_ms: Option<i64>,
    pub start_code_state: Option<CodeState>,
    pub end_code_state: Option<CodeState>,
    pub stdout_summary: Option<String>,
    pub stderr_summary: Option<String>,
    pub stdout_bytes: Option<i64>,
    pub stderr_bytes: Option<i64>,
    pub stdout_truncated: bool,
    pub stderr_truncated: bool,
    pub sensitive_flags: Vec<String>,
    pub output_digest: Option<String>,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
    pub cancel_reason: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VerificationRunSummary {
    pub id: String,
    pub task_id: String,
    pub criterion_id: Option<String>,
    pub criterion_description: String,
    pub executable: String,
    pub args: Vec<String>,
    pub cwd: String,
    pub execution_status: String,
    pub result: Option<VerificationResult>,
    pub reason: Option<String>,
    pub evidence_id: Option<String>,
    pub spec_revision: i64,
    pub task_revision: i64,
    pub criterion_revision: i64,
    pub started_at: String,
    pub finished_at: Option<String>,
    pub exit_code: Option<i32>,
    pub duration_ms: Option<i64>,
    pub stdout_truncated: bool,
    pub stderr_truncated: bool,
    pub sensitive_flags: Vec<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VerificationCheck {
    pub id: String,
    pub run_id: String,
    pub task_id: String,
    pub criterion_id: Option<String>,
    pub result: VerificationResult,
    pub reason: String,
    pub evidence_id: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Evidence {
    pub id: String,
    pub source_type: String,
    pub task_id: String,
    pub run_id: Option<String>,
    pub summary: String,
    pub redacted_output: Option<String>,
    pub content_digest: String,
    pub sensitive_flags: Vec<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CriterionVerification {
    pub status: VerificationResult,
    pub spec: Option<VerificationSpec>,
    pub latest_run: Option<VerificationRunSummary>,
    pub latest_check: Option<VerificationCheck>,
    pub valid_pass_run_id: Option<String>,
    pub stale_reason: Option<String>,
    pub detail: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VerificationQueueItem {
    pub task: Task,
    pub project_name: String,
    pub completion: crate::models::CompletionStatus,
    pub auto_total: i64,
    pub auto_satisfied: i64,
    pub manual_pending: i64,
    pub running_count: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AcceptanceReport {
    pub file_name: String,
    pub markdown: String,
}

/// `prepare_run` 的产物；Tauri 层据此启动后台线程。
#[derive(Debug, Clone)]
pub struct PreparedRun {
    pub run: VerificationRun,
    pub plan: RunPlan,
    pub project_root: PathBuf,
    /// 运行前复查失败时设置；调用方应记录 start_failed，而不是启动进程。
    pub preflight_error: Option<String>,
}

// ---------------------------------------------------------------------------
// 服务
// ---------------------------------------------------------------------------

pub struct VerificationService<'a> {
    database: &'a Database,
}

impl<'a> VerificationService<'a> {
    pub fn new(database: &'a Database) -> Self {
        Self { database }
    }

    pub fn templates(&self) -> Vec<VerificationTemplate> {
        vec![
            VerificationTemplate {
                id: "cargo-test".to_owned(),
                name: "Cargo test".to_owned(),
                description: "运行 Rust workspace 测试；仅退出码 0 判定通过。".to_owned(),
                executable: "cargo".to_owned(),
                args: vec!["test".to_owned()],
                timeout_seconds: 600,
            },
            VerificationTemplate {
                id: "cargo-clippy".to_owned(),
                name: "Cargo clippy".to_owned(),
                description: "把 Clippy 警告视为失败。".to_owned(),
                executable: "cargo".to_owned(),
                args: vec![
                    "clippy".to_owned(),
                    "--workspace".to_owned(),
                    "--all-targets".to_owned(),
                    "--".to_owned(),
                    "-D".to_owned(),
                    "warnings".to_owned(),
                ],
                timeout_seconds: 600,
            },
            VerificationTemplate {
                id: "node-vitest".to_owned(),
                name: "Node · Vitest".to_owned(),
                description: "通过 node 直接运行本地 vitest，不经过 pnpm/npm 批处理包装器。"
                    .to_owned(),
                executable: "node".to_owned(),
                args: vec![
                    "node_modules/vitest/vitest.mjs".to_owned(),
                    "run".to_owned(),
                ],
                timeout_seconds: 300,
            },
            VerificationTemplate {
                id: "node-test".to_owned(),
                name: "Node test runner".to_owned(),
                description: "运行 node --test。".to_owned(),
                executable: "node".to_owned(),
                args: vec!["--test".to_owned()],
                timeout_seconds: 300,
            },
            VerificationTemplate {
                id: "python-pytest".to_owned(),
                name: "Python · pytest".to_owned(),
                description: "运行 python -m pytest -q。".to_owned(),
                executable: "python".to_owned(),
                args: vec!["-m".to_owned(), "pytest".to_owned(), "-q".to_owned()],
                timeout_seconds: 300,
            },
            VerificationTemplate {
                id: "go-test".to_owned(),
                name: "Go test".to_owned(),
                description: "运行 go test ./...。".to_owned(),
                executable: "go".to_owned(),
                args: vec!["test".to_owned(), "./...".to_owned()],
                timeout_seconds: 600,
            },
            VerificationTemplate {
                id: "dotnet-test".to_owned(),
                name: ".NET test".to_owned(),
                description: "运行 dotnet test。".to_owned(),
                executable: "dotnet".to_owned(),
                args: vec!["test".to_owned()],
                timeout_seconds: 600,
            },
        ]
    }

    pub fn save_spec(
        &self,
        input: SaveVerificationSpecInput,
    ) -> Result<VerificationSpec, CoreError> {
        let criterion_id = validate_uuid(&input.criterion_id, "验收条件 ID")?;
        validate_revision(input.expected_task_revision, "任务修订号")?;
        validate_revision(input.expected_criterion_revision, "验收条件修订号")?;
        if let Some(revision) = input.expected_spec_revision {
            validate_revision(revision, "验证配置修订号")?;
        }

        let executable = validate_executable(&input.executable)?;
        let args = validate_arguments(input.args)?;
        if input.timeout_seconds < TIMEOUT_MIN_SECONDS
            || input.timeout_seconds > TIMEOUT_MAX_SECONDS
        {
            return Err(CoreError::Validation(format!(
                "超时时间必须在 {TIMEOUT_MIN_SECONDS}–{TIMEOUT_MAX_SECONDS} 秒之间"
            )));
        }
        if input.output_limit_bytes < OUTPUT_MIN_BYTES
            || input.output_limit_bytes > OUTPUT_MAX_BYTES
        {
            return Err(CoreError::Validation(format!(
                "输出上限必须在 {OUTPUT_MIN_BYTES}–{OUTPUT_MAX_BYTES} 字节之间"
            )));
        }
        let args_json = serde_json::to_string(&args)
            .map_err(|error| CoreError::Validation(format!("参数序列化失败: {error}")))?;

        self.database.with_transaction(|transaction| {
            let context = load_run_context(transaction, &criterion_id)?;
            ensure_project_active(&context)?;
            ensure_task_editable(&context.task)?;
            check_task_revision(&context.task, input.expected_task_revision)?;
            check_criterion_revision(&context.criterion, input.expected_criterion_revision)?;
            if context.criterion.kind != CriterionKind::Auto {
                return Err(CoreError::Validation(
                    "只有自动验收条件可以配置验证命令".to_owned(),
                ));
            }

            let cwd = validate_cwd(Path::new(&context.project_root), input.cwd.as_deref())?;
            let existing = spec_for_criterion(transaction, &criterion_id)?;
            let now = now_utc_rfc3339();

            match existing {
                Some(existing) => {
                    if input.expected_spec_revision != Some(existing.revision) {
                        return Err(CoreError::Conflict(format!(
                            "验证配置已被修改（当前 revision {}），请刷新后重试",
                            existing.revision
                        )));
                    }
                    let spec = VerificationSpec {
                        executable,
                        args,
                        cwd,
                        timeout_seconds: input.timeout_seconds,
                        output_limit_bytes: input.output_limit_bytes,
                        success_rule: SUCCESS_RULE_EXIT_CODE_ZERO.to_owned(),
                        revision: existing.revision + 1,
                        updated_at: now,
                        ..existing
                    };
                    write_spec(transaction, &spec, &args_json)?;
                    Ok(spec)
                }
                None => {
                    if input.expected_spec_revision.is_some() {
                        return Err(CoreError::Conflict(
                            "验证配置不存在或已被删除，请刷新后重试".to_owned(),
                        ));
                    }
                    let spec = VerificationSpec {
                        id: Uuid::new_v4().to_string(),
                        criterion_id,
                        executable,
                        args,
                        cwd,
                        timeout_seconds: input.timeout_seconds,
                        output_limit_bytes: input.output_limit_bytes,
                        success_rule: SUCCESS_RULE_EXIT_CODE_ZERO.to_owned(),
                        revision: 1,
                        created_at: now.clone(),
                        updated_at: now,
                    };
                    write_spec(transaction, &spec, &args_json)?;
                    Ok(spec)
                }
            }
        })
    }

    pub fn get_spec(&self, criterion_id: &str) -> Result<Option<VerificationSpec>, CoreError> {
        let criterion_id = validate_uuid(criterion_id, "验收条件 ID")?;
        let connection = self.database.lock()?;
        spec_for_criterion(&connection, &criterion_id)
    }

    pub fn preview(&self, criterion_id: &str) -> Result<VerificationPreview, CoreError> {
        let criterion_id = validate_uuid(criterion_id, "验收条件 ID")?;
        let connection = self.database.lock()?;
        let context = load_run_context(&connection, &criterion_id)?;
        ensure_project_active(&context)?;
        let spec = context
            .spec
            .ok_or_else(|| CoreError::Validation("该验收条件尚未配置验证命令".to_owned()))?;
        if context.criterion.kind != CriterionKind::Auto {
            return Err(CoreError::Validation(
                "只有自动验收条件可以运行验证".to_owned(),
            ));
        }

        let resolved = resolve_executable(&spec.executable);
        let mut warnings = vec![
            "验证程序将以当前用户权限运行；测试脚本本身可能执行任意代码。".to_owned(),
            "请只对可信项目运行验证，执行前确认程序、参数与工作目录。".to_owned(),
            "首版成功规则：仅当退出码为 0 时判定为 PASS。".to_owned(),
        ];
        if resolved.is_none() {
            warnings
                .push("当前 PATH 中未解析到可执行程序；运行会以 NOT_RUN 记录失败原因。".to_owned());
        }
        if spec.cwd != context.project_root {
            warnings.push("工作目录是项目根目录下的子目录。".to_owned());
        }

        Ok(VerificationPreview {
            criterion_id: context.criterion.id.clone(),
            criterion_description: context.criterion.description.clone(),
            task_id: context.task.id.clone(),
            task_title: context.task.title.clone(),
            executable: spec.executable.clone(),
            resolved_executable: resolved.map(|path| path.to_string_lossy().into_owned()),
            args: spec.args.clone(),
            cwd: spec.cwd.clone(),
            timeout_seconds: spec.timeout_seconds,
            output_limit_bytes: spec.output_limit_bytes,
            success_rule: spec.success_rule.clone(),
            spec_revision: spec.revision,
            task_revision: context.task.revision,
            criterion_revision: context.criterion.revision,
            warnings,
        })
    }

    /// 校验批准与修订并写入一条 `running` 记录；不启动进程。
    pub fn prepare_run(&self, input: StartVerificationInput) -> Result<PreparedRun, CoreError> {
        if !input.approved {
            return Err(CoreError::ApprovalRequired);
        }

        let criterion_id = validate_uuid(&input.criterion_id, "验收条件 ID")?;
        validate_revision(input.expected_task_revision, "任务修订号")?;
        validate_revision(input.expected_criterion_revision, "验收条件修订号")?;
        validate_revision(input.expected_spec_revision, "验证配置修订号")?;

        let context = {
            let connection = self.database.lock()?;
            load_run_context(&connection, &criterion_id)?
        };
        ensure_project_active(&context)?;
        ensure_task_startable(&context.task)?;
        check_task_revision(&context.task, input.expected_task_revision)?;
        check_criterion_revision(&context.criterion, input.expected_criterion_revision)?;
        if context.criterion.kind != CriterionKind::Auto {
            return Err(CoreError::Validation(
                "只有自动验收条件可以运行验证".to_owned(),
            ));
        }
        let spec = context
            .spec
            .clone()
            .ok_or_else(|| CoreError::Validation("该验收条件尚未配置验证命令".to_owned()))?;
        if spec.revision != input.expected_spec_revision {
            return Err(CoreError::Conflict(format!(
                "验证配置已被修改（当前 revision {}），请刷新后重试",
                spec.revision
            )));
        }

        {
            let connection = self.database.lock()?;
            let running: i64 = connection.query_row(
                "SELECT COUNT(*) FROM verification_runs
                 WHERE task_id = ?1 AND execution_status = 'running'",
                [&context.task.id],
                |row| row.get(0),
            )?;
            if running > 0 {
                return Err(CoreError::Conflict(
                    "该任务已有验证正在运行，请等待结束或取消后再试".to_owned(),
                ));
            }
        }

        let code_state = code_state::capture(Path::new(&context.project_root)).map_err(|_| {
            CoreError::Validation("无法采集当前项目代码状态，验证未启动".to_owned())
        })?;
        let resolved = resolve_executable(&spec.executable);
        let mut preflight_error = validate_cwd_for_run(&context.project_root, &spec.cwd)
            .err()
            .map(|error| match error {
                CoreError::Validation(message) => message,
                _ => "验证工作目录无效".to_owned(),
            });
        if resolved.is_none() {
            preflight_error.get_or_insert_with(|| {
                "无法解析验证程序；请检查程序名或绝对路径，运行未启动".to_owned()
            });
        }
        let resolved_executable = resolved
            .as_ref()
            .map(|path| path.to_string_lossy().into_owned());
        let now = now_utc_rfc3339();

        let run = self.database.with_transaction(|transaction| {
            let fresh = load_run_context(transaction, &criterion_id)?;
            ensure_project_active(&fresh)?;
            ensure_task_startable(&fresh.task)?;
            check_task_revision(&fresh.task, input.expected_task_revision)?;
            check_criterion_revision(&fresh.criterion, input.expected_criterion_revision)?;
            let fresh_spec = fresh
                .spec
                .as_ref()
                .ok_or_else(|| CoreError::Validation("验证配置已被删除".to_owned()))?;
            if fresh_spec.revision != input.expected_spec_revision {
                return Err(CoreError::Conflict(
                    "验证配置已被修改，请刷新后重试".to_owned(),
                ));
            }
            let running: i64 = transaction.query_row(
                "SELECT COUNT(*) FROM verification_runs
                 WHERE task_id = ?1 AND execution_status = 'running'",
                [&fresh.task.id],
                |row| row.get(0),
            )?;
            if running > 0 {
                return Err(CoreError::Conflict(
                    "该任务已有验证正在运行，请等待结束或取消后再试".to_owned(),
                ));
            }

            let run = VerificationRun {
                id: Uuid::new_v4().to_string(),
                task_id: fresh.task.id.clone(),
                criterion_id: Some(fresh.criterion.id.clone()),
                criterion_description: fresh.criterion.description.clone(),
                spec_id: Some(fresh_spec.id.clone()),
                spec_revision: fresh_spec.revision,
                task_revision: fresh.task.revision,
                criterion_revision: fresh.criterion.revision,
                executable: fresh_spec.executable.clone(),
                resolved_executable: resolved_executable.clone(),
                args: fresh_spec.args.clone(),
                cwd: fresh_spec.cwd.clone(),
                timeout_seconds: fresh_spec.timeout_seconds,
                output_limit_bytes: fresh_spec.output_limit_bytes,
                success_rule: fresh_spec.success_rule.clone(),
                approved_at: now.clone(),
                started_at: now.clone(),
                finished_at: None,
                execution_status: "running".to_owned(),
                exit_code: None,
                duration_ms: None,
                start_code_state: Some(code_state.clone()),
                end_code_state: None,
                stdout_summary: None,
                stderr_summary: None,
                stdout_bytes: None,
                stderr_bytes: None,
                stdout_truncated: false,
                stderr_truncated: false,
                sensitive_flags: Vec::new(),
                output_digest: None,
                error_code: None,
                error_message: None,
                cancel_reason: None,
                created_at: now.clone(),
            };
            insert_run(transaction, &run)?;
            Ok(run)
        })?;

        let plan = RunPlan {
            // 已在准备阶段解析到绝对路径时执行该路径，避免批准后 PATH 变化
            // 导致实际执行另一个程序；未解析时 preflight_error 会阻止启动。
            executable: resolved
                .map(|path| path.to_string_lossy().into_owned())
                .unwrap_or_else(|| spec.executable.clone()),
            args: spec.args.clone(),
            cwd: PathBuf::from(&spec.cwd),
            timeout: Duration::from_secs(spec.timeout_seconds as u64),
            output_limit_bytes: spec.output_limit_bytes as usize,
        };

        Ok(PreparedRun {
            run,
            plan,
            project_root: PathBuf::from(context.project_root),
            preflight_error,
        })
    }

    pub fn finalize_run(
        &self,
        run_id: &str,
        outcome: RunOutcome,
        end_code_state: Result<CodeState, String>,
    ) -> Result<VerificationRun, CoreError> {
        let run_id = validate_uuid(run_id, "验证运行 ID")?;
        self.database.with_transaction(|transaction| {
            finalize_run_on(transaction, &run_id, &outcome, &end_code_state)
        })
    }

    /// 应用启动时把上次进程遗留的 `running` 记录标为中断，并留下检查与证据。
    pub fn recover_interrupted(&self) -> Result<usize, CoreError> {
        let run_ids: Vec<String> = {
            let connection = self.database.lock()?;
            let mut statement = connection.prepare(
                "SELECT id FROM verification_runs WHERE execution_status = 'running' ORDER BY created_at",
            )?;
            let rows = statement.query_map([], |row| row.get::<_, String>(0))?;
            rows.collect::<Result<Vec<_>, _>>()?
        };

        let mut recovered = 0;
        for run_id in run_ids {
            let outcome = RunOutcome {
                execution_status: ExecutionStatus::Interrupted,
                exit_code: None,
                duration_ms: 0,
                stdout: String::new(),
                stderr: String::new(),
                stdout_bytes: 0,
                stderr_bytes: 0,
                stdout_truncated: false,
                stderr_truncated: false,
                note: Some("应用重启时发现上次验证运行未正常结束".to_owned()),
            };
            self.finalize_run(
                &run_id,
                outcome,
                Err("应用重启，无法确认结束时代码状态".to_owned()),
            )?;
            recovered += 1;
        }
        Ok(recovered)
    }

    pub fn get_run(&self, run_id: &str) -> Result<VerificationRun, CoreError> {
        let run_id = validate_uuid(run_id, "验证运行 ID")?;
        let connection = self.database.lock()?;
        run_by_id(&connection, &run_id)?.ok_or(CoreError::NotFound)
    }

    pub fn get_check(&self, run_id: &str) -> Result<Option<VerificationCheck>, CoreError> {
        let run_id = validate_uuid(run_id, "验证运行 ID")?;
        let connection = self.database.lock()?;
        check_for_run(&connection, &run_id)
    }

    pub fn list_runs(&self, task_id: &str) -> Result<Vec<VerificationRunSummary>, CoreError> {
        let task_id = validate_uuid(task_id, "任务 ID")?;
        let connection = self.database.lock()?;
        let mut statement = connection.prepare(
            "SELECT r.id, r.task_id, r.criterion_id, r.criterion_description,
                    r.executable, r.args_json, r.cwd,
                    r.execution_status, c.result, c.reason, c.evidence_id,
                    r.spec_revision, r.task_revision, r.criterion_revision,
                    r.started_at, r.finished_at, r.exit_code, r.duration_ms,
                    r.stdout_truncated, r.stderr_truncated, r.sensitive_flags, r.created_at
             FROM verification_runs r
             LEFT JOIN verification_checks c ON c.run_id = r.id
             WHERE r.task_id = ?1
             ORDER BY r.created_at DESC, r.rowid DESC
             LIMIT ?2",
        )?;
        let rows =
            statement.query_map(params![task_id, RUN_HISTORY_LIMIT], run_summary_from_row)?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    pub fn get_evidence(&self, evidence_id: &str) -> Result<Evidence, CoreError> {
        let evidence_id = validate_uuid(evidence_id, "证据 ID")?;
        let connection = self.database.lock()?;
        evidence_by_id(&connection, &evidence_id)?.ok_or(CoreError::NotFound)
    }

    pub fn export_report(
        &self,
        task_id: &str,
        exports_directory: &Path,
    ) -> Result<AcceptanceReport, CoreError> {
        let task_id = validate_uuid(task_id, "任务 ID")?;
        let detail = TaskService::new(self.database).get_task(&task_id)?;
        let runs = self.list_runs(&task_id)?;
        let project_name: String = {
            let connection = self.database.lock()?;
            connection.query_row(
                "SELECT p.name FROM projects p JOIN tasks t ON t.project_id = p.id WHERE t.id = ?1",
                [&task_id],
                |row| row.get(0),
            )?
        };

        let mut evidence_outputs = std::collections::HashMap::new();
        for run in &runs {
            if let Some(evidence_id) = &run.evidence_id {
                if let Ok(evidence) = self.get_evidence(evidence_id) {
                    evidence_outputs.insert(run.id.clone(), evidence);
                }
            }
        }

        let markdown = build_report_markdown(&detail, &runs, &evidence_outputs, &project_name);
        fs::create_dir_all(exports_directory)?;
        let file_name = format!(
            "acceptance-{}-{}.md",
            detail.task.id,
            chrono::Utc::now().format("%Y%m%d-%H%M%S")
        );
        fs::write(exports_directory.join(&file_name), &markdown)?;

        Ok(AcceptanceReport {
            file_name,
            markdown,
        })
    }
}

// ---------------------------------------------------------------------------
// 评估与过期判定
// ---------------------------------------------------------------------------

pub(crate) fn evaluate_criterion(
    connection: &Connection,
    task: &Task,
    criterion: &AcceptanceCriterion,
    current_code_state: Option<&CodeState>,
) -> Result<CriterionVerification, CoreError> {
    let spec = spec_for_criterion(connection, &criterion.id)?;
    let Some(run) = latest_run_for_criterion(connection, &criterion.id)? else {
        return Ok(CriterionVerification {
            status: VerificationResult::NotRun,
            spec,
            latest_run: None,
            latest_check: None,
            valid_pass_run_id: None,
            stale_reason: None,
            detail: Some("尚未运行验证".to_owned()),
        });
    };
    let check = check_for_run(connection, &run.id)?;
    let summary = Some(run_to_summary(&run, check.as_ref()));

    if run.execution_status == "running" {
        return Ok(CriterionVerification {
            status: VerificationResult::NotRun,
            spec,
            latest_run: summary,
            latest_check: check,
            valid_pass_run_id: None,
            stale_reason: None,
            detail: Some("验证正在运行".to_owned()),
        });
    }

    let Some(check) = check else {
        return Ok(CriterionVerification {
            status: VerificationResult::Inconclusive,
            spec,
            latest_run: summary,
            latest_check: None,
            valid_pass_run_id: None,
            stale_reason: None,
            detail: Some("验证运行缺少检查记录，结果不可用".to_owned()),
        });
    };

    let stale_reason = revision_mismatch_reason(task, criterion, spec.as_ref(), &run);
    if let Some(reason) = stale_reason {
        return Ok(CriterionVerification {
            status: VerificationResult::Stale,
            spec,
            latest_run: summary,
            latest_check: Some(check.clone()),
            valid_pass_run_id: None,
            stale_reason: Some(reason),
            detail: Some(check.reason),
        });
    }

    if check.result != VerificationResult::Pass {
        return Ok(CriterionVerification {
            status: check.result,
            spec,
            latest_run: summary,
            latest_check: Some(check.clone()),
            valid_pass_run_id: None,
            stale_reason: None,
            detail: Some(check.reason),
        });
    }

    let Some(current) = current_code_state else {
        return Ok(CriterionVerification {
            status: VerificationResult::Inconclusive,
            spec,
            latest_run: summary,
            latest_check: Some(check.clone()),
            valid_pass_run_id: None,
            stale_reason: None,
            detail: Some("无法采集当前代码状态，无法确认 PASS 证据仍然有效".to_owned()),
        });
    };

    let start = run.start_code_state.as_ref();
    let end = run.end_code_state.as_ref();
    if start.is_none() || end.is_none() {
        return Ok(CriterionVerification {
            status: VerificationResult::Inconclusive,
            spec,
            latest_run: summary,
            latest_check: Some(check.clone()),
            valid_pass_run_id: None,
            stale_reason: None,
            detail: Some("验证运行缺少完整的代码状态记录".to_owned()),
        });
    }
    let start = start.expect("checked above");
    let end = end.expect("checked above");

    if !start.fingerprint_complete || !end.fingerprint_complete {
        return Ok(CriterionVerification {
            status: VerificationResult::Inconclusive,
            spec,
            latest_run: summary,
            latest_check: Some(check.clone()),
            valid_pass_run_id: None,
            stale_reason: None,
            detail: Some("验证运行时的代码状态指纹不完整，不能作为 PASS 证据".to_owned()),
        });
    }

    if !code_states_match(start, end) {
        return Ok(CriterionVerification {
            status: VerificationResult::Inconclusive,
            spec,
            latest_run: summary,
            latest_check: Some(check.clone()),
            valid_pass_run_id: None,
            stale_reason: None,
            detail: Some("验证运行期间代码状态发生变化，结果不可信".to_owned()),
        });
    }

    if !current.fingerprint_complete {
        return Ok(CriterionVerification {
            status: VerificationResult::Inconclusive,
            spec,
            latest_run: summary,
            latest_check: Some(check.clone()),
            valid_pass_run_id: None,
            stale_reason: None,
            detail: Some(
                current
                    .note
                    .clone()
                    .unwrap_or_else(|| "当前代码状态指纹不完整".to_owned()),
            ),
        });
    }

    if !code_states_match(end, current) {
        return Ok(CriterionVerification {
            status: VerificationResult::Stale,
            spec,
            latest_run: summary,
            latest_check: Some(check.clone()),
            valid_pass_run_id: None,
            stale_reason: Some("验证完成后代码状态已发生变化".to_owned()),
            detail: Some(check.reason),
        });
    }

    Ok(CriterionVerification {
        status: VerificationResult::Pass,
        spec,
        latest_run: summary,
        latest_check: Some(check),
        valid_pass_run_id: Some(run.id),
        stale_reason: None,
        detail: None,
    })
}

pub(crate) fn blocker_for(
    criterion: &AcceptanceCriterion,
    verification: &CriterionVerification,
) -> String {
    let description = &criterion.description;
    match verification.status {
        VerificationResult::Pass => String::new(),
        VerificationResult::Fail => format!(
            "自动验收条件未通过：{description}（{}）",
            verification.detail.as_deref().unwrap_or("验证未通过")
        ),
        VerificationResult::Inconclusive => format!(
            "自动验收结果无法判定：{description}（{}）",
            verification.detail.as_deref().unwrap_or("结果不完整")
        ),
        VerificationResult::Stale => format!(
            "自动验收证据已过期：{description}（{}）",
            verification
                .stale_reason
                .as_deref()
                .unwrap_or("需要重新验证")
        ),
        VerificationResult::NotRun => {
            if verification.spec.is_none() {
                format!("自动验收条件尚未配置验证：{description}")
            } else if verification
                .latest_run
                .as_ref()
                .map(|run| run.execution_status == "running")
                .unwrap_or(false)
            {
                format!("自动验收条件正在运行验证：{description}")
            } else {
                format!(
                    "自动验收条件尚未运行验证：{description}（{}）",
                    verification.detail.as_deref().unwrap_or("未运行")
                )
            }
        }
    }
}

fn revision_mismatch_reason(
    task: &Task,
    criterion: &AcceptanceCriterion,
    spec: Option<&VerificationSpec>,
    run: &VerificationRun,
) -> Option<String> {
    let mut reasons = Vec::new();
    if run.task_revision != task.revision {
        reasons.push(format!(
            "任务修订已变化（{} → {}）",
            run.task_revision, task.revision
        ));
    }
    if run.criterion_revision != criterion.revision {
        reasons.push(format!(
            "验收条件修订已变化（{} → {}）",
            run.criterion_revision, criterion.revision
        ));
    }
    match spec {
        Some(spec) if spec.revision != run.spec_revision => reasons.push(format!(
            "验证配置修订已变化（{} → {}）",
            run.spec_revision, spec.revision
        )),
        None => reasons.push("验证配置已不存在".to_owned()),
        _ => {}
    }
    if reasons.is_empty() {
        None
    } else {
        Some(reasons.join("；"))
    }
}

fn code_states_match(left: &CodeState, right: &CodeState) -> bool {
    left.fingerprint.is_some()
        && left.fingerprint_complete
        && right.fingerprint_complete
        && left.head == right.head
        && left.branch == right.branch
        && left.detached == right.detached
        && left.unborn == right.unborn
        && left.dirty == right.dirty
        && left.fingerprint == right.fingerprint
}

fn run_to_summary(
    run: &VerificationRun,
    check: Option<&VerificationCheck>,
) -> VerificationRunSummary {
    VerificationRunSummary {
        id: run.id.clone(),
        task_id: run.task_id.clone(),
        criterion_id: run.criterion_id.clone(),
        criterion_description: run.criterion_description.clone(),
        executable: run.executable.clone(),
        args: run.args.clone(),
        cwd: run.cwd.clone(),
        execution_status: run.execution_status.clone(),
        result: check.map(|check| check.result),
        reason: check.map(|check| check.reason.clone()),
        evidence_id: check.and_then(|check| check.evidence_id.clone()),
        spec_revision: run.spec_revision,
        task_revision: run.task_revision,
        criterion_revision: run.criterion_revision,
        started_at: run.started_at.clone(),
        finished_at: run.finished_at.clone(),
        exit_code: run.exit_code,
        duration_ms: run.duration_ms,
        stdout_truncated: run.stdout_truncated,
        stderr_truncated: run.stderr_truncated,
        sensitive_flags: run.sensitive_flags.clone(),
        created_at: run.created_at.clone(),
    }
}

// ---------------------------------------------------------------------------
// finalize
// ---------------------------------------------------------------------------

fn finalize_run_on(
    connection: &Connection,
    run_id: &str,
    outcome: &RunOutcome,
    end_code_state: &Result<CodeState, String>,
) -> Result<VerificationRun, CoreError> {
    let run = run_by_id(connection, run_id)?.ok_or(CoreError::NotFound)?;
    if run.execution_status != "running" {
        return Err(CoreError::Conflict(
            "验证运行已经结束，不能重复写结果".to_owned(),
        ));
    }

    let stdout = redaction::redact(&outcome.stdout);
    let stderr = redaction::redact(&outcome.stderr);
    let mut sensitive_flags = stdout.flags.clone();
    sensitive_flags.extend(stderr.flags.clone());
    sensitive_flags.sort();
    sensitive_flags.dedup();

    let (check_result, check_reason, error_code) = match outcome.execution_status {
        ExecutionStatus::StartFailed => (
            VerificationResult::NotRun,
            outcome
                .note
                .clone()
                .unwrap_or_else(|| "验证程序未能启动".to_owned()),
            "START_FAILED",
        ),
        ExecutionStatus::Cancelled => (
            VerificationResult::Inconclusive,
            outcome
                .note
                .clone()
                .unwrap_or_else(|| "用户取消验证运行".to_owned()),
            "CANCELLED",
        ),
        ExecutionStatus::TimedOut => (
            VerificationResult::Fail,
            outcome
                .note
                .clone()
                .unwrap_or_else(|| "验证运行超时".to_owned()),
            "TIMED_OUT",
        ),
        ExecutionStatus::Interrupted => (
            VerificationResult::Inconclusive,
            outcome
                .note
                .clone()
                .unwrap_or_else(|| "验证运行异常中断".to_owned()),
            "INTERRUPTED",
        ),
        ExecutionStatus::Completed => match end_code_state {
            Err(_) => (
                VerificationResult::Inconclusive,
                "运行结束后无法采集代码状态，结果不可信".to_owned(),
                "CODE_STATE_UNAVAILABLE",
            ),
            Ok(end) => {
                let start_complete = run
                    .start_code_state
                    .as_ref()
                    .map(|state| state.fingerprint_complete)
                    .unwrap_or(false);
                if !start_complete || !end.fingerprint_complete {
                    (
                        VerificationResult::Inconclusive,
                        "代码状态指纹不完整，结果不能作为 PASS 证据".to_owned(),
                        "CODE_STATE_INCOMPLETE",
                    )
                } else if run
                    .start_code_state
                    .as_ref()
                    .map(|start| !code_states_match(start, end))
                    .unwrap_or(true)
                {
                    (
                        VerificationResult::Inconclusive,
                        "验证期间代码状态发生变化，结果不可信".to_owned(),
                        "CODE_CHANGED_DURING_RUN",
                    )
                } else if outcome.exit_code == Some(0) {
                    (
                        VerificationResult::Pass,
                        "退出码为 0，符合 exit_code_zero 规则".to_owned(),
                        "",
                    )
                } else {
                    (
                        VerificationResult::Fail,
                        format!(
                            "退出码为 {}，不满足 exit_code_zero 规则",
                            outcome
                                .exit_code
                                .map(|code| code.to_string())
                                .unwrap_or_else(|| "未知".to_owned())
                        ),
                        "",
                    )
                }
            }
        },
    };

    let now = now_utc_rfc3339();
    let (end_state, end_captured, end_notes) = match end_code_state {
        Ok(state) => (Some(state), true, state.note.clone()),
        Err(note) => (None, false, Some(note.clone())),
    };
    let start_note = run
        .start_code_state
        .as_ref()
        .and_then(|state| state.note.clone());
    let code_state_note = match (start_note, end_notes) {
        (Some(start), Some(end)) => Some(format!("开始：{start}；结束：{end}")),
        (Some(start), None) => Some(format!("开始：{start}")),
        (None, Some(end)) => Some(format!("结束：{end}")),
        (None, None) => None,
    };

    let summary = match outcome.execution_status {
        ExecutionStatus::Completed => format!(
            "验证程序退出码 {}，耗时 {} ms",
            outcome
                .exit_code
                .map(|code| code.to_string())
                .unwrap_or_else(|| "未知".to_owned()),
            outcome.duration_ms
        ),
        ExecutionStatus::TimedOut => format!("验证超时，耗时 {} ms", outcome.duration_ms),
        ExecutionStatus::Cancelled => "验证被用户取消".to_owned(),
        ExecutionStatus::StartFailed => "验证程序未能启动".to_owned(),
        ExecutionStatus::Interrupted => "验证运行异常中断".to_owned(),
    };

    let digest = evidence_digest(&run, &stdout.text, &stderr.text, &sensitive_flags);
    let evidence_id = Uuid::new_v4().to_string();
    let check_id = Uuid::new_v4().to_string();
    let redacted_output = combine_outputs(&stdout.text, &stderr.text);
    let flags = sensitive_flags.join(",");

    connection.execute(
        "UPDATE verification_runs SET
            finished_at = ?2,
            execution_status = ?3,
            exit_code = ?4,
            duration_ms = ?5,
            end_git_head = ?6,
            end_git_branch = ?7,
            end_git_detached = ?8,
            end_git_unborn = ?9,
            end_git_dirty = ?10,
            end_code_fingerprint = ?11,
            end_fingerprint_complete = ?12,
            end_code_state_captured = ?13,
            end_changed_file_count = ?14,
            code_state_note = ?15,
            stdout_summary = ?16,
            stderr_summary = ?17,
            stdout_bytes = ?18,
            stderr_bytes = ?19,
            stdout_truncated = ?20,
            stderr_truncated = ?21,
            sensitive_flags = ?22,
            output_digest = ?23,
            error_code = ?24,
            error_message = ?25,
            cancel_reason = ?26
         WHERE id = ?1",
        params![
            &run.id,
            &now,
            outcome.execution_status.as_str(),
            outcome.exit_code,
            outcome.duration_ms as i64,
            end_state.as_ref().and_then(|state| state.head.clone()),
            end_state.as_ref().and_then(|state| state.branch.clone()),
            end_state
                .as_ref()
                .map(|state| state.detached)
                .unwrap_or(false),
            end_state
                .as_ref()
                .map(|state| state.unborn)
                .unwrap_or(false),
            end_state.as_ref().map(|state| state.dirty).unwrap_or(false),
            end_state
                .as_ref()
                .and_then(|state| state.fingerprint.clone()),
            end_state
                .as_ref()
                .map(|state| state.fingerprint_complete)
                .unwrap_or(false),
            end_captured,
            end_state
                .as_ref()
                .map(|state| state.changed_file_count as i64)
                .unwrap_or(0),
            code_state_note,
            &stdout.text,
            &stderr.text,
            outcome.stdout_bytes as i64,
            outcome.stderr_bytes as i64,
            outcome.stdout_truncated,
            outcome.stderr_truncated,
            &flags,
            &digest,
            if error_code.is_empty() {
                None
            } else {
                Some(error_code)
            },
            if error_code.is_empty() {
                None
            } else {
                Some(check_reason.clone())
            },
            if outcome.execution_status == ExecutionStatus::Cancelled {
                outcome.note.clone()
            } else {
                None
            },
        ],
    )?;

    connection.execute(
        "INSERT INTO evidence
            (id, source_type, task_id, run_id, summary, redacted_output, content_digest, sensitive_flags, created_at)
         VALUES (?1, 'verification_run', ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            &evidence_id,
            &run.task_id,
            &run.id,
            summary,
            redacted_output,
            &digest,
            &flags,
            &now,
        ],
    )?;

    connection.execute(
        "INSERT INTO verification_checks
            (id, run_id, task_id, criterion_id, result, reason, evidence_id, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            &check_id,
            &run.id,
            &run.task_id,
            &run.criterion_id,
            check_result,
            &check_reason,
            &evidence_id,
            &now,
        ],
    )?;

    run_by_id(connection, &run.id)?.ok_or(CoreError::NotFound)
}

fn evidence_digest(
    run: &VerificationRun,
    stdout: &str,
    stderr: &str,
    sensitive_flags: &[String],
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"xushu-evidence-v1\0");
    hasher.update(run.id.as_bytes());
    hasher.update(b"\0");
    hasher.update(run.task_id.as_bytes());
    hasher.update(b"\0");
    hasher.update(run.criterion_description.as_bytes());
    hasher.update(b"\0");
    hasher.update(stdout.as_bytes());
    hasher.update(b"\0");
    hasher.update(stderr.as_bytes());
    hasher.update(b"\0");
    hasher.update(sensitive_flags.join(",").as_bytes());
    format!("sha256:{}", hex(&hasher.finalize()))
}

fn combine_outputs(stdout: &str, stderr: &str) -> Option<String> {
    if stdout.trim().is_empty() && stderr.trim().is_empty() {
        return None;
    }
    let mut output = String::new();
    if !stdout.trim().is_empty() {
        output.push_str("STDOUT:\n");
        output.push_str(stdout);
        output.push('\n');
    }
    if !stderr.trim().is_empty() {
        output.push_str("STDERR:\n");
        output.push_str(stderr);
    }
    Some(output)
}

fn hex(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        use std::fmt::Write;
        let _ = write!(output, "{byte:02x}");
    }
    output
}

// ---------------------------------------------------------------------------
// SQL 读写
// ---------------------------------------------------------------------------

struct RunContext {
    task: Task,
    criterion: AcceptanceCriterion,
    spec: Option<VerificationSpec>,
    project_root: String,
    project_archived: bool,
}

fn load_run_context(connection: &Connection, criterion_id: &str) -> Result<RunContext, CoreError> {
    let criterion = criterion_by_id(connection, criterion_id)?.ok_or(CoreError::NotFound)?;
    let task = task_by_id(connection, &criterion.task_id)?.ok_or(CoreError::NotFound)?;
    let (project_root, archived_at): (String, Option<String>) = connection.query_row(
        "SELECT p.root_path, p.archived_at
         FROM projects p JOIN tasks t ON t.project_id = p.id
         WHERE t.id = ?1",
        [&task.id],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;
    let spec = spec_for_criterion(connection, criterion_id)?;
    Ok(RunContext {
        task,
        criterion,
        spec,
        project_root,
        project_archived: archived_at.is_some(),
    })
}

fn ensure_project_active(context: &RunContext) -> Result<(), CoreError> {
    if context.project_archived {
        return Err(CoreError::Validation(
            "项目已归档，不能运行验证；请先恢复项目".to_owned(),
        ));
    }
    Ok(())
}

fn ensure_task_editable(task: &Task) -> Result<(), CoreError> {
    if task.archived_at.is_some() {
        return Err(CoreError::Validation("已归档任务不能修改".to_owned()));
    }
    if matches!(task.status, TaskStatus::Done | TaskStatus::Cancelled) {
        return Err(CoreError::Validation(
            "已结束任务不能修改；已取消任务可使用重新开启".to_owned(),
        ));
    }
    Ok(())
}

fn ensure_task_startable(task: &Task) -> Result<(), CoreError> {
    if task.archived_at.is_some() {
        return Err(CoreError::Validation("已归档任务不能运行验证".to_owned()));
    }
    if task.status != TaskStatus::AwaitingVerification {
        return Err(CoreError::Validation(
            "只有待验证状态的任务可以启动验证".to_owned(),
        ));
    }
    Ok(())
}

fn check_task_revision(task: &Task, expected: i64) -> Result<(), CoreError> {
    if task.revision != expected {
        return Err(CoreError::Conflict(format!(
            "任务已被修改（当前 revision {}），请刷新后重试",
            task.revision
        )));
    }
    Ok(())
}

fn check_criterion_revision(
    criterion: &AcceptanceCriterion,
    expected: i64,
) -> Result<(), CoreError> {
    if criterion.revision != expected {
        return Err(CoreError::Conflict(format!(
            "验收条件已被修改（当前 revision {}），请刷新后重试",
            criterion.revision
        )));
    }
    Ok(())
}

fn criterion_by_id(
    connection: &Connection,
    id: &str,
) -> Result<Option<AcceptanceCriterion>, CoreError> {
    connection
        .query_row(
            "SELECT id, task_id, description, kind, required, revision, created_at, updated_at
             FROM acceptance_criteria WHERE id = ?1",
            [id],
            |row| {
                Ok(AcceptanceCriterion {
                    id: row.get(0)?,
                    task_id: row.get(1)?,
                    description: row.get(2)?,
                    kind: row.get(3)?,
                    required: row.get::<_, i64>(4)? != 0,
                    revision: row.get(5)?,
                    created_at: row.get(6)?,
                    updated_at: row.get(7)?,
                })
            },
        )
        .optional()
        .map_err(CoreError::from)
}

fn task_by_id(connection: &Connection, id: &str) -> Result<Option<Task>, CoreError> {
    connection
        .query_row(
            "SELECT id, project_id, title, description, priority, status, revision,
                    block_reason, archived_at, created_at, updated_at
             FROM tasks WHERE id = ?1",
            [id],
            |row| {
                Ok(Task {
                    id: row.get(0)?,
                    project_id: row.get(1)?,
                    title: row.get(2)?,
                    description: row.get(3)?,
                    priority: row.get(4)?,
                    status: row.get(5)?,
                    revision: row.get(6)?,
                    block_reason: row.get(7)?,
                    archived_at: row.get(8)?,
                    created_at: row.get(9)?,
                    updated_at: row.get(10)?,
                })
            },
        )
        .optional()
        .map_err(CoreError::from)
}

fn spec_for_criterion(
    connection: &Connection,
    criterion_id: &str,
) -> Result<Option<VerificationSpec>, CoreError> {
    connection
        .query_row(
            "SELECT id, criterion_id, executable, args_json, cwd, timeout_seconds,
                    output_limit_bytes, success_rule, revision, created_at, updated_at
             FROM verification_specs WHERE criterion_id = ?1",
            [criterion_id],
            spec_from_row,
        )
        .optional()
        .map_err(CoreError::from)
}

fn spec_from_row(row: &Row<'_>) -> rusqlite::Result<VerificationSpec> {
    let args_json: String = row.get(3)?;
    let args: Vec<String> = serde_json::from_str(&args_json).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(3, rusqlite::types::Type::Text, Box::new(error))
    })?;

    Ok(VerificationSpec {
        id: row.get(0)?,
        criterion_id: row.get(1)?,
        executable: row.get(2)?,
        args,
        cwd: row.get(4)?,
        timeout_seconds: row.get(5)?,
        output_limit_bytes: row.get(6)?,
        success_rule: row.get(7)?,
        revision: row.get(8)?,
        created_at: row.get(9)?,
        updated_at: row.get(10)?,
    })
}

fn write_spec(
    connection: &Connection,
    spec: &VerificationSpec,
    args_json: &str,
) -> Result<(), CoreError> {
    connection.execute(
        "INSERT INTO verification_specs
            (id, criterion_id, executable, args_json, cwd, timeout_seconds,
             output_limit_bytes, success_rule, revision, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
         ON CONFLICT(criterion_id) DO UPDATE SET
            executable = excluded.executable,
            args_json = excluded.args_json,
            cwd = excluded.cwd,
            timeout_seconds = excluded.timeout_seconds,
            output_limit_bytes = excluded.output_limit_bytes,
            success_rule = excluded.success_rule,
            revision = excluded.revision,
            updated_at = excluded.updated_at",
        params![
            spec.id,
            spec.criterion_id,
            spec.executable,
            args_json,
            spec.cwd,
            spec.timeout_seconds,
            spec.output_limit_bytes,
            spec.success_rule,
            spec.revision,
            spec.created_at,
            spec.updated_at,
        ],
    )?;
    Ok(())
}

fn insert_run(connection: &Connection, run: &VerificationRun) -> Result<(), CoreError> {
    let args_json = serde_json::to_string(&run.args)
        .map_err(|error| CoreError::Verification(format!("参数序列化失败: {error}")))?;
    let start = run.start_code_state.as_ref();
    connection.execute(
        "INSERT INTO verification_runs
            (id, task_id, criterion_id, criterion_description, spec_id, spec_revision,
             task_revision, criterion_revision, executable, resolved_executable, args_json,
             cwd, timeout_seconds, output_limit_bytes, success_rule, approved_at, started_at,
             execution_status, start_git_head, start_git_branch, start_git_detached,
             start_git_unborn, start_git_dirty, start_code_fingerprint,
             start_fingerprint_complete, start_code_state_captured,
             start_changed_file_count, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14,
                 ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26, ?27, ?28)",
        params![
            run.id,
            run.task_id,
            run.criterion_id,
            run.criterion_description,
            run.spec_id,
            run.spec_revision,
            run.task_revision,
            run.criterion_revision,
            run.executable,
            run.resolved_executable,
            args_json,
            run.cwd,
            run.timeout_seconds,
            run.output_limit_bytes,
            run.success_rule,
            run.approved_at,
            run.started_at,
            run.execution_status,
            start.and_then(|state| state.head.clone()),
            start.and_then(|state| state.branch.clone()),
            start.map(|state| state.detached).unwrap_or(false),
            start.map(|state| state.unborn).unwrap_or(false),
            start.map(|state| state.dirty).unwrap_or(false),
            start.and_then(|state| state.fingerprint.clone()),
            start
                .map(|state| state.fingerprint_complete)
                .unwrap_or(false),
            start.is_some(),
            start
                .map(|state| state.changed_file_count as i64)
                .unwrap_or(0),
            run.created_at,
        ],
    )?;
    Ok(())
}

fn run_by_id(connection: &Connection, id: &str) -> Result<Option<VerificationRun>, CoreError> {
    connection
        .query_row(
            "SELECT * FROM verification_runs WHERE id = ?1",
            [id],
            run_from_row,
        )
        .optional()
        .map_err(CoreError::from)
}

fn latest_run_for_criterion(
    connection: &Connection,
    criterion_id: &str,
) -> Result<Option<VerificationRun>, CoreError> {
    connection
        .query_row(
            "SELECT * FROM verification_runs
             WHERE criterion_id = ?1
             ORDER BY created_at DESC, rowid DESC
             LIMIT 1",
            [criterion_id],
            run_from_row,
        )
        .optional()
        .map_err(CoreError::from)
}

fn run_from_row(row: &Row<'_>) -> rusqlite::Result<VerificationRun> {
    let args_json: String = row.get(10)?;
    let args: Vec<String> = serde_json::from_str(&args_json).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(10, rusqlite::types::Type::Text, Box::new(error))
    })?;

    let start_captured: bool = row.get::<_, i64>(28)? != 0;
    let start_code_state = if start_captured {
        Some(CodeState {
            head: row.get(21)?,
            branch: row.get(22)?,
            detached: row.get::<_, i64>(23)? != 0,
            unborn: row.get::<_, i64>(24)? != 0,
            dirty: row.get::<_, i64>(25)? != 0,
            fingerprint: row.get(26)?,
            fingerprint_complete: row.get::<_, i64>(27)? != 0,
            changed_file_count: row.get::<_, i64>(29)? as usize,
            note: None,
        })
    } else {
        None
    };

    let end_captured: bool = row.get::<_, i64>(37)? != 0;
    let end_code_state = if end_captured {
        Some(CodeState {
            head: row.get(30)?,
            branch: row.get(31)?,
            detached: row.get::<_, i64>(32)? != 0,
            unborn: row.get::<_, i64>(33)? != 0,
            dirty: row.get::<_, i64>(34)? != 0,
            fingerprint: row.get(35)?,
            fingerprint_complete: row.get::<_, i64>(36)? != 0,
            changed_file_count: row.get::<_, i64>(38)? as usize,
            note: row.get(39)?,
        })
    } else {
        None
    };

    Ok(VerificationRun {
        id: row.get(0)?,
        task_id: row.get(1)?,
        criterion_id: row.get(2)?,
        criterion_description: row.get(3)?,
        spec_id: row.get(4)?,
        spec_revision: row.get(5)?,
        task_revision: row.get(6)?,
        criterion_revision: row.get(7)?,
        executable: row.get(8)?,
        resolved_executable: row.get(9)?,
        args,
        cwd: row.get(11)?,
        timeout_seconds: row.get(12)?,
        output_limit_bytes: row.get(13)?,
        success_rule: row.get(14)?,
        approved_at: row.get(15)?,
        started_at: row.get(16)?,
        finished_at: row.get(17)?,
        execution_status: row.get(18)?,
        exit_code: row.get(19)?,
        duration_ms: row.get(20)?,
        start_code_state,
        end_code_state,
        stdout_summary: row.get(40)?,
        stderr_summary: row.get(41)?,
        stdout_bytes: row.get(42)?,
        stderr_bytes: row.get(43)?,
        stdout_truncated: row.get::<_, i64>(44)? != 0,
        stderr_truncated: row.get::<_, i64>(45)? != 0,
        sensitive_flags: split_flags(row.get::<_, String>(46)?),
        output_digest: row.get(47)?,
        error_code: row.get(48)?,
        error_message: row.get(49)?,
        cancel_reason: row.get(50)?,
        created_at: row.get(51)?,
    })
}

fn run_summary_from_row(row: &Row<'_>) -> rusqlite::Result<VerificationRunSummary> {
    let args_json: String = row.get(5)?;
    let args: Vec<String> = serde_json::from_str(&args_json).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(5, rusqlite::types::Type::Text, Box::new(error))
    })?;

    Ok(VerificationRunSummary {
        id: row.get(0)?,
        task_id: row.get(1)?,
        criterion_id: row.get(2)?,
        criterion_description: row.get(3)?,
        executable: row.get(4)?,
        args,
        cwd: row.get(6)?,
        execution_status: row.get(7)?,
        result: row.get(8)?,
        reason: row.get(9)?,
        evidence_id: row.get(10)?,
        spec_revision: row.get(11)?,
        task_revision: row.get(12)?,
        criterion_revision: row.get(13)?,
        started_at: row.get(14)?,
        finished_at: row.get(15)?,
        exit_code: row.get(16)?,
        duration_ms: row.get(17)?,
        stdout_truncated: row.get::<_, i64>(18)? != 0,
        stderr_truncated: row.get::<_, i64>(19)? != 0,
        sensitive_flags: split_flags(row.get::<_, String>(20)?),
        created_at: row.get(21)?,
    })
}

fn check_for_run(
    connection: &Connection,
    run_id: &str,
) -> Result<Option<VerificationCheck>, CoreError> {
    connection
        .query_row(
            "SELECT id, run_id, task_id, criterion_id, result, reason, evidence_id, created_at
             FROM verification_checks WHERE run_id = ?1",
            [run_id],
            |row| {
                Ok(VerificationCheck {
                    id: row.get(0)?,
                    run_id: row.get(1)?,
                    task_id: row.get(2)?,
                    criterion_id: row.get(3)?,
                    result: row.get(4)?,
                    reason: row.get(5)?,
                    evidence_id: row.get(6)?,
                    created_at: row.get(7)?,
                })
            },
        )
        .optional()
        .map_err(CoreError::from)
}

fn evidence_by_id(connection: &Connection, id: &str) -> Result<Option<Evidence>, CoreError> {
    connection
        .query_row(
            "SELECT id, source_type, task_id, run_id, summary, redacted_output,
                    content_digest, sensitive_flags, created_at
             FROM evidence WHERE id = ?1",
            [id],
            |row| {
                Ok(Evidence {
                    id: row.get(0)?,
                    source_type: row.get(1)?,
                    task_id: row.get(2)?,
                    run_id: row.get(3)?,
                    summary: row.get(4)?,
                    redacted_output: row.get(5)?,
                    content_digest: row.get(6)?,
                    sensitive_flags: split_flags(row.get::<_, String>(7)?),
                    created_at: row.get(8)?,
                })
            },
        )
        .optional()
        .map_err(CoreError::from)
}

fn split_flags(value: String) -> Vec<String> {
    value
        .split(',')
        .map(str::trim)
        .filter(|flag| !flag.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

// ---------------------------------------------------------------------------
// 输入校验
// ---------------------------------------------------------------------------

fn validate_executable(raw: &str) -> Result<String, CoreError> {
    let value = raw.trim();
    if value.is_empty() {
        return Err(CoreError::Validation("验证程序不能为空".to_owned()));
    }
    if value.chars().count() > EXECUTABLE_MAX_CHARS {
        return Err(CoreError::Validation(format!(
            "验证程序最多 {EXECUTABLE_MAX_CHARS} 个字符"
        )));
    }
    if value.chars().any(|character| character.is_control()) {
        return Err(CoreError::Validation(
            "验证程序包含不支持的控制字符".to_owned(),
        ));
    }

    let path = Path::new(value);
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(value);
    let stem = file_name
        .trim_end_matches(".exe")
        .trim_end_matches(".com")
        .to_ascii_lowercase();
    if BANNED_SHELLS.contains(&stem.as_str()) {
        return Err(CoreError::Validation(format!(
            "首版禁止直接执行 Shell 可执行文件：{file_name}"
        )));
    }

    if !path.is_absolute() && (value.contains('/') || value.contains('\\')) {
        return Err(CoreError::Validation(
            "验证程序必须是绝对路径或不含路径分隔符的程序名".to_owned(),
        ));
    }

    #[cfg(windows)]
    if let Some(extension) = path.extension().and_then(|extension| extension.to_str()) {
        let extension = extension.to_ascii_lowercase();
        if BANNED_WINDOWS_EXTENSIONS.contains(&extension.as_str()) {
            return Err(CoreError::Validation(
                "首版不支持 .bat/.cmd/.ps1 等脚本包装器；请直接配置 .exe 程序".to_owned(),
            ));
        }
    }

    Ok(value.to_owned())
}

fn validate_arguments(args: Vec<String>) -> Result<Vec<String>, CoreError> {
    if args.len() > MAX_ARGUMENTS {
        return Err(CoreError::Validation(format!(
            "验证参数最多 {MAX_ARGUMENTS} 个"
        )));
    }
    let mut validated = Vec::with_capacity(args.len());
    for argument in args {
        if argument.chars().count() > ARGUMENT_MAX_CHARS {
            return Err(CoreError::Validation(format!(
                "单个参数最多 {ARGUMENT_MAX_CHARS} 个字符"
            )));
        }
        if argument
            .chars()
            .any(|character| character.is_control() && character != '\t')
        {
            return Err(CoreError::Validation(
                "验证参数包含不支持的控制字符".to_owned(),
            ));
        }
        validated.push(argument);
    }
    Ok(validated)
}

fn validate_cwd(root: &Path, requested: Option<&str>) -> Result<String, CoreError> {
    let root = root
        .canonicalize()
        .map_err(|_| CoreError::Validation("项目根目录不可访问".to_owned()))?;
    let requested = requested.unwrap_or("").trim();
    let candidate = if requested.is_empty() {
        root.clone()
    } else {
        PathBuf::from(requested)
    };
    let canonical = candidate
        .canonicalize()
        .map_err(|_| CoreError::Validation("验证工作目录不存在或不可访问".to_owned()))?;
    if !canonical.is_dir() {
        return Err(CoreError::Validation("验证工作目录不是目录".to_owned()));
    }
    if !path_within_root(&root, &canonical) {
        return Err(CoreError::Validation(
            "验证工作目录必须位于项目根目录内".to_owned(),
        ));
    }
    Ok(display_path(canonical))
}

fn validate_cwd_for_run(project_root: &str, cwd: &str) -> Result<(), CoreError> {
    let root = Path::new(project_root)
        .canonicalize()
        .map_err(|_| CoreError::Validation("项目根目录不可访问".to_owned()))?;
    let canonical = Path::new(cwd)
        .canonicalize()
        .map_err(|_| CoreError::Validation("验证工作目录不存在或不可访问".to_owned()))?;
    if !canonical.is_dir() {
        return Err(CoreError::Validation("验证工作目录不是目录".to_owned()));
    }
    if !path_within_root(&root, &canonical) {
        return Err(CoreError::Validation(
            "验证工作目录必须位于项目根目录内".to_owned(),
        ));
    }
    Ok(())
}

fn path_within_root(root: &Path, candidate: &Path) -> bool {
    #[cfg(windows)]
    {
        let root = root
            .to_string_lossy()
            .replace('/', "\\")
            .trim_end_matches('\\')
            .to_ascii_lowercase();
        let candidate = candidate
            .to_string_lossy()
            .replace('/', "\\")
            .to_ascii_lowercase();
        candidate == root || candidate.starts_with(&format!("{root}\\"))
    }
    #[cfg(not(windows))]
    {
        candidate == root || candidate.starts_with(root)
    }
}

fn display_path(path: PathBuf) -> String {
    let text = path.to_string_lossy();
    if let Some(rest) = text.strip_prefix("\\\\?\\") {
        if !rest.starts_with("UNC\\") {
            return rest.to_owned();
        }
    }
    text.into_owned()
}

/// 解析可执行程序的绝对路径；不经过 Shell，也不接受批处理包装器。
pub(crate) fn resolve_executable(executable: &str) -> Option<PathBuf> {
    let path = Path::new(executable);
    if path.is_absolute() {
        return safe_executable_path(path);
    }
    if executable.contains('/') || executable.contains('\\') {
        return None;
    }

    let search_path = std::env::var_os("PATH")?;
    for directory in std::env::split_paths(&search_path) {
        if let Some(resolved) = safe_executable_path(&directory.join(executable)) {
            return Some(resolved);
        }

        #[cfg(windows)]
        if path.extension().is_none() {
            for extension in ["exe", "com"] {
                let candidate = directory.join(format!("{executable}.{extension}"));
                if let Some(resolved) = safe_executable_path(&candidate) {
                    return Some(resolved);
                }
            }
        }
    }
    None
}

fn safe_executable_path(path: &Path) -> Option<PathBuf> {
    let canonical = path.canonicalize().ok()?;
    if !canonical.is_file() {
        return None;
    }

    #[cfg(windows)]
    {
        let extension = canonical
            .extension()
            .and_then(|extension| extension.to_str())
            .map(|extension| extension.to_ascii_lowercase())?;
        if !matches!(extension.as_str(), "exe" | "com") {
            return None;
        }
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if canonical.metadata().ok()?.permissions().mode() & 0o111 == 0 {
            return None;
        }
    }

    Some(PathBuf::from(display_path(canonical)))
}

// ---------------------------------------------------------------------------
// 报告
// ---------------------------------------------------------------------------

fn build_report_markdown(
    detail: &crate::models::TaskDetail,
    runs: &[VerificationRunSummary],
    evidence_outputs: &std::collections::HashMap<String, Evidence>,
    project_name: &str,
) -> String {
    let mut output = String::new();
    output.push_str(&format!("# 验收报告：{}\n\n", detail.task.title));
    output.push_str(&format!("- 生成时间（UTC）：{}\n", now_utc_rfc3339()));
    output.push_str(&format!("- 任务 ID：`{}`\n", detail.task.id));
    output.push_str(&format!("- 项目：{}\n", project_name));
    output.push_str(&format!(
        "- 任务状态：{}；修订：{}\n",
        detail.task.status, detail.task.revision
    ));
    output.push_str(&format!(
        "- 完成门禁：{}（必需 {}/{} 已满足）\n",
        if detail.completion.can_complete {
            "可完成"
        } else {
            "不可完成"
        },
        detail.completion.satisfied_required_count,
        detail.completion.required_count
    ));
    output.push_str("\n## 验收条件\n\n");
    output.push_str("| 类型 | 必需 | 条件 | 有效状态 | 说明 |\n");
    output.push_str("|---|---|---|---|---|\n");
    for view in &detail.criteria {
        let (status, detail_text) = match &view.verification {
            Some(verification) => (
                verification.status.as_str().to_owned(),
                verification
                    .stale_reason
                    .clone()
                    .or_else(|| verification.detail.clone())
                    .unwrap_or_default(),
            ),
            None => (
                if view.confirmation.is_some() {
                    "pass".to_owned()
                } else {
                    "not_run".to_owned()
                },
                view.confirmation
                    .as_ref()
                    .map(|confirmation| format!("人工确认于 {}", confirmation.confirmed_at))
                    .unwrap_or_else(|| "尚未人工确认".to_owned()),
            ),
        };
        output.push_str(&format!(
            "| {} | {} | {} | {} | {} |\n",
            match view.criterion.kind {
                CriterionKind::Auto => "自动",
                CriterionKind::Manual => "人工",
            },
            if view.criterion.required {
                "是"
            } else {
                "否"
            },
            escape_markdown_cell(&view.criterion.description),
            status,
            escape_markdown_cell(&detail_text),
        ));
    }

    if !detail.completion.blockers.is_empty() {
        output.push_str("\n## 阻塞项\n\n");
        for blocker in &detail.completion.blockers {
            output.push_str(&format!("- {}\n", blocker));
        }
    }

    output.push_str("\n## 验证历史\n\n");
    if runs.is_empty() {
        output.push_str("尚无验证运行记录。\n");
    } else {
        for run in runs {
            output.push_str(&format!(
                "### {} · {}\n\n",
                run.criterion_description, run.created_at
            ));
            output.push_str(&format!(
                "- 运行 ID：`{}`\n- 执行状态：{}；结果：{}\n",
                run.id,
                run.execution_status,
                run.result
                    .map(|result| result.as_str().to_owned())
                    .unwrap_or_else(|| "无".to_owned())
            ));
            output.push_str("\n**命令**\n\n```text\n");
            output.push_str(&report_command(&run.executable, &run.args));
            output.push_str("\n```\n\n");
            output.push_str(&format!("- 工作目录：`{}`\n", run.cwd.replace('`', "'")));
            output.push_str(&format!(
                "- 任务修订 {}；条件修订 {}；配置修订 {}\n",
                run.task_revision, run.criterion_revision, run.spec_revision
            ));
            output.push_str(&format!(
                "- 退出码：{}；耗时：{} ms\n",
                run.exit_code
                    .map(|code| code.to_string())
                    .unwrap_or_else(|| "无".to_owned()),
                run.duration_ms
                    .map(|duration| duration.to_string())
                    .unwrap_or_else(|| "无".to_owned())
            ));
            if let Some(evidence) = run
                .evidence_id
                .as_ref()
                .and_then(|id| evidence_outputs.get(id))
            {
                output.push_str(&format!(
                    "- 证据：`{}`；内容摘要：`{}`\n",
                    evidence.id, evidence.content_digest
                ));
                output.push_str(&format!(
                    "- 敏感性标记：{}\n",
                    if evidence.sensitive_flags.is_empty() {
                        "无".to_owned()
                    } else {
                        evidence.sensitive_flags.join(", ")
                    }
                ));
                if let Some(text) = &evidence.redacted_output {
                    output.push_str("\n```text\n");
                    output.push_str(&truncate_chars(text, 4000));
                    output.push_str("\n```\n");
                }
            }
            output.push('\n');
        }
    }

    output.push_str(
        "\n---\n\n本报告由 XUSHU 根据本地验证记录生成。验证通过只代表指定命令在指定代码状态下退出码为 0，不代表代码绝对正确；历史结果会随任务、条件、配置或代码状态变化而过期。报告可能包含本地路径与命令输出，分享前请检查。\n",
    );
    output
}

fn escape_markdown_cell(value: &str) -> String {
    value.replace('|', "\\|").replace(['\n', '\r'], " ")
}

fn report_command(executable: &str, args: &[String]) -> String {
    let quote = |value: &str| {
        if value.is_empty() {
            "\"\"".to_owned()
        } else if value.contains([' ', '\t', '"', '\\']) {
            format!("\"{}\"", value.replace('"', "\\\""))
        } else {
            value.to_owned()
        }
    };

    std::iter::once(executable.to_owned())
        .chain(args.iter().cloned())
        .map(|value| quote(&value))
        .collect::<Vec<_>>()
        .join(" ")
}

fn truncate_chars(value: &str, max_chars: usize) -> String {
    let mut output: String = value.chars().take(max_chars).collect();
    if value.chars().count() > max_chars {
        output.push_str("\n…（输出已截断）");
    }
    output
}

// ---------------------------------------------------------------------------
// 单元测试：数据库失败与回滚
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    const RUN_COLUMNS: &str = "id, task_id, criterion_id, criterion_description, spec_id,
        spec_revision, task_revision, criterion_revision, executable, resolved_executable,
        args_json, cwd, timeout_seconds, output_limit_bytes, success_rule, approved_at,
        started_at, finished_at, execution_status, exit_code, duration_ms,
        start_git_head, start_git_branch, start_git_detached, start_git_unborn,
        start_git_dirty, start_code_fingerprint, start_fingerprint_complete,
        start_code_state_captured, start_changed_file_count, end_git_head, end_git_branch,
        end_git_detached, end_git_unborn, end_git_dirty, end_code_fingerprint,
        end_fingerprint_complete, end_code_state_captured, end_changed_file_count,
        code_state_note, stdout_summary, stderr_summary, stdout_bytes, stderr_bytes,
        stdout_truncated, stderr_truncated, sensitive_flags, output_digest, error_code,
        error_message, cancel_reason, created_at";

    fn fixture() -> Connection {
        let mut connection = Connection::open_in_memory().expect("open");
        crate::run_migrations(&mut connection, crate::MIGRATIONS).expect("migrate");
        connection
            .execute_batch(
                "INSERT INTO projects (id, name, root_path, root_path_key, created_at, updated_at)
                 VALUES ('p1', 'P', 'C:\\p', 'c:\\p', '2026-01-01T00:00:00.000Z', '2026-01-01T00:00:00.000Z');
                 INSERT INTO tasks (id, project_id, title, description, priority, status, revision, created_at, updated_at)
                 VALUES ('t1', 'p1', 'T', '', 'medium', 'awaiting_verification', 3, '2026-01-01T00:00:00.000Z', '2026-01-01T00:00:00.000Z');
                 INSERT INTO acceptance_criteria (id, task_id, description, kind, required, revision, created_at, updated_at)
                 VALUES ('c1', 't1', 'C', 'auto', 1, 1, '2026-01-01T00:00:00.000Z', '2026-01-01T00:00:00.000Z');
                 INSERT INTO verification_specs (id, criterion_id, executable, args_json, cwd, timeout_seconds, output_limit_bytes, success_rule, revision, created_at, updated_at)
                 VALUES ('s1', 'c1', 'git', '[]', 'C:\\p', 30, 1024, 'exit_code_zero', 1, '2026-01-01T00:00:00.000Z', '2026-01-01T00:00:00.000Z');",
            )
            .expect("fixture");
        connection
            .execute(
                &format!(
                    "INSERT INTO verification_runs ({RUN_COLUMNS}) VALUES (
                        'r1', 't1', 'c1', 'C', 's1', 1, 3, 1, 'git', NULL, '[]', 'C:\\p',
                        30, 1024, 'exit_code_zero', '2026-01-01T00:00:00.000Z',
                        '2026-01-01T00:00:00.000Z', NULL, 'running', NULL, NULL,
                        'abc', 'main', 0, 0, 0, 'sha256:one', 1, 1, 1, NULL, NULL, 0, 0, 0,
                        NULL, 0, 0, 0, NULL, NULL, NULL, NULL, NULL, 0, 0, '', NULL, NULL,
                        NULL, NULL, '2026-01-01T00:00:00.000Z'
                    )"
                ),
                [],
            )
            .expect("run");
        connection
    }

    fn passing_outcome() -> RunOutcome {
        RunOutcome {
            execution_status: ExecutionStatus::Completed,
            exit_code: Some(0),
            duration_ms: 12,
            stdout: "ok".to_owned(),
            stderr: String::new(),
            stdout_bytes: 2,
            stderr_bytes: 0,
            stdout_truncated: false,
            stderr_truncated: false,
            note: None,
        }
    }

    fn end_state() -> CodeState {
        CodeState {
            head: Some("abc".to_owned()),
            branch: Some("main".to_owned()),
            detached: false,
            unborn: false,
            dirty: false,
            fingerprint: Some("sha256:one".to_owned()),
            fingerprint_complete: true,
            changed_file_count: 0,
            note: None,
        }
    }

    #[test]
    fn write_failure_rolls_back_run_evidence_and_check() {
        let mut connection = fixture();
        let transaction = connection.transaction().expect("tx");
        // 预先插入同 run 的 check，让 finalize 的最后一步触发唯一约束失败。
        transaction
            .execute(
                "INSERT INTO verification_checks
                    (id, run_id, task_id, criterion_id, result, reason, evidence_id, created_at)
                 VALUES ('conflict', 'r1', 't1', 'c1', 'inconclusive', 'synthetic', NULL, '2026-01-01T00:00:00.000Z')",
                [],
            )
            .expect("conflict check");

        let result = finalize_run_on(&transaction, "r1", &passing_outcome(), &Ok(end_state()));
        assert!(result.is_err(), "finalize must fail on unique constraint");
        drop(transaction);

        let status: String = connection
            .query_row(
                "SELECT execution_status FROM verification_runs WHERE id = 'r1'",
                [],
                |row| row.get(0),
            )
            .expect("status");
        assert_eq!(status, "running");
        let evidence_count: i64 = connection
            .query_row("SELECT COUNT(*) FROM evidence", [], |row| row.get(0))
            .expect("evidence count");
        assert_eq!(evidence_count, 0);
    }

    #[test]
    fn read_only_database_reports_write_failure_without_partial_state() {
        let connection = fixture();
        connection
            .pragma_update(None, "query_only", "ON")
            .expect("query_only");
        let result = finalize_run_on(&connection, "r1", &passing_outcome(), &Ok(end_state()));
        assert!(result.is_err());
        let status: String = connection
            .query_row(
                "SELECT execution_status FROM verification_runs WHERE id = 'r1'",
                [],
                |row| row.get(0),
            )
            .expect("status");
        assert_eq!(status, "running");
    }
}
