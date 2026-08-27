BEGIN IMMEDIATE;

CREATE TABLE collection_roots (
    id TEXT PRIMARY KEY,
    path_key TEXT NOT NULL UNIQUE,
    path_display TEXT NOT NULL,
    path_bytes BLOB NOT NULL,
    path_encoding TEXT NOT NULL CHECK (
        path_encoding IN ('unix_bytes', 'windows_utf16le', 'utf8')
    ),
    platform_id TEXT REFERENCES platforms(id) ON DELETE SET NULL,
    recursive INTEGER NOT NULL DEFAULT 1 CHECK (recursive IN (0, 1)),
    last_scan_started_at TEXT,
    last_scan_completed_at TEXT,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
) STRICT, WITHOUT ROWID;

CREATE TABLE local_files (
    id TEXT PRIMARY KEY,
    root_id TEXT NOT NULL REFERENCES collection_roots(id) ON DELETE CASCADE,
    source_record_id TEXT NOT NULL UNIQUE REFERENCES source_records(id) ON DELETE CASCADE,
    artifact_id TEXT NOT NULL REFERENCES artifacts(id),
    relative_path_key TEXT NOT NULL,
    relative_path_display TEXT NOT NULL,
    relative_path_bytes BLOB NOT NULL,
    path_encoding TEXT NOT NULL CHECK (
        path_encoding IN ('unix_bytes', 'windows_utf16le', 'utf8')
    ),
    archive_member TEXT NOT NULL DEFAULT '',
    byte_size INTEGER NOT NULL CHECK (byte_size >= 0),
    modified_unix_ns INTEGER,
    availability TEXT NOT NULL DEFAULT 'present' CHECK (
        availability IN ('present', 'missing')
    ),
    first_seen_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    last_seen_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE (root_id, relative_path_key, archive_member)
) STRICT, WITHOUT ROWID;

CREATE INDEX local_files_artifact_idx ON local_files(artifact_id);
CREATE INDEX local_files_availability_idx ON local_files(root_id, availability);

INSERT INTO schema_migrations (version, name, applied_at)
VALUES (3, 'local collection inventory', '1970-01-01T00:00:00Z');

COMMIT;
