use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use tempfile::TempDir;
use xushu_core::{
    ArchiveMemoryInput, ConfirmMemoryInput, CreateMemoryInput, CreateTaskInput, Database,
    ImportProjectInput, MemoryCategory, MemoryConfirmationStatus, MemoryFilter, MemoryService,
    MemorySourceType, MemoryValidityStatus, Project, ProjectService, Task, TaskService,
    UpdateMemoryInput,
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
            title: "记忆关联任务".to_owned(),
            description: Some("用于验证关联".to_owned()),
            priority: "medium".to_owned(),
        })
        .expect("create task")
}

fn create_memory(
    database: &Database,
    project_id: &str,
    title: &str,
    content: &str,
) -> xushu_core::ProjectMemoryView {
    MemoryService::new(database)
        .create(CreateMemoryInput {
            project_id: project_id.to_owned(),
            category: "constraint".to_owned(),
            title: title.to_owned(),
            content: content.to_owned(),
            source_type: None,
            confirmed: None,
            linked_task_id: None,
            linked_evidence_id: None,
        })
        .expect("create memory")
}

#[test]
fn phase5_schema_is_applied_with_memory_decision_and_recovery_tables() {
    let (_directory, database) = open_db();
    let health = database.health().expect("database health");
    assert_eq!(health.schema_version, 7);

    let tables = database
        .with_transaction(|transaction| {
            let mut statement = transaction.prepare(
                "SELECT name FROM sqlite_master
                 WHERE type = 'table' AND name IN (
                    'project_memories',
                    'project_memory_revisions',
                    'decisions',
                    'decision_revisions',
                    'project_recovery_preferences',
                    'recovery_snapshots'
                 )
                 ORDER BY name",
            )?;
            let rows = statement.query_map([], |row| row.get::<_, String>(0))?;
            Ok(rows.collect::<Result<Vec<_>, _>>()?)
        })
        .expect("query schema tables");

    assert_eq!(
        tables,
        vec![
            "decision_revisions".to_owned(),
            "decisions".to_owned(),
            "project_memories".to_owned(),
            "project_memory_revisions".to_owned(),
            "project_recovery_preferences".to_owned(),
            "recovery_snapshots".to_owned(),
        ]
    );
}

