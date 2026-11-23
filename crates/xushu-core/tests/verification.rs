use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread;
use std::time::Duration;

use tempfile::TempDir;
use xushu_core::{
    code_state, runner, AcceptanceCriterion, CoreError, CreateCriterionInput, CreateTaskInput,
    CriterionKind, Database, ImportProjectInput, Project, ProjectService, RunControl,
    SaveVerificationSpecInput, StartVerificationInput, Task, TaskDetail, TaskService, TaskStatus,
    TransitionTaskInput, UpdateCriterionInput, UpdateTaskInput, VerificationResult,
    VerificationRun, VerificationService, VerificationSpec, VerificationTemplate,
};

fn temp_database() -> (TempDir, Database) {
    let directory = TempDir::new().expect("temp directory");
    let database = Database::open(directory.path().join("xushu.db")).expect("open database");
    (directory, database)
}

fn run_git(path: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .arg("-C")
        .arg(path)
        .args(args)
        .output()
        .expect("run git");
    assert!(
        output.status.success(),
        "git {:?} failed: {}",
        args,
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).trim().to_owned()
}

fn init_repo() -> (TempDir, PathBuf) {
    let directory = TempDir::new().expect("temp directory");
    let path = directory.path().join("sample-repo");
    fs::create_dir_all(&path).expect("create repo directory");
    run_git(&path, &["init", "-b", "main"]);
    run_git(&path, &["config", "user.email", "test@xushu.local"]);
    run_git(&path, &["config", "user.name", "XUSHU Test"]);
    fs::write(path.join("README.md"), "initial\n").expect("write readme");
    run_git(&path, &["add", "-A"]);
    run_git(&path, &["commit", "-m", "initial"]);
    (directory, path)
}

fn import_project(database: &Database, path: &Path) -> Project {
    ProjectService::new(database)
        .import(ImportProjectInput {
            root_path: path.to_string_lossy().to_string(),
            name: None,
        })
        .expect("import project")
}

fn create_task(database: &Database, project_id: &str) -> Task {
    TaskService::new(database)
        .create_task(CreateTaskInput {
            project_id: project_id.to_owned(),
            title: "Phase 3 验证任务".to_owned(),
            description: Some("验证引擎测试".to_owned()),
            priority: "medium".to_owned(),
        })
        .expect("create task")
}

fn current_detail(database: &Database, task_id: &str) -> TaskDetail {
    TaskService::new(database)
        .get_task(task_id)
        .expect("task detail")
}

fn add_criterion(
    database: &Database,
    task_id: &str,
    kind: &str,
    required: bool,
    description: &str,
) -> AcceptanceCriterion {
    let detail = current_detail(database, task_id);
    TaskService::new(database)
        .create_criterion(CreateCriterionInput {
            task_id: task_id.to_owned(),
            expected_task_revision: detail.task.revision,
            description: description.to_owned(),
            kind: kind.to_owned(),
            required,
        })
        .expect("create criterion")
}

fn transition(
    database: &Database,
    task_id: &str,
    to: &str,
    reason: Option<&str>,
) -> Result<Task, CoreError> {
    let detail = current_detail(database, task_id);
    TaskService::new(database).transition_task_status(TransitionTaskInput {
        task_id: task_id.to_owned(),
        expected_revision: detail.task.revision,
        to_status: to.to_owned(),
        reason: reason.map(str::to_owned),
    })
}

fn transition_ok(database: &Database, task_id: &str, to: &str) {
    transition(database, task_id, to, None).expect("transition should succeed");
}

fn advance_to_awaiting(database: &Database, task_id: &str) {
    transition_ok(database, task_id, "ready");
    transition_ok(database, task_id, "running");
    transition_ok(database, task_id, "awaiting_verification");
}

fn criterion_view(database: &Database, task_id: &str, criterion_id: &str) -> AcceptanceCriterion {
    current_detail(database, task_id)
        .criteria
        .into_iter()
        .find(|view| view.criterion.id == criterion_id)
        .expect("criterion view")
        .criterion
}

#[allow(clippy::too_many_arguments)]
fn save_spec(
    database: &Database,
    task_id: &str,
    criterion_id: &str,
    executable: &str,
    args: &[&str],
    cwd: Option<&Path>,
    timeout_seconds: i64,
    output_limit_bytes: i64,
) -> Result<VerificationSpec, CoreError> {
    let detail = current_detail(database, task_id);
    let criterion = detail
        .criteria
        .iter()
        .find(|view| view.criterion.id == criterion_id)
        .expect("criterion");
    let existing = VerificationService::new(database)
        .get_spec(criterion_id)
        .expect("existing spec");

    VerificationService::new(database).save_spec(SaveVerificationSpecInput {
        criterion_id: criterion_id.to_owned(),
        expected_task_revision: detail.task.revision,
        expected_criterion_revision: criterion.criterion.revision,
        expected_spec_revision: existing.map(|spec| spec.revision),
        executable: executable.to_owned(),
        args: args.iter().map(|arg| (*arg).to_owned()).collect(),
        cwd: cwd.map(|path| path.to_string_lossy().to_string()),
        timeout_seconds,
        output_limit_bytes,
    })
}

