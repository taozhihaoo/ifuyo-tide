-- M3: torrent tasks and file selection persistence.
--
-- Layering (M3 §14): SQLite stores Tide's product-level task state;
-- librqbit owns piece state / fast resume / storage state. This table
-- must be interpretable WITHOUT librqbit.
--
-- `info_hash` is the torrent identity (M3 §38) but `task_id` remains
-- the Tide task identity — one active task per info_hash (M3 §17).

CREATE TABLE torrent_tasks (
    id                 TEXT PRIMARY KEY,
    info_hash          TEXT,
    source_kind        TEXT NOT NULL CHECK (source_kind IN ('magnet', 'torrent_file')),
    magnet_uri         TEXT,
    metadata_path      TEXT,
    metadata_sha256    TEXT,
    display_name       TEXT NOT NULL,
    storage_root       TEXT NOT NULL,
    state              TEXT NOT NULL CHECK (state IN ('created', 'ready', 'running', 'paused', 'completed', 'failed', 'cancelled')),
    phase              TEXT NOT NULL DEFAULT 'metadata_pending' CHECK (phase IN ('metadata_pending', 'metadata_failed', 'downloading', 'checking', 'seeding', 'completed', 'error')),
    total_bytes        INTEGER,
    downloaded_bytes   INTEGER NOT NULL DEFAULT 0 CHECK (downloaded_bytes >= 0),
    uploaded_bytes     INTEGER NOT NULL DEFAULT 0 CHECK (uploaded_bytes >= 0),
    selected_count     INTEGER,
    error_code         TEXT,
    error_message      TEXT,
    recovery_reason    TEXT,
    created_at         INTEGER NOT NULL,
    updated_at         INTEGER NOT NULL,
    completed_at       INTEGER
);

CREATE INDEX idx_torrent_tasks_state ON torrent_tasks (state);
CREATE UNIQUE INDEX idx_torrent_tasks_info_hash_active
    ON torrent_tasks (id, info_hash);

CREATE TABLE torrent_files (
    id             INTEGER PRIMARY KEY AUTOINCREMENT,
    task_id        TEXT NOT NULL REFERENCES torrent_tasks (id) ON DELETE CASCADE,
    info_hash      TEXT NOT NULL,
    file_index     INTEGER NOT NULL CHECK (file_index >= 0),
    relative_path  TEXT NOT NULL,
    size           INTEGER NOT NULL CHECK (size >= 0),
    selected       INTEGER NOT NULL DEFAULT 1,
    UNIQUE (task_id, info_hash, file_index)
);

CREATE INDEX idx_torrent_files_task ON torrent_files (task_id);
