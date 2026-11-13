use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use tempfile::TempDir;
use xushu_core::{
    ConfirmDecisionInput, CreateDecisionInput, CreateTaskInput, Database, DecisionFilter,
    DecisionService, DecisionStatus, DeprecateDecisionInput, ImportProjectInput, Project,
    ProjectService, Task, TaskService, UpdateDecisionInput,
};

fn open_db() -> (TempDir, Database) {
    let directory = TempDir::new().expect("temp directory");
    let database = Database::open(directory.path().join("xushu.db")).expect("open database");
    (directory, database)
}

fn run_git(path: &Path, args: &[&str]) {
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
            title: "决策关联任务".to_owned(),
            description: None,
            priority: "medium".to_owned(),
        })
        .expect("create task")
}

fn decision_input(project_id: &str, title: &str) -> CreateDecisionInput {
    CreateDecisionInput {
        project_id: project_id.to_owned(),
        task_id: None,
        title: title.to_owned(),
        problem: "需要选择本地持久化方案".to_owned(),
        context: "离线优先，不引入云服务。".to_owned(),
        alternatives: vec!["SQLite".to_owned(), "JSON 文件".to_owned()],
        decision: "使用 SQLite 并启用迁移".to_owned(),
        rationale: "事务、约束与可恢复性更好。".to_owned(),
        risks: "需要维护迁移版本。".to_owned(),
        supersedes_id: None,
    }
}

fn create_decision(database: &Database, project_id: &str, title: &str) -> xushu_core::DecisionView {
    DecisionService::new(database)
        .create(decision_input(project_id, title))
        .expect("create decision")
}

