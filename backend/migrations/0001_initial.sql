-- Initial schema for Lunchbox

-- Platforms/Systems
CREATE TABLE IF NOT EXISTS platforms (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    screenscraper_id INTEGER,
    retroarch_core TEXT,
    file_extensions TEXT,  -- JSON array
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL
);

-- Games (metadata)
CREATE TABLE IF NOT EXISTS games (
    id TEXT PRIMARY KEY,  -- UUID
    title TEXT NOT NULL,
    platform_id INTEGER NOT NULL REFERENCES platforms(id),
    launchbox_db_id INTEGER,
    screenscraper_id INTEGER,
    igdb_id INTEGER,
    description TEXT,
    release_date DATE,
    developer TEXT,
    publisher TEXT,
    genres TEXT,  -- JSON array
    players TEXT,
    rating REAL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_games_platform ON games(platform_id);
CREATE INDEX IF NOT EXISTS idx_games_launchbox_id ON games(launchbox_db_id);
CREATE INDEX IF NOT EXISTS idx_games_title ON games(title);

-- ROMs (physical files)
CREATE TABLE IF NOT EXISTS roms (
    id TEXT PRIMARY KEY,  -- UUID
    game_id TEXT REFERENCES games(id),
    file_path TEXT NOT NULL,
    file_name TEXT NOT NULL,
    file_size INTEGER NOT NULL,
    crc32 TEXT,
    md5 TEXT,
    sha1 TEXT,
    region TEXT,
    version TEXT,
    verified INTEGER DEFAULT 0 NOT NULL,
    last_played TIMESTAMP,
    play_count INTEGER DEFAULT 0 NOT NULL,
    play_time_seconds INTEGER DEFAULT 0 NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_roms_game ON roms(game_id);
CREATE INDEX IF NOT EXISTS idx_roms_sha1 ON roms(sha1);
CREATE INDEX IF NOT EXISTS idx_roms_crc32 ON roms(crc32);
CREATE UNIQUE INDEX IF NOT EXISTS idx_roms_path ON roms(file_path);

-- Media assets
CREATE TABLE IF NOT EXISTS media (
    id TEXT PRIMARY KEY,
    game_id TEXT NOT NULL REFERENCES games(id),
    media_type TEXT NOT NULL,  -- 'box_front', 'box_back', 'screenshot', 'video', 'fanart', etc.
    file_path TEXT NOT NULL,
    source TEXT,  -- 'screenscraper', 'emumovies', 'launchbox', 'local'
    width INTEGER,
    height INTEGER,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_media_game ON media(game_id);
CREATE INDEX IF NOT EXISTS idx_media_type ON media(media_type);

-- Emulators
CREATE TABLE IF NOT EXISTS emulators (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    executable_path TEXT,
    emulator_type TEXT NOT NULL,  -- 'retroarch', 'standalone'
    version TEXT,
    installed INTEGER DEFAULT 0 NOT NULL
);

-- Platform-Emulator mapping
CREATE TABLE IF NOT EXISTS platform_emulators (
    platform_id INTEGER NOT NULL REFERENCES platforms(id),
    emulator_id TEXT NOT NULL REFERENCES emulators(id),
    core_name TEXT,  -- RetroArch core name
    is_default INTEGER DEFAULT 0 NOT NULL,
    command_line_args TEXT,
    PRIMARY KEY (platform_id, emulator_id)
);

-- Collections/Playlists
CREATE TABLE IF NOT EXISTS collections (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT,
    is_smart INTEGER DEFAULT 0 NOT NULL,  -- smart collection with filters
    filter_rules TEXT,  -- JSON for smart collections
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL
);

CREATE TABLE IF NOT EXISTS collection_games (
    collection_id TEXT NOT NULL REFERENCES collections(id),
    game_id TEXT NOT NULL REFERENCES games(id),
    sort_order INTEGER,
    PRIMARY KEY (collection_id, game_id)
);

-- Settings key-value store
CREATE TABLE IF NOT EXISTS settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