fn prepare_run(
    database: &Database,
    task_id: &str,
    criterion_id: &str,
    approved: bool,
) -> Result<xushu_core::verification::PreparedRun, CoreError> {
    let detail = current_detail(database, task_id);
    let criterion = detail
        .criteria
        .iter()
        .find(|view| view.criterion.id == criterion_id)
        .expect("criterion");
    let spec = VerificationService::new(database)
        .get_spec(criterion_id)
        .expect("spec")
        .expect("spec exists");
    VerificationService::new(database).prepare_run(StartVerificationInput {
        criterion_id: criterion_id.to_owned(),
        expected_task_revision: detail.task.revision,
        expected_criterion_revision: criterion.criterion.revision,
        expected_spec_revision: spec.revision,
        approved,
    })
}

fn execute_prepared(
    database: &Database,
    prepared: xushu_core::verification::PreparedRun,
) -> VerificationRun {
    let control = RunControl::new();
    let outcome = match prepared.preflight_error.clone() {
        Some(message) => runner::RunOutcome::start_failed(message),
        None => runner::execute(&prepared.plan, &control),
    };
    let end_code_state =
        code_state::capture(&prepared.project_root).map_err(|error| error.code().to_owned());
    VerificationService::new(database)
        .finalize_run(&prepared.run.id, outcome, end_code_state)
        .expect("finalize")
}

fn run_approved(database: &Database, task_id: &str, criterion_id: &str) -> VerificationRun {
    let prepared = prepare_run(database, task_id, criterion_id, true).expect("prepare");
    execute_prepared(database, prepared)
}

fn detail_with_status(database: &Database, task_id: &str) -> VerificationResult {
    let detail = current_detail(database, task_id);
    let view = detail
        .criteria
        .iter()
        .find(|view| view.criterion.kind == CriterionKind::Auto)
        .expect("auto criterion");
    view.verification
        .as_ref()
        .expect("verification view")
        .status
}

fn create_auto_task(database: &Database, repo: &Path) -> (Task, String) {
    let project = import_project(database, repo);
    let task = create_task(database, &project.id);
    let criterion = add_criterion(database, &task.id, "auto", true, "自动验收条件");
    advance_to_awaiting(database, &task.id);
    (task, criterion.id)
}

#[test]
fn templates_are_available_without_shell_wrappers() {
    let (_directory, database) = temp_database();
    let templates: Vec<VerificationTemplate> = VerificationService::new(&database).templates();
    assert!(!templates.is_empty());
    for template in templates {
        let lower = template.executable.to_ascii_lowercase();
        assert_ne!(lower, "cmd");
        assert_ne!(lower, "powershell");
        assert_ne!(lower, "pwsh");
        assert_ne!(lower, "sh");
        assert_ne!(lower, "bash");
    }
}

#[test]
fn rejects_unapproved_runs_and_shell_executables() {
    let (_repo_directory, repo) = init_repo();
    let (_database_directory, database) = temp_database();
    let (task, criterion_id) = create_auto_task(&database, &repo);
    save_spec(
        &database,
        &task.id,
        &criterion_id,
        "git",
        &["--version"],
        None,
        30,
        4096,
    )
    .expect("save spec");

    let unapproved = prepare_run(&database, &task.id, &criterion_id, false).expect_err("approval");
    assert!(matches!(unapproved, CoreError::ApprovalRequired));

    let shell = save_spec(
        &database,
        &task.id,
        &criterion_id,
        "cmd",
        &["/c", "echo"],
        None,
        30,
        4096,
    );
    assert!(matches!(shell, Err(CoreError::Validation(_))));
}

#[test]
fn rejects_cwd_outside_project() {
    let (_repo_directory, repo) = init_repo();
    let (_database_directory, database) = temp_database();
    let outside = TempDir::new().expect("outside");
    let (task, criterion_id) = create_auto_task(&database, &repo);

    let result = save_spec(
        &database,
        &task.id,
        &criterion_id,
        "git",
        &["--version"],
        Some(outside.path()),
        30,
        4096,
    );
    assert!(matches!(result, Err(CoreError::Validation(_))));
}

#[test]
fn successful_run_satisfies_auto_criterion_and_allows_done() {
    let (_repo_directory, repo) = init_repo();
    let (_database_directory, database) = temp_database();
    let (task, criterion_id) = create_auto_task(&database, &repo);
    save_spec(
        &database,
        &task.id,
        &criterion_id,
        "git",
        &["--version"],
        None,
        30,
        4096,
    )
    .expect("save spec");

    let run = run_approved(&database, &task.id, &criterion_id);
    assert_eq!(run.execution_status, "completed");
    assert_eq!(run.exit_code, Some(0));

    let check = VerificationService::new(&database)
        .get_check(&run.id)
        .expect("check")
        .expect("check exists");
    let evidence = VerificationService::new(&database)
        .get_evidence(&check.evidence_id.expect("evidence id"))
        .expect("evidence");
    assert!(
        evidence.summary.contains("退出码 0"),
        "unexpected summary: {}",
        evidence.summary
    );
    assert!(!evidence.summary.contains("Some("));

    let detail = current_detail(&database, &task.id);
    let view = &detail.criteria[0].verification.as_ref().expect("view");
    assert_eq!(view.status, VerificationResult::Pass);
    assert!(detail.criteria[0].satisfied);
    assert!(detail.completion.can_complete);

    transition_ok(&database, &task.id, "done");
    assert_eq!(
        current_detail(&database, &task.id).task.status,
        TaskStatus::Done
    );
}

