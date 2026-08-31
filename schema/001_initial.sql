PRAGMA foreign_keys = ON;

CREATE TABLE schema_migrations (
    version INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    applied_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
) STRICT, WITHOUT ROWID;

INSERT INTO schema_migrations (version, name, applied_at) VALUES
    (1, 'canonical identity and emulator catalog', '1970-01-01T00:00:00Z'),
    (2, 'pinned Libretro evidence pack', '1970-01-01T00:00:00Z'),
    (3, 'local collection inventory', '1970-01-01T00:00:00Z'),
    (4, 'host scoped emulator install sources', '1970-01-01T00:00:00Z'),
    (5, 'runtime scoped firmware rules', '1970-01-01T00:00:00Z');

CREATE TABLE build_metadata (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
) STRICT, WITHOUT ROWID;

CREATE TABLE providers (
    id TEXT PRIMARY KEY,
    slug TEXT NOT NULL UNIQUE,
    name TEXT NOT NULL,
    homepage TEXT,
    data_license TEXT,
    redistribution_policy TEXT NOT NULL CHECK (
        redistribution_policy IN ('allowed', 'runtime_only', 'review_required', 'internal')
    )
) STRICT, WITHOUT ROWID;

CREATE TABLE source_snapshots (
    id TEXT PRIMARY KEY,
    provider_id TEXT NOT NULL REFERENCES providers(id),
    revision TEXT NOT NULL,
    source_uri TEXT NOT NULL,
    content_sha256 TEXT NOT NULL,
    content_bytes INTEGER NOT NULL CHECK (content_bytes >= 0),
    data_license TEXT NOT NULL,
    imported_at TEXT NOT NULL,
    UNIQUE (provider_id, revision)
) STRICT, WITHOUT ROWID;

CREATE TABLE libretro_databases (
    id INTEGER PRIMARY KEY,
    source_snapshot_id TEXT NOT NULL REFERENCES source_snapshots(id),
    source_path TEXT NOT NULL,
    source_name TEXT NOT NULL,
    platform_id TEXT NOT NULL REFERENCES platforms(id),
    record_count INTEGER NOT NULL CHECK (record_count >= 0),
    UNIQUE (source_snapshot_id, source_path)
) STRICT;

CREATE TABLE libretro_records (
    id INTEGER PRIMARY KEY,
    database_id INTEGER NOT NULL REFERENCES libretro_databases(id) ON DELETE CASCADE,
    source_ordinal INTEGER NOT NULL CHECK (source_ordinal >= 0),
    title TEXT,
    description TEXT,
    file_name TEXT,
    byte_size INTEGER CHECK (byte_size IS NULL OR byte_size >= 0),
    region TEXT,
    release_date TEXT,
    metadata_json TEXT NOT NULL DEFAULT '{}' CHECK (json_valid(metadata_json)),
    CHECK (id = database_id * 4294967296 + source_ordinal)
) STRICT;

CREATE INDEX libretro_records_database_idx ON libretro_records(database_id);

CREATE TABLE libretro_hashes (
    algorithm TEXT NOT NULL CHECK (algorithm IN ('crc32', 'md5', 'sha1')),
    digest BLOB NOT NULL,
    record_id INTEGER NOT NULL REFERENCES libretro_records(id) ON DELETE CASCADE,
    PRIMARY KEY (algorithm, digest, record_id)
) STRICT, WITHOUT ROWID;

CREATE TABLE libretro_serials (
    identifier TEXT NOT NULL,
    record_id INTEGER NOT NULL REFERENCES libretro_records(id) ON DELETE CASCADE,
    PRIMARY KEY (identifier, record_id)
) STRICT, WITHOUT ROWID;

CREATE TABLE platforms (
    id TEXT PRIMARY KEY,
    canonical_name TEXT NOT NULL,
    normalized_name TEXT NOT NULL UNIQUE,
    category TEXT,
    status TEXT NOT NULL DEFAULT 'provisional' CHECK (
        status IN ('provisional', 'canonical', 'deprecated')
    ),
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
) STRICT, WITHOUT ROWID;

