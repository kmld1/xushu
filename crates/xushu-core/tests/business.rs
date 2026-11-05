use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use tempfile::TempDir;
use xushu_core::{
    ArchiveTaskInput, ConfirmManualInput, CoreError, CreateCriterionInput, CreateTaskInput,
    CriterionKind, Database, ImportProjectInput, ManualConfirmation, Project, ProjectService,
    ReopenTaskInput, Task, TaskDetail, TaskFilter, TaskService, TaskStatus, TransitionTaskInput,
    UpdateCriterionInput, UpdateProjectInput, UpdateTaskInput,
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

fn init_empty_repo() -> (TempDir, PathBuf) {
    let directory = TempDir::new().expect("temp directory");
    let path = directory.path().join("empty-repo");
    fs::create_dir_all(&path).expect("create empty repo directory");
    run_git(&path, &["init", "-b", "main"]);
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
            title: "实现本地任务".to_owned(),
            description: Some("验证任务领域服务".to_owned()),
            priority: "medium".to_owned(),
        })
        .expect("create task")
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
) -> Result<Task, CoreError> {
    let detail = current_detail(database, task_id);
    TaskService::new(database).transition_task_status(TransitionTaskInput {
        task_id: task_id.to_owned(),
        expected_revision: detail.task.revision,
        to_status: to.to_owned(),
        reason: reason.map(str::to_owned),
    })
}

fn transition_ok(database: &Database, task_id: &str, to: &str, reason: Option<&str>) -> Task {
    transition(database, task_id, to, reason).expect("transition should succeed")
}

fn advance_to_awaiting(database: &Database, task_id: &str) {
    transition_ok(database, task_id, "ready", None);
    transition_ok(database, task_id, "running", None);
    transition_ok(database, task_id, "awaiting_verification", None);
}

fn confirm_required_manual(database: &Database, task_id: &str) -> ManualConfirmation {
    let detail = current_detail(database, task_id);
    let view = detail
        .criteria
        .iter()
        .find(|view| view.criterion.required && view.criterion.kind == CriterionKind::Manual)
        .expect("required manual criterion");

    TaskService::new(database)
        .confirm_manual_criterion(ConfirmManualInput {
            criterion_id: view.criterion.id.clone(),
            expected_task_revision: detail.task.revision,
            expected_criterion_revision: view.criterion.revision,
            note: Some("测试人工确认".to_owned()),
        })
        .expect("confirm manual criterion")
}

fn task_filter(project_id: &str, include_archived: bool) -> TaskFilter {
    TaskFilter {
        project_id: Some(project_id.to_owned()),
        status: None,
        search: None,
        include_archived: Some(include_archived),
    }
}

#[test]
fn imports_real_git_project_and_reads_read_only_status() {
    let (_repo_directory, repo_path) = init_repo();
    let (_database_directory, database) = temp_database();

    let project = import_project(&database, &repo_path);
    assert!(project.archived_at.is_none());
    assert_eq!(
        Path::new(&project.root_path)
            .file_name()
            .and_then(|name| name.to_str()),
        Some("sample-repo")
    );

    let status = ProjectService::new(&database)
        .git_status(&project.id)
        .expect("git status");
    assert_eq!(status.branch.as_deref(), Some("main"));
    assert!(status.head.as_deref().is_some_and(|head| head.len() >= 40));
    assert!(!status.dirty);
    assert!(!status.detached);
    assert!(!status.unborn);

    fs::write(repo_path.join("dirty.txt"), "untracked\n").expect("write dirty file");
    let dirty_status = ProjectService::new(&database)
        .git_status(&project.id)
        .expect("dirty git status");
    assert!(dirty_status.dirty);
}

#[test]
fn rejects_duplicate_imports_including_subdirectories() {
    let (_repo_directory, repo_path) = init_repo();
    let (_database_directory, database) = temp_database();
    import_project(&database, &repo_path);

    let duplicate = ProjectService::new(&database)
        .import(ImportProjectInput {
            root_path: repo_path.to_string_lossy().to_string(),
            name: None,
        })
        .expect_err("duplicate root");
    assert!(matches!(duplicate, CoreError::Conflict(_)));

    let nested = repo_path.join("nested");
    fs::create_dir_all(&nested).expect("create nested directory");
    let nested_import = ProjectService::new(&database)
        .import(ImportProjectInput {
            root_path: nested.to_string_lossy().to_string(),
            name: None,
        })
        .expect_err("subdirectory resolves to same root");
    assert!(matches!(nested_import, CoreError::Conflict(_)));
}

