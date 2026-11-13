use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use tempfile::TempDir;
use xushu_core::{
    code_state, runner, ArchiveMemoryInput, ConfirmDecisionInput, CreateCriterionInput,
    CreateDecisionInput, CreateMemoryInput, CreateTaskInput, Database, DecisionFilter,
    DecisionService, DecisionStatus, ImportProjectInput, MemoryFilter, MemoryService, Project,
    ProjectService, RecoveryFreshnessState, RecoveryGitState, RecoveryNextStepKind,
    RecoveryService, RunControl, SaveVerificationSpecInput, SetNextTaskInput,
    StartVerificationInput, Task, TaskDetail, TaskService, TaskStatus, TransitionTaskInput,
    UpdateTaskInput, VerificationService,
};

fn open_db() -> (TempDir, Database) {
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
    fs::write(path.join("CLAUDE.md"), "# 项目约定\n").expect("write claude md");
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

fn create_task(database: &Database, project_id: &str, title: &str) -> Task {
    TaskService::new(database)
        .create_task(CreateTaskInput {
            project_id: project_id.to_owned(),
            title: title.to_owned(),
            description: None,
            priority: "medium".to_owned(),
        })
        .expect("create task")
}

fn current_detail(database: &Database, task_id: &str) -> TaskDetail {
    TaskService::new(database)
        .get_task(task_id)
        .expect("task detail")
}

fn transition(
    database: &Database,
    task_id: &str,
    to: &str,
    reason: Option<&str>,
) -> Result<Task, xushu_core::CoreError> {
    let detail = current_detail(database, task_id);
    TaskService::new(database).transition_task_status(TransitionTaskInput {
        task_id: task_id.to_owned(),
        expected_revision: detail.task.revision,
        to_status: to.to_owned(),
        reason: reason.map(str::to_owned),
    })
}

fn create_memory(
    database: &Database,
    project_id: &str,
    category: &str,
    title: &str,
    content: &str,
    confirmed: bool,
    source_type: Option<&str>,
) -> xushu_core::ProjectMemoryView {
    MemoryService::new(database)
        .create(CreateMemoryInput {
            project_id: project_id.to_owned(),
            category: category.to_owned(),
            title: title.to_owned(),
            content: content.to_owned(),
            source_type: source_type.map(str::to_owned),
            confirmed: Some(confirmed),
            linked_task_id: None,
            linked_evidence_id: None,
        })
        .expect("create memory")
}

fn create_decision(database: &Database, project_id: &str, title: &str) -> xushu_core::DecisionView {
    DecisionService::new(database)
        .create(CreateDecisionInput {
            project_id: project_id.to_owned(),
            task_id: None,
            title: title.to_owned(),
            problem: "需要选择方案".to_owned(),
            context: "本地优先".to_owned(),
            alternatives: vec!["方案 A".to_owned(), "方案 B".to_owned()],
            decision: "采用方案 A".to_owned(),
            rationale: "风险更低".to_owned(),
            risks: "需要维护".to_owned(),
            supersedes_id: None,
        })
        .expect("create decision")
}

fn confirm_decision(
    database: &Database,
    project_id: &str,
    decision: &xushu_core::DecisionView,
) -> xushu_core::DecisionView {
    DecisionService::new(database)
        .confirm(ConfirmDecisionInput {
            project_id: project_id.to_owned(),
            decision_id: decision.decision.id.clone(),
            expected_revision: decision.decision.revision,
        })
        .expect("confirm decision")
}

fn add_criterion(
    database: &Database,
    task_id: &str,
    kind: &str,
    required: bool,
    description: &str,
) -> xushu_core::AcceptanceCriterion {
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

fn save_spec(
    database: &Database,
    task_id: &str,
    criterion_id: &str,
    executable: &str,
    args: &[&str],
) -> xushu_core::VerificationSpec {
    let detail = current_detail(database, task_id);
    let criterion = detail
        .criteria
        .iter()
        .find(|view| view.criterion.id == criterion_id)
        .expect("criterion");
    let existing = VerificationService::new(database)
        .get_spec(criterion_id)
        .expect("existing spec");
    VerificationService::new(database)
        .save_spec(SaveVerificationSpecInput {
            criterion_id: criterion_id.to_owned(),
            expected_task_revision: detail.task.revision,
            expected_criterion_revision: criterion.criterion.revision,
            expected_spec_revision: existing.map(|spec| spec.revision),
            executable: executable.to_owned(),
            args: args.iter().map(|arg| (*arg).to_owned()).collect(),
            cwd: None,
            timeout_seconds: 30,
            output_limit_bytes: 4096,
        })
        .expect("save spec")
}

fn run_approved(database: &Database, task_id: &str, criterion_id: &str) {
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
    let prepared = VerificationService::new(database)
        .prepare_run(StartVerificationInput {
            criterion_id: criterion_id.to_owned(),
            expected_task_revision: detail.task.revision,
            expected_criterion_revision: criterion.criterion.revision,
            expected_spec_revision: spec.revision,
            approved: true,
        })
        .expect("prepare run");
    let control = RunControl::new();
    let outcome = match prepared.preflight_error.clone() {
        Some(message) => runner::RunOutcome::start_failed(message),
        None => runner::execute(&prepared.plan, &control),
    };
    let end_code_state =
        code_state::capture(&prepared.project_root).map_err(|error| error.code().to_owned());
    VerificationService::new(database)
        .finalize_run(&prepared.run.id, outcome, end_code_state)
        .expect("finalize");
}

#[test]
fn generates_complete_package_and_persists_snapshot() {
    let (_repo_dir, repo_path) = init_repo();
    let (_db_dir, database) = open_db();
    let project = import_project(&database, &repo_path);
    let task = create_task(&database, &project.id, "等待验收的任务");
    add_criterion(&database, &task.id, "manual", true, "人工确认目标");
    transition(&database, &task.id, "ready", None).expect("ready");
    transition(&database, &task.id, "running", None).expect("running");
    transition(&database, &task.id, "awaiting_verification", None).expect("awaiting");

    create_memory(
        &database,
        &project.id,
        "constraint",
        "必须由用户授权提交",
        "任何 Git 写入都必须由用户确认。",
        true,
        None,
    );
    create_memory(
        &database,
        &project.id,
        "fact",
        "未确认推断",
        "这只是推断，不应作为权威上下文。",
        false,
        Some("inference"),
    );
    let old = create_decision(&database, &project.id, "旧方案");
    let old = confirm_decision(&database, &project.id, &old);
    let mut new_input = CreateDecisionInput {
        project_id: project.id.clone(),
        task_id: None,
        title: "新方案".to_owned(),
        problem: "旧方案存在风险".to_owned(),
        context: "本地优先".to_owned(),
        alternatives: vec!["新方案".to_owned()],
        decision: "采用新方案".to_owned(),
        rationale: "更符合约束".to_owned(),
        risks: "迁移成本".to_owned(),
        supersedes_id: Some(old.decision.id.clone()),
    };
    let new = DecisionService::new(&database)
        .create(new_input.clone())
        .expect("new decision");
    new_input.title = "新方案（已确认）".to_owned();
    let new = confirm_decision(&database, &project.id, &new);
    assert_eq!(new.decision.status, xushu_core::DecisionStatus::Confirmed);

    let service = RecoveryService::new(&database);
    let snapshot = service.generate(&project.id).expect("generate recovery");

    assert_eq!(snapshot.project_id, project.id);
    assert!(snapshot.fingerprint.starts_with("sha256:"));
    assert_eq!(snapshot.package.project.name, "sample-repo");
    assert_eq!(snapshot.package.git.state, RecoveryGitState::Available);
    assert_eq!(snapshot.package.git.branch.as_deref(), Some("main"));
    assert!(snapshot.package.git.head.is_some());
    assert!(!snapshot.package.git.dirty);
    assert!(snapshot.package.git.workspace_fingerprint.is_some());
    assert_eq!(snapshot.package.tasks.awaiting_verification.len(), 1);
    assert_eq!(
        snapshot.package.tasks.awaiting_verification[0].title,
        "等待验收的任务"
    );
    assert_eq!(
        snapshot.package.next_step.kind,
        RecoveryNextStepKind::Suggested
    );
    assert!(snapshot.package.next_step.requires_confirmation);

    assert_eq!(snapshot.package.memories.len(), 1);
    assert_eq!(snapshot.package.memories[0].title, "必须由用户授权提交");
    assert_eq!(snapshot.package.unconfirmed_memories.len(), 1);
    assert_eq!(snapshot.package.unconfirmed_memories[0].title, "未确认推断");
    assert_eq!(snapshot.package.decisions.len(), 1);
    assert_eq!(snapshot.package.decisions[0].title, "新方案");
    assert_eq!(snapshot.package.superseded_decisions.len(), 1);
    assert_eq!(snapshot.package.superseded_decisions[0].title, "旧方案");
    assert!(snapshot
        .package
        .files_to_read
        .iter()
        .any(|file| file == "README.md"));
    assert!(snapshot
        .package
        .files_to_read
        .iter()
        .any(|file| file == "CLAUDE.md"));
    assert!(snapshot
        .package
        .missing_information
        .iter()
        .any(|item| item.contains("尚无独立验证")));
    assert!(snapshot
        .package
        .missing_information
        .iter()
        .any(|item| item.contains("尚未确认")));
    assert!(snapshot.markdown.contains("XUSHU 恢复上下文"));
    assert!(snapshot.markdown.contains(&snapshot.fingerprint));
    assert!(snapshot.markdown.contains("生成时间"));
    assert!(snapshot.markdown.contains("必须由用户授权提交"));
    assert!(snapshot.markdown.contains("不是事实"));
    assert!(!snapshot.markdown.contains("当前测试通过"));
    assert!(!snapshot.markdown.contains("AI 已完成"));

    let overview = service.overview(&project.id).expect("overview");
    let latest = overview.latest.expect("latest snapshot");
    assert_eq!(latest.id, snapshot.id);
    assert_eq!(overview.freshness.state, RecoveryFreshnessState::Fresh);
    assert!(overview.freshness.changed.is_empty());

    let summaries = service.list_snapshots(&project.id).expect("list snapshots");
    assert_eq!(summaries.len(), 1);
    assert_eq!(summaries[0].id, snapshot.id);

    let loaded = service
        .get_snapshot(&project.id, &snapshot.id)
        .expect("load snapshot");
    assert_eq!(loaded.package.fingerprint, snapshot.package.fingerprint);
    assert_eq!(loaded.markdown, snapshot.markdown);
}

#[test]
fn detects_stale_snapshot_after_git_or_task_changes() {
    let (_repo_dir, repo_path) = init_repo();
    let (_db_dir, database) = open_db();
    let project = import_project(&database, &repo_path);
    let task = create_task(&database, &project.id, "状态变化任务");
    add_criterion(&database, &task.id, "manual", true, "人工确认");
    transition(&database, &task.id, "ready", None).expect("ready");

    let service = RecoveryService::new(&database);
    let first = service.generate(&project.id).expect("first snapshot");
    fs::write(repo_path.join("README.md"), "changed\n").expect("modify readme");

    let overview = service.overview(&project.id).expect("overview");
    assert_eq!(overview.freshness.state, RecoveryFreshnessState::Stale);
    assert!(overview
        .freshness
        .changed
        .iter()
        .any(|item| item.contains("Git")));

    let second = service.generate(&project.id).expect("second snapshot");
    assert_ne!(second.id, first.id);
    assert_ne!(second.fingerprint, first.fingerprint);
    let overview = service
        .overview(&project.id)
        .expect("overview after regenerate");
    assert_eq!(overview.freshness.state, RecoveryFreshnessState::Fresh);
    assert_eq!(
        overview
            .latest
            .as_ref()
            .map(|snapshot| snapshot.id.as_str()),
        Some(second.id.as_str())
    );
    assert_eq!(service.list_snapshots(&project.id).expect("list").len(), 2);

    transition(&database, &task.id, "running", None).expect("running");
    let overview = service
        .overview(&project.id)
        .expect("overview after task change");
    assert_eq!(overview.freshness.state, RecoveryFreshnessState::Stale);
    assert!(overview
        .freshness
        .changed
        .iter()
        .any(|item| item.contains("任务")));
}

#[test]
fn handles_git_failure_as_unknown_and_reports_missing_information() {
    let (repo_dir, repo_path) = init_repo();
    let (_db_dir, database) = open_db();
    let project = import_project(&database, &repo_path);

    // Git for Windows marks loose objects read-only. Clear that attribute
    // on regular files in this disposable repository before removing it.
    #[cfg(windows)]
    #[expect(
        clippy::permissions_set_readonly_false,
        reason = "this Windows-only fixture clears a Windows file attribute"
    )]
    for directory in fs::read_dir(repo_path.join(".git/objects")).expect("list fixture objects") {
        let directory = directory.expect("read fixture object directory");
        if !directory
            .file_type()
            .expect("fixture object directory type")
            .is_dir()
        {
            continue;
        }
        for entry in fs::read_dir(directory.path()).expect("list fixture object files") {
            let entry = entry.expect("read fixture object file");
            if !entry
                .file_type()
                .expect("fixture object file type")
                .is_file()
            {
                continue;
            }
            let mut permissions = entry
                .metadata()
                .expect("fixture object metadata")
                .permissions();
            if permissions.readonly() {
                permissions.set_readonly(false);
                fs::set_permissions(entry.path(), permissions)
                    .expect("make fixture object writable");
            }
        }
    }
    fs::remove_dir_all(&repo_path).expect("remove repo");
    repo_dir.close().expect("close repository fixture");

    let snapshot = RecoveryService::new(&database)
        .generate(&project.id)
        .expect("recovery with git failure");
    assert_eq!(snapshot.package.git.state, RecoveryGitState::Unknown);
    assert!(snapshot.package.git.note.is_some());
    assert!(snapshot
        .package
        .missing_information
        .iter()
        .any(|item| item.contains("Git")));
    assert!(snapshot.package.files_to_read.is_empty());
    assert!(snapshot.markdown.contains("UNKNOWN"));
}