CREATE TABLE platform_aliases (
    platform_id TEXT NOT NULL REFERENCES platforms(id) ON DELETE CASCADE,
    alias TEXT NOT NULL,
    normalized_alias TEXT NOT NULL,
    provider_id TEXT REFERENCES providers(id),
    PRIMARY KEY (platform_id, normalized_alias)
) STRICT, WITHOUT ROWID;

CREATE INDEX platform_aliases_normalized_idx ON platform_aliases(normalized_alias);

CREATE TABLE games (
    id TEXT PRIMARY KEY,
    canonical_title TEXT NOT NULL,
    sort_title TEXT,
    game_type TEXT NOT NULL DEFAULT 'game' CHECK (
        game_type IN ('game', 'expansion', 'dlc', 'mod', 'homebrew', 'prototype', 'application', 'other')
    ),
    status TEXT NOT NULL DEFAULT 'canonical' CHECK (
        status IN ('provisional', 'canonical', 'deprecated', 'merged')
    ),
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
) STRICT, WITHOUT ROWID;

CREATE TABLE releases (
    id TEXT PRIMARY KEY,
    game_id TEXT NOT NULL REFERENCES games(id) ON DELETE CASCADE,
    platform_id TEXT NOT NULL REFERENCES platforms(id),
    release_title TEXT,
    region TEXT,
    languages TEXT,
    version TEXT,
    edition TEXT,
    release_date TEXT,
    status TEXT NOT NULL DEFAULT 'canonical' CHECK (
        status IN ('provisional', 'canonical', 'deprecated', 'merged')
    ),
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
) STRICT, WITHOUT ROWID;

CREATE INDEX releases_game_idx ON releases(game_id);
CREATE INDEX releases_platform_idx ON releases(platform_id);

CREATE TABLE artifacts (
    id TEXT PRIMARY KEY,
    artifact_type TEXT NOT NULL CHECK (
        artifact_type IN ('rom', 'disc', 'digital', 'executable', 'archive', 'multi_file', 'firmware', 'other')
    ),
    file_name TEXT,
    byte_size INTEGER CHECK (byte_size IS NULL OR byte_size >= 0),
    status TEXT NOT NULL DEFAULT 'canonical' CHECK (
        status IN ('provisional', 'canonical', 'bad_dump', 'deprecated')
    ),
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
) STRICT, WITHOUT ROWID;

CREATE TABLE release_artifacts (
    release_id TEXT NOT NULL REFERENCES releases(id) ON DELETE CASCADE,
    artifact_id TEXT NOT NULL REFERENCES artifacts(id) ON DELETE CASCADE,
    role TEXT NOT NULL DEFAULT 'primary' CHECK (role IN ('primary', 'component', 'firmware', 'patch', 'other')),
    PRIMARY KEY (release_id, artifact_id, role)
) STRICT, WITHOUT ROWID;

CREATE INDEX release_artifacts_artifact_idx ON release_artifacts(artifact_id);

CREATE TABLE artifact_hashes (
    artifact_id TEXT NOT NULL REFERENCES artifacts(id) ON DELETE CASCADE,
    algorithm TEXT NOT NULL CHECK (algorithm IN ('crc32', 'md5', 'sha1', 'sha256')),
    digest TEXT NOT NULL,
    PRIMARY KEY (artifact_id, algorithm, digest)
) STRICT, WITHOUT ROWID;

CREATE INDEX artifact_hashes_lookup_idx ON artifact_hashes(algorithm, digest);