#[test]
fn rejects_missing_non_git_and_non_directory_paths() {
    let (_database_directory, database) = temp_database();
    let missing = ProjectService::new(&database)
        .import(ImportProjectInput {
            root_path: "D:\\XUSHU\\path-that-does-not-exist".to_owned(),
            name: None,
        })
        .expect_err("missing path");
    assert!(matches!(missing, CoreError::Validation(_)));

    let plain_directory = TempDir::new().expect("plain directory");
    let non_git = ProjectService::new(&database)
        .import(ImportProjectInput {
            root_path: plain_directory.path().to_string_lossy().to_string(),
            name: None,
        })
        .expect_err("non git directory");
    assert!(matches!(non_git, CoreError::Validation(_)));

    let file_path = plain_directory.path().join("file.txt");
    fs::write(&file_path, "not a directory\n").expect("write file");
    let not_directory = ProjectService::new(&database)
        .import(ImportProjectInput {
            root_path: file_path.to_string_lossy().to_string(),
            name: None,
        })
        .expect_err("file path");
    assert!(matches!(not_directory, CoreError::Validation(_)));
}

#[test]
fn supports_unborn_and_detached_repositories() {
    let (_empty_directory, empty_path) = init_empty_repo();
    let (_repo_directory, repo_path) = init_repo();
    run_git(&repo_path, &["checkout", "--detach", "HEAD"]);

    let (_database_directory, database) = temp_database();
    let empty_project = import_project(&database, &empty_path);
    let empty_status = ProjectService::new(&database)
        .git_status(&empty_project.id)
        .expect("unborn status");
    assert_eq!(empty_status.branch.as_deref(), Some("main"));
    assert!(empty_status.head.is_none());
    assert!(empty_status.unborn);
    assert!(!empty_status.detached);

    let detached_project = import_project(&database, &repo_path);
    let detached_status = ProjectService::new(&database)
        .git_status(&detached_project.id)
        .expect("detached status");
    assert!(detached_status.branch.is_none());
    assert!(detached_status.head.is_some());
    assert!(detached_status.detached);
    assert!(!detached_status.unborn);
}

#[test]
fn project_rename_archive_and_restore_retain_tasks() {
    let (_repo_directory, repo_path) = init_repo();
    let (_database_directory, database) = temp_database();
    let project = import_project(&database, &repo_path);
    let task = create_task(&database, &project.id);

    let renamed = ProjectService::new(&database)
        .rename(UpdateProjectInput {
            project_id: project.id.clone(),
            name: "重命名项目".to_owned(),
        })
        .expect("rename project");
    assert_eq!(renamed.name, "重命名项目");

    let archived = ProjectService::new(&database)
        .archive(&project.id)
        .expect("archive project");
    assert!(archived.archived_at.is_some());
    assert!(ProjectService::new(&database)
        .list(false)
        .expect("active projects")
        .is_empty());
    assert_eq!(
        ProjectService::new(&database)
            .list(true)
            .expect("all projects")
            .len(),
        1
    );

    let default_tasks = TaskService::new(&database)
        .list_tasks(task_filter(&project.id, false))
        .expect("default task list");
    assert!(default_tasks.is_empty());
    let archived_tasks = TaskService::new(&database)
        .list_tasks(task_filter(&project.id, true))
        .expect("archived project task list");
    assert_eq!(archived_tasks.len(), 1);
    assert!(current_detail(&database, &task.id)
        .allowed_transitions
        .is_empty());

    let duplicate = ProjectService::new(&database)
        .import(ImportProjectInput {
            root_path: repo_path.to_string_lossy().to_string(),
            name: None,
        })
        .expect_err("archived duplicate");
    assert!(matches!(duplicate, CoreError::Conflict(_)));

    let restored = ProjectService::new(&database)
        .restore(&project.id)
        .expect("restore project");
    assert!(restored.archived_at.is_none());
    assert_eq!(
        ProjectService::new(&database)
            .list(false)
            .expect("restored project list")
            .len(),
        1
    );
    assert!(!current_detail(&database, &task.id)
        .allowed_transitions
        .is_empty());
}

