//! 备份清单格式与完整性校验。
//!
//! 清单位于备份目录中，只描述数据库快照，不包含可执行代码、绝对路径或密钥。
//! 校验和证明“文件内容与清单一致”，不证明备份来源可信；恢复预检还会实际
//! 打开隔离副本执行 SQLite 完整性与业务关系检查。

use std::path::Path;
use std::time::{Duration, Instant};

use chrono::DateTime;
use rusqlite::{config::DbConfig, Connection, OpenFlags};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::db::{run_migrations, Database, MIGRATIONS};
use crate::error::CoreError;

use super::paths::{join_relative, read_bounded_file, sha256_file};

pub const BACKUP_FORMAT_VERSION: u32 = 1;
pub const BACKUP_PRODUCT: &str = "XUSHU";
pub const BACKUP_SCOPE_DATABASE: &str = "database";
pub const MANIFEST_FILE_NAME: &str = "manifest.json";
pub const PAYLOAD_FILE_NAME: &str = "xushu.db";
pub const MAX_MANIFEST_BYTES: u64 = 256 * 1024;
pub const MAX_BACKUP_BYTES: u64 = 2 * 1024 * 1024 * 1024;
const ISOLATED_SQLITE_TIMEOUT: Duration = Duration::from_secs(60);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BackupOrigin {
    Manual,
    Automatic,
}