#[test]
fn done_task_with_stale_evidence_is_reported_as_conflict() {
    let (_repo_dir, repo_path) = init_repo();
    let (_db_dir, database) = open_db();
    let project = import_project(&database, &repo_path);
    let task = create_task(&database, &project.id, "已完成但证据会过期");
    let criterion = add_criterion(&database, &task.id, "auto", true, "自动验证通过");
    transition(&database, &task.id, "ready", None).expect("ready");
    transition(&database, &task.id, "running", None).expect("running");
    transition(&database, &task.id, "awaiting_verification", None).expect("awaiting");
    save_spec(&database, &task.id, &criterion.id, "git", &["--version"]);
    run_approved(&database, &task.id, &criterion.id);
    transition(&database, &task.id, "done", None).expect("done");

    let fresh = RecoveryService::new(&database)
        .generate(&project.id)
        .expect("fresh recovery");
    assert_eq!(fresh.package.tasks.done_with_valid_evidence.len(), 1);
    assert!(fresh.package.tasks.done_with_invalid_evidence.is_empty());

    fs::write(repo_path.join("README.md"), "after verification\n").expect("modify readme");
    let stale = RecoveryService::new(&database)
        .generate(&project.id)
        .expect("stale recovery");
    assert!(stale.package.tasks.done_with_valid_evidence.is_empty());
    assert_eq!(stale.package.tasks.done_with_invalid_evidence.len(), 1);
    assert!(stale
        .package
        .conflicts
        .iter()
        .any(|conflict| conflict.kind == "done_evidence_invalid"));
    assert!(stale.markdown.contains("已过期"));
    assert!(!stale.markdown.contains("当前测试通过"));
}