#[test]
fn failing_command_blocks_done() {
    let (_repo_directory, repo) = init_repo();
    let (_database_directory, database) = temp_database();
    let (task, criterion_id) = create_auto_task(&database, &repo);
    save_spec(
        &database,
        &task.id,
        &criterion_id,
        "git",
        &["rev-parse", "--verify", "refs/heads/definitely-missing"],
        None,
        30,
        4096,
    )
    .expect("save spec");

    let run = run_approved(&database, &task.id, &criterion_id);
    assert_eq!(run.execution_status, "completed");
    assert_ne!(run.exit_code, Some(0));
    assert_eq!(
        detail_with_status(&database, &task.id),
        VerificationResult::Fail
    );

    let blocked = transition(&database, &task.id, "done", None).expect_err("must block");
    match blocked {
        CoreError::TaskNotCompletable(message) => {
            assert!(message.contains("未通过"), "unexpected: {message}");
        }
        other => panic!("unexpected error: {other:?}"),
    }
}

#[test]
fn missing_command_is_not_run() {
    let (_repo_directory, repo) = init_repo();
    let (_database_directory, database) = temp_database();
    let (task, criterion_id) = create_auto_task(&database, &repo);
    save_spec(
        &database,
        &task.id,
        &criterion_id,
        "xushu-missing-binary-123456789",
        &[],
        None,
        30,
        4096,
    )
    .expect("save spec");

    let run = run_approved(&database, &task.id, &criterion_id);
    assert_eq!(run.execution_status, "start_failed");
    assert_eq!(
        detail_with_status(&database, &task.id),
        VerificationResult::NotRun
    );
    assert!(transition(&database, &task.id, "done", None).is_err());
}

#[test]
fn invalid_working_directory_is_not_run() {
    let (_repo_directory, repo) = init_repo();
    let (_database_directory, database) = temp_database();
    let subdirectory = repo.join("sub");
    fs::create_dir_all(&subdirectory).expect("subdir");
    let (task, criterion_id) = create_auto_task(&database, &repo);
    save_spec(
        &database,
        &task.id,
        &criterion_id,
        "git",
        &["--version"],
        Some(subdirectory.as_path()),
        30,
        4096,
    )
    .expect("save spec");
    fs::remove_dir_all(&subdirectory).expect("remove subdir");

    let run = run_approved(&database, &task.id, &criterion_id);
    assert_eq!(run.execution_status, "start_failed");
    assert_eq!(
        detail_with_status(&database, &task.id),
        VerificationResult::NotRun
    );
}

#[test]
fn timeout_is_fail_not_pass() {
    let (_repo_directory, repo) = init_repo();
    let (_database_directory, database) = temp_database();
    let (task, criterion_id) = create_auto_task(&database, &repo);
    save_spec(
        &database,
        &task.id,
        &criterion_id,
        "node",
        &["-e", "setTimeout(()=>{}, 5000)"],
        None,
        1,
        4096,
    )
    .expect("save spec");

    let run = run_approved(&database, &task.id, &criterion_id);
    assert_eq!(run.execution_status, "timed_out");
    assert_eq!(
        detail_with_status(&database, &task.id),
        VerificationResult::Fail
    );
    assert!(transition(&database, &task.id, "done", None).is_err());
}

#[test]
fn cancellation_is_inconclusive_and_does_not_fake_pass() {
    let (_repo_directory, repo) = init_repo();
    let (_database_directory, database) = temp_database();
    let (task, criterion_id) = create_auto_task(&database, &repo);
    save_spec(
        &database,
        &task.id,
        &criterion_id,
        "node",
        &["-e", "setTimeout(()=>{}, 5000)"],
        None,
        30,
        4096,
    )
    .expect("save spec");

    let prepared = prepare_run(&database, &task.id, &criterion_id, true).expect("prepare");
    let control = RunControl::new();
    let control_for_thread = control.clone();
    let handle = thread::spawn(move || runner::execute(&prepared.plan, &control_for_thread));
    thread::sleep(Duration::from_millis(500));
    control.cancel();
    let outcome = handle.join().expect("join runner");
    assert_eq!(outcome.execution_status, runner::ExecutionStatus::Cancelled);

    let end_state =
        code_state::capture(&prepared.project_root).map_err(|error| error.code().to_owned());
    let run = VerificationService::new(&database)
        .finalize_run(&prepared.run.id, outcome, end_state)
        .expect("finalize");
    assert_eq!(run.execution_status, "cancelled");
    assert_eq!(
        detail_with_status(&database, &task.id),
        VerificationResult::Inconclusive
    );
    assert!(transition(&database, &task.id, "done", None).is_err());
}