impl BackupOrigin {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Manual => "manual",
            Self::Automatic => "automatic",
        }
    }

    pub fn parse(value: &str) -> Result<Self, CoreError> {
        match value {
            "manual" => Ok(Self::Manual),
            "automatic" => Ok(Self::Automatic),
            _ => Err(CoreError::Compatibility("备份来源标记无效".to_owned())),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BackupFileEntry {
    pub path: String,
    pub size: u64,
    pub sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BackupManifest {
    pub format_version: u32,
    pub product: String,
    pub backup_id: String,
    pub app_version: String,
    pub schema_version: i64,
    pub created_at: String,
    pub origin: BackupOrigin,
    pub scope: String,
    pub encrypted: bool,
    pub complete: bool,
    pub files: Vec<BackupFileEntry>,
}

#[derive(Debug, Clone)]
pub struct ManifestRead {
    pub manifest: BackupManifest,
    pub manifest_size: u64,
    pub manifest_sha256: String,
}

impl BackupManifest {
    pub fn payload(&self) -> &BackupFileEntry {
        &self.files[0]
    }
}

pub fn build_manifest(
    backup_id: &str,
    app_version: &str,
    schema_version: i64,
    created_at: &str,
    origin: BackupOrigin,
    payload_size: u64,
    payload_sha256: &str,
) -> BackupManifest {
    BackupManifest {
        format_version: BACKUP_FORMAT_VERSION,
        product: BACKUP_PRODUCT.to_owned(),
        backup_id: backup_id.to_owned(),
        app_version: app_version.to_owned(),
        schema_version,
        created_at: created_at.to_owned(),
        origin,
        scope: BACKUP_SCOPE_DATABASE.to_owned(),
        encrypted: false,
        complete: true,
        files: vec![BackupFileEntry {
            path: PAYLOAD_FILE_NAME.to_owned(),
            size: payload_size,
            sha256: payload_sha256.to_owned(),
        }],
    }
}

pub fn manifest_bytes(manifest: &BackupManifest) -> Result<Vec<u8>, CoreError> {
    let mut bytes = serde_json::to_vec_pretty(manifest)
        .map_err(|_| CoreError::Backup("无法序列化备份清单".to_owned()))?;
    bytes.push(b'\n');
    Ok(bytes)
}

pub fn validate_manifest(manifest: &BackupManifest) -> Result<(), CoreError> {
    if manifest.format_version != BACKUP_FORMAT_VERSION {
        return Err(CoreError::Compatibility(format!(
            "不支持的备份格式版本 {}",
            manifest.format_version
        )));
    }
    if manifest.product != BACKUP_PRODUCT {
        return Err(CoreError::Compatibility("备份产品标识不匹配".to_owned()));
    }
    if uuid::Uuid::parse_str(&manifest.backup_id).is_err() {
        return Err(CoreError::Compatibility("备份 ID 格式无效".to_owned()));
    }
    if manifest.app_version.trim().is_empty()
        || manifest.app_version.len() > 64
        || manifest.app_version.chars().any(char::is_control)
    {
        return Err(CoreError::Compatibility(
            "备份中的应用版本标记无效".to_owned(),
        ));
    }
    if manifest.schema_version < 1 {
        return Err(CoreError::Compatibility("备份数据库版本无效".to_owned()));
    }
    if DateTime::parse_from_rfc3339(&manifest.created_at).is_err() {
        return Err(CoreError::Compatibility("备份创建时间无效".to_owned()));
    }
    if manifest.scope != BACKUP_SCOPE_DATABASE {
        return Err(CoreError::Compatibility("不支持的备份数据范围".to_owned()));
    }
    if manifest.encrypted {
        return Err(CoreError::Compatibility(
            "当前版本不支持加密备份，已拒绝导入".to_owned(),
        ));
    }
    if !manifest.complete {
        return Err(CoreError::Compatibility(
            "备份未标记为完整，不能用于恢复".to_owned(),
        ));
    }
    if manifest.files.len() != 1 {
        return Err(CoreError::Compatibility(
            "备份清单必须且只能包含一个数据库文件".to_owned(),
        ));
    }

    let payload = manifest.payload();
    if payload.path != PAYLOAD_FILE_NAME {
        return Err(CoreError::Compatibility(
            "备份清单包含不支持的相对路径".to_owned(),
        ));
    }
    join_relative(Path::new("."), &payload.path)?;
    if payload.size == 0 || payload.size > MAX_BACKUP_BYTES {
        return Err(CoreError::Compatibility(
            "备份数据文件大小超出允许范围".to_owned(),
        ));
    }
    let sha256 = payload.sha256.to_ascii_lowercase();
    if sha256.len() != 64 || !sha256.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(CoreError::Compatibility(
            "备份数据文件校验和格式无效".to_owned(),
        ));
    }

    Ok(())
}

pub fn read_manifest(directory: &Path) -> Result<ManifestRead, CoreError> {
    if directory
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.to_ascii_lowercase().starts_with(".partial-"))
    {
        return Err(CoreError::Backup(
            "备份尚未发布，不能使用临时目录".to_owned(),
        ));
    }
    read_staged_manifest(directory)
}

pub(super) fn read_staged_manifest(directory: &Path) -> Result<ManifestRead, CoreError> {
    let bytes = read_bounded_file(&directory.join(MANIFEST_FILE_NAME), MAX_MANIFEST_BYTES)
        .map_err(|error| match error {
            CoreError::Io(_) => CoreError::Backup("备份清单不存在或无法读取".to_owned()),
            other => other,
        })?;
    let manifest: BackupManifest = serde_json::from_slice(&bytes)
        .map_err(|_| CoreError::Compatibility("备份清单无法解析或包含未知字段".to_owned()))?;
    validate_manifest(&manifest)?;

    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    let manifest_sha256 = super::paths::to_hex(&hasher.finalize());
    Ok(ManifestRead {
        manifest,
        manifest_size: bytes.len() as u64,
        manifest_sha256,
    })
}

pub fn verify_payload(
    directory: &Path,
    manifest: &BackupManifest,
) -> Result<(u64, String), CoreError> {
    let payload = manifest.payload();
    let path = join_relative(directory, &payload.path)?;
    let (sha256, size) = sha256_file(&path, MAX_BACKUP_BYTES).map_err(|error| match error {
        CoreError::Io(_) => CoreError::Backup("备份数据文件不存在或无法读取".to_owned()),
        other => other,
    })?;
    if size != payload.size {
        return Err(CoreError::Backup("备份数据文件大小与清单不一致".to_owned()));
    }
    if sha256 != payload.sha256.to_ascii_lowercase() {
        return Err(CoreError::Backup("备份数据文件校验和不一致".to_owned()));
    }
    Ok((size, sha256))
}

pub(super) fn open_isolated_payload(
    path: &Path,
    schema_version: i64,
) -> Result<Database, CoreError> {
    let mut source = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_WRITE)?;
    source.set_db_config(DbConfig::SQLITE_DBCONFIG_TRUSTED_SCHEMA, false)?;
    source.set_db_config(DbConfig::SQLITE_DBCONFIG_DEFENSIVE, true)?;
    let deadline = Instant::now() + ISOLATED_SQLITE_TIMEOUT;
    source.progress_handler(10_000, Some(move || Instant::now() >= deadline))?;
    let migrations = MIGRATIONS
        .iter()
        .position(|migration| migration.version == schema_version)
        .map(|index| &MIGRATIONS[..=index])
        .ok_or_else(|| CoreError::Compatibility("备份数据库版本不受支持".to_owned()))?;
    let declared_schema =
        source.pragma_query_value(None, "user_version", |row| row.get::<_, i64>(0))?;
    if declared_schema != schema_version {
        return Err(CoreError::Compatibility(
            "数据库 Schema 版本与备份清单不一致".to_owned(),
        ));
    }
    let mut expected = Connection::open_in_memory()?;
    run_migrations(&mut expected, migrations)?;
    let expected_schema = {
        let mut statement = expected.prepare(SCHEMA_SQL)?;
        let rows = statement.query_map([], read_schema_row)?;
        rows.collect::<Result<Vec<_>, _>>()?
    };
    {
        let mut statement = source.prepare(SCHEMA_SQL)?;
        let mut rows = statement.query([])?;
        for expected_row in expected_schema {
            if rows.next()?.map(read_schema_row).transpose()?.as_ref() != Some(&expected_row) {
                return Err(CoreError::Compatibility(
                    "备份数据库包含未知或更改的 Schema 定义".to_owned(),
                ));
            }
        }
        if rows.next()?.is_some() {
            return Err(CoreError::Compatibility(
                "备份数据库包含额外的 Schema 对象".to_owned(),
            ));
        }
    }
    {
        let mut statement =
            source.prepare("SELECT version, name FROM schema_migrations ORDER BY version")?;
        let mut rows = statement.query([])?;
        for migration in migrations {
            let row = rows
                .next()?
                .ok_or_else(|| CoreError::Compatibility("备份数据库迁移历史不完整".to_owned()))?;
            if row.get::<_, i64>(0)? != migration.version
                || row.get::<_, String>(1)? != migration.name
            {
                return Err(CoreError::Compatibility(
                    "备份数据库迁移历史与清单不一致".to_owned(),
                ));
            }
        }
        if rows.next()?.is_some() {
            return Err(CoreError::Compatibility(
                "备份数据库含有额外迁移记录".to_owned(),
            ));
        }
    }
    Database::from_connection(path, source)
}