#[test]
fn no_task_project_needs_human_confirmation_and_has_explicit_gaps() {
    let (_repo_dir, repo_path) = init_repo();
    let (_db_dir, database) = open_db();
    let project = import_project(&database, &repo_path);

    let snapshot = RecoveryService::new(&database)
        .generate(&project.id)
        .expect("empty project recovery");
    assert_eq!(
        snapshot.package.next_step.kind,
        RecoveryNextStepKind::NeedsConfirmation
    );
    assert!(snapshot.package.next_step.requires_confirmation);
    assert!(snapshot.package.tasks.in_progress.is_empty());
    assert!(snapshot.package.tasks.ready.is_empty());
    assert!(snapshot.package.tasks.blocked.is_empty());
    assert!(snapshot.package.tasks.awaiting_verification.is_empty());
    assert!(snapshot
        .package
        .missing_information
        .iter()
        .any(|item| item.contains("尚无任务")));
    assert!(snapshot
        .package
        .missing_information
        .iter()
        .any(|item| item.contains("尚无独立验证")));
    assert!(snapshot
        .package
        .missing_information
        .iter()
        .any(|item| item.contains("Claude Code")));
    assert!(snapshot
        .package
        .sources
        .exclusions
        .iter()
        .any(|item| item.contains("未导入")));
    assert!(snapshot.markdown.contains("需要人工确认"));
}

