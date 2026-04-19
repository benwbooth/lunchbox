CREATE TABLE IF NOT EXISTS firmware_packages (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    source TEXT NOT NULL,
    package_key TEXT NOT NULL,
    package_name TEXT NOT NULL,
    archive_path TEXT NOT NULL,
    extracted_root TEXT NOT NULL,
    source_url TEXT,
    sha256 TEXT,
    imported_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    UNIQUE(source, package_key)
);

CREATE INDEX IF NOT EXISTS idx_firmware_packages_source
    ON firmware_packages(source);

CREATE INDEX IF NOT EXISTS idx_firmware_packages_name
    ON firmware_packages(package_name);

CREATE TABLE IF NOT EXISTS firmware_files (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    package_id INTEGER NOT NULL REFERENCES firmware_packages(id) ON DELETE CASCADE,
    relative_path TEXT NOT NULL,
    store_path TEXT NOT NULL,
    sha256 TEXT,
    file_size INTEGER NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    UNIQUE(package_id, relative_path)
);

CREATE INDEX IF NOT EXISTS idx_firmware_files_package
    ON firmware_files(package_id);

CREATE INDEX IF NOT EXISTS idx_firmware_files_sha256
    ON firmware_files(sha256);

CREATE TABLE IF NOT EXISTS firmware_rules (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    rule_key TEXT NOT NULL UNIQUE,
    runtime_kind TEXT NOT NULL,
    runtime_name TEXT NOT NULL,
    platform_name TEXT,
    source TEXT NOT NULL,
    source_package_name TEXT NOT NULL,
    target_subdir TEXT NOT NULL DEFAULT '',
    install_mode TEXT NOT NULL DEFAULT 'merge_tree',
    required INTEGER DEFAULT 1 NOT NULL,
    notes TEXT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_firmware_rules_runtime
    ON firmware_rules(runtime_kind, runtime_name);

CREATE INDEX IF NOT EXISTS idx_firmware_rules_platform
    ON firmware_rules(platform_name);

CREATE TABLE IF NOT EXISTS firmware_installs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    rule_key TEXT NOT NULL REFERENCES firmware_rules(rule_key) ON DELETE CASCADE,
    runtime_install_path TEXT NOT NULL,
    package_id INTEGER REFERENCES firmware_packages(id) ON DELETE SET NULL,
    target_path TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'synced',
    synced_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    UNIQUE(rule_key, runtime_install_path, target_path)
);

CREATE INDEX IF NOT EXISTS idx_firmware_installs_rule
    ON firmware_installs(rule_key);

CREATE INDEX IF NOT EXISTS idx_firmware_installs_package
    ON firmware_installs(package_id);