#[cfg(windows)]
#[test]
fn cancellation_kills_child_process_tree() {
    let (_repo_directory, repo) = init_repo();
    let (_database_directory, database) = temp_database();
    let marker_directory = TempDir::new().expect("marker directory");
    let marker = marker_directory.path().join("marker.txt");
    let (task, criterion_id) = create_auto_task(&database, &repo);

    let inner = format!(
        "setTimeout(()=>require('fs').writeFileSync({:?}, 'x'), 3000)",
        marker.to_string_lossy()
    );
    let outer = format!(
        "require('child_process').spawn(process.execPath, ['-e', {:?}], {{stdio: 'ignore'}}); setTimeout(()=>{{}}, 5000)",
        inner
    );
    save_spec(
        &database,
        &task.id,
        &criterion_id,
        "node",
        &["-e", &outer],
        None,
        30,
        4096,
    )
    .expect("save spec");

    let prepared = prepare_run(&database, &task.id, &criterion_id, true).expect("prepare");
    let control = RunControl::new();
    let control_for_thread = control.clone();
    let handle = thread::spawn(move || runner::execute(&prepared.plan, &control_for_thread));
    thread::sleep(Duration::from_millis(500));
    control.cancel();
    let _ = handle.join().expect("join runner");

    thread::sleep(Duration::from_secs(4));
    assert!(
        !marker.exists(),
        "child process survived cancellation and wrote marker"
    );
}

#[cfg(unix)]
#[test]
fn unix_runner_cleans_up_descendants_with_inherited_pipes() {
    use std::time::Instant;

    let fixture = TempDir::new().unwrap();
    let outcomes = [
        runner::ExecutionStatus::Completed,
        runner::ExecutionStatus::Cancelled,
        runner::ExecutionStatus::TimedOut,
    ];
    for (index, expected) in outcomes.into_iter().enumerate() {
        let marker = fixture.path().join(format!("descendant-{index}"));
        let ready = fixture.path().join(format!("ready-{index}"));
        let plan = runner::RunPlan {
            executable: "node".to_owned(),
            args: [
                "-e",
                "const [ready, marker, linger] = process.argv.slice(1);
                 const child = require('child_process').spawn(process.execPath, [
                   '-e', 'setTimeout(() => require(\"fs\").writeFileSync(process.argv[1], \"late\"), 2000)',
                   marker
                 ], {stdio: 'inherit'});
                 child.once('spawn', () => {
                   require('fs').writeFileSync(ready, 'started');
                   if (linger === 'false') process.exit(0);
                 });",
                ready.to_str().unwrap(),
                marker.to_str().unwrap(),
                if expected == runner::ExecutionStatus::Completed { "false" } else { "true" },
            ].map(str::to_owned).to_vec(),
            cwd: fixture.path().to_owned(),
            timeout: Duration::from_secs(if expected == runner::ExecutionStatus::TimedOut { 1 } else { 5 }),
            output_limit_bytes: 4096,
        };
        let control = RunControl::new();
        let run_control = control.clone();
        let start = Instant::now();
        let worker = thread::spawn(move || runner::execute(&plan, &run_control));
        while !ready.exists() && start.elapsed() < Duration::from_secs(1) {
            thread::sleep(Duration::from_millis(10));
        }
        assert!(ready.exists(), "fixture never started its descendant");
        if expected == runner::ExecutionStatus::Cancelled {
            control.cancel();
        }
        let outcome = worker.join().unwrap();
        assert_eq!(outcome.execution_status, expected, "{:?}", outcome.note);
        assert!(
            start.elapsed() < Duration::from_millis(1800),
            "{expected:?} waited for a descendant"
        );
    }
    thread::sleep(Duration::from_secs(2));
    for index in 0..outcomes.len() {
        assert!(!fixture.path().join(format!("descendant-{index}")).exists());
    }
}

