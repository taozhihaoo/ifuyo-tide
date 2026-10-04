-- M6(上) §29/§30/§31: the local library. Two tables only — no scan
-- audit tables (§70: runtime state + root.last_scan_at is enough).
-- task_id is deliberately NULLABLE (§31: imported files have no task).

CREATE TABLE library_roots (
    id            TEXT PRIMARY KEY,
    path          TEXT NOT NULL UNIQUE,
    display_name  TEXT NOT NULL,
    enabled       INTEGER NOT NULL DEFAULT 1 CHECK (enabled IN (0, 1)),
    -- Runtime-ish status kept on the root (§70): idle | scanning | ok | partial | failed
    scan_status   TEXT NOT NULL DEFAULT 'idle'
                  CHECK (scan_status IN ('idle', 'scanning', 'ok', 'partial', 'failed')),
    last_scan_at  INTEGER,
    created_at    INTEGER NOT NULL,
    updated_at    INTEGER NOT NULL
);

CREATE TABLE library_items (
    id                   TEXT PRIMARY KEY,
    root_id              TEXT NOT NULL REFERENCES library_roots (id) ON DELETE CASCADE,
    -- §11: canonical, verified local path; UNIQUE per root (§21/§31 —
    -- scanning ten times must not create ten rows).
    canonical_path       TEXT NOT NULL,
    relative_path        TEXT NOT NULL,
    display_name         TEXT NOT NULL,
    extension            TEXT,
    media_category       TEXT NOT NULL CHECK (media_category IN
                         ('video', 'audio', 'image', 'subtitle', 'archive', 'document', 'other')),
    mime_type            TEXT,
    size_bytes           INTEGER NOT NULL CHECK (size_bytes >= 0),
    modified_at          INTEGER NOT NULL,
    availability_status  TEXT NOT NULL CHECK (availability_status IN
                         ('available', 'missing', 'inaccessible')),
    origin_type          TEXT NOT NULL CHECK (origin_type IN
                         ('download', 'imported_local', 'unknown')),
    task_id              TEXT,
    checksum             TEXT,
    last_verified_at     INTEGER NOT NULL,
    created_at           INTEGER NOT NULL,
    updated_at           INTEGER NOT NULL,
    UNIQUE (root_id, canonical_path)
);

CREATE INDEX idx_library_items_category ON library_items (media_category);
CREATE INDEX idx_library_items_name ON library_items (display_name);
CREATE INDEX idx_library_items_task ON library_items (task_id);
CREATE INDEX idx_library_items_availability ON library_items (availability_status);