fn confirm(
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

#[test]
fn creates_confirms_and_preserves_decision_history() {
    let (_repo_dir, repo_path) = init_repo();
    let (_db_dir, database) = open_db();
    let project = import_project(&database, &repo_path);
    let service = DecisionService::new(&database);

    let created = create_decision(&database, &project.id, "选择 SQLite");
    assert_eq!(created.decision.status, DecisionStatus::Draft);
    assert!(created.decision.confirmed_at.is_none());
    assert_eq!(created.decision.alternatives.len(), 2);
    assert_eq!(created.decision.revision, 1);

    let detail = service
        .get(&project.id, &created.decision.id)
        .expect("decision detail");
    assert_eq!(detail.revisions.len(), 1);
    assert_eq!(detail.revisions[0].change_note.as_deref(), Some("创建"));

    let updated = service
        .update(UpdateDecisionInput {
            project_id: project.id.clone(),
            decision_id: created.decision.id.clone(),
            expected_revision: created.decision.revision,
            task_id: None,
            title: "选择 SQLite（修订）".to_owned(),
            problem: "需要选择本地持久化方案".to_owned(),
            context: "离线优先，不引入云服务。".to_owned(),
            alternatives: vec!["SQLite".to_owned(), "JSON 文件".to_owned()],
            decision: "使用 SQLite + rusqlite + 迁移".to_owned(),
            rationale: "事务、约束与可恢复性更好。".to_owned(),
            risks: "需要维护迁移版本。".to_owned(),
            supersedes_id: None,
            change_note: Some("补充实现细节".to_owned()),
        })
        .expect("update draft decision");
    assert_eq!(updated.decision.revision, 2);
    assert_eq!(updated.decision.status, DecisionStatus::Draft);

    let confirmed = confirm(&database, &project.id, &updated);
    assert_eq!(confirmed.decision.status, DecisionStatus::Confirmed);
    assert!(confirmed.decision.confirmed_at.is_some());
    assert_eq!(confirmed.decision.revision, 3);

    let detail = service
        .get(&project.id, &created.decision.id)
        .expect("decision detail");
    assert_eq!(detail.revisions.len(), 3);
    let first = detail
        .revisions
        .iter()
        .find(|revision| revision.revision == 1)
        .expect("revision 1");
    assert_eq!(first.title, "选择 SQLite");
    assert_eq!(first.status, DecisionStatus::Draft);
    let last = detail
        .revisions
        .iter()
        .find(|revision| revision.revision == 3)
        .expect("revision 3");
    assert_eq!(last.status, DecisionStatus::Confirmed);
    assert_eq!(last.change_note.as_deref(), Some("确认决策"));
}

#[test]
fn supersede_links_new_and_old_decision_without_touching_task_state() {
    let (_repo_dir, repo_path) = init_repo();
    let (_db_dir, database) = open_db();
    let project = import_project(&database, &repo_path);
    let task = create_task(&database, &project.id);
    let before = TaskService::new(&database)
        .get_task(&task.id)
        .expect("task before");

    let old = create_decision(&database, &project.id, "旧决策");
    let old = confirm(&database, &project.id, &old);

    let mut input = decision_input(&project.id, "新决策");
    input.task_id = Some(task.id.clone());
    input.supersedes_id = Some(old.decision.id.clone());
    let new = DecisionService::new(&database)
        .create(input)
        .expect("create superseding decision");
    assert_eq!(new.decision.status, DecisionStatus::Draft);
    assert_eq!(
        new.decision.supersedes_id.as_deref(),
        Some(old.decision.id.as_str())
    );
    assert_eq!(
        new.supersedes_title.as_deref(),
        Some("旧决策"),
        "draft should already show the decision it will replace"
    );

    let old_still_confirmed = DecisionService::new(&database)
        .get(&project.id, &old.decision.id)
        .expect("old decision");
    assert_eq!(
        old_still_confirmed.decision.decision.status,
        DecisionStatus::Confirmed
    );

    let new = confirm(&database, &project.id, &new);
    assert_eq!(new.decision.status, DecisionStatus::Confirmed);

    let old_detail = DecisionService::new(&database)
        .get(&project.id, &old.decision.id)
        .expect("old decision detail");
    assert_eq!(
        old_detail.decision.decision.status,
        DecisionStatus::Superseded
    );
    assert_eq!(
        old_detail.decision.decision.superseded_by_id.as_deref(),
        Some(new.decision.id.as_str())
    );
    assert_eq!(
        old_detail.decision.superseded_by_title.as_deref(),
        Some("新决策")
    );
    assert!(old_detail.decision.decision.confirmed_at.is_some());

    let after = TaskService::new(&database)
        .get_task(&task.id)
        .expect("task after");
    assert_eq!(after.task.status, before.task.status);
    assert_eq!(after.task.revision, before.task.revision);
    assert_eq!(after.task.updated_at, before.task.updated_at);
}

#[test]
fn rejects_invalid_transitions_and_confirmed_edits() {
    let (_repo_dir, repo_path) = init_repo();
    let (_db_dir, database) = open_db();
    let project = import_project(&database, &repo_path);
    let service = DecisionService::new(&database);
    let draft = create_decision(&database, &project.id, "草稿决策");

    let error = service
        .deprecate(DeprecateDecisionInput {
            project_id: project.id.clone(),
            decision_id: draft.decision.id.clone(),
            expected_revision: draft.decision.revision,
            reason: "不再采用".to_owned(),
        })
        .expect("deprecate draft");
    assert_eq!(error.decision.status, DecisionStatus::Deprecated);

    let error = service
        .confirm(ConfirmDecisionInput {
            project_id: project.id.clone(),
            decision_id: draft.decision.id.clone(),
            expected_revision: error.decision.revision,
        })
        .expect_err("deprecated cannot be confirmed");
    assert!(matches!(error, xushu_core::CoreError::Validation(_)));

    let confirmed = create_decision(&database, &project.id, "已确认决策");
    let confirmed = confirm(&database, &project.id, &confirmed);
    let error = service
        .update(UpdateDecisionInput {
            project_id: project.id.clone(),
            decision_id: confirmed.decision.id.clone(),
            expected_revision: confirmed.decision.revision,
            task_id: None,
            title: "直接覆盖".to_owned(),
            problem: "不应允许".to_owned(),
            context: "不应允许".to_owned(),
            alternatives: Vec::new(),
            decision: "不应允许".to_owned(),
            rationale: "不应允许".to_owned(),
            risks: "不应允许".to_owned(),
            supersedes_id: None,
            change_note: Some("越权修改".to_owned()),
        })
        .expect_err("confirmed decision edit");
    assert!(matches!(error, xushu_core::CoreError::Validation(_)));

    let stale_confirm = service
        .confirm(ConfirmDecisionInput {
            project_id: project.id.clone(),
            decision_id: confirmed.decision.id.clone(),
            expected_revision: confirmed.decision.revision + 1,
        })
        .expect_err("stale confirm");
    assert!(matches!(stale_confirm, xushu_core::CoreError::Conflict(_)));

    let again = confirm(&database, &project.id, &confirmed);
    assert_eq!(again.decision.revision, confirmed.decision.revision);
}

#[test]
fn deprecates_confirmed_decision_with_reason_and_history() {
    let (_repo_dir, repo_path) = init_repo();
    let (_db_dir, database) = open_db();
    let project = import_project(&database, &repo_path);
    let service = DecisionService::new(&database);
    let confirmed = create_decision(&database, &project.id, "待废弃决策");
    let confirmed = confirm(&database, &project.id, &confirmed);

    let deprecated = service
        .deprecate(DeprecateDecisionInput {
            project_id: project.id.clone(),
            decision_id: confirmed.decision.id.clone(),
            expected_revision: confirmed.decision.revision,
            reason: "外部约束发生变化".to_owned(),
        })
        .expect("deprecate confirmed");
    assert_eq!(deprecated.decision.status, DecisionStatus::Deprecated);
    assert!(deprecated.decision.deprecated_at.is_some());
    assert!(deprecated.decision.confirmed_at.is_some());

    let detail = service
        .get(&project.id, &confirmed.decision.id)
        .expect("decision detail");
    assert_eq!(detail.revisions.len(), 3);
    assert_eq!(
        detail.revisions[0].change_note.as_deref(),
        Some("废弃：外部约束发生变化")
    );
    assert_eq!(detail.revisions[0].status, DecisionStatus::Deprecated);

    let error = service
        .deprecate(DeprecateDecisionInput {
            project_id: project.id.clone(),
            decision_id: confirmed.decision.id.clone(),
            expected_revision: deprecated.decision.revision,
            reason: "再次废弃".to_owned(),
        })
        .expect("deprecate is idempotent");
    assert_eq!(error.decision.revision, deprecated.decision.revision);
}

#[test]
fn enforces_project_isolation_for_decisions_and_task_links() {
    let (_repo_a, repo_a) = init_repo();
    let (_repo_b, repo_b) = init_repo();
    let (_db_dir, database) = open_db();
    let project_a = import_project(&database, &repo_a);
    let project_b = import_project(&database, &repo_b);
    let task_b = create_task(&database, &project_b.id);
    let service = DecisionService::new(&database);
    let decision = create_decision(&database, &project_a.id, "项目 A 决策");

    let error = service
        .get(&project_b.id, &decision.decision.id)
        .expect_err("cross project read");
    assert!(matches!(error, xushu_core::CoreError::NotFound));

    let mut input = decision_input(&project_a.id, "错误关联");
    input.task_id = Some(task_b.id.clone());
    let error = service.create(input).expect_err("cross project task link");
    assert!(matches!(error, xushu_core::CoreError::Validation(_)));

    let listed_b = service
        .list(DecisionFilter {
            project_id: project_b.id.clone(),
            status: None,
            task_id: None,
        })
        .expect("list project B");
    assert!(listed_b.is_empty());

    let listed_a = service
        .list(DecisionFilter {
            project_id: project_a.id.clone(),
            status: None,
            task_id: None,
        })
        .expect("list project A");
    assert_eq!(listed_a.len(), 1);
}

#[test]
fn validates_decision_input_limits_and_alternatives() {
    let (_repo_dir, repo_path) = init_repo();
    let (_db_dir, database) = open_db();
    let project = import_project(&database, &repo_path);
    let service = DecisionService::new(&database);

    let invalid_cases: Vec<(CreateDecisionInput, &str)> = vec![
        (
            CreateDecisionInput {
                title: "".to_owned(),
                ..decision_input(&project.id, "placeholder")
            },
            "标题不能为空",
        ),
        (
            CreateDecisionInput {
                problem: "a".repeat(4001),
                ..decision_input(&project.id, "placeholder")
            },
            "最多 4000 个字符",
        ),
        (
            CreateDecisionInput {
                alternatives: vec!["a".to_owned(); 21],
                ..decision_input(&project.id, "placeholder")
            },
            "备选方案最多 20 个",
        ),
        (
            CreateDecisionInput {
                alternatives: vec!["a".repeat(2001)],
                ..decision_input(&project.id, "placeholder")
            },
            "备选方案最多 2000 个字符",
        ),
        (
            CreateDecisionInput {
                decision: "".to_owned(),
                ..decision_input(&project.id, "placeholder")
            },
            "最终方案不能为空",
        ),
        (
            CreateDecisionInput {
                title: "bad\n标题".to_owned(),
                ..decision_input(&project.id, "placeholder")
            },
            "控制字符",
        ),
    ];

    for (input, expected) in invalid_cases {
        let error = service.create(input).expect_err("invalid decision");
        match error {
            xushu_core::CoreError::Validation(message) => {
                assert!(message.contains(expected), "unexpected message: {message}");
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }

    let invalid_status = service.list(DecisionFilter {
        project_id: project.id.clone(),
        status: Some("unknown".to_owned()),
        task_id: None,
    });
    assert!(matches!(
        invalid_status,
        Err(xushu_core::CoreError::Validation(_))
    ));

    let empty_alternatives = service
        .create(CreateDecisionInput {
            alternatives: Vec::new(),
            ..decision_input(&project.id, "无备选方案")
        })
        .expect("empty alternatives allowed");
    assert!(empty_alternatives.decision.alternatives.is_empty());
}

#[test]
fn decision_history_write_failure_rolls_back_confirmation_and_supersede() {
    let (_repo_dir, repo_path) = init_repo();
    let (_db_dir, database) = open_db();
    let project = import_project(&database, &repo_path);
    let service = DecisionService::new(&database);

    let old = create_decision(&database, &project.id, "旧决策");
    let old = confirm(&database, &project.id, &old);
    let mut input = decision_input(&project.id, "新决策");
    input.supersedes_id = Some(old.decision.id.clone());
    let new = service.create(input).expect("new draft");

    database
        .with_transaction(|transaction| {
            transaction.execute_batch(
                "CREATE TRIGGER fail_decision_revision
                 BEFORE INSERT ON decision_revisions
                 WHEN NEW.decision_id = NEW.decision_id AND NEW.revision > 2
                 BEGIN
                     SELECT RAISE(ABORT, 'synthetic decision revision failure');
                 END;",
            )?;
            Ok(())
        })
        .expect("create trigger");

    let error = service
        .confirm(ConfirmDecisionInput {
            project_id: project.id.clone(),
            decision_id: new.decision.id.clone(),
            expected_revision: new.decision.revision,
        })
        .expect_err("trigger must fail confirmation");
    assert!(matches!(error, xushu_core::CoreError::Database(_)));

    let new_after = service
        .get(&project.id, &new.decision.id)
        .expect("new detail");
    assert_eq!(new_after.decision.decision.status, DecisionStatus::Draft);
    assert_eq!(new_after.decision.decision.revision, 1);

    let old_after = service
        .get(&project.id, &old.decision.id)
        .expect("old detail");
    assert_eq!(
        old_after.decision.decision.status,
        DecisionStatus::Confirmed
    );
    assert!(old_after.decision.decision.superseded_by_id.is_none());
}

#[test]
fn filters_decisions_by_status_and_task() {
    let (_repo_dir, repo_path) = init_repo();
    let (_db_dir, database) = open_db();
    let project = import_project(&database, &repo_path);
    let task = create_task(&database, &project.id);
    let service = DecisionService::new(&database);

    let mut linked_input = decision_input(&project.id, "任务决策");
    linked_input.task_id = Some(task.id.clone());
    let linked = service.create(linked_input).expect("linked decision");
    let _other = create_decision(&database, &project.id, "项目决策");
    let confirmed = confirm(&database, &project.id, &linked);

    let by_status = service
        .list(DecisionFilter {
            project_id: project.id.clone(),
            status: Some("confirmed".to_owned()),
            task_id: None,
        })
        .expect("filter status");
    assert_eq!(by_status.len(), 1);
    assert_eq!(by_status[0].decision.id, confirmed.decision.id);

    let by_task = service
        .list(DecisionFilter {
            project_id: project.id.clone(),
            status: None,
            task_id: Some(task.id.clone()),
        })
        .expect("filter task");
    assert_eq!(by_task.len(), 1);
    assert_eq!(by_task[0].decision.title, "任务决策");

    let all = service
        .list(DecisionFilter {
            project_id: project.id.clone(),
            status: None,
            task_id: None,
        })
        .expect("list all");
    assert_eq!(all.len(), 2);
}