#[cfg(windows)]
#[test]
fn windows_job_reaps_the_tree_and_closes_both_pipes_before_reporting() {
    use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};
    use std::process::{Command, Stdio};
    use std::time::Instant;
    use windows_sys::Win32::Foundation::{WAIT_OBJECT_0, WAIT_TIMEOUT};
    use windows_sys::Win32::System::Threading::{
        OpenProcess, WaitForSingleObject, PROCESS_SYNCHRONIZE,
    };

    let fixture = TempDir::new().unwrap();
    let script = fixture.path().join("tree.cjs");
    fs::write(
        &script,
        r#"
const fs = require('node:fs');
const { spawn } = require('node:child_process');
const [role, ready, exitSignal, code, ...ancestors] = process.argv.slice(2);
if (role === 'leaf') {
  console.log('leaf stdout');
  console.error('leaf stderr');
  fs.writeFileSync(ready + '.pids', [...ancestors, process.pid].join('\n'));
  fs.writeFileSync(ready, 'ready');
} else {
  spawn(process.execPath, [__filename, role === 'root' ? 'child' : 'leaf',
    ready, exitSignal, code, ...ancestors, process.pid], { stdio: 'inherit' })
    .on('error', error => { console.error(error); process.exit(98); });
  if (role === 'root' && code !== '-') {
    setInterval(() => {
      if (fs.existsSync(exitSignal)) process.exit(Number(code));
    }, 10);
  }
}
setTimeout(() => process.exit(99), 15000);
"#,
    )
    .unwrap();

    // Hold a real process handle so the assertion cannot follow a reused PID.
    let mut unrelated = Command::new("node")
        .args(["-e", "setTimeout(() => {}, 20000)"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    for (index, (expected, root_code)) in [
        (runner::ExecutionStatus::Completed, "0"),
        (runner::ExecutionStatus::Completed, "37"),
        (runner::ExecutionStatus::Cancelled, "-"),
        (runner::ExecutionStatus::TimedOut, "-"),
    ]
    .into_iter()
    .enumerate()
    {
        let ready = fixture.path().join(format!("ready-{index}"));
        let exit_signal = fixture.path().join(format!("exit-{index}"));
        let plan = runner::RunPlan {
            executable: "node".into(),
            args: [
                script.to_str().unwrap(),
                "root",
                ready.to_str().unwrap(),
                exit_signal.to_str().unwrap(),
                root_code,
            ]
            .map(str::to_owned)
            .to_vec(),
            cwd: fixture.path().to_owned(),
            timeout: Duration::from_secs(if expected == runner::ExecutionStatus::TimedOut {
                4
            } else {
                10
            }),
            output_limit_bytes: 4096,
        };
        let control = RunControl::new();
        let worker_control = control.clone();
        let worker = thread::spawn(move || runner::execute(&plan, &worker_control));
        let deadline = Instant::now() + Duration::from_secs(3);
        while !ready.is_file() && Instant::now() < deadline {
            thread::sleep(Duration::from_millis(10));
        }
        let pids = match fs::read_to_string(ready.with_extension("pids")) {
            Ok(pids) => pids,
            Err(error) => {
                control.cancel();
                panic!(
                    "scenario {index} did not start its grandchild: {error}; {:?}",
                    worker.join().unwrap()
                );
            }
        };
        let handles: Vec<_> = pids
            .lines()
            .map(|pid| {
                let handle = unsafe { OpenProcess(PROCESS_SYNCHRONIZE, 0, pid.parse().unwrap()) };
                assert!(
                    !handle.is_null(),
                    "fixture PID {pid}: {}",
                    std::io::Error::last_os_error()
                );
                let handle = unsafe { OwnedHandle::from_raw_handle(handle) };
                assert_eq!(
                    unsafe { WaitForSingleObject(handle.as_raw_handle(), 0) },
                    WAIT_TIMEOUT
                );
                handle
            })
            .collect();
        assert_eq!(handles.len(), 3);

        if expected == runner::ExecutionStatus::Cancelled {
            control.cancel();
        } else if expected == runner::ExecutionStatus::Completed {
            fs::write(&exit_signal, "exit").unwrap();
        }
        let result = worker.join().unwrap();
        assert_eq!(result.execution_status, expected, "{result:?}");
        if let Ok(code) = root_code.parse::<i32>() {
            assert_eq!(result.exit_code, Some(code));
        }
        assert!(result.stdout.contains("leaf stdout"), "{result:?}");
        assert!(result.stderr.contains("leaf stderr"), "{result:?}");
        assert!(
            !result.stdout_truncated && !result.stderr_truncated,
            "{result:?}"
        );
        for handle in handles {
            assert_eq!(
                unsafe { WaitForSingleObject(handle.as_raw_handle(), 0) },
                WAIT_OBJECT_0
            );
        }
        assert!(
            unrelated.try_wait().unwrap().is_none(),
            "unrelated process was stopped"
        );
    }
    unrelated.kill().unwrap();
    unrelated.wait().unwrap();
}

#[test]
fn output_limit_truncates_but_drains_pipe() {
    let (_repo_directory, repo) = init_repo();
    let (_database_directory, database) = temp_database();
    let (task, criterion_id) = create_auto_task(&database, &repo);
    save_spec(
        &database,
        &task.id,
        &criterion_id,
        "node",
        &["-e", "process.stdout.write('x'.repeat(200000))"],
        None,
        30,
        1024,
    )
    .expect("save spec");

    let run = run_approved(&database, &task.id, &criterion_id);
    assert_eq!(run.execution_status, "completed");
    assert!(run.stdout_truncated);
    assert!(run.stdout_bytes.expect("bytes") >= 200_000);
    let stored = run.stdout_summary.expect("stdout");
    assert!(
        stored.len() <= 1100,
        "stored output was not bounded: {}",
        stored.len()
    );
}

#[test]
fn sensitive_output_is_redacted_in_evidence() {
    let (_repo_directory, repo) = init_repo();
    let (_database_directory, database) = temp_database();
    let (task, criterion_id) = create_auto_task(&database, &repo);
    save_spec(
        &database,
        &task.id,
        &criterion_id,
        "node",
        &[
            "-e",
            "console.log('API_KEY=sk-test-secret-123456'); console.log('Authorization: Bearer abcdefghijklmnop')",
        ],
        None,
        30,
        4096,
    )
    .expect("save spec");

    let run = run_approved(&database, &task.id, &criterion_id);
    let check = VerificationService::new(&database)
        .get_check(&run.id)
        .expect("check")
        .expect("check exists");
    let evidence = VerificationService::new(&database)
        .get_evidence(&check.evidence_id.expect("evidence id"))
        .expect("evidence");
    let output = evidence.redacted_output.expect("redacted output");
    assert!(!output.contains("sk-test-secret-123456"), "{output}");
    assert!(!output.contains("abcdefghijklmnop"), "{output}");
    assert!(output.contains("[REDACTED]"), "{output}");
    assert!(!evidence.sensitive_flags.is_empty());
}

#[test]
fn code_change_during_run_is_inconclusive() {
    let (_repo_directory, repo) = init_repo();
    let (_database_directory, database) = temp_database();
    let (task, criterion_id) = create_auto_task(&database, &repo);
    save_spec(
        &database,
        &task.id,
        &criterion_id,
        "git",
        &["--version"],
        None,
        30,
        4096,
    )
    .expect("save spec");

    let prepared = prepare_run(&database, &task.id, &criterion_id, true).expect("prepare");
    fs::write(repo.join("README.md"), "changed during run\n").expect("change file");
    let run = execute_prepared(&database, prepared);

    assert_eq!(run.execution_status, "completed");
    assert_eq!(
        detail_with_status(&database, &task.id),
        VerificationResult::Inconclusive
    );
    assert!(transition(&database, &task.id, "done", None).is_err());
}

#[test]
fn task_revision_change_makes_evidence_stale() {
    let (_repo_directory, repo) = init_repo();
    let (_database_directory, database) = temp_database();
    let (task, criterion_id) = create_auto_task(&database, &repo);
    save_spec(
        &database,
        &task.id,
        &criterion_id,
        "git",
        &["--version"],
        None,
        30,
        4096,
    )
    .expect("save spec");
    run_approved(&database, &task.id, &criterion_id);

    let detail = current_detail(&database, &task.id);
    TaskService::new(&database)
        .update_task(UpdateTaskInput {
            task_id: task.id.clone(),
            expected_revision: detail.task.revision,
            title: "修改后的任务标题".to_owned(),
            description: Some("修改后".to_owned()),
            priority: "high".to_owned(),
        })
        .expect("update task");

    assert_eq!(
        detail_with_status(&database, &task.id),
        VerificationResult::Stale
    );
    assert!(transition(&database, &task.id, "done", None).is_err());
}

#[test]
fn criterion_revision_change_makes_evidence_stale() {
    let (_repo_directory, repo) = init_repo();
    let (_database_directory, database) = temp_database();
    let (task, criterion_id) = create_auto_task(&database, &repo);
    save_spec(
        &database,
        &task.id,
        &criterion_id,
        "git",
        &["--version"],
        None,
        30,
        4096,
    )
    .expect("save spec");
    run_approved(&database, &task.id, &criterion_id);

    let detail = current_detail(&database, &task.id);
    let criterion = criterion_view(&database, &task.id, &criterion_id);
    TaskService::new(&database)
        .update_criterion(UpdateCriterionInput {
            criterion_id: criterion_id.clone(),
            expected_task_revision: detail.task.revision,
            expected_criterion_revision: criterion.revision,
            description: "修改后的自动验收条件".to_owned(),
            kind: "auto".to_owned(),
            required: true,
        })
        .expect("update criterion");

    assert_eq!(
        detail_with_status(&database, &task.id),
        VerificationResult::Stale
    );
}

#[test]
fn spec_revision_change_makes_evidence_stale() {
    let (_repo_directory, repo) = init_repo();
    let (_database_directory, database) = temp_database();
    let (task, criterion_id) = create_auto_task(&database, &repo);
    save_spec(
        &database,
        &task.id,
        &criterion_id,
        "git",
        &["--version"],
        None,
        30,
        4096,
    )
    .expect("save spec");
    run_approved(&database, &task.id, &criterion_id);

    save_spec(
        &database,
        &task.id,
        &criterion_id,
        "git",
        &["--version"],
        None,
        60,
        4096,
    )
    .expect("update spec");

    assert_eq!(
        detail_with_status(&database, &task.id),
        VerificationResult::Stale
    );
}

#[test]
fn working_tree_change_makes_evidence_stale() {
    let (_repo_directory, repo) = init_repo();
    let (_database_directory, database) = temp_database();
    let (task, criterion_id) = create_auto_task(&database, &repo);
    save_spec(
        &database,
        &task.id,
        &criterion_id,
        "git",
        &["--version"],
        None,
        30,
        4096,
    )
    .expect("save spec");
    run_approved(&database, &task.id, &criterion_id);

    fs::write(repo.join("README.md"), "changed after run\n").expect("change file");
    assert_eq!(
        detail_with_status(&database, &task.id),
        VerificationResult::Stale
    );
}

#[test]
fn git_head_change_makes_evidence_stale() {
    let (_repo_directory, repo) = init_repo();
    let (_database_directory, database) = temp_database();
    let (task, criterion_id) = create_auto_task(&database, &repo);
    save_spec(
        &database,
        &task.id,
        &criterion_id,
        "git",
        &["--version"],
        None,
        30,
        4096,
    )
    .expect("save spec");
    run_approved(&database, &task.id, &criterion_id);

    fs::write(repo.join("README.md"), "committed change\n").expect("change file");
    run_git(&repo, &["add", "-A"]);
    run_git(&repo, &["commit", "-m", "change after run"]);

    assert_eq!(
        detail_with_status(&database, &task.id),
        VerificationResult::Stale
    );
}

#[test]
fn mixed_manual_and_auto_requires_both() {
    let (_repo_directory, repo) = init_repo();
    let (_database_directory, database) = temp_database();
    let project = import_project(&database, &repo);
    let task = create_task(&database, &project.id);
    let auto = add_criterion(&database, &task.id, "auto", true, "自动条件");
    add_criterion(&database, &task.id, "manual", true, "人工条件");
    advance_to_awaiting(&database, &task.id);
    save_spec(
        &database,
        &task.id,
        &auto.id,
        "git",
        &["--version"],
        None,
        30,
        4096,
    )
    .expect("save spec");
    run_approved(&database, &task.id, &auto.id);

    let before = current_detail(&database, &task.id);
    assert!(!before.completion.can_complete);
    assert_eq!(
        before
            .criteria
            .iter()
            .find(|view| view.criterion.kind == CriterionKind::Auto)
            .and_then(|view| view.verification.as_ref())
            .map(|view| view.status),
        Some(VerificationResult::Pass)
    );

    let detail = current_detail(&database, &task.id);
    let manual = detail
        .criteria
        .iter()
        .find(|view| view.criterion.kind == CriterionKind::Manual)
        .expect("manual");
    TaskService::new(&database)
        .confirm_manual_criterion(xushu_core::ConfirmManualInput {
            criterion_id: manual.criterion.id.clone(),
            expected_task_revision: detail.task.revision,
            expected_criterion_revision: manual.criterion.revision,
            note: Some("人工确认".to_owned()),
        })
        .expect("confirm");

    assert!(current_detail(&database, &task.id).completion.can_complete);
    transition_ok(&database, &task.id, "done");
}

#[test]
fn auto_fail_with_manual_confirmation_still_blocks_done() {
    let (_repo_directory, repo) = init_repo();
    let (_database_directory, database) = temp_database();
    let project = import_project(&database, &repo);
    let task = create_task(&database, &project.id);
    let auto = add_criterion(&database, &task.id, "auto", true, "自动条件");
    add_criterion(&database, &task.id, "manual", true, "人工条件");
    advance_to_awaiting(&database, &task.id);
    save_spec(
        &database,
        &task.id,
        &auto.id,
        "git",
        &["rev-parse", "--verify", "refs/heads/missing"],
        None,
        30,
        4096,
    )
    .expect("save spec");
    run_approved(&database, &task.id, &auto.id);

    let detail = current_detail(&database, &task.id);
    let manual = detail
        .criteria
        .iter()
        .find(|view| view.criterion.kind == CriterionKind::Manual)
        .expect("manual");
    TaskService::new(&database)
        .confirm_manual_criterion(xushu_core::ConfirmManualInput {
            criterion_id: manual.criterion.id.clone(),
            expected_task_revision: detail.task.revision,
            expected_criterion_revision: manual.criterion.revision,
            note: None,
        })
        .expect("confirm");

    assert!(!current_detail(&database, &task.id).completion.can_complete);
    assert!(transition(&database, &task.id, "done", None).is_err());
}

#[test]
fn not_run_without_evidence_blocks_done() {
    let (_repo_directory, repo) = init_repo();
    let (_database_directory, database) = temp_database();
    let project = import_project(&database, &repo);
    let task = create_task(&database, &project.id);
    add_criterion(&database, &task.id, "auto", true, "自动条件");
    advance_to_awaiting(&database, &task.id);

    let detail = current_detail(&database, &task.id);
    let view = detail.criteria[0].verification.as_ref().expect("view");
    assert_eq!(view.status, VerificationResult::NotRun);
    assert!(!detail.completion.can_complete);
    assert!(detail
        .completion
        .blockers
        .iter()
        .any(|blocker| blocker.contains("尚未配置验证")));

    assert!(transition(&database, &task.id, "done", None).is_err());
}

#[test]
fn run_history_and_evidence_survive_database_reopen() {
    let directory = TempDir::new().expect("temp directory");
    let database_path = directory.path().join("xushu.db");
    let (_repo_directory, repo) = init_repo();
    let (task_id, criterion_id, run_id) = {
        let database = Database::open(&database_path).expect("open database");
        let (task, criterion_id) = create_auto_task(&database, &repo);
        save_spec(
            &database,
            &task.id,
            &criterion_id,
            "git",
            &["--version"],
            None,
            30,
            4096,
        )
        .expect("save spec");
        let run = run_approved(&database, &task.id, &criterion_id);
        (task.id, criterion_id, run.id)
    };

    let reopened = Database::open(&database_path).expect("reopen database");
    let run = VerificationService::new(&reopened)
        .get_run(&run_id)
        .expect("run after reopen");
    assert_eq!(run.execution_status, "completed");
    let runs = VerificationService::new(&reopened)
        .list_runs(&task_id)
        .expect("runs");
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].result, Some(VerificationResult::Pass));
    assert_eq!(runs[0].executable, "git");
    assert_eq!(runs[0].args, vec!["--version".to_owned()]);
    assert!(!runs[0].cwd.is_empty());
    assert!(VerificationService::new(&reopened)
        .get_spec(&criterion_id)
        .expect("spec")
        .is_some());
}