#[test]
fn next_task_preference_is_user_selected_and_validated() {
    let (_repo_a, repo_a) = init_repo();
    let (_repo_b, repo_b) = init_repo();
    let (_db_dir, database) = open_db();
    let project_a = import_project(&database, &repo_a);
    let project_b = import_project(&database, &repo_b);
    let task = create_task(&database, &project_a.id, "明确下一步");
    let other_task = create_task(&database, &project_b.id, "其他项目任务");
    let cancelled = create_task(&database, &project_a.id, "已取消任务");
    transition(&database, &cancelled.id, "cancelled", Some("不再继续")).expect("cancel task");
    let service = RecoveryService::new(&database);

    let preference = service
        .set_next_task(SetNextTaskInput {
            project_id: project_a.id.clone(),
            task_id: Some(task.id.clone()),
        })
        .expect("set next task");
    assert_eq!(preference.next_task_id.as_deref(), Some(task.id.as_str()));

    let snapshot = service.generate(&project_a.id).expect("recovery");
    assert_eq!(
        snapshot.package.next_step.kind,
        RecoveryNextStepKind::UserSelected
    );
    assert_eq!(
        snapshot.package.next_step.task_id.as_deref(),
        Some(task.id.as_str())
    );
    assert!(!snapshot.package.next_step.requires_confirmation);

    let cross_project = service.set_next_task(SetNextTaskInput {
        project_id: project_a.id.clone(),
        task_id: Some(other_task.id.clone()),
    });
    assert!(matches!(
        cross_project,
        Err(xushu_core::CoreError::Validation(_))
    ));

    let cancelled_task = service.set_next_task(SetNextTaskInput {
        project_id: project_a.id.clone(),
        task_id: Some(cancelled.id.clone()),
    });
    assert!(matches!(
        cancelled_task,
        Err(xushu_core::CoreError::Validation(_))
    ));

    service
        .set_next_task(SetNextTaskInput {
            project_id: project_a.id.clone(),
            task_id: None,
        })
        .expect("clear preference");
    let snapshot = service.generate(&project_a.id).expect("recovery");
    assert_eq!(
        snapshot.package.next_step.kind,
        RecoveryNextStepKind::NeedsConfirmation
    );

    let _ = project_b;
}

