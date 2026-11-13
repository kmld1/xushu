-- Phase 5：项目长期记忆、开发决策与跨会话恢复快照。
-- 所有记忆与决策均为项目本地数据；它们不能替代任务状态、验收证据或完成门禁。

CREATE TABLE project_memories (
    id TEXT PRIMARY KEY NOT NULL,
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE RESTRICT,
    category TEXT NOT NULL CHECK (
        category IN (
            'goal', 'constraint', 'architecture', 'convention',
            'fact', 'risk', 'follow_up', 'knowledge'
        )
    ),
    title TEXT NOT NULL,
    content TEXT NOT NULL,
    source_type TEXT NOT NULL CHECK (
        source_type IN ('user', 'system', 'git', 'verification', 'hook', 'inference')
    ),
    confirmation_status TEXT NOT NULL CHECK (
        confirmation_status IN ('unconfirmed', 'confirmed')
    ),
    validity_status TEXT NOT NULL CHECK (validity_status IN ('active', 'archived')),
    linked_task_id TEXT REFERENCES tasks(id) ON DELETE SET NULL,
    linked_evidence_id TEXT REFERENCES evidence(id) ON DELETE SET NULL,
    revision INTEGER NOT NULL DEFAULT 1 CHECK (revision >= 1),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    confirmed_at TEXT,
    archived_at TEXT,
    CHECK ((confirmation_status = 'confirmed') = (confirmed_at IS NOT NULL)),
    CHECK ((validity_status = 'archived') = (archived_at IS NOT NULL))
);

CREATE INDEX idx_project_memories_project_updated
    ON project_memories (project_id, updated_at DESC);

CREATE INDEX idx_project_memories_project_category
    ON project_memories (project_id, category, validity_status);

CREATE INDEX idx_project_memories_linked_task
    ON project_memories (linked_task_id);

CREATE TABLE project_memory_revisions (
    id TEXT PRIMARY KEY NOT NULL,
    memory_id TEXT NOT NULL REFERENCES project_memories(id) ON DELETE CASCADE,
    revision INTEGER NOT NULL CHECK (revision >= 1),
    category TEXT NOT NULL CHECK (
        category IN (
            'goal', 'constraint', 'architecture', 'convention',
            'fact', 'risk', 'follow_up', 'knowledge'
        )
    ),
    title TEXT NOT NULL,
    content TEXT NOT NULL,
    source_type TEXT NOT NULL CHECK (
        source_type IN ('user', 'system', 'git', 'verification', 'hook', 'inference')
    ),
    confirmation_status TEXT NOT NULL CHECK (
        confirmation_status IN ('unconfirmed', 'confirmed')
    ),
    validity_status TEXT NOT NULL CHECK (validity_status IN ('active', 'archived')),
    linked_task_id TEXT,
    linked_evidence_id TEXT,
    change_note TEXT,
    changed_at TEXT NOT NULL,
    UNIQUE (memory_id, revision)
);

CREATE INDEX idx_project_memory_revisions_memory_revision
    ON project_memory_revisions (memory_id, revision DESC);

CREATE TABLE decisions (
    id TEXT PRIMARY KEY NOT NULL,
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE RESTRICT,
    task_id TEXT REFERENCES tasks(id) ON DELETE SET NULL,
    title TEXT NOT NULL,
    problem TEXT NOT NULL,
    context TEXT NOT NULL,
    alternatives_json TEXT NOT NULL,
    decision TEXT NOT NULL,
    rationale TEXT NOT NULL,
    risks TEXT NOT NULL,
    status TEXT NOT NULL CHECK (
        status IN ('draft', 'confirmed', 'superseded', 'deprecated')
    ),
    supersedes_id TEXT REFERENCES decisions(id) ON DELETE SET NULL,
    superseded_by_id TEXT REFERENCES decisions(id) ON DELETE SET NULL,
    revision INTEGER NOT NULL DEFAULT 1 CHECK (revision >= 1),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    confirmed_at TEXT,
    deprecated_at TEXT,
    CHECK (
        (status IN ('confirmed', 'superseded') AND confirmed_at IS NOT NULL)
        OR (status = 'draft' AND confirmed_at IS NULL)
        OR status = 'deprecated'
    ),
    CHECK ((status = 'deprecated') = (deprecated_at IS NOT NULL)),
    CHECK (supersedes_id IS NULL OR supersedes_id <> id),
    CHECK (superseded_by_id IS NULL OR superseded_by_id <> id),
    CHECK ((status = 'superseded') = (superseded_by_id IS NOT NULL))
);

CREATE INDEX idx_decisions_project_status_updated
    ON decisions (project_id, status, updated_at DESC);

CREATE INDEX idx_decisions_task
    ON decisions (task_id);

CREATE INDEX idx_decisions_supersedes
    ON decisions (supersedes_id);

CREATE INDEX idx_decisions_superseded_by
    ON decisions (superseded_by_id);

CREATE TABLE decision_revisions (
    id TEXT PRIMARY KEY NOT NULL,
    decision_id TEXT NOT NULL REFERENCES decisions(id) ON DELETE CASCADE,
    revision INTEGER NOT NULL CHECK (revision >= 1),
    title TEXT NOT NULL,
    problem TEXT NOT NULL,
    context TEXT NOT NULL,
    alternatives_json TEXT NOT NULL,
    decision TEXT NOT NULL,
    rationale TEXT NOT NULL,
    risks TEXT NOT NULL,
    status TEXT NOT NULL CHECK (
        status IN ('draft', 'confirmed', 'superseded', 'deprecated')
    ),
    supersedes_id TEXT,
    superseded_by_id TEXT,
    change_note TEXT,
    changed_at TEXT NOT NULL,
    UNIQUE (decision_id, revision)
);

CREATE INDEX idx_decision_revisions_decision_revision
    ON decision_revisions (decision_id, revision DESC);

CREATE TABLE project_recovery_preferences (
    project_id TEXT PRIMARY KEY NOT NULL REFERENCES projects(id) ON DELETE RESTRICT,
    next_task_id TEXT REFERENCES tasks(id) ON DELETE SET NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE recovery_snapshots (
    id TEXT PRIMARY KEY NOT NULL,
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE RESTRICT,
    generated_at TEXT NOT NULL,
    fingerprint TEXT NOT NULL,
    markdown TEXT NOT NULL,
    package_json TEXT NOT NULL,
    task_count INTEGER NOT NULL DEFAULT 0 CHECK (task_count >= 0),
    memory_count INTEGER NOT NULL DEFAULT 0 CHECK (memory_count >= 0),
    decision_count INTEGER NOT NULL DEFAULT 0 CHECK (decision_count >= 0),
    created_at TEXT NOT NULL
);

CREATE INDEX idx_recovery_snapshots_project_generated
    ON recovery_snapshots (project_id, generated_at DESC);