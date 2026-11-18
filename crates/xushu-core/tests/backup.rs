use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use rusqlite::{params, Connection};
use serde_json::Value;
use sha2::{Digest, Sha256};
use tempfile::TempDir;
use uuid::Uuid;
use xushu_core::{
    backup::{BackupService, DataIntegrityService, RestorePreflightInput, RestoreService},
    code_state, runner, BackupOrigin, ConfirmDecisionInput, CoreError, CreateCriterionInput,
    CreateDecisionInput, CreateMemoryInput, CreateTaskInput, Database, DecisionService,
    ImportProjectInput, MemoryService, Project, ProjectService, RecoveryService, RunControl,
    SaveVerificationSpecInput, StartVerificationInput, Task, TaskFilter, TaskService,
    TransitionTaskInput, VerificationService,
};

struct Seed {
    project: Project,
    task: Task,
    verification_run_id: String,
    memory_title: String,
    decision_title: String,
}

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
    fs::write(path.join("README.md"), "phase 6 fixture\n").expect("write readme");
    run_git(&path, &["add", "-A"]);
    run_git(&path, &["commit", "-m", "initial"]);
    (directory, path)
}

fn detail(database: &Database, task_id: &str) -> xushu_core::TaskDetail {
    TaskService::new(database)
        .get_task(task_id)
        .expect("task detail")
}

fn seed(database: &Database, repo: &Path) -> Seed {
    let project = ProjectService::new(database)
        .import(ImportProjectInput {
            root_path: repo.to_string_lossy().to_string(),
            name: None,
        })
        .expect("import project");
    let task = TaskService::new(database)
        .create_task(CreateTaskInput {
            project_id: project.id.clone(),
            title: "Phase 6 备份恢复任务".to_owned(),
            description: Some("备份与恢复测试".to_owned()),
            priority: "high".to_owned(),
        })
        .expect("create task");
    let criterion = TaskService::new(database)
        .create_criterion(CreateCriterionInput {
            task_id: task.id.clone(),
            expected_task_revision: task.revision,
            description: "git --version 必须成功".to_owned(),
            kind: "auto".to_owned(),
            required: true,
        })
        .expect("create criterion");
    TaskService::new(database)
        .create_criterion(CreateCriterionInput {
            task_id: task.id.clone(),
            expected_task_revision: detail(database, &task.id).task.revision,
            description: "人工确认恢复结果".to_owned(),
            kind: "manual".to_owned(),
            required: true,
        })
        .expect("create manual criterion");

    for status in ["ready", "running", "awaiting_verification"] {
        let current = detail(database, &task.id);
        TaskService::new(database)
            .transition_task_status(TransitionTaskInput {
                task_id: task.id.clone(),
                expected_revision: current.task.revision,
                to_status: status.to_owned(),
                reason: None,
            })
            .expect("transition task");
    }

    let current = detail(database, &task.id);
    let current_criterion = current
        .criteria
        .iter()
        .find(|view| view.criterion.id == criterion.id)
        .expect("criterion view")
        .criterion
        .clone();
    VerificationService::new(database)
        .save_spec(SaveVerificationSpecInput {
            criterion_id: criterion.id.clone(),
            expected_task_revision: current.task.revision,
            expected_criterion_revision: current_criterion.revision,
            expected_spec_revision: None,
            executable: "git".to_owned(),
            args: vec!["--version".to_owned()],
            cwd: Some(repo.to_string_lossy().to_string()),
            timeout_seconds: 30,
            output_limit_bytes: 4096,
        })
        .expect("save spec");
    let spec = VerificationService::new(database)
        .get_spec(&criterion.id)
        .expect("get spec")
        .expect("spec exists");
    let prepared = VerificationService::new(database)
        .prepare_run(StartVerificationInput {
            criterion_id: criterion.id.clone(),
            expected_task_revision: detail(database, &task.id).task.revision,
            expected_criterion_revision: current_criterion.revision,
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
        .expect("finalize run");

    let memory = MemoryService::new(database)
        .create(CreateMemoryInput {
            project_id: project.id.clone(),
            category: "constraint".to_owned(),
            title: "备份记忆：必须保留版本历史".to_owned(),
            content: "恢复后修订历史必须仍然可读。".to_owned(),
            source_type: Some("user".to_owned()),
            confirmed: Some(true),
            linked_task_id: Some(task.id.clone()),
            linked_evidence_id: None,
        })
        .expect("create memory");
    let memory = MemoryService::new(database)
        .update(xushu_core::UpdateMemoryInput {
            project_id: project.id.clone(),
            memory_id: memory.memory.id,
            expected_revision: memory.memory.revision,
            category: "constraint".to_owned(),
            title: memory.memory.title,
            content: "恢复后两条修订都必须可读。".to_owned(),
            linked_task_id: Some(task.id.clone()),
            linked_evidence_id: None,
            change_note: Some("校验恢复后的版本链".to_owned()),
        })
        .expect("revise memory");
    let decision = DecisionService::new(database)
        .create(CreateDecisionInput {
            project_id: project.id.clone(),
            task_id: Some(task.id.clone()),
            title: "备份决策：使用 SQLite 备份 API".to_owned(),
            problem: "需要一致性备份".to_owned(),
            context: "数据库使用 WAL".to_owned(),
            alternatives: vec!["复制文件".to_owned(), "Online Backup API".to_owned()],
            decision: "使用 Online Backup API".to_owned(),
            rationale: "避免复制 WAL 导致不一致".to_owned(),
            risks: "需要磁盘空间".to_owned(),
            supersedes_id: None,
        })
        .expect("create decision");
    DecisionService::new(database)
        .confirm(ConfirmDecisionInput {
            project_id: project.id.clone(),
            decision_id: decision.decision.id.clone(),
            expected_revision: decision.decision.revision,
        })
        .expect("confirm decision");
    let replacement = DecisionService::new(database)
        .create(CreateDecisionInput {
            project_id: project.id.clone(),
            task_id: Some(task.id.clone()),
            title: "取代旧决策的备份决策".to_owned(),
            problem: "验证恢复后决策链".to_owned(),
            context: "保留原始决策".to_owned(),
            alternatives: vec![],
            decision: "继续使用 SQLite 在线备份".to_owned(),
            rationale: "恢复时校验两条决策的关系".to_owned(),
            risks: "无".to_owned(),
            supersedes_id: Some(decision.decision.id.clone()),
        })
        .expect("create replacement decision");
    DecisionService::new(database)
        .confirm(ConfirmDecisionInput {
            project_id: project.id.clone(),
            decision_id: replacement.decision.id,
            expected_revision: replacement.decision.revision,
        })
        .expect("confirm replacement decision");
    RecoveryService::new(database)
        .generate(&project.id)
        .expect("generate recovery snapshot");

    seed_claude_metadata(database, &project.id);

    Seed {
        project,
        task,
        verification_run_id: prepared.run.id,
        memory_title: memory.memory.title,
        decision_title: decision.decision.title,
    }
}

fn seed_claude_metadata(database: &Database, project_id: &str) {
    let connection = Connection::open(database.path()).expect("raw database");
    let session_id = Uuid::new_v4().to_string();
    let now = "2026-11-13T10:00:00.000Z";
    connection
        .execute(
            "INSERT INTO claude_integrations (project_id, desired_enabled, created_at, updated_at)
             VALUES (?1, 1, ?2, ?2)",
            params![project_id, now],
        )
        .expect("insert integration");
    connection
        .execute(
            "INSERT INTO claude_sessions (
                id, project_id, session_hash, started_at, last_event_at, ended_at,
                start_source, end_reason, start_observed, created_at, updated_at
             ) VALUES (?1, ?2, ?3, ?4, ?4, NULL, 'startup', NULL, 1, ?4, ?4)",
            params![session_id, project_id, "session-hash-fixture", now],
        )
        .expect("insert session");
    connection
        .execute(
            "INSERT INTO claude_events (
                id, delivery_id, dedupe_key, project_id, session_id, tool_call_id,
                event_type, occurred_at, ingested_at, prompt_hash, tool_name, tool_category,
                failure_kind, is_interrupt, duration_ms, source, end_reason, created_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, NULL, 'session_start', ?6, ?6, NULL, NULL, NULL,
                       NULL, NULL, NULL, 'startup', NULL, ?6)",
            params![
                Uuid::new_v4().to_string(),
                Uuid::new_v4().to_string(),
                "dedupe-fixture",
                project_id,
                session_id,
                now
            ],
        )
        .expect("insert event");
}

