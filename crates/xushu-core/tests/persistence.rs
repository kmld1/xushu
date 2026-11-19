use chrono::DateTime;
use rusqlite::{params, Connection};
use tempfile::TempDir;
use xushu_core::{
    parse_created_at, run_migrations, CoreError, CreateProbeInput, Database, Migration, MIGRATIONS,
};

fn open_db() -> (TempDir, Database) {
    let directory = TempDir::new().expect("create temp directory");
    let database = Database::open(directory.path().join("xushu.db")).expect("open database");
    (directory, database)
}

fn probe_input(label: &str, content: &str) -> CreateProbeInput {
    CreateProbeInput {
        label: label.to_owned(),
        content: content.to_owned(),
    }
}

#[test]
fn opens_database_with_safe_pragmas_and_migrations() {
    let (_directory, database) = open_db();
    let health = database.health().expect("database health");

    assert!(health.ok);
    assert!(health.integrity_ok);
    assert!(health.foreign_keys_enabled);
    assert!(!health.foreign_key_violations);
    assert_eq!(health.journal_mode.to_ascii_lowercase(), "wal");
    assert_eq!(health.schema_version, 7);
    assert_eq!(health.record_count, 0);
    assert_eq!(health.database_file, "xushu.db");
}

#[test]
fn migrations_are_idempotent() {
    let (_directory, database) = open_db();
    let report = database.migrate().expect("repeat migrations");

    assert_eq!(report.applied, Vec::<i64>::new());
    assert_eq!(report.current_version, 7);
    assert_eq!(database.health().expect("health").schema_version, 7);
}

#[test]
fn migration_failure_rolls_back_the_whole_migration() {
    let directory = TempDir::new().expect("create temp directory");
    let mut connection = Connection::open(directory.path().join("migration-failure.db"))
        .expect("open raw connection");

    let good = Migration {
        version: 1,
        name: "good",
        sql: "CREATE TABLE good_table (id INTEGER PRIMARY KEY);",
    };
    let bad = Migration {
        version: 2,
        name: "bad",
        sql: "CREATE TABLE should_rollback (id INTEGER PRIMARY KEY);
              INSERT INTO missing_table (id) VALUES (1);",
    };

    let error = run_migrations(&mut connection, &[good, bad]).expect_err("migration must fail");
    assert!(matches!(error, CoreError::Database(_)));

    let applied_count: i64 = connection
        .query_row("SELECT COUNT(*) FROM schema_migrations", [], |row| {
            row.get(0)
        })
        .expect("count applied migrations");
    assert_eq!(applied_count, 1);

    let leftover_table_count: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'should_rollback'",
            [],
            |row| row.get(0),
        )
        .expect("check rolled back table");
    assert_eq!(leftover_table_count, 0);
}

#[test]
fn duplicate_migration_versions_are_rejected() {
    let directory = TempDir::new().expect("create temp directory");
    let mut connection =
        Connection::open(directory.path().join("duplicate.db")).expect("open raw connection");

    let first = Migration {
        version: 1,
        name: "first",
        sql: "CREATE TABLE first_table (id INTEGER PRIMARY KEY);",
    };
    let second = Migration {
        version: 1,
        name: "second",
        sql: "CREATE TABLE second_table (id INTEGER PRIMARY KEY);",
    };

    let error = run_migrations(&mut connection, &[first, second]).expect_err("duplicate versions");
    assert!(matches!(error, CoreError::InvalidMigration(_)));
}

#[test]
fn newer_schema_is_rejected_instead_of_downgraded() {
    let directory = TempDir::new().expect("create temp directory");
    let mut connection =
        Connection::open(directory.path().join("future.db")).expect("open raw connection");

    let supported_version = MIGRATIONS.last().unwrap().version;
    let future = Migration {
        version: supported_version + 1,
        name: "future",
        sql: "CREATE TABLE future_table (id INTEGER PRIMARY KEY);",
    };
    run_migrations(&mut connection, &[future]).expect("apply future migration");

    let error = run_migrations(&mut connection, MIGRATIONS).expect_err("reject downgrade");
    assert!(matches!(
        error,
        CoreError::SchemaTooNew { found, supported }
            if found == future.version && supported == supported_version
    ));
}

