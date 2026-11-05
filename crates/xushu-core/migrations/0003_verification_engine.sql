CREATE TABLE verification_specs (
    id TEXT PRIMARY KEY NOT NULL,
    criterion_id TEXT NOT NULL REFERENCES acceptance_criteria(id) ON DELETE CASCADE,
    executable TEXT NOT NULL,
    args_json TEXT NOT NULL,
    cwd TEXT NOT NULL,
    timeout_seconds INTEGER NOT NULL CHECK (timeout_seconds BETWEEN 1 AND 3600),
    output_limit_bytes INTEGER NOT NULL CHECK (output_limit_bytes BETWEEN 1024 AND 1048576),
    success_rule TEXT NOT NULL CHECK (success_rule IN ('exit_code_zero')),
    revision INTEGER NOT NULL DEFAULT 1 CHECK (revision >= 1),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE UNIQUE INDEX idx_verification_specs_criterion
    ON verification_specs (criterion_id);

CREATE TABLE verification_runs (
    id TEXT PRIMARY KEY NOT NULL,
    task_id TEXT NOT NULL REFERENCES tasks(id) ON DELETE RESTRICT,
    criterion_id TEXT REFERENCES acceptance_criteria(id) ON DELETE SET NULL,
    criterion_description TEXT NOT NULL,
    spec_id TEXT REFERENCES verification_specs(id) ON DELETE SET NULL,
    spec_revision INTEGER NOT NULL CHECK (spec_revision >= 1),
    task_revision INTEGER NOT NULL CHECK (task_revision >= 1),
    criterion_revision INTEGER NOT NULL CHECK (criterion_revision >= 1),
    executable TEXT NOT NULL,
    resolved_executable TEXT,
    args_json TEXT NOT NULL,
    cwd TEXT NOT NULL,
    timeout_seconds INTEGER NOT NULL CHECK (timeout_seconds BETWEEN 1 AND 3600),
    output_limit_bytes INTEGER NOT NULL CHECK (output_limit_bytes BETWEEN 1024 AND 1048576),
    success_rule TEXT NOT NULL CHECK (success_rule IN ('exit_code_zero')),
    approved_at TEXT NOT NULL,
    started_at TEXT NOT NULL,
    finished_at TEXT,
    execution_status TEXT NOT NULL CHECK (
        execution_status IN (
            'running', 'completed', 'cancelled', 'timed_out', 'start_failed', 'interrupted'
        )
    ),
    exit_code INTEGER,
    duration_ms INTEGER CHECK (duration_ms IS NULL OR duration_ms >= 0),
    start_git_head TEXT,
    start_git_branch TEXT,
    start_git_detached INTEGER NOT NULL DEFAULT 0 CHECK (start_git_detached IN (0, 1)),
    start_git_unborn INTEGER NOT NULL DEFAULT 0 CHECK (start_git_unborn IN (0, 1)),
    start_git_dirty INTEGER NOT NULL DEFAULT 0 CHECK (start_git_dirty IN (0, 1)),
    start_code_fingerprint TEXT,
    start_fingerprint_complete INTEGER NOT NULL DEFAULT 0 CHECK (start_fingerprint_complete IN (0, 1)),
    start_code_state_captured INTEGER NOT NULL DEFAULT 0 CHECK (start_code_state_captured IN (0, 1)),
    start_changed_file_count INTEGER NOT NULL DEFAULT 0 CHECK (start_changed_file_count >= 0),
    end_git_head TEXT,
    end_git_branch TEXT,
    end_git_detached INTEGER NOT NULL DEFAULT 0 CHECK (end_git_detached IN (0, 1)),
    end_git_unborn INTEGER NOT NULL DEFAULT 0 CHECK (end_git_unborn IN (0, 1)),
    end_git_dirty INTEGER NOT NULL DEFAULT 0 CHECK (end_git_dirty IN (0, 1)),
    end_code_fingerprint TEXT,
    end_fingerprint_complete INTEGER NOT NULL DEFAULT 0 CHECK (end_fingerprint_complete IN (0, 1)),
    end_code_state_captured INTEGER NOT NULL DEFAULT 0 CHECK (end_code_state_captured IN (0, 1)),
    end_changed_file_count INTEGER NOT NULL DEFAULT 0 CHECK (end_changed_file_count >= 0),
    code_state_note TEXT,
    stdout_summary TEXT,
    stderr_summary TEXT,
    stdout_bytes INTEGER,
    stderr_bytes INTEGER,
    stdout_truncated INTEGER NOT NULL DEFAULT 0 CHECK (stdout_truncated IN (0, 1)),
    stderr_truncated INTEGER NOT NULL DEFAULT 0 CHECK (stderr_truncated IN (0, 1)),
    sensitive_flags TEXT NOT NULL DEFAULT '',
    output_digest TEXT,
    error_code TEXT,
    error_message TEXT,
    cancel_reason TEXT,
    created_at TEXT NOT NULL
);

CREATE INDEX idx_verification_runs_task_created
    ON verification_runs (task_id, created_at DESC);

CREATE INDEX idx_verification_runs_criterion_created
    ON verification_runs (criterion_id, created_at DESC);

CREATE INDEX idx_verification_runs_status
    ON verification_runs (execution_status);

CREATE TABLE evidence (
    id TEXT PRIMARY KEY NOT NULL,
    source_type TEXT NOT NULL CHECK (source_type IN ('verification_run')),
    task_id TEXT NOT NULL REFERENCES tasks(id) ON DELETE RESTRICT,
    run_id TEXT REFERENCES verification_runs(id) ON DELETE SET NULL,
    summary TEXT NOT NULL,
    redacted_output TEXT,
    content_digest TEXT NOT NULL,
    sensitive_flags TEXT NOT NULL DEFAULT '',
    created_at TEXT NOT NULL
);

CREATE INDEX idx_evidence_task_created
    ON evidence (task_id, created_at DESC);

CREATE UNIQUE INDEX idx_evidence_run_id
    ON evidence (run_id)
    WHERE run_id IS NOT NULL;

CREATE TABLE verification_checks (
    id TEXT PRIMARY KEY NOT NULL,
    run_id TEXT NOT NULL REFERENCES verification_runs(id) ON DELETE RESTRICT,
    task_id TEXT NOT NULL REFERENCES tasks(id) ON DELETE RESTRICT,
    criterion_id TEXT REFERENCES acceptance_criteria(id) ON DELETE SET NULL,
    result TEXT NOT NULL CHECK (result IN ('pass', 'fail', 'not_run', 'inconclusive')),
    reason TEXT NOT NULL,
    evidence_id TEXT REFERENCES evidence(id) ON DELETE SET NULL,
    created_at TEXT NOT NULL
);

CREATE UNIQUE INDEX idx_verification_checks_run_id
    ON verification_checks (run_id);

CREATE INDEX idx_verification_checks_criterion_created
    ON verification_checks (criterion_id, created_at DESC);