fn create_backup(database: &Database, data_directory: &Path) -> xushu_core::BackupRecord {
    let target = data_directory.join("backups");
    BackupService::new(database, data_directory)
        .create_manual(&target.to_string_lossy(), &AtomicBool::new(false))
        .expect("create backup")
}

fn tamper_payload(record: &xushu_core::BackupRecord, fill: &[u8]) {
    let payload = Path::new(&record.directory_path).join("xushu.db");
    fs::write(&payload, fill).expect("tamper payload");
}

fn rewrite_manifest(record: &xushu_core::BackupRecord, mutate: impl FnOnce(&mut Value)) {
    let manifest_path = Path::new(&record.directory_path).join("manifest.json");
    let mut manifest: Value =
        serde_json::from_slice(&fs::read(&manifest_path).expect("read manifest"))
            .expect("parse manifest");
    mutate(&mut manifest);
    fs::write(
        &manifest_path,
        serde_json::to_vec_pretty(&manifest).expect("serialize manifest"),
    )
    .expect("write manifest");
}

fn file_sha256(path: &Path) -> String {
    let bytes = fs::read(path).expect("read file");
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

fn open_preflight(
    database: &Database,
    data_directory: &Path,
    record: &xushu_core::BackupRecord,
) -> xushu_core::RestorePreflightOutcome {
    RestoreService::new(database, data_directory)
        .preflight(RestorePreflightInput {
            backup_directory: record.directory_path.clone(),
        })
        .expect("preflight")
}

fn switch_to_plan(
    database: Database,
    data_directory: &Path,
    plan: &xushu_core::RestorePlan,
    operation_id: &str,
) -> xushu_core::RestoreJournal {
    let mut journal = RestoreService::new(&database, data_directory)
        .create_rollback(operation_id)
        .expect("create rollback");
    database.checkpoint().expect("checkpoint");
    drop(database);
    RestoreService::begin_switch(data_directory, &mut journal, plan).expect("begin switch");
    RestoreService::switch_files(data_directory, &journal, plan).expect("switch files");
    journal
}

#[test]
fn backup_round_trip_preserves_business_rows_and_relations() {
    let (_repo_root, repo) = init_repo();
    let (directory, database) = temp_database();
    let seed = seed(&database, &repo);
    let hook_directory = directory.path().join("hook-inbox");
    let spool = hook_directory.join("pending").join(&seed.project.id);
    fs::create_dir_all(&spool).unwrap();
    fs::write(spool.join("fixture.json"), b"pending fixture envelope").unwrap();
    let salt_path = hook_directory.join(xushu_hook_protocol::SALT_FILE_NAME);
    fs::write(&salt_path, b"fixture salt must stay outside the backup").unwrap();
    let record = create_backup(&database, directory.path());
    assert_eq!(
        record.integrity_status,
        xushu_core::BackupIntegrityStatus::Verified
    );
    assert_eq!(record.origin, BackupOrigin::Manual);
    assert_eq!(record.schema_version, 7);
    let service = BackupService::new(&database, directory.path());
    let verification = service.verify(&record.backup_id).expect("verify backup");
    assert!(verification.verified, "{:?}", verification.checks);
    let registered = service.list().expect("list registered backups");
    assert_eq!(registered.len(), 1);
    assert_eq!(registered[0].backup_id, record.backup_id);
    let package = Path::new(&record.directory_path);
    let manifest_hash = file_sha256(&package.join("manifest.json"));
    let payload_hash = file_sha256(&package.join("xushu.db"));
    let snapshot_rows = read_business_rows(
        &Connection::open_with_flags(
            package.join("xushu.db"),
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )
        .unwrap(),
    );

    let manifest =
        fs::read(Path::new(&record.directory_path).join("manifest.json")).expect("manifest bytes");
    let manifest: Value = serde_json::from_slice(&manifest).expect("manifest json");
    assert_eq!(manifest["files"].as_array().expect("files").len(), 1);
    assert_eq!(manifest["files"][0]["path"], "xushu.db");
    assert_eq!(manifest["encrypted"], false);
    assert_eq!(manifest["scope"], "database");
    assert!(manifest.get("directoryPath").is_none());
    assert!(!manifest.to_string().contains("hook-inbox"));
    let completion = |database: &Database| {
        let task = detail(database, &seed.task.id).task;
        TaskService::new(database).transition_task_status(TransitionTaskInput {
            task_id: task.id,
            expected_revision: task.revision,
            to_status: "done".to_owned(),
            reason: None,
        })
    };
    assert!(matches!(
        completion(&database),
        Err(CoreError::TaskNotCompletable(_))
    ));

    // 备份后修改当前数据库，确保恢复确实回到备份时点。
    ProjectService::new(&database)
        .rename(xushu_core::UpdateProjectInput {
            project_id: seed.project.id.clone(),
            name: "被修改的项目名称".to_owned(),
        })
        .expect("rename project");
    TaskService::new(&database)
        .create_task(CreateTaskInput {
            project_id: seed.project.id.clone(),
            title: "备份之后创建的任务".to_owned(),
            description: None,
            priority: "low".to_owned(),
        })
        .expect("create post-backup task");

    let outcome = open_preflight(&database, directory.path(), &record);
    let preflight = outcome.preflight;
    assert!(preflight.ready);
    assert_eq!(preflight.backup_id, record.backup_id);
    assert!(preflight.incoming.is_some());
    assert!(preflight
        .changes_since_backup
        .iter()
        .any(|change| change.label == "任务更新" && change.count >= 1));
    assert!(preflight
        .changes_since_backup
        .iter()
        .any(|change| change.label == "项目更新" && change.count >= 1));
    let plan = outcome.plan.expect("restore plan");
    let journal = switch_to_plan(database, directory.path(), &plan, "round-trip");
    let report = RestoreService::verify_restored_database(&directory.path().join("xushu.db"))
        .expect("verify restored");
    assert!(report.ok);
    assert!(report
        .checks
        .iter()
        .all(|check| check.status == xushu_core::IntegrityStatus::Pass));

    let reopened = Database::open(directory.path().join("xushu.db")).expect("reopen restored");
    assert!(matches!(
        completion(&reopened),
        Err(CoreError::TaskNotCompletable(_))
    ));
    RestoreService::record_operation(
        &reopened,
        "round-trip",
        &journal,
        "completed",
        true,
        "available",
    )
    .expect("record operation");
    RestoreService::finish_journal(directory.path(), &journal, "completed")
        .expect("finish journal");

    let connection = Connection::open(directory.path().join("xushu.db")).expect("read restored");
    assert_eq!(read_business_rows(&connection), snapshot_rows);
    assert_eq!(file_sha256(&package.join("manifest.json")), manifest_hash);
    assert_eq!(file_sha256(&package.join("xushu.db")), payload_hash);

    let restored_registry = BackupService::new(&reopened, directory.path());
    assert!(restored_registry.list().unwrap().is_empty());
    assert!(!restored_registry.policy().unwrap().automatic_enabled);
    assert!(restored_registry
        .policy()
        .unwrap()
        .target_directory
        .is_none());
    let external = open_preflight(&reopened, directory.path(), &record);
    assert!(external.preflight.ready, "{:?}", external.preflight.checks);
    assert!(restored_registry.list().unwrap().is_empty());
    RestoreService::cleanup_plan_workspace(&external.plan.unwrap());
    println!(
        "NONEMPTY_RESTORE_EVIDENCE {}",
        serde_json::json!({
            "backupId": record.backup_id,
            "manifestSha256": manifest_hash,
            "payloadSha256": payload_hash,
            "schemaVersion": report.schema_version,
            "registryBeforeRestore": registered.len(),
            "registryAfterRestore": 0,
            "unregisteredBackupPreflightReady": external.preflight.ready,
            "matchedRows": snapshot_rows,
        })
    );
    let project_name: String = connection
        .query_row(
            "SELECT name FROM projects WHERE id = ?1",
            [&seed.project.id],
            |row| row.get(0),
        )
        .expect("project name");
    assert_ne!(project_name, "被修改的项目名称");
    let post_backup_tasks: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM tasks WHERE title = '备份之后创建的任务'",
            [],
            |row| row.get(0),
        )
        .expect("post backup task count");
    assert_eq!(post_backup_tasks, 0);
    let task_status: String = connection
        .query_row(
            "SELECT status FROM tasks WHERE id = ?1",
            [&seed.task.id],
            |row| row.get(0),
        )
        .expect("task status");
    assert_eq!(task_status, "awaiting_verification");
    let memory_title: String = connection
        .query_row("SELECT title FROM project_memories LIMIT 1", [], |row| {
            row.get(0)
        })
        .expect("memory title");
    assert_eq!(memory_title, seed.memory_title);
    let memory_revisions: i64 = connection
        .query_row("SELECT COUNT(*) FROM project_memory_revisions", [], |row| {
            row.get(0)
        })
        .expect("memory revisions");
    assert_eq!(memory_revisions, 2);
    let decision_status: String = connection
        .query_row(
            "SELECT status FROM decisions WHERE supersedes_id IS NOT NULL",
            [],
            |row| row.get(0),
        )
        .expect("decision status");
    assert_eq!(decision_status, "confirmed");
    let decision_title: String = connection
        .query_row(
            "SELECT old.title FROM decisions new JOIN decisions old ON new.supersedes_id = old.id",
            [],
            |row| row.get(0),
        )
        .expect("decision title");
    assert_eq!(decision_title, seed.decision_title);
    let run_count: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM verification_runs WHERE id = ?1",
            [&seed.verification_run_id],
            |row| row.get(0),
        )
        .expect("run count");
    assert_eq!(run_count, 1);
    let evidence_count: i64 = connection
        .query_row("SELECT COUNT(*) FROM evidence", [], |row| row.get(0))
        .expect("evidence count");
    assert!(evidence_count >= 1);
    let snapshot_count: i64 = connection
        .query_row("SELECT COUNT(*) FROM recovery_snapshots", [], |row| {
            row.get(0)
        })
        .expect("snapshot count");
    assert_eq!(snapshot_count, 1);
    let claude_events: i64 = connection
        .query_row("SELECT COUNT(*) FROM claude_events", [], |row| row.get(0))
        .expect("claude events");
    assert_eq!(claude_events, 1);
    let integration_enabled: i64 = connection
        .query_row(
            "SELECT desired_enabled FROM claude_integrations WHERE project_id = ?1",
            [&seed.project.id],
            |row| row.get(0),
        )
        .expect("integration");
    assert_eq!(integration_enabled, 1);
    let restore_records: i64 = connection
        .query_row("SELECT COUNT(*) FROM restore_operations", [], |row| {
            row.get(0)
        })
        .expect("restore operations");
    assert_eq!(restore_records, 1);
    assert_eq!(
        fs::read(spool.join("fixture.json")).unwrap(),
        b"pending fixture envelope"
    );
    assert_eq!(
        fs::read(salt_path).unwrap(),
        b"fixture salt must stay outside the backup"
    );
    assert!(!repo.join(".claude").exists());

    // 回退副本仍然保留，且备份目录中只有清单与数据库文件。
    assert!(Path::new(&journal.rollback_directory()).is_dir());
    let backup_entries: Vec<String> = fs::read_dir(&record.directory_path)
        .expect("backup directory")
        .flatten()
        .map(|entry| entry.file_name().to_string_lossy().to_string())
        .collect();
    let mut sorted = backup_entries.clone();
    sorted.sort();
    assert_eq!(
        sorted,
        vec!["manifest.json".to_owned(), "xushu.db".to_owned()]
    );

    RestoreService::cleanup_plan_workspace(&plan);
    drop(reopened);
    drop(connection);
    drop(directory);
}