#[test]
fn editing_the_selected_draft_invalidates_the_snapshot() {
    let (_repo, repo) = init_repo();
    let (_db_dir, database) = open_db();
    let project = import_project(&database, &repo);
    let task = create_task(&database, &project.id, "下一步草稿");
    let service = RecoveryService::new(&database);
    service
        .set_next_task(SetNextTaskInput {
            project_id: project.id.clone(),
            task_id: Some(task.id.clone()),
        })
        .expect("select draft task");
    let snapshot = service.generate(&project.id).expect("generate");
    assert_eq!(
        service
            .overview(&project.id)
            .expect("overview")
            .freshness
            .state,
        RecoveryFreshnessState::Fresh
    );

    TaskService::new(&database)
        .update_task(UpdateTaskInput {
            task_id: task.id,
            expected_revision: task.revision,
            title: task.title,
            description: Some("更新下一步任务要求".to_owned()),
            priority: task.priority.to_string(),
        })
        .expect("edit draft task");
    let overview = service.overview(&project.id).expect("check updated draft");
    assert_eq!(overview.freshness.state, RecoveryFreshnessState::Stale);
    assert!(overview
        .freshness
        .changed
        .iter()
        .any(|change| change.contains("下一步任务")));
    assert_eq!(overview.latest.expect("snapshot").id, snapshot.id);
    service.generate(&project.id).expect("regenerate");
    assert_eq!(
        service
            .overview(&project.id)
            .expect("overview")
            .freshness
            .state,
        RecoveryFreshnessState::Fresh
    );
}

