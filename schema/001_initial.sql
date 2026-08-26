PRAGMA foreign_keys = ON;

CREATE TABLE schema_migrations (
    version INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    applied_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
) STRICT;

INSERT INTO schema_migrations (version, name, applied_at)
VALUES (1, 'canonical identity and emulator catalog', '1970-01-01T00:00:00Z');

CREATE TABLE build_metadata (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
) STRICT;

CREATE TABLE providers (
    id TEXT PRIMARY KEY,
    slug TEXT NOT NULL UNIQUE,
    name TEXT NOT NULL,
    homepage TEXT,
    data_license TEXT,
    redistribution_policy TEXT NOT NULL CHECK (
        redistribution_policy IN ('allowed', 'runtime_only', 'review_required', 'internal')
    )
) STRICT;

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
) STRICT;

CREATE TABLE platform_aliases (
    platform_id TEXT NOT NULL REFERENCES platforms(id) ON DELETE CASCADE,
    alias TEXT NOT NULL,
    normalized_alias TEXT NOT NULL,
    provider_id TEXT REFERENCES providers(id),
    PRIMARY KEY (platform_id, normalized_alias)
) STRICT;

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
) STRICT;

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
) STRICT;

CREATE INDEX releases_game_idx ON releases(game_id);
CREATE INDEX releases_platform_idx ON releases(platform_id);

CREATE TABLE artifacts (
    id TEXT PRIMARY KEY,
    release_id TEXT REFERENCES releases(id) ON DELETE SET NULL,
    artifact_type TEXT NOT NULL CHECK (
        artifact_type IN ('rom', 'disc', 'digital', 'executable', 'archive', 'multi_file', 'firmware', 'other')
    ),
    file_name TEXT,
    byte_size INTEGER CHECK (byte_size IS NULL OR byte_size >= 0),
    status TEXT NOT NULL DEFAULT 'canonical' CHECK (
        status IN ('provisional', 'canonical', 'bad_dump', 'deprecated')
    ),
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
) STRICT;

CREATE INDEX artifacts_release_idx ON artifacts(release_id);

CREATE TABLE artifact_hashes (
    artifact_id TEXT NOT NULL REFERENCES artifacts(id) ON DELETE CASCADE,
    algorithm TEXT NOT NULL CHECK (algorithm IN ('crc32', 'md5', 'sha1', 'sha256')),
    digest TEXT NOT NULL,
    PRIMARY KEY (artifact_id, algorithm),
    UNIQUE (algorithm, digest)
) STRICT;

CREATE INDEX artifact_hashes_lookup_idx ON artifact_hashes(algorithm, digest);

CREATE TABLE source_records (
    id TEXT PRIMARY KEY,
    provider_id TEXT NOT NULL REFERENCES providers(id),
    record_type TEXT NOT NULL CHECK (
        record_type IN ('platform', 'game', 'release', 'artifact', 'emulator', 'offer', 'media', 'other')
    ),
    external_id TEXT NOT NULL,
    payload_json TEXT NOT NULL CHECK (json_valid(payload_json)),
    payload_sha256 TEXT NOT NULL,
    imported_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE (provider_id, record_type, external_id)
) STRICT;

CREATE TABLE source_links (
    source_record_id TEXT NOT NULL REFERENCES source_records(id) ON DELETE CASCADE,
    entity_type TEXT NOT NULL CHECK (
        entity_type IN ('platform', 'game', 'release', 'artifact', 'emulator', 'offer')
    ),
    entity_id TEXT NOT NULL,
    confidence INTEGER NOT NULL CHECK (confidence BETWEEN 0 AND 1000),
    method TEXT NOT NULL CHECK (
        method IN ('exact_hash', 'trusted_external_id', 'source_native', 'reviewed', 'fuzzy_candidate')
    ),
    status TEXT NOT NULL CHECK (status IN ('accepted', 'pending', 'rejected')),
    decided_at TEXT,
    PRIMARY KEY (source_record_id, entity_type, entity_id)
) STRICT;

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
) STRICT;

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
) STRICT;

CREATE TABLE identity_events (
    id TEXT PRIMARY KEY,
    entity_type TEXT NOT NULL,
    action TEXT NOT NULL CHECK (action IN ('create', 'merge', 'split', 'unlink', 'relink')),
    details_json TEXT NOT NULL CHECK (json_valid(details_json)),
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
) STRICT;

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
) STRICT;

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
) STRICT;

CREATE TABLE emulator_aliases (
    emulator_id TEXT NOT NULL REFERENCES emulators(id) ON DELETE CASCADE,
    alias TEXT NOT NULL,
    normalized_alias TEXT NOT NULL,
    PRIMARY KEY (emulator_id, alias)
) STRICT;

CREATE INDEX emulator_aliases_normalized_idx ON emulator_aliases(normalized_alias);

CREATE TABLE host_systems (
    slug TEXT PRIMARY KEY,
    display_name TEXT NOT NULL,
    family TEXT NOT NULL CHECK (family IN ('desktop', 'mobile', 'console', 'web', 'other'))
) STRICT;

CREATE TABLE emulator_host_systems (
    emulator_id TEXT NOT NULL REFERENCES emulators(id) ON DELETE CASCADE,
    host_system_slug TEXT NOT NULL REFERENCES host_systems(slug),
    PRIMARY KEY (emulator_id, host_system_slug)
) STRICT;

CREATE TABLE emulator_packages (
    emulator_id TEXT NOT NULL REFERENCES emulators(id) ON DELETE CASCADE,
    manager TEXT NOT NULL CHECK (manager IN ('winget', 'homebrew', 'flatpak', 'snap', 'nix', 'other')),
    package_id TEXT NOT NULL,
    PRIMARY KEY (emulator_id, manager, package_id)
) STRICT;

CREATE TABLE emulator_platforms (
    emulator_id TEXT NOT NULL REFERENCES emulators(id) ON DELETE CASCADE,
    platform_id TEXT NOT NULL REFERENCES platforms(id) ON DELETE CASCADE,
    core_name TEXT NOT NULL DEFAULT '',
    recommended INTEGER NOT NULL DEFAULT 1 CHECK (recommended IN (0, 1)),
    PRIMARY KEY (emulator_id, platform_id, core_name)
) STRICT;

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
) STRICT;

CREATE INDEX data_quality_issues_open_idx ON data_quality_issues(severity, status);
