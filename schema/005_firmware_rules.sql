BEGIN IMMEDIATE;

CREATE TABLE firmware_sources (
    id TEXT PRIMARY KEY,
    transport TEXT NOT NULL CHECK (
        transport IN ('minerva', 'github_release', 'direct', 'manual')
    ),
    locator_url TEXT,
    torrent_file TEXT,
    path_prefix TEXT,
    metadata_json TEXT NOT NULL DEFAULT '{}' CHECK (json_valid(metadata_json)),
    CHECK (
        (transport = 'minerva' AND torrent_file IS NOT NULL AND path_prefix IS NOT NULL)
        OR (transport IN ('github_release', 'direct') AND locator_url LIKE 'https://%')
        OR transport = 'manual'
    )
) STRICT, WITHOUT ROWID;

CREATE TABLE firmware_rules (
    rule_key TEXT PRIMARY KEY,
    runtime_kind TEXT NOT NULL,
    runtime_name TEXT NOT NULL,
    emulator_id TEXT NOT NULL REFERENCES emulators(id) ON DELETE CASCADE,
    platform_id TEXT NOT NULL REFERENCES platforms(id) ON DELETE CASCADE,
    source_id TEXT NOT NULL REFERENCES firmware_sources(id),
    source_package_name TEXT NOT NULL,
    target_subdir TEXT NOT NULL DEFAULT '',
    install_mode TEXT NOT NULL CHECK (install_mode IN ('merge_tree', 'copy_archive')),
    target_strategy TEXT NOT NULL CHECK (
        target_strategy IN ('runtime_dir', 'launch_scoped', 'mame_rompath', 'manual_import', 'managed_import')
    ),
    required INTEGER NOT NULL CHECK (required IN (0, 1)),
    supports_hle_fallback INTEGER NOT NULL CHECK (supports_hle_fallback IN (0, 1)),
    notes TEXT NOT NULL,
    UNIQUE (
        runtime_kind, runtime_name, platform_id, source_id,
        source_package_name, target_subdir, install_mode
    )
) STRICT, WITHOUT ROWID;

CREATE INDEX firmware_rules_runtime_idx
ON firmware_rules(runtime_kind, runtime_name, platform_id);

CREATE INDEX firmware_rules_source_idx ON firmware_rules(source_id, source_package_name);

INSERT INTO schema_migrations (version, name, applied_at)
VALUES (5, 'runtime scoped firmware rules', '1970-01-01T00:00:00Z');

COMMIT;