fn read_business_rows(connection: &Connection) -> Value {
    let queries = [
        ("projects", "SELECT id, name FROM projects ORDER BY id"),
        (
            "tasks",
            "SELECT id, project_id, title, status FROM tasks ORDER BY id",
        ),
        (
            "criteria",
            "SELECT id, task_id, description, kind FROM acceptance_criteria ORDER BY id",
        ),
        (
            "verificationRuns",
            "SELECT id, task_id, criterion_id, execution_status FROM verification_runs ORDER BY id",
        ),
        (
            "evidence",
            "SELECT id, task_id, run_id, source_type, content_digest FROM evidence ORDER BY id",
        ),
        (
            "memories",
            "SELECT id, project_id, title, content FROM project_memories ORDER BY id",
        ),
        (
            "decisions",
            "SELECT id, project_id, title, status FROM decisions ORDER BY id",
        ),
    ];
    let mut output = serde_json::Map::new();
    for (name, query) in queries {
        let mut statement = connection.prepare(query).unwrap();
        let columns = statement.column_count();
        let rows = statement
            .query_map([], |row| {
                (0..columns)
                    .map(|index| row.get::<_, Option<String>>(index))
                    .collect::<rusqlite::Result<Vec<_>>>()
            })
            .unwrap()
            .collect::<rusqlite::Result<Vec<_>>>()
            .unwrap();
        assert!(!rows.is_empty(), "fixture has no {name}");
        output.insert(name.to_owned(), serde_json::json!(rows));
    }
    Value::Object(output)
}