#[test]
fn interrupted_run_is_recovered_as_inconclusive() {
    let directory = TempDir::new().expect("temp directory");
    let database_path = directory.path().join("xushu.db");
    let (_repo_directory, repo) = init_repo();

    let run_id = {
        let database = Database::open(&database_path).expect("open database");
        let (task, criterion_id) = create_auto_task(&database, &repo);
        save_spec(
            &database,
            &task.id,
            &criterion_id,
            "git",
            &["--version"],
            None,
            30,
            4096,
        )
        .expect("save spec");
        let prepared = prepare_run(&database, &task.id, &criterion_id, true).expect("prepare");
        prepared.run.id
    };

    let reopened = Database::open(&database_path).expect("reopen database");
    let recovered = VerificationService::new(&reopened)
        .recover_interrupted()
        .expect("recover");
    assert_eq!(recovered, 1);
    let run = VerificationService::new(&reopened)
        .get_run(&run_id)
        .expect("run");
    assert_eq!(run.execution_status, "interrupted");
    let check = VerificationService::new(&reopened)
        .get_check(&run_id)
        .expect("check")
        .expect("check exists");
    assert_eq!(check.result, VerificationResult::Inconclusive);
}

#[test]
fn report_export_contains_real_results_and_is_not_a_pass_guarantee() {
    let (_repo_directory, repo) = init_repo();
    let (_database_directory, database) = temp_database();
    let export_directory = TempDir::new().expect("export directory");
    let (task, criterion_id) = create_auto_task(&database, &repo);
    save_spec(
        &database,
        &task.id,
        &criterion_id,
        "git",
        &["--version"],
        None,
        30,
        4096,
    )
    .expect("save spec");
    run_approved(&database, &task.id, &criterion_id);

    let report = VerificationService::new(&database)
        .export_report(&task.id, export_directory.path())
        .expect("export");
    assert!(export_directory.path().join(&report.file_name).is_file());
    assert!(report.markdown.contains("验收报告"));
    assert!(report.markdown.contains(&task.title));
    assert!(report.markdown.contains("pass"));
    assert!(report.markdown.contains("git"));
    assert!(report.markdown.contains("--version"));
    assert!(report.markdown.contains("工作目录"));
    assert!(report.markdown.contains("不代表代码绝对正确"));
    assert!(report.markdown.contains("分享前请检查"));
}

