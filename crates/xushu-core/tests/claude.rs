use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::json;
use tempfile::TempDir;
use xushu_core::{
    ClaudeActionInput, ClaudeInstallState, ClaudeIntegrationService, ClaudeRuntimePaths,
    CreateTaskInput, Database, ImportProjectInput, InstallClaudeHooksInput, Project,
    ProjectService, TaskService,
};
use xushu_hook_protocol::{
    decode_salt, filter_hook_input, FilterContext, HookEnvelope, SALT_FILE_NAME,
};

const OTHER_PROJECT_ID: &str = "99999999-9999-4999-8999-999999999999";
static SPOOL_ORDER: AtomicU64 = AtomicU64::new(0);

struct Fixture {
    directory: TempDir,
    data_directory: PathBuf,
    database: &'static Database,
    project: Project,
    service: ClaudeIntegrationService<'static>,
}

// 用泄漏的引用把 service 的生命周期简化到测试结束；每个测试进程独立，测试结束即退出。
fn fixture() -> Fixture {
    fixture_in(TempDir::new().expect("temp dir"))
}

fn isolated_fixture() -> Fixture {
    let target = Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .unwrap()
        .join("target");
    fs::create_dir_all(&target).unwrap();
    fixture_in(TempDir::new_in(target).expect("isolated fixture directory"))
}

fn fixture_in(directory: TempDir) -> Fixture {
    let data_directory = directory.path().join("app data 中文");
    let database = Database::open(data_directory.join("xushu.db")).expect("database");

    let project_path = directory.path().join("项目 & spaces").join("repo");
    fs::create_dir_all(&project_path).expect("project dir");
    run_git(&project_path, &["init", "-b", "main"]);
    run_git(&project_path, &["config", "user.email", "test@xushu.local"]);
    run_git(&project_path, &["config", "user.name", "XUSHU Test"]);
    fs::write(project_path.join("README.md"), "test\n").expect("readme");
    run_git(&project_path, &["add", "-A"]);
    run_git(&project_path, &["commit", "-m", "initial"]);

    let project = ProjectService::new(&database)
        .import(ImportProjectInput {
            root_path: project_path.to_string_lossy().to_string(),
            name: None,
        })
        .expect("import project");

    let binary = data_directory.join("bin").join("xushu-hook.exe");
    fs::create_dir_all(binary.parent().expect("binary parent")).expect("bin dir");
    fs::write(&binary, b"fake hook binary").expect("fake binary");
    let runtime =
        ClaudeRuntimePaths::prepare(&data_directory, Some(binary), "0.1.0").expect("runtime paths");

    let database: &'static Database = Box::leak(Box::new(database));
    let runtime: &'static ClaudeRuntimePaths = Box::leak(Box::new(runtime));
    let service = ClaudeIntegrationService::new(database, runtime);

    Fixture {
        directory,
        data_directory,
        database,
        project,
        service,
    }
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

impl Fixture {
    fn action(&self) -> ClaudeActionInput {
        ClaudeActionInput {
            project_id: self.project.id.clone(),
            approved: true,
        }
    }

    fn settings_path(&self) -> PathBuf {
        Path::new(&self.project.root_path)
            .join(".claude")
            .join("settings.local.json")
    }

    fn spool_dir(&self) -> PathBuf {
        self.data_directory
            .join("hook-inbox")
            .join("pending")
            .join(&self.project.id)
    }

    fn salt(&self) -> Vec<u8> {
        let content =
            fs::read_to_string(self.data_directory.join("hook-inbox").join(SALT_FILE_NAME))
                .expect("read salt");
        decode_salt(content.trim()).expect("valid salt")
    }

    fn preview(&self) -> xushu_core::ClaudeHookPreview {
        self.service.preview(&self.project.id).expect("preview")
    }

    fn install(&self) -> xushu_core::ClaudeIntegrationStatus {
        let preview = self.preview();
        assert!(
            preview.blocked_reason.is_none(),
            "preview blocked: {:?}",
            preview.blocked_reason
        );
        self.service
            .install(InstallClaudeHooksInput {
                project_id: self.project.id.clone(),
                preview_token: preview.preview_token,
                approved: true,
            })
            .expect("install")
    }

    fn write_spool(&self, envelope: &HookEnvelope) -> PathBuf {
        fs::create_dir_all(self.spool_dir()).expect("spool dir");
        let order = SPOOL_ORDER.fetch_add(1, Ordering::Relaxed);
        let millis = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_millis())
            .unwrap_or(0);
        let path = self.spool_dir().join(format!(
            "{millis:013}-{order:05}-{}.json",
            envelope.delivery_id
        ));
        fs::write(
            &path,
            serde_json::to_vec(envelope).expect("serialize envelope"),
        )
        .expect("write spool");
        path
    }

    fn filter(&self, raw: &serde_json::Value) -> HookEnvelope {
        let mut envelope = filter_hook_input(
            &serde_json::to_vec(raw).expect("serialize raw"),
            &FilterContext {
                project_id: &self.project.id,
                project_root: Path::new(&self.project.root_path),
                salt: &self.salt(),
            },
        )
        .expect("filter hook input");
        envelope.capture_generation = self.database.with_transaction(|tx| {
            let generation = tx.query_row(
                "SELECT coalesce((SELECT capture_generation FROM claude_integrations WHERE project_id = ?1), 0)",
                [&self.project.id],
                |row| row.get(0),
            )?;
            Ok(generation)
        }).expect("read fixture generation");
        envelope
    }

    fn spool_raw(&self, raw: &serde_json::Value) -> HookEnvelope {
        let envelope = self.filter(raw);
        self.write_spool(&envelope);
        envelope
    }
}

fn session_start(project: &Project) -> serde_json::Value {
    json!({
        "session_id": "session-alpha",
        "cwd": project.root_path,
        "hook_event_name": "SessionStart",
        "source": "startup"
    })
}

fn pre_tool(project: &Project, tool_use_id: &str, tool_name: &str) -> serde_json::Value {
    json!({
        "session_id": "session-alpha",
        "cwd": project.root_path,
        "hook_event_name": "PreToolUse",
        "prompt_id": "prompt-alpha",
        "tool_name": tool_name,
        "tool_input": {
            "command": "curl -H 'Authorization: Bearer SECRET_TOKEN' https://example.com",
            "description": "SECRET_DESCRIPTION"
        },
        "tool_use_id": tool_use_id
    })
}