#[test]
fn tampered_payload_is_rejected_and_current_database_is_untouched() {
    let (_repo_root, repo) = init_repo();
    let (directory, database) = temp_database();
    let seed = seed(&database, &repo);
    let record = create_backup(&database, directory.path());
    tamper_payload(&record, b"tampered-sqlite-payload");

    let before = TaskService::new(&database)
        .list_tasks(TaskFilter {
            project_id: Some(seed.project.id.clone()),
            status: None,
            search: None,
            include_archived: None,
        })
        .expect("tasks before")
        .len();
    let error = RestoreService::new(&database, directory.path())
        .preflight(RestorePreflightInput {
            backup_directory: record.directory_path.clone(),
        })
        .expect_err("tampered payload must be refused");
    assert!(matches!(
        error,
        CoreError::Backup(_) | CoreError::Compatibility(_) | CoreError::Validation(_)
    ));
    let after = TaskService::new(&database)
        .list_tasks(TaskFilter {
            project_id: Some(seed.project.id.clone()),
            status: None,
            search: None,
            include_archived: None,
        })
        .expect("tasks after")
        .len();
    assert_eq!(before, after);
    assert!(!directory.path().join("restore-journal.json").exists());
    assert!(!directory.path().join("restore-rollback").exists());
}

#[test]
fn future_schema_backup_is_blocked_without_touching_current_data() {
    let (_repo_root, repo) = init_repo();
    let (directory, database) = temp_database();
    let seed = seed(&database, &repo);
    let record = create_backup(&database, directory.path());
    rewrite_manifest(&record, |manifest| {
        manifest["schemaVersion"] = Value::from(999);
    });

    let outcome = open_preflight(&database, directory.path(), &record);
    assert!(!outcome.preflight.ready);
    assert!(outcome.plan.is_none());
    assert!(outcome
        .preflight
        .checks
        .iter()
        .any(|check| check.id == "schema_compatibility"
            && check.status == xushu_core::IntegrityStatus::Fail));
    let tasks = TaskService::new(&database)
        .list_tasks(TaskFilter {
            project_id: Some(seed.project.id.clone()),
            status: None,
            search: None,
            include_archived: None,
        })
        .expect("tasks");
    assert_eq!(tasks.len(), 1);
}

#[test]
fn corrupted_database_with_matching_hash_is_rejected_by_isolated_restore() {
    let (_repo_root, repo) = init_repo();
    let (directory, database) = temp_database();
    let _seed = seed(&database, &repo);
    let record = create_backup(&database, directory.path());
    let corrupted = b"this is not a valid sqlite database file";
    tamper_payload(&record, corrupted);
    let payload = Path::new(&record.directory_path).join("xushu.db");
    let digest = file_sha256(&payload);
    rewrite_manifest(&record, |manifest| {
        manifest["files"][0]["size"] = Value::from(corrupted.len() as u64);
        manifest["files"][0]["sha256"] = Value::from(digest.clone());
    });
    Connection::open(database.path())
        .unwrap()
        .execute(
            "UPDATE backup_records SET payload_sha256 = ?1 WHERE backup_id = ?2",
            params![digest, record.backup_id],
        )
        .unwrap();

    let verification = BackupService::new(&database, directory.path())
        .verify(&record.backup_id)
        .unwrap();
    assert!(!verification.verified);
    assert!(verification.checks.iter().any(|check| {
        check.id == "sqlite_integrity"
            && check.status == xushu_core::IntegrityStatus::Fail
            && check
                .detail
                .as_deref()
                .is_some_and(|detail| detail.contains("NotADatabase") && detail.contains("26"))
    }));

    let outcome = open_preflight(&database, directory.path(), &record);
    assert!(!outcome.preflight.ready);
    assert!(outcome.plan.is_none());
    assert!(outcome
        .preflight
        .checks
        .iter()
        .any(|check| check.id == "isolated_restore"
            && check.status == xushu_core::IntegrityStatus::Fail
            && check
                .detail
                .as_deref()
                .is_some_and(|detail| detail.contains("NotADatabase"))));
    assert!(!directory.path().join("restore-journal.json").exists());
}