#[test]
fn snapshots_are_project_isolated_and_retained_with_a_limit() {
    let (_repo_a, repo_a) = init_repo();
    let (_repo_b, repo_b) = init_repo();
    let (_db_dir, database) = open_db();
    let project_a = import_project(&database, &repo_a);
    let project_b = import_project(&database, &repo_b);
    let service = RecoveryService::new(&database);

    let mut ids = Vec::new();
    for _ in 0..21 {
        ids.push(service.generate(&project_a.id).expect("generate").id);
    }

    let cross = service.get_snapshot(&project_b.id, &ids[20]);
    assert!(matches!(cross, Err(xushu_core::CoreError::NotFound)));
    assert!(service
        .overview(&project_b.id)
        .expect("overview B")
        .latest
        .is_none());

    let summaries = service
        .list_snapshots(&project_a.id)
        .expect("list snapshots");
    assert_eq!(summaries.len(), 20);
    assert!(summaries.iter().any(|summary| summary.id == ids[20]));
    assert!(!summaries.iter().any(|summary| summary.id == ids[0]));
}

#[test]
fn recovery_markdown_uses_only_pseudonymous_claude_metadata() {
    let (_repo_dir, repo_path) = init_repo();
    let (_db_dir, database) = open_db();
    let project = import_project(&database, &repo_path);
    let session_hash = "abcdef123456SECRETFULLSESSIONHASH";
    let now = "2026-01-01T00:00:00.000Z";
    database
        .with_transaction(|transaction| {
            transaction.execute(
                "INSERT INTO claude_sessions (
                    id, project_id, session_hash, started_at, last_event_at, ended_at,
                    start_source, end_reason, start_observed, created_at, updated_at
                 ) VALUES (
                    'session-1', ?1, ?2, ?3, ?3, NULL, 'startup', NULL, 1, ?3, ?3
                 )",
                rusqlite::params![project.id, session_hash, now],
            )?;
            transaction.execute(
                "INSERT INTO claude_events (
                    id, delivery_id, dedupe_key, project_id, session_id, tool_call_id,
                    event_type, occurred_at, ingested_at, prompt_hash, tool_name,
                    tool_category, failure_kind, is_interrupt, duration_ms, source,
                    end_reason, created_at
                 ) VALUES (
                    'event-1', 'delivery-1', NULL, ?1, 'session-1', NULL,
                    'session_start', ?2, ?2, 'PROMPTHASHSECRET', NULL,
                    NULL, NULL, NULL, NULL, 'startup', NULL, ?2
                 )",
                rusqlite::params![project.id, now],
            )?;
            Ok(())
        })
        .expect("insert claude metadata");

    let snapshot = RecoveryService::new(&database)
        .generate(&project.id)
        .expect("recovery");
    assert_eq!(snapshot.package.claude.event_count, 1);
    assert_eq!(snapshot.package.claude.session_count, 1);
    assert!(!snapshot.package.claude.end_confirmed);
    assert!(snapshot.package.claude.note.is_some());
    assert!(snapshot
        .package
        .missing_information
        .iter()
        .any(|item| item.contains("结束未确认")));
    assert!(snapshot.markdown.contains("abcdef123456"));
    assert!(!snapshot.markdown.contains("SECRETFULLSESSIONHASH"));
    assert!(!snapshot.markdown.contains("PROMPTHASHSECRET"));
    assert!(!snapshot.markdown.contains("session_hash"));
    assert!(!snapshot.package_json_contains_raw_hook_payload());
}

