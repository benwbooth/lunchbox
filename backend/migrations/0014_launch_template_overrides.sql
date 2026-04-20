-- Launch template overrides (user database)
-- Stores unresolved command template overrides over built-in launch behavior.

CREATE TABLE IF NOT EXISTS emulator_launch_template_overrides (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    platform_name TEXT NOT NULL DEFAULT '',
    emulator_name TEXT NOT NULL,
    runtime_kind TEXT NOT NULL,
    command_template TEXT NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    UNIQUE(platform_name, emulator_name, runtime_kind)
);

CREATE INDEX IF NOT EXISTS idx_emulator_launch_template_overrides_lookup
    ON emulator_launch_template_overrides(platform_name, emulator_name, runtime_kind);

CREATE TABLE IF NOT EXISTS game_launch_template_overrides (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    launchbox_db_id INTEGER NOT NULL,
    emulator_name TEXT NOT NULL,
    runtime_kind TEXT NOT NULL,
    command_template TEXT NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    UNIQUE(launchbox_db_id, emulator_name, runtime_kind)
);

CREATE INDEX IF NOT EXISTS idx_game_launch_template_overrides_lookup
    ON game_launch_template_overrides(launchbox_db_id, emulator_name, runtime_kind);