#[test]
fn creates_updates_and_rejects_stale_task_edits() {
    let (_repo_directory, repo_path) = init_repo();
    let (_database_directory, database) = temp_database();
    let project = import_project(&database, &repo_path);
    let task = create_task(&database, &project.id);

    let initial = current_detail(&database, &task.id);
    assert_eq!(initial.task.status, TaskStatus::Draft);
    assert_eq!(initial.task.revision, 1);
    assert_eq!(initial.transitions.len(), 1);

    let updated = TaskService::new(&database)
        .update_task(UpdateTaskInput {
            task_id: task.id.clone(),
            expected_revision: 1,
            title: "更新后的标题".to_owned(),
            description: Some("更新后的描述".to_owned()),
            priority: "high".to_owned(),
        })
        .expect("update task");
    assert_eq!(updated.title, "更新后的标题");
    assert_eq!(updated.priority.as_str(), "high");
    assert_eq!(updated.revision, 2);

    let stale = TaskService::new(&database)
        .update_task(UpdateTaskInput {
            task_id: task.id.clone(),
            expected_revision: 1,
            title: "过期编辑".to_owned(),
            description: None,
            priority: "low".to_owned(),
        })
        .expect_err("stale edit");
    assert!(matches!(stale, CoreError::Conflict(_)));
    assert_eq!(
        current_detail(&database, &task.id).task.title,
        "更新后的标题"
    );
}

#[test]
fn rejects_invalid_task_inputs_and_unknown_projects() {
    let (_repo_directory, repo_path) = init_repo();
    let (_database_directory, database) = temp_database();
    let project = import_project(&database, &repo_path);

    let empty_title = TaskService::new(&database)
        .create_task(CreateTaskInput {
            project_id: project.id.clone(),
            title: "   ".to_owned(),
            description: None,
            priority: "medium".to_owned(),
        })
        .expect_err("empty title");
    assert!(matches!(empty_title, CoreError::Validation(_)));

    let long_title = TaskService::new(&database)
        .create_task(CreateTaskInput {
            project_id: project.id.clone(),
            title: "a".repeat(201),
            description: None,
            priority: "medium".to_owned(),
        })
        .expect_err("long title");
    assert!(matches!(long_title, CoreError::Validation(_)));

    let invalid_priority = TaskService::new(&database)
        .create_task(CreateTaskInput {
            project_id: project.id.clone(),
            title: "任务".to_owned(),
            description: None,
            priority: "critical".to_owned(),
        })
        .expect_err("invalid priority");
    assert!(matches!(invalid_priority, CoreError::Validation(_)));

    let unknown_project = TaskService::new(&database)
        .create_task(CreateTaskInput {
            project_id: "00000000-0000-4000-8000-000000000000".to_owned(),
            title: "任务".to_owned(),
            description: None,
            priority: "medium".to_owned(),
        })
        .expect_err("unknown project");
    assert!(matches!(unknown_project, CoreError::NotFound));

    let invalid_project_id = TaskService::new(&database)
        .list_tasks(TaskFilter {
            project_id: Some("not-a-uuid".to_owned()),
            ..TaskFilter::default()
        })
        .expect_err("invalid project id");
    assert!(matches!(invalid_project_id, CoreError::Validation(_)));
}

#[test]
fn rejects_illegal_transitions_and_requires_readiness_criteria() {
    let (_repo_directory, repo_path) = init_repo();
    let (_database_directory, database) = temp_database();
    let project = import_project(&database, &repo_path);
    let task = create_task(&database, &project.id);

    let no_criterion = transition(&database, &task.id, "ready", None).expect_err("no criteria");
    assert!(matches!(no_criterion, CoreError::Validation(_)));

    add_criterion(&database, &task.id, "manual", false, "可选条件");
    let optional_only = transition(&database, &task.id, "ready", None).expect_err("optional only");
    assert!(matches!(optional_only, CoreError::Validation(_)));

    add_criterion(&database, &task.id, "manual", true, "必需条件");
    transition_ok(&database, &task.id, "ready", None);

    let illegal = transition(&database, &task.id, "done", None).expect_err("illegal done");
    assert!(matches!(illegal, CoreError::InvalidTransition { .. }));
    let illegal_running = transition(&database, &task.id, "draft", None).expect_err("illegal back");
    assert!(matches!(
        illegal_running,
        CoreError::InvalidTransition { .. }
    ));
}

