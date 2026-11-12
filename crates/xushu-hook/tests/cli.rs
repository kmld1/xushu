use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use tempfile::TempDir;
use xushu_hook_protocol::{
    generate_salt_hex, FailureKind, HookEnvelope, HookEventType, MAX_PENDING_EVENTS_PER_PROJECT,
    SALT_FILE_NAME,
};

const PROJECT_ID: &str = "11111111-1111-4111-8111-111111111111";

struct Fixture {
    _directory: TempDir,
    project: PathBuf,
    inbox: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        Self::in_directory(TempDir::new().expect("temp dir"))
    }

    fn in_directory(directory: TempDir) -> Self {
        let project = directory.path().join("项目 & spaces").join("repo");
        fs::create_dir_all(&project).expect("create project");
        let inbox = directory.path().join("hook-inbox");
        fs::create_dir_all(inbox.join("pending")).expect("create inbox");
        fs::write(inbox.join(SALT_FILE_NAME), generate_salt_hex()).expect("write salt");
        Self {
            _directory: directory,
            project,
            inbox,
        }
    }

    fn run(&self, input: &[u8]) -> std::process::Output {
        let mut child = Command::new(env!("CARGO_BIN_EXE_xushu-hook"))
            .arg("--xushu-managed")
            .arg(PROJECT_ID)
            .arg("--project-root")
            .arg(&self.project)
            .arg("--inbox")
            .arg(&self.inbox)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn xushu-hook");
        child
            .stdin
            .take()
            .expect("stdin")
            .write_all(input)
            .expect("write stdin");
        child.wait_with_output().expect("wait xushu-hook")
    }

    fn project_spool(&self) -> PathBuf {
        self.inbox.join("pending").join(PROJECT_ID)
    }

    fn spool_files(&self) -> Vec<PathBuf> {
        let directory = self.project_spool();
        if !directory.exists() {
            return Vec::new();
        }
        let mut entries: Vec<PathBuf> = fs::read_dir(directory)
            .expect("read spool")
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| path.extension().and_then(|value| value.to_str()) == Some("json"))
            .collect();
        entries.sort();
        entries
    }
}

#[test]
fn publishes_envelope_in_the_appdata_filesystem() {
    let directory = match std::env::var_os("XUSHU_TEST_APPDATA") {
        Some(root) => TempDir::new_in(root),
        None => TempDir::new(),
    }
    .expect("temp app data");
    let fixture = Fixture::in_directory(directory);
    let input = hook_input(
        &fixture.project,
        "SessionStart",
        serde_json::json!({"source": "startup"}),
    );
    let output = fixture.run(&input);
    assert!(output.status.success());
    let files = fixture.spool_files();
    assert_eq!(files.len(), 1);
    assert_eq!(
        read_envelope(&files[0]).event_type,
        HookEventType::SessionStart
    );
}

fn hook_input(project: &Path, event: &str, extra: serde_json::Value) -> Vec<u8> {
    let mut object = serde_json::json!({
        "session_id": "raw-session-id",
        "transcript_path": "C:\\Users\\someone\\.claude\\transcript.jsonl",
        "cwd": project.to_string_lossy(),
        "hook_event_name": event,
        "prompt_id": "raw-prompt-id"
    });
    if let (Some(target), Some(extra)) = (object.as_object_mut(), extra.as_object()) {
        for (key, value) in extra {
            target.insert(key.clone(), value.clone());
        }
    }
    serde_json::to_vec(&object).expect("serialize input")
}

fn read_envelope(path: &Path) -> HookEnvelope {
    serde_json::from_slice(&fs::read(path).expect("read spool file")).expect("parse envelope")
}

#[test]
fn filters_and_writes_minimal_envelope_without_secret_material() {
    let fixture = Fixture::new();
    let input = hook_input(
        &fixture.project,
        "PreToolUse",
        serde_json::json!({
            "tool_name": "Bash",
            "tool_input": {
                "command": "curl -H 'Authorization: Bearer SECRET_TOKEN' https://example.com",
                "description": "SECRET_DESCRIPTION"
            },
            "tool_use_id": "toolu_raw_123"
        }),
    );

    let output = fixture.run(&input);
    assert!(output.status.success());
    assert!(output.stdout.is_empty(), "hook must not write stdout");
    assert!(output.stderr.is_empty(), "hook must not write stderr");

    let files = fixture.spool_files();
    assert_eq!(files.len(), 1);
    let envelope = read_envelope(&files[0]);
    assert_eq!(envelope.event_type, HookEventType::PreToolUse);
    assert_eq!(envelope.tool_name.as_deref(), Some("Bash"));

    let serialized = serde_json::to_string(&envelope).expect("serialize");
    assert!(!serialized.contains("SECRET_TOKEN"));
    assert!(!serialized.contains("SECRET_DESCRIPTION"));
    assert!(!serialized.contains("raw-session-id"));
    assert!(!serialized.contains("raw-prompt-id"));
    assert!(!serialized.contains("toolu_raw_123"));
    assert!(!serialized.contains("transcript.jsonl"));
}

