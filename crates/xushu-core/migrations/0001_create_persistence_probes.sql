CREATE TABLE persistence_probes (
    id TEXT PRIMARY KEY NOT NULL,
    label TEXT NOT NULL,
    content TEXT NOT NULL,
    created_at TEXT NOT NULL
);

CREATE INDEX idx_persistence_probes_created_at
    ON persistence_probes (created_at DESC);