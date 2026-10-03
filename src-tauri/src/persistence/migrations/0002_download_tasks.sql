-- M2: persistent download tasks (crash recovery + resume).
--
-- One row per download task. This is the LOGICAL state (M2 §5): the
-- filesystem (.part size) remains the source of truth for physical
-- progress; `downloaded_bytes` is a checkpoint hint that must never
-- exceed the physically durable file position (enforced by recovery
-- reconciliation, not by this schema).
--
-- Timestamps are UTC Unix epoch milliseconds. NULL total_bytes means
-- unknown length (chunked/absent header) — never stored as 0.

CREATE TABLE download_tasks (
    id                 TEXT PRIMARY KEY,
    source_url         TEXT NOT NULL,
    allow_private      INTEGER NOT NULL DEFAULT 0,
    destination_dir    TEXT NOT NULL,
    filename           TEXT NOT NULL,
    part_path          TEXT NOT NULL,
    final_path         TEXT NOT NULL,
    overwrite_policy   TEXT NOT NULL CHECK (overwrite_policy IN ('fail', 'overwrite', 'auto_rename')),
    state              TEXT NOT NULL CHECK (state IN ('created', 'ready', 'running', 'paused', 'completed', 'failed', 'cancelled')),
    total_bytes        INTEGER,
    downloaded_bytes   INTEGER NOT NULL DEFAULT 0 CHECK (downloaded_bytes >= 0),
    content_type       TEXT,
    etag               TEXT,
    last_modified      TEXT,
    checksum_algorithm TEXT,
    checksum_expected  TEXT,
    retry_count        INTEGER NOT NULL DEFAULT 0 CHECK (retry_count >= 0),
    last_error_code    TEXT,
    last_error_message TEXT,
    recovery_reason    TEXT,
    created_at         INTEGER NOT NULL,
    updated_at         INTEGER NOT NULL,
    started_at         INTEGER,
    completed_at       INTEGER
);

CREATE INDEX idx_download_tasks_state ON download_tasks (state);
CREATE INDEX idx_download_tasks_updated_at ON download_tasks (updated_at);