#[test]
fn failure_event_records_only_category_and_interrupt_flag() {
    let fixture = Fixture::new();
    let input = hook_input(
        &fixture.project,
        "PostToolUseFailure",
        serde_json::json!({
            "tool_name": "Bash",
            "tool_input": {"command": "npm test"},
            "tool_use_id": "toolu_failure",
            "error": "Exit code 1\nError: SECRET_API_KEY=sk-live-123",
            "is_interrupt": false
        }),
    );

    let output = fixture.run(&input);
    assert!(output.status.success());
    let files = fixture.spool_files();
    assert_eq!(files.len(), 1);
    let envelope = read_envelope(&files[0]);
    assert_eq!(envelope.failure_kind, Some(FailureKind::ExitCodeNonzero));
    let serialized = serde_json::to_string(&envelope).expect("serialize");
    assert!(!serialized.contains("sk-live-123"));
    assert!(!serialized.contains("SECRET_API_KEY"));
    assert!(!serialized.contains("npm test"));
}

#[test]
fn stop_event_does_not_persist_assistant_message() {
    let fixture = Fixture::new();
    let input = hook_input(
        &fixture.project,
        "Stop",
        serde_json::json!({
            "last_assistant_message": "I have finished and everything is SECRET",
            "stop_hook_active": false
        }),
    );

    let output = fixture.run(&input);
    assert!(output.status.success());
    let files = fixture.spool_files();
    assert_eq!(files.len(), 1);
    let envelope = read_envelope(&files[0]);
    assert_eq!(envelope.event_type, HookEventType::Stop);
    let serialized = serde_json::to_string(&envelope).expect("serialize");
    assert!(!serialized.contains("I have finished"));
    assert!(!serialized.contains("SECRET"));
}

#[test]
fn rejects_invalid_oversized_or_outside_project_input_without_blocking() {
    let fixture = Fixture::new();

    let invalid = fixture.run(b"{not-json");
    assert!(invalid.status.success());
    assert!(fixture.spool_files().is_empty());

    let oversized = fixture.run(&vec![b'{'; 256 * 1024 + 1]);
    assert!(oversized.status.success());
    assert!(fixture.spool_files().is_empty());

    let outside = std::env::temp_dir();
    let raw = hook_input(
        &outside,
        "PreToolUse",
        serde_json::json!({
            "tool_name": "Read",
            "tool_input": {"file_path": "C:\\secret.txt"},
            "tool_use_id": "toolu_outside"
        }),
    );
    let result = fixture.run(&raw);
    assert!(result.status.success());
    assert!(fixture.spool_files().is_empty());
}

#[test]
fn missing_salt_or_missing_inbox_is_silently_ignored() {
    let fixture = Fixture::new();
    fs::remove_file(fixture.inbox.join(SALT_FILE_NAME)).expect("remove salt");
    let input = hook_input(
        &fixture.project,
        "SessionStart",
        serde_json::json!({"source": "startup"}),
    );
    let output = fixture.run(&input);
    assert!(output.status.success());
    assert!(fixture.spool_files().is_empty());
}

#[test]
fn respects_pending_capacity_and_version_flag() {
    let fixture = Fixture::new();
    let project_dir = fixture.project_spool();
    fs::create_dir_all(&project_dir).expect("create project spool");
    for index in 0..MAX_PENDING_EVENTS_PER_PROJECT {
        fs::write(
            project_dir.join(format!("{index:013}-existing.json")),
            b"{}",
        )
        .expect("write existing spool");
    }

    let input = hook_input(
        &fixture.project,
        "SessionStart",
        serde_json::json!({"source": "startup"}),
    );
    let output = fixture.run(&input);
    assert!(output.status.success());
    assert_eq!(fixture.spool_files().len(), MAX_PENDING_EVENTS_PER_PROJECT);

    let version = Command::new(env!("CARGO_BIN_EXE_xushu-hook"))
        .arg("--version")
        .output()
        .expect("run version");
    assert!(version.status.success());
    assert_eq!(
        String::from_utf8_lossy(&version.stdout).trim(),
        format!("xushu-hook {}", env!("CARGO_PKG_VERSION"))
    );
}

#[test]
fn simultaneous_receivers_respect_pending_capacity() {
    let fixture = Fixture::new();
    let project_dir = fixture.project_spool();
    fs::create_dir_all(&project_dir).unwrap();
    for index in 0..MAX_PENDING_EVENTS_PER_PROJECT - 1 {
        fs::write(project_dir.join(format!("{index:013}.json")), b"{}").unwrap();
    }
    let input = hook_input(
        &fixture.project,
        "SessionStart",
        serde_json::json!({"source": "startup"}),
    );
    let start = std::sync::Barrier::new(12);
    std::thread::scope(|scope| {
        for _ in 0..12 {
            scope.spawn(|| {
                start.wait();
                assert!(fixture.run(&input).status.success());
            });
        }
    });
    assert_eq!(fixture.spool_files().len(), MAX_PENDING_EVENTS_PER_PROJECT);
}