#[test]
fn foreign_keys_are_enforced() {
    let (_directory, database) = open_db();

    database
        .with_transaction(|transaction| {
            transaction.execute_batch(
                "CREATE TABLE fk_parent (id INTEGER PRIMARY KEY);
                 CREATE TABLE fk_child (
                     id INTEGER PRIMARY KEY,
                     parent_id INTEGER NOT NULL REFERENCES fk_parent(id)
                 );",
            )?;
            Ok(())
        })
        .expect("create related tables");

    let result = database.with_transaction(|transaction| {
        transaction.execute("INSERT INTO fk_child (id, parent_id) VALUES (1, 999)", [])?;
        Ok(())
    });

    assert!(result.is_err(), "foreign key violation must fail");
}

#[test]
fn transaction_rolls_back_on_error() {
    let (_directory, database) = open_db();

    let result: Result<(), CoreError> = database.with_transaction(|transaction| {
        transaction.execute(
            "INSERT INTO persistence_probes (id, label, content, created_at)
             VALUES (?1, ?2, ?3, ?4)",
            params![
                "00000000-0000-4000-8000-000000000000",
                "rollback",
                "must not persist",
                "2026-01-01T00:00:00.000Z"
            ],
        )?;
        Err(CoreError::Validation("forced rollback".to_owned()))
    });

    assert!(result.is_err());
    assert_eq!(database.health().expect("health").record_count, 0);
}

#[test]
fn probe_roundtrip_preserves_id_content_and_time() {
    let (_directory, database) = open_db();

    let created = database
        .create_probe(probe_input("跨进程验证", "phase-1-persistence"))
        .expect("create probe");

    assert_eq!(created.label, "跨进程验证");
    assert_eq!(created.content, "phase-1-persistence");
    assert!(created.created_at.ends_with('Z'));

    let parsed = DateTime::parse_from_rfc3339(&created.created_at).expect("RFC3339 timestamp");
    assert_eq!(parsed.offset().local_minus_utc(), 0);
    assert!(parse_created_at(&created.created_at).is_some());

    let fetched = database
        .get_probe(&created.id)
        .expect("read probe")
        .expect("probe exists");
    assert_eq!(fetched, created);

    let listed = database.list_probes(Some(10)).expect("list probes");
    assert_eq!(listed, vec![created]);
}

