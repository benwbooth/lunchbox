-- Game files: tracks imported/downloaded ROM files for games
CREATE TABLE IF NOT EXISTS game_files (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    launchbox_db_id INTEGER NOT NULL,
    game_title TEXT NOT NULL,
    platform TEXT NOT NULL,
    file_path TEXT NOT NULL,
    file_size INTEGER,
    imported_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    import_source TEXT DEFAULT 'graboid' NOT NULL,
    graboid_job_id TEXT,
    UNIQUE(launchbox_db_id)
);

-- Graboid import jobs: tracks in-progress and completed import jobs
CREATE TABLE IF NOT EXISTS graboid_jobs (
    id TEXT PRIMARY KEY,
    launchbox_db_id INTEGER NOT NULL,
    game_title TEXT NOT NULL,
    platform TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending',
    progress_percent REAL DEFAULT 0,
    status_message TEXT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL
);

-- Graboid prompt overrides: customizable prompts at global, platform, and game level
-- scope: 'global' (one row, platform/launchbox_db_id null), 'platform' (platform set), 'game' (launchbox_db_id set)
CREATE TABLE IF NOT EXISTS graboid_prompts (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    scope TEXT NOT NULL CHECK(scope IN ('global', 'platform', 'game')),
    platform TEXT,
    launchbox_db_id INTEGER,
    prompt TEXT NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    UNIQUE(scope, platform, launchbox_db_id)
);