#[test]
fn auto_required_criterion_blocks_done_and_is_never_faked_as_pass() {
    let (_repo_directory, repo_path) = init_repo();
    let (_database_directory, database) = temp_database();
    let project = import_project(&database, &repo_path);
    let task = create_task(&database, &project.id);
    add_criterion(&database, &task.id, "auto", true, "自动测试必须通过");
    advance_to_awaiting(&database, &task.id);

    let blocked = transition(&database, &task.id, "done", None).expect_err("auto gate");
    match blocked {
        CoreError::TaskNotCompletable(message) => {
            assert!(message.contains("Phase 3"), "unexpected message: {message}");
        }
        other => panic!("unexpected error: {other:?}"),
    }

    let detail = current_detail(&database, &task.id);
    assert_eq!(detail.task.status, TaskStatus::AwaitingVerification);
    assert!(!detail.completion.can_complete);
    assert_eq!(detail.completion.satisfied_required_count, 0);
    assert_eq!(detail.criteria.len(), 1);
    assert!(!detail.criteria[0].satisfied);

    let auto_confirm = TaskService::new(&database)
        .confirm_manual_criterion(ConfirmManualInput {
            criterion_id: detail.criteria[0].criterion.id.clone(),
            expected_task_revision: detail.task.revision,
            expected_criterion_revision: detail.criteria[0].criterion.revision,
            note: None,
        })
        .expect_err("auto cannot be manually confirmed");
    assert!(matches!(auto_confirm, CoreError::Validation(_)));
}

#[test]
fn manual_confirmation_flow_allows_done_only_after_confirmation() {
    let (_repo_directory, repo_path) = init_repo();
    let (_database_directory, database) = temp_database();
    let project = import_project(&database, &repo_path);
    let task = create_task(&database, &project.id);
    add_criterion(&database, &task.id, "manual", true, "人工检查结果");
    advance_to_awaiting(&database, &task.id);

    let before = current_detail(&database, &task.id);
    assert!(!before.completion.can_complete);
    assert_eq!(before.completion.required_count, 1);

    let confirmation = confirm_required_manual(&database, &task.id);
    assert_eq!(confirmation.task_revision, before.task.revision);

    let after = current_detail(&database, &task.id);
    assert!(after.completion.can_complete);
    assert_eq!(after.completion.satisfied_required_count, 1);
    assert!(after.criteria[0].satisfied);
    assert!(after.criteria[0].confirmation.is_some());

    let done = transition_ok(&database, &task.id, "done", None);
    assert_eq!(done.status, TaskStatus::Done);
    let completed = current_detail(&database, &task.id);
    assert_eq!(completed.task.status, TaskStatus::Done);
    assert!(completed.allowed_transitions.is_empty());
    assert!(completed
        .transitions
        .iter()
        .any(|transition| transition.to_status == TaskStatus::Done));
}

#[test]
fn manual_confirmation_requires_awaiting_verification() {
    let (_repo_directory, repo_path) = init_repo();
    let (_database_directory, database) = temp_database();
    let project = import_project(&database, &repo_path);
    let task = create_task(&database, &project.id);
    let criterion = add_criterion(&database, &task.id, "manual", true, "人工检查");
    let detail = current_detail(&database, &task.id);

    let too_early = TaskService::new(&database)
        .confirm_manual_criterion(ConfirmManualInput {
            criterion_id: criterion.id,
            expected_task_revision: detail.task.revision,
            expected_criterion_revision: detail.criteria[0].criterion.revision,
            note: None,
        })
        .expect_err("confirmation before awaiting");
    assert!(matches!(too_early, CoreError::Validation(_)));
}

