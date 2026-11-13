-- Phase 6：一致性备份、灾难恢复、备份策略与数据保护审计。
-- 仅新增表，不修改既有业务表；备份与恢复操作不得改变任务状态或完成门禁。

CREATE TABLE backup_records (
    id TEXT PRIMARY KEY NOT NULL,
    backup_id TEXT NOT NULL UNIQUE,
    directory_path TEXT NOT NULL UNIQUE,
    format_version INTEGER NOT NULL CHECK (format_version >= 1),
    schema_version INTEGER NOT NULL CHECK (schema_version >= 1),
    app_version TEXT NOT NULL,
    origin TEXT NOT NULL CHECK (origin IN ('manual', 'automatic')),
    created_at TEXT NOT NULL,
    registered_at TEXT NOT NULL,
    data_size_bytes INTEGER NOT NULL CHECK (data_size_bytes >= 0),
    manifest_size_bytes INTEGER NOT NULL DEFAULT 0 CHECK (manifest_size_bytes >= 0),
    payload_sha256 TEXT NOT NULL,
    integrity_status TEXT NOT NULL CHECK (
        integrity_status IN ('verified', 'failed', 'unverified', 'missing')
    ),
    verified_at TEXT,
    last_error TEXT
);

CREATE INDEX idx_backup_records_created
    ON backup_records (created_at DESC);

CREATE INDEX idx_backup_records_origin_created
    ON backup_records (origin, created_at DESC);

CREATE TABLE backup_settings (
    id INTEGER PRIMARY KEY NOT NULL CHECK (id = 1),
    automatic_enabled INTEGER NOT NULL DEFAULT 0 CHECK (automatic_enabled IN (0, 1)),
    target_directory TEXT,
    interval_hours INTEGER NOT NULL DEFAULT 24 CHECK (interval_hours BETWEEN 1 AND 720),
    retention_count INTEGER NOT NULL DEFAULT 5 CHECK (retention_count BETWEEN 1 AND 50),
    last_automatic_at TEXT,
    last_automatic_error_code TEXT,
    last_automatic_error_at TEXT,
    updated_at TEXT NOT NULL
);

INSERT INTO backup_settings (id, updated_at)
VALUES (1, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'));

CREATE TABLE restore_operations (
    id TEXT PRIMARY KEY NOT NULL,
    backup_id TEXT NOT NULL,
    backup_directory TEXT NOT NULL,
    started_at TEXT NOT NULL,
    finished_at TEXT,
    outcome TEXT NOT NULL CHECK (
        outcome IN ('completed', 'rolled_back', 'failed', 'rollback_failed')
    ),
    switch_verified INTEGER NOT NULL DEFAULT 0 CHECK (switch_verified IN (0, 1)),
    rollback_status TEXT NOT NULL CHECK (
        rollback_status IN ('not_needed', 'available', 'used', 'failed')
    ),
    detail_json TEXT NOT NULL DEFAULT '{}',
    created_at TEXT NOT NULL
);

CREATE INDEX idx_restore_operations_started
    ON restore_operations (started_at DESC);