#[test]
fn verification_queue_reports_real_pending_state() {
    let (_repo_directory, repo) = init_repo();
    let (_database_directory, database) = temp_database();
    let (task, _criterion_id) = create_auto_task(&database, &repo);

    let queue = TaskService::new(&database)
        .verification_queue()
        .expect("queue");
    let item = queue
        .iter()
        .find(|item| item.task.id == task.id)
        .expect("queue item");
    assert_eq!(item.auto_total, 1);
    assert_eq!(item.auto_satisfied, 0);
    assert_eq!(item.running_count, 0);
    assert!(!item.completion.can_complete);
}

#[test]
fn spec_save_uses_optimistic_revision() {
    let (_repo_directory, repo) = init_repo();
    let (_database_directory, database) = temp_database();
    let (task, criterion_id) = create_auto_task(&database, &repo);
    save_spec(
        &database,
        &task.id,
        &criterion_id,
        "git",
        &["--version"],
        None,
        30,
        4096,
    )
    .expect("save spec");

    let detail = current_detail(&database, &task.id);
    let criterion = criterion_view(&database, &task.id, &criterion_id);
    let conflict = VerificationService::new(&database).save_spec(SaveVerificationSpecInput {
        criterion_id,
        expected_task_revision: detail.task.revision,
        expected_criterion_revision: criterion.revision,
        expected_spec_revision: None,
        executable: "git".to_owned(),
        args: vec!["--version".to_owned()],
        cwd: None,
        timeout_seconds: 60,
        output_limit_bytes: 4096,
    });
    assert!(matches!(conflict, Err(CoreError::Conflict(_))));
}