#[test]
fn long_memory_content_is_bounded_in_recovery_package() {
    let (_repo_dir, repo_path) = init_repo();
    let (_db_dir, database) = open_db();
    let project = import_project(&database, &repo_path);
    create_memory(
        &database,
        &project.id,
        "knowledge",
        "长记忆",
        &"a".repeat(5_000),
        true,
        None,
    );

    let snapshot = RecoveryService::new(&database)
        .generate(&project.id)
        .expect("recovery");
    let memory = snapshot
        .package
        .memories
        .iter()
        .find(|memory| memory.title == "长记忆")
        .expect("long memory");
    assert_eq!(memory.content.chars().count(), 4_000);
    assert!(memory.content.ends_with('…'));
    assert!(snapshot
        .package
        .sources
        .included_limits
        .iter()
        .any(|limit| limit.contains("4000")));
    assert!(snapshot.markdown.len() < 20_000);
}

#[test]
fn snapshot_insert_failure_leaves_no_partial_snapshot() {
    let (_repo_dir, repo_path) = init_repo();
    let (_db_dir, database) = open_db();
    let project = import_project(&database, &repo_path);
    database
        .with_transaction(|transaction| {
            transaction.execute_batch(
                "CREATE TRIGGER fail_snapshot
                 BEFORE INSERT ON recovery_snapshots
                 BEGIN
                     SELECT RAISE(ABORT, 'synthetic snapshot failure');
                 END;",
            )?;
            Ok(())
        })
        .expect("create trigger");

    let error = RecoveryService::new(&database)
        .generate(&project.id)
        .expect_err("snapshot insert must fail");
    assert!(matches!(error, xushu_core::CoreError::Database(_)));

    let overview = RecoveryService::new(&database)
        .overview(&project.id)
        .expect("overview");
    assert!(overview.latest.is_none());
    assert!(RecoveryService::new(&database)
        .list_snapshots(&project.id)
        .expect("list")
        .is_empty());
}