#[test]
fn task_and_criterion_edits_invalidate_existing_confirmations() {
    let (_repo_directory, repo_path) = init_repo();
    let (_database_directory, database) = temp_database();
    let project = import_project(&database, &repo_path);

    let task = create_task(&database, &project.id);
    add_criterion(&database, &task.id, "manual", true, "人工检查");
    advance_to_awaiting(&database, &task.id);
    confirm_required_manual(&database, &task.id);
    let confirmed = current_detail(&database, &task.id);
    assert!(confirmed.completion.can_complete);

    let edited = TaskService::new(&database)
        .update_task(UpdateTaskInput {
            task_id: task.id.clone(),
            expected_revision: confirmed.task.revision,
            title: "编辑后任务".to_owned(),
            description: Some("任务内容已变化".to_owned()),
            priority: "high".to_owned(),
        })
        .expect("edit task");
    assert_eq!(edited.revision, confirmed.task.revision + 1);

    let stale = current_detail(&database, &task.id);
    assert!(!stale.completion.can_complete);
    assert!(!stale.criteria[0].satisfied);
    let blocked = transition(&database, &task.id, "done", None).expect_err("stale confirmation");
    assert!(matches!(blocked, CoreError::TaskNotCompletable(_)));

    let reconfirmed = confirm_required_manual(&database, &task.id);
    assert_eq!(reconfirmed.task_revision, stale.task.revision);
    transition_ok(&database, &task.id, "done", None);

    let second_task = create_task(&database, &project.id);
    add_criterion(&database, &second_task.id, "manual", true, "第二个人工条件");
    advance_to_awaiting(&database, &second_task.id);
    confirm_required_manual(&database, &second_task.id);
    let second_detail = current_detail(&database, &second_task.id);
    TaskService::new(&database)
        .update_criterion(UpdateCriterionInput {
            criterion_id: second_detail.criteria[0].criterion.id.clone(),
            expected_task_revision: second_detail.task.revision,
            expected_criterion_revision: second_detail.criteria[0].criterion.revision,
            description: "修改后的条件".to_owned(),
            kind: "manual".to_owned(),
            required: true,
        })
        .expect("edit criterion");

    let after_criterion_edit = current_detail(&database, &second_task.id);
    assert!(!after_criterion_edit.completion.can_complete);
    assert!(!after_criterion_edit.criteria[0].satisfied);
    let blocked = transition(&database, &second_task.id, "done", None)
        .expect_err("criterion edit invalidates");
    assert!(matches!(blocked, CoreError::TaskNotCompletable(_)));
}

#[test]
fn leaving_awaiting_verification_invalidates_confirmations() {
    let (_repo_directory, repo_path) = init_repo();
    let (_database_directory, database) = temp_database();
    let project = import_project(&database, &repo_path);
    let task = create_task(&database, &project.id);
    add_criterion(&database, &task.id, "manual", true, "人工检查");
    advance_to_awaiting(&database, &task.id);
    confirm_required_manual(&database, &task.id);
    assert!(current_detail(&database, &task.id).completion.can_complete);

    transition_ok(&database, &task.id, "running", None);
    let reworked = current_detail(&database, &task.id);
    assert_eq!(reworked.task.status, TaskStatus::Running);
    assert!(!reworked.completion.can_complete);
    assert!(!reworked.criteria[0].satisfied);

    transition_ok(&database, &task.id, "awaiting_verification", None);
    let blocked = transition(&database, &task.id, "done", None).expect_err("needs re-confirmation");
    assert!(matches!(blocked, CoreError::TaskNotCompletable(_)));

    confirm_required_manual(&database, &task.id);
    transition_ok(&database, &task.id, "done", None);
}

#[test]
fn blocked_status_requires_reason_and_clears_when_unblocked() {
    let (_repo_directory, repo_path) = init_repo();
    let (_database_directory, database) = temp_database();
    let project = import_project(&database, &repo_path);
    let task = create_task(&database, &project.id);
    add_criterion(&database, &task.id, "manual", true, "人工检查");
    transition_ok(&database, &task.id, "ready", None);

    let missing_reason = transition(&database, &task.id, "blocked", None).expect_err("no reason");
    assert!(matches!(missing_reason, CoreError::Validation(_)));

    let blocked = transition_ok(&database, &task.id, "blocked", Some("等待依赖接口完成"));
    assert_eq!(blocked.status, TaskStatus::Blocked);
    assert_eq!(blocked.block_reason.as_deref(), Some("等待依赖接口完成"));
    let detail = current_detail(&database, &task.id);
    assert!(detail.allowed_transitions.contains(&TaskStatus::Running));

    let resumed = transition_ok(&database, &task.id, "running", None);
    assert_eq!(resumed.status, TaskStatus::Running);
    assert!(resumed.block_reason.is_none());
}