fn post_tool(project: &Project, tool_use_id: &str, tool_name: &str) -> serde_json::Value {
    json!({
        "session_id": "session-alpha",
        "cwd": project.root_path,
        "hook_event_name": "PostToolUse",
        "prompt_id": "prompt-alpha",
        "tool_name": tool_name,
        "tool_input": {"command": "SECRET_COMMAND"},
        "tool_response": {"stdout": "SECRET_OUTPUT"},
        "tool_use_id": tool_use_id,
        "duration_ms": 42
    })
}

fn failed_tool(project: &Project, tool_use_id: &str, tool_name: &str) -> serde_json::Value {
    json!({
        "session_id": "session-alpha",
        "cwd": project.root_path,
        "hook_event_name": "PostToolUseFailure",
        "prompt_id": "prompt-alpha",
        "tool_name": tool_name,
        "tool_input": {"command": "SECRET_FAILED_COMMAND"},
        "tool_use_id": tool_use_id,
        "error": "Exit code 1\nError: SECRET_API_KEY=sk-live-123",
        "is_interrupt": false,
        "duration_ms": 7
    })
}

fn stop(project: &Project) -> serde_json::Value {
    json!({
        "session_id": "session-alpha",
        "cwd": project.root_path,
        "hook_event_name": "Stop",
        "prompt_id": "prompt-alpha",
        "last_assistant_message": "SECRET_ASSISTANT_MESSAGE",
        "stop_hook_active": false
    })
}

fn session_end(project: &Project) -> serde_json::Value {
    json!({
        "session_id": "session-alpha",
        "cwd": project.root_path,
        "hook_event_name": "SessionEnd",
        "reason": "other"
    })
}

#[test]
fn install_preserves_other_config_and_uninstall_is_reversible() {
    let fixture = fixture();
    fs::create_dir_all(fixture.settings_path().parent().expect("claude dir")).expect("claude dir");
    let original = json!({
        "permissions": {"allow": ["Bash(npm test)"]},
        "env": {"KEEP_ME": "yes"},
        "custom": {"keep": true},
        "hooks": {
            "PreToolUse": [
                {
                    "matcher": "Bash",
                    "hooks": [
                        {
                            "type": "command",
                            "command": "C:\\other\\third-party-hook.exe",
                            "args": ["--third-party"]
                        }
                    ]
                }
            ]
        }
    });
    let original_bytes = serde_json::to_vec_pretty(&original).expect("serialize original");
    fs::write(fixture.settings_path(), &original_bytes).expect("write original settings");

    let preview = fixture.preview();
    assert!(preview.blocked_reason.is_none());
    assert_eq!(preview.managed_entries.len(), 6);
    assert_eq!(preview.other_hook_group_count, 1);
    assert!(preview.settings_file.ends_with("settings.local.json"));
    assert!(!preview.warnings.is_empty());

    let status = fixture.install();
    assert_eq!(status.install_state, ClaudeInstallState::Installed);
    assert_eq!(status.managed_hook_count, 6);
    assert_eq!(status.other_hook_group_count, 1);

    let installed: serde_json::Value =
        serde_json::from_slice(&fs::read(fixture.settings_path()).expect("read installed"))
            .expect("parse installed");
    assert_eq!(installed["permissions"], original["permissions"]);
    assert_eq!(installed["env"], original["env"]);
    assert_eq!(installed["custom"], original["custom"]);
    assert_eq!(
        installed["hooks"]["PreToolUse"][0],
        original["hooks"]["PreToolUse"][0]
    );
    let managed = installed["hooks"]["SessionStart"]
        .as_array()
        .expect("session start groups");
    assert_eq!(managed.len(), 1);
    assert_eq!(managed[0]["hooks"][0]["args"][0], json!("--xushu-managed"));
    assert_eq!(managed[0]["hooks"][0]["args"][1], json!(fixture.project.id));
    assert!(!Path::new(&fixture.project.root_path)
        .join(".claude")
        .join("settings.json")
        .exists());

    let backups = fs::read_dir(
        fixture
            .data_directory
            .join("hook-backups")
            .join(&fixture.project.id),
    )
    .expect("read backups")
    .filter_map(Result::ok)
    .count();
    assert!(backups >= 1);

    let installed_bytes = fs::read(fixture.settings_path()).expect("read installed bytes");
    let second = fixture.install();
    assert_eq!(second.install_state, ClaudeInstallState::Installed);
    assert_eq!(
        fs::read(fixture.settings_path()).expect("read after second install"),
        installed_bytes,
        "repeat install must not rewrite an already exact configuration"
    );

    let disabled = fixture
        .service
        .disable(ClaudeActionInput {
            project_id: fixture.project.id.clone(),
            approved: true,
        })
        .expect("disable");
    assert_eq!(disabled.install_state, ClaudeInstallState::Disabled);
    assert_eq!(disabled.managed_hook_count, 0);
    let after_disable: serde_json::Value =
        serde_json::from_slice(&fs::read(fixture.settings_path()).expect("read disabled"))
            .expect("parse disabled");
    assert_eq!(after_disable["permissions"], original["permissions"]);
    assert_eq!(after_disable["env"], original["env"]);
    assert_eq!(after_disable["custom"], original["custom"]);
    assert_eq!(
        after_disable["hooks"]["PreToolUse"][0],
        original["hooks"]["PreToolUse"][0]
    );
    assert!(after_disable["hooks"].get("SessionStart").is_none());

    fixture.install();
    let uninstalled = fixture
        .service
        .uninstall(ClaudeActionInput {
            project_id: fixture.project.id.clone(),
            approved: true,
        })
        .expect("uninstall");
    assert_eq!(uninstalled.install_state, ClaudeInstallState::Disabled);
    assert!(uninstalled.uninstalled_at.is_some());
    let second_uninstall = fixture
        .service
        .uninstall(ClaudeActionInput {
            project_id: fixture.project.id.clone(),
            approved: true,
        })
        .expect("repeat uninstall");
    assert_eq!(second_uninstall.install_state, ClaudeInstallState::Disabled);

    let after_uninstall: serde_json::Value =
        serde_json::from_slice(&fs::read(fixture.settings_path()).expect("read uninstalled"))
            .expect("parse uninstalled");
    assert_eq!(after_uninstall["permissions"], original["permissions"]);
    assert_eq!(after_uninstall["env"], original["env"]);
    assert_eq!(after_uninstall["custom"], original["custom"]);
    assert_eq!(
        after_uninstall["hooks"]["PreToolUse"][0],
        original["hooks"]["PreToolUse"][0]
    );
}