const SCHEMA_SQL: &str = "SELECT type, name, tbl_name, sql FROM sqlite_schema ORDER BY type, name";

fn read_schema_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<(String, String, String, Option<String>)> {
    Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn valid_manifest() -> BackupManifest {
        build_manifest(
            "11111111-1111-4111-8111-111111111111",
            "0.1.0",
            7,
            "2026-11-13T10:00:00Z",
            BackupOrigin::Manual,
            10,
            &"a".repeat(64),
        )
    }

    #[test]
    fn valid_manifest_round_trips() {
        let manifest = valid_manifest();
        validate_manifest(&manifest).expect("valid manifest");
        let bytes = manifest_bytes(&manifest).expect("serialize");
        let parsed: BackupManifest = serde_json::from_slice(&bytes).expect("parse");
        assert_eq!(parsed, manifest);
    }

    #[test]
    fn unknown_fields_and_future_versions_are_rejected() {
        let mut manifest = valid_manifest();
        manifest.format_version = 2;
        assert!(validate_manifest(&manifest).is_err());

        let mut value = serde_json::to_value(valid_manifest()).expect("json");
        value["unexpected"] = serde_json::json!(true);
        let parsed: Result<BackupManifest, _> = serde_json::from_value(value);
        assert!(parsed.is_err());
    }

    #[test]
    fn missing_fields_and_absolute_payload_paths_are_rejected() {
        let mut value = serde_json::to_value(valid_manifest()).unwrap();
        value.as_object_mut().unwrap().remove("backupId");
        assert!(serde_json::from_value::<BackupManifest>(value).is_err());
        let mut manifest = valid_manifest();
        for path in ["/xushu.db", r"D:\fixture\xushu.db"] {
            manifest.files[0].path = path.to_owned();
            assert!(validate_manifest(&manifest).is_err());
        }
    }

    #[test]
    fn invalid_manifest_encoding_is_rejected() {
        let directory = TempDir::new().expect("temp dir");
        fs::write(
            directory.path().join(MANIFEST_FILE_NAME),
            [0xff_u8, 0xfe, 0x00, 0x7b],
        )
        .expect("write invalid manifest");
        let error = read_manifest(directory.path()).expect_err("invalid encoding must fail");
        assert!(matches!(error, CoreError::Compatibility(_)));
    }

    #[test]
    fn oversized_payload_is_rejected_by_capacity_limit() {
        let mut manifest = valid_manifest();
        manifest.files[0].size = MAX_BACKUP_BYTES + 1;
        assert!(validate_manifest(&manifest).is_err());
    }

    #[test]
    fn traversal_and_duplicate_files_are_rejected() {
        let mut manifest = valid_manifest();
        manifest.files[0].path = "../xushu.db".to_owned();
        assert!(validate_manifest(&manifest).is_err());

        let mut manifest = valid_manifest();
        manifest.files.push(manifest.files[0].clone());
        assert!(validate_manifest(&manifest).is_err());
    }

    #[test]
    fn payload_hash_mismatch_is_detected() {
        let directory = TempDir::new().expect("temp dir");
        let manifest = valid_manifest();
        fs::write(directory.path().join(PAYLOAD_FILE_NAME), b"0123456789").expect("write payload");
        let error = verify_payload(directory.path(), &manifest).expect_err("hash mismatch");
        assert!(matches!(error, CoreError::Backup(_)));

        let mut good = valid_manifest();
        good.files[0].sha256 =
            "2a8a8b3a0c50d5a3d0f0d9b1b2c3d4e5f60718293a4b5c6d7e8f901234567890".to_owned();
        let actual = sha256_file(&directory.path().join(PAYLOAD_FILE_NAME), MAX_BACKUP_BYTES)
            .expect("hash file")
            .0;
        good.files[0].sha256 = actual;
        verify_payload(directory.path(), &good).expect("hash match");
    }
}
