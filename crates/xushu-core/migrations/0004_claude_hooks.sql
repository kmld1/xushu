CREATE TABLE claude_integrations (
    project_id TEXT PRIMARY KEY NOT NULL REFERENCES projects(id) ON DELETE RESTRICT,
    desired_enabled INTEGER NOT NULL DEFAULT 0 CHECK (desired_enabled IN (0, 1)),
    hook_version TEXT,
    hook_binary_path TEXT,
    hook_binary_digest TEXT,
    installed_at TEXT,
    uninstalled_at TEXT,
    last_checked_at TEXT,
    last_import_at TEXT,
    last_error_code TEXT,
    last_error_at TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE claude_sessions (
    id TEXT PRIMARY KEY NOT NULL,
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE RESTRICT,
    session_hash TEXT NOT NULL,
    started_at TEXT NOT NULL,
    last_event_at TEXT NOT NULL,
    ended_at TEXT,
    start_source TEXT NOT NULL CHECK (
        start_source IN ('startup', 'resume', 'clear', 'compact', 'fork', 'unknown')
    ),
    end_reason TEXT CHECK (
        end_reason IS NULL OR end_reason IN (
            'clear', 'resume', 'logout', 'prompt_input_exit', 'other', 'unknown'
        )
    ),
    start_observed INTEGER NOT NULL DEFAULT 0 CHECK (start_observed IN (0, 1)),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE UNIQUE INDEX idx_claude_sessions_project_hash
    ON claude_sessions (project_id, session_hash);

CREATE INDEX idx_claude_sessions_project_last_event
    ON claude_sessions (project_id, last_event_at DESC);

CREATE TABLE claude_tool_calls (
    id TEXT PRIMARY KEY NOT NULL,
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE RESTRICT,
    session_id TEXT NOT NULL REFERENCES claude_sessions(id) ON DELETE CASCADE,
    tool_use_hash TEXT NOT NULL,
    tool_name TEXT NOT NULL,
    tool_category TEXT NOT NULL CHECK (
        tool_category IN (
            'shell', 'file_read', 'file_write', 'search', 'web', 'agent', 'mcp', 'task', 'other'
        )
    ),
    status TEXT NOT NULL CHECK (status IN ('running', 'succeeded', 'failed', 'unknown')),
    failure_kind TEXT CHECK (
        failure_kind IS NULL OR failure_kind IN (
            'exit_code_nonzero', 'command_not_started', 'tool_error', 'timeout',
            'interrupted', 'unknown'
        )
    ),
    started_at TEXT,
    finished_at TEXT,
    duration_ms INTEGER CHECK (duration_ms IS NULL OR duration_ms >= 0),
    start_observed INTEGER NOT NULL DEFAULT 0 CHECK (start_observed IN (0, 1)),
    finish_observed INTEGER NOT NULL DEFAULT 0 CHECK (finish_observed IN (0, 1)),
    conflict INTEGER NOT NULL DEFAULT 0 CHECK (conflict IN (0, 1)),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE UNIQUE INDEX idx_claude_tool_calls_session_hash
    ON claude_tool_calls (session_id, tool_use_hash);

CREATE INDEX idx_claude_tool_calls_project_started
    ON claude_tool_calls (project_id, started_at DESC);

CREATE INDEX idx_claude_tool_calls_project_status
    ON claude_tool_calls (project_id, status);

CREATE TABLE claude_events (
    id TEXT PRIMARY KEY NOT NULL,
    delivery_id TEXT NOT NULL UNIQUE,
    dedupe_key TEXT UNIQUE,
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE RESTRICT,
    session_id TEXT NOT NULL REFERENCES claude_sessions(id) ON DELETE CASCADE,
    tool_call_id TEXT REFERENCES claude_tool_calls(id) ON DELETE SET NULL,
    event_type TEXT NOT NULL CHECK (
        event_type IN (
            'session_start', 'pre_tool_use', 'post_tool_use',
            'post_tool_use_failure', 'stop', 'session_end'
        )
    ),
    occurred_at TEXT NOT NULL,
    ingested_at TEXT NOT NULL,
    prompt_hash TEXT,
    tool_name TEXT,
    tool_category TEXT CHECK (
        tool_category IS NULL OR tool_category IN (
            'shell', 'file_read', 'file_write', 'search', 'web', 'agent', 'mcp', 'task', 'other'
        )
    ),
    failure_kind TEXT CHECK (
        failure_kind IS NULL OR failure_kind IN (
            'exit_code_nonzero', 'command_not_started', 'tool_error', 'timeout',
            'interrupted', 'unknown'
        )
    ),
    is_interrupt INTEGER CHECK (is_interrupt IS NULL OR is_interrupt IN (0, 1)),
    duration_ms INTEGER CHECK (duration_ms IS NULL OR duration_ms >= 0),
    source TEXT CHECK (
        source IS NULL OR source IN ('startup', 'resume', 'clear', 'compact', 'fork', 'unknown')
    ),
    end_reason TEXT CHECK (
        end_reason IS NULL OR end_reason IN (
            'clear', 'resume', 'logout', 'prompt_input_exit', 'other', 'unknown'
        )
    ),
    created_at TEXT NOT NULL
);

CREATE INDEX idx_claude_events_project_occurred
    ON claude_events (project_id, occurred_at DESC);

CREATE INDEX idx_claude_events_session_occurred
    ON claude_events (session_id, occurred_at DESC);