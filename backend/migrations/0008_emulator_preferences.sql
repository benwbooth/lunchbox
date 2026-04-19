-- Emulator preferences (user database)
-- Stores per-game and per-platform emulator preferences

CREATE TABLE IF NOT EXISTS emulator_preferences (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    -- Only one of these should be set
    launchbox_db_id INTEGER,           -- For per-game preference (game's LaunchBox DB ID)
    platform_name TEXT,                 -- For per-platform preference
    -- The selected emulator
    emulator_name TEXT NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    -- Ensure uniqueness
    UNIQUE(launchbox_db_id),
    UNIQUE(platform_name)
);

CREATE INDEX IF NOT EXISTS idx_emulator_prefs_game ON emulator_preferences(launchbox_db_id);
CREATE INDEX IF NOT EXISTS idx_emulator_prefs_platform ON emulator_preferences(platform_name);