#[test]
fn creates_updates_archives_and_restores_memory_with_full_history() {
    let (_repo_dir, repo_path) = init_repo();
    let (_db_dir, database) = open_db();
    let project = import_project(&database, &repo_path);
    let task = create_task(&database, &project.id);
    let service = MemoryService::new(&database);

    let created = service
        .create(CreateMemoryInput {
            project_id: project.id.clone(),
            category: "constraint".to_owned(),
            title: "禁止自动提交".to_owned(),
            content: "任何 Git 提交都必须由用户明确授权。".to_owned(),
            source_type: None,
            confirmed: None,
            linked_task_id: Some(task.id.clone()),
            linked_evidence_id: None,
        })
        .expect("create memory");

    assert_eq!(created.memory.category, MemoryCategory::Constraint);
    assert_eq!(created.memory.source_type, MemorySourceType::User);
    assert_eq!(
        created.memory.confirmation_status,
        MemoryConfirmationStatus::Confirmed
    );
    assert_eq!(created.memory.validity_status, MemoryValidityStatus::Active);
    assert_eq!(created.memory.revision, 1);
    assert!(created.memory.confirmed_at.is_some());
    assert!(created.memory.archived_at.is_none());
    assert_eq!(
        created
            .linked_task
            .as_ref()
            .map(|linked| linked.title.as_str()),
        Some("记忆关联任务")
    );

    let detail = service
        .get(&project.id, &created.memory.id)
        .expect("memory detail");
    assert_eq!(detail.revisions.len(), 1);
    assert_eq!(detail.revisions[0].change_note.as_deref(), Some("创建"));

    let updated = service
        .update(UpdateMemoryInput {
            project_id: project.id.clone(),
            memory_id: created.memory.id.clone(),
            expected_revision: created.memory.revision,
            category: "architecture".to_owned(),
            title: "Git 写入必须授权".to_owned(),
            content: "Git 提交、推送与重置都不得自动执行。".to_owned(),
            linked_task_id: None,
            linked_evidence_id: None,
            change_note: Some("补充适用范围".to_owned()),
        })
        .expect("update memory");

    assert_eq!(updated.memory.revision, 2);
    assert_eq!(updated.memory.category, MemoryCategory::Architecture);
    assert_eq!(updated.memory.title, "Git 写入必须授权");
    assert!(updated.linked_task.is_none());
    assert_eq!(
        updated.memory.confirmation_status,
        MemoryConfirmationStatus::Confirmed
    );

    let detail = service
        .get(&project.id, &created.memory.id)
        .expect("memory detail");
    assert_eq!(detail.revisions.len(), 2);
    let first = detail
        .revisions
        .iter()
        .find(|revision| revision.revision == 1)
        .expect("revision 1");
    assert_eq!(first.title, "禁止自动提交");
    assert_eq!(first.content, "任何 Git 提交都必须由用户明确授权。");

    let archived = service
        .archive(ArchiveMemoryInput {
            project_id: project.id.clone(),
            memory_id: created.memory.id.clone(),
            expected_revision: updated.memory.revision,
            reason: Some("已被新规范取代".to_owned()),
        })
        .expect("archive memory");
    assert_eq!(
        archived.memory.validity_status,
        MemoryValidityStatus::Archived
    );
    assert!(archived.memory.archived_at.is_some());

    let active = service
        .list(MemoryFilter {
            project_id: project.id.clone(),
            category: None,
            include_archived: None,
            task_id: None,
        })
        .expect("list active");
    assert!(active.is_empty());

    let all = service
        .list(MemoryFilter {
            project_id: project.id.clone(),
            category: None,
            include_archived: Some(true),
            task_id: None,
        })
        .expect("list all");
    assert_eq!(all.len(), 1);

    let restored = service
        .restore(ArchiveMemoryInput {
            project_id: project.id.clone(),
            memory_id: created.memory.id.clone(),
            expected_revision: archived.memory.revision,
            reason: Some("恢复使用".to_owned()),
        })
        .expect("restore memory");
    assert_eq!(
        restored.memory.validity_status,
        MemoryValidityStatus::Active
    );
    assert!(restored.memory.archived_at.is_none());

    let detail = service
        .get(&project.id, &created.memory.id)
        .expect("memory detail");
    assert_eq!(detail.revisions.len(), 4);
}

#[test]
fn inference_memory_starts_unconfirmed_and_requires_human_confirmation() {
    let (_repo_dir, repo_path) = init_repo();
    let (_db_dir, database) = open_db();
    let project = import_project(&database, &repo_path);
    let service = MemoryService::new(&database);

    let error = service
        .create(CreateMemoryInput {
            project_id: project.id.clone(),
            category: "fact".to_owned(),
            title: "AI 推断".to_owned(),
            content: "推断内容".to_owned(),
            source_type: Some("inference".to_owned()),
            confirmed: Some(true),
            linked_task_id: None,
            linked_evidence_id: None,
        })
        .expect_err("inference must not be created as confirmed");
    assert!(matches!(error, xushu_core::CoreError::Validation(_)));

    let created = service
        .create(CreateMemoryInput {
            project_id: project.id.clone(),
            category: "fact".to_owned(),
            title: "AI 推断".to_owned(),
            content: "推断内容".to_owned(),
            source_type: Some("inference".to_owned()),
            confirmed: Some(false),
            linked_task_id: None,
            linked_evidence_id: None,
        })
        .expect("create unconfirmed inference memory");
    assert_eq!(
        created.memory.confirmation_status,
        MemoryConfirmationStatus::Unconfirmed
    );
    assert!(created.memory.confirmed_at.is_none());

    let updated = service
        .update(UpdateMemoryInput {
            project_id: project.id.clone(),
            memory_id: created.memory.id.clone(),
            expected_revision: created.memory.revision,
            category: "fact".to_owned(),
            title: "AI 推断（修订）".to_owned(),
            content: "仍未确认".to_owned(),
            linked_task_id: None,
            linked_evidence_id: None,
            change_note: None,
        })
        .expect("update unconfirmed memory");
    assert_eq!(
        updated.memory.confirmation_status,
        MemoryConfirmationStatus::Unconfirmed
    );

    let confirmed = service
        .confirm(ConfirmMemoryInput {
            project_id: project.id.clone(),
            memory_id: created.memory.id.clone(),
            expected_revision: updated.memory.revision,
        })
        .expect("confirm memory");
    assert_eq!(
        confirmed.memory.confirmation_status,
        MemoryConfirmationStatus::Confirmed
    );
    assert!(confirmed.memory.confirmed_at.is_some());

    let again = service
        .confirm(ConfirmMemoryInput {
            project_id: project.id.clone(),
            memory_id: created.memory.id.clone(),
            expected_revision: confirmed.memory.revision,
        })
        .expect("confirm is idempotent");
    assert_eq!(again.memory.revision, confirmed.memory.revision);
}

