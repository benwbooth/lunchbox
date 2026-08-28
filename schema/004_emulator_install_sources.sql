BEGIN IMMEDIATE;

ALTER TABLE emulator_packages RENAME TO emulator_packages_unscoped;

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

INSERT INTO emulator_packages
    (emulator_id, host_system_slug, manager, package_id, metadata_json)
SELECT emulator_id,
       CASE manager
           WHEN 'winget' THEN 'windows'
           WHEN 'homebrew' THEN 'macos'
           ELSE 'linux'
       END,
       manager,
       package_id,
       '{}'
FROM emulator_packages_unscoped
WHERE manager IN ('winget', 'homebrew', 'flatpak', 'nix');

DROP TABLE emulator_packages_unscoped;

INSERT INTO schema_migrations (version, name, applied_at)
VALUES (4, 'host scoped emulator install sources', '1970-01-01T00:00:00Z');

COMMIT;