#[test]
fn invalid_configuration_is_never_overwritten() {
    let fixture = fixture();
    fs::create_dir_all(fixture.settings_path().parent().expect("claude dir")).expect("claude dir");
    fs::write(fixture.settings_path(), b"{ not valid json").expect("write broken settings");

    let preview = fixture.preview();
    assert!(preview.blocked_reason.is_some());
    let error = fixture
        .service
        .install(InstallClaudeHooksInput {
            project_id: fixture.project.id.clone(),
            preview_token: preview.preview_token,
            approved: true,
        })
        .expect_err("install must refuse broken config");
    assert_eq!(error.code(), "CLAUDE_INTEGRATION_ERROR");
    assert_eq!(
        fs::read(fixture.settings_path()).expect("read unchanged"),
        b"{ not valid json"
    );
    let status = fixture.service.status(&fixture.project.id).expect("status");
    assert_eq!(status.install_state, ClaudeInstallState::ConfigurationError);

    fs::write(fixture.settings_path(), br#"{"hooks": ["not-an-object"]}"#)
        .expect("write malformed hooks");
    let preview = fixture.preview();
    assert!(preview.blocked_reason.is_some());
    assert_eq!(
        fs::read(fixture.settings_path()).expect("read malformed"),
        br#"{"hooks": ["not-an-object"]}"#
    );
}

#[test]
fn install_requires_approval_and_fresh_preview() {
    let fixture = fixture();
    let preview = fixture.preview();
    let error = fixture
        .service
        .install(InstallClaudeHooksInput {
            project_id: fixture.project.id.clone(),
            preview_token: preview.preview_token.clone(),
            approved: false,
        })
        .expect_err("approval required");
    assert_eq!(error.code(), "APPROVAL_REQUIRED");
    assert!(!fixture.settings_path().exists());

    let error = fixture
        .service
        .install(InstallClaudeHooksInput {
            project_id: fixture.project.id.clone(),
            preview_token: "stale".to_owned(),
            approved: true,
        })
        .expect_err("stale preview");
    assert_eq!(error.code(), "CONFLICT");
    assert!(!fixture.settings_path().exists());

    ProjectService::new(fixture.database)
        .archive(&fixture.project.id)
        .expect("archive project");
    let preview = fixture.preview();
    assert!(preview.blocked_reason.is_some());
    let error = fixture
        .service
        .install(InstallClaudeHooksInput {
            project_id: fixture.project.id.clone(),
            preview_token: preview.preview_token,
            approved: true,
        })
        .expect_err("archived project");
    assert_eq!(error.code(), "CONFLICT");
}

#[test]
fn ingests_minimal_events_without_persisting_sensitive_material_and_never_touches_tasks() {
    let fixture = fixture();
    fixture.install();

    let task = TaskService::new(fixture.database)
        .create_task(CreateTaskInput {
            project_id: fixture.project.id.clone(),
            title: "观察但不改任务".to_owned(),
            description: None,
            priority: "medium".to_owned(),
        })
        .expect("task");
    let before = TaskService::new(fixture.database)
        .get_task(&task.id)
        .expect("task detail");

    let sensitive = [
        "SECRET_TOKEN",
        "SECRET_DESCRIPTION",
        "SECRET_COMMAND",
        "SECRET_OUTPUT",
        "SECRET_FAILED_COMMAND",
        "sk-live-123",
        "SECRET_ASSISTANT_MESSAGE",
        "session-alpha",
        "tool-1",
    ];

    let envelopes = vec![
        fixture.spool_raw(&session_start(&fixture.project)),
        fixture.spool_raw(&pre_tool(&fixture.project, "tool-1", "Bash")),
        fixture.spool_raw(&post_tool(&fixture.project, "tool-1", "Bash")),
        fixture.spool_raw(&failed_tool(&fixture.project, "tool-2", "Read")),
        fixture.spool_raw(&stop(&fixture.project)),
        fixture.spool_raw(&session_end(&fixture.project)),
    ];
    for envelope in &envelopes {
        let serialized = serde_json::to_string(envelope).expect("serialize envelope");
        for value in sensitive {
            assert!(
                !serialized.contains(value),
                "spool envelope leaked {value}: {serialized}"
            );
        }
    }

    let report = fixture
        .service
        .import_pending(&fixture.project.id)
        .expect("import");
    assert_eq!(report.imported, 6);
    assert_eq!(report.remaining, 0);

    let activity = fixture
        .service
        .activity(&fixture.project.id)
        .expect("activity");
    assert_eq!(activity.events.len(), 6);
    assert_eq!(activity.sessions.len(), 1);
    assert_eq!(activity.sessions[0].activity_state, "ended");
    assert_eq!(activity.tool_calls.len(), 2);
    assert!(activity
        .tool_calls
        .iter()
        .any(|call| call.status == "succeeded" && call.tool_name == "Bash"));
    assert!(activity
        .tool_calls
        .iter()
        .any(|call| call.status == "failed" && call.tool_name == "Read"));
    assert_eq!(
        activity
            .events
            .iter()
            .filter(|event| event.event_type == "stop")
            .count(),
        1
    );

    let after = TaskService::new(fixture.database)
        .get_task(&task.id)
        .expect("task detail after");
    assert_eq!(after.task.status, before.task.status);
    assert_eq!(after.task.revision, before.task.revision);
    assert!(xushu_core::VerificationService::new(fixture.database)
        .list_runs(&task.id)
        .expect("verification runs")
        .is_empty());

    for path in database_files(&fixture.data_directory) {
        let bytes = fs::read(&path).expect("read database file");
        for value in sensitive {
            assert!(
                !bytes
                    .windows(value.len())
                    .any(|window| window == value.as_bytes()),
                "database file {} contains {value}",
                path.display()
            );
        }
    }
}

fn database_files(directory: &Path) -> Vec<PathBuf> {
    ["xushu.db", "xushu.db-wal", "xushu.db-shm"]
        .iter()
        .map(|name| directory.join(name))
        .filter(|path| path.exists())
        .collect()
}

#[test]
fn duplicate_completion_is_counted_once() {
    let fixture = fixture();
    fixture.install();
    fixture.spool_raw(&session_start(&fixture.project));
    fixture.spool_raw(&pre_tool(&fixture.project, "tool-duplicate", "Bash"));
    fixture.spool_raw(&post_tool(&fixture.project, "tool-duplicate", "Bash"));
    fixture.spool_raw(&post_tool(&fixture.project, "tool-duplicate", "Bash"));
    fixture.spool_raw(&session_end(&fixture.project));

    let report = fixture
        .service
        .import_pending(&fixture.project.id)
        .expect("import");
    assert_eq!(report.imported, 4);
    assert_eq!(report.duplicates, 1);

    let activity = fixture
        .service
        .activity(&fixture.project.id)
        .expect("activity");
    assert_eq!(activity.tool_calls.len(), 1);
    assert_eq!(activity.tool_calls[0].status, "succeeded");
    assert_eq!(
        activity
            .events
            .iter()
            .filter(|event| event.event_type == "post_tool_use")
            .count(),
        1
    );
}

#[test]
fn missing_completion_is_reported_as_unknown_not_success() {
    let fixture = fixture();
    fixture.install();
    fixture.spool_raw(&session_start(&fixture.project));
    fixture.spool_raw(&pre_tool(&fixture.project, "tool-never-finished", "Bash"));

    let report = fixture
        .service
        .import_pending(&fixture.project.id)
        .expect("import");
    assert_eq!(report.imported, 2);

    let status = fixture.service.status(&fixture.project.id).expect("status");
    assert_eq!(status.running_tool_count, 1);
    let activity = fixture
        .service
        .activity(&fixture.project.id)
        .expect("activity");
    assert_eq!(activity.tool_calls[0].status, "running");

    fixture.spool_raw(&session_end(&fixture.project));
    fixture
        .service
        .import_pending(&fixture.project.id)
        .expect("import end");
    let status = fixture.service.status(&fixture.project.id).expect("status");
    assert_eq!(status.running_tool_count, 0);
    let activity = fixture
        .service
        .activity(&fixture.project.id)
        .expect("activity");
    assert_eq!(activity.tool_calls[0].status, "unknown");
    assert!(!activity.tool_calls[0].finish_observed);
}

#[test]
fn tool_start_after_session_end_is_unknown() {
    let fixture = fixture();
    fixture.install();
    fixture.spool_raw(&session_start(&fixture.project));
    fixture.spool_raw(&session_end(&fixture.project));
    fixture.spool_raw(&pre_tool(&fixture.project, "tool-late-start", "Bash"));
    fixture
        .service
        .import_pending(&fixture.project.id)
        .expect("import");

    let activity = fixture
        .service
        .activity(&fixture.project.id)
        .expect("activity");
    assert_eq!(activity.tool_calls.len(), 1);
    assert_eq!(activity.tool_calls[0].status, "unknown");
    assert!(!activity.tool_calls[0].finish_observed);
}

#[test]
fn rejects_unauthorized_malformed_and_oversized_spool_files() {
    let fixture = fixture();
    fixture.install();

    let other_envelope = filter_hook_input(
        &serde_json::to_vec(&pre_tool(&fixture.project, "tool-other", "Bash")).expect("raw"),
        &FilterContext {
            project_id: OTHER_PROJECT_ID,
            project_root: Path::new(&fixture.project.root_path),
            salt: &fixture.salt(),
        },
    )
    .expect("filter other");
    fixture.write_spool(&other_envelope);

    fs::create_dir_all(fixture.spool_dir()).expect("spool dir");
    fs::write(fixture.spool_dir().join("invalid.json"), b"{broken").expect("invalid spool");
    let oversized_path = fixture.spool_dir().join("oversized.json");
    fs::write(&oversized_path, vec![b'x'; 17 * 1024]).expect("oversized spool");

    let report = fixture
        .service
        .import_pending(&fixture.project.id)
        .expect("import rejected");
    assert_eq!(report.imported, 0);
    assert_eq!(report.rejected, 3);
    assert_eq!(report.remaining, 0);
    assert!(
        fixture
            .service
            .status(&fixture.project.id)
            .expect("status")
            .event_count
            == 0
    );
}

#[test]
fn disabling_stops_ingesting_new_events() {
    let fixture = fixture();
    fixture.install();
    fixture.spool_raw(&session_start(&fixture.project));
    fixture
        .service
        .import_pending(&fixture.project.id)
        .expect("first import");

    fixture
        .service
        .disable(ClaudeActionInput {
            project_id: fixture.project.id.clone(),
            approved: true,
        })
        .expect("disable");

    fixture.spool_raw(&pre_tool(&fixture.project, "tool-after-disable", "Bash"));
    let report = fixture
        .service
        .import_pending(&fixture.project.id)
        .expect("import disabled");
    assert_eq!(report.imported, 0);
    assert_eq!(report.ignored, 1);
    let status = fixture.service.status(&fixture.project.id).expect("status");
    assert_eq!(status.event_count, 1);
    assert_eq!(status.pending_event_count, 0);
}

#[test]
fn import_is_bounded_and_retention_removes_old_events() {
    let fixture = fixture();
    fixture.install();

    for index in 0..205 {
        fixture.spool_raw(&pre_tool(
            &fixture.project,
            &format!("tool-batch-{index}"),
            "Read",
        ));
    }
    let first = fixture
        .service
        .import_pending(&fixture.project.id)
        .expect("first batch");
    assert_eq!(first.imported, 200);
    assert_eq!(first.remaining, 5);
    let second = fixture
        .service
        .import_pending(&fixture.project.id)
        .expect("second batch");
    assert_eq!(second.imported, 5);
    assert_eq!(second.remaining, 0);

    let mut old = fixture.filter(&pre_tool(&fixture.project, "tool-old", "Read"));
    old.captured_at = "2020-01-01T00:00:00.000Z".to_owned();
    fixture.write_spool(&old);
    let report = fixture
        .service
        .import_pending(&fixture.project.id)
        .expect("import old");
    assert_eq!(report.imported, 1);
    let activity = fixture
        .service
        .activity(&fixture.project.id)
        .expect("activity");
    assert!(activity
        .events
        .iter()
        .all(|event| event.occurred_at != "2020-01-01T00:00:00.000Z"));
}

#[test]
fn project_root_mismatch_is_rejected() {
    let fixture = fixture();
    fixture.install();
    let other_root = fixture.directory.path().join("other-repo");
    fs::create_dir_all(&other_root).expect("other root");
    let envelope = filter_hook_input(
        &serde_json::to_vec(&json!({
            "session_id": "session-other",
            "cwd": other_root.to_string_lossy(),
            "hook_event_name": "SessionStart",
            "source": "startup"
        }))
        .expect("raw"),
        &FilterContext {
            project_id: &fixture.project.id,
            project_root: &other_root,
            salt: &fixture.salt(),
        },
    )
    .expect("filter other root");
    fixture.write_spool(&envelope);
    let report = fixture
        .service
        .import_pending(&fixture.project.id)
        .expect("import");
    assert_eq!(report.rejected, 1);
    assert_eq!(report.imported, 0);
}

#[test]
fn empty_activity_and_unconfigured_status_are_honest() {
    let fixture = fixture();
    let status = fixture.service.status(&fixture.project.id).expect("status");
    assert_eq!(status.install_state, ClaudeInstallState::NotConfigured);
    assert!(!status.desired_enabled);
    assert!(!status.has_record);
    assert_eq!(status.event_count, 0);
    assert_eq!(status.pending_event_count, 0);

    let activity = fixture
        .service
        .activity(&fixture.project.id)
        .expect("activity");
    assert!(activity.events.is_empty());
    assert!(activity.sessions.is_empty());
    assert!(activity.tool_calls.is_empty());
}

#[test]
fn external_modification_and_receiver_update_are_detected_and_repaired() {
    let fixture = fixture();
    fixture.install();

    let settings: serde_json::Value =
        serde_json::from_slice(&fs::read(fixture.settings_path()).expect("read settings"))
            .expect("parse settings");
    let mut modified = settings.clone();
    let groups = modified["hooks"]["PreToolUse"]
        .as_array_mut()
        .expect("pre tool groups");
    groups.retain(|group| group["hooks"][0]["args"][0].as_str() != Some("--xushu-managed"));
    fs::write(
        fixture.settings_path(),
        serde_json::to_vec_pretty(&modified).expect("serialize modified"),
    )
    .expect("write modified");

    let status = fixture.service.status(&fixture.project.id).expect("status");
    assert_eq!(status.install_state, ClaudeInstallState::Partial);
    assert_eq!(status.managed_hook_count, 5);
    let repaired = fixture.install();
    assert_eq!(repaired.install_state, ClaudeInstallState::Installed);

    fs::write(
        fixture.data_directory.join("bin").join("xushu-hook.exe"),
        b"updated hook binary",
    )
    .expect("update binary");
    let status = fixture.service.status(&fixture.project.id).expect("status");
    assert_eq!(status.install_state, ClaudeInstallState::NeedsRepair);
    assert!(status.receiver_changed);
    let repaired = fixture.install();
    assert_eq!(repaired.install_state, ClaudeInstallState::Installed);
    assert!(!repaired.receiver_changed);
}

#[test]
fn interleaved_tool_calls_are_tracked_independently() {
    let fixture = fixture();
    fixture.install();
    fixture.spool_raw(&session_start(&fixture.project));
    fixture.spool_raw(&pre_tool(&fixture.project, "tool-a", "Bash"));
    fixture.spool_raw(&pre_tool(&fixture.project, "tool-b", "Read"));
    fixture.spool_raw(&post_tool(&fixture.project, "tool-b", "Read"));
    fixture.spool_raw(&post_tool(&fixture.project, "tool-a", "Bash"));
    fixture.spool_raw(&session_end(&fixture.project));
    fixture
        .service
        .import_pending(&fixture.project.id)
        .expect("import");

    let activity = fixture
        .service
        .activity(&fixture.project.id)
        .expect("activity");
    assert_eq!(activity.tool_calls.len(), 2);
    assert!(activity
        .tool_calls
        .iter()
        .all(|call| call.status == "succeeded"));
    assert_eq!(
        activity
            .events
            .iter()
            .filter(|event| event.event_type == "post_tool_use")
            .count(),
        2
    );
}

#[test]
fn conflicting_completion_events_become_unknown() {
    let fixture = fixture();
    fixture.install();
    fixture.spool_raw(&session_start(&fixture.project));
    fixture.spool_raw(&pre_tool(&fixture.project, "tool-conflict", "Bash"));
    fixture.spool_raw(&post_tool(&fixture.project, "tool-conflict", "Bash"));
    fixture.spool_raw(&failed_tool(&fixture.project, "tool-conflict", "Bash"));
    fixture.spool_raw(&session_end(&fixture.project));
    fixture
        .service
        .import_pending(&fixture.project.id)
        .expect("import");

    let activity = fixture
        .service
        .activity(&fixture.project.id)
        .expect("activity");
    assert_eq!(activity.tool_calls.len(), 1);
    assert_eq!(activity.tool_calls[0].status, "unknown");
    assert!(activity.tool_calls[0].conflict);
}

#[test]
fn directory_redirection_is_refused() {
    let fixture = fixture();
    let claude_dir = Path::new(&fixture.project.root_path).join(".claude");
    let redirected = fixture.directory.path().join("redirected-settings");
    fs::create_dir_all(&redirected).expect("redirect target");

    #[cfg(windows)]
    {
        junction(&claude_dir, &redirected);

        let preview = fixture.preview();
        assert!(preview.blocked_reason.is_some());
        let error = fixture
            .service
            .install(InstallClaudeHooksInput {
                project_id: fixture.project.id.clone(),
                preview_token: preview.preview_token,
                approved: true,
            })
            .expect_err("redirected path");
        assert_eq!(error.code(), "CLAUDE_INTEGRATION_ERROR");
        assert!(!redirected.join("settings.local.json").exists());
    }
    #[cfg(not(windows))]
    {
        let _ = (claude_dir, redirected);
    }
}

#[test]
fn claude_path_that_is_a_file_is_refused() {
    let fixture = fixture();
    let claude_dir = Path::new(&fixture.project.root_path).join(".claude");
    fs::write(&claude_dir, b"not a directory").expect("write .claude file");
    let preview = fixture.preview();
    assert!(preview.blocked_reason.is_some());
    assert_eq!(
        fs::read(&claude_dir).expect("read .claude file"),
        b"not a directory"
    );
}

#[test]
fn missing_receiver_blocks_install_and_reports_unavailable() {
    let fixture = fixture();
    fixture.install();
    fs::remove_file(fixture.data_directory.join("bin").join("xushu-hook.exe"))
        .expect("remove receiver");

    let status = fixture.service.status(&fixture.project.id).expect("status");
    assert_eq!(
        status.install_state,
        ClaudeInstallState::ReceiverUnavailable
    );
    assert!(!status.hook_binary_available);
    let preview = fixture.preview();
    assert!(preview.blocked_reason.is_some());
    assert!(!preview.receiver_available);
}

#[test]
fn runtime_paths_are_stable_across_restarts() {
    let fixture = fixture();
    let first_salt = fixture.salt();
    let runtime =
        ClaudeRuntimePaths::prepare(&fixture.data_directory, None, "0.1.0").expect("prepare again");
    let second_salt = fs::read_to_string(
        fixture
            .data_directory
            .join("hook-inbox")
            .join(SALT_FILE_NAME),
    )
    .expect("read salt");
    assert_eq!(decode_salt(second_salt.trim()).expect("decode"), first_salt);
    drop(runtime);
}

#[test]
fn runtime_initializes_in_the_appdata_filesystem() {
    let directory = match std::env::var_os("XUSHU_TEST_APPDATA") {
        Some(root) => TempDir::new_in(root),
        None => TempDir::new(),
    }
    .expect("temp app data");
    ClaudeRuntimePaths::prepare(directory.path(), None, "0.1.0").expect("initialize inbox");
    assert!(xushu_hook_protocol::read_salt(&directory.path().join("hook-inbox")).is_ok());
}

#[test]
fn changed_matcher_or_failure_policy_requires_repair() {
    let fixture = fixture();
    fixture.install();
    let settings = fixture.settings_path();
    let original: serde_json::Value =
        serde_json::from_slice(&fs::read(&settings).expect("settings")).expect("json");

    for changed in [
        {
            let mut document = original.clone();
            document["hooks"]["PreToolUse"][0]["matcher"] = json!("NeverMatches");
            document
        },
        {
            let mut document = original.clone();
            document["hooks"]["PreToolUse"][0]["hooks"][0]["onFailure"] = json!("block");
            document
        },
    ] {
        fs::write(&settings, serde_json::to_vec(&changed).expect("json")).expect("modify");
        let status = fixture.service.status(&fixture.project.id).expect("status");
        assert_eq!(status.install_state, ClaudeInstallState::NeedsRepair);
        assert_eq!(
            fixture.install().install_state,
            ClaudeInstallState::Installed
        );
    }
}

#[test]
fn pre_tool_timeline_row_does_not_claim_a_later_outcome() {
    let fixture = fixture();
    fixture.install();
    fixture.spool_raw(&pre_tool(&fixture.project, "read-1", "Read"));
    fixture.spool_raw(&post_tool(&fixture.project, "read-1", "Read"));
    let activity = fixture
        .service
        .activity(&fixture.project.id)
        .expect("activity");
    let start = activity
        .events
        .iter()
        .find(|event| event.event_type == "pre_tool_use")
        .unwrap();
    assert_eq!(start.status, "observed");
    assert!(start.failure_kind.is_none());
    assert_eq!(activity.tool_calls[0].status, "succeeded");
}

#[test]
fn idle_refresh_enforces_event_age_and_count_after_disable() {
    let fixture = fixture();
    fixture.install();
    fixture.spool_raw(&session_start(&fixture.project));
    assert_eq!(
        fixture
            .service
            .import_pending(&fixture.project.id)
            .unwrap()
            .imported,
        1
    );
    fixture
        .service
        .disable(ClaudeActionInput {
            project_id: fixture.project.id.clone(),
            approved: true,
        })
        .expect("disable");
    fixture
        .database
        .with_transaction(|transaction| {
            transaction.execute(
                "UPDATE claude_events SET occurred_at = '2020-01-01T00:00:00.000Z'",
                [],
            )?;
            transaction.execute(
                "WITH RECURSIVE counter(n) AS (
                SELECT 1 UNION ALL SELECT n + 1 FROM counter WHERE n < 5001
             )
             INSERT INTO claude_events (
                 id, delivery_id, project_id, session_id, event_type,
                 occurred_at, ingested_at, created_at
             )
             SELECT 'fixture-' || n, 'fixture-' || n, s.project_id, s.id, 'stop',
                    s.started_at, s.started_at, s.started_at
             FROM claude_sessions s CROSS JOIN counter",
                [],
            )?;
            Ok(())
        })
        .expect("seed retention boundary");
    let status = fixture
        .service
        .refresh(&fixture.project.id)
        .expect("idle refresh");
    assert_eq!(status.event_count, 5000);
    fixture
        .database
        .with_transaction(|transaction| {
            let old: i64 = transaction.query_row(
                "SELECT COUNT(*) FROM claude_events WHERE occurred_at = '2020-01-01T00:00:00.000Z'",
                [],
                |row| row.get(0),
            )?;
            assert_eq!(old, 0);
            Ok(())
        })
        .expect("check retained events");
}