#[test]
fn cancelled_is_terminal_and_reopen_is_explicit() {
    let (_repo_directory, repo_path) = init_repo();
    let (_database_directory, database) = temp_database();
    let project = import_project(&database, &repo_path);
    let task = create_task(&database, &project.id);

    let missing_reason = transition(&database, &task.id, "cancelled", None).expect_err("no reason");
    assert!(matches!(missing_reason, CoreError::Validation(_)));

    let cancelled = transition_ok(&database, &task.id, "cancelled", Some("需求取消"));
    assert_eq!(cancelled.status, TaskStatus::Cancelled);

    let illegal = transition(&database, &task.id, "draft", None).expect_err("terminal");
    assert!(matches!(illegal, CoreError::InvalidTransition { .. }));

    let no_reason = TaskService::new(&database)
        .reopen_task(ReopenTaskInput {
            task_id: task.id.clone(),
            expected_revision: cancelled.revision,
            reason: " ".to_owned(),
        })
        .expect_err("reopen reason");
    assert!(matches!(no_reason, CoreError::Validation(_)));

    let reopened = TaskService::new(&database)
        .reopen_task(ReopenTaskInput {
            task_id: task.id.clone(),
            expected_revision: cancelled.revision,
            reason: "重新评估需求".to_owned(),
        })
        .expect("reopen");
    assert_eq!(reopened.status, TaskStatus::Draft);
    assert_eq!(reopened.revision, cancelled.revision + 1);
    let detail = current_detail(&database, &task.id);
    assert!(detail.transitions.iter().any(|transition| {
        transition.from_status == Some(TaskStatus::Cancelled)
            && transition.to_status == TaskStatus::Draft
    }));
}

#[test]
fn criteria_crud_bumps_task_revision_and_rejects_stale_edits() {
    let (_repo_directory, repo_path) = init_repo();
    let (_database_directory, database) = temp_database();
    let project = import_project(&database, &repo_path);
    let task = create_task(&database, &project.id);

    let criterion = add_criterion(&database, &task.id, "manual", true, "初始条件");
    let after_create = current_detail(&database, &task.id);
    assert_eq!(after_create.task.revision, 2);

    let updated = TaskService::new(&database)
        .update_criterion(UpdateCriterionInput {
            criterion_id: criterion.id.clone(),
            expected_task_revision: 2,
            expected_criterion_revision: 1,
            description: "更新后的条件".to_owned(),
            kind: "manual".to_owned(),
            required: true,
        })
        .expect("update criterion");
    assert_eq!(updated.revision, 2);
    assert_eq!(current_detail(&database, &task.id).task.revision, 3);

    let stale_add = TaskService::new(&database)
        .create_criterion(CreateCriterionInput {
            task_id: task.id.clone(),
            expected_task_revision: 2,
            description: "过期新增".to_owned(),
            kind: "manual".to_owned(),
            required: false,
        })
        .expect_err("stale criterion add");
    assert!(matches!(stale_add, CoreError::Conflict(_)));

    TaskService::new(&database)
        .remove_criterion(xushu_core::RemoveCriterionInput {
            criterion_id: criterion.id,
            expected_task_revision: 3,
            expected_criterion_revision: 2,
        })
        .expect("remove criterion");
    let after_remove = current_detail(&database, &task.id);
    assert_eq!(after_remove.task.revision, 4);
    assert!(after_remove.criteria.is_empty());
}