#[test]
fn schema_v7_backup_with_other_platform_line_endings_can_be_reverified_and_restored() {
    let directory = TempDir::new().expect("temp directory");
    let path = directory.path().join("xushu.db");
    {
        let mut connection = Connection::open(&path).unwrap();
        xushu_core::run_migrations(&mut connection, &[]).unwrap();
        for migration in xushu_core::MIGRATIONS {
            let sql = migration.sql.replace("\r\n", "\n");
            let sql = if sql == migration.sql {
                sql.replace('\n', "\r\n")
            } else {
                sql
            };
            let transaction = connection.transaction().unwrap();
            transaction.execute_batch(&sql).unwrap();
            transaction
                .execute(
                    "INSERT INTO schema_migrations (version, name, applied_at) VALUES (?1, ?2, ?3)",
                    params![migration.version, migration.name, "2026-11-15T00:00:00Z"],
                )
                .unwrap();
            transaction
                .pragma_update(None, "user_version", migration.version)
                .unwrap();
            transaction.commit().unwrap();
        }
    }
    let database = Database::open(&path).unwrap();
    let record = create_backup(&database, directory.path());
    let package = Path::new(&record.directory_path);
    let manifest_path = package.join("manifest.json");
    let payload_path = package.join("xushu.db");
    let manifest_hash = file_sha256(&manifest_path);
    let payload_hash = file_sha256(&payload_path);
    assert_eq!(record.schema_version, 7);
    assert_eq!(record.payload_sha256, payload_hash);
    drop(database);

    let reopened = Database::open(&path).unwrap();
    let verification = BackupService::new(&reopened, directory.path())
        .verify(&record.backup_id)
        .unwrap();
    assert!(verification.verified, "{:?}", verification.checks);
    assert!(verification.integrity.unwrap().ok);
    let outcome = open_preflight(&reopened, directory.path(), &record);
    assert!(outcome.preflight.ready, "{:?}", outcome.preflight.checks);
    assert_eq!(outcome.preflight.manifest_sha256, manifest_hash);
    let plan = outcome.plan.unwrap();
    let _journal = switch_to_plan(reopened, directory.path(), &plan, "alternate-eol");
    let report = RestoreService::verify_restored_database(&path).unwrap();
    assert!(report.ok, "{:?}", report.checks);
    assert_eq!(report.schema_version, 7);
    assert!(Database::open(&path).unwrap().health().unwrap().ok);
    assert_eq!(file_sha256(&manifest_path), manifest_hash);
    assert_eq!(file_sha256(&payload_path), payload_hash);
}

#[test]
fn backup_with_unexpected_schema_is_never_published_or_registered() {
    let (directory, database) = temp_database();
    Connection::open(database.path())
        .unwrap()
        .execute_batch("CREATE VIEW fixture_unexpected_view AS SELECT 1;")
        .unwrap();
    assert!(database.health().unwrap().ok);
    let target = directory.path().join("incompatible-backups");
    let service = BackupService::new(&database, directory.path());
    let error = service
        .create_manual(&target.to_string_lossy(), &AtomicBool::new(false))
        .expect_err("backup should use the same schema gate as re-verification");
    assert_eq!(error.code(), "BACKUP_INCOMPATIBLE");
    assert!(service.list().unwrap().is_empty());
    assert!(fs::read_dir(target).unwrap().next().is_none());
}

#[test]
fn preflight_reports_missing_project_roots_and_hook_review_from_backup() {
    let (repo_root, repo) = init_repo();
    let (directory, database) = temp_database();
    let _seed = seed(&database, &repo);
    let record = create_backup(&database, directory.path());
    let moved_repo = repo_root.path().join("moved-repo");
    fs::rename(&repo, &moved_repo).expect("move fixture repository");
    assert!(
        !repo.exists(),
        "preflight fixture still points to an existing directory"
    );

    let outcome = open_preflight(&database, directory.path(), &record);
    let preflight = outcome.preflight;
    assert!(preflight.ready);
    let incoming = preflight.incoming.expect("incoming summary");
    assert_eq!(incoming.missing_project_root_count, 1);
    assert_eq!(incoming.desired_hook_project_count, 1);
    assert_eq!(preflight.hooks_review.project_count, 1);
    assert!(!preflight.hooks_review.settings_files_modified);
    assert!(!preflight.hooks_review.salt_included);
    RestoreService::cleanup_plan_workspace(&outcome.plan.expect("plan"));
}

#[test]
fn policy_cleanup_removes_only_registered_automatic_backups() {
    let (_repo_root, repo) = init_repo();
    let (directory, database) = temp_database();
    let _seed = seed(&database, &repo);
    let target = directory.path().join("backups");
    let manual = BackupService::new(&database, directory.path())
        .create_manual(&target.to_string_lossy(), &AtomicBool::new(false))
        .expect("manual backup");
    BackupService::new(&database, directory.path())
        .update_policy(xushu_core::UpdateBackupPolicyInput {
            automatic_enabled: true,
            target_directory: Some(target.to_string_lossy().to_string()),
            interval_hours: 1,
            retention_count: 5,
        })
        .expect("enable policy");
    let cancel = AtomicBool::new(false);
    let first = BackupService::new(&database, directory.path())
        .run_due_automatic(&cancel)
        .expect("first automatic");
    assert_eq!(first.status, "created");
    reset_last_automatic(&database);
    let second = BackupService::new(&database, directory.path())
        .run_due_automatic(&cancel)
        .expect("second automatic");
    assert_eq!(second.status, "created");
    let auto_records: Vec<_> = BackupService::new(&database, directory.path())
        .list()
        .expect("list")
        .into_iter()
        .filter(|record| record.origin == BackupOrigin::Automatic)
        .collect();
    assert_eq!(auto_records.len(), 2);

    BackupService::new(&database, directory.path())
        .update_policy(xushu_core::UpdateBackupPolicyInput {
            automatic_enabled: true,
            target_directory: Some(target.to_string_lossy().to_string()),
            interval_hours: 1,
            retention_count: 1,
        })
        .expect("shrink retention");
    let preview = BackupService::new(&database, directory.path())
        .cleanup_preview()
        .expect("preview");
    assert_eq!(preview.len(), 1);
    let removed_directory = preview[0].directory_path.clone();
    let outcome = BackupService::new(&database, directory.path())
        .apply_cleanup(true)
        .expect("apply cleanup");
    assert_eq!(outcome.removed_backup_ids.len(), 1);
    assert!(!Path::new(&removed_directory).exists());
    let remaining = BackupService::new(&database, directory.path())
        .list()
        .expect("list after cleanup");
    assert!(remaining
        .iter()
        .any(|record| record.backup_id == manual.backup_id));
    assert_eq!(
        remaining
            .iter()
            .filter(|record| record.origin == BackupOrigin::Automatic)
            .count(),
        1
    );
}

fn reset_last_automatic(database: &Database) {
    let connection = Connection::open(database.path()).expect("raw database");
    connection
        .execute(
            "UPDATE backup_settings SET last_automatic_at = NULL WHERE id = 1",
            [],
        )
        .expect("reset automatic timestamp");
}

#[test]
fn cancelled_backup_leaves_no_valid_backup_directory() {
    let (_repo_root, repo) = init_repo();
    let (directory, database) = temp_database();
    let _seed = seed(&database, &repo);
    let target = directory.path().join("cancelled-backups");
    let cancelled = AtomicBool::new(true);
    let error = BackupService::new(&database, directory.path())
        .create_manual(&target.to_string_lossy(), &cancelled)
        .expect_err("backup must be cancelled");
    assert!(matches!(error, CoreError::BackupCancelled));
    if target.exists() {
        let entries = fs::read_dir(&target)
            .expect("target directory")
            .flatten()
            .count();
        assert_eq!(entries, 0, "cancelled backup left a directory");
    }
}

