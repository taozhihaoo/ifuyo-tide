-- M0 initial schema.
-- Timestamps are UTC Unix epoch milliseconds (INTEGER).
-- IDs are UUIDv4 strings (TEXT). States use canonical snake_case TEXT
-- values constrained by CHECK — no JSON blobs, no mixed time formats.

CREATE TABLE sources (
    source_id    TEXT PRIMARY KEY,
    source_type  TEXT NOT NULL CHECK (source_type IN ('http', 'torrent', 'magnet', 'local_file')),
    display_name TEXT NOT NULL,
    created_at   INTEGER NOT NULL
);

CREATE TABLE tasks (
    task_id    TEXT PRIMARY KEY,
    source_id  TEXT NOT NULL REFERENCES sources (source_id),
    name       TEXT NOT NULL,
    state      TEXT NOT NULL CHECK (state IN ('created', 'ready', 'paused', 'running', 'completed', 'failed', 'cancelled')),
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE INDEX idx_tasks_state ON tasks (state);
CREATE INDEX idx_tasks_updated_at ON tasks (updated_at);

CREATE TABLE settings (
    key        TEXT PRIMARY KEY,
    value      TEXT NOT NULL,
    updated_at INTEGER NOT NULL
);