#[cfg(windows)]
#[test]
fn import_retries_when_an_envelope_is_still_open_for_exclusive_write() {
    use std::io::Write;
    use std::os::windows::fs::OpenOptionsExt;

    let fixture = fixture();
    fixture.install();
    fs::create_dir_all(fixture.spool_dir()).unwrap();
    let path = fixture.spool_dir().join("pending.json");
    let mut writer = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .share_mode(0)
        .open(&path)
        .unwrap();
    let report = fixture.service.import_pending(&fixture.project.id).unwrap();
    assert_eq!(report.remaining, 1);
    assert_eq!(report.rejected, 0);
    let event = fixture.filter(&session_start(&fixture.project));
    writer
        .write_all(&serde_json::to_vec(&event).unwrap())
        .unwrap();
    drop(writer);
    let report = fixture.service.import_pending(&fixture.project.id).unwrap();
    assert_eq!(report.imported, 1);
    assert_eq!(report.remaining, 0);
}

#[test]
fn install_and_uninstall_keep_empty_third_party_hook_groups() {
    let fixture = fixture();
    let settings = fixture.settings_path();
    fs::create_dir_all(settings.parent().unwrap()).unwrap();
    let original = json!({
        "hooks": {"PreToolUse": [{"matcher":"Read","hooks":[]}], "ConfigChange": []}
    });
    fs::write(&settings, serde_json::to_vec(&original).unwrap()).unwrap();
    fixture.install();
    fixture
        .service
        .uninstall(ClaudeActionInput {
            project_id: fixture.project.id.clone(),
            approved: true,
        })
        .unwrap();
    let restored: serde_json::Value =
        serde_json::from_slice(&fs::read(&settings).unwrap()).unwrap();
    assert_eq!(restored, original);
}

#[test]
fn closeout_config_stays_off_if_database_update_fails_and_retry_syncs() {
    let fixture = isolated_fixture();
    fixture.install();
    fixture.spool_raw(&session_start(&fixture.project));
    fixture.database.with_transaction(|transaction| {
        transaction.execute_batch(
            "CREATE TRIGGER fixture_disable_fail BEFORE UPDATE OF desired_enabled ON claude_integrations
             WHEN NEW.desired_enabled = 0 BEGIN SELECT RAISE(ABORT, 'fixture write failure'); END;",
        )?;
        Ok(())
    }).unwrap();
    assert!(fixture.service.disable(fixture.action()).is_err());
    assert!(fixture.service.uninstall(fixture.action()).is_err());
    let status = fixture.service.status(&fixture.project.id).unwrap();
    assert_eq!(status.managed_hook_count, 0);
    assert!(status.desired_enabled);
    assert_eq!(status.install_state, ClaudeInstallState::NeedsRepair);
    let report = fixture.service.import_pending(&fixture.project.id).unwrap();
    assert_eq!(report.imported, 0);
    assert_eq!(report.remaining, 1);
    fixture
        .database
        .with_transaction(|tx| {
            tx.execute_batch("DROP TRIGGER fixture_disable_fail")?;
            Ok(())
        })
        .unwrap();
    assert_eq!(
        fixture
            .service
            .uninstall(fixture.action())
            .unwrap()
            .install_state,
        ClaudeInstallState::Disabled
    );
    let report = fixture.service.import_pending(&fixture.project.id).unwrap();
    assert_eq!(report.ignored, 1);
    assert_eq!(report.remaining, 0);
}

#[cfg(windows)]
#[test]
fn closeout_removal_does_not_wait_for_exclusive_spool_writer() {
    use std::io::Write;
    use std::os::windows::fs::OpenOptionsExt;

    let fixture = isolated_fixture();
    fixture.install();
    fs::create_dir_all(fixture.spool_dir()).unwrap();
    let pending = fixture.spool_dir().join("locked.json");
    let mut writer = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .share_mode(0)
        .open(&pending)
        .unwrap();
    let disabled = fixture.service.disable(fixture.action()).unwrap();
    assert_eq!(disabled.install_state, ClaudeInstallState::Disabled);
    let old = fixture.filter(&session_start(&fixture.project));
    let uninstalled = fixture.service.uninstall(fixture.action()).unwrap();
    assert!(uninstalled.uninstalled_at.is_some());
    assert_eq!(uninstalled.managed_hook_count, 0);
    writer
        .write_all(&serde_json::to_vec(&old).unwrap())
        .unwrap();
    drop(writer);
    fixture.install();
    let report = fixture.service.import_pending(&fixture.project.id).unwrap();
    assert_eq!(
        (report.imported, report.ignored, report.remaining),
        (0, 1, 0)
    );
    fixture.spool_raw(&session_start(&fixture.project));
    assert_eq!(
        fixture
            .service
            .import_pending(&fixture.project.id)
            .unwrap()
            .imported,
        1
    );
}

#[test]
fn closeout_removal_does_not_require_event_import() {
    let fixture = isolated_fixture();
    fixture.install();
    fixture.spool_raw(&session_start(&fixture.project));
    fixture
        .database
        .with_transaction(|tx| {
            tx.execute_batch(
                "CREATE TRIGGER reject_events BEFORE INSERT ON claude_events
             BEGIN SELECT RAISE(ABORT, 'fixture import failure'); END;",
            )?;
            Ok(())
        })
        .unwrap();
    assert!(fixture.service.import_pending(&fixture.project.id).is_err());
    let disabled = fixture.service.uninstall(fixture.action()).unwrap();
    assert_eq!(disabled.managed_hook_count, 0);
    assert!(!disabled.desired_enabled);
}

#[test]
fn closeout_writer_lock_leaves_configuration_off_and_supports_retry() {
    let fixture = isolated_fixture();
    fixture.install();
    let writer = rusqlite::Connection::open(fixture.data_directory.join("xushu.db")).unwrap();
    writer.execute_batch("BEGIN IMMEDIATE").unwrap();
    let error = fixture.service.disable(fixture.action()).unwrap_err();
    assert_eq!(
        fixture
            .service
            .status(&fixture.project.id)
            .unwrap()
            .managed_hook_count,
        0
    );
    assert_eq!(error.code(), "CLAUDE_INTEGRATION_ERROR");
    writer.execute_batch("ROLLBACK").unwrap();
    let disabled = fixture.service.disable(fixture.action()).unwrap();
    assert_eq!(disabled.install_state, ClaudeInstallState::Disabled);
}

#[cfg(windows)]
fn junction(path: &Path, target: &Path) {
    let script = format!(
        "New-Item -ItemType Junction -Path '{}' -Target '{}' -ErrorAction Stop | Out-Null",
        path.to_string_lossy().replace('\'', "''"),
        target.to_string_lossy().replace('\'', "''"),
    );
    let result = Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", &script])
        .output()
        .expect("launch PowerShell for fixture junction");
    assert!(
        result.status.success(),
        "fixture junction could not be created ({}): {}",
        result.status,
        String::from_utf8_lossy(&result.stderr).trim()
    );
    assert_eq!(
        path.canonicalize().expect("resolve fixture junction"),
        target
            .canonicalize()
            .expect("resolve fixture junction target"),
        "fixture junction must redirect to the intended target"
    );
}

#[cfg(windows)]
#[test]
fn closeout_registered_root_rejects_junction_even_after_new_preview() {
    let fixture = isolated_fixture();
    let project = Path::new(&fixture.project.root_path);
    let previous = fixture.preview();
    let elsewhere = fixture.directory.path().join("unrelated");
    fs::create_dir(&elsewhere).unwrap();
    fs::rename(project, project.with_file_name("previous-repo")).unwrap();
    junction(project, &elsewhere);

    let result = fixture.service.install(InstallClaudeHooksInput {
        project_id: fixture.project.id.clone(),
        preview_token: previous.preview_token,
        approved: true,
    });
    assert!(result.is_err());
    assert!(!elsewhere.join(".claude").exists());
    assert!(fixture.preview().blocked_reason.is_some());
}

#[test]
fn closeout_preview_rejects_replaced_directory_at_same_path() {
    let fixture = isolated_fixture();
    let project = Path::new(&fixture.project.root_path);
    let previous = fixture.preview();
    fs::rename(project, project.with_file_name("previous-repo")).unwrap();
    fs::create_dir(project).unwrap();

    let result = fixture.service.install(InstallClaudeHooksInput {
        project_id: fixture.project.id.clone(),
        preview_token: previous.preview_token,
        approved: true,
    });
    assert!(result.is_err());
    assert!(!project.join(".claude").exists());
}

#[test]
fn closeout_removes_hooks_after_receiver_and_inbox_fail_on_restart() {
    let fixture = isolated_fixture();
    fixture.install();
    fs::rename(
        fixture.data_directory.join("bin"),
        fixture.data_directory.join("unavailable-bin"),
    )
    .unwrap();
    fs::rename(
        fixture.data_directory.join("hook-inbox"),
        fixture.data_directory.join("unavailable-inbox"),
    )
    .unwrap();
    fs::write(fixture.data_directory.join("hook-inbox"), b"blocked").unwrap();
    let runtime = ClaudeRuntimePaths::prepare(&fixture.data_directory, None, "0.1.0").unwrap();
    let service = ClaudeIntegrationService::new(fixture.database, &runtime);
    assert_eq!(
        service.status(&fixture.project.id).unwrap().install_state,
        ClaudeInstallState::ReceiverUnavailable
    );
    let result = service.uninstall(fixture.action()).unwrap();
    assert_eq!(result.install_state, ClaudeInstallState::Disabled);
    assert!(!result.hook_binary_available);
    assert!(!result.inbox_available);
    assert!(result.uninstalled_at.is_some());
}

#[test]
fn closeout_uninstall_refuses_replaced_installed_root() {
    let fixture = isolated_fixture();
    fixture.install();
    let project = Path::new(&fixture.project.root_path);
    let moved = project.with_file_name("previous-repo");
    fs::rename(project, &moved).unwrap();
    fs::create_dir_all(project.join(".claude")).unwrap();
    let settings = moved.join(".claude/settings.local.json");
    let original = fs::read(&settings).unwrap();
    fs::write(fixture.settings_path(), &original).unwrap();
    assert!(fixture.service.uninstall(fixture.action()).is_err());
    assert!(fixture.service.disable(fixture.action()).is_err());
    assert!(fixture.preview().blocked_reason.is_some());
    assert_eq!(fs::read(fixture.settings_path()).unwrap(), original);
    assert!(
        fixture
            .service
            .status(&fixture.project.id)
            .unwrap()
            .desired_enabled
    );
}

#[cfg(windows)]
#[test]
fn closeout_rejects_redirected_parent_and_backup_directory() {
    let fixture = isolated_fixture();
    let root = Path::new(&fixture.project.root_path);
    let parent = root.parent().unwrap();
    let moved = parent.with_file_name("previous-parent");
    fs::rename(parent, &moved).unwrap();
    junction(parent, &moved);
    assert!(fixture.preview().blocked_reason.is_some());
    assert!(fixture.service.disable(fixture.action()).is_err());
    assert!(!moved.join("repo/.claude").exists());

    let second = isolated_fixture();
    let original = br#"{"permissions":{"deny":["Read"]}}"#;
    fs::create_dir_all(second.settings_path().parent().unwrap()).unwrap();
    fs::write(second.settings_path(), original).unwrap();
    let elsewhere = second.directory.path().join("unrelated-backup");
    fs::create_dir(&elsewhere).unwrap();
    junction(
        &second
            .data_directory
            .join("hook-backups")
            .join(&second.project.id),
        &elsewhere,
    );
    let preview = second.preview();
    let result = second.service.install(InstallClaudeHooksInput {
        project_id: second.project.id.clone(),
        preview_token: preview.preview_token,
        approved: true,
    });
    assert!(result.is_err());
    assert_eq!(fs::read(second.settings_path()).unwrap(), original);
    assert_eq!(fs::read_dir(elsewhere).unwrap().count(), 0);
}

#[test]
fn closeout_concurrent_refresh_does_not_revive_pending_events() {
    let fixture = isolated_fixture();
    fixture.install();
    let prior = fixture.filter(&session_start(&fixture.project));
    let gate = std::sync::Barrier::new(2);
    std::thread::scope(|scope| {
        let refresh = scope.spawn(|| {
            gate.wait();
            fixture.service.refresh(&fixture.project.id).unwrap();
        });
        gate.wait();
        fixture.service.disable(fixture.action()).unwrap();
        refresh.join().unwrap();
    });
    // A receiver already running may publish an envelope after uninstall.
    fixture.service.uninstall(fixture.action()).unwrap();
    fixture.write_spool(&prior);
    fixture.install();
    let report = fixture.service.import_pending(&fixture.project.id).unwrap();
    assert_eq!((report.imported, report.ignored), (0, 1));
}

#[test]
fn closeout_install_preserves_external_edit_and_recovers_from_database_failure() {
    let fixture = isolated_fixture();
    let settings = fixture.settings_path();
    fs::create_dir_all(settings.parent().unwrap()).unwrap();
    fs::write(&settings, b"{}").unwrap();
    let install = |preview: xushu_core::ClaudeHookPreview| {
        fixture.service.install(InstallClaudeHooksInput {
            project_id: fixture.project.id.clone(),
            preview_token: preview.preview_token,
            approved: true,
        })
    };

    let stale = fixture.preview();
    let edited = br#"{"permissions":{"deny":["Read"]},"hooks":{}}"#;
    fs::write(&settings, edited).unwrap();
    assert_eq!(install(stale).unwrap_err().code(), "CONFLICT");
    assert_eq!(fs::read(&settings).unwrap(), edited);

    fixture
        .database
        .with_transaction(|tx| {
            tx.execute_batch(
                "CREATE TRIGGER reject_install BEFORE INSERT ON claude_integrations
             BEGIN SELECT RAISE(ABORT, 'fixture installation write failure'); END;",
            )?;
            Ok(())
        })
        .unwrap();
    let failed = fixture.preview();
    let failed_args = failed.managed_entries[0].args.clone();
    assert!(install(failed).is_err());
    assert_eq!(fs::read(&settings).unwrap(), edited);
    assert!(
        !fixture
            .service
            .status(&fixture.project.id)
            .unwrap()
            .has_record
    );
    fixture
        .database
        .with_transaction(|tx| {
            tx.execute_batch("DROP TRIGGER reject_install")?;
            Ok(())
        })
        .unwrap();
    let next = fixture.preview();
    assert_ne!(next.managed_entries[0].args, failed_args);
    assert_eq!(
        install(next).unwrap().install_state,
        ClaudeInstallState::Installed
    );
    fixture.service.uninstall(fixture.action()).unwrap();
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&fs::read(&settings).unwrap()).unwrap(),
        serde_json::from_slice::<serde_json::Value>(edited).unwrap()
    );
}

#[test]
fn closeout_legacy_installation_imports_until_first_reenable() {
    let fixture = isolated_fixture();
    fixture.install();
    let settings = fixture.settings_path();
    let mut document: serde_json::Value =
        serde_json::from_slice(&fs::read(&settings).unwrap()).unwrap();
    for groups in document["hooks"].as_object_mut().unwrap().values_mut() {
        let args = groups[0]["hooks"][0]["args"].as_array_mut().unwrap();
        assert_eq!(args[6], "--capture-generation");
        args.truncate(6);
    }
    fs::write(&settings, serde_json::to_vec(&document).unwrap()).unwrap();
    fixture.database.with_transaction(|tx| {
        tx.execute("UPDATE claude_integrations SET capture_generation = 0, project_root_identity = NULL", [])?;
        Ok(())
    }).unwrap();
    assert_eq!(
        fixture
            .service
            .status(&fixture.project.id)
            .unwrap()
            .install_state,
        ClaudeInstallState::Installed
    );
    let old = fixture.spool_raw(&session_start(&fixture.project));
    assert_eq!(old.capture_generation, 0);
    assert_eq!(
        fixture
            .service
            .import_pending(&fixture.project.id)
            .unwrap()
            .imported,
        1
    );
    fixture.service.uninstall(fixture.action()).unwrap();
    fixture.install();
    fixture.write_spool(&old);
    assert_eq!(
        fixture
            .service
            .import_pending(&fixture.project.id)
            .unwrap()
            .ignored,
        1
    );
}