#[test]
fn archive_task_hides_it_and_restore_returns_it() {
    let (_repo_directory, repo_path) = init_repo();
    let (_database_directory, database) = temp_database();
    let project = import_project(&database, &repo_path);
    let task = create_task(&database, &project.id);

    let archived = TaskService::new(&database)
        .archive_task(ArchiveTaskInput {
            task_id: task.id.clone(),
            expected_revision: 1,
        })
        .expect("archive task");
    assert!(archived.archived_at.is_some());
    assert!(TaskService::new(&database)
        .list_tasks(task_filter(&project.id, false))
        .expect("active tasks")
        .is_empty());
    assert_eq!(
        TaskService::new(&database)
            .list_tasks(task_filter(&project.id, true))
            .expect("all tasks")
            .len(),
        1
    );
    let transition_archived = transition(&database, &task.id, "cancelled", Some("取消"))
        .expect_err("archived task transition");
    assert!(matches!(transition_archived, CoreError::Validation(_)));

    let restored = TaskService::new(&database)
        .restore_task(ArchiveTaskInput {
            task_id: task.id.clone(),
            expected_revision: archived.revision,
        })
        .expect("restore task");
    assert!(restored.archived_at.is_none());
    assert_eq!(
        TaskService::new(&database)
            .list_tasks(task_filter(&project.id, false))
            .expect("restored tasks")
            .len(),
        1
    );
}

#[test]
fn task_search_and_status_filters_use_real_data() {
    let (_repo_directory, repo_path) = init_repo();
    let (_database_directory, database) = temp_database();
    let project = import_project(&database, &repo_path);

    let alpha = create_task(&database, &project.id);
    let beta = TaskService::new(&database)
        .create_task(CreateTaskInput {
            project_id: project.id.clone(),
            title: "Beta 任务".to_owned(),
            description: Some("另一个任务".to_owned()),
            priority: "low".to_owned(),
        })
        .expect("create beta");
    add_criterion(&database, &alpha.id, "manual", true, "条件");
    transition_ok(&database, &alpha.id, "ready", None);

    let search = TaskService::new(&database)
        .list_tasks(TaskFilter {
            project_id: Some(project.id.clone()),
            status: None,
            search: Some("Beta".to_owned()),
            include_archived: Some(false),
        })
        .expect("search tasks");
    assert_eq!(search.len(), 1);
    assert_eq!(search[0].id, beta.id);

    let ready = TaskService::new(&database)
        .list_tasks(TaskFilter {
            project_id: Some(project.id.clone()),
            status: Some("ready".to_owned()),
            search: None,
            include_archived: Some(false),
        })
        .expect("filter status");
    assert_eq!(ready.len(), 1);
    assert_eq!(ready[0].id, alpha.id);
}

#[test]
fn workspace_summary_reports_real_counts_and_attention_items() {
    let (_repo_directory, repo_path) = init_repo();
    let (_database_directory, database) = temp_database();
    let project = import_project(&database, &repo_path);

    let draft = create_task(&database, &project.id);

    let awaiting = create_task(&database, &project.id);
    add_criterion(&database, &awaiting.id, "manual", true, "人工验收");
    advance_to_awaiting(&database, &awaiting.id);

    let blocked = create_task(&database, &project.id);
    add_criterion(&database, &blocked.id, "manual", true, "人工验收");
    transition_ok(&database, &blocked.id, "ready", None);
    transition_ok(&database, &blocked.id, "blocked", Some("等待外部依赖"));

    let summary = TaskService::new(&database)
        .workspace_summary()
        .expect("workspace summary");
    assert_eq!(summary.project_count, 1);
    assert_eq!(summary.archived_project_count, 0);
    assert_eq!(summary.task_counts.total, 3);
    assert_eq!(summary.task_counts.draft, 1);
    assert_eq!(summary.task_counts.awaiting_verification, 1);
    assert_eq!(summary.task_counts.blocked, 1);
    assert_eq!(summary.blocked_tasks.len(), 1);
    assert_eq!(summary.blocked_tasks[0].id, blocked.id);
    assert_eq!(summary.awaiting_tasks.len(), 1);
    assert_eq!(summary.awaiting_tasks[0].id, awaiting.id);
    assert_eq!(summary.recent_tasks.len(), 3);
    assert!(summary.recent_tasks.iter().any(|task| task.id == draft.id));

    ProjectService::new(&database)
        .archive(&project.id)
        .expect("archive project");
    let archived_summary = TaskService::new(&database)
        .workspace_summary()
        .expect("archived workspace summary");
    assert_eq!(archived_summary.project_count, 0);
    assert_eq!(archived_summary.archived_project_count, 1);
    assert_eq!(archived_summary.task_counts.total, 0);
    assert!(archived_summary.blocked_tasks.is_empty());
    assert!(archived_summary.awaiting_tasks.is_empty());
}