#[test]
fn backup_with_invalid_business_relations_is_never_published_or_registered() {
    let (_repo_root, repo) = init_repo();
    let (directory, database) = temp_database();
    let seed = seed(&database, &repo);
    let other = Uuid::new_v4().to_string();
    let connection = Connection::open(database.path()).unwrap();
    connection
        .pragma_update(None, "foreign_keys", false)
        .unwrap();
    connection
        .execute(
            "UPDATE tasks SET project_id = ?1 WHERE id = ?2",
            params![other, seed.task.id],
        )
        .unwrap();

    let target = directory.path().join("invalid-backups");
    let service = BackupService::new(&database, directory.path());
    assert!(service
        .create_manual(&target.to_string_lossy(), &AtomicBool::new(false))
        .is_err());
    assert!(service.list().unwrap().is_empty());
    assert!(fs::read_dir(target).unwrap().next().is_none());
    assert_eq!(detail(&database, &seed.task.id).task.project_id, other);
}

#[test]
fn concurrent_writes_do_not_corrupt_the_backup_snapshot() {
    let (_repo_root, repo) = init_repo();
    let (directory, database) = temp_database();
    let _seed = seed(&database, &repo);
    let before = database.health().expect("live database health");
    assert_eq!(before.journal_mode, "wal");
    let stop = Arc::new(AtomicBool::new(false));
    let writer_stop = stop.clone();
    let path = database.path().to_path_buf();
    let (committed, first_commit) = std::sync::mpsc::sync_channel(1);
    let writer = thread::spawn(move || {
        let mut connection = Connection::open(&path).expect("writer connection");
        connection
            .busy_timeout(Duration::from_secs(5))
            .expect("busy timeout");
        let mut index = 0_u32;
        while !writer_stop.load(Ordering::Relaxed) && index < 150 {
            let transaction = connection.transaction().unwrap();
            for side in ["left", "right"] {
                transaction
                    .execute(
                        "INSERT INTO persistence_probes (id, label, content, created_at)
                     VALUES (?1, ?2, ?3, '2026-11-13T10:00:00.000Z')",
                        params![
                            Uuid::new_v4().to_string(),
                            format!("concurrent-{side}"),
                            index.to_string()
                        ],
                    )
                    .unwrap();
            }
            transaction.commit().unwrap();
            if index == 0 {
                committed.send(()).unwrap();
            }
            index += 1;
            thread::sleep(Duration::from_millis(1));
        }
        index
    });
    first_commit.recv_timeout(Duration::from_secs(5)).unwrap();
    assert!(
        fs::metadata(database.path().with_file_name("xushu.db-wal"))
            .unwrap()
            .len()
            > 0
    );
    let record = create_backup(&database, directory.path());
    stop.store(true, Ordering::Relaxed);
    let commits = writer.join().expect("writer thread");
    let snapshot = Connection::open_with_flags(
        Path::new(&record.directory_path).join("xushu.db"),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let (transactions, split): (i64, i64) = snapshot.query_row(
        "SELECT COUNT(*), COALESCE(SUM(left_count != 1 OR right_count != 1), 0) FROM (
            SELECT SUM(label = 'concurrent-left') AS left_count, SUM(label = 'concurrent-right') AS right_count
            FROM persistence_probes WHERE label IN ('concurrent-left', 'concurrent-right') GROUP BY content
        )", [], |row| Ok((row.get(0)?, row.get(1)?))
    ).unwrap();
    assert!((1..=i64::from(commits)).contains(&transactions));
    assert_eq!(split, 0);
    drop(snapshot);

    let verification = BackupService::new(&database, directory.path())
        .verify(&record.backup_id)
        .expect("verify backup");
    assert!(verification.verified, "{}", verification.message);
    let report = verification.integrity.expect("integrity report");
    assert!(report.ok);
    assert!(report.counts.projects >= 1);
}

#[cfg(windows)]
#[test]
fn exclusively_open_backup_payload_is_rejected_without_touching_live_database() {
    use std::os::windows::fs::OpenOptionsExt;

    let (directory, database) = temp_database();
    let backup = create_backup(&database, directory.path());
    let live_sha = file_sha256(database.path());
    let payload = Path::new(&backup.directory_path).join("xushu.db");
    let open_file = fs::OpenOptions::new()
        .read(true)
        .share_mode(0)
        .open(payload)
        .unwrap();
    let restore = RestoreService::new(&database, directory.path());
    let input = || RestorePreflightInput {
        backup_directory: backup.directory_path.clone(),
    };
    assert!(restore.preflight(input()).is_err());
    assert_eq!(file_sha256(database.path()), live_sha);
    assert!(!directory.path().join("restore-journal.json").exists());
    drop(open_file);
    let ready = restore.preflight(input()).unwrap();
    assert!(ready.preflight.ready);
    RestoreService::cleanup_plan_workspace(&ready.plan.unwrap());
}

#[test]
fn rollback_restores_the_original_database_after_a_failed_switch() {
    let (_repo_root, repo) = init_repo();
    let (directory, database) = temp_database();
    let seed = seed(&database, &repo);
    let record = create_backup(&database, directory.path());
    TaskService::new(&database)
        .create_task(CreateTaskInput {
            project_id: seed.project.id.clone(),
            title: "只存在于当前数据库的任务".to_owned(),
            description: None,
            priority: "medium".to_owned(),
        })
        .expect("create current-only task");

    let outcome = open_preflight(&database, directory.path(), &record);
    let plan = outcome.plan.expect("plan");
    let journal = switch_to_plan(database, directory.path(), &plan, "rollback-test");
    let database_path = directory.path().join("xushu.db");
    assert_eq!(
        RestoreService::verify_restored_database(&database_path)
            .expect("restored report")
            .counts
            .tasks,
        1
    );
    // 模拟切换后最终校验失败：破坏当前数据库文件，再回退。
    fs::write(&database_path, b"broken after switch").expect("break restored database");
    let rollback = RestoreService::rollback_switch(directory.path(), &journal);
    assert!(rollback.verified, "{}", rollback.message);
    let reopened = Database::open(&database_path).expect("reopen original");
    let tasks = TaskService::new(&reopened)
        .list_tasks(TaskFilter {
            project_id: Some(seed.project.id.clone()),
            status: None,
            search: Some("只存在于当前数据库的任务".to_owned()),
            include_archived: None,
        })
        .expect("tasks after rollback");
    assert_eq!(tasks.len(), 1);
    RestoreService::finish_journal(directory.path(), &journal, "rolled_back")
        .expect("finish rollback journal");
    RestoreService::cleanup_plan_workspace(&plan);
}

