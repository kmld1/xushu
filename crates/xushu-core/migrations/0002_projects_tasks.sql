CREATE TABLE projects (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    root_path TEXT NOT NULL,
    root_path_key TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    archived_at TEXT
);

CREATE UNIQUE INDEX idx_projects_root_path_key
    ON projects (root_path_key);

CREATE INDEX idx_projects_archived_at
    ON projects (archived_at);

CREATE TABLE tasks (
    id TEXT PRIMARY KEY NOT NULL,
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE RESTRICT,
    title TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    priority TEXT NOT NULL CHECK (priority IN ('low', 'medium', 'high', 'urgent')),
    status TEXT NOT NULL CHECK (
        status IN ('draft', 'ready', 'running', 'awaiting_verification', 'done', 'blocked', 'cancelled')
    ),
    revision INTEGER NOT NULL DEFAULT 1 CHECK (revision >= 1),
    block_reason TEXT,
    archived_at TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX idx_tasks_project_status
    ON tasks (project_id, status);

CREATE INDEX idx_tasks_project_archived
    ON tasks (project_id, archived_at);

CREATE INDEX idx_tasks_updated_at
    ON tasks (updated_at DESC);

CREATE TABLE acceptance_criteria (
    id TEXT PRIMARY KEY NOT NULL,
    task_id TEXT NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
    description TEXT NOT NULL,
    kind TEXT NOT NULL CHECK (kind IN ('auto', 'manual')),
    required INTEGER NOT NULL CHECK (required IN (0, 1)),
    revision INTEGER NOT NULL DEFAULT 1 CHECK (revision >= 1),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX idx_acceptance_criteria_task_id
    ON acceptance_criteria (task_id);

CREATE TABLE task_transitions (
    id TEXT PRIMARY KEY NOT NULL,
    task_id TEXT NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
    from_status TEXT CHECK (
        from_status IS NULL OR from_status IN (
            'draft', 'ready', 'running', 'awaiting_verification', 'done', 'blocked', 'cancelled'
        )
    ),
    to_status TEXT NOT NULL CHECK (
        to_status IN ('draft', 'ready', 'running', 'awaiting_verification', 'done', 'blocked', 'cancelled')
    ),
    reason TEXT,
    task_revision INTEGER NOT NULL CHECK (task_revision >= 1),
    created_at TEXT NOT NULL
);

CREATE INDEX idx_task_transitions_task_created
    ON task_transitions (task_id, created_at);

CREATE TABLE manual_confirmations (
    id TEXT PRIMARY KEY NOT NULL,
    criterion_id TEXT NOT NULL REFERENCES acceptance_criteria(id) ON DELETE CASCADE,
    criterion_revision INTEGER NOT NULL CHECK (criterion_revision >= 1),
    task_revision INTEGER NOT NULL CHECK (task_revision >= 1),
    confirmed_at TEXT NOT NULL,
    note TEXT
);

CREATE UNIQUE INDEX idx_manual_confirmations_revision_unique
    ON manual_confirmations (criterion_id, criterion_revision, task_revision);

CREATE INDEX idx_manual_confirmations_criterion_id
    ON manual_confirmations (criterion_id);