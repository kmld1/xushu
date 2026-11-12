ALTER TABLE claude_integrations
    ADD COLUMN capture_generation INTEGER NOT NULL DEFAULT 0 CHECK (capture_generation >= 0);

ALTER TABLE claude_integrations
    ADD COLUMN project_root_identity TEXT;