#[test]
fn startup_recovery_rolls_back_an_interrupted_switch() {
    let (_repo_root, repo) = init_repo();
    let (directory, database) = temp_database();
    let seed = seed(&database, &repo);
    let record = create_backup(&database, directory.path());
    TaskService::new(&database)
        .create_task(CreateTaskInput {
            project_id: seed.project.id.clone(),
            title: "中断切换前只存在于当前数据库的任务".to_owned(),
            description: None,
            priority: "medium".to_owned(),
        })
        .expect("create current-only task");
    let outcome = open_preflight(&database, directory.path(), &record);
    let plan = outcome.plan.expect("plan");
    let journal = switch_to_plan(database, directory.path(), &plan, "startup-recovery");
    assert!(!journal.rollback_directory().is_empty());

    let notice = xushu_core::backup::recover_interrupted_restore(directory.path())
        .expect("startup recovery")
        .expect("notice");
    assert_eq!(notice.outcome, "rolled_back");
    assert!(!directory.path().join("restore-journal.json").exists());
    let recovered = Database::open(directory.path().join("xushu.db")).expect("reopen recovered");
    let tasks = TaskService::new(&recovered)
        .list_tasks(TaskFilter {
            project_id: Some(seed.project.id.clone()),
            status: None,
            search: Some("中断切换前只存在于当前数据库的任务".to_owned()),
            include_archived: None,
        })
        .expect("tasks after startup recovery");
    assert_eq!(tasks.len(), 1);
    RestoreService::cleanup_plan_workspace(&plan);
}

#[test]
fn failed_rollback_preserves_the_current_file_when_recovery_sources_are_unavailable() {
    let (directory, database) = temp_database();
    let operation_id = Uuid::new_v4().to_string();
    let journal = RestoreService::new(&database, directory.path())
        .create_rollback(&operation_id)
        .expect("create rollback");
    drop(database);
    let current = directory.path().join("xushu.db");
    let digest = file_sha256(&current);
    fs::remove_file(Path::new(&journal.rollback_directory()).join("rollback-xushu.db"))
        .expect("simulate unavailable rollback snapshot");

    let report = RestoreService::rollback_switch(directory.path(), &journal);
    assert_eq!(report.status, "failed");
    assert!(
        current.is_file(),
        "failed rollback moved the current database"
    );
    assert_eq!(file_sha256(&current), digest);
    assert!(directory.path().join("restore-journal.json").is_file());
}

#[test]
fn preflight_cannot_use_an_unpublished_partial_backup() {
    let (directory, database) = temp_database();
    let record = create_backup(&database, directory.path());
    let original = Path::new(&record.directory_path);
    let partial = original.with_file_name(format!(".partial-{}", record.backup_id));
    fs::rename(original, &partial).expect("simulate interruption before publication");

    let result =
        RestoreService::new(&database, directory.path()).preflight(RestorePreflightInput {
            backup_directory: partial.to_string_lossy().into_owned(),
        });
    assert!(result.is_err());
    assert!(!directory.path().join("restore-journal.json").exists());
}

#[test]
fn payload_schema_must_match_the_manifest_even_if_both_versions_are_supported() {
    let (directory, database) = temp_database();
    let record = create_backup(&database, directory.path());
    rewrite_manifest(&record, |manifest| {
        manifest["schemaVersion"] = Value::from(6)
    });

    let outcome = open_preflight(&database, directory.path(), &record);
    assert!(!outcome.preflight.ready);
    assert!(outcome.plan.is_none());
}

#[test]
fn backup_with_unexpected_schema_objects_cannot_be_restored() {
    let (directory, database) = temp_database();
    let record = create_backup(&database, directory.path());
    let payload = Path::new(&record.directory_path).join("xushu.db");
    {
        let fixture = Connection::open(&payload).unwrap();
        fixture
            .execute_batch("CREATE VIEW fixture_unexpected_view AS SELECT 1;")
            .unwrap();
    }
    let digest = file_sha256(&payload);
    let size = fs::metadata(&payload).unwrap().len();
    rewrite_manifest(&record, |manifest| {
        manifest["files"][0]["sha256"] = Value::from(digest);
        manifest["files"][0]["size"] = Value::from(size);
    });
    let outcome = open_preflight(&database, directory.path(), &record);
    assert!(!outcome.preflight.ready);
    assert!(outcome.plan.is_none());
}

#[test]
fn registered_backup_with_bad_foreign_keys_never_shows_a_passing_integrity_check() {
    let (_repo_root, repo) = init_repo();
    let (directory, database) = temp_database();
    let seed = seed(&database, &repo);
    let record = create_backup(&database, directory.path());
    let payload = Path::new(&record.directory_path).join("xushu.db");
    {
        let connection = Connection::open(&payload).expect("open fixture payload");
        connection
            .pragma_update(None, "foreign_keys", false)
            .unwrap();
        connection
            .execute(
                "UPDATE tasks SET project_id = ?1 WHERE id = ?2",
                params![Uuid::new_v4().to_string(), seed.task.id],
            )
            .unwrap();
        assert_eq!(
            connection
                .query_row("PRAGMA integrity_check", [], |row| row.get::<_, String>(0))
                .unwrap(),
            "ok"
        );
    }
    let digest = file_sha256(&payload);
    let size = fs::metadata(&payload).unwrap().len();
    rewrite_manifest(&record, |manifest| {
        manifest["files"][0]["sha256"] = Value::from(digest.clone());
        manifest["files"][0]["size"] = Value::from(size);
    });
    let verification = BackupService::new(&database, directory.path())
        .verify(&record.backup_id)
        .unwrap();
    assert!(!verification.verified);
    assert!(verification.checks.iter().any(|check| {
        check.id == "sqlite_integrity" && check.status == xushu_core::IntegrityStatus::Fail
    }));
    let preflight = open_preflight(&database, directory.path(), &record);
    assert!(preflight.plan.is_none());
    assert_eq!(
        detail(&database, &seed.task.id).task.project_id,
        seed.project.id
    );
}

#[test]
fn restore_is_refused_while_a_verification_run_is_running() {
    let (_repo_root, repo) = init_repo();
    let (directory, database) = temp_database();
    let seed = seed(&database, &repo);
    let connection = Connection::open(database.path()).expect("raw database");
    connection
        .execute(
            "UPDATE verification_runs SET execution_status = 'running' WHERE id = ?1",
            [&seed.verification_run_id],
        )
        .expect("mark running");
    let error = RestoreService::new(&database, directory.path())
        .assert_no_running_verification()
        .expect_err("running verification must block restore");
    assert!(matches!(error, CoreError::Conflict(_)));
}