#[test]
fn probe_validation_rejects_invalid_input() {
    let (_directory, database) = open_db();

    let cases = [
        (probe_input("   ", "content"), "标签不能为空".to_owned()),
        (
            probe_input(&"a".repeat(65), "content"),
            "标签最多".to_owned(),
        ),
        (probe_input("label", "   "), "内容不能为空".to_owned()),
        (
            probe_input("label", &"a".repeat(2001)),
            "内容最多".to_owned(),
        ),
        (probe_input("bad\nlabel", "content"), "控制字符".to_owned()),
        (
            probe_input("label", "bad\u{0000}content"),
            "控制字符".to_owned(),
        ),
    ];

    for (input, expected_message) in cases {
        let error = database.create_probe(input).expect_err("invalid input");
        match error {
            CoreError::Validation(message) => {
                assert!(
                    message.contains(&expected_message),
                    "unexpected message: {message}"
                );
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }

    assert_eq!(database.health().expect("health").record_count, 0);
}

#[test]
fn get_probe_rejects_invalid_id_and_missing_record_is_none() {
    let (_directory, database) = open_db();

    let invalid = database.get_probe("not-a-uuid").expect_err("invalid id");
    assert!(matches!(invalid, CoreError::Validation(_)));

    let missing = database
        .get_probe("00000000-0000-4000-8000-000000000000")
        .expect("valid but missing id");
    assert!(missing.is_none());
}

#[test]
fn list_limit_is_validated() {
    let (_directory, database) = open_db();

    assert!(matches!(
        database.list_probes(Some(0)).expect_err("zero limit"),
        CoreError::Validation(_)
    ));
    assert!(matches!(
        database
            .list_probes(Some(101))
            .expect_err("too large limit"),
        CoreError::Validation(_)
    ));
    assert!(database
        .list_probes(None)
        .expect("default limit")
        .is_empty());
}

#[test]
fn phase5_and_phase6_migrations_preserve_phase4_data() {
    let directory = TempDir::new().expect("create temp directory");
    let mut connection =
        Connection::open(directory.path().join("phase4.db")).expect("open raw connection");

    let phase_four =
        run_migrations(&mut connection, &MIGRATIONS[..5]).expect("apply phase 4 migrations");
    assert_eq!(phase_four.current_version, 5);

    connection
        .execute_batch(
            "INSERT INTO projects (id, name, root_path, root_path_key, created_at, updated_at)
             VALUES ('p1', '旧项目', 'C:\\old', 'c:\\old',
                     '2026-01-01T00:00:00.000Z', '2026-01-01T00:00:00.000Z');
             INSERT INTO tasks (
                 id, project_id, title, description, priority, status, revision,
                 block_reason, archived_at, created_at, updated_at
             ) VALUES (
                 't1', 'p1', '旧任务', '', 'medium', 'draft', 1,
                 NULL, NULL, '2026-01-01T00:00:00.000Z', '2026-01-01T00:00:00.000Z'
             );",
        )
        .expect("seed phase 4 data");

    let phase_six = run_migrations(&mut connection, MIGRATIONS).expect("apply later migrations");
    assert_eq!(phase_six.applied, vec![6, 7]);
    assert_eq!(phase_six.current_version, 7);

    let project_count: i64 = connection
        .query_row("SELECT COUNT(*) FROM projects", [], |row| row.get(0))
        .expect("project count");
    assert_eq!(project_count, 1);
    let task_title: String = connection
        .query_row("SELECT title FROM tasks WHERE id = 't1'", [], |row| {
            row.get(0)
        })
        .expect("task title");
    assert_eq!(task_title, "旧任务");

    let new_table_count: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master
             WHERE type = 'table' AND name IN (
                'project_memories', 'project_memory_revisions',
                'decisions', 'decision_revisions',
                'project_recovery_preferences', 'recovery_snapshots'
             )",
            [],
            |row| row.get(0),
        )
        .expect("new table count");
    assert_eq!(new_table_count, 6);

    let phase_six_table_count: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master
             WHERE type = 'table' AND name IN (
                'backup_records', 'backup_settings', 'restore_operations'
             )",
            [],
            |row| row.get(0),
        )
        .expect("phase 6 table count");
    assert_eq!(phase_six_table_count, 3);

    let backup_settings_count: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM backup_settings WHERE id = 1",
            [],
            |row| row.get(0),
        )
        .expect("backup settings row");
    assert_eq!(backup_settings_count, 1);
}

#[test]
fn database_creates_nested_parent_directories() {
    let directory = TempDir::new().expect("create temp directory");
    let database_path = directory
        .path()
        .join("nested")
        .join("application-data")
        .join("xushu.db");

    let database = Database::open(&database_path).expect("open nested database");

    assert!(database_path.exists());
    assert_eq!(database.health().expect("health").schema_version, 7);
}

#[cfg(unix)]
#[test]
fn database_requires_a_protected_directory_and_private_new_file() {
    use std::fs;
    use std::os::unix::fs::PermissionsExt;

    let fixture = TempDir::new().unwrap();
    let shared = fixture.path().join("shared");
    fs::create_dir(&shared).unwrap();
    fs::set_permissions(&shared, fs::Permissions::from_mode(0o777)).unwrap();
    let invalid_database = shared.join("xushu.db");
    assert!(matches!(
        Database::open(&invalid_database),
        Err(CoreError::UnsafePath)
    ));
    assert!(!invalid_database.exists());

    let path = fixture.path().join("app-data/xushu.db");
    let database = Database::open(&path).unwrap();
    assert!(database.health().unwrap().ok);
    assert_eq!(
        fs::metadata(&path).unwrap().permissions().mode() & 0o777,
        0o600
    );
    drop(database);
    assert!(Database::open(&path).unwrap().health().unwrap().ok);
}