#[test]
fn memory_decision_and_recovery_survive_database_reopen() {
    let (_repo_dir, repo_path) = init_repo();
    let directory = TempDir::new().expect("temp directory");
    let database_path = directory.path().join("xushu.db");

    let (project_id, task_id, memory_id, decision_id, snapshot_id) = {
        let database = Database::open(&database_path).expect("open database");
        let project = import_project(&database, &repo_path);
        let task = create_task(&database, &project.id, "重启恢复任务");
        let memory = create_memory(
            &database,
            &project.id,
            "constraint",
            "重启记忆",
            "重启后仍然存在。",
            true,
            None,
        );
        let decision = create_decision(&database, &project.id, "重启决策");
        let decision = confirm_decision(&database, &project.id, &decision);
        RecoveryService::new(&database)
            .set_next_task(SetNextTaskInput {
                project_id: project.id.clone(),
                task_id: Some(task.id.clone()),
            })
            .expect("set next task");
        let snapshot = RecoveryService::new(&database)
            .generate(&project.id)
            .expect("generate");
        (
            project.id,
            task.id,
            memory.memory.id,
            decision.decision.id,
            snapshot.id,
        )
    };

    let database = Database::open(&database_path).expect("reopen database");
    let memories = MemoryService::new(&database)
        .list(MemoryFilter {
            project_id: project_id.clone(),
            category: None,
            include_archived: None,
            task_id: None,
        })
        .expect("list memories");
    assert_eq!(memories.len(), 1);
    assert_eq!(memories[0].memory.id, memory_id);

    let decisions = DecisionService::new(&database)
        .list(DecisionFilter {
            project_id: project_id.clone(),
            status: Some("confirmed".to_owned()),
            task_id: None,
        })
        .expect("list decisions");
    assert_eq!(decisions.len(), 1);
    assert_eq!(decisions[0].decision.id, decision_id);
    assert_eq!(decisions[0].decision.status, DecisionStatus::Confirmed);

    let overview = RecoveryService::new(&database)
        .overview(&project_id)
        .expect("overview after reopen");
    assert_eq!(
        overview
            .latest
            .as_ref()
            .map(|snapshot| snapshot.id.as_str()),
        Some(snapshot_id.as_str())
    );
    assert_eq!(overview.freshness.state, RecoveryFreshnessState::Fresh);

    let detail = TaskService::new(&database)
        .get_task(&task_id)
        .expect("task after reopen");
    assert_eq!(detail.task.status, TaskStatus::Draft);
    assert_eq!(detail.task.revision, 1);
}

#[test]
fn archived_memory_is_excluded_from_recovery_and_counted() {
    let (_repo_dir, repo_path) = init_repo();
    let (_db_dir, database) = open_db();
    let project = import_project(&database, &repo_path);
    let memory = create_memory(
        &database,
        &project.id,
        "risk",
        "已归档风险",
        "这条记忆已经归档。",
        true,
        None,
    );
    MemoryService::new(&database)
        .archive(ArchiveMemoryInput {
            project_id: project.id.clone(),
            memory_id: memory.memory.id.clone(),
            expected_revision: memory.memory.revision,
            reason: Some("不再适用".to_owned()),
        })
        .expect("archive memory");

    let snapshot = RecoveryService::new(&database)
        .generate(&project.id)
        .expect("recovery");
    assert!(snapshot.package.memories.is_empty());
    assert!(snapshot.package.unconfirmed_memories.is_empty());
    assert_eq!(snapshot.package.sources.memory_archived_count, 1);
    assert!(snapshot
        .package
        .missing_information
        .iter()
        .any(|item| item.contains("已归档记忆")));
}

#[test]
fn memory_linked_to_cancelled_task_is_reported_as_conflict() {
    let (_repo_dir, repo_path) = init_repo();
    let (_db_dir, database) = open_db();
    let project = import_project(&database, &repo_path);
    let task = create_task(&database, &project.id, "会被取消的任务");
    MemoryService::new(&database)
        .create(CreateMemoryInput {
            project_id: project.id.clone(),
            category: "fact".to_owned(),
            title: "关联已取消任务的记忆".to_owned(),
            content: "关联任务后来被取消。".to_owned(),
            source_type: None,
            confirmed: Some(true),
            linked_task_id: Some(task.id.clone()),
            linked_evidence_id: None,
        })
        .expect("create linked memory");
    transition(&database, &task.id, "cancelled", Some("不再继续")).expect("cancel task");

    let snapshot = RecoveryService::new(&database)
        .generate(&project.id)
        .expect("recovery");
    assert!(snapshot
        .package
        .conflicts
        .iter()
        .any(|conflict| conflict.kind == "memory_linked_task_inactive"));
    assert!(snapshot.markdown.contains("memory_linked_task_inactive"));
}

trait PackageJsonCheck {
    fn package_json_contains_raw_hook_payload(&self) -> bool;
}

impl PackageJsonCheck for xushu_core::RecoverySnapshot {
    fn package_json_contains_raw_hook_payload(&self) -> bool {
        let json = serde_json::to_string(&self.package).expect("serialize package");
        json.contains("PROMPTHASHSECRET") || json.contains("SECRETFULLSESSIONHASH")
    }
}