#[cfg(windows)]
#[test]
fn junction_backup_target_is_refused_when_environment_allows_creation() {
    let (_repo_root, repo) = init_repo();
    let (directory, database) = temp_database();
    let _seed = seed(&database, &repo);
    let fixture = directory.path().join("junction-fixture");
    fs::create_dir(&fixture).expect("create fixture");
    let real = fixture.join("real");
    fs::create_dir(&real).expect("create real directory");
    fs::create_dir(real.join("child")).expect("create child");
    let link = fixture.join("link");
    let status = Command::new("cmd")
        .args(["/C", "mklink", "/J"])
        .arg(&link)
        .arg(&real)
        .status();
    let created = status.map(|status| status.success()).unwrap_or(false);
    if !created {
        assert!(
            std::env::var_os("XUSHU_REQUIRE_JUNCTION_TEST").is_none(),
            "Windows must create a real fixture junction for this retest"
        );
        eprintln!(
            "ENVIRONMENT_BLOCKED: Windows refused to create a junction; product assertion NOT_RUN"
        );
        return;
    }

    let direct = BackupService::new(&database, directory.path())
        .create_manual(&link.to_string_lossy(), &AtomicBool::new(false))
        .expect_err("junction target must be refused");
    assert!(matches!(direct, CoreError::UnsafePath));
    let nested = link.join("child");
    let ancestor = BackupService::new(&database, directory.path())
        .create_manual(&nested.to_string_lossy(), &AtomicBool::new(false))
        .expect_err("junction ancestor must be refused");
    assert!(matches!(ancestor, CoreError::UnsafePath));
    assert!(!directory.path().join("restore-journal.json").exists());
}

#[test]
fn older_schema_backup_is_upgraded_in_isolation_and_can_be_restored() {
    let (_repo_root, repo) = init_repo();
    let (directory, database) = temp_database();
    let _seed = seed(&database, &repo);

    let legacy_root = TempDir::new().expect("legacy root");
    let legacy_path = legacy_root.path().join("legacy.db");
    {
        let mut connection = Connection::open(&legacy_path).expect("legacy database");
        xushu_core::run_migrations(&mut connection, &xushu_core::MIGRATIONS[..6])
            .expect("apply schema 6");
        connection
            .execute(
                "INSERT INTO projects (id, name, root_path, root_path_key, created_at, updated_at)
                 VALUES ('legacy-project', '旧版本项目', 'C:\\legacy', 'c:\\legacy',
                         '2026-01-01T00:00:00.000Z', '2026-01-01T00:00:00.000Z')",
                [],
            )
            .expect("insert legacy project");
        connection
            .execute(
                "INSERT INTO tasks (id, project_id, title, description, priority, status, revision,
                    block_reason, archived_at, created_at, updated_at)
                 VALUES ('legacy-task', 'legacy-project', '旧版本任务', '', 'medium', 'draft', 1,
                         NULL, NULL, '2026-01-01T00:00:00.000Z', '2026-01-01T00:00:00.000Z')",
                [],
            )
            .expect("insert legacy task");
        let mode: String = connection
            .query_row("PRAGMA journal_mode = DELETE", [], |row| row.get(0))
            .expect("delete journal");
        assert_eq!(mode.to_ascii_lowercase(), "delete");
    }

    let payload = fs::read(&legacy_path).expect("read legacy payload");
    let mut hasher = Sha256::new();
    hasher.update(&payload);
    let digest = format!("{:x}", hasher.finalize());
    let backup_id = Uuid::new_v4().to_string();
    let package_directory = directory
        .path()
        .join(format!("legacy-backup-{}", &backup_id[..8]));
    fs::create_dir(&package_directory).expect("create package");
    fs::copy(&legacy_path, package_directory.join("xushu.db")).expect("copy legacy payload");
    let manifest = xushu_core::BackupManifest {
        format_version: 1,
        product: "XUSHU".to_owned(),
        backup_id: backup_id.clone(),
        app_version: "0.1.0".to_owned(),
        schema_version: 6,
        created_at: "2026-11-13T10:00:00Z".to_owned(),
        origin: BackupOrigin::Manual,
        scope: "database".to_owned(),
        encrypted: false,
        complete: true,
        files: vec![xushu_core::BackupFileEntry {
            path: "xushu.db".to_owned(),
            size: payload.len() as u64,
            sha256: digest,
        }],
    };
    fs::write(
        package_directory.join("manifest.json"),
        serde_json::to_vec_pretty(&manifest).expect("serialize manifest"),
    )
    .expect("write manifest");

    let outcome = RestoreService::new(&database, directory.path())
        .preflight(RestorePreflightInput {
            backup_directory: package_directory.to_string_lossy().to_string(),
        })
        .expect("preflight legacy backup");
    assert!(
        outcome.preflight.ready,
        "{:?}",
        outcome.preflight.blocking_reason
    );
    assert_eq!(outcome.preflight.schema_action, "upgrade");
    assert_eq!(outcome.preflight.backup_schema_version, 6);
    let incoming = outcome.preflight.incoming.expect("incoming");
    assert_eq!(incoming.projects, 1);
    assert_eq!(incoming.tasks, 1);
    let plan = outcome.plan.expect("legacy plan");
    assert_eq!(plan.backup_schema_version, 6);
    RestoreService::cleanup_plan_workspace(&plan);
}

#[test]
fn data_integrity_check_reports_current_schema_and_counts() {
    let (_repo_root, repo) = init_repo();
    let (_directory, database) = temp_database();
    let _seed = seed(&database, &repo);
    let report = DataIntegrityService::new(&database).check().expect("check");
    assert!(report.ok);
    assert_eq!(report.schema_version, 7);
    assert_eq!(report.counts.projects, 1);
    assert_eq!(report.counts.tasks, 1);
    assert!(report.counts.verification_runs >= 1);
    assert!(report.counts.evidence >= 1);
    assert_eq!(report.counts.project_memories, 1);
    assert_eq!(report.counts.decisions, 2);
    assert_eq!(
        report
            .checks
            .iter()
            .find(|check| check.id == "decision_supersede_links")
            .expect("decision check")
            .status,
        xushu_core::IntegrityStatus::Pass
    );
}

#[test]
fn backup_target_can_be_created_but_parent_must_exist() {
    let (_repo_root, repo) = init_repo();
    let (directory, database) = temp_database();
    let _seed = seed(&database, &repo);
    let nested_missing_parent = directory.path().join("missing").join("child");
    let error = BackupService::new(&database, directory.path())
        .create_manual(
            &nested_missing_parent.to_string_lossy(),
            &AtomicBool::new(false),
        )
        .expect_err("missing parent must be refused");
    assert!(matches!(error, CoreError::Io(_) | CoreError::UnsafePath));
    let single_missing = directory.path().join("new-backup-root");
    let record = BackupService::new(&database, directory.path())
        .create_manual(&single_missing.to_string_lossy(), &AtomicBool::new(false))
        .expect("create target then backup");
    assert!(single_missing.is_dir());
    assert!(Path::new(&record.directory_path).is_dir());
}