CREATE TABLE source_records (
    id TEXT PRIMARY KEY,
    provider_id TEXT NOT NULL REFERENCES providers(id),
    source_snapshot_id TEXT REFERENCES source_snapshots(id),
    source_path TEXT,
    source_ordinal INTEGER CHECK (source_ordinal IS NULL OR source_ordinal >= 0),
    record_type TEXT NOT NULL CHECK (
        record_type IN ('platform', 'game', 'release', 'artifact', 'emulator', 'offer', 'media', 'other')
    ),
    external_id TEXT NOT NULL,
    payload_json TEXT CHECK (payload_json IS NULL OR json_valid(payload_json)),
    payload_sha256 TEXT,
    imported_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CHECK ((payload_json IS NULL) = (payload_sha256 IS NULL)),
    UNIQUE (provider_id, record_type, external_id)
) STRICT, WITHOUT ROWID;

CREATE TABLE source_links (
    source_record_id TEXT NOT NULL REFERENCES source_records(id) ON DELETE CASCADE,
    entity_type TEXT NOT NULL CHECK (
        entity_type IN ('platform', 'game', 'release', 'artifact', 'emulator', 'offer')
    ),
    entity_id TEXT NOT NULL,
    confidence INTEGER NOT NULL CHECK (confidence BETWEEN 0 AND 1000),
    method TEXT NOT NULL CHECK (
        method IN ('exact_hash', 'exact_name', 'trusted_external_id', 'source_native', 'reviewed', 'fuzzy_candidate')
    ),
    status TEXT NOT NULL CHECK (status IN ('accepted', 'pending', 'rejected')),
    decided_at TEXT,
    PRIMARY KEY (source_record_id, entity_type, entity_id)
) STRICT, WITHOUT ROWID;

CREATE INDEX source_links_entity_idx ON source_links(entity_type, entity_id);
CREATE INDEX source_links_pending_idx ON source_links(status) WHERE status = 'pending';

CREATE TABLE entity_assertions (
    id TEXT PRIMARY KEY,
    entity_type TEXT NOT NULL CHECK (
        entity_type IN ('platform', 'game', 'release', 'artifact', 'emulator', 'offer')
    ),
    entity_id TEXT NOT NULL,
    field_name TEXT NOT NULL,
    value_json TEXT NOT NULL CHECK (json_valid(value_json)),
    provider_id TEXT REFERENCES providers(id),
    source_record_id TEXT REFERENCES source_records(id) ON DELETE CASCADE,
    priority INTEGER NOT NULL DEFAULT 0,
    selected INTEGER NOT NULL DEFAULT 0 CHECK (selected IN (0, 1)),
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
) STRICT, WITHOUT ROWID;

CREATE UNIQUE INDEX entity_assertions_selected_idx
ON entity_assertions(entity_type, entity_id, field_name)
WHERE selected = 1;

CREATE TABLE entity_redirects (
    entity_type TEXT NOT NULL CHECK (
        entity_type IN ('platform', 'game', 'release', 'artifact', 'emulator', 'offer')
    ),
    old_id TEXT NOT NULL,
    new_id TEXT NOT NULL,
    reason TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (entity_type, old_id),
    CHECK (old_id <> new_id)
) STRICT, WITHOUT ROWID;

CREATE TABLE release_identifiers (
    release_id TEXT NOT NULL REFERENCES releases(id) ON DELETE CASCADE,
    provider_id TEXT REFERENCES providers(id),
    identifier_type TEXT NOT NULL CHECK (identifier_type IN ('serial', 'product_code', 'store_id', 'other')),
    identifier TEXT NOT NULL,
    PRIMARY KEY (release_id, identifier_type, identifier)
) STRICT, WITHOUT ROWID;

CREATE INDEX release_identifiers_lookup_idx
ON release_identifiers(identifier_type, identifier);

CREATE TABLE identity_events (
    id TEXT PRIMARY KEY,
    entity_type TEXT NOT NULL,
    action TEXT NOT NULL CHECK (action IN ('create', 'merge', 'split', 'unlink', 'relink')),
    details_json TEXT NOT NULL CHECK (json_valid(details_json)),
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
) STRICT, WITHOUT ROWID;

