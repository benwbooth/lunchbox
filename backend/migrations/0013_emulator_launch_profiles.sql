-- Emulator launch argument overrides (user database)
-- Stores optional CLI argument strings keyed by emulator, platform, and runtime kind.

CREATE TABLE IF NOT EXISTS emulator_launch_profiles (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    emulator_name TEXT NOT NULL,
    platform_name TEXT NOT NULL DEFAULT '',
    runtime_kind TEXT NOT NULL,
    args_text TEXT NOT NULL DEFAULT '',
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    UNIQUE(emulator_name, platform_name, runtime_kind)
);

CREATE INDEX IF NOT EXISTS idx_emulator_launch_profiles_lookup
    ON emulator_launch_profiles(emulator_name, platform_name, runtime_kind);