#[test]
fn validates_memory_input_and_revision_conflicts() {
    let (_repo_dir, repo_path) = init_repo();
    let (_db_dir, database) = open_db();
    let project = import_project(&database, &repo_path);
    let service = MemoryService::new(&database);

    let invalid_cases = [
        ("".to_owned(), "内容".to_owned(), "标题不能为空"),
        ("a".repeat(201), "内容".to_owned(), "最多 200 个字符"),
        ("标题".to_owned(), "".to_owned(), "内容不能为空"),
        ("标题".to_owned(), "a".repeat(20_001), "最多 20000 个字符"),
        ("bad\n标题".to_owned(), "内容".to_owned(), "控制字符"),
    ];
    for (title, content, expected) in invalid_cases {
        let error = service
            .create(CreateMemoryInput {
                project_id: project.id.clone(),
                category: "knowledge".to_owned(),
                title,
                content,
                source_type: None,
                confirmed: None,
                linked_task_id: None,
                linked_evidence_id: None,
            })
            .expect_err("invalid memory");
        match error {
            xushu_core::CoreError::Validation(message) => {
                assert!(message.contains(expected), "unexpected message: {message}");
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }

    let invalid_category = service
        .create(CreateMemoryInput {
            project_id: project.id.clone(),
            category: "unknown".to_owned(),
            title: "标题".to_owned(),
            content: "内容".to_owned(),
            source_type: None,
            confirmed: None,
            linked_task_id: None,
            linked_evidence_id: None,
        })
        .expect_err("invalid category");
    assert!(matches!(
        invalid_category,
        xushu_core::CoreError::Validation(_)
    ));

    let memory = create_memory(&database, &project.id, "约束", "内容");
    let stale = service
        .update(UpdateMemoryInput {
            project_id: project.id.clone(),
            memory_id: memory.memory.id.clone(),
            expected_revision: memory.memory.revision + 1,
            category: "constraint".to_owned(),
            title: "约束".to_owned(),
            content: "内容".to_owned(),
            linked_task_id: None,
            linked_evidence_id: None,
            change_note: None,
        })
        .expect_err("stale revision");
    assert!(matches!(stale, xushu_core::CoreError::Conflict(_)));

    let missing = service
        .get(&project.id, "99999999-9999-4999-8999-999999999999")
        .expect_err("missing memory");
    assert!(matches!(missing, xushu_core::CoreError::NotFound));
}

#[test]
fn enforces_project_isolation_for_memory_reads_and_writes() {
    let (_repo_a, repo_a) = init_repo();
    let (_repo_b, repo_b) = init_repo();
    let (_db_dir, database) = open_db();
    let project_a = import_project(&database, &repo_a);
    let project_b = import_project(&database, &repo_b);
    let task_b = create_task(&database, &project_b.id);
    let service = MemoryService::new(&database);
    let memory = create_memory(&database, &project_a.id, "隔离", "只属于项目 A");

    let error = service
        .get(&project_b.id, &memory.memory.id)
        .expect_err("cross project read");
    assert!(matches!(error, xushu_core::CoreError::NotFound));

    let error = service
        .update(UpdateMemoryInput {
            project_id: project_b.id.clone(),
            memory_id: memory.memory.id.clone(),
            expected_revision: memory.memory.revision,
            category: "constraint".to_owned(),
            title: "越权".to_owned(),
            content: "不得写入".to_owned(),
            linked_task_id: None,
            linked_evidence_id: None,
            change_note: None,
        })
        .expect_err("cross project update");
    assert!(matches!(error, xushu_core::CoreError::NotFound));

    let error = service
        .create(CreateMemoryInput {
            project_id: project_a.id.clone(),
            category: "fact".to_owned(),
            title: "错误关联".to_owned(),
            content: "关联了其他项目的任务".to_owned(),
            source_type: None,
            confirmed: None,
            linked_task_id: Some(task_b.id.clone()),
            linked_evidence_id: None,
        })
        .expect_err("cross project task link");
    assert!(matches!(error, xushu_core::CoreError::Validation(_)));

    let listed_b = service
        .list(MemoryFilter {
            project_id: project_b.id.clone(),
            category: None,
            include_archived: None,
            task_id: None,
        })
        .expect("list project B");
    assert!(listed_b.is_empty());

    let listed_a = service
        .list(MemoryFilter {
            project_id: project_a.id.clone(),
            category: None,
            include_archived: None,
            task_id: None,
        })
        .expect("list project A");
    assert_eq!(listed_a.len(), 1);
}

#[test]
fn validates_linked_evidence_and_archived_project_writes() {
    let (_repo_a, repo_a) = init_repo();
    let (_repo_b, repo_b) = init_repo();
    let (_db_dir, database) = open_db();
    let project_a = import_project(&database, &repo_a);
    let project_b = import_project(&database, &repo_b);
    let task_a = create_task(&database, &project_a.id);
    let task_b = create_task(&database, &project_b.id);
    let evidence_a = "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa";
    let evidence_b = "bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb";
    database
        .with_transaction(|transaction| {
            for (id, task_id) in [
                (evidence_a, task_a.id.as_str()),
                (evidence_b, task_b.id.as_str()),
            ] {
                transaction.execute(
                    "INSERT INTO evidence (
                        id, source_type, task_id, run_id, summary, redacted_output,
                        content_digest, sensitive_flags, created_at
                     ) VALUES (?1, 'verification_run', ?2, NULL, '摘要', NULL,
                        'sha256:test', '', '2026-01-01T00:00:00.000Z')",
                    rusqlite::params![id, task_id],
                )?;
            }
            Ok(())
        })
        .expect("insert evidence");

    let service = MemoryService::new(&database);
    let error = service
        .create(CreateMemoryInput {
            project_id: project_a.id.clone(),
            category: "fact".to_owned(),
            title: "错误证据".to_owned(),
            content: "来自其他项目".to_owned(),
            source_type: None,
            confirmed: None,
            linked_task_id: None,
            linked_evidence_id: Some(evidence_b.to_owned()),
        })
        .expect_err("cross project evidence link");
    assert!(matches!(error, xushu_core::CoreError::Validation(_)));

    let linked = service
        .create(CreateMemoryInput {
            project_id: project_a.id.clone(),
            category: "fact".to_owned(),
            title: "本项目的证据".to_owned(),
            content: "引用本项目证据".to_owned(),
            source_type: None,
            confirmed: None,
            linked_task_id: None,
            linked_evidence_id: Some(evidence_a.to_owned()),
        })
        .expect("link evidence");
    assert_eq!(
        linked.memory.linked_evidence_id.as_deref(),
        Some(evidence_a)
    );

    ProjectService::new(&database)
        .archive(&project_a.id)
        .expect("archive project");
    let error = service
        .create(CreateMemoryInput {
            project_id: project_a.id.clone(),
            category: "fact".to_owned(),
            title: "归档后写入".to_owned(),
            content: "应该失败".to_owned(),
            source_type: None,
            confirmed: None,
            linked_task_id: None,
            linked_evidence_id: None,
        })
        .expect_err("archived project write");
    assert!(matches!(error, xushu_core::CoreError::Validation(_)));
}

#[test]
fn memory_history_write_failure_rolls_back_the_memory_change() {
    let (_repo_dir, repo_path) = init_repo();
    let (_db_dir, database) = open_db();
    let project = import_project(&database, &repo_path);
    let service = MemoryService::new(&database);
    let memory = create_memory(&database, &project.id, "原始标题", "原始内容");

    database
        .with_transaction(|transaction| {
            transaction.execute_batch(
                "CREATE TRIGGER fail_memory_revision
                 BEFORE INSERT ON project_memory_revisions
                 WHEN NEW.revision > 1
                 BEGIN
                     SELECT RAISE(ABORT, 'synthetic revision failure');
                 END;",
            )?;
            Ok(())
        })
        .expect("create trigger");

    let error = service
        .update(UpdateMemoryInput {
            project_id: project.id.clone(),
            memory_id: memory.memory.id.clone(),
            expected_revision: memory.memory.revision,
            category: "risk".to_owned(),
            title: "不应写入".to_owned(),
            content: "不应写入".to_owned(),
            linked_task_id: None,
            linked_evidence_id: None,
            change_note: Some("事务失败".to_owned()),
        })
        .expect_err("trigger must fail update");
    assert!(matches!(error, xushu_core::CoreError::Database(_)));

    let detail = service
        .get(&project.id, &memory.memory.id)
        .expect("memory detail");
    assert_eq!(detail.memory.memory.revision, 1);
    assert_eq!(detail.memory.memory.title, "原始标题");
    assert_eq!(detail.memory.memory.category, MemoryCategory::Constraint);
    assert_eq!(detail.revisions.len(), 1);
}

#[test]
fn filters_memories_by_category_task_and_archive_state() {
    let (_repo_dir, repo_path) = init_repo();
    let (_db_dir, database) = open_db();
    let project = import_project(&database, &repo_path);
    let task = create_task(&database, &project.id);
    let service = MemoryService::new(&database);

    service
        .create(CreateMemoryInput {
            project_id: project.id.clone(),
            category: "risk".to_owned(),
            title: "风险一".to_owned(),
            content: "内容".to_owned(),
            source_type: None,
            confirmed: None,
            linked_task_id: Some(task.id.clone()),
            linked_evidence_id: None,
        })
        .expect("create risk memory");
    service
        .create(CreateMemoryInput {
            project_id: project.id.clone(),
            category: "goal".to_owned(),
            title: "目标一".to_owned(),
            content: "内容".to_owned(),
            source_type: None,
            confirmed: None,
            linked_task_id: None,
            linked_evidence_id: None,
        })
        .expect("create goal memory");

    let risks = service
        .list(MemoryFilter {
            project_id: project.id.clone(),
            category: Some("risk".to_owned()),
            include_archived: None,
            task_id: None,
        })
        .expect("filter category");
    assert_eq!(risks.len(), 1);
    assert_eq!(risks[0].memory.category, MemoryCategory::Risk);

    let by_task = service
        .list(MemoryFilter {
            project_id: project.id.clone(),
            category: None,
            include_archived: None,
            task_id: Some(task.id.clone()),
        })
        .expect("filter task");
    assert_eq!(by_task.len(), 1);
    assert_eq!(by_task[0].memory.title, "风险一");

    let invalid = service.list(MemoryFilter {
        project_id: project.id.clone(),
        category: Some("invalid".to_owned()),
        include_archived: None,
        task_id: None,
    });
    assert!(matches!(invalid, Err(xushu_core::CoreError::Validation(_))));
}