CREATE TABLE acquisition_offers (
    id TEXT PRIMARY KEY,
    provider_id TEXT NOT NULL REFERENCES providers(id),
    artifact_id TEXT REFERENCES artifacts(id) ON DELETE SET NULL,
    external_id TEXT NOT NULL,
    display_name TEXT NOT NULL,
    platform_hint TEXT,
    byte_size INTEGER CHECK (byte_size IS NULL OR byte_size >= 0),
    checksum_algorithm TEXT,
    checksum_digest TEXT,
    availability TEXT NOT NULL DEFAULT 'unknown' CHECK (
        availability IN ('available', 'unavailable', 'unknown')
    ),
    metadata_json TEXT NOT NULL DEFAULT '{}' CHECK (json_valid(metadata_json)),
    UNIQUE (provider_id, external_id)
) STRICT, WITHOUT ROWID;

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

CREATE TABLE emulators (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    normalized_name TEXT NOT NULL UNIQUE,
    homepage TEXT,
    notes TEXT,
    save_directory TEXT,
    save_extensions TEXT,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
) STRICT, WITHOUT ROWID;

CREATE TABLE emulator_aliases (
    emulator_id TEXT NOT NULL REFERENCES emulators(id) ON DELETE CASCADE,
    alias TEXT NOT NULL,
    normalized_alias TEXT NOT NULL,
    PRIMARY KEY (emulator_id, alias)
) STRICT, WITHOUT ROWID;

CREATE INDEX emulator_aliases_normalized_idx ON emulator_aliases(normalized_alias);

CREATE TABLE host_systems (
    slug TEXT PRIMARY KEY,
    display_name TEXT NOT NULL,
    family TEXT NOT NULL CHECK (family IN ('desktop', 'mobile', 'console', 'web', 'other'))
) STRICT, WITHOUT ROWID;

CREATE TABLE emulator_host_systems (
    emulator_id TEXT NOT NULL REFERENCES emulators(id) ON DELETE CASCADE,
    host_system_slug TEXT NOT NULL REFERENCES host_systems(slug),
    PRIMARY KEY (emulator_id, host_system_slug)
) STRICT, WITHOUT ROWID;

CREATE TABLE emulator_packages (
    emulator_id TEXT NOT NULL REFERENCES emulators(id) ON DELETE CASCADE,
    host_system_slug TEXT NOT NULL REFERENCES host_systems(slug),
    manager TEXT NOT NULL CHECK (
        manager IN ('winget', 'homebrew', 'flatpak', 'appimage', 'nix', 'github', 'direct')
    ),
    package_id TEXT NOT NULL,
    metadata_json TEXT NOT NULL DEFAULT '{}' CHECK (json_valid(metadata_json)),
    PRIMARY KEY (emulator_id, host_system_slug, manager, package_id)
) STRICT, WITHOUT ROWID;

CREATE TABLE emulator_platforms (
    emulator_id TEXT NOT NULL REFERENCES emulators(id) ON DELETE CASCADE,
    platform_id TEXT NOT NULL REFERENCES platforms(id) ON DELETE CASCADE,
    core_name TEXT NOT NULL DEFAULT '',
    recommended INTEGER NOT NULL DEFAULT 1 CHECK (recommended IN (0, 1)),
    PRIMARY KEY (emulator_id, platform_id, core_name)
) STRICT, WITHOUT ROWID;

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

CREATE TABLE data_quality_issues (
    id TEXT PRIMARY KEY,
    severity TEXT NOT NULL CHECK (severity IN ('info', 'warning', 'error')),
    issue_type TEXT NOT NULL,
    entity_type TEXT,
    entity_id TEXT,
    details_json TEXT NOT NULL CHECK (json_valid(details_json)),
    status TEXT NOT NULL DEFAULT 'open' CHECK (status IN ('open', 'resolved', 'ignored')),
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    resolved_at TEXT
) STRICT, WITHOUT ROWID;

CREATE INDEX data_quality_issues_open_idx ON data_quality_issues(severity, status